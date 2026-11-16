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

//! Dispatching a `namespace ensemble` call: subcommand resolution, the wrong-arguments message and the `-unknown` handler.

use super::super::{Vm, native_ensemble_objects, native_name_consumers, native_numeric_ops};
use super::NativeCommandLookupUnavailable;
use crate::command::{EnsembleDef, err_with_code};
use crate::value::Value;
use tcl_core_types::NameBytes;
use tcl_runtime_api::{Code, Completion, Namespaces, NsId};

impl Vm {
    /// Dispatch a child-as-command call (`$child sub ?arg …?`) — the `interp`
    /// ensemble restricted to that child.
    /// The exported command tails of namespace `ns` (the default ensemble
    /// subcommand set): commands directly in `ns` whose tail matches an export
    /// pattern.
    pub(crate) fn native_namespace_match(
        &self,
        purpose: tcl_syntax::native_glob::NativeNameGlobPurpose,
        pattern: &[u8],
        name: &[u8],
    ) -> Result<bool, tcl_syntax::native_glob::NativeGlobUnavailable> {
        let policy = self
            .name_policy_protocol()
            .ok_or(tcl_syntax::native_glob::NativeGlobUnavailable::ProtocolUnavailable)?;
        tcl_syntax::native_glob::NativeGlobProtocol::from_name_policy(policy)
            .match_name_pattern(purpose, pattern, name)
    }

    fn exported_command_tails(
        &self,
        namespace: NsId,
    ) -> Result<Vec<NameBytes>, tcl_syntax::native_glob::NativeGlobUnavailable> {
        let patterns = self
            .retained_record_of(namespace)
            .and_then(|record| record.exports.get(&namespace).cloned())
            .or_else(|| self.ns_exports.get(&namespace).cloned())
            .unwrap_or_default();
        let mut out = Vec::new();
        let mut keys: Vec<_> = self.commands.keys().cloned().collect();
        if let Some(record) = self.retained_record_of(namespace) {
            keys.extend(record.commands.keys().cloned());
        }
        keys.sort();
        keys.dedup();
        for key in &keys {
            let Some(slot) = self.command_slot(key) else {
                continue;
            };
            if slot.namespace != namespace || slot.simple.is_empty() {
                continue;
            }
            for pattern in &patterns {
                if self.native_namespace_match(
                    tcl_syntax::native_glob::NativeNameGlobPurpose::ExportFilter,
                    pattern.as_bytes(),
                    slot.simple.as_bytes(),
                )? {
                    out.push(slot.simple.clone());
                    break;
                }
            }
        }
        Ok(out)
    }

    /// Dispatch a `namespace ensemble` call (`ens ?param …? sub ?arg …?`):
    /// resolve `sub` against the ensemble's subcommands through the shared
    /// `tcl_cmd_core::ensemble` scan and invoke the mapped target (`-map`, else
    /// `namespace::sub`), threading any `-parameters` values in after the
    /// target prefix (`NsEnsembleImplementationCmd`, `tclEnsemble.c`).
    fn ensemble_wrong_args(
        &mut self,
        ens_name: &str,
        parameters: &[NameBytes],
    ) -> Completion<Value> {
        let scope = match native_name_consumers::OriginalNameConsumerScope::enter(self) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let reached = self.ensemble_wrong_args_in_original_name(ens_name, parameters, &scope);
        self.original_name_finish(scope, reached)
            .unwrap_or_else(|error| crate::command::completion_from_cmd_error(self, error))
    }

    fn ensemble_wrong_args_in_original_name(
        &mut self,
        ens_name: &str,
        parameters: &[NameBytes],
        original_name_scope: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Completion<Value> {
        if let Err(error) = original_name_scope.check(self) {
            return crate::command::completion_from_cmd_error(self, error);
        }

        let Some(configuration) = self
            .actual_native_invocation_dialect()
            .native_ensemble_configuration_protocol()
        else {
            return self.refuse_host_command("ensemble argument usage".into());
        };
        let mut words = vec![
            self.invoked_name_value()
                .unwrap_or_else(|| Value::string(ens_name)),
        ];
        words.extend(
            parameters
                .iter()
                .map(|name| Value::from_native_string_bytes(name.as_bytes())),
        );
        let bytes: Vec<_> = match words
            .iter()
            .map(|value| self.native_name_operand_bytes_checked(value))
            .collect::<Result<_, _>>()
        {
            Ok(bytes) => bytes,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let Some(protocol) = self.actual_native_invocation_dialect().usage_protocol(None) else {
            return self.refuse_host_command(
                "native ensemble argument usage protocol is unavailable".into(),
            );
        };
        let words: Vec<_> = bytes
            .iter()
            .map(|word| tcl_registry::native_usage::NativeUsageWord::Original(word))
            .collect();
        let Some(mut usage) = protocol.render_header(&words) else {
            return self.refuse_host_command(
                "native ensemble argument usage storage is unavailable".into(),
            );
        };
        usage.push(b' ');
        usage.extend_from_slice(configuration.missing_selector_usage());
        crate::command::native_wrong_args_bytes(self, &usage)
    }

    fn dispatch_resolved_ensemble(
        &mut self,
        ens_name: &str,
        e: &EnsembleDef,
        argv: &[Value],
        layout: &tcl_cmd_core::ensemble::InvocationLayout,
        selection: (&[NameBytes], usize),
    ) -> Completion<Value> {
        let (subs, index) = selection;
        let resolved = &subs[index];
        let params = &argv[layout.parameters.clone()];
        let rest = &argv[layout.arguments..];
        if let Err(error) = self.build_original_ensemble_table(e, subs) {
            return self.refuse_host_command(error.to_string());
        }
        let Some((prefix, mapped)) = e.originals.table.prefix(resolved) else {
            return self.refuse_host_command("ensemble selected original prefix".into());
        };
        let prefix_len = match self
            .actual_native_invocation_dialect()
            .native_string_materialization(None)
        {
            Some(recipe) => match prefix.native_object_list_elements(recipe.protocol()) {
                Ok(members) => members.len(),
                Err(error) => return self.refuse_host_command(error.to_string()),
            },
            None => {
                return self.refuse_host_command("ensemble selected List recipe".into());
            }
        };
        let mut usage = Self::borrow_native_argument_usage(&self.native_invocation.usage_rewrites);
        if self
            .actual_native_invocation_dialect()
            .argument_usage_rewriting()
            == Some(true)
        {
            let head = self
                .invoked_name_value()
                .unwrap_or_else(|| Value::string(ens_name));
            let mut original_prefix = vec![head.native_lifetime_lease().into_value()];
            drop(head);
            original_prefix.extend(
                params
                    .iter()
                    .map(|word| word.native_lifetime_lease().into_value()),
            );
            original_prefix.push(argv[layout.subcommand].native_lifetime_lease().into_value());
            usage.push(crate::command::NativeArgumentUsageRewrite {
                original_prefix,
                removed_words: prefix_len + params.len(),
            });
        }
        let (mut completion, target_was_missing) = self.invoke_original_ensemble_prefix(
            prefix,
            native_ensemble_objects::OriginalEnsembleInvocation {
                namespace: e.namespace,
                parameters: params,
                arguments: rest,
                original_word_count: argv.len() + 1,
                usage: &usage,
                default_target: !mapped,
            },
        );
        if target_was_missing && completion.code == Code::Error {
            // The default target's fully-qualified spelling is an
            // ensemble implementation detail. C rewrites only the
            // result seen by the caller; the `unknown` command owns
            // the completion options (including a custom errorCode).
            // Do not classify the error by matching user-controlled
            // message text.
            let mut message = b"invalid command name \"".to_vec();
            message.extend_from_slice(tcl_core_types::c_string_extent(resolved.as_bytes()));
            message.push(b'"');
            completion.result = Value::from_native_string_bytes(message);
        }
        completion
    }

    pub(crate) fn dispatch_ensemble(
        &mut self,
        ens_name: &str,
        token: &tcl_cmd_core::ensemble::EnsembleToken<EnsembleDef, NameBytes>,
        argv: &[Value],
    ) -> Completion<Value> {
        let mut current = token.config();
        let mut reparsed = false;
        loop {
            // `-parameters` formals sit between the ensemble command and the
            // subcommand word, so the subcommand is at `nparams`. Recompute
            // this after an empty `-unknown` result: the callback may have
            // reconfigured any part of the ensemble, not just its map.
            let e = &current;
            let Some(layout) =
                tcl_cmd_core::ensemble::invocation_layout(argv.len(), 0, e.parameters.len())
            else {
                return self.ensemble_wrong_args(ens_name, &e.parameters);
            };
            let Some(configuration) = self
                .actual_native_invocation_dialect()
                .native_ensemble_configuration_protocol()
            else {
                return self.refuse_host_command("ensemble member names".into());
            };
            let original_sub =
                match self.native_name_operand_bytes_checked(&argv[layout.subcommand]) {
                    Ok(bytes) => bytes,
                    Err(error) => return crate::command::completion_from_cmd_error(self, error),
                };
            let sub = configuration.member_name(&original_sub);
            let exports = match self.exported_command_tails(e.namespace) {
                Ok(names) => names,
                Err(error) => return self.refuse_host_command(error.to_string()),
            };
            let plan = Self::native_original_ensemble_table_plan(configuration, e, &exports);
            let subs: Vec<_> = plan
                .entries
                .into_iter()
                .map(|entry| NameBytes::from(entry.member))
                .collect();
            if let Some(index) = configuration.resolve_member(&subs, &original_sub, e.prefixes) {
                return self.dispatch_resolved_ensemble(ens_name, e, argv, &layout, (&subs, index));
            }
            if e.unknown.is_some() && !reparsed {
                reparsed = true;
                let replacement = match self.ensemble_unknown(token, e, argv) {
                    Ok(Some(prefix)) => prefix,
                    Ok(None) => {
                        current = token.config();
                        continue;
                    }
                    Err(completion) => return completion,
                };
                // Tcl splices the replacement prefix with the parameter and
                // argument ranges from the live post-callback configuration.
                // The callback may have changed `-parameters` even when it
                // returns a non-empty prefix, so the pre-callback slices above
                // are no longer authoritative.
                current = token.config();
                let Some(live_layout) = tcl_cmd_core::ensemble::invocation_layout(
                    argv.len(),
                    0,
                    current.parameters.len(),
                ) else {
                    return self.ensemble_wrong_args(ens_name, &current.parameters);
                };
                return self
                    .invoke_original_ensemble_prefix(
                        replacement,
                        native_ensemble_objects::OriginalEnsembleInvocation {
                            namespace: current.namespace,
                            parameters: &argv[live_layout.parameters],
                            arguments: &argv[live_layout.arguments..],
                            original_word_count: argv.len() + 1,
                            usage: &[],
                            default_target: false,
                        },
                    )
                    .0;
            }
            let message = tcl_cmd_core::ensemble::unknown_subcommand_message(
                &subs,
                sub,
                e.prefixes,
                &Namespaces::name_bytes(self, e.namespace),
            );
            let mut code = b"TCL LOOKUP SUBCOMMAND ".to_vec();
            tcl_syntax::list::append_list_element(&mut code, sub, false);
            return crate::command::completion_from_cmd_error(
                self,
                tcl_cmd_core::CmdError::with_error_code_bytes(message, code),
            );
        }
    }

    /// Invoke an ensemble's `-unknown` prefix. A successful empty list asks the
    /// caller to reparse once; a non-empty list is its replacement command
    /// prefix. The handler sees the ensemble's fully-qualified command name.
    fn ensemble_unknown(
        &mut self,
        token: &tcl_cmd_core::ensemble::EnsembleToken<EnsembleDef, NameBytes>,
        definition: &EnsembleDef,
        argv: &[Value],
    ) -> Result<Option<Value>, Completion<Value>> {
        let scope = native_name_consumers::OriginalNameConsumerScope::enter(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = self.ensemble_unknown_in_original_name(token, definition, argv, &scope);
        self.original_name_finish(scope, reached)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?
    }

    fn finish_ensemble_unknown(
        &mut self,
        token: &tcl_cmd_core::ensemble::EnsembleToken<EnsembleDef, NameBytes>,
        completion: Completion<Value>,
        handler_call: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        original_name_scope: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<Option<Value>, Completion<Value>> {
        match completion.code {
            Code::Ok => {}
            Code::Error => {
                self.seed_error_info_frame(
                    completion.result.string_bytes(),
                    "\n    (ensemble unknown subcommand handler)",
                );
                return Err(completion);
            }
            bad_code => {
                let name = match bad_code {
                    Code::Return => "return".to_owned(),
                    Code::Break => "break".to_owned(),
                    Code::Continue => "continue".to_owned(),
                    Code::Other(code) => code.to_string(),
                    Code::Ok | Code::Error => unreachable!("handled above"),
                };
                let message = format!("unknown subcommand handler returned bad code: {name}");
                let bytes = match self.native_name_operand_bytes_checked(handler_call) {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        return Err(crate::command::completion_from_cmd_error(self, error));
                    }
                };
                let mut frame = b"\n    result of ensemble unknown subcommand handler: ".to_vec();
                frame.extend_from_slice(&bytes);
                // `Tcl_ResetResult` drops the handler's return options first.
                self.reset_native_error_result();
                self.seed_error_info_frame(&message, &frame);
                return Err(err_with_code(message, "TCL ENSEMBLE UNKNOWN_RESULT"));
            }
        }
        if token.is_deleted() {
            let message = tcl_cmd_core::ensemble::UNKNOWN_DELETED_MESSAGE;
            self.seed_error_info_frame(message, "\n    (ensemble unknown subcommand handler)");
            return Err(err_with_code(
                message,
                tcl_cmd_core::ensemble::UNKNOWN_DELETED_ERROR_CODE,
            ));
        }
        let reached = native_numeric_ops::OriginalNumericOps::in_original_scope(
            self,
            original_name_scope.operation_scope(),
        )
        .and_then(|ops| ops.list(&completion.result, protocol));
        let members = match self.original_name_outcome(original_name_scope, reached) {
            Ok(members) => members,
            Err(error) => {
                let failure = crate::command::completion_from_cmd_error(self, error);
                if self.execution_refusal.is_none() {
                    self.seed_error_info_frame(
                        failure.result.string_bytes(),
                        "\n    while parsing result of ensemble unknown subcommand handler",
                    );
                }
                return Err(failure);
            }
        };
        let nonempty = !members.is_empty();
        drop(members);
        original_name_scope
            .check(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        Ok(nonempty.then_some(completion.result))
    }

    fn ensemble_unknown_in_original_name(
        &mut self,
        token: &tcl_cmd_core::ensemble::EnsembleToken<EnsembleDef, NameBytes>,
        definition: &EnsembleDef,
        argv: &[Value],
        original_name_scope: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<Option<Value>, Completion<Value>> {
        original_name_scope
            .check(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;

        let protocol = self
            .actual_native_invocation_dialect()
            .native_string_materialization(None)
            .ok_or_else(|| {
                self.refuse_host_command("ensemble unknown original List protocol".into())
            })?
            .protocol();
        let reached = definition
            .originals
            .unknown
            .as_ref()
            .and_then(|role| {
                role.inspect(|root| {
                    root.inspect(|root| {
                        original_name_scope.current_for_value(self, root)?;
                        let duplicate = root.duplicate_native_object_in(protocol);
                        original_name_scope.current_for_value(self, root)?;
                        Ok::<_, tcl_cmd_core::CmdError>(duplicate)
                    })
                })
                .flatten()
            })
            .ok_or_else(|| {
                self.refuse_host_command("ensemble unknown configured original root".into())
            })?;
        let handler_call = self
            .original_name_outcome(original_name_scope, reached)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let checked = native_numeric_ops::OriginalNumericOps::in_original_scope(
            self,
            original_name_scope.operation_scope(),
        )
        .and_then(|ops| ops.list(&handler_call, protocol));
        let prepared = self
            .original_name_outcome(original_name_scope, checked)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        drop(prepared);
        original_name_scope
            .check(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let mut additions = vec![Value::from_native_string_bytes(token.name().as_bytes())];
        additions.extend_from_slice(argv);
        let reached = handler_call
            .native_list_append_prepared_elements(&additions, protocol)
            .map_err(Into::into);
        drop(additions);
        self.original_name_outcome(original_name_scope, reached)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = native_numeric_ops::OriginalNumericOps::in_original_scope(
            self,
            original_name_scope.operation_scope(),
        )
        .and_then(|ops| ops.list(&handler_call, protocol));
        let members = self
            .original_name_outcome(original_name_scope, reached)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let completion = self.invoke_command_value_ordinary_in_context(
            self.current_namespace_execution_context(),
            &members[0],
            &members[1..],
            &[],
            tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
        );
        drop(members);
        original_name_scope
            .check(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        self.finish_ensemble_unknown(
            token,
            completion,
            &handler_call,
            protocol,
            original_name_scope,
        )
    }

    /// Counted ensemble map construction from an actual entered namespace.
    /// Its report and parent state are independent from later command lookup.
    pub(crate) fn native_ensemble_map_prefix_input<'a>(
        &self,
        context: NsId,
        original: &'a [u8],
    ) -> Result<std::borrow::Cow<'a, [u8]>, NativeCommandLookupUnavailable> {
        let protocol = self
            .name_policy_protocol()
            .ok_or(NativeCommandLookupUnavailable::ProtocolUnavailable)?
            .recipe();
        let report =
            tcl_syntax::naming::root_unrooted_key_bytes(self.ns_name_bytes(context).as_bytes());
        let has_parent = self
            .name_world
            .borrow()
            .physical_namespace_parent(context)
            .is_some();
        protocol
            .ensemble_map_prefix_input(&report, has_parent, original)
            .map_err(|_| NativeCommandLookupUnavailable::NamespaceContextUnavailable)
    }
}
