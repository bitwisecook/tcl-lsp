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

//! Generic dictionary publication resolves its original name at the store boundary.

use super::{Code, Interp};
use crate::value_ops::{RuntimeAppendValue, RuntimeDictionaryObjects};
use tcl_cmd_core::native_dictionary::NativeDictionaryObjects;
use tcl_syntax::native_variable_name::NativeVariableNameLookupPurpose as Purpose;
use tcl_syntax::value::ValueOps;

impl Interp {
    pub(crate) fn dictionary_variable_update(
        &mut self,
        name: &[u8],
        objects: &RuntimeDictionaryObjects,
        operation: impl FnOnce(
            &mut Self,
            crate::dict::PreparedNativeDictionary,
        ) -> Result<RuntimeAppendValue, tcl_cmd_core::CmdError>,
    ) -> Code {
        match self.native_invocation_dialect().native_dictionary_variable_lookup() {
            Some(tcl_registry::native_dictionary::NativeDictionaryVariableLookup::ExistingRootWithElementCreation) => {}
            None => return self.report_cmd_error(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native dictionary variable lookup").into()),
        }
        let (base, element) = match self.variable_name_parts(name) {
            Ok(parts) => parts,
            Err(error) => return crate::builtins::var_error(self, name, error),
        };
        let selected_result = crate::vars::capture_get_variable_receiver(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            &base,
            element.as_deref(),
        );
        let selected = match selected_result {
            Ok(selected) => selected,
            Err(error) => return crate::builtins::var_error(self, name, error),
        };
        let original = if let Some(receiver) = &selected {
            let home = self.trace_identity(&base);
            let access = self.trace_access(name, &base, element.as_deref(), &home, false);
            let failed = self.fire_var_trace_resolved(&home, &access, b"read");
            if self.host_refusal_pending() {
                return Code::Error;
            }
            let value = if failed {
                None
            } else {
                receiver.read_initial().ok().flatten()
            };
            if value.is_none() {
                // TclPtrGetVarIdx retains this primitive state even when its
                // flags omit guest result presentation and the caller proceeds.
                self.traces.borrow_mut().pending_err.take();
                let mut exception = self.exc.borrow_mut();
                exception.code = b"TCL READ VARNAME".to_vec();
                exception.code_explicit = false;
            }
            value.map(RuntimeAppendValue::borrowed)
        } else {
            None
        };
        let prepared = match objects.prepare(original.as_ref()) {
            Ok(prepared) => prepared,
            Err(error) => return self.report_cmd_error(error),
        };
        drop(selected);
        let value = match operation(self, prepared) {
            Ok(value) => value,
            Err(error) => return self.report_cmd_error(error),
        };
        // Generic Tcl dictionary commands do a new lookup here. The initial
        // read receiver does not donate its identity to publication.
        let receiver_result = crate::vars::capture_variable_receiver(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            &base,
            element.as_deref(),
        );
        let receiver = match receiver_result {
            Ok(receiver) => receiver,
            Err(error) => return crate::builtins::var_error(self, name, error),
        };
        let home = self.trace_identity(&base);
        let access = self.trace_access(name, &base, element.as_deref(), &home, false);
        if let Err(error) = receiver.store(value.as_ptr()) {
            return crate::builtins::var_error(self, name, error);
        }
        drop(value);
        if self.fire_var_trace_resolved(&home, &access, b"write") {
            return crate::builtins::var_error(self, name, crate::frame::VarError::TraceError);
        }
        if self.host_refusal_pending() {
            return Code::Error;
        }
        match receiver.read() {
            Ok(Some(value)) => self.set_result(value),
            Ok(None) | Err(_) => self.set_result_bytes(b""),
        }
        Code::Ok
    }
}

/// Original dictionary scope reads use separate initial and epilogue flags.
#[derive(Clone, Copy)]
pub(crate) enum DictionaryScopeRead {
    Initial,
    UpdateWriteback,
    WithWriteback,
}

impl Interp {
    pub(crate) fn dictionary_scope_variable_read(
        &mut self,
        original: *mut crate::obj::TclObj,
        phase: DictionaryScopeRead,
    ) -> Result<Option<*mut crate::obj::TclObj>, Code> {
        if matches!(phase, DictionaryScopeRead::Initial) {
            return self.read_original_named_variable(original).map(Some);
        }
        if self.native_c_variable_name_protocol().is_none() {
            // Jim has no C variable trace/receiver protocol. Keep its selected
            // name grammar and existing missing-root scope behavior.
            let bytes = self
                .native_string_bytes(&original)
                .map_err(|error| self.report_cmd_error(error.into()))?;
            let (root, element) = self
                .variable_name_parts(&bytes)
                .map_err(|error| crate::builtins::var_error(self, &bytes, error))?;
            return Ok(match element {
                Some(element) => self.var_get_elem(&root, &element),
                None => self.var_get(&root),
            });
        }
        let purpose = match phase {
            DictionaryScopeRead::Initial => unreachable!(),
            DictionaryScopeRead::UpdateWriteback => Purpose::Exists,
            DictionaryScopeRead::WithWriteback => Purpose::QuietWrite,
        };
        let selected = match self.capture_original_c_variable_report(original, purpose) {
            Ok(selected) => selected,
            Err(code) if self.host_refusal_pending() => return Err(code),
            Err(_) => return Ok(None),
        };
        let Some(selected) = selected else {
            return Ok(None);
        };
        let failed = if self.original_variable_trace_requires_name(
            &selected.home,
            selected.element.as_deref(),
            b"read",
        ) {
            let bytes = self
                .native_string_bytes(&original)
                .map_err(|error| self.report_cmd_error(error.into()))?;
            let access = self.trace_access(
                &bytes,
                &selected.root,
                selected.element.as_deref(),
                &selected.home,
                false,
            );
            self.fire_var_trace_resolved(&selected.home, &access, b"read")
        } else {
            false
        };
        if self.host_refusal_pending() {
            return Err(Code::Error);
        }
        let value = if failed {
            None
        } else {
            selected.receiver.read_initial().ok().flatten()
        };
        if value.is_none() {
            self.traces.borrow_mut().pending_err.take();
            {
                let mut exception = self.exc.borrow_mut();
                exception.code = b"TCL READ VARNAME".to_vec();
                exception.code_explicit = false;
            }
            self.replace_native_error_code(b"TCL READ VARNAME");
        }
        Ok(value)
    }
}

/// The private error headers saved at a reached native dictionary epilogue.
pub(crate) struct DictionaryScopeErrorState {
    original: Option<super::native_error_variables::NativeErrorTraceState>,
}

impl Interp {
    pub(crate) fn save_dictionary_scope_error(&self) -> DictionaryScopeErrorState {
        DictionaryScopeErrorState {
            original: self.save_native_error_trace_state(),
        }
    }

    pub(crate) fn restore_dictionary_scope_error(&self, state: DictionaryScopeErrorState) {
        if let Some(original) = state.original {
            self.restore_native_error_trace_state(original);
        }
    }
}
