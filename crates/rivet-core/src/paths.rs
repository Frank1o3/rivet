use std::path::PathBuf;

/// Convert a path to an absolute path.
pub fn absolute_path(path: PathBuf) -> anyhow::Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path);
    }

    Ok(std::env::current_dir()?.join(path))
}

/// Root of Rivet's persistent data directory: `repositories/`,
/// `packages/`, and (by default) `db.json` all live under here.
///
/// Override with `RIVET_DATA`.
pub fn default_data_dir() -> anyhow::Result<PathBuf> {
    let path = if let Ok(path) = std::env::var("RIVET_DATA") {
        PathBuf::from(path)
    } else {
        dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("could not determine home directory"))?
            .join(".rivet")
    };

    absolute_path(path)
}

/// Directory containing repository pointer definitions
/// (`~/.rivet/repositories/*.lua`).
pub fn default_repositories_dir() -> anyhow::Result<PathBuf> {
    Ok(default_data_dir()?.join("repositories"))
}

/// Directory containing materialized package definitions, one
/// subdirectory per repository slug, plus the fixed `local/` pseudo-repo.
pub fn default_packages_dir() -> anyhow::Result<PathBuf> {
    Ok(default_data_dir()?.join("packages"))
}

/// The `local/` pseudo-repository: hand-authored package definitions
/// that aren't backed by any remote repository.
pub fn default_local_packages_dir() -> anyhow::Result<PathBuf> {
    Ok(default_packages_dir()?.join("local"))
}

/// Directory containing disposable, blob-less Git mirrors of remote
/// repositories — cache, not persistent state, hence nested under
/// `default_source_cache()` rather than `default_data_dir()`.
pub fn default_repo_mirror_cache() -> anyhow::Result<PathBuf> {
    Ok(default_source_cache()?.join("repos"))
}

/// Default location for cached package sources.
///
/// Override with `RIVET_CACHE`.
pub fn default_source_cache() -> anyhow::Result<PathBuf> {
    let path = if let Ok(path) = std::env::var("RIVET_CACHE") {
        PathBuf::from(path)
    } else {
        dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("could not determine home directory"))?
            .join(".cache")
            .join("rivet")
    };

    absolute_path(path)
}

/// Default database location.
///
/// Override with `RIVET_DB`.
pub fn default_path() -> anyhow::Result<PathBuf> {
    let path = if let Ok(path) = std::env::var("RIVET_DB") {
        PathBuf::from(path)
    } else {
        default_data_dir()?.join("db.json")
    };

    absolute_path(path)
}

/// Default installation prefix.
///
/// Override with `RIVET_PREFIX`.
///
/// For a normal user installation this defaults to `$HOME/.local`.
pub fn default_prefix() -> anyhow::Result<PathBuf> {
    let path = if let Ok(path) = std::env::var("RIVET_PREFIX") {
        PathBuf::from(path)
    } else {
        dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("could not determine home directory"))?
            .join(".local")
    };

    absolute_path(path)
}
