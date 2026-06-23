//! Differential parity tests for the read-only `f5 query` verb.
//!
//! Runs the built `f5-query` binary against the committed `bigip.conf`
//! fixture and asserts stdout matches the captured golden output.
//! Self-contained: no external tool runs at test time.
//!
//! Multi-file goldens embed a `__FIXTURES__` placeholder in place of the
//! absolute `file://` URI of the fixtures directory; the test substitutes the
//! real (canonicalised) path before comparing so the goldens stay portable.

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The `file://` URI of the canonicalised fixtures dir — the prefix the
/// multi-file banners / JSON envelope embed for each input.
fn fixtures_uri() -> String {
    let canon = std::fs::canonicalize(fixtures_dir()).expect("canonicalize fixtures dir");
    format!("file://{}", canon.to_string_lossy())
}

/// Run `f5-query query ...`, asserting the exit code is acceptable, returning
/// stdout as a string.
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

/// Assert that the verb's stdout for `args` matches `golden`, with the
/// `__FIXTURES__` placeholder in the golden expanded to the real URI.
fn assert_query(golden: &str, ok_codes: &[i32], args: &[&str]) {
    let expected = std::fs::read_to_string(fixtures_dir().join(golden))
        .expect("read golden")
        .replace("__FIXTURES__", &fixtures_uri());
    let actual = run_query(args, ok_codes);
    assert_eq!(
        actual, expected,
        "f5 query output does not match the golden ({golden})\nargs: {args:?}"
    );
}

/// Path to the single-config fixture as a `&str`, kept alive for the test.
fn conf() -> String {
    fixtures_dir()
        .join("bigip.conf")
        .to_string_lossy()
        .into_owned()
}

// --- core query matrix --------------------------------------------------

#[test]
fn vs_names() {
    assert_query(
        "query-vs-names.golden",
        &[0],
        &[".ltm.virtual[].name", &conf()],
    );
}

#[test]
fn vs_pool() {
    assert_query(
        "query-vs-pool.golden",
        &[0],
        &[".ltm.virtual[] | .pool", &conf()],
    );
}

#[test]
fn pool_names() {
    assert_query(
        "query-pool-names.golden",
        &[0],
        &[".ltm.pool[].name", &conf()],
    );
}

#[test]
fn pool_member_addrs() {
    assert_query(
        "query-pool-member-addrs.golden",
        &[0],
        &[".ltm.pool[] | .members[].address", &conf()],
    );
}

#[test]
fn vs_name_pool_obj() {
    assert_query(
        "query-vs-name-pool-obj.golden",
        &[0],
        &[".ltm.virtual[] | {name, pool}", &conf()],
    );
}

#[test]
fn vs_length() {
    assert_query(
        "query-vs-length.golden",
        &[0],
        &["[.ltm.virtual[]] | length", &conf()],
    );
}

#[test]
fn vs_regex_name() {
    assert_query(
        "query-vs-regex-name.golden",
        &[0],
        &[r#".ltm.virtual["~/Common/www"] | .name"#, &conf()],
    );
}

#[test]
fn vs_dest() {
    assert_query(
        "query-vs-dest.golden",
        &[0],
        &[".ltm.virtual[].destination", &conf()],
    );
}

#[test]
fn select_pool() {
    assert_query(
        "query-select-pool.golden",
        &[0],
        &[
            r#".ltm.virtual[] | select(.pool == "/Common/web_pool") | .name"#,
            &conf(),
        ],
    );
}

// --- output modes -------------------------------------------------------

#[test]
fn mode_json() {
    assert_query(
        "query-mode-json.golden",
        &[0],
        &[".ltm.virtual[].name", &conf(), "--json"],
    );
}

#[test]
fn mode_raw() {
    assert_query(
        "query-mode-raw.golden",
        &[0],
        &[".ltm.virtual[].name", &conf(), "--raw"],
    );
}

#[test]
fn mode_paths() {
    assert_query(
        "query-mode-paths.golden",
        &[0],
        &[".ltm.virtual[]", &conf(), "--paths-only"],
    );
}

#[test]
fn mode_scf() {
    assert_query(
        "query-mode-scf.golden",
        &[0],
        &[".ltm.pool[]", &conf(), "--scf"],
    );
}

#[test]
fn mode_table() {
    assert_query(
        "query-mode-table.golden",
        &[0],
        &[".ltm.virtual[] | {name, pool}", &conf(), "--table"],
    );
}

// --- cookbook examples (read-only, non-`.refs`, fully supported) --------

#[test]
fn ex1_vs_pool() {
    assert_query(
        "query-ex1.golden",
        &[0],
        &[".ltm.virtual[] | .pool", &conf()],
    );
}

#[test]
fn ex2_regex_name() {
    // No `vs_prod_*` virtuals in the fixture — empty output, exit 0.
    assert_query(
        "query-ex2.golden",
        &[0],
        &[r#".ltm.virtual["~/vs_prod_"] | .name"#, &conf()],
    );
}

#[test]
fn ex3_select_irule() {
    assert_query(
        "query-ex3.golden",
        &[0],
        &[
            r#".ltm.virtual[] | select(contains(.rules, "/Common/route_by_host")) | .name"#,
            &conf(),
        ],
    );
}

#[test]
fn ex4_cidr_member() {
    assert_query(
        "query-ex4.golden",
        &[0],
        &[
            r#".ltm.virtual[] | select(any(.pool.members[].address | in_cidr(., "10.0.0.0/8"))) | .name"#,
            &conf(),
        ],
    );
}

#[test]
fn ex14_group_by() {
    assert_query(
        "query-ex14.golden",
        &[0],
        &[
            r#"[.ltm.virtual[]] | group_by(partition(."full-path")) | map({partition: (.[0]."full-path" | partition(.)), count: length})"#,
            &conf(),
        ],
    );
}

#[test]
fn ex19_named_source() {
    assert_query(
        "query-ex19.golden",
        &[0],
        &["$bigip.ltm.virtual[].name", &conf()],
    );
}

#[test]
fn ex20_dupes() {
    assert_query(
        "query-ex20.golden",
        &[0],
        &["[.ltm.virtual[].pool] | dupes", &conf()],
    );
}

#[test]
fn ex21_sort_by() {
    assert_query(
        "query-ex21.golden",
        &[0],
        &[
            "[.ltm.virtual[]] | sort_by(.pool.members | length) | map(.name)",
            &conf(),
        ],
    );
}

#[test]
fn ex22_max_by() {
    assert_query(
        "query-ex22.golden",
        &[0],
        &["[.ltm.pool[]] | max_by(.members | length)", &conf()],
    );
}

#[test]
fn ex24_index() {
    assert_query(
        "query-ex24.golden",
        &[0],
        &["[.ltm.virtual[]] | INDEX(.name)", &conf()],
    );
}

// --- multi-file (banners + JSON envelope) -------------------------------

#[test]
fn multi_file_banner() {
    let a = fixtures_dir()
        .join("part-a.conf")
        .to_string_lossy()
        .into_owned();
    let b = fixtures_dir()
        .join("part-b.conf")
        .to_string_lossy()
        .into_owned();
    assert_query(
        "query-multi-banner.golden",
        &[0],
        &[".ltm.pool[].name", &a, &b],
    );
}

#[test]
fn multi_file_json() {
    let a = fixtures_dir()
        .join("part-a.conf")
        .to_string_lossy()
        .into_owned();
    let b = fixtures_dir()
        .join("part-b.conf")
        .to_string_lossy()
        .into_owned();
    assert_query(
        "query-multi-json.golden",
        &[0],
        &[".ltm.pool[].name", &a, &b, "--json"],
    );
}
