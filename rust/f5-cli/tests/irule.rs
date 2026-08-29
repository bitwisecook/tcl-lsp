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

//! Behavioural tests for the `f5 irule` verb group.
//!
//! Runs the built `f5-query` binary against committed `.irule` / `.conf`
//! fixtures and asserts exit codes and error/usage text for cases that don't
//! depend on comparing full stdout against a captured golden file (input
//! validation and `--help` for every sub). Self-contained: no external tool
//! runs at test time.
//!
//! `pgo` (profile-guided branch-reorder suggestions) is deliberately absent
//! from the `irule` command surface — see the module doc on
//! `f5-cli/src/commands/irule.rs` and issue #1315 — so
//! `pgo_is_not_a_known_subcommand` pins that it is genuinely gone rather
//! than silently reappearing.

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fixture(name: &str) -> String {
    fixtures_dir().join(name).to_string_lossy().into_owned()
}

/// Run `f5-query <args…>`; return `(code, stdout, stderr)`.
fn run(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .args(args)
        .output()
        .expect("failed to spawn f5-query binary");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn extract_rejects_standalone_irule() {
    // Contract: extract only consumes configs.
    let (code, _out, stderr) = run(&[
        "irule",
        "extract",
        &fixture("irule-sample.irule"),
        "/tmp/_x",
    ]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("extract only accepts bigip.conf / SCF / UCS"),
        "stderr: {stderr}"
    );
}

/// Issue #1625: `.irules` is the long spelling the dialect catalogue owns and
/// every editor registers, but this command's three suffix lists were
/// hand-written with only `.irul` and `.irule` — so `foo.irules` was not
/// recognised as a standalone iRule, and `extract` cheerfully tried to parse
/// one as a BIG-IP configuration instead of refusing it.
#[test]
fn extract_rejects_the_long_irules_spelling_too() {
    let (code, _out, stderr) = run(&[
        "irule",
        "extract",
        &fixture("irule-sample.irules"),
        "/tmp/_x",
    ]);
    assert_eq!(code, 2, "stderr: {stderr}");
    assert!(
        stderr.contains("extract only accepts bigip.conf / SCF / UCS"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("irule-sample.irules"),
        "the refusal must name the file it refused; stderr: {stderr}"
    );
}

#[test]
fn no_input_errors_exit_2() {
    let (code, _out, stderr) = run(&["irule", "format"]);
    assert_eq!(code, 2);
    assert_eq!(
        stderr,
        "error: no input provided; pass files, --source, or `-` for stdin\n"
    );
}

/// The source every dialect assertion below formats: TMM accepts the `}{`
/// ghost separator (`if {expr}{body}`), stock Tcl does not.
const GHOST_SEPARATOR_IRULE: &str =
    "when HTTP_REQUEST {\n    if { 1 }{\n        pool p\n    }\n}\n";

#[test]
fn format_resolves_the_irules_profile_for_its_dialect() {
    // Issue #1465: the formatter used to start from `FormatterConfig::default()`
    // and never see `--dialect`, so `f5 irule format` — the primary non-editor
    // entry point for formatting iRules — tokenised them with the Tcl 9 lexer
    // and left `}{` unsplit. The resolved profile now carries the lexer
    // grammar, so the default dialect (f5-irules) splits it.
    let (code, out, stderr) = run(&["irule", "format", "--source", GHOST_SEPARATOR_IRULE]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(out.contains("} {"), "stdout: {out}");
    assert!(!out.contains("}{"), "stdout: {out}");
}

#[test]
fn format_accepts_both_irules_dialect_spellings() {
    // `irules` / `tcl-irule` are aliases of the canonical `f5-irules`, and
    // profile resolution canonicalises them, so all three format alike.
    for dialect in ["f5-irules", "irules", "tcl-irule"] {
        let (code, out, stderr) = run(&[
            "irule",
            "format",
            "--dialect",
            dialect,
            "--source",
            GHOST_SEPARATOR_IRULE,
        ]);
        assert_eq!(code, 0, "{dialect} stderr: {stderr}");
        assert!(out.contains("} {"), "{dialect} stdout: {out}");
        assert!(!out.contains("}{"), "{dialect} stdout: {out}");
    }
}

#[test]
fn format_under_a_core_tcl_dialect_leaves_the_ghost_separator_alone() {
    // The control half: `}{` is an iRules rule, not a Tcl one, so asking for a
    // core release must not synthesise the separator — proving the output
    // above comes from the resolved dialect and not from a hardcoded rewrite.
    let (code, out, stderr) = run(&[
        "irule",
        "format",
        "--dialect",
        "tcl9.0",
        "--source",
        GHOST_SEPARATOR_IRULE,
    ]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(out.contains("}{"), "stdout: {out}");
}

#[test]
fn lint_clean_input_no_findings() {
    // A config / standalone iRule with no issues prints the no-findings line
    // and exits 0.
    let (code, out, _) = run(&["irule", "lint", &fixture("irule-sample.irule")]);
    assert_eq!(code, 0);
    assert_eq!(out, "validate: no findings\n");
}

#[test]
fn context_no_irules_found_exits_1() {
    // A `--rule` filter matching nothing yields no bundles → exit 1.
    let (code, _out, stderr) = run(&[
        "irule",
        "context",
        &fixture("bigip.conf"),
        "--rule",
        "/Common/does-not-exist",
    ]);
    assert_eq!(code, 1);
    assert_eq!(stderr, "error: no iRules found in input\n");
}

#[test]
fn trace_keeps_data_group_from_braced_expression_command() {
    let source = "when HTTP_REQUEST { if {[class match [HTTP::host] equals braced_dg]} { pool selected_pool } }";
    let (code, output, stderr) = run(&[
        "irule",
        "trace",
        "HTTP_REQUEST",
        "--source",
        source,
        "--json",
    ]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        output.contains("\"name\": \"braced_dg\""),
        "the trace must retain the braced-expression data-group: {output}",
    );
}

#[test]
fn trace_uses_the_event_closure_not_the_first_handler_body() {
    let source = concat!(
        "proc helper {} { pool helper_pool }\n",
        "proc dormant {} { pool dormant_pool }\n",
        "when HTTP_REQUEST { pool first_pool; call helper; dormant }\n",
        "when HTTP_REQUEST { pool second_pool }\n",
        "when CLIENT_DATA { pool other_event_pool; call dormant }\n",
    );
    let (code, output, stderr) = run(&[
        "irule",
        "trace",
        "HTTP_REQUEST",
        "--source",
        source,
        "--json",
    ]);
    assert_eq!(code, 0, "stderr: {stderr}");
    for pool in ["helper_pool", "first_pool", "second_pool"] {
        assert!(
            output.contains(&format!("\"name\": \"{pool}\"")),
            "{output}"
        );
    }
    for dormant in ["dormant_pool", "other_event_pool"] {
        assert!(
            !output.contains(&format!("\"name\": \"{dormant}\"")),
            "{output}"
        );
    }
    assert!(
        output.contains("\"commandCount\": 4"),
        "two handler pools, call, and reached helper pool: {output}"
    );
}

// `--help` must work for every sub, all of which are fully implemented.

#[test]
fn help_works_for_all_subs() {
    for sub in [
        "event-order",
        "event-info",
        "lint",
        "trace",
        "extract",
        "format",
        "minify",
        "context",
    ] {
        let (code, out, _) = run(&["irule", sub, "--help"]);
        assert_eq!(code, 0, "irule {sub} --help should exit 0");
        assert!(!out.is_empty(), "irule {sub} --help should print usage");
    }
}

/// `pgo` was advertised in `--help` with a description but always exited 2
/// (issue #1315) — a stub dressed up as a real verb. It is now genuinely
/// absent from the command surface: clap rejects it as an unknown
/// subcommand (exit 2, "unrecognized subcommand"), not the old bespoke
/// "not yet implemented" deferral text.
#[test]
fn pgo_is_not_a_known_subcommand() {
    let (code, _out, stderr) = run(&["irule", "pgo", "--help"]);
    assert_eq!(code, 2, "stderr: {stderr}");
    assert!(
        stderr.contains("unrecognized subcommand") && stderr.contains("pgo"),
        "stderr: {stderr}"
    );
    assert!(
        !stderr.contains("not yet implemented"),
        "must not resurrect the old deferred-verb error text: {stderr}"
    );
}

/// The `irule --help` command listing itself must not advertise `pgo`.
#[test]
fn pgo_not_listed_in_irule_help() {
    let (code, out, _) = run(&["irule", "--help"]);
    assert_eq!(code, 0);
    assert!(
        !out.lines().any(|l| l.trim_start().starts_with("pgo")),
        "irule --help must not list pgo: {out}"
    );
}

// Native-stack safety (issue #996) — `irule minify` calls straight into
// `tcl_lsp_core::minify`, which recurses into `tcl_compiler::analyser`
// (`Analyser::analyse`), on caller-supplied `.irule` file content. Before
// this fix, `f5_cli::run` had no stack-size guard at all (unlike
// `tcl-lsp-server`/`tcl-mcp`/the `tcl` CLI), so deeply nested iRule input
// crashed the process with an uncatchable SIGABRT.

/// `irule minify --aggressive` on a deeply nested iRule (well past the
/// analyser's `MAX_BODY_DEPTH` of 256) must exit cleanly — with a real
/// result or a reported error — not crash the process.
#[test]
fn minify_aggressive_survives_deeply_nested_irule() {
    const DEPTH: usize = 500;
    let mut source = String::from("when HTTP_REQUEST {\n");
    for _ in 0..DEPTH {
        source.push_str("if {1} {\n");
    }
    source.push_str("pool /Common/p\n");
    for _ in 0..DEPTH {
        source.push_str("}\n");
    }
    source.push_str("}\n");

    let path = std::env::temp_dir().join(format!(
        "f5-irule-issue996-deepnest-{}.irule",
        std::process::id()
    ));
    std::fs::write(&path, &source).expect("write deeply nested fixture");

    let (code, out, err) = run(&["irule", "minify", "--aggressive", path.to_str().unwrap()]);
    let _ = std::fs::remove_file(&path);

    assert!(
        code == 0 || code == 2,
        "expected a clean exit (0 or a reported error, 2), got {code}; stdout={out:?} stderr={err:?}"
    );
}
