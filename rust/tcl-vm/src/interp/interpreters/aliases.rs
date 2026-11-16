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

//! `interp alias`: publishing an alias, target-death backrefs, the alias-loop check, and invoking an alias's words with its usage rewrite.

use super::super::commands::CommandSidecarKey;
use super::super::{
    AliasBackref, CommandRenameTransaction, CommandTokenIdentity, InterpId, InterpState, Vm, err,
    native_name_consumers, ok,
};
use crate::command::{Command, settle_control_options};
use crate::value::Value;
use std::rc::Rc;
use tcl_core_types::NameBytes;
use tcl_runtime_api::completion_options::ControlOptionPolicy;
use tcl_runtime_api::{Code, Completion, ROOT_NS};

impl Vm {
    /// Record a just-registered cross-interp alias for the target-death sweep.
    pub(crate) fn note_alias_backref(&mut self, key: &str, target: InterpId) {
        self.note_alias_backref_key(CommandSidecarKey::visible(key), target);
    }

    fn note_alias_backref_key(&mut self, key: CommandSidecarKey, target: InterpId) {
        let source = self.cur;
        self.alias_backrefs
            .retain(|b| !(b.source == source && b.key.eq(&key)));
        self.alias_backrefs.push(AliasBackref {
            source,
            key,
            target,
        });
    }

    /// Move a cross-interp alias's target-death backref from `old_key` to
    /// `new_key` (an alias renamed in the current interp).
    pub(crate) fn retarget_alias_backref(
        &mut self,
        old_key: &str,
        new_key: &str,
        target: InterpId,
    ) {
        self.retarget_alias_backref_key(
            &CommandSidecarKey::visible(old_key),
            CommandSidecarKey::visible(new_key),
            target,
        );
    }

    pub(super) fn retarget_alias_backref_key(
        &mut self,
        old_key: &CommandSidecarKey,
        new_key: CommandSidecarKey,
        target: InterpId,
    ) {
        let source = self.cur;
        self.alias_backrefs
            .retain(|b| !(b.source == source && (b.key.eq(old_key) || b.key == new_key)));
        self.alias_backrefs.push(AliasBackref {
            source,
            key: new_key,
            target,
        });
    }

    /// Drop any cross-interp alias backref for `key` in the current interp (the
    /// alias was deleted or overwritten with a non-alias).
    pub(crate) fn drop_alias_backref(&mut self, key: &str) {
        self.drop_alias_backref_key(&CommandSidecarKey::visible(key));
    }

    pub(in super::super) fn drop_alias_backref_key(&mut self, key: &CommandSidecarKey) {
        let source = self.cur;
        self.alias_backrefs
            .retain(|b| !(b.source == source && b.key.eq(key)));
    }

    /// `interp alias srcPath srcCmd targetPath target…` — route the alias by
    /// its two `interp` paths (`{}` = this interp, else a path reachable from
    /// it), install it in the SOURCE interp's table, and refuse a loop the way
    /// C's `TclPreventAliasLoop` does: create first, walk the chain, roll back
    /// on a hit (tclsh-pinned: a failed self-alias also destroys the proc it
    /// clobbered — C does not restore it).  Source and target may be any two
    /// interpreters in the tree (child→parent, parent→child, and sibling↔
    /// sibling all route through the shared engine — C supports every pairing).
    pub(crate) fn interp_alias_create(
        &mut self,
        src_path: &Value,
        src_cmd: &Value,
        target_path: &Value,
        target_words: Vec<Value>,
    ) -> Completion<Value> {
        let src = match self.resolve_interp_path_original(src_path) {
            Ok(s) => s,
            Err(c) => return c,
        };
        let target = match self.resolve_interp_path_original(target_path) {
            Ok(t) => t,
            Err(c) => return c,
        };
        self.interp_alias_at(src, src_cmd, target, target_words)
    }

    /// C interp aliases reports alias-record registration spelling, independently
    /// of current command placement and Jim's core-only info aliases inventory.
    pub(crate) fn interpreter_alias_names_original(
        &mut self,
        path: Option<&Value>,
    ) -> Completion<Value> {
        if self
            .native_invocation_dialect()
            .native_command_name_protocol()
            .is_none()
        {
            return self.refuse_host_command("C interpreter alias inventory issuer".into());
        }
        let id = match path {
            Some(path) => match self.resolve_interp_path_original(path) {
                Ok(id) => id,
                Err(error) => return error,
            },
            None => self.cur,
        };
        match self.in_interp(id, Vm::original_alias_registration_names) {
            Ok(names) => ok(Value::list(names)),
            Err(error) => crate::command::completion_from_cmd_error(self, error),
        }
    }

    pub(in super::super) fn interp_alias_at(
        &mut self,
        src: InterpId,
        src_cmd: &Value,
        target: InterpId,
        target_words: Vec<Value>,
    ) -> Completion<Value> {
        let scope = match native_name_consumers::OriginalNameConsumerScope::enter(self) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let reached =
            self.interp_alias_at_in_original_name(src, src_cmd, target, target_words, &scope);
        self.original_name_finish(scope, reached)
            .unwrap_or_else(|error| crate::command::completion_from_cmd_error(self, error))
    }

    fn original_alias_command(
        src: InterpId,
        target: InterpId,
        src_cmd: &Value,
        words: &Rc<Vec<Value>>,
        original_jim_prefix: Option<Rc<Value>>,
        publication_name: NameBytes,
        recipe: tcl_syntax::naming::NativeNameProtocol,
    ) -> Command {
        if src == target {
            Command::Alias {
                words: Rc::clone(words),
                publication_name,
                registration_name: recipe
                    .alias_record_retains_original_registration_name()
                    .then(|| src_cmd.clone()),
            }
        } else {
            Command::CrossAlias {
                target,
                words: Rc::clone(words),
                original_jim_prefix,
                publication_name,
                registration_name: recipe
                    .alias_record_retains_original_registration_name()
                    .then(|| src_cmd.clone()),
            }
        }
    }

    fn publish_original_alias(
        &mut self,
        src: InterpId,
        target: InterpId,
        src_cmd: &Value,
        original: &[u8],
        words: &Rc<Vec<Value>>,
        original_jim_prefix: Option<Rc<Value>>,
    ) -> Result<(String, Option<CommandTokenIdentity>), tcl_cmd_core::CmdError> {
        self.in_interp(src, |vm| {
            let source = native_name_consumers::OriginalNameConsumerScope::enter_for_namespace(
                vm,
                Some(ROOT_NS),
            )?;
            let reached = (|| {
                let recipe = vm
                    .name_policy_protocol()
                    .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original alias publication policy",
                    ))?
                    .recipe();
                let selected = recipe
                    .child_alias_publication_input(
                        tcl_syntax::naming::NativeNameContext::root(),
                        original,
                    )
                    .map_err(|_| {
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original alias publication namespace",
                        )
                    })?;
                let publication_name =
                    NameBytes::from(selected.alias_registration_report_bytes().ok_or(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original alias publication policy",
                        ),
                    )?);
                let cmd = Self::original_alias_command(
                    src,
                    target,
                    src_cmd,
                    words,
                    original_jim_prefix,
                    publication_name,
                    recipe,
                );
                let report = vm
                    .jim_command_table_key_for_original(
                        original,
                        tcl_syntax::naming::NativeNamePurpose::ChildAliasPublication,
                    )
                    .map_err(|_| {
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original alias publication report",
                        )
                    })?;
                let slot = vm
                    .native_child_alias_publication_slot(original)
                    .map_err(|_| {
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original alias publication slot",
                        )
                    })?;
                source.check(vm)?;
                let key = vm.register_command_in_slot_with_jim_key(slot, cmd, report);
                source.check(vm)?;
                if src != target {
                    vm.note_alias_backref(&key, target);
                }
                let token = if selected
                    .alias_registration_key()
                    .map_err(|_| {
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original alias registration purpose",
                        )
                    })?
                    .is_some()
                {
                    Some(
                        vm.command_token_identity(&CommandSidecarKey::visible(&key))
                            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                "original alias publication token",
                            ))?,
                    )
                } else {
                    None
                };
                Ok::<_, tcl_cmd_core::CmdError>((key, token))
            })();
            vm.original_name_finish(source, reached)?
        })
    }

    fn replace_c84_alias_registration(
        &mut self,
        src_cmd: &Value,
        published_token: &CommandTokenIdentity,
        scope: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        loop {
            scope.current_for_value(self, src_cmd)?;
            let registration_name = self.native_name_operand_bytes_checked(src_cmd)?;
            scope.check(self)?;
            let selected = self
                .name_policy_protocol()
                .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original alias registration retry issuer",
                ))?
                .recipe()
                .child_alias_publication_input(
                    tcl_syntax::naming::NativeNameContext::root(),
                    &registration_name,
                )
                .map_err(|_| {
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original alias registration retry coordinate",
                    )
                })?;
            let prior = self
                .alias_registrations
                .prior(&selected)
                .map_err(|_| {
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original alias registration collision",
                    )
                })?
                .cloned();
            let Some(prior) = prior else {
                let live = self
                    .command_token_at_generation(published_token.generation)
                    .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original alias registration token retired",
                    ))?;
                self.alias_registrations
                    .install(&selected, live)
                    .map_err(|_| {
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original alias registration installation",
                        )
                    })?;
                scope.check(self)?;
                return Ok(());
            };
            drop(selected);
            drop(registration_name);
            scope.check(self)?;
            if self
                .name_world
                .borrow()
                .command_deletion_active
                .contains(&prior.generation)
            {
                return Err(self.retain_name_query_refusal(
                    "alias registration collision during actual token deletion",
                ));
            }
            if let Some(live) = self.command_token_at_generation(prior.generation)
                && let Some(command) = self.command_at_sidecar_key(&live.key)
            {
                self.retire_registered_real_command(&live, &command);
                scope.check(self)?;
            } else {
                self.alias_registrations
                    .forget_deleted(|token| token.generation == prior.generation);
            }
        }
    }

    fn complete_c84_alias_registration(
        &mut self,
        src: InterpId,
        src_cmd: &Value,
        published_token: CommandTokenIdentity,
        original: &[u8],
    ) -> Result<Result<(), tcl_cmd_core::CmdError>, tcl_cmd_core::CmdError> {
        let reached = self.in_interp(src, |vm| {
            let scope = native_name_consumers::OriginalNameConsumerScope::enter_for_namespace(
                vm,
                Some(ROOT_NS),
            )?;
            let reached = (|| {
                let selected = vm
                    .name_policy_protocol()
                    .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original alias registration issuer",
                    ))?
                    .recipe()
                    .child_alias_publication_input(
                        tcl_syntax::naming::NativeNameContext::root(),
                        original,
                    )
                    .map_err(|_| {
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original alias registration coordinate",
                        )
                    })?;
                if selected
                    .alias_registration_replacement_key()
                    .map_err(|_| {
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original alias registration purpose",
                        )
                    })?
                    .is_none()
                {
                    return Ok::<_, tcl_cmd_core::CmdError>(());
                }
                drop(selected);
                vm.replace_c84_alias_registration(src_cmd, &published_token, &scope)
            })();
            vm.original_name_finish(scope, reached)
        });
        drop(published_token);
        reached
    }

    fn check_published_alias_loop(
        &mut self,
        src: InterpId,
        src_key: &str,
        original_name_scope: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<(), Completion<Value>> {
        let loops = match self.alias_chain_loops(src, src_key) {
            Ok(loops) => loops,
            Err(error) => {
                if let Some(first) = error.native_execution_refusal() {
                    self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(
                        first.clone(),
                    ));
                }
                self.in_interp(src, |vm| {
                    vm.remove_command_exact(src_key);
                    vm.drop_alias_backref(src_key);
                });
                if let Err(first) = original_name_scope.check(self) {
                    return Err(crate::command::completion_from_cmd_error(self, first));
                }
                return Err(crate::command::completion_from_cmd_error(self, error));
            }
        };
        if let Err(error) = original_name_scope.check(self) {
            return Err(crate::command::completion_from_cmd_error(self, error));
        }
        if loops {
            let tail = self
                .st_of(src)
                .and_then(|state| state.command_slot(src_key))
                .map_or_else(tcl_core_types::NameBytes::default, |slot| {
                    slot.simple.clone()
                });
            let record = self.in_interp(src, |vm| match vm.visible_command_at_key(src_key) {
                Some(
                    Command::Alias {
                        publication_name, ..
                    }
                    | Command::CrossAlias {
                        publication_name, ..
                    },
                ) => publication_name,
                _ => tail.clone(),
            });
            let reported = self
                .actual_native_name_protocol()
                .and_then(|recipe| {
                    recipe
                        .rename_alias_loop_name(record.as_bytes(), tail.as_bytes())
                        .map(<[u8]>::to_vec)
                })
                .unwrap_or_else(|| tail.as_bytes().to_vec());
            self.in_interp(src, |vm| {
                vm.remove_command_exact(src_key);
                vm.drop_alias_backref(src_key);
            });
            if let Err(error) = original_name_scope.check(self) {
                return Err(crate::command::completion_from_cmd_error(self, error));
            }
            let mut message = Vec::from(b"cannot define or rename alias \"".as_slice());
            message.extend_from_slice(&reported);
            message.extend_from_slice(b"\": would create a loop");
            return Err(err(message));
        }
        Ok(())
    }

    fn interp_alias_at_in_original_name(
        &mut self,
        src: InterpId,
        src_cmd: &Value,
        target: InterpId,
        target_words: Vec<Value>,
        original_name_scope: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Completion<Value> {
        if let Err(error) = original_name_scope.check(self) {
            return crate::command::completion_from_cmd_error(self, error);
        }

        let Some(producer) = self.name_policy_protocol() else {
            return self.refuse_host_command("parent alias publication issuer".into());
        };
        for realm in [src, target] {
            let Some(receiver) = self
                .st_of(realm)
                .and_then(InterpState::name_policy_protocol)
            else {
                return self.refuse_host_command("alias interpreter publication issuer".into());
            };
            if receiver != producer {
                return self.refuse_host_command("alias producer realm".into());
            }
        }
        let (original_jim_prefix, words) = if producer.recipe().is_jim084() && src != target {
            match self.capture_original_jim_parent_alias_prefix(target, target_words) {
                Ok((prefix, words)) => (Some(prefix), words),
                Err(error) => return crate::command::completion_from_cmd_error(self, error),
            }
        } else {
            (None, Rc::new(target_words))
        };
        let original = match self.native_name_operand_bytes_checked(src_cmd) {
            Ok(original) => original,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let src_key = self.publish_original_alias(
            src,
            target,
            src_cmd,
            &original,
            &words,
            original_jim_prefix,
        );
        let (src_key, published_token) =
            match self.original_name_outcome(original_name_scope, src_key) {
                Ok(key) => key,
                Err(error) => return crate::command::completion_from_cmd_error(self, error),
            };
        if let Err(error) = original_name_scope.current_for_value(self, src_cmd) {
            return crate::command::completion_from_cmd_error(self, error);
        }
        if let Err(error) = self.check_published_alias_loop(src, &src_key, original_name_scope) {
            return error;
        }
        if let Some(published_token) = published_token {
            if producer.recipe()
                != tcl_syntax::naming::NativeNameProtocol::C(tcl_dialect::TclVersion::V8_4)
            {
                let registered = self.in_interp(src, |vm| {
                    vm.complete_modern_alias_registration(src_cmd, published_token.generation)
                });
                return match self.original_name_outcome(original_name_scope, registered) {
                    Ok(name) => ok(name),
                    Err(error) => crate::command::completion_from_cmd_error(self, error),
                };
            }
            let registration =
                self.complete_c84_alias_registration(src, src_cmd, published_token, &original);
            if let Err(error) = self.original_name_outcome(original_name_scope, registration) {
                return crate::command::completion_from_cmd_error(self, error);
            }
        }
        ok(src_cmd.clone())
    }

    /// C's `TclPreventAliasLoop` (8.6 `tclInterp.c`), on the just-installed
    /// alias at (`defining_interp`, `defining_key`): follow the chain — each
    /// hop resolves the alias's target name from the TARGET interp's
    /// **global** namespace at define time; an unresolved target ends the
    /// chain (legal — aliases late-bind), a non-alias ends it, and a hop
    /// landing back on the defining command is a loop.  Cross-interp hops
    /// address the target by its stable [`InterpId`], so the walk follows the
    /// alias graph across the whole tree.  Terminates because every *existing*
    /// alias already passed this check (no pre-existing loops to spin in).
    fn alias_chain_loops_from(
        &mut self,
        defining_interp: InterpId,
        defining_key: &str,
        first: &Command,
        rename: Option<&CommandRenameTransaction>,
    ) -> Result<bool, tcl_cmd_core::CmdError> {
        let original = native_name_consumers::OriginalNameConsumerScope::enter(self)?;
        let reached = self.alias_chain_loops_from_in_original_name(
            defining_interp,
            defining_key,
            first,
            rename,
            &original,
        );
        let reached = self.original_name_outcome(&original, reached);
        self.original_name_finish(original, reached)?
    }

    fn alias_chain_loops_from_in_original_name(
        &mut self,
        defining_interp: InterpId,
        defining_key: &str,
        first: &Command,
        rename: Option<&CommandRenameTransaction>,
        original: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<bool, tcl_cmd_core::CmdError> {
        use tcl_syntax::value::ValueError;
        original.check(self)?;
        let defining_state =
            self.st_of(defining_interp)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original alias defining interpreter",
                ))?;
        let defining_protocol = defining_state.actual_native_name_protocol().ok_or(
            ValueError::CommandProtocolUnavailable("original alias defining physical name policy"),
        )?;
        if defining_protocol.is_jim084() {
            // Selected Jim publication admits cycles and reaches no loop worker.
            return Ok(false);
        }
        let start = (defining_interp, defining_key.to_owned());
        let defining_slot = defining_state.command_slot(defining_key);
        let owner_key = rename.map_or(defining_key, |transaction| transaction.old_key.as_str());
        let owner_slot = defining_state.command_slot(owner_key);
        let defining_generation = defining_state.visible_command_generation(owner_key).ok_or(
            ValueError::CommandProtocolUnavailable("original alias defining command generation"),
        )?;
        if rename.is_some_and(|transaction| transaction.source_generation != defining_generation) {
            return Err(self.retain_name_query_refusal("original alias rename source token"));
        }
        let defining_world = Rc::clone(&defining_state.name_world);
        tcl_syntax::naming::alias_chain_loops(start.clone(), |(interp, key)| {
            original.check(self)?;
            let command = if (*interp, key.as_str()) == (defining_interp, defining_key) {
                Some(first.clone())
            } else {
                self.st_of(*interp)
                    .and_then(|state| state.visible_command_at_key(key))
            };
            let (target_interp, target_words) = match command {
                Some(Command::Alias { words, .. }) => (*interp, words),
                Some(Command::CrossAlias { target, words, .. }) => (target, words),
                _ => return Ok(None),
            };
            let Some(target_operand) = target_words.first() else {
                return Ok(None);
            };
            if self.st_of(target_interp).is_none() || !self.interp_alive(target_interp) {
                return Err(self.retain_name_query_refusal("original alias target interpreter"));
            }
            // TclPreventAliasLoop reaches GetString, then FindCommand in this
            // genuine target's global namespace. Activation carries only its
            // checked String/Host context; it performs no command invocation.
            let reached = self.in_interp(target_interp, |vm| {
                let target = native_name_consumers::OriginalNameConsumerScope::enter_for_namespace(
                    vm,
                    Some(ROOT_NS),
                )?;
                let result = (|| {
                    let bytes = target.read(vm, target_operand)?;
                    let protocol = vm.actual_native_name_protocol().ok_or(
                        ValueError::CommandProtocolUnavailable(
                            "original alias target physical name policy",
                        ),
                    )?;
                    let selected_slot = protocol
                        .command_lookup_slot(tcl_syntax::naming::NativeNameContext::root(), &bytes)
                        .map_err(|_| {
                            ValueError::CommandProtocolUnavailable(
                                "original alias target root projection",
                            )
                        })?;
                    let projected_namespace = vm
                        .namespace_descendant_token(ROOT_NS, selected_slot.namespace.as_segments());
                    if target_interp == defining_interp
                        && defining_slot.as_ref().is_some_and(|slot| {
                            projected_namespace == Some(slot.namespace)
                                && selected_slot.simple == slot.simple
                        })
                    {
                        return Ok(Some(start.clone()));
                    }
                    vm.resolve_command_bytes_checked(ROOT_NS, &bytes, false)
                        .map(|next| next.map(|key| (target_interp, key)))
                        .map_err(|_| {
                            ValueError::CommandProtocolUnavailable(
                                "original alias target global lookup",
                            )
                            .into()
                        })
                })();
                target.finish(vm)?;
                result
            });
            original.check(self)?;
            let defining_current = self.st_of(defining_interp).is_some_and(|state| {
                Rc::ptr_eq(&state.name_world, &defining_world)
                    && state.visible_command_generation(owner_key) == Some(defining_generation)
                    && state.command_slot(owner_key) == owner_slot
                    && state.command_slot(defining_key) == defining_slot
            });
            if !defining_current {
                return Err(self.retain_name_query_refusal("original alias defining command owner"));
            }
            reached
        })
    }

    fn alias_chain_loops(
        &mut self,
        defining_interp: InterpId,
        defining_key: &str,
    ) -> Result<bool, tcl_cmd_core::CmdError> {
        let Some(first) = self
            .st_of(defining_interp)
            .and_then(|state| state.visible_command_at_key(defining_key))
        else {
            return Ok(false);
        };
        self.alias_chain_loops_from(defining_interp, defining_key, &first, None)
    }

    /// Check the pending destination while retaining the actual source token.
    pub(crate) fn alias_chain_loops_for_rename_transaction(
        &mut self,
        rename: &CommandRenameTransaction,
        key: &str,
        alias: &Command,
    ) -> Result<bool, tcl_cmd_core::CmdError> {
        self.alias_chain_loops_from(self.cur, key, alias, Some(rename))
    }

    #[cfg(test)]
    pub(crate) fn alias_chain_loops_for_rename(
        &mut self,
        key: &str,
        alias: &Command,
    ) -> Result<bool, tcl_cmd_core::CmdError> {
        self.alias_chain_loops_from(self.cur, key, alias, None)
    }

    pub(crate) fn alias_usage_rewrites_value(
        &self,
        original: &Value,
        prefix_words: usize,
        incoming: &[crate::command::NativeArgumentUsageRewrite],
    ) -> Vec<crate::command::NativeArgumentUsageRewrite> {
        let mut usage = incoming.to_vec();
        if self.native_invocation_dialect().argument_usage_rewriting() == Some(true) {
            usage.push(crate::command::NativeArgumentUsageRewrite {
                original_prefix: vec![original.clone()],
                removed_words: prefix_words,
            });
        }
        usage
    }

    pub(crate) fn invoke_alias_words_with_usage(
        &mut self,
        target_interp: InterpId,
        argv: &[Value],
        lookup: tcl_registry::AliasTargetLookup,
        usage: &[crate::command::NativeArgumentUsageRewrite],
    ) -> Completion<Value> {
        if !self.interp_alive(target_interp) {
            return err("could not find interpreter");
        }
        let Some((head, tail)) = argv.split_first() else {
            return err("alias target has no command");
        };
        let completion = if target_interp == self.cur {
            let context = match lookup {
                tcl_registry::AliasTargetLookup::Global => ROOT_NS,
                tcl_registry::AliasTargetLookup::CallerNamespace => self.current_ns_id(),
            };
            self.invoke_command_value_internal_at(
                context,
                head,
                tail,
                usage,
                tcl_registry::command_lookup::CommandLookupOrigin::AliasInvocation,
            )
        } else {
            // C's `TclTransferResult` carries the target's return options,
            // `-errorcode` and `-errorinfo` among them, back to the caller.
            let completion = self.in_interp(target_interp, |vm| {
                let mut completion = vm.invoke_command_value_internal_at(
                    ROOT_NS,
                    head,
                    tail,
                    usage,
                    tcl_registry::command_lookup::CommandLookupOrigin::AliasInvocation,
                );
                if completion.code == Code::Error {
                    completion.options = match vm.completion_options_snapshot(&completion) {
                        Ok(options) => options,
                        Err(error) => return crate::command::completion_from_tcl_error(vm, error),
                    };
                }
                completion
            });
            if completion.code == Code::Error
                && let Err(error) = self.restore_completion_error_state(&completion)
            {
                return crate::command::completion_from_tcl_error(self, error);
            }
            completion
        };
        settle_control_options(self, completion, ControlOptionPolicy::FRESH_FORWARDED)
    }
}
