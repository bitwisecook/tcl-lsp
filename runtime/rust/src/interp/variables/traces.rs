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

//! Variable traces: registration, read traces and read-for-update, firing with the resolved identity, trace errors and array traces.

use super::super::{
    drop_fresh, native_operation_currency, native_variable_names, native_variable_observers,
    new_string, obj_bytes, Code, Interp, TraceAccess, VarTeardownCallback,
    VariableTraceGuestOutcome,
};
use crate::frame::VarError;
use crate::obj;
use crate::obj::TclObj;
use tcl_runtime_api::guard::GuardDomain;

impl Interp {
    /// Whether any variable trace can observe accesses to `name` — the runtime
    /// half of a guarded `TraceBarrier`.
    ///
    /// The name is resolved to the same `(home, simple name)` identity trace
    /// *firing* uses, so this follows `upvar`/`global`/`variable` links to the
    /// target's traces and reports an array as traced when any of its elements
    /// is. It is deliberately conservative in that direction: a `true` may be
    /// broader than the exact access, but a `false` is a promise that nothing
    /// can observe the cell.
    pub(crate) fn var_is_traced(&self, name: &[u8]) -> bool {
        if !self.traces_may_observe(name) {
            return false;
        }
        let Ok((base, _)) = self.variable_name_parts(name) else {
            return true;
        };
        let home = self.trace_identity(&base);
        self.env_trace_reaches(&home)
            || self.traces.borrow().traces.iter().any(|t| {
                crate::cmd_trace::same_variable(t, &home.base, home.ns, home.level)
                    && t.binding_id == home.binding_id
            })
    }

    pub(crate) fn prepare_trace_registration(
        &self,
        name: &[u8],
        element: Option<&[u8]>,
        level: Option<usize>,
    ) -> Result<crate::vars::TraceHome, VarError> {
        let operation =
            native_operation_currency::NativeOperationScope::enter(self).map_err(|first| {
                self.clone().refuse_native_execution(first);
                VarError::NameProtocolUnavailable
            })?;
        self.require_variable_name_protocol()?;
        let captured = crate::vars::EnteredVariablePlace::capture_trace(
            self,
            self.current_ns.get(),
            level,
            name,
            operation.currency(),
        );
        operation
            .currency()
            .ensure_current_or_refuse()
            .map_err(|_| VarError::NameProtocolUnavailable)?;
        let mut captured = captured.map_err(|error| self.byte_store_failure(error))?;
        let reached = captured.prepare_trace_registration(self, element, operation.currency());
        // Retain the complete intrinsic refusal before selected metadata drops.
        let reached = reached.map_err(|error| self.byte_store_failure(error));
        let current = captured
            .check(self, operation.currency())
            .map_err(|error| self.byte_store_failure(error));
        drop(captured);
        operation
            .currency()
            .ensure_current_or_refuse()
            .map_err(|_| VarError::NameProtocolUnavailable)?;
        current?;
        reached
    }

    pub(crate) fn cleanup_trace_shell(
        &self,
        home: &crate::vars::TraceHome,
        key: Option<&[u8]>,
        current: impl Fn() -> Result<(), tcl_cmd_core::CmdError>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        if key.is_none()
            && (self.native_error_variable_at(home).is_some()
                || self.native_precision_trace_at(home))
        {
            return current();
        }
        match crate::vars::CapturedTraceShell::capture(self, home, key) {
            Some(original) => original.finish(self, current),
            None => current(),
        }
    }

    /// The call-frame level a variable trace on `base` should be tied to, so it
    /// dies with the frame (C frees a local var's trace list at frame teardown).
    /// `Some(level)` for an unqualified name resolving frame-local in a proc;
    /// `None` (persistent) for qualified / global / `global`-or-`upvar`-linked
    /// names, which outlive the frame.
    /// The home namespace a variable trace on `base` is scoped to — `Some(ns)`
    /// for a trace on a namespace variable (registered at namespace/global scope,
    /// or a qualified name), so it fires only for that namespace's variable and
    /// dies with the namespace; `None` for a proc-local trace, which matches by
    /// raw name. Used at both trace-add and trace-fire time so they agree.
    /// The `(home namespace, home frame level, simple name)` identity a variable
    /// trace on `base` belongs to.
    ///
    /// Registration and firing both key on this, so every spelling that
    /// resolves to the one cell shares one trace list — including an `upvar`
    /// alias, whose level can only be read off the *resolved* place. C gets
    /// this for free: there the alias and its target are the same `Var`, and
    /// the trace list hangs off that `Var`.
    pub(crate) fn trace_identity(&self, base: &[u8]) -> crate::vars::TraceHome {
        let _ = self.require_variable_name_protocol();

        crate::vars::trace_home(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            self.current_ns.get(),
            base,
        )
    }

    /// Shared reentrancy identity of the actual selected root and member allocations.
    pub(in super::super::super) fn variable_trace_scope(
        &self,
        home: &crate::vars::TraceHome,
        element: Option<&[u8]>,
    ) -> crate::cmd_trace::VarTraceScope {
        let member = element.and_then(|element| {
            crate::vars::trace_element_identity(
                &self.frames.borrow(),
                &self.namespaces.borrow(),
                home,
                element,
            )
        });
        crate::cmd_trace::VarTraceScope::cell(home, element, member)
    }

    /// Whether this release recovers an array element from the resolved `Var`
    /// when the access spelling names none — the release axis itself lives in
    /// `tcl-dialect`, beside `namespace_var_global_fallback`. The two visible
    /// consequences are pinned in `tests/trace_semantics.rs`:
    ///
    /// - `upvar #0 a(k) e; set e 5` fires the array's traces *and* the
    ///   element's with `name2 = k` at 9.0; at 8.6 only the element's own, with
    ///   an empty `name2`.
    /// - `unset a(k)` reports `name1 = a(k)` at 9.0 (the recovered `part2`
    ///   stops `TclCallVarTraces` re-splitting the name) and `name1 = a` at
    ///   8.6.
    fn traces_recover_the_linked_element(&self) -> bool {
        self.runtime_version().traces_recover_linked_array_element()
    }

    /// The unset-trace callbacks a proc frame's locals contribute as the frame
    /// is torn down — C's `TclDeleteVars`, which runs `UnsetVarStruct` (and,
    /// for an array local, `DeleteArray`) over every variable in the frame.
    ///
    /// Read while the frame is still on the stack: after the pop its variables
    /// are gone, and an array local's elements with them. Each variable's own
    /// callbacks fire newest-first and contiguously; *which* variable comes
    /// first follows the frame's retained compiled declarations and native
    /// physical hash entries, including undefined entries carrying traces.
    pub(in super::super) fn frame_teardown_unset_traces(
        &self,
        level: usize,
    ) -> Vec<VarTeardownCallback> {
        if self
            .traces
            .borrow()
            .traces
            .iter()
            .all(|t| t.frame_level != Some(level))
        {
            return Vec::new();
        }
        let names: Vec<Vec<u8>> = match self.frames.borrow().table(level) {
            Some(table) => table
                .teardown_names()
                .into_iter()
                .map(<[u8]>::to_vec)
                .collect(),
            None => return Vec::new(),
        };
        let mut victims = Vec::new();
        for name in names {
            let home = crate::vars::TraceHome {
                binding_id: self
                    .frames
                    .borrow()
                    .table(level)
                    .and_then(|table| table.binding_id(&name)),
                selected_member: None,
                ns: None,
                level: Some(level),
                base: name.clone(),
                link_elem: None,
            };
            victims.extend(self.cell_unset_traces(&home, None, &name, b""));
            let elements = self
                .frames
                .borrow()
                .table(level)
                .and_then(|table| table.capture_array_cell(&name))
                .and_then(|array| array.search_keys())
                .unwrap_or_default();
            for elem in elements {
                victims.extend(self.cell_unset_traces(&home, Some(&elem), &name, &elem));
            }
        }
        victims
    }

    /// Drop every variable trace tied to call-frame `level` (the frame is being
    /// popped; its local variables and their traces go away).
    pub(crate) fn clear_frame_var_traces(&self, level: usize) {
        let mut t = self.traces.borrow_mut();
        let removed = t.traces.iter().any(|v| v.frame_level == Some(level));
        if removed {
            t.traces.retain(|v| v.frame_level != Some(level));
        }
        drop(t);
        if removed {
            self.invalidate_guard_domain(GuardDomain::VariableTrace);
        }
    }

    /// Fire a read trace for `name` before a read (the `&mut` chokepoints that
    /// resolve `$var` call this). Returns `Some(Code::Error)` — with the interp
    /// result set to `can't read "name": <msg>` — if a read trace callback
    /// errored (C's `TclObjCallVarTraces` propagation); else `None`.
    pub(crate) fn fire_read_trace(&mut self, name: &[u8], key: Option<&[u8]>) -> Option<Code> {
        if self.observed_name_policy_selected() {
            return self
                .observed_substitution_receiver(name, key)
                .err()
                .map(|_| Code::Error);
        }
        let input = match key {
            Some(key) => self.separate_variable_input(name, Some(key)),
            None => self.combined_variable_input(name),
        };
        let Ok(input) = input else {
            return Some(Code::Error);
        };
        if !self.variable_traces_may_observe(name) {
            return None;
        }
        let base = input.root().selected().to_vec();
        let key = input.element().map(|element| element.selected());
        match self.fire_var_trace(&base, key, b"read") {
            Ok(VariableTraceGuestOutcome::Complete) => return None,
            Ok(VariableTraceGuestOutcome::GuestError) => {}
            Err(first) => return Some(self.refuse_native_execution(first)),
        }
        let msg = self
            .traces
            .borrow_mut()
            .pending_err
            .take()
            .unwrap_or_default();
        // Display name: `base` or `base(key)`.
        let mut display = base.clone();
        if let Some(k) = key {
            display.push(b'(');
            display.extend_from_slice(k);
            display.push(b')');
        }
        Some(self.var_trace_error(&display, b"read", &msg))
    }

    /// Read a variable's current value for a **read-modify-write** command
    /// (`lappend`, `incr`), firing its read trace but **swallowing** any error
    /// the trace raises — the value simply reads as absent.
    ///
    /// This is C's `TclPtrGetVarIdx` seen from a caller that treats a `NULL`
    /// return as "no current value" rather than as a failure:
    /// `Tcl_LappendObjCmd` creates the element instead (bug 3057639,
    /// append-7.2/7.3/9.0) and `TclPtrIncrObjVar` substitutes 0. Both are
    /// oracle-pinned in `tests/trace_semantics.rs`: with an erroring read trace
    /// on `x`, tclsh 8.6.16 and 9.0.4 both leave `incr x` succeeding with 1.
    ///
    /// `set`/`append`-read, by contrast, propagate the error via
    /// [`fire_read_trace`](Self::fire_read_trace).
    pub(crate) fn read_for_update(
        &mut self,
        base: &[u8],
        elem: Option<&[u8]>,
    ) -> Result<Option<*mut TclObj>, tcl_runtime_api::NativeExecutionError> {
        self.check_entered_native_operation()?;
        if self.variable_traces_may_observe(base)
            && self.fire_var_trace(base, elem, b"read")?.is_guest_error()
        {
            self.traces.borrow_mut().pending_err.take();
            return Ok(None);
        }
        self.check_entered_native_operation()?;
        Ok(match elem {
            Some(k) => self.var_get_elem(base, k),
            None => self.var_get(base),
        })
    }

    /// `lappend`'s name for the same checked read-update owner.
    pub(crate) fn lappend_read(
        &mut self,
        base: &[u8],
        elem: Option<&[u8]>,
    ) -> Result<Option<*mut TclObj>, tcl_runtime_api::NativeExecutionError> {
        self.read_for_update(base, elem)
    }

    /// Invoke every variable trace matching `(base, elem, op)`, as
    /// `command base element op`. A running callback suppresses nested traces
    /// on the same resolved variable cell, while traces on unrelated cells
    /// remain active. The interp result is preserved across the callbacks (the
    /// triggering operation owns the result). For `read`/`write` ops a callback
    /// error is **propagated** as `GuestError`: the message is stashed in
    /// `pending_err` and the access then fails. The selected coordinator keeps
    /// its operation-specific Guest propagation rules. A Host refusal is the
    /// outer error and never permits Guest variable-error formatting.
    pub(super) fn fire_var_trace(
        &mut self,
        base: &[u8],
        elem: Option<&[u8]>,
        op: &[u8],
    ) -> Result<VariableTraceGuestOutcome, tcl_runtime_api::NativeExecutionError> {
        self.check_entered_native_operation()?;
        let home = self.trace_identity(base);
        let access = self.trace_access(base, base, elem, &home, false);
        self.check_entered_native_operation()?;
        self.fire_var_trace_resolved(&home, &access, op)
    }

    /// How one access presents itself to the trace machinery: which cell's
    /// element the traces match, what the callback is told, and whether the
    /// containing array's whole-array traces take part.
    ///
    /// `spelling` is the access as written (`a(k)`), `base`/`elem` its split
    /// halves, and `home` the resolved cell. Three shapes:
    ///
    /// - an explicit element (`set a(k) 2`) matches and reports that element,
    ///   and reports `name1 = a` — C's `TclCallVarTraces` splits `part1` at the
    ///   `(` when the caller left `part2` NULL. The `unset` path is where 9.0
    ///   differs: it recovers `part2` from the `Var` *before* the call, so the
    ///   split never runs and `name1` stays the whole `a(k)`.
    /// - a link into an element (`upvar #0 a(k) e`) matches that element. At
    ///   9.0 it reports it too, and the array's traces fire; at 8.4-8.6
    ///   `part2` stays NULL, so `name2` is empty and only the element's own
    ///   traces run. See [`traces_recover_the_linked_element`](
    ///   Self::traces_recover_the_linked_element).
    /// - anything else is a plain scalar/whole-array access.
    pub(in super::super) fn trace_access(
        &self,
        spelling: &[u8],
        base: &[u8],
        elem: Option<&[u8]>,
        home: &crate::vars::TraceHome,
        unset: bool,
    ) -> TraceAccess {
        let recover = self.traces_recover_the_linked_element();
        match (elem, home.link_elem.as_deref()) {
            (Some(e), _) => TraceAccess {
                reported: if unset && recover { spelling } else { base }.to_vec(),
                match_elem: Some(e.to_vec()),
                report_elem: Some(e.to_vec()),
                spelling_elem: Some(e.to_vec()),
                whole_array: true,
            },
            (None, Some(k)) => TraceAccess {
                reported: base.to_vec(),
                match_elem: Some(k.to_vec()),
                report_elem: recover.then(|| k.to_vec()),
                spelling_elem: None,
                whole_array: recover,
            },
            (None, None) => TraceAccess {
                reported: base.to_vec(),
                match_elem: None,
                report_elem: None,
                spelling_elem: None,
                whole_array: true,
            },
        }
    }

    /// Native precision is a hidden C variable trace and is not reported by
    /// script-level `trace info`. Its registration follows resolved cell homes.
    /// [`Self::has_variable_traces`] for an access spelt `name`, including the
    /// native `env` trace where the spelling may reach the global `env` array
    /// ([`Self::traces_may_observe`]).
    /// The value a read of whole name `base` returns when no trace can observe
    /// it: no script or `env` trace reaches the spelling, and the cell is not a
    /// native `errorInfo`/`errorCode`/`tcl_precision` cell. Exactly the reads
    /// whose `fire_read_trace` completes without effect.
    #[cfg(have_tommath)]
    pub(crate) fn direct_variable_value(&self, base: &[u8]) -> Option<*mut TclObj> {
        if self.observed_name_policy_selected()
            || self.traces_may_observe(base)
            || self.check_entered_native_operation().is_err()
        {
            return None;
        }
        let home = self.trace_identity(base);
        if home.link_elem.is_some()
            || home.binding_id.is_some_and(|id| {
                self.native_precision_cell.get() == Some(id)
                    || self
                        .native_error_cells
                        .borrow()
                        .iter()
                        .any(|(_, registered)| *registered == id)
            })
        {
            return None;
        }
        self.var_get(base)
    }

    pub(super) fn variable_traces_may_observe(&self, name: &[u8]) -> bool {
        !self.native_error_cells.borrow().is_empty()
            || self.native_precision_cell.get().is_some()
            || self.traces_may_observe(name)
    }

    pub(in super::super) fn has_variable_traces(&self) -> bool {
        !self.native_error_cells.borrow().is_empty()
            || !self.traces.borrow().traces.is_empty()
            || self.native_precision_cell.get().is_some()
    }

    /// [`Self::fire_var_trace`] with the identity already resolved — for
    /// `unset`, which must resolve *before* it removes the variable (resolution
    /// can depend on the cell existing).
    pub(in super::super) fn fire_var_trace_resolved(
        &mut self,
        home: &crate::vars::TraceHome,
        access: &TraceAccess,
        op: &[u8],
    ) -> Result<VariableTraceGuestOutcome, tcl_runtime_api::NativeExecutionError> {
        self.fire_var_trace_resolved_with_errors(home, access, op, true)
    }

    pub(in super::super) fn fire_var_trace_resolved_with_errors(
        &mut self,
        home: &crate::vars::TraceHome,
        access: &TraceAccess,
        op: &[u8],
        leave_error_message: bool,
    ) -> Result<VariableTraceGuestOutcome, tcl_runtime_api::NativeExecutionError> {
        // With no script trace and no native variable hook on this cell, no
        // callback can run; only a retained refusal is observable.
        if self.traces.borrow().traces.is_empty()
            && !(access.whole_array && self.env_trace_reaches(home))
            && !self.native_precision_trace_at(home)
            && self.native_error_variable_at(home).is_none()
        {
            self.check_entered_native_operation()?;
            return Ok(VariableTraceGuestOutcome::Complete);
        }
        let _operation = native_operation_currency::NativeOperationScope::enter(self)?;
        self.check_entered_native_operation()?;
        let elem = access.match_elem.as_deref();
        let reported = access.reported.as_slice();
        // The cell this access reaches, and the array cell containing it — C's
        // `varPtr` and `arrayPtr`, each with its own `VAR_TRACE_ACTIVE`.
        let cell = self.variable_trace_scope(home, elem);
        let traces = self.traces.borrow();
        // "If there are already similar trace functions active for the
        // variable, don't call them again" — C's early return on
        // `TclIsVarTraceActive(varPtr)` (tclTrace.c 9.0.4:2513). Per *cell*: a
        // callback writing a different element of the same array is a different
        // `Var` and fires.
        if self.active_var_trace_scopes.borrow().contains(&cell) {
            return Ok(VariableTraceGuestOutcome::Complete);
        }
        let array_active = elem.is_some()
            && self
                .active_var_trace_scopes
                .borrow()
                .contains(&cell.array());
        let whole_array = access.whole_array && !array_active;
        let any = traces.traces.iter().any(|trace| {
            let whole = trace.elem.is_none();
            (!whole || (access.whole_array && !array_active)) && cell.matches(trace, op)
        });
        drop(traces);
        if !(any || (whole_array && self.env_trace_reaches(home))) {
            self.fire_native_error_variable_trace(home, access, op);
            self.check_entered_native_operation()?;
            let outcome = self.native_precision_trace(home, access, op);
            self.check_entered_native_operation()?;
            return outcome;
        }
        // C aborts the chain on the first callback error for every op *except*
        // unset — "ignore errors in unset traces" (tclTrace.c 9.0.4:2600). An
        // `array` trace's error therefore fails the `array` subcommand.
        let propagate = op != b"unset";
        // Preserve the result object across the callbacks.
        let saved = self.save_native_variable_trace_result(false)?;
        let global_error_flags = self.capture_native_global_error_flags();

        // The cell is marked active for the whole firing, as C marks `varPtr`
        // once on entry and clears it on the way out — not per callback.
        self.active_var_trace_scopes.borrow_mut().push(cell.clone());

        let mut errored = false;
        let mut callback_failure: Option<obj::Owned> = None;
        let op_name = String::from_utf8_lossy(op).into_owned();
        'groups: for whole_array_group in [true, false] {
            if whole_array_group && !whole_array {
                continue;
            }
            // Resolve each group only when Tcl reaches it. A whole-array
            // callback can therefore add or remove the selected element's
            // trace before the element group begins.
            let order: Vec<u64> = self
                .traces
                .borrow()
                .traces
                .iter()
                .rev()
                .filter(|trace| trace.elem.is_none() == whole_array_group)
                .filter(|trace| cell.matches(trace, op))
                .map(|trace| trace.id)
                .collect();
            for id in order {
                // Still registered? A previous callback in this group may
                // have removed it from C's live linked-list walk.
                let Some((cmd, old_style)) = self
                    .traces
                    .borrow()
                    .traces
                    .iter()
                    .find(|trace| trace.id == id)
                    .map(|trace| {
                        (
                            native_variable_observers::Callback::from_trace(trace),
                            trace.old_style,
                        )
                    })
                else {
                    continue;
                };
                let code = match cmd {
                    native_variable_observers::Callback::Native(observer) => self
                        .call_native_variable_observer(
                            observer,
                            reported,
                            access.report_elem.as_deref().unwrap_or(b""),
                            &op_name,
                        ),
                    native_variable_observers::Callback::Script(cmd) => {
                        let op_word = tcl_cmd_core::trace::callback_op_word(&op_name, old_style);
                        let args = self.new_list_object(&[
                            new_string(reported),
                            new_string(access.report_elem.as_deref().unwrap_or(b"")),
                            new_string(op_word.as_bytes()),
                        ]);
                        let mut line = cmd;
                        line.push(b' ');
                        line.extend_from_slice(&obj_bytes(args));
                        drop_fresh(args);
                        self.reset_native_error_objects_before_trace_script();
                        if self.clear_return_options_checked().is_err() {
                            errored = true;
                            break 'groups;
                        }
                        let saved = match self.save_native_variable_trace_result(true) {
                            Ok(saved) => saved,
                            Err(_) => {
                                errored = true;
                                break 'groups;
                            }
                        };
                        let code = match self
                            .native_invocation_dialect()
                            .native_variable_trace_protocol()
                        {
                            Some(protocol) => {
                                self.eval_str(protocol.variable_callback_source(&line))
                            }
                            None => self.report_cmd_error(
                                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                    "variable trace callback source",
                                )
                                .into(),
                            ),
                        };
                        if code != Code::Ok
                            && self
                                .native_invocation_dialect()
                                .native_variable_trace_protocol()
                                .is_some()
                        {
                            callback_failure = Some(obj::Owned::retain(self.result.get()));
                        }
                        if let Some(saved) = saved {
                            let _ = self.restore_native_variable_trace_result(saved);
                        }
                        code
                    }
                };
                // Every callback is a separate reached stage. Keep mandatory
                // trace-scope cleanup, but do not render a Guest failure or
                // run later callbacks after an original Host refusal.
                if self.check_entered_native_operation().is_err() {
                    errored = true;
                    break 'groups;
                }
                if propagate && code == Code::Error {
                    // Capture the callback's error message; stop firing (C
                    // aborts the trace chain on the first error).
                    let msg = callback_failure.as_ref().map_or_else(
                        || self.result_bytes(),
                        |failure| obj_bytes(failure.as_ptr()),
                    );
                    // The callback's original result getter can itself reach
                    // an extension updater. Classify that Host stage before
                    // publishing a Guest message or trace frame.
                    if self.check_entered_native_operation().is_err() {
                        errored = true;
                        break 'groups;
                    }
                    self.traces.borrow_mut().pending_err = Some(msg);
                    if leave_error_message {
                        let mut frame = op.to_vec();
                        frame.extend_from_slice(b" trace on \"");
                        frame.extend_from_slice(reported);
                        if let Some(k) = access.spelling_elem.as_deref() {
                            frame.push(b'(');
                            frame.extend_from_slice(k);
                            frame.push(b')');
                        }
                        frame.push(b'"');
                        self.append_frame_noline(&frame);
                    }
                    errored = true;
                    break 'groups;
                }
                drop(callback_failure.take());
                if self.check_entered_native_operation().is_err() {
                    errored = true;
                    break 'groups;
                }
            }
            // The intrinsic was installed before user registrations: it is
            // last in the whole-variable group, before any element traces.
            if whole_array_group {
                self.fire_native_error_variable_trace(home, access, op);
                if self.check_entered_native_operation().is_err() {
                    errored = true;
                    break 'groups;
                }
            }
            if whole_array_group {
                let outcome = self.native_precision_trace(home, access, op);
                if self.check_entered_native_operation().is_err() {
                    errored = true;
                    break;
                }
                match outcome {
                    Ok(VariableTraceGuestOutcome::Complete) => {}
                    Ok(VariableTraceGuestOutcome::GuestError) => {
                        errored = true;
                        break;
                    }
                    Err(first) => {
                        self.refuse_native_execution(first);
                        errored = true;
                        break;
                    }
                }
            }
            // The `env` trace is the array's oldest, so it runs last among the
            // whole-array traces — after any a script added to `env`, before
            // the element's own (tclsh 8.4-9.1). Re-checked here because a
            // callback may have unset the array.
            if whole_array_group
                && self.env_trace_reaches(home)
                && self.run_env_trace(elem, op)
                && propagate
            {
                self.error(b"no such variable");
                if leave_error_message {
                    self.var_trace_failed(op, reported, access);
                } else {
                    let message = self.result_bytes();
                    self.traces.borrow_mut().pending_err = Some(message);
                }
                errored = true;
                break 'groups;
            }
        }
        let popped = self.active_var_trace_scopes.borrow_mut().pop();
        debug_assert_eq!(popped, Some(cell));
        if !errored {
            self.restore_native_global_error_flags(global_error_flags);
        }
        // Retire callback-owned state before deciding how to settle the
        // independently selected saved chain state. A typed Host refusal must
        // restore that state rather than publish the callback's Guest result.
        drop(callback_failure);
        let refused = self.check_entered_native_operation().is_err();
        if !errored || !leave_error_message || refused {
            if let Some(saved) = saved {
                let _ = self.restore_native_variable_trace_result(saved);
            }
        } else {
            drop(saved);
        }
        // Saved-state restoration/retirement can itself reach an extension
        // free hook. Mandatory cleanup is complete before observing its cause.
        self.check_entered_native_operation()?;
        Ok(if errored {
            VariableTraceGuestOutcome::GuestError
        } else {
            VariableTraceGuestOutcome::Complete
        })
    }

    /// Record a variable trace's failure, whose message is the interp result:
    /// stash it for the aborted access and add the `(<op> trace on "…")`
    /// frame to `errorInfo`.
    fn var_trace_failed(&mut self, op: &[u8], reported: &[u8], access: &TraceAccess) {
        let msg = self.result_bytes();
        self.traces.borrow_mut().pending_err = Some(msg);
        let mut frame = op.to_vec();
        frame.extend_from_slice(b" trace on \"");
        frame.extend_from_slice(reported);
        if let Some(k) = access.spelling_elem.as_deref() {
            frame.push(b'(');
            frame.extend_from_slice(k);
            frame.push(b')');
        }
        frame.push(b'"');
        self.append_frame_noline(&frame);
    }

    /// C's `TclVarErrMsg` tail after a variable trace aborted an access: the
    /// *result* becomes `can't <verb> "<name>": <reason>` and `-errorcode`
    /// becomes `TCL <READ|WRITE> VARNAME` (`tclVar.c` 9.0.4:1472 / :2073),
    /// while `errorInfo` keeps the chain `TclCallVarTraces` already built — the
    /// callback's own trace plus its `(<type> trace on "…")` frame.
    ///
    /// This is why it is not `set_error`: that starts a *fresh* error and would
    /// throw the callback's trace away, leaving `errorInfo` as the bare
    /// `can't set "x": …` line.
    pub(crate) fn var_trace_error(&mut self, name: &[u8], op: &[u8], reason: &[u8]) -> Code {
        self.var_trace_error_using(name, op, reason, None)
    }

    pub(crate) fn var_trace_error_using(
        &mut self,
        name: &[u8],
        op: &[u8],
        reason: &[u8],
        stage: native_variable_names::OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(first) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error(first);
        }

        // C's `TclCallVarTraces` verb table (tclTrace.c 9.0.4:2668-2681). The
        // `-errorcode` is *not* set there but by the access that failed, and
        // only `TclPtrGetVarIdx`/`TclPtrSetVarIdx` do so — `TclCheckArrayTraces`
        // has no such tail, so an `array` trace error keeps whatever
        // `-errorcode` the callback left (tclsh: `NONE` for a bare `error`).
        let (verb, word): (&[u8], Option<&[u8]>) = match op {
            b"read" => (b"read", Some(b"READ")),
            b"array" => (b"trace array", None),
            _ => (b"set", Some(b"WRITE")),
        };
        let mut msg = b"can't ".to_vec();
        msg.extend_from_slice(verb);
        msg.extend_from_slice(b" \"");
        msg.extend_from_slice(name);
        msg.extend_from_slice(b"\": ");
        msg.extend_from_slice(reason);
        self.set_result_bytes(&msg);
        if self.check_original_c_name_stage(stage).is_err() {
            return Code::Error;
        }
        if let Some(word) = word {
            let mut code = b"TCL ".to_vec();
            code.extend_from_slice(word);
            code.extend_from_slice(b" VARNAME");
            if stage.is_some() {
                let Ok(scope) = native_operation_currency::NativeOperationScope::enter(self) else {
                    return Code::Error;
                };
                if self
                    .replace_native_error_code_using(scope.currency(), &code, stage)
                    .is_err()
                {
                    return Code::Error;
                }
            } else {
                self.replace_native_error_code(&code);
            }
            if self.check_original_c_name_stage(stage).is_err() {
                return Code::Error;
            }
            let mut exc = self.exc.borrow_mut();
            exc.code = code;
            exc.code_explicit = false;
        }
        Code::Error
    }

    /// Fire `name`'s `array` traces — C's `TclCheckArrayTraces`, which every
    /// `array` subcommand reaches through `LocateArray` (tclVar.c:330-350).
    /// `Some(Code::Error)` when a callback errored and the subcommand must
    /// fail with `can't trace array "name": <msg>` (`LocateArray` passes
    /// `leaveErrMsg` 1), else `None`.
    ///
    /// C gates on `TclIsVarArray(varPtr) || TclIsVarUndefined(varPtr)`, so an
    /// `array` trace fires for an array or for a variable that does not exist
    /// yet, and never for a scalar — nor for an array *element*, which is not
    /// an array however it was spelled or aliased. Ordinary element reads and
    /// writes do not fire it at all: it is the `array` command's own hook.
    pub(crate) fn fire_array_trace(&mut self, name: &[u8]) -> Option<Code> {
        if !self.traces_may_observe(name) {
            return None;
        }
        let (base, elem) = match self.variable_name_parts(name) {
            Ok(parts) => parts,
            Err(_) => return Some(Code::Error),
        };
        if elem.is_some() || (!self.var_is_array(&base) && self.var_exists(&base)) {
            return None;
        }
        match self.fire_var_trace(&base, None, b"array") {
            Ok(VariableTraceGuestOutcome::Complete) => return None,
            Ok(VariableTraceGuestOutcome::GuestError) => {}
            Err(first) => return Some(self.refuse_native_execution(first)),
        }
        let msg = self
            .traces
            .borrow_mut()
            .pending_err
            .take()
            .unwrap_or_default();
        Some(self.var_trace_error(&base, b"array", &msg))
    }

    /// Fire `var`'s `op` traces; on a read/write callback error return the
    /// access-aborting message, already wrapped as `can't read/set "var": <msg>`
    /// (`TclCallVarTraces`), else `None`. The `Traces::fire` engine (`state_traits.rs`)
    /// — it keeps the trace internals (the firing guard, `pending_err`, the
    /// per-op wrapping) here. The selected trace coordinator decides which
    /// operations propagate Guest errors; an original Host refusal is always
    /// the outer error and reaches no Guest message formatting.
    pub(crate) fn fire_var_traces_for(
        &mut self,
        var: &[u8],
        op: &[u8],
    ) -> Result<Option<Vec<u8>>, tcl_runtime_api::NativeExecutionError> {
        self.check_entered_native_operation()?;
        let Ok((base, elem)) = self.variable_name_parts(var) else {
            self.check_entered_native_operation()?;
            return Ok(None);
        };
        if !self
            .fire_var_trace(&base, elem.as_deref(), op)?
            .is_guest_error()
        {
            return Ok(None);
        }
        let raw = self
            .traces
            .borrow_mut()
            .pending_err
            .take()
            .unwrap_or_default();
        // The user-facing verb: a write trace reports `can't set` (C's wording).
        let verb: &[u8] = if op == b"read" { b"read" } else { b"set" };
        let mut m = b"can't ".to_vec();
        m.extend_from_slice(verb);
        m.extend_from_slice(b" \"");
        m.extend_from_slice(var);
        m.extend_from_slice(b"\": ");
        m.extend_from_slice(&raw);
        Ok(Some(m))
    }
}
