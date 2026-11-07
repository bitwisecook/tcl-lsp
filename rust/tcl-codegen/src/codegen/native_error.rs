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

//! Shared native Error instruction preparation and original error context.

use super::{CodegenCtx, NativeEmissionTask};
use tcl_registry::native_error_compilation::{NativeErrorInstruction, NativeErrorStep};

impl CodegenCtx<'_> {
    pub(super) fn native_error_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: NativeErrorInstruction,
    ) -> Option<Vec<NativeEmissionTask>> {
        use NativeEmissionTask as Task;
        let mut tasks = Vec::with_capacity(recipe.steps.len());
        for step in recipe.steps {
            match step {
                NativeErrorStep::Word(operand) => {
                    tasks.push(Self::native_namespace_word_task(&command.words, operand)?);
                }
                NativeErrorStep::Literal(value) => tasks.push(Task::Literal(value)),
                NativeErrorStep::List(count) => tasks.push(Task::Operation(
                    super::Op::LIST,
                    vec![super::Operand::Imm(i32::try_from(count).ok()?)],
                )),
                NativeErrorStep::DictionaryPut => {
                    tasks.push(Task::Operation(super::Op::DICT_PUT, vec![]));
                }
                NativeErrorStep::ReturnError => tasks.push(Task::ErrorReturn),
                NativeErrorStep::Pop => tasks.push(Task::Operation(super::Op::POP, vec![])),
                NativeErrorStep::ThrowTypeCheck => {
                    // TclCompileThrowCmd over `type -errorcode msg`.
                    let empty = self.fresh_label("throw_empty_type");
                    tasks.extend([
                        Task::Operation(super::Op::REVERSE, vec![super::Operand::Imm(3)]),
                        Task::Operation(super::Op::DUP, vec![]),
                        Task::Operation(super::Op::LIST_LENGTH, vec![]),
                        Task::Operation(
                            super::Op::JUMP_FALSE1,
                            vec![super::Operand::Label(empty.clone())],
                        ),
                        Task::Operation(super::Op::LIST, vec![super::Operand::Imm(2)]),
                        Task::ErrorReturn,
                        Task::Label(empty),
                        Task::Operation(super::Op::POP, vec![]),
                        Task::Operation(super::Op::POP, vec![]),
                        Task::Operation(super::Op::POP, vec![]),
                        Task::Literal(
                            tcl_registry::native_error_compilation::THROW_EMPTY_TYPE_MESSAGE
                                .to_vec(),
                        ),
                        Task::Literal(
                            tcl_registry::native_error_compilation::THROW_EMPTY_TYPE_OPTIONS
                                .to_vec(),
                        ),
                    ]);
                }
            }
        }
        Some(tasks)
    }

    pub(in crate::codegen) fn emit_native_error_return(&mut self, source_command: &str) {
        let instruction = self.emit(
            super::Op::RETURN_IMM,
            vec![super::Operand::Imm(1), super::Operand::Imm(0)],
        );
        let source_command = if source_command.is_empty() {
            // The command through the whitespace before its terminator, as
            // C's `errorInfo` names it (`error_info_source_text`).
            match self
                .current_span
                .filter(|_| !self.instructions[instruction].source_cmd_text.is_empty())
            {
                Some(span) => self.error_info_source_text(span).bytes().to_vec(),
                None => self.instructions[instruction]
                    .source_cmd_text
                    .bytes()
                    .to_vec(),
            }
        } else {
            // C logs the command through its terminator: a trailing space
            // stays in `errorInfo`, only the leading separators go.
            source_command.trim_start().as_bytes().to_vec()
        };
        self.instructions[instruction].error_stack_context =
            Some(tcl_bytecode::ErrorStackContext::ReturnImmediate {
                error_info_command: source_command,
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{LexerConfig, SourceImage, Span};

    #[test]
    fn original_error_emission_matches_native_options_and_local_order() {
        let sources = [
            b"error BODY".as_slice(),
            b"error [set message BODY] [set info STACK]",
            b"error [set message BODY] [set info STACK] [set code {FOO BAR}]",
            b"error {*}{BODY STACK {FOO BAR}}",
        ];
        for (profile, rows) in [
            (
                "tcl8.6",
                include_str!("../../../tcl-registry/tests/data/native_error_compilation/8.6.tsv"),
            ),
            (
                "tcl9.0",
                include_str!("../../../tcl-registry/tests/data/native_error_compilation/9.0.tsv"),
            ),
            (
                "tcl9.1",
                include_str!("../../../tcl-registry/tests/data/native_error_compilation/9.1.tsv"),
            ),
        ] {
            let profile = tcl_dialect::DialectProfile::find(profile).unwrap();
            let registry =
                tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
            let entry: crate::test_support::CapturedNativeEntry =
                crate::test_support::captured_native_entry(profile);
            for (index, source) in sources.iter().enumerate() {
                let parsed = tcl_lexer::native_script_words_in(
                    SourceImage::native(*source),
                    Span::new(0, u32::try_from(source.len()).unwrap()),
                    LexerConfig::from_grammar(profile.grammar),
                )
                .unwrap();
                let mut context = CodegenCtx::new(true, &[], &registry);
                context.native_entry = Some(&entry);
                context.invocation_dialect =
                    Some(tcl_registry::InvocationDialect::of_profile(profile));
                context.source_string_protocol = entry.source_string_protocol;
                context.native_compilation.frame =
                    tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode;
                context.emit_native_words(&parsed.commands[0].words);
                assert!(
                    !context.native_dependency_refusal,
                    "{} / {index}",
                    profile.name
                );
                let row = rows
                    .lines()
                    .nth(index + 1)
                    .unwrap()
                    .split('\t')
                    .collect::<Vec<_>>();
                let names = context
                    .lvt
                    .native_slot_names()
                    .into_iter()
                    .flatten()
                    .map(|name| name.as_bytes().to_vec())
                    .collect::<Vec<_>>();
                assert_eq!(
                    names,
                    row[3]
                        .split_whitespace()
                        .map(|name| name.as_bytes().to_vec())
                        .collect::<Vec<_>>()
                );
                let native_dict_count = row[4]
                    .split_whitespace()
                    .filter(|op| *op == "dictPut")
                    .count();
                assert_eq!(
                    context
                        .instructions
                        .iter()
                        .filter(|instruction| instruction.op == super::super::Op::DICT_PUT)
                        .count(),
                    native_dict_count
                );
                let finish = context
                    .instructions
                    .iter()
                    .find(|instruction| instruction.op == super::super::Op::RETURN_IMM)
                    .unwrap();
                assert_eq!(
                    finish.operands,
                    [super::super::Operand::Imm(1), super::super::Operand::Imm(0)]
                );
                assert!(finish.error_stack_context.is_some());
            }
        }
    }

    #[test]
    fn original_error_registration_is_absent_before_c86() {
        for profile in ["tcl8.4", "tcl8.5"] {
            let profile = tcl_dialect::DialectProfile::find(profile).unwrap();
            let entry: crate::test_support::CapturedNativeEntry =
                crate::test_support::captured_native_entry(profile);
            let registry =
                tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
            let source = b"error BODY";
            let parsed = tcl_lexer::native_script_words_in(
                SourceImage::native(source.as_slice()),
                Span::new(0, u32::try_from(source.len()).unwrap()),
                LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            let mut context = CodegenCtx::new(true, &[], &registry);
            context.native_entry = Some(&entry);
            context.source_string_protocol = entry.source_string_protocol;
            context.emit_native_words(&parsed.commands[0].words);
            assert!(!context.native_dependency_refusal);
            assert!(
                context
                    .instructions
                    .iter()
                    .all(|instruction| instruction.op != super::super::Op::RETURN_IMM)
            );
        }
    }
}
