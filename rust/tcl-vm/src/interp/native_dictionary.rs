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

//! Native dictionary variable publication over explicit physical receivers.

use super::{CapturedVariableUpdate, Local, Value, VarTraceCell, Vm};
use tcl_cmd_core::native_dictionary::NativeDictionaryObjects;
use tcl_runtime_api::{Completion, VariableUpdateResult};

/// The native caller's write receiver, independent of dictionary transformation.
#[derive(Clone, Copy)]
pub(crate) enum DictionaryVariablePublication {
    /// Generic Tcl command: resolve its original name again for publication.
    CommandName,
    /// Compiled local opcode: keep the selected cell across read callbacks.
    RetainedLocalCell,
    /// Indexed opcode: retain exactly this slot, including duplicate formals.
    RetainedCompiledLocal(usize),
}

impl Vm {
    fn dictionary_get_cell(
        &mut self,
        name: &[u8],
    ) -> Result<Option<(VarTraceCell, Option<tcl_core_types::VarId>)>, Completion<Value>> {
        self.native_invocation_dialect()
            .native_dictionary_variable_lookup()
            .ok_or_else(|| {
                self.refuse_host_command("native dictionary initial lookup is unavailable".into())
            })?;
        let Some(mut resolved) = self.resolve_var_from_bytes(name, self.current_level()) else {
            return Ok(None);
        };
        if resolved.elem.is_some() {
            let Some(array) = resolved.base_id else {
                return Ok(None);
            };
            if !self
                .var_arena
                .get(array)
                .is_some_and(|cell| matches!(cell.state(), Local::Array(_)))
            {
                return Ok(None);
            }
            self.prepare_existing_array_read(&mut resolved)?;
            return Ok(self
                .trace_cell_from_resolved(&resolved)
                .map(|cell| (cell, resolved.base_id)));
        }
        Ok(self
            .trace_cell_from_resolved(&resolved)
            .map(|cell| (cell, None)))
    }

    pub(crate) fn dictionary_variable_update_bytes<O: NativeDictionaryObjects<Value = Value>>(
        &mut self,
        name: &[u8],
        publication: DictionaryVariablePublication,
        objects: &O,
        operation: impl FnOnce(&mut Self, O::Prepared) -> Result<Value, Completion<Value>>,
    ) -> Result<VariableUpdateResult<Value>, Completion<Value>> {
        if self.name_policy_protocol().is_none() {
            return Err(
                self.refuse_host_command("native dictionary name policy is unavailable".into())
            );
        }
        match publication {
            DictionaryVariablePublication::RetainedCompiledLocal(slot) => {
                let (original, captured) = self.capture_compiled_update(slot, None)?;
                self.with_variable_operation(&captured.cell, |vm| {
                    vm.dictionary_update_selected(
                        original.as_bytes(),
                        objects,
                        Some(&captured),
                        Some(&captured.cell),
                        captured.explicit_array,
                        operation,
                    )
                })
            }
            DictionaryVariablePublication::RetainedLocalCell => {
                let captured = self.capture_update_cell(name, None)?;
                self.with_variable_operation(&captured.cell, |vm| {
                    vm.dictionary_update_selected(
                        name,
                        objects,
                        Some(&captured),
                        Some(&captured.cell),
                        captured.explicit_array,
                        operation,
                    )
                })
            }
            DictionaryVariablePublication::CommandName => {
                let selected = self.dictionary_get_cell(name)?;
                match selected {
                    Some((cell, explicit_array)) => self.with_variable_operation(&cell, |vm| {
                        vm.dictionary_update_selected(
                            name,
                            objects,
                            None,
                            Some(&cell),
                            explicit_array,
                            operation,
                        )
                    }),
                    None => {
                        self.dictionary_update_selected(name, objects, None, None, None, operation)
                    }
                }
            }
        }
    }

    /// The Jim core path worker owns ordinary variable access; the C primitive
    /// lookup/read frontier and retained compiled receivers do not select it.
    pub(crate) fn jim_dictionary_path_update_bytes<O: NativeDictionaryObjects<Value = Value>>(
        &mut self,
        name: &[u8],
        objects: &O,
        operation: impl FnOnce(&mut Self, O::Prepared) -> Result<Value, Completion<Value>>,
    ) -> Result<VariableUpdateResult<Value>, Completion<Value>> {
        if self.native_invocation_dialect().native_dictionary_path_publication()
            != Some(tcl_registry::native_dictionary::NativeDictionaryPathPublication::JimOriginalVariable)
        {
            return Err(self.refuse_host_command("original Jim dictionary path worker is unavailable".into()));
        }
        // Jim_SetDictKeysVector publishes this fresh root before conversion or
        // a failing intermediate-key lookup. Prepare borrows the actual stored
        // member/root without adding a caller handle before its COW decision.
        if self.get_var_bytes(name).is_none() {
            self.store_var_result_bytes(name, Value::dict(Vec::new()))?;
        }
        let prepared = self
            .prepare_dictionary_variable_container(name, objects)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let dictionary = operation(self, prepared)?;
        let stored = self.store_var_result_bytes(name, dictionary)?;
        Ok(Self::variable_update_result(stored, &Value::empty()))
    }

    pub(crate) fn start_compiled_dictionary_update(
        &mut self,
        root_slot: usize,
        key_list: &Value,
        target_slots: &[usize],
    ) -> Result<(), Completion<Value>> {
        let dictionary = self.read_compiled_variable_result(root_slot, None)?;
        let objects = crate::cmd_dict::VmDictionaryObjects::selected(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let keys = self
            .native_object_list_elements_in(key_list, objects.string_protocol())
            .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?;
        if keys.len() != target_slots.len() {
            return Err(
                self.refuse_host_command("dictionary update slot/key count mismatch".into())
            );
        }
        for (key, slot) in keys.iter().zip(target_slots) {
            let pairs = self
                .native_object_dict_pairs_in(&dictionary, objects.string_protocol())
                .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?;
            let wanted = key
                .native_string_bytes(objects.string_protocol())
                .map_err(|error| self.refuse_host_command(error.to_string()))?;
            let mut member = None;
            for (stored, value) in pairs {
                let bytes = stored
                    .native_string_bytes(objects.string_protocol())
                    .map_err(|error| self.refuse_host_command(error.to_string()))?;
                if bytes == wanted {
                    member = Some(value);
                    break;
                }
            }
            if let Some(value) = member {
                self.store_compiled_variable_result(*slot, None, value)?;
            } else {
                // TclObjUnsetVar2 uses the original local-name object here.
                let Some(binding) = self.compiled_local_binding(*slot) else {
                    return Err(self.refuse_host_command(
                        "dictionary update target slot is unavailable".into(),
                    ));
                };
                let _ = self.unset_var_bytes(binding.name.as_bytes());
                if let Some(refusal) = self.refused_completion() {
                    return Err(refusal);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn finish_compiled_dictionary_update(
        &mut self,
        root_slot: usize,
        key_list: &Value,
        target_slots: &[usize],
    ) -> Result<(), Completion<Value>> {
        let objects = crate::cmd_dict::VmDictionaryObjects::selected(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?
            .with_preparation(
                tcl_cmd_core::native_dictionary::NativeDictionaryPreparation::ConvertBeforeCopy,
            );
        let (name, captured) = self.capture_compiled_update(root_slot, None)?;
        self.with_variable_operation(&captured.cell, |vm| {
            let read = vm.fire_var_traces_from_cell_bytes(
                name.as_bytes(),
                "read",
                None,
                None,
                Some(captured.cell.clone()),
            );
            if let Some(refusal) = vm.refused_completion() {
                return Err(refusal);
            }
            if read.is_err() {
                vm.publish_swallowed_trace_error();
                return Ok(());
            }
            let current = match vm.initial_value_at_cell(
                captured.cell.id.expect("captured dictionary receiver"),
                captured.explicit_array,
            ) {
                Ok(current) => current,
                Err(error) => {
                    return Err(crate::command::completion_from_cmd_error(vm, error.into()));
                }
            };
            let Some(current) = current else {
                return Ok(());
            };
            // Native conversion precedes key-list validation and root sharing selection.
            vm.native_object_dict_pairs_in(current, objects.string_protocol())
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
            let keys = vm
                .native_object_list_elements_in(key_list, objects.string_protocol())
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
            if keys.len() != target_slots.len() {
                return Err(
                    vm.refuse_host_command("dictionary update slot/key count mismatch".into())
                );
            }
            let current = match vm.initial_value_at_cell(
                captured.cell.id.expect("captured dictionary receiver"),
                captured.explicit_array,
            ) {
                Ok(current) => current,
                Err(error) => {
                    return Err(crate::command::completion_from_cmd_error(vm, error.into()));
                }
            };
            let mut prepared = objects
                .prepare(current)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
            if !keys.is_empty() {
                prepared.invalidate_string();
            }
            for (key, slot) in keys.iter().zip(target_slots) {
                if let Ok(value) = vm.read_compiled_variable_result(*slot, None) {
                    let value = if prepared.is_same_object(&value) {
                        prepared.duplicate_value(&value)
                    } else {
                        value
                    };
                    prepared.set_member(key.clone(), value).map_err(|error| {
                        crate::command::completion_from_cmd_error(vm, error.into())
                    })?;
                } else {
                    if let Some(refusal) = vm.refused_completion() {
                        return Err(refusal);
                    }
                    vm.publish_swallowed_trace_error();
                    prepared.remove_member(key).map_err(|error| {
                        crate::command::completion_from_cmd_error(vm, error.into())
                    })?;
                }
            }
            vm.store_captured_update(name.as_bytes(), None, &captured, prepared.into_value())?;
            Ok(())
        })
    }

    /// A scope epilogue skips a failed/missing read instead of manufacturing
    /// an empty root. Its caller owns the preserved body completion.
    pub(crate) fn dictionary_scope_writeback_bytes(
        &mut self,
        name: &[u8],
        publication: DictionaryVariablePublication,
        objects: &crate::cmd_dict::VmDictionaryObjects,
        operation: impl FnOnce(
            &mut Self,
            crate::value::PreparedNativeDictionary,
        ) -> Result<Option<Value>, Completion<Value>>,
    ) -> Result<(), Completion<Value>> {
        let captured = match publication {
            DictionaryVariablePublication::RetainedCompiledLocal(slot) => {
                Some(self.capture_compiled_update(slot, None)?.1)
            }
            DictionaryVariablePublication::RetainedLocalCell => {
                Some(self.capture_update_cell(name, None)?)
            }
            DictionaryVariablePublication::CommandName => None,
        };
        let selected = captured
            .as_ref()
            .map(|receiver| receiver.cell.clone())
            .or_else(|| self.trace_cell_bytes(name).filter(|cell| cell.id.is_some()));
        let explicit_array = captured.as_ref().map_or_else(
            || {
                self.resolve_var_from_bytes(name, self.current_level())
                    .filter(|resolved| resolved.elem.is_some())
                    .and_then(|resolved| resolved.base_id)
            },
            |receiver| receiver.explicit_array,
        );
        let execute = |vm: &mut Self| {
            let read =
                vm.fire_var_traces_from_cell_bytes(name, "read", None, None, selected.clone());
            if let Some(refusal) = vm.refused_completion() {
                return Err(refusal);
            }
            if read.is_err() {
                vm.publish_swallowed_trace_error();
                return Ok(());
            }
            let current = if let Some(id) = selected.as_ref().and_then(|cell| cell.id) {
                match vm.initial_value_at_cell(id, explicit_array) {
                    Ok(current) => current,
                    Err(error) => {
                        return Err(crate::command::completion_from_cmd_error(vm, error.into()));
                    }
                }
            } else {
                None
            };
            let Some(current) = current else {
                return Ok(());
            };
            let prepared = objects
                .prepare(Some(current))
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
            let Some(value) = operation(vm, prepared)? else {
                return Ok(());
            };
            match captured.as_ref() {
                Some(receiver) => {
                    vm.store_captured_update(name, None, receiver, value)?;
                }
                None => {
                    vm.store_var_result_bytes(name, value)?;
                }
            }
            Ok(())
        };
        match selected.as_ref() {
            Some(cell) => self.with_variable_operation(cell, execute),
            None => execute(self),
        }
    }

    fn dictionary_update_selected<O: NativeDictionaryObjects<Value = Value>>(
        &mut self,
        name: &[u8],
        objects: &O,
        captured: Option<&CapturedVariableUpdate>,
        selected: Option<&VarTraceCell>,
        explicit_array: Option<tcl_core_types::VarId>,
        operation: impl FnOnce(&mut Self, O::Prepared) -> Result<Value, Completion<Value>>,
    ) -> Result<VariableUpdateResult<Value>, Completion<Value>> {
        let trace = if let Some(cell) = selected {
            self.fire_var_traces_from_cell_bytes(name, "read", None, None, Some(cell.clone()))
        } else {
            Ok(())
        };
        if let Some(refusal) = self.refused_completion() {
            return Err(refusal);
        }
        let (prepared, read_options) = match trace {
            Ok(()) => {
                let prepared = if self.dictionary_variable_containers() {
                    self.prepare_dictionary_variable_container(name, objects)
                } else {
                    let id = selected.and_then(|cell| cell.id).or_else(|| {
                        self.resolve_var_from_bytes(name, self.current_level())
                            .and_then(|resolved| resolved.id)
                    });
                    let current = if let Some(id) = id {
                        match self.initial_value_at_cell(id, explicit_array) {
                            Ok(current) => current,
                            Err(error) => {
                                return Err(crate::command::completion_from_cmd_error(
                                    self,
                                    error.into(),
                                ));
                            }
                        }
                    } else {
                        None
                    };
                    objects.prepare(current)
                }
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
                (prepared, Value::empty())
            }
            Err(error) => {
                let options = self.completion_options_snapshot(&error);
                self.publish_swallowed_trace_error();
                let prepared = objects
                    .prepare(None)
                    .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
                (prepared, options)
            }
        };
        if let Some(refusal) = self.refused_completion() {
            return Err(refusal);
        }
        self.retain_variable_read_error_code(&read_options);
        let dictionary = operation(self, prepared)?;
        let stored = match captured {
            Some(captured) => self.store_captured_update(name, None, captured, dictionary)?,
            None => self.store_var_result_bytes(name, dictionary)?,
        };
        if let Some(refusal) = self.refused_completion() {
            return Err(refusal);
        }
        Ok(Self::variable_update_result(stored, &read_options))
    }

    fn prepare_dictionary_variable_container<O: NativeDictionaryObjects<Value = Value>>(
        &self,
        name: &[u8],
        objects: &O,
    ) -> Result<O::Prepared, tcl_cmd_core::CmdError> {
        let resolved = self.resolve_var_from_bytes(name, self.current_level());
        let Some(resolved) = resolved else {
            return objects.prepare(None);
        };
        let id = resolved.base_id.or(resolved.id);
        let root = id
            .and_then(|id| self.var_arena.get(id))
            .and_then(|cell| match cell.state() {
                Local::Scalar(value) => Some(value),
                _ => None,
            });
        if let Some(key) = resolved.elem.as_ref() {
            let Some(root) = root else {
                return objects.prepare(None);
            };
            let pairs = self.native_variable_dict_pairs(root)?;
            drop(pairs);
            root.with_cached_dictionary_member(key, |member| objects.prepare(member))
                .ok_or_else(|| {
                    tcl_cmd_core::CmdError::from(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "native dictionary member owner",
                        ),
                    )
                })?
        } else {
            objects.prepare(root)
        }
    }
}
