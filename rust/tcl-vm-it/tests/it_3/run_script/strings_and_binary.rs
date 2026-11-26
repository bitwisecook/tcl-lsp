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

//! String, `format`, `binary`, `regexp` and `encoding` commands and their compiled forms.

use super::*;

/// `string map` with three arguments whose first is not `-nocase` reports a
/// bad-option error, not a wrong-arg-count error (string-10.2).
#[test]
fn string_map_three_args_bad_option() {
    let (ok, result, _) = run("catch {string map {a b} abba oops} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "bad option \"a b\": must be -nocase");
}

/// `binary encode`/`decode` — newly added to the VM, sharing the byte codecs
/// with the WASM runtime (`tcl_cmd_core::binary`). Values cross the VM's value
/// boundary via the byte-array convention. Pinned against tclsh 9.0.
#[test]
fn binary_encode_decode_shared() {
    assert_eq!(run("binary encode hex abc").1, "616263");
    assert_eq!(run("binary decode hex 616263").1, "abc");
    assert_eq!(run("binary encode base64 hello").1, "aGVsbG8=");
    assert_eq!(run("binary decode base64 aGVsbG8=").1, "hello");
    // base64 line wrapping at -maxlen.
    assert_eq!(run("binary encode base64 -maxlen 4 Manny").1, "TWFu\nbnk=");
    // uuencode round-trips through a command substitution.
    assert_eq!(
        run("binary decode uuencode [binary encode uuencode Cat]").1,
        "Cat"
    );
    // bad codec and the bad-subcommand dispatch message.
    let (ok, msg, _) = run("binary encode zip x");
    assert!(!ok);
    assert_eq!(
        msg,
        "unknown subcommand \"zip\": must be base64, hex, or uuencode"
    );
    let (ok, msg, _) = run("binary frobnicate x");
    assert!(!ok);
    assert_eq!(
        msg,
        "unknown or ambiguous subcommand \"frobnicate\": must be decode, encode, format, or scan"
    );
    // decode error message + errorCode.
    let (ok, msg, _) = run("binary decode hex 6z");
    assert!(!ok);
    assert_eq!(
        msg,
        "invalid hexadecimal digit \"z\" (U+00007A) at position 1"
    );
    assert_eq!(
        run("catch {binary decode hex 6z}; set ::errorCode").1,
        "TCL BINARY DECODE INVALID"
    );
}

/// `binary format`/`scan` routed through the shared grammar
/// (`tcl_cmd_core::binary::{format, scan}`), lifting the VM from its old
/// integer-only subset to the runtime's full code set — floats, 64-bit and
/// big-endian ints, `*` counts, and the round-trip. Pinned against tclsh 9.0.
#[test]
fn binary_format_scan_shared() {
    // codes the VM lacked before sharing: f/d floats, W (64-bit big-endian).
    assert_eq!(
        run("binary scan [binary format f 1.5] H* h; set h").1,
        "0000c03f"
    );
    assert_eq!(
        run("binary scan [binary format d 2.5] H* h; set h").1,
        "0000000000000440"
    );
    assert_eq!(
        run("binary scan [binary format W 1] H* h; set h").1,
        "0000000000000001"
    );
    // `*` count over a list.
    assert_eq!(
        run("binary scan [binary format s* {1 2 3}] H* h; set h").1,
        "010002000300"
    );
    // float round-trips through scan.
    assert_eq!(run("binary scan [binary format f 1.5] f v; set v").1, "1.5");
    // scan returns the conversion count.
    assert_eq!(run("binary scan abcd a2a2 x y").1, "2");
}

/// The `string` subcommands served by the shared `tcl-cmd-core` (driven over
/// the VM's `ValueOps`) behave identically end-to-end.
#[test]
fn string_core_helpers() {
    assert_eq!(run("puts [string length hello]").2, "5\n");
    assert_eq!(run("puts [string index hello 1]").2, "e\n");
    assert_eq!(run("puts [string range hello 1 3]").2, "ell\n");
    assert_eq!(run("puts [string reverse hello]").2, "olleh\n");
    assert_eq!(run("puts [string repeat ab 3]").2, "ababab\n");
    // end-relative and arithmetic index forms resolve in the shared core.
    assert_eq!(run("puts [string index hello end]").2, "o\n");
    assert_eq!(run("puts [string index hello 1+1]").2, "l\n");
    assert_eq!(run("puts [string range hello 1 end-1]").2, "ell\n");
    // count <= 0 yields empty; an out-of-range index yields empty.
    assert_eq!(run("puts \"[string repeat ab 0]<\"").2, "<\n");
    assert_eq!(run("puts \"[string index hello 99]<\"").2, "<\n");
    // Original Tcl 9.0 word-boundary controls use the shared core.
    assert_eq!(run("string wordstart {hello world} 7").1, "6");
    assert_eq!(run("string wordend {hello world} 0").1, "5");
    assert_eq!(run("string wordstart {a_b.c} 2").1, "0"); // underscore is a word char
    assert_eq!(run("string wordend {ab cd} -5").1, "2"); // negative clamps to 0
    assert_eq!(run("string wordstart {ab cd} 10").1, "3"); // past-end clamps
}

/// A non-integer `string repeat` count produces the canonical coercion error,
/// surfaced through `tcl-cmd-core`'s `CmdError` from `ValueError`.
#[test]
fn string_repeat_bad_count_errors() {
    let (ok, result, _out) = run("string repeat ab x");
    assert!(!ok);
    assert_eq!(result, "expected integer but got \"x\"");
}

/// Case conversions (with the `?first? ?last?` range form), `replace`, `insert`.
#[test]
fn string_case_replace_insert() {
    assert_eq!(run("puts [string toupper hello]").2, "HELLO\n");
    assert_eq!(run("puts [string toupper hello 0 2]").2, "HELlo\n");
    assert_eq!(run("puts [string tolower HELLO 1]").2, "HeLLO\n");
    assert_eq!(run("puts [string totitle hello]").2, "Hello\n");
    assert_eq!(run("puts [string replace abcde 1 3]").2, "ae\n");
    assert_eq!(run("puts [string replace abcde 1 3 XY]").2, "aXYe\n");
    assert_eq!(run("puts [string insert abc 1 XY]").2, "aXYbc\n");
    assert_eq!(run("puts [string insert abc end Z]").2, "abcZ\n");
}

/// `string cat` and the `string trim` family, served by the shared core.
#[test]
fn string_cat_and_trim() {
    assert_eq!(run("puts [string cat foo bar baz]").2, "foobarbaz\n");
    assert_eq!(run("puts \"[string trim {  hi  }]<\"").2, "hi<\n");
    assert_eq!(run("puts \"[string trimleft xxabc x]<\"").2, "abc<\n");
    assert_eq!(run("puts \"[string trimright abcxx x]<\"").2, "abc<\n");
}

/// `string is` classification, served by the shared core (`string_is`).
#[test]
fn string_is_classes() {
    assert_eq!(run("puts [string is alpha abc]").2, "1\n");
    assert_eq!(run("puts [string is alpha abc1]").2, "0\n");
    assert_eq!(run("puts [string is integer 123]").2, "1\n");
    assert_eq!(run("puts [string is integer 12x]").2, "0\n");
    assert_eq!(run("puts [string is double 1.5]").2, "1\n");
    assert_eq!(run("puts [string is boolean yes]").2, "1\n");
    assert_eq!(run("puts [string is list {a b c}]").2, "1\n");
    // -failindex reports the first failing character.
    assert_eq!(run("string is alpha -failindex fi abc5\nset fi").1, "3");
}

/// `format` rendering, served by the shared core over `ValueOps`.
#[test]
fn format_core() {
    assert_eq!(run("puts [format %d-%s 5 hi]").2, "5-hi\n");
    assert_eq!(run("puts [format %05d 42]").2, "00042\n");
    assert_eq!(run("puts [format %05d -42]").2, "-0042\n");
    assert_eq!(run("puts [format %+d 42]").2, "+42\n");
    assert_eq!(run("puts \"[format %-5d 42]|\"").2, "42   |\n");
    assert_eq!(run("puts [format %#x 255]").2, "0xff\n");
    assert_eq!(run("puts [format %c 65]").2, "A\n");
    assert_eq!(run("puts [format %.2f 1.23456]").2, "1.23\n");
    assert_eq!(run("puts [format %.3s hello]").2, "hel\n");
    assert_eq!(run("puts [format 100%%]").2, "100%\n");
}

/// `string first` / `string last` (character-indexed substring search).
#[test]
fn string_first_last() {
    assert_eq!(run("puts [string first bc abcbc]").2, "1\n");
    assert_eq!(run("puts [string first bc abcbc 2]").2, "3\n");
    assert_eq!(run("puts [string last bc abcbc]").2, "3\n");
    assert_eq!(run("puts [string first zz abc]").2, "-1\n");
}

/// `STR_CLASS` (operand = class id) reports whether every character of the
/// popped string belongs to the class; the empty string matches.
#[test]
fn opcode_str_class() {
    use tcl_bytecode::{Instruction, Op, Operand};
    let asm = |lit: &str, class: &str| {
        let id = i32::from(tcl_bytecode::str_class_id(class).unwrap());
        run_asm(
            &[lit],
            vec![
                pushv(0),
                Instruction::new(Op::STR_CLASS, vec![Operand::Imm(id)]),
                Instruction::new(Op::DONE, vec![]),
            ],
        )
    };
    assert_eq!(asm("abc", "alpha"), "1");
    assert_eq!(asm("ab2", "alpha"), "0");
    assert_eq!(asm("12345", "digit"), "1");
    assert_eq!(asm("12x45", "digit"), "0");
    assert_eq!(asm("ABC", "upper"), "1");
    assert_eq!(asm("DeadBEEFG", "xdigit"), "0"); // G is not a hex digit
    assert_eq!(asm("dEadBEEF", "xdigit"), "1");
    assert_eq!(asm("", "alpha"), "1"); // empty string matches
}

/// `REGEXP` (operand = cflags) reports whether the pattern (under top) matches
/// the subject (top); the `TCL_REG_NOCASE` (8) bit makes it case-insensitive.
#[test]
fn opcode_regexp() {
    use tcl_bytecode::{Instruction, Op, Operand};
    let asm = |pat: &str, subj: &str, cflags: i32| {
        run_asm(
            &[pat, subj],
            vec![
                pushv(0), // pattern (under top)
                pushv(1), // subject (top)
                Instruction::new(Op::REGEXP, vec![Operand::Imm(cflags)]),
                Instruction::new(Op::DONE, vec![]),
            ],
        )
    };
    // TCL_REG_ADVANCED = 3 (the codegen's plain-form cflags).
    assert_eq!(asm("^[0-9]+$", "12345", 3), "1");
    assert_eq!(asm("^[0-9]+$", "12a45", 3), "0");
    assert_eq!(asm("ab+c", "xxabbbcyy", 3), "1");
    assert_eq!(asm("ABC", "abc", 3), "0");
    assert_eq!(asm("ABC", "abc", 3 | 0o10), "1"); // TCL_REG_NOCASE
}

/// `string is boolean` in value position compiles to C's `INST_TRY_CVT_TO_BOOLEAN`
/// sequence (`tryCvtToBoolean; jumpTrue L; push ""; streq; jump end; L: pop;
/// push "1"`), which only balances because the opcode leaves the value in place
/// and pushes the 0/1 flag on top (`tclExecute.c:6404-6414`, table
/// `tclCompile.c:616`: stack effect **+1**, "No errors"). While the arm
/// pop/re-pushed the *value* and errored on a non-boolean, the first two cases
/// raised `expected boolean value but got "foo"` and the third silently lost the
/// `set` (the codegen's `pop` ate `storeStk`'s operand instead of the value).
#[test]
fn string_is_boolean_inline_try_cvt_to_boolean() {
    assert_eq!(run("set x [string is boolean foo]").1, "0");
    assert_eq!(run("set x [string is boolean {}]").1, "1");
    // The stack-corruption case: `$x` has to be readable afterwards.
    assert_eq!(run("set x [string is boolean yes]; set x").1, "1");
    assert_eq!(run("puts [list a [string is boolean yes] b]").2, "a 1 b\n");
    // The flag is C's *strict* acceptor (`ParseBoolean`, `tclObj.c:2100-2280`),
    // so a truthy non-boolean number is not a boolean — matching the
    // uncompiled `string is boolean` command.
    assert_eq!(run("set x [string is boolean 2]").1, "0");
    assert_eq!(run("set x [string is boolean 1]").1, "1");
    assert_eq!(
        run("string is boolean 2").1,
        run("set x [string is boolean 2]").1
    );
}

/// `string index`/`string range` with a non-constant index compile to
/// `STR_INDEX`/`STR_RANGE`, whose indices go through `TclGetIntForIndexM` with a
/// parse failure as `goto gotError` (`tclExecute.c:5348-5351`, `5388-5397`).
/// Clamping a bad spec to 0/-1 returned a plausible substring instead.
#[test]
fn compiled_str_index_and_range_reject_bad_indices() {
    let bad = |spec: &str| {
        format!("bad index \"{spec}\": must be integer?[+-]integer? or end?[+-]integer?")
    };
    for (src, spec) in [
        ("set i foo; set r [string index abc $i]", "foo"),
        ("set i {1 0}; set r [string index abc $i]", "1 0"),
        ("set i foo; set r [string range abcde $i 2]", "foo"),
        ("set i foo; set r [string range abcde 1 $i]", "foo"),
    ] {
        let (ok, msg, _) = run(src);
        assert!(!ok, "{src} should error");
        assert_eq!(msg, bad(spec), "{src}");
    }
    // Valid indices — including out-of-range ones — keep C's clamp/empty result.
    assert_eq!(run("set i 1; set r [string index abc $i]").1, "b");
    assert_eq!(run("set i end; set r [string index abc $i]").1, "c");
    assert_eq!(run("set i 9; set r [string index abc $i]").1, "");
    assert_eq!(run("set i -1; set r [string index abc $i]").1, "");
    assert_eq!(run("set i 1; set r [string range abcde $i 3]").1, "bcd");
    assert_eq!(run("set i -3; set r [string range abcde $i 2]").1, "abc");
    assert_eq!(run("set i 3; set r [string range abcde $i 1]").1, "");
}

/// `INVOKE_REPLACE` (ensemble-rewrite invoke): `[string equal -nocase …]` /
/// `[string compare -length …]` in value position inline to the resolved
/// implementation (`::tcl::string::equal …`) via the opcode. This is sensitive to
/// inline `set x [cmd]` routing the flagged forms through the
/// (otherwise unimplemented) opcode; verified against tclsh 9.0.
#[test]
fn invoke_replace_string_ensemble() {
    assert_eq!(
        run("set x [string equal -nocase ABC abc]; puts $x").2,
        "1\n"
    );
    assert_eq!(run("set x [string equal abc abd]; puts $x").2, "0\n");
    assert_eq!(
        run("proc f {} { set x [string compare -nocase A a]; return $x }; puts [f]").2,
        "0\n"
    );
    assert_eq!(
        run("set x [string equal -length 2 abcd abXX]; puts $x").2,
        "1\n"
    );
}

/// `encoding` — matches the tree-walking runtime (`runtime/rust`): UTF-8
/// internal model, so `convertto`/`convertfrom` pass data
/// through, `system` retains the mutable channel default, `names` lists the
/// supported set, and `dirs` is ignored. (Additional codepages remain
/// unimplemented on both sides.)
#[test]
fn encoding_command() {
    // The unset default follows the host locale (`iso8859-1` under C/POSIX), so
    // the fixture pins the system encoding itself.
    assert_eq!(run("encoding system utf-8; encoding system").1, "utf-8");
    assert_eq!(
        run("encoding system utf-8; \
             list [encoding system] [encoding system iso8859-1] [encoding system]")
        .1,
        "utf-8 {} iso8859-1"
    );
    assert_eq!(
        run("encoding system ascii; encoding system {}; encoding system").1,
        "iso8859-1"
    );
    assert_eq!(run("encoding names").1, "utf-8 unicode ascii iso8859-1");
    assert_eq!(run("encoding convertto utf-8 abc").1, "abc");
    assert_eq!(run("encoding convertfrom utf-8 hello").1, "hello");
    assert_eq!(run("encoding convertto abc").1, "abc"); // no encoding arg
    assert_eq!(run("encoding dirs").1, "");
    let (ok, msg, _) = run("encoding bogus");
    assert!(!ok);
    assert_eq!(
        msg,
        // tclsh 9.0.4 lists TIP 656's profiles and user.
        "unknown or ambiguous subcommand \"bogus\": must be convertfrom, convertto, dirs, names, profiles, system, or user"
    );
    let (ok, msg, _) = run("encoding");
    assert!(!ok);
    assert_eq!(
        msg,
        "wrong # args: should be \"encoding subcommand ?arg ...?\""
    );
}
