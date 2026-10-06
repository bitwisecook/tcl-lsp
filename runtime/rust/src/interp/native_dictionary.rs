// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Generic dictionary publication resolves its original name at the store boundary.

use super::{Code, Interp};
use crate::value_ops::{RuntimeAppendValue, RuntimeDictionaryObjects};
use tcl_cmd_core::native_dictionary::NativeDictionaryObjects;

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
