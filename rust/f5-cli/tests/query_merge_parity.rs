//! Differential parity tests for `f5 query --merge`.
//!
//! `--merge` treats every loaded config as one logical namespace:
//! `.ltm.virtual[]` enumerates virtuals from every source, `refs` /
//! `referenced_by` walk references across files, and two sources defining the
//! same `(kind, full-path)` are refused. Each test asserts stdout against the
//! captured golden output for `query --merge ...`; self-contained (no external
//! tool runs at test time).
//!
//! Goldens embed the `__FIXTURES__` placeholder in place of the absolute
//! `file://` URI of the fixtures directory; the test substitutes the real
//! (canonicalised) path before comparing so the goldens stay portable.

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The `file://` URI of the canonicalised fixtures dir — the prefix the
/// merge banner / JSON envelope embeds for each input.
fn fixtures_uri() -> String {
    let canon = std::fs::canonicalize(fixtures_dir()).expect("canonicalize fixtures dir");
    format!("file://{}", canon.to_string_lossy())
}

/// The absolute path to a fixture file as a `String`.
fn fixture(name: &str) -> String {
    fixtures_dir().join(name).to_string_lossy().into_owned()
}

/// Run `f5-query query ...`, returning `(stdout + stderr, exit_code)`. Merge
/// goldens cover both a stdout case (enumeration) and a stderr case (the
/// collision refusal), so we compare the combined stream.
fn run_query(args: &[&str]) -> (String, i32) {
    let output = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .arg("query")
        .args(args)
        .output()
        .expect("failed to spawn f5-query binary");
    let code = output.status.code().unwrap_or(-1);
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    (combined, code)
}

/// Assert the verb's combined output for `args` matches `golden` (with the
/// `__FIXTURES__` placeholder expanded) and that the exit code is acceptable.
fn assert_merge(golden: &str, ok_codes: &[i32], args: &[&str]) {
    let expected = std::fs::read_to_string(fixtures_dir().join(golden))
        .expect("read golden")
        .replace("__FIXTURES__", &fixtures_uri());
    let (actual, code) = run_query(args);
    assert!(
        ok_codes.contains(&code),
        "f5-query query {args:?} exited {code}\noutput: {actual}"
    );
    assert_eq!(
        actual, expected,
        "f5 query --merge output does not match the golden ({golden})\nargs: {args:?}"
    );
}

// --- unified enumeration ------------------------------------------------

#[test]
fn merge_virtual_names() {
    assert_merge(
        "query-merge-vnames.golden",
        &[0],
        &[
            "--merge",
            ".ltm.virtual[].name",
            &fixture("merge-ltm.conf"),
            &fixture("merge-gtm.conf"),
        ],
    );
}

#[test]
fn merge_pool_length() {
    assert_merge(
        "query-merge-pool-length.golden",
        &[0],
        &[
            "--merge",
            "[.ltm.pool[]] | length",
            &fixture("merge-ltm.conf"),
            &fixture("merge-gtm.conf"),
        ],
    );
}

#[test]
fn merge_json_envelope() {
    assert_merge(
        "query-merge-json.golden",
        &[0],
        &[
            "--merge",
            "--json",
            ".ltm.pool[].name",
            &fixture("merge-ltm.conf"),
            &fixture("merge-gtm.conf"),
        ],
    );
}

// --- cross-file references ----------------------------------------------

#[test]
fn merge_referenced_by_cross_file() {
    // `web_pool` (merge-ltm.conf) is referenced by `vs_a` (same file) AND
    // `vs_b` (merge-gtm.conf) — the cross-file reverse walk.
    assert_merge(
        "query-merge-refby.golden",
        &[0],
        &[
            "--merge",
            ".ltm.pool[] | referenced_by(.)",
            &fixture("merge-ltm.conf"),
            &fixture("merge-gtm.conf"),
        ],
    );
}

#[test]
fn merge_refs_cross_file() {
    // `vs_b` (merge-gtm.conf) forward-references `web_pool` (merge-ltm.conf).
    assert_merge(
        "query-merge-refs.golden",
        &[0],
        &[
            "--merge",
            ".ltm.virtual[] | refs(.)",
            &fixture("merge-ltm.conf"),
            &fixture("merge-gtm.conf"),
        ],
    );
}

// --- (kind, full-path) collision refusal --------------------------------

#[test]
fn merge_collision_refused() {
    assert_merge(
        "query-merge-collision.err.golden",
        &[2],
        &[
            "--merge",
            ".ltm.pool[].name",
            &fixture("merge-dup-a.conf"),
            &fixture("merge-dup-b.conf"),
        ],
    );
}
