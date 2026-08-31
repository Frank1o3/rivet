use rivet_core::{InstalledDatabase, PackageName};
use rivet_repository::MultiRepositoryManager;
use rivet_resolver::PackageProvider;

pub fn execute(
    repos: &MultiRepositoryManager,
    db: &InstalledDatabase,
    package_name_str: &str,
) -> anyhow::Result<()> {
    let name = PackageName::new(package_name_str)?;
    let candidates = repos.get_candidates(&name);

    if candidates.is_empty() {
        if let Some(record) = db.get(&name) {
            println!("Package:      {}", record.name);
            println!("Version:      {} (installed)", record.version);
            if let Some(desc) = &record.description {
                println!("Description:  {}", desc);
            }
            println!(
                "Installed:    {} file(s), {}",
                record.installed_files.len(),
                if record.is_explicit {
                    "explicit"
                } else {
                    "dependency"
                }
            );
            if let Some(source) = &record.source_repository {
                println!("Source repo:  {}", source);
            }

            println!();
            println!(
                "⚠️  '{}' is no longer listed in any currently configured repository{}.",
                record.name,
                record
                    .source_repository
                    .as_deref()
                    .map(|s| format!(" (originally installed from '{}')", s))
                    .unwrap_or_default()
            );
            println!(
                "    It may have been removed, replaced, or deprecated upstream — worth investigating why."
            );
            return Ok(());
        }

        println!(
            "Package '{}' not found in available repositories.",
            package_name_str
        );
        return Ok(());
    }

    for manifest in candidates {
        println!("Package:      {}", manifest.name);
        println!("Version:      {}", manifest.version);
        if let Some(desc) = &manifest.description {
            println!("Description:  {}", desc);
        }
        if let Some(license) = &manifest.license {
            println!("License:      {}", license);
        }
        if let Some(homepage) = &manifest.homepage {
            println!("Homepage:     {}", homepage);
        }
        if let Some(publisher) = &manifest.publisher {
            println!("Publisher:    {}", publisher);
        }

        if let Some(source) = &manifest.source {
            println!("Source:       {:?}", source);
        }

        if !manifest.dependencies.is_empty() {
            println!("Dependencies:");
            for dep in &manifest.dependencies {
                println!("  - {} ({:?})", dep, dep.kind);
            }
        }

        if !manifest.features.is_empty() {
            println!("Features:");
            for (feat, deps) in &manifest.features {
                let is_default = manifest.default_features.contains(feat);
                let flag = if is_default { " [default]" } else { "" };
                println!("  - {}{}", feat, flag);
                for d in deps {
                    println!("      requires: {}", d);
                }
            }
        }

        if !manifest.supported_architectures.is_empty() {
            println!(
                "Arch:         {}",
                manifest
                    .supported_architectures
                    .iter()
                    .map(|a| a.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        if !manifest.supported_os.is_empty() {
            println!(
                "OS:           {}",
                manifest
                    .supported_os
                    .iter()
                    .map(|o| o.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        println!();
    }

    Ok(())
}
