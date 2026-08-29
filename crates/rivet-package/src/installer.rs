//! Orchestrates the source → build → install pipeline for a resolved
//! package, and its inverse at removal time.
//!
//! What this does NOT do yet: fetch remote sources. `Source::Archive` and
//! `Source::Git` are rejected with a clear error until a fetch mechanism
//! exists (tracked separately — see the project roadmap, "implement
//! package downloading/building"). `Source::Local` works today, which is
//! enough to develop and test the hook pipeline itself.

use std::fs;
use std::path::{Path, PathBuf};

use rivet_core::{InstalledDatabase, InstalledRecord};
use walkdir::WalkDir;

use crate::context::BuildContext;
use crate::error::{PackageError, Result};
use crate::loader::PackageLoader;
use crate::manifest::PackageManifest;
use crate::source::Source;

/// Installs a single resolved package into `prefix`, running its
/// `pre_install`, `build`, `install`, and `post_install` hooks in order,
/// and records the result in `db`.
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
    let dest_dir = prefix.join(format!("{}-{}", manifest.name, manifest.version));
    fs::create_dir_all(&dest_dir).map_err(PackageError::Io)?;

    let ctx = BuildContext::new(&source_dir, build_dir.path(), &dest_dir);

    let loader = PackageLoader::new()?;
    loader.run_hooks(
        &script,
        &ctx,
        &["pre_install", "build", "install", "post_install"],
    )?;

    let installed_files = collect_installed_files(&dest_dir);

    let record = InstalledRecord::new(
        manifest.name.clone(),
        manifest.version.clone(),
        manifest.description.clone(),
        installed_files,
        is_explicit,
        Some(script),
    );

    db.record_install(record.clone())?;

    Ok(record)
}

/// Runs a package's `uninstall` hook, if it defined one, using the recipe
/// snapshot taken at install time — deliberately independent of whatever
/// the repository currently contains.
pub fn uninstall(record: &InstalledRecord, prefix: &Path) -> Result<()> {
    let Some(script) = &record.recipe_snapshot else {
        return Ok(());
    };

    let dest_dir = prefix.join(format!("{}-{}", record.name, record.version));
    let ctx = BuildContext::new(&dest_dir, &dest_dir, &dest_dir);

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
