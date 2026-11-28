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

//! Constant propagation and folding: into commands, strings and expressions, across procedures and loops, and the call-site evidence it rests on.

use super::*;

#[test]
fn propagation_and_constant_folding_core() {
    crate::install_surfaces();
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
    assert_eq!(opt_codes(dyn_src, TCL), [] as [std::string::String; 0]);

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
    crate::install_surfaces();
    // A proc-local constant folds through `return [expr {...}]`: tclsh 8.4.20,
    // 8.6.18 and 9.0.4 all print 3 for `proc add_two {} {set a 1; return
    // [expr {$a + 2}]}; puts [add_two]`.
    let proc = "proc add_two {} {\n    set a 1\n    return [expr {$a + 2}]\n}\n";
    assert_eq!(
        optimised(proc, TCL),
        "proc add_two {} {\n    set a 1\n    return 3\n}\n"
    );

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

/// A name linked with `global`, `upvar` or `variable` exists exactly when
/// its target does, whichever spelling wrote or removed the target.
///
/// tclsh 8.4.20, 8.6.18, 9.0.4 and 9.1.0 print `no yes yes yes yes no` for:
///
/// ```tcl
/// proc init {} { set ::x 1 }
/// proc q1 {} { global x; init; if {[info exists x]} { return yes } else { return no } }
/// proc q2 {} { global x; if {[info exists x]} { return yes } else { return no } }
/// proc q3 {} { upvar #0 x y; if {[info exists y]} { return yes } else { return no } }
/// namespace eval n {variable v 1}
/// proc n::q {} { variable v; if {[info exists v]} { return yes } else { return no } }
/// proc q4 {} { global x; unset ::x; if {[info exists x]} { return yes } else { return no } }
/// puts [list [q2] [q1] [q2] [q3] [n::q] [q4]]
/// ```
#[test]
fn a_linked_name_reads_the_existence_of_its_target() {
    crate::install_surfaces();
    let guard = "if {[info exists LINK]} { puts yes } else { puts no }";
    for (source, link) in [
        (
            "proc init {} { set ::x 1 }\nproc q {} {\n    global x\n    init\n    GUARD\n}\nq\n",
            "x",
        ),
        ("proc q {} {\n    global x\n    GUARD\n}\nset x 1\nq\n", "x"),
        (
            "proc q {} {\n    upvar #0 x y\n    GUARD\n}\nset x 1\nq\n",
            "y",
        ),
        (
            "namespace eval n {variable v 1}\nproc n::q {} {\n    variable v\n    GUARD\n}\nn::q\n",
            "v",
        ),
        (
            "proc q {n} {\n    upvar 1 $n v\n    GUARD\n}\nset x 1\nq x\n",
            "v",
        ),
    ] {
        let source = source.replace("GUARD", &guard.replace("LINK", link));
        for release in ["tcl8.4", "tcl8.6", "tcl9.0"] {
            assert!(
                !optimised(&source, release).contains("if {0}"),
                "{release}: {source}"
            );
        }
    }
    let removed = "proc q {} {\n    global x\n    unset ::x\n    GUARD\n}\nset x 1\nq\n"
        .replace("GUARD", &guard.replace("LINK", "x"));
    for release in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        assert!(
            !optimised(&removed, release).contains("if {1}"),
            "{release}"
        );
    }
}

#[test]
fn interprocedural_constant_folding() {
    crate::install_surfaces();
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
    crate::install_surfaces();
    // Outer constants are NOT propagated into a loop body that redefines them.
    let loop_src =
        "set total 0\nfor {set i 0} {$i < 5} {incr i} {\n    set total [expr {$total + $i}]\n}\n";
    assert_eq!(optimised(loop_src, TCL), loop_src);
    assert_eq!(opt_codes(loop_src, TCL), [] as [std::string::String; 0]);

    // After a statically-bounded loop, $total > 5 is known true ⇒ `if {1}` (O101).
    let post = "set total 0\nfor {set i 0} {$i < 5} {incr i} {\n    set total [expr {$total + $i}]\n}\nif {$total > 5} {\n    puts ok\n}\n";
    assert!(optimised(post, TCL).contains("if {1}"));
    assert!(opt_fires(post, TCL, "O101"));
}

#[test]
fn string_write_chains_o104() {
    crate::install_surfaces();
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
    crate::install_surfaces();
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
    crate::install_surfaces();
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
    crate::install_surfaces();
    let src = "proc f {n} {\n    incr n\n    puts $n\n}";
    assert_eq!(optimised(src, TCL), src, "a parameter is not a constant");
}

#[test]
fn constant_propagation_into_commands_o100() {
    crate::install_surfaces();
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
    assert_eq!(opt_codes(arr, TCL), [] as [std::string::String; 0]);
}

#[test]
fn constant_propagation_into_strings_o105() {
    crate::install_surfaces();
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

#[test]
fn multipass_string_build_collapses_to_literal() {
    crate::install_surfaces();
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
    crate::install_surfaces();
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
    crate::install_surfaces();
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
    crate::install_surfaces();
    let src =
        "proc id {v} { return $v }\nproc a {} { return [list {[id 9]}] }\nputs [id 7]\nputs [a]\n";
    // tclsh 8.4-9.1: `7`, `{[id 9]}`; a later `id 9` returns 9, so the body
    // keeps `return $v` whatever this file's only call passes.
    let out = optimised(src, TCL);
    assert!(
        out.contains("proc id {v} { return $v }") && !out.contains("return 7"),
        "a body is never rewritten from its callers: {:?}",
        opt_rewrites(src, TCL)
    );
}

/// An element index that substitutes selects the element by its value, so a
/// constant stored under the index's literal spelling is not that element.
#[test]
fn a_substituting_element_index_is_not_its_literal_spelling() {
    crate::install_surfaces();
    // tclsh 8.6.18 / 9.0.4: `v=6`.
    let src = "set {arr($idx)} 5\nset idx k\nset arr(k) 6\nputs \"v=$arr($idx)\"\n";
    let out = optimised(src, TCL);
    assert!(out.contains("\"v=$arr($idx)\""), "{out}");
}

/// The empty variable name `{}` is a real variable: inlining it must keep the
/// text after `${}`, and the dead-store coupling must not panic on the name.
#[test]
fn a_constant_in_the_empty_name_variable_inlines_cleanly() {
    crate::install_surfaces();
    // tclsh 8.4.20 / 8.6.18 / 9.0.4: `a=5b`, `a=5`, `55`.
    let src = "set {} 5\nputs \"a=${}b\"\nputs \"a=${}\"\nputs \"${}${}\"\n";
    let out = optimised(src, TCL);
    for want in ["\"a=5b\"", "\"a=5\"", "\"55\""] {
        assert!(out.contains(want), "{want}: {out}");
    }
}
