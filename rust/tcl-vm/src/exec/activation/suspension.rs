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

//! Suspending and resuming an activation stack: saved original currencies, coroutine park, resume and retirement, and aborting refused activations.

use super::super::{DriveMode, Frame, RunExit, Tick, YieldReq};
use crate::interp::native_operation_currency::{NativeOperationCurrency, NativeOperationScope};
use crate::interp::{Vm, err};
use crate::value::Value;
use tcl_runtime_api::Completion;

impl Vm {
    pub(crate) fn original_activation_currencies(acts: &[Frame]) -> Vec<NativeOperationCurrency> {
        acts.iter()
            .flat_map(|activation| {
                activation
                    .exec_leave
                    .iter()
                    .flat_map(|context| context.original_scopes.iter())
                    .chain(activation.deferred_dispatch.iter().filter_map(
                        |tick| match tick.as_ref() {
                            Tick::Tailcall(request) => Some(&request.original_scope),
                            _ => None,
                        },
                    ))
            })
            .map(|owner| owner.currency().clone())
            .collect()
    }

    /// Attach saved currency solely for retirement of its authentic current target.
    pub(crate) fn original_activation_cleanup(
        &mut self,
        currencies: &[NativeOperationCurrency],
    ) -> Vec<NativeOperationScope> {
        currencies
            .iter()
            .filter_map(|currency| NativeOperationScope::cleanup_saved(self, currency).ok())
            .collect()
    }

    /// Restore the genuine suspended owners before any resume callback or opcode.
    pub(crate) fn resume_original_activations(
        &mut self,
        acts: &mut [Frame],
    ) -> Result<Vec<NativeOperationScope>, tcl_cmd_core::CmdError> {
        let mut scopes = Vec::new();
        for activation in acts {
            for owner in activation
                .exec_leave
                .iter_mut()
                .flat_map(|context| &mut context.original_scopes)
            {
                scopes.push(owner.resume(self)?);
            }
            if let Some(tick) = &mut activation.deferred_dispatch
                && let Tick::Tailcall(request) = tick.as_mut()
            {
                scopes.push(request.original_scope.resume(self)?);
            }
        }
        scopes.extend(self.resume_original_flow_tailcalls()?);
        Ok(scopes)
    }

    pub(crate) fn resume_original_currencies(
        &mut self,
        currencies: &[NativeOperationCurrency],
    ) -> Result<Vec<NativeOperationScope>, tcl_runtime_api::NativeExecutionError> {
        currencies
            .iter()
            .map(|currency| NativeOperationScope::resume_saved(self, currency))
            .collect()
    }

    pub(crate) fn check_original_scopes_native(
        &mut self,
        scopes: &[NativeOperationScope],
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        if let Some(first) = self.execution_refusal.clone() {
            return Err(first);
        }
        for scope in scopes {
            scope.check_native(self)?;
        }
        Ok(())
    }

    pub(crate) fn check_original_scopes(
        &mut self,
        scopes: &[NativeOperationScope],
    ) -> Result<(), tcl_cmd_core::CmdError> {
        self.check_original_scopes_native(scopes)
            .map_err(tcl_cmd_core::CmdError::from_execution_refusal)
    }

    /// Restore hidden frame owners before a parked coroutine is destroyed.
    /// Walk innermost first so nested selections retire only their own children.
    pub(crate) fn restore_parked_execution_frames(&mut self, acts: &mut [Frame]) {
        for activation in acts.iter_mut().rev() {
            self.restore_execution_frame(activation.selected_frame_restore.take());
        }
    }

    /// Retire temporary lambda publications after the parked locals have unwound.
    pub(crate) fn retire_parked_lambda_registrations(&mut self, acts: &mut [Frame]) {
        for activation in acts.iter_mut().rev() {
            if let Some(registration) = activation.lambda_registration.take() {
                self.retire_original_lambda_registration(&registration);
            }
        }
    }

    /// Retire only the actual publication, preserving a later same-name replacement.
    pub(in super::super) fn retire_original_lambda_registration(
        &mut self,
        registration: &crate::interp::CommandSidecarHandle,
    ) {
        if let Some(key) = registration.key() {
            self.retire_command_lifecycle_key(&key);
        }
    }

    /// Retire engine storage without catch/try/finally, completion presentation,
    /// or guest leave/unset callbacks. The returned carrier is internal only;
    /// the host boundary returns the retained typed refusal instead.
    pub(crate) fn abort_refused_activations(&mut self, acts: &mut Vec<Frame>) -> Completion<Value> {
        self.abort_refused_activations_using(acts, None)
    }

    pub(in super::super) fn abort_refused_activations_using(
        &mut self,
        acts: &mut Vec<Frame>,
        saved: Option<&crate::interp::native_operation_currency::SavedExecutionTarget>,
    ) -> Completion<Value> {
        // Capture only already-issued frame/continuation metadata before consuming
        // the last activation. The callback-selected current target is not a donor.
        let target = saved.cloned().or_else(|| {
            acts.last()
                .and_then(|activation| activation.original_target.clone())
        });
        let original_scopes: Vec<_> = acts
            .iter()
            .flat_map(|activation| activation.exec_leave.iter())
            .flat_map(|context| context.original_scopes.iter().cloned())
            .collect();
        for original in &original_scopes {
            let _ = original.check(self);
        }
        while let Some(mut activation) = acts.pop() {
            // A require whose loader is refused still retires its loading
            // marker, as the synchronous require's cleanup did.
            if let Some((name, version)) = activation
                .package
                .as_ref()
                .and_then(|suspended| suspended.loading_marker())
            {
                self.end_abandoned_package_loading(name, version);
            }
            let original = activation.original_target.take();
            if let Some(original) = original {
                let retired = original.in_original(self, |vm| {
                    activation.close_native_script();
                    if activation.is_proc {
                        if let Some(exit) = activation
                            .oo_call_exit
                            .as_ref()
                            .or(activation.procedure_call_exit.as_ref())
                            && let Err(error) =
                                exit.in_original(vm, |vm, original| exit.retire(vm, original))
                        {
                            crate::interp::oo_cleanup::retain_error(vm, &error);
                        }
                    } else if let Some(previous) = activation.replay_namespace_restore.take() {
                        vm.leave_replay_namespace(previous);
                    }
                    vm.restore_execution_frame(activation.selected_frame_restore.take());
                    for context in activation.exec_leave.drain(..) {
                        vm.pop_exec_step_scopes(context.step_scopes);
                        drop(context);
                    }
                    if let Some(name) = activation.cleanup_proc.take() {
                        let retired = vm.take_command_unchecked(&name);
                        drop(retired);
                    }
                    if let Some(registration) = activation.lambda_registration.take() {
                        vm.retire_original_lambda_registration(&registration);
                        drop(registration);
                    }
                    // Actual owned operands/results retire before the saved target's
                    // post-cleanup and post-restoration full first-cause checks.
                    drop(activation);
                });
                if let Err(first) = retired {
                    self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(first));
                }
            } else {
                // A hand-built activation without an original physical observation
                // supplies no authority to pop any callback-created frame or namespace.
                let error: tcl_cmd_core::CmdError =
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original activation cleanup target",
                    )
                    .into();
                crate::interp::oo_cleanup::retain_error(self, &error);
                drop(activation);
            }
        }
        if let Some(target) = target {
            let retired = target.in_original(self, |vm| {
                if let Some(req) = vm.pending.procedure_call.take() {
                    vm.retire_original_lambda_registration(&req.registration);
                    drop(req);
                }
                let pending = std::mem::take(&mut vm.pending);
                drop(pending);
                if let Some(context) = vm.pending_exec_leave.take() {
                    vm.pop_exec_step_scopes(context.step_scopes);
                    drop(context);
                }
            });
            if let Err(first) = retired {
                self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(first));
            }
        }
        for original in &original_scopes {
            let _ = original.check(self);
        }
        err("")
    }

    pub(super) fn handle_suspend(
        &mut self,
        acts: &mut Vec<Frame>,
        mode: DriveMode,
        req: YieldReq,
    ) -> Option<RunExit> {
        let original_scopes: Vec<_> = acts
            .iter()
            .flat_map(|activation| activation.exec_leave.iter())
            .chain(self.pending_exec_leave.iter())
            .flat_map(|context| context.original_scopes.iter().cloned())
            .collect();
        for original in &original_scopes {
            let _ = original.check(self);
        }
        if self.execution_refusal.is_some() {
            return Some(RunExit::Done(self.abort_refused_activations(acts)));
        }
        match mode {
            // A coroutine `yield`/`yieldto` freezes `acts` in place (pc already
            // past the suspend point) and hands the request out to its resume.
            DriveMode::CoroDriver => {
                if let Some(message) = self.current_activation_stack_stale_message(acts) {
                    // A freshly compiled try handler/finally can sit above an
                    // activation made stale by the command that selected it.
                    // Never park that mixed stack.
                    return self
                        .settle_stale_activation(acts, message)
                        .map(RunExit::Done);
                }
                if let Some(context) = self.pending_exec_leave.take() {
                    let Some(activation) = acts.last_mut() else {
                        self.pending_exec_leave = Some(context);
                        let refused = self.refuse_host_command(
                            "suspended VM continuation has no activation owner".into(),
                        );
                        drop(refused);
                        return Some(RunExit::Done(self.abort_refused_activations(acts)));
                    };
                    activation.exec_leave.push(context);
                }
                for activation in acts.iter_mut() {
                    for owner in activation
                        .exec_leave
                        .iter_mut()
                        .flat_map(|context| &mut context.original_scopes)
                    {
                        owner.park();
                    }
                    if let Some(tick) = &mut activation.deferred_dispatch
                        && let Tick::Tailcall(request) = tick.as_mut()
                    {
                        request.original_scope.park();
                    }
                }
                self.park_original_flow_tailcalls();
                // The temporary check views must also retire before the parked flow escapes.
                drop(original_scopes);
                Some(RunExit::Yielded(req))
            }
            // Defensive: the `yield` builtin's boundary check rejects a
            // top-level suspend (`cannot yield: C stack busy`) before here.
            DriveMode::Plain => self
                .unwind(acts, err("cannot yield: C stack busy"))
                .map(RunExit::Done),
        }
    }

    /// Reject a frozen coroutine through the ordinary settlement/unwind path.
    /// Plain drive mode turns any attempted yield during cleanup into another
    /// unwinding error, so a stale continuation can never become suspended
    /// again.
    /// Raise `c` at a resumed coroutine's suspend point — the completion of
    /// the `yield` a failing `coroinject` command ran under — then drive the
    /// coroutine on from wherever its catch ranges and loops take it.
    pub(crate) fn raise_in_coroutine(
        &mut self,
        acts: &mut Vec<Frame>,
        c: Completion<Value>,
    ) -> RunExit {
        match self.settle_completion(acts, c) {
            Some(done) => RunExit::Done(done),
            None => self.drive_coro(acts),
        }
    }

    /// Tear down a coroutine's frozen continuation without running it (no
    /// `finally` runs, as in C): one that deleted its own command while it ran
    /// has nowhere to park. Its call frames are destroyed, firing their
    /// locals' `unset` traces (coroutine-4.3), and its lambda registrations
    /// retire with it.
    /// A require suspended in its loader inside a coroutine that is going
    /// away retires that loader's marker (upstream's `abandon_require`).
    pub(crate) fn abandon_parked_requires(&mut self, acts: &[Frame]) {
        for frame in acts {
            if let Some((name, version)) = frame
                .package
                .as_ref()
                .and_then(|suspended| suspended.loading_marker())
            {
                self.abandon_package_loading(name, version);
            }
        }
    }

    pub(crate) fn retire_parked_coroutine(
        &mut self,
        acts: &mut Vec<Frame>,
        parked: &mut crate::interp::ParkedFlow,
    ) {
        let _ = self.fire_parked_unset_traces(parked, acts);
        self.retire_parked_lambda_registrations(acts);
        self.abandon_parked_requires(acts);
        acts.clear();
    }
}
