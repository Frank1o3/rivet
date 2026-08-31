use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "rivet",
    about = "A general-purpose, cross-platform package manager",
    version
)]
pub struct Cli {
    /// Optional path to a local repository directory (defaults to current directory or ./packages)
    #[arg(short, long, global = true)]
    pub repo: Option<PathBuf>,

    /// Path to the installed package database (defaults to .rivet/db.json)
    #[arg(long, global = true)]
    pub db: Option<PathBuf>,

    #[arg(long, global = true)]
    pub prefix: Option<PathBuf>,

    #[arg(long, global = true)]
    pub cache: Option<PathBuf>,

    #[arg(long, global = true)]
    pub system: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Set up Rivet's data directory and configure the official repository
    Init,

    /// Install packages by resolving dependencies and executing build/install hooks
    Install {
        /// Package name(s) or constraints to install (e.g. "neovim", "zlib >= 1.3")
        #[arg(required = true)]
        packages: Vec<String>,

        /// Preview the resolution plan without executing actions
        #[arg(short = 'd', long)]
        dry_run: bool,

        /// Enable specific features for the requested packages
        #[arg(short = 'F', long)]
        feature: Vec<String>,
    },

    /// Remove an installed package and its tracked files
    Remove {
        /// Name of the package to remove
        package: String,

        /// Force removal even if other installed packages depend on it
        #[arg(short = 'f', long)]
        force: bool,
    },

    /// Remove orphaned packages that were installed as dependencies but are no longer needed
    Autoremove {
        /// Preview orphaned packages without removing them
        #[arg(short = 'd', long)]
        dry_run: bool,
    },

    /// List all currently installed packages
    List,

    /// Search for packages across available repositories
    Search {
        /// Query string to search in package names and descriptions
        query: String,
    },

    /// Display detailed information about a package
    Info {
        /// Package name
        package: String,
    },

    /// Synchronize and index repository package definitions
    Sync,

    /// Parse, validate, and check a local Lua package recipe
    Build {
        /// Path to the .lua recipe file
        recipe: PathBuf,

        /// Check metadata only without attempting build
        #[arg(long)]
        check_only: bool,
    },

    /// Fetch the latest package information from all configured remote repositories
    Update,

    /// Upgrade installed packages to newer versions found during the last update
    Upgrade {
        /// Optional package name(s) to upgrade (if omitted, all eligible packages are checked)
        #[arg()]
        packages: Vec<String>,

        /// Preview available upgrades without making changes
        #[arg(short = 'd', long)]
        dry_run: bool,
    },

    /// Remove disposable cached downloads, sources, and build artifacts
    Clean,

    /// Verify file integrity and presence for installed packages
    Verify {
        /// Optional package name(s) to verify (verifies all installed packages if omitted)
        #[arg()]
        packages: Vec<String>,
    },

    /// Manage package repositories (list, add, remove, enable, disable)
    Repo {
        #[command(subcommand)]
        command: RepoCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum RepoCommands {
    /// List all configured local and remote repositories
    List,

    /// Add a new remote repository definition
    Add {
        /// Repository identifier / slug (e.g. "community", "extra")
        slug: String,

        /// Remote Git repository URL
        url: String,

        /// Git branch to follow (defaults to "main")
        #[arg(short, long)]
        branch: Option<String>,

        /// Subdirectory containing index.json and packages/
        #[arg(short, long)]
        path: Option<String>,

        /// Repository priority (higher values take precedence, defaults to 10)
        #[arg(short = 'P', long)]
        priority: Option<i32>,

        /// Human-readable display name (defaults to slug)
        #[arg(short, long)]
        name: Option<String>,

        /// Repository description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Remove a configured repository definition
    #[command(alias = "rm")]
    Remove {
        /// Repository slug to remove
        slug: String,
    },

    /// Enable a disabled repository
    Enable {
        /// Repository slug to enable
        slug: String,
    },

    /// Disable an active repository without removing its definition
    Disable {
        /// Repository slug to disable
        slug: String,
    },
}
