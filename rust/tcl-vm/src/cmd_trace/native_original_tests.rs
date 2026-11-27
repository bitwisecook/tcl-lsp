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

//! Trace operands holding raw bytes, through the trace command directly.

use super::*;
use tcl_dialect::TclVersion;

fn physical_vm(version: TclVersion) -> Vm {
    crate::native_fixture::core(
        tcl_registry::model::resolve_environment(version.dialect_profile_name()).unit_profile(),
    )
}

#[test]
fn opaque_missing_trace_command_reports_original_bytes_without_panicking() {
    for version in TclVersion::ALL {
        let mut vm = physical_vm(version);
        let completion = cmd_trace(
            &mut vm,
            &[
                Value::string("info"),
                Value::string("command"),
                Value::from_native_string_bytes(b"absent\xff".as_slice()),
            ],
        );
        assert_eq!(completion.code, tcl_runtime_api::Code::Error);
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            b"unknown command \"absent\xff\""
        );
    }
}

#[test]
fn trace_type_wrong_arity_uses_selected_member_without_decoding_original_operand() {
    // Native proof: naming.trace.selected-type-wrong-arity
    // docs/design/analysis/name-resolution-proofs/trace-selected-type-wrong-arity.md
    // Native proof: naming.trace.original-type-wrong-arity-inputs
    // docs/design/analysis/name-resolution-proofs/trace-original-type-wrong-arity-inputs.md
    for (version, rows) in [
        (
            TclVersion::V8_4,
            include_str!(
                "../../../tcl-cmd-core/tests/data/native_trace_selected_type/tcl8.4/stdout.tsv"
            ),
        ),
        (
            TclVersion::V8_5,
            include_str!(
                "../../../tcl-cmd-core/tests/data/native_trace_selected_type/tcl8.5/stdout.tsv"
            ),
        ),
        (
            TclVersion::V8_6,
            include_str!(
                "../../../tcl-cmd-core/tests/data/native_trace_selected_type/tcl8.6/stdout.tsv"
            ),
        ),
        (
            TclVersion::V9_0,
            include_str!(
                "../../../tcl-cmd-core/tests/data/native_trace_selected_type/tcl9.0/stdout.tsv"
            ),
        ),
        (
            TclVersion::V9_1,
            include_str!(
                "../../../tcl-cmd-core/tests/data/native_trace_selected_type/tcl9.1/stdout.tsv"
            ),
        ),
    ] {
        for (kind, abbreviation) in [
            (b"var\0\xff".as_slice(), "var"),
            (b"com\0\xff".as_slice(), "com"),
            (b"exec\0\xff".as_slice(), "exec"),
        ] {
            for member in ["info", "add"] {
                let label = format!("DIRECT_RAW_ZERO_{abbreviation}_{member}|");
                let expected = rows
                    .lines()
                    .find_map(|row| row.strip_prefix(&label))
                    .unwrap();
                let (code, result) = expected.split_once('|').unwrap();
                let result = result
                    .as_bytes()
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect::<Vec<_>>();
                let mut vm = physical_vm(version);
                let original = Value::from_native_string_bytes(kind);
                let completion = cmd_trace(&mut vm, &[Value::string(member), original]);
                assert_eq!(completion.code.as_int().to_string(), code);
                assert_eq!(
                    completion.result.string_bytes().as_ref(),
                    result,
                    "{version:?}/{member}/{abbreviation}"
                );
            }
        }
    }
}

#[test]
fn legacy_operation_objects_match_native_counted_and_cstring_controls() {
    // Native proof: naming.variable.legacy-trace-operation-cstring
    // docs/design/analysis/name-resolution-proofs/legacy-trace-operation-cstring.md
    let cases = [
        ("LEGACY_PLAIN", b"w".as_slice()),
        ("LEGACY_RAW_ZERO", b"w\0bad"),
        ("LEGACY_ENCODED_ZERO", b"w\xc0\x80bad"),
        ("LEGACY_RAW_FF", b"w\xff"),
        ("LEGACY_ONLY_ZERO", b"\0"),
    ];
    for (engine, rows) in [
        (
            "tcl8.4",
            include_str!("../../tests/data/native_private_legacy_trace/v3/tcl8.4/stdout.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../tests/data/native_private_legacy_trace/v3/tcl8.5/stdout.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../tests/data/native_private_legacy_trace/v3/tcl8.6/stdout.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../tests/data/native_private_legacy_trace/v3/tcl9.0/stdout.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../tests/data/native_private_legacy_trace/v3/tcl9.1/stdout.tsv"),
        ),
    ] {
        for (label, operations) in cases {
            let mut vm = crate::native_fixture::core(
                tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
            );
            let arguments = [b"variable".as_slice(), b"v", operations, b"callback"]
                .map(Value::new_native_string_bytes);
            let registered = vm.try_invoke_command("trace", &arguments).unwrap();
            for (position, row_label) in [label.to_owned(), format!("{label}_INFO")]
                .into_iter()
                .enumerate()
            {
                // Inspect each interpreter result before the next command releases it.
                let result = if position == 0 {
                    registered.clone()
                } else {
                    vm.try_invoke_command(
                        "trace",
                        &[
                            Value::new_native_string_bytes(b"vinfo".as_slice()),
                            Value::new_native_string_bytes(b"v".as_slice()),
                        ],
                    )
                    .unwrap()
                };
                let fields: Vec<_> = rows
                    .lines()
                    .find(|row| row.starts_with(&format!("{row_label}|")))
                    .unwrap()
                    .split('|')
                    .collect();
                assert_eq!(
                    result.code.as_int(),
                    fields[1].parse::<i64>().unwrap(),
                    "{engine}/{row_label}: {result:?}"
                );
                let expected: Vec<_> = fields[2]
                    .as_bytes()
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect();
                assert_eq!(
                    result
                        .result
                        .native_string_bytes(vm.actual_native_string_protocol().unwrap())
                        .unwrap_or_else(|error| panic!(
                            "{engine}/{row_label}: {error:?}, type {}",
                            result.result.native_object_type_name()
                        ))
                        .as_ref(),
                    expected,
                    "{engine}/{row_label}"
                );
            }
        }
    }
}

#[test]
fn unavailable_trace_prefix_refuses_before_materializing_original_name() {
    use tcl_core_types::ROOT_NS;
    use tcl_syntax::native_namespace_name::NativeNamespaceObjectProducer;

    let mut vm = physical_vm(TclVersion::V9_0);
    let prefix = vm
        .native_namespace_result_object(ROOT_NS, NativeNamespaceObjectProducer::Current)
        .unwrap();
    prefix.invalidate_native_string_for_test();
    let name = Value::int(7);
    assert!(name.resident_string_bytes().is_none());
    let completion = cmd_trace(
        &mut vm,
        &[
            Value::string("add"),
            Value::string("command"),
            name.clone(),
            Value::string("delete"),
            prefix,
        ],
    );
    assert_eq!(completion.code, tcl_runtime_api::Code::Error);
    assert!(vm.refused_completion().is_some());
    assert!(name.resident_string_bytes().is_none());
}
