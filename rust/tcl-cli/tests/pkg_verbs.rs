// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Integration tests for the `pkg` / `venv` / `docker` verb groups.
//!
//! These drive the built `tcl` binary and assert deterministic output that was
//! diffed byte-for-byte against the captured golden output. They
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
fn pkg_discover_uses_analysis_and_optimisation() {
    let dir = temp_dir("pkg-discover-analysis");
    std::fs::write(
        dir.join("tclpkg.tcl"),
        "package demo\nversion 1.0.0\nlicense MIT\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("main.tcl"),
        "proc load_json {} { package require json 1.2 }\n\
         proc load_tls {} {\n\
             set dep [string cat t l s]\n\
             set minimum 1.7\n\
             package require $dep $minimum\n\
         }\n\
         if {$optional} { package require Tk 8.6 }\n\
         package require $runtime_package\n",
    )
    .unwrap();

    let (stdout, stderr, code) = run_in(&dir, &["pkg", "discover", "--json"]);
    assert_eq!(code, 0, "{stderr}");
    let output: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(output["scanned_files"], 1);
    let requirements = output["requirements"].as_array().unwrap();
    assert!(requirements.iter().any(|requirement| {
        requirement["name"] == "json"
            && requirement["minimum"] == "1.2"
            && requirement["resolution"] == "literal"
            && requirement["status"] == "candidate"
    }));
    assert!(requirements.iter().any(|requirement| {
        requirement["name"] == "tls"
            && requirement["minimum"] == "1.7"
            && requirement["version_expression"] == "${minimum}"
            && requirement["resolution"] == "optimiser"
            && requirement["status"] == "candidate"
    }));
    assert!(requirements.iter().any(|requirement| {
        requirement["name"] == "Tk"
            && requirement["conditional"] == true
            && requirement["status"] == "review"
    }));
    assert!(requirements.iter().any(|requirement| {
        requirement["expression"] == "${runtime_package}"
            && requirement["name"].is_null()
            && requirement["status"] == "unresolved"
    }));
    assert!(output["added"].as_array().unwrap().is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pkg_discover_add_is_idempotent_and_skips_dependency_trees() {
    let dir = temp_dir("pkg-discover-add");
    std::fs::write(
        dir.join("tclpkg.tcl"),
        "package demo\nversion 1.0.0\nlicense MIT\nrequire json 1.0\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("main.tcl"),
        "package require json 1.2\n\
         package require tls 1.7\n\
         package require -exact pinned 2.0\n\
         package require ranged 1.0-2.0\n\
         package require Tcl 8.6\n\
         package require demo 1.0\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("vendor/dep")).unwrap();
    std::fs::write(
        dir.join("vendor/dep/dep.tcl"),
        "package require should_not_be_direct 9.0\n",
    )
    .unwrap();

    let (stdout, stderr, code) = run_in(&dir, &["pkg", "discover", "--add", "--json"]);
    assert_eq!(code, 0, "{stderr}");
    let output: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(output["added"].as_array().unwrap().len(), 1);
    assert_eq!(output["added"][0]["name"], "tls");
    assert_eq!(output["added"][0]["minimum"], "1.7");
    let requirements = output["requirements"].as_array().unwrap();
    assert!(requirements.iter().any(|requirement| {
        requirement["name"] == "pinned" && requirement["status"] == "review"
    }));
    assert!(requirements.iter().any(|requirement| {
        requirement["name"] == "ranged" && requirement["status"] == "review"
    }));
    assert!(
        requirements.iter().any(|requirement| {
            requirement["name"] == "Tcl" && requirement["status"] == "runtime"
        })
    );
    assert!(
        requirements.iter().any(|requirement| {
            requirement["name"] == "demo" && requirement["status"] == "self"
        })
    );

    let manifest = std::fs::read_to_string(dir.join("tclpkg.tcl")).unwrap();
    assert_eq!(manifest.matches("require json").count(), 1);
    assert_eq!(manifest.matches("require tls").count(), 1);
    assert!(!manifest.contains("require pinned"));
    assert!(!manifest.contains("require ranged"));
    assert!(!manifest.contains("require Tcl"));
    assert!(!manifest.contains("require demo"));
    assert!(!manifest.contains("should_not_be_direct"));
    assert!(!dir.join("tclpkg.lock").exists());

    let (_stdout, stderr, code) = run_in(&dir, &["pkg", "discover", "--add"]);
    assert_eq!(code, 0, "{stderr}");
    let manifest = std::fs::read_to_string(dir.join("tclpkg.tcl")).unwrap();
    assert_eq!(manifest.matches("require tls").count(), 1);
    let _ = std::fs::remove_dir_all(&dir);
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
    // The native CLI half: which releases and architectures it can install.
    assert!(stdout.contains("Native tcl CLI:"));
    assert!(stdout.contains("x86_64-unknown-linux-gnu"));
    assert!(stdout.contains("base families    debian, redhat"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn docker_recipe_cli_fetches_a_verified_release_asset() {
    let dir = temp_dir("docker-recipe-cli");
    let (stdout, _stderr, code) = run_in(
        &dir,
        &[
            "docker",
            "recipe",
            "debian:bookworm-slim",
            "--cli",
            "--cli-version",
            "2.2.1",
        ],
    );
    assert_eq!(code, 0);
    assert!(stdout.contains("ARG TCL_LSP_VERSION=2.2.1"));
    assert!(stdout.contains("SHA256SUMS"));
    assert!(stdout.contains("sha256sum -c -"));
    assert!(!stdout.contains("python"), "{stdout}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn docker_create_writes_dockerfile() {
    let dir = temp_dir("docker-create");
    let (_stdout, _stderr, code) = run_in(
        &dir,
        &["docker", "create", "--tcl-version", "8.6", "--no-packages"],
    );
    assert_eq!(code, 0);
    let content = std::fs::read_to_string(dir.join("Dockerfile")).unwrap();
    assert!(content.contains("require glibc, so Debian is the safe default"));
    assert!(content.contains("Alpine/musl is required, build tcl-lsp from source"));
    assert!(content.contains("FROM debian:bookworm-slim\n"));
    assert!(content.contains("# Install Tcl 8.6"));
    assert!(content.contains("WORKDIR /app"));
    assert!(content.trim_end().ends_with("CMD [\"tclsh\"]"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn docker_create_installs_the_native_cli() {
    let dir = temp_dir("docker-create-cli");
    let (_stdout, _stderr, code) = run_in(
        &dir,
        &[
            "docker",
            "create",
            "debian:bookworm-slim",
            "--tcl-version",
            "8.6",
            "--cli-version",
            "2.2.1",
        ],
    );
    assert_eq!(code, 0);
    let content = std::fs::read_to_string(dir.join("Dockerfile")).unwrap();
    // The CLI arrives as a verified native release asset, never a zipapp.
    assert!(content.contains("ARG TCL_LSP_VERSION=2.2.1"));
    assert!(content.contains("releases/download/$tag"));
    assert!(content.contains("sha256sum -c -"));
    assert!(content.contains("RUN if [ -f tclpkg.lock ]; then tcl pkg install --frozen; fi"));
    assert!(
        !content.to_lowercase().contains("python"),
        "generated Dockerfile still mentions Python:\n{content}"
    );
    assert!(!content.contains(".pyz"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn docker_create_rejects_the_cli_on_musl() {
    let dir = temp_dir("docker-create-alpine");
    let (_stdout, stderr, code) = run_in(&dir, &["docker", "create", "alpine:3.19"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("musl"), "{stderr}");
    assert!(stderr.contains("build tcl-lsp from source"), "{stderr}");
    assert!(!dir.join("Dockerfile").exists());

    // Without the CLI verbs an alpine image is still generated.
    let (_stdout, _stderr, code) =
        run_in(&dir, &["docker", "create", "alpine:3.19", "--no-packages"]);
    assert_eq!(code, 0);
    let content = std::fs::read_to_string(dir.join("Dockerfile")).unwrap();
    assert!(content.contains("RUN apk add --no-cache tcl"));
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

/// Shared setup: init a project with one local `dep` package and install it,
/// returning `(base, cache, proj_dir)`. The caller cleans up `base`.
fn install_local_dep(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
    let base = temp_dir(tag);
    let cache = base.join("cache");
    let dir = base.join("proj");
    std::fs::create_dir_all(&dir).unwrap();

    let dep = base.join("dep-src");
    std::fs::create_dir_all(&dep).unwrap();
    std::fs::write(
        dep.join("tclpkg.tcl"),
        "package dep\nversion 1.0.0\nprovides dep::api\nlicense MIT\n",
    )
    .unwrap();
    std::fs::write(dep.join("dep.tcl"), "proc dep::hi {} { return hi }\n").unwrap();

    let (_o, _e, code) = run_in_cache(
        &dir,
        Some(&cache),
        &["pkg", "init", "--name", "demo", "--version", "1.0.0"],
    );
    assert_eq!(code, 0);
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
    let (_o, _e, code) = run_in_cache(&dir, Some(&cache), &["pkg", "install"]);
    assert_eq!(code, 0);
    (base, cache, dir)
}

// `pkg verify` must recompute and compare the integrity hash,
// so tampering with a materialised file is detected — not just check that the
// lockfile's integrity string is non-empty.
#[test]
fn pkg_verify_detects_tampered_content() {
    let (base, cache, dir) = install_local_dep("pkg-verify-tamper");

    // A clean tree verifies.
    let (_o, _e, code) = run_in_cache(&dir, Some(&cache), &["pkg", "verify"]);
    assert_eq!(code, 0, "clean tree should verify");

    // Tamper with the materialised package content.
    let victim = dir.join("lib").join("dep-1.0.0").join("dep.tcl");
    std::fs::write(&victim, "proc dep::hi {} { return TAMPERED }\n").unwrap();

    // verify must now fail.
    let (_o, stderr, code) = run_in_cache(&dir, Some(&cache), &["pkg", "verify"]);
    assert_ne!(code, 0, "tampered tree must fail verification");
    assert!(
        stderr.contains("integrity verification") || stderr.contains("mismatch"),
        "stderr: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&base);
}

// `pkg sync` must actually materialise the locked packages into
// `lib/`, reconstructing them from the CAS — not merely print the lockfile.
#[test]
fn pkg_sync_materialises_from_lockfile() {
    let (base, cache, dir) = install_local_dep("pkg-sync-materialise");

    // Remove the materialised tree; the lockfile + CAS still describe it.
    let lib = dir.join("lib");
    std::fs::remove_dir_all(&lib).unwrap();
    assert!(!lib.exists());

    // sync must recreate it from the lockfile.
    let (_o, _e, code) = run_in_cache(&dir, Some(&cache), &["pkg", "sync"]);
    assert_eq!(code, 0, "sync should succeed");
    let materialised = dir.join("lib").join("dep-1.0.0").join("dep.tcl");
    assert!(
        materialised.exists(),
        "sync did not materialise the package: {}",
        materialised.display()
    );

    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn pkg_install_resolves_transitive_dependencies() {
    // Root -> dep -> sub, all via local path sources (no network, no tclsh).
    // Before the resolver was wired with a provider, `sub` never appeared and
    // `dep`'s lockfile `requires` were empty; this pins the fixed behaviour.
    let base = temp_dir("pkg-transitive");
    let cache = base.join("cache");
    let dir = base.join("proj");
    std::fs::create_dir_all(&dir).unwrap();

    // Leaf transitive package `sub`.
    let sub = base.join("sub-src");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(
        sub.join("tclpkg.tcl"),
        "package sub\nversion 1.0.0\nprovides sub::api\nlicense MIT\n",
    )
    .unwrap();
    std::fs::write(sub.join("sub.tcl"), "proc sub::hi {} { return hi }\n").unwrap();

    // Direct dependency `dep`, which itself requires `sub` (with a -source so
    // the transitive package can be fetched without a registry).
    let dep = base.join("dep-src");
    std::fs::create_dir_all(&dep).unwrap();
    std::fs::write(
        dep.join("tclpkg.tcl"),
        format!(
            "package dep\nversion 1.0.0\nlicense MIT\nrequire sub 1.0.0 -source {}\n",
            sub.to_str().unwrap()
        ),
    )
    .unwrap();
    std::fs::write(dep.join("dep.tcl"), "proc dep::hi {} { return hi }\n").unwrap();

    // init + add the direct dependency with an explicit local source.
    let (_o, _e, code) = run_in_cache(
        &dir,
        Some(&cache),
        &["pkg", "init", "--name", "demo", "--version", "1.0.0"],
    );
    assert_eq!(code, 0);
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

    let (_o, stderr, code) = run_in_cache(&dir, Some(&cache), &["pkg", "install"]);
    assert_eq!(code, 0, "stderr: {stderr}");

    let lock = std::fs::read_to_string(dir.join("tclpkg.lock")).unwrap();
    // The transitive package is now present in the lockfile...
    assert!(lock.contains("\"name\": \"sub\""), "lockfile: {lock}");
    // ...and materialised on disk.
    assert!(
        dir.join("lib").join("sub-1.0.0").join("sub.tcl").exists(),
        "expected transitive package materialised"
    );
    // ...and `dep` records its requirement on `sub`.
    assert!(
        lock.contains("sub@1.0.0"),
        "expected dep to record requires sub@1.0.0, lockfile: {lock}"
    );

    // `pkg list` classifies dep as direct and sub as transitive.
    let (stdout, _e, code) = run_in_cache(&dir, Some(&cache), &["pkg", "list"]);
    assert_eq!(code, 0);
    let dep_line = stdout.lines().find(|l| l.starts_with("dep ")).unwrap();
    let sub_line = stdout.lines().find(|l| l.starts_with("sub ")).unwrap();
    assert!(dep_line.contains("direct"), "dep line: {dep_line}");
    assert!(sub_line.contains("trans"), "sub line: {sub_line}");

    // A subsequent --frozen install reproduces the same graph (transitive deps
    // survive the offline re-resolution driven by the lockfile provider).
    let (_o, stderr, code) = run_in_cache(&dir, Some(&cache), &["pkg", "install", "--frozen"]);
    assert_eq!(code, 0, "frozen stderr: {stderr}");

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
