use rivet_core::InstalledDatabase;

pub fn execute(db: &InstalledDatabase) -> anyhow::Result<()> {
    let installed = db.list_installed();

    if installed.is_empty() {
        println!("No packages currently installed.");
        return Ok(());
    }

    println!("📦 Installed Packages ({} total):\n", installed.len());
    println!("{:<24} {:<12} {:<12} {}", "PACKAGE", "VERSION", "TYPE", "FILES");
    println!("{:-<70}", "");

    for record in installed {
        let install_type = if record.is_explicit { "explicit" } else { "dependency" };
        println!(
            "{:<24} {:<12} {:<12} {} file(s)",
            record.name.as_str(),
            record.version.to_string(),
            install_type,
            record.installed_files.len()
        );
    }

    Ok(())
}
