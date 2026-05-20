use anyhow::{Context, Result};
use scraper::{Html, Selector};
use serde::Serialize;
use std::collections::HashMap;

use crate::settings;

#[derive(clap::Parser)]
pub struct SearchArgs {
    /// CSS selector query
    pub query: Option<String>,

    /// HTML file(s) to search (repeatable; defaults to paths in settings)
    #[arg(short, long, value_name = "file")]
    pub file: Vec<String>,

    /// Filter by tag name (e.g. --tag a)
    #[arg(short, long, value_name = "tag")]
    pub tag: Option<String>,

    /// Filter by attribute name or name=value, repeatable (e.g. --attr href --attr class=nav)
    #[arg(short, long = "attr", value_name = "name[=value]")]
    pub attrs: Vec<String>,

    /// Filter by text content (case-insensitive substring match)
    #[arg(long, value_name = "pattern")]
    pub text: Option<String>,

    /// Output JSON with the specified fields (tag,attrs,text,html,path)
    #[arg(long, value_name = "fields")]
    pub json: Option<String>,

    /// Filter JSON output using a jq expression
    #[arg(short = 'q', long, value_name = "expression")]
    pub jq: Option<String>,

    /// Output results as Markdown
    #[arg(long)]
    pub md: bool,

    /// Maximum number of results
    #[arg(short = 'L', long, default_value_t = 30)]
    pub limit: usize,
}

#[derive(Serialize, Debug)]
pub struct Match {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attrs: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

pub fn run(args: &SearchArgs) -> Result<()> {
    let files = resolve_files(args)?;

    let selector_str = build_selector(args);
    let selector = Selector::parse(&selector_str)
        .map_err(|e| anyhow::anyhow!("invalid selector '{}': {}", selector_str, e))?;

    let json_fields: Option<Vec<&str>> = args.json.as_deref().map(|f| f.split(',').collect());

    // --md needs the html field; --json uses the requested fields; plain needs all
    const MD_FIELDS: &[&str] = &["html"];
    let fields_for_match: Option<&[&str]> = if args.md {
        Some(MD_FIELDS)
    } else {
        json_fields.as_deref()
    };

    let sources: Vec<(String, String)> = files
        .iter()
        .map(|path| {
            let content = std::fs::read_to_string(path)
                .with_context(|| format!("reading {path}"))?;
            Ok((path.clone(), content))
        })
        .collect::<Result<_>>()?;

    let results = search_sources(
        &sources,
        &selector,
        &args.attrs,
        args.text.as_deref(),
        args.limit,
        fields_for_match,
    );

    if args.md {
        for m in &results {
            print_md(m)?;
        }
    } else if json_fields.is_some() {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        for m in &results {
            print_plain(m);
        }
    }

    Ok(())
}

pub fn search_sources(
    sources: &[(String, String)],
    selector: &Selector,
    attrs: &[String],
    text: Option<&str>,
    limit: usize,
    json_fields: Option<&[&str]>,
) -> Vec<Match> {
    use std::collections::HashSet;
    let mut count = 0;
    let mut results = Vec::new();
    'outer: for (path, html) in sources {
        let doc = Html::parse_document(html);
        let mut emitted: HashSet<ego_tree::NodeId> = HashSet::new();
        for el in doc.select(selector) {
            if count >= limit {
                break 'outer;
            }
            if !attr_filter_matches(&el, attrs) {
                continue;
            }
            if !text_filter_matches(&el, text) {
                continue;
            }
            // When text filtering, emit the nearest semantic enclosing block first
            // (article/section/element-with-id), then the matched element itself.
            if text.is_some() {
                if let Some(enclosing) = find_enclosing_context(el) {
                    if emitted.insert(enclosing.id()) && count < limit {
                        results.push(build_match(&enclosing, path, json_fields));
                        count += 1;
                    }
                }
            }
            if emitted.insert(el.id()) {
                if count < limit {
                    results.push(build_match(&el, path, json_fields));
                    count += 1;
                }
            }
        }
    }
    results
}

fn resolve_files(args: &SearchArgs) -> Result<Vec<String>> {
    if !args.file.is_empty() {
        return Ok(args.file.clone());
    }

    let cfg = settings::load()?;
    if cfg.paths.is_empty() {
        anyhow::bail!(
            "no files specified and no paths configured\n\
             hint: run `ctlgr config add <path>` to register a search path"
        );
    }

    let files = settings::expand_paths(&cfg)?;
    if files.is_empty() {
        anyhow::bail!(
            "no files found matching configured paths\n\
             hint: run `ctlgr config list` to review registered paths"
        );
    }

    Ok(files)
}

pub fn build_selector(args: &SearchArgs) -> String {
    match (&args.query, &args.tag) {
        (Some(q), _) => q.clone(),
        (None, Some(t)) => t.clone(),
        (None, None) => "*".to_string(),
    }
}

pub fn attr_filter_matches(el: &scraper::ElementRef, filters: &[String]) -> bool {
    for filter in filters {
        if let Some((name, value)) = filter.split_once('=') {
            match el.value().attr(name) {
                Some(v) if v == value => {}
                _ => return false,
            }
        } else if el.value().attr(filter.as_str()).is_none() {
            return false;
        }
    }
    true
}

/// Walk up from `el` looking for the nearest ancestor that is a semantic block
/// element (`article`, `section`, `main`, `nav`, `header`, `footer`, `aside`) or
/// any element that has an `id` attribute. Falls back to the immediate parent if
/// no such ancestor exists.
pub fn find_enclosing_context(el: scraper::ElementRef<'_>) -> Option<scraper::ElementRef<'_>> {
    const SEMANTIC: &[&str] =
        &["article", "section", "main", "nav", "header", "footer", "aside"];
    let immediate = el.parent().and_then(scraper::ElementRef::wrap)?;
    let mut node = immediate;
    loop {
        if SEMANTIC.contains(&node.value().name()) || node.value().attr("id").is_some() {
            return Some(node);
        }
        match node.parent().and_then(scraper::ElementRef::wrap) {
            Some(parent) => node = parent,
            None => return Some(immediate),
        }
    }
}

pub fn text_filter_matches(el: &scraper::ElementRef, pattern: Option<&str>) -> bool {
    match pattern {
        None => true,
        Some(p) => {
            let text: String = el.text().collect::<Vec<_>>().join(" ");
            text.to_lowercase().contains(&p.to_lowercase())
        }
    }
}

pub fn build_match(el: &scraper::ElementRef, path: &str, fields: Option<&[&str]>) -> Match {
    let want = |f: &str| fields.map_or(true, |fs| fs.contains(&f));

    Match {
        tag: want("tag").then(|| el.value().name().to_string()),
        attrs: want("attrs").then(|| {
            el.value()
                .attrs()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        }),
        text: want("text").then(|| el.text().collect::<Vec<_>>().join(" ").trim().to_string()),
        html: want("html").then(|| el.html()),
        path: want("path").then(|| path.to_string()),
    }
}

pub fn print_md(m: &Match) -> Result<()> {
    if let Some(html) = &m.html {
        let md = htmd::convert(html).context("converting HTML to Markdown")?;
        println!("{}", md.trim());
        println!();
    }
    Ok(())
}

pub fn print_plain(m: &Match) {
    let tag = m.tag.as_deref().unwrap_or("?");
    let attrs: String = m
        .attrs
        .as_ref()
        .map(|a| {
            let mut pairs: Vec<_> = a.iter().collect();
            pairs.sort_by_key(|(k, _)| k.as_str());
            pairs.iter().map(|(k, v)| format!(" {k}=\"{v}\"")).collect()
        })
        .unwrap_or_default();
    let text = m.text.as_deref().unwrap_or("");
    println!("<{tag}{attrs}> {text}");
    println!("</{tag}>");
}
