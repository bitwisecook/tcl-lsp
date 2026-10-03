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

//! The runtime's C API exports called as C calls them, for what the shared
//! extension (`tests/pkga_extension.rs`) does not reach: a null interpreter,
//! the option lookup's flags and widths, the integer reads at the edges of their
//! ranges, the error state a command leaves when it states no code, the list
//! calls' refusals, the UTF-8 helpers over a NUL and over malformed bytes, and
//! the ownership of an error code.

use core::ffi::{c_char, c_int, c_long, c_void, CStr};

use tcl_runtime::capi::{
    tcl_test_double_free_count, tcl_test_finalize, tcl_test_reset_counters,
    TclHost_AppendResultString, TclHost_SetResultString, Tcl_CreateObjCommand,
    Tcl_GetDoubleFromObj, Tcl_GetIndexFromObjStruct, Tcl_GetIntFromObj, Tcl_GetLongFromObj,
    Tcl_GetString, Tcl_GetWideIntFromObj, Tcl_ListObjAppendElement, Tcl_ListObjGetElements,
    Tcl_ListObjLength, Tcl_NewIntObj, Tcl_NewListObj, Tcl_NewLongObj, Tcl_NewStringObj,
    Tcl_NumUtfChars, Tcl_PkgProvideEx, Tcl_ResetResult, Tcl_SetObjErrorCode, Tcl_SetObjResult,
    Tcl_UtfNcmp, Tcl_WrongNumArgs,
};
use tcl_runtime::interp::{Code, Interp};
use tcl_runtime::obj::{self, TclObj};

const TCL_OK: c_int = 0;
const TCL_ERROR: c_int = 1;
const TCL_EXACT: c_int = 1;
const TCL_NULL_OK: c_int = 32;
const TCL_INDEX_TEMP_TABLE: c_int = 64;

/// A fresh string object holding `text`.
fn string(text: &str) -> *mut TclObj {
    // SAFETY: `text` addresses `len` readable bytes.
    unsafe { Tcl_NewStringObj(text.as_ptr().cast::<c_char>(), text.len() as isize) }
}

/// An object's text.
fn text(object: *mut TclObj) -> String {
    // SAFETY: the object is live.
    unsafe { CStr::from_ptr(Tcl_GetString(object)) }
        .to_string_lossy()
        .into_owned()
}

/// Hold `object` for the length of `body`, then let it go.
fn held<T>(object: *mut TclObj, body: impl FnOnce(*mut TclObj) -> T) -> T {
    // SAFETY: the object is live; the hold and the release balance.
    unsafe { obj::incr_ref_count(object) };
    let answer = body(object);
    // SAFETY: as above.
    unsafe { obj::decr_ref_count(object) };
    answer
}

fn run(interp: &mut Interp, script: &str) -> (Code, String) {
    let code = interp.eval_str(script.as_bytes());
    (
        code,
        String::from_utf8_lossy(&interp.result_bytes()).into_owned(),
    )
}

fn result(interp: &Interp) -> String {
    String::from_utf8_lossy(&interp.result_bytes()).into_owned()
}

type Proc = unsafe extern "C" fn(*mut c_void, *mut Interp, c_int, *const *mut TclObj) -> c_int;

fn register(interp: &mut Interp, name: &CStr, proc_: Proc) {
    // SAFETY: the interpreter and the name are live; the procedure is a valid
    // `Tcl_ObjCmdProc`.
    let token = unsafe {
        Tcl_CreateObjCommand(
            std::ptr::from_mut(interp),
            name.as_ptr(),
            Some(proc_),
            std::ptr::null_mut(),
            None,
        )
    };
    assert!(!token.is_null());
}

/// `say message` — an error with `message` and no code stated.
unsafe extern "C" fn say(
    _client_data: *mut c_void,
    interp: *mut Interp,
    _objc: c_int,
    objv: *const *mut TclObj,
) -> c_int {
    // SAFETY: the runtime passes a live interpreter and the call's words.
    unsafe { TclHost_SetResultString(interp, Tcl_GetString(*objv.add(1))) };
    TCL_ERROR
}

/// `coded code message` — an error with the code its first argument states.
unsafe extern "C" fn coded(
    _client_data: *mut c_void,
    interp: *mut Interp,
    _objc: c_int,
    objv: *const *mut TclObj,
) -> c_int {
    // SAFETY: as above.
    unsafe {
        Tcl_SetObjErrorCode(interp, *objv.add(1));
        TclHost_SetResultString(interp, Tcl_GetString(*objv.add(2)));
    }
    TCL_ERROR
}

/// `stated code` — states `code` and succeeds.
unsafe extern "C" fn stated(
    _client_data: *mut c_void,
    interp: *mut Interp,
    _objc: c_int,
    objv: *const *mut TclObj,
) -> c_int {
    // SAFETY: as above.
    unsafe { Tcl_SetObjErrorCode(interp, *objv.add(1)) };
    TCL_OK
}

/// `provide name version` — `Tcl_PkgProvide` from a command, answering its code.
unsafe extern "C" fn provide(
    _client_data: *mut c_void,
    interp: *mut Interp,
    _objc: c_int,
    objv: *const *mut TclObj,
) -> c_int {
    // SAFETY: as above.
    unsafe {
        Tcl_PkgProvideEx(
            interp,
            Tcl_GetString(*objv.add(1)),
            Tcl_GetString(*objv.add(2)),
            std::ptr::null(),
        )
    }
}

/// `reset` — states a code and a result, resets them, and fails with no code.
unsafe extern "C" fn reset(
    _client_data: *mut c_void,
    interp: *mut Interp,
    _objc: c_int,
    _objv: *const *mut TclObj,
) -> c_int {
    // SAFETY: as above.
    unsafe {
        Tcl_SetObjErrorCode(interp, string("STALE"));
        TclHost_SetResultString(interp, c"stale".as_ptr());
        Tcl_ResetResult(interp);
        TclHost_AppendResultString(interp, c"after".as_ptr());
        TclHost_AppendResultString(interp, std::ptr::null());
        TclHost_AppendResultString(interp, c" reset".as_ptr());
    }
    TCL_ERROR
}

/// `grown kind word` — sets a result the interpreter cannot grow in place (an
/// integer object for `int`, the call's own word, which the caller holds, for
/// `word`) and appends to it.
unsafe extern "C" fn grown(
    _client_data: *mut c_void,
    interp: *mut Interp,
    _objc: c_int,
    objv: *const *mut TclObj,
) -> c_int {
    // SAFETY: as above.
    unsafe {
        if text(*objv.add(1)) == "int" {
            Tcl_SetObjResult(interp, Tcl_NewIntObj(5));
        } else {
            Tcl_SetObjResult(interp, *objv.add(2));
        }
        TclHost_AppendResultString(interp, c"!".as_ptr());
    }
    TCL_OK
}

/// `read kind value` — reads `value` through `Tcl_Get<kind>FromObj` and
/// answers what it wrote, or fails with the error the read left.
unsafe extern "C" fn read(
    _client_data: *mut c_void,
    interp: *mut Interp,
    _objc: c_int,
    objv: *const *mut TclObj,
) -> c_int {
    // SAFETY: as above; each output is a local of the width the call writes.
    unsafe {
        let (kind, value) = (text(*objv.add(1)), *objv.add(2));
        let answer = match kind.as_str() {
            "int" => {
                let mut out: c_int = 0;
                (
                    Tcl_GetIntFromObj(interp, value, &raw mut out),
                    out.to_string(),
                )
            }
            "long" => {
                let mut out: c_long = 0;
                (
                    Tcl_GetLongFromObj(interp, value, &raw mut out),
                    out.to_string(),
                )
            }
            "wide" => {
                let mut out: i64 = 0;
                (
                    Tcl_GetWideIntFromObj(interp, value, &raw mut out),
                    out.to_string(),
                )
            }
            _ => {
                let mut out = 0.0_f64;
                (
                    Tcl_GetDoubleFromObj(interp, value, &raw mut out),
                    out.to_string(),
                )
            }
        };
        if answer.0 == TCL_OK {
            TclHost_SetResultString(interp, std::ffi::CString::new(answer.1).unwrap().as_ptr());
        }
        answer.0
    }
}

#[test]
fn an_error_a_command_states_no_code_for_is_none_whatever_came_before() {
    let mut interp = Interp::new();
    register(&mut interp, c"say", say);
    register(&mut interp, c"coded", coded);
    register(&mut interp, c"reset", reset);
    register(&mut interp, c"stated", stated);
    register(&mut interp, c"provide", provide);
    assert_eq!(
        run(
            &mut interp,
            "list [catch {coded {MY CODE} first} m] $m $::errorCode"
        ),
        (Code::Ok, "1 first {MY CODE}".to_owned())
    );
    assert_eq!(
        run(&mut interp, "list [catch {say second} m] $m $::errorCode"),
        (Code::Ok, "1 second NONE".to_owned()),
        "an error before it states nothing about this one"
    );
    assert_eq!(
        run(
            &mut interp,
            "catch {error x {} {OLD CODE}}; list [catch {say third} m o] [dict get $o -errorcode]"
        ),
        (Code::Ok, "1 NONE".to_owned())
    );
    assert_eq!(
        run(&mut interp, "list [catch {reset} m] $m $::errorCode"),
        (Code::Ok, "1 {after reset} NONE".to_owned()),
        "`Tcl_ResetResult` takes back the result and the code stated before it"
    );
    assert_eq!(
        run(
            &mut interp,
            "stated {LEFT OVER}; list [catch {say fourth} m] $m $::errorCode"
        ),
        (Code::Ok, "1 fourth NONE".to_owned()),
        "a code a command stated and did not fail with is not the next error's"
    );
    assert_eq!(
        run(
            &mut interp,
            "package provide p 1.0; list [catch {provide p 2.0} m] $m $::errorCode"
        ),
        (
            Code::Ok,
            "1 {conflicting versions provided for package \"p\": 1.0, then 2.0} {TCL PACKAGE VERSIONCONFLICT}"
                .to_owned()
        ),
        "a package call's refusal keeps the package command's code"
    );
}

#[test]
fn an_appended_result_that_is_not_the_interpreter_s_own_string_is_copied() {
    let mut interp = Interp::new();
    register(&mut interp, c"grown", grown);
    assert_eq!(run(&mut interp, "grown int"), (Code::Ok, "5!".to_owned()));
    assert_eq!(
        run(&mut interp, "set w kept; list [grown word $w] $w"),
        (Code::Ok, "kept! kept".to_owned()),
        "the caller's value is not appended to"
    );
}

#[test]
fn the_reads_take_c_tcl_s_ranges_and_report_its_errors() {
    let mut interp = Interp::new();
    register(&mut interp, c"read", read);
    let mut cases = vec![
        ("int 4294967295", "0 -1"),
        ("int 2147483648", "0 -2147483648"),
        (
            "int 4294967296",
            "1 {integer value too large to represent} {ARITH IOVERFLOW {integer value too large to represent}}",
        ),
        ("int 0x10", "0 16"),
        (
            "wide 99999999999999999999",
            "1 {integer value too large to represent} {ARITH IOVERFLOW {integer value too large to represent}}",
        ),
        (
            "double NaN",
            "1 {floating point value is Not a Number} {TCL VALUE DOUBLE NAN}",
        ),
        ("double 0x10", "0 16"),
    ];
    // A double made by `expr`, which needs the numeric tower.
    #[cfg(have_tommath)]
    cases.push((
        "wide [expr {1.5}]",
        "1 {expected integer but got \"1.5\"} {TCL VALUE INTEGER}",
    ));
    if core::mem::size_of::<c_long>() == 8 {
        cases.push(("int -4294967295", "0 1"));
        cases.push(("long 9223372036854775807", "0 9223372036854775807"));
    } else {
        cases.push((
            "long 4294967296",
            "1 {integer value too large to represent} {ARITH IOVERFLOW {integer value too large to represent}}",
        ));
    }
    for (call, wanted) in cases {
        let probe = format!(
            "set c [catch {{read {call}}} m]; switch -- $c {{0 {{list $c $m}} default {{list $c $m $::errorCode}}}}"
        );
        assert_eq!(
            run(&mut interp, &probe),
            (Code::Ok, wanted.to_owned()),
            "{call}"
        );
    }
}

#[test]
fn a_null_interpreter_is_told_nothing() {
    let mut interp = Interp::new();
    assert_eq!(
        run(&mut interp, "set keep kept"),
        (Code::Ok, "kept".to_owned())
    );
    let mut int: c_int = 7;
    held(string("x"), |word| {
        // SAFETY: a null interpreter is the case under test; the rest is live.
        let code = unsafe { Tcl_GetIntFromObj(std::ptr::null_mut(), word, &raw mut int) };
        assert_eq!(code, TCL_ERROR);
    });
    assert_eq!(int, 7, "nothing is written on a failure");
    assert_eq!(result(&interp), "kept", "and no interpreter's result moved");
    assert_eq!(text(Tcl_NewIntObj(-5)), "-5");
    assert_eq!(text(Tcl_NewLongObj(c_long::from(i32::MAX))), "2147483647");
}

/// A table of option names, as C writes one: pointers to static strings, the
/// last NULL.
struct Table([*const c_char; 4]);
// SAFETY: the entries point at static, immutable strings.
unsafe impl Sync for Table {}
static NAMES: Table = Table([
    c"add".as_ptr(),
    c"apply".as_ptr(),
    c"sub".as_ptr(),
    std::ptr::null(),
]);

/// Look `word` up in [`NAMES`] with `flags`, writing the index `width` bytes
/// wide as the header's macro encodes it; answers the code and the index.
fn lookup(interp: &mut Interp, word: *mut TclObj, flags: c_int, width: usize) -> (c_int, i64) {
    let mut slot = [0xFF_u8; 8];
    let encoded = flags | c_int::try_from(width << 1).expect("a width");
    // SAFETY: the table is NULL-terminated with pointer-sized entries, the
    // message terminated, and the slot writable at every width.
    let code = unsafe {
        Tcl_GetIndexFromObjStruct(
            interp,
            word,
            NAMES.0.as_ptr().cast::<c_void>(),
            core::mem::size_of::<*const c_char>() as isize,
            c"option".as_ptr(),
            encoded,
            slot.as_mut_ptr().cast::<c_void>(),
        )
    };
    let index = match width {
        1 => i64::from(slot[0]),
        2 => i64::from(u16::from_ne_bytes([slot[0], slot[1]])),
        4 => i64::from(i32::from_ne_bytes([slot[0], slot[1], slot[2], slot[3]])),
        _ => i64::from_ne_bytes(slot),
    };
    (code, index)
}

#[test]
fn an_option_lookup_follows_its_flags_and_writes_the_width_it_was_given() {
    let mut interp = Interp::new();
    for width in [1, 2, 4, 8] {
        held(string("su"), |word| {
            assert_eq!(
                lookup(&mut interp, word, 0, width),
                (TCL_OK, 2),
                "width {width}"
            );
        });
    }
    held(string("a"), |word| {
        assert_eq!(lookup(&mut interp, word, 0, 4).0, TCL_ERROR);
    });
    assert_eq!(
        result(&interp),
        "ambiguous option \"a\": must be add, apply, or sub"
    );
    held(string("ad"), |word| {
        assert_eq!(lookup(&mut interp, word, TCL_EXACT, 4).0, TCL_ERROR);
    });
    assert_eq!(
        result(&interp),
        "bad option \"ad\": must be add, apply, or sub",
        "an exact lookup takes no abbreviation"
    );
    held(string(""), |word| {
        assert_eq!(lookup(&mut interp, word, TCL_NULL_OK, 4), (TCL_OK, -1));
    });
    held(string("zz"), |word| {
        assert_eq!(lookup(&mut interp, word, TCL_NULL_OK, 4).0, TCL_ERROR);
    });
    assert_eq!(
        result(&interp),
        "bad option \"zz\": must be add, apply, sub, or \"\""
    );
    held(string("add"), |word| {
        // SAFETY: an offset shorter than a pointer is the case under test.
        let code = unsafe {
            Tcl_GetIndexFromObjStruct(
                &mut interp,
                word,
                NAMES.0.as_ptr().cast::<c_void>(),
                1,
                c"option".as_ptr(),
                0,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(code, TCL_ERROR);
    });
    assert_eq!(result(&interp), "Invalid struct offset value 1.");
}

#[test]
fn wrong_num_args_quotes_each_word_and_spells_a_resolved_one_in_full() {
    let mut interp = Interp::new();
    let words = [string("cmd"), string("su"), string("a b"), string("ap")];
    for &word in &words {
        // SAFETY: each object is live and now held here.
        unsafe { obj::incr_ref_count(word) };
    }
    assert_eq!(lookup(&mut interp, words[1], 0, 4), (TCL_OK, 2));
    assert_eq!(
        lookup(&mut interp, words[3], TCL_INDEX_TEMP_TABLE, 4),
        (TCL_OK, 1)
    );
    // SAFETY: the interpreter and the words are live.
    unsafe { Tcl_WrongNumArgs(&mut interp, 4, words.as_ptr(), c"value".as_ptr()) };
    assert_eq!(
        result(&interp),
        "wrong # args: should be \"cmd sub {a b} ap value\"",
        "the word resolved through a temporary table keeps its own spelling"
    );
    // SAFETY: as above, with no message.
    unsafe { Tcl_WrongNumArgs(&mut interp, 1, words.as_ptr(), std::ptr::null()) };
    assert_eq!(result(&interp), "wrong # args: should be \"cmd\"");
    for &word in &words {
        // SAFETY: releases the holds taken above.
        unsafe { obj::decr_ref_count(word) };
    }
}

#[test]
fn the_list_calls_refuse_what_c_tcl_refuses() {
    let mut interp = Interp::new();
    // SAFETY: a fresh, empty list.
    let list = unsafe { Tcl_NewListObj(0, std::ptr::null()) };
    held(list, |list| {
        // SAFETY: the list is unshared (one hold) and the element fresh.
        assert_eq!(
            unsafe { Tcl_ListObjAppendElement(&mut interp, list, string("x")) },
            TCL_OK
        );
        // SAFETY: a second hold makes the list shared, the case under test.
        unsafe { obj::incr_ref_count(list) };
        held(string("y"), |element| {
            // SAFETY: as above.
            let code = unsafe { Tcl_ListObjAppendElement(&mut interp, list, element) };
            assert_eq!(code, TCL_ERROR);
        });
        // SAFETY: releases the second hold.
        unsafe { obj::decr_ref_count(list) };
        let mut length: isize = 0;
        // SAFETY: the list is live and the length writable.
        let code = unsafe { Tcl_ListObjLength(&mut interp, list, &raw mut length) };
        assert_eq!(
            (code, length),
            (TCL_OK, 1),
            "the refused element was not added"
        );
    });
    assert_eq!(
        result(&interp),
        "Tcl_ListObjAppendElement called with shared object"
    );
    held(string("a {b"), |bad| {
        let (mut count, mut elements) = (0_isize, std::ptr::null_mut());
        // SAFETY: the word is live and the outputs writable.
        let code =
            unsafe { Tcl_ListObjGetElements(&mut interp, bad, &raw mut count, &raw mut elements) };
        assert_eq!(code, TCL_ERROR);
    });
    assert_eq!(result(&interp), "unmatched open brace in list");
    held(string("p {q r} s"), |good| {
        let (mut count, mut elements) = (0_isize, std::ptr::null_mut::<*mut TclObj>());
        // SAFETY: as above.
        let code =
            unsafe { Tcl_ListObjGetElements(&mut interp, good, &raw mut count, &raw mut elements) };
        assert_eq!((code, count), (TCL_OK, 3));
        // SAFETY: the array holds `count` elements the list owns.
        assert_eq!(text(unsafe { *elements.add(1) }), "q r");
    });
}

/// What `Tcl_NumUtfChars` answers in C Tcl 9.0.4 for well-formed, modified
/// (`C0 80`) and malformed UTF-8: the bytes, the length given, the count. Each
/// answer was taken from a program linked with C Tcl's own library.
const COUNTS: &[(&[u8], isize, isize)] = &[
    (b"hello", -1, 5),
    (b"h\xc3\xa9llo", 6, 5),
    (b"a\xc0\x80b", 4, 3),
    (b"a\xc0\x80b", -1, 3),
    (b"\xc1\x80", 2, 2),
    (b"\x80a", 2, 2),
    (b"\x80a", -1, 2),
    (b"\xe0\x80\x41", 3, 3),
    (b"\xe0\xa0\x80", 3, 1),
    (b"\xe0\x80\x80", 3, 3),
    (b"\xf0\x90\x80\x80", 4, 1),
    (b"\xf4\x90\x80\x80", 4, 4),
    (b"\xf0\x80\x80\x80", 4, 4),
    (b"ab\xc3", 3, 3),
    (b"a\xe2\x82", 3, 3),
    (b"\xff\xfe", 2, 2),
    (b"ab\x80", 3, 3),
    (b"a\xe2\x82\xac", 3, 3),
    (b"\xf0\x9f\x98\x80", 3, 3),
    (b"a\xe2\x82\xac", 4, 2),
];

/// What `Tcl_UtfNcmp` answers in C Tcl 9.0.4, taken the same way: the two
/// strings, the characters compared, the answer.
const COMPARISONS: &[(&[u8], &[u8], usize, c_int)] = &[
    (b"\x80", b"\x81", 1, 8235),
    (b"\x9f", b"\xa0", 1, 216),
    (b"\xc0\x80", b"\x01", 1, -1),
    (b"\xc0\x80", b"", 1, 0),
    (b"\xc1\x80", b"\xc1a", 2, 8267),
    (b"\xc3\xa9", b"e", 1, 132),
    (b"\xf0\x9f\x98\x80", b"\xf0\x9f\x98\x81", 1, -1),
    (b"\xe0\x80\x41", b"\xe0\x81\x41", 3, 8235),
];

/// `bytes` with C's terminator after them.
fn terminated(bytes: &[u8]) -> Vec<u8> {
    let mut text = bytes.to_vec();
    text.push(0);
    text
}

#[test]
fn the_utf8_helpers_read_characters_and_a_nul_is_one() {
    let (with, without) = (b"a\0b\0", b"a\0c\0");
    let (with, without) = (
        with.as_ptr().cast::<c_char>(),
        without.as_ptr().cast::<c_char>(),
    );
    // SAFETY: each buffer holds the characters read, and the literals are
    // terminated.
    unsafe {
        assert_eq!(Tcl_NumUtfChars(c"héllo".as_ptr(), 6), 5);
        assert_eq!(Tcl_NumUtfChars(c"héllo".as_ptr(), -1), 5);
        assert_eq!(Tcl_NumUtfChars(with, 3), 3);
        assert!(Tcl_UtfNcmp(with, without, 3) < 0, "the NUL is passed over");
        assert_eq!(Tcl_UtfNcmp(with, with, 3), 0);
        assert_eq!(Tcl_UtfNcmp(c"é".as_ptr(), c"e".as_ptr(), 1), 0xE9 - 0x65);
        assert!(Tcl_UtfNcmp(c"ab".as_ptr(), c"abc".as_ptr(), 3) < 0);
    }
}

#[test]
fn the_utf8_helpers_read_malformed_bytes_as_c_tcl_reads_them() {
    for &(bytes, length, expected) in COUNTS {
        let text = terminated(bytes);
        // SAFETY: the buffer holds `length` bytes and is terminated.
        let count = unsafe { Tcl_NumUtfChars(text.as_ptr().cast(), length) };
        assert_eq!(count, expected, "Tcl_NumUtfChars({bytes:x?}, {length})");
    }
    for &(left, right, characters, expected) in COMPARISONS {
        let (one, two) = (terminated(left), terminated(right));
        // SAFETY: each buffer holds the characters compared and is terminated.
        let answer = unsafe { Tcl_UtfNcmp(one.as_ptr().cast(), two.as_ptr().cast(), characters) };
        assert_eq!(
            answer, expected,
            "Tcl_UtfNcmp({left:x?}, {right:x?}, {characters})"
        );
    }
}

#[test]
fn an_error_code_object_is_released_once_it_is_stated() {
    tcl_test_reset_counters();
    {
        let mut interp = Interp::new();
        // SAFETY: a fresh list the call is the only taker of.
        unsafe {
            let code = Tcl_NewListObj(0, std::ptr::null());
            assert_eq!(
                Tcl_ListObjAppendElement(&mut interp, code, string("A")),
                TCL_OK
            );
            Tcl_SetObjErrorCode(&mut interp, code);
        }
    }
    assert_eq!(tcl_test_finalize(), 0, "the fresh code was freed");
    assert_eq!(tcl_test_double_free_count(), 0);
}
