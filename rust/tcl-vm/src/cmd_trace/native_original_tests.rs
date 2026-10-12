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

//! Original byte operands against five direct native trace measurements.

use super::*;
use std::{cell::RefCell, io::Write, rc::Rc};
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn physical_vm(version: TclVersion, output: Capture) -> Vm {
    let profile = tcl_registry::model::resolve_environment(version.dialect_name()).unit_profile();
    let mut vm = Vm::with_native_core(
        Box::new(output),
        Rc::new(crate::host_native::NativeHost::new()),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .expect("matching independently selected physical core");
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    vm
}

fn check_original_source(source: &[u8], expected: impl Fn(TclVersion) -> &'static [u8]) {
    for version in TclVersion::ALL {
        let output = Capture::default();
        let mut vm = physical_vm(version, output.clone());
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
        include_bytes!("../../../tcl-syntax/tests/data/native_command_trace/opaque.tcl"),
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
        include_bytes!("../../../tcl-syntax/tests/data/native_command_trace/counted_nul.tcl"),
        |version| {
            if version <= TclVersion::V8_5 {
                b"6362ff\n0\n6362ff\n0\n{} {}\n"
            } else {
                b"6362ff\n0\n6362ff\n0\nenter leave\n"
            }
        },
    );
}

#[test]
fn opaque_missing_trace_command_reports_original_bytes_without_panicking() {
    for version in TclVersion::ALL {
        let mut vm = physical_vm(version, Capture::default());
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
                "../../../tcl-cmd-core/tests/data/native_trace_selected_type/8.4.20/stdout.tsv"
            ),
        ),
        (
            TclVersion::V8_5,
            include_str!(
                "../../../tcl-cmd-core/tests/data/native_trace_selected_type/8.5.19/stdout.tsv"
            ),
        ),
        (
            TclVersion::V8_6,
            include_str!(
                "../../../tcl-cmd-core/tests/data/native_trace_selected_type/8.6.18/stdout.tsv"
            ),
        ),
        (
            TclVersion::V9_0,
            include_str!(
                "../../../tcl-cmd-core/tests/data/native_trace_selected_type/9.0.4/stdout.tsv"
            ),
        ),
        (
            TclVersion::V9_1,
            include_str!(
                "../../../tcl-cmd-core/tests/data/native_trace_selected_type/9.1.0/stdout.tsv"
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
                let mut vm = physical_vm(version, Capture::default());
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
            include_str!("../../tests/data/native_private_legacy_trace/v3/8.4.20/stdout.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../tests/data/native_private_legacy_trace/v3/8.5.19/stdout.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../tests/data/native_private_legacy_trace/v3/8.6.18/stdout.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../tests/data/native_private_legacy_trace/v3/9.0.4/stdout.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../tests/data/native_private_legacy_trace/v3/9.1.0/stdout.tsv"),
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
                    .chunks_exact(2)
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect();
                assert_eq!(
                    result
                        .result
                        .native_string_bytes(
                            vm.actual_native_invocation_dialect()
                                .native_string_protocol()
                                .unwrap()
                        )
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

    let mut vm = physical_vm(TclVersion::V9_0, Capture::default());
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

#[test]
fn variable_trace_prefix_copy_and_purpose_extents_match_public_native_controls() {
    // Native proof: naming.variable.copied-prefix-storage-removal-report-evaluation
    // docs/design/analysis/name-resolution-proofs/copied-prefix-storage-removal-report-evaluation.md
    for version in TclVersion::ALL {
        for (removal, remains) in [
            (b"watch A\0Y".as_slice(), false),
            (b"watch A\0YY".as_slice(), true),
            (b"watch A\xc0\x80X".as_slice(), true),
        ] {
            let mut vm = physical_vm(version, Capture::default());
            let setup = vm
                .try_eval_source_bytes(b"proc watch {token args} {set ::seen $token}")
                .unwrap();
            assert_eq!(setup.code, tcl_runtime_api::Code::Ok);
            let prefix = Value::new_native_string_bytes(b"watch A\0X".as_slice());
            vm.add_var_trace_bytes(
                b"v",
                vec!["write".into()],
                &prefix,
                version == TclVersion::V8_4,
            );
            assert!(vm.refused_completion().is_none());
            // Registration owns copied bytes, not the input object's string updater
            // or its later invalidation and cache lifetime.
            prefix.invalidate_native_string_for_test();
            let info = vm.var_trace_info_bytes(b"v");
            assert_eq!(info.len(), 1);
            assert_eq!(info[0].1.string_bytes().as_ref(), b"watch A");
            let fired = vm.try_eval_source_bytes(b"set v VALUE").unwrap();
            assert_eq!(
                fired.code,
                tcl_runtime_api::Code::Ok,
                "{version:?}: {:?}",
                fired.result
            );
            assert_eq!(
                vm.get_var_bytes(b"::seen").unwrap().string_bytes().as_ref(),
                b"A\0X"
            );
            vm.remove_var_trace_bytes(b"v", &["write".into()], removal);
            assert_eq!(
                !vm.var_trace_info_bytes(b"v").is_empty(),
                remains,
                "{version:?}: {removal:?}"
            );
        }
    }
}
