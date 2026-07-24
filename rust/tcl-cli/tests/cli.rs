// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Structural / literal-assertion tests for the native `tcl` CLI.
//!
//! Each test runs the built `tcl` binary and asserts its output against
//! expectations written directly in this file — exit codes, output
//! substrings, or structural checks on parsed JSON — rather than against
//! committed golden/snapshot fixtures.

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Run the built `tcl` binary with `args`, returning captured stdout bytes.
fn run_tcl(args: &[&str]) -> Vec<u8> {
    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(args)
        .output()
        .expect("failed to spawn tcl binary");
    assert!(
        output.status.success(),
        "tcl {args:?} exited {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

#[test]
fn explore_json_emits_the_contract_keys() {
    let out = run_tcl(&["explore", "--source", "set x 1\nputs $x", "--json"]);
    let value: serde_json::Value =
        serde_json::from_slice(&out).expect("explore --json must emit valid JSON");
    let obj = value.as_object().expect("top-level object");
    // A representative spread of views is present.
    for key in [
        "meta",
        "ir",
        "cfgPreSsa",
        "cfgPostSsa",
        "segments",
        "asm",
        "stats",
    ] {
        assert!(obj.contains_key(key), "missing explorer key {key:?}");
    }
}

#[test]
fn explore_summary_lists_views() {
    let out = run_tcl(&["explore", "--source", "set x 1"]);
    let text = String::from_utf8(out).expect("utf-8 summary");
    assert!(text.contains("Compiler explorer summary"));
    assert!(text.contains("ir:"));
}

#[test]
fn explore_text_renders_box_drawing_trees() {
    let out = run_tcl(&[
        "explore",
        "--source",
        "set x 1\nputs $x",
        "--text",
        "--show",
        "ir",
        "--no-colour",
    ]);
    let text = String::from_utf8(out).expect("utf-8 text render");
    assert!(text.contains("=== ir ==="), "section header present");
    assert!(
        text.contains("├── ") || text.contains("└── "),
        "box-drawing tree connectors present"
    );
    assert!(!text.contains('\x1b'), "no ANSI escapes with --no-colour");
}

// When CODE fires on no input, the verb prints `CODE does not fire on any
// input.` to stderr and exits 1.
#[test]
fn minimize_missing_code_errors() {
    let input = fixtures_dir().join("minimize.tcl");
    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(["minimize", input.to_str().unwrap(), "ZZZ999"])
        .output()
        .expect("failed to spawn tcl binary");
    assert_eq!(output.status.code(), Some(1), "exit code");
    assert!(output.stdout.is_empty(), "no stdout on the no-fire path");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "ZZZ999 does not fire on any input.\n",
    );
}

// Property test (the task's explicit requirement): the reduced reproducer must
// still fire CODE. We assert it two ways — the verb's own `reproduces` flag in
// the JSON, and independently by re-running `tcl diag` on the reduced source
// and confirming CODE is present. This is the invariant that makes a minimised
// snippet a valid bug-report repro regardless of how far ddmin reduced.
#[test]
fn minimize_reduced_output_still_fires() {
    let input = fixtures_dir().join("minimize.tcl");
    let out = run_tcl(&["minimize", input.to_str().unwrap(), "W100", "--json"]);
    let value: serde_json::Value =
        serde_json::from_slice(&out).expect("minimize --json must emit valid JSON");
    let items = value.as_array().expect("top-level array");
    assert!(!items.is_empty(), "W100 fires, so there is a result");
    for item in items {
        assert_eq!(
            item["reproduces"],
            serde_json::Value::Bool(true),
            "the verb reports the reduction reproduces"
        );
        let reduced = item["source"].as_str().expect("source string");
        // Independently confirm the analyser still fires W100 on the reduced
        // snippet via `tcl diag --json`. `diag` exits 1 when it finds a
        // problem-severity diagnostic (W100 is an error), so we read its stdout
        // directly rather than through the success-asserting `run_tcl`.
        let diag = Command::new(env!("CARGO_BIN_EXE_tcl"))
            .args(["diag", "--source", reduced, "--json"])
            .output()
            .expect("failed to spawn tcl binary")
            .stdout;
        let report: serde_json::Value =
            serde_json::from_slice(&diag).expect("diag --json must emit valid JSON");
        let fires = report
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|f| f["diagnostics"].as_array())
            .flatten()
            .any(|d| d["code"] == serde_json::Value::String("W100".to_owned()));
        assert!(fires, "reduced source {reduced:?} must still fire W100");
    }
}

#[test]
fn minify_symbol_map_written_for_plain_minify() {
    // `tcl minify --symbol-map FILE` without `--compact`/`--aggressive` must
    // still create the map file (an empty / identity map), not silently skip
    // it — otherwise a later `unminify-error` fails on a missing path
    // (issue 198).
    let input = fixtures_dir().join("greet.tcl");
    let tmp = std::env::temp_dir().join(format!(
        "tcl-cli-symmap-{}-{}.txt",
        std::process::id(),
        line!()
    ));
    let _ = std::fs::remove_file(&tmp);
    let _ = run_tcl(&[
        "minify",
        "--symbol-map",
        tmp.to_str().unwrap(),
        input.to_str().unwrap(),
    ]);
    assert!(
        tmp.exists(),
        "plain minify must still write the requested --symbol-map file"
    );
    let _ = std::fs::remove_file(&tmp);
}
