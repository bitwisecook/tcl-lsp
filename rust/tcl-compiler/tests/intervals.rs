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

//! Interval analysis suites for `tcl-compiler`:
//!   * Interval-driven dynamic bounds checks (W230 lindex / W231 lset / W232
//!     string index / W233 divide-by-zero).
//!   * The CFG-driven integer interval lattice (arithmetic, join/meet/widen,
//!     `compute_intervals`, guard narrowing).
//!
//! ## Two observation surfaces
//!
//! * **Diagnostic surface** (the bulk of the bounds checks). The W23x
//!   diagnostics come from
//!   `Analyser::new().analyse(src, dialect).diagnostics` — the same merged
//!   surface the in-crate FP catalogue (`analyser/diagnostics/fp/bnd.rs`) and the
//!   sibling `core_analyses.rs` use. W230/W231/W232/W233 all flow through
//!   the analyser pass, so `Analyser::analyse` alone surfaces them. `count(src,
//!   code)` counts diagnostics of one code.
//!
//! * **Lattice surface** (`compute_intervals` / guard narrowing). The
//!   lowering bundle (`lower_to_ir` → `build_cfg` →
//!   `build_ssa` → `_sccp` → `compute_intervals`) is
//!   reachable from a built `CompilationUnit`: `fu.cfg`, `fu.ssa`,
//!   `fu.sccp.values`, fed to the public `compute_intervals` /
//!   `build_guard_index` / `refine_interval`. An SSA value `(name, version)` is
//!   keyed by an interned `(Symbol, Version)`; resolve a name with
//!   `fu.ssa.var_symbol` (the tuple key `("j", 1)`).
//!
//! ## C-Tcl proof split
//!
//! * **Tcl-semantic** facts — what `expr {…}` / `lindex` / `lset` / `string
//!   index` actually *do* — are the load-bearing ground truth for every
//!   positive/negative pairing, and were verified against real tclsh 8.6 + 9.0
//!   via `scripts/dev/tclsh_check.sh` (the two agree on every fact used here).
//!   Each is cited with a `// tclsh:` comment. Tcl's integer `/` floors toward
//!   −∞ (`expr {-7/2}` == −4, `expr {-7%2}` == 1) and `expr {1/0}` / `expr {5%0}`
//!   both raise *divide by zero* — these are NOT C semantics and were checked,
//!   not assumed.
//!
//! * **Lattice-structural** facts — the *shape* of the interval domain
//!   (join/meet/widening behaviour, which SSA version a loop-rotation assigns,
//!   how guard narrowing intersects half-lines) — are compiler-internal and have
//!   no direct Tcl observation. They are flagged "structural" at each site. The
//!   raw arithmetic operators (`add`/`sub`/`mul`/`join`/`widen`/`intersect`/
//!   `negate`) and the guard helpers are **private** to `intervals.rs`, so
//!   they cannot be called from an
//!   integration test; their algebra is already covered by the in-crate
//!   `#[cfg(test)]` module (`add_sub_intervals`, `intersect_uses_none_as_identity`,
//!   `join_absorbs_bottom_and_widens_to_inf`, …). Where a lattice fact is
//!   *observable end-to-end* (a constant-fold value, a widened loop bound) it is
//!   covered here through `compute_intervals`.
//!
//! ## Divergences (sound but less aggressive — captured at the actual verdict)
//!
//! Two bounds-check cases would ideally *fire* a finding the analysis declines
//! to (a sound under-approximation — silence is never a miscompile):
//!   * `[lindex …]` nested in an `expr` *value*
//!     position (`set u [expr {[lindex $l $i] + 1}]`). The
//!     candidate is collected but the embedded command-sub's index var is not in
//!     the flat `AssignExpr`'s use-version map, so no finding. The *same* shape
//!     in a `return`/`while` condition DOES fire (below). Captured at the
//!     actual (silent) verdict.
//!   * Driving the interval fixpoint to non-convergence requires patching the
//!     `MAX_ITERS` constant, but it is a private `const` with no injection point,
//!     so the *unconverged* branch is unreachable from a test. The *convergent*
//!     counterpart (a well-behaved loop yields sound, non-bottom intervals) is
//!     covered instead.
//!
//! ## `{*}` expansion and list length
//!
//! `set l [list {*}{a b}]` expands at runtime to a 2-element list, so `lindex $l
//! 1` is in range (tclsh: `set l [list {*}{a b}]; llength $l` → 2). A per-arg
//! `starts_with("{*}")` guard cannot see this: the segmenter strips the `{*}`
//! expansion prefix, leaving the single arg `"a b"`, and a naive length of
//! **1** would fire a false-positive W230. `list_command_length` therefore
//! bails whenever the command text contains `{*}` (length unknown); the repro
//! below pins that (no W230).

use tcl_compiler::analyser::Analyser;
use tcl_compiler::cfg::Terminator;
use tcl_compiler::compilation_unit::{CompilationUnit, FunctionUnit};
use tcl_compiler::intervals::{
    Interval, build_guard_index, compute_intervals_with, constant, numbers_for_dialect,
    refine_interval,
};
use tcl_registry::model::ingress::static_context_for;

/// Default dialect for the shared helpers below (8.6 and 9.0 agree on every
/// Tcl fact they exercise). The numeral-grammar tests name their dialect
/// explicitly, because 8.6 and 9.0 *disagree* there — see
/// [`dialect_numerals`].
const D: &str = "tcl8.6";

/// The numeral grammar of [`D`], threaded into every lattice-surface call the
/// way the analyser threads the document's dialect.
fn numbers() -> tcl_dialect::NumberSyntax {
    numbers_for_dialect(Some(
        tcl_registry::model::ingress::resolve_environment(D).analyser_profile(),
    ))
}

// Diagnostic-surface helpers.

/// How many diagnostics of `code` the analyser emits for `src`. The analyser
/// pass owns W230/W231/W232/W233, so this single surface suffices (matches the
/// in-crate `bnd.rs` catalogue's behaviour).
fn count(src: &str, code: &str) -> usize {
    Analyser::new()
        .analyse(src, D)
        .diagnostics
        .iter()
        .filter(|d| d.code.to_string() == code)
        .count()
}

/// The messages of every `code` diagnostic for `src` (for `"$j" in message`).
fn messages(src: &str, code: &str) -> Vec<String> {
    Analyser::new()
        .analyse(src, D)
        .diagnostics
        .iter()
        .filter(|d| d.code.to_string() == code)
        .map(|d| d.message.clone())
        .collect()
}

// Lattice-surface helpers.

/// Build `src` and return its `::f` `FunctionUnit` (the lower→cfg→ssa→sccp
/// pipeline collapsed into the shared `CompilationUnit` build).
fn func(src: &str) -> (CompilationUnit, String) {
    // Return the owning CU alongside the proc key so the borrow lives.
    (
        CompilationUnit::build_for(src, static_context_for(D).commands(), false),
        "::f".to_owned(),
    )
}

/// `compute_intervals` over `::f`, plus a name→Symbol resolver bound to it.
/// Look up an interval as `iv[("name", version)]`.
fn intervals_of(fu: &FunctionUnit) -> impl Fn(&str, u32) -> Option<Interval> + '_ {
    let iv = compute_intervals_with(&fu.cfg, &fu.ssa, &fu.sccp.values, numbers());
    move |name: &str, version: u32| {
        let sym = fu.ssa.var_symbol(name)?;
        iv.get(&(sym, version)).copied()
    }
}

/// The loop-body block: the `true_target` of the `Branch` whose condition is a
/// real comparison (`$i < N`/`$i < [llength …]`), i.e. the loop's exit test.
/// (Rust lowers `for` to a rotated loop where the latch — not the synthetic
/// `for_header` with its constant `1` guard — carries the comparison.)
fn loop_body_block(fu: &FunctionUnit) -> tcl_compiler::cfg::BlockId {
    fu.cfg
        .blocks
        .values()
        .find_map(|b| match &b.terminator {
            Some(Terminator::Branch {
                true_target,
                condition: tcl_compiler::expr_ast::ExprNode::Binary { .. },
                ..
            }) => Some(*true_target),
            _ => None,
        })
        .expect("a comparison-guarded branch")
}

/// Predecessor count per block — `refine_interval` requires a guarded branch
/// target have a single entry edge before applying its constraint (issue 148).
fn pred_counts(fu: &FunctionUnit) -> std::collections::HashMap<tcl_compiler::cfg::BlockId, usize> {
    fu.cfg
        .predecessors()
        .into_iter()
        .map(|(bid, preds)| (bid, preds.len()))
        .collect()
}

// Dynamic lindex out of range (W230).
mod dynamic_lindex_out_of_range {
    use super::*;

    #[test]
    fn literal_list_loop_index_past_end_fires() {
        // j ∈ [3, 9] (init 3, bounded `< 10`), list length 3 → always past end.
        // tclsh: `lindex {a b c} 5` → "" (silent OOR) — the W230 smell.
        let src = "proc f {} { for {set j 3} {$j < 10} {incr j} { set x [lindex {a b c} $j] } }";
        assert_eq!(count(src, "W230"), 1);
        assert!(
            messages(src, "W230")[0].contains("$j"),
            "message must name the dynamic index $j: {:?}",
            messages(src, "W230")
        );
    }

    #[test]
    fn const_index_past_end_fires() {
        // SCCP proves j == 5; list length 3. tclsh: `lindex {a b c} 5` → "".
        let src = "proc f {} { set j 5\n set x [lindex {a b c} $j] }";
        assert_eq!(count(src, "W230"), 1);
    }

    #[test]
    fn tracked_list_var_length_fires() {
        // `[list a b]` tracks length 2; j == 5 is past the end.
        // tclsh: `list a b` → "a b" (2 elements); `lindex {a b} 5` → "".
        let src = "proc f {} { set l [list a b]\n set j 5\n set x [lindex $l $j] }";
        assert_eq!(count(src, "W230"), 1);
    }

    #[test]
    fn idiomatic_loop_is_silent() {
        // `$j < [llength $l]` — canonical in-range iteration; must not warn (the
        // symbolic RHS gives no constant upper bound to prove OOR).
        let src =
            "proc f {l} { for {set j 0} {$j < [llength $l]} {incr j} { set x [lindex $l $j] } }";
        assert_eq!(count(src, "W230"), 0);
    }

    #[test]
    fn in_range_const_index_silent() {
        // tclsh: `lindex {a b c} 1` → "b" (in range).
        let src = "proc f {} { set j 1\n set x [lindex {a b c} $j] }";
        assert_eq!(count(src, "W230"), 0);
    }

    #[test]
    fn unknown_param_index_silent() {
        // A param index has no bound (TOP) — not provably out of range.
        let src = "proc f {i} { set x [lindex {a b c} $i] }";
        assert_eq!(count(src, "W230"), 0);
    }

    #[test]
    fn break_exit_does_not_apply_loop_guard_silent() {
        // `n` is unknown; a `break` inside `while {$n < 5}` reaches the loop
        // exit *without* satisfying the `n >= 5` false-edge guard. The exit
        // merge (two predecessors: the header false edge and the break edge)
        // must NOT refine `n` to `[5, +inf)` — doing so would fire a false
        // W230 on `lindex {a b c} $n`, since on the break path `n` may be in
        // range (issue 148). The exit block has >1 predecessor, so the guard
        // is not edge-dominating and must not apply.
        let src = "proc f {} {\n\
            \x20   set n [bar]\n\
            \x20   while {$n < 5} { if {[foo]} { break } }\n\
            \x20   set x [lindex {a b c} $n]\n\
            }";
        assert_eq!(count(src, "W230"), 0);
    }

    #[test]
    fn conditional_length_phi_silent() {
        // The list length merges 2 and 4 (unproven) — must not warn even though
        // j == 5 exceeds both, because the version's length is unknown.
        let src = "proc f {c} {\n\
            \x20   if {$c} { set l {a b} } else { set l {a b c d} }\n\
            \x20   set j 5\n\
            \x20   set x [lindex $l $j]\n\
            }";
        assert_eq!(count(src, "W230"), 0);
    }

    #[test]
    fn no_double_fire_with_syntactic_literal_index() {
        // A literal index is handled by the syntactic check; the interval check
        // must not add a second W230 (exactly one). tclsh: `lindex {a b c} 5` → "".
        assert_eq!(count("lindex {a b c} 5", "W230"), 1);
    }
}

// Nested-position lindex (W230 in nested shapes).
mod nested_position_lindex {
    use super::*;

    #[test]
    fn puts_nested_fires() {
        let src = "proc f {} { set j 5\n puts [lindex {a b} $j] }";
        assert_eq!(count(src, "W230"), 1);
    }

    #[test]
    fn lappend_nested_fires() {
        let src = "proc f {} { set j 5\n set r {}\n lappend r [lindex {a b} $j] }";
        assert_eq!(count(src, "W230"), 1);
    }

    #[test]
    fn return_value_fires() {
        let src = "proc f {} { set j 5\n return [lindex {a b} $j] }";
        assert_eq!(count(src, "W230"), 1);
    }

    #[test]
    fn nested_idiomatic_silent() {
        let src =
            "proc f {l} { for {set j 0} {$j < [llength $l]} {incr j} { puts [lindex $l $j] } }";
        assert_eq!(count(src, "W230"), 0);
    }

    #[test]
    fn nested_in_range_silent() {
        // tclsh: `lindex {a b c} 1` → "b".
        let src = "proc f {} { set j 1\n puts [lindex {a b c} $j] }";
        assert_eq!(count(src, "W230"), 0);
    }

    #[test]
    fn expr_embedded_fires_diverges_silent() {
        // DIVERGENCE (sound, less aggressive): `[lindex $l $i]` embedded in an
        // `expr` *value* position (`set u [expr {[lindex $l $i] + 1}]`, $l length
        // 3, $i == 5) would ideally fire
        // W230. tclsh: `lindex {a b c} 5` → "" so the access IS out of range —
        // but the analysis does not reach the command-sub index var from the flat
        // `AssignExpr` use-map, so no finding. Silence here is sound (the only
        // consumer turns a *proven* fact into a warning; missing one is never a
        // miscompile). The same shape in a `return`/branch position DOES fire —
        // see `expr_embedded_in_return_fires`. Captured at the actual verdict.
        let src =
            "proc f {} { set l {a b c}\n set i 5\n set u [expr {[lindex $l $i] + 1}]\n return $u }";
        assert_eq!(
            count(src, "W230"),
            0,
            "Rust does not flag an expr-value-embedded lindex (sound under-approx)"
        );
    }

    #[test]
    fn expr_embedded_in_return_fires() {
        // The `return`-terminator path DOES reach the embedded `[lindex …]` (the
        // partner positive that Rust *does* fire). tclsh: `lindex {a b c} 5` → "".
        let src = "proc f {} { set i 5\n return [expr {[lindex {a b c} $i] + 1}] }";
        assert_eq!(count(src, "W230"), 1);
    }

    #[test]
    fn branch_condition_fires() {
        // `[lindex …]` as a command substitution in a `while` condition. tclsh:
        // `lindex {a b} 5` → "" (the loop body simply never runs), still a smell.
        let src = "proc f {} { set l {a b}\n set i 5\n while {[lindex $l $i]} { break } }";
        assert_eq!(count(src, "W230"), 1);
    }

    #[test]
    fn expr_embedded_in_range_silent() {
        // tclsh: `lindex {a b c} 1` → "b" (in range) — silent regardless of the
        // expr-position handling.
        let src =
            "proc f {} { set l {a b c}\n set i 1\n set u [expr {[lindex $l $i] + 1}]\n return $u }";
        assert_eq!(count(src, "W230"), 0);
    }
}

// Dynamic lset out of range (W231).
mod dynamic_lset_out_of_range {
    use super::*;

    #[test]
    fn loop_index_past_append_slot_fires() {
        // j ∈ [4, 8]; list length 3 → index > length (3) every iteration.
        // tclsh: `set l {a b c}; lset l 4 x` → 'index "4" out of range' (a
        // runtime error) — W231.
        let src = "proc f {v} { set l {a b c}\n for {set j 4} {$j < 9} {incr j} { lset l $j $v } }";
        assert_eq!(count(src, "W231"), 1);
        assert!(
            messages(src, "W231")[0].contains("$j"),
            "message must name $j: {:?}",
            messages(src, "W231")
        );
    }

    #[test]
    fn append_slot_is_silent() {
        // index == length is the legal append slot for lset; must not warn.
        // tclsh: `set l {a b c}; lset l 3 x; set l` → "a b c x" (legal append).
        let src = "proc f {v} { set l {a b c}\n set j 3\n lset l $j $v }";
        assert_eq!(count(src, "W231"), 0);
    }

    #[test]
    fn in_range_silent() {
        let src = "proc f {v} { set l {a b c}\n set j 1\n lset l $j $v }";
        assert_eq!(count(src, "W231"), 0);
    }
}

// Dynamic string index (W232).
mod dynamic_string_index {
    use super::*;

    #[test]
    fn past_end_fires() {
        // tclsh: `string index hello 10` → "" (out of range) — W232 smell.
        let src = "proc f {} { set s \"hello\"\n set i 10\n return [string index $s $i] }";
        assert_eq!(count(src, "W232"), 1);
    }

    #[test]
    fn index_equals_length_fires() {
        // idx == length is out of range for `string index` (returns "").
        // tclsh: `string index hello 5` → "".
        let src = "proc f {} { set s \"hello\"\n set i 5\n return [string index $s $i] }";
        assert_eq!(count(src, "W232"), 1);
    }

    #[test]
    fn in_range_silent() {
        // tclsh: `string index hello 2` → "l" (in range).
        let src = "proc f {} { set s \"hello\"\n set i 2\n return [string index $s $i] }";
        assert_eq!(count(src, "W232"), 0);
    }

    #[test]
    fn unknown_silent() {
        let src = "proc f {s i} { return [string index $s $i] }";
        assert_eq!(count(src, "W232"), 0);
    }

    #[test]
    fn quoted_escape_uses_resolved_char_length() {
        // `"a\nb"` is 3 Tcl chars (the `\n` escape resolves to one newline), so
        // index 3 is OOR and index 2 is valid.
        // tclsh: `string length "a\nb"` → 3; `string index "a\nb" 3` → "";
        //        `string index "a\nb" 2` → "b".
        let oob = "proc f {} { set s \"a\\nb\"\n set i 3\n return [string index $s $i] }";
        assert_eq!(count(oob, "W232"), 1);
        let ok = "proc f {} { set s \"a\\nb\"\n set i 2\n return [string index $s $i] }";
        assert_eq!(count(ok, "W232"), 0);
    }

    #[test]
    fn braced_escape_keeps_source_length() {
        // `{a\nb}` is a *braced* literal — no backslash processing — so it is 4
        // Tcl chars and index 3 is in range.
        // tclsh: `string length {a\nb}` → 4 (backslash is literal in braces).
        let src = "proc f {} { set s {a\\nb}\n set i 3\n return [string index $s $i] }";
        assert_eq!(count(src, "W232"), 0);
    }

    #[test]
    fn unicode_escape_char_length() {
        // `ÿ` (ÿ) is one char, so `"xÿy"` is 3 chars; index 3 is OOR.
        // tclsh: `string length "xÿy"` → 3; `string index "xÿy" 3` → "".
        let src = "proc f {} { set s \"x\\u00ffy\"\n set i 3\n return [string index $s $i] }";
        assert_eq!(count(src, "W232"), 1);
    }
}

// Divide by zero (W233).
//
// tclsh: `expr {1/0}` and `expr {5%0}` both raise *divide by zero* (8.6 + 9.0).
// `expr` is lazy: a short-circuited `&&`/`||` operand and a non-selected `?:`
// arm never execute, so a `1/0` there must NOT fire; but a lazy arm forced by a
// *constant* guard is guaranteed to run and MUST fire. Every guard outcome
// below is pinned to tclsh.
mod divide_by_zero {
    use super::*;

    #[test]
    fn literal_div_zero_fires() {
        // tclsh: `expr {1 / 0}` → divide by zero.
        assert_eq!(count("proc f {} { return [expr {1 / 0}] }", "W233"), 1);
    }

    #[test]
    fn literal_mod_zero_fires() {
        // tclsh: `expr {5 % 0}` → divide by zero (modulo by zero is the same trap).
        assert_eq!(count("proc f {} { return [expr {5 % 0}] }", "W233"), 1);
    }

    #[test]
    fn const_var_divisor_fires() {
        // SCCP proves d == 0. tclsh: `set d 0; expr {10 / $d}` → divide by zero.
        let src = "proc f {} { set d 0\n return [expr {10 / $d}] }";
        assert_eq!(count(src, "W233"), 1);
    }

    #[test]
    fn zero_excluding_guard_silent() {
        // `if {$d != 0}` makes the division unreachable when d is provably 0
        // (SCCP prunes the dead branch) — must not warn.
        let src = "proc f {} { set d 0\n if {$d != 0} { return [expr {1 / $d}] }\n return safe }";
        assert_eq!(count(src, "W233"), 0);
    }

    #[test]
    fn nonzero_divisor_silent() {
        let src = "proc f {} { set d 3\n return [expr {10 / $d}] }";
        assert_eq!(count(src, "W233"), 0);
    }

    #[test]
    fn unknown_divisor_silent() {
        // A param divisor has no bound — not provably zero.
        let src = "proc f {n} { return [expr {10 / $n}] }";
        assert_eq!(count(src, "W233"), 0);
    }

    #[test]
    fn subtraction_to_zero_fires() {
        // The interval domain proves $x - $x == 0 when x is constant.
        // tclsh: `set x 5; expr {10 / ($x - $x)}` → divide by zero.
        let src = "proc f {} { set x 5\n return [expr {10 / ($x - $x)}] }";
        assert_eq!(count(src, "W233"), 1);
    }

    // --- laziness: dead arms must stay silent ---

    #[test]
    fn dead_ternary_arm_div_zero_silent() {
        // tclsh: `expr {0 ? 1/0 : 7}` → 7 (the 1/0 arm is never evaluated).
        assert_eq!(
            count("proc f {} { return [expr {0 ? 1/0 : 7}] }", "W233"),
            0
        );
    }

    #[test]
    fn short_circuit_or_div_zero_silent() {
        // tclsh: `expr {1 || 1/0}` → 1 (RHS short-circuited away).
        assert_eq!(count("proc f {} { return [expr {1 || 1/0}] }", "W233"), 0);
    }

    #[test]
    fn short_circuit_and_div_zero_silent() {
        // tclsh: `expr {0 && 1/0}` → 0 (RHS short-circuited away).
        assert_eq!(count("proc f {} { return [expr {0 && 1/0}] }", "W233"), 0);
    }

    #[test]
    fn eager_left_of_and_div_zero_fires() {
        // The left operand of `&&` is always evaluated.
        // tclsh: `expr {1/0 && 1}` → divide by zero.
        assert_eq!(count("proc f {} { return [expr {1/0 && 1}] }", "W233"), 1);
    }

    // --- laziness: a constant guard FORCES the lazy arm (guaranteed error) ---

    #[test]
    fn forced_and_rhs_div_zero_fires() {
        // LHS constant-true → the && RHS always runs.
        // tclsh: `expr {1 && 1/0}` → divide by zero.
        assert_eq!(count("proc f {} { return [expr {1 && 1/0}] }", "W233"), 1);
    }

    #[test]
    fn forced_or_rhs_div_zero_fires() {
        // LHS constant-false → the || RHS always runs.
        // tclsh: `expr {0 || 1/0}` → divide by zero.
        assert_eq!(count("proc f {} { return [expr {0 || 1/0}] }", "W233"), 1);
    }

    #[test]
    fn forced_true_ternary_arm_div_zero_fires() {
        // Condition constant-true → the then-arm always runs.
        // tclsh: `expr {1 ? 1/0 : 7}` → divide by zero.
        assert_eq!(
            count("proc f {} { return [expr {1 ? 1/0 : 7}] }", "W233"),
            1
        );
    }

    #[test]
    fn forced_false_ternary_arm_div_zero_fires() {
        // Condition constant-false → the else-arm always runs.
        // tclsh: `expr {0 ? 7 : 1/0}` → divide by zero.
        assert_eq!(
            count("proc f {} { return [expr {0 ? 7 : 1/0}] }", "W233"),
            1
        );
    }

    #[test]
    fn bool_keyword_guard_forces_arm() {
        // `true`/`no` are constant guards too.
        // tclsh: `expr {true && 1/0}` and `expr {no || 1/0}` → divide by zero.
        assert_eq!(
            count("proc f {} { return [expr {true && 1/0}] }", "W233"),
            1
        );
        assert_eq!(count("proc f {} { return [expr {no || 1/0}] }", "W233"), 1);
    }

    #[test]
    fn float_constant_guard_forces_arm() {
        // A float literal guard is constant too — `1.0` true, `0.0` false.
        // tclsh: `expr {1.0 && 1/0}` / `expr {0.0 || 1/0}` / `expr {1.5 ? 1/0 : 7}`
        // all raise divide by zero.
        assert_eq!(count("proc f {} { return [expr {1.0 && 1/0}] }", "W233"), 1);
        assert_eq!(count("proc f {} { return [expr {0.0 || 1/0}] }", "W233"), 1);
        assert_eq!(
            count("proc f {} { return [expr {1.5 ? 1/0 : 7}] }", "W233"),
            1
        );
    }

    #[test]
    fn uppercase_bool_guard_forces_arm() {
        // `expr` accepts booleans case-insensitively.
        // tclsh: `expr {True && 1/0}`, `expr {TRUE && 1/0}`, `expr {No || 1/0}`
        // all raise divide by zero.
        assert_eq!(
            count("proc f {} { return [expr {True && 1/0}] }", "W233"),
            1
        );
        assert_eq!(
            count("proc f {} { return [expr {TRUE && 1/0}] }", "W233"),
            1
        );
        assert_eq!(count("proc f {} { return [expr {No || 1/0}] }", "W233"), 1);
    }

    #[test]
    fn unary_constant_guard_forces_arm() {
        // `-`/`+` keep truth (`-1` true), `!`/`not` invert (`!0` true).
        // tclsh: `expr {-1 && 1/0}` / `expr {!0 && 1/0}` → divide by zero.
        assert_eq!(count("proc f {} { return [expr {-1 && 1/0}] }", "W233"), 1);
        assert_eq!(count("proc f {} { return [expr {!0 && 1/0}] }", "W233"), 1);
    }

    #[test]
    fn unary_constant_dead_arm_stays_silent() {
        // `!1` is constant-false → the && RHS is short-circuited.
        // tclsh: `expr {!1 && 1/0}` → 0; `expr {False && 1/0}` → 0.
        assert_eq!(count("proc f {} { return [expr {!1 && 1/0}] }", "W233"), 0);
        assert_eq!(
            count("proc f {} { return [expr {False && 1/0}] }", "W233"),
            0
        );
    }

    #[test]
    fn nonconstant_guard_arm_stays_silent() {
        // A non-constant guard leaves the arm maybe-dead — no guaranteed error,
        // so W233 stays silent (preserves the dead-arm discipline).
        assert_eq!(count("proc f {c} { return [expr {$c && 1/0}] }", "W233"), 0);
        assert_eq!(count("proc f {c} { return [expr {$c || 1/0}] }", "W233"), 0);
        assert_eq!(
            count("proc f {c} { return [expr {$c ? 1/0 : 7}] }", "W233"),
            0
        );
    }
}

// Unreachable bounds suppressed.
//
// Dynamic-bounds findings must not fire from statically unreachable blocks (the
// same reachability discipline as divide-by-zero). tclsh: `expr {0}` → 0,
// `expr {1}` → 1, so the dead arms below genuinely never run.
mod unreachable_bounds_suppressed {
    use super::*;

    #[test]
    fn if_false_branch_no_w230() {
        let src = "proc f {} { if {0} { set i 5\n set x [lindex {a b} $i] } }";
        assert_eq!(count(src, "W230"), 0);
    }

    #[test]
    fn unreachable_else_branch_no_w230() {
        // cond constant-true → the else arm is unreachable.
        let src = "proc f {} { if {1} { set y 1 } else { set i 5\n set x [lindex {a b} $i] } }";
        assert_eq!(count(src, "W230"), 0);
    }

    #[test]
    fn reachable_control_still_fires() {
        // The same OOR access in live code must still warn.
        let src = "proc f {} { set i 5\n set x [lindex {a b} $i] }";
        assert_eq!(count(src, "W230"), 1);
    }
}

// List-expansion length.
//
// `[list {*}{...}]` expands at runtime, so its element count is not the arg
// count — `list_command_length` bails on any `{*}` expansion.
mod list_expansion_length {
    use super::*;

    // `set l [list {*}{a b}]` expands to a 2-element list, so `lindex $l 1`
    // is IN range and must NOT fire W230 — tclsh:
    //   `set l [list {*}{a b}]; llength $l` → 2 (8.6 + 9.0), `lindex $l 1` → "b".
    // A per-arg `starts_with("{*}")` bail-out cannot catch this (the segmenter
    // strips the `{*}` prefix, so the single arg `"a b"` never trips it), so
    // `list_command_length` bails whenever the command text contains `{*}`,
    // treating the length as unknown.
    #[test]
    fn list_expansion_does_not_false_fire() {
        // l = {a b} (length 2 after expansion); index 1 is valid.
        let src = "proc f {} { set l [list {*}{a b}]\n set i 1\n set x [lindex $l $i] }";
        assert_eq!(count(src, "W230"), 0);
    }

    #[test]
    fn plain_list_length_still_inferred() {
        // Control (no expansion) → length 2 known, index 5 is OOR. This is the
        // negative half of the bug pair and passes — it proves the W230 path is
        // otherwise healthy and the false positive above is specific to `{*}`.
        // tclsh: `list a b` → 2 elements; `lindex {a b} 5` → "".
        let src = "proc g {} { set l [list a b]\n set i 5\n set x [lindex $l $i] }";
        assert_eq!(count(src, "W230"), 1);
    }
}

// Interval fixpoint soundness.
//
// Forcing non-convergence requires patching `MAX_ITERS = 1`.
// `MAX_ITERS` is a private `const` in `intervals.rs` with no injection point, so
// the *unconverged* degrade-to-TOP branch is unreachable from an integration
// test. We cover the *convergent* counterpart: a well-behaved loop fixpoint
// yields sound, non-bottom intervals (and still fires the genuine OOR finding).
mod interval_fixpoint_soundness {
    use super::*;

    #[test]
    fn converged_fixpoint_still_fires_genuine_finding() {
        // The convergent counterpart to the unconverged-suppression case:
        // *with* convergence (the default cap), j proven past the end of a 3-list
        // → W230 fires. (The unconverged-suppression branch needs the private
        // MAX_ITERS=1 patch and is unreachable here.)
        let src = "proc f {} { for {set j 3} {$j < 10} {incr j} { set x [lindex {a b c} $j] } }";
        assert_eq!(count(src, "W230"), 1);
    }

    #[test]
    fn converged_compute_intervals_are_sound() {
        // The convergent counterpart to the all-TOP unconverged case:
        // the *converged* fixpoint for a counting loop produces a well-formed map
        // — every computed interval is a sound lattice element (never the empty
        // `lo > hi` sentinel), and the loop counter `i` is bounded below by 0.
        let src = "proc f {} { set i 0\n while {$i < 5} { incr i }\n return $i }";
        let (cu, key) = func(src);
        let fu = cu.procedures.get(&key).expect("::f lowered");
        let iv = compute_intervals_with(&fu.cfg, &fu.ssa, &fu.sccp.values, numbers());
        assert!(!iv.is_empty(), "a counting loop must yield interval facts");
        for &interval in iv.values() {
            assert!(
                !interval.is_bottom(),
                "a converged interval must never be the empty (lo>hi) sentinel: {interval:?}"
            );
        }
        // i is non-negative from its `set i 0` init and the `incr` (lo never < 0).
        let lat = intervals_of(fu);
        let some_i_nonneg = (1u32..=6)
            .filter_map(|v| lat("i", v))
            .any(|itv| matches!(itv.lo, Some(l) if l >= 0));
        assert!(
            some_i_nonneg,
            "the loop counter i must have a non-negative lower bound"
        );
    }
}

// compute_intervals (lattice surface, end-to-end).
//
// These drive the public `compute_intervals` directly. The constant
// values are Tcl-grounded; the widened loop-bound shape is structural.
mod compute_intervals_suite {
    use super::*;

    #[test]
    fn constant_arithmetic() {
        // tclsh: `set i 0; set n 10; expr {$i + $n}` → 10. The interval for
        // j₁ is the point [10, 10] == const(10).
        let src = "proc f {} { set i 0\n set n 10\n set j [expr {$i + $n}]\n return $j }";
        let (cu, key) = func(src);
        let fu = cu.procedures.get(&key).unwrap();
        let lat = intervals_of(fu);
        assert_eq!(lat("j", 1), Some(constant(10)));
    }

    #[test]
    fn loop_induction_widens_soundly() {
        // STRUCTURAL: the loop-header phi for i is widened to [0, +inf) — sound
        // (i >= 0), the upper bound unbounded without guard narrowing. (The
        // rotated-loop lowering keeps the header-phi at version 2, the
        // `("i", 2)` key.)
        let src = "proc f {} { for {set i 0} {$i < 10} {incr i} { puts $i } }";
        let (cu, key) = func(src);
        let fu = cu.procedures.get(&key).unwrap();
        let lat = intervals_of(fu);
        let header = lat("i", 2).expect("loop-header phi i₂");
        assert_eq!(header.lo, Some(0), "widened lower bound stays at 0");
        assert_eq!(header.hi, None, "widened upper bound is +inf (unbounded)");
    }

    #[test]
    fn unknown_is_top() {
        // A value derived from an opaque call has no bound.
        let src = "proc f {} { set x [someUnknownCmd]\n return $x }";
        let (cu, key) = func(src);
        let fu = cu.procedures.get(&key).unwrap();
        let lat = intervals_of(fu);
        assert!(
            lat("x", 1).expect("x₁").is_top(),
            "opaque-call result is TOP"
        );
    }
}

// Guard narrowing (refine_interval at a use site).
//
// `for` lowers to a *rotated* loop, so the body is dominated by the latch's
// `$i < 10` true edge and the narrowable version is the post-increment i₃, which
// refines to [1, 9] rather than i₂→[0,9]. The
// intent — the constant `< 10` guard pulls the widened upper bound
// down to 9 inside the body — holds exactly; only the SSA version / lower bound
// the loop-rotation assigns differs (a structural detail, not a Tcl fact). So we
// assert the load-bearing fact (some version narrows to a bounded `hi == 9`) and
// the symbolic-bound non-narrowing, rather than pinning the rotation's version.
//
// tclsh grounding: inside `for {set i 0} {$i < 10} {incr i}` the body runs with
// i ∈ 0..9 (the guard excludes 10) — the upper bound of 9 is the real Tcl range.
mod guard_narrowing {
    use super::*;

    #[test]
    fn constant_bound_loop_body_narrows() {
        // The body is dominated by the `$i < 10` true edge, so some SSA version
        // of i narrows from the widened [_, +inf) to a bounded interval with
        // upper bound 9 (tclsh: the loop body sees i ≤ 9). STRUCTURAL on which
        // version; the `hi == 9` ceiling is the Tcl-grounded fact.
        let src = "proc f {} { for {set i 0} {$i < 10} {incr i} { puts $i } }";
        let (cu, key) = func(src);
        let fu = cu.procedures.get(&key).unwrap();
        let iv = compute_intervals_with(&fu.cfg, &fu.ssa, &fu.sccp.values, numbers());
        let gi = build_guard_index(&fu.cfg, &fu.ssa);
        let body = loop_body_block(fu);
        let sym = fu.ssa.var_symbol("i").expect("i interned");
        let narrowed_to_9 = (1u32..=6).any(|ver| {
            // Only consider versions that actually have a base interval (skip
            // version 0 / undefined which refine to TOP).
            iv.get(&(sym, ver)).is_some_and(|_| {
                let pc = pred_counts(fu);
                let r = refine_interval(
                    &iv,
                    &fu.cfg,
                    &fu.ssa,
                    body,
                    "i",
                    ver,
                    tcl_compiler::intervals::GuardTables {
                        guard_index: &gi,
                        pred_counts: &pc,
                        numbers: numbers(),
                    },
                );
                !r.is_top() && !r.is_bottom() && r.hi == Some(9)
            })
        });
        assert!(
            narrowed_to_9,
            "the `< 10` guard must narrow some version of i to an upper bound of 9 in the loop body"
        );
    }

    #[test]
    fn symbolic_bound_does_not_narrow() {
        // `$i < [llength $l]` has a symbolic RHS — a non-relational interval
        // domain cannot turn it into a constant bound, so NO version gains a
        // finite upper bound (no unsound narrowing); the lower bound stays ≥ 0.
        // STRUCTURAL (domain non-relationality); grounded by the idiom being the
        // canonical safe iteration in Tcl.
        let src = "proc f {l} { for {set i 0} {$i < [llength $l]} {incr i} { puts $i } }";
        let (cu, key) = func(src);
        let fu = cu.procedures.get(&key).unwrap();
        let iv = compute_intervals_with(&fu.cfg, &fu.ssa, &fu.sccp.values, numbers());
        let gi = build_guard_index(&fu.cfg, &fu.ssa);
        let body = loop_body_block(fu);
        let sym = fu.ssa.var_symbol("i").expect("i interned");
        // Consider only the *widened* versions — those whose base interval is
        // already unbounded above (`hi == None`); a point-valued init like
        // i₁ == [0,0] legitimately has a finite `hi` and is not what "narrow"
        // means. For every widened version, the symbolic `[llength $l]` guard
        // must NOT pull the upper bound in (it stays +inf), and the lower bound
        // stays ≥ 0. At least one such widened version must exist (the loop phi).
        let mut saw_widened = false;
        for ver in 1u32..=6 {
            let Some(base) = iv.get(&(sym, ver)).copied() else {
                continue;
            };
            if base.hi.is_some() {
                continue; // not a widened-above version (e.g. the init const)
            }
            saw_widened = true;
            let pc = pred_counts(fu);
            let r = refine_interval(
                &iv,
                &fu.cfg,
                &fu.ssa,
                body,
                "i",
                ver,
                tcl_compiler::intervals::GuardTables {
                    guard_index: &gi,
                    pred_counts: &pc,
                    numbers: numbers(),
                },
            );
            if r.is_bottom() {
                continue;
            }
            assert_eq!(
                r.hi, None,
                "a symbolic bound must not give the widened i a finite upper bound (version {ver}): {r:?}"
            );
            if let Some(lo) = r.lo {
                assert!(
                    lo >= 0,
                    "the lower bound stays non-negative (version {ver}): {r:?}"
                );
            }
        }
        assert!(
            saw_widened,
            "the loop must produce at least one widened (+inf-above) version of i"
        );
    }
}

// Interval arithmetic / intersect (observable subset).
//
// The raw operators are private to `intervals.rs` (covered by its in-crate
// `#[cfg(test)]` module). Their *observable* algebra surfaces end-to-end through
// `compute_intervals`: a constant fold exercises `add` over point intervals, the
// loop widening exercises `widen`/`join`, and the `set j [expr {$i + $n}]` fold
// proves `add(const, const)` == const. These cover the
// integration-reachable slice of the interval arithmetic.
mod interval_arithmetic_observable {
    use super::*;

    #[test]
    fn add_of_point_intervals_via_fold() {
        // `add(const(2), const(3)) == const(5)`,
        // observed through the analysis: `set a 2; set b 3; expr {$a + $b}` → 5.
        // tclsh: `expr {2 + 3}` → 5.
        let src = "proc f {} { set a 2\n set b 3\n set c [expr {$a + $b}]\n return $c }";
        let (cu, key) = func(src);
        let fu = cu.procedures.get(&key).unwrap();
        let lat = intervals_of(fu);
        assert_eq!(lat("c", 1), Some(constant(5)));
    }

    #[test]
    fn sub_of_point_intervals_via_fold() {
        // `sub(const(5), const(3)) == const(2)` observed end-to-end.
        // tclsh: `expr {5 - 3}` → 2.
        let src = "proc f {} { set a 5\n set b 3\n set c [expr {$a - $b}]\n return $c }";
        let (cu, key) = func(src);
        let fu = cu.procedures.get(&key).unwrap();
        let lat = intervals_of(fu);
        assert_eq!(lat("c", 1), Some(constant(2)));
    }

    #[test]
    fn mul_of_point_intervals_via_fold() {
        // `mul(const(2), const(3)) == const(6)` observed end-to-end.
        // tclsh: `expr {2 * 3}` → 6.
        let src = "proc f {} { set a 2\n set b 3\n set c [expr {$a * $b}]\n return $c }";
        let (cu, key) = func(src);
        let fu = cu.procedures.get(&key).unwrap();
        let lat = intervals_of(fu);
        assert_eq!(lat("c", 1), Some(constant(6)));
    }

    #[test]
    fn widen_sends_growing_loop_bound_to_infinity() {
        // `widen([0,0], [0,1]) == [0, None]` is exactly what the loop-induction
        // widening produces: the upper bound that grows each iteration is sent to
        // +inf while the stable lower bound (0) is kept. Observed via the loop
        // header phi i₂ == [0, +inf). STRUCTURAL (widening shape).
        let src = "proc f {} { for {set i 0} {$i < 10} {incr i} { puts $i } }";
        let (cu, key) = func(src);
        let fu = cu.procedures.get(&key).unwrap();
        let lat = intervals_of(fu);
        let header = lat("i", 2).expect("i₂");
        assert_eq!((header.lo, header.hi), (Some(0), None));
    }
}

// Dialect-dependent numerals (the threaded `NumberSyntax`).
//
// The interval domain reads every literal through the one shared numeral
// facility (`tcl_syntax::number`), under the grammar of the release being
// analysed *for*, passed explicitly. The releases genuinely disagree:
//
//   tclsh8.6:  expr {0755}  → 493      (octal by leading zero)
//   tclsh9.0:  expr {0755}  → 755      (`KILL_OCTAL`; leading zero is decimal)
//   tclsh8.4:  expr {0o17}  → invalid bareword; tclsh8.5+ → 15
//   tclsh9.0:  expr {1_000} → 1000;    8.x → invalid bareword
//
// so a dialect-blind read is *wrong*, not merely imprecise — hence these run
// the same source under two grammars and assert two different constants.
mod dialect_numerals {
    use super::*;
    use tcl_dialect::NumberSyntax;

    /// The finite upper bound the guard `$i < <literal>` proves for the widened
    /// loop counter inside the body of
    /// `for {set i 0} {$i < <literal>} {incr i} …`, under `dialect`.
    ///
    /// This is the guard-narrowing path (`refine_interval` →
    /// `guard_constraint` → the numeral reader), deliberately chosen over a
    /// foldable `set x [expr {…}]`: SCCP's own const-folder seeds that case
    /// from its lattice before the interval transfer runs, so it would not
    /// exercise this module's reader at all. Only versions whose base interval
    /// is unbounded above are considered, so `None` means "the guard proved no
    /// ceiling" rather than "some point interval happened to have one".
    fn guard_ceiling(literal: &str, dialect: &str) -> Option<i64> {
        let src = format!(
            "proc f {{}} {{ for {{set i 0}} {{$i < {literal}}} {{incr i}} {{ puts $i }} }}"
        );
        let cu = CompilationUnit::build_for(&src, static_context_for(dialect).commands(), false);
        let fu = cu.procedures.get("::f").expect("::f lowered");
        let numbers = numbers_for_dialect(Some(
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        ));
        let iv = compute_intervals_with(&fu.cfg, &fu.ssa, &fu.sccp.values, numbers);
        let gi = build_guard_index(&fu.cfg, &fu.ssa);
        let pc = pred_counts(fu);
        // A spelling that is not a numeral for this release may not even parse
        // to a comparison — `$i < 0xZZ` leaves no `Binary`-guarded branch to
        // narrow at, which is itself "no bound proved".
        let body = fu.cfg.blocks.values().find_map(|b| match &b.terminator {
            Some(Terminator::Branch {
                true_target,
                condition: tcl_compiler::expr_ast::ExprNode::Binary { .. },
                ..
            }) => Some(*true_target),
            _ => None,
        })?;
        let sym = fu.ssa.var_symbol("i").expect("i interned");
        let mut ceiling: Option<i64> = None;
        for ver in 1u32..=6 {
            let Some(base) = iv.get(&(sym, ver)).copied() else {
                continue;
            };
            if base.hi.is_some() {
                continue; // not a widened-above version (e.g. the `set i 0` init)
            }
            let refined = refine_interval(
                &iv,
                &fu.cfg,
                &fu.ssa,
                body,
                "i",
                ver,
                tcl_compiler::intervals::GuardTables {
                    guard_index: &gi,
                    pred_counts: &pc,
                    numbers,
                },
            );
            if refined.is_bottom() {
                continue;
            }
            if let Some(hi) = refined.hi {
                ceiling = Some(ceiling.map_or(hi, |best: i64| best.max(hi)));
            }
        }
        ceiling
    }

    /// The same `$i < 0755` guard proves `i <= 492` for an 8.x target and
    /// `i <= 754` for a 9.0 one (tclsh8.6 `expr {0755}` → 493, tclsh9.0 → 755).
    /// Before the dialect was threaded both read 755, so an 8.x analysis got a
    /// range 262 wider than reality — and interval facts feed the W230–W233
    /// range diagnostics and the native-integer proof.
    #[test]
    fn leading_zero_guard_bound_is_octal_for_8_x_and_decimal_for_9_0() {
        assert_eq!(guard_ceiling("0755", "tcl8.5"), Some(492));
        assert_eq!(guard_ceiling("0755", "tcl8.6"), Some(492));
        assert_eq!(guard_ceiling("0755", "tcl9.0"), Some(754));
    }

    /// `0o17` is 15 from 8.5 and not a numeral at all in 8.4 (no `0o` prefix
    /// before `tclStrToD.c`'s `ZERO_O` state), so no bound is proved there.
    #[test]
    fn octal_prefix_guard_bound_appears_in_8_5() {
        assert_eq!(guard_ceiling("0o17", "tcl8.5"), Some(14));
        assert_eq!(guard_ceiling("0o17", "tcl9.0"), Some(14));
        assert_eq!(
            guard_ceiling("0o17", "tcl8.4"),
            None,
            "0o17 is a bareword under 8.4 — no constant bound"
        );
    }

    /// `0d99` and `1_000` are 9.0-only spellings: a bound there, none before.
    #[test]
    fn decimal_prefix_and_digit_separators_are_9_0_only() {
        for (literal, ceiling) in [("0d99", 98), ("1_000", 999)] {
            assert_eq!(
                guard_ceiling(literal, "tcl9.0"),
                Some(ceiling),
                "{literal} under 9.0"
            );
            for old in ["tcl8.4", "tcl8.5", "tcl8.6"] {
                assert_eq!(
                    guard_ceiling(literal, old),
                    None,
                    "{literal} must prove no bound under {old}"
                );
            }
        }
    }

    /// A radix-invalid digit is never a numeral, in any release — no bound.
    #[test]
    fn radix_invalid_digits_prove_no_bound() {
        for literal in ["0o8", "0xZZ"] {
            for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0"] {
                assert_eq!(
                    guard_ceiling(literal, dialect),
                    None,
                    "{literal} must prove no bound under {dialect}"
                );
            }
        }
    }

    /// A `set x <literal>` const value is read only as canonical decimal, so a
    /// dialect-ambiguous leading-zero spelling contributes no bound in any
    /// release rather than a possibly-wrong one. (`0755` parses as 755 to a
    /// plain decimal reader but *means* 493 up to 8.6.)
    #[test]
    fn leading_zero_const_value_abstains() {
        let x1 = |src: &str, dialect: &str| -> Option<Interval> {
            let cu = CompilationUnit::build_for(src, static_context_for(dialect).commands(), false);
            let fu = cu.procedures.get("::f").expect("::f lowered");
            let iv = compute_intervals_with(
                &fu.cfg,
                &fu.ssa,
                &fu.sccp.values,
                numbers_for_dialect(Some(
                    tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
                )),
            );
            let sym = fu.ssa.var_symbol("x")?;
            iv.get(&(sym, 1)).copied()
        };
        let src = "proc f {} { set x 0755\n return $x }";
        for dialect in ["tcl8.5", "tcl8.6", "tcl9.0"] {
            assert!(
                x1(src, dialect).is_none_or(Interval::is_top),
                "`set x 0755` must contribute no bound under {dialect}"
            );
        }
        // A canonical decimal still does.
        let plain = "proc f {} { set x 755\n return $x }";
        assert_eq!(x1(plain, "tcl8.6"), Some(constant(755)));
    }

    /// `NumberSyntax` for a dialect name comes from its profile's grammar, and
    /// a nameless build falls back to 9.0 (the crate-wide convention).
    #[test]
    fn numbers_for_dialect_reads_the_profile_grammar() {
        assert_eq!(
            numbers_for_dialect(Some(
                tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile()
            )),
            NumberSyntax::Tcl84
        );
        assert_eq!(
            numbers_for_dialect(Some(
                tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile()
            )),
            NumberSyntax::Tcl85
        );
        assert_eq!(
            numbers_for_dialect(Some(
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
            )),
            NumberSyntax::Tcl85
        );
        assert_eq!(
            numbers_for_dialect(Some(
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
            )),
            NumberSyntax::Tcl90
        );
        assert_eq!(numbers_for_dialect(None), NumberSyntax::Tcl90);
    }
}
