use rivet_repository::MultiRepositoryManager;

pub fn execute(repos: &mut MultiRepositoryManager) -> anyhow::Result<()> {
    println!("🔄 Refreshing local package repositories...");
    let indexed = repos.scan_all()?;
    println!(
        "✅ Refresh complete. {} package definition(s) currently available.",
        indexed
    );
    println!(
        "(Rescans local files and reloads cached repository indexes — no network access. \
         Run 'rivet update' to fetch new data from remote repositories.)"
    );
    Ok(())
}
