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

//! Reads and writes nested in expression words, conditions and structural bodies, and what they keep alive or kill.

use super::*;

/// #2050 — the store feeding a nested read-modify-write is observed, not
/// overwritten.
///
/// `[incr n]` reads `n` and writes it back. The write was already recorded (as
/// the embedded-substitution effect that invalidates the old version), but the
/// read was not, so `set n 1`'s version had no consumer and looked
/// overwritten-before-read. O109 deleted it and the program changed:
/// tclsh 9.0.4 / 8.6.18 print `2` then `2` for the original; the rewritten
/// program printed `1` then `1` (8.4 raises `can't read "n"`, which does not
/// even create the variable). The store stays, and both reads hold what the
/// increment left.
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
        "set n 1\nset result [incr n]\nputs 2\nputs 2\n",
        "the store and the increment stay; both reads are 2"
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
/// against tclsh 9.0.4: `3`/`2` for the first, `5`/`3` for the second. The
/// store the first `[incr n]` reads stays, and the later reads are of the
/// nested store: each is forwarded the value the expression left, which the
/// statement's definitions hold.
#[test]
fn a_write_nested_in_a_braced_expr_word_keeps_its_feeding_store() {
    for (src, forwarded) in [
        (
            "set n 1\nset r [expr {$n + [incr n]}]\nputs $r\nputs $n\n",
            "set n 1\nset r [expr {$n + [incr n]}]\nputs 3\nputs 2\n",
        ),
        (
            "set n 1\nset r [expr {[incr n] + [incr n]}]\nputs $r\nputs $n\n",
            "set n 1\nset r [expr {[incr n] + [incr n]}]\nputs 5\nputs 3\n",
        ),
    ] {
        assert!(
            !opt_fires(src, TCL, "O109"),
            "`set n 1` feeds the first `[incr n]`: {:?}",
            opt_codes(src, TCL)
        );
        assert_eq!(
            optimised(src, TCL),
            forwarded,
            "{src}: the later reads are the nested store's"
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
    let completed = tcl_engine_tclvm::execute_optimised_closed_source(
        src.to_owned(),
        tcl_dialect::TclVersion::V8_6,
    )
    .expect("actual private Native consumer");
    assert_eq!(completed.transcript.stdout, b"2\n1\n");
    assert_eq!(completed.transcript.stderr, Vec::<u8>::new());
    let out = completed.rewritten_source;
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
///
/// The write chain folds through the lattice's proven `$x` too (O104 / O130
/// over a lattice operand), so the
/// forwarded read and the store it extends become the one store
/// `set x {1 1}`; tclsh 8.4.20 – 9.1b0 print `1 1` for both programs.
#[test]
fn a_direct_read_before_write_still_forwards_its_literal() {
    let appended = optimised("set x 1\nlappend x $x\nputs $x\n", TCL);
    assert_eq!(
        appended, "set x {1 1}\nputs $x\n",
        "a direct RMW read still forwards, and the chain folds through it"
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
///
/// The existence rung decides `[info exists
/// x]` inside the fixed point, so the first program's condition folds to `1`
/// (O101) — a sound rewrite that still prints `yes` — and the store it read
/// stays.
#[test]
fn a_condition_substitution_reads_the_frames_variables() {
    for src in [
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
    let exists_src = "proc p {} {\n  set x 1\n  if {[info exists x]} { puts yes }\n}\np\n";
    assert_eq!(
        optimised(exists_src, TCL),
        "proc p {} {\n  set x 1\n  if {1} { puts yes }\n}\np\n",
        "only the decided query folds, and the store it read stays: {:?}",
        opt_codes(exists_src, TCL)
    );

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
