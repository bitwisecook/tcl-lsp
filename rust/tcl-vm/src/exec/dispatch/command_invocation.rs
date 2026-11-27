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

//! Invoking a command to a completion outside the instruction loop: command values, missing commands, host object vectors, resolved commands and native settlement.

use super::super::{
    CatchReq, ExecLeaveCtx, Frame, NativeCatchEpilogue, NativeJimVectorFrame, NativeSettlement,
    OriginalExecutionOwner, ResolvedCommandEntry,
};
use crate::command::Command;
use crate::interp::native_operation_currency::NativeOperationScope;
use crate::interp::{CommandSidecarHandle, CommandSidecarKey, Vm, err};
use crate::value::Value;
use std::rc::Rc;
use tcl_runtime_api::{Code, Completion};
use tcl_syntax::value::ValueOps;

impl Vm {
    fn command_lookup_error_value(&mut self, original: &Value) -> Completion<Value> {
        let bytes = match self.native_name_operand_bytes_checked(original) {
            Ok(bytes) => bytes,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        // Both native command error formatters consume C-string reporting operands.
        // This reporting extent is independent from the already-selected table key.
        let Some(policy) = self.name_policy_protocol() else {
            return self
                .refuse_host_command("native command reporting protocol is unavailable".into());
        };
        let name = match tcl_syntax::naming::report_native_name_bytes(
            policy.recipe(),
            tcl_syntax::naming::NativeNameReportPurpose::CommandLookupError,
            &bytes,
        ) {
            Ok(name) => name,
            Err(error) => return self.refuse_host_command(format!("{error:?}")),
        };
        let mut message = b"invalid command name \"".to_vec();
        message.extend_from_slice(name);
        message.push(b'"');
        if policy.recipe().is_jim084() {
            let completion = crate::command::completion_from_cmd_error(
                self,
                tcl_cmd_core::CmdError::new_bytes(message),
            );
            return self.publish_native_lookup_failure(completion);
        }
        let mut code = b"TCL LOOKUP COMMAND ".to_vec();
        tcl_syntax::list::append_list_element(&mut code, name, false);
        let mut error = if self.runtime_version().has_lookup_error_code("COMMAND") {
            tcl_cmd_core::CmdError::with_error_code_bytes(message, code)
        } else {
            tcl_cmd_core::CmdError::new_bytes(message)
        };
        if policy.authority() == tcl_syntax::naming::NamePolicyAuthority::Native {
            let Some(producer) = self
                .actual_native_invocation_dialect()
                .native_object_vector_protocol()
                .filter(|producer| producer.strings() == policy.string_protocol())
            else {
                return self.refuse_host_command(
                    "native missing-command result producer is unavailable".into(),
                );
            };
            if producer.missing_command_has_string_primary() {
                error = error.with_native_string_result(producer.strings());
            }
        }
        let completion = crate::command::completion_from_cmd_error(self, error);
        self.publish_native_lookup_failure(completion)
    }

    /// Failed lookup has no entered-handler epilogue to publish its result.
    /// Use the same interpreter result owner as an actual handler completion.
    fn publish_native_lookup_failure(
        &mut self,
        completion: Completion<Value>,
    ) -> Completion<Value> {
        self.publish_native_interp_completion(completion)
            .unwrap_or_else(|error| crate::command::completion_from_tcl_error(self, error))
    }

    pub(super) fn invoke_missing_command_value_in_context(
        &mut self,
        context: &crate::interp::namespace_context::NamespaceExecutionContext,
        original: &Value,
        arguments: &[Value],
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
    ) -> Completion<Value> {
        match context.resolved_token() {
            Some(namespace) => {
                self.invoke_missing_command_value(namespace, original, arguments, origin)
            }
            None => self.refuse_host_command(
                "missing command fallback namespace coordinates are unavailable".into(),
            ),
        }
    }

    fn invoke_missing_command_value(
        &mut self,
        context: tcl_core_types::NsId,
        original: &Value,
        arguments: &[Value],
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
    ) -> Completion<Value> {
        use tcl_registry::command_lookup::{
            UnknownHandlerNamespace, native_lookup_fallback_policy,
        };
        let Some(policy) = native_lookup_fallback_policy(self.native_invocation_dialect(), origin)
        else {
            return self.command_lookup_error_value(original);
        };
        let mut words = match policy.namespace_handler {
            Some(selection) => {
                let selected = match selection {
                    UnknownHandlerNamespace::Caller => self.current_ns_id(),
                    UnknownHandlerNamespace::Lookup => context,
                };
                let root = self.ns_unknown_handler_at(selected);
                match tcl_syntax::value::ValueOps::list_elements(self, &root) {
                    Ok(words) => words,
                    Err(error) => {
                        return crate::command::completion_from_cmd_error(self, error.into());
                    }
                }
            }
            None => vec![Value::string(policy.default_handler)],
        };
        let Some(head) = words.first().cloned() else {
            return self.command_lookup_error_value(original);
        };
        let selected = match self.lookup_original_command_at(context, &head) {
            Ok(Some(selected)) => selected,
            Ok(None) => return self.command_lookup_error_value(original),
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        if let Err(refusal) = self.reset_native_ensemble_rewrite(
            tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent::AfterSuccessfulOrdinaryLookup,
        ) {
            return refusal;
        }
        words.push(original.clone());
        words.extend_from_slice(arguments);
        let previous = self.enter_missing_handler_namespace(context);
        let completion = self.invoke_resolved_command_value_with_usage(
            &head,
            CommandSidecarKey::visible(selected.0),
            selected.1,
            &words[1..],
            &[],
        );
        self.leave_missing_handler_namespace(previous);
        completion
    }

    /// Evaluate an original command object in an actual retained namespace token,
    /// including the selected ordinary-lookup rewrite reset events.
    pub(crate) fn invoke_command_value_at(
        &mut self,
        context: tcl_core_types::NsId,
        original: &Value,
        arguments: &[Value],
        usage: &[crate::command::NativeArgumentUsageRewrite],
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
    ) -> Completion<Value> {
        self.invoke_command_value_ordinary_in_context(
            crate::interp::namespace_context::NamespaceExecutionContext::Resolved(context),
            original,
            arguments,
            usage,
            origin,
        )
    }

    pub(crate) fn invoke_command_value_ordinary_in_context(
        &mut self,
        context: crate::interp::namespace_context::NamespaceExecutionContext,
        original: &Value,
        arguments: &[Value],
        usage: &[crate::command::NativeArgumentUsageRewrite],
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
    ) -> Completion<Value> {
        let reached =
            self.invoke_command_value_entry_at(&context, original, arguments, usage, origin, true);
        drop(context);
        reached
    }

    pub(crate) fn invoke_command_value_internal_at(
        &mut self,
        context: tcl_core_types::NsId,
        original: &Value,
        arguments: &[Value],
        usage: &[crate::command::NativeArgumentUsageRewrite],
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
    ) -> Completion<Value> {
        self.invoke_command_value_in_context(
            crate::interp::namespace_context::NamespaceExecutionContext::Resolved(context),
            original,
            arguments,
            usage,
            origin,
        )
    }

    /// Internal relays retain an unresolved original execution namespace.
    pub(crate) fn invoke_command_value_in_context(
        &mut self,
        context: crate::interp::namespace_context::NamespaceExecutionContext,
        original: &Value,
        arguments: &[Value],
        usage: &[crate::command::NativeArgumentUsageRewrite],
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
    ) -> Completion<Value> {
        let reached =
            self.invoke_command_value_entry_at(&context, original, arguments, usage, origin, false);
        drop(context);
        reached
    }

    fn prepare_command_value_entry(
        &mut self,
        context: &crate::interp::namespace_context::NamespaceExecutionContext,
        original: &Value,
        ordinary: bool,
    ) -> Result<(Option<String>, u64), Completion<Value>> {
        use tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent as Event;
        if ordinary
            && let Err(refusal) = self.reset_native_ensemble_rewrite(Event::BeforeOrdinaryLookup)
        {
            return Err(refusal);
        }
        let key = match self.resolve_original_command_key_in_context(context, original) {
            Ok(key) => key,
            Err(error) => return Err(crate::command::completion_from_cmd_error(self, error)),
        };
        if ordinary
            && key.is_some()
            && let Err(refusal) =
                self.reset_native_ensemble_rewrite(Event::AfterSuccessfulOrdinaryLookup)
        {
            return Err(refusal);
        }
        let usage_revision = self.native_invocation.usage_revision;
        if let Some(refused) = self.refused_completion() {
            return Err(refused);
        }
        if let Some(exceeded) = self.charge_command() {
            return Err(exceeded);
        }
        Ok((key, usage_revision))
    }

    fn invoke_command_value_entry_at(
        &mut self,
        context: &crate::interp::namespace_context::NamespaceExecutionContext,
        original: &Value,
        arguments: &[Value],
        usage: &[crate::command::NativeArgumentUsageRewrite],
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
        ordinary: bool,
    ) -> Completion<Value> {
        let original_scope = match NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let reached = self.invoke_command_value_entry_at_in_original_scope(
            context,
            (original, arguments),
            usage,
            origin,
            ordinary,
            &original_scope,
        );
        self.finish_original_completion(&original_scope, reached)
    }

    fn invoke_command_value_entry_at_in_original_scope(
        &mut self,
        context: &crate::interp::namespace_context::NamespaceExecutionContext,
        invocation: (&Value, &[Value]),
        usage: &[crate::command::NativeArgumentUsageRewrite],
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
        ordinary: bool,
        original_scope: &NativeOperationScope,
    ) -> Completion<Value> {
        let (original, arguments) = invocation;
        let (key, usage_revision) =
            match self.prepare_command_value_entry(context, original, ordinary) {
                Ok(key) => key,
                Err(completion) => return completion,
            };
        if !self.exec_traces_can_fire() {
            let result =
                self.invoke_command_value_inner(context, original, arguments, None, usage, origin);
            if result.code == Code::Error {
                self.observe_native_error_result(&result.result);
            }
            return result;
        }
        let mut words = vec![original.clone()];
        words.extend_from_slice(arguments);
        let cmd_string = Value::list(words);
        for scope in self.step_scopes_to_fire() {
            if scope.active_for("enterstep") && self.exec_trace_entry_live(&scope.key, &scope.entry)
            {
                let result = self.run_cmd_trace_callback(
                    &scope.entry,
                    &[cmd_string.clone(), Value::string("enterstep")],
                );
                if let Some(refused) = self.original_scope_refusal(original_scope) {
                    return refused;
                }
                if !result.code.is_ok() {
                    return result;
                }
            }
        }
        let sidecar_key =
            self.renamed_command_key(CommandSidecarKey::visible(key.unwrap_or_default()));
        let sidecar = self.active_sidecar(sidecar_key.clone());
        let own = self
            .exec_traces
            .get(&sidecar_key)
            .cloned()
            .unwrap_or_default();
        for entry in own.iter().rev() {
            if !sidecar.is_attached() {
                break;
            }
            if self.exec_trace_entry_live(&sidecar, entry) && entry.has_op("enter") {
                let result = self
                    .run_cmd_trace_callback(entry, &[cmd_string.clone(), Value::string("enter")]);
                if let Some(refused) = self.original_scope_refusal(original_scope) {
                    return refused;
                }
                if !result.code.is_ok() {
                    return result;
                }
            }
        }
        let resolved = match self.resolve_original_command_key_in_context(context, original) {
            Ok(key) => key
                .map(CommandSidecarKey::visible)
                .map(|key| self.renamed_command_key(key)),
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let post = if resolved.as_ref() == sidecar.key().as_ref() {
            sidecar
                .key()
                .map(|key| self.exec_traces.get(&key).cloned().unwrap_or_default())
        } else {
            None
        };
        let leave_owner = (!own.is_empty() && post.is_some()).then(|| sidecar.clone());
        let step_scopes = self.push_exec_step_scopes(&sidecar, post.as_deref().unwrap_or_default());
        let ctx = ExecLeaveCtx {
            original_scopes: vec![OriginalExecutionOwner::Entered(original_scope.clone())],
            traces: true,
            jim_commands: Vec::new(),
            cmd_string: Some(cmd_string),
            leave_owner,
            step_scopes,
        };
        let usage = if usage_revision == self.native_invocation.usage_revision {
            usage
        } else {
            &[]
        };
        let result = self.invoke_command_value_inner(
            context,
            original,
            arguments,
            Some(&sidecar),
            usage,
            origin,
        );
        let result = self.settle_original_exec_leave(ctx, result);
        if result.code == Code::Error {
            self.observe_native_error_result(&result.result);
        }
        result
    }

    /// Dispatch a fully-resolved command by `name` + `argv` to a completion —
    /// the `Commands::dispatch` engine. Unlike [`dispatch_words`](Self::dispatch_words)
    /// (bytecode-frame-coupled: it pushes the result onto `f.stack` and defers a
    /// proc call to the trampoline as a `Tick::Call`), this is self-contained and
    /// usable outside the bytecode loop: a builtin runs inline, a proc body runs
    /// to completion in a nested `is_proc` activation (so its call-frame and
    /// `return` are handled), and an alias re-evaluates its target prefix.
    pub fn invoke_command(&mut self, name: &str, argv: &[Value]) -> Completion<Value> {
        self.host_execution_depth += 1;
        let completion = self.invoke_host_object_vector(name, argv);
        self.host_execution_depth -= 1;
        assert!(
            self.activation_depth != 0
                || self.host_execution_depth != 0
                || self.execution_refusal.is_none(),
            "command requires a genuine native expression provider"
        );
        completion
    }

    /// Invoke an actual command while keeping neutral host refusals outside
    /// every Tcl completion code and guest control handler.
    ///
    /// # Errors
    /// Returns an expression-provider refusal after any preceding effects.
    pub fn try_invoke_command(
        &mut self,
        name: &str,
        argv: &[Value],
    ) -> Result<Completion<Value>, tcl_runtime_api::NativeExecutionError> {
        self.host_execution_depth += 1;
        let completion = self.invoke_host_object_vector(name, argv);
        self.host_execution_depth -= 1;
        self.finish_host_execution(completion)
    }

    /// Public object-vector evaluation has a native error-log frontier that
    /// internal command relays and bytecode invocation do not share.
    fn invoke_host_object_vector(&mut self, name: &str, argv: &[Value]) -> Completion<Value> {
        let original_head = Value::new_native_string_bytes(name.as_bytes());
        self.invoke_host_original_object_vector(&original_head, argv)
    }

    /// Actual object-vector entry, preserving the original counted command name.
    pub(crate) fn invoke_host_original_object_vector(
        &mut self,
        original_head: &Value,
        argv: &[Value],
    ) -> Completion<Value> {
        if self.uses_jim_error_stack() {
            return self.invoke_native_jim_object_vector(original_head, argv);
        }
        let completion = self.invoke_command_value_at(
            self.current_ns_id(),
            original_head,
            argv,
            &[],
            tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
        );
        self.log_original_object_vector_completion(original_head, argv, completion)
    }

    pub(crate) fn log_original_object_vector_completion(
        &mut self,
        original_head: &Value,
        argv: &[Value],
        completion: Completion<Value>,
    ) -> Completion<Value> {
        if completion.code != Code::Error || self.execution_refusal.is_some() {
            return completion;
        }
        let Some(protocol) = self
            .actual_native_invocation_dialect()
            .native_error_log_protocol()
        else {
            return completion;
        };
        if protocol.observes_error_words(self.command_error_is_logged()) {
            let (text, retained_words) = if protocol.retains_list_words() {
                let mut words = Vec::with_capacity(argv.len() + 1);
                words.push(original_head.clone());
                words.extend_from_slice(argv);
                let original_list =
                    Value::native_list_constructor(words, protocol.string_protocol());
                match original_list.native_string_bytes(protocol.string_protocol()) {
                    Ok(bytes) => (bytes, Some(original_list)),
                    Err(error) => return self.refuse_host_command(error.to_string()),
                }
            } else {
                let mut words = Vec::with_capacity(argv.len() + 1);
                for original in std::iter::once(original_head).chain(argv) {
                    match original.native_string_bytes(protocol.string_protocol()) {
                        Ok(bytes) => words.push(bytes),
                        Err(error) => return self.refuse_host_command(error.to_string()),
                    }
                }
                (
                    Rc::<[u8]>::from(protocol.object_vector_command(&words)),
                    None,
                )
            };
            let message = match completion
                .result
                .native_string_bytes(protocol.string_protocol())
            {
                Ok(bytes) => bytes,
                Err(error) => return self.refuse_host_command(error.to_string()),
            };
            self.log_command_info(&text, &message, 1);
            drop(retained_words);
        }
        self.clear_error_logged();
        completion
    }

    fn invoke_native_jim_object_vector(
        &mut self,
        original: &Value,
        arguments: &[Value],
    ) -> Completion<Value> {
        let mut values = Vec::with_capacity(arguments.len() + 1);
        values.push(original.clone());
        values.extend_from_slice(arguments);
        // Jim_EvalObjVector retains every actual argument for the duration of
        // the invocation; frame metadata borrows that same vector.
        let words = crate::NativeListItems::invocation_view(Rc::new(values));
        let frame_index = self.jim_errors.frames.len();
        let parent = self
            .jim_errors
            .frames
            .last()
            .map(Self::borrow_jim_evaluation_frame);
        self.jim_errors
            .frames
            .push(crate::interp::OriginalJimEvaluationFrame {
                evaluation_frame_entered: true,
                procedure_level: parent.as_ref().map_or(0, |frame| frame.procedure_level),
                command: None,
                is_procedure: false,
                script: parent.and_then(|frame| frame.script),
                invocation: Value::empty(),
            });
        self.jim_errors.vectors.push(NativeJimVectorFrame {
            frame_index,
            words: words.lifetime_view(),
        });
        let completion = self.invoke_command_value_ordinary_in_context(
            self.current_namespace_execution_context(),
            &words[0],
            &words[1..],
            &[],
            tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
        );
        let invoked = self
            .jim_errors
            .frames
            .get(frame_index)
            .is_some_and(|frame| frame.command.is_some());
        if invoked && completion.code == Code::Error && self.execution_refusal.is_none() {
            self.capture_jim_error_stack();
        }
        let vector = self
            .jim_errors
            .vectors
            .pop()
            .expect("entered Jim vector frame");
        self.jim_errors.frames.truncate(vector.frame_index);
        // Pop the borrowed evaluation frame before releasing actual argv refs.
        drop(vector);
        drop(words);
        completion
    }

    fn enter_native_jim_vector_command(&mut self, original: &Value, command: &Command) {
        let Some(vector) = self.jim_errors.vectors.last() else {
            return;
        };
        if !vector.words[0].is_same_object(original) {
            return;
        }
        let frame_index = vector.frame_index;
        let invocation = Value::invocation_list_view(&vector.words);
        let name = self.original_jim_command_name_reference(original);
        let Some(frame) = self.jim_errors.frames.get_mut(frame_index) else {
            return;
        };
        frame.command = name;
        frame.is_procedure = matches!(command, Command::Proc(_));
        frame.invocation = invocation;
    }

    /// Invoke an already-resolved command without consulting the visible
    /// command table.  Hidden-command dispatch uses this to keep a hidden
    /// binding private while preserving its display spelling and trace key.
    pub(crate) fn invoke_resolved_command(
        &mut self,
        display_name: &str,
        trace_key: CommandSidecarKey,
        command: Command,
        argv: &[Value],
    ) -> Completion<Value> {
        self.invoke_resolved_command_with_usage(display_name, trace_key, command, argv, &[])
    }

    fn invoke_resolved_command_with_usage(
        &mut self,
        display_name: &str,
        trace_key: CommandSidecarKey,
        command: Command,
        argv: &[Value],
        usage: &[crate::command::NativeArgumentUsageRewrite],
    ) -> Completion<Value> {
        self.invoke_resolved_command_value_with_usage(
            &Value::string(display_name),
            trace_key,
            command,
            argv,
            usage,
        )
    }

    pub(crate) fn invoke_resolved_command_value_with_usage(
        &mut self,
        original: &Value,
        trace_key: CommandSidecarKey,
        command: Command,
        argv: &[Value],
        usage: &[crate::command::NativeArgumentUsageRewrite],
    ) -> Completion<Value> {
        let original_scope = match NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let reached = self.invoke_resolved_command_value_with_usage_in_original_scope(
            original,
            trace_key,
            command,
            argv,
            usage,
            &original_scope,
        );
        self.finish_original_completion(&original_scope, reached)
    }

    fn invoke_resolved_command_value_with_usage_in_original_scope(
        &mut self,
        original: &Value,
        trace_key: CommandSidecarKey,
        command: Command,
        argv: &[Value],
        usage: &[crate::command::NativeArgumentUsageRewrite],
        original_scope: &NativeOperationScope,
    ) -> Completion<Value> {
        if let Some(exceeded) = self.charge_command() {
            return exceeded;
        }
        if !self.exec_traces_can_fire() {
            return self.invoke_resolved_command_inner(
                original,
                command,
                argv,
                Some(trace_key),
                None,
                usage,
            );
        }
        let mut words = Vec::with_capacity(argv.len() + 1);
        words.push(original.clone());
        words.extend_from_slice(argv);
        let cmd_string = Value::list(words);
        // As in `dispatch_words_traced`: a rename window moved the trace list
        // to the destination key, and both names reach it.
        let trace_key = self.renamed_command_key(trace_key);
        for scope in self.step_scopes_to_fire() {
            if scope.active_for("enterstep") && self.exec_trace_entry_live(&scope.key, &scope.entry)
            {
                let r = self.run_cmd_trace_callback(
                    &scope.entry,
                    &[cmd_string.clone(), Value::string("enterstep")],
                );
                if let Some(refused) = self.original_scope_refusal(original_scope) {
                    return refused;
                }
                if !r.code.is_ok() {
                    return r;
                }
            }
        }
        let sidecar = self.active_sidecar(trace_key.clone());
        let own_at_entry = self
            .exec_traces
            .get(&trace_key)
            .cloned()
            .unwrap_or_default();
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
                    return refused;
                }
                if !r.code.is_ok() {
                    return r;
                }
            }
        }
        let post_enter = self.post_enter_trace_snapshot(None, None, &sidecar, None);
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
        let Some(live_key) = sidecar.key() else {
            return self
                .settle_original_exec_leave(ctx, err("attempt to invoke a deleted command"));
        };
        let c = self.invoke_resolved_command_inner(
            original,
            command,
            argv,
            Some(live_key),
            Some(&sidecar),
            usage,
        );
        self.settle_original_exec_leave(ctx, c)
    }

    /// Settle a synchronously-run native command on the native re-entry path
    /// ([`Self::invoke_command`]): a deferred body runs via a nested drive,
    /// else the completion is the command's own. The
    /// [`dispatch_words`](Self::dispatch_words) twin is
    /// [`settle_native_dispatch`](Self::settle_native_dispatch); the two differ
    /// only in having a trampoline to push onto.
    /// Keep the `-errorcode` a host command's error names. The interpreter
    /// holds the error code as its own state, and a host command states it in
    /// its completion's options, as a C command does with `Tcl_SetErrorCode`.
    pub(super) fn adopt_host_error_code(&mut self, completion: &Completion<Value>) {
        if completion.code != Code::Error {
            return;
        }
        if let Ok(Some(code)) =
            crate::command::opt_get_checked(self, &completion.options, b"-errorcode")
        {
            self.retain_return_error_code(Some(&code), false);
        }
    }

    pub(crate) fn settle_native_invoke(&mut self, res: Completion<Value>) -> Completion<Value> {
        if let Some(refused) = self.refused_completion() {
            return refused;
        }
        // The deferred activation is built out of line (`take_native_settlement`)
        // so none of its construction temporaries (a `Frame` is about 5 KB)
        // stay on the native stack while the body runs: a natively settled
        // `eval`/`catch`/`try`/lambda body can recurse back through here.
        match self.take_native_settlement() {
            NativeSettlement::Unchanged => res,
            NativeSettlement::Finished(completion) => completion,
            NativeSettlement::Run(frame) => self.run_activation(frame),
            NativeSettlement::Catch(frame, epilogue) => {
                let completion = self.run_activation(frame);
                self.finish_native_catch(completion, *epilogue)
            }
        }
    }

    /// Take what a natively settled invocation deferred, in the order
    /// [`Self::settle_native_invoke`] runs it, as a boxed activation.
    #[inline(never)]
    fn take_native_settlement(&mut self) -> NativeSettlement {
        if let Some(req) = self.pending.procedure_call.take() {
            return match self.enter_proc(
                &req.proc.proc,
                &req.proc.body,
                &req.invoked,
                &req.arguments,
            ) {
                Ok(()) => {
                    self.install_native_procedure_binding(req.proc.declaration_binding);
                    let mut frame = Box::new(Frame::new(req.proc.body, true));
                    frame.lambda_registration = Some(req.registration);
                    NativeSettlement::Run(frame)
                }
                Err(completion) => {
                    self.retire_original_lambda_registration(&req.registration);
                    NativeSettlement::Finished(completion)
                }
            };
        }
        if let Some(req) = self.pending.expression.take() {
            let placeholder = self.current_placeholder_unit();
            return NativeSettlement::Run(Box::new(Frame::new_expression(req, placeholder)));
        }
        if let Some(state) = self.pending.control.take() {
            let placeholder = self.current_placeholder_unit();
            return NativeSettlement::Run(Box::new(Frame::new_control(state, placeholder)));
        }
        // An `eval`/`uplevel`/`apply`-style builtin deferred its body to
        // the pending eval request. This call site is on the native Rust stack (no
        // trampoline to push onto), so run the body via a nested drive —
        // a `yield` inside cannot cross it, exactly like every other
        // `invoke_command` re-entry.
        if let Some(req) = self.pending.eval.take() {
            return NativeSettlement::Run(Box::new(Frame::new_eval(req)));
        }
        // A `catch` deferred its body: run it via a nested drive (a `yield`
        // inside cannot cross this native re-entry), then absorb its
        // completion with the catch epilogue.
        if let Some(req) = self.pending.catch.take() {
            let CatchReq {
                script,
                resvar,
                optvar,
                ignored_codes,
                fatal_tail,
                evaluates_word: _,
            } = req;
            return NativeSettlement::Catch(
                Box::new(Frame::new_script(script, None)),
                Box::new(NativeCatchEpilogue {
                    resvar,
                    optvar,
                    ignored_codes,
                    fatal_tail,
                }),
            );
        }
        // A `subst` deferred: run the scanner-driven subst frame via a
        // nested drive (its `[…]` bodies can't yield across this native
        // re-entry), returning its accumulated result.
        if let Some(req) = self.pending.subst.take() {
            let placeholder = self.current_placeholder_unit();
            return NativeSettlement::Run(Box::new(Frame::new_subst(req, placeholder)));
        }
        // A `foreach`/`lmap` runtime-fallback loop deferred: run the
        // scanner-driven each-loop frame via a nested drive (a `yield` in
        // its body can't cross this native re-entry either).
        if let Some(req) = self.pending.each_loop.take() {
            let placeholder = self.current_placeholder_unit();
            return NativeSettlement::Run(Box::new(Frame::new_each_loop(req, placeholder)));
        }
        // A `try` deferred its body: run it (and, via `Vm::unwind`'s own
        // `try_ctx` handling, every subsequent phase `advance_try`
        // decides on) via a nested drive — a `yield` anywhere in it
        // can't cross this native re-entry either. `unwind` pushes each
        // next phase onto the same nested `acts`, so one
        // `run_activation` call carries the whole construct through to
        // its final completion.
        if let Some(req) = self.pending.try_phase.take() {
            return NativeSettlement::Run(Box::new(Frame::new_try(req, Value::empty())));
        }
        NativeSettlement::Unchanged
    }

    /// The `catch` epilogue of [`Self::settle_native_invoke`], out of line.
    #[inline(never)]
    fn finish_native_catch(
        &mut self,
        mut completion: Completion<Value>,
        epilogue: NativeCatchEpilogue,
    ) -> Completion<Value> {
        if completion.code == Code::Ok
            && let Some(tail) = epilogue.fatal_tail
        {
            completion = self.raise_fatal_tail(tail);
        }
        self.finish_catch(
            completion,
            epilogue.resvar.as_ref(),
            epilogue.optvar.as_ref(),
            epilogue.ignored_codes,
        )
    }

    fn invoke_command_value_inner(
        &mut self,
        context: &crate::interp::namespace_context::NamespaceExecutionContext,
        original: &Value,
        argv: &[Value],
        sidecar_handle: Option<&CommandSidecarHandle>,
        usage: &[crate::command::NativeArgumentUsageRewrite],
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
    ) -> Completion<Value> {
        match self.lookup_original_command_in_context(context, original) {
            Ok(Some((key, command))) => {
                let sidecar = Some(CommandSidecarKey::visible(key));
                let handle =
                    sidecar_handle.filter(|handle| handle.key().as_ref() == sidecar.as_ref());
                self.invoke_resolved_command_inner(original, command, argv, sidecar, handle, usage)
            }
            Ok(None) => match context.resolved_token() {
                Some(namespace) => {
                    self.invoke_missing_command_value(namespace, original, argv, origin)
                }
                None => self.refuse_host_command(
                    "missing command fallback namespace coordinates are unavailable".into(),
                ),
            },
            Err(error) => crate::command::completion_from_cmd_error(self, error),
        }
    }

    fn invoke_resolved_command_inner(
        &mut self,
        original: &Value,
        command: Command,
        argv: &[Value],
        sidecar: Option<CommandSidecarKey>,
        sidecar_handle: Option<&CommandSidecarHandle>,
        usage: &[crate::command::NativeArgumentUsageRewrite],
    ) -> Completion<Value> {
        self.invoke_resolved_command_with_entry(
            original,
            command,
            argv,
            ResolvedCommandEntry {
                sidecar,
                handle: sidecar_handle,
                direct_import: None,
            },
            usage,
        )
    }

    /// C8.4 calls the already selected global objProc without ordinary lookup,
    /// evaluator reset, execution traces, or an invented invoking command word.
    pub(crate) fn invoke_original_c_import_preload_direct(
        &mut self,
        selected: crate::interp::OriginalCImportPreload,
        head: &Value,
        operand: &Value,
        original: &crate::interp::native_name_consumers::OriginalNameConsumerScope,
    ) -> Completion<Value> {
        let reached = (|| {
            original.current_for_value(self, head)?;
            original.current_for_value(self, operand)?;
            let command = selected.command();
            let sidecar = selected.key();
            let completion = self.invoke_resolved_command_with_entry(
                head,
                command,
                std::slice::from_ref(operand),
                ResolvedCommandEntry {
                    sidecar: Some(sidecar),
                    handle: None,
                    direct_import: Some(selected),
                },
                &[],
            );
            original.current_for_value(self, head)?;
            original.current_for_value(self, operand)?;
            original.check(self)?;
            Ok(completion)
        })();
        reached.unwrap_or_else(|error| crate::command::completion_from_cmd_error(self, error))
    }

    fn invoke_resolved_command_with_entry(
        &mut self,
        original: &Value,
        command: Command,
        argv: &[Value],
        entry: ResolvedCommandEntry<'_>,
        usage: &[crate::command::NativeArgumentUsageRewrite],
    ) -> Completion<Value> {
        let ResolvedCommandEntry {
            sidecar,
            handle: sidecar_handle,
            direct_import,
        } = entry;
        // The native reentry path borrows the actual complete vector just as
        // bytecode dispatch_builtin_words does. These views add no native +1.
        let mut words = Vec::with_capacity(argv.len() + 1);
        words.push(original.native_lifetime_lease().into_value());
        words.extend(
            argv.iter()
                .map(|word| word.native_lifetime_lease().into_value()),
        );
        let previous_arguments = self
            .native_invocation
            .arguments
            .replace(crate::NativeListItems::invocation_view(Rc::new(words)));
        let selected_key = sidecar
            .as_ref()
            .and_then(|key| match key {
                CommandSidecarKey::Visible(key) => Some(key.as_str()),
                CommandSidecarKey::Hidden(_) => None,
            })
            .unwrap_or("");
        let jim_lease = self.retain_original_jim_command(original, Some(selected_key));
        self.enter_native_jim_vector_command(original, &command);
        let saved_handler = self.take_native_handler_metadata();
        let usage_scope = self.enter_native_usage_scope(usage);
        let entered = if let Some(selected) = direct_import.as_ref() {
            selected.install_invoked_head(self, original)
        } else {
            self.set_invoked_name_value(original, selected_key);
            self.reset_native_jim_result()
                .map_err(tcl_cmd_core::CmdError::from)
        };
        if let Err(error) = entered {
            self.leave_native_usage_scope(usage_scope);
            self.restore_native_handler_metadata(saved_handler);
            drop(direct_import);
            drop(jim_lease);
            let retired =
                std::mem::replace(&mut self.native_invocation.arguments, previous_arguments);
            drop(retired);
            return crate::command::completion_from_cmd_error(self, error);
        }
        let result = self.invoke_selected_command_worker(
            original,
            command,
            argv,
            sidecar,
            sidecar_handle,
            usage,
        );
        self.leave_native_usage_scope(usage_scope);
        self.restore_native_handler_metadata(saved_handler);
        let completion = self
            .publish_native_interp_completion(result)
            .unwrap_or_else(|error| crate::command::completion_from_tcl_error(self, error));
        drop(direct_import);
        // Native result ownership is settled before the invoked node retires;
        // its cleanup still observes this actual argv until it has finished.
        drop(jim_lease);
        let retired = std::mem::replace(&mut self.native_invocation.arguments, previous_arguments);
        drop(retired);
        completion
    }

    fn invoke_selected_command_worker(
        &mut self,
        original: &Value,
        command: Command,
        argv: &[Value],
        sidecar: Option<CommandSidecarKey>,
        sidecar_handle: Option<&CommandSidecarHandle>,
        usage: &[crate::command::NativeArgumentUsageRewrite],
    ) -> Completion<Value> {
        match command {
            Command::Builtin(bf) => {
                let saved = std::mem::replace(&mut self.native_invocation.sidecar, sidecar);
                let res = bf(self, argv);
                self.native_invocation.sidecar = saved;
                self.settle_native_invoke(res)
            }
            Command::Native(cmd) => {
                let saved = std::mem::replace(&mut self.native_invocation.sidecar, sidecar);
                self.native_command_depth += 1;
                let res = cmd.invoke(self, argv);
                self.native_command_depth -= 1;
                self.native_invocation.sidecar = saved;
                self.adopt_host_error_code(&res);
                self.settle_native_invoke(res)
            }
            Command::Proc(p) => {
                match self.early_procedure_activation(&p, argv) {
                    Ok(Some(result)) => return result,
                    Ok(None) => {}
                    Err(error) => return error,
                }
                let p = match self.ensure_proc_ready_in(p, sidecar.as_ref(), sidecar_handle) {
                    Ok(proc) => proc,
                    Err(error) => {
                        return crate::command::completion_from_tcl_error(self, error);
                    }
                };
                match self.enter_proc(&p.proc, &p.body, original, argv) {
                    Ok(()) => {
                        self.install_native_procedure_binding(p.declaration_binding);
                        self.run_activation(Box::new(Frame::new(p.body, true)))
                    }
                    Err(c) => c,
                }
            }
            Command::CallerAlias(original_prefix) => {
                let target = match original_prefix.native_object_list_elements(
                    tcl_syntax::native_string::NativeStringProtocol::Jim084,
                ) {
                    Ok(target) => target,
                    Err(error) => {
                        return crate::command::completion_from_cmd_error(self, error.into());
                    }
                };
                let mut full: Vec<Value> = target.as_ref().clone();
                full.extend_from_slice(argv);
                let usage = self.alias_usage_rewrites_value(original, target.len(), usage);
                self.invoke_alias_words_with_usage(
                    self.cur_interp(),
                    &full,
                    tcl_registry::AliasTargetLookup::CallerNamespace,
                    &usage,
                )
            }
            Command::Alias { words: target, .. } => {
                // Target resolves in the GLOBAL namespace, caller's frame kept
                // — see the `dispatch_words` Alias arm for the tclsh pins.
                let mut full: Vec<Value> = (*target).clone();
                full.extend_from_slice(argv);
                let here = self.cur_interp();
                let usage = self.alias_usage_rewrites_value(original, target.len(), usage);
                self.invoke_alias_words_with_usage(
                    here,
                    &full,
                    tcl_registry::AliasTargetLookup::Global,
                    &usage,
                )
            }
            // Cross-interp alias: switch to the target interp and run the words
            // there.  Identical to the bytecode path — a cross-interp alias
            // works from a native re-entry (coroutine resume, `lsort
            // -command`, `invoke_command`), unlike C Tcl's shared C stack,
            // where the same re-entry raises `cannot invoke parent-interp
            // alias: C stack busy`.
            Command::CrossAlias {
                target: target_interp,
                words: target,
                original_jim_prefix,
                ..
            } => {
                if let Some(prefix) = original_jim_prefix {
                    self.invoke_original_jim_parent_alias(target_interp, &prefix, argv)
                } else {
                    let mut full: Vec<Value> = (*target).clone();
                    full.extend_from_slice(argv);
                    let usage = self.alias_usage_rewrites_value(original, target.len(), usage);
                    self.invoke_alias_words_with_usage(
                        target_interp,
                        &full,
                        tcl_registry::AliasTargetLookup::Global,
                        &usage,
                    )
                }
            }
            Command::ChildInterp(child) => self.dispatch_child(original, child, argv),
            Command::Ensemble(e) => self.dispatch_ensemble("", &e, argv),
            Command::Object(key) => crate::cmd_oo::oo_dispatch(self, key, original, argv),
            // Miss fallback chain (see `dispatch_words`): `namespace unknown`
            // handler first, then the plain `unknown` proc, then a hard error.
        }
    }
}
