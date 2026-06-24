//! Differential parity tests for the `f5 convert scf2as3` verb (AS3 engine).
//!
//! Runs the built `f5-query` binary against the committed `bigip.conf` fixture
//! and asserts stdout / stderr match the captured golden output for
//! `convert scf2as3 …`. Self-contained: no external tool runs at test time.

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn conf() -> String {
    fixtures_dir()
        .join("bigip.conf")
        .to_string_lossy()
        .into_owned()
}

fn golden(name: &str) -> String {
    std::fs::read_to_string(fixtures_dir().join(name))
        .unwrap_or_else(|e| panic!("read golden {name}: {e}"))
}

/// Run `f5-query convert scf2as3 <conf> [extra…]`, asserting exit 0; return
/// `(stdout, stderr)`.
fn run_convert(extra: &[&str]) -> (String, String) {
    let conf = conf();
    let mut args: Vec<&str> = vec!["convert", "scf2as3", &conf];
    args.extend_from_slice(extra);
    let output = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .args(&args)
        .output()
        .expect("failed to spawn f5-query binary");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(
        code,
        0,
        "f5-query {args:?} exited {code}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn default_tenant_app_matches_python() {
    let (out, err) = run_convert(&[]);
    assert_eq!(out, golden("convert-as3-default.golden"), "default stdout");
    assert!(err.is_empty(), "no report flag → empty stderr: {err}");
}

#[test]
fn custom_tenant_app_matches_python() {
    let (out, _err) = run_convert(&["--tenant", "Prod", "--application", "web"]);
    assert_eq!(out, golden("convert-as3-custom.golden"), "custom stdout");
}

#[test]
fn report_flag_matches_python() {
    let (out, err) = run_convert(&["--report"]);
    // `--report` does not change the declaration (default tenant/app here).
    assert_eq!(
        out,
        golden("convert-as3-default.golden"),
        "report stdout unchanged"
    );
    assert_eq!(
        err,
        golden("convert-as3-report.stderr.golden"),
        "report stderr"
    );
}

#[test]
fn output_file_matches_stdout() {
    let tmp = std::env::temp_dir().join("f5-convert-parity-out.as3.json");
    let _ = std::fs::remove_file(&tmp);
    let conf = conf();
    let out_path = tmp.to_string_lossy().into_owned();
    let output = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .args(["convert", "scf2as3", &conf, "-o", &out_path])
        .output()
        .expect("failed to spawn f5-query binary");
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty(), "-o should leave stdout empty");
    let written = std::fs::read_to_string(&tmp).expect("read -o output file");
    // The file carries the same bytes (including trailing newline) as stdout.
    assert_eq!(
        written,
        golden("convert-as3-default.golden"),
        "-o file content"
    );
    let _ = std::fs::remove_file(&tmp);
}
