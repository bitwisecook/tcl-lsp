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

//! What a failed variable operation reports: the reporting input, the failure input and the captured report of a name or its parts.

use super::super::{Code, Interp};
use super::{OriginalCNameStage, OriginalCVariableCapture, OriginalCVariableReportingInput};
use crate::obj::TclObj;
use tcl_syntax::native_variable_name::NativeVariableNameLookupPurpose;
use tcl_syntax::value::ValueError;

impl Interp {
    pub(in super::super) fn original_c_variable_reporting_input(
        &mut self,
        fallback: (&[u8], Option<&[u8]>),
        originals: (Option<*mut TclObj>, Option<*mut TclObj>, bool),
    ) -> Result<OriginalCVariableReportingInput, Code> {
        self.original_c_variable_reporting_input_using(fallback, originals, None)
    }

    pub(in super::super) fn original_c_variable_reporting_input_using(
        &mut self,
        fallback: (&[u8], Option<&[u8]>),
        originals: (Option<*mut TclObj>, Option<*mut TclObj>, bool),
        stage: OriginalCNameStage<'_>,
    ) -> Result<OriginalCVariableReportingInput, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        if self.native_c_variable_name_protocol().is_none() {
            return Err(self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original C variable reporting input",
                ),
            ));
        }
        let (original_root, original_element, combined) = originals;
        if combined {
            let original = original_root.ok_or_else(|| {
                self.refuse_native_access(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "combined original variable reporting operand",
                    ),
                )
            })?;
            if original_element.is_some() {
                return Err(self.refuse_native_access(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "combined variable reporting geometry",
                    ),
                ));
            }
            return self
                .original_c_name_bytes_using(original, stage)
                .map(|bytes| OriginalCVariableReportingInput::Combined(bytes.to_vec()))
                .map_err(|error| self.report_cmd_error_using(error, stage));
        }
        let root = original_root
            .map_or_else(
                || Ok(fallback.0.to_vec()),
                |original| {
                    self.original_c_name_bytes_using(original, stage)
                        .map(|bytes| bytes.to_vec())
                },
            )
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let element = original_element
            .map_or_else(
                || Ok(fallback.1.map(<[u8]>::to_vec)),
                |original| {
                    self.original_c_name_bytes_using(original, stage)
                        .map(|bytes| Some(bytes.to_vec()))
                },
            )
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        Ok(OriginalCVariableReportingInput::Separate { root, element })
    }

    pub(super) fn original_c_variable_failure_for_object(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        reason: tcl_syntax::naming::NativeVariableDiagnosticReason,
        site: tcl_syntax::naming::NativeVariableFailureSite,
    ) -> Code {
        self.original_c_variable_failure_for_object_using(original, purpose, reason, site, None)
    }

    pub(super) fn original_c_variable_failure_for_object_using(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        reason: tcl_syntax::naming::NativeVariableDiagnosticReason,
        site: tcl_syntax::naming::NativeVariableFailureSite,
        stage: OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error_using(error, stage);
        }
        if !purpose.leaves_error_message() {
            return Code::Error;
        }
        let bytes = match self.original_c_name_bytes_using(original, stage) {
            Ok(bytes) => bytes.to_vec(),
            Err(error) => return self.report_cmd_error_using(error, stage),
        };
        self.original_c_variable_failure_using(&bytes, purpose, reason, site, stage)
    }

    pub(super) fn original_c_variable_failure_using(
        &mut self,
        original: &[u8],
        purpose: NativeVariableNameLookupPurpose,
        reason: tcl_syntax::naming::NativeVariableDiagnosticReason,
        site: tcl_syntax::naming::NativeVariableFailureSite,
        stage: OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error_using(error, stage);
        }
        self.original_c_variable_failure_input_using(
            tcl_syntax::naming::NativeVariableInputForm::Combined(original),
            purpose,
            reason,
            site,
            stage,
        )
    }

    pub(in super::super) fn original_c_variable_failure_input(
        &mut self,
        input: tcl_syntax::naming::NativeVariableInputForm<'_>,
        purpose: NativeVariableNameLookupPurpose,
        reason: tcl_syntax::naming::NativeVariableDiagnosticReason,
        site: tcl_syntax::naming::NativeVariableFailureSite,
    ) -> Code {
        self.original_c_variable_failure_input_using(input, purpose, reason, site, None)
    }

    pub(in super::super) fn original_c_variable_failure_input_using(
        &mut self,
        input: tcl_syntax::naming::NativeVariableInputForm<'_>,
        purpose: NativeVariableNameLookupPurpose,
        reason: tcl_syntax::naming::NativeVariableDiagnosticReason,
        site: tcl_syntax::naming::NativeVariableFailureSite,
        stage: OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error_using(error, stage);
        }
        if !purpose.leaves_error_message() {
            return Code::Error;
        }
        use tcl_syntax::naming::{
            report_native_variable_diagnostic_at, NativeVariableDiagnosticOperation as Operation,
        };
        let operation = match purpose {
            NativeVariableNameLookupPurpose::Read
            | NativeVariableNameLookupPurpose::Exists
            | NativeVariableNameLookupPurpose::Array
            | NativeVariableNameLookupPurpose::IncrementRead
            | NativeVariableNameLookupPurpose::IncrementExisting => Operation::Read,
            NativeVariableNameLookupPurpose::Unset
            | NativeVariableNameLookupPurpose::QuietUnset => Operation::Unset,
            NativeVariableNameLookupPurpose::Write
            | NativeVariableNameLookupPurpose::QuietWrite
            | NativeVariableNameLookupPurpose::Link
            | NativeVariableNameLookupPurpose::Refer
            | NativeVariableNameLookupPurpose::ArrayMake => Operation::Write,
            NativeVariableNameLookupPurpose::Define => Operation::Define,
        };
        let protocol = self
            .native_c_variable_name_protocol()
            .expect("actual C original lookup");
        let diagnostic = match report_native_variable_diagnostic_at(
            tcl_syntax::naming::NativeNameProtocol::C(protocol.version()),
            operation,
            reason,
            site,
            input,
        ) {
            Ok(diagnostic) => diagnostic,
            Err(_) => {
                return self.report_cmd_error_using(
                    ValueError::CommandProtocolUnavailable("original variable lookup diagnostic")
                        .into(),
                    stage,
                );
            }
        };
        let mut message = format!("can't {} \"", purpose.diagnostic_verb()).into_bytes();
        message.extend_from_slice(&diagnostic.name);
        message.extend_from_slice(b"\": ");
        message.extend_from_slice(diagnostic.reason.message().as_bytes());
        if protocol.diagnostic_resets_interpreter_result() {
            if let Err(error) = self.reset_native_global_error_episode_using(stage) {
                return self.report_cmd_error_using(error, stage);
            }
        }
        self.report_cmd_error_using(
            tcl_cmd_core::CmdError::from_byte_details(tcl_cmd_core::CmdErrorDetails {
                message,
                string_result: protocol.diagnostic_string_protocol(),
                error_code: diagnostic.error_code.map_or(
                    tcl_cmd_core::CmdErrorCodeUpdate::Default,
                    |words| {
                        let words = words.iter().map(Vec::as_slice).collect::<Vec<_>>();
                        tcl_cmd_core::CmdErrorCodeUpdate::Set(super::super::error_code_list(&words))
                    },
                ),
                error_info: None,
                error_line: None,
                primitive_getter: None,
            }),
            stage,
        )
    }

    pub(in super::super) fn capture_original_c_variable_report(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<Option<OriginalCVariableCapture>, Code> {
        self.capture_original_c_variable_report_using(original, purpose, None)
    }

    pub(in super::super) fn capture_original_c_variable_report_using(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        stage: OriginalCNameStage<'_>,
    ) -> Result<Option<OriginalCVariableCapture>, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let selected =
            self.prepare_original_c_name_using(original, purpose.creates_entries(), stage)?;
        Ok(self
            .capture_original_c_selection(original, &selected, purpose)?
            .map(|(receiver, home)| OriginalCVariableCapture {
                receiver,
                home,
                root: selected.root,
                element: selected.element,
            }))
    }

    /// Generic original-object lookup retains its actual selected name purpose.
    /// It does not infer or admit an indexed/compiled caller.
    pub(in super::super) fn capture_original_c_variable_for(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<OriginalCVariableCapture, Code> {
        let reached = self.prepare_original_c_name_for(original, purpose);
        self.check_original_c_name_operation(&[original])
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let selected = match reached? {
            Some(selected) => selected,
            None => {
                return Err(self.original_c_variable_failure_for_object(
                    original,
                    purpose,
                    tcl_syntax::naming::NativeVariableDiagnosticReason::NoSuchVariable,
                    tcl_syntax::naming::NativeVariableFailureSite::NameLookup,
                ));
            }
        };
        let captured = self.capture_original_c_selection(original, &selected, purpose);
        self.check_original_c_name_operation(&[original])
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let Some((receiver, home)) = captured? else {
            return Err(self.original_c_variable_failure_for_object(
                original,
                purpose,
                tcl_syntax::naming::NativeVariableDiagnosticReason::NoSuchVariable,
                tcl_syntax::naming::NativeVariableFailureSite::NameLookup,
            ));
        };
        Ok(OriginalCVariableCapture {
            receiver,
            home,
            root: selected.root,
            element: selected.element,
        })
    }

    /// Capture the two original operands of a selected array stack opcode.
    /// C85+ entry creation retains the same original index object.
    pub(in super::super) fn capture_original_c_parts_report(
        &mut self,
        root: *mut TclObj,
        element: Option<*mut TclObj>,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<Option<OriginalCVariableCapture>, Code> {
        self.capture_original_c_parts_report_using(root, element, purpose, None)
    }

    pub(in super::super) fn capture_original_c_parts_report_using(
        &mut self,
        root: *mut TclObj,
        element: Option<*mut TclObj>,
        purpose: NativeVariableNameLookupPurpose,
        stage: OriginalCNameStage<'_>,
    ) -> Result<Option<OriginalCVariableCapture>, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        // The root lookup cannot publish a root-only error before the original
        // element operand reaches the caller's separate-input presenter.
        let Some(mut selected) = self.prepare_original_c_name_for_using(root, purpose, stage)?
        else {
            return Ok(None);
        };
        if selected.element.is_some() {
            return Err(self.report_cmd_error_using(
                ValueError::CommandProtocolUnavailable("compiled separate base-name geometry")
                    .into(),
                stage,
            ));
        }
        selected.element = element
            .map(|element| {
                self.original_c_name_bytes_using(element, stage)
                    .map(|bytes| bytes.to_vec())
            })
            .transpose()
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let captured = self
            .capture_original_c_parts_selection_using(root, element, &selected, purpose, stage)?;
        if purpose.creates_entries()
            && self
                .native_c_variable_name_protocol()
                .is_some_and(|protocol| protocol.element_table_retains_original())
        {
            if let Some((receiver, _)) = &captured {
                receiver.retain_original_element_key(element);
            }
        }
        Ok(captured.map(|(receiver, home)| OriginalCVariableCapture {
            receiver,
            home,
            root: selected.root,
            element: selected.element,
        }))
    }
}
