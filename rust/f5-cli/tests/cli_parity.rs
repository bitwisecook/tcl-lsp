//! Differential parity tests for the native `f5-query` CLI.
//!
//! Runs the built binary on committed fixtures and asserts stdout matches the
//! captured golden output. Only verbs whose engine is implemented
//! (file-I/O-only for now) are asserted byte-for-byte.

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
        "f5 diff output does not match the golden ({golden})"
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
        "f5 merge output does not match the golden"
    );
}

#[test]
fn split_then_merge_matches_python() {
    // split a multi-partition fixture into a temp dir, then merge it back, and
    // assert the round-trip equals the captured golden output.
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
        "f5 split→merge round-trip does not match the golden"
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
fn diff_tmsh_input_matches_python() {
    // tmsh-script input is normalised to SCF before parsing (the `to_scf` path),
    // so `tmsh create/modify` headers compare against the real objects.
    let dir = fixtures_dir();
    let a = dir.join("diff-a.tmsh");
    let b = dir.join("diff-b.tmsh");
    assert_diff_matches(
        &["diff", a.to_str().unwrap(), b.to_str().unwrap()],
        "diff-tmsh.text.golden",
    );
}

#[test]
fn explain_virtual_text_matches_python() {
    let conf = fixtures_dir().join("bigip.conf");
    assert_diff_matches(
        &[
            "explain",
            "virtual",
            "/Common/www_vs",
            conf.to_str().unwrap(),
        ],
        "explain-virtual.text.golden",
    );
}

#[test]
fn explain_virtual_json_matches_python() {
    let conf = fixtures_dir().join("bigip.conf");
    assert_diff_matches(
        &[
            "explain",
            "virtual",
            "/Common/www_vs",
            conf.to_str().unwrap(),
            "--json",
        ],
        "explain-virtual.json.golden",
    );
}

/// Assert that `f5 extract`'s stdout matches the captured golden output,
/// optionally with `F5_UCS_PASSPHRASE` set for an encrypted archive.
fn assert_extract_matches(args: &[&str], passphrase: Option<&str>, golden: &str) {
    let expected = std::fs::read(fixtures_dir().join(golden)).expect("read golden");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_f5-query"));
    cmd.args(args);
    if let Some(p) = passphrase {
        cmd.env("F5_UCS_PASSPHRASE", p);
    }
    let output = cmd.output().expect("failed to spawn f5-query binary");
    assert!(
        output.status.success(),
        "f5-query {args:?} failed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&expected),
        "f5 extract output does not match the golden ({golden})"
    );
}

#[test]
fn extract_plain_ucs_matches_python() {
    let ucs = fixtures_dir().join("sample.ucs");
    assert_extract_matches(
        &["extract", ucs.to_str().unwrap()],
        None,
        "extract-default.scf.golden",
    );
}

#[test]
fn extract_plain_ucs_include_extras_matches_python() {
    let ucs = fixtures_dir().join("sample.ucs");
    assert_extract_matches(
        &["extract", "--include-extras", ucs.to_str().unwrap()],
        None,
        "extract-extras.scf.golden",
    );
}

#[test]
fn extract_encrypted_ucs_matches_python() {
    // OpenPGP-symmetric (gpg --symmetric) archive decrypted purely in Rust;
    // passphrase supplied via the F5_UCS_PASSPHRASE environment variable.
    let ucs = fixtures_dir().join("sample-encrypted.ucs");
    assert_extract_matches(
        &["extract", ucs.to_str().unwrap()],
        Some("correct horse battery staple"),
        "extract-encrypted.scf.golden",
    );
}

#[test]
fn explain_reads_ucs_matches_python() {
    // The UCS input layer is wired into the model-reading verbs: `f5 explain`
    // accepts a `.ucs` (transparently extracted to SCF) exactly like a `.conf`.
    let ucs = fixtures_dir().join("sample.ucs");
    assert_diff_matches(
        &["explain", "auto", "/Common/vs_web", ucs.to_str().unwrap()],
        "explain-ucs.text.golden",
    );
}

#[test]
fn diff_reads_ucs_matches_python() {
    // `f5 diff` accepts `.ucs` inputs on either side; diffing an archive
    // against itself yields no changes, byte-identical to the captured golden output.
    let ucs = fixtures_dir().join("sample.ucs");
    assert_diff_matches(
        &["diff", ucs.to_str().unwrap(), ucs.to_str().unwrap()],
        "diff-ucs-self.text.golden",
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
