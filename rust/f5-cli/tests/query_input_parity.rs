//! Differential parity tests for `f5 query` side-inputs.
//!
//! Covers the `--input-json` / `--input-jsonl` / `--input-csv` /
//! `--input-f5log` / generic `--input KIND` binding flags (which expose a
//! file's parsed content as `$NAME`) and the in-query `json_load` /
//! `jsonl_load` / `csv_load` / `f5log_load` loader builtins.
//!
//! The captured golden output drives the `query` assertions; the suite is
//! self-contained (no external tool runs at test time). Binding goldens embed a
//! `__FIXTURES__` placeholder in place of the absolute fixtures `file://`
//! URI (the per-file banner / JSON envelope embeds it). Loader-builtin
//! goldens are path-independent (single positional config → no banner, and
//! the loader output carries no path), so the test substitutes the real
//! absolute fixture path into the query string at run time.

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The canonicalised fixtures dir (absolute) as a `String`.
fn fixtures_abs() -> String {
    std::fs::canonicalize(fixtures_dir())
        .expect("canonicalize fixtures dir")
        .to_string_lossy()
        .into_owned()
}

/// The `file://` URI of the canonicalised fixtures dir — the prefix the
/// banners / JSON envelope embed.
fn fixtures_uri() -> String {
    format!("file://{}", fixtures_abs())
}

/// Absolute path of a fixture file, as a `String`.
fn fixture(name: &str) -> String {
    fixtures_dir().join(name).to_string_lossy().into_owned()
}

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

/// Assert the verb's stdout for `args` matches `golden` (with `__FIXTURES__`
/// expanded to the real URI).
fn assert_query(golden: &str, ok_codes: &[i32], args: &[&str]) {
    let expected = std::fs::read_to_string(fixtures_dir().join(golden))
        .expect("read golden")
        .replace("__FIXTURES__", &fixtures_uri());
    let actual = run_query(args, ok_codes);
    assert_eq!(
        actual, expected,
        "f5 query output does not match the Python CLI ({golden})\nargs: {args:?}"
    );
}

fn conf() -> String {
    fixture("bigip.conf")
}

// --- binding flags ------------------------------------------------------

#[test]
fn input_json_obj() {
    assert_query(
        "query-input-json-obj.golden",
        &[0],
        &[
            "$d",
            &conf(),
            "--input-json",
            &format!("d={}", fixture("side.json")),
        ],
    );
}

#[test]
fn input_json_field() {
    assert_query(
        "query-input-json-field.golden",
        &[0],
        &[
            "$d.x",
            &conf(),
            "--input-json",
            &format!("d={}", fixture("side.json")),
        ],
    );
}

#[test]
fn input_json_iter() {
    assert_query(
        "query-input-json-iter.golden",
        &[0],
        &[
            "$d.items[] | .name",
            &conf(),
            "--input-json",
            &format!("d={}", fixture("side.json")),
        ],
    );
}

#[test]
fn input_json_select() {
    assert_query(
        "query-input-json-select.golden",
        &[0],
        &[
            "$d.items[] | select(.port > 100) | .name",
            &conf(),
            "--input-json",
            &format!("d={}", fixture("side.json")),
        ],
    );
}

#[test]
fn input_csv_col() {
    assert_query(
        "query-input-csv-col.golden",
        &[0],
        &[
            "$rows[] | .name",
            &conf(),
            "--input-csv",
            &format!("rows={}", fixture("side.csv")),
        ],
    );
}

#[test]
fn input_csv_json() {
    assert_query(
        "query-input-csv-json.golden",
        &[0],
        &[
            "$rows[]",
            &conf(),
            "--input-csv",
            &format!("rows={}", fixture("side.csv")),
            "--json",
        ],
    );
}

#[test]
fn input_csv_headers() {
    assert_query(
        "query-input-csv-headers.golden",
        &[0],
        &[
            "$rows[] | .port",
            &conf(),
            "--input-csv",
            &format!("rows={}:name,port", fixture("side-noheader.csv")),
        ],
    );
}

#[test]
fn input_jsonl_iter() {
    assert_query(
        "query-input-jsonl-iter.golden",
        &[0],
        &[
            "$log[]",
            &conf(),
            "--input-jsonl",
            &format!("log={}", fixture("side.jsonl")),
        ],
    );
}

#[test]
fn input_jsonl_select() {
    assert_query(
        "query-input-jsonl-select.golden",
        &[0],
        &[
            r#"$log[] | select(.severity=="err") | .vs"#,
            &conf(),
            "--input-jsonl",
            &format!("log={}", fixture("side.jsonl")),
        ],
    );
}

#[test]
fn input_f5log_json() {
    assert_query(
        "query-input-f5log-json.golden",
        &[0],
        &[
            "$evt[]",
            &conf(),
            "--input-f5log",
            &format!("evt={}", fixture("side.ltm")),
            "--json",
        ],
    );
}

#[test]
fn input_f5log_select() {
    assert_query(
        "query-input-f5log-select.golden",
        &[0],
        &[
            r#"$evt[] | select(.module=="01230140") | .message"#,
            &conf(),
            "--input-f5log",
            &format!("evt={}", fixture("side.ltm")),
        ],
    );
}

#[test]
fn input_generic_json() {
    assert_query(
        "query-input-generic-json.golden",
        &[0],
        &[
            "$d.x",
            &conf(),
            "--input",
            "json",
            &format!("d={}", fixture("side.json")),
        ],
    );
}

// --- loader builtins (path-independent goldens) -------------------------

/// Assert a loader-builtin query (whose only path is inside the query
/// string, absent from the output) matches a static golden.
fn assert_load(golden: &str, query: &str) {
    let expected = std::fs::read_to_string(fixtures_dir().join(golden)).expect("read golden");
    let actual = run_query(&[query, &conf()], &[0]);
    assert_eq!(
        actual, expected,
        "loader-builtin output mismatch ({golden})"
    );
}

#[test]
fn load_json() {
    assert_load(
        "query-load-json.golden",
        &format!(r#"json_load("{}/side.json").x"#, fixtures_abs()),
    );
}

#[test]
fn load_json_iter() {
    assert_load(
        "query-load-json-iter.golden",
        &format!(
            r#"json_load("{}/side.json").items[] | .name"#,
            fixtures_abs()
        ),
    );
}

#[test]
fn load_csv() {
    assert_load(
        "query-load-csv.golden",
        &format!(r#"csv_load("{}/side.csv") | map(.name)"#, fixtures_abs()),
    );
}

#[test]
fn load_jsonl() {
    assert_load(
        "query-load-jsonl.golden",
        &format!(r#"jsonl_load("{}/side.jsonl")[] | .vs"#, fixtures_abs()),
    );
}

#[test]
fn load_f5log() {
    assert_load(
        "query-load-f5log.golden",
        &format!(
            r#"f5log_load("{}/side.ltm")[] | select(.severity=="err") | .message"#,
            fixtures_abs()
        ),
    );
}

// --- error cases --------------------------------------------------------

/// Run an erroring invocation and assert exit code + stderr substring.
fn assert_error(args: &[&str], code: i32, stderr_contains: &str) {
    let output = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .arg("query")
        .args(args)
        .output()
        .expect("spawn");
    assert_eq!(output.status.code(), Some(code), "args: {args:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(stderr_contains),
        "stderr {stderr:?} missing {stderr_contains:?} (args: {args:?})"
    );
}

#[test]
fn err_bad_binding_identifier() {
    assert_error(
        &["$d.x", &conf(), "--input-json", "bad!name=x"],
        2,
        "is not a valid DSL identifier",
    );
}

#[test]
fn err_unknown_kind() {
    assert_error(
        &[
            "$d.x",
            &conf(),
            "--input",
            "weird",
            &format!("d={}", fixture("side.json")),
        ],
        2,
        "unknown input format 'weird' (registered: csv, f5log, json, jsonl)",
    );
}

#[test]
fn err_missing_file() {
    assert_error(
        &[
            "$d.x",
            &conf(),
            "--input-json",
            &format!("d={}", fixture("does-not-exist.json")),
        ],
        2,
        "not a file",
    );
}

#[test]
fn err_duplicate_name() {
    assert_error(
        &[
            "$d.x",
            &conf(),
            "--input-json",
            &format!("d={}", fixture("side.json")),
            "--input-jsonl",
            &format!("d={}", fixture("side.jsonl")),
        ],
        2,
        "is already bound by a prior --input* flag",
    );
}

#[test]
fn err_load_missing_file() {
    assert_error(
        &[r#"json_load("/no/such/file.json")"#, &conf()],
        2,
        "json_load: file not found: /no/such/file.json",
    );
}

// --- help-inputs --------------------------------------------------------

#[test]
fn help_inputs() {
    let out = run_query(&["--help-inputs"], &[0]);
    assert!(out.starts_with("Registered input formats:\n"));
    for fmt in ["csv", "f5log", "json", "jsonl"] {
        assert!(out.contains(&format!("  {fmt}\n")), "missing format {fmt}");
    }
    assert!(out.contains("Use --input KIND NAME=PATH to bind a file via any registered format.\n"));
}
