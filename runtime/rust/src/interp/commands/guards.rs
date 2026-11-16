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

//! Command guards: issuing, re-checking and releasing a guard, executing an intrinsic under one, and invalidating guard domains.

use super::super::{Code, Interp};
#[cfg(test)]
use crate::interp::command_guard_lookup;
use crate::obj::TclObj;
use tcl_runtime_api::guard::{GuardDomain, GuardDomains, GuardError, GuardIdentity, GuardToken};

impl Interp {
    /// Verify live command identity and issue a guard over `domains`.
    ///
    /// A request for a registry intrinsic must cover the domains its family
    /// requires ([`tcl_registry::IntrinsicId::family`]) whatever the caller
    /// asked for: a Family-B member reaches the variable store, so its guard is
    /// refused while a variable trace exists and stales when one is added.
    pub fn prepare_command_guard(
        &self,
        name: &[u8],
        expected: GuardIdentity,
        domains: GuardDomains,
    ) -> Result<GuardToken, GuardError> {
        self.synchronise_command_environment_guards();
        if !domains.covers(tcl_registry::IntrinsicId::required_guard_domains(expected)) {
            return Err(GuardError::DomainsInsufficient);
        }
        let traces = self.traces.borrow();
        if (domains.contains(GuardDomain::CommandTrace) && !traces.cmd_traces.is_empty())
            || (domains.contains(GuardDomain::VariableTrace) && !traces.traces.is_empty())
        {
            return Err(GuardError::PrerequisiteUnsatisfied);
        }
        drop(traces);
        let (generation, identities) = self
            .attested_command_identity(name)
            .ok_or(GuardError::IdentityUnavailable)?;
        let observed = if identities.contains(&expected) {
            Some(expected)
        } else {
            identities.first().copied()
        };
        let token = self
            .guards
            .borrow_mut()
            .prepare(expected, observed, domains, generation)?;
        let lookup = match self.prepare_command_guard_lookup(generation, domains) {
            Ok(lookup) => lookup,
            Err(error) => {
                self.guards.borrow_mut().release(token);
                return Err(error);
            }
        };
        if let Some(retained) = self.guards.borrow_mut().data_mut(token) {
            *retained = lookup;
        }
        Ok(token)
    }

    /// Re-check a guard against the current resolved implementation identity.
    #[must_use]
    pub fn check_command_guard(&self, token: GuardToken, name: &[u8]) -> bool {
        #[cfg(test)]
        command_guard_lookup::reset_diagnostic();
        self.synchronise_command_environment_guards();
        let Some((generation, identities)) = self.attested_command_identity(name) else {
            #[cfg(test)]
            command_guard_lookup::remember_stage("attestation", None, None);
            return false;
        };
        if !self.command_guard_lookup_is_current(token, generation) {
            #[cfg(test)]
            command_guard_lookup::remember_stage("lookup", Some(generation), None);
            return false;
        }
        #[cfg(test)]
        {
            command_guard_lookup::checked_result(
                generation,
                identities.iter().any(|identity| {
                    self.guards
                        .borrow()
                        .check(token, Some(*identity), &generation)
                }),
            )
        }
        #[cfg(not(test))]
        {
            identities.iter().any(|identity| {
                self.guards
                    .borrow()
                    .check(token, Some(*identity), &generation)
            })
        }
    }

    /// Re-check a guard for one exact registry intrinsic identity.
    ///
    /// This is the current-interpreter boundary used by generated code: it
    /// refuses a token minted for another intrinsic form even when both forms
    /// share one command head.
    #[must_use]
    pub fn check_command_guard_identity(
        &self,
        token: GuardToken,
        name: &[u8],
        expected: GuardIdentity,
    ) -> bool {
        self.synchronise_command_environment_guards();
        let Some((generation, identities)) = self.attested_command_identity(name) else {
            return false;
        };
        if !self.command_guard_lookup_is_current(token, generation) {
            return false;
        }
        if !identities.contains(&expected) {
            return false;
        }
        self.guards
            .borrow()
            .check_expected(token, expected, Some(expected), &generation)
    }

    /// Execute one registry intrinsic over arguments after command and
    /// subcommand dispatch. `None` declines to the caller's slow path.
    pub fn execute_intrinsic(
        &mut self,
        intrinsic: tcl_registry::IntrinsicId,
        args: &[*mut TclObj],
    ) -> Option<Code> {
        match (intrinsic, args) {
            (tcl_registry::IntrinsicId::StringLength, [value]) => {
                Some(match tcl_cmd_core::string::length(self, value) {
                    Ok(result) => {
                        self.set_result(result);
                        Code::Ok
                    }
                    Err(error) => self.report_cmd_error(error),
                })
            }
            _ => None,
        }
    }

    /// Release one runtime guard token exactly once.
    #[must_use]
    pub fn release_command_guard(&self, token: GuardToken) -> bool {
        self.guards.borrow_mut().release(token)
    }

    pub(crate) fn invalidate_guard_domain(&self, domain: GuardDomain) {
        if matches!(
            domain,
            GuardDomain::CommandTrace | GuardDomain::VariableTrace | GuardDomain::Interpreter
        ) {
            self.invalidate_ordinary_entry_observation();
        }
        if domain == GuardDomain::VariableTrace {
            // Every change to the variable-trace set already funnels through
            // here, so this is the one place the per-cell trace bit's epoch
            // needs to move.
            self.var_trace_epoch
                .set(self.var_trace_epoch.get().wrapping_add(1).max(1));
        }
        self.guards.borrow_mut().invalidate(domain);
    }

    /// Invalidate speculative assumptions about this interpreter's visibility,
    /// safety, topology/lifecycle, capability, and execution policy.
    ///
    /// Keep every write to the corresponding private [`InterpState`] fields
    /// behind a method that calls this owner. The epoch is deliberately broader
    /// than any one intrinsic needs: registry dispatch dependencies describe
    /// interpreter policy as an irreducible live-runtime fact.
    pub(in super::super) fn invalidate_interpreter_policy(&self) {
        self.invalidate_guard_domain(GuardDomain::Interpreter);
    }

    pub(in super::super) fn retire_guarded_command_identities(&self) {
        self.guarded_commands.borrow_mut().retain(|generation, _| {
            self.raw_command_location_by_generation(*generation)
                .is_some()
        });
    }

    /// Invalidate the guard domains that depend on the command lookup
    /// environment: namespace structure and lifecycle, and interpreter topology.
    ///
    /// Ordinary command mutations are checked by resolving the guarded name
    /// to its actual generation and semantic attestation. Renaming that same
    /// token preserves its guard; replacing, deleting, hiding or resolving a
    /// different token refuses it without invalidating unrelated commands.
    pub(in super::super) fn invalidate_command_environment(&self) {
        let mut guards = self.guards.borrow_mut();
        guards.invalidate(GuardDomain::CommandEnvironment);
        guards.invalidate(GuardDomain::Namespace);
        guards.invalidate(GuardDomain::UnknownHandling);
    }
}
