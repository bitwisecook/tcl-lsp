// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared native Error instruction preparation and original error context.

use super::{CodegenCtx, NativeEmissionTask};
use tcl_registry::native_error_compilation::{NativeErrorInstruction, NativeErrorStep};

impl CodegenCtx<'_> {
    pub(super) fn native_error_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: NativeErrorInstruction,
    ) -> Option<Vec<NativeEmissionTask>> {
        use NativeEmissionTask as Task;
        recipe
            .steps
            .into_iter()
            .map(|step| {
                Some(match step {
                    NativeErrorStep::Word(operand) => {
                        Self::native_namespace_word_task(&command.words, operand)?
                    }
                    NativeErrorStep::Literal(value) => Task::Literal(value),
                    NativeErrorStep::List(count) => Task::Operation(
                        super::Op::LIST,
                        vec![super::Operand::Imm(i32::try_from(count).ok()?)],
                    ),
                    NativeErrorStep::DictionaryPut => Task::Operation(super::Op::DICT_PUT, vec![]),
                    NativeErrorStep::ReturnError => Task::ErrorReturn,
                })
            })
            .collect()
    }

    pub(in crate::codegen) fn emit_native_error_return(&mut self, source_command: &str) {
        let instruction = self.emit(
            super::Op::RETURN_IMM,
            vec![super::Operand::Imm(1), super::Operand::Imm(0)],
        );
        let source_command = if source_command.is_empty() {
            self.instructions[instruction]
                .source_cmd_text
                .bytes()
                .to_vec()
        } else {
            source_command.trim().as_bytes().to_vec()
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
            let entry = crate::environment_ingress::captured_native_entry(profile);
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
            let entry = crate::environment_ingress::captured_native_entry(profile);
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
