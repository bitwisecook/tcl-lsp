//! Differential tests for `f5 validate` (alias `lint`).
//!
//! Runs the built `f5-query` binary on committed fixtures and asserts stdout +
//! exit code match the captured golden output for `validate`.
//! Self-contained: no external tool is needed at test time.
//!
//! The lint engine is a *sibling* of the query engine — it walks the BIG-IP
//! model directly (reusing `tcl-bigip`'s model, the `tcl-irules` object-ref
//! walker, and the `tcl-registry` event set) rather than the query DSL. These
//! goldens verify each of the eight built-in rules fires identically across all
//! three formats, plus the `--category` / `--severity` filters, multi-file
//! merge, the no-findings path, and the input-error path.

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Run `f5-query`, asserting the exit code and returning stdout/stderr.
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

/// Assert `f5 validate <args>` matches `golden` on stdout and `code` on exit.
fn assert_golden(args: &[&str], golden: &str, code: i32) {
    let dir = fixtures_dir();
    // Resolve fixture basenames to absolute paths (so `full_path` URIs are
    // stable regardless of cwd; lint output keys on object paths, not URIs, so
    // text/json/sarif are cwd-independent — but the binary still needs to find
    // the files).
    let resolved: Vec<String> = args
        .iter()
        .map(|a| {
            let candidate = dir.join(a);
            if candidate.exists() {
                candidate.to_string_lossy().into_owned()
            } else {
                (*a).to_owned()
            }
        })
        .collect();
    let resolved_args: Vec<&str> = resolved.iter().map(String::as_str).collect();
    let (got_code, stdout, stderr) = run(&resolved_args);
    let expected = std::fs::read_to_string(dir.join(golden)).expect("read golden");
    assert_eq!(
        stdout, expected,
        "stdout mismatch for {args:?} (golden {golden})\nstderr: {stderr}"
    );
    assert_eq!(got_code, code, "exit code mismatch for {args:?}");
}

// Existing corpus fixture (empty-pool + irule-missing-{pool,data-group,
//    snat-pool}) across all three formats.

#[test]
fn bigip_text() {
    assert_golden(&["validate", "bigip.conf"], "validate-bigip.text.golden", 1);
}

#[test]
fn bigip_json() {
    assert_golden(
        &["validate", "bigip.conf", "--format", "json"],
        "validate-bigip.json.golden",
        1,
    );
}

#[test]
fn bigip_sarif() {
    assert_golden(
        &["validate", "bigip.conf", "--format", "sarif"],
        "validate-bigip.sarif.golden",
        1,
    );
}

// Dedicated fixture firing the other six rules: orphan-monitor (warning +
//    /Common/ info), virtual-without-pool, pool-without-monitor,
//    irule-deprecated-command, irule-empty-when, irule-unknown-event.

#[test]
fn rules_text() {
    assert_golden(
        &["validate", "validate-rules.conf"],
        "validate-rules.text.golden",
        1,
    );
}

#[test]
fn rules_json() {
    assert_golden(
        &["validate", "validate-rules.conf", "--format", "json"],
        "validate-rules.json.golden",
        1,
    );
}

#[test]
fn rules_sarif() {
    assert_golden(
        &["validate", "validate-rules.conf", "--format", "sarif"],
        "validate-rules.sarif.golden",
        1,
    );
}

// Filters.

#[test]
fn category_irule() {
    assert_golden(
        &["validate", "validate-rules.conf", "--category", "irule"],
        "validate-rules-cat-irule.text.golden",
        1,
    );
}

#[test]
fn category_config() {
    assert_golden(
        &["validate", "validate-rules.conf", "--category", "config"],
        "validate-rules-cat-config.text.golden",
        1,
    );
}

#[test]
fn severity_info() {
    // Only info findings survive the filter, so the exit code drops to 0.
    assert_golden(
        &["validate", "validate-rules.conf", "--severity", "info"],
        "validate-rules-sev-info.text.golden",
        0,
    );
}

// irule-missing buckets (node + pool), with a resolvable ref that must
//    NOT fire.

#[test]
fn irule_refs_json() {
    assert_golden(
        &["validate", "validate-irule-refs.conf", "--format", "json"],
        "validate-irule-refs.json.golden",
        1,
    );
}

// Multi-file merge (cross-file reference resolution).

#[test]
fn multi_file_json() {
    assert_golden(
        &[
            "validate",
            "bigip.conf",
            "validate-rules.conf",
            "--format",
            "json",
        ],
        "validate-multi.json.golden",
        1,
    );
}

// No findings (clean config) — text + sarif, exit 0.

#[test]
fn clean_text() {
    assert_golden(
        &["validate", "validate-clean.conf"],
        "validate-clean.text.golden",
        0,
    );
}

#[test]
fn clean_sarif() {
    assert_golden(
        &["validate", "validate-clean.conf", "--format", "sarif"],
        "validate-clean.sarif.golden",
        0,
    );
}

// Alias: `lint` == `validate`.

#[test]
fn lint_alias_matches_validate() {
    let dir = fixtures_dir();
    let conf = dir.join("validate-rules.conf");
    let conf = conf.to_str().unwrap();
    let (vc, vout, _) = run(&["validate", conf, "--format", "json"]);
    let (lc, lout, _) = run(&["lint", conf, "--format", "json"]);
    assert_eq!(vout, lout, "lint alias output differs from validate");
    assert_eq!(vc, lc, "lint alias exit code differs from validate");
}

// Input error: missing file → `error: not a file: <path>` on stderr,
//    exit 2 (the OS-error path).

#[test]
fn missing_file_errors() {
    let (code, stdout, stderr) = run(&["validate", "/nonexistent/does-not-exist.conf"]);
    assert_eq!(code, 2, "expected exit 2 on missing input");
    assert_eq!(stdout, "", "no stdout expected on input error");
    assert_eq!(
        stderr, "error: not a file: /nonexistent/does-not-exist.conf\n",
        "stderr mismatch on input error"
    );
}
