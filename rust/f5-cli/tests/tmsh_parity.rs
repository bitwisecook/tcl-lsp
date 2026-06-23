//! Differential parity tests for the `f5 tmsh` verb (SCF → tmsh emit engine).
//!
//! Runs the built `f5-query` binary against the committed `bigip.conf`
//! fixture and asserts stdout matches the captured golden output.
//! Self-contained: the goldens are committed, so no external tool runs at
//! test time.
//!
//! The delta engine (`emit_tmsh_delta`) has no direct CLI verb (the `tmsh`
//! verb has no `--delta`; delta lives in the shared config-render helper
//! consumed by the config verbs), so its cases drive
//! `tcl_bigip::tmsh_emit::emit_tmsh_delta` directly against the captured
//! `tmsh-delta` golden output.

use std::path::PathBuf;
use std::process::Command;

use tcl_bigip::parser::parse_bigip_conf;
use tcl_bigip::tmsh_emit::{DEFAULT_ORDER, emit_tmsh, emit_tmsh_delta};

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

fn fixture(name: &str) -> String {
    std::fs::read_to_string(fixtures_dir().join(name))
        .unwrap_or_else(|e| panic!("read fixture {name}: {e}"))
}

/// Run `f5-query tmsh <conf> [extra…]`, asserting exit 0, return `(stdout, stderr)`.
fn run_tmsh(extra: &[&str]) -> (String, String) {
    let conf = conf();
    let mut args: Vec<&str> = vec!["tmsh", &conf];
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
fn create_script_matches_python() {
    let (out, err) = run_tmsh(&[]);
    assert_eq!(out, golden("tmsh-create.golden"), "create stdout");
    assert_eq!(err, "tmsh: emitted 28 command(s)\n", "create stderr");
}

#[test]
fn modify_script_matches_python() {
    let (out, _err) = run_tmsh(&["--modify"]);
    assert_eq!(out, golden("tmsh-modify.golden"), "modify stdout");
}

#[test]
fn include_filter_matches_python() {
    let (out, _err) = run_tmsh(&["--include", "pool", "--include", "virtual"]);
    assert_eq!(out, golden("tmsh-include.golden"), "include stdout");
}

#[test]
fn output_file_matches_stdout() {
    let tmp = std::env::temp_dir().join("f5-tmsh-parity-out.tmsh");
    let _ = std::fs::remove_file(&tmp);
    let conf = conf();
    let out_path = tmp.to_string_lossy().into_owned();
    let output = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .args(["tmsh", &conf, "-o", &out_path])
        .output()
        .expect("failed to spawn f5-query binary");
    assert_eq!(output.status.code(), Some(0));
    // `-o` writes the script to the file (no trailing-newline coercion) and
    // emits nothing on stdout.
    assert!(output.stdout.is_empty(), "-o should leave stdout empty");
    let written = std::fs::read_to_string(&tmp).expect("read -o output file");
    assert_eq!(written, golden("tmsh-create.golden"), "-o file content");
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn unknown_include_kind_errors() {
    let conf = conf();
    let output = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .args(["tmsh", &conf, "--include", "bogus"])
        .output()
        .expect("failed to spawn f5-query binary");
    assert_eq!(output.status.code(), Some(2));
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(
        err.contains("unknown kind 'bogus'"),
        "stderr should flag the unknown kind: {err}"
    );
}

#[test]
fn delta_full_matches_python() {
    let before = fixture("tmsh-delta-before.conf");
    let after = fixture("tmsh-delta-after.conf");
    let old_cfg = parse_bigip_conf(&before, "Common");
    let new_cfg = parse_bigip_conf(&after, "Common");
    let script = emit_tmsh_delta(&old_cfg, &new_cfg, &before, &after, &DEFAULT_ORDER);
    assert_eq!(script.text, golden("tmsh-delta.golden"), "delta script");
}

#[test]
fn delta_simple_pool_modify_matches_python() {
    let before = fixture("diff-mod-before.conf");
    let after = fixture("diff-mod-after.conf");
    let old_cfg = parse_bigip_conf(&before, "Common");
    let new_cfg = parse_bigip_conf(&after, "Common");
    let script = emit_tmsh_delta(&old_cfg, &new_cfg, &before, &after, &DEFAULT_ORDER);
    assert_eq!(
        script.text,
        golden("tmsh-delta-simple.golden"),
        "simple delta script"
    );
}

#[test]
fn merge_format_tmsh_matches_python() {
    // The `merge --format tmsh` payoff: the sibling verb now calls the emit
    // engine instead of rejecting the flag.
    let ltm = fixtures_dir().join("merge-ltm.conf");
    let gtm = fixtures_dir().join("merge-gtm.conf");
    let output = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .args([
            "merge",
            &ltm.to_string_lossy(),
            &gtm.to_string_lossy(),
            "--format",
            "tmsh",
        ])
        .output()
        .expect("failed to spawn f5-query binary");
    assert_eq!(output.status.code(), Some(0));
    let out = String::from_utf8_lossy(&output.stdout);
    assert_eq!(out, golden("merge-tmsh.golden"), "merge --format tmsh");
}

#[test]
fn emit_tmsh_library_matches_cli() {
    // The library entry point reproduces the CLI's create script exactly.
    let source = fixture("bigip.conf");
    let cfg = parse_bigip_conf(&source, "Common");
    let script = emit_tmsh(&cfg, &source, &DEFAULT_ORDER, false);
    assert_eq!(script.text, golden("tmsh-create.golden"));
    assert_eq!(script.command_count, 28);
}
