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
        methods.add_method("destdir", |_, this, ()| {
            Ok(this.dest_dir.to_string_lossy().to_string())
        });

        methods.add_method("build_dir", |_, this, ()| {
            Ok(this.build_dir.to_string_lossy().to_string())
        });

        methods.add_method("source_dir", |_, this, ()| {
            Ok(this.source_dir.to_string_lossy().to_string())
        });

        methods.add_method_mut("set_env", |_, this, (key, value): (String, String)| {
            this.env_vars.insert(key, value);
            Ok(())
        });

        methods.add_method(
            "run",
            |_, this, (cmd, args_val): (String, Option<Table>)| {
                let mut cmd_builder = Command::new(&cmd);
                cmd_builder.current_dir(&this.build_dir);

                // Set configured environment variables
                for (k, v) in &this.env_vars {
                    cmd_builder.env(k, v);
                }
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
            fs::create_dir_all(path).map_err(|e| mlua::Error::runtime(e.to_string()))?;
            Ok(())
        });
    }
}
