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
    /// Create a .ctlgr config file in the current directory with a catalog path
    Init {
        /// Catalog directory to register
        path: String,
        /// Create .ctlgr.local instead (gitignored, higher priority than .ctlgr)
        #[arg(long)]
        local: bool,
    },
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
        ConfigCommands::Init { path, local } => {
            let p = std::path::Path::new(&path);
            anyhow::ensure!(p.exists(), "path does not exist: {path}");
            anyhow::ensure!(p.is_dir(), "path is not a directory: {path}");
            let config_path = settings::config_path(local)?;
            anyhow::ensure!(
                !config_path.exists(),
                "{} already exists at {}",
                config_path.file_name().unwrap().to_string_lossy(),
                config_path.display()
            );
            let cfg = settings::Settings { path: Some(path), lint: None };
            settings::write_to(&cfg, &config_path)?;
            println!("created: {}", config_path.display());
        }
        ConfigCommands::List => {
            let cfg = settings::load()?;
            let path = settings::resolve_path(&cfg);
            if cfg.path.is_none() {
                println!("(default) {}", path.display());
            } else {
                println!("{}", path.display());
            }
        }
    }
    Ok(())
}
