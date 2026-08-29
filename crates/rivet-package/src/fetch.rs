//! Fetches upstream package sources for source types that aren't `local`.
//! Currently only `git` is implemented — `archive` still errors out in
//! `installer::resolve_source`.

use std::path::{Path, PathBuf};
use std::process::Command;

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
            std::fs::create_dir_all(parent).map_err(PackageError::Io)?;
        }
        // No `current_dir` here — `dest` is already a complete path
        // (relative to the process's real cwd, or absolute), so setting
        // `current_dir` too would make git resolve it a second time,
        // nesting the clone one directory too deep.
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

/// Turns a URL into a filesystem-safe directory name. Not collision-proof
/// against pathological URLs, but fine for the common case.
fn sanitize_url(url: &str) -> String {
    url.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}
