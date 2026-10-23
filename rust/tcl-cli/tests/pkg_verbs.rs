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
//! These drive the built `tcl` binary without network access. Environment-list
//! controls pair original C Tcl binaries with their matching script libraries.

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
fn pkg_remove_uses_exact_manifest_declarations_and_preserves_neighbouring_commands() {
    let dir = temp_dir("pkg-remove-declarations");
    let manifest = dir.join("tclpkg.tcl");
    let source = "package demo; version 1.0\n\
require foo-bar 1.0; require foo::bar 1.0; require foo.bar 1.0\n\
require \"f\\x6fo\" \\\n    1.0; provides foo\n\
dev-require {test name} 1.0\n";
    std::fs::write(&manifest, source).unwrap();
    let (stdout, stderr, code) = run_in(&dir, &["pkg", "remove", "foo", "--json"]);
    assert_eq!(code, 0, "{stderr}");
    let output: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(output["directives_removed"], 1);
    let edited = std::fs::read_to_string(&manifest).unwrap();
    assert_eq!(
        edited,
        source.replace("require \"f\\x6fo\" \\\n    1.0", "")
    );
    let (_stdout, stderr, code) = run_in(&dir, &["pkg", "remove", "test name"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(
        !std::fs::read_to_string(&manifest)
            .unwrap()
            .contains("dev-require")
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn pkg_remove_refuses_unmatched_names_and_invalid_manifests_without_writing() {
    let dir = temp_dir("pkg-remove-refusal");
    let manifest = dir.join("tclpkg.tcl");
    for source in [
        "package demo\nversion 1.0\nrequire foo-bar 1.0\n",
        "package demo\nrequire foo 1.0\n",
        "package demo\nversion 1.0\nrequire foo {unclosed\n",
    ] {
        std::fs::write(&manifest, source).unwrap();
        let (_stdout, stderr, code) = run_in(&dir, &["pkg", "remove", "foo"]);
        assert_eq!(code, 1, "{stderr}");
        assert_eq!(std::fs::read_to_string(&manifest).unwrap(), source);
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn pkg_add_preserves_literal_name_and_source_argv_and_remove_selects_the_same_name() {
    let dir = temp_dir("pkg-add-literal-words");
    let manifest = dir.join("tclpkg.tcl");
    let original = "package demo\nversion 1.0\n";
    std::fs::write(&manifest, original).unwrap();
    let package = "foo; require injected 9.0 [literal]$value";
    let url = "https://example.test/a b;[literal]$value\\tail";
    let (_stdout, stderr, code) = run_in(&dir, &["pkg", "add", package, "1.2", "--source", url]);
    assert_eq!(code, 0, "{stderr}");
    let source = std::fs::read_to_string(&manifest).unwrap();
    let parsed = tcl_pkg::manifest::load_manifest_text(&source, None).unwrap();
    assert_eq!(parsed.requires.len(), 1);
    assert_eq!(parsed.requires[0].name, package);
    assert_eq!(parsed.requires[0].source_url.as_deref(), Some(url));
    let (_stdout, stderr, code) = run_in(&dir, &["pkg", "remove", package]);
    assert_eq!(code, 0, "{stderr}");
    assert!(
        tcl_pkg::manifest::load_manifest(&manifest)
            .unwrap()
            .requires
            .is_empty()
    );
    std::fs::write(&manifest, original).unwrap();
    let (_stdout, stderr, code) = run_in(&dir, &["pkg", "add", "foo", "1.0; require injected 9.0"]);
    assert_eq!(code, 1, "{stderr}");
    assert_eq!(std::fs::read_to_string(&manifest).unwrap(), original);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn pkg_init_preserves_literal_package_names_and_refuses_invalid_headers_before_writing() {
    let dir = temp_dir("pkg-init-literal-name");
    let path = dir.join("tclpkg.tcl");
    let name = "demo; require injected 9.0 [literal]$value";
    let (_stdout, stderr, code) = run_in(&dir, &["pkg", "init", "--name", name]);
    assert_eq!(code, 0, "{stderr}");
    let original = std::fs::read_to_string(&path).unwrap();
    let ast = tcl_pkg::manifest::load_manifest(&path).unwrap();
    assert_eq!(ast.name, name);
    assert!(ast.requires.is_empty());
    let (stdout, stderr, code) = run_in(
        &dir,
        &[
            "pkg",
            "init",
            "--force",
            "--name",
            "replacement",
            "--version",
            "1.0; require injected 9.0",
        ],
    );
    assert_eq!(code, 1, "{stderr}");
    assert!(stdout.is_empty());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    std::fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn pkg_init_refuses_a_lossy_directory_package_name_and_accepts_an_explicit_name() {
    use std::os::unix::ffi::OsStringExt as _;
    let base = temp_dir("pkg-init-original-directory-name");
    let dir = base.join(std::ffi::OsString::from_vec(b"project-\xff".to_vec()));
    std::fs::create_dir_all(&dir).unwrap();
    let (_stdout, stderr, code) = run_in(&dir, &["pkg", "init"]);
    assert_eq!(code, 1, "{stderr}");
    assert!(stderr.contains("supply --name"), "{stderr}");
    assert!(!dir.join("tclpkg.tcl").exists());
    let (_stdout, stderr, code) = run_in(&dir, &["pkg", "init", "--name", "explicit"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        tcl_pkg::manifest::load_manifest(dir.join("tclpkg.tcl"))
            .unwrap()
            .name,
        "explicit"
    );
    std::fs::remove_dir_all(base).unwrap();
}

#[test]
fn pkg_freeze_preserves_literal_dependencies_and_stages_all_source_before_emission() {
    use tcl_pkg::lockfile::{LockFile, LockedPackage, SourceSpec, write_lockfile};
    let dir = temp_dir("pkg-freeze-literal-requirements");
    let header = "package demo\nversion 1.0\n";
    std::fs::write(dir.join("tclpkg.tcl"), header).unwrap();
    let package = |name: &str, version: &str, url: &str, dev| LockedPackage {
        name: name.to_owned(),
        version: version.to_owned(),
        source: SourceSpec::new("path", url),
        integrity: String::new(),
        size: 0,
        requires: Vec::new(),
        provides: Vec::new(),
        license: "MIT".to_owned(),
        dev,
        spec_integrity: None,
    };
    let name = "test name; [literal]$value";
    let url = "https://example.test/a b;[literal]$value\\tail";
    let mut locked = LockFile {
        name: "demo".to_owned(),
        packages: vec![
            package("plain", "1.0", "", false),
            package(name, "2.0", url, true),
        ],
        ..LockFile::default()
    };
    write_lockfile(&locked, dir.join("tclpkg.lock")).unwrap();
    let (stdout, stderr, code) = run_in(&dir, &["pkg", "freeze"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("require plain 1.0\n"));
    let ast = tcl_pkg::manifest::load_manifest_text(&format!("{header}{stdout}"), None).unwrap();
    assert_eq!(ast.requires[0].name, "plain");
    assert_eq!(ast.dev_requires[0].name, name);
    assert_eq!(ast.dev_requires[0].source_url.as_deref(), Some(url));
    locked
        .packages
        .push(package("zzz", "1.0; require injected 9.0", "", false));
    write_lockfile(&locked, dir.join("tclpkg.lock")).unwrap();
    let (stdout, stderr, code) = run_in(&dir, &["pkg", "freeze"]);
    assert_eq!(code, 1, "{stderr}");
    assert!(stdout.is_empty());
    let (stdout, stderr, code) = run_in(&dir, &["pkg", "freeze", "--json"]);
    assert_eq!(code, 0, "{stderr}");
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json[name], "2.0");
    assert_eq!(json["zzz"], "1.0; require injected 9.0");
    std::fs::remove_dir_all(dir).unwrap();
}

/// `pkg discover --dialect` is validated by the one resolver: a language id
/// that only selects an environment and an environment a workspace pack
/// declares are accepted, and an unknown name is refused.
#[test]
fn pkg_discover_validates_its_dialect_after_the_packs_are_published() {
    let dir = temp_dir("pkg-discover-dialect");
    std::fs::write(
        dir.join("tclpkg.tcl"),
        "package demo\nversion 1.0.0\nlicense MIT\n",
    )
    .unwrap();
    std::fs::write(dir.join("main.tcl"), "package require json 1.2\n").unwrap();
    std::fs::create_dir_all(dir.join(".tcl-lsp")).unwrap();
    std::fs::write(
        dir.join(".tcl-lsp/mypack.tclspec"),
        "speclib mypack 2.0 {\n\
             environment mypack-shell {\n\
                 display_name {Mypack Shell}\n\
                 core         tcl 8.6\n\
             }\n\
         }\n",
    )
    .unwrap();

    for dialect in ["tcl-bpf", "mypack-shell"] {
        let (_stdout, stderr, code) = run_in(&dir, &["pkg", "discover", "--dialect", dialect]);
        assert_eq!(code, 0, "`--dialect {dialect}`: {stderr}");
    }
    let (_stdout, stderr, code) = run_in(&dir, &["pkg", "discover", "--dialect", "nonsense"]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("unknown dialect `nonsense`"), "{stderr}");
    std::fs::remove_dir_all(&dir).ok();
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
             set dep tls\n\
             set minimum 1.7\n\
             package require $dep $minimum\n\
         }\n\
         proc load_computed {} {\n\
             set computed [string cat constructed _dependency]\n\
             package require $computed 3.0\n\
         }\n\
         if {$optional} { package require Tk 8.6 }\n\
         set alternative 2.0-2.5\n\
         package require alternatives 1.0 $alternative\n\
         package require partial 1.0 $runtime_requirement\n\
         package require $runtime_package\n",
    )
    .unwrap();

    // Keep the optimiser assertion on literal value propagation. A
    // `package require` may load arbitrary commands, so command-derived
    // values such as `[string cat t l s]` correctly abstain once the shared
    // command-binding analysis sees this module's external-load surface.

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
            && requirement["resolution"] == "original-value"
            && requirement["status"] == "candidate"
    }));
    assert!(requirements.iter().any(|requirement| {
        requirement["name"] == "Tk"
            && requirement["conditional"] == true
            && requirement["status"] == "review"
    }));
    assert!(requirements.iter().any(|requirement| {
        requirement["name"] == "alternatives"
            && requirement["requirements"] == serde_json::json!(["1.0", "2.0-2.5"])
            && requirement["requirement_expressions"]
                == serde_json::json!(["1.0", "${alternative}"])
            && requirement["resolution"] == "original-value"
            && requirement["status"] == "review"
    }));
    assert!(requirements.iter().any(|requirement| {
        requirement["name"] == "partial"
            && requirement["requirements"] == serde_json::json!(["1.0", null])
            && requirement["requirement_expressions"]
                == serde_json::json!(["1.0", "${runtime_requirement}"])
            && requirement["resolution"] == "unresolved"
            && requirement["status"] == "review"
    }));
    assert!(requirements.iter().any(|requirement| {
        requirement["expression"] == "${runtime_package}"
            && requirement["name"].is_null()
            && requirement["status"] == "unresolved"
    }));
    assert!(requirements.iter().any(|requirement| {
        requirement["expression"] == "${computed}"
            && requirement["name"].is_null()
            && requirement["status"] == "unresolved"
    }));
    assert!(output["added"].as_array().unwrap().is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pkg_discover_add_retains_manifest_when_source_decoding_loses_a_name() {
    let dir = temp_dir("pkg-discover-lossy-name");
    let manifest = "package demo\nversion 1.0.0\nlicense MIT\n";
    let manifest_path = dir.join("tclpkg.tcl");
    std::fs::write(&manifest_path, manifest).unwrap();
    std::fs::write(dir.join("main.tcl"), b"package require \xff 1.2\n").unwrap();
    let (stdout, stderr, code) = run_in(&dir, &["pkg", "discover", "--add", "--json"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("decoding errors"), "{stderr}");
    assert!(stdout.is_empty());
    assert_eq!(std::fs::read_to_string(manifest_path).unwrap(), manifest);
    std::fs::remove_dir_all(dir).unwrap();
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
         package require alternatives 1.0 2.0-2.5\n\
         while {$optional} { package require looped 3.0 }\n\
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
    assert!(requirements.iter().any(|requirement| {
        requirement["name"] == "alternatives"
            && requirement["requirement_expressions"] == serde_json::json!(["1.0", "2.0-2.5"])
            && requirement["requirements"] == serde_json::json!(["1.0", "2.0-2.5"])
            && requirement["minimum"].is_null()
            && requirement["resolution"] == "literal"
            && requirement["status"] == "review"
    }));
    assert!(requirements.iter().any(|requirement| {
        requirement["name"] == "looped"
            && requirement["control_flow"] == true
            && requirement["status"] == "review"
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
    assert!(!manifest.contains("require alternatives"));
    assert!(!manifest.contains("require looped"));
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

/// A project whose specs declare a command host-native needs the extension
/// that provides it in the image: the generated Dockerfile lists it and
/// checks that it loads. A command that is not host-native lists nothing,
/// and one that names no providing package is said so, not guessed at.
#[test]
fn docker_create_lists_a_host_native_commands_extension() {
    let dir = temp_dir("docker-create-native");
    std::fs::create_dir_all(dir.join(".tcl-lsp")).unwrap();
    std::fs::write(
        dir.join(".tcl-lsp/xml.tclspec"),
        "speclib xml 2.0 {\n\
         \x20   provides tdom\n\
         \x20   command tdom::parse { arity 1; runtime_backing host-native }\n\
         \x20   command tdom::version { arity 0; runtime_backing none }\n\
         }\n",
    )
    .unwrap();
    std::fs::write(
        dir.join(".tcl-lsp/anon.tclspec"),
        "speclib anon 2.0 {\n\
         \x20   command anon::poke { arity 0; runtime_backing host-native }\n\
         }\n",
    )
    .unwrap();

    let (stdout, stderr, code) = run_in(&dir, &["docker", "create", "--no-packages"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("native extensions"), "{stdout}");
    assert!(stdout.contains("tdom"), "{stdout}");
    assert!(
        stderr.contains("anon::poke") && stderr.contains("names no providing package"),
        "{stderr}"
    );
    let content = std::fs::read_to_string(dir.join("Dockerfile")).unwrap();
    assert!(
        content.contains("foreach e {tdom} {if {[catch {package require $e} m]}"),
        "{content}"
    );
    assert!(!content.contains("anon"), "{content}");

    // The same listing, machine-readable.
    let (stdout, stderr, code) = run_in(
        &dir,
        &["docker", "create", "--no-packages", "--force", "--json"],
    );
    assert_eq!(code, 0, "{stderr}");
    let output: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(output["native_extensions"], serde_json::json!(["tdom"]));

    // Without the host-native declaration there is nothing to list.
    std::fs::remove_file(dir.join(".tcl-lsp/xml.tclspec")).unwrap();
    std::fs::remove_file(dir.join(".tcl-lsp/anon.tclspec")).unwrap();
    let (stdout, _stderr, code) = run_in(&dir, &["docker", "create", "--no-packages", "--force"]);
    assert_eq!(code, 0);
    assert!(!stdout.contains("native extensions"), "{stdout}");
    let content = std::fs::read_to_string(dir.join("Dockerfile")).unwrap();
    assert!(!content.contains("native extension"), "{content}");
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

/// A package that ships packs records their hash in the lockfile, and a changed
/// pack in an unchanged release is a lockfile change: the hash is the content
/// hash a compiled unit's claim on the pack carries, it moves when the pack
/// does, and it stays put when the pack does not.
#[test]
fn a_changed_pack_changes_the_lockfile() {
    let base = temp_dir("pkg-spec-integrity");
    let cache = base.join("cache");
    let dir = base.join("proj");
    std::fs::create_dir_all(&dir).unwrap();
    let dep = base.join("dep-src");
    std::fs::create_dir_all(&dep).unwrap();
    std::fs::write(
        dep.join("tclpkg.tcl"),
        "package dep\nversion 1.0.0\nlicense MIT\nspec {packs {dep.tclspec}}\n",
    )
    .unwrap();
    std::fs::write(dep.join("dep.tcl"), "proc dep::hi {} { return hi }\n").unwrap();
    let pack = "speclib dep 1 {\n    command dep::hi { arity 0 }\n}\n";
    std::fs::write(dep.join("dep.tclspec"), pack).unwrap();

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
    let spec_integrity = || {
        let lock = tcl_pkg::lockfile::read_lockfile(dir.join("tclpkg.lock")).unwrap();
        lock.lookup("dep").unwrap().spec_integrity.clone()
    };

    let (_o, e, code) = run_in_cache(&dir, Some(&cache), &["pkg", "install"]);
    assert_eq!(code, 0, "{e}");
    let first = spec_integrity().expect("the package ships a pack");
    // The one pack's hash is its content hash, written as the lockfile does.
    assert_eq!(
        first,
        tcl_pkg::lockfile::format_spec_integrity(&[tcl_spectcl::package_specs::pack_file_hash(
            &dep.join("dep.tclspec"),
            pack
        )])
    );
    let lock = std::fs::read_to_string(dir.join("tclpkg.lock")).unwrap();
    assert!(
        lock.contains(&format!("\"spec_integrity\": \"{first}\"")),
        "{lock}"
    );

    // An unchanged pack keeps the hash through a second install.
    let (_o, e, code) = run_in_cache(&dir, Some(&cache), &["pkg", "install"]);
    assert_eq!(code, 0, "{e}");
    assert_eq!(spec_integrity().as_deref(), Some(first.as_str()));

    // A changed pack, with the version and the manifest as they were, moves it.
    std::fs::write(
        dep.join("dep.tclspec"),
        "speclib dep 1 {\n    command dep::hi { arity 0..1 }\n}\n",
    )
    .unwrap();
    let (_o, e, code) = run_in_cache(&dir, Some(&cache), &["pkg", "install"]);
    assert_eq!(code, 0, "{e}");
    let changed = spec_integrity().expect("still ships a pack");
    assert_ne!(changed, first, "a changed pack must change the lockfile");

    // A package that names no packs records nothing.
    std::fs::write(
        dep.join("tclpkg.tcl"),
        "package dep\nversion 1.0.0\nlicense MIT\n",
    )
    .unwrap();
    let (_o, e, code) = run_in_cache(&dir, Some(&cache), &["pkg", "install"]);
    assert_eq!(code, 0, "{e}");
    assert_eq!(spec_integrity(), None);
    let lock = std::fs::read_to_string(dir.join("tclpkg.lock")).unwrap();
    assert!(!lock.contains("spec_integrity"), "{lock}");

    let _ = std::fs::remove_dir_all(&base);
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
    // The resolver's provider must surface transitive dependencies: `sub`
    // appears in the lockfile and `dep`'s `requires` records it.
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

#[cfg(unix)]
#[test]
#[allow(clippy::too_many_lines)]
fn direct_tcllibpath_producers_preserve_selected_original_c_tcl_lists_and_paths() {
    use std::os::unix::fs::symlink;
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let base = temp_dir("tcllibpath-native");
    let project = base.join("café space {braces} [literal];$value\\tail");
    let venv = project.join("selected venv {literal}");
    std::fs::create_dir_all(project.join("lib")).unwrap();
    std::fs::create_dir_all(venv.join("lib")).unwrap();
    std::fs::create_dir_all(venv.join("bin")).unwrap();
    std::fs::write(project.join("lib/sentinel"), "project").unwrap();
    std::fs::write(venv.join("lib/sentinel"), "venv").unwrap();
    std::fs::write(venv.join("tclvenv.cfg"), "venv_tool = tcl-lsp\n").unwrap();
    std::fs::write(
        project.join("tclpkg.tcl"),
        "package demo\nversion 1.0\nentry entry.tcl\n",
    )
    .unwrap();
    let script = r#"if {![string equal [info patchlevel] $env(EXPECTED_PATCHLEVEL)]} {error "wrong actual provider"}
if {![string equal [info library] $env(EXPECTED_LIBRARY)]} {error "wrong original library"}
set source $env(TCLLIBPATH)
set old $env(EXPECTED_SUFFIX)
if {![string equal $env(EXPECTED_ESCAPE) ""] && ![string equal [lindex $old 0] $env(EXPECTED_ESCAPE)]} {error "original release escape observation changed"}
if {![string equal [lindex $source 0] $env(EXPECTED_PATH)]} {error "directory identity changed"}
if {![file isfile [file join [lindex $source 0] sentinel]]} {error "actual directory unavailable"}
if {[llength $source] != [expr {[llength $old] + 1}]} {error "list cardinality changed"}
for {set i 0} {$i < [llength $old]} {incr i} {
    if {![string equal [lindex $source [expr {$i + 1}]] [lindex $old $i]]} {error "original suffix meaning changed"}
}
if {[string length $old] && ![string equal [string range $source end-[expr {[string length $old] - 1}] end] $old]} {error "original suffix source changed"}
set receipt [open $env(RESULT_PATH) w]
puts $receipt [list PASS [info patchlevel] [encoding system]]
close $receipt
"#;
    std::fs::write(project.join("entry.tcl"), script).unwrap();
    for version in tcl_dialect::TclVersion::ALL {
        let tree = tcl_test_support::locate_source_tree(&repo, version, None)
            .unwrap()
            .expect("required original pinned Tcl source tree");
        let interpreter = tcl_test_support::tclsh_from_source_tree(&tree, version)
            .expect("required matching original Tcl binary");
        let selected = venv.join("bin/tclsh");
        if selected.exists() {
            std::fs::remove_file(&selected).unwrap();
        }
        symlink(&interpreter.path, &selected).unwrap();
        for suffix in [
            None,
            Some(""),
            Some("  {old path} \t\n"),
            Some(r"old\x4142 {literal\n} trailing\ "),
        ] {
            for surface in ["pkg", "venv"] {
                let result = base.join("native-result");
                if result.exists() {
                    std::fs::remove_file(&result).unwrap();
                }
                let mut command = Command::new(env!("CARGO_BIN_EXE_tcl"));
                if surface == "pkg" {
                    command
                        .args(["pkg", "run", "--manifest"])
                        .arg(project.join("tclpkg.tcl"));
                } else {
                    command
                        .args(["venv", "run"])
                        .arg(&venv)
                        .arg("--")
                        .arg(&interpreter.path)
                        .arg(project.join("entry.tcl"));
                }
                command
                    .current_dir(&project)
                    .env("TCL_VENV", &venv)
                    .env("TCL_LIBRARY", tree.library_dir())
                    .env("LD_LIBRARY_PATH", tree.root.join("unix"))
                    .env("EXPECTED_PATCHLEVEL", &interpreter.patchlevel)
                    .env("EXPECTED_LIBRARY", tree.library_dir())
                    .env(
                        "EXPECTED_PATH",
                        if surface == "pkg" {
                            project.join("lib")
                        } else {
                            venv.join("lib")
                        },
                    )
                    .env("EXPECTED_SUFFIX", suffix.unwrap_or(""))
                    .env(
                        "EXPECTED_ESCAPE",
                        if suffix == Some(r"old\x4142 {literal\n} trailing\ ") {
                            if version <= tcl_dialect::TclVersion::V8_5 {
                                "oldB"
                            } else {
                                "oldA42"
                            }
                        } else {
                            ""
                        },
                    )
                    .env("RESULT_PATH", &result)
                    .env("XDG_CACHE_HOME", base.join("cache"));
                match suffix {
                    Some(value) => {
                        command.env("TCLLIBPATH", value);
                    }
                    None => {
                        command.env_remove("TCLLIBPATH");
                    }
                }
                let output = command.output().unwrap();
                assert!(
                    output.status.success(),
                    "{version:?}/{surface}/{suffix:?}: {output:?}"
                );
                assert!(
                    output.stderr.is_empty(),
                    "{version:?}/{surface}: {:?}",
                    output.stderr
                );
                let receipt = std::fs::read_to_string(&result).unwrap();
                assert!(
                    receipt.starts_with(&format!("PASS {} ", interpreter.patchlevel)),
                    "{receipt}"
                );
            }
        }
    }
    std::fs::remove_dir_all(base).unwrap();
}

#[cfg(unix)]
#[test]
fn direct_tcllibpath_producers_refuse_invalid_or_opaque_source_before_entry() {
    use std::os::unix::ffi::OsStringExt;
    use std::os::unix::fs::PermissionsExt;
    let base = temp_dir("tcllibpath-refusal");
    let project = base.join("project");
    let venv = base.join("venv");
    std::fs::create_dir_all(project.join("lib")).unwrap();
    std::fs::create_dir_all(venv.join("lib")).unwrap();
    std::fs::create_dir_all(venv.join("bin")).unwrap();
    std::fs::write(
        project.join("tclpkg.tcl"),
        "package demo\nversion 1.0\nentry entry.tcl\n",
    )
    .unwrap();
    std::fs::write(project.join("entry.tcl"), "exit 0\n").unwrap();
    std::fs::write(venv.join("tclvenv.cfg"), "venv_tool = tcl-lsp\n").unwrap();
    let executable = venv.join("bin/tclsh");
    std::fs::write(
        &executable,
        "#!/bin/sh\nprintf reached > \"$ENTRY_MARKER\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
    for inherited in [
        std::ffi::OsString::from("{unclosed"),
        std::ffi::OsString::from("{closed}junk"),
        std::ffi::OsString::from_vec(b"original-\xff".to_vec()),
    ] {
        for surface in ["pkg", "venv"] {
            let marker = base.join("entry-marker");
            let mut command = Command::new(env!("CARGO_BIN_EXE_tcl"));
            if surface == "pkg" {
                command
                    .args(["pkg", "run", "--manifest"])
                    .arg(project.join("tclpkg.tcl"));
            } else {
                command
                    .args(["venv", "run"])
                    .arg(&venv)
                    .arg("--")
                    .arg(&executable);
            }
            let output = command
                .current_dir(&project)
                .env("TCL_VENV", &venv)
                .env("TCLLIBPATH", &inherited)
                .env("ENTRY_MARKER", &marker)
                .env("XDG_CACHE_HOME", base.join("cache"))
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(1), "{surface}: {output:?}");
            assert!(
                String::from_utf8(output.stderr)
                    .unwrap()
                    .contains("TCLLIBPATH")
            );
            assert!(
                !marker.exists(),
                "{surface}: unrepresentable list source entered user code"
            );
        }
    }
    std::fs::remove_dir_all(base).unwrap();
}

#[cfg(unix)]
#[test]
#[allow(clippy::too_many_lines)]
fn generated_posix_venv_source_preserves_executable_arguments_and_activation_values() {
    use std::os::unix::fs::PermissionsExt;
    let base = temp_dir("venv-posix-source");
    let selected_dir = base.join("selected '$literal`quote` directory");
    std::fs::create_dir_all(&selected_dir).unwrap();
    let selected = selected_dir.join("tclsh9.0");
    let target = r#"#!/bin/sh
if [ "$1" = - ]; then printf '9.0.4\n'; exit 0; fi
printf '%s\000' "$0" "$@" "$TCL_VENV" "$TCLLIBPATH" > "$RECEIPT"
"#;
    std::fs::write(&selected, target).unwrap();
    std::fs::set_permissions(&selected, std::fs::Permissions::from_mode(0o755)).unwrap();
    let venv = base.join("venv '$literal`quote` {path}");
    let prompt = "prompt '$literal`quote`\";\nsecond line";
    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(["venv", "create"])
        .arg(&venv)
        .args(["--prompt", prompt])
        .env("PATH", selected_dir.file_name().unwrap())
        .env("XDG_CACHE_HOME", base.join("cache"))
        .current_dir(&base)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let receipt = base.join("wrapper-receipt");
    let arguments = ["one argument", "'$literal`quote`\";", "line\nend", ""];
    let output = Command::new("/bin/sh")
        .arg(venv.join("bin/tclsh"))
        .args(arguments)
        .env("RECEIPT", &receipt)
        .env("PATH", "/bin:/usr/bin")
        .env("TCLLIBPATH", "{original suffix}")
        .current_dir(&base)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let raw = std::fs::read(&receipt).unwrap();
    let values = raw
        .strip_suffix(&[0])
        .unwrap()
        .split(|byte| *byte == 0)
        .collect::<Vec<_>>();
    assert_eq!(values[0], selected.to_str().unwrap().as_bytes());
    for (actual, expected) in values[1..5].iter().zip(arguments) {
        assert_eq!(*actual, expected.as_bytes());
    }
    assert_eq!(values[5], venv.to_str().unwrap().as_bytes());
    let activation_receipt = base.join("activation-receipt");
    let driver = base.join("activation-driver.sh");
    std::fs::write(
        &driver,
        r#". "$ACTIVATION"
printf '%s\000' "$TCL_VENV" "$PATH" "$PS1" "$TCLLIBPATH" > "$RECEIPT"
deactivate
printf '%s\000' "$PATH" "$PS1" "$TCLLIBPATH" "${TCL_VENV+x}" >> "$RECEIPT"
"#,
    )
    .unwrap();
    let original_path = "before path:original tail";
    let original_prompt = "original '$prompt`value` ";
    let original_suffix = r"{original path} tail\ ";
    let output = Command::new("/bin/sh")
        .arg(&driver)
        .env("ACTIVATION", venv.join("bin/activate"))
        .env("RECEIPT", &activation_receipt)
        .env("PATH", original_path)
        .env("PS1", original_prompt)
        .env("TCLLIBPATH", original_suffix)
        .env_remove("TCL_VENV")
        .current_dir(&base)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let raw = std::fs::read(activation_receipt).unwrap();
    let values = raw
        .strip_suffix(&[0])
        .unwrap()
        .split(|byte| *byte == 0)
        .collect::<Vec<_>>();
    let expected_list = tcl_pkg::venv::prepare_tcllibpath(
        &venv.join("lib"),
        Some(std::ffi::OsStr::new(original_suffix)),
    )
    .unwrap();
    let expected_path = format!("{}:{original_path}", venv.join("bin").to_str().unwrap());
    let expected_prompt = format!("({prompt}) {original_prompt}");
    assert_eq!(
        values,
        [
            venv.to_str().unwrap().as_bytes(),
            expected_path.as_bytes(),
            expected_prompt.as_bytes(),
            expected_list.to_str().unwrap().as_bytes(),
            original_path.as_bytes(),
            original_prompt.as_bytes(),
            original_suffix.as_bytes(),
            b""
        ]
    );
    std::fs::remove_dir_all(base).unwrap();
}

#[cfg(unix)]
#[test]
fn generated_posix_venv_source_refuses_opaque_paths_before_writing() {
    use std::os::unix::ffi::OsStringExt;
    use std::os::unix::fs::PermissionsExt;
    let base = temp_dir("venv-posix-opaque");
    let selected_dir = base.join("selected");
    std::fs::create_dir_all(&selected_dir).unwrap();
    let selected = selected_dir.join("tclsh9.0");
    std::fs::write(&selected, "#!/bin/sh\nprintf '9.0.4\\n'\n").unwrap();
    std::fs::set_permissions(&selected, std::fs::Permissions::from_mode(0o755)).unwrap();
    let colon = base.join("venv:component");
    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(["venv", "create"])
        .arg(&colon)
        .env("PATH", &selected_dir)
        .env("XDG_CACHE_HOME", base.join("cache"))
        .current_dir(&base)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(!colon.exists());
    let opaque = base.join(std::ffi::OsString::from_vec(b"venv-\xff".to_vec()));
    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(["venv", "create"])
        .arg(&opaque)
        .args(["--prompt", "explicit"])
        .env("PATH", &selected_dir)
        .env("XDG_CACHE_HOME", base.join("cache"))
        .current_dir(&base)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(!opaque.exists());
    let opaque_selected = base.join(std::ffi::OsString::from_vec(b"selected-\xff".to_vec()));
    std::fs::create_dir_all(&opaque_selected).unwrap();
    std::fs::copy(&selected, opaque_selected.join("tclsh9.0")).unwrap();
    let venv = base.join("new-venv");
    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(["venv", "create"])
        .arg(&venv)
        .env("PATH", &opaque_selected)
        .env("XDG_CACHE_HOME", base.join("cache"))
        .current_dir(&base)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(!venv.exists());
    std::fs::create_dir_all(venv.join("bin")).unwrap();
    std::fs::write(
        venv.join("tclvenv.cfg"),
        "venv_tool = tcl-lsp\ntcl_executable = original\n",
    )
    .unwrap();
    std::fs::write(venv.join("bin/tclsh"), "original wrapper\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(["venv", "update"])
        .arg(&venv)
        .args(["--tcl", "9.0"])
        .env("PATH", &opaque_selected)
        .env("XDG_CACHE_HOME", base.join("cache"))
        .current_dir(&base)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(
        std::fs::read_to_string(venv.join("tclvenv.cfg")).unwrap(),
        "venv_tool = tcl-lsp\ntcl_executable = original\n"
    );
    assert_eq!(
        std::fs::read_to_string(venv.join("bin/tclsh")).unwrap(),
        "original wrapper\n"
    );
    std::fs::remove_dir_all(base).unwrap();
}

#[cfg(unix)]
#[test]
fn venv_config_publication_preserves_original_paths_values_and_legacy_rows() {
    use std::os::unix::fs::PermissionsExt;
    let base = temp_dir("venv-config-source");
    let selected_dir = base.join("selected \"quote\"\nnext=path");
    std::fs::create_dir_all(&selected_dir).unwrap();
    let executable = selected_dir.join("tclsh9.0");
    std::fs::write(&executable, "#!/bin/sh\nprintf '9.0.4\\n'\n").unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
    let venv = base.join("venv");
    let prompt = "literal \"quotes\"\\path\nnext = not-a-key";
    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(["venv", "create"])
        .arg(&venv)
        .args(["--prompt", prompt])
        .env("PATH", &selected_dir)
        .env("XDG_CACHE_HOME", base.join("cache"))
        .current_dir(&base)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let pairs = tcl_pkg::venv::read_venv_config(&venv).unwrap();
    assert!(pairs.contains(&("prompt".into(), prompt.into())));
    assert!(pairs.contains(&("tcl_executable".into(), executable.to_str().unwrap().into())));
    assert!(!pairs.iter().any(|(key, _)| key == "next"));
    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(["venv", "info"])
        .arg(&venv)
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["prompt"], prompt);
    let legacy = "tcl_version = 8.6\ntcl_executable = original\ntcl_version_extra = retain-version\ntcl_executable_backup = retain-path\nunknown = first\nunknown = second\nprompt = \"literal\"\n";
    std::fs::write(venv.join("tclvenv.cfg"), legacy).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(["venv", "update"])
        .arg(&venv)
        .args(["--tcl", "9.0"])
        .env("PATH", &selected_dir)
        .env("XDG_CACHE_HOME", base.join("cache"))
        .current_dir(&base)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        tcl_pkg::venv::read_venv_config(&venv).unwrap(),
        vec![
            ("tcl_version".into(), "9.0".into()),
            ("tcl_executable".into(), executable.to_str().unwrap().into()),
            ("tcl_version_extra".into(), "retain-version".into()),
            ("tcl_executable_backup".into(), "retain-path".into()),
            ("unknown".into(), "first".into()),
            ("unknown".into(), "second".into()),
            ("prompt".into(), "\"literal\"".into()),
        ]
    );
    let wrapper = std::fs::read(venv.join("bin/tclsh")).unwrap();
    for invalid in [
        "# tclpkg venv config TOML 1\nbroken source\n",
        "# tclpkg venv config TOML 2\ntcl_version = legacy-fallback-must-not-run\n",
        "tcl_version = one\ntcl_version = two\n",
        "prompt = one\nprompt = two\n",
    ] {
        std::fs::write(venv.join("tclvenv.cfg"), invalid).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
            .args(["venv", "update"])
            .arg(&venv)
            .args(["--tcl", "9.0"])
            .env("PATH", &selected_dir)
            .env("XDG_CACHE_HOME", base.join("cache"))
            .current_dir(&base)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert_eq!(
            std::fs::read_to_string(venv.join("tclvenv.cfg")).unwrap(),
            invalid
        );
        assert_eq!(std::fs::read(venv.join("bin/tclsh")).unwrap(), wrapper);
    }
    std::fs::remove_dir_all(base).unwrap();
}

#[test]
fn venv_deletion_requires_successful_actual_active_path_identity() {
    let base = temp_dir("venv-delete-identity");
    let target = base.join("selected");
    let distinct = base.join("distinct");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::create_dir_all(&distinct).unwrap();
    let marker = b"venv_tool = tcl-lsp\n";
    std::fs::write(target.join("tclvenv.cfg"), marker).unwrap();
    let delete = |active: &std::path::Path, force: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_tcl"));
        command
            .current_dir(&base)
            .args(["venv", "delete"])
            .arg(&target)
            .env("TCL_VENV", active);
        if force {
            command.arg("--force");
        }
        command.output().unwrap()
    };

    let output = delete(&base.join("missing-active"), false);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("cannot establish active venv path identity")
    );
    assert_eq!(std::fs::read(target.join("tclvenv.cfg")).unwrap(), marker);

    let output = delete(&target, false);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("cannot delete the currently active venv")
    );
    assert_eq!(std::fs::read(target.join("tclvenv.cfg")).unwrap(), marker);

    #[cfg(unix)]
    {
        let alias = base.join("active-alias");
        std::os::unix::fs::symlink(&target, &alias).unwrap();
        let output = delete(&alias, false);
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("cannot delete the currently active venv")
        );
        assert_eq!(std::fs::read(target.join("tclvenv.cfg")).unwrap(), marker);
    }

    let output = delete(&distinct, false);
    assert!(output.status.success(), "{:?}", output.stderr);
    assert!(!target.exists());
    assert!(distinct.is_dir());

    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("tclvenv.cfg"), marker).unwrap();
    let output = delete(&base.join("missing-active"), true);
    assert!(output.status.success(), "{:?}", output.stderr);
    assert!(!target.exists());
    std::fs::remove_dir_all(base).unwrap();
}

#[test]
fn docker_cmd_source_preserves_complete_original_exec_argv() {
    let dir = temp_dir("docker-cmd-argv");
    for entry in [
        "main.tcl",
        "literal\"], \"other\"]\nRUN injected\n#",
        "space [literal] $VALUE \\ café\r\nscript.tcl",
        "",
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
            .current_dir(&dir)
            .args([
                "docker",
                "create",
                "--no-packages",
                "--force",
                "--entrypoint",
                entry,
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "{:?}", output.stderr);
        let source = std::fs::read_to_string(dir.join("Dockerfile")).unwrap();
        let instructions = source
            .lines()
            .filter_map(|line| line.strip_prefix("CMD "))
            .collect::<Vec<_>>();
        assert_eq!(instructions.len(), 1);
        let argv: Vec<String> = serde_json::from_str(instructions[0]).unwrap();
        assert_eq!(argv, ["tclsh", entry]);
        assert!(!source.lines().any(|line| line == "RUN injected"));
        if entry == "main.tcl" {
            assert!(source.trim_end().ends_with("CMD [\"tclsh\", \"main.tcl\"]"));
        }
    }

    let path = dir.join("Dockerfile");
    let before = std::fs::read(&path).unwrap();
    let invalid = tcl_pkg::docker::DockerfileSpec {
        install_packages: false,
        entrypoint: Some("bad\0argv".into()),
        ..Default::default()
    };
    assert!(tcl_pkg::docker::write_dockerfile(&path, &invalid, true).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    std::fs::remove_dir_all(dir).unwrap();
}
