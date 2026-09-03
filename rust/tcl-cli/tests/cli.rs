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

//! Structural / literal-assertion tests for the native `tcl` CLI.
//!
//! Each test runs the built `tcl` binary and asserts its output against
//! expectations written directly in this file — exit codes, output
//! substrings, or structural checks on parsed JSON — rather than against
//! committed golden/snapshot fixtures.

use std::path::PathBuf;
use std::process::{Command, Output};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn spec_pack_project() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/spec-packs/tiny-project")
}

/// Run the built `tcl` binary with `args`, returning captured stdout bytes.
fn run_tcl(args: &[&str]) -> Vec<u8> {
    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(args)
        .output()
        .expect("failed to spawn tcl binary");
    assert!(
        output.status.success(),
        "tcl {args:?} exited {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn run_tcl_in(current_dir: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tcl"))
        .current_dir(current_dir)
        .args(args)
        .output()
        .expect("failed to spawn tcl binary")
}

#[test]
fn command_info_discovers_the_current_projects_spec_pack() {
    let project = spec_pack_project();
    let with_pack = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .current_dir(&project)
        .args(["command-info", "::tcl_lsp_fixture::collect", "--json"])
        .output()
        .expect("failed to spawn tcl binary");
    assert!(
        with_pack.status.success(),
        "project pack was not discovered: {}",
        String::from_utf8_lossy(&with_pack.stderr)
    );
    let found: serde_json::Value =
        serde_json::from_slice(&with_pack.stdout).expect("command-info JSON");
    assert_eq!(found["found"], true);
    assert_eq!(
        found["summary"],
        "Evaluate a script while collecting a result in a caller variable."
    );

    let without_pack = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .current_dir(project.join("lib"))
        .args(["command-info", "::tcl_lsp_fixture::collect", "--json"])
        .output()
        .expect("failed to spawn tcl binary");
    assert_eq!(without_pack.status.code(), Some(1));
    let missing: serde_json::Value =
        serde_json::from_slice(&without_pack.stdout).expect("command-info JSON");
    assert_eq!(missing["found"], false);
}

#[test]
fn diag_analysis_changes_when_the_current_projects_spec_pack_is_present() {
    let project = spec_pack_project();
    let args = [
        "diag",
        "--source",
        "::tcl_lsp_fixture::collect output",
        "--json",
    ];

    let with_pack = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .current_dir(&project)
        .args(args)
        .output()
        .expect("failed to spawn tcl binary");
    assert_eq!(with_pack.status.code(), Some(1));
    let analysed: serde_json::Value =
        serde_json::from_slice(&with_pack.stdout).expect("diag JSON with pack");
    let codes: Vec<&str> = analysed[0]["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str())
        .collect();
    assert_eq!(codes, ["E002", "W120"]);

    let without_pack = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .current_dir(project.join("lib"))
        .args(args)
        .output()
        .expect("failed to spawn tcl binary");
    assert!(without_pack.status.success());
    let opaque: serde_json::Value =
        serde_json::from_slice(&without_pack.stdout).expect("diag JSON without pack");
    assert_eq!(opaque[0]["diagnostics"], serde_json::json!([]));
}

#[test]
fn analyser_only_verbs_install_the_current_projects_spec_pack_overlay() {
    let project = spec_pack_project();
    let source = "::tcl_lsp_fixture::collect output { proc nested {} { expr $input + 1 } }";

    let symbols = run_tcl_in(&project, &["symbols", "--source", source, "--json"]);
    assert!(symbols.status.success());
    let symbols: serde_json::Value =
        serde_json::from_slice(&symbols.stdout).expect("symbols JSON with pack");
    assert!(
        symbols["symbols"]
            .as_array()
            .expect("symbols array")
            .iter()
            .any(|symbol| symbol["name"] == "nested"),
        "the Body role must expose the nested procedure"
    );

    let legacy = run_tcl_in(&project, &["find-legacy", "--source", source, "--json"]);
    assert!(legacy.status.success());
    let legacy: serde_json::Value =
        serde_json::from_slice(&legacy.stdout).expect("find-legacy JSON with pack");
    assert_eq!(legacy["issues"][0]["code"], "W100");

    let minimized = run_tcl_in(
        &project,
        &["minimize", "--source", source, "W100", "--json"],
    );
    assert!(minimized.status.success());
    let minimized: serde_json::Value =
        serde_json::from_slice(&minimized.stdout).expect("minimize JSON with pack");
    assert_eq!(minimized[0]["reproduces"], true);

    let without_pack = run_tcl_in(
        &project.join("lib"),
        &["symbols", "--source", source, "--json"],
    );
    assert!(without_pack.status.success());
    let without_pack: serde_json::Value =
        serde_json::from_slice(&without_pack.stdout).expect("symbols JSON without pack");
    assert_eq!(without_pack["count"], 0);
}

#[test]
fn explore_uses_the_current_projects_active_spec_pack_registry() {
    let project = spec_pack_project();
    let source = "::tcl_lsp_fixture::collect output { set value 1 }";
    let args = ["explore", "--source", source, "--json"];

    let with_pack = run_tcl_in(&project, &args);
    assert!(with_pack.status.success());
    let with_pack: serde_json::Value =
        serde_json::from_slice(&with_pack.stdout).expect("explore JSON with pack");
    assert_eq!(
        with_pack["semantic"][0]["invocations"][0]["resolution"],
        "resolved"
    );
    assert_eq!(
        with_pack["worldSsa"][0]["invocations"][0]["command"],
        "::tcl_lsp_fixture::collect"
    );

    let without_pack = run_tcl_in(&project.join("lib"), &args);
    assert!(without_pack.status.success());
    let without_pack: serde_json::Value =
        serde_json::from_slice(&without_pack.stdout).expect("explore JSON without pack");
    assert_eq!(
        without_pack["semantic"][0]["invocations"][0]["resolution"],
        "unresolved-unknown-literal-head"
    );
}

#[test]
fn cli_publishes_project_pack_hooks() {
    let project = spec_pack_project();
    let args = [
        "opt",
        "--source",
        "set length [::tcl_lsp_fixture::strlen abcde]",
    ];

    let with_pack = run_tcl_in(&project, &args);
    assert!(with_pack.status.success());
    let with_pack = String::from_utf8(with_pack.stdout).expect("UTF-8 opt output");
    assert!(
        with_pack.contains("set length 5"),
        "hook did not fold: {with_pack}"
    );
    assert!(
        with_pack.contains("O129"),
        "hook rewrite was not reported: {with_pack}"
    );

    let without_pack = run_tcl_in(&project.join("lib"), &args);
    assert!(without_pack.status.success());
    let without_pack = String::from_utf8(without_pack.stdout).expect("UTF-8 opt output");
    assert!(without_pack.contains("::tcl_lsp_fixture::strlen abcde"));
    assert!(!without_pack.contains("O129"));
}

#[test]
fn highlight_uses_the_current_projects_spec_pack_registry() {
    let project = spec_pack_project();
    let args = [
        "highlight",
        "--source",
        "::tcl_lsp_fixture::catalog names",
        "--format",
        "html",
    ];

    let with_pack = run_tcl_in(&project, &args);
    assert!(with_pack.status.success());
    let with_pack = String::from_utf8(with_pack.stdout).expect("UTF-8 highlight output");
    assert!(
        with_pack.contains("color:#2c5282;\">names</span>"),
        "pack subcommand was not highlighted: {with_pack}"
    );

    let without_pack = run_tcl_in(&project.join("lib"), &args);
    assert!(without_pack.status.success());
    let without_pack = String::from_utf8(without_pack.stdout).expect("UTF-8 highlight output");
    assert!(!without_pack.contains("color:#2c5282;\">names</span>"));
}

#[test]
fn explore_json_emits_the_contract_keys() {
    let out = run_tcl(&["explore", "--source", "set x 1\nputs $x", "--json"]);
    let value: serde_json::Value =
        serde_json::from_slice(&out).expect("explore --json must emit valid JSON");
    let obj = value.as_object().expect("top-level object");
    // A representative spread of views is present.
    for key in [
        "meta",
        "ir",
        "cfgPreSsa",
        "cfgPostSsa",
        "segments",
        "asm",
        "stats",
    ] {
        assert!(obj.contains_key(key), "missing explorer key {key:?}");
    }
}

#[test]
fn explore_summary_lists_views() {
    let out = run_tcl(&["explore", "--source", "set x 1"]);
    let text = String::from_utf8(out).expect("utf-8 summary");
    assert!(text.contains("Compiler explorer summary"));
    assert!(text.contains("ir:"));
}

/// Smoke variant of [`explore_summary_lists_views`]: the cheapest possible
/// "the `tcl` binary still starts, parses its args, and reports success" CLI
/// invocation, on the default-features surface only (no `--tui`).
#[test]
fn smoke_explore_reports_summary_for_tiny_snippet() {
    let out = run_tcl(&["explore", "--source", "set x 1"]);
    let text = String::from_utf8(out).expect("utf-8 summary");
    assert!(text.contains("Compiler explorer summary"));
}

#[test]
fn explore_text_renders_box_drawing_trees() {
    let out = run_tcl(&[
        "explore",
        "--source",
        "set x 1\nputs $x",
        "--text",
        "--show",
        "ir",
        "--no-colour",
    ]);
    let text = String::from_utf8(out).expect("utf-8 text render");
    assert!(text.contains("=== ir ==="), "section header present");
    assert!(
        text.contains("├── ") || text.contains("└── "),
        "box-drawing tree connectors present"
    );
    assert!(!text.contains('\x1b'), "no ANSI escapes with --no-colour");
}

// When CODE fires on no input, the verb prints `CODE does not fire on any
// input.` to stderr and exits 1.
#[test]
fn minimize_missing_code_errors() {
    let input = fixtures_dir().join("minimize.tcl");
    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(["minimize", input.to_str().unwrap(), "ZZZ999"])
        .output()
        .expect("failed to spawn tcl binary");
    assert_eq!(output.status.code(), Some(1), "exit code");
    assert!(output.stdout.is_empty(), "no stdout on the no-fire path");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "ZZZ999 does not fire on any input.\n",
    );
}

// Property test (the task's explicit requirement): the reduced reproducer must
// still fire CODE. We assert it two ways — the verb's own `reproduces` flag in
// the JSON, and independently by re-running `tcl diag` on the reduced source
// and confirming CODE is present. This is the invariant that makes a minimised
// snippet a valid bug-report repro regardless of how far ddmin reduced.
#[test]
fn minimize_reduced_output_still_fires() {
    let input = fixtures_dir().join("minimize.tcl");
    let out = run_tcl(&["minimize", input.to_str().unwrap(), "W100", "--json"]);
    let value: serde_json::Value =
        serde_json::from_slice(&out).expect("minimize --json must emit valid JSON");
    let items = value.as_array().expect("top-level array");
    assert!(!items.is_empty(), "W100 fires, so there is a result");
    for item in items {
        assert_eq!(
            item["reproduces"],
            serde_json::Value::Bool(true),
            "the verb reports the reduction reproduces"
        );
        let reduced = item["source"].as_str().expect("source string");
        // Independently confirm the analyser still fires W100 on the reduced
        // snippet via `tcl diag --json`. `diag` exits 1 when it finds a
        // problem-severity diagnostic (W100 is an error), so we read its stdout
        // directly rather than through the success-asserting `run_tcl`.
        let diag = Command::new(env!("CARGO_BIN_EXE_tcl"))
            .args(["diag", "--source", reduced, "--json"])
            .output()
            .expect("failed to spawn tcl binary")
            .stdout;
        let report: serde_json::Value =
            serde_json::from_slice(&diag).expect("diag --json must emit valid JSON");
        let fires = report
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|f| f["diagnostics"].as_array())
            .flatten()
            .any(|d| d["code"] == serde_json::Value::String("W100".to_owned()));
        assert!(fires, "reduced source {reduced:?} must still fire W100");
    }
}

#[test]
fn minify_symbol_map_written_for_plain_minify() {
    // `tcl minify --symbol-map FILE` without `--compact`/`--aggressive` must
    // still create the map file (an empty / identity map), not silently skip
    // it — otherwise a later `unminify-error` fails on a missing path
    // (issue 198).
    let input = fixtures_dir().join("greet.tcl");
    let tmp = std::env::temp_dir().join(format!(
        "tcl-cli-symmap-{}-{}.txt",
        std::process::id(),
        line!()
    ));
    let _ = std::fs::remove_file(&tmp);
    let _ = run_tcl(&[
        "minify",
        "--symbol-map",
        tmp.to_str().unwrap(),
        input.to_str().unwrap(),
    ]);
    assert!(
        tmp.exists(),
        "plain minify must still write the requested --symbol-map file"
    );
    let _ = std::fs::remove_file(&tmp);
}

/// Issue #977: `tcl diag` over several inputs is a multi-file compilation, so
/// a call in one file must be visible to another file's interprocedural
/// constant seed.  On its own, the library's two agreeing callers make
/// `$mode eq "prod"` fold (I230); adding the file that calls it with `dev`
/// must retract that.
#[test]
fn diag_shares_call_sites_across_inputs() {
    let lib = fixtures_dir().join("issue977Lib.tcl");
    let main = fixtures_dir().join("issue977Main.tcl");
    let alone = String::from_utf8(run_tcl_allow_failure(&["diag", lib.to_str().unwrap()]))
        .expect("utf-8 output");
    assert!(
        alone.contains("I230"),
        "the library alone has only agreeing callers, so the fold is expected: {alone}"
    );
    let together = String::from_utf8(run_tcl_allow_failure(&[
        "diag",
        lib.to_str().unwrap(),
        main.to_str().unwrap(),
    ]))
    .expect("utf-8 output");
    assert!(
        !together.contains("I230"),
        "issue977Main.tcl calls the helper with \"dev\": {together}"
    );
}

/// Issue #1048: the transform verbs auto-detect a document's dialect, so an
/// iRule folds the same with and without an explicit `--dialect`.
///
/// Before the fix `dialect_or_default()` returned `tcl8.6` whenever the flag
/// was absent, so the optimiser ran the file as plain Tcl: `contains` was not
/// an operator, the condition never folded, and no `O101` was reported.
#[test]
fn opt_detects_the_irules_dialect_without_the_flag() {
    let input = fixtures_dir().join("wordOperator.irule");
    let detected =
        String::from_utf8(run_tcl(&["opt", input.to_str().unwrap()])).expect("utf-8 output");
    let explicit = String::from_utf8(run_tcl(&[
        "opt",
        "--dialect",
        "f5-irules",
        input.to_str().unwrap(),
    ]))
    .expect("utf-8 output");
    assert!(
        detected.contains("O112") && detected.contains("HTTP::respond 200"),
        "the detected dialect must fold the word-operator condition: {detected}"
    );
    assert_eq!(
        detected, explicit,
        "detection must produce exactly what --dialect f5-irules produces"
    );
}

/// The control for [`opt_detects_the_irules_dialect_without_the_flag`]: the
/// same condition in plain Tcl source stays plain Tcl, where `contains` is not
/// an operator and nothing folds.
#[test]
fn opt_leaves_a_word_operator_alone_in_plain_tcl() {
    let out = String::from_utf8(run_tcl(&[
        "opt",
        "--source",
        "set x \"abcdef\"\nif {$x contains \"cd\"} { puts hit }",
    ]))
    .expect("utf-8 output");
    assert!(
        !out.contains("if {1}"),
        "plain Tcl has no `contains` operator, so the condition must not fold: {out}"
    );
}

/// Like [`run_tcl`] but tolerates a non-zero exit — `diag` returns 1 whenever
/// it reports a problem-severity finding, which is not a harness failure.
fn run_tcl_allow_failure(args: &[&str]) -> Vec<u8> {
    Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(args)
        .output()
        .expect("failed to spawn tcl binary")
        .stdout
}

/// A `.sslictcl` document terminated with lone `\r` must draw the same loader
/// findings as the `\n` form. `tclsh` ends a command at a bare CR, but the
/// lexer treats one as horizontal whitespace, so the loader has to read the
/// normalised analysis text — exactly as the server does before publishing
/// `SSLIC1xxx`. Without that, every declaration collapses into one command and
/// the CLI disagrees with the editor on a file the editor handles correctly.
#[test]
fn diag_reads_a_cr_terminated_sslictcl_document_the_way_the_editor_does() {
    let lf = "sslictcl 1\nsite-owner {a}\nrenewal-window {b}\ndeployment-note {c}\n";
    let cr = lf.replace('\n', "\r");

    let lf_codes = sslictcl_diag_rows("lf", lf);
    let cr_codes = sslictcl_diag_rows("cr", &cr);

    // Three unknown top-level words, each preserved as an extension on its own
    // line — the whole point of the vocabulary's forwards-compatibility rule.
    assert_eq!(
        lf_codes,
        vec![
            ("SSLIC1101".to_owned(), 2),
            ("SSLIC1101".to_owned(), 3),
            ("SSLIC1101".to_owned(), 4),
        ],
        "the `\\n` form is the reference reading"
    );
    assert_eq!(
        cr_codes, lf_codes,
        "a lone-CR document must read identically to the `\\n` one"
    );
}

/// Run `tcl diag --json` over one `.sslictcl` document written to a scratch
/// file (so the dialect routes by extension, not by content signature), and
/// return its `SSLIC*` rows as `(code, line)` pairs in report order.
fn sslictcl_diag_rows(tag: &str, text: &str) -> Vec<(String, u64)> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("tcl-cli-sslictcl-{tag}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let path = dir.join("doc.sslictcl");
    std::fs::write(&path, text).expect("write document");

    let out = run_tcl_allow_failure(&["diag", path.to_str().expect("utf-8 path"), "--json"]);
    let report: serde_json::Value = serde_json::from_slice(&out).expect("diag JSON");
    let rows = report[0]["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .filter_map(|d| {
            let code = d["code"].as_str()?;
            code.starts_with("SSLIC")
                .then(|| (code.to_owned(), d["line"].as_u64().expect("line")))
        })
        .collect();
    std::fs::remove_dir_all(&dir).ok();
    rows
}
