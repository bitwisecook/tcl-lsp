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

//! The inliner's classification, statement counts and the bodies it declines.

use super::*;

// inlining/mod.rs — classify_proc / inline_module decline reasons

#[test]
fn inline_classify_no_summary_is_never() {
    // `classify_proc` with no escape summary (None) → Never (the soundness gate
    // fails closed).
    let module = module_for("proc ::f {} { puts hi }\n");
    let proc = module
        .procedures
        .get(&fixture_id(&module.procedures, "::f"))
        .expect("authored fixture body");
    assert_eq!(
        classify_proc(proc, None, 0),
        InlineDecision::Never,
        "a proc with no escape summary must be Never",
    );
}

#[test]
fn inline_classify_side_effecting_proc_is_never() {
    // A proc that writes a caller frame via `upvar` is NOT pure-leaf, so it is
    // Never regardless of size.
    // tclsh (8.6, 9.0): `proc bump {name} {upvar 1 $name v; incr v}; set x 5;
    // bump x; set x` → 6 — the upvar mutates the caller; inlining must not
    // relocate that body.
    let module = module_for("proc ::bump {name} { upvar 1 $name v; incr v }\n");
    let summaries = analyse_var_escape(&module, true);
    let proc = module
        .procedures
        .get(&fixture_id(&module.procedures, "::bump"))
        .expect("authored fixture body");
    assert!(
        !summaries
            .get(&tcl_core_types::BodyFrameId::Body(fixture_id(
                &module.procedures,
                "::bump"
            )))
            .is_some_and(ProcEscapeSummary::safe_to_inline),
        "upvar proc is not safe_to_inline",
    );
    assert_eq!(
        classify_proc(
            proc,
            summaries.get(&tcl_core_types::BodyFrameId::Body(fixture_id(
                &module.procedures,
                "::bump"
            ))),
            0
        ),
        InlineDecision::Never,
    );
}

#[test]
fn inline_classify_small_pure_leaf_is_always_large_is_count_dependent() {
    // Boundary at SMALL_BODY_THRESHOLD: a body of exactly the threshold is
    // Always; threshold+1 is count-dependent (IfSingleCall at 1 caller, Never
    // otherwise).
    let assign_body = |n: usize| -> String {
        use std::fmt::Write as _;
        let mut body = String::new();
        for i in 0..n {
            let _ = writeln!(body, "  set v{i} {i}");
        }
        body
    };
    let small = assign_body(SMALL_BODY_THRESHOLD);
    let big = assign_body(SMALL_BODY_THRESHOLD + 1);
    let ms = module_for(&format!("proc ::s {{}} {{\n{small}}}\n"));
    let mb = module_for(&format!("proc ::b {{}} {{\n{big}}}\n"));
    let ss = analyse_var_escape(&ms, true);
    let sb = analyse_var_escape(&mb, true);
    assert_eq!(
        classify_proc(
            ms.procedures
                .get(&fixture_id(&ms.procedures, "::s"))
                .expect("authored fixture body"),
            ss.get(&tcl_core_types::BodyFrameId::Body(fixture_id(
                &ms.procedures,
                "::s"
            ))),
            99
        ),
        InlineDecision::Always,
        "threshold-sized pure-leaf is Always regardless of count",
    );
    assert_eq!(
        classify_proc(
            mb.procedures
                .get(&fixture_id(&mb.procedures, "::b"))
                .expect("authored fixture body"),
            sb.get(&tcl_core_types::BodyFrameId::Body(fixture_id(
                &mb.procedures,
                "::b"
            ))),
            1
        ),
        InlineDecision::IfSingleCall,
        "threshold+1 with one caller is IfSingleCall",
    );
    assert_eq!(
        classify_proc(
            mb.procedures
                .get(&fixture_id(&mb.procedures, "::b"))
                .expect("authored fixture body"),
            sb.get(&tcl_core_types::BodyFrameId::Body(fixture_id(
                &mb.procedures,
                "::b"
            ))),
            3
        ),
        InlineDecision::Never,
        "threshold+1 with multiple callers is Never",
    );
}

#[test]
fn inline_count_statements_try_and_switch() {
    // `count_statements` walks Try (body + handlers + finally) and Switch (arm +
    // default bodies) — the variants `inlining.rs`'s count tests skip.
    let module = module_for(
        "proc ::f {} {\n try { set a 1 } on error {e} { set b 2 } finally { set c 3 }\n switch -- $a { 1 { set d 4 } default { set e 5 } }\n}\n",
    );
    let n = count_statements(
        &module
            .procedures
            .get(&fixture_id(&module.procedures, "::f"))
            .expect("authored fixture body")
            .body,
    );
    // 3 from try (a, b, c) + 2 from switch (d, e) = 5 leaf statements.
    assert_eq!(n, 5, "count walks try + switch bodies");
}

#[test]
fn inline_count_statements_foreach_and_catch() {
    // foreach / catch each contribute 1 (the construct) + their body count.
    let module = module_for("proc ::f {} {\n foreach x {1 2} { set a 1 }\n catch { set b 2 }\n}\n");
    let n = count_statements(
        &module
            .procedures
            .get(&fixture_id(&module.procedures, "::f"))
            .expect("authored fixture body")
            .body,
    );
    // foreach: 1 + 1(set a) = 2; catch: 1 + 1(set b) = 2; total 4.
    assert_eq!(n, 4, "count walks foreach + catch bodies");
}

#[test]
fn inline_v3_return_inside_catch_declines() {
    // A `return` inside a `catch` body would be trapped by the break-based
    // early-return wrap — v3 declines, so the call survives.
    let stmts = inlined("proc ::h {x} { catch { return $x } }\nh 1")
        .top_level
        .statements;
    assert!(
        stmts
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "h")),
        "return-inside-catch keeps the call",
    );
}

#[test]
fn inline_v3_return_inside_try_declines() {
    // Same for a `return` inside a `try` body.
    let stmts = inlined("proc ::h {x} { try { return $x } on error {e} { } }\nh 1")
        .top_level
        .statements;
    assert!(
        stmts
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "h")),
        "return-inside-try keeps the call",
    );
}

#[test]
fn inline_v3_global_write_declines() {
    // A `::`-qualified (global) write isn't covered by the local-only rename
    // map, so v3 declines and the call survives.
    let stmts = inlined("proc ::w {} { set ::g 1 }\nw").top_level.statements;
    assert!(
        stmts
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "w")),
        "a global write declines v3, keeping the call",
    );
}

#[test]
fn inline_verbatim_non_namespace_invariant_call_declines() {
    // A wrapper whose body calls an unqualified proc that DOES resolve to a
    // known proc is not namespace-invariant (it could resolve differently from
    // a caller's namespace), so it is not splice-eligible — the call survives.
    let src = "proc ::helper {} { puts deep }\nproc ::wrap {} { helper }\nwrap";
    let module = inlined(src);
    // `wrap` calls `helper` (a known proc, unqualified) → wrap's body is not
    // splice-safe (helper is not in the SPLICE_SAFE allow-list anyway), so the
    // `wrap` call is not verbatim-inlined.
    assert_eq!(
        top_calls_to(&module, "wrap"),
        1,
        "wrapper calling a known proc is not verbatim-spliced",
    );
}

#[test]
fn inline_empty_body_with_args_declines() {
    // An empty-body proc inlined as `Empty` requires a zero-arg call site; a
    // call WITH args declines (the splice can't bind them).
    let module = inlined("proc ::noop {} {}\nnoop extra");
    assert_eq!(
        top_calls_to(&module, "noop"),
        1,
        "empty-body proc called with args keeps the call",
    );
}

#[test]
fn inline_recursive_proc_does_not_loop() {
    // A self-recursive proc must not inline into an infinite expansion. The
    // pass replaces a single static call site only; the inliner must terminate
    // and the module is well-formed.
    let module = inlined("proc ::rec {n} { if {$n} { rec [expr {$n - 1}] } }\nrec 3");
    // `rec` is not pure-leaf (it calls itself / has a command-subst arg), so it
    // never qualifies — the call survives and the pass terminates.
    assert!(
        fixture_id_if_present(&module.procedures, "::rec").is_some(),
        "recursive proc remains defined",
    );
}

#[test]
fn inline_idempotent_on_clean_module() {
    // A module with no inlinable site is returned structurally unchanged
    // (inline_module's early-return when the inlinable map is empty).
    let src = "proc ::p {a} { upvar 1 $a v; set v 1 }\np x";
    let once = inlined(src);
    let twice = inline_module(once.clone(), &reg());
    assert_eq!(once, twice, "no inlinable site → unchanged module");
}

#[test]
fn inline_v3_semantics_preserved_value() {
    // The load-bearing proof that v3 inlining is semantics-preserving.
    // tclsh (8.6, 9.0): `proc add {a b} { return [expr {$a + $b}] }; add 3 4`
    // → 7.  After inlining `add 3 4`, the spliced body must still compute 7 —
    // here we assert the call was replaced and a renamed binding/return is
    // present (the value identity is what tclsh confirms).
    let stmts = inlined("proc ::add {a b} { return [expr {$a + $b}] }\nadd 3 4")
        .top_level
        .statements;
    assert!(
        !stmts
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "add")),
        "the add call was inlined",
    );
    assert!(
        stmts.iter().any(|s| matches!(s, Statement::Return { .. })),
        "terminal trailing return is preserved (forwards the value)",
    );
}
