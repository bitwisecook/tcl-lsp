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

//! A body's variable targets: capturing the cell, reading and storing through it, and the stage checks around each.

use super::super::{obj, Code, Interp};
use super::{
    native_list_conversion, BodyExecution, CommandInstruction, EvaluatedTarget, NativeBodyArtifact,
    Target,
};
use tcl_syntax::native_variable_words::NativeVariableWordOperand;

impl Interp {
    pub(super) fn body_target(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        target: &Target,
        execution: &mut BodyExecution,
    ) -> Result<EvaluatedTarget, Code> {
        self.body_target_at(artifact, command, target, 1, execution)
    }

    pub(super) fn body_target_at(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        target: &Target,
        word: usize,
        execution: &mut BodyExecution,
    ) -> Result<EvaluatedTarget, Code> {
        let original_name = target.root_literal.map(|index| {
            obj::Owned::retain(
                artifact
                    .literals
                    .original(index)
                    .expect("emitted base name PUSH"),
            )
        });
        match &target.original {
            NativeVariableWordOperand::Literal { name, index, .. } => Ok(EvaluatedTarget {
                root: name.clone(),
                element: index.clone(),
                original_name,
                original_index: target.index_literal.map(|index| {
                    obj::Owned::retain(
                        artifact
                            .literals
                            .original(index)
                            .expect("emitted index PUSH"),
                    )
                }),
                combined: false,
            }),
            NativeVariableWordOperand::CompoundArray { name, .. } => {
                let value = self.body_arena(
                    artifact,
                    target.index.as_ref().expect("emitted array index"),
                    execution,
                )?;
                Ok(EvaluatedTarget {
                    root: name.clone(),
                    element: None,
                    original_name,
                    original_index: Some(value),
                    combined: false,
                })
            }
            NativeVariableWordOperand::DynamicWord => {
                let value = self.body_word(artifact, &command.words[word], execution)?;
                Ok(EvaluatedTarget {
                    root: Vec::new(),
                    element: None,
                    original_name: Some(value),
                    original_index: None,
                    combined: true,
                })
            }
        }
    }

    fn check_body_variable_stage(
        &mut self,
        stage: super::super::native_variable_names::OriginalCNameStage<'_>,
    ) -> Result<(), Code> {
        if let Some(check) = stage {
            check().map_err(|error| self.report_cmd_error_using(error, stage))?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn body_capture_target(
        &mut self,
        evaluated: &EvaluatedTarget,
        slot: Option<usize>,
        creates: bool,
    ) -> Result<super::super::native_variable_names::OriginalCVariableCapture, Code> {
        self.body_capture_target_using(evaluated, slot, creates, None)
    }

    pub(super) fn body_capture_target_using(
        &mut self,
        evaluated: &EvaluatedTarget,
        slot: Option<usize>,
        creates: bool,
        original: Option<&native_list_conversion::BodyObjectOperation>,
    ) -> Result<super::super::native_variable_names::OriginalCVariableCapture, Code> {
        let actual = self.clone();
        match original {
            Some(original) => self.body_capture_target_with_check(
                evaluated,
                slot,
                creates,
                Some(&|| original.check(&actual)),
            ),
            None => self.body_capture_target_with_check(evaluated, slot, creates, None),
        }
    }

    pub(super) fn body_capture_target_with_check(
        &mut self,
        evaluated: &EvaluatedTarget,
        slot: Option<usize>,
        creates: bool,
        stage: super::super::native_variable_names::OriginalCNameStage<'_>,
    ) -> Result<super::super::native_variable_names::OriginalCVariableCapture, Code> {
        self.check_body_variable_stage(stage)?;
        use tcl_syntax::naming::{
            NativeVariableDiagnosticReason as Reason, NativeVariableFailureSite as Site,
        };
        use tcl_syntax::native_variable_name::NativeVariableNameLookupPurpose as Purpose;
        let purpose = if creates {
            Purpose::Write
        } else {
            Purpose::Read
        };
        let root = evaluated.root.as_slice();
        if let Some(slot) = slot {
            // The original PUSH operands are materialised only when the
            // variable instruction executes, after later value/amount words.
            let element = evaluated
                .original_index
                .as_ref()
                .map(|value| {
                    self.original_c_name_bytes_using(value.as_ptr(), stage)
                        .map(|bytes| bytes.to_vec())
                })
                .transpose()
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
            let captured = crate::vars::capture_original_indexed_receiver(
                &mut self.frames.borrow_mut(),
                &mut self.namespaces.borrow_mut(),
                slot,
                element.clone(),
                creates,
            );
            self.check_body_variable_stage(stage)?;
            // A slot linked outside the activation selects a cell a confined
            // interpreter may not store to.
            let captured = captured.and_then(|captured| {
                if let Some((_, home)) = &captured {
                    self.refuse_confined_receiver(home, purpose)?;
                }
                Ok(captured)
            });
            let captured = captured
                .map_err(|error| {
                    let site = if creates && error == crate::frame::VarError::DeletedArray {
                        Site::ValueWrite
                    } else if creates && error == crate::frame::VarError::DeletedNamespace {
                        if element.is_some() {
                            Site::IndexedNameLookup
                        } else {
                            Site::ValueWrite
                        }
                    } else {
                        Site::NameLookup
                    };
                    self.body_variable_receiver_error_with_check(evaluated, purpose, site, error, stage)
                })?
                .ok_or_else(|| {
                    if creates {
                    self.refuse_native_access(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("compiled created variable receiver"))
                } else {
                    self.body_variable_missing_value_with_check(evaluated, Reason::NoSuchVariable, stage)
                }
                })?;
            if creates
                && self
                    .native_invocation_dialect()
                    .tcl_version
                    .is_some_and(|version| version >= tcl_dialect::TclVersion::V8_5)
            {
                captured.0.retain_original_element_key(
                    evaluated.original_index.as_ref().map(obj::Owned::as_ptr),
                );
            }
            Ok(
                super::super::native_variable_names::OriginalCVariableCapture {
                    receiver: captured.0,
                    home: captured.1,
                    root: root.to_vec(),
                    element,
                },
            )
        } else {
            if evaluated.combined {
                return self
                    .capture_original_c_variable_report_using(
                        evaluated
                            .original_name
                            .as_ref()
                            .expect("emitted variable name")
                            .as_ptr(),
                        if creates {
                            Purpose::Write
                        } else {
                            Purpose::Read
                        }, stage
                    )?
                    .ok_or_else(|| {
                        if creates {
                    self.refuse_native_access(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("compiled created variable receiver"))
                } else {
                    self.body_variable_missing_value_with_check(evaluated, Reason::NoSuchVariable, stage)
                }
                    });
            }
            self.capture_original_c_parts_report_using(
                evaluated
                    .original_name
                    .as_ref()
                    .expect("emitted stack base name")
                    .as_ptr(),
                evaluated.original_index.as_ref().map(obj::Owned::as_ptr),
                if creates {
                    Purpose::Write
                } else {
                    Purpose::Read
                }, stage
            )?
            .ok_or_else(|| if creates {
                    self.refuse_native_access(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("compiled created variable receiver"))
                } else {
                    self.body_variable_missing_value_with_check(evaluated, Reason::NoSuchVariable, stage)
                })
        }
    }

    fn body_variable_reporting_input_with_check(
        &mut self,
        evaluated: &EvaluatedTarget,
        stage: super::super::native_variable_names::OriginalCNameStage<'_>,
    ) -> Result<super::super::native_variable_names::OriginalCVariableReportingInput, Code> {
        self.check_body_variable_stage(stage)?;
        self.original_c_variable_reporting_input_using(
            (&evaluated.root, evaluated.element.as_deref()),
            (
                evaluated.original_name.as_ref().map(obj::Owned::as_ptr),
                evaluated.original_index.as_ref().map(obj::Owned::as_ptr),
                evaluated.combined,
            ),
            stage,
        )
    }

    fn body_variable_receiver_error_with_check(
        &mut self,
        evaluated: &EvaluatedTarget,
        purpose: tcl_syntax::native_variable_name::NativeVariableNameLookupPurpose,
        site: tcl_syntax::naming::NativeVariableFailureSite,
        error: crate::frame::VarError,
        stage: super::super::native_variable_names::OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(code) = self.check_body_variable_stage(stage) {
            return code;
        }
        if self.host_refusal_pending() {
            return Code::Error;
        }
        let input = match self.body_variable_reporting_input_with_check(evaluated, stage) {
            Ok(input) => input,
            Err(code) => return code,
        };
        self.original_c_variable_receiver_error_using(
            input.input(),
            purpose,
            error,
            Some(site),
            stage,
        )
    }

    fn body_variable_missing_value_with_check(
        &mut self,
        evaluated: &EvaluatedTarget,
        reason: tcl_syntax::naming::NativeVariableDiagnosticReason,
        stage: super::super::native_variable_names::OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(code) = self.check_body_variable_stage(stage) {
            return code;
        }
        if self.host_refusal_pending() {
            return Code::Error;
        }
        let input = match self.body_variable_reporting_input_with_check(evaluated, stage) {
            Ok(input) => input,
            Err(code) => return code,
        };
        self.original_c_variable_failure_input_using(
            input.input(),
            tcl_syntax::native_variable_name::NativeVariableNameLookupPurpose::Read,
            reason,
            tcl_syntax::naming::NativeVariableFailureSite::ValueRead,
            stage,
        )
    }

    pub(super) fn body_read_original_parts_using(
        &mut self,
        evaluated: &EvaluatedTarget,
        slot: Option<usize>,
        original: &native_list_conversion::BodyObjectOperation,
    ) -> Result<obj::Owned, Code> {
        let actual = self.clone();
        self.body_read_original_parts_with_check(evaluated, slot, Some(&|| original.check(&actual)))
    }

    fn body_read_original_parts_with_check(
        &mut self,
        evaluated: &EvaluatedTarget,
        slot: Option<usize>,
        stage: super::super::native_variable_names::OriginalCNameStage<'_>,
    ) -> Result<obj::Owned, Code> {
        self.check_body_variable_stage(stage)?;
        let reached = (|| {
            use tcl_syntax::naming::{
                NativeVariableDiagnosticReason as Reason, NativeVariableFailureSite as Site,
            };
            use tcl_syntax::native_variable_name::NativeVariableNameLookupPurpose as Purpose;
            let capture = self.body_capture_target_with_check(evaluated, slot, false, stage)?;
            let root = capture.root.as_slice();
            let element = capture.element.as_deref();
            let receiver = capture.receiver;
            let home = capture.home;
            if self.has_variable_traces() {
                let access = self.trace_access(root, root, element, &home, false);
                let traced = self
                    .fire_var_trace_resolved(&home, &access, b"read")
                    .map_err(|first| self.refuse_native_execution(first))?;
                self.check_body_variable_stage(stage)?;
                if traced.is_guest_error() {
                    return Err(self.body_variable_receiver_error_with_check(
                        evaluated,
                        Purpose::Read,
                        Site::ValueRead,
                        crate::frame::VarError::TraceError,
                        stage,
                    ));
                }
            }
            if self.host_refusal_pending() {
                return Err(Code::Error);
            }
            let value = receiver
                .read()
                .map_err(|error| {
                    self.body_variable_receiver_error_with_check(
                        evaluated,
                        Purpose::Read,
                        Site::ValueRead,
                        error,
                        stage,
                    )
                })?
                .ok_or_else(|| {
                    self.body_variable_missing_value_with_check(
                        evaluated,
                        if element.is_some() && receiver.is_array() {
                            Reason::NoSuchElement
                        } else {
                            Reason::NoSuchVariable
                        },
                        stage,
                    )
                })?;
            Ok(obj::Owned::retain(value))
        })();
        self.check_body_variable_stage(stage)?;
        reached
    }

    pub(super) fn body_store_using(
        &mut self,
        evaluated: &EvaluatedTarget,
        slot: Option<usize>,
        value: &obj::Owned,
        original: &native_list_conversion::BodyObjectOperation,
    ) -> Code {
        let actual = self.clone();
        self.body_store_with_check(evaluated, slot, value, Some(&|| original.check(&actual)))
    }

    fn body_store_with_check(
        &mut self,
        evaluated: &EvaluatedTarget,
        slot: Option<usize>,
        value: &obj::Owned,
        stage: super::super::native_variable_names::OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(code) = self.check_body_variable_stage(stage) {
            return code;
        }
        match self.body_store_value_with_check(evaluated, slot, value, stage) {
            Ok(Some(result)) => {
                self.set_result(result.as_ptr());
                let current = self.check_body_variable_stage(stage);
                drop(result);
                match current.and_then(|()| self.check_body_variable_stage(stage)) {
                    Ok(()) => Code::Ok,
                    Err(code) => code,
                }
            }
            Ok(None) => {
                self.set_result_bytes(b"");
                match self.check_body_variable_stage(stage) {
                    Ok(()) => Code::Ok,
                    Err(code) => code,
                }
            }
            Err(code) => code,
        }
    }

    #[cfg(test)]
    pub(super) fn body_store_value(
        &mut self,
        evaluated: &EvaluatedTarget,
        slot: Option<usize>,
        value: &obj::Owned,
    ) -> Result<Option<obj::Owned>, Code> {
        self.body_store_value_with_check(evaluated, slot, value, None)
    }

    pub(super) fn body_store_value_using(
        &mut self,
        evaluated: &EvaluatedTarget,
        slot: Option<usize>,
        value: &obj::Owned,
        original: &native_list_conversion::BodyObjectOperation,
    ) -> Result<Option<obj::Owned>, Code> {
        let actual = self.clone();
        self.body_store_value_with_check(evaluated, slot, value, Some(&|| original.check(&actual)))
    }

    fn body_store_value_with_check(
        &mut self,
        evaluated: &EvaluatedTarget,
        slot: Option<usize>,
        value: &obj::Owned,
        stage: super::super::native_variable_names::OriginalCNameStage<'_>,
    ) -> Result<Option<obj::Owned>, Code> {
        self.check_body_variable_stage(stage)?;
        let reached = (|| {
            use tcl_syntax::naming::NativeVariableFailureSite as Site;
            use tcl_syntax::native_variable_name::NativeVariableNameLookupPurpose as Purpose;
            let capture = self.body_capture_target_with_check(evaluated, slot, true, stage)?;
            let root = capture.root.as_slice();
            let element = capture.element.as_deref();
            let receiver = capture.receiver;
            let home = capture.home;
            let stored = receiver.store(value.as_ptr());
            self.check_body_variable_stage(stage)?;
            if let Err(error) = stored {
                return Err(self.body_variable_receiver_error_with_check(
                    evaluated,
                    Purpose::Write,
                    Site::ValueWrite,
                    error,
                    stage,
                ));
            }
            if self.has_variable_traces() {
                let access = self.trace_access(root, root, element, &home, false);
                let traced = self
                    .fire_var_trace_resolved(&home, &access, b"write")
                    .map_err(|first| self.refuse_native_execution(first))?;
                self.check_body_variable_stage(stage)?;
                if traced.is_guest_error() {
                    return Err(self.body_variable_receiver_error_with_check(
                        evaluated,
                        Purpose::Write,
                        Site::ValueWrite,
                        crate::frame::VarError::TraceError,
                        stage,
                    ));
                }
            }
            if self.host_refusal_pending() {
                return Err(Code::Error);
            }
            let value = match receiver.read() {
                Err(crate::frame::VarError::NameProtocolUnavailable) if stage.is_some() => {
                    return Err(self.body_variable_receiver_error_with_check(
                        evaluated,
                        Purpose::Write,
                        Site::ValueRead,
                        crate::frame::VarError::NameProtocolUnavailable,
                        stage,
                    ));
                }
                reached => reached.ok().flatten(),
            };
            Ok(value.map(obj::Owned::retain))
        })();
        self.check_body_variable_stage(stage)?;
        reached
    }

    pub(super) fn body_helper_operand(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        words: &[tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand],
        literal_slots: &[Option<usize>],
        index: usize,
        execution: &mut BodyExecution,
    ) -> Result<obj::Owned, Code> {
        match &words[index] {
            tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original(
                original,
            ) => self.body_word(artifact, &command.words[*original], execution),
            _ => Ok(obj::Owned::retain(
                artifact
                    .literals
                    .original(literal_slots[index].expect("projected member slot"))
                    .expect("compiled original member"),
            )),
        }
    }
}
