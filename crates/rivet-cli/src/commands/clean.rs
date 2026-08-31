use std::fs;
use std::path::Path;

pub fn execute(cache_dir: &Path) -> anyhow::Result<()> {
    println!("🧹 Cleaning cache directory '{}'...", cache_dir.display());

    if !cache_dir.exists() {
        println!("✨ Cache directory does not exist — nothing to clean.");
        return Ok(());
    }

    let mut removed_count = 0;
    for entry in fs::read_dir(cache_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            fs::remove_dir_all(&path)?;
            removed_count += 1;
        } else if path.is_file() {
            fs::remove_file(&path)?;
            removed_count += 1;
        }
    }

    println!(
        "✅ Cleaned up {} disposable cache item(s).",
        removed_count
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_clean_cache() {
        let tmp = tempdir().unwrap();
        let cache_dir = tmp.path().join("cache");
        fs::create_dir_all(&cache_dir).unwrap();

        fs::write(cache_dir.join("temp.tar.gz"), "archive data").unwrap();
        let sub = cache_dir.join("git-repo");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("file.rs"), "source").unwrap();

        assert!(cache_dir.join("temp.tar.gz").exists());
        assert!(sub.exists());

        execute(&cache_dir).unwrap();

        assert!(cache_dir.exists());
        assert_eq!(fs::read_dir(&cache_dir).unwrap().count(), 0);
    }
}
