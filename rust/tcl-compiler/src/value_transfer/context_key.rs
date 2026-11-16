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

//! `AnalysisContextKey`: the analysis context's hashable identity as a per-function memo key, with the connection-scoped names it covers.

use crate::command_binding::CommandTrustSnapshot;
use tcl_registry::CommandRegistry;
use tcl_registry::value_transfer::AnalysisTier;

/// The analysis context's hashable identity, as a per-function memo key
/// carries it: the facts that can change an answer and that the key's
/// other fields (body, dialect, grammar, traces, seeds) do not already
/// cover. Built once per module and shared by every function in it, so a
/// `rename` anywhere in the module re-keys every function's lattice — the
/// sensitivity the design asks for.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnalysisContextKey {
    /// The generation of the registry the module resolved against
    /// ([`CommandRegistry::generation`]): the exact command surface, so two
    /// builds against different registries never share a lattice.
    pub registry_generation: u64,
    /// The workspace pack overlay that registry carries
    /// ([`CommandRegistry::overlay_generation`]), which the memoised
    /// per-procedure lattice resolves its own registry by.
    pub overlay_generation: Option<u64>,
    /// The module's command-binding evidence.
    pub bindings: CommandTrustSnapshot,
    /// The precision tier the request runs at.
    pub tier: AnalysisTier,
    /// The generation of the building thread's evaluator host
    /// ([`tcl_registry::pack_hooks::evaluator_generation`]): which host
    /// serves the declared implementations, and in what health, so a
    /// host-present and a host-absent worker never share a lattice.
    pub evaluator_revision: u64,
    /// The names an iRules `when` handler of the module may find bound on
    /// entry ([`ConnectionScoped`]); empty for a module with no handler.
    pub connection_scoped: ConnectionScoped,
    /// The names the module's callback scripts write
    /// ([`crate::ir::Module::deferred_writes`]): each is externally mutable
    /// in every function, so a callback edited anywhere in the module
    /// re-keys every function's lattice.
    pub deferred_writes: crate::ir::DeferredWrites,
}

/// The names an iRules `when` handler may find bound on entry
/// (`docs/design/compiler/value-transfers.md` § *Existence*, the entry
/// state): a handler's variables live as long as its connection, so another
/// event — or an earlier firing of the same one — may have bound any name a
/// handler of the module binds.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ConnectionScoped {
    /// Every name a handler of the module binds, and the array each element
    /// among them sits in, sorted and without repeats.
    pub names: Vec<String>,
    /// Whether a handler writes a computed name, so any name may be bound.
    pub any: bool,
}

impl ConnectionScoped {
    /// Whether a handler may find `name` bound on entry.
    #[must_use]
    pub fn holds(&self, name: &str) -> bool {
        self.any
            || self
                .names
                .binary_search_by(|held| held.as_str().cmp(name))
                .is_ok()
    }
}

impl AnalysisContextKey {
    /// The key for a module whose command mutations were scanned, resolved
    /// against `registry`, under the building thread's evaluator
    /// generation.
    #[must_use]
    pub fn for_module(
        mutations: &crate::command_binding::ModuleCommandMutations,
        registry: &CommandRegistry,
    ) -> Self {
        Self {
            registry_generation: registry.generation(),
            overlay_generation: registry.overlay_generation(),
            bindings: mutations.snapshot(),
            tier: AnalysisTier::Deep,
            evaluator_revision: u64::from(tcl_registry::pack_hooks::evaluator_generation().0),
            connection_scoped: ConnectionScoped::default(),
            deferred_writes: crate::ir::DeferredWrites::default(),
        }
    }

    /// The key with the module's connection-scoped names
    /// ([`Self::connection_scoped`]).
    #[must_use]
    pub fn with_connection_scoped(mut self, scoped: ConnectionScoped) -> Self {
        self.connection_scoped = scoped;
        self
    }

    /// The key with the names the module's callback scripts write
    /// ([`Self::deferred_writes`]).
    #[must_use]
    pub fn with_deferred_writes(mut self, writes: crate::ir::DeferredWrites) -> Self {
        self.deferred_writes = writes;
        self
    }

    /// The key for a request at `tier`. Below the deep tier a function
    /// built under the key runs no existence rung, so the driver answers
    /// each existence read `Unavailable(tier)` and so does
    /// [`crate::compilation_unit::FunctionUnit::existence`].
    #[must_use]
    pub fn at_tier(mut self, tier: AnalysisTier) -> Self {
        self.tier = tier;
        self
    }

    /// The key for a consumer with no module view: no mutations observed,
    /// against the process-wide default registry.
    #[must_use]
    pub fn detached() -> Self {
        Self::for_module(
            &crate::command_binding::ModuleCommandMutations::default(),
            tcl_registry::default_registry(),
        )
    }
}
