//! Byte-for-byte tests for the static `f5 query` help surfaces.
//!
//! Runs the built `f5-query` binary with `--help-dsl` / `--help-examples` and
//! asserts stdout matches the captured golden output for `query --help-*`.
//! Self-contained: no external tool runs at test time. These help actions
//! short-circuit before any expression / input is required.

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Run `f5-query query <flag>`, asserting a clean exit 0, returning stdout.
fn run_help(flag: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .arg("query")
        .arg(flag)
        .output()
        .expect("failed to spawn f5-query binary");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(
        code,
        0,
        "f5-query query {flag} exited {code}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn assert_help(flag: &str, golden: &str) {
    let expected = std::fs::read_to_string(fixtures_dir().join(golden)).expect("read golden");
    let actual = run_help(flag);
    assert_eq!(
        actual, expected,
        "`f5 query {flag}` output does not match the golden ({golden})"
    );
}

#[test]
fn help_dsl() {
    assert_help("--help-dsl", "query_help_dsl.golden");
}

#[test]
fn help_examples() {
    assert_help("--help-examples", "query_help_examples.golden");
}

/// `--help-dsl` short-circuits before requiring an expression / input — the
/// flag alone exits 0 even though no positional config was supplied.
#[test]
fn help_dsl_short_circuits_without_inputs() {
    let out = run_help("--help-dsl");
    assert!(out.starts_with("F5 QUERY DSL — GRAMMAR"));
}
