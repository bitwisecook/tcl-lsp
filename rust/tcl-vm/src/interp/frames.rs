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

//! Call frames and flows: pushing and destroying a call frame, procedure owners, coroutine flow swaps, recursion depth, the `source` script stack, evaluation at a level and tailcall requests.

#[cfg(test)]
use super::NamespacePath;
use super::errors::ErrorLogged;
use super::selected_frame::SelectedFrameRestore;
use super::{
    ParkedFlow, Vm, err, namespace_context, native_operation_currency, oo_cleanup, variable_cleanup,
};
use crate::command::{Command, ProcDef};
use crate::error::TclError;
use crate::frame::CallFrame;
use crate::value::Value;
use crate::vars::VarState as Local;
use std::rc::Rc;
use tcl_core_types::{NameBytes, RecursionLimit};
use tcl_runtime_api::{Code, Completion, Namespaces, NsId, ROOT_NS};

/// Native-stack safety net for `TclOO` method dispatch (`cmd_oo.rs::run_step`).
///
/// Unlike ordinary proc-to-proc calls, which the trampoline
/// ([`Tick::Call`](crate::exec)) runs without growing the *native* Rust
/// stack (each nested call is a new activation pushed onto an explicit
/// `Vec`, not a recursive Rust call), a `$obj method`/`my method`/`next`
/// dispatch is a genuine, undelegated native call chain: `run_step` (reached
/// via `oo_dispatch` → `oo_invoke` for `$obj method`, `cmd_my` →
/// `oo_invoke` for `my method`, or directly for `next`/`nextto`) →
/// [`Vm::oo_run_method`] → [`Vm::run_activation`] (a fresh native call, not
/// a trampoline push) → … back to `run_step` for a nested method call. So a
/// recursive method (or a long `next` chain) consumes real native stack
/// per level, unlike a recursive `proc`. `run_step` is guarded rather than
/// `oo_dispatch` because `my method` and `next`/`nextto` — the two most
/// common ways a method calls another method recursively — reach it
/// directly, bypassing `oo_dispatch` entirely.
/// Measured directly (a `method go {n} { … return [my go [expr {$n-1}]]
/// }`-style self-recursive method): SIGABRT between depth 45 and 48 on a
/// 2 MiB thread (`cargo test`'s per-test default, and Tokio's worker-thread
/// default — the same floor native dispatch recursion is calibrated
/// against). `tcl-vm` is also consumed from a WASM host with no
/// stack-size guarantee (`tcl-vm-wasm`), so this must hold on a small
/// ambient stack, not just a generously-sized one.
///
/// This is a real, deliberate compatibility gap versus C Tcl: an ordinary
/// recursive `proc` is bounded by [`RECURSION_LIMIT`] (1000, matching
/// `interp recursionlimit`'s default) with no native-stack cost at all,
/// while a recursive `TclOO` method hits this much lower bound instead. The
/// architecturally correct fix is routing method dispatch through the same
/// trampoline ordinary calls use (so it becomes as cheap as a proc call);
/// that is a substantially larger change to the dispatch/activation engine
/// than this counter, which is a mitigation, not that fix — an uncatchable
/// process abort is strictly worse than a catchable error arriving earlier
/// than tclsh's own limit would. 20 leaves better than 2x margin under the
/// measured crash floor.
pub(super) const OO_DISPATCH_DEPTH_LIMIT: RecursionLimit = RecursionLimit(20);

impl ParkedFlow {
    pub(crate) fn original_tailcall_currencies(
        &self,
    ) -> Vec<native_operation_currency::NativeOperationCurrency> {
        self.frames
            .iter()
            .filter_map(|frame| frame.tailcall.as_ref())
            .map(|request| request.original_scope.currency().clone())
            .collect()
    }

    /// The saved tail omits the actual shared global entry of a fresh flow.
    pub(crate) fn current_namespace_execution_context(
        &self,
    ) -> namespace_context::NamespaceExecutionContext {
        self.namespaces.current_namespace().unwrap_or(
            namespace_context::NamespaceExecutionContext::Resolved(ROOT_NS),
        )
    }

    #[cfg(test)]
    pub(crate) fn current_namespace_token(&self) -> NsId {
        self.current_namespace_execution_context()
            .resolved_token()
            .expect("test requires a resolved parked namespace")
    }
}

impl Vm {
    /// Authored framework bookkeeping uses retained physical cells. Cleanup
    /// neither calls user traces after host refusal nor recreates deleted cells.
    pub(crate) fn with_timer_metadata<R>(
        &mut self,
        event: &Value,
        rule: &Value,
        run: impl FnOnce(&mut Vm) -> R,
    ) -> R {
        let mut saved = Vec::new();
        for (name, value) in [
            ("::itest::_current_rule", rule.clone()),
            ("::itest::_executing_rule", Value::int(1)),
            ("::itest::current_event", event.clone()),
        ] {
            if let Some(resolved) = self.resolve_var_from(name, 0)
                && let Some(id) = resolved.id
                && let Some(old) = self.read_variable_contents(&resolved)
            {
                self.var_arena.pin(id);
                self.var_arena.replace_state(id, Local::Scalar(value));
                saved.push((name, id, old));
            }
        }
        let result = run(self);
        for (name, id, old) in saved {
            if self
                .resolve_var_from(name, 0)
                .and_then(|resolved| resolved.id)
                == Some(id)
            {
                self.var_arena.replace_state(id, Local::Scalar(old));
            }
            self.var_arena.unpin(id);
        }
        result
    }

    pub(crate) fn retained_frame_identity(
        &self,
        root: bool,
    ) -> Rc<crate::frame::ActivationIdentity> {
        Rc::clone(&self.frames[if root { 0 } else { self.frames.len() - 1 }].activation)
    }

    pub(crate) fn retained_frame_level(
        &self,
        identity: &Rc<crate::frame::ActivationIdentity>,
    ) -> Option<usize> {
        self.frames
            .iter()
            .position(|frame| Rc::ptr_eq(&frame.activation, identity))
    }

    pub(crate) fn eval_retained_frame(
        &mut self,
        level: usize,
        source: &Value,
    ) -> Result<Completion<Value>, tcl_runtime_api::NativeExecutionError> {
        self.host_execution_depth += 1;
        let completion = self.eval_value_at_level(level, source);
        self.host_execution_depth -= 1;
        self.finish_host_execution(completion)
    }
}

impl Vm {
    // Frames.

    /// Install the original shared native `TclOO` unknown-handler header.
    pub(crate) fn install_native_oo_unknown(&mut self, original: &Value) {
        for name in ["::oo::define", "::oo::objdefine"] {
            if let Ok(Some(namespace)) = Namespaces::find_namespace_bytes_checked(
                self,
                self.current_ns_id(),
                name.as_bytes(),
            ) {
                self.ns_unknown_set_at(namespace, original.clone(), 1);
            }
        }
    }

    /// Issued identity of the actual selected variable activation.
    pub(crate) fn native_oo_variable_activation(&self) -> u64 {
        self.frames
            .last()
            .expect("actual variable frame")
            .activation
            .serial
    }

    /// Actual variable-frame currency used only to authenticate a reached OO context.
    pub(crate) fn native_oo_frame_identity(&self, level: usize) -> Option<(u64, NsId)> {
        let frame = self.frames.get(level)?;
        Some((frame.activation.serial, frame.ns.resolved_token()?))
    }

    /// Counted names from the actual definition namespace command table.
    pub(crate) fn native_oo_definition_command_names(&self) -> Vec<NameBytes> {
        self.names_directly_in_token(self.current_ns_id(), false)
    }

    /// Whether every word `mask` selects (one bit per word index, counting the
    /// command word) names a compiled local of the current procedure frame:
    /// C's `TclFindCompiledLocal` reaches only those, so a release compiles a
    /// command binding such a word only where it is one.
    pub(crate) fn frame_compiled_locals_named(&self, mask: u8, words: &[Value]) -> bool {
        if mask == 0 {
            return true;
        }
        let Some(frame) = self.frames.last() else {
            return false;
        };
        if frame.compiled_locals.is_empty() {
            return false;
        }
        (0..u8::BITS).filter(|bit| mask >> bit & 1 == 1).all(|bit| {
            usize::try_from(bit)
                .ok()
                .and_then(|index| words.get(index))
                .is_some_and(|word| {
                    let name = word.string_bytes();
                    frame
                        .local_bindings()
                        .any(|(local, _)| local.as_bytes() == &*name)
                        && frame
                            .compiled_locals
                            .iter()
                            .any(|(local, _)| local.as_bytes() == &*name)
                })
        })
    }

    /// The word that invoked the current procedure frame (`info level 0`'s
    /// first word), which C's `MakeProcError` names the procedure by.
    pub(crate) fn current_invoking_word(&self) -> Option<String> {
        self.frames
            .last()
            .and_then(|frame| frame.call_argv.first())
            .map(|word| word.to_str().to_string())
    }

    pub(crate) fn current_level(&self) -> usize {
        self.frames.len() - 1
    }

    pub(crate) fn push_call_frame(
        &mut self,
        proc_name: Option<String>,
        call_argv: Vec<Value>,
    ) -> usize {
        let level = self.frames.len();
        let mut frame = CallFrame::new(
            level,
            self.current_namespace_execution_context(),
            proc_name,
            call_argv,
        );
        if self.uses_native_jim_lookup() {
            frame.jim_storage = self.jim_storage.acquire();
        }
        self.frames.push(frame);
        self.recursion_depth += 1;
        level
    }

    /// Retain the original namespace object owned by the entered Jim procedure frame.
    pub(crate) fn install_jim_procedure_namespace_owner(
        &mut self,
        namespace: Option<NsId>,
        original: Value,
    ) {
        *self
            .frames
            .last()
            .expect("entered procedure frame")
            .jim_namespace
            .borrow_mut() = Some((namespace, original));
    }

    /// Retain original body and parameter objects for the selected procedure frame.
    pub(crate) fn install_procedure_object_owners(
        &mut self,
        body: Value,
        parameters: Option<Value>,
    ) {
        let frame = self.frames.last_mut().expect("procedure frame exists");
        frame.procedure_body = Some(body);
        frame.procedure_parameters = parameters;
    }

    /// Attach the persistent bindings before the selected formal plan writes arguments.
    pub(crate) fn install_procedure_statics(
        &mut self,
        statics: Option<Rc<crate::vars::StaticVariables>>,
    ) {
        self.frames
            .last_mut()
            .expect("procedure frame exists")
            .statics = statics;
    }

    pub(crate) fn pop_call_frame(&mut self) {
        let cleanup = oo_cleanup::OriginalOoCleanup::enter(self);
        if let Err(error) = cleanup.pop_call_frame(self) {
            let _ = crate::command::completion_from_cmd_error(self, error);
        }
    }

    /// Install the actual command binding after successful procedure entry.
    pub(crate) fn install_native_procedure_binding(
        &mut self,
        binding: crate::command::NativeProcedureCommand,
    ) {
        self.frames
            .last_mut()
            .expect("entered procedure frame")
            .native_procedure_binding = Some(binding);
    }

    /// Acquire the execution role only after all formal binding has succeeded.
    pub(crate) fn acquire_native_procedure_execution(&mut self, procedure: &Rc<ProcDef>) {
        let frame = self.frames.last_mut().expect("bound procedure frame");
        assert!(frame.native_procedure_execution.is_none());
        frame.native_procedure_execution =
            Some(crate::command::NativeProcedureReference::acquire(procedure));
    }

    pub(crate) fn release_native_procedure_execution(&mut self) {
        let reference = self
            .frames
            .last_mut()
            .and_then(|frame| frame.native_procedure_execution.take());
        drop(reference);
    }

    /// Destroy every live activation above `len`, running the same variable
    /// lifecycle as an ordinary return. Error unwinding and `uplevel` cleanup
    /// must not bypass trace firing, link unbinding, or arena collection by
    /// truncating the frame vector directly.
    pub(super) fn drain_call_frames_to(&mut self, len: usize) {
        while self.frames.len() > len {
            self.pop_call_frame();
            self.pop_ns();
        }
    }

    /// Settle any Tcl call frames left in a coroutine flow whose bytecode
    /// activation stack has completed. The flow is still installed here, so
    /// namespace activation counts return to their exact tokens and can trigger
    /// deferred namespace teardown before the resumer's flow is restored.
    pub(crate) fn finish_coroutine_flow(&mut self) {
        self.drain_call_frames_to(1);
    }

    /// Fire `unset` variable traces on the current frame's genuine locals just
    /// before the frame is destroyed — C Tcl unsets a call frame's locals when
    /// the frame is deleted, firing their unset traces (coroutine-4.1/4.2, and
    /// the general proc-return case). Links are skipped: they alias a variable
    /// owned by another frame, whose trace fires when *that* frame goes. The
    /// fired traces are then dropped, since their variables no longer exist.
    /// Guarded on `var_traces` being non-empty, so it is free in the common case.
    pub(super) fn destroy_call_frame(
        &mut self,
        mut frame: CallFrame,
        original: &oo_cleanup::OriginalOoCleanup,
    ) {
        let operation = native_operation_currency::NativeOperationScope::enter(self);
        if let Err(error) = &operation {
            let _ = crate::command::completion_from_cmd_error(self, error.clone());
        }
        let storage = frame.jim_storage.clone();
        let locals = std::mem::take(&mut frame.jim_local_commands);
        let cleaned = original.in_original(self, |vm| vm.clean_jim_local_commands(locals));
        if let Err(error) = cleaned.and_then(std::convert::identity) {
            let _ = crate::command::completion_from_cmd_error(self, error);
        }
        frame.release_native_jim_owners();
        for (name, raw) in frame.into_local_bindings() {
            let retired = original.in_original(self, |vm| {
                if vm.var_arena.has_static_refs(raw) {
                    let variables = variable_cleanup::OriginalVariableCleanup::enter(vm);
                    let _ = variables.retire(vm, crate::vars::VariableRetirement::unbind(raw));
                } else if matches!(
                    vm.var_arena.get(raw).map(crate::vars::VarCell::state),
                    Some(Local::Link(_) | Local::NameLink(_))
                ) {
                    let variables = variable_cleanup::OriginalVariableCleanup::enter(vm);
                    variables.purge_binding(vm, raw);
                } else if !vm.retire_plain_local(raw) {
                    vm.destroy_owned_variable(&name, raw, None);
                }
            });
            if let Err(error) = retired {
                let _ = crate::command::completion_from_cmd_error(self, error);
            }
        }
        let retired = original.in_original(self, |vm| {
            vm.retire_released_native_alias_entries();
            if vm.uses_native_jim_lookup() {
                vm.jim_storage.recycle(storage);
            }
        });
        if let Err(error) = retired {
            let _ = crate::command::completion_from_cmd_error(self, error);
        }
        let checked = operation.and_then(|scope| scope.check(self));
        if let Err(error) = checked {
            let _ = crate::command::completion_from_cmd_error(self, error);
        }
    }

    /// Hand the error trace a coroutine's flow built up to its resumer, once
    /// that flow has finished with `code` and the resumer's flow is back; only
    /// an error carries one. C keeps one `errorInfo` per interp, so the trace
    /// crosses the coroutine boundary like a proc boundary: the resume command
    /// then logs its own `invoked from within` frame.
    pub(crate) fn adopt_flow_error_trace(&mut self, code: Code, p: &mut ParkedFlow) {
        if code != Code::Error || self.uses_jim_error_stack() {
            return;
        }
        self.native_errors.error_info = p.error_info.take();
        // The trace's native errorInfo object travels with its bytes; a flow
        // that built none seeds it from them.
        self.native_errors.native_error_info = p.native_error_info.take();
        self.native_errors.native_error_info_len = p.native_error_info_len;
        if self.native_errors.native_error_info.is_none()
            && let Some(info) = self.native_errors.error_info.clone()
        {
            self.native_errors.native_error_info_len = 0;
            self.seed_native_error_result_object(&info);
        }
        self.native_errors.error_line = p.error_line;
        // C keeps one interpreter error code, so the body's code reaches the
        // resumer with its trace.
        self.native_errors.primitive_error_code = p.primitive_error_code.take();
        std::mem::swap(&mut self.native_errors.error_stack, &mut p.error_stack);
        let recipe = self
            .actual_native_invocation_dialect()
            .native_error_objects_protocol();
        self.native_errors.error_stack.configure(recipe);
        self.native_errors.error_logged = ErrorLogged::No;
        self.update_native_error_info();
    }

    /// Fire `unset` traces on a suspended coroutine's parked locals as the
    /// coroutine is torn down: its frozen frames are about to vanish, so swap the
    /// parked flow in, unwind it frame by frame (each `pop_call_frame` fires that
    /// frame's unset traces), then swap the caller's flow back (`parked` is left
    /// emptied for the caller to drop). Matches C Tcl unsetting a deleted
    /// coroutine's variables (coroutine-4.3).
    pub(crate) fn fire_parked_unset_traces(
        &mut self,
        parked: &mut ParkedFlow,
        activations: &mut [crate::exec::Frame],
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let scope = native_operation_currency::NativeOperationScope::enter_native(self);
        self.swap_flow(parked);
        self.restore_parked_execution_frames(activations);
        while self.frames.len() > 1 {
            self.pop_call_frame();
            // Releasing the parked call frame also releases the exact namespace
            // token it retained. This can complete a deferred namespace
            // teardown; dropping only the display stack strand left that token
            // and every command it owned indefinitely.
            self.pop_ns();
        }
        self.swap_flow(parked);
        scope?.check_native(self)
    }

    /// A caught return can retain a tailcall request until a later coroutine yield.
    pub(crate) fn park_original_flow_tailcalls(&mut self) {
        for frame in &mut self.frames {
            if let Some(request) = &mut frame.tailcall {
                request.original_scope.park();
            }
        }
    }

    pub(crate) fn resume_original_flow_tailcalls(
        &mut self,
    ) -> Result<Vec<native_operation_currency::NativeOperationScope>, tcl_cmd_core::CmdError> {
        let mut scopes = Vec::new();
        for level in 0..self.frames.len() {
            if let Some(mut request) = self.frames[level].tailcall.take() {
                let reached = request.original_scope.resume(self);
                self.frames[level].tailcall = Some(request);
                scopes.push(reached?);
            }
        }
        Ok(scopes)
    }

    /// The current call-nesting depth (proc recursion bound).
    pub(crate) fn recursion_depth(&self) -> usize {
        self.recursion_depth
    }

    /// Enter one level of `TclOO` method-dispatch recursion — see
    /// [`OO_DISPATCH_DEPTH_LIMIT`]'s doc comment. Checked
    /// before incrementing. The original OO cleanup owner restores the captured
    /// physical interpreter's depth after the method and mandatory retirement.
    pub(crate) fn enter_oo_dispatch(&mut self) -> Result<(), Completion<Value>> {
        if OO_DISPATCH_DEPTH_LIMIT
            .exceeded(u32::try_from(self.oo_dispatch_depth + 1).unwrap_or(u32::MAX))
        {
            return Err(err("too many nested evaluations (infinite loop?)"));
        }
        self.oo_dispatch_depth += 1;
        Ok(())
    }

    /// This interp's recursion bound (`interp recursionlimit`).
    pub(crate) fn recursion_limit(&self) -> usize {
        self.recursion_limit
    }

    /// `interp recursionlimit` get / set on this interp. `newlimit` is the
    /// optional new limit, read as C 9's `TclGetWideIntFromObj` reads it;
    /// returns the resulting limit or the error the caller should raise.
    pub(crate) fn recursion_limit_apply(
        &mut self,
        newlimit: Option<&Value>,
    ) -> Result<i64, TclError> {
        match newlimit {
            None => Ok(i64::try_from(self.recursion_limit).unwrap_or(i64::MAX)),
            Some(word) => {
                let n = tcl_syntax::value::ValueOps::as_int(self, word)?;
                if n <= 0 {
                    return Err(TclError::with_error_code(
                        "recursion limit must be > 0",
                        "TCL OPERATION INTERP BADLIMIT",
                    ));
                }
                self.recursion_limit = usize::try_from(n).unwrap_or(usize::MAX);
                Ok(n)
            }
        }
    }

    /// Push the path of a file being `source`d; the matching [`Self::pop_script`]
    /// restores the previous one. Drives `info script`.
    pub(crate) fn push_script(&mut self, path: String) {
        self.script_stack.push(path);
    }

    /// Pop the current `source` path (see [`Self::push_script`]).
    pub(crate) fn pop_script(&mut self) {
        self.script_stack.pop();
    }

    /// The path of the file currently being `source`d (`info script`); empty
    /// when evaluation is not inside a `source`.
    pub(crate) fn current_script(&self) -> &str {
        self.script_stack.last().map_or("", String::as_str)
    }

    /// Evaluate the original source object in an explicitly selected frame.
    /// Byte materialisation and analytical source admission remain separate;
    /// unsupported source bytes retain a host refusal outside guest completion.
    pub(crate) fn eval_value_at_level(
        &mut self,
        target: usize,
        source: &Value,
    ) -> Completion<Value> {
        let selected = match self.select_execution_frame(target) {
            Ok(selected) => selected,
            Err(message) => return err(message),
        };
        self.eval_value_in_selected_frame(source, selected)
    }

    pub(crate) fn eval_value_at_level_checked(
        &mut self,
        target: usize,
        source: &Value,
    ) -> Completion<Value> {
        let selected = match self.select_execution_frame_checked(target) {
            Ok(selected) => selected,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        self.eval_value_in_selected_frame(source, selected)
    }

    fn eval_value_in_selected_frame(
        &mut self,
        source: &Value,
        selected: Option<SelectedFrameRestore>,
    ) -> Completion<Value> {
        let completion = match self.eval_original_script_value(
            source,
            tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
            source.source_location(),
        ) {
            Ok(completion) => completion,
            Err(error) => crate::command::completion_from_tcl_error(self, error),
        };
        self.restore_execution_frame(selected);
        completion
    }

    /// Exchange the live per-flow execution context with `p` — the coroutine
    /// context switch (`cmd_coro::resume`). Its own inverse: two calls restore
    /// the original. The **shared** global frame (`frames[0]`) and global
    /// namespace (`ns_stack[0]`/`ns_id_stack[0]`) stay in place — only the
    /// supra-global tails move — so globals, `uplevel #0`, and `::x` stay
    /// coherent across flows and each coroutine roots at frame level 1.
    /// Everything else (error trace, script stack, OO call/def stacks) swaps
    /// wholesale. Registries, channels, commands, etc. are shared and untouched
    /// (they are not part of a flow).
    ///
    /// The flow owns a split-off/restore of `frames` +
    /// both namespace stacks + `recursion_depth`.
    pub(crate) fn swap_flow(&mut self, p: &mut ParkedFlow) {
        // Frames / namespace stacks: exchange the tails above the shared
        // global entry.
        let mut ftail = self.frames.split_off(1);
        std::mem::swap(&mut ftail, &mut p.frames);
        self.frames.append(&mut ftail);
        let ns_cut = 1.min(self.resolution_stacks.ns_stack.len());
        let mut namespaces = self.resolution_stacks.take_tail(ns_cut);
        std::mem::swap(&mut namespaces, &mut p.namespaces);
        self.resolution_stacks.append_tail(namespaces);
        std::mem::swap(&mut self.timer_anchor, &mut p.timer_anchor);
        // Scalars / stacks: plain exchange.
        std::mem::swap(&mut self.ns_script_frames, &mut p.ns_script_frames);
        std::mem::swap(&mut self.recursion_depth, &mut p.recursion_depth);
        std::mem::swap(&mut self.native_errors.error_info, &mut p.error_info);
        std::mem::swap(
            &mut self.native_errors.native_error_info,
            &mut p.native_error_info,
        );
        std::mem::swap(
            &mut self.native_errors.native_error_info_len,
            &mut p.native_error_info_len,
        );
        std::mem::swap(
            &mut self.native_errors.native_error_result,
            &mut p.native_error_result,
        );
        std::mem::swap(
            &mut self.native_errors.native_return_options,
            &mut p.native_return_options,
        );
        std::mem::swap(
            &mut self.native_errors.native_c_return_state,
            &mut p.native_c_return_state,
        );
        std::mem::swap(
            &mut self.native_errors.native_error_legacy_copy,
            &mut p.native_error_legacy_copy,
        );
        std::mem::swap(
            &mut self.native_errors.c84_global_code_set,
            &mut p.c84_global_code_set,
        );
        std::mem::swap(
            &mut self.native_errors.primitive_error_code,
            &mut p.primitive_error_code,
        );
        std::mem::swap(&mut self.native_errors.error_logged, &mut p.error_logged);
        std::mem::swap(&mut self.native_errors.error_stack, &mut p.error_stack);
        let recipe = self
            .actual_native_invocation_dialect()
            .native_error_objects_protocol();
        self.native_errors.error_stack.configure(recipe);
        std::mem::swap(&mut self.jim_errors, &mut p.jim_errors);
        std::mem::swap(&mut self.native_errors.error_line, &mut p.error_line);
        std::mem::swap(&mut self.native_invocation.name, &mut p.invoked_name);
        std::mem::swap(
            &mut self.native_invocation.original_name,
            &mut p.original_invoked_name,
        );
        std::mem::swap(
            &mut self.native_invocation.builtin_identity,
            &mut p.invoked_builtin_identity,
        );
        std::mem::swap(&mut self.script_stack, &mut p.script_stack);
        self.oo.swap_exec(&mut p.oo);
    }

    /// The unqualified names of commands (or, with `procs_only`, just user
    /// procedures) defined **directly** in namespace `canonical` (unrooted; `""`
    /// = global) — the `Namespaces::commands_in`/`procs_in` enumeration, filtering
    /// the flat command map. Direct members only (`foo::sub::x` is not in `foo`).
    #[cfg(test)]
    pub(crate) fn names_directly_in(&self, canonical: &str, procs_only: bool) -> Vec<String> {
        let path = NamespacePath::from_segments(tcl_syntax::naming::key_segments(canonical));
        let namespace = self
            .name_world
            .borrow()
            .ns_intern
            .get(&path)
            .copied()
            .unwrap_or(ROOT_NS);
        self.names_directly_in_token(namespace, procs_only)
            .into_iter()
            .map(|name| {
                name.try_utf8()
                    .expect("Unicode enumeration requires a checked name")
                    .to_owned()
            })
            .collect()
    }

    pub(super) fn names_directly_in_token(
        &self,
        namespace: NsId,
        procs_only: bool,
    ) -> Vec<tcl_core_types::NameBytes> {
        self.commands
            .iter()
            .filter(|(key, command)| self.builtin_command_visible_for_surface(key, command))
            .filter(|(_, c)| !procs_only || matches!(c, Command::Proc(_)))
            .filter_map(|(key, _)| {
                self.command_slot(key)
                    .filter(|slot| slot.namespace == namespace)
                    .map(|slot| slot.simple.clone())
            })
            .collect()
    }

    /// The `ProcDef` for a user proc, if `name` resolves to one (`info body`/`args`).
    #[cfg(test)]
    pub(crate) fn proc_def(&self, name: &str) -> Option<Rc<crate::command::ProcDef>> {
        match self.lookup_command(name) {
            Some(Command::Proc(p)) => Some(p.declaration()),
            _ => None,
        }
    }

    /// Query a procedure using the purpose-selected native byte lookup.
    pub(crate) fn proc_def_bytes(&self, original: &[u8]) -> Option<Rc<crate::command::ProcDef>> {
        let key = self.resolve_command_from_token_bytes(self.current_ns_id(), original, true)?;
        match self.visible_command_at_key(&key)? {
            Command::Proc(procedure) => Some(procedure.declaration()),
            _ => None,
        }
    }

    /// The invocation argv of the frame at absolute `level` (`info level N`).
    pub(crate) fn frame_argv(&self, level: usize) -> Option<Vec<Value>> {
        self.frames.get(level).map(|f| f.call_argv.clone())
    }

    /// The variable names of the current (active) frame — the `Frames::var_names`
    /// enumeration. Genuine locals (scalars, arrays) always; `upvar`/`global`/
    /// `variable` links iff `include_links` (`info vars` lists links by their
    /// local alias, `info locals` does not).
    pub(crate) fn frame_var_names(&self, include_links: bool) -> Vec<String> {
        self.frame_var_name_bytes(include_links)
            .into_iter()
            .map(|name| {
                String::from_utf8(name).expect("byte-valued frame requires byte enumeration")
            })
            .collect()
    }

    pub(crate) fn frame_var_name_bytes(&self, include_links: bool) -> Vec<Vec<u8>> {
        let mut names: Vec<Vec<u8>> = self.frames.last().map_or_else(Vec::new, |f| {
            f.local_bindings()
                .filter(|(_, id)| {
                    matches!(
                        self.var_arena.get(**id).map(crate::vars::VarCell::state),
                        Some(Local::Scalar(_) | Local::Array(_))
                    ) || (include_links
                        && matches!(
                            self.var_arena.get(**id).map(crate::vars::VarCell::state),
                            Some(Local::Link(_) | Local::NameLink(_))
                        ))
                })
                .map(|(name, _)| name.as_bytes().to_vec())
                .collect()
        });
        if include_links
            && let Ok(public) =
                crate::cmd_oo::native_variables::reported_names(self, self.current_level())
        {
            for name in public {
                if !names
                    .iter()
                    .any(|existing| existing.as_slice() == name.as_bytes())
                {
                    names.push(name.as_bytes().to_vec());
                }
            }
        }
        names
    }

    pub(crate) fn schedule_tailcall(
        &mut self,
        request: crate::exec::TailcallReq,
    ) -> Result<(), Completion<Value>> {
        if !self.frame_owns_local_variables(self.frames.len().saturating_sub(1)) {
            return Err(crate::command::err_with_code(
                "tailcall can only be called from a proc, lambda or method",
                "TCL TAILCALL ILLEGAL",
            ));
        }
        let previous = self
            .frames
            .last_mut()
            .expect("procedure frame present")
            .tailcall
            .take();
        drop(previous);
        if !request.words.is_empty() {
            self.frames
                .last_mut()
                .expect("procedure frame present")
                .tailcall = Some(request);
        }

        Ok(())
    }

    pub(crate) fn cancel_original_tailcall(&mut self) -> Result<(), Completion<Value>> {
        if !self.frame_owns_local_variables(self.current_level()) {
            return Err(crate::command::err_with_code(
                "tailcall can only be called from a proc, lambda or method",
                "TCL TAILCALL ILLEGAL",
            ));
        }
        let previous = self
            .frames
            .last_mut()
            .expect("procedure frame present")
            .tailcall
            .take();
        drop(previous);
        Ok(())
    }

    pub(crate) fn take_frame_tailcall(&mut self) -> Option<crate::exec::TailcallReq> {
        self.frames.last_mut()?.tailcall.take()
    }
}
