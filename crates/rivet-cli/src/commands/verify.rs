use rivet_core::{InstalledDatabase, PackageName, PackageVerificationResult};

pub fn execute(db: &InstalledDatabase, package_names: &[String]) -> anyhow::Result<()> {
    let installed_records = db.list_installed();
    if installed_records.is_empty() {
        println!("No packages currently installed.");
        return Ok(());
    }

    let results: Vec<PackageVerificationResult> = if package_names.is_empty() {
        db.verify_all()
    } else {
        let mut list = Vec::new();
        for name_str in package_names {
            let pkg_name = PackageName::new(name_str)?;
            if let Some(res) = db.verify_package(&pkg_name) {
                list.push(res);
            } else {
                anyhow::bail!("package '{}' is not installed", name_str);
            }
        }
        list
    };

    println!("🔍 Verifying installed package files ({} package(s))...\n", results.len());

    let mut intact_count = 0;
    let mut damaged_count = 0;
    let mut total_missing_files = 0;

    for result in &results {
        if result.is_intact() {
            intact_count += 1;
            println!(
                "  ● {:<20} v{:<10} ({} file(s)) - OK",
                result.name.as_str(),
                result.version.to_string(),
                result.total_files
            );
        } else {
            damaged_count += 1;
            total_missing_files += result.missing_files.len();
            println!(
                "  ✖ {:<20} v{:<10} ({} file(s)) - {} MISSING FILE(S)",
                result.name.as_str(),
                result.version.to_string(),
                result.total_files,
                result.missing_files.len()
            );
            for missing in &result.missing_files {
                println!("      - {}", missing.display());
            }
        }
    }

    println!("\nVerification Summary:");
    println!("  Total packages checked: {}", results.len());
    println!("  Intact packages:        {}", intact_count);
    println!("  Damaged packages:       {}", damaged_count);
    if total_missing_files > 0 {
        println!("  Total missing files:    {}", total_missing_files);
    }

    if damaged_count > 0 {
        anyhow::bail!(
            "Verification failed: {} package(s) have missing files",
            damaged_count
        );
    }

    println!("\n✨ All verified packages are intact.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rivet_core::{InstalledRecord, Version};
    use tempfile::tempdir;

    #[test]
    fn test_verify_all_and_specific() {
        let tmp = tempdir().unwrap();
        let db_file = tmp.path().join("db.json");
        let mut db = InstalledDatabase::open(&db_file).unwrap();

        let file1 = tmp.path().join("bin/tool1");
        let file2 = tmp.path().join("bin/tool2");
        std::fs::create_dir_all(tmp.path().join("bin")).unwrap();
        std::fs::write(&file1, "tool1").unwrap();

        let p1 = PackageName::new("pkg1").unwrap();
        let p2 = PackageName::new("pkg2").unwrap();

        db.record_install(InstalledRecord::new(
            p1.clone(),
            Version::parse("1.0.0").unwrap(),
            None,
            vec![file1.clone()],
            true,
            None,
            vec![],
            None,
        ))
        .unwrap();

        // pkg2 points to file2 which was not created (missing)
        db.record_install(InstalledRecord::new(
            p2.clone(),
            Version::parse("2.0.0").unwrap(),
            None,
            vec![file2.clone()],
            true,
            None,
            vec![],
            None,
        ))
        .unwrap();

        // 1. Verify specific intact package
        let ok_res = execute(&db, &[String::from("pkg1")]);
        assert!(ok_res.is_ok());

        // 2. Verify specific damaged package
        let bad_res = execute(&db, &[String::from("pkg2")]);
        assert!(bad_res.is_err());

        // 3. Verify all
        let all_res = execute(&db, &[]);
        assert!(all_res.is_err());

        // 4. Verify uninstalled package
        let uninstalled_res = execute(&db, &[String::from("nonexistent")]);
        assert!(uninstalled_res.is_err());
    }
}
