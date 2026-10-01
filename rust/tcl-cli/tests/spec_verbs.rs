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

/// `--restyle` (D13) re-emits the upgraded pack in canonical form; with
/// `--check` it only says so.
#[test]
fn spec_upgrade_restyle_emits_canonical_form() {
    let tree = Tree::new("upgrade-restyle");
    let pack = tree.path().join("demo.tclspec");
    let source = "# a comment the restyle drops\n\
                  speclib demo 1.2 {\n\
                  \x20   command demo::greet {\n\
                  \x20           arity 1 ;# odd indent\n\
                  \x20       dialects {tcl8.6+ tk}\n\
                  \x20   }\n\
                  }\n";
    std::fs::write(&pack, source).expect("write pack");

    let path = pack.to_string_lossy().into_owned();
    let (stdout, _stderr, code) = run(&["spec", "upgrade", "--check", "--restyle", &path]);
    assert_eq!(code, 0, "{stdout}");
    assert!(
        stdout.contains("would re-emit in canonical form"),
        "{stdout}"
    );
    assert_eq!(
        std::fs::read_to_string(&pack).expect("read back"),
        source,
        "--check writes nothing"
    );

    let (stdout, _stderr, code) = run(&["spec", "upgrade", "--restyle", &path]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("re-emitted in canonical form"), "{stdout}");
    assert_eq!(
        std::fs::read_to_string(&pack).expect("read back"),
        "speclib demo 2.0 {\n\
         \n\
         command demo::greet {\n\
         \x20   arity 1\n\
         \x20   available {tcl 8.6-} {package Tk}\n\
         }\n\
         \n\
         }\n"
    );

    // A programmed pack is refused whole (E-R12), and the file is untouched.
    let programmed = "speclib demo 2.0 {\n\
                      foreach name {a b} {\n\
                      \x20   command $name { arity 0 }\n\
                      }\n\
                      }\n";
    std::fs::write(&pack, programmed).expect("write pack");
    let (_stdout, stderr, code) = run(&["spec", "upgrade", "--restyle", &path]);
    assert_ne!(code, 0);
    assert!(stderr.contains("E-R12"), "{stderr}");
    assert_eq!(
        std::fs::read_to_string(&pack).expect("read back"),
        programmed
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
/// through the live registry.
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

// `tcl spec export` — the canonical renderer.

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

    // And the expansion is a pack: it reloads to the same snapshot with no
    // templating left to run.
    let reloaded = tcl_spectcl::evaluate_pack(&stdout);
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

        let before = tcl_spectcl::evaluate_pack(&source);
        let after = tcl_spectcl::evaluate_pack(&stdout);
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

/// Where a pack sits relative to the project `tcl spec test` is run in.
#[derive(Clone, Copy)]
enum Layout {
    /// In the project directory, which has no manifest.
    Flat,
    /// In a `specs` directory below the project's manifest.
    Nested,
    /// In a dependency vendored into the project, which has a manifest and a
    /// policy of its own.
    Vendored,
}

/// A Tcl package on disk, a pack that describes it and the project policy that
/// opts the package in, for `tcl spec test`.
struct Described {
    tree: Tree,
    /// The operator's project, which the verb is run in.
    project: PathBuf,
    pack: PathBuf,
    library: PathBuf,
}

impl Described {
    /// `package` is the Tcl source of the `demo` package, `pack` the pack that
    /// describes it, and `trusted` the packages the project's policy opts in.
    fn new(tag: &str, package: &str, pack: &str, trusted: &[&str]) -> Self {
        Self::build(tag, package, pack, trusted, Layout::Flat)
    }

    /// The same, with the pack in a `specs` directory below a project that has a
    /// manifest and holds the policy: the project is not the pack's directory.
    fn nested(tag: &str, package: &str, pack: &str, trusted: &[&str]) -> Self {
        Self::build(tag, package, pack, trusted, Layout::Nested)
    }

    /// The same, with the pack in a dependency vendored into the project. `trusted`
    /// is the dependency's own policy, and the project's says nothing until
    /// [`Self::trust_in_the_project`].
    fn vendored(tag: &str, package: &str, pack: &str, trusted: &[&str]) -> Self {
        Self::build(tag, package, pack, trusted, Layout::Vendored)
    }

    /// Write `tclpkg.toml` in `dir`, opting the named packages in.
    fn write_policy(dir: &Path, trusted: &[&str]) {
        let names: Vec<String> = trusted.iter().map(|name| format!("{name:?}")).collect();
        std::fs::write(
            dir.join("tclpkg.toml"),
            format!(
                "[build]\nallow-build-scripts = true\ntrusted = [{}]\n",
                names.join(", ")
            ),
        )
        .expect("write the policy");
    }

    /// The operator's own policy opts the named packages in.
    fn trust_in_the_project(&self, trusted: &[&str]) {
        Self::write_policy(&self.project, trusted);
    }

    fn build(tag: &str, package: &str, pack: &str, trusted: &[&str], layout: Layout) -> Self {
        let tree = Tree::new(tag);
        let library = tree.path().join("lib");
        let package_dir = library.join("demo");
        std::fs::create_dir_all(&package_dir).expect("package dir");
        std::fs::write(
            package_dir.join("pkgIndex.tcl"),
            "package ifneeded demo 1.0 [list source [file join $dir demo.tcl]]\n",
        )
        .expect("write the index");
        std::fs::write(package_dir.join("demo.tcl"), package).expect("write the package");
        let project_dir = tree.path().join("project");
        std::fs::create_dir_all(&project_dir).expect("project dir");
        let (pack_dir, policy_dir) = match layout {
            Layout::Flat => (project_dir.clone(), project_dir.clone()),
            Layout::Nested => {
                std::fs::write(project_dir.join("tclpkg.tcl"), "package demo 1.0\n")
                    .expect("write the manifest");
                (project_dir.join("specs"), project_dir.clone())
            }
            Layout::Vendored => {
                std::fs::write(project_dir.join("tclpkg.tcl"), "package app 1.0\n")
                    .expect("write the project's manifest");
                let dependency = project_dir.join("vendor").join("demo");
                std::fs::create_dir_all(&dependency).expect("dependency dir");
                std::fs::write(dependency.join("tclpkg.tcl"), "package demo 1.0\n")
                    .expect("write the dependency's manifest");
                (dependency.join("specs"), dependency)
            }
        };
        std::fs::create_dir_all(&pack_dir).expect("pack dir");
        let pack_path = pack_dir.join("demo.tclspec");
        std::fs::write(&pack_path, pack).expect("write the pack");
        if !trusted.is_empty() {
            Self::write_policy(&policy_dir, trusted);
        }
        Self {
            tree,
            project: project_dir,
            pack: pack_path,
            library,
        }
    }

    /// `tcl spec test` on the pack, with the package's directory named and every
    /// per-user directory pointed into the tree, and no shell named.
    fn command(&self) -> Command {
        let home = self.tree.path().join("home");
        std::fs::create_dir_all(&home).expect("home");
        let mut command = Command::new(env!("CARGO_BIN_EXE_tcl"));
        command
            .current_dir(&self.project)
            .args(["spec", "test", &self.pack.to_string_lossy()])
            .env("TCLLIBPATH", &self.library)
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", home.join("config"))
            .env("XDG_STATE_HOME", home.join("state"))
            .env("XDG_CACHE_HOME", home.join("cache"))
            .env_remove("TCL_VENV");
        command
    }

    /// The same, with the shell named.
    fn run(&self, tclsh: &Path) -> (String, String, i32) {
        let mut command = self.command();
        command.arg("--tclsh").arg(tclsh);
        finished(&mut command)
    }
}

/// Run `command` to its end: its standard output and error, and its status.
fn finished(command: &mut Command) -> (String, String, i32) {
    let output = command.output().expect("failed to spawn tcl binary");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code().unwrap_or(-1),
    )
}

/// The first `tclsh` on `PATH`, when there is one.
fn tclsh_on_path() -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join("tclsh"))
        .find(|candidate| candidate.is_file())
}

const DEMO_PACKAGE: &str = "package provide demo 1.0\n\
namespace eval demo {}\n\
# One to three words.\n\
proc demo::flex {a {b x} {c y}} {return \"$a$b$c\"}\n\
# Exactly two.\n\
proc demo::fixed {a b} {return \"$a$b\"}\n";

fn demo_pack(flex: &str, fixed: &str) -> String {
    format!(
        "speclib demo 2.0 {{\n    command demo::flex {{\n        arity {flex}\n        \
         required_package demo\n    }}\n    command demo::fixed {{\n        arity {fixed}\n        \
         required_package demo\n    }}\n}}\n"
    )
}

/// A pack whose arity is narrower than the package's, and one wider, are each
/// reported against the real shell: `demo::flex` takes one to three words and the
/// pack says two, so a call with one and a call with three are accepted where the
/// pack says they are errors; `demo::fixed` takes two and the pack says one or
/// two, so a call with one is refused where the pack says it is valid. A pack
/// that describes the package truthfully has no row and exits 0.
#[test]
fn spec_test_reports_an_arity_divergence() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let wrong = Described::new(
        "spec-test-arity",
        DEMO_PACKAGE,
        &demo_pack("2", "1..2"),
        &["demo"],
    );
    let (stdout, stderr, code) = wrong.run(&tclsh);
    assert_eq!(code, 1, "stdout: {stdout}\nstderr: {stderr}");
    for row in [
        "demo::flex: arity: declares at least 2 argument(s), but `demo::flex x` was not refused as `wrong # args`",
        "demo::flex: arity: declares at most 2 argument(s), but `demo::flex x x x` was not refused as `wrong # args`",
        "demo::fixed: arity: declares 1 argument(s) valid, but `demo::fixed x` was refused as `wrong # args`",
        "2 command(s) tested against 'demo', 3 divergence(s)",
    ] {
        assert!(stdout.contains(row), "missing {row:?} in:\n{stdout}");
    }
    assert_eq!(
        stdout.matches("arity:").count(),
        3,
        "one row per divergence: {stdout}"
    );

    let truthful = Described::new(
        "spec-test-truth",
        DEMO_PACKAGE,
        &demo_pack("1..3", "2"),
        &["demo"],
    );
    let (stdout, stderr, code) = truthful.run(&tclsh);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stdout.contains("2 command(s) tested against 'demo', 0 divergence(s)"),
        "{stdout}"
    );
}

/// A command the pack declares and the package does not define is one row and no
/// other question, and the rest of the pack is still held to the package.
#[test]
fn spec_test_reports_a_command_the_package_does_not_define() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let pack = "speclib demo 2.0 {\n    command demo::fixed {\n        arity 2\n        \
                required_package demo\n    }\n    command demo::ghost {\n        arity 1\n        \
                required_package demo\n        traits {PURE}\n        hover { example {demo::ghost 1} }\n    }\n}\n";
    let ghost = Described::new("spec-test-ghost", DEMO_PACKAGE, pack, &["demo"]);
    let (stdout, stderr, code) = ghost.run(&tclsh);
    assert_eq!(code, 1, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stdout.contains("demo::ghost: missing: declared, but the package does not define it"),
        "{stdout}"
    );
    assert_eq!(
        stdout.matches("demo::ghost:").count(),
        1,
        "one row, and no question asked of a command that is not there: {stdout}"
    );
    assert!(
        stdout.contains("2 command(s) tested against 'demo', 1 divergence(s)"),
        "{stdout}"
    );
}

/// The policy that decides is the operator's: the project `tcl` is run in, whose
/// manifest is found from the working directory as `tcl pkg` finds it, and not the
/// directory the pack happens to be in.
#[test]
fn spec_test_reads_the_policy_of_the_project_it_is_run_in() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let held = Described::nested(
        "spec-test-project",
        DEMO_PACKAGE,
        &demo_pack("1..3", "2"),
        &["demo"],
    );
    // Run from a directory below the manifest, which is where the policy is.
    let mut command = held.command();
    command
        .current_dir(held.pack.parent().expect("the specs directory"))
        .arg("--tclsh")
        .arg(&tclsh);
    let (stdout, stderr, code) = finished(&mut command);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stdout.contains("2 command(s) tested against 'demo', 0 divergence(s)"),
        "{stdout}"
    );
}

/// A dependency vendored into the project has a manifest and a `tclpkg.toml` of
/// its own, and neither can be what lets it run: the policy is the operator's,
/// which says nothing of the package until the operator does.
#[test]
fn spec_test_ignores_the_policy_of_the_tree_the_pack_was_found_in() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let vendored = Described::vendored(
        "spec-test-vendored",
        DEMO_PACKAGE,
        &demo_pack("1..3", "2"),
        &["demo"],
    );
    let (stdout, stderr, code) = vendored.run(&tclsh);
    assert_eq!(code, 1, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stderr.contains("running the package 'demo' is not permitted by policy"),
        "the dependency opted itself in: {stderr}"
    );
    assert!(stdout.is_empty(), "nothing ran: {stdout}");

    vendored.trust_in_the_project(&["demo"]);
    let (stdout, stderr, code) = vendored.run(&tclsh);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stdout.contains("2 command(s) tested against 'demo', 0 divergence(s)"),
        "{stdout}"
    );
}

const BEHAVING_PACKAGE: &str = "package provide demo 1.0\n\
puts -nonewline \"demo loaded, with no newline\"\n\
namespace eval demo {variable cache; variable count 0}\n\
proc demo::double {x} {expr {$x * 2}}\n\
proc demo::label {x} {return \"value $x\"}\n\
proc demo::remember {x} {set ::remembered $x; return $x}\n\
proc demo::raises {x} {error \"no such thing\"}\n\
proc demo::memo {x} {if {![info exists ::memoised]} {set ::memoised $x}; return $x}\n\
proc demo::tidy {x} {set local $x; return $local}\n\
proc demo::cached {x} {variable cache; if {![info exists cache($x)]} {set cache($x) [expr {$x + 1}]}; return $cache($x)}\n\
proc demo::counted {x} {variable count; incr count; return $x}\n";

const BEHAVING_PACK: &str = "speclib demo 2.0 {\n\
    command demo::double {\n\
        arity 1\n\
        required_package demo\n\
        return_type Int\n\
        runtime_backing tcl-body {-pack-text {proc demo::double {x} {expr {$x * 3}}}}\n\
        hover { example {demo::double 21} }\n\
    }\n\
    command demo::label {\n\
        arity 1\n\
        required_package demo\n\
        return_type Int\n\
        hover { example {demo::label 7} }\n\
    }\n\
    command demo::remember {\n\
        arity 1\n\
        required_package demo\n\
        traits {PURE}\n\
        hover { example {demo::remember 5} }\n\
    }\n\
    command demo::raises {\n\
        arity 1\n\
        required_package demo\n\
        hover { example {demo::raises 1} }\n\
    }\n\
    command demo::memo {\n\
        arity 1\n\
        required_package demo\n\
        traits {PURE}\n\
        hover { example {demo::memo 3} }\n\
    }\n\
    command demo::tidy {\n\
        arity 1\n\
        required_package demo\n\
        traits {PURE}\n\
        hover { example {demo::tidy 4} }\n\
    }\n\
    command demo::cached {\n\
        arity 1\n\
        required_package demo\n\
        traits {PURE}\n\
        hover { example {demo::cached 4} }\n\
    }\n\
    command demo::counted {\n\
        arity 1\n\
        required_package demo\n\
        traits {PURE}\n\
        hover { example {demo::counted 4} }\n\
    }\n\
}\n";

/// The other questions: a `returns` type the answer is not a value of, an
/// `example` that raises, a command declared `pure` that writes a global or fills
/// a namespace variable its first call, and a Tcl-body reference body that answers
/// differently from the command it describes, each one row naming the command and
/// what the shell did.
#[test]
fn spec_test_reports_what_the_examples_the_purity_and_the_reference_body_disagree_on() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let behaving = Described::new(
        "spec-test-facts",
        BEHAVING_PACKAGE,
        BEHAVING_PACK,
        &["demo"],
    );
    let (stdout, stderr, code) = behaving.run(&tclsh);
    assert_eq!(code, 1, "stdout: {stdout}\nstderr: {stderr}");
    for row in [
        "demo::label: returns: declares it returns int, but `demo::label 7` answered `value 7`",
        "demo::remember: pure: declared pure, but `demo::remember 5` wrote the variable(s) remembered",
        "demo::raises: example: `demo::raises 1` raised: no such thing",
        "demo::memo: pure: declared pure, but running it created or changed the variable(s) memoised",
        "demo::cached: pure: declared pure, but running it created or changed the variable(s) ::demo::cache",
        "demo::counted: pure: declared pure, but `demo::counted 4` wrote the variable(s) ::demo::count",
        "demo::double: reference: `demo::double 21` answers `42` (0) as the command and `63` (0) as its reference body",
    ] {
        assert!(stdout.contains(row), "missing {row:?} in:\n{stdout}");
    }
    assert!(
        !stdout.contains("demo::double: returns")
            && !stdout.contains("demo::double: example")
            && !stdout.contains("demo::tidy"),
        "a command the pack describes truthfully has no row: {stdout}"
    );
    assert!(
        stdout.contains("8 command(s) tested against 'demo', 7 divergence(s)"),
        "{stdout}"
    );
}

/// A reference body a pack takes from a file of the package it ships is read when
/// the pack loads and held to the command as one written in the pack is.
#[test]
fn spec_test_compares_a_reference_body_the_pack_takes_from_a_package_file() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let pack = "speclib demo 2.0 {\n    command demo::double {\n        arity 1\n        \
                required_package demo\n        runtime_backing tcl-body {-package-source src/double.tcl}\n        \
                hover { example {demo::double 21} }\n    }\n}\n";
    let held = Described::nested(
        "spec-test-package-source",
        BEHAVING_PACKAGE,
        pack,
        &["demo"],
    );
    let project = held
        .pack
        .parent()
        .and_then(Path::parent)
        .expect("the project that holds the pack");
    std::fs::create_dir_all(project.join("src")).expect("src");
    std::fs::write(
        project.join("src").join("double.tcl"),
        "proc demo::double {x} {expr {$x * 3}}\n",
    )
    .expect("write the body");
    let (stdout, stderr, code) = held.run(&tclsh);
    assert_eq!(code, 1, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stdout.contains(
            "demo::double: reference: `demo::double 21` answers `42` (0) as the command and `63` (0) as its reference body"
        ),
        "{stdout}"
    );
}

/// Requiring a package runs its Tcl, so the verb runs only for a package the
/// project's policy opts in; nothing runs and the exit is 1 when it does not.
#[test]
fn spec_test_runs_only_a_package_the_policy_opts_in() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let unlisted = Described::new(
        "spec-test-policy",
        DEMO_PACKAGE,
        &demo_pack("2", "1..2"),
        &[],
    );
    let (stdout, stderr, code) = unlisted.run(&tclsh);
    assert_eq!(code, 1, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stderr.contains("running the package 'demo' is not permitted by policy"),
        "{stderr}"
    );
    assert!(stderr.contains("tcl pkg trust demo"), "{stderr}");
    assert!(stdout.is_empty(), "nothing ran: {stdout}");
}

/// A package the shell cannot require is one row and exit 1, and a pack with
/// no `required_package` agreement needs the package named.
#[test]
fn spec_test_says_when_the_package_cannot_be_required() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let missing = Described::new(
        "spec-test-missing",
        "package provide demo 1.0\n",
        &demo_pack("2", "2").replace("required_package demo", "required_package absent"),
        &["absent"],
    );
    let (stdout, stderr, code) = missing.run(&tclsh);
    assert_eq!(code, 1, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stdout.contains("-: load: can't find package absent"),
        "{stdout}"
    );
    assert!(
        stdout.contains("0 command(s) tested against 'absent', 1 divergence(s)"),
        "the package is not there, so nothing is asked: {stdout}"
    );

    let unnamed = Described::new(
        "spec-test-unnamed",
        DEMO_PACKAGE,
        "speclib demo 2.0 {\n    command demo::flex {\n        arity 1\n    }\n}\n",
        &["demo"],
    );
    let (_stdout, stderr, code) = unnamed.run(&tclsh);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("name the Tcl package with --package"),
        "{stderr}"
    );
}

/// A pack that declares no command has nothing to ask a shell, and a file that is
/// not there is an error, not an empty pack.
#[test]
fn spec_test_says_when_there_is_nothing_to_test() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let empty = Described::new(
        "spec-test-empty",
        DEMO_PACKAGE,
        "speclib demo 2.0 {\n}\n",
        &["demo"],
    );
    let (stdout, stderr, code) = empty.run(&tclsh);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stdout.contains("the pack declares no command to test"),
        "{stdout}"
    );

    let absent = empty.pack.with_file_name("absent.tclspec");
    let (stdout, stderr, code) = run(&[
        "spec",
        "test",
        &absent.to_string_lossy(),
        "--tclsh",
        &tclsh.to_string_lossy(),
    ]);
    assert_eq!(code, 2, "stdout: {stdout}\nstderr: {stderr}");
    assert!(stderr.contains("not a file"), "{stderr}");
}

/// A shell that stops before it has asked every command is not a pass, whatever
/// status it stops with: the verb says what the shell said and which commands it
/// never asked, and exits 1.
#[test]
fn spec_test_says_when_the_shell_stops_before_it_has_asked_every_command() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let stopping = Described::new(
        "spec-test-stops",
        "package provide demo 1.0\nputs stderr {demo is leaving}\nexit 3\n",
        &demo_pack("1..3", "2"),
        &["demo"],
    );
    let (stdout, stderr, code) = stopping.run(&tclsh);
    assert_eq!(code, 1, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stderr.contains("the shell exited with status 3 before it had asked every command")
            && stderr.contains("0 of 2 asked; not asked: demo::flex, demo::fixed")
            && stderr.contains("demo is leaving"),
        "{stderr}"
    );
    assert!(
        stdout.contains("0 of 2 command(s) tested against 'demo', 0 divergence(s)"),
        "{stdout}"
    );
}

/// A package that ends the shell with status 0 — when it is required, or when one
/// of its commands is asked — has not passed, and the commands it never let the
/// verb ask are named, not counted as tested.
#[test]
fn spec_test_does_not_pass_a_package_that_exits_the_shell_with_status_0() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let on_require = Described::new(
        "spec-test-exit-require",
        "package provide demo 1.0\nexit 0\n",
        &demo_pack("1..3", "2"),
        &["demo"],
    );
    let (stdout, stderr, code) = on_require.run(&tclsh);
    assert_eq!(code, 1, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stderr.contains("the shell exited with status 0 before it had asked every command")
            && stderr.contains("not asked: demo::flex, demo::fixed"),
        "{stderr}"
    );

    // The second command ends the shell when it is asked, so the first was asked
    // and its row is there, and the second and the third were not.
    let package = "package provide demo 1.0\nnamespace eval demo {}\n\
                   proc demo::flex {a {b x} {c y}} {return \"$a$b$c\"}\n\
                   proc demo::fixed {a b} {exit 0}\n\
                   proc demo::last {a} {return $a}\n";
    let pack = "speclib demo 2.0 {\n    command demo::flex {\n        arity 2\n        \
                required_package demo\n    }\n    command demo::fixed {\n        arity 2\n        \
                required_package demo\n    }\n    command demo::last {\n        arity 1\n        \
                required_package demo\n    }\n}\n";
    let part_way = Described::new("spec-test-exit-part-way", package, pack, &["demo"]);
    let (stdout, stderr, code) = part_way.run(&tclsh);
    assert_eq!(code, 1, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stdout.contains("demo::flex: arity: declares at least 2 argument(s)"),
        "the row found before the shell stopped is kept: {stdout}"
    );
    assert!(
        stdout.contains("1 of 3 command(s) tested against 'demo'"),
        "{stdout}"
    );
    assert!(
        stderr.contains("1 of 3 asked; not asked: demo::fixed, demo::last"),
        "{stderr}"
    );
}

/// With no shell named, the verb runs the one in the active virtual environment
/// (`TCL_VENV`), which here is a wrapper that records its use.
#[cfg(unix)]
#[test]
fn spec_test_uses_the_shell_of_the_active_venv_when_none_is_named() {
    use std::os::unix::fs::PermissionsExt;

    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let described = Described::new(
        "spec-test-venv",
        DEMO_PACKAGE,
        &demo_pack("1..3", "2"),
        &["demo"],
    );
    let venv = described.tree.path().join("venv");
    let bin = venv.join("bin");
    std::fs::create_dir_all(&bin).expect("venv bin");
    let wrapper = bin.join("tclsh");
    std::fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\necho used >> '{}'\nexec '{}' \"$@\"\n",
            venv.join("used").display(),
            tclsh.display()
        ),
    )
    .expect("write the wrapper");
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755))
        .expect("make the wrapper executable");
    let mut command = described.command();
    command.env("TCL_VENV", &venv);
    let (stdout, stderr, code) = finished(&mut command);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stdout.contains("2 command(s) tested against 'demo', 0 divergence(s)"),
        "{stdout}"
    );
    assert!(venv.join("used").is_file(), "the venv's shell ran");
}

/// `--package` names the package to require, and what it names wins over the
/// `required_package` the pack's commands declare.
#[test]
fn spec_test_takes_the_package_from_the_flag() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let flagged = Described::new(
        "spec-test-flag",
        DEMO_PACKAGE,
        &demo_pack("1..3", "2").replace("required_package demo", "required_package absent"),
        &["demo"],
    );
    let mut command = flagged.command();
    command.args(["--package", "demo", "--tclsh"]).arg(&tclsh);
    let (stdout, stderr, code) = finished(&mut command);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stdout.contains("2 command(s) tested against 'demo', 0 divergence(s)"),
        "{stdout}"
    );
}

/// The shell runs under the package manager's timeout, so a package that does
/// not finish is stopped and said so, and is not a pass.
#[test]
fn spec_test_stops_a_package_that_does_not_finish_in_time() {
    let Some(tclsh) = tclsh_on_path() else {
        eprintln!("skipped: no tclsh on PATH");
        return;
    };
    let slow = Described::new(
        "spec-test-timeout",
        "package provide demo 1.0\nafter 20000\n",
        &demo_pack("1..3", "2"),
        &["demo"],
    );
    let policy = slow.pack.parent().expect("the project").join("tclpkg.toml");
    let mut text = std::fs::read_to_string(&policy).expect("the policy");
    text.push_str("[sandbox]\nmax-timeout-secs = 1\n");
    std::fs::write(&policy, text).expect("write the policy");
    let started = std::time::Instant::now();
    let (stdout, stderr, code) = slow.run(&tclsh);
    assert_eq!(code, 2, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        stderr.contains("did not finish testing 'demo' in time"),
        "{stderr}"
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(15));
}
