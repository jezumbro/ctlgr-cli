use anyhow::{Context, Result};
use pulldown_cmark::{html as cmark_html, Options, Parser};

use crate::settings;

#[derive(clap::Parser)]
#[command(
    about = "Convert .md files to .html catalog documents",
    after_help = "MODES:\n  (default)  convert discovered .md files, remove originals\n  --dry-run  print what would happen without making changes\n\nSOURCE:\n  --file     explicit list of .md files\n  --dir      directory to walk for *.md files\n  (neither)  configured catalog directory"
)]
pub struct ConvertArgs {
    /// .md file(s) to convert (repeatable)
    #[arg(short, long, value_name = "file")]
    pub file: Vec<String>,

    /// Directory to walk for *.md files
    #[arg(long, value_name = "dir")]
    pub dir: Option<String>,

    /// Print what would happen without writing or deleting files
    #[arg(long)]
    pub dry_run: bool,
}

pub fn run(args: &ConvertArgs) -> Result<()> {
    let files = resolve_files(args)?;
    let mut failed = false;

    for path in &files {
        if let Err(e) = convert_md_to_html(path, args.dry_run) {
            eprintln!("error: {e}");
            failed = true;
        }
    }

    if failed {
        std::process::exit(1);
    }
    Ok(())
}

pub fn convert_md_to_html(md_path: &str, dry_run: bool) -> Result<()> {
    let source = std::fs::read_to_string(md_path)
        .with_context(|| format!("reading {md_path}"))?;
    let html_path = md_html_path(md_path);

    if std::path::Path::new(&html_path).exists() {
        let existing = std::fs::read_to_string(&html_path)
            .with_context(|| format!("reading {html_path}"))?;
        let fragment = md_to_html_fragment(&source);
        let merged = merge_html(&existing, &fragment);
        if !dry_run {
            std::fs::write(&html_path, merged)
                .with_context(|| format!("writing {html_path}"))?;
            std::fs::remove_file(md_path)
                .with_context(|| format!("removing {md_path}"))?;
        }
        println!("{md_path}: merged into {html_path}");
    } else {
        let html = md_to_html(&source);
        if !dry_run {
            std::fs::write(&html_path, html)
                .with_context(|| format!("writing {html_path}"))?;
            std::fs::remove_file(md_path)
                .with_context(|| format!("removing {md_path}"))?;
        }
        println!("{md_path}: converted to {html_path}");
    }

    Ok(())
}

pub fn md_html_path(md_path: &str) -> String {
    md_path.strip_suffix(".md").unwrap_or(md_path).to_string() + ".html"
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

fn resolve_files(args: &ConvertArgs) -> Result<Vec<String>> {
    if !args.file.is_empty() {
        return Ok(args.file.clone());
    }

    if let Some(dir) = &args.dir {
        let pattern = format!("{dir}/**/*.md");
        let paths: Vec<String> = glob::glob(&pattern)
            .with_context(|| format!("invalid glob pattern: {pattern}"))?
            .filter_map(|e| e.ok())
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        return Ok(paths);
    }

    let cfg = settings::load()?;
    let all_files = settings::expand_path(&cfg)?;
    let md_files: Vec<String> = all_files.into_iter().filter(|f| f.ends_with(".md")).collect();

    if md_files.is_empty() {
        eprintln!("warning: no .md files found in catalog directory");
    }

    Ok(md_files)
}
