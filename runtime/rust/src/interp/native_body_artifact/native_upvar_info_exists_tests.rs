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

//! Original ordinary upvar/existence artifact completions and name-header windows.
use super::*;
use std::cell::RefCell;
include!("../../../../../rust/tcl-registry/tests/data/native_upvar_info_exists/cases.rs");
const TABLES: &[(&str, &str)] = &[
    (
        "tcl8.4",
        include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_upvar_info_exists/8.4.20.txt"
        ),
    ),
    (
        "tcl8.5",
        include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_upvar_info_exists/8.5.19.txt"
        ),
    ),
    (
        "tcl8.6",
        include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_upvar_info_exists/8.6.18.txt"
        ),
    ),
    (
        "tcl9.0",
        include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_upvar_info_exists/9.0.4.txt"
        ),
    ),
    (
        "tcl9.1",
        include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_upvar_info_exists/9.1.0.txt"
        ),
    ),
];
include!("../../../../../rust/tcl-registry/tests/data/native_upvar_info_exists/quiet/cases.rs");
const QUIET_TABLES: &[(&str, &str)] = &[
    (
        "tcl8.4",
        include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_upvar_info_exists/quiet/8.4.20.txt"
        ),
    ),
    (
        "tcl8.5",
        include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_upvar_info_exists/quiet/8.5.19.txt"
        ),
    ),
    (
        "tcl8.6",
        include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_upvar_info_exists/quiet/8.6.18.txt"
        ),
    ),
    (
        "tcl9.0",
        include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_upvar_info_exists/quiet/9.0.4.txt"
        ),
    ),
    (
        "tcl9.1",
        include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_upvar_info_exists/quiet/9.1.0.txt"
        ),
    ),
];
const SETUP: &str = "catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}";
fn unhex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

thread_local! {
    static MARKS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}
fn mark(_interp: &mut Interp, arguments: &[*mut TclObj]) -> Code {
    let value = arguments[1];
    let descriptor = obj::obj_type_ptr(value);
    // SAFETY: the active original argv owns this live header and descriptor.
    let (primary, references) = unsafe {
        (
            if descriptor.is_null() {
                "none"
            } else {
                core::ffi::CStr::from_ptr((*descriptor).name)
                    .to_str()
                    .unwrap()
            },
            (*value).ref_count,
        )
    };
    let mut bytes = format!(
        "{primary}|{}|{references}|",
        usize::from(obj::has_string_rep(value))
    );
    for byte in obj::bytes_of(value) {
        use std::fmt::Write;
        write!(bytes, "{byte:02x}").unwrap();
    }
    MARKS.with(|marks| marks.borrow_mut().push(bytes));
    Code::Ok
}
fn compare_native_tables(tables: &[(&str, &str)], cases: &[&str]) -> (usize, usize) {
    let mut results = 0;
    let mut headers = 0;
    for &(engine, table) in tables {
        let mut interp = super::tests::interpreter(engine);
        interp.register_builtin(b"mark", mark);
        for row in table.lines().filter(|row| row.starts_with("R|")) {
            let fields: Vec<_> = row.split('|').collect();
            let case = fields[1].parse::<usize>().unwrap();
            assert_eq!(interp.eval_str(SETUP.as_bytes()), Code::Ok);
            assert_eq!(
                interp.eval_str(
                    format!(
                        "proc p {{name idx}} {{set x LOCAL;set a(k) LOCALARRAY;{}}}",
                        cases[case]
                    )
                    .as_bytes()
                ),
                Code::Ok
            );
            MARKS.with(|marks| marks.borrow_mut().clear());
            let owners =
                [b"p".as_slice(), b"x", b"k"].map(|bytes| obj::Owned::fresh(new_string(bytes)));
            let argv = owners.each_ref().map(obj::Owned::as_ptr);
            let code = interp.eval_original_object_vector(&argv);
            assert!(
                !interp.host_refusal_pending(),
                "{engine}/{case}: {:?}",
                interp.native_access_refusal()
            );
            assert_eq!(code.as_int().to_string(), fields[2], "{engine}/{case}");
            let value = interp.result_obj();
            let descriptor = obj::obj_type_ptr(value);
            // SAFETY: the actual interpreter result owns this live header.
            let (primary, refs) = unsafe {
                (
                    if descriptor.is_null() {
                        "none"
                    } else {
                        core::ffi::CStr::from_ptr((*descriptor).name)
                            .to_str()
                            .unwrap()
                    },
                    (*value).ref_count,
                )
            };
            assert_eq!(primary, fields[3], "{engine}/{case} primary");
            assert_eq!(
                usize::from(obj::has_string_rep(value)).to_string(),
                fields[4],
                "{engine}/{case} resident"
            );
            assert_eq!(refs.to_string(), fields[5], "{engine}/{case} references");
            assert_eq!(
                interp.result_bytes(),
                unhex(fields[6]),
                "{engine}/{case} result"
            );
            let expected = table
                .lines()
                .filter(|row| row.starts_with(&format!("H|{case}|")))
                .map(|row| row.split('|').skip(3).collect::<Vec<_>>().join("|"))
                .collect::<Vec<_>>();
            MARKS.with(|marks| {
                assert_eq!(
                    *marks.borrow(),
                    expected,
                    "{engine}/{case} reached originals"
                )
            });
            results += 1;
            headers += expected.len();
        }
    }
    (results, headers)
}

// Native proof: naming.variable.original-upvar-and-exists-completion-and-name-windows
// docs/design/analysis/name-resolution-proofs/variable.original-upvar-and-exists-completion-and-name-windows.md
#[test]
fn original_upvar_and_exists_artifact_preserves_140_native_completions_and_20_name_headers() {
    assert_eq!(compare_native_tables(TABLES, CASES), (140, 20));
}
// Native proof: naming.variable.exists-read-trace-quiet-result-purpose
// docs/design/analysis/name-resolution-proofs/variable.exists-read-trace-quiet-result-purpose.md
#[test]
fn original_exists_quiet_trace_semantics_preserve_15_native_result_windows() {
    assert_eq!(compare_native_tables(QUIET_TABLES, QUIET_CASES), (15, 0));
}
