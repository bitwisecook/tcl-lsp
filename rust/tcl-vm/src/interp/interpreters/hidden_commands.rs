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

//! Hidden commands: `interp hide` and `expose`, the hidden-name table, their visibility errors, and `interp invokehidden`.

use super::super::commands::CommandSidecarKey;
use super::super::namespaces::NativeNamespaceAddress;
use super::super::{
    CommandSlot, InterpId, Vm, err, native_name_consumers, native_operation_currency,
};
use crate::command::{Command, err_with_code};
use crate::value::Value;
use tcl_core_types::NameBytes;
use tcl_runtime_api::{Completion, ROOT_NS};
use tcl_syntax::naming::{NamePolicyProtocol, NativeCommandVisibilityFailure};

/// A typed failure while moving a command between an interpreter's visible
/// and hidden tables.  The command adapter turns these identities into Tcl's
/// structured error completions; keeping them typed here prevents an ordinary
/// user error with similar wording from being misclassified later.
#[derive(Debug)]
pub(crate) enum CommandVisibilityError {
    InvalidHiddenToken(NamePolicyProtocol),
    HideNonGlobal(NamePolicyProtocol),
    ExposeNonGlobal(NamePolicyProtocol),
    MissingCommand(NameBytes, NamePolicyProtocol),
    MissingHiddenToken(NameBytes, NamePolicyProtocol),
    AlreadyHidden(NameBytes, NamePolicyProtocol),
    CommandExists(NameBytes, NamePolicyProtocol),
    InterpreterGone,
    NativeUnavailable(Completion<Value>),
}

pub(in super::super) fn native_visibility_completion(
    policy: NamePolicyProtocol,
    failure: NativeCommandVisibilityFailure,
    name: &[u8],
) -> Completion<Value> {
    let diagnostic = policy
        .recipe()
        .command_visibility_diagnostic(failure, name)
        .expect("selected C hidden-table operation has a visibility recipe");
    match diagnostic.error_code {
        Some(code) => err_with_code(diagnostic.message, code),
        None => err(diagnostic.message),
    }
}

impl CommandVisibilityError {
    pub(crate) fn into_completion(self) -> Completion<Value> {
        let (failure, name, policy) = match self {
            Self::InvalidHiddenToken(policy) => (
                NativeCommandVisibilityFailure::InvalidHiddenToken,
                None,
                policy,
            ),
            Self::HideNonGlobal(policy) => {
                (NativeCommandVisibilityFailure::HideNonGlobal, None, policy)
            }
            Self::ExposeNonGlobal(policy) => (
                NativeCommandVisibilityFailure::ExposeNonGlobal,
                None,
                policy,
            ),
            Self::MissingCommand(name, policy) => (
                NativeCommandVisibilityFailure::MissingCommand,
                Some(name),
                policy,
            ),
            Self::MissingHiddenToken(name, policy) => (
                NativeCommandVisibilityFailure::MissingHiddenToken,
                Some(name),
                policy,
            ),
            Self::AlreadyHidden(name, policy) => (
                NativeCommandVisibilityFailure::AlreadyHidden,
                Some(name),
                policy,
            ),
            Self::CommandExists(name, policy) => (
                NativeCommandVisibilityFailure::CommandExists,
                Some(name),
                policy,
            ),
            Self::NativeUnavailable(completion) => return completion,
            Self::InterpreterGone => return err("interpreter no longer exists"),
        };
        native_visibility_completion(
            policy,
            failure,
            name.as_ref().map_or(&[], NameBytes::as_bytes),
        )
    }
}

impl Vm {
    /// `interp invokehidden path cmd ?arg ...?` — invoke a hidden command inside
    /// the child. The command is temporarily restored to the child's visible
    /// table for the call (then the previous binding is put back), so it runs in
    /// the child without permanently un-hiding it. `None` if the child is gone.
    #[cfg(test)]
    pub(crate) fn invoke_hidden_in_child(
        &mut self,
        name: &str,
        cmd: &str,
        args: &[Value],
    ) -> Option<Completion<Value>> {
        if name.is_empty() {
            return Some(self.invoke_own_hidden(cmd, args));
        }
        let id = self.child_id(name).filter(|&id| self.interp_alive(id))?;
        Some(self.in_interp(id, |vm| vm.invoke_own_hidden(cmd, args)))
    }

    /// Invoke one of this interpreter's hidden commands without making its
    /// temporary visibility observable after dispatch.  The command's stable
    /// identity and import provenance must accompany the transient visible
    /// entry: command-surface checks and `namespace origin` consult those
    /// tables while a command is running.
    #[cfg(test)]
    fn invoke_own_hidden(&mut self, cmd: &str, args: &[Value]) -> Completion<Value> {
        self.invoke_own_hidden_value(&Value::string(cmd), args)
    }

    pub(in super::super) fn invoke_own_hidden_value(
        &mut self,
        original: &Value,
        args: &[Value],
    ) -> Completion<Value> {
        let scope = match native_name_consumers::OriginalNameConsumerScope::enter(self) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let reached = self.invoke_own_hidden_value_in_original_name(original, args, &scope);
        self.original_name_finish(scope, reached)
            .unwrap_or_else(|error| crate::command::completion_from_cmd_error(self, error))
    }

    fn invoke_own_hidden_value_in_original_name(
        &mut self,
        original: &Value,
        args: &[Value],
        original_name_scope: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Completion<Value> {
        if let Err(error) = original_name_scope.check(self) {
            return crate::command::completion_from_cmd_error(self, error);
        }

        let bytes = match self.native_name_operand_bytes_checked(original) {
            Ok(bytes) => bytes,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let selected = match self.select_hidden_token_bytes(&bytes) {
            Ok(name) => name,
            Err(error) => return error.into_completion(),
        };
        let key = self.hidden.tokens_by_name.get(&selected).cloned();
        let Some((key, hidden)) = key.and_then(|key| {
            self.hidden
                .commands
                .get(&key)
                .cloned()
                .map(|command| (key, command))
        }) else {
            let policy = match self.command_visibility_policy() {
                Ok(policy) => policy,
                Err(error) => return error.into_completion(),
            };
            return native_visibility_completion(
                policy,
                NativeCommandVisibilityFailure::InvokeHiddenMissing,
                selected.as_bytes(),
            );
        };
        let identity = self.hidden.builtin_identities.get(&key).map(String::as_str);
        if !self.builtin_command_visible_for_identity(&key, &hidden, identity) {
            let mut message = b"invalid command name \"".to_vec();
            message.extend_from_slice(selected.as_bytes());
            message.push(b'"');
            return err(message);
        }
        self.invoke_resolved_command_value_with_usage(
            original,
            CommandSidecarKey::hidden(key),
            hidden,
            args,
            &[],
        )
    }

    pub(crate) fn take_invoked_sidecar(&mut self) -> Option<CommandSidecarKey> {
        self.native_invocation.sidecar.take()
    }

    pub(in super::super) fn command_visibility_policy(
        &mut self,
    ) -> Result<NamePolicyProtocol, CommandVisibilityError> {
        self.name_policy_protocol().ok_or_else(|| {
            CommandVisibilityError::NativeUnavailable(
                self.refuse_host_command(
                    "native command-visibility protocol unavailable".to_owned(),
                ),
            )
        })
    }

    fn select_hidden_token_bytes(
        &mut self,
        original: &[u8],
    ) -> Result<NameBytes, CommandVisibilityError> {
        let Some(policy) = self.name_policy_protocol() else {
            return Err(CommandVisibilityError::NativeUnavailable(
                self.refuse_host_command("native hidden-token protocol unavailable".to_owned()),
            ));
        };
        match policy.recipe().hidden_token_input(original) {
            Ok(projected) => Ok(NameBytes::from(projected.selected())),
            Err(error) => Err(CommandVisibilityError::NativeUnavailable(
                self.refuse_host_command(error.to_string()),
            )),
        }
    }

    pub(super) fn reserve_hidden_storage_key(&mut self, name: NameBytes) -> String {
        if let Some(key) = self.hidden.tokens_by_name.get(&name) {
            return key.clone();
        }
        let key = if let Ok(text) = name.try_utf8() {
            text.to_owned()
        } else {
            let key = format!("\0hidden:{}", self.hidden.next_key);
            self.hidden.next_key = self
                .hidden
                .next_key
                .checked_add(1)
                .expect("hidden token counter exhausted");
            key
        };
        self.hidden.tokens_by_name.insert(name.clone(), key.clone());
        self.hidden.names_by_key.insert(key.clone(), name);
        key
    }

    /// Exact hidden-token enumeration; opaque sidecar storage keys are never reported.
    pub(crate) fn own_hidden_names_bytes(&self) -> Vec<NameBytes> {
        let mut names: Vec<_> = self
            .hidden
            .tokens_by_name
            .iter()
            .filter(|(_, key)| self.hidden.commands.contains_key(*key))
            .map(|(name, _)| name.clone())
            .collect();
        names.sort();
        names
    }

    /// `interp hide {} cmd` — move one visible command plus all of its
    /// metadata into the hidden table.  This is the only public hide seam;
    /// every caller therefore observes the same release-visibility check.
    #[cfg(test)]
    pub(crate) fn hide_command(
        &mut self,
        cmd: &str,
        token: &str,
    ) -> Result<(), CommandVisibilityError> {
        self.hide_command_bytes(cmd.as_bytes(), token.as_bytes())
    }

    pub(crate) fn hide_command_bytes(
        &mut self,
        cmd: &[u8],
        token: &[u8],
    ) -> Result<(), CommandVisibilityError> {
        let policy = self.command_visibility_policy()?;
        let cmd = self.select_hidden_token_bytes(cmd)?;
        let token_name = self.select_hidden_token_bytes(token)?;
        Self::validate_command_visibility_args_bytes(
            true,
            cmd.as_bytes(),
            token_name.as_bytes(),
            policy,
        )?;
        let token = self.reserve_hidden_storage_key(token_name.clone());
        if self.hidden.commands.contains_key(&token) {
            return Err(CommandVisibilityError::AlreadyHidden(
                token_name.clone(),
                policy,
            ));
        }
        let source =
            match self.resolve_command_bytes_checked(self.current_ns_id(), cmd.as_bytes(), true) {
                Ok(Some(key)) => key,
                Ok(None) => {
                    return Err(CommandVisibilityError::MissingCommand(cmd.clone(), policy));
                }
                Err(error) => {
                    return Err(CommandVisibilityError::NativeUnavailable(
                        self.refuse_host_command(error.to_string()),
                    ));
                }
            };
        if !self
            .visible_command_at_key(&source)
            .is_some_and(|command| self.builtin_command_visible_for_surface(&source, &command))
        {
            return Err(CommandVisibilityError::MissingCommand(cmd.clone(), policy));
        }
        let source_generation = self.live_command_generation(&source);
        let import_origin = self.imported_commands.get(&source).cloned();
        let builtin_identity = self.builtin_identity_for_key(&source);
        let Some(command) = self.take_command_unchecked_key(&source) else {
            return Err(CommandVisibilityError::MissingCommand(cmd.clone(), policy));
        };
        let cross_target = match &command {
            Command::CrossAlias { target, .. } => Some(*target),
            _ => None,
        };
        let object = match &command {
            Command::Object(object) => Some(*object),
            _ => None,
        };
        if import_origin.is_none()
            && let Command::Ensemble(ensemble) = &command
        {
            let name = tcl_syntax::naming::root_unrooted_key_bytes(token_name.as_bytes());
            ensemble.rename(NameBytes::from(name));
        }
        self.hidden.commands.insert(token.clone(), command);
        if let Some(object) = object {
            crate::cmd_oo::oo_command_hidden(self, object, token.clone());
        }
        if let Some(generation) = source_generation {
            self.hidden.generations.insert(token.clone(), generation);
        }
        crate::cmd_coro::on_command_hidden(self, &source, &token);
        self.move_command_traces(
            &CommandSidecarKey::visible(&source),
            CommandSidecarKey::hidden(&token),
        );
        if let Some(origin) = import_origin {
            self.hidden.imports.insert(token.clone(), origin);
        }
        if let Some(identity) = builtin_identity {
            self.hidden
                .builtin_identities
                .insert(token.clone(), identity);
        }
        // Hiding does not change the Tcl-visible origin. Retarget the internal
        // reference into the hidden domain so the lineage remains traversable
        // without colliding with an equal visible token.
        self.retarget_imports_key(
            &CommandSidecarKey::visible(&source),
            &CommandSidecarKey::hidden(&token),
        );
        if let Some(target) = cross_target {
            self.retarget_alias_backref_key(
                &CommandSidecarKey::visible(&source),
                CommandSidecarKey::hidden(&token),
                target,
            );
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn expose_own_command(
        &mut self,
        command: &str,
        token: &str,
    ) -> Result<(), CommandVisibilityError> {
        self.expose_own_command_bytes(command.as_bytes(), token.as_bytes())
    }

    pub(crate) fn expose_own_command_bytes(
        &mut self,
        cmd: &[u8],
        token: &[u8],
    ) -> Result<(), CommandVisibilityError> {
        let policy = self.command_visibility_policy()?;
        let source_name = self.select_hidden_token_bytes(cmd)?;
        let token = self.select_hidden_token_bytes(token)?;
        Self::validate_command_visibility_args_bytes(
            false,
            source_name.as_bytes(),
            token.as_bytes(),
            policy,
        )?;
        let Some(cmd) = self.hidden.tokens_by_name.get(&source_name).cloned() else {
            return Err(CommandVisibilityError::MissingHiddenToken(
                source_name,
                policy,
            ));
        };
        // `interp expose` always installs the destination in the global
        // namespace.  A contextual lookup here would incorrectly see (and
        // reject because of) a same-spelled local command in the caller's
        // namespace, instead of checking the exact global table owner.
        let selected = policy
            .recipe()
            .exposed_command_publication_slot(token.as_bytes())
            .map_err(|error| {
                CommandVisibilityError::NativeUnavailable(
                    self.refuse_host_command(error.to_string()),
                )
            })?;
        let destination_slot = CommandSlot {
            namespace: ROOT_NS,
            simple: selected.simple,
        };
        if self
            .command_storage_key_at_slot(&destination_slot)
            .is_some_and(|key| self.visible_command_exists_exact(&key))
        {
            return Err(CommandVisibilityError::CommandExists(token.clone(), policy));
        }
        self.note_native_command_token_changed(&CommandSidecarKey::hidden(&cmd));
        let Some(c) = self.hidden.commands.remove(&cmd) else {
            return Err(CommandVisibilityError::MissingHiddenToken(
                source_name.clone(),
                policy,
            ));
        };
        let source_generation = self.hidden.generations.remove(&cmd);
        let imported = self.hidden.imports.contains_key(&cmd);
        let cross_target = match &c {
            Command::CrossAlias { target, .. } => Some(*target),
            _ => None,
        };
        let object = match &c {
            Command::Object(object) => Some(*object),
            _ => None,
        };
        let destination = self.register_command_in_slot(destination_slot, c);
        if let Some(object) = object {
            crate::cmd_oo::oo_command_exposed(self, object, destination.clone());
        }
        if let Some(generation) = source_generation {
            self.name_world
                .borrow_mut()
                .command_identity
                .generations
                .insert(destination.clone(), generation);
        }
        if !imported && let Some(Command::Ensemble(token)) = self.commands.get(&destination) {
            let name = tcl_syntax::naming::root_unrooted_key_bytes(
                self.command_display_key_bytes(&destination).as_bytes(),
            );
            token.rename(NameBytes::from(name));
        }
        crate::cmd_coro::on_command_exposed(self, &cmd, &destination);
        self.move_command_traces(
            &CommandSidecarKey::hidden(&cmd),
            CommandSidecarKey::visible(&destination),
        );
        if let Some(origin) = self.hidden.imports.remove(&cmd) {
            self.imported_commands.insert(destination.clone(), origin);
        }
        if let Some(identity) = self.hidden.builtin_identities.remove(&cmd) {
            self.builtin_identities
                .insert(destination.clone(), identity);
        }
        // Reconnect internal references to the visible domain; their
        // Tcl-visible ultimate source lineage remains unchanged.
        self.retarget_imports_key(
            &CommandSidecarKey::hidden(&cmd),
            &CommandSidecarKey::visible(&destination),
        );
        if let Some(target) = cross_target {
            self.retarget_alias_backref_key(
                &CommandSidecarKey::hidden(&cmd),
                CommandSidecarKey::visible(&destination),
                target,
            );
        }
        self.hidden.tokens_by_name.remove(&source_name);
        Ok(())
    }

    /// Validate the public `interp hide`/`interp expose` name roles before
    /// consulting either command table. All three entry forms route through
    /// this owner, including the `$child hide|expose name` shorthand.
    fn validate_command_visibility_args_bytes(
        hide: bool,
        cmd: &[u8],
        token: &[u8],
        policy: NamePolicyProtocol,
    ) -> Result<(), CommandVisibilityError> {
        if hide {
            // The hide destination is the hidden token. Check it first: in
            // the one-argument child shorthand the source and token spellings
            // are equal, and Tcl identifies the qualified token role.
            if tcl_syntax::naming::is_qualified(token) {
                return Err(CommandVisibilityError::InvalidHiddenToken(policy));
            }
            if !tcl_cmd_core::namespace::qualifiers(cmd).is_empty() {
                return Err(CommandVisibilityError::HideNonGlobal(policy));
            }
        } else {
            // Expose checks its visible destination first. Thus a qualified
            // one-argument shorthand is NON_GLOBAL even though that same word
            // also names the hidden source token.
            if tcl_syntax::naming::is_qualified(token) {
                return Err(CommandVisibilityError::ExposeNonGlobal(policy));
            }
        }
        Ok(())
    }

    /// Sorted hidden-command names of *this* interp (`interp hidden {}`).
    pub(crate) fn own_hidden_names(&self) -> Vec<NameBytes> {
        self.own_hidden_names_bytes()
    }

    /// [`Self::invoke_hidden_in_child`] addressing the child by id.
    #[cfg(test)]
    pub(in super::super) fn invoke_hidden_by_id(
        &mut self,
        id: InterpId,
        cmd: &str,
        args: &[Value],
    ) -> Option<Completion<Value>> {
        self.invoke_hidden_value_by_id(id, &Value::string(cmd), args)
    }

    #[cfg(test)]
    pub(crate) fn invoke_hidden_value_by_id(
        &mut self,
        id: InterpId,
        cmd: &Value,
        args: &[Value],
    ) -> Option<Completion<Value>> {
        self.invoke_hidden_value_by_id_in_namespace(id, cmd, args, None)
    }

    pub(crate) fn invoke_hidden_value_by_id_in_namespace(
        &mut self,
        id: InterpId,
        cmd: &Value,
        args: &[Value],
        namespace: Option<&[u8]>,
    ) -> Option<Completion<Value>> {
        if !self.interp_alive(id) {
            return None;
        }
        Some(self.in_interp(id, |vm| match namespace {
            Some(namespace) => vm.invoke_own_hidden_in_namespace(cmd, args, namespace),
            None => vm.invoke_own_hidden_value(cmd, args),
        }))
    }

    fn invoke_own_hidden_in_namespace(
        &mut self,
        cmd: &Value,
        args: &[Value],
        original_namespace: &[u8],
    ) -> Completion<Value> {
        let operation = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(operation) => operation,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let target = native_operation_currency::SavedExecutionTarget::capture(self);
        let Some(grammar) = self
            .native_invocation_dialect()
            .native_name_protocol()
            .and_then(|protocol| protocol.hidden_invocation_grammar().ok())
        else {
            return self
                .refuse_host_command("original hidden-invocation frame policy unavailable".into());
        };
        let address = match self.namespace_address_from_native_bytes(ROOT_NS, original_namespace) {
            Ok(NativeNamespaceAddress::C { path, .. }) => path,
            Ok(NativeNamespaceAddress::Jim { .. }) | Err(_) => {
                return self.refuse_host_command(
                    "original hidden-invocation namespace address unavailable".into(),
                );
            }
        };
        if let Err(error) = operation.check(self) {
            return crate::command::completion_from_cmd_error(self, error);
        }
        if grammar.uses_global_variable_frame() && !address.is_empty() {
            return self.refuse_host_command(
                "original hidden-invocation global namespace purpose unavailable".into(),
            );
        }
        self.declare_namespace_path_with_origin(address.clone(), true);
        let namespace = self.definition_namespace_token_at_path(&address, true);
        if !self.native_namespace_token_has_owner(namespace) {
            return self.refuse_host_command(
                "original hidden-invocation namespace token unavailable".into(),
            );
        }
        let frame_depth = self.frames.len();
        let global_variable_frame = grammar.uses_global_variable_frame();
        let restored = if global_variable_frame {
            if namespace != ROOT_NS {
                return self.refuse_host_command(
                    "original hidden-invocation global frame purpose unavailable".into(),
                );
            }
            match self.select_execution_frame_checked(0) {
                Ok(restored) => restored,
                Err(error) => return crate::command::completion_from_cmd_error(self, error),
            }
        } else {
            self.push_ns_eval_token_frame(namespace, Vec::new());
            None
        };
        let reached = self.invoke_own_hidden_value(cmd, args);
        match target.in_original(self, |vm| {
            if global_variable_frame {
                vm.restore_execution_frame(restored);
            } else {
                vm.drain_call_frames_to(frame_depth);
            }
            reached
        }) {
            Ok(completion) => completion,
            Err(first) => crate::command::completion_from_cmd_error(
                self,
                tcl_cmd_core::CmdError::from_execution_refusal(first),
            ),
        }
    }
}
