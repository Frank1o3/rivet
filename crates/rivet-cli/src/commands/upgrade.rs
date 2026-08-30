use std::path::Path;

use rivet_core::{InstalledDatabase, PackageName, Target, Version, VersionReq};
use rivet_package::{Dependency, DependencyKind};
use rivet_repository::MultiRepositoryManager;
use rivet_resolver::DependencySolver;

/// A package whose installed version differs from what currently
/// resolves against configured repositories.
struct Upgradeable {
    name: PackageName,
    installed: Version,
    available: Version,
}

pub fn execute(
    repos: &MultiRepositoryManager,
    db: &mut InstalledDatabase,
    prefix: &Path,
    cache_dir: &Path,
    dry_run: bool,
) -> anyhow::Result<()> {
    let target = Target::host();
    let solver = DependencySolver::new(repos, &target);

    let mut upgrades = Vec::new();

    // Only explicitly-installed packages are candidates: a package
    // pulled in purely as a dependency upgrades implicitly whenever
    // whatever depends on it does — same reasoning `install` already
    // uses to decide what counts as "explicit".
    for record in db.list_installed() {
        if !record.is_explicit {
            continue;
        }

        let dep = Dependency::new(
            record.name.clone(),
            VersionReq::STAR,
            DependencyKind::Runtime,
        );
        let plan = match solver.resolve(&[dep]) {
            Ok(plan) => plan,
            Err(e) => {
                eprintln!("⚠️  could not resolve '{}': {}", record.name, e);
                continue;
            }
        };

        let Some(resolved) = plan.iter().find(|p| p.manifest.name == record.name) else {
            continue;
        };

        if resolved.manifest.version != record.version {
            upgrades.push(Upgradeable {
                name: record.name.clone(),
                installed: record.version.clone(),
                available: resolved.manifest.version.clone(),
            });
        }
    }

    if upgrades.is_empty() {
        println!("✨ Everything is up to date.");
        return Ok(());
    }

    println!("📦 {} package(s) have updates available:\n", upgrades.len());
    for u in &upgrades {
        println!(
            "  {:<20} {} -> {}",
            u.name.as_str(),
            u.installed,
            u.available
        );
    }

    if dry_run {
        println!("\n🚀 Dry run — no changes made. Run without --dry-run to upgrade.");
        return Ok(());
    }

    println!();
    for u in &upgrades {
        // NOTE: this upgrades the named package in isolation — it
        // re-resolves and reinstalls just that package's newer version,
        // but doesn't re-run a full plan for the rest of the dependency
        // graph. A package pulled in transitively picks up its own
        // newer version the next time something explicit forces a
        // re-resolve of it. A coordinated whole-graph upgrade is real
        // future work, not something to quietly half-implement here.
        let dep = Dependency::new(u.name.clone(), VersionReq::STAR, DependencyKind::Runtime);
        let plan = solver.resolve(&[dep])?;
        let Some(resolved) = plan.iter().find(|p| p.manifest.name == u.name) else {
            continue;
        };

        println!(
            "⬆️  Upgrading '{}' {} -> {}...",
            u.name, u.installed, u.available
        );

        if let Some(old_record) = db.get(&u.name).cloned() {
            if let Err(e) = rivet_package::uninstall(&old_record, cache_dir, prefix) {
                eprintln!(
                    "⚠️  uninstall hook for old '{}' failed: {}. Continuing anyway.",
                    u.name, e
                );
            }
            // Removes the old file list from disk and the DB record —
            // without this, `install` below would silently overwrite
            // the DB entry while the old version's files leak on disk.
            db.remove_package(&u.name)?;
        }

        rivet_package::install(&resolved.manifest, prefix, cache_dir, db, true)?;
        println!("✅ '{}' upgraded to v{}.", u.name, u.available);
    }

    println!("\n🚀 Upgrade complete.");
    Ok(())
}
