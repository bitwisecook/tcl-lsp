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

//! What generated code enters and leaves: compiled activations, the procedure frame, and its indexed local slots.

use super::limits::NATIVE_EVAL_DEPTH_LIMIT;
use super::{trace_native_depth_refusal, Code, Interp};
use crate::namespace::NsId;
use crate::obj::TclObj;

impl Interp {
    /// Whether an already entered compiled body has room for one ABI activation.
    pub(crate) fn codegen_body_has_invoke_headroom(&self) -> bool {
        !NATIVE_EVAL_DEPTH_LIMIT.exceeded(self.eval_depth.get().saturating_add(1))
    }

    /// Enter a **compiled activation** — the eval-loop activation a generated
    /// function or ABI dispatch stands in for.
    ///
    /// The eval loop's outermost-eval rule (`eval_script_mode`, depth 0) is what
    /// publishes an uncaught error's trace and drains the background-error
    /// queue. Compiled code that dispatches a command without entering that loop
    /// therefore runs at depth 0, and any command that evaluates a body — `catch`
    /// above all — sees the rule fire *inside* its body, resetting the exception
    /// state before it can read `error_code()`. Holding an activation for the
    /// span of compiled work restores the invariant that interpreted Tcl always
    /// has: the enclosing activation is depth ≥ 1, so only the true outermost
    /// completion publishes.
    ///
    /// Returns `false` — with the interpreter's error set, and **no** activation
    /// entered, so the caller must not leave one — when the activation would
    /// exceed the native nesting bound. Pair every `true` with exactly one
    /// [`codegen_activation_leave`](Self::codegen_activation_leave).
    pub(crate) fn codegen_activation_enter(&mut self) -> bool {
        if !self.evaluation_is_live() {
            return false;
        }
        if NATIVE_EVAL_DEPTH_LIMIT.exceeded(self.eval_depth.get() + 1) {
            #[cfg(debug_assertions)]
            trace_native_depth_refusal(
                "codegen-activation",
                self.eval_depth.get(),
                self.native_dispatch_depth.get(),
            );
            self.error(b"too many nested evaluations (infinite loop?)");
            return false;
        }
        self.reset_outermost_native_error();
        if self.uses_jim_error_stack() {
            self.jim_error_stack.borrow_mut().mark_reset();
        }
        self.eval_depth.set(self.eval_depth.get() + 1);
        true
    }

    /// Leave a compiled activation entered by
    /// [`codegen_activation_enter`](Self::codegen_activation_enter), applying the
    /// outermost-eval rule with `code` as the activation's completion.
    ///
    /// This is the same two-step tail `eval_script_mode` runs after decrementing
    /// the depth — publish an uncaught error's trace to
    /// `::errorInfo`/`::errorCode`, then drain the background-error queue — so
    /// the policy lives in one place and a compiled statement at the true top
    /// level leaves exactly the error state its interpreted twin would.
    pub(crate) fn codegen_activation_leave(&mut self, code: Code) {
        self.codegen_activation_leave_unpublished();
        self.finish_outermost_eval(code);
    }

    pub(super) fn codegen_activation_leave_unpublished(&self) {
        self.eval_depth.set(self.eval_depth.get().saturating_sub(1));
    }

    /// Enter a generated procedure body using the ordinary Tcl variable frame.
    pub(crate) fn pop_native_call_frame(&mut self) -> Option<NsId> {
        self.release_native_procedure_execution();
        self.clean_current_jim_local_commands();
        let departed = self.frames.borrow_mut().take_frame_for_pop();
        let (namespace, owners) = departed?;
        let storage = owners.release();
        self.frames.borrow_mut().recycle_jim_storage(storage);
        Some(namespace)
    }

    pub(crate) fn retain_native_jim_frame_namespace(
        &mut self,
        namespace: NsId,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        if self.invocation_string_protocol()
            != Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            return Ok(());
        }
        let _context = self.native_jim_object_context()?;
        let owner = self
            .namespaces
            .borrow_mut()
            .take_jim_namespace_owner(namespace)
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim activation namespace object",
            ))?;
        self.frames
            .borrow_mut()
            .retain_jim_activation_objects(None, None, owner);
        Ok(())
    }

    pub(crate) fn codegen_frame_push(&mut self) {
        let ns = self.current_ns.get();
        self.enter_namespace_activation(ns);
        self.frames.borrow_mut().push(ns);
        if let Err(error) = self.retain_native_jim_frame_namespace(ns) {
            self.report_cmd_error(error.into());
        }
    }

    /// Leave a generated procedure body and restore its caller's namespace.
    pub(crate) fn codegen_frame_pop(&mut self) {
        self.clean_current_jim_local_commands();
        let popped = {
            let mut frames = self.frames.borrow_mut();
            let popped = frames.take_frame_for_pop();
            self.current_ns.set(frames.frame_ns(frames.current_level()));
            popped
        };
        let popped = popped.map(|(namespace, owners)| {
            let storage = owners.release();
            self.frames.borrow_mut().recycle_jim_storage(storage);
            namespace
        });
        self.leave_namespace_activation(popped);
    }

    /// Associate an indexed generated local with its name-addressable Tcl cell.
    pub(crate) fn codegen_bind_slot(
        &self,
        slot: usize,
        name: &[u8],
    ) -> Result<(), tcl_cmd_core::CmdError> {
        self.frames.borrow_mut().bind_compiled_slot(slot, name);
        self.refresh_native_local_name_table()
    }

    /// Resolve a generated local index to its Tcl-visible name.
    pub(crate) fn codegen_slot_name(&self, slot: usize) -> Option<Vec<u8>> {
        self.frames
            .borrow()
            .compiled_slot_name(slot)
            .map(<[u8]>::to_vec)
    }

    /// [`var_is_traced`](Self::var_is_traced) for a compiled slot, answered from
    /// the cell's own cached bit when it is current for this interpreter's
    /// variable-trace epoch.
    pub(crate) fn codegen_slot_is_traced(&self, slot: usize) -> bool {
        let epoch = self.var_trace_epoch.get();
        if let Some(cached) = self.frames.borrow().compiled_slot_trace_flag(slot, epoch) {
            return cached;
        }
        let Some(name) = self.codegen_slot_name(slot) else {
            return false;
        };
        let traced = self.var_is_traced(&name);
        self.frames
            .borrow()
            .set_compiled_slot_trace_flag(slot, epoch, traced);
        traced
    }

    /// The value a generated local slot addresses, by the **O(1) cell path**.
    ///
    /// Taken only when nothing can observe the read differently from a plain
    /// cell load: the cell holds a scalar (not a link, which crosses tables and
    /// is the coordinator's walk), and the interpreter has no variable traces at
    /// all. `None` means "no fast path" — an unbound slot, an undefined or
    /// linked cell, or a traced interpreter — and the caller takes the name path,
    /// which owns the link walk, the trace firing, and the error text.
    pub(crate) fn codegen_slot_scalar(&self, slot: usize) -> Option<*mut TclObj> {
        if self.has_variable_traces() {
            return None;
        }
        match &*self.frames.borrow().compiled_slot_var(slot)? {
            crate::frame::Var::Scalar(value) => Some(*value),
            _ => None,
        }
    }
}
