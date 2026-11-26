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

//! String opcodes: `strmap`, `strfind`, the case operators, `tryCvtToBoolean`, `strindex` and `strrange`.

use super::*;

// `strmap`.

/// `strmap` is the one-pair, case-sensitive `string map`: the stack is
/// `from to string` with the subject on top (C `INST_STR_MAP`).
#[test]
fn str_map_stack_order_and_mapping() {
    let mut a = Asm::new();
    a.push("ab").push("X").push("abcab").op(Op::STR_MAP, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "XcX");
}

/// The C fast paths: an empty `from`, a `from` longer than the subject, and a
/// whole-string match all behave like the general mapping loop.
#[test]
fn str_map_edge_cases() {
    for (from, to, subject, want) in [
        ("", "X", "abc", "abc"),     // empty key never matches
        ("abcd", "X", "abc", "abc"), // key longer than the subject
        ("abc", "X", "abc", "X"),    // whole-string match
        ("abc", "X", "abd", "abd"),  // same length, no match
        ("a", "", "banana", "bnn"),  // empty replacement deletes
        ("A", "X", "aAa", "aXa"),    // case-sensitive (no -nocase form)
    ] {
        let mut a = Asm::new();
        a.push(from).push(to).push(subject).op(Op::STR_MAP, &[]);
        let (_vm, c) = run_fresh(a);
        assert_eq!(ok_str(&c), want, "strmap {from} {to} {subject}");
    }
}

/// The opcode and the `string map` command agree on the same one-pair map.
#[test]
fn str_map_matches_string_map_command() {
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(compiler::svc()));
    let cmd = vm
        .eval_source("string map {ab X} abcab")
        .expect("compiles")
        .result
        .to_str()
        .to_string();
    let mut a = Asm::new();
    a.push("ab").push("X").push("abcab").op(Op::STR_MAP, &[]);
    let c = run(&mut vm, a);
    assert_eq!(ok_str(&c), cmd);
}

// `strfind` / `strrfind`.

/// An **empty needle is a miss**. Both C helpers open with
/// `if (ln == 0) { /* We don't find empty substrings.  Bizarre! */ … }` leaving
/// the result at -1 (`tclStringObj.c:3853-3858` `TclStringFirst`, `:3948-3956`
/// `TclStringLast`), where Rust's `str::find`/`rfind` would answer 0 and the
/// haystack length. Stack order is C's `TclStringFirst(OBJ_UNDER_TOS,
/// OBJ_AT_TOS, …)` — needle under, haystack on top (`tclExecute.c:5555-5567`).
#[test]
fn str_find_empty_needle_is_minus_one() {
    for (op, want) in [(Op::STR_FIND, "-1"), (Op::STR_RFIND, "-1")] {
        let mut a = Asm::new();
        a.push("").push("abc").op(op, &[]);
        let (_vm, c) = run_fresh(a);
        assert_eq!(ok_str(&c), want, "{op:?} with an empty needle");
    }
    // A non-empty needle keeps its character index (first vs last occurrence).
    let mut a = Asm::new();
    a.push("l").push("hello").op(Op::STR_FIND, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "2");
    let mut a = Asm::new();
    a.push("l").push("hello").op(Op::STR_RFIND, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "3");
}

/// The opcodes agree with the `string first`/`string last` commands, which
/// already returned -1 for an empty needle.
#[test]
fn str_find_matches_string_first_last_commands() {
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(compiler::svc()));
    for (sub, op) in [("first", Op::STR_FIND), ("last", Op::STR_RFIND)] {
        let cmd = vm
            .eval_source(&format!("string {sub} \"\" abc"))
            .expect("compiles")
            .result
            .to_str()
            .to_string();
        let mut a = Asm::new();
        a.push("").push("abc").op(op, &[]);
        let c = run(&mut vm, a);
        assert_eq!(ok_str(&c), cmd, "string {sub} vs {op:?}");
    }
}

// `strupper` / `strlower` / `strtitle`.

/// C `INST_STR_UPPER`/`LOWER`/`TITLE` (`tclExecute.c:5284-5334`) call
/// `Tcl_UtfToUpper`/`ToLower`/`ToTitle`, which map **one code point to one code
/// point** through `Tcl_UniCharToUpper` &co (`tclUtf.c:1777-1858`) — Tcl's
/// tables hold only Unicode's *simple* mappings, so `ß` (U+00DF, no simple
/// uppercase) is unchanged and the character count never changes. Rust's
/// `to_uppercase()`/`to_lowercase()` implement *full* mapping and would
/// expand (`ß` → `SS`, `İ` → `i` + U+0307) if these arms used them.
#[test]
fn str_case_ops_use_simple_unicode_mapping() {
    for (op, subject, want) in [
        // Expanding full mappings have no C counterpart: character preserved.
        (Op::STR_UPPER, "\u{00DF}", "\u{00DF}"), // ß, not SS
        // …but U+0130 *does* have a simple mapping (`UnicodeData.txt` field 13
        // = 0069), so `Tcl_UniCharToLower` gives plain `i` where Rust's full
        // mapping gives `i` + U+0307. Sole such code point in Unicode.
        (Op::STR_LOWER, "\u{0130}", "i"),
        (Op::STR_TITLE, "\u{00DF}x", "\u{00DF}x"),
        // Ordinary 1:1 mappings still apply.
        (Op::STR_UPPER, "aé\u{0131}", "AÉI"),
        (Op::STR_LOWER, "AÉ", "aé"),
        (Op::STR_TITLE, "hELLO wORLD", "Hello world"),
        // `Tcl_UniCharToTitle` differs from uppercase for the Latin digraphs.
        (Op::STR_TITLE, "\u{01C4}z", "\u{01C5}z"), // DŽ → Dž
    ] {
        let mut a = Asm::new();
        a.push(subject).op(op, &[]);
        let (_vm, c) = run_fresh(a);
        assert_eq!(ok_str(&c), want, "{op:?} {subject:?}");
    }
}

/// The opcodes and the `string toupper`/`tolower`/`totitle` commands share the
/// simple-mapping helpers, so they cannot drift apart again.
#[test]
fn str_case_ops_match_string_case_commands() {
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(compiler::svc()));
    for (sub, op) in [
        ("toupper", Op::STR_UPPER),
        ("tolower", Op::STR_LOWER),
        ("totitle", Op::STR_TITLE),
    ] {
        for subject in ["\u{00DF}", "\u{0130}", "hELLO wORLD", "\u{01C4}z"] {
            let cmd = vm
                .eval_source(&format!("string {sub} {{{subject}}}"))
                .expect("compiles")
                .result
                .to_str()
                .to_string();
            let mut a = Asm::new();
            a.push(subject).op(op, &[]);
            let c = run(&mut vm, a);
            assert_eq!(ok_str(&c), cmd, "string {sub} {subject:?} vs {op:?}");
        }
    }
}

// `tryCvtToBoolean`.

/// C `INST_TRY_CVT_TO_BOOLEAN` (`tclExecute.c:6404-6414`; table
/// `tclCompile.c:616` — `{"tryCvtToBoolean", 1, +1, 0, {OPERAND_NONE}}`, "Try
/// converting stktop to boolean if possible. **No errors.** Stack: … value =>
/// … value isStrictBool") leaves the value in place and pushes a 0/1 flag, and
/// never raises. Here the flag is consumed by `strEq` against a literal so both
/// the flag *and* the surviving value are observed. The flag is the **strict**
/// acceptor (`TclSetBooleanFromAny` → `ParseBoolean`, `tclObj.c:2100-2280`), so
/// `2` and `1.5` are *not* booleans even though they are truthy in `if`.
#[test]
fn try_cvt_to_boolean_pushes_flag_and_keeps_value() {
    for (subject, want_flag) in [
        ("yes", "1"),
        ("0", "1"),
        ("1", "1"),
        ("true", "1"),
        ("of", "1"), // unique prefix of `off`
        ("foo", "0"),
        ("", "0"),
        ("2", "0"),
        ("1.5", "0"),
        ("o", "0"), // ambiguous prefix (`on`/`off`)
    ] {
        // … value flag => the flag, with the value still underneath.
        let mut a = Asm::new();
        a.push(subject).op(Op::TRY_CVT_TO_BOOLEAN, &[]);
        a.push(want_flag).op(Op::STR_EQ, &[]);
        let (_vm, c) = run_fresh(a);
        assert_eq!(ok_str(&c), "1", "tryCvtToBoolean {subject:?} flag");

        // Discard the flag and the original value must still be there — the +1
        // stack effect, not the 0 our old pop/re-push arm had.
        let mut a = Asm::new();
        a.push(subject).op(Op::TRY_CVT_TO_BOOLEAN, &[]);
        a.op(Op::POP, &[]);
        let (_vm, c) = run_fresh(a);
        assert_eq!(ok_str(&c), subject, "tryCvtToBoolean {subject:?} value");
    }
}

// `strindex` / `strrange`.

/// C `INST_STR_INDEX` (`tclExecute.c:5336-5380`) and `INST_STR_RANGE`
/// (`:5382-5406`) both run their indices through `TclGetIntForIndexM` and treat
/// a *parse* failure as `goto gotError`; only the separate range tests
/// (`index < 0 || index >= slength`, and `toIdx == TCL_INDEX_NONE`) produce the
/// empty result. Clamping a bad spec to 0/-1 turned a typo into a
/// plausible-looking substring.
#[test]
fn str_index_and_range_reject_bad_indices() {
    let bad = |spec: &str| {
        format!("bad index \"{spec}\": must be integer?[+-]integer? or end?[+-]integer?")
    };
    let mut a = Asm::new();
    a.push("abc").push("foo").op(Op::STR_INDEX, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), bad("foo"));

    // A list-valued index is not an index spec for the string ops.
    let mut a = Asm::new();
    a.push("abc").push("1 0").op(Op::STR_INDEX, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), bad("1 0"));

    // `first` is validated before `last`, matching C's two sequential
    // `TclGetIntForIndexM` calls.
    let mut a = Asm::new();
    a.push("abcde").push("foo").push("2").op(Op::STR_RANGE, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), bad("foo"));

    let mut a = Asm::new();
    a.push("abcde").push("1").push("bar").op(Op::STR_RANGE, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), bad("bar"));
}

/// Valid-but-out-of-range indices keep C's clamp/empty behaviour — no error.
#[test]
fn str_index_and_range_still_clamp_valid_indices() {
    for (idx, want) in [("0", "a"), ("2", "c"), ("end", "c"), ("9", ""), ("-1", "")] {
        let mut a = Asm::new();
        a.push("abc").push(idx).op(Op::STR_INDEX, &[]);
        let (_vm, c) = run_fresh(a);
        assert_eq!(ok_str(&c), want, "strindex abc {idx}");
    }
    for (from, to, want) in [
        ("1", "3", "bcd"),
        ("0", "end", "abcde"),
        ("-3", "2", "abc"), // negative `first` clamps to 0
        ("0", "end+1", "abcde"),
        ("3", "1", ""),  // reversed range
        ("0", "-1", ""), // `toIdx == TCL_INDEX_NONE`
        ("9", "20", ""),
    ] {
        let mut a = Asm::new();
        a.push("abcde").push(from).push(to).op(Op::STR_RANGE, &[]);
        let (_vm, c) = run_fresh(a);
        assert_eq!(ok_str(&c), want, "strrange abcde {from} {to}");
    }
}
