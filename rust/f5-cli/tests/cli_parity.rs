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

fn run_f5(args: &[&str]) -> Vec<u8> {
    let output = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .args(args)
        .output()
        .expect("failed to spawn f5-query binary");
    assert!(
        output.status.success(),
        "f5-query {args:?} exited {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
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
