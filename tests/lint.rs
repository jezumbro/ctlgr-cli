use ctlgr::lint::{check_html, fix_html};

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
