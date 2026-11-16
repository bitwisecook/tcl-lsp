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

//! The errors of an original-name operation: lookup, trace, receiver and constant-increment messages.

use super::super::{Code, Interp};
use super::OriginalCNameStage;
use crate::obj::TclObj;
use tcl_syntax::native_variable_name::NativeVariableNameLookupPurpose;
use tcl_syntax::value::ValueError;

impl Interp {
    fn original_c_variable_error_without_operand_using(
        &mut self,
        purpose: NativeVariableNameLookupPurpose,
        error: crate::frame::VarError,
        stage: OriginalCNameStage<'_>,
    ) -> Option<Code> {
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return Some(self.report_cmd_error_using(error, stage));
        }
        if !purpose.leaves_error_message()
            && error != crate::frame::VarError::NameProtocolUnavailable
        {
            #[cfg(test)]
            if purpose == NativeVariableNameLookupPurpose::Link {
                crate::env_trace::remember_original_c_link_error(error);
            }
            self.traces.borrow_mut().pending_err.take();
            return Some(Code::Error);
        }
        (error == crate::frame::VarError::NameProtocolUnavailable).then(|| {
            self.report_cmd_error_using(
                ValueError::CommandProtocolUnavailable("actual indexed variable receiver").into(),
                stage,
            )
        })
    }

    pub(super) fn original_c_lookup_var_error(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        error: crate::frame::VarError,
    ) -> Code {
        self.original_c_lookup_var_error_using(original, purpose, error, None)
    }

    pub(super) fn original_c_lookup_var_error_using(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        error: crate::frame::VarError,
        stage: OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error_using(error, stage);
        }
        if let Some(code) =
            self.original_c_variable_error_without_operand_using(purpose, error, stage)
        {
            return code;
        }
        // Retire the reached callback failure before materialising its name,
        // just as the original-object access did before it selected a presenter.
        let trace_reason = (error == crate::frame::VarError::TraceError).then(|| {
            self.traces
                .borrow_mut()
                .pending_err
                .take()
                .unwrap_or_default()
        });
        let bytes = match self.original_c_name_bytes_using(original, stage) {
            Ok(bytes) => bytes.to_vec(),
            Err(error) => return self.report_cmd_error_using(error, stage),
        };
        let input = tcl_syntax::naming::NativeVariableInputForm::Combined(&bytes);
        if let Some(reason) = trace_reason {
            return self.original_c_variable_trace_error_using(input, purpose, &reason, stage);
        }
        self.original_c_variable_receiver_error_using(input, purpose, error, None, stage)
    }

    fn original_c_variable_trace_error_using(
        &mut self,
        input: tcl_syntax::naming::NativeVariableInputForm<'_>,
        purpose: NativeVariableNameLookupPurpose,
        reason: &[u8],
        stage: OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error_using(error, stage);
        }
        let Some(protocol) = self.native_c_variable_name_protocol() else {
            return self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original C variable trace report",
                ),
            );
        };
        let name =
            match tcl_syntax::naming::report_native_c_variable_value_name(
                tcl_syntax::naming::NativeNameProtocol::C(protocol.version()),
                input,
            ) {
                Ok(name) => name,
                Err(_) => return self.refuse_native_access(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "original C variable trace report name",
                    ),
                ),
            };
        self.var_trace_error_using(
            &name,
            if matches!(
                purpose,
                NativeVariableNameLookupPurpose::Read
                    | NativeVariableNameLookupPurpose::IncrementRead
                    | NativeVariableNameLookupPurpose::IncrementExisting
            ) {
                b"read"
            } else {
                b"write"
            },
            reason,
            stage,
        )
    }

    pub(in super::super) fn original_c_increment_constant_error(
        &mut self,
        original: &[u8],
    ) -> Code {
        use tcl_syntax::naming::{
            NativeVariableDiagnosticOperation as Operation,
            NativeVariableDiagnosticReason as Reason, NativeVariableFailureSite as Site,
        };
        let Some(protocol) = self.native_c_variable_name_protocol() else {
            return self.report_cmd_error(
                ValueError::CommandProtocolUnavailable("original increment constant presenter")
                    .into(),
            );
        };
        let diagnostic = match tcl_syntax::naming::report_native_variable_diagnostic_at(
            tcl_syntax::naming::NativeNameProtocol::C(protocol.version()),
            Operation::Increment,
            Reason::Constant,
            Site::ValueWrite,
            tcl_syntax::naming::NativeVariableInputForm::Combined(original),
        ) {
            Ok(diagnostic) => diagnostic,
            Err(_) => {
                return self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("selected increment constant failure")
                        .into(),
                );
            }
        };
        let mut message = b"can't incr \"".to_vec();
        message.extend_from_slice(&diagnostic.name);
        message.extend_from_slice(b"\": ");
        message.extend_from_slice(diagnostic.reason.message().as_bytes());
        self.report_cmd_error(tcl_cmd_core::CmdError::from_byte_details(
            tcl_cmd_core::CmdErrorDetails {
                message,
                string_result: protocol.diagnostic_string_protocol(),
                error_code: diagnostic.error_code.map_or(
                    tcl_cmd_core::CmdErrorCodeUpdate::Default,
                    |words| {
                        tcl_cmd_core::CmdErrorCodeUpdate::Set(super::super::error_code_list(
                            &words.iter().map(Vec::as_slice).collect::<Vec<_>>(),
                        ))
                    },
                ),
                error_info: None,
                error_line: None,
                primitive_getter: None,
            },
        ))
    }

    pub(in super::super) fn original_c_variable_receiver_error(
        &mut self,
        input: tcl_syntax::naming::NativeVariableInputForm<'_>,
        purpose: NativeVariableNameLookupPurpose,
        error: crate::frame::VarError,
        reached_site: Option<tcl_syntax::naming::NativeVariableFailureSite>,
    ) -> Code {
        self.original_c_variable_receiver_error_using(input, purpose, error, reached_site, None)
    }

    pub(in super::super) fn original_c_variable_receiver_error_using(
        &mut self,
        input: tcl_syntax::naming::NativeVariableInputForm<'_>,
        purpose: NativeVariableNameLookupPurpose,
        error: crate::frame::VarError,
        reached_site: Option<tcl_syntax::naming::NativeVariableFailureSite>,
        stage: OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error_using(error, stage);
        }
        use crate::frame::VarError;
        use tcl_syntax::naming::{
            NativeVariableDiagnosticReason as Reason, NativeVariableFailureSite as Site,
        };
        if let Some(code) =
            self.original_c_variable_error_without_operand_using(purpose, error, stage)
        {
            return code;
        }
        let reason = match error {
            VarError::IsScalar => Reason::NotArray,
            VarError::IsArray => Reason::IsArray,
            VarError::NoSuchNamespace => Reason::MissingParentNamespace,
            VarError::DeletedArray => Reason::DetachedElement,
            VarError::DeletedNamespace => Reason::RetiredNamespace,
            VarError::IsConstant => Reason::Constant,
            VarError::Confined => {
                let Some(protocol) = self.native_c_variable_name_protocol() else {
                    return self.report_cmd_error_using(
                        ValueError::CommandProtocolUnavailable(
                            "original C confined variable report",
                        )
                        .into(),
                        stage,
                    );
                };
                let name = match tcl_syntax::naming::report_native_c_variable_value_name(
                    tcl_syntax::naming::NativeNameProtocol::C(protocol.version()),
                    input,
                ) {
                    Ok(name) => name,
                    Err(_) => {
                        return self.report_cmd_error_using(
                            ValueError::CommandProtocolUnavailable(
                                "original C confined variable report name",
                            )
                            .into(),
                            stage,
                        );
                    }
                };
                return if matches!(
                    purpose,
                    NativeVariableNameLookupPurpose::Unset
                        | NativeVariableNameLookupPurpose::QuietUnset
                ) {
                    self.confined_unset_error_using(&name, stage)
                } else {
                    self.confined_store_error_using(&name, stage)
                };
            }

            VarError::NameProtocolUnavailable => {
                unreachable!("operand-free refusal returned above")
            }
            VarError::TraceError => {
                let reason = self
                    .traces
                    .borrow_mut()
                    .pending_err
                    .take()
                    .unwrap_or_default();
                return self.original_c_variable_trace_error_using(input, purpose, &reason, stage);
            }
        };
        let site = reached_site.unwrap_or_else(|| match purpose {
            NativeVariableNameLookupPurpose::Read if reason == Reason::IsArray => Site::ValueRead,
            NativeVariableNameLookupPurpose::Write
            | NativeVariableNameLookupPurpose::QuietWrite
                if matches!(
                    reason,
                    Reason::IsArray
                        | Reason::Constant
                        | Reason::DetachedElement
                        | Reason::RetiredNamespace
                ) =>
            {
                Site::ValueWrite
            }
            NativeVariableNameLookupPurpose::Unset
            | NativeVariableNameLookupPurpose::QuietUnset
                if reason == Reason::Constant =>
            {
                Site::ValueUnset
            }
            _ => Site::NameLookup,
        });
        self.original_c_variable_failure_input_using(input, purpose, reason, site, stage)
    }

    pub(in super::super) fn original_variable_trace_requires_name(
        &self,
        home: &crate::vars::TraceHome,
        element: Option<&[u8]>,
        operation: &[u8],
    ) -> bool {
        // Geometry comes from the selected receiver; the placeholder spelling
        // is not read or exposed by this trace-selection query.
        let access = self.trace_access(&home.base, &home.base, element, home, false);
        let cell = self.variable_trace_scope(home, access.match_elem.as_deref());
        let active = self.active_var_trace_scopes.borrow();
        // Unset callbacks run on the destroyed cell even when its read or
        // write callback is active. The containing array keeps its own guard.
        let destroyed = tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation::from_bytes(operation)
            .is_some_and(tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation::uses_destroyed_cell_callbacks);
        if !destroyed && active.contains(&cell) {
            return false;
        }
        let array_active = access.match_elem.is_some() && active.contains(&cell.array());
        let traces = self.traces.borrow();
        let registered = traces.traces.iter().any(|trace| {
            (trace.elem.is_some() || (access.whole_array && !array_active))
                && cell.matches(trace, operation)
        });
        let intrinsic = access.whole_array
            && !array_active
            && ((access.match_elem.is_none()
                && matches!(operation, b"read" | b"unset")
                && self.native_error_variable_at(home).is_some())
                || (self.native_precision_trace_at(home)
                    && matches!(operation, b"read" | b"write" | b"unset"))
                || (self.env_trace_reaches(home)
                    && matches!(operation, b"read" | b"write" | b"unset" | b"array")));
        registered || intrinsic
    }
}
