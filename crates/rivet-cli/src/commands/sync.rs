use rivet_repository::MultiRepositoryManager;

pub fn execute(repos: &mut MultiRepositoryManager) -> anyhow::Result<()> {
    println!("🔄 Synchronizing repositories...");
    let indexed = repos.scan_all()?;
    println!("✅ Sync complete. Indexed {} package definitions.", indexed);
    Ok(())
}
