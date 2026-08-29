use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use mlua::{Lua, LuaOptions, StdLib, Table, Value};
use rivet_core::{Checksum, Feature, PackageName, TargetArch, TargetOs, Version};

use crate::dependency::{Dependency, DependencyKind};
use crate::error::{PackageError, Result};
use crate::manifest::PackageManifest;
use crate::source::{GitRef, Source};

/// Loader responsible for evaluating `.lua` package recipes in a secure, sandboxed Lua environment.
pub struct PackageLoader {
    lua: Lua,
}

impl PackageLoader {
    /// Creates a new sandboxed `PackageLoader`.
    ///
    /// Sandboxing rules:
    /// - Only standard table, string, math, and basic utilities are available.
    /// - `io`, `os`, `debug`, and `package` (module loader) are strictly disabled.
    pub fn new() -> Result<Self> {
        let options = LuaOptions::new().catch_rust_panics(true);
        // Only load safe standard libraries
        let libs = StdLib::TABLE | StdLib::STRING | StdLib::MATH;
        let lua = Lua::new_with(libs, options)?;

        Ok(Self { lua })
    }

    /// Loads and parses a package definition from a Lua script string.
    pub fn load_from_str(&self, script: &str) -> Result<PackageManifest> {
        let manifest_cell: Rc<RefCell<Option<PackageManifest>>> = Rc::new(RefCell::new(None));
        let manifest_clone = manifest_cell.clone();

        // Register the global `package({...})` DSL function
        let package_fn = self.lua.create_function(move |_, table: Table| {
            if manifest_clone.borrow().is_some() {
                return Err(mlua::Error::runtime(
                    "multiple `package()` calls in one recipe",
                ));
            }

            let manifest = parse_package_table(&table).map_err(mlua::Error::runtime)?;
            *manifest_clone.borrow_mut() = Some(manifest);
            Ok(())
        })?;

        self.lua.globals().set("package", package_fn)?;

        // Execute the script in sandbox
        self.lua.load(script).exec()?;

        let result = manifest_cell
            .borrow_mut()
            .take()
            .ok_or(PackageError::NoPackageDefined)?;

        Ok(result)
    }

    /// Loads and parses a package definition from a file path.
    pub fn load_from_file(&self, path: impl AsRef<Path>) -> Result<PackageManifest> {
        let path = path.as_ref();
        let script = std::fs::read_to_string(path)?;
        let mut manifest = self.load_from_str(&script)?;
        manifest.recipe_path = path.to_path_buf();
        Ok(manifest)
    }

    /// Loads a recipe and runs whichever of the given lifecycle function
    /// names it defines, in the order they appear in `hooks`. Hooks the
    /// recipe doesn't define are silently skipped — a package with no
    /// `post_install` is completely normal.
    pub fn run_hooks(
        &self,
        script: &str,
        ctx: &crate::context::BuildContext,
        hooks: &[&str],
    ) -> Result<()> {
        let table_cell: Rc<RefCell<Option<Table>>> = Rc::new(RefCell::new(None));
        let table_clone = table_cell.clone();

        let package_fn = self.lua.create_function(move |_, table: Table| {
            *table_clone.borrow_mut() = Some(table);
            Ok(())
        })?;

        self.lua.globals().set("package", package_fn)?;
        self.lua.load(script).exec()?;

        let table = table_cell
            .borrow_mut()
            .take()
            .ok_or(PackageError::NoPackageDefined)?;

        for hook in hooks {
            if let Ok(func) = table.get::<mlua::Function>(*hook) {
                println!("  [hook] running '{}'...", hook);
                func.call::<()>(ctx.clone())?;
            }
        }

        Ok(())
    }
}

impl Default for PackageLoader {
    fn default() -> Self {
        Self::new().expect("failed to initialize sandboxed Lua runtime")
    }
}

/// Parses a Lua table into a strongly typed `PackageManifest`.
fn parse_package_table(table: &Table) -> std::result::Result<PackageManifest, String> {
    // 1. Name (required)
    let name_str: String = table
        .get("name")
        .map_err(|_| "missing required field 'name'".to_string())?;
    let name = PackageName::new(name_str).map_err(|e| e.to_string())?;

    // 2. Version (required)
    let version_str: String = table
        .get("version")
        .map_err(|_| "missing required field 'version'".to_string())?;
    let version = Version::parse(&version_str).map_err(|e| e.to_string())?;

    // 3. Optional metadata
    let description: Option<String> = table.get("description").ok();
    let license: Option<String> = table.get("license").ok();
    let homepage: Option<String> = table.get("homepage").ok();

    // 4. Source
    let source = parse_source(table)?;

    // 5. Dependencies
    let mut dependencies = Vec::new();
    if let Ok(deps_table) = table.get::<Table>("dependencies") {
        for value in deps_table.sequence_values::<Value>() {
            let val = value.map_err(|e| format!("invalid dependency entry: {}", e))?;
            let dep = parse_dependency_value(val, DependencyKind::Runtime)?;
            dependencies.push(dep);
        }
    }

    if let Ok(build_deps_table) = table.get::<Table>("build_dependencies") {
        for value in build_deps_table.sequence_values::<Value>() {
            let val = value.map_err(|e| format!("invalid build_dependency entry: {}", e))?;
            let dep = parse_dependency_value(val, DependencyKind::Build)?;
            dependencies.push(dep);
        }
    }

    // 6. Features
    let mut features = HashMap::new();
    let mut default_features = Vec::new();

    if let Ok(feat_table) = table.get::<Table>("features") {
        for pair in feat_table.pairs::<String, Value>() {
            let (feat_name, val) = pair.map_err(|e| format!("invalid feature entry: {}", e))?;
            let feat = Feature::new(feat_name).map_err(|e| e.to_string())?;

            match val {
                Value::Boolean(is_default) => {
                    if is_default {
                        default_features.push(feat.clone());
                    }
                    features.entry(feat).or_insert_with(Vec::new);
                }
                Value::Table(dep_table) => {
                    let mut feat_deps = Vec::new();
                    for dep_val in dep_table.sequence_values::<Value>() {
                        let d_val =
                            dep_val.map_err(|e| format!("invalid feature dependency: {}", e))?;
                        let dep = parse_dependency_value(d_val, DependencyKind::Runtime)?
                            .with_feature(feat.clone());
                        feat_deps.push(dep);
                    }
                    features.insert(feat, feat_deps);
                }
                _ => {}
            }
        }
    }

    // 7. Supported Architectures
    let mut supported_architectures = Vec::new();
    if let Ok(archs) = table.get::<Table>("architectures") {
        for val in archs.sequence_values::<String>() {
            let arch_str = val.map_err(|e| format!("invalid architecture entry: {}", e))?;
            let arch = arch_str.parse::<TargetArch>().map_err(|e| e.to_string())?;
            supported_architectures.push(arch);
        }
    }

    // 8. Supported OS
    let mut supported_os = Vec::new();
    if let Ok(os_list) = table.get::<Table>("os") {
        for val in os_list.sequence_values::<String>() {
            let os_str = val.map_err(|e| format!("invalid os entry: {}", e))?;
            let os = os_str.parse::<TargetOs>().map_err(|e| e.to_string())?;
            supported_os.push(os);
        }
    }

    // Provider check — lets this package say "skip building me if a
    // compatible version is already on the system."
    let provider_check = if let Ok(pc_table) = table.get::<Table>("provides_check") {
        let command: String = pc_table
            .get("command")
            .map_err(|_| "provides_check missing 'command'".to_string())?;
        let version_flag: String = pc_table
            .get("version_flag")
            .unwrap_or_else(|_| "--version".to_string());

        Some(crate::provider::ProviderCheck {
            command,
            version_flag,
        })
    } else {
        None
    };

    Ok(PackageManifest {
        name,
        version,
        description,
        license,
        homepage,
        source,
        dependencies,
        features,
        default_features,
        supported_architectures,
        supported_os,
        recipe_path: PathBuf::new(),
        provider_check,
    })
}

/// Helper to parse a dependency value (string or table).
fn parse_dependency_value(
    val: Value,
    default_kind: DependencyKind,
) -> std::result::Result<Dependency, String> {
    match val {
        Value::String(s) => {
            let s_str = s.to_str().map_err(|e| e.to_string())?;
            Dependency::parse_shorthand(&s_str, default_kind).map_err(|e| e.to_string())
        }
        Value::Table(t) => {
            let name_str: String = t
                .get("name")
                .map_err(|_| "dependency table missing 'name'".to_string())?;
            let name = PackageName::new(name_str).map_err(|e| e.to_string())?;

            let req = if let Ok(req_str) = t.get::<String>("version") {
                rivet_core::VersionReq::parse(&req_str).map_err(|e| e.to_string())?
            } else {
                rivet_core::VersionReq::STAR
            };

            let feature = if let Ok(feat_str) = t.get::<String>("feature") {
                Some(Feature::new(feat_str).map_err(|e| e.to_string())?)
            } else {
                None
            };

            Ok(Dependency {
                name,
                req,
                kind: default_kind,
                feature,
            })
        }
        _ => Err("dependency must be a string or table".to_string()),
    }
}

/// Helper to parse source definitions.
fn parse_source(table: &Table) -> std::result::Result<Option<Source>, String> {
    let source_table: Table = match table.get("source") {
        Ok(t) => t,
        Err(_) => return Ok(None),
    };

    let source_type: String = source_table
        .get("type")
        .unwrap_or_else(|_| "archive".to_string());

    match source_type.as_str() {
        "archive" | "tarball" => {
            let url: String = source_table
                .get("url")
                .map_err(|_| "archive source missing 'url'".to_string())?;
            let checksum_str: String = if let Ok(s) = source_table.get::<String>("sha256") {
                format!("sha256:{}", s)
            } else if let Ok(s) = source_table.get::<String>("sha512") {
                format!("sha512:{}", s)
            } else if let Ok(s) = source_table.get::<String>("checksum") {
                s
            } else {
                return Err("archive source missing 'sha256' or 'checksum'".to_string());
            };

            let checksum = checksum_str
                .parse::<Checksum>()
                .map_err(|e| e.to_string())?;
            Ok(Some(Source::Archive { url, checksum }))
        }
        "git" => {
            let url: String = source_table
                .get("url")
                .map_err(|_| "git source missing 'url'".to_string())?;
            let reference = if let Ok(tag) = source_table.get::<String>("tag") {
                Some(GitRef::Tag(tag))
            } else if let Ok(branch) = source_table.get::<String>("branch") {
                Some(GitRef::Branch(branch))
            } else if let Ok(commit) = source_table.get::<String>("commit") {
                Some(GitRef::Commit(commit))
            } else {
                None
            };

            let checksum = source_table
                .get::<String>("checksum")
                .ok()
                .and_then(|s| s.parse::<Checksum>().ok());

            Ok(Some(Source::Git {
                url,
                reference,
                checksum,
            }))
        }
        "local" => {
            let path: String = source_table
                .get("path")
                .map_err(|_| "local source missing 'path'".to_string())?;
            Ok(Some(Source::Local { path }))
        }
        "virtual" => Ok(Some(Source::Virtual)),
        other => Err(format!("unknown source type: '{}'", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_basic_package() {
        let script = r#"
            package({
                name = "zlib",
                version = "1.3.1",
                description = "A Massively Spiffy Yet Delicately Unobtrusive Compression Library",
                license = "Zlib",
                homepage = "https://zlib.net",

                source = {
                    type = "archive",
                    url = "https://zlib.net/zlib-1.3.1.tar.gz",
                    sha256 = "9a93b2b7dfdac77ceba5a558a580e7466ffdd6fede45852c882f64fe473d73cf",
                },

                build_dependencies = {
                    "cmake >= 3.20",
                    "ninja",
                },

                dependencies = {
                    "glibc >= 2.30",
                },

                features = {
                    minizip = false,
                },

                architectures = { "x86_64", "aarch64" },
                os = { "linux", "macos" },
            })
        "#;

        let loader = PackageLoader::new().unwrap();
        let manifest = loader.load_from_str(script).unwrap();

        assert_eq!(manifest.name.as_str(), "zlib");
        assert_eq!(manifest.version.to_string(), "1.3.1");
        assert_eq!(manifest.license.as_deref(), Some("Zlib"));
        assert_eq!(manifest.dependencies.len(), 3); // 2 build + 1 runtime

        assert_eq!(manifest.supported_architectures.len(), 2);
        assert_eq!(manifest.supported_os.len(), 2);
    }

    #[test]
    fn test_sandbox_blocks_os_and_io() {
        let script = r#"
            if os ~= nil or io ~= nil then
                error("security breach: os or io standard library is present!")
            end

            package({
                name = "safe-pkg",
                version = "1.0.0",
            })
        "#;

        let loader = PackageLoader::new().unwrap();
        let manifest = loader.load_from_str(script).unwrap();
        assert_eq!(manifest.name.as_str(), "safe-pkg");
    }

    #[test]
    fn test_execute_build_and_install_hooks() {
        use tempfile::tempdir;
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let bld = tmp.path().join("bld");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&bld).unwrap();
        std::fs::create_dir_all(&dst).unwrap();

        // Create a dummy source file
        std::fs::write(src.join("hello.txt"), "hello rivet").unwrap();

        let script = r#"
            package({
                name = "test-pkg",
                version = "1.0.0",

                build = function(ctx)
                    ctx:mkdir(ctx:build_dir() .. "/out")
                    ctx:copy(ctx:source_dir() .. "/hello.txt", ctx:build_dir() .. "/out/hello.txt")
                end,

                install = function(ctx)
                    ctx:copy(ctx:build_dir() .. "/out/hello.txt", ctx:destdir() .. "/installed_hello.txt")
                end,
            })
        "#;

        let ctx = crate::context::BuildContext::new(&src, &bld, &dst, tmp.path());
        let loader = PackageLoader::new().unwrap();
        loader
            .run_hooks(
                script,
                &ctx,
                &["pre_install", "build", "install", "post_install"],
            )
            .unwrap();

        // Verify that install hook copied file to dst
        assert!(dst.join("installed_hello.txt").exists());
        let content = std::fs::read_to_string(dst.join("installed_hello.txt")).unwrap();
        assert_eq!(content, "hello rivet");
    }
}
