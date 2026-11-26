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

//! `exprStk` syntax errors and the `returnImm` and `returnStk` forms.

use super::*;

// `exprStk`.

/// C `INST_EXPR_STK` (`tclExecute.c`) calls `TclCompileExpr` on the popped
/// string, so an unparsable expression is a `TCL_ERROR` from `ParseExpr` — never
/// a value. An opcode that hands the source text back instead would make
/// every unparsable expression a silent wrong answer.
///
/// Messages transcribed from C's `tests/parseExpr.test` (`parseExpr-21.*`).
#[test]
fn expr_stk_raises_c_syntax_errors() {
    for (expr, message, code) in [
        (
            "1+",
            "missing operand at _@_\nin expression \"1+_@_\"",
            "TCL PARSE EXPR MISSING",
        ),
        (
            "1 2",
            "missing operator at _@_\nin expression \"1 _@_2\"",
            "TCL PARSE EXPR MISSING",
        ),
        (
            "",
            "empty expression\nin expression \"\"",
            "TCL PARSE EXPR EMPTY",
        ),
        (
            "foo bar baz",
            "invalid bareword \"foo\"\nin expression \"foo bar baz\";\n\
             should be \"$foo\" or \"{foo}\" or \"foo(...)\" or ...",
            "TCL PARSE EXPR BAREWORD",
        ),
    ] {
        let mut a = Asm::new();
        a.push(expr).op(Op::EXPR_STK, &[]);
        let (_vm, c) = run_fresh(a);
        assert_eq!(err_str(&c), message, "exprStk over {expr:?}");
        // The opcode must publish C's structured `-errorcode`, not just the
        // message (`tclCompExpr.c:1467`).
        assert_eq!(err_code(&c), code, "exprStk -errorcode over {expr:?}");
    }
}

/// TP control: a parsable expression still evaluates through the same opcode.
#[test]
fn expr_stk_still_evaluates_a_valid_expression() {
    for (expr, want) in [("2+3", "5"), ("1 ? 2 : 3", "2"), ("\"a\" eq \"a\"", "1")] {
        let mut a = Asm::new();
        a.push(expr).op(Op::EXPR_STK, &[]);
        let (_vm, c) = run_fresh(a);
        assert_eq!(ok_str(&c), want, "exprStk over {expr:?}");
    }
}

// `returnImm` / syntax.

/// `returnImm`'s two immediates are the *merged* `(code, level)` pair
/// (`TclMergeReturnOptions` strips `-code`/`-level` out of the literal options
/// dict, which is why they ride the operands), and `TclProcessReturn`
/// (`tclResult.c:777-785`) turns them into the completion:
///
/// ```text
/// if (level != 0) { returnLevel = level; returnCode = code; return TCL_RETURN; }
/// if (code == TCL_ERROR) { flags |= ERR_LEGACY_COPY; }
/// return code;
/// ```
///
/// So `level != 0` is always `TCL_RETURN` — the proc-boundary countdown applies
/// `-code` later — regardless of what `code` says.
#[test]
fn return_imm_positive_level_raises_tcl_return() {
    for (code, level) in [(0, 1), (1, 1), (3, 1), (4, 1), (1, 2)] {
        let mut a = Asm::new();
        a.push("v").push("").op(Op::RETURN_IMM, &[code, level]);
        let (_vm, c) = run_fresh(a);
        assert_eq!(
            c.code,
            Code::Return,
            "returnImm {code} {level} must be TCL_RETURN"
        );
        assert_eq!(&*c.result.to_str(), "v");
        // The carried `-code`/`-level` are what the boundary counts down; they
        // travel as the merge's out parameters, not as dictionary keys.
        assert_eq!(
            c.option_origin,
            tcl_core_types::CompletionOptionOrigin::MergedReturnOptions {
                code,
                level: i64::from(level),
            },
            "carried -code and -level"
        );
    }
}

/// `level == 0` makes the completion *be* `code`, so `(0, 0)` is `TCL_OK`: C's
/// `INST_RETURN_IMM` takes the `result == TCL_OK` branch and
/// `NEXT_INST_F(9, 1, 0)` continues to the next instruction, popping only the
/// options dict and leaving the result on the stack. It is emphatically *not* a
/// return — the instructions after it still run.
#[test]
fn return_imm_level_zero_code_ok_continues_with_result_on_stack() {
    let mut a = Asm::new();
    a.push("kept").push("").op(Op::RETURN_IMM, &[0, 0]);
    // Still executing: append a second value and concatenate the two, proving
    // `kept` stayed on the stack and the stream did not end at the returnImm.
    a.push("-after").op(Op::STR_CONCAT1, &[2]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "kept-after");
}

/// `level == 0` with `code == TCL_ERROR` is an immediate error (this is how
/// `error msg` compiles — `TclCompileErrorCmd` emits `returnImm 1 0`).
#[test]
fn return_imm_level_zero_code_error_raises() {
    let mut a = Asm::new();
    a.push("boom").push("").op(Op::RETURN_IMM, &[1, 0]);
    a.push("unreachable").op(Op::STR_CONCAT1, &[2]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), "boom");
}

/// `level == 0` passes `break`/`continue`/a non-standard code straight through
/// as the completion, and `syntax` shares the arm (C falls through the two
/// `case` labels), so it decodes its `(1, 0)` the same way.
#[test]
fn return_imm_level_zero_passes_other_codes_and_syntax_shares_the_arm() {
    for (code, want) in [(3, Code::Break), (4, Code::Continue), (7, Code::Other(7))] {
        let mut a = Asm::new();
        a.push("v").push("").op(Op::RETURN_IMM, &[code, 0]);
        let (_vm, c) = run_fresh(a);
        assert_eq!(c.code, want, "returnImm {code} 0");
    }
    let mut a = Asm::new();
    a.push("bad expr").push("").op(Op::SYNTAX, &[1, 0]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), "bad expr");
}

/// The literal options dict still reaches the completion, with the operands
/// overriding any `-code`/`-level` in it — C's out-params win the same way.
#[test]
fn return_imm_merges_literal_options_dict() {
    let mut a = Asm::new();
    a.push("v")
        .push("-errorcode {MY CODE} -level 9")
        .op(Op::RETURN_IMM, &[1, 0]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), "v");
    assert_eq!(err_code(&c), "MY CODE", "dict keys survive");
    assert_eq!(
        c.option_origin,
        tcl_core_types::CompletionOptionOrigin::MergedReturnOptions { code: 1, level: 0 },
        "the operands override the dict"
    );
}

/// The end-to-end countdown: `return -level 2 -code error` is seen as `TCL_RETURN`
/// one proc boundary up and only becomes the error at the second, so the inner
/// `catch` reports 2 while the outer call errors.
#[test]
fn return_level_two_counts_down_across_two_proc_boundaries() {
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(compiler::svc()));
    let c = vm
        .eval_source(
            "proc inner {} { return -level 2 -code error deep }\n\
             proc outer {} { set r [catch {inner} m]; list $r $m }\n\
             outer",
        )
        .expect("compiles");
    assert_eq!(
        ok_str(&c),
        "2 deep",
        "the inner boundary still sees a RETURN"
    );
    let c = vm
        .eval_source("proc mid {} { inner }\ncatch {mid} m2; list [catch {mid}] $m2")
        .expect("compiles");
    assert_eq!(
        ok_str(&c),
        "1 deep",
        "the second boundary applies -code error"
    );
}

/// The end-to-end surface of every `returnImm` encoding, since the operand
/// realignment changes control flow for *every* `return`. Each row is the value
/// C Tcl 9.0.4 produces for the same script.
#[test]
fn return_forms_behave_as_c_tcl() {
    for (src, want) in [
        // a proc returning a value, and a bare `return` (empty result, and the
        // statement after it does not run)
        ("proc p {} { return 42 }\np", "42"),
        (
            "proc p {} { set ::x 1; return; set ::x 2 }\nset r [p]\nlist [string length $r] $::x",
            "0 1",
        ),
        // `return -code error` is an error one level up
        (
            "proc p {} { return -code error boom }\nlist [catch {p} m] $m",
            "1 boom",
        ),
        // `-code break`/`-code continue` complete the proc with that code, which
        // reaches the *caller's* loop.
        (
            "proc p {} { return -code break }\nset r {}\nforeach i {a b c} { p; lappend r $i }\nset r",
            "",
        ),
        (
            "proc p {} { return -code continue }\nset r {}\nforeach i {a b c} { p; lappend r skipped }\nllength $r",
            "0",
        ),
        // Outside a loop the code is not turned into an error at the proc
        // boundary: C's `TclProcessProcResultCode` runs its switch once, so the
        // countdown's TCL_BREAK is returned as-is and `catch` reports 3 with an
        // empty result (`tclProc.c:1855-1875`).
        (
            "proc p {} { return -code break }\nlist [catch {p} m] [string length $m]",
            "3 0",
        ),
        // `return -code return` merges to `-code ok -level 2`, so the carried
        // TCL_RETURN is what an enclosing `catch` reports (code 2). Only the
        // in-`catch` form compiles to `returnImm`; a plain proc-body
        // `return -code …` is a generic invoke of the runtime `return`.
        (
            "proc p {} { set rc [catch {return -code return v} m]; list $rc $m }\np",
            "2 v",
        ),
        // `catch` absorbs a carried return as code 2
        ("proc p {} { list [catch {return x} m] $m }\np", "2 x"),
        // `-level 0` makes the code immediate: `ok` falls through to the next
        // statement, `error` raises.
        (
            "proc p {} { return -level 0 -code ok first; return second }\np",
            "second",
        ),
        (
            "proc p {} { return -level 0 -code error e }\nlist [catch {p} m] $m",
            "1 e",
        ),
        // The `(0, 0)` encoding inside a catch range: not a return at all, so the
        // body completes ok with the value.
        (
            "proc p {} { list [catch {return -level 0 x} m] $m }\np",
            "0 x",
        ),
        // `error msg` is unchanged by the realignment (`returnImm 1 0`)
        (
            "proc p {} { error kaboom }\nlist [catch {p} m] $m",
            "1 kaboom",
        ),
    ] {
        let mut vm = tcl_vm::test_support::native_vm();
        vm.set_compiler(Box::new(compiler::svc()));
        let c = vm.eval_source(src).expect("compiles");
        assert_eq!(ok_str(&c), want, "script: {src}");
    }
}

/// A top-level `return` ends the script — it must not fall through to the
/// statements after it, which is exactly what a `(0, 0)` encoding would do.
#[test]
fn top_level_return_ends_the_script() {
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(compiler::svc()));
    let c = vm
        .eval_source("set x before\nreturn done\nset x after")
        .expect("compiles");
    assert_eq!(&*c.result.to_str(), "done");
    assert_eq!(
        vm.get_var("x").map(|v| v.to_str().to_string()),
        Some("before".into()),
        "the statement after a top-level return must not run"
    );
}
