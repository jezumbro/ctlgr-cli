use scraper::{Html, Selector};

use ctlgr::search::{
    attr_filter_matches, build_match, build_selector, find_enclosing_context, print_md,
    print_plain, search_sources, text_filter_matches, Match, SearchArgs,
};

fn default_args() -> SearchArgs {
    SearchArgs {
        query: None,
        file: vec![],
        tag: None,
        attrs: vec![],
        text: None,
        json: None,
        jq: None,
        md: false,
        limit: 30,
    }
}

fn src(path: &str, html: &str) -> (String, String) {
    (path.to_string(), html.to_string())
}

// build_selector

#[test]
fn build_selector_uses_query() {
    let args = SearchArgs { query: Some("nav a".into()), ..default_args() };
    assert_eq!(build_selector(&args), "nav a");
}

#[test]
fn build_selector_uses_tag_when_no_query() {
    let args = SearchArgs { tag: Some("h2".into()), ..default_args() };
    assert_eq!(build_selector(&args), "h2");
}

#[test]
fn build_selector_query_beats_tag() {
    let args = SearchArgs { query: Some("p".into()), tag: Some("a".into()), ..default_args() };
    assert_eq!(build_selector(&args), "p");
}

#[test]
fn build_selector_defaults_to_wildcard() {
    assert_eq!(build_selector(&default_args()), "*");
}

// attr_filter_matches

#[test]
fn attr_filter_no_filters_always_true() {
    let doc = Html::parse_document("<a href='/x'>link</a>");
    let sel = Selector::parse("a").unwrap();
    let el = doc.select(&sel).next().unwrap();
    assert!(attr_filter_matches(&el, &[]));
}

#[test]
fn attr_filter_presence_passes() {
    let doc = Html::parse_document("<a href='/x'>link</a>");
    let sel = Selector::parse("a").unwrap();
    let el = doc.select(&sel).next().unwrap();
    assert!(attr_filter_matches(&el, &["href".to_string()]));
}

#[test]
fn attr_filter_presence_fails_when_absent() {
    let doc = Html::parse_document("<a>link</a>");
    let sel = Selector::parse("a").unwrap();
    let el = doc.select(&sel).next().unwrap();
    assert!(!attr_filter_matches(&el, &["href".to_string()]));
}

#[test]
fn attr_filter_value_match_passes() {
    let doc = Html::parse_document("<a href='/foo'>link</a>");
    let sel = Selector::parse("a").unwrap();
    let el = doc.select(&sel).next().unwrap();
    assert!(attr_filter_matches(&el, &["href=/foo".to_string()]));
}

#[test]
fn attr_filter_value_mismatch_fails() {
    let doc = Html::parse_document("<a href='/bar'>link</a>");
    let sel = Selector::parse("a").unwrap();
    let el = doc.select(&sel).next().unwrap();
    assert!(!attr_filter_matches(&el, &["href=/foo".to_string()]));
}

#[test]
fn attr_filter_multiple_all_pass() {
    let doc = Html::parse_document(r#"<a href="/x" class="nav">link</a>"#);
    let sel = Selector::parse("a").unwrap();
    let el = doc.select(&sel).next().unwrap();
    assert!(attr_filter_matches(&el, &["href=/x".to_string(), "class=nav".to_string()]));
}

#[test]
fn attr_filter_multiple_one_fails() {
    let doc = Html::parse_document("<a href='/x'>link</a>");
    let sel = Selector::parse("a").unwrap();
    let el = doc.select(&sel).next().unwrap();
    assert!(!attr_filter_matches(&el, &["href=/x".to_string(), "class=nav".to_string()]));
}

// text_filter_matches

#[test]
fn text_filter_none_always_true() {
    let doc = Html::parse_document("<p>hello</p>");
    let sel = Selector::parse("p").unwrap();
    let el = doc.select(&sel).next().unwrap();
    assert!(text_filter_matches(&el, None));
}

#[test]
fn text_filter_substring_found() {
    let doc = Html::parse_document("<p>Hello World</p>");
    let sel = Selector::parse("p").unwrap();
    let el = doc.select(&sel).next().unwrap();
    assert!(text_filter_matches(&el, Some("World")));
}

#[test]
fn text_filter_case_insensitive() {
    let doc = Html::parse_document("<p>UPPER</p>");
    let sel = Selector::parse("p").unwrap();
    let el = doc.select(&sel).next().unwrap();
    assert!(text_filter_matches(&el, Some("upper")));
}

#[test]
fn text_filter_not_found() {
    let doc = Html::parse_document("<p>hello</p>");
    let sel = Selector::parse("p").unwrap();
    let el = doc.select(&sel).next().unwrap();
    assert!(!text_filter_matches(&el, Some("xyz")));
}

// find_enclosing_context

#[test]
fn find_enclosing_context_returns_article() {
    let doc = Html::parse_document(r#"<html><body><article id="a"><h3>title</h3></article></body></html>"#);
    let sel = Selector::parse("h3").unwrap();
    let el = doc.select(&sel).next().unwrap();
    let enc = find_enclosing_context(el).unwrap();
    assert_eq!(enc.value().name(), "article");
}

#[test]
fn find_enclosing_context_returns_section() {
    let doc = Html::parse_document("<html><body><section><p>text</p></section></body></html>");
    let sel = Selector::parse("p").unwrap();
    let el = doc.select(&sel).next().unwrap();
    let enc = find_enclosing_context(el).unwrap();
    assert_eq!(enc.value().name(), "section");
}

#[test]
fn find_enclosing_context_falls_back_to_immediate_parent() {
    let doc = Html::parse_document("<html><body><div><p>text</p></div></body></html>");
    let sel = Selector::parse("p").unwrap();
    let el = doc.select(&sel).next().unwrap();
    let enc = find_enclosing_context(el).unwrap();
    assert_eq!(enc.value().name(), "div");
}

#[test]
fn find_enclosing_context_none_when_no_parent() {
    let doc = Html::parse_document("<html>text</html>");
    let sel = Selector::parse("html").unwrap();
    let el = doc.select(&sel).next().unwrap();
    assert!(find_enclosing_context(el).is_none());
}

// search_sources

#[test]
fn search_sources_returns_matches() {
    let sources = vec![src("t.html", "<html><body><a href='/x'>link</a></body></html>")];
    let sel = Selector::parse("a").unwrap();
    let results = search_sources(&sources, &sel, &[], None, 100, None);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].tag.as_deref(), Some("a"));
}

#[test]
fn search_sources_respects_limit() {
    let sources = vec![src("t.html", "<html><body><a>1</a><a>2</a><a>3</a></body></html>")];
    let sel = Selector::parse("a").unwrap();
    let results = search_sources(&sources, &sel, &[], None, 2, None);
    assert_eq!(results.len(), 2);
}

#[test]
fn search_sources_text_filter_emits_enclosing_and_match() {
    let sources = vec![src("t.html", "<html><body><a>foo</a><a>bar</a></body></html>")];
    let sel = Selector::parse("a").unwrap();
    let results = search_sources(&sources, &sel, &[], Some("foo"), 100, None);
    assert_eq!(results.len(), 2);
    let tags: Vec<_> = results.iter().map(|r| r.tag.as_deref().unwrap()).collect();
    assert!(tags.contains(&"body"));
    assert!(tags.contains(&"a"));
}

#[test]
fn search_sources_text_walks_up_to_article_with_id() {
    let html = r#"<html><body><article id="init"><h3>config init</h3></article></body></html>"#;
    let sources = vec![src("t.html", html)];
    let sel = Selector::parse("h3").unwrap();
    let results = search_sources(&sources, &sel, &[], Some("config"), 100, None);
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].tag.as_deref(), Some("article"));
    assert_eq!(results[1].tag.as_deref(), Some("h3"));
}

#[test]
fn search_sources_text_walks_up_to_section() {
    let html = "<html><body><section><p>config desc</p></section></body></html>";
    let sources = vec![src("t.html", html)];
    let sel = Selector::parse("p").unwrap();
    let results = search_sources(&sources, &sel, &[], Some("config"), 100, None);
    assert_eq!(results[0].tag.as_deref(), Some("section"));
}

#[test]
fn search_sources_text_deduplicates_enclosing() {
    let html =
        r#"<html><body><section id="s"><p>config one</p><p>config two</p></section></body></html>"#;
    let sources = vec![src("t.html", html)];
    let sel = Selector::parse("p").unwrap();
    let results = search_sources(&sources, &sel, &[], Some("config"), 100, None);
    assert_eq!(results.len(), 3);
    assert_eq!(results.iter().filter(|r| r.tag.as_deref() == Some("section")).count(), 1);
}

#[test]
fn search_sources_applies_attr_filter() {
    let sources = vec![src(
        "t.html",
        r#"<html><body><a href="/x">one</a><a href="/y">two</a></body></html>"#,
    )];
    let sel = Selector::parse("a").unwrap();
    let results = search_sources(&sources, &sel, &["href=/x".to_string()], None, 100, None);
    assert_eq!(results.len(), 1);
}

#[test]
fn search_sources_multiple_sources() {
    let sources = vec![
        src("a.html", "<html><body><h1>first</h1></body></html>"),
        src("b.html", "<html><body><h1>second</h1></body></html>"),
    ];
    let sel = Selector::parse("h1").unwrap();
    let results = search_sources(&sources, &sel, &[], None, 100, None);
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].path.as_deref(), Some("a.html"));
    assert_eq!(results[1].path.as_deref(), Some("b.html"));
}

#[test]
fn search_sources_no_matches_returns_empty() {
    let sources = vec![src("t.html", "<html><body><p>text</p></body></html>")];
    let sel = Selector::parse("a").unwrap();
    let results = search_sources(&sources, &sel, &[], None, 100, None);
    assert!(results.is_empty());
}

#[test]
fn search_sources_limit_spans_multiple_sources() {
    let sources = vec![
        src("a.html", "<html><body><a>1</a><a>2</a><a>3</a></body></html>"),
        src("b.html", "<html><body><a>4</a><a>5</a><a>6</a></body></html>"),
    ];
    let sel = Selector::parse("a").unwrap();
    let results = search_sources(&sources, &sel, &[], None, 4, None);
    assert_eq!(results.len(), 4);
}

#[test]
fn search_sources_text_no_parent_for_root() {
    let sources = vec![src("t.html", "<html>config</html>")];
    let sel = Selector::parse("html").unwrap();
    let results = search_sources(&sources, &sel, &[], Some("config"), 100, None);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].tag.as_deref(), Some("html"));
}

// build_match

#[test]
fn build_match_all_fields_when_no_filter() {
    let doc = Html::parse_document(r#"<a href="/x">link</a>"#);
    let sel = Selector::parse("a").unwrap();
    let el = doc.select(&sel).next().unwrap();
    let m = build_match(&el, "file.html", None);
    assert_eq!(m.tag.as_deref(), Some("a"));
    assert_eq!(m.text.as_deref(), Some("link"));
    assert_eq!(m.path.as_deref(), Some("file.html"));
    assert!(m.attrs.as_ref().unwrap().contains_key("href"));
    assert!(m.html.is_some());
}

#[test]
fn build_match_field_filter_excludes_others() {
    let doc = Html::parse_document("<p>text</p>");
    let sel = Selector::parse("p").unwrap();
    let el = doc.select(&sel).next().unwrap();
    let m = build_match(&el, "f.html", Some(&["tag", "text"]));
    assert!(m.tag.is_some());
    assert!(m.text.is_some());
    assert!(m.attrs.is_none());
    assert!(m.html.is_none());
    assert!(m.path.is_none());
}

#[test]
fn build_match_text_is_trimmed() {
    let doc = Html::parse_document("<p>  spaces  </p>");
    let sel = Selector::parse("p").unwrap();
    let el = doc.select(&sel).next().unwrap();
    let m = build_match(&el, "f.html", None);
    assert_eq!(m.text.as_deref(), Some("spaces"));
}

#[test]
fn build_match_empty_element() {
    let doc = Html::parse_document("<div></div>");
    let sel = Selector::parse("div").unwrap();
    let el = doc.select(&sel).next().unwrap();
    let m = build_match(&el, "f.html", None);
    assert_eq!(m.tag.as_deref(), Some("div"));
    assert_eq!(m.text.as_deref(), Some(""));
}

// print_md

#[test]
fn print_md_converts_html() {
    let m = Match {
        tag: None,
        attrs: None,
        text: None,
        html: Some("<h3>Config Init</h3><p>Creates a config file.</p>".into()),
        path: None,
    };
    print_md(&m).unwrap();
}

#[test]
fn print_md_no_html_is_noop() {
    let m = Match { tag: None, attrs: None, text: None, html: None, path: None };
    print_md(&m).unwrap();
}

// print_plain

#[test]
fn print_plain_with_all_fields() {
    let m = Match {
        tag: Some("a".into()),
        attrs: Some([("href".into(), "/x".into())].into_iter().collect()),
        text: Some("link".into()),
        html: None,
        path: None,
    };
    print_plain(&m);
}

#[test]
fn print_plain_with_no_fields() {
    let m = Match { tag: None, attrs: None, text: None, html: None, path: None };
    print_plain(&m);
}
