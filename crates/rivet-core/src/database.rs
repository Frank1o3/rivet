use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json;

use crate::error::Result;
use crate::package_name::PackageName;
use crate::version::Version;

/// Record of an installed package on the host system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledRecord {
    pub name: PackageName,
    pub version: Version,
    pub description: Option<String>,
    pub installed_files: Vec<PathBuf>,
    pub installed_at: u64,
    pub is_explicit: bool,
    /// A snapshot of the recipe script at install time, so `uninstall`
    /// hooks can still run even if the originating repository no longer
    /// has this package (or has a different version of it) by the time
    /// the package is removed.
    #[serde(default)]
    pub recipe_snapshot: Option<String>,
}

impl InstalledRecord {
    pub fn new(
        name: PackageName,
        version: Version,
        description: Option<String>,
        installed_files: Vec<PathBuf>,
        is_explicit: bool,
        recipe_snapshot: Option<String>,
    ) -> Self {
        let installed_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            name,
            version,
            description,
            installed_files,
            installed_at,
            is_explicit,
            recipe_snapshot,
        }
    }
}

/// Local database tracking all installed packages and their file manifests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledDatabase {
    #[serde(skip)]
    db_path: PathBuf,
    packages: HashMap<PackageName, InstalledRecord>,
}

impl InstalledDatabase {
    /// Opens or creates an installed package database at the given file path.
    pub fn open(db_path: impl AsRef<Path>) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();
        if db_path.exists() {
            let content = fs::read_to_string(&db_path)?;
            let mut db: InstalledDatabase = serde_json::from_str(&content)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            db.db_path = db_path;
            Ok(db)
        } else {
            Ok(Self {
                db_path,
                packages: HashMap::new(),
            })
        }
    }

    /// Checks if a package is installed.
    pub fn is_installed(&self, name: &PackageName) -> bool {
        self.packages.contains_key(name)
    }

    /// Retrieves an installed package record by name.
    pub fn get(&self, name: &PackageName) -> Option<&InstalledRecord> {
        self.packages.get(name)
    }

    /// Lists all installed package records.
    pub fn list_installed(&self) -> Vec<&InstalledRecord> {
        let mut list: Vec<&InstalledRecord> = self.packages.values().collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    /// Records or updates an installed package entry and saves to disk.
    pub fn record_install(&mut self, record: InstalledRecord) -> Result<()> {
        self.packages.insert(record.name.clone(), record);
        self.save()
    }

    /// Removes an installed package entry and deletes its tracked files from disk.
    pub fn remove_package(&mut self, name: &PackageName) -> Result<Option<InstalledRecord>> {
        if let Some(record) = self.packages.remove(name) {
            for file_path in &record.installed_files {
                if file_path.exists() && file_path.is_file() {
                    let _ = fs::remove_file(file_path);
                }
            }
            self.save()?;
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    /// Saves the database to disk.
    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.db_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&self.db_path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_installed_database_lifecycle() {
        let tmp = tempdir().unwrap();
        let db_file = tmp.path().join("db.json");

        // 1. Create files
        let file1 = tmp.path().join("bin/my-bin");
        fs::create_dir_all(tmp.path().join("bin")).unwrap();
        fs::write(&file1, "binary").unwrap();

        let mut db = InstalledDatabase::open(&db_file).unwrap();
        let name = PackageName::new("my-tool").unwrap();
        let version = Version::parse("1.0.0").unwrap();

        let record = InstalledRecord::new(
            name.clone(),
            version,
            Some("A test tool".to_string()),
            vec![file1.clone()],
            true,
            None, // recipe_snapshot
        );

        db.record_install(record).unwrap();
        assert!(db.is_installed(&name));
        assert_eq!(db.list_installed().len(), 1);

        // Reload DB from file
        let mut db2 = InstalledDatabase::open(&db_file).unwrap();
        assert!(db2.is_installed(&name));

        // Remove package
        let removed = db2.remove_package(&name).unwrap();
        assert!(removed.is_some());
        assert!(!db2.is_installed(&name));
        assert!(!file1.exists()); // Tracked file was safely deleted
    }
}
