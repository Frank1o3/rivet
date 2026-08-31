---@meta

---Git source for a repository's package definitions.
---Repositories are always Git-based; `url` and `branch` are required,
---and `path` points to a subdirectory within the repo that contains
---`index.json` and `packages/`. Omit `path` (or set it to nil) to use
---the repo root.
---@class RivetRepositorySource
---@field url string    # HTTPS or SSH URL of the git repository.
---@field branch string? # Branch to track. Defaults to "main" when omitted.
---@field path string?  # Subdirectory within the repo. Nil means repo root.

---@class RivetRepositoryDef
---@field name string           # Human-readable display name, e.g. "Rivet Official".
---@field description string?   # Short description of the repository's purpose.
---@field license string?       # SPDX license identifier for the repo's package recipes.
---@field source RivetRepositorySource
---@field priority integer?     # Lookup precedence — higher wins. Defaults to 10.
---@field enabled boolean?      # Whether the repository is active. Defaults to true.

---Defines a Rivet repository. Call exactly once per file.
---@param def RivetRepositoryDef
function repository(def) end
