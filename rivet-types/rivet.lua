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

---Sets an environment variable for subsequent `run` calls within the
---*same* hook invocation. Does not currently carry over between hooks.
---@param key string
---@param value string
function BuildContext:set_env(key, value) end

---Runs a subprocess inside the build directory.
---@param cmd string
---@param args string[]?
function BuildContext:run(cmd, args) end

---Copies a file, creating parent directories as needed.
---@param src string
---@param dst string
function BuildContext:copy(src, dst) end

---Creates a directory, and its parents, at `path`.
---@param path string
function BuildContext:mkdir(path) end

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

---Defines a Rivet package recipe. Call exactly once per file.
---@param def RivetPackageDef
function package(def) end
