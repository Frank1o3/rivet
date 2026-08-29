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
}
