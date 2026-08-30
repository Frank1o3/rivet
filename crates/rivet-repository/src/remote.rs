use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use rivet_core::{Checksum, PackageName};
use rivet_package::{PackageLoader, PackageManifest};
use rivet_resolver::PackageProvider;
use serde::{Deserialize, Serialize};

use crate::definition::RepositoryDefinition;
use crate::error::{RepositoryError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteIndexEntry {
    pub path: String,
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub sha: RemoteIndexChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteIndexChecksum {
    pub sha256: String,
    pub sha512: String,
}

#[derive(Debug, Clone)]
pub struct RemoteRepository {
    pub slug: String,
    pub definition: RepositoryDefinition,
    mirror_dir: PathBuf,
    packages_dir: PathBuf,
    index: RefCell<Option<Vec<RemoteIndexEntry>>>,
}

impl RemoteRepository {
    pub fn new(
        slug: impl Into<String>,
        definition: RepositoryDefinition,
        mirror_root: &Path,
        packages_dir: PathBuf,
    ) -> Self {
        let slug = slug.into();
        let mirror_dir = mirror_root.join(&slug);
        Self {
            slug,
            definition,
            mirror_dir,
            packages_dir,
            index: RefCell::new(None),
        }
    }

    pub fn sync_mirror(&self) -> Result<()> {
        let branch = &self.definition.source.branch;

        if self.mirror_dir.join("HEAD").exists() {
            run_git(
                Some(&self.mirror_dir),
                &[
                    "fetch",
                    "--force",
                    "--depth",
                    "1",
                    "origin",
                    &format!("{branch}:{branch}"),
                ],
            )?;
        } else {
            if let Some(parent) = self.mirror_dir.parent() {
                fs::create_dir_all(parent).map_err(RepositoryError::Io)?;
            }
            let dest = self.mirror_dir.to_string_lossy().to_string();
            run_git(
                None,
                &[
                    "clone",
                    "--bare",
                    "--filter=blob:none",
                    "--depth",
                    "1",
                    "--single-branch",
                    "--branch",
                    branch,
                    &self.definition.source.url,
                    &dest,
                ],
            )?;
        }

        Ok(())
    }

    fn repo_relative(&self, path: &str) -> String {
        match &self.definition.source.path {
            Some(p) if !p.is_empty() => format!("{}/{}", p.trim_end_matches('/'), path),
            _ => path.to_string(),
        }
    }

    pub fn entry_count(&self) -> usize {
        self.index.borrow().as_ref().map(Vec::len).unwrap_or(0)
    }

    pub fn load_index(&self) -> Result<()> {
        if self.index.borrow().is_some() {
            return Ok(());
        }

        let raw = self.git_show(&self.repo_relative("index.json"))?;
        let entries: Vec<RemoteIndexEntry> = serde_json::from_slice(&raw)?;
        *self.index.borrow_mut() = Some(entries);
        Ok(())
    }

    pub fn search(&self, query: &str) -> Vec<RemoteIndexEntry> {
        let query = query.to_lowercase();
        self.index
            .borrow()
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter(|e| {
                e.name.to_lowercase().contains(&query)
                    || e.description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&query))
                        .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    fn materialize_or_reuse(&self, entry: &RemoteIndexEntry) -> Result<PackageManifest> {
        let filename = Path::new(&entry.path).file_name().ok_or_else(|| {
            RepositoryError::InvalidConfig(format!(
                "index entry path '{}' has no filename component",
                entry.path
            ))
        })?;
        let dest = self.packages_dir.join(filename);

        if dest.exists() {
            let existing = fs::read(&dest).map_err(RepositoryError::Io)?;
            let checksum: Checksum = format!("sha256:{}", entry.sha.sha256).parse()?;
            if checksum.verify(&existing).is_ok() {
                let loader = PackageLoader::new()?;
                return Ok(loader.load_from_file(&dest)?);
            }
        }

        self.fetch_and_write(entry, &dest)
    }

    fn fetch_and_write(&self, entry: &RemoteIndexEntry, dest: &Path) -> Result<PackageManifest> {
        let relative = self.repo_relative(&entry.path);
        let bytes = self.git_show(&relative)?;

        let checksum: Checksum = format!("sha256:{}", entry.sha.sha256).parse()?;
        checksum
            .verify(&bytes)
            .map_err(|e| RepositoryError::SyncFailed {
                name: self.slug.clone(),
                reason: format!(
                    "'{}' failed integrity check against published index: {e}",
                    entry.path
                ),
            })?;

        fs::create_dir_all(&self.packages_dir).map_err(RepositoryError::Io)?;
        fs::write(dest, &bytes).map_err(RepositoryError::Io)?;

        let loader = PackageLoader::new()?;
        Ok(loader.load_from_file(dest)?)
    }

    fn git_show(&self, repo_relative_path: &str) -> Result<Vec<u8>> {
        let branch = &self.definition.source.branch;
        run_git_capture(
            Some(&self.mirror_dir),
            &["show", &format!("{branch}:{repo_relative_path}")],
        )
    }
}

impl PackageProvider for RemoteRepository {
    fn get_candidates(&self, name: &PackageName) -> Vec<PackageManifest> {
        let Some(index) = self.index.borrow().clone() else {
            eprintln!(
                "[{}] index not loaded — call load_index() before resolving",
                self.slug
            );
            return Vec::new();
        };

        index
            .iter()
            .filter(|e| e.name == name.as_str())
            .filter_map(|entry| match self.materialize_or_reuse(entry) {
                Ok(manifest) => Some(manifest),
                Err(e) => {
                    eprintln!("[{}] failed to fetch '{}': {e}", self.slug, entry.path);
                    None
                }
            })
            .collect()
    }
}

fn run_git(cwd: Option<&Path>, args: &[&str]) -> Result<()> {
    run_git_capture(cwd, args).map(|_| ())
}

fn run_git_capture(cwd: Option<&Path>, args: &[&str]) -> Result<Vec<u8>> {
    let mut cmd = Command::new("git");
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    let output = cmd
        .args(args)
        .output()
        .map_err(|e| RepositoryError::SyncFailed {
            name: "git".to_string(),
            reason: format!("failed to run 'git {}': {e}", args.join(" ")),
        })?;

    if !output.status.success() {
        return Err(RepositoryError::SyncFailed {
            name: "git".to_string(),
            reason: format!(
                "'git {}' exited with {}: {}",
                args.join(" "),
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ),
        });
    }

    Ok(output.stdout)
}
