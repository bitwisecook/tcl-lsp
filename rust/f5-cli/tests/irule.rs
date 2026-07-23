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

//! Differential tests for the `f5 irule` verb group.
//!
//! Runs the built `f5-query` binary against committed `.irule` / `.conf`
//! fixtures and asserts stdout matches the captured golden output for
//! `irule <sub> …` for the implemented sub-subcommands (event-order,
//! event-info, lint, context, trace, format, minify, extract). Self-contained:
//! no external tool runs at test time. Also asserts the unimplemented `pgo` sub
//! exits 2 with the expected error message.

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fixture(name: &str) -> String {
    fixtures_dir().join(name).to_string_lossy().into_owned()
}

fn golden(name: &str) -> String {
    std::fs::read_to_string(fixtures_dir().join(name))
        .unwrap_or_else(|e| panic!("read golden {name}: {e}"))
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

// Implemented subs — byte-for-byte stdout against the captured golden output.

#[test]
fn event_order_text() {
    let (code, out, _) = run(&["irule", "event-order", &fixture("irule-sample.irule")]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-event-order.golden"));
}

#[test]
fn event_order_json() {
    let (code, out, _) = run(&[
        "irule",
        "event-order",
        &fixture("irule-sample.irule"),
        "--json",
    ]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-event-order.json.golden"));
}

#[test]
fn event_order_alias_and_config() {
    // `eventorder` alias + a bigip.conf input (per-rule sources combined).
    let (code, out, _) = run(&["irule", "eventorder", &fixture("irule-config.conf")]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-event-order-config.golden"));
}

#[test]
fn format_single() {
    let (code, out, _) = run(&["irule", "format", &fixture("irule-sample.irule")]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-format.golden"));
}

#[test]
fn format_config_multi_rule() {
    // Multi-rule input → per-rule banners on stdout.
    let (code, out, _) = run(&["irule", "fmt", &fixture("irule-config.conf")]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-format-config.golden"));
}

#[test]
fn minify_default() {
    let (code, out, _) = run(&["irule", "minify", &fixture("irule-sample.irule")]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-minify.golden"));
}

#[test]
fn minify_aggressive() {
    let (code, out, _) = run(&[
        "irule",
        "min",
        "--aggressive",
        &fixture("irule-sample.irule"),
    ]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-minify-aggressive.golden"));
}

#[test]
fn extract_writes_per_rule_files() {
    let out_dir = std::env::temp_dir().join("f5-irule-extract-parity");
    let _ = std::fs::remove_dir_all(&out_dir);
    let out_arg = out_dir.to_string_lossy().into_owned();

    let (code, stdout, stderr) =
        run(&["irule", "extract", &fixture("irule-config.conf"), &out_arg]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.is_empty(), "extract writes only to stderr");
    assert_eq!(
        stderr,
        format!("extracted 2 iRule(s) to {}\n", out_dir.display())
    );

    let r1 = std::fs::read_to_string(out_dir.join("Common__r1.tcl")).expect("r1 file");
    let r2 = std::fs::read_to_string(out_dir.join("Common__sub__r2.tcl")).expect("r2 file");
    assert_eq!(r1, golden("irule-extract-r1.golden"));
    assert_eq!(r2, golden("irule-extract-r2.golden"));

    let _ = std::fs::remove_dir_all(&out_dir);
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

#[test]
fn no_input_errors_exit_2() {
    let (code, _out, stderr) = run(&["irule", "format"]);
    assert_eq!(code, 2);
    assert_eq!(
        stderr,
        "error: no input provided; pass files, --source, or `-` for stdin\n"
    );
}

// Unimplemented subs — clean exit-2 error naming the missing engine.

fn assert_deferred(args: &[&str], expect_sub: &str) {
    let (code, _out, stderr) = run(args);
    assert_eq!(code, 2, "{args:?} should exit 2; stderr: {stderr}");
    let expected = format!("error: f5 irule {expect_sub} is not yet implemented (requires the ");
    assert!(
        stderr.starts_with(&expected) && stderr.ends_with(" engine)\n"),
        "{args:?} stderr: {stderr}"
    );
}

/// Assert `f5 irule event-info EVENT [--json]` matches the golden and exits
/// `expect_code` (0 known, 1 unknown).
fn assert_event_info(base: &str, event: &str, expect_code: i32) {
    let (code, out, _) = run(&["irule", "event-info", event]);
    assert_eq!(code, expect_code, "{base}: text exit code");
    assert_eq!(
        out,
        golden(&format!("irule-eventinfo-{base}.text.golden")),
        "{base}: text stdout"
    );
    let (jcode, jout, _) = run(&["irule", "event-info", event, "--json"]);
    assert_eq!(jcode, expect_code, "{base}: json exit code");
    assert_eq!(
        jout,
        golden(&format!("irule-eventinfo-{base}.json.golden")),
        "{base}: json stdout"
    );
}

#[test]
fn event_info_http_request() {
    // client-side, tcp, FASTHTTP/HTTP profiles, per_request, 1099 commands.
    assert_event_info("http-request", "HTTP_REQUEST", 0);
}

#[test]
fn event_info_rule_init() {
    // global, no transport (null), no profiles, init multiplicity.
    assert_event_info("rule-init", "RULE_INIT", 0);
}

#[test]
fn event_info_both_sides_dual_transport() {
    // client-side and server-side, tcp/udp transport.
    assert_event_info("lb-selected", "LB_SELECTED", 0);
}

#[test]
fn event_info_client_accepted() {
    assert_event_info("client-accepted", "CLIENT_ACCEPTED", 0);
}

#[test]
fn event_info_props_deprecated_still_reports_no() {
    // HTTP_CLASS_FAILED carries EventProps.deprecated=true, but the
    // `when`-argument-value path reports `deprecated: no` — so do we.
    assert_event_info("class-failed", "HTTP_CLASS_FAILED", 0);
}

#[test]
fn event_info_unknown_event_exits_1() {
    assert_event_info("unknown", "BOGUS_EVENT", 1);
}

#[test]
fn event_info_event_name_is_upcased() {
    // A lowercase query resolves to the same known event (exit 0); output is
    // byte-identical to the upper-case HTTP_REQUEST form.
    assert_event_info("lowercase", "http_request", 0);
}

#[test]
fn event_info_alias_routes_to_handler() {
    // The `eventinfo` alias produces the same output as `event-info`.
    let (code, out, _) = run(&["irule", "eventinfo", "HTTP_REQUEST"]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-eventinfo-http-request.text.golden"));
}

// lint — byte-for-byte stdout for `irule lint`
// (reuses the `tcl-bigip::lint` engine + the `f5 validate` formatters; only the
// four irule-category rules run). Severity-based exit codes: error→2, warn→1.

#[test]
fn lint_config_all_irule_rules_text() {
    // A bigip.conf exercising every irule rule (deprecated-command ×2,
    // unknown-event, empty-when): 3 warnings + 1 info → exit 1.
    let (code, out, _) = run(&["irule", "lint", &fixture("validate-rules.conf")]);
    assert_eq!(code, 1);
    assert_eq!(out, golden("irule-lint-rules.text.golden"));
}

#[test]
fn lint_config_all_irule_rules_json() {
    let (code, out, _) = run(&["irule", "lint", &fixture("validate-rules.conf"), "--json"]);
    assert_eq!(code, 1);
    assert_eq!(out, golden("irule-lint-rules.json.golden"));
}

#[test]
fn lint_missing_object_refs_json() {
    // iRule referencing a non-existent pool + node → irule-missing-* warnings.
    let (code, out, _) = run(&[
        "irule",
        "lint",
        &fixture("validate-irule-refs.conf"),
        "--json",
    ]);
    assert_eq!(code, 1);
    assert_eq!(out, golden("irule-lint-refs.json.golden"));
}

#[test]
fn lint_standalone_irule_synthesises_single_rule() {
    // A standalone `.irule` is linted as a synthetic single-rule config at
    // `/<stem>` (here `/irule-lint-standalone`).
    let (code, out, _) = run(&["irule", "lint", &fixture("irule-lint-standalone.irule")]);
    assert_eq!(code, 1);
    assert_eq!(out, golden("irule-lint-standalone.text.golden"));
}

#[test]
fn lint_standalone_irule_json() {
    let (code, out, _) = run(&[
        "irule",
        "lint",
        &fixture("irule-lint-standalone.irule"),
        "--json",
    ]);
    assert_eq!(code, 1);
    assert_eq!(out, golden("irule-lint-standalone.json.golden"));
}

#[test]
fn lint_inline_source_synthesises_inline_rule() {
    // `--source` snippets are linted as a synthetic rule at `/inline_<n>`.
    let (code, out, _) = run(&[
        "irule",
        "lint",
        "--source",
        "when HTTP_REQUEST { X509::extensions $cert }",
    ]);
    assert_eq!(code, 1);
    assert_eq!(out, golden("irule-lint-source.text.golden"));
}

#[test]
fn lint_severity_filter_text() {
    // `--severity info` keeps only the single info finding → exit 0.
    let (code, out, _) = run(&[
        "irule",
        "lint",
        &fixture("validate-rules.conf"),
        "--severity",
        "info",
    ]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-lint-sev-info.text.golden"));
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
fn pgo_is_deferred() {
    assert_deferred(
        &[
            "irule",
            "pgo",
            "--profile",
            "/dev/null",
            &fixture("irule-sample.irule"),
        ],
        "pgo",
    );
}

// context — byte-for-byte stdout for `irule context`
// (the `tcl-bigip::irule_context` engine: reference walk + one-hop
// transitive expansion + source slices; JSON / Tcl-flavoured text).

#[test]
fn context_full_config_all_sections_text() {
    // Exercises every section: pool, data-group, persistence, snat-pool,
    // profile, monitor, node (transitive), plus unresolved refs + slices.
    let (code, out, _) = run(&["irule", "context", &fixture("irule-context-full.conf")]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-context-full.text.golden"));
}

#[test]
fn context_full_config_all_sections_json() {
    let (code, out, _) = run(&[
        "irule",
        "context",
        &fixture("irule-context-full.conf"),
        "--json",
    ]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-context-full.json.golden"));
}

#[test]
fn context_realistic_bigip_text() {
    // Multi-rule config; transitive pool→node/monitor expansion + real slices.
    let (code, out, _) = run(&["irule", "context", &fixture("bigip.conf")]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-context-bigip.text.golden"));
}

#[test]
fn context_realistic_bigip_json() {
    let (code, out, _) = run(&["irule", "context", &fixture("bigip.conf"), "--json"]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-context-bigip.json.golden"));
}

#[test]
fn context_missing_object_refs_json() {
    let (code, out, _) = run(&[
        "irule",
        "context",
        &fixture("validate-irule-refs.conf"),
        "--json",
    ]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-context-refs.json.golden"));
}

#[test]
fn context_no_transitive_json() {
    // `--no-transitive` drops the pool→node/monitor expansion.
    let (code, out, _) = run(&[
        "irule",
        "context",
        &fixture("bigip.conf"),
        "--no-transitive",
        "--json",
    ]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-context-notrans.json.golden"));
}

#[test]
fn context_standalone_irule_text() {
    // A standalone `.irule` is bundled as a synthetic single-rule config.
    let (code, out, _) = run(&["irule", "context", &fixture("irule-lint-standalone.irule")]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-context-standalone.text.golden"));
}

#[test]
fn context_inline_source_json() {
    let (code, out, _) = run(&[
        "irule",
        "context",
        "--source",
        "when HTTP_REQUEST { pool /Common/web_pool }",
        "--json",
    ]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-context-source.json.golden"));
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

// trace — byte-for-byte stdout for `irule trace EVENT`
// (purely static: `when EVENT {…}` block-match + balanced-brace
// slice + command/object-reference extraction; no VM).

#[test]
fn trace_bigip_http_request_text() {
    let (code, out, _) = run(&["irule", "trace", "HTTP_REQUEST", &fixture("bigip.conf")]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-trace-bigip.text.golden"));
}

#[test]
fn trace_bigip_http_request_json() {
    let (code, out, _) = run(&[
        "irule",
        "trace",
        "HTTP_REQUEST",
        &fixture("bigip.conf"),
        "--json",
    ]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-trace-bigip.json.golden"));
}

#[test]
fn trace_full_config_all_reference_kinds_text() {
    let (code, out, _) = run(&[
        "irule",
        "trace",
        "HTTP_REQUEST",
        &fixture("irule-context-full.conf"),
    ]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-trace-full.text.golden"));
}

#[test]
fn trace_full_config_all_reference_kinds_json() {
    // resolved + unresolved refs across pool/persistence/snat-pool/profile/node.
    let (code, out, _) = run(&[
        "irule",
        "trace",
        "HTTP_REQUEST",
        &fixture("irule-context-full.conf"),
        "--json",
    ]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-trace-full.json.golden"));
}

#[test]
fn trace_event_name_is_case_insensitive() {
    // `http_request` matches `when HTTP_REQUEST` (event-name match is case-insensitive);
    // the `event` field echoes the query as typed, so the golden differs only
    // in that one string from the upper-case run.
    let (code, out, _) = run(&[
        "irule",
        "trace",
        "http_request",
        &fixture("bigip.conf"),
        "--json",
    ]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-trace-lowercase.json.golden"));
}

#[test]
fn trace_no_matching_event_exits_1() {
    let (code, out, _) = run(&["irule", "trace", "TOTALLY_FAKE", &fixture("bigip.conf")]);
    assert_eq!(code, 1);
    assert_eq!(out, golden("irule-trace-nomatch.text.golden"));
}

// `--help` must work for every sub (including the unimplemented ones), since
// they parse their args before the handler runs.

#[test]
fn help_works_for_all_subs() {
    for sub in [
        "event-order",
        "event-info",
        "lint",
        "trace",
        "pgo",
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

// ===========================================================================
// Native-stack safety (issue #996) — `irule minify` calls straight into
// `tcl_lsp_core::minify`, which recurses into `tcl_compiler::analyser`
// (`Analyser::analyse`), on caller-supplied `.irule` file content. Before
// this fix, `f5_cli::run` had no stack-size guard at all (unlike
// `tcl-lsp-server`/`tcl-mcp`/the `tcl` CLI), so deeply nested iRule input
// crashed the process with an uncatchable SIGABRT.
// ===========================================================================

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
