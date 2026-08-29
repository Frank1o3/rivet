use std::path::Path;

use rivet_package::PackageLoader;

pub fn execute(recipe_path: &Path, check_only: bool) -> anyhow::Result<()> {
    println!("📦 Loading recipe from '{}'...", recipe_path.display());

    let loader = PackageLoader::new()?;
    let manifest = loader.load_from_file(recipe_path)?;

    println!("✅ Valid package definition:");
    println!("  Name:         {}", manifest.name);
    println!("  Version:      {}", manifest.version);
    if let Some(desc) = &manifest.description {
        println!("  Description:  {}", desc);
    }
    if let Some(lic) = &manifest.license {
        println!("  License:      {}", lic);
    }
    if let Some(src) = &manifest.source {
        println!("  Source:       {:?}", src);
    }

    println!("  Dependencies: {} item(s)", manifest.dependencies.len());
    for dep in &manifest.dependencies {
        println!("    - {} ({:?})", dep, dep.kind);
    }

    if check_only {
        println!("\n🔍 Validation check passed successfully.");
    } else {
        println!("\n🔨 Build recipe verified and ready for execution.");
    }

    Ok(())
}
