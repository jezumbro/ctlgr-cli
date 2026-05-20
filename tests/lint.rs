use ctlgr::lint::{
    check_html, convert_md_to_html, fix_html, md_html_path, md_to_html, md_to_html_fragment,
    merge_html,
};
use tempfile::TempDir;

// ── check_html ─────────────────────────────────────────────────────────────

#[test]
fn clean_html_has_no_violations() {
    let html = "<article><h2>Title</h2><p>Content</p></article>";
    let violations = check_html(html, "test.html");
    assert!(violations.is_empty());
}

#[test]
fn detects_style_block() {
    let html = "<article><style>h2 { color: red }</style><h2>Title</h2></article>";
    let violations = check_html(html, "test.html");
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].rule, "no-style-blocks");
    assert_eq!(violations[0].file, "test.html");
    assert!(violations[0].snippet.contains("<style>"));
}

#[test]
fn detects_inline_style() {
    let html = "<h2 style=\"font-weight: bold\">Title</h2>";
    let violations = check_html(html, "test.html");
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].rule, "no-inline-styles");
    assert!(violations[0].snippet.contains("style="));
}

#[test]
fn detects_multiple_violations() {
    let html = "<div>\
        <style>body{}</style>\
        <h2 style=\"color:red\">A</h2>\
        <p style=\"color:blue\">B</p>\
        </div>";
    let violations = check_html(html, "f.html");
    assert_eq!(violations.len(), 3);
    let rules: Vec<_> = violations.iter().map(|v| v.rule).collect();
    assert!(rules.contains(&"no-style-blocks"));
    assert_eq!(rules.iter().filter(|&&r| r == "no-inline-styles").count(), 2);
}

#[test]
fn line_numbers_are_reported() {
    let html = "<article>\n<style>h2{}</style>\n<h2>X</h2>\n</article>";
    let violations = check_html(html, "f.html");
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 2);
}

#[test]
fn inline_style_line_number() {
    let html = "<div>\n<h2 style=\"color:red\">X</h2>\n</div>";
    let violations = check_html(html, "f.html");
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 2);
}

#[test]
fn snippet_truncated_at_80_chars() {
    let long_css = "a".repeat(200);
    let html = format!("<style>{long_css}</style>");
    let violations = check_html(&html, "f.html");
    assert_eq!(violations.len(), 1);
    // truncated snippet ends with ellipsis
    assert!(violations[0].snippet.contains('\u{2026}'));
}

// ── fix_html ───────────────────────────────────────────────────────────────

#[test]
fn fix_removes_style_block() {
    let html = "<article><style>h2 { color: red }</style><h2>Title</h2></article>";
    let (fixed, violations) = fix_html(html, "f.html");
    assert_eq!(violations.len(), 1);
    assert!(!fixed.contains("<style>"));
    assert!(fixed.contains("<h2>Title</h2>"));
}

#[test]
fn fix_removes_inline_style() {
    let html = "<h2 style=\"font-weight: bold\">Title</h2>";
    let (fixed, violations) = fix_html(html, "f.html");
    assert_eq!(violations.len(), 1);
    assert!(!fixed.contains("style="));
    assert!(fixed.contains("<h2>Title</h2>"));
}

#[test]
fn fix_preserves_other_attributes() {
    let html = "<a href=\"/page\" style=\"color:red\" class=\"nav\">link</a>";
    let (fixed, _) = fix_html(html, "f.html");
    assert!(fixed.contains("href=\"/page\""));
    assert!(fixed.contains("class=\"nav\""));
    assert!(!fixed.contains("style="));
}

#[test]
fn fix_handles_multiple_violations() {
    let html = "<div><style>body{}</style><p style=\"color:blue\">text</p></div>";
    let (fixed, violations) = fix_html(html, "f.html");
    assert_eq!(violations.len(), 2);
    assert!(!fixed.contains("<style>"));
    assert!(!fixed.contains("style="));
    assert!(fixed.contains("<p>text</p>"));
}

#[test]
fn fix_clean_file_unchanged() {
    let html = "<article id=\"intro\"><h2>Hello</h2><p>World</p></article>";
    let (fixed, violations) = fix_html(html, "f.html");
    assert!(violations.is_empty());
    assert_eq!(fixed, html);
}

#[test]
fn fix_removes_single_quoted_inline_style() {
    let html = "<h2 style='color:red'>Title</h2>";
    let (fixed, violations) = fix_html(html, "f.html");
    assert_eq!(violations.len(), 1);
    assert!(!fixed.contains("style="));
    assert!(fixed.contains("<h2>Title</h2>"));
}

#[test]
fn check_detects_single_quoted_inline_style() {
    let html = "<h2 style='color:red'>Title</h2>";
    let violations = check_html(html, "f.html");
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].rule, "no-inline-styles");
}

#[test]
fn fix_style_block_trailing_newline_consumed() {
    let html = "<div><style>h2{}</style>\n<p>text</p></div>";
    let (fixed, _) = fix_html(html, "f.html");
    assert!(!fixed.contains("<style>"));
    // trailing newline after </style> is consumed; <p> directly follows <div>
    assert!(!fixed.contains("\n<p>"));
}

#[test]
fn style_custom_element_not_flagged() {
    // <style-custom> is not a <style> element and must not be treated as one
    let html = "<div><style-custom>foo</style-custom><p>text</p></div>";
    let violations = check_html(html, "f.html");
    assert!(violations.is_empty());
}

#[test]
fn fix_style_custom_element_unchanged() {
    let html = "<div><style-custom>foo</style-custom><p>text</p></div>";
    let (fixed, violations) = fix_html(html, "f.html");
    assert!(violations.is_empty());
    assert_eq!(fixed, html);
}

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
    assert!(violations.is_empty(), "generated HTML has style violations: {violations:?}", violations = violations.iter().map(|v| &v.rule).collect::<Vec<_>>());
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

// ── convert_md_to_html ─────────────────────────────────────────────────────

#[test]
fn convert_md_to_html_creates_html_file() {
    let tmp = TempDir::new().unwrap();
    let md = tmp.path().join("page.md");
    std::fs::write(&md, "# Title\n\nContent.").unwrap();
    let md_str = md.to_string_lossy().to_string();
    convert_md_to_html(&md_str).unwrap();
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
    let md_str = md.to_string_lossy().to_string();
    convert_md_to_html(&md_str).unwrap();
    assert!(!md.exists());
}

#[test]
fn convert_md_to_html_html_has_no_style_violations() {
    let tmp = TempDir::new().unwrap();
    let md = tmp.path().join("clean.md");
    std::fs::write(&md, "# Clean\n\nNo styles here.\n\n- list item").unwrap();
    let md_str = md.to_string_lossy().to_string();
    convert_md_to_html(&md_str).unwrap();
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
    let md_str = md.to_string_lossy().to_string();
    convert_md_to_html(&md_str).unwrap();
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
    let md_str = md.to_string_lossy().to_string();
    convert_md_to_html(&md_str).unwrap();
    let content = std::fs::read_to_string(&html).unwrap();
    // new content must appear before </body>
    let new_pos = content.find("New").unwrap();
    let close_pos = content.find("</body>").unwrap();
    assert!(new_pos < close_pos, "new content should be before </body>");
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
    // rfind ensures we target the final </body> in malformed HTML
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
