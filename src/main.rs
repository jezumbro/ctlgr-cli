use ctlgr::{lint, search, settings, update};

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ctlgr-cli", about = "HTML catalog — create and search .html files")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Search HTML files using CSS selectors
    Search(search::SearchArgs),
    /// Manage settings
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Check for a newer version and upgrade if one is available
    Update,
    /// Lint catalog HTML files for style violations
    Lint(lint::LintArgs),
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Create a .ctlgr config file in the current directory
    Init,
    /// Set the catalog directory for this config (replaces any existing value)
    Add {
        /// Directory path to use as the catalog root
        path: String,
    },
    /// Clear the configured catalog directory
    Remove,
    /// Show the resolved catalog path
    List,
}

fn main() -> Result<()> {
    update::check_and_notify();
    settings::migrate_legacy_config();
    settings::ensure_lint_defaults();
    let cli = Cli::parse();
    match cli.command {
        Commands::Search(args) => search::run(&args),
        Commands::Config { command } => run_config(command),
        Commands::Update => update::run_update(),
        Commands::Lint(args) => lint::run(&args),
    }
}

fn run_config(cmd: ConfigCommands) -> Result<()> {
    match cmd {
        ConfigCommands::Init => {
            let path = settings::config_path()?;
            anyhow::ensure!(
                !path.exists(),
                "{} already exists at {}",
                path.file_name().unwrap().to_string_lossy(),
                path.display()
            );
            settings::write_to(&settings::Settings::default(), &path)?;
            println!("created: {}", path.display());
        }
        ConfigCommands::Add { path } => {
            let p = std::path::Path::new(&path);
            anyhow::ensure!(p.exists(), "path does not exist: {path}");
            anyhow::ensure!(p.is_dir(), "path is not a directory: {path}");
            let mut cfg = settings::load()?;
            if cfg.path.as_deref() == Some(&path) {
                println!("already registered: {path}");
            } else {
                cfg.path = Some(path.clone());
                settings::save(&cfg)?;
                println!("added: {path}");
            }
        }
        ConfigCommands::Remove => {
            let mut cfg = settings::load()?;
            if cfg.path.is_none() {
                println!("no path configured");
            } else {
                cfg.path = None;
                settings::save(&cfg)?;
                println!("removed");
            }
        }
        ConfigCommands::List => {
            let cfg = settings::load()?;
            let path = settings::resolve_path(&cfg);
            let is_default = cfg.path.is_none();
            if is_default {
                println!("(default) {}", path.display());
            } else {
                println!("{}", path.display());
            }
        }
    }
    Ok(())
}
