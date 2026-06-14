//! Differential parity tests for the native `tcl` CLI.
//!
//! Each test runs the built `tcl` binary on a committed fixture and asserts its
//! stdout matches a golden file captured from the Python CLI
//! (`python -m tooling.tcl.main <verb> ...`). This locks byte-for-byte parity
//! for the verbs whose engines are fully ported; regenerate the `.golden`
//! files from the Python CLI if intended behaviour changes.
//!
//! Verbs gated here: `format`, `minify`, `minify --compact`. (Verbs whose
//! Rust engine is still reaching parity — e.g. `diag`/`validate` via the
//! analyser — are intentionally not asserted byte-for-byte yet.)

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
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

fn assert_matches_golden(args: &[&str], golden: &str) {
    let fixtures = fixtures_dir();
    let golden_path = fixtures.join(golden);
    let expected = std::fs::read(&golden_path)
        .unwrap_or_else(|e| panic!("read golden {}: {e}", golden_path.display()));
    let actual = run_tcl(args);
    assert_eq!(
        String::from_utf8_lossy(&actual),
        String::from_utf8_lossy(&expected),
        "output for `tcl {}` does not match {golden}",
        args.join(" ")
    );
}

#[test]
fn minify_matches_python() {
    let input = fixtures_dir().join("greet.tcl");
    assert_matches_golden(&["minify", input.to_str().unwrap()], "greet.minify.golden");
}

#[test]
fn format_matches_python() {
    let input = fixtures_dir().join("greet.tcl");
    assert_matches_golden(&["format", input.to_str().unwrap()], "greet.format.golden");
}

#[test]
fn minify_compact_matches_python() {
    let input = fixtures_dir().join("greet.tcl");
    assert_matches_golden(
        &["minify", "--compact", input.to_str().unwrap()],
        "greet.minify-compact.golden",
    );
}
