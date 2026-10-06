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

//! Original List and index owners for the admitted C list-index instruction.

use super::*;
use tcl_registry::native_list_index_compilation::{
    NativeListIndexInstruction, NativeListIndexOperation,
};

pub(super) struct ListIndexOperation {
    operation: NativeListIndexOperation,
    operands: Vec<NamespaceOperand>,
    pub(super) prepared_words: HashMap<usize, WordInstruction>,
}

impl Builder<'_> {
    pub(super) fn list_index_operation(
        &mut self,
        words: &NativeCompilerWords<'_>,
        recipe: NativeListIndexInstruction,
        depth: u32,
    ) -> Result<ListIndexOperation, ValueError> {
        let mut prepared_words = HashMap::new();
        let mut operands = Vec::with_capacity(recipe.operands.len());
        for operand in recipe.operands {
            operands.push(self.namespace_operand(words, &operand, &mut prepared_words, depth)?);
        }
        Ok(ListIndexOperation {
            operation: recipe.operation,
            operands,
            prepared_words,
        })
    }
}

impl Interp {
    pub(super) fn execute_body_list_index(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        index: &ListIndexOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let mut values = Vec::with_capacity(index.operands.len());
        for operand in &index.operands {
            values.push(self.body_namespace_operand(artifact, command, operand, execution)?);
            if execution.done {
                return Ok(Code::Ok);
            }
        }
        let pointers = values.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
        let result = match index.operation {
            NativeListIndexOperation::Single => {
                tcl_cmd_core::list::lindex(self, &pointers[0], &pointers[1..])
            }
            NativeListIndexOperation::Multi(_) => {
                tcl_cmd_core::list::lindex_flat(self, &pointers[0], &pointers[1..])
            }
            NativeListIndexOperation::Immediate(coordinate) => self
                .list_elements(&pointers[0])
                .map(|members| {
                    coordinate
                        .resolve(members.len())
                        .map_or_else(|| self.empty(), |index| members[index])
                })
                .map_err(Into::into),
        };
        match result {
            Ok(value) => {
                self.set_result(value);
                Ok(Code::Ok)
            }
            Err(error) => Ok(self.report_cmd_error(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    include!("../../../../../rust/tcl-registry/tests/data/native_list_index_compilation/cases.rs");
    const TABLES: &[(&str, &str)] = &[
        ("tcl8.4", include_str!("../../../../../rust/tcl-registry/tests/data/native_list_index_compilation/8.4.20.txt")),
        ("tcl8.5", include_str!("../../../../../rust/tcl-registry/tests/data/native_list_index_compilation/8.5.19.txt")),
        ("tcl8.6", include_str!("../../../../../rust/tcl-registry/tests/data/native_list_index_compilation/8.6.18.txt")),
        ("tcl9.0", include_str!("../../../../../rust/tcl-registry/tests/data/native_list_index_compilation/9.0.4.txt")),
        ("tcl9.1", include_str!("../../../../../rust/tcl-registry/tests/data/native_list_index_compilation/9.1.0.txt")),
    ];
    fn unhex(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    #[test]
    fn original_list_index_artifact_preserves_95_native_header_and_completion_windows() {
        let mut windows = 0;
        for &(engine, table) in TABLES {
            let mut interp = super::super::tests::interpreter(engine);
            for row in table.lines() {
                let fields: Vec<_> = row.split('|').collect();
                let case = fields[0].parse::<usize>().unwrap();
                assert_eq!(
                    interp.eval_str(format!("proc p {{x i}} {{{}}}", CASES[case]).as_bytes()),
                    Code::Ok,
                    "{engine}/{case} definition"
                );
                let owners = [b"p".as_slice(), b"{{A B} C} D", b"0"]
                    .map(|bytes| obj::Owned::fresh(new_string(bytes)));
                let argv = owners.each_ref().map(obj::Owned::as_ptr);
                let code = interp.eval_original_object_vector(&argv);
                assert!(
                    !interp.host_refusal_pending(),
                    "{engine}/{case}: {:?}",
                    interp.native_access_refusal()
                );
                assert_eq!(code.as_int().to_string(), fields[1], "{engine}/{case}");
                let result = interp.result_obj();
                let descriptor = obj::obj_type_ptr(result);
                // SAFETY: the interpreter result owns this header and its descriptor.
                let (primary, references) = unsafe {
                    (
                        if descriptor.is_null() {
                            "none"
                        } else {
                            core::ffi::CStr::from_ptr((*descriptor).name)
                                .to_str()
                                .unwrap()
                        },
                        (*result).ref_count,
                    )
                };
                assert_eq!(primary, fields[2], "{engine}/{case} primary");
                assert_eq!(
                    usize::from(obj::has_string_rep(result)).to_string(),
                    fields[3],
                    "{engine}/{case} resident"
                );
                assert_eq!(
                    references.to_string(),
                    fields[4],
                    "{engine}/{case} references"
                );
                assert_eq!(
                    usize::from(result == argv[1]).to_string(),
                    fields[5],
                    "{engine}/{case} original List"
                );
                assert_eq!(
                    interp.result_bytes(),
                    unhex(fields[6]),
                    "{engine}/{case} result"
                );
                windows += 1;
            }
        }
        assert_eq!(windows, 95);
    }
}
