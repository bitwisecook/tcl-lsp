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

//! Original array name lookup and element stores preserve the selected root through observers.

use super::*;

enum OriginalArraySetValues {
    List {
        members: Vec<*mut TclObj>,
        // A genuine C85+ TclListObjCopy header owns the same member backing;
        // it adds no fabricated reference to each original key/value object.
        copy: Option<Owned>,
    },
    Dictionary(Vec<(*mut TclObj, *mut TclObj)>),
}
impl OriginalArraySetValues {
    fn is_empty(&self) -> bool {
        match self {
            Self::List { members, .. } => members.is_empty(),
            Self::Dictionary(pairs) => pairs.is_empty(),
        }
    }
}

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
        self.capture_original_c_array_name(original, purpose)
    }

    /// Generic handlers and selected opcodes share physical name selection;
    /// this receipt grants no inline instruction or local-table declaration.
    fn capture_original_c_array_name(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<Option<OriginalCVariableCapture>, Code> {
        let create = purpose.creates_entries();
        let selection = self.prepare_original_c_name_parts(original, purpose, None, None);
        let selected = match selection {
            Ok(Some(selected)) => selected,
            Ok(None) => return Ok(None),
            Err(code) if create || self.host_refusal_pending() => return Err(code),
            Err(_) => return Ok(None),
        };
        // ARRAY_MAKE creates only part1. The preparer has created that
        // root; selecting an existing part2 must not mint an element here.
        let capture_purpose = if purpose == NativeVariableNameLookupPurpose::ArrayMake
            && selected.element.is_some()
        {
            NativeVariableNameLookupPurpose::Array
        } else {
            purpose
        };
        let capture = self.capture_original_c_selection(original, &selected, capture_purpose);
        if purpose == NativeVariableNameLookupPurpose::ArrayMake
            && selected.element.is_some()
            && matches!(capture, Ok(None))
        {
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

    /// Reach the original name-object lookup before generic array observers.
    /// Missing roots remain missing, and only a genuine C issuer can use this.
    pub(crate) fn prepare_original_c_array_name(
        &mut self,
        original: *mut TclObj,
    ) -> Result<(), Code> {
        if self.native_c_variable_name_protocol().is_none() {
            return Err(self.report_cmd_error(
                ValueError::CommandProtocolUnavailable("original C array name lookup").into(),
            ));
        }
        self.prepare_original_c_name_for(original, NativeVariableNameLookupPurpose::Array)
            .map(|_| ())
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
        captured
            .receiver
            .ensure_array()
            .map_err(|error| self.original_c_empty_array_error(&captured.root, error))
    }
    fn original_c_empty_array_error(&mut self, root: &[u8], error: crate::frame::VarError) -> Code {
        if error != crate::frame::VarError::IsScalar {
            return crate::builtins::var_error(self, root, error);
        }
        let Some(protocol) = self.native_c_variable_name_protocol() else {
            return self.report_cmd_error(
                ValueError::CommandProtocolUnavailable("original C empty array diagnostic").into(),
            );
        };
        let mut message = b"can't array set \"".to_vec();
        message.extend_from_slice(tcl_core_types::c_string_extent(root));
        message.extend_from_slice(b"\": variable isn't array");
        self.original_c_array_set_error(
            message,
            protocol
                .array_set_has_specific_error_codes()
                .then_some(b"TCL WRITE ARRAY"),
            protocol.diagnostic_string_protocol(),
        )
    }

    fn original_c_array_set_error(
        &mut self,
        message: Vec<u8>,
        error_code: Option<&[u8]>,
        string_result: Option<tcl_syntax::native_string::NativeStringProtocol>,
    ) -> Code {
        self.report_cmd_error(tcl_cmd_core::CmdError::from_byte_details(
            tcl_cmd_core::CmdErrorDetails {
                message,
                string_result,
                error_code: error_code
                    .map_or(tcl_cmd_core::CmdErrorCodeUpdate::Unchanged, |code| {
                        tcl_cmd_core::CmdErrorCodeUpdate::Set(code.to_vec())
                    }),
                error_info: None,
                error_line: None,
                primitive_getter: None,
            },
        ))
    }

    /// Execute generic C array-set over original name/value headers. This is
    /// a handler path, independent of source compiler admission or opcodes.
    pub(crate) fn set_original_c_array(
        &mut self,
        original: *mut TclObj,
        values: *mut TclObj,
    ) -> Result<(), Code> {
        let protocol = self.native_c_variable_name_protocol().ok_or_else(|| {
            self.report_cmd_error(
                ValueError::CommandProtocolUnavailable("original C array set").into(),
            )
        })?;
        let original_bytes = self
            .native_string_bytes(&original)
            .map_err(|error| self.report_cmd_error(error.into()))?
            .to_vec();
        let captured = self.prepare_original_c_array_set(original, &original_bytes, protocol)?;
        // Root selection precedes conversion/parity checks; an error must not
        // erase its authentic name cache or invent a replacement receiver.
        let pairs = self.original_c_array_set_pairs(values, protocol)?;
        if pairs.is_empty() {
            return captured
                .receiver
                .ensure_array()
                .map_err(|error| self.original_c_empty_array_error(&captured.root, error));
        }
        match pairs {
            OriginalArraySetValues::Dictionary(pairs) => {
                // The active original argv owns the dictionary throughout its
                // search. Native key/value objects go directly to the store.
                for (key, value) in pairs {
                    self.store_original_c_array_member(
                        &captured,
                        &original_bytes,
                        key,
                        value,
                        protocol,
                    )?;
                }
            }
            OriginalArraySetValues::List { mut members, copy } => {
                let mut index = 0;
                while index < members.len() {
                    let pair = members.get(index..index + 2).ok_or_else(|| {
                        self.report_cmd_error(
                            ValueError::CommandProtocolUnavailable("original array-set list pair")
                                .into(),
                        )
                    })?;
                    self.store_original_c_array_member(
                        &captured,
                        &original_bytes,
                        pair[0],
                        pair[1],
                        protocol,
                    )?;
                    index += 2;
                    if copy.is_none() {
                        // C84 re-fetches the same original list after callbacks;
                        // later C pins its actual copied List backing instead.
                        members = ValueOps::list_elements(self, &values)
                            .map_err(|error| self.report_cmd_error(error.into()))?;
                    }
                }
                drop(copy);
            }
        }
        Ok(())
    }

    fn prepare_original_c_array_set(
        &mut self,
        original: *mut TclObj,
        original_bytes: &[u8],
        protocol: tcl_syntax::native_variable_name::NativeVariableNameProtocol,
    ) -> Result<OriginalCVariableCapture, Code> {
        let purpose = protocol.array_set_lookup_purpose();
        if purpose == NativeVariableNameLookupPurpose::ArrayMake
            && protocol.parsed_array_parts(original_bytes).is_some()
        {
            return Err(self.original_c_variable_failure_for_object(
                original,
                purpose,
                tcl_syntax::naming::NativeVariableDiagnosticReason::NotArray,
                tcl_syntax::naming::NativeVariableFailureSite::NameLookup,
            ));
        }
        let captured = self
            .capture_original_c_array_name(original, purpose)?
            .ok_or_else(|| {
                self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("original array-set root").into(),
                )
            })?;
        if captured.element.is_some() {
            return Err(self.original_c_variable_failure_for_object(
                original,
                purpose,
                tcl_syntax::naming::NativeVariableDiagnosticReason::NotArray,
                tcl_syntax::naming::NativeVariableFailureSite::NameLookup,
            ));
        }
        Ok(captured)
    }

    fn original_c_array_set_pairs(
        &mut self,
        original: *mut TclObj,
        protocol: tcl_syntax::native_variable_name::NativeVariableNameProtocol,
    ) -> Result<OriginalArraySetValues, Code> {
        if obj::obj_type_ptr(original) == &crate::dict::TCL_DICT_TYPE
            && protocol.array_set_uses_dictionary(obj::has_string_rep(original))
        {
            return ValueOps::dict_pairs(self, &original)
                .map(OriginalArraySetValues::Dictionary)
                .map_err(|error| self.report_cmd_error(error.into()));
        }
        let members = ValueOps::list_elements(self, &original)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        if members.len() % 2 != 0 {
            let message = b"list must have an even number of elements";
            return Err(self.original_c_array_set_error(
                message.to_vec(),
                protocol
                    .array_set_has_specific_error_codes()
                    .then_some(b"TCL ARGUMENT FORMAT"),
                protocol.array_set_parity_string_protocol(),
            ));
        }
        let copy = if !members.is_empty() && protocol.version() >= tcl_dialect::TclVersion::V8_5 {
            Some(
                crate::list::native_list_copy(
                    original,
                    tcl_syntax::native_string::NativeStringProtocol::C(protocol.version()),
                )
                .map_err(|error| self.report_cmd_error(error.into()))?,
            )
        } else {
            None
        };
        Ok(OriginalArraySetValues::List { members, copy })
    }

    fn store_original_c_array_member(
        &mut self,
        captured: &OriginalCVariableCapture,
        report_root: &[u8],
        original_key: *mut TclObj,
        value: *mut TclObj,
        protocol: tcl_syntax::native_variable_name::NativeVariableNameProtocol,
    ) -> Result<(), Code> {
        let raw_key = self
            .native_string_bytes(&original_key)
            .map_err(|error| self.report_cmd_error(error.into()))?
            .to_vec();
        let input = tcl_syntax::naming::NativeVariableInputForm::Separate {
            root: report_root,
            element: Some(&raw_key),
        };
        let key = tcl_syntax::naming::NativeNameProtocol::C(protocol.version())
            .separate_variable_input(&captured.root, Some(&raw_key))
            .element()
            .unwrap()
            .selected()
            .to_vec();
        let purpose = NativeVariableNameLookupPurpose::Write;
        captured.receiver.ensure_array().map_err(|error| {
            self.original_c_variable_receiver_error(
                input,
                purpose,
                error,
                Some(tcl_syntax::naming::NativeVariableFailureSite::NameLookup),
            )
        })?;
        let array = captured.receiver.selected_array().ok_or_else(|| {
            self.report_cmd_error(
                ValueError::CommandProtocolUnavailable("original array-set member table").into(),
            )
        })?;
        let receiver = array.capture_receiver(key.clone());
        receiver.retain_original_element_key(
            protocol
                .element_table_retains_original()
                .then_some(original_key),
        );
        let home = captured
            .home
            .for_selected_receiver(&receiver)
            .ok_or_else(|| {
                self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("original array-set member home").into(),
                )
            })?;
        receiver.store(value).map_err(|error| {
            self.original_c_variable_receiver_error(
                input,
                purpose,
                error,
                Some(tcl_syntax::naming::NativeVariableFailureSite::ValueWrite),
            )
        })?;
        if self.original_variable_trace_requires_name(&home, Some(&key), b"write") {
            let access = self.trace_access(report_root, &captured.root, Some(&key), &home, false);
            if self.fire_var_trace_resolved(&home, &access, b"write") {
                return Err(self.original_c_variable_receiver_error(
                    input,
                    purpose,
                    crate::frame::VarError::TraceError,
                    None,
                ));
            }
        }
        if self.host_refusal_pending() {
            return Err(Code::Error);
        }
        Ok(())
    }
}
