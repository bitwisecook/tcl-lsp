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

//! Original scalar getters and genuine native integer result manufacture.

use super::{Value, Vm};
use tcl_registry::native_scalar_compilation::NativeScalarOperation;
use tcl_syntax::value::{ValueError, ValueOps};

impl Vm {
    pub(super) fn execute_native_scalar_length(
        &mut self,
        original: &Value,
        operation: NativeScalarOperation,
        version: tcl_dialect::TclVersion,
    ) -> Result<Value, ValueError> {
        let dialect = self.actual_native_invocation_dialect();
        if dialect.native_error_log_protocol().is_none() || dialect.tcl_version != Some(version) {
            return Err(ValueError::CommandProtocolUnavailable(
                "native scalar length result issuer",
            ));
        }
        let length = match operation {
            NativeScalarOperation::StringLength => self.native_char_len(original)?,
            NativeScalarOperation::ListLength => self.list_len(original)?,
            NativeScalarOperation::StringEqual => {
                return Err(ValueError::CommandProtocolUnavailable(
                    "native length instruction",
                ));
            }
        };
        let length = i64::try_from(length)
            .map_err(|_| ValueError::CommandProtocolUnavailable("native scalar length width"))?;
        let result = Value::int(length);
        result.set_native_unshared_integer(length, version)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    include!("../../../tcl-registry/tests/data/native_scalar_compilation/cases.rs");

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
    fn compiled_scalars_preserve_forty_one_native_original_header_windows() {
        let mut windows = 0;
        for (engine, table) in [
            (
                "tcl8.4",
                include_str!(
                    "../../../tcl-registry/tests/data/native_scalar_compilation/8.4.20.tsv"
                ),
            ),
            (
                "tcl8.5",
                include_str!(
                    "../../../tcl-registry/tests/data/native_scalar_compilation/8.5.19.tsv"
                ),
            ),
            (
                "tcl8.6",
                include_str!(
                    "../../../tcl-registry/tests/data/native_scalar_compilation/8.6.18.tsv"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../tcl-registry/tests/data/native_scalar_compilation/9.0.4.tsv"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../tcl-registry/tests/data/native_scalar_compilation/9.1.0.tsv"
                ),
            ),
        ] {
            let mut vm = crate::native_fixture::interpreter(
                tcl_dialect::DialectProfile::find(engine).unwrap(),
            );
            for row in table.lines() {
                let fields: Vec<_> = row.split('\t').collect();
                if !fields[9]
                    .split(',')
                    .any(|op| matches!(op, "streq" | "strlen" | "listlength" | "listLength"))
                {
                    continue;
                }
                let case = fields[0].parse::<usize>().unwrap();
                vm.try_eval_source(&format!("proc p {{left right}} {{{}}}", CASES[case]))
                    .unwrap();
                let head = Value::new_native_string_bytes(b"p".as_slice());
                let left = if matches!(case, 7..=9) {
                    b"a b".as_slice()
                } else {
                    b"A\0x".as_slice()
                };
                let arguments = [
                    Value::new_native_string_bytes(left),
                    Value::new_native_string_bytes(b"A\0y".as_slice()),
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
                let result = &completion.result;
                if result.native_object_type_name() != fields[2] {
                    let Some(crate::command::Command::Proc(command)) = vm.lookup_command("p")
                    else {
                        panic!("retained original scalar procedure");
                    };
                    let declaration = command.declaration();
                    let cache = declaration.body_src.native_bytecode_cache();
                    panic!(
                        "{engine}/{case}: actual result {:?}; retained instruction metadata {:?}",
                        (
                            result.native_object_type_name(),
                            result.resident_string_bytes(),
                            result.native_scalar_cache()
                        ),
                        cache.as_ref().map(|cache| cache
                            .unit
                            .asm
                            .instructions
                            .iter()
                            .take(40)
                            .map(|instruction| (
                                instruction.op,
                                &instruction.operands,
                                instruction.native_switch_version
                            ))
                            .collect::<Vec<_>>())
                    );
                }
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
                windows += 1;
            }
        }
        assert_eq!(windows, 41);
    }
}
