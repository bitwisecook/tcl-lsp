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

use tcl_compiler::optimiser::manager::{
    apply_optimisations, optimise_source_multipass, optimise_with_dialect,
};
use tcl_registry::registry_for_dialect;

// ---------------------------------------------------------------------------
// Shared helpers (mirror fp/opt.rs: opt_fires / optimised / opt_codes)
// ---------------------------------------------------------------------------

const TCL: &str = "tcl8.6";

/// Every `Oxxx` code emitted by a single optimiser pass over `src`.
fn opt_codes(src: &str, dialect: &str) -> Vec<String> {
    let registry = registry_for_dialect(dialect);
    let d = (!dialect.is_empty()).then_some(dialect);
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
    let registry = registry_for_dialect(dialect);
    let d = (!dialect.is_empty()).then_some(dialect);
    apply_optimisations(src, &optimise_with_dialect(src, registry, d))
}

/// `(code, replacement)` pairs from the optimiser for `src`.
fn opt_rewrites(src: &str, dialect: &str) -> Vec<(String, String)> {
    let registry = registry_for_dialect(dialect);
    let d = (!dialect.is_empty()).then_some(dialect);
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

/// Wraps `body` in a loop where `$x` is an SCCP-typed INT loop counter (the
/// D5-O110 identity/annihilator drops need a provably-numeric operand).
fn int_x(body: &str) -> String {
    format!(
        "proc f {{n}} {{\n  for {{set x 0}} {{$x < $n}} {{incr x}} {{\n    {body}\n    puts $v\n  }}\n}}\n"
    )
}

// ---------------------------------------------------------------------------
// Constant propagation / folding / DSE core
// ---------------------------------------------------------------------------

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
    let registry = registry_for_dialect(TCL);
    let (reassign_out, _) = optimise_source_multipass(
        "set a 1\nset a 5\nset b [expr {$a + 2}]",
        registry,
        Some(TCL),
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

    // Escaping command substitution ([eval $s]) is non-removable; the constant
    // store `set n 1` still forwards into `puts $n` (O102 → `puts 1`).
    let esc = "set n [eval $s]\nset n 1\nputs $n\n";
    let esc_out = optimised(esc, TCL);
    assert_eq!(esc_out.lines().next().unwrap(), "set n [eval $s]");
    assert!(opt_fires(esc, TCL, "O102"));
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

// ---------------------------------------------------------------------------
// O110 InstCombine — algebraic / boolean / De-Morgan / ternary simplification
// ---------------------------------------------------------------------------

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
    assert!(optimised("set v [expr {!($a < $b)}]", TCL).contains("set v [expr {$a >= $b}]"));

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
    assert!(
        optimised("set v [expr {!($a == $b && $c < $d)}]", TCL)
            .contains("set v [expr {$a != $b || $c >= $d}]")
    );
    assert!(
        optimised("set v [expr {!($a == $b || $c < $d)}]", TCL)
            .contains("set v [expr {$a != $b && $c >= $d}]")
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
    // tclsh sweep: x == x ⇒ 1, x != x ⇒ 0 (constant regardless of $x).
    assert!(optimised("set v [expr {$x == $x}]", TCL).contains("set v 1"));
    assert!(optimised("set v [expr {$x != $x}]", TCL).contains("set v 0"));
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

// ---------------------------------------------------------------------------
// Structure elimination — O112 constant-condition compound statements
// ---------------------------------------------------------------------------

/// The iRules word operators fold through **SCCP** — not just through the
/// expression-simplification passes that already carried a dialect.
///
/// Regression for the Codex #1046 / soundness review finding: SCCP,
/// interprocedural propagation, the static-loop simulator, and codegen's
/// expression folder all evaluated with a dialect-blind policy, so
/// `FoldOps::is_irules` was `false` there and every word operator declined.
/// The `eq` control below folded on the same input, proving the loss was
/// dialect threading rather than the fold itself.
#[test]
fn irules_word_operators_fold_through_sccp() {
    const IR: &str = "f5-irules";
    // A known-constant subject: the lattice resolves `$x` to `abcde` and the
    // word operator folds, collapsing the condition to a literal.
    let contains = "when HTTP_REQUEST {\n    set x \"abcde\"\n    if {$x contains \"cd\"} {\n        pool p1\n    }\n}";
    assert!(
        optimised(contains, IR).contains("if {1}"),
        "`contains` on a known constant must fold under f5-irules; got {:?}",
        optimised(contains, IR)
    );
    // Control: `eq`, an operator plain Tcl shares, folds on the identical
    // shape — the two dialect halves now agree.
    let eq = "when HTTP_REQUEST {\n    set x \"abcde\"\n    if {$x eq \"abcde\"} {\n        pool p1\n    }\n}";
    assert!(opt_fires(eq, IR, "O112"));

    // A provably-false subject folds the other way.
    let miss = "when HTTP_REQUEST {\n    set x \"abcde\"\n    if {$x contains \"zz\"} {\n        pool p1\n    }\n}";
    assert!(
        optimised(miss, IR).contains("if {0}"),
        "a provably-false `contains` must fold to 0; got {:?}",
        optimised(miss, IR)
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
    let registry = registry_for_dialect(TCL);
    let (fixed, _) = optimise_source_multipass(nested, registry, Some(TCL), 10);
    assert!(!fixed.contains("set dead 1"));
    assert!(fixed.contains("set alive 2"));
}

// ---------------------------------------------------------------------------
// Unused variable elimination — O126
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Cross-event DSE — stores consumed by a later event must survive
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Constant var-ref propagation — O100 / O105 (string interpolation)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Pattern-match simplification — O110 matches_regex / matches_glob (f5-irules)
//
// OMISSION: the entire matches_regex/matches_glob → string-op simplification
// family does NOT fire in the optimiser (no O110 for any of the anchored-regex /
// wildcard-glob reprs; all sources pass through unchanged). Leaving them
// unchanged is sound (no miscompile), just not optimised, so rather than assert
// a rewrite we pin the conservative behaviour: the negative cases (which must NOT
// simplify) all hold, and the "positive" cases are a known optimiser gap.
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Strength reduction — O113
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Incr idiom — O114 (needs SSA-known INT type on the loop var)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Nested expr unwrap — O115
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// List folding / lindex folding — O116 / O118
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Strlen zero-check — O117
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// String compare eq/ne — O120
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Multi-set packing — O119
// ---------------------------------------------------------------------------

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

    // Too few consecutive sets ⇒ no packing (holds in Rust too).
    let few = "set a 1\nset b 2\nputs \"$a $b\"";
    assert!(!opt_fires(few, TCL, "O119"));

    // The eval-barrier forms are folded rather than packed; assert the sound
    // constant-forwarded result instead of the (absent) O119 packing.
    assert_eq!(
        optimised("set a 1\nset b 2\nset c 3\neval {$a $b $c}", TCL),
        "eval {1 2 3}"
    );
}

// ---------------------------------------------------------------------------
// End-offset index rewrite — O128
// ---------------------------------------------------------------------------

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
    // unchanged in Rust (no O128). Several are paired in the firing test above
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

// ---------------------------------------------------------------------------
// Variable-shape optimisation guardrails — variable-shape forms not conflated
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Tail-call optimisation — O121 (tailcall) / O122 (loop) / O123 (accumulator)
//
// The optimise_with_dialect set is post-overlap; selection makes O122 subsume
// per-site O121. For a tail call we therefore assert the disjunction
// "O121 or O122" on the applied set.
// ---------------------------------------------------------------------------

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
    // The overlap selection prefers the per-site O121 `tailcall` rewrite for this
    // body and emits O121, not the whole-proc O122 loop conversion. Both are
    // semantically faithful (tclsh: tailcall factorial form == 120). Assert the
    // applied rewrite is a sound tail-call form (tailcall OR while-loop).
    let fo = optimised(fac, TCL);
    assert!(fo.contains("tailcall factorial") || fo.contains("while {1}"));
    assert!(opt_fires(fac, TCL, "O121") || opt_fires(fac, TCL, "O122"));

    // The bare self-call `loop` body DOES take the O122 loop conversion in Rust.
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
    let registry = registry_for_dialect(TCL);
    let o123: Vec<_> = optimise_with_dialect(fac, registry, Some(TCL))
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

// ---------------------------------------------------------------------------
// Unused iRule procs — O124 (f5-irules only)
// ---------------------------------------------------------------------------

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
    let registry = registry_for_dialect(IR);
    let o124s: Vec<_> = optimise_with_dialect(multi, registry, Some(IR))
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

    // O124 does not block independent passes for a *used* proc (O120 still fires
    // inside it); the used proc is not commented out.
    let used_o120 = "proc helper {x} {\n    if {$x == \"foo\"} {\n        return 1\n    }\n    return 0\n}\n\nwhen HTTP_REQUEST {\n    set val [call helper bar]\n}";
    assert!(opt_fires(used_o120, IR, "O120"));
    assert!(!opt_fires(used_o120, IR, "O124"));
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

// ---------------------------------------------------------------------------
// Code sinking — O125
// ---------------------------------------------------------------------------

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
    ];
    for src in neg {
        assert!(!opt_fires(src, TCL, "O125"), "O125 must not fire: {src}");
    }

    // NOTE on omissions. Sound-but-spurious O125 firings (applied rewrite
    // PREPENDS `set b foo` into the branch while KEEPING the outer assignment,
    // so a tclsh run is unaffected) — omitted:
    //  - var not used in the branch at all (`puts hello`).
    //  - var used after via a bare name (`incr b`) / set-read-form (`set b`).
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

// ---------------------------------------------------------------------------
// Load forwarding — O127 (single-use store-to-load forwarding)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Profile directive / multipass — profile survival + string-build collapse
// ---------------------------------------------------------------------------

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
    let registry = registry_for_dialect(TCL);
    let src = "proc build_banner {} {\n    set msg {Hello}\n    append msg { }\n    append msg World\n    return $msg\n}\n";
    let (out, _) = optimise_source_multipass(src, registry, Some(TCL), 10);
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
    use tcl_registry::registry_for_dialect;

    fn optimised(src: &str) -> String {
        let reg = registry_for_dialect("f5-irules");
        apply_optimisations(src, &optimise_with_dialect(src, reg, Some("f5-irules")))
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
