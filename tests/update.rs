use std::time::Duration;
use tempfile::TempDir;

use ctlgr::update::{
    check_update_message, cooldown_expired, current_target, current_version, global_check_path,
    run_update_impl, touch_check_file, update_available,
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
