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

//! Original compiled List append and argument-List publication stages.
use super::native_list_conversion::BodyObjectOperation;
use super::*;
use crate::interp::native_arguments::OriginalArguments;
use crate::interp::native_variable_names::OriginalCVariableCapture;
use std::cell::Cell;
use tcl_cmd_core::CmdError;

struct AppendReceiver {
    capture: Rc<OriginalCVariableCapture>,
    expected: Cell<Result<Option<*mut TclObj>, crate::frame::VarError>>,
    _original: Option<obj::NativeObjectLifetime>,
}
impl AppendReceiver {
    fn new(capture: OriginalCVariableCapture) -> Self {
        let original = capture.receiver.read_initial();
        Self {
            _original: original
                .ok()
                .flatten()
                .map(obj::NativeObjectLifetime::retain),
            capture: Rc::new(capture),
            expected: Cell::new(original),
        }
    }
    fn check(&self, actual: &Interp, original: &BodyObjectOperation) -> Result<(), CmdError> {
        original.check(actual)?;
        self.capture
            .receiver
            .check_current()
            .map_err(|_| original.refuse(unavailable("original compiled append receiver")))?;
        let current = self.capture.receiver.read_initial();
        if current == Err(crate::frame::VarError::NameProtocolUnavailable) {
            return Err(original.refuse(unavailable("original compiled append scalar role")));
        }
        if current != self.expected.get() {
            return Err(original.refuse(unavailable("original compiled append receiver header")));
        }
        if let Ok(Some(value)) = current {
            obj::check_native_liveness(value).map_err(|error| original.refuse(error))?;
        }
        Ok(())
    }
}

// The converted input remains borrowed through a reached receiver read trace.
// Ordinary List members can change without changing their primary descriptor;
// abstract GetElements caches are immutable once reached, and empty non-List
// shortcuts keep their actual primary rather than fabricating List backing.
struct AppendInput {
    pointer: *mut TclObj,
    header: OriginalArguments,
    children: OriginalArguments,
    primary: obj::NativePrimaryReceipt,
    backing: Option<crate::list::NativeListBacking>,
    abstract_series: bool,
    elements: Vec<*mut TclObj>,
    protocol: NativeStringProtocol,
}
impl AppendInput {
    fn capture(
        actual: &Interp,
        original: &BodyObjectOperation,
        pointer: *mut TclObj,
        protocol: NativeStringProtocol,
    ) -> Result<Self, CmdError> {
        original.check(actual)?;
        let header = OriginalArguments::enter(actual, &[pointer])?;
        let elements = crate::list::list_elements_native_using(
            pointer,
            protocol,
            &|| {
                original.check(actual)?;
                header.check_original_for(actual, 0, pointer)
            },
            &|error| original.refuse(error),
        )?;
        let input = Self {
            pointer,
            children: OriginalArguments::enter(actual, &elements)?,
            primary: obj::NativePrimaryReceipt::capture(pointer)
                .map_err(|error| original.refuse(error))?,
            backing: crate::list::native_list_backing(pointer),
            abstract_series: crate::native_arithseries::is_series(pointer),
            header,
            elements,
            protocol,
        };
        input.check(actual, original)?;
        Ok(input)
    }
    fn check(&self, actual: &Interp, original: &BodyObjectOperation) -> Result<(), CmdError> {
        original.check(actual)?;
        self.header.check_original_for(actual, 0, self.pointer)?;
        self.children.check_for(actual)?;
        self.primary
            .check_for(self.pointer, "original compiled append input primary")
            .map_err(|error| original.refuse(error))?;
        let same = if let Some(backing) = &self.backing {
            let elements = backing.elements().map_err(|error| original.refuse(error))?;
            elements.as_ref() == self.elements.as_slice()
        } else if self.abstract_series {
            let (values, length) =
                crate::native_arithseries::cached_elements_raw(self.pointer, self.protocol)
                    .map_err(|error| original.refuse(error))?;
            length == self.elements.len()
                && (length == 0
                    || unsafe { std::slice::from_raw_parts(values, length) }
                        == self.elements.as_slice())
        } else {
            self.elements.is_empty()
        };
        if !same {
            return Err(original.refuse(unavailable("original compiled append input members")));
        }
        Ok(())
    }
}

impl Interp {
    pub(super) fn execute_body_append(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        target: &Target,
        recipe: &NativeAppendInstruction,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let original =
            BodyObjectOperation::capture(self).map_err(|first| self.report_cmd_error(first))?;
        let mut values = Vec::<obj::Owned>::new();
        let reached = (|| {
            original
                .check(self)
                .map_err(|first| self.report_cmd_error(first))?;
            let evaluated = self.body_target(artifact, command, target, execution)?;
            original
                .check(self)
                .map_err(|first| self.report_cmd_error(first))?;
            match &recipe.operands {
                NativeAppendOperands::StringObjects => {
                    // All values are evaluated before the first selected append.
                    for word in &command.words[recipe.values.clone()] {
                        values.push(self.body_word(artifact, word, execution)?);
                        original
                            .check(self)
                            .map_err(|first| self.report_cmd_error(first))?;
                    }
                    for value in &values {
                        let capture = self.body_capture_target_using(
                            &evaluated,
                            target.slot,
                            true,
                            Some(&original),
                        )?;
                        original
                            .check(self)
                            .map_err(|first| self.report_cmd_error(first))?;
                        let code = self.append_native_captured(capture, &[value.as_ptr()]);
                        original
                            .check(self)
                            .map_err(|first| self.report_cmd_error(first))?;
                        if code != Code::Ok {
                            return Ok(code);
                        }
                    }
                    Ok(Code::Ok)
                }
                NativeAppendOperands::ListElement => {
                    values.push(self.body_word(
                        artifact,
                        &command.words[recipe.values.start],
                        execution,
                    )?);
                    original
                        .check(self)
                        .map_err(|first| self.report_cmd_error(first))?;
                    let capture = self.body_capture_target_using(
                        &evaluated,
                        target.slot,
                        true,
                        Some(&original),
                    )?;
                    original
                        .check(self)
                        .map_err(|first| self.report_cmd_error(first))?;
                    self.body_append_list_element(
                        capture,
                        values[0].as_ptr(),
                        artifact.stamp.source_protocol,
                        &original,
                    )
                }
                NativeAppendOperands::ListElements {
                    steps,
                    strip_single_expanded,
                } => {
                    values.push(self.body_argument_list(
                        artifact,
                        command,
                        steps,
                        *strip_single_expanded,
                        execution,
                        &original,
                    )?);
                    self.body_append_list_values(
                        &evaluated,
                        target.slot,
                        values[0].as_ptr(),
                        artifact.stamp.source_protocol,
                        &original,
                    )
                }
            }
        })();
        drop(values);
        original
            .check(self)
            .map_err(|first| self.report_cmd_error(first))?;
        reached
    }

    fn body_argument_list(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        steps: &[NativeArgumentListStep],
        strip_single_expanded: bool,
        execution: &mut BodyExecution,
        original: &BodyObjectOperation,
    ) -> Result<obj::Owned, Code> {
        let mut stack = Vec::<obj::Owned>::new();
        let reached = (|| {
            for step in steps {
                original
                    .check(self)
                    .map_err(|first| self.report_cmd_error(first))?;
                match *step {
                    NativeArgumentListStep::Word(index) => {
                        stack.push(self.body_word(artifact, &command.words[index], execution)?);
                        original
                            .check(self)
                            .map_err(|first| self.report_cmd_error(first))?;
                    }
                    NativeArgumentListStep::List(count) => {
                        let operands = stack.split_off(stack.len() - count);
                        let members = operands.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
                        stack.push(obj::Owned::fresh(crate::list::new_list_obj_native(
                            &members,
                            artifact.stamp.source_protocol,
                        )));
                        drop(operands);
                        original
                            .check(self)
                            .map_err(|first| self.report_cmd_error(first))?;
                    }
                    NativeArgumentListStep::Concat => {
                        let source = stack.pop().expect("argument List source");
                        let target = stack.pop().expect("argument List target");
                        let headers =
                            OriginalArguments::enter(self, &[target.as_ptr(), source.as_ptr()])
                                .map_err(|first| self.report_cmd_error(first))?;
                        let reached = crate::list::concatenate_native_lists_using(
                            target.as_ptr(),
                            source.as_ptr(),
                            artifact.stamp.source_protocol,
                            &|| {
                                original.check(self)?;
                                headers.check_for(self)
                            },
                            &|error| original.refuse(error),
                        );
                        let reached = reached.map_err(|first| self.report_cmd_error(first));
                        drop(source);
                        drop(target);
                        original
                            .check(self)
                            .map_err(|first| self.report_cmd_error(first))?;
                        stack.push(reached?);
                    }
                }
            }
            let value = stack.pop().expect("native argument List result");
            debug_assert!(stack.is_empty());
            if strip_single_expanded {
                let header = OriginalArguments::enter(self, &[value.as_ptr()])
                    .map_err(|first| self.report_cmd_error(first))?;
                crate::list::native_full_list_range_using(
                    value.as_ptr(),
                    artifact.stamp.source_protocol,
                    &|| {
                        original.check(self)?;
                        header.check_for(self)
                    },
                    &|error| original.refuse(error),
                )
                .map_err(|first| self.report_cmd_error(first))
            } else {
                Ok(value)
            }
        })();
        drop(stack);
        original
            .check(self)
            .map_err(|first| self.report_cmd_error(first))?;
        reached
    }

    fn body_publish_append(
        &mut self,
        selected: &AppendReceiver,
        value: obj::Owned,
        original: &BodyObjectOperation,
    ) -> Result<Code, Code> {
        let reached = (|| {
            selected
                .check(self, original)
                .map_err(|first| self.report_cmd_error(first))?;
            let capture = &selected.capture;
            let root = capture.root.as_slice();
            selected.expected.set(Ok(Some(value.as_ptr())));
            if let Err(error) = capture.receiver.store(value.as_ptr()) {
                original
                    .check(self)
                    .map_err(|first| self.report_cmd_error(first))?;
                return Ok(crate::builtins::var_error(self, root, error));
            }
            selected
                .check(self, original)
                .map_err(|first| self.report_cmd_error(first))?;
            Ok(Code::Ok)
        })();
        drop(value);
        original
            .check(self)
            .map_err(|first| self.report_cmd_error(first))?;
        if reached? != Code::Ok {
            return reached;
        }
        selected
            .check(self, original)
            .map_err(|first| self.report_cmd_error(first))?;
        let capture = &selected.capture;
        let root = capture.root.as_slice();
        let access =
            self.trace_access(root, root, capture.element.as_deref(), &capture.home, false);
        let failed = self
            .fire_var_trace_resolved(&capture.home, &access, b"write")
            .map_err(|first| self.refuse_native_execution(first))?
            .is_guest_error();
        original
            .check(self)
            .map_err(|first| self.report_cmd_error(first))?;
        if failed {
            return Ok(crate::builtins::var_error(
                self,
                root,
                crate::frame::VarError::TraceError,
            ));
        }
        capture.receiver.check_current().map_err(|_| {
            self.report_cmd_error(original.refuse(unavailable("original compiled append receiver")))
        })?;
        match capture.receiver.read() {
            Ok(Some(value)) => self.set_result(value),
            Err(crate::frame::VarError::NameProtocolUnavailable) => {
                return Err(self.report_cmd_error(
                    original.refuse(unavailable("original compiled append result scalar role")),
                ))
            }
            Ok(None) | Err(_) => self.set_result_bytes(b""),
        }
        original
            .check(self)
            .map_err(|first| self.report_cmd_error(first))?;
        Ok(Code::Ok)
    }

    fn body_append_list_element(
        &mut self,
        capture: OriginalCVariableCapture,
        element: *mut TclObj,
        protocol: NativeStringProtocol,
        original: &BodyObjectOperation,
    ) -> Result<Code, Code> {
        let selected = AppendReceiver::new(capture);
        let actual = self.clone();
        let element_header = OriginalArguments::enter(self, &[element])
            .map_err(|first| self.report_cmd_error(first))?;
        let check = || {
            selected.check(&actual, original)?;
            element_header.check_for(&actual)
        };
        check().map_err(|first| self.report_cmd_error(first))?;
        let root = selected.capture.root.as_slice();
        if selected.capture.receiver.is_constant() {
            return Ok(crate::builtins::var_error(
                self,
                root,
                crate::frame::VarError::IsConstant,
            ));
        }
        let value = match selected.expected.get() {
            Err(error) => return Ok(crate::builtins::var_error(self, root, error)),
            Ok(None) => obj::Owned::fresh(obj::new_string_bytes(b"")),
            Ok(Some(value)) if obj::is_shared(value) => {
                let duplicate =
                    obj::duplicate_with_stage_check(value, &mut || check(), &mut |first| {
                        actual.clone().report_cmd_error(first.clone());
                    })
                    .map_err(|first| self.report_cmd_error(first))?;
                crate::list::duplicate_native_backing(value, duplicate, protocol);
                obj::Owned::fresh(duplicate)
            }
            Ok(Some(value)) => obj::Owned::retain(value),
        };
        let reached = (|| {
            selected
                .check(self, original)
                .map_err(|first| self.report_cmd_error(first))?;
            // Single-element TclPtrSetVar installs its selected header before conversion.
            selected.expected.set(Ok(Some(value.as_ptr())));
            if let Err(error) = selected.capture.receiver.store(value.as_ptr()) {
                original
                    .check(self)
                    .map_err(|first| self.report_cmd_error(first))?;
                return Ok(crate::builtins::var_error(self, root, error));
            }
            crate::list::append_prepared_native_elements_using(
                value.as_ptr(),
                &[element],
                protocol,
                &|| {
                    selected.check(self, original)?;
                    element_header.check_for(self)
                },
                &|error| original.refuse(error),
            )
            .map_err(|first| self.report_cmd_error(first))?;
            Ok(Code::Ok)
        })();
        if reached != Ok(Code::Ok) {
            drop(value);
            original
                .check(self)
                .map_err(|first| self.report_cmd_error(first))?;
            return reached;
        }
        self.body_publish_append(&selected, value, original)
    }

    fn body_append_list_values(
        &mut self,
        evaluated: &EvaluatedTarget,
        slot: Option<usize>,
        input: *mut TclObj,
        protocol: NativeStringProtocol,
        original: &BodyObjectOperation,
    ) -> Result<Code, Code> {
        // LAPPEND_LIST converts its evaluated input before variable lookup.
        let input = AppendInput::capture(self, original, input, protocol)
            .map_err(|first| self.report_cmd_error(first))?;
        let capture = self.body_capture_target_using(evaluated, slot, true, Some(original))?;
        input
            .check(self, original)
            .map_err(|first| self.report_cmd_error(first))?;
        let root = capture.root.as_slice();
        let access =
            self.trace_access(root, root, capture.element.as_deref(), &capture.home, false);
        let failed = self
            .fire_var_trace_resolved(&capture.home, &access, b"read")
            .map_err(|first| self.refuse_native_execution(first))?
            .is_guest_error();
        input
            .check(self, original)
            .map_err(|first| self.report_cmd_error(first))?;
        let selected = AppendReceiver::new(capture);
        let root = selected.capture.root.as_slice();
        selected
            .check(self, original)
            .map_err(|first| self.report_cmd_error(first))?;
        let value = if failed {
            let reason = self
                .traces
                .borrow_mut()
                .pending_err
                .take()
                .unwrap_or_default();
            self.var_trace_error(root, b"read", &reason);
            None
        } else {
            match selected.expected.get() {
                Ok(value) => value,
                Err(error) => {
                    crate::builtins::var_error(self, root, error);
                    None
                }
            }
        };
        if value.is_none() && !failed {
            self.no_such_variable(root, selected.capture.element.as_deref());
        }
        let empty =
            tcl_registry::native_instruction_plan::native_list_append_empty_publication(protocol)
                .ok_or_else(|| {
                self.report_cmd_error(
                    original.refuse(unavailable("native List append instruction")),
                )
            })?;
        let value = match value {
            None if input.elements.is_empty() && empty.fresh_missing_receiver => {
                obj::Owned::fresh(obj::new_obj())
            }
            None => obj::Owned::retain(input.pointer),
            Some(value) => {
                let actual = self.clone();
                let check = || {
                    selected.check(&actual, original)?;
                    input.check(&actual, original)
                };
                if input.elements.is_empty() && empty.skip_existing_store {
                    crate::list::list_elements_native_using(value, protocol, &check, &|error| {
                        original.refuse(error)
                    })
                    .map_err(|first| self.report_cmd_error(first))?;
                    unsafe { self.set_obj_result(value) };
                    original
                        .check(self)
                        .map_err(|first| self.report_cmd_error(first))?;
                    return Ok(Code::Ok);
                }
                crate::list::append_native_list_elements_using(
                    value,
                    &input.elements,
                    protocol,
                    &check,
                    &|error| original.refuse(error),
                )
                .map_err(|first| self.report_cmd_error(first))?
            }
        };
        input
            .check(self, original)
            .map_err(|first| self.report_cmd_error(first))?;
        self.body_publish_append(&selected, value, original)
    }
}

#[cfg(test)]
#[path = "native_lappend/original_boundary_tests.rs"]
mod original_boundary_tests;
