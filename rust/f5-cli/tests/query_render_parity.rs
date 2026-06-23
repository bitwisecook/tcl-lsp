//! Differential parity tests for `f5 query --render NAME` (the renderer
//! plugins: mermaid / gantt / ascii-blocks) and the `--render-opt KEY=VALUE`
//! wiring.
//!
//! Runs the built `f5-query` binary against the committed `bigip.conf`
//! fixture and asserts stdout matches the captured golden output for
//! `query ... --render ...`. Self-contained: no external tool runs at test
//! time. Error cases assert stderr + exit code directly.

use std::path::PathBuf;
use std::process::{Command, Output};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn conf() -> String {
    fixtures_dir()
        .join("bigip.conf")
        .to_string_lossy()
        .into_owned()
}

/// A TSV string literal shaping `(timestamp, label, state)` rows for `gantt`.
/// The outer query is just the string, so the renderer sees one TSV string.
const GANTT_TSV: &str = "\"Jan 01 10:00:00\\t/Common/web\\tDOWN\\n\
     Jan 01 10:30:00\\t/Common/web\\tUP\\n\
     Jan 01 10:15:00\\t/Common/api\\tDOWN\\n\
     Jan 01 11:00:00\\t/Common/api\\tUP\"";

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .arg("query")
        .args(args)
        .output()
        .expect("failed to spawn f5-query binary")
}

/// Assert stdout for `args` matches `golden` and the exit code is in
/// `ok_codes`.
fn assert_stdout(golden: &str, ok_codes: &[i32], args: &[&str]) {
    let expected = std::fs::read_to_string(fixtures_dir().join(golden)).expect("read golden");
    let output = run(args);
    let code = output.status.code().unwrap_or(-1);
    assert!(
        ok_codes.contains(&code),
        "f5-query query {args:?} exited {code}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let actual = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        actual, expected,
        "render output does not match the Python CLI ({golden})\nargs: {args:?}"
    );
}

/// Assert the verb fails with `expected_stderr` and exit code 2 for `args`.
fn assert_error(expected_stderr: &str, args: &[&str]) {
    let output = run(args);
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(code, 2, "expected exit 2 for {args:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr, expected_stderr,
        "render error does not match the Python CLI\nargs: {args:?}"
    );
}

// --- mermaid ------------------------------------------------------------

#[test]
fn mermaid_chain() {
    assert_stdout(
        "render-mermaid.golden",
        &[0],
        &[".ltm.virtual[]", &conf(), "--render", "mermaid"],
    );
}

#[test]
fn mermaid_direction_tb() {
    assert_stdout(
        "render-mermaid-tb.golden",
        &[0],
        &[
            ".ltm.virtual[]",
            &conf(),
            "--render",
            "mermaid",
            "--render-opt",
            "direction=TB",
        ],
    );
}

// --- gantt --------------------------------------------------------------

#[test]
fn gantt_default() {
    assert_stdout(
        "render-gantt.golden",
        &[0],
        &[GANTT_TSV, &conf(), "--render", "gantt"],
    );
}

#[test]
fn gantt_unit_minutes_10() {
    assert_stdout(
        "render-gantt-unit10.golden",
        &[0],
        &[
            GANTT_TSV,
            &conf(),
            "--render",
            "gantt",
            "--render-opt",
            "unit-minutes=10",
        ],
    );
}

// --- ascii-blocks -------------------------------------------------------

#[test]
fn ascii_blocks_flat() {
    assert_stdout(
        "render-ascii-blocks.golden",
        &[0],
        &[
            ".ltm.pool[] | {title: .name}",
            &conf(),
            "--render",
            "ascii-blocks",
        ],
    );
}

#[test]
fn ascii_blocks_nested() {
    assert_stdout(
        "render-ascii-nested.golden",
        &[0],
        &[
            "{title: \"Pools\", rows: [.ltm.pool[] | {title: .name}]}",
            &conf(),
            "--render",
            "ascii-blocks",
        ],
    );
}

#[test]
fn ascii_blocks_square_min_width() {
    assert_stdout(
        "render-ascii-square.golden",
        &[0],
        &[
            ".ltm.pool[] | {title: .name}",
            &conf(),
            "--render",
            "ascii-blocks",
            "--render-opt",
            "style=square",
            "--render-opt",
            "min-width=20",
        ],
    );
}

// --- error cases --------------------------------------------------------

#[test]
fn unknown_renderer() {
    assert_error(
        "error: unknown output mode: nope\n",
        &[".ltm.virtual[]", &conf(), "--render", "nope"],
    );
}

#[test]
fn mermaid_bad_direction() {
    assert_error(
        "error: mermaid: unknown direction 'XX' (expected one of LR, RL, TB, BT)\n",
        &[
            ".ltm.virtual[]",
            &conf(),
            "--render",
            "mermaid",
            "--render-opt",
            "direction=XX",
        ],
    );
}

#[test]
fn gantt_objectref_rows_rejected() {
    assert_error(
        "error: gantt: ObjectRef rows are not supported; project the \
         (timestamp, label, state) fields explicitly via `tsv(...)` or \
         `{timestamp, label, state}`\n",
        &[".ltm.virtual[]", &conf(), "--render", "gantt"],
    );
}

#[test]
fn ascii_blocks_bad_style() {
    assert_error(
        "error: ascii-blocks: unknown style 'fancy' (expected one of ascii, rounded, square)\n",
        &[
            ".ltm.pool[] | {title: .name}",
            &conf(),
            "--render",
            "ascii-blocks",
            "--render-opt",
            "style=fancy",
        ],
    );
}

#[test]
fn malformed_render_opt() {
    assert_error(
        "error: --render-opt expects KEY=VALUE (got 'direction')\n",
        &[
            ".ltm.virtual[]",
            &conf(),
            "--render",
            "mermaid",
            "--render-opt",
            "direction",
        ],
    );
}

#[test]
fn empty_key_render_opt() {
    assert_error(
        "error: --render-opt '=foo': KEY side cannot be empty\n",
        &[
            ".ltm.virtual[]",
            &conf(),
            "--render",
            "mermaid",
            "--render-opt",
            "=foo",
        ],
    );
}
