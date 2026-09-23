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

/// An `XDG_CONFIG_HOME` for one spawn that names no directory — unique to
/// the spawn and never created — so no `tcl-lsp/config.ini` can be found
/// under it and nothing is left behind: the global layer every
/// policy-reading verb resolves (`diag`, `lint`, `validate`, `opt`) is empty,
/// and a developer's own `config.ini` cannot fail a test CI passes. A missing
/// file is an absent layer, read silently.
fn absent_config_home() -> PathBuf {
    static SPAWNS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let spawn = SPAWNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "tcl-cli-tests-no-config-{}-{spawn}",
        std::process::id()
    ))
}

/// The built `tcl` binary, isolated from the machine's global configuration
/// (`XDG_CONFIG_HOME` is consulted first on every platform). Every spawn in
/// this file starts here; a test that wants a global layer of its own sets
/// `XDG_CONFIG_HOME` again, which overrides this one.
fn tcl() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tcl"));
    command.env("XDG_CONFIG_HOME", absent_config_home());
    command
}

/// Run the built `tcl` binary with `args`, returning captured stdout bytes.
fn run_tcl(args: &[&str]) -> Vec<u8> {
    let output = tcl()
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
    tcl()
        .current_dir(current_dir)
        .args(args)
        .output()
        .expect("failed to spawn tcl binary")
}

#[test]
fn command_info_discovers_the_current_projects_spec_pack() {
    let project = spec_pack_project();
    let with_pack = tcl()
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

    let without_pack = tcl()
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

    let with_pack = tcl()
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

    let without_pack = tcl()
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
    let output = tcl()
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
        let diag = tcl()
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
    let scratch = Scratch::new("symmap");
    let map = scratch.0.join("symmap.txt");
    let _ = run_tcl(&[
        "minify",
        "--symbol-map",
        map.to_str().unwrap(),
        input.to_str().unwrap(),
    ]);
    assert!(
        map.exists(),
        "plain minify must still write the requested --symbol-map file"
    );
}

/// `tcl diag` over several inputs is a multi-file compilation, so a call in
/// one file must be visible to another file's interprocedural constant seed.
/// On its own, the library's two agreeing callers make `$mode eq "prod"`
/// fold (I230); adding the file that calls it with `dev` must retract that.
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

/// The transform verbs auto-detect a document's dialect, so an iRule folds
/// the same with and without an explicit `--dialect`.
///
/// If `dialect_or_default()` fell back to `tcl8.6` whenever the flag was
/// absent, the optimiser would run the file as plain Tcl: `contains` would
/// not be an operator, the condition would never fold, and no `O101` would
/// be reported.
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

/// Companion to #2120: fixing the cross-file fold must not change what the
/// output *is*. README and `kcs-feature-tcl-verb-cli` document
/// `tcl opt src/ -o build/optimised.tcl` as optimising a tree "into one
/// output script", so the rendered text stays a program: no `# file:` banner
/// may precede it, or a leading `#!` is pushed off byte zero and the result
/// is no longer executable. Per-file attribution belongs in the trailing
/// comment block, which cannot corrupt the script.
#[test]
fn opt_over_several_inputs_keeps_the_first_shebang_at_byte_zero() {
    let out = String::from_utf8(run_tcl(&[
        "opt",
        "--source",
        "#!/usr/bin/env tclsh\nset a [expr {1 + 1}]\nputs $a",
        "--source",
        "set b [expr {2 + 2}]\nputs $b",
    ]))
    .expect("utf-8 output");
    assert!(
        out.starts_with("#!/usr/bin/env tclsh"),
        "the first input's shebang must stay at byte 0 so the bundled script \
         is still executable: {out}"
    );
    // The attribution still has to be somewhere — in the comment block.
    assert!(
        out.contains("# optimised:"),
        "the rewrite summary must still be reported: {out}"
    );
}

/// Regression for #2120: `tcl opt` over several inputs used to concatenate
/// them into one program before optimising, so a `set` in the first file
/// could be constant-propagated into a read in the second and the first
/// file's now-"unused" store eliminated as dead — even though the two files
/// are never run in the same scope. Each input must be optimised on its own.
#[test]
fn opt_does_not_fold_a_store_across_a_file_boundary() {
    let out = String::from_utf8(run_tcl(&[
        "opt",
        "--source",
        "set shared_value 42",
        "--source",
        "puts $shared_value",
    ]))
    .expect("utf-8 output");
    assert!(
        out.contains("puts $shared_value"),
        "the second input never sees the first input's assignment at run \
         time, so the read must stay a variable read, not fold to a literal: \
         {out}"
    );
    assert!(
        !out.contains("puts 42"),
        "the old concatenating path forwarded the literal across the file \
         boundary: {out}"
    );
    assert!(
        out.contains("set shared_value 42"),
        "the first input's store must survive — it is not dead just because \
         a *different* file never reads it: {out}"
    );
    assert!(
        !out.contains("O109"),
        "the old path eliminated the store as a dead store once the fold \
         made it look unused: {out}"
    );
}

/// Like [`run_tcl`] but tolerates a non-zero exit — `diag` returns 1 whenever
/// it reports a problem-severity finding, which is not a harness failure.
fn run_tcl_allow_failure(args: &[&str]) -> Vec<u8> {
    tcl()
        .args(args)
        .output()
        .expect("failed to spawn tcl binary")
        .stdout
}

/// `# noqa` silences a diagnostic for `tcl diag` exactly as it does in the
/// editor (`docs/kcs/kcs-howto-suppress-diagnostics.md`): the directive covers
/// the analyser families (`W210`) and the compiler-check families (`S100`)
/// alike, because both surfaces ask the one shared `line_suppressed` helper.
///
/// A comment that merely mentions the word is not a directive, so the finding
/// below it still fires.
///
/// The control is the same fixture with its directive lines stripped: every
/// code the markers silence must come back, or this test would pass on a
/// `diag` that had simply stopped reporting.
#[test]
fn diag_honours_noqa_directives_the_way_the_editor_does() {
    let fixture = fixtures_dir().join("noqaSuppression.tcl");
    let source = std::fs::read_to_string(&fixture).expect("fixture is readable");

    let marked = diag_messages(&["diag", "--json", fixture.to_str().unwrap()]);
    for silenced in [
        // `# noqa: W210` — the named analyser code.
        "suppressedByCode",
        // bare `# noqa` — every code on the following command.
        "suppressedByBareNoqa",
        // `# noqa: S100` — a compiler-check code from the other lift.
        "dictValue",
    ] {
        assert!(
            !marked.iter().any(|m| m.contains(silenced)),
            "a preceding noqa must silence the finding on `{silenced}`: {marked:?}"
        );
    }
    assert!(
        marked.iter().any(|m| m.contains("reportedWithoutAMarker")),
        "an unmarked W210 must still be reported: {marked:?}"
    );
    assert!(
        marked.iter().any(|m| m.contains("reportedBesideProse")),
        "a comment that only mentions the word is not a directive: {marked:?}"
    );
    assert!(
        marked.iter().any(|m| m.contains("otherDict")),
        "an unmarked S100 must still be reported: {marked:?}"
    );

    let unmarked: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("# noqa"))
        .collect::<Vec<_>>()
        .join("\n");
    let without_markers = diag_messages(&["diag", "--json", "--source", &unmarked]);
    for reported in [
        "suppressedByCode",
        "suppressedByBareNoqa",
        "reportedWithoutAMarker",
        "reportedBesideProse",
        "dictValue",
        "otherDict",
    ] {
        assert!(
            without_markers.iter().any(|m| m.contains(reported)),
            "without its marker the finding on `{reported}` must fire: {without_markers:?}"
        );
    }
}

/// Every diagnostic message a `diag --json` run reports, across all its files.
fn diag_messages(args: &[&str]) -> Vec<String> {
    let report: serde_json::Value =
        serde_json::from_slice(&run_tcl_allow_failure(args)).expect("diag JSON");
    report
        .as_array()
        .expect("diag reports an array of files")
        .iter()
        .flat_map(|file| {
            file["diagnostics"]
                .as_array()
                .expect("diagnostics array")
                .iter()
                .map(|d| {
                    format!(
                        "{} {}",
                        d["code"].as_str().unwrap_or_default(),
                        d["message"].as_str().unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
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

/// `tcl diag` runs the source-text pass the editor publishes, so a style
/// finding is not something you have to open an editor to see.
///
/// W111 / W112 / W115 / W118 come from the same `source_style` orchestrator the
/// server's style lift calls, and the byte-backed W107 / W109 ride with them.
#[test]
fn diag_reports_the_source_style_findings_the_editor_publishes() {
    let messages = diag_messages(&["diag", "--json", "--source", "set x 1   \nset y $x\n"]);
    assert!(
        messages.iter().any(|m| m.starts_with("W112")),
        "trailing whitespace must be reported: {messages:?}"
    );

    let long = format!("set x \"{}\"\nputs $x\n", "a".repeat(200));
    let long_messages = diag_messages(&["diag", "--json", "--source", &long]);
    assert!(
        long_messages.iter().any(|m| m.starts_with("W111")),
        "an over-long line must be reported: {long_messages:?}"
    );
}

/// A top-of-file `# tcl-lsp: disable=…` silences a code for every pass, not
/// only the analyser's own.
///
/// The analyser folds the directive into its internal disabled set, so its
/// codes obeyed it already; the source-text and compiler-check passes are
/// filtered by the caller, which is where the directive has to reach them.
#[test]
fn diag_honours_a_file_directive_across_every_pass() {
    let source = "# tcl-lsp: disable=W112\nset x 1   \nputs $x\n";
    let messages = diag_messages(&["diag", "--json", "--source", source]);
    assert!(
        !messages.iter().any(|m| m.starts_with("W112")),
        "a file-level directive must silence the style pass too: {messages:?}"
    );

    let without = diag_messages(&["diag", "--json", "--source", "set x 1   \nputs $x\n"]);
    assert!(
        without.iter().any(|m| m.starts_with("W112")),
        "without the directive the same document reports it: {without:?}"
    );
}

/// A file whose bytes are not UTF-8 text reports the integrity code alone, and
/// a file-level directive silences it by name or by the `*` wildcard.
///
/// `*` is the spelling `# tcl-lsp: disable=*` records, and it governs this
/// family as it governs every other. The document below is NUL-interleaved,
/// which is what makes the analysis abstain: everything derived from the
/// decoded text would describe positions the file does not have.
#[test]
fn diag_honours_a_file_directive_on_an_abstaining_document() {
    let nul_run = "\u{0}".repeat(80);
    let plain = format!("set x 1\n{nul_run}");
    let by_name = format!("# tcl-lsp: disable=W109\nset x 1\n{nul_run}");
    let by_wildcard = format!("# tcl-lsp: disable=*\nset x 1\n{nul_run}");

    let plain_rows = tcl_diag_rows("abstain-plain", &plain);
    assert_eq!(
        plain_rows
            .iter()
            .map(|(code, _)| code.as_str())
            .collect::<Vec<_>>(),
        ["W109"],
        "an abstaining document reports the integrity code and nothing else"
    );
    assert!(
        tcl_diag_rows("abstain-named", &by_name).is_empty(),
        "`disable=W109` must silence it"
    );
    assert!(
        tcl_diag_rows("abstain-wildcard", &by_wildcard).is_empty(),
        "`disable=*` must silence it too"
    );
}

/// The rows a lone-CR document and its `\n` twin must agree on: everything
/// except `W118`, the one lint whose subject *is* the line terminators.
fn without_line_ending_lint(rows: &[(String, u64)]) -> Vec<(String, u64)> {
    rows.iter()
        .filter(|(code, _)| code != "W118")
        .cloned()
        .collect()
}

/// Every code on the `tcl diag` path, not only `SSLIC1xxx`, must read the
/// analysis form of a lone-CR document.
///
/// If the analyser and compiler-checks passes read the raw bytes instead
/// (even with the loader branch normalising for itself), that diverges from
/// the editor twice over: the lexer treats a bare `\r` as horizontal
/// whitespace, so the whole file parses as one command — inventing findings
/// and hiding real ones — and `LineIndex` starts a line only after a `\n`,
/// so whatever survives is reported at line 1.
///
/// The reproducer is an unclosed bracket on the second line.
#[test]
fn diag_reads_a_cr_terminated_tcl_document_the_way_the_editor_does() {
    let lf = "set a 1\nset b [\nputs $a\n";
    let cr = lf.replace('\n', "\r");

    let lf_rows = tcl_diag_rows("lf", lf);
    let cr_rows = tcl_diag_rows("cr", &cr);

    assert!(
        lf_rows.contains(&("E201".to_owned(), 2)),
        "the `\\n` form is the reference reading: {lf_rows:?}"
    );
    assert_eq!(
        without_line_ending_lint(&cr_rows),
        without_line_ending_lint(&lf_rows),
        "a lone-CR document must read identically to the `\\n` one"
    );
    // The terminators themselves are the one legitimate difference: the CR form
    // is not the expected `\n`, so it earns the W118 its twin cannot.
    assert!(
        cr_rows.iter().any(|(code, _)| code == "W118"),
        "the CR form's terminators must be reported: {cr_rows:?}"
    );
    assert!(
        !lf_rows.iter().any(|(code, _)| code == "W118"),
        "the `\\n` form's terminators are the expected ones: {lf_rows:?}"
    );
    // The two specific ways the raw form diverged, named so a regression is
    // legible rather than just "the vectors differ". W118 is exempt: it is a
    // whole-file verdict anchored at the top of the document, not a finding
    // about the line it sits on.
    assert!(
        without_line_ending_lint(&cr_rows)
            .iter()
            .all(|(_, line)| *line > 1),
        "no finding may collapse onto line 1: {cr_rows:?}"
    );
    assert!(
        !cr_rows.iter().any(|(code, _)| code == "E003"),
        "parsing the file as one command invents an arity error: {cr_rows:?}"
    );
}

/// Dialect *detection* must read the analysis form too.
///
/// `detect_dialect`'s directive, shebang and version-guard tiers scan by line,
/// and Rust's `lines()` splits on `\n` only, so on the raw form of an old-Mac
/// document the whole file is line 1 and those tiers see nothing. The document
/// below is routed to `f5-irules` by its content, and under the wrong dialect
/// `when` and `HTTP::uri` are unknown commands.
#[test]
fn diag_detects_the_dialect_of_a_cr_terminated_document() {
    let lf = "# ordinary header\nwhen HTTP_REQUEST {\n    set u [HTTP::uri]\n}\n";
    let cr = lf.replace('\n', "\r");

    let lf_rows = tcl_diag_rows("dialect-lf", lf);
    let cr_rows = tcl_diag_rows("dialect-cr", &cr);

    assert_eq!(
        without_line_ending_lint(&cr_rows),
        without_line_ending_lint(&lf_rows),
        "the dialect a lone-CR document resolves to must match its `\\n` twin"
    );
    assert!(
        !cr_rows.iter().any(|(code, _)| code == "W123"),
        "resolving as generic Tcl makes the iRules commands unknown: {cr_rows:?}"
    );
}

/// The cross-file evidence scans must read the analysis form too.
///
/// `tcl diag a.tcl b.tcl` is one compilation: the declared-procedure set
/// and the call-site scan decide what may be folded. On the raw form of a
/// lone-CR pair both scans parse each file as one command, so the caller in the
/// second file is invisible and the fold the pair should retract survives.
///
/// This is `diag_shares_call_sites_across_inputs` over the same fixtures with
/// every `\n` rewritten to `\r`.
#[test]
fn diag_shares_call_sites_across_cr_terminated_inputs() {
    let lib = std::fs::read_to_string(fixtures_dir().join("issue977Lib.tcl")).expect("lib fixture");
    let main =
        std::fs::read_to_string(fixtures_dir().join("issue977Main.tcl")).expect("main fixture");

    let alone = multi_file_diag_text("cr-alone", &[("issue977Lib.tcl", &lib.replace('\n', "\r"))]);
    assert!(
        alone.contains("I230"),
        "the library alone still has only agreeing callers: {alone}"
    );
    let together = multi_file_diag_text(
        "cr-together",
        &[
            ("issue977Lib.tcl", &lib.replace('\n', "\r")),
            ("issue977Main.tcl", &main.replace('\n', "\r")),
        ],
    );
    assert!(
        !together.contains("I230"),
        "the `dev` call in the second file must be visible on the CR form too: {together}"
    );
}

/// Write each `(name, text)` into one scratch directory, run `tcl diag` over
/// all of them in order, and return the combined report text.
fn multi_file_diag_text(tag: &str, files: &[(&str, &str)]) -> String {
    let scratch = Scratch::new(&format!("multi-{tag}"));
    let mut args: Vec<String> = vec!["diag".to_owned()];
    for (name, text) in files {
        let path = scratch.write(name, text);
        args.push(path.to_str().expect("utf-8 path").to_owned());
    }
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    String::from_utf8(run_tcl_allow_failure(&borrowed)).expect("utf-8 output")
}

/// Run `tcl diag --json` over one `.tcl` document written to a scratch file and
/// return every row as a `(code, line)` pair in report order.
fn tcl_diag_rows(tag: &str, text: &str) -> Vec<(String, u64)> {
    let scratch = Scratch::new(&format!("cr-{tag}"));
    let path = scratch.write("doc.tcl", text);

    let out = run_tcl_allow_failure(&["diag", path.to_str().expect("utf-8 path"), "--json"]);
    let report: serde_json::Value = serde_json::from_slice(&out).expect("diag JSON");
    report[0]["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .map(|d| {
            (
                d["code"].as_str().expect("code").to_owned(),
                d["line"].as_u64().expect("line"),
            )
        })
        .collect()
}

/// Run `tcl diag --json` over one `.sslictcl` document written to a scratch
/// file (so the dialect routes by extension, not by content signature), and
/// return its `SSLIC*` rows as `(code, line)` pairs in report order.
fn sslictcl_diag_rows(tag: &str, text: &str) -> Vec<(String, u64)> {
    let scratch = Scratch::new(&format!("sslictcl-{tag}"));
    let path = scratch.write("doc.sslictcl", text);

    let out = run_tcl_allow_failure(&["diag", path.to_str().expect("utf-8 path"), "--json"]);
    let report: serde_json::Value = serde_json::from_slice(&out).expect("diag JSON");
    report[0]["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .filter_map(|d| {
            let code = d["code"].as_str()?;
            code.starts_with("SSLIC")
                .then(|| (code.to_owned(), d["line"].as_u64().expect("line")))
        })
        .collect()
}

/// Write `text` to a scratch `doc.tcl` and run `tcl` with `args` against it
/// (the path is appended last), returning stdout. Mirrors the scratch-file
/// shape [`tcl_diag_rows`] uses, for verbs other than `diag`.
fn run_tcl_on_scratch_doc(tag: &str, text: &str, args: &[&str]) -> Vec<u8> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("tcl-cli-cr-{tag}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let path = dir.join("doc.tcl");
    std::fs::write(&path, text).expect("write document");

    let mut full: Vec<&str> = args.to_vec();
    let path_str = path.to_str().expect("utf-8 path").to_owned();
    full.push(&path_str);
    let out = run_tcl_allow_failure(&full);
    std::fs::remove_dir_all(&dir).ok();
    out
}

/// `tcl find-legacy` (`rust/tcl-cli/src/commands/misc.rs`) must analyse the
/// normalised form of a lone-CR document (#1953): raw, the whole document
/// mis-parses as one command, so a legacy pattern past the first line is
/// either missed entirely or reported at the wrong line.
#[test]
fn find_legacy_reads_a_cr_terminated_tcl_document_the_way_the_editor_does() {
    let lf = "set x 1\nset y [expr $x+1]\nputs $y\n";
    let cr = lf.replace('\n', "\r");

    let lf_out = run_tcl_on_scratch_doc("legacy-lf", lf, &["find-legacy", "--json"]);
    let cr_out = run_tcl_on_scratch_doc("legacy-cr", &cr, &["find-legacy", "--json"]);
    let lf_json: serde_json::Value = serde_json::from_slice(&lf_out).expect("find-legacy JSON");
    let cr_json: serde_json::Value = serde_json::from_slice(&cr_out).expect("find-legacy JSON");

    assert_eq!(
        lf_json["issues"][0]["code"], "W100",
        "the `\\n` form is the reference reading: {lf_json}"
    );
    assert_eq!(
        lf_json["issues"][0]["line"], 2,
        "the unbraced expr sits on line 2: {lf_json}"
    );
    assert_eq!(
        cr_json, lf_json,
        "a lone-CR document must report the same legacy pattern at the same line as its `\\n` twin"
    );
}

/// `tcl minimize` (`rust/tcl-cli/src/commands/minimize.rs`) must reduce the
/// normalised form of a lone-CR document (#1953): raw, the document mis-parses
/// as one command, so the target diagnostic never fires and reduction fails
/// outright rather than reproducing a wrong minimal snippet.
#[test]
fn minimize_reduces_a_cr_terminated_document_the_way_the_editor_does() {
    let lf = "set a 1\nset b 2\nputs $a\n";
    let cr = lf.replace('\n', "\r");

    // `tcl minimize FILE CODE [--json]`: CODE is the final positional
    // argument (`InputArgs::inputs.split_last()`), so the scratch path must
    // come *before* it — `run_tcl_on_scratch_doc` appends its path last and
    // would leave CODE looking like a second input file.
    let minimize = |tag: &str, text: &str| -> Vec<u8> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("tcl-cli-minimize-{tag}-{nanos}"));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let path = dir.join("doc.tcl");
        std::fs::write(&path, text).expect("write document");
        let out = run_tcl_allow_failure(&[
            "minimize",
            path.to_str().expect("utf-8 path"),
            "W211",
            "--json",
        ]);
        std::fs::remove_dir_all(&dir).ok();
        out
    };
    let lf_out = minimize("lf", lf);
    let cr_out = minimize("cr", &cr);
    let lf_json: serde_json::Value = serde_json::from_slice(&lf_out)
        .unwrap_or_else(|e| panic!("minimize JSON: {e}\n{lf_out:?}"));
    let cr_json: serde_json::Value = serde_json::from_slice(&cr_out)
        .unwrap_or_else(|e| panic!("minimize JSON: {e}\n{cr_out:?}"));

    assert_eq!(
        lf_json[0]["reproduces"], true,
        "the `\\n` form is the reference reading: {lf_json}"
    );
    assert_eq!(
        lf_json[0]["source"], "set a 2",
        "W211 minimises to the unused-set alone: {lf_json}"
    );
    assert_eq!(
        cr_json[0]["source"], lf_json[0]["source"],
        "a lone-CR document must minimise to the same reproducer as its `\\n` twin: {cr_json}"
    );
    assert_eq!(cr_json[0]["reproduces"], true);
}

/// `tcl callgraph` (`rust/tcl-cli/src/commands/graphs.rs`, shared by
/// `symbols` / `symbolgraph` / `dataflow` through the same `combine_sources`
/// call) must analyse the normalised form of a lone-CR document (#1953): raw,
/// both `proc` definitions collapse into one mis-parsed command and the whole
/// call graph comes back empty.
#[test]
fn callgraph_reads_a_cr_terminated_tcl_document_the_way_the_editor_does() {
    let lf = "proc foo {} {\n    bar\n}\nproc bar {} {\n    puts hi\n}\nfoo\n";
    let cr = lf.replace('\n', "\r");

    let lf_out = run_tcl_on_scratch_doc("cg-lf", lf, &["callgraph", "--json"]);
    let cr_out = run_tcl_on_scratch_doc("cg-cr", &cr, &["callgraph", "--json"]);
    let lf_json: serde_json::Value = serde_json::from_slice(&lf_out).expect("callgraph JSON");
    let cr_json: serde_json::Value = serde_json::from_slice(&cr_out).expect("callgraph JSON");

    assert_eq!(
        lf_json["nodes"].as_array().expect("nodes").len(),
        2,
        "the `\\n` form is the reference reading: {lf_json}"
    );
    // Neither side names its input file, so the two payloads must agree
    // byte-for-byte once both are read as the analyser reads them.
    assert_eq!(
        cr_json, lf_json,
        "a lone-CR document's call graph must match its `\\n` twin: {cr_json}"
    );
}

/// `tcl diff` (`rust/tcl-cli/src/commands/diff.rs`) must combine and analyse
/// the normalised form of each side (#1953): raw, a lone-CR side mis-parses as
/// one command while its `\n` twin parses as three, so the AST/IR/CFG layers
/// report a structural difference between two documents that are the same
/// script under a different line ending.
#[test]
fn diff_treats_a_lone_cr_document_as_identical_to_its_lf_twin() {
    let lf = "set a 1\nset b 2\nputs $a\n";
    let cr = lf.replace('\n', "\r");

    let out = run_tcl_allow_failure(&[
        "diff",
        "--left-source",
        lf,
        "--right-source",
        &cr,
        "--show",
        "all",
        "--json",
    ]);
    let report: serde_json::Value = serde_json::from_slice(&out).expect("diff JSON");

    assert_eq!(
        report["equal"], true,
        "a lone-CR document and its `\\n` twin carry the same script, so every \
         layer must report equal rather than a spurious structural diff: {report}"
    );
    for layer in ["ast", "ir", "cfg"] {
        assert_eq!(
            report["layers"][layer]["equal"], true,
            "layer `{layer}` must not diverge on line-ending alone: {report}"
        );
    }
}

/// `tcl pkg discover` (`rust/tcl-cli/src/commands/pkg_discover.rs`) must scan
/// the normalised form of a lone-CR document (#1953): raw, the whole document
/// mis-parses as one command, so every `package require` after the first line
/// is invisible to discovery.
#[test]
fn pkg_discover_reads_a_cr_terminated_tcl_document_the_way_the_editor_does() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("tcl-cli-pkg-discover-cr-{nanos}"));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    std::fs::write(
        dir.join("tclpkg.tcl"),
        "package demo-app\nversion 0.1.0\ntcl >=8.6\n",
    )
    .expect("write manifest");

    let lf = "set x 1\nputs $x\npackage require json 1.0\nputs done\npackage require http 2.9\n";
    let cr = lf.replace('\n', "\r");
    std::fs::write(dir.join("lf.tcl"), lf).expect("write lf source");
    std::fs::write(dir.join("cr.tcl"), &cr).expect("write cr source");

    let discover = |source: &str| -> Vec<(String, u64)> {
        let manifest = dir.join("tclpkg.tcl");
        let out = run_tcl_allow_failure(&[
            "pkg",
            "discover",
            dir.join(source).to_str().expect("utf-8 path"),
            "--manifest",
            manifest.to_str().expect("utf-8 path"),
            "--json",
        ]);
        let report: serde_json::Value = serde_json::from_slice(&out)
            .unwrap_or_else(|e| panic!("pkg discover JSON: {e}\n{out:?}"));
        report["requirements"]
            .as_array()
            .expect("requirements array")
            .iter()
            .map(|r| {
                (
                    r["name"].as_str().expect("name").to_owned(),
                    r["line"].as_u64().expect("line"),
                )
            })
            .collect()
    };

    let lf_requirements = discover("lf.tcl");
    let cr_requirements = discover("cr.tcl");
    std::fs::remove_dir_all(&dir).ok();

    assert_eq!(
        lf_requirements,
        vec![("json".to_owned(), 3), ("http".to_owned(), 5)],
        "the `\\n` form is the reference reading"
    );
    assert_eq!(
        cr_requirements, lf_requirements,
        "a lone-CR document must discover the same requirements at the same lines \
         as its `\\n` twin: {cr_requirements:?}"
    );
}

/// `tcl compwasm` (`rust/tcl-cli/src/commands/compile.rs`) must build the
/// `CompilationUnit` from the normalised form of a lone-CR document (#1953):
/// raw, the document mis-parses as one command, so the compiled module never
/// defines the `proc` at all — a mis-parse is not just a wrong report here,
/// it is wrong emitted code.
#[test]
fn compwasm_compiles_a_cr_terminated_document_the_way_the_editor_does() {
    let lf = "proc foo {} {\n    return 1\n}\nputs [foo]\n";
    let cr = lf.replace('\n', "\r");

    let wat_for = |tag: &str, text: &str| -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("tcl-cli-compwasm-{tag}-{nanos}"));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let src_path = dir.join("doc.tcl");
        std::fs::write(&src_path, text).expect("write document");
        let wasm_path = dir.join("out.wasm");
        let wat_path = dir.join("out.wat");
        let output = tcl()
            .args([
                "compwasm",
                src_path.to_str().expect("utf-8 path"),
                "-o",
                wasm_path.to_str().expect("utf-8 path"),
                "--wat-output",
                wat_path.to_str().expect("utf-8 path"),
            ])
            .output()
            .expect("failed to spawn tcl binary");
        assert!(
            output.status.success(),
            "tcl compwasm exited {:?}\nstderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        let wat = std::fs::read_to_string(&wat_path).expect("read WAT output");
        std::fs::remove_dir_all(&dir).ok();
        wat
    };

    let lf_wat = wat_for("lf", lf);
    let cr_wat = wat_for("cr", &cr);

    assert!(
        lf_wat.contains("$::foo"),
        "the `\\n` form is the reference reading and must compile `foo` as a \
         function: {lf_wat}"
    );
    assert_eq!(
        cr_wat, lf_wat,
        "a lone-CR document must compile to the same module as its `\\n` twin"
    );
}

/// The committed `samples/optimiser/` outputs are what the current optimiser
/// produces, byte for byte.
///
/// Without a test comparing them to a real run, committed samples can drift
/// from actual behaviour undetected — e.g. showing an `incr` rewrite the
/// current optimiser declines, or a footer format it no longer emits. The
/// regeneration loop in `samples/optimiser/README.md` is exactly this test,
/// so a pass that changes what any profile emits fails here until the
/// samples are refreshed with it.
#[test]
fn samples_optimiser_profiles_are_regenerated() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = root.join("samples/optimiser/input.tcl");
    let input = input.to_str().expect("the sample path is UTF-8");
    for profile in ["readability", "standard", "full", "aggressive"] {
        let produced = String::from_utf8(run_tcl(&["opt", "--profile", profile, input]))
            .expect("tcl opt emits UTF-8");
        let committed_path = root.join(format!("samples/optimiser/profile_{profile}.tcl"));
        let committed = std::fs::read_to_string(&committed_path)
            .unwrap_or_else(|e| panic!("reading {}: {e}", committed_path.display()));
        assert_eq!(
            produced, committed,
            "samples/optimiser/profile_{profile}.tcl no longer matches the optimiser. \
             Regenerate the four outputs with the loop in samples/optimiser/README.md, \
             and check the per-profile prose there still describes what they show.",
        );
    }
}

/// A scratch directory for one test, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("tcl-cli-{tag}-{nanos}"));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        Self(dir)
    }

    /// Write `text` at `rel` (directories created) and return its path.
    fn write(&self, rel: &str, text: &str) -> PathBuf {
        self.write_bytes(rel, text.as_bytes())
    }

    /// Write `bytes` at `rel` (directories created) and return its path.
    fn write_bytes(&self, rel: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(rel);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
        std::fs::write(&path, bytes).expect("write file");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

/// Run the built `tcl` binary with `args` and `env`, tolerating a non-zero
/// exit, and return its stdout. `env` applies over [`tcl`]'s isolation, so
/// an `XDG_CONFIG_HOME` there names the global layer the run reads.
fn run_tcl_env(args: &[&str], env: &[(&str, &std::ffi::OsStr)]) -> Vec<u8> {
    let mut command = tcl();
    command.args(args);
    for (key, value) in env {
        command.env(key, value);
    }
    command.output().expect("failed to spawn tcl binary").stdout
}

/// `(file label, code)` pairs of a `diag --json` report.
fn diag_codes_by_file(out: &[u8]) -> Vec<(String, String)> {
    let report: serde_json::Value = serde_json::from_slice(out).expect("diag JSON");
    report
        .as_array()
        .expect("report array")
        .iter()
        .flat_map(|file| {
            let label = file["file"].as_str().expect("file").to_owned();
            file["diagnostics"]
                .as_array()
                .expect("diagnostics")
                .iter()
                .map(move |d| (label.clone(), d["code"].as_str().expect("code").to_owned()))
        })
        .collect()
}

/// Every `suppressed` row of a `diag --show-suppressed --json` report,
/// across all files — empty for a file with no `suppressed` key (the flag
/// was not given).
fn diag_suppressed_rows(out: &[u8]) -> Vec<serde_json::Value> {
    let report: serde_json::Value = serde_json::from_slice(out).expect("diag JSON");
    report
        .as_array()
        .expect("report array")
        .iter()
        .flat_map(|file| {
            file.get("suppressed")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default()
        })
        .collect()
}

/// A `while` whose counter the body never touches: W242, the one code the
/// catalogue declares default-off.
const UNPROVABLE_LOOP: &str = "set i 0\nwhile {$i < 3} {\n    puts $i\n}\n";

/// An abstaining document keeps the codes its bytes justify — the integrity
/// code and a bidirectional control, W305, which reads the decoded text as
/// text — as the editor's abstention does (D10 in `git show c6ae07da`). A UTF-16
/// byte-order mark ahead of UTF-8 text is what abstains here; the tail
/// decodes losslessly, so the override it carries is still there to find.
#[test]
fn diag_keeps_a_bidi_control_on_an_abstaining_document() {
    let scratch = Scratch::new("abstain-bidi");
    let no_config = scratch.write("config/.keep", "");
    let xdg = no_config.parent().expect("config dir").as_os_str();
    let env: &[(&str, &std::ffi::OsStr)] = &[("XDG_CONFIG_HOME", xdg)];
    let bom_then = |text: &str| {
        let mut bytes = vec![0xFF, 0xFE];
        bytes.extend_from_slice(text.as_bytes());
        bytes
    };
    let with = scratch.write_bytes("with.tcl", &bom_then("# \u{202E}hidden\nputs hi\n"));
    let without = scratch.write_bytes("without.tcl", &bom_then("# hidden\nputs hi\n"));
    let codes = |path: &std::path::Path| -> Vec<String> {
        let path = path.to_str().expect("utf-8 path");
        diag_codes_by_file(&run_tcl_env(&["diag", "--json", path], env))
            .into_iter()
            .map(|(_, code)| code)
            .collect()
    };
    assert_eq!(
        codes(&with),
        ["W109", "W305"],
        "the integrity code and the bidi control survive abstention"
    );
    assert_eq!(
        codes(&without),
        ["W109"],
        "without the control the integrity code stands alone"
    );
}

/// The catalogue's default-off codes are off for `tcl diag` as they are in
/// the editor, and `--enable` turns one on — the seed is the lowest layer,
/// under every flag (`docs/design/compiler/diagnostic-policy.md`
/// § Configuration).
#[test]
fn diag_seeds_the_default_off_codes_like_the_editor() {
    let scratch = Scratch::new("default-off");
    let no_config = scratch.write("config/.keep", "");
    let xdg = no_config.parent().expect("config dir").as_os_str();
    let env: &[(&str, &std::ffi::OsStr)] = &[("XDG_CONFIG_HOME", xdg)];
    let off = diag_codes_by_file(&run_tcl_env(
        &["diag", "--json", "--source", UNPROVABLE_LOOP],
        env,
    ));
    assert!(
        !off.iter().any(|(_, code)| code == "W242"),
        "W242 is default-off and must not fire unasked: {off:?}"
    );
    let on = diag_codes_by_file(&run_tcl_env(
        &[
            "diag",
            "--json",
            "--enable",
            "W242",
            "--source",
            UNPROVABLE_LOOP,
        ],
        env,
    ));
    assert!(
        on.iter().any(|(_, code)| code == "W242"),
        "`--enable W242` must reach a default-off code: {on:?}"
    );
}

/// The project and global layers resolve per input file (issue #2063): a
/// file under a `.tcl-lsp.ini` that turns W112 off reports none while its
/// sibling from another directory still does. The global `config.ini` is the
/// lowest layer, the flags sit above it, and a project file sits above both.
/// A configuration file only turns codes off (`[diagnostics] disabled = …`),
/// so the flag's `--enable` is what turns a code the global file disabled
/// back on.
#[test]
fn diag_resolves_the_project_and_global_layers_per_input_file() {
    let scratch = Scratch::new("layers");
    let trailing = "set x 1   \nputs $x\n";
    scratch.write("quiet/.tcl-lsp.ini", "[diagnostics]\ndisabled = W112\n");
    let quiet = scratch.write("quiet/nested/a.tcl", trailing);
    let loud = scratch.write("loud/b.tcl", trailing);
    let config = scratch.write("xdg/tcl-lsp/config.ini", "[diagnostics]\ndisabled = W112\n");
    let xdg = config
        .parent()
        .and_then(|p| p.parent())
        .expect("xdg root")
        .as_os_str();
    let empty = scratch.write("empty-xdg/.keep", "");
    let no_global = empty.parent().expect("empty xdg").as_os_str();

    let per_file = diag_codes_by_file(&run_tcl_env(
        &[
            "diag",
            "--json",
            quiet.to_str().unwrap(),
            loud.to_str().unwrap(),
        ],
        &[("XDG_CONFIG_HOME", no_global)],
    ));
    let has = |rows: &[(String, String)], file: &PathBuf, code: &str| {
        rows.iter().any(|(label, c)| {
            label.ends_with(file.file_name().unwrap().to_str().unwrap()) && c == code
        })
    };
    assert!(
        !has(&per_file, &quiet, "W112"),
        "the project file above a.tcl turns W112 off: {per_file:?}"
    );
    assert!(
        has(&per_file, &loud, "W112"),
        "b.tcl sits under no project file and keeps W112: {per_file:?}"
    );

    // The global file reaches both.
    let with_global = diag_codes_by_file(&run_tcl_env(
        &[
            "diag",
            "--json",
            quiet.to_str().unwrap(),
            loud.to_str().unwrap(),
        ],
        &[("XDG_CONFIG_HOME", xdg)],
    ));
    assert!(
        !has(&with_global, &quiet, "W112"),
        "global and project both disable W112 for a.tcl: {with_global:?}"
    );
    assert!(
        !has(&with_global, &loud, "W112"),
        "the global file disables W112 for b.tcl: {with_global:?}"
    );

    // `--enable` overrules the global file, and the project file overrules
    // the flag.
    let enabled = diag_codes_by_file(&run_tcl_env(
        &[
            "diag",
            "--json",
            "--enable",
            "W112",
            quiet.to_str().unwrap(),
            loud.to_str().unwrap(),
        ],
        &[("XDG_CONFIG_HOME", xdg)],
    ));
    assert!(
        has(&enabled, &loud, "W112"),
        "`--enable W112` overrules the global `disabled = W112`: {enabled:?}"
    );
    assert!(
        !has(&enabled, &quiet, "W112"),
        "the project `disabled = W112` overrules `--enable`: {enabled:?}"
    );

    // An inline `--source` has no path, so no project layer: the global
    // file alone decides, and `--enable` in the invocation layer overrules it.
    let inline = diag_codes_by_file(&run_tcl_env(
        &["diag", "--json", "--source", trailing],
        &[("XDG_CONFIG_HOME", xdg)],
    ));
    assert!(
        !inline.iter().any(|(_, code)| code == "W112"),
        "the global layer reaches an inline source: {inline:?}"
    );
    let flagged = diag_codes_by_file(&run_tcl_env(
        &["diag", "--json", "--enable", "W112", "--source", trailing],
        &[("XDG_CONFIG_HOME", xdg)],
    ));
    assert!(
        flagged.iter().any(|(_, code)| code == "W112"),
        "`--enable` sits above the global file: {flagged:?}"
    );
}

/// A project file turns back on a code the global file turned off: an INI
/// layer's contribution is a per-code tri-state, spelled `CODE = true`
/// (`docs/design/contracts/xdg-config.md` § `[diagnostics]`), and the
/// project layer sits above the global one.
#[test]
fn diag_a_project_file_turns_a_code_back_on() {
    let scratch = Scratch::new("turn-back-on");
    let trailing = "set x 1   \nputs $x\n";
    scratch.write("project/.tcl-lsp.ini", "[diagnostics]\nW112 = true\n");
    let inside = scratch.write("project/inside.tcl", trailing);
    let outside = scratch.write("sibling/outside.tcl", trailing);
    let config = scratch.write("xdg/tcl-lsp/config.ini", "[diagnostics]\ndisabled = W112\n");
    let xdg = config
        .parent()
        .and_then(std::path::Path::parent)
        .expect("xdg root")
        .as_os_str();
    let rows = diag_codes_by_file(&run_tcl_env(
        &[
            "diag",
            "--json",
            inside.to_str().unwrap(),
            outside.to_str().unwrap(),
        ],
        &[("XDG_CONFIG_HOME", xdg)],
    ));
    let has = |file: &str, code: &str| {
        rows.iter()
            .any(|(label, c)| label.ends_with(file) && c == code)
    };
    assert!(
        has("inside.tcl", "W112"),
        "the project's `W112 = true` overrules the global disable: {rows:?}"
    );
    assert!(
        !has("outside.tcl", "W112"),
        "outside the project the global disable stands: {rows:?}"
    );
}

/// `--show-suppressed` lists every suppressed finding with its reason
/// (`docs/design/compiler/diagnostic-policy.md` § Adapters, CLI rows); the
/// plain report is unaffected — no `suppressed` key at all.
#[test]
fn diag_show_suppressed_lists_every_hidden_finding_with_its_reason() {
    let fixture = fixtures_dir().join("noqaSuppression.tcl");
    let path = fixture.to_str().unwrap();

    let plain: serde_json::Value =
        serde_json::from_slice(&run_tcl_allow_failure(&["diag", "--json", path]))
            .expect("diag JSON");
    for file in plain.as_array().expect("report array") {
        assert!(
            file.get("suppressed").is_none(),
            "without the flag the JSON carries no `suppressed` key: {file}"
        );
    }

    let rows = diag_suppressed_rows(&run_tcl_allow_failure(&[
        "diag",
        "--json",
        "--show-suppressed",
        path,
    ]));
    let find = |code: &str| {
        rows.iter()
            .find(|r| r["code"] == code)
            .unwrap_or_else(|| panic!("no suppressed {code} row: {rows:?}"))
    };
    let w210 = find("W210");
    assert_eq!(w210["reason"], "inline-directive", "{w210}");
    assert!(
        w210["message"]
            .as_str()
            .is_some_and(|m| m.contains("suppressedByCode")),
        "the W210 on the `suppressedByCode` line: {w210}"
    );
    let s100 = find("S100");
    assert_eq!(s100["reason"], "inline-directive", "{s100}");
    assert!(
        s100["message"]
            .as_str()
            .is_some_and(|m| m.contains("dictValue")),
        "the S100 on the `dictValue` line: {s100}"
    );
}

/// A code a layer disables is a declared gap when nothing computes a
/// finding for it — `--disable` occupies the invocation layer — and the
/// default-off seed (W242 here) is omitted: it is the catalogue's baseline,
/// identical for every file, and listing it on every file buries the answer.
#[test]
fn diag_show_suppressed_lists_a_disabled_analyser_code_as_a_gap() {
    let rows = diag_suppressed_rows(&run_tcl_allow_failure(&[
        "diag",
        "--disable",
        "W210",
        "--show-suppressed",
        "--json",
        "--source",
        "puts $y",
    ]));
    // The optimiser, which `diag` never runs, is one row on every document
    // (D47); the rest is the one code the invocation turned off.
    let (not_run, rows): (Vec<&serde_json::Value>, Vec<&serde_json::Value>) =
        rows.iter().partition(|r| r.get("codes").is_some());
    assert_eq!(not_run.len(), 1, "{not_run:?}");
    assert_eq!(not_run[0]["producer"], "optimiser", "{not_run:?}");
    assert_eq!(not_run[0]["reason"], "optimiser-off", "{not_run:?}");
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["line"], serde_json::Value::Null, "{rows:?}");
    assert_eq!(rows[0]["code"], "W210", "{rows:?}");
    assert_eq!(rows[0]["reason"], "disabled:invocation", "{rows:?}");
    assert!(
        !rows.iter().any(|r| r["code"] == "W242"),
        "the default-off seed is not listed as a gap: {rows:?}"
    );
}

/// The codes only the optimiser emits, which `tcl diag` declares as the one
/// producer it did not run: every catalogued optimisation code but those
/// the compiler checks and the O111 producer emit.
fn codes_only_the_optimiser_emits() -> Vec<String> {
    use tcl_compiler::compiler_checks::DiagCode;
    use tcl_lsp_core::diagnostic_policy::PRODUCED_WITHOUT_THE_OPTIMISER;
    DiagCode::ALL
        .iter()
        .filter(|code| code.is_optimisation() && !PRODUCED_WITHOUT_THE_OPTIMISER.contains(*code))
        .map(ToString::to_string)
        .collect()
}

/// The optimiser `diag` never runs is one `--show-suppressed` row per reason,
/// not one per code (`docs/design/compiler/diagnostic-policy.md` § Adapters):
/// the text row counts the codes, the JSON entry lists them, and a code the
/// compiler checks or the O111 producer emit is in neither, since those
/// producers ran.
#[test]
fn diag_show_suppressed_collapses_the_optimiser_into_one_row() {
    let only_the_optimisers = codes_only_the_optimiser_emits();
    let rows = diag_suppressed_rows(&run_tcl_allow_failure(&[
        "diag",
        "--show-suppressed",
        "--json",
        "--source",
        "set x 1   \nputs $y\n",
    ]));
    let not_run: Vec<&serde_json::Value> =
        rows.iter().filter(|r| r.get("codes").is_some()).collect();
    assert_eq!(not_run.len(), 1, "{rows:?}");
    let row = not_run[0];
    assert_eq!(row["producer"], "optimiser", "{row}");
    assert_eq!(row["reason"], "optimiser-off", "{row}");
    assert_eq!(row["message"], "optimiser not run on this surface", "{row}");
    assert_eq!(row["line"], serde_json::Value::Null, "{row}");
    assert!(row.get("code").is_none(), "{row}");
    assert_eq!(
        row["codes"],
        serde_json::json!(only_the_optimisers),
        "{row}"
    );
    for code in ["O100", "O105", "O106", "O111"] {
        assert!(
            !rows.iter().any(|r| r["code"] == code),
            "{code} is computed without the optimiser, so it is no gap: {rows:?}"
        );
    }

    let output = tcl()
        .args([
            "diag",
            "--show-suppressed",
            "--source",
            "set x 1   \nputs $y\n",
        ])
        .output()
        .expect("failed to spawn tcl binary");
    let rendered = String::from_utf8(output.stdout).expect("utf-8 output");
    let sentence = format!(
        "optimiser not run on this surface ({} codes) [optimiser-off]",
        only_the_optimisers.len()
    );
    assert_eq!(
        rendered
            .lines()
            .filter(|line| line.contains(&sentence))
            .count(),
        1,
        "{rendered}"
    );
    assert_eq!(
        rendered.matches("[optimiser-off]").count(),
        1,
        "no per-code optimiser row: {rendered}"
    );
}

/// An abstaining document declares what `tcl diag` did not run, as any
/// document does: the analyser's skip and the optimiser, each for the
/// reason the policy gives — the abstention, the step that fires first. A
/// UTF-16 byte-order mark ahead of UTF-8 text is what abstains here.
#[test]
fn diag_show_suppressed_declares_what_an_abstaining_document_did_not_run() {
    let scratch = Scratch::new("abstain-declared");
    let bom = scratch.write_bytes("bom.tcl", b"\xFF\xFEset x 1   \nputs $y\n");
    let path = bom.to_str().expect("utf-8 path");

    let output = tcl()
        .args(["diag", "--show-suppressed", "--json", path])
        .output()
        .expect("failed to spawn tcl binary");
    let codes: Vec<String> = diag_codes_by_file(&output.stdout)
        .into_iter()
        .map(|(_, code)| code)
        .collect();
    assert_eq!(codes, ["W109"], "the integrity code alone shows");
    let rows = diag_suppressed_rows(&output.stdout);
    let w242 = rows
        .iter()
        .find(|r| r["code"] == "W242")
        .unwrap_or_else(|| panic!("the analyser's declared skip: {rows:?}"));
    assert_eq!(w242["reason"], "encoding-abstention", "{w242}");
    let not_run: Vec<&serde_json::Value> =
        rows.iter().filter(|r| r.get("codes").is_some()).collect();
    assert_eq!(not_run.len(), 1, "{rows:?}");
    assert_eq!(not_run[0]["producer"], "optimiser", "{rows:?}");
    assert_eq!(not_run[0]["reason"], "encoding-abstention", "{rows:?}");
    assert_eq!(
        not_run[0]["codes"],
        serde_json::json!(codes_only_the_optimiser_emits()),
        "{rows:?}"
    );
    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");
    assert!(stderr.contains("suppressed=2 "), "{stderr}");
}

/// O111 pairs with every W100 the analyser finds (DP8.2); `tcl diag` keeps
/// the optimiser off by policy (D4), so O111 is an `OptimiserOff`
/// suppression here rather than a finding that silently never existed.
#[test]
fn diag_show_suppressed_lists_o111_as_optimiser_off() {
    let src = "set a 1\nset b [expr $a + 1]\n";
    let shown = diag_codes_by_file(&run_tcl_allow_failure(&["diag", "--json", "--source", src]));
    assert!(shown.iter().any(|(_, code)| code == "W100"), "{shown:?}");
    let hidden = diag_suppressed_rows(&run_tcl_allow_failure(&[
        "diag",
        "--show-suppressed",
        "--json",
        "--source",
        src,
    ]));
    let o111 = hidden
        .iter()
        .find(|r| r["code"] == "O111")
        .unwrap_or_else(|| panic!("no suppressed O111 row: {hidden:?}"));
    assert_eq!(o111["reason"], "optimiser-off", "{o111}");
}

/// The text form's `--show-suppressed` rows carry `[reason]`, and a file
/// whose only findings are suppressed still exits `0` — the exit status
/// counts shown problems only.
#[test]
fn diag_show_suppressed_text_rows_keep_the_exit_status() {
    let scratch = Scratch::new("show-suppressed-text");
    let only_suppressed = scratch.write("noqa.tcl", "# noqa: W210\nputs $y\n");
    let output = tcl()
        .args([
            "diag",
            "--show-suppressed",
            only_suppressed.to_str().unwrap(),
        ])
        .output()
        .expect("failed to spawn tcl binary");
    assert!(
        output.status.success(),
        "a document whose only finding is suppressed must exit 0: {output:?}"
    );
    let rendered = String::from_utf8(output.stdout).expect("utf-8 output");
    assert!(
        rendered.contains("hidden") && rendered.contains("[inline-directive]"),
        "{rendered}"
    );
}

/// #2062's own program: `tcl opt --profile full` keeps a dead store a
/// `# noqa: O109` marks, and eliminates it when the marker is gone.
#[test]
fn opt_keeps_a_store_a_noqa_o109_marks() {
    let scratch = Scratch::new("opt-o109");
    let marked = "proc f {} {\n    # noqa: O109\n    set x 1\n    set x 2\n    return $x\n}\n";
    let with = scratch.write("b.tcl", marked);
    let without = scratch.write("plain.tcl", &marked.replace("    # noqa: O109\n", ""));
    let opt = |path: &PathBuf| {
        String::from_utf8(run_tcl(&[
            "opt",
            path.to_str().expect("utf-8 path"),
            "--profile",
            "full",
        ]))
        .expect("utf-8 output")
    };
    let kept = opt(&with);
    assert_eq!(kept, marked, "no rewrite applies to the marked store");
    let removed = opt(&without);
    assert!(
        !removed.contains("set x 1"),
        "the unmarked store goes: {removed}"
    );
    assert!(removed.contains("O109"), "{removed}");
}

/// `tcl opt` writes a tab in a value as a tab, not a run of spaces, when
/// stdout is not a terminal: tab expansion belongs to the highlighted
/// terminal rendering, never to the plain text a redirected or
/// `--no-colour` output carries (issue #2232).
#[test]
fn opt_writes_a_tab_as_a_tab_under_a_redirected_stdout() {
    let scratch = Scratch::new("opt-tab");
    let file = scratch.write("tab.tcl", "set r {a\tb}\nputs $r\n");
    let rendered =
        String::from_utf8(run_tcl(&["opt", file.to_str().expect("utf-8 path")])).unwrap();
    assert!(
        rendered.contains("a\tb"),
        "the tab in `r`'s value must survive a redirected `tcl opt`: {rendered:?}"
    );
}

/// `tcl opt` applies only the rewrites the document's policy shows (issue
/// #2062): a `# noqa` on the command keeps its fold off, a top-of-file
/// `# tcl-lsp: disable=*` keeps every rewrite off, and two inputs whose
/// directives differ are optimised each under its own policy rather than
/// folded into one text where the first file's directive would govern the
/// second.
#[test]
fn opt_applies_only_the_rewrites_the_policy_shows() {
    let scratch = Scratch::new("opt-policy");
    let empty = scratch.write("xdg/.keep", "");
    let xdg = empty.parent().expect("xdg").as_os_str();
    let env: &[(&str, &std::ffi::OsStr)] = &[("XDG_CONFIG_HOME", xdg)];
    // O101 folds the constant expression; a global stays a live store.
    let folding = "set x [expr {1 + 2}]\n";

    let plain = String::from_utf8(run_tcl_env(&["opt", "--source", folding], env)).unwrap();
    assert!(plain.contains("set x 3"), "the control folds: {plain}");

    let marked = format!("# noqa: O101\n{folding}");
    let kept = String::from_utf8(run_tcl_env(&["opt", "--source", &marked], env)).unwrap();
    assert!(
        kept.contains("[expr {1 + 2}]"),
        "a `# noqa` on the command keeps the fold off: {kept}"
    );

    let silenced = scratch.write("silenced.tcl", &format!("# tcl-lsp: disable=*\n{folding}"));
    let open = scratch.write("open.tcl", folding);
    let both = String::from_utf8(run_tcl_env(
        &["opt", silenced.to_str().unwrap(), open.to_str().unwrap()],
        env,
    ))
    .unwrap();
    assert!(
        both.contains("[expr {1 + 2}]"),
        "the silenced file's expression survives: {both}"
    );
    assert!(
        both.contains("set x 3"),
        "the open file's expression folds under its own policy: {both}"
    );
}

/// The owner's ruling: `--profile` is the profile in force over every
/// configuration file, and a file's `[optimiser] profile` — the project's,
/// then the global's — applies only when the flag is omitted
/// (`docs/design/compiler/diagnostic-policy.md` § Configuration).
#[test]
fn opt_a_named_profile_overrules_the_project_file() {
    let scratch = Scratch::new("opt-profile");
    let config = scratch.write("xdg/tcl-lsp/config.ini", "[optimiser]\nprofile = full\n");
    let xdg = config
        .parent()
        .and_then(|p| p.parent())
        .expect("xdg root")
        .as_os_str();
    let env: &[(&str, &std::ffi::OsStr)] = &[("XDG_CONFIG_HOME", xdg)];
    let folding = "set x [expr {1 + 2}]\n";
    scratch.write("proj/.tcl-lsp.ini", "[optimiser]\nprofile = readability\n");
    let inside = scratch.write("proj/fold.tcl", folding);
    let outside = scratch.write("loose/fold.tcl", folding);
    let opt = |args: &[&str]| String::from_utf8(run_tcl_env(args, env)).unwrap();

    let named = opt(&["opt", "--profile", "full", inside.to_str().unwrap()]);
    assert!(
        named.contains("set x 3"),
        "`--profile full` folds over the project's `readability`: {named}"
    );
    let unnamed = opt(&["opt", inside.to_str().unwrap()]);
    assert!(
        unnamed.contains("[expr {1 + 2}]"),
        "with no `--profile` the project's `readability` runs: {unnamed}"
    );

    let global = opt(&["opt", "--profile", "readability", outside.to_str().unwrap()]);
    assert!(
        global.contains("[expr {1 + 2}]"),
        "`--profile readability` wins over the global `full` too: {global}"
    );
    let defaulted = opt(&["opt", outside.to_str().unwrap()]);
    assert!(
        defaulted.contains("set x 3"),
        "outside the project the global `full` is the default: {defaulted}"
    );
}

/// The pass count is the profile in force's: a project `aggressive` runs a
/// second pass that removes the store the first pass's folds left unused
/// (O126), and a named `full` over it is one pass.
#[test]
fn opt_runs_the_passes_of_the_profile_in_force() {
    let scratch = Scratch::new("opt-passes");
    let empty = scratch.write("xdg/.keep", "");
    let xdg = empty.parent().expect("xdg").as_os_str();
    let env: &[(&str, &std::ffi::OsStr)] = &[("XDG_CONFIG_HOME", xdg)];
    scratch.write("proj/.tcl-lsp.ini", "[optimiser]\nprofile = aggressive\n");
    let file = scratch.write(
        "proj/chain.tcl",
        "proc p {} {\n    set a [expr {1 + 2}]\n    set b [expr {$a * 2}]\n    return $b\n}\n",
    );
    let file = file.to_str().unwrap();

    let fixpoint = String::from_utf8(run_tcl_env(&["opt", file], env)).unwrap();
    assert!(
        fixpoint.contains("# O126"),
        "the project's `aggressive` runs a second pass: {fixpoint}"
    );
    let single = String::from_utf8(run_tcl_env(&["opt", "--profile", "full", file], env)).unwrap();
    assert!(
        !single.contains("# O126") && single.contains("# O100"),
        "a named `full` is one pass that still folds: {single}"
    );
}

/// A row's `xdg/tcl-lsp/config.ini`, `proj/.tcl-lsp.ini` and
/// `proj/<name>.tcl` (the program, or the bytes for an abstaining document)
/// — the two truth-table passes' shared scratch layout
/// (`docs/design/compiler/diagnostic-policy.md` § The truth table). Returns
/// the input file's path and the `XDG_CONFIG_HOME` directory the run
/// resolves the global layer under.
fn truth_table_scratch(
    scratch: &Scratch,
    row: &tcl_lsp_core::diagnostic_policy::truth_table::Row,
) -> (String, std::ffi::OsString) {
    use tcl_lsp_core::diagnostic_policy::truth_table::Row;

    let config = scratch.write("xdg/tcl-lsp/config.ini", &Row::ini(row.global));
    let xdg = config
        .parent()
        .and_then(std::path::Path::parent)
        .expect("xdg root")
        .to_owned()
        .into_os_string();
    scratch.write("proj/.tcl-lsp.ini", &Row::ini(row.project));
    let rel = format!("proj/{}.tcl", row.name);
    let file = match row.bytes {
        Some(bytes) => scratch.write_bytes(&rel, bytes),
        None => scratch.write(&rel, row.program),
    };
    let file = file.to_str().expect("utf-8 scratch path").to_owned();
    (file, xdg)
}

/// The slot's `--disable` / `--enable` codes as repeated flag pairs,
/// appended to `args`.
fn push_slot_flags(
    args: &mut Vec<String>,
    flags: &tcl_lsp_core::diagnostic_policy::truth_table::SlotFlags,
) {
    for code in &flags.disable {
        args.push("--disable".to_owned());
        args.push(code.clone());
    }
    for code in &flags.enable {
        args.push("--enable".to_owned());
        args.push(code.clone());
    }
}

/// The diagnostic-policy truth table's diagnostics rows, run through the
/// built `tcl diag --show-suppressed`: each row's observed diagnostics and
/// suppressions hold against what `Surface::Cli` expects of it
/// (`docs/design/compiler/diagnostic-policy.md` § The truth table). One
/// spawn per row; not in the smoke tier.
#[test]
fn truth_table_rows_render_through_tcl_diag() {
    use tcl_compiler::analyser::Severity;
    use tcl_compiler::compiler_checks::DiagCode;
    use tcl_lsp_core::diagnostic_policy::truth_table::{
        Observed, ObservedState, ROWS, Surface, check,
    };

    /// The CLI's `severity` label back to the producers' [`Severity`]: the
    /// one lossy step (`"info"` for both `Info` and `Suggestion`) matches no
    /// row, since only `Error` and `Warning` are ever asserted with `ShownAt`.
    fn severity_of(label: &str) -> Option<Severity> {
        match label {
            "error" => Some(Severity::Error),
            "warning" => Some(Severity::Warning),
            "info" => Some(Severity::Info),
            "hint" => Some(Severity::Hint),
            _ => None,
        }
    }

    /// One file's `diagnostics` and `suppressed` rows (from `--json
    /// --show-suppressed`) as observations: `diag`'s lines are already
    /// 1-based, and a gap's `line` is `null`. A not-run row is one
    /// observation per code it lists.
    fn observed_in(file: &serde_json::Value) -> Vec<Observed> {
        let mut observed: Vec<Observed> = file["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .filter_map(|d| {
                Some(Observed {
                    code: d["code"].as_str()?.parse::<DiagCode>().ok()?,
                    line: d["line"].as_u64().and_then(|n| u32::try_from(n).ok()),
                    state: ObservedState::Shown(d["severity"].as_str().and_then(severity_of)),
                })
            })
            .collect();
        for s in file["suppressed"]
            .as_array()
            .expect("suppressed array (--show-suppressed was given)")
        {
            let reason = s["reason"].as_str().unwrap_or_default().to_owned();
            if let Some(codes) = s["codes"].as_array() {
                let producer = s["producer"].as_str().unwrap_or_default();
                observed.extend(codes.iter().filter_map(|code| {
                    Some(Observed {
                        code: code.as_str()?.parse::<DiagCode>().ok()?,
                        line: None,
                        state: ObservedState::NotRun {
                            producer: producer.to_owned(),
                            reason: reason.clone(),
                        },
                    })
                }));
            } else if let Some(code) = s["code"]
                .as_str()
                .and_then(|code| code.parse::<DiagCode>().ok())
            {
                observed.push(Observed {
                    code,
                    line: s["line"].as_u64().and_then(|n| u32::try_from(n).ok()),
                    state: ObservedState::Suppressed(reason),
                });
            }
        }
        observed
    }

    let mut failures: Vec<String> = Vec::new();
    for row in ROWS.iter().filter(|row| row.runs_on(Surface::Cli)) {
        let scratch = Scratch::new("truth-table-diag");
        let (file, xdg) = truth_table_scratch(&scratch, row);
        let mut args: Vec<String> = vec![
            "diag".to_owned(),
            "--json".to_owned(),
            "--show-suppressed".to_owned(),
            "--dialect".to_owned(),
            row.dialect.to_owned(),
        ];
        push_slot_flags(&mut args, &row.slot_flags());
        args.push(file);
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

        let stdout = run_tcl_env(&arg_refs, &[("XDG_CONFIG_HOME", xdg.as_os_str())]);
        let report: serde_json::Value = serde_json::from_slice(&stdout).unwrap_or_else(|err| {
            panic!(
                "row `{}`: diag --json ({err}): {}",
                row.name,
                String::from_utf8_lossy(&stdout)
            )
        });
        let file_report = report
            .as_array()
            .and_then(|files| files.first())
            .unwrap_or_else(|| panic!("row `{}`: no file in the report: {report}", row.name));

        if let Err(failure) = check(row, Surface::Cli, &observed_in(file_report)) {
            failures.push(failure);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The truth table's rewrite rows, run through the built `tcl opt`: the
/// fold applies exactly when `Surface::CliRewrite` wants it shown
/// (`docs/design/compiler/diagnostic-policy.md` § The truth table). One
/// spawn per row; not in the smoke tier.
#[test]
fn truth_table_rewrite_rows_render_through_tcl_opt() {
    use tcl_lsp_core::diagnostic_policy::truth_table::{
        Observed, ObservedState, ROWS, Surface, check,
    };

    let mut failures: Vec<String> = Vec::new();
    for row in ROWS.iter().filter(|row| row.runs_on(Surface::CliRewrite)) {
        let scratch = Scratch::new("truth-table-opt");
        let (file, xdg) = truth_table_scratch(&scratch, row);
        let flags = row.slot_flags();
        let mut args: Vec<String> = vec!["opt".to_owned()];
        if let Some(profile) = &flags.profile {
            args.push("--profile".to_owned());
            args.push(profile.clone());
        }
        push_slot_flags(&mut args, &flags);
        args.push(file);
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

        let stdout = run_tcl_env(&arg_refs, &[("XDG_CONFIG_HOME", xdg.as_os_str())]);
        let applied = String::from_utf8_lossy(&stdout).contains("set x 3");
        let observed: Vec<Observed> = row
            .expected(Surface::CliRewrite)
            .iter()
            .map(|expect| Observed {
                code: expect.code,
                line: expect.line,
                state: ObservedState::Applied(applied),
            })
            .collect();
        if let Err(failure) = check(row, Surface::CliRewrite, &observed) {
            failures.push(failure);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
