use rivet_repository::MultiRepositoryManager;

pub fn execute(repos: &MultiRepositoryManager, query: &str) -> anyhow::Result<()> {
    let results = repos.search(query);

    if results.is_empty() {
        println!("No packages found matching query: '{}'", query);
        return Ok(());
    }

    println!("Found {} package(s) matching '{}':\n", results.len(), query);
    println!("{:<24} {:<12} {}", "PACKAGE", "VERSION", "DESCRIPTION");
    println!("{:-<70}", "");

    for manifest in results {
        let desc = manifest.description.as_deref().unwrap_or("-");
        println!(
            "{:<24} {:<12} {}",
            manifest.name.as_str(),
            manifest.version.to_string(),
            desc
        );
    }

    Ok(())
}
