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

//! Compiling a script object: its commands, substitutions and word parts.

use super::super::{line_of, parse, Interp, NsId, NATIVE_EVAL_DEPTH_LIMIT};
use super::{source_word, CompiledChunk, CompilerEntry, SelectedInvocation};
use tcl_registry::native_compilation::{
    NativeCompilationContext as Context, NativeCompilationFailureScope,
    NativeCompilationSelection as Selection, NativeCompilationStep, NativeCompilationSteps,
    NativeExpressionCompilerStep, NativeMathFunctionResolution,
};
use tcl_registry::{InvocationWord, InvocationWords};
use tcl_runtime_api::{NativeCompilationError, NativeCompilationErrorCommand};

impl Interp {
    pub(super) fn compile_native_script(
        &self,
        source: &[u8],
        namespace: NsId,
        context: Context,
        chunk: &mut CompiledChunk,
        depth: u32,
    ) -> Option<NativeCompilationError> {
        if NATIVE_EVAL_DEPTH_LIMIT.exceeded(depth) {
            return None;
        }
        let key = (source.to_vec(), context);
        if chunk.scripts.contains_key(&key) {
            return None;
        }
        let commands = parse::parse_script_with_config(source, self.lexer_config());
        let selections: Vec<_> = commands
            .iter()
            .map(|command| self.select_native_invocation(command, namespace, context))
            .collect();
        chunk.scripts.insert(key, selections.clone());
        for (index, (command, mut selected)) in commands.iter().zip(selections).enumerate() {
            if let Some(failure) = self.compile_native_command(
                source,
                command,
                selected.as_deref_mut(),
                CompilerEntry { namespace, context },
                chunk,
                depth,
            ) {
                return Some(failure);
            }
            if chunk.unpresented_failure {
                return None;
            }
            if let Some(entries) = chunk.scripts.get_mut(&(source.to_vec(), context)) {
                entries[index] = selected;
            }
        }
        None
    }

    fn compile_native_command(
        &self,
        source: &[u8],
        command: &parse::Command<'_>,
        selected: Option<&mut SelectedInvocation>,
        entry: CompilerEntry,
        chunk: &mut CompiledChunk,
        depth: u32,
    ) -> Option<NativeCompilationError> {
        let CompilerEntry { namespace, context } = entry;
        let context_note = || {
            Some(NativeCompilationErrorCommand {
                text: String::from_utf8(source[command.start..command.end].to_vec()).ok()?,
                line: line_of(source, command.start),
                before_context: Vec::new(),
                after_context: Vec::new(),
            })
        };
        let has_selected = selected.is_some();
        if let Some(selected) = selected {
            let arguments: Vec<_> = command.words[1..].iter().map(source_word).collect();
            let head = &selected.facts.canonical_command;
            let words = InvocationWords::structured(InvocationWord::Literal(head), &arguments)
                .with_dialect(self.native_invocation_dialect());
            let spellings = command
                .words
                .iter()
                .map(|word| std::str::from_utf8(word.raw_source))
                .collect::<Result<Vec<_>, _>>()
                .ok()?;
            if let Some(failure) = selected.spec.failure_for_selection_with_source(
                selected.selection,
                words,
                &selected.facts,
                Some(self.native_invocation_dialect()),
                command.words.first().and_then(|word| match &word.body {
                    parse::WordBody::Literal(value) => std::str::from_utf8(value).ok(),
                    _ => None,
                })?,
                &spellings[1..],
            ) {
                let Some(message) = failure.message else {
                    chunk.unpresented_failure = true;
                    return None;
                };
                return Some(NativeCompilationError {
                    message,
                    error_code: failure.error_code,
                    command_contexts: vec![context_note()?],
                    body_line: line_of(source, command.start),
                });
            }
            if let Some(failure) =
                self.compile_native_substitutions(source, command, entry, chunk, depth)
            {
                return Some(failure);
            }
            if chunk.unpresented_failure {
                return None;
            }
            if let NativeCompilationSteps::Known(steps) = selected.spec.compilation_steps(
                selected.selection,
                words,
                &selected.shapes,
                &selected.facts,
                Some(self.native_invocation_dialect()),
            ) {
                for step in steps {
                    let body = match step {
                        NativeCompilationStep::Body(body) => body,
                        NativeCompilationStep::Expression(expression) => {
                            let parse::WordBody::Literal(text) =
                                &command.words.get(expression.argument + 1)?.body
                            else {
                                continue;
                            };
                            let parse_context =
                                tcl_syntax::expr::parser::ExprParseContext::for_profile(
                                    self.dialect_profile(),
                                );
                            let text = std::str::from_utf8(text).ok()?;
                            let table = self.native_math_function_table();
                            let visits = expression.compiler_steps(text, &parse_context, |name| {
                                let Some(table) = &table else {
                                    return NativeMathFunctionResolution::Unknown;
                                };
                                match table.lookup(name) {
                                    tcl_runtime_api::native_compilation::NativeMathFunctionResolution::Present(row) => row.arity.map_or(NativeMathFunctionResolution::Unknown, |arity| NativeMathFunctionResolution::Known { arity }),
                                    tcl_runtime_api::native_compilation::NativeMathFunctionResolution::Absent => NativeMathFunctionResolution::Absent,
                                    tcl_runtime_api::native_compilation::NativeMathFunctionResolution::Unknown => NativeMathFunctionResolution::Unknown,
                                }
                            });
                            for visit in visits {
                                match visit {
                                    NativeExpressionCompilerStep::Unknown => {
                                        chunk.unpresented_failure = true;
                                        return None;
                                    }
                                    NativeExpressionCompilerStep::Failure(failure) => {
                                        let mut note = context_note()?;
                                        note.before_context
                                            .push(expression.error_context.note().into());
                                        let Some(message) = failure.message else {
                                            chunk.unpresented_failure = true;
                                            return None;
                                        };
                                        return Some(NativeCompilationError {
                                            message,
                                            error_code: failure.error_code,
                                            command_contexts: vec![note],
                                            body_line: line_of(source, command.start),
                                        });
                                    }
                                    NativeExpressionCompilerStep::Script(span) => {
                                        let script = text
                                            .get(span.as_range())?
                                            .strip_prefix('[')?
                                            .strip_suffix(']')?;
                                        if let Some(mut failure) = self.compile_native_script(
                                            script.as_bytes(),
                                            namespace,
                                            context,
                                            chunk,
                                            depth + 1,
                                        ) {
                                            failure
                                                .command_contexts
                                                .last_mut()?
                                                .after_context
                                                .push(expression.error_context.note().into());
                                            failure.command_contexts.push(context_note()?);
                                            failure.body_line = line_of(source, command.start);
                                            return Some(failure);
                                        }
                                        if chunk.unpresented_failure {
                                            return None;
                                        }
                                    }
                                }
                            }
                            continue;
                        }
                    };
                    let parse::WordBody::Literal(script) =
                        &command.words.get(body.argument + 1)?.body
                    else {
                        continue;
                    };
                    let entered = selected.spec.body_context_for_operand(
                        Some(self.native_invocation_dialect()),
                        context,
                        selected.selection,
                        selected.shapes[body.argument],
                    );
                    let entered = body.entered_context(entered)?;
                    if let Some(mut failure) =
                        self.compile_native_script(script, namespace, entered, chunk, depth + 1)
                    {
                        if body.failure_scope == NativeCompilationFailureScope::FallbackToGeneric {
                            // A declined catch compiles its protected body only
                            // after runtime reaches the wrapper.
                            chunk.scripts.remove(&(script.to_vec(), entered));
                            selected.selection = Selection::Generic;
                            selected.handler = None;
                            selected.body_contexts.clear();
                            continue;
                        }
                        let note = body.error_context.note(Some(failure.body_line))?;
                        failure
                            .command_contexts
                            .last_mut()?
                            .after_context
                            .push(note);
                        failure.command_contexts.push(context_note()?);
                        failure.body_line = line_of(source, command.start);
                        return Some(failure);
                    }
                    if chunk.unpresented_failure {
                        if body.failure_scope == NativeCompilationFailureScope::FallbackToGeneric {
                            chunk.unpresented_failure = false;
                            chunk.scripts.remove(&(script.to_vec(), entered));
                            selected.selection = Selection::Generic;
                            selected.handler = None;
                            selected.body_contexts.clear();
                        } else {
                            return None;
                        }
                    }
                }
            }
        }
        if !has_selected {
            return self.compile_native_substitutions(source, command, entry, chunk, depth);
        }
        None
    }

    fn compile_native_substitutions(
        &self,
        source: &[u8],
        command: &parse::Command<'_>,
        entry: CompilerEntry,
        chunk: &mut CompiledChunk,
        depth: u32,
    ) -> Option<NativeCompilationError> {
        for word in &command.words {
            if let parse::WordBody::Parts(parts) = &word.body {
                if let Some(mut failure) =
                    self.compile_native_word_parts(parts, entry, chunk, depth)
                {
                    failure
                        .command_contexts
                        .push(NativeCompilationErrorCommand {
                            text: String::from_utf8(source[command.start..command.end].to_vec())
                                .ok()?,
                            line: line_of(source, command.start),
                            before_context: Vec::new(),
                            after_context: Vec::new(),
                        });
                    failure.body_line = line_of(source, command.start);
                    return Some(failure);
                }
                if chunk.unpresented_failure {
                    return None;
                }
            }
        }
        None
    }

    fn compile_native_word_parts(
        &self,
        parts: &[parse::WordPart<'_>],
        entry: CompilerEntry,
        chunk: &mut CompiledChunk,
        depth: u32,
    ) -> Option<NativeCompilationError> {
        if NATIVE_EVAL_DEPTH_LIMIT.exceeded(depth) {
            return None;
        }
        for part in parts {
            let failure = match part {
                parse::WordPart::Command(script) => self.compile_native_script(
                    script,
                    entry.namespace,
                    entry.context,
                    chunk,
                    depth + 1,
                ),
                parse::WordPart::Variable(variable) => variable.index.as_ref().and_then(|parts| {
                    self.compile_native_word_parts(parts, entry, chunk, depth + 1)
                }),
                _ => None,
            };
            if failure.is_some() || chunk.unpresented_failure {
                return failure;
            }
        }
        None
    }
}
