// SPDX-License-Identifier: AGPL-3.0-or-later
//! Alias-record lookup retains genuine VM command generations and name objects.

use super::{
    Command, CommandSidecarKey, InterpId, Vm, native_name_consumers::OriginalNameConsumerScope, ok,
};
use crate::{Completion, Value, command::completion_from_cmd_error};
use tcl_cmd_core::CmdError;
use tcl_core_types::NameBytes;
use tcl_syntax::{naming::NativeNameContext, value::ValueError};

impl Vm {
    fn alias_registration_binding(
        &mut self,
        name: &Value,
    ) -> Result<Option<(super::CommandTokenIdentity, Command)>, CmdError> {
        let scope = OriginalNameConsumerScope::enter(self)?;
        let reached = (|| {
            let original = scope.read(self, name)?;
            let recipe = self
                .name_policy_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original alias query issuer",
                ))?
                .recipe();
            let selected = recipe
                .alias_registration_query_input(&original)
                .map_err(|_| {
                    ValueError::CommandProtocolUnavailable("original alias query coordinate")
                })?;
            let token = self
                .alias_registrations
                .query(&selected)
                .map_err(|_| {
                    ValueError::CommandProtocolUnavailable("original alias query purpose")
                })?
                .cloned();
            scope.check(self)?;
            let Some(token) = token else { return Ok(None) };
            let live = self.command_token_at_generation(token.generation).ok_or(
                ValueError::CommandProtocolUnavailable("original alias query live token"),
            )?;
            let command = self.command_at_sidecar_key(&live.key).ok_or(
                ValueError::CommandProtocolUnavailable("original alias query implementation"),
            )?;
            if !matches!(command, Command::Alias { .. } | Command::CrossAlias { .. }) {
                return Err(ValueError::CommandProtocolUnavailable(
                    "original alias query implementation kind",
                )
                .into());
            }
            Ok(Some((live, command)))
        })();
        self.original_name_finish(scope, reached)?
    }

    fn store_alias_registration_command(
        &mut self,
        key: CommandSidecarKey,
        command: Command,
    ) -> Result<(), CmdError> {
        match key {
            CommandSidecarKey::Hidden(key) => {
                self.hidden.commands.insert(key, command);
            }
            CommandSidecarKey::Visible(key) => {
                if let Some(root) = self.retained_root_for_visible_key(&key) {
                    let mut world = self.name_world.borrow_mut();
                    let record = world.ns_deferral.retained.get_mut(&root).ok_or(
                        ValueError::CommandProtocolUnavailable("original alias retained owner"),
                    )?;
                    record.commands.insert(key, command);
                } else {
                    self.commands.insert(key, command);
                }
            }
        }
        Ok(())
    }

    pub(super) fn complete_modern_alias_registration(
        &mut self,
        original: &Value,
        generation: u64,
    ) -> Result<Value, CmdError> {
        let scope = OriginalNameConsumerScope::enter(self)?;
        let reached = (|| {
            let recipe = self
                .name_policy_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original alias registration issuer",
                ))?
                .recipe();
            let mut current = original.clone();
            loop {
                let bytes = scope.read(self, &current)?;
                let selected = recipe
                    .child_alias_publication_input(NativeNameContext::root(), &bytes)
                    .map_err(|_| {
                        ValueError::CommandProtocolUnavailable(
                            "original alias registration coordinate",
                        )
                    })?;
                let occupied = self
                    .alias_registrations
                    .query(&selected)
                    .map_err(|_| {
                        ValueError::CommandProtocolUnavailable(
                            "original alias registration purpose",
                        )
                    })?
                    .is_some();
                let live = self.command_token_at_generation(generation).ok_or(
                    ValueError::CommandProtocolUnavailable("original alias registration token"),
                )?;
                let mut command = self.command_at_sidecar_key(&live.key).ok_or(
                    ValueError::CommandProtocolUnavailable(
                        "original alias registration implementation",
                    ),
                )?;
                let (Command::Alias {
                    registration_name: registration,
                    publication_name: report,
                    ..
                }
                | Command::CrossAlias {
                    registration_name: registration,
                    publication_name: report,
                    ..
                }) = &mut command
                else {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "original alias registration kind",
                    )
                    .into());
                };
                if !registration
                    .as_ref()
                    .is_some_and(|original| original.is_same_object(&current))
                {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "original alias registration header owner",
                    )
                    .into());
                }
                if !occupied {
                    self.alias_registrations
                        .install_unique(&selected, live)
                        .map_err(|_| {
                            ValueError::CommandProtocolUnavailable(
                                "original alias registration installation",
                            )
                        })?;
                    scope.check(self)?;
                    return Ok(current);
                }
                let token = recipe
                    .alias_registration_collision_token(&bytes)
                    .map_err(|_| {
                        ValueError::CommandProtocolUnavailable("original alias collision policy")
                    })?;
                let replacement = Value::new_native_string_bytes(token.as_slice());
                *registration = Some(replacement.clone());
                *report = NameBytes::from(token.as_slice());
                self.store_alias_registration_command(live.key, command)?;
                drop(selected);
                drop(bytes);
                drop(current);
                scope.check(self)?;
                current = replacement;
            }
        })();
        self.original_name_finish(scope, reached)?
    }

    pub(super) fn original_alias_registration_names(&mut self) -> Result<Vec<Value>, CmdError> {
        let scope = OriginalNameConsumerScope::enter(self)?;
        let reached = (|| {
            let recipe = self
                .name_policy_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original alias inventory issuer",
                ))?
                .recipe();
            let abi = tcl_runtime_api::native_hash_abi::supported_backend_hash_abi(None).ok_or(
                ValueError::CommandProtocolUnavailable("original alias inventory ABI"),
            )?;
            self.alias_registrations.select_order(recipe, abi);
            let tokens = self
                .alias_registrations
                .ordered_tokens()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original alias inventory order",
                ))?
                .into_iter()
                .cloned()
                .collect::<Vec<_>>();
            tokens
                .into_iter()
                .map(|token| {
                    let live = self.command_token_at_generation(token.generation).ok_or(
                        ValueError::CommandProtocolUnavailable("original alias inventory token"),
                    )?;
                    let Some(
                        Command::Alias {
                            registration_name: Some(registration),
                            ..
                        }
                        | Command::CrossAlias {
                            registration_name: Some(registration),
                            ..
                        },
                    ) = self.command_at_sidecar_key(&live.key)
                    else {
                        return Err(ValueError::CommandProtocolUnavailable(
                            "original alias inventory implementation",
                        )
                        .into());
                    };
                    scope.current_for_value(self, &registration)?;
                    Ok(registration)
                })
                .collect()
        })();
        self.original_name_finish(scope, reached)?
    }

    pub(crate) fn interpreter_alias_query_original(
        &mut self,
        path: &Value,
        name: &Value,
    ) -> Completion<Value> {
        let source = match self.resolve_interp_path_original(path) {
            Ok(source) => source,
            Err(error) => return error,
        };
        self.alias_query_at(source, name)
    }

    pub(super) fn alias_query_at(&mut self, source: InterpId, name: &Value) -> Completion<Value> {
        match self.in_interp(source, |vm| vm.alias_registration_binding(name)) {
            Ok(Some((_, Command::Alias { words, .. } | Command::CrossAlias { words, .. }))) => {
                ok(Value::list(words.as_ref().clone()))
            }
            Ok(None) => ok(Value::new_native_string_bytes(b"".as_slice())),
            Ok(Some(_)) => self.refuse_host_command("original alias query kind".into()),
            Err(error) => completion_from_cmd_error(self, error),
        }
    }

    pub(crate) fn interpreter_alias_delete_original(
        &mut self,
        path: &Value,
        name: &Value,
        empty: &Value,
    ) -> Completion<Value> {
        let bytes = match self.native_name_operand_bytes_checked(empty) {
            Ok(bytes) => bytes,
            Err(error) => return completion_from_cmd_error(self, error),
        };
        if !bytes.is_empty() {
            return super::err(
                "wrong # args: should be \"interp alias path alias ?targetPath targetCmd ?arg ...?\"",
            );
        }
        let source = match self.resolve_interp_path_original(path) {
            Ok(source) => source,
            Err(error) => return error,
        };
        self.alias_delete_at(source, name)
    }

    pub(super) fn alias_delete_at(&mut self, source: InterpId, name: &Value) -> Completion<Value> {
        let reached = self.in_interp(source, |vm| {
            let scope = OriginalNameConsumerScope::enter(vm)?;
            let reached = (|| {
                let Some((token, command)) = vm.alias_registration_binding(name)? else {
                    return Ok(());
                };
                scope.current_for_value(vm, name)?;
                vm.retire_registered_real_command(&token, &command);
                scope.check(vm)
            })();
            vm.original_name_finish(scope, reached)?
        });
        match reached {
            Ok(()) => ok(Value::new_native_string_bytes(b"".as_slice())),
            Err(error) => completion_from_cmd_error(self, error),
        }
    }

    pub(crate) fn interpreter_alias_target_original(
        &mut self,
        path: &Value,
        name: &Value,
    ) -> Completion<Value> {
        let source = match self.resolve_interp_path_original(path) {
            Ok(source) => source,
            Err(error) => return error,
        };
        let target = match self.in_interp(source, |vm| vm.alias_registration_binding(name)) {
            Ok(Some((_, Command::Alias { .. }))) => source,
            Ok(Some((_, Command::CrossAlias { target, .. }))) => target,
            Ok(_) => {
                let bytes = match self.native_name_operand_bytes_checked(name) {
                    Ok(bytes) => bytes,
                    Err(error) => return completion_from_cmd_error(self, error),
                };
                return super::err([b"alias \"".as_slice(), &bytes, b"\" not found"].concat());
            }
            Err(error) => return completion_from_cmd_error(self, error),
        };
        match self.original_descendant_interpreter_path(self.cur, target) {
            Some(path) => ok(Value::list(path)),
            None => super::err("target interpreter is not a descendant of current interpreter"),
        }
    }

    fn original_descendant_interpreter_path(
        &self,
        source: InterpId,
        target: InterpId,
    ) -> Option<Vec<Value>> {
        if source == target {
            return Some(Vec::new());
        }
        for (name, child) in &self.st_of(source)?.children {
            if !self.interp_alive(*child) {
                continue;
            }
            if let Some(mut path) = self.original_descendant_interpreter_path(*child, target) {
                path.insert(0, Value::new_native_string_bytes(name.as_bytes()));
                return Some(path);
            }
        }
        None
    }
}
