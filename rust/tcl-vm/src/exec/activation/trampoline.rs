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

//! The NRE trampoline: driving an activation stack tick by tick, pushing frames and installing what a tick requests.

use super::super::{
    DriveMode, Frame, OriginalExecutionOwner, RunExit, ScriptNamespace, Tick, TickAction,
};
use crate::interp::{Vm, err};
use crate::value::Value;
use tcl_runtime_api::{Code, Completion};

impl Vm {
    /// Run one activation stack to completion (the ordinary, non-coroutine path).
    /// The initial frame's `is_proc` decides whether the outermost body is a proc
    /// call (so `unwind` pops a call-frame + namespace and absorbs `return`):
    /// `false` for a module top-level,
    /// `true` for [`invoke_command`](Self::invoke_command) running a proc body.
    ///
    /// The frame is boxed so no copy of it stays in this frame (or the
    /// caller's) while the activation runs: a `TclOO` method call recurses
    /// natively through here (see `OO_DISPATCH_DEPTH_LIMIT`).
    pub(in super::super) fn run_activation(&mut self, initial: Box<Frame>) -> Completion<Value> {
        let mut acts = Vec::new();
        self.push_boxed_frame(&mut acts, initial);
        match self.drive(&mut acts, DriveMode::Plain) {
            RunExit::Done(c) => c,
            // A `yield` reached at top level (outside a coroutine driver) is
            // rejected by the `yield` builtin before it sets `coro.pending`, so
            // `Plain` mode never observes a suspend.
            RunExit::Yielded(_) => unreachable!("Plain-mode drive cannot suspend"),
        }
    }

    /// Drive the NRE trampoline over `acts` until it empties (`RunExit::Done`) or,
    /// in [`DriveMode::CoroDriver`], a `yield`/`yieldto` suspends the coroutine
    /// (`RunExit::Yielded`, `acts` left frozen). Both `run_activation` and a
    /// coroutine's `resume` funnel through here so the trampoline logic
    /// (`enter_proc`/`unwind`/`dispatch_tailcall`/inline-loop redirection) is shared.
    ///
    /// `activation_depth` counts nested `drive` invocations — the host re-entry
    /// counter `yield` consults to reject a suspend across a `catch`/`uplevel`/
    /// `eval`/OO-method boundary (`cannot yield: C stack busy`).
    /// Charge the value the last tick left on top of the running activation's
    /// stack against the value-size limit: a command's result, or what an
    /// inline instruction built — a concatenation, an append, a list. Every
    /// value a body keeps passes through there, so none over the limit
    /// outlives the instruction that built it.
    fn charge_built_value(&mut self, acts: &[Frame]) -> Option<Completion<Value>> {
        let limit = self.value_size_limit_value()?;
        let built = acts.last()?.stack.last()?;
        (built.measured_bytes(limit) > limit)
            .then(|| self.exceed_limit_inner(crate::LimitKind::ValueSize))
    }

    pub(in super::super) fn drive(&mut self, acts: &mut Vec<Frame>, mode: DriveMode) -> RunExit {
        let original = crate::interp::oo_cleanup::OriginalOoCleanup::enter(self);
        let previous_depth = self.activation_depth;
        let current_evaluations = self
            .jim_errors
            .frames
            .iter()
            .map(Self::borrow_jim_evaluation_frame)
            .collect();
        let previous_outer = std::mem::replace(&mut self.jim_errors.outer, current_evaluations);
        self.activation_depth += 1;
        let exit = self.drive_loop(acts, mode);
        let restored = original.in_original(self, |vm| {
            let errors = &mut vm.jim_errors;
            errors.frames = errors
                .outer
                .iter()
                .map(Self::borrow_jim_evaluation_frame)
                .collect();
            vm.jim_errors.outer = previous_outer;
            vm.activation_depth = previous_depth;
        });
        if let Err(error) = restored {
            crate::interp::oo_cleanup::retain_error(self, &error);
        }
        if let Err(first) = original.check(self) {
            crate::interp::oo_cleanup::retain_error(self, &first);
            drop(exit);
            if !acts.is_empty() {
                let retired = original.in_original(self, |vm| vm.abort_refused_activations(acts));
                drop(retired);
            }
            let first = original.check(self).err().unwrap_or(first);
            return RunExit::Done(crate::command::completion_from_cmd_error(self, first));
        }
        exit
    }

    /// Drive a coroutine's activation stack until it suspends (`yield`) or
    /// completes — the `resume` entry point (`cmd_coro`). Runs in
    /// [`DriveMode::CoroDriver`] so a `Tick::Suspend` freezes `acts` and returns
    /// `RunExit::Yielded` instead of erroring.
    pub(crate) fn drive_coro(&mut self, acts: &mut Vec<Frame>) -> RunExit {
        self.drive(acts, DriveMode::CoroDriver)
    }

    pub(in super::super) fn refresh_jim_evaluation_frames(&mut self, acts: &[Frame]) {
        if self.uses_jim_error_stack() {
            let errors = &mut self.jim_errors;
            errors.frames = errors
                .outer
                .iter()
                .map(Self::borrow_jim_evaluation_frame)
                .collect();
            self.jim_errors.frames.extend(
                acts.iter()
                    .map(|frame| Self::borrow_jim_evaluation_frame(&frame.jim_evaluation)),
            );
        }
    }

    #[inline(never)]
    fn push_boxed_frame(&mut self, acts: &mut Vec<Frame>, frame: Box<Frame>) {
        self.push_frame(acts, *frame);
    }

    pub(in super::super) fn push_frame(&mut self, acts: &mut Vec<Frame>, mut frame: Frame) {
        frame.original_target =
            Some(crate::interp::native_operation_currency::SavedExecutionTarget::capture(self));
        if frame.is_proc && frame.oo_call_exit.is_none() {
            frame.procedure_call_exit =
                crate::interp::oo_cleanup::OriginalOoCallExit::capture_procedure(self);
        }
        if self.uses_jim_error_stack() {
            let parent = acts
                .last()
                .map(|frame| &frame.jim_evaluation)
                .or_else(|| self.jim_errors.outer.last());
            frame.jim_evaluation.procedure_level =
                parent.map_or(0, |frame| frame.procedure_level) + u32::from(frame.is_proc);
            self.jim_errors.stack.mark_reset();
        }
        for required in frame.asm.instructions.iter().flat_map(|instruction| {
            instruction
                .native_compiler_selections()
                .map(|site| &site.prerequisite)
        }) {
            if required.guard() == tcl_runtime_api::CommandBindingGuard::ChunkEntry
                && self.native_compiler_selection_prerequisite_matches(required)
            {
                frame
                    .chunk_native_compiler_selections
                    .insert(required.clone());
            }
        }
        for (instruction_index, instruction) in frame.asm.instructions.iter().enumerate() {
            for (site_index, site) in instruction.native_operation_selections.iter().enumerate() {
                if site
                    .compiler_selection_prerequisite()
                    .as_ref()
                    .is_none_or(|required| {
                        required.guard() != tcl_runtime_api::CommandBindingGuard::ChunkEntry
                            || self.native_compiler_selection_prerequisite_matches(required)
                    })
                    && site
                        .requirements
                        .iter()
                        .filter(|binding| {
                            binding.guard == tcl_runtime_api::CommandBindingGuard::ChunkEntry
                        })
                        .all(|binding| self.command_binding_matches(binding))
                {
                    frame
                        .chunk_native_operations
                        .insert((instruction_index, site_index));
                }
            }
        }
        for binding in frame.asm.instructions.iter().filter_map(|instruction| {
            instruction
                .entered_command
                .as_ref()
                .map(|entered| &entered.binding)
        }) {
            if binding.guard != tcl_runtime_api::CommandBindingGuard::ChunkEntry
                || frame.chunk_commands.contains_key(binding)
            {
                continue;
            }
            if let Some(captured) = self.capture_native_command(binding) {
                frame.chunk_commands.insert(binding.clone(), captured);
            }
        }
        if let Some(ctx) = self.pending_exec_leave.take() {
            frame.exec_leave.push(ctx);
        }
        acts.push(frame);
    }

    /// Consume every tick shape at the single activation-install boundary.
    /// Both the ordinary trampoline and a command dispatched by `tailcall` use
    /// this path, so a deferred builtin cannot silently look synchronous in one
    /// of them. Each pushed frame takes the trace context for the command that
    /// created it, and each placeholder unit was stamped when the tick was
    /// issued rather than reconstructed here from later VM state.
    fn install_tick(&mut self, acts: &mut Vec<Frame>, tick: Tick) -> TickAction {
        match tick {
            Tick::Continue => TickAction::Resume,
            Tick::Return(completion) => TickAction::Complete(completion),
            Tick::Call {
                proc,
                invoked,
                argv,
                lambda_registration,
            } => match self.enter_proc(&proc.proc, &proc.body, &invoked, &argv) {
                Ok(()) => {
                    self.install_native_procedure_binding(proc.declaration_binding);
                    let mut frame = Frame::new(proc.body, true);
                    // The C bytecode caller keeps its operand stack until
                    // TclNREvalObjv returns (tclExecute.c doInvocation/cleanupV).
                    // Move those same owning references with this activation.
                    frame.procedure_invocation = Some((invoked, argv));
                    frame.lambda_registration = lambda_registration;
                    self.push_frame(acts, frame);
                    TickAction::Resume
                }
                Err(completion) => {
                    if let Some(registration) = lambda_registration {
                        self.retire_original_lambda_registration(&registration);
                    }
                    TickAction::Complete(self.settle_pending_exec_leave(completion))
                }
            },
            Tick::PushScript {
                script,
                label,
                cleanup_proc,
                fatal_tail,
                namespace,
            } => {
                let mut frame = Frame::new_script(*script, label);
                frame.cleanup_proc = cleanup_proc;
                frame.fatal_tail = fatal_tail.or(frame.fatal_tail);
                if let ScriptNamespace::CommandBoundary(namespace) = namespace {
                    match self.enter_replay_namespace(&namespace) {
                        Ok(previous) => frame.replay_namespace_restore = previous,
                        Err(message) => {
                            if let Some(name) = frame.cleanup_proc.take() {
                                self.take_command_unchecked(&name);
                            }
                            return TickAction::Complete(
                                self.settle_pending_exec_leave(err(message)),
                            );
                        }
                    }
                }
                self.push_frame(acts, frame);
                TickAction::Resume
            }
            Tick::PushEval(req) => {
                self.push_frame(acts, Frame::new_eval(*req));
                TickAction::Resume
            }
            Tick::PushCatch(req) => {
                self.push_frame(acts, Frame::new_catch(*req));
                TickAction::Resume
            }
            Tick::PushSubst { req, placeholder } => {
                self.push_frame(acts, Frame::new_subst(*req, *placeholder));
                TickAction::Resume
            }
            Tick::PushEachLoop { req, placeholder } => {
                self.push_frame(acts, Frame::new_each_loop(*req, *placeholder));
                TickAction::Resume
            }
            Tick::PushExpression { req, placeholder } => {
                self.push_frame(acts, Frame::new_expression(*req, *placeholder));
                TickAction::Resume
            }
            Tick::PushControl { state, placeholder } => {
                self.push_frame(acts, Frame::new_control(*state, *placeholder));
                TickAction::Resume
            }
            Tick::PushTry {
                req,
                initial_options,
            } => {
                self.push_frame(acts, Frame::new_try(*req, initial_options));
                TickAction::Resume
            }
            Tick::Tailcall(request) => {
                TickAction::Complete(match self.schedule_tailcall(request) {
                    Ok(()) => Completion::new(Code::Return, Value::empty(), Value::empty()),
                    Err(completion) => completion,
                })
            }
            Tick::Suspend(req) => TickAction::Suspend(req),
        }
    }

    /// Check every activation's retained execution owners before a tick, and
    /// the exit when that or a stale compiled-local layout refuses execution.
    fn refused_before_tick(&mut self, acts: &mut Vec<Frame>) -> Option<RunExit> {
        if let Some(original) = acts.last().and_then(|frame| frame.original_target.as_ref()) {
            let _ = original.check_current(self);
        }
        for original in acts
            .iter()
            .flat_map(|activation| activation.exec_leave.iter())
            .flat_map(|context| context.original_scopes.iter())
        {
            if matches!(original, OriginalExecutionOwner::Parked(_)) {
                let _ = original.check(self);
                let refused = self.refuse_host_command(
                    "parked VM continuation has no entered execution owner".into(),
                );
                drop(refused);
                break;
            }
            let _ = original.check(self);
        }
        if self.execution_refusal.is_some() {
            return Some(RunExit::Done(self.abort_refused_activations(acts)));
        }
        if acts
            .last()
            .is_some_and(|frame| !self.compiled_local_layout_matches(&frame.asm))
        {
            let _ = self.refuse_host_command(
                "borrowed compiled-local layout is no longer available".into(),
            );
            return Some(RunExit::Done(self.abort_refused_activations(acts)));
        }
        None
    }

    pub(in super::super) fn drive_loop(
        &mut self,
        acts: &mut Vec<Frame>,
        mode: DriveMode,
    ) -> RunExit {
        // The top activation the previous tick's freshness check accepted, when
        // that tick resumed it without running anything else.
        let mut fresh_top: Option<(usize, u64)> = None;
        // The top activation last found fresh, and the world it was found in.
        let mut verified: Option<((usize, u64), crate::interp::ActivationFreshnessWorld)> = None;
        loop {
            let top = acts.last().map(|frame| (acts.len(), frame.serial));
            if let Some(exit) = self.refused_before_tick(acts) {
                return exit;
            }
            // Enforce `interp limit $i time` for unbounded bytecode loops: the
            // counter is Vm-scoped so it survives the short activations a
            // command-driven loop re-enters (see `limit_check_tick`).
            if let Some(c) = self
                .limit_check_tick()
                .or_else(|| self.charge_built_value(acts))
            {
                fresh_top = None;
                let c = match self.publish_and_log_completion(acts, c) {
                    Ok(completion) => completion,
                    Err(completion) => return RunExit::Done(completion),
                };
                if let Some(done) = self.unwind(acts, c) {
                    return RunExit::Done(done);
                }
                continue;
            }
            if fresh_top.take().is_none_or(|fresh| Some(fresh) != top) {
                match self.unwind_stale_compilation_frame(acts, None) {
                    Ok(false) => {
                        verified = top.map(|key| (key, self.activation_freshness_world()));
                    }
                    Ok(true) => continue,
                    Err(exit) => return exit,
                }
            }
            if let Some(completion) = self.native_script_parse_failure(acts) {
                if let Some(done) = self.settle_completion(acts, completion) {
                    return RunExit::Done(done);
                }
                continue;
            }
            self.refresh_jim_evaluation_frames(acts);
            self.pending.bottom_activation = acts.len() == 1 && self.activation_depth == 1;
            let mut tick = {
                let top = acts.last_mut().expect("activation stack is non-empty");
                self.tick(top)
            };
            if self.execution_refusal.is_some() {
                self.restore_execution_frame(tick.take_selected_frame_restore());
                return RunExit::Done(self.abort_refused_activations(acts));
            }
            // Uplevel has selected its variable frame, while its child
            // activation still belongs to this tick. The parent consumes the
            // namespace retained by that selection, not the child's namespace.
            let parent_namespace = tick
                .selected_frame_restore()
                .and_then(crate::interp::SelectedFrameRestore::original_namespace);
            if parent_namespace.as_ref().is_some_and(|namespace| {
                self.top_activation_stale_message_in_context(acts, namespace)
                    .is_some()
            }) {
                // A failed handoff must restore the selected physical frame
                // before the parent's catch/unwind consumes the failure.
                self.restore_execution_frame(tick.take_selected_frame_restore());
            }
            // An instruction that changes nothing the freshness query reads
            // leaves the frame as fresh as it was when last checked.
            let unchanged = parent_namespace.is_none()
                && verified.as_ref().is_some_and(|(key, world)| {
                    Some(*key) == top && *world == self.activation_freshness_world()
                });
            if !unchanged || crate::VERIFY_CURRENCY {
                match self.unwind_stale_compilation_frame(acts, parent_namespace.as_ref()) {
                    Ok(false) => {
                        if parent_namespace.is_none() {
                            verified = top.map(|key| (key, self.activation_freshness_world()));
                        }
                    }
                    Ok(true) => {
                        debug_assert!(!unchanged, "an unchanged world found a fresh frame stale");
                        continue;
                    }
                    Err(exit) => {
                        debug_assert!(!unchanged, "an unchanged world found a fresh frame stale");
                        return exit;
                    }
                }
            }
            let checked_top = parent_namespace
                .is_none()
                .then(|| acts.last().map(|frame| (acts.len(), frame.serial)))
                .flatten();
            match self.install_tick(acts, tick) {
                TickAction::Resume => fresh_top = checked_top,
                TickAction::Complete(completion) => {
                    if let Some(done) = self.settle_completion(acts, completion) {
                        return RunExit::Done(done);
                    }
                }
                TickAction::Suspend(req) => {
                    if let Some(exit) = self.handle_suspend(acts, mode, req) {
                        return exit;
                    }
                }
            }
        }
    }

    fn native_script_parse_failure(&mut self, acts: &mut [Frame]) -> Option<Completion<Value>> {
        if acts
            .last()
            .is_none_or(|frame| frame.pc != 0 || frame.fatal_tail.is_none())
            || self.native_invocation_dialect().script_parse_timing()
                != Some(tcl_registry::invocation_words::NativeScriptParseTiming::BeforeScript)
        {
            return None;
        }
        let frame = acts.last_mut()?;
        let tail = frame.fatal_tail.take()?;
        let level = frame.jim_evaluation.procedure_level;
        let location = frame.source_location.clone();
        self.refresh_jim_evaluation_frames(&acts[..acts.len() - 1]);
        Some(self.raise_jim_script_parse_failure(tail, level, location))
    }
}
