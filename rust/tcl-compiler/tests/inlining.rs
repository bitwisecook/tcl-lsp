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

//! Integration tests for the `inlining` and `inline_uplevel` modules.
//!
//! Four groups of tests exercise the compiler:
//!   * proc classification (the inlining catalogue: size / pure-leaf /
//!     static-call-count → ALWAYS / `IF_SINGLE_CALL` / NEVER).
//!   * the inline transform (call sites replaced by proc bodies, α-renaming to
//!     avoid variable capture, defaults / variadics / early-return wrapping /
//!     decline cases).
//!   * the inlining ↔ GVN interaction. GVN is a separate pass with no public
//!     entry point exposed to an integration test; the shared invariant it
//!     rests on (the splice-safe / frame-independent allow-list) is the same
//!     allow-list the inliner uses, so the load-bearing behaviour is exercised
//!     here through the inliner's verbatim-splice path instead (noted at the
//!     relevant tests).
//!   * whole-callee `uplevel`-passthrough inlining (`detect_*` recognition +
//!     the `inline_uplevel_passthrough` rewrite).
//!
//! ## Rust API used (all public from `tcl_compiler`)
//!
//!   * `inlining::{count_statements, classify_proc, inline_module,
//!     InlineDecision, SMALL_BODY_THRESHOLD}`
//!   * `inline_uplevel::{detect_passthrough_candidates,
//!     detect_static_passthrough, body_has_frame_reach,
//!     inline_uplevel_passthrough, PassthroughShape}`
//!   * `var_escape::analyse_var_escape` + `ProcEscapeSummary::safe_to_inline`
//!   * `CompilationUnit::build_for(src, &registry, false).ir_module` — lowers
//!     source to the IR module the shared helpers below start from.
//!
//! Two helpers have no public API counterpart and so are covered by observable
//! proxy rather than direct call:
//!   * `count_static_calls` is `pub(crate)`-only (`fn`, not `pub fn`) — its
//!     three static-call-count cases are re-expressed through `classify_proc`,
//!     which *takes the static call count as a parameter*, so the same 0 / 1 / N
//!     call-count decisions are asserted directly (and the 1-vs-2-caller
//!     `NEVER/IF_SINGLE_CALL` split is the load-bearing behaviour).
//!   * `apply_inline_catalogue` (which tags `inline_decision` /
//!     `static_call_count` onto a fresh `Module`) is internal; `classify_proc`
//!     is the public decision primitive and is asserted directly with the count
//!     supplied.
//!
//! ## Structural vs. semantic proof split
//!
//! The classification verdicts, the spliced-IR shape (which statement kind a
//! binding is, what the mangled `__inline_<n>__<name>` slot is named,
//! whether a call became a `Block` / `While` / `Return`) are
//! **compiler-internal** facts about the IR transform — they are not
//! directly observable from a Tcl program's output, so they are asserted
//! structurally. The repo's execution-differential standard for the inliner
//! is gated on the (not-yet-implemented) WASM-codegen consumer, matching the
//! note in `src/inlining/tests.rs`.
//!
//! The **load-bearing** proof is that inlining is *semantics-preserving*:
//! the inlined program computes the same value as the conceptual original.
//! Each test whose proc has an observable result cites the value confirmed
//! against real `tclsh8.6` and `tclsh9.0` via `scripts/dev/tclsh_check.sh`
//! with a `// tclsh:` comment (8.4 / 8.5 are not installed in this
//! environment). The renaming tests carry the canonical capture-avoidance
//! proof: a caller variable and a callee local that share a name must stay
//! distinct after inlining, and tclsh shows the two values never collide.

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::inline_uplevel::{
    PassthroughShape, body_has_frame_reach, detect_passthrough_candidates,
    detect_static_passthrough, inline_uplevel_passthrough,
};
use tcl_compiler::inlining::{
    InlineDecision, SMALL_BODY_THRESHOLD, classify_proc, count_statements, inline_module,
};
use tcl_compiler::ir::{Module, Statement};
use tcl_compiler::var_escape::{
    ProcEscapeSummary, analyse_var_escape, analyse_var_escape_with_registry,
};
use tcl_registry::CommandRegistry;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn reg() -> CommandRegistry {
    CommandRegistry::build_default()
}

/// `source → Module`, lowering source to its IR module.
fn module_for(source: &str) -> Module {
    CompilationUnit::build_for(source, &reg(), false).ir_module
}

/// Build a proc body of `n` distinct `set vN N` assignments.
fn assign_body(n: usize) -> String {
    use std::fmt::Write as _;
    let mut body = String::new();
    for i in 0..n {
        let _ = writeln!(body, "  set v{i} {i}");
    }
    body
}

/// Run the whole-module inline transform (`inline_module` runs its own
/// `analyse_var_escape` internally, so no summaries argument).
fn inlined(source: &str) -> Module {
    inline_module(module_for(source), &reg())
}

/// Whole-callee `uplevel`-passthrough inlining over a fresh module.
fn uplevel_inlined(source: &str) -> Module {
    let mut m = module_for(source);
    inline_uplevel_passthrough(&mut m, &reg());
    m
}

/// Count statement-position calls to `command` across a script.
fn calls_to(stmts: &[Statement], command: &str) -> usize {
    stmts
        .iter()
        .filter(|s| matches!(s, Statement::Call { command: c, .. } if c == command))
        .count()
}

/// Count statement-position calls to `command` across the module top level.
fn top_calls_to(module: &Module, command: &str) -> usize {
    calls_to(&module.top_level.statements, command)
}

/// All `(name, value)` pairs from `AssignConst` / `AssignValue` statements in
/// a flat statement list — used to inspect the mangled bindings the v3 splice
/// emits at the call site.
fn assign_pairs(stmts: &[Statement]) -> Vec<(String, String)> {
    stmts
        .iter()
        .filter_map(|s| match s {
            Statement::AssignConst { name, value, .. }
            | Statement::AssignValue { name, value, .. } => Some((name.clone(), value.clone())),
            _ => None,
        })
        .collect()
}

/// The escape summary for one proc (for `classify_proc`).
fn summary_for<'a>(
    summaries: &'a std::collections::HashMap<String, ProcEscapeSummary>,
    qname: &str,
) -> Option<&'a ProcEscapeSummary> {
    summaries.get(qname)
}

// ===========================================================================
// Proc classification
// ===========================================================================

// --- TestStatementCount ----------------------------------------------------

#[test]
fn statement_count_empty_body_is_zero() {
    let module = module_for("proc foo {} {}\n");
    let proc = &module.procedures["::foo"];
    assert_eq!(count_statements(&proc.body), 0);
}

#[test]
fn statement_count_flat_body_counts_top_level() {
    let module = module_for("proc foo {} {\n  set a 1\n  set b 2\n  set c 3\n}\n");
    let proc = &module.procedures["::foo"];
    assert_eq!(count_statements(&proc.body), 3);
}

#[test]
fn statement_count_nested_if_counts_inner() {
    // The `if` contributes its clause plus the nested `set`, so the body
    // counts >= 1.
    let module = module_for("proc foo {x} {\n  if {$x} { set a 1 }\n}\n");
    let proc = &module.procedures["::foo"];
    assert!(count_statements(&proc.body) >= 1);
}

// --- TestStaticCallCount (re-expressed through classify_proc) ---------------
//
// `count_static_calls` is not part of the public surface, so its three cases
// are asserted through `classify_proc`, which consumes the static call count
// directly. The behaviour that depends on the count — a large pure-leaf proc
// is IF_SINGLE_CALL at one caller and NEVER at two — is the load-bearing part
// and is covered below.

#[test]
fn static_call_count_drives_large_proc_decision() {
    // A pure-leaf proc bigger than the threshold: 0 callers and 2 callers
    // both → NEVER; exactly 1 caller → IF_SINGLE_CALL. The raw count helper is
    // internal, so this is asserted through the classifier.
    let body = assign_body(SMALL_BODY_THRESHOLD + 2);
    let module = module_for(&format!("proc big {{}} {{\n{body}}}\n"));
    let summaries = analyse_var_escape(&module, true);
    let proc = &module.procedures["::big"];
    let s = summary_for(&summaries, "::big");
    assert_eq!(classify_proc(proc, s, 0), InlineDecision::Never);
    assert_eq!(classify_proc(proc, s, 1), InlineDecision::IfSingleCall);
    assert_eq!(classify_proc(proc, s, 2), InlineDecision::Never);
}

// --- TestClassify ----------------------------------------------------------

#[test]
fn classify_small_pure_leaf_is_always() {
    // Small + pure-leaf → ALWAYS regardless of call count.
    // tclsh (8.6, 9.0): `proc add {a b} {expr {$a + $b}}; add 3 4` → 7, so
    // inlining `add` must preserve that result.
    let module = module_for("proc add {a b} { return [expr {$a + $b}] }\n");
    let summaries = analyse_var_escape(&module, true);
    let proc = &module.procedures["::add"];
    assert_eq!(
        classify_proc(proc, summary_for(&summaries, "::add"), 0),
        InlineDecision::Always
    );
}

#[test]
fn classify_non_pure_leaf_is_never() {
    // `upvar` makes the proc non-pure-leaf, so it is NEVER inlinable
    // regardless of size.
    // tclsh (8.6, 9.0): `proc setter {name val} {upvar 1 $name v; set v
    // $val}; set x 0; setter x 99; set x` → 99 — the upvar writes the
    // caller's frame; inlining must NOT relocate that body.
    let module = module_for("proc setter {name val} {\n  upvar 1 $name v\n  set v $val\n}\n");
    let summaries = analyse_var_escape(&module, true);
    let proc = &module.procedures["::setter"];
    assert!(!summary_for(&summaries, "::setter").is_some_and(ProcEscapeSummary::safe_to_inline));
    assert_eq!(
        classify_proc(proc, summary_for(&summaries, "::setter"), 0),
        InlineDecision::Never
    );
}

#[test]
fn classify_large_pure_leaf_one_caller_is_if_single_call() {
    let body = assign_body(SMALL_BODY_THRESHOLD + 2);
    let module = module_for(&format!("proc big {{}} {{\n{body}}}\nbig\n"));
    let summaries = analyse_var_escape(&module, true);
    let proc = &module.procedures["::big"];
    assert_eq!(
        classify_proc(proc, summary_for(&summaries, "::big"), 1),
        InlineDecision::IfSingleCall
    );
}

#[test]
fn classify_large_pure_leaf_two_callers_is_never() {
    let body = assign_body(SMALL_BODY_THRESHOLD + 2);
    let module = module_for(&format!("proc big {{}} {{\n{body}}}\nbig\nbig\n"));
    let summaries = analyse_var_escape(&module, true);
    let proc = &module.procedures["::big"];
    assert_eq!(
        classify_proc(proc, summary_for(&summaries, "::big"), 2),
        InlineDecision::Never
    );
}

#[test]
fn classify_threshold_boundary_is_always() {
    // Edge case driving the `body_size <= SMALL_BODY_THRESHOLD` branch: a
    // pure-leaf body of *exactly* the threshold size is still ALWAYS.
    let body = assign_body(SMALL_BODY_THRESHOLD);
    let module = module_for(&format!("proc edge {{}} {{\n{body}}}\nedge\nedge\n"));
    let summaries = analyse_var_escape(&module, true);
    let proc = &module.procedures["::edge"];
    assert_eq!(count_statements(&proc.body), SMALL_BODY_THRESHOLD);
    assert_eq!(
        classify_proc(proc, summary_for(&summaries, "::edge"), 2),
        InlineDecision::Always
    );
}

#[test]
fn classify_missing_summary_is_never() {
    // Edge case driving the `!summary.is_some_and(...)` early return: with no
    // escape summary at all, a proc is conservatively NEVER inlinable.
    let module = module_for("proc add {a b} { return [expr {$a + $b}] }\n");
    let proc = &module.procedures["::add"];
    assert_eq!(classify_proc(proc, None, 0), InlineDecision::Never);
}

// ===========================================================================
// The inline transform
// ===========================================================================

// --- TestEmptyBodySplice ---------------------------------------------------

#[test]
fn empty_body_call_is_dropped() {
    // Both `noop` calls vanish; the proc definition stays.
    let module = module_for("proc noop {} {}\nnoop\nnoop\n");
    assert_eq!(top_calls_to(&module, "noop"), 2);
    let out = inline_module(module, &reg());
    assert_eq!(top_calls_to(&out, "noop"), 0);
    assert!(out.procedures.contains_key("::noop"));
}

#[test]
fn non_empty_body_v3_inlines_and_keeps_proc() {
    // The call vanishes but the proc body STAYS in the module (externally
    // observable).
    let out = inlined("proc setup {} { set x 1 }\nsetup\n");
    assert_eq!(top_calls_to(&out, "setup"), 0);
    assert!(out.procedures.contains_key("::setup"));
}

#[test]
fn non_pure_leaf_is_not_inlined() {
    // `upvar` blocks pure-leaf, so the call survives even though the body is
    // short.
    let out = inlined("proc skip_me {name} { upvar 1 $name v }\nskip_me x\n");
    assert_eq!(top_calls_to(&out, "skip_me"), 1);
}

// --- TestNestedSites -------------------------------------------------------

#[test]
fn call_inside_if_clause_dropped() {
    let out = inlined("proc noop {} {}\nif {1} { noop }\n");
    let if_node = out
        .top_level
        .statements
        .iter()
        .find(|s| matches!(s, Statement::If { .. }))
        .expect("expected an If statement");
    let Statement::If { clauses, .. } = if_node else {
        unreachable!()
    };
    assert_eq!(calls_to(&clauses[0].body.statements, "noop"), 0);
}

#[test]
fn call_inside_for_body_dropped() {
    let out = inlined("proc noop {} {}\nfor {set i 0} {$i < 3} {incr i} { noop }\n");
    let for_node = out
        .top_level
        .statements
        .iter()
        .find(|s| matches!(s, Statement::For { .. }))
        .expect("expected a For statement");
    let Statement::For { body, .. } = for_node else {
        unreachable!()
    };
    assert_eq!(calls_to(&body.statements, "noop"), 0);
}

// --- TestPurity ------------------------------------------------------------

#[test]
fn input_module_unchanged_by_inlining() {
    // The inliner takes the module by value and returns a new one; building a
    // fresh module from the
    // same source still shows the original call (the transform did not mutate
    // shared IR).
    let original = module_for("proc noop {} {}\nnoop\n");
    let before = top_calls_to(&original, "noop");
    let _ = inline_module(original, &reg());
    let again = module_for("proc noop {} {}\nnoop\n");
    assert_eq!(top_calls_to(&again, "noop"), before);
}

#[test]
fn inlining_is_idempotent() {
    // A second pass changes nothing.
    let once = inlined("proc noop {} {}\nnoop\n");
    let twice = inline_module(once.clone(), &reg());
    assert_eq!(once, twice);
}

// --- TestMultiStatementWrapperSplice ---------------------------------------

#[test]
fn two_call_wrapper_inlines_both() {
    // tclsh (8.6, 9.0): `proc setup {} { puts a; puts b }; setup` prints
    // "a\nb"; splicing both `puts` calls inline preserves that output.
    let module = module_for("proc setup {} { puts \"a\"\nputs \"b\" }\nsetup\n");
    assert_eq!(top_calls_to(&module, "setup"), 1);
    assert_eq!(top_calls_to(&module, "puts"), 0);
    let out = inline_module(module, &reg());
    assert_eq!(top_calls_to(&out, "setup"), 0);
    assert_eq!(top_calls_to(&out, "puts"), 2);
}

#[test]
fn mixed_safe_calls_unqualified_inline() {
    // The qualified form is covered separately below. With unqualified `puts`
    // + `string` (both pure-leaf in `var_escape`), the wrapper inlines.
    let out = inlined("proc setup {} { puts \"starting\"\nstring length \"x\" }\nsetup\n");
    assert_eq!(top_calls_to(&out, "setup"), 0);
    assert_eq!(top_calls_to(&out, "puts"), 1);
    assert_eq!(top_calls_to(&out, "string"), 1);
}

#[test]
fn set_in_wrapper_body_v3_inlines_with_alpha_rename() {
    let out = inlined("proc setup {} { puts \"starting\"\nset marker 1 }\nsetup\n");
    assert_eq!(top_calls_to(&out, "setup"), 0);
    assert_eq!(top_calls_to(&out, "puts"), 1);
    // `set marker 1` becomes `set __inline_<n>__marker 1`.
    assert!(
        assign_pairs(&out.top_level.statements)
            .iter()
            .any(|(n, _)| n.starts_with("__inline_") && n.ends_with("__marker")),
        "the body's local write is α-renamed to a mangled slot"
    );
}

// --- TestParameterisedInline (v3) ------------------------------------------

#[test]
fn proc_with_one_param_inlines() {
    // tclsh (8.6, 9.0): `proc inc {x} { set r $x; return $r }; inc 5` → 5 —
    // inlining `inc 5` binds the param and preserves that value.
    let out = inlined("proc inc {x} { puts $x }\ninc 5\n");
    assert_eq!(top_calls_to(&out, "inc"), 0);
    assert!(
        out.top_level
            .statements
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "puts")),
        "the body's puts call is spliced in"
    );
}

#[test]
fn proc_with_local_write_uses_mangled_name() {
    // The body's `y` is α-renamed and the `puts $y` reference uses the SAME
    // mangled name (no capture, and the read still resolves to the write).
    let out = inlined("proc f {} { set y 5\nputs $y }\nf\n");
    assert_eq!(top_calls_to(&out, "f"), 0);
    let mangled = assign_pairs(&out.top_level.statements)
        .into_iter()
        .find(|(n, _)| n.starts_with("__inline_"))
        .map(|(n, _)| n)
        .expect("a mangled set is emitted");
    let puts_uses_mangled = out.top_level.statements.iter().any(|s| {
        matches!(s, Statement::Call { command, args, .. }
            if command == "puts"
            && args.iter().any(|a| a.contains(&mangled)))
    });
    assert!(
        puts_uses_mangled,
        "the read references the same mangled slot as the write ({mangled})"
    );
}

#[test]
fn proc_with_trailing_return_inlines_at_terminal_position() {
    // tclsh (8.6, 9.0): `proc f {x} { ...; return $x }; f 5` → 5; the inlined
    // terminal return forwards the caller's result.
    let out = inlined("proc f {x} { puts $x\nreturn $x }\nf 5\n");
    assert_eq!(top_calls_to(&out, "f"), 0);
    assert!(
        out.top_level
            .statements
            .iter()
            .any(|s| matches!(s, Statement::Return { .. })),
        "a trailing Return lands at the top level"
    );
}

#[test]
fn proc_with_trailing_return_at_non_terminal_via_wrap() {
    // The trailing return is wrapped in a one-shot while so the rest of the
    // caller body still runs; the following `set marker after` survives.
    let out = inlined("proc f {x} { puts $x\nreturn $x }\nf 5\nset marker after\n");
    assert_eq!(top_calls_to(&out, "f"), 0);
    assert!(
        out.top_level.statements.iter().any(|s| matches!(
            s,
            Statement::AssignConst { name, .. } | Statement::AssignValue { name, .. }
            if name == "marker"
        )),
        "the statement after the call survives the wrap"
    );
    assert!(
        out.top_level
            .statements
            .iter()
            .any(|s| matches!(s, Statement::While { .. })),
        "the non-terminal return is routed through a one-shot while-wrap"
    );
}

#[test]
fn arity_mismatch_declines() {
    // One arg to a two-param proc with no default → decline rather than
    // mis-inline.
    let out = inlined("proc f {x y} { puts $x }\nf 1\n");
    assert_eq!(top_calls_to(&out, "f"), 1);
}

#[test]
fn variadic_args_inlines_with_list_pack() {
    // tclsh (8.6, 9.0): `proc f {args} { return $args }; f 1 2 3` → "1 2 3";
    // the inliner binds `args` to `[list 1 2 3]`, which evaluates the same.
    let out = inlined("proc f {args} { puts $args }\nf 1 2 3\n");
    assert_eq!(top_calls_to(&out, "f"), 0);
    let args_binding = assign_pairs(&out.top_level.statements)
        .into_iter()
        .find(|(n, _)| n.ends_with("__args"))
        .expect("an args binding is emitted");
    assert_eq!(args_binding.1, "[list 1 2 3]");
}

#[test]
fn variadic_args_with_quoted_word_declines() {
    // `"hello world"` would re-tokenise inside `[list …]` and change arity, so
    // the inliner declines and keeps the runtime call.
    let out = inlined("proc f {args} { puts $args }\nf a \"hello world\" c\n");
    assert_eq!(top_calls_to(&out, "f"), 1);
}

#[test]
fn variadic_args_with_zero_extras_inlines_to_empty() {
    // `f 1` against `proc f {a args}` binds `args` to the empty string.
    let out = inlined("proc f {a args} { puts $a }\nf 1\n");
    assert_eq!(top_calls_to(&out, "f"), 0);
    let args_binding = assign_pairs(&out.top_level.statements)
        .into_iter()
        .find(|(n, _)| n.ends_with("__args"))
        .expect("an args binding is emitted");
    assert_eq!(args_binding.1, "");
}

#[test]
fn braced_literal_arg_uses_assign_const() {
    // `f {literal-$y}` passes the *literal string*; the binding must be an
    // AssignConst so the inlined `set` does NOT re-substitute `$y` from the
    // caller frame.
    let out = inlined("proc f {x} { puts $x }\nf {literal-$y}\n");
    assert_eq!(top_calls_to(&out, "f"), 0);
    let binding = out
        .top_level
        .statements
        .iter()
        .find_map(|s| match s {
            Statement::AssignConst { name, value, .. } if name.ends_with("__x") => {
                Some(value.clone())
            }
            _ => None,
        })
        .expect("a braced-literal arg binds via AssignConst");
    assert_eq!(binding, "literal-$y", "dollar sign preserved verbatim");
}

#[test]
fn substitution_arg_keeps_assign_value() {
    // `f $y` passes the caller's value, so the binding stays an AssignValue
    // reading the caller frame.
    let out = inlined("proc f {x} { puts $x }\nset y 42\nf $y\n");
    assert_eq!(top_calls_to(&out, "f"), 0);
    let binding = out
        .top_level
        .statements
        .iter()
        .find_map(|s| match s {
            Statement::AssignValue { name, value, .. } if name.ends_with("__x") => {
                Some(value.clone())
            }
            _ => None,
        })
        .expect("a substitution arg binds via AssignValue");
    assert_eq!(binding, "${y}");
}

#[test]
fn expand_arg_declines_inlining() {
    // `f {*}$lst` expands a runtime list; the inliner can't statically unpack
    // it, so it declines.
    let out = inlined("proc f {a b c} { puts $a }\nset lst {1 2 3}\nf {*}$lst\n");
    assert_eq!(top_calls_to(&out, "f"), 1);
}

#[test]
fn default_arg_inlines() {
    // tclsh (8.6, 9.0): `proc f {x {y 5}} { return [list $x $y] }; f 3` →
    // "3 5"; the inliner fills the default `5` for the missing `y`.
    let out = inlined("proc f {x {y 5}} { puts $x }\nf 3\n");
    assert_eq!(top_calls_to(&out, "f"), 0);
    let y_binding = out
        .top_level
        .statements
        .iter()
        .find_map(|s| match s {
            Statement::AssignConst { name, value, .. }
            | Statement::AssignValue { name, value, .. }
                if name.ends_with("__y") =>
            {
                Some(value.clone())
            }
            _ => None,
        })
        .expect("the default fills in for y");
    assert_eq!(y_binding, "5");
}

#[test]
fn missing_arg_no_default_declines() {
    let out = inlined("proc f {x y} { puts $x }\nf 1\n");
    assert_eq!(top_calls_to(&out, "f"), 1);
}

#[test]
fn early_return_in_if_inlines_via_wrap() {
    // tclsh (8.6, 9.0): `proc f {x} { if {$x > 0} { return 1 }; return 0 };
    // f 5` → 1; the inlined site routes both returns through the wrap and
    // forwards the value.
    let out = inlined("proc f {x} { if {$x > 0} { return 1 }\nreturn 0 }\nf 5\n");
    assert_eq!(top_calls_to(&out, "f"), 0);
    assert!(
        out.top_level
            .statements
            .iter()
            .any(|s| matches!(s, Statement::While { .. })),
        "early return is wrapped in a one-shot while-loop"
    );
    assert!(
        out.top_level
            .statements
            .iter()
            .any(|s| matches!(s, Statement::Return { .. })),
        "terminal call appends the trailing `return $__result`"
    );
}

#[test]
fn implicit_trailing_return_with_early_return_captured() {
    // The wrap must capture the implicit trailing value (`set y 2` returns 2)
    // on the if-false fall-through.
    // tclsh (8.6, 9.0): `proc f {x} { if {$x} { return 1 }; set y 2 }; f 0`
    // → 2 — the fall-through value must be forwarded, not silently "".
    let out = inlined("proc f {x} { if {$x} { return 1 }\nset y 2 }\nf 5\n");
    assert_eq!(top_calls_to(&out, "f"), 0);
    let loop_stmt = out
        .top_level
        .statements
        .iter()
        .find(|s| matches!(s, Statement::While { .. }))
        .expect("a one-shot while-wrap is emitted");
    let Statement::While { body, .. } = loop_stmt else {
        unreachable!()
    };
    let captured = body.statements.iter().any(|s| {
        matches!(s, Statement::AssignValue { name, value, .. }
            if name.ends_with("__RESULT") && value == "2")
    });
    assert!(
        captured,
        "the implicit trailing return value (set y 2 → __RESULT=2) is captured on the fall-through"
    );
}

#[test]
fn implicit_trailing_call_declines() {
    // Capturing an implicit return from a trailing *call* needs command-subst
    // rewriting we don't do, so the inliner declines.
    let out = inlined("proc f {x} { if {$x} { return 1 }\nputs ok }\nf 5\n");
    assert_eq!(top_calls_to(&out, "f"), 1);
}

#[test]
fn return_inside_loop_declines() {
    // A `return` inside a `for` body would be trapped by the break-based wrap,
    // so v3 declines.
    let out = inlined(
        "proc f {} {\n  for {set i 0} {$i < 10} {incr i} {\n    if {$i == 5} { return $i }\n  }\n}\nf\n",
    );
    assert_eq!(top_calls_to(&out, "f"), 1);
}

#[test]
fn return_inside_catch_declines() {
    // `catch` traps the synthesised `break`, so v3 declines.
    let out = inlined("proc f {x} {\n  catch { if {$x < 0} { return -1 } }\n  return $x\n}\nf 5\n");
    assert_eq!(top_calls_to(&out, "f"), 1);
}

#[test]
fn array_element_write_inlines_with_array_rename() {
    // Both array-element writes get the same mangled base with the `(idx)`
    // suffix preserved.
    let out = inlined("proc f {} { set arr(0) 1\nset arr(1) 2 }\nf\n");
    assert_eq!(top_calls_to(&out, "f"), 0);
    let array_writes: Vec<String> = assign_pairs(&out.top_level.statements)
        .into_iter()
        .map(|(n, _)| n)
        .filter(|n| n.contains('('))
        .collect();
    assert_eq!(array_writes.len(), 2);
    let bases: std::collections::HashSet<&str> = array_writes
        .iter()
        .map(|n| n.split('(').next().unwrap())
        .collect();
    assert_eq!(bases.len(), 1, "both writes share one mangled base");
    let base = *bases.iter().next().unwrap();
    assert!(base.starts_with("__inline_") && base.ends_with("__arr"));
    let indices: std::collections::HashSet<String> = array_writes
        .iter()
        .map(|n| {
            n.split('(')
                .nth(1)
                .unwrap()
                .trim_end_matches(')')
                .to_owned()
        })
        .collect();
    assert_eq!(
        indices,
        ["0".to_owned(), "1".to_owned()].into_iter().collect()
    );
}

#[test]
fn array_read_after_write_uses_same_rename() {
    // The read of `$arr(k)` after the write resolves to the same renamed array.
    let out = inlined("proc f {} { set arr(k) \"hi\"\nputs $arr(k) }\nf\n");
    let write_name = out
        .top_level
        .statements
        .iter()
        .find_map(|s| match s {
            Statement::AssignValue { name, .. } | Statement::AssignConst { name, .. }
                if name.contains('(') =>
            {
                Some(name.clone())
            }
            _ => None,
        })
        .expect("an array-element write is emitted");
    let write_base = write_name.split('(').next().unwrap().to_owned();
    let puts_call = out
        .top_level
        .statements
        .iter()
        .find(|s| matches!(s, Statement::Call { command, .. } if command == "puts"))
        .expect("the spliced puts");
    let Statement::Call { args, .. } = puts_call else {
        unreachable!()
    };
    assert!(
        args.iter().any(|a| a.contains(&write_base)),
        "the read targets the same renamed array base"
    );
}

#[test]
fn nested_control_flow_inlines() {
    // A body with a nested `if`/`puts` inlines and the If survives.
    let out = inlined("proc f {x} {\n  if {$x > 0} { puts \"positive\" }\n}\nf 5\n");
    assert_eq!(top_calls_to(&out, "f"), 0);
    assert!(
        out.top_level
            .statements
            .iter()
            .any(|s| matches!(s, Statement::If { .. })),
        "the nested if survives the inline"
    );
}

#[test]
fn two_call_sites_get_distinct_mangling() {
    // Each call site gets a unique mangling counter so the two bound slots
    // don't collide.
    let out = inlined("proc f {x} { puts $x }\nf 1\nf 2\n");
    let param_writes: Vec<String> = out
        .top_level
        .statements
        .iter()
        .filter_map(|s| match s {
            Statement::AssignValue { name, .. } | Statement::AssignConst { name, .. }
                if name.starts_with("__inline_") =>
            {
                Some(name.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(param_writes.len(), 2);
    assert_ne!(param_writes[0], param_writes[1]);
}

// --- TestDeadProcElimination -----------------------------------------------

#[test]
fn inlinable_proc_kept_after_inlining() {
    // User procs are externally observable Tcl commands, so they survive even
    // when every static call site is spliced away.
    let out = inlined("proc noop {} {}\nnoop\nnoop\n");
    assert!(out.procedures.contains_key("::noop"));
}

#[test]
fn unreferenced_inlinable_proc_kept() {
    let out = inlined("proc lonely {} {}\n");
    assert!(out.procedures.contains_key("::lonely"));
}

#[test]
fn non_inlinable_proc_kept_even_if_unreferenced() {
    let out = inlined("proc opaque {name} { upvar 1 $name v }\n");
    assert!(out.procedures.contains_key("::opaque"));
}

// (There is no `compiler_synthetic`-drop test: the `Procedure` type carries no
//  `compiler_synthetic` flag, so dead-proc elimination is documented as omitted
//  in `inline_module`'s doc-comment. There is no gate to exercise.)

// --- TestSingleCallWrapperSplice (v1) --------------------------------------

#[test]
fn wrapper_call_is_replaced_with_inner() {
    // tclsh (8.6, 9.0): `proc setup {} { puts "starting" }; setup` prints
    // "starting"; the splice replaces the wrapper call with the inner puts.
    let module = module_for("proc setup {} { puts \"starting\" }\nsetup\n");
    assert_eq!(top_calls_to(&module, "setup"), 1);
    assert_eq!(top_calls_to(&module, "puts"), 0);
    let out = inline_module(module, &reg());
    assert_eq!(top_calls_to(&out, "setup"), 0);
    assert_eq!(top_calls_to(&out, "puts"), 1);
}

#[test]
fn wrapper_with_args_at_call_site_declined() {
    // Passing args to a zero-param wrapper would discard their evaluation;
    // decline.
    let out = inlined("proc setup {} { puts \"hi\" }\nsetup [list 1 2]\n");
    assert_eq!(top_calls_to(&out, "setup"), 1);
    assert_eq!(top_calls_to(&out, "puts"), 0);
}

#[test]
fn wrapper_with_unqualified_proc_call_declined() {
    // The wrapped bare `helper` resolves to a tracked proc, which could bind
    // differently from another namespace; the verbatim splice declines (it is
    // not namespace-invariant). The `setup` call therefore stays. (After
    // first-pass helper-inlining, setup's body would empty; a second pass is
    // not run here.)
    let out = inlined("proc helper {} {}\nproc setup {} { helper }\nsetup\n");
    assert_eq!(top_calls_to(&out, "setup"), 1);
}

#[test]
fn qualified_registry_command_body_is_inlined_without_rewriting_the_head() {
    // Registry invocation resolution normalises the absolute spelling
    // `::puts` to the same FRAMELESS_RUNTIME command facts as `puts`.
    // A fully-qualified head is also namespace-invariant, so lifting it from
    // the wrapper cannot change which command Tcl dispatches.  The splice must
    // retain the written `::puts` head rather than canonicalising emitted IR.
    let module = module_for("proc setup {} { ::puts \"start\" }\nsetup\n");
    let registry = reg();
    let summaries = analyse_var_escape_with_registry(&module, true, &registry);
    assert!(
        summaries
            .get("::setup")
            .is_some_and(ProcEscapeSummary::safe_to_inline),
        "an absolute spelling must receive its registry command's frameless facts"
    );
    let out = inline_module(module, &registry);
    assert_eq!(top_calls_to(&out, "setup"), 0);
    assert_eq!(top_calls_to(&out, "::puts"), 1);
    assert_eq!(top_calls_to(&out, "puts"), 0);
}

// --- TestResolution --------------------------------------------------------

#[test]
fn unqualified_call_in_namespace() {
    // A bare `noop` inside `::ns::caller` resolves to `::ns::noop` and is
    // inlined.
    let module =
        module_for("namespace eval ::ns {\n  proc noop {} {}\n  proc caller {} { noop }\n}\n");
    let summaries = analyse_var_escape(&module, true);
    let proc = &module.procedures["::ns::noop"];
    assert_eq!(
        classify_proc(proc, summary_for(&summaries, "::ns::noop"), 0),
        InlineDecision::Always
    );
    let out = inline_module(module, &reg());
    let caller = &out.procedures["::ns::caller"];
    assert_eq!(calls_to(&caller.body.statements, "noop"), 0);
}

#[test]
fn namespace_eval_body_is_opaque_barrier_not_a_block() {
    // A top-level `namespace eval ::ns { … }` lowers to an opaque
    // `Statement::Barrier` that carries the unparsed body *as a string
    // argument* — there is no recursable statement body, so the inner `noop`
    // call site is unreachable to the general inliner. The proc itself is still
    // registered (`::ns::noop`). This test pins that lowering shape; the
    // proc-body resolution case (which does work) is covered by
    // `unqualified_call_in_namespace` above.
    let out = inlined("namespace eval ::ns {\n  proc noop {} {}\n  noop\n}\n");
    assert!(
        out.procedures.contains_key("::ns::noop"),
        "the namespaced proc is registered"
    );
    assert!(
        out.top_level.statements.iter().any(|s| matches!(
            s,
            Statement::Barrier { command, .. } if command == "namespace"
        )),
        "top-level namespace eval lowers to an opaque Barrier, not a recursable Block"
    );
    assert!(
        !out.top_level
            .statements
            .iter()
            .any(|s| matches!(s, Statement::Block { .. })),
        "no recursable Block is produced for the namespace-eval body in Rust"
    );
}

// ===========================================================================
// Inlining ↔ GVN interaction
// ===========================================================================
//
// The GVN pass itself has no public integration entry point. The single
// behavioural fact GVN and the inliner SHARE is the splice-safe /
// frame-independent command allow-list: a `puts` between two computations does
// not invalidate either pass's reasoning, whereas a user-defined helper call
// does. The inliner exercises exactly that allow-list, so the shared invariant
// is proven here through the inliner's verbatim path.

#[test]
fn gvn_shared_allowlist_puts_is_frame_independent() {
    // `puts` is frame-independent (splice-safe), so a zero-param wrapper of two
    // `puts` calls inlines verbatim — the same allow-list property GVN reuses
    // to keep its value table across an intervening `puts`.
    let out = inlined("proc w {} { puts \"a\"\nputs \"b\" }\nw\n");
    assert_eq!(top_calls_to(&out, "w"), 0);
    assert_eq!(top_calls_to(&out, "puts"), 2);
}

#[test]
fn gvn_shared_allowlist_user_call_is_opaque() {
    // A user-defined helper call is NOT frame-independent, so a wrapper whose
    // body calls it is not verbatim-spliceable into another namespace. The
    // wrapper call to `w` stays — GVN likewise treats such a call as
    // potentially mutating any local.
    let out = inlined("proc helper {} { return 0 }\nproc w {} { helper }\nw\n");
    assert_eq!(top_calls_to(&out, "w"), 1);
}

// ===========================================================================
// Whole-callee uplevel-passthrough inlining
// ===========================================================================

// --- TestPassthroughRecognition --------------------------------------------

#[test]
fn zero_param_single_uplevel_is_candidate() {
    // The callsite becomes a Block splicing the body, not a `reset` call.
    // tclsh (8.6, 9.0): `proc reset {} { uplevel 1 {set counter 0} }; proc
    // run {} { set counter 10; reset; return $counter }; run` → 0 — the
    // body runs in the caller's frame; inlining as a same-frame Block
    // preserves that.
    let out =
        uplevel_inlined("proc reset {} { uplevel 1 {set counter 0} }\nproc caller {} { reset }\n");
    let caller = &out.procedures["::caller"];
    let first = caller.body.statements.first().expect("non-empty body");
    assert!(
        matches!(first, Statement::Block { .. }),
        "callsite collapses to a Block, got {first:?}"
    );
    assert_eq!(
        calls_to(&caller.body.statements, "reset"),
        0,
        "no residual call to reset"
    );
}

#[test]
fn non_passthrough_not_inlined() {
    // Real work before the uplevel disqualifies the passthrough shape.
    let out = uplevel_inlined(
        "proc worker {} {\n  set local 1\n  uplevel 1 {set g 2}\n}\nproc caller {} { worker }\n",
    );
    let caller = &out.procedures["::caller"];
    let first = caller.body.statements.first().expect("non-empty body");
    assert!(
        matches!(first, Statement::Call { command, .. } if command == "worker"),
        "non-passthrough callsite stays a Call, got {first:?}"
    );
}

#[test]
fn non_unit_frame_shift_not_inlined() {
    // `uplevel #0` goes to the absolute global frame; not a same-frame inline.
    let out =
        uplevel_inlined("proc reset {} { uplevel #0 {set ::g 0} }\nproc caller {} { reset }\n");
    let caller = &out.procedures["::caller"];
    let first = caller.body.statements.first().expect("non-empty body");
    assert!(
        matches!(first, Statement::Call { .. } | Statement::Barrier { .. }),
        "uplevel #0 keeps the runtime call, got {first:?}"
    );
}

#[test]
fn non_body_param_not_inlined() {
    // A quoted (substituting) body, not the literal `uplevel 1 $param` shape,
    // is not recognised.
    let out = uplevel_inlined(
        "proc setter {n} { uplevel 1 \"set counter $n\" }\nproc caller {} { setter 5 }\n",
    );
    let caller = &out.procedures["::caller"];
    let first = caller.body.statements.first().expect("non-empty body");
    assert!(matches!(
        first,
        Statement::Call { .. } | Statement::Barrier { .. }
    ));
}

#[test]
fn nested_uplevel_poisons_inlining() {
    // A body that itself does `uplevel` would reach the caller's caller after
    // inlining; reject conservatively.
    let out = uplevel_inlined(
        "proc double_up {} { uplevel 1 { uplevel 1 {set g 1} } }\nproc caller {} { double_up }\n",
    );
    let caller = &out.procedures["::caller"];
    let first = caller.body.statements.first().expect("non-empty body");
    assert!(matches!(
        first,
        Statement::Call { .. } | Statement::Barrier { .. }
    ));
}

#[test]
fn args_at_callsite_prevents_inline() {
    // An unexpected arg would error at runtime; don't inline the error away.
    let out = uplevel_inlined(
        "proc reset {} { uplevel 1 {set g 0} }\nproc caller {} { reset extra_arg }\n",
    );
    let caller = &out.procedures["::caller"];
    let first = caller.body.statements.first().expect("non-empty body");
    assert!(matches!(
        first,
        Statement::Call { .. } | Statement::Barrier { .. }
    ));
}

// --- detection-only helpers (detect_static_passthrough / body_has_frame_reach) ---

#[test]
fn detect_static_passthrough_recognises_zero_param() {
    // Direct recognition assertion: `detect_static_passthrough` exposes the
    // candidate set explicitly.
    let m = module_for("proc reset {} { uplevel 1 {set counter 0} }\n");
    let candidates = detect_static_passthrough(&m, &reg());
    assert_eq!(candidates.len(), 1);
    assert!(candidates.contains_key("::reset"));
}

#[test]
fn detect_static_passthrough_rejects_param_and_frame_reach() {
    // A param proc is not the *static* zero-param shape, and a nested upvar
    // body fails the frame-reach gate — both excluded from the static set.
    let with_param = module_for("proc reset {x} { uplevel 1 {set counter 0} }\n");
    assert!(detect_static_passthrough(&with_param, &reg()).is_empty());
    let nested_reach = module_for("proc bind {} { uplevel 1 {upvar foo bar} }\n");
    assert!(detect_static_passthrough(&nested_reach, &reg()).is_empty());
}

#[test]
fn body_has_frame_reach_flags_uplevel_and_upvar() {
    // `body_has_frame_reach` is the public gate the rewriter consults on the
    // callsite's materialised body. A plain assignment does not reach; an
    // `uplevel` / `upvar` body does.
    let plain = module_for("set x 1\n");
    assert!(!body_has_frame_reach(&plain.top_level, &reg()));
    let reaches = module_for("uplevel 1 {set x 1}\n");
    assert!(body_has_frame_reach(&reaches.top_level, &reg()));
    let upvar = module_for("upvar 1 foo bar\n");
    assert!(body_has_frame_reach(&upvar.top_level, &reg()));
}

// --- TestInliningIdempotence -----------------------------------------------

#[test]
fn uplevel_running_twice_is_noop() {
    let mut m = module_for("proc reset {} { uplevel 1 {set g 0} }\nproc caller {} { reset }\n");
    inline_uplevel_passthrough(&mut m, &reg());
    let first = m.procedures["::caller"].body.clone();
    inline_uplevel_passthrough(&mut m, &reg());
    let second = &m.procedures["::caller"].body;
    assert_eq!(first.statements.len(), second.statements.len());
    assert_eq!(&first, second, "second pass is structurally a no-op");
}

// --- TestParamBodyPassthrough ----------------------------------------------

#[test]
fn param_body_explicit_level_one_call_site_inlined() {
    // tclsh (8.6, 9.0): `proc dispatcher {body} { uplevel 1 $body }; proc run
    // {} { set counter 10; dispatcher {set counter 0}; return $counter }; run`
    // → 0 — the brace-literal body runs in the caller's frame.
    let out = uplevel_inlined(
        "proc dispatcher {body} { uplevel 1 $body }\nproc caller {} { dispatcher {set x 1} }\n",
    );
    let caller = &out.procedures["::caller"];
    let first = caller.body.statements.first().expect("non-empty body");
    assert!(
        matches!(first, Statement::Block { .. }),
        "the brace-literal callsite collapses to a Block, got {first:?}"
    );
}

#[test]
fn param_body_implicit_level_call_site_inlined() {
    // `uplevel $body` (implicit level 1) matches the same shape.
    let out = uplevel_inlined(
        "proc dispatcher2 {body} { uplevel $body }\nproc caller {} { dispatcher2 {set x 1} }\n",
    );
    let caller = &out.procedures["::caller"];
    let first = caller.body.statements.first().expect("non-empty body");
    assert!(matches!(first, Statement::Block { .. }));
}

#[test]
fn param_body_dynamic_arg_at_call_site_stays_call() {
    // A `$var` arg is an unknown body at compile time; runtime dispatch stays.
    let out = uplevel_inlined(
        "proc dispatcher {body} { uplevel 1 $body }\nproc caller {} { set b {set x 1}\ndispatcher $b }\n",
    );
    let caller = &out.procedures["::caller"];
    assert!(
        caller
            .body
            .statements
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } | Statement::Barrier { command, .. } if command == "dispatcher")),
        "dynamic arg keeps the dispatcher call"
    );
}

#[test]
fn param_body_wrong_arity_at_call_site_stays_call() {
    let out = uplevel_inlined(
        "proc dispatcher {body} { uplevel 1 $body }\nproc caller {} { dispatcher {set x 1} extra }\n",
    );
    let caller = &out.procedures["::caller"];
    let first = caller.body.statements.first().expect("non-empty body");
    assert!(matches!(
        first,
        Statement::Call { .. } | Statement::Barrier { .. }
    ));
}

#[test]
fn param_body_multiple_params_not_recognised() {
    // Two params fail the single-body-param gate at *detection* time.
    let m = module_for("proc dispatcher2 {a body} { uplevel 1 $body }\n");
    assert!(detect_passthrough_candidates(&m, &reg()).is_empty());
    let out = uplevel_inlined(
        "proc dispatcher2 {a body} { uplevel 1 $body }\nproc caller {} { dispatcher2 x {set x 1} }\n",
    );
    let caller = &out.procedures["::caller"];
    let first = caller.body.statements.first().expect("non-empty body");
    assert!(matches!(
        first,
        Statement::Call { .. } | Statement::Barrier { .. }
    ));
}

#[test]
fn param_body_non_param_var_reference_not_recognised() {
    // The body `$other` is not the sole param name.
    let m = module_for("proc dispatcher {body} { uplevel 1 $other }\n");
    assert!(detect_passthrough_candidates(&m, &reg()).is_empty());
}

#[test]
fn param_body_explicit_level_other_than_one_not_recognised() {
    // `uplevel 2 $body` is a deeper shift with different semantics.
    let m = module_for("proc dispatcher {body} { uplevel 2 $body }\n");
    assert!(detect_passthrough_candidates(&m, &reg()).is_empty());
}

#[test]
fn param_body_shape_carries_param_name() {
    // The `detect_passthrough_candidates` map distinguishes the ParamBody
    // shape and records the sole parameter name (exercising the public
    // `PassthroughShape` enum directly).
    let m = module_for("proc dispatcher {body} { uplevel 1 $body }\n");
    let candidates = detect_passthrough_candidates(&m, &reg());
    match candidates.get("::dispatcher") {
        Some(PassthroughShape::ParamBody { param_name }) => assert_eq!(param_name, "body"),
        other => panic!("expected ParamBody {{ body }}, got {other:?}"),
    }
}

#[test]
fn static_shape_carries_body() {
    // The Static shape carries the pre-lowered body to splice.
    let m = module_for("proc reset {} { uplevel 1 {set counter 0} }\n");
    let candidates = detect_passthrough_candidates(&m, &reg());
    match candidates.get("::reset") {
        Some(PassthroughShape::Static { body }) => {
            assert!(!body.statements.is_empty(), "static body is non-empty");
        }
        other => panic!("expected Static, got {other:?}"),
    }
}
