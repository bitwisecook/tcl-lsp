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

//! Unsetting a variable: retiring its storage, array elements and traces, the unset traces it reports, and the storage-only forms.

use super::super::{
    ResolvedVar, VarTableOwner, VarTraceCell, Vm, authored_tmm_static, variable_cleanup,
};
use crate::value::Value;
use crate::vars::{VarState as Local, VarTable};
use tcl_runtime_api::{Completion, VarId};

impl Vm {
    /// Remove the binding reached by a resolved access while preserving a
    /// target cell that still has `upvar` links. Linked cells become undefined
    /// in place so every alias continues to share the same identity.
    fn unbind_resolved(&mut self, resolved: &ResolvedVar) -> bool {
        if let Some(result) = self.detach_static_reference(resolved) {
            return result;
        }
        let Some(id) = resolved.id else {
            return false;
        };
        let Some(state) = self.var_arena.get(id).map(crate::vars::VarCell::state) else {
            return false;
        };
        let existed = matches!(state, Local::Scalar(_) | Local::Array(_));
        self.drop_array_descendant_traces(id);
        if matches!(
            self.var_arena.get(id).map(crate::vars::VarCell::state),
            Some(Local::Array(_))
        ) {
            // This storage-only path runs no child unset callbacks, so release
            // the detached table and its bindings immediately.
            let _ = self.var_arena.replace_state(id, Local::Undefined);
        } else {
            let _ = self.var_arena.unset_state(id);
        }

        let raw = if let Some(key) = &resolved.elem {
            let Some(base_id) = resolved.base_id else {
                return existed;
            };
            match self.var_arena.get(base_id).map(crate::vars::VarCell::state) {
                Some(Local::Array(elements)) => elements.get(key).copied(),
                _ => None,
            }
        } else {
            self.raw_variable_binding(&resolved.binding)
        };
        let Some(raw) = raw else {
            return existed;
        };
        if matches!(resolved.binding.owner, VarTableOwner::CompiledLocal { .. })
            || matches!(
                self.var_arena.get(raw).map(crate::vars::VarCell::state),
                Some(Local::Link(_) | Local::NameLink(_))
            )
            || self.var_arena.has_link_refs(id)
            || self.var_arena.has_operation_refs(id)
        {
            return existed;
        }
        if let Some(key) = &resolved.elem {
            if let Some(base_id) = resolved.base_id {
                let _ = self.var_arena.array_remove(base_id, key);
            }
        } else if let Some(table) = self.var_table_mut(resolved.binding.owner) {
            table.remove(&resolved.binding.name);
        }
        self.const_vars.remove(&id);
        self.var_arena.unbind(raw);
        existed
    }

    /// Remove a scalar; returns whether it existed.
    ///
    /// C's `UnsetVarStruct` (`tclVar.c` 9.0.4:2560-2745) marks the variable
    /// undefined and moves its trace list off the cell **before** the unset
    /// callbacks run, then frees whatever list is left. So a callback sees the
    /// variable already gone, a value it stores survives the unset, and the
    /// revived variable carries no traces — not even a write trace that would
    /// otherwise have fired on that store. The trace key is resolved *before*
    /// the removal: resolution can depend on the cell still existing.
    pub fn unset_var(&mut self, name: &str) -> bool {
        self.unset_var_bytes(name.as_bytes())
    }

    pub fn unset_var_bytes(&mut self, name: &[u8]) -> bool {
        if self.observed_names.is_some() {
            return self
                .observed_variable_unset(name, self.current_level())
                .unwrap_or(false);
        }
        if self.execution_refusal.is_some() {
            return false;
        }
        let Some(resolved) = self.resolve_var_from_bytes(name, self.current_level()) else {
            return false;
        };
        self.unset_resolved(name, &resolved)
    }

    pub(in super::super) fn unset_resolved(
        &mut self,
        name: impl AsRef<[u8]>,
        resolved: &ResolvedVar,
    ) -> bool {
        self.unset_resolved_checked(name, resolved).unwrap_or(false)
    }

    fn retire_unset_array_elements(
        &mut self,
        name: &[u8],
        parent: Option<VarId>,
        elements: VarTable,
        preserve_definition: bool,
        table_holds_vars: bool,
        original: &variable_cleanup::OriginalVariableCleanup,
    ) -> Vec<VarId> {
        original.retire_elements(
            self,
            name,
            parent,
            elements,
            Some(name),
            variable_cleanup::ElementRetirementPolicy {
                preserve: preserve_definition,
                table_holds: table_holds_vars,
                dying: false,
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn retire_unset_storage(
        &mut self,
        name: &[u8],
        reported_name1: Option<&[u8]>,
        resolved: &ResolvedVar,
        old_state: variable_cleanup::DetachedVariableState,
        traced_cell: Option<VarTraceCell>,
        publication: Option<authored_tmm_static::StaticAddress>,
        original: &variable_cleanup::OriginalVariableCleanup,
    ) -> (Option<Local>, Result<(), Completion<Value>>) {
        let (old_state, array_default) = old_state.into_parts();
        let key_drain = if let Local::Array(elements) = &old_state {
            if let Ok(state) = original.state_mut(self) {
                state
                    .var_arena
                    .begin_element_destruction(elements.values().copied());
            }
            Some(elements.destruction_key_drain())
        } else {
            None
        };
        let table_holds_vars = original
            .c_protocol()
            .is_some_and(tcl_syntax::native_variable_name::NativeVariableNameProtocol::element_table_retains_original);
        let preserve_definition = original
            .c_protocol()
            .is_some_and(tcl_syntax::native_variable_name::NativeVariableNameProtocol::element_unset_preserves_definition_during_trace);
        let publication_result = if original.check(self).is_ok() {
            self.publish_authored_static_unset(publication)
        } else {
            Ok(())
        };
        let mut retired_bindings = Vec::new();
        if let Some(cell) = traced_cell {
            let _ = original.fire_taken(self, name, reported_name1, cell, true);
        }

        // An array's old element table is detached from the parent before its
        // callback, then visited one cell at a time. Later old elements remain
        // live while an earlier callback runs, and callback-created elements
        // belong to the revived array rather than this destruction pass.
        let terminal_old = match old_state {
            Local::Array(elements) => {
                retired_bindings = self.retire_unset_array_elements(
                    name,
                    resolved.id,
                    elements,
                    preserve_definition,
                    table_holds_vars,
                    original,
                );
                None
            }
            state if original.c_protocol().is_some() => {
                let _ = original.retire(self, crate::vars::VariableRetirement::state(state));
                None
            }
            state => Some(state),
        };
        for id in retired_bindings {
            let _ = original.retire(self, crate::vars::VariableRetirement::unbind(id));
        }
        let _ = original.in_original(self, |_| drop(array_default));
        let _ = original.in_original(self, |_| drop(key_drain));
        (terminal_old, publication_result)
    }

    fn refresh_unset_jim_lookup_identity(
        &mut self,
        resolved: &ResolvedVar,
        existed: bool,
        original: &variable_cleanup::OriginalVariableCleanup,
    ) {
        if original.check(self).is_ok()
            && existed
            && resolved.elem.is_none()
            && self.uses_native_jim_lookup()
        {
            let level = match resolved.binding.owner {
                VarTableOwner::Frame(level)
                | VarTableOwner::ProcedureStatics(level)
                | VarTableOwner::CompiledLocal { level, .. } => level,
                VarTableOwner::Namespace(_) => 0,
            };
            if let Some(frame) = self.frames.get(level) {
                frame
                    .jim_lookup_id
                    .set(crate::frame::ActivationIdentity::new().serial);
            }
        }
    }

    pub(in super::super) fn unset_resolved_checked(
        &mut self,
        name: impl AsRef<[u8]>,
        resolved: &ResolvedVar,
    ) -> Result<bool, Completion<Value>> {
        self.unset_resolved_reporting(name.as_ref(), None, resolved)
    }

    /// [`Self::unset_resolved_checked`], reporting `reported_name1` to unset
    /// traces where the caller held the name as two parts (C's `part1`).
    fn unset_resolved_reporting(
        &mut self,
        name: &[u8],
        reported_name1: Option<&[u8]>,
        resolved: &ResolvedVar,
    ) -> Result<bool, Completion<Value>> {
        let original = variable_cleanup::OriginalVariableCleanup::enter(self);
        original
            .check(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        if self.dictionary_variable_containers()
            && let Some(key) = &resolved.elem
        {
            let removed = resolved
                .base_id
                .is_some_and(|id| self.remove_dictionary_element(id, key));
            original
                .check(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            return Ok(removed);
        }
        if let Some(result) = self.detach_static_reference(resolved) {
            original
                .check(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            return Ok(result);
        }
        let native_error_unset = resolved
            .elem
            .is_none()
            .then(|| {
                resolved
                    .id
                    .and_then(|id| self.take_native_error_variable_trace(id))
            })
            .flatten();
        let native_precision_unset = resolved.elem.is_none()
            && resolved.id.is_some()
            && resolved.id == self.variable_observers.precision_cell;
        if native_precision_unset {
            self.variable_observers.precision_cell = None;
        }
        let publication = resolved
            .id
            .and_then(|id| self.authored_static_unset_address(id));
        let traced_cell = self.trace_cell_from_resolved(resolved);
        // Old element aliases retain the array receiver during its destruction
        // callbacks, even after its name-table binding and trace pins disappear.
        let receiver = resolved.id;
        if let Some(id) = receiver {
            self.var_arena.pin(id);
        }
        let Some((existed, old_state)) = self.detach_variable_state(resolved, &original) else {
            if let Some(id) = receiver {
                let _ = original.retire(self, crate::vars::VariableRetirement::unpin(id));
            }
            if original.check(self).is_ok()
                && let Some(variable) = native_error_unset
            {
                self.install_native_error_variable_trace(variable);
            }
            original
                .check(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            return Ok(false);
        };

        let (terminal_old, publication_result) = self.retire_unset_storage(
            name,
            reported_name1,
            resolved,
            old_state,
            traced_cell,
            publication,
            &original,
        );
        if native_precision_unset && original.check(self).is_ok() {
            self.install_native_precision_trace_at(name);
        }
        if let Some(id) = receiver {
            let _ = original.retire(self, crate::vars::VariableRetirement::unpin(id));
        }
        self.refresh_unset_jim_lookup_identity(resolved, existed, &original);
        if original.check(self).is_ok()
            && let Some(variable) = native_error_unset
        {
            self.install_native_error_variable_trace(variable);
        }
        let _ = original.in_original(self, |_| drop(terminal_old));
        original
            .check(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        publication_result?;
        Ok(existed)
    }

    /// Retire one binding whose frame/namespace owner has already been
    /// detached. The stable cell identity remains available while callbacks
    /// run, but any value or trace recreated through an alias is forcibly
    /// discarded because the owner itself is dying.
    /// Retire a plain scalar local directly, as C's `TclDeleteVars` frees an
    /// untraced variable: with no observer on the cell and no Jim storage,
    /// nothing can observe the general cleanup driver's stages.
    pub(in super::super) fn retire_plain_local(&mut self, raw: VarId) -> bool {
        if self.variable_cell_observed(raw)
            || self.uses_native_jim_lookup()
            || !self.var_arena.is_plain_scalar_local(raw)
        {
            return false;
        }
        drop(self.var_arena.take_state(raw));
        self.const_vars.remove(&raw);
        self.var_arena.release_binding(raw);
        true
    }

    pub(in super::super) fn destroy_owned_variable(
        &mut self,
        name: impl AsRef<[u8]>,
        raw_id: VarId,
        reported_name1: Option<&[u8]>,
    ) {
        let original = variable_cleanup::OriginalVariableCleanup::enter(self);
        original.destroy(self, name.as_ref(), raw_id, reported_name1);
    }

    /// Remove every callback pinned to one stable variable cell. Lifecycle
    /// paths that intentionally do not execute callbacks still must release
    /// their pin before the final binding is dropped.
    fn drop_var_traces(&mut self, id: VarId) {
        let original = variable_cleanup::OriginalVariableCleanup::enter(self);
        original.drop_traces(self, id);
    }

    /// Retire trace pins below an array before a no-callback storage mutation
    /// releases the element bindings. The arena owns state/bindings and this
    /// VM owner owns callbacks, so every raw replacement funnels through this
    /// bridge instead of leaving trace-pinned, unreachable element cells.
    pub(super) fn drop_array_descendant_traces(&mut self, id: VarId) {
        let original = variable_cleanup::OriginalVariableCleanup::enter(self);
        original.drop_descendant_traces(self, id);
    }

    fn detach_variable_state(
        &mut self,
        resolved: &ResolvedVar,
        original: &variable_cleanup::OriginalVariableCleanup,
    ) -> Option<(bool, variable_cleanup::DetachedVariableState)> {
        original.detach(self, resolved)
    }

    /// Unset a single variable by name, the shared core of the `unset` command
    /// and the unset opcodes. An `a(k)` array element is removed via
    /// [`array_unset_elem`](Self::array_unset_elem) (which splits the base/key);
    /// a scalar/array variable is removed via [`unset_var`](Self::unset_var).
    /// When `complain` is set and the variable did not exist, this returns the
    /// Tcl diagnostic that distinguishes a missing variable, a scalar used as
    /// an array, and a missing array element.
    pub(crate) fn unset_one(
        &mut self,
        name: &str,
        complain: bool,
    ) -> Result<(), Completion<Value>> {
        self.unset_one_bytes(name.as_bytes(), complain)
    }

    pub(crate) fn unset_one_bytes(
        &mut self,
        name: &[u8],
        complain: bool,
    ) -> Result<(), Completion<Value>> {
        self.confine_store_bytes(name, self.current_level(), true)?;
        if let Some(refused) = self.refused_completion() {
            return Err(refused);
        }
        if self.observed_name_policy_selected() {
            return match self.observed_variable_unset(name, self.current_level()) {
                Ok(true) => Ok(()),
                Ok(false) if !complain => Ok(()),
                Ok(false) => Err(self.refuse_host_command(
                    "observed missing unset completion is unavailable".into(),
                )),
                Err(error) => Err(self.refuse_host_command(format!(
                    "observed variable storage is unavailable: {error}"
                ))),
            };
        }
        if self.name_policy_protocol().is_none() {
            return Err(
                self.refuse_host_command("native variable name policy is unavailable".into())
            );
        }
        let resolved = self.resolve_var_from_bytes(name, self.current_level());
        self.unset_selected_original_variable(name, None, resolved.as_ref(), complain)
    }

    /// Unset the already selected original-object receiver without replaying
    /// its written name through the current frame's resolver.
    pub(in super::super) fn unset_selected_original_variable(
        &mut self,
        name: &[u8],
        reported_name1: Option<&[u8]>,
        resolved: Option<&ResolvedVar>,
        complain: bool,
    ) -> Result<(), Completion<Value>> {
        if resolved
            .as_ref()
            .and_then(|resolved| resolved.id)
            .is_some_and(|id| self.const_vars.contains(&id))
        {
            return if complain {
                Err(self.variable_access_error_at(
                    "unset",
                    tcl_syntax::naming::NativeVariableInputForm::Combined(name),
                    "variable is a constant",
                    tcl_syntax::naming::NativeVariableFailureSite::ValueUnset,
                ))
            } else {
                Ok(())
            };
        }
        let reason = resolved.as_ref().map_or("no such variable", |resolved| {
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
            } else {
                "no such variable"
            }
        });
        let existed = if self.dictionary_variable_containers()
            && let Some(resolved) = &resolved
            && let Some(key) = &resolved.elem
            && let Some(root) = resolved.base_id
        {
            match self.remove_dictionary_element_bytes(root, key.as_bytes()) {
                Ok(removed) => removed,
                Err(error) => {
                    if error.native_access_refusal().is_some() {
                        return Err(crate::command::completion_from_cmd_error(
                            self,
                            error.into(),
                        ));
                    }
                    if complain {
                        return Err(self.variable_access_error_bytes(
                            "unset",
                            name,
                            "variable isn't array",
                        ));
                    }
                    return Ok(());
                }
            }
        } else {
            match resolved.as_ref() {
                Some(resolved) => self.unset_resolved_reporting(name, reported_name1, resolved)?,
                None => false,
            }
        };
        if let Some(refused) = self.refused_completion() {
            return Err(refused);
        }
        if !existed && complain {
            // A name that reached a valueless cell, such as an element a trace
            // created, fails at the unset itself.
            let reached_cell = resolved.is_some_and(|resolved| resolved.id.is_some());
            return Err(self.variable_access_error_at(
                "unset",
                tcl_syntax::naming::NativeVariableInputForm::Combined(name),
                reason,
                if reached_cell {
                    tcl_syntax::naming::NativeVariableFailureSite::ValueUnset
                } else {
                    tcl_syntax::naming::NativeVariableFailureSite::NameLookup
                },
            ));
        }
        Ok(())
    }

    /// Frame-addressed unset (storage only — no unset-trace firing).
    pub(crate) fn unset_from(&mut self, start: usize, name: &str) -> bool {
        self.unset_from_bytes(start, name.as_bytes())
    }

    pub(in super::super) fn unset_from_bytes(&mut self, start: usize, name: &[u8]) -> bool {
        let Some(resolved) = self.resolve_var_from_bytes(name, start) else {
            return false;
        };
        self.unbind_storage_resolved(&resolved)
    }

    /// Storage-only destruction shared by frame-addressed whole-variable and
    /// element façades. No callback runs, so the reached cell's trace pin is
    /// retired after the binding/state transition.
    pub(in super::super) fn unbind_storage_resolved(&mut self, resolved: &ResolvedVar) -> bool {
        if self.dictionary_variable_containers()
            && let Some(key) = &resolved.elem
        {
            return resolved
                .base_id
                .is_some_and(|id| self.remove_dictionary_element(id, key));
        }
        let id = resolved.id;
        let publication = id.and_then(|id| self.authored_static_unset_address(id));
        let existed = self.unbind_resolved(resolved);
        let _ = self.publish_authored_static_unset(publication);
        if let Some(id) = id {
            self.drop_var_traces(id);
        }
        existed
    }
}
