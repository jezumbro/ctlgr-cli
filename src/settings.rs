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
/// in the chain (.ctlgr.local, .ctlgr at each level, then the optional
/// `global` path). Returns a deduplicated merged list; first occurrence wins.
fn collect_excluded_from(start: &Path, global: Option<&Path>) -> Vec<String> {
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
    if let Some(global_path) = global {
        if let Ok(cfg) = load_from(global_path) {
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
    let global = global_config_path().ok();
    let mut settings = load_from(&find_config_from(&cwd)?)?;
    settings.excluded = collect_excluded_from(&cwd, global.as_deref());
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

/// Apply all global settings.json defaults and migrations in a single pass:
///   1. Migrate legacy `paths` array → `path`
///   2. Seed `excluded: ["AGENTS\\.md"]` if the key is absent
///   3. Seed `lint` defaults if the key is absent
/// Reads the file once and writes at most once (only if something changed).
/// Creates the file if it doesn't exist. Silently ignores errors.
pub fn ensure_global_defaults() {
    let Ok(global) = global_config_path() else { return };
    ensure_global_defaults_at(&global);
}

fn ensure_global_defaults_at(path: &Path) {
    let content = if path.exists() {
        match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return,
        }
    } else {
        "{}".to_string()
    };
    let Ok(raw) = serde_json::from_str::<serde_json::Value>(&content) else { return };
    let Ok(mut cfg) = serde_json::from_str::<Settings>(&content) else { return };
    let mut dirty = false;

    // 1. Migrate legacy paths → path
    if cfg.path.is_none() {
        if let Ok(legacy) = serde_json::from_str::<LegacySettings>(&content) {
            if let Some(first) = legacy.paths.into_iter().next() {
                cfg.path = Some(first);
                dirty = true;
            }
        }
    }

    // 2. Seed excluded default (only if key is entirely absent)
    if raw.get("excluded").is_none() {
        cfg.excluded = vec!["AGENTS\\.md".into()];
        dirty = true;
    }

    // 3. Seed lint defaults
    if cfg.lint.is_none() {
        cfg.lint = Some(LintConfig::default());
        dirty = true;
    }

    if dirty {
        let _ = write_to(&cfg, path);
    }
}

#[cfg(test)]
mod tests_collect_excluded {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn merges_excluded_from_global_path() {
        let tmp = TempDir::new().unwrap();
        let global = tmp.path().join("global.json");
        write_to(
            &Settings { path: None, lint: None, excluded: vec!["global-pattern".into()] },
            &global,
        )
        .unwrap();
        let result = collect_excluded_from(tmp.path(), Some(&global));
        assert!(result.contains(&"global-pattern".to_string()));
    }

    #[test]
    fn deduplicates_between_project_and_global() {
        let tmp = TempDir::new().unwrap();
        write_to(
            &Settings { path: None, lint: None, excluded: vec!["shared".into()] },
            &tmp.path().join(".ctlgr"),
        )
        .unwrap();
        let global = tmp.path().join("global.json");
        write_to(
            &Settings { path: None, lint: None, excluded: vec!["shared".into()] },
            &global,
        )
        .unwrap();
        let result = collect_excluded_from(tmp.path(), Some(&global));
        assert_eq!(result.iter().filter(|p| *p == "shared").count(), 1);
    }

    #[test]
    fn skips_malformed_config_file() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".ctlgr"), "not json").unwrap();
        let result = collect_excluded_from(tmp.path(), None);
        assert!(result.is_empty());
    }

    #[test]
    fn no_global_path_is_safe() {
        let tmp = TempDir::new().unwrap();
        let result = collect_excluded_from(tmp.path(), None);
        assert!(result.is_empty());
    }

    #[test]
    fn malformed_global_file_is_skipped() {
        let tmp = TempDir::new().unwrap();
        let global = tmp.path().join("global.json");
        std::fs::write(&global, "not json").unwrap();
        let result = collect_excluded_from(tmp.path(), Some(&global));
        assert!(result.is_empty());
    }
}

#[cfg(test)]
mod tests_global_defaults {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn seeds_all_defaults_on_empty_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(&path, "{}").unwrap();
        ensure_global_defaults_at(&path);
        let loaded = load_from(&path).unwrap();
        assert_eq!(loaded.excluded, vec!["AGENTS\\.md"]);
        assert!(loaded.lint.is_some());
    }

    #[test]
    fn creates_file_with_defaults_when_missing() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("settings.json");
        ensure_global_defaults_at(&path);
        let loaded = load_from(&path).unwrap();
        assert_eq!(loaded.excluded, vec!["AGENTS\\.md"]);
        assert!(loaded.lint.is_some());
    }

    #[test]
    fn migrates_legacy_paths_to_path() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(&path, r#"{"paths":["/catalog","/other"]}"#).unwrap();
        ensure_global_defaults_at(&path);
        let loaded = load_from(&path).unwrap();
        assert_eq!(loaded.path, Some("/catalog".into()));
    }

    #[test]
    fn migration_preserves_lint_from_legacy_json() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{"paths":["/catalog"],"lint":{"rules":["no-style-blocks"]}}"#,
        )
        .unwrap();
        ensure_global_defaults_at(&path);
        let loaded = load_from(&path).unwrap();
        assert_eq!(loaded.path, Some("/catalog".into()));
        assert_eq!(loaded.lint.unwrap().rules, vec!["no-style-blocks"]);
    }

    #[test]
    fn skips_migration_when_path_already_set() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(&path, r#"{"path":"/current","excluded":[],"lint":{"rules":[]}}"#).unwrap();
        ensure_global_defaults_at(&path);
        let loaded = load_from(&path).unwrap();
        assert_eq!(loaded.path, Some("/current".into()));
    }

    #[test]
    fn does_not_overwrite_existing_excluded() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(&path, r#"{"excluded":["custom\\.md"],"lint":{"rules":[]}}"#).unwrap();
        ensure_global_defaults_at(&path);
        let loaded = load_from(&path).unwrap();
        assert_eq!(loaded.excluded, vec!["custom\\.md"]);
    }

    #[test]
    fn respects_intentionally_empty_excluded() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(&path, r#"{"excluded":[],"lint":{"rules":[]}}"#).unwrap();
        ensure_global_defaults_at(&path);
        let loaded = load_from(&path).unwrap();
        assert!(loaded.excluded.is_empty());
    }

    #[test]
    fn no_write_when_all_defaults_already_set() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("settings.json");
        let initial = r#"{"excluded":["AGENTS\\.md"],"lint":{"rules":["no-style-blocks"]}}"#;
        std::fs::write(&path, initial).unwrap();
        let mtime_before = std::fs::metadata(&path).unwrap().modified().unwrap();
        ensure_global_defaults_at(&path);
        let mtime_after = std::fs::metadata(&path).unwrap().modified().unwrap();
        assert_eq!(mtime_before, mtime_after, "file should not be rewritten");
    }

    #[test]
    fn empty_paths_array_leaves_path_as_none() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(&path, r#"{"paths":[]}"#).unwrap();
        ensure_global_defaults_at(&path);
        let loaded = load_from(&path).unwrap();
        assert!(loaded.path.is_none());
        assert!(loaded.lint.is_some());
    }

    #[test]
    fn malformed_paths_field_is_skipped_during_migration() {
        // "paths" is not an array — LegacySettings fails to parse, so migration
        // is skipped; defaults are still seeded.
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(&path, r#"{"paths":"not-an-array"}"#).unwrap();
        ensure_global_defaults_at(&path);
        let loaded = load_from(&path).unwrap();
        assert!(loaded.path.is_none()); // not migrated
        assert!(loaded.lint.is_some()); // lint defaults seeded
    }

    #[test]
    fn returns_early_on_unreadable_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("settings.json");
        // Writing a directory at the path makes read_to_string fail.
        std::fs::create_dir(&path).unwrap();
        ensure_global_defaults_at(&path); // must not panic
        // The "file" is still a directory — nothing was written.
        assert!(path.is_dir());
    }
}

/// Ensure the resolved project config contains a `lint` section. Skips if the
/// resolved config is the global settings.json — that is handled by
/// `ensure_global_defaults`. Silently ignores errors.
pub fn ensure_lint_defaults() {
    let Ok(cwd) = std::env::current_dir() else { return };
    let Ok(config_path) = find_config_from(&cwd) else { return };
    // Global settings handled in ensure_global_defaults — avoid double read/write.
    if global_config_path().map_or(false, |g| g == config_path) {
        return;
    }
    let Ok(mut cfg) = load_from(&config_path) else { return };
    if cfg.lint.is_none() {
        cfg.lint = Some(LintConfig::default());
        let _ = write_to(&cfg, &config_path);
    }
}
