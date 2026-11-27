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

//! Executing one instruction: source revalidation, the generic invocation and the opcode dispatch.

use super::super::{line_of, obj, Code, Interp};
use super::native_list_conversion::{BodyListArguments, BodyListFrame};
use super::{
    native_list_conversion, unavailable, BodyExecution, CommandInstruction, EvaluatedTarget,
    NamespaceOperand, NamespaceOperation, NativeBodyArtifact, Operation,
};
use tcl_registry::native_compiler_words::NativeCompiledListRecipe;

impl Interp {
    pub(super) fn execute_body_instruction(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        execution: &mut BodyExecution,
    ) -> Code {
        // Complete currency inspection before entering any recursive callback.
        // Error construction and source re-evaluation have separate frames.
        match self.body_instruction_source_is_current(artifact, command) {
            Ok(true) => {}
            Ok(false) => return self.execute_body_revalidated_source(artifact, command),
            Err(code) => return code,
        }
        // A generic callback can evaluate another original body synchronously.
        // Select its small dispatch frame before entering the opcode executor.
        let result = if matches!(
            command.operation,
            Operation::Invoke
                | Operation::NamespaceBindings(NamespaceOperation {
                    generic: Some(_),
                    ..
                })
        ) {
            self.execute_body_generic_invocation(artifact, command, execution)
        } else {
            self.execute_body_opcode(artifact, command, execution)
        };
        result.unwrap_or_else(|code| code)
    }

    // This check must unwind before argument evaluation or nested invocation.
    // Keep its error temporaries outside the recursive command dispatch frame.
    #[inline(never)]
    fn body_instruction_source_is_current(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
    ) -> Result<bool, Code> {
        // naming.command.native-command-entry-currency
        // docs/design/analysis/name-resolution-proofs/command-native-command-entry-currency.md
        // C's START_CMD checks compiler/resolver currency before arguments.
        // The selected operation remains captured during those arguments.
        let Some(guard) = command.guard else {
            return Ok(true);
        };
        if !guard.requires_current_epochs() {
            return Ok(true);
        }
        let Some(current) = self.native_compiler_cache_epochs(artifact.stamp.namespace) else {
            return Err(
                self.report_cmd_error(unavailable("native original command-entry epochs").into())
            );
        };
        Ok(!guard.revalidate_source(artifact.stamp.epochs, current))
    }

    // The stale-source path evaluates the exact command through the ordinary
    // interpreter entry, without enlarging every current opcode's dispatch frame.
    #[inline(never)]
    fn execute_body_revalidated_source(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
    ) -> Code {
        self.eval_script_mode_unpublished(
            &artifact.image.bytes()[command.source_extent.as_range()],
            None,
            false,
        )
    }

    fn execute_body_generic_invocation(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let arguments = crate::interp::native_arguments::OriginalArguments::enter(self, &[])
            .map_err(|first| self.report_cmd_error(first))?;
        let frame = BodyListFrame::capture(self);
        frame
            .check(self, &arguments)
            .map_err(|first| self.report_cmd_error(first))?;
        let reached = (|| {
            let mut values = Vec::new();
            let mut original_words = Vec::new();
            let written;
            let operands = if let Operation::NamespaceBindings(NamespaceOperation {
                generic: Some(operands),
                ..
            }) = &command.operation
            {
                operands.as_slice()
            } else {
                written = (0..command.words.len())
                    .map(NamespaceOperand::Original)
                    .collect::<Vec<_>>();
                written.as_slice()
            };
            for operand in operands {
                let word = match operand {
                    NamespaceOperand::Original(index) => Some(&command.words[*index]),
                    NamespaceOperand::Literal(_) => None,
                };
                frame
                    .check(self, &arguments)
                    .map_err(|first| self.report_cmd_error(first))?;
                let value = self.body_namespace_operand(artifact, command, operand, execution)?;
                frame
                    .check(self, &arguments)
                    .map_err(|first| self.report_cmd_error(first))?;
                if execution.done {
                    return Ok(Code::Ok);
                }
                if word.is_some_and(|word| word.original.group().expand) {
                    let captured = BodyListArguments::capture(self, value.as_ptr(), None)
                        .map_err(|first| self.report_cmd_error(first))?;
                    let members = captured
                        .members(self, artifact.stamp.source_protocol)
                        .map_err(|first| self.report_cmd_error(first))?;
                    original_words.extend(std::iter::repeat_n(None, members.len()));
                    values.extend(members.into_iter().map(obj::Owned::retain));
                } else {
                    original_words.push(word);
                    values.push(value);
                }
            }
            frame
                .check(self, &arguments)
                .map_err(|first| self.report_cmd_error(first))?;
            if values.is_empty() {
                self.set_result_bytes(b"");
                return Ok(Code::Ok);
            }
            let argv: Vec<_> = values.iter().map(obj::Owned::as_ptr).collect();
            let file = self
                .cmd_frames
                .borrow()
                .last()
                .and_then(|frame| frame.file.clone());
            let mut added = 0;
            if file.is_some() {
                for (word, value) in original_words.iter().zip(&values) {
                    if let Some(word) = word.filter(|word| word.literal.is_some()) {
                        let line = self
                            .cmd_frames
                            .borrow()
                            .last()
                            .map_or(0, |frame| frame.line_base)
                            + line_of(
                                artifact.image.bytes(),
                                word.original.span().start() as usize,
                            );
                        self.arg_locs
                            .borrow_mut()
                            .push((value.as_ptr(), file.clone(), line));
                        added += 1;
                    }
                }
            }
            frame
                .check(self, &arguments)
                .map_err(|first| self.report_cmd_error(first))?;
            let code = self.dispatch(&argv);
            if added != 0 {
                let mut locations = self.arg_locs.borrow_mut();
                let length = locations.len() - added;
                locations.truncate(length);
            }
            Ok(code)
        })();
        frame
            .check(self, &arguments)
            .map_err(|first| self.report_cmd_error(first))?;
        reached
    }

    pub(super) fn execute_body_opcode(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        match &command.operation {
            Operation::Upvar(upvar) => self.execute_body_upvar(artifact, command, upvar, execution),
            Operation::InfoExists(exists) => {
                self.execute_body_info_exists(artifact, command, exists, execution)
            }
            Operation::Array(recipe) => {
                self.execute_body_array(artifact, command, recipe, execution)
            }
            Operation::Introspection(recipe) => {
                self.execute_body_introspection(artifact, command, recipe, execution)
            }
            Operation::MathOperator(mathop) => {
                self.execute_body_mathop(artifact, command, mathop, execution)
            }
            Operation::NamespaceString(recipe) => {
                self.execute_body_namespace_string(artifact, command, recipe, execution)
            }
            Operation::Scalar(scalar) => {
                self.execute_body_scalar(artifact, command, scalar, execution)
            }
            Operation::ListOperations(list) => {
                self.execute_body_list_operation(artifact, command, list, execution)
            }
            Operation::ListIndex(index) => {
                self.execute_body_list_index(artifact, command, index, execution)
            }
            Operation::StringTrim(trim) => {
                self.execute_body_string_trim(artifact, command, trim, execution)
            }
            Operation::StringRange(string) => {
                self.execute_body_string_range(artifact, command, string, execution)
            }
            Operation::StringOperand(string) => {
                self.execute_body_string_operand(artifact, command, string, execution)
            }
            Operation::StringMatch(matcher) => {
                self.execute_body_string_match(artifact, command, matcher, execution)
            }
            Operation::Coroutine(coroutine) => {
                self.execute_body_coroutine(artifact, command, coroutine, execution)
            }
            Operation::Error(error) => self.execute_body_error(artifact, command, error, execution),
            Operation::DictionaryScope(dictionary) => {
                self.execute_body_dictionary_scope(artifact, command, dictionary, execution)
            }
            Operation::DictionaryMutation(dictionary) => {
                self.execute_body_dictionary_mutation(artifact, command, dictionary, execution)
            }
            Operation::DictionaryLookup(dictionary) => {
                self.execute_body_dictionary_lookup(artifact, command, dictionary, execution)
            }
            Operation::NamedInvocation(named) => {
                self.execute_body_named(artifact, command, named, execution)
            }
            Operation::Uplevel(recipe, default) => {
                let level = match recipe.level_word {
                    Some(index) => self.body_word(artifact, &command.words[index], execution)?,
                    None => obj::Owned::retain(
                        artifact
                            .literals
                            .original(default.expect("implicit uplevel level"))
                            .expect("original level literal"),
                    ),
                };
                let mut fragments = command.words[recipe.script_words.clone()]
                    .iter()
                    .map(|word| self.body_word(artifact, word, execution))
                    .collect::<Result<Vec<_>, _>>()?;
                let script = if fragments.len() == 1 {
                    fragments.pop().expect("original sole uplevel script")
                } else {
                    let originals = fragments.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
                    let value = self
                        .concat_original_arguments(&originals)
                        .map_err(|error| self.report_cmd_error(error))?;
                    obj::Owned::fresh(value)
                };
                let target = crate::cmd_eval::select_compiled_uplevel_frame(self, level.as_ptr())?;
                let location = if recipe.script_words.len() == 1 {
                    command.words[recipe.script_words.start]
                        .literal
                        .and_then(|_| {
                            let frames = self.cmd_frames.borrow();
                            let frame = frames.last()?;
                            let file = frame.file.clone()?;
                            Some((
                                Some(file),
                                frame.line_base
                                    + line_of(
                                        artifact.image.bytes(),
                                        command.words[recipe.script_words.start]
                                            .original
                                            .span()
                                            .start()
                                            as usize,
                                    ),
                            ))
                        })
                } else {
                    None
                };
                if let Some((file, line)) = location.as_ref() {
                    self.arg_locs
                        .borrow_mut()
                        .push((script.as_ptr(), file.clone(), *line));
                }
                let code = self.eval_uplevel_obj(target, script.as_ptr());
                if location.is_some() {
                    self.arg_locs.borrow_mut().pop();
                }
                if code == Code::Error {
                    self.append_body_frame(b"uplevel");
                }
                Ok(code)
            }
            Operation::Expression(expression) => {
                self.execute_body_expression_instruction(artifact, command, expression, execution)
            }
            Operation::Try(operation) => {
                self.execute_body_try(artifact, command, operation, execution)
            }
            Operation::Break => Ok(Code::Break),
            Operation::Continue => Ok(Code::Continue),
            Operation::Control(control) => {
                self.execute_body_control(artifact, command, control, execution)
            }
            Operation::Each(each) => self.execute_body_each(artifact, command, each, execution),
            Operation::Switch(switch) => {
                self.execute_body_switch(artifact, command, switch, execution)
            }
            Operation::TclOoHelper(recipe, literal_slots) => {
                use tcl_registry::native_tcloo_compilation::NativeTclOoInstruction as Helper;
                let code = match recipe {
                    Helper::SelfObject | Helper::SelfNamespace => {
                        self.execute_native_oo_self(matches!(recipe, Helper::SelfNamespace))
                    }
                    Helper::ObjectInfo { operation, operand } => {
                        let original = self.body_helper_operand(
                            artifact,
                            command,
                            std::slice::from_ref(operand),
                            literal_slots,
                            0,
                            execution,
                        )?;
                        self.execute_native_oo_object_info(*operation, original.as_ptr())
                    }
                    Helper::Next { .. } => {
                        return self.execute_body_next(
                            artifact,
                            command,
                            recipe,
                            literal_slots,
                            execution,
                        );
                    }
                };
                Ok(code)
            }

            Operation::Invoke
            | Operation::NamespaceBindings(NamespaceOperation {
                generic: Some(_), ..
            }) => self.execute_body_generic_invocation(artifact, command, execution),
            Operation::NamespaceBindings(scope) => {
                let original = native_list_conversion::BodyObjectOperation::capture(self)
                    .map_err(|error| self.report_cmd_error(error))?;
                let actual = self.clone();
                let stage = || original.check(&actual);
                let reached = (|| {
                    use tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingKind;
                    let prefix = scope.prefix.map(|index| {
                        obj::Owned::retain(
                            artifact
                                .literals
                                .original(index)
                                .expect("pooled root namespace operand"),
                        )
                    });
                    let namespace_object = scope
                        .namespace
                        .as_ref()
                        .map(|operand| {
                            self.body_namespace_operand(artifact, command, operand, execution)
                        })
                        .transpose()?;
                    original
                        .check(self)
                        .map_err(|error| self.report_cmd_error_using(error, Some(&stage)))?;
                    if execution.done {
                        return Ok(Code::Ok);
                    }
                    for binding in &scope.bindings {
                        let name = self.body_namespace_operand(
                            artifact,
                            command,
                            &binding.name,
                            execution,
                        )?;
                        original
                            .check(self)
                            .map_err(|error| self.report_cmd_error_using(error, Some(&stage)))?;
                        if execution.done {
                            return Ok(Code::Ok);
                        }
                        let namespace = if scope.kind == NativeNamespaceBindingKind::Global {
                            let prefix = prefix.as_ref().expect("original root namespace operand");
                            self.native_namespace_object_lookup_using(
                                prefix.as_ptr(),
                                &stage,
                                &|error| original.refuse(error),
                            )
                            .map_err(|error| self.report_cmd_error_using(error, Some(&stage)))?
                            .ok_or_else(|| {
                                self.report_cmd_error_using(
                                    unavailable("compiled root namespace operand lookup").into(),
                                    Some(&stage),
                                )
                            })?
                        } else if scope.kind == NativeNamespaceBindingKind::Upvar {
                            let namespace_original = namespace_object
                                .as_ref()
                                .expect("original namespace upvar operand");
                            match self.native_namespace_object_lookup_using(namespace_original.as_ptr(), &stage, &|error| original.refuse(error)) {
                            Ok(Some(namespace)) => namespace,
                            Ok(None) => return Ok(crate::cmd_namespace::ns_operation_not_found_using(
                                self,
                                namespace_original.as_ptr(),
                                tcl_syntax::naming::NativeNamespaceLookupOperation::ObjectLookup,
                                Some(&stage),
                            )),
                            Err(error) => {
                                return Ok(self.report_cmd_error_using(error, Some(&stage)));
                            }
                        }
                        } else {
                            let frames = self.frames.borrow();
                            frames.frame_ns(frames.current_level())
                        };
                        original
                            .check(self)
                            .map_err(|error| self.report_cmd_error_using(error, Some(&stage)))?;
                        let code = self.link_original_compiled_namespace_variable_using(
                            name.as_ptr(),
                            namespace,
                            binding.slot,
                            scope.kind == NativeNamespaceBindingKind::Variable,
                            Some(&stage),
                        );
                        if code != Code::Ok || self.host_refusal_pending() {
                            return Ok(code);
                        }
                        drop(name);
                        original
                            .check(self)
                            .map_err(|error| self.report_cmd_error_using(error, Some(&stage)))?;
                        if let Some(operand) = &binding.value {
                            let value =
                                self.body_namespace_operand(artifact, command, operand, execution)?;
                            original.check(self).map_err(|error| {
                                self.report_cmd_error_using(error, Some(&stage))
                            })?;
                            if execution.done {
                                return Ok(Code::Ok);
                            }
                            let code = self.body_store_using(
                                &EvaluatedTarget {
                                    root: binding.local.clone(),
                                    element: None,
                                    original_name: None,
                                    original_index: None,
                                    combined: false,
                                },
                                Some(binding.slot),
                                &value,
                                &original,
                            );
                            if code != Code::Ok || self.host_refusal_pending() {
                                return Ok(code);
                            }
                        }
                    }
                    drop(prefix);
                    original
                        .check(self)
                        .map_err(|error| self.report_cmd_error_using(error, Some(&stage)))?;
                    self.set_result(
                        artifact
                            .literals
                            .original(scope.empty.expect("namespace empty result"))
                            .expect("pooled namespace empty result"),
                    );
                    Ok(Code::Ok)
                })();
                original
                    .check(self)
                    .map_err(|error| self.report_cmd_error_using(error, Some(&stage)))?;
                reached
            }
            Operation::Unset(unset) => self.execute_body_unset(artifact, command, unset, execution),
            Operation::Load(target) => {
                let original = native_list_conversion::BodyObjectOperation::capture(self)
                    .map_err(|error| self.report_cmd_error(error))?;
                let actual = self.clone();
                let stage = || original.check(&actual);
                let reached = (|| {
                    let evaluated = self.body_target(artifact, command, target, execution)?;
                    original
                        .check(self)
                        .map_err(|error| self.report_cmd_error_using(error, Some(&stage)))?;
                    let value = if evaluated.combined {
                        obj::Owned::retain(
                            self.read_original_c_variable_using(
                                evaluated
                                    .original_name
                                    .as_ref()
                                    .expect("original dynamic name")
                                    .as_ptr(),
                                Some(&stage),
                            )?,
                        )
                    } else {
                        self.body_read_original_parts_using(&evaluated, target.slot, &original)?
                    };
                    self.set_result(value.as_ptr());
                    let current = original.check(self);
                    drop(value);
                    current.map_err(|error| self.report_cmd_error_using(error, Some(&stage)))?;
                    original
                        .check(self)
                        .map_err(|error| self.report_cmd_error_using(error, Some(&stage)))?;
                    Ok(Code::Ok)
                })();
                original
                    .check(self)
                    .map_err(|error| self.report_cmd_error_using(error, Some(&stage)))?;
                reached
            }
            Operation::Store(target, value) => {
                let original = native_list_conversion::BodyObjectOperation::capture(self)
                    .map_err(|error| self.report_cmd_error(error))?;
                let actual = self.clone();
                let stage = || original.check(&actual);
                let reached = (|| {
                    let evaluated = self.body_target(artifact, command, target, execution)?;
                    let value = self.body_word(artifact, &command.words[*value], execution)?;
                    original
                        .check(self)
                        .map_err(|error| self.report_cmd_error_using(error, Some(&stage)))?;
                    if evaluated.combined {
                        self.store_original_c_variable_using(
                            evaluated
                                .original_name
                                .as_ref()
                                .expect("original dynamic name")
                                .as_ptr(),
                            value.as_ptr(),
                            Some(&stage),
                        )?;
                        return Ok(Code::Ok);
                    }
                    Ok(self.body_store_using(&evaluated, target.slot, &value, &original))
                })();
                original
                    .check(self)
                    .map_err(|error| self.report_cmd_error_using(error, Some(&stage)))?;
                reached
            }
            Operation::Increment(target, amount, immediate) => {
                let evaluated = self.body_target(artifact, command, target, execution)?;
                let value = if let Some(immediate) = immediate {
                    obj::Owned::fresh(obj::new_wide_int_obj(i64::from(*immediate)))
                } else {
                    self.body_word(
                        artifact,
                        &command.words[amount.expect("explicit nonimmediate amount")],
                        execution,
                    )?
                };
                Ok(self.increment_original_compiled_target(
                    &evaluated.root,
                    evaluated.element.as_deref(),
                    target.slot,
                    (
                        evaluated.original_name.as_ref().map(obj::Owned::as_ptr),
                        evaluated.original_index.as_ref().map(obj::Owned::as_ptr),
                        evaluated.combined,
                    ),
                    value.as_ptr(),
                ))
            }
            Operation::Append(target, recipe) => {
                self.execute_body_append(artifact, command, target, recipe, execution)
            }
            Operation::List(recipe, index) => {
                let value = if let Some(index) = index {
                    obj::Owned::retain(
                        artifact
                            .literals
                            .original(*index)
                            .expect("actual compiler List literal"),
                    )
                } else {
                    debug_assert!(matches!(
                        recipe,
                        NativeCompiledListRecipe::DynamicElements { .. }
                    ));
                    let values = command.words[1..]
                        .iter()
                        .map(|word| self.body_word(artifact, word, execution))
                        .collect::<Result<Vec<_>, _>>()?;
                    let members: Vec<_> = values.iter().map(obj::Owned::as_ptr).collect();
                    obj::Owned::fresh(crate::list::new_list_obj_native(
                        &members,
                        artifact.stamp.source_protocol,
                    ))
                };
                self.set_result(value.as_ptr());
                Ok(Code::Ok)
            }
            Operation::FormatStringConcat { operands, .. } => {
                let value =
                    self.body_format_string_concat(artifact, command, operands, execution)?;
                self.set_result(value.as_ptr());
                Ok(Code::Ok)
            }
            Operation::ListOperands { operands, .. } => {
                let values = operands
                    .iter()
                    .map(|operand| {
                        self.body_namespace_operand(artifact, command, operand, execution)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let members: Vec<_> = values.iter().map(obj::Owned::as_ptr).collect();
                let value = obj::Owned::fresh(crate::list::new_list_obj_native(
                    &members,
                    artifact.stamp.source_protocol,
                ));
                self.set_result(value.as_ptr());
                Ok(Code::Ok)
            }
            Operation::Concat {
                literal, operands, ..
            } => {
                let value = if let Some(index) = literal {
                    obj::Owned::retain(
                        artifact
                            .literals
                            .original(*index)
                            .expect("original concat literal"),
                    )
                } else {
                    let originals = operands
                        .iter()
                        .map(|operand| {
                            self.body_namespace_operand(artifact, command, operand, execution)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    let pointers = originals.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
                    obj::Owned::fresh(
                        self.concat_original_arguments(&pointers)
                            .map_err(|error| self.report_cmd_error(error))?,
                    )
                };
                self.set_result(value.as_ptr());
                Ok(Code::Ok)
            }
            Operation::SelectedReturn(recipe, empty, options) => {
                let original_operation = native_list_conversion::BodyObjectOperation::capture(self)
                    .map_err(|error| self.report_cmd_error(error))?;
                let reached = (|| {
                    use tcl_registry::native_return_compilation::{
                        NativeReturnExit as Exit, NativeReturnOptionsOperand as Options,
                    };
                    let stack_options = match &recipe.options {
                        Options::StackWord(index) => {
                            Some(self.body_word(artifact, &command.words[*index], execution)?)
                        }
                        Options::StackPairs(range) => {
                            let originals = command.words[range.clone()]
                                .iter()
                                .map(|word| self.body_word(artifact, word, execution))
                                .collect::<Result<Vec<_>, _>>()?;
                            let members =
                                originals.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
                            Some(obj::Owned::fresh(crate::list::new_list_obj_native(
                                &members,
                                artifact.stamp.source_protocol,
                            )))
                        }
                        Options::Static(_) => None,
                    };
                    let value = match recipe.value_word {
                        Some(index) => {
                            self.body_word(artifact, &command.words[index], execution)?
                        }
                        None => obj::Owned::retain(
                            artifact
                                .literals
                                .original(empty.expect("implicit return result"))
                                .expect("empty literal"),
                        ),
                    };
                    if recipe.exit == Exit::Done {
                        self.set_result(value.as_ptr());
                        original_operation
                            .check(self)
                            .map_err(|error| self.report_cmd_error(error))?;
                        execution.done = true;
                        Ok(Code::Return)
                    } else if recipe.exit == Exit::Fallthrough {
                        self.set_result(value.as_ptr());
                        original_operation
                            .check(self)
                            .map_err(|error| self.report_cmd_error(error))?;
                        Ok(Code::Ok)
                    } else {
                        let merged;
                        let (code, level, original) = if recipe.exit == Exit::Stack {
                            let options = stack_options.as_ref().expect("stack original options");
                            let (ops, protocol) =
                                crate::return_options::NativeReturnOps::selected(self)
                                    .map_err(|error| self.report_cmd_error(error))?;
                            let mut ops = ops.with_original_objects(
                                original_operation
                                    .objects(self)
                                    .map_err(|error| self.report_cmd_error(error))?,
                            );
                            merged = tcl_cmd_core::native_return_merge::merge_stack(
                                &mut ops, protocol, options,
                            )
                            .map_err(|error| self.report_cmd_error(error))?;
                            (
                                merged.code,
                                i64::from(merged.level),
                                merged.options.as_ptr(),
                            )
                        } else {
                            let Options::Static(literal) = &recipe.options else {
                                unreachable!()
                            };
                            (
                                literal.code,
                                i64::from(literal.level),
                                artifact
                                    .literals
                                    .original(options.expect("private Return slot"))
                                    .expect("private Return original"),
                            )
                        };
                        let objects = original_operation
                            .objects(self)
                            .map_err(|error| self.report_cmd_error(error))?;
                        let code=self.process_original_c_return_options_with_objects(tcl_registry::native_return_options::NativeReturnOptionsApplication::Immediate,code,level,original,&objects).map_err(|error|self.report_cmd_error(error))?;
                        self.set_result(value.as_ptr());
                        original_operation
                            .check(self)
                            .map_err(|error| self.report_cmd_error(error))?;
                        if code == Code::Error && recipe.exit == Exit::Immediate {
                            self.capture_original_return_instruction_context(
                            tcl_registry::native_return_options::NativeReturnOptionsApplication::Immediate,
                            value.as_ptr(),
                            original,
                        ).map_err(|first| self.refuse_native_execution(first))?;
                        }
                        Ok(code)
                    }
                })();
                original_operation
                    .check(self)
                    .map_err(|error| self.report_cmd_error(error))?;
                reached
            }
        }
    }
}
