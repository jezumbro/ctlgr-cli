use anyhow::{Context, Result};
use scraper::{Html, Selector};

use crate::settings::{self, LintConfig};

pub use settings::LintConfig as Config;

#[derive(clap::Parser)]
#[command(
    about = "Lint catalog HTML files for style violations",
    after_help = "MODES:\n  check (default)  report violations, exit non-zero if any found\n  --write          fix in place: strip style blocks and inline styles\n\nRULES (all enabled by default; configure via lint.rules in settings):\n  no-style-blocks   <style> blocks are not allowed\n  no-inline-styles  style=\"...\" attributes are not allowed"
)]
pub struct LintArgs {
    /// Files to lint (repeatable; defaults to all registered paths)
    #[arg(short, long, value_name = "file")]
    pub file: Vec<String>,

    /// Fix violations in place: strip style blocks and inline style attributes
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
    let lint_cfg = settings::load()?.lint.unwrap_or_default();
    let mut total = 0usize;

    for path in &files {
        if path.ends_with(".md") {
            continue;
        }

        let source = std::fs::read_to_string(path)
            .with_context(|| format!("reading {path}"))?;

        if args.write {
            let (fixed_source, fixed) = fix_html(&source, path, &lint_cfg);
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
            let active: Vec<_> =
                violations.into_iter().filter(|v| lint_cfg.is_enabled(v.rule)).collect();
            for v in &active {
                println!("{}:{}: [{}] {}", v.file, v.line, v.rule, v.snippet);
            }
            total += active.len();
        }
    }

    if total > 0 && !args.write {
        std::process::exit(1);
    }

    Ok(())
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

/// Remove violations for enabled rules. Returns the fixed source and the list
/// of violations that were addressed.
pub fn fix_html(source: &str, path: &str, cfg: &LintConfig) -> (String, Vec<Violation>) {
    let all_violations = check_html(source, path);
    let violations: Vec<_> =
        all_violations.into_iter().filter(|v| cfg.is_enabled(v.rule)).collect();
    (apply_fixes(source, cfg), violations)
}

fn apply_fixes(source: &str, cfg: &LintConfig) -> String {
    let mut s = source.to_string();

    if cfg.is_enabled("no-style-blocks") {
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
    }

    if cfg.is_enabled("no-inline-styles") {
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
    }

    s
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
    let files = settings::expand_path(&cfg)?;
    if files.is_empty() {
        anyhow::bail!(
            "no files found\n\
             hint: run `ctlgr config init <path>` to register a catalog directory"
        );
    }

    Ok(files)
}
