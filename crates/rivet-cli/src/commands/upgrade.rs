use std::collections::HashSet;
use std::path::Path;

use rivet_core::{InstalledDatabase, PackageName, Target, Version, VersionReq};
use rivet_package::{Dependency, DependencyKind};
use rivet_repository::MultiRepositoryManager;
use rivet_resolver::{DependencySolver, ResolutionPlan};

/// A planned operation during upgrade.
pub struct UpgradeAction {
    pub name: PackageName,
    pub current_version: Option<Version>,
    pub target_version: Version,
    pub is_new_dependency: bool,
}

pub fn execute(
    repos: &MultiRepositoryManager,
    package_filter: &[String],
    db: &mut InstalledDatabase,
    prefix: &Path,
    cache_dir: &Path,
    dry_run: bool,
) -> anyhow::Result<()> {
    let target = Target::host();
    let solver = DependencySolver::new(repos, &target);

    // 1. Determine target packages to check
    let installed_records = db.list_installed();
    if installed_records.is_empty() {
        println!("✨ No packages are currently installed.");
        return Ok(());
    }

    let target_names: Vec<PackageName> = if !package_filter.is_empty() {
        let mut names = Vec::new();
        for arg in package_filter {
            let name = PackageName::new(arg)?;
            if !db.is_installed(&name) {
                eprintln!("⚠️  Package '{}' is not currently installed.", name);
            } else {
                names.push(name);
            }
        }
        if names.is_empty() {
            println!("✨ None of the specified packages are installed.");
            return Ok(());
        }
        names
    } else {
        // By default, target all explicitly installed packages (or all installed if none marked explicit).
        let explicit: Vec<PackageName> = installed_records
            .iter()
            .filter(|r| r.is_explicit)
            .map(|r| r.name.clone())
            .collect();

        if explicit.is_empty() {
            installed_records.iter().map(|r| r.name.clone()).collect()
        } else {
            explicit
        }
    };

    println!(
        "🔍 Resolving upgrades for {} root package(s)...",
        target_names.len()
    );

    // 2. Build root dependencies for the target packages
    let root_deps: Vec<Dependency> = target_names
        .iter()
        .map(|name| Dependency::new(name.clone(), VersionReq::STAR, DependencyKind::Runtime))
        .collect();

    // 3. Resolve the dependency graph (whole-graph with fallback)
    let plan = match solver.resolve(&root_deps) {
        Ok(plan) => plan,
        Err(err) => {
            eprintln!("⚠️  Whole-graph resolution encountered an issue: {}", err);
            eprintln!("🔄 Resolving upgradeable packages individually...");

            let mut combined = Vec::new();
            let mut seen = HashSet::new();

            for dep in &root_deps {
                match solver.resolve(std::slice::from_ref(dep)) {
                    Ok(single_plan) => {
                        for item in single_plan.into_iter() {
                            if seen.insert(item.manifest.name.clone()) {
                                combined.push(item);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("⚠️  Could not resolve '{}': {}", dep.name, e);
                    }
                }
            }

            if combined.is_empty() {
                println!("✨ No packages could be resolved for upgrade.");
                return Ok(());
            }

            ResolutionPlan::new(combined)
        }
    };

    // 4. Identify packages needing upgrade or new dependencies needing installation
    let mut actions = Vec::new();
    for item in plan.iter() {
        if item.is_system_provided {
            continue;
        }

        if let Some(installed) = db.get(&item.manifest.name) {
            if installed.version != item.manifest.version {
                actions.push(UpgradeAction {
                    name: item.manifest.name.clone(),
                    current_version: Some(installed.version.clone()),
                    target_version: item.manifest.version.clone(),
                    is_new_dependency: false,
                });
            }
        } else {
            actions.push(UpgradeAction {
                name: item.manifest.name.clone(),
                current_version: None,
                target_version: item.manifest.version.clone(),
                is_new_dependency: true,
            });
        }
    }

    if actions.is_empty() {
        println!("✨ Everything is up to date.");
        return Ok(());
    }

    println!("\n📦 {} package operation(s) required:\n", actions.len());
    for act in &actions {
        if act.is_new_dependency {
            println!(
                "  {:<20} (new dependency) -> v{}",
                act.name.as_str(),
                act.target_version
            );
        } else {
            println!(
                "  {:<20} v{} -> v{}",
                act.name.as_str(),
                act.current_version.as_ref().unwrap(),
                act.target_version
            );
        }
    }

    if dry_run {
        println!("\n🚀 Dry run — no changes made. Run without --dry-run to upgrade.");
        return Ok(());
    }

    println!();

    // 5. Execute upgrades in topological plan order (dependencies before dependents)
    for item in plan.iter() {
        if item.is_system_provided {
            continue;
        }

        let pkg_name = &item.manifest.name;
        if let Some(old_record) = db.get(pkg_name).cloned() {
            if old_record.version == item.manifest.version {
                continue;
            }

            println!(
                "⬆️  Upgrading '{}' v{} -> v{}...",
                pkg_name, old_record.version, item.manifest.version
            );

            if let Err(e) = rivet_package::uninstall(&old_record, cache_dir, prefix) {
                eprintln!(
                    "⚠️  uninstall hook for old '{}' failed: {}. Continuing anyway.",
                    pkg_name, e
                );
            }
            db.remove_package(pkg_name)?;

            rivet_package::install(
                &item.manifest,
                prefix,
                cache_dir,
                db,
                old_record.is_explicit,
            )?;
            println!("✅ '{}' upgraded to v{}.", pkg_name, item.manifest.version);
        } else {
            println!(
                "⬇️  Installing new dependency '{}' v{}...",
                pkg_name, item.manifest.version
            );
            rivet_package::install(&item.manifest, prefix, cache_dir, db, false)?;
            println!("✅ '{}' v{} installed.", pkg_name, item.manifest.version);
        }
    }

    println!("\n🚀 Upgrade complete.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rivet_core::{InstalledRecord, Version};
    use rivet_repository::LocalRepository;
    use tempfile::tempdir;

    #[test]
    fn test_upgrade_dry_run_and_execution() {
        let tmp = tempdir().unwrap();
        let db_path = tmp.path().join("db.json");
        let prefix = tmp.path().join("prefix");
        let cache_dir = tmp.path().join("cache");
        let repo_dir = tmp.path().join("repo");

        std::fs::create_dir_all(&prefix).unwrap();
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::create_dir_all(&repo_dir).unwrap();

        // 1. Write newer package recipes to repo_dir:
        // zlib 1.3.1 (was installed as 1.2.11)
        // ripgrep 14.1.0 (depends on zlib)
        std::fs::write(
            repo_dir.join("zlib.lua"),
            r#"
            package({
                name = "zlib",
                version = "1.3.1",
                install = function(ctx)
                    ctx:mkdir(ctx:destdir() .. "/lib")
                    ctx:write_file(ctx:destdir() .. "/lib/libz.so", "v1.3.1")
                end,
            })
            "#,
        )
        .unwrap();

        let mut local_repo = LocalRepository::open(&repo_dir, "test-repo");
        local_repo.scan_and_index().unwrap();

        let mut repos = MultiRepositoryManager::new();
        repos.add_local_repository(local_repo, 10, true);

        // 2. Setup installed database with older version
        let mut db = InstalledDatabase::open(&db_path).unwrap();
        let zlib_file = prefix.join("lib/libz.so");
        std::fs::create_dir_all(zlib_file.parent().unwrap()).unwrap();
        std::fs::write(&zlib_file, "v1.2.11").unwrap();

        let old_record = InstalledRecord::new(
            PackageName::new("zlib").unwrap(),
            Version::parse("1.2.11").unwrap(),
            Some("Old zlib".to_string()),
            vec![zlib_file.clone()],
            true,
            Some(r#"package({ name = "zlib", version = "1.2.11" })"#.to_string()),
            vec![],
            Some("test-repo".to_string()),
        );
        db.record_install(old_record).unwrap();

        // 3. Run dry run upgrade
        execute(&repos, &[], &mut db, &prefix, &cache_dir, true).unwrap();
        assert_eq!(
            db.get(&PackageName::new("zlib").unwrap()).unwrap().version,
            Version::parse("1.2.11").unwrap()
        );

        // 4. Run real upgrade
        execute(&repos, &[], &mut db, &prefix, &cache_dir, false).unwrap();
        assert_eq!(
            db.get(&PackageName::new("zlib").unwrap()).unwrap().version,
            Version::parse("1.3.1").unwrap()
        );

        // 5. Subsequent upgrade reports everything up to date
        execute(&repos, &[], &mut db, &prefix, &cache_dir, false).unwrap();
    }
}
