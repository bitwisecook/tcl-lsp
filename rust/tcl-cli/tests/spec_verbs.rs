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

//! Integration tests for the `spec` verb group.
//!
//! These drive the built `tcl` binary against fixtures built in a temporary
//! directory at test time — a two-release fake package, one release as a
//! directory and one as a `.zip`. **No test here touches the network**: the
//! `--github` mode's units (tag mapping, glob filtering, URL construction) are
//! tested in `src/commands/spec.rs`, and everything downstream of "sources in
//! hand" is exercised through `--snapshot`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

/// v1 of the fake package: one command.
const V1: &str = "package provide demo 1.0

# Greet someone by name.
proc demo::greet {name} {
    return \"hello $name\"
}
";

/// v2: `greet` unchanged, `farewell` added.
const V2: &str = "package provide demo 2.0

# Greet someone by name.
proc demo::greet {name} {
    return \"hello $name\"
}

# Say goodbye.
proc demo::farewell {name {punctuation !}} {
    return \"bye $name$punctuation\"
}
";

/// A temporary directory removed when the guard drops.
struct Tree(PathBuf);

impl Tree {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("tcl-spec-verbs-{tag}-{nanos}"));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    /// One release as an unpacked directory.
    fn release_dir(&self, version: &str, text: &str) -> PathBuf {
        let dir = self.0.join(version);
        std::fs::create_dir_all(&dir).expect("release dir");
        std::fs::write(dir.join("demo.tcl"), text).expect("write source");
        // A non-Tcl file that must be ignored, and a dot-directory that must
        // not be walked.
        std::fs::write(dir.join("README.md"), "# demo\n").expect("write readme");
        dir
    }

    /// One release as a `.zip`, with the sources under a top-level directory
    /// the way a real release archive ships them.
    fn release_zip(&self, version: &str, text: &str) -> PathBuf {
        let path = self.0.join(format!("demo-{version}.zip"));
        let file = std::fs::File::create(&path).expect("create zip");
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file(format!("demo-{version}/demo.tcl"), options)
            .expect("zip entry");
        zip.write_all(text.as_bytes()).expect("zip write");
        zip.finish().expect("finish zip");
        path
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Run `tcl spec import …`, returning `(stdout, stderr, exit code)`.
fn run(args: &[&str]) -> (String, String, i32) {
    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(args)
        .output()
        .expect("failed to spawn tcl binary");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code().unwrap_or(-1),
    )
}

/// The body of one `command NAME { … }` block of a rendered pack.
fn command_block<'a>(pack: &'a str, name: &str) -> &'a str {
    let start = pack
        .find(&format!("command {name} {{"))
        .unwrap_or_else(|| panic!("no `command {name}` in:\n{pack}"));
    let rest = &pack[start..];
    let end = rest.find("\n}\n").unwrap_or(rest.len());
    &rest[..end]
}

#[test]
fn a_command_added_in_the_second_release_is_introduced_there() {
    let tree = Tree::new("added");
    let v1 = tree.release_dir("1.0", V1);
    let v2 = tree.release_dir("2.0", V2);

    let (pack, stderr, code) = run(&[
        "spec",
        "import",
        "--snapshot",
        &format!("1.0={}", v1.display()),
        "--snapshot",
        &format!("2.0={}", v2.display()),
    ]);
    assert_eq!(code, 0, "stderr: {stderr}");

    assert!(
        command_block(&pack, "demo::farewell").contains("introduced_version 2.0"),
        "{pack}"
    );
    // `greet` is in the oldest snapshot, and the history was not declared
    // complete, so its introduction is not derivable and must not be invented.
    assert!(
        !command_block(&pack, "demo::greet").contains("introduced_version"),
        "{pack}"
    );
    // The evidence travels with the pack.
    assert!(
        pack.starts_with("# Derived by `tcl spec import` from 2 release snapshot(s)"),
        "{pack}"
    );
    assert!(
        pack.contains("# Releases, oldest first: 1.0, 2.0"),
        "{pack}"
    );
    // The pack declares the renderer's own vocabulary version, whatever it is
    // today — hardcoding it here broke once already when 1.0 became 1.1.
    let speclib_line = format!(
        "speclib demo {} {{",
        tcl_spec_studio::render_spectcl::DSL_VERSION
    );
    assert!(pack.contains(&speclib_line), "{pack}");
    // The human summary goes to stderr, never into the pack on stdout.
    assert!(
        stderr.contains("2 command(s) from 2 local release(s)"),
        "{stderr}"
    );
}

#[test]
fn a_zip_snapshot_is_read_like_a_directory_one() {
    let tree = Tree::new("zip");
    let v1 = tree.release_zip("1.0", V1);
    let v2 = tree.release_dir("2.0", V2);

    let (pack, stderr, code) = run(&[
        "spec",
        "import",
        "--snapshot",
        &format!("1.0={}", v1.display()),
        "--snapshot",
        &format!("2.0={}", v2.display()),
    ]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        command_block(&pack, "demo::farewell").contains("introduced_version 2.0"),
        "{pack}"
    );
}

#[test]
fn a_complete_history_pins_the_oldest_release_as_the_introduction() {
    let tree = Tree::new("complete");
    let v1 = tree.release_dir("1.0", V1);
    let v2 = tree.release_dir("2.0", V2);

    let (pack, stderr, code) = run(&[
        "spec",
        "import",
        "--complete-history",
        "--snapshot",
        &format!("2.0={}", v2.display()),
        "--snapshot",
        &format!("1.0={}", v1.display()),
    ]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        command_block(&pack, "demo::greet").contains("introduced_version 1.0"),
        "{pack}"
    );
    // Snapshots given newest first are re-ordered, and said so.
    assert!(
        pack.contains("# Releases, oldest first: 1.0, 2.0"),
        "{pack}"
    );
    assert!(stderr.contains("ascending version order"), "{stderr}");
}

#[test]
fn json_mode_reports_the_pack_and_the_per_command_ranges() {
    let tree = Tree::new("json");
    let v1 = tree.release_dir("1.0", V1);
    let v2 = tree.release_dir("2.0", V2);

    let (stdout, stderr, code) = run(&[
        "spec",
        "import",
        "--json",
        "--package",
        "demolib",
        "--snapshot",
        &format!("1.0={}", v1.display()),
        "--snapshot",
        &format!("2.0={}", v2.display()),
    ]);
    assert_eq!(code, 0, "stderr: {stderr}");

    let value: serde_json::Value =
        serde_json::from_str(&stdout).expect("spec import --json must emit valid JSON");
    assert_eq!(value["package"], "demolib");
    assert_eq!(value["versions"], serde_json::json!(["1.0", "2.0"]));
    assert!(
        value["pack"]
            .as_str()
            .is_some_and(|p| p.contains("speclib demolib")),
        "{value}"
    );
    let farewell = value["commands"]
        .as_array()
        .expect("commands")
        .iter()
        .find(|c| c["name"] == "demo::farewell")
        .expect("demo::farewell");
    assert_eq!(farewell["introduced_version"], "2.0");
    assert_eq!(farewell["retired_version"], serde_json::Value::Null);
    assert!(
        farewell["notes"].as_array().is_some_and(|n| !n.is_empty()),
        "every derived field carries its evidence: {farewell}"
    );
}

#[test]
fn a_retired_command_carries_its_exclusive_bound() {
    let tree = Tree::new("retired");
    // v2 is the *older* shape here: the command it adds is gone again in 3.0.
    let v1 = tree.release_dir("2.0", V2);
    let v2 = tree.release_dir("3.0", V1);

    let (pack, stderr, code) = run(&[
        "spec",
        "import",
        "--snapshot",
        &format!("2.0={}", v1.display()),
        "--snapshot",
        &format!("3.0={}", v2.display()),
    ]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        command_block(&pack, "demo::farewell").contains("retired_version 3.0"),
        "{pack}"
    );
    assert!(stderr.contains("1 retired"), "{stderr}");
}

#[test]
fn the_pack_can_be_written_to_a_file() {
    let tree = Tree::new("out");
    let v1 = tree.release_dir("1.0", V1);
    let v2 = tree.release_dir("2.0", V2);
    let out = tree.path().join("demo.tclspec");

    let (stdout, stderr, code) = run(&[
        "spec",
        "import",
        "--out",
        &out.display().to_string(),
        "--snapshot",
        &format!("1.0={}", v1.display()),
        "--snapshot",
        &format!("2.0={}", v2.display()),
    ]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.is_empty(), "nothing should reach stdout: {stdout}");
    let written = std::fs::read_to_string(&out).expect("pack file");
    assert!(written.contains("introduced_version 2.0"), "{written}");
}

#[test]
fn a_malformed_snapshot_argument_is_refused() {
    let (_, stderr, code) = run(&["spec", "import", "--snapshot", "./releases/1.2"]);
    assert_eq!(code, 2, "stderr: {stderr}");
    assert!(stderr.contains("VERSION=PATH"), "{stderr}");
}

#[test]
fn a_snapshot_that_is_not_there_names_its_release() {
    let (_, stderr, code) = run(&["spec", "import", "--snapshot", "1.2=/definitely/not/here"]);
    assert_eq!(code, 2, "stderr: {stderr}");
    assert!(stderr.contains("snapshot 1.2"), "{stderr}");
}

#[test]
fn no_source_at_all_says_what_to_pass() {
    let (_, stderr, code) = run(&["spec", "import"]);
    assert_eq!(code, 2, "stderr: {stderr}");
    assert!(
        stderr.contains("--snapshot") && stderr.contains("--github"),
        "{stderr}"
    );
}

/// The network-only flags cannot be smuggled into a local import.
#[test]
fn the_github_flags_require_github() {
    for args in [
        vec!["spec", "import", "--tag-pattern", "v*"],
        vec!["spec", "import", "--limit", "3"],
        vec!["spec", "import", "--list-tags"],
    ] {
        let (_, stderr, code) = run(&args);
        assert_ne!(code, 0, "{args:?} should not run without --github");
        assert!(stderr.contains("--github"), "{args:?}: {stderr}");
    }
}
