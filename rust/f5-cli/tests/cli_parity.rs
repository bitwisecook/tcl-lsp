//! Differential parity tests for the native `f5-query` CLI.
//!
//! Runs the built binary on committed fixtures and asserts stdout matches a
//! golden captured from `python -m tooling.f5.main`. Only verbs whose engine is
//! fully ported (file-I/O-only today) are asserted byte-for-byte; the rest await
//! their BIG-IP engine ports.

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Run f5-query, returning stdout. `diff` exits 1 when there are changes, so
/// callers pass the set of acceptable exit codes.
fn run_f5_codes(args: &[&str], ok_codes: &[i32]) -> Vec<u8> {
    let output = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .args(args)
        .output()
        .expect("failed to spawn f5-query binary");
    let code = output.status.code().unwrap_or(-1);
    assert!(
        ok_codes.contains(&code),
        "f5-query {args:?} exited {code}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn run_f5(args: &[&str]) -> Vec<u8> {
    run_f5_codes(args, &[0])
}

fn assert_diff_matches(args: &[&str], golden: &str) {
    let expected = std::fs::read(fixtures_dir().join(golden)).expect("read golden");
    let actual = run_f5_codes(args, &[0, 1]);
    assert_eq!(
        String::from_utf8_lossy(&actual),
        String::from_utf8_lossy(&expected),
        "f5 diff output does not match the Python CLI ({golden})"
    );
}

#[test]
fn merge_matches_python() {
    let dir = fixtures_dir();
    let a = dir.join("part-a.conf");
    let b = dir.join("part-b.conf");
    let expected = std::fs::read(dir.join("merge-ab.golden")).expect("read golden");
    let actual = run_f5(&["merge", a.to_str().unwrap(), b.to_str().unwrap()]);
    assert_eq!(
        String::from_utf8_lossy(&actual),
        String::from_utf8_lossy(&expected),
        "f5 merge output does not match the Python CLI"
    );
}

#[test]
fn split_then_merge_matches_python() {
    // split a multi-partition fixture into a temp dir, then merge it back, and
    // assert the round-trip equals the golden captured from the Python CLI.
    let dir = fixtures_dir();
    let input = dir.join("split-multi.conf");
    let expected = std::fs::read(dir.join("split-multi.roundtrip.golden")).expect("read golden");

    let out_dir = std::env::temp_dir().join(format!("f5-split-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out_dir);
    run_f5(&["split", input.to_str().unwrap(), out_dir.to_str().unwrap()]);
    let actual = run_f5(&["merge", out_dir.to_str().unwrap()]);
    let _ = std::fs::remove_dir_all(&out_dir);

    assert_eq!(
        String::from_utf8_lossy(&actual),
        String::from_utf8_lossy(&expected),
        "f5 split→merge round-trip does not match the Python CLI"
    );
}

#[test]
fn diff_add_remove_text_matches_python() {
    let dir = fixtures_dir();
    let a = dir.join("part-a.conf");
    let b = dir.join("part-b.conf");
    assert_diff_matches(
        &["diff", a.to_str().unwrap(), b.to_str().unwrap()],
        "diff-addrm.text.golden",
    );
}

#[test]
fn diff_add_remove_json_matches_python() {
    let dir = fixtures_dir();
    let a = dir.join("part-a.conf");
    let b = dir.join("part-b.conf");
    assert_diff_matches(
        &["diff", a.to_str().unwrap(), b.to_str().unwrap(), "--json"],
        "diff-addrm.json.golden",
    );
}

#[test]
fn diff_scalar_modify_matches_python() {
    // Scalar-field modification (load-balancing-mode) — object-list fields
    // (members/records) are excluded here since their display diverges.
    let dir = fixtures_dir();
    let before = dir.join("diff-mod-before.conf");
    let after = dir.join("diff-mod-after.conf");
    assert_diff_matches(
        &["diff", before.to_str().unwrap(), after.to_str().unwrap()],
        "diff-mod.text.golden",
    );
}
