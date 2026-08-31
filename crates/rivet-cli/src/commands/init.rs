use std::fs;

use rivet_repository::RepositoryLoader;

/// Rivet's own repository URL, inherited from the workspace `repository`
/// field in `Cargo.toml` at compile time — not duplicated as a second
/// hardcoded literal here.
const OWN_REPO_URL: &str = env!("CARGO_PKG_REPOSITORY");

pub fn execute() -> anyhow::Result<()> {
    let data_dir = rivet_core::default_data_dir()?;
    let repositories_dir = rivet_core::default_repositories_dir()?;
    let local_packages_dir = rivet_core::default_local_packages_dir()?;

    fs::create_dir_all(&repositories_dir)?;
    fs::create_dir_all(&local_packages_dir)?;

    println!("📁 Rivet data directory ready at '{}'.", data_dir.display());

    let official_repo_path = repositories_dir.join("rivet.lua");
    if official_repo_path.exists() {
        println!(
            "  '{}' already exists — leaving it as-is.",
            official_repo_path.display()
        );
        return Ok(());
    }

    let Some(url) = default_repo_source_url() else {
        println!(
            "⚠️  Could not derive the official repository URL from this build \
             (unexpected repository URL shape). Add one manually with a file under '{}'.",
            repositories_dir.display()
        );
        return Ok(());
    };

    let contents = default_repository_lua(&url);

    // Sanity-check what we're about to write before it's trusted as a
    // real repository definition — catches a malformed template at
    // `rivet init` time instead of silently producing a broken pointer
    // file that only fails later, confusingly, during `rivet update`.
    let loader = RepositoryLoader::new()?;
    loader.load_from_str(&contents)?;

    fs::write(&official_repo_path, &contents)?;
    println!(
        "✅ Configured the official Rivet repository at '{}'.",
        official_repo_path.display()
    );
    println!("Run 'rivet update' to fetch its package index.");

    Ok(())
}

/// Derives the curated `rivet-repo` URL from this crate's own
/// `repository` field, by convention (`.../rivet.git` -> `.../rivet-repository.git`),
/// instead of hardcoding the org/user name a second time.
fn default_repo_source_url() -> Option<String> {
    OWN_REPO_URL
        .strip_suffix("/rivet.git")
        .map(|base| format!("{base}/rivet-repository.git"))
}

fn default_repository_lua(url: &str) -> String {
    format!(
        r#"repository({{
    name = "Rivet",
    description = "Official curated Rivet package repository",
    license = "BSD-3-Clause",

    source = {{
        url = "{url}",
        branch = "main",
        path = "src",
    }},
}})
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_repository_lua_parses() {
        let contents = default_repository_lua("https://github.com/Frank1o3/rivet-repository.git");
        let loader = RepositoryLoader::new().unwrap();
        let def = loader.load_from_str(&contents).unwrap();
        assert_eq!(def.name, "Rivet");
        assert_eq!(
            def.source.url,
            "https://github.com/Frank1o3/rivet-repository.git"
        );
        assert_eq!(def.source.path.as_deref(), Some("src"));
    }

    #[test]
    fn test_derives_repo_repo_url_from_own_repository() {
        // Mirrors what env!("CARGO_PKG_REPOSITORY") actually resolves to
        // for this workspace, without depending on the macro inside a
        // unit test.
        let own = "https://github.com/Frank1o3/rivet.git";
        let derived = own
            .strip_suffix("/rivet.git")
            .map(|base| format!("{base}/rivet-repository.git"));
        assert_eq!(
            derived.as_deref(),
            Some("https://github.com/Frank1o3/rivet-repository.git")
        );
    }
}
