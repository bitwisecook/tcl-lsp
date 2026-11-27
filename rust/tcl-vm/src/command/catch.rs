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

//! `catch`: running the body, storing the result and options variables, and Jim's catch.

use super::carried_options::{completion_option_failure, opt_get_checked, resolved_error_code};
use super::error_completion::{completion_from_cmd_error, completion_from_tcl_error};
use super::{CatchErrorMetadata, native_wrong_arguments_message};
use crate::error::TclError;
use crate::interp::{Vm, err, ok};
use crate::value::Value;
use tcl_runtime_api::{Code, Completion};

fn finish_catch_preparation(
    vm: &mut Vm,
    prepared: Result<crate::interp::PreparedScript, TclError>,
    script: &Value,
    resvar: Option<&Value>,
    optvar: Option<&Value>,
    ignored_codes: u64,
    evaluates_word: bool,
) -> Completion<Value> {
    match prepared {
        Ok(prepared) if prepared.prefix.is_some() => {
            vm.pending.catch = Some(crate::exec::CatchReq {
                script: prepared
                    .prefix
                    .expect("checked above")
                    .with_source_location(script.source_location()),
                resvar: resvar.map(|v| (*v).clone()),
                optvar: optvar.map(|v| (*v).clone()),
                fatal_tail: prepared.fatal_tail,
                ignored_codes,
                evaluates_word,
            });
            ok(Value::empty())
        }
        Ok(prepared) => {
            let comp = prepared.fatal_tail.map_or_else(
                || ok(Value::empty()),
                |tail| vm.raise_script_parse_failure(tail, script.source_location()),
            );
            vm.finish_catch(comp, resvar, optvar, ignored_codes)
        }
        // A body that fails to *parse* is itself a catchable error: run the
        // epilogue directly with the parse-error completion (no body to execute).
        Err(e) => {
            let comp = completion_from_tcl_error(vm, e);
            vm.finish_catch(comp, resvar, optvar, ignored_codes)
        }
    }
}

fn run_catch_body(
    vm: &mut Vm,
    args: &[Value],
    dialect: tcl_native_model::InvocationDialect,
    evaluates_word: bool,
    original: Option<&crate::interp::native_name_consumers::OriginalNameConsumerScope>,
) -> Completion<Value> {
    let selection = if let Some(original) = &original {
        let reached = tcl_registry::catch_invocation::select_native_jim_catch_invocation(
            args.len(),
            dialect,
            |index| original.read(vm, &args[index]).map(|bytes| bytes.to_vec()),
        );
        if let Err(error) = original.check(vm) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        match reached {
            Ok(selection) => selection,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        }
    } else {
        match tcl_registry::catch_invocation::select_original_catch_invocation(
            args.len(),
            dialect,
            |index| {
                vm.native_name_operand_bytes_checked(&args[index])
                    .map(|bytes| bytes.to_vec())
            },
        ) {
            Ok(selection) => selection,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        }
    };
    let selected = match selection {
        tcl_registry::catch_invocation::CatchInvocationSelection::Valid(selected) => selected,
        _ if dialect.completion_options_policy()
            == Some(tcl_native_model::CompletionOptionsPolicy::Legacy) =>
        {
            return native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"catch command ?varName?\"",
            );
        }
        _ => {
            return native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"catch script ?resultVarName? ?optionVarName?\"",
            );
        }
    };
    let script = &args[selected.script_at];
    let resvar = selected.result_var_at.map(|index| &args[index]);
    let optvar = selected.options_var_at.map(|index| &args[index]);
    // Defer the body to the *explicit* stack so a `yield` inside it stays
    // yieldable: compile it and hand it to the trampoline via
    // the pending catch request. A catch frame runs the body and its completion — of any
    // code — is absorbed by `finish_catch` (which binds the result/options vars
    // and yields the status code). Mirrors `cmd_eval`'s pending request, but
    // catch's completion is caught rather than propagated.
    if let Some(original) = &original
        && let Err(error) = original.current_for_value(vm, script)
    {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    let prepared = vm.prepare_script_commands_value(script);
    if let Some(original) = &original
        && let Err(error) = original.check(vm)
    {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    finish_catch_preparation(
        vm,
        prepared,
        script,
        resvar,
        optvar,
        selected.ignored_codes,
        evaluates_word,
    )
}

/// `catch script ?resultVarName? ?optionsVarName?`.
pub(super) fn cmd_catch(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    // A compiled `catch` whose script is not a literal evaluates it as a
    // word inside its catch range (C's `INST_EVAL_STK`), and logs itself as
    // an error leaves that script.
    let evaluates_word = vm
        .pending
        .release_compiled
        .take()
        .is_some_and(|compiled| compiled.script_word.is_some());
    if vm.uses_jim_error_stack()
        && let Err(error) = vm.reset_original_jim_global_error_code()
        && vm.refused_completion().is_some()
    {
        return error;
    }
    let jim = vm.uses_jim_error_stack();
    let dialect = if jim {
        vm.actual_native_invocation_dialect()
    } else {
        vm.native_invocation_dialect()
    };
    let original = if jim {
        match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
            Ok(original) => Some(original),
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        }
    } else {
        None
    };
    let reached = run_catch_body(vm, args, dialect, evaluates_word, original.as_ref());
    if let Some(original) = original
        && let Err(error) = original.finish(vm)
    {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    reached
}

impl Vm {
    /// The `catch` epilogue, shared by the explicit-stack catch frame
    /// ([`crate::exec`]'s `unwind`) and the `invoke_command` / parse-error
    /// fallbacks: from the body's completion `comp`, bind the result and options
    /// variables and return the status code as an integer (this `catch` command's
    /// result). An uncatchable `exit` unwind passes straight through; a failure to
    /// write a bind variable propagates as this catch's own error.
    ///
    /// `take_error_info` consumes the accumulated trace, so the same values feed
    /// both the options dict and the `$errorInfo`/`$errorCode` globals; a
    /// non-error completion still clears any in-flight trace so the next error
    /// starts fresh.
    pub(crate) fn finish_catch(
        &mut self,
        comp: Completion<Value>,
        resvar: Option<&Value>,
        optvar: Option<&Value>,
        ignored_codes: u64,
    ) -> Completion<Value> {
        if let Some(refused) = self.refused_completion() {
            return refused;
        }
        if self.uses_jim_error_stack() {
            return self.finish_original_jim_catch(comp, resvar, optvar, ignored_codes);
        }
        let code = u32::try_from(comp.code.as_int()).ok();
        if code.is_some_and(|code| code < 64 && ignored_codes & (1_u64 << code) != 0) {
            return comp;
        }
        if self.exit_pending() {
            if self.native_invocation_dialect().family() == Some(tcl_dialect::model::Family::Jim)
                && comp.code == Code::from_int(6)
            {
                let _ = self.take_exit();
            } else {
                return comp;
            }
        }
        let (opts, error_meta) = match self.checked_catch_options(&comp) {
            Ok(captured) => captured,
            Err(error) => return completion_from_tcl_error(self, error),
        };
        // 8.4 and 8.5 reset the result before a failed store replaces its
        // message; 8.6+ store first, so a failed store's error continues the
        // body's errorInfo (CatchObjCmdCallback).
        let resets_before_store = self.runtime_version() < tcl_dialect::TclVersion::V8_6;
        if resets_before_store {
            self.reset_native_error_result();
            let _ = self.take_error_info();
        } else {
            self.forget_error_logged();
        }
        if let Some(r) = resvar
            && let Err(e) = self.store_original_named_variable(r, comp.result.clone())
        {
            return self.catch_store_failed(e, "couldn't save command result in variable");
        }
        if let Some(o) = optvar
            && let Err(e) = self.store_original_named_variable(o, opts)
        {
            return self.catch_store_failed(e, "couldn't save return options in variable");
        }
        if !resets_before_store {
            let _ = self.take_error_info();
        }
        if let Some((einfo, ecode)) = &error_meta {
            self.publish_error(einfo, ecode);
        }
        // The catch epilogue replaces the interpreter's body result with its
        // status integer (JimCatchTryHelper's Jim_SetResultInt). A deferred
        // Script invocation reads that actual result owner after this returns;
        // the completion carries only a lifetime view of the same object.
        match self.publish_native_interp_completion(ok(Value::int(comp.code.as_int()))) {
            Ok(completion) => completion,
            Err(error) => completion_from_tcl_error(self, error),
        }
    }

    fn finish_original_jim_catch(
        &mut self,
        mut comp: Completion<Value>,
        resvar: Option<&Value>,
        optvar: Option<&Value>,
        ignored_codes: u64,
    ) -> Completion<Value> {
        let captured = match self.capture_original_jim_error_code() {
            Ok(captured) => captured,
            Err(error) => return completion_from_cmd_error(self, error),
        };
        comp.result = match self.with_native_interp_result(Value::native_lifetime_lease) {
            Ok(result) => result.into_value(),
            Err(error) => return completion_from_cmd_error(self, error.into()),
        };
        let code = u32::try_from(comp.code.as_int()).ok();
        if code.is_some_and(|code| code < 64 && ignored_codes & (1_u64 << code) != 0) {
            return comp;
        }
        if self.exit_pending() {
            if comp.code == Code::from_int(6) {
                let _ = self.take_exit();
            } else {
                return comp;
            }
        }
        let mut bindings_ok = true;
        if let Some(name) = resvar {
            let bytes = match self
                .native_name_operand_bytes_checked_using(name, &|vm| captured.check(vm))
            {
                Ok(bytes) => bytes,
                Err(error) => return completion_from_cmd_error(self, error),
            };
            if !bytes.is_empty()
                && let Err(error) = self.store_original_named_variable(name, comp.result.clone())
            {
                if self.refused_completion().is_some() {
                    return error;
                }
                bindings_ok = false;
            }
        }
        if bindings_ok && let Some(name) = optvar {
            let bytes = match self
                .native_name_operand_bytes_checked_using(name, &|vm| captured.check(vm))
            {
                Ok(bytes) => bytes,
                Err(error) => return completion_from_cmd_error(self, error),
            };
            if !bytes.is_empty() {
                let options = match self
                    .current_jim_options_for_exit_code_with_error_code(comp.code, &captured)
                {
                    Ok(options) => options,
                    Err(error) => return completion_from_cmd_error(self, error),
                };
                if let Err(error) = self.store_original_named_variable(name, options)
                    && self.refused_completion().is_some()
                {
                    return error;
                }
            }
        }
        if let Err(error) = captured.check(self) {
            return completion_from_cmd_error(self, error);
        }
        match self.publish_native_interp_completion(ok(Value::int(comp.code.as_int()))) {
            Ok(completion) => completion,
            Err(error) => completion_from_tcl_error(self, error),
        }
    }

    /// The error `catch` raises when it cannot store into its result or
    /// options variable: the store's own up to 8.5's replacement `message`.
    fn catch_store_failed(&self, failed: Completion<Value>, message: &str) -> Completion<Value> {
        if self.runtime_version() < tcl_dialect::TclVersion::V8_6 {
            err(message)
        } else {
            failed
        }
    }

    /// The options dict a *compiled* catch range records when it absorbs
    /// `comp` — the same digestion [`Vm::finish_catch`] performs (consume the
    /// accumulated `errorInfo` trace, resolve the error code and line, publish
    /// the `$errorInfo`/`$errorCode` globals) minus the variable binding,
    /// which the bytecode epilogue does itself from
    /// `PUSH_RESULT`/`PUSH_RETURN_CODE`/`PUSH_RETURN_OPTS`.
    fn checked_catch_options(
        &mut self,
        comp: &Completion<Value>,
    ) -> Result<(Value, CatchErrorMetadata), TclError> {
        let options = self.completion_options_snapshot(comp)?;
        let metadata = if comp.code == Code::Error {
            let info = opt_get_checked(self, &options, b"-errorinfo")?
                .unwrap_or_else(|| comp.result.clone());
            let bytes = tcl_syntax::value::ValueOps::native_string_bytes(self, &info)
                .map_err(|error| completion_option_failure(self, error))?;
            let code = match opt_get_checked(self, &options, b"-errorcode")? {
                Some(code) => code,
                None => resolved_error_code(self, comp)?,
            };
            Some((bytes, code))
        } else {
            None
        };
        Ok((options, metadata))
    }

    pub(crate) fn digest_catch_options(
        &mut self,
        comp: &Completion<Value>,
    ) -> Result<Value, TclError> {
        let (options, metadata) = self.checked_catch_options(comp)?;
        let _ = self.take_error_info();
        if let Some((info, code)) = metadata {
            self.publish_error(&info, &code);
        }
        Ok(options)
    }
}
