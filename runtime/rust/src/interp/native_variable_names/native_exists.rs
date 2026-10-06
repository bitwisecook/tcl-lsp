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

//! Existence observes the original receiver without a value or guest-error grant.

use super::*;

impl Interp {
    pub(crate) fn exists_original_c_parts(
        &mut self,
        original: *mut TclObj,
        element: Option<*mut TclObj>,
    ) -> Result<bool, Code> {
        let purpose = NativeVariableNameLookupPurpose::Exists;
        // TclObjLookupVarEx gets part2 before inspecting part1's cache.
        let element_bytes = element
            .map(|element| {
                self.native_string_bytes(&element)
                    .map(|bytes| bytes.to_vec())
            })
            .transpose()
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let selection =
            self.prepare_original_c_name_parts(original, purpose, None, element_bytes.as_deref());
        let selected = match selection {
            Ok(Some(selected)) => selected,
            Ok(None) => return Ok(false),
            Err(code) if self.host_refusal_pending() => return Err(code),
            Err(_) => return Ok(false),
        };
        let captured = self.capture_original_c_selection(original, &selected, purpose);
        let capture = match captured {
            Ok(Some((receiver, home))) => OriginalCVariableCapture {
                receiver,
                home,
                root: selected.root,
                element: selected.element,
            },
            Ok(None) => return Ok(false),
            Err(code) if self.host_refusal_pending() => return Err(code),
            Err(_) => return Ok(false),
        };
        if capture.element.is_some() {
            let parsed_key = element
                .is_none()
                .then(|| self.original_c_parsed_element_key(original))
                .flatten();
            capture.receiver.retain_original_element_key(
                element.or_else(|| parsed_key.as_ref().map(Owned::as_ptr)),
            );
        }
        self.exists_original_c_capture(capture, Some(original))
    }

    pub(in crate::interp) fn exists_original_c_indexed(
        &mut self,
        slot: usize,
        root: &[u8],
        element: Option<*mut TclObj>,
    ) -> Result<bool, Code> {
        let element_bytes = element
            .map(|element| {
                self.native_string_bytes(&element)
                    .map(|bytes| bytes.to_vec())
            })
            .transpose()
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let captured = crate::vars::capture_original_indexed_receiver(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            slot,
            element_bytes.clone(),
            false,
        );
        let (receiver, home) = match captured {
            Ok(Some(captured)) => captured,
            Ok(None) => return Ok(false),
            Err(crate::frame::VarError::NameProtocolUnavailable) => {
                return Err(self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("actual original existence local slot")
                        .into(),
                ))
            }
            Err(_) => return Ok(false),
        };
        if let Some(element) = element {
            receiver.retain_original_element_key(Some(element));
        }
        self.exists_original_c_capture(
            OriginalCVariableCapture {
                receiver,
                home,
                root: root.to_vec(),
                element: element_bytes,
            },
            None,
        )
    }

    fn exists_original_c_capture(
        &mut self,
        captured: OriginalCVariableCapture,
        original: Option<*mut TclObj>,
    ) -> Result<bool, Code> {
        if !self.original_variable_trace_requires_name(
            &captured.home,
            captured.element.as_deref(),
            b"read",
        ) {
            return Ok(captured.receiver.read().ok().flatten().is_some()
                || captured.element.is_none() && captured.receiver.is_array());
        }
        let report = original
            .map(|original| {
                self.native_string_bytes(&original)
                    .map(|bytes| bytes.to_vec())
            })
            .transpose()
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let access = self.trace_access(
            report.as_deref().unwrap_or(&captured.root),
            &captured.root,
            captured.element.as_deref(),
            &captured.home,
            false,
        );
        self.fire_var_trace_resolved_with_errors(&captured.home, &access, b"read", false);
        if self.host_refusal_pending() {
            return Err(Code::Error);
        }
        Ok(captured.receiver.read().ok().flatten().is_some()
            || captured.element.is_none() && captured.receiver.is_array())
    }
}
