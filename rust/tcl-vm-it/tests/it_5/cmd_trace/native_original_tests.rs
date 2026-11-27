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

//! Command and variable trace prefixes holding raw bytes and a counted zero, through scripts and object words, against tclsh.

use tcl_dialect::TclVersion;
use tcl_runtime_api::Code;
use tcl_vm::Value;

fn check_original_source(source: &[u8], expected: impl Fn(TclVersion) -> &'static [u8]) {
    for version in TclVersion::ALL {
        let output = crate::fixture::Output::default();
        let mut vm = crate::fixture::interpreter_with_output(
            tcl_vm::test_support::profile(version.dialect_profile_name()),
            Box::new(output.clone()),
        );
        let completion = vm
            .try_eval_source_bytes(source)
            .expect("original byte source admitted");
        assert_eq!(
            completion.code,
            tcl_runtime_api::Code::Ok,
            "{version:?}: {:?}",
            completion.result
        );
        assert_eq!(
            output.0.borrow().as_slice(),
            expected(version),
            "{version:?}"
        );
    }
}

#[test]
fn opaque_command_trace_operands_match_all_five_direct_native_engines() {
    // Native proof: naming.command-trace.opaque-prefix-report-and-removal
    // docs/design/analysis/name-resolution-proofs/command-trace.opaque-prefix-report-and-removal.md
    check_original_source(
        include_bytes!("../../../../tcl-syntax/tests/data/native_command_trace/opaque.tcl"),
        |_| b"6362ff\n6362ff\n0\n1\n0\nenter leave rename delete\n",
    );
}

#[test]
fn counted_nul_command_trace_prefixes_preserve_native_storage_and_eval_extents() {
    // Native proof: naming.command-trace.counted-zero-prefix-storage-removal
    // docs/design/analysis/name-resolution-proofs/command-trace.counted-zero-prefix-storage-removal.md
    // Native proof: naming.command-trace.counted-zero-callback-evaluation
    // docs/design/analysis/name-resolution-proofs/command-trace.counted-zero-callback-evaluation.md
    check_original_source(
        include_bytes!("../../../../tcl-syntax/tests/data/native_command_trace/counted_nul.tcl"),
        |version| {
            if version <= TclVersion::V8_5 {
                b"6362ff\n0\n6362ff\n0\n{} {}\n"
            } else {
                b"6362ff\n0\n6362ff\n0\nenter leave\n"
            }
        },
    );
}

fn trace(vm: &mut tcl_vm::Vm, words: &[Value]) -> tcl_runtime_api::Completion<Value> {
    let completion = crate::fixture::call(vm, "trace", words);
    assert_eq!(completion.code, Code::Ok, "{:?}", completion.result);
    completion
}

#[test]
fn variable_trace_prefix_copy_and_purpose_extents_match_public_native_controls() {
    // Native proof: naming.variable.copied-prefix-storage-removal-report-evaluation
    // docs/design/analysis/name-resolution-proofs/copied-prefix-storage-removal-report-evaluation.md
    for version in TclVersion::ALL {
        let (add, remove, info, operation) = if version == TclVersion::V8_4 {
            (&["variable"][..], &["vdelete"][..], &["vinfo"][..], "w")
        } else {
            (
                &["add", "variable"][..],
                &["remove", "variable"][..],
                &["info", "variable"][..],
                "write",
            )
        };
        let words = |head: &[&str], tail: &[Value]| {
            head.iter()
                .map(|word| Value::string(*word))
                .chain(tail.iter().cloned())
                .collect::<Vec<_>>()
        };
        for (removal, remains) in [
            (b"watch A\0Y".as_slice(), false),
            (b"watch A\0YY".as_slice(), true),
            (b"watch A\xc0\x80X".as_slice(), true),
        ] {
            let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(
                version.dialect_profile_name(),
            ));
            crate::fixture::eval(&mut vm, "proc watch {token args} {set ::seen $token}");
            let variable = [Value::string("v"), Value::string(operation)];
            let prefix = Value::new_native_string_bytes(b"watch A\0X".as_slice());
            trace(
                &mut vm,
                &words(add, &[variable[0].clone(), variable[1].clone(), prefix]),
            );
            let report = trace(&mut vm, &words(info, &variable[..1])).result;
            let report = report.as_list().unwrap();
            assert_eq!(report.len(), 1, "{version:?}");
            let entry = report[0].as_list().unwrap();
            assert_eq!(entry[1].string_bytes().as_ref(), b"watch A", "{version:?}");
            let fired = crate::fixture::eval(&mut vm, "set v VALUE");
            assert_eq!(fired.code, Code::Ok, "{version:?}: {:?}", fired.result);
            assert_eq!(
                crate::fixture::eval(&mut vm, "set ::seen")
                    .result
                    .string_bytes()
                    .as_ref(),
                b"A\0X",
                "{version:?}"
            );
            let removal = Value::new_native_string_bytes(removal);
            trace(
                &mut vm,
                &words(remove, &[variable[0].clone(), variable[1].clone(), removal]),
            );
            let report = trace(&mut vm, &words(info, &variable[..1])).result;
            assert_eq!(
                !report.as_list().unwrap().is_empty(),
                remains,
                "{version:?}"
            );
        }
    }
}
