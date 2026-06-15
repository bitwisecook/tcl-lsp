//! Differential parity tests for the **mutating** `f5 query` verb — the
//! field-value edit-plan engine (`=` / `|=` / `+=` / `-=`).
//!
//! Runs the built `f5-query` binary against the committed `bigip.conf`
//! fixture and asserts stdout matches a golden captured from
//! `python -m tooling.f5.main query`. Self-contained: no Python at test time.
//!
//! Goldens embed a `__FIXTURES__` placeholder where the diff's `--- ` /
//! `+++ ` headers carry the on-disk path of the fixtures directory; the test
//! substitutes the real (canonicalised) path before comparing so the goldens
//! stay portable. Only in-scope field-edit cases are covered — identity-field
//! writes and `rename*` are deferred (and separately asserted to error).

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The on-disk path of the canonicalised fixtures dir — the prefix the diff
/// headers embed for each input.
fn fixtures_path() -> String {
    std::fs::canonicalize(fixtures_dir())
        .expect("canonicalize fixtures dir")
        .to_string_lossy()
        .into_owned()
}

fn conf() -> String {
    std::fs::canonicalize(fixtures_dir().join("bigip.conf"))
        .expect("canonicalize bigip.conf")
        .to_string_lossy()
        .into_owned()
}

/// Run `f5-query query ...`, asserting the exit code is in `ok_codes`, and
/// return stdout.
fn run_query(args: &[&str], ok_codes: &[i32]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .arg("query")
        .args(args)
        .output()
        .expect("failed to spawn f5-query binary");
    let code = output.status.code().unwrap_or(-1);
    assert!(
        ok_codes.contains(&code),
        "f5-query query {args:?} exited {code}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Assert the verb's stdout for `args` matches `golden`, expanding the
/// `__FIXTURES__` placeholder to the real fixtures path.
fn assert_query(golden: &str, ok_codes: &[i32], args: &[&str]) {
    let expected = std::fs::read_to_string(fixtures_dir().join(golden))
        .expect("read golden")
        .replace("__FIXTURES__", &fixtures_path());
    let actual = run_query(args, ok_codes);
    assert_eq!(
        actual, expected,
        "f5 query mutation output does not match the Python CLI ({golden})\nargs: {args:?}"
    );
}

/// Assert the verb exits with `code` and writes nothing to stdout (no-op /
/// strict cases).
fn assert_exit_empty(code: i32, args: &[&str]) {
    let actual = run_query(args, &[code]);
    assert_eq!(actual, "", "expected empty stdout for {args:?}");
}

// --- diff preview (default) ---------------------------------------------

#[test]
fn destination_compound_transform_diff() {
    // `|=` against the current field value, transformed by `sub`.
    assert_query(
        "query-mut-desc-diff.golden",
        &[0],
        &[
            r#".ltm.virtual[] | .destination |= sub(., ":[0-9]+$", ":0")"#,
            &conf(),
        ],
    );
}

#[test]
fn pool_monitor_assign_diff() {
    // `=` of a literal onto a scalar (PathRef-typed) field.
    assert_query(
        "query-mut-monitor-diff.golden",
        &[0],
        &[".ltm.pool[] | .monitor = \"/Common/tcp\"", &conf()],
    );
}

#[test]
fn virtual_pool_assign_diff() {
    assert_query(
        "query-mut-pool-diff.golden",
        &[0],
        &[".ltm.virtual[] | .pool = \"/Common/api_pool\"", &conf()],
    );
}

#[test]
fn pool_member_address_assign_diff() {
    // Pool-member field edit via the member's `field_offsets` slots.
    assert_query(
        "query-mut-member-diff.golden",
        &[0],
        &[
            r#".ltm.pool["/Common/web_pool"].members[] | .address = "10.9.9.9""#,
            &conf(),
        ],
    );
}

#[test]
fn virtual_rules_append_diff() {
    // `+=` on a real list field — exercises the compound-block materialiser
    // (overwrite the existing `rules { ... }` block).
    assert_query(
        "query-mut-rules-add-diff.golden",
        &[0],
        &[
            r#".ltm.virtual["/Common/api_vs"] | .rules += "/Common/ssl_info""#,
            &conf(),
        ],
    );
}

// --- --write (rewritten config) -----------------------------------------

#[test]
fn destination_compound_transform_write() {
    assert_query(
        "query-mut-desc-write.golden",
        &[0],
        &[
            r#".ltm.virtual[] | .destination |= sub(., ":[0-9]+$", ":0")"#,
            &conf(),
            "--write",
        ],
    );
}

// --- no-op / strict exit codes ------------------------------------------

#[test]
fn noop_matches_object_but_same_text_exits_1() {
    // Assigns the existing value: matches an object, queues an edit, but the
    // spliced text is identical → exit 1 (tolerant no-op).
    assert_exit_empty(
        1,
        &[
            r#".ltm.pool["/Common/web_pool"] | .monitor = "/Common/my_http_monitor""#,
            &conf(),
        ],
    );
}

#[test]
fn noop_strict_exits_2() {
    assert_exit_empty(
        2,
        &[
            "--strict",
            r#".ltm.pool["/Common/web_pool"] | .monitor = "/Common/my_http_monitor""#,
            &conf(),
        ],
    );
}

#[test]
fn select_nothing_queues_no_edit_exits_0() {
    // The predicate drops every object, so no edit op is queued: the query is
    // not a mutation and flows through the read path → exit 0, empty output.
    assert_exit_empty(
        0,
        &[
            r#".ltm.virtual[] | select(.name == "nope") | .pool = "x""#,
            &conf(),
        ],
    );
}

// --- deferred: identity-field writes error cleanly ----------------------

#[test]
fn identity_field_write_is_rejected() {
    let output = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .arg("query")
        .args([
            r#".ltm.pool["/Common/web_pool"] | .name = "/Common/wp2""#,
            &conf(),
        ])
        .output()
        .expect("spawn");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("identity-field rewrites / rename are not yet supported"),
        "unexpected stderr: {stderr}"
    );
}
