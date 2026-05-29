use std::path::Path;
use tempfile::TempDir;

use ctlgr::settings::{
    config_path_from, default_catalog_dir, ensure_lint_defaults, expand_path, find_config_from,
    load, load_from, migrate_legacy_config, resolve_path, write_to, LintConfig, Settings,
};


#[test]
fn settings_default_has_no_path() {
    assert!(Settings::default().path.is_none());
}

#[test]
fn settings_roundtrip() {
    let s = Settings { path: Some("/catalog".into()), lint: None, excluded: vec![] };
    let json = serde_json::to_string(&s).unwrap();
    let s2: Settings = serde_json::from_str(&json).unwrap();
    assert_eq!(s2.path, s.path);
}

#[test]
fn settings_missing_path_field_defaults_none() {
    let s: Settings = serde_json::from_str("{}").unwrap();
    assert!(s.path.is_none());
}

#[test]
fn config_path_from_returns_ctlgr() {
    let p = config_path_from(Path::new("/some/dir"), false);
    assert_eq!(p, Path::new("/some/dir/.ctlgr"));
}

#[test]
fn config_path_from_local_returns_ctlgr_local() {
    let p = config_path_from(Path::new("/some/dir"), true);
    assert_eq!(p, Path::new("/some/dir/.ctlgr.local"));
}

#[test]
fn find_config_from_finds_ctlgr() {
    let tmp = TempDir::new().unwrap();
    let config = tmp.path().join(".ctlgr");
    std::fs::write(&config, "{}").unwrap();
    assert_eq!(find_config_from(tmp.path()).unwrap(), config);
}

#[test]
fn find_config_from_finds_ctlgr_local() {
    let tmp = TempDir::new().unwrap();
    let local = tmp.path().join(".ctlgr.local");
    std::fs::write(&local, "{}").unwrap();
    assert_eq!(find_config_from(tmp.path()).unwrap(), local);
}

#[test]
fn find_config_from_local_beats_ctlgr_at_same_level() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join(".ctlgr.local"), "{}").unwrap();
    std::fs::write(tmp.path().join(".ctlgr"), "{}").unwrap();
    let found = find_config_from(tmp.path()).unwrap();
    assert_eq!(found.file_name().unwrap(), ".ctlgr.local");
}

#[test]
fn find_config_from_walks_up() {
    let tmp = TempDir::new().unwrap();
    let sub = tmp.path().join("a").join("b");
    std::fs::create_dir_all(&sub).unwrap();
    let config = tmp.path().join(".ctlgr");
    std::fs::write(&config, "{}").unwrap();
    assert_eq!(find_config_from(&sub).unwrap(), config);
}

#[test]
fn find_config_from_exhausts_names_at_level_before_ascending() {
    // sub has .ctlgr but not .ctlgr.local; parent has .ctlgr.local
    // should return sub/.ctlgr, not ascend to parent's .ctlgr.local
    let tmp = TempDir::new().unwrap();
    let sub = tmp.path().join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join(".ctlgr"), "{}").unwrap();
    std::fs::write(tmp.path().join(".ctlgr.local"), "{}").unwrap();
    assert_eq!(find_config_from(&sub).unwrap(), sub.join(".ctlgr"));
}

#[test]
fn find_config_from_stops_at_nearest_ancestor() {
    let tmp = TempDir::new().unwrap();
    let sub = tmp.path().join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join(".ctlgr"), "{}").unwrap();
    std::fs::write(tmp.path().join(".ctlgr"), "{}").unwrap();
    assert_eq!(find_config_from(&sub).unwrap(), sub.join(".ctlgr"));
}

#[test]
fn find_config_from_falls_back_to_global() {
    let found = find_config_from(Path::new("/")).unwrap();
    assert!(found.to_string_lossy().contains(".ctlgr-cli"));
    assert_eq!(found.file_name().unwrap(), "settings.json");
}

#[test]
fn write_to_creates_parent_dirs() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("nested").join("deep").join("c.json");
    write_to(&Settings::default(), &path).unwrap();
    assert!(path.exists());
}

#[test]
fn write_to_serializes_and_load_from_roundtrips() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("c.json");
    let s = Settings { path: Some("/mypath".into()), lint: None, excluded: vec![] };
    write_to(&s, &path).unwrap();
    let s2 = load_from(&path).unwrap();
    assert_eq!(s2.path, Some("/mypath".into()));
}

#[test]
fn load_from_missing_file_returns_default() {
    let tmp = TempDir::new().unwrap();
    let s = load_from(&tmp.path().join("nonexistent.json")).unwrap();
    assert!(s.path.is_none());
}

#[test]
fn load_from_reads_valid_file() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("c.json");
    std::fs::write(&path, r#"{"path":"/foo"}"#).unwrap();
    let s = load_from(&path).unwrap();
    assert_eq!(s.path, Some("/foo".into()));
}

#[test]
fn load_from_errors_on_invalid_json() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("c.json");
    std::fs::write(&path, "not json").unwrap();
    assert!(load_from(&path).is_err());
}

// ── save ───────────────────────────────────────────────────────────────────

#[test]
fn save_writes_to_resolved_config() {
    let _guard = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let config = tmp.path().join(".ctlgr");
    write_to(&Settings { path: None, lint: None, excluded: vec![] }, &config).unwrap();
    std::env::set_current_dir(&tmp).unwrap();
    let mut cfg = load_from(&config).unwrap();
    cfg.path = Some("/saved-path".into());
    ctlgr::settings::save(&cfg).unwrap();
    let loaded = load_from(&config).unwrap();
    assert_eq!(loaded.path, Some("/saved-path".into()));
}

// ── resolve_path ───────────────────────────────────────────────────────────

#[test]
fn resolve_path_returns_configured_path() {
    let s = Settings { path: Some("/catalog".into()), lint: None, excluded: vec![] };
    assert_eq!(resolve_path(&s).to_string_lossy(), "/catalog");
}

#[test]
fn resolve_path_falls_back_to_catalog_dir() {
    let s = Settings { path: None, lint: None, excluded: vec![] };
    let resolved = resolve_path(&s);
    let default = default_catalog_dir();
    assert_eq!(resolved, default);
    assert!(resolved.to_string_lossy().contains(".ctlgr-cli"));
    assert!(resolved.to_string_lossy().ends_with("catalog"));
}

// ── expand_path ────────────────────────────────────────────────────────────

#[test]
fn expand_path_empty_nonexistent_dir_returns_empty() {
    let s = Settings { path: Some("/nonexistent/xyz/abc/definitely/not/real".into()), lint: None, excluded: vec![] };
    let files = expand_path(&s).unwrap();
    assert!(files.is_empty());
}

#[test]
fn expand_path_finds_html_files() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("a.html"), "<h1>hi</h1>").unwrap();
    let s = Settings { path: Some(tmp.path().to_string_lossy().into_owned()), lint: None, excluded: vec![] };
    let files = expand_path(&s).unwrap();
    assert!(files.iter().any(|f| f.ends_with("a.html")));
}

#[test]
fn expand_path_finds_md_files() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("readme.md"), "# hi").unwrap();
    let s = Settings { path: Some(tmp.path().to_string_lossy().into_owned()), lint: None, excluded: vec![] };
    let files = expand_path(&s).unwrap();
    assert!(files.iter().any(|f| f.ends_with("readme.md")));
}

#[test]
fn expand_path_ignores_other_extensions() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("script.js"), "").unwrap();
    std::fs::write(tmp.path().join("page.html"), "").unwrap();
    let s = Settings { path: Some(tmp.path().to_string_lossy().into_owned()), lint: None, excluded: vec![] };
    let mut files = expand_path(&s).unwrap();
    files.retain(|f| !f.ends_with(".js"));
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("page.html"));
}

#[test]
fn expand_path_recurses_into_subdirs() {
    let tmp = TempDir::new().unwrap();
    let deep = tmp.path().join("a").join("b");
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(deep.join("deep.html"), "").unwrap();
    let s = Settings { path: Some(tmp.path().to_string_lossy().into_owned()), lint: None, excluded: vec![] };
    let files = expand_path(&s).unwrap();
    assert!(files.iter().any(|f| f.ends_with("deep.html")));
}

// ── excluded ───────────────────────────────────────────────────────────────

#[test]
fn settings_excluded_defaults_empty() {
    let s: Settings = serde_json::from_str("{}").unwrap();
    assert!(s.excluded.is_empty());
}

#[test]
fn settings_excluded_roundtrip() {
    let json = r#"{"path":"/c","excluded":["AGENTS\\.md","drafts/"]}"#;
    let s: Settings = serde_json::from_str(json).unwrap();
    assert_eq!(s.excluded, vec!["AGENTS\\.md", "drafts/"]);
}

#[test]
fn settings_excluded_omitted_when_empty_on_serialize() {
    let s = Settings { path: None, lint: None, excluded: vec![] };
    let json = serde_json::to_string(&s).unwrap();
    assert!(!json.contains("excluded"));
}

#[test]
fn expand_path_excludes_matched_files() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("keep.html"), "").unwrap();
    std::fs::write(tmp.path().join("AGENTS.html"), "").unwrap();
    let s = Settings {
        path: Some(tmp.path().to_string_lossy().into_owned()),
        lint: None,
        excluded: vec!["AGENTS\\.html".into()],
    };
    let files = expand_path(&s).unwrap();
    assert!(files.iter().any(|f| f.ends_with("keep.html")));
    assert!(!files.iter().any(|f| f.ends_with("AGENTS.html")));
}

#[test]
fn expand_path_exclusion_matches_full_path() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("keep.html"), "").unwrap();
    std::fs::write(tmp.path().join("draft.html"), "").unwrap();
    let s = Settings {
        path: Some(tmp.path().to_string_lossy().into_owned()),
        lint: None,
        excluded: vec!["draft\\.html$".into()],
    };
    let files = expand_path(&s).unwrap();
    assert!(files.iter().any(|f| f.ends_with("keep.html")));
    assert!(!files.iter().any(|f| f.ends_with("draft.html")));
}

#[test]
fn expand_path_invalid_regex_is_skipped() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("page.html"), "").unwrap();
    let s = Settings {
        path: Some(tmp.path().to_string_lossy().into_owned()),
        lint: None,
        excluded: vec!["[invalid".into()],
    };
    let files = expand_path(&s).unwrap();
    assert!(files.iter().any(|f| f.ends_with("page.html")));
}

// ── excluded merging across config levels ─────────────────────────────────

#[test]
fn load_merges_excluded_from_parent_configs() {
    let _guard = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    // parent dir: .ctlgr with one excluded pattern
    let parent_cfg = tmp.path().join(".ctlgr");
    write_to(
        &Settings { path: None, lint: None, excluded: vec!["parent-pattern".into()] },
        &parent_cfg,
    )
    .unwrap();
    // child dir: .ctlgr with a different excluded pattern
    let child = tmp.path().join("child");
    std::fs::create_dir(&child).unwrap();
    let child_cfg = child.join(".ctlgr");
    write_to(
        &Settings { path: None, lint: None, excluded: vec!["child-pattern".into()] },
        &child_cfg,
    )
    .unwrap();
    std::env::set_current_dir(&child).unwrap();
    let loaded = load().unwrap();
    assert!(loaded.excluded.contains(&"child-pattern".to_string()));
    assert!(loaded.excluded.contains(&"parent-pattern".to_string()));
}

#[test]
fn load_deduplicates_excluded_patterns() {
    let _guard = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let parent_cfg = tmp.path().join(".ctlgr");
    write_to(
        &Settings { path: None, lint: None, excluded: vec!["same-pattern".into()] },
        &parent_cfg,
    )
    .unwrap();
    let child = tmp.path().join("child");
    std::fs::create_dir(&child).unwrap();
    write_to(
        &Settings { path: None, lint: None, excluded: vec!["same-pattern".into()] },
        &child.join(".ctlgr"),
    )
    .unwrap();
    std::env::set_current_dir(&child).unwrap();
    let loaded = load().unwrap();
    assert_eq!(loaded.excluded.iter().filter(|p| *p == "same-pattern").count(), 1);
}

// ── LintConfig ─────────────────────────────────────────────────────────────

#[test]
fn lint_config_default_enables_all_rules() {
    let cfg = LintConfig::default();
    assert!(cfg.is_enabled("no-style-blocks"));
    assert!(cfg.is_enabled("no-inline-styles"));
    assert!(cfg.is_enabled("prefer-html"));
}

#[test]
fn lint_config_is_enabled_returns_false_for_unknown_rule() {
    let cfg = LintConfig::default();
    assert!(!cfg.is_enabled("unknown-rule"));
}

#[test]
fn lint_config_disabled_rule_not_enabled() {
    let cfg = LintConfig { rules: vec!["no-style-blocks".into()] };
    assert!(cfg.is_enabled("no-style-blocks"));
    assert!(!cfg.is_enabled("no-inline-styles"));
    assert!(!cfg.is_enabled("prefer-html"));
}

#[test]
fn settings_lint_field_absent_by_default() {
    let s: Settings = serde_json::from_str(r#"{}"#).unwrap();
    assert!(s.lint.is_none());
}

// ── ensure_lint_defaults ───────────────────────────────────────────────────
//
// These tests mutate process-global CWD, so they must not run concurrently.
static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn ensure_lint_defaults_writes_rules_to_existing_config() {
    let _guard = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let config = tmp.path().join(".ctlgr");
    write_to(&Settings { path: None, lint: None, excluded: vec![] }, &config).unwrap();
    std::env::set_current_dir(&tmp).unwrap();
    ensure_lint_defaults();
    let loaded = load_from(&config).unwrap();
    assert!(loaded.lint.is_some());
    let rules = loaded.lint.unwrap().rules;
    assert!(rules.contains(&"no-style-blocks".to_string()));
    assert!(rules.contains(&"no-inline-styles".to_string()));
    assert!(rules.contains(&"prefer-html".to_string()));
}

#[test]
fn ensure_lint_defaults_is_idempotent() {
    let _guard = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let config = tmp.path().join(".ctlgr");
    let custom = LintConfig { rules: vec!["no-style-blocks".into()] };
    write_to(&Settings { path: None, lint: Some(custom), excluded: vec![] }, &config).unwrap();
    std::env::set_current_dir(&tmp).unwrap();
    ensure_lint_defaults();
    let loaded = load_from(&config).unwrap();
    assert_eq!(loaded.lint.unwrap().rules, vec!["no-style-blocks"]);
}

#[test]
fn ensure_lint_defaults_does_nothing_when_no_config_file() {
    let _guard = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let isolated = tmp.path().join("isolated");
    std::fs::create_dir(&isolated).unwrap();
    std::env::set_current_dir(&isolated).unwrap();
    ensure_lint_defaults();
    assert!(std::fs::read_dir(&isolated).unwrap().next().is_none());
}

// ── migrate_legacy_config ──────────────────────────────────────────────────

#[test]
fn migrate_ctlgr_json_creates_ctlgr() {
    let _guard = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join(".ctlgr.json"),
        r#"{"paths":["/catalog/docs"]}"#,
    )
    .unwrap();
    std::env::set_current_dir(&tmp).unwrap();
    migrate_legacy_config();
    let new_config = tmp.path().join(".ctlgr");
    assert!(new_config.exists());
    assert!(!tmp.path().join(".ctlgr.json").exists(), "legacy file should be removed");
    let loaded = load_from(&new_config).unwrap();
    assert_eq!(loaded.path, Some("/catalog/docs".into()));
}

#[test]
fn migrate_both_legacy_files_independently() {
    let _guard = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join(".ctlgr.json"), r#"{"paths":["/committed"]}"#).unwrap();
    std::fs::write(tmp.path().join(".ctlgr.local.json"), r#"{"paths":["/local"]}"#).unwrap();
    std::env::set_current_dir(&tmp).unwrap();
    migrate_legacy_config();
    // both legacy files removed
    assert!(!tmp.path().join(".ctlgr.json").exists());
    assert!(!tmp.path().join(".ctlgr.local.json").exists());
    // each migrated to its new name
    let committed = load_from(&tmp.path().join(".ctlgr")).unwrap();
    assert_eq!(committed.path, Some("/committed".into()));
    let local = load_from(&tmp.path().join(".ctlgr.local")).unwrap();
    assert_eq!(local.path, Some("/local".into()));
}

#[test]
fn migrate_preserves_lint_config() {
    let _guard = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join(".ctlgr.json"),
        r#"{"paths":["/docs"],"lint":{"rules":["no-style-blocks"]}}"#,
    )
    .unwrap();
    std::env::set_current_dir(&tmp).unwrap();
    migrate_legacy_config();
    let loaded = load_from(&tmp.path().join(".ctlgr")).unwrap();
    assert_eq!(loaded.path, Some("/docs".into()));
    assert_eq!(loaded.lint.unwrap().rules, vec!["no-style-blocks"]);
}

#[test]
fn migrate_takes_first_path_from_array() {
    let _guard = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join(".ctlgr.json"),
        r#"{"paths":["/first","/second","/third"]}"#,
    )
    .unwrap();
    std::env::set_current_dir(&tmp).unwrap();
    migrate_legacy_config();
    let loaded = load_from(&tmp.path().join(".ctlgr")).unwrap();
    assert_eq!(loaded.path, Some("/first".into()));
}

#[test]
fn migrate_is_idempotent_when_new_files_exist() {
    let _guard = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join(".ctlgr.json"), r#"{"paths":["/old"]}"#).unwrap();
    std::fs::write(tmp.path().join(".ctlgr.local.json"), r#"{"paths":["/old-local"]}"#).unwrap();
    std::fs::write(tmp.path().join(".ctlgr"), r#"{"path":"/current"}"#).unwrap();
    std::fs::write(tmp.path().join(".ctlgr.local"), r#"{"path":"/current-local"}"#).unwrap();
    std::env::set_current_dir(&tmp).unwrap();
    migrate_legacy_config();
    // existing new files must not be overwritten
    assert_eq!(load_from(&tmp.path().join(".ctlgr")).unwrap().path, Some("/current".into()));
    assert_eq!(
        load_from(&tmp.path().join(".ctlgr.local")).unwrap().path,
        Some("/current-local".into())
    );
}

#[test]
fn migrate_does_nothing_when_no_legacy_files() {
    let _guard = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let isolated = tmp.path().join("isolated");
    std::fs::create_dir(&isolated).unwrap();
    std::env::set_current_dir(&isolated).unwrap();
    migrate_legacy_config();
    assert!(!isolated.join(".ctlgr").exists());
}

#[test]
fn migrate_empty_paths_array_produces_no_path() {
    let _guard = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join(".ctlgr.json"), r#"{"paths":[]}"#).unwrap();
    std::env::set_current_dir(&tmp).unwrap();
    migrate_legacy_config();
    let loaded = load_from(&tmp.path().join(".ctlgr")).unwrap();
    assert!(loaded.path.is_none());
}

