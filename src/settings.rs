use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LintConfig {
    pub rules: Vec<String>,
}

impl LintConfig {
    pub fn default_rules() -> Vec<String> {
        vec![
            "no-style-blocks".to_string(),
            "no-inline-styles".to_string(),
            "prefer-html".to_string(),
        ]
    }

    pub fn is_enabled(&self, rule: &str) -> bool {
        self.rules.iter().any(|r| r == rule)
    }
}

impl Default for LintConfig {
    fn default() -> Self {
        Self { rules: Self::default_rules() }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lint: Option<LintConfig>,
}

pub fn default_notes_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".ctlgr-cli")
        .join("notes")
}

fn global_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".ctlgr-cli").join("settings.json"))
}

pub fn find_config_from(start: &Path) -> Result<PathBuf> {
    let mut dir = start;
    loop {
        let candidate = dir.join(".ctlgr");
        if candidate.exists() {
            return Ok(candidate);
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }
    }
    global_config_path()
}

pub fn config_path_from(cwd: &Path) -> PathBuf {
    cwd.join(".ctlgr")
}

pub fn config_path() -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("could not determine current directory")?;
    Ok(config_path_from(&cwd))
}

pub fn load_from(path: &Path) -> Result<Settings> {
    if !path.exists() {
        return Ok(Settings::default());
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("parsing {}", path.display()))
}

pub fn load() -> Result<Settings> {
    let cwd = std::env::current_dir().context("could not determine current directory")?;
    load_from(&find_config_from(&cwd)?)
}

pub fn save(settings: &Settings) -> Result<()> {
    let cwd = std::env::current_dir().context("could not determine current directory")?;
    write_to(settings, &find_config_from(&cwd)?)
}

pub fn write_to(settings: &Settings, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(settings)?;
    std::fs::write(path, content)
        .with_context(|| format!("writing {}", path.display()))
}

/// Resolve the catalog directory: configured path, or ~/.ctlgr-cli/notes/.
pub fn resolve_path(settings: &Settings) -> PathBuf {
    settings.path.as_deref().map(PathBuf::from).unwrap_or_else(default_notes_dir)
}

/// Expand the resolved catalog directory into a list of .html and .md files.
pub fn expand_path(settings: &Settings) -> Result<Vec<String>> {
    let dir = resolve_path(settings);
    let dir_str = dir.to_string_lossy();
    let mut files = Vec::new();
    for ext in &["html", "md"] {
        let pattern = format!("{dir_str}/**/*.{ext}");
        let entries = glob::glob(&pattern)
            .with_context(|| format!("invalid glob pattern: {pattern}"))?;
        for entry in entries {
            let path = entry.with_context(|| format!("expanding {pattern}"))?;
            files.push(path.to_string_lossy().into_owned());
        }
    }
    Ok(files)
}

/// Ensure the resolved config file contains a `lint` section. If the file
/// exists but the key is absent, write the defaults in place. Silently ignores
/// all errors — this is a best-effort migration on every invocation.
pub fn ensure_lint_defaults() {
    let Ok(cwd) = std::env::current_dir() else { return };
    let Ok(config_path) = find_config_from(&cwd) else { return };
    if !config_path.exists() {
        return;
    }
    let Ok(mut cfg) = load_from(&config_path) else { return };
    if cfg.lint.is_none() {
        cfg.lint = Some(LintConfig::default());
        let _ = write_to(&cfg, &config_path);
    }
}
