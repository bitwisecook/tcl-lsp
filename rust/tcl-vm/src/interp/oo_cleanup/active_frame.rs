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

//! Actual physical frame and namespace retirement; weak observations own no native roles.

use super::{CmdError, OriginalOoCleanup, retain_error};
use crate::cmd_oo::{OoFrame, native_context::OriginalOoCleanupFrame};
use crate::frame::ActivationIdentity;
use crate::interp::Vm;
use crate::interp::namespace_stack::NamespaceActivationObservation;
use std::rc::{Rc, Weak};

#[derive(Clone)]
struct CallFrameObservation {
    position: usize,
    activation: Weak<ActivationIdentity>,
    caller: Weak<ActivationIdentity>,
}
impl CallFrameObservation {
    fn capture(vm: &Vm) -> Option<Self> {
        let position = vm.frames.len().checked_sub(1)?;
        if position == 0 {
            return None;
        }
        Some(Self {
            position,
            activation: Rc::downgrade(&vm.frames[position].activation),
            caller: Rc::downgrade(&vm.frames[position - 1].activation),
        })
    }
    fn present(&self, state: &crate::interp::InterpState) -> bool {
        state
            .frames
            .get(self.position)
            .is_some_and(|frame| Weak::ptr_eq(&self.activation, &Rc::downgrade(&frame.activation)))
    }
    fn caller_restored(&self, state: &crate::interp::InterpState) -> bool {
        state.frames.len() == self.position
            && state
                .frames
                .last()
                .is_some_and(|frame| Weak::ptr_eq(&self.caller, &Rc::downgrade(&frame.activation)))
    }
}
impl OriginalOoCleanup {
    pub(crate) fn pop_call_frame(&self, vm: &mut Vm) -> Result<(), CmdError> {
        let selected = self.in_original(vm, |vm| CallFrameObservation::capture(vm))?;
        if let Some(selected) = selected {
            self.pop_selected_call_frame(vm, &selected, false)?;
        }
        self.check_cleanup(vm)
    }
    fn pop_selected_call_frame(
        &self,
        vm: &mut Vm,
        selected: &CallFrameObservation,
        completed: bool,
    ) -> Result<(), CmdError> {
        if let Err(error) = self.check(vm) {
            retain_error(vm, &error);
        }
        let state = self.state_mut(vm)?;
        if completed && selected.caller_restored(state) {
            return self.check_cleanup(vm);
        }
        if !selected.present(state) || state.frames.len() != selected.position + 1 {
            return Err(self.refuse(vm, "original OO procedure cleanup activation"));
        }
        // Native execution ownership leaves before the actual proc frame, as in the existing owner.
        let execution = state.frames[selected.position]
            .native_procedure_execution
            .take();
        drop(execution);
        if let Err(error) = self.check(vm) {
            retain_error(vm, &error);
        }
        let state = self.state_mut(vm)?;
        if !selected.present(state) || state.frames.len() != selected.position + 1 {
            return Err(self.refuse(vm, "original OO procedure cleanup activation"));
        }
        let frame = state.frames.pop().expect("selected non-global frame");
        state.recursion_depth = state.recursion_depth.saturating_sub(1);
        let namespace = (state.resolution_stacks.ns_stack.len() > state.frames.len())
            .then(|| state.resolution_stacks.observe_activation())
            .flatten();
        let departing = namespace
            .as_ref()
            .and_then(|original| state.resolution_stacks.take_original_activation(original));
        let destroyed = self.in_original(vm, |vm| vm.destroy_call_frame(frame, self));
        if let Err(error) = destroyed {
            retain_error(vm, &error);
        }
        if let Some(departing) = departing {
            let restored = self.state_mut(vm).map(|state| {
                if namespace
                    .as_ref()
                    .is_some_and(|old| old.caller_restored(&state.resolution_stacks))
                {
                    state.resolution_stacks.restore_activation(departing);
                    None
                } else {
                    Some(departing.namespace)
                }
            });
            match restored {
                Ok(None) => {}
                Ok(Some(namespace)) => {
                    let error = self.refuse(vm, "original departing namespace activation");
                    retain_error(vm, &error);
                    let retired = self.in_original(vm, |vm| {
                        if let super::super::NamespaceExecutionContext::Resolved(token) = namespace
                        {
                            vm.retire_namespace_activation_role(token);
                        }
                    });
                    if let Err(error) = retired {
                        retain_error(vm, &error);
                    }
                }
                Err(error) => retain_error(vm, &error),
            }
        }
        self.check_cleanup(vm)
    }
    fn retire_selected_namespace(
        &self,
        vm: &mut Vm,
        namespace: &NamespaceActivationObservation,
        completed: bool,
    ) {
        if let Err(error) = self.check(vm) {
            retain_error(vm, &error);
        }
        let retired = self.state_mut(vm).map(|state| {
            if completed && namespace.caller_restored(&state.resolution_stacks) {
                return Some((None, true));
            }
            let last = state.resolution_stacks.ns_stack.len() == namespace.position + 1;
            state
                .resolution_stacks
                .take_original_activation(namespace)
                .map(|entry| (Some(entry), last))
        });
        match retired {
            Ok(Some((entry, last))) => {
                if let Some(entry) = entry {
                    let result = self.in_original(vm, |vm| {
                        if let super::super::NamespaceExecutionContext::Resolved(token) =
                            entry.namespace
                        {
                            vm.retire_namespace_activation_role(token);
                        }
                    });
                    if let Err(error) = result {
                        retain_error(vm, &error);
                    }
                }
                if !last {
                    let error = self.refuse(vm, "original OO namespace cleanup stack");
                    retain_error(vm, &error);
                }
            }
            Ok(None) => {
                let error = self.refuse(vm, "original OO namespace cleanup activation");
                retain_error(vm, &error);
            }
            Err(error) => retain_error(vm, &error),
        }
        if let Err(error) = self.check_cleanup(vm) {
            retain_error(vm, &error);
        }
    }
    pub(crate) fn capture_dispatch_depth(&self, vm: &mut Vm) -> Result<usize, CmdError> {
        Ok(self.state_mut(vm)?.oo_dispatch_depth)
    }
    pub(crate) fn restore_dispatch_depth(&self, vm: &mut Vm, prior: usize) -> Result<(), CmdError> {
        let state = self.state_mut(vm)?;
        let balanced = state.oo_dispatch_depth == prior + 1;
        state.oo_dispatch_depth = prior;
        if !balanced {
            return Err(self.refuse(vm, "original OO dispatch depth"));
        }
        self.check_cleanup(vm)
    }
}

/// Exact weak cleanup observations transported by the actual OO procedure Frame.
/// No entered invocation or Native payload role is retained while stored.
#[derive(Clone)]
pub(crate) struct OriginalOoCallExit {
    original: super::SavedOriginalOoCleanup,
    call: CallFrameObservation,
    namespace: NamespaceActivationObservation,
    method: Option<OriginalOoCleanupFrame>,
    /// A plain procedure has no selected OO Method purpose.
    procedure_only: bool,
}
impl OriginalOoCallExit {
    /// Capture exact call/namespace births at the actual procedure body entry.
    /// Saved metadata holds no entered Scope or additional Native object role.
    pub(crate) fn capture_procedure(vm: &mut Vm) -> Option<Self> {
        let original = OriginalOoCleanup::observe(vm);
        let call = CallFrameObservation::capture(vm);
        let namespace = vm.resolution_stacks.observe_activation();
        if let Err(error) = original.check(vm) {
            retain_error(vm, &error);
        }
        match (call, namespace) {
            (Some(call), Some(namespace)) => Some(Self {
                original: original.saved(),
                call,
                namespace,
                method: None,
                procedure_only: true,
            }),
            _ => {
                let error = original.refuse(vm, "original procedure entry activation");
                retain_error(vm, &error);
                None
            }
        }
    }

    pub(crate) fn in_original<T>(
        &self,
        vm: &mut Vm,
        work: impl FnOnce(&mut Vm, &OriginalOoCleanup) -> T,
    ) -> Result<T, CmdError> {
        let original = self.original.attach(vm);
        let reached = original.in_original(vm, |vm| work(vm, &original));
        if let Err(first) = original.check(vm) {
            retain_error(vm, &first);
            drop(reached);
            return Err(original.check(vm).err().unwrap_or(first));
        }
        reached
    }
    pub(crate) fn check_running(
        &self,
        vm: &mut Vm,
        original: &OriginalOoCleanup,
    ) -> Result<(), CmdError> {
        original.check(vm)?;
        let state = original.state_mut(vm)?;
        let method = self.procedure_only
            || self.method.as_ref().is_some_and(|selected| {
                state.oo.call_stack.len() == selected.position() + 1
                    && state
                        .oo
                        .call_stack
                        .get(selected.position())
                        .is_some_and(|frame| selected.matches_description(frame))
            });
        if !method
            || !self.call.present(state)
            || state.frames.len() != self.call.position + 1
            || !self.namespace.original_present(&state.resolution_stacks)
            || state.resolution_stacks.ns_stack.len() != self.namespace.position + 1
        {
            return Err(original.refuse(vm, "original OO procedure epilogue activation"));
        }
        Ok(())
    }
    pub(crate) fn retire(&self, vm: &mut Vm, original: &OriginalOoCleanup) {
        if let Err(error) = original.pop_selected_call_frame(vm, &self.call, true) {
            retain_error(vm, &error);
        }
        original.retire_selected_namespace(vm, &self.namespace, true);
    }
}

/// The pushed descriptive OO context and optional real call/namespace activations.
/// No strong Method, Namespace, activation or additional native object role is retained.
pub(crate) struct OriginalOoActivation {
    original: OriginalOoCleanup,
    method: Option<OriginalOoCleanupFrame>,
    call: Option<CallFrameObservation>,
    namespace: Option<NamespaceActivationObservation>,
}
impl OriginalOoActivation {
    pub(crate) fn enter(vm: &mut Vm) -> Self {
        Self {
            original: OriginalOoCleanup::enter(vm),
            method: None,
            call: None,
            namespace: None,
        }
    }
    pub(crate) fn capture_call_and_namespace(&mut self, vm: &mut Vm) -> Result<(), CmdError> {
        self.call = CallFrameObservation::capture(vm);
        self.capture_namespace(vm)
    }
    pub(crate) fn capture_namespace(&mut self, vm: &mut Vm) -> Result<(), CmdError> {
        self.namespace = vm.resolution_stacks.observe_activation();
        self.original.check(vm)?;
        if self.namespace.is_none() {
            return Err(self.original.refuse(vm, "original OO namespace activation"));
        }
        Ok(())
    }
    /// The existing actual physical cleanup owner, independently of Method purpose.
    pub(crate) fn cleanup(&self) -> &OriginalOoCleanup {
        &self.original
    }
    /// Require the same genuinely pushed call and namespace activations.
    pub(crate) fn check_call_and_namespace(&self, vm: &mut Vm) -> Result<(), CmdError> {
        self.original.check(vm)?;
        let state = self.original.state_mut(vm)?;
        let current = self
            .call
            .as_ref()
            .is_some_and(|call| call.present(state) && state.frames.len() == call.position + 1)
            && self.namespace.as_ref().is_some_and(|namespace| {
                namespace.original_present(&state.resolution_stacks)
                    && state.resolution_stacks.ns_stack.len() == namespace.position + 1
            });
        if !current {
            return Err(self
                .original
                .refuse(vm, "original OO definition activation"));
        }
        Ok(())
    }
    pub(crate) fn saved_call_exit(&self) -> Option<OriginalOoCallExit> {
        Some(OriginalOoCallExit {
            original: self.original.saved(),
            call: self.call.as_ref()?.clone(),
            namespace: self.namespace.as_ref()?.clone(),
            method: self.method.clone(),
            procedure_only: false,
        })
    }
    pub(crate) fn push_method(&mut self, vm: &mut Vm, frame: OoFrame) -> Result<(), CmdError> {
        self.original.check(vm)?;
        let selected = OriginalOoCleanupFrame::capture(vm.oo.call_stack.len(), &frame);
        vm.oo.call_stack.push(frame);
        self.method = Some(selected);
        Ok(())
    }
    pub(crate) fn retire_method(&mut self, vm: &mut Vm) {
        if let Err(error) = self.original.check(vm) {
            retain_error(vm, &error);
        }
        let Some(selected) = self.method.take() else {
            return;
        };
        let position = selected.position();
        let description = self.original.oo_mut(vm).map(|oo| {
            oo.call_stack
                .get(position)
                .is_some_and(|frame| selected.matches_description(frame))
        });
        if matches!(description, Ok(false)) {
            let error = self
                .original
                .refuse(vm, "original OO cleanup frame description");
            retain_error(vm, &error);
        }
        let retired = self.original.oo_mut(vm).map(|oo| {
            if oo
                .call_stack
                .get(position)
                .is_some_and(|frame| selected.matches_birth(frame))
            {
                let last = position + 1 == oo.call_stack.len();
                Some((oo.call_stack.remove(position), last))
            } else {
                None
            }
        });
        match retired {
            Ok(Some((frame, last))) => {
                drop(frame);
                if let Err(error) = self.original.check(vm) {
                    retain_error(vm, &error);
                }
                if !last {
                    let error = self.original.refuse(vm, "original OO method cleanup stack");
                    retain_error(vm, &error);
                }
            }
            Ok(None) => {
                let error = self
                    .original
                    .refuse(vm, "original OO method cleanup activation");
                retain_error(vm, &error);
            }
            Err(error) => retain_error(vm, &error),
        }
        if let Err(error) = self.original.check_cleanup(vm) {
            retain_error(vm, &error);
        }
    }
    pub(crate) fn retire_call(&mut self, vm: &mut Vm, completed: bool) {
        if let Some(call) = self.call.take()
            && let Err(error) = self.original.pop_selected_call_frame(vm, &call, completed)
        {
            retain_error(vm, &error);
        }
    }
    pub(crate) fn retire_namespace(&mut self, vm: &mut Vm, completed: bool) {
        if let Some(namespace) = self.namespace.take() {
            self.original
                .retire_selected_namespace(vm, &namespace, completed);
        }
    }
    pub(crate) fn finish(
        self,
        vm: &mut Vm,
        result: tcl_runtime_api::Completion<crate::value::Value>,
    ) -> tcl_runtime_api::Completion<crate::value::Value> {
        self.original.finish_completion(vm, result)
    }
}

#[cfg(test)]
mod tests;
