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

//! What the transforming verbs read and write: `minimize`, `minify`, the lossy-source refusal and tabs under a redirected stdout.

use super::*;

// When CODE fires on no input, the verb prints `CODE does not fire on any
// input.` to stderr and exits 1.
#[test]
fn minimize_missing_code_errors() {
    let input = fixtures_dir().join("minimize.tcl");
    let output = tcl()
        .args(["minimize", input.to_str().unwrap(), "ZZZ999"])
        .reused_output()
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
        let diag = tcl()
            .args(["diag", "--source", reduced, "--json"])
            .reused_output()
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
fn transformations_refuse_lossy_source_before_writing_artifacts() {
    let scratch = Scratch::new("faithful-source");
    let input = scratch.write_bytes("invalid.tcl", b"proc \xff {} {return HELLO}\n");
    for verb in ["format", "opt", "minify", "dis", "compwasm", "minimize"] {
        let output_path = scratch.0.join(format!("{verb}.out"));
        let sidecar = scratch.0.join(format!("{verb}.sidecar"));
        let mut command = tcl();
        command.args([
            verb,
            input.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
        ]);
        match verb {
            "minimize" => {
                command.arg("W100");
            }
            "minify" => {
                command.args(["--symbol-map", sidecar.to_str().unwrap()]);
            }
            "compwasm" => {
                command.args(["--wat-output", sidecar.to_str().unwrap()]);
            }
            _ => {}
        }
        let output = command.reused_output().expect("spawn tcl");
        assert!(!output.status.success(), "{verb} accepted altered source");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("decoding errors"),
            "{verb}: {output:?}"
        );
        assert!(!output_path.exists(), "{verb} published an output");
        assert!(!sidecar.exists(), "{verb} published a sidecar");
    }
}

#[test]
fn literal_replacement_characters_remain_faithful_transform_input() {
    let scratch = Scratch::new("literal-replacement-character");
    let input = scratch.write("valid.tcl", "proc \u{fffd} {} {return HELLO}\n");
    let output = tcl()
        .args(["format", input.to_str().unwrap()])
        .reused_output()
        .expect("spawn tcl");
    assert!(output.status.success(), "{output:?}");
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains('\u{fffd}')
    );
}

#[test]
fn minify_symbol_map_written_for_plain_minify() {
    // `tcl minify --symbol-map FILE` without `--compact`/`--aggressive` must
    // still create the map file (an empty / identity map), not silently skip
    // it — otherwise a later `unminify-error` fails on a missing path
    // (issue 198).
    let input = fixtures_dir().join("greet.tcl");
    let scratch = Scratch::new("symmap");
    let map = scratch.0.join("symmap.txt");
    let _ = run_tcl(&[
        "minify",
        "--symbol-map",
        map.to_str().unwrap(),
        input.to_str().unwrap(),
    ]);
    assert!(
        map.exists(),
        "plain minify must still write the requested --symbol-map file"
    );
}

/// `tcl opt` writes a tab in a value as a tab, not a run of spaces, when
/// stdout is not a terminal: tab expansion belongs to the highlighted
/// terminal rendering, never to the plain text a redirected or
/// `--no-colour` output carries (issue #2232).
#[test]
fn opt_writes_a_tab_as_a_tab_under_a_redirected_stdout() {
    let scratch = Scratch::new("opt-tab");
    let file = scratch.write("tab.tcl", "set r {a\tb}\nputs $r\n");
    let rendered =
        String::from_utf8(run_tcl(&["opt", file.to_str().expect("utf-8 path")])).unwrap();
    assert!(
        rendered.contains("a\tb"),
        "the tab in `r`'s value must survive a redirected `tcl opt`: {rendered:?}"
    );
}

/// `tcl highlight` writes the source's tabs verbatim when stdout is not a
/// terminal, under `--no-colour`, and as HTML; only the ANSI rendering
/// expands them, counting source columns rather than escape-code bytes
/// (issue #2241).
#[test]
fn highlight_writes_a_tab_as_a_tab_under_a_redirected_stdout() {
    let scratch = Scratch::new("highlight-tab");
    let source = "proc p {} {\n\tset r \"a\tb\"\n\treturn $r\n}\n";
    let file = scratch.write("tab.tcl", source);
    let path = file.to_str().expect("utf-8 path");

    let redirected = String::from_utf8(run_tcl(&["highlight", path])).unwrap();
    assert_eq!(
        redirected, source,
        "a redirected stdout carries the source verbatim"
    );

    let plain = String::from_utf8(run_tcl(&["highlight", "--no-colour", path])).unwrap();
    assert_eq!(plain, source, "`--no-colour` carries the source verbatim");

    let html = String::from_utf8(run_tcl(&["highlight", "--format", "html", path])).unwrap();
    assert!(html.contains("a\tb"), "HTML keeps the tab: {html:?}");

    let ansi = String::from_utf8(run_tcl(&["highlight", "--colour", path])).unwrap();
    assert!(
        !ansi.contains('\t'),
        "the ANSI rendering expands tabs: {ansi:?}"
    );
    assert!(
        ansi.contains("\"a    b\""),
        "the tab stop counts source columns, not escape codes: {ansi:?}"
    );

    // `a\<TAB>{b}` is one word in every Tcl from 8.4 to 9.1: expanding the
    // tab before lexing would split off a braced `{b}` word.
    let escaped = scratch.write("escaped.tcl", "set x a\\\t{b}\n");
    let ansi = String::from_utf8(run_tcl(&[
        "highlight",
        "--colour",
        escaped.to_str().expect("utf-8 path"),
    ]))
    .unwrap();
    assert!(
        ansi.contains("a\\    {b}"),
        "the escaped tab stays inside the word: {ansi:?}"
    );
}
