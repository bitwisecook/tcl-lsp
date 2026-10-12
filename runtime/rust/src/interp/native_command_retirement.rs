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

//! Actual command deletion, with captured roles and physical cleanup currency.

use super::{
    native_arguments::OriginalArgumentFrame,
    native_operation_currency::{
        NativeOperationScope, NativeSavedCleanupScope, SavedNativeOperationCurrency,
    },
    Command, Interp,
};
use std::rc::Rc;
use tcl_cmd_core::{
    native_command_retirement::{retire_original_command, NativeCommandRetirementStage},
    CmdError,
};
use tcl_runtime_api::native_namespace_name::NativeNamespaceNameToken;
use tcl_syntax::{native_namespace_name::NativeNamespaceLifecycle, value::ValueError};

#[derive(Clone, Copy)]
pub(super) struct OriginalCommandDeletionRecipe {
    order: tcl_cmd_core::native_command_retirement::NativeCommandRetirementOrder,
    names: tcl_registry::native_command_literal::NativeCommandNameProtocol,
    namespace_frontier: Option<tcl_syntax::native_namespace_name::NativeNamespaceCommandTeardown>,
}

struct OriginalDeletion {
    frame: OriginalArgumentFrame,
    caller_namespace: crate::namespace::NsId,
    namespace: Option<(NativeNamespaceNameToken, NativeNamespaceLifecycle)>,
    scope: NativeSavedCleanupScope,
}
impl OriginalDeletion {
    fn capture(actual: &Interp) -> Result<Self, CmdError> {
        let frame = OriginalArgumentFrame::capture(actual);
        let caller_namespace = actual.current_ns.get();
        let namespace = actual
            .namespaces
            .borrow_mut()
            .namespace_name_token(actual.native_command_interpreter, caller_namespace)
            .map(|token| {
                let lifecycle = token.lifecycle();
                (token, lifecycle)
            });
        let saved = SavedNativeOperationCurrency::capture(actual);
        let scope = NativeOperationScope::enter_saved_cleanup(&saved, actual)
            .map_err(CmdError::from_execution_refusal)?;
        Ok(Self {
            frame,
            caller_namespace,
            namespace,
            scope,
        })
    }
    fn check(&self, actual: &Interp) -> Result<(), CmdError> {
        self.scope
            .check()
            .map_err(CmdError::from_execution_refusal)?;
        let same_namespace = actual.current_ns.get() == self.caller_namespace
            && self.namespace.as_ref().is_some_and(|(token, lifecycle)| {
                let namespaces = actual.namespaces.borrow();
                namespaces.owns_namespace_name_token(token)
                    && token.lifecycle() == *lifecycle
                    && *lifecycle != NativeNamespaceLifecycle::Dead
            });
        let frame = self.frame.check_frame(actual);
        if !same_namespace || frame.is_err() {
            actual
                .clone()
                .refuse_completion_value_access(frame.err().unwrap_or(
                    ValueError::CommandProtocolUnavailable(
                        "original command deletion caller namespace",
                    ),
                ));
        }
        self.scope.check().map_err(CmdError::from_execution_refusal)
    }
}

impl Interp {
    pub(crate) fn finish_original_command_deletion(
        &mut self,
        generation: u64,
        traces: bool,
    ) -> bool {
        match self.retire_original_command_generation(generation, traces) {
            Ok(removed) => removed,
            Err(error) => {
                self.report_cmd_error(error);
                false
            }
        }
    }

    pub(super) fn retire_original_namespace_command_table(
        &mut self,
        namespace: crate::namespace::NsId,
    ) {
        let Some(recipe) = self.original_command_deletion_recipe() else {
            return;
        };
        let traversal = recipe.namespace_frontier;
        self.original_command_retirement_recipes
            .borrow_mut()
            .push(recipe);
        loop {
            let mut frontier = self.namespaces.borrow().command_hash_order(namespace);
            if let Some(traversal) = traversal {
                frontier.truncate(traversal.frontier_len(frontier.len()));
            }
            if frontier.is_empty() {
                break;
            }
            let mut progressed = false;
            for (tail, generation) in frontier {
                if self
                    .namespaces
                    .borrow()
                    .command_generation(namespace, &tail)
                    != Some(generation)
                {
                    continue;
                }
                self.finish_original_command_deletion(generation, true);
                progressed = true;
            }
            if !progressed {
                break;
            }
        }
        let _ = self.original_command_retirement_recipes.borrow_mut().pop();
    }

    pub(super) fn sweep_original_dying_namespace(
        &mut self,
        root: crate::namespace::NsId,
        retained: &[crate::namespace::NsId],
    ) {
        let original = match OriginalDeletion::capture(self) {
            Ok(original) => original,
            Err(error) => {
                self.report_cmd_error(error);
                return;
            }
        };
        let Some(recipe) = self.original_command_deletion_recipe() else {
            return;
        };
        self.original_command_retirement_recipes
            .borrow_mut()
            .push(recipe);
        let mut ids = retained
            .iter()
            .copied()
            .filter(|id| !self.namespaces.borrow().under_deferred_token(*id))
            .collect::<Vec<_>>();
        loop {
            for namespace in self.retained_teardown_ids(root) {
                if !ids.contains(&namespace) {
                    ids.push(namespace);
                }
            }
            for &namespace in &ids {
                self.retire_namespace_owned_ensembles(namespace);
                self.retire_original_namespace_command_table(namespace);
            }
            if let Err(error) = original.check(self) {
                self.report_cmd_error(error);
            }
            let reached = self.retained_teardown_ids(root);
            let additional = reached.iter().any(|namespace| !ids.contains(namespace));
            let remaining = self.namespaces.borrow().command_slots_in_ids(&ids);
            if !additional && remaining.is_empty() {
                break;
            }
        }
        let retired = self.namespaces.borrow_mut().clear_namespace_ids(&ids);
        drop(retired);
        if let Err(error) = original.check(self) {
            self.report_cmd_error(error);
        }
        if let Err(error) = self.retire_pending_native_ensemble_roles() {
            self.refuse_native_execution(error);
        }
        let _ = self.original_command_retirement_recipes.borrow_mut().pop();
    }

    /// A pure order is consumed only with this actual selected C command issuer.
    /// Jim and authored neutral owners retain their separate deletion workers.
    pub(super) fn original_command_deletion_recipe(&self) -> Option<OriginalCommandDeletionRecipe> {
        if let Some(recipe) = self
            .original_command_retirement_recipes
            .borrow()
            .last()
            .copied()
        {
            return Some(recipe);
        }
        let dialect = self.native_invocation_dialect();
        Some(OriginalCommandDeletionRecipe {
            order: dialect
                .native_command_creation_protocol()?
                .c_deletion_order()?,
            names: dialect.native_command_name_protocol()?,
            namespace_frontier: self
                .name_policy_protocol()
                .and_then(|policy| policy.recipe().namespace_command_teardown()),
        })
    }
    pub(crate) fn uses_original_c_command_retirement(&self) -> bool {
        self.original_command_deletion_recipe().is_some()
    }

    fn unlink_original_deletion(&mut self, generation: u64) -> bool {
        let visible = self
            .namespaces
            .borrow_mut()
            .unlink_original_command_generation(generation);
        if let Some(binding) = visible {
            drop(binding);
            return true;
        }
        let hidden =
            self.hidden.borrow().iter().find_map(|(name, binding)| {
                (binding.generation == generation).then(|| name.clone())
            });
        let retired = hidden.and_then(|name| self.hidden.borrow_mut().remove(&name));
        let removed = retired.is_some();
        drop(retired);
        removed
    }

    pub(super) fn retire_original_command_generation(
        &mut self,
        generation: u64,
        fire_traces: bool,
    ) -> Result<bool, CmdError> {
        self.retire_original_command_generation_using(generation, fire_traces, false)
    }

    /// C creation preserves its actual imported-command roster through the
    /// old-token Delete call: C84 detaches importRefPtr; C85+ marks redefinition.
    pub(super) fn retire_original_command_for_creation(
        &mut self,
        generation: u64,
    ) -> Result<bool, CmdError> {
        self.retire_original_command_generation_using(generation, true, true)
    }

    fn retire_original_command_generation_using(
        &mut self,
        generation: u64,
        fire_traces: bool,
        preserve_creation_imports: bool,
    ) -> Result<bool, CmdError> {
        let recipe = self.original_command_deletion_recipe().ok_or(
            ValueError::CommandProtocolUnavailable("original C command deletion issuer"),
        )?;
        let order = recipe.order;
        let protocol = recipe.names;
        let recursive = self
            .namespaces
            .borrow()
            .original_command_deletion_started(generation);
        let captured = if recursive {
            None
        } else {
            self.raw_command_location_by_generation(generation)
        };
        if !recursive && captured.is_none() {
            return Ok(false);
        }
        if protocol.deletion_epoch_before_trace(recursive) {
            self.namespaces
                .borrow_mut()
                .advance_original_deletion_epoch(generation);
        }
        if recursive {
            let removed = self.unlink_original_deletion(generation);
            if protocol.deletion_epoch_after_unlink(true) {
                self.namespaces
                    .borrow_mut()
                    .advance_original_deletion_epoch(generation);
            }
            // The outer actual owner still retires implementation and imports.
            return Ok(removed);
        }
        let (original_fqn, command) = captured.expect("original captured deletion");
        let original = OriginalDeletion::capture(self)?;
        self.namespaces
            .borrow_mut()
            .begin_original_command_deletion(generation);
        self.original_command_retirement_recipes
            .borrow_mut()
            .push(recipe);
        let mut first = original.check(self).err();
        if fire_traces && first.is_none() {
            self.fire_delete_traces_of_token(&original_fqn, Some(generation));
            if let Err(error) = original.check(self) {
                first.get_or_insert(error);
            }
        }
        self.namespaces.borrow_mut().original_deletion_after_trace(
            generation,
            protocol.deletion_invalidates_namespace_after_trace(),
        );
        let stages = retire_original_command(order, |stage| match stage {
            NativeCommandRetirementStage::Implementation => {
                self.retire_original_command_implementation(generation, &command, &original)
            }
            NativeCommandRetirementStage::Imports if preserve_creation_imports => {
                original.check(self)
            }
            NativeCommandRetirementStage::Imports => {
                let tokens = match &command {
                    Command::Ensemble(token) => vec![Rc::clone(token)],
                    _ => Vec::new(),
                };
                self.retire_original_imports([generation], &tokens, &original)
            }
        });
        if let Err(error) = stages {
            first.get_or_insert(error);
        }
        // Only the actual original generation is unlinked wherever callbacks
        // moved it. A replacement at its former report is a different command.
        let moved_fqn = self
            .raw_command_location_by_generation(generation)
            .map(|(fqn, _)| fqn);
        let removed = self.unlink_original_deletion(generation);
        if removed && protocol.deletion_epoch_after_unlink(false) {
            self.namespaces
                .borrow_mut()
                .advance_original_deletion_epoch(generation);
        }
        if let Some(fqn) = moved_fqn {
            self.remove_cmd_traces_of_token(&fqn, Some(generation));
        }
        self.remove_cmd_traces_of_token(&original_fqn, Some(generation));
        if let Err(error) = original.check(self) {
            first.get_or_insert(error);
        }
        if let Command::ObjCmd(record) = &command {
            record.original_deleted.set(true);
        }
        drop(command);
        if let Err(error) = original.check(self) {
            first.get_or_insert(error);
        }
        if let Err(error) = self.retire_pending_native_ensemble_roles() {
            first.get_or_insert(CmdError::from_execution_refusal(error));
        }
        if let Err(error) = original.check(self) {
            first.get_or_insert(error);
        }
        let _ = self.original_command_retirement_recipes.borrow_mut().pop();
        // An inner raw unlink does not cancel the outer captured deletion.
        first.map_or(Ok(true), Err)
    }

    fn retire_original_command_implementation(
        &mut self,
        generation: u64,
        command: &Command,
        original: &OriginalDeletion,
    ) -> Result<(), CmdError> {
        let mut first = original.check(self).err();
        crate::cmd_coro::on_command_deleted(self, generation);
        if let Err(error) = original.check(self) {
            first.get_or_insert(error);
        }
        match command {
            Command::ObjCmd(record) => record.retire_original_delete_procedure(),
            Command::Proc(record) => {
                drop(record.take_for_retirement());
            }
            Command::Alias {
                original_prefix,
                registration_name,
                ..
            }
            | Command::ParentAlias {
                original_prefix,
                registration_name,
                ..
            } => {
                let registration = registration_name.as_ref().and_then(|name| name.take_role());
                drop(registration);
                if let Err(error) = original.check(self) {
                    first.get_or_insert(error);
                }
                let prefix = original_prefix.take_role();
                drop(prefix);
            }
            Command::Ensemble(record) => record.mark_deleted(),
            Command::ChildInterp(record) => {
                record.retire();
                self.retire_pending_children();
            }
            Command::OoObject(owner) => {
                self.oo_command_renamed(*owner, None);
            }
            Command::Builtin(_)
            | Command::Imported { .. }
            | Command::OoMy(_)
            | Command::OoMyClass(_) => {}
        }
        if let Err(error) = original.check(self) {
            first.get_or_insert(error);
        }
        first.map_or(Ok(()), Err)
    }

    fn next_original_import(
        &self,
        origins: &std::collections::HashSet<u64>,
        tokens: &[Rc<crate::ensemble::EnsembleToken>],
    ) -> Option<u64> {
        let visible = self
            .namespaces
            .borrow()
            .imports_for_origins(origins, tokens);
        for (_, identity) in visible {
            if let Some((_, generation)) = self.import_identity_location(&identity) {
                return Some(generation);
            }
        }
        self.hidden.borrow().values().find_map(|binding| {
            let Command::Imported {
                source_generation,
                ensemble,
                ..
            } = &binding.command
            else {
                return None;
            };
            let retains = ensemble
                .as_ref()
                .is_some_and(|current| tokens.iter().any(|token| Rc::ptr_eq(current, token)));
            (origins.contains(source_generation) || retains).then_some(binding.generation)
        })
    }

    fn retire_original_imports(
        &mut self,
        origins: impl IntoIterator<Item = u64>,
        tokens: &[Rc<crate::ensemble::EnsembleToken>],
        original: &OriginalDeletion,
    ) -> Result<(), CmdError> {
        let origins = origins.into_iter().collect();
        let mut first = original.check(self).err();
        while let Some(generation) = self.next_original_import(&origins, tokens) {
            if let Err(error) = self.retire_original_command_generation(generation, first.is_none())
            {
                first.get_or_insert(error);
            }
            if let Err(error) = original.check(self) {
                first.get_or_insert(error);
            }
        }
        first.map_or(Ok(()), Err)
    }

    pub(super) fn retire_original_import_origins(
        &mut self,
        origins: impl IntoIterator<Item = u64>,
        tokens: &[Rc<crate::ensemble::EnsembleToken>],
    ) {
        let original = match OriginalDeletion::capture(self) {
            Ok(original) => original,
            Err(error) => {
                self.report_cmd_error(error);
                return;
            }
        };
        if let Err(error) = self.retire_original_imports(origins, tokens, &original) {
            self.report_cmd_error(error);
        }
    }
}

#[cfg(test)]
#[path = "native_command_retirement/tests.rs"]
mod tests;
