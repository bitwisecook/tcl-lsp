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

//! Inline emitters that carry native argument lists: `error`, `yield`, `yieldto`, expanded native calls, procedure no-ops and named invocations.

use super::super::helpers::parse_subst_template;
use super::super::{CodegenCtx, Op, Operand, bytecode_imm};
use tcl_compiler::ir::{CommandTokens, WordExpr};
use tcl_registry::hooks::InlineCodegenHookId;

impl CodegenCtx<'_> {
    // Private inline helpers for emit_inline_cmd_subst.

    /// Native Error uses the same return-options stack protocol as returnImm,
    /// preserving optional errorInfo/errorCode argv and their evaluation order.
    pub(in super::super) fn emit_inline_error(
        &mut self,
        args: &[(String, bool)],
        source_command: &str,
    ) -> bool {
        use tcl_registry::native_error_compilation::{NativeErrorStep, native_error_instruction};
        let Some(version) = self
            .native_hook_dialect()
            .and_then(|dialect| dialect.tcl_version)
        else {
            return false;
        };
        let operands = (0..args.len())
            .map(tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original)
            .collect::<Vec<_>>();
        let Ok(recipe) = native_error_instruction(&operands, version) else {
            return false;
        };
        self.used_inline_cmd_subst = true;
        for step in recipe.steps {
            match step {
                NativeErrorStep::Word(tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original(index)) => self.emit_native_argument_word(index, &args[index].0, args[index].1),
                NativeErrorStep::Word(_) => unreachable!("selected original argv operands"),
                NativeErrorStep::Literal(value) => { self.push_lit_bytes_exact(&value); }
                NativeErrorStep::List(count) => { self.emit(Op::LIST, vec![Operand::Imm(i32::try_from(count).expect("native Error pair count"))]); }
                NativeErrorStep::DictionaryPut => { self.emit(Op::DICT_PUT, vec![]); }
                NativeErrorStep::ReturnError => self.emit_native_error_return(source_command),
                NativeErrorStep::Pop => { self.emit(Op::POP, vec![]); }
                NativeErrorStep::ThrowTypeCheck => unreachable!("Error recipes carry no throw type check"),
            }
        }
        true
    }

    pub(in super::super) fn emit_inline_yield(&mut self, args: &[(String, bool)]) {
        self.used_inline_cmd_subst = true;
        if let Some((word, braced)) = args.first() {
            self.emit_native_argument_word(0, word, *braced);
        } else {
            self.push_lit_exact("");
        }
        self.emit(Op::YIELD, vec![]);
    }

    /// Native relay list construction preserves namespace capture before argv
    /// evaluation and Tcl 9.1's expansion boundaries.
    pub(in super::super) fn emit_inline_yield_to<'a>(
        &mut self,
        words: impl IntoIterator<Item = (&'a str, bool, bool)>,
    ) {
        self.used_inline_cmd_subst = true;
        self.emit(Op::CURRENT_NAMESPACE, vec![]);
        self.emit_native_argument_list(words, 0, 1);
        self.emit(Op::YIELD_TO_INVOKE, vec![]);
    }

    /// Build a native list from original argument words and expansion segments.
    /// `arguments_from` retains the hook's operand map; `prefix_count` counts
    /// already evaluated stack values included before those arguments.
    pub(in super::super) fn emit_native_argument_list<'a>(
        &mut self,
        words: impl IntoIterator<Item = (&'a str, bool, bool)>,
        arguments_from: usize,
        prefix_count: usize,
    ) {
        let segment_limit = self.native_hook_dialect().and_then(
            tcl_registry::native_compilation::NativeTailcallStack::argument_list_segment_limit,
        );
        let mut pending = prefix_count;
        let mut concatenated = false;
        for (index, (word, braced, expanded)) in words.into_iter().enumerate() {
            if expanded && pending > 0 {
                self.emit(Op::LIST, vec![Operand::Imm(bytecode_imm(pending))]);
                if concatenated {
                    self.emit(Op::LIST_CONCAT, vec![]);
                }
                pending = 0;
                concatenated = true;
            }
            self.emit_native_argument_word(arguments_from + index, word, braced);
            if expanded {
                if concatenated {
                    self.emit(Op::LIST_CONCAT, vec![]);
                }
                concatenated = true;
            } else {
                pending += 1;
                if segment_limit.is_some_and(|limit| pending > limit) {
                    self.emit(Op::LIST, vec![Operand::Imm(bytecode_imm(pending))]);
                    if concatenated {
                        self.emit(Op::LIST_CONCAT, vec![]);
                    }
                    pending = 0;
                    concatenated = true;
                }
            }
        }
        if pending > 0 {
            self.emit(Op::LIST, vec![Operand::Imm(bytecode_imm(pending))]);
            if concatenated {
                self.emit(Op::LIST_CONCAT, vec![]);
            }
        }
        if pending == 0 && !concatenated {
            self.emit(Op::LIST, vec![Operand::Imm(0)]);
        }
    }

    pub(in super::super) fn emit_native_argument_word(
        &mut self,
        index: usize,
        text: &str,
        braced: bool,
    ) {
        let word = self.original_hook_argument(index).and_then(|original| {
            self.invocation_tokens
                .as_deref()?
                .words()
                .get(original + 1)
                .cloned()
        });
        // `text` is the written word as the substituted script spells it
        // (`{}$z`); the original word's compatibility text spells its
        // variables `${z}`. The compiled-word convention reads a bare `$` as
        // data, so the written spelling must never reach the word path: there
        // `string length "{}$z"` pushed the four bytes `{}$z` unsubstituted.
        // Use the original word only when both spellings name the same parts,
        // and then in its own convention; otherwise the written text compiles
        // through the substituted-script word path.
        let word = word.filter(|word| {
            let legacy = word.legacy_text();
            legacy == text
                || (!braced
                    && parse_subst_template(&legacy, self.lexer_config()).is_some_and(|parts| {
                        parse_subst_template(text, self.lexer_config()).as_ref() == Some(&parts)
                    }))
        });
        if let Some(word) = word {
            self.emit_word_from_source(&word.legacy_text(), braced, Some(&word));
        } else {
            self.emit_cmd_subst_arg(text, braced);
        }
    }

    pub(in super::super) fn try_emit_expanded_native_call(
        &mut self,
        parts: &[(String, bool, bool)],
    ) -> bool {
        let Some(tokens) = self.invocation_tokens.as_deref() else {
            return false;
        };
        if tokens.source_binding.is_none() {
            let dialect = tcl_compiler::environment_ingress::authoring_invocation_dialect(
                self.registry,
                self.dialect,
                self.lexer_config(),
            );
            let Some((
                _,
                tcl_registry::native_compilation::NativeCompilationSelection::Inline { .. },
            )) = tcl_compiler::registry_invocation::native_compilation_syntax(
                self.registry,
                tokens,
                dialect,
                self.native_compilation,
            )
            else {
                return false;
            };
        }
        let Some((head, _, false)) = parts.first() else {
            return false;
        };
        let args = parts[1..]
            .iter()
            .map(|(word, _, _)| word.as_str())
            .collect::<Vec<_>>();
        if super::super::emitter::bytecoded::try_expanded_tailcall(self, head, &parts[1..]) {
            return true;
        }
        if super::super::emitter::bytecoded::try_expanded_lappend(self, head, &parts[1..]) {
            self.used_inline_cmd_subst = true;
            return true;
        }
        let Some((InlineCodegenHookId::YieldTo, binding)) =
            self.inline_codegen_resolution(head, &args)
        else {
            return false;
        };
        self.emit_inline_yield_to(
            parts[1..]
                .iter()
                .map(|(word, braced, expanded)| (word.as_str(), *braced, *expanded)),
        );
        // A run-time expansion leaves the words after the command unknown.
        self.require_site_binding(
            &binding,
            &tcl_registry::release_compilation::InvocationShape::default(),
        );
        true
    }

    pub(super) fn emit_inline_incr(&mut self, args: &[(String, bool)]) {
        let Some((name, braced)) = args.first() else {
            return;
        };
        let amount = args.get(1);
        self.emit_incr(
            name,
            *braced,
            amount.map(|(amount, _)| amount.as_str()),
            amount.is_some_and(|(_, braced)| *braced),
        );
    }

    /// One retained compiler selection and original-command replay boundary.
    fn emit_native_compiler_selection(
        &mut self,
        source: &str,
        namespace: &str,
        namespace_context: Option<tcl_runtime_api::CompiledNamespaceContext>,
        prerequisite: tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite,
    ) -> String {
        let selection = self.emit(Op::NOP, vec![]);
        let end = self.fresh_label("native_selection_end");
        self.instructions[selection].source_cmd_text =
            tcl_lexer::SourceImage::from_bytes(source.as_bytes(), self.source_image().channel());
        self.instructions[selection].source_command_namespace =
            namespace_context.as_ref().map_or_else(
                || super::super::namespace_path_from_constructed_key(namespace),
                |context| context.path().clone(),
            );
        self.instructions[selection].source_command_namespace_context = namespace_context;
        self.instructions[selection].source_command_boundary =
            tcl_bytecode::SourceCommandBoundary::Start;
        self.instructions[selection].no_fold = true;
        self.instructions[selection].native_compiler_selection =
            Some(tcl_bytecode::NativeCompilerSelectionSite {
                prerequisite,
                end: end.clone(),
            });
        end
    }

    /// Execute every original operand, then produce the result selected by the
    /// actual native empty variadic procedure compiler. No procedure frame or
    /// late invocation is introduced by Tcl's `NoOp` compiler.
    pub(in super::super) fn emit_native_procedure_noop(
        &mut self,
        tokens: Option<&CommandTokens>,
    ) -> bool {
        if self.plain_command_dispatch {
            return false;
        }
        let Some(tokens) = tokens else { return false };
        let Some(plan) = tcl_compiler::registry_invocation::native_procedure_noop_plan(tokens)
        else {
            return false;
        };
        let end = self.emit_native_compiler_selection(
            &plan.source,
            &plan.namespace,
            plan.namespace_context,
            tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::ProcedureHeader(Box::new(plan.prerequisite)),
        );
        for (index, word) in tokens.words().iter().enumerate().skip(1) {
            let braced = tokens.argv_kinds.get(index) == Some(&tcl_lexer::TokenType::Str)
                && tokens
                    .single_token_word
                    .get(index)
                    .copied()
                    .unwrap_or(false);
            self.emit_word_from_source(&tokens.argv_texts[index], braced, Some(word));
            self.emit(Op::POP, vec![]);
        }
        self.push_lit_exact("");
        self.place_label(&end);
        self.used_inline_cmd_subst = true;
        true
    }

    /// Emit a captured private name followed by late ordinary handler lookup.
    /// Source substitutions execute once, in original order, without capturing
    /// an entered token or substituting a canonical handler for the private name.
    pub(in super::super) fn emit_native_named_invocation(
        &mut self,
        tokens: Option<&CommandTokens>,
    ) -> bool {
        if self.plain_command_dispatch {
            return false;
        }
        let Some(tokens) = tokens else { return false };
        let Some(binding) = tokens.source_binding.as_ref() else {
            return false;
        };
        let Some(named) = binding.admitted_named_invocation() else {
            return false;
        };
        let Some((image, originals, offset)) = binding.original_compiler_source(tokens) else {
            return false;
        };
        let Some(dialect) = binding.native_compiler_dialect() else {
            return false;
        };
        let Some(protocol) = binding.compiler_source_protocol() else {
            return false;
        };
        let Some(originals) = tcl_compiler::registry_invocation::original_native_compiler_words(
            image,
            originals,
            offset,
            tokens.native_lexer_config(self.lexer_config()),
        ) else {
            return false;
        };
        let Ok(captured) =
            tcl_registry::native_compiler_words::NativeCompilerWords::capture(&originals, protocol)
        else {
            return false;
        };
        let replacements: Vec<_> = named
            .replacement_words
            .iter()
            .map(|word| word.as_bytes().to_vec())
            .collect();
        let Ok(recipe) = tcl_registry::native_instruction_plan::native_named_invocation_instruction(
            &captured,
            dialect,
            named.captured_name().as_bytes(),
            named.arguments_from,
            named.protocol,
            &replacements,
        ) else {
            return false;
        };
        // INVOKE_REPLACE has a fixed original word count. The genuine compiler
        // selects ordinary dispatch when a rewrite still has dynamic expansion.
        if recipe.protocol
            == tcl_registry::native_compilation::NativeNamedInvocationProtocol::EnsembleRewrite
            && recipe.expanded.iter().any(|expanded| *expanded)
        {
            return false;
        }
        let selection_end = if let Some(required) = &named.compiler_prerequisite {
            self.retain_entry_named_compiler_prerequisite(required);
            let Some(source) = tcl_compiler::registry_invocation::native_compiler_replay_source(
                tokens,
                &named.compilation_site,
            ) else {
                return false;
            };
            let end = self.emit_native_compiler_selection(
                &source,
                &tokens.source_binding.as_ref().unwrap().lookup_namespace,
                tcl_compiler::registry_invocation::compiled_namespace_context(tokens),
                tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::from_command_registration(
                    std::sync::Arc::clone(required),
                ),
            );
            Some(end)
        } else {
            None
        };
        if !self.emit_native_original_preparation_receipt(&named.preparations) {
            return false;
        }
        self.emit_named_instruction(tokens, named, &recipe);
        if let Some(end) = selection_end {
            self.place_label(&end);
        }
        self.used_inline_cmd_subst = false;
        true
    }

    fn emit_named_instruction(
        &mut self,
        tokens: &CommandTokens,
        named: &tcl_compiler::command_binding::SourceNamedInvocationProof,
        recipe: &tcl_registry::native_instruction_plan::NativeNamedInvocationInstruction,
    ) {
        use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand;
        use tcl_registry::native_instruction_plan::NativeNamedInvocationWord;
        let rewrite = recipe.protocol
            == tcl_registry::native_compilation::NativeNamedInvocationProtocol::EnsembleRewrite;
        let expanding = recipe.expanded.iter().any(|expanded| *expanded);
        if expanding {
            self.emit(Op::EXPAND_START, vec![]);
        }
        if !rewrite {
            self.emit_selected_native_command_name(tokens, named);
        }
        for (position, (word, expanded)) in recipe.words.iter().zip(&recipe.expanded).enumerate() {
            match word {
                NativeNamedInvocationWord::Replacement(bytes)
                | NativeNamedInvocationWord::Original(
                    NativeCompilerWordOperand::LiteralExpansion { value: bytes, .. },
                ) => self.push_lit_bytes_exact(bytes),
                NativeNamedInvocationWord::Original(NativeCompilerWordOperand::Original(index)) => {
                    let original = &tokens.words()[*index];
                    let payload = match original {
                        WordExpr::Expand { word, .. } => word.as_ref(),
                        original => original,
                    };
                    let braced = tokens.argv_kinds.get(*index) == Some(&tcl_lexer::TokenType::Str)
                        && tokens
                            .single_token_word
                            .get(*index)
                            .copied()
                            .unwrap_or(false);
                    self.emit_word_from_source(&tokens.argv_texts[*index], braced, Some(payload));
                }
            }
            if *expanded {
                self.emit(
                    Op::EXPAND_STKTOP,
                    vec![Operand::Imm(bytecode_imm(position + 2))],
                );
            }
        }
        if rewrite {
            self.emit_selected_native_command_name(tokens, named);
            self.emit(
                Op::INVOKE_REPLACE,
                vec![
                    Operand::Imm(bytecode_imm(recipe.words.len())),
                    Operand::Imm(bytecode_imm(recipe.arguments_from + 1)),
                ],
            );
        } else if expanding {
            self.emit(Op::INVOKE_EXPANDED, vec![]);
        } else {
            let argc = bytecode_imm(recipe.words.len() + 1);
            self.emit(
                if argc < 256 {
                    Op::INVOKE_STK1
                } else {
                    Op::INVOKE_STK4
                },
                vec![Operand::Imm(argc)],
            );
        }
    }
}
