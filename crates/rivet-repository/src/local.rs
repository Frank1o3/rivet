use std::path::{Path, PathBuf};

use rivet_core::PackageName;
use rivet_package::{PackageLoader, PackageManifest};
use rivet_resolver::PackageProvider;
use walkdir::WalkDir;

use crate::error::Result;
use crate::index::RepositoryIndex;

/// A local filesystem repository containing `.lua` package definitions.
#[derive(Debug, Clone)]
pub struct LocalRepository {
    pub name: String,
    pub root_path: PathBuf,
    pub index: RepositoryIndex,
}

impl LocalRepository {
    /// Opens or initializes a local repository at the given directory.
    pub fn open(root_path: impl AsRef<Path>, name: impl Into<String>) -> Self {
        let name = name.into();
        let root_path = root_path.as_ref().to_path_buf();
        let index = RepositoryIndex::new(name.clone());

        Self {
            name,
            root_path,
            index,
        }
    }

    /// Scans the repository directory recursively, parses all `.lua` package recipes, and populates the index.
    pub fn scan_and_index(&mut self) -> Result<usize> {
        if !self.root_path.exists() {
            return Ok(0);
        }

        let loader = PackageLoader::new()?;
        let mut count = 0;

        for entry in WalkDir::new(&self.root_path)
            .follow_links(true)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("lua") {
                if let Ok(manifest) = loader.load_from_file(path) {
                    self.index.add(manifest);
                    count += 1;
                }
            }
        }

        Ok(count)
    }

    /// Saves the current in-memory index to a cached JSON file.
    pub fn save_index(&self, cache_file: impl AsRef<Path>) -> Result<()> {
        self.index.save_to_file(cache_file)
    }

    /// Loads a pre-built index from a cached JSON file.
    pub fn load_index(&mut self, cache_file: impl AsRef<Path>) -> Result<()> {
        self.index = RepositoryIndex::load_from_file(cache_file)?;
        Ok(())
    }
}

impl PackageProvider for LocalRepository {
    fn get_candidates(&self, name: &PackageName) -> Vec<PackageManifest> {
        self.index.get(name).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_local_repository_scan_and_index() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create packages/zlib/zlib.lua
        let zlib_dir = root.join("packages/zlib");
        fs::create_dir_all(&zlib_dir).unwrap();
        fs::write(
            zlib_dir.join("zlib.lua"),
            r#"package({ name = "zlib", version = "1.3.1", description = "compression" })"#,
        )
        .unwrap();

        // Create packages/libpng.lua
        let libpng_file = root.join("packages/libpng.lua");
        fs::write(
            libpng_file,
            r#"package({ name = "libpng", version = "1.6.43", dependencies = { "zlib" } })"#,
        )
        .unwrap();

        let mut repo = LocalRepository::open(root, "test-repo");
        let indexed = repo.scan_and_index().unwrap();
        assert_eq!(indexed, 2);

        assert_eq!(repo.index.len(), 2);
        let zlib = repo.index.get(&PackageName::new("zlib").unwrap()).unwrap();
        assert_eq!(zlib[0].version.to_string(), "1.3.1");

        // Test search
        let search_results = repo.index.search("compression");
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0].name.as_str(), "zlib");
    }
}
