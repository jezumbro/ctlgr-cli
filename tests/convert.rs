use assert_cmd::Command;
use ctlgr::convert::{
    convert_md_to_html, md_html_path, md_to_html, md_to_html_fragment, merge_html,
};
use ctlgr::lint::check_html;
use tempfile::TempDir;

// ── md_to_html ─────────────────────────────────────────────────────────────

#[test]
fn md_to_html_produces_html_document() {
    let html = md_to_html("# Hello\n\nSome text.");
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("<html"));
    assert!(html.contains("<body>"));
    assert!(html.contains("</body>"));
}

#[test]
fn md_to_html_uses_first_h1_as_title() {
    let html = md_to_html("# My Page\n\nContent.");
    assert!(html.contains("<title>My Page</title>"));
}

#[test]
fn md_to_html_falls_back_to_document_title_when_no_h1() {
    let html = md_to_html("Just some text with no heading.");
    assert!(html.contains("<title>Document</title>"));
}

#[test]
fn md_to_html_renders_markdown_content() {
    let html = md_to_html("# Title\n\nA paragraph.\n\n- item one\n- item two");
    assert!(html.contains("<h1>"));
    assert!(html.contains("<p>"));
    assert!(html.contains("<ul>"));
    assert!(html.contains("item one"));
}

#[test]
fn md_to_html_generates_no_style_violations() {
    let html = md_to_html("# Title\n\nParagraph with **bold** and *italic*.");
    let violations = check_html(&html, "f.html");
    assert!(
        violations.is_empty(),
        "generated HTML has style violations: {:?}",
        violations.iter().map(|v| &v.rule).collect::<Vec<_>>()
    );
}

// ── md_html_path ───────────────────────────────────────────────────────────

#[test]
fn md_html_path_replaces_md_extension() {
    assert_eq!(md_html_path("docs/readme.md"), "docs/readme.html");
}

#[test]
fn md_html_path_appends_html_when_no_md_suffix() {
    assert_eq!(md_html_path("README"), "README.html");
}

// ── merge_html ─────────────────────────────────────────────────────────────

#[test]
fn merge_html_inserts_before_closing_body() {
    let existing = "<html><body><p>old</p></body></html>";
    let fragment = "<p>new</p>";
    let merged = merge_html(existing, fragment);
    assert!(merged.contains("<p>old</p>"));
    assert!(merged.contains("<p>new</p>"));
    let new_pos = merged.find("<p>new</p>").unwrap();
    let close_pos = merged.find("</body>").unwrap();
    assert!(new_pos < close_pos);
}

#[test]
fn merge_html_appends_when_no_body_tag() {
    let existing = "<p>old</p>";
    let fragment = "<p>new</p>";
    let merged = merge_html(existing, fragment);
    assert!(merged.contains("old"));
    assert!(merged.contains("new"));
}

#[test]
fn merge_html_uses_last_body_tag() {
    let existing = "<body><p>one</p></body><body><p>two</p></body>";
    let fragment = "<p>appended</p>";
    let merged = merge_html(existing, fragment);
    assert!(merged.contains("appended"));
}

// ── md_to_html_fragment ────────────────────────────────────────────────────

#[test]
fn md_to_html_fragment_renders_without_document_wrapper() {
    let frag = md_to_html_fragment("# Title\n\nParagraph.");
    assert!(!frag.contains("<!DOCTYPE"));
    assert!(!frag.contains("<html"));
    assert!(frag.contains("<h1>"));
    assert!(frag.contains("<p>"));
}

// ── convert_md_to_html ─────────────────────────────────────────────────────

#[test]
fn convert_md_to_html_creates_html_file() {
    let tmp = TempDir::new().unwrap();
    let md = tmp.path().join("page.md");
    std::fs::write(&md, "# Title\n\nContent.").unwrap();
    convert_md_to_html(&md.to_string_lossy(), false).unwrap();
    let html = tmp.path().join("page.html");
    assert!(html.exists());
    let content = std::fs::read_to_string(&html).unwrap();
    assert!(content.contains("<title>Title</title>"));
}

#[test]
fn convert_md_to_html_removes_md_file() {
    let tmp = TempDir::new().unwrap();
    let md = tmp.path().join("page.md");
    std::fs::write(&md, "# Hello").unwrap();
    convert_md_to_html(&md.to_string_lossy(), false).unwrap();
    assert!(!md.exists());
}

#[test]
fn convert_md_to_html_html_has_no_style_violations() {
    let tmp = TempDir::new().unwrap();
    let md = tmp.path().join("clean.md");
    std::fs::write(&md, "# Clean\n\nNo styles here.\n\n- list item").unwrap();
    convert_md_to_html(&md.to_string_lossy(), false).unwrap();
    let html_path = tmp.path().join("clean.html");
    let source = std::fs::read_to_string(&html_path).unwrap();
    let violations = check_html(&source, "clean.html");
    assert!(violations.is_empty());
}

#[test]
fn convert_md_to_html_merges_when_html_already_exists() {
    let tmp = TempDir::new().unwrap();
    let html = tmp.path().join("page.html");
    let md = tmp.path().join("page.md");
    std::fs::write(
        &html,
        "<html><body><article id=\"existing\"><h2>Existing</h2></article></body></html>",
    )
    .unwrap();
    std::fs::write(&md, "# New Section\n\nNew content.").unwrap();
    convert_md_to_html(&md.to_string_lossy(), false).unwrap();
    assert!(!md.exists(), ".md should be removed after merge");
    let content = std::fs::read_to_string(&html).unwrap();
    assert!(content.contains("Existing"), "existing content preserved");
    assert!(content.contains("New content"), "new content added");
}

#[test]
fn convert_md_to_html_merged_content_before_closing_body() {
    let tmp = TempDir::new().unwrap();
    let html = tmp.path().join("page.html");
    let md = tmp.path().join("page.md");
    std::fs::write(&html, "<html><body><p>old</p></body></html>").unwrap();
    std::fs::write(&md, "## New").unwrap();
    convert_md_to_html(&md.to_string_lossy(), false).unwrap();
    let content = std::fs::read_to_string(&html).unwrap();
    let new_pos = content.find("New").unwrap();
    let close_pos = content.find("</body>").unwrap();
    assert!(new_pos < close_pos, "new content should be before </body>");
}

#[test]
fn convert_prints_merged_into_when_html_exists() {
    let tmp = TempDir::new().unwrap();
    let html = tmp.path().join("page.html");
    let md = tmp.path().join("page.md");
    std::fs::write(&html, "<html><body><p>old</p></body></html>").unwrap();
    std::fs::write(&md, "## New").unwrap();
    // Run via CLI to capture stdout
    let md_str = md.to_string_lossy().to_string();
    let html_str = html.to_string_lossy().to_string();
    let mut cmd = Command::cargo_bin("ctlgr").unwrap();
    let output = cmd.args(["convert", "--file", &md_str]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("merged into"), "expected 'merged into' in: {stdout}");
    assert!(stdout.contains(&html_str) || stdout.contains("page.html"));
}

// ── dry-run ────────────────────────────────────────────────────────────────

#[test]
fn dry_run_does_not_create_html() {
    let tmp = TempDir::new().unwrap();
    let md = tmp.path().join("page.md");
    std::fs::write(&md, "# Title\n\nContent.").unwrap();
    convert_md_to_html(&md.to_string_lossy(), true).unwrap();
    let html = tmp.path().join("page.html");
    assert!(!html.exists(), "html should not be created in dry-run");
}

#[test]
fn dry_run_does_not_remove_md() {
    let tmp = TempDir::new().unwrap();
    let md = tmp.path().join("page.md");
    std::fs::write(&md, "# Title\n\nContent.").unwrap();
    convert_md_to_html(&md.to_string_lossy(), true).unwrap();
    assert!(md.exists(), ".md should still exist in dry-run");
}

#[test]
fn dry_run_prints_would_convert() {
    let tmp = TempDir::new().unwrap();
    let md = tmp.path().join("page.md");
    std::fs::write(&md, "# Title\n\nContent.").unwrap();
    let md_str = md.to_string_lossy().to_string();
    let mut cmd = Command::cargo_bin("ctlgr").unwrap();
    let output = cmd.args(["convert", "--dry-run", "--file", &md_str]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("converted to"), "expected status line in dry-run: {stdout}");
}

#[test]
fn dry_run_via_cli_flag() {
    let tmp = TempDir::new().unwrap();
    let md = tmp.path().join("page.md");
    std::fs::write(&md, "# Hello").unwrap();
    let md_str = md.to_string_lossy().to_string();
    let mut cmd = Command::cargo_bin("ctlgr").unwrap();
    cmd.args(["convert", "--dry-run", "--file", &md_str]).assert().success();
    assert!(md.exists(), ".md must remain after dry-run");
    assert!(!tmp.path().join("page.html").exists(), ".html must not be created");
}

// ── CLI integration ────────────────────────────────────────────────────────

#[test]
fn convert_via_file_flag() {
    let tmp = TempDir::new().unwrap();
    let md = tmp.path().join("page.md");
    std::fs::write(&md, "# Hello\n\nWorld.").unwrap();
    let md_str = md.to_string_lossy().to_string();
    let mut cmd = Command::cargo_bin("ctlgr").unwrap();
    cmd.args(["convert", "--file", &md_str]).assert().success();
    assert!(tmp.path().join("page.html").exists());
    assert!(!md.exists());
}

#[test]
fn convert_via_dir_flag() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("a.md"), "# A").unwrap();
    std::fs::write(tmp.path().join("b.md"), "# B").unwrap();
    let dir_str = tmp.path().to_string_lossy().to_string();
    let mut cmd = Command::cargo_bin("ctlgr").unwrap();
    cmd.args(["convert", "--dir", &dir_str]).assert().success();
    assert!(tmp.path().join("a.html").exists());
    assert!(tmp.path().join("b.html").exists());
    assert!(!tmp.path().join("a.md").exists());
    assert!(!tmp.path().join("b.md").exists());
}
