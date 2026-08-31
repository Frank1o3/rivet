use std::collections::HashSet;

use rivet_core::InstalledDatabase;
use rivet_repository::MultiRepositoryManager;

pub fn execute(repos: &mut MultiRepositoryManager, db: &InstalledDatabase) -> anyhow::Result<()> {
    println!("🌐 Fetching latest package information from remote repositories...");

    let results = repos.update_remotes();

    if results.is_empty() {
        println!("No remote repositories configured — nothing to update.");
        return Ok(());
    }

    let mut had_failure = false;
    let mut updated_ok: HashSet<&str> = HashSet::new();

    for result in &results {
        match &result.outcome {
            Ok(count) => {
                println!("  ✅ {:<20} {} package(s) indexed", result.slug, count);
                updated_ok.insert(result.slug.as_str());
            }
            Err(e) => {
                had_failure = true;
                eprintln!("  ⚠️  {:<20} failed: {}", result.slug, e);
            }
        }
    }

    if had_failure {
        println!(
            "\n⚠️  Update completed with errors — some repositories may be showing stale data."
        );
    } else {
        println!("\n✅ All repositories up to date.");
    }

    let mut removed_upstream = Vec::new();
    for record in db.list_installed() {
        let Some(source) = record.source_repository.as_deref() else {
            continue;
        };
        if !updated_ok.contains(source) {
            continue;
        }
        if repos.repository_still_provides(source, &record.name) == Some(false) {
            removed_upstream.push((record.name.as_str().to_string(), source.to_string()));
        }
    }

    if !removed_upstream.is_empty() {
        println!();
        for (name, source) in &removed_upstream {
            println!(
                "⚠️  '{}' (installed from '{}') is no longer listed there — it may have been \
                 removed, replaced, or deprecated upstream. Worth investigating why.",
                name, source
            );
        }
    }

    println!(
        "\nRun 'rivet upgrade' to see if any installed packages have newer versions available."
    );

    Ok(())
}
