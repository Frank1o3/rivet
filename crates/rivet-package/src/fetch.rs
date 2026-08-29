//! Fetches upstream package sources for non-local source types.
//!
//! Git repositories are cloned into the package cache.
//! Archive sources are downloaded, verified, and extracted into the package cache.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

use flate2::read::GzDecoder;
use rivet_core::Checksum;
use xz2::read::XzDecoder;

use crate::error::{PackageError, Result};
use crate::source::GitRef;

fn run_git(cwd: Option<&Path>, args: &[&str]) -> Result<()> {
    let mut cmd = Command::new("git");

    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    let status = cmd.args(args).status().map_err(|e| {
        PackageError::SourceFetch(format!("failed to run 'git {}': {e}", args.join(" ")))
    })?;

    if !status.success() {
        return Err(PackageError::SourceFetch(format!(
            "'git {}' exited with {status}",
            args.join(" ")
        )));
    }

    Ok(())
}

pub fn fetch_git(url: &str, reference: Option<&GitRef>, cache_dir: &Path) -> Result<PathBuf> {
    let repo_dir = cache_dir.join("git").join(sanitize_url(url));

    if repo_dir.join(".git").exists() {
        println!("  [source] updating git repository '{}'...", url);
        run_git(Some(&repo_dir), &["fetch", "--all", "--tags"])?;
    } else {
        println!("  [source] cloning git repository '{}'...", url);

        if let Some(parent) = repo_dir.parent() {
            fs::create_dir_all(parent).map_err(PackageError::Io)?;
        }

        let dest = repo_dir.to_string_lossy().to_string();
        run_git(None, &["clone", url, &dest])?;
    }

    if let Some(reference) = reference {
        let target = match reference {
            GitRef::Tag(t) | GitRef::Branch(t) | GitRef::Commit(t) => t.as_str(),
        };

        run_git(Some(&repo_dir), &["checkout", "--force", target])?;
    }

    Ok(repo_dir)
}

/// Downloads and extracts an archive source.
///
/// The returned path is the directory that should be used as the source
/// directory by the build hooks.
pub fn fetch_archive(url: &str, checksum: &Checksum, cache_dir: &Path) -> Result<PathBuf> {
    let archive_id = sanitize_url(url);
    let archive_dir = cache_dir.join("archives").join(&archive_id);
    let archive_path = archive_dir.join("source.archive");
    let extract_dir = archive_dir.join("source");

    fs::create_dir_all(&archive_dir).map_err(PackageError::Io)?;

    // If we've already extracted this exact archive URL and its expected
    // checksum was verified during the previous fetch, reuse it.
    //
    // The checksum marker also prevents us from reusing a stale extraction
    // if the same URL was given a different checksum.
    let checksum_marker = archive_dir.join(format!(
        ".checksum-{}",
        sanitize_filename(checksum.hex_value())
    ));

    if extract_dir.exists() && checksum_marker.exists() {
        println!("  [source] using cached archive '{}'", url);
        return find_source_root(&extract_dir);
    }

    println!("  [source] downloading archive '{}'...", url);

    let response = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(None)
        .build()
        .map_err(|e| PackageError::SourceFetch(format!("failed to create HTTP client: {e}")))?
        .get(url)
        .send()
        .map_err(|e| PackageError::SourceFetch(format!("failed to download '{}': {e}", url)))?;

    if !response.status().is_success() {
        return Err(PackageError::SourceFetch(format!(
            "failed to download '{}': HTTP {}",
            url,
            response.status()
        )));
    }

    let bytes = response.bytes().map_err(|e| {
        eprintln!("Reqwest error: {e}");
        eprintln!("Debug: {e:?}");

        PackageError::SourceFetch(format!("failed to read downloaded archive '{}': {e}", url))
    })?;

    checksum.verify(&bytes).map_err(|e| {
        PackageError::SourceFetch(format!("checksum verification failed for '{}': {e}", url))
    })?;

    // Write only after checksum verification succeeds.
    fs::write(&archive_path, &bytes).map_err(PackageError::Io)?;

    if extract_dir.exists() {
        fs::remove_dir_all(&extract_dir).map_err(PackageError::Io)?;
    }

    fs::create_dir_all(&extract_dir).map_err(PackageError::Io)?;

    extract_archive(&archive_path, &extract_dir, url)?;

    fs::write(&checksum_marker, checksum.to_string()).map_err(PackageError::Io)?;

    find_source_root(&extract_dir)
}

/// Extract an archive according to its URL/extension.
fn extract_archive(archive_path: &Path, output_dir: &Path, url: &str) -> Result<()> {
    let filename = url_filename(url)
        .unwrap_or_else(|| archive_path.to_string_lossy().to_string())
        .to_ascii_lowercase();

    if filename.ends_with(".tar.gz") || filename.ends_with(".tgz") {
        let file = File::open(archive_path).map_err(PackageError::Io)?;
        let decoder = GzDecoder::new(file);

        unpack_tar(decoder, output_dir)
    } else if filename.ends_with(".tar.xz") || filename.ends_with(".txz") {
        let file = File::open(archive_path).map_err(PackageError::Io)?;
        let decoder = XzDecoder::new(file);

        unpack_tar(decoder, output_dir)
    } else if filename.ends_with(".tar.bz2") || filename.ends_with(".tbz2") {
        let file = File::open(archive_path).map_err(PackageError::Io)?;
        let decoder = bzip2::read::BzDecoder::new(file);

        unpack_tar(decoder, output_dir)
    } else if filename.ends_with(".tar") {
        let file = File::open(archive_path).map_err(PackageError::Io)?;

        unpack_tar(file, output_dir)
    } else if filename.ends_with(".zip") {
        extract_zip(archive_path, output_dir)
    } else {
        Err(PackageError::SourceFetch(format!(
            "unsupported archive format for '{}'; \
             supported formats are .tar, .tar.gz, .tgz, .tar.xz, \
             .txz, .tar.bz2, .tbz2, and .zip",
            url
        )))
    }
}

fn unpack_tar<R: Read>(reader: R, output_dir: &Path) -> Result<()> {
    let mut archive = tar::Archive::new(reader);

    for entry in archive
        .entries()
        .map_err(|e| PackageError::SourceFetch(format!("failed to read tar archive: {e}")))?
    {
        let mut entry = entry
            .map_err(|e| PackageError::SourceFetch(format!("failed to read tar entry: {e}")))?;

        // `entry.path()` borrows from `entry`, so make an owned copy
        // before mutably borrowing `entry` with `unpack_in()`.
        let entry_path = entry
            .path()
            .map_err(|e| PackageError::SourceFetch(format!("failed to read tar entry path: {e}")))?
            .to_path_buf();

        validate_archive_path(&entry_path)?;

        entry.unpack_in(output_dir).map_err(|e| {
            PackageError::SourceFetch(format!(
                "failed to extract '{}' from tar archive: {e}",
                entry_path.display()
            ))
        })?;
    }

    Ok(())
}

/// Extract ZIP while preventing path traversal.
fn extract_zip(archive_path: &Path, output_dir: &Path) -> Result<()> {
    let file = File::open(archive_path).map_err(PackageError::Io)?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| PackageError::SourceFetch(format!("failed to open zip archive: {e}")))?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|e| {
            PackageError::SourceFetch(format!("failed to read zip entry {index}: {e}"))
        })?;

        let raw_name = entry.name().to_string();
        let entry_path = Path::new(&raw_name);

        validate_archive_path(entry_path)?;

        let output_path = output_dir.join(entry_path);

        if entry.is_dir() || raw_name.ends_with('/') {
            fs::create_dir_all(&output_path).map_err(|e| {
                PackageError::SourceFetch(format!(
                    "failed to create directory '{}': {e}",
                    output_path.display()
                ))
            })?;

            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(PackageError::Io)?;
        }

        let mut output = File::create(&output_path).map_err(|e| {
            PackageError::SourceFetch(format!("failed to create '{}': {e}", output_path.display()))
        })?;

        io::copy(&mut entry, &mut output).map_err(|e| {
            PackageError::SourceFetch(format!(
                "failed to extract '{}' from zip archive: {e}",
                raw_name
            ))
        })?;
    }

    Ok(())
}

/// Ensure an archive entry cannot escape its extraction directory.
fn validate_archive_path(path: &Path) -> Result<()> {
    if path.is_absolute() {
        return Err(PackageError::SourceFetch(format!(
            "archive contains absolute path '{}'",
            path.display()
        )));
    }

    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(PackageError::SourceFetch(format!(
                "archive contains path traversal entry '{}'",
                path.display()
            )));
        }
    }

    Ok(())
}

/// Most source archives contain a single top-level directory.
///
/// If there is exactly one directory and nothing else at the root, return it.
/// Otherwise return the extraction directory itself.
fn find_source_root(extract_dir: &Path) -> Result<PathBuf> {
    let mut entries = fs::read_dir(extract_dir).map_err(PackageError::Io)?;

    let first = match entries.next() {
        None => {
            return Err(PackageError::SourceFetch(
                "archive extracted to an empty directory".to_string(),
            ));
        }
        Some(entry) => entry.map_err(PackageError::Io)?,
    };

    if entries.next().is_none() && first.path().is_dir() {
        Ok(first.path())
    } else {
        Ok(extract_dir.to_path_buf())
    }
}

/// Get the final URL path component without query/fragment data.
fn url_filename(url: &str) -> Option<String> {
    let without_fragment = url.split('#').next()?;
    let without_query = without_fragment.split('?').next()?;

    without_query
        .rsplit('/')
        .next()
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

/// Turns a URL into a filesystem-safe directory name.
fn sanitize_url(url: &str) -> String {
    url.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// Sanitizes a value used as part of a cache filename.
fn sanitize_filename(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
