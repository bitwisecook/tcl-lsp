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
    ReplaceInsert {
        list: NamespaceOperand,
        index: NamespaceOperand,
        instruction:
            tcl_registry::native_list_operations_compilation::NativeListInsertionInstruction,
        elements: Vec<NamespaceOperand>,
    },
    Insert {
        steps: Vec<InsertionStep>,
    },
}
enum InsertionStep {
    Operand(NamespaceOperand),
    List(usize),
    Over(usize),
    Reverse(usize),
    Range(NativeCompiledListRange),
    Concat,
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
    ) -> Result<ListOperationsOperation, tcl_cmd_core::CmdError> {
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
            Plan::Insert {
                list,
                index: tcl_registry::native_list_operations_compilation::NativeListInsertionIndex::Original { operand, instruction },
                elements,
            } => {
                let list = self.namespace_operand(words, &list, &mut prepared_words, depth)?;
                let index = self.namespace_operand(words, &operand, &mut prepared_words, depth)?;
                let mut prepared = Vec::with_capacity(elements.len());
                for operand in elements {
                    prepared.push(self.namespace_operand(words, &operand, &mut prepared_words, depth)?);
                }
                ListOperation::ReplaceInsert { list, index, instruction, elements: prepared }
            }
            Plan::Insert {
                list,
                index,
                elements,
            } => {
                use tcl_registry::native_list_operations_compilation::{
                    c86_list_insertion_steps, NativeListInsertionStep as Step,
                };
                let steps = c86_list_insertion_steps(list, index, elements)
                    .ok_or_else(|| unavailable("original list insertion instruction"))?;
                let mut prepared = Vec::with_capacity(steps.len());
                for step in steps {
                    prepared.push(match step {
                        Step::Operand(operand) => InsertionStep::Operand(self.namespace_operand(
                            words,
                            &operand,
                            &mut prepared_words,
                            depth,
                        )?),
                        Step::List(count) => InsertionStep::List(count),
                        Step::Over(depth) => InsertionStep::Over(depth),
                        Step::Reverse(count) => InsertionStep::Reverse(count),
                        Step::Range(range) => InsertionStep::Range(range),
                        Step::Concat => InsertionStep::Concat,
                    });
                }
                ListOperation::Insert { steps: prepared }
            }
            _ => return Err((unavailable("original list replacement storage transaction")).into()),
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
    ) -> Result<ListTarget, tcl_cmd_core::CmdError> {
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
            _ => return Err((unavailable("original list target geometry")).into()),
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
        let variable_original = super::native_list_conversion::BodyObjectOperation::capture(self)
            .map_err(|error| self.report_cmd_error(error))?;
        let actual = self.clone();
        let current = || variable_original.check(&actual);
        let index_current = |actual: &Interp| variable_original.check(actual);
        let protocol = NativeStringProtocol::C(artifact.stamp.physical);
        let reached = (|| -> Result<Option<obj::Owned>, Code> {
            let result = match &recipe.operation {
                ListOperation::Range { list, range } => {
                    let original =
                        self.body_namespace_operand(artifact, command, list, execution)?;
                    current()
                        .map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
                    if execution.done {
                        return Ok(None);
                    }
                    crate::list::native_list_range_using(
                        original.as_ptr(),
                        *range,
                        protocol,
                        &current,
                        &|error| variable_original.refuse(error),
                    )
                    .map_err(|error| self.report_cmd_error_using(error, Some(&current)))?
                }
                ListOperation::ReplaceInsert {
                    list,
                    index,
                    instruction,
                    elements,
                } => {
                    use tcl_registry::native_list_operations_compilation::NativeListInsertionInstruction;
                    let expected = match instruction {
                        NativeListInsertionInstruction::ReplaceFour => {
                            tcl_dialect::TclVersion::V9_0
                        }
                        NativeListInsertionInstruction::Replace => tcl_dialect::TclVersion::V9_1,
                    };
                    if artifact.stamp.physical != expected {
                        return Err(self.report_cmd_error_using(
                            variable_original
                                .refuse(unavailable("original compiled list insertion release")),
                            Some(&current),
                        ));
                    }
                    let original =
                        self.body_namespace_operand(artifact, command, list, execution)?;
                    current()
                        .map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
                    if execution.done {
                        return Ok(None);
                    }
                    let index = self.body_namespace_operand(artifact, command, index, execution)?;
                    current()
                        .map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
                    if execution.done {
                        return Ok(None);
                    }
                    let mut values = Vec::with_capacity(elements.len());
                    for operand in elements {
                        values.push(
                            self.body_namespace_operand(artifact, command, operand, execution)?,
                        );
                        current()
                            .map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
                        if execution.done {
                            return Ok(None);
                        }
                    }
                    let pointers = std::iter::once(original.as_ptr())
                        .chain(std::iter::once(index.as_ptr()))
                        .chain(values.iter().map(obj::Owned::as_ptr))
                        .collect::<Vec<_>>();
                    let headers =
                        crate::interp::native_arguments::OriginalArguments::enter(self, &pointers)
                            .map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
                    let entered = || {
                        current()?;
                        headers.check_for(&actual)
                    };
                    let length = crate::value_ops::list_length_using(
                        self,
                        &original.as_ptr(),
                        &mut |actual, _| {
                            variable_original.check(actual)?;
                            headers.check_for(actual)
                        },
                        |actual, original| {
                            crate::list::list_elements_native_using(
                                *original,
                                protocol,
                                &|| {
                                    variable_original.check(actual)?;
                                    headers.check_for(actual)
                                },
                                &|error| variable_original.refuse(error),
                            )
                        },
                    )
                    .map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
                    let end = i64::try_from(length).expect("native compiled list length");
                    let index_guard = |actual: &Interp| {
                        variable_original.check(actual)?;
                        headers.check_for(actual)
                    };
                    let position = tcl_cmd_core::native_list_index::original_index_using(
                        self,
                        &index.as_ptr(),
                        end,
                        true,
                        Some(&index_guard),
                    )
                    .map_err(|error| self.report_cmd_error_using(error, Some(&current)))?
                    .expect("reported original insertion index");
                    entered()
                        .map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
                    let position = usize::try_from(position.max(0))
                        .unwrap_or(usize::MAX)
                        .min(length);
                    let replacements = values.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
                    let reached = crate::list::replace_prepared_native_range_original_using(
                        original.as_ptr(),
                        position,
                        0,
                        &replacements,
                        protocol,
                        &entered,
                        &|error| variable_original.refuse(error),
                    );
                    let reached =
                        reached.map_err(|error| self.report_cmd_error_using(error, Some(&current)));
                    drop(headers);
                    drop(values);
                    drop(index);
                    drop(original);
                    current()
                        .map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
                    reached?
                }
                ListOperation::Insert { steps } => {
                    let mut stack = Vec::<obj::Owned>::new();
                    for step in steps {
                        current()
                            .map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
                        match step {
                            InsertionStep::Operand(operand) => {
                                stack.push(self.body_namespace_operand(
                                    artifact, command, operand, execution,
                                )?);
                                current().map_err(|error| {
                                    self.report_cmd_error_using(error, Some(&current))
                                })?;
                                if execution.done {
                                    return Ok(None);
                                }
                            }
                            InsertionStep::List(count) => {
                                let values = stack.split_off(
                                    stack
                                        .len()
                                        .checked_sub(*count)
                                        .expect("native insertion List stack"),
                                );
                                let pointers =
                                    values.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
                                stack.push(obj::Owned::fresh(crate::list::new_list_obj_native(
                                    &pointers, protocol,
                                )));
                            }
                            InsertionStep::Over(depth) => {
                                let index = stack
                                    .len()
                                    .checked_sub(depth + 1)
                                    .expect("native insertion OVER stack");
                                stack.push(obj::Owned::retain(stack[index].as_ptr()));
                            }
                            InsertionStep::Reverse(count) => {
                                let start = stack
                                    .len()
                                    .checked_sub(*count)
                                    .expect("native insertion REVERSE stack");
                                stack[start..].reverse();
                            }
                            InsertionStep::Range(range) => {
                                let original = stack.pop().expect("native insertion range stack");
                                let headers =
                                    crate::interp::native_arguments::OriginalArguments::enter(
                                        self,
                                        &[original.as_ptr()],
                                    )
                                    .map_err(|error| {
                                        self.report_cmd_error_using(error, Some(&current))
                                    })?;
                                let reached = crate::list::native_list_range_using(
                                    original.as_ptr(),
                                    *range,
                                    protocol,
                                    &|| {
                                        current()?;
                                        headers.check_for(&actual)
                                    },
                                    &|error| variable_original.refuse(error),
                                );
                                let reached = reached.map_err(|error| {
                                    self.report_cmd_error_using(error, Some(&current))
                                });
                                drop(headers);
                                drop(original);
                                current().map_err(|error| {
                                    self.report_cmd_error_using(error, Some(&current))
                                })?;
                                stack.push(reached?);
                            }
                            InsertionStep::Concat => {
                                let source = stack.pop().expect("native insertion List source");
                                let target = stack.pop().expect("native insertion List target");
                                let headers =
                                    crate::interp::native_arguments::OriginalArguments::enter(
                                        self,
                                        &[target.as_ptr(), source.as_ptr()],
                                    )
                                    .map_err(|error| {
                                        self.report_cmd_error_using(error, Some(&current))
                                    })?;
                                let reached = crate::list::concatenate_native_lists_using(
                                    target.as_ptr(),
                                    source.as_ptr(),
                                    protocol,
                                    &|| {
                                        current()?;
                                        headers.check_for(&actual)
                                    },
                                    &|error| variable_original.refuse(error),
                                );
                                let reached = reached.map_err(|error| {
                                    self.report_cmd_error_using(error, Some(&current))
                                });
                                drop(headers);
                                drop(source);
                                drop(target);
                                current().map_err(|error| {
                                    self.report_cmd_error_using(error, Some(&current))
                                })?;
                                stack.push(reached?);
                            }
                        }
                    }
                    debug_assert_eq!(stack.len(), 1);
                    stack.pop().expect("native insertion result")
                }
                ListOperation::Assign { list, targets } => {
                    let original =
                        self.body_namespace_operand(artifact, command, list, execution)?;
                    current()
                        .map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
                    if execution.done {
                        return Ok(None);
                    }
                    for (index, target) in targets.iter().enumerate() {
                        let evaluated =
                            self.body_list_target(artifact, command, target, execution)?;
                        current()
                            .map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
                        if execution.done {
                            return Ok(None);
                        }
                        // Genuine DUP/OVER stack role stays before the original getter.
                        let duplicate = obj::Owned::retain(original.as_ptr());
                        let member = tcl_cmd_core::native_list_index::immediate_using(
                            self,
                            &duplicate.as_ptr(),
                            Index::from_encoded(i32::try_from(index).expect("native target count")),
                            Some(&index_current),
                        )
                        .map_err(|error| self.report_cmd_error_using(error, Some(&current)));
                        drop(duplicate);
                        current()
                            .map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
                        let member = member?;
                        drop(self.body_store_value_using(
                            &evaluated,
                            target.slot(),
                            member.owned(),
                            &variable_original,
                        )?);
                        current()
                            .map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
                        drop(member);
                        current()
                            .map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
                    }
                    crate::list::native_list_range_using(
                        original.as_ptr(),
                        NativeCompiledListRange {
                            first: Index::from_encoded(
                                i32::try_from(targets.len()).expect("native target count"),
                            ),
                            last: Index::from_encoded(-2),
                        },
                        protocol,
                        &current,
                        &|error| variable_original.refuse(error),
                    )
                    .map_err(|error| self.report_cmd_error_using(error, Some(&current)))?
                }
            };
            Ok(Some(result))
        })();
        // All actual operand/target/DUP roles retire before result publication.
        if let Err(first) = current() {
            let first = variable_original.retain(first);
            drop(reached);
            let _ = current();
            return Err(self.report_cmd_error_using(first, Some(&current)));
        }
        let Some(result) = reached? else {
            return Ok(Code::Ok);
        };
        self.set_result(result.as_ptr());
        let first = current().map_err(|error| variable_original.retain(error));
        drop(result);
        let settled = current();
        first
            .and(settled)
            .map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
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

    #[test]
    fn original_c86_insertion_matches_five_native_result_and_original_header_pairs() {
        let rows = include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_list_operations/8.6.18.txt"
        );
        let mut windows = 0;
        for line in rows.lines().filter(|line| line.starts_with("R|")) {
            let result = line.split('|').collect::<Vec<_>>();
            let case = result[1].parse::<usize>().unwrap();
            if ![5, 6, 7, 8, 29].contains(&case) {
                continue;
            }
            let prefix = format!("O|{case}|");
            let original = rows
                .lines()
                .find(|line| line.starts_with(&prefix))
                .unwrap()
                .split('|')
                .collect::<Vec<_>>();
            run("tcl8.6", CASES[case], &result, &original);
            windows += 1;
        }
        assert_eq!(windows, 5);
    }

    #[test]
    fn original_c9_insertion_matches_fourteen_native_result_and_original_header_pairs() {
        let mut windows = 0;
        for &(engine, rows, _) in LIST_TABLES {
            if !["tcl9.0", "tcl9.1"].contains(&engine) {
                continue;
            }
            for line in rows.lines().filter(|line| line.starts_with("R|")) {
                let result = line.split('|').collect::<Vec<_>>();
                let case = result[1].parse::<usize>().unwrap();
                if ![5, 6, 7, 8, 9, 10, 29].contains(&case) {
                    continue;
                }
                let prefix = format!("O|{case}|");
                let original = rows
                    .lines()
                    .find(|line| line.starts_with(&prefix))
                    .unwrap()
                    .split('|')
                    .collect::<Vec<_>>();
                run(engine, CASES[case], &result, &original);
                windows += 1;
            }
        }
        assert_eq!(windows, 14);
    }
}

#[cfg(test)]
#[path = "native_list_operations/original_boundary_tests.rs"]
mod original_boundary_tests;
