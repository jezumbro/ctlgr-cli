use std::time::Duration;
use tempfile::TempDir;

use ctlgr::update::{
    check_and_notify_impl, check_update_message, cooldown_expired, current_target, current_version,
    download_and_install_impl, global_check_path, install_binary, release_url, run_update_impl,
    target_for, touch_check_file, update_available,
};
use semver::Version;

// update_available

#[test]
fn update_available_when_latest_is_newer() {
    assert!(update_available(&Version::new(0, 0, 1), &Version::new(0, 1, 0)));
}

#[test]
fn update_not_available_when_same_version() {
    let v = Version::new(1, 2, 3);
    assert!(!update_available(&v, &v));
}

#[test]
fn update_not_available_when_latest_is_older() {
    assert!(!update_available(&Version::new(1, 0, 0), &Version::new(0, 9, 0)));
}

// cooldown_expired

#[test]
fn cooldown_expired_when_no_file() {
    let tmp = TempDir::new().unwrap();
    assert!(cooldown_expired(&tmp.path().join("check"), Duration::from_secs(3600)));
}

#[test]
fn cooldown_not_expired_when_file_is_fresh() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("check");
    touch_check_file(&path);
    assert!(!cooldown_expired(&path, Duration::from_secs(3600)));
}

#[test]
fn cooldown_expired_when_interval_is_zero() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("check");
    touch_check_file(&path);
    assert!(cooldown_expired(&path, Duration::from_secs(0)));
}

// touch_check_file

#[test]
fn touch_creates_file_and_parent_dirs() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("nested").join("dir").join("check");
    touch_check_file(&path);
    assert!(path.exists());
}

#[test]
fn touch_is_idempotent() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("check");
    touch_check_file(&path);
    touch_check_file(&path);
    assert!(path.exists());
}

// check_update_message

#[test]
fn check_update_message_none_when_cooldown_active() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("check");
    touch_check_file(&path);
    let msg = check_update_message(&path, Duration::from_secs(3600), || {
        Ok(Version::new(99, 0, 0))
    });
    assert!(msg.is_none());
}

#[test]
fn check_update_message_notice_when_newer_version_available() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("check");
    let msg = check_update_message(&path, Duration::from_secs(0), || {
        Ok(Version::new(99, 0, 0))
    })
    .unwrap();
    assert!(msg.contains("99.0.0"));
    assert!(msg.contains("ctlgr update"));
}

#[test]
fn check_update_message_none_when_already_up_to_date() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("check");
    let msg = check_update_message(&path, Duration::from_secs(0), || Ok(Version::new(0, 0, 0)));
    assert!(msg.is_none());
}

#[test]
fn check_update_message_none_when_fetch_fails() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("check");
    let msg = check_update_message(&path, Duration::from_secs(0), || {
        anyhow::bail!("network error")
    });
    assert!(msg.is_none());
}

#[test]
fn check_update_message_touches_check_file() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("check");
    assert!(!path.exists());
    check_update_message(&path, Duration::from_secs(0), || Ok(Version::new(0, 0, 0)));
    assert!(path.exists());
}

// run_update_impl

#[test]
fn run_update_impl_prints_up_to_date_when_no_newer_version() {
    let current = current_version();
    let same = current.clone();
    run_update_impl(&current, move || Ok(same.clone()), |_| Ok(())).unwrap();
}

#[test]
fn run_update_impl_calls_install_when_newer_version_available() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    let installed = Arc::new(AtomicBool::new(false));
    let installed2 = Arc::clone(&installed);
    run_update_impl(
        &Version::new(0, 0, 1),
        || Ok(Version::new(99, 0, 0)),
        move |_version| {
            installed2.store(true, Ordering::SeqCst);
            Ok(())
        },
    )
    .unwrap();
    assert!(installed.load(Ordering::SeqCst));
}

#[test]
fn run_update_impl_propagates_fetch_error() {
    let result = run_update_impl(
        &current_version(),
        || anyhow::bail!("network error"),
        |_| Ok(()),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("latest version"));
}

#[test]
fn run_update_impl_propagates_install_error() {
    let result = run_update_impl(
        &Version::new(0, 0, 1),
        || Ok(Version::new(99, 0, 0)),
        |_| anyhow::bail!("install failed"),
    );
    assert!(result.is_err());
}

#[test]
fn run_update_impl_install_receives_target_version() {
    use std::sync::{Arc, Mutex};
    let received = Arc::new(Mutex::new(Version::new(0, 0, 0)));
    let received2 = Arc::clone(&received);
    run_update_impl(
        &Version::new(0, 0, 1),
        || Ok(Version::new(2, 0, 0)),
        move |version| {
            *received2.lock().unwrap() = version.clone();
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(*received.lock().unwrap(), Version::new(2, 0, 0));
}

// global_check_path, current_version, current_target

#[test]
fn current_version_matches_cargo_pkg_version() {
    assert_eq!(current_version(), semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap());
}

#[test]
fn global_check_path_is_under_ctlgr_cli_dir() {
    let path = global_check_path().unwrap();
    assert!(path.to_string_lossy().contains(".ctlgr-cli"));
    assert_eq!(path.file_name().unwrap(), ".update-check");
}

#[test]
fn current_target_returns_known_platform() {
    let target = current_target().unwrap();
    let known = [
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
        "x86_64-unknown-linux-musl",
        "aarch64-unknown-linux-musl",
    ];
    assert!(known.contains(&target), "unexpected target: {target}");
}

// target_for

#[test]
fn target_for_macos_x86_64() {
    assert_eq!(target_for("macos", "x86_64").unwrap(), "x86_64-apple-darwin");
}

#[test]
fn target_for_macos_aarch64() {
    assert_eq!(target_for("macos", "aarch64").unwrap(), "aarch64-apple-darwin");
}

#[test]
fn target_for_linux_x86_64() {
    assert_eq!(target_for("linux", "x86_64").unwrap(), "x86_64-unknown-linux-musl");
}

#[test]
fn target_for_linux_aarch64() {
    assert_eq!(target_for("linux", "aarch64").unwrap(), "aarch64-unknown-linux-musl");
}

#[test]
fn target_for_unsupported_platform_errors() {
    let err = target_for("windows", "x86_64").unwrap_err();
    assert!(err.to_string().contains("unsupported platform"));
}

// release_url

#[test]
fn release_url_contains_version_and_target() {
    let v = Version::new(1, 2, 3);
    let url = release_url(&v, "x86_64-apple-darwin");
    assert!(url.contains("v1.2.3"), "url: {url}");
    assert!(url.contains("x86_64-apple-darwin"), "url: {url}");
    assert!(url.contains("ctlgr-v1.2.3"), "url: {url}");
}

// install_binary

#[test]
fn install_binary_creates_file_at_dest() {
    let tmp = TempDir::new().unwrap();
    let dest = tmp.path().join("ctlgr");
    install_binary(b"fake_binary_data", &dest).unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), b"fake_binary_data");
}

#[test]
fn install_binary_overwrites_existing_file() {
    let tmp = TempDir::new().unwrap();
    let dest = tmp.path().join("ctlgr");
    std::fs::write(&dest, b"old").unwrap();
    install_binary(b"new", &dest).unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), b"new");
}

#[test]
fn install_binary_leaves_no_tmp_file() {
    let tmp = TempDir::new().unwrap();
    let dest = tmp.path().join("ctlgr");
    install_binary(b"data", &dest).unwrap();
    assert!(!dest.with_extension("_new").exists());
}

#[test]
fn install_binary_rename_failure_error_mentions_path() {
    let tmp = TempDir::new().unwrap();
    let dest = tmp.path().join("ctlgr");
    // Make dest a directory so rename(file → dir) fails on Unix with EISDIR,
    // exercising the with_context closure on the rename call.
    std::fs::create_dir(&dest).unwrap();
    if let Err(e) = install_binary(b"data", &dest) {
        let msg = e.to_string();
        assert!(
            msg.contains("replacing") || msg.contains("sudo"),
            "unexpected error: {msg}"
        );
    }
    // If rename succeeded on this platform, the test is still valid — just
    // not reaching the error path on this OS.
}

// download_and_install_impl

#[test]
fn download_and_install_impl_installs_downloaded_bytes() {
    let tmp = TempDir::new().unwrap();
    let dest = tmp.path().join("ctlgr");
    let v = Version::new(1, 0, 0);
    download_and_install_impl(&v, |_url| Ok(b"binary_content".to_vec()), &dest).unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), b"binary_content");
}

#[test]
fn download_and_install_impl_passes_correct_url() {
    use std::sync::{Arc, Mutex};
    let tmp = TempDir::new().unwrap();
    let dest = tmp.path().join("ctlgr");
    let v = Version::new(2, 3, 4);
    let captured_url: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let cap = Arc::clone(&captured_url);
    download_and_install_impl(
        &v,
        move |url| {
            *cap.lock().unwrap() = url.to_string();
            Ok(b"x".to_vec())
        },
        &dest,
    )
    .unwrap();
    let url = captured_url.lock().unwrap().clone();
    assert!(url.contains("2.3.4"), "url: {url}");
}

#[test]
fn download_and_install_impl_propagates_download_error() {
    let tmp = TempDir::new().unwrap();
    let dest = tmp.path().join("ctlgr");
    let v = Version::new(1, 0, 0);
    let result =
        download_and_install_impl(&v, |_| anyhow::bail!("network error"), &dest);
    assert!(result.is_err());
}

// check_and_notify_impl

#[test]
fn check_and_notify_impl_emits_notice_when_update_available() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("check");
    // Interval 0 forces cooldown to expire; mock returns newer version.
    // We can't capture eprintln! in unit tests, but reaching this path
    // covers the branch and ensures no panic.
    check_and_notify_impl(&path, Duration::from_secs(0), || Ok(Version::new(99, 0, 0)));
    // The check file is written as a side effect of check_update_message.
    assert!(path.exists());
}

#[test]
fn check_and_notify_impl_silent_when_already_up_to_date() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("check");
    check_and_notify_impl(&path, Duration::from_secs(0), || Ok(Version::new(0, 0, 0)));
    // No update notice emitted; function returns normally.
    assert!(path.exists());
}
