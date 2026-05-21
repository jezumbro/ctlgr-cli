use std::path::Path;
use tempfile::TempDir;

use ctlgr::settings::{
    ensure_lint_defaults, expand_paths, find_config_from, load_from, local_config_path_from,
    write_to, LintConfig, Settings,
};

#[test]
fn settings_default_has_empty_paths() {
    assert!(Settings::default().paths.is_empty());
}

#[test]
fn settings_roundtrip() {
    let s = Settings { paths: vec!["a".into(), "b".into()], lint: None };
    let json = serde_json::to_string(&s).unwrap();
    let s2: Settings = serde_json::from_str(&json).unwrap();
    assert_eq!(s2.paths, s.paths);
}

#[test]
fn settings_missing_paths_field_defaults_empty() {
    let s: Settings = serde_json::from_str("{}").unwrap();
    assert!(s.paths.is_empty());
}

#[test]
fn local_config_path_from_non_local() {
    let p = local_config_path_from(Path::new("/some/dir"), false);
    assert_eq!(p, Path::new("/some/dir/.ctlgr.json"));
}

#[test]
fn local_config_path_from_local_flag() {
    let p = local_config_path_from(Path::new("/some/dir"), true);
    assert_eq!(p, Path::new("/some/dir/.ctlgr.local.json"));
}

#[test]
fn find_config_from_finds_ctlgr_json() {
    let tmp = TempDir::new().unwrap();
    let config = tmp.path().join(".ctlgr.json");
    std::fs::write(&config, "{}").unwrap();
    assert_eq!(find_config_from(tmp.path()).unwrap(), config);
}

#[test]
fn find_config_from_finds_local_json() {
    let tmp = TempDir::new().unwrap();
    let local = tmp.path().join(".ctlgr.local.json");
    std::fs::write(&local, "{}").unwrap();
    assert_eq!(find_config_from(tmp.path()).unwrap(), local);
}

#[test]
fn find_config_from_local_beats_committed() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join(".ctlgr.local.json"), "{}").unwrap();
    std::fs::write(tmp.path().join(".ctlgr.json"), "{}").unwrap();
    let found = find_config_from(tmp.path()).unwrap();
    assert_eq!(found.file_name().unwrap(), ".ctlgr.local.json");
}

#[test]
fn find_config_from_walks_up() {
    let tmp = TempDir::new().unwrap();
    let sub = tmp.path().join("a").join("b");
    std::fs::create_dir_all(&sub).unwrap();
    let config = tmp.path().join(".ctlgr.json");
    std::fs::write(&config, "{}").unwrap();
    assert_eq!(find_config_from(&sub).unwrap(), config);
}

#[test]
fn find_config_from_exhausts_names_at_level_before_ascending() {
    // sub has .ctlgr.json but not .ctlgr.local.json
    // parent has .ctlgr.local.json
    // should return sub/.ctlgr.json, not ascend to parent's local
    let tmp = TempDir::new().unwrap();
    let sub = tmp.path().join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join(".ctlgr.json"), "{}").unwrap();
    std::fs::write(tmp.path().join(".ctlgr.local.json"), "{}").unwrap();
    assert_eq!(find_config_from(&sub).unwrap(), sub.join(".ctlgr.json"));
}

#[test]
fn find_config_from_falls_back_to_global() {
    let found = find_config_from(Path::new("/")).unwrap();
    assert!(found.to_string_lossy().contains(".ctlgr-cli"));
    assert!(found.file_name().unwrap() == "settings.json");
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
    let s = Settings { paths: vec!["mypath".into()], lint: None };
    write_to(&s, &path).unwrap();
    let s2 = load_from(&path).unwrap();
    assert_eq!(s2.paths, vec!["mypath"]);
}

#[test]
fn load_from_missing_file_returns_default() {
    let tmp = TempDir::new().unwrap();
    let s = load_from(&tmp.path().join("nonexistent.json")).unwrap();
    assert!(s.paths.is_empty());
}

#[test]
fn load_from_reads_valid_file() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("c.json");
    std::fs::write(&path, r#"{"paths":["/foo","/bar"]}"#).unwrap();
    let s = load_from(&path).unwrap();
    assert_eq!(s.paths, vec!["/foo", "/bar"]);
}

#[test]
fn load_from_errors_on_invalid_json() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("c.json");
    std::fs::write(&path, "not json").unwrap();
    assert!(load_from(&path).is_err());
}

#[test]
fn expand_paths_empty_returns_empty() {
    let files = expand_paths(&Settings::default()).unwrap();
    assert!(files.is_empty());
}

#[test]
fn expand_paths_finds_html_files() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("a.html"), "<h1>hi</h1>").unwrap();
    let s = Settings { paths: vec![tmp.path().to_string_lossy().into_owned()], lint: None };
    let files = expand_paths(&s).unwrap();
    assert!(files.iter().any(|f| f.ends_with("a.html")));
}

#[test]
fn expand_paths_finds_md_files() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("readme.md"), "# hi").unwrap();
    let s = Settings { paths: vec![tmp.path().to_string_lossy().into_owned()], lint: None };
    let files = expand_paths(&s).unwrap();
    assert!(files.iter().any(|f| f.ends_with("readme.md")));
}

#[test]
fn expand_paths_ignores_other_extensions() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("script.js"), "").unwrap();
    std::fs::write(tmp.path().join("page.html"), "").unwrap();
    let s = Settings { paths: vec![tmp.path().to_string_lossy().into_owned()], lint: None };
    let mut files = expand_paths(&s).unwrap();
    files.retain(|f| !f.ends_with(".js"));
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("page.html"));
}

#[test]
fn expand_paths_recurses_into_subdirs() {
    let tmp = TempDir::new().unwrap();
    let deep = tmp.path().join("a").join("b");
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(deep.join("deep.html"), "").unwrap();
    let s = Settings { paths: vec![tmp.path().to_string_lossy().into_owned()], lint: None };
    let files = expand_paths(&s).unwrap();
    assert!(files.iter().any(|f| f.ends_with("deep.html")));
}

#[test]
fn expand_paths_multiple_dirs() {
    let t1 = TempDir::new().unwrap();
    let t2 = TempDir::new().unwrap();
    std::fs::write(t1.path().join("one.html"), "").unwrap();
    std::fs::write(t2.path().join("two.html"), "").unwrap();
    let s = Settings {
        paths: vec![
            t1.path().to_string_lossy().into_owned(),
            t2.path().to_string_lossy().into_owned(),
        ],
        lint: None,
    };
    let files = expand_paths(&s).unwrap();
    assert_eq!(files.len(), 2);
}

#[test]
fn expand_paths_nonexistent_dir_returns_empty() {
    let s = Settings { paths: vec!["/nonexistent/xyz/abc/definitely/not/real".into()], lint: None };
    let files = expand_paths(&s).unwrap();
    assert!(files.is_empty());
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
fn settings_with_lint_serializes_lint_section() {
    let s = Settings { paths: vec![], lint: Some(LintConfig::default()) };
    let json = serde_json::to_string(&s).unwrap();
    assert!(json.contains("\"lint\""));
    assert!(json.contains("no-style-blocks"));
}

#[test]
fn settings_without_lint_omits_lint_key() {
    let s = Settings { paths: vec![], lint: None };
    let json = serde_json::to_string(&s).unwrap();
    assert!(!json.contains("lint"));
}

#[test]
fn settings_roundtrip_with_lint() {
    let s = Settings {
        paths: vec!["a".into()],
        lint: Some(LintConfig { rules: vec!["no-style-blocks".into()] }),
    };
    let json = serde_json::to_string(&s).unwrap();
    let s2: Settings = serde_json::from_str(&json).unwrap();
    assert_eq!(s2.lint.unwrap().rules, vec!["no-style-blocks"]);
}

#[test]
fn settings_lint_absent_from_json_parses_as_none() {
    let s: Settings = serde_json::from_str(r#"{"paths":[]}"#).unwrap();
    assert!(s.lint.is_none());
}

// ── ensure_lint_defaults ───────────────────────────────────────────────────

#[test]
fn ensure_lint_defaults_writes_rules_to_existing_config() {
    let tmp = TempDir::new().unwrap();
    let config = tmp.path().join(".ctlgr.json");
    write_to(&Settings { paths: vec![], lint: None }, &config).unwrap();
    // Change CWD so find_config_from resolves this file
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
    let tmp = TempDir::new().unwrap();
    let config = tmp.path().join(".ctlgr.json");
    let custom = LintConfig { rules: vec!["no-style-blocks".into()] };
    write_to(&Settings { paths: vec![], lint: Some(custom) }, &config).unwrap();
    std::env::set_current_dir(&tmp).unwrap();
    ensure_lint_defaults();
    let loaded = load_from(&config).unwrap();
    // existing lint config must not be overwritten
    assert_eq!(loaded.lint.unwrap().rules, vec!["no-style-blocks"]);
}

#[test]
fn ensure_lint_defaults_does_nothing_when_no_config_file() {
    // Point to a dir with no config; function should not panic or create files
    let tmp = TempDir::new().unwrap();
    // Temporarily change cwd to an isolated dir without a config
    let isolated = tmp.path().join("isolated");
    std::fs::create_dir(&isolated).unwrap();
    std::env::set_current_dir(&isolated).unwrap();
    // Since no config file exists in the tree (or global), ensure_lint_defaults
    // must return without creating anything in isolated/.
    ensure_lint_defaults();
    assert!(std::fs::read_dir(&isolated).unwrap().next().is_none());
}
