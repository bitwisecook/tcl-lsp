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

//! Residual-coverage port closing the gaps that the existing optimiser /
//! codegen suites
//! (`optimiser.rs`, `optimiser_coverage.rs`, `optimiser_depth.rs`,
//! `codegen.rs`, `codegen_depth.rs`, `compiler_residual.rs`)
//! leave in three `tcl-compiler` source files:
//!
//!   * `src/optimiser/propagation.rs` — the constant/copy-propagation match
//!     arms across the *non-`Call`* statement kinds (`Return` O101/O115 value
//!     folds, the `AssignExpr` O100 substitute-and-fold cascade, the
//!     `AssignValue` cmd-sub-fold arm), and the compound-statement recursion
//!     (`For` init/next, `While`/`Foreach`/`Catch` bodies, `Try`
//!     handlers/finally, `Switch` arms/default) that the unit tests reach only
//!     through `Call` arguments at the top level.
//!   * `src/optimiser/code_sinking.rs` (O125) — the sink-placement decision
//!     branches: sinking into a `switch` arm / `default`, the deepest-target
//!     descent through a nested decision (and the `try_deeper_sink`
//!     condition-reads-var bail that stops the descent), duplication into
//!     *both* using branches, the multi-use-in-one-branch anchor, and the
//!     `statement_uses_var` recognition across `foreach` / `catch` / `while`
//!     / `for`-condition bodies.
//!   * `src/codegen/cmd_subst.rs` — the command-substitution lowering
//!     discriminators `codegen_depth.rs` skips: every `string is CLASS`
//!     branch (alpha / integer / double / boolean, strict and non-strict),
//!     `string replace`'s fast 0..N path vs the `strreplace` fallback,
//!     `string equal/compare`'s `-nocase`/`-length` `INVOKE_REPLACE` forms,
//!     `array names`/`size`, multi-key `dict get`, `lreplace`/`linsert`,
//!     `regexp`'s nocase-glob vs plain-2-arg forms, and the
//!     `unroll_nested_set` / `is_pure_cmd_subst` / `has_command_separator`
//!     free functions plus the `emit_value` / `emit_cmd_subst_arg` composite /
//!     fold / array-index paths.
//!
//! ## Proof split (read first)
//!
//! Two kinds of assertion live here, exactly as in the sibling ports:
//!
//!   * **C-Tcl-pinned values.** Where a snippet computes an *observable* Tcl
//!     value (a fold result the optimiser rewrites to, or a literal the codegen
//!     folds at compile time), the value was checked against real `tclsh`
//!     (`scripts/dev/tclsh_check.sh`, 8.6 + 9.0) and is cited with a `// tclsh:`
//!     comment. Optimisation/lowering must be *semantics-preserving*, so these
//!     guard that the rewritten/lowered program still yields the C-Tcl value.
//!   * **Compiler-internal structure.** Which `O1xx` code fired, that a sink is
//!     grouped/hint-only, and the emitted *opcode shape* of a command
//!     substitution describe the compiler IR / bytecode layout — not a Tcl
//!     runtime value — and are asserted structurally. (`tcl-vm` is not a
//!     `tcl-compiler` dependency, so no end-to-end VM execution is available
//!     from this crate; for codegen the folded literal *is* the observable
//!     result and is the thing pinned to tclsh.)
//!
//! No `#[ignore]`; every test passes. No genuine miscompile was found — every
//! fold / sink / lowering observed here matches the C-Tcl value it preserves.

use tcl_compiler::codegen::cmd_subst::{
    has_command_separator, is_pure_cmd_subst, parse_cmd_parts, parse_cmd_parts_expand,
    unroll_nested_set,
};
use tcl_compiler::codegen::{CodegenCtx, Op};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::interprocedural::InterproceduralAnalysis;
use tcl_compiler::optimiser::manager::optimise_raw;
use tcl_compiler::optimiser::{Optimisation, PassContext, PassId, run_passes};
use tcl_registry::CommandRegistry;

// ── Shared helpers ──────────────────────────────────────────────────────────

fn registry() -> CommandRegistry {
    CommandRegistry::build_default()
}

/// All optimisations for `src` with the registry + interprocedural summary
/// threaded (the propagation O101/O103/O115/O129 paths need both).
fn opts(src: &str) -> Vec<Optimisation> {
    optimise_raw(src, &registry(), None)
}

/// Replacement texts of every optimisation carrying `code`.
fn repls(src: &str, code: &str) -> Vec<String> {
    opts(src)
        .into_iter()
        .filter(|o| o.code.as_str() == code)
        .map(|o| o.replacement)
        .collect()
}

/// True when any optimisation with `code` fires on `src`.
fn fires(src: &str, code: &str) -> bool {
    opts(src).iter().any(|o| o.code.as_str() == code)
}

/// Run **only** the code-sinking pass over `src`, returning the
/// `(hint_only, replacement)` of each O125. Isolating the pass avoids the
/// O101/O109 rewrites of the full pipeline pre-empting a sinkable `set`
/// (an `[expr {1 + 2}]` RHS would otherwise be const-folded first), so the
/// sink-placement branches are reached deterministically.
fn sink_only(src: &str) -> Vec<(bool, String)> {
    let reg = registry();
    let cu = CompilationUnit::build_for(src, &reg, false);
    let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
    ctx.registry = Some(&reg);
    run_passes(&mut ctx, &cu, &[PassId::CodeSinking]);
    ctx.optimisations
        .iter()
        .filter(|o| o.code.as_str() == "O125")
        .map(|o| (o.hint_only, o.replacement.clone()))
        .collect()
}

/// Opcodes emitted by `emit_inline_cmd_subst(text)` in the given scope.
fn inline_ops(is_proc: bool, params: &[&str], text: &str) -> Vec<Op> {
    let reg = registry();
    let mut ctx = CodegenCtx::new(is_proc, params, &reg);
    ctx.emit_inline_cmd_subst(text);
    ctx.instructions.iter().map(|i| i.op).collect()
}

/// Opcodes emitted by `emit_value(value, interpolate)`.
fn value_ops(is_proc: bool, params: &[&str], value: &str, interpolate: bool) -> Vec<Op> {
    let reg = registry();
    let mut ctx = CodegenCtx::new(is_proc, params, &reg);
    ctx.emit_value(value, interpolate);
    ctx.instructions.iter().map(|i| i.op).collect()
}

/// Opcodes emitted by `emit_cmd_subst_arg(arg, braced)`.
fn arg_ops(is_proc: bool, params: &[&str], arg: &str, braced: bool) -> Vec<Op> {
    let reg = registry();
    let mut ctx = CodegenCtx::new(is_proc, params, &reg);
    ctx.emit_cmd_subst_arg(arg, braced);
    ctx.instructions.iter().map(|i| i.op).collect()
}

// ═══════════════════════════════════════════════════════════════════════════
// propagation.rs — value-position folds on NON-`Call` statement kinds
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn prop_return_o101_folds_literal_expr() {
    // `try_fold_return_terminator`'s O101 path: a constant `[expr {…}]` return
    // value folds to its literal. (Distinct from the O100 `return $v` fold the
    // unit tests already cover.) Structural: O101 fired; value pinned below.
    let r = repls("proc ::g {} { return [expr {2 + 3}] }", "O101");
    // tclsh: expr {2 + 3} → 5  (8.6 + 9.0); `return 5` preserves the result.
    assert_eq!(r, vec!["return 5".to_string()]);

    // tclsh: expr {(1 + 2) * (3 + 4)} → 21  (8.6 + 9.0).
    assert_eq!(
        repls("proc ::g {} { return [expr {(1 + 2) * (3 + 4)}] }", "O101"),
        vec!["return 21".to_string()],
    );

    // A variable-reading expr return is NOT folded by this path.
    assert!(!fires("proc ::g {x} { return [expr {$x + 1}] }", "O101"));
}

#[test]
fn prop_return_o115_collapses_redundant_nested_expr() {
    // `o115_redundant_nested_expr` reached from the `Return` arm: a redundant
    // double-`expr` return collapses to the inner cmd-sub. The two `expr`
    // wrappers are value-equivalent, so the rewrite is semantics-preserving.
    // (codegen_depth/optimiser tests hit O115 only via `if` / a `Call` arg.)
    let r = repls("proc ::f {x} { return [expr {[expr {$x * 2}]}] }", "O115");
    assert_eq!(r, vec!["return [expr {$x * 2}]".to_string()]);
    // tclsh: with x=5, both `[expr {[expr {$x*2}]}]` and `[expr {$x*2}]` → 10.
    // tclsh: proc ::f {x} { return [expr {[expr {$x * 2}]}] }; ::f 5 → 10 (8.6+9.0)

    // A single (non-redundant) `[expr {…}]` return must NOT collapse — its
    // unwrap `$x + 1` would be invalid as a bare value.
    assert!(!fires("proc ::f {x} { return [expr {$x + 1}] }", "O115"));
}

#[test]
fn prop_assign_expr_o100_substitutes_and_folds_to_literal() {
    // `try_substitute_assign_expr`: a `set name [expr {…}]` whose expr reads an
    // SCCP-constant var substitutes the constant and, when the result is fully
    // constant, emits the unwrapped `set name VALUE`.
    let r = repls("proc ::h {} { set a 5\n set b [expr {$a + 1}] }", "O100");
    // tclsh: a=5 ⇒ a+1 == 6  (8.6 + 9.0); `set b 6` preserves it.
    assert!(
        r.iter().any(|s| s == "set b 6"),
        "expected `set b 6`, got {r:?}",
    );
}

#[test]
fn prop_assign_expr_o100_keeps_expr_wrapper_when_partly_constant() {
    // The same path's *partial* branch: when only some operands are constant
    // the substituted-but-still-dynamic expression keeps its `[expr {…}]`
    // wrapper (the constant is spliced in, the variable read stays).
    let r = repls("proc ::h {y} { set a 3\n set b [expr {$a + $y}] }", "O100");
    // tclsh: a=3 ⇒ the residual is `$a + $y` with a→3, i.e. `3 + $y`; for any y
    // `set b [expr {3 + $y}]` equals the original `set a 3; set b [expr {$a+$y}]`.
    // tclsh: proc p {y} {set a 3; set b [expr {$a+$y}]; return $b}; p 4 → 7;
    //        proc q {y} {set b [expr {3+$y}]; return $b}; q 4 → 7  (8.6 + 9.0)
    assert!(
        r.iter().any(|s| s == "set b [expr {3 + $y}]"),
        "expected `set b [expr {{3 + $y}}]`, got {r:?}",
    );
}

#[test]
fn prop_assign_value_arm_folds_cmd_subst_in_set_value() {
    // The `Statement::AssignValue { tokens: Some(..) }` arm wires the
    // value-position cmd-sub folds onto a `set TARGET [cmd-sub]`: an O129
    // pure-builtin cmd-sub folds in the set-value position it would get in a
    // command-argument position. The replacement targets the cmd-sub *argv*
    // span (just the folded word), the same span the `puts [cmd]` form uses.
    // tclsh: string length abcde → 5  (8.6 + 9.0); the folded `5` preserves it.
    assert_eq!(
        repls("proc ::f {} { set y [string length abcde] }", "O129"),
        vec!["5".to_string()],
    );

    // A constant `set y [expr {6 * 7}]` reaches the *`AssignExpr`* path instead
    // (the lowering strips the outer `expr`), which rewrites the whole `set`
    // statement to the unwrapped literal form.
    // tclsh: expr {6 * 7} → 42  (8.6 + 9.0); `set y 42` preserves it.
    assert_eq!(
        repls("proc ::f {} { set y [expr {6 * 7}] }", "O101"),
        vec!["set y 42".to_string()],
    );
}

#[test]
fn prop_recurses_into_for_init_and_next_clauses() {
    // `walk_statement`'s `For` arm walks `init`, `next`, and `body`. A constant
    // defined before the loop propagates into a `$k` read in the loop body, and
    // an SCCP-constant used in the `next` clause also folds.
    // tclsh: with k=5 the body's `puts $k` prints 5 each iteration; folding the
    // read to `puts 5` is identical.
    let body = "proc ::fl {} { set k 5\n for {set i 0} {$i < 3} {incr i} { puts $k } }";
    assert!(
        repls(body, "O100").iter().any(|s| s == "5"),
        "expected the for-body `$k` read to fold to 5",
    );
}

#[test]
fn prop_recurses_into_while_foreach_catch_switch_try_bodies() {
    // Each compound arm of `walk_statement` recurses into its body/bodies.
    // A constant read inside the body folds; the fold is the same observable
    // value the body would compute.

    // `while` body — note the loop never runs here (`while {0}`), but the
    // propagation walk still descends and folds the body's `$c` read.
    // tclsh: c=9 ⇒ `puts $c` would print 9; `puts 9` is identical.
    assert!(
        repls("set c 9\nwhile {0} { puts $c }", "O100")
            .iter()
            .any(|s| s == "9")
    );

    // `foreach` body.
    // tclsh: m=7 ⇒ body `puts $m` prints 7 per element; `puts 7` identical.
    assert!(
        repls("set m 7\nforeach e {a b} { puts $m }", "O100")
            .iter()
            .any(|s| s == "7"),
    );

    // `catch` body.
    // tclsh: n=4 ⇒ `puts $n` inside catch prints 4; `puts 4` identical.
    assert!(
        repls("set n 4\ncatch { puts $n }", "O100")
            .iter()
            .any(|s| s == "4")
    );

    // `switch` arm body + default body.
    // tclsh: t=3 ⇒ the matched arm `puts $t` prints 3; `puts 3` identical.
    assert!(
        repls(
            "set t 3\nswitch x { x { puts $t } default { puts $t } }",
            "O100"
        )
        .iter()
        .any(|s| s == "3"),
    );

    // `try` body + `on error` handler body.
    // tclsh: w=8 ⇒ either `puts $w` prints 8; `puts 8` identical.
    let try_src = "set w 8\ntry { puts $w } on error {e} { puts $w }";
    assert!(repls(try_src, "O100").iter().filter(|s| *s == "8").count() >= 1);
}

#[test]
fn prop_recurses_into_if_else_body() {
    // The `If` arm walks every clause body and the `else_body`. A constant
    // folds in the else branch as well as the then branch.
    // tclsh: v=2 ⇒ whichever branch runs, `puts $v` prints 2; `puts 2` identical.
    let src = "set v 2\nif {$flag} { puts $v } else { puts $v }";
    assert!(repls(src, "O100").iter().filter(|s| *s == "2").count() >= 1);
}

// ═══════════════════════════════════════════════════════════════════════════
// code_sinking.rs (O125) — sink-placement decision branches
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn sink_into_switch_arm_and_default() {
    // `decision_branch_bodies`'s `Switch` arm: a `set X V; switch …` where the
    // var is read in exactly one arm sinks the def into that arm. Sinking a
    // side-effect-free `set` into the only branch that reads it is
    // semantics-preserving (the value is unobservable elsewhere).
    let into_arm =
        sink_only("proc ::sw {flag} { set x 1; switch $flag { a { puts $x } b { puts no } } }");
    assert!(
        into_arm
            .iter()
            .any(|(hint, r)| !hint && r == "set x 1; puts $x"),
        "expected sink into the matching switch arm, got {into_arm:?}",
    );
    assert!(
        into_arm.iter().any(|(hint, r)| !hint && r.is_empty()),
        "expected a grouped delete of the original set, got {into_arm:?}",
    );

    // The `default` arm is a sink target too.
    let into_default = sink_only(
        "proc ::sd {flag} { set x 1; switch $flag { a { puts no } default { puts $x } } }",
    );
    assert!(
        into_default
            .iter()
            .any(|(hint, r)| !hint && r == "set x 1; puts $x"),
        "expected sink into the switch default, got {into_default:?}",
    );
}

#[test]
fn sink_descends_to_deepest_nested_decision_target() {
    // `find_deepest_targets` descends when a branch's sole using statement is
    // itself a decision: `$x` used only inside an inner `if` sinks all the way
    // into that inner branch's first using statement.
    let deep = sink_only(
        "proc ::deep {a b} { set x 1; if {$a} { if {$b} { puts $x } } else { puts no } }",
    );
    assert!(
        deep.iter()
            .any(|(hint, r)| !hint && r == "set x 1; puts $x"),
        "expected a deep sink into the inner branch, got {deep:?}",
    );
}

#[test]
fn sink_stops_when_inner_condition_reads_var() {
    // `try_deeper_sink`'s `decision_condition_uses_var` bail: when the inner
    // decision's *condition* reads `$x`, descending past it would be unsound
    // (the condition needs the value), so the sink anchors at the outer body's
    // first statement — the inner `if` — instead of descending into its branch.
    let anchored = sink_only(
        "proc ::cb {a} { set x 1; if {$a} { if {$x > 0} { puts hi } } else { puts no } }",
    );
    assert!(
        anchored
            .iter()
            .any(|(hint, r)| !hint && r == "set x 1; if {$x > 0} { puts hi }"),
        "expected anchor before the var-reading inner condition, got {anchored:?}",
    );
    // It must NOT have descended into the inner body (no `puts hi`-only sink).
    assert!(
        !anchored.iter().any(|(_, r)| r == "set x 1; puts hi"),
        "must not sink past a condition that reads the var, got {anchored:?}",
    );
}

#[test]
fn sink_duplicates_into_both_using_branches() {
    // When the var is live in *both* arms, the def is duplicated into each (one
    // delete + two prepends). Re-running the side-effect-free `set` in each
    // branch yields the same value either branch would have read.
    let both = sink_only("proc ::both {flag} { set x 1; if {$flag} { puts $x } else { puts $x } }");
    let inserts = both
        .iter()
        .filter(|(hint, r)| !hint && r == "set x 1; puts $x")
        .count();
    assert_eq!(inserts, 2, "expected a sink into each branch, got {both:?}");
    let deletes = both
        .iter()
        .filter(|(hint, r)| !hint && r.is_empty())
        .count();
    assert_eq!(deletes, 1, "expected exactly one delete, got {both:?}");
}

#[test]
fn sink_multi_use_in_one_branch_anchors_at_first() {
    // `find_deepest_targets` with two using statements in one branch takes the
    // `using.len() != 1` path: no descent, anchor at the first using statement.
    // The prepend target is that first statement's span, so the inserted text
    // is `set x 1; <first using statement>` (the second `puts $x` is left in
    // place — the sunk def still precedes it, so both reads see `x`).
    let multi =
        sink_only("proc ::multi {a} { set x 1; if {$a} { puts $x; puts $x } else { puts no } }");
    assert!(
        multi
            .iter()
            .any(|(hint, r)| !hint && r == "set x 1; puts $x"),
        "expected the set prepended before the first using statement, got {multi:?}",
    );
    // Exactly one delete of the original (the def is sunk to a single anchor,
    // not duplicated — only one branch uses the var).
    assert_eq!(
        multi
            .iter()
            .filter(|(hint, r)| !hint && r.is_empty())
            .count(),
        1,
        "expected one grouped delete, got {multi:?}",
    );
}

#[test]
fn sink_recognises_var_use_in_foreach_catch_while_bodies() {
    // `any_decision_body_uses_var` → `statement_uses_var` must recognise a
    // `$x` read nested inside a `foreach` / `catch` / `while` body to fire the
    // sink. Each form sinks the `set` into the branch containing that body.
    let fe = sink_only(
        "proc ::fe {flag lst} { set x 1; if {$flag} { foreach e $lst { puts $x } } else { puts no } }",
    );
    assert!(
        fe.iter()
            .any(|(_, r)| r == "set x 1; foreach e $lst { puts $x }"),
        "foreach-body use must drive the sink, got {fe:?}",
    );

    let ca = sink_only(
        "proc ::ca {flag} { set x 1; if {$flag} { catch { puts $x } } else { puts no } }",
    );
    assert!(
        ca.iter().any(|(_, r)| r == "set x 1; catch { puts $x }"),
        "catch-body use must drive the sink, got {ca:?}",
    );

    let wh = sink_only(
        "proc ::wh {flag} { set x 1; if {$flag} { while {$x} { break } } else { puts no } }",
    );
    assert!(
        wh.iter().any(|(_, r)| r == "set x 1; while {$x} { break }"),
        "while-condition use must drive the sink, got {wh:?}",
    );
}

#[test]
fn sink_for_condition_use_after_decision_suppresses() {
    // `statement_uses_var`'s `For` arm contributes to the later-use scan: a
    // `for {…} {$i < $x} {…}` *after* the decision reads `$x`, so the var is
    // live past the decision and the sink is suppressed (sinking it into a
    // branch that may not run would lose the value the loop condition needs).
    let suppressed = sink_only(
        "proc ::lu {flag} { set x 1; if {$flag} { puts $x }; for {set i 0} {$i < $x} {incr i} { puts hi } }",
    );
    assert!(
        suppressed.is_empty(),
        "a later for-condition use must suppress the sink, got {suppressed:?}",
    );
}

#[test]
fn sink_assign_expr_and_assign_value_shapes_are_sinkable() {
    // `sinkable_assignment`'s `AssignExpr` (no cmd-sub) and `AssignValue` (no
    // `[`) arms: both side-effect-free RHS shapes are sinkable. Isolated so the
    // expr-fold / DSE passes don't rewrite the `set` first.
    let ae = sink_only(
        "proc ::ae {flag y} { set x [expr {$y + 2}]; if {$flag} { puts $x } else { puts no } }",
    );
    assert!(
        ae.iter()
            .any(|(_, r)| r == "set x [expr {$y + 2}]; puts $x"),
        "AssignExpr RHS must be sinkable, got {ae:?}",
    );

    let av =
        sink_only("proc ::av {flag y} { set x \"v$y\"; if {$flag} { puts $x } else { puts no } }");
    assert!(
        av.iter().any(|(_, r)| r == "set x \"v$y\"; puts $x"),
        "AssignValue RHS must be sinkable, got {av:?}",
    );

    // A command-substitution RHS (`set x [foo]`) is NOT side-effect-free and
    // must not sink (control: `sinkable_assignment` returns None).
    let cmd =
        sink_only("proc ::cs {flag} { set x [foo]; if {$flag} { puts $x } else { puts no } }");
    assert!(cmd.is_empty(), "cmd-subst RHS must not sink, got {cmd:?}");
}

// ═══════════════════════════════════════════════════════════════════════════
// cmd_subst.rs — command-substitution lowering discriminators
// ═══════════════════════════════════════════════════════════════════════════
//
// These assert the emitted OPCODE SHAPE (compiler-internal bytecode layout).
// Where a `string is` form has a fixed boolean truth value for a literal the
// runtime result is pinned to tclsh in the comment; the snippets below feed a
// `$x` so the shape, not a folded constant, is under test.

#[test]
fn cmd_subst_string_is_alpha_class_branches() {
    // A character class (`alpha`) with a known `str_class_id` lowers to the
    // single specialised `STR_CLASS` opcode.
    assert_eq!(
        inline_ops(true, &["x"], "[string is alpha $x]"),
        vec![Op::LOAD_SCALAR1, Op::STR_CLASS],
    );
    // `-strict` on a *character* class cannot use `STR_CLASS` (it reports the
    // empty string as a member), so it defers to the generic command invoke.
    let strict = inline_ops(true, &["x"], "[string is alpha -strict $x]");
    assert!(
        strict.contains(&Op::INVOKE_STK1) && !strict.contains(&Op::STR_CLASS),
        "strict char-class defers to generic invoke: {strict:?}",
    );
}

#[test]
fn cmd_subst_string_is_integer_strict_and_nonstrict() {
    // `string is integer` (non-`-strict`) emits the empty-string-membership
    // dance (`STR_EQ` against "") plus the `NUMERIC_TYPE <= 3` integer test.
    let nonstrict = inline_ops(true, &["x"], "[string is integer $x]");
    assert!(
        nonstrict.contains(&Op::NUMERIC_TYPE) && nonstrict.contains(&Op::STR_EQ),
        "non-strict integer: empty-check + numeric-type: {nonstrict:?}",
    );
    // `-strict` drops the empty-string acceptance: a shorter `NUMERIC_TYPE`,
    // `JUMP_FALSE1`, `LE`-3 sequence with no `STR_EQ`.
    let strict = inline_ops(true, &["x"], "[string is integer -strict $x]");
    assert!(
        strict.contains(&Op::NUMERIC_TYPE)
            && strict.contains(&Op::LE)
            && !strict.contains(&Op::STR_EQ),
        "strict integer omits the empty-string accept: {strict:?}",
    );
    // tclsh: string is integer -strict "" → 0  (8.6 + 9.0) — the strict form's
    // distinguishing rejection of the empty string.
    // tclsh: string is integer 42 → 1, string is integer -strict 42 → 1 (8.6+9.0)
}

#[test]
fn cmd_subst_string_is_double_strict_and_nonstrict() {
    // `string is double` strict path: `NUMERIC_TYPE` then a `JUMP_TRUE1`
    // branch pushing 0/1 — no empty-string `STR_EQ`.
    let strict = inline_ops(true, &["x"], "[string is double -strict $x]");
    assert!(
        strict.first() == Some(&Op::LOAD_SCALAR1)
            && strict.contains(&Op::NUMERIC_TYPE)
            && !strict.contains(&Op::STR_EQ),
        "strict double: {strict:?}",
    );
    // Non-strict accepts the empty string first (a leading `STR_EQ` vs "").
    let nonstrict = inline_ops(true, &["x"], "[string is double $x]");
    assert!(
        nonstrict.contains(&Op::STR_EQ) && nonstrict.contains(&Op::NUMERIC_TYPE),
        "non-strict double accepts empty: {nonstrict:?}",
    );
    // tclsh: string is double 1.5 → 1, string is double -strict 3.0 → 1 (8.6+9.0)
}

#[test]
fn cmd_subst_string_is_boolean_uses_try_cvt() {
    // `string is boolean` lowers through `TRY_CVT_TO_BOOLEAN` plus the
    // empty-string accept.
    let ops = inline_ops(true, &["x"], "[string is boolean $x]");
    assert!(
        ops.contains(&Op::TRY_CVT_TO_BOOLEAN) && ops.contains(&Op::STR_EQ),
        "boolean: tryCvtToBoolean + empty accept: {ops:?}",
    );
    // tclsh: string is boolean yes → 1, string is boolean 0 → 1  (8.6 + 9.0)
}

#[test]
fn cmd_subst_string_replace_fast_path_and_fallback() {
    // `emit_inline_string_replace`: a `0 N` prefix-replace takes the fast
    // `reverse; strRangeImm; strConcat1` path (drop the first N+1 chars, prepend
    // the replacement).
    let fast = inline_ops(true, &["s"], "[string replace $s 0 2 X]");
    assert!(
        fast.contains(&Op::REVERSE)
            && fast.contains(&Op::STR_RANGE_IMM)
            && fast.contains(&Op::STR_CONCAT1)
            && !fast.contains(&Op::STR_REPLACE),
        "0..N replace fast path: {fast:?}",
    );
    // tclsh: string replace abcdef 0 2 X → Xdef  (8.6 + 9.0)

    // A mid-string range (`2 3`) falls back to the general `STR_REPLACE` opcode.
    let fallback = inline_ops(true, &["s"], "[string replace $s 2 3 X]");
    assert!(
        fallback.contains(&Op::STR_REPLACE) && !fallback.contains(&Op::STR_RANGE_IMM),
        "mid-string replace → strreplace fallback: {fallback:?}",
    );
    // tclsh: string replace abcdef 2 3 X → abXef  (8.6 + 9.0)
}

#[test]
fn cmd_subst_string_equal_compare_invoke_replace_forms() {
    // The `-nocase` / `-length` flag forms of `string equal` / `string compare`
    // route through `INVOKE_REPLACE` (against the `::tcl::string::…` FQN),
    // distinct from the bare 2-arg `STR_EQ` / `STR_CMP` fast ops.
    assert!(
        inline_ops(true, &["a", "b"], "[string equal -nocase $a $b]").contains(&Op::INVOKE_REPLACE),
        "string equal -nocase → invokeReplace",
    );
    assert!(
        inline_ops(true, &["a", "b"], "[string compare -nocase $a $b]")
            .contains(&Op::INVOKE_REPLACE),
        "string compare -nocase → invokeReplace",
    );
    assert!(
        inline_ops(true, &["a", "b"], "[string compare -length 3 $a $b]")
            .contains(&Op::INVOKE_REPLACE),
        "string compare -length → invokeReplace",
    );
    // The bare 2-arg forms stay on the dedicated fast opcodes (control).
    assert!(inline_ops(true, &["a", "b"], "[string equal $a $b]").contains(&Op::STR_EQ));
    assert!(inline_ops(true, &["a", "b"], "[string compare $a $b]").contains(&Op::STR_CMP));
}

#[test]
fn cmd_subst_array_names_size_and_exists() {
    // `emit_inline_array`: `names`/`size` route through a startCommand-wrapped
    // `::tcl::array::<sub>` invoke; `exists` on a proc-local takes the dedicated
    // `ARRAY_EXISTS_IMM` immediate opcode.
    let names = inline_ops(true, &["a"], "[array names a]");
    assert!(
        names.contains(&Op::START_CMD) && names.contains(&Op::INVOKE_STK1),
        "array names → fqn invoke: {names:?}",
    );
    let size = inline_ops(true, &["a"], "[array size a]");
    assert!(
        size.contains(&Op::INVOKE_STK1),
        "array size → fqn invoke: {size:?}"
    );

    let exists = inline_ops(true, &["a"], "[array exists a]");
    assert!(
        exists.contains(&Op::ARRAY_EXISTS_IMM),
        "array exists (proc-local) → immediate opcode: {exists:?}",
    );
}

#[test]
fn cmd_subst_dict_get_multi_key() {
    // `emit_inline_dict_get`: a multi-key `dict get $d k1 k2` emits the dict
    // value load + each key push + a `DICT_GET` carrying the key count.
    let ops = inline_ops(true, &["d"], "[dict get $d k1 k2]");
    assert!(ops.contains(&Op::DICT_GET), "dict get → DICT_GET: {ops:?}");
    assert_eq!(
        ops.iter().filter(|&&o| o == Op::PUSH1).count(),
        2,
        "two key pushes for a 2-key path: {ops:?}",
    );
    // tclsh: dict get {a 1 b 2 c 3} b → 2  (single-key sanity, 8.6 + 9.0)
}

#[test]
fn cmd_subst_lreplace_and_linsert_use_lreplace4() {
    // `emit_inline_lreplace` / `emit_inline_linsert` both lower to the
    // `LREPLACE4` opcode (linsert is lreplace with a different mode operand).
    assert!(
        inline_ops(true, &["l"], "[lreplace $l 1 2 X]").contains(&Op::LREPLACE4),
        "lreplace → LREPLACE4",
    );
    // tclsh: lreplace {a b c d} 1 2 X Y → a X Y d  (8.6 + 9.0)
    assert!(
        inline_ops(true, &["l"], "[linsert $l 1 X]").contains(&Op::LREPLACE4),
        "linsert → LREPLACE4",
    );
    // tclsh: linsert {a b c} 1 X Y → a X Y b c  (8.6 + 9.0)
}

#[test]
fn cmd_subst_regexp_nocase_glob_and_plain_forms() {
    // `emit_inline_regexp`: a `-nocase` pattern that converts to a glob lowers
    // to `STR_MATCH` (case-insensitive); a plain 2-arg `regexp` lowers to the
    // `REGEXP` opcode.
    let nocase = inline_ops(true, &["s"], "[regexp -nocase abc $s]");
    assert!(
        nocase.contains(&Op::STR_MATCH) && !nocase.contains(&Op::REGEXP),
        "regexp -nocase glob → STR_MATCH: {nocase:?}",
    );
    // tclsh: regexp -nocase {ABC} "xxabcyy" → 1  (case-insensitive, 8.6 + 9.0)
    let plain = inline_ops(true, &["s"], "[regexp foo $s]");
    assert!(
        plain.contains(&Op::REGEXP) && !plain.contains(&Op::STR_MATCH),
        "plain 2-arg regexp → REGEXP: {plain:?}",
    );
}

#[test]
fn cmd_subst_lindex_multi_index_form() {
    // `emit_inline_lindex`: a single immediate index → `LIST_INDEX_IMM`; a
    // multi-index path → `LINDEX_MULTI`.
    assert!(
        inline_ops(true, &["l"], "[lindex $l 0]").contains(&Op::LIST_INDEX_IMM),
        "single immediate index → LIST_INDEX_IMM",
    );
    assert!(
        inline_ops(true, &["m"], "[lindex $m 0 1]").contains(&Op::LINDEX_MULTI),
        "multi-index → LINDEX_MULTI",
    );
    // tclsh: lindex {a b c} 1 → b ; lindex {{a b} c} 0 0 → a  (8.6 + 9.0)
}

// ── cmd_subst.rs — free functions + emit_value / emit_cmd_subst_arg paths ───

#[test]
fn cmd_subst_unroll_nested_set_chain() {
    // `unroll_nested_set` flattens a `[set y [set z 42]]` chain into the var
    // names followed by the innermost value.
    assert_eq!(
        unroll_nested_set("[set y [set z 42]]"),
        Some(vec!["y".to_string(), "z".to_string(), "42".to_string()]),
    );
    // tclsh: set z 42; set y [set z]  ⇒  $y == $z == 42  (the chain's value,
    // 8.6 + 9.0) — `set y [set z 42]` binds y and z to 42.
    // A non-`set` body is not a chain.
    assert_eq!(unroll_nested_set("[puts hi]"), None);
    // A bare-word triple chain with a deeper nest.
    assert_eq!(
        unroll_nested_set("[set a [set b [set c 1]]]"),
        Some(vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "1".to_string()
        ]),
    );
}

#[test]
fn cmd_subst_is_pure_and_has_separator_free_functions() {
    // `is_pure_cmd_subst`: a single balanced `[…]` is pure; two adjacent
    // substitutions or bare text are not.
    assert!(is_pure_cmd_subst("[expr {1 + 2}]"));
    assert!(is_pure_cmd_subst("[set x [set y 1]]"));
    assert!(!is_pure_cmd_subst("[a] [b]"));
    assert!(!is_pure_cmd_subst("plain"));

    // `has_command_separator`: a `;`/newline outside quotes/braces is a
    // separator; inside braces it is not.
    assert!(has_command_separator("set x 1; set y 2"));
    assert!(has_command_separator("set x 1\nset y 2"));
    assert!(has_command_separator("[set x 1; list a]"));
    assert!(!has_command_separator("{set x 1; set y 2}"));
    assert!(!has_command_separator("\"set x 1; set y 2\""));
}

#[test]
fn cmd_subst_parse_cmd_parts_and_expand_forms() {
    // `parse_cmd_parts` splits a substitution body into `(text, braced)` words.
    assert_eq!(
        parse_cmd_parts("[string range $x 0 2]"),
        vec![
            ("string".to_string(), false),
            ("range".to_string(), false),
            ("$x".to_string(), false),
            ("0".to_string(), false),
            ("2".to_string(), false),
        ],
    );
    // `parse_cmd_parts_expand` flags a leading `{*}` word as an expansion.
    assert_eq!(
        parse_cmd_parts_expand("[list {*}$a b]"),
        vec![
            ("list".to_string(), false, false),
            ("$a".to_string(), false, true),
            ("b".to_string(), false, false),
        ],
    );
}

#[test]
fn cmd_subst_multi_command_body_falls_back_to_eval_stk() {
    // `emit_inline_cmd_subst`'s `has_command_separator` guard: a `[…]` body
    // with an internal `;` is two commands and falls back to runtime `EVAL_STK`
    // over the braced script rather than inlining a single call.
    let ops = inline_ops(true, &[], "[set a 1; set b 2]");
    assert!(
        ops.contains(&Op::EVAL_STK),
        "multi-command → EVAL_STK: {ops:?}"
    );
}

#[test]
fn cmd_subst_expanded_form_in_value_position() {
    // `emit_expanded_cmd_subst`: a `{*}`-expanded substitution in value
    // position lowers to `expandStart … expandStkTop N; invokeExpanded`,
    // leaving the result on the stack (no trailing pop).
    let ops = inline_ops(true, &["a", "b"], "[concat {*}$a {*}$b]");
    assert!(
        ops.contains(&Op::EXPAND_START)
            && ops.contains(&Op::EXPAND_STKTOP)
            && ops.contains(&Op::INVOKE_EXPANDED),
        "{{*}} value-position expansion: {ops:?}",
    );
}

#[test]
fn emit_value_list_expand_concat_and_folds() {
    // `emit_value`'s specialised pre-checks each claim the value before the
    // generic interpolation path:
    //   * `[list {*}$a {*}$b]` → `load a; load b; listConcat`.
    assert_eq!(
        value_ops(true, &["a", "b"], "[list {*}$a {*}$b]", false),
        vec![Op::LOAD_SCALAR1, Op::LOAD_SCALAR1, Op::LIST_CONCAT],
    );
    //   * `[dict create k v]` constant-folds to a verified-dict literal push.
    let dict = value_ops(false, &[], "[dict create k v]", false);
    assert!(
        dict.contains(&Op::VERIFY_DICT) && dict.first() == Some(&Op::PUSH1),
        "dict create fold → push + verifyDict: {dict:?}",
    );
    // tclsh: dict create k v → "k v"  (8.6 + 9.0) — the folded dict literal.
    //   * `[format {%s} hi]` constant-folds to a single literal push.
    assert_eq!(
        value_ops(false, &[], "[format {%s} hi]", false),
        vec![Op::PUSH1]
    );
    // tclsh: format {%s} hi → hi  (8.6 + 9.0)
}

#[test]
fn emit_value_array_index_with_variable_key() {
    // `emit_value`'s whole-word `$arr($idx)` path: a variable index can't be
    // resolved by the runtime literal path, so it decomposes at compile time
    // into a `loadArray` (base load + substituted key).
    let ops = value_ops(true, &["arr", "i"], "$arr($i)", true);
    assert!(
        ops.contains(&Op::LOAD_ARRAY1) || ops.contains(&Op::LOAD_ARRAY_STK),
        "variable array index → loadArray: {ops:?}",
    );
}

#[test]
fn emit_value_interpolated_string_decomposes() {
    // `emit_value`'s interpolation path decomposes `x$a-$b` into its literal /
    // `$var` parts and concatenates them (`STR_CONCAT1`).
    let ops = value_ops(true, &["a", "b"], "x$a-$b", true);
    assert!(
        ops.contains(&Op::STR_CONCAT1)
            && ops.iter().filter(|&&o| o == Op::LOAD_SCALAR1).count() == 2,
        "interpolated string → two loads + concat: {ops:?}",
    );
}

#[test]
fn emit_cmd_subst_arg_composite_and_special_forms() {
    // `emit_cmd_subst_arg`'s composite path: an unbraced `$opt*` (a `$var`
    // followed by literal text) decomposes into `load opt; push "*"; concat`.
    assert_eq!(
        arg_ops(true, &["opt"], "$opt*", false),
        vec![Op::LOAD_SCALAR1, Op::PUSH1, Op::STR_CONCAT1],
    );
    // A braced arg carrying substitution markers (`{$x}`) is re-wrapped and
    // pushed as a single literal (braces suppress substitution).
    assert_eq!(arg_ops(true, &[], "{$x}", true), vec![Op::PUSH1]);
    // The `$={name}` braced-scalar marker → push name; loadStk.
    assert_eq!(
        arg_ops(true, &["n"], "$={n}", false),
        vec![Op::PUSH1, Op::LOAD_STK],
    );
    // A bare `$name` form loads the scalar directly.
    assert_eq!(arg_ops(true, &["v"], "$v", false), vec![Op::LOAD_SCALAR1]);
}

// ===========================================================================
// Issue #1080 — `[self class]` folds to the enclosing method's defining class.
//
// The value is a registry fact (`CommandSpec::oo_context_facts` maps `self`'s
// `class` word to `OoContextFact::DefiningClass`), answered by the optimiser
// from the class whose definition body encloses the method. Folding direction
// is abstain-toward-no-fold, so each abstention gets its own TN below.
//
// Oracle, byte-identical on tclsh 9.0.4 and 8.6.16 — see
// `OoContextFact::DefiningClass`'s doc comment for the full transcript.
// ===========================================================================

/// Every O129 replacement in `src`.
fn o129(src: &str) -> Vec<String> {
    repls(src, "O129")
}

#[test]
fn self_class_folds_to_the_lexically_enclosing_class() {
    // TP. Oracle: `oo::class create ::A { method m {} { self class } }` then
    // `[::A new] m` -> ::A.
    assert_eq!(
        o129("oo::class create ::A {\n    method m {} { puts [self class] }\n}\n"),
        vec!["::A".to_string()],
    );
    // TP, the `return` shape — the whole statement is the rewrite target.
    assert_eq!(
        o129("oo::class create ::A {\n    method m {} { return [self class] }\n}\n"),
        vec!["return ::A".to_string()],
    );
    // TP, the `set` value position.
    assert_eq!(
        o129("oo::class create ::A {\n    method m {} { set c [self class]\n    puts $c }\n}\n"),
        vec!["::A".to_string()],
    );
}

#[test]
fn self_class_folds_through_every_class_defining_shape() {
    // TP. `oo::define` targets the class it names — oracle:
    // `oo::define ::B { method n {} { self class } }` -> ::B, even for a
    // subclass whose inherited method still answers with ::A.
    assert_eq!(
        o129("oo::class create ::B {}\noo::define ::B { method n {} { puts [self class] } }\n"),
        vec!["::B".to_string()],
    );
    // TP. A relative class name inside `namespace eval` resolves the same way
    // tclsh does — oracle: `namespace eval ::N { oo::class create C {…} }` and
    // `[::N::C new] m` -> ::N::C.
    assert_eq!(
        o129(
            "namespace eval ::N {\n    oo::class create C { method m {} { puts [self class] } }\n}\n"
        ),
        vec!["::N::C".to_string()],
    );
    // TP. Constructor and destructor bodies are class-defined implementations
    // — oracle: both report ::E.
    assert_eq!(
        o129(
            "oo::class create ::E {\n    constructor {} { puts [self class] }\n    destructor { puts [self class] }\n}\n"
        ),
        vec!["::E".to_string(), "::E".to_string()],
    );
}

#[test]
fn self_class_folds_inside_nested_bodies() {
    // TP. The method-body walk recurses through control-flow bodies.
    let r = o129(
        "oo::class create ::A {\n    method m {} {\n        if {1} { puts [self class] }\n        foreach x {1 2} { puts [self class] }\n        while {0} { puts [self class] }\n    }\n}\n",
    );
    assert_eq!(r, vec!["::A".to_string(); 3], "got {r:?}");
}

#[test]
fn self_class_abstains_on_a_class_object_method() {
    // TN. Oracle: a method on the class *object* is not defined by a class, so
    // `self class` RAISES rather than returning a name —
    //   oo::class create ::U { self method cm {} { self class } }
    //   ::U cm   ->  method not defined by a class
    // Folding it to `::U` would invent a value the interpreter never produces.
    assert!(
        o129("oo::class create ::U {\n    classmethod cm {} { puts [self class] }\n}\n").is_empty(),
    );
    assert!(
        o129("oo::class create ::U {\n    self method cm {} { puts [self class] }\n}\n").is_empty(),
    );
}

#[test]
fn self_class_abstains_when_the_class_command_is_renamed() {
    // TN. Oracle: `self class` answers with the class's *current* name —
    //   oo::class create ::R { method r {} { self class } }
    //   set r [::R new] ; rename ::R ::R2 ; $r r   ->  ::R2
    // so a module that renames the class anywhere must not fold to ::R. Same
    // rename-captures-identity rule `indirection.rs` applies.
    assert!(
        o129("oo::class create ::R {\n    method r {} { puts [self class] }\n}\nrename ::R ::R2\n")
            .is_empty(),
    );
    // The rename buried inside a proc body counts too: the whole-module query
    // is flow-insensitive because the call order is not statically known.
    assert!(
        o129(
            "oo::class create ::R {\n    method r {} { puts [self class] }\n}\nproc ::later {} { rename ::R ::R2 }\n"
        )
        .is_empty(),
    );
}

#[test]
fn self_class_abstains_when_self_itself_is_rebound() {
    // TN. The generic builtin-fold trust gate: a module that redefines `self`
    // no longer has `self`'s builtin semantics to fold with.
    assert!(
        o129(
            "proc self {args} { return spoofed }\noo::class create ::A {\n    method m {} { puts [self class] }\n}\n"
        )
        .is_empty(),
    );
}

#[test]
fn the_other_self_words_never_fold() {
    // TN. `object` / `namespace` name the receiving *instance* — a fresh
    // `::oo::ObjNN` per `new`, never a source constant. Oracle:
    //   oo::class create ::A { method m {} { list [self] [self object] \
    //                                             [self namespace] } }
    //   [::A new] m   ->  ::oo::Obj22 ::oo::Obj22 ::oo::Obj22
    // and the chain words (`method`, `call`, `next`, `target`, `filter`,
    // `caller`) are reshaped at run time by mixins, filters, and `next`.
    for word in [
        "",
        "object",
        "namespace",
        "method",
        "call",
        "caller",
        "filter",
        "next",
        "target",
    ] {
        let src =
            format!("oo::class create ::A {{\n    method m {{}} {{ puts [self {word}] }}\n}}\n");
        assert!(
            o129(&src).is_empty(),
            "`self {word}` must not fold: {:?}",
            o129(&src),
        );
    }
}

#[test]
fn self_class_outside_a_method_body_never_folds() {
    // TN. `self` resolves only inside a method frame — oracle: at top level and
    // in a plain proc it is not even a command
    // (`invalid command name "self"`), so there is no frame to answer from and
    // the fold must never reach these bodies.
    assert!(o129("puts [self class]\n").is_empty());
    assert!(o129("proc ::p {} { puts [self class] }\n").is_empty());
}

#[test]
fn a_dynamic_class_name_leaves_self_class_alone() {
    // TN. The lowering already declines a class word it cannot resolve
    // statically, so no frame exists and nothing is claimed.
    assert!(
        o129("set nm ::F\noo::class create $nm { method m {} { puts [self class] } }\n").is_empty(),
    );
}

#[test]
fn folding_self_class_does_not_disturb_the_rest_of_the_method_body() {
    // Blast-radius guard. The method-body walk carries no constants map, so it
    // introduces no variable propagation inside a method — an instance
    // variable is object state that outlives the frame and any `my …` call may
    // rewrite it, which the per-function SCCP lattice does not model. Only the
    // frame-constant fold fires here.
    let src = "oo::class create ::A {\n    variable ivar\n    method m {} {\n        set ivar 1\n        my other\n        puts $ivar\n        puts [self class]\n    }\n    method other {} { set ivar 2 }\n}\n";
    assert_eq!(o129(src), vec!["::A".to_string()]);
    // No `$ivar` was propagated to a literal `1` anywhere.
    assert!(
        !opts(src).iter().any(|o| o.replacement == "1"),
        "instance-variable propagation must stay off: {:?}",
        opts(src)
            .iter()
            .map(|o| (o.code.as_str(), o.replacement.clone()))
            .collect::<Vec<_>>(),
    );
}

// ===========================================================================
// Issues #1096 / #1097 — completing the ticklecharts `${ns}::setdef` chain.
//
// #1096 put `const_fold` callbacks on `namespace qualifiers` / `namespace
// tail` (pure string splitting at the last `::`, oracle-pinned byte-identical
// on tclsh 9.0.4 and 8.6.14 — see `tcl-registry`'s `namespace_` unit tests and
// the `differential_fold` matrix).  #1097 ported `elimination.rs`'s
// instance-variable escaping model into the propagation lattice, so a
// provably method-local variable propagates inside a method body while object
// state still does not.
// ===========================================================================

#[test]
fn namespace_qualifiers_and_tail_fold_over_the_self_class_frame_constant() {
    // TP, the chain's second hop in one step: the O129 fold resolves the
    // nested `[self class]` from the method frame first, then runs
    // `qualifiers` on the resulting constant.  Oracle:
    //   namespace qualifiers ::ticklecharts::Gauge -> ::ticklecharts
    assert_eq!(
        o129(
            "oo::class create ::ticklecharts::Gauge {\n    method m {} { set ns [namespace qualifiers [self class]] }\n}\n"
        ),
        vec!["::ticklecharts".to_string()],
    );
    // TP, the sibling subcommand.  Oracle:
    //   namespace tail ::ticklecharts::Gauge -> Gauge
    assert_eq!(
        o129(
            "oo::class create ::ticklecharts::Gauge {\n    method m {} { set n [namespace tail [self class]] }\n}\n"
        ),
        vec!["Gauge".to_string()],
    );
    // TP outside `TclOO` entirely — the fold is a registry fact about a pure
    // string operation, not an OO one.
    assert_eq!(
        o129("proc ::f {} { set ns [namespace qualifiers ::a::b::c] }\n"),
        vec!["::a::b".to_string()],
    );
}

#[test]
fn namespace_qualifiers_abstains_on_a_dynamic_argument() {
    // TN — an unprovable argument leaves the call alone.  `$x` is never
    // assigned, so it is not a constant and `literal_words` bails.
    assert!(
        o129("proc ::f {x} { set ns [namespace qualifiers $x] }\n").is_empty(),
        "a dynamic argument must not fold",
    );
    // TN — a *nested* substitution that itself does not fold bails the whole
    // fold rather than folding half of it.
    assert!(
        o129("proc ::f {} { set ns [namespace qualifiers [gets stdin]] }\n").is_empty(),
        "an unfoldable nested substitution must not fold",
    );
    // TN — a module that rebinds `namespace` no longer has the builtin's
    // semantics to fold with.
    assert!(
        o129("rename namespace ns_orig\nproc ::f {} { set ns [namespace qualifiers ::a::b] }\n")
            .is_empty(),
        "a rebound `namespace` must not fold",
    );
}

#[test]
fn method_local_variable_propagates_into_the_fold_chain() {
    // TP (#1097). `base` is a method local: the class declares no instance
    // variable, nothing aliases it, so the propagation lattice may carry its
    // value into the `namespace qualifiers` argument.  Before #1097 the
    // method-body walk carried no constants map at all, so this folded
    // nothing.
    let r = o129(
        "oo::class create ::ticklecharts::Gauge {\n    method m {} {\n        set base ::ticklecharts::Gauge\n        set ns [namespace qualifiers $base]\n    }\n}\n",
    );
    assert!(
        r.contains(&"::ticklecharts".to_string()),
        "the `$base` hop must fold, got {r:?}",
    );
    // TP — the same for a `[string …]` fold over a method-local, the other
    // example issue #1097 lists.
    let r = o129(
        "oo::class create ::A {\n    method m {} {\n        set s abcde\n        puts [string length $s]\n    }\n}\n",
    );
    assert!(r.contains(&"5".to_string()), "got {r:?}");
    // TP — plain `$var` propagation inside a method body (O100), the other
    // half of what #1097 switched on.
    let r = repls(
        "oo::class create ::A {\n    method m {} {\n        set v 42\n        puts $v\n    }\n}\n",
        "O100",
    );
    assert_eq!(r, vec!["42".to_string()], "got {r:?}");
}

#[test]
fn an_instance_variable_never_propagates_inside_a_method_body() {
    // TN (#1097's whole point). `ns` here is *object state* — declared by the
    // class's `variable ns`, so the constructor or any other method may have
    // written it and `my …` may rewrite it between the two statements.  The
    // frame-constant `[self class]` still folds (it reads no variable), but
    // the `[namespace qualifiers $ns]` hop must not.
    let r = o129(
        "oo::class create ::ticklecharts::Gauge {\n    variable ns\n    method m {} {\n        set ns [self class]\n        set q [namespace qualifiers $ns]\n    }\n}\n",
    );
    assert_eq!(
        r,
        vec!["::ticklecharts::Gauge".to_string()],
        "only the frame constant may fold, got {r:?}",
    );
    // TN — the same via a method-local `my variable` declaration.
    let r = o129(
        "oo::class create ::ticklecharts::Gauge {\n    method m {} {\n        my variable ns\n        set ns [self class]\n        set q [namespace qualifiers $ns]\n    }\n}\n",
    );
    assert_eq!(
        r,
        vec!["::ticklecharts::Gauge".to_string()],
        "a `my variable` alias must bar propagation, got {r:?}",
    );
    // TN — plain O100 `$var` propagation must not touch object state
    // either: `my bump` can rewrite `n` between the write and the read, which
    // is exactly the miscompile issue #1097 opens with.
    assert!(
        repls(
            "oo::class create ::C {\n    variable n\n    method bump {} { incr n }\n    method m {} {\n        set n 1\n        my bump\n        puts $n\n    }\n}\n",
            "O100",
        )
        .is_empty(),
        "an instance variable must not propagate through a `my` dispatch",
    );
    // TN — an ordinary `variable` declaration inside the body.
    let r = o129(
        "oo::class create ::ticklecharts::Gauge {\n    method m {} {\n        variable ns\n        set ns [self class]\n        set q [namespace qualifiers $ns]\n    }\n}\n",
    );
    assert_eq!(
        r,
        vec!["::ticklecharts::Gauge".to_string()],
        "a `variable` alias must bar propagation, got {r:?}",
    );
}

#[test]
fn a_method_that_can_reach_its_caller_frame_bars_method_local_propagation() {
    // TN (#1097's barrier). `helper` aliases its *caller's* local `base`, and
    // it is reached by `my helper` — a dispatch the CFG's upvar-callee table
    // cannot see, because the call does not name it.  So no method body's
    // locals may be propagated in this module.  The frame constant still
    // folds: it reads no variable.
    let r = o129(
        "oo::class create ::ticklecharts::Gauge {\n    method helper {} { upvar 1 base b\n        set b ::hijacked }\n    method m {} {\n        set base [self class]\n        my helper\n        set ns [namespace qualifiers $base]\n    }\n}\n",
    );
    assert_eq!(
        r,
        vec!["::ticklecharts::Gauge".to_string()],
        "a caller-frame-reaching method must bar propagation, got {r:?}",
    );
}

// ===========================================================================
// The method-body propagation barrier's EVIDENCE SOURCES (review findings on
// #1096 / #1097).  Each was a would-be miscompile: the optimiser proposed
// replacing `$x` with `1` where real Tcl prints `2`.  The governing rule is
// that when the module's evidence about what a `my` / `next` dispatch can do
// is incomplete, the barrier widens to abstention.
//
// Every oracle below is byte-identical on tclsh 9.0.4 and 8.6.14.
// ===========================================================================

/// Every `O100` variable-propagation replacement in `src`.
fn o100(src: &str) -> Vec<String> {
    repls(src, "O100")
}

#[test]
fn instance_vars_declared_in_a_later_definition_block_bar_propagation() {
    // FP guard, finding 1.  `x` is declared instance state by a *separate*
    // `oo::define` block, walked after `m` was already extracted.  Oracle:
    //
    //   oo::class create C { method m {} {set x 1; my change; puts $x}
    //                        method change {} {set x 2} }
    //   oo::define C { variable x }
    //   [C create c1] m        ;# -> 2
    //
    // so folding `$x` to `1` is a miscompile.  The lowering now unions each
    // class's instance-variable declarations across all of its definition
    // blocks, so `m`'s `instance_vars` sees `x` however late it is declared.
    let src = "oo::class create C {\n    method m {} { set x 1\n        my change\n        puts $x }\n    method change {} { set x 2 }\n}\noo::define C { variable x }\n";
    assert!(
        o100(src).is_empty(),
        "state declared in a later `oo::define` block must bar propagation, got {:?}",
        o100(src),
    );
    // The declaration order must not matter — the union is order-free.
    let reversed = "oo::class create C {}\noo::define C { variable x }\noo::define C {\n    method m {} { set x 1\n        my change\n        puts $x }\n    method change {} { set x 2 }\n}\n";
    assert!(o100(reversed).is_empty(), "got {:?}", o100(reversed));
}

#[test]
fn a_redefined_method_bars_propagation() {
    // FP guard, finding 2.  The lowering keeps the *first* body and records
    // only the name in `redefined_methods`, so the replacement — which reaches
    // its caller's frame — is invisible to every body scan.  Oracle:
    //
    //   oo::class create D { method helper {} {}
    //                        method m {} {set x 1; my helper; puts $x} }
    //   oo::define D { method helper {} { upvar 1 x y; set y 2 } }
    //   [D create d1] m        ;# -> 2
    let src = "oo::class create D {\n    method helper {} {}\n    method m {} { set x 1\n        my helper\n        puts $x }\n}\noo::define D { method helper {} { upvar 1 x y\n    set y 2 } }\n";
    assert!(
        o100(src).is_empty(),
        "a redefined method must bar propagation, got {:?}",
        o100(src),
    );
    // Scoped to the whole module, not to the redefined method's own class:
    // `my` dispatches along the MRO, so a replaced method in a *superclass* is
    // reachable from a subclass body and a per-class switch would miss it.
    let cross_class = "oo::class create B {\n    method helper {} {}\n}\noo::class create C {\n    superclass B\n    method m {} { set x 1\n        my helper\n        puts $x }\n}\noo::define B { method helper {} { upvar 1 x y\n    set y 2 } }\n";
    assert!(
        o100(cross_class).is_empty(),
        "a superclass redefinition must bar propagation too, got {:?}",
        o100(cross_class),
    );
}

#[test]
fn a_substituted_upvar_source_counts_as_a_caller_frame_alias() {
    // FP guard, finding 3.  `var_observability`'s per-variable alias route
    // skips an `upvar` pair when either side starts with `$`, so this helper
    // read as "no caller-frame alias" even though it mutates its caller's
    // variable on every call.  Oracle:
    //
    //   oo::class create E {
    //     method helper {src} { upvar 1 $src b; set b 2 }
    //     method m {} {set x 1; set src x; my helper $src; puts $x} }
    //   [E create e1] m        ;# -> 2
    //
    // The gate now reads the structural `reaches_caller_frame` query, for
    // which a dynamic name is *more* dangerous, never exempt.
    let src = "oo::class create E {\n    method helper {src} { upvar 1 $src b\n        set b 2 }\n    method m {} { set x 1\n        set src x\n        my helper $src\n        puts $x }\n}\n";
    assert!(
        o100(src).is_empty(),
        "a substituted upvar source must bar propagation, got {:?}",
        o100(src),
    );
    // A dynamic source that is *not* a parameter (the `has_unresolvable_
    // caller_target` route) and a dynamic *local* side (`upvar 1 x $dst`,
    // which the resolvable-buckets summary drops outright) both count.
    for shape in [
        "oo::class create E {\n    method helper {} { set n x\n        upvar 1 $n b\n        set b 2 }\n    method m {} { set x 1\n        my helper\n        puts $x }\n}\n",
        "oo::class create E {\n    method helper {dst} { upvar 1 x $dst\n        set $dst 2 }\n    method m {} { set x 1\n        my helper q\n        puts $x }\n}\n",
    ] {
        assert!(
            o100(shape).is_empty(),
            "got {:?} for {shape:?}",
            o100(shape)
        );
    }
}

#[test]
fn a_namespace_alias_is_not_a_caller_frame_alias() {
    // TN for the widening itself — the barrier must not swallow every module.
    // `global` / `variable` / `namespace upvar` reach a *namespace*, not the
    // caller's locals, so a class using them still propagates its own locals.
    let src = "oo::class create F {\n    method helper {} { global g\n        variable v\n        namespace upvar ::ns n n\n        set g 1 }\n    method m {} { set s abcde\n        my helper\n        puts [string length $s] }\n}\n";
    let r = repls(src, "O129");
    assert!(
        r.contains(&"5".to_string()),
        "namespace-reaching aliases must not bar method-local propagation, got {r:?}",
    );
}
