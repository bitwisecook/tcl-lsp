//! Differential parity tests for the `f5 irule` verb group.
//!
//! Runs the built `f5-query` binary against committed `.irule` / `.conf`
//! fixtures and asserts stdout matches goldens captured from
//! `python -m tooling.f5.main irule <sub> …` for the **ported** sub-subcommands
//! (event-order, format, minify, extract). Self-contained: no Python at test
//! time. Also asserts each **deferred** sub (event-info, lint, trace, pgo,
//! context) exits 2 with the expected "not yet ported" message.

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

// ---------------------------------------------------------------------------
// Ported subs — byte-for-byte stdout parity with the Python CLI.
// ---------------------------------------------------------------------------

#[test]
fn event_order_text_matches_python() {
    let (code, out, _) = run(&["irule", "event-order", &fixture("irule-sample.irule")]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-event-order.golden"));
}

#[test]
fn event_order_json_matches_python() {
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
fn event_order_alias_and_config_matches_python() {
    // `eventorder` alias + a bigip.conf input (per-rule sources combined).
    let (code, out, _) = run(&["irule", "eventorder", &fixture("irule-config.conf")]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-event-order-config.golden"));
}

#[test]
fn format_single_matches_python() {
    let (code, out, _) = run(&["irule", "format", &fixture("irule-sample.irule")]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-format.golden"));
}

#[test]
fn format_config_multi_rule_matches_python() {
    // Multi-rule input → per-rule banners on stdout.
    let (code, out, _) = run(&["irule", "fmt", &fixture("irule-config.conf")]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-format-config.golden"));
}

#[test]
fn minify_default_matches_python() {
    let (code, out, _) = run(&["irule", "minify", &fixture("irule-sample.irule")]);
    assert_eq!(code, 0);
    assert_eq!(out, golden("irule-minify.golden"));
}

#[test]
fn minify_aggressive_matches_python() {
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
fn extract_writes_per_rule_files_matching_python() {
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
    // Matches Python: extract only consumes configs.
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

// ---------------------------------------------------------------------------
// Deferred subs — clean exit-2 error naming the missing engine.
// ---------------------------------------------------------------------------

fn assert_deferred(args: &[&str], expect_sub: &str) {
    let (code, _out, stderr) = run(args);
    assert_eq!(code, 2, "{args:?} should exit 2; stderr: {stderr}");
    let expected = format!("error: f5 irule {expect_sub} is not yet ported (requires the ");
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
    // client-side, tcp, FASTHTTP/HTTP profiles, per_request, 1290 commands.
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
    // HTTP_CLASS_FAILED carries EventProps.deprecated=true, but Python's
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

#[test]
fn lint_is_deferred() {
    assert_deferred(&["irule", "lint", &fixture("irule-sample.irule")], "lint");
}

#[test]
fn trace_is_deferred() {
    assert_deferred(
        &[
            "irule",
            "trace",
            "HTTP_REQUEST",
            &fixture("irule-sample.irule"),
        ],
        "trace",
    );
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

#[test]
fn context_is_deferred() {
    assert_deferred(
        &["irule", "context", &fixture("irule-sample.irule")],
        "context",
    );
}

// ---------------------------------------------------------------------------
// `--help` must work for every sub (including the deferred ones), since they
// parse their args before the handler runs.
// ---------------------------------------------------------------------------

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
