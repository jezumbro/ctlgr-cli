use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn cmd() -> Command {
    Command::cargo_bin("ctlgr").unwrap()
}

// ── search ────────────────────────────────────────────────────────────────────

#[test]
fn search_explicit_file_finds_elements() {
    let tmp = TempDir::new().unwrap();
    let page = tmp.path().join("page.html");
    std::fs::write(&page, "<html><body><a href='/x'>link</a></body></html>").unwrap();
    cmd()
        .args(["search", "a", "--file"])
        .arg(&page)
        .assert()
        .success()
        .stdout(predicate::str::contains("<a"));
}

#[test]
fn search_text_filter_matches() {
    let tmp = TempDir::new().unwrap();
    let page = tmp.path().join("page.html");
    std::fs::write(&page, "<html><body><a>link text</a></body></html>").unwrap();
    cmd()
        .args(["search", "--file"])
        .arg(&page)
        .args(["--text", "link"])
        .assert()
        .success()
        // enclosing parent (<body>) + matched element (<a>) both appear
        .stdout(predicate::str::contains("link text"))
        .stdout(predicate::str::contains("<body"));
}

#[test]
fn search_text_filter_no_match_produces_empty_output() {
    let tmp = TempDir::new().unwrap();
    let page = tmp.path().join("page.html");
    std::fs::write(&page, "<html><body><a>link</a></body></html>").unwrap();
    cmd()
        .args(["search", "--file"])
        .arg(&page)
        .args(["--text", "xyz_no_match"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn search_limit_caps_results() {
    let tmp = TempDir::new().unwrap();
    let page = tmp.path().join("page.html");
    std::fs::write(&page, "<html><body><a>1</a><a>2</a><a>3</a><a>4</a><a>5</a></body></html>")
        .unwrap();
    let output = cmd()
        .args(["search", "a", "--file"])
        .arg(&page)
        .args(["-L", "2"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    // each result prints opening + closing tag = 2 lines per result
    assert_eq!(stdout.lines().count(), 4);
}

#[test]
fn search_json_output_contains_requested_fields_only() {
    let tmp = TempDir::new().unwrap();
    let page = tmp.path().join("page.html");
    std::fs::write(&page, "<html><body><a href='/x'>link</a></body></html>").unwrap();
    let output = cmd()
        .args(["search", "a", "--file"])
        .arg(&page)
        .args(["--json", "tag,text"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_array());
    let item = &parsed[0];
    assert!(item.get("tag").is_some());
    assert!(item.get("text").is_some());
    assert!(item.get("attrs").is_none());
    assert!(item.get("html").is_none());
}

#[test]
fn search_with_no_files_and_no_config_errors() {
    let tmp = TempDir::new().unwrap();
    // Write a config with no path to shadow any ancestor or global config
    std::fs::write(tmp.path().join(".ctlgr"), r#"{}"#).unwrap();
    cmd()
        .args(["search", "a"])
        .current_dir(&tmp)
        .assert()
        .failure()
        .stderr(predicate::str::contains("no files found"));
}

#[test]
fn search_with_configured_path_but_no_matching_files_errors() {
    let tmp = TempDir::new().unwrap();
    let empty = tmp.path().join("empty");
    std::fs::create_dir(&empty).unwrap();
    std::fs::write(
        tmp.path().join(".ctlgr"),
        serde_json::json!({ "path": empty.to_string_lossy() }).to_string(),
    )
    .unwrap();
    cmd()
        .args(["search", "a"])
        .current_dir(&tmp)
        .assert()
        .failure()
        .stderr(predicate::str::contains("no files found"));
}

#[test]
fn search_via_configured_path() {
    let tmp = TempDir::new().unwrap();
    let docs = tmp.path().join("docs");
    std::fs::create_dir(&docs).unwrap();
    std::fs::write(docs.join("page.html"), "<html><body><a>link</a></body></html>").unwrap();
    std::fs::write(
        tmp.path().join(".ctlgr"),
        serde_json::json!({ "path": docs.to_string_lossy() }).to_string(),
    )
    .unwrap();
    cmd()
        .args(["search", "a"])
        .current_dir(&tmp)
        .assert()
        .success()
        .stdout(predicate::str::contains("<a"));
}

#[test]
fn search_inherits_config_from_ancestor_directory() {
    let tmp = TempDir::new().unwrap();
    let docs = tmp.path().join("docs");
    let sub = tmp.path().join("src").join("components");
    std::fs::create_dir(&docs).unwrap();
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(docs.join("page.html"), "<html><body><a>link</a></body></html>").unwrap();
    std::fs::write(
        tmp.path().join(".ctlgr"),
        serde_json::json!({ "path": docs.to_string_lossy() }).to_string(),
    )
    .unwrap();
    cmd()
        .args(["search", "a"])
        .current_dir(&sub)
        .assert()
        .success()
        .stdout(predicate::str::contains("<a"));
}

#[test]
fn search_ctlgr_local_takes_priority_over_ctlgr() {
    let tmp = TempDir::new().unwrap();
    let committed_docs = tmp.path().join("committed_docs");
    let local_docs = tmp.path().join("local_docs");
    std::fs::create_dir(&committed_docs).unwrap();
    std::fs::create_dir(&local_docs).unwrap();
    std::fs::write(
        committed_docs.join("c.html"),
        "<html><body><p>committed</p></body></html>",
    )
    .unwrap();
    std::fs::write(
        local_docs.join("l.html"),
        "<html><body><p>local-only</p></body></html>",
    )
    .unwrap();
    std::fs::write(
        tmp.path().join(".ctlgr"),
        serde_json::json!({ "path": committed_docs.to_string_lossy() }).to_string(),
    )
    .unwrap();
    std::fs::write(
        tmp.path().join(".ctlgr.local"),
        serde_json::json!({ "path": local_docs.to_string_lossy() }).to_string(),
    )
    .unwrap();
    let output = cmd()
        .args(["search", "p", "--json", "text"])
        .current_dir(&tmp)
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("local-only"));
    assert!(!stdout.contains("committed"));
}

#[test]
fn search_invalid_selector_errors() {
    let tmp = TempDir::new().unwrap();
    let page = tmp.path().join("page.html");
    std::fs::write(&page, "<html></html>").unwrap();
    // ":not(" is an incomplete pseudo-class that scraper rejects
    cmd()
        .args(["search", ":not(", "--file"])
        .arg(&page)
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid selector"));
}

#[test]
fn search_attr_filter() {
    let tmp = TempDir::new().unwrap();
    let page = tmp.path().join("page.html");
    std::fs::write(
        &page,
        r#"<html><body><a href="/x">one</a><a href="/y">two</a></body></html>"#,
    )
    .unwrap();
    let output = cmd()
        .args(["search", "a", "--file"])
        .arg(&page)
        .args(["--attr", "href=/x"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    // opening + closing tag = 2 lines
    assert_eq!(stdout.lines().count(), 2);
    assert!(stdout.contains("one"));
}

// ── update ────────────────────────────────────────────────────────────────────

#[test]
fn update_already_up_to_date_exits_zero() {
    // The published release matches the binary version, so update reports
    // "already up to date" and exits 0.
    cmd()
        .args(["update"])
        .timeout(std::time::Duration::from_secs(10))
        .assert()
        .success()
        .stdout(predicate::str::contains("already up to date"));
}

// ── search --md ───────────────────────────────────────────────────────────────

#[test]
fn search_md_converts_html_to_markdown() {
    let tmp = TempDir::new().unwrap();
    let page = tmp.path().join("page.html");
    std::fs::write(
        &page,
        "<html><body><article id='init'><h3>Config Init</h3><p>Creates a config file.</p></article></body></html>",
    )
    .unwrap();
    cmd()
        .args(["search", "article", "--file"])
        .arg(&page)
        .args(["--md"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Config Init"))
        .stdout(predicate::str::contains("Creates a config file"));
}

#[test]
fn search_md_with_text_filter() {
    let tmp = TempDir::new().unwrap();
    let page = tmp.path().join("page.html");
    std::fs::write(
        &page,
        "<html><body><article id='a'><h3>config add</h3></article><article id='b'><h3>search</h3></article></body></html>",
    )
    .unwrap();
    let output = cmd()
        .args(["search", "h3", "--file"])
        .arg(&page)
        .args(["--md", "--text", "config"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("config add"));
    assert!(!stdout.contains("search"));
}

// ── config init ───────────────────────────────────────────────────────────────

#[test]
fn config_init_creates_ctlgr_with_path() {
    let tmp = TempDir::new().unwrap();
    let docs = tmp.path().join("docs");
    std::fs::create_dir(&docs).unwrap();
    cmd()
        .args(["config", "init"])
        .arg(&docs)
        .current_dir(&tmp)
        .assert()
        .success()
        .stdout(predicate::str::contains("created"));
    assert!(tmp.path().join(".ctlgr").exists());
    assert!(!tmp.path().join(".ctlgr.local").exists());
    let content = std::fs::read_to_string(tmp.path().join(".ctlgr")).unwrap();
    assert!(content.contains(docs.to_string_lossy().as_ref()));
}

#[test]
fn config_init_local_creates_ctlgr_local() {
    let tmp = TempDir::new().unwrap();
    let docs = tmp.path().join("docs");
    std::fs::create_dir(&docs).unwrap();
    cmd()
        .args(["config", "init", "--local"])
        .arg(&docs)
        .current_dir(&tmp)
        .assert()
        .success()
        .stdout(predicate::str::contains("created"));
    assert!(tmp.path().join(".ctlgr.local").exists());
    assert!(!tmp.path().join(".ctlgr").exists());
}

#[test]
fn config_init_nonexistent_path_errors() {
    let tmp = TempDir::new().unwrap();
    cmd()
        .args(["config", "init", "/nonexistent/path/xyz/abc"])
        .current_dir(&tmp)
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn config_init_file_not_directory_errors() {
    let tmp = TempDir::new().unwrap();
    let f = tmp.path().join("afile.txt");
    std::fs::write(&f, "").unwrap();
    cmd()
        .args(["config", "init"])
        .arg(&f)
        .current_dir(&tmp)
        .assert()
        .failure()
        .stderr(predicate::str::contains("not a directory"));
}

#[test]
fn config_init_twice_fails_with_already_exists() {
    let tmp = TempDir::new().unwrap();
    let docs = tmp.path().join("docs");
    std::fs::create_dir(&docs).unwrap();
    cmd().args(["config", "init"]).arg(&docs).current_dir(&tmp).assert().success();
    cmd()
        .args(["config", "init"])
        .arg(&docs)
        .current_dir(&tmp)
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

// ── config list ───────────────────────────────────────────────────────────────

#[test]
fn config_list_shows_registered_path() {
    let tmp = TempDir::new().unwrap();
    let docs = tmp.path().join("docs");
    std::fs::create_dir(&docs).unwrap();
    cmd().args(["config", "init"]).arg(&docs).current_dir(&tmp).assert().success();
    cmd()
        .args(["config", "list"])
        .current_dir(&tmp)
        .assert()
        .success()
        .stdout(predicate::str::contains(docs.to_string_lossy().as_ref()));
}

#[test]
fn config_list_shows_default_when_no_config() {
    let tmp = TempDir::new().unwrap();
    // Write a config with no path to shadow ancestors
    std::fs::write(tmp.path().join(".ctlgr"), "{}").unwrap();
    cmd()
        .args(["config", "list"])
        .current_dir(&tmp)
        .assert()
        .success()
        .stdout(predicate::str::contains(".ctlgr-cli"))
        .stdout(predicate::str::contains("catalog"));
}

// ── lint ──────────────────────────────────────────────────────────────────

#[test]
fn lint_clean_file_exits_zero() {
    let tmp = TempDir::new().unwrap();
    let page = tmp.path().join("page.html");
    std::fs::write(&page, "<article><h2>Title</h2><p>Content</p></article>").unwrap();
    cmd()
        .args(["lint", "--file"])
        .arg(&page)
        .assert()
        .success();
}

#[test]
fn lint_style_block_exits_nonzero() {
    let tmp = TempDir::new().unwrap();
    let page = tmp.path().join("page.html");
    std::fs::write(&page, "<article><style>h2{color:red}</style><h2>X</h2></article>")
        .unwrap();
    cmd()
        .args(["lint", "--file"])
        .arg(&page)
        .assert()
        .failure()
        .stdout(predicate::str::contains("no-style-blocks"));
}

#[test]
fn lint_inline_style_exits_nonzero() {
    let tmp = TempDir::new().unwrap();
    let page = tmp.path().join("page.html");
    std::fs::write(&page, "<h2 style=\"color:red\">Title</h2>").unwrap();
    cmd()
        .args(["lint", "--file"])
        .arg(&page)
        .assert()
        .failure()
        .stdout(predicate::str::contains("no-inline-styles"));
}

#[test]
fn lint_check_output_includes_file_line_rule() {
    let tmp = TempDir::new().unwrap();
    let page = tmp.path().join("page.html");
    std::fs::write(&page, "<h2 style=\"color:red\">X</h2>").unwrap();
    let output = cmd()
        .args(["lint", "--file"])
        .arg(&page)
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("no-inline-styles"));
    assert!(stdout.contains(':'));
}

#[test]
fn lint_write_fixes_violations_and_exits_zero() {
    let tmp = TempDir::new().unwrap();
    let page = tmp.path().join("page.html");
    std::fs::write(&page, "<article><style>h2{}</style><h2>X</h2></article>").unwrap();
    cmd()
        .args(["lint", "--write", "--file"])
        .arg(&page)
        .assert()
        .success()
        .stdout(predicate::str::contains("fixed"));
    let content = std::fs::read_to_string(&page).unwrap();
    assert!(!content.contains("<style>"));
}

#[test]
fn lint_write_clean_file_exits_zero_silently() {
    let tmp = TempDir::new().unwrap();
    let page = tmp.path().join("page.html");
    let clean = "<article><h2>Title</h2></article>";
    std::fs::write(&page, clean).unwrap();
    cmd()
        .args(["lint", "--write", "--file"])
        .arg(&page)
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn lint_with_no_file_and_no_config_errors() {
    let tmp = TempDir::new().unwrap();
    // Write a config with no path to shadow any ancestor or global config
    std::fs::write(tmp.path().join(".ctlgr"), r#"{}"#).unwrap();
    cmd()
        .args(["lint"])
        .current_dir(&tmp)
        .assert()
        .failure()
        .stderr(predicate::str::contains("no files found"));
}

#[test]
fn lint_via_configured_path() {
    let tmp = TempDir::new().unwrap();
    let docs = tmp.path().join("docs");
    std::fs::create_dir(&docs).unwrap();
    std::fs::write(docs.join("a.html"), "<article><h2>Clean</h2></article>").unwrap();
    cmd().args(["config", "init"]).arg(&docs).current_dir(&tmp).assert().success();
    cmd().args(["lint"]).current_dir(&tmp).assert().success();
}

#[test]
fn lint_with_configured_path_but_no_html_files_errors() {
    let tmp = TempDir::new().unwrap();
    let docs = tmp.path().join("docs");
    std::fs::create_dir(&docs).unwrap();
    cmd().args(["config", "init"]).arg(&docs).current_dir(&tmp).assert().success();
    cmd()
        .args(["lint"])
        .current_dir(&tmp)
        .assert()
        .failure()
        .stderr(predicate::str::contains("no files found"));
}

#[test]
fn lint_skips_md_files_silently() {
    let tmp = TempDir::new().unwrap();
    let page = tmp.path().join("page.md");
    std::fs::write(&page, "# Title\n\nContent.").unwrap();
    // .md files are skipped — lint exits 0 with no output
    cmd()
        .args(["lint", "--file"])
        .arg(&page)
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
    assert!(page.exists(), ".md must remain untouched");
}

#[test]
fn any_command_writes_default_lint_rules_to_existing_config() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join(".ctlgr"), r#"{}"#).unwrap();
    // Run a non-lint command — defaults should still be seeded
    cmd().args(["config", "list"]).current_dir(&tmp).assert().success();
    let config = std::fs::read_to_string(tmp.path().join(".ctlgr")).unwrap();
    assert!(config.contains("\"lint\""), "lint section should be written by any command");
    assert!(config.contains("no-style-blocks"));
    assert!(!config.contains("prefer-html"), "prefer-html must not appear in default rules");
}

#[test]
fn lint_writes_default_rules_to_existing_config() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join(".ctlgr"), r#"{}"#).unwrap();
    let page = tmp.path().join("page.html");
    std::fs::write(&page, "<article><h2>Clean</h2></article>").unwrap();
    cmd()
        .args(["lint", "--file"])
        .arg(&page)
        .current_dir(&tmp)
        .assert()
        .success();
    let config = std::fs::read_to_string(tmp.path().join(".ctlgr")).unwrap();
    assert!(config.contains("\"lint\""));
    assert!(config.contains("no-style-blocks"));
}

#[test]
fn lint_respects_disabled_rule_in_config() {
    let tmp = TempDir::new().unwrap();
    // Only no-style-blocks enabled; no-inline-styles disabled
    std::fs::write(
        tmp.path().join(".ctlgr"),
        r#"{"lint":{"rules":["no-style-blocks"]}}"#,
    )
    .unwrap();
    let page = tmp.path().join("page.html");
    std::fs::write(&page, "<h2 style=\"color:red\">Title</h2>").unwrap();
    cmd()
        .args(["lint", "--file"])
        .arg(&page)
        .current_dir(&tmp)
        .assert()
        .success(); // exits 0 because no-inline-styles is disabled
}
