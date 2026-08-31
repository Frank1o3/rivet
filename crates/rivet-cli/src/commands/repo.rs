use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use rivet_repository::{LocalRepository, RepositoryDefinition, RepositoryLoader};

use crate::args::RepoCommands;

pub fn execute(command: RepoCommands) -> Result<()> {
    match command {
        RepoCommands::List => list(),
        RepoCommands::Add {
            slug,
            url,
            branch,
            path,
            priority,
            name,
            description,
        } => add(
            &slug,
            &url,
            branch.as_deref(),
            path.as_deref(),
            priority,
            name.as_deref(),
            description.as_deref(),
        ),
        RepoCommands::Remove { slug } => remove(&slug),
        RepoCommands::Enable { slug } => enable(&slug),
        RepoCommands::Disable { slug } => disable(&slug),
    }
}

#[derive(Debug, Clone)]
pub struct RepoDisplayInfo {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub branch: Option<String>,
    pub path: Option<String>,
    pub priority: i32,
    pub enabled: bool,
    pub is_local: bool,
    pub definition_file: Option<PathBuf>,
    pub package_count: usize,
}

pub fn collect_repositories(
    repositories_dir: &Path,
    local_dir: &Path,
) -> Result<Vec<RepoDisplayInfo>> {
    let mut list = Vec::new();

    // 1. Local pseudo-repository
    let mut local_repo = LocalRepository::open(local_dir, "local");
    let local_count = local_repo.scan_and_index().unwrap_or(0);
    list.push(RepoDisplayInfo {
        slug: "local".to_string(),
        name: "Local pseudo-repository".to_string(),
        description: Some("Locally authored package recipes".to_string()),
        url: None,
        branch: None,
        path: Some(local_dir.to_string_lossy().to_string()),
        priority: 1000,
        enabled: true,
        is_local: true,
        definition_file: None,
        package_count: local_count,
    });

    // 2. Remote repository definitions
    if !repositories_dir.exists() {
        return Ok(list);
    }

    let loader = RepositoryLoader::new()?;
    let packages_root = rivet_core::default_packages_dir().ok();

    for entry in fs::read_dir(repositories_dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };

        let (slug, is_disabled_file) = if let Some(s) = file_name.strip_suffix(".lua.disabled") {
            (s.to_string(), true)
        } else if let Some(s) = file_name.strip_suffix(".lua") {
            (s.to_string(), false)
        } else {
            continue;
        };

        if slug == "local" {
            continue;
        }

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let def: RepositoryDefinition = match loader.load_from_str(&content) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let enabled = if is_disabled_file { false } else { def.enabled };

        let package_count = if let Some(root) = &packages_root {
            let pkg_dir = root.join(&slug);
            if pkg_dir.exists() {
                fs::read_dir(&pkg_dir)
                    .map(|entries| {
                        entries
                            .filter_map(Result::ok)
                            .filter(|e| {
                                e.path().extension().and_then(|x| x.to_str()) == Some("lua")
                            })
                            .count()
                    })
                    .unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        };

        list.push(RepoDisplayInfo {
            slug,
            name: def.name,
            description: def.description,
            url: Some(def.source.url),
            branch: Some(def.source.branch),
            path: def.source.path,
            priority: def.priority,
            enabled,
            is_local: false,
            definition_file: Some(path),
            package_count,
        });
    }

    // Sort remote repos by priority descending, then slug ascending
    let (mut local_items, mut remote_items): (Vec<_>, Vec<_>) =
        list.into_iter().partition(|r| r.is_local);

    remote_items.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.slug.cmp(&b.slug)));

    local_items.extend(remote_items);
    Ok(local_items)
}

pub fn list() -> Result<()> {
    let repositories_dir = rivet_core::default_repositories_dir()?;
    let local_dir = rivet_core::default_local_packages_dir()?;

    let repos = collect_repositories(&repositories_dir, &local_dir)?;

    println!("📚 Configured Repositories ({} total):\n", repos.len());

    for repo in repos {
        let status_tag = if repo.enabled {
            "● [enabled]"
        } else {
            "○ [disabled]"
        };

        println!("{} {} (priority: {})", status_tag, repo.slug, repo.priority);
        println!("    Name:        {}", repo.name);
        if let Some(desc) = &repo.description {
            println!("    Description: {}", desc);
        }
        if repo.is_local {
            if let Some(p) = &repo.path {
                println!("    Path:        {}", p);
            }
        } else {
            if let Some(url) = &repo.url {
                println!("    Source:      {}", url);
            }
            if let Some(b) = &repo.branch {
                println!("    Branch:      {}", b);
            }
            if let Some(p) = &repo.path {
                println!("    Subpath:     {}", p);
            }
            if let Some(def) = &repo.definition_file {
                println!("    Definition:  {}", def.display());
            }
        }
        println!("    Packages:    {} available", repo.package_count);
        println!();
    }

    Ok(())
}

pub fn add(
    slug: &str,
    url: &str,
    branch: Option<&str>,
    path: Option<&str>,
    priority: Option<i32>,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    validate_slug(slug)?;

    let repositories_dir = rivet_core::default_repositories_dir()?;
    fs::create_dir_all(&repositories_dir).context("failed to create repositories directory")?;

    let active_path = repositories_dir.join(format!("{slug}.lua"));
    let disabled_path = repositories_dir.join(format!("{slug}.lua.disabled"));

    if active_path.exists() || disabled_path.exists() {
        bail!("repository '{slug}' already exists in '{}'", repositories_dir.display());
    }

    let repo_name = name.unwrap_or(slug);
    let repo_branch = branch.unwrap_or("main");
    let repo_priority = priority.unwrap_or(10);

    let script = generate_repository_lua(
        repo_name,
        description,
        url,
        repo_branch,
        path,
        repo_priority,
    );

    // Validate syntax before writing
    let loader = RepositoryLoader::new()?;
    loader
        .load_from_str(&script)
        .context("generated repository definition failed validation")?;

    fs::write(&active_path, script).context("failed to write repository definition")?;

    println!("✅ Added repository '{slug}' (priority: {repo_priority}).");
    println!("  URL:        {url}");
    println!("  Branch:     {repo_branch}");
    if let Some(p) = path {
        println!("  Subpath:    {p}");
    }
    println!("  Definition: {}", active_path.display());
    println!("\nRun 'rivet update' to fetch its package index.");

    Ok(())
}

pub fn remove(slug: &str) -> Result<()> {
    if slug == "local" {
        bail!("cannot remove the local pseudo-repository");
    }

    let repositories_dir = rivet_core::default_repositories_dir()?;
    let active_path = repositories_dir.join(format!("{slug}.lua"));
    let disabled_path = repositories_dir.join(format!("{slug}.lua.disabled"));

    let target_file = if active_path.exists() {
        active_path
    } else if disabled_path.exists() {
        disabled_path
    } else {
        bail!("repository '{slug}' not found");
    };

    fs::remove_file(&target_file)
        .with_context(|| format!("failed to remove repository file '{}'", target_file.display()))?;

    // Cleanup mirror cache if present
    if let Ok(mirror_root) = rivet_core::default_repo_mirror_cache() {
        let mirror_dir = mirror_root.join(slug);
        if mirror_dir.exists() {
            let _ = fs::remove_dir_all(&mirror_dir);
        }
    }

    // Cleanup packages dir if present
    if let Ok(packages_root) = rivet_core::default_packages_dir() {
        let pkg_dir = packages_root.join(slug);
        if pkg_dir.exists() {
            let _ = fs::remove_dir_all(&pkg_dir);
        }
    }

    println!("✅ Removed repository '{slug}'.");
    Ok(())
}

pub fn enable(slug: &str) -> Result<()> {
    if slug == "local" {
        println!("Repository 'local' is always enabled.");
        return Ok(());
    }

    let repositories_dir = rivet_core::default_repositories_dir()?;
    let active_path = repositories_dir.join(format!("{slug}.lua"));
    let disabled_path = repositories_dir.join(format!("{slug}.lua.disabled"));

    if disabled_path.exists() {
        fs::rename(&disabled_path, &active_path)
            .context("failed to rename disabled repository file")?;
        println!("✅ Enabled repository '{slug}'.");
        return Ok(());
    }

    if active_path.exists() {
        println!("Repository '{slug}' is already enabled.");
        return Ok(());
    }

    bail!("repository '{slug}' not found");
}

pub fn disable(slug: &str) -> Result<()> {
    if slug == "local" {
        bail!("cannot disable the local pseudo-repository");
    }

    let repositories_dir = rivet_core::default_repositories_dir()?;
    let active_path = repositories_dir.join(format!("{slug}.lua"));
    let disabled_path = repositories_dir.join(format!("{slug}.lua.disabled"));

    if active_path.exists() {
        fs::rename(&active_path, &disabled_path)
            .context("failed to rename repository file to disabled")?;
        println!("⏸️  Disabled repository '{slug}'.");
        return Ok(());
    }

    if disabled_path.exists() {
        println!("Repository '{slug}' is already disabled.");
        return Ok(());
    }

    bail!("repository '{slug}' not found");
}

fn validate_slug(slug: &str) -> Result<()> {
    if slug.is_empty() {
        bail!("repository slug cannot be empty");
    }
    if slug == "local" {
        bail!("'local' is reserved for the local pseudo-repository");
    }
    if !slug
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        bail!(
            "repository slug '{}' contains invalid characters (must be alphanumeric, '-', or '_')",
            slug
        );
    }
    Ok(())
}

fn generate_repository_lua(
    name: &str,
    description: Option<&str>,
    url: &str,
    branch: &str,
    path: Option<&str>,
    priority: i32,
) -> String {
    let mut out = String::new();
    out.push_str("repository({\n");
    out.push_str(&format!("    name = \"{name}\",\n"));
    if let Some(desc) = description {
        out.push_str(&format!("    description = \"{desc}\",\n"));
    }
    out.push_str(&format!("    priority = {priority},\n"));
    out.push_str("\n    source = {\n");
    out.push_str(&format!("        url = \"{url}\",\n"));
    out.push_str(&format!("        branch = \"{branch}\",\n"));
    if let Some(p) = path {
        if !p.is_empty() {
            out.push_str(&format!("        path = \"{p}\",\n"));
        }
    }
    out.push_str("    },\n");
    out.push_str("})\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_validate_slug() {
        assert!(validate_slug("community").is_ok());
        assert!(validate_slug("extra-repo").is_ok());
        assert!(validate_slug("my_repo_1").is_ok());

        assert!(validate_slug("").is_err());
        assert!(validate_slug("local").is_err());
        assert!(validate_slug("bad slug").is_err());
        assert!(validate_slug("bad/slug").is_err());
    }

    #[test]
    fn test_generate_and_parse_repository_lua() {
        let script = generate_repository_lua(
            "Community Repo",
            Some("A community packages repository"),
            "https://github.com/example/community.git",
            "stable",
            Some("pkgs"),
            25,
        );

        let loader = RepositoryLoader::new().unwrap();
        let def = loader.load_from_str(&script).unwrap();

        assert_eq!(def.name, "Community Repo");
        assert_eq!(def.description.as_deref(), Some("A community packages repository"));
        assert_eq!(def.source.url, "https://github.com/example/community.git");
        assert_eq!(def.source.branch, "stable");
        assert_eq!(def.source.path.as_deref(), Some("pkgs"));
        assert_eq!(def.priority, 25);
        assert!(def.enabled);
    }

    #[test]
    fn test_collect_repositories_sorting_and_status() {
        let temp = tempdir().unwrap();
        let repos_dir = temp.path().join("repositories");
        let local_dir = temp.path().join("local");
        fs::create_dir_all(&repos_dir).unwrap();
        fs::create_dir_all(&local_dir).unwrap();

        // Write a low priority repo
        let low_script = generate_repository_lua(
            "low",
            None,
            "https://github.com/example/low.git",
            "main",
            None,
            5,
        );
        fs::write(repos_dir.join("low.lua"), low_script).unwrap();

        // Write a high priority repo
        let high_script = generate_repository_lua(
            "high",
            None,
            "https://github.com/example/high.git",
            "main",
            None,
            50,
        );
        fs::write(repos_dir.join("high.lua"), high_script).unwrap();

        // Write a disabled repo
        let disabled_script = generate_repository_lua(
            "disabled-repo",
            None,
            "https://github.com/example/disabled.git",
            "main",
            None,
            20,
        );
        fs::write(repos_dir.join("disabled-repo.lua.disabled"), disabled_script).unwrap();

        let list = collect_repositories(&repos_dir, &local_dir).unwrap();

        // First is local (priority 1000)
        assert_eq!(list[0].slug, "local");
        assert_eq!(list[0].priority, 1000);
        assert!(list[0].enabled);

        // Next is high (priority 50)
        assert_eq!(list[1].slug, "high");
        assert_eq!(list[1].priority, 50);
        assert!(list[1].enabled);

        // Next is disabled-repo (priority 20)
        assert_eq!(list[2].slug, "disabled-repo");
        assert_eq!(list[2].priority, 20);
        assert!(!list[2].enabled);

        // Next is low (priority 5)
        assert_eq!(list[3].slug, "low");
        assert_eq!(list[3].priority, 5);
        assert!(list[3].enabled);
    }

    #[test]
    fn test_repo_add_remove_enable_disable_lifecycle() {
        let temp = tempdir().unwrap();
        let repos_dir = temp.path().join("repositories");
        let local_dir = temp.path().join("local");
        fs::create_dir_all(&repos_dir).unwrap();
        fs::create_dir_all(&local_dir).unwrap();

        // 1. Validate slug checks
        assert!(validate_slug("local").is_err());
        assert!(validate_slug("invalid/name").is_err());
        assert!(validate_slug("valid-name_1").is_ok());

        // 2. Add repo
        let script = generate_repository_lua(
            "myrepo",
            Some("My repo"),
            "https://github.com/test/repo.git",
            "main",
            None,
            15,
        );
        let repo_file = repos_dir.join("myrepo.lua");
        fs::write(&repo_file, script).unwrap();

        let list = collect_repositories(&repos_dir, &local_dir).unwrap();
        let myrepo = list.iter().find(|r| r.slug == "myrepo").unwrap();
        assert!(myrepo.enabled);
        assert_eq!(myrepo.priority, 15);

        // 3. Disable repo
        let disabled_file = repos_dir.join("myrepo.lua.disabled");
        fs::rename(&repo_file, &disabled_file).unwrap();

        let list_disabled = collect_repositories(&repos_dir, &local_dir).unwrap();
        let myrepo_dis = list_disabled.iter().find(|r| r.slug == "myrepo").unwrap();
        assert!(!myrepo_dis.enabled);

        // 4. Re-enable repo
        fs::rename(&disabled_file, &repo_file).unwrap();
        let list_reenabled = collect_repositories(&repos_dir, &local_dir).unwrap();
        let myrepo_en = list_reenabled.iter().find(|r| r.slug == "myrepo").unwrap();
        assert!(myrepo_en.enabled);

        // 5. Remove repo
        fs::remove_file(&repo_file).unwrap();
        let list_removed = collect_repositories(&repos_dir, &local_dir).unwrap();
        assert!(list_removed.iter().all(|r| r.slug != "myrepo"));
    }
}
