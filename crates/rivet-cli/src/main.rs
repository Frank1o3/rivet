mod args;
mod commands;
mod repo_helper;

use clap::Parser;
use rivet_core::InstalledDatabase;

fn main() -> anyhow::Result<()> {
    let cli = args::Cli::parse();

    match cli.command {
        args::Commands::Install {
            packages,
            dry_run,
            feature,
        } => {
            let repos = repo_helper::load_repositories(cli.repo.as_deref())?;

            let scope = if cli.system {
                rivet_core::InstallScope::System
            } else {
                rivet_core::InstallScope::User
            };
            scope.check_permitted()?;

            let db_path = match cli.db {
                Some(path) => rivet_core::absolute_path(path)?,
                None => scope.default_db_path()?,
            };
            let prefix = match cli.prefix {
                Some(path) => rivet_core::absolute_path(path)?,
                None => scope.default_prefix()?,
            };
            let cache_dir = match cli.cache {
                Some(path) => rivet_core::absolute_path(path)?,
                None => rivet_core::default_source_cache()?,
            };

            let mut db = InstalledDatabase::open(db_path)?;
            commands::install::execute(
                &repos, &packages, dry_run, &feature, &mut db, &prefix, &cache_dir,
            )?;
        }
        args::Commands::Search { query } => {
            let repos = repo_helper::load_repositories(cli.repo.as_deref())?;
            commands::search::execute(&repos, &query)?;
        }
        args::Commands::Info { package } => {
            let repos = repo_helper::load_repositories(cli.repo.as_deref())?;
            commands::info::execute(&repos, &package)?;
        }
        args::Commands::Sync => {
            let mut repos = repo_helper::load_repositories(cli.repo.as_deref())?;
            commands::sync::execute(&mut repos)?;
        }
        args::Commands::Build { recipe, check_only } => {
            commands::build::execute(&recipe, check_only)?;
        }
        args::Commands::Remove { package } => {
            let scope = if cli.system {
                rivet_core::InstallScope::System
            } else {
                rivet_core::InstallScope::User
            };
            scope.check_permitted()?;

            let db_path = match cli.db {
                Some(path) => rivet_core::absolute_path(path)?,
                None => scope.default_db_path()?,
            };
            let mut db = InstalledDatabase::open(db_path)?;
            let prefix = match cli.prefix {
                Some(path) => rivet_core::absolute_path(path)?,
                None => scope.default_prefix()?,
            };
            let cache_dir = match cli.cache {
                Some(path) => rivet_core::absolute_path(path)?,
                None => rivet_core::default_source_cache()?,
            };
            commands::remove::execute(&mut db, &package, &cache_dir, &prefix)?;
        }
        args::Commands::List => {
            let scope = if cli.system {
                rivet_core::InstallScope::System
            } else {
                rivet_core::InstallScope::User
            };
            let db_path = match cli.db {
                Some(path) => rivet_core::absolute_path(path)?,
                None => scope.default_db_path()?,
            };
            let db = InstalledDatabase::open(db_path)?;
            commands::list::execute(&db)?;
        }
        args::Commands::Update => {
            let mut repos = repo_helper::load_repositories(cli.repo.as_deref())?;
            commands::update::execute(&mut repos)?;
        }
        args::Commands::Upgrade { dry_run } => {
            let repos = repo_helper::load_repositories(cli.repo.as_deref())?;

            let scope = if cli.system {
                rivet_core::InstallScope::System
            } else {
                rivet_core::InstallScope::User
            };
            scope.check_permitted()?;

            let db_path = match cli.db {
                Some(path) => rivet_core::absolute_path(path)?,
                None => scope.default_db_path()?,
            };
            let prefix = match cli.prefix {
                Some(path) => rivet_core::absolute_path(path)?,
                None => scope.default_prefix()?,
            };
            let cache_dir = match cli.cache {
                Some(path) => rivet_core::absolute_path(path)?,
                None => rivet_core::default_source_cache()?,
            };

            let mut db = InstalledDatabase::open(db_path)?;
            commands::upgrade::execute(&repos, &mut db, &prefix, &cache_dir, dry_run)?;
        }
    }

    Ok(())
}
