use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(default)]
    pub paths: Vec<String>,
}

fn global_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".ctlgr-cli").join("settings.json"))
}

pub fn find_config_from(start: &Path) -> Result<PathBuf> {
    let mut dir = start;
    loop {
        for name in &[".ctlgr.local.json", ".ctlgr.json"] {
            let candidate = dir.join(name);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }
    }
    global_config_path()
}

pub fn local_config_path_from(cwd: &Path, local: bool) -> PathBuf {
    let name = if local { ".ctlgr.local.json" } else { ".ctlgr.json" };
    cwd.join(name)
}

pub fn load_from(path: &Path) -> Result<Settings> {
    if !path.exists() {
        return Ok(Settings::default());
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("parsing {}", path.display()))
}

pub fn local_config_path(local: bool) -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("could not determine current directory")?;
    Ok(local_config_path_from(&cwd, local))
}

pub fn load() -> Result<Settings> {
    let cwd = std::env::current_dir().context("could not determine current directory")?;
    load_from(&find_config_from(&cwd)?)
}

pub fn save(settings: &Settings) -> Result<()> {
    let cwd = std::env::current_dir().context("could not determine current directory")?;
    write_to(settings, &find_config_from(&cwd)?)
}

pub fn write_to(settings: &Settings, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(settings)?;
    std::fs::write(path, content)
        .with_context(|| format!("writing {}", path.display()))
}

pub fn expand_paths(settings: &Settings) -> Result<Vec<String>> {
    let mut files = Vec::new();
    for dir in &settings.paths {
        for ext in &["html", "md"] {
            let pattern = format!("{dir}/**/*.{ext}");
            let entries = glob::glob(&pattern)
                .with_context(|| format!("invalid glob pattern: {pattern}"))?;
            for entry in entries {
                let path = entry.with_context(|| format!("expanding {pattern}"))?;
                files.push(path.to_string_lossy().into_owned());
            }
        }
    }
    Ok(files)
}
