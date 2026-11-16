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

//! Invoking what a name is bound to: builtins, object commands, Jim evaluation scopes, aliases and parent aliases.

use super::super::commands::Command;
use super::super::native_alias_roles::NativeAliasHeader;
use super::super::{
    native_jim_tailcall, native_operation_currency, native_script, release_all, BuiltinFn, Code,
    ExceptionState, Interp, JimBorrowedInvocation, ObjCommand, OriginalJimEvaluationFrame,
    ALIAS_DISPATCH_DEPTH, CROSS_INTERP_DEPTH,
};
use super::{MAX_ALIAS_DISPATCH_DEPTH, MAX_CROSS_INTERP_DEPTH};
use crate::namespace::GLOBAL;
use crate::obj;
use crate::obj::TclObj;
use core::ffi::c_int;
use std::rc::Rc;
use tcl_runtime_api::jim_error_stack::JimScriptLocation;

/// Restore the diagnostic borrow stack before the caller releases its argv.
/// Coroutine handoff swaps this stack with its evaluation frames; resumption
/// restores the same flow before this native invocation can return or unwind.
struct JimInvocationScope {
    pub(in super::super) interp: Interp,
    pub(in super::super) previous_len: usize,
}

/// Restore the actual original-script command frame before its argv retire.
pub(in super::super) struct JimEvaluationScope {
    pub(in super::super) interp: Interp,
    pub(in super::super) previous_len: usize,
    previous_borrows_len: usize,
}

impl Drop for JimEvaluationScope {
    fn drop(&mut self) {
        self.interp
            .jim_invocation_borrows
            .borrow_mut()
            .truncate(self.previous_borrows_len);
        self.interp
            .jim_evaluation_frames
            .borrow_mut()
            .truncate(self.previous_len);
    }
}

impl Drop for JimInvocationScope {
    fn drop(&mut self) {
        self.interp
            .jim_invocation_borrows
            .borrow_mut()
            .truncate(self.previous_len);
    }
}

impl Interp {
    pub(in super::super) fn invoke_bound(
        &mut self,
        cmd: Command,
        generation: Option<u64>,
        argv: &[*mut TclObj],
    ) -> Code {
        self.invoke_bound_with_jim_reference(cmd, generation, argv, None)
    }

    pub(in super::super) fn invoke_bound_with_jim_reference(
        &mut self,
        cmd: Command,
        generation: Option<u64>,
        argv: &[*mut TclObj],
        selected: Option<native_jim_tailcall::JimCommandReference>,
    ) -> Code {
        self.invoke_bound_with_source(cmd, generation, argv, selected, None)
    }

    pub(super) fn invoke_bound_with_source(
        &mut self,
        cmd: Command,
        generation: Option<u64>,
        argv: &[*mut TclObj],
        selected: Option<native_jim_tailcall::JimCommandReference>,
        source: Option<&Rc<native_script::OriginalSourceInvocation>>,
    ) -> Code {
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                // A refused entry still retires its actual argument worker.
                drop(cmd);
                if let Err(first) = self.retire_pending_native_ensemble_roles() {
                    self.refuse_native_execution(first);
                }
                return Code::Error;
            }
        };
        let mut jim_command = match self.retain_active_jim_command(&cmd, generation, selected) {
            Ok(command) => command,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let code = (|| {
            if self.check_entered_native_operation().is_err() {
                return Code::Error;
            }
            if let Some(source) = source {
                if let Err(error) = source.check(self, argv) {
                    return self.report_cmd_error(error.into());
                }
            }
            if let Err(error) = self.associate_native_jim_arguments(argv) {
                return self.report_cmd_error(error.into());
            }
            if self.uses_jim_error_stack() {
                let mut frames = self.jim_evaluation_frames.borrow_mut();
                if let Some(frame) = frames.last_mut() {
                    frame.command = generation;
                    frame.is_procedure = matches!(cmd, Command::Proc(_));
                }
            }
            let _jim_invocation = self.borrow_jim_invocation(argv, source);
            if self.invocation_string_protocol()
                == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
            {
                let context = match self.native_jim_object_context() {
                    Ok(context) => context,
                    Err(error) => return self.report_cmd_error(error.into()),
                };
                self.set_result(context.empty_object().as_ptr());
            }
            let code = self.invoke_bound_body(cmd, generation, argv);
            // Retirement releases reached owners even after a Host refusal.
            let retirement = self.retire_pending_native_ensemble_roles();
            // Final current and previous worker cleanup is part of this operation,
            // before Guest projection or a success status can leave the boundary.
            drop(jim_command.take());
            if retirement.is_err() {
                return Code::Error;
            }
            // Check the original context before Guest projection or later work.
            if self.check_entered_native_operation().is_err() {
                return Code::Error;
            }
            if code == Code::Error && self.uses_jim_error_stack() {
                if let Err(first) = self.capture_jim_error_stack() {
                    return self.refuse_native_execution(first);
                }
            }
            code
        })();
        drop(jim_command);
        // Also covers early reporting and the actual invocation-scope cleanup.
        if scope.currency().ensure_current_or_refuse().is_err() {
            return Code::Error;
        }
        code
    }

    pub(in super::super) fn enter_original_jim_evaluation(
        &self,
        line: u32,
        filename: *mut TclObj,
    ) -> JimEvaluationScope {
        // naming.source.jim-original-source-entry
        // docs/design/analysis/name-resolution-proofs/jim-original-source-entry.md
        let script = self
            .cmd_frames
            .borrow()
            .last()
            .map(|frame| JimScriptLocation {
                file: frame.file.as_deref().unwrap_or(b"").to_vec(),
                line,
            });
        let mut frames = self.jim_evaluation_frames.borrow_mut();
        let previous_len = frames.len();
        frames.push(OriginalJimEvaluationFrame {
            procedure_level: self.jim_procedure_level.get(),
            command: None,
            is_procedure: false,
            script,
            invocation: Vec::new(),
        });
        let mut borrows = self.jim_invocation_borrows.borrow_mut();
        let previous_borrows_len = borrows.len();
        borrows.push(JimBorrowedInvocation {
            frame_index: previous_len,
            argv: Vec::new(),
            script_filename: Some(filename),
            source: None,
        });
        JimEvaluationScope {
            interp: self.clone(),
            previous_len,
            previous_borrows_len,
        }
    }

    fn borrow_jim_invocation(
        &self,
        argv: &[*mut TclObj],
        source: Option<&Rc<native_script::OriginalSourceInvocation>>,
    ) -> Option<JimInvocationScope> {
        if !self.uses_jim_error_stack() {
            return None;
        }
        let frame_index = self.jim_evaluation_frames.borrow().len().checked_sub(1)?;
        let mut borrows = self.jim_invocation_borrows.borrow_mut();
        let previous_len = borrows.len();
        borrows.push(JimBorrowedInvocation {
            frame_index,
            // Copy pointer slots, not object references: the caller owns the
            // objects through invoke_bound's synchronous return or unwind.
            argv: argv.to_vec(),
            script_filename: None,
            source: source.cloned(),
        });
        Some(JimInvocationScope {
            interp: self.clone(),
            previous_len,
        })
    }

    fn invoke_bound_body(
        &mut self,
        cmd: Command,
        generation: Option<u64>,
        argv: &[*mut TclObj],
    ) -> Code {
        match cmd {
            Command::Builtin(f) => {
                self.observe_codegen_depth_labeled(
                    || {
                        if std::ptr::fn_addr_eq(f, crate::builtins::ret as BuiltinFn) {
                            "selected-stock-return"
                        } else {
                            "selected-other-builtin"
                        }
                    },
                    f as usize,
                );
                let _native_admission = self.enter_native_builtin(generation, argv);
                self.invoke_guarded_definition(generation, argv)
                    .unwrap_or_else(|| f(self, argv))
            }
            Command::Alias {
                original_prefix, ..
            } => self.dispatch_alias(&original_prefix, argv),
            Command::ObjCmd(command) => self.invoke_obj_cmd(&command, argv),
            Command::Imported {
                source,
                source_generation,
                ensemble,
                ..
            } => {
                if let Some(token) = ensemble {
                    if !token.is_deleted() {
                        return self.dispatch_ensemble(&token, argv);
                    }
                }
                match self.resolve_import_source(source_generation) {
                    // Transparent redirect: forward argv unchanged to the source.
                    Some(cmd) => self.invoke_bound(cmd, Some(source_generation), argv),
                    None => self.invalid_command(&source),
                }
            }
            Command::Ensemble(token) => self.dispatch_ensemble(&token, argv),
            Command::Proc(def) => self.call_bound_proc(&def, argv),
            Command::ChildInterp(command) => self.dispatch_original_child(&command, argv),
            Command::OoObject(id) => self.oo_dispatch(id, argv),
            Command::OoMy(id) => crate::cmd_oo::my_cmd(self, id, argv),
            Command::OoMyClass(id) => crate::cmd_oo::myclass_cmd(self, id, argv),
            Command::ParentAlias {
                original_prefix, ..
            } => self.dispatch_parent_alias(&original_prefix, argv),
        }
    }

    /// Call an extension's `Tcl_ObjCmdProc` with `argv` as its `objv`, and
    /// take the completion code it answers. The result is whatever the
    /// procedure left in the interpreter through `Tcl_SetObjResult`, which
    /// dispatch emptied before the call.
    pub(in super::super) fn invoke_obj_cmd(
        &mut self,
        command: &ObjCommand,
        argv: &[*mut TclObj],
    ) -> Code {
        if command.host_backing_retired.get() {
            self.refuse_completion_value_access(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original Runtime host backing retired",
                ),
            );
            return Code::Error;
        }
        if command.original_deleted.get() {
            self.refuse_completion_value_access(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original C object command retired",
                ),
            );
            return Code::Error;
        }

        let Ok(scope) = native_operation_currency::NativeOperationScope::enter(self) else {
            return Code::Error;
        };
        let Ok(objc) = c_int::try_from(argv.len()) else {
            self.set_result_bytes(b"too many arguments for a C command");
            return Code::Error;
        };
        let stated = self.c_api_errors.get();
        // SAFETY: the extension registered this procedure and client data
        // together; `self` stays borrowed for the call, so the pointer is live
        // for as long as the procedure may use it, and `argv` holds `objc` live
        // objects.
        let code = unsafe {
            (command.proc_)(
                command.client_data,
                std::ptr::from_mut(self),
                objc,
                argv.as_ptr(),
            )
        };
        let code = Code::from_int(code);
        if scope.currency().ensure_current_or_refuse().is_err() {
            return Code::Error;
        }
        // C Tcl starts every command with no error in flight, so an error the
        // procedure returned without stating a code through the C API is
        // `NONE`, whatever error came before it.
        if code == Code::Error && self.c_api_errors.get() == stated {
            if let Err(first) =
                self.replace_exception_state_checked(scope.currency(), ExceptionState::default())
            {
                return self.refuse_native_execution(first);
            }
        }
        code
    }

    /// Run a cross-interp alias (`ParentAlias`): invoke `target` (+ `prefix` +
    /// the call args) in this interp's *parent*, copying the parent's result
    /// back. The parent is reached by upgrading the `parent` `Weak` to an owned
    /// `Interp` handle and dispatching through it — re-entrancy (the parent
    /// calling `interp invokehidden $child …` back into this child) works via the
    /// shared interior-mutable state and is only **bounded** by
    /// `MAX_CROSS_INTERP_DEPTH` to cap native-stack growth.
    fn dispatch_parent_alias(
        &mut self,
        original_prefix: &NativeAliasHeader,
        argv: &[*mut TclObj],
    ) -> Code {
        let policy = tcl_runtime_api::completion_options::ControlOptionPolicy::FRESH_FORWARDED;
        if let Err(first) = self.begin_control_options_checked(policy) {
            return self.refuse_native_execution(first);
        }
        let Some(parent_state) = self.parent.borrow().upgrade() else {
            return self.error(b"cannot invoke a parent alias from the root interpreter");
        };
        if CROSS_INTERP_DEPTH.with(|d| d.get()) >= MAX_CROSS_INTERP_DEPTH {
            return self.error(b"too many nested cross-interpreter calls");
        }
        let mut parent = Interp(parent_state);
        if self.invocation_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            return self.dispatch_original_jim_parent_alias(&mut parent, original_prefix, argv);
        }
        let scope = match native_operation_currency::NativeOperationScope::enter(&parent) {
            Ok(scope) => scope,
            Err(first) => {
                parent.refuse_native_execution(first);
                return self.transport_host_refusal_from(&parent);
            }
        };
        let original_prefix_ptr = match original_prefix.checked_ptr() {
            Ok(pointer) => pointer,
            Err(error) => {
                self.refuse_completion_value_access(error);
                return Code::Error;
            }
        };
        let members =
            match tcl_syntax::value::ValueOps::list_elements(&mut parent, &original_prefix_ptr) {
                Ok(members) => members,
                Err(error) => {
                    // This is the captured Native List, not a guest list parse.
                    // A failed original-member projection retains a typed cause.
                    parent.refuse_completion_value_access(error);
                    return self.transport_host_refusal_from(&parent);
                }
            };
        if let Err(error) = original_prefix.checked_ptr() {
            self.refuse_completion_value_access(error);
            return Code::Error;
        }
        let mut new_argv = Vec::with_capacity(members.len() + argv.len());
        for original in members.into_iter().chain(argv.iter().skip(1).copied()) {
            unsafe { obj::incr_ref_count(original) };
            new_argv.push(original);
        }
        CROSS_INTERP_DEPTH.with(|d| d.set(d.get() + 1));
        let code = match parent.begin_control_options_checked(policy) {
            Ok(()) => parent.dispatch_in_lookup_namespace(GLOBAL, &new_argv),
            Err(first) => parent.refuse_native_execution(first),
        };
        CROSS_INTERP_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
        release_all(&new_argv);
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            parent.refuse_native_execution(first);
            return self.transport_host_refusal_from(&parent);
        }
        let result = match parent.native_object_string_bytes(parent.result_obj()) {
            Ok(result) => result,
            Err(error) => {
                parent.refuse_completion_value_access(error);
                return self.transport_host_refusal_from(&parent);
            }
        };
        let options = match parent.pending_return_option_objects_checked(scope.currency()) {
            Ok(options) => options,
            Err(first) => {
                parent.refuse_native_execution(first);
                return self.transport_host_refusal_from(&parent);
            }
        };
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            parent.refuse_native_execution(first);
            return self.transport_host_refusal_from(&parent);
        }
        self.set_result_bytes(&result);
        if let Err(error) = self.set_return_option_objects_checked(options) {
            return self.report_cmd_error(error);
        }
        code
    }

    /// The alias trampoline (`docs/design/runtime/rename-alias.md` §4.2): resolve
    /// the stored `target` by name **anchored at the global namespace** (so a
    /// target deleted after the alias was created surfaces lazily here, but a
    /// *renamed* target is not followed), synthesise
    /// `[target, *prefix, *caller_tail]`, and invoke. Alias-of-alias chains fall
    /// out naturally (the resolved target may itself be an `Alias`) and are
    /// bounded by [`MAX_ALIAS_DISPATCH_DEPTH`], so a cycle that escaped the
    /// definition-time gate errors instead of exhausting the native stack. The
    /// wrapper begins a fresh completion-option scope before invoking its
    /// target, including across an interpreter boundary.
    pub(in super::super) fn dispatch_alias(
        &mut self,
        original_prefix: &NativeAliasHeader,
        argv: &[*mut TclObj],
    ) -> Code {
        if let Err(first) = self.begin_control_options_checked(
            tcl_runtime_api::completion_options::ControlOptionPolicy::FRESH_FORWARDED,
        ) {
            return self.refuse_native_execution(first);
        }
        if ALIAS_DISPATCH_DEPTH.with(|d| d.get()) >= MAX_ALIAS_DISPATCH_DEPTH {
            return self.error(b"too many nested alias invocations (infinite loop?)");
        }
        // Build [target, *prefix, *argv[1..]] — each element owned (+1).
        let mut new_argv: Vec<*mut TclObj> = Vec::new();
        let push_owned = |v: &mut Vec<*mut TclObj>, o: *mut TclObj| {
            // SAFETY: `o` is a live object; take the owning +1 the new argv holds.
            unsafe { obj::incr_ref_count(o) };
            v.push(o);
        };
        let original_prefix_ptr = match original_prefix.checked_ptr() {
            Ok(pointer) => pointer,
            Err(error) => {
                self.refuse_completion_value_access(error);
                return Code::Error;
            }
        };
        let words = match tcl_syntax::value::ValueOps::list_elements(self, &original_prefix_ptr) {
            Ok(words) => words,
            Err(error) => {
                // The captured Native List is retained state, not a guest
                // list parse. Keep its first operational refusal typed.
                self.refuse_completion_value_access(error);
                return Code::Error;
            }
        };
        if let Err(error) = original_prefix.checked_ptr() {
            self.refuse_completion_value_access(error);
            return Code::Error;
        }
        if words.is_empty() {
            self.refuse_completion_value_access(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original alias target member",
                ),
            );
            return Code::Error;
        }
        for word in words {
            push_owned(&mut new_argv, word);
        }
        for &a in &argv[1..] {
            push_owned(&mut new_argv, a);
        }
        ALIAS_DISPATCH_DEPTH.with(|d| d.set(d.get() + 1));
        let lookup = if self
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_some()
        {
            self.current_ns.get()
        } else {
            GLOBAL
        };
        let code = match self.resolve_original_command_at(lookup, new_argv[0]) {
            Ok(Some((command, generation))) => self.invoke_bound(command, generation, &new_argv),
            Ok(None) => self.dispatch_missing_command(
                &new_argv,
                lookup,
                self.current_ns.get(),
                tcl_registry::command_lookup::CommandLookupOrigin::AliasInvocation,
            ),
            Err(error) => self.report_cmd_error(error.into()),
        };
        ALIAS_DISPATCH_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
        release_all(&new_argv);
        code
    }
}
