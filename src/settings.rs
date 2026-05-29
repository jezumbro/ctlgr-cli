use anyhow::{Context, Result};
use regex::Regex;
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded: Vec<String>,
}

pub fn default_catalog_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".ctlgr-cli")
        .join("catalog")
}

fn global_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".ctlgr-cli").join("settings.json"))
}

/// Walk up from `start` checking `.ctlgr.local` then `.ctlgr` at each level.
/// Falls back to the global config when neither is found anywhere.
pub fn find_config_from(start: &Path) -> Result<PathBuf> {
    let mut dir = start;
    loop {
        for name in &[".ctlgr.local", ".ctlgr"] {
            let candidate = dir.join(name);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }
    }
    global_config_path()
}

pub fn config_path_from(cwd: &Path, local: bool) -> PathBuf {
    let name = if local { ".ctlgr.local" } else { ".ctlgr" };
    cwd.join(name)
}

pub fn config_path(local: bool) -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("could not determine current directory")?;
    Ok(config_path_from(&cwd, local))
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

/// Walk up from `start` collecting `excluded` patterns from every config file
/// in the chain (.ctlgr.local, .ctlgr at each level, then global settings).
/// Returns a deduplicated merged list; first occurrence of each pattern wins.
fn collect_excluded_from(start: &Path) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut patterns = Vec::new();
    let mut dir = start;
    loop {
        for name in &[".ctlgr.local", ".ctlgr"] {
            let candidate = dir.join(name);
            if let Ok(cfg) = load_from(&candidate) {
                for p in cfg.excluded {
                    if seen.insert(p.clone()) {
                        patterns.push(p);
                    }
                }
            }
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }
    }
    if let Ok(global) = global_config_path() {
        if let Ok(cfg) = load_from(&global) {
            for p in cfg.excluded {
                if seen.insert(p.clone()) {
                    patterns.push(p);
                }
            }
        }
    }
    patterns
}

pub fn load() -> Result<Settings> {
    let cwd = std::env::current_dir().context("could not determine current directory")?;
    let mut settings = load_from(&find_config_from(&cwd)?)?;
    settings.excluded = collect_excluded_from(&cwd);
    Ok(settings)
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

/// Resolve the catalog directory: configured path, or ~/.ctlgr-cli/catalog/.
pub fn resolve_path(settings: &Settings) -> PathBuf {
    settings.path.as_deref().map(PathBuf::from).unwrap_or_else(default_catalog_dir)
}

/// Expand the resolved catalog directory into a list of .html and .md files.
/// Files whose path matches any pattern in `settings.excluded` are omitted.
/// Invalid regex patterns are silently skipped.
pub fn expand_path(settings: &Settings) -> Result<Vec<String>> {
    let dir = resolve_path(settings);
    let dir_str = dir.to_string_lossy();
    let excludes: Vec<Regex> =
        settings.excluded.iter().filter_map(|p| Regex::new(p).ok()).collect();
    let mut files = Vec::new();
    for ext in &["html", "md"] {
        let pattern = format!("{dir_str}/**/*.{ext}");
        let entries = glob::glob(&pattern)
            .with_context(|| format!("invalid glob pattern: {pattern}"))?;
        for entry in entries {
            let path = entry.with_context(|| format!("expanding {pattern}"))?;
            let path_str = path.to_string_lossy();
            if excludes.iter().any(|re| re.is_match(&path_str)) {
                continue;
            }
            files.push(path_str.into_owned());
        }
    }
    Ok(files)
}

#[derive(Deserialize)]
struct LegacySettings {
    #[serde(default)]
    paths: Vec<String>,
    lint: Option<LintConfig>,
}

/// Migrate legacy config files in CWD to the new format. Each file is handled
/// independently: `.ctlgr.local.json` → `.ctlgr.local`, `.ctlgr.json` → `.ctlgr`.
/// Skips a file if its new-format target already exists. Silently ignores errors.
pub fn migrate_legacy_config() {
    let Ok(cwd) = std::env::current_dir() else { return };
    for (legacy_name, new_name) in
        &[(".ctlgr.local.json", ".ctlgr.local"), (".ctlgr.json", ".ctlgr")]
    {
        let legacy = cwd.join(legacy_name);
        let new_config = cwd.join(new_name);
        if !legacy.exists() || new_config.exists() {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&legacy) else { continue };
        let Ok(old) = serde_json::from_str::<LegacySettings>(&content) else { continue };
        let new = Settings { path: old.paths.into_iter().next(), lint: old.lint, excluded: vec![] };
        if write_to(&new, &new_config).is_ok() {
            let _ = std::fs::remove_file(&legacy);
        }
    }
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
