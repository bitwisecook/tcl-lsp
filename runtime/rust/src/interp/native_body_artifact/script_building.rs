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

//! Building the instruction list of one script.

use super::super::NATIVE_EVAL_DEPTH_LIMIT;
use super::{
    unavailable, ArenaInstruction, Builder, CommandInstruction, NamespaceOperand,
    NamespaceOperation, Operation, Script, WordInstruction,
};
use std::collections::HashMap;
use tcl_lexer::Span;
use tcl_registry::native_compiler_words::{NativeCompiledListRecipe, NativeCompilerWords};
use tcl_syntax::native_variable_words::NativeVariableWordOperand;
use tcl_syntax::value::ValueError;

impl Builder<'_> {
    pub(super) fn script(
        &mut self,
        region: Span,
        depth: u32,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        if self.scripts.contains_key(&region) {
            return Ok(());
        }
        if NATIVE_EVAL_DEPTH_LIMIT.exceeded(depth) {
            return Err((unavailable("native body compiler script depth")).into());
        }
        let plan =
            tcl_lexer::native_script_words_in(self.image.clone(), region, self.stamp.grammar)
                .map_err(|_| unavailable("native body executable script geometry"))?;
        if plan.fatal_tail.is_some() && self.stamp.physical == tcl_dialect::TclVersion::V8_4 {
            // C8.4 has no executable parse-error instruction to publish.
            self.parse_failure = plan.fatal_tail;
            return Err(
                (ValueError::CommandProtocolUnavailable("native C8.4 failed body parse")).into(),
            );
        }
        let mut commands = Vec::with_capacity(plan.commands.len());
        for command in plan.commands {
            let Some(mut operation) = self.operation(&command.words, depth)? else {
                return Err((unavailable("native body registered instruction capability")).into());
            };
            let capture = NativeCompilerWords::capture(&command.words, self.stamp.source_protocol)
                .map_err(|_| unavailable("native body original word value geometry"))?;
            let mut words = Vec::with_capacity(command.words.len());
            for (index, original) in command.words.iter().enumerate() {
                if let Operation::Expression(expression) = &mut operation {
                    if let Some(word) = expression.prepared.words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::Try(control) = &mut operation {
                    if let Some(word) = control.prepared.words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::Control(control) = &mut operation {
                    if let Some(word) = control.prepared.words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::Each(each) = &mut operation {
                    if let Some(word) = each.prepared.words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::NamespaceBindings(scope) = &mut operation {
                    if let Some(word) = scope.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::Concat { prepared_words, .. }
                | Operation::FormatStringConcat { prepared_words, .. } = &mut operation
                {
                    if let Some(word) = prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::NamedInvocation(named) = &mut operation {
                    if let Some(word) = named.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::Coroutine(coroutine) = &mut operation {
                    if let Some(word) = coroutine.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::Error(error) = &mut operation {
                    if let Some(word) = error.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::StringTrim(trim) = &mut operation {
                    if let Some(word) = trim.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::StringRange(string) = &mut operation {
                    if let Some(word) = string.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::StringOperand(string) = &mut operation {
                    if let Some(word) = string.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::StringMatch(matcher) = &mut operation {
                    if let Some(word) = matcher.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::ListOperations(list) = &mut operation {
                    if let Some(word) = list.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::ListIndex(indexer) = &mut operation {
                    if let Some(word) = indexer.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::Array(array) = &mut operation {
                    if let Some(word) = array.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::Introspection(introspection) = &mut operation {
                    if let Some(word) = introspection.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::MathOperator(mathop) = &mut operation {
                    if let Some(word) = mathop.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::NamespaceString(recipe) = &mut operation {
                    if let Some(word) = recipe.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::Scalar(scalar) = &mut operation {
                    if let Some(word) = scalar.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::DictionaryScope(dictionary) = &mut operation {
                    if let Some(word) = dictionary.prepared.words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::DictionaryMutation(dictionary) = &mut operation {
                    if let Some(word) = dictionary.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::DictionaryLookup(dictionary) = &mut operation {
                    if let Some(word) = dictionary.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::Upvar(upvar) = &mut operation {
                    if let Some(word) = upvar.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::InfoExists(exists) = &mut operation {
                    if let Some(word) = exists.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::Unset(unset) = &mut operation {
                    if let Some(word) = unset.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                // Opcode variable operands are not emitted as ordinary argument
                // objects. Their indexed receiver layout was allocated above.
                let emitted = match &operation {
                    Operation::StringMatch(_) | Operation::StringTrim(_) | Operation::StringOperand(_) | Operation::StringRange(_) => false,
                    Operation::ListIndex(_) | Operation::ListOperations(_) => false,
                    Operation::NamespaceString(_) | Operation::MathOperator(_) | Operation::Scalar(_) | Operation::Introspection(_) | Operation::Array(_) => false,
                    Operation::Upvar(_) | Operation::InfoExists(_) => false,
                    Operation::Error(_) | Operation::Coroutine(_) | Operation::DictionaryLookup(_) | Operation::DictionaryScope(_) | Operation::DictionaryMutation(_) | Operation::Unset(_) => false,
                    Operation::TclOoHelper(tcl_registry::native_tcloo_compilation::NativeTclOoInstruction::Next{words,..},_)=>words.iter().any(|word|matches!(word,tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original(original) if index==*original)),
                    Operation::TclOoHelper(tcl_registry::native_tcloo_compilation::NativeTclOoInstruction::ObjectInfo{operand,..},_)=>matches!(operand,tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original(original) if index==*original),
                    Operation::Invoke => true,
                    Operation::Load(target) => {
                        index == 1
                            && matches!(target.original, NativeVariableWordOperand::DynamicWord)
                    }
                    Operation::Store(target, value) => {
                        index == *value
                            || (index == 1
                                && matches!(
                                    target.original,
                                    NativeVariableWordOperand::DynamicWord
                                ))
                    }
                    Operation::Increment(target, amount, immediate) => {
                        (index == 1
                            && matches!(target.original, NativeVariableWordOperand::DynamicWord))
                            || (Some(index) == *amount && immediate.is_none())
                    }
                    Operation::Append(target, recipe) => {
                        recipe.values.contains(&index)
                            || (index == 1
                                && matches!(
                                    target.original,
                                    NativeVariableWordOperand::DynamicWord
                                ))
                    }
                    Operation::List(NativeCompiledListRecipe::DynamicElements { .. }, _) => index > 0,
                    Operation::Concat { operands, .. } | Operation::FormatStringConcat { operands, .. } => operands.iter().any(|operand| matches!(operand, NamespaceOperand::Original(word) if index == *word)),
                    Operation::Switch(switch) => matches!(
                        switch.recipe.subject,
                        tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original(word)
                            if index == word
                    ),
                    Operation::SelectedReturn(recipe,_,_) => {
                        Some(index)==recipe.value_word || match &recipe.options {
                            tcl_registry::native_return_compilation::NativeReturnOptionsOperand::StackWord(word)=>index==*word,
                            tcl_registry::native_return_compilation::NativeReturnOptionsOperand::StackPairs(range)=>range.contains(&index),
                            _=>false,
                        }
                    }
                    Operation::Uplevel(recipe, _) => Some(index) == recipe.level_word || recipe.script_words.contains(&index),
                    _ => false,
                };
                let literal = if emitted && matches!(operation, Operation::Invoke) && index == 0 {
                    self.command_literal(&capture)?
                } else if emitted {
                    capture
                        .literal(index)
                        .map(|bytes| self.literals.intern_bytes(bytes))
                } else {
                    None
                };
                let arena = if emitted && literal.is_none() {
                    self.arena(original.executable_parts(), depth)?
                } else {
                    ArenaInstruction {
                        original: original.executable_parts().clone(),
                        texts: HashMap::new(),
                        locals: HashMap::new(),
                        roots: HashMap::new(),
                        syntax: HashMap::new(),
                    }
                };
                words.push(WordInstruction {
                    original: original.clone(),
                    literal,
                    arena,
                });
            }
            if let Operation::Switch(switch) = &mut operation {
                self.prepare_body_switch(switch, depth)?;
            }
            if let Operation::SelectedReturn(recipe, empty, options) = &mut operation {
                if recipe.value_word.is_none() {
                    *empty = Some(self.literals.intern_bytes(b""));
                }
                if recipe.exit
                    == tcl_registry::native_return_compilation::NativeReturnExit::Immediate
                {
                    let tcl_registry::native_return_compilation::NativeReturnOptionsOperand::Static(
                        literal,
                    ) = &recipe.options
                    else {
                        unreachable!("static Return")
                    };
                    *options = Some(
                        self.literals
                            .register_private_return_options(literal.clone()),
                    );
                }
            }
            commands.push(CommandInstruction {
                source_extent: command
                    .source_extent
                    .ok_or_else(|| unavailable("native C original command source extent"))?,
                words,
                guard: (!matches!(
                    operation,
                    Operation::Invoke
                        | Operation::NamespaceBindings(NamespaceOperation {
                            generic: Some(_),
                            ..
                        })
                ))
                .then(|| {
                    tcl_registry::native_compilation::NativeCompilationGuard::for_native_dialect(
                        self.interp.native_invocation_dialect(),
                    )
                })
                .flatten(),
                operation,
            });
        }
        let empty = if commands.is_empty() && plan.fatal_tail.is_none() {
            Some(
                if tcl_runtime_api::native_literal::source_literal_empty_result_is_unshared(
                    self.stamp.source_protocol,
                ) {
                    self.literals.register_unshared(b"")
                } else {
                    self.literals.intern_bytes(b"")
                },
            )
        } else {
            None
        };
        let fatal = plan
            .fatal_tail
            .map(|cut| self.syntax_instruction(cut))
            .transpose()?;
        self.scripts.insert(
            region,
            Script {
                commands,
                fatal,
                empty,
            },
        );
        Ok(())
    }
}
