use anyhow::{Context, Result};
use semver::Version;
use std::path::{Path, PathBuf};
use std::time::Duration;

const CURRENT: &str = env!("CARGO_PKG_VERSION");
const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
const GITHUB_REPO: &str = "jezumbro/ctlgr-cli";
const CHECK_INTERVAL: Duration = Duration::from_secs(60 * 60 * 24);

pub fn current_version() -> Version {
    Version::parse(CURRENT).expect("CARGO_PKG_VERSION is always valid semver")
}

pub fn global_check_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".ctlgr-cli").join(".update-check"))
}

// ── pure logic (fully testable) ───────────────────────────────────────────────

pub fn update_available(current: &Version, latest: &Version) -> bool {
    latest > current
}

pub fn cooldown_expired(check_path: &Path, interval: Duration) -> bool {
    if !check_path.exists() {
        return true;
    }
    std::fs::metadata(check_path)
        .and_then(|m| m.modified())
        .map(|modified| {
            std::time::SystemTime::now()
                .duration_since(modified)
                .unwrap_or(Duration::MAX)
                >= interval
        })
        .unwrap_or(true)
}

pub fn touch_check_file(check_path: &Path) {
    if let Some(parent) = check_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(check_path, b"");
}

pub fn current_target() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-musl"),
        ("linux", "aarch64") => Ok("aarch64-unknown-linux-musl"),
        (os, arch) => anyhow::bail!("unsupported platform: {os}/{arch}"),
    }
}

/// Returns a notice string if a newer version is available, `None` otherwise.
/// Uses `fetch` for the version lookup so tests can inject a mock.
pub fn check_update_message(
    check_path: &Path,
    interval: Duration,
    fetch: impl Fn() -> Result<Version>,
) -> Option<String> {
    if !cooldown_expired(check_path, interval) {
        return None;
    }
    touch_check_file(check_path);
    let current = current_version();
    let latest = fetch().ok()?;
    if update_available(&current, &latest) {
        Some(format!(
            "notice: ctlgr {latest} is available (you have {current}). Run `ctlgr update` to upgrade."
        ))
    } else {
        None
    }
}

/// Core update logic with injected fetch and install so tests run without
/// real network or filesystem access.
pub fn run_update_impl(
    current: &Version,
    fetch: impl Fn() -> Result<Version>,
    install: impl Fn(&Version) -> Result<()>,
) -> Result<()> {
    println!("checking for updates...");
    let latest = fetch().context("could not fetch latest version")?;
    if !update_available(current, &latest) {
        println!("already up to date ({})", current);
        return Ok(());
    }
    println!("ctlgr {} is available (you have {})", latest, current);
    println!("downloading ctlgr {}...", latest);
    install(&latest)?;
    println!("updated to {}", latest);
    Ok(())
}

// ── I/O wrappers (thin, not unit-tested) ─────────────────────────────────────

pub fn fetch_latest_version() -> Result<Version> {
    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");
    let body: serde_json::Value = ureq::get(&url)
        .timeout(Duration::from_secs(5))
        .set("User-Agent", &format!("{CRATE_NAME}/{CURRENT}"))
        .set("Accept", "application/vnd.github.v3+json")
        .call()
        .context("fetching latest release from GitHub")?
        .into_json()
        .context("parsing GitHub releases response")?;
    let tag = body["tag_name"]
        .as_str()
        .context("missing tag_name in GitHub response")?;
    Version::parse(tag.trim_start_matches('v'))
        .context("invalid version tag from GitHub")
}

fn download_and_install(version: &Version) -> Result<()> {
    use std::io::Read;
    let target = current_target()?;
    let url = format!(
        "https://github.com/{GITHUB_REPO}/releases/download/v{version}/ctlgr-v{version}-{target}"
    );
    println!("downloading {url}");

    let mut bytes = Vec::new();
    ureq::get(&url)
        .timeout(Duration::from_secs(60))
        .set("User-Agent", &format!("{CRATE_NAME}/{CURRENT}"))
        .call()
        .context("downloading binary")?
        .into_reader()
        .read_to_end(&mut bytes)
        .context("reading downloaded binary")?;

    let current_exe =
        std::env::current_exe().context("could not determine current executable path")?;
    let tmp = current_exe.with_extension("_new");
    std::fs::write(&tmp, &bytes).context("writing new binary")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))
            .context("setting permissions on new binary")?;
    }

    std::fs::rename(&tmp, &current_exe).with_context(|| {
        format!(
            "replacing {} — try with sudo if permission is denied",
            current_exe.display()
        )
    })?;
    Ok(())
}

/// Prints a notice to stderr if a newer version is available. Respects a
/// 24-hour cooldown so the check fires at most once per day.
pub fn check_and_notify() {
    if let Ok(path) = global_check_path() {
        if let Some(msg) = check_update_message(&path, CHECK_INTERVAL, fetch_latest_version) {
            eprintln!("{msg}");
        }
    }
}

pub fn run_update() -> Result<()> {
    run_update_impl(&current_version(), fetch_latest_version, download_and_install)
}
