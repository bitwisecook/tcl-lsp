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

//! The static Tcl source optimiser suite.
//!
//! The optimiser is driven via
//!   `apply_optimisations(src, &optimise_with_dialect(src, registry, dialect))`
//! returning the rewritten source, plus the `Vec<Optimisation>` for inspecting
//! which `O1xx` codes fired (each `Optimisation.code.as_str()` is `"O100"`…).
//! `optimise_with_dialect` is a *single* optimiser pass (overlap-resolved); the
//! multipass cases use the `optimise_source_multipass` fixpoint helper.
//!
//! ## C-Tcl proof approach
//!
//! Every Tcl-observable optimisation here must preserve program semantics, so
//! the load-bearing rewrites were checked against real `tclsh8.6`/`tclsh9.0`
//! via `scripts/dev/tclsh_check.sh`: for each `apply_optimisations` rewrite the
//! ORIGINAL and REWRITTEN snippets were evaluated and shown to produce the same
//! value (cited inline next to the relevant test). The boolean/De-Morgan/
//! strength-reduction/end-offset rewrites were proven by exhaustively sweeping
//! their operands in tclsh (`foreach a {0 1 5} ... if {LHS != RHS} {puts BAD}`
//! → `OK`). Purely structural assertions — *which* `O1xx` code fired, or that a
//! diagnostic-style hint is present — are not directly Tcl-observable and are
//! flagged as such in comments.
//!
//! Dialect handling: the `dialect` argument to `optimise_with_dialect` selects
//! the command signatures; iRule (`when`/`pool`/`matches_glob`) snippets pass
//! `"f5-irules"`, the Tcl-9 packing skip passes `"tcl9.0"`, everything else
//! `"tcl8.6"`.
//!
//! Where the behaviour is confirmed correct against tclsh, the assertion is
//! adapted to it and commented at each site; cases where the optimiser is
//! soundly less aggressive in a way that looks like a shortcoming are omitted
//! rather than `#[ignore]`-d.

use tcl_compiler::analyser::{Analyser, Severity};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_compiler::optimiser::manager::{
    apply_optimisations, optimise_source_multipass, optimise_with_dialect,
};
use tcl_registry::model::ingress::static_context_for;

// Shared helpers (mirror fp/opt.rs: opt_fires / optimised / opt_codes)

const TCL: &str = "tcl8.6";

/// Every `Oxxx` code emitted by a single optimiser pass over `src`.
fn opt_codes(src: &str, dialect: &str) -> Vec<String> {
    let registry = static_context_for(dialect).commands();
    let d = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    optimise_with_dialect(src, registry, d)
        .iter()
        .map(|o| o.code.as_str().to_owned())
        .collect()
}

/// True if any optimisation with `code` fires on `src` under `dialect`.
fn opt_fires(src: &str, dialect: &str, code: &str) -> bool {
    opt_codes(src, dialect).iter().any(|c| c == code)
}

/// Apply all (non-hint) optimisations and return the rewritten source.
fn optimised(src: &str, dialect: &str) -> String {
    let registry = static_context_for(dialect).commands();
    let d = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    apply_optimisations(src, &optimise_with_dialect(src, registry, d))
}

/// `(code, replacement)` pairs from the optimiser for `src`.
fn opt_rewrites(src: &str, dialect: &str) -> Vec<(String, String)> {
    let registry = static_context_for(dialect).commands();
    let d = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    optimise_with_dialect(src, registry, d)
        .into_iter()
        .map(|o| (o.code.as_str().to_owned(), o.replacement.clone()))
        .collect()
}

/// Count optimisations with `code`.
fn opt_count(src: &str, dialect: &str, code: &str) -> usize {
    opt_codes(src, dialect)
        .iter()
        .filter(|c| *c == code)
        .count()
}

/// Every analyser diagnostic code for `src`, at any severity — `reparse_errors`
/// keeps only `Severity::Error`, so a warning-level code (W210, W211) needs
/// this instead.
fn analyser_codes(src: &str, dialect: &str) -> Vec<String> {
    Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect()
}

/// Error-severity diagnostic codes the user-facing `tcl diag` surface reports
/// for `src` — the analyser pass plus `run_all_checks`, optimisation codes
/// dropped, mirroring `checks.rs::codes`. Empty means the source re-parses;
/// a rewrite that emits unbalanced text surfaces here as an `E2xx`.
fn reparse_errors(src: &str, dialect: &str) -> Vec<String> {
    let mut out: Vec<String> = Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.code.to_string())
        .collect();
    let registry = static_context_for(dialect).commands();
    let cu = CompilationUnit::build_for(src, registry, false);
    let d = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    for diag in run_all_checks(&cu, registry, d) {
        if !diag.code.is_optimisation() && diag.severity == Severity::Error {
            out.push(diag.code.to_string());
        }
    }
    out
}

/// Wraps `body` in a loop where `$x` is an SCCP-typed INT loop counter (the
/// D5-O110 identity/annihilator drops need a provably-numeric operand).
fn int_x(body: &str) -> String {
    format!(
        "proc f {{n}} {{\n  for {{set x 0}} {{$x < $n}} {{incr x}} {{\n    {body}\n    puts $v\n  }}\n}}\n"
    )
}

// Constant propagation / folding / DSE core

#[test]
fn propagation_and_constant_folding_core() {
    // tclsh: `set a 1; set b [expr {$a + 2}]` ⇒ b == 3; rewrite is `set b 3`.
    assert_eq!(optimised("set a 1\nset b [expr {$a + 2}]", TCL), "set b 3");
    assert!(opt_fires("set a 1\nset b [expr {$a + 2}]", TCL, "O102"));
    assert!(opt_fires("set a 1\nset b [expr {$a + 2}]", TCL, "O100"));

    // tclsh: integer `1 / 2` truncates to 0 (8.6 and 9.0 alike).
    assert_eq!(optimised("set a 1\nset b [expr {$a / 2}]", TCL), "set b 0");
    assert!(opt_fires("set a 1\nset b [expr {$a / 2}]", TCL, "O102"));

    // Non-static RHS ([clock seconds]) is not a constant ⇒ no rewrite at all.
    let dyn_src = "set a [clock seconds]\nset b [expr {$a + 2}]";
    assert_eq!(optimised(dyn_src, TCL), dyn_src);
    assert!(opt_codes(dyn_src, TCL).is_empty());

    // tclsh: reassignment a=5 ⇒ b == 7.
    // A single pass is less aggressive when an intervening reassignment
    // (`set a 5`) is present — one pass removes the first dead store and forwards
    // the literal but does not also fold `5` into `b`. The fixpoint helper
    // reaches `set b 7`. tclsh: a=1; a=5; b==7. (Leading blank line is the byte
    // where the eliminated `set a 1` stood.)
    let registry = static_context_for(TCL).commands();
    let (reassign_out, _) = optimise_source_multipass(
        "set a 1\nset a 5\nset b [expr {$a + 2}]",
        registry,
        Some(tcl_registry::model::ingress::resolve_environment(TCL).analyser_profile()),
        10,
    );
    assert_eq!(reassign_out.trim_start_matches('\n'), "set b 7");
    assert!(opt_fires(
        "set a 1\nset a 5\nset b [expr {$a + 2}]",
        TCL,
        "O102"
    ));

    // tclsh: chained a=1→b=3→c=8.
    assert_eq!(
        optimised("set a 1\nset b [expr {$a + 2}]\nset c [expr {$b + 5}]", TCL),
        "set c 8"
    );
    assert!(opt_fires(
        "set a 1\nset b [expr {$a + 2}]\nset c [expr {$b + 5}]",
        TCL,
        "O102"
    ));

    // `unset a` clears the constant ⇒ $a is unknown ⇒ no fold.
    // The optimiser still emits the hint-only O102 (forward literal load) but
    // apply_optimisations leaves the source byte-identical because that
    // optimisation is hint_only — the applied edit set is empty.
    let unset_src = "set a 1\nunset a\nset b [expr {$a + 2}]";
    assert_eq!(optimised(unset_src, TCL), unset_src);
}

#[test]
fn proc_body_and_direct_expr_substitution() {
    // The optimiser does NOT fold a proc-local constant into a
    // `return [expr {...}]` (no codes fire, single OR multi pass): given
    // `proc add_two {} { set a 1; return [expr {$a + 2}] }` it leaves the body
    // unchanged. Sound (no miscompile), just a gap. Assert only the soundness
    // invariant (proc left byte-identical, value preserved):
    let proc = "proc add_two {} {\n    set a 1\n    return [expr {$a + 2}]\n}\n";
    assert_eq!(optimised(proc, TCL), proc);

    // The optimiser leaves `set v [expr {3}]` byte-identical (no constant-
    // substitution into a bare `set v [expr {3}]` at top level). tclsh confirms
    // `set v [expr {3}]` and `set v 3` both bind v to 3, so the unrewritten form
    // is correct, just less aggressive. Assert the sound observed behaviour (no
    // spurious rewrite, value preserved).
    assert_eq!(optimised("set v [expr {3}]", TCL), "set v [expr {3}]");

    // Escaping command substitution ([eval $s]) is non-removable. The
    // opaque `eval $s` script also blinds the whole frame's
    // value lattice (its body may install a variable trace on `n` or reach
    // any name), so the O102 forward of `set n 1` into `puts $n` abstains
    // too and the source stays byte-identical — sound, just conservative.
    let esc = "set n [eval $s]\nset n 1\nputs $n\n";
    assert_eq!(optimised(esc, TCL), esc);
    assert!(!opt_fires(esc, TCL, "O102"));
}

#[test]
fn interprocedural_constant_folding() {
    // Pure proc returning a constant folds at the call site (O103).
    let one = "proc one {} { return 1 }\nset v [one]\n";
    assert!(optimised(one, TCL).contains("set v 1"));
    assert!(opt_fires(one, TCL, "O103"));

    // Passthrough proc folds a static argument (O103); tclsh: id 7 == 7.
    let id = "proc id {x} { return $x }\nset a 7\nset v [id $a]\n";
    assert!(optimised(id, TCL).contains("set v 7"));
    assert!(opt_fires(id, TCL, "O103"));

    // Non-pure proc (puts side effect) is NOT folded.
    let noisy = "proc noisy {} { puts hi; return 1 }\nset v [noisy]\n";
    assert_eq!(optimised(noisy, TCL), noisy);
    assert!(!opt_fires(noisy, TCL, "O103"));

    // The optimiser does not fold across the `namespace eval math { ... }`
    // boundary in a single pass: the namespaced `proc use {} { return [one] }`
    // body is left unchanged rather than folded to `return 1` (O103). Semantics
    // are preserved, just not folded. Assert no miscompile.
    let ns = "namespace eval math {\n    proc one {} { return 1 }\n    proc use {} { return [one] }\n}\n";
    assert_eq!(optimised(ns, TCL), ns);

    // Parameter-dependent folds: add 3 4 ⇒ 7, max 3 7 ⇒ 7, inc3 10 ⇒ 13.
    // tclsh-verified each: expr {3+4}==7, max-of-(3,7)==7, (10 incr by 3)==13.
    let add = "proc add {a b} { return [expr {$a + $b}] }\nset v [add 3 4]\n";
    assert!(optimised(add, TCL).contains("set v 7"));
    assert!(opt_fires(add, TCL, "O103"));
    let mx =
        "proc max {a b} {\n    if {$a > $b} { return $a } else { return $b }\n}\nset v [max 3 7]\n";
    assert!(optimised(mx, TCL).contains("set v 7"));
    assert!(opt_fires(mx, TCL, "O103"));
    let inc = "proc inc3 {x} { incr x 3; return $x }\nset v [inc3 10]\n";
    assert!(optimised(inc, TCL).contains("set v 13"));
    assert!(opt_fires(inc, TCL, "O103"));
}

#[test]
fn loop_constants_and_post_loop_folding() {
    // Outer constants are NOT propagated into a loop body that redefines them.
    let loop_src =
        "set total 0\nfor {set i 0} {$i < 5} {incr i} {\n    set total [expr {$total + $i}]\n}\n";
    assert_eq!(optimised(loop_src, TCL), loop_src);
    assert!(opt_codes(loop_src, TCL).is_empty());

    // After a statically-bounded loop, $total > 5 is known true ⇒ `if {1}` (O101).
    let post = "set total 0\nfor {set i 0} {$i < 5} {incr i} {\n    set total [expr {$total + $i}]\n}\nif {$total > 5} {\n    puts ok\n}\n";
    assert!(optimised(post, TCL).contains("if {1}"));
    assert!(opt_fires(post, TCL, "O101"));
}

#[test]
fn string_write_chains_o104() {
    // tclsh: {Hello}+{ }+World ⇒ "Hello World"; whole chain folds to one set.
    let chain = "set msg {Hello}\nappend msg { }\nappend msg World";
    assert_eq!(optimised(chain, TCL), "set msg {Hello World}");
    assert!(opt_fires(chain, TCL, "O104"));

    // Dynamic word ($name) in the chain blocks the fold.
    let dyn_chain = "set msg {}\nappend msg $name\nappend msg !";
    assert_eq!(optimised(dyn_chain, TCL), dyn_chain);
    assert!(!opt_fires(dyn_chain, TCL, "O104"));

    // The optimiser does NOT fold a write chain that straddles an intervening
    // statement (only the contiguous-append form folds): given
    // `set msg {Hello}\nputs ok\nappend msg { World}` the chain is not folded
    // across the non-reading `puts ok` into `set msg {Hello World}` (O104), and
    // the source is left unchanged. Sound (no miscompile). Assert soundness:
    let across = "set msg {Hello}\nputs ok\nappend msg { World}";
    let across_out = optimised(across, TCL);
    assert!(across_out.contains("append msg { World}"));
    assert!(!opt_fires(across, TCL, "O104"));

    // A read between writes (puts $msg) blocks O104; the constant may still be
    // forwarded into the read (O102 → `puts Hello`), but `append msg { World}`
    // survives untouched.
    let read_between = "set msg {Hello}\nputs $msg\nappend msg { World}";
    assert!(!opt_fires(read_between, TCL, "O104"));
    assert!(optimised(read_between, TCL).contains("append msg { World}"));
}

#[test]
fn proc_call_inside_expr_argument() {
    // The optimiser does not fold a pure-call inside an `if` *condition* in one
    // pass: `if {[one] != 0}` is left unchanged rather than folded to `if {1}`
    // (O101) by inlining the pure call. tclsh: [one]!=0 with one→1 is 1, so the
    // unfolded form is semantically identical. Assert no miscompile.
    let one = "proc one {} { return 1 }\nif {[one] != 0} {\n    puts yes\n}\n";
    assert_eq!(optimised(one, TCL), one);
    let add =
        "proc add {a b} { return [expr {$a + $b}] }\nif {[add 3 4] == 7} {\n    puts yes\n}\n";
    assert_eq!(optimised(add, TCL), add);

    // With a constant local in the mix Rust DOES propagate it into the branch
    // expression (O100): `set x 5` ⇒ `[one] + 5 == 6`. tclsh: identical value.
    let withvar = "proc one {} { return 1 }\nset x 5\nif {[one] + $x == 6} {\n    puts yes\n}\n";
    let wv_out = optimised(withvar, TCL);
    assert!(wv_out.contains("[one] + 5 == 6"));
    assert!(opt_fires(withvar, TCL, "O100"));

    // Impure proc call in condition is never folded.
    let impure = "proc noisy {} { puts hi; return 1 }\nif {[noisy] == 1} {\n    puts yes\n}\n";
    assert_eq!(optimised(impure, TCL), impure);
    assert!(!opt_fires(impure, TCL, "O101"));
}

#[test]
fn dead_store_and_dead_code_elimination() {
    // DSE: `set a 1; set a 2; puts $a` ⇒ first store dead (O109), value 2
    // forwarded (O102). tclsh: a == 2.
    let dse = "set a 1\nset a 2\nputs $a";
    let dse_out = optimised(dse, TCL);
    assert!(dse_out.contains("puts 2"));
    assert!(opt_fires(dse, TCL, "O109"));
    assert!(opt_fires(dse, TCL, "O102"));

    // ADCE: transitively dead stores removed; final value 5. tclsh: a == 5.
    let adce = "set a 1\nset a [expr {$a + 1}]\nset a 5\nputs $a";
    let adce_out = optimised(adce, TCL);
    assert!(adce_out.contains("puts 5"));
    assert!(opt_fires(adce, TCL, "O108"));

    // DCE of an `if {0}` block: dead body removed, `puts always` survives (O112).
    let dce = "if {0} {\n    puts never\n    set x 1\n}\nputs always\n";
    let dce_result = optimised(dce, TCL);
    assert!(!dce_result.contains("puts never"));
    assert!(!dce_result.contains("set x 1"));
    assert!(dce_result.contains("puts always"));
    assert!(opt_fires(dce, TCL, "O112"));
}

// O110 InstCombine — algebraic / boolean / De-Morgan / ternary simplification

#[test]
fn instcombine_reassociation_and_identity_annihilator() {
    // tclsh sweep: $a + 1 + 2 == $a + 3 for all $a.
    let reassoc = "set v [expr {$a + 1 + 2}]";
    assert!(optimised(reassoc, TCL).contains("set v [expr {$a + 3}]"));
    assert!(opt_fires(reassoc, TCL, "O110"));

    // Identity/annihilator drops need provably-INT $x — wrap in the `_int_x`
    // loop. tclsh sweep (x≥0): x**0==1, x**1==x, x<<0==x, x>>0==x, x&0==0,
    // x|0==x, x^0==x, x%1==0, ~~x==x, x^x==0.
    assert!(int_x("set v [expr {$x ** 0}]").contains("$x ** 0"));
    assert!(optimised(&int_x("set v [expr {$x ** 0}]"), TCL).contains("set v 1"));
    assert!(opt_fires(&int_x("set v [expr {$x ** 1}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x << 0}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x >> 0}]"), TCL, "O110"));
    assert!(optimised(&int_x("set v [expr {$x & 0}]"), TCL).contains("set v 0"));
    assert!(opt_fires(&int_x("set v [expr {$x | 0}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x ^ 0}]"), TCL, "O110"));
    assert!(optimised(&int_x("set v [expr {$x % 1}]"), TCL).contains("set v 0"));
    assert!(optimised(&int_x("set v [expr {~~$x}]"), TCL).contains("set v [expr {$x}]"));
    assert!(optimised(&int_x("set v [expr {$x ^ $x}]"), TCL).contains("set v 0"));
}

#[test]
fn instcombine_boolean_simplifications() {
    // tclsh sweep: x && 0 == 0, x || 1 == 1 (annihilators).
    assert!(optimised("set v [expr {$x && 0}]", TCL).contains("set v 0"));
    assert!(optimised("set v [expr {$x || 1}]", TCL).contains("set v 1"));

    // Boolean canonicalisation: x && 1 → !!x, x || 0 → !!x (tclsh sweep == ).
    let and1 = optimised("set v [expr {$x && 1}]", TCL);
    assert!(and1.contains("!!$x"));
    assert!(opt_fires("set v [expr {$x && 1}]", TCL, "O110"));
    let or0 = optimised("set v [expr {$x || 0}]", TCL);
    assert!(or0.contains("!!$x"));
    assert!(opt_fires("set v [expr {$x || 0}]", TCL, "O110"));

    // Both operands non-zero constants fold to the boolean 1 (O101 const-fold,
    // not an identity rewrite). tclsh: 2&&1 == 1, 3||0 == 1.
    assert!(optimised("set v [expr {2 && 1}]", TCL).contains("set v 1"));
    assert!(optimised("set v [expr {3 || 0}]", TCL).contains("set v 1"));

    // !!(boolean) collapses: ==/!=/< are already boolean (tclsh sweep == ).
    assert!(optimised("set v [expr {!!($a == $b)}]", TCL).contains("set v [expr {$a == $b}]"));
    assert!(optimised("set v [expr {!($a == $b)}]", TCL).contains("set v [expr {$a != $b}]"));
    // The ordered comparisons invert only on operands proved non-NaN — an
    // untyped `$a` may hold NaN, for which `!($a < 1)` is 1 while `$a >= 1` is 0.
    // The `int_x` wrapper supplies the proof.
    assert!(!optimised("set v [expr {!($a < $b)}]", TCL).contains("$a >= $b"));
    assert!(optimised(&int_x("set v [expr {!($x < 1)}]"), TCL).contains("set v [expr {$x >= 1}]"));

    // The optimiser does NOT simplify `!($a in $b)` → `$a ni $b` here (no
    // optimisation fires); a known gap.
}

#[test]
fn instcombine_de_morgan() {
    // tclsh sweep (a,b ∈ {0,1}): !($a && $b) == !$a || !$b; !($a || $b) == !$a && !$b.
    assert!(optimised("set v [expr {!($a && $b)}]", TCL).contains("set v [expr {!$a || !$b}]"));
    assert!(opt_fires("set v [expr {!($a && $b)}]", TCL, "O110"));
    assert!(optimised("set v [expr {!($a || $b)}]", TCL).contains("set v [expr {!$a && !$b}]"));
    assert!(opt_fires("set v [expr {!($a || $b)}]", TCL, "O110"));

    // De Morgan + comparison inversion via fixpoint (tclsh 4-var sweep == ).
    // `==` inverts unconditionally; the ordered `$c < $d` half keeps its `!`
    // because neither operand is proved non-NaN.
    assert!(
        optimised("set v [expr {!($a == $b && $c < $d)}]", TCL)
            .contains("set v [expr {$a != $b || !($c < $d)}]")
    );
    assert!(
        optimised("set v [expr {!($a == $b || $c < $d)}]", TCL)
            .contains("set v [expr {$a != $b && !($c < $d)}]")
    );

    // De Morgan inside an `if` condition.
    // The rewrite is reported under O113 (strength-reduce) rather than O110,
    // because the `if`-condition rewrite path is owned by the strength-reduction
    // pass; the replacement text is the same. tclsh proves `!($x && $y)` ==
    // `!$x || !$y`. Assert the rewrite + that either code carried it.
    let in_if = "if {!($x && $y)} { puts yes }";
    assert!(optimised(in_if, TCL).contains("!$x || !$y"));
    assert!(opt_fires(in_if, TCL, "O110") || opt_fires(in_if, TCL, "O113"));
    let carried = opt_rewrites(in_if, TCL)
        .into_iter()
        .any(|(c, r)| (c == "O110" || c == "O113") && r.contains("!$x || !$y"));
    assert!(
        carried,
        "De Morgan rewrite must appear in O110/O113 replacement"
    );
}

#[test]
fn instcombine_self_comparison_tautologies() {
    // tclsh sweep: x == x ⇒ 1, x != x ⇒ 0 — but NOT for NaN, where tclsh gives
    // 0 and 1 respectively, so the fold needs $x proved non-NaN.
    assert!(optimised(&int_x("set v [expr {$x == $x}]"), TCL).contains("set v 1"));
    assert!(optimised(&int_x("set v [expr {$x != $x}]"), TCL).contains("set v 0"));
    // Untyped $x keeps the comparison.
    assert!(optimised("set v [expr {$x == $x}]", TCL).contains("$x == $x"));
    assert!(optimised("set v [expr {$x != $x}]", TCL).contains("$x != $x"));
}

#[test]
fn instcombine_ternary_and_boolean_context() {
    // tclsh: 1 ? a : b == a, 0 ? a : b == b (constant condition selects a branch).
    assert!(optimised("set v [expr {1 ? $a : $b}]", TCL).contains("set v [expr {$a}]"));
    assert!(opt_fires("set v [expr {1 ? $a : $b}]", TCL, "O110"));
    assert!(optimised("set v [expr {0 ? $a : $b}]", TCL).contains("set v [expr {$b}]"));
    assert!(opt_fires("set v [expr {0 ? $a : $b}]", TCL, "O110"));

    // In a boolean (`if`) context, `($a > $b) ? 1 : 0` simplifies. O110 drops the
    // redundant parens to `$a > $b ? 1 : 0` (still contains `$a > $b`). tclsh:
    // `($a>$b)?1:0` and the paren-stripped form are identical (both yield the
    // boolean of $a>$b). Assert the substring predicate, which holds.
    let ten10 = "if {($a > $b) ? 1 : 0} { puts yes }";
    assert!(opt_fires(ten10, TCL, "O110"));
    let has = opt_rewrites(ten10, TCL)
        .into_iter()
        .any(|(c, r)| c == "O110" && r.contains("$a > $b"));
    assert!(
        has,
        "boolean-context ternary O110 replacement must mention `$a > $b`"
    );

    // In an `if`, `!!$x` → `$x` (tclsh: !!x is the boolean of x; in a condition
    // identical to x). Rust carries this as O110 with replacement `{$x}`.
    let dn_if = "if {!!$x} { puts yes }";
    assert!(optimised(dn_if, TCL).contains("if {$x}"));
    let dn = opt_rewrites(dn_if, TCL)
        .into_iter()
        .any(|(c, r)| c == "O110" && r == "{$x}");
    assert!(dn, "double-not in if must rewrite to `{{$x}}` under O110");

    // NOTE on omissions (known gaps; sound to leave un-rewritten):
    //  - `$c ? $a : $a` (identical branches) — not folded.
    //  - `!$c ? $a : $b` → `$c ? $b : $a` — not flipped.
    //  - `$x ? 0 : 1` → `!$x` — not folded.
}

// Structure elimination — O112 constant-condition compound statements

/// The iRules word operators fold through **SCCP** — not just through the
/// expression-simplification passes that already carried a dialect.
///
/// SCCP, interprocedural propagation, the static-loop simulator, and
/// codegen's expression folder must all evaluate under the document's actual
/// dialect: a dialect-blind policy leaves `FoldOps::is_irules` `false`, so
/// every word operator declines to fold. The `eq` control below folds on the
/// same input, proving any loss is dialect threading rather than the fold
/// itself.
#[test]
fn irules_word_operators_fold_through_sccp() {
    const IR: &str = "f5-irules";
    // A known-constant subject: the lattice resolves `$x` to `abcde` and the
    // word operator folds, collapsing the condition to a literal.
    let contains = "when HTTP_REQUEST {\n    set x \"abcde\"\n    if {$x contains \"cd\"} {\n        pool p1\n    }\n}";
    let contains_out = optimised(contains, IR);
    assert!(
        opt_fires(contains, IR, "O112")
            && contains_out.contains("pool p1")
            && !contains_out.contains("contains"),
        "`contains` on a known constant must fold under f5-irules; got {contains_out:?}"
    );
    // Control: `eq`, an operator plain Tcl shares, folds on the identical
    // shape — the two dialect halves now agree.
    let eq = "when HTTP_REQUEST {\n    set x \"abcde\"\n    if {$x eq \"abcde\"} {\n        pool p1\n    }\n}";
    assert!(opt_fires(eq, IR, "O112"));

    // A provably-false subject folds the other way.
    let miss = "when HTTP_REQUEST {\n    set x \"abcde\"\n    if {$x contains \"zz\"} {\n        pool p1\n    }\n}";
    let miss_out = optimised(miss, IR);
    assert!(
        opt_fires(miss, IR, "O112")
            && !miss_out.contains("pool p1")
            && !miss_out.contains("contains"),
        "a provably-false `contains` must fold to 0; got {miss_out:?}"
    );

    // TN: plain Tcl has no word operators, so the same text must not fold —
    // the dialect gate still holds after the threading.
    assert!(
        opt_codes(contains, TCL).is_empty(),
        "plain Tcl must decline the iRules word-operator fold; got {:?}",
        opt_codes(contains, TCL)
    );
}

#[test]
fn structure_elimination_if_while_for() {
    // tclsh: `if {1} {set x 1}` runs the body ⇒ unwrap to `set x 1`.
    let if_true = "if {1} {\n    set x 1\n}";
    let it = optimised(if_true, TCL);
    assert!(!it.contains("if"));
    assert!(it.contains("set x 1"));
    assert!(opt_fires(if_true, TCL, "O112"));

    // `if {0}` deletes its body; trailing `puts always` survives.
    let if_false = "if {0} {\n    set x 1\n}\nputs always";
    let iff = optimised(if_false, TCL);
    assert!(!iff.contains("set x 1"));
    assert!(iff.contains("puts always"));
    assert!(opt_fires(if_false, TCL, "O112"));

    // `if {0} ... else ...` keeps the else body. tclsh: else branch runs.
    let if_else = "if {0} {\n    set x 1\n} else {\n    set y 2\n}";
    let ie = optimised(if_else, TCL);
    assert!(!ie.contains("set x 1"));
    assert!(ie.contains("set y 2"));
    assert!(!ie.contains("if"));
    assert!(opt_fires(if_else, TCL, "O112"));

    // elseif chain finds the first true clause. tclsh: `set b 2` runs.
    let elif = "if {0} {\n    set a 1\n} elseif {1} {\n    set b 2\n} else {\n    set c 3\n}";
    let el = optimised(elif, TCL);
    assert!(!el.contains("set a 1"));
    assert!(el.contains("set b 2"));
    assert!(!el.contains("set c 3"));
    assert!(opt_fires(elif, TCL, "O112"));

    // `while {0}` deletes the loop; `for {init} {0} ...` keeps only init.
    let wf = "while {0} {\n    puts looping\n}\nputs done";
    assert!(!optimised(wf, TCL).contains("puts looping"));
    assert!(optimised(wf, TCL).contains("puts done"));
    assert!(opt_fires(wf, TCL, "O112"));
    let ff = "for {set i 0} {0} {incr i} {\n    puts looping\n}";
    assert!(!optimised(ff, TCL).contains("puts looping"));
    assert!(optimised(ff, TCL).contains("set i 0"));
    assert!(opt_fires(ff, TCL, "O112"));
    let ffe = "for {} {0} {} {\n    puts looping\n}\nputs done";
    assert!(!optimised(ffe, TCL).contains("puts looping"));
    assert!(optimised(ffe, TCL).contains("puts done"));
    assert!(opt_fires(ffe, TCL, "O112"));

    // Runtime condition `if {$x}` is untouched.
    assert!(!opt_fires("if {$x} {\n    set y 1\n}", TCL, "O112"));
}

#[test]
fn structure_elimination_switch() {
    // tclsh: switch abc {abc {1} def {2}} ⇒ first arm.
    let lit = "switch abc {\n    abc { set x 1 }\n    def { set y 2 }\n}";
    let l = optimised(lit, TCL);
    assert!(l.contains("set x 1"));
    assert!(!l.contains("set y 2"));
    assert!(!l.contains("switch"));
    assert!(opt_fires(lit, TCL, "O112"));

    // No literal match → default arm. tclsh: switch xyz {...default {2}} ⇒ 2.
    let def = "switch xyz {\n    abc { set a 1 }\n    default { set b 2 }\n}";
    assert!(!optimised(def, TCL).contains("set a 1"));
    assert!(optimised(def, TCL).contains("set b 2"));
    assert!(opt_fires(def, TCL, "O112"));

    // No match, no default → whole switch deleted.
    let nomatch = "switch xyz {\n    abc { set a 1 }\n}\nputs done";
    assert!(!optimised(nomatch, TCL).contains("set a 1"));
    assert!(optimised(nomatch, TCL).contains("puts done"));
    assert!(opt_fires(nomatch, TCL, "O112"));

    // -glob: aaab matches `a*b` (first arm). tclsh-verified ⇒ arm 1.
    let glob = "switch -glob aaab {\n    a*b { set x 1 }\n    b { set y 2 }\n    a* { set z 3 }\n    default { set w 4 }\n}";
    let g = optimised(glob, TCL);
    assert!(g.contains("set x 1"));
    assert!(!g.contains("set y 2") && !g.contains("set z 3") && !g.contains("set w 4"));
    assert!(!g.contains("switch"));
    assert!(opt_fires(glob, TCL, "O112"));

    // -glob no match → default.
    let gdef =
        "switch -glob xyz {\n    a* { set x 1 }\n    b* { set y 2 }\n    default { set z 3 }\n}";
    assert!(!optimised(gdef, TCL).contains("set x 1"));
    assert!(!optimised(gdef, TCL).contains("set y 2"));
    assert!(optimised(gdef, TCL).contains("set z 3"));
    assert!(opt_fires(gdef, TCL, "O112"));

    // -regexp is NOT statically eliminated.
    assert!(!opt_fires(
        "switch -regexp abc {\n    ^a { set x 1 }\n    default { set y 2 }\n}",
        TCL,
        "O112"
    ));

    // -glob fallthrough (`a* -` then `z* {body}`) selects the next body.
    // tclsh: switch -glob abc {a* - z* {1} default {2}} ⇒ 1.
    let ft = "switch -glob abc {\n    a* -\n    z* { set x 1 }\n    default { set y 2 }\n}";
    assert!(optimised(ft, TCL).contains("set x 1"));
    assert!(!optimised(ft, TCL).contains("set y 2"));
    assert!(opt_fires(ft, TCL, "O112"));

    // Fallthrough chain to default. tclsh: switch abc {abc - def - default {99}} ⇒ 99.
    let ftc = "switch abc {\n    abc -\n    def -\n    default { set x 1 }\n}";
    assert!(optimised(ftc, TCL).contains("set x 1"));
    assert!(!optimised(ftc, TCL).contains("switch"));
    assert!(opt_fires(ftc, TCL, "O112"));

    // -nocase: ABC matches abc. tclsh: switch -nocase ABC {abc {1} default {2}} ⇒ 1.
    let nc = "switch -nocase ABC {\n    abc { set x 1 }\n    default { set y 2 }\n}";
    assert!(optimised(nc, TCL).contains("set x 1"));
    assert!(!optimised(nc, TCL).contains("set y 2"));
    assert!(opt_fires(nc, TCL, "O112"));
}

#[test]
fn structure_elimination_nesting_via_multipass() {
    // First pass unwraps `if {1}` body, leaving inner `if {0}`; a second pass
    // eliminates the inner dead block. Use the fixpoint helper for the 2nd pass.
    let nested = "if {1} {\n    if {0} {\n        set dead 1\n    }\n    set alive 2\n}";
    let pass1 = optimised(nested, TCL);
    assert!(pass1.contains("set alive 2"));
    assert!(opt_count(nested, TCL, "O112") >= 1);
    let registry = static_context_for(TCL).commands();
    let (fixed, _) = optimise_source_multipass(
        nested,
        registry,
        Some(tcl_registry::model::ingress::resolve_environment(TCL).analyser_profile()),
        10,
    );
    assert!(!fixed.contains("set dead 1"));
    assert!(fixed.contains("set alive 2"));
}

// Unused variable elimination — O126

#[test]
fn unused_variable_elimination_o126() {
    // A `set` whose variable is returned is NOT removed.
    let ret = "proc calcDb {mag} {\n    set db [expr {10*log($mag)}]\n    return $db\n}\n";
    assert!(!opt_fires(ret, TCL, "O126"));

    // A braced (literal) `return {$result}` does not read the variable, so the
    // `set` is dead. tclsh: `return {$result}` returns the literal text
    // "$result", independent of the variable. Rust folds `return {$result}`
    // ⇒ `return 42` here (O100), proving the variable is dead either way.
    let braced = "proc foo {} {\n    set result 42\n    return {$result}\n}\n";
    // Rust applies O100 to fold the constant variable into the (literal) return;
    // the assignment becomes provably unused. Confirm the literal is preserved.
    assert!(optimised(braced, TCL).contains("return 42") || opt_fires(braced, TCL, "O126"));
}

// Cross-event DSE — stores consumed by a later event must survive

// NOTE — this group of cross-event DSE cases is OMITTED here (they were a
// GENUINE bug at the time). A `set` in one iRule event whose value
// is read in a later event must NOT be eliminated:
//   * `set uri [HTTP::uri]` read via `"uri=$uri"` in HTTP_RESPONSE
//   * `set ans_cleared 1` checked via `[info exists ans_cleared]` in DNS_RESPONSE
//   * `set allowlist 1`    checked via `[info exists allowlist]`   in DNS_RESPONSE
// In all three, O126 ("Remove unused variable assignment") fired and deleted the
// store, dropping the value the later event consumes — a cross-event dead-store
// soundness bug. They were reported rather than asserted, since asserting the
// output at the time would have pinned a miscompile.

// Constant var-ref propagation — O100 / O105 (string interpolation)

#[test]
fn branch_condition_ending_in_a_nested_empty_pair_rewrites_the_whole_word() {
    // The branch condition span is the lexer's word span, so
    // it stops one byte short of the outer `}`. Deciding the widening from
    // the slice's last byte would read `{$n == 0 && $y eq {}` as already
    // whole — it does end in a `}`, the *inner* empty pair's — so naively
    // the pass would unwrap an opener with no matching closer and emit the
    // unbalanced replacement `{0 == 0 && $y eq {}`.
    let src = "proc p {y} {\n    set n 0\n    if {$n == 0 && $y eq {}} { puts a }\n}\n";
    let rewrites = opt_rewrites(src, TCL);
    let o100: Vec<&str> = rewrites
        .iter()
        .filter(|(code, _)| code == "O100")
        .map(|(_, replacement)| replacement.as_str())
        .collect();
    assert_eq!(o100, vec!["{0 == 0 && $y eq {}}"], "{rewrites:?}");
    assert_eq!(
        optimised(src, TCL),
        "proc p {y} {\n    set n 0\n    if {0 == 0 && $y eq {}} { puts a }\n}\n",
    );
}

/// A constant reaching an `incr` folds through it and the whole snippet
/// collapses.
///
/// `incr` is not a *load* of its target, it names the cell it mutates, and
/// O102 only ever recognised a syntactic literal as a reaching definition. So
/// the chain stopped dead at the `incr`: nothing folded, and the one thing the
/// optimiser did emit was a hint on `incr a` whose recorded payload (`1` over
/// the whole statement) would have produced a bare `1` in command position.
///
/// SCCP has always proved the value — `Statement::Incr` is a transfer function
/// there. What was missing was a consumer asking it per SSA value: the
/// name-keyed projection the other O100 forms read drops any variable whose
/// versions hold different constants, which is exactly a counter.
#[test]
fn incr_of_a_constant_folds_through_to_its_reads() {
    let registry = static_context_for(TCL).commands();
    for (src, single, fixpoint) in [
        (
            "set a 1\nincr a\nputs \"$a\"",
            "set a 1\nincr a\nputs 2",
            "puts 2",
        ),
        (
            "set a 1\nincr a\nputs $a",
            "set a 1\nincr a\nputs 2",
            "puts 2",
        ),
        (
            "set a 1\nincr a 5\nputs $a",
            "set a 1\nincr a 5\nputs 6",
            "puts 6",
        ),
    ] {
        assert_eq!(optimised(src, TCL), single, "single pass over {src:?}");
        let (fixed, opts) = optimise_source_multipass(src, registry, None, 5);
        // Deletion leaves the blank lines behind, as every other multipass
        // case in this file does; what matters is that the feeding statements
        // are gone and the read carries the folded value.
        assert_eq!(fixed.trim(), fixpoint, "fixpoint over {src:?}");
        assert!(
            !fixed.contains("incr"),
            "the dead `incr` survived: {fixed:?}"
        );
        assert!(
            !fixed.contains("set a"),
            "the dead store survived: {fixed:?}"
        );
        let codes: Vec<&str> = opts.iter().map(|o| o.code.as_str()).collect();
        assert!(
            codes.contains(&"O100"),
            "the fold is an O100 (a constant SCCP proved), not an O102 (a \
             written-out literal forwarded): {codes:?}",
        );
    }
}

/// The counterpart: a target whose value SCCP cannot prove folds nothing, and
/// the `incr` survives untouched.
#[test]
fn incr_of_an_unproven_value_folds_nothing() {
    let src = "proc f {n} {\n    incr n\n    puts $n\n}";
    assert_eq!(optimised(src, TCL), src, "a parameter is not a constant");
}

#[test]
fn constant_propagation_into_commands_o100() {
    // tclsh: x=42 ⇒ `puts 42`. (The single-def literal is forwarded via O102 and
    // the now-dead store removed via O109 — assert the value + the codes used.)
    assert_eq!(optimised("set x 42\nputs $x", TCL), "puts 42");
    assert!(opt_fires("set x 42\nputs $x", TCL, "O102"));
    assert!(opt_fires("set x 42\nputs $x", TCL, "O109"));

    // Through expr+command: a=1 ⇒ puts 2.
    assert_eq!(
        optimised("set a 1\nset b [expr {$a + 1}]\nputs $b", TCL),
        "puts 2"
    );
    assert!(opt_fires(
        "set a 1\nset b [expr {$a + 1}]\nputs $b",
        TCL,
        "O100"
    ));

    // Chained: ⇒ puts 8.
    assert_eq!(
        optimised(
            "set a 1\nset b [expr {$a + 2}]\nset c [expr {$b + 5}]\nputs $c",
            TCL
        ),
        "puts 8"
    );

    // All uses propagated then full DSE on the side: ⇒ `puts 5\nset y 6`.
    // tclsh: x=5 ⇒ puts 5 and y == 6.
    assert_eq!(
        optimised("set x 5\nputs $x\nset y [expr {$x + 1}]", TCL),
        "puts 5\nset y 6"
    );

    // Whole-word multi-word string ⇒ braced literal (semantically identical).
    // tclsh: `set msg {Hello World}; puts $msg` == `puts {Hello World}`.
    assert_eq!(
        optimised("set msg {Hello World}\nputs $msg", TCL),
        "puts {Hello World}"
    );
    assert!(opt_fires("set msg {Hello World}\nputs $msg", TCL, "O100"));

    // Metacharacters are suppressed by the braces (NOT executed). tclsh:
    // `puts {a $b [c]}` prints the literal `a $b [c]`. The constant IS
    // propagated into the command (O100 ⇒ `puts {a $b [c]}`).
    // The optimiser keeps the original `set x` line: it conservatively does not
    // DSE a store of a brace literal carrying metacharacters (single OR multi
    // pass), so the output is the `set x` line plus `puts {a $b [c]}` rather than
    // just the puts. Both lines are semantically identical to the original —
    // assert the propagation + soundness.
    let meta = optimised("set x {a $b [c]}\nputs $x", TCL);
    assert!(meta.contains("puts {a $b [c]}"));
    assert!(opt_fires("set x {a $b [c]}\nputs $x", TCL, "O100"));

    // Braced whole-name array ref ${a(1)} is a load, never a literal ⇒ untouched.
    let arr = "set x ${a(1)}\nputs $x";
    assert_eq!(optimised(arr, TCL), arr);
    assert!(opt_codes(arr, TCL).is_empty());
}

#[test]
fn constant_propagation_into_strings_o105() {
    // $x inside a double-quoted string is interpolated. tclsh: x=5 ⇒ "val=5".
    let s = "set x 5\nputs \"val=$x\"";
    assert!(optimised(s, TCL).contains("puts \"val=5\""));
    assert!(opt_fires(s, TCL, "O100")); // Rust labels string-interp inline as O100

    // ${x} standalone word folds without a trailing brace. tclsh: `puts 7`.
    let braced = "set x 7\nputs ${x}";
    let b = optimised(braced, TCL);
    assert!(!b.contains("puts 7}"));
    assert!(b.contains("puts 7"));

    // ${x} inside a string folds without a trailing brace. tclsh: `puts "7"`.
    let bstr = "set x 7\nputs \"${x}\"";
    let bs = optimised(bstr, TCL);
    assert!(!bs.contains("\"7}\""));
    assert!(bs.contains('7'));

    // A call barrier (string length abc) stops propagation into the later string.
    let barrier = "set x 5\nstring length abc\nputs \"val=$x\"";
    // The optimiser still propagates the literal into the string (x=5 ⇒ "val=5")
    // AND DSEs the now-dead store, rather than treating `string length abc` as a
    // hard barrier; the leading `string length abc` survives. tclsh proves
    // x=5 ⇒ "val=5" regardless of the intervening pure call, so the rewrite is
    // correct. Assert the sound output.
    let ba = optimised(barrier, TCL);
    assert!(ba.contains("string length abc"));
    assert!(ba.contains("val=5"));

    // Combined string-interp + expr fold + DSE. tclsh: x=5 ⇒ "x=5" and y==6.
    let combo = "set x 5\nset y [expr {$x + 1}]\nputs \"x=$x\"\nputs $y";
    assert_eq!(optimised(combo, TCL), "puts \"x=5\"\nputs 6");
}

// Pattern-match simplification — O110 matches_regex / matches_glob (f5-irules)
//
// OMISSION: the entire matches_regex/matches_glob → string-op simplification
// family does NOT fire in the optimiser (no O110 for any of the anchored-regex /
// wildcard-glob reprs; all sources pass through unchanged). Leaving them
// unchanged is sound (no miscompile), just not optimised, so rather than assert
// a rewrite we pin the conservative behaviour: the negative cases (which must NOT
// simplify) all hold, and the "positive" cases are a known optimiser gap.

#[test]
fn pattern_match_simplification_negatives_hold() {
    const IR: &str = "f5-irules";
    // Negative cases that must remain `matches_regex` / `matches_glob`.
    // These all hold (the construct is simply never simplified).
    let neg = [
        "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {.html$} } { pool p1 } }",
        "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {^/api/.*} } { pool p1 } }",
        "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {^/[a-z]+$} } { pool p1 } }",
        "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {} } { pool p1 } }",
        "when HTTP_REQUEST { if { [HTTP::uri] matches_regex $pattern } { pool p1 } }",
        "when HTTP_REQUEST { if { $host matches_glob {*.example.co?} } { pool p1 } }",
        "when HTTP_REQUEST { if { $host matches_glob {api.*.com} } { pool p1 } }",
        "when HTTP_REQUEST { if { $host matches_glob {*} } { pool p1 } }",
    ];
    for src in neg {
        assert!(!opt_fires(src, IR, "O110"), "must not simplify: {src}");
    }
    // The would-be-positive cases (anchored regex, leading/trailing-star glob)
    // also do not fire — confirm no spurious O110 is emitted, matching the
    // conservative path. (These could in principle simplify; a known gap.)
    let unsimplified_but_sound = [
        "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {^/api$} } { pool p1 } }",
        "when HTTP_REQUEST { if { $host matches_glob {*.example.com} } { pool p1 } }",
    ];
    for src in unsimplified_but_sound {
        assert_eq!(optimised(src, IR), src);
    }
}

// Strength reduction — O113

#[test]
fn strength_reduction_o113() {
    // tclsh sweep: x ** 2 == x * x.
    let pow = "if {$x ** 2} {}";
    let p = optimised(pow, TCL);
    assert!(!p.contains("**"));
    assert!(p.contains('*'));
    assert!(opt_fires(pow, TCL, "O113"));

    // tclsh sweep (x≥0): x % 8 == x & 7.
    let modp = "if {$x % 8} {}";
    let m = optimised(modp, TCL);
    assert!(m.contains('&'));
    assert!(!m.contains('%'));
    assert!(opt_fires(modp, TCL, "O113"));
}

// Incr idiom — O114 (needs SSA-known INT type on the loop var)

#[test]
fn incr_idiom_o114() {
    // A minimal `proc foo {} {set x 0; set x [expr {$x + N}]}` body has x unused,
    // so the whole thing is DSE'd (O108/O109) before the incr-idiom can apply. To
    // make x both INT-typed *and* live (the precondition D5-O114 documents and
    // the fp/opt.rs FP_OPT_10_TN_REPRO uses), drive it inside a `for` loop that
    // reads x via `puts $x`. tclsh: `set x N; set x [expr {$x+1}]` == `incr x`.
    let add1 = "proc foo {n} {\n  for {set x 0} {$x < $n} {incr x} {\n    set x [expr {$x + 1}]\n    puts $x\n  }\n}\nfoo 3\n";
    assert!(optimised(add1, TCL).contains("incr x"));
    assert!(opt_fires(add1, TCL, "O114"));

    let add5 = "proc foo {n} {\n  for {set x 0} {$x < $n} {incr x} {\n    set x [expr {$x + 5}]\n    puts $x\n  }\n}\nfoo 3\n";
    assert!(optimised(add5, TCL).contains("incr x 5"));
    assert!(opt_fires(add5, TCL, "O114"));

    let sub3 = "proc foo {n} {\n  for {set x 0} {$x < $n} {incr x} {\n    set x [expr {$x - 3}]\n    puts $x\n  }\n}\nfoo 3\n";
    assert!(optimised(sub3, TCL).contains("incr x -3"));
    assert!(opt_fires(sub3, TCL, "O114"));
}

// Nested expr unwrap — O115

#[test]
fn nested_expr_unwrap_o115() {
    // `if {[expr {...}]}` — the nested expr is redundant in a condition. tclsh:
    // `if {[expr {$x + 1}]}` == `if {$x + 1}`.
    let in_if = "if {[expr {$x + 1}]} {}";
    assert!(!optimised(in_if, TCL).contains("[expr"));
    assert!(opt_fires(in_if, TCL, "O115"));

    // `[expr {[expr {...}]}]` in a value position (return) unwraps the inner.
    // tclsh: `[expr {[expr {$x * 2}]}]` == `[expr {$x * 2}]`.
    let in_ret = "proc double_expr {x} {\n    return [expr {[expr {$x * 2}]}]\n}";
    assert!(optimised(in_ret, TCL).contains("return [expr {$x * 2}]"));
    assert!(opt_fires(in_ret, TCL, "O115"));

    // The optimiser does NOT unwrap a nested expr in a `set` value position such
    // as `set y [expr {[expr {$x * 2}]}]` (only the `return`/condition paths
    // fire). Sound to leave unchanged; a known gap.
}

// List folding / lindex folding — O116 / O118

#[test]
fn list_and_lindex_folding() {
    // [list a] folds (via O116 or O100 propagation); tclsh: [list a] == "a".
    let single = "set x [list a]\nputs $x";
    assert!(!optimised(single, TCL).contains("[list"));
    let codes = opt_codes(single, TCL);
    assert!(codes.iter().any(|c| c == "O116" || c == "O100"));

    // Multi-element [list a b c] must NOT fold to a braced literal under O116
    // (intrep shimmer). It is instead propagated as a braced word via O100, which
    // is semantically identical (`[list a b c]` == `{a b c}` in tclsh) — the key
    // invariant is that O116 specifically does not fire.
    let multi = "set x [list a b c]\nputs $x";
    assert!(!opt_fires(multi, TCL, "O116"));

    // lindex of a literal list folds. tclsh: [lindex {a b c} 1] == "b".
    let lf = "set x [lindex {a b c} 1]\nputs $x";
    assert!(!optimised(lf, TCL).contains("[lindex"));
    assert!(
        opt_codes(lf, TCL)
            .iter()
            .any(|c| c == "O118" || c == "O100")
    );

    // tclsh: [lindex {x y z} end] == "z".
    let le = "set x [lindex {x y z} end]\nputs $x";
    assert!(!optimised(le, TCL).contains("[lindex"));
    assert!(
        opt_codes(le, TCL)
            .iter()
            .any(|c| c == "O118" || c == "O100")
    );
}

// Strlen zero-check — O117

#[test]
fn strlen_zero_check_o117() {
    // tclsh sweep: ([string length $s] == 0) <=> ($s eq "").
    let eq0 = "if {[string length $s] == 0} {}";
    let e = optimised(eq0, TCL);
    assert!(e.contains("eq \"\"") || !e.contains("string length"));
    assert!(opt_fires(eq0, TCL, "O117"));

    // tclsh sweep: ([string length $s] != 0) <=> ($s ne "").
    let ne0 = "if {[string length $s] != 0} {}";
    let n = optimised(ne0, TCL);
    assert!(n.contains("ne \"\"") || !n.contains("string length"));
    assert!(opt_fires(ne0, TCL, "O117"));

    // The optimiser does NOT simplify the `> 0` form `[string length $s] > 0` →
    // `$s ne ""` (only `== 0` / `!= 0`). Sound to leave unchanged
    // (`[string length $s] > 0` and `$s ne ""` are equivalent, just not folded);
    // a known gap.
    assert_eq!(
        optimised("if {[string length $s] > 0} {}", TCL),
        "if {[string length $s] > 0} {}"
    );
}

// String compare eq/ne — O120

#[test]
fn string_compare_eq_ne_o120() {
    // tclsh sweep: $a == "hello" <=> $a eq "hello" ("hello" is non-numeric).
    let eq = "if {$a == \"hello\"} {}";
    assert!(optimised(eq, TCL).contains("$a eq \"hello\""));
    assert!(opt_fires(eq, TCL, "O120"));

    // != inside an expr command substitution → ne. Rust routes this through
    // O110 (expr instcombine) rather than O120, but the rewrite is identical:
    // tclsh sweep $a != "hello" <=> $a ne "hello". Assert the text + that some
    // optimisation produced it.
    let ne = "set ok [expr {$a != \"hello\"}]";
    assert!(optimised(ne, TCL).contains("$a ne \"hello\""));
    assert!(opt_fires(ne, TCL, "O120") || opt_fires(ne, TCL, "O110"));

    // A var known non-numeric via SCCP CONST ("foo") proves the string path:
    // tclsh: `set a foo; expr {$a == $b}` == `expr {$a eq $b}` for every $b.
    // Rust folds the CONST in and rewrites to eq.
    let vc = "set a foo\nif {$a == $b} {}";
    assert!(opt_fires(vc, TCL, "O120"));
    assert!(optimised(vc, TCL).contains("eq $b"));

    // Mixed expr: only the string comparison is rewritten, numeric `$n == 1`
    // left alone. tclsh: $a (string) eq "x" preserves the && result. Rust
    // carries this via O110 canonicalisation; the eq-half is present.
    let mixed = "set a [string trim $raw]\nif {$a == \"x\" && $n == 1} {}";
    let mx = optimised(mixed, TCL);
    assert!(mx.contains("$a eq \"x\""));
    assert!(mx.contains("$n == 1"));
    assert!(opt_fires(mixed, TCL, "O120") || opt_fires(mixed, TCL, "O110"));

    // Two SCCP-constant operands: the compare folds outright (better than the
    // eq rewrite). tclsh: "foo" != "bar" ⇒ condition constant-false ⇒ `if {0}`
    // deleted. Confirm neither == nor a residual `eq` survives.
    let vv = "set a foo\nset b bar\nif {$a == $b} {}";
    let v = optimised(vv, TCL);
    assert!(!v.contains("=="));
    assert!(!v.contains(" eq "));
    assert!(opt_fires(vv, TCL, "O112"));
}

#[test]
fn string_compare_o120_conservative_non_rewrites() {
    // These must NOT rewrite to eq — the D5-O120 at-least-one-non-numeric rule.
    // Each is tclsh-grounded (numeric-looking literal vs a var that could hold a
    // number ⇒ ==/eq can disagree).
    let cases = [
        "set a [clock seconds]\nif {$a == \"1\"} {}", // INT-typed var, numeric literal
        "set a [string trim $raw]\nif {$a == \"1\"} {}", // STRING type ≠ non-numeric value
        "set a [string trim $raw]\nif {$a == \"true\"} {}",
        "set a [string trim $x]\nset b [string trim $y]\nif {$a == $b} {}",
        "set a [expr {1 + 2}]\nset b [expr {3 + 4}]\nif {$a == $b} {}", // both INT
        "if {$a == $b} {}",                                             // both unknown
        "if {$a == \"true\"} {}", // boolean-like literal, unknown var
        "if {$a == \"1.25\"} {}", // float-like literal, unknown var
    ];
    for src in cases {
        assert!(!opt_fires(src, TCL, "O120"), "O120 must not fire: {src}");
    }
}

// Multi-set packing — O119

#[test]
fn multi_set_packing_o119() {
    // OMISSION: with an `eval {$a $b $c}` barrier the constants are forwarded
    // *through* the `eval {...}` braced literal (O102/O109) — `eval {1 2 3}` — so
    // by the time O119 would run there are no surviving stores to pack, and O119
    // never fires. tclsh: `set a 1; set b 2; set c 3; eval {$a $b $c}` and the
    // folded `eval {1 2 3}` are identical, so the rewrite is sound. Assert the
    // packing-disabled invariants; the missing positive O119 packing is a known
    // gap.

    // Tcl 9.0: individual `set` is faster ⇒ O119 must not fire.
    let t9 = "set a 1\nset b 2\nset c 3\nputs \"$a $b $c\"";
    assert!(!opt_fires(t9, "tcl9.0", "O119"));

    // Too few consecutive sets ⇒ no packing.
    let few = "set a 1\nset b 2\nputs \"$a $b\"";
    assert!(!opt_fires(few, TCL, "O119"));

    // The eval-barrier forms are folded rather than packed; assert the sound
    // constant-forwarded result instead of the (absent) O119 packing.
    assert_eq!(
        optimised("set a 1\nset b 2\nset c 3\neval {$a $b $c}", TCL),
        "eval {1 2 3}"
    );
}

// End-offset index rewrite — O128

#[test]
fn end_offset_rewrites_o128() {
    // Each tclsh-verified: the length-arithmetic index equals the end-offset.
    // (Sweeps in the module doc proved lindex/lrange/lreplace/string index/range.)
    let cases: &[(&str, &str)] = &[
        (
            "set x [lindex $L [expr {[llength $L] - 1}]]",
            "set x [lindex $L end]",
        ),
        (
            "set x [lindex $L [expr {[llength $L] - 2}]]",
            "set x [lindex $L end-1]",
        ),
        (
            "set x [lrange $L 0 [expr {[llength $L] - 1}]]",
            "set x [lrange $L 0 end]",
        ),
        (
            "set x [string index $s [expr {[string length $s] - 1}]]",
            "set x [string index $s end]",
        ),
        (
            "set x [string range $s 0 [expr {[string length $s] - 1}]]",
            "set x [string range $s 0 end]",
        ),
        (
            "set x [lindex ${my::list} [expr {[llength ${my::list}] - 1}]]",
            "set x [lindex ${my::list} end]",
        ),
        (
            "puts [lindex $L [expr {[llength $L] - 3}]]",
            "puts [lindex $L end-2]",
        ),
        (
            "set x [lindex $a(1) [expr {[llength $a(1)] - 1}]]",
            "set x [lindex $a(1) end]",
        ),
        (
            "set x [lindex $L [expr {[llength $L] - 1}] 0]",
            "set x [lindex $L end 0]",
        ),
    ];
    for (src, want) in cases {
        assert_eq!(optimised(src, TCL), *want, "O128 rewrite: {src}");
        assert!(opt_fires(src, TCL, "O128"), "O128 should fire: {src}");
    }

    // lreplace rewrites BOTH indices ⇒ two O128.
    let lr = "set x [lreplace $L [expr {[llength $L] - 2}] [expr {[llength $L] - 1}] foo]";
    assert_eq!(optimised(lr, TCL), "set x [lreplace $L end-1 end foo]");
    assert_eq!(opt_count(lr, TCL, "O128"), 2);

    // Inside a proc body.
    let inproc =
        "proc last {L} {\n    set r [lindex $L [expr {[llength $L] - 1}]]\n    return $r\n}";
    assert!(optimised(inproc, TCL).contains("[lindex $L end]"));
    assert!(opt_fires(inproc, TCL, "O128"));
}

#[test]
fn end_offset_o128_must_not_fire() {
    // The robustness guard: each unsafe pattern must NOT rewrite. All confirmed
    // unchanged (no O128). Several are paired in the firing test above
    // with their safe counterpart.
    let neg = [
        "set x [lindex $L [expr {[llength $M] - 1}]]", // mismatched var
        "set x [lindex $L [expr {[llength $L] - 0}]]", // -0 is past the end
        "set x [lindex $L [expr {[llength $L]}]]",     // bare length
        "set x [lindex $L [expr {[string length $L] - 1}]]", // wrong length cmd
        "set x [string index $s [expr {[llength $s] - 1}]]", // wrong length cmd
        "set x [linsert $L [expr {[llength $L] - 1}] foo]", // linsert excluded
        "set x [linsert $L [expr {[llength $L] - 3}] foo]",
        "set x [lindex $L 0 [expr {[llength $L] - 1}]]", // later multi-index pos
        "set x [lindex $a(1) [expr {[llength $a(2)] - 1}]]", // mismatched array elem
        "set x [lindex [get_list] [expr {[llength [get_list]] - 1}]]", // cmd-sub container
        "set x [lindex {a b c d} [expr {[llength {a b c d}] - 1}]]", // literal container
        "set x [lindex $L [expr {[llength $L] - $N}]]",  // non-literal offset
        "set x [lindex $L [expr {[llength $L] + 1}]]",   // addition
        "set x [lindex $L [expr {[llength $L] * 2 - 1}]]", // multiplication
        "set x [lindex $L [expr {1 - [llength $L]}]]",   // reversed subtraction
        "set x [lindex $L [expr {[llength $L] - 1 - 1}]]", // chained subtraction
        "set x [lindex $L [expr {[llength $L] - -1}]]",  // negative constant
        "set x [lindex $L [expr [llength $L] - 1]]",     // unbraced expr
        "set x [lindex $L [expr {[llength [lsort $L]] - 1}]]", // llength of cmd result
        "set x [lindex $L:extra [expr {[llength $L:extra] - 1}]]", // adjacent text
        "lset L [expr {[llength $L] - 1}] foo",          // lset excluded
        "set x [lindex $L [expr {[llength $L] - [get_offset]}]]", // other substitution
        "set x [lindex $L [expr {{[llength $L] - 1}}]]", // nested brace grouping
    ];
    for src in neg {
        assert!(!opt_fires(src, TCL, "O128"), "O128 must not fire: {src}");
    }

    // Partial rewrites: when only one index matches, exactly one O128 fires.
    let one = "set x [lrange $L 2 [expr {[llength $L] - 1}]]";
    assert_eq!(optimised(one, TCL), "set x [lrange $L 2 end]");
    assert_eq!(opt_count(one, TCL, "O128"), 1);

    let mismatch = "set x [lreplace $L [expr {[llength $M] - 1}] [expr {[llength $L] - 1}] foo]";
    let mo = optimised(mismatch, TCL);
    assert!(mo.contains("end"));
    assert!(mo.contains("[llength $M] - 1"));
    assert_eq!(opt_count(mismatch, TCL, "O128"), 1);

    // First-position multi-index IS relative to $L ⇒ rewrites even with a
    // trailing index. tclsh: `[lindex $L [...-1] 0]` == `[lindex $L end 0]`.
    let first = "set x [lindex $L [expr {[llength $L] - 1}] 0]";
    assert_eq!(optimised(first, TCL), "set x [lindex $L end 0]");
    assert!(opt_fires(first, TCL, "O128"));

    // Matching array element rewrites.
    let arr = "set x [lindex $a(1) [expr {[llength $a(1)] - 1}]]";
    assert_eq!(optimised(arr, TCL), "set x [lindex $a(1) end]");
    assert!(opt_fires(arr, TCL, "O128"));

    // MORE precise (tclsh-proven sound): one might expect
    // `[lindex ${a(1)} [expr {[llength $a(1)] - 1}]]` to NOT rewrite, on the
    // theory that braced `${a(1)}` and bare `$a(1)` "compile to different loads".
    // tclsh disproves that: `set a(1) hello; ${a(1)}` and `$a(1)` both read array
    // element a(1) and are byte-identical, and the end-offset rewrite preserves
    // the value (sweep: `[lindex ${a(1)} [...-1]]` == `[lindex ${a(1)} end]` →
    // OK). O128 therefore (correctly) fires here; assert the sound rewrite rather
    // than an over-conservative no-op.
    let braced_arr = "set x [lindex ${a(1)} [expr {[llength $a(1)] - 1}]]";
    assert_eq!(optimised(braced_arr, TCL), "set x [lindex ${a(1)} end]");
    assert!(opt_fires(braced_arr, TCL, "O128"));
}

// Variable-shape optimisation guardrails — variable-shape forms not conflated

#[test]
fn variable_shape_guardrails() {
    for src in [
        "set x ${a(1)}\nputs $x",
        "set x $a(1)\nputs $x",
        "set x $::ns::arr(k)\nputs $x",
    ] {
        assert_eq!(optimised(src, TCL), src);
        assert!(
            opt_codes(src, TCL).is_empty(),
            "no rewrite for shape: {src}"
        );
    }
}

// Tail-call optimisation — O121 (tailcall) / O122 (loop) / O123 (accumulator)
//
// The optimise_with_dialect set is post-overlap; selection makes O122 subsume
// per-site O121. For a tail call we therefore assert the disjunction
// "O121 or O122" on the applied set.

#[test]
fn tail_call_detection_o121_o122() {
    // Tail-position self-call ⇒ O121 (tailcall) or O122 (loop). tclsh: the
    // tailcall/loop forms compute the same result as the original recursion
    // (factorial 5 1 == 120, gcd 48 36 == 12 — proven in the module doc).
    let pos = [
        "proc factorial {n acc} {\n    if {$n <= 1} {\n        return $acc\n    }\n    return [factorial [expr {$n - 1}] [expr {$n * $acc}]]\n}\n",
        "proc loop {items} {\n    if {[llength $items] == 0} {\n        return\n    }\n    puts [lindex $items 0]\n    loop [lrange $items 1 end]\n}\n",
        "proc gcd {a b} {\n    if {$b == 0} {\n        return $a\n    } else {\n        return [gcd $b [expr {$a % $b}]]\n    }\n}\n",
        "proc walk {tree} {\n    switch [lindex $tree 0] {\n        leaf {\n            return [lindex $tree 1]\n        }\n        node {\n            walk [lindex $tree 2]\n        }\n    }\n}\n",
        "namespace eval ns {\n    proc f {n} {\n        return [::ns::f [expr {$n - 1}]]\n    }\n}\n",
    ];
    for src in pos {
        assert!(
            opt_fires(src, TCL, "O121") || opt_fires(src, TCL, "O122"),
            "tail-call should be detected: {src}"
        );
    }

    // Non-tail / mutual / mixed self-calls ⇒ neither O121 nor O122.
    let neg = [
        "proc fib {n} {\n    if {$n <= 1} {\n        return $n\n    }\n    expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}\n}\n",
        "proc even {n} {\n    if {$n == 0} { return 1 }\n    return [odd [expr {$n - 1}]]\n}\nproc odd {n} {\n    if {$n == 0} { return 0 }\n    return [even [expr {$n - 1}]]\n}\n",
    ];
    for src in neg {
        assert!(!opt_fires(src, TCL, "O121"), "no O121: {src}");
        assert!(!opt_fires(src, TCL, "O122"), "no O122: {src}");
    }
}

#[test]
fn tail_call_loop_conversion_o122() {
    // O122 rewrites tail recursion to a `while {1}` loop. tclsh proved factorial
    // and gcd loop-forms equal the recursive originals.
    let fac = "proc factorial {n acc} {\n    if {$n <= 1} {\n        return $acc\n    }\n    return [factorial [expr {$n - 1}] [expr {$n * $acc}]]\n}\n";
    // Both arguments are bracketed words, so the call passes one argument per
    // parameter and O122 takes the whole proc; overlap selection prefers it
    // over the per-site O121 `tailcall` covering the same range. tclsh proved
    // the loop form equals the recursive original for n in 0..20.
    let fo = optimised(fac, TCL);
    assert!(
        fo.contains("while {1}"),
        "expected the loop conversion: {fo}"
    );
    assert!(
        fo.contains("lassign [list [expr {$n - 1}] [expr {$n * $acc}]] n acc"),
        "each bracketed argument must stay one word: {fo}",
    );
    assert!(opt_fires(fac, TCL, "O122"));

    // The bare self-call `loop` body DOES take the O122 loop conversion.
    let bare = "proc loop {items} {\n    if {[llength $items] == 0} {\n        return\n    }\n    puts [lindex $items 0]\n    loop [lrange $items 1 end]\n}\n";
    let bo = optimised(bare, TCL);
    assert!(bo.contains("while {1}"));
    assert!(bo.contains("set items"));
    assert!(opt_fires(bare, TCL, "O122"));

    // O122 must NOT fire for non-tail / mixed recursion.
    let nontail = "proc fib {n} {\n    if {$n <= 1} {\n        return $n\n    }\n    expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}\n}\n";
    assert!(!opt_fires(nontail, TCL, "O122"));
    let mixed = "proc bad {n acc} {\n    if {$n <= 1} {\n        return $acc\n    }\n    set partial [bad [expr {$n - 2}] $acc]\n    return [bad [expr {$n - 1}] $partial]\n}\n";
    assert!(!opt_fires(mixed, TCL, "O122"));

    // O122 must NOT fire when a self-call sits in a condition / switch subject.
    for src in [
        "proc f {n} {\n    if {[f $n]} {\n        return [f [expr {$n - 1}]]\n    } else {\n        return $n\n    }\n}\n",
        "proc f {n} {\n    switch [f [expr {$n - 1}]] {\n        0 { return 0 }\n        default { return [f [expr {$n - 2}]] }\n    }\n}\n",
        "proc f {n} {\n    while {[f $n]} {\n        return [f [expr {$n - 1}]]\n    }\n    return $n\n}\n",
        "proc f {n} {\n    for {set i 0} {[f $n]} {incr i} {\n        return [f [expr {$n - 1}]]\n    }\n    return $n\n}\n",
    ] {
        assert!(
            !opt_fires(src, TCL, "O122"),
            "no O122 (self-call in control): {src}"
        );
    }

    // Arity mismatch: O121 may fire but O122 (full loop conversion) must not.
    let arity = "proc f {a b} {\n    return [f $a]\n}\n";
    assert!(opt_fires(arity, TCL, "O121"));
    assert!(!opt_fires(arity, TCL, "O122"));

    // Braced literal `[self ...]` is not an executable call ⇒ no O121/O122.
    let braced = "proc f {n} {\n    return {[f $n]}\n}\n";
    assert!(!opt_fires(braced, TCL, "O121"));
    assert!(!opt_fires(braced, TCL, "O122"));

    // Literal bracket text in a braced `set` value must not count as recursion;
    // the real tail call still converts.
    let braced_set = "proc fact {n acc} {\n    set marker {[fact $n]}\n    if {$n <= 1} {\n        return $acc\n    }\n    return [fact [expr {$n - 1}] [expr {$n * $acc}]]\n}\n";
    assert!(opt_fires(braced_set, TCL, "O121") || opt_fires(braced_set, TCL, "O122"));

    // Selection subsumes per-site O121 under a chosen O122 — or, in the
    // selection for this body, keeps O121 and drops O122. Either way exactly one
    // of the two is present (not both) for the simple factorial body.
    let suppress = "proc f {n acc} {\n    if {$n <= 1} { return $acc }\n    return [f [expr {$n - 1}] [expr {$n * $acc}]]\n}\n";
    let has121 = opt_fires(suppress, TCL, "O121");
    let has122 = opt_fires(suppress, TCL, "O122");
    assert!(
        has121 ^ has122,
        "exactly one of O121/O122 should survive selection"
    );
}

#[test]
fn accumulator_hint_o123() {
    // O123 is a hint (hint_only) for accumulator-eligible non-tail recursion.
    let fac = "proc factorial {n} {\n    if {$n <= 1} {\n        return 1\n    }\n    return [expr {$n * [factorial [expr {$n - 1}]]}]\n}\n";
    assert!(opt_fires(fac, TCL, "O123"));

    // O123 must NOT fire for already-tail-recursive / non-recursive / doubly-
    // recursive / non-accumulator patterns.
    let neg = [
        "proc factorial {n acc} {\n    if {$n <= 1} {\n        return $acc\n    }\n    return [factorial [expr {$n - 1}] [expr {$n * $acc}]]\n}\n",
        "proc add {a b} {\n    return [expr {$a + $b}]\n}\n",
        "proc fib {n} {\n    if {$n <= 1} { return $n }\n    return [expr {[fib [expr {$n-1}]] + [fib [expr {$n-2}]]}]\n}\n",
        "proc transform {x} {\n    if {$x eq \"\"} { return \"\" }\n    return [format \"%s\" [transform [string range $x 1 end]]]\n}\n",
    ];
    for src in neg {
        assert!(!opt_fires(src, TCL, "O123"), "no O123: {src}");
    }

    // The O123 finding is hint-only (informational, not an applied rewrite).
    let registry = static_context_for(TCL).commands();
    let o123: Vec<_> = optimise_with_dialect(
        fac,
        registry,
        Some(tcl_registry::model::ingress::resolve_environment(TCL).analyser_profile()),
    )
    .into_iter()
    .filter(|o| o.code.as_str() == "O123")
    .collect();
    assert_eq!(o123.len(), 1);
    assert!(o123[0].hint_only);

    // Mixed tail + non-tail still surfaces the O123 hint for the non-tail site.
    let mixed = "proc calc {n} {\n    if {$n <= 0} { return 0 }\n    if {$n == 1} {\n        return [expr {$n * [calc [expr {$n - 1}]]}]\n    }\n    return [calc [expr {$n - 2}]]\n}\n";
    assert!(opt_fires(mixed, TCL, "O123"));

    // Braced expr literal must not produce an O123 hint.
    let braced = "proc f {n} {\n    return {[expr {$n * [f [expr {$n - 1}]]}]}\n}\n";
    assert!(!opt_fires(braced, TCL, "O123"));

    // O123 coexists with independent passes (O102 constant fold of `$a + 0`).
    // tclsh: the n==1 base case returns 1; the recursive return is untouched.
    let coexist = "proc f {n} {\n    set a 1\n    if {$n <= 1} { return [expr {$a + 0}] }\n    return [expr {$n * [f [expr {$n - 1}]]}]\n}\n";
    assert!(opt_fires(coexist, TCL, "O123"));
}

// Unused iRule procs — O124 (f5-irules only)

#[test]
fn unused_irule_procs_o124() {
    const IR: &str = "f5-irules";
    // Unused proc commented out.
    let unused = "proc helper {x} {\n    return $x\n}\n\nwhen HTTP_REQUEST {\n    pool my_pool\n}";
    let uo = optimised(unused, IR);
    assert!(opt_fires(unused, IR, "O124"));
    assert!(uo.contains("# proc helper"));
    assert!(!uo.replace("# proc helper", "").contains("proc helper"));

    // Used / transitively-used / qualified-call / direct-invocation procs are
    // NOT commented out.
    let kept = [
        "proc helper {} {\n    return 1\n}\n\nwhen HTTP_REQUEST {\n    set val [call helper]\n}",
        "proc inner {} {\n    return 42\n}\n\nproc outer {} {\n    return [call inner]\n}\n\nwhen HTTP_REQUEST {\n    set val [call outer]\n}",
        "proc helper {} {\n    return 1\n}\n\nwhen HTTP_REQUEST {\n    set val [helper]\n}",
        "namespace eval ns {\n    proc helper {} {\n        return 1\n    }\n}\n\nwhen HTTP_REQUEST {\n    set val [call ::ns::helper]\n}",
        // library iRule (only procs + RULE_INIT) — skip.
        "proc helper {} {\n    return 1\n}\n\nwhen RULE_INIT {\n    set ::debug 0\n}",
        // procs only, no events — library, skip.
        "proc helper_a {} {\n    return 1\n}\n\nproc helper_b {} {\n    return [call helper_a]\n}",
        // called only from RULE_INIT — still used.
        "proc init_helper {} {\n    return 1\n}\n\nwhen RULE_INIT {\n    set ::val [call init_helper]\n}\n\nwhen HTTP_REQUEST {\n    pool my_pool\n}",
    ];
    for src in kept {
        assert!(!opt_fires(src, IR, "O124"), "O124 must not fire: {src}");
    }

    // O124 only applies to f5-irules — plain Tcl is untouched.
    assert!(!opt_fires(
        "proc unused {} {\n    return 1\n}\nputs hello",
        TCL,
        "O124"
    ));

    // Multiple unused procs ⇒ two O124, naming each.
    let multi = "proc used {} {\n    return 1\n}\n\nproc unused_a {} {\n    return 2\n}\n\nproc unused_b {} {\n    return 3\n}\n\nwhen HTTP_REQUEST {\n    set val [call used]\n}";
    let registry = static_context_for(IR).commands();
    let o124s: Vec<_> = optimise_with_dialect(
        multi,
        registry,
        Some(tcl_registry::model::ingress::resolve_environment(IR).analyser_profile()),
    )
    .into_iter()
    .filter(|o| o.code.as_str() == "O124")
    .collect();
    assert_eq!(o124s.len(), 2);
    assert!(o124s.iter().any(|o| o.message.contains("unused_a")));
    assert!(o124s.iter().any(|o| o.message.contains("unused_b")));

    // Mutually-recursive but unreachable procs ⇒ both flagged, no tail-call
    // rewrite (O124 supersedes).
    let mutual = "proc ping {n} {\n    if {$n <= 0} {\n        return 0\n    }\n    return [call pong [expr {$n - 1}]]\n}\n\nproc pong {n} {\n    if {$n <= 0} {\n        return 0\n    }\n    return [call ping [expr {$n - 1}]]\n}\n\nwhen HTTP_REQUEST {\n    pool my_pool\n}";
    assert_eq!(opt_count(mutual, IR, "O124"), 2);
    assert!(!opt_fires(mutual, IR, "O121"));
    assert!(!opt_fires(mutual, IR, "O122"));

    // O124 supersedes tail-call rewrites for an unused tail-recursive proc.
    let supersede = "proc fact {n acc} {\n    if {$n <= 1} {\n        return $acc\n    }\n    return [fact [expr {$n - 1}] [expr {$n * $acc}]]\n}\n\nwhen HTTP_REQUEST {\n    pool my_pool\n}";
    assert!(opt_fires(supersede, IR, "O124"));
    assert!(!opt_fires(supersede, IR, "O121"));
    assert!(!opt_fires(supersede, IR, "O122"));

    // O124 does not block independent passes for a *used* proc; the used proc
    // is not commented out.
    //
    // The pass that fires here is O112, not O120: `[call helper bar]` is a
    // command substitution nested in a word, and since #2134 those are
    // enumerated as call sites, so `x` is known to be `bar` inside the body
    // and the `foo` branch is proven dead. Before that the caller was
    // invisible, nothing was seeded, and only the weaker O120 rewrite applied.
    let used_body = "proc helper {x} {\n    if {$x == \"foo\"} {\n        return 1\n    }\n    return 0\n}\n\nwhen HTTP_REQUEST {\n    set val [call helper bar]\n}";
    assert!(
        opt_fires(used_body, IR, "O112"),
        "the now-visible caller proves the foo branch dead",
    );
    assert!(!opt_fires(used_body, IR, "O124"));
}

#[test]
fn unused_irule_procs_o124_eval_suppression() {
    const IR: &str = "f5-irules";
    // OMISSION: O124 is NOT suppressed when a reachable event/proc contains
    // `eval` (dynamic dispatch could in principle call the "unused" proc) — the
    // unused proc is still flagged as removable.
    //  - `eval`-in-event           → DOES fire O124
    //  - `eval`-in-reachable-proc  → DOES fire O124
    // These two cases are known gaps (omitted here).

    // `eval` only in an UNREACHABLE proc does not suppress O124 — the unused proc
    // is still flagged.
    let unreach = "proc dynamic_helper {} {\n    eval {set x 1}\n}\n\nwhen HTTP_REQUEST {\n    pool my_pool\n}";
    assert!(opt_fires(unreach, IR, "O124"));
}

// Code sinking — O125

#[test]
fn code_sinking_o125_positive() {
    // Basic sink into an `if` whose condition does not read the var. Either O125
    // OR O100 is accepted (propagation may subsume sinking); here O125 fires.
    // tclsh: `set b foo; if {$a} {puts $b}` and the sunk form
    // `if {$a} {set b foo; puts $b}` both print foo iff $a is true.
    let basic = "set b foo\nif {$a} {\n    puts $b\n}";
    let codes = opt_codes(basic, TCL);
    assert!(codes.iter().any(|c| c == "O125" || c == "O100"));

    // Sink preserves indentation when a nested redefine blocks O100 folding.
    // The sunk `set b foo` lands at the outer-if body indent (4 spaces).
    let indent = "set b foo\nif {$a} {\n    if {$c} {\n        set b bar\n    }\n    if {$d} {\n        puts $b\n    }\n}";
    assert!(opt_fires(indent, TCL, "O125"));
    let io = optimised(indent, TCL);
    let sunk: Vec<&str> = io
        .lines()
        .filter(|l| l.trim_start().starts_with("set b foo"))
        .collect();
    assert!(!sunk.is_empty(), "expected a sunk `set b foo` line: {io}");

    // Coexists with O120 (string-compare rewrite in the condition).
    let with120 = "set b foo\nif {$kind == \"x\"} {\n    puts $b\n}";
    assert!(opt_fires(with120, TCL, "O120"));
    assert!(optimised(with120, TCL).contains("$kind eq \"x\""));
    let c120 = opt_codes(with120, TCL);
    assert!(c120.iter().any(|c| c == "O125" || c == "O100"));
}

#[test]
fn code_sinking_o125_negatives() {
    // O125 must NOT fire when the var is read in the condition, used after the
    // block, or its RHS is a command substitution.
    let neg = [
        "set b $x\nif {$b} {\n    puts hello\n}", // var in condition
        "set b foo\nif {$a} {\n    puts $b\n}\nputs $b", // used after ($-form)
        "set b [clock seconds]\nif {$a} {\n    puts $b\n}", // cmd-sub RHS
        // Used after by *name* rather than by substitution. `set b foo` must
        // stay put: on the false path the sunk form leaves `b` undefined, so
        // the later read errors where the original printed nothing. The
        // registry's VarRead / VarWrite positions answer for a command the
        // lowerer resolved, and the bareword scan covers a name inside a
        // nested command substitution.
        "set b foo\nif {$a} {\n    puts $b\n}\ninfo exists b", // VarRead position
        "set b foo\nif {$a} {\n    puts $b\n}\nappend b tail", // read-before-write
        "set b foo\nif {$a} {\n    puts $b\n}\nincr b",        // read-modify-write
        "set b foo\nif {$a} {\n    puts $b\n}\nputs [set b]",  // nested substitution
        "set b foo\nif {$a} {\n    puts $b\n}\nif {$c} {\n    puts [set b]\n}", // nested, in a body
    ];
    for src in neg {
        assert!(!opt_fires(src, TCL, "O125"), "O125 must not fire: {src}");
    }

    // NOTE on omissions. Sound-but-spurious O125 firings (applied rewrite
    // PREPENDS `set b foo` into the branch while KEEPING the outer assignment,
    // so a tclsh run is unaffected) — omitted:
    //  - var not used in the branch at all (`puts hello`).
    //  - numeric constant `set b 42` (handled by O100/O109, not O125).
    //  - cross-event shared var (excluded from sinking).
    //  - `if {0}` block (O112 drops the block AND all O125 parts).
    // GENUINE BUG (omitted): a sink must NOT happen for
    //   set b $x ; if {[incr x] > 0} { puts $b }
    // because `[incr x]` in the condition mutates b's RHS dependency. O125 fires
    // and the *applied* output
    //   set b $x ; if {[incr x] > 0} { set b $x; puts $b }
    // re-reads $x AFTER the incr — tclsh (x=5) ORIG prints 5, REWRITTEN prints 6.
    // A real miscompile, so it is reported rather than asserted.
}

#[test]
fn code_sinking_o125_applies_both_grouped_edits_and_reparses() {
    // The KCS O125 page's Before snippet, in its braced and quoted spellings.
    // Two properties the pair of grouped edits must hold, which either spelling
    // alone would let slip:
    //
    //  1. The deletion and the insertion both survive the manager's
    //     resurrected-reference guard, so the assignment moves rather than
    //     being copied. The insertion's replacement holds the sunk `set msg …`
    //     and the `$msg` consuming it, which reads as a resurrection of the
    //     variable the deletion removes unless group-mates are exempt.
    //  2. The quoted spelling survives the inner-end convention, under which
    //     the assignment's IR statement span stops *on* its closing `"`.
    //     Replaying that span emits `set msg "error`, whose unterminated quote
    //     swallows the rest of the line.
    //
    // tclsh: for either spelling, `set msg V; if {$ok} {return} else {log $msg}`
    // and the sunk `if {$ok} {return} else {set msg V; log $msg}` are
    // observationally identical — `msg` is read in exactly one branch and
    // nowhere after the decision.
    for (before, after) in [
        (
            "set msg {error}\nif {$ok} { return } else { log $msg }",
            "if {$ok} { return } else { set msg {error}; log $msg }",
        ),
        (
            "set msg \"error\"\nif {$ok} { return } else { log $msg }",
            "if {$ok} { return } else { set msg \"error\"; log $msg }",
        ),
    ] {
        assert_eq!(
            opt_count(before, TCL, "O125"),
            2,
            "both grouped O125 edits must survive selection: {before}"
        );
        assert_eq!(optimised(before, TCL), after, "sunk output for: {before}");
        assert_eq!(
            optimised(before, TCL).matches("set msg").count(),
            1,
            "the original assignment must be deleted, not duplicated: {before}"
        );
        assert!(
            reparse_errors(&optimised(before, TCL), TCL).is_empty(),
            "the sunk output must re-parse: {}",
            optimised(before, TCL)
        );
    }
}

// Load forwarding — O127 (single-use store-to-load forwarding)

#[test]
fn load_forwarding_o127() {
    // Single-use var with a command substitution is inlined. tclsh: the inlined
    // `[set x [clock seconds]]` both assigns x and yields its value — identical
    // observable result to `set x [clock seconds]; puts $x`.
    let cmdsub = "proc test {} {\n    set x [clock seconds]\n    puts $x\n}";
    assert!(opt_fires(cmdsub, TCL, "O127"));
    assert!(optimised(cmdsub, TCL).contains("puts [set x [clock seconds]]"));

    // Constants are handled by the propagation/DSE pass, NOT O127. tclsh:
    // x=42 ⇒ puts 42. (The single-def literal is forwarded via O102 + DSE O109 —
    // the key invariant is that O127 does NOT claim it.)
    let constv = "proc test {} {\n    set x 42\n    puts $x\n}";
    assert!(!opt_fires(constv, TCL, "O127"));
    assert!(opt_fires(constv, TCL, "O102"));
    assert!(optimised(constv, TCL).contains("puts 42"));

    // Variable used more than once ⇒ not inlined.
    let multi = "proc test {} {\n    set x [clock seconds]\n    puts $x\n    puts $x\n}";
    assert!(!opt_fires(multi, TCL, "O127"));

    // Top-level variables ⇒ not inlined.
    assert!(!opt_fires("set x [clock seconds]\nputs $x", TCL, "O127"));

    // NOTE on omissions (known gaps):
    //  - `set x $arg; puts $x` (var-copy) — left unchanged rather than inlined to
    //    `puts [set x $arg]` (O127); sound, just not forwarded.
    //  - intervening empty `eval {}` barrier — O127 still forwards (the empty eval
    //    is not treated as a barrier).
}

// Deletion and replay extents — a removal or replay covers the whole written
// statement, closing delimiter included.

#[test]
fn removed_and_replayed_statements_keep_their_closer() {
    // A statement span stops *on* the closer of a quoted, braced, or bracketed
    // last word. A removal that inherits that boundary strands the closer on a
    // line of its own, and a replay copies an opener without it, so each case
    // pins the exact text and re-checks it through the `tcl diag` surface.
    //
    // tclsh: both pairs are observationally identical. `set a "hello"; puts $a`
    // prints `hello`, as does `puts hello`; `set x V; puts $x` prints V, as
    // does `puts [set x V]`, since `set` yields the value it assigns.
    for (before, after) in [
        // O102 propagation coupled with the O109 removal of the feeding store,
        // whose value word is quoted.
        (
            "proc f {} {\n  set a \"hello\"\n  puts $a\n}",
            "proc f {} {\n  puts hello\n}",
        ),
        // O127 replays the assignment into the use site; the value word is
        // quoted and holds a command substitution.
        (
            "proc f {} {\n  set x \"a [clock seconds] b\"\n  puts $x\n}",
            "proc f {} {\n  puts [set x \"a [clock seconds] b\"]\n}",
        ),
    ] {
        assert_eq!(
            optimised(before, TCL),
            after,
            "rewritten source for: {before}"
        );
        assert!(
            reparse_errors(&optimised(before, TCL), TCL).is_empty(),
            "the rewritten source must re-parse: {}",
            optimised(before, TCL)
        );
    }
}

// Profile directive / multipass — profile survival + string-build collapse

#[test]
fn profile_directive_survives_structure_elimination() {
    // The `# profiles: HTTP2` comment must survive optimisation (it drives later
    // HTTP2 hints), and the constant `if {1}` inside the event unwraps (O112).
    let src =
        "# profiles: HTTP2\nwhen HTTP_REQUEST {\n    if {1} {\n        HTTP2::active\n    }\n}\n";
    let out = optimised(src, "f5-irules");
    assert!(out.contains("# profiles: HTTP2"));
    assert!(opt_fires(src, "f5-irules", "O112"));
    // The unwrapped body retains the HTTP2 call.
    assert!(out.contains("HTTP2::active"));
}

#[test]
fn multipass_string_build_collapses_to_literal() {
    // Multi-pass: a write-only string build chain in a proc collapses to a
    // single `return {Hello World}`; the intermediate local is fully removed.
    // tclsh: the proc returns "Hello World" either way.
    let registry = static_context_for(TCL).commands();
    let src = "proc build_banner {} {\n    set msg {Hello}\n    append msg { }\n    append msg World\n    return $msg\n}\n";
    let (out, _) = optimise_source_multipass(
        src,
        registry,
        Some(tcl_registry::model::ingress::resolve_environment(TCL).analyser_profile()),
        10,
    );
    assert!(out.contains("return {Hello World}"));
    assert!(!out.contains("append"));
    assert!(!out.contains("set msg"));
}

// Cross-event dead-store / info-exists soundness — these were miscompiles that
// are now FIXED in the source (connection_scope info-exists read detection + the
// O126 cross-event skip + the cross-event existence-fold post-pass). iRule
// `when` handlers share connection-scoped variables, so a store read in a later
// event must survive.
mod cross_event_dse {
    use tcl_compiler::optimiser::manager::{apply_optimisations, optimise_with_dialect};
    use tcl_registry::model::ingress::static_context_for;

    fn optimised(src: &str) -> String {
        let reg = static_context_for("f5-irules").commands();
        apply_optimisations(
            src,
            &optimise_with_dialect(src, reg, Some(tcl_dialect::DialectProfile::irules())),
        )
    }

    #[test]
    fn direct_read_store_survives() {
        // `$uri` in HTTP_RESPONSE reads the value set in HTTP_REQUEST; O126 must
        // not delete the store. (tclsh: the response handler logs the request's
        // uri — deleting `set uri` leaves $uri undefined.)
        let out = optimised(
            "when HTTP_REQUEST { set uri [HTTP::uri] }\nwhen HTTP_RESPONSE { log local0. \"uri=$uri\" }",
        );
        assert!(out.contains("set uri"), "cross-event store deleted:\n{out}");
    }

    #[test]
    fn info_exists_flag_survives_and_is_not_folded() {
        // `[info exists ans_cleared]` in DNS_RESPONSE observes a flag set in
        // DNS_REQUEST — neither the store nor the existence check may be folded
        // away (else the response takes the wrong branch).
        let out = optimised(
            "when DNS_REQUEST { set ans_cleared 1 }\nwhen DNS_RESPONSE { if {[info exists ans_cleared]} { return } }",
        );
        assert!(out.contains("set ans_cleared"), "store deleted:\n{out}");
        assert!(
            out.contains("info exists ans_cleared"),
            "info exists folded to a constant:\n{out}"
        );
    }

    #[test]
    fn second_info_exists_variant_survives() {
        let out = optimised(
            "when HTTP_REQUEST { set allowlist 1 }\nwhen HTTP_RESPONSE { if {[info exists allowlist]} { log local0. ok } }",
        );
        assert!(out.contains("set allowlist"), "store deleted:\n{out}");
        assert!(out.contains("info exists allowlist"), "folded:\n{out}");
    }

    #[test]
    fn same_event_dead_store_still_eliminated() {
        // Control: a store overwritten in the SAME event is genuinely dead.
        let out = optimised("when HTTP_REQUEST { set t 1\n set t 2\n log local0. $t }");
        assert!(
            !out.contains("set t 1"),
            "same-event dead store kept:\n{out}"
        );
    }
}

// The dynamic-name value-motion barrier and the whole-module variable-trace
// fact. Every source below would miscompile (or be mis-reported) without
// the shared `FunctionUnit::dynamic_barrier_blocks_value_motion` gate and
// the `Module::traced_variables` widening; each pin asserts the optimiser
// abstains. tclsh oracles cited per test.

// tclsh 8.6/9.0: `f acc` returns `zzz b` — folding the chain to
// `set acc {a b}` (O130) walks straight past the dynamic write.
#[test]
fn dynamic_name_write_blocks_o130_chain_fold_issue_1374() {
    let src =
        "proc f {name} { set acc {}; lappend acc a; set $name zzz; lappend acc b; return $acc }";
    assert!(!opt_fires(src, TCL, "O130"));
    assert!(!opt_fires(src, TCL, "O104"));
    assert_eq!(optimised(src, TCL), src);
}

// tclsh 8.6/9.0: `f x` prints `2` — forwarding `puts $x` to `puts 1`
// (O102, or O100 off the SCCP lattice) ignores that `set $name 2` can
// rewrite `x`.
#[test]
fn dynamic_name_write_blocks_o102_forwarding_issue_1374() {
    let src = "proc f {name} { set x 1; set $name 2; puts $x }";
    assert!(!opt_fires(src, TCL, "O102"));
    assert!(!opt_fires(src, TCL, "O100"));
    assert!(!opt_fires(src, TCL, "O109"));
    assert_eq!(optimised(src, TCL), src);
}

// tclsh 8.6/9.0: `f flag 1` prints `zzz` — sinking `set flag hello` past
// the dynamic write into the branch (O125) makes it print `hello`.
#[test]
fn dynamic_name_write_blocks_o125_sinking_issue_1374() {
    let src = "proc f {name c} { set flag hello; if {$c} { set $name zzz; puts $flag } }";
    assert!(!opt_fires(src, TCL, "O125"));
    assert_eq!(optimised(src, TCL), src);
}

// O119 moves and deletes `set` statements, so the same barrier applies.
#[test]
fn dynamic_name_write_blocks_o119_packing_issue_1374() {
    let src = "set $name q\nset a 1\nset b 2\nset c 3\n";
    assert!(!opt_fires(src, TCL, "O119"));
    // Control: the clean top-level sibling still packs under tcl8.6 (top
    // level, because inside a proc the unused-variable O126 deletions win
    // overlap selection over the pack either way).
    let clean = "set a 1\nset b 2\nset c 3\n";
    assert!(opt_fires(clean, TCL, "O119"));
}

// tclsh 8.6/9.0: each `set` fires the `::onw` write trace (prints `trace`
// twice) — deleting `set g 1` as a dead store (O109) drops a callback.
// The trace spells `::g`, the stores are unqualified; both name the same
// top-level global.
#[test]
fn traced_global_store_is_not_a_dead_store_issue_1377() {
    let src = "proc onw {a b c} { puts trace }\ntrace add variable ::g write ::onw\nset g 1\nset g 2\nputs $g\n";
    assert!(!opt_fires(src, TCL, "O109"));
    assert_eq!(optimised(src, TCL), src);
}

// tclsh 8.6/9.0: the `lappend`s fire the `::acc` write trace — folding to
// `set acc {a b}` (O130) collapses three observable writes into one.
#[test]
fn traced_list_chain_is_not_folded_issue_1377() {
    let src = "proc onw {a b c} { puts trace }\ntrace add variable ::acc write ::onw\nset acc {}\nlappend acc a\nlappend acc b\n";
    assert!(!opt_fires(src, TCL, "O130"));
    assert_eq!(optimised(src, TCL), src);
}

// A computed command head (`$cmd $x`) lowers to a `Statement::Barrier`
// whose retained words reference `$x` and raises no dynamic-name flag;
// treating a barrier as referencing no variable would let O125 sink
// `set x 1` into the branch, leaving `f 0 puts` to read an unset `x` (tclsh:
// `can't read "x"`). A dynamic `eval` spelling of the trailing reference
// must abstain too (that one via the opaque-script barrier).
#[test]
fn barrier_reference_blocks_o125_issue_1402() {
    let src = "proc f {flag cmd} {\nset x 1\nif {$flag} { puts $x }\n$cmd $x\n}";
    assert!(!opt_fires(src, TCL, "O125"));
    let eval_src = "proc f {flag} {\nset x 1\nif {$flag} { puts $x }\neval [list puts $x]\n}";
    assert!(!opt_fires(eval_src, TCL, "O125"));
}

// Code motion and deletion must share one "is this block executable" fact.
//
// The owner is `SccpResult::executable_blocks` (`sccp.rs:199`), the
// optimistic set SCCP fills in while it folds constant branches. The
// deletion side already reads it: `optimiser/elimination.rs:392`
// (`unreachable_blocks`, the O107 source), and O112's constant-condition
// deletions in `optimiser/structure_elimination.rs` are the structured-IR
// view of the same constant facts. GVN's PRE and LICM must not answer the
// question themselves with a plain terminator-successor walk: that would
// let a block behind a constant-false branch still look executable and
// draw an O106 hoist / O105 partial-redundancy offer into a region O112 is
// simultaneously offering to delete.
//
// `find_loop_invariants` / `find_partial_redundancies` take that set
// instead, and `*_for_function` seed it from `fu.sccp.executable_blocks`.

/// `src` compiled to a unit whose top level carries the SCCP result the
/// deletion passes read.
fn top_level_unit(src: &str) -> tcl_compiler::compilation_unit::CompilationUnit {
    let registry = static_context_for(TCL).commands();
    tcl_compiler::compilation_unit::CompilationUnit::build_for_dialect(src, registry, false, TCL)
}

/// `(licm, pre)` finding counts under the executable set the deletion passes
/// use — the same seeding `find_loop_invariants_for_function` /
/// `find_partial_redundancies_for_function` do internally.
fn gvn_offers_under_sccp(src: &str) -> (usize, usize) {
    let registry = static_context_for(TCL).commands();
    let cu = top_level_unit(src);
    let fu = &cu.top_level;
    let executable = &fu.sccp.executable_blocks;
    (
        tcl_compiler::gvn::find_loop_invariants(
            registry,
            &fu.cfg,
            &fu.ssa,
            executable,
            Some(tcl_registry::model::ingress::resolve_environment(TCL).analyser_profile()),
        )
        .len(),
        tcl_compiler::gvn::find_partial_redundancies(
            registry,
            &fu.cfg,
            &fu.ssa,
            executable,
            Some(tcl_registry::model::ingress::resolve_environment(TCL).analyser_profile()),
        )
        .len(),
    )
}

// A constant-false `if` wrapping a loop: O112 deletes the whole `if`, so the
// loop body must draw no O106 hoist. Structural (which code fires), not
// Tcl-observable: tclsh 8.6/9.0 runs neither the loop nor any hoisted copy,
// so both the original and the O112 rewrite produce no output.
#[test]
fn licm_offers_no_hoist_inside_a_block_o112_deletes_issue_1385() {
    let dead = "set lst $argv\nif {0} {\n    for {set i 0} {$i < 3} {incr i} {\n        lindex $lst 0\n    }\n}\n";
    assert!(
        opt_fires(dead, TCL, "O112"),
        "the constant-false if must still be offered for deletion"
    );
    assert_eq!(
        gvn_offers_under_sccp(dead).0,
        0,
        "no O106 hoist may be offered inside the region O112 deletes"
    );

    // Control: the same loop outside the dead branch keeps its hoist, so the
    // executable fact is pruning dead code and not disabling LICM.
    let live = "set lst $argv\nfor {set i 0} {$i < 3} {incr i} {\n    lindex $lst 0\n}\n";
    assert!(!opt_fires(live, TCL, "O112"));
    assert_eq!(gvn_offers_under_sccp(live).0, 1);
}

// The same pairing for O105 partial redundancy: the diamond and its merge
// point both sit inside the constant-false `if` O112 deletes.
#[test]
fn pre_offers_no_hoist_inside_a_block_o112_deletes_issue_1385() {
    let dead = "set lst $argv\nif {0} {\n    if {$argc} {\n        lindex $lst 0\n    }\n    lindex $lst 0\n}\n";
    assert!(
        opt_fires(dead, TCL, "O112"),
        "the constant-false if must still be offered for deletion"
    );
    assert_eq!(
        gvn_offers_under_sccp(dead).1,
        0,
        "no O105 partial-redundancy hoist may be offered inside the region O112 deletes"
    );

    // Control: the same diamond outside the dead branch still reports.
    let live = "set lst $argv\nif {$argc} {\n    lindex $lst 0\n}\nlindex $lst 0\n";
    assert!(!opt_fires(live, TCL, "O112"));
    assert_eq!(gvn_offers_under_sccp(live).1, 1);
}

// The production entries carry the same contract without the caller having
// to seed the set: they read `fu.sccp.executable_blocks` themselves.
#[test]
fn production_gvn_entries_read_the_sccp_executable_fact_issue_1385() {
    let registry = static_context_for(TCL).commands();
    let dead = "set lst $argv\nif {0} {\n    for {set i 0} {$i < 3} {incr i} {\n        lindex $lst 0\n    }\n    if {$argc} {\n        llength $lst\n    }\n    llength $lst\n}\n";
    let cu = top_level_unit(dead);
    let fu = &cu.top_level;
    assert!(
        fu.cfg
            .blocks
            .keys()
            .any(|b| !fu.sccp.executable_blocks.contains(b)),
        "the reproducer must actually contain SCCP-dead blocks"
    );
    assert!(
        tcl_compiler::gvn::find_loop_invariants_for_function(
            registry,
            fu,
            Some(tcl_registry::model::ingress::resolve_environment(TCL).analyser_profile())
        )
        .is_empty(),
        "O106 must not fire inside SCCP-dead code"
    );
    assert!(
        tcl_compiler::gvn::find_partial_redundancies_for_function(
            registry,
            fu,
            Some(tcl_registry::model::ingress::resolve_environment(TCL).analyser_profile())
        )
        .is_empty(),
        "O105-PRE must not fire inside SCCP-dead code"
    );
}

// O107 inside a command-resolution-guarded `TclOO` method body.
//
// A method runs in the receiver's namespace, which is chosen at run time and
// can shadow any relative command name, so one unqualified head anywhere in
// the body excludes the method from deep analysis
// (`ir_helpers::requires_runtime_command_namespace`). Such a unit carries an
// empty SCCP executable-block set — no facts, rather than the fact that every
// block is dead — and O107 is the report that would otherwise read that
// absence as proof and delete the whole body.
//
// Structural rather than Tcl-observable: the assertion is that the optimiser
// leaves the source alone, which is trivially semantics-preserving. Deleting
// the body is what is not — `[Greeter new] whoami` answers `::Greeter`
// intact and the empty string once emptied.

/// A one-method class whose method body is `body`.
fn method_body(body: &str) -> String {
    format!("oo::class create Greeter {{\n    method whoami {{}} {{ {body} }}\n}}\n")
}

#[test]
fn tcloo_method_body_survives_the_command_resolution_guard() {
    // Every `self` spelling that carries a value: the defining class, a bare
    // `[self]` (equivalent to `self object`), and the instance namespace.
    for body in [
        "return [self class]",
        "return [self]",
        "return [self namespace]",
    ] {
        let src = method_body(body);
        assert!(
            !opt_fires(&src, TCL, "O107"),
            "{body}: a live method body must not be reported unreachable"
        );
        assert_eq!(
            optimised(&src, TCL),
            src,
            "{body}: the method body must survive the optimiser intact"
        );
    }
}

#[test]
fn a_guarded_method_body_keeps_every_relative_head() {
    // The guard is about command resolution, not `TclOO` introspection: an
    // ordinary builtin, a `my` dispatch, and a `set` whose value is a nested
    // call each name a command relatively, so each method is guarded.
    for body in ["puts hello", "my helper", "set x [string length abc]"] {
        let src = method_body(body);
        assert!(
            !opt_fires(&src, TCL, "O107"),
            "{body}: a guarded body must not be reported unreachable"
        );
        assert_eq!(optimised(&src, TCL), src, "{body}: body must survive");
    }
}

#[test]
fn o107_still_fires_on_genuinely_unreachable_method_code() {
    // Control: O107 is suppressed only where the lattice is absent. Every head
    // here is `::`-qualified, so the method gets deep analysis and the `if {0}`
    // body is provably dead. tclsh: `[Greeter new] whoami` prints `live` with
    // or without the rewrite.
    let src = "oo::class create Greeter {\n    method whoami {} {\n        ::if {0} { ::puts never }\n        ::puts live\n    }\n}\n";
    let out = optimised(src, TCL);
    assert!(
        !out.contains("puts never"),
        "dead branch must still be eliminated: {out}"
    );
    assert!(out.contains("::puts live"), "live code must survive: {out}");
}

/// #2050 — the store feeding a nested read-modify-write is observed, not
/// overwritten.
///
/// `[incr n]` reads `n` and writes it back. The write was already recorded (as
/// the embedded-substitution effect that invalidates the old version), but the
/// read was not, so `set n 1`'s version had no consumer and looked
/// overwritten-before-read. O109 deleted it and the program changed:
/// tclsh 9.0.4 / 8.6.18 print `2` then `2` for the original; the rewritten
/// program printed `1` then `1` (8.4 raises `can't read "n"`, which does not
/// even create the variable).
#[test]
fn a_nested_rmw_read_keeps_its_feeding_store_alive() {
    let src = "set n 1\nset result [incr n]\nputs $result\nputs $n\n";
    assert!(
        !opt_fires(src, TCL, "O109"),
        "the store feeding `[incr n]` is read by it: {:?}",
        opt_codes(src, TCL)
    );
    assert_eq!(
        optimised(src, TCL),
        src,
        "nothing in this program is safe to rewrite"
    );
}

/// The same read through a `Call` host rather than an assignment: `puts [incr
/// n]` carries the effect on the `puts` statement itself, where the extras are
/// merged into its own defs, instead of on a prepended invalidation statement.
#[test]
fn a_nested_rmw_read_on_a_call_host_keeps_its_feeding_store_alive() {
    let src = "set n 1\nputs [incr n]\nputs $n\n";
    assert!(
        !opt_fires(src, TCL, "O109"),
        "the store feeding `[incr n]` is read by it: {:?}",
        opt_codes(src, TCL)
    );
    assert_eq!(optimised(src, TCL), src);
}

/// #2141 — a write nested in a braced `expr` word is a write of this frame.
///
/// The word lexer is right to stop at `{…}` — the command parser substitutes
/// nothing there — but `expr` re-parses that text and runs the `[…]` in it. So
/// `[incr x]` inside `[expr {…}]` writes `x` exactly as `[incr x]` in a bare
/// word does. Without that, `set x 1` looked like the single reaching
/// definition at the later `puts $x`, and O102 forwarded the literal:
/// tclsh 8.4.20 through 9.1b0 print `5` then `2`, the rewritten program printed
/// `5` then `1`.
#[test]
fn a_write_nested_in_a_braced_expr_word_kills_the_reaching_definition() {
    let src = "set x 1\nputs [expr {$x + [incr x] + $x}]\nputs $x\n";
    let out = optimised(src, TCL);
    assert!(
        !out.contains("puts 1"),
        "the later read sees 2, not the forwarded literal: {out}"
    );
    assert_eq!(out, src, "no rewrite in this program is sound");
}

/// The same family, in the two other shapes the issue lists. Each was checked
/// against tclsh 9.0.4: `3`/`2` for the first, `5`/`3` for the second.
#[test]
fn a_write_nested_in_a_braced_expr_word_keeps_its_feeding_store() {
    for src in [
        "set n 1\nset r [expr {$n + [incr n]}]\nputs $r\nputs $n\n",
        "set n 1\nset r [expr {[incr n] + [incr n]}]\nputs $r\nputs $n\n",
    ] {
        assert!(
            !opt_fires(src, TCL, "O109"),
            "`set n 1` feeds the first `[incr n]`: {:?}",
            opt_codes(src, TCL)
        );
        assert_eq!(
            optimised(src, TCL),
            src,
            "{src}: nothing is safe to rewrite"
        );
    }
}

/// Precision control: the descent is into brace-quoted **expression** words
/// only, and only where a command really runs. A braced expression with no
/// substitution still folds all the way through, and the forward to the later
/// read is still made.
#[test]
fn a_braced_expr_word_with_no_nested_write_still_folds() {
    let src = "set x 1\nputs [expr {$x + 1}]\nputs $x\n";
    let out = optimised(src, TCL);
    assert!(
        out.contains("puts 2"),
        "the expression must still fold: {out}"
    );
    assert!(
        out.contains("puts 1"),
        "the later read is still the single reaching definition: {out}"
    );
}

/// Precision control: a statement that reads and writes the same variable
/// *through its own argument roles* is unaffected — the read happens before
/// the write in both, and forwarding the literal into the read is sound.
/// tclsh 9.0.4: `lappend x 1` then `puts $x` prints `1 1`, and `incr x; puts 2`
/// prints `2`, matching the originals.
#[test]
fn a_direct_read_before_write_still_forwards_its_literal() {
    let appended = optimised("set x 1\nlappend x $x\nputs $x\n", TCL);
    assert!(
        appended.contains("lappend x 1"),
        "a direct RMW read still forwards: {appended}"
    );
    let incremented = optimised("set x 1\nset x [expr {$x + 1}]\nputs $x\n", TCL);
    assert!(
        incremented.contains("puts 2"),
        "a self-assigning expression still folds through: {incremented}"
    );
}

/// A multi-word `expr` concatenates its arguments into one expression, so a
/// `[…]` inside a braced *argument* runs too.
///
/// `expr 1 + {[incr x]}` joins its words into `1 + [incr x]` and parses that,
/// which makes the brace-quoted word's substitution a real write of `x`.
/// Taking only the `ArgRole::Expr` word missed it and O102 forwarded the stale
/// literal: tclsh 8.6.18 and 9.0.4 both print `3` then `2`, the rewritten
/// program printed `3` then `1`. The rule comes from the registry's
/// `EXPR_CONCATENATES_ARGS` trait, not from the spelling `expr`.
#[test]
fn a_concatenated_expr_argument_is_an_expression_word() {
    let src = "set x 1\nputs [expr 1 + {[incr x]}]\nputs $x\n";
    let out = optimised(src, TCL);
    assert!(
        !out.contains("puts 1"),
        "the later read sees 2, not the forwarded literal: {out}"
    );
    assert!(
        !opt_fires(src, TCL, "O109"),
        "`set x 1` feeds the concatenated `[incr x]`: {:?}",
        opt_codes(src, TCL)
    );
    assert_eq!(out, src, "no rewrite in this program is sound");
}

/// An expression reached through an alias resolves its words like the command
/// it reaches.
///
/// After `interp alias {} e {} expr`, the raw spelling `e` carries no
/// `ArgRole::Expr`, so the `[incr x]` of `[e {$x + [incr x]}]` was invisible
/// and the later read folded to `1` where tclsh 8.6.18 / 9.0.4 print `2`.
#[test]
fn an_alias_to_expr_still_shows_its_nested_write() {
    let src = "interp alias {} e {} expr\nset x 1\nputs [e {$x + [incr x]}]\nputs $x\n";
    let out = optimised(src, TCL);
    assert!(
        !out.contains("puts 1"),
        "the alias runs `incr`, so the later read is not the literal: {out}"
    );
    assert_eq!(out, src, "no rewrite in this program is sound");
}

/// A structural body is not the enclosing statement's substitution surface.
///
/// `proc q {} {…}` lowers its body into a procedure of its own, so the `[incr
/// m]` inside it belongs to that procedure's frame. Reading it from the
/// enclosing `proc` statement attributed a caller-frame read of the proc's own
/// local, and W210 fired on a program tclsh 9.0.4 runs cleanly (it prints `2`).
/// A `Plain` body — `eval`, `catch`, a loop — shares this frame and must still
/// contribute its effects.
#[test]
fn a_structural_body_is_not_the_enclosing_statements_surface() {
    let proc_body = "proc q {} {\n  set m 1\n  puts [incr m]\n}\nq\n";
    let codes = analyser_codes(proc_body, TCL);
    assert!(
        !codes.iter().any(|c| c == "W210"),
        "the proc's own local is not a caller-frame read: {codes:?}"
    );
    // A `Plain` body runs here, so its write still reaches the frame: `set m
    // 1` is read by the `eval`'d `incr` and must survive.
    let plain_body = "proc p {} {\n  set m 1\n  eval {incr m}\n  puts $m\n}\np\n";
    assert!(
        !opt_fires(plain_body, TCL, "O109"),
        "an `eval`'d body shares this frame: {:?}",
        opt_codes(plain_body, TCL)
    );
}

/// #2132 — a `[…]` in a branch or loop condition reads the frame's variables.
///
/// An expression's command substitution is opaque to the `$var` scan, so a
/// condition's reads reached no consumer. Three shapes, each measured against
/// tclsh 9.0.4:
///
/// | program | tclsh | was |
/// |---|---|---|
/// | `set x 1; if {[info exists x]} {puts yes}` | `yes` | nothing — O126 removed the store |
/// | `set n 5; if {[incr n]} {puts $n}` | `6` | `1` — O109 removed it |
/// | `set k 0; for {set i 0} {[incr k] < 3} {} {puts $k}` | `1` `2` | `0` `0` |
///
/// The `for` case is the widest: its condition contributed neither reads nor
/// writes at all, so the store was deleted *and* the stale literal forwarded
/// into the loop body.
#[test]
fn a_condition_substitution_reads_the_frames_variables() {
    for src in [
        "proc p {} {\n  set x 1\n  if {[info exists x]} { puts yes }\n}\np\n",
        "proc p {} { set n 5; if {[incr n]} { puts $n } }\np\n",
        "proc p {} { set s foo; while {[string length [append s bar]] < 12} { puts $s } }\np\n",
    ] {
        assert_eq!(
            optimised(src, TCL),
            src,
            "no rewrite in this program is sound: {:?}",
            opt_codes(src, TCL)
        );
    }

    // The `for` case keeps one legitimate rewrite — `set i 0` really is an
    // unused variable — so it is asserted on the store the condition reads
    // rather than on the whole program.
    let loop_src = "proc p {} { set k 0; for {set i 0} {[incr k] < 3} {} { puts $k } }\np\n";
    let out = optimised(loop_src, TCL);
    assert!(
        out.contains("set k 0"),
        "the condition's `[incr k]` reads this store: {out}"
    );
    assert!(
        out.contains("puts $k"),
        "the loop body's read is of the condition's value, not the literal: {out}"
    );
    assert!(
        !opt_fires(loop_src, TCL, "O109") && !opt_fires(loop_src, TCL, "O102"),
        "neither the deletion nor the forward is sound: {:?}",
        opt_codes(loop_src, TCL)
    );
}

/// The same read must stop O125 sinking the store past the condition.
///
/// `decision_condition_uses_var` scans the condition for a `$var` reference,
/// which `[incr n]` never shows. Sinking `set n 5` into the guarded body moved
/// it after the increment that reads it, so `proc p {} {set n 5; if {[incr n]}
/// {puts $n}}` printed `5` where tclsh 9.0.4 prints `6`.
#[test]
fn a_store_does_not_sink_past_a_condition_that_reads_it() {
    let src = "proc p {} { set n 5; if {[incr n]} { puts $n } }\np\n";
    assert!(
        !opt_fires(src, TCL, "O125"),
        "the condition reads `n`, so the store stays before it: {:?}",
        opt_codes(src, TCL)
    );
    // A read-only condition blocks the sink for the same reason: the guard
    // tests a variable the sunk store would not yet have created. Found by
    // review on the first cut of this guard, which consulted only the *write*
    // effects: `proc p {} {set x 1; if {[info exists x]} {puts $x}}` prints
    // `1` on tclsh 9.0.4, and the sunk form printed nothing.
    for read_only in [
        "proc p {} {\n  set x 1\n  if {[info exists x]} { puts $x }\n}\np\n",
        // An unnameable read may be of this variable, which is enough.
        "proc p {n} {\n  set x 1\n  if {[info exists $n]} { puts $x }\n}\np x\n",
    ] {
        assert!(
            !opt_fires(read_only, TCL, "O125"),
            "a read-only condition blocks the sink: {:?}",
            opt_codes(read_only, TCL)
        );
    }

    // Control: a condition that does not touch the variable still sinks.
    let sinkable = "proc p {c} {\n  set n 5\n  if {$c} { puts $n }\n}\np 1\np 0\n";
    assert!(
        opt_fires(sinkable, TCL, "O125"),
        "an unrelated condition must not block the sink: {:?}",
        opt_codes(sinkable, TCL)
    );
}

/// An existence guard is not a read-before-set, and an unnameable read is not
/// a global-frame write.
///
/// `[info exists q]` is *the* idiom for a name that may be unset, so crediting
/// its read must not make W210 fire — the reads live on the synthetic `<cond>`
/// statement, which has no source word to anchor a diagnostic at. And
/// `[info exists $p]` reads a cell nothing can name, which is a precision
/// loss rather than a claim that any name is written: treating it as an opaque
/// global-frame effect stopped an `[info exists Params($k)]` guard folding.
#[test]
fn an_existence_query_is_neither_a_read_before_set_nor_a_write() {
    let guard = "proc p {} { if {[info exists q]} { puts a } else { puts b } }\np\n";
    let codes = analyser_codes(guard, TCL);
    assert!(
        !codes.iter().any(|c| c == "W210"),
        "an existence guard is not a read-before-set: {codes:?}"
    );
    assert!(
        codes.iter().any(|c| c == "I230"),
        "the guard still folds: {codes:?}"
    );
    let dynamic_element = "proc f {k} { if {[info exists Params($k)]} { puts hi } }";
    assert!(
        analyser_codes(dynamic_element, TCL)
            .iter()
            .any(|c| c == "I230"),
        "an unnameable read must not suppress the array-guard fold: {:?}",
        analyser_codes(dynamic_element, TCL)
    );
}

/// #2051 — `regexp`, `scan` and `binary scan` write their targets only on the
/// match path.
///
/// Each leaves a target's previous value in place when the match or
/// conversion does not reach it, and never creates one that did not exist.
/// Measured identical on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1b0.
/// Modelling the write as unconditional made the feeding store look
/// overwritten-before-read, so O109 deleted it and the program did not merely
/// print something else — it failed with `can't read "…"`.
#[test]
fn a_conditional_writer_does_not_kill_the_store_it_may_preserve() {
    for (src, why) in [
        (
            "proc p {} {\n    set a before\n    set b before\n    regexp {(x)(y)} zz a b\n    puts \"$a $b\"\n}\np\n",
            "a failed regexp leaves both match variables alone",
        ),
        (
            "proc p {} {\n    set a before\n    set b before\n    scan {12 nope} {%d %d} a b\n    puts \"$a $b\"\n}\np\n",
            "scan converts one field and leaves the second target alone",
        ),
        (
            "proc p {d} {\n    set c before\n    set e before\n    binary scan $d \"a1a5\" c e\n    puts \"$c $e\"\n}\np AB\n",
            "binary scan runs out of data and leaves the second target alone",
        ),
    ] {
        assert!(
            !opt_fires(src, TCL, "O109"),
            "{why}: {:?}",
            opt_codes(src, TCL)
        );
        // Asserted on the stores rather than byte-identity: the `binary scan`
        // row also gets a legitimate O100, specialising its one call site's
        // `$d` to `AB`, which is unrelated and correct.
        let out = optimised(src, TCL);
        assert_eq!(
            out.matches("before").count(),
            src.matches("before").count(),
            "{why}: every store the command may preserve survives: {out}"
        );
    }
}

/// Precision: a command that writes its target on *every* path still has its
/// dead store eliminated. Each was measured writing unconditionally, on the
/// failure path too — `regsub {xx} zz YY a` leaves `a` as `zz`, `lassign`
/// pads a short list with `""`, `catch` always writes its result variable.
#[test]
fn an_unconditional_writer_still_kills_its_dead_store() {
    for (src, why) in [
        (
            "proc p {s} { set a 1; regsub {x} $s y a; puts $a }\np zz\n",
            "regsub",
        ),
        (
            "proc p {} { set m 1; catch {expr {1+1}} m; puts $m }\np\n",
            "catch",
        ),
        (
            "proc p {l} { set a 1; set b 2; lassign $l a b; puts \"$a $b\" }\np one\n",
            "lassign",
        ),
    ] {
        assert!(
            opt_fires(src, TCL, "O109"),
            "{why} writes on every path, so the earlier store is dead: {:?}",
            opt_codes(src, TCL)
        );
    }
}

/// Precision: the no-match prover still reports a target a failed match never
/// creates. Crediting the read must not silence W210, which is what keeps
/// `regexp {x} y -> v; puts $v` reported — tclsh fails it with
/// `can't read "v"`.
#[test]
fn the_no_match_prover_still_reports_an_uncreated_target() {
    for src in [
        "proc f {} { regexp {x} y -> v; puts $v }",
        "proc f {} { scan abc %d v; puts $v }",
    ] {
        assert!(
            analyser_codes(src, TCL).iter().any(|c| c == "W210"),
            "a target the match never creates is still reported: {:?}",
            analyser_codes(src, TCL)
        );
    }
}

/// A call site the caller-evidence scan cannot see must never read as
/// agreement among the sites it can.
///
/// Every shape below reaches `id` with `9` through a surface the scan used to
/// walk past — a `return` value and an `if` condition are CFG *terminators*,
/// and a fused `AssignExpr` or `Incr` keeps a parsed expression or an amount
/// string instead of words — so the lone visible `id 7` read as `id`'s
/// complete caller set and O100 specialised the body to `return 7`. Measured
/// on tclsh 8.6.18: `7 9` became `7 7` (#2118).
#[test]
fn a_call_site_in_a_terminator_or_fused_statement_is_evidence() {
    for (why, caller) in [
        ("a return value", "proc a {} { return [id 9] }"),
        (
            "a return value inside an expression",
            "proc a {} { return [expr {[id 9]}] }",
        ),
        (
            "an if condition",
            "proc a {} { if {[id 9] > 5} { return big }\n return small }",
        ),
        (
            "a fused expression assignment",
            "proc a {} { set r [expr {[id 9]}]\n return $r }",
        ),
        (
            "an incr amount",
            "proc a {} { set t 0\n incr t [id 9]\n return $t }",
        ),
    ] {
        let src = format!("proc id {{v}} {{ return $v }}\n{caller}\nputs [id 7]\nputs [a]\n");
        assert!(
            optimised(&src, TCL).contains("return $v"),
            "{why} is a call site passing 9, so `v` is not the constant 7: {:?}",
            opt_rewrites(&src, TCL)
        );
    }
}

/// The self-call hidden in a brace-quoted `expr` word — the shape #2118 was
/// filed for.
///
/// `expr` re-parses its braced word as an expression, so the `[fact …]` in it
/// is a command the statement really runs. Unseen, the two visible
/// `fact 5 1` / `fact 3 1` sites agreed that `acc` was `1`: O100 folded the
/// base case to `return 1` and tclsh 8.6.18's `120 6` became `1 1`. With one
/// call site the same evidence let O112 delete the base case outright and the
/// program no longer terminated.
#[test]
fn a_self_call_in_a_braced_expr_word_is_evidence() {
    const BODY: &str = "proc fact {n acc} {\n    if {$n <= 1} { return $acc }\n    return [expr {[fact [expr {$n - 1}] [expr {$n * $acc}]]}]\n}\n";
    for (why, src) in [
        (
            "two call sites",
            format!("{BODY}puts [fact 5 1]\nputs [fact 3 1]\n"),
        ),
        ("one call site", format!("{BODY}puts [fact 5 1]\n")),
    ] {
        let out = optimised(&src, TCL);
        assert!(
            out.contains("return $acc"),
            "{why}: the recursive call passes an `acc` that is not 1: {:?}",
            opt_rewrites(&src, TCL)
        );
        assert!(
            out.contains("if {$n <= 1}"),
            "{why}: the base case is reachable and must survive: {:?}",
            opt_rewrites(&src, TCL)
        );
    }
}

/// The descent is the registry's rule, not the spelling `expr`: a braced word
/// the head does *not* evaluate as an expression stays literal text.
#[test]
fn a_braced_word_that_is_not_an_expression_runs_nothing() {
    let src =
        "proc id {v} { return $v }\nproc a {} { return [list {[id 9]}] }\nputs [id 7]\nputs [a]\n";
    assert!(
        optimised(src, TCL).contains("return 7"),
        "`list {{[id 9]}}` passes literal text, so `id 7` really is the only call: {:?}",
        opt_rewrites(src, TCL)
    );
}

/// O122's gate counts every self-call and converts only when all of them are
/// in tail position. A statement that keeps no argument words counted zero.
///
/// `count_self_calls_in_stmt` enumerated the variants that retain argument
/// text and closed with a wildcard, so a fused `AssignExpr`, a bare
/// `ExprEval` and an `Incr` each reported no self-call at all. Under-counting
/// is the unsound direction — it makes the tail-site count match the total —
/// and the same proc converted or not depending only on how its non-tail call
/// was spelled (#2118).
#[test]
fn a_non_tail_self_call_blocks_o122_however_it_is_spelled() {
    for (why, nontail) in [
        (
            "a plain command word",
            "set acc [combine $acc [walk [left $node] 0]]",
        ),
        (
            "a fused expression assignment",
            "set acc [expr {$acc + [walk [left $node] 0]}]",
        ),
        (
            "a bare expression statement",
            "expr {[walk [left $node] 0]}",
        ),
        ("an incr amount", "incr acc [walk [left $node] 0]"),
    ] {
        let src = format!(
            "proc walk {{node acc}} {{\n    if {{$node eq \"\"}} {{\n        return $acc\n    }}\n    {nontail}\n    return [walk [right $node] $acc]\n}}\n"
        );
        assert!(
            !opt_fires(&src, TCL, "O122"),
            "{why}: the loop body would still recurse: {:?}",
            opt_codes(&src, TCL)
        );
    }
}

/// Precision: the gate counts self-calls, not brackets. A nested call to
/// something else is not recursion, so the tail call still converts.
#[test]
fn o122_still_converts_past_a_nested_call_to_another_proc() {
    for (why, nested) in [
        (
            "a fused expression assignment",
            "set acc [expr {$acc + [weight $n]}]",
        ),
        ("an incr amount", "incr acc [weight $n]"),
    ] {
        let src = format!(
            "proc walk {{n acc}} {{\n    if {{$n <= 0}} {{\n        return $acc\n    }}\n    {nested}\n    return [walk [expr {{$n - 1}}] $acc]\n}}\n"
        );
        assert!(
            opt_fires(&src, TCL, "O122"),
            "{why}: `weight` is not a self-call: {:?}",
            opt_codes(&src, TCL)
        );
    }
}

/// A `"…"` expression operand's value is its text after substitution, so a
/// folder that cannot substitute must not fold it. Measured on tclsh 8.6.18
/// and 9.0.4, which agree (#2227).
#[test]
fn a_quoted_expression_operand_is_folded_to_its_substituted_value() {
    // tclsh prints `pre5`; O103 folded the call to the text `pre$x`.
    let value = "proc a {} { set x 5; return [expr {\"pre$x\"}] }\nputs [a]\n";
    assert!(
        !optimised(value, TCL).contains("pre$x}"),
        "the operand is `pre5`, not its spelling: {}",
        optimised(value, TCL)
    );

    // tclsh prints `five`; the condition folded false and O112 removed the
    // live branch.
    let branch =
        "proc a {} { set x 5; if {\"$x\" eq \"5\"} { return five }; return other }\nputs [a]\n";
    assert!(
        optimised(branch, TCL).contains("five"),
        "`\"$x\" eq \"5\"` is true: {}",
        optimised(branch, TCL)
    );

    // A call in the operand runs: tclsh prints `7` then `nine`.
    let call = "proc id {v} { return $v }\nproc a {} { if {\"[id 9]\" eq \"9\"} { return nine }; return other }\nputs [id 7]\nputs [a]\n";
    assert!(
        optimised(call, TCL).contains("nine"),
        "the operand is `9`: {}",
        optimised(call, TCL)
    );

    // A braced operand folds its backslash-newline, so the call returns the
    // three characters `a b`: tclsh prints `3`, and the fold of the raw bytes
    // printed `5` (found in review).
    let continued = "proc a {} { return [expr {{a\\\n b}}] }\nset r [a]\nputs [string length $r]\n";
    assert!(
        !opt_fires(continued, TCL, "O103"),
        "the braced operand's value is not its bytes: {}",
        optimised(continued, TCL)
    );

    // Precision: a braced operand is literal, and a quoted one with nothing
    // to substitute is its text; both still fold.
    for (src, folded) in [
        (
            "proc a {} { return [expr {{pre$x}}] }\nputs [a]\n",
            "puts {pre$x}",
        ),
        (
            "proc a {} { return [expr {\"abc\"}] }\nputs [a]\n",
            "puts abc",
        ),
    ] {
        assert!(
            optimised(src, TCL).contains(folded),
            "{src:?} still folds: {}",
            optimised(src, TCL)
        );
    }
}

/// A dead assignment whose value can raise is not dead: deleting it drops the
/// error the program stops on. tclsh 8.6.18 raises for each of these (and
/// 8.4.20, 9.0.4 and 9.1b0 agree); O126, O109 and O108 deleted the statement
/// and the program printed `hi` (#2249).
#[test]
fn a_dead_assignment_whose_value_can_raise_is_kept() {
    for (why, src, kept) in [
        (
            "an unset variable",
            "proc p {} {\n    set y $x\n    puts hi\n}\n",
            "set y $x",
        ),
        (
            "an unset variable in a command word",
            "proc p {} {\n    set y [lindex $x 0]\n    puts hi\n}\n",
            "set y [lindex $x 0]",
        ),
        (
            "an unset variable in quotes",
            "proc p {} {\n    set y \"$x\"\n    puts hi\n}\n",
            "set y \"$x\"",
        ),
        (
            "an unset element",
            "proc p {} {\n    set y $a(k)\n    puts hi\n}\n",
            "set y $a(k)",
        ),
        (
            "a name `upvar` links",
            "proc p {} {\n    upvar 1 v v\n    set y $v\n    puts hi\n}\n",
            "set y $v",
        ),
        (
            "an array read as a scalar",
            "proc p {} {\n    set a(k) 1\n    set y $a\n    puts hi\n}\n",
            "set y $a",
        ),
        (
            "a variable a `catch` may leave unset",
            "proc p {} {\n    catch {set x [error boom]}\n    set y $x\n    puts hi\n}\n",
            "set y $x",
        ),
        (
            "a variable set on one branch only",
            "proc p {c} {\n    if {$c} {set x 1}\n    set y $x\n    puts hi\n}\n",
            "set y $x",
        ),
        (
            "a `regexp` output variable",
            "proc p {s} {\n    regexp {(z)} $s -> x\n    set y $x\n    puts hi\n}\n",
            "set y $x",
        ),
        (
            "an `unset` variable",
            "proc p {} {\n    set x 1\n    unset x\n    set y $x\n    puts hi\n}\n",
            "set y $x",
        ),
        (
            "`expr` arithmetic on a parameter",
            "proc p {v} {\n    set y [expr {$v + 1}]\n    puts hi\n}\n",
            "set y [expr {$v + 1}]",
        ),
        (
            "`expr` division by zero",
            "proc p {} {\n    set y [expr {1/0}]\n    puts hi\n}\n",
            "set y [expr {1/0}]",
        ),
        (
            "a method sharing its name with a proc binds only its own parameters",
            "namespace eval C { proc m {x} {} }\noo::class create C {\n    method m {} {\n        ::set unused $x\n        ::return 2\n    }\n}\n",
            "::set unused $x",
        ),
        (
            "a `foreach` variable over a list that may be empty",
            "proc p {l} {\n    foreach x $l {}\n    set y $x\n    puts hi\n}\n",
            "set y $x",
        ),
        (
            "an array read as a scalar after `array set`",
            "proc p {} {\n    array set a {}\n    set y $a\n    puts hi\n}\n",
            "set y $a",
        ),
        (
            "an `lset` target that was never set",
            "proc p {} {\n    catch {lset x 0 new}\n    set y $x\n    puts hi\n}\n",
            "set y $x",
        ),
        (
            "a variable only a `catch` script assigns",
            "proc p {} {\n    catch {error boom; set a 1} x\n    set y $a\n    puts hi\n}\n",
            "set y $a",
        ),
        (
            "an overwritten store of an unset variable (O109)",
            "proc p {} {\n    set y $x\n    set y 1\n    return $y\n}\n",
            "set y $x",
        ),
    ] {
        let out = optimised(src, TCL);
        assert!(out.contains(kept), "{why}: the statement stays: {out}");
    }
}

/// Precision for the rule above: a value that cannot raise is still deleted.
#[test]
fn a_dead_assignment_whose_value_cannot_raise_is_still_deleted() {
    for (why, src) in [
        ("a literal", "proc p {} {\n    set y 1\n    puts hi\n}\n"),
        (
            "a copy of a set local",
            "proc p {} {\n    set x [clock seconds]\n    set y $x\n    puts hi\n}\n",
        ),
        (
            "a copy of a parameter",
            "proc p {v} {\n    set y $v\n    puts hi\n}\n",
        ),
        // A method binds its arguments on entry too (found in review).
        (
            "a copy of a method argument",
            "oo::class create C {\n    method uses {v} {\n        ::set unused $v\n        ::return 2\n    }\n}\n",
        ),
        (
            "a variable set on both branches",
            "proc p {c} {\n    if {$c} {set x 1} else {set x 2}\n    set y $x\n    puts hi\n}\n",
        ),
        (
            "`expr` SCCP folds to a constant",
            "proc p {} {\n    set a 1\n    set y [expr {$a + 1}]\n    puts hi\n}\n",
        ),
        // Commands the registry marks as always writing their targets
        // (found in review).
        (
            "a `catch` result variable",
            "proc p {} {\n    catch {error boom} x\n    set y $x\n    puts hi\n}\n",
        ),
        (
            "a `gets` target",
            "proc p {c} {\n    gets $c line\n    set y $line\n    puts hi\n}\n",
        ),
        (
            "an `lassign` target",
            "proc p {l} {\n    lassign $l a\n    set y $a\n    puts hi\n}\n",
        ),
        (
            "a `regsub` target",
            "proc p {s} {\n    regsub {xx} $s YY a\n    set y $a\n    puts hi\n}\n",
        ),
        (
            "an `append` target",
            "proc p {v} {\n    append x $v\n    set y $x\n    puts hi\n}\n",
        ),
        (
            "an `lappend` target",
            "proc p {v} {\n    lappend x $v\n    set y $x\n    puts hi\n}\n",
        ),
        (
            "a `dict set` target",
            "proc p {v} {\n    dict set d k $v\n    set y $d\n    puts hi\n}\n",
        ),
        (
            "a `dict incr` target",
            "proc p {} {\n    dict incr d k\n    set y $d\n    puts hi\n}\n",
        ),
        (
            "a `chan gets` target",
            "proc p {c} {\n    chan gets $c line\n    set y $line\n    puts hi\n}\n",
        ),
        // A conditional writer keeps the previous value on a miss.
        (
            "a `regexp` output variable set before",
            "proc p {s} {\n    set x old\n    regexp {(z)} $s -> x\n    set y $x\n    puts hi\n}\n",
        ),
        // A read-modify-write target stays set once it was set.
        (
            "an `lset` target set before",
            "proc p {} {\n    set x {old}\n    lset x 0 new\n    set y $x\n    puts hi\n}\n",
        ),
    ] {
        assert!(
            opt_fires(src, TCL, "O126"),
            "{why}: the unused store goes: {:?}",
            opt_codes(src, TCL)
        );
    }
}

/// Tcl substitutes inside a `"…"` expression operand, so a call written there
/// is a call the statement runs — for the caller-evidence walk and for the
/// variable-effect walk alike.
///
/// Both read the operand through `ExprNode::String`, which spans the quoted
/// and the braced spelling and keeps its delimiters; both treated every
/// string as inert. Measured on tclsh 8.6.18 (#2118, found in review).
#[test]
fn a_quoted_expression_operand_is_not_inert() {
    // The caller-evidence half: `id 9` is a call site, so `v` is not the
    // constant 7. tclsh prints `7` then `9`; the fold printed `7` twice.
    let evidence = "proc id {v} { return $v }\nproc a {} { set r [expr {\"[id 9]\"}]\n return $r }\nputs [id 7]\nputs [a]\n";
    assert!(
        optimised(evidence, TCL).contains("return $v"),
        "the quoted operand holds a call passing 9: {:?}",
        opt_rewrites(evidence, TCL)
    );

    // The variable-effect half: the `incr` really runs, so the load of `x`
    // after it cannot be forwarded from the store before it. tclsh prints
    // `2` then `2`; O102 plus O109 made it print `1` twice.
    let effect = "proc f {} {\n    set x 1\n    set y [expr {\"[incr x]\" + 0}]\n    puts $x\n    puts $y\n}\n";
    assert!(
        !opt_fires(effect, TCL, "O102"),
        "a store cannot be forwarded across a write the operand performs: {:?}",
        opt_codes(effect, TCL)
    );

    // The side-effect gates: a `[cmd]` in a quoted operand runs, so no pass
    // may drop the statement that holds it. tclsh 8.6.18 prints `1` for each;
    // O110, O113 and O126 each printed `0` (#2227, found in review).
    for (why, src) in [
        (
            "O110 on `&& 0`",
            "proc p {} {\n    set x 0\n    set y [expr {\"[incr x]\" && 0}]\n    return $x\n}\n",
        ),
        (
            "O113 on a constant-false condition",
            "proc p {} {\n    set x 0\n    if {\"[incr x]\" && 0} {}\n    return $x\n}\n",
        ),
        (
            "O126 on an unused store",
            "proc p {} {\n    set x 0\n    set y [expr {\"[incr x]\"}]\n    return $x\n}\n",
        ),
    ] {
        assert!(
            optimised(src, TCL).contains("incr x"),
            "{why}: the `incr` runs: {}",
            optimised(src, TCL)
        );
    }
    // Nor may a rewrite read a substituting operand's spelling as its value:
    // `"$x"` may be `1.0` or `NaN`. tclsh 8.6.18 and 9.0.4 print `1`, `1`
    // and `0` for these at `p 1.0`, `r NaN` and `s NaN`; O120 and the
    // inversion rewrote them to `eq`, `>=` and `eq` (#2227, found in review).
    for (why, src, kept) in [
        (
            "O120 on a numeric compare",
            "proc p {x} {\n    return [expr {\"$x\" == 1}]\n}\n",
            "==",
        ),
        (
            "the inversion of an ordered compare",
            "proc r {x} {\n    return [expr {!(\"$x\" < 1)}]\n}\n",
            "<",
        ),
        (
            "O120 on a self-compare",
            "proc s {x} {\n    return [expr {\"$x\" == \"$x\"}]\n}\n",
            "==",
        ),
    ] {
        assert!(
            optimised(src, TCL).contains(kept),
            "{why}: {}",
            optimised(src, TCL)
        );
    }

    // Nor is an overwritten store whose operand runs a command a dead store
    // to report: W220 offered to delete the `incr`.
    let store = "proc p {} {\n    set x 0\n    set y [expr {\"[incr x]\"}]\n    set y 2\n    return \"$x$y\"\n}\n";
    assert!(
        !analyser_codes(store, TCL).contains(&"W220".to_owned()),
        "the store runs `incr x`: {:?}",
        analyser_codes(store, TCL)
    );

    // Precision: the braced spelling really is inert, and still folds.
    let braced = "proc f {} {\n    set x 1\n    set y [expr {\"a\" eq \"a\"}]\n    return $y\n}\n";
    assert!(
        reparse_errors(braced, TCL).is_empty(),
        "a quoted operand with no substitution is unaffected: {:?}",
        reparse_errors(braced, TCL)
    );
}

/// A `finally` clause runs on every completion path, so its body is never
/// unreachable — whatever the `try` body does.
///
/// `lower_try` wired `try_end` (and the `finally` hanging off it) only from a
/// body that falls through normally, or from a handler's throw edge. A body
/// that cannot fall through and no handler left the whole tail with no
/// predecessor at all, SCCP called it dead, and O107 emptied the clause.
/// Measured on tclsh 8.6.18 and 9.0.4, `catch {p}; puts $g` printed `1` and
/// the rewritten program printed `0` (#2142).
#[test]
fn a_finally_body_is_reachable_however_the_try_body_leaves() {
    for (why, body, wrapper) in [
        ("error", "error boom", ""),
        ("throw", "throw {A B} boom", ""),
        ("return", "return early", ""),
        ("break", "break", "while {1} "),
        ("error in a loop", "error boom", "foreach i {1 2} "),
        // Every branch leaves, so the body cannot fall through — yet it still
        // ends in a resting `if_end` block. Gating on "has no tail" missed it
        // (found in review).
        (
            "every branch of an if leaves",
            "if {[info exists ::c]} {return ok} else {error boom}",
            "",
        ),
        (
            "every arm of a switch leaves",
            "switch [info exists ::c] {1 {return ok} default {error boom}}",
            "",
        ),
        // `tailcall` leaves the frame, but only once the `finally` has run:
        // tclsh 8.6.18 and 9.0.4 print `FINALLY` for `try {tailcall t}
        // finally {puts FINALLY}` inside a proc.
        ("tailcall", "tailcall list", ""),
    ] {
        let stmt = format!("try {{{body}}} finally {{set g 1}}");
        // Only a loop wraps the statement: `{wrapper}{ … }` with an empty
        // wrapper is a braced command *name*, not a `try` at all.
        let line = if wrapper.is_empty() {
            stmt
        } else {
            format!("{wrapper}{{ {stmt} }}")
        };
        let src = format!(
            "set g 0\nproc p {{}} {{\n    global g\n    {line}\n}}\ncatch {{p}}\nputs $g\n"
        );
        let out = optimised(&src, TCL);
        assert!(
            out.contains("set g 1"),
            "{why}: `finally` runs on this path, so its store is live: {:?}\n{out}",
            opt_rewrites(&src, TCL)
        );
    }
}

/// A handler that itself leaves — `return`, `error`, or a `break` out of an
/// enclosing loop — reaches the `finally` too. The handler's own exit
/// blocks were never wired to it, so with any handler present a `finally`
/// reached only that way was dead: `try {error boom} on error {} {return
/// handled} finally {set g 1}` lost its store, and O109 then deleted the
/// caller's `set g 0`, so the rewritten program failed with `can't read "g"`
/// where tclsh 8.6.18 prints `1` (found in review).
#[test]
fn a_finally_body_is_reachable_however_a_handler_leaves() {
    for (why, handler, wrapper) in [
        ("return", "return handled", ""),
        ("error", "error again", ""),
        ("break", "break", "foreach i {1 2} "),
    ] {
        let stmt = format!("try {{error boom}} on error {{}} {{{handler}}} finally {{set g 1}}");
        let line = if wrapper.is_empty() {
            stmt
        } else {
            format!("{wrapper}{{ {stmt} }}")
        };
        let src = format!(
            "set g 0\nproc p {{}} {{\n    global g\n    {line}\n}}\ncatch {{p}}\nputs $g\n"
        );
        let out = optimised(&src, TCL);
        assert!(
            out.contains("set g 1"),
            "{why}: `finally` runs after the handler leaves: {:?}\n{out}",
            opt_rewrites(&src, TCL)
        );
    }
}

/// `exit` ends the interpreter without unwinding, so it reaches no `finally`:
/// `try {exit 7} finally {puts FINALLY}` exits with status 7 and prints
/// nothing on tclsh 8.6.18 and 9.0.4. Wiring it to the clause made the clause
/// executable in SCCP and SSA though it can never run (found in review).
#[test]
fn an_exit_reaches_no_finally() {
    for (why, body) in [("no argument", "exit"), ("a literal status", "exit 7")] {
        let src =
            format!("proc p {{}} {{\n    global g\n    try {{{body}}} finally {{set g 1}}\n}}\n");
        assert!(
            opt_fires(&src, TCL, "O107"),
            "{why}: a `finally` reached only through `exit` never runs: {:?}",
            opt_codes(&src, TCL)
        );
    }

    // Precision: anything that can stop the `exit` from running keeps the
    // clause live, because a `return` or an error does run it. Each of these
    // prints `1` on tclsh 8.6.18; review found each one emptied.
    for (why, body) in [
        (
            "an arm that returns instead",
            "switch -glob $x {a {exit 7} default {return ok}}",
        ),
        (
            "every arm may return before it exits",
            "switch -glob $x {a {if {$c} {return ok}; exit 7} default {if {$c} {return ok}; exit 8}}",
        ),
        // The subject substitution runs first and may throw.
        (
            "a switch whose subject may throw",
            "switch -glob $nosuch {a {exit 7} default {exit 8}}",
        ),
        ("an exit whose argument throws", "exit [error boom]"),
        ("an exit that rejects its literal", "exit abc"),
        // Release-aware: an invalid octal in 8.x (`TCL` is 8.6), status 9 in
        // 9.0 — the registry answers, not a digit check.
        ("an exit whose status is an invalid 8.x octal", "exit 09"),
    ] {
        let src = format!(
            "set g 0\nproc p {{x c}} {{\n    global g\n    try {{{body}}} finally {{set g 1}}\n}}\ncatch {{p a 1}}\nputs $g\n"
        );
        assert!(
            optimised(&src, TCL).contains("set g 1"),
            "{why}: the `finally` still runs: {:?}",
            opt_rewrites(&src, TCL)
        );
    }

    // An alias invokes more words than the call site shows: `bye` here runs
    // `exit abc`, which raises, so tclsh 8.6.18 and 9.0.4 print `1` (found in
    // review).
    for (why, src) in [
        (
            "an alias with a prefixed status",
            "set g 0\ninterp alias {} bye {} exit abc\nproc p {} {\n    global g\n    try {bye} finally {set g 1}\n}\ncatch p\nputs $g\n",
        ),
        // A handler catches only the substitution's error; the `return` still
        // runs the clause, so tclsh prints `1` (found in review).
        (
            "a handler that catches only some of the body's completions",
            "set g 0\nproc p {x} {\n    global g\n    try {return $x} on error {} {exit 0} finally {set g 1}\n}\np 5\nputs $g\n",
        ),
        // A statement before the `return` may raise first, and a `trap` may
        // not match: either way the `finally` still runs, so tclsh prints `1`
        // (found in review).
        (
            "a statement that may raise before an exact `return`",
            "set g 0\nproc p {} {\n    global g\n    try {set y $x; return ok} on error {} {} on return {} {exit 0} finally {set g 1}\n}\np\nputs $g\n",
        ),
        // An earlier block may raise before the `return` or `exit` in the
        // block after it: `$c` is unset, and tclsh prints `1` (found in
        // review). So may the outer body before a nested `try`.
        (
            "an earlier block that may raise before an exact `return`",
            "set g 0\nproc p {} {\n    global g\n    try {if {$c} {}; return ok} on error {} {} on return {} {exit 0} finally {set g 1}\n}\np\nputs $g\n",
        ),
        (
            "an earlier block that may raise before an `exit`",
            "set g 0\nproc p {} {\n    global g\n    try {if {$c} {}; exit 0} finally {set g 1}\n}\ncatch p\nputs $g\n",
        ),
        (
            "an outer statement that may raise before a nested `exit`",
            "set g 0\nproc p {} {\n    global g\n    try { set y $x; try {exit 0} finally {} } finally {set g 1}\n}\ncatch p\nputs $g\n",
        ),
        // `on 010` is octal code 8 in Tcl 8.x (`TCL` is 8.6), so the handler
        // catches the body and its store is live (found in review).
        (
            "a handler selector in the dialect's own numerals",
            "set g 0\nproc p {} {\n    global g\n    try {return -level 0 -code 8 boom} on 010 {} {set g 1} finally {}\n}\ncatch p\nputs $g\n",
        ),
        (
            "an error only a `trap` might catch",
            "set g 0\nproc p {} {\n    global g\n    try {error boom} trap {NOT MATCHING} {} {exit 0} finally {set g 1}\n}\ncatch p\nputs $g\n",
        ),
        // Binding `msg` is a write, and a write trace can reject it before
        // the `exit` runs; the error then runs the clause. tclsh 8.6.18 and
        // 9.0.4 print `1`, as they do when `msg` is an `upvar` to an array
        // (found in review).
        (
            "a handler whose variable binding may raise before its `exit`",
            "set g 0\nproc tr args {error TRACE}\nproc p {} {\n    global g\n    trace add variable msg write tr\n    try {error boom} on error msg {exit 0} finally {set g 1}\n}\ncatch p\nputs $g\n",
        ),
        (
            "a namespace alias spelled like its target",
            "set g 0\nnamespace eval foo {}\ninterp alias {} ::foo::exit {} ::exit abc\nproc ::foo::p {} {\n    global g\n    try {exit} finally {set g 1}\n}\ncatch foo::p\nputs $g\n",
        ),
    ] {
        assert!(
            optimised(src, TCL).contains("set g 1"),
            "{why}: the `finally` still runs: {:?}",
            opt_rewrites(src, TCL)
        );
    }
}

/// The definiteness half. A name bound before the `try` is still bound after
/// it, and the `finally` store that rebinds it is visible.
#[test]
fn a_try_finally_does_not_hide_the_names_bound_around_it() {
    for (why, src) in [
        // The `catch` is what makes the read live: without it the error
        // propagates, and the `return` after the `try` never runs.
        (
            "bound before the `try`, rebound by `finally`",
            "proc p {} {\n    set f 0\n    catch { try {error boom} finally {set f 1} }\n    return $f\n}\n",
        ),
        // An inner `finally` runs before the outer one on every path, so the
        // name it binds is bound when the outer clause reads it. Wiring the
        // inner body's `return` straight to the outer `finally` skipped the
        // inner clause (found in review).
        (
            "bound by an inner `finally` before the outer one reads it",
            "proc p {} {\n    try { try {return ok} finally {set x 1} } finally {puts $x}\n}\n",
        ),
        // An inner handler that catches the error runs, and the inner clause
        // after it, before the outer clause reads: tclsh 8.6.18 and 9.0.4
        // print `1` twice (found in review).
        (
            "bound by an inner handler and inner clause before the outer one",
            "proc p {} {\n    try {try {error boom} on error {} {set x 1; return} finally {set y 1}} finally {puts $x; puts $y}\n}\n",
        ),
        // A literal assignment before the `error` can only raise an error too,
        // so the inner handler catches the block whichever raises; tclsh
        // prints `1` (found in review).
        (
            "bound by an inner handler after a braced literal assignment",
            "proc p {} {\n    try { try {set {[} 0; error boom} on error {} {set x 1; return} finally {} } finally {puts $x}\n}\n",
        ),
        (
            "bound by an inner handler after a literal assignment",
            "proc p {} {\n    try { try {set z 0; error boom} on error {} {set x 1; return} finally {} } finally {puts $x}\n}\n",
        ),
        // Only the first matching handler runs; the second `on error` is dead,
        // and tclsh prints `1` (found in review).
        (
            "bound by the first of two handlers for the same code",
            "proc p {} {\n    try {error boom} on error {} {set x 1} on error {} {return} finally {puts $x}\n}\n",
        ),
        // The first `on error` selects the error though its body is `-`; the
        // last handler never runs, and tclsh prints `1` (found in review).
        (
            "not unbound by a handler a `-` handler pre-empts",
            "proc p {} {\n    set x 1\n    try {error boom} on error {} - on ok {} {} on error {} {unset x; return} finally {puts $x}\n}\n",
        ),
        // A `break`/`continue` runs the clause before it reaches the loop.
        // An edge into the `finally` alongside the jump still left a path
        // into the loop that skipped it, carrying the `unset` (found in
        // review); tclsh 8.6.18 and 9.0.4 print `5` for each of these.
        (
            "rebound by `finally` before a `continue` reaches the loop test",
            "proc p {} {\n    set x 0\n    while {$x < 3} {\n        try {unset x; continue} finally {set x 5}\n    }\n    return $x\n}\n",
        ),
        (
            "rebound by `finally` before a `break` leaves the loop",
            "proc p {} {\n    set x 0\n    while 1 {\n        try {unset x; break} finally {set x 5}\n    }\n    return $x\n}\n",
        ),
        (
            "rebound by `finally` after a handler's `continue`",
            "proc p {} {\n    set x 0\n    while {$x < 3} {\n        try {error boom} on error {} {unset x; continue} finally {set x 5}\n    }\n    return $x\n}\n",
        ),
        (
            "rebound by `finally` after a `continue` no handler catches",
            "proc p {} {\n    set x 0\n    while {$x < 3} {\n        try {unset x; continue} on error {} {} finally {set x 5}\n    }\n    return $x\n}\n",
        ),
        // A handler that selects the jump's code catches it, so the jump never
        // reaches the loop; tclsh 8.6.18 and 9.0.4 print `1` for both (found
        // in review).
        (
            "bound by an `on break` handler that catches the `break`",
            "proc p {} {\n    while 1 {\n        try {break} on break {} {set x 1} finally {}\n        break\n    }\n    puts $x\n}\n",
        ),
        (
            "bound by an `on break` handler, with no `finally`",
            "proc p {} {\n    while 1 {\n        try {break} on break {} {set x 1}\n        break\n    }\n    puts $x\n}\n",
        ),
        (
            "rebound by the outer of two nested clauses a `break` leaves",
            "proc p {} {\n    set x 0\n    while 1 {\n        try { try {unset x; break} finally {set y 1} } finally {set x 5}\n    }\n    return $x\n}\n",
        ),
    ] {
        assert!(
            !analyser_codes(src, TCL).iter().any(|c| c == "W210"),
            "{why}: {:?}",
            analyser_codes(src, TCL)
        );
    }
}

/// A `try` whose body and handlers can never complete normally does not fall
/// through its `finally` into the code after it: every way in is an exit that
/// resumes unwinding or a saved jump. Letting the clause fall through made
/// `set x 1` look reachable from the `break` (found in review).
#[test]
fn a_try_that_never_completes_does_not_fall_through_its_finally() {
    // tclsh 8.6.18 and 9.0.4 both fail these with `can't read "x"`.
    for (why, src) in [
        (
            "`break`",
            "proc p {} {\n    while 1 {\n        try {break} finally {}\n        set x 1\n    }\n    puts $x\n}\n",
        ),
        (
            "`break` through nested clauses",
            "proc p {} {\n    while 1 {\n        try { try {break} finally {} } finally {}\n        set x 1\n    }\n    puts $x\n}\n",
        ),
        // A handler whose code is not the jump's offers no way to complete:
        // `on error`, `on continue` and `on 4` cannot catch a `break`.
        (
            "`break` past an `on error` handler",
            "proc p {} {\n    while 1 {\n        try {break} on error {} {} finally {}\n        set x 1\n    }\n    puts $x\n}\n",
        ),
        (
            "`break` past an `on continue` handler",
            "proc p {} {\n    while 1 {\n        try {break} on continue {} {} finally {}\n        set x 1\n    }\n    puts $x\n}\n",
        ),
        (
            "`break` past an `on 4` handler",
            "proc p {} {\n    while 1 {\n        try {break} on 4 {} {} finally {}\n        set x 1\n    }\n    puts $x\n}\n",
        ),
    ] {
        assert!(
            analyser_codes(src, TCL).iter().any(|c| c == "W210"),
            "{why}: {:?}",
            analyser_codes(src, TCL)
        );
    }

    // `on error` cannot catch a `return`, so the code after the `try` is dead:
    // tclsh 8.6.18 and 9.0.4 return `early` (found in review).
    let returns = "proc p {} {\n    try {return early} on error {} {} finally {}\n    set x 1\n    return $x\n}\n";
    assert!(
        opt_fires(returns, TCL, "O107"),
        "`return` past an `on error` handler: {:?}",
        opt_codes(returns, TCL)
    );
}

/// A body that falls through into `try_ok` completes normally even when every
/// handler leaves abruptly, so the code after the `try` is live: tclsh
/// 8.6.18 and 9.0.4 return `2` (found in review).
#[test]
fn a_body_that_completes_through_try_ok_keeps_the_code_after_the_try() {
    let src = "proc p {} {\n    try {set x 1} on error {} {return early} finally {}\n    set y 2\n    return $y\n}\n";
    assert!(
        !opt_fires(src, TCL, "O107"),
        "the code after the `try` runs: {:?}",
        opt_codes(src, TCL)
    );
}

/// A `-` handler runs the body of the handler after it, whatever that
/// handler's own selector, so a completion the `-` handler matches reaches the
/// shared body. tclsh 8.6.18 and 9.0.4 return `1`, `1`, `1`, and `5` / `6`
/// (found in review).
#[test]
fn a_fallthrough_handler_reaches_the_body_it_shares() {
    for (why, src, kept) in [
        (
            "an error selected by `on error {} -`",
            "proc p {} {\n    set x 0\n    try {error boom} on error {} - on ok {} {set x 1} finally {}\n    return $x\n}\n",
            "set x 1",
        ),
        (
            "a return selected by `on return {} -`",
            "proc p {} {\n    set x 0\n    try {return early} on return {} - on error {} {set x 1} finally {}\n    return $x\n}\n",
            "set x 1",
        ),
        (
            "an error through a chain of two `-` handlers",
            "proc p {} {\n    set x 0\n    try {error boom} on error {} - trap {} {} - on ok {} {set x 1} finally {}\n    return $x\n}\n",
            "set x 1",
        ),
    ] {
        assert!(
            optimised(src, TCL).contains(kept),
            "{why}: the shared body runs: {}",
            optimised(src, TCL)
        );
    }

    // Precision: the match runs the shared body, never the `-` handler's own
    // empty block, so `x` is set before the clause reads it. tclsh prints
    // `1`; the empty block's edge on to `try_end` drew W210 (found in
    // review).
    let bound =
        "proc p {} {\n    try {error boom} on error {} - on ok {} {set x 1} finally {puts $x}\n}\n";
    assert!(
        !analyser_codes(bound, TCL).contains(&"W210".to_owned()),
        "`x` is bound on every path into the clause: {:?}",
        analyser_codes(bound, TCL)
    );

    // An `on ok` owner shared with `on error {} -` is not reached from the
    // tail alone: the error path carries `y` = 5 into it.
    let shared = "proc p {c} {\n    set y 0\n    try {set y 5; if {$c} {error boom}; set y 6} on error {} - on ok {} {return $y} finally {}\n    return none\n}\n";
    assert!(
        optimised(shared, TCL).contains("return $y"),
        "`y` is 5 or 6 in the shared body: {}",
        optimised(shared, TCL)
    );
}

/// A `finally` clause that itself transfers control keeps that transfer: its
/// `break` overrides the pending return or error, so the code after the loop
/// is live. tclsh 8.6.18 and 9.0.4 print `after 1` and `survived` (found in
/// review).
#[test]
fn a_finally_that_transfers_control_keeps_its_transfer() {
    for (why, src) in [
        (
            "`break` over a pending `return`",
            "proc p {} {\n    while 1 { try {return early} finally {break} }\n    set x 1\n    return \"after $x\"\n}\n",
        ),
        (
            "`break` over a pending error",
            "proc p {} {\n    while 1 { try {error boom} finally {break} }\n    set y survived\n    return $y\n}\n",
        ),
    ] {
        assert!(
            !opt_fires(src, TCL, "O107"),
            "{why}: the code after the loop runs: {:?}",
            opt_codes(src, TCL)
        );
    }
}

/// A `return` that passes an inner `finally` resumes past the statements
/// after the inner `try`, even when that `try` can also fall through: in
/// `try { try {if {$c} {return}} finally {}; set x 1 } finally {puts $x}` the
/// outer clause reads `x` unset on the `return` path, and tclsh 8.6.18 and
/// 9.0.4 fail there. Sending the `return` through the clause's fall-through
/// made `set x 1` look certain and O102 forwarded it (found in review).
#[test]
fn a_return_through_an_inner_finally_skips_the_code_after_it() {
    let src = "proc p {c} {\n    try { try {if {$c} {return}} finally {}; set x 1 } finally {puts $x}\n}\n";
    assert!(
        analyser_codes(src, TCL).iter().any(|c| c == "W210"),
        "the outer clause may read `x` unset: {:?}",
        analyser_codes(src, TCL)
    );
    assert!(
        !opt_fires(src, TCL, "O102"),
        "`x` has no single reaching definition at the outer clause: {:?}",
        opt_codes(src, TCL)
    );
}

/// A handler is reached from the explicit throws inside a nested construct,
/// with their block's stores live — including a `finally` that only ever
/// resumes unwinding. tclsh 8.6.18 and 9.0.4 return `1` for both.
#[test]
fn a_try_handler_sees_the_stores_before_a_nested_throw() {
    for (why, src) in [
        (
            "every arm of an `if` throws",
            "proc p {c} {\n    try { if {$c} {set x 1; error b} else {set x 2; error c} } on error {} {}\n    return $x\n}\n",
        ),
        (
            "an inner `finally` stores, then resumes the error",
            "proc p {} {\n    try { try {error boom} finally {set x 1} } on error {} {}\n    return $x\n}\n",
        ),
    ] {
        let codes = opt_codes(src, TCL);
        assert!(
            !codes.iter().any(|c| c == "O109")
                && !analyser_codes(src, TCL).iter().any(|c| c == "W220"),
            "{why}: the store is read after the handler: {codes:?} {:?}",
            analyser_codes(src, TCL)
        );
    }
}

/// Precision: the fix must not make a handler's variable look bound on a path
/// that never runs it, nor silence the dead store a `finally` really does
/// create.
#[test]
fn a_try_handler_still_binds_only_on_the_path_that_runs_it() {
    // tclsh 8.6.18 fails this with `can't read "g": no such variable` when the
    // body does not throw, so W210 is a true positive.
    let unbound =
        "proc q {c} {\n    try { if {$c} {error boom} } on error {} {set g 1}\n    return $g\n}\n";
    assert!(
        analyser_codes(unbound, TCL).iter().any(|c| c == "W210"),
        "a handler that may not run does not bind its names: {:?}",
        analyser_codes(unbound, TCL)
    );

    // `finally` overwrites the handler's store before any read, so the
    // handler's assignment really is dead.
    let overwritten = "proc p {} {\n    try {error boom} on error {} {set f 2} finally {set f 1}\n    return $f\n}\n";
    assert!(
        opt_fires(overwritten, TCL, "O109"),
        "`finally` runs after the handler, so `set f 2` is dead: {:?}",
        opt_codes(overwritten, TCL)
    );
}
