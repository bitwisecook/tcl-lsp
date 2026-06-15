//! Differential parity tests for the `f5 rename` verb — the thin shell over
//! the token-bounded rename engine (`rename(OLD, NEW)` routed through the
//! query runner).
//!
//! Runs the built `f5-query` binary against the committed `bigip.conf`
//! fixture and asserts **both** streams match goldens captured from
//! `python -m tooling.f5.main rename`. Self-contained: no Python at test time.
//!
//! Goldens embed a `__FIXTURES__` placeholder where the diff's `--- ` / `+++ `
//! headers carry the on-disk path of the fixtures directory; the test
//! substitutes the real (canonicalised) path before comparing so the goldens
//! stay portable. Each case asserts stdout (`<base>.out.golden`) and the
//! stderr `renamed …` report / `warning:` / `error:` line
//! (`<base>.err.golden`).

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The on-disk path of the canonicalised fixtures dir — the prefix the diff
/// headers and the error text embed.
fn fixtures_path() -> String {
    std::fs::canonicalize(fixtures_dir())
        .expect("canonicalize fixtures dir")
        .to_string_lossy()
        .into_owned()
}

/// The canonical absolute path of the fixture config (so the diff headers and
/// any error text embed exactly the `__FIXTURES__` prefix the goldens capture).
fn conf() -> String {
    std::fs::canonicalize(fixtures_dir().join("bigip.conf"))
        .expect("canonicalize bigip.conf")
        .to_string_lossy()
        .into_owned()
}

/// Read a golden, expanding `__FIXTURES__` to the real fixtures path.
fn golden(name: &str) -> String {
    std::fs::read_to_string(fixtures_dir().join(name))
        .unwrap_or_else(|e| panic!("read golden {name}: {e}"))
        .replace("__FIXTURES__", &fixtures_path())
}

/// Run `f5-query rename OLD NEW <conf> [extra…]`, asserting the exit code is in
/// `ok_codes`, and return `(stdout, stderr)`.
fn run_rename(old: &str, new: &str, extra: &[&str], ok_codes: &[i32]) -> (String, String) {
    let conf = conf();
    let mut args: Vec<&str> = vec!["rename", old, new, &conf];
    args.extend_from_slice(extra);
    let output = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .args(&args)
        .output()
        .expect("failed to spawn f5-query binary");
    let code = output.status.code().unwrap_or(-1);
    assert!(
        ok_codes.contains(&code),
        "f5-query {args:?} exited {code}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// Assert a rename reproduces the Python CLI byte-for-byte on both streams,
/// matching `<base>.out.golden` / `<base>.err.golden`.
fn assert_rename(base: &str, old: &str, new: &str, extra: &[&str], ok_codes: &[i32]) {
    let (out, err) = run_rename(old, new, extra, ok_codes);
    assert_eq!(
        out,
        golden(&format!("{base}.out.golden")),
        "{base}: stdout does not match the Python CLI"
    );
    assert_eq!(
        err,
        golden(&format!("{base}.err.golden")),
        "{base}: stderr (renamed report / warning / error) does not match the Python CLI"
    );
}

#[test]
fn real_object_rename_diff() {
    // Default dry-run: a unified diff on stdout, the `renamed …` report on
    // stderr (4 occurrences: the pool header + 3 references).
    assert_rename(
        "rename-verb-real-diff",
        "/Common/web_pool",
        "/Common/web_pool_v2",
        &[],
        &[0],
    );
}

#[test]
fn real_object_rename_write() {
    // `--write`: the rewritten config to stdout, same `renamed …` report on
    // stderr.
    assert_rename(
        "rename-verb-real-write",
        "/Common/web_pool",
        "/Common/web_pool_v2",
        &["--write"],
        &[0],
    );
}

#[test]
fn zero_occurrence_is_noop() {
    // Renaming a non-existent object: no report, no diff, `warning:` + exit 1.
    assert_rename(
        "rename-verb-zero-occ",
        "/Common/nonexistent",
        "/Common/whatever",
        &[],
        &[1],
    );
}

#[test]
fn common_partition_token_is_noop() {
    // `/Common` never appears as a standalone token (always `/Common/…`), so a
    // partition rename through `rename()` is a tolerant no-op: `warning:` +
    // exit 1, identical to Python.
    assert_rename("rename-verb-common-noop", "/Common", "/Tenant_A", &[], &[1]);
}

#[test]
fn cross_partition_visibility_refused() {
    // Moving `/Common/web_pool` to `/Tenant_A` breaks visibility for its
    // `/Common` referrers — the engine refuses with the same `error:` line and
    // exit 2.
    assert_rename(
        "rename-verb-crosspart-refused",
        "/Common/web_pool",
        "/Tenant_A/web_pool",
        &[],
        &[2],
    );
}
