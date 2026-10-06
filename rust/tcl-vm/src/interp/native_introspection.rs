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

//! Original frame introspection retains real argv and selected numeric getter caches.

use super::{Value, Vm};
use tcl_cmd_core::native_info_level::NativeInfoLevelObjects;
use tcl_syntax::{
    scalar_getter::{NativeScalarGetterKind, NativeScalarGetterValue},
    value::ValueError,
};

impl NativeInfoLevelObjects for Vm {
    fn native_level_integer(
        &mut self,
        original: &Value,
        kind: NativeScalarGetterKind,
    ) -> Result<i64, ValueError> {
        let dialect = self.actual_native_invocation_dialect();
        if dialect.native_error_log_protocol().is_none() {
            return Err(ValueError::CommandProtocolUnavailable(
                "native info level issuer",
            ));
        }
        match original.native_scalar_getter(dialect, kind)? {
            NativeScalarGetterValue::Wide(integer) => Ok(integer),
            _ => Err(ValueError::ScalarNumericInputUnavailable),
        }
    }
    fn native_level_arguments(&self, level: usize) -> Result<Option<Value>, ValueError> {
        let Some(arguments) = self.frame_argv(level) else {
            return Ok(None);
        };
        if arguments.is_empty() {
            return Err(ValueError::CommandProtocolUnavailable(
                "native original frame argv",
            ));
        }
        Ok(Some(Value::list(arguments)))
    }
}

impl Vm {
    pub(crate) fn native_info_level(
        &mut self,
        original: Option<&Value>,
    ) -> Result<Value, tcl_cmd_core::CmdError> {
        let dialect = self.actual_native_invocation_dialect();
        let version = dialect
            .tcl_version
            .filter(|version| {
                *version >= tcl_dialect::TclVersion::V8_6
                    && dialect.native_error_log_protocol().is_some()
            })
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native info level opcode",
            ))?;
        tcl_cmd_core::native_info_level::native_level(
            self,
            original,
            tcl_registry::native_introspection_compilation::native_info_level_getter(version),
            tcl_syntax::native_string::NativeStringProtocol::C(version),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_syntax::value::ValueOps;
    include!("../../../tcl-registry/tests/data/native_introspection_compilation/cases.rs");

    fn unhex(bytes: &str) -> Vec<u8> {
        bytes
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    #[test]
    fn compiled_introspection_preserves_thirty_three_native_original_header_windows() {
        original_windows(false);
    }

    #[test]
    fn compiled_arrays_preserve_forty_eight_native_original_header_and_local_windows() {
        original_windows(true);
    }

    #[test]
    fn compiled_array_make_preserves_the_native_write_array_error_category() {
        for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = crate::native_fixture::interpreter(
                tcl_dialect::DialectProfile::find(engine).unwrap(),
            );
            let completion = vm
                .try_eval_source(
                    "proc p {} {set a scalar; array set a {}}; catch {p} message; set ::errorCode",
                )
                .unwrap();
            assert_eq!(completion.code, crate::Code::Ok, "{engine}");
            assert_eq!(
                vm.native_string_bytes(&completion.result).unwrap().as_ref(),
                b"TCL WRITE ARRAY",
                "{engine}"
            );
        }
    }

    fn assert_original_headers(
        vm: &mut Vm,
        completion: &crate::Completion<Value>,
        arguments: &[Value],
        fields: &[&str],
        engine: &str,
        case: usize,
    ) {
        let result = &completion.result;
        assert_eq!(
            result.native_object_type_name(),
            fields[2],
            "{engine}/{case}"
        );
        assert_eq!(
            usize::from(result.resident_string_bytes().is_some()).to_string(),
            fields[3],
            "{engine}/{case}"
        );
        assert_eq!(
            result.native_object_reference_count().to_string(),
            fields[4],
            "{engine}/{case}"
        );
        assert_eq!(
            arguments[0].native_object_type_name(),
            fields[5],
            "{engine}/{case}"
        );
        assert_eq!(
            usize::from(arguments[0].resident_string_bytes().is_some()).to_string(),
            fields[6],
            "{engine}/{case}"
        );
        assert_eq!(
            usize::from(result.is_same_object(&arguments[0])).to_string(),
            fields[7],
            "{engine}/{case}"
        );
        assert_eq!(
            vm.native_string_bytes(result).unwrap().as_ref(),
            unhex(fields[8]),
            "{engine}/{case}"
        );
    }

    fn original_windows(arrays: bool) {
        let mut windows = 0;
        for (engine, table) in [
            (
                "tcl8.6",
                include_str!(
                    "../../../tcl-registry/tests/data/native_introspection_compilation/8.6.18.tsv"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../tcl-registry/tests/data/native_introspection_compilation/9.0.4.tsv"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../tcl-registry/tests/data/native_introspection_compilation/9.1.0.tsv"
                ),
            ),
        ] {
            let mut vm = crate::native_fixture::interpreter(
                tcl_dialect::DialectProfile::find(engine).unwrap(),
            );
            vm.try_eval_source("namespace eval ::N {}; namespace eval ::source {proc target {} {}; namespace export target}; namespace eval ::N {namespace import ::source::target}").unwrap();
            for row in table.lines() {
                let fields: Vec<_> = row.split('\t').collect();
                let case = fields[0].parse::<usize>().unwrap();
                if if arrays {
                    case < 15
                } else {
                    !matches!(case, 0 | 2 | 3 | 4 | 6 | 8 | 9 | 10 | 11 | 12 | 13)
                } {
                    continue;
                }
                vm.try_eval_source(&format!("proc ::N::p {{left right}} {{{}}}", CASES[case]))
                    .unwrap();
                let head = Value::new_native_string_bytes(b"::N::p".as_slice());
                let left = match case {
                    2 => b"target".as_slice(),
                    10 => b"0".as_slice(),
                    _ => b"a".as_slice(),
                };
                let arguments = [
                    Value::new_native_string_bytes(left),
                    Value::new_native_string_bytes(b"k V j W".as_slice()),
                ];
                let completion = vm.invoke_host_original_object_vector(&head, &arguments);
                assert!(
                    vm.execution_refusal.is_none(),
                    "{engine}/{case}: {:?}",
                    vm.execution_refusal
                );
                assert_eq!(
                    completion.code.as_int().to_string(),
                    fields[1],
                    "{engine}/{case}"
                );
                assert_original_headers(&mut vm, &completion, &arguments, &fields, engine, case);
                if arrays {
                    let Some(crate::command::Command::Proc(procedure)) =
                        vm.lookup_command("::N::p")
                    else {
                        panic!("actual original array procedure");
                    };
                    let declaration = procedure.declaration();
                    let cache = declaration
                        .body_src
                        .native_bytecode_cache()
                        .expect("original array Bytecode");
                    assert_eq!(
                        cache.unit.asm.lvt.len().to_string(),
                        fields[10],
                        "{engine}/{case}: compiled locals"
                    );
                }
                windows += 1;
            }
        }
        assert_eq!(windows, if arrays { 48 } else { 33 });
    }
}
