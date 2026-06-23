//! Differential parity tests for the **mutating** `f5 query` verb — the
//! field-value edit-plan engine (`=` / `|=` / `+=` / `-=`) and the token-bounded
//! rename engine (identity-field writes + the `rename*` builtins).
//!
//! Runs the built `f5-query` binary against the committed `bigip.conf`
//! fixture and asserts stdout matches the captured golden output for `query`.
//! Self-contained: no external tool runs at test time.
//!
//! Goldens embed a `__FIXTURES__` placeholder where the diff's `--- ` /
//! `+++ ` headers (and any error text) carry the on-disk path of the fixtures
//! directory; the test substitutes the real (canonicalised) path before
//! comparing so the goldens stay portable. Rename cases assert **both**
//! streams — stdout (`.out.golden`) and the stderr `renamed …` reports /
//! `error:` line (`.err.golden`).

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

/// Run the verb and capture both streams plus the exit code, asserting the
/// code is in `ok_codes`.
fn run_query_both(args: &[&str], ok_codes: &[i32]) -> (String, String) {
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
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// Read a golden, expanding `__FIXTURES__` to the real fixtures path.
fn golden(name: &str) -> String {
    std::fs::read_to_string(fixtures_dir().join(name))
        .unwrap_or_else(|e| panic!("read golden {name}: {e}"))
        .replace("__FIXTURES__", &fixtures_path())
}

/// Assert a rename query reproduces the captured golden output byte-for-byte on
/// **both** stdout (the diff / `--write` payload) and stderr (the `renamed …`
/// reports / `error:` line), matching `<base>.out.golden` / `<base>.err.golden`.
///
/// The fixture is passed as its canonical absolute path so the diff headers and
/// any error text embed exactly the prefix the goldens capture (under
/// `__FIXTURES__`).
fn assert_rename(base: &str, ok_codes: &[i32], extra: &[&str]) {
    let conf = conf();
    let mut args: Vec<&str> = Vec::new();
    args.push(extra[0]); // the query expression
    args.push(&conf);
    args.extend_from_slice(&extra[1..]);
    let (out, err) = run_query_both(&args, ok_codes);
    assert_eq!(
        out,
        golden(&format!("{base}.out.golden")),
        "{base}: stdout does not match the Python CLI"
    );
    assert_eq!(
        err,
        golden(&format!("{base}.err.golden")),
        "{base}: stderr (renamed reports / error) does not match the Python CLI"
    );
}

// --- rename engine: identity-field writes + rename* builtins ------------

#[test]
fn rename_identity_field_write_diff() {
    // Cookbook #6 — identity-field write routes through `rename_object`,
    // rewriting the header + every reference; the `renamed …` report lands on
    // stderr.
    assert_rename(
        "rename-identity-diff",
        &[0],
        &[r#".ltm.pool["/Common/web_pool"].name = "/Common/web_pool_v2""#],
    );
}

#[test]
fn rename_builtin_diff() {
    assert_rename(
        "rename-builtin-diff",
        &[0],
        &[r#"rename("/Common/web_pool","/Common/web_pool_renamed")"#],
    );
}

#[test]
fn rename_builtin_write() {
    assert_rename(
        "rename-builtin-write",
        &[0],
        &[
            r#"rename("/Common/web_pool","/Common/web_pool_renamed")"#,
            "--write",
        ],
    );
}

#[test]
fn rename_prefix_cascade_diff() {
    // `rename_prefix` — token-bounded prefix cascade (covers headers, node
    // refs, pool-member identifiers, and references).
    assert_rename(
        "rename-prefix-diff",
        &[0],
        &[r#"rename_prefix("/Common/web","/Tenant_A/web")"#],
    );
}

#[test]
fn rename_cookbook11_with_partition_diff() {
    // Cookbook #11 — `.name |= with_partition(., "Tenant_A")` per-object
    // identity rewrite; emits one `renamed …` report per matched pool.
    assert_rename(
        "rename-cookbook11-diff",
        &[0],
        &[r#".ltm.pool["~^/Common/"] | .name |= with_partition(., "Tenant_A")"#],
    );
}

#[test]
fn rename_partition_common_refused() {
    // Cookbook #10 shape — both engines refuse renaming `/Common`; assert the
    // identical `error:` line and exit code 2.
    assert_rename(
        "rename-partition-refused",
        &[2],
        &[r#"rename_partition("Common","Tenant_A")"#],
    );
}

#[test]
fn rename_cross_partition_visibility_refused() {
    // Moving `/Common/web_pool` to `/Tenant_A` would break visibility for its
    // `/Common` referrers — both engines refuse with the same message.
    assert_rename(
        "rename-crosspart-refused",
        &[2],
        &[r#"rename("/Common/web_pool","/Tenant_A/web_pool")"#],
    );
}

#[test]
fn rename_zero_occurrence_is_noop() {
    // Tolerant `rename()` of a non-existent object: no report, no diff, exit 1.
    assert_rename(
        "rename-zero-occ",
        &[1],
        &[r#"rename("/Common/nonexistent","/Common/whatever")"#],
    );
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

// --- cross-file edits ---------------------------------------------------

#[test]
fn cross_file_edit_targets_named_source() {
    // A mutation reaching another loaded source via `$name` (here `$xfb`,
    // while the per-file runner iterates `xfa` then `xfb`) must apply against
    // that source — not fail with "no source loaded". The diff lands on the
    // named file, byte-identical to the captured golden output.
    let xfa = std::fs::canonicalize(fixtures_dir().join("xfa.conf"))
        .expect("canonicalize xfa.conf")
        .to_string_lossy()
        .into_owned();
    let xfb = std::fs::canonicalize(fixtures_dir().join("xfb.conf"))
        .expect("canonicalize xfb.conf")
        .to_string_lossy()
        .into_owned();
    let out = run_query(
        &[r#"$xfb.ltm.pool[].monitor = "/Common/tcp""#, &xfa, &xfb],
        &[0],
    );
    assert_eq!(
        out,
        golden("query-crossfile-edit.golden"),
        "cross-file edit diff does not match the Python CLI"
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
