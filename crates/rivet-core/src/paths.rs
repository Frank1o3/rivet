use std::path::PathBuf;

/// Convert a path to an absolute path.
pub fn absolute_path(path: PathBuf) -> anyhow::Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path);
    }

    Ok(std::env::current_dir()?.join(path))
}

/// Default location for cached package sources.
/// Override with `RIVET_CACHE`.
pub fn default_source_cache() -> anyhow::Result<PathBuf> {
    let path = if let Ok(path) = std::env::var("RIVET_CACHE") {
        PathBuf::from(path)
    } else {
        dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("could not determine home directory"))?
            .join(".rivet")
            .join("cache")
    };

    absolute_path(path)
}

/// Default database location for the current user/system.
/// Override with `RIVET_DB`.
pub fn default_path() -> anyhow::Result<PathBuf> {
    let path = if let Ok(path) = std::env::var("RIVET_DB") {
        PathBuf::from(path)
    } else {
        dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("could not determine home directory"))?
            .join(".rivet")
            .join("db.json")
    };

    absolute_path(path)
}

/// Default installation prefix for the current user/system.
/// Override with `RIVET_PREFIX`.
pub fn default_prefix() -> anyhow::Result<PathBuf> {
    let path = if let Ok(path) = std::env::var("RIVET_PREFIX") {
        PathBuf::from(path)
    } else {
        dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("could not determine home directory"))?
            .join(".rivet")
            .join("store")
    };

    absolute_path(path)
}
