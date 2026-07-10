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
    Case {
        code: "T101",
        fire: "set x [gets stdin]\nputs $x\n",
        silent: "set x [gets stdin]\nset n [string length $x]\nputs $n\n",
        note: "tainted data flows into puts output sink",
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
        assert_eq!(
            sev,
            Some(1),
            "{} is an error code but severity={sev:?}",
            case.code
        );
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
#[test]
fn t101_fires_on_defect() {
    assert_fires(case_for("T101"));
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
#[test]
fn t101_silent_on_corrected_form() {
    assert_silent(case_for("T101"));
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

// -- O103 (pure-proc constant fold) ---------------------------------------

#[test]
fn o103_folds_implicit_return_proc_end_to_end() {
    // TP: the KCS O103 doc's own canonical example — a proc with no
    // explicit `return`, relying on Tcl's "value of the last command
    // executed" rule — must fold through the full LSP pipeline, not just
    // the in-process pass. See docs/kcs/codes/kcs-optimisation-o103-static-proc-folding.md.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc double {n} { expr {$n * 2} }\nset x [double 21]\n",
    );
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", serde_json::json!([uri, "full"]));
    let source = result.get("source").and_then(Value::as_str).unwrap_or("");
    assert!(source.contains("set x 42"), "{source:?}");
}

#[test]
fn o103_does_not_fold_proc_renamed_over() {
    // FP guard / miscompile-guard: `rename triple double` moves `triple`'s
    // body onto the name `double`, so `[double 21]` now runs `triple`'s
    // body (63), not the original `double` proc's body (42). The optimiser
    // must leave the call site untouched rather than fold to the wrong
    // constant.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc double {n} { expr {$n * 2} }\nproc triple {n} { expr {$n * 3} }\nrename triple double\nset x [double 21]\n";
    lsp.open_ready(&uri, src);
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", serde_json::json!([uri, "full"]));
    let source = result.get("source").and_then(Value::as_str).unwrap_or("");
    assert!(source.contains("set x [double 21]"), "{source:?}");
}

// O101 deep-review (F1-F4) end-to-end coverage: each case exercises the real
// server's own dialect detection / whole-module scan, not just the compiler
// API directly, per the review's "editor-facing" requirement.

#[test]
fn irules_dialect_leading_zero_folds_as_octal() {
    // TP: opened with the `tcl-irule` language id, so the server's own
    // dialect detection selects iRules' octal-leading-zero reading (same as
    // tcl8.x) — tclsh8.6/iRules: `010 + 1` == 9 (octal 8 + 1), not 11.
    let mut lsp = Lsp::irules();
    let uri = unique_uri("irule");
    lsp.open_ready_lang(
        &uri,
        "when RULE_INIT {\n log local0. [expr {010 + 1}]\n}\n",
        "tcl-irule",
    );
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", serde_json::json!([uri, "full"]));
    let source = result.get("source").and_then(Value::as_str).unwrap_or("");
    assert!(source.contains("log local0. 9"), "{source:?}");
}

#[test]
fn expr_redefinition_blocks_fold() {
    // FP guard (F2): once `expr` is renamed, `[expr {1 + 2}]` no longer
    // invokes the builtin evaluator — O101 must leave it unrewritten.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "rename expr real_expr\nproc p {} { return [expr {1 + 2}] }\n",
    );
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", serde_json::json!([uri, "full"]));
    let source = result.get("source").and_then(Value::as_str).unwrap_or("");
    assert!(source.contains("expr {1 + 2}"), "{source:?}");
    assert!(!source.contains("return 3"), "{source:?}");
}

#[test]
fn expr_not_redefined_still_folds() {
    // TN/control: the same source with no `rename` still folds normally —
    // proves the F2 guard is keyed on an actual rebinding, not a blanket
    // regression.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc p {} { return [expr {1 + 2}] }\n");
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", serde_json::json!([uri, "full"]));
    let source = result.get("source").and_then(Value::as_str).unwrap_or("");
    assert!(source.contains("return 3"), "{source:?}");
}

#[test]
fn global_write_across_opaque_call_blocks_fold() {
    // FP guard (F4a): `mutate` writes the global `x` via `global x; set x
    // 99`, called between the top-level `set x 5` and the `expr {$x + 1}`
    // read. tclsh: `x` is 99 by the time `puts` runs, so the correct value
    // is 100 — O101 must not fold this to `puts 6`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc mutate {} { global x; set x 99 }\nset x 5\nmutate\nputs [expr {$x + 1}]\n",
    );
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", serde_json::json!([uri, "full"]));
    let source = result.get("source").and_then(Value::as_str).unwrap_or("");
    assert!(!source.contains("puts 6"), "{source:?}");
}

// Regression: a top-level variable reassigned via `global` inside a called
// procedure must never be folded as if its initial assignment were a stable
// constant. Confirmed against tclsh 8.6/9.0 as a real miscompile before this
// was fixed — `puts $g` prints `17`, not the pre-call literal `4`.
#[test]
fn top_level_global_reassigned_by_callee_is_not_folded() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set g 4\nproc helper {} { global g\nset g 17 }\nhelper\nputs $g\n";
    lsp.open_ready(&uri, src);
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", serde_json::json!([uri, "full"]));
    let source = result.get("source").and_then(Value::as_str).unwrap_or("");
    assert!(
        source.contains("puts $g"),
        "must not fold `puts $g` to the stale literal 4: {source:?}"
    );
}

// TN control: a top-level constant no procedure ever touches via `global`
// must still fold — the whole-module scan introduced by the fix above must
// not over-widen to every top-level variable.
#[test]
fn top_level_constant_untouched_by_any_proc_still_folds() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set safe_const 42\nproc other {} { puts unrelated }\nother\nputs $safe_const\n";
    lsp.open_ready(&uri, src);
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", serde_json::json!([uri, "full"]));
    let source = result.get("source").and_then(Value::as_str).unwrap_or("");
    assert!(source.contains("puts 42"), "{source:?}");
}

// Regression: O103 must not fold a call to a procedure renamed away
// elsewhere in the file — confirmed against tclsh, which raises "invalid
// command name" for the same script (the rename means `::foo` no longer
// refers to the original body at runtime).
#[test]
fn proc_renamed_away_is_not_folded() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc ::foo {} { return 42 }\nrename ::foo ::bar\nputs [::foo]\n";
    lsp.open_ready(&uri, src);
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", serde_json::json!([uri, "full"]));
    let source = result.get("source").and_then(Value::as_str).unwrap_or("");
    assert!(source.contains("puts [::foo]"), "{source:?}");
}

// FN/TN control: an ordinary, untouched pure proc in the same file still
// folds — the rename/alias trust gate must not over-widen to every
// procedure in the module.
#[test]
fn untouched_proc_in_same_file_still_folds() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc ::foo {} { return 42 }\nproc ::other {} { return 1 }\nputs [::foo]\n";
    lsp.open_ready(&uri, src);
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", serde_json::json!([uri, "full"]));
    let source = result.get("source").and_then(Value::as_str).unwrap_or("");
    assert!(source.contains("puts 42"), "{source:?}");
}

// TP: a proc with a trailing variadic `args` parameter is seeded as a
// canonical Tcl list (not skipped) — `args={2}` for `[::foo 1 2]` is a
// one-element list, which renders bare as `2`, so `return $args` folds
// end to end through the full LSP pipeline. Confirmed against tclsh 8.6:
// `puts [::foo 1 2]` prints `2`.
#[test]
fn variadic_args_proc_returning_args_directly_folds() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc ::foo {a args} { return $args }\nputs [::foo 1 2]\n";
    lsp.open_ready(&uri, src);
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", serde_json::json!([uri, "full"]));
    let source = result.get("source").and_then(Value::as_str).unwrap_or("");
    assert!(source.contains("puts 2"), "{source:?}");
}

// Regression: a literal-body `uplevel #0 {...}` reassigns a variable in the
// absolute global frame — at top level that coincides with the calling
// scope, so it can mutate a variable there with no `global`/`upvar`
// declaration of its own. Confirmed against tclsh 8.6/9.0 as a real
// miscompile before SCCP widened tracked values across `Statement::UpFrame`:
// `set n 5; uplevel #0 {set n 99}; puts $n` prints `99`, not the stale `5`.
#[test]
fn uplevel_hash0_reassignment_is_not_folded() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set n 5\nuplevel #0 { set n 99 }\nputs $n\n";
    lsp.open_ready(&uri, src);
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", serde_json::json!([uri, "full"]));
    let source = result.get("source").and_then(Value::as_str).unwrap_or("");
    assert!(
        source.contains("puts $n"),
        "must not fold `puts $n` to the stale literal 5: {source:?}"
    );
}

// TN control: a plain top-level constant with no intervening `uplevel`/
// `interp eval` barrier must still fold — the UpFrame-widening fix above
// must not over-widen to every top-level variable.
#[test]
fn top_level_constant_untouched_by_uplevel_still_folds() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set n 5\nputs $n\n";
    lsp.open_ready(&uri, src);
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", serde_json::json!([uri, "full"]));
    let source = result.get("source").and_then(Value::as_str).unwrap_or("");
    assert!(source.contains("puts 5"), "{source:?}");
}

// Regression (P1, code review on #859): a `global` declaration hidden inside
// a *static-body* `uplevel #0 { ... }` inside a proc lowers to
// `Statement::UpFrame`, not a plain nested block. The whole-module
// `scan_module_global_names` scan is built on the shared `for_each_statement`
// visitor, which didn't descend into `UpFrame` bodies — so this name was
// invisible to SCCP/O102's extra-escaping guard and the final read could
// still fold to the stale pre-call literal. Confirmed against tclsh 8.6:
// `set g 4; proc helper {} { uplevel #0 { global g; set g 17 } }; helper;
// puts $g` prints `17`, not `4`.
#[test]
fn global_hidden_inside_uplevel_body_in_proc_is_not_folded() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set g 4\nproc helper {} { uplevel #0 { global g\nset g 17 } }\nhelper\nputs $g\n";
    lsp.open_ready(&uri, src);
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", serde_json::json!([uri, "full"]));
    let source = result.get("source").and_then(Value::as_str).unwrap_or("");
    assert!(
        source.contains("puts $g"),
        "must not fold `puts $g` to the stale literal 4: {source:?}"
    );
}

// S101 deep-review end-to-end coverage: exercises the real server's own
// dialect detection / whole-module scan / TclOO method pipeline, per the
// review's "editor-facing" requirement — the compiler-level TP/FP/TN/FN
// coverage lives in `rust/tcl-compiler/src/shimmer/*.rs` and
// `rust/tcl-compiler/src/compiler_checks.rs`.

#[test]
fn s101_fires_for_shimmer_inside_loop() {
    // TP: `$x` is a foreach element (String-typed) used as a List via
    // `lindex` on every iteration — the canonical S101 case.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc f {items} {\n  foreach x $items {\n    set y [lindex $x 0]\n  }\n}\n",
    );
    assert!(
        codes(&diags).contains("S101"),
        "expected S101 for the in-loop shimmer, got {:?}",
        codes(&diags)
    );
}

#[test]
fn s101_fires_inside_tcloo_method_body() {
    // TP (deep-review finding): before the fix, `find_shimmer_warnings_for_cu`
    // / `run_all_checks`'s per-function loop iterated `cu.analysable_functions()`,
    // which excludes TclOO method bodies — this exact per-iteration shimmer
    // fired for an equivalent plain `proc` but was silently dropped for a
    // `method`. See `shimmer::find_shimmer_warnings_for_cu`'s doc comment.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create Foo {\n  method run {items} {\n    foreach x $items {\n      set y [lindex $x 0]\n    }\n  }\n}\n",
    );
    assert!(
        codes(&diags).contains("S101"),
        "expected S101 inside the TclOO method body, got {:?}",
        codes(&diags)
    );
}

#[test]
fn s101_related_information_points_at_the_definition_site() {
    // Precision: a phi-merge shimmer's `relatedInformation` must point back
    // at the branch that defined the conflicting value, not be silently
    // dropped (`ShimmerWarning::related` → `compiler_checks::Diagnostic::related`
    // → LSP `relatedInformation` — see `lift_compiler_diagnostics`).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set cond [gets stdin]\nif {$cond} { set x 1 } else { set x \"hello\" }\nputs $x\n",
    );
    let shimmer = diags
        .iter()
        .find(|d| d.get("code").and_then(Value::as_str) == Some("S100"))
        .unwrap_or_else(|| panic!("expected an S100 phi shimmer, got {:?}", codes(&diags)));
    let related = shimmer
        .get("relatedInformation")
        .and_then(Value::as_array)
        .filter(|r| !r.is_empty())
        .unwrap_or_else(|| panic!("expected non-empty relatedInformation, got {shimmer:?}"));
    assert!(
        related.iter().any(|r| r
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("version from")),
        "{related:?}"
    );
}

#[test]
fn s101_silent_for_incr_after_module_wide_rename() {
    // FP guard: `rename incr {}` deletes the builtin module-wide — a later
    // `incr`-shaped call can no longer be trusted to mean Tcl's builtin, so
    // no incr-shimmer should be claimed for it.
    //
    // Driven through `tcl-lsp.compilerExplorer` (which calls
    // `find_shimmer_warnings_for_cu`, a fresh whole-unit rebuild), not the
    // live `open_ready` push-diagnostics path: `ModuleCommandMutations` is a
    // *module-wide* fact, and threading it through the salsa-incremental
    // per-procedure diagnostics cache (`tcl_lsp_db::function_checks`) needs
    // the same `#[salsa::interned]` treatment `ModuleVariableTraces` already
    // received for `CfgContext` — a documented follow-up, not yet done (see
    // `compiler_checks::function_nontaint_checks`'s doc comment). The live
    // path currently still reports the (harmless, non-fix-bearing) message;
    // this test covers the surface that already gets the full fix:
    // compiler-explorer / MCP-server / CLI / `code_action` (all of which
    // rebuild fresh, uncached, per call).
    let mut lsp = Lsp::tcl();
    let src = "rename incr {}\nproc f {items} {\n  foreach x $items {\n    incr x\n  }\n}\n";
    let result = lsp.execute_command("tcl-lsp.compilerExplorer", serde_json::json!([src, "tcl"]));
    let shimmer = result
        .get("shimmer")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        !shimmer
            .iter()
            .any(|w| { w.get("command").and_then(Value::as_str) == Some("incr") }),
        "incr must not be trusted after a module-wide rename: {shimmer:?}"
    );
}

#[test]
fn s101_fires_for_incr_without_rename_control_via_explorer() {
    // TN/control: the identical code with no `rename` still fires through
    // the same `compilerExplorer` surface — proves the guard above is keyed
    // on an actual rebinding, not a blanket regression of that surface.
    let mut lsp = Lsp::tcl();
    let src = "proc f {items} {\n  foreach x $items {\n    incr x\n  }\n}\n";
    let result = lsp.execute_command("tcl-lsp.compilerExplorer", serde_json::json!([src, "tcl"]));
    let shimmer = result
        .get("shimmer")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        shimmer
            .iter()
            .any(|w| w.get("command").and_then(Value::as_str) == Some("incr")),
        "expected the no-rename control to still fire: {shimmer:?}"
    );
}

#[test]
fn s101_fires_through_interp_alias() {
    // TP: a call through an `interp alias`-created name is checked exactly
    // like a call to its target.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "interp alias {} li {} lindex\nproc f {items} {\n  foreach x $items {\n    li $x 0\n  }\n}\n",
    );
    assert!(
        codes(&diags).contains("S101"),
        "expected S101 through the 'li' alias to lindex, got {:?}",
        codes(&diags)
    );
}

#[test]
fn no_s100_shimmer_for_tcloo_instance_variable() {
    // FP guard: `my variable count` links an object-instance variable whose
    // true intrep depends on another method's last write, not the nominal
    // return type of `my`. Before the registry fix (`oo_my.rs`'s `variable`
    // subcommand), this spuriously claimed a String->Int shimmer.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create Foo {\n  method bump {} {\n    my variable count\n    incr count\n  }\n}\n",
    );
    assert!(
        !diags.iter().any(|d| {
            matches!(d.get("code").and_then(Value::as_str), Some("S100" | "S101"))
                && d.get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .contains("'count'")
        }),
        "'my variable'-linked instance var must not spuriously shimmer: {diags:?}"
    );
}

#[test]
fn s101_tight_span_for_nested_command_substitution_via_explorer() {
    // Precision: `set y [lindex $x 0]` is a `Statement::AssignValue` (the
    // `[lindex …]` is a command substitution nested in the value, not a
    // top-level `Statement::Call`), which carries no per-word tokens of its
    // own for the nested call — the single most common shape a shimmer
    // arises in. The highlighted span must still cover just `$x`, recovered
    // from source via `nested_call_arg_spans`, not the whole statement.
    //
    // Driven through `tcl-lsp.compilerExplorer` (a fresh, uncached rebuild
    // with real source text) rather than live `open_ready`: the LSP db's
    // offset-0 memoised `function_checks` salsa query passes `""` as source
    // for a regular top-level procedure's *cached* diagnostics, so that path
    // still falls back to the whole-statement span — a documented, safe
    // (never-wrong, just wider) limitation; see `function_checks`'s doc
    // comment in `tcl-lsp-db`.
    let mut lsp = Lsp::tcl();
    let src = "proc f {items} {\n  foreach x $items {\n    set y [lindex $x 0]\n  }\n}\n";
    let result = lsp.execute_command("tcl-lsp.compilerExplorer", serde_json::json!([src, "tcl"]));
    let shimmer = result
        .get("shimmer")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let w = shimmer
        .iter()
        .find(|w| w.get("command").and_then(Value::as_str) == Some("lindex"))
        .unwrap_or_else(|| panic!("expected a lindex shimmer, got {shimmer:?}"));
    let range = w.get("range").unwrap_or_else(|| panic!("no range: {w:?}"));
    let start = range.get("startOffset").and_then(Value::as_u64).unwrap();
    let end = range.get("endOffset").and_then(Value::as_u64).unwrap();
    let arg_start = src.find("$x").unwrap() as u64;
    let arg_end = arg_start + "$x".len() as u64;
    assert_eq!(
        (start, end),
        (arg_start, arg_end),
        "expected the span to cover exactly '$x' ({arg_start}..{arg_end}), \
         not the whole statement: got {w:?}"
    );
}
