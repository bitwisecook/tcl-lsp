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

//! Depth coverage for the `tcl-compiler` optimiser passes — the *uncovered*
//! branches the existing `optimiser.rs` / `optimiser_coverage.rs`
//! suites do not reach.
//!
//! Those two suites drive the common O100–O129 happy paths (constant folding,
//! the InstCombine/strength-reduction families, the headline DSE/structure-
//! elimination cases). This file deliberately targets the *edge* branches in
//! `src/optimiser/{elimination,code_sinking,chain_fold,branch_folding,
//! structure_elimination,tail_call}.rs`:
//!
//!  * `elimination.rs` — O107 unreachable code via the **after-`return`** path
//!    (distinct from the O112 structure-elimination path that owns `if {0}` /
//!    `while {0}`), the multi-step **O108 ADCE** fixpoint, the scope-alias
//!    suppression for `trace` / `namespace upvar` / `variable`, the
//!    **RMW-hidden-read** keep-alive (`lappend acc [incr i $j]`, `incr i [expr
//!    {$w}]`), and the cross-scope traced/`::`-global survival.
//!  * `code_sinking.rs` (O125) — sinking into a **switch** arm, the
//!    `try_deeper_sink` **decline** when a nested decision's condition reads the
//!    variable, the multi-use **anchor-at-first** case, sinking inside a
//!    **loop body** (compound-statement recursion), and the conditional-redefine
//!    decline.
//!  * `chain_fold.rs` — the **O130** list-build chain (untouched by the existing
//!    suites, which only exercise O104), and the precise-flow fold **past an
//!    interleaved literal `set` to a different variable**.
//!  * `branch_folding.rs` — the in-condition rewrite cascade (`strength_reduce`
//!    O113, `strlen` O117, De-Morgan carried as O113).
//!  * `structure_elimination.rs` — the `for {} {0} {}` / `while {1}` edge cases.
//!  * `tail_call.rs` — O122 loop conversion driven from a **switch**-arm
//!    self-call.
//!  * iRules cross-event O126/O109 connection-scope paths (multi-event
//!    `when` scripts) — a store consumed by a later event must survive.
//!
//! ## C-Tcl proof vs compiler-internal facts
//!
//! Two kinds of assertion live here, and the split matters:
//!
//!  * **Semantics-preserving values** — whenever a snippet has an observable
//!    computed value, the optimised program is asserted to still yield it, and
//!    the value is pinned to real C-Tcl via `scripts/dev/tclsh_check.sh` on
//!    tclsh **8.6 and 9.0** (both agree for every value cited). Each is tagged
//!    with a `// tclsh:` comment carrying the exact ground-truth value/command.
//!  * **Optimiser-structural facts** — *which* store was eliminated, *which*
//!    `O###` fired, *where* a sunk assignment landed. These are properties of
//!    the compiler IR, not of Tcl, so they are asserted structurally (on the
//!    emitted codes / rewritten text) and are flagged as such. Several pair a
//!    firing case with a control snippet whose only difference is the feature
//!    under test, so the structural delta is the load-bearing signal.
//!
//! No genuine miscompiles were found while authoring this file; every test
//! below is live and passing.

use tcl_compiler::optimiser::manager::{
    apply_optimisations, optimise_source_multipass, optimise_with_dialect,
};
use tcl_registry::registry_for_dialect;

const TCL: &str = "tcl8.6";
const IR: &str = "f5-irules";

// ---------------------------------------------------------------------------
// Shared helpers (mirror the existing suites' harness exactly).
// ---------------------------------------------------------------------------

/// Every `Oxxx` code emitted by a single optimiser pass over `src`.
fn opt_codes(src: &str, dialect: &str) -> Vec<String> {
    let registry = registry_for_dialect(dialect);
    let d = (!dialect.is_empty()).then(|| tcl_dialect::DialectProfile::by_name(dialect));
    optimise_with_dialect(src, registry, d)
        .iter()
        .map(|o| o.code.as_str().to_owned())
        .collect()
}

/// True if any optimisation with `code` fires on `src` under `dialect`.
fn opt_fires(src: &str, dialect: &str, code: &str) -> bool {
    opt_codes(src, dialect).iter().any(|c| c == code)
}

/// Count optimisations carrying `code`.
fn opt_count(src: &str, dialect: &str, code: &str) -> usize {
    opt_codes(src, dialect)
        .iter()
        .filter(|c| *c == code)
        .count()
}

/// Apply all (non-hint) optimisations and return the rewritten source.
fn optimised(src: &str, dialect: &str) -> String {
    let registry = registry_for_dialect(dialect);
    let d = (!dialect.is_empty()).then(|| tcl_dialect::DialectProfile::by_name(dialect));
    apply_optimisations(src, &optimise_with_dialect(src, registry, d))
}

// ===========================================================================
// elimination.rs — O107 unreachable code via the after-`return` path
//
// The existing suites only ever reach O112 (structure elimination) for `if {0}`
// / `while {0}`: the overlap selector prefers O112 (priority 9) over the
// per-statement O107. Dead code *after a `return`* is not owned by any
// structural collapse, so it is the clean way to drive the O107
// `emit_unreachable` branch through to the selected output.
// ===========================================================================

#[test]
fn elimination_o107_dead_code_after_return() {
    // Code after `return` in a proc is unreachable (SCCP marks the block
    // non-executable). tclsh: the proc returns 1 regardless of the dead tail.
    // tclsh: `proc f {} { return 1; puts never; set z 2 }; f`  ⇒ 1
    //        `proc f {} { return 1 }; f`                        ⇒ 1
    let after_ret = "proc ::f {} {\n    return 1\n    puts never\n    set z 2\n}";
    assert!(
        opt_fires(after_ret, TCL, "O107"),
        "O107 should fire on dead post-return code"
    );
    // Two dead statements ⇒ two O107 emissions (structural).
    assert_eq!(opt_count(after_ret, TCL, "O107"), 2);
    let out = optimised(after_ret, TCL);
    assert!(out.contains("return 1"), "live return must survive: {out}");
    assert!(
        !out.contains("puts never"),
        "dead `puts never` must be removed: {out}"
    );
    assert!(
        !out.contains("set z 2"),
        "dead `set z 2` must be removed: {out}"
    );

    // `if {1} { return 5 }` followed by code: O112 unwraps the `if`, and the
    // now-after-return `puts never` is reported O107. tclsh:
    //   `proc f {} { if {1} { return 5 }; puts never }; f`  ⇒ 5
    //   `proc f {} { return 5 }; f`                          ⇒ 5
    let if_ret = "proc ::f {} {\n    if {1} { return 5 }\n    puts never\n}";
    assert!(
        opt_fires(if_ret, TCL, "O107"),
        "O107 should fire on code after an unwrapped if-return"
    );
    assert!(
        opt_fires(if_ret, TCL, "O112"),
        "O112 should unwrap the constant if"
    );
    let out = optimised(if_ret, TCL);
    assert!(out.contains("return 5"));
    assert!(
        !out.contains("puts never"),
        "post-return code must be removed: {out}"
    );
}

// ===========================================================================
// elimination.rs — multi-step O108 ADCE fixpoint
//
// The existing suites assert a 1-link transitive chain; here a 4-deep
// `a → b → c → d` chain where only the unused links collapse, exercising the
// `run_adce_fixpoint` loop iterating more than once.
// ===========================================================================

#[test]
fn elimination_o108_deep_transitive_chain() {
    // `set a 1 → set b [expr {$a+1}] → set c [expr {$b+1}] → set d [expr {$c+1}]`
    // with `d` never read: the whole chain is transitively dead. tclsh proves
    // the value is unaffected:
    //   `proc f {} { set a 1; set b [expr {$a+1}]; set c [expr {$b+1}]; set d [expr {$c+1}]; return 7 }; f` ⇒ 7
    //   `proc f {} { return 7 }; f`                                                                          ⇒ 7
    let chain = "proc ::f {} {\n    set a 1\n    set b [expr {$a + 1}]\n    set c [expr {$b + 1}]\n    set d [expr {$c + 1}]\n    return 7\n}";
    // Structural: the head store is O126 (unused), and the three transitive
    // links collapse via the ADCE fixpoint (O108 ×3).
    assert!(opt_fires(chain, TCL, "O108"));
    assert_eq!(
        opt_count(chain, TCL, "O108"),
        3,
        "expected the 3 transitive links as O108"
    );
    let out = optimised(chain, TCL);
    assert!(out.contains("return 7"));
    assert!(
        !out.contains("set b"),
        "transitive dead link must be removed: {out}"
    );
    assert!(
        !out.contains("set d"),
        "head of the dead chain must be removed: {out}"
    );
}

// ===========================================================================
// elimination.rs — scope-alias suppression (`scan_scope_aliases` branches the
// existing suites never reach: `trace`, `namespace upvar`, `variable`).
//
// Each is a firing/control pair: the control (without the alias) shows the
// dead/unused store IS removed, so the suppression is the load-bearing delta.
// These are compiler-internal (a write through an alias escapes the frame, so
// the optimiser keeps it) — structural assertions only.
// ===========================================================================

#[test]
fn elimination_trace_variable_suppresses_dead_store() {
    // A write trace fires its callback on *every* `set`, so a traced variable is
    // observable and must never be flagged dead/unused. Recognises both the 8.5+
    // `trace add variable …` and the 8.4 `trace variable …` spellings.
    let traced = "proc ::f {} {\n    trace add variable c write log\n    set c 1\n    set c 2\n}";
    assert!(
        !opt_fires(traced, TCL, "O126"),
        "traced var must not be O126"
    );
    assert!(
        !opt_fires(traced, TCL, "O109"),
        "traced var must not be O109"
    );
    assert_eq!(
        optimised(traced, TCL),
        traced,
        "traced writes must survive verbatim"
    );

    let traced84 = "proc ::f {} {\n    trace variable c w log\n    set c 1\n    set c 2\n}";
    assert!(!opt_fires(traced84, TCL, "O126"));
    assert!(!opt_fires(traced84, TCL, "O109"));

    // Control: identical body without the trace → both stores are flagged unused
    // (O126 ×2), proving the trace is what suppresses them.
    let control = "proc ::f {} {\n    set c 1\n    set c 2\n}";
    assert_eq!(
        opt_count(control, TCL, "O126"),
        2,
        "control must fire O126 on both unused stores"
    );
}

#[test]
fn elimination_namespace_upvar_and_variable_alias_suppress() {
    // `namespace upvar ns caller local` binds `local` to another scope — a write
    // to it is visible there, so it is never dead/unused.
    let nsupvar = "proc ::f {} {\n    namespace upvar ::ns s local\n    set local 5\n}";
    assert!(
        !opt_fires(nsupvar, TCL, "O126"),
        "namespace-upvar local must not be O126"
    );
    assert_eq!(optimised(nsupvar, TCL), nsupvar);
    // Control: the same `set local 5` without the alias is unused (O126).
    let nsupvar_control = "proc ::f {} {\n    set local 5\n}";
    assert!(
        opt_fires(nsupvar_control, TCL, "O126"),
        "control without alias must fire O126"
    );

    // `variable v 1` declares a namespace variable; a later `set v 2` writes
    // namespace state and must be kept.
    let var_decl = "proc ::f {} { variable v 1; set v 2 }";
    assert!(
        !opt_fires(var_decl, TCL, "O126"),
        "namespace `variable` write must not be O126"
    );
    assert_eq!(optimised(var_decl, TCL), var_decl);
    // Control: replace `variable` with a plain `set` → the first store is unused.
    let var_decl_control = "proc ::f {} { set v 1; set v 2 }";
    assert!(
        opt_fires(var_decl_control, TCL, "O126"),
        "control without `variable` must fire O126"
    );
}

// ===========================================================================
// elimination.rs — RMW-hidden-read keep-alive (`collect_rmw_hidden_reads` /
// `dollar_reads_in_cmd_subs`). A read-modify-write command's target buried in a
// substitution keeps a feeding `set` alive even though the shallow word scan
// (and name-level SSA) sees no read.
// ===========================================================================

#[test]
fn elimination_rmw_hidden_read_keeps_feeding_set() {
    // `lappend acc [incr i $j]` reads + writes `i` inside the substitution, so
    // the feeding `set i 0` is live. tclsh: the value depends on it —
    //   `proc f {j} { set i 0; lappend acc [incr i $j]; return $acc }; f 2` ⇒ 2
    let rmw = "proc ::f {j} {\n    set i 0\n    lappend acc [incr i $j]\n    return $acc\n}";
    assert!(
        !opt_fires(rmw, TCL, "O126"),
        "feeding `set i 0` must not be O126"
    );
    assert!(
        !opt_fires(rmw, TCL, "O109"),
        "feeding `set i 0` must not be O109"
    );
    assert!(
        optimised(rmw, TCL).contains("set i 0"),
        "RMW-fed store must survive"
    );
    // Control: replace the `[incr i $j]` operand with a literal — now `set i 0`
    // is genuinely unused and IS removed (O126).
    let rmw_control = "proc ::f {j} {\n    set i 0\n    lappend acc foo\n    return $acc\n}";
    assert!(
        opt_fires(rmw_control, TCL, "O126"),
        "control without the hidden read must fire O126"
    );

    // `incr i [expr {$w}]`: the `$w` read is buried inside the `[expr {…}]`
    // braces (invisible to the brace-aware shallow scan), but `expr`
    // re-evaluates it — `dollar_reads_in_cmd_subs` keeps `set w 3` alive.
    // tclsh: `proc f {} { set w 3; set i 0; incr i [expr {$w}]; return $i }; f` ⇒ 3
    let expr_read =
        "proc ::f {} {\n    set w 3\n    set i 0\n    incr i [expr {$w}]\n    puts $i\n}";
    assert!(
        !opt_fires(expr_read, TCL, "O126"),
        "`set w 3` read inside [expr] must not be O126"
    );
    assert!(
        optimised(expr_read, TCL).contains("set w 3"),
        "expr-buried read must keep `set w 3`"
    );
}

// ===========================================================================
// elimination.rs — a `::`-qualified / traced global write survives cross-scope.
// `scan_module_traced_globals` closes the gap where the `trace` lives in a
// different proc than the write; the `var.starts_with("::")` guard also keeps a
// fully-qualified global write. Either way the write must not be eliminated —
// a soundness (semantics-preserving) requirement.
// ===========================================================================

#[test]
fn elimination_cross_proc_global_write_survives() {
    // `set ::counter 1` in one proc is observable everywhere (and a write trace
    // in another proc makes that explicit). Eliminating it leaves `::counter`
    // undefined — a miscompile. The whole program must be left intact here.
    let src = "proc ::tr {} { trace add variable ::counter write log }\nproc ::w {} { set ::counter 1 }\n::tr\n::w";
    assert!(
        !opt_fires(src, TCL, "O109"),
        "cross-proc global write must not be O109"
    );
    assert!(
        !opt_fires(src, TCL, "O126"),
        "cross-proc global write must not be O126"
    );
    assert!(
        optimised(src, TCL).contains("set ::counter 1"),
        "global write must survive: {}",
        optimised(src, TCL)
    );
}

// ===========================================================================
// code_sinking.rs (O125) — sink into a `switch` arm.
//
// The existing suites only sink into `if` branches; the `Statement::Switch` arm
// is the untested decision shape in `decision_branch_bodies` /
// `any_decision_body_uses_var`.
// ===========================================================================

#[test]
fn code_sinking_o125_into_switch_arm() {
    // `set b foo` followed by a `switch` whose subject does not read `b` and
    // whose first arm uses it: sink into that arm. tclsh proves the value is
    // preserved (with the matching subject `a == x`):
    //   `set b foo; set a x; switch $a { x { set out $b } y { set out no } }; set out` ⇒ foo
    //   sunk:      `set a x; switch $a { x { set b foo; set out $b } … }; set out`     ⇒ foo
    let src = "set b foo\nswitch $a {\n    x { puts $b }\n    y { puts no }\n}";
    assert!(
        opt_fires(src, TCL, "O125"),
        "O125 should sink into the switch arm"
    );
    let out = optimised(src, TCL);
    // Structural: the sunk `set b foo` is prepended into the arm that uses `b`
    // (the `y` arm is untouched). The overlap selector keeps the outer
    // assignment too — re-assigning the same constant is harmless. tclsh proves
    // the keep-both applied form is still sound:
    //   `set b foo; set a x; switch $a { x { set b foo; set out $b } … }; set out` ⇒ foo
    assert!(
        out.contains("set b foo; puts $b"),
        "sunk def must be prepended into the using arm: {out}"
    );
    assert!(
        !out.contains("y { set b foo"),
        "the non-using `y` arm must be untouched: {out}"
    );
}

// ===========================================================================
// code_sinking.rs (O125) — `try_deeper_sink` declines when a *nested* decision
// reads the variable in its condition (so descending past it is unsound). The
// def then sinks only to the outer branch body, not into the inner arm.
// ===========================================================================

#[test]
fn code_sinking_o125_declines_deeper_when_inner_condition_reads_var() {
    // `set b $x; if {$a} { if {$b > 0} { … } else { … } } else { … }`. The inner
    // `if {$b > 0}` condition reads `b`, so the deep sink is refused; `set b $x`
    // is sunk to the *outer* branch body (anchored before the inner `if`).
    // tclsh proves both forms agree (a=1, x=5 ⇒ the inner true arm runs):
    //   `proc f {a x} { set b $x; if {$a} { if {$b > 0} { return hi } else { return $b } } else { return no } }; f 1 5` ⇒ hi
    //   sunk-to-outer: `… if {$a} { set b $x; if {$b > 0} { return hi } … } …`; f 1 5                                  ⇒ hi
    let src = "proc ::f {a x} { set b $x; if {$a} { if {$b > 0} { puts hi } else { puts $b } } else { puts no } }";
    assert!(opt_fires(src, TCL, "O125"));
    let out = optimised(src, TCL);
    // The sunk text anchors at the OUTER branch (it carries the inner `if`), not
    // inside the inner arm — `set b $x` immediately precedes the inner `if`.
    assert!(
        out.contains("set b $x; if {$b > 0}"),
        "decline-deeper must anchor `set b $x` before the inner condition: {out}"
    );
}

// ===========================================================================
// code_sinking.rs (O125) — multi-use branch anchors at the first using
// statement (`find_deepest_targets` with `using.len() > 1`, which skips the
// descend-into-decision optimisation and prepends at the first use).
// ===========================================================================

#[test]
fn code_sinking_o125_multi_use_anchors_at_first() {
    // The if-body uses `$x` twice, so the def is sunk to the FIRST use rather
    // than descending. tclsh (a=1) proves the value is preserved:
    //   `proc f {a} { set x 1; if {$a} { return [list $x $x] } else { return no } }; f 1` ⇒ 1 1
    //   sunk: `… if {$a} { set x 1; return [list $x $x] } …`; f 1                          ⇒ 1 1
    let src = "proc ::f {a} { set x 1; if {$a} { puts $x; puts $x } else { puts no } }";
    assert!(opt_fires(src, TCL, "O125"));
    let out = optimised(src, TCL);
    // Structural: the sunk text prepends at the first `puts $x` of the branch.
    assert!(
        out.contains("set x 1; puts $x"),
        "multi-use sink must anchor at the first using statement: {out}"
    );
}

// ===========================================================================
// code_sinking.rs (O125) — recursion into compound bodies. A sink target inside
// a `for` / `while` body exercises `walk_script`'s loop-body recursion (the
// existing suites only test top-level / proc-body sinks).
// ===========================================================================

#[test]
fn code_sinking_o125_inside_loop_body() {
    // `set x 1; if {$a} { puts $x } …` nested inside a `for` body. tclsh proves
    // the per-iteration value is preserved (n=2, a=1 ⇒ "1" each pass):
    //   while form: `…{ set x 1; if {$a} { lappend r $x } else { lappend r no }; … }`; f 2 1 ⇒ 1 1
    //   sunk:       `…{ if {$a} { set x 1; lappend r $x } else { lappend r no }; … }`; f 2 1 ⇒ 1 1
    let for_body = "proc ::f {n a} { for {set i 0} {$i < $n} {incr i} { set x 1; if {$a} { puts $x } else { puts no } } }";
    assert!(
        opt_fires(for_body, TCL, "O125"),
        "O125 should sink inside a for-body"
    );
    assert!(optimised(for_body, TCL).contains("set x 1; puts $x"));

    let while_body = "proc ::f {n a} { while {$n > 0} { set x 1; if {$a} { puts $x } else { puts no }; incr n -1 } }";
    assert!(
        opt_fires(while_body, TCL, "O125"),
        "O125 should sink inside a while-body"
    );
    assert!(optimised(while_body, TCL).contains("set x 1; puts $x"));
}

// ===========================================================================
// code_sinking.rs (O125) — declines when the branch *conditionally redefines*
// the variable before its use, so sinking the def in would be observed
// differently. A clean negative for the pass.
// ===========================================================================

#[test]
fn code_sinking_o125_declines_on_conditional_redefine() {
    // The branch body redefines `x` (inside a nested `if`) before reading it, so
    // sinking the outer `set x 1` into the branch is refused — the source is left
    // byte-identical, which is trivially semantics-preserving. tclsh confirms the
    // ORIGINAL program's values (a=1 ⇒ c selects 2 or 1):
    //   `proc f {a c} { set x 1; if {$a} { if {$c} { set x 2 }; return $x } else { return no } }; list [f 1 1] [f 1 0]` ⇒ 2 1
    let src =
        "proc ::f {a c} { set x 1; if {$a} { if {$c} { set x 2 } puts $x } else { puts no } }";
    assert!(
        !opt_fires(src, TCL, "O125"),
        "conditional redefine must suppress the sink"
    );
    assert_eq!(
        optimised(src, TCL),
        src,
        "declined sink leaves the source unchanged"
    );
}

// ===========================================================================
// chain_fold.rs — O130 list-build chain folding (the existing suites cover only
// O104 string chains; O130 is entirely untested there).
// ===========================================================================

#[test]
fn chain_fold_o130_list_build_chain() {
    // `set l {}; lappend l a; lappend l b c` folds to one `set l {a b c}`.
    // tclsh: both yield the list `a b c`.
    //   `set l {}; lappend l a; lappend l b c; set l` ⇒ a b c
    //   `set l {a b c}; set l`                        ⇒ a b c
    let chain = "set l {}\nlappend l a\nlappend l b c";
    assert_eq!(optimised(chain, TCL), "set l {a b c}");
    assert!(
        opt_fires(chain, TCL, "O130"),
        "list chain must fold via O130"
    );
    // Structural: one fold over the last write + a deletion per earlier write.
    assert_eq!(opt_count(chain, TCL, "O130"), 3);

    // A spacey element is re-quoted as a list word. tclsh:
    //   `set l {}; lappend l {a b}; lappend l c; set l` ⇒ {a b} c
    //   `set l {{a b} c}; set l`                        ⇒ {a b} c
    let spacey = "set l {}\nlappend l {a b}\nlappend l c";
    assert_eq!(optimised(spacey, TCL), "set l {{a b} c}");
    assert!(opt_fires(spacey, TCL, "O130"));

    // Inside a proc body. tclsh:
    //   `proc f {} { set l {}; lappend l a; lappend l b; return $l }; f` ⇒ a b
    //   `proc f {} { return {a b} }; f`                                  ⇒ a b
    let inproc = "proc ::f {} {\n    set l {}\n    lappend l a\n    lappend l b\n}";
    assert_eq!(optimised(inproc, TCL), "proc ::f {} {\n    set l {a b}\n}");
    assert!(opt_fires(inproc, TCL, "O130"));
}

// ===========================================================================
// chain_fold.rs — the precise-flow branch: a build chain folds *across* a
// static-literal write to a DIFFERENT variable (which cannot read/write the
// accumulator). `optimiser.rs` asserts the opposite (the optimiser "does not
// fold a write chain that straddles an intervening statement") — but that is
// only true for a *dynamic*/reading statement. An inert literal `set` to
// another var is the
// uncovered case where the chain genuinely continues.
// ===========================================================================

#[test]
fn chain_fold_o104_continues_past_interleaved_literal_set() {
    // `set s ""; set t 1; append s foo; append s bar` — the `set t 1` is inert,
    // so the string chain folds around it and the interleaved write stays put.
    // tclsh proves both `t` and `s`:
    //   `set s ""; set t 1; append s foo; append s bar; list $t $s` ⇒ 1 foobar
    //   `set t 1; set s foobar; list $t $s`                         ⇒ 1 foobar
    let src = "set s \"\"\nset t 1\nappend s foo\nappend s bar";
    assert_eq!(optimised(src, TCL), "set t 1\nset s foobar");
    assert!(
        opt_fires(src, TCL, "O104"),
        "chain must fold past an inert literal write"
    );
}

// ===========================================================================
// branch_folding.rs — the in-condition rewrite cascade
// (`propagate_into_branches` → `branch_cascade`: `strength_reduce` O113 →
// `strlen` O117 → `streq` O120 → `instcombine`). Drives O113 and O117 via
// `if` conditions, plus the De-Morgan-in-condition case the existing suites
// note is carried under O113 rather than O110.
// ===========================================================================

#[test]
fn branch_folding_condition_cascade() {
    // Strength reduction in a condition: `$x % 8` ⇒ `$x & 7`. tclsh sweep
    // proves equivalence for x ≥ 0:
    //   `foreach x {0 1 5 8 15 16 100} { if {($x % 8) != ($x & 7)} {…} }` ⇒ never differs
    let sred = "if {$x % 8} { puts a }";
    assert_eq!(optimised(sred, TCL), "if {$x & 7} { puts a }");
    assert!(
        opt_fires(sred, TCL, "O113"),
        "modulo strength-reduction in a condition is O113"
    );

    // De-Morgan inside a condition: `!($x && $y)` ⇒ `!$x || !$y`. The
    // if-condition rewrite is carried under O113 (the
    // strength-reduce pass owns the condition path), not O110. tclsh sweep:
    //   `foreach x {0 1 5} { foreach y {0 1 5} { if {(!($x && $y)) != (!$x || !$y)} {…} } }` ⇒ never differs
    let demorgan = "if {!($x && $y)} { puts yes }";
    assert!(optimised(demorgan, TCL).contains("!$x || !$y"));
    assert!(
        opt_fires(demorgan, TCL, "O110") || opt_fires(demorgan, TCL, "O113"),
        "De-Morgan in a condition must be carried by O110 or O113"
    );

    // strlen zero-check simplification fires on a non-parameter operand (a global
    // here): `[string length $::g] == 0` ⇒ `$::g eq ""`. tclsh sweep proves the
    // equivalence for empty and non-empty:
    //   `([string length $::g] == 0) == ($::g eq "")` ⇒ 1 for "" and for "abc"
    let strlen = "proc ::f {} { if {[string length $::g] == 0} { puts e } }";
    assert!(
        opt_fires(strlen, TCL, "O117"),
        "strlen zero-check in a condition is O117"
    );
    assert!(optimised(strlen, TCL).contains("$::g eq \"\""));
}

// ===========================================================================
// structure_elimination.rs — loop edge cases the existing suites under-cover:
// the `for {} {0} {}` all-empty form (whole loop deleted, trailing survives),
// and the `while {1}` infinite loop that must NOT be eliminated (only the
// constant condition folds; the loop body stays).
// ===========================================================================

#[test]
fn structure_elimination_loop_edges() {
    // `for {} {0} {} { … }` — empty init/next, false condition: the whole loop is
    // deleted, the trailing statement survives. tclsh: only `puts done` runs.
    let for_empty = "for {} {0} {} {\n    puts loop\n}\nputs done";
    assert!(opt_fires(for_empty, TCL, "O112"));
    let out = optimised(for_empty, TCL);
    assert!(
        !out.contains("puts loop"),
        "dead loop body must be removed: {out}"
    );
    assert!(
        out.contains("puts done"),
        "trailing statement must survive: {out}"
    );

    // `while {1} { break }` is an infinite (well, break-terminated) loop — O112
    // must NOT delete it. tclsh: it runs once and breaks (no observable value;
    // the structural requirement is that the loop survives).
    let while_one = "while {1} {\n    break\n}";
    assert!(
        !opt_fires(while_one, TCL, "O112"),
        "while {{1}} must not be O112-eliminated"
    );
    assert!(
        optimised(while_one, TCL).contains("while {1}"),
        "the loop must survive"
    );
}

// ===========================================================================
// tail_call.rs — O122 loop conversion driven from a `switch`-arm self-call.
// The existing suite lists this body under the O121/O122 detection set; here we
// pin the *applied* O122 loop rewrite (the switch-arm tail call becoming a
// `set tree …` inside a `while {1}`).
// ===========================================================================

#[test]
fn tail_call_o122_loop_conversion_from_switch_arm() {
    // A tail self-call in a `switch` arm converts to a `while {1}` loop, the
    // recursive call replaced by re-assigning the parameter. tclsh proves the
    // recursive and loop forms agree on a nested tree:
    //   recursive: `proc walk {tree} { switch [lindex $tree 0] { leaf { return [lindex $tree 1] } node { walk [lindex $tree 2] } } }`
    //   loop:      `proc walk {tree} { while {1} { switch … { leaf { return [lindex $tree 1] } node { set tree [lindex $tree 2] } } } }`
    //   `walk {node x {node y {leaf z 42}}}` ⇒ z  (both forms)
    let walk = "proc walk {tree} {\n    switch [lindex $tree 0] {\n        leaf {\n            return [lindex $tree 1]\n        }\n        node {\n            walk [lindex $tree 2]\n        }\n    }\n}\n";
    assert!(
        opt_fires(walk, TCL, "O121") || opt_fires(walk, TCL, "O122"),
        "switch-arm tail call must be detected"
    );
    let out = optimised(walk, TCL);
    assert!(
        out.contains("while {1}"),
        "O122 loop conversion expected: {out}"
    );
    // The recursive `walk [lindex $tree 2]` becomes a parameter re-assignment.
    assert!(
        out.contains("set tree [lindex $tree 2]"),
        "tail call must become a re-assignment: {out}"
    );
    assert!(
        !out.contains("walk [lindex $tree 2]"),
        "the recursive call must be gone: {out}"
    );
}

// ===========================================================================
// iRules cross-event O126/O109 — the connection-scope paths. A `set` in one
// `when EVENT {…}` handler whose value is consumed by a *later* event must
// survive (the `::when::*` lowering threads `connection_scope`'s
// cross_event_defs/imports into `cross_event_vars`, so the elimination pass
// skips them). Each survival case is paired with a control whose only
// difference is whether the later event reads the variable.
// ===========================================================================

#[test]
fn cross_event_direct_read_survives() {
    // `$start` in CLIENT_CLOSED reads the value set in CLIENT_ACCEPTED — the
    // store must not be deleted. tclsh (single-interp model): the later handler
    // logs the earlier value, so deleting `set start` leaves `$start` undefined.
    let read = "when CLIENT_ACCEPTED { set start [clock clicks] }\nwhen CLIENT_CLOSED { log local0. \"start=$start\" }";
    assert!(
        !opt_fires(read, IR, "O126"),
        "cross-event store read later must not be O126"
    );
    assert!(
        !opt_fires(read, IR, "O109"),
        "cross-event store read later must not be O109"
    );
    assert!(
        optimised(read, IR).contains("set start"),
        "cross-event store must survive"
    );

    // Control: the later event does NOT read `start`, so within the single
    // CLIENT_ACCEPTED handler it is a genuinely unused store ⇒ O126 fires.
    let control = "when CLIENT_ACCEPTED { set start [clock clicks] }\nwhen CLIENT_CLOSED { log local0. done }";
    assert!(
        opt_fires(control, IR, "O126"),
        "control (value never read) must fire O126"
    );
    assert!(
        !optimised(control, IR).contains("set start"),
        "the genuinely-unused store is removed"
    );
}

#[test]
fn cross_event_info_exists_flag_survives_and_is_not_folded() {
    // `[info exists seen]` in HTTP_RESPONSE observes a flag set in HTTP_REQUEST:
    // neither the store nor the existence check may be folded away (else the
    // response takes the wrong branch). tclsh: with `seen` set, `info exists`
    // is true — `set seen 1; if {[info exists seen]} {…}` takes the `has` branch.
    let src = "when HTTP_REQUEST { set seen 1 }\nwhen HTTP_RESPONSE { if {[info exists seen]} { log local0. yes } }";
    assert!(
        !opt_fires(src, IR, "O126"),
        "cross-event flag store must survive (no O126)"
    );
    let out = optimised(src, IR);
    assert!(out.contains("set seen 1"), "flag store must survive: {out}");
    assert!(
        out.contains("info exists seen"),
        "info-exists must NOT be folded to a constant: {out}"
    );
}

#[test]
fn cross_event_same_event_dead_store_still_eliminated() {
    // Control for the cross-event guard: a store *overwritten within the same
    // event* is genuinely dead and IS eliminated (O109). tclsh: `set t 1; set t
    // 2; … $t` observes 2, so dropping `set t 1` is sound.
    let src = "when HTTP_REQUEST { set t 1\n set t 2\n log local0. $t }";
    assert!(
        opt_fires(src, IR, "O109"),
        "same-event overwritten store must fire O109"
    );
    assert!(
        !optimised(src, IR).contains("set t 1"),
        "the dead same-event store is removed"
    );
}

// ===========================================================================
// Structure elimination of a nested switch through the multipass fixpoint —
// the first pass unwraps the outer `if {1}`, a later pass eliminates the inner
// dead `switch` whose subject matches no arm and has no default.
// ===========================================================================

#[test]
fn structure_elimination_nested_switch_via_multipass() {
    // `if {1} { switch xyz { abc { set dead 1 } }; set alive 2 }`. tclsh: the
    // switch matches nothing (no default), so only `set alive 2` ever runs.
    //   `if {1} { switch xyz { abc { set dead 1 } }; set alive 2 }; info exists dead` ⇒ 0
    //   `set alive 2; info exists dead`                                                ⇒ 0
    let nested =
        "if {1} {\n    switch xyz {\n        abc { set dead 1 }\n    }\n    set alive 2\n}";
    // One pass already removes the dead switch arm / unwraps the if (structural).
    assert!(opt_count(nested, TCL, "O112") >= 1);
    let registry = registry_for_dialect(TCL);
    let (fixed, _) = optimise_source_multipass(
        nested,
        registry,
        Some(tcl_dialect::DialectProfile::by_name(TCL)),
        10,
    );
    assert!(
        !fixed.contains("set dead 1"),
        "dead switch arm must be eliminated: {fixed}"
    );
    assert!(
        fixed.contains("set alive 2"),
        "live statement must survive: {fixed}"
    );
}
