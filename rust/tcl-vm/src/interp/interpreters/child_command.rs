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

//! The child-as-command form (`$child sub ?arg ...?`): dispatch of its subcommands for C and Jim children.

use super::super::{InterpId, Vm, err, native_name_consumers, ok};
use super::hidden_commands::CommandVisibilityError;
use crate::value::Value;
use tcl_core_types::NameBytes;
use tcl_runtime_api::Completion;

impl Vm {
    /// Dispatch a child-as-command call (`$child sub ?arg …?`): the `interp`
    /// ensemble restricted to interpreter `id`. `name` is the command word
    /// (the child's own command name) used in error messages.
    fn child_wrong_arguments(&mut self, original: &Value, synopsis: &[u8]) -> Completion<Value> {
        let prefix = tcl_cmd_core::ensemble::rewrite_argument_usage(
            std::slice::from_ref(original),
            &self.native_invocation.usage_rewrites,
        );
        let mut usage = match self.native_argument_usage_header(&prefix) {
            Ok(usage) => usage,
            Err(completion) => return completion,
        };
        usage.push(b' ');
        usage.extend_from_slice(synopsis);
        crate::command::native_wrong_args_bytes(self, &usage)
    }

    fn child_interpreter_not_found(&mut self, original: &Value) -> Completion<Value> {
        let scope = match native_name_consumers::OriginalNameConsumerScope::enter(self) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let reached = self.child_interpreter_not_found_in_original_name(original, &scope);
        self.original_name_finish(scope, reached)
            .unwrap_or_else(|error| crate::command::completion_from_cmd_error(self, error))
    }

    fn child_interpreter_not_found_in_original_name(
        &mut self,
        original: &Value,
        original_name_scope: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Completion<Value> {
        if let Err(error) = original_name_scope.check(self) {
            return crate::command::completion_from_cmd_error(self, error);
        }

        let bytes = match self.native_name_operand_bytes_checked(original) {
            Ok(bytes) => bytes,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let Some(policy) = self.name_policy_protocol() else {
            return self
                .refuse_host_command("native interpreter name reporting is unavailable".into());
        };
        let name = match tcl_syntax::naming::report_native_name_bytes(
            policy.recipe(),
            tcl_syntax::naming::NativeNameReportPurpose::InterpreterLookupError,
            &bytes,
        ) {
            Ok(name) => name,
            Err(error) => return self.refuse_host_command(format!("{error:?}")),
        };
        let mut message = b"could not find interpreter \"".to_vec();
        message.extend_from_slice(name);
        message.push(b'"');
        err(message)
    }

    pub(crate) fn dispatch_child(
        &mut self,
        name: &Value,
        id: InterpId,
        argv: &[Value],
    ) -> Completion<Value> {
        let scope = match native_name_consumers::OriginalNameConsumerScope::enter(self) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let reached = self.dispatch_child_in_original_name(name, id, argv, &scope);
        self.original_name_finish(scope, reached)
            .unwrap_or_else(|error| crate::command::completion_from_cmd_error(self, error))
    }

    pub(crate) fn dispatch_child_in_original_name(
        &mut self,
        name: &Value,
        id: InterpId,
        argv: &[Value],
        original_name_scope: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Completion<Value> {
        if let Err(error) = original_name_scope.check(self) {
            return crate::command::completion_from_cmd_error(self, error);
        }

        let Some((sub, rest)) = argv.split_first() else {
            return self.child_wrong_arguments(name, b"cmd ?arg ...?");
        };
        if self
            .actual_native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_some()
        {
            let original = match self.native_name_operand_bytes_checked(sub) {
                Ok(bytes) => bytes,
                Err(error) => return crate::command::completion_from_cmd_error(self, error),
            };
            let Some(policy) = self.name_policy_protocol() else {
                return self.refuse_host_command(
                    "native interpreter subcommand protocol unavailable".to_owned(),
                );
            };
            let word = policy.recipe().interpreter_subcommand_input(&original);
            let choices = tcl_dialect::InterpreterProtocol::JIM_HANDLE_SUBCOMMANDS;
            let sub = match crate::command::resolve_interp_option_bytes(
                choices,
                choices,
                word.selected(),
            ) {
                Ok(sub) => sub,
                Err(error) => return crate::command::completion_from_cmd_error(self, error),
            };
            return match sub {
                "eval" if !rest.is_empty() => self.eval_values_in_interp(id, rest),
                "delete" if rest.is_empty() => {
                    self.delete_interp(id);
                    ok(Value::empty())
                }
                "alias" if rest.len() >= 2 => {
                    let result = self.interp_alias_at(id, &rest[0], self.cur, rest[1..].to_vec());
                    if result.code.is_ok() {
                        ok(Value::empty())
                    } else {
                        result
                    }
                }
                _ => self.child_wrong_arguments(name, sub.as_bytes()),
            };
        }
        self.dispatch_c_child(name, id, sub, rest)
    }

    fn dispatch_c_child(
        &mut self,
        name: &Value,
        id: InterpId,
        sub: &Value,
        rest: &[Value],
    ) -> Completion<Value> {
        let scope = match native_name_consumers::OriginalNameConsumerScope::enter(self) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let reached = self.dispatch_c_child_in_original_name(name, id, sub, rest, &scope);
        self.original_name_finish(scope, reached)
            .unwrap_or_else(|error| crate::command::completion_from_cmd_error(self, error))
    }

    fn dispatch_child_visibility(
        &mut self,
        name: &Value,
        id: InterpId,
        rest: &[Value],
        hide: bool,
    ) -> Completion<Value> {
        // `$child hide   cmdName    ?hiddenCmdName?`
        // `$child expose hiddenName ?cmdName?`
        let (cmd, token) = match rest {
            [cmd] => (cmd, cmd),
            [cmd, token] => (cmd, token),
            _ => {
                let form = if hide {
                    "hide cmdName ?hiddenCmdName?"
                } else {
                    "expose hiddenCmdName ?cmdName?"
                };
                return self.child_wrong_arguments(name, form.as_bytes());
            }
        };
        if self.is_safe() {
            let verb = if hide { "hide" } else { "expose" };
            return err(format!(
                "permission denied: safe interpreter cannot {verb} commands"
            ));
        }
        let cmd = match self.native_name_operand_bytes_checked(cmd) {
            Ok(bytes) => bytes,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let token = match self.native_name_operand_bytes_checked(token) {
            Ok(bytes) => bytes,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        match self.child_hide_by_id(id, &cmd, &token, hide) {
            Ok(()) => ok(Value::empty()),
            Err(problem) => problem.into_completion(),
        }
    }

    fn child_hidden_names(&self, id: InterpId) -> Completion<Value> {
        let mut names: Vec<NameBytes> = self.st_of(id).map_or_default(|state| {
            state
                .hidden
                .tokens_by_name
                .iter()
                .filter(|(_, key)| state.hidden.commands.contains_key(*key))
                .map(|(name, _)| name.clone())
                .collect()
        });
        names.sort();
        ok(Value::list(
            names
                .into_iter()
                .map(|name| Value::from_native_string_bytes(name.as_bytes().to_vec()))
                .collect(),
        ))
    }

    fn dispatch_c_child_in_original_name(
        &mut self,
        name: &Value,
        id: InterpId,
        sub: &Value,
        rest: &[Value],
        original_name_scope: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Completion<Value> {
        if let Err(error) = original_name_scope.check(self) {
            return crate::command::completion_from_cmd_error(self, error);
        }

        let sub = match self.native_interpreter_option_from_original_checked(sub, true) {
            Ok(name) => name,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        match sub {
            "alias" => match rest {
                [alias] => self.alias_query_at(id, alias),
                [alias, target, arguments @ ..] => {
                    let bytes = match original_name_scope.read(self, target) {
                        Ok(bytes) => bytes,
                        Err(error) => {
                            return crate::command::completion_from_cmd_error(self, error);
                        }
                    };
                    let Some(recipe) = self
                        .name_policy_protocol()
                        .map(tcl_syntax::naming::NamePolicyProtocol::recipe)
                    else {
                        return self
                            .refuse_host_command("original child alias target issuer".into());
                    };
                    if recipe.child_alias_pretested_target_is_empty(&bytes) == Some(true) {
                        if arguments.is_empty() {
                            self.alias_delete_at(id, alias)
                        } else {
                            self.child_wrong_arguments(
                                name,
                                b"alias aliasName ?targetName? ?arg ...?",
                            )
                        }
                    } else {
                        let mut words = vec![target.clone()];
                        words.extend_from_slice(arguments);
                        self.interp_alias_at(id, alias, self.cur, words)
                    }
                }
                _ => self.child_wrong_arguments(name, b"alias aliasName ?targetName? ?arg ...?"),
            },
            "aliases" if rest.is_empty() => {
                match self.in_interp(id, Vm::original_alias_registration_names) {
                    Ok(names) => ok(Value::list(names)),
                    Err(error) => crate::command::completion_from_cmd_error(self, error),
                }
            }
            "aliases" => self.child_wrong_arguments(name, b"aliases"),

            "eval" => self.eval_values_in_interp(id, rest),
            "issafe" => ok(Value::bool(self.st_of(id).is_some_and(|st| st.is_safe))),
            "delete" => {
                self.delete_interp(id);
                ok(Value::empty())
            }
            "hidden" => self.child_hidden_names(id),
            "hide" | "expose" => self.dispatch_child_visibility(name, id, rest, sub == "hide"),
            "marktrusted" => {
                if self.is_safe() {
                    return err("permission denied: safe interpreter cannot mark trusted");
                }
                self.mark_interp_trusted(id);
                ok(Value::empty())
            }
            "invokehidden" => self.dispatch_child_invokehidden(name, id, rest),
            "recursionlimit" if rest.len() <= 1 => {
                self.dispatch_child_recursionlimit(name, id, rest)
            }
            "recursionlimit" => self.child_wrong_arguments(name, b"recursionlimit ?newlimit?"),
            // `$child debug ?-frame ?bool??` — the same switch `interp debug
            // path …` reaches, with the child spelling in the arity message.
            "debug" => {
                let dargs = rest.to_vec();
                let mut usage = match self.native_argument_usage_header(std::slice::from_ref(name))
                {
                    Ok(usage) => usage,
                    Err(completion) => return completion,
                };
                usage.extend_from_slice(b" debug ?-frame ?bool??");
                match self.in_interp(id, |vm| vm.debug_apply(&dargs, &usage)) {
                    Ok(v) => ok(v),
                    Err(error) => crate::command::completion_from_cmd_error(self, error),
                }
            }
            "limit" => self.dispatch_child_limit(name, id, rest),
            _ => self.refuse_host_command("native child interpreter worker unavailable".to_owned()),
        }
    }

    /// [`Self::dispatch_child`]'s `invokehidden` arm.
    fn dispatch_child_invokehidden(
        &mut self,
        name: &Value,
        id: InterpId,
        rest: &[Value],
    ) -> Completion<Value> {
        if self.is_safe() {
            return err("not allowed to invoke hidden commands from safe interpreter");
        }
        let selected = match crate::command::hidden_invocation_options(self, rest) {
            Ok(selected) => selected,
            Err(completion) => return completion,
        };
        let i = selected.command_index;
        match rest.get(i) {
            Some(cmd) => self
                .invoke_hidden_value_by_id_in_namespace(
                    id,
                    cmd,
                    &rest[i + 1..],
                    selected.namespace.as_deref(),
                )
                .unwrap_or_else(|| self.child_interpreter_not_found(name)),
            None => self.child_wrong_arguments(
                name,
                b"invokehidden ?-namespace ns? ?-global? ?--? cmd ?arg ..?",
            ),
        }
    }

    /// [`Self::dispatch_child`]'s `recursionlimit` arm.
    fn dispatch_child_recursionlimit(
        &mut self,
        name: &Value,
        id: InterpId,
        rest: &[Value],
    ) -> Completion<Value> {
        let nl = rest.first();
        if !self.interp_alive(id) {
            return self.child_interpreter_not_found(name);
        }
        // C answers a new limit with the word it was given.
        match self.in_interp(id, |vm| vm.recursion_limit_apply(nl)) {
            Ok(n) => ok(nl.cloned().unwrap_or_else(|| Value::int(n))),
            Err(e) => crate::command::completion_from_tcl_error(self, e),
        }
    }

    /// [`Self::dispatch_child`]'s `limit` arm.
    fn dispatch_child_limit(
        &mut self,
        name: &Value,
        id: InterpId,
        rest: &[Value],
    ) -> Completion<Value> {
        let (ltype, opts) = match rest {
            [ltype, opts @ ..] => (ltype.to_str().to_string(), opts.to_vec()),
            _ => {
                return self.child_wrong_arguments(name, b"limit limitType ?-option value ...?");
            }
        };
        if !self.interp_alive(id) {
            return self.child_interpreter_not_found(name);
        }
        match self.in_interp(id, |vm| vm.limit_apply(&ltype, &opts)) {
            Ok(v) => ok(v),
            Err(error) => crate::command::completion_from_cmd_error(self, error),
        }
    }

    /// Hide/expose on the actual reached child command's interpreter identity.
    fn child_hide_by_id(
        &mut self,
        id: InterpId,
        cmd: &[u8],
        token: &[u8],
        hide: bool,
    ) -> Result<(), CommandVisibilityError> {
        if !self.interp_alive(id) {
            return Err(CommandVisibilityError::InterpreterGone);
        }
        // This is the `$child hide` spelling of the same public operation.
        self.in_interp(id, |vm| {
            if hide {
                vm.hide_command_bytes(cmd, token)
            } else {
                vm.expose_own_command_bytes(cmd, token)
            }
        })
    }
}
