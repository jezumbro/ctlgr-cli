use assert_cmd::Command;
use criterion::{criterion_group, criterion_main, Criterion};
use tempfile::TempDir;

fn bench_help(c: &mut Criterion) {
    c.bench_function("ctlgr --help", |b| {
        b.iter(|| {
            Command::cargo_bin("ctlgr")
                .unwrap()
                .arg("--help")
                .output()
                .unwrap()
        })
    });
}

fn bench_config_list(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    let catalog = tmp.path().join("catalog");
    std::fs::create_dir(&catalog).unwrap();
    std::fs::write(
        tmp.path().join(".ctlgr"),
        format!(r#"{{"path":"{}"}}"#, catalog.display()),
    )
    .unwrap();

    c.bench_function("ctlgr config list", |b| {
        b.iter(|| {
            Command::cargo_bin("ctlgr")
                .unwrap()
                .args(["config", "list"])
                .current_dir(tmp.path())
                .output()
                .unwrap()
        })
    });
}

fn bench_search_no_results(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    let catalog = tmp.path().join("catalog");
    std::fs::create_dir(&catalog).unwrap();
    std::fs::write(catalog.join("page.html"), "<html><body><h1>hi</h1></body></html>").unwrap();
    std::fs::write(
        tmp.path().join(".ctlgr"),
        format!(r#"{{"path":"{}"}}"#, catalog.display()),
    )
    .unwrap();

    c.bench_function("ctlgr search (1 file, 0 results)", |b| {
        b.iter(|| {
            Command::cargo_bin("ctlgr")
                .unwrap()
                .args(["search", ".nonexistent"])
                .current_dir(tmp.path())
                .output()
                .unwrap()
        })
    });
}

criterion_group!(benches, bench_help, bench_config_list, bench_search_no_results);
criterion_main!(benches);
