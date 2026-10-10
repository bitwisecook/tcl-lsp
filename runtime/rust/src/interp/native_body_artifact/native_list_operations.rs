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

//! Original list receivers, sequential target stores and selected COW transactions.

use super::*;
use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand;
use tcl_registry::native_list_operations_compilation::{
    NativeListOperationInstruction as Plan, NativeListVariableOperand,
};
use tcl_syntax::native_compiled_index::{
    NativeCompiledListIndex as Index, NativeCompiledListRange,
};

enum ListTarget {
    Original {
        target: Box<Target>,
        word: usize,
    },
    Expanded {
        root: Vec<u8>,
        slot: Option<usize>,
        root_literal: Option<usize>,
        index_literal: Option<usize>,
    },
}
impl ListTarget {
    fn slot(&self) -> Option<usize> {
        match self {
            Self::Original { target, .. } => target.slot,
            Self::Expanded { slot, .. } => *slot,
        }
    }
}
enum ListOperation {
    Range {
        list: NamespaceOperand,
        range: NativeCompiledListRange,
    },
    Assign {
        list: NamespaceOperand,
        targets: Vec<ListTarget>,
    },
    // Insert and Set require the concrete replacement and nested path-COW owners.
}
pub(super) struct ListOperationsOperation {
    operation: ListOperation,
    pub(super) prepared_words: HashMap<usize, WordInstruction>,
}
impl Builder<'_> {
    pub(super) fn list_operation(
        &mut self,
        words: &NativeCompilerWords<'_>,
        recipe: Plan,
        depth: u32,
    ) -> Result<ListOperationsOperation, ValueError> {
        let mut prepared_words = HashMap::new();
        let operation = match recipe {
            Plan::Range { list, first, last } => ListOperation::Range {
                list: self.namespace_operand(words, &list, &mut prepared_words, depth)?,
                range: NativeCompiledListRange { first, last },
            },
            Plan::Assign { list, targets } => {
                let list = self.namespace_operand(words, &list, &mut prepared_words, depth)?;
                let mut prepared = Vec::with_capacity(targets.len());
                for target in targets {
                    prepared.push(self.list_operation_target(
                        words,
                        target,
                        &mut prepared_words,
                        depth,
                    )?);
                }
                ListOperation::Assign {
                    list,
                    targets: prepared,
                }
            }
            _ => return Err(unavailable("original list replacement storage transaction")),
        };
        Ok(ListOperationsOperation {
            operation,
            prepared_words,
        })
    }
    fn list_operation_target(
        &mut self,
        words: &NativeCompilerWords<'_>,
        original: NativeListVariableOperand,
        prepared_words: &mut HashMap<usize, WordInstruction>,
        depth: u32,
    ) -> Result<ListTarget, ValueError> {
        Ok(match original {
            NativeListVariableOperand::Original {
                operand: NativeCompilerWordOperand::Original(word),
                variable,
            } => {
                if matches!(variable, NativeVariableWordOperand::DynamicWord) {
                    prepared_words.insert(word, self.namespace_word(words, word, false, depth)?);
                }
                ListTarget::Original {
                    target: Box::new(self.target(variable, depth)?),
                    word,
                }
            }
            NativeListVariableOperand::ExpandedLiteral { name, index, .. } => {
                let slot = self.local(&name, None);
                let root_literal = slot.is_none().then(|| self.literals.intern_bytes(&name));
                let index_literal = index.map(|index| self.literals.intern_bytes(&index));
                ListTarget::Expanded {
                    root: name,
                    slot,
                    root_literal,
                    index_literal,
                }
            }
            _ => return Err(unavailable("original list target geometry")),
        })
    }
}
impl Interp {
    fn body_list_target(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        target: &ListTarget,
        execution: &mut BodyExecution,
    ) -> Result<EvaluatedTarget, Code> {
        match target {
            ListTarget::Original { target, word } => {
                self.body_target_at(artifact, command, target, *word, execution)
            }
            ListTarget::Expanded {
                root,
                root_literal,
                index_literal,
                ..
            } => {
                let retain = |index| {
                    obj::Owned::retain(
                        artifact
                            .literals
                            .original(index)
                            .expect("expanded target PUSH"),
                    )
                };
                Ok(EvaluatedTarget {
                    root: root.clone(),
                    element: None,
                    original_name: root_literal.map(retain),
                    original_index: index_literal.map(retain),
                    combined: false,
                })
            }
        }
    }
    pub(super) fn execute_body_list_operation(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        recipe: &ListOperationsOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let protocol = NativeStringProtocol::C(artifact.stamp.physical);
        let result = match &recipe.operation {
            ListOperation::Range { list, range } => {
                let original = self.body_namespace_operand(artifact, command, list, execution)?;
                if execution.done {
                    return Ok(Code::Ok);
                }
                crate::list::native_list_range(original.as_ptr(), *range, protocol)
                    .map_err(|error| self.report_cmd_error(error.into()))?
            }
            ListOperation::Assign { list, targets } => {
                let original = self.body_namespace_operand(artifact, command, list, execution)?;
                if execution.done {
                    return Ok(Code::Ok);
                }
                for (index, target) in targets.iter().enumerate() {
                    let evaluated = self.body_list_target(artifact, command, target, execution)?;
                    if execution.done {
                        return Ok(Code::Ok);
                    }
                    // Genuine DUP/OVER stack role precedes the original List getter.
                    let duplicate = obj::Owned::retain(original.as_ptr());
                    let member = tcl_cmd_core::native_list_index::immediate(
                        self,
                        &duplicate.as_ptr(),
                        Index::from_encoded(i32::try_from(index).expect("native target count")),
                    )
                    .map_err(|error| self.report_cmd_error(error))?;
                    drop(duplicate);
                    drop(self.body_store_value(&evaluated, target.slot(), member.owned())?);
                }
                crate::list::native_list_range(
                    original.as_ptr(),
                    NativeCompiledListRange {
                        first: Index::from_encoded(
                            i32::try_from(targets.len()).expect("native target count"),
                        ),
                        last: Index::from_encoded(-2),
                    },
                    protocol,
                )
                .map_err(|error| self.report_cmd_error(error.into()))?
            }
        };
        self.set_result(result.as_ptr());
        Ok(Code::Ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    include!("../../../../../rust/tcl-registry/tests/data/native_list_operations/cases.rs");
    include!("../../../../../rust/tcl-registry/tests/data/native_list_operations/tables.rs");
    include!("../../../../../rust/tcl-registry/tests/data/native_list_assignment_order/cases.rs");
    fn unhex(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    fn header(value: *mut TclObj) -> [String; 3] {
        let descriptor = obj::obj_type_ptr(value);
        // SAFETY: the caller holds the actual result or original argument owner.
        unsafe {
            [
                if descriptor.is_null() {
                    "none".to_owned()
                } else {
                    core::ffi::CStr::from_ptr((*descriptor).name)
                        .to_str()
                        .unwrap()
                        .to_owned()
                },
                usize::from(obj::has_string_rep(value)).to_string(),
                (*value).ref_count.to_string(),
            ]
        }
    }
    fn run(engine: &str, body: &str, result: &[&str], original: &[&str]) {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(engine),
            tcl_registry::special_vars::NativeBootstrapInputs {
                package_path: Vec::new(),
                default_library: None,
            },
        )
        .expect("authenticated original native constructor");
        assert_eq!(interp.eval_str(format!("set ::log {{}}; proc tap {{v}} {{lappend ::log $v;return $v}}; proc p {{name idx}} {{set x $name;set a(1) $name;{body}}}").as_bytes()),Code::Ok);
        let owners =
            [b"p".as_slice(), b"{A} B C", b"1"].map(|bytes| obj::Owned::fresh(new_string(bytes)));
        let argv = owners.each_ref().map(obj::Owned::as_ptr);
        let code = interp.eval_original_object_vector(&argv);
        assert!(
            !interp.host_refusal_pending(),
            "{engine}/{body}: {:?}",
            interp.native_access_refusal()
        );
        assert_eq!(code.as_int().to_string(), result[2], "{engine}/{body}");
        assert_eq!(
            header(interp.result_obj()).each_ref().map(String::as_str),
            result[3..6],
            "{engine}/{body}: result header"
        );
        assert_eq!(
            interp.result_bytes(),
            unhex(result[6]),
            "{engine}/{body}: result bytes"
        );
        assert_eq!(
            header(argv[1]).each_ref().map(String::as_str),
            original[2..5],
            "{engine}/{body}: original header"
        );
        assert_eq!(obj_bytes(argv[1]), unhex(original[5]));
    }
    #[test]
    fn original_range_and_assignment_match_84_native_completion_and_original_header_pairs() {
        // Native proof: naming.list.assignment-interleaved-target-effects
        // docs/design/analysis/name-resolution-proofs/list.assignment-interleaved-target-effects.md

        // Native proof: naming.list.original-assign-objects-and-instructions
        // docs/design/analysis/name-resolution-proofs/list.original-assign-objects-and-instructions.md

        // Native proof: naming.list.original-range-objects-and-instructions
        // docs/design/analysis/name-resolution-proofs/list.original-range-objects-and-instructions.md

        let mut windows = 0;
        for &(engine, list, order) in LIST_TABLES {
            for (rows, bodies, cases) in [
                (list, CASES, &[0usize, 1, 2, 3, 4, 11, 12, 13, 14, 28][..]),
                (order, ORDER_CASES, &[0usize, 1, 2, 3][..]),
            ] {
                for line in rows.lines().filter(|line| line.starts_with("R|")) {
                    let result = line.split('|').collect::<Vec<_>>();
                    let case = result[1].parse::<usize>().unwrap();
                    if !cases.contains(&case) {
                        continue;
                    }
                    let prefix = format!("O|{case}|");
                    let original = rows
                        .lines()
                        .find(|line| line.starts_with(&prefix))
                        .unwrap()
                        .split('|')
                        .collect::<Vec<_>>();
                    run(engine, bodies[case], &result, &original);
                    windows += 1;
                }
            }
        }
        assert_eq!(windows, 84);
    }
}
