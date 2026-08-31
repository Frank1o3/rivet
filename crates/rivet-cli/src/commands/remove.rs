use std::path::Path;

use rivet_core::{InstalledDatabase, PackageName};

pub fn execute(
    db: &mut InstalledDatabase,
    package_name_str: &str,
    cache_dir: &Path,
    prefix: &Path,
    force: bool,
) -> anyhow::Result<()> {
    let name = PackageName::new(package_name_str)?;

    let Some(record) = db.get(&name).cloned() else {
        println!("Package '{}' is not installed.", name);
        return Ok(());
    };

    // Check for reverse dependencies
    let rev_deps = db.reverse_dependencies(&name);
    if !rev_deps.is_empty() {
        if !force {
            eprintln!(
                "❌ Cannot remove '{}' because {} installed package(s) depend on it:",
                name,
                rev_deps.len()
            );
            for dep in &rev_deps {
                eprintln!("  • {} (v{})", dep.name, dep.version);
            }
            eprintln!("\nUse --force (-f) to remove anyway (may break dependent packages).");
            anyhow::bail!("Removal aborted: package has dependent reverse dependencies");
        }

        eprintln!(
            "⚠️  Forcing removal of '{}' despite dependent package(s): {} (--force specified).",
            name,
            rev_deps
                .iter()
                .map(|d| d.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    println!("🗑️  Removing package '{}'...", name);

    if let Err(e) = rivet_package::uninstall(&record, cache_dir, prefix) {
        eprintln!(
            "⚠️  uninstall hook for '{}' failed: {}. Continuing with file removal anyway.",
            name, e
        );
    }

    if let Some(record) = db.remove_package(&name)? {
        println!(
            "✅ Successfully removed '{}' v{} (cleaned up {} tracked file(s)).",
            record.name,
            record.version,
            record.installed_files.len()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rivet_core::{InstalledRecord, RecordedDependency, Version};
    use tempfile::tempdir;

    #[test]
    fn test_remove_reverse_dependency_safety_and_force() {
        let tmp = tempdir().unwrap();
        let db_file = tmp.path().join("db.json");
        let prefix = tmp.path().join("prefix");
        let cache_dir = tmp.path().join("cache");
        std::fs::create_dir_all(&prefix).unwrap();
        std::fs::create_dir_all(&cache_dir).unwrap();

        let mut db = InstalledDatabase::open(&db_file).unwrap();
        let libz = PackageName::new("zlib").unwrap();
        let neovim = PackageName::new("neovim").unwrap();

        let libz_file = prefix.join("libz.so");
        std::fs::write(&libz_file, "libz").unwrap();

        db.record_install(InstalledRecord::new(
            libz.clone(),
            Version::parse("1.3.1").unwrap(),
            None,
            vec![libz_file.clone()],
            false,
            None,
            vec![],
            None,
        ))
        .unwrap();

        db.record_install(InstalledRecord::new(
            neovim.clone(),
            Version::parse("0.10.0").unwrap(),
            None,
            vec![],
            true,
            None,
            vec![RecordedDependency {
                name: libz.clone(),
                runtime: true,
            }],
            None,
        ))
        .unwrap();

        // 1. Attempting to remove zlib without force should fail
        let result = execute(&mut db, "zlib", &cache_dir, &prefix, false);
        assert!(result.is_err());
        assert!(db.is_installed(&libz));
        assert!(libz_file.exists());

        // 2. Removing with force should succeed
        let result_force = execute(&mut db, "zlib", &cache_dir, &prefix, true);
        assert!(result_force.is_ok());
        assert!(!db.is_installed(&libz));
        assert!(!libz_file.exists());
    }
}
