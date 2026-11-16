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

//! Reading and assigning through an original name: the reported read, prepared selections, and property and regex variables.

use super::super::native_arguments::OriginalArguments;
use super::super::{Code, Interp};
use super::{OriginalCNameSelection, OriginalCNameStage, OriginalVariableAssignmentObjects};
use crate::obj::{Owned, TclObj};
use tcl_syntax::native_variable_name::NativeVariableNameLookupPurpose;
use tcl_syntax::value::{ValueError, ValueOps};

impl Interp {
    pub(in super::super) fn read_original_c_variable(
        &mut self,
        original: *mut TclObj,
    ) -> Result<*mut TclObj, Code> {
        self.read_original_c_variable_reported(original, original, false)
    }

    pub(crate) fn read_original_c_property_variable(
        &mut self,
        lookup: *mut TclObj,
        reported: *mut TclObj,
    ) -> Result<*mut TclObj, Code> {
        self.read_original_c_variable_reported(lookup, reported, true)
    }

    fn read_original_c_variable_reported(
        &mut self,
        lookup: *mut TclObj,
        original: *mut TclObj,
        create: bool,
    ) -> Result<*mut TclObj, Code> {
        self.read_original_c_variable_reported_using(lookup, original, create, None)
    }

    pub(super) fn read_original_c_variable_reported_using(
        &mut self,
        lookup: *mut TclObj,
        original: *mut TclObj,
        create: bool,
        stage: OriginalCNameStage<'_>,
    ) -> Result<*mut TclObj, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let purpose = NativeVariableNameLookupPurpose::Read;
        let selected = self.prepare_original_c_name_using(lookup, create, stage)?;
        let capture_purpose = if create {
            NativeVariableNameLookupPurpose::Link
        } else {
            purpose
        };
        let captured =
            self.capture_original_c_selection_using(lookup, &selected, capture_purpose, stage)?;
        let Some((receiver, home)) = captured else {
            let bytes = self
                .original_c_name_bytes_using(original, stage)
                .map_err(|error| self.report_cmd_error_using(error, stage))?
                .to_vec();
            return Err(self.original_c_variable_failure_using(
                &bytes,
                purpose,
                tcl_syntax::naming::NativeVariableDiagnosticReason::NoSuchVariable,
                tcl_syntax::naming::NativeVariableFailureSite::NameLookup,
                stage,
            ));
        };
        if self.original_variable_trace_requires_name(&home, selected.element.as_deref(), b"read") {
            let bytes = self
                .original_c_name_bytes_using(original, stage)
                .map_err(|error| self.report_cmd_error_using(error, stage))?
                .to_vec();
            let access = self.trace_access(
                &bytes,
                &selected.root,
                selected.element.as_deref(),
                &home,
                false,
            );
            let traced = self
                .fire_var_trace_resolved(&home, &access, b"read")
                .map_err(|first| self.refuse_native_execution(first))?;
            self.check_original_c_name_stage(stage)
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
            if traced.is_guest_error() {
                return Err(self.original_c_lookup_var_error_using(
                    original,
                    purpose,
                    crate::frame::VarError::TraceError,
                    stage,
                ));
            }
        }
        if self.host_refusal_pending() {
            return Err(Code::Error);
        }
        receiver
            .read()
            .map_err(|error| {
                self.original_c_lookup_var_error_using(original, purpose, error, stage)
            })?
            .ok_or_else(|| {
                self.original_c_variable_failure_for_object_using(
                    original,
                    purpose,
                    if selected.element.is_some() && receiver.is_array() {
                        tcl_syntax::naming::NativeVariableDiagnosticReason::NoSuchElement
                    } else {
                        tcl_syntax::naming::NativeVariableDiagnosticReason::NoSuchVariable
                    },
                    tcl_syntax::naming::NativeVariableFailureSite::ValueRead,
                    stage,
                )
            })
    }

    fn assign_original_c_selection(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
        publish_result: bool,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<(), Code> {
        self.assign_original_c_selection_using(original, value, publish_result, purpose, None)
    }

    pub(super) fn assign_original_c_selection_using(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
        publish_result: bool,
        purpose: NativeVariableNameLookupPurpose,
        stage: OriginalCNameStage<'_>,
    ) -> Result<(), Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let arguments = OriginalArguments::enter(self, &[original, value])
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let result = (|| {
            let reached = self.prepare_original_c_name_for_using(original, purpose, stage);
            arguments
                .original(1)
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
            let selected = reached?.ok_or_else(|| {
                self.report_cmd_error_using(
                    ValueError::CommandProtocolUnavailable("actual original write selection")
                        .into(),
                    stage,
                )
            })?;
            self.assign_prepared_original_c_selection_using(
                original,
                value,
                publish_result,
                purpose,
                &selected,
                stage,
            )
        })();
        arguments
            .check()
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        result
    }

    pub(super) fn assign_prepared_original_c_selection(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
        publish_result: bool,
        purpose: NativeVariableNameLookupPurpose,
        selected: &OriginalCNameSelection,
    ) -> Result<(), Code> {
        self.assign_prepared_original_c_selection_using(
            original,
            value,
            publish_result,
            purpose,
            selected,
            None,
        )
    }

    fn assign_prepared_original_c_selection_using(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
        publish_result: bool,
        purpose: NativeVariableNameLookupPurpose,
        selected: &OriginalCNameSelection,
        stage: OriginalCNameStage<'_>,
    ) -> Result<(), Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        self.assign_prepared_original_c_selection_reported_using(
            OriginalVariableAssignmentObjects {
                lookup: original,
                original,
                value,
            },
            publish_result,
            purpose,
            selected,
            stage,
        )
    }

    fn assign_prepared_original_c_selection_reported(
        &mut self,
        lookup: *mut TclObj,
        original: *mut TclObj,
        value: *mut TclObj,
        publish_result: bool,
        purpose: NativeVariableNameLookupPurpose,
        selected: &OriginalCNameSelection,
    ) -> Result<(), Code> {
        self.assign_prepared_original_c_selection_reported_using(
            OriginalVariableAssignmentObjects {
                lookup,
                original,
                value,
            },
            publish_result,
            purpose,
            selected,
            None,
        )
    }

    fn assign_prepared_original_c_selection_reported_using(
        &mut self,
        OriginalVariableAssignmentObjects {
            lookup,
            original,
            value,
        }: OriginalVariableAssignmentObjects,
        publish_result: bool,
        purpose: NativeVariableNameLookupPurpose,
        selected: &OriginalCNameSelection,
        stage: OriginalCNameStage<'_>,
    ) -> Result<(), Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let arguments = OriginalArguments::enter(self, &[lookup, original, value])
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let result = (|| {
            let reached = self.capture_original_c_selection_using(lookup, selected, purpose, stage);
            arguments
                .original(2)
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
            let (receiver, home) = reached?.ok_or_else(|| {
                self.report_cmd_error_using(
                    ValueError::CommandProtocolUnavailable("actual variable store receiver").into(),
                    stage,
                )
            })?;
            let protective_value = publish_result.then(|| Owned::retain(value));
            let stored = receiver.store(value);
            self.check_original_c_name_stage(stage)
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
            arguments
                .original(2)
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
            stored.map_err(|error| {
                self.original_c_lookup_var_error_using(original, purpose, error, stage)
            })?;
            if self.original_variable_trace_requires_name(
                &home,
                selected.element.as_deref(),
                b"write",
            ) {
                let bytes = match stage {
                    Some(_) => self.original_c_name_bytes_using(original, stage),
                    None => arguments.bytes(1),
                }
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
                let access = self.trace_access(
                    &bytes,
                    &selected.root,
                    selected.element.as_deref(),
                    &home,
                    false,
                );
                let traced = self
                    .fire_var_trace_resolved_with_errors(
                        &home,
                        &access,
                        b"write",
                        purpose.leaves_error_message(),
                    )
                    .map_err(|first| self.refuse_native_execution(first))?
                    .is_guest_error();
                arguments
                    .check()
                    .map_err(|error| self.report_cmd_error_using(error, stage))?;
                self.check_original_c_name_stage(stage)
                    .map_err(|error| self.report_cmd_error_using(error, stage))?;
                if traced {
                    return Err(self.original_c_lookup_var_error_using(
                        original,
                        purpose,
                        crate::frame::VarError::TraceError,
                        stage,
                    ));
                }
            }
            if publish_result {
                match receiver.read().map_err(|error| {
                    self.original_c_lookup_var_error_using(original, purpose, error, stage)
                })? {
                    Some(value) => self.set_result(value),
                    None => self.set_result_bytes(b""),
                }
            }
            self.check_original_c_name_stage(stage)
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
            drop(protective_value);
            Ok(())
        })();
        // Receiver and protective result roles retire before final settlement.
        arguments
            .check()
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        result
    }

    pub(crate) fn store_original_c_property_variable(
        &mut self,
        lookup: *mut TclObj,
        reported: *mut TclObj,
        value: *mut TclObj,
    ) -> Result<(), Code> {
        let purpose = NativeVariableNameLookupPurpose::Write;
        let selected = self.prepare_original_c_name(lookup, true)?;
        self.assign_prepared_original_c_selection_reported(
            lookup, reported, value, false, purpose, &selected,
        )
    }

    pub(in super::super) fn store_original_c_variable(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
    ) -> Result<(), Code> {
        self.assign_original_c_selection(
            original,
            value,
            true,
            NativeVariableNameLookupPurpose::Write,
        )
    }

    pub(crate) fn assign_original_regex_variable(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
    ) -> Result<(), Code> {
        let Some(protocol) = self.native_c_variable_name_protocol() else {
            return self.assign_original_named_variable(original, value);
        };
        match self.assign_original_c_selection(
            original,
            value,
            false,
            protocol.regex_write_purpose(),
        ) {
            Ok(()) => Ok(()),
            Err(code)
                if !protocol.regex_write_purpose().leaves_error_message()
                    && !self.host_refusal_pending() =>
            {
                let name = self
                    .native_string_bytes(&original)
                    .map_err(|error| self.report_cmd_error(error.into()))?;
                let mut message = self.result_bytes();
                message.extend_from_slice(b"couldn't set variable \"");
                message.extend_from_slice(tcl_core_types::c_string_extent(&name));
                message.push(b'"');
                self.report_cmd_error(tcl_cmd_core::CmdError::from_byte_details(
                    tcl_cmd_core::CmdErrorDetails {
                        message,
                        string_result: protocol.diagnostic_string_protocol(),
                        error_code: tcl_cmd_core::CmdErrorCodeUpdate::Unchanged,
                        error_info: None,
                        error_line: None,
                        primitive_getter: None,
                    },
                ));
                Err(code)
            }
            Err(code) => Err(code),
        }
    }

    pub(in super::super) fn assign_original_c_variable(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
    ) -> Result<(), Code> {
        self.assign_original_c_variable_using(original, value, None)
    }

    pub(crate) fn assign_original_c_variable_using(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
        stage: OriginalCNameStage<'_>,
    ) -> Result<(), Code> {
        self.assign_original_c_selection_using(
            original,
            value,
            false,
            NativeVariableNameLookupPurpose::Write,
            stage,
        )
    }
}
