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

//! Creating an ensemble command in its selected slot, and reading an ensemble's configuration.

use super::super::{
    native_operation_currency, ImportQueryPurpose, Interp, OriginalCommandPublication,
};
use super::Command;
use crate::namespace::NsId;
use std::rc::Rc;

impl Interp {
    /// Return the actual shared publisher's generation after its cleanup.
    /// Stock installation must never derive this receipt from a later name lookup.
    pub(crate) fn create_ensemble(
        &mut self,
        name: &[u8],
        cfg: crate::ensemble::EnsembleConfig,
    ) -> Option<u64> {
        let (ns, tail) = self
            .namespaces
            .borrow_mut()
            .command_publication_at(self.current_ns.get(), name)
            .expect("stock ensemble has a native publication context");
        self.create_ensemble_in_slot_with_publication(ns, &tail, cfg)
    }

    /// Install one ensemble in its already selected physical command holder.
    pub(crate) fn create_ensemble_in_slot(
        &mut self,
        ns: NsId,
        tail: &[u8],
        cfg: crate::ensemble::EnsembleConfig,
    ) {
        // This void facade retains the existing caller contract; publication
        // refusal is already latched by the same checked worker.
        let _publication = self.create_ensemble_in_slot_with_publication(ns, tail, cfg);
    }

    fn create_ensemble_in_slot_with_publication(
        &mut self,
        ns: NsId,
        tail: &[u8],
        cfg: crate::ensemble::EnsembleConfig,
    ) -> Option<u64> {
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return None;
            }
        };
        let reached = self.create_ensemble_in_slot_entered(ns, tail, cfg, &scope);
        // The entered worker's original token/configuration cleanup has finished.
        // A recreated spelling cannot become this publication's receipt.
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            self.refuse_native_execution(first);
            return None;
        }
        let generation = reached?;
        self.original_command_publication_is_current(ns, tail, generation)
            .then_some(generation)
    }

    fn create_ensemble_in_slot_entered(
        &mut self,
        ns: NsId,
        tail: &[u8],
        mut cfg: crate::ensemble::EnsembleConfig,
        scope: &native_operation_currency::NativeOperationScope,
    ) -> Option<u64> {
        let Some(holder) = self.native_namespace_token_at(ns) else {
            self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original ensemble publication namespace",
                ),
            );
            return None;
        };
        let holder_lifecycle = holder.lifecycle();
        let Some(configuration_holder) = self.native_namespace_token_at(cfg.ns) else {
            self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original ensemble configuration namespace",
                ),
            );
            return None;
        };
        let configuration_namespace = cfg.ns;
        let configuration_lifecycle = configuration_holder.lifecycle();
        self.prepare_original_ensemble_configuration(&mut cfg);
        if scope.currency().ensure_current_or_refuse().is_err()
            || !self.original_publication_namespace_is_current(ns, &holder, holder_lifecycle)
            || !self.original_publication_namespace_is_current(
                configuration_namespace,
                &configuration_holder,
                configuration_lifecycle,
            )
        {
            return None;
        }
        let fqn = self.namespaces.borrow().command_fqn_at(ns, tail);
        let displaced = self.namespaces.borrow().command_in(ns, tail);
        let had_displaced = displaced.is_some();
        let old_token = match displaced.as_ref() {
            Some(Command::Ensemble(token)) => Some(Rc::clone(token)),
            _ => None,
        };
        // Only the actual ensemble identity is needed below. A generic command
        // transport must not protect an idle extension from the shared delete.
        drop(displaced);
        let new_token = Rc::new(
            crate::ensemble::EnsembleToken::with_configuration_retirement(
                cfg,
                fqn.clone(),
                tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Absent,
                crate::ensemble::retire_configuration,
            ),
        );

        // Creating an ensemble at an occupied name is command replacement, not
        // in-place token mutation. The old token must become dead (an active
        // unknown callback observes UNKNOWN_DELETED), while imports of the
        // occupied source binding are explicitly reattached to the new token.
        if let Some(old_token) = old_token.as_ref() {
            self.retire_ensemble_replacement_identity(old_token);
        }
        if scope.currency().ensure_current_or_refuse().is_err()
            || !self.original_publication_namespace_is_current(ns, &holder, holder_lifecycle)
            || !self.original_publication_namespace_is_current(
                configuration_namespace,
                &configuration_holder,
                configuration_lifecycle,
            )
        {
            return None;
        }
        let OriginalCommandPublication::Published(new_generation) = self
            .bind_command_replacement_with_jim_key(
                ns,
                tail,
                Command::Ensemble(Rc::clone(&new_token)),
                None,
            )
        else {
            return None;
        };
        if scope.currency().ensure_current_or_refuse().is_err()
            || !self.original_publication_namespace_is_current(ns, &holder, holder_lifecycle)
            || !self.original_publication_namespace_is_current(
                configuration_namespace,
                &configuration_holder,
                configuration_lifecycle,
            )
        {
            return None;
        }
        new_token.config().originals.activate();
        self.namespaces
            .borrow_mut()
            .advance_ensemble_export_epoch(new_token.config().ns);
        let new_fqn = self.namespaces.borrow().command_fqn_at(ns, tail);
        if had_displaced {
            self.reattach_missing_import_sources(&new_fqn, new_generation);
        }
        self.retarget_imports_to_ensemble(new_generation, &new_token);
        Some(new_generation)
    }

    /// Whether `name` resolves to an ensemble command (`namespace ensemble
    /// exists`).
    pub(crate) fn is_ensemble(&self, name: &[u8]) -> bool {
        self.ensemble_config_at(name).is_some()
    }

    /// The configuration of the ensemble command `name` resolves to (or `None`
    /// if `name` is not an ensemble), plus the fully-qualified name of the
    /// command that actually **owns** that config.
    ///
    /// `namespace import` follows the source's retained ensemble configuration.
    /// Configuring through an import changes the same live token observed by
    /// the origin; the import remains a separate binding and namespace origin
    /// follows its actual command-generation chain.
    pub(crate) fn ensemble_config_at(
        &self,
        name: &[u8],
    ) -> Option<Rc<crate::ensemble::EnsembleToken>> {
        let target = self.resolve_import_query(
            self.import_query_binding(name)?,
            ImportQueryPurpose::Introspection,
        )?;
        if let Some(ensemble) = target.retained_ensemble {
            return Some(ensemble);
        }
        match target.command {
            Command::Ensemble(token) => Some(token),
            _ => None,
        }
    }
}
