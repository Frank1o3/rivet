# Rivet

**Rivet** is a general-purpose, source-oriented, cross-platform package manager.

It draws its philosophy from systems like Gentoo and Nix — explicit package
definitions, inspectable local metadata, and reproducible builds — but it is
not tied to any single operating system or distribution. Rivet is designed to
work with [Veyra OS](https://github.com/Veyra), but nothing about its core
architecture assumes Veyra OS, or even Linux, specifically.

The Rivet *executable* provides the package-management machinery.
**Repositories provide package definitions.** Anyone can write their own.

```text
$ rivet search ripgrep
Found 1 package(s) matching 'ripgrep':

PACKAGE     VERSION   REPOSITORY   DESCRIPTION
ripgrep     15.2.0    rivet        Fast recursive grep replacement

$ rivet install ripgrep
🔍 Resolving dependencies for 1 package(s)...

📦 Resolved Installation Plan (2 packages):
  1. rust                14.1.0
  2. ripgrep              15.2.0

⬇️  Installing 'rust' v14.1.0...
⬇️  Installing 'ripgrep' v15.2.0...
🚀 Installation complete.
```

## Why Rivet

Most package managers fall into one of two camps: a binary catalog baked
into the tool itself, or a source-based system so tightly coupled to one
distribution that it can't be used anywhere else. Rivet tries to avoid both:

- **Explicit, inspectable package definitions.** Every package is a plain
  Lua recipe you can read before you run it — no opaque binary metadata.
- **Source-oriented by default.** Packages are built from source through
  declared `build`/`install` hooks, with room for pre-built/virtual packages
  (toolchains, meta-packages) where that makes more sense.
- **Repositories are just data.** The official repository is a curated,
  human-reviewed source of package definitions — not a hardcoded catalog
  compiled into the binary. You can add third-party repositories or write
  local packages of your own.
- **No hidden catalog.** Rivet only stores the package definitions it
  actually needs — the ones you've installed, or written yourself. `search`
  queries repositories directly instead of requiring you to mirror all of
  them locally.
- **Portable by design.** Architecture (x86_64, aarch64, ...) and platform
  (Linux, macOS, Windows) are modeled explicitly and kept separate from
  software capabilities (Wayland, X11, systemd, ...), so the core isn't
  quietly Linux-shaped.

## How a package definition looks

Package definitions are Lua recipes, sandboxed at load time (no `os`, `io`,
or filesystem access outside the API Rivet exposes):

```lua
package({
    name = "ripgrep",
    version = "15.2.0",
    description = "Fast recursive grep replacement",
    license = "MIT OR Unlicense",
    homepage = "https://github.com/BurntSushi/ripgrep",
    architectures = { "x86_64", "aarch64" },
    dependencies = { "rust" },

    source = {
        type = "archive",
        url = "https://github.com/BurntSushi/ripgrep/archive/refs/tags/15.2.0.tar.gz",
        sha256 = "7605249d3eb0d5f170e3414498e3344e26b1e7a147aec518b57090b80036a562",
    },

    build = function(ctx)
        ctx:run_in(ctx:source_dir(), "cargo", { "build", "--release", "--locked" })
    end,

    install = function(ctx)
        local dest = ctx:destdir()
        ctx:mkdir(dest .. "/usr/bin")
        ctx:copy(ctx:source_dir() .. "/target/release/rg", dest .. "/usr/bin/rg")
    end,
})
```

Rivet fetches and verifies the source, runs the declared lifecycle hooks in
a sandboxed context, and records the result in a local, human-readable
JSON database — no network access required to later inspect, verify, or
uninstall what's installed.

## Workspace layout

Rivet is a Cargo workspace of small, focused crates:

| Crate | Responsibility |
| --------------------- | ---------------- |
| [`rivet-core`](crates/rivet-core) | Shared domain types: package names, versions, targets, checksums, features, install scope, paths, and the installed-package database. |
| [`rivet-package`](crates/rivet-package) | Package-definition loading (sandboxed Lua), manifest interpretation, source fetching, and build/install execution. |
| [`rivet-repository`](crates/rivet-repository) | Repository discovery and management — local and Git-backed repositories, with room for other backends. |
| [`rivet-resolver`](crates/rivet-resolver) | Dependency and version resolution. Independent of the CLI and repository implementation; usable as a standalone crate. |
| [`rivet-cli`](crates/rivet-cli) | The `rivet` command-line frontend. Thin — it calls into the crates above rather than implementing package management itself. |
| [`rivet-tui`](crates/rivet-tui) | An interactive terminal UI built on the same underlying crates as the CLI. |

Each crate is meant to be usable on its own. In particular, `rivet-resolver`
and `rivet-core` are intentionally free of CLI- and repository-specific
concerns so they can be depended on independently.

## Repositories

Package definitions come from repositories, not from Rivet itself. A
repository is declared with a small Lua definition:

```lua
repository({
    name = "Rivet",
    description = "Official curated Rivet package repository",
    license = "BSD-3-Clause",
    source = {
        url = "https://github.com/Frank1o3/rivet-repository.git",
        branch = "main",
        path = "src",
    },
})
```

`rivet init` configures the official repository automatically so you don't
have to add it by hand, but it's a repository like any other — not a
built-in catalog. Adding a third-party repository is a statement of trust
in that repository, not an endorsement from the Rivet project. The official
repository lives in a [separate repo](https://github.com/Frank1o3/rivet-repository)
and requires human review before a package definition is merged.

## Getting started

**1. Install Rust** (Rivet is built with it, and most packages currently
build with it too):

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
```

**2. Build the workspace:**

```sh
cargo build --workspace --release
```

**3. Initialize Rivet** (sets up the data directory and configures the
official repository):

```sh
cargo run -p rivet-cli -- init
cargo run -p rivet-cli -- update
```

**4. Try it out:**

```sh
cargo run -p rivet-cli -- search ripgrep
cargo run -p rivet-cli -- install ripgrep
```

**5. Or use the TUI:**

```sh
cargo run -p rivet-tui
```

**Run the test suite:**

```sh
cargo test --workspace
```

## Project status

Rivet is under active development. Core package installation, dependency
resolution, local and remote (Git-backed) repositories, upgrades, and
autoremove/orphan cleanup are implemented. Windows and macOS support,
additional repository backends, and richer feature/capability modeling are
planned but not yet complete — see the crate-level docs for the current
state of each piece.

## License

BSD-3-Clause — see [LICENSE](LICENSE).
