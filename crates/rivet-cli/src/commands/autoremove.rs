use std::path::Path;

use rivet_core::InstalledDatabase;

pub fn execute(
    db: &mut InstalledDatabase,
    cache_dir: &Path,
    prefix: &Path,
    dry_run: bool,
) -> anyhow::Result<()> {
    let orphans = db.find_orphans();

    if orphans.is_empty() {
        println!("✨ No orphaned packages found — system is clean.");
        return Ok(());
    }

    println!("🧹 Found {} orphaned package(s):\n", orphans.len());
    for orphan in &orphans {
        println!(
            "  {:<20} v{} ({} file(s))",
            orphan.name.as_str(),
            orphan.version,
            orphan.installed_files.len()
        );
    }

    if dry_run {
        println!("\n🚀 Dry run — no packages removed. Run without --dry-run to autoremove.");
        return Ok(());
    }

    println!();
    let orphan_names: Vec<_> = orphans.iter().map(|o| o.name.clone()).collect();
    let mut removed_count = 0;
    let mut total_files_cleaned = 0;

    for name in orphan_names {
        if let Some(record) = db.get(&name).cloned() {
            println!("🗑️  Removing orphan '{}' v{}...", record.name, record.version);

            if let Err(e) = rivet_package::uninstall(&record, cache_dir, prefix) {
                eprintln!(
                    "⚠️  uninstall hook for '{}' failed: {}. Continuing with file removal anyway.",
                    record.name, e
                );
            }

            if let Some(removed) = db.remove_package(&name)? {
                total_files_cleaned += removed.installed_files.len();
                removed_count += 1;
            }
        }
    }

    println!(
        "\n✅ Successfully removed {} orphaned package(s) (cleaned up {} tracked file(s)).",
        removed_count, total_files_cleaned
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rivet_core::{InstalledRecord, PackageName, Version};
    use tempfile::tempdir;

    #[test]
    fn test_autoremove_dry_run_and_execution() {
        let tmp = tempdir().unwrap();
        let db_file = tmp.path().join("db.json");
        let prefix = tmp.path().join("prefix");
        let cache_dir = tmp.path().join("cache");
        std::fs::create_dir_all(&prefix).unwrap();
        std::fs::create_dir_all(&cache_dir).unwrap();

        let mut db = InstalledDatabase::open(&db_file).unwrap();
        let libz = PackageName::new("zlib").unwrap();
        let orphan_file = prefix.join("libz.so");
        std::fs::write(&orphan_file, "zlib").unwrap();

        // Installed as a dependency (non-explicit), but no other package depends on it
        db.record_install(InstalledRecord::new(
            libz.clone(),
            Version::parse("1.3.1").unwrap(),
            None,
            vec![orphan_file.clone()],
            false,
            None,
            vec![],
            None,
        ))
        .unwrap();

        // 1. Dry run autoremove
        execute(&mut db, &cache_dir, &prefix, true).unwrap();
        assert!(db.is_installed(&libz));
        assert!(orphan_file.exists());

        // 2. Real autoremove
        execute(&mut db, &cache_dir, &prefix, false).unwrap();
        assert!(!db.is_installed(&libz));
        assert!(!orphan_file.exists());

        // 3. Subsequent autoremove is a no-op
        execute(&mut db, &cache_dir, &prefix, false).unwrap();
    }
}
