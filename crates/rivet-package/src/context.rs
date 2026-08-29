use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mlua::{Table, UserData, UserDataMethods};

/// Execution context exposed to Lua `build(ctx)` and `install(ctx)` functions.
#[derive(Debug, Clone)]
pub struct BuildContext {
    pub source_dir: PathBuf,
    pub build_dir: PathBuf,
    pub dest_dir: PathBuf,
    pub env_vars: HashMap<String, String>,
}

impl BuildContext {
    pub fn new(
        source_dir: impl AsRef<Path>,
        build_dir: impl AsRef<Path>,
        dest_dir: impl AsRef<Path>,
    ) -> Self {
        Self {
            source_dir: source_dir.as_ref().to_path_buf(),
            build_dir: build_dir.as_ref().to_path_buf(),
            dest_dir: dest_dir.as_ref().to_path_buf(),
            env_vars: HashMap::new(),
        }
    }
}

impl UserData for BuildContext {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // ---------------------------------------------------------------------
        // Paths
        // ---------------------------------------------------------------------

        methods.add_method("destdir", |_, this, ()| {
            Ok(this.dest_dir.to_string_lossy().to_string())
        });

        methods.add_method("build_dir", |_, this, ()| {
            Ok(this.build_dir.to_string_lossy().to_string())
        });

        methods.add_method("source_dir", |_, this, ()| {
            Ok(this.source_dir.to_string_lossy().to_string())
        });

        methods.add_method("home", |_, _, ()| {
            dirs::home_dir()
                .map(|path| path.to_string_lossy().to_string())
                .ok_or_else(|| mlua::Error::runtime("could not determine home directory"))
        });

        // ---------------------------------------------------------------------
        // Platform information
        // ---------------------------------------------------------------------

        methods.add_method("os", |_, _, ()| Ok(std::env::consts::OS.to_string()));

        methods.add_method("arch", |_, _, ()| Ok(std::env::consts::ARCH.to_string()));

        // ---------------------------------------------------------------------
        // Environment
        // ---------------------------------------------------------------------

        methods.add_method_mut("set_env", |_, this, (key, value): (String, String)| {
            this.env_vars.insert(key, value);
            Ok(())
        });

        methods.add_method("get_env", |_, this, key: String| {
            if let Some(value) = this.env_vars.get(&key) {
                return Ok(Some(value.clone()));
            }

            Ok(std::env::var(key).ok())
        });

        // ---------------------------------------------------------------------
        // Process execution
        // ---------------------------------------------------------------------

        methods.add_method(
            "run",
            |_, this, (cmd, args_val): (String, Option<Table>)| {
                let mut cmd_builder = Command::new(&cmd);

                cmd_builder.current_dir(&this.build_dir);

                // Set configured environment variables.
                for (k, v) in &this.env_vars {
                    cmd_builder.env(k, v);
                }

                // Always expose DESTDIR to package build systems.
                cmd_builder.env("DESTDIR", &this.dest_dir);

                if let Some(args_table) = args_val {
                    for arg in args_table.sequence_values::<String>() {
                        let arg_str = arg?;
                        cmd_builder.arg(arg_str);
                    }
                }

                println!("  [build] Running: {} ...", cmd);

                let status = cmd_builder.status().map_err(|e| {
                    mlua::Error::runtime(format!("failed to execute '{}': {}", cmd, e))
                })?;

                if !status.success() {
                    return Err(mlua::Error::runtime(format!(
                        "command '{}' failed with exit status {:?}",
                        cmd, status
                    )));
                }

                Ok(())
            },
        );

        // ---------------------------------------------------------------------
        // Filesystem inspection
        // ---------------------------------------------------------------------

        methods.add_method("exists", |_, _, path: String| Ok(Path::new(&path).exists()));

        methods.add_method("is_file", |_, _, path: String| {
            Ok(Path::new(&path).is_file())
        });

        methods.add_method("is_dir", |_, _, path: String| Ok(Path::new(&path).is_dir()));

        methods.add_method("is_symlink", |_, _, path: String| {
            let path = Path::new(&path);

            match fs::symlink_metadata(path) {
                Ok(metadata) => Ok(metadata.file_type().is_symlink()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(error) => Err(mlua::Error::runtime(error.to_string())),
            }
        });

        // ---------------------------------------------------------------------
        // Filesystem manipulation
        // ---------------------------------------------------------------------

        methods.add_method("copy", |_, _, (src, dst): (String, String)| {
            let src_path = Path::new(&src);
            let dst_path = Path::new(&dst);

            if let Some(parent) = dst_path.parent() {
                fs::create_dir_all(parent).map_err(|e| mlua::Error::runtime(e.to_string()))?;
            }

            fs::copy(src_path, dst_path).map_err(|e| mlua::Error::runtime(e.to_string()))?;

            Ok(())
        });

        methods.add_method("mkdir", |_, _, path: String| {
            fs::create_dir_all(&path).map_err(|e| mlua::Error::runtime(e.to_string()))?;

            Ok(())
        });

        methods.add_method("rename", |_, _, (src, dst): (String, String)| {
            let src_path = Path::new(&src);
            let dst_path = Path::new(&dst);

            if let Some(parent) = dst_path.parent() {
                fs::create_dir_all(parent).map_err(|e| mlua::Error::runtime(e.to_string()))?;
            }

            fs::rename(src_path, dst_path).map_err(|e| mlua::Error::runtime(e.to_string()))?;

            Ok(())
        });

        methods.add_method("read_file", |_, _, path: String| {
            fs::read_to_string(&path).map_err(|e| mlua::Error::runtime(e.to_string()))
        });

        methods.add_method("write_file", |_, _, (path, contents): (String, String)| {
            let path = Path::new(&path);

            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| mlua::Error::runtime(e.to_string()))?;
            }

            fs::write(path, contents).map_err(|e| mlua::Error::runtime(e.to_string()))?;

            Ok(())
        });

        methods.add_method("remove_file", |_, _, path: String| {
            fs::remove_file(&path).map_err(|e| mlua::Error::runtime(e.to_string()))?;

            Ok(())
        });

        methods.add_method("remove_dir", |_, _, path: String| {
            fs::remove_dir(&path).map_err(|e| mlua::Error::runtime(e.to_string()))?;

            Ok(())
        });

        methods.add_method("remove_dir_all", |_, _, path: String| {
            fs::remove_dir_all(&path).map_err(|e| mlua::Error::runtime(e.to_string()))?;

            Ok(())
        });

        // ---------------------------------------------------------------------
        // Symbolic links
        // ---------------------------------------------------------------------

        methods.add_method("symlink", |_, _, (target, link): (String, String)| {
            let target_path = Path::new(&target);
            let link_path = Path::new(&link);

            if let Some(parent) = link_path.parent() {
                fs::create_dir_all(parent).map_err(|e| mlua::Error::runtime(e.to_string()))?;
            }

            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(target_path, link_path)
                    .map_err(|e| mlua::Error::runtime(e.to_string()))?;
            }

            #[cfg(windows)]
            {
                if target_path.is_dir() {
                    std::os::windows::fs::symlink_dir(target_path, link_path)
                        .map_err(|e| mlua::Error::runtime(e.to_string()))?;
                } else {
                    std::os::windows::fs::symlink_file(target_path, link_path)
                        .map_err(|e| mlua::Error::runtime(e.to_string()))?;
                }
            }

            Ok(())
        });

        methods.add_method("remove_symlink", |_, _, path: String| {
            let path = Path::new(&path);

            let metadata =
                fs::symlink_metadata(path).map_err(|e| mlua::Error::runtime(e.to_string()))?;

            if !metadata.file_type().is_symlink() {
                return Err(mlua::Error::runtime(format!(
                    "'{}' is not a symbolic link",
                    path.display()
                )));
            }

            fs::remove_file(path).map_err(|e| mlua::Error::runtime(e.to_string()))?;

            Ok(())
        });

        // ---------------------------------------------------------------------
        // File permissions
        // ---------------------------------------------------------------------

        methods.add_method("chmod", |_, _, (path, mode): (String, u32)| {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                let metadata =
                    fs::metadata(&path).map_err(|e| mlua::Error::runtime(e.to_string()))?;

                let mut permissions = metadata.permissions();
                permissions.set_mode(mode);

                fs::set_permissions(&path, permissions)
                    .map_err(|e| mlua::Error::runtime(e.to_string()))?;

                Ok(())
            }

            #[cfg(not(unix))]
            {
                let _ = (path, mode);

                Err(mlua::Error::runtime(
                    "chmod is not supported on this platform",
                ))
            }
        });
    }
}
