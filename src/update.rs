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

/// Resolve the binary target triple for a given OS and architecture.
pub fn target_for(os: &str, arch: &str) -> Result<&'static str> {
    match (os, arch) {
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-musl"),
        ("linux", "aarch64") => Ok("aarch64-unknown-linux-musl"),
        (os, arch) => anyhow::bail!("unsupported platform: {os}/{arch}"),
    }
}

pub fn current_target() -> Result<&'static str> {
    target_for(std::env::consts::OS, std::env::consts::ARCH)
}

/// Build the GitHub release download URL for a given version and target.
pub fn release_url(version: &Version, target: &str) -> String {
    format!(
        "https://github.com/{GITHUB_REPO}/releases/download/v{version}/ctlgr-v{version}-{target}"
    )
}

/// Write `bytes` to `dest`, using an adjacent `.._new` temp file and an
/// atomic rename. Sets executable permissions on Unix.
pub fn install_binary(bytes: &[u8], dest: &Path) -> Result<()> {
    let tmp = dest.with_extension("_new");
    std::fs::write(&tmp, bytes).context("writing new binary")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))
            .context("setting permissions on new binary")?;
    }

    std::fs::rename(&tmp, dest).with_context(|| {
        format!(
            "replacing {} — try with sudo if permission is denied",
            dest.display()
        )
    })?;
    Ok(())
}

/// Core download + install logic with injected HTTP and destination path so
/// tests can run without real network access or touching the running binary.
pub fn download_and_install_impl(
    version: &Version,
    download: impl Fn(&str) -> Result<Vec<u8>>,
    dest: &Path,
) -> Result<()> {
    let target = current_target()?;
    let url = release_url(version, target);
    println!("downloading {url}");
    let bytes = download(&url)?;
    install_binary(&bytes, dest)
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

/// Print a notice to stderr if a newer version is available. Uses injected
/// `fetch` and `check_path` so tests can exercise the print path without
/// a real network call.
pub fn check_and_notify_impl(
    check_path: &Path,
    interval: Duration,
    fetch: impl Fn() -> Result<Version>,
) {
    if let Some(msg) = check_update_message(check_path, interval, fetch) {
        eprintln!("{msg}");
    }
}

// ── I/O wrappers (thin, not unit-tested) ─────────────────────────────────────

pub fn fetch_latest_version() -> Result<Version> {
    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(5)))
        .build()
        .new_agent();
    let body: serde_json::Value = agent
        .get(&url)
        .header("User-Agent", &format!("{CRATE_NAME}/{CURRENT}"))
        .header("Accept", "application/vnd.github.v3+json")
        .call()
        .context("fetching latest release from GitHub")?
        .body_mut()
        .read_json()
        .context("parsing GitHub releases response")?;
    let tag = body["tag_name"]
        .as_str()
        .context("missing tag_name in GitHub response")?;
    Version::parse(tag.trim_start_matches('v'))
        .context("invalid version tag from GitHub")
}

fn http_download(url: &str) -> Result<Vec<u8>> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
        .build()
        .new_agent();
    agent
        .get(url)
        .header("User-Agent", &format!("{CRATE_NAME}/{CURRENT}"))
        .call()
        .context("downloading binary")?
        .body_mut()
        .read_to_vec()
        .context("reading downloaded binary")
}

fn download_and_install(version: &Version) -> Result<()> {
    let dest = std::env::current_exe().context("could not determine current executable path")?;
    download_and_install_impl(version, http_download, &dest)
}

/// Prints a notice to stderr if a newer version is available. Respects a
/// 24-hour cooldown so the check fires at most once per day.
pub fn check_and_notify() {
    if let Ok(path) = global_check_path() {
        check_and_notify_impl(&path, CHECK_INTERVAL, fetch_latest_version);
    }
}

pub fn run_update() -> Result<()> {
    run_update_impl(&current_version(), fetch_latest_version, download_and_install)
}
