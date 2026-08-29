use std::collections::HashSet;
use std::path::Path;

use rivet_core::{Feature, InstalledDatabase, Target};
use rivet_package::{Dependency, DependencyKind};
use rivet_repository::MultiRepositoryManager;
use rivet_resolver::DependencySolver;

pub fn execute(
    repos: &MultiRepositoryManager,
    package_args: &[String],
    dry_run: bool,
    features: &[String],
    db: &mut InstalledDatabase,
    prefix: &Path,
    cache_dir: &Path,
) -> anyhow::Result<()> {
    println!(
        "🔍 Resolving dependencies for {} package(s)...",
        package_args.len()
    );

    let mut root_deps = Vec::new();
    for arg in package_args {
        let mut dep = Dependency::parse_shorthand(arg, DependencyKind::Runtime)?;
        for feat in features {
            dep = dep.with_feature(Feature::new(feat)?);
        }
        root_deps.push(dep);
    }

    let target = Target::host();
    let solver = DependencySolver::new(repos, &target);
    let plan = solver.resolve(&root_deps)?;

    if plan.is_empty() {
        println!("✨ Nothing to install.");
        return Ok(());
    }

    println!("\n📦 Resolved Installation Plan ({} packages):", plan.len());
    println!("{:-<60}", "");

    for (i, item) in plan.iter().enumerate() {
        let build_deps_str = if item.build_dependencies.is_empty() {
            String::new()
        } else {
            format!(
                " [build-deps: {}]",
                item.build_dependencies
                    .iter()
                    .map(|d| d.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };

        let features_str = if item.enabled_features.is_empty() {
            String::new()
        } else {
            format!(
                " (+features: {})",
                item.enabled_features
                    .iter()
                    .map(|f| f.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };

        println!(
            " {:2}. {:<20} {:<10}{}{}",
            i + 1,
            item.manifest.name.as_str(),
            item.manifest.version.to_string(),
            features_str,
            build_deps_str
        );
    }

    println!("{:-<60}", "");

    if dry_run {
        println!("🚀 Dry run complete. No changes were made to the system.");
        return Ok(());
    }

    // Only packages the user asked for by name are "explicit" — everything
    // else in the plan was pulled in purely to satisfy a dependency.
    let explicit: HashSet<_> = root_deps.iter().map(|d| d.name.clone()).collect();

    for item in plan.iter() {
        if db.is_installed(&item.manifest.name) {
            println!(
                "⏭️  '{}' is already installed, skipping.",
                item.manifest.name
            );
            continue;
        }
        if item.is_system_provided {
            println!(
                "🖥️  '{}' is already provided by the system, skipping.",
                item.manifest.name
            );
            continue;
        }

        println!(
            "⬇️  Installing '{}' v{}...",
            item.manifest.name, item.manifest.version
        );
        let is_explicit = explicit.contains(&item.manifest.name);
        rivet_package::install(&item.manifest, prefix, cache_dir, db, is_explicit)?;
        println!(
            "✅ '{}' v{} installed.",
            item.manifest.name, item.manifest.version
        );
    }

    println!("🚀 Installation complete.");
    Ok(())
}
