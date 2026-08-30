use rivet_repository::MultiRepositoryManager;

pub fn execute(repos: &MultiRepositoryManager, query: &str) -> anyhow::Result<()> {
    let results = repos.search(query);

    if results.is_empty() {
        println!("No packages found matching query: '{}'", query);
        return Ok(());
    }

    println!("Found {} package(s) matching '{}':\n", results.len(), query);
    println!(
        "{:<24} {:<12} {:<12} {}",
        "PACKAGE", "VERSION", "REPOSITORY", "DESCRIPTION"
    );
    println!("{:-<80}", "");

    for pkg in results {
        let version = pkg.version.as_deref().unwrap_or("-");
        let desc = pkg.description.as_deref().unwrap_or("-");
        println!(
            "{:<24} {:<12} {:<12} {}",
            pkg.name, version, pkg.repository, desc
        );
    }

    Ok(())
}
