use anyhow::{Context, Result};
use pulldown_cmark::{html as cmark_html, Options, Parser};
use scraper::{Html, Selector};

use crate::settings;

#[derive(clap::Parser)]
pub struct LintArgs {
    /// Files to lint (repeatable; defaults to paths in settings)
    #[arg(short, long, value_name = "file")]
    pub file: Vec<String>,

    /// Auto-fix violations where possible (default: check mode)
    #[arg(long)]
    pub write: bool,
}

pub struct Violation {
    pub file: String,
    pub line: usize,
    pub rule: &'static str,
    pub snippet: String,
}

pub fn run(args: &LintArgs) -> Result<()> {
    let files = resolve_files(args)?;
    let mut total = 0usize;

    for path in &files {
        if path.ends_with(".md") {
            if args.write {
                convert_md_to_html(path)?;
                total += 1;
            } else {
                let html_path = md_html_path(path);
                println!("{path}:1: [prefer-html] {html_path}");
                total += 1;
            }
        } else {
            let source = std::fs::read_to_string(path)
                .with_context(|| format!("reading {path}"))?;

            if args.write {
                let (fixed_source, fixed) = fix_html(&source, path);
                for v in &fixed {
                    println!("{}: fixed [{}] {}", v.file, v.rule, v.snippet);
                }
                if !fixed.is_empty() {
                    std::fs::write(path, &fixed_source)
                        .with_context(|| format!("writing {path}"))?;
                }
                total += fixed.len();
            } else {
                let violations = check_html(&source, path);
                for v in &violations {
                    println!("{}:{}: [{}] {}", v.file, v.line, v.rule, v.snippet);
                }
                total += violations.len();
            }
        }
    }

    if total > 0 && !args.write {
        std::process::exit(1);
    }

    Ok(())
}

/// Convert a Markdown file to HTML and remove the original.
///
/// If a corresponding `.html` file already exists, the rendered Markdown body
/// is merged into it (inserted before `</body>`) rather than overwriting it.
pub fn convert_md_to_html(md_path: &str) -> Result<()> {
    let source = std::fs::read_to_string(md_path)
        .with_context(|| format!("reading {md_path}"))?;
    let html_path = md_html_path(md_path);

    if std::path::Path::new(&html_path).exists() {
        let existing = std::fs::read_to_string(&html_path)
            .with_context(|| format!("reading {html_path}"))?;
        let fragment = md_to_html_fragment(&source);
        let merged = merge_html(&existing, &fragment);
        std::fs::write(&html_path, merged)
            .with_context(|| format!("writing {html_path}"))?;
        println!("{md_path}: merged into {html_path}");
    } else {
        let html = md_to_html(&source);
        std::fs::write(&html_path, html)
            .with_context(|| format!("writing {html_path}"))?;
        println!("{md_path}: converted to {html_path}");
    }

    std::fs::remove_file(md_path)
        .with_context(|| format!("removing {md_path}"))?;
    Ok(())
}

/// Render Markdown into a self-contained HTML catalog document.
/// Title taken from the first H1 heading, falling back to "Document".
pub fn md_to_html(content: &str) -> String {
    let title = content
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .unwrap_or("Document");
    let body = md_to_html_fragment(content);
    format!(
        "<!DOCTYPE html>\n\
         <html lang=\"en\">\n\
         <head>\n\
         \x20 <meta charset=\"UTF-8\">\n\
         \x20 <title>{title}</title>\n\
         </head>\n\
         <body>\n\
         {body}\
         </body>\n\
         </html>\n"
    )
}

/// Render Markdown into an HTML fragment (body content only, no document wrapper).
pub fn md_to_html_fragment(content: &str) -> String {
    let mut out = String::new();
    cmark_html::push_html(&mut out, Parser::new_ext(content, Options::all()));
    out
}

/// Insert `fragment` before `</body>` in `existing_html`. If no `</body>` is
/// found, the fragment is appended at the end.
pub fn merge_html(existing_html: &str, fragment: &str) -> String {
    match existing_html.rfind("</body>") {
        Some(pos) => format!("{}{}{}", &existing_html[..pos], fragment, &existing_html[pos..]),
        None => format!("{}\n{}", existing_html.trim_end(), fragment),
    }
}

/// Check HTML source for lint violations. Returns violations in document order.
pub fn check_html(source: &str, path: &str) -> Vec<Violation> {
    let doc = Html::parse_document(source);
    let mut violations = Vec::new();

    let style_sel = Selector::parse("style").unwrap();
    let mut offset = 0usize;
    for el in doc.select(&style_sel) {
        let pos = find_from(source, "<style", offset);
        let line = pos.map(|p| line_at(source, p)).unwrap_or(1);
        if let Some(p) = pos {
            offset = p + 1;
        }
        violations.push(Violation {
            file: path.to_string(),
            line,
            rule: "no-style-blocks",
            snippet: truncate(&el.html(), 80),
        });
    }

    let inline_sel = Selector::parse("[style]").unwrap();
    let mut offset = 0usize;
    for el in doc.select(&inline_sel) {
        let style_val = el.value().attr("style").unwrap_or("");
        let search_dq = format!(" style=\"{style_val}\"");
        let search_sq = format!(" style='{style_val}'");
        let pos = find_from(source, &search_dq, offset)
            .or_else(|| find_from(source, &search_sq, offset));
        let line = pos.map(|p| line_at(source, p)).unwrap_or(1);
        if let Some(p) = pos {
            offset = p + 1;
        }
        violations.push(Violation {
            file: path.to_string(),
            line,
            rule: "no-inline-styles",
            snippet: format!("style=\"{style_val}\""),
        });
    }

    violations
}

/// Remove style blocks and inline style attributes. Returns the fixed source
/// and the list of violations that were removed (from the original source).
pub fn fix_html(source: &str, path: &str) -> (String, Vec<Violation>) {
    let violations = check_html(source, path);
    (apply_fixes(source), violations)
}

fn apply_fixes(source: &str) -> String {
    let mut s = source.to_string();

    loop {
        let Some(start) = s.find("<style") else { break };
        let after = start + "<style".len();
        match s[after..].chars().next() {
            Some(' ' | '\t' | '\n' | '>') => {}
            _ => break,
        }
        let Some(rel_end) = s[start..].find("</style>") else { break };
        let mut end = start + rel_end + "</style>".len();
        if s.get(end..end + 1) == Some("\n") {
            end += 1;
        }
        s.replace_range(start..end, "");
    }

    loop {
        let Some(pos) = s.find(" style=\"") else { break };
        let val_start = pos + " style=\"".len();
        let Some(rel_end) = s[val_start..].find('"') else { break };
        s.replace_range(pos..val_start + rel_end + 1, "");
    }

    loop {
        let Some(pos) = s.find(" style='") else { break };
        let val_start = pos + " style='".len();
        let Some(rel_end) = s[val_start..].find('\'') else { break };
        s.replace_range(pos..val_start + rel_end + 1, "");
    }

    s
}

pub fn md_html_path(md_path: &str) -> String {
    md_path.strip_suffix(".md").unwrap_or(md_path).to_string() + ".html"
}

fn find_from(source: &str, needle: &str, from: usize) -> Option<usize> {
    source[from..].find(needle).map(|p| p + from)
}

fn line_at(source: &str, pos: usize) -> usize {
    source[..pos].chars().filter(|&c| c == '\n').count() + 1
}

fn truncate(s: &str, max_chars: usize) -> String {
    let mut chars = s.chars();
    let mut result: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        result.push('\u{2026}');
    }
    result
}

fn resolve_files(args: &LintArgs) -> Result<Vec<String>> {
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
