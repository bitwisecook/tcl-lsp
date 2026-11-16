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

//! Evaluating the words of a body: regions, arenas, substitution targets, syntax objects and concatenation.

use super::super::{line_of, obj, Code, Interp, Rc};
use super::{
    native_list_conversion, unavailable, ArenaInstruction, BodyExecution, CommandInstruction,
    EvaluatedTarget, NamespaceOperand, NativeArenaFrame, NativeBodyArtifact, WordInstruction,
};
use tcl_lexer::{ExecutablePart, Span};

impl Interp {
    pub(super) fn execute_body_region(
        &mut self,
        artifact: &NativeBodyArtifact,
        region: Span,
        execution: &mut BodyExecution,
    ) -> Code {
        let script = artifact
            .scripts
            .get(&region)
            .expect("admitted original script instruction region");
        let fatal_original = if script.fatal.is_some() {
            match native_list_conversion::BodyObjectOperation::capture(self) {
                Ok(original) => Some(original),
                Err(error) => return self.report_cmd_error(error),
            }
        } else {
            None
        };
        if let Some(empty) = script.empty {
            self.set_result(
                artifact
                    .literals
                    .original(empty)
                    .expect("emitted empty script literal"),
            );
        }
        for command in &script.commands {
            if !self.evaluation_is_live() {
                return Code::Error;
            }
            if let Some(frame) = self.cmd_frames.borrow_mut().last_mut() {
                frame.line = frame.line_base
                    + line_of(
                        artifact.image.bytes(),
                        command.source_extent.start() as usize,
                    );
                frame.cmd = artifact.image.bytes()[command.source_extent.as_range()].to_vec();
                frame.original_command = None;
            }
            // An embedder's limits hold for a command this body runs without
            // a dispatch, as C counts a compiled command where it starts.
            if let Some(code) = self.charge_tick() {
                return code;
            }
            let counted = self.cmd_count.get();
            let code = self.execute_body_instruction(artifact, command, execution);
            let code = if self.cmd_count.get() == counted {
                self.cmd_count.set(counted + 1);
                // The dispatch boundary charges a dispatched command's
                // result; one run here is charged here.
                if code == Code::Ok {
                    self.charge_result(code)
                } else {
                    code
                }
            } else {
                code
            };
            if execution.done {
                return code;
            }
            if code != Code::Ok || self.host_refusal_pending() {
                if code == Code::Error && !self.host_refusal_pending() {
                    self.log_command_bytes(
                        line_of(
                            artifact.image.bytes(),
                            command.source_extent.start() as usize,
                        ),
                        &artifact.image.bytes()[command.source_extent.as_range()],
                    );
                }
                return if self.host_refusal_pending() {
                    Code::Error
                } else {
                    code
                };
            }
        }
        if let Some(fatal) = &script.fatal {
            let options = artifact
                .literals
                .original(fatal.options)
                .expect("emitted original SYNTAX options");
            let original = fatal_original.as_ref().expect("actual fatal script entry");
            let objects = match original.objects(self) {
                Ok(objects) => objects,
                Err(error) => return self.report_cmd_error(error),
            };
            let code = match self.process_original_c_return_options_with_objects(
                tcl_registry::native_return_options::NativeReturnOptionsApplication::Syntax,
                1,
                0,
                options,
                &objects,
            ) {
                Ok(code) => code,
                Err(error) => return self.report_cmd_error(error),
            };
            if code != Code::Ok {
                self.set_result(
                    artifact
                        .literals
                        .original(fatal.message)
                        .expect("emitted original SYNTAX result"),
                );
                if let Err(error) = original.check(self) {
                    return self.report_cmd_error(error);
                }
                self.clear_error_logged();
                if code == Code::Error {
                    if let Err(first) = self.capture_original_return_instruction_context(
                        tcl_registry::native_return_options::NativeReturnOptionsApplication::Syntax,
                        self.result.get(),
                        options,
                    ) {
                        return self.refuse_native_execution(first);
                    }
                }
            }
            if code == Code::Error {
                self.log_command_bytes(
                    line_of(artifact.image.bytes(), fatal.cut.command_start as usize),
                    &artifact.image.bytes()
                        [fatal.cut.command_start as usize..region.end() as usize],
                );
            }
            return code;
        }
        Code::Ok
    }

    pub(super) fn body_word(
        &mut self,
        artifact: &NativeBodyArtifact,
        word: &WordInstruction,
        execution: &mut BodyExecution,
    ) -> Result<obj::Owned, Code> {
        if let Some(index) = word.literal {
            return Ok(obj::Owned::retain(
                artifact
                    .literals
                    .original(index)
                    .expect("admitted PUSH literal"),
            ));
        }
        self.body_arena(artifact, &word.arena, execution)
    }

    pub(super) fn body_format_string_concat(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        operands: &[NamespaceOperand],
        execution: &mut BodyExecution,
    ) -> Result<obj::Owned, Code> {
        let original = native_list_conversion::BodyObjectOperation::capture(self)
            .map_err(|error| self.report_cmd_error(error))?;
        let reached = (|| {
            let values = operands
                .iter()
                .map(|operand| self.body_namespace_operand(artifact, command, operand, execution))
                .collect::<Result<Vec<_>, _>>()?;
            self.concatenate_body_values_using(values, Some(&original))
                .map_err(|error| self.report_cmd_error(error))
        })();
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        reached
    }

    pub(super) fn body_namespace_operand(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        operand: &NamespaceOperand,
        execution: &mut BodyExecution,
    ) -> Result<obj::Owned, Code> {
        match operand {
            NamespaceOperand::Original(index) => {
                self.body_word(artifact, &command.words[*index], execution)
            }
            NamespaceOperand::Literal(index) => Ok(obj::Owned::retain(
                artifact
                    .literals
                    .original(*index)
                    .expect("original parser-expanded namespace literal"),
            )),
        }
    }

    pub(super) fn body_arena(
        &mut self,
        artifact: &NativeBodyArtifact,
        arena: &ArenaInstruction,
        execution: &mut BodyExecution,
    ) -> Result<obj::Owned, Code> {
        let end = arena.original.list(arena.original.root()).len();
        self.body_arena_range(artifact, arena, execution, 0..end)
    }

    pub(super) fn body_arena_range(
        &mut self,
        artifact: &NativeBodyArtifact,
        arena: &ArenaInstruction,
        execution: &mut BodyExecution,
        range: std::ops::Range<usize>,
    ) -> Result<obj::Owned, Code> {
        let original = native_list_conversion::BodyObjectOperation::capture(self)
            .map_err(|error| self.report_cmd_error(error))?;
        let reached = (|| {
            // One component passes its actual object through; concatenation reaches
            // the native getter only when multiple components require bytes.
            let mut pending: Vec<NativeArenaFrame> =
                vec![(arena.original.root(), range.start, Vec::new(), None)];
            loop {
                original
                    .check(self)
                    .map_err(|error| self.report_cmd_error(error))?;
                let root = pending.len() == 1;
                let frame = pending.last_mut().expect("native word root");
                let Some(component) = arena
                    .original
                    .list(frame.0)
                    .get(frame.1)
                    .filter(|_| !root || frame.1 < range.end)
                else {
                    let (_, _, values, _) = pending.pop().expect("completed native operand");
                    let value = self
                        .concatenate_body_values_using(values, Some(&original))
                        .map_err(|error| self.report_cmd_error(error))?;
                    let Some(parent) = pending.last_mut() else {
                        return Ok(value);
                    };
                    let (name, slot) = parent.3.take().expect("actual array index owner");
                    let mut evaluated =
                        self.body_variable_substitution_target(artifact, arena, name);
                    evaluated.original_index = Some(value);
                    evaluated.combined = false;
                    let read = self.body_read_original_parts_using(&evaluated, slot, &original)?;
                    original
                        .check(self)
                        .map_err(|error| self.report_cmd_error(error))?;
                    parent.2.push(read);
                    continue;
                };
                frame.1 += 1;
                let value = match &component.part {
                    ExecutablePart::Text(_) => obj::Owned::retain(
                        artifact
                            .literals
                            .original(
                                *arena
                                    .texts
                                    .get(&component.span)
                                    .expect("actual emitted TEXT literal"),
                            )
                            .expect("actual TEXT object"),
                    ),
                    ExecutablePart::Variable {
                        name,
                        index: Some(index),
                    } => {
                        frame.3 = Some((*name, arena.locals.get(name).copied()));
                        pending.push((*index, 0, Vec::new(), None));
                        continue;
                    }
                    ExecutablePart::Variable { name, index: None } => self
                        .body_read_original_parts_using(
                            &self.body_variable_substitution_target(artifact, arena, *name),
                            arena.locals.get(name).copied(),
                            &original,
                        )?,
                    ExecutablePart::Command { body } => {
                        let mut saved = self.cmd_frames.borrow().last().map(|frame| {
                            (
                                frame.line,
                                frame.cmd.clone(),
                                frame.original_command.clone(),
                            )
                        });
                        let code = self.execute_body_region(artifact, *body, execution);
                        // Saved command metadata is mandatory cleanup even when
                        // the reached command withdrew this original operation.
                        let current = original.check(self);
                        let retired = {
                            let mut frames = self.cmd_frames.borrow_mut();
                            match (frames.last_mut(), saved.as_mut()) {
                                (Some(frame), Some((line, cmd, saved_original))) => {
                                    frame.line = *line;
                                    frame.cmd = core::mem::take(cmd);
                                    core::mem::replace(
                                        &mut frame.original_command,
                                        saved_original.take(),
                                    )
                                }
                                _ => None,
                            }
                        };
                        drop(retired);
                        drop(saved);
                        current.map_err(|error| self.report_cmd_error(error))?;
                        original
                            .check(self)
                            .map_err(|error| self.report_cmd_error(error))?;
                        if code != Code::Ok {
                            return Err(code);
                        }
                        obj::Owned::retain(self.result_obj())
                    }
                    ExecutablePart::ParseError(message) => {
                        if let Some(&(message, options)) = arena.syntax.get(&component.span) {
                            return Err(
                                self.execute_body_syntax_objects(artifact, message, options)
                            );
                        }
                        return Err(self.error(message.as_bytes()));
                    }
                    ExecutablePart::Expression { .. } => {
                        unreachable!("unadmitted expression sugar")
                    }
                };
                original
                    .check(self)
                    .map_err(|error| self.report_cmd_error(error))?;
                frame.2.push(value);
            }
        })();
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        reached
    }

    fn body_variable_substitution_target(
        &self,
        artifact: &NativeBodyArtifact,
        arena: &ArenaInstruction,
        name: Span,
    ) -> EvaluatedTarget {
        let frames = self.frames.borrow();
        let slot = arena.locals.get(&name).copied();
        let root = slot.map_or_else(
            || arena.original.bytes(name).expect("retained variable root"),
            |slot| {
                frames
                    .compiled_slot_name(slot)
                    .expect("actual compiled local")
            },
        );
        // Indexed instructions report their selected canonical local. C8.4
        // owns its counted declaration bytes; later releases own name headers.
        let original_name = if let Some(slot) = slot {
            frames
                .native_local_name_table()
                .and_then(|table| table.names.get(slot))
                .and_then(Option::as_ref)
                .cloned()
        } else {
            arena.roots.get(&name).map(|index| {
                obj::Owned::retain(
                    artifact
                        .literals
                        .original(*index)
                        .expect("emitted variable name"),
                )
            })
        };
        EvaluatedTarget {
            root: root.to_vec(),
            element: None,
            original_name,
            original_index: None,
            combined: slot.is_none(),
        }
    }

    pub(super) fn execute_body_syntax_objects(
        &mut self,
        artifact: &NativeBodyArtifact,
        message: usize,
        options: usize,
    ) -> Code {
        let original = match native_list_conversion::BodyObjectOperation::capture(self) {
            Ok(original) => original,
            Err(error) => return self.report_cmd_error(error),
        };
        let objects = match original.objects(self) {
            Ok(objects) => objects,
            Err(error) => return self.report_cmd_error(error),
        };
        let options = artifact
            .literals
            .original(options)
            .expect("original syntax options");
        let code = match self.process_original_c_return_options_with_objects(
            tcl_registry::native_return_options::NativeReturnOptionsApplication::Syntax,
            1,
            0,
            options,
            &objects,
        ) {
            Ok(code) => code,
            Err(error) => return self.report_cmd_error(error),
        };
        self.set_result(
            artifact
                .literals
                .original(message)
                .expect("original syntax message"),
        );
        if let Err(error) = original.check(self) {
            return self.report_cmd_error(error);
        }
        self.clear_error_logged();
        code
    }

    #[cfg(test)]
    pub(super) fn concatenate_body_values(
        &mut self,
        values: Vec<obj::Owned>,
    ) -> Result<obj::Owned, Code> {
        self.concatenate_body_values_using(values, None)
            .map_err(|error| self.report_cmd_error(error))
    }

    pub(super) fn concatenate_body_values_using(
        &mut self,
        mut values: Vec<obj::Owned>,
        original: Option<&Rc<native_list_conversion::BodyObjectOperation>>,
    ) -> Result<obj::Owned, tcl_cmd_core::CmdError> {
        let actual = self.clone();
        let objects = original
            .map(|original| original.objects(&actual))
            .transpose()?;
        let check = || original.map_or(Ok(()), |original| original.check(&actual));
        check()?;
        let reached = (|| {
            if values.len() == 1 {
                return Ok(values.pop().expect("sole component"));
            }
            let dialect = actual.native_invocation_dialect();
            let protocol = dialect
                .native_string_protocol()
                .filter(|protocol| protocol.tcl_version().is_some())
                .ok_or_else(|| {
                    tcl_cmd_core::CmdError::from(unavailable(
                        "C original body word concatenation issuer",
                    ))
                })?;
            let base = crate::value_ops::RuntimeAppendObjects {
                dialect,
                binary_recipe: dialect.byte_array_string_recipe(None),
            };
            let originals = values
                .iter()
                .map(|value| crate::value_ops::RuntimeAppendValue::borrowed(value.as_ptr()))
                .collect::<Vec<_>>();
            let result = if let Some(objects) = &objects {
                let append = objects.original_append_objects(base)?;
                append.finish(tcl_cmd_core::native_cat::concatenate_compiled(
                    &append, protocol, &originals,
                ))?
            } else {
                tcl_cmd_core::native_cat::concatenate_compiled(&base, protocol, &originals)
                    .map_err(tcl_cmd_core::CmdError::from)?
            };
            Ok(obj::Owned::retain(result.as_ptr()))
        })();
        // Keep the genuine operation through actual component-owner retirement.
        // Operational mapper retention happens before any unused owner cleanup.
        let reached = reached.map_err(|error| match &objects {
            Some(objects) => objects.original_retain(error),
            None => error,
        });
        drop(values);
        check()?;
        reached
    }
}
