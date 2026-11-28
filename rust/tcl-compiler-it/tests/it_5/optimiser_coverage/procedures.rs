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

//! Procedure-level passes: O103 interprocedural folding, O121 to O123 tail calls, O124 unused procedures, and procedures that shadow a builtin.

use super::*;

// O103 — interprocedural folding

#[test]
fn o103_interprocedural_folding() {
    crate::install_surfaces();
    // tclsh: pi == 3.
    assert!(optimised("proc pi {} { return 3 }\nset v [pi]\n", TCL).contains("set v 3"));
    // tclsh: double 21 == 42.
    assert!(
        optimised(
            "proc double {x} { return [expr {$x * 2}] }\nset v [double 21]\n",
            TCL
        )
        .contains("set v 42")
    );
    // tclsh: id 99 == 99.
    assert!(
        optimised("proc id {x} { return $x }\nset a 99\nset v [id $a]\n", TCL).contains("set v 99")
    );
    // tclsh: bump 5 (incr x; return x) == 6.
    assert!(
        optimised("proc bump {x} { incr x; return $x }\nset v [bump 5]\n", TCL).contains("set v 6")
    );
    // tclsh: util::add 10 20 == 30.
    assert!(optimised(
        "namespace eval util {\n    proc add {a b} { return [expr {$a + $b}] }\n}\nset v [util::add 10 20]\n",
        TCL
    )
    .contains("set v 30"));

    // Impure proc (puts) is not folded.
    assert!(opt_absent(
        "proc effect {} { puts hi; return 1 }\nset v [effect]\n",
        TCL,
        "O103"
    ));
    // A recursive proc folds where the argument-sensitive re-run reaches the
    // end of its recursion — tclsh: fact 5 == 120 — and not where its seeds
    // never end (tclsh: too many nested evaluations) or it recurses past the
    // re-run's depth (tclsh: down 50 == 0).
    assert!(optimised(
        "proc fact {n} {\n    if {$n <= 1} { return 1 }\n    return [expr {$n * [fact [expr {$n - 1}]]}]\n}\nset v [fact 5]\n",
        TCL
    )
    .contains("set v 120"));
    assert!(opt_absent(
        "proc loop {n} {return [loop $n]}\nset v [loop 1]\n",
        TCL,
        "O103"
    ));
    assert!(opt_absent(
        "proc down {n} {if {$n <= 0} {return 0}; return [down [expr {$n - 1}]]}\nset v [down 50]\n",
        TCL,
        "O103"
    ));
}

#[test]
fn o103_alias_resolved_proc_replacement_is_not_folded_as_the_original() {
    crate::install_surfaces();
    let src = "proc p {} {return P}\nproc q {} {return Q}\n\
               interp alias {} r {} rename\n\
               r p oldp\nr q p\nset value [p]\n";
    // tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 all set `value` to Q:
    // `r` is an alias of `rename`, so `[p]` runs `q`'s body. A fresh-document
    // build that follows the alias may fold the call, but only to Q; the
    // original `p`'s P is never a valid answer (correction 2026-10-21: the
    // former `O103 absent` check pinned the dialect-blind build, which could
    // not see through the alias).
    let registry = static_context_for(TCL).commands();
    let profile = tcl_registry::model::ingress::resolve_environment(TCL).analyser_profile();
    for o in optimise_with_dialect(src, registry, Some(profile))
        .iter()
        .filter(|o| o.code.as_str() == "O103")
    {
        assert_eq!(o.replacement, "Q", "{o:?}");
    }
    let out = optimised(src, TCL);
    assert!(
        out.contains("set value [p]") || out.contains("set value Q"),
        "{out}"
    );
    assert!(!out.contains("set value P"), "{out}");
    // An alias of a command that moves nothing leaves `p` the original:
    // tclsh `interp alias {} r {} list` then `[p]` is P.
    let listed = "proc p {} {return P}\nproc q {} {return Q}\n\
                  interp alias {} r {} list\n\
                  r p oldp\nr q p\nset value [p]\n";
    let out = optimised(listed, TCL);
    assert!(!out.contains("set value Q"), "{out}");
}

// O124 — comment out unused procs (f5-irules only)

#[test]
fn o124_unused_irule_procs() {
    crate::install_surfaces();
    // Unused proc commented out.
    let unused = "proc helper {} { return 1 }\nwhen HTTP_REQUEST { pool p1 }";
    assert!(opt_fires(unused, IR, "O124"));
    let uo = optimised(unused, IR);
    assert!(uo.contains("# proc helper"));
    assert!(uo.contains("# [O124]"));

    // Used / transitively-used / direct-call / qualified procs are NOT commented.
    let kept = [
        "proc helper {} { return 1 }\nwhen HTTP_REQUEST { set v [call helper] }",
        "proc helper {} { return 1 }\nwhen HTTP_REQUEST { set v [helper] }",
        "proc inner {} { return 1 }\nproc outer {} { return [call inner] }\nwhen HTTP_REQUEST { set v [call outer] }",
        // library iRule (only procs + RULE_INIT) — skip.
        "proc helper {} { return 1 }\nwhen RULE_INIT { set ::x 1 }",
    ];
    for src in kept {
        assert!(opt_absent(src, IR, "O124"), "O124 must not fire: {src}");
    }

    // Plain Tcl is never touched by O124.
    assert!(opt_absent(
        "proc helper {} { return 1 }\nputs hello",
        TCL,
        "O124"
    ));

    // O124 supersedes tail-call rewrites for an unused tail-recursive proc.
    let supersede = "proc fact {n acc} {\n    if {$n <= 1} { return $acc }\n    return [fact [expr {$n - 1}] [expr {$n * $acc}]]\n}\nwhen HTTP_REQUEST { pool p1 }";
    let codes = opt_codes(supersede, IR);
    assert!(codes.iter().any(|c| c == "O124"));
    assert!(!codes.iter().any(|c| c == "O121"));
    assert!(!codes.iter().any(|c| c == "O122"));

    // A used proc allows other rewrites inside it; not commented out.
    //
    // O112 rather than O120 since #2134: `[call helper bar]` is a command
    // substitution nested in a word, now enumerated as a call site, so `x` is
    // known to be `bar` in the body and the `foo` branch is proven dead.
    let used = "proc helper {x} {\n    if {$x == \"foo\"} { return 1 }\n    return 0\n}\nwhen HTTP_REQUEST { set v [call helper bar] }";
    assert!(opt_fires(used, IR, "O112"));
    assert!(opt_absent(used, IR, "O124"));

    // O124 coexists with an in-event rewrite.
    let coexist = "proc helper {} { return 1 }\nwhen HTTP_REQUEST {\n    set x [expr {1 + 2}]\n    pool p1\n}";
    let cc = opt_codes(coexist, IR);
    assert!(cc.iter().any(|c| c == "O124"));
    assert!(cc.iter().any(|c| c == "O101" || c == "O102" || c == "O126"));

    // eval only in an UNREACHABLE proc does not suppress O124.
    let unreach = "proc dynamic_helper {} {\n    set cmd foo\n    eval $cmd\n}\nwhen HTTP_REQUEST { pool p1 }";
    assert!(opt_fires(unreach, IR, "O124"));
}

// O121 / O122 / O123 — tail-call / loop / accumulator

#[test]
fn tail_call_o121_o122_o123() {
    crate::install_surfaces();
    // Tail-position self-call ⇒ O121 (tailcall) or O122 (loop). tclsh: the
    // tailcall/loop forms compute the same result (factorial 5 1 == 120).
    let fac = "proc factorial {n acc} {\n    if {$n <= 1} { return $acc }\n    return [factorial [expr {$n - 1}] [expr {$n * $acc}]]\n}\n";
    assert!(opt_any_of(fac, TCL, &["O121", "O122"]));

    // Arity mismatch ⇒ O121 may fire but O122 (full loop conversion) must not.
    let arity = "proc f {a b} {\n    return [f $a]\n}\n";
    assert!(opt_fires(arity, TCL, "O121"));
    assert!(opt_absent(arity, TCL, "O122"));

    // Non-tail accumulator-eligible recursion ⇒ O123 hint.
    let nontail = "proc factorial {n} {\n    if {$n <= 1} { return 1 }\n    return [expr {$n * [factorial [expr {$n - 1}]]}]\n}\n";
    assert!(opt_fires(nontail, TCL, "O123"));
    // An already-tail-recursive proc does NOT get the O123 hint.
    assert!(opt_absent(fac, TCL, "O123"));
    // A non-recursive proc gets neither tail-call code nor O123.
    let plain = "proc add {a b} {\n    return [expr {$a + $b}]\n}\n";
    assert!(opt_absent(plain, TCL, "O121"));
    assert!(opt_absent(plain, TCL, "O123"));
    // Doubly-recursive (fib) ⇒ no O122 (not a tail call).
    let fib = "proc fib {n} {\n    if {$n <= 1} { return $n }\n    return [expr {[fib [expr {$n-1}]] + [fib [expr {$n-2}]]}]\n}\n";
    assert!(opt_absent(fib, TCL, "O122"));
}

// A `proc` shadowing a builtin (#2164): one call, one answer

/// Helper for the shadowing-`proc` regression below: the multipass rewrite.
fn multipassed(src: &str) -> String {
    let registry = static_context_for(TCL).commands();
    optimise_source_multipass(
        src,
        registry,
        Some(tcl_registry::model::ingress::resolve_environment(TCL).analyser_profile()),
        10,
    )
    .0
}

/// A user `proc llength` shadows the builtin, so `[llength {a b c}]` is a call
/// to *that* proc. The optimiser used to answer the same call twice in one
/// statement: O103 folded the call through the shadowing proc (99) while the
/// value lattice — built without the whole-module command-trust fact — handed
/// O100 the *builtin's* answer (3), rewriting `set v [llength {a b c}]; return
/// $v` to `set v 99; return 3`.
///
/// tclsh 8.4.20 / 8.5.19 / 8.6.18 / 9.0.4 / 9.1b0 (unanimous): `::n::g`
/// returns **99** with the shadow and **3** without it. Both forms create the
/// namespace first — real Tcl refuses `proc ::n::f …` for a namespace that
/// does not exist ("can't create procedure: unknown namespace").
#[test]
fn a_proc_shadowing_a_builtin_is_never_folded_with_builtin_semantics() {
    crate::install_surfaces();
    // The namespace-local shadow, exactly as reported.
    let shadowed = "namespace eval ::n { proc llength {l} { return 99 } }\n\
                    proc ::n::g {} { set v [llength {a b c}]; return $v }\n";
    let out = multipassed(shadowed);
    assert!(
        out.contains("return 99"),
        "shadowed call must fold to the shadowing proc's 99, got {out:?}",
    );
    assert!(
        !out.contains("return 3") && !out.contains("set v 3"),
        "the builtin's answer must not appear anywhere, got {out:?}",
    );

    // A plain global shadow is the same program shape and the same answer.
    let global = "proc llength {l} { return 99 }\nnamespace eval ::n {}\n\
                  proc ::n::g {} { set v [llength {a b c}]; return $v }\n";
    let out = multipassed(global);
    assert!(
        out.contains("return 99") && !out.contains("return 3"),
        "global shadow must fold to 99, got {out:?}",
    );

    // Positive control: with nothing shadowing `llength` the very same shape
    // folds through the builtin to 3, so the assertions above are about the
    // shadow and not about the optimiser having given up on this shape.
    let control = "namespace eval ::n {}\n\
                   proc ::n::g {} { set v [llength {a b c}]; return $v }\n";
    let out = multipassed(control);
    assert!(
        out.contains("return 3"),
        "unshadowed control must still fold to the builtin's 3, got {out:?}",
    );
}

/// The same shadowed value reaching the other two rewrites that read the
/// value lattice: the O100 string-interpolation inline and the O112 constant
/// `if`. Both used to publish the builtin's 3.
///
/// tclsh 8.4.20 – 9.1b0 (unanimous), with the shadow in place: `set v
/// [llength {a b c}]` leaves `v` as **99**, so `"x$v"` is `x99` and `$v == 3`
/// is false. Without it, `x3` and true.
#[test]
fn a_shadowed_builtin_value_is_not_published_into_strings_or_branches() {
    crate::install_surfaces();
    let interp = "proc llength {l} { return 99 }\n\
                  proc g {} { set v [llength {a b c}]; puts \"x$v\" }\n";
    let out = multipassed(interp);
    assert!(
        out.contains("x99") && !out.contains("x3"),
        "interpolation must carry the shadowing proc's 99, got {out:?}",
    );

    let branch = "proc llength {l} { return 99 }\n\
                  proc g {} { set v [llength {a b c}]; if {$v == 3} { return yes } ; return no }\n";
    let out = multipassed(branch);
    assert!(
        out.contains("return no") && !out.contains("return yes"),
        "`$v == 3` is false for the shadowed 99, got {out:?}",
    );

    // Positive controls: unshadowed, both rewrites still fire with the
    // builtin's 3 — the two assertions above measure the shadow, not a dead
    // pass.
    let out = multipassed("proc g {} { set v [llength {a b c}]; puts \"x$v\" }\n");
    assert!(
        out.contains("x3"),
        "unshadowed control must still inline 3 into the string, got {out:?}",
    );
    let out = multipassed(
        "proc g {} { set v [llength {a b c}]; if {$v == 3} { return yes } ; return no }\n",
    );
    assert!(
        out.contains("return yes"),
        "unshadowed control must still fold the branch true, got {out:?}",
    );
}

/// The O104 / O130 write-chain fold is the same question again: each arm of
/// its write classifier *is* that command's semantics, so it may only run
/// while the name still denotes the builtin. It had no trust gate at all.
///
/// tclsh 8.6.18 / 9.0.4: with `proc append {varName args} {return ZZZ}` in
/// scope, `set s ""; append s foo; append s bar; return $s` returns the empty
/// string — the shadowing proc never touches `s` — where the fold answered
/// `foobar`. Likewise a shadowed `lappend` leaves the list empty.
#[test]
fn a_shadowed_append_or_lappend_stops_the_write_chain_fold() {
    crate::install_surfaces();
    let appended = "proc append {varName args} { return ZZZ }\n\
                    proc g {} { set s \"\"; append s foo; append s bar; return $s }\n";
    let out = multipassed(appended);
    assert!(
        !out.contains("foobar"),
        "a shadowed `append` must not be folded as the builtin, got {out:?}",
    );

    let listed = "proc lappend {varName args} { return ZZZ }\n\
                  proc g {} { set l {}; lappend l a; lappend l b c; return $l }\n";
    let out = multipassed(listed);
    assert!(
        !out.contains("a b c"),
        "a shadowed `lappend` must not be folded as the builtin, got {out:?}",
    );

    // Positive controls: unshadowed, both chains still collapse — so the two
    // assertions above measure the shadow, not a pass that stopped firing.
    let out = multipassed("proc g {} { set s \"\"; append s foo; append s bar; return $s }\n");
    assert!(
        out.contains("foobar"),
        "unshadowed control must still fold the append chain, got {out:?}",
    );
    let out = multipassed("proc g {} { set l {}; lappend l a; lappend l b c; return $l }\n");
    assert!(
        out.contains("a b c"),
        "unshadowed control must still fold the lappend chain, got {out:?}",
    );
}
