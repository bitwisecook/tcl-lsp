// SPDX-License-Identifier: AGPL-3.0-or-later
//! Retain physical append receivers across the selected variable publication boundaries.

use super::{CapturedVariableUpdate, Local, ResolvedVar, Value, VarId, Vm};
use crate::value::VmAppendObjects;
use tcl_cmd_core::native_append::{
    NativeAppendVariable, PreparedAppendValue, append_continuation, append_object, append_operands,
};
use tcl_runtime_api::Completion;
use tcl_syntax::native_object_append::NativeObjectAppendProtocol;

struct CellAppend<'a> {
    vm: &'a mut Vm,
    name: &'a [u8],
    key: Option<&'a [u8]>,
    captured: &'a CapturedVariableUpdate,
    protocol: NativeObjectAppendProtocol,
    pending: Option<PreparedAppendValue<Value>>,
}

impl NativeAppendVariable for CellAppend<'_> {
    type Value = Value;
    type Error = Completion<Value>;

    fn append_operand(&mut self, source: &Value) -> Result<(), Self::Error> {
        self.vm
            .check_captured_update_with_errors(self.name, self.key, self.captured, true)?;
        let id = self.captured.cell.id.expect("captured append cell");
        // Borrow actual table ownership: a getter clone would falsely make an
        // otherwise unshared receiver shared before the native COW decision.
        if let Some(pending) = &mut self.pending {
            return append_continuation(&VmAppendObjects, pending, source)
                .map_err(|error| crate::command::completion_from_cmd_error(self.vm, error.into()));
        }
        let value = self
            .vm
            .initial_value_at_cell(id, self.captured.explicit_array)
            .and_then(|receiver| append_object(&VmAppendObjects, self.protocol, receiver, source))
            .map_err(|error| crate::command::completion_from_cmd_error(self.vm, error.into()))?;
        self.pending = Some(value);
        Ok(())
    }

    fn publish(&mut self) -> Result<Value, Self::Error> {
        self.vm.store_captured_update(
            self.name,
            self.key,
            self.captured,
            self.pending
                .take()
                .expect("pending append value")
                .into_value(),
        )
    }
}

struct DictionaryAppend<'a> {
    vm: &'a mut Vm,
    root: ResolvedVar,
    id: VarId,
    key: tcl_core_types::NameBytes,
    protocol: NativeObjectAppendProtocol,
    pending: Option<PreparedAppendValue<Value>>,
}

impl NativeAppendVariable for DictionaryAppend<'_> {
    type Value = Value;
    type Error = Completion<Value>;

    fn append_operand(&mut self, source: &Value) -> Result<(), Self::Error> {
        if let Some(pending) = &mut self.pending {
            return append_continuation(&VmAppendObjects, pending, source)
                .map_err(|error| crate::command::completion_from_cmd_error(self.vm, error.into()));
        }
        let root_copy = match self
            .vm
            .var_arena
            .get(self.id)
            .map(crate::vars::VarCell::state)
        {
            Some(Local::Scalar(root)) => self.vm.native_variable_dict_pairs(root).map(|pairs| {
                drop(pairs);
                let present = root
                    .with_cached_dictionary_member(&self.key, |value| value.is_some())
                    .expect("native Dictionary conversion");
                (present && root.native_object_is_shared())
                    .then(|| root.duplicate_native_object_in(self.protocol.string_protocol()))
            }),
            _ => Ok(None),
        }
        .map_err(|error| crate::command::completion_from_cmd_error(self.vm, error.into()))?;
        if let Some(copy) = root_copy {
            let _ = self
                .vm
                .var_arena
                .replace_state(self.id, Local::Scalar(copy));
        }
        let value = {
            match self
                .vm
                .var_arena
                .get(self.id)
                .map(crate::vars::VarCell::state)
            {
                Some(Local::Scalar(root)) => {
                    // Conversion owns the root cache, then all temporary member
                    // handles are dropped before borrowing the original member.
                    self.vm.native_variable_dict_pairs(root).and_then(|pairs| {
                        drop(pairs);
                        root.with_cached_dictionary_member(&self.key, |receiver| {
                            append_object(&VmAppendObjects, self.protocol, receiver, source)
                        })
                        .expect("native Dictionary conversion installs primary members")
                    })
                }
                _ => append_object(&VmAppendObjects, self.protocol, None, source),
            }
        }
        .map_err(|error| crate::command::completion_from_cmd_error(self.vm, error.into()))?;
        self.pending = Some(value);
        Ok(())
    }

    fn publish(&mut self) -> Result<Value, Self::Error> {
        let value = self
            .pending
            .take()
            .expect("pending append member")
            .into_value();
        let key = Value::new_native_string_bytes(self.key.as_ref());
        let root = match self
            .vm
            .var_arena
            .get(self.id)
            .map(crate::vars::VarCell::state)
        {
            Some(Local::Scalar(root)) => {
                root.native_dictionary_set_member(key, value, self.protocol.string_protocol())
            }
            _ => Value::from_native_dictionary_cache(Vec::new()).native_dictionary_set_member(
                key,
                value,
                self.protocol.string_protocol(),
            ),
        }
        .map_err(|error| crate::command::completion_from_cmd_error(self.vm, error.into()))?;
        self.vm.store_dictionary_root(&self.root, root)?;
        Ok(self
            .vm
            .read_resolved_elem(self.id, &self.key)
            .unwrap_or_else(Value::empty))
    }
}

impl Vm {
    /// Append through one selected cell; C writes each operand, Jim writes the batch.
    pub(crate) fn append_captured_bytes(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        sources: &[Value],
    ) -> Result<Value, Completion<Value>> {
        self.append_selected_bytes(name, key, sources, None)
    }

    pub(super) fn append_selected_bytes(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        sources: &[Value],
        selected: Option<ResolvedVar>,
    ) -> Result<Value, Completion<Value>> {
        let issued = self
            .native_invocation_dialect()
            .native_object_append_protocol(Some(
                tcl_registry::native_object_append::LogicalAppendProvider::Tcl84CoreSimulation,
            ))
            .ok_or_else(|| {
                self.refuse_host_command("native object append protocol is unavailable".into())
            })?;
        let protocol = issued.recipe();
        if self.dictionary_variable_containers() {
            let selected = if key.is_some() {
                self.resolve_var_parts_from_bytes(name, key, self.current_level())
            } else {
                self.resolve_var_from_bytes(name, self.current_level())
            }
            .ok_or_else(|| {
                self.variable_access_error_bytes("set", name, "parent namespace doesn't exist")
            })?;
            if let Some(element) = selected.elem {
                let root = self
                    .resolve_binding_var(
                        selected.binding,
                        None,
                        &mut std::collections::HashSet::new(),
                    )
                    .ok_or_else(|| {
                        self.variable_access_error_bytes("set", name, "variable link cycle")
                    })?;
                let id = root
                    .id
                    .or_else(|| self.ensure_target_var_at_binding(&root.binding, None))
                    .ok_or_else(|| {
                        self.refuse_host_command(
                            "native dictionary append root is unavailable".into(),
                        )
                    })?;
                let mut operation = DictionaryAppend {
                    vm: self,
                    root,
                    id,
                    key: element,
                    protocol,
                    pending: None,
                };
                return append_operands(&mut operation, protocol, sources);
            }
        }
        let captured = match selected {
            Some(resolved) => self.capture_selected_update(name, key, &resolved)?,
            None => self.capture_update_cell(name, key)?,
        };
        self.with_variable_operation(&captured.cell, |vm| {
            let mut operation = CellAppend {
                vm,
                name,
                key,
                captured: &captured,
                protocol,
                pending: None,
            };
            append_operands(&mut operation, protocol, sources)
        })
    }
}
