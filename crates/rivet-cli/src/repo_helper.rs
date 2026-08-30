use std::path::Path;

use rivet_repository::{
    LocalRepository, MultiRepositoryManager, RemoteRepository, RepositoryLoader,
};


const LOCAL_REPO_PRIORITY: i32 = 1000;
const REMOTE_REPO_PRIORITY: i32 = 10;

pub fn load_repositories(explicit_path: Option<&Path>) -> anyhow::Result<MultiRepositoryManager> {
    let mut multi = MultiRepositoryManager::new();

    if let Some(path) = explicit_path {
        let mut repo = LocalRepository::open(path, "custom");
        repo.scan_and_index()?;
        multi.add_local_repository(repo, LOCAL_REPO_PRIORITY, true);
        return Ok(multi);
    }

    let local_dir = rivet_core::default_local_packages_dir()?;
    let mut local_repo = LocalRepository::open(&local_dir, "local");
    let _ = local_repo.scan_and_index();
    multi.add_local_repository(local_repo, LOCAL_REPO_PRIORITY, true);

    load_remote_repositories(&mut multi)?;

    Ok(multi)
}

fn load_remote_repositories(multi: &mut MultiRepositoryManager) -> anyhow::Result<()> {
    let repositories_dir = rivet_core::default_repositories_dir()?;
    if !repositories_dir.exists() {
        return Ok(());
    }

    let loader = RepositoryLoader::new()?;
    let mirror_root = rivet_core::default_repo_mirror_cache()?;
    let packages_root = rivet_core::default_packages_dir()?;

    for entry in std::fs::read_dir(&repositories_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("lua") {
            continue;
        }

        let Some(slug) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };

        let definition = match loader.load_from_file(&path) {
            Ok(def) => def,
            Err(e) => {
                eprintln!(
                    "⚠️  failed to load repository definition '{}': {e}",
                    path.display()
                );
                continue;
            }
        };

        let packages_dir = packages_root.join(slug);
        let repo = RemoteRepository::new(slug, definition, &mirror_root, packages_dir);

        // Best-effort, same reasoning as the local repo above: an
        // un-synced repository (no mirror cloned yet) just contributes
        // nothing until `rivet update` runs.
        let _ = repo.load_index();

        multi.add_remote_repository(repo, REMOTE_REPO_PRIORITY, true);
    }

    Ok(())
}
