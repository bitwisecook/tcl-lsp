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

//! The native publication service: committing prepared native registrations and deletions, and capturing and retaining registered command tokens.

use super::super::native_name_world::{
    NativePreparedPublication, NativePublicationCommit, NativePublicationError,
    NativePublicationService,
};
use super::super::{CommandTokenIdentity, NativeRegisteredCommandToken, Vm, native_name_world};
use super::CommandSidecarKey;
use crate::command::Command;
use std::rc::Rc;

impl Vm {
    /// Issue an owned native callback-scope publication service.
    /// Guest-executing deletion lifecycles are refused before bridge callbacks.
    ///
    /// # Errors
    /// Refuses unavailable protocols or retired interpreter/namespace scopes.
    pub fn native_publication_service(
        &self,
    ) -> Result<NativePublicationService, NativePublicationError> {
        let policy = self
            .name_policy_protocol()
            .ok_or(NativePublicationError::ProtocolUnavailable)?;
        if policy.authority() != tcl_syntax::naming::NamePolicyAuthority::Native {
            return Err(NativePublicationError::ProtocolUnavailable);
        }
        self.refresh_native_publication_deletion_effects();
        NativePublicationService::new(&self.name_world, self.current_ns_id(), policy.recipe())
    }

    fn refresh_native_publication_deletion_effects(&self) {
        use native_name_world::NativeDeletionEffects;
        let slots: Vec<_> = {
            let world = self.name_world.borrow();
            world
                .command_identity
                .slot_by_key
                .iter()
                .map(|(key, slot)| (key.clone(), slot.clone()))
                .collect()
        };
        let rows: Vec<_> = slots
            .into_iter()
            .filter_map(|(key, slot)| {
                self.visible_command_generation(&key)
                    .map(|generation| (key, slot, generation))
            })
            .collect();
        let classifications: Vec<_> = rows
            .into_iter()
            .map(|(key, slot, generation)| {
                let sidecar = CommandSidecarKey::visible(&key);
                let token = CommandTokenIdentity {
                    key: sidecar.clone(),
                    generation,
                };
                let guest = self.command_retirement_requires_guest(&sidecar)
                    || self
                        .downstream_import_bindings(&token)
                        .iter()
                        .any(|key| self.command_retirement_requires_guest(key));
                (
                    slot,
                    generation,
                    if guest {
                        NativeDeletionEffects::GuestExecutionRequired
                    } else {
                        NativeDeletionEffects::NoGuestExecution
                    },
                )
            })
            .collect();
        let mut world = self.name_world.borrow_mut();
        for (slot, generation, effects) in classifications {
            world.note_deletion_effects(slot, generation, effects);
        }
    }

    fn command_retirement_requires_guest(&self, key: &CommandSidecarKey) -> bool {
        self.cmd_traces.get(key).is_some_and(|traces| {
            traces.iter().any(|trace| {
                !trace.untraced.get() && trace.ops.iter().any(|operation| operation == "delete")
            })
        }) || crate::cmd_coro::is_coroutine(self, key.name())
            || matches!(
                self.command_at_sidecar_key(key),
                Some(Command::Object(_) | Command::ChildInterp(_))
            )
    }

    /// Commit an authenticated exact C registration slot after bridge callbacks.
    ///
    /// # Errors
    /// Refuses foreign, unnoted, consumed or retired receipts.
    pub fn define_prepared_native_command(
        &mut self,
        service: &NativePublicationService,
        receipt: &NativePreparedPublication,
        command: Rc<dyn crate::command::NativeCommand>,
    ) -> Result<Option<NativeRegisteredCommandToken>, NativePublicationError> {
        service.validate_world(&self.name_world)?;
        let action = service.inspect_commit(receipt)?;
        let token = match action {
            NativePublicationCommit::Superseded => None,
            NativePublicationCommit::Apply {
                slot,
                present: true,
            } => {
                let key = self.register_command_in_slot(slot.clone(), Command::Native(command));
                self.visible_command_generation(&key).map(|generation| {
                    NativeRegisteredCommandToken {
                        owner: receipt.key().owner,
                        interpreter: receipt.key().interpreter,
                        slot,
                        generation,
                    }
                })
            }
            NativePublicationCommit::Apply { present: false, .. } => {
                return Err(NativePublicationError::WrongDisposition);
            }
        };
        service.finish_commit(receipt)?;
        self.refresh_native_publication_deletion_effects();
        Ok(token)
    }

    /// Commit an authenticated native deletion without resolving its name again.
    ///
    /// # Errors
    /// Refuses foreign, unnoted, consumed or retired receipts.
    pub fn remove_prepared_native_command(
        &mut self,
        service: &NativePublicationService,
        receipt: &NativePreparedPublication,
    ) -> Result<bool, NativePublicationError> {
        service.validate_world(&self.name_world)?;
        let removed = match service.inspect_commit(receipt)? {
            NativePublicationCommit::Superseded => false,
            NativePublicationCommit::Apply {
                slot,
                present: false,
            } => {
                if let Some(key) = self.command_storage_key_at_slot(&slot) {
                    self.materialise_retained_binding(&key);
                    let present = self.commands.contains_key(&key);
                    self.remove_registered_command(&key);
                    present
                } else {
                    false
                }
            }
            NativePublicationCommit::Apply { present: true, .. } => {
                return Err(NativePublicationError::WrongDisposition);
            }
        };
        service.finish_commit(receipt)?;
        self.refresh_native_publication_deletion_effects();
        Ok(removed)
    }

    /// Capture the current binding reached through the actual byte lookup recipe.
    #[must_use]
    pub fn registered_command_token_bytes(
        &self,
        original: &[u8],
    ) -> Option<NativeRegisteredCommandToken> {
        let key = self.resolve_command_from_token_bytes(self.current_ns_id(), original, true)?;
        Some(NativeRegisteredCommandToken {
            owner: self.owner_nonce,
            interpreter: u64::try_from(self.cur.0).ok()?,
            slot: self.command_slot(&key)?,
            generation: self.visible_command_generation(&key)?,
        })
    }

    /// True only after every native callback publication has been committed.
    #[must_use]
    pub fn native_publications_are_committed(&self) -> bool {
        self.name_world
            .borrow()
            .pending_publications_are_committed()
    }

    /// Actual stock ensemble worker generations needed by retained command tokens.
    /// The original native mapping is required; configured or user-authored
    /// ensembles grant no additional retention through this query.
    #[must_use]
    pub fn registered_stock_implementation_tokens(
        &self,
        roots: &[NativeRegisteredCommandToken],
    ) -> Vec<NativeRegisteredCommandToken> {
        let mut tokens = roots.to_vec();
        let mut cursor = 0;
        while cursor < tokens.len() {
            let root = tokens[cursor].clone();
            cursor += 1;
            if root.owner != self.owner_nonce
                || root.interpreter != u64::try_from(self.cur.0).expect("interpreter slot fits u64")
            {
                continue;
            }
            let Some((key, command)) = self.commands.iter().find(|(key, _)| {
                self.command_slot(key).as_ref() == Some(&root.slot)
                    && self.visible_command_generation(key) == Some(root.generation)
            }) else {
                continue;
            };
            if matches!(command, Command::Builtin(_))
                && !self.is_host_builtin(key)
                && self.builtin_identity_for_key(key).as_deref() == Some("expr")
            {
                for token in self.stock_expression_implementation_tokens() {
                    if !tokens.contains(&token) {
                        tokens.push(token);
                    }
                }
            }
            let Command::Ensemble(ensemble) = command else {
                continue;
            };
            let config = ensemble.config();
            let Some(native) = config.native_implementation() else {
                continue;
            };
            for (_, prefix) in &native.map {
                let Some(head) = prefix.first() else { continue };
                let Some(key) =
                    self.resolve_command_from_token_bytes(config.namespace, head.as_bytes(), false)
                else {
                    continue;
                };
                let Some(slot) = self.command_slot(&key) else {
                    continue;
                };
                let Some(generation) = self.visible_command_generation(&key) else {
                    continue;
                };
                let token = NativeRegisteredCommandToken {
                    owner: root.owner,
                    interpreter: root.interpreter,
                    slot,
                    generation,
                };
                if !tokens.contains(&token) {
                    tokens.push(token);
                }
            }
        }
        tokens
    }

    /// Expression workers are selected from actual stock registrations. The
    /// shared function roster controls membership; stateful random generators
    /// are excluded from the restricted host surface.
    fn stock_expression_implementation_tokens(&self) -> Vec<NativeRegisteredCommandToken> {
        self.commands
            .iter()
            .filter_map(|(key, command)| {
                if !matches!(command, Command::Builtin(_)) || self.is_host_builtin(key) {
                    return None;
                }
                let identity = self.builtin_identity_for_key(key)?;
                if !tcl_registry::mathfunc::restricted_expression_command_identity(&identity) {
                    return None;
                }
                Some(NativeRegisteredCommandToken {
                    owner: self.owner_nonce,
                    interpreter: u64::try_from(self.cur.0).ok()?,
                    slot: self.command_slot(key)?,
                    generation: self.visible_command_generation(key)?,
                })
            })
            .collect()
    }

    /// Retain only captured generations that still occupy their original slots.
    pub fn retain_registered_command_tokens(
        &mut self,
        tokens: &[NativeRegisteredCommandToken],
    ) -> usize {
        let owner = self.owner_nonce;
        let interpreter = u64::try_from(self.cur.0).expect("interpreter token fits u64");
        let remove: Vec<_> = self
            .commands
            .keys()
            .filter(|key| {
                let Some(slot) = self.command_slot(key) else {
                    return true;
                };
                let Some(generation) = self.visible_command_generation(key) else {
                    return true;
                };
                !tokens.iter().any(|token| {
                    token.owner == owner
                        && token.interpreter == interpreter
                        && token.slot == slot
                        && token.generation == generation
                })
            })
            .cloned()
            .collect();
        for key in &remove {
            self.remove_registered_command(key);
        }
        remove.len()
    }
}
