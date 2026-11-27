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

//! The untraced dispatch of a word list by command kind (procedure, alias, native, builtin) and settling a synchronously run native command.

use super::super::{
    EnteredCommand, ExecLeaveCtx, Frame, NativeWordDispatch, OriginalExecutionOwner, Tick,
};
use crate::command::Command;
use crate::interp::native_operation_currency::NativeOperationScope;
use crate::interp::{CommandSidecarHandle, CommandSidecarKey, Vm, err};
use crate::value::Value;
use std::rc::Rc;
use tcl_bytecode::FunctionAsm;
use tcl_runtime_api::{Code, Completion};

impl Vm {
    /// Deliver a synchronously-completed dispatch into `f`: an ok result is
    /// pushed onto the operand stack, a non-ok completion unwinds.
    pub(in super::super) fn deliver_sync(
        &mut self,
        f: &mut Frame,
        res: Completion<Value>,
    ) -> Result<Option<Tick>, Completion<Value>> {
        if res.code.is_ok() {
            // Jim's evaluation frame borrows argv only while this command is
            // active. Completed diagnostic views must not keep arguments shared.
            f.jim_evaluation.invocation = Value::empty();
            #[cfg(test)]
            if std::env::var_os("TCL_LSP_TRACE_NATIVE_OPTIONS").is_some() {
                res.options
                    .report_native_compound_ownership("deliver-sync-options");
            }
            let protocol = self.actual_native_string_protocol();
            let present = res
                .options
                .native_return_options_nonempty(protocol)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?;
            if present {
                f.last_options = res.options;
            }
            f.stack.push(res.result.into_native_reference());
            Ok(None)
        } else {
            Err(res)
        }
    }

    /// Build the generation-bearing empty unit used by scanner activations at
    /// the moment their native command hands control to the trampoline.
    pub(in super::super) fn current_placeholder_unit(&mut self) -> crate::compiled::CompiledUnit {
        if self.actual_native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            return crate::compiled::CompiledUnit::native_driver(
                self.current_namespace_execution_context(),
                self.profile_generation(),
                self.native_interpreter_identity(),
            );
        }
        self.compiled_unit(
            Rc::new(FunctionAsm::default()),
            self.source_namespace_path(),
        )
    }

    /// Settle a synchronously-run native command (a [`BuiltinFn`] or an
    /// embedder's [`NativeCommand`](crate::command::NativeCommand)): drain any
    /// body it deferred to the explicit stack, else deliver its completion.
    pub(in super::super) fn settle_native_dispatch(
        &mut self,
        f: &mut Frame,
        res: Completion<Value>,
    ) -> Result<Option<Tick>, Completion<Value>> {
        if let Some(refused) = self.refused_completion() {
            return Err(refused);
        }
        // Built out of line: the deferred-work `Tick`s are large, and
        // `deliver_sync` below can run guest code (traces) natively.
        if let Some(tick) = self.take_pending_dispatch_tick(f) {
            return Ok(Some(tick));
        }
        let res = self
            .publish_native_interp_completion(res)
            .map_err(|error| crate::command::completion_from_tcl_error(self, error))?;
        self.deliver_sync(f, res)
    }

    /// The deferred work a builtin left for [`Self::settle_native_dispatch`],
    /// as the `Tick` that runs it on the explicit stack.
    #[inline(never)]
    fn take_pending_dispatch_tick(&mut self, f: &Frame) -> Option<Tick> {
        // A `yield`/`yieldto` sets `coro.pending` (after its own boundary
        // check); convert it into a suspend `Tick` that freezes the whole
        // activation stack. The builtin's placeholder result is dropped —
        // the resume value replaces it on the operand stack.
        if let Some(req) = self.coro.pending.take() {
            return Some(Tick::Suspend(req));
        }
        if let Some(req) = self.pending.procedure_call.take() {
            return Some(Tick::Call {
                proc: Box::new(req.proc),
                invoked: req.invoked,
                argv: req.arguments,
                lambda_registration: Some(req.registration),
            });
        }
        // An `eval`/`uplevel`/`apply`-style builtin defers its body to the
        // explicit stack (yieldable): drain it into a `PushScript`, whose
        // frame result replaces this builtin's placeholder (as for yield).
        if let Some(req) = self.pending.eval.take() {
            return Some(Tick::PushEval(Box::new(req)));
        }
        // A `catch` defers its body the same way, but into a catch frame
        // whose completion the epilogue absorbs (see `Frame::catch`).
        if let Some(req) = self.pending.catch.take() {
            return Some(Tick::PushCatch(Box::new(req)));
        }
        // A `subst` defers to a scanner-driven subst frame, whose `[…]`
        // bodies run yieldably as child frames (see `Frame::subst`).
        if let Some(req) = self.pending.subst.take() {
            return Some(Tick::PushSubst {
                req: Box::new(req),
                placeholder: Box::new(self.current_placeholder_unit()),
            });
        }
        // A `foreach`/`lmap` runtime-fallback loop defers to a
        // scanner-driven each-loop frame, whose iterations run yieldably
        // as child frames (see `Frame::each_loop`).
        if let Some(req) = self.pending.each_loop.take() {
            return Some(Tick::PushEachLoop {
                req: Box::new(req),
                placeholder: Box::new(self.current_placeholder_unit()),
            });
        }
        if let Some(req) = self.pending.expression.take() {
            return Some(Tick::PushExpression {
                req: Box::new(req),
                placeholder: Box::new(self.current_placeholder_unit()),
            });
        }
        if let Some(state) = self.pending.control.take() {
            return Some(Tick::PushControl {
                state: Box::new(state),
                placeholder: Box::new(self.current_placeholder_unit()),
            });
        }
        // A `try` defers its body (and, from `advance_try`, each
        // subsequent phase) to a try-phase frame (see `Frame::try_ctx`).
        if let Some(req) = self.pending.try_phase.take() {
            return Some(Tick::PushTry {
                req: Box::new(req),
                initial_options: f.last_options.clone(),
            });
        }
        None
    }

    fn retain_jim_dispatch(
        &mut self,
        f: &mut Frame,
        words: &crate::NativeListItems,
        command: Option<&Command>,
    ) {
        if self.uses_jim_error_stack() {
            f.jim_evaluation.command = self.original_jim_command_name_reference(&words[0]);
            f.jim_evaluation.is_procedure = matches!(command, Some(Command::Proc(_)));
            f.jim_evaluation.invocation = if command.is_some() {
                Value::invocation_list_view(words)
            } else {
                Value::empty()
            };
            if f.jim_script.is_none()
                && let Some(location) = f.jim_evaluation.script.as_mut()
            {
                let line = f
                    .asm
                    .instructions
                    .get(f.pc.saturating_sub(1))
                    .map_or(1, |instruction| instruction.source_line.max(1));
                location.line = f
                    .source_location
                    .as_ref()
                    .map_or(1, |location| location.line)
                    .saturating_add(line.saturating_sub(1));
            }
            if let Some(current) = self.jim_errors.frames.last_mut() {
                *current = Self::borrow_jim_evaluation_frame(&f.jim_evaluation);
            }
        }
    }

    /// The untraced dispatch body — see [`Self::dispatch_words`].
    fn prepare_dispatch_procedure(
        &mut self,
        p: crate::command::NativeProcedureCommand,
        entered: Option<&EnteredCommand>,
        sidecar_handle: Option<&CommandSidecarHandle>,
        selected: Option<CommandSidecarKey>,
    ) -> Result<crate::command::PreparedProcedureActivation, Completion<Value>> {
        if let Some(entered) = entered {
            let sidecar = entered.sidecar.key();
            if sidecar.is_some() {
                self.ensure_proc_ready_in(p, sidecar.as_ref(), Some(&entered.sidecar))
            } else {
                // The entered token can outlive deletion, but publishing
                // its refreshed body by ProcDef::name would resurrect it
                // or overwrite a replacement created during substitution.
                self.ensure_proc_traced(p)
            }
        } else {
            let sidecar = selected;
            // A traced dispatch re-resolves after its callbacks. Keep the
            // handle only when it still identifies that resolved binding;
            // a callback may have renamed or replaced the original key.
            let sidecar_handle =
                sidecar_handle.filter(|handle| handle.key().as_ref() == sidecar.as_ref());
            self.ensure_proc_ready_in(p, sidecar.as_ref(), sidecar_handle)
        }
        .map_err(|error| crate::command::completion_from_tcl_error(self, error))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn dispatch_words_inner(
        &mut self,
        invocation: (&mut Frame, &crate::NativeListItems),
        sidecar_handle: Option<&CommandSidecarHandle>,
        entered: Option<&EnteredCommand>,
        resolved: Option<&crate::interp::ResolvedHead>,
        context: &crate::interp::namespace_context::NamespaceExecutionContext,
        usage: &[crate::command::NativeArgumentUsageRewrite],
        original_scope: &NativeOperationScope,
    ) -> Result<Option<Tick>, Completion<Value>> {
        let (f, words) = invocation;
        let original = &words[0];
        let selected = if let Some(entered) = entered {
            let key = entered.sidecar.key().and_then(|key| match key {
                CommandSidecarKey::Visible(key) => Some(key),
                CommandSidecarKey::Hidden(_) => None,
            });
            (key, Some(entered.command.clone()))
        } else {
            let looked = match resolved {
                Some(resolved) => {
                    self.lookup_resolved_command_in_context(context, original, resolved)
                }
                None => self.lookup_original_command_in_context(context, original),
            };
            match looked.map_err(|error| crate::command::completion_from_cmd_error(self, error))? {
                Some((key, command)) => (Some(key), Some(command)),
                None => (None, None),
            }
        };
        let (selected_key, command) = selected;
        let jim_lease = command
            .as_ref()
            .and_then(|_| self.retain_original_jim_command(original, selected_key.as_deref()));
        self.retain_jim_dispatch(f, words, command.as_ref());
        if command.is_some() {
            self.reset_native_jim_result()
                .map_err(|error| crate::command::completion_from_tcl_error(self, error.into()))?;
        }
        let saved_handler = self.take_native_handler_metadata();
        let usage_scope = self.enter_native_usage_scope(usage);
        self.set_invoked_name_value(original, selected_key.as_deref().unwrap_or(""));
        let result = self.dispatch_selected_words(
            f,
            words,
            command,
            NativeWordDispatch {
                selected_key: &selected_key,
                sidecar_handle,
                entered,
                context,
                usage,
            },
        );
        self.leave_native_usage_scope(usage_scope);
        self.restore_native_handler_metadata(saved_handler);
        if matches!(
            &result,
            Ok(Some(
                Tick::Call { .. }
                    | Tick::PushScript { .. }
                    | Tick::PushCatch(_)
                    | Tick::PushEval(_)
                    | Tick::PushSubst { .. }
                    | Tick::PushEachLoop { .. }
                    | Tick::PushControl { .. }
                    | Tick::PushExpression { .. }
                    | Tick::PushTry { .. }
            ))
        ) && let Some(lease) = jim_lease
        {
            let context = self.pending_exec_leave.get_or_insert_with(|| ExecLeaveCtx {
                original_scopes: vec![OriginalExecutionOwner::Entered(original_scope.clone())],
                traces: false,
                jim_commands: Vec::new(),
                cmd_string: None,
                leave_owner: None,
                step_scopes: 0,
            });
            context.jim_commands.push(lease);
        }
        result
    }

    fn dispatch_selected_words(
        &mut self,
        f: &mut Frame,
        words: &crate::NativeListItems,
        command: Option<Command>,
        dispatch: NativeWordDispatch<'_>,
    ) -> Result<Option<Tick>, Completion<Value>> {
        let original = &words[0];
        match command {
            Some(Command::Proc(p)) => self.dispatch_procedure_words(f, words, p, dispatch),
            Some(Command::Builtin(bf)) => {
                self.dispatch_builtin_words(f, words, bf, dispatch.entered, dispatch.context)
            }
            Some(Command::Native(cmd)) => {
                self.dispatch_native_words(f, words, cmd.as_ref(), dispatch)
            }
            Some(
                command @ (Command::CallerAlias(_)
                | Command::Alias { .. }
                | Command::CrossAlias { .. }),
            ) => self.dispatch_alias_command_words(f, words, command, dispatch),
            Some(Command::ChildInterp(child)) => {
                let res = self.dispatch_child(original, child, &words[1..]);
                self.settle_native_dispatch(f, res)
            }
            Some(Command::Ensemble(e)) => {
                let res = self.dispatch_ensemble("", &e, &words[1..]);
                self.settle_native_dispatch(f, res)
            }
            Some(Command::Object(key)) => {
                let res = crate::cmd_oo::oo_dispatch(self, key, original, &words[1..]);
                self.settle_native_dispatch(f, res)
            }
            None => {
                let result = self.invoke_missing_command_value_in_context(
                    dispatch.context,
                    original,
                    &words[1..],
                    tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
                );
                self.settle_native_dispatch(f, result)
            }
        }
    }

    /// The procedure arm of [`Self::dispatch_selected_words`], out of line so
    /// its locals are not in the frame a native command dispatch recurses
    /// through (a `TclOO` method call; see `OO_DISPATCH_DEPTH_LIMIT`).
    #[inline(never)]
    fn dispatch_procedure_words(
        &mut self,
        f: &mut Frame,
        words: &crate::NativeListItems,
        p: crate::command::NativeProcedureCommand,
        dispatch: NativeWordDispatch<'_>,
    ) -> Result<Option<Tick>, Completion<Value>> {
        if let Some(result) = self.early_procedure_activation(&p, &words[0], &words[1..])? {
            self.deliver_sync(f, result)?;
            return Ok(None);
        }
        let p = self.prepare_dispatch_procedure(
            p,
            dispatch.entered,
            dispatch.sidecar_handle,
            dispatch
                .selected_key
                .clone()
                .map(CommandSidecarKey::visible),
        )?;
        Ok(Some(Tick::Call {
            proc: Box::new(p),
            invoked: words[0].clone(),
            argv: words[1..].to_vec(),
            lambda_registration: None,
        }))
    }

    /// The alias arms of [`Self::dispatch_selected_words`], out of line for the
    /// same reason as [`Self::dispatch_procedure_words`].
    #[inline(never)]
    fn dispatch_alias_command_words(
        &mut self,
        f: &mut Frame,
        words: &crate::NativeListItems,
        command: Command,
        dispatch: NativeWordDispatch<'_>,
    ) -> Result<Option<Tick>, Completion<Value>> {
        let original = &words[0];
        match command {
            Command::CallerAlias(original_prefix) => {
                let target = original_prefix
                    .native_object_list_elements(
                        tcl_syntax::native_string::NativeStringProtocol::Jim084,
                    )
                    .map_err(|error| {
                        crate::command::completion_from_cmd_error(self, error.into())
                    })?;
                let mut argv = target.as_ref().clone();
                argv.extend_from_slice(&words[1..]);
                let usage = self.alias_usage_rewrites_value(original, target.len(), dispatch.usage);
                self.dispatch_alias_words(
                    f,
                    &crate::NativeListItems::invocation_view(Rc::new(argv)),
                    dispatch.context,
                    &usage,
                )
            }
            Command::Alias { words: target, .. } => {
                let mut argv = (*target).clone();
                argv.extend_from_slice(&words[1..]);
                let usage = self.alias_usage_rewrites_value(original, target.len(), dispatch.usage);
                self.dispatch_alias_words(
                    f,
                    &crate::NativeListItems::invocation_view(Rc::new(argv)),
                    &crate::interp::namespace_context::NamespaceExecutionContext::Resolved(
                        tcl_core_types::ROOT_NS,
                    ),
                    &usage,
                )
            }
            Command::CrossAlias {
                target: target_interp,
                words: target,
                original_jim_prefix,
                ..
            } => {
                if let Some(prefix) = original_jim_prefix {
                    let result =
                        self.invoke_original_jim_parent_alias(target_interp, &prefix, &words[1..]);
                    self.settle_native_dispatch(f, result)
                } else {
                    let mut argv = (*target).clone();
                    argv.extend_from_slice(&words[1..]);
                    let usage =
                        self.alias_usage_rewrites_value(original, target.len(), dispatch.usage);
                    let res = self.invoke_alias_words_with_usage(
                        target_interp,
                        &argv,
                        tcl_registry::AliasTargetLookup::Global,
                        &usage,
                    );
                    self.settle_native_dispatch(f, res)
                }
            }
            _ => Err(self
                .refuse_host_command("native alias dispatch selected a non-alias command".into())),
        }
    }

    fn dispatch_native_words(
        &mut self,
        frame: &mut Frame,
        words: &crate::NativeListItems,
        command: &dyn crate::command::NativeCommand,
        dispatch: NativeWordDispatch<'_>,
    ) -> Result<Option<Tick>, Completion<Value>> {
        let selected = dispatch
            .entered
            .and_then(|entered| entered.sidecar.key())
            .or_else(|| dispatch.sidecar_handle.and_then(CommandSidecarHandle::key))
            .or_else(|| {
                dispatch
                    .selected_key
                    .clone()
                    .map(CommandSidecarKey::visible)
            });
        let previous = std::mem::replace(&mut self.native_invocation.sidecar, selected);
        self.native_command_depth += 1;
        let res = command.invoke(self, &words[1..]);
        self.native_command_depth -= 1;
        self.native_invocation.sidecar = previous;
        self.adopt_host_error_code(&res);
        self.settle_native_dispatch(frame, res)
    }

    pub(super) fn dispatch_builtin_words(
        &mut self,
        frame: &mut Frame,
        words: &crate::NativeListItems,
        builtin: crate::command::BuiltinFn,
        entered: Option<&EnteredCommand>,
        context: &crate::interp::namespace_context::NamespaceExecutionContext,
    ) -> Result<Option<Tick>, Completion<Value>> {
        let key = self
            .resolve_original_command_key_in_context(context, &words[0])
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        self.set_invoked_name_value(&words[0], key.as_deref().unwrap_or(""));
        let identity = match entered {
            Some(entered) => entered.builtin_identity.clone(),
            None => self
                .jim_original_builtin_identity(&words[0])
                .or_else(|| key.as_ref().and_then(|key| self.stock_native_identity(key))),
        };
        self.retain_invoked_builtin_identity(identity);
        let previous_sidecar = std::mem::replace(
            &mut self.native_invocation.sidecar,
            key.map(CommandSidecarKey::visible),
        );
        let previous_arguments = self
            .native_invocation
            .arguments
            .replace(words.lifetime_view());
        let result = builtin(self, &words[1..]);
        self.native_invocation.sidecar = previous_sidecar;
        if result.code == Code::Error {
            self.observe_native_error_result(&result.result);
        }
        self.native_invocation.arguments = previous_arguments;
        self.settle_native_dispatch(frame, result)
    }

    fn dispatch_alias_words(
        &mut self,
        frame: &mut Frame,
        words: &crate::NativeListItems,
        context: &crate::interp::namespace_context::NamespaceExecutionContext,
        usage: &[crate::command::NativeArgumentUsageRewrite],
    ) -> Result<Option<Tick>, Completion<Value>> {
        let Some((head, arguments)) = words.split_first() else {
            return Err(err("alias target has no command"));
        };
        if self
            .resolve_original_command_key_in_context(context, head)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?
            .is_some()
        {
            return self.dispatch_words_entry_in_context(frame, words, None, context, usage, false);
        }
        let result = self.invoke_missing_command_value_in_context(
            context,
            head,
            arguments,
            tcl_registry::command_lookup::CommandLookupOrigin::AliasInvocation,
        );
        self.settle_native_dispatch(frame, result)
    }

    pub(in super::super) fn original_return_code_for_branch(
        &mut self,
        value: &Value,
    ) -> Result<Option<i64>, Completion<Value>> {
        use crate::interp::native_numeric_ops::OriginalNumericOps;
        use tcl_syntax::scalar_getter::{NativeScalarGetterKind, NativeScalarGetterValue};
        let original = NativeOperationScope::enter(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let before = OriginalNumericOps::in_original_scope(self, &original)
            .and_then(|numeric| numeric.current_for_value(value));
        before.map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = if let Some(authored) = self.authored_numeric_context() {
            match value.as_int_with_stages(Some(authored)) {
                Ok(integer) => Ok(Some(integer)),
                Err(error @ crate::TclError::Host(_)) => {
                    Err(crate::command::completion_from_tcl_error(self, error))
                }
                Err(_) => Ok(None),
            }
        } else {
            match OriginalNumericOps::in_original_scope(self, &original)
                .and_then(|numeric| numeric.getter(value, NativeScalarGetterKind::Int))
            {
                Ok(NativeScalarGetterValue::Wide(integer)) => Ok(Some(integer)),
                Err(error) if error.native_execution_refusal().is_some() => {
                    Err(crate::command::completion_from_cmd_error(self, error))
                }
                Ok(_) | Err(_) => Ok(None),
            }
        };
        self.finish_original_dispatch(&original, reached)
    }
}
