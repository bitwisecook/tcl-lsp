// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Actual selected-publication dependencies for shared command guards.

use super::{GuardDomain, GuardDomains, GuardError, GuardToken, Interp};
use crate::namespace::CommandGuardLookup;

impl Interp {
    pub(super) fn prepare_command_guard_lookup(
        &self,
        generation: u64,
        domains: GuardDomains,
    ) -> Result<Option<CommandGuardLookup>, GuardError> {
        if domains.contains(GuardDomain::CommandEnvironment) {
            self.namespaces()
                .command_guard_lookup(self.current_ns.get(), generation)
                .map(Some)
                .ok_or(GuardError::IdentityUnavailable)
        } else {
            Ok(None)
        }
    }

    pub(super) fn command_guard_lookup_is_current(
        &self,
        token: GuardToken,
        generation: u64,
    ) -> bool {
        let lookups = self.command_guard_lookups.borrow();
        let Some(original) = lookups.get(&token) else {
            return false;
        };
        original.as_ref().is_none_or(|original| {
            self.namespaces().command_guard_lookup_is_current(
                original,
                self.current_ns.get(),
                generation,
            )
        })
    }

    pub(super) fn invalidate_namespace_structure_guards(&self) {
        let mut guards = self.guards.borrow_mut();
        guards.invalidate(GuardDomain::Namespace);
        guards.invalidate(GuardDomain::UnknownHandling);
    }
}
