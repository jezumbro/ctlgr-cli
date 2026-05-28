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
    /// Create a .ctlgr.json in the current directory (takes priority over global config)
    Init {
        /// Create .ctlgr.local.json instead (gitignored, takes priority over .ctlgr.json)
        #[arg(long)]
        local: bool,
    },
    /// Add a directory to the search catalog (searches *.html and *.md recursively)
    Add {
        /// Directory path to register
        path: String,
    },
    /// Remove a directory from the search catalog
    Remove {
        /// Directory path to remove
        path: String,
    },
    /// List configured search paths
    List,
}

fn main() -> Result<()> {
    update::check_and_notify();
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
        ConfigCommands::Init { local } => {
            let path = settings::local_config_path(local)?;
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
            if cfg.paths.contains(&path) {
                println!("already registered: {path}");
            } else {
                cfg.paths.push(path.clone());
                settings::save(&cfg)?;
                println!("added: {path}");
            }
        }
        ConfigCommands::Remove { path } => {
            let mut cfg = settings::load()?;
            let before = cfg.paths.len();
            cfg.paths.retain(|p| p != &path);
            if cfg.paths.len() == before {
                println!("not found: {path}");
            } else {
                settings::save(&cfg)?;
                println!("removed: {path}");
            }
        }
        ConfigCommands::List => {
            let cfg = settings::load()?;
            if cfg.paths.is_empty() {
                println!("no paths configured");
                println!("hint: run `ctlgr config add <path>` to register a search path");
            } else {
                for path in &cfg.paths {
                    println!("{path}");
                }
            }
        }
    }
    Ok(())
}
