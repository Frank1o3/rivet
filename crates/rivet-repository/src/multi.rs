use rivet_core::PackageName;
use rivet_package::PackageManifest;
use rivet_resolver::PackageProvider;

use crate::error::Result;
use crate::local::LocalRepository;

/// Entry holding a repository instance and its priority.
#[derive(Debug, Clone)]
pub struct RepositoryEntry {
    pub repo: LocalRepository,
    pub priority: i32,
    pub enabled: bool,
}

/// Manages multiple local or remote repositories and provides a unified `PackageProvider`.
#[derive(Debug, Default, Clone)]
pub struct MultiRepositoryManager {
    repositories: Vec<RepositoryEntry>,
}

impl MultiRepositoryManager {
    pub fn new() -> Self {
        Self {
            repositories: Vec::new(),
        }
    }

    /// Adds a repository to the manager.
    pub fn add_repository(&mut self, repo: LocalRepository, priority: i32, enabled: bool) {
        self.repositories.push(RepositoryEntry {
            repo,
            priority,
            enabled,
        });

        // Sort by priority descending (higher priority first)
        self.repositories
            .sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Scans and indexes all enabled repositories.
    pub fn scan_all(&mut self) -> Result<usize> {
        let mut total = 0;
        for entry in &mut self.repositories {
            if entry.enabled {
                total += entry.repo.scan_and_index()?;
            }
        }
        Ok(total)
    }

    /// Searches across all enabled repositories.
    pub fn search(&self, query: &str) -> Vec<&PackageManifest> {
        let mut results = Vec::new();
        for entry in &self.repositories {
            if entry.enabled {
                results.extend(entry.repo.index.search(query));
            }
        }
        results
    }
}

impl PackageProvider for MultiRepositoryManager {
    fn get_candidates(&self, name: &PackageName) -> Vec<PackageManifest> {
        let mut candidates = Vec::new();
        for entry in &self.repositories {
            if entry.enabled {
                if let Some(list) = entry.repo.index.get(name) {
                    candidates.extend(list.clone());
                }
            }
        }
        candidates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_multi_repository_priority() {
        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();

        // Repo 1: core (priority 10)
        fs::write(
            dir1.path().join("gcc.lua"),
            r#"package({ name = "gcc", version = "14.1.0" })"#,
        )
        .unwrap();

        // Repo 2: community (priority 5)
        fs::write(
            dir2.path().join("ripgrep.lua"),
            r#"package({ name = "ripgrep", version = "14.1.0" })"#,
        )
        .unwrap();

        let mut repo1 = LocalRepository::open(dir1.path(), "core");
        let mut repo2 = LocalRepository::open(dir2.path(), "community");
        repo1.scan_and_index().unwrap();
        repo2.scan_and_index().unwrap();

        let mut multi = MultiRepositoryManager::new();
        multi.add_repository(repo1, 10, true);
        multi.add_repository(repo2, 5, true);

        let gcc = multi.get_candidates(&PackageName::new("gcc").unwrap());
        assert_eq!(gcc.len(), 1);

        let rg = multi.get_candidates(&PackageName::new("ripgrep").unwrap());
        assert_eq!(rg.len(), 1);
    }
}
