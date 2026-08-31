use rivet_core::PackageName;
use rivet_package::PackageManifest;
use rivet_resolver::PackageProvider;

use crate::error::Result;
use crate::local::LocalRepository;
use crate::remote::RemoteRepository;
use crate::summary::PackageSummary;

/// The two ways a configured repository can be backed.
#[derive(Debug, Clone)]
pub enum RepositoryBackend {
    Local(LocalRepository),
    Remote(RemoteRepository),
}

/// Entry holding a repository instance and its priority.
#[derive(Debug, Clone)]
pub struct RepositoryEntry {
    pub backend: RepositoryBackend,
    pub priority: i32,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct RemoteUpdateResult {
    pub slug: String,
    pub outcome: std::result::Result<usize, String>,
}

/// Manages multiple local- or remote-backed repositories and provides a
/// unified `PackageProvider`.
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

    pub fn entries(&self) -> &[RepositoryEntry] {
        &self.repositories
    }

    pub fn add_local_repository(&mut self, repo: LocalRepository, priority: i32, enabled: bool) {
        self.insert(RepositoryBackend::Local(repo), priority, enabled);
    }

    pub fn add_remote_repository(&mut self, repo: RemoteRepository, priority: i32, enabled: bool) {
        self.insert(RepositoryBackend::Remote(repo), priority, enabled);
    }

    fn insert(&mut self, backend: RepositoryBackend, priority: i32, enabled: bool) {
        self.repositories.push(RepositoryEntry {
            backend,
            priority,
            enabled,
        });

        // Sort by priority descending (higher priority first)
        self.repositories
            .sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub fn scan_all(&mut self) -> Result<usize> {
        let mut total = 0;
        for entry in &mut self.repositories {
            if !entry.enabled {
                continue;
            }
            match &mut entry.backend {
                RepositoryBackend::Local(repo) => total += repo.scan_and_index()?,
                RepositoryBackend::Remote(repo) => {
                    repo.load_index()?;
                    total += repo.entry_count();
                }
            }
        }
        Ok(total)
    }

    pub fn search(&self, query: &str) -> Vec<PackageSummary> {
        let mut results = Vec::new();
        for entry in &self.repositories {
            if !entry.enabled {
                continue;
            }
            match &entry.backend {
                RepositoryBackend::Local(repo) => {
                    results.extend(
                        repo.index
                            .search(query)
                            .into_iter()
                            .map(|m| PackageSummary {
                                name: m.name.as_str().to_string(),
                                version: Some(m.version.to_string()),
                                description: m.description.clone(),
                                repository: repo.name.clone(),
                            }),
                    );
                }
                RepositoryBackend::Remote(repo) => {
                    results.extend(repo.search(query).into_iter().map(|e| PackageSummary {
                        name: e.name,
                        version: e.version,
                        description: e.description,
                        repository: repo.slug.clone(),
                    }));
                }
            }
        }
        results
    }

    pub fn update_remotes(&mut self) -> Vec<RemoteUpdateResult> {
        let mut results = Vec::new();

        for entry in &self.repositories {
            if !entry.enabled {
                continue;
            }
            let RepositoryBackend::Remote(repo) = &entry.backend else {
                continue;
            };

            let outcome = repo
                .sync_mirror()
                .and_then(|()| {
                    repo.invalidate_index();
                    repo.load_index()
                })
                .map(|()| repo.entry_count())
                .map_err(|e| e.to_string());

            results.push(RemoteUpdateResult {
                slug: repo.slug.clone(),
                outcome,
            });
        }

        results
    }

    pub fn repository_still_provides(&self, repo_slug: &str, name: &PackageName) -> Option<bool> {
        for entry in &self.repositories {
            if !entry.enabled {
                continue;
            }
            if let RepositoryBackend::Remote(repo) = &entry.backend {
                if repo.slug == repo_slug {
                    return Some(!repo.get_candidates(name).is_empty());
                }
            }
        }
        None
    }
}

impl PackageProvider for MultiRepositoryManager {
    fn get_candidates(&self, name: &PackageName) -> Vec<PackageManifest> {
        let mut candidates = Vec::new();
        for entry in &self.repositories {
            if !entry.enabled {
                continue;
            }
            match &entry.backend {
                RepositoryBackend::Local(repo) => candidates.extend(repo.get_candidates(name)),
                RepositoryBackend::Remote(repo) => candidates.extend(repo.get_candidates(name)),
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

        fs::write(
            dir1.path().join("gcc.lua"),
            r#"package({ name = "gcc", version = "14.1.0" })"#,
        )
        .unwrap();

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
        multi.add_local_repository(repo1, 10, true);
        multi.add_local_repository(repo2, 5, true);

        let gcc = multi.get_candidates(&PackageName::new("gcc").unwrap());
        assert_eq!(gcc.len(), 1);

        let rg = multi.get_candidates(&PackageName::new("ripgrep").unwrap());
        assert_eq!(rg.len(), 1);
    }
}
