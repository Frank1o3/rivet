use std::path::{Path, PathBuf};

use rivet_repository::{LocalRepository, MultiRepositoryManager};

/// Resolves and loads local repositories based on user flags or standard search directories.
pub fn load_repositories(explicit_path: Option<&Path>) -> anyhow::Result<MultiRepositoryManager> {
    let mut multi = MultiRepositoryManager::new();

    if let Some(path) = explicit_path {
        let mut repo = LocalRepository::open(path, "custom");
        repo.scan_and_index()?;
        multi.add_local_repository(repo, 10, true);
        return Ok(multi);
    }

    // Default search locations
    let candidates = [
        PathBuf::from("packages"),
        PathBuf::from("recipes"),
        PathBuf::from("."),
    ];

    let mut found = false;
    for candidate in &candidates {
        if candidate.exists() && candidate.is_dir() {
            let mut repo = LocalRepository::open(candidate, "local");
            if let Ok(count) = repo.scan_and_index() {
                if count > 0 {
                    multi.add_local_repository(repo, 10, true);
                    found = true;
                    break;
                }
            }
        }
    }

    if !found {
        // Fallback: create an empty local repo in current dir
        let repo = LocalRepository::open(".", "local");
        multi.add_local_repository(repo, 10, true);
    }

    Ok(multi)
}
