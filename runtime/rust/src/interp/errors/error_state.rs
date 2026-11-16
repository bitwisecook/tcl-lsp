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

//! The live error state: snapshots and restores, error-code and completion bytes, the error stack log, and publishing or resetting an error episode.

use super::super::{
    drop_fresh, native_operation_currency, new_string, ErrorSnapshot, ErrorStateSnapshot,
    ExceptionState, Interp,
};
use crate::obj;
use crate::obj::TclObj;

impl Interp {
    /// The current accumulated `errorInfo` (for `catch`'s `-errorinfo`): the
    /// trace if any frame was logged, else the bare error message.
    pub(crate) fn error_info(&mut self) -> Result<Vec<u8>, tcl_runtime_api::NativeExecutionError> {
        self.completion_error_info_bytes_checked()
    }

    /// Capture the error trace state (`errorInfo`/`errorCode` accumulation), so a
    /// command run in a *different* flow can transplant it into this one. Used by
    /// `coroprobe`: the probe runs in the coroutine's (swapped-in) exception
    /// state, but its error must surface in the *caller's* trace once that state
    /// is swapped back out.
    pub(crate) fn snapshot_error(&self) -> ErrorSnapshot {
        let currency = native_operation_currency::SavedNativeOperationCurrency::capture(self);
        let exc = self.exc.borrow();
        ErrorSnapshot {
            currency,
            native: exc.native.clone(),
            jim_error_stack: self.jim_error_stack.borrow().clone(),
            info: exc.info.clone(),
            code: exc.code.clone(),
            code_explicit: exc.code_explicit,
            already_logged: exc.already_logged,
            primitive_getter: exc.primitive_getter.clone(),
            expression_error_stage: exc.expression_error_stage.clone(),
        }
    }

    /// Restore an [`ErrorSnapshot`] captured by [`snapshot_error`] into this
    /// flow's exception state (the trace continues from there — e.g. `coroprobe`
    /// then appends its own `(injected coroutine probe command)` frame).
    pub(crate) fn restore_error(
        &self,
        snap: ErrorSnapshot,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let cleanup = native_operation_currency::NativeOperationScope::enter_saved_cleanup(
            &snap.currency,
            self,
        )?;
        let retired_stack = self.jim_error_stack.replace(snap.jim_error_stack);
        let retired_native = {
            let mut exc = self.exc.borrow_mut();
            let retired = std::mem::replace(&mut exc.native, snap.native);
            exc.info = snap.info;
            exc.code = snap.code;
            exc.code_explicit = snap.code_explicit;
            exc.already_logged = snap.already_logged;
            exc.primitive_getter = snap.primitive_getter;
            exc.expression_error_stage = snap.expression_error_stage;
            retired
        };
        // Publish the complete saved state before mandatory original retirement.
        drop(retired_stack);
        drop(retired_native);
        cleanup.check()
    }

    /// Snapshot the current error/result state (the result bytes + the
    /// `errorInfo`/`errorCode` accumulator) so a side-effecting cleanup (e.g.
    /// running a destructor after a failed constructor) can run and then have
    /// the original error restored.
    pub(crate) fn error_snapshot(&mut self) -> ErrorStateSnapshot {
        let currency = native_operation_currency::SavedNativeOperationCurrency::capture(self);
        let result = self.completion_result_bytes_checked();
        let _ = currency.check(self);
        ErrorStateSnapshot {
            currency,
            result,
            exception: self.exc.borrow().clone(),
            jim_stack: self.jim_error_stack.borrow().clone(),
        }
    }

    /// Restore a previously taken [`error_snapshot`](Self::error_snapshot).
    pub(crate) fn error_restore(
        &mut self,
        snap: ErrorStateSnapshot,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let cleanup = native_operation_currency::NativeOperationScope::enter_saved_cleanup(
            &snap.currency,
            self,
        )?;
        let retired_stack = self.jim_error_stack.replace(snap.jim_stack);
        // Retained result bytes are actual getter output. Missing bytes remain
        // an explicit first operational failure, never an empty replacement.
        if let Ok(result) = &snap.result {
            self.set_result_bytes(result);
        }
        let retired_state = self.exc.replace(snap.exception);
        drop(retired_stack);
        drop(retired_state);
        cleanup.check()?;
        snap.result.map(|_| ())
    }

    /// Exact current error-code bytes, with an absent implicit code represented
    /// by the guest `NONE` list. A positive explicit empty code remains empty.
    /// Host failures are returned before and after the actual original getter.
    pub(crate) fn error_code_bytes_checked(
        &mut self,
    ) -> Result<Vec<u8>, tcl_runtime_api::NativeExecutionError> {
        let scope = native_operation_currency::NativeOperationScope::enter(self)?;
        self.check_native_error_object_readable(
            tcl_registry::special_vars::NativeErrorStorageVariable::ErrorCode,
        )?;
        let (original, stamped, explicit) = {
            let exc = self.exc.borrow();
            (
                exc.native
                    .code
                    .as_ref()
                    .map(|value| obj::NativeObjectLifetime::retain(value.as_ptr())),
                exc.code.clone(),
                exc.code_explicit,
            )
        };
        let original = if self.uses_c84_global_error_info() {
            let reached = self.var_get_at(b"::errorCode", 0);
            scope.currency().ensure_current_or_refuse()?;
            if let Some(original) = reached {
                obj::check_native_liveness(original)
                    .map_err(|error| self.refuse_completion_value_access(error))?;
                Some(obj::NativeObjectLifetime::retain(original))
            } else {
                None
            }
        } else {
            original
        };
        let bytes = if let Some(original) = original {
            let protocol = self.invocation_string_protocol().ok_or(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "private error-code string",
                ),
            );
            let reached = protocol
                .and_then(|protocol| crate::dict::native_object_bytes(original.as_ptr(), protocol));
            scope.currency().ensure_current_or_refuse()?;
            obj::check_native_liveness(original.as_ptr())
                .map_err(|error| self.refuse_completion_value_access(error))?;
            reached.map_err(|error| self.refuse_completion_value_access(error))?
        } else if stamped.is_empty() && !explicit {
            b"NONE".to_vec()
        } else {
            stamped
        };
        scope.currency().ensure_current_or_refuse()?;
        Ok(bytes)
    }

    /// Read the genuine current result without a context-free presentation getter.
    pub(crate) fn completion_result_bytes_checked(
        &mut self,
    ) -> Result<Vec<u8>, tcl_runtime_api::NativeExecutionError> {
        self.completion_object_bytes_checked(self.result_obj())
    }

    /// Project accumulated error info or the original result at its actual getter.
    pub(crate) fn completion_error_info_bytes_checked(
        &mut self,
    ) -> Result<Vec<u8>, tcl_runtime_api::NativeExecutionError> {
        let scope = native_operation_currency::NativeOperationScope::enter(self)?;
        let reached = if self.uses_jim_error_stack() {
            self.jim_stacktrace()
        } else {
            let accumulated = self.exc.borrow().info.clone();
            match accumulated {
                Some(bytes) => Ok(bytes),
                None => self.completion_result_bytes_checked(),
            }
        };
        scope.currency().ensure_current_or_refuse()?;
        reached
    }

    fn completion_object_bytes_checked(
        &mut self,
        original: *mut TclObj,
    ) -> Result<Vec<u8>, tcl_runtime_api::NativeExecutionError> {
        let scope = native_operation_currency::NativeOperationScope::enter(self)?;
        obj::check_native_liveness(original)
            .map_err(|error| self.refuse_completion_value_access(error))?;
        let original = obj::NativeObjectLifetime::retain(original);
        let reached = self.native_object_string_bytes(original.as_ptr());
        scope.currency().ensure_current_or_refuse()?;
        obj::check_native_liveness(original.as_ptr())
            .map_err(|error| self.refuse_completion_value_access(error))?;
        let bytes = reached.map_err(|error| self.refuse_completion_value_access(error))?;
        drop(original);
        scope.currency().ensure_current_or_refuse()?;
        Ok(bytes.to_vec())
    }

    /// Retain a reached original metadata getter failure without changing guest
    /// state. Reentrant callbacks' earlier host cause remains authoritative.
    pub(crate) fn refuse_completion_value_access(
        &mut self,
        error: tcl_syntax::value::ValueError,
    ) -> tcl_runtime_api::NativeExecutionError {
        if let Some(cause) = error.native_access_refusal() {
            self.refuse_native_access(cause);
        } else {
            self.refuse_host_command(format!("native completion value access failed: {error}"));
        }
        self.native_execution_refusal()
            .expect("original host cause retained")
    }

    /// Positive guest fixture projection; production uses the fallible owner.
    #[cfg(test)]
    pub(crate) fn error_code(&self) -> Vec<u8> {
        self.clone()
            .error_code_bytes_checked()
            .expect("fixture has genuine guest error-code state")
    }

    /// Mark the live error's `-errorcode` as explicitly supplied (so an explicit
    /// empty code is preserved instead of defaulting to `NONE`; error-4.5).
    pub(crate) fn mark_error_code_explicit(&mut self) {
        self.exc.borrow_mut().code_explicit = true;
    }

    /// Record the inner context and actual call-frame role at the reached
    /// command-log operation. The caller owns the already-logged guard.
    pub(crate) fn error_stack_log(
        &self,
        command: &[u8],
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let scope = native_operation_currency::NativeOperationScope::enter(self)?;
        let reached = (|| {
            if !self.runtime_version().has_error_stack() {
                return Ok(());
            }
            self.begin_error_stack_bytes_checked(command)?;
            use tcl_runtime_api::error_stack::ErrorStackFrame;
            // Preserve the existing actual frame/argv selection, but release its
            // borrow before constructing or replacing any real List/header.
            let frame = {
                let frames = self.frames.borrow();
                frames.error_stack_frame().map(|(delta, words)| {
                    (delta, words.to_vec(), frames.original_error_stack_argv())
                })
            };
            let strings = self
                .native_invocation_dialect()
                .native_error_objects_protocol()
                .ok_or_else(|| {
                    self.clone().refuse_completion_value_access(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "actual C error stack",
                        ),
                    )
                })?
                .strings();
            let frame = match frame {
                Some((Some(delta), _, _)) => ErrorStackFrame::Redirect(obj::Owned::fresh(
                    obj::new_wide_int_obj(i64::try_from(delta).unwrap_or(i64::MAX)),
                )),
                Some((None, words, original)) => {
                    if original.as_ref().is_some_and(Vec::is_empty) {
                        return Ok(());
                    }
                    let replacements = if original.is_none() {
                        words
                            .iter()
                            .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)))
                            .collect::<Vec<_>>()
                    } else {
                        Vec::new()
                    };
                    let pointers = original
                        .unwrap_or_else(|| replacements.iter().map(obj::Owned::as_ptr).collect());
                    let allocations = pointers
                        .iter()
                        .map(|&original| {
                            obj::check_native_liveness(original).map_err(|error| {
                                self.clone().refuse_completion_value_access(error)
                            })?;
                            Ok(obj::NativeObjectLifetime::retain(original))
                        })
                        .collect::<Result<Vec<_>, tcl_runtime_api::NativeExecutionError>>()?;
                    let frame =
                        obj::Owned::fresh(crate::list::new_list_obj_native(&pointers, strings));
                    drop(allocations);
                    drop(replacements);
                    scope.currency().ensure_current_or_refuse()?;
                    ErrorStackFrame::Call(frame)
                }
                None => ErrorStackFrame::Unreported,
            };
            self.log_original_error_stack_frame_checked(frame)
        })();
        scope.currency().ensure_current_or_refuse()?;
        reached
    }

    /// Retain the actual private List header without reconstructing its members.
    #[cfg(test)]
    pub(crate) fn original_error_stack_value(&self) -> obj::Owned {
        self.error_stack.borrow().value()
    }

    /// Copy supplied original members into the selected private List header.
    pub(crate) fn seed_original_error_stack(
        &self,
        original: *mut TclObj,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        self.seed_original_error_stack_checked(original)
    }

    /// The innermost error command's 1-based source line.
    pub(crate) fn error_line(&self) -> u32 {
        self.error_line.get()
    }

    /// Mark the start of a new error episode (C's `iPtr->resetErrorStack = 1`,
    /// set by `Tcl_ResetResult`): the *next* logged command rebuilds the stack.
    /// The current contents are kept until then (so `info errorstack` after a
    /// `catch` still reports the last error).
    pub(crate) fn mark_error_stack_reset(&self) {
        self.error_stack.borrow_mut().mark_reset();
    }

    /// Preserve the reached expression failure record until script propagation.
    #[cfg_attr(not(have_tommath), allow(dead_code))]
    pub(crate) fn retain_expression_error_stage(
        &self,
        stage: Box<tcl_registry::native_expression_error::NativeExpressionErrorStage>,
    ) {
        self.exc.borrow_mut().expression_error_stage = Some(stage);
    }

    /// Primitive callers retain their result and state. Script propagation owns
    /// the separate message projection and selected expression error-code update.
    pub(in super::super) fn propagate_error_stage(
        &mut self,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let scope = native_operation_currency::NativeOperationScope::enter(self)?;
        let stage = self.exc.borrow_mut().expression_error_stage.take();
        if let Some(stage) = stage {
            if let tcl_registry::native_numeric_error::NativeExpressionErrorCodeUpdate::Set(code) =
                stage.eval_update()
            {
                self.replace_native_error_code_checked(scope.currency(), code)?;
                let mut exc = self.exc.borrow_mut();
                exc.code.clone_from(code);
                exc.code_explicit = false;
            }
        }
        let primitive = self.exc.borrow_mut().primitive_getter.take();
        if let Some(primitive) = primitive {
            if let tcl_syntax::scalar_getter::NativeScalarGetterErrorCode::Set(code) =
                primitive.eval_error_code_update()
            {
                if self.uses_c84_global_error_info() {
                    // This selected default-code setter is a normal global
                    // write, without Tcl_SetObjErrorCode's explicit flag.
                    let original = obj::Owned::fresh(new_string(code));
                    let _ = self.var_set(b"::errorCode", original.as_ptr());
                    drop(original);
                    scope.currency().ensure_current_or_refuse()?;
                } else {
                    self.replace_native_error_code_checked(scope.currency(), code)?;
                }
                let mut exc = self.exc.borrow_mut();
                exc.code.clone_from(code);
                exc.code_explicit = false;
            }
            let original = obj::NativeObjectLifetime::retain(self.result.get());
            let reached = self.completion_result_bytes_checked();
            scope.currency().ensure_current_or_refuse()?;
            obj::check_native_liveness(original.as_ptr())
                .map_err(|error| self.refuse_completion_value_access(error))?;
            if self.result.get() != original.as_ptr() {
                return Err(self.refuse_completion_value_access(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "primitive error result replaced during getter",
                    ),
                ));
            }
            let current = reached?;
            let projected = primitive.eval_result_bytes(&current);
            if projected.len() != current.len() {
                self.set_result_bytes(projected);
            }
            drop(original);
            scope.currency().ensure_current_or_refuse()?;
        }
        scope.currency().ensure_current_or_refuse()
    }

    /// Publish and consume the accumulated exception at a native result reset.
    pub(in super::super) fn publish_error(&mut self) {
        self.publish_error_episode(true);
    }

    /// Publish the accumulated exception, consuming it when `reset`.
    pub(in super::super) fn publish_error_episode(&mut self, reset: bool) {
        #[cfg(test)]
        self.observe_c84_error_episode("publish.enter", None);
        let Ok(scope) = native_operation_currency::NativeOperationScope::enter(self) else {
            return;
        };
        if self.propagate_error_stage().is_err() {
            return;
        }
        if self
            .native_invocation_dialect()
            .error_arguments()
            .is_some_and(|protocol| protocol.publishes_tcl_error_globals())
        {
            if self
                .native_invocation_dialect()
                .native_error_variable_protocol()
                .is_some()
            {
                if self
                    .publish_native_error_objects_checked(scope.currency())
                    .is_err()
                {
                    return;
                }
            } else if !self.uses_c84_global_error_info() || !self.exc.borrow().native.legacy_copy {
                #[cfg(test)]
                self.observe_c84_error_episode("publish.compatibility_globals", None);
                let info = match self.completion_error_info_bytes_checked() {
                    Ok(info) => info,
                    Err(_) => return,
                };
                if scope.currency().ensure_current_or_refuse().is_err() {
                    return;
                }
                let code = match self.error_code_bytes_checked() {
                    Ok(code) => code,
                    Err(_) => return,
                };
                if scope.currency().ensure_current_or_refuse().is_err() {
                    return;
                }
                for (name, bytes) in [
                    (b"::errorInfo".as_slice(), info),
                    (b"::errorCode".as_slice(), code),
                ] {
                    let original = new_string(&bytes);
                    let memory = obj::NativeObjectLifetime::retain(original);
                    let reached = self.var_set(name, original);
                    if reached.is_err() && obj::allocation_is_live(memory.as_ptr()) {
                        drop_fresh(original);
                    }
                    drop(memory);
                    if scope.currency().ensure_current_or_refuse().is_err() {
                        return;
                    }
                }
            }
        }
        if !reset {
            return;
        }
        if self
            .replace_exception_state_checked(scope.currency(), ExceptionState::default())
            .is_err()
        {
            return;
        }
        self.clear_during();
        if scope.currency().ensure_current_or_refuse().is_err() {
            return;
        }
        self.mark_error_stack_reset();
    }

    /// A new public native evaluation resets the active exception episode.
    /// Modern C publishes its retained private originals before releasing them;
    /// C8.4 leaves the existing global headers in their cells.
    pub(in super::super) fn reset_outermost_native_error(&mut self) {
        if self.eval_depth.get() != 0 || self.host_refusal_pending() {
            return;
        }
        if self.uses_c84_global_error_info() {
            self.reset_native_global_error_episode();
        } else if self
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .is_some()
        {
            self.publish_error();
        }
    }

    /// Publish + reset, for `catch`/`try` once they have captured the options.
    pub(crate) fn publish_and_reset_error(&mut self) {
        if !self.host_refusal_pending() {
            self.publish_error();
        }
    }

    /// Stash the `-during` chain link for the next error-options build (TIP 329
    /// exception chaining): `opts` is the options dict of the exception the
    /// just-thrown handler/`finally` exception supersedes. Takes its own owning
    /// reference (releasing any prior link); the caller keeps its own.
    pub(crate) fn set_during(&self, opts: *mut TclObj) {
        // SAFETY: `opts` is a live object; retain it for the field's own ref.
        unsafe { obj::incr_ref_count(opts) };
        if let Some(old) = self.during.replace(Some(opts)) {
            // SAFETY: drop the previously-held link's owning reference.
            unsafe { obj::decr_ref_count(old) };
        }
    }

    /// Drop the pending `-during` chain link, if any (no error to chain).
    pub(crate) fn clear_during(&self) {
        if let Some(old) = self.during.take() {
            // SAFETY: release the field's owning reference.
            unsafe { obj::decr_ref_count(old) };
        }
    }

    /// The pending `-during` chain link (borrowed), for
    /// [`completion_options`](crate::cmd_error::completion_options) to splice
    /// into an error's options dict. `None` when no chaining is active.
    pub(crate) fn during_opts(&self) -> Option<*mut TclObj> {
        let d = self.during.take();
        self.during.set(d);
        d
    }
}
