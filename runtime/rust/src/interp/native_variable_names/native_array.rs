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

//! Original Array opcode lookup preserves the selected root through its observers.

use super::*;

impl Interp {
    pub(in crate::interp) fn capture_original_array_opcode(
        &mut self,
        slot: Option<usize>,
        root: &[u8],
        original: Option<*mut TclObj>,
        create: bool,
    ) -> Result<Option<OriginalCVariableCapture>, Code> {
        if let Some(slot) = slot {
            let captured = crate::vars::capture_original_indexed_receiver(
                &mut self.frames.borrow_mut(),
                &mut self.namespaces.borrow_mut(),
                slot,
                None,
                create,
            );
            return captured
                .map(|captured| {
                    captured.map(|(receiver, home)| OriginalCVariableCapture {
                        receiver,
                        home,
                        root: root.to_vec(),
                        element: None,
                    })
                })
                .map_err(|error| crate::builtins::var_error(self, root, error));
        }
        let original = original.ok_or_else(|| {
            self.report_cmd_error(
                ValueError::CommandProtocolUnavailable("actual original Array stack operand")
                    .into(),
            )
        })?;
        let purpose = if create {
            NativeVariableNameLookupPurpose::ArrayMake
        } else {
            NativeVariableNameLookupPurpose::Array
        };
        let selection = self.prepare_original_c_name_parts(original, purpose, None, None);
        let selected = match selection {
            Ok(Some(selected)) => selected,
            Ok(None) => return Ok(None),
            Err(code) if create || self.host_refusal_pending() => return Err(code),
            Err(_) => return Ok(None),
        };
        // ARRAY_MAKE creates only part1. The preparer has created that
        // root; selecting an existing part2 must not mint an element here.
        let capture_purpose = if create && selected.element.is_some() {
            NativeVariableNameLookupPurpose::Array
        } else {
            purpose
        };
        let capture = self.capture_original_c_selection(original, &selected, capture_purpose);
        if create && selected.element.is_some() && matches!(capture, Ok(None)) {
            return Err(self.original_c_variable_failure_for_object(
                original,
                purpose,
                tcl_syntax::naming::NativeVariableDiagnosticReason::NoSuchElement,
                tcl_syntax::naming::NativeVariableFailureSite::NameLookup,
            ));
        }
        match capture {
            Ok(captured) => Ok(captured.map(|(receiver, home)| OriginalCVariableCapture {
                receiver,
                home,
                root: selected.root,
                element: selected.element,
            })),
            Err(code) if create || self.host_refusal_pending() => Err(code),
            Err(_) => Ok(None),
        }
    }
    pub(in crate::interp) fn array_exists_original_opcode(
        &mut self,
        slot: Option<usize>,
        root: &[u8],
        original: Option<*mut TclObj>,
    ) -> Result<bool, Code> {
        let Some(captured) = self.capture_original_array_opcode(slot, root, original, false)?
        else {
            return Ok(false);
        };
        let report = if self.original_variable_trace_requires_name(
            &captured.home,
            captured.element.as_deref(),
            b"array",
        ) {
            original
                .map(|original| {
                    self.native_string_bytes(&original)
                        .map(|bytes| bytes.to_vec())
                })
                .transpose()
                .map_err(|error| self.report_cmd_error(error.into()))?
        } else {
            None
        };
        let access = self.trace_access(
            report.as_deref().unwrap_or(&captured.root),
            &captured.root,
            captured.element.as_deref(),
            &captured.home,
            false,
        );
        if self.fire_var_trace_resolved(&captured.home, &access, b"array") {
            return Err(crate::builtins::var_error(
                self,
                &captured.root,
                crate::frame::VarError::TraceError,
            ));
        }
        if self.host_refusal_pending() {
            return Err(Code::Error);
        }
        Ok(captured.element.is_none() && captured.receiver.is_array())
    }
    pub(in crate::interp) fn array_make_original_opcode(
        &mut self,
        slot: Option<usize>,
        root: &[u8],
        original: Option<*mut TclObj>,
    ) -> Result<(), Code> {
        let captured = self
            .capture_original_array_opcode(slot, root, original, true)?
            .ok_or_else(|| self.no_such_variable(root, None))?;
        captured.receiver.ensure_array().map_err(|error| {
            if matches!(error, crate::frame::VarError::IsScalar) {
                let mut message = b"can't array set \"".to_vec();
                message.extend_from_slice(&captured.root);
                message.extend_from_slice(b"\": variable isn't array");
                self.error_with_code(&message, b"TCL WRITE ARRAY")
            } else {
                crate::builtins::var_error(self, &captured.root, error)
            }
        })
    }
}
