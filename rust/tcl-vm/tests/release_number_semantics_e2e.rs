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

//! `expr` and number semantics per release, as C has them: `tcl_precision`
//! and double formatting, 8.4's 64-bit integers, range errors and parser,
//! word substitution in `expr` words and conditions, `string`'s dispatch, and
//! the errorCodes each release sets.
//!
//! Every expectation is the result of the same script under real tclsh
//! 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0, grouped where releases agree.

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;
use tcl_vm::Vm;

/// The result of `src` under the `version` pin; panics if it errors.
fn result_for(version: TclVersion, src: &str) -> String {
    let profile = tcl_registry::model::ingress::resolve_environment(version.dialect_name())
        .analyser_profile();
    let capture = Capture::default();
    let mut vm = Vm::with_native_core(
        Box::new(capture.clone()),
        std::rc::Rc::new(tcl_vm::host_native::NativeHost::new()),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: Vec::new(),
            default_library: None,
        },
    )
    .expect("native core for the selected profile");
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    let completion = vm.try_eval_source(src).expect("original source provider");
    assert!(
        completion.code.is_ok(),
        "{version:?}: script errored: {}",
        completion.result.to_str()
    );
    // The result is read as a Tcl string by the interpreter that made it: a
    // numeric object's string needs that interpreter's numeric operation.
    vm.set_var("s41_result", completion.result)
        .expect("result variable");
    let printed = vm
        .try_eval_source("puts -nonewline $s41_result")
        .expect("original source provider");
    assert!(printed.code.is_ok(), "{version:?}: result print");
    String::from_utf8_lossy(&capture.0.borrow()).into_owned()
}

#[derive(Clone, Default)]
struct Capture(std::rc::Rc<std::cell::RefCell<Vec<u8>>>);

impl std::io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn assert_releases(releases: &[TclVersion], src: &str, expected: &str) {
    for &version in releases {
        assert_eq!(result_for(version, src), expected, "{version:?}");
    }
}

/// `tcl_precision` is linked to `Tcl_PrintDouble` through 8.6: 8.4 prints
/// `%.<p>g` from 12, 8.5 and 8.6 the shortest digits at 0 and `p`
/// significant digits otherwise. 9.0 has no such variable.
#[test]
fn tcl_precision_sets_how_doubles_print() {
    let src = r"set out {}
lappend out [expr {1/3.0}] [expr {1e-7}] [expr {123456789012345.0}] [info exists tcl_precision]
foreach p {6 17 0 18 abc} { lappend out [catch {set tcl_precision $p} m] $m [expr {2/3.0}] [expr {1e-7}] }
set out";
    assert_releases(
        &[TclVersion::V8_4],
        src,
        r#"0.33333333333333331 9.9999999999999995e-08 123456789012345.0 1 0 6 0.66666666666666663 9.9999999999999995e-08 0 17 0.66666666666666663 9.9999999999999995e-08 1 {can't set "tcl_precision": improper value for precision} 0.66666666666666663 9.9999999999999995e-08 1 {can't set "tcl_precision": improper value for precision} 0.66666666666666663 9.9999999999999995e-08 1 {can't set "tcl_precision": improper value for precision} 0.66666666666666663 9.9999999999999995e-08"#,
    );
    assert_releases(
        &[TclVersion::V8_5, TclVersion::V8_6],
        src,
        r#"0.3333333333333333 1e-7 123456789012345.0 1 0 6 0.6666666666666666 1e-7 0 17 0.6666666666666666 1e-7 0 0 0.6666666666666666 1e-7 1 {can't set "tcl_precision": improper value for precision} 0.6666666666666666 1e-7 1 {can't set "tcl_precision": improper value for precision} 0.6666666666666666 1e-7"#,
    );
    assert_releases(
        &[TclVersion::V9_0, TclVersion::V9_1],
        src,
        r"0.3333333333333333 1e-7 123456789012345.0 0 0 6 0.6666666666666666 1e-7 0 17 0.6666666666666666 1e-7 0 0 0.6666666666666666 1e-7 0 18 0.6666666666666666 1e-7 0 abc 0.6666666666666666 1e-7",
    );
}

/// The precision is thread data in C, shared by the interpreter tree, and its
/// trace re-installs itself when the variable is unset.
#[test]
fn tcl_precision_is_shared_and_survives_unset() {
    let src = r"set tcl_precision 5
interp create c
c eval {set tcl_precision 3}
set a [expr {2/3.0}]
unset tcl_precision
list $a [info exists tcl_precision] $tcl_precision [expr {2/3.0}] [trace info variable tcl_precision]";
    assert_releases(
        &[TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6],
        src,
        r"0.667 1 3 0.667 {}",
    );
}

/// 8.4's failed variable trace starts errorInfo again from the access error;
/// later releases keep the trace's frame.
#[test]
fn refused_tcl_precision_reports_like_its_release() {
    let src = r"proc f {} { global tcl_precision; set tcl_precision 99 }
list [catch f m] $m $::errorInfo";
    assert_releases(
        &[TclVersion::V8_4],
        src,
        r#"1 {can't set "tcl_precision": improper value for precision} {can't set "tcl_precision": improper value for precision
    while executing
"set tcl_precision 99 "
    (procedure "f" line 1)
    invoked from within
"f"}"#,
    );
    assert_releases(
        &[TclVersion::V8_5, TclVersion::V8_6],
        src,
        r#"1 {can't set "tcl_precision": improper value for precision} {improper value for precision
    (write trace on "tcl_precision")
    invoked from within
"set tcl_precision 99 "
    (procedure "f" line 1)
    invoked from within
"f"}"#,
    );
}

/// An `expr` nested in a command substitution canonicalises its result.
#[test]
fn nested_expr_literal_is_canonical() {
    let src = r"list [list [expr {1e3}]] [concat [expr {0x10}]] [string length [expr {1e3}]] [list [expr 1e3]]";
    assert_releases(
        &[
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ],
        src,
        r"1000.0 16 6 1000.0",
    );
}

/// A word that is not simple is substituted and its value evaluated again,
/// so `expr $x` runs a command in `x`'s value and `\"` reaches `expr` as `"`.
#[test]
fn unbraced_expr_word_is_substituted_then_evaluated() {
    let src = r#"set x {[set z 7]}
set a {1+1}
set r [list [expr $x] [info exists z] [expr $a*2] [expr "$a*2"]]
proc p {} { set a {1+1}; list [expr $a*2] [expr \"a\"=={a}] }
lappend r [expr \"a\"=={a}] [p] [expr 1+2]"#;
    assert_releases(
        &[
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ],
        src,
        r"7 1 3 3 1 {3 1} 3",
    );
}

/// An `if`, `while` or `for` condition word with substitutions is substituted
/// and its value evaluated as an expression, at top level and in a body.
#[test]
fn substituted_conditions_evaluate_their_value() {
    let src = r#"set c {1==1}
set d {2<1}
set r {}
if $c {lappend r if}
while $d {lappend r never}
for {set i 0} "\$i<2" {incr i} {}
lappend r $i
if \"a\"=={a} {lappend r bs}
proc p {} { set c {1==1}; set r {}; if $c {lappend r p}; if "$c" {lappend r q}; return $r }
lappend r [p]"#;
    assert_releases(
        &[
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ],
        src,
        r"if 2 bs {p q}",
    );
}

/// 8.4's integers are 64 bits: arithmetic and `incr` wrap, a literal up to
/// 2^64-1 wraps and a larger one is too large; a computed `INT64_MIN` prints
/// `-8` on 8.4 and 8.5. From 8.5 integers are unbounded.
#[test]
fn integers_wrap_at_64_bits_on_8_4() {
    let src = r#"proc t {e} { if {[catch {uplevel 1 [list expr $e]} r]} { return "ERR $r | $::errorCode" } else { return $r } }
set big 18446744073709551616; set wrap 18446744073709551615
set i 9223372036854775807
list [t {9223372036854775807 + 1}] [t {9223372036854775807 * 2}] [t {-(-9223372036854775808)}] [t {$wrap + 0}] [t {$big + 0}] [t {18446744073709551616}] [incr i] [t {1 << 64}] [t {1 << -1}] [t {-1 >> 100}]"#;
    assert_releases(
        &[TclVersion::V8_4],
        src,
        r#"-8 -2 -8 -1 {ERR can't use integer value too large to represent as operand of "+" | ARITH IOVERFLOW {integer value too large to represent}} {ERR integer value too large to represent | ARITH IOVERFLOW {integer value too large to represent}} -8 0 -8 -1"#,
    );
    assert_releases(
        &[
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ],
        src,
        r"9223372036854775808 18446744073709551614 9223372036854775808 18446744073709551615 18446744073709551616 18446744073709551616 9223372036854775808 18446744073709551616 {ERR negative shift argument | NONE} -1",
    );
}

/// 8.4 raises on a double result that overflows or is NaN and on a division
/// by a zero double, and reads out-of-range literals as errors; later
/// releases return `Inf`, and NaN is a domain error everywhere.
#[test]
fn doubles_overflow_and_divide_by_zero_per_release() {
    let src = r#"proc t {e} { if {[catch {uplevel 1 [list expr $e]} r]} { return "ERR $r | $::errorCode" } else { return $r } }
list [t {1e308 * 10}] [t {1.0 / 0}] [t {0.0 / 0}] [t {1e999}] [t {"1e999"}] [t {1e-999}] [t {1e-300 * 1e-300}] [t {exp(1000)}] [t {exp(-1000)}] [t {log(0)}]"#;
    assert_releases(
        &[TclVersion::V8_4],
        src,
        r"{ERR floating-point value too large to represent | ARITH OVERFLOW {floating-point value too large to represent}} {ERR divide by zero | ARITH DIVZERO {divide by zero}} {ERR divide by zero | ARITH DIVZERO {divide by zero}} {ERR floating-point value too large to represent | ARITH OVERFLOW {floating-point value too large to represent}} 1e999 {ERR floating-point value too small to represent | ARITH UNDERFLOW {floating-point value too small to represent}} 0.0 {ERR floating-point value too large to represent | ARITH OVERFLOW {floating-point value too large to represent}} {ERR floating-point value too small to represent | ARITH UNDERFLOW {floating-point value too small to represent}} {ERR floating-point value too large to represent | ARITH OVERFLOW {floating-point value too large to represent}}",
    );
    assert_releases(
        &[
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ],
        src,
        r"Inf Inf {ERR domain error: argument not in valid range | ARITH DOMAIN {domain error: argument not in valid range}} Inf Inf 0.0 0.0 Inf 0.0 -Inf",
    );
}

/// Math-function arguments, arity and integer conversions as each release's
/// table checks them.
#[test]
fn math_functions_follow_the_release() {
    let src = r#"proc t {e} { if {[catch {uplevel 1 [list expr $e]} r]} { return "ERR $r | $::errorCode" } else { return $r } }
list [t {abs(-9223372036854775808)}] [t {abs("x")}] [t {int(1e20)}] [t {round(-2.5)}] [t {round(9.3e18)}] [t {double("x")}] [t {sin()}] [t {sqrt("x")}] [t {int(-9223372036854775808.0)}] [t {srand(1.5)}]"#;
    assert_releases(
        &[TclVersion::V8_4],
        src,
        r#"{ERR integer value too large to represent | ARITH IOVERFLOW {integer value too large to represent}} {ERR argument to math function didn't have numeric value | NONE} {ERR integer value too large to represent | ARITH IOVERFLOW {integer value too large to represent}} -3 {ERR integer value too large to represent | ARITH IOVERFLOW {integer value too large to represent}} {ERR argument to math function didn't have numeric value | NONE} {ERR too few arguments for math function | NONE} {ERR argument to math function didn't have numeric value | NONE} -8 {ERR expected integer but got "1.5" | NONE}"#,
    );
    assert_releases(
        &[TclVersion::V8_5],
        src,
        r#"9223372036854775808 {ERR expected number but got "x" | TCL VALUE NUMBER} 7766279631452241920 -3 9300000000000000000 {ERR expected floating-point number but got "x" | TCL VALUE NUMBER} {ERR too few arguments for math function "sin" | NONE} {ERR expected floating-point number but got "x" | TCL VALUE NUMBER} -8 {ERR expected integer but got "1.5" | NONE}"#,
    );
    assert_releases(
        &[TclVersion::V8_6],
        src,
        r#"9223372036854775808 {ERR expected number but got "x" | TCL VALUE NUMBER} 7766279631452241920 -3 9300000000000000000 {ERR expected floating-point number but got "x" | TCL VALUE NUMBER} {ERR not enough arguments for math function "sin" | TCL WRONGARGS} {ERR expected floating-point number but got "x" | TCL VALUE NUMBER} -9223372036854775808 {ERR expected integer but got "1.5" | TCL VALUE INTEGER}"#,
    );
    assert_releases(
        &[TclVersion::V9_0, TclVersion::V9_1],
        src,
        r#"9223372036854775808 {ERR expected number but got "x" | TCL VALUE NUMBER} 100000000000000000000 -3 9300000000000000000 {ERR expected floating-point number but got "x" | TCL VALUE NUMBER} {ERR not enough arguments for math function "sin" | TCL WRONGARGS} {ERR expected floating-point number but got "x" | TCL VALUE NUMBER} -9223372036854775808 {ERR  | NONE}"#,
    );
}

/// 8.4 reports `syntax error in expression "…": …` from `tclParseExpr.c`;
/// 8.5 has today's messages without the `TCL PARSE EXPR` errorCode 8.6 adds.
#[test]
fn expr_syntax_errors_per_release() {
    let src = r#"proc t {e} { if {[catch {uplevel 1 [list expr $e]} r]} { return "ERR $r | $::errorCode" } else { return $r } }
list [t {1 +}] [t "(1"] [t {1 2}] [t {foo}] [t {2 ** 3}] [t {"a" in {a b}}] [t {0o17}] [t {08}] [t {sin 1}] [t {,}]"#;
    assert_releases(
        &[TclVersion::V8_4],
        src,
        r#"{ERR syntax error in expression "1 +": premature end of expression | NONE} {ERR syntax error in expression "(1": looking for close parenthesis | NONE} {ERR syntax error in expression "1 2": extra tokens at end of expression | NONE} {ERR syntax error in expression "foo": variable references require preceding $ | NONE} {ERR syntax error in expression "2 ** 3": unexpected operator * | NONE} {ERR syntax error in expression ""a" in {a b}": extra tokens at end of expression | NONE} {ERR syntax error in expression "0o17": extra tokens at end of expression | NONE} {ERR expected integer but got "08" (looks like invalid octal number) | NONE} {ERR syntax error in expression "sin 1": expected parenthesis enclosing function arguments | NONE} {ERR syntax error in expression ",": commas can only separate function arguments | NONE}"#,
    );
    assert_releases(
        &[TclVersion::V8_5],
        src,
        r#"{ERR missing operand at _@_
in expression "1 +_@_" | NONE} {ERR unbalanced open paren
in expression "(1" | NONE} {ERR missing operator at _@_
in expression "1 _@_2" | NONE} {ERR invalid bareword "foo"
in expression "foo";
should be "$foo" or "{foo}" or "foo(...)" or ... | NONE} 8 1 15 {ERR invalid bareword "08"
in expression "08";
should be "$08" or "{08}" or "08(...)" or ... (invalid octal number?) | NONE} {ERR invalid bareword "sin"
in expression "sin 1";
should be "$sin" or "{sin}" or "sin(...)" or ... | NONE} {ERR missing operand at _@_
in expression "_@_," | NONE}"#,
    );
    assert_releases(
        &[TclVersion::V8_6],
        src,
        r#"{ERR missing operand at _@_
in expression "1 +_@_" | TCL PARSE EXPR MISSING} {ERR unbalanced open paren
in expression "(1" | TCL PARSE EXPR UNBALANCED} {ERR missing operator at _@_
in expression "1 _@_2" | TCL PARSE EXPR MISSING} {ERR invalid bareword "foo"
in expression "foo";
should be "$foo" or "{foo}" or "foo(...)" or ... | TCL PARSE EXPR BAREWORD} 8 1 15 {ERR invalid bareword "08"
in expression "08";
should be "$08" or "{08}" or "08(...)" or ... (invalid octal number?) | TCL PARSE EXPR BADNUMBER OCTAL} {ERR invalid bareword "sin"
in expression "sin 1";
should be "$sin" or "{sin}" or "sin(...)" or ... | TCL PARSE EXPR BAREWORD} {ERR missing operand at _@_
in expression "_@_," | TCL PARSE EXPR MISSING}"#,
    );
    assert_releases(
        &[TclVersion::V9_0, TclVersion::V9_1],
        src,
        r#"{ERR missing operand at _@_
in expression "1 +_@_" | TCL PARSE EXPR MISSING} {ERR unbalanced open paren
in expression "(1" | TCL PARSE EXPR UNBALANCED} {ERR missing operator at _@_
in expression "1 _@_2" | TCL PARSE EXPR MISSING} {ERR invalid bareword "foo"
in expression "foo";
should be "$foo" or "{foo}" or "foo(...)" or ... | TCL PARSE EXPR BAREWORD} 8 1 15 8 {ERR invalid bareword "sin"
in expression "sin 1";
should be "$sin" or "{sin}" or "sin(...)" or ... | TCL PARSE EXPR BAREWORD} {ERR missing operand at _@_
in expression "_@_," | TCL PARSE EXPR MISSING}"#,
    );
}

/// Operand errors are worded per release and carry `ARITH DOMAIN` from 8.5;
/// an unknown math function is a command lookup with its errorCode from 8.6.
#[test]
fn operand_errors_per_release() {
    let src = r#"proc t {e} { if {[catch {uplevel 1 [list expr $e]} r]} { return "ERR $r | $::errorCode" } else { return $r } }
list [t {"x" + 1}] [t {1.5 % 1}] [t {~1.5}] [t {!"x"}] [t {foo(1)}]"#;
    assert_releases(
        &[TclVersion::V8_4],
        src,
        r#"{ERR can't use non-numeric string as operand of "+" | NONE} {ERR can't use floating-point value as operand of "%" | NONE} {ERR can't use floating-point value as operand of "~" | NONE} {ERR can't use non-numeric string as operand of "!" | NONE} {ERR unknown math function "foo" | NONE}"#,
    );
    assert_releases(
        &[TclVersion::V8_5],
        src,
        r#"{ERR can't use non-numeric string as operand of "+" | ARITH DOMAIN {non-numeric string}} {ERR can't use floating-point value as operand of "%" | ARITH DOMAIN {floating-point value}} {ERR can't use floating-point value as operand of "~" | ARITH DOMAIN {floating-point value}} {ERR can't use non-numeric string as operand of "!" | ARITH DOMAIN {non-numeric string}} {ERR invalid command name "tcl::mathfunc::foo" | NONE}"#,
    );
    assert_releases(
        &[TclVersion::V8_6],
        src,
        r#"{ERR can't use non-numeric string as operand of "+" | ARITH DOMAIN {non-numeric string}} {ERR can't use floating-point value as operand of "%" | ARITH DOMAIN {floating-point value}} {ERR can't use floating-point value as operand of "~" | ARITH DOMAIN {floating-point value}} {ERR can't use non-numeric string as operand of "!" | ARITH DOMAIN {non-numeric string}} {ERR invalid command name "tcl::mathfunc::foo" | TCL LOOKUP COMMAND tcl::mathfunc::foo}"#,
    );
    assert_releases(
        &[TclVersion::V9_0, TclVersion::V9_1],
        src,
        r#"{ERR cannot use non-numeric string "x" as left operand of "+" | ARITH DOMAIN {non-numeric string}} {ERR cannot use floating-point value "1.5" as left operand of "%" | ARITH DOMAIN {floating-point value}} {ERR cannot use floating-point value "1.5" as operand of "~" | ARITH DOMAIN {floating-point value}} {ERR cannot use non-numeric string "x" as operand of "!" | ARITH DOMAIN {non-numeric string}} {ERR invalid command name "tcl::mathfunc::foo" | TCL LOOKUP COMMAND tcl::mathfunc::foo}"#,
    );
}

/// 8.4's `string` reads its option with `Tcl_GetIndexFromObj` (`bad option`,
/// table order); from 8.5 it is an ensemble with `TCL LOOKUP SUBCOMMAND`.
/// `bytelength` exists through 8.6.
#[test]
fn string_follows_the_release() {
    let src = r"set r {}
foreach c {{string bytelength abc} {string bogus x} {string} {string t x} {string totitle abc}} {
  if {[catch $c m]} { lappend r $m $::errorCode } else { lappend r $m }
}
set r";
    assert_releases(
        &[TclVersion::V8_4],
        src,
        r#"3 {bad option "bogus": must be bytelength, compare, equal, first, index, is, last, length, map, match, range, repeat, replace, tolower, toupper, totitle, trim, trimleft, trimright, wordend, or wordstart} NONE {wrong # args: should be "string option arg ?arg ...?"} NONE {ambiguous option "t": must be bytelength, compare, equal, first, index, is, last, length, map, match, range, repeat, replace, tolower, toupper, totitle, trim, trimleft, trimright, wordend, or wordstart} NONE Abc"#,
    );
    assert_releases(
        &[TclVersion::V8_5],
        src,
        r#"3 {unknown or ambiguous subcommand "bogus": must be bytelength, compare, equal, first, index, is, last, length, map, match, range, repeat, replace, reverse, tolower, totitle, toupper, trim, trimleft, trimright, wordend, or wordstart} {TCL LOOKUP SUBCOMMAND bogus} {wrong # args: should be "string subcommand ?argument ...?"} NONE {unknown or ambiguous subcommand "t": must be bytelength, compare, equal, first, index, is, last, length, map, match, range, repeat, replace, reverse, tolower, totitle, toupper, trim, trimleft, trimright, wordend, or wordstart} {TCL LOOKUP SUBCOMMAND t} Abc"#,
    );
    assert_releases(
        &[TclVersion::V8_6],
        src,
        r#"3 {unknown or ambiguous subcommand "bogus": must be bytelength, cat, compare, equal, first, index, is, last, length, map, match, range, repeat, replace, reverse, tolower, totitle, toupper, trim, trimleft, trimright, wordend, or wordstart} {TCL LOOKUP SUBCOMMAND bogus} {wrong # args: should be "string subcommand ?arg ...?"} {TCL WRONGARGS} {unknown or ambiguous subcommand "t": must be bytelength, cat, compare, equal, first, index, is, last, length, map, match, range, repeat, replace, reverse, tolower, totitle, toupper, trim, trimleft, trimright, wordend, or wordstart} {TCL LOOKUP SUBCOMMAND t} Abc"#,
    );
    assert_releases(
        &[TclVersion::V9_0, TclVersion::V9_1],
        src,
        r#"{unknown or ambiguous subcommand "bytelength": must be cat, compare, equal, first, index, insert, is, last, length, map, match, range, repeat, replace, reverse, tolower, totitle, toupper, trim, trimleft, trimright, wordend, or wordstart} {TCL LOOKUP SUBCOMMAND bytelength} {unknown or ambiguous subcommand "bogus": must be cat, compare, equal, first, index, insert, is, last, length, map, match, range, repeat, replace, reverse, tolower, totitle, toupper, trim, trimleft, trimright, wordend, or wordstart} {TCL LOOKUP SUBCOMMAND bogus} {wrong # args: should be "string subcommand ?arg ...?"} {TCL WRONGARGS} {unknown or ambiguous subcommand "t": must be cat, compare, equal, first, index, insert, is, last, length, map, match, range, repeat, replace, reverse, tolower, totitle, toupper, trim, trimleft, trimright, wordend, or wordstart} {TCL LOOKUP SUBCOMMAND t} Abc"#,
    );
}

/// `string bytelength` counts Tcl's internal modified UTF-8, which spells a
/// NUL in two bytes and keeps every character within three.
#[test]
fn string_bytelength_counts_internal_utf8() {
    assert_releases(
        &[TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6],
        r#"list [string bytelength "a\0b"] [string bytelength [format %c 0xFFFF]] [string bytelength [format %c 0x1F600]]"#,
        "4 3 3",
    );
}

/// `wrong # args` carries `TCL WRONGARGS` from 8.6.
#[test]
fn wrong_args_error_code_starts_in_8_6() {
    let src = r"proc p {a} {}
list [catch set m] $m $::errorCode [catch p m] $m $::errorCode";
    assert_releases(
        &[TclVersion::V8_4, TclVersion::V8_5],
        src,
        r#"1 {wrong # args: should be "set varName ?newValue?"} NONE 1 {wrong # args: should be "p a"} NONE"#,
    );
    assert_releases(
        &[TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1],
        src,
        r#"1 {wrong # args: should be "set varName ?newValue?"} {TCL WRONGARGS} 1 {wrong # args: should be "p a"} {TCL WRONGARGS}"#,
    );
}
