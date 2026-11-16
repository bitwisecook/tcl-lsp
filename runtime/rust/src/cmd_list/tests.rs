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

use crate::counters;
use crate::interp::{Code, Interp};

fn leak_free<R>(body: impl FnOnce(&mut Interp) -> R) -> R {
    counters::reset();
    let result = {
        let mut i = Interp::new();
        body(&mut i)
    };
    assert_eq!(
        counters::finalize(),
        0,
        "leak: {} objs {} bufs",
        counters::live_objs(),
        counters::live_bufs()
    );
    assert_eq!(counters::double_free_count(), 0);
    result
}

fn run(src: &[u8]) -> (Code, Vec<u8>) {
    leak_free(|interp| {
        let code = interp.eval_str(src);
        let result = interp.result_bytes();
        if std::env::var_os("TCL_LSP_RUNTIME_BASELINE_DIAGNOSTIC").as_deref()
            == Some(std::ffi::OsStr::new("1"))
        {
            eprintln!(
                    "LIST_TEST_DIAGNOSTIC code={code:?} source_prefix={:?} result_prefix={:?} refusal={:?}",
                    &src[..src.len().min(256)],
                    &result[..result.len().min(256)],
                    interp.native_execution_refusal(),
                );
        }
        (code, result)
    })
}

fn ok(src: &[u8]) -> Vec<u8> {
    let (c, b) = run(src);
    assert_eq!(c, Code::Ok, "result={:?}", String::from_utf8_lossy(&b));
    b
}

// Needs the numeric tower: `-command` comparators are `expr` lambdas.
#[cfg(have_tommath)]
#[test]
fn lsort_shared_core() {
    // Pinned against tclsh 9.0 (each case leak-checked by `ok`/`run`).
    assert_eq!(ok(b"lsort {b a c}"), b"a b c");
    assert_eq!(ok(b"lsort -decreasing {b a c}"), b"c b a");
    assert_eq!(ok(b"lsort -integer {10 2 33 4}"), b"2 4 10 33");
    assert_eq!(ok(b"lsort -real {1.5 0.5 2.25}"), b"0.5 1.5 2.25");
    assert_eq!(ok(b"lsort -nocase {B a C b}"), b"a B b C");
    assert_eq!(ok(b"lsort -dictionary {x10 x9 x100}"), b"x9 x10 x100");
    assert_eq!(ok(b"lsort -unique {a b a c b}"), b"a b c");
    assert_eq!(ok(b"lsort -integer -unique {1 01 1 2}"), b"1 2");
    assert_eq!(ok(b"lsort -indices {c a b}"), b"1 2 0");
    assert_eq!(
        ok(b"lsort -index 1 {{a 3} {b 1} {c 2}}"),
        b"{b 1} {c 2} {a 3}"
    );
    assert_eq!(ok(b"lsort -index 0 {{b 1} {a 2}}"), b"{a 2} {b 1}");
    assert_eq!(ok(b"lsort -stride 2 {c 3 a 1 b 2}"), b"a 1 b 2 c 3");
    assert_eq!(
        ok(b"lsort -stride 2 -index 1 {x 3 y 1 z 2}"),
        b"y 1 z 2 x 3"
    );
    assert_eq!(
        ok(b"lsort -stride 2 -indices {c 3 a 1 b 2}"),
        b"2 3 4 5 0 1"
    );
    assert_eq!(
        ok(b"lsort -decreasing -dictionary {x9 x10 x100}"),
        b"x100 x10 x9"
    );
    // `-command` (Family-B: the comparator evaluates Tcl).
    assert_eq!(
        ok(b"lsort -command {apply {{a b} {expr {$a - $b}}}} {3 1 2}"),
        b"1 2 3"
    );
    assert_eq!(
        ok(b"lsort -command {apply {{a b} {expr {$b - $a}}}} {3 1 2}"),
        b"3 2 1"
    );
    assert_eq!(
        ok(b"lsort -unique -command {apply {{a b} {expr {$a - $b}}}} {3 1 3 2 1}"),
        b"1 2 3"
    );
    assert_eq!(ok(b"lsort {}"), b"");
    // Errors.
    let (c, b) = run(b"lsort -bogus {a b}");
    assert_eq!(c, Code::Error);
    assert!(b.starts_with(b"bad option \"-bogus\""));
    let (c, b) = run(b"lsort -integer {1 x 3}");
    assert_eq!(c, Code::Error);
    assert_eq!(b, b"expected integer but got \"x\"");
    let (c, b) = run(b"lsort -index 5 {{a b} {c d}}");
    assert_eq!(c, Code::Error);
    assert_eq!(b, b"element 5 missing from sublist \"a b\"");
    let (c, b) = run(b"lsort -stride 3 {a b}");
    assert_eq!(c, Code::Error);
    assert_eq!(b, b"list size must be a multiple of the stride length");
}

#[test]
fn lsearch_shared_core() {
    // Pinned against tclsh 9.0 (every option exercised; each case is also
    // leak-checked by `ok`/`run`).
    assert_eq!(ok(b"lsearch {a b c d} c"), b"2");
    assert_eq!(ok(b"lsearch {a b c d} x"), b"-1");
    assert_eq!(ok(b"lsearch -exact {aa ab ac} ab"), b"1");
    assert_eq!(ok(b"lsearch -all {a b a c a} a"), b"0 2 4");
    assert_eq!(ok(b"lsearch -inline {foo bar baz} ba*"), b"bar");
    assert_eq!(ok(b"lsearch -all -inline {x1 y2 x3} x*"), b"x1 x3");
    assert_eq!(ok(b"lsearch -not {a b a} a"), b"1");
    assert_eq!(ok(b"lsearch -all -not {a b a c} a"), b"1 3");
    assert_eq!(ok(b"lsearch -start 2 {a b a a} a"), b"2");
    assert_eq!(ok(b"lsearch -nocase {AB cd EF} ef"), b"2");
    assert_eq!(ok(b"lsearch -integer {3 1 4 1 5} 4"), b"2");
    assert_eq!(ok(b"lsearch -real {1.5 2.5 3.5} 2.5"), b"1");
    assert_eq!(ok(b"lsearch -sorted {1 3 5 7 9} 7"), b"3");
    assert_eq!(ok(b"lsearch -sorted -integer {1 3 5 7} 5"), b"2");
    assert_eq!(ok(b"lsearch -sorted -decreasing {9 7 5 3 1} 5"), b"2");
    assert_eq!(ok(b"lsearch -bisect -integer {2 4 6 8} 5"), b"1");
    assert_eq!(ok(br"lsearch -regexp {foo123 bar456} {[0-9]+}"), b"0");
    assert_eq!(
        ok(br"lsearch -all -inline -regexp {a1 b2 c3} {\d}"),
        b"a1 b2 c3"
    );
    assert_eq!(ok(b"lsearch -index 1 {{a 1} {b 2} {c 3}} 2"), b"1");
    assert_eq!(ok(b"lsearch -index 0 -inline {{a 1} {b 2}} b"), b"b 2");
    assert_eq!(ok(b"lsearch -all -index 1 {{a 1} {b 2} {c 1}} 1"), b"0 2");
    assert_eq!(ok(b"lsearch -subindices -index 1 {{a 1} {b 2}} 2"), b"1 1");
    assert_eq!(ok(b"lsearch -stride 2 -index 0 {a 1 b 2 c 3} b"), b"2");
    assert_eq!(ok(b"lsearch -stride 2 {a 1 b 2} 2"), b"-1");
    assert_eq!(
        ok(b"lsearch -all -inline -stride 2 {a 1 b 2 c 3} *"),
        b"a 1 b 2 c 3"
    );
    assert_eq!(ok(b"lsearch -exact -sorted {a b c d} c"), b"2");
    assert_eq!(ok(b"lsearch {} x"), b"-1");
    assert_eq!(ok(b"lsearch -integer {1 x 3} 3"), b"2");
    assert_eq!(ok(b"lsearch -index end {{a b} {c d}} d"), b"1");
    // Errors.
    let (c, b) = run(b"lsearch -bogus {a b} a");
    assert_eq!(c, Code::Error);
    assert!(b.starts_with(b"bad option \"-bogus\""));
    let (c, b) = run(b"lsearch -subindices {a b} a");
    assert_eq!(c, Code::Error);
    assert_eq!(b, b"-subindices cannot be used without -index option");
}

#[test]
fn lrepeat_linsert_lreplace() {
    assert_eq!(ok(b"lrepeat 3 a b"), b"a b a b a b");
    assert_eq!(ok(b"lrepeat 0 a"), b"");
    assert_eq!(ok(b"linsert {a b c} end X"), b"a b c X");
    assert_eq!(ok(b"linsert {a b c} 1 X Y"), b"a X Y b c");
    assert_eq!(ok(b"linsert {a b c} 0 X"), b"X a b c");
    assert_eq!(ok(b"lreplace {a b c d} 1 2 X"), b"a X d");
    assert_eq!(ok(b"lreplace {a b c d} 1 2"), b"a d");
    assert_eq!(ok(b"lreplace {a b c} end end Z"), b"a b Z");
    // `first > last` is a pure insertion at `first`.
    assert_eq!(ok(b"lreplace {a b c} 1 0 X"), b"a X b c");
    // Out-of-range indices clamp (no error); `end`/`end±N` offset correctly:
    // past-end appends, negative prepends.
    assert_eq!(ok(b"linsert {a b c} end+1 X"), b"a b c X");
    assert_eq!(ok(b"linsert {a b c} -5 X"), b"X a b c");
    assert_eq!(ok(b"lreplace {a b c} 5 7 X"), b"a b c X");
    // A malformed index spec errors faithfully (shared index parser).
    assert!(err(b"linsert {a b c} foo X").starts_with(b"bad index"));
    assert!(err(b"lreplace {a b c} foo 1 X").starts_with(b"bad index"));
    assert!(err(b"lreplace {a b c} 1 foo X").starts_with(b"bad index"));
}

fn err(src: &[u8]) -> Vec<u8> {
    let (c, b) = run(src);
    assert_eq!(
        c,
        Code::Error,
        "expected error, got {:?}",
        String::from_utf8_lossy(&b)
    );
    b
}

#[test]
fn native_index_grammar_matches_real_c_and_jim_observations() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        leak_free(|interp| {
            interp.set_dialect_profile(
                tcl_registry::model::ingress::resolve_environment(dialect).unit_profile(),
            );
            let old = dialect == "tcl8.4";
            let c8 = matches!(dialect, "tcl8.4" | "tcl8.5" | "tcl8.6");
            let jim = dialect == "jim";
            for (index, answer) in [
                ("e", c8.then_some("C")),
                ("en", c8.then_some("C")),
                ("end", Some("C")),
                ("1+1", (!old).then_some("C")),
                ("1-1", (!old).then_some("A")),
                ("end+1", (!old).then_some("")),
                ("end- 1", (old || jim).then_some("B")),
                (" end-1 ", None),
                ("2*1", jim.then_some("C")),
                ("(1+1)", jim.then_some("C")),
                ("$n", None),
                ("[set n]", None),
                ("abs(-1)", jim.then_some("B")),
                ("bool(1)", None),
                ("true", None),
                ("1.0", None),
                ("1 +1", jim.then_some("C")),
                ("1+ 1", jim.then_some("C")),
                ("-4294967295", Some(if c8 { "B" } else { "" })),
                ("2147483648", (!jim).then_some("")),
            ] {
                let source = format!("set n 1; string index ABC {{{index}}}");
                let code = interp.eval_str(source.as_bytes());
                assert_eq!(
                    code,
                    if answer.is_some() {
                        Code::Ok
                    } else {
                        Code::Error
                    },
                    "{dialect}: {index}: {:?}",
                    interp.result_bytes()
                );
                if let Some(answer) = answer {
                    assert_eq!(
                        interp.result_bytes(),
                        answer.as_bytes(),
                        "{dialect}: {index}"
                    );
                }
            }
        });
    }
}

#[test]
fn native_concat_preserves_selected_engine_and_object_representation() {
    // Native proof: naming.list.original-jim-source-length-conversion
    // docs/design/analysis/name-resolution-proofs/list-original-jim-source-length-conversion.md
    // These three source channels share one interpreter, as in the native driver.
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        leak_free(|interp| {
            *interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(dialect),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .expect("actual original native object constructor");
            assert_eq!(interp.eval_str(b"concat word {}"), Code::Ok);
            assert_eq!(
                interp.result_bytes(),
                if dialect == "jim" {
                    b"word ".as_slice()
                } else {
                    b"word".as_slice()
                }
            );
            assert_eq!(
                interp.eval_str(b"set first {A  B}; set second C; concat $first $second"),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), b"A  B C");
            let storage = [b"first".as_slice(), b"second".as_slice()].map(|name| {
                interp.var_get(name).map(|value| {
                    (
                        crate::obj::stock_list_input_class(value),
                        crate::obj::native_object_snapshot(value),
                        core::ptr::eq(
                            crate::obj::obj_type_ptr(value),
                            &crate::native_source::JIM_SOURCE_TYPE,
                        ),
                    )
                })
            });
            let code = interp.eval_str(b"llength $first; llength $second; concat $first $second");
            assert_eq!(
                code,
                Code::Ok,
                "{dialect}: result={:?}, access={:?}, original storage={storage:?}",
                interp.result_bytes(),
                interp.native_access_refusal()
            );
            assert_eq!(
                interp.result_bytes(),
                if dialect == "jim" {
                    b"A B C".as_slice()
                } else {
                    b"A  B C".as_slice()
                }
            );
        });
    }
}

#[test]
fn lset_bounds_follow_real_c_releases_and_current_jim() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        leak_free(|interp| {
            interp.set_dialect_profile(
                tcl_registry::model::ingress::resolve_environment(dialect).unit_profile(),
            );
            for (index, expected) in [
                ("0", b"0 {X B}".as_slice()),
                ("1", b"0 {A X}".as_slice()),
                (
                    "2",
                    if matches!(dialect, "tcl8.4" | "tcl8.5" | "jim") {
                        b"1 {A B}".as_slice()
                    } else {
                        b"0 {A B X}".as_slice()
                    },
                ),
                ("3", b"1 {A B}".as_slice()),
                ("-1", b"1 {A B}".as_slice()),
            ] {
                let source =
                    format!("set data {{A B}}; list [catch {{lset data {index} X}}] $data");
                assert_eq!(interp.eval_str(source.as_bytes()), Code::Ok);
                assert_eq!(interp.result_bytes(), expected, "{dialect}, {index}");
            }
        });
    }
}

#[test]
fn lset_sets_and_appends() {
    // Replace, append, end-relative; updates the variable and returns it.
    assert_eq!(ok(b"set x {a b c}; lset x 1 Z"), b"a Z c");
    assert_eq!(ok(b"set x {a b c}; lset x 1 Z; set x"), b"a Z c");
    assert_eq!(ok(b"set x {a b c}; lset x end Z"), b"a b Z");
    assert_eq!(ok(b"set x {a b c}; lset x 3 Z"), b"a b c Z"); // append at len
                                                              // No index → whole-list replace (lset is set).
    assert_eq!(ok(b"set x {a b c}; lset x Z"), b"Z");
    assert_eq!(ok(b"set x {a b}; lset x {} Z"), b"Z");
    // Nested: a lone arg is an index path; multiple args each an index.
    assert_eq!(ok(b"set x {{a b} {c d}}; lset x 1 0 Z"), b"{a b} {Z d}");
    assert_eq!(ok(b"set x {{a b} {c d}}; lset x {1 0} Z"), b"{a b} {Z d}");
    assert_eq!(ok(b"set x {a {b c}}; lset x 1 1 Z"), b"a {b Z}");
    // Empty-list quirks (single-element sublists stringify without braces).
    assert_eq!(ok(b"set x {}; lset x 0 0 Z"), b"Z");
    assert_eq!(ok(b"set x {}; lset x end+1 Z"), b"Z");
    // Array-element addressing, like `ledit`/`lappend`.
    assert_eq!(ok(b"set a(k) {1 2 3}; lset a(k) 1 Z; set a(k)"), b"1 Z 3");
    // COW: a shared value isn't mutated through the alias.
    assert_eq!(
        ok(b"set l {a b c}; set m $l; lset l 0 X; list $l $m"),
        b"{X b c} {a b c}"
    );
}

/// An index equal to a level's length appends from 8.6 and raises before
/// it, as tclsh 8.4.20 to 9.1.0 do (the empty list's first slot, `end+1`
/// and a nested level included).
#[test]
fn lset_appends_at_the_length_only_from_8_6() {
    use tcl_dialect::TclVersion;

    let at = |release: TclVersion, src: &[u8]| {
        counters::reset();
        let (code, bytes);
        {
            let mut i = Interp::new();
            i.set_runtime_version(release);
            code = i.eval_str(src);
            bytes = i.result_bytes();
        }
        assert_eq!(counters::finalize(), 0);
        (code, bytes)
    };
    for release in TclVersion::ALL {
        for (src, appended) in [
            (&b"set x {a b c}; lset x 3 Z"[..], &b"a b c Z"[..]),
            (b"set x {a b c}; lset x {3} Z", b"a b c Z"),
            (b"set x {}; lset x 0 Z", b"Z"),
            (b"set x {{a b c}}; lset x 0 3 Z", b"{a b c Z}"),
            (b"set x {a b c}; lset x end+1 Z", b"a b c Z"),
        ] {
            let (code, bytes) = at(release, src);
            let label = String::from_utf8_lossy(src);
            if release.lset_appends_at_length() {
                assert_eq!(code, Code::Ok, "{release:?} {label}");
                assert_eq!(bytes, appended, "{release:?} {label}");
            } else {
                assert_eq!(code, Code::Error, "{release:?} {label}");
                assert!(
                    bytes == b"list index out of range" || bytes.starts_with(b"bad index"),
                    "{release:?} {label}: {:?}",
                    String::from_utf8_lossy(&bytes)
                );
            }
        }
    }
}

#[test]
fn lset_errors() {
    assert_eq!(
        err(b"set x {a b c}; lset x 5 Z"),
        b"index \"5\" out of range"
    );
    assert_eq!(
        err(b"set x {a b c}; lset x -1 Z"),
        b"index \"-1\" out of range"
    );
    assert_eq!(
        err(b"set x {a b c}; lset x a Z"),
        b"bad index \"a\": must be integer?[+-]integer? or end?[+-]integer?"
    );
    assert_eq!(
        err(b"lset nosuchvar 0 Z"),
        b"can't read \"nosuchvar\": no such variable"
    );
    assert_eq!(
        err(b"lset"),
        b"wrong # args: should be \"lset listVar ?index? ?index ...? value\""
    );
}

#[test]
fn ledit_replaces_in_place() {
    // Returns the new value *and* updates the variable in place.
    assert_eq!(ok(b"set l {1 2 3 4 5}; ledit l 1 1 a"), b"1 a 3 4 5");
    assert_eq!(ok(b"set l {1 2 3 4 5}; ledit l 1 1 a; set l"), b"1 a 3 4 5");
    assert_eq!(ok(b"set l {1 2 3 4 5}; ledit l 1 3; set l"), b"1 5");
    assert_eq!(ok(b"set l {1 2 3}; ledit l 1 0 x y; set l"), b"1 x y 2 3"); // first>last
    assert_eq!(ok(b"set l {a b c d}; ledit l end-1 end Z"), b"a b Z");
    assert_eq!(ok(b"set l {a b}; ledit l end+1 end+1 c"), b"a b c"); // append
                                                                     // Array-element addressing, like `lappend a(k)`.
    assert_eq!(
        ok(b"set a(k) {1 2 3}; ledit a(k) 0 0 X; set a(k)"),
        b"X 2 3"
    );
    // COW: a shared value isn't mutated through the alias.
    assert_eq!(
        ok(b"set l {a b c}; set m $l; ledit l 0 0 X; list $l $m"),
        b"{X b c} {a b c}"
    );
}

#[test]
fn ledit_errors() {
    let (c, b) = run(b"ledit l 0");
    assert_eq!(c, Code::Error);
    assert_eq!(
        b,
        b"wrong # args: should be \"ledit listVar first last ?element ...?\""
    );
    // A wholly missing variable is a read error (C's TCL_LEAVE_ERR_MSG).
    let (c, b) = run(b"ledit nope 0 0 x");
    assert_eq!(c, Code::Error);
    assert_eq!(b, b"can't read \"nope\": no such variable");
    // Missing element of an existing array → "no such element in array".
    let (c, b) = run(b"set arr(y) y; ledit arr(x) 0 0 z");
    assert_eq!(c, Code::Error);
    assert_eq!(b, b"can't read \"arr(x)\": no such element in array");
}

#[test]
fn var_read_miss_three_way() {
    // The C `tclVar.c` distinction, shared by `set`/`ledit`/`expr $var`.
    let (c, b) = run(b"set nope");
    assert_eq!(
        (c, b),
        (
            Code::Error,
            b"can't read \"nope\": no such variable".to_vec()
        )
    );
    let (c, b) = run(b"set arr(y) y; set arr");
    assert_eq!(
        (c, b),
        (
            Code::Error,
            b"can't read \"arr\": variable is array".to_vec()
        )
    );
    let (c, b) = run(b"set arr(y) y; set arr(x)");
    assert_eq!(
        (c, b),
        (
            Code::Error,
            b"can't read \"arr(x)\": no such element in array".to_vec()
        )
    );
}

#[test]
fn lsearch_modes() {
    assert_eq!(ok(b"lsearch {a b c b} b"), b"1");
    assert_eq!(ok(b"lsearch -all {a b c b} b"), b"1 3");
    assert_eq!(ok(b"lsearch {x ab cd} a*"), b"1"); // default glob
    assert_eq!(ok(b"lsearch -exact {x ab cd} ab"), b"1");
    assert_eq!(ok(b"lsearch -inline {one two three} t*"), b"two");
    assert_eq!(ok(b"lsearch {a b c} z"), b"-1");
    // Datatypes, -not, -all -inline.
    assert_eq!(ok(b"lsearch -integer {1 5 3 5} 5"), b"1");
    assert_eq!(ok(b"lsearch -not {a a b a} a"), b"2");
    assert_eq!(ok(b"lsearch -all -inline {a1 b2 a3} a*"), b"a1 a3");
    // -sorted binary search + -bisect.
    assert_eq!(ok(b"lsearch -sorted -integer {1 3 5 7 9} 5"), b"2");
    assert_eq!(ok(b"lsearch -sorted -integer {1 3 5 7 9} 6"), b"-1");
    assert_eq!(ok(b"lsearch -sorted -bisect -integer {1 3 5 7 9} 6"), b"2");
    // -index, -stride, -subindices, -start, -regexp, -dictionary.
    assert_eq!(ok(b"lsearch -index 1 {{a 1} {b 2} {c 3}} 2"), b"1");
    assert_eq!(ok(b"lsearch -stride 2 -index 0 {a 1 b 2 c 3} b"), b"2");
    assert_eq!(ok(b"lsearch -subindices -index 1 {{a 1} {b 2}} 2"), b"1 1");
    assert_eq!(ok(b"lsearch -start 2 {a b a b} a"), b"2");
    assert_eq!(ok(b"lsearch -all -regexp {foo bar baz} {^ba}"), b"1 2");
    assert_eq!(ok(b"lsearch -dictionary -sorted {x1 x9 x10} x9"), b"1");
}

#[test]
fn lsearch_errors() {
    assert_eq!(
        err(b"lsearch -stride 0 {a b} x"),
        b"stride length must be at least 1"
    );
    assert_eq!(
        err(b"lsearch -exact -integer {a b} 1"),
        b"expected integer but got \"a\""
    );
    assert_eq!(
            err(b"lsearch -bogus {a} b"),
            b"bad option \"-bogus\": must be -all, -ascii, -bisect, -decreasing, -dictionary, -exact, -glob, -increasing, -index, -inline, -integer, -nocase, -not, -real, -regexp, -sorted, -start, -stride, or -subindices"
        );
}

// Needs the numeric tower: `-command` comparators are `expr` lambdas.
#[cfg(have_tommath)]
#[test]
fn lsort_options() {
    assert_eq!(ok(b"lsort {c a b}"), b"a b c");
    assert_eq!(ok(b"lsort -decreasing {c a b}"), b"c b a");
    assert_eq!(ok(b"lsort -integer {10 2 33 4}"), b"2 4 10 33");
    assert_eq!(ok(b"lsort -unique {b a a c}"), b"a b c");
    assert_eq!(ok(b"lsort -nocase {B a C}"), b"a B C");
    // -stride groups; the key defaults to the group's first element.
    assert_eq!(ok(b"lsort -stride 2 {c 3 a 1 b 2}"), b"a 1 b 2 c 3");
    assert_eq!(
        ok(b"lsort -stride 2 -index 1 {c 3 a 1 b 2}"),
        b"a 1 b 2 c 3"
    );
    // -index drills into each element.
    assert_eq!(ok(b"lsort -index 0 {{b 2} {a 1}}"), b"{a 1} {b 2}");
    assert_eq!(ok(b"lsort -index 1 {{b 1} {a 2}}"), b"{b 1} {a 2}");
    // -dictionary: embedded numbers compared numerically, case-insensitive.
    assert_eq!(ok(b"lsort -dictionary {x10 x9 x1}"), b"x1 x9 x10");
    // -indices returns positions; -real; -command.
    assert_eq!(ok(b"lsort -indices {c a b}"), b"1 2 0");
    assert_eq!(ok(b"lsort -real {1.5 0.2 3}"), b"0.2 1.5 3");
    assert_eq!(
        ok(b"lsort -command {apply {{a b} {expr {$a - $b}}}} {3 1 2}"),
        b"1 2 3"
    );
}

#[test]
fn lsort_errors() {
    assert_eq!(
        err(b"lsort -stride 1 {a b}"),
        b"stride length must be at least 2"
    );
    assert_eq!(
        err(b"lsort -stride 2 {a b c}"),
        b"list size must be a multiple of the stride length"
    );
    assert_eq!(
        err(b"lsort -integer {a b}"),
        b"expected integer but got \"a\""
    );
    assert_eq!(
        err(b"lsort -real {a b}"),
        b"expected floating-point number but got \"a\""
    );
    assert_eq!(
        err(b"lsort -index 5 {{a b}}"),
        b"element 5 missing from sublist \"a b\""
    );
    assert_eq!(
            err(b"lsort -bogus {a}"),
            b"bad option \"-bogus\": must be -ascii, -command, -decreasing, -dictionary, -increasing, -index, -indices, -integer, -nocase, -real, -stride, or -unique"
        );
}

#[test]
fn list_and_llength() {
    assert_eq!(ok(b"list a b c"), b"a b c");
    assert_eq!(ok(b"llength {a b c d}"), b"4");
    assert_eq!(ok(b"llength {}"), b"0");
    assert_eq!(ok(b"list a {b c} {}"), b"a {b c} {}"); // quoting
}

#[test]
fn string_rep_survives_shimmer() {
    // A string→list shimmer (here via `llength`) keeps the original spelling
    // (irregular spacing), Tcl's dual-rep — not the canonical list form.
    assert_eq!(ok(b"set x {a  b   c}; llength $x; set x"), b"a  b   c");
    // An in-place mutation invalidates the cached rep → canonical regenerates.
    assert_eq!(
        ok(b"set x {a  b   c}; llength $x; lappend x d; set x"),
        b"a b c d"
    );
}

#[test]
fn duplicate_preserves_string_rep() {
    // `set y $x` shares x's (shimmered) obj; `lappend x d` copies-on-write,
    // so the original obj y holds must keep its original spelling, and x gets
    // the canonical mutated form.
    assert_eq!(
        ok(b"set x {a  b   c}; llength $x; set y $x; lappend x d; set y"),
        b"a  b   c"
    );
    assert_eq!(
        ok(b"set x {a  b   c}; llength $x; set y $x; lappend x d; set x"),
        b"a b c d"
    );
}

#[test]
fn hex_indices_and_counts_accepted() {
    // Lset/ledit and string index/repeat now share the
    // radix-aware index/integer core, so hex specs resolve like real Tcl.
    assert_eq!(ok(b"set x {a b c}; lset x 0x1 Z; set x"), b"a Z c");
    assert_eq!(ok(b"string index abcdef 0x2"), b"c");
    assert_eq!(ok(b"string index abcdef end-0x1"), b"e");
    assert_eq!(ok(b"string repeat x 0x3"), b"xxx");
    // lrepeat count is radix-aware too.
    assert_eq!(ok(b"lrepeat 0x2 a"), b"a a");
}

#[test]
fn lindex_and_lrange() {
    assert_eq!(ok(b"lindex {a b c} 1"), b"b");
    assert_eq!(ok(b"lindex {a b c} end"), b"c");
    assert_eq!(ok(b"lindex {a b c} end-1"), b"b");
    assert_eq!(ok(b"lindex {a b c} 9"), b""); // out of range
    assert_eq!(ok(b"lrange {a b c d e} 1 3"), b"b c d");
    assert_eq!(ok(b"lrange {a b c} 1 end"), b"b c");
}

#[test]
fn lappend_builds_and_cow() {
    assert_eq!(ok(b"lappend x a; lappend x b c"), b"a b c");
    // COW: y shares x's value; appending to y must not change x
    assert_eq!(
        ok(b"set x {a b}; set y $x; lappend y c; list $x $y"),
        b"{a b} {a b c}"
    );
    // lappend onto a string var shimmers it to a list
    assert_eq!(ok(b"set s {1 2}; lappend s 3"), b"1 2 3");
    // lappend addresses array elements (not a scalar literally named `a(k)`),
    // including fully-qualified element keys (the safe-base / opt case).
    assert_eq!(ok(b"lappend a(k) 1 2; lappend a(k) 3"), b"1 2 3");
    assert_eq!(ok(b"lappend a(k) 1 2; set a(k)"), b"1 2");
    assert_eq!(
        ok(b"namespace eval n { variable arr; lappend arr(::x::y) a b }; set ::n::arr(::x::y)"),
        b"a b"
    );
}

/// `lappend` routed through the shared COW core: the no-values form now
/// validates the current value as a list (erroring on a malformed one, like
/// tclsh — the old runtime skipped this), creates an empty list when unset,
/// and a mutating `lappend` fires the write trace once.
#[test]
fn lappend_shared_core_parity() {
    // no-values on a malformed list errors (was silently returned before).
    let (c, m) = run(b"set y \"{\"; lappend y");
    assert_eq!(c, Code::Error);
    assert_eq!(m, b"unmatched open brace in list");
    // no-values on an unset variable creates an empty list.
    assert_eq!(ok(b"lappend fresh; info exists fresh"), b"1");
    assert_eq!(ok(b"lappend fresh; set fresh"), b"");
    // a mutating lappend fires the write trace exactly once.
    assert_eq!(
        ok(b"set l {1 2}; set m 0; trace add variable l write {incr ::m;#}; lappend l 3; set m"),
        b"1"
    );
}

#[test]
fn empty_append_native_contracts_preserve_bytes_and_selected_validation() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        leak_free(|interp| {
            interp.set_dialect_profile(
                tcl_registry::model::ingress::resolve_environment(dialect).unit_profile(),
            );
            for (source, expected) in [
                    (b"list [catch {append fresh}] [info exists fresh]".as_slice(), b"1 0".as_slice()),
                    (b"lappend created; list [info exists created] [set created]", b"1 {}"),
                    (b"set spaced {a  b}; lappend spaced", b"a  b"),
                    (
                        br#"set malformed "\{"; list [catch {lappend malformed}] [string equal $malformed "\{"]"#,
                        if dialect == "jim" { b"0 1" } else { b"1 1" },
                    ),
                ] {
                    assert_eq!(interp.eval_str(source), Code::Ok, "{dialect}: {source:?}");
                    assert_eq!(interp.result_bytes(), expected, "{dialect}: {source:?}");
                }
        });
    }
}

/// A write trace that mutates or unsets the variable during `lappend`
/// (append-7.x): the result is the variable's *post-trace* value (empty when
/// unset, the trace's new value otherwise), matching C — and the fresh list
/// object is not freed mid-command (the `run` helper's leak / double-free
/// counters guard against a use-after-free here).
#[test]
fn lappend_write_trace_unset_and_rewrite() {
    // The write trace unsets the variable: result is empty, var gone.
    assert_eq!(
        ok(b"proc foo args {global x; unset x}\ntrace add variable x write foo\nlappend x 1"),
        b""
    );
    assert_eq!(
            ok(b"proc foo args {global x; unset x}\ntrace add variable x write foo\nlappend x 1; info exists x"),
            b"0"
        );
    // The write trace rewrites the variable: result reflects the new value.
    assert_eq!(
        ok(b"proc foo args {global y; set y ZZZ}\ntrace add variable y write foo\nlappend y 1"),
        b"ZZZ"
    );
}

/// `lappend` fires a read trace (its side effects run) but swallows a trace
/// error, creating a missing element instead of failing (append-7.2/9.0,
/// bug 3057639) — where `set`/`append`-read would propagate the error.
#[test]
fn lappend_read_trace_fires_but_swallows_error() {
    // Side effects run: the read trace observes `name {} read`.
    assert_eq!(
            ok(b"set ::r {}\nproc foo args {append ::r $args}\ntrace add variable v read foo\nlappend v a\nset ::r"),
            b"v {} read"
        );
    // A read trace that errors does not fail lappend; it appends to empty.
    assert_eq!(
        ok(b"set v 1\ntrace add variable v read {error boom}\nlappend v a"),
        b"a"
    );
    // A succeeding read trace lets lappend see the real current value.
    assert_eq!(
        ok(b"set v 1\nproc foo args {}\ntrace add variable v read foo\nlappend v a"),
        b"1 a"
    );
    // bug 3057639: a read trace erroring on a missing element still creates it.
    let (c, m) = run(
            b"array set a {}\nproc nn {var key val} {upvar 1 $var l\n if {![info exists l($key)]} {return -code error x}}\ntrace add variable a read nn\nlappend a(key) hi",
        );
    assert_eq!(c, Code::Ok);
    assert_eq!(&m, b"hi");
}

#[test]
fn lreverse_concat_join_split() {
    assert_eq!(ok(b"lreverse {a b c}"), b"c b a");
    assert_eq!(ok(b"concat {a b} {c  d} { e }"), b"a b c  d e");
    assert_eq!(ok(b"join {a b c} -"), b"a-b-c");
    assert_eq!(ok(b"split a,b,c ,"), b"a b c");
    assert_eq!(ok(b"split {a b c}"), b"a b c"); // default whitespace
}

#[test]
fn lassign_assigns_and_returns_rest() {
    assert_eq!(ok(b"lassign {a b c} x y; list $x $y"), b"a b");
    assert_eq!(ok(b"lassign {a b c d} x y"), b"c d"); // returns the tail
    assert_eq!(ok(b"lassign {a} x y; list $x $y"), b"a {}"); // missing → empty
}

#[test]
fn errors() {
    let (c, b) = run(b"llength");
    assert_eq!(c, Code::Error);
    assert!(b.starts_with(b"wrong # args"));
}
