// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native append keeps one physical receiver across operand publication boundaries.

use super::{Code, Interp, TraceAccess};
use crate::frame::VariableReceiver;
use crate::obj::TclObj;
use crate::value_ops::{RuntimeAppendObjects, RuntimeAppendValue};
use tcl_cmd_core::native_append::{
    append_continuation, append_object, append_operands, NativeAppendVariable, PreparedAppendValue,
};
use tcl_syntax::native_object_append::NativeObjectAppendProtocol;

struct AppendVariable<'a> {
    interp: &'a mut Interp,
    name: &'a [u8],
    receiver: VariableReceiver,
    home: crate::vars::TraceHome,
    access: TraceAccess,
    protocol: NativeObjectAppendProtocol,
    objects: RuntimeAppendObjects,
    pending: Option<PreparedAppendValue<RuntimeAppendValue>>,
}

impl NativeAppendVariable for AppendVariable<'_> {
    type Value = RuntimeAppendValue;
    type Error = Code;

    fn append_operand(&mut self, source: &RuntimeAppendValue) -> Result<(), Code> {
        if let Some(pending) = &mut self.pending {
            return append_continuation(&self.objects, pending, source)
                .map_err(|error| self.interp.report_cmd_error(error.into()));
        }
        let original = self
            .receiver
            .read_initial()
            .map_err(|error| crate::builtins::var_error(self.interp, self.name, error))?
            .map(RuntimeAppendValue::borrowed);
        let prepared = append_object(&self.objects, self.protocol, original.as_ref(), source)
            .map_err(|error| self.interp.report_cmd_error(error.into()))?;
        self.pending = Some(prepared);
        Ok(())
    }

    fn publish(&mut self) -> Result<RuntimeAppendValue, Code> {
        let pending = self
            .pending
            .take()
            .expect("pending native append value")
            .into_value();
        self.receiver
            .store(pending.as_ptr())
            .map_err(|error| crate::builtins::var_error(self.interp, self.name, error))?;
        // The variable owns the object before observers run. A protective
        // working hold must not alter their native copy-on-write observations.
        drop(pending);
        if self
            .interp
            .fire_var_trace_resolved(&self.home, &self.access, b"write")
        {
            return Err(crate::builtins::var_error(
                self.interp,
                self.name,
                crate::frame::VarError::TraceError,
            ));
        }
        if self.interp.host_refusal_pending() {
            return Err(Code::Error);
        }
        match self.receiver.read() {
            Ok(Some(value)) => Ok(RuntimeAppendValue::borrowed(value)),
            Ok(None) | Err(_) => Ok(RuntimeAppendValue::fresh_string(b"")),
        }
    }
}

impl Interp {
    /// `TCL_APPEND_VALUE` with counted bytes reaches the selected variable's
    /// read/write observers and String append, without replacing the result.
    pub(super) fn append_native_counted_variable_bytes(
        &mut self,
        name: &[u8],
        bytes: &[u8],
    ) -> Result<(), Code> {
        let dialect = self.native_invocation_dialect();
        let protocol = dialect.native_object_append_protocol(None).ok_or_else(|| {
            self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native counted variable append",
                ),
            )
        })?;
        let receiver = crate::vars::capture_variable_receiver(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            name,
            None,
        );
        let receiver = receiver.map_err(|error| crate::builtins::var_error(self, name, error))?;
        let home = self.trace_identity(name);
        let access = self.trace_access(name, name, None, &home, false);
        if self.fire_var_trace_resolved(&home, &access, b"read") {
            return Err(crate::builtins::var_error(
                self,
                name,
                crate::frame::VarError::TraceError,
            ));
        }
        if self.host_refusal_pending() {
            return Err(Code::Error);
        }
        let original = receiver
            .read_initial()
            .map_err(|error| crate::builtins::var_error(self, name, error))?
            .map_or_else(
                || RuntimeAppendValue::fresh_string(b""),
                RuntimeAppendValue::borrowed,
            );
        let objects = RuntimeAppendObjects {
            dialect,
            binary_recipe: dialect.byte_array_string_recipe(None),
        };
        let working = tcl_cmd_core::native_append::append_counted_bytes(
            &objects,
            protocol.recipe(),
            &original,
            bytes,
        )
        .map_err(|error| self.report_cmd_error(error.into()))?;
        receiver
            .store(working.as_ptr())
            .map_err(|error| crate::builtins::var_error(self, name, error))?;
        drop(working);
        drop(original);
        if self.fire_var_trace_resolved(&home, &access, b"write") {
            return Err(crate::builtins::var_error(
                self,
                name,
                crate::frame::VarError::TraceError,
            ));
        }
        if self.host_refusal_pending() {
            return Err(Code::Error);
        }
        Ok(())
    }

    pub(crate) fn append_native_operands(
        &mut self,
        name: &[u8],
        base: &[u8],
        element: Option<&[u8]>,
        sources: &[*mut TclObj],
    ) -> Code {
        let dialect = self.native_invocation_dialect();
        let issued =
            match dialect.native_object_append_protocol(Some(
                tcl_registry::native_object_append::LogicalAppendProvider::Tcl84CoreSimulation,
            )) {
                Some(issued) => issued,
                None => return self.refuse_native_access(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "native object append",
                    ),
                ),
            };
        let receiver_result = crate::vars::capture_variable_receiver(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            base,
            element,
        );
        let receiver = match receiver_result {
            Ok(receiver) => receiver,
            Err(error) => return crate::builtins::var_error(self, name, error),
        };
        let home = self.trace_identity(base);
        let access = self.trace_access(name, base, element, &home, false);
        self.append_retained_operands(name, receiver, home, access, issued.recipe(), sources)
    }

    /// An admitted variable instruction has already selected its physical
    /// receiver. This door does not resolve its reporting name again.
    pub(super) fn append_native_captured(
        &mut self,
        captured: super::native_variable_names::OriginalCVariableCapture,
        sources: &[*mut TclObj],
    ) -> Code {
        let dialect = self.native_invocation_dialect();
        let Some(protocol) = dialect.native_object_append_protocol(None) else {
            return self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native compiled append",
                ),
            );
        };
        let access = self.trace_access(
            &captured.root,
            &captured.root,
            captured.element.as_deref(),
            &captured.home,
            false,
        );
        self.append_retained_operands(
            &captured.root,
            captured.receiver,
            captured.home,
            access,
            protocol.recipe(),
            sources,
        )
    }

    fn append_retained_operands(
        &mut self,
        name: &[u8],
        receiver: VariableReceiver,
        home: crate::vars::TraceHome,
        access: TraceAccess,
        protocol: NativeObjectAppendProtocol,
        sources: &[*mut TclObj],
    ) -> Code {
        if receiver.is_constant() {
            return crate::builtins::var_error(self, name, crate::frame::VarError::IsConstant);
        }
        if receiver.is_array() && !receiver.is_element() {
            return crate::builtins::var_error(self, name, crate::frame::VarError::IsArray);
        }
        let dialect = self.native_invocation_dialect();
        let objects = RuntimeAppendObjects {
            dialect,
            binary_recipe: dialect.byte_array_string_recipe(Some(tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation)),
        };
        let sources: Vec<_> = sources
            .iter()
            .copied()
            .map(RuntimeAppendValue::borrowed)
            .collect();
        let mut operation = AppendVariable {
            interp: self,
            name,
            receiver,
            home,
            access,
            protocol,
            objects,
            pending: None,
        };
        match append_operands(&mut operation, protocol, &sources) {
            Ok(value) => {
                operation.interp.set_result(value.as_ptr());
                Code::Ok
            }
            Err(code) => code,
        }
    }
}
