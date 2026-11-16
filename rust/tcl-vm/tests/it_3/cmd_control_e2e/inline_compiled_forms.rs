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

//! `catch`, `info exists` and `return` where the compiler inlines them and where it must leave them dispatched.

use super::*;

/// A straight-line `catch` body compiles into the enclosing procedure rather
/// than being dispatched whole (#2207), so its variables are real locals.
///
/// Verified against tclsh 9.0.4: `proc f {} { catch {set x 42}; return $x }`
/// is `42` on both, and the disassembly agrees instruction for instruction —
/// `beginCatch4`, `loadScalar1`, `storeScalar1`, where before the whole body
/// was one `invokeStk1` and `x` never reached the local table.
#[test]
fn a_catch_body_write_is_visible_after_the_catch() {
    assert_eq!(run("proc f {} { catch {set x 42}; return $x }; f").1, "42");
    assert_eq!(run("proc g {} { catch {incr n 2}; return $n }; g").1, "2");
    assert_eq!(
        run("proc h {} { set n 5; catch {incr n 2}; return $n }; h").1,
        "7"
    );
}

/// The body still runs inside the exception range: a failure is caught, and
/// nothing the body did not reach is defined.
#[test]
fn an_inlined_catch_still_catches() {
    assert_eq!(run("proc f {} { catch {error boom} }; f").1, "1");
    assert_eq!(
        run("proc f {} { catch {error boom} m; return $m }; f").1,
        "boom"
    );
    assert_eq!(
        run("proc f {} { catch {expr {1/0}} m; return $m }; f").1,
        "divide by zero"
    );
    // The write never happened, so the name is not defined.
    assert_eq!(
        run("proc f {} { catch {error a}; return [info exists q] }; f").1,
        "0"
    );
}

/// `catch`'s own value is its return code, in every position.
#[test]
fn catch_returns_its_code_wherever_it_sits() {
    assert_eq!(run("proc f {} { catch {set q 1} }; f").1, "0");
    assert_eq!(run("proc f {} { catch {set q 1}; set z 2 }; f").1, "2");
    assert_eq!(
        run("proc f {} { set rc [catch {error x}]; return $rc }; f").1,
        "1"
    );
    assert_eq!(
        run("proc f {} { catch {return 42} m; return \"caught:$m\" }; f").1,
        "caught:42"
    );
}

/// A `catch` outside a procedure keeps the dispatched form, as C does: there
/// is no local variable table to hold the result variable's slot, and tclsh
/// compiles a top-level `catch {set a 1} m o` to a plain `invokeStk`.
#[test]
fn a_top_level_catch_is_not_inlined() {
    assert_eq!(run("catch {set a 1} m o; set m").1, "1");
    assert_eq!(run("catch {error boom} m; set m").1, "boom");
}

/// `info exists` in statement position compiles inline in a procedure, so the
/// name it tests becomes a compiled local (#2207).
///
/// tclsh 9.0.4 emits `existScalar %v0` for a bare `info exists pub` in a
/// procedure, against our previous `invokeStk1`.
#[test]
fn statement_position_info_exists_answers_like_tclsh() {
    assert_eq!(run("proc f {} { info exists pub }; f").1, "0");
    assert_eq!(run("proc f {} { set pub 1; info exists pub }; f").1, "1");
    // Not the last statement: the value is discarded, the next one stands.
    assert_eq!(
        run("proc f {} { set pub 1; info exists pub; return done }; f").1,
        "done"
    );
}

/// A name the inline form cannot address keeps the dispatched path: outside a
/// procedure there is no slot, and a qualified name is not a frame local.
#[test]
fn info_exists_without_a_slot_stays_dispatched() {
    assert_eq!(run("info exists nope; set x 1").1, "1");
    assert_eq!(run("set ::gv 1; proc f {} { info exists ::gv }; f").1, "1");
    // A braced name resolves once and is not substituted again.
    assert_eq!(
        run("set {{zz}} V\nputs [info exists {{zz}}]:[set {{zz}}]\n").1,
        ""
    );
}

/// Only the last statement of a `catch` body is its result; the earlier ones
/// are discarded as ordinary script execution discards them.
///
/// Without that, `catch {set x 1; set y 2}` leaves `1` beneath the result and
/// a loop around it grows the operand stack without bound. The bytecode is
/// byte-identical to tclsh 9.0.4, which pops after each non-final command.
#[test]
fn a_multi_command_catch_body_discards_all_but_its_last_result() {
    assert_eq!(
        run("proc f {} { catch {set x 1; set y 2} m; return \"$m $x $y\" }; f").1,
        "2 1 2"
    );
    assert_eq!(
        run("proc f {} { catch {set x 1; set y 2; set z 3} m; return $m }; f").1,
        "3"
    );
    // Repeated execution must not accumulate stack.
    assert_eq!(
        run("proc f {} { for {set i 0} {$i < 2000} {incr i} { catch {set x 1; set y 2} }; return ok }; f").1,
        "ok"
    );
}

/// A destination word that is not a literal scalar local keeps the dispatched
/// path, because lowering has already normalised it: `$dst` arrives as `dst`
/// and `a(key)` as `a`, so the inline store would write the wrong variable.
#[test]
fn catch_destinations_that_need_resolving_are_not_inlined() {
    assert_eq!(
        run("proc f {} { set dst m; catch {set x ok} $dst; return \"[info exists m] $dst\" }; f").1,
        "1 m"
    );
    // `catch script ?result? ?options?` and no more — C raises this before
    // running the body, so the body must not run.
    let (ok, result, _) = run("proc f {} { catch {set ran 1} r o extra }; f");
    assert!(!ok, "over-arity catch must fail, got {result}");
    assert!(
        result.contains("wrong # args"),
        "and fail the way C does: {result}"
    );
}

/// A `return` word with a backslash escape and no `$` or `[` returns the
/// escape's value: tclsh 8.6.18 and 9.0.4 give `A` for `return \101`,
/// `return \x41` and `return "\101"`, and a one-character newline for
/// `return \n`; tclvm pushed the word's source text (#2291).
#[test]
fn a_return_word_decodes_its_backslash_escapes() {
    for (src, want) in [
        ("proc p {} {return \\101}; p", "A"),
        ("proc p {} {return \\x41}; p", "A"),
        ("proc p {} {return \"\\101\"}; p", "A"),
        ("proc p {} {set z 1; return \\101}; p", "A"),
        ("proc p {} {return \\n}; p", "\n"),
        ("proc p {} {return {\\101}}; p", "\\101"),
    ] {
        assert_eq!(run(src).1, want, "{src}");
    }
}

/// A braced `return` value is literal text: `return {[id 9]}` returns the six
/// characters `[id 9]` and runs nothing.
///
/// `Terminator::Return` carries `braced`, but both bytecode emitters read only
/// the flattened `value`, whose braces are already stripped — so a proc-body
/// return treated `{[id 9]}` as the whole-word substitution `[id 9]` and
/// inlined the call, and any other braced value went through interpolation.
/// tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1b0 all return the literal; tclvm
/// ran `id` at every `--tcl-version` (#2228).
#[test]
fn a_braced_return_value_runs_no_substitution() {
    for (why, src, want) in [
        (
            "a whole-word bracket pair",
            "proc id {v} {return RAN}; proc m {} {return {[id 9]}}; m",
            "[id 9]",
        ),
        (
            "brackets inside a longer braced word",
            "proc id {v} {return RAN}; proc m {} {return {a [id 9] b}}; m",
            "a [id 9] b",
        ),
        (
            "a return that is not the proc's only command",
            "proc id {v} {return RAN}; proc m {} {set z 1; return {[id 9]}}; m",
            "[id 9]",
        ),
        (
            "a return inside a branch",
            "proc id {v} {return RAN}; proc m {} {if {1} {return {[id 9]}}}; m",
            "[id 9]",
        ),
        (
            "a variable reference",
            "set x GLOBAL; proc m {} {return {$x}}; m",
            "$x",
        ),
    ] {
        assert_eq!(run(src).1, want, "{why}");
    }

    // Precision: an unbraced return still substitutes.
    assert_eq!(
        run("proc id {v} {return RAN}; proc m {} {return [id 9]}; m").1,
        "RAN"
    );
}
