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

//! Variables: resolving a name to its cell, links, traced reads, scalar and array access, unset, the read-modify-write updates (`append`, `lappend`, `incr`) and procedure statics.

use super::Vm;
use crate::value::Value;
use crate::vars::VarState as Local;
use tcl_runtime_api::{Completion, VarId};

mod arrays;
mod captured_updates;
mod increment;
mod links;
mod procedure_statics;
mod resolution;
mod scalar_access;
mod traced_reads;
mod unset;

pub(crate) use links::UpvarLinkError;

/// Split an `arr(key)` variable reference into `(base, key)`, or `None` for a
/// plain scalar/array name — `TclObjLookupVarEx`'s rule, from the shared
/// naming owner. Both halves may be empty: `(x)` is element `x` of the array
/// named `""`.
fn elem_ref(name: &str) -> Option<(&str, &str)> {
    tcl_syntax::naming::split_element_ref(name)
}

impl Vm {
    // frame-addressed storage (the `VarStore` `FrameId`-honouring path)
    //
    // These resolve `name` starting from an *explicit* frame (following links),
    // touching only storage: no current-eval-context namespace fallback and no
    // trace firing (both are current-frame concerns). The `VarStore` impl uses
    // them only for a non-current `FrameId`; a `FrameId` equal to the current
    // frame delegates to the full inherent accessors above, so the common case
    // keeps its exact behaviour (fallback + traces).

    /// Frame-addressed scalar read (the storage half of [`get_var`](Self::get_var)).
    pub(crate) fn get_var_from(&self, start: usize, name: &str) -> Option<Value> {
        self.get_var_from_bytes(start, name.as_bytes())
    }

    pub(crate) fn get_var_from_bytes(&self, start: usize, name: &[u8]) -> Option<Value> {
        if self.observed_name_policy_selected() {
            return self.observed_variable_get(name, start).ok().flatten();
        }
        self.read_variable_contents(&self.resolve_var_from_bytes(name, start)?)
    }

    /// Frame-addressed scalar write (the storage half of
    /// [`write_scalar_raw`](Self::write_scalar_raw)).
    pub(crate) fn write_scalar_from(&mut self, start: usize, name: &str, value: Value) {
        self.write_scalar_bytes_from(start, name.as_bytes(), value);
    }

    pub(super) fn write_scalar_bytes_from(&mut self, start: usize, name: &[u8], value: Value) {
        // This legacy storage-only facade has no guest completion channel.
        let _ = self.write_scalar_bytes_from_checked(start, name, value);
    }

    pub(super) fn write_scalar_bytes_from_checked(
        &mut self,
        start: usize,
        name: &[u8],
        value: Value,
    ) -> Result<(), Completion<Value>> {
        if let Some(refusal) = self.refused_completion() {
            return Err(refusal);
        }
        let Some(resolved) = self.resolve_var_from_bytes(name, start) else {
            return Ok(());
        };
        if self.dictionary_variable_containers()
            && let Some(key) = &resolved.elem
        {
            return self.write_dictionary_element(&resolved.binding, key, value);
        }
        let written = if let Some(id) = resolved.id {
            self.drop_array_descendant_traces(id);
            let _ = self.var_arena.replace_state(id, Local::Scalar(value));
            Some(id)
        } else if resolved.elem.is_none() {
            self.bind_new_var(&resolved.binding, Local::Scalar(value))
        } else {
            None
        };
        if let Some(id) = written {
            self.publish_authored_static_write(id)?;
        }
        Ok(())
    }

    /// Frame-addressed scalar existence (the storage half of
    /// [`var_exists`](Self::var_exists)).
    pub(crate) fn exists_from(&self, start: usize, name: &str) -> bool {
        self.resolve_var_from(name, start).is_some_and(|resolved| {
            self.read_variable_contents(&resolved).is_some()
                || resolved.elem.is_none()
                    && resolved
                        .id
                        .and_then(|id| self.var_arena.get(id))
                        .is_some_and(|cell| matches!(cell.state(), Local::Array(_)))
        })
    }

    /// C's three-way read-miss message (`tclVar.c`): a scalar read of an array
    /// (`variable is array`), a missing element of an existing array (`no such
    /// element in array`), an existing scalar accessed with an index (`variable
    /// isn't array`), or a wholly missing variable (`no such variable`).
    pub(crate) fn read_miss_msg(&self, name: &str) -> String {
        self.read_miss_msg_from(self.current_level(), name)
    }

    /// Render the same variable read failure in an explicitly selected activation.
    pub(crate) fn read_miss_msg_from(&self, level: usize, name: &str) -> String {
        let (base, has_idx) = elem_ref(name).map_or((name, false), |(b, _)| (b, true));
        let state = self
            .resolve_var_from(base, level)
            .and_then(|resolved| resolved.id)
            .and_then(|id| self.var_arena.get(id))
            .map(crate::vars::VarCell::state);
        let what = if matches!(state, Some(Local::Array(_))) {
            if has_idx {
                "no such element in array"
            } else {
                "variable is array"
            }
        } else if has_idx && matches!(state, Some(Local::Scalar(_))) {
            "variable isn't array"
        } else {
            "no such variable"
        };
        format!("can't read \"{name}\": {what}")
    }

    /// Whether `name` exists, resolving an `arr(key)` element reference to the
    /// element — the `info exists` / `existStk` semantic.
    pub(crate) fn exists_var(&self, name: &str) -> bool {
        self.exists_var_bytes_from(name.as_bytes(), self.current_level())
    }

    /// Read a combined native variable operand through the common resolver.
    pub(crate) fn var_get(&self, name: &str) -> Option<Value> {
        self.get_var_bytes(name.as_bytes())
    }

    /// Write a combined native variable operand through the common resolver.
    pub(crate) fn var_set(&mut self, name: &str, value: Value) -> Result<(), Completion<Value>> {
        self.set_var_bytes(name.as_bytes(), value)
    }

    /// Store `value` in `name` and return the value the variable **reads back**
    /// afterwards, which is what `set`/`append`/`lappend`/`incr` evaluate to.
    /// A write trace that rewrites the variable therefore changes the command's
    /// result, and one that unsets it — or turns it into an array — makes the
    /// result empty: C returns `varPtr->value.objPtr` only while the variable
    /// is still a defined scalar and the interp's empty object otherwise
    /// (`TclPtrSetVarIdx`, `tclVar.c` 9.0.4:2050-2065; 8.6.16:2006-2012).
    /// Named after the runtime's `Interp::store_var_result` so both engines'
    /// read-back owners are greppable together.
    pub(crate) fn store_var_result(
        &mut self,
        name: &str,
        value: Value,
    ) -> Result<Value, Completion<Value>> {
        self.store_var_result_bytes(name.as_bytes(), value)
    }

    pub(crate) fn store_var_result_bytes(
        &mut self,
        name: &[u8],
        value: Value,
    ) -> Result<Value, Completion<Value>> {
        self.confine_store_bytes(name, self.current_level(), false)?;
        if !self.variable_observers_active() || self.dictionary_variable_containers() {
            self.set_var_bytes(name, value.clone())?;
            return Ok(self.get_var_bytes(name).unwrap_or_else(Value::empty));
        }
        let selected = self.ensure_target_var_from_bytes(name, self.current_level());
        let cell = self
            .resolve_var_from_bytes(name, self.current_level())
            .and_then(|resolved| self.trace_cell_from_resolved(&resolved));
        if let Some(cell) = cell {
            return self.with_variable_operation(&cell, |vm| {
                vm.set_var_bytes(name, value)?;
                Ok(selected
                    .and_then(|id| vm.read_resolved_cell(id))
                    .unwrap_or_else(Value::empty))
            });
        }
        self.set_var_bytes(name, value)?;
        Ok(selected
            .and_then(|id| self.read_resolved_cell(id))
            .unwrap_or_else(Value::empty))
    }

    pub(crate) fn read_miss_completion_bytes_from(
        &mut self,
        level: usize,
        name: &[u8],
    ) -> Completion<Value> {
        let reason =
            self.resolve_var_from_bytes(name, level)
                .map_or("no such variable", |resolved| {
                    if resolved.elem.is_some() {
                        match resolved
                            .base_id
                            .and_then(|id| self.var_arena.get(id))
                            .map(crate::vars::VarCell::state)
                        {
                            Some(Local::Array(_)) => "no such element in array",
                            Some(Local::Scalar(_)) => "variable isn't array",
                            _ => "no such variable",
                        }
                    } else if resolved
                        .id
                        .and_then(|id| self.var_arena.get(id))
                        .is_some_and(|cell| matches!(cell.state(), Local::Array(_)))
                    {
                        "variable is array"
                    } else {
                        "no such variable"
                    }
                });
        self.variable_access_error_bytes("read", name, reason)
    }

    /// Read the scalar at an already-resolved cell —
    /// the tail of [`get_var`](Self::get_var) with the name lookup already done.
    pub(super) fn read_resolved_cell(&self, id: VarId) -> Option<Value> {
        match self.var_arena.get(id)?.state() {
            Local::Scalar(value) => Some(value.clone()),
            _ => None,
        }
    }

    /// Borrow the initial native read without manufacturing an object owner.
    /// Post-write command results use `read_resolved_cell` instead.
    pub(super) fn initial_value_at_cell(
        &self,
        id: VarId,
        explicit_array: Option<VarId>,
    ) -> Option<&Value> {
        let cell = self.var_arena.get(id)?;
        if let Local::Scalar(value) = cell.state() {
            return Some(value);
        }
        if !matches!(cell.state(), Local::Undefined)
            || !self
                .actual_native_string_protocol()
                .is_some_and(|protocol| {
                    protocol
                        .tcl_version()
                        .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_0)
                })
        {
            return None;
        }
        if let Some(array) = explicit_array {
            return self.var_arena.array_default(array);
        }
        let (array, _) = self.var_arena.element_parent(id)?;
        // C clears a deleted array's default before freeing its table, so a
        // link to one of its elements reads an undefined cell with no default.
        if !self.var_arena.element_is_attached(id) {
            return None;
        }
        self.var_arena.array_default(array)
    }

    /// [`read_resolved_cell`](Self::read_resolved_cell) for element `key` of an
    /// already-resolved array cell — the tail of
    /// [`get_array_elem`](Self::get_array_elem).
    pub(super) fn read_resolved_elem(
        &self,
        base_id: VarId,
        key: impl AsRef<[u8]>,
    ) -> Option<Value> {
        if self.dictionary_variable_containers() {
            return self
                .dictionary_pairs_at(base_id)
                .ok()??
                .into_iter()
                .find(|(name, _)| name.string_bytes().as_ref() == key.as_ref())
                .map(|(_, value)| value);
        }
        let Local::Array(elements) = self.var_arena.get(base_id)?.state() else {
            return None;
        };
        let id = self.var_arena.resolve(*elements.get(key.as_ref())?)?;
        self.read_resolved_cell(id)
    }

    pub(crate) fn store_elem_result_bytes(
        &mut self,
        root: &[u8],
        element: &[u8],
        value: Value,
    ) -> Result<Value, Completion<Value>> {
        self.confine_store_parts_bytes(root, Some(element), self.current_level(), false)?;
        if !self.variable_observers_active() || self.dictionary_variable_containers() {
            self.set_array_elem_bytes_from(self.current_level(), root, element, value, true)?;
            return Ok(self
                .get_array_elem_bytes_from(self.current_level(), root, element)
                .unwrap_or_else(Value::empty));
        }
        let resolved = self.resolve_var_parts_from_bytes(root, Some(element), self.current_level());
        let selected = resolved.as_ref().and_then(|resolved| {
            self.ensure_target_var_at_binding(&resolved.binding, resolved.elem.as_deref())
        });
        let cell = self
            .resolve_var_parts_from_bytes(root, Some(element), self.current_level())
            .and_then(|resolved| self.trace_cell_from_resolved(&resolved));
        if let Some(cell) = cell {
            return self.with_variable_operation(&cell, |vm| {
                vm.set_array_elem_bytes_from(vm.current_level(), root, element, value, true)?;
                Ok(selected
                    .and_then(|id| vm.read_resolved_cell(id))
                    .unwrap_or_else(Value::empty))
            });
        }
        self.set_array_elem_bytes_from(self.current_level(), root, element, value, true)?;
        Ok(selected
            .and_then(|id| self.read_resolved_cell(id))
            .unwrap_or_else(Value::empty))
    }
}
