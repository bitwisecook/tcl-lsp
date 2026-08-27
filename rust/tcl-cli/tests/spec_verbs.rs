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

/// `tcl spec upgrade` rewrites the 1.x `dialects` vocabulary in place,
/// moves the `speclib` word, and leaves every other byte alone.
#[test]
fn spec_upgrade_translates_dialects_rows_and_moves_the_version_word() {
    let tree = Tree::new("upgrade");
    let pack = tree.path().join("demo.tclspec");
    let source = "# a comment the rewriter must not touch\n\
                  speclib demo 1.2 {\n\
                  \x20   command demo::greet {\n\
                  \x20       arity 1\n\
                  \x20       dialects {tcl8.6+ tk}\n\
                  \x20   }\n\
                  }\n";
    std::fs::write(&pack, source).expect("write pack");

    let path = pack.to_string_lossy().into_owned();
    let (stdout, _stderr, code) = run(&["spec", "upgrade", "--check", &path]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("would translate 1 row(s)"), "{stdout}");
    assert_eq!(
        std::fs::read_to_string(&pack).expect("read back"),
        source,
        "--check writes nothing"
    );

    let (stdout, _stderr, code) = run(&["spec", "upgrade", &path]);
    assert_eq!(code, 0, "{stdout}");
    let upgraded = std::fs::read_to_string(&pack).expect("read back");
    assert_eq!(
        upgraded,
        "# a comment the rewriter must not touch\n\
         speclib demo 2.0 {\n\
         \x20   command demo::greet {\n\
         \x20       arity 1\n\
         \x20       available {tcl 8.6-} {package Tk}\n\
         \x20   }\n\
         }\n"
    );
}

/// `--verify` proves the rewrite is behaviour-preserving (U9) and never
/// writes; `--to` older than `--from` is refused (U10).
#[test]
fn spec_upgrade_verifies_and_refuses_downgrades() {
    let tree = Tree::new("upgrade-verify");
    let pack = tree.path().join("demo.tclspec");
    let source = "speclib demo 1.2 {\n\
                  \x20 command demo::greet {\n\
                  \x20   arity 1\n\
                  \x20   dialects tcl8.x\n\
                  \x20 }\n\
                  }\n";
    std::fs::write(&pack, source).expect("write pack");
    let path = pack.to_string_lossy().into_owned();

    let (stdout, stderr, code) = run(&["spec", "upgrade", "--verify", &path]);
    assert_eq!(code, 0, "{stdout}{stderr}");
    assert!(stdout.contains("byte-identical"), "{stdout}");
    assert_eq!(
        std::fs::read_to_string(&pack).expect("read back"),
        source,
        "--verify writes nothing"
    );

    let (_stdout, stderr, code) = run(&["spec", "upgrade", "--from", "2.0", "--to", "1.2", &path]);
    assert_ne!(code, 0, "a downgrade must not succeed");
    assert!(stderr.contains("refusing to downgrade"), "{stderr}");
}

/// An environment-membership token whose environment declares **no**
/// ambient package provider (`spectcl` — its surface is compiled) is
/// left byte-identical, marked, and the file reports partial; a token
/// whose environment does declare one (`f5-iapps`) translates for real
/// through the live registry (upgrade spec U3, landed with P2-H — this
/// test previously pinned the pre-U3 all-deferred behaviour and had gone
/// stale against `tcl-spectcl`'s own U3 gates).
#[test]
fn spec_upgrade_defers_environment_membership_tokens() {
    let tree = Tree::new("upgrade-partial");
    let pack = tree.path().join("demo.tclspec");
    std::fs::write(
        &pack,
        "speclib demo 1.2 {\n command demo::greet {\n arity 1\n \
         dialects {tcl8.6 spectcl}\n }\n}\n",
    )
    .expect("write pack");
    let path = pack.to_string_lossy().into_owned();

    let (stdout, _stderr, code) = run(&["spec", "upgrade", &path]);
    assert_ne!(code, 0, "a partial upgrade exits non-zero: {stdout}");
    assert!(stdout.contains("partially upgraded"), "{stdout}");
    let written = std::fs::read_to_string(&pack).expect("read back");
    assert!(written.contains("# TODO(spectcl 2.0):"), "{written}");
    assert!(written.contains("speclib demo 1.2"), "{written}");
    assert!(written.contains("dialects {tcl8.6 spectcl}"), "{written}");

    // The ambient-provider half really translates (U3): the row becomes
    // the environment's own package claim and the header moves to 2.0.
    let full = tree.path().join("full.tclspec");
    std::fs::write(
        &full,
        "speclib demo 1.2 {\n command demo::greet {\n arity 1\n \
         dialects {tcl8.6 f5-iapps}\n }\n}\n",
    )
    .expect("write pack");
    let path = full.to_string_lossy().into_owned();
    let (stdout, stderr, code) = run(&["spec", "upgrade", &path]);
    assert_eq!(code, 0, "{stdout}{stderr}");
    let written = std::fs::read_to_string(&full).expect("read back");
    assert!(
        written.contains("available {tcl 8.6} {package f5-iapps-cmds}"),
        "{written}"
    );
    assert!(written.contains("speclib demo 2.0"), "{written}");
}

// ---------------------------------------------------------------------------
// `tcl spec export` — the canonical renderer (design E §15.1, E-R11)
// ---------------------------------------------------------------------------

/// A templated pack — the shape `spec export` exists for.
const PROGRAM: &str = "speclib fleet 2.0 {
    proc fleet-command {name arity} {
        command math::fleet::$name {
            arity $arity
            traits {PURE}
            option -verbose -detail {Report each step as it runs.}

            subcommand probe {
                arity 0
                detail {Probe one input.}
            }
        }
    }

    foreach {name arity} {alpha 2 beta 1 gamma 3} {
        fleet-command $name $arity
    }
}
";

#[test]
fn spec_export_expands_a_programmed_pack_into_canonical_source() {
    let tree = Tree::new("export");
    let pack = tree.path().join("fleet.tclspec");
    std::fs::write(&pack, PROGRAM).expect("write pack");
    let path = pack.to_string_lossy().into_owned();

    let (stdout, stderr, code) = run(&["spec", "export", &path]);
    assert_eq!(code, 0, "stderr: {stderr}");

    // One literal declaration per iteration, and none of the program.
    for name in ["alpha", "beta", "gamma"] {
        assert!(
            stdout.contains(&format!("command math::fleet::{name} {{")),
            "{stdout}"
        );
    }
    for word in ["proc ", "foreach ", "$name", "$arity"] {
        assert!(!stdout.contains(word), "{stdout}");
    }
    // The loop's data is in place, per iteration.
    assert!(
        stdout.contains("arity 2") && stdout.contains("arity 3"),
        "{stdout}"
    );
    // The pack's own vocabulary word survives: raising it is `spec upgrade`.
    assert!(stdout.contains("speclib fleet 2.0 {"), "{stdout}");

    // And the expansion is a pack: it reloads to the same snapshot, through
    // the CST loader, which cannot evaluate anything.
    let reloaded = tcl_spectcl::load_pack(&stdout);
    assert!(reloaded.load_error.is_none(), "{:#?}", reloaded.notices);
    let names: Vec<&str> = reloaded.commands.iter().map(|c| c.spec.name).collect();
    assert_eq!(
        names,
        vec![
            "math::fleet::alpha",
            "math::fleet::beta",
            "math::fleet::gamma"
        ]
    );
}

#[test]
fn spec_export_round_trips_a_canonical_pack_through_the_shared_formatter() {
    // Every shipped example pack: export, reload, and compare the snapshot
    // the registry would see. This is the CLI-side half of the E-R11 gate —
    // the part that also passes the text through `format_pack`.
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/design/spec-dsl-examples");
    let mut packs = 0;
    for entry in std::fs::read_dir(&dir).expect("the spec-dsl-examples directory") {
        let path = entry.expect("a directory entry").path();
        if path.extension().is_none_or(|ext| ext != "tclspec") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("readable pack");
        let (stdout, stderr, code) = run(&["spec", "export", &path.to_string_lossy()]);
        assert_eq!(code, 0, "{}: {stderr}", path.display());

        let before = tcl_spectcl::load_pack(&source);
        let after = tcl_spectcl::load_pack(&stdout);
        let render = |pack: &tcl_spectcl::Pack| {
            pack.commands
                .iter()
                .map(|c| format!("{:?}", c.spec))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            render(&before),
            render(&after),
            "{}: the formatted export is not the same snapshot",
            path.display()
        );
        packs += 1;
    }
    assert!(packs >= 8, "only {packs} example packs exported");
}

#[test]
fn spec_export_json_reports_the_expansion_and_its_notices() {
    let tree = Tree::new("export-json");
    let pack = tree.path().join("fleet.tclspec");
    std::fs::write(&pack, PROGRAM).expect("write pack");

    let (stdout, stderr, code) = run(&["spec", "export", &pack.to_string_lossy(), "--json"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("JSON");
    assert_eq!(value["pack"], serde_json::json!("fleet"));
    assert_eq!(value["commands"], serde_json::json!(3));
    assert_eq!(value["target_dependent"], serde_json::json!(false));
    assert!(
        value["canonical_source"]
            .as_str()
            .expect("canonical source")
            .contains("command math::fleet::alpha {"),
        "{stdout}"
    );
    assert!(value["notices"].is_array(), "{stdout}");
}

#[test]
fn spec_export_reports_a_pack_whose_evaluation_failed() {
    let tree = Tree::new("export-denied");
    let pack = tree.path().join("clocky.tclspec");
    std::fs::write(
        &pack,
        "speclib clocky 2.0 {\n    set now [clock seconds]\n    command demo { arity 1 }\n}\n",
    )
    .expect("write pack");

    let (_stdout, stderr, code) = run(&["spec", "export", &pack.to_string_lossy()]);
    assert_eq!(code, 1, "a failed evaluation exits non-zero: {stderr}");
    assert!(stderr.contains("determinism axis"), "{stderr}");
}
