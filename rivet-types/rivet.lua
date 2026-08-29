---@meta

---@class RivetSourceArchive
---@field type "archive"|"tarball"
---@field url string
---@field sha256 string?
---@field sha512 string?
---@field checksum string?

---@class RivetSourceGit
---@field type "git"
---@field url string
---@field tag string?
---@field branch string?
---@field commit string?
---@field checksum string?

---@class RivetSourceLocal
---@field type "local"
---@field path string

---@class RivetSourceVirtual
---@field type "virtual"

---@class RivetProviderCheck
---@field command string
---@field version_flag string?

---@alias RivetSource RivetSourceArchive|RivetSourceGit|RivetSourceLocal|RivetSourceVirtual

---Execution context passed to lifecycle hooks.
---@class RivetBuildContext
local BuildContext = {}

---Absolute path to the install/destination directory.
---@return string
function BuildContext:destdir() end

---Absolute path to the scratch build directory.
---@return string
function BuildContext:build_dir() end

---Absolute path to the source directory (local path, or extracted
---archive / checked-out git repo once source fetching is implemented).
---@return string
function BuildContext:source_dir() end

---Absolute path to the current user's home directory.
---@return string
function BuildContext:home() end

---Returns the target operating system.
---Examples: "linux", "macos", "windows".
---@return string
function BuildContext:os() end

---Returns the target CPU architecture.
---Examples: "x86_64", "aarch64", "riscv64", "arm".
---@return string
function BuildContext:arch() end

---Sets an environment variable for subsequent `run` calls within the
---same hook invocation. Does not currently carry over between hooks.
---@param key string
---@param value string
function BuildContext:set_env(key, value) end

---Gets an environment variable.
---Recipe-local variables take precedence over the process environment.
---@param key string
---@return string?
function BuildContext:get_env(key) end

---Runs a subprocess inside the build directory.
---@param cmd string
---@param args string[]?
function BuildContext:run(cmd, args) end

---Runs a subprocess inside the specified directory.
---@param path string
---@param cmd string
---@param args string[]?
function BuildContext:run_in(path, cmd, args) end

---Returns whether a path exists.
---@param path string
---@return boolean
function BuildContext:exists(path) end

---Returns whether a path refers to a regular file.
---@param path string
---@return boolean
function BuildContext:is_file(path) end

---Returns whether a path refers to a directory.
---@param path string
---@return boolean
function BuildContext:is_dir(path) end

---Returns whether a path is a symbolic link.
---@param path string
---@return boolean
function BuildContext:is_symlink(path) end

---Copies a file, creating parent directories as needed.
---@param src string
---@param dst string
function BuildContext:copy(src, dst) end

---Renames or moves a file or directory.
---Creates parent directories for the destination when needed.
---@param src string
---@param dst string
function BuildContext:rename(src, dst) end

---Reads a UTF-8 text file.
---@param path string
---@return string
function BuildContext:read_file(path) end

---Writes a UTF-8 text file, creating parent directories as needed.
---@param path string
---@param contents string
function BuildContext:write_file(path, contents) end

---Creates a directory, and its parents, at `path`.
---@param path string
function BuildContext:mkdir(path) end

---Removes a file.
---@param path string
function BuildContext:remove_file(path) end

---Removes an empty directory.
---@param path string
function BuildContext:remove_dir(path) end

---Recursively removes a directory and its contents.
---@param path string
function BuildContext:remove_dir_all(path) end

---Creates a symbolic link.
---@param target string
---@param link string
function BuildContext:symlink(target, link) end

---Absolute path to the current user's installation prefix.
---Defaults to `$HOME/.local`.
---@return string
function BuildContext:prefix() end

---Removes a symbolic link.
---Fails if `path` is not a symbolic link.
---@param path string
function BuildContext:remove_symlink(path) end

---Changes file or directory permissions.
---On Unix, `mode` uses standard permission bits, e.g. `0o755`.
---@param path string
---@param mode string
function BuildContext:chmod(path, mode) end

---@class RivetPackageDef
---@field name string
---@field version string
---@field description string?
---@field license string?
---@field homepage string?
---@field source RivetSource?
---@field dependencies string[]?
---@field build_dependencies string[]?
---@field features table<string, boolean|string[]>?
---@field architectures string[]? # CPU targets only: x86_64, aarch64, riscv64, armv7 — NOT things like wayland/x11
---@field os string[]? # linux, macos, windows, veyra, ...
---@field pre_install fun(ctx: RivetBuildContext)?
---@field build fun(ctx: RivetBuildContext)?
---@field install fun(ctx: RivetBuildContext)?
---@field post_install fun(ctx: RivetBuildContext)?
---@field uninstall fun(ctx: RivetBuildContext)?
---@field provides_check RivetProviderCheck?

---Defines a Rivet package recipe. Call exactly once per file.
---@param def RivetPackageDef
function package(def) end
