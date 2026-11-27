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

//! The builder's operands: compilation reports, literals and command literals, locals, word arenas and variable targets.

use super::super::{obj, Code};
use super::{unavailable, ArenaInstruction, Builder, SyntaxInstruction, Target};
use std::collections::HashMap;
use tcl_lexer::{ExecutablePart, ExecutablePartArena};
use tcl_registry::native_compilation::NativeCompilationFrame;
use tcl_registry::native_compiler_words::NativeCompilerWords;
use tcl_syntax::naming::{
    NativeCompiledVariableEnvironment as Environment, NativeCompiledVariableLookup as Lookup,
};
use tcl_syntax::native_variable_words::NativeVariableWordOperand;
use tcl_syntax::value::ValueError;

impl Builder<'_> {
    pub(super) fn reject_native_compilation(
        &mut self,
        failure: tcl_registry::native_compilation::NativeCompilationFailure,
    ) -> ValueError {
        self.compilation_failure = Some(failure);
        unavailable("native registered compiler rejected original source")
    }

    pub(super) fn report_compilation_error(&mut self, error: tcl_cmd_core::CmdError) -> Code {
        if let Err(first) = self.interp.check_entered_native_operation() {
            return self.interp.refuse_native_execution(first);
        }
        if let Some(failure) = self.compilation_failure.take() {
            return self
                .interp
                .report_cmd_error(tcl_cmd_core::CmdError::from_byte_details(
                    tcl_cmd_core::CmdErrorDetails {
                        message: failure.message.unwrap_or_default().into_bytes(),
                        string_result: None,
                        error_code: tcl_cmd_core::CmdErrorCodeUpdate::Set(
                            failure
                                .error_code
                                .unwrap_or_else(|| "NONE".to_owned())
                                .into_bytes(),
                        ),
                        error_info: failure.error_info.map(String::into_bytes),
                        error_line: None,
                        primitive_getter: None,
                    },
                ));
        }
        if let Some(fatal) = self.parse_failure.take() {
            return self.interp.error(fatal.cut.message.as_bytes());
        }
        self.interp.report_cmd_error(error)
    }

    pub(super) fn check_compilation_stage<T>(
        &mut self,
        outcome: Result<T, tcl_cmd_core::CmdError>,
    ) -> Result<T, tcl_cmd_core::CmdError> {
        self.interp.check_entered_native_operation()?;
        outcome
    }

    pub(super) fn original_literal(&mut self, original: obj::Owned) -> usize {
        let index = self.literals.register_private_original();
        self.private_objects.insert(index, original);
        index
    }

    pub(super) fn syntax_instruction(
        &mut self,
        cut: tcl_lexer::NativeScriptWordCut,
    ) -> Result<SyntaxInstruction, tcl_cmd_core::CmdError> {
        let (message, options) = self.syntax_literal_objects(cut.cut.message.as_bytes())?;
        Ok(SyntaxInstruction {
            cut,
            message,
            options,
        })
    }

    fn syntax_literal_objects(
        &mut self,
        message_bytes: &[u8],
    ) -> Result<(usize, usize), tcl_cmd_core::CmdError> {
        use tcl_native_model::native_return_options::{
            NativeReturnOptionsApplication::Syntax, NativeSyntaxMessageAllocation,
        };
        let protocol = self
            .interp
            .native_invocation_dialect()
            .native_return_options_application(Syntax)
            .ok_or_else(|| unavailable("native syntax instruction producer"))?;
        self.interp.error(message_bytes);
        self.interp.check_entered_native_operation()?;
        let message = match protocol
            .syntax_message_allocation()
            .ok_or_else(|| unavailable("native syntax message allocation"))?
        {
            NativeSyntaxMessageAllocation::UnsharedString => {
                self.literals.register_unshared(message_bytes)
            }
            NativeSyntaxMessageAllocation::RegisteredString => {
                self.literals.intern_bytes(message_bytes)
            }
            NativeSyntaxMessageAllocation::OriginalObject => {
                self.original_literal(obj::Owned::retain(self.interp.result_obj()))
            }
        };
        let options = self.interp.capture_original_c_syntax_options()?;
        let options = self.original_literal(options);
        if !protocol.syntax_retains_error_stack() {
            self.interp.reset_original_c_compiler_result()?;
        }
        Ok((message, options))
    }

    pub(super) fn command_literal(
        &mut self,
        words: &NativeCompilerWords<'_>,
    ) -> Result<Option<usize>, ValueError> {
        let Some(head) = words.literal(0) else {
            return Ok(None);
        };
        let (point, names, context, binding) = self.command_literal_lookup(head)?;
        let selected =
            tcl_registry::native_command_literal::native_compiled_command_literal_from_lookup(
                point, names, context, words, binding,
            )
            .map_err(|_| unavailable("native command literal original lookup"))?;
        let Some(selected) = selected else {
            return Ok(None);
        };
        let index = self.literals.intern_native_command_bytes(
            &selected.bytes,
            &selected.context,
            selected.fully_qualified,
        );
        if let Some(priming) = selected.priming {
            if !self.literals.prime_native_command_name(index, priming) {
                return Err(unavailable("native command literal priming geometry"));
            }
        }
        if selected.hide {
            self.literals.hide_native_literal(index);
        }
        Ok(Some(index))
    }

    fn command_literal_lookup(
        &self,
        bytes: &[u8],
    ) -> Result<
        (
            tcl_dialect::model::DialectPoint,
            tcl_syntax::naming::NamePolicyProtocol,
            tcl_runtime_api::native_command_name::NativeLiteralContext,
            Option<tcl_runtime_api::native_compilation::NativeCompilationBinding>,
        ),
        ValueError,
    > {
        use tcl_runtime_api::native_compilation::{
            NativeCommandImplementation, NativeCompilationBinding,
        };
        let names = self
            .interp
            .name_policy_protocol()
            .ok_or_else(|| unavailable("native command literal name issuer"))?;
        let point = self
            .interp
            .native_invocation_dialect()
            .execution_point()
            .ok_or_else(|| unavailable("native command literal engine"))?;
        let context = tcl_runtime_api::native_command_name::NativeLiteralContext {
            interpreter: self.stamp.interpreter,
            namespace_token: self.stamp.namespace as u64,
            entry_epoch: self.stamp.epochs.0,
            namespace_path: self
                .interp
                .namespaces
                .borrow()
                .native_context_path(self.stamp.namespace)
                .ok_or_else(|| unavailable("native command literal namespace"))?,
        };
        let binding = self
            .interp
            .native_command_name_from_binding(self.stamp.namespace, bytes)?
            .map(|cache| {
                let compiler_hook = self
                    .interp
                    .namespaces
                    .borrow()
                    .native_compiler_hook(cache.token)
                    .unwrap_or(
                        tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Unknown,
                    );
                NativeCompilationBinding {
                    slot: cache.slot,
                    namespace_token: cache.namespace_token,
                    token: cache.token,
                    implementation_generation: cache.implementation_generation,
                    implementation: NativeCommandImplementation::Opaque,
                    compiler_hook,
                    compiler: None,
                    procedure_header: None,
                    has_execution_trace: self
                        .stamp
                        .observers
                        .iter()
                        .any(|(_, token, _)| *token == Some(cache.token)),
                }
            });
        Ok((point, names, context, binding))
    }

    pub(super) fn selected_command_literal(&mut self, bytes: &[u8]) -> Result<usize, ValueError> {
        let (point, names, context, binding) = self.command_literal_lookup(bytes)?;
        let selected =
            tcl_registry::native_command_literal::native_compiled_command_name_literal_from_lookup(
                point, names, context, bytes, binding,
            )
            .map_err(|_| unavailable("native selected command literal original lookup"))?;
        let index = self.literals.intern_native_command_bytes(
            &selected.bytes,
            &selected.context,
            selected.fully_qualified,
        );
        if let Some(priming) = selected.priming {
            if !self.literals.prime_native_command_name(index, priming) {
                return Err(unavailable(
                    "native selected command literal priming geometry",
                ));
            }
        }
        Ok(index)
    }

    pub(super) fn retained_selected_command_literal(
        &mut self,
        bytes: &[u8],
        prerequisite: &tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite,
    ) -> Result<usize, ValueError> {
        let point = self
            .interp
            .native_invocation_dialect()
            .execution_point()
            .ok_or_else(|| unavailable("native selected command literal engine"))?;
        let names = self
            .interp
            .name_policy_protocol()
            .ok_or_else(|| unavailable("native selected command literal name issuer"))?;
        let context = tcl_runtime_api::native_command_name::NativeLiteralContext {
            interpreter: self.stamp.interpreter,
            namespace_token: self.stamp.namespace as u64,
            entry_epoch: self.stamp.epochs.0,
            namespace_path: self
                .interp
                .namespaces
                .borrow()
                .native_context_path(self.stamp.namespace)
                .ok_or_else(|| unavailable("native selected command literal namespace"))?,
        };
        let selected = tcl_registry::native_command_literal::native_compiled_selected_command_name_literal_from_lookup(
            point, names, context, bytes, prerequisite.clone(),
        ).map_err(|_| unavailable("native retained selected worker literal"))?;
        let index = self.literals.intern_native_command_bytes(
            &selected.bytes,
            &selected.context,
            selected.fully_qualified,
        );
        if let Some(priming) = selected.priming {
            if !self.literals.prime_native_command_name(index, priming) {
                return Err(unavailable(
                    "native retained selected worker priming geometry",
                ));
            }
        }
        Ok(index)
    }

    pub(super) fn local(&mut self, name: &[u8], substitution: Option<bool>) -> Option<usize> {
        let protocol = self
            .interp
            .native_invocation_dialect()
            .native_compiled_variable_protocol()?;
        let environment = if self.context.frame == NativeCompilationFrame::ProcedureCode {
            Environment::DeclareProcedure
        } else if self.stamp.borrowed_table.is_some() {
            Environment::BorrowFrameSlots
        } else {
            Environment::None
        };
        let lookup = substitution.map_or_else(
            || protocol.command_lookup(name, environment),
            |single| protocol.substitution_lookup(name, single, environment),
        );
        match lookup {
            Lookup::CreateLocal => Some(self.lvt.intern_native(protocol, name)),
            Lookup::ExistingLocalOnly => self.lvt.find_native(protocol, name),
            Lookup::DynamicName => None,
        }
    }

    pub(super) fn arena(
        &mut self,
        original: &ExecutablePartArena,
        depth: u32,
    ) -> Result<ArenaInstruction, tcl_cmd_core::CmdError> {
        self.arena_in(original, depth, false)
    }

    pub(super) fn arena_in(
        &mut self,
        original: &ExecutablePartArena,
        depth: u32,
        template: bool,
    ) -> Result<ArenaInstruction, tcl_cmd_core::CmdError> {
        let mut syntax = HashMap::new();
        let mut texts = HashMap::new();
        let mut locals = HashMap::new();
        let mut roots = HashMap::new();
        // List IDs preserve original index ownership without recursive trees.
        let mut pending = vec![(original.root(), 0usize)];
        while let Some((list, offset)) = pending.last_mut() {
            let Some(component) = original.list(*list).get(*offset) else {
                pending.pop();
                continue;
            };
            *offset += 1;
            match &component.part {
                ExecutablePart::Text(_) => {
                    let bytes = tcl_syntax::backslash::native_arena_text(
                        original,
                        component,
                        self.stamp.grammar.escapes,
                        self.stamp.source_protocol,
                    )
                    .map_err(|_| unavailable("native body original text geometry"))?;
                    texts.insert(component.span, self.literals.intern_bytes(&bytes));
                }
                ExecutablePart::Variable { name, index } => {
                    let bytes = original
                        .bytes(*name)
                        .ok_or_else(|| unavailable("native body variable geometry"))?;
                    if let Some(slot) = self.local(bytes, Some(index.is_none())) {
                        locals.insert(*name, slot);
                    } else {
                        roots.insert(*name, self.literals.intern_bytes(bytes));
                    }
                    if let Some(index) = index {
                        pending.push((*index, 0));
                    }
                }
                ExecutablePart::Command { body } => self.script(*body, depth + 1)?,
                ExecutablePart::Expression { .. } => {
                    return Err(
                        (unavailable("native body expression-sugar compiler instruction")).into(),
                    );
                }
                ExecutablePart::ParseError(message) => {
                    if !template {
                        return Err(
                            (unavailable("native body complete word parse ownership")).into()
                        );
                    }
                    syntax.insert(
                        component.span,
                        self.syntax_literal_objects(message.as_bytes())?,
                    );
                }
            }
        }
        Ok(ArenaInstruction {
            original: original.clone(),
            texts,
            locals,
            roots,
            syntax,
        })
    }

    pub(super) fn target(
        &mut self,
        original: NativeVariableWordOperand,
        depth: u32,
    ) -> Result<Target, tcl_cmd_core::CmdError> {
        let slot = match &original {
            NativeVariableWordOperand::Literal { name, .. }
            | NativeVariableWordOperand::CompoundArray { name, .. } => self.local(name, None),
            NativeVariableWordOperand::DynamicWord => None,
        };
        let index = match &original {
            NativeVariableWordOperand::CompoundArray { index, .. } => {
                Some(self.arena(index, depth)?)
            }
            _ => None,
        };
        let root_literal = if slot.is_none() {
            match &original {
                NativeVariableWordOperand::Literal { name, .. }
                | NativeVariableWordOperand::CompoundArray { name, .. } => {
                    Some(self.literals.intern_bytes(name))
                }
                NativeVariableWordOperand::DynamicWord => None,
            }
        } else {
            None
        };
        let index_literal = match &original {
            NativeVariableWordOperand::Literal {
                index: Some(index), ..
            } => Some(self.literals.intern_bytes(index)),
            _ => None,
        };
        Ok(Target {
            original,
            slot,
            index,
            root_literal,
            index_literal,
        })
    }
}
