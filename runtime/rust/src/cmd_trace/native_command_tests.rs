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

//! Original command/execution trace operands and native callback byte extents.

use super::*;
use crate::obj::{self, Owned};

fn interpreter(version: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(version),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

#[test]
fn command_trace_reaches_original_prefix_before_a_refused_name() {
    for is_add in [false, true] {
        let mut interp = interpreter("tcl9.1");
        assert_eq!(interp.eval_str(b"namespace current"), Code::Ok);
        let name = Owned::retain(interp.result_obj());
        obj::invalidate_string(name.as_ptr());
        let member = Owned::fresh(obj::new_string_bytes(b"callback"));
        let prefix = Owned::fresh(interp.new_list_object(&[member.as_ptr()]));
        assert!(!obj::has_string_rep(prefix.as_ptr()));
        let operation = if is_add {
            b"add".as_slice()
        } else {
            b"remove".as_slice()
        };
        let words = [b"trace".as_slice(), operation, b"command", b"rename"]
            .map(|word| Owned::fresh(obj::new_string_bytes(word)));
        let argv = [
            words[0].as_ptr(),
            words[1].as_ptr(),
            words[2].as_ptr(),
            name.as_ptr(),
            words[3].as_ptr(),
            prefix.as_ptr(),
        ];
        assert_eq!(
            cmd_trace_add_remove(&mut interp, &argv, is_add, ops::CMD_ANY),
            Code::Error
        );
        assert!(interp.host_refusal_pending());
        assert!(
            obj::has_string_rep(prefix.as_ptr()),
            "prefix getter was reached first"
        );
        assert!(!obj::has_string_rep(name.as_ptr()));
        assert!(interp.traces.borrow().cmd_traces.is_empty());
    }
}

#[test]
fn legacy_operation_objects_match_native_counted_and_cstring_controls() {
    // Native proof: naming.variable.legacy-trace-operation-cstring
    // docs/design/analysis/name-resolution-proofs/legacy-trace-operation-cstring.md
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6"] {
        for (input, expected_code, expected) in [
            (b"w\0bad".as_slice(), Code::Ok, b"".as_slice()),
            (
                b"w\xc0\x80bad",
                Code::Error,
                b"bad operations \"w\xc0\x80bad\": should be one or more of rwua",
            ),
            (
                b"w\xff",
                Code::Error,
                b"bad operations \"w\xff\": should be one or more of rwua",
            ),
            (
                b"\0",
                Code::Error,
                b"bad operation list \"\": must be one or more of array, read, unset, or write",
            ),
        ] {
            let mut interp = interpreter(engine);
            let words = [b"trace".as_slice(), b"variable", b"v", input, b"callback"]
                .map(|word| Owned::fresh(obj::new_string_bytes(word)));
            let arguments: Vec<_> = words.iter().map(Owned::as_ptr).collect();
            assert_eq!(
                interp.dispatch(&arguments),
                expected_code,
                "{engine}/{input:?}"
            );
            assert_eq!(interp.result_bytes(), expected, "{engine}/{input:?}");
            assert!(!interp.host_refusal_pending());
            assert_eq!(interp.eval_str(b"trace vinfo v"), Code::Ok);
            assert_eq!(
                interp.result_bytes(),
                if expected_code == Code::Ok {
                    b"{w callback}".as_slice()
                } else {
                    b""
                }
            );
        }
    }
}

#[test]
fn command_trace_info_refuses_a_stringless_original_name() {
    let mut interp = interpreter("tcl9.1");
    assert_eq!(interp.eval_str(b"namespace current"), Code::Ok);
    let name = Owned::retain(interp.result_obj());
    obj::invalidate_string(name.as_ptr());
    let words = [b"trace".as_slice(), b"info", b"command"]
        .map(|word| Owned::fresh(obj::new_string_bytes(word)));
    let argv = [
        words[0].as_ptr(),
        words[1].as_ptr(),
        words[2].as_ptr(),
        name.as_ptr(),
    ];
    assert_eq!(
        cmd_trace_info(&mut interp, &argv, ops::CMD_ANY),
        Code::Error
    );
    assert!(interp.host_refusal_pending());
    assert!(!obj::has_string_rep(name.as_ptr()));
}

#[test]
fn original_command_traces_match_ten_native_byte_windows() {
    // Native proof: naming.command-trace.opaque-prefix-report-and-removal
    // docs/design/analysis/name-resolution-proofs/command-trace.opaque-prefix-report-and-removal.md
    // Native proof: naming.command-trace.counted-zero-prefix-storage-removal
    // docs/design/analysis/name-resolution-proofs/command-trace.counted-zero-prefix-storage-removal.md
    // Native proof: naming.command-trace.counted-zero-callback-evaluation
    // docs/design/analysis/name-resolution-proofs/command-trace.counted-zero-callback-evaluation.md
    fn decode(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    let mut compared = 0;
    for row in include_str!("../../tests/data/native_command_traces/windows.tsv").lines() {
        let fields = row.split('\t').collect::<Vec<_>>();
        let mut interp = interpreter(&format!("tcl{}", fields[0]));
        let source = decode(fields[2]);
        assert_eq!(
            interp.eval_str(&source).as_int(),
            fields[3].parse::<i64>().unwrap(),
            "{} {}",
            fields[0],
            fields[1]
        );
        assert_eq!(
            interp.result_bytes(),
            decode(fields[4]),
            "{} {}",
            fields[0],
            fields[1]
        );
        assert!(!interp.host_refusal_pending());
        compared += 1;
    }
    assert_eq!(compared, 10);
}

#[test]
fn variable_trace_prefix_copy_and_purpose_extents_match_public_native_controls() {
    // Native proof: naming.variable.copied-prefix-storage-removal-report-evaluation
    // docs/design/analysis/name-resolution-proofs/copied-prefix-storage-removal-report-evaluation.md
    for version in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for (removal, remains) in [
            (b"watch A\0Y".as_slice(), false),
            (b"watch A\0YY".as_slice(), true),
            (b"watch A\xc0\x80X".as_slice(), true),
        ] {
            let mut interp = interpreter(version);
            assert_eq!(
                interp.eval_str(b"proc watch {token args} {set ::seen $token}"),
                Code::Ok
            );
            let prefix = Owned::fresh(obj::new_string_bytes(b"watch A\0X"));
            let old_style = version == "tcl8.4";
            let texts: &[&[u8]] = if old_style {
                &[b"trace", b"variable", b"v", b"w"]
            } else {
                &[b"trace", b"add", b"variable", b"v", b"write"]
            };
            let owned: Vec<_> = texts
                .iter()
                .map(|text| Owned::fresh(obj::new_string_bytes(text)))
                .collect();
            let mut argv: Vec<_> = owned.iter().map(Owned::as_ptr).collect();
            argv.push(prefix.as_ptr());
            assert_eq!(
                if old_style {
                    legacy_var_add_remove(&mut interp, &argv, true)
                } else {
                    trace_var_add_remove(&mut interp, &argv, true)
                },
                Code::Ok
            );
            // The public native control mutates the original object after registration;
            // the trace's copied prefix must not share its later representation.
            unsafe { obj::set_string_rep(prefix.as_ptr(), b"watch MUTATED") };
            assert_eq!(
                interp.traces.borrow().traces.last().unwrap().command,
                b"watch A\0X"
            );
            assert_eq!(interp.eval_str(b"set v VALUE"), Code::Ok);
            assert_eq!(interp.eval_str(b"set ::seen"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"A\0X", "{version}");
            assert_eq!(
                var_trace_apply(
                    &mut interp,
                    b"v".to_vec(),
                    vec![b"write".to_vec()],
                    removal.to_vec(),
                    false,
                    old_style
                ),
                Code::Ok
            );
            assert_eq!(
                !interp.traces.borrow().traces.is_empty(),
                remains,
                "{version}: {removal:?}"
            );
        }
    }
}
