use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use mlua::{Lua, LuaOptions, StdLib, Table};

use crate::definition::{RepositoryDefinition, RepositorySource};
use crate::error::{RepositoryError, Result};

/// Loader responsible for evaluating `repository.lua` definitions in a
/// sandboxed Lua environment. Deliberately a separate Lua instance from
/// `rivet_package::PackageLoader` — repository parsing has nothing to do
/// with package installation, and the two shouldn't be coupled just to
/// save a few lines of sandbox setup.
pub struct RepositoryLoader {
    lua: Lua,
}

impl RepositoryLoader {
    pub fn new() -> Result<Self> {
        let options = LuaOptions::new().catch_rust_panics(true);
        let libs = StdLib::TABLE | StdLib::STRING | StdLib::MATH;
        let lua = Lua::new_with(libs, options).map_err(RepositoryError::from)?;
        Ok(Self { lua })
    }

    pub fn load_from_str(&self, script: &str) -> Result<RepositoryDefinition> {
        let def_cell: Rc<RefCell<Option<RepositoryDefinition>>> = Rc::new(RefCell::new(None));
        let def_clone = def_cell.clone();

        let repository_fn = self.lua.create_function(move |_, table: Table| {
            if def_clone.borrow().is_some() {
                return Err(mlua::Error::runtime(
                    "multiple `repository()` calls in one definition",
                ));
            }
            let def = parse_repository_table(&table).map_err(mlua::Error::runtime)?;
            *def_clone.borrow_mut() = Some(def);
            Ok(())
        })?;

        self.lua.globals().set("repository", repository_fn)?;
        self.lua.load(script).exec()?;

        def_cell.borrow_mut().take().ok_or_else(|| {
            RepositoryError::Definition(
                "no `repository({ ... })` call found in definition".to_string(),
            )
        })
    }

    pub fn load_from_file(&self, path: impl AsRef<Path>) -> Result<RepositoryDefinition> {
        let path = path.as_ref();
        let script = std::fs::read_to_string(path).map_err(RepositoryError::Io)?;
        let mut def = self.load_from_str(&script)?;
        def.definition_path = path.to_path_buf();
        Ok(def)
    }
}

impl Default for RepositoryLoader {
    fn default() -> Self {
        Self::new().expect("failed to initialize sandboxed Lua runtime")
    }
}

fn parse_repository_table(table: &Table) -> std::result::Result<RepositoryDefinition, String> {
    let name: String = table
        .get("name")
        .map_err(|_| "missing required field 'name'".to_string())?;
    let description: Option<String> = table.get("description").ok();
    let license: Option<String> = table.get("license").ok();

    let source_table: Table = table
        .get("source")
        .map_err(|_| "missing required field 'source'".to_string())?;
    let url: String = source_table
        .get("url")
        .map_err(|_| "source missing required field 'url'".to_string())?;
    let branch: String = source_table
        .get("branch")
        .unwrap_or_else(|_| "main".to_string());
    let path: Option<String> = source_table.get("path").ok();

    let priority: i32 = table
        .get::<Option<i32>>("priority")
        .ok()
        .flatten()
        .unwrap_or(10);
    let enabled: bool = table
        .get::<Option<bool>>("enabled")
        .ok()
        .flatten()
        .unwrap_or(true);

    Ok(RepositoryDefinition {
        name,
        description,
        license,
        source: RepositorySource { url, branch, path },
        priority,
        enabled,
        definition_path: std::path::PathBuf::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_basic_repository() {
        let script = r#"
            repository({
                name = "Rivet",
                description = "Official Rivet package repository",
                license = "MIT",

                source = {
                    url = "https://github.com/example/rivet-repo.git",
                    branch = "stable",
                },
            })
        "#;

        let loader = RepositoryLoader::new().unwrap();
        let def = loader.load_from_str(script).unwrap();

        assert_eq!(def.name, "Rivet");
        assert_eq!(def.license.as_deref(), Some("MIT"));
        assert_eq!(def.source.url, "https://github.com/example/rivet-repo.git");
        assert_eq!(def.source.branch, "stable");
        assert_eq!(def.priority, 10);
        assert!(def.enabled);
    }

    #[test]
    fn test_load_repository_with_priority_and_enabled() {
        let script = r#"
            repository({
                name = "custom-priority",
                priority = 50,
                enabled = false,
                source = { url = "https://github.com/example/repo.git" },
            })
        "#;

        let loader = RepositoryLoader::new().unwrap();
        let def = loader.load_from_str(script).unwrap();
        assert_eq!(def.name, "custom-priority");
        assert_eq!(def.priority, 50);
        assert!(!def.enabled);
    }

    #[test]
    fn test_branch_defaults_to_main() {
        let script = r#"
            repository({
                name = "community",
                source = { url = "https://github.com/example/community-repo.git" },
            })
        "#;

        let loader = RepositoryLoader::new().unwrap();
        let def = loader.load_from_str(script).unwrap();
        assert_eq!(def.source.branch, "main");
    }

    #[test]
    fn test_missing_source_is_error() {
        let script = r#"repository({ name = "broken" })"#;
        let loader = RepositoryLoader::new().unwrap();
        assert!(loader.load_from_str(script).is_err());
    }

    #[test]
    fn test_sandbox_blocks_os_and_io() {
        let script = r#"
            if os ~= nil or io ~= nil then
                error("security breach: os or io standard library is present!")
            end
            repository({
                name = "safe-repo",
                source = { url = "https://example.com/repo.git", branch = "main" },
            })
        "#;
        let loader = RepositoryLoader::new().unwrap();
        assert!(loader.load_from_str(script).is_ok());
    }

    #[test]
    fn test_source_path_is_parsed() {
        let script = r#"
            repository({
                name = "rivet",
                source = {
                    url = "https://github.com/Frank1o3/rivet-repo.git",
                    branch = "stable",
                    path = "src",
                },
            })
        "#;

        let loader = RepositoryLoader::new().unwrap();
        let def = loader.load_from_str(script).unwrap();
        assert_eq!(def.source.path.as_deref(), Some("src"));
    }

    #[test]
    fn test_source_path_defaults_to_none() {
        let script = r#"
            repository({
                name = "rivet",
                source = { url = "https://example.com/repo.git", branch = "main" },
            })
        "#;

        let loader = RepositoryLoader::new().unwrap();
        let def = loader.load_from_str(script).unwrap();
        assert_eq!(def.source.path, None);
    }
}
