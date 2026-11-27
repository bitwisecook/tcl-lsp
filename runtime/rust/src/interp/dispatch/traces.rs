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

//! Command and execution traces: `rename` and `delete` traces, `enter`, `leave` and step traces, and the traced dispatch path.

use super::super::native_error_headers::NativeReturnOptions;
use super::super::{
    cmd_trace_owned_by, drop_fresh, native_operation_currency, native_script, new_string,
    obj_bytes, Code, Interp,
};
use super::CommandDispatchSelection;
use crate::obj;
use crate::obj::TclObj;
use std::rc::Rc;
use tcl_runtime_api::guard::GuardDomain;

impl Interp {
    /// Fire matching command traces (`rename`/`delete`) as `command oldName
    /// newName op` (C's `TraceCommandProc`). `new_fqn` is empty for a delete.
    /// Callback errors are **ignored** (C: "We ignore errors in these traced
    /// commands"). Re-entrant firing on *this* command is suppressed (C's
    /// per-`Command` `CMD_TRACE_ACTIVE`/`CMD_DYING`); the interp result is
    /// preserved across the callbacks.
    /// Fire one command token's own trace list.
    /// C walks `cmdPtr->tracePtr`, so a trace registered on a *later* token
    /// bound at the same name — a replacement, or a same-named command in a
    /// namespace that took the retained one's spelling — never fires for the
    /// token being deleted. `dying` is `None` only when an internal caller has
    /// no bound token to discriminate by.
    ///
    /// `key` addresses the trace list and gates re-entry; `old_fqn` only names
    /// the command in the callback's first word. They differ for a rename,
    /// which fires from the *destination's* list (see
    /// [`Self::move_bound_command`]) while still naming the command it is
    /// leaving.
    pub(in super::super) fn fire_cmd_trace_of_token(
        &mut self,
        key: &[u8],
        old_fqn: &[u8],
        new_fqn: &[u8],
        op_bit: u8,
        dying: Option<u64>,
    ) {
        if self.host_refusal_pending() {
            return;
        }
        if self
            .traces
            .borrow()
            .firing_cmd_traces
            .iter()
            .any(|firing| firing.0 == key && firing.1 == dying)
        {
            return;
        }
        // C prepends each new command trace (`Tcl_TraceCommand`, tclTrace.c
        // 9.0.4:1016-1018) and `CallCommandTraces` walks the list head→tail
        // (tclBasic.c:3972-3974), so the newest fires first. Our Vec pushes
        // newest-last.
        // The callbacks are captured up front, not re-read per step: this walk
        // owns the dying token's list, which a callback's re-creation of the
        // command detaches from the name. See [`Self::cmd_trace_untraced`].
        let entries: Vec<(u64, Vec<u8>)> = self
            .traces
            .borrow()
            .cmd_traces
            .iter()
            .rev()
            // Both mechanisms are needed and neither can do the other's
            // job: the token generation says which registrations this
            // deletion *owns* (a callback's re-creation binds a different
            // token under the same name), while the id lets the walk skip a
            // registration an explicit `trace remove` cancelled mid-walk.
            .filter(|t| {
                t.name == key && (t.ops & op_bit) != 0 && cmd_trace_owned_by(t.token, dying)
            })
            .map(|t| (t.id, t.command.clone()))
            .collect();
        if entries.is_empty() {
            return;
        }
        let op: &[u8] = if op_bit == crate::cmd_trace::ops::RENAME {
            b"rename"
        } else {
            b"delete"
        };
        // Preserve the result object across the callbacks.
        let saved = match self.save_native_command_trace_result(false) {
            Ok(saved) => saved,
            Err(_) => return,
        };

        // Only `firing_cmd_traces` is raised, never `exec_firing`: C sets
        // `INTERP_TRACE_IN_PROGRESS` in exactly one place — `TraceExecutionProc`
        // (tclTrace.c 9.0.4:1765), around an *execution* trace's callback —
        // and `CallCommandTraces` sets nothing. So a command dispatched from a
        // `rename`/`delete` callback is traced like any other: its `enter` and
        // `leave` traces fire, and an enclosing `enterstep`/`leavestep` scope
        // steps the callback's own commands. Re-entering *this* command's
        // rename/delete traces is what is suppressed, per command, above.
        // `firing_cmd_traces` is therefore what marks this walk as in flight
        // for `TraceTable::trace_walk_in_flight`.
        self.traces
            .borrow_mut()
            .firing_cmd_traces
            .push((key.to_vec(), dying));
        for (id, cmd) in entries {
            if self.cmd_trace_untraced(id) {
                continue;
            }
            // Append `oldName newName op` as properly-quoted list elements.
            let args =
                self.new_list_object(&[new_string(old_fqn), new_string(new_fqn), new_string(op)]);
            let mut line = cmd;
            line.push(b' ');
            line.extend_from_slice(&obj_bytes(args));
            drop_fresh(args);
            self.reset_native_error_objects_before_trace_script();
            if self.clear_return_options_checked().is_err() {
                break;
            }
            let saved_script = match self.save_native_command_trace_result(true) {
                Ok(saved) => saved,
                Err(_) => break,
            };
            let _ = self.eval_native_command_trace_script(false, &line);
            if let Some(saved) = saved_script {
                if self.restore_native_variable_trace_result(saved).is_err() {
                    break;
                }
            }
        }
        {
            let mut traces = self.traces.borrow_mut();
            traces.firing_cmd_traces.pop();
            if !traces.trace_walk_in_flight() {
                traces.untraced_cmd_trace_ids.clear();
            }
        }

        if let Some(saved) = saved {
            let _ = self.restore_native_variable_trace_result(saved);
        }
    }

    /// The callback prefix of the live command/execution trace `id`, or `None`
    /// when a callback has since removed it. C walks the trace list through
    /// `nextPtr` and `Tcl_UntraceCommand` unlinks a record at once, so a trace
    /// removed mid-firing never fires in that pass.
    ///
    /// This is the **execution** rule: `TclCheckExecutionTraces` follows the
    /// list of whatever command the name now holds, so a callback that
    /// redefines the traced command stops the rest of the walk — measured on
    /// tclsh 8.6.16 and 9.0.4, where `proc t {}` inside an `enter` callback
    /// keeps the older `enter` callback from running. The delete walk answers
    /// differently; see [`Self::cmd_trace_untraced`].
    fn live_cmd_trace(&self, id: u64) -> Option<Vec<u8>> {
        self.traces
            .borrow()
            .cmd_traces
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.command.clone())
    }

    /// Whether execution trace `id` is currently running its own callback —
    /// C's per-trace `TCL_TRACE_EXEC_IN_PROGRESS` (`tclTrace.c` 9.0.4:1655),
    /// which is what bounds a callback that invokes the command it traces. Per
    /// *trace*: a second trace on the same command still fires for that inner
    /// call, and so does a `leave` trace while an `enter` one is running
    /// (tclsh 8.6/9.0-pinned).
    fn exec_trace_is_firing(&self, id: u64) -> bool {
        self.traces.borrow().firing_exec_traces.contains(&id)
    }

    /// Whether `trace remove` has unlinked command trace `id` since this walk
    /// began. `CallCommandTraces` is handed the dying token's own list
    /// (`tclBasic.c` 9.0.4:3972-3993), so a callback that re-creates the
    /// command under the same name takes the name-keyed table entry over while
    /// the remaining callbacks still run; only an explicit untrace cancels one.
    /// Measured on tclsh 8.6.16 and 9.0.4: with two `delete` traces whose newer
    /// callback runs `proc foo …`, **both** fire.
    fn cmd_trace_untraced(&self, id: u64) -> bool {
        self.traces.borrow().untraced_cmd_trace_ids.contains(&id)
    }

    pub(in super::super) fn finish_saved_execution_result(
        &self,
        currency: &native_operation_currency::SavedNativeOperationCurrency,
        saved: *mut TclObj,
        saved_options: NativeReturnOptions,
        restore: bool,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let cleanup =
            native_operation_currency::NativeOperationScope::enter_saved_cleanup(currency, self)?;
        let options_outcome = if restore {
            let retired = self.result.replace(saved);
            // SAFETY: publish the saved owning reference before retiring the
            // callback result; neither role is borrowed under its destructor.
            unsafe { obj::decr_ref_count(retired) };
            self.restore_return_options_for_cleanup(saved_options, &cleanup)
        } else {
            // These are genuine discarded saved roles, not protective holds.
            unsafe { obj::decr_ref_count(saved) };
            drop(saved_options);
            Ok(())
        };
        cleanup.check()?;
        options_outcome
    }

    /// Fire `enter` execution traces on `fqn` (creation order), invoking each as
    /// `<prefix> {cmd args} enter`. Returns `Some(code)` if a callback completed
    /// non-OK — the command is then aborted with that code and the callback's
    /// result (C's `TclEvalObjvInternal`: `traceCode != TCL_OK ⇒ return`).
    fn fire_exec_enter(&mut self, fqn: &[u8], token: Option<u64>, cmd_word: &[u8]) -> Option<Code> {
        use crate::cmd_trace::ops;
        // C fires `enter` newest-first (the trace list is prepended; the loop
        // walks it head→tail). Our Vec pushes newest-last, so iterate reversed.
        let ids: Vec<u64> = self
            .traces
            .borrow()
            .cmd_traces
            .iter()
            .rev()
            .filter(|t| t.name == fqn && t.token == token && (t.ops & ops::ENTER) != 0)
            .map(|t| t.id)
            .collect();
        if ids.is_empty() {
            return None;
        }
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => return Some(self.refuse_native_execution(first)),
        };
        let saved_currency = native_operation_currency::SavedNativeOperationCurrency::capture(self);
        let saved = self.result.get();
        unsafe { obj::incr_ref_count(saved) };
        let saved_options = self.take_return_options();
        self.traces.borrow_mut().exec_firing += 1;
        let mut abort: Option<Code> = None;
        for id in ids {
            // A trace whose own callback is running is skipped, per trace
            // rather than per command (C's `TCL_TRACE_EXEC_IN_PROGRESS`).
            if self.exec_trace_is_firing(id) {
                continue;
            }
            let Some(cmd) = self.live_cmd_trace(id) else {
                continue;
            };
            let args = self.new_list_object(&[new_string(cmd_word), new_string(b"enter")]);
            let mut line = cmd;
            line.push(b' ');
            line.extend_from_slice(&obj_bytes(args));
            drop_fresh(args);
            self.traces.borrow_mut().firing_exec_traces.push(id);
            let c = match self.clear_return_options_checked() {
                Ok(()) => self.eval_native_command_trace_script(true, &line),
                Err(first) => self.refuse_native_execution(first),
            };
            self.traces.borrow_mut().firing_exec_traces.pop();
            if c != Code::Ok {
                // The callback's result becomes the command's result; abort.
                abort = Some(c);
                break;
            }
        }
        self.traces.borrow_mut().exec_firing -= 1;
        let cleanup = self.finish_saved_execution_result(
            &saved_currency,
            saved,
            saved_options,
            abort.is_none(),
        );
        if let Err(first) = scope.currency().ensure_current_or_refuse().and(cleanup) {
            return Some(self.refuse_native_execution(first));
        }
        abort
    }

    /// Fire `leave` execution traces on `fqn` (reverse creation order), invoking
    /// each as `<prefix> {cmd args} <code> <result> leave`. A leave-trace non-OK
    /// code overrides the command's result/code (C's `TEOV_RunLeaveTraces`).
    fn fire_exec_leave(
        &mut self,
        fqn: &[u8],
        token: Option<u64>,
        cmd_word: &[u8],
        code: Code,
    ) -> Code {
        use crate::cmd_trace::ops;
        // C fires `leave` oldest-first (reverse-scan of the prepended list). Our
        // Vec pushes newest-last, so iterate forward.
        let ids: Vec<u64> = self
            .traces
            .borrow()
            .cmd_traces
            .iter()
            .filter(|t| t.name == fqn && t.token == token && (t.ops & ops::LEAVE) != 0)
            .map(|t| t.id)
            .collect();
        if ids.is_empty() {
            return code;
        }
        // Save the command's result once; restore it after the callbacks (C's
        // single Tcl_SaveInterpState/RestoreInterpState around the loop). The
        // result is NOT restored *between* callbacks: each leave callback's
        // `<result>` element is the live result, so a callback that changes the
        // result is observed by the next one.
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => return self.refuse_native_execution(first),
        };
        let saved_currency = native_operation_currency::SavedNativeOperationCurrency::capture(self);
        let saved = self.result.get();
        unsafe { obj::incr_ref_count(saved) };
        let saved_options = self.take_return_options();
        let code_str = code.as_int().to_string().into_bytes();

        self.traces.borrow_mut().exec_firing += 1;
        let mut override_code: Option<Code> = None;
        for id in ids {
            // A trace whose own callback is running is skipped, per trace
            // rather than per command (C's `TCL_TRACE_EXEC_IN_PROGRESS`).
            if self.exec_trace_is_firing(id) {
                continue;
            }
            let Some(cmd) = self.live_cmd_trace(id) else {
                continue;
            };
            let result_bytes = obj_bytes(self.result.get());
            let args = self.new_list_object(&[
                new_string(cmd_word),
                new_string(&code_str),
                new_string(&result_bytes),
                new_string(b"leave"),
            ]);
            let mut line = cmd;
            line.push(b' ');
            line.extend_from_slice(&obj_bytes(args));
            drop_fresh(args);
            self.traces.borrow_mut().firing_exec_traces.push(id);
            let c = match self.clear_return_options_checked() {
                Ok(()) => self.eval_native_command_trace_script(true, &line),
                Err(first) => self.refuse_native_execution(first),
            };
            self.traces.borrow_mut().firing_exec_traces.pop();
            if c != Code::Ok {
                override_code = Some(c);
                break;
            }
        }
        self.traces.borrow_mut().exec_firing -= 1;

        let cleanup = self.finish_saved_execution_result(
            &saved_currency,
            saved,
            saved_options,
            override_code.is_none(),
        );
        if let Err(first) = scope.currency().ensure_current_or_refuse().and(cleanup) {
            return self.refuse_native_execution(first);
        }
        override_code.unwrap_or(code)
    }

    /// Keep a moved token's in-flight trace suppression attached to that token.
    fn relocate_rename_state(&self, old_fqn: &[u8], new_fqn: &[u8], generation: u64) {
        let mut traces = self.traces.borrow_mut();
        for (firing, token) in &mut traces.firing_cmd_traces {
            if firing == old_fqn && *token == Some(generation) {
                *firing = new_fqn.to_vec();
            }
        }
    }

    /// Move every command/execution trace on `old_fqn` to `new_fqn` (the trace
    /// follows a renamed command, as C keeps the trace list on the moving
    /// `Command`).
    pub(in super::super) fn move_cmd_traces(
        &mut self,
        old_fqn: &[u8],
        new_fqn: &[u8],
        generation: u64,
    ) {
        self.relocate_rename_state(old_fqn, new_fqn, generation);
        // C moves one `Command` and its trace list. A visible and hidden token
        // may legally share display spelling, so move only this generation.
        let mut traces = self.traces.borrow_mut();
        let mut moved = false;
        for t in traces.cmd_traces.iter_mut() {
            if t.name == old_fqn && t.token == Some(generation) {
                t.name = new_fqn.to_vec();
                moved = true;
            }
        }
        drop(traces);
        if moved {
            self.invalidate_guard_domain(GuardDomain::CommandTrace);
        }
    }

    /// Slow path: the command may carry execution (enter/leave/step) traces, or
    /// a step trace is active. Mirrors C's `TclEvalObjvInternal` order: interp
    /// (step) enter traces fire before per-command enter; per-command leave
    /// fires before interp (step) leave.
    pub(super) fn dispatch_traced(
        &mut self,
        argv: &[*mut TclObj],
        prebound: CommandDispatchSelection,
        ordinary: bool,
        source: Option<&Rc<native_script::OriginalSourceInvocation>>,
    ) -> Code {
        use crate::cmd_trace::ops;
        // Inside a rename's callbacks the vacating name still resolves, but the
        // one command's trace list has already moved to the destination key —
        // so look the traces up there, as C reaches them through the shared
        // `Command` from either hash entry. Only the *key* is canonicalised:
        // the callback's own words stay the spelling the caller invoked
        // (`cmd_word` below), which is what tclsh passes.
        let lookup_context = match &prebound {
            CommandDispatchSelection::LookupAt(lookup, origin) => Some((*lookup, *origin)),
            _ => None,
        };
        let mut missing = match &prebound {
            CommandDispatchSelection::Missing {
                lookup,
                caller,
                origin,
            } => Some((*lookup, *caller, *origin)),
            _ => None,
        };
        let (fqn, token, prebound_command) = match prebound {
            CommandDispatchSelection::Bound(fqn, binding) => {
                if ordinary {
                    if let Err(code) = self.reset_native_ensemble_rewrite(
                        tcl_native_model::native_ensemble_rewrite::EnsembleRewriteResetEvent::AfterSuccessfulOrdinaryLookup,
                    ) { return code; }
                }
                (Some(fqn), Some(binding.generation), Some(binding.command))
            }
            CommandDispatchSelection::Unselected | CommandDispatchSelection::LookupAt(..) => {
                let lookup = lookup_context.map_or(self.current_ns.get(), |(lookup, _)| lookup);
                let selected = match self.resolve_original_command_at(lookup, argv[0]) {
                    Ok(selected) => selected,
                    Err(error) => return self.report_cmd_error(error.into()),
                };
                if selected.is_none() {
                    if let Some((lookup, origin)) = lookup_context {
                        missing = Some((lookup, self.current_ns.get(), origin));
                    }
                }
                if ordinary && selected.is_some() {
                    if let Err(code) = self.reset_native_ensemble_rewrite(
                        tcl_native_model::native_ensemble_rewrite::EnsembleRewriteResetEvent::AfterSuccessfulOrdinaryLookup,
                    ) { return code; }
                }
                let token = selected.and_then(|(_, generation)| generation);
                let fqn = token.and_then(|generation| {
                    self.namespaces
                        .borrow()
                        .native_command_at_node(generation)
                        .map(|(_, fqn)| fqn)
                });
                (fqn, token, None)
            }
            CommandDispatchSelection::Missing { .. } => (None, None, None),
        };
        let (has_enter, has_leave, has_step) = match &fqn {
            Some(f) => {
                let t = self.traces.borrow();
                let (mut he, mut hl, mut hs) = (false, false, false);
                for tr in t
                    .cmd_traces
                    .iter()
                    .filter(|tr| tr.name == *f && tr.token == token)
                {
                    he |= (tr.ops & ops::ENTER) != 0;
                    hl |= (tr.ops & ops::LEAVE) != 0;
                    hs |= (tr.ops & ops::STEP_ANY) != 0;
                }
                (he, hl, hs)
            }
            None => (false, false, false),
        };
        // C's one read of `INTERP_TRACE_IN_PROGRESS`: `TclCheckInterpTraces`
        // returns immediately while an execution callback is running, so the
        // callback's own commands are never step-observed.
        let stepping = {
            let t = self.traces.borrow();
            !t.step_active.is_empty() && t.exec_firing == 0
        };
        if !has_enter && !has_leave && !has_step && !stepping {
            return match prebound_command {
                Some(command) => self.invoke_bound_with_source(command, token, argv, None, source),
                None => match missing {
                    Some((lookup, caller, origin)) => {
                        self.dispatch_missing_command(argv, lookup, caller, origin)
                    }
                    None => match lookup_context {
                        Some((lookup, origin)) => self
                            .dispatch_inner_at_with_source(argv, ordinary, lookup, origin, source),
                        None => self.dispatch_inner_at_with_source(
                            argv,
                            ordinary,
                            self.current_ns.get(),
                            tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
                            source,
                        ),
                    },
                },
            };
        }
        // The `{cmd arg ...}` word: argv rendered as a single list element (C's
        // `TraceExecutionProc` builds it via per-arg `DStringAppendElement`).
        let cmd_word = {
            let lst = self.new_list_object(argv);
            let bytes = obj_bytes(lst);
            drop_fresh(lst);
            bytes
        };
        // (A) enterstep for active step traces (interp traces fire on enter
        // before per-command enter); a non-OK enterstep aborts the command.
        if stepping {
            if let Some(c) = self.fire_step(&cmd_word, ops::ENTERSTEP, None) {
                return c;
            }
        }
        // (B) per-command enter; a non-OK enter aborts with the callback result.
        if has_enter {
            if let Some(c) = self.fire_exec_enter(fqn.as_deref().unwrap(), token, &cmd_word) {
                return c;
            }
        }
        // (C) install this command's step traces (deduped against recursion).
        let installed = if has_step {
            self.install_step_traces(fqn.as_deref().unwrap(), token)
        } else {
            0
        };
        let mut code = match prebound_command {
            Some(command) => self.invoke_bound_with_source(command, token, argv, None, source),
            None => match missing {
                Some((lookup, caller, origin)) => {
                    self.dispatch_missing_command(argv, lookup, caller, origin)
                }
                None => self.dispatch_inner_at_with_source(
                    argv,
                    ordinary,
                    self.current_ns.get(),
                    tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
                    source,
                ),
            },
        };
        // (D) remove the step traces installed above (they are the last pushed).
        if installed > 0 {
            self.remove_installed_step_traces(installed);
        }
        if self.host_refusal_pending() {
            return Code::Error;
        }
        // (E) per-command leave (before interp/step leave), then (F) leavestep.
        if has_leave {
            code = self.fire_exec_leave(fqn.as_deref().unwrap(), token, &cmd_word, code);
        }
        if stepping {
            if let Some(c) = self.fire_step(&cmd_word, ops::LEAVESTEP, Some(code)) {
                code = c;
            }
        }
        code
    }

    /// Push a `StepActive` for each step trace on `fqn` not already live (dedup
    /// by owner+prefix handles recursion: only the outermost installs). Returns
    /// how many were pushed (the last `n` of `step_active`, popped on exit).
    fn install_step_traces(&mut self, fqn: &[u8], token: Option<u64>) -> usize {
        use crate::cmd_trace::{ops, StepActive};
        let to_install: Vec<(u8, Vec<u8>)> = {
            let t = self.traces.borrow();
            t.cmd_traces
                .iter()
                .filter(|c| c.name == fqn && c.token == token && (c.ops & ops::STEP_ANY) != 0)
                .filter(|c| {
                    !t.step_active
                        .iter()
                        .any(|s| s.owner == fqn && s.token == token && s.command == c.command)
                })
                .map(|c| (c.ops & ops::STEP_ANY, c.command.clone()))
                .collect()
        };
        let n = to_install.len();
        let mut tt = self.traces.borrow_mut();
        for (ops_bits, command) in to_install {
            tt.step_active.push(StepActive {
                owner: fqn.to_vec(),
                token,
                ops: ops_bits,
                command,
            });
        }
        n
    }

    /// Pop the `n` step traces this command installed (balanced nesting keeps
    /// them at the end: any nested step-traced command popped its own first).
    fn remove_installed_step_traces(&mut self, n: usize) {
        let mut tt = self.traces.borrow_mut();
        let keep = tt.step_active.len() - n;
        tt.step_active.truncate(keep);
    }

    /// Fire active step traces for the current command. `ENTERSTEP` fires in
    /// reverse install order with `<prefix> {cmd args} enterstep` (a non-OK code
    /// aborts); `LEAVESTEP` fires in install order with `<prefix> {cmd args}
    /// <code> <result> leavestep` (a non-OK code overrides). The result is saved
    /// once and restored after, but live between callbacks (C's interp-trace
    /// `SaveInterpState`/`RestoreInterpState`).
    fn fire_step(&mut self, cmd_word: &[u8], op_bit: u8, code: Option<Code>) -> Option<Code> {
        use crate::cmd_trace::ops;
        let is_enter = op_bit == ops::ENTERSTEP;
        let mut cmds: Vec<Vec<u8>> = self
            .traces
            .borrow()
            .step_active
            .iter()
            .filter(|s| (s.ops & op_bit) != 0)
            .map(|s| s.command.clone())
            .collect();
        if is_enter {
            cmds.reverse();
        }
        if cmds.is_empty() {
            return None;
        }
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => return Some(self.refuse_native_execution(first)),
        };
        let saved_currency = native_operation_currency::SavedNativeOperationCurrency::capture(self);
        let saved = self.result.get();
        unsafe { obj::incr_ref_count(saved) };
        let saved_options = self.take_return_options();
        let code_str = code.map(|c| c.as_int().to_string().into_bytes());
        let op_label: &[u8] = if is_enter { b"enterstep" } else { b"leavestep" };

        self.traces.borrow_mut().exec_firing += 1;
        let mut outcome: Option<Code> = None;
        for cmd in cmds {
            let args = if is_enter {
                self.new_list_object(&[new_string(cmd_word), new_string(op_label)])
            } else {
                let result_bytes = obj_bytes(self.result.get());
                self.new_list_object(&[
                    new_string(cmd_word),
                    new_string(code_str.as_deref().unwrap_or(b"0")),
                    new_string(&result_bytes),
                    new_string(op_label),
                ])
            };
            let mut line = cmd;
            line.push(b' ');
            line.extend_from_slice(&obj_bytes(args));
            drop_fresh(args);
            let c = match self.clear_return_options_checked() {
                Ok(()) => self.eval_native_command_trace_script(true, &line),
                Err(first) => self.refuse_native_execution(first),
            };
            if c != Code::Ok {
                outcome = Some(c);
                break;
            }
        }
        self.traces.borrow_mut().exec_firing -= 1;

        let cleanup = self.finish_saved_execution_result(
            &saved_currency,
            saved,
            saved_options,
            outcome.is_none(),
        );
        if let Err(first) = scope.currency().ensure_current_or_refuse().and(cleanup) {
            return Some(self.refuse_native_execution(first));
        }
        outcome
    }
}
