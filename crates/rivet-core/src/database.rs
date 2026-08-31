use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json;

use crate::error::Result;
use crate::package_name::PackageName;
use crate::version::Version;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedDependency {
    pub name: PackageName,
    pub runtime: bool,
}

/// Record of an installed package on the host system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledRecord {
    pub name: PackageName,
    pub version: Version,
    pub description: Option<String>,
    pub installed_files: Vec<PathBuf>,
    pub installed_at: u64,
    pub is_explicit: bool,
    #[serde(default)]
    pub recipe_snapshot: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<RecordedDependency>,
    #[serde(default)]
    pub source_repository: Option<String>,
}

impl InstalledRecord {
    pub fn new(
        name: PackageName,
        version: Version,
        description: Option<String>,
        installed_files: Vec<PathBuf>,
        is_explicit: bool,
        recipe_snapshot: Option<String>,
        dependencies: Vec<RecordedDependency>,
        source_repository: Option<String>,
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
            dependencies,
            source_repository,
        }
    }

    /// Checks that all tracked installed files still exist on the filesystem.
    pub fn verify_files(&self) -> PackageVerificationResult {
        let mut missing_files = Vec::new();
        for file in &self.installed_files {
            if !file.exists() && !file.is_symlink() {
                missing_files.push(file.clone());
            }
        }
        PackageVerificationResult {
            name: self.name.clone(),
            version: self.version.clone(),
            total_files: self.installed_files.len(),
            missing_files,
        }
    }
}

/// Result of an integrity check on an installed package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageVerificationResult {
    pub name: PackageName,
    pub version: Version,
    pub total_files: usize,
    pub missing_files: Vec<PathBuf>,
}

impl PackageVerificationResult {
    pub fn is_intact(&self) -> bool {
        self.missing_files.is_empty()
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
                if file_path.is_symlink() || file_path.is_file() || file_path.exists() {
                    let _ = fs::remove_file(file_path);
                }
            }
            self.save()?;
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    /// Returns all installed packages that declare a runtime dependency on the given package.
    pub fn reverse_dependencies(&self, name: &PackageName) -> Vec<&InstalledRecord> {
        let mut dependents = Vec::new();
        for record in self.packages.values() {
            if record
                .dependencies
                .iter()
                .any(|dep| dep.runtime && &dep.name == name)
            {
                dependents.push(record);
            }
        }
        dependents.sort_by(|a, b| a.name.cmp(&b.name));
        dependents
    }

    /// Finds all orphaned installed packages (non-explicit packages that are no longer
    /// required directly or transitively by any explicitly installed package).
    pub fn find_orphans(&self) -> Vec<&InstalledRecord> {
        use std::collections::{HashSet, VecDeque};

        let mut reachable: HashSet<&PackageName> = HashSet::new();
        let mut queue: VecDeque<&PackageName> = VecDeque::new();

        for record in self.packages.values() {
            if record.is_explicit {
                reachable.insert(&record.name);
                queue.push_back(&record.name);
            }
        }

        while let Some(current_name) = queue.pop_front() {
            if let Some(record) = self.packages.get(current_name) {
                for dep in &record.dependencies {
                    if dep.runtime && self.packages.contains_key(&dep.name) {
                        if reachable.insert(&dep.name) {
                            queue.push_back(&dep.name);
                        }
                    }
                }
            }
        }

        let mut orphans: Vec<&InstalledRecord> = self
            .packages
            .values()
            .filter(|record| !reachable.contains(&record.name))
            .collect();

        orphans.sort_by(|a, b| a.name.cmp(&b.name));
        orphans
    }

    /// Verifies that all tracked files for a specific package exist on disk.
    pub fn verify_package(&self, name: &PackageName) -> Option<PackageVerificationResult> {
        self.packages.get(name).map(|r| r.verify_files())
    }

    /// Verifies tracked files for all installed packages.
    pub fn verify_all(&self) -> Vec<PackageVerificationResult> {
        let mut results: Vec<_> = self.packages.values().map(|r| r.verify_files()).collect();
        results.sort_by(|a, b| a.name.cmp(&b.name));
        results
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
            None,
            vec![],
            Some("local".to_string()),
        );

        db.record_install(record).unwrap();
        assert!(db.is_installed(&name));
        assert_eq!(db.list_installed().len(), 1);
        assert_eq!(
            db.get(&name).unwrap().source_repository.as_deref(),
            Some("local")
        );

        let mut db2 = InstalledDatabase::open(&db_file).unwrap();
        assert!(db2.is_installed(&name));

        let removed = db2.remove_package(&name).unwrap();
        assert!(removed.is_some());
        assert!(!db2.is_installed(&name));
        assert!(!file1.exists());
    }

    #[test]
    fn test_installed_database_reverse_dependencies() {
        let tmp = tempdir().unwrap();
        let db_file = tmp.path().join("db.json");
        let mut db = InstalledDatabase::open(&db_file).unwrap();

        let libz = PackageName::new("zlib").unwrap();
        let libpng = PackageName::new("libpng").unwrap();
        let neovim = PackageName::new("neovim").unwrap();

        db.record_install(InstalledRecord::new(
            libz.clone(),
            Version::parse("1.3.1").unwrap(),
            None,
            vec![],
            false,
            None,
            vec![],
            None,
        ))
        .unwrap();

        db.record_install(InstalledRecord::new(
            libpng.clone(),
            Version::parse("1.6.43").unwrap(),
            None,
            vec![],
            false,
            None,
            vec![RecordedDependency {
                name: libz.clone(),
                runtime: true,
            }],
            None,
        ))
        .unwrap();

        db.record_install(InstalledRecord::new(
            neovim.clone(),
            Version::parse("0.10.0").unwrap(),
            None,
            vec![],
            true,
            None,
            vec![
                RecordedDependency {
                    name: libz.clone(),
                    runtime: true,
                },
                RecordedDependency {
                    name: libpng.clone(),
                    runtime: true,
                },
            ],
            None,
        ))
        .unwrap();

        let zlib_rev_deps = db.reverse_dependencies(&libz);
        assert_eq!(zlib_rev_deps.len(), 2);
        let names: Vec<&str> = zlib_rev_deps.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["libpng", "neovim"]);

        let neovim_rev_deps = db.reverse_dependencies(&neovim);
        assert!(neovim_rev_deps.is_empty());
    }

    #[test]
    fn test_installed_database_find_orphans() {
        let tmp = tempdir().unwrap();
        let db_file = tmp.path().join("db.json");
        let mut db = InstalledDatabase::open(&db_file).unwrap();

        let libz = PackageName::new("zlib").unwrap();
        let libpng = PackageName::new("libpng").unwrap();
        let unused_lib = PackageName::new("unused-lib").unwrap();
        let neovim = PackageName::new("neovim").unwrap();

        // libz: dependency (required by libpng)
        db.record_install(InstalledRecord::new(
            libz.clone(),
            Version::parse("1.3.1").unwrap(),
            None,
            vec![],
            false,
            None,
            vec![],
            None,
        ))
        .unwrap();

        // libpng: dependency (required by neovim)
        db.record_install(InstalledRecord::new(
            libpng.clone(),
            Version::parse("1.6.43").unwrap(),
            None,
            vec![],
            false,
            None,
            vec![RecordedDependency {
                name: libz.clone(),
                runtime: true,
            }],
            None,
        ))
        .unwrap();

        // unused_lib: dependency (not required by anything)
        db.record_install(InstalledRecord::new(
            unused_lib.clone(),
            Version::parse("1.0.0").unwrap(),
            None,
            vec![],
            false,
            None,
            vec![],
            None,
        ))
        .unwrap();

        // neovim: explicit package (requires libpng)
        db.record_install(InstalledRecord::new(
            neovim.clone(),
            Version::parse("0.10.0").unwrap(),
            None,
            vec![],
            true,
            None,
            vec![RecordedDependency {
                name: libpng.clone(),
                runtime: true,
            }],
            None,
        ))
        .unwrap();

        let orphans = db.find_orphans();
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].name.as_str(), "unused-lib");

        // Now remove neovim: libpng, libz, and unused_lib should all be orphans
        db.remove_package(&neovim).unwrap();
        let new_orphans = db.find_orphans();
        assert_eq!(new_orphans.len(), 3);
        let orphan_names: Vec<&str> = new_orphans.iter().map(|o| o.name.as_str()).collect();
        assert_eq!(orphan_names, vec!["libpng", "unused-lib", "zlib"]);
    }

    #[test]
    fn test_installed_database_verification() {
        let tmp = tempdir().unwrap();
        let db_file = tmp.path().join("db.json");
        let mut db = InstalledDatabase::open(&db_file).unwrap();

        let intact_file = tmp.path().join("bin/intact");
        let missing_file = tmp.path().join("bin/missing");
        fs::create_dir_all(tmp.path().join("bin")).unwrap();
        fs::write(&intact_file, "exists").unwrap();

        let pkg1 = PackageName::new("pkg-intact").unwrap();
        let pkg2 = PackageName::new("pkg-damaged").unwrap();

        db.record_install(InstalledRecord::new(
            pkg1.clone(),
            Version::parse("1.0.0").unwrap(),
            None,
            vec![intact_file.clone()],
            true,
            None,
            vec![],
            None,
        ))
        .unwrap();

        db.record_install(InstalledRecord::new(
            pkg2.clone(),
            Version::parse("2.0.0").unwrap(),
            None,
            vec![intact_file.clone(), missing_file.clone()],
            true,
            None,
            vec![],
            None,
        ))
        .unwrap();

        let v1 = db.verify_package(&pkg1).unwrap();
        assert!(v1.is_intact());
        assert_eq!(v1.missing_files.len(), 0);

        let v2 = db.verify_package(&pkg2).unwrap();
        assert!(!v2.is_intact());
        assert_eq!(v2.missing_files, vec![missing_file.clone()]);

        let all = db.verify_all();
        assert_eq!(all.len(), 2);
        // verify_all sorts by name; "pkg-damaged" < "pkg-intact" alphabetically
        assert_eq!(all[0].name.as_str(), "pkg-damaged");
        assert!(!all[0].is_intact());
        assert_eq!(all[1].name.as_str(), "pkg-intact");
        assert!(all[1].is_intact());
    }
}
