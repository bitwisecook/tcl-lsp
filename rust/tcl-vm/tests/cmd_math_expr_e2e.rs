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

//! End-to-end coverage of `expr` operators, `tcl::mathfunc::*` math functions,
//! and `::tcl::mathop::*` operator commands — each compiled as real Tcl via
//! `tcl-compiler` and run through `tcl-vm`. Expected outputs are taken from real
//! `tclsh` (8.6 and 9.0); each non-obvious assertion cites a `// tclsh:` comment.
//!
//! The `bug_*` tests document former VM-vs-tclsh divergences on valid input:
//! each asserts the *correct* tclsh behaviour and now passes, guarding the fix
//! against regression.
//!
//! Targets: `tcl-vm/src/cmd_math.rs`, `expr.rs`, `cmd_mathop.rs`.
//!
//! Note on the oracle: `lt`/`le`/`gt`/`ge` (as `expr` operators and as
//! `::tcl::mathop::*` commands), underscore digit separators, and `0NNN` octal
//! were introduced in Tcl 8.7/9.0; tclsh 8.6 rejects them. The VM implements the
//! 9.0 semantics, so those assertions are pinned to tclsh 9.0. Everything else is
//! pinned to behaviour shared by tclsh 8.6 and 9.0.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

/// A `tcl-compiler`-backed compile service so the VM can resolve runtime
/// `eval` / `[command substitution]` (the injection seam — `tcl-vm` itself
/// never depends on the compiler).
struct CompilerSvc {
    registry: CommandRegistry,
}

impl CompileService for CompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error(src) {
            return Err(CompileError(msg));
        }
        let ir = lower_to_ir(src, &self.registry);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.registry))
    }
}

/// A `Write` sink backed by a shared buffer the test can read afterwards.
#[derive(Clone)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Compile and run `src`; return `(ok, result-string, captured-stdout)`.
fn run(src: &str) -> (bool, String, String) {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);

    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(CompilerSvc {
        registry: CommandRegistry::build_default(),
    }));
    let completion = vm.run_module(&asm);

    let out = String::from_utf8(buf.borrow().clone()).expect("utf-8 output");
    (
        completion.code.is_ok(),
        completion.result.to_str().to_string(),
        out,
    )
}

/// Assert `expr {EXPR}` evaluates OK and equals `want`.
fn expr_eq(e: &str, want: &str) {
    let (ok, result, _) = run(&format!("expr {{{e}}}"));
    assert!(ok, "`expr {{{e}}}` should evaluate, got error: {result}");
    assert_eq!(result, want, "expr {{{e}}}");
}

/// Assert `expr {EXPR}` fails with exactly `msg`.
fn expr_err(e: &str, msg: &str) {
    let (ok, result, _) = run(&format!("expr {{{e}}}"));
    assert!(!ok, "`expr {{{e}}}` should error, got ok: {result}");
    assert_eq!(result, msg, "expr {{{e}}}");
}

/// Assert command `cmd` runs OK and equals `want`.
fn cmd_eq(cmd: &str, want: &str) {
    let (ok, result, _) = run(cmd);
    assert!(ok, "`{cmd}` should run, got error: {result}");
    assert_eq!(result, want, "{cmd}");
}

/// Assert command `cmd` fails with exactly `msg`.
fn cmd_err(cmd: &str, msg: &str) {
    let (ok, result, _) = run(cmd);
    assert!(!ok, "`{cmd}` should error, got ok: {result}");
    assert_eq!(result, msg, "{cmd}");
}

// expr: arithmetic operators (+ - * / % ** integer path)

/// The *dynamic* `expr $e` form (the expression text comes from a variable) is
/// compiled to `EXPR_STK` and evaluated at runtime through the [`ExprEval`]
/// adapter (`tcl-vm/src/expr.rs`), exercising its `literal`/`var`/`command`/
/// `call`/`compare`/`in_list`/`string`/`to_bool` methods — the path the braced
/// `expr {…}` form const-folds away. All values pinned to tclsh 8.6/9.0.
#[test]
fn expr_stk_dynamic_evaluation() {
    cmd_eq("set e {2 + 3}; expr $e", "5"); // arith
    cmd_eq("set e {sqrt(16)}; expr $e", "4.0"); // call
    cmd_eq("set x 5; set e {$x * 2}; expr $e", "10"); // var
    cmd_eq("set e {[string length abc] + 1}; expr $e", "4"); // command
    cmd_eq("set e {\"a\" in {a b}}; expr $e", "1"); // in_list
    cmd_eq("set e {\"z\" ni {a b}}; expr $e", "1"); // ni
    cmd_eq("set e {2 < 3}; expr $e", "1"); // compare_numeric
    cmd_eq("set e {\"x\" eq \"y\"}; expr $e", "0"); // compare_string
    cmd_eq("set e {1 ? 2 : 3}; expr $e", "2"); // ternary / to_bool
    cmd_eq("set e {true && false}; expr $e", "0"); // bool_value
    // The runtime literal/var paths DO normalize (unlike the braced bare form).
    cmd_eq("set e {1e3}; expr $e", "1000.0"); // literal normalization
    cmd_eq("set e {0xff}; expr $e", "255");
    cmd_eq("set x 5; set e {\"v$x\"}; expr $e", "v5"); // quoted-string operand
}

#[test]
fn expr_stk_call_error_and_unknown_function() {
    // The dynamic `ExprEval::call` path: a real function that errors, and an
    // unknown function.  tclsh 8.6/9.0:
    let (ok, result, _) = run("set e {sqrt(-1)}; expr $e");
    assert!(!ok);
    assert_eq!(result, "domain error: argument not in valid range");
    let (ok, result, _) = run("set e {nosuchfn(1)}; expr $e");
    assert!(!ok);
    assert!(result.contains("nosuchfn"), "got: {result}");
}

#[test]
fn expr_stk_unary_plus_promotion() {
    // `ExprEval::unary` Pos over a big-int and a double (dynamic path).
    // tclsh 8.6/9.0:
    cmd_eq(
        "set a 1180591620717411303424; set e {+$a}; expr $e",
        "1180591620717411303424",
    );
    cmd_eq("set a 2.5; set e {+$a}; expr $e", "2.5");
    // unary `~` over a big-int (dynamic path).
    cmd_eq(
        "set a 1180591620717411303424; set e {~$a}; expr $e",
        "-1180591620717411303425",
    );
}

#[test]
fn expr_stk_dynamic_errors() {
    // Error propagation through the dynamic ExprEval path.  tclsh 8.6/9.0:
    let (ok, result, _) = run("set e {1 / 0}; expr $e");
    assert!(!ok);
    assert_eq!(result, "divide by zero");
    let (ok, result, _) = run("set e {$nope + 1}; expr $e");
    assert!(!ok);
    assert_eq!(result, "can't read \"nope\": no such variable");
    let (ok, result, _) = run("set e {[error boom]}; expr $e");
    assert!(!ok);
    assert_eq!(result, "boom"); // command-subst error propagates
}

// expr: an unparsable expression is a syntax error

/// C Tcl's syntax errors for expressions that do not parse, through the `expr`
/// *command*.
///
/// `ParseExpr` (`tclCompExpr.c`) names the failure mode, quotes the expression
/// around the offending position, and — where the position has no offending text
/// of its own — inserts the `_@_` mark. Each expectation is a `tclsh` result
/// transcribed from C's own suite, `tests/parseExpr.test`.
///
/// These all used to succeed, yielding the expression's own source text: an
/// unparsable expression short-circuited `Vm::eval_expr` before the shared
/// walker's `syntax error in expression` could fire.
#[test]
fn expr_command_rejects_an_unparsable_expression() {
    // A dangling operator. tclsh: `missing operand at _@_` (`tclCompExpr.c:1151`)
    expr_err("1+", "missing operand at _@_\nin expression \"1+_@_\"");
    // Two operands, no operator (parseExpr-21.6's `expr {0 0}` shape).
    expr_err("1 2", "missing operator at _@_\nin expression \"1 _@_2\"");
    // parseExpr-21.19.
    expr_err("", "empty expression\nin expression \"\"");
    // A bareword run: C names the *first* word and appends the postscript
    // listing the spellings it would accept (parseExpr-21.3).
    expr_err(
        "foo bar baz",
        "invalid bareword \"foo\"\nin expression \"foo bar baz\";\n\
         should be \"$foo\" or \"{foo}\" or \"foo(...)\" or ...",
    );
    // TN: the happy path is untouched — a valid expression still evaluates.
    expr_eq("2+3", "5");
    expr_eq("1 ? 2 : 3", "2");
    expr_eq("\"a\" in {a b}", "1");
}

/// The same four failures through the `EXPR_STK` opcode (the dynamic `expr $e`
/// form, where the expression text is only known at runtime), plus the
/// `-errorcode` and `errorInfo` C attaches at the same site.
#[test]
fn expr_stk_rejects_an_unparsable_expression() {
    for (expr, message) in [
        ("1+", "missing operand at _@_\nin expression \"1+_@_\""),
        ("1 2", "missing operator at _@_\nin expression \"1 _@_2\""),
        ("", "empty expression\nin expression \"\""),
        (
            "foo bar baz",
            "invalid bareword \"foo\"\nin expression \"foo bar baz\";\n\
             should be \"$foo\" or \"{foo}\" or \"foo(...)\" or ...",
        ),
    ] {
        let (ok, result, _) = run(&format!("set e {{{expr}}}\nexpr $e"));
        assert!(
            !ok,
            "`expr $e` with {expr:?} should error, got ok: {result}"
        );
        assert_eq!(result, message, "exprStk over {expr:?}");
    }
    // `-errorcode` is `TCL PARSE EXPR <detail>` and `errorInfo` carries the
    // `(parsing expression "…")` frame (`tclCompExpr.c:1461-1469`).
    // tclsh: `TCL PARSE EXPR MISSING`
    cmd_eq(
        "set e {1+}\ncatch {expr $e} m opts\ndict get $opts -errorcode",
        "TCL PARSE EXPR MISSING",
    );
    // tclsh: `TCL PARSE EXPR BAREWORD`
    cmd_eq(
        "set e {foo bar}\ncatch {expr $e} m opts\ndict get $opts -errorcode",
        "TCL PARSE EXPR BAREWORD",
    );
    // The frame is seeded, not terminal: the enclosing `expr` still logs its own
    // `invoked from within` frame on top (as `apply`'s
    // `(parsing lambda expression …)` frame does — if-2.4 pins the same shape).
    cmd_eq(
        "set e {1+}\ncatch {expr $e} m\nset ::errorInfo",
        "missing operand at _@_\nin expression \"1+_@_\"\n    \
         (parsing expression \"1+\")\n    invoked from within\n\"expr $e\"",
    );
    // TN: a valid dynamic expression still evaluates through the same opcode.
    cmd_eq("set e {2+3}\nexpr $e", "5");
}

// expr: TIP 582 `#` comments

/// A `#` comment inside an expression, which C has supported since TIP 582
/// (`tclCompExpr.c:168` makes `COMMENT` a first-class lexeme; `:1931-1942`
/// lexes it; `:701` skips it like whitespace).
///
/// This is the regression `e1b3e534a` surfaced. Our expr lexer classified `#`
/// as an unknown character, so `parse_expr` returned `ExprNode::Raw`; while an
/// unparsable expression silently evaluated to its own source text that was
/// merely a wrong answer, but once `Raw` became C's syntax error it turned into
/// a **spurious error on valid Tcl** — `expr {1 + 2 # note}` errored where C
/// returns 3.
#[test]
fn expr_accepts_a_comment_as_c_does() {
    // The headline case: a trailing comment. C returns 3.
    expr_eq("1 + 2 # note", "3");
    // A comment after a command substitution (the operand is not a literal).
    expr_eq("[llength {a b}] # c", "2");
    // expr-62.1: the comment eats the rest of the line, so this is just `1`.
    expr_eq("1 # + 2", "1");
    // expr-62.3's shape: comments interleaved with a multi-line expression.
    expr_eq(
        "\n\t# a comment\n\t1 + 2 + 3\n\t# another\n\t+ 4 + 5\n",
        "15",
    );
    // expr-62.7/8/9/10: comments inside a function call, including between the
    // function name and its `(` — C's `:750` "obscuring comments" lookahead.
    expr_eq("max(1,# comment\n2)", "2");
    expr_eq("max(1# comment\n,2)", "2");
    expr_eq("max(# comment\n1,2)", "2");
    expr_eq("max# comment\n(1,2)", "2");
    // A comment containing `#` is one comment, not two.
    expr_eq("1 + 2 # a # b", "3");
    // Comments are skipped in the dynamic `expr $e` form too (the EXPR_STK
    // opcode), not just the compiled one.
    cmd_eq("set e {1 + 2 # note}\nexpr $e", "3");
    // …and in a condition, where the same parse feeds a branch.
    cmd_eq(
        "if {1 + 1 == 2 # sanity\n} {set r yes} else {set r no}",
        "yes",
    );
}

/// The terminating newline is not part of the comment (C returns
/// `size - (byte == '\n')`), so an expression continues on the next line.
/// expr-62.2 pins exactly this: `expr "1 #\n+ 2"` is 3, not 1.
#[test]
fn a_comment_ends_at_its_newline_not_the_expression() {
    expr_eq("1 #\n+ 2", "3");
    expr_eq("1 # comment\n+ 2", "3");
    // expr-62.5/62.6: a comment does not splice the tokens it separates —
    // `$a#…\nne#…\nfalse` stays three lexemes, so this is a `ne` comparison.
    cmd_eq(
        "set a False\nexpr {$a#don't splice\nne#don't splice\nfalse}",
        "1",
    );
    cmd_eq("expr {0x2#don't splice\nne#don't splice\n2}", "1");
}

/// A comment-only body has no operands, so it is C's *empty* expression —
/// `ParseExpr` skips the comment and reaches `END` with `lastParsed == START`
/// (`tclCompExpr.c:1135`). The comment is not an operand and not an error of
/// its own.
#[test]
fn a_comment_only_expression_is_the_empty_expression() {
    expr_err("# c", "empty expression\nin expression \"# c\"");
    expr_err("#", "empty expression\nin expression \"#\"");
    cmd_eq(
        "set e {# c}\ncatch {expr $e} m opts\ndict get $opts -errorcode",
        "TCL PARSE EXPR EMPTY",
    );
}

/// The regression guard, stated as the bug: valid Tcl carrying an expression
/// comment must not raise the syntax error `e1b3e534a` introduced.
#[test]
fn a_comment_does_not_raise_the_unparsable_expression_error() {
    for script in [
        "expr {1 + 2 # note}",
        "expr {[llength {a b}] # c}",
        "expr {max# c\n(1,2)}",
        "set e {1 + 2 # note}\nexpr $e",
        "if {1 # ok\n} {set r y}",
        "set i 0\nwhile {$i < 2 # bound\n} {incr i}\nset i",
    ] {
        let (ok, result, _) = run(script);
        assert!(
            ok,
            "valid Tcl with an expr comment must not error, but {script:?} gave: {result}"
        );
        for fragment in [
            "syntax error",
            "invalid character",
            "empty expression",
            "missing operand",
            "invalid bareword",
        ] {
            assert!(
                !result.contains(fragment),
                "{script:?} reported {fragment:?}: {result}"
            );
        }
    }
}

/// A `switch` subject and a `[…]` expression operand are *words*, not
/// expressions — neither may be re-parsed as one.
///
/// Both used to be lowered through `exprStk`, so both depended on an unparsable
/// expression evaluating to its own text: `switch -- abc …` ran `expr {abc}`, and
/// `[…]` was pushed (which substitutes it) and then evaluated a second time.
/// Where the intermediate text *was* parsable the double evaluation gave a wrong
/// answer outright, which is what the `1+1` / `1+2` vectors pin.
#[test]
fn a_word_operand_is_not_re_parsed_as_an_expression() {
    // tclsh: `lit` — the subject is the string `1+1`, so the `1+1` arm matches.
    assert_eq!(
        run("switch -- 1+1 {2 {set r two} 1+1 {set r lit} default {set r miss}}").1,
        "lit"
    );
    // tclsh: `hit` for every subject shape.
    for subject in ["abc", "$s", "a$t", "[string cat a b]", "0o289", "-"] {
        let script =
            format!("set s abc; set t bc; switch -- {subject} {{{{}} {{}} default {{set r hit}}}}");
        // A subject with no matching arm takes `default`; the point is that it
        // runs at all.
        let (ok, result, _) = run(&script);
        assert!(ok, "switch over {subject:?}: {result}");
        assert_eq!(result, "hit", "switch over {subject:?}");
    }
    // tclsh: `1+2` — `expr` returns the command's result, not `3`.
    cmd_eq("set x {1+2}\nexpr {[set x]}", "1+2");
    // tclsh: `1` — the `in` right operand is the command's *list* result.
    cmd_eq("expr {\"set\" in [info commands]}", "1");
    // tclsh: `a b` — a `[…]` operand keeps its string value.
    cmd_eq("expr {[list a b]}", "a b");
}

#[test]
fn expr_integer_arithmetic() {
    // tclsh: shared 8.6/9.0
    expr_eq("3 + 4", "7");
    expr_eq("10 - 3", "7");
    expr_eq("6 * 7", "42");
    expr_eq("20 / 4", "5");
    expr_eq("17 % 5", "2");
    expr_eq("2 ** 10", "1024");
    // unary
    expr_eq("-5", "-5");
    expr_eq("+5", "5");
    expr_eq("- -5", "5");
}

#[test]
fn expr_floored_division_and_modulo() {
    // Tcl `/` rounds toward negative infinity; `%` takes the divisor's sign.
    // tclsh: -7 / 2 -> -4 ; -7 % 2 -> 1
    expr_eq("-7 / 2", "-4");
    expr_eq("-7 % 2", "1");
    expr_eq("7 / -2", "-4");
    expr_eq("7 % -2", "-1");
    expr_eq("-10 % 3", "2");
    expr_eq("10 % -3", "-2");
    expr_eq("-10 / 3", "-4");
}

#[test]
fn expr_division_by_zero_errors() {
    // tclsh 8.6/9.0: both raise "divide by zero".
    expr_err("1 / 0", "divide by zero");
    expr_err("5 % 0", "divide by zero");
    expr_err("0 / 0", "divide by zero");
}

#[test]
fn expr_power_right_associative() {
    // `**` is right-associative: 2 ** 3 ** 2 == 2 ** 9 == 512.  tclsh: 512
    expr_eq("2 ** 3 ** 2", "512");
    // Negative exponent of an integer base other than +/-1 is 0 (integer pow).
    // tclsh: 2 ** -1 -> 0
    expr_eq("2 ** -1", "0");
    expr_eq("1 ** -5", "1");
    expr_eq("-1 ** -3", "-1"); // (-1)**odd
    expr_eq("-1 ** -2", "1"); // (-1)**even
}

#[test]
fn expr_integer_promotion_on_overflow() {
    // The VM promotes i64 overflow into i128 (its bounded bignum stand-in),
    // matching tclsh's bignum on the values that fit i128.
    // tclsh: 9223372036854775807 + 1 -> 9223372036854775808
    expr_eq("9223372036854775807 + 1", "9223372036854775808");
    expr_eq("2 ** 64", "18446744073709551616");
    expr_eq("2 ** 100", "1267650600228229401496703205376");
    expr_eq("1000000000000 * 1000000000000", "1000000000000000000000000");
}

// expr: floating-point arithmetic and formatting

#[test]
fn expr_double_arithmetic() {
    // tclsh: shared 8.6/9.0
    expr_eq("1.5 + 2.5", "4.0");
    expr_eq("5.0 - 1.5", "3.5");
    expr_eq("2.0 * 3.0", "6.0");
    expr_eq("7.0 / 2.0", "3.5");
    expr_eq("2.5 ** 2", "6.25");
    // mixed int/double promotes to double
    expr_eq("2 * 1.5", "3.0");
    expr_eq("5 / 2.0", "2.5");
}

#[test]
fn expr_double_infinity() {
    // Division of a finite double by 0.0 is +/-Inf (NOT "divide by zero").
    // tclsh 8.6/9.0: 5.0/0.0 -> Inf ; -5.0/0.0 -> -Inf
    expr_eq("5.0 / 0.0", "Inf");
    expr_eq("-5.0 / 0.0", "-Inf");
    expr_eq("1.0 / 0.0", "Inf");
}

#[test]
fn expr_double_default_formatting() {
    // Tcl's default %g-ish formatting (17-sig shortest round-trip). These all
    // go through an operation/function (not a bare literal), so the VM emits the
    // canonical form.  tclsh 8.6/9.0:
    expr_eq("double(9223372036854775807)", "9.223372036854776e+18");
    expr_eq("1e3 + 0.0", "1000.0");
    expr_eq("3.0e-2 / 2", "0.015");
    expr_eq("0.1 + 0.2", "0.30000000000000004");
    expr_eq("1.0 * 1000000000000000.0", "1000000000000000.0");
}

// BUG: a *bare* literal (or bare `$var`) that is the entire `expr` is returned
// verbatim instead of being coerced to its canonical numeric string. tclsh
// normalizes the single operand to its number form. The VM only normalizes when
// the literal participates in an operation (the dynamic `expr $v` form, which
// recompiles, is also correct — the divergence is specific to a single-operand
// braced `expr {…}`).
//   script:        expr {1e3}            tclsh 8.6/9.0: 1000.0   tcl-vm: 1e3
//   script:        expr {0xff}           tclsh 8.6/9.0: 255      tcl-vm: 0xff
//   script:        expr {0o17}           tclsh 8.6/9.0: 15       tcl-vm: 0o17
//   script:        set v 1e3; expr {$v}  tclsh 8.6/9.0: 1000.0   tcl-vm: 1e3
#[test]
fn bug_bare_literal_operand_not_normalized() {
    expr_eq("1e3", "1000.0"); // tclsh: 1000.0
    expr_eq("5e0", "5.0"); // tclsh: 5.0
    expr_eq("1.5e-2", "0.015"); // tclsh: 0.015
    expr_eq("0xff", "255"); // tclsh: 255
    expr_eq("0o17", "15"); // tclsh: 15
    expr_eq("0b1010", "10"); // tclsh: 10
    // A bare `$var` operand whose value is an un-normalized number, likewise.
    let (ok, result, _) = run("set v 1e3; expr {$v}");
    assert!(ok, "expr {{$v}} should evaluate: {result}");
    assert_eq!(result, "1000.0"); // tclsh: 1000.0
}

// BUG: a double operation that produces NaN from non-NaN operands is a
// "domain error" in tclsh (tclExecute.c checks the result), but the VM's
// `dbl_arith` returns the NaN silently and stringifies it as "NaN".
//   script:        expr {0.0 / 0.0}
//   tclsh 8.6/9.0: domain error: argument not in valid range
//   tcl-vm:        NaN (ok=true)
#[test]
fn bug_double_nan_result_is_domain_error() {
    // tclsh: expr {0.0 / 0.0} -> domain error: argument not in valid range
    expr_err("0.0 / 0.0", "domain error: argument not in valid range");
    // tclsh: expr {Inf - Inf} -> domain error: argument not in valid range
    expr_err("Inf - Inf", "domain error: argument not in valid range");
}

// expr: bitwise & shift operators (& | ^ ~ << >>)

#[test]
fn expr_bitwise_operators() {
    // tclsh: shared 8.6/9.0
    expr_eq("12 & 10", "8");
    expr_eq("12 | 10", "14");
    expr_eq("12 ^ 10", "6");
    expr_eq("~5", "-6");
    expr_eq("~0", "-1");
    expr_eq("~-1", "0");
}

#[test]
fn expr_shift_operators() {
    // tclsh: shared 8.6/9.0
    expr_eq("1 << 4", "16");
    expr_eq("256 >> 2", "64");
    expr_eq("17 << 2", "68");
    // arithmetic right shift keeps the sign
    expr_eq("-8 >> 1", "-4");
    expr_eq("-1 >> 10", "-1");
    // a large left shift promotes through i128
    expr_eq("1 << 70", "1180591620717411303424");
}

#[test]
fn expr_integer_only_operator_rejects_float() {
    // The bitwise/shift/% operators reject a (valid) floating-point operand,
    // naming the offending side. tclsh 9.0 wording (VM tracks 9.0):
    expr_err(
        "3 & 2.0",
        "cannot use floating-point value \"2.0\" as right operand of \"&\"",
    );
    expr_err(
        "2.0 | 3",
        "cannot use floating-point value \"2.0\" as left operand of \"|\"",
    );
    expr_err(
        "5.5 % 2",
        "cannot use floating-point value \"5.5\" as left operand of \"%\"",
    );
    expr_err(
        "1.0 << 2",
        "cannot use floating-point value \"1.0\" as left operand of \"<<\"",
    );
}

// expr: comparison operators (< > <= >= == !=) and string forms (eq ne in ni)

#[test]
fn expr_numeric_comparisons() {
    // tclsh: shared 8.6/9.0
    expr_eq("9 < 10", "1");
    expr_eq("10 < 9", "0");
    expr_eq("5 <= 5", "1");
    expr_eq("5 >= 6", "0");
    expr_eq("3 > 2", "1");
    expr_eq("3 == 3", "1");
    expr_eq("3 != 4", "1");
    // numeric `==` ignores representation: 10 == 10.0
    expr_eq("10 == 10.0", "1");
    expr_eq("10 != 10.0", "0");
}

#[test]
fn expr_string_vs_numeric_comparison() {
    // `<` on two numeric-looking strings compares numerically (9 < 10),
    // but on non-numeric strings it is lexical.  tclsh: shared 8.6/9.0
    expr_eq("\"9\" < \"10\"", "1"); // numeric
    expr_eq("\"abc\" < \"abd\"", "1"); // lexical
    expr_eq("\"abd\" < \"abc\"", "0");
    // mixed: a numeric and a non-numeric operand fall back to string compare
    expr_eq("\"10\" < \"foo\"", "1"); // "1" < "f"
}

#[test]
fn expr_string_equality_operators() {
    // `eq`/`ne` are always string comparisons: 10 eq 10.0 is false (distinct
    // strings) even though 10 == 10.0 is true.  tclsh: shared 8.6/9.0
    expr_eq("\"abc\" eq \"abc\"", "1");
    expr_eq("\"abc\" ne \"abd\"", "1");
    expr_eq("10 eq 10", "1");
    expr_eq("10 eq 10.0", "0"); // string, not numeric
    expr_eq("10 ne 10.0", "1");
}

#[test]
fn expr_in_and_ni_operators() {
    // `in`/`ni` test list membership by string equality.  tclsh: shared 8.6/9.0
    expr_eq("\"b\" in {a b c}", "1");
    expr_eq("\"z\" in {a b c}", "0");
    expr_eq("\"z\" ni {a b c}", "1");
    expr_eq("\"b\" ni {a b c}", "0");
    expr_eq("2 in {1 2 3}", "1");
    expr_eq("\"a b\" in {{a b} c}", "1"); // multi-word element
}

// expr: logical operators (! && ||) and the ternary ?:

#[test]
fn expr_logical_operators() {
    // tclsh: shared 8.6/9.0
    expr_eq("1 && 1", "1");
    expr_eq("1 && 0", "0");
    expr_eq("0 || 1", "1");
    expr_eq("0 || 0", "0");
    expr_eq("!0", "1");
    expr_eq("!5", "0"); // any nonzero is true
    expr_eq("!1", "0");
    // boolean words
    expr_eq("true && false", "0");
    expr_eq("yes || no", "1");
    expr_eq("!true", "0");
}

#[test]
fn expr_short_circuit_evaluation() {
    // `&&`/`||` short-circuit: the divide-by-zero on the dead side is not
    // evaluated.  tclsh 8.6/9.0: both yield a clean boolean.
    expr_eq("0 && (1/0)", "0");
    expr_eq("1 || (1/0)", "1");
}

#[test]
fn expr_ternary_operator() {
    // tclsh: shared 8.6/9.0
    expr_eq("1 ? 10 : 20", "10");
    expr_eq("0 ? 10 : 20", "20");
    expr_eq("5 > 3 ? \"yes\" : \"no\"", "yes");
    // nested / right-associative
    expr_eq("1 ? 2 ? 3 : 4 : 5", "3");
    expr_eq("0 ? 1 : 1 ? 2 : 3", "2");
    // only the taken branch is evaluated (dead side has a divide-by-zero)
    expr_eq("1 ? 42 : (1/0)", "42");
}

// expr: unary operand-type errors

#[test]
fn expr_unary_operand_errors() {
    // tclsh 9.0 wording (the VM tracks 9.0's operand-type messages):
    // `~` on a double / non-number
    expr_err(
        "~3.5",
        "cannot use floating-point value \"3.5\" as operand of \"~\"",
    );
    expr_err(
        "~\"abc\"",
        "cannot use non-numeric string \"abc\" as operand of \"~\"",
    );
    // `-` / `+` on a non-number
    expr_err(
        "-\"abc\"",
        "cannot use non-numeric string \"abc\" as operand of \"-\"",
    );
}

#[test]
fn expr_binary_operand_errors() {
    // A non-numeric operand to `+` names the side (tclsh 9.0 wording).
    expr_err(
        "\"abc\" + 1",
        "cannot use non-numeric string \"abc\" as left operand of \"+\"",
    );
    expr_err(
        "1 + \"abc\"",
        "cannot use non-numeric string \"abc\" as right operand of \"+\"",
    );
}

// expr: literals — hex / octal / binary / scientific

#[test]
fn expr_radix_literals() {
    // Radix prefixes parsed in an arithmetic context (a bare literal hits the
    // not-normalized bug, covered separately).  tclsh: shared 8.6/9.0 (avoiding
    // bare 0NNN octal, which 8.6/9.0 disagree on).
    expr_eq("0xff + 0", "255");
    expr_eq("0xFF * 1", "255");
    expr_eq("0o17 + 0", "15"); // explicit octal prefix
    expr_eq("0b1010 + 0", "10"); // binary
    expr_eq("0xFF + 0b10", "257");
    expr_eq("0x10 * 0o10", "128"); // 16 * 8
}

#[test]
fn expr_scientific_notation() {
    // Scientific notation parsed in an arithmetic context.  tclsh: shared 8.6/9.0
    expr_eq("1e3 + 0.0", "1000.0");
    expr_eq("2.5e2 + 0.0", "250.0");
    expr_eq("1.5e-2 * 1.0", "0.015");
    expr_eq("1E2 + 1", "101.0");
    expr_eq("6.022e23 > 1", "1");
}

// expr: precedence & associativity

#[test]
fn expr_precedence_and_associativity() {
    // tclsh: shared 8.6/9.0
    expr_eq("2 + 3 * 4", "14"); // * before +
    expr_eq("2 + 3 * 4 - 1", "13");
    expr_eq("(2 + 3) * 4", "20"); // parens
    expr_eq("10 - 3 - 2", "5"); // - is left-assoc
    expr_eq("1 << 2 + 1", "8"); // + before << : 1 << 3
    expr_eq("5 & 3 | 8", "9"); // & before |
    expr_eq("1 | 2 & 0", "1"); // & (=0) before | : 1 | 0
    expr_eq("3 + 4 == 7", "1"); // + before ==
    expr_eq("2 * 3 ** 2", "18"); // ** before * : 2 * 9
}

// expr: variable / command substitution / quoted-string operands

#[test]
fn expr_variable_and_command_operands() {
    // a $var operand
    cmd_eq("set x 5; expr {$x * 2}", "10");
    // a [command] operand
    cmd_eq("expr {[string length hello] + 1}", "6");
    // a quoted-string operand substitutes $var/[cmd]
    cmd_eq("set i 7; expr {\"item $i\"}", "item 7");
}

#[test]
fn expr_unknown_variable_errors() {
    // tclsh: can't read "nope": no such variable
    let (ok, result, _) = run("expr {$nope + 1}");
    assert!(!ok);
    assert_eq!(result, "can't read \"nope\": no such variable");
}

// expr with *variable* operands: these are NOT const-folded by the
// compiler, so they drive the VM's runtime arithmetic tower in `expr.rs`
// (`arith`/`int_arith`/`dbl_arith`/`ipow`/`unary`), not just the codegen
// constant fold. Each value is pinned to tclsh 8.6/9.0 (which agree).

/// Helper: run `expr {EXPR}` with `a`/`b` bound first.
fn expr_ab(a: &str, b: &str, e: &str, want: &str) {
    let (ok, result, _) = run(&format!("set a {a}; set b {b}; expr {{{e}}}"));
    assert!(ok, "`{e}` (a={a} b={b}) should evaluate: {result}");
    assert_eq!(result, want, "expr {{{e}}} with a={a} b={b}");
}

/// Helper: run `expr {EXPR}` with `a`/`b` bound first, expecting error `msg`.
fn expr_ab_err(a: &str, b: &str, e: &str, msg: &str) {
    let (ok, result, _) = run(&format!("set a {a}; set b {b}; expr {{{e}}}"));
    assert!(!ok, "`{e}` (a={a} b={b}) should error: {result}");
    assert_eq!(result, msg, "expr {{{e}}} with a={a} b={b}");
}

#[test]
fn expr_runtime_integer_arith() {
    // Integer path through `int_arith` (variable operands).  tclsh 8.6/9.0:
    expr_ab("5", "2", "$a + $b", "7");
    expr_ab("5", "2", "$a - $b", "3");
    expr_ab("5", "2", "$a * $b", "10");
    expr_ab("5", "2", "$a / $b", "2"); // floor
    expr_ab("-7", "2", "$a / $b", "-4"); // floored toward -inf
    expr_ab("-7", "2", "$a % $b", "1"); // sign of divisor
    expr_ab("2", "10", "$a ** $b", "1024");
    expr_ab("12", "10", "$a & $b", "8");
    expr_ab("12", "10", "$a | $b", "14");
    expr_ab("12", "10", "$a ^ $b", "6");
    expr_ab("1", "4", "$a << $b", "16");
    expr_ab("256", "2", "$a >> $b", "64");
    expr_ab("-8", "1", "$a >> $b", "-4"); // arithmetic shift keeps sign
}

#[test]
fn expr_runtime_integer_promotion() {
    // i64 overflow promotes through i128 in `int_value` (variable operands).
    // tclsh 8.6/9.0:
    expr_ab("9223372036854775807", "1", "$a + $b", "9223372036854775808");
    expr_ab("2", "70", "$a ** $b", "1180591620717411303424");
    expr_ab("2", "100", "$a ** $b", "1267650600228229401496703205376");
    expr_ab("1", "70", "$a << $b", "1180591620717411303424");
    expr_ab(
        "1180591620717411303424",
        "2",
        "$a * $b",
        "2361183241434822606848",
    );
}

/// The 61-digit value of `2**200`, an integer past the VM's `i128` fast tier —
/// operands this size exercise the arbitrary-precision (`num-bigint`) path.
const P200: &str = "1606938044258990275541962092341162602522202993782792835301376";

#[test]
fn expr_runtime_bignum_operands_past_i128() {
    // An operand already past i128 (e.g. a prior 2**200 result) is numeric —
    // it reaches the exact bignum path instead of erroring as a non-numeric
    // string (the operand gap).  tclsh 8.6/9.0:
    expr_ab(
        P200,
        "1",
        "$a + $b",
        "1606938044258990275541962092341162602522202993782792835301377",
    );
    expr_ab(P200, "7", "$a % $b", "4");
    expr_ab(
        P200,
        "-3",
        "$a / $b",
        "-535646014752996758513987364113720867507400997927597611767126",
    );
    expr_ab(P200, "137", "$a >> $b", "9223372036854775808");
    expr_ab(P200, "255", "$a & $b", "0");
    // Unary over a beyond-i128 operand: exact negate and complement.
    cmd_eq(&format!("set a {P200}; expr {{-$a}}"), &format!("-{P200}"));
    cmd_eq(
        &format!("set a {P200}; expr {{~$a}}"),
        "-1606938044258990275541962092341162602522202993782792835301377",
    );
    // The bignum tier keeps C's error text.  tclsh: divide by zero
    expr_ab_err(P200, "0", "$a / $b", "divide by zero");
    expr_ab_err(P200, "0", "$a % $b", "divide by zero");
}

#[test]
fn expr_runtime_bignum_float_contagion() {
    // A bignum operand mixed with a float converts to its nearest double
    // (C's `TclBignumToDouble`, round-to-nearest-even) and computes in f64 —
    // never an error.  tclsh 8.6/9.0 agree on the *values*; the printed forms
    // here are this toolchain's shortest round-trip rendering (tclsh 8.6's
    // printer emits "1.844674407370955e+19" for exactly 2^64, a string that
    // re-parses as 2^64 - 2048; `entier(1.5 + 18446744073709551616)` in 8.6
    // confirms the value is 18446744073709551616).
    expr_ab(
        "1.5",
        "18446744073709551616",
        "$a + $b",
        "1.8446744073709552e+19",
    );
    expr_ab(
        "1.5",
        "18446744073709551616",
        "$a * $b",
        "2.7670116110564327e+19",
    );
    // tclsh 8.6/9.0: expr {2**200 + 1.5} → 1.6069380442589903e+60
    expr_ab(P200, "1.5", "$a + $b", "1.6069380442589903e+60");
    // An integer-only operator still faults the *float* side (Tcl 9 wording).
    expr_ab_err(
        P200,
        "1.5",
        "$a % $b",
        "cannot use floating-point value \"1.5\" as right operand of \"%\"",
    );
}

#[test]
fn expr_runtime_pow_and_shift_limits() {
    // tclsh 8.6/9.0: the `**` exponent limit is 2^28 - 1; past it the result
    // is "exponent too large" (never computed), while |base| <= 1 collapses
    // for any exponent magnitude, parity included.
    expr_ab_err("2", "268435456", "$a ** $b", "exponent too large");
    expr_ab_err(
        "2",
        "99999999999999999999",
        "$a ** $b",
        "exponent too large",
    );
    expr_ab("1", "99999999999999999999", "$a ** $b", "1");
    expr_ab("-1", "99999999999999999999", "$a ** $b", "-1"); // odd
    expr_ab("-1", "99999999999999999998", "$a ** $b", "1"); // even
    // tclsh: a left-shift count must fit C's int (`2 << 2**32` errors); a
    // right-shift by any huge count collapses to the sign.
    expr_ab_err(
        "2",
        "4294967296",
        "$a << $b",
        "integer value too large to represent",
    );
    expr_ab("2", "99999999999999999999", "$a >> $b", "0");
    expr_ab("-2", "99999999999999999999", "$a >> $b", "-1");
}

#[test]
fn expr_runtime_ipow_negative_exponent() {
    // The integer `ipow` negative-exponent special cases (variable operands).
    // tclsh 8.6/9.0:
    expr_ab("2", "-1", "$a ** $b", "0"); // |base|>1 -> 0
    expr_ab("1", "-5", "$a ** $b", "1"); // 1**anything -> 1
    expr_ab("-1", "-3", "$a ** $b", "-1"); // (-1)**odd -> -1
    expr_ab("-1", "-2", "$a ** $b", "1"); // (-1)**even -> 1
}

#[test]
fn expr_runtime_double_arith() {
    // Double path through `dbl_arith` (variable operands).  tclsh 8.6/9.0:
    expr_ab("1.5", "2.5", "$a + $b", "4.0");
    expr_ab("5.0", "2.0", "$a - $b", "3.0");
    expr_ab("2.0", "3.0", "$a * $b", "6.0");
    expr_ab("7.0", "2.0", "$a / $b", "3.5");
    expr_ab("5.0", "2.0", "$a ** $b", "25.0");
    expr_ab("5.0", "0.0", "$a / $b", "Inf"); // finite/0.0 -> Inf
    // mixed int/double promotes to double
    expr_ab("2", "1.5", "$a * $b", "3.0");
}

#[test]
fn expr_runtime_divide_by_zero() {
    // `int_arith` div/mod by zero (variable operands).  tclsh 8.6/9.0:
    expr_ab_err("1", "0", "$a / $b", "divide by zero");
    expr_ab_err("5", "0", "$a % $b", "divide by zero");
}

#[test]
fn expr_runtime_comparisons() {
    // `compare` numeric and string paths (variable operands).  tclsh 8.6/9.0:
    expr_ab("9", "10", "$a < $b", "1"); // numeric
    expr_ab("10", "9", "$a < $b", "0");
    expr_ab("5", "5", "$a == $b", "1");
    expr_ab("5", "6", "$a != $b", "1");
    expr_ab("3", "3", "$a >= $b", "1");
    expr_ab("foo", "bar", "$a < $b", "0"); // string (f > b)
    expr_ab("abc", "abc", "$a eq $b", "1"); // string eq
    expr_ab("abc", "abd", "$a ne $b", "1");
    expr_ab("10", "10.0", "$a eq $b", "0"); // string, not numeric
    expr_ab("10", "10.0", "$a == $b", "1"); // numeric
}

#[test]
fn expr_runtime_unary_operators() {
    // `unary` Neg/Pos/BitNot/Not (variable operands).  tclsh 8.6/9.0:
    expr_ab("5", "0", "-$a", "-5");
    expr_ab("5", "0", "+$a", "5");
    expr_ab("5", "0", "~$a", "-6");
    expr_ab("0", "0", "!$a", "1");
    expr_ab("5", "0", "!$a", "0");
    // unary minus through the i128 promotion path
    expr_ab("9223372036854775808", "0", "-$a", "-9223372036854775808");
}

#[test]
fn expr_runtime_operand_type_errors() {
    // `arith`/`unary` operand-type errors (variable operands, tclsh 9.0 wording).
    // The integer-only operators reject a float operand, naming the side.
    expr_ab_err(
        "1.5",
        "2",
        "$a & $b",
        "cannot use floating-point value \"1.5\" as left operand of \"&\"",
    );
    expr_ab_err(
        "5.5",
        "2",
        "$a % $b",
        "cannot use floating-point value \"5.5\" as left operand of \"%\"",
    );
    // A non-numeric string operand.
    expr_ab_err(
        "abc",
        "1",
        "$a + $b",
        "cannot use non-numeric string \"abc\" as left operand of \"+\"",
    );
    // A NaN operand to arithmetic is the "floating-point value" wording.
    let (ok, result, _) = run("set a NaN; expr {$a + 1}");
    assert!(!ok);
    assert_eq!(
        result,
        "cannot use non-numeric floating-point value \"NaN\" as left operand of \"+\""
    );
    // `~` on a double operand.
    let (ok, result, _) = run("set a 2.5; expr {~$a}");
    assert!(!ok);
    assert_eq!(
        result,
        "cannot use floating-point value \"2.5\" as operand of \"~\""
    );
}

#[test]
fn expr_runtime_unary_list_operand_error() {
    // A multi-element *list* operand to a unary operator is phrased without
    // quotes (the `max_list_length > 1` branch of `unary_operand_err`).
    // tclsh 9.0:
    let (ok, result, _) = run("set a {1 2 3}; expr {-$a}");
    assert!(!ok);
    assert_eq!(result, "cannot use a list as operand of \"-\"");
}

// BUG: a negative shift count reports "negative shift count" in the VM, but
// tclsh (8.6 and 9.0) reports "negative shift argument".
//   script:        set a 1; set b -1; expr {$a << $b}
//   tclsh 8.6/9.0: negative shift argument
//   tcl-vm:        negative shift count
#[test]
fn bug_negative_shift_error_wording() {
    expr_ab_err("1", "-1", "$a << $b", "negative shift argument");
    expr_ab_err("1", "-2", "$a >> $b", "negative shift argument");
}

#[test]
fn expr_command_substitution_operand() {
    // A `[command]` operand evaluated by `ExprEval::command`; a non-Ok
    // completion (error) propagates rather than becoming the string result.
    cmd_eq("expr {[expr {2 + 3}] * 2}", "10"); // tclsh: 10
    let (ok, result, _) = run("expr {[error boom] + 1}");
    assert!(!ok);
    assert_eq!(result, "boom"); // the error propagates, not "boom" as a value
}

#[test]
fn expr_unknown_math_function() {
    // `ExprEval::call` -> unknown function.  tclsh 8.6/9.0:
    let (ok, result, _) = run("expr {nosuchfn(1)}");
    assert!(!ok);
    // tclsh: unknown math function "nosuchfn"  (VM may word it the same)
    assert!(
        result.contains("nosuchfn"),
        "expected the unknown-function name in: {result}"
    );
}

#[test]
fn expr_read_trace_fires_on_operand() {
    // `ExprEval::var` fires a read trace; a callback error aborts the read.
    let (ok, result, _) = run(concat!(
        "set x 5\n",
        "proc tr args {error nope}\n",
        "trace add variable x read tr\n",
        "catch {expr {$x + 1}} m\n",
        "set m",
    ));
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "can't read \"x\": nope"); // tclsh wraps the trace error
}

// tcl::mathfunc: integer / rounding / conversion functions

#[test]
fn mathfunc_abs() {
    // tclsh: shared 8.6/9.0
    expr_eq("abs(-5)", "5");
    expr_eq("abs(5)", "5");
    expr_eq("abs(0)", "0");
    expr_eq("abs(-5.5)", "5.5"); // double stays double
    expr_eq("abs(3.0)", "3.0");
}

// BUG: abs() of the most-negative wide overflows. tclsh promotes to a bignum
// and returns the positive magnitude; the VM uses `i64::wrapping_abs`, which
// leaves the most-negative wide unchanged (negative).
//   script:        expr {abs(-9223372036854775808)}
//   tclsh 8.6/9.0: 9223372036854775808
//   tcl-vm:        -9223372036854775808
#[test]
fn bug_abs_of_most_negative_wide() {
    // tclsh: expr {abs(-9223372036854775808)} -> 9223372036854775808
    expr_eq("abs(-9223372036854775808)", "9223372036854775808");
}

#[test]
fn mathfunc_int_wide_entier() {
    // int()/wide()/entier() truncate toward zero.  tclsh: shared 8.6/9.0
    expr_eq("int(3.9)", "3");
    expr_eq("int(-3.9)", "-3");
    expr_eq("int(3.0)", "3");
    expr_eq("wide(3.9)", "3");
    expr_eq("wide(-1)", "-1");
    expr_eq("entier(3.9)", "3");
    expr_eq("entier(-3.9)", "-3");
    expr_eq("entier(42)", "42"); // integer passes through
}

#[test]
fn mathfunc_wide_truncates_out_of_range_literal() {
    // wide() wraps an out-of-range integer literal two's-complement to 64 bits.
    // tclsh 8.6/9.0: wide(0x8000000000000000) -> -9223372036854775808
    expr_eq("wide(0x8000000000000000)", "-9223372036854775808");
    expr_eq("wide(0xFFFFFFFFFFFFFFFF)", "-1");
}

#[test]
fn mathfunc_int_wide_are_the_64bit_window() {
    // `wide()` is the mod-2^64 window in every release: an integer of any
    // magnitude folds two's-complement; a double truncates toward zero first,
    // then wraps the same way (variable args defeat const folding).
    //
    // `int()` is **not** the same function at 9.0 — TIP 237's unbounded
    // `ExprIntFunc` (issue #1382). This suite runs the VM at its default 9.0
    // release, so `int()` here is `entier()`; the 8.6 window is pinned
    // separately (`tcl-vm/tests/numeric_tower_e2e.rs`
    // `int_follows_the_vms_release`, which drives both releases).
    // Measured, tclsh9.0.4 / tclsh8.6.16:
    //   int(18446744073709551617) -> 18446744073709551617 / 1
    //   int(1.5e19)               -> 15000000000000000000 / -3446744073709551616
    //   wide(...)                 -> the 8.6 column in both releases
    cmd_eq(
        "set x 18446744073709551617; expr {int($x)}",
        "18446744073709551617",
    );
    cmd_eq("set x 18446744073709551617; expr {wide($x)}", "1");
    cmd_eq(
        "set x -18446744073709551617; expr {int($x)}",
        "-18446744073709551617",
    );
    cmd_eq("set x 1e300; expr {wide($x)}", "0"); // 10^300 divides by 2^64
    cmd_eq("set x 1.5e19; expr {int($x)}", "15000000000000000000");
    cmd_eq("set x 1.5e19; expr {wide($x)}", "-3446744073709551616");
    cmd_eq("set x 2.5e19; expr {int($x)}", "25000000000000000000");
    cmd_eq("set x 2.5e19; expr {wide($x)}", "6553255926290448384");
    cmd_eq(
        "set x 9223372036854775808.0; expr {int($x)}",
        "9223372036854775808",
    );
    cmd_eq(
        "set x 9223372036854775808.0; expr {wide($x)}",
        "-9223372036854775808",
    );
    // Inf cannot wrap; NaN is not a number at all.  tclsh 8.6/9.0:
    cmd_err(
        "set x Inf; expr {int($x)}",
        "integer value too large to represent",
    );
    cmd_err(
        "set x Inf; expr {wide($x)}",
        "integer value too large to represent",
    );
    cmd_err(
        "set x NaN; expr {int($x)}",
        "floating point value is Not a Number",
    );
}

#[test]
fn mathfunc_entier_round_are_exact_unbounded() {
    // entier()/round() are exact and *unbounded* — no 64-bit window.  tclsh
    // 8.6/9.0 return the full 301-digit integer for 1e300 (the exact value of
    // the nearest double to 10^300), where int()/wide() wrap to 0.
    let full_1e300 = "1000000000000000052504760255204420248704468581108159154915854115511802457988908195786371375080447864043704443832883878176942523235360430575644792184786706982848387200926575803737830233794788090059368953234970799945081119038967640880074652742780142494579258788820056842838115669472196386865459400540160";
    cmd_eq("set x 1e300; expr {entier($x)}", full_1e300);
    cmd_eq("set x 1e300; expr {round($x)}", full_1e300);
    cmd_eq("set x 1.5e19; expr {entier($x)}", "15000000000000000000");
    // Integers of any magnitude pass through exactly.  tclsh: round(2**200)
    // is 2^200 itself.
    cmd_eq(&format!("set x {P200}; expr {{round($x)}}"), P200);
    cmd_eq(&format!("set x {P200}; expr {{entier($x)}}"), P200);
    cmd_err(
        "set x Inf; expr {entier($x)}",
        "integer value too large to represent",
    );
    cmd_err(
        "set x NaN; expr {round($x)}",
        "floating point value is Not a Number",
    );
}

#[test]
fn mathfunc_abs_and_double_stay_bignum_exact() {
    // abs() of an integer past i64 — or past i128 — is the exact magnitude,
    // never a rounded double.  tclsh 8.6/9.0 (2^64 and 2^150):
    cmd_eq(
        "set x -18446744073709551616; expr {abs($x)}",
        "18446744073709551616",
    );
    cmd_eq(
        "set x -1427247692705959881058285969449495136382746624; expr {abs($x)}",
        "1427247692705959881058285969449495136382746624",
    );
    // double() of a bignum is C's `TclBignumToDouble` conversion.
    // tclsh 8.6/9.0: double(2**200) → 1.6069380442589903e+60
    cmd_eq(
        &format!("set x {P200}; expr {{double($x)}}"),
        "1.6069380442589903e+60",
    );
    // tclsh 8.6/9.0: double(nan) is the NaN domain error.
    cmd_err(
        "set x NaN; expr {double($x)}",
        "floating point value is Not a Number",
    );
}

#[test]
fn mathfunc_double() {
    // tclsh: shared 8.6/9.0
    expr_eq("double(5)", "5.0");
    expr_eq("double(0)", "0.0");
    expr_eq("double(-3)", "-3.0");
    expr_eq("double(2.5)", "2.5"); // already double
}

#[test]
fn mathfunc_round() {
    // round() rounds half away from zero, returns an integer.  tclsh: 8.6/9.0
    expr_eq("round(2.5)", "3");
    expr_eq("round(2.4)", "2");
    expr_eq("round(-2.5)", "-3");
    expr_eq("round(0.5)", "1");
    expr_eq("round(-0.5)", "-1");
    expr_eq("round(1.5)", "2");
    expr_eq("round(7)", "7"); // integer passes through
}

#[test]
fn mathfunc_floor_ceil() {
    // floor()/ceil() return doubles.  tclsh: shared 8.6/9.0
    expr_eq("floor(2.7)", "2.0");
    expr_eq("floor(-2.1)", "-3.0");
    expr_eq("ceil(2.1)", "3.0");
    expr_eq("ceil(-2.7)", "-2.0");
    expr_eq("floor(5.0)", "5.0");
}

#[test]
fn mathfunc_isqrt() {
    // isqrt() is the integer floor of the square root.  tclsh: shared 8.6/9.0
    expr_eq("isqrt(0)", "0");
    expr_eq("isqrt(1)", "1");
    expr_eq("isqrt(15)", "3");
    expr_eq("isqrt(16)", "4");
    expr_eq("isqrt(17)", "4");
    expr_eq("isqrt(24)", "4");
    expr_eq("isqrt(25)", "5");
    expr_eq("isqrt(1000000000000000000)", "1000000000"); // exact near 1e18
}

#[test]
fn mathfunc_isqrt_negative_errors() {
    // tclsh: expr {isqrt(-1)} -> square root of negative argument
    expr_err("isqrt(-1)", "square root of negative argument");
    expr_err("isqrt(-100)", "square root of negative argument");
}

#[test]
fn mathfunc_bool() {
    // bool() coerces a boolean-ish value to 0/1.  tclsh: shared 8.6/9.0
    expr_eq("bool(5)", "1");
    expr_eq("bool(0)", "0");
    expr_eq("bool(1)", "1");
    expr_eq("bool(true)", "1");
    expr_eq("bool(false)", "0");
    expr_eq("bool(yes)", "1");
    expr_eq("bool(2.5)", "1");
}

// tcl::mathfunc: sqrt / pow / exp / log and the domain guard

#[test]
fn mathfunc_sqrt_pow_exp_log() {
    // tclsh: shared 8.6/9.0
    expr_eq("sqrt(16)", "4.0");
    expr_eq("sqrt(2)", "1.4142135623730951");
    expr_eq("pow(2,10)", "1024.0");
    expr_eq("pow(2.0,0.5)", "1.4142135623730951");
    expr_eq("exp(0)", "1.0");
    expr_eq("exp(1)", "2.718281828459045");
    expr_eq("log(1)", "0.0");
    expr_eq("log10(1000)", "3.0");
    expr_eq("log10(100)", "2.0");
}

#[test]
fn mathfunc_log_pole_is_negative_infinity() {
    // log(0) is the pole -Inf (allowed), NOT a domain error.  tclsh: 8.6/9.0
    expr_eq("log(0)", "-Inf");
    expr_eq("log10(0)", "-Inf");
}

#[test]
fn mathfunc_domain_errors() {
    // A non-NaN argument that yields NaN is a domain error.  tclsh: 8.6/9.0
    expr_err("sqrt(-1)", "domain error: argument not in valid range");
    expr_err("log(-1)", "domain error: argument not in valid range");
    expr_err("log10(-5)", "domain error: argument not in valid range");
    expr_err("acos(2)", "domain error: argument not in valid range");
    expr_err("asin(2)", "domain error: argument not in valid range");
    expr_err("pow(-1,0.5)", "domain error: argument not in valid range");
}

#[test]
fn mathfunc_pow_negative_base_pole() {
    // pow(0,-1) is the pole Inf (allowed).  tclsh 8.6/9.0: Inf
    expr_eq("pow(0,-1)", "Inf");
}

// tcl::mathfunc: trigonometric / two-arg functions

#[test]
fn mathfunc_trigonometric() {
    // tclsh: shared 8.6/9.0
    expr_eq("sin(0)", "0.0");
    expr_eq("cos(0)", "1.0");
    expr_eq("tan(0)", "0.0");
    expr_eq("asin(1)", "1.5707963267948966");
    expr_eq("acos(1)", "0.0");
    expr_eq("atan(1)", "0.7853981633974483");
}

#[test]
fn mathfunc_two_arg_functions() {
    // atan2 / hypot / fmod.  tclsh: shared 8.6/9.0
    expr_eq("atan2(1,1)", "0.7853981633974483");
    expr_eq("hypot(3,4)", "5.0");
    expr_eq("hypot(3.0,4.0)", "5.0");
    expr_eq("fmod(7,3)", "1.0");
    expr_eq("fmod(5.5,2.0)", "1.5");
}

#[test]
fn mathfunc_fmod_by_zero_is_domain_error() {
    // fmod(x, 0) yields NaN -> domain error.  tclsh 8.6/9.0:
    expr_err(
        "fmod(5.0, 0.0)",
        "domain error: argument not in valid range",
    );
    expr_err("fmod(7, 0)", "domain error: argument not in valid range");
}

// tcl::mathfunc: classification predicates (isfinite/isinf/isnan)

#[test]
fn mathfunc_classification_predicates() {
    // The classification predicates (Tcl 9.0; 8.6 lacks them) inspect a double
    // and return 0/1 with no domain check. Pinned to tclsh 9.0. Note `Inf` is
    // supplied via `1.0/0.0` (a literal `Inf` operand hits the bare-literal bug).
    expr_eq("isfinite(1.0)", "1"); // tclsh 9.0
    expr_eq("isfinite(1.0/0.0)", "0"); // Inf is not finite
    expr_eq("isinf(1.0/0.0)", "1"); // Inf
    expr_eq("isinf(1.0)", "0");
    expr_eq("isnan(1.0)", "0");
    expr_eq("isnormal(2.5)", "1"); // a normal double
    expr_eq("isnormal(1.0/0.0)", "0"); // Inf is not "normal"
    expr_eq("issubnormal(1.0)", "0"); // 1.0 is normal, not subnormal
    expr_eq("isunordered(1.0, 2.0)", "0"); // neither is NaN
}

#[test]
fn mathfunc_hyperbolic() {
    // sinh/cosh/tanh.  tclsh: shared 8.6/9.0
    expr_eq("sinh(0)", "0.0");
    expr_eq("cosh(0)", "1.0");
    expr_eq("tanh(0)", "0.0");
    expr_eq("sinh(1)", "1.1752011936438014");
}

#[test]
fn mathfunc_nonnumeric_argument_float_error() {
    // A non-numeric `$var` argument to a float function gives the canonical
    // "floating-point number" coercion error (the `as_double` Err branch).
    // tclsh 9.0:
    for f in [
        "double", "sqrt", "floor", "ceil", "sin", "cos", "exp", "log",
    ] {
        let (ok, result, _) = run(&format!("set x abc; expr {{{f}($x)}}"));
        assert!(!ok, "{f}($x) should error");
        assert_eq!(
            result, "expected floating-point number but got \"abc\"",
            "{f}($x)"
        );
    }
    // Two-arg float functions, error on the bad operand.  tclsh 9.0:
    for f in ["pow", "atan2", "hypot", "fmod"] {
        let (ok, result, _) = run(&format!("set x abc; expr {{{f}($x, 1)}}"));
        assert!(!ok, "{f}($x,1) should error");
        assert_eq!(
            result, "expected floating-point number but got \"abc\"",
            "{f}($x,1)"
        );
    }
}

#[test]
fn mathfunc_bool_nonboolean_argument_error() {
    // bool() of a non-boolean `$var` gives the boolean coercion error (the
    // `as_bool` Err branch).  tclsh 9.0:
    let (ok, result, _) = run("set x abc; expr {bool($x)}");
    assert!(!ok);
    assert_eq!(result, "expected boolean value but got \"abc\"");
}

// BUG: the integer-ish functions (abs/int/round/entier/isqrt/max/min) funnel a
// non-numeric argument through `as_double()`, so they report "expected
// floating-point number" where tclsh 9.0 reports the integer-flavoured
// "expected number" (the function never accepts a fractional value here).
//   script:        set x abc; expr {abs($x)}   tclsh 9.0: expected number but got "abc"
//   tcl-vm:        expected floating-point number but got "abc"
#[test]
fn bug_integer_mathfunc_nonnumeric_error_wording() {
    for f in ["abs", "int", "round", "entier", "isqrt"] {
        let (ok, result, _) = run(&format!("set x abc; expr {{{f}($x)}}"));
        assert!(!ok, "{f}($x) should error");
        assert_eq!(result, "expected number but got \"abc\"", "{f}($x)");
    }
    // max/min report the same integer-flavoured message in tclsh 9.0.
    let (ok, result, _) = run("set x abc; expr {max($x, 1)}");
    assert!(!ok);
    assert_eq!(result, "expected number but got \"abc\"");
}

// BUG: the classification predicates reject the literal `NaN` value. tclsh 9.0
// accepts `NaN`/`Inf` as floating-point values for `isnan`/`isunordered`/… (the
// whole point of `isnan` is to detect one), but the VM's `pred_fn`/`pred_fn2`
// coerce via `as_double()`, which parses `Inf` yet errors on `NaN`.
//   script:        expr {isnan(NaN)}             tclsh 9.0: 1
//   tcl-vm:        expected floating-point number but got "NaN" (ok=false)
//   script:        expr {isunordered(NaN, 1.0)}  tclsh 9.0: 1
//   tcl-vm:        expected floating-point number but got "NaN" (ok=false)
#[test]
fn bug_classification_predicate_rejects_nan_literal() {
    expr_eq("isnan(NaN)", "1"); // tclsh 9.0
    expr_eq("isunordered(NaN, 1.0)", "1"); // tclsh 9.0
}

// tcl::mathfunc: max / min

#[test]
fn mathfunc_max_min_integer() {
    // All-integer args -> integer result.  tclsh: shared 8.6/9.0
    expr_eq("max(1,2,3)", "3");
    expr_eq("min(3,1,2)", "1");
    expr_eq("max(5)", "5");
    expr_eq("min(5)", "5");
    expr_eq("max(-1,-5,-2)", "-1");
    expr_eq("max(1,2,3,4,5,6)", "6");
}

#[test]
fn mathfunc_max_min_double() {
    // All-double (or chosen element is a double) -> double result.
    // tclsh: shared 8.6/9.0
    expr_eq("max(1.5,2.5)", "2.5");
    expr_eq("min(1.5,2.5)", "1.5");
    expr_eq("max(1,2,3.0)", "3.0"); // winner 3.0 is a double
    expr_eq("min(1.0,2,3)", "1.0"); // winner 1.0 is a double
}

// BUG: max()/min() over a mix of integers and doubles returns the *winning
// element with its own type* in tclsh. The VM's `min_max` coerces every
// argument to f64 and returns a double whenever any argument is non-integer,
// so it prints the integer winner as a double.
//   script:        expr {max(1.5, 2)}
//   tclsh 8.6/9.0: 2        (the integer 2 wins and stays an integer)
//   tcl-vm:        2.0
#[test]
fn bug_max_min_mixed_preserves_winner_type() {
    // tclsh: expr {max(1.5, 2)} -> 2
    expr_eq("max(1.5, 2)", "2");
    // tclsh: expr {min(1, 2.5)} -> 1
    expr_eq("min(1, 2.5)", "1");
    // tclsh: expr {max(2, 1.5)} -> 2
    expr_eq("max(2, 1.5)", "2");
}

// tcl::mathfunc: arg-count and bad-argument errors

// BUG: the math-function arity error wording diverges across every function.
// tclsh 9.0 says "not enough arguments for math function \"X\"" (too few) and
// "too many arguments for math function \"X\"" (too many); the VM emits the
// single phrasing "too many/few args to math function \"X\"" (and "too few args
// to math function \"X\"" for max/min).
//   script:        expr {abs(1,2)}    tclsh 9.0: too many arguments for math function "abs"
//   script:        expr {sqrt()}      tclsh 9.0: not enough arguments for math function "sqrt"
//   tcl-vm:        too many/few args to math function "…"
#[test]
fn bug_mathfunc_arity_error_wording() {
    // "too many" direction (one-arg funcs given two; two-arg funcs given three).
    expr_err("abs(1,2)", "too many arguments for math function \"abs\"");
    expr_err(
        "round(1,2)",
        "too many arguments for math function \"round\"",
    );
    expr_err("sin(1,2)", "too many arguments for math function \"sin\"");
    expr_err("sqrt(1,2)", "too many arguments for math function \"sqrt\"");
    expr_err("pow(1,2,3)", "too many arguments for math function \"pow\"");
    // "not enough" direction (one-arg funcs given zero; two-arg funcs given one).
    expr_err("sqrt()", "not enough arguments for math function \"sqrt\"");
    expr_err("abs()", "not enough arguments for math function \"abs\"");
    expr_err("int()", "not enough arguments for math function \"int\"");
    expr_err(
        "double()",
        "not enough arguments for math function \"double\"",
    );
    expr_err("bool()", "not enough arguments for math function \"bool\"");
    expr_err(
        "isqrt()",
        "not enough arguments for math function \"isqrt\"",
    );
    expr_err("pow(1)", "not enough arguments for math function \"pow\"");
    expr_err(
        "atan2(1)",
        "not enough arguments for math function \"atan2\"",
    );
    expr_err(
        "hypot(1)",
        "not enough arguments for math function \"hypot\"",
    );
    expr_err("fmod(1)", "not enough arguments for math function \"fmod\"");
    expr_err("max()", "not enough arguments for math function \"max\"");
    expr_err("min()", "not enough arguments for math function \"min\"");
    expr_err(
        "isunordered(1)",
        "not enough arguments for math function \"isunordered\"",
    );
}

// tcl::mathfunc: rand / srand (gap — not implemented in the VM)

// BUG: rand()/srand() are missing from the VM. tclsh implements both; the VM
// has no `tcl::mathfunc::rand` / `::srand` registration, so an `expr` using
// them fails with "invalid command name" instead of returning a number.
// srand(N) is deterministic in tclsh: srand(1) seeds and returns the first
// random number, 7.826369259425611e-6, on both 8.6 and 9.0.
//   script:        expr {srand(1)}
//   tclsh 8.6/9.0: 7.826369259425611e-6
//   tcl-vm:        invalid command name "tcl::mathfunc::srand" (ok=false)
#[test]
fn bug_srand_is_deterministic_and_missing() {
    // tclsh 8.6/9.0: expr {srand(1)} -> 7.826369259425611e-6
    expr_eq("srand(1)", "7.826369259425611e-6");
}

// BUG: rand() is likewise missing; in tclsh it returns a double in [0,1).
//   script:        srand(7); set x [expr {rand()}]; expr {$x >= 0 && $x < 1}
//   tclsh 8.6/9.0: 1
//   tcl-vm:        invalid command name "tcl::mathfunc::rand" (ok=false)
#[test]
fn bug_rand_returns_unit_interval_and_missing() {
    // Seed first (deterministic), then check rand() lands in [0,1).
    let (ok, result, _) = run("expr {srand(99)}; set x [expr {rand()}]; expr {$x >= 0 && $x < 1}");
    assert!(ok, "rand() should produce a number in [0,1): {result}");
    assert_eq!(result, "1"); // tclsh 8.6/9.0
}

// ::tcl::mathop::* operator commands

#[test]
fn mathop_arithmetic_fold() {
    // Variadic fold / identity / single-arg forms.  tclsh: shared 8.6/9.0
    cmd_eq("::tcl::mathop::+ 1 2 3", "6");
    cmd_eq("::tcl::mathop::+", "0"); // additive identity
    cmd_eq("::tcl::mathop::* 2 3 4", "24");
    cmd_eq("::tcl::mathop::*", "1"); // multiplicative identity
    cmd_eq("::tcl::mathop::- 10 3 2", "5"); // left fold
    cmd_eq("::tcl::mathop::- 5", "-5"); // 1 arg negates
    cmd_eq("::tcl::mathop::/ 100 5 2", "10"); // left fold
}

#[test]
fn mathop_division_forms() {
    // tclsh: shared 8.6/9.0
    cmd_eq("::tcl::mathop::/ 7 2", "3"); // integer floor
    cmd_eq("::tcl::mathop::/ 7.0 2", "3.5"); // double
    cmd_eq("::tcl::mathop::/ 4", "0.25"); // 1-arg reciprocal -> double
    cmd_eq("::tcl::mathop::/ 0", "Inf"); // reciprocal of 0 -> Inf
}

#[test]
fn mathop_power_right_associative() {
    // `**` folds right-associatively: 2 ** (3 ** 2) == 512.  tclsh: 8.6/9.0
    cmd_eq("::tcl::mathop::** 2 3 2", "512");
    cmd_eq("::tcl::mathop::** 2 10", "1024");
}

#[test]
fn mathop_modulo_and_shifts() {
    // tclsh: shared 8.6/9.0
    cmd_eq("::tcl::mathop::% 17 5", "2");
    cmd_eq("::tcl::mathop::<< 1 4", "16");
    cmd_eq("::tcl::mathop::>> 256 2", "64");
}

#[test]
fn mathop_bitwise() {
    // tclsh: shared 8.6/9.0
    cmd_eq("::tcl::mathop::& 12 10", "8");
    cmd_eq("::tcl::mathop::| 1 2 4", "7");
    cmd_eq("::tcl::mathop::^ 12 10", "6");
    cmd_eq("::tcl::mathop::~ 5", "-6"); // unary
    cmd_eq("::tcl::mathop::! 0", "1"); // unary logical not
    cmd_eq("::tcl::mathop::! 5", "0");
}

#[test]
fn mathop_comparison_chained() {
    // Comparison operators chain over all args.  tclsh: shared 8.6/9.0
    cmd_eq("::tcl::mathop::< 1 2 3", "1"); // strictly increasing
    cmd_eq("::tcl::mathop::< 1 3 2", "0");
    cmd_eq("::tcl::mathop::<= 1 1 2", "1");
    cmd_eq("::tcl::mathop::> 3 2 1", "1");
    cmd_eq("::tcl::mathop::>= 3 3 1", "1");
    cmd_eq("::tcl::mathop::== 5 5 5", "1");
    cmd_eq("::tcl::mathop::!= 1 2", "1");
    cmd_eq("::tcl::mathop::< 1", "1"); // single arg is trivially true
}

#[test]
fn mathop_string_comparisons() {
    // `eq`/`ne` always string; `lt`/`le`/`gt`/`ge` (Tcl 9) string compare.
    // tclsh 9.0 (8.6 lacks lt/le/gt/ge as mathop commands):
    cmd_eq("::tcl::mathop::eq abc abc", "1");
    cmd_eq("::tcl::mathop::ne a b", "1");
    cmd_eq("::tcl::mathop::lt a b", "1");
    cmd_eq("::tcl::mathop::le 1 1", "1");
    cmd_eq("::tcl::mathop::gt 2 1", "1"); // "2" > "1" lexically
    cmd_eq("::tcl::mathop::ge 2 2", "1");
}

#[test]
fn mathop_lt_le_gt_ge_are_lexicographic_not_numeric() {
    // Adversarial-review finding (issue #984): every existing lt/le/gt/ge
    // case above (`a`/`b`, `1`/`1`, `2`/`1`, `2`/`2`) happens to agree
    // whether compared as strings or as numbers, so none of them would
    // catch a regression that silently flipped these operators to numeric
    // comparison. `9` vs `10` is a genuine divergence: numerically 9 < 10,
    // but lexicographically "9" > "10" (`'9'` > `'1'` byte-wise) — the
    // opposite ordering. TIP 461 specifies lt/le/gt/ge as always string
    // comparisons, confirmed against tclsh9.0.
    cmd_eq("::tcl::mathop::lt 9 10", "0"); // "9" < "10" is false lexically
    cmd_eq("::tcl::mathop::le 9 10", "0"); // "9" <= "10" is false lexically
    cmd_eq("::tcl::mathop::gt 9 10", "1"); // "9" > "10" is true lexically
    cmd_eq("::tcl::mathop::ge 9 10", "1"); // "9" >= "10" is true lexically
}

#[test]
fn mathop_membership() {
    // tclsh: shared 8.6/9.0
    cmd_eq("::tcl::mathop::in b {a b c}", "1");
    cmd_eq("::tcl::mathop::in z {a b c}", "0");
    cmd_eq("::tcl::mathop::ni x {a b c}", "1");
    cmd_eq("::tcl::mathop::ni b {a b c}", "0");
}

#[test]
fn mathop_errors() {
    // Division by zero / modulo by zero.  tclsh: shared 8.6/9.0
    cmd_err("::tcl::mathop::/ 5 0", "divide by zero");
    cmd_err("::tcl::mathop::% 5 0", "divide by zero");
    // A float operand to `~`.  tclsh 9.0 wording:
    cmd_err(
        "::tcl::mathop::~ 1.5",
        "cannot use floating-point value \"1.5\" as operand of \"~\"",
    );
    // wrong # args — the usage names the operator as invoked.  tclsh: 8.6/9.0
    cmd_err(
        "::tcl::mathop::%",
        "wrong # args: should be \"::tcl::mathop::% integer integer\"",
    );
    cmd_err(
        "::tcl::mathop::in x",
        "wrong # args: should be \"::tcl::mathop::in value list\"",
    );
    cmd_err(
        "::tcl::mathop::-",
        "wrong # args: should be \"::tcl::mathop::- value ?value ...?\"",
    );
}

#[test]
fn mathop_invoked_name_via_namespace_path() {
    // When invoked unqualified through `namespace path`, the usage error names
    // the operator as the bare word, not the resolved `::tcl::mathop::!`.
    // tclsh 8.6/9.0 (mathop-3.9/4.9 family):
    let (ok, result, _) = run(concat!(
        "namespace eval foo {\n",
        "  namespace path ::tcl::mathop\n",
        "  catch {! a b} m\n",
        "  set m\n",
        "}",
    ));
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "wrong # args: should be \"! boolean\"");
}

#[test]
fn mathop_namespace_import() {
    // `::tcl::mathop` exports every operator, so `namespace import` works.
    // tclsh 8.6/9.0:
    let (ok, result, _) = run(concat!(
        "namespace eval foo {\n",
        "  namespace import ::tcl::mathop::+\n",
        "  + 10 20 30\n",
        "}",
    ));
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "60");
}

// -- radix-invalid numerals are barewords, not their own text (found by the
//    differential fuzzer's malformed-expression campaigns) --

/// A numeral whose digits are invalid for its radix is not a number: C's
/// `ParseLexeme` fails to consume it with `TclParseNumber` and classifies the
/// text as a bareword (`tclCompExpr.c:716-780`). It used to evaluate to its own
/// source text — `expr {0o8}` returned the string `0o8`.
#[test]
fn radix_invalid_numerals_are_barewords() {
    for (bad, radix) in [("0o8", "octal"), ("0o9", "octal"), ("0b2", "binary")] {
        let (ok, result, _) = run(&format!("expr {{{bad}}}"));
        assert!(!ok, "`expr {{{bad}}}` must error, got: {result}");
        assert!(
            result.starts_with(&format!("invalid bareword \"{bad}\"")),
            "{bad}: {result}"
        );
        // C guesses the radix in the postscript (`tclCompExpr.c:788-807`).
        assert!(
            result.ends_with(&format!("(invalid {radix} number?)")),
            "{bad} should carry the {radix} hint: {result}"
        );
    }
}

/// A bare prefix or a bad *hex* digit is still a bareword, but C has no `case
/// 'x'` in its hint switch, so these get no parenthesised guess.
#[test]
fn bad_hex_numerals_are_barewords_without_a_hint() {
    for bad in ["0x", "0xg"] {
        let (ok, result, _) = run(&format!("expr {{{bad}}}"));
        assert!(!ok, "`expr {{{bad}}}` must error, got: {result}");
        assert!(
            result.starts_with(&format!("invalid bareword \"{bad}\"")),
            "{bad}: {result}"
        );
        assert!(
            !result.contains("number?)"),
            "{bad} must get no hint: {result}"
        );
    }
}

/// The valid numerals in every radix keep working — including `0d`, which the
/// expression lexer did not recognise at all before, and a leading-zero integer,
/// which is decimal under the 9.0 default.
#[test]
fn valid_numerals_in_every_radix_still_evaluate() {
    for (src, want) in [
        ("0x1f", "31"),
        ("0o17", "15"),
        ("0b101", "5"),
        ("0d99", "99"),
        ("0", "0"),
        ("007", "7"),
        ("08", "8"),
        ("0755", "755"),
        ("1.5e3", "1500.0"),
        ("0x7fffffffffffffff + 1", "9223372036854775808"),
    ] {
        let (ok, result, _) = run(&format!("expr {{{src}}}"));
        assert!(ok, "`expr {{{src}}}` should evaluate, got error: {result}");
        assert_eq!(result, want, "expr {{{src}}}");
    }
}
