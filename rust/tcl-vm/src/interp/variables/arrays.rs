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

//! Array access: creating an array, reading, writing and unsetting an element, and enumerating keys.

use super::super::{ResolvedVar, VarTraceCell, Vm, err, variable_cleanup};
use crate::value::Value;
use crate::vars::{VarState as Local, VarTable};
use std::collections::HashSet;
use tcl_runtime_api::{Completion, VarId};

impl Vm {
    /// Ensure `name` is an array (creating an empty one if unset) — `array set
    /// name {}` with an empty value list still materialises the array (C's
    /// `TclArraySet`). A scalar `name` errors `variable isn't array`; this is the
    /// empty-list path, which C words under the command (`can't array set "n"`),
    /// unlike the per-element write that names `n(key)`.
    pub(crate) fn ensure_array(&mut self, name: &str) -> Result<(), Completion<Value>> {
        self.ensure_array_bytes(name.as_bytes())
    }

    pub(crate) fn ensure_array_bytes(&mut self, name: &[u8]) -> Result<(), Completion<Value>> {
        self.confine_store_bytes(name, self.current_level(), false)?;
        if self.observed_names.is_some() {
            return self
                .observed_ensure_array(name, self.current_level())
                .map_err(|_| {
                    self.refuse_host_command("observed array root storage is unavailable".into())
                });
        }
        let input = tcl_syntax::naming::NativeVariableInputForm::Combined(name);
        let resolved = self
            .resolve_var_from_bytes(name, self.current_level())
            .ok_or_else(|| {
                self.variable_access_error_input(
                    "array set",
                    input,
                    "parent namespace doesn't exist",
                )
            })?;
        self.ensure_array_selected(name, &resolved)
    }

    pub(in super::super) fn ensure_array_selected(
        &mut self,
        name: &[u8],
        resolved: &ResolvedVar,
    ) -> Result<(), Completion<Value>> {
        let input = tcl_syntax::naming::NativeVariableInputForm::Combined(name);
        // An element-form target fails its lookup as an array (`TclLookupArray`
        // reports with `set`): tclsh 8.4-9.1 `array set z(k) {}` and
        // `array set z(k) {a 1}` both say `can't set "z(k)": variable isn't array`.
        if resolved.elem.is_some() {
            return Err(self.variable_access_error_input("set", input, "variable isn't array"));
        }
        if self.dictionary_variable_containers() {
            if let Some(value) = self.read_variable_contents(resolved) {
                self.native_variable_dict_pairs(&value).map_err(|error| {
                    crate::command::completion_from_cmd_error(self, error.into())
                })?;
            } else {
                self.store_dictionary_root(resolved, Value::dict(Vec::new()))?;
            }
            return Ok(());
        }
        let id = resolved
            .id
            .or_else(|| self.ensure_target_var_at_binding(&resolved.binding, None))
            .ok_or_else(|| {
                self.variable_access_error_input("array set", input, "variable isn't array")
            })?;
        match self.var_arena.get(id).map(crate::vars::VarCell::state) {
            Some(Local::Array(_)) => {}
            Some(Local::Undefined) => {
                let _ = self
                    .var_arena
                    .replace_state(id, Local::Array(VarTable::new()));
            }
            Some(Local::Scalar(_)) => {
                return Err(self.variable_access_error_input(
                    "array set",
                    input,
                    "variable isn't array",
                ));
            }
            Some(Local::Link(_) | Local::NameLink(_)) | None => {
                let _ = self.bind_new_var(&resolved.binding, Local::Array(VarTable::new()));
            }
        }
        self.publish_authored_static_array(id)?;
        Ok(())
    }

    // Arrays (link-aware via `locate`)

    pub(crate) fn get_array_elem(&self, name: &str, key: &str) -> Option<Value> {
        self.get_array_elem_from(self.current_level(), name, key)
    }

    pub(in super::super) fn get_array_elem_from(
        &self,
        start: usize,
        name: &str,
        key: &str,
    ) -> Option<Value> {
        self.get_array_elem_bytes_from(start, name.as_bytes(), key.as_bytes())
    }

    pub(in super::super) fn get_array_elem_bytes_from(
        &self,
        start: usize,
        name: &[u8],
        key: &[u8],
    ) -> Option<Value> {
        self.read_variable_contents(&self.resolve_var_parts_from_bytes(name, Some(key), start)?)
    }

    pub(in super::super) fn set_array_elem_bytes_from(
        &mut self,
        start: usize,
        root: &[u8],
        element: &[u8],
        value: Value,
        traced: bool,
    ) -> Result<(), Completion<Value>> {
        use tcl_syntax::naming::NativeVariableInputForm;
        let input = NativeVariableInputForm::Separate {
            root,
            element: Some(element),
        };
        if let Some(refused) = self.refused_completion() {
            return Err(refused);
        }
        self.confine_store_parts_bytes(root, Some(element), start, false)?;
        let resolved = self
            .resolve_var_parts_from_bytes(root, Some(element), start)
            .ok_or_else(|| {
                self.variable_access_error_input("set", input, "parent namespace doesn't exist")
            })?;
        if self.dictionary_variable_containers() {
            let base = self
                .resolve_binding_var(resolved.binding.clone(), None, &mut HashSet::new())
                .ok_or_else(|| {
                    self.variable_access_error_input("set", input, "no such variable")
                })?;
            self.write_dictionary_member(
                &base,
                resolved.elem.as_deref().unwrap_or(element),
                value,
            )?;
            return Ok(());
        }
        let id = self
            .ensure_target_var_at_binding(&resolved.binding, resolved.elem.as_deref())
            .ok_or_else(|| {
                self.variable_access_error_input("set", input, "variable isn't array")
            })?;
        if self.const_vars.contains(&id) {
            return Err(self.variable_access_error_at(
                "set",
                input,
                "variable is a constant",
                tcl_syntax::naming::NativeVariableFailureSite::ValueWrite,
            ));
        }
        self.drop_array_descendant_traces(id);
        let _ = self.var_arena.replace_state(id, Local::Scalar(value));
        if traced && (self.variable_observers_active() || self.authored_tmm_static.policy.is_some())
        {
            let cell = self
                .resolve_var_parts_from_bytes(root, Some(element), start)
                .and_then(|resolved| self.trace_cell_from_resolved(&resolved));
            self.variable_trace_guest_or_unwind(|vm| {
                vm.fire_elem_traces_from_cell_bytes(root, element, "write", cell)
            })?;
        }
        Ok(())
    }

    /// Write an array element with no trace firing.
    pub(crate) fn write_array_raw(
        &mut self,
        name: &str,
        key: &str,
        value: Value,
    ) -> Result<(), Completion<Value>> {
        if let Some(refused) = self.refused_completion() {
            return Err(refused);
        }
        self.write_array_raw_from(self.current_level(), name, key, value)
    }

    pub(in super::super) fn write_array_raw_from(
        &mut self,
        start: usize,
        name: &str,
        key: &str,
        value: Value,
    ) -> Result<(), Completion<Value>> {
        self.confine_store(name, start)?;
        self.validate_var_parent(name)?;
        if self.dictionary_variable_containers() {
            let resolved = self
                .resolve_var_from(name, start)
                .ok_or_else(|| err(format!("can't set \"{name}({key})\": no such variable")))?;
            return self
                .write_dictionary_member(&resolved, key, value)
                .map_err(|_| err(format!("can't set \"{name}({key})\": variable isn't array")));
        }
        let Some((_, base_id)) = self.ensure_base_var_from(name, start) else {
            return Err(err(format!(
                "can't set \"{name}({key})\": no such variable"
            )));
        };
        match self.var_arena.get(base_id).map(crate::vars::VarCell::state) {
            Some(Local::Scalar(_)) => {
                return Err(err(format!(
                    "can't set \"{name}({key})\": variable isn't array"
                )));
            }
            Some(Local::Undefined) => {
                let _ = self
                    .var_arena
                    .replace_state(base_id, Local::Array(VarTable::new()));
            }
            Some(Local::Array(_)) => {}
            Some(Local::Link(_) | Local::NameLink(_)) | None => return Ok(()),
        }
        let existing = match self.var_arena.get(base_id).map(crate::vars::VarCell::state) {
            Some(Local::Array(elements)) => elements.get(key).copied(),
            _ => None,
        };
        if let Some(id) = existing.and_then(|id| self.var_arena.resolve(id)) {
            let _ = self.var_arena.replace_state(id, Local::Scalar(value));
            self.publish_authored_static_write(id)?;
            return Ok(());
        }
        let Some(id) = self
            .var_arena
            .alloc_element(Local::Scalar(value), base_id, key.to_owned())
        else {
            return Err(err("too many variable cells"));
        };
        if self.var_arena.array_insert(base_id, key.to_owned(), id) {
            self.var_arena.bind(id);
            self.publish_authored_static_write(id)?;
        } else {
            self.var_arena.discard_unbound(id);
        }
        Ok(())
    }

    /// Write an array element, firing `write` traces afterwards. Like
    /// [`Self::set_var`], a callback error fails the command without
    /// un-storing the element.
    pub(crate) fn set_array_elem(
        &mut self,
        name: &str,
        key: &str,
        value: Value,
    ) -> Result<(), Completion<Value>> {
        self.set_array_elem_bytes_from(
            self.current_level(),
            name.as_bytes(),
            key.as_bytes(),
            value,
            true,
        )
    }

    #[cfg(test)]
    pub(crate) fn set_array_elem_bytes(
        &mut self,
        root: &[u8],
        element: &[u8],
        value: Value,
    ) -> Result<(), Completion<Value>> {
        self.set_array_elem_bytes_from(self.current_level(), root, element, value, true)
    }

    /// Remove array element `name(key)` through a **two-part** access — C's
    /// `TclObjUnsetVar2(part1, part2, …)`, which `array unset` and the
    /// `INST_UNSET_ARRAY*` opcodes use. The traces report `name1 = name`.
    pub(crate) fn array_unset_elem(&mut self, name: &str, key: &str) -> bool {
        self.array_unset_elem_reporting(name, key, None)
    }

    fn array_unset_elem_reporting(
        &mut self,
        name: &str,
        key: &str,
        reported: Option<&str>,
    ) -> bool {
        if self.dictionary_variable_containers() {
            let Some(resolved) = self.resolve_var_from(name, self.current_level()) else {
                return false;
            };
            return self.remove_dictionary_member(&resolved, key);
        }
        let Some(base_id) = self
            .resolve_var_from(name, self.current_level())
            .and_then(|resolved| resolved.base_id)
        else {
            return false;
        };
        self.array_unset_elem_reporting_at(base_id, name, key, reported)
    }

    pub(in super::super) fn array_unset_elem_reporting_at(
        &mut self,
        base_id: VarId,
        name: &str,
        key: &str,
        reported: Option<&str>,
    ) -> bool {
        self.array_unset_elem_reporting_bytes_at(
            base_id,
            name.as_bytes(),
            key.as_bytes(),
            reported.map(str::as_bytes),
        )
    }

    pub(in super::super) fn array_unset_elem_reporting_bytes_at(
        &mut self,
        base_id: VarId,
        name: &[u8],
        key: &[u8],
        reported: Option<&[u8]>,
    ) -> bool {
        let original = variable_cleanup::OriginalVariableCleanup::enter(self);
        if original.check(self).is_err() {
            return false;
        }
        if self.dictionary_variable_containers() {
            return self.remove_dictionary_element(base_id, key);
        }
        // The selected element becomes undefined and its own traces move out
        // before callbacks. Its original hash slot stays while the operation
        // holds the cell. The containing array's traces remain live, including
        // when this member has no callback registration of its own.
        let raw = match self.var_arena.get(base_id).map(crate::vars::VarCell::state) {
            Some(Local::Array(elements)) => elements.get(key).copied(),
            _ => None,
        };
        let id = raw.and_then(|raw| self.var_arena.resolve(raw));
        let mut spelling = name.to_vec();
        spelling.push(b'(');
        spelling.extend_from_slice(key);
        spelling.push(b')');
        let Some((raw, id)) = raw.zip(id) else {
            return false;
        };
        let cell = VarTraceCell {
            id: Some(id),
            array: Some(base_id),
            elem: Some(tcl_core_types::NameBytes::from(key)),
        };
        let traced = self.variable_observers_active();
        let publication = self.authored_static_unset_address(id);
        // Retain the actual member and parent before destruction. A parent
        // unset trace needs that member even when it has no trace pin of its
        // own. TclPtrUnsetVarIdx retains the reached Var until CleanupVar
        // after callbacks (C8.6 tclVar.c:2460–2514).
        let existed = self.with_variable_operation(&cell, |vm| {
            let Some((existed, old)) = vm.unbind_array_element(base_id, key, raw, id, &original)
            else {
                return false;
            };
            let (old, array_default) = old.into_parts();
            let terminal_old = if original.c_protocol().is_some() {
                Some(old)
            } else {
                let _ = original.retire(vm, crate::vars::VariableRetirement::state(old));
                None
            };
            if original.check(vm).is_ok() {
                let _ = vm.publish_authored_static_unset(publication);
            }
            if traced {
                let _ =
                    original.fire_taken(vm, &spelling, reported.or(Some(name)), cell.clone(), true);
            }
            if let Some(old) = terminal_old {
                let _ = original.retire(vm, crate::vars::VariableRetirement::state(old));
            }
            let _ = original.in_original(vm, |_| drop(array_default));
            existed
        });
        // A refill, alias or fresh trace keeps this selected member. Cleanup
        // removes only its unchanged undefined table slot after the hold ends.
        if original.check(self).is_ok() {
            self.discard_undefined_array_shell(base_id, key, id);
        }
        existed
    }

    fn unbind_array_element(
        &mut self,
        base_id: VarId,
        key: impl AsRef<[u8]>,
        raw: VarId,
        id: VarId,
        original: &variable_cleanup::OriginalVariableCleanup,
    ) -> Option<(bool, variable_cleanup::DetachedVariableState)> {
        original.detach_member(self, base_id, key.as_ref(), raw, id)
    }

    fn resolve_array_elem_from(&self, start: usize, name: &str, key: &str) -> Option<ResolvedVar> {
        let base = self.resolve_var_from(name, start)?;
        let base_id = base.base_id?;
        let raw = match self.var_arena.get(base_id).map(crate::vars::VarCell::state) {
            Some(Local::Array(elements)) => elements.get(key).copied(),
            _ => None,
        };
        let raw = raw?;
        Some(ResolvedVar {
            binding: base.binding,
            base_id: Some(base_id),
            id: self.var_arena.resolve(raw),
            elem: Some(tcl_core_types::NameBytes::from(key)),
        })
    }

    pub(in super::super) fn unset_array_elem_from(
        &mut self,
        start: usize,
        name: &str,
        key: &str,
    ) -> bool {
        if self.dictionary_variable_containers() {
            let Some(resolved) = self.resolve_var_from(name, start) else {
                return false;
            };
            return self.remove_dictionary_member(&resolved, key);
        }
        let Some(resolved) = self.resolve_array_elem_from(start, name, key) else {
            return false;
        };
        self.unbind_storage_resolved(&resolved)
    }

    /// Frame-addressed array-key enumeration. The explicit frame selects both
    /// proc-local and namespace-token ownership before the stable array cell is
    /// read, so equal spellings in two frames never collapse.
    pub(in super::super) fn array_keys_from(
        &self,
        start: usize,
        name: &str,
    ) -> Option<Vec<String>> {
        self.array_key_bytes_from(start, name.as_bytes())
            .map(Self::checked_variable_names)
    }

    fn checked_variable_names(names: Vec<Vec<u8>>) -> Vec<String> {
        names
            .into_iter()
            .map(|name| {
                String::from_utf8(name)
                    .expect("byte-valued variable names require byte enumeration")
            })
            .collect()
    }

    pub(in super::super) fn array_key_bytes_from(
        &self,
        start: usize,
        name: &[u8],
    ) -> Option<Vec<Vec<u8>>> {
        let resolved = self.resolve_var_from_bytes(name, start)?;
        if resolved.elem.is_some() {
            return None;
        }
        self.array_key_bytes_at_id(resolved.id?)
    }

    pub(in super::super) fn array_key_bytes_at_id(&self, id: VarId) -> Option<Vec<Vec<u8>>> {
        if self.dictionary_variable_containers() {
            return self.dictionary_pairs_at(id).ok()?.map(|pairs| {
                pairs
                    .into_iter()
                    .map(|(key, _)| key.string_bytes().to_vec())
                    .collect()
            });
        }
        let Local::Array(elements) = self.var_arena.get(id)?.state() else {
            return None;
        };
        Some(
            elements
                .iter()
                .filter_map(|(key, raw)| {
                    self.var_arena
                        .resolve(*raw)
                        .and_then(|id| self.read_resolved_cell(id))
                        .map(|_| key.as_bytes().to_vec())
                })
                .collect(),
        )
    }

    pub(in super::super) fn array_keys_at_id(&self, id: VarId) -> Option<Vec<String>> {
        self.array_key_bytes_at_id(id)
            .map(Self::checked_variable_names)
    }

    pub(in super::super) fn array_search_key_bytes_at_id(&self, id: VarId) -> Option<Vec<Vec<u8>>> {
        if self.dictionary_variable_containers() {
            return self.array_key_bytes_at_id(id);
        }
        let Local::Array(elements) = self.var_arena.get(id)?.state() else {
            return None;
        };
        Some(
            elements
                .keys()
                .map(|name| name.as_bytes().to_vec())
                .collect(),
        )
    }

    pub(in super::super) fn array_search_keys_at_id(&self, id: VarId) -> Option<Vec<String>> {
        self.array_search_key_bytes_at_id(id)
            .map(Self::checked_variable_names)
    }

    pub(in super::super) fn array_elem_exists_at_id(&self, id: VarId, key: &str) -> bool {
        if self.dictionary_variable_containers() {
            return self.read_resolved_elem(id, key).is_some();
        }
        let Some(Local::Array(elements)) = self.var_arena.get(id).map(crate::vars::VarCell::state)
        else {
            return false;
        };
        elements
            .get(key)
            .and_then(|raw| self.var_arena.resolve(*raw))
            .and_then(|element| self.read_resolved_cell(element))
            .is_some()
    }
}
