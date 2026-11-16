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

//! Retiring command identities when their owner or source goes away: TclOO command roles, imports and ensembles.

use super::super::{ImportToken, Interp};
use super::{Command, OoCommandRole, OO_COMMAND_RETIREMENT_ORDER, OO_PRIVATE_COMMAND_ROLES};
use std::rc::Rc;
use tcl_core_types::OoId;

impl Interp {
    /// Current visible, retained, and hidden locations of an owner's exact
    /// command tokens, optionally restricted to one TclOO dispatcher role.
    fn oo_command_locations(
        &self,
        owner: OoId,
        role: Option<OoCommandRole>,
    ) -> Vec<(Vec<u8>, u64)> {
        let mut locations = self
            .namespaces
            .borrow()
            .oo_command_role_locations(owner, role);
        locations.extend(
            self.hidden
                .borrow()
                .iter()
                .filter(|(_, binding)| {
                    binding.command.oo_binding().is_some_and(|(id, candidate)| {
                        id == owner && role.is_none_or(|expected| expected == candidate)
                    })
                })
                .map(|(name, binding)| {
                    let fqn = tcl_syntax::naming::root_unrooted_key_bytes(name);
                    (fqn, binding.generation)
                }),
        );
        locations
    }

    /// Fire one delete-trace walk per command generation, resolving the
    /// generation's current placement immediately before its turn. An earlier
    /// callback may rename or hide a later token. The callback may also move
    /// the token currently firing, so clean its trace sidecar again at every
    /// post-callback placement rather than falling back to a display name.
    pub(crate) fn fire_oo_command_delete_traces(
        &mut self,
        owner: OoId,
        role: Option<OoCommandRole>,
    ) {
        let generations: std::collections::BTreeSet<u64> = self
            .oo_command_locations(owner, role)
            .into_iter()
            .map(|(_, generation)| generation)
            .collect();
        for generation in generations {
            let location = self
                .oo_command_locations(owner, role)
                .into_iter()
                .find(|(_, candidate)| *candidate == generation);
            if let Some((fqn, _)) = location {
                self.fire_delete_traces_of_token(&fqn, Some(generation));
            }
            for (fqn, _) in self
                .oo_command_locations(owner, role)
                .into_iter()
                .filter(|(_, candidate)| *candidate == generation)
            {
                self.remove_cmd_traces_of_token(&fqn, Some(generation));
            }
        }
    }

    /// Fire an owner's public and private command-token traces in TclOO's
    /// semantic teardown order. Resolving each role immediately before its
    /// turn lets an earlier callback relocate a later dispatcher.
    pub(crate) fn fire_oo_command_role_delete_traces(&mut self, owner: OoId) {
        for role in OO_COMMAND_RETIREMENT_ORDER {
            self.fire_oo_command_delete_traces(owner, Some(role));
        }
    }

    /// Remove already-prefired command roles without replaying their delete
    /// callbacks. Object teardown uses this for the private dispatchers before
    /// instance-variable traces run; the public command remains until after
    /// that namespace phase.
    pub(crate) fn remove_prefired_oo_private_commands(&mut self, owner: OoId) {
        self.remove_oo_command_roles(owner, &OO_PRIVATE_COMMAND_ROLES);
    }

    /// Retire every command token carried by one TclOO owner identity. The
    /// namespace arena covers both visible and retained generations; the
    /// interpreter-owned hidden table is folded into the same transaction.
    /// Delete-trace callbacks may move a token between those stores, so the
    /// commands are rescanned by identity after callbacks before removal.
    pub(crate) fn retire_oo_command_identity(&mut self, owner: OoId) {
        self.retire_oo_command_identity_impl(owner, true);
    }

    /// Remove an owner's command tokens after its semantic role walk already
    /// fired. A later-role callback may add a trace to an earlier-role token;
    /// final removal discards that late trace without revisiting the role.
    pub(crate) fn retire_prefired_oo_command_identity(&mut self, owner: OoId) {
        self.retire_oo_command_identity_impl(owner, false);
    }

    fn retire_oo_command_identity_impl(&mut self, owner: OoId, fire_traces: bool) {
        if !self.0.retiring_oo_commands.borrow_mut().insert(owner) {
            return;
        }

        if fire_traces {
            self.fire_oo_command_role_delete_traces(owner);
        }

        self.remove_oo_command_roles(owner, &OO_COMMAND_RETIREMENT_ORDER);
        self.forget_registry_object_root(owner);
        self.0.retiring_oo_commands.borrow_mut().remove(&owner);
    }

    fn remove_oo_command_roles(&mut self, owner: OoId, roles: &[OoCommandRole]) {
        if self.uses_original_c_command_retirement() {
            let generations = self
                .oo_command_locations(owner, None)
                .into_iter()
                .filter_map(|(_, generation)| {
                    self.raw_command_by_generation(generation)
                        .and_then(|command| command.oo_binding())
                        .filter(|(_, role)| roles.contains(role))
                        .map(|_| generation)
                })
                .collect::<std::collections::BTreeSet<_>>();
            for generation in generations {
                self.finish_original_command_deletion(generation, false);
            }
            return;
        }

        let mut removed: Vec<(Vec<u8>, u64)> = self
            .namespaces
            .borrow_mut()
            .remove_oo_command_roles(owner, roles);
        let hidden_names: Vec<Vec<u8>> = self
            .hidden
            .borrow()
            .iter()
            .filter(|(_, binding)| {
                binding
                    .command
                    .oo_binding()
                    .is_some_and(|(id, role)| id == owner && roles.contains(&role))
            })
            .map(|(name, _)| name.clone())
            .collect();
        for name in hidden_names {
            if let Some(binding) = self.hidden.borrow_mut().remove(&name) {
                self.namespaces
                    .borrow_mut()
                    .retire_native_command_node(binding.generation);
                let fqn = tcl_syntax::naming::root_unrooted_key_bytes(&name);
                removed.push((fqn, binding.generation));
            }
        }
        let origins: std::collections::BTreeSet<u64> =
            removed.iter().map(|(_, generation)| *generation).collect();
        self.remove_imports_for_deleted_origins(origins, &[]);
        for (fqn, generation) in removed {
            self.remove_cmd_traces_of_token(&fqn, Some(generation));
        }
    }

    /// Fire `fqn`'s `delete` traces and drop the ones the dying token owned.
    pub(in super::super) fn fire_delete_traces_of_token(&mut self, fqn: &[u8], dying: Option<u64>) {
        if self
            .traces
            .borrow()
            .cmd_traces
            .iter()
            .all(|t| t.name != fqn)
        {
            return;
        }
        self.fire_cmd_trace_of_token(fqn, fqn, b"", crate::cmd_trace::ops::DELETE, dying);
        self.remove_cmd_traces_of_token(fqn, dying);
    }

    /// Move every import retaining one exact source command generation. Hidden
    /// imports carry the same metadata as visible ones and must move too.
    pub(super) fn retarget_import_sources(&mut self, source_generation: u64, new_fqn: &[u8]) {
        self.namespaces
            .borrow_mut()
            .retarget_imports(source_generation, new_fqn);
        for binding in self.hidden.borrow_mut().values_mut() {
            if let Command::Imported {
                source,
                source_generation: candidate,
                ..
            } = &mut binding.command
            {
                if *candidate == source_generation {
                    *source = new_fqn.to_vec();
                }
            }
        }
    }

    /// Bind surviving by-name replacement redirects to a replacement's fresh
    /// generation. An import whose old generation still exists elsewhere was
    /// moved by a callback and remains attached to that exact token.
    pub(super) fn reattach_missing_import_sources(
        &mut self,
        source_fqn: &[u8],
        new_generation: u64,
    ) {
        let mut live_generations = self.namespaces.borrow().command_generations();
        live_generations.extend(
            self.hidden
                .borrow()
                .values()
                .map(|binding| binding.generation),
        );
        self.namespaces.borrow_mut().reattach_missing_imports(
            source_fqn,
            new_generation,
            &live_generations,
        );
        for binding in self.hidden.borrow_mut().values_mut() {
            let Command::Imported {
                source,
                source_generation,
                ..
            } = &mut binding.command
            else {
                continue;
            };
            if source == source_fqn && !live_generations.contains(source_generation) {
                *source_generation = new_generation;
            }
        }
    }

    /// Retarget imports of one exact source generation to a newly-created
    /// ensemble token, across visible and hidden import bindings.
    pub(super) fn retarget_imports_to_ensemble(
        &mut self,
        source_generation: u64,
        new: &Rc<crate::ensemble::EnsembleToken>,
    ) {
        self.namespaces
            .borrow_mut()
            .retarget_imports_to_ensemble(source_generation, new);
        for binding in self.hidden.borrow_mut().values_mut() {
            let Command::Imported {
                source_generation: candidate,
                ensemble,
                ..
            } = &mut binding.command
            else {
                continue;
            };
            if *candidate == source_generation {
                *ensemble = Some(Rc::clone(new));
            }
        }
    }

    /// Remove one imported command by stable identity wherever a delete-trace
    /// callback may have renamed or hidden it. A callback replacement/re-import
    /// has a new identity and is deliberately left alone.
    pub(in super::super) fn import_identity_location(
        &self,
        identity: &Rc<ImportToken>,
    ) -> Option<(Vec<u8>, u64)> {
        if let Some(location) = self.namespaces.borrow().import_identity_location(identity) {
            return Some(location);
        }
        self.hidden.borrow().iter().find_map(|(name, binding)| {
            matches!(
                &binding.command,
                Command::Imported { identity: current, .. }
                    if Rc::ptr_eq(current, identity)
            )
            .then(|| {
                let fqn = tcl_syntax::naming::root_unrooted_key_bytes(name);
                (fqn, binding.generation)
            })
        })
    }

    pub(super) fn remove_import_identity(
        &mut self,
        identity: &Rc<ImportToken>,
    ) -> Option<(Vec<u8>, u64)> {
        let visible = self
            .namespaces
            .borrow_mut()
            .remove_import_identity(identity);
        if let Err(first) = self.retire_pending_native_ensemble_roles() {
            self.refuse_native_execution(first);
        }
        if let Some(fqn) = visible {
            return Some(fqn);
        }
        let hidden_name = self.hidden.borrow().iter().find_map(|(name, binding)| {
            matches!(
                &binding.command,
                Command::Imported { identity: current, .. }
                    if Rc::ptr_eq(current, identity)
            )
            .then(|| name.clone())
        })?;
        let binding = self.hidden.borrow_mut().remove(&hidden_name)?;
        self.namespaces
            .borrow_mut()
            .retire_native_command_node(binding.generation);
        let fqn = tcl_syntax::naming::root_unrooted_key_bytes(&hidden_name);
        Some((fqn, binding.generation))
    }

    pub(in super::super) fn ensemble_identity_location(
        &self,
        identity: &Rc<crate::ensemble::EnsembleToken>,
    ) -> Option<(Vec<u8>, u64)> {
        if let Some(location) = self
            .namespaces
            .borrow()
            .ensemble_identity_location(identity)
        {
            return Some(location);
        }
        self.hidden.borrow().iter().find_map(|(name, binding)| {
            matches!(
                &binding.command,
                Command::Ensemble(current) if Rc::ptr_eq(current, identity)
            )
            .then(|| {
                let fqn = tcl_syntax::naming::root_unrooted_key_bytes(name);
                (fqn, binding.generation)
            })
        })
    }

    /// Remove one ensemble by stable token identity after running the delete
    /// trace at the name where deletion began. The callback may rename, hide,
    /// expose, or replace the command; only the captured token is retired, and
    /// any trace sidecar moved with it is dropped without firing twice.
    pub(in super::super) fn retire_ensemble_identity(
        &mut self,
        identity: &Rc<crate::ensemble::EnsembleToken>,
    ) -> Option<(Vec<u8>, u64)> {
        self.retire_ensemble_identity_using(identity, true)
    }

    pub(super) fn retire_ensemble_replacement_identity(
        &mut self,
        identity: &Rc<crate::ensemble::EnsembleToken>,
    ) -> Option<(Vec<u8>, u64)> {
        self.retire_ensemble_identity_using(identity, false)
    }

    fn retire_ensemble_identity_using(
        &mut self,
        identity: &Rc<crate::ensemble::EnsembleToken>,
        true_deletion: bool,
    ) -> Option<(Vec<u8>, u64)> {
        if true_deletion && self.uses_original_c_command_retirement() {
            let original = self.ensemble_identity_location(identity)?;
            self.finish_original_command_deletion(original.1, true);
            return Some(original);
        }

        if let Some((trace_fqn, generation)) = self.ensemble_identity_location(identity) {
            self.fire_delete_traces_of_token(&trace_fqn, Some(generation));
        }
        let visible_removed = self
            .namespaces
            .borrow_mut()
            .remove_ensemble_identity(identity);
        if let Err(first) = self.retire_pending_native_ensemble_roles() {
            self.refuse_native_execution(first);
        }
        let removed_fqn = visible_removed.or_else(|| {
            let hidden_name = self.hidden.borrow().iter().find_map(|(name, binding)| {
                matches!(
                    &binding.command,
                    Command::Ensemble(current) if Rc::ptr_eq(current, identity)
                )
                .then(|| name.clone())
            })?;
            let binding = self.hidden.borrow_mut().remove(&hidden_name)?;
            self.namespaces
                .borrow_mut()
                .retire_native_command_node(binding.generation);
            let fqn = tcl_syntax::naming::root_unrooted_key_bytes(&hidden_name);
            Some((fqn, binding.generation))
        });
        if let Some((live_fqn, generation)) = removed_fqn.as_ref() {
            self.remove_cmd_traces_of_token(live_fqn, Some(*generation));
        }
        identity.mark_deleted();
        removed_fqn
    }

    /// Remove every visible or hidden import whose immediate origin was truly
    /// deleted. Delete traces fire while each visible imported command is still
    /// in its table; the stable import identity then prevents a callback's
    /// replacement command from being removed as if it were the old import.
    /// Removed aliases are fed back into the set until transitive chains reach a
    /// fixed point. A replacement never calls this seam, so recreating an old
    /// source name cannot resurrect aliases deleted here.
    pub(in super::super) fn remove_imports_for_deleted_origins(
        &mut self,
        origins: impl IntoIterator<Item = u64>,
        tokens: &[Rc<crate::ensemble::EnsembleToken>],
    ) {
        if self.uses_original_c_command_retirement() {
            self.retire_original_import_origins(origins, tokens);
            return;
        }

        let mut origins: std::collections::HashSet<u64> = origins.into_iter().collect();
        loop {
            let visible = self
                .namespaces
                .borrow()
                .imports_for_origins(&origins, tokens);
            let hidden: Vec<(Vec<u8>, Rc<ImportToken>)> = self
                .hidden
                .borrow()
                .iter()
                .filter_map(|(name, binding)| {
                    let Command::Imported {
                        source_generation,
                        ensemble,
                        identity,
                        ..
                    } = &binding.command
                    else {
                        return None;
                    };
                    let retains_token = ensemble.as_ref().is_some_and(|imported| {
                        tokens.iter().any(|victim| Rc::ptr_eq(imported, victim))
                    });
                    if !origins.contains(source_generation) && !retains_token {
                        return None;
                    }
                    let fqn = tcl_syntax::naming::root_unrooted_key_bytes(name);
                    Some((fqn, Rc::clone(identity)))
                })
                .collect();

            let mut removed_any = false;
            for (_, identity) in visible {
                let Some((fqn, generation)) = self.import_identity_location(&identity) else {
                    continue;
                };
                if self.check_entered_native_operation().is_ok() {
                    self.fire_delete_traces_of_token(&fqn, Some(generation));
                }
                if let Some((removed_fqn, removed_generation)) =
                    self.remove_import_identity(&identity)
                {
                    self.remove_cmd_traces_of_token(&removed_fqn, Some(removed_generation));
                    origins.insert(removed_generation);
                    removed_any = true;
                }
            }
            for (_, identity) in hidden {
                let Some((fqn, generation)) = self.import_identity_location(&identity) else {
                    continue;
                };
                if self.check_entered_native_operation().is_ok() {
                    self.fire_delete_traces_of_token(&fqn, Some(generation));
                }
                if let Some((removed_fqn, removed_generation)) =
                    self.remove_import_identity(&identity)
                {
                    self.remove_cmd_traces_of_token(&removed_fqn, Some(removed_generation));
                    origins.insert(removed_generation);
                    removed_any = true;
                }
            }
            if !removed_any {
                break;
            }
        }
    }
}
