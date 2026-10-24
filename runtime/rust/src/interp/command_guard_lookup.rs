// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Actual selected-publication dependencies for shared command guards.

use super::{GuardDomain, GuardDomains, GuardError, GuardToken, Interp};
use crate::namespace::CommandGuardLookup;

#[cfg(test)]
pub(super) type CommandGuardDiagnostic = (
    &'static str,
    Option<u64>,
    (Option<u64>, Option<u64>),
    (Option<u64>, Option<u64>),
    Option<bool>,
);

#[cfg(test)]
std::thread_local! {
    static COMMAND_GUARD_DIAGNOSTIC: std::cell::Cell<CommandGuardDiagnostic> = const {
        std::cell::Cell::new(("start", None, (None, None), (None, None), None))
    };
}

#[cfg(test)]
pub(super) fn reset_diagnostic() {
    COMMAND_GUARD_DIAGNOSTIC.with(|last| {
        last.set(("start", None, (None, None), (None, None), None));
    });
}

#[cfg(test)]
pub(super) fn remember_sync(
    previous: Option<u64>,
    revision: Option<u64>,
    previous_policy: Option<u64>,
    policy: Option<u64>,
) {
    COMMAND_GUARD_DIAGNOSTIC.with(|last| {
        let mut snapshot = last.get();
        snapshot.2 = (previous, revision);
        snapshot.3 = (previous_policy, policy);
        last.set(snapshot);
    });
}

#[cfg(test)]
pub(super) fn remember_stage(phase: &'static str, generation: Option<u64>, current: Option<bool>) {
    COMMAND_GUARD_DIAGNOSTIC.with(|last| {
        let mut snapshot = last.get();
        snapshot.0 = phase;
        snapshot.1 = generation;
        snapshot.4 = current;
        last.set(snapshot);
    });
}

#[cfg(test)]
pub(super) fn checked_result(generation: u64, current: bool) -> bool {
    remember_stage("kernel", Some(generation), Some(current));
    current
}

#[cfg(test)]
pub(super) fn diagnostic() -> CommandGuardDiagnostic {
    COMMAND_GUARD_DIAGNOSTIC.with(std::cell::Cell::get)
}

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
