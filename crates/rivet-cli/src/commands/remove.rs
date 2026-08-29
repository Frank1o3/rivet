use std::path::Path;

use rivet_core::{InstalledDatabase, PackageName};

pub fn execute(
    db: &mut InstalledDatabase,
    package_name_str: &str,
    cache_dir: &Path,
    prefix: &Path,
) -> anyhow::Result<()> {
    let name = PackageName::new(package_name_str)?;

    let Some(record) = db.get(&name).cloned() else {
        println!("Package '{}' is not installed.", name);
        return Ok(());
    };

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
