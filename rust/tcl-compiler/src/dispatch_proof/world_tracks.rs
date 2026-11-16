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

//! The versioned partitions of the tracked world and the value-numbering token of one track.

use tcl_registry::{DispatchDependencyDomain, WorldStateDomain};

/// One versioned partition of the tracked world, at dispatch-proof
/// granularity.
///
/// Tracks deliberately mirror [`WorldRegionKind`] (minus the per-resource
/// split of external state) so that world-state SSA, effect footprints, and
/// this pass name the same partitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorldTrack {
    /// Name-to-command bindings in the current interpreter.
    CommandBindings,
    /// Namespace lookup, import, export, ensemble, and path state.
    NamespaceLookup,
    /// Namespace-specific `unknown` handler state.
    NamespaceUnknown,
    /// Command execution (enter/leave/step) trace registrations.
    ExecutionTraces,
    /// Command rename/delete trace registrations.
    CommandTraces,
    /// Variable trace registrations.
    VariableTraces,
    /// `TclOO` class, object, and method dispatch state.
    ObjectDispatch,
    /// Safe-interpreter and hidden-command policy of the current interpreter.
    InterpreterPolicy,
    /// Current interpreter result storage.
    InterpreterResult,
    /// Current interpreter completion metadata.
    CompletionState,
    /// Child-interpreter existence and identity.
    InterpreterTopology,
    /// Tcl variable cells.
    VariableStore,
    /// Package-loading state.
    PackageState,
    /// Host-provided capability state.
    HostCapabilities,
    /// Externally visible resources retained from the legacy effect model.
    ExternalState,
}

impl WorldTrack {
    /// Every track, in deterministic declaration order.
    pub const ALL: [Self; 15] = [
        Self::CommandBindings,
        Self::NamespaceLookup,
        Self::NamespaceUnknown,
        Self::ExecutionTraces,
        Self::CommandTraces,
        Self::VariableTraces,
        Self::ObjectDispatch,
        Self::InterpreterPolicy,
        Self::InterpreterTopology,
        Self::VariableStore,
        Self::PackageState,
        Self::HostCapabilities,
        Self::ExternalState,
        Self::InterpreterResult,
        Self::CompletionState,
    ];

    pub(super) const fn index(self) -> usize {
        match self {
            Self::CommandBindings => 0,
            Self::NamespaceLookup => 1,
            Self::NamespaceUnknown => 2,
            Self::ExecutionTraces => 3,
            Self::CommandTraces => 4,
            Self::VariableTraces => 5,
            Self::ObjectDispatch => 6,
            Self::InterpreterPolicy => 7,
            Self::InterpreterTopology => 8,
            Self::VariableStore => 9,
            Self::PackageState => 10,
            Self::HostCapabilities => 11,
            Self::ExternalState => 12,
            Self::InterpreterResult => 13,
            Self::CompletionState => 14,
        }
    }

    /// Stable label used when a track version participates in a value key.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::CommandBindings => "cmdbind",
            Self::NamespaceLookup => "nslookup",
            Self::NamespaceUnknown => "nsunknown",
            Self::ExecutionTraces => "exectrace",
            Self::CommandTraces => "cmdtrace",
            Self::VariableTraces => "vartrace",
            Self::ObjectDispatch => "oodispatch",
            Self::InterpreterPolicy => "interppolicy",
            Self::InterpreterTopology => "interptopo",
            Self::VariableStore => "varstore",
            Self::PackageState => "package",
            Self::HostCapabilities => "host",
            Self::ExternalState => "external",
            Self::InterpreterResult => "interpresult",
            Self::CompletionState => "completion",
        }
    }

    /// Project a registry versioned-world domain onto its track.
    #[must_use]
    pub const fn from_world_state_domain(domain: WorldStateDomain) -> Self {
        match domain {
            WorldStateDomain::InterpreterTopology => Self::InterpreterTopology,
            WorldStateDomain::CommandBindings => Self::CommandBindings,
            WorldStateDomain::NamespaceLookup => Self::NamespaceLookup,
            WorldStateDomain::NamespaceUnknown => Self::NamespaceUnknown,
            WorldStateDomain::ExecutionTraces => Self::ExecutionTraces,
            WorldStateDomain::VariableTraces => Self::VariableTraces,
            WorldStateDomain::CommandTraces => Self::CommandTraces,
            WorldStateDomain::OoDispatch => Self::ObjectDispatch,
            WorldStateDomain::InterpreterPolicy => Self::InterpreterPolicy,
            WorldStateDomain::InterpreterResult => Self::InterpreterResult,
            WorldStateDomain::CompletionState => Self::CompletionState,
            WorldStateDomain::PackageState => Self::PackageState,
            WorldStateDomain::HostCapabilities => Self::HostCapabilities,
            WorldStateDomain::VariableStore => Self::VariableStore,
            WorldStateDomain::LegacyExternal(_) => Self::ExternalState,
        }
    }

    /// The tracks whose versions key one dispatch dependency domain.
    #[must_use]
    pub const fn for_dispatch_dependency(domain: DispatchDependencyDomain) -> &'static [Self] {
        match domain {
            DispatchDependencyDomain::CommandBinding => &[Self::CommandBindings],
            DispatchDependencyDomain::NamespaceLookup => &[Self::NamespaceLookup],
            DispatchDependencyDomain::CommandTraces => {
                &[Self::ExecutionTraces, Self::CommandTraces]
            }
            DispatchDependencyDomain::InterpreterPolicy => {
                &[Self::InterpreterPolicy, Self::InterpreterTopology]
            }
            DispatchDependencyDomain::ObjectDispatch => &[Self::ObjectDispatch],
            DispatchDependencyDomain::UnknownHandling => {
                &[Self::NamespaceUnknown, Self::CommandBindings]
            }
        }
    }
}

/// A value-numbering token for one track: equal tokens prove equal contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrackVersion {
    /// Unchanged since function entry.
    Entry,
    /// Last changed by the instruction at the given coordinates.
    Site {
        /// Executable block index.
        block: u32,
        /// Instruction index within the block.
        instruction: u32,
    },
    /// Joined from differing predecessor versions at the given block.
    Phi {
        /// Executable block index owning the join.
        block: u32,
    },
}

impl TrackVersion {
    pub(super) fn render(self, out: &mut String) {
        use std::fmt::Write as _;
        match self {
            Self::Entry => out.push('e'),
            Self::Site { block, instruction } => {
                let _ = write!(out, "b{block}i{instruction}");
            }
            Self::Phi { block } => {
                let _ = write!(out, "p{block}");
            }
        }
    }
}
