use rivet_repository::MultiRepositoryManager;

pub fn execute(repos: &mut MultiRepositoryManager) -> anyhow::Result<()> {
    println!("🌐 Fetching latest package information from remote repositories...");

    let results = repos.update_remotes();

    if results.is_empty() {
        println!("No remote repositories configured — nothing to update.");
        return Ok(());
    }

    let mut had_failure = false;
    for result in &results {
        match &result.outcome {
            Ok(count) => println!("  ✅ {:<20} {} package(s) indexed", result.slug, count),
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

    println!("Run 'rivet upgrade' to see if any installed packages have newer versions available.");

    Ok(())
}
