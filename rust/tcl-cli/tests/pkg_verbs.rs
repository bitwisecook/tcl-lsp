//! Integration tests for the `pkg` / `venv` / `docker` verb groups.
//!
//! These drive the built `tcl` binary and assert deterministic output that was
//! diffed byte-for-byte against the Python CLI (`tooling/tcl/verbs/*`). They
//! avoid any network or `tclsh` dependency so they are stable in CI.

use std::path::PathBuf;
use std::process::Command;

fn run_in(dir: &std::path::Path, args: &[&str]) -> (String, String, i32) {
    run_in_cache(dir, None, args)
}

/// Like [`run_in`] but pins `XDG_CACHE_HOME` so the content-addressable store
/// writes under a test-scoped directory (keeps `install`/`vendor` hermetic).
fn run_in_cache(
    dir: &std::path::Path,
    cache: Option<&std::path::Path>,
    args: &[&str],
) -> (String, String, i32) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_tcl"));
    cmd.args(args).current_dir(dir);
    if let Some(cache) = cache {
        cmd.env("XDG_CACHE_HOME", cache);
    }
    let output = cmd.output().expect("failed to spawn tcl binary");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code().unwrap_or(-1),
    )
}

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("tcl-pkg-it-{tag}-{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn docker_recipe_alpine_86() {
    let dir = temp_dir("docker-recipe");
    let (stdout, _stderr, code) = run_in(
        &dir,
        &["docker", "recipe", "alpine:3.19", "--tcl-version", "8.6"],
    );
    assert_eq!(code, 0);
    assert_eq!(stdout, "RUN apk add --no-cache tcl\n");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn docker_info_lists_families() {
    let dir = temp_dir("docker-info");
    let (stdout, _stderr, code) = run_in(&dir, &["docker", "info"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Supported Tcl versions:"));
    assert!(stdout.contains("alpine        8.4, 8.5, 8.6, 9.0"));
    assert!(stdout.contains("debian        8.4, 8.5, 8.6, 9.0"));
    assert!(stdout.contains("redhat        8.4, 8.5, 8.6, 9.0"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn docker_create_writes_dockerfile() {
    let dir = temp_dir("docker-create");
    let (_stdout, _stderr, code) = run_in(
        &dir,
        &[
            "docker",
            "create",
            "debian:bookworm-slim",
            "--tcl-version",
            "8.6",
            "--no-packages",
        ],
    );
    assert_eq!(code, 0);
    let content = std::fs::read_to_string(dir.join("Dockerfile")).unwrap();
    assert!(content.starts_with("FROM debian:bookworm-slim\n"));
    assert!(content.contains("# Install Tcl 8.6"));
    assert!(content.contains("WORKDIR /app"));
    assert!(content.trim_end().ends_with("CMD [\"tclsh\"]"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pkg_init_install_materialise_roundtrip() {
    let base = temp_dir("pkg-roundtrip");
    let cache = base.join("cache");
    let dir = base.join("proj");
    std::fs::create_dir_all(&dir).unwrap();

    // A local package to install (exercises fetch+materialise offline, no net).
    let dep = base.join("dep-src");
    std::fs::create_dir_all(&dep).unwrap();
    std::fs::write(
        dep.join("tclpkg.tcl"),
        "package dep\nversion 1.0.0\nprovides dep::api\nlicense MIT\n",
    )
    .unwrap();
    std::fs::write(dep.join("dep.tcl"), "proc dep::hi {} { return hi }\n").unwrap();

    // init
    let (_o, _e, code) = run_in_cache(
        &dir,
        Some(&cache),
        &["pkg", "init", "--name", "demo", "--version", "1.0.0"],
    );
    assert_eq!(code, 0);
    let manifest = std::fs::read_to_string(dir.join("tclpkg.tcl")).unwrap();
    assert!(manifest.contains("package     demo"));
    assert!(manifest.contains("version     1.0.0"));

    // add the dependency with an explicit local source
    let (_o, _e, code) = run_in_cache(
        &dir,
        Some(&cache),
        &[
            "pkg",
            "add",
            "dep",
            "1.0.0",
            "--source",
            dep.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0);

    // install resolves, fetches, stores, materialises, and writes the lockfile
    let (_o, _e, code) = run_in_cache(&dir, Some(&cache), &["pkg", "install"]);
    assert_eq!(code, 0);
    let lock = std::fs::read_to_string(dir.join("tclpkg.lock")).unwrap();
    assert!(lock.contains("\"name\": \"dep\""));
    assert!(lock.contains("\"version\": \"1.0.0\""));
    assert!(
        lock.contains("\"integrity\": \"sha256-"),
        "lockfile: {lock}"
    );
    assert!(lock.contains("\"type\": \"path\""));
    // Canonical JSON: sorted keys, 2-space indent, trailing newline.
    assert!(lock.ends_with("}\n"));

    // the package is materialised into ./lib/<name>-<version>/
    let materialised = dir.join("lib").join("dep-1.0.0").join("dep.tcl");
    assert!(materialised.exists(), "expected {}", materialised.display());

    // verify passes now that integrity is populated
    let (_o, _e, code) = run_in_cache(&dir, Some(&cache), &["pkg", "verify"]);
    assert_eq!(code, 0);

    // list shows the resolved package
    let (stdout, _e, code) = run_in_cache(&dir, Some(&cache), &["pkg", "list"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("NAME"));
    assert!(stdout.contains("dep"));
    assert!(stdout.contains("1.0.0"));

    // tree --json round-trips through the canonical emitter
    let (stdout, _e, code) = run_in_cache(&dir, Some(&cache), &["pkg", "tree", "--json"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("\"name\": \"demo\""));

    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn pkg_search_offline_without_cache_errors() {
    let dir = temp_dir("pkg-search");
    let empty_cache = temp_dir("empty-cache");
    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(["pkg", "search", "json", "--offline"])
        .current_dir(&dir)
        .env("XDG_CACHE_HOME", &empty_cache)
        .output()
        .expect("spawn");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("offline"), "stderr was: {stderr}");
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&empty_cache);
}
