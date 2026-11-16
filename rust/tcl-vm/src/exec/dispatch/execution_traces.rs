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

//! Execution traces around a dispatch: trace owners and snapshots, step scopes, the traced dispatch path and the leave side.

use super::super::{
    EnteredCommand, ExecLeaveCtx, ExecStepScope, Frame, HeldResult, OriginalExecutionOwner,
    SelectedNativeExecutionTraces, Tick,
};
use crate::interp::native_operation_currency::NativeOperationScope;
use crate::interp::{CmdTraceEntry, CommandSidecarHandle, CommandSidecarKey, Vm, ok};
use crate::value::Value;
use std::rc::Rc;
use tcl_runtime_api::{Code, Completion};

impl Vm {
    /// Resolve the trace owner and take the own-trace snapshot for one invoke.
    /// A retained specialised entry had no execution trace at command entry, so
    /// traces added by its arguments are deliberately absent from the snapshot.
    fn dispatch_trace_owner(
        &mut self,
        original: &Value,
        entered: Option<&EnteredCommand>,
        context: &crate::interp::namespace_context::NamespaceExecutionContext,
    ) -> Result<(CommandSidecarHandle, Vec<Rc<CmdTraceEntry>>), Completion<Value>> {
        let sidecar = if let Some(entered) = entered {
            entered.sidecar.clone()
        } else {
            let key = self
                .resolve_original_command_key_in_context(context, original)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?
                .unwrap_or_default();
            let key = self.renamed_command_key(CommandSidecarKey::visible(key));
            self.active_sidecar(key)
        };
        let own = if entered.is_some_and(|entered| {
            matches!(
                entered.execution_traces,
                SelectedNativeExecutionTraces::Omitted
            )
        }) {
            Vec::new()
        } else {
            sidecar
                .key()
                .and_then(|key| self.exec_traces.get(&key).cloned())
                .unwrap_or_default()
        };
        Ok((sidecar, own))
    }

    /// Validate the command owner and snapshot the step-capable traces which
    /// own its body immediately after all `enter` callbacks have completed and
    /// immediately before dispatch.
    ///
    /// C Tcl lets an `enter` callback add a `leave`/step trace for this same
    /// invocation. Step scopes are fixed at this point; leave traces are looked
    /// up live at settlement, but only when this validated owner already had an
    /// execution trace at entry. Generic dispatch also re-resolves the invoked
    /// spelling after the callbacks: if a callback renamed, deleted, or
    /// replaced the entry-time owner, its traces do not own the different
    /// command selected by that re-resolution. An already-resolved invocation
    /// passes no name and continues to own its captured command through
    /// relocation.
    pub(super) fn post_enter_trace_snapshot(
        &self,
        generic_name: Option<&str>,
        lookup_namespace: Option<&str>,
        sidecar: &CommandSidecarHandle,
        selected_traces: Option<SelectedNativeExecutionTraces>,
    ) -> Option<Vec<Rc<CmdTraceEntry>>> {
        // A specialised entered token had no execution trace before argument
        // substitution. A trace added by an argument belongs only to a later
        // invocation, even though this helper runs immediately before dispatch.
        if matches!(
            selected_traces,
            Some(SelectedNativeExecutionTraces::Omitted)
        ) {
            return None;
        }
        let owner = sidecar.key()?;
        if let Some(name) = generic_name {
            let resolved = self
                .resolve_command_fqn(lookup_namespace.unwrap_or(self.current_ns()), name)
                .map(CommandSidecarKey::visible)
                .map(|key| self.renamed_command_key(key));
            if resolved.as_ref() != Some(&owner) {
                return None;
            }
        }
        Some(self.exec_traces.get(&owner).cloned().unwrap_or_default())
    }

    /// Install the step scopes from the post-enter body-owner snapshot.
    pub(super) fn push_exec_step_scopes(
        &mut self,
        sidecar: &CommandSidecarHandle,
        own: &[Rc<CmdTraceEntry>],
    ) -> usize {
        let was_disabled = self.step_trace_active();
        let mut pushed = 0usize;
        for entry in own.iter().rev() {
            if !sidecar.is_attached() {
                break;
            }
            if !self.exec_trace_entry_live(sidecar, entry) {
                continue;
            }
            if (entry.has_op("enterstep") || entry.has_op("leavestep"))
                && !self
                    .exec_step_scopes
                    .iter()
                    .any(|scope| Rc::ptr_eq(&scope.entry, entry) && !scope.entry.untraced())
            {
                self.exec_step_scopes.push(ExecStepScope {
                    entry: Rc::clone(entry),
                    key: sidecar.clone(),
                });
                pushed += 1;
            }
        }
        self.publish_native_inline_trace_transition(was_disabled);
        pushed
    }

    pub(in super::super) fn pop_exec_step_scopes(&mut self, count: usize) {
        let was_disabled = self.step_trace_active();
        for _ in 0..count {
            self.exec_step_scopes.pop();
        }
        self.publish_native_inline_trace_transition(was_disabled);
    }

    /// Whether the `enterstep`/`leavestep` machinery may fire right now. This
    /// is C's one read of `INTERP_TRACE_IN_PROGRESS` — `TclCheckInterpTraces`
    /// (`tclTrace.c` 9.0.4:1426) returns immediately while an execution
    /// callback is running, so the callback's own commands are never
    /// step-observed. Scopes are still *pushed* while one runs, as C still
    /// installs its interp trace; only the firing is gated.
    fn step_scopes_can_fire(&self) -> bool {
        !self.exec_step_scopes.is_empty() && !self.trace_in_progress.get()
    }

    /// The step scopes to fire for this command — empty while an execution
    /// callback is running (see [`Self::step_scopes_can_fire`]).
    pub(super) fn step_scopes_to_fire(&self) -> Vec<ExecStepScope> {
        if self.step_scopes_can_fire() {
            self.exec_step_scopes.clone()
        } else {
            Vec::new()
        }
    }

    /// Whether this dispatch needs the traced path at all. Being inside a trace
    /// callback is *not* a reason to skip it: C's `TclCheckExecutionTraces`
    /// (:1301) never consults the flag, so a command a callback dispatches
    /// still fires its own `enter`/`leave` traces — only the step machinery is
    /// gated. Re-entering the *same* trace is bounded per trace instead, by
    /// `CmdTraceEntry::firing` in [`Vm::run_cmd_trace_callback`].
    pub(super) fn exec_traces_can_fire(&self) -> bool {
        !self.exec_traces.is_empty() || self.step_scopes_can_fire()
    }

    /// The traced dispatch path (M16.3): fire `enterstep` for every active
    /// step scope and `enter` for the command's own execution traces (a trace
    /// error aborts the command with that error — tclsh-pinned), push the
    /// command's own step scopes, dispatch, then settle the leave side — for
    /// synchronous completions here, for deferred bodies when their frame
    /// unwinds (the context rides on the frame via `pending_exec_leave`).
    pub(super) fn settle_traced_word_dispatch(
        &mut self,
        f: &mut Frame,
        mut ctx: ExecLeaveCtx,
        dispatched: Result<Option<Tick>, Completion<Value>>,
    ) -> Result<Option<Tick>, Completion<Value>> {
        match dispatched {
            Ok(Some(tick)) => {
                match &tick {
                    // The body runs on a pushed frame: the context rides
                    // along and settles when that frame completes.
                    Tick::Call { .. }
                    | Tick::PushScript { .. }
                    | Tick::PushCatch(_)
                    | Tick::PushEval(_)
                    | Tick::PushSubst { .. }
                    | Tick::PushEachLoop { .. }
                    | Tick::PushControl { .. }
                    | Tick::PushExpression { .. }
                    | Tick::PushTry { .. } => {
                        if let Some(mut prior) = self.pending_exec_leave.take() {
                            ctx.jim_commands.append(&mut prior.jim_commands);
                            ctx.original_scopes.append(&mut prior.original_scopes);
                        }
                        self.pending_exec_leave = Some(ctx);
                    }
                    // Control shapes with no owning frame (a traced `yield` /
                    // `tailcall` builtin itself): settle with an empty ok —
                    // their real result forms elsewhere.
                    Tick::Suspend(_) => {
                        let settled = self.finish_exec_leave(&ctx, &ok(Value::empty()));
                        let _ = settled;
                        if let Some(refused) = self.exec_leave_scope_refusal(&ctx) {
                            return Err(refused);
                        }
                        ctx.traces = false;
                        ctx.step_scopes = 0;
                        ctx.leave_owner = None;
                        ctx.cmd_string = None;
                        if let Some(refused) = self.exec_leave_scope_refusal(&ctx) {
                            let original = ctx.original_scopes.clone();
                            drop(ctx);
                            for scope in &original {
                                let _ = scope.check(self);
                            }
                            return Err(refused);
                        }
                        f.exec_leave.push(ctx);
                    }
                    Tick::Continue | Tick::Return(_) | Tick::Tailcall(_) => {
                        let settled = self.settle_original_exec_leave(ctx, ok(Value::empty()));
                        let _ = settled;
                        if let Some(refused) = self.refused_completion() {
                            return Err(refused);
                        }
                    }
                }
                Ok(Some(tick))
            }
            Ok(None) => {
                // Synchronous completion: the result is on top of `f`'s stack.
                let result = f.stack.last().cloned().unwrap_or_else(Value::empty);
                let replacement = self.settle_original_exec_leave(ctx, ok(result));
                if replacement.code.is_ok() {
                    Ok(None)
                } else {
                    // A leave-trace error replaces the command's result
                    // (tclsh-pinned: LEAVEFAIL).
                    f.stack.pop();
                    Err(replacement)
                }
            }
            Err(c) => Err(self.settle_original_exec_leave(ctx, c)),
        }
    }

    pub(super) fn dispatch_words_traced(
        &mut self,
        f: &mut Frame,
        words: &crate::NativeListItems,
        entered: Option<&EnteredCommand>,
        context: &crate::interp::namespace_context::NamespaceExecutionContext,
        usage: &[crate::command::NativeArgumentUsageRewrite],
        original_scope: &NativeOperationScope,
    ) -> Result<Option<Tick>, Completion<Value>> {
        let usage_revision = self.native_invocation.usage_revision;
        let original = &words[0];
        // The complete current command, list-merged (tclsh-pinned shape:
        // `p one {t w o}`).
        let cmd_string = Value::list(words.to_vec());
        for scope in self.step_scopes_to_fire() {
            if scope.active_for("enterstep") && self.exec_trace_entry_live(&scope.key, &scope.entry)
            {
                let r = self.run_cmd_trace_callback(
                    &scope.entry,
                    &[cmd_string.clone(), Value::string("enterstep")],
                );
                if let Some(refused) = self.original_scope_refusal(original_scope) {
                    return Err(refused);
                }
                if !r.code.is_ok() {
                    return Err(r);
                }
            }
        }
        // A retained entry represents a command that was specialised while it
        // had no execution trace. A trace installed by one of its argument
        // substitutions applies only to later invocations in C Tcl; consulting
        // the live table here would make it fire retroactively. When a trace
        // already existed at command entry no token was retained, so ordinary
        // live lookup remains correct for that generic invocation. The owner
        // resolver also follows an open rename window to the command's moved
        // trace list while leaving `cmd_string` in the caller's spelling.
        let (sidecar, own_at_entry) = self.dispatch_trace_owner(original, entered, context)?;
        for entry in own_at_entry.iter().rev() {
            if !sidecar.is_attached() {
                break;
            }
            if !self.exec_trace_entry_live(&sidecar, entry) {
                continue;
            }
            if entry.has_op("enter") {
                let r = self
                    .run_cmd_trace_callback(entry, &[cmd_string.clone(), Value::string("enter")]);
                if let Some(refused) = self.original_scope_refusal(original_scope) {
                    return Err(refused);
                }
                if !r.code.is_ok() {
                    self.exec_trace_failed("enter", &cmd_string, &r);
                    return Err(r);
                }
            }
        }
        let post_enter = if entered.is_some_and(|entered| {
            matches!(
                entered.execution_traces,
                SelectedNativeExecutionTraces::Omitted
            )
        }) {
            None
        } else {
            let selected = self
                .resolve_original_command_key_in_context(context, original)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?
                .map(CommandSidecarKey::visible)
                .map(|key| self.renamed_command_key(key));
            let attached = sidecar.key();
            if entered.is_some() || selected.as_ref() == attached.as_ref() {
                attached.map(|key| self.exec_traces.get(&key).cloned().unwrap_or_default())
            } else {
                None
            }
        };
        let leave_owner =
            (!own_at_entry.is_empty() && post_enter.is_some()).then(|| sidecar.clone());
        let pushed =
            self.push_exec_step_scopes(&sidecar, post_enter.as_deref().unwrap_or_default());
        let ctx = ExecLeaveCtx {
            original_scopes: vec![OriginalExecutionOwner::Entered(original_scope.clone())],
            traces: true,
            jim_commands: Vec::new(),
            cmd_string: Some(cmd_string),
            leave_owner,
            step_scopes: pushed,
        };
        let usage = if usage_revision == self.native_invocation.usage_revision {
            usage
        } else {
            &[]
        };
        let dispatched = self.dispatch_words_inner(
            (&mut *f, words),
            Some(&sidecar),
            entered,
            None,
            context,
            usage,
            original_scope,
        );
        self.settle_traced_word_dispatch(f, ctx, dispatched)
    }

    /// Settle the leave side of one traced dispatch: pop the step scopes it
    /// pushed, fire its own `leave` traces, then the still-active outer
    /// scopes' `leavestep` — each as `callback cmd-string code result op`
    /// (tclsh-pinned shapes).  Returns `Some(error)` when a trace errored —
    /// the error REPLACES the command's completion (tclsh-pinned) — else
    /// `None`.
    /// An `enter` or `leave` execution trace on `command` failed with `c`.
    /// From 8.6 C names the trace in a frame of its own and the command is
    /// not logged again; before, the failure is the command's, which a leave
    /// trace's caller logs as the error leaves it.
    fn exec_trace_failed(&mut self, op: &str, command: &Value, c: &Completion<Value>) {
        if c.code != Code::Error {
            return;
        }
        let command = command.to_str();
        if self.runtime_version().has_execution_trace_error_frames() {
            const LIMIT: usize = 55;
            let mut end = command.len().min(LIMIT);
            while !command.is_char_boundary(end) {
                end -= 1;
            }
            let marker = if end < command.len() { "..." } else { "" };
            let message = c.result.to_str().to_string();
            self.seed_error_info_frame(
                &message,
                format!("\n    ({op} trace on \"{}{marker}\")", &command[..end]),
            );
            self.mark_error_logged();
        } else if op == "leave" {
            self.clear_error_logged();
            self.log_command_info(command.as_bytes(), c.result.to_str().as_bytes(), 0);
        }
    }

    pub(crate) fn finish_exec_leave(
        &mut self,
        ctx: &ExecLeaveCtx,
        c: &Completion<Value>,
    ) -> Option<Completion<Value>> {
        debug_assert!(
            ctx.jim_commands
                .iter()
                .all(super::super::super::interp::native_jim_lookup::JimCommandLease::is_active)
        );
        self.pop_exec_step_scopes(ctx.step_scopes);
        if let Some(refused) = self.exec_leave_scope_refusal(ctx) {
            return Some(refused);
        }
        if !ctx.traces || (self.exec_traces.is_empty() && self.exec_step_scopes.is_empty()) {
            return None;
        }
        let command = ctx.cmd_string.clone().unwrap_or_else(Value::empty);
        let args = |result: Value, op: &str| {
            [
                command.clone(),
                Value::string(c.code.as_int().to_string()),
                result,
                Value::string(op),
            ]
        };
        let mut replacement = None;
        // Tcl threads a successful leave callback's result to the next
        // callback, while the traced command's own completion remains `c`.
        // The list is intentionally live: when this owner already had a trace
        // at entry, a leave trace added by an enter callback or by the command
        // body joins the current invocation. Removal, deletion and relocation
        // are observed through the sidecar at settlement.
        // C's interpreter holds its own reference to the result while leave
        // traces run. The completion may carry it unowned, where the first
        // reference a callback takes and drops would release it.
        let _held = HeldResult(Some(c.result.clone()));
        let mut trace_result = c.result.clone();
        if let Some(owner) = &ctx.leave_owner
            && let Some(key) = owner.key()
            && let Some(entries) = self.exec_traces.get(&key).cloned()
        {
            for entry in entries {
                if !owner.is_attached() {
                    break;
                }
                if !self.exec_trace_entry_live(owner, &entry) {
                    continue;
                }
                if entry.has_op("leave") {
                    let r =
                        self.run_cmd_trace_callback(&entry, &args(trace_result.clone(), "leave"));
                    if let Some(refused) = self.exec_leave_scope_refusal(ctx) {
                        return Some(refused);
                    }
                    if !r.code.is_ok() {
                        self.exec_trace_failed("leave", &command, &r);
                        replacement = Some(r);
                        break;
                    }
                    trace_result = r.result;
                }
            }
        }
        let mut leaving = self.step_scopes_to_fire();
        leaving.reverse();
        for scope in leaving {
            if scope.active_for("leavestep") && self.exec_trace_entry_live(&scope.key, &scope.entry)
            {
                let r = self
                    .run_cmd_trace_callback(&scope.entry, &args(trace_result.clone(), "leavestep"));
                if let Some(refused) = self.exec_leave_scope_refusal(ctx) {
                    return Some(refused);
                }
                if !r.code.is_ok() {
                    replacement = Some(r);
                    break;
                }
                trace_result = r.result;
            }
        }
        replacement
    }

    /// A cloned trace snapshot may outlive an earlier callback removing its
    /// registration.  Rc pointer identity distinguishes duplicate callbacks;
    /// a later addition is absent from the snapshot and therefore excluded.
    pub(super) fn exec_trace_entry_live(
        &self,
        handle: &CommandSidecarHandle,
        entry: &Rc<CmdTraceEntry>,
    ) -> bool {
        handle.key().is_some_and(|key| {
            self.exec_traces
                .get(&key)
                .is_some_and(|entries| entries.iter().any(|live| Rc::ptr_eq(live, entry)))
        })
    }
}
