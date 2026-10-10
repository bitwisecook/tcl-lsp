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

//! Original native coroutine worklist, without reconstructed operand strings.

use super::{CodegenCtx, NativeEmissionTask, Op, Operand};
use tcl_registry::native_coroutine_compilation::{NativeCoroutineInstruction, NativeCoroutineStep};

impl CodegenCtx<'_> {
    pub(super) fn native_coroutine_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: NativeCoroutineInstruction,
    ) -> Option<Vec<NativeEmissionTask>> {
        use NativeEmissionTask as Task;
        recipe.steps.into_iter().map(|step| Some(match step {
            NativeCoroutineStep::Word(operand) => Self::native_namespace_word_task(&command.words, operand)?,
            NativeCoroutineStep::CommandWord { bytes, .. } => {
                let entry = self.native_entry?;
                let literal = tcl_registry::native_command_literal::native_compiled_command_name_literal(entry, &bytes).ok()?;
                Task::NativeCommandLiteral(Box::new(literal))
            }
            NativeCoroutineStep::Empty => Task::Literal(Vec::new()),
            NativeCoroutineStep::CurrentNamespace => Task::Operation(Op::CURRENT_NAMESPACE, vec![]),
            NativeCoroutineStep::List(count) => Task::Operation(Op::LIST, vec![Operand::Imm(i32::try_from(count).ok()?)]),
            NativeCoroutineStep::Concat => Task::Operation(Op::LIST_CONCAT, vec![]),
            NativeCoroutineStep::Tailcall { count, legacy } => Task::Operation(if legacy { Op::TAILCALL } else { Op::TAILCALL4 }, vec![Operand::Imm(i32::try_from(count).ok()?)]),
            NativeCoroutineStep::TailcallList => Task::Operation(Op::TAILCALL_LIST, vec![]),
            NativeCoroutineStep::Yield => Task::Operation(Op::YIELD, vec![]),
            NativeCoroutineStep::YieldTo => Task::Operation(Op::YIELD_TO_INVOKE, vec![]),
            NativeCoroutineStep::Name => Task::Operation(Op::CORO_NAME, vec![]),
        })).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{LexerConfig, SourceImage, Span};

    #[test]
    fn original_coroutine_emission_keeps_native_head_namespace_and_operand_order() {
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(dialect).unwrap();
            let registry =
                tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
            let entry = crate::environment_ingress::captured_native_entry(profile);
            for (source, terminal) in [
                (
                    b"tailcall target [set seen VALUE]".as_slice(),
                    if dialect == "tcl9.1" {
                        Op::TAILCALL4
                    } else {
                        Op::TAILCALL
                    },
                ),
                (b"yield [set seen VALUE]", Op::YIELD),
                (b"yieldto target [set seen VALUE]", Op::YIELD_TO_INVOKE),
            ] {
                let parsed = tcl_lexer::native_script_words_in(
                    SourceImage::native(source),
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
                assert!(!context.native_dependency_refusal, "{dialect}/{terminal:?}");
                let ops = context
                    .instructions
                    .iter()
                    // Portable source boundaries may carry metadata-only NOPs.
                    // Compare the operations which prepare/evaluate native operands.
                    .map(|instruction| instruction.op)
                    .filter(|op| *op != Op::NOP)
                    .collect::<Vec<_>>();
                assert!(ops.contains(&terminal), "{dialect}: {ops:?}");
                if terminal == Op::TAILCALL {
                    assert!(
                        !ops.contains(&Op::CURRENT_NAMESPACE),
                        "{dialect}: no namespace object before native execution"
                    );
                    assert!(
                        context
                            .literals
                            .entries()
                            .iter()
                            .any(|literal| literal == "tailcall")
                    );
                } else if terminal == Op::TAILCALL4 || terminal == Op::YIELD_TO_INVOKE {
                    assert_eq!(
                        ops.first(),
                        Some(&Op::CURRENT_NAMESPACE),
                        "{dialect}: {ops:?}"
                    );
                }
            }
        }
    }
}
