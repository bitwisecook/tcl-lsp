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

//! Native port of `tests/lsp_e2e/test_diagnostic_matrix_e2e.py`.
//!
//! Data-driven diagnostic matrix: a must-fire / must-stay-silent pair per code.
//! Every pair is ground-truthed against the reference server; the `silent`
//! snippet is the *minimal* edit that removes the defect, so each pair also
//! proves the diagnostic is specific.
//!
//! The pytest suite parametrises a single positive test and a single negative
//! test over a `_MATRIX` of `_Case` rows. Rust integration tests can't
//! parametrise the same way, so each matrix row is unrolled into its own
//! `#[test]` for the fire case and the silent case.


use crate::common::{Lsp, unique_uri};

use serde_json::Value;
use std::collections::BTreeSet;

/// One matrix row: a code with a source that MUST raise it and the corrected
/// source that MUST NOT.
struct Case {
    code: &'static str,
    fire: &'static str,
    silent: &'static str,
    note: &'static str,
}

/// The matrix, one row per code (mirrors the pytest `_MATRIX`).
const MATRIX: &[Case] = &[
    Case {
        code: "E002",
        fire: "set\n",
        silent: "set x 1\nputs $x\n",
        note: "wrong arg count (too few)",
    },
    Case {
        code: "E003",
        fire: "string length a b c\n",
        silent: "string length abc\n",
        note: "too many arguments to a subcommand",
    },
    Case {
        code: "W100",
        fire: "proc p {a} {\n    if $a { puts x }\n}\n",
        silent: "proc p {a} {\n    if {$a} { puts x }\n}\n",
        note: "unbraced expr",
    },
    Case {
        code: "W110",
        fire: "proc p {x} {\n    if {$x == \"foo\"} { return 1 }\n    return 0\n}\n",
        silent: "proc p {x} {\n    if {$x eq \"foo\"} { return 1 }\n    return 0\n}\n",
        note: "== string comparison (use eq)",
    },
    Case {
        code: "W211",
        fire: "proc p {} {\n    set x 1\n    return 0\n}\n",
        silent: "proc p {} {\n    set x 1\n    return $x\n}\n",
        note: "variable set but never read",
    },
    Case {
        code: "W220",
        fire: "proc p {} {\n    set x 1\n    set x 2\n    return $x\n}\n",
        silent: "proc p {} {\n    set x 1\n    puts $x\n    set x 2\n    return $x\n}\n",
        note: "dead store (overwritten before read)",
    },
    Case {
        code: "W210",
        fire: "proc p {x} {\n    if {$x} {\n        set y 1\n    }\n    return $y\n}\n",
        silent: "proc p {x} {\n    if {$x} {\n        set y 1\n    } else {\n        set y 2\n    }\n    return $y\n}\n",
        note: "read of a possibly-unset variable",
    },
    Case {
        code: "W302",
        fire: "catch {error e}\n",
        silent: "catch {error e} result\n",
        note: "catch without a result variable",
    },
    Case {
        code: "I230",
        fire: "proc p {} {\n    if {1} { return 1 }\n    return 0\n}\n",
        silent: "proc p {cond} {\n    if {$cond} { return 1 }\n    return 0\n}\n",
        note: "constant condition",
    },
];

/// The set of `code` strings carried by `diags` (mirrors Python `_codes`).
fn codes(diags: &[Value]) -> BTreeSet<String> {
    diags
        .iter()
        .map(|d| match d.get("code") {
            Some(Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
            None => "None".to_owned(),
        })
        .collect()
}

/// A diagnostic's `code`, stringified.
fn code_str(d: &Value) -> String {
    match d.get("code") {
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => "None".to_owned(),
    }
}

/// The `test_code_fires_on_defect` body for a single case.
fn assert_fires(case: &Case) {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, case.fire);
    let matching: Vec<&Value> = diags.iter().filter(|d| code_str(d) == case.code).collect();
    assert!(
        !matching.is_empty(),
        "{} ({}) must fire on:\n{}\ngot {:?}",
        case.code,
        case.note,
        case.fire,
        codes(&diags)
    );
    // Severity is a valid LSP value, and an `E`-prefixed code is an Error.
    let sev = matching[0].get("severity").and_then(Value::as_i64);
    assert!(
        matches!(sev, Some(1..=4)),
        "{}: invalid severity {sev:?}",
        case.code
    );
    if case.code.starts_with('E') {
        assert_eq!(sev, Some(1), "{} is an error code but severity={sev:?}", case.code);
    }
}

/// The `test_code_silent_on_corrected_form` body for a single case.
fn assert_silent(case: &Case) {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, case.silent);
    assert!(
        !codes(&diags).contains(case.code),
        "{} ({}) must NOT fire on the corrected form:\n{}\ngot {:?}",
        case.code,
        case.note,
        case.silent,
        codes(&diags)
    );
}

fn case_for(code: &str) -> &'static Case {
    MATRIX
        .iter()
        .find(|c| c.code == code)
        .unwrap_or_else(|| panic!("no matrix row for {code}"))
}

// -- fire cases ----------------------------------------------------------

#[test]
fn e002_fires_on_defect() {
    assert_fires(case_for("E002"));
}
#[test]
fn e003_fires_on_defect() {
    assert_fires(case_for("E003"));
}
#[test]
fn w100_fires_on_defect() {
    assert_fires(case_for("W100"));
}
#[test]
fn w110_fires_on_defect() {
    assert_fires(case_for("W110"));
}
#[test]
fn w211_fires_on_defect() {
    assert_fires(case_for("W211"));
}
#[test]
fn w220_fires_on_defect() {
    assert_fires(case_for("W220"));
}
#[test]
fn w210_fires_on_defect() {
    assert_fires(case_for("W210"));
}
#[test]
fn w302_fires_on_defect() {
    assert_fires(case_for("W302"));
}
#[test]
fn i230_fires_on_defect() {
    assert_fires(case_for("I230"));
}

// -- silent cases --------------------------------------------------------

#[test]
fn e002_silent_on_corrected_form() {
    assert_silent(case_for("E002"));
}
#[test]
fn e003_silent_on_corrected_form() {
    assert_silent(case_for("E003"));
}
#[test]
fn w100_silent_on_corrected_form() {
    assert_silent(case_for("W100"));
}
#[test]
fn w110_silent_on_corrected_form() {
    assert_silent(case_for("W110"));
}
#[test]
fn w211_silent_on_corrected_form() {
    assert_silent(case_for("W211"));
}
#[test]
fn w220_silent_on_corrected_form() {
    assert_silent(case_for("W220"));
}
#[test]
fn w210_silent_on_corrected_form() {
    assert_silent(case_for("W210"));
}
#[test]
fn w302_silent_on_corrected_form() {
    assert_silent(case_for("W302"));
}
#[test]
fn i230_silent_on_corrected_form() {
    assert_silent(case_for("I230"));
}

// -- TestOptimisationMatrix ----------------------------------------------
// Optimiser offers (O-codes) surface through `optimiseDocument`.

#[test]
fn constant_llength_folds() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "puts [llength [list a b c]]\n");
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", serde_json::json!([uri, "full"]));
    let source = result.get("source").and_then(Value::as_str).unwrap_or("");
    assert!(source.contains("puts 3"), "{source:?}");
}

#[test]
fn constant_expr_folds() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc p {} { return [expr {1 + 2}] }\n");
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", serde_json::json!([uri, "full"]));
    let source = result.get("source").and_then(Value::as_str).unwrap_or("");
    assert!(source.contains("return 3"), "{source:?}");
}

#[test]
fn nothing_to_fold_is_unchanged() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {a b} { return [expr {$a + $b}] }\n";
    lsp.open_ready(&uri, src);
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", serde_json::json!([uri, "full"]));
    let source = result.get("source").and_then(Value::as_str).unwrap_or("");
    // A dynamic expression has no safe constant fold — source is preserved.
    assert!(source.contains("expr {$a + $b}"), "{source:?}");
}
