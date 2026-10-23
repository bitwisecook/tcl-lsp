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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Mandatory retirement in the actual original variable arena.

use super::native_operation_currency::{
    CleanupRestoration, NativeOperationScope, SavedExecutionTarget,
};
use super::{GuardDomain, InterpId, InterpState, Local, TakenVarTraces, VarTraceCell, Vm};
use crate::vars::{VarState, VariableRetirement, WeakJimVariableCell};
use std::rc::{Rc, Weak};
use tcl_cmd_core::CmdError;
use tcl_core_types::VarId;
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::value::ValueError;

/// Physical storage observation is separate from an entered operation. Neither
/// this receipt nor a weak cell creates a native Var/reference or name purpose.
pub(super) struct OriginalVariableCleanup {
    target: SavedExecutionTarget,
    interpreter: InterpId,
    arena: Weak<()>,
    world: Weak<std::cell::RefCell<super::NativeNameWorld>>,
    frame: Weak<crate::frame::ActivationIdentity>,
    level: usize,
    namespace: Option<super::namespace_stack::NamespaceActivationObservation>,
    namespace_depth: usize,
    operation: Result<NativeOperationScope, CmdError>,
    c_protocol: Option<tcl_syntax::native_variable_name::NativeVariableNameProtocol>,
}

pub(super) fn retain(vm: &mut Vm, error: &CmdError) {
    if let Some(first) = error.native_execution_refusal() {
        vm.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(first.clone()));
    } else if let Some(first) = error.native_access_refusal() {
        vm.retain_tcl_host_failure(crate::error::TclHostFailure::ValueAccess(first));
    }
}

impl OriginalVariableCleanup {
    pub(super) fn c_protocol(
        &self,
    ) -> Option<tcl_syntax::native_variable_name::NativeVariableNameProtocol> {
        self.c_protocol
    }

    pub(super) fn scope(&self) -> &NativeOperationScope {
        self.operation
            .as_ref()
            .expect("checked entered variable operation")
    }

    pub(super) fn enter(vm: &mut Vm) -> Self {
        let arena = vm.var_arena.weak_birth();
        let world = Rc::downgrade(&vm.name_world);
        let level = vm.current_level();
        let frame = Rc::downgrade(&vm.frames[level].activation);
        let namespace = vm.resolution_stacks.observe_activation();
        let namespace_depth = vm.resolution_stacks.ns_id_stack.len();
        let c_protocol = vm.native_c_variable_name_protocol();
        let operation = NativeOperationScope::enter(vm);
        if let Err(first) = &operation {
            retain(vm, first);
        }
        let target = SavedExecutionTarget::capture(vm);
        let interpreter = target.interpreter();
        Self {
            target,
            interpreter,
            arena,
            world,
            frame,
            level,
            namespace,
            namespace_depth,
            operation,
            c_protocol,
        }
    }

    fn unavailable(vm: &mut Vm) -> CmdError {
        let error: CmdError =
            ValueError::CommandProtocolUnavailable("original variable cleanup arena").into();
        retain(vm, &error);
        vm.execution_refusal
            .clone()
            .map(CmdError::from_execution_refusal)
            .unwrap_or(error)
    }

    fn physical(&self, vm: &Vm) -> bool {
        self.target.owns_target(vm)
            && vm.st_of(self.interpreter).is_some_and(|state| {
                state.var_arena.has_original_birth(&self.arena)
                    && self
                        .world
                        .upgrade()
                        .is_some_and(|world| Rc::ptr_eq(&world, &state.name_world))
            })
    }

    pub(super) fn state_mut<'a>(&self, vm: &'a mut Vm) -> Result<&'a mut InterpState, CmdError> {
        if !self.physical(vm) {
            return Err(Self::unavailable(vm));
        }
        if vm.cur == self.interpreter {
            return Ok(&mut vm.state);
        }
        Ok(vm.interps[self.interpreter.0]
            .parked
            .as_deref_mut()
            .expect("checked original variable storage"))
    }

    pub(super) fn cell(&self, vm: &mut Vm, id: VarId) -> Option<WeakJimVariableCell> {
        self.state_mut(vm).ok()?.var_arena.weak_jim_cell(id)
    }

    pub(super) fn has_cell(&self, vm: &mut Vm, selected: &WeakJimVariableCell) -> bool {
        self.state_mut(vm)
            .is_ok_and(|state| state.var_arena.validate_jim_cell(selected).is_some())
    }

    pub(super) fn check(&self, vm: &mut Vm) -> Result<(), CmdError> {
        let current = match &self.operation {
            Ok(operation) => operation.check(vm),
            Err(first) => Err(first.clone()),
        };
        if let Err(first) = &current {
            retain(vm, first);
        }
        current?;
        if !self.physical(vm)
            || vm.current_level() != self.level
            || vm.resolution_stacks.ns_id_stack.len() != self.namespace_depth
            || self
                .namespace
                .as_ref()
                .is_some_and(|namespace| !namespace.original_present(&vm.resolution_stacks))
            || !self.frame.upgrade().is_some_and(|frame| {
                vm.frames
                    .get(self.level)
                    .is_some_and(|current| Rc::ptr_eq(&frame, &current.activation))
            })
        {
            return Err(Self::unavailable(vm));
        }
        Ok(())
    }

    /// Re-enter only the authentic physical owner for already-owned cleanup.
    /// Check before restoring the caller so a child callback cannot be erased.
    pub(super) fn in_original<T>(
        &self,
        vm: &mut Vm,
        work: impl FnOnce(&mut Vm) -> T,
    ) -> Result<T, CmdError> {
        let _ = self.check(vm);
        if !self.physical(vm) {
            return Err(Self::unavailable(vm));
        }
        let reached = self
            .target
            .retire_in_original(vm, CleanupRestoration::PreserveCallbackSelection, |vm| {
                let metadata = work(vm);
                if let Err(first) = self.check(vm) {
                    retain(vm, &first);
                }
                metadata
            })
            .map_err(CmdError::from_execution_refusal)?;
        if let Some(first) = reached.first_cause {
            retain(vm, &CmdError::from_execution_refusal(first));
        }
        Ok(reached.metadata)
    }

    /// Every release leaves storage access, while later pure steps still use
    /// the original arena. A prior first cause does not abandon owning roles.
    pub(super) fn retire(&self, vm: &mut Vm, mut work: VariableRetirement) -> Option<VarState> {
        loop {
            let _ = self.check(vm);
            let release = match self.state_mut(vm) {
                Ok(state) => work.next(&mut state.var_arena),
                Err(first) => {
                    retain(vm, &first);
                    // No current or reborn arena receives the withdrawn work.
                    let _ = self.in_original(vm, |_| drop(work));
                    return None;
                }
            };
            let Some(release) = release else {
                break;
            };
            let retired = self.in_original(vm, |_| release.retire(&mut work));
            if let Err(first) = retired {
                retain(vm, &first);
                return None;
            }
        }
        let _ = self.check(vm);
        work.take_detached()
    }

    pub(super) fn check_native(&self, vm: &mut Vm) -> Result<(), NativeExecutionError> {
        match self.check(vm) {
            Ok(()) => Ok(()),
            Err(error) => {
                let first = error
                    .native_execution_refusal()
                    .cloned()
                    .unwrap_or_else(|| {
                        NativeExecutionError::ValueAccessRefusal(
                            error
                                .native_access_refusal()
                                .expect("physical cleanup emits a typed original refusal"),
                        )
                    });
                Err(vm.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(first)))
            }
        }
    }
}

impl OriginalVariableCleanup {
    /// The pin belongs to the original trace list, even if its callback leaves
    /// a child current. Later callback getters stop; existing roles still retire.
    pub(super) fn fire_taken(
        &self,
        vm: &mut Vm,
        name: &[u8],
        reported: Option<&[u8]>,
        cell: VarTraceCell,
        include_array: bool,
    ) -> Result<(), NativeExecutionError> {
        let Some(id) = cell.id else {
            return self.check_native(vm);
        };
        let (taken, has_array) = match self.state_mut(vm) {
            Ok(state) => {
                let taken = state.variable_observers.script_traces.remove(&id);
                let has_array = include_array
                    && cell.array_id().is_some_and(|array| {
                        state.variable_observers.script_traces.contains_key(&array)
                    });
                (taken, has_array)
            }
            Err(_) => return self.check_native(vm),
        };
        let had_taken = taken.is_some();
        if !had_taken && !has_array {
            return self.check_native(vm);
        }
        if had_taken {
            let _ = self.in_original(vm, |vm| {
                vm.invalidate_guard_domain(GuardDomain::VariableTrace);
            });
        }
        let selected = self.cell(vm, id);
        let reached = match self.check_native(vm) {
            Ok(()) => self
                .in_original(vm, |vm| {
                    vm.fire_var_traces_from_cell_bytes(
                        name,
                        "unset",
                        reported,
                        Some(TakenVarTraces {
                            cell,
                            entries: taken.unwrap_or_default(),
                            include_array,
                        }),
                        None,
                    )
                })
                .unwrap_or_else(|error| Err(native_failure(vm, &error))),
            Err(first) => {
                let _ = self.in_original(vm, |_| drop(taken));
                Err(first)
            }
        };
        if let Err(first) = &reached {
            vm.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(first.clone()));
        }
        if had_taken
            && selected
                .as_ref()
                .is_some_and(|selected| self.has_cell(vm, selected))
        {
            let _ = self.retire(vm, VariableRetirement::unpin(id));
        }
        let current = self.check_native(vm);
        match reached {
            Err(first) => Err(first),
            Ok(_ignored_guest) => current,
        }
    }

    pub(super) fn drop_traces(&self, vm: &mut Vm, id: VarId) {
        let selected = self.cell(vm, id);
        let taken = self
            .state_mut(vm)
            .ok()
            .and_then(|state| state.variable_observers.script_traces.remove(&id));
        if let Some(taken) = taken {
            let _ = self.in_original(vm, |_| drop(taken));
            if selected
                .as_ref()
                .is_some_and(|selected| self.has_cell(vm, selected))
            {
                let _ = self.retire(vm, VariableRetirement::unpin(id));
            }
            let _ = self.in_original(vm, |vm| {
                vm.invalidate_guard_domain(GuardDomain::VariableTrace);
            });
        }
        let _ = self.check(vm);
    }

    /// Remove only already selected cells. All recursive child binding/key
    /// roles use the same physical arena; no callback name is resolved anew.
    pub(super) fn purge(&self, vm: &mut Vm, raw: VarId) {
        let (link, resolved) = match self.state_mut(vm) {
            Ok(state) => (
                matches!(
                    state.var_arena.get(raw).map(crate::vars::VarCell::state),
                    Some(Local::Link(_) | Local::NameLink(_))
                ),
                state.var_arena.resolve(raw),
            ),
            Err(_) => return,
        };
        if link || resolved.is_none() {
            let _ = self.retire(vm, VariableRetirement::clear(raw));
            return;
        }
        let id = resolved.expect("selected non-link cell");
        self.drop_traces(vm, id);
        if let Some(old) = self.retire(vm, VariableRetirement::detach(id)) {
            match old {
                Local::Array(elements) => {
                    if let Ok(elements) = self.in_original(vm, |_| elements.into_iter()) {
                        for (_, child) in elements {
                            self.purge(vm, child);
                            let _ = self.retire(vm, VariableRetirement::retire_key(child));
                            let _ = self.retire(vm, VariableRetirement::unbind(child));
                        }
                    }
                }
                state => {
                    let _ = self.retire(vm, VariableRetirement::state(state));
                }
            }
        }
        let _ = self.retire(vm, VariableRetirement::clear(id));
        if let Ok(state) = self.state_mut(vm) {
            state.const_vars.remove(&id);
        }
        let _ = self.check(vm);
    }

    pub(super) fn purge_binding(&self, vm: &mut Vm, raw: VarId) {
        self.purge(vm, raw);
        let _ = self.retire(vm, VariableRetirement::unbind(raw));
    }

    pub(super) fn retire_elements(
        &self,
        vm: &mut Vm,
        name: &[u8],
        parent: Option<VarId>,
        elements: crate::vars::VarTable,
        reported: Option<&[u8]>,
        preserve: bool,
        table_holds: bool,
        dying: bool,
    ) -> Vec<VarId> {
        let mut retired = Vec::new();
        let Ok(elements) = self.in_original(vm, |_| elements.into_iter()) else {
            return retired;
        };
        for (element, raw) in elements {
            let id = self
                .state_mut(vm)
                .ok()
                .and_then(|state| state.var_arena.resolve(raw));
            let Some(id) = id else {
                let _ = self.retire(vm, VariableRetirement::retire_key(raw));
                let _ = self.retire(vm, VariableRetirement::unbind(raw));
                continue;
            };
            if self.c_protocol.is_some() {
                // DeleteArray releases the old stored scalar while its pointer
                // is visible, then clears it and invalidates the entry.
                self.release_visible_scalar(vm, id);
                if let Ok(state) = self.state_mut(vm) {
                    state.var_arena.invalidate_destroyed_element(id, preserve);
                }
            } else {
                if let Ok(state) = self.state_mut(vm) {
                    state.var_arena.invalidate_destroyed_element(id, preserve);
                }
                if let Some(old) = self.retire(vm, VariableRetirement::detach(id)) {
                    let _ = self.retire(vm, VariableRetirement::state(old));
                }
            }
            let cell = VarTraceCell {
                id: Some(id),
                array: parent,
                elem: Some(element.clone()),
            };
            let mut spelling = name.to_vec();
            spelling.push(b'(');
            spelling.extend_from_slice(element.as_bytes());
            spelling.push(b')');
            let _ = self.fire_taken(vm, &spelling, reported.or(Some(name)), cell, false);
            if let Ok(state) = self.state_mut(vm) {
                state.var_arena.finish_destroyed_element_trace(id);
            }
            if dying {
                self.purge(vm, raw);
            } else {
                let _ = self.retire(vm, VariableRetirement::clear(id));
                if let Ok(state) = self.state_mut(vm) {
                    state.const_vars.remove(&id);
                }
            }
            let _ = self.retire(vm, VariableRetirement::retire_key(raw));
            if table_holds {
                retired.push(raw);
            } else {
                let _ = self.retire(vm, VariableRetirement::unbind(raw));
            }
        }
        retired
    }

    pub(super) fn destroy(&self, vm: &mut Vm, name: &[u8], raw: VarId, reported: Option<&[u8]>) {
        let protocol = self.c_protocol;
        let (is_static, id) = match self.state_mut(vm) {
            Ok(state) => (
                state.var_arena.has_static_refs(raw),
                state.var_arena.resolve(raw),
            ),
            Err(_) => return,
        };
        if is_static {
            let _ = self.retire(vm, VariableRetirement::unbind(raw));
            return;
        }
        let Some(id) = id else {
            self.purge_binding(vm, raw);
            return;
        };
        let detached = self
            .unset_state(vm, id, true)
            .unwrap_or(DetachedVariableState {
                state: Local::Undefined,
                default: None,
            });
        let (old, default) = detached.into_parts();
        let key_drain = if let Local::Array(elements) = &old {
            if let Ok(state) = self.state_mut(vm) {
                state
                    .var_arena
                    .begin_element_destruction(elements.values().copied());
            }
            Some(elements.destruction_key_drain())
        } else {
            None
        };
        let table_holds = protocol.is_some_and(
            tcl_syntax::native_variable_name::NativeVariableNameProtocol::element_table_retains_original);
        let preserve = protocol.is_some_and(
            tcl_syntax::native_variable_name::NativeVariableNameProtocol::element_unset_preserves_definition_during_trace);
        let _ = self.fire_taken(
            vm,
            name,
            reported,
            VarTraceCell {
                id: Some(id),
                array: None,
                elem: None,
            },
            true,
        );
        let terminal_old = match old {
            Local::Array(elements) => {
                let retired = self.retire_elements(
                    vm,
                    name,
                    Some(id),
                    elements,
                    reported,
                    preserve,
                    table_holds,
                    true,
                );
                for id in retired {
                    let _ = self.retire(vm, VariableRetirement::unbind(id));
                }
                None
            }
            state if protocol.is_some() => {
                let _ = self.retire(vm, VariableRetirement::state(state));
                None
            }
            state => Some(state),
        };
        let _ = self.in_original(vm, |_| drop(default));
        let _ = self.in_original(vm, |_| drop(key_drain));
        self.purge_binding(vm, raw);
        // The neutral worker kept an unmatched old scalar/name payload until
        // its terminal local drop. This wrapper preserves that distinct phase.
        let _ = self.in_original(vm, |_| drop(terminal_old));
        let _ = self.check(vm);
    }
}

fn native_failure(vm: &mut Vm, error: &CmdError) -> NativeExecutionError {
    let first = error
        .native_execution_refusal()
        .cloned()
        .unwrap_or_else(|| {
            NativeExecutionError::ValueAccessRefusal(
                error
                    .native_access_refusal()
                    .expect("physical cleanup emits a typed original refusal"),
            )
        });
    vm.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(first))
}

impl OriginalVariableCleanup {
    fn release_visible_scalar(&self, vm: &mut Vm, id: VarId) {
        let ticket = self
            .state_mut(vm)
            .ok()
            .and_then(|state| state.var_arena.take_visible_scalar(id));
        if let Some(ticket) = ticket {
            let (old, marker) = ticket.into_parts();
            let _ = self.in_original(vm, |_| drop(old));
            let visible = self
                .state_mut(vm)
                .ok()
                .and_then(|state| state.var_arena.finish_visible_scalar(id, &marker));
            if let Some(visible) = visible {
                let _ = self.retire(vm, VariableRetirement::state(visible));
            } else {
                let error: CmdError = ValueError::CommandProtocolUnavailable(
                    "original variable scalar retirement slot",
                )
                .into();
                retain(vm, &error);
            }
        }
        let _ = self.check(vm);
    }
}

/// Actual caller transport at the shared operation-reference boundary.
/// The existing Boolean facade retains its full refusal in the VM; fallible
/// consumers receive the same cause as their existing Completion error.
pub(super) trait VariableOperationOutcome {
    fn refused(vm: &mut Vm, error: CmdError) -> Self;
}
impl<T> VariableOperationOutcome for Result<T, tcl_runtime_api::Completion<crate::Value>> {
    fn refused(vm: &mut Vm, error: CmdError) -> Self {
        Err(crate::command::completion_from_cmd_error(vm, error))
    }
}
impl VariableOperationOutcome for tcl_runtime_api::Completion<crate::Value> {
    fn refused(vm: &mut Vm, error: CmdError) -> Self {
        crate::command::completion_from_cmd_error(vm, error)
    }
}
impl VariableOperationOutcome for bool {
    fn refused(vm: &mut Vm, error: CmdError) -> Self {
        retain(vm, &error);
        false
    }
}

impl OriginalVariableCleanup {
    pub(super) fn release_binding(
        &self,
        vm: &mut Vm,
        selected: &super::ArrayOperationBinding,
        id: VarId,
    ) {
        let super::ArrayOperationBinding::Variable {
            binding,
            table,
            frame,
        } = selected
        else {
            return;
        };
        let removed = self
            .in_original(vm, |vm| {
                if let Some(frame) = frame {
                    let super::VarTableOwner::Frame(level) = binding.owner else {
                        return None;
                    };
                    if !frame.upgrade().is_some_and(|selected| {
                        vm.frames
                            .get(level)
                            .is_some_and(|current| Rc::ptr_eq(&current.activation, &selected))
                    }) {
                        return None;
                    }
                }
                let discard = vm.var_arena.is_final_operation_ref(id)
                    && matches!(
                        vm.var_arena.get(id).map(crate::vars::VarCell::state),
                        Some(Local::Undefined)
                    )
                    && !vm.var_arena.has_link_refs(id)
                    && !vm.variable_observers.script_traces.contains_key(&id);
                if !discard {
                    return None;
                }
                let actual = vm.var_table_mut(binding.owner)?;
                if !actual.has_original_birth(table) || actual.get(&binding.name) != Some(&id) {
                    return None;
                }
                let removed = actual.take_binding(&binding.name);
                if removed.is_some() {
                    vm.const_vars.remove(&id);
                }
                removed
            })
            .ok()
            .flatten();
        if let Some(removed) = removed {
            let raw = removed.id();
            let _ = self.in_original(vm, |_| {
                removed.release_key();
            });
            let _ = self.retire(vm, VariableRetirement::unbind(raw));
        }
        let _ = self.check(vm);
    }
}

/// The already selected slot is independent of a callback's current frame.
/// Table/cell observations acquire no owning variable or key role.
struct VariableSlot {
    binding: Option<super::VarBinding>,
    key: tcl_core_types::NameBytes,
    table: Weak<()>,
    frame: Option<Weak<crate::frame::ActivationIdentity>>,
    parent: Option<WeakJimVariableCell>,
    raw: VarId,
}

fn table(state: &InterpState, owner: super::VarTableOwner) -> Option<&crate::vars::VarTable> {
    match owner {
        super::VarTableOwner::Frame(level) => state.frames.get(level).map(|frame| &frame.locals),
        super::VarTableOwner::Namespace(super::ROOT_NS) => {
            state.frames.first().map(|frame| &frame.locals)
        }
        super::VarTableOwner::Namespace(id) => state.ns_vars.get(&id),
        super::VarTableOwner::ProcedureStatics(level) => state
            .frames
            .get(level)?
            .statics
            .as_ref()
            .map(|statics| &statics.table),
        super::VarTableOwner::CompiledLocal { .. } => None,
    }
}
fn table_mut(
    state: &mut InterpState,
    owner: super::VarTableOwner,
) -> Option<&mut crate::vars::VarTable> {
    match owner {
        super::VarTableOwner::Frame(level) => {
            state.frames.get_mut(level).map(|frame| &mut frame.locals)
        }
        super::VarTableOwner::Namespace(super::ROOT_NS) => {
            state.frames.first_mut().map(|frame| &mut frame.locals)
        }
        super::VarTableOwner::Namespace(id) => state.ns_vars.get_mut(&id),
        super::VarTableOwner::ProcedureStatics(_) | super::VarTableOwner::CompiledLocal { .. } => {
            None
        }
    }
}

impl OriginalVariableCleanup {
    pub(super) fn detach(
        &self,
        vm: &mut Vm,
        resolved: &super::ResolvedVar,
    ) -> Option<(bool, DetachedVariableState)> {
        let id = resolved.id?;
        let (raw_is_link, selected) = {
            let state = self.state_mut(vm).ok()?;
            let (raw, actual, parent) = if let Some(key) = &resolved.elem {
                let parent = state.var_arena.weak_jim_cell(resolved.base_id?)?;
                let Local::Array(elements) = state.var_arena.get(resolved.base_id?)?.state() else {
                    return None;
                };
                (*elements.get(key)?, Some(elements), Some(parent))
            } else if let super::VarTableOwner::CompiledLocal { level, slot } =
                resolved.binding.owner
            {
                (
                    state.frames.get(level)?.compiled_locals.get(slot)?.1,
                    None,
                    None,
                )
            } else {
                let actual = table(state, resolved.binding.owner)?;
                (*actual.get(&resolved.binding.name)?, Some(actual), None)
            };
            let raw_is_link = matches!(
                state.var_arena.get(raw).map(crate::vars::VarCell::state),
                Some(Local::Link(_) | Local::NameLink(_))
            );
            let frame = match resolved.binding.owner {
                super::VarTableOwner::Frame(level) => {
                    Some(Rc::downgrade(&state.frames.get(level)?.activation))
                }
                super::VarTableOwner::Namespace(super::ROOT_NS) => {
                    Some(Rc::downgrade(&state.frames.first()?.activation))
                }
                _ => None,
            };
            let selected = actual.map(|actual| VariableSlot {
                binding: Some(resolved.binding.clone()),
                key: resolved
                    .elem
                    .clone()
                    .unwrap_or_else(|| resolved.binding.name.clone()),
                table: actual.weak_birth(),
                frame,
                parent,
                raw,
            });
            (raw_is_link, selected)
        };
        let old = self.unset_state(vm, id, false)?;
        let existed = matches!(old.state, Local::Scalar(_) | Local::Array(_));
        let remove = !raw_is_link
            && self.state_mut(vm).is_ok_and(|state| {
                !state.var_arena.has_link_refs(id) && !state.var_arena.has_operation_refs(id)
            });
        if remove && let Some(selected) = selected {
            self.remove_selected(vm, &selected, true, true);
        }
        Some((existed, old))
    }

    fn remove_selected(
        &self,
        vm: &mut Vm,
        selected: &VariableSlot,
        invalidate_searches: bool,
        forget_constant: bool,
    ) {
        let removed = {
            let Ok(state) = self.state_mut(vm) else {
                return;
            };
            if let Some(frame) = &selected.frame {
                let level = match selected.binding.as_ref().map(|binding| binding.owner) {
                    Some(super::VarTableOwner::Frame(level)) => level,
                    _ => 0,
                };
                if !frame.upgrade().is_some_and(|frame| {
                    state
                        .frames
                        .get(level)
                        .is_some_and(|current| Rc::ptr_eq(&frame, &current.activation))
                }) {
                    return;
                }
            }
            let key = selected.key.as_bytes();
            if let Some(parent) = &selected.parent {
                state.var_arena.take_array_binding_for_cleanup(
                    parent,
                    &selected.table,
                    key,
                    selected.raw,
                )
            } else {
                let Some(actual) = table_mut(
                    state,
                    selected
                        .binding
                        .as_ref()
                        .expect("selected ordinary table")
                        .owner,
                ) else {
                    return;
                };
                if !actual.has_original_birth(&selected.table)
                    || actual.get(key) != Some(&selected.raw)
                {
                    return;
                }
                actual.take_binding(key)
            }
        };
        if let Some(removed) = removed {
            let _ = self.in_original(vm, |_| {
                removed.release_key();
            });
            if let Some(parent) = &selected.parent {
                if let Some(parent_id) = self
                    .state_mut(vm)
                    .ok()
                    .and_then(|state| state.var_arena.validate_jim_cell(parent))
                {
                    let current = self.state_mut(vm).is_ok_and(|state| state.var_arena.get(parent_id)
                        .is_some_and(|cell| matches!(cell.state(), Local::Array(actual) if actual.has_original_birth(&selected.table))));
                    if current && invalidate_searches {
                        let _ = self.retire(vm, VariableRetirement::searches(parent_id));
                    }
                }
                let _ = self.retire(vm, VariableRetirement::retire_key(selected.raw));
            }
            if forget_constant && let Ok(state) = self.state_mut(vm) {
                state.const_vars.remove(&selected.raw);
            }
            let _ = self.retire(vm, VariableRetirement::unbind(selected.raw));
        }
        let _ = self.check(vm);
    }
}

impl OriginalVariableCleanup {
    pub(super) fn drop_descendant_traces(&self, vm: &mut Vm, id: VarId) {
        let children = self
            .state_mut(vm)
            .ok()
            .map(
                |state| match state.var_arena.get(id).map(crate::vars::VarCell::state) {
                    Some(Local::Array(elements)) => elements.values().copied().collect::<Vec<_>>(),
                    _ => Vec::new(),
                },
            )
            .unwrap_or_default();
        for raw in children {
            if let Some(child) = self
                .state_mut(vm)
                .ok()
                .and_then(|state| state.var_arena.resolve(raw))
            {
                self.drop_descendant_traces(vm, child);
                self.drop_traces(vm, child);
                if let Ok(state) = self.state_mut(vm) {
                    state.const_vars.remove(&child);
                }
            }
        }
        let _ = self.check(vm);
    }

    pub(super) fn namespace(
        &self,
        vm: &mut Vm,
        canonical: &[u8],
        namespace: tcl_runtime_api::NsId,
    ) {
        let jim = vm.jim_variable_lookup();
        let owner = super::VarTableOwner::Namespace(if jim { super::ROOT_NS } else { namespace });
        let Some(table_birth) = self
            .state_mut(vm)
            .ok()
            .and_then(|state| table(state, owner).map(crate::vars::VarTable::weak_birth))
        else {
            return;
        };
        if jim {
            let prefix = (!canonical.is_empty()).then(|| {
                let mut prefix = canonical.to_vec();
                prefix.extend_from_slice(b"::");
                prefix
            });
            let names = self
                .state_mut(vm)
                .ok()
                .and_then(|state| table(state, owner))
                .map(|table| {
                    table
                        .iter()
                        .filter(|(name, _)| {
                            prefix
                                .as_ref()
                                .is_none_or(|prefix| name.as_bytes().starts_with(prefix))
                        })
                        .map(|(name, raw)| (name.clone(), *raw))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            for (name, raw) in names {
                let binding = super::VarBinding {
                    owner,
                    name: name.clone(),
                };
                if self.check(vm).is_ok() {
                    let mut written = b"::".to_vec();
                    written.extend_from_slice(name.as_bytes());
                    let _ = self.in_original(vm, |vm| vm.unset_var_bytes(&written));
                } else {
                    let selected = VariableSlot {
                        binding: Some(binding),
                        key: name,
                        table: table_birth.clone(),
                        frame: None,
                        parent: None,
                        raw,
                    };
                    self.remove_selected(vm, &selected, true, true);
                    self.purge(vm, raw);
                }
            }
            let _ = self.check(vm);
            return;
        }
        loop {
            let next = self.state_mut(vm).ok().and_then(|state| {
                let actual = table(state, owner)?;
                if !actual.has_original_birth(&table_birth) {
                    return None;
                }
                actual
                    .keys()
                    .min()
                    .and_then(|name| actual.get(name).map(|raw| (name.clone(), *raw)))
            });
            let Some((name, raw)) = next else {
                break;
            };
            let binding = super::VarBinding {
                owner,
                name: name.clone(),
            };
            let original_nonlink = self.state_mut(vm).is_ok_and(|state| {
                !matches!(
                    state.var_arena.get(raw).map(crate::vars::VarCell::state),
                    Some(Local::Link(_) | Local::NameLink(_))
                )
            });
            if original_nonlink && self.check(vm).is_ok() {
                let mut written = b"::".to_vec();
                if !canonical.is_empty() {
                    written.extend_from_slice(canonical);
                    written.extend_from_slice(b"::");
                }
                written.extend_from_slice(name.as_bytes());
                let _ = self.in_original(vm, |vm| {
                    let resolved = super::ResolvedVar {
                        binding: binding.clone(),
                        base_id: vm.var_arena.resolve(raw),
                        id: vm.var_arena.resolve(raw),
                        elem: None,
                    };
                    vm.unset_resolved(&written, &resolved)
                });
            }
            // The dying original table owns callback-created same-slot contents.
            // Select only its physical row, never the callback's current world.
            let removed = self.state_mut(vm).ok().and_then(|state| {
                let actual = table_mut(state, owner)?;
                actual
                    .has_original_birth(&table_birth)
                    .then(|| actual.take_binding(&name))
                    .flatten()
            });
            if let Some(removed) = removed {
                let recreated = removed.id();
                let _ = self.in_original(vm, |_| {
                    removed.release_key();
                });
                self.purge_binding(vm, recreated);
            }
        }
        if namespace != super::ROOT_NS {
            let removed = self.state_mut(vm).ok().and_then(|state| {
                state
                    .ns_vars
                    .get(&namespace)
                    .is_some_and(|actual| actual.has_original_birth(&table_birth))
                    .then(|| state.ns_vars.remove(&namespace))
                    .flatten()
            });
            let _ = self.in_original(vm, |_| drop(removed));
        }
        let _ = self.check(vm);
    }
}

impl OriginalVariableCleanup {
    pub(super) fn detach_member(
        &self,
        vm: &mut Vm,
        base: VarId,
        key: &[u8],
        raw: VarId,
        id: VarId,
    ) -> Option<(bool, DetachedVariableState)> {
        let (selected, cell, raw_is_link) = {
            let state = self.state_mut(vm).ok()?;
            let parent = state.var_arena.weak_jim_cell(base)?;
            let cell = state.var_arena.weak_jim_cell(id)?;
            let Local::Array(elements) = state.var_arena.get(base)?.state() else {
                return None;
            };
            if elements.get(key) != Some(&raw) {
                return None;
            }
            let raw_is_link = matches!(
                state.var_arena.get(raw).map(crate::vars::VarCell::state),
                Some(Local::Link(_) | Local::NameLink(_))
            );
            (
                VariableSlot {
                    binding: None,
                    key: key.into(),
                    table: elements.weak_birth(),
                    frame: None,
                    parent: Some(parent),
                    raw,
                },
                cell,
                raw_is_link,
            )
        };
        self.drop_descendant_traces(vm, id);
        if !self.has_cell(vm, &cell) {
            return None;
        }
        let old = self.unset_state(vm, id, false)?;
        let existed = matches!(old.state, Local::Scalar(_) | Local::Array(_));
        let remove = !raw_is_link
            && self.state_mut(vm).is_ok_and(|state| {
                !state.var_arena.has_link_refs(id) && !state.var_arena.has_operation_refs(id)
            });
        if remove {
            self.remove_selected(vm, &selected, true, true);
        }
        Some((existed, old))
    }
}

/// Actual detached roles; C91 default ownership belongs to the old array
/// until its reached `DeleteArrayVar` stage. This carrier adds no references.
pub(super) struct DetachedVariableState {
    state: VarState,
    default: Option<[crate::Value; 2]>,
}
impl DetachedVariableState {
    pub(super) fn into_parts(self) -> (VarState, Option<[crate::Value; 2]>) {
        (self.state, self.default)
    }
}
impl OriginalVariableCleanup {
    fn unset_state(
        &self,
        vm: &mut Vm,
        id: VarId,
        mandatory: bool,
    ) -> Option<DetachedVariableState> {
        if self.c_protocol.is_none() {
            return self.retire(vm, VariableRetirement::unset(id)).map(|state| {
                DetachedVariableState {
                    state,
                    default: None,
                }
            });
        }
        let selected = self.cell(vm, id)?;
        let searches = self
            .state_mut(vm)
            .ok()
            .and_then(|state| state.var_arena.original_unset_search_owner(id));
        if let Some(searches) = searches {
            let _ = self.retire(vm, VariableRetirement::searches(searches));
        }
        // Reached old search owners always retire. A refused ordinary unset
        // does not advance into a new undefined publication after that release.
        if !mandatory && self.check(vm).is_err() {
            return None;
        }
        if !self.has_cell(vm, &selected) {
            return None;
        }
        let default = self
            .state_mut(vm)
            .ok()
            .and_then(|state| state.var_arena.take_array_default_for_cleanup(id));
        let state = self.retire(vm, VariableRetirement::detach_original_c(id))?;
        Some(DetachedVariableState { state, default })
    }
}

#[cfg(test)]
mod tests;

impl OriginalVariableCleanup {
    pub(super) fn detach_static(&self, vm: &mut Vm, resolved: &super::ResolvedVar) -> Option<bool> {
        if let Err(first) = self.check(vm) {
            retain(vm, &first);
            return Some(false);
        }
        if resolved.elem.is_some() {
            return None;
        }
        if matches!(
            resolved.binding.owner,
            super::VarTableOwner::ProcedureStatics(_)
        ) {
            return Some(false);
        }
        let selected = {
            let state = self.state_mut(vm).ok()?;
            let actual = table(state, resolved.binding.owner)?;
            let raw = *actual.get(&resolved.binding.name)?;
            if !state.var_arena.has_static_refs(raw) {
                return None;
            }
            let frame = match resolved.binding.owner {
                super::VarTableOwner::Frame(level) => {
                    Some(Rc::downgrade(&state.frames.get(level)?.activation))
                }
                super::VarTableOwner::Namespace(super::ROOT_NS) => {
                    Some(Rc::downgrade(&state.frames.first()?.activation))
                }
                _ => None,
            };
            VariableSlot {
                binding: Some(resolved.binding.clone()),
                key: resolved.binding.name.clone(),
                table: actual.weak_birth(),
                frame,
                parent: None,
                raw,
            }
        };
        self.remove_selected(vm, &selected, false, false);
        Some(self.check(vm).is_ok())
    }

    pub(super) fn discard_shell(&self, vm: &mut Vm, parent: VarId, key: &[u8], id: VarId) {
        if self.check(vm).is_err() {
            return;
        }
        let selected = {
            let Ok(state) = self.state_mut(vm) else {
                return;
            };
            let discard = matches!(
                state.var_arena.get(id).map(crate::vars::VarCell::state),
                Some(Local::Undefined)
            ) && !state.var_arena.has_link_refs(id)
                && !state.var_arena.has_operation_refs(id)
                && !state.variable_observers.script_traces.contains_key(&id);
            if !discard {
                return;
            }
            let Some(original_parent) = state.var_arena.weak_jim_cell(parent) else {
                return;
            };
            let Some(Local::Array(elements)) =
                state.var_arena.get(parent).map(crate::vars::VarCell::state)
            else {
                return;
            };
            if elements.get(key) != Some(&id) {
                return;
            }
            VariableSlot {
                binding: None,
                key: key.into(),
                table: elements.weak_birth(),
                frame: None,
                parent: Some(original_parent),
                raw: id,
            }
        };
        // Removing a trace shell has no operation-level search invalidation.
        self.remove_selected(vm, &selected, false, true);
        let _ = self.check(vm);
    }
}
