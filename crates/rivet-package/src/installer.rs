use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use rivet_core::{InstalledDatabase, InstalledRecord, PackageName, RecordedDependency};
use walkdir::WalkDir;

use crate::context::BuildContext;
use crate::dependency::DependencyKind;
use crate::error::{PackageError, Result};
use crate::loader::PackageLoader;
use crate::manifest::PackageManifest;
use crate::source::Source;

pub fn install(
    manifest: &PackageManifest,
    prefix: &Path,
    cache_dir: &Path,
    db: &mut InstalledDatabase,
    is_explicit: bool,
) -> Result<InstalledRecord> {
    let script =
        fs::read_to_string(&manifest.recipe_path).map_err(|_| PackageError::InvalidField {
            field: "recipe_path",
            reason: format!(
                "could not re-read recipe at '{}' — was it moved or was this manifest built \
                 in-memory (e.g. in a test) rather than loaded from disk?",
                manifest.recipe_path.display()
            ),
        })?;

    let source_dir = resolve_source(manifest, cache_dir)?;
    let build_dir = tempfile::tempdir().map_err(PackageError::Io)?;
    fs::create_dir_all(cache_dir).map_err(PackageError::Io)?;
    let staged_package = tempfile::tempdir_in(cache_dir).map_err(PackageError::Io)?;
    let staged_dir = staged_package.path().to_path_buf();
    let dest_dir = cache_dir.join(format!("{}-{}", manifest.name, manifest.version));

    let file_owners: HashMap<_, _> = db
        .list_installed()
        .into_iter()
        .flat_map(|record| {
            record
                .installed_files
                .iter()
                .map(move |path| (path.clone(), record.name.to_string()))
        })
        .collect();
    let ctx = BuildContext::new(&source_dir, build_dir.path(), &staged_dir, prefix)
        .with_file_ownership(manifest.name.to_string(), file_owners);

    let loader = PackageLoader::new()?;
    if let Err(error) = loader.run_hooks(&script, &ctx, &["pre_install", "build", "install"]) {
        ctx.rollback_new_writes().map_err(PackageError::Io)?;
        return Err(error.into());
    }

    // Preserve a same-version install until the new hooks have succeeded.
    // The staging directory is inside the cache, so both renames stay on one
    // filesystem and the active artifact is switched as a single operation.
    let backup = match fs::symlink_metadata(&dest_dir) {
        Ok(_) => {
            let backup_dir = tempfile::tempdir_in(cache_dir).map_err(PackageError::Io)?;
            let backup_path = backup_dir.path().join("previous");
            if let Err(error) = fs::rename(&dest_dir, &backup_path) {
                return Err(PackageError::Io(error));
            }
            Some((backup_dir, backup_path))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(PackageError::Io(error)),
    };

    if let Err(error) = fs::rename(&staged_dir, &dest_dir) {
        if let Some((backup_dir, backup_path)) = backup {
            if let Err(restore_error) = fs::rename(&backup_path, &dest_dir) {
                let _ = backup_dir.keep();
                return Err(PackageError::Io(restore_error));
            }
        }
        return Err(PackageError::Io(error));
    }
    let _ = staged_package.keep();

    // Run post_install only after the artifact has its stable final path.
    // Recipes commonly create prefix symlinks that point into this directory.
    let mut post_install_ctx = ctx.clone();
    post_install_ctx.dest_dir = dest_dir.clone();
    if let Err(error) = loader.run_hooks(&script, &post_install_ctx, &["post_install"]) {
        let rollback_error = post_install_ctx.rollback_new_writes().err();
        if let Err(remove_error) = fs::remove_dir_all(&dest_dir) {
            if let Some((backup_dir, _)) = backup {
                let _ = backup_dir.keep();
            }
            return Err(PackageError::Io(remove_error));
        }
        if let Some((backup_dir, backup_path)) = backup {
            if let Err(restore_error) = fs::rename(&backup_path, &dest_dir) {
                let _ = backup_dir.keep();
                return Err(PackageError::Io(restore_error));
            }
        }
        if let Some(rollback_error) = rollback_error {
            return Err(PackageError::Io(rollback_error));
        }
        return Err(error.into());
    }

    let mut installed_files: HashSet<_> = collect_installed_files(&dest_dir).into_iter().collect();
    installed_files.extend(ctx.written_paths());
    installed_files = installed_files
        .into_iter()
        .map(|path| {
            path.strip_prefix(&staged_dir)
                .map(|relative| dest_dir.join(relative))
                .unwrap_or(path)
        })
        .filter(|path| fs::symlink_metadata(path).is_ok())
        .collect();

    let recorded_dependencies = manifest
        .dependencies
        .iter()
        .map(|dep| RecordedDependency {
            name: dep.name.clone(),
            runtime: dep.kind == DependencyKind::Runtime,
        })
        .collect();
    let record = InstalledRecord::new(
        manifest.name.clone(),
        manifest.version.clone(),
        manifest.description.clone(),
        installed_files.into_iter().collect(),
        is_explicit,
        Some(script),
        recorded_dependencies,
        manifest.source_repository.clone(),
    );

    if let Err(error) = db.record_install(record.clone()) {
        let rollback_error = post_install_ctx.rollback_new_writes().err();
        if let Err(remove_error) = fs::remove_dir_all(&dest_dir) {
            if let Some((backup_dir, _)) = backup {
                let _ = backup_dir.keep();
            }
            return Err(PackageError::Io(remove_error));
        }
        if let Some((backup_dir, backup_path)) = backup {
            if let Err(restore_error) = fs::rename(&backup_path, &dest_dir) {
                let _ = backup_dir.keep();
                return Err(PackageError::Io(restore_error));
            }
        }
        if let Some(rollback_error) = rollback_error {
            return Err(PackageError::Io(rollback_error));
        }
        return Err(error.into());
    }
    drop(backup);

    for candidate in &manifest.cleanup {
        if let Some(removed) = try_cleanup(candidate, db, cache_dir, prefix)? {
            println!(
                "🧹 cleaned up build-only dependency '{}' v{} (no longer needed at runtime)",
                removed.name, removed.version
            );
        }
    }

    Ok(record)
}

fn try_cleanup(
    candidate: &PackageName,
    db: &mut InstalledDatabase,
    cache_dir: &Path,
    prefix: &Path,
) -> Result<Option<InstalledRecord>> {
    let Some(record) = db.get(candidate) else {
        return Ok(None);
    };

    if record.is_explicit {
        return Ok(None);
    }

    let still_needed_at_runtime = db.list_installed().iter().any(|installed| {
        installed
            .dependencies
            .iter()
            .any(|dep| dep.runtime && &dep.name == candidate)
    });

    if still_needed_at_runtime {
        return Ok(None);
    }

    let record = record.clone();

    if let Err(e) = uninstall(&record, cache_dir, prefix) {
        eprintln!(
            "⚠️  cleanup uninstall hook for '{}' failed: {}. Continuing with file removal anyway.",
            record.name, e
        );
    }

    db.remove_package(candidate)?;

    Ok(Some(record))
}

/// Runs a package's `uninstall` hook, if it defined one, using the recipe
/// snapshot taken at install time — deliberately independent of whatever
/// the repository currently contains.
pub fn uninstall(record: &InstalledRecord, cache_dir: &Path, prefix: &Path) -> Result<()> {
    let Some(script) = &record.recipe_snapshot else {
        return Ok(());
    };

    let dest_dir = cache_dir.join(format!("{}-{}", record.name, record.version));
    let ctx = BuildContext::new(&dest_dir, &dest_dir, &dest_dir, prefix);

    let loader = PackageLoader::new()?;
    loader.run_hooks(script, &ctx, &["uninstall"])
}

/// Resolves a package's declared `source` into a local directory the
/// build hook can read from.
fn resolve_source(manifest: &PackageManifest, cache_dir: &Path) -> Result<PathBuf> {
    match &manifest.source {
        None | Some(Source::Virtual) => {
            let dir = tempfile::tempdir().map_err(PackageError::Io)?;
            Ok(dir.keep().to_path_buf())
        }

        Some(Source::Local { path }) => {
            let p = PathBuf::from(path);

            if p.is_absolute() {
                Ok(p)
            } else {
                let base = manifest
                    .recipe_path
                    .parent()
                    .unwrap_or_else(|| Path::new("."));

                Ok(base.join(p))
            }
        }

        Some(Source::Git {
            url,
            reference,
            checksum: _,
        }) => crate::fetch::fetch_git(url, reference.as_ref(), cache_dir),

        Some(Source::Archive { url, checksum }) => {
            crate::fetch::fetch_archive(url, checksum, cache_dir)
        }
    }
}

/// Walks `dest_dir` and returns the absolute paths of every regular file
/// it contains, so they can be tracked for later removal.
fn collect_installed_files(dest_dir: &Path) -> Vec<PathBuf> {
    WalkDir::new(dest_dir)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect()
}
