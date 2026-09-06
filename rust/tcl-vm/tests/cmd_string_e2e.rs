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

//! End-to-end coverage for the `string` / `format` command surface
//! (`tcl-vm/src/cmd_string.rs`, `cmd_string_is.rs`, `cmd_format.rs`), compiled as
//! real Tcl via `tcl-compiler` and run through `tcl-vm`. Every expectation is
//! pinned against real tclsh — the oracle is `tclsh8.6` + `tclsh9.0`. Where the
//! two reference interpreters disagree (e.g. the Tcl 9 `%#o`/`%#d` prefixes,
//! `string is dict`, arbitrary-precision `string is integer`) the VM targets
//! Tcl 9.0, so those cases cite `tclsh9.0` specifically.
//!
//! The `bug_*` tests document former VM-vs-tclsh divergences on valid input:
//! each asserts the *correct* tclsh behaviour and now passes, guarding the
//! fix against regression.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_vm::{CompileService, Vm};

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
fn run_for_version(src: &str, version: tcl_dialect::TclVersion) -> (bool, String, String) {
    let profile = tcl_registry::model::ingress::resolve_environment(version.dialect_name())
        .analyser_profile();
    let service = BytecodeCompileService::for_profile(profile);
    let asm = service
        .compile_for_profile(src, profile)
        .expect("test script compiles for its selected profile");

    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_runtime_version(version);
    vm.set_compiler(Box::new(service));
    let completion = vm.run_module(&asm);

    let out = String::from_utf8(buf.borrow().clone()).expect("utf-8 output");
    (
        completion.code.is_ok(),
        completion.result.to_str().to_string(),
        out,
    )
}

fn run(src: &str) -> (bool, String, String) {
    run_for_version(src, tcl_dialect::TclVersion::V9_0)
}

/// Assert a script runs OK and its result value equals `want`.
fn res_eq(src: &str, want: &str) {
    let (ok, result, _) = run(src);
    assert!(ok, "script errored: {result}\n  src: {src}");
    assert_eq!(result, want, "for script: {src}");
}

/// Assert a script errors with exactly `want`.
fn err_eq(src: &str, want: &str) {
    let (ok, result, _) = run(src);
    assert!(
        !ok,
        "expected an error, got ok with: {result}\n  src: {src}"
    );
    assert_eq!(result, want, "for script: {src}");
}

// string length / index / range / repeat / reverse / cat

/// `string length`, `index`, `range`, `repeat`, `reverse`, `cat` — the
/// core-backed subcommands, exercised through the VM's `string` dispatcher.
#[test]
fn string_basics_index_range_repeat() {
    // tclsh: string length hello -> 5 ; héllo (1 char accent) -> 5
    res_eq("string length hello", "5");
    res_eq("string length héllo", "5");
    res_eq("string length {}", "0");
    // tclsh: string index
    res_eq("string index hello 1", "e");
    res_eq("string index hello end", "o");
    res_eq("string index hello end-1", "l");
    res_eq("string index hello 1+1", "l");
    res_eq("string index hello 99", ""); // out of range -> empty
    res_eq("string index hello -1", ""); // negative -> empty
    // tclsh: string range
    res_eq("string range hello 1 3", "ell");
    res_eq("string range hello 1 end", "ello");
    res_eq("string range hello 3 99", "lo"); // last clamps
    res_eq("string range hello 1 end-1", "ell");
    // tclsh: string repeat
    res_eq("string repeat ab 3", "ababab");
    res_eq("string repeat ab 0", ""); // count 0 -> empty
    res_eq("string repeat ab -2", ""); // negative -> empty
    // tclsh: string reverse
    res_eq("string reverse hello", "olleh");
    // tclsh: string cat foo bar baz -> foobarbaz ; cat (no args) -> {}
    res_eq("string cat foo bar baz", "foobarbaz");
    res_eq("string cat", "");
    res_eq("string cat a", "a");
}

#[test]
fn compiled_string_length_uses_the_selected_runtime_character_model() {
    let script = "string length A😀B";
    assert_eq!(
        run_for_version(script, tcl_dialect::TclVersion::V8_6).1,
        "4"
    );
    assert_eq!(
        run_for_version(script, tcl_dialect::TclVersion::V9_0).1,
        "3"
    );
}

/// `string repeat` with a non-integer count -> canonical coercion error.
#[test]
fn string_repeat_bad_count() {
    // tclsh: string repeat ab x -> expected integer but got "x"
    err_eq("string repeat ab x", "expected integer but got \"x\"");
}

// append (registered alongside `string` in cmd_string.rs)

/// `append varName ?value ...?` — concatenation, the no-values read form
/// (returns the current value, or errors when unset), array elements, and the
/// argument/target errors.
#[test]
fn append_command() {
    // tclsh: concatenation of several values
    res_eq("set s foo; append s bar baz", "foobarbaz");
    // tclsh: appending to an unset var creates it from empty
    res_eq("append fresh abc", "abc");
    // tclsh: no values is a read of the current value
    res_eq("set x ab; append x", "ab");
    // tclsh: an array element target
    res_eq("set arr(k) hi; append arr(k) bye; set arr(k)", "hibye");
    // tclsh: no-values read of an unset variable errors
    err_eq("append nope", "can't read \"nope\": no such variable");
    // tclsh: arity error
    err_eq(
        "append",
        "wrong # args: should be \"append varName ?value ...?\"",
    );
    // tclsh: appending to an array base (not an element) is a variable error
    err_eq(
        "array set ar {a 1}; append ar x",
        "can't set \"ar\": variable is array",
    );
}

// string compare / equal — options (-nocase, -length, abbreviations) + errors

/// `string compare` / `string equal` with `-nocase` / `-length` and their
/// abbreviations.
#[test]
fn string_compare_equal_options() {
    // tclsh: compare returns -1 / 0 / 1
    res_eq("string compare abc abd", "-1");
    res_eq("string compare abc abc", "0");
    res_eq("string compare abd abc", "1");
    // tclsh: -nocase
    res_eq("string compare -nocase ABC abc", "0");
    res_eq("string equal -nocase ABC abc", "1");
    // tclsh: -length limits the compared prefix
    res_eq("string compare -length 2 abcX abcY", "0");
    res_eq("string equal -length 2 abX abY", "1");
    // tclsh: -nocase -length together
    res_eq("string compare -nocase -length 2 ABxx abYY", "0");
    // Tcl folds before applying -length: U+0130 lowercases to `i` plus a
    // combining dot, so the one-character prefix compares equal to `i`.
    res_eq("string equal -nocase -length 1 \u{0130} i", "1");
    // tclsh: equal basics
    res_eq("string equal abc abc", "1");
    res_eq("string equal abc abd", "0");
    // tclsh: abbreviated options (-noc, -len)
    res_eq("string compare -noc ABC abc", "0");
    res_eq("string equal -len 2 abX abY", "1");
    // tclsh: -length with a negative int disables the limit (compares fully)
    res_eq("string equal -length -1 abc abc", "1");
}

/// `string compare` / `string equal` argument and option errors.
#[test]
fn string_compare_equal_errors() {
    // tclsh: bad option
    err_eq(
        "string compare -bogus a b",
        "bad option \"-bogus\": must be -nocase or -length",
    );
    err_eq(
        "string equal -bogus a b",
        "bad option \"-bogus\": must be -nocase or -length",
    );
    // tclsh: -length with no following integer -> the usage error
    err_eq(
        "string compare -length",
        "wrong # args: should be \"string compare ?-nocase? ?-length int? string1 string2\"",
    );
    // tclsh: -length where its value slot would overlap the two operands (so no
    // integer is actually available) -> the usage error.
    err_eq(
        "string compare -length a b",
        "wrong # args: should be \"string compare ?-nocase? ?-length int? string1 string2\"",
    );
    // tclsh: -length with a non-integer argument
    err_eq(
        "string compare -length x a b",
        "expected integer but got \"x\"",
    );
    // tclsh: too few / too many operands -> usage error
    err_eq(
        "string compare a",
        "wrong # args: should be \"string compare ?-nocase? ?-length int? string1 string2\"",
    );
    err_eq(
        "string compare a b c d e f",
        "wrong # args: should be \"string compare ?-nocase? ?-length int? string1 string2\"",
    );
    err_eq(
        "string equal x",
        "wrong # args: should be \"string equal ?-nocase? ?-length int? string1 string2\"",
    );
}

// string match

/// `string match ?-nocase? pattern string` including glob metacharacters,
/// `-nocase`, escaped metacharacters, and the option/arity errors.
#[test]
fn string_match_command() {
    // tclsh: glob `*` and `?`
    res_eq("string match {f*o} foo", "1");
    res_eq("string match {a?c} abc", "1");
    res_eq("string match {a?c} ac", "0");
    // tclsh: -nocase
    res_eq("string match -nocase {F*O} foo", "1");
    // tclsh: a bracket set
    res_eq("string match {a[bc]d} abd", "1");
    res_eq("string match {a[bc]d} aed", "0");
    // tclsh: escaped metacharacters match literally
    res_eq("string match {a\\*b} a*b", "1");
    res_eq("string match {a\\?b} a?b", "1");
    res_eq("string match {a\\*b} axb", "0");
    // tclsh: abbreviated -nocase (-n)
    res_eq("string match -n {F*O} foo", "1");
    // tclsh: arity error (the bad-option case is a VM bug — see
    // `bug_string_match_bad_option`).
    err_eq(
        "string match a",
        "wrong # args: should be \"string match ?-nocase? pattern string\"",
    );
}

/// BUG: `string match <bad-option> pattern string` (three args where the first
/// is an unrecognised option) reports a generic "wrong # args" instead of the
/// bad-option error. Both tclsh 8.6 and 9.0 flag the option; the VM routes the
/// 3-argument form through the shared core's arity check before validating the
/// leading option word.
///
/// Script:        string match -bogus a b
/// tclsh 8.6/9.0: bad option "-bogus": must be -nocase
/// VM (wrong):    wrong # args: should be "string match ?-nocase? pattern string"
#[test]
fn bug_string_match_bad_option() {
    // tclsh 8.6 + 9.0
    err_eq(
        "string match -bogus a b",
        "bad option \"-bogus\": must be -nocase",
    );
    err_eq("string match -- a a", "bad option \"--\": must be -nocase");
}

// string first / string last (with start/last indices)

/// `string first needle haystack ?startIndex?`.
#[test]
fn string_first_command() {
    // tclsh
    res_eq("string first bc abcbc", "1");
    res_eq("string first bc abcbc 2", "3"); // resume at index 2
    res_eq("string first bc abcbc end", "-1"); // start past last match
    res_eq("string first bc abcbc 0", "1");
    res_eq("string first zz abc", "-1"); // not found
    res_eq("string first {} abc", "-1"); // empty needle -> -1
    res_eq("string first a {}", "-1"); // empty haystack -> -1
    // tclsh: errors
    err_eq(
        "string first a abc xx",
        "bad index \"xx\": must be integer?[+-]integer? or end?[+-]integer?",
    );
    err_eq(
        "string first a",
        "wrong # args: should be \"string first needleString haystackString ?startIndex?\"",
    );
}

/// `string last needle haystack ?lastIndex?`.
#[test]
fn string_last_command() {
    // tclsh
    res_eq("string last bc abcbc", "3");
    res_eq("string last bc abcbc 2", "1"); // search at/before index 2
    res_eq("string last bc abcbc 0", "-1"); // window too small
    res_eq("string last bc abcbc -1", "-1"); // negative lastIndex -> -1
    res_eq("string last {} abc", "-1"); // empty needle -> -1
    res_eq("string last bc abcbcbc", "5");
    // tclsh: errors
    err_eq(
        "string last a abc xx",
        "bad index \"xx\": must be integer?[+-]integer? or end?[+-]integer?",
    );
    err_eq(
        "string last a",
        "wrong # args: should be \"string last needleString haystackString ?lastIndex?\"",
    );
}

// string tolower / toupper / totitle — with ?first? ?last? ranges

/// Case conversion with the optional `?first? ?last?` range form.
#[test]
fn string_case_conversion() {
    // tclsh: whole-string
    res_eq("string toupper hello", "HELLO");
    res_eq("string tolower HELLO", "hello");
    res_eq("string totitle hello", "Hello");
    res_eq("string totitle {hello world}", "Hello world");
    // tclsh: with a `first` index only -> just that one character
    res_eq("string toupper hello 2", "heLlo");
    res_eq("string totitle HELLO 2", "HELLO"); // titlecase of 'L' is 'L'
    // tclsh: with `first last`
    res_eq("string toupper hello 0 2", "HELlo");
    res_eq("string tolower HELLO 1 3", "HellO");
    res_eq("string totitle {hello world} 0 4", "Hello world");
    // tclsh: end-relative range
    res_eq("string toupper hello end-1 end", "helLO");
    // tclsh: a negative first clamps to 0
    res_eq("string toupper hello -3 2", "HELlo");
    // tclsh: empty string -> empty
    res_eq("string toupper {}", "");
    res_eq("string totitle {}", "");
}

/// Case-conversion argument and index errors.
#[test]
fn string_case_conversion_errors() {
    // tclsh: bad `first` / `last` index
    err_eq(
        "string toupper hello xx",
        "bad index \"xx\": must be integer?[+-]integer? or end?[+-]integer?",
    );
    err_eq(
        "string toupper hello 0 xx",
        "bad index \"xx\": must be integer?[+-]integer? or end?[+-]integer?",
    );
    // tclsh: too many args
    err_eq(
        "string toupper a b c d",
        "wrong # args: should be \"string toupper string ?first? ?last?\"",
    );
    err_eq(
        "string tolower a b c d",
        "wrong # args: should be \"string tolower string ?first? ?last?\"",
    );
    err_eq(
        "string totitle a b c d",
        "wrong # args: should be \"string totitle string ?first? ?last?\"",
    );
}

// string trim / trimleft / trimright

/// The `string trim` family, default and custom trim sets.
#[test]
fn string_trim_family() {
    // tclsh: default whitespace trim
    res_eq("string trim {  hi  }", "hi");
    res_eq("string trimleft {  hi}", "hi");
    res_eq("string trimright {hi  }", "hi");
    // tclsh: custom trim set
    res_eq("string trim xxhixx x", "hi");
    res_eq("string trimleft xxabc x", "abc");
    res_eq("string trimright abcxx x", "abc");
    // tclsh: empty string with a custom set -> empty
    res_eq("string trim {} x", "");
    // tclsh: nothing to trim
    res_eq("string trim hi x", "hi");
    // tclsh: errors (arity)
    err_eq(
        "string trim",
        "wrong # args: should be \"string trim string ?chars?\"",
    );
    err_eq(
        "string trim a b c",
        "wrong # args: should be \"string trim string ?chars?\"",
    );
    err_eq(
        "string trimleft a b c",
        "wrong # args: should be \"string trimleft string ?chars?\"",
    );
    err_eq(
        "string trimright a b c",
        "wrong # args: should be \"string trimright string ?chars?\"",
    );
}

// string map

/// `string map ?-nocase? charMap string`.
#[test]
fn string_map_command() {
    // tclsh
    res_eq("string map {a A o O} banana", "bAnAnA");
    res_eq("string map {a A b B} abcab", "ABcAB");
    // tclsh: -nocase
    res_eq("string map -nocase {ok 0} {OK ok}", "0 0");
    // tclsh: an empty key in the map is ignored; a A -> b
    res_eq("string map {{} X a b} aa", "bb");
    // tclsh: first matching pair (in map order) wins
    res_eq("string map {ab X abc Y} abc", "Xc");
    // tclsh: empty map leaves the string unchanged
    res_eq("string map {} abc", "abc");
    // tclsh: errors
    err_eq("string map {a} foo", "char map list unbalanced");
    err_eq(
        "string map {a b} abba oops",
        "bad option \"a b\": must be -nocase",
    );
    err_eq(
        "string map x",
        "wrong # args: should be \"string map ?-nocase? charMap string\"",
    );
}

// string replace / string insert

/// `string replace string first last ?newstring?`.
#[test]
fn string_replace_command() {
    // tclsh
    res_eq("string replace abcde 1 3", "ae");
    res_eq("string replace abcde 1 3 XY", "aXYe");
    res_eq("string replace abcde 2 end", "ab");
    // tclsh: an inverted range leaves the string unchanged
    res_eq("string replace abcde 3 1", "abcde");
    // tclsh: a leading negative `first` clamps to 0
    res_eq("string replace abcde -1 2 X", "Xde");
    // tclsh: replacing into the empty string with first=-1 last=0 (tclCmdMZ.c)
    res_eq("string replace {} -1 0 A", "A");
    // tclsh: errors
    err_eq(
        "string replace ab 1",
        "wrong # args: should be \"string replace string first last ?string?\"",
    );
    err_eq(
        "string replace ab 1 2 3 4",
        "wrong # args: should be \"string replace string first last ?string?\"",
    );
    err_eq(
        "string replace abc x 2",
        "bad index \"x\": must be integer?[+-]integer? or end?[+-]integer?",
    );
    err_eq(
        "string replace abc 0 x",
        "bad index \"x\": must be integer?[+-]integer? or end?[+-]integer?",
    );
}

/// `string insert string index insertString` (Tcl 9). Pinned against tclsh9.0
/// (the subcommand does not exist in 8.6).
#[test]
fn string_insert_command() {
    // tclsh9.0
    res_eq("string insert abc 1 XY", "aXYbc");
    res_eq("string insert abc 0 Q", "Qabc");
    res_eq("string insert abc end Z", "abcZ"); // `end` appends (position after last)
    // tclsh9.0: a negative index clamps to the front
    res_eq("string insert abc -5 Q", "Qabc");
    // tclsh9.0: arity error
    err_eq(
        "string insert abc 1",
        "wrong # args: should be \"string insert string index insertString\"",
    );
}

// string subcommand dispatch — prefix resolution + bad subcommand

/// `string` subcommand resolution: unique-prefix abbreviation, the
/// ambiguous/unknown error, and the no-subcommand error.
#[test]
fn string_subcommand_dispatch() {
    const MUST: &str = "must be cat, compare, equal, first, index, insert, is, last, length, \
                        map, match, range, repeat, replace, reverse, tolower, totitle, \
                        toupper, trim, trimleft, trimright, wordend, or wordstart";
    // tclsh: unique prefixes resolve
    res_eq("string le hello", "5"); // le -> length
    res_eq("string rev hello", "olleh"); // rev -> reverse
    res_eq("string eq abc abc", "1"); // eq -> equal
    // tclsh9.0.4: an unknown subcommand lists the canonical set, joined by the
    // ensemble's rule (a comma before `or`) — since #1607 the whole sentence is
    // `tcl_cmd_core::ensemble`'s, so it is pinned byte for byte.
    //
    // tclsh9.0.4:
    //   string bogus x -> unknown or ambiguous subcommand "bogus": must be cat,
    //                     compare, equal, first, index, insert, is, last,
    //                     length, map, match, range, repeat, replace, reverse,
    //                     tolower, totitle, toupper, trim, trimleft, trimright,
    //                     wordend, or wordstart
    //   string to abc  -> unknown or ambiguous subcommand "to": must be <same>
    //   string {} a    -> unknown or ambiguous subcommand "": must be <same>
    let (ok, msg, _) = run("string bogus x");
    assert!(!ok);
    assert_eq!(
        msg,
        format!("unknown or ambiguous subcommand \"bogus\": {MUST}")
    );
    // tclsh: an ambiguous prefix (`to` -> tolower/totitle/toupper) errors too.
    let (ok, msg, _) = run("string to abc");
    assert!(!ok);
    assert_eq!(
        msg,
        format!("unknown or ambiguous subcommand \"to\": {MUST}")
    );
    // The empty word prefixes every entry, so it lands on the same sentence.
    let (ok, msg, _) = run("string {} a");
    assert!(!ok);
    assert_eq!(msg, format!("unknown or ambiguous subcommand \"\": {MUST}"));
    // tclsh: no subcommand at all.
    err_eq(
        "string",
        "wrong # args: should be \"string subcommand ?arg ...?\"",
    );
}

// string is — class membership

/// `string is class str` — character and value class membership. The class set
/// follows Tcl 9 (`dict`, arbitrary-precision `integer`), so the divergent
/// classes cite tclsh9.0.
#[test]
fn string_is_class_membership() {
    // tclsh: character classes
    res_eq("string is alnum abc123", "1");
    res_eq("string is alnum {ab c}", "0"); // space breaks alnum
    res_eq("string is alpha abc", "1");
    res_eq("string is alpha ab1", "0");
    res_eq("string is ascii abc", "1");
    res_eq("string is ascii é", "0"); // non-ASCII
    res_eq("string is digit 123", "1");
    res_eq("string is digit 12x", "0");
    res_eq("string is upper ABC", "1");
    res_eq("string is upper Abc", "0");
    res_eq("string is lower abc", "1");
    res_eq("string is lower aBc", "0");
    res_eq("string is xdigit deadBEEF", "1");
    res_eq("string is xdigit xyz", "0");
    res_eq("string is punct {!@#}", "1");
    res_eq("string is punct abc", "0");
    res_eq("string is wordchar foo_1", "1");
    res_eq("string is wordchar foo-1", "0");
    // tclsh: graph excludes space; print includes it; control is non-printable.
    res_eq("string is graph abc", "1");
    res_eq("string is graph { }", "0");
    res_eq("string is graph {a b}", "0");
    res_eq("string is print {a b}", "1");
    res_eq("string is space { }", "1");
    res_eq("string is space abc", "0");
    res_eq("string is control [format %c 7]", "1");
    res_eq("string is control abc", "0");
    res_eq("string is print [format %c 7]", "0"); // a bell is not printable
    // tclsh: numeric value classes
    res_eq("string is integer 123", "1");
    res_eq("string is integer 12x", "0");
    res_eq("string is integer { 5 }", "1"); // surrounding whitespace allowed
    res_eq("string is wideinteger 99", "1");
    res_eq("string is wideinteger 1.5", "0");
    res_eq("string is double 1.5", "1");
    res_eq("string is double 1e3", "1");
    res_eq("string is double abc", "0");
    res_eq("string is entier 123", "1");
    res_eq("string is entier 1.5", "0");
    // tclsh: boolean / true / false
    res_eq("string is boolean yes", "1");
    res_eq("string is boolean 0", "1");
    res_eq("string is boolean maybe", "0");
    res_eq("string is true true", "1");
    res_eq("string is true no", "0");
    res_eq("string is false false", "1");
    res_eq("string is false 1", "0");
    // tclsh: list / empty
    res_eq("string is list {a b c}", "1");
    res_eq("string is list \"a \\{b\"", "0"); // unbalanced brace
    res_eq("string is list {}", "1"); // empty is a valid (empty) list
}

/// `string is` classes whose behaviour follows Tcl 9 — pinned against tclsh9.0
/// (8.6 lacks `dict` and treats an over-wide `integer` as a non-member).
#[test]
fn string_is_tcl9_classes() {
    // tclsh9.0: `dict` class.
    res_eq("string is dict {a 1 b 2}", "1");
    res_eq("string is dict {a 1 b}", "0"); // odd length
    res_eq("string is dict {}", "1"); // empty is a valid (empty) dict
    // tclsh9.0: `integer` is arbitrary-precision (8.6 -> 0 here).
    res_eq("string is integer 99999999999999999999999", "1");
    // tclsh9.0: `wideinteger` stays bounded to 64 bits (over-wide -> non-member).
    res_eq("string is wideinteger 99999999999999999999999", "0");
}

/// `string is class -strict ?-failindex var? str` — the `-strict` flag (empty
/// string becomes a non-member), `-failindex` (records the first failing char),
/// option abbreviation/order, and the class-name/option errors.
#[test]
fn string_is_strict_and_failindex() {
    // tclsh: the empty string is a member of every char class unless -strict.
    res_eq("string is alpha {}", "1");
    res_eq("string is alpha -strict {}", "0");
    res_eq("string is digit -strict {}", "0");
    res_eq("string is integer -strict {}", "0");
    res_eq("string is alpha -strict abc", "1"); // non-empty: -strict no-ops
    // tclsh: -failindex writes the first failing character index (and the result
    // is the membership boolean). (The internal-whitespace integer case is a VM
    // bug — see `bug_string_is_integer_failindex_internal_space`.)
    res_eq("string is alpha -failindex fi abc5; set fi", "3");
    res_eq("string is double -failindex fi 1.5x; set fi", "3");
    // tclsh: when the string IS a member, the fail var is left untouched.
    res_eq(
        "set fi PRE; string is alpha -failindex fi abc; set fi",
        "PRE",
    );
    // tclsh: -strict combined with -failindex on the empty string -> index 0.
    res_eq("string is alpha -strict -failindex fi {}; set fi", "0");
    // tclsh: option order may be reversed (-failindex then -strict).
    res_eq("string is alpha -failindex fi -strict 5a; set fi", "0");
    // tclsh: abbreviated options (-s, -f).
    res_eq("string is alpha -s {}", "0");
    res_eq("string is alpha -f fi abc5; set fi", "3");
}

/// `string is` class-name and argument errors. The class list follows Tcl 9
/// (includes `dict`), so the bad/ambiguous-class messages cite tclsh9.0.
#[test]
fn string_is_errors() {
    // tclsh9.0: a bad class lists the canonical classes (with `dict`).
    let (ok, msg, _) = run("string is bogus abc");
    assert!(!ok);
    assert_eq!(
        msg,
        "bad class \"bogus\": must be alnum, alpha, ascii, control, boolean, dict, digit, \
         double, entier, false, graph, integer, list, lower, print, punct, space, true, \
         upper, wideinteger, wordchar, or xdigit"
    );
    // tclsh9.0: an ambiguous class prefix.
    let (ok, msg, _) = run("string is a abc");
    assert!(!ok);
    assert!(
        msg.starts_with("ambiguous class \"a\": must be alnum, alpha,"),
        "got: {msg}"
    );
    // tclsh: too few args -> the generic usage error.
    err_eq(
        "string is alpha",
        "wrong # args: should be \"string is class ?-strict? ?-failindex var? str\"",
    );
    // tclsh: with exactly one trailing word, that word is the *string* under
    // test, not an option — `string is alpha -failindex` tests the literal
    // "-failindex" (which is not alpha) and returns 0, no error.
    res_eq("string is alpha -failindex", "0");
    res_eq("string is alpha -strict", "0");
    // tclsh: `-failindex var` with no remaining string consumes the var as the
    // option argument, leaving nothing to test -> the per-class usage error.
    err_eq(
        "string is alpha -failindex fi",
        "wrong # args: should be \"string is alpha ?-strict? ?-failindex var? str\"",
    );
    // tclsh: a -failindex target that cannot be written (the variable name
    // resolves to an existing array base) surfaces the variable error from the
    // failing-classification write path.
    err_eq(
        "array set arr {a 1}; string is alpha -failindex arr ab5",
        "can't set \"arr\": variable is array",
    );
}

/// BUG: `string is <class> <option> <str>` where `<option>` is an unrecognised
/// word and a trailing operand follows reports the *wrong* error. Real tclsh
/// flags the bad option; the VM mis-counts the operands and reports a generic
/// "wrong # args".
///
/// Script:        string is alpha a b
/// tclsh 8.6/9.0: bad option "a": must be -strict or -failindex
/// VM (wrong):    wrong # args: should be "string is class ?-strict? ?-failindex var? str"
#[test]
fn bug_string_is_bad_option_with_trailing_arg() {
    // tclsh 8.6 + 9.0
    err_eq(
        "string is alpha a b",
        "bad option \"a\": must be -strict or -failindex",
    );
}

/// A lone `-` (or the empty word) in option position prefixes *both* options,
/// so C's `Tcl_GetIndexFromObj` reports it as ambiguous, not bad. (Previously
/// the VM's hand-rolled `len >= 2` prefix test reported `bad option`.)
#[test]
fn string_is_dash_and_empty_option_words_are_ambiguous() {
    // tclsh 8.6.16: ambiguous option "-": must be -strict or -failindex
    err_eq(
        "string is integer - 5",
        "ambiguous option \"-\": must be -strict or -failindex",
    );
    // tclsh 8.6.16: same report for the empty word.
    err_eq(
        "string is integer \"\" 0",
        "ambiguous option \"\": must be -strict or -failindex",
    );
}

/// BUG: `string is integer -failindex` reports the wrong failure index when the
/// integer has internal whitespace followed by a non-digit. For `"12 x"` real
/// tclsh records the failure at the `x` (char index 3); the VM stops at the
/// interior space (index 2). (`"12 "` with only trailing space is a valid
/// integer in both, so the divergence is specifically the digit-after-space.)
///
/// Script:        string is integer -failindex fi {12 x}; set fi
/// tclsh 8.6/9.0: 3
/// VM (wrong):    2
#[test]
fn bug_string_is_integer_failindex_internal_space() {
    // tclsh 8.6 + 9.0
    res_eq("string is integer -failindex fi {12 x}; set fi", "3");
}

// format — conversions, flags, width, precision, positional specifiers

/// `format` integer conversions with flags, width, precision, and `*`.
#[test]
fn format_integer_conversions() {
    // tclsh
    res_eq("format %d 42", "42");
    res_eq("format %5d 42", "   42");
    res_eq("format %-5d| 42", "42   |");
    res_eq("format %05d 42", "00042");
    res_eq("format %+d 42", "+42");
    res_eq("format {% d} 42", " 42"); // space flag
    res_eq("format {% d} -42", "-42");
    res_eq("format %.5d 42", "00042"); // precision on an int zero-pads digits
    res_eq("format %x 255", "ff");
    res_eq("format %X 255", "FF");
    res_eq("format %#x 255", "0xff");
    res_eq("format %o 8", "10");
    res_eq("format %b 10", "1010");
    res_eq("format %u 42", "42");
    res_eq("format %i 42", "42");
    res_eq("format %lld 42", "42"); // size modifiers accepted
    res_eq("format %llx 255", "ff");
    // tclsh: `*` width/precision pull an integer argument.
    res_eq("format %*d 5 42", "   42");
    res_eq("format %.*f 2 3.14159", "3.14");
    res_eq("format %*d -5 42", "42   "); // negative `*` width left-justifies
}

/// `format` string and character conversions.
#[test]
fn format_string_char_conversions() {
    // tclsh
    res_eq("format %s hello", "hello");
    res_eq("format %.3s hello", "hel"); // precision truncates
    res_eq("format %10s hi", "        hi");
    res_eq("format %-10s| hi", "hi        |");
    res_eq("format %c 65", "A");
    res_eq("format %c 0x4A", "J"); // a numeric code point
    // tclsh: literal percent and a multi-conversion string.
    res_eq("format 100%%", "100%");
    res_eq("format {%d-%s-%x} 5 hi 255", "5-hi-ff");
}

/// `format` floating-point conversions. (The exponential forms `%e`/`%g`'s
/// exponential range are VM bugs — see `bug_format_exponent_format` and
/// `bug_format_g_exponential_range`.)
#[test]
fn format_float_conversions() {
    // tclsh
    res_eq("format %f 3.14159", "3.141590");
    res_eq("format %.2f 3.14159", "3.14");
    res_eq("format %8.2f 3.14159", "    3.14");
    res_eq("format %+.2f 3.14", "+3.14"); // sign flag (the `0`-flag form is a VM bug)
    res_eq("format %-8.2f| 3.14", "3.14    |"); // left-justify
    res_eq("format %f 0.0", "0.000000");
    // tclsh: `%g` over a normal-magnitude value renders fixed-point, trimming
    // trailing zeros (the exponential-range behaviour is a VM bug).
    res_eq("format %g 1.5", "1.5");
    res_eq("format %g 100.0", "100");
    res_eq("format %g 0.0001", "0.0001");
}

/// `format` argument / specifier errors. (The `%5` trailing-spec and `%n$`
/// positional/mixing cases are VM bugs — see `bug_format_*`.)
#[test]
fn format_errors() {
    // tclsh
    err_eq(
        "format %d",
        "not enough arguments for all format specifiers",
    );
    err_eq("format %y 1", "bad field specifier \"y\"");
    err_eq(
        "format",
        "wrong # args: should be \"format formatString ?arg ...?\"",
    );
    err_eq("format %d abc", "expected integer but got \"abc\"");
    err_eq(
        "format %f abc",
        "expected floating-point number but got \"abc\"",
    );
    // tclsh: a complete specifier (space flag + `d`) with no argument supplied.
    err_eq(
        "format {% d}",
        "not enough arguments for all format specifiers",
    );
    // tclsh: a `bad field specifier` (a space after the width is not a verb).
    err_eq("format {%5 } 1", "bad field specifier \" \"");
}

/// BUG: `format %#b` omits the `0b` alternate-form prefix. Both tclsh 8.6 and
/// tclsh 9.0 prepend `0b`; the VM's `tcl_cmd_core::format::based_digits` returns
/// no prefix for the binary verb.
///
/// Script:        format %#b 5
/// tclsh 8.6/9.0: 0b101
/// VM (wrong):    101
#[test]
fn bug_format_hash_binary_prefix() {
    // tclsh 8.6 + 9.0
    res_eq("format %#b 5", "0b101");
}

/// BUG: `format %#o` uses the legacy `0` octal prefix instead of Tcl 9's `0o`.
/// The VM matches tclsh 8.6, but it otherwise targets Tcl 9.0 (lowercase `0x`
/// for `%#X`, `entier`/`dict` classes, arbitrary-precision `string is integer`),
/// where the octal alternate form is `0o`.
///
/// Script:     format %#o 8
/// tclsh 9.0:  0o10
/// VM (wrong): 010   (tclsh 8.6 behaviour)
#[test]
fn bug_format_hash_octal_prefix_tcl9() {
    // tclsh9.0
    res_eq("format %#o 8", "0o10");
}

/// BUG: `format %#d` drops the Tcl 9 `0d` alternate-form prefix. As with `%#o`,
/// the VM matches tclsh 8.6 (no prefix) but targets Tcl 9.0, where `%#d` renders
/// a leading `0d`.
///
/// Script:     format %#d 42
/// tclsh 9.0:  0d42
/// VM (wrong): 42   (tclsh 8.6 behaviour)
#[test]
fn bug_format_hash_decimal_prefix_tcl9() {
    // tclsh9.0
    res_eq("format %#d 42", "0d42");
}

/// BUG: the `0` (zero-pad) flag is ignored for floating-point conversions. Both
/// tclsh 8.6 and 9.0 zero-pad floats to the field width; the VM pads with spaces
/// (the `0` flag works for integers — `%08d` is fine — so this is float-specific
/// in `tcl_cmd_core::format`).
///
/// Script:        format %08.2f 3.14
/// tclsh 8.6/9.0: 00003.14
/// VM (wrong):    "    3.14"  (space padded)
#[test]
fn bug_format_zero_flag_on_float() {
    // tclsh 8.6 + 9.0
    res_eq("format %08.2f 3.14", "00003.14");
    res_eq("format %+08.2f 3.14", "+0003.14");
}

/// BUG: `%e`/`%E` render the exponent without the C/Tcl minimum-two-digit,
/// always-signed form. Both tclsh 8.6 and 9.0 print `e+04`; the VM (Rust's
/// `{:e}` formatter) prints `e4` — no sign, no zero-padding.
///
/// Script:        format %e 12345.678
/// tclsh 8.6/9.0: 1.234568e+04
/// VM (wrong):    1.234568e4
#[test]
fn bug_format_exponent_format() {
    // tclsh 8.6 + 9.0
    res_eq("format %e 12345.678", "1.234568e+04");
    res_eq("format %.2e 12345.678", "1.23e+04");
    res_eq("format %E 12345.678", "1.234568E+04");
    res_eq("format %e 1.5", "1.500000e+00");
}

/// BUG: `%g` does not switch to exponential notation for out-of-range
/// magnitudes. C/Tcl `%g` uses `%e` when the exponent is < -4 or >= the
/// precision (default 6); the VM always renders fixed-point and merely trims
/// trailing zeros, so large/small values print in full / with leading zeros.
///
/// Script:        format %g 1234567.0   |   format %g 0.00001
/// tclsh 8.6/9.0: 1.23457e+06           |   1e-05
/// VM (wrong):    1234567               |   0.00001
#[test]
fn bug_format_g_exponential_range() {
    // tclsh 8.6 + 9.0
    res_eq("format %g 1234567.0", "1.23457e+06");
    res_eq("format %g 0.00001", "1e-05");
}

/// BUG: positional conversion specifiers (`%n$`) are unsupported. C/Tcl's
/// `format` accepts `%1$s` to index a specific argument and reuse it; the VM's
/// `tcl_cmd_core::format` parser rejects the `$` as a bad field specifier.
///
/// Script:        format {%1$s %2$s %1$s} a b
/// tclsh 8.6/9.0: a b a
/// VM (wrong):    bad field specifier "$"
#[test]
fn bug_format_positional_specifiers() {
    // tclsh 8.6 + 9.0
    res_eq("format {%1$s %2$s %1$s} a b", "a b a");
    res_eq("format {%2$d-%1$d} 10 20", "20-10");
}

/// Positional (`%n$`) argument-mode rules, oracle-pinned to tclsh 9.0:
/// a positional spec draws (and may reuse) a specific argument; with a `*`
/// width it consumes consecutively from `n` (width from arg `n`, value from
/// `n+1`); and a format must not *mix* positional and sequential specifiers.
#[test]
fn format_positional_mode_star_and_mixing() {
    // A lone positional spec selects its argument, leaving extras unused.
    res_eq("format {%2$d} 10 20", "20");
    // The same argument can be reused without advancing a cursor.
    res_eq("format {%1$s %1$s} a b", "a a");
    // `%2$*d`: width from arg 2 (3), value from arg 3 (42) → " 42".
    res_eq("format {%2$*d} 5 3 42", " 42");
    // Mixing positional and sequential conversions is an error either order.
    err_eq(
        "format {%2$d %d} 10 20",
        "cannot mix \"%\" and \"%n$\" conversion specifiers",
    );
    err_eq(
        "format {%d %1$d} 5 6",
        "cannot mix \"%\" and \"%n$\" conversion specifiers",
    );
}

/// BUG: a format string that ends with an incomplete specifier (a width but no
/// conversion verb) is silently echoed instead of erroring. tclsh reports
/// "not enough arguments for all format specifiers"; the VM returns the literal
/// text.
///
/// Script:        format %5
/// tclsh 8.6/9.0: (error) not enough arguments for all format specifiers
/// VM (wrong):    %5
#[test]
fn bug_format_trailing_incomplete_specifier() {
    // tclsh 8.6 + 9.0
    err_eq(
        "format %5",
        "not enough arguments for all format specifiers",
    );
}

// puts-captured output forms (exercise the dispatch through stdout too)

/// A spot-check that the same results flow through `puts` to captured stdout
/// (the form many real scripts use), covering the `string`/`format` adapters
/// end to end.
#[test]
fn string_format_through_puts() {
    // tclsh
    assert_eq!(run("puts [string toupper hello]").2, "HELLO\n");
    assert_eq!(run("puts [string is integer 42]").2, "1\n");
    assert_eq!(run("puts [format %05d 42]").2, "00042\n");
    assert_eq!(run("puts [format %.2f 1.5]").2, "1.50\n");
    assert_eq!(run("puts [string map {a A} banana]").2, "bAnAnA\n");
}
