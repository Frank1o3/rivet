# rivet

Workspace for "rivet" — a Rust workspace containing a CLI, core libraries, package/repository tooling, resolver logic, and an optional terminal UI. This repository is organized as a Cargo workspace with multiple crates that work together to provide the rivet toolset.

Version: 0.2.1  
License: BSD-3-Clause  
Maintainer: Frank1o3

## Contents

- crates/rivet-cli — command-line interface
- crates/rivet-core — core library/shared types and helpers
- crates/rivet-package — package handling and archive utilities
- crates/rivet-repository — repository-related logic and tooling
- crates/rivet-resolver — dependency / package resolution logic
- crates/rivet-tui — terminal UI (TUI) front-end

See Cargo.toml at the repository root for workspace configuration and dependency pins.

## Goals / Overview

This workspace groups the pieces needed for building and experimenting with rivet tooling. Each crate is focused and re-usable:

- rivet-core: shared primitives, types, utilities
- rivet-package: pack/unpack, archive and format handling
- rivet-repository: repository layout and higher-level operations
- rivet-resolver: resolution strategies, semver handling, dependency lookup
- rivet-cli: user-facing command-line tool that wires together the libraries
- rivet-tui: optional terminal UI frontend for interactive use

(If you want a more specific project description or user-facing feature list, tell me what rivet is intended to do and I’ll update this section.)

## Requirements

- Rust toolchain (rustup recommended). The workspace uses 2024 edition in Cargo.toml; recommended stable rust >= recent stable that supports the 2024 edition.
- Typical build tools for crates that may use native libs (e.g., build-essential on Linux). Consult crate-specific docs if a native dependency fails to build.

## Quickstart

1. Install Rust (if not already):
   - curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   - rustup default stable

2. Build the entire workspace:
   - cargo build --workspace --release

3. Run the CLI (example):
   - cargo run -p rivet-cli -- [ARGS]
   Replace [ARGS] with the CLI arguments the rivet-cli supports (see subcommands/help).

4. Run the TUI (if you want to try the terminal UI):
   - cargo run -p rivet-tui -- [ARGS]

5. Run tests:
   - cargo test --workspace

## Contributing

- Open issues describing bugs, feature requests, or improvements.
- Create PRs against main. Keep changes small and focused.
- Write tests for bug fixes or new behavior.
- Follow Rust idioms and use workspace dependencies where possible.
- If you need to change public crate APIs, consider semver and document breaking changes.

If you’d like, I can add a CONTRIBUTING.md with a more formal contribution guide.

## License

BSD-3-Clause — see the LICENSE file in this repository.