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

//! Command guards: attested intrinsic identities, the backing report, preparing, checking and releasing a guard, and executing an intrinsic.

use super::super::{Vm, ok};
use super::CommandSidecarKey;
use crate::command::Command;
use crate::value::Value;
use std::collections::BTreeSet;
use tcl_runtime_api::guard::{GuardDomain, GuardDomains, GuardError, GuardIdentity, GuardToken};
use tcl_runtime_api::{Completion, Namespaces, RegisteredBacking};

impl Vm {
    /// The one writer of the attestation table: attest `identities` for the
    /// command token `generation` and drop the entry of the token it displaced.
    /// What is already attested for `generation` stays.
    pub(in super::super) fn attest(
        &self,
        generation: u64,
        displaced: Option<u64>,
        identities: BTreeSet<GuardIdentity>,
    ) {
        let mut attested = self.guarded_commands.attested.borrow_mut();
        if let Some(displaced) = displaced {
            attested.remove(&displaced);
        }
        if !identities.is_empty() {
            attested.entry(generation).or_default().extend(identities);
        }
    }

    /// Attach the registry's intrinsic identities to the builtins this VM
    /// registered, from the generation it is pinned to.
    ///
    /// The sweep reads that generation's shipped store and nothing an overlay
    /// installed: a pack's command is not a runtime implementation, and only
    /// the runtime may attest. A name is attested only while the command bound
    /// at it is still the builtin registered there, so a name a script has
    /// since redefined, or renamed another builtin over, gains no attestation
    /// for that command, and a builtin moved to another name keeps the one it
    /// already has (the table is keyed by the command token's generation, which
    /// a rename carries). It runs once, at the end of registration: the table
    /// survives the profile pin, and a later sweep would attest whatever an
    /// embedder registered at a registry name as the registry's command.
    pub(crate) fn attach_identities(&mut self) {
        let registry = crate::environment::store_for_profile(self.source_profile());
        self.attach_identities_from(registry);
    }

    /// The sweep over one generation's store. Only [`Self::attach_identities`]
    /// chooses the store in production: the generation the VM is pinned to.
    pub(in super::super) fn attach_identities_from(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
    ) {
        for name in registry.command_names() {
            let Some(spec) = registry.get_exact(name) else {
                continue;
            };
            let identities: BTreeSet<GuardIdentity> = spec
                .intrinsic_ids()
                .into_iter()
                .flat_map(|id| {
                    id.guard_semantics_variants().iter().map(move |semantics| {
                        GuardIdentity::registry_intrinsic_with_semantics(id.stable_id(), *semantics)
                    })
                })
                .collect();
            if identities.is_empty() {
                continue;
            }
            let key = tcl_syntax::naming::unroot_rooted_key(spec.name).unwrap_or(spec.name);
            if !matches!(self.commands.get(key), Some(Command::Builtin(_)))
                || self.builtin_identity_for_key(key).is_none_or(|identity| {
                    tcl_syntax::naming::unroot_rooted_key(&identity).unwrap_or(&identity) != key
                })
            {
                continue;
            }
            let Some(generation) = self.visible_command_generation(key) else {
                continue;
            };
            self.attest(generation, None, identities);
        }
    }

    /// What this VM's command table backs, by command name: the runtime's own
    /// answer to "what did you register", which a spec's `runtime_backing`
    /// declaration is held to.
    ///
    /// Every visible command is classified by what it is: a native handler
    /// (the engine's own or an embedder's) or an engine-installed `TclOO`
    /// root. A command a script defines is no backing. This VM embeds no Tcl
    /// library, so nothing is reported as defined by one. Names are without a
    /// leading `::`. This Unicode catalogue omits opaque byte names. Private
    /// storage keys never become public command identities; counted lookup and
    /// publication retain their exact names independently.
    #[must_use]
    pub fn backing_report(&self) -> Vec<(String, RegisteredBacking)> {
        let mut report: Vec<(String, RegisteredBacking)> = self
            .commands
            .iter()
            .filter_map(|(key, command)| {
                let backing = match command {
                    Command::Builtin(_) | Command::Native(_) => RegisteredBacking::Builtin,
                    Command::Object(_) if self.registry_object_roots.contains_key(key) => {
                        RegisteredBacking::Object
                    }
                    _ => return None,
                };
                let display = self.command_display_key_bytes(key);
                Some((display.try_utf8().ok()?.to_owned(), backing))
            })
            .collect();
        report.sort();
        report
    }

    /// The identities attested for the command `name` resolves to from the
    /// current namespace. The name is resolved afresh, so a command that was
    /// replaced, renamed away, hidden, or is no longer admitted by the command
    /// surface answers `None`, and one restored by `rename` or `expose` answers
    /// what it did before.
    pub(in super::super) fn attested_identities(
        &self,
        name: &str,
    ) -> Option<BTreeSet<GuardIdentity>> {
        let key = self.resolve_command_fqn(self.current_ns(), name)?;
        let generation = self.visible_command_generation(&key)?;
        self.guarded_commands
            .attested
            .borrow()
            .get(&generation)
            .cloned()
    }

    /// Derive stock semantics only from live bootstrap registrations. An
    /// ensemble additionally needs its original map, namespace and the mapped
    /// worker owning each identity; descriptors alone prove none of these.
    fn stock_command_guard_identities(
        &self,
        key: &str,
        seen: &mut std::collections::HashSet<String>,
    ) -> Option<BTreeSet<GuardIdentity>> {
        if !seen.insert(key.to_owned())
            || self
                .command_token_identity(&CommandSidecarKey::visible(key))
                .is_none()
            || self.command_has_execution_trace(&CommandSidecarKey::visible(key))
        {
            return None;
        }
        let dialect = self.native_invocation_dialect();
        let characters = dialect.characters?;
        let registry = crate::environment::store_for_profile(self.native_execution_profile());
        let identities = match self.commands.get(key)? {
            Command::Builtin(_) => {
                let registration = self.builtin_identities.get(key)?;
                registry
                    .native_registration_intrinsic_ids(registration, dialect)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|id| {
                        GuardIdentity::registry_intrinsic_with_semantics(
                            id.stable_id(),
                            id.guard_semantics_key_for_characters(characters),
                        )
                    })
                    .collect()
            }
            Command::Ensemble(token) if !token.is_deleted() => {
                let config = token.config();
                let native = config.native_implementation()?;
                if !Namespaces::namespace_is_live(self, native.namespace) {
                    return None;
                }
                let mut identities = BTreeSet::new();
                for (_, prefix) in &native.map {
                    let [target] = prefix.as_slice() else {
                        return None;
                    };
                    let Some(target_key) = self.resolve_command_fqn("", target) else {
                        continue;
                    };
                    if self.builtin_identity_for_key(&target_key).as_deref()
                        != Some(tcl_syntax::naming::unroot_rooted_key(target).unwrap_or(target))
                    {
                        continue;
                    }
                    if let Some(worker) = self.stock_command_guard_identities(&target_key, seen) {
                        identities.extend(worker);
                    }
                }
                identities
            }
            _ => return None,
        };
        seen.remove(key);
        Some(identities)
    }

    fn live_command_guard_identities(&self, key: &str) -> BTreeSet<GuardIdentity> {
        let mut identities = self
            .visible_command_generation(key)
            .and_then(|generation| {
                self.guarded_commands
                    .attested
                    .borrow()
                    .get(&generation)
                    .cloned()
            })
            .unwrap_or_default();
        if let Some(stock) =
            self.stock_command_guard_identities(key, &mut std::collections::HashSet::new())
        {
            identities.extend(stock);
        }
        identities
    }

    /// Verify the live command identity and snapshot the requested mutation
    /// domains. Any active trace in a requested trace domain conservatively
    /// refuses issuance; an epoch snapshot is not an absence proof.
    ///
    /// A request for a registry intrinsic must cover the domains its family
    /// requires ([`tcl_registry::IntrinsicId::family`]) whatever the caller
    /// asked for: a Family-B member reaches the variable store, so its guard is
    /// refused while a script variable trace exists and stales when one is
    /// added. The engine's bounded native hooks (`env`, `tcl_precision`, the
    /// error variables) run in the store itself and do not refuse it.
    pub fn prepare_command_guard(
        &self,
        name: &str,
        expected: GuardIdentity,
        domains: GuardDomains,
    ) -> Result<GuardToken, GuardError> {
        if domains.is_empty() {
            return Err(GuardError::EmptyDomains);
        }
        self.synchronise_unknown_binding_guards();
        let key = self
            .resolve_command_fqn(self.current_ns(), name)
            .ok_or(GuardError::IdentityUnavailable)?;
        let owner = self
            .command_token_identity(&CommandSidecarKey::visible(&key))
            .ok_or(GuardError::IdentityUnavailable)?;
        let stock = self
            .stock_command_guard_identities(&key, &mut std::collections::HashSet::new())
            .unwrap_or_default();
        // Actual stock selection depends on the original root, worker map and
        // absence of traces. Explicit host contracts retain their requested
        // domains, even when they request a registry intrinsic semantic ID.
        let domains = if stock.contains(&expected) {
            domains
                .with(GuardDomain::CommandEnvironment)
                .with(GuardDomain::Namespace)
                .with(GuardDomain::CommandTrace)
        } else {
            domains
        };
        if !domains.covers(tcl_registry::IntrinsicId::required_guard_domains(expected)) {
            return Err(GuardError::DomainsInsufficient);
        }
        if (domains.contains(GuardDomain::CommandTrace)
            && !(self.cmd_traces.is_empty() && self.exec_traces.is_empty()))
            || (domains.contains(GuardDomain::VariableTrace)
                && !self.variable_observers.script_traces.is_empty())
        {
            return Err(GuardError::PrerequisiteUnsatisfied);
        }
        let mut identities = self.attested_identities(name).unwrap_or_default();
        identities.extend(stock);
        let observed = if identities.contains(&expected) {
            Some(expected)
        } else {
            identities.first().copied()
        };
        self.guards
            .borrow_mut()
            .prepare(expected, observed, domains, owner)
    }

    /// Move `UnknownHandling` when the `::unknown` binding differs from the
    /// one the last guard door saw: a definition, rename or deletion of the
    /// default handler changes what an unresolved name runs.
    pub(in super::super) fn synchronise_unknown_binding_guards(&self) {
        let unknown = self.command_token_identity(&CommandSidecarKey::visible("unknown"));
        if self
            .guard_unknown_binding
            .replace(Some(unknown.clone()))
            .is_some_and(|seen| seen != unknown)
        {
            self.guards
                .borrow_mut()
                .invalidate(GuardDomain::UnknownHandling);
        }
    }

    /// Re-check a command guard against current live identity and epochs.
    #[must_use]
    pub fn check_command_guard(&self, token: GuardToken, name: &str) -> bool {
        self.synchronise_unknown_binding_guards();
        let Some(key) = self.resolve_command_fqn(self.current_ns(), name) else {
            return false;
        };
        let Some(owner) = self.command_token_identity(&CommandSidecarKey::visible(&key)) else {
            return false;
        };
        let identities = self.live_command_guard_identities(&key);
        identities
            .iter()
            .any(|identity| self.guards.borrow().check(token, Some(*identity), &owner))
    }

    /// Execute one target-neutral intrinsic over arguments after command and
    /// subcommand dispatch. `None` means this runtime does not implement the ID
    /// or invocation shape and the caller must use its slow path.
    pub fn execute_intrinsic(
        &mut self,
        intrinsic: tcl_registry::IntrinsicId,
        args: &[Value],
    ) -> Option<Completion<Value>> {
        match (intrinsic, args) {
            (tcl_registry::IntrinsicId::StringLength, [value]) => {
                Some(match tcl_cmd_core::string::length(self, value) {
                    Ok(value) => ok(value),
                    Err(error) => crate::command::completion_from_cmd_error(self, error),
                })
            }
            _ => None,
        }
    }

    /// Release one command guard token.
    #[must_use]
    pub fn release_command_guard(&self, token: GuardToken) -> bool {
        self.guards.borrow_mut().release(token)
    }

    /// Stable registry identity for a visible builtin or embedder-native
    /// command, if any. Actual stock registrations and moved commands carry
    /// their identity in [`Self::builtin_identities`]. Compiler binding
    /// validation separately accepts only [`Command::Builtin`], so a native
    /// replacement at a registry name never inherits compiler trust.
    pub(in super::super) fn builtin_identity_for_key(&self, key: &str) -> Option<String> {
        if let Some(Command::Ensemble(token)) = self.commands.get(key) {
            return token
                .config()
                .native_implementation()
                .map(|native| native.identity.to_owned());
        }
        matches!(
            self.commands.get(key),
            Some(Command::Builtin(_) | Command::Native(_))
        )
        .then(|| self.builtin_identities.get(key).cloned())
        .flatten()
    }

    pub(crate) fn stock_native_identity(&self, name: &str) -> Option<String> {
        self.builtin_identity_for_key(tcl_syntax::naming::unroot_rooted_key(name).unwrap_or(name))
    }

    /// Whether the builtin bound at `key` is an embedder's handler
    /// ([`Self::register_guarded_builtin`]) and not one this VM registered.
    pub(in super::super) fn is_host_builtin(&self, key: &str) -> bool {
        self.visible_command_generation(key)
            .is_some_and(|generation| self.guarded_commands.host.contains(&generation))
    }
}
