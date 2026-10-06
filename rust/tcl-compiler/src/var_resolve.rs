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

//! Variable canonicalisation / resolution — syntactic ref → canonical
//! [`Place`].
//!
//! The variable analogue of lowering's command canonicalisation: given a
//! variable reference as written (`x`, `a(k)`, `$x`, `${tok}(k)`,
//! `$state($whom)`) plus the per-frame scope context (current namespace, the
//! `global` / `variable` / `upvar` declarations in effect, active traces),
//! produce a canonical [`Place`].
//!
//! Builds on the shared resolution substrate — [`split_array_name`] /
//! [`normalise_qualified_name`] ([`crate::naming`]) and the variable-reference
//! scanner ([`crate::var_refs`]) — rather than re-deriving it.  Anything that
//! cannot be pinned down statically (dynamic variable name, computed array
//! name, `upvar` to a dynamic target) resolves to `UNKNOWN` / `dynamic` so the
//! [`overlap`](crate::place::overlap) relation stays sound (over-approximating).

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use tcl_registry::{CommandRegistry, FrameLevel};

use crate::naming::split_array_name;
use crate::place::{
    self, CellGeneration, CellIdentity, CellOwner, Index, IndexKind, Place, PlaceKind,
};
use crate::var_refs::{VarReferenceScanner, VarScanOptions};
mod cell_key;
mod namespace_identity;
pub use cell_key::{
    VariableCellKey, VariableCellKeyQuery, VariableCellSet, VariableCellTable,
    VariableNamespaceMembership, VariableNamespaceQuery, VariableNamespaceSet, namespace_contains,
};

/// Actual variable activation selected by executable source interpretation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub enum VariableExecutionFrame {
    /// Interpreter global frame, including uplevel #0.
    Global,
    /// Namespace-eval activation; Jim keeps its ordinary variables local.
    Namespace(String),
    /// Namespace-eval activation with a distinct executable source site.
    NamespaceActivation {
        /// Selected namespace table.
        namespace: String,
        /// Stable allocation site, distinct from its caller and sibling sites.
        identity: String,
    },
    /// Procedure or event activation with a defining namespace and identity.
    Procedure {
        /// Defining command namespace.
        namespace: String,
        /// Source execution identity of the activation.
        identity: String,
    },
    /// Receiver method activation whose local table is known independently of
    /// the receiver's namespace and instance storage.
    ReceiverMethod {
        /// Stable execution identity of this local activation.
        identity: String,
    },
    /// A foreign stack activation, whose namespace may be unavailable.
    Selected {
        /// Validated caller/absolute frame selector.
        selector: tcl_registry::FrameLevel,
        /// Proven selected frame namespace, when available.
        namespace: Option<String>,
    },
    /// Exact namespace identity retained independently of an activation layout.
    NamespaceIdentity {
        /// Original namespace incarnation and component boundaries.
        namespace: crate::command_binding::SourceNamespaceKey,
        /// Selected activation layout; display fields carry no lookup authority.
        frame: Box<VariableExecutionFrame>,
    },
    /// Neither the selected frame nor its namespace is known.
    #[default]
    Unknown,
}

impl VariableExecutionFrame {
    /// Attach an original namespace identity without changing frame selection.
    #[must_use]
    pub fn with_namespace_identity(
        self,
        namespace: crate::command_binding::SourceNamespaceKey,
    ) -> Self {
        if matches!(
            namespace,
            crate::command_binding::SourceNamespaceKey::Authored(_)
        ) {
            self.into_layout()
        } else {
            Self::NamespaceIdentity {
                namespace,
                frame: Box::new(self.into_layout()),
            }
        }
    }

    fn into_layout(self) -> Self {
        match self {
            Self::NamespaceIdentity { frame, .. } => frame.into_layout(),
            frame => frame,
        }
    }

    /// Activation layout, excluding its independent namespace identity carrier.
    #[must_use]
    pub fn layout(&self) -> &Self {
        match self {
            Self::NamespaceIdentity { frame, .. } => frame.layout(),
            frame => frame,
        }
    }

    /// Original namespace identity; authored display-only frames return none.
    #[must_use]
    pub fn namespace_identity(&self) -> Option<&crate::command_binding::SourceNamespaceKey> {
        match self {
            Self::NamespaceIdentity { namespace, .. } => Some(namespace),
            _ => None,
        }
    }
}

/// Namespace-cell allocation facts, independent of whether a value exists.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NamespaceCellPresence {
    /// Cells proved allocated on every reaching path.
    pub present: VariableCellSet,
    /// Cells which may still be allocated, including undefined aliased cells.
    pub possible: VariableCellSet,
    /// Fresh namespace tables contain only the enumerated baseline and writes.
    pub closed: bool,
    /// Independently captured complete tables, retaining exact incarnations.
    pub closed_namespaces: VariableNamespaceSet,
}

impl NamespaceCellPresence {
    /// Join allocation facts, reporting whether any retained fact changed.
    pub fn join(&mut self, other: &Self) -> bool {
        let before = (
            self.present.len(),
            self.possible.len(),
            self.closed,
            self.closed_namespaces.len(),
        );
        self.possible
            .extend(self.present.symmetric_difference(&other.present).cloned());
        self.possible.extend(other.possible.iter().cloned());
        self.present.retain(|cell| other.present.contains(cell));
        self.closed_namespaces
            .retain(|namespace| other.closed_namespaces.contains(namespace));
        self.closed &= other.closed;
        before
            != (
                self.present.len(),
                self.possible.len(),
                self.closed,
                self.closed_namespaces.len(),
            )
    }

    /// Reentrant Tcl may remove, create or retarget namespace cells.
    pub fn widen(&mut self) {
        self.possible.extend(self.present.drain());
        self.closed = false;
        self.closed_namespaces.clear();
    }

    /// Prove a candidate absent only with a closed allocation surface.
    #[must_use]
    pub fn absent<Q: VariableCellKeyQuery + ?Sized>(&self, cell: &Q) -> bool {
        (self.closed || self.captured_table_absent(cell))
            && !self.present.contains(cell)
            && !self.possible.contains(cell)
    }

    /// Absence from an independently captured table, without assuming fresh
    /// incoming contents in other tables or following an existing link.
    #[must_use]
    pub fn captured_table_absent<Q: VariableCellKeyQuery + ?Sized>(&self, cell: &Q) -> bool {
        let key = cell.variable_cell_key();
        matches!(key.root(), VariableCellKey::Namespace { identity, .. }
            if self.closed_namespaces.contains(identity))
            && !self.present.contains(key.root())
            && !self.possible.contains(key.root())
    }
}

/// Variable table selected by the current activation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VariableFrameKind {
    /// Procedure or dialect-local namespace activation.
    #[default]
    Local,
    /// C Tcl namespace variable table.
    Namespace,
    /// The interpreter's global activation and namespace table.
    Global,
    /// A selected frame whose activation class is not proved.
    Unknown,
}

/// Whether binding addresses are proof-bearing or legacy lexical projections.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BindingIdentity {
    /// Isolated compatibility analysis without execution-point identities.
    #[default]
    Legacy,
    /// Actual frame/cell bindings at the selected point.
    Bound,
}

/// Reaching contents-store provenance independent of a cell's allocation lifetime.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ContentsOrigin {
    /// Contents supplied at interpreter or activation entry.
    Incoming,
    /// A store whose exact source invocation is known.
    WrittenAt(u32),
    /// Enumerated branch or loop alternatives, including an incoming value.
    Alternatives {
        /// Whether the activation's incoming contents can still reach this point.
        incoming: bool,
        /// Source writes whose contents can reach this point.
        writes: Vec<u32>,
    },
    /// Different or unenumerated reaching writes.
    Unknown,
}

impl ContentsOrigin {
    /// Join enumerated normal stores without erasing their provenance.
    #[must_use]
    pub fn joined(&self, other: &Self) -> Self {
        if self == other {
            return self.clone();
        }
        if matches!(self, Self::Unknown) || matches!(other, Self::Unknown) {
            return Self::Unknown;
        }
        let mut incoming = false;
        let mut writes = Vec::new();
        for origin in [self, other] {
            match origin {
                Self::Incoming => incoming = true,
                Self::WrittenAt(source) => writes.push(*source),
                Self::Alternatives {
                    incoming: entry,
                    writes: sites,
                } => {
                    incoming |= entry;
                    writes.extend(sites);
                }
                Self::Unknown => return Self::Unknown,
            }
        }
        writes.sort_unstable();
        writes.dedup();
        Self::Alternatives { incoming, writes }
    }
}

/// Whether a resolved binding has contents, separately from allocation or known value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentsPresence {
    /// A successful store or allocated parameter supplied contents.
    Defined,
    /// Contents are definitely absent; an unobserved read errors.
    Undefined,
    /// A closed union of defined and missing contents. Public Must queries
    /// project this to Unknown; only diagnostic alternatives consume the union.
    DefinedOrUndefined,
    /// Contents or presence depend on an unenumerated execution alternative.
    Unknown,
}

fn contents_presence_after_write(
    previous: Option<ContentsPresence>,
    conditional: bool,
) -> ContentsPresence {
    if !conditional {
        return ContentsPresence::Defined;
    }
    match previous {
        Some(ContentsPresence::Defined) => ContentsPresence::Defined,
        Some(ContentsPresence::Undefined | ContentsPresence::DefinedOrUndefined) => {
            ContentsPresence::DefinedOrUndefined
        }
        None | Some(ContentsPresence::Unknown) => ContentsPresence::Unknown,
    }
}

/// Completion evidence for one actual variable read, before its observer runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableReadCompletion {
    /// No contents or observer can supply a value; this read raises Tcl error.
    Error,
    /// Type, contents or an observer may change completion; no route is guaranteed.
    Unknown,
}

/// Proved root contents kind, separately from cell existence and value contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RootContentsKind {
    /// A root contains one scalar value.
    Scalar,
    /// A root owns an array element table, including an empty table.
    Array,
}

/// Storage class bounds for an access whose cell address is unresolved.
/// This is a May domain proof, never an existence, contents or value definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PossibleVariableAccessDomain {
    /// Every permitted binding target belongs to some namespace table.
    NamespaceOnly,
    /// A local, foreign activation or namespace cell can be selected.
    Any,
}

/// Whether unnamed cells retain their incoming contents provenance.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ContentsWorld {
    /// Stores are accounted for by named or namespace-scoped effects.
    #[default]
    Tracked,
    /// Unenumerated stores can affect cells absent from the named inventory.
    Unknown,
}

impl ContentsWorld {
    const fn is_unknown(self) -> bool {
        matches!(self, Self::Unknown)
    }

    const fn joined(self, other: Self) -> Self {
        if self.is_unknown() || other.is_unknown() {
            Self::Unknown
        } else {
            Self::Tracked
        }
    }
}

/// A known callback registered on an address that normal execution could not enumerate.
/// This is a May registration, distinct from an unknown callback implementation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PossibleVariableTraceRegistration {
    /// Bounded possible physical target domain.
    pub target: Place,
    /// Actual registered operation list.
    pub operations: Vec<tcl_registry::TraceOperation>,
    /// Actual evaluated callback prefix.
    pub prefix: String,
}

/// Original namespace tree awaiting the last actual activation release.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PendingNamespaceRetirement {
    /// Original incarnations captured before a same-path replacement can be created.
    pub namespaces: VariableNamespaceSet,
    /// Reached deletion operation, retained for cell lifetime provenance.
    pub source: u32,
    /// Some reaching paths retain the original namespace rather than delete it.
    pub conditional: bool,
}

/// Per-frame scope context for resolving a variable reference.
///
/// Built once per proc/method/namespace scope and reused, the same way the
/// lowerer reuses its command-canonicalisation maps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveContext {
    /// Current FQ namespace.
    pub namespace: String,
    /// Exact namespace owner; presentation text is not its inverse.
    pub namespace_identity: Option<crate::command_binding::SourceNamespaceKey>,
    /// Reached namespace tables retaining independent component boundaries.
    pub namespace_identities: HashSet<crate::command_binding::SourceNamespaceKey>,
    /// Namespace incarnations reachable by a new written-address lookup.
    pub namespace_addressable_identities: HashSet<crate::command_binding::SourceNamespaceKey>,
    /// Whether the retained namespace inventory closes missing table candidates.
    pub namespace_inventory: NamespaceInventoryClosure,
    /// Selected native variable-name input recipe.
    pub namespace_name_protocol: Option<tcl_syntax::naming::NativeNameProtocol>,
    /// Name input issuer retained independently of namespace tokens and string producers.
    pub execution_name_policy: Option<tcl_syntax::naming::ExecutionNamePolicy>,
    /// Actual Jim namespace-object bytes retained from the entry snapshot.
    pub namespace_objects:
        HashMap<crate::command_binding::SourceNamespaceKey, tcl_core_types::NameBytes>,
    /// Independently selected authored publication context, after actual cell resolution.
    pub authored_tmm_static:
        Option<tcl_runtime_api::authored_tmm::AuthoredTmmStaticCompilationContext>,
    /// Whether the selected frame's namespace is statically known.
    pub namespace_known: bool,
    /// Explicit core policies, independent of catalogue availability.
    pub invocation_dialect: Option<tcl_registry::InvocationDialect>,
    /// Foreign frame selected for this region, if it cannot be the current frame.
    pub selected_frame: Option<tcl_registry::FrameLevel>,
    /// Proven current activation allocation site, when supplied by source execution.
    pub activation: Option<String>,
    /// Names under a `global` declaration.
    pub globals: HashSet<String>,
    /// Names under a `variable` declaration.
    pub ns_vars: HashSet<String>,
    /// Local alias → caller target (`""` = dynamic/unresolvable).
    pub upvar_aliases: HashMap<String, String>,
    /// Names with an active `trace`.
    pub traced: VariableCellSet,
    /// Known trace registrations, retaining duplicate registrations for removal.
    pub trace_registrations: VariableCellTable<Vec<(Vec<tcl_registry::TraceOperation>, String)>>,
    /// Selected typed registration receivers, retained independently of key encoding.
    pub trace_registration_receivers: VariableCellTable<std::sync::Arc<Place>>,
    /// Enumerated callback prefixes on unenumerated addresses; never definite registrations.
    pub possible_trace_registrations: Vec<PossibleVariableTraceRegistration>,
    /// Traces whose incoming registration or operation list is unenumerated.
    pub untracked_traces: VariableCellSet,
    /// `TclOO` instance vars in scope.
    pub instance_vars: HashSet<String>,
    /// Owning class/object FQ name for `instance_vars`.
    pub instance_owner: String,
    /// Typed selection of the current variable table.
    pub frame_kind: VariableFrameKind,
    /// Known interpreter for a selected execution region.
    pub interpreter: Option<String>,
    /// Optional worker and connection identity supplied by host execution.
    pub execution: Option<tcl_registry::f5::WorkerExecution>,
    /// Point-specific aliases after following their known links.
    pub alias_bindings: VariableCellTable<Place>,
    /// Namespace binding slots whose links survive entering and leaving activations.
    pub namespace_alias_bindings: VariableCellTable<Place>,
    /// Selected-frame name links, when the runtime follows names after retargeting.
    pub name_alias_bindings: VariableCellTable<Place>,
    /// Name-following namespace links shared by every activation.
    pub namespace_name_alias_bindings: VariableCellTable<Place>,
    /// Namespace tables destroyed during this activation, retained for caller restoration.
    pub outward_namespace_destructions: VariableNamespaceSet,
    /// Pending native namespace teardowns whose original active frame still owns the cells.
    pub pending_namespace_retirements:
        HashMap<crate::command_binding::SourceNamespaceKey, PendingNamespaceRetirement>,
    /// Retained variable wrappers shared by namespace and activation views.
    pub raw_bindings: crate::raw_binding::RawBindingArena,
    /// Active captured-cell operations, authoritative across selected frames.
    pub captured_cells: crate::captured_cell::CapturedCellArena,
    /// Proved actual caller activation; lexical procedure entry supplies none.
    pub caller: Option<std::sync::Arc<ResolveContext>>,
    /// Proved literal contents keyed by shared binding-value identity.
    pub constant_values: VariableCellTable<String>,
    /// Closed physical text alternatives for purpose-only operand layout queries.
    pub(crate) closed_literal_contents:
        VariableCellTable<crate::literal_contents::ClosedLiteralContents>,
    /// Active dictionary-wrapper inputs, retained until their reached epilogue.
    pub dictionary_scopes: HashMap<
        crate::dictionary_bindings::DictionaryScopeId,
        std::sync::Arc<crate::dictionary_bindings::DictionaryScopeActivation>,
    >,
    /// Native representation evidence, independent of equal string contents.
    pub value_representations: VariableCellTable<crate::native_numeric::StoredNativeRepresentation>,
    /// Shared representation world stamp; unequal or exhausted stamps decline
    /// retaining a frozen argument's numeric representation across effects.
    pub(crate) representation_epoch: Option<u64>,
    /// Greatest issued representation stamp, retained even after a control join.
    /// Exhaustion permanently prevents issuing another frozen receipt.
    pub(crate) representation_epoch_high_water: Option<u64>,
    /// Reaching source writes, used to abstain on unrepresented nested definitions.
    pub contents_origins: VariableCellTable<ContentsOrigin>,
    /// Source-instance attestation of represented physical contents stores.
    pub(crate) contents_source_proofs: crate::contents_source::ContentsSourceProofs,
    /// Defined contents versus allocated-but-undefined cells.
    pub contents_presence: VariableCellTable<ContentsPresence>,
    /// Typed addresses recorded by the same presence owner. Join uses these
    /// receivers to query an untouched predecessor, never parsed identity labels.
    pub contents_presence_slots: VariableCellTable<std::sync::Arc<Place>>,
    /// Successful root allocation/store kind; missing entries are unproved.
    pub contents_kinds: VariableCellTable<RootContentsKind>,
    /// An unenumerated write can affect a cell absent from the named inventory.
    pub contents_world: ContentsWorld,
    /// Closure of the newly entered activation's private contents, independent
    /// of unknown incoming namespace values. Only the actual frame-entry owner
    /// issues this axis; callbacks and unresolved writes withdraw it.
    pub(crate) activation_contents_world: Option<ContentsWorld>,
    /// A newly entered private activation starts with no inherited variable
    /// observers. This is separate from its contents and namespace trace world.
    pub(crate) activation_observers_closed: bool,
    /// Actual fresh receiver allocations whose observer world remains enumerated.
    pub(crate) closed_observer_allocations: HashSet<crate::command_binding::SourceObjectAllocation>,
    /// Namespace-scoped contents clobbers without enumerated cell names.
    pub contents_unknown_namespaces: VariableNamespaceSet,
    /// Physical array roots affected by stores or deletions to an unknown element.
    /// Explicit later element writes override this mask for that named element.
    pub contents_unknown_arrays: VariableCellSet,
    /// Exact arrays born from a proved absent root and original literal stores.
    pub(crate) closed_array_roots: VariableCellSet,
    /// Names whose bindings differ across incoming control-flow paths.
    pub unknown_bindings: VariableCellSet,
    /// All current-frame bindings may have been retargeted by evaluated code.
    pub dynamic_bindings: bool,
    /// Namespace tables known to exist at this program point.
    pub known_namespaces: HashSet<String>,
    /// Reaching namespace-cell allocation, used by Tcl8 global fallback.
    pub namespace_cells: NamespaceCellPresence,
    /// Root lifetimes changed by array or namespace destruction.
    pub generations: VariableCellTable<CellGeneration>,
    /// Cell identities are available in this context.
    pub binding_identity: BindingIdentity,
    /// Unknown trace registrations may affect any cell operation.
    pub dynamic_traces: bool,
}

/// Whether retained namespace membership can prove a missing candidate.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum NamespaceInventoryClosure {
    /// Unenumerated incarnations may remain reachable.
    #[default]
    Open,
    /// The independently retained inventory enumerates reachable incarnations.
    Closed,
}

impl NamespaceInventoryClosure {
    /// Missing candidates are known absent only in a closed inventory.
    #[must_use]
    pub const fn is_closed(self) -> bool {
        matches!(self, Self::Closed)
    }

    fn joined(self, other: Self, same_membership: bool) -> Self {
        if self.is_closed() && other.is_closed() && same_membership {
            Self::Closed
        } else {
            Self::Open
        }
    }
}

/// Exact alpha-renaming of proved activation allocations and source write sites.
/// Unmapped external identities remain unchanged and therefore retain cache dependencies.
/// A reusable template must use an injective mapping and preserve its inverse.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VariableProofRelocation {
    /// Actual activation allocation identity to template or instantiated identity.
    pub activations: std::collections::BTreeMap<String, String>,
    /// Exact allocation/store source offsets to their relocated offsets.
    pub source_offsets: std::collections::BTreeMap<u32, u32>,
    /// Canonical key mappings derived from retained typed cell addresses.
    pub storage_keys: HashMap<VariableCellKey, VariableCellKey>,
    /// Exact source instances mapped by an authorised body-template owner.
    pub source_origins: std::collections::BTreeMap<
        std::sync::Arc<crate::command_binding::SourceOriginId>,
        std::sync::Arc<crate::command_binding::SourceOriginId>,
    >,
}

impl VariableProofRelocation {
    /// Invert an injective alpha-renaming; ambiguous mappings cannot instantiate a proof.
    #[must_use]
    pub fn inverse(&self) -> Option<Self> {
        let activations: std::collections::BTreeMap<_, _> = self
            .activations
            .iter()
            .map(|(from, to)| (to.clone(), from.clone()))
            .collect();
        let source_offsets: std::collections::BTreeMap<_, _> = self
            .source_offsets
            .iter()
            .map(|(&from, &to)| (to, from))
            .collect();
        let source_origins: std::collections::BTreeMap<_, _> = self
            .source_origins
            .iter()
            .map(|(from, to)| (std::sync::Arc::clone(to), std::sync::Arc::clone(from)))
            .collect();
        let storage_keys: HashMap<_, _> = self
            .storage_keys
            .iter()
            .map(|(from, to)| (to.clone(), from.clone()))
            .collect();
        (storage_keys.len() == self.storage_keys.len()
            && activations.len() == self.activations.len()
            && source_offsets.len() == self.source_offsets.len()
            && source_origins.len() == self.source_origins.len())
        .then_some(Self {
            activations,
            source_offsets,
            storage_keys,
            source_origins,
        })
    }

    /// Relocate an exact source instance, retaining unmapped external origins.
    #[must_use]
    pub fn source_origin(
        &self,
        source: &std::sync::Arc<crate::command_binding::SourceOriginId>,
    ) -> std::sync::Arc<crate::command_binding::SourceOriginId> {
        self.source_origins
            .get(source)
            .cloned()
            .unwrap_or_else(|| std::sync::Arc::clone(source))
    }

    /// Relocate one exact source site, retaining external sites as dependencies.
    #[must_use]
    pub fn source_offset(&self, source: u32) -> u32 {
        self.source_offsets.get(&source).copied().unwrap_or(source)
    }

    /// Relocate a source-authored lifetime while retaining incoming or unknown identity.
    #[must_use]
    pub fn cell_generation(&self, generation: CellGeneration) -> CellGeneration {
        match generation {
            CellGeneration::After(source) => CellGeneration::After(self.source_offset(source)),
            other => other,
        }
    }

    /// Relocate a typed physical cell address and its name/index dependencies.
    #[must_use]
    pub fn place(&self, place: &Place) -> Place {
        let mut relocated = place.clone();
        if let Some(cell) = &mut relocated.cell {
            if let CellOwner::NamespaceIdentity(identity) = &mut cell.owner {
                **identity = self.namespace_key(identity);
            }
            if let CellOwner::AllocatedInstance(allocation) = &mut cell.owner {
                **allocation = crate::allocated_instance::relocate_allocation(allocation, self);
            }
            if let CellOwner::RetainedSlot(identity) = &mut cell.owner {
                **identity = identity.relocated(self);
            }
            if let CellOwner::Activation(identity) = &mut cell.owner
                && let Some(replacement) = self.activations.get(identity)
            {
                identity.clone_from(replacement);
            }
            if let CellGeneration::After(source) = &mut cell.generation {
                *source = self.source_offset(*source);
            }
        }
        for dependency in &mut relocated.name_reads {
            *dependency = self.place(dependency);
        }
        for index in relocated.index.iter_mut().chain(relocated.keys.iter_mut()) {
            for dependency in &mut index.read_places {
                *dependency = self.place(dependency);
            }
        }
        relocated
    }

    /// Relocate an exact storage slot without parsing its presentation.
    #[must_use]
    pub fn cell_key(&self, key: &VariableCellKey) -> VariableCellKey {
        if let Some(replacement) = self.storage_keys.get(key) {
            return replacement.clone();
        }
        match key {
            VariableCellKey::Lifetime { source, cell } => self
                .cell_key(cell)
                .with_lifetime(self.source_offset(*source)),
            VariableCellKey::Element { cell, index } => self.cell_key(cell).with_index(index),
            VariableCellKey::RetainedSlot(slot) => {
                VariableCellKey::RetainedSlot(Box::new(slot.relocated(self)))
            }
            VariableCellKey::Activation { identity, simple } => VariableCellKey::Activation {
                identity: self
                    .activations
                    .get(identity)
                    .cloned()
                    .unwrap_or_else(|| identity.clone()),
                simple: simple.clone(),
            },
            VariableCellKey::AllocatedInstance { allocation, simple } => {
                VariableCellKey::AllocatedInstance {
                    allocation: Box::new(crate::allocated_instance::relocate_allocation(
                        allocation, self,
                    )),
                    simple: simple.clone(),
                }
            }
            VariableCellKey::Namespace { identity, simple } => VariableCellKey::Namespace {
                identity: self.namespace_key(identity),
                simple: simple.clone(),
            },
            key => key.clone(),
        }
    }

    /// Relocate an exact source-created namespace; native and authored owners persist.
    #[must_use]
    pub fn namespace_key(
        &self,
        key: &crate::command_binding::SourceNamespaceKey,
    ) -> crate::command_binding::SourceNamespaceKey {
        use crate::command_binding::{CommandAllocationSite, SourceNamespaceKey};
        match key {
            SourceNamespaceKey::Allocated {
                site,
                incarnation,
                path,
            } => SourceNamespaceKey::Allocated {
                site: CommandAllocationSite {
                    source: self.source_origin(&site.source),
                    offset: self.source_offset(site.offset),
                },
                incarnation: *incarnation,
                path: path.clone(),
            },
            key => key.clone(),
        }
    }

    /// Text queries explicitly select only authored storage.
    #[must_use]
    pub fn storage_key<Q: VariableCellKeyQuery + ?Sized>(&self, key: &Q) -> VariableCellKey {
        self.cell_key(key.variable_cell_key().as_ref())
    }

    fn storage_map<T: Clone>(&self, values: &VariableCellTable<T>) -> VariableCellTable<T> {
        values
            .iter()
            .map(|(key, value)| (self.cell_key(key), value.clone()))
            .collect()
    }

    fn place_map(
        &self,
        values: &VariableCellTable<std::sync::Arc<Place>>,
    ) -> VariableCellTable<std::sync::Arc<Place>> {
        values
            .iter()
            .map(|(key, place)| (self.cell_key(key), std::sync::Arc::new(self.place(place))))
            .collect()
    }

    /// Relocate a typed selected activation while retaining external frame selections.
    #[must_use]
    pub fn frame(&self, frame: &VariableExecutionFrame) -> VariableExecutionFrame {
        if let VariableExecutionFrame::NamespaceIdentity { namespace, frame } = frame {
            return self
                .frame(frame)
                .with_namespace_identity(self.namespace_key(namespace));
        }
        let mut frame = frame.clone();
        if let VariableExecutionFrame::Procedure { identity, .. }
        | VariableExecutionFrame::NamespaceActivation { identity, .. }
        | VariableExecutionFrame::ReceiverMethod { identity } = &mut frame
            && let Some(replacement) = self.activations.get(identity)
        {
            identity.clone_from(replacement);
        }
        frame
    }

    fn contents_origin(&self, origin: &ContentsOrigin) -> ContentsOrigin {
        match origin {
            ContentsOrigin::WrittenAt(source) => {
                ContentsOrigin::WrittenAt(self.source_offset(*source))
            }
            ContentsOrigin::Alternatives { incoming, writes } => {
                let mut writes: Vec<_> = writes
                    .iter()
                    .map(|source| self.source_offset(*source))
                    .collect();
                writes.sort_unstable();
                writes.dedup();
                ContentsOrigin::Alternatives {
                    incoming: *incoming,
                    writes,
                }
            }
            other => other.clone(),
        }
    }
}

impl PossibleVariableTraceRegistration {
    fn relocated(&self, relocation: &VariableProofRelocation) -> Self {
        Self {
            target: relocation.place(&self.target),
            operations: self.operations.clone(),
            prefix: self.prefix.clone(),
        }
    }
}

impl ResolveContext {
    /// Relocate the entire binding and contents proof, including caller views and trace cells.
    /// No dialect, existence, observer, or unknown-world field is erased for memoisation.
    #[must_use]
    pub fn relocated(&self, relocation: &VariableProofRelocation) -> Self {
        let expanded = crate::allocated_instance::relocation_with_cell_keys(self, relocation);
        let relocation = &expanded;
        let mut context = self.clone();
        context.closed_observer_allocations = self
            .closed_observer_allocations
            .iter()
            .map(|allocation| {
                crate::allocated_instance::relocate_allocation(allocation, relocation)
            })
            .collect();
        context.namespace_identity = self
            .namespace_identity
            .as_ref()
            .map(|key| relocation.namespace_key(key));
        context.namespace_objects = self
            .namespace_objects
            .iter()
            .map(|(key, object)| (relocation.namespace_key(key), object.clone()))
            .collect();
        context.pending_namespace_retirements = self
            .pending_namespace_retirements
            .iter()
            .map(|(key, pending)| {
                (
                    relocation.namespace_key(key),
                    PendingNamespaceRetirement {
                        namespaces: pending
                            .namespaces
                            .iter()
                            .map(|key| relocation.namespace_key(key))
                            .collect(),
                        source: relocation.source_offset(pending.source),
                        conditional: pending.conditional,
                    },
                )
            })
            .collect();
        context.namespace_cells.present = self
            .namespace_cells
            .present
            .iter()
            .map(|key| relocation.cell_key(key))
            .collect();
        context.namespace_cells.possible = self
            .namespace_cells
            .possible
            .iter()
            .map(|key| relocation.cell_key(key))
            .collect();
        context.namespace_identities = self
            .namespace_identities
            .iter()
            .map(|key| relocation.namespace_key(key))
            .collect();
        context.namespace_addressable_identities = self
            .namespace_addressable_identities
            .iter()
            .map(|key| relocation.namespace_key(key))
            .collect();
        context.outward_namespace_destructions = self
            .outward_namespace_destructions
            .iter()
            .map(|key| relocation.namespace_key(key))
            .collect();
        context.contents_unknown_namespaces = self
            .contents_unknown_namespaces
            .iter()
            .map(|key| relocation.namespace_key(key))
            .collect();
        context.raw_bindings = self.raw_bindings.relocated(relocation);
        context.captured_cells = self.captured_cells.relocated(relocation);
        context.possible_trace_registrations = self
            .possible_trace_registrations
            .iter()
            .map(|registration| registration.relocated(relocation))
            .collect();
        context.dictionary_scopes = self
            .dictionary_scopes
            .iter()
            .map(|(source, activation)| {
                (
                    source.relocated(relocation),
                    std::sync::Arc::new(activation.relocated(relocation)),
                )
            })
            .collect();
        if let Some(identity) = &mut context.activation
            && let Some(replacement) = relocation.activations.get(identity)
        {
            identity.clone_from(replacement);
        }
        context.relocate_storage_addresses(relocation);
        context.relocate_contents_and_observers(relocation);
        context
    }

    fn relocate_storage_addresses(&mut self, relocation: &VariableProofRelocation) {
        for map in [&mut self.alias_bindings, &mut self.name_alias_bindings] {
            *map = map
                .iter()
                .map(|(key, value)| (relocation.cell_key(key), relocation.place(value)))
                .collect();
        }
        for map in [
            &mut self.namespace_alias_bindings,
            &mut self.namespace_name_alias_bindings,
        ] {
            *map = map
                .iter()
                .map(|(key, value)| (relocation.storage_key(key), relocation.place(value)))
                .collect();
        }
        self.generations = self
            .generations
            .iter()
            .map(|(key, &generation)| {
                (
                    relocation.storage_key(key),
                    relocation.cell_generation(generation),
                )
            })
            .collect();
    }

    fn relocate_contents_and_observers(&mut self, relocation: &VariableProofRelocation) {
        self.value_representations = relocation.storage_map(&self.value_representations);
        self.constant_values = relocation.storage_map(&self.constant_values);
        self.closed_literal_contents = self
            .closed_literal_contents
            .iter()
            .map(|(key, contents)| (relocation.storage_key(key), contents.relocated(relocation)))
            .collect();
        self.contents_source_proofs = self.contents_source_proofs.relocated(relocation);
        self.contents_presence = relocation.storage_map(&self.contents_presence);
        self.contents_presence_slots = relocation.place_map(&self.contents_presence_slots);
        self.contents_kinds = relocation.storage_map(&self.contents_kinds);
        self.contents_origins = self
            .contents_origins
            .iter()
            .map(|(key, origin)| {
                (
                    relocation.storage_key(key),
                    relocation.contents_origin(origin),
                )
            })
            .collect();
        self.contents_unknown_arrays = self
            .contents_unknown_arrays
            .iter()
            .map(|key| relocation.storage_key(key))
            .collect();
        self.closed_array_roots = self
            .closed_array_roots
            .iter()
            .map(|key| relocation.storage_key(key))
            .collect();
        self.namespace_cells.closed_namespaces = self
            .namespace_cells
            .closed_namespaces
            .iter()
            .map(|key| relocation.namespace_key(key))
            .collect();
        self.trace_registrations = relocation.storage_map(&self.trace_registrations);
        self.trace_registration_receivers =
            relocation.place_map(&self.trace_registration_receivers);
        for names in [
            &mut self.traced,
            &mut self.untracked_traces,
            &mut self.unknown_bindings,
        ] {
            *names = names
                .iter()
                .map(|key| relocation.storage_key(key))
                .collect();
        }
        if let Some(caller) = &self.caller {
            self.caller = Some(std::sync::Arc::new(caller.relocated(relocation)));
        }
    }
}

/// Restore the caller activation while retaining effects on outward namespace storage.
#[must_use]
pub fn restore_execution_frame(parent: &ResolveContext, child: &ResolveContext) -> ResolveContext {
    let mut restored = restore_selected_bindings(parent, child);
    restore_namespace_bindings(&mut restored, child);
    let mut parent_activations = HashSet::new();
    let mut frame = Some(parent);
    while let Some(context) = frame {
        if let Some(identity) = &context.activation {
            parent_activations.insert(identity.clone());
        }
        frame = context.caller.as_deref();
    }
    let instance_keys = crate::allocated_instance::outward_cell_keys(parent)
        .union(&crate::allocated_instance::outward_cell_keys(child))
        .cloned()
        .collect::<HashSet<_>>();
    let parent_key = |key: &VariableCellKey| {
        instance_keys.contains(key)
            || match key.root() {
                VariableCellKey::Namespace { .. } | VariableCellKey::RetainedSlot(_) => true,
                VariableCellKey::Activation { identity, .. } => {
                    parent_activations.contains(identity)
                }
                VariableCellKey::Authored(name) => name.starts_with("::"),
                _ => false,
            }
    };
    restored.generations.retain(|key, _| !parent_key(key));
    restored.generations.extend(
        child
            .generations
            .iter()
            .filter(|(key, _)| parent_key(key))
            .map(|(key, generation)| (key.clone(), *generation)),
    );
    restored.traced.retain(|key| !parent_key(key));
    restored
        .traced
        .extend(child.traced.iter().filter(|key| parent_key(key)).cloned());
    restored
        .trace_registrations
        .retain(|key, _| !parent_key(key));
    restored.trace_registrations.extend(
        child
            .trace_registrations
            .iter()
            .filter(|(key, _)| parent_key(key))
            .map(|(key, registrations)| (key.clone(), registrations.clone())),
    );
    restored
        .trace_registration_receivers
        .retain(|key, _| !parent_key(key));
    restored.trace_registration_receivers.extend(
        child
            .trace_registration_receivers
            .iter()
            .filter(|(key, _)| parent_key(key))
            .map(|(key, value)| (key.clone(), std::sync::Arc::clone(value))),
    );
    restored.untracked_traces.retain(|key| !parent_key(key));
    restored.untracked_traces.extend(
        child
            .untracked_traces
            .iter()
            .filter(|key| parent_key(key))
            .cloned(),
    );
    restore_outward_values(&mut restored, child, &parent_key);
    restored
        .contents_unknown_arrays
        .retain(|key| !parent_key(key));
    restored.contents_unknown_arrays.extend(
        child
            .contents_unknown_arrays
            .iter()
            .filter(|key| parent_key(key))
            .cloned(),
    );
    restored.closed_array_roots.retain(|key| !parent_key(key));
    restored.closed_array_roots.extend(
        child
            .closed_array_roots
            .iter()
            .filter(|key| parent_key(key))
            .cloned(),
    );
    restored.contents_world = restored.contents_world.joined(child.contents_world);
    restored
        .contents_unknown_namespaces
        .extend(child.contents_unknown_namespaces.iter().cloned());
    restored
        .outward_namespace_destructions
        .extend(child.outward_namespace_destructions.iter().cloned());
    restored.dynamic_traces |= child.dynamic_traces;
    restored
        .closed_observer_allocations
        .clone_from(&child.closed_observer_allocations);
    restore_outward_aliases(parent, child, &mut restored);
    if child.dynamic_bindings {
        restored.widen();
    }
    crate::variable_bindings::complete_pending_namespace_retirements(&mut restored);
    restored
}

fn restore_namespace_bindings(restored: &mut ResolveContext, child: &ResolveContext) {
    restored.raw_bindings.clone_from(&child.raw_bindings);
    restored.captured_cells.clone_from(&child.captured_cells);
    restored
        .possible_trace_registrations
        .clone_from(&child.possible_trace_registrations);
    restored.namespace_cells.clone_from(&child.namespace_cells);
    restored
        .pending_namespace_retirements
        .clone_from(&child.pending_namespace_retirements);
    restored
        .namespace_identities
        .clone_from(&child.namespace_identities);
    restored
        .namespace_addressable_identities
        .clone_from(&child.namespace_addressable_identities);
    restored.namespace_inventory = child.namespace_inventory;
    restored.namespace_name_protocol = child.namespace_name_protocol;
    restored.execution_name_policy = child.execution_name_policy;
    let retained_current = restored.namespace_identity.as_ref().and_then(|key| {
        restored
            .namespace_objects
            .get(key)
            .cloned()
            .map(|object| (key.clone(), object))
    });
    restored
        .namespace_objects
        .clone_from(&child.namespace_objects);
    if let Some((key, object)) = retained_current {
        restored.namespace_objects.entry(key).or_insert(object);
    }
    restored
        .authored_tmm_static
        .clone_from(&child.authored_tmm_static);
    restored
        .namespace_alias_bindings
        .clone_from(&child.namespace_alias_bindings);
    restored
        .namespace_name_alias_bindings
        .clone_from(&child.namespace_name_alias_bindings);
    restored
        .known_namespaces
        .clone_from(&child.known_namespaces);
}

fn restore_outward_aliases(
    parent: &ResolveContext,
    child: &ResolveContext,
    restored: &mut ResolveContext,
) {
    for (name, alias) in &mut restored.alias_bindings {
        if let Some(retained) = child.raw_bindings.retained_array_member(alias) {
            *alias = retained;
            continue;
        }
        let deleted = alias.is_global()
            && (child
                .outward_namespace_destructions
                .iter()
                .any(|namespace| cell_key(alias).is_owned_by_namespace(namespace))
                || (cell_key(alias).authored_spelling().is_some()
                    && parent.known_namespaces.contains(&alias.ns)
                    && !child.known_namespaces.contains(&alias.ns)));
        let generation = child
            .generations
            .get(&cell_key(alias))
            .copied()
            .unwrap_or_default();
        let changed = alias
            .cell
            .as_ref()
            .is_some_and(|cell| cell.generation != generation);
        let jim = child
            .invocation_dialect
            .and_then(|dialect| dialect.variable_link_binding)
            == Some(tcl_dialect::VariableLinkBinding::SelectedFrameName);
        if !jim && (deleted || (changed && alias.kind == PlaceKind::ArrayElem)) {
            restored.unknown_bindings.insert(name.clone());
        } else if changed && let Some(cell) = &mut alias.cell {
            cell.generation = generation;
        }
    }
}

fn restore_outward_values(
    restored: &mut ResolveContext,
    child: &ResolveContext,
    parent_key: &impl Fn(&VariableCellKey) -> bool,
) {
    restored
        .constant_values
        .retain(|key, _| !parent_key(key) || child.constant_values.contains_key(key));
    restored.constant_values.extend(
        child
            .constant_values
            .iter()
            .filter(|(key, _)| parent_key(key))
            .map(|(key, value)| (key.clone(), value.clone())),
    );
    restored
        .closed_literal_contents
        .retain(|key, _| !parent_key(key));
    restored.closed_literal_contents.extend(
        child
            .closed_literal_contents
            .iter()
            .filter(|(key, _)| parent_key(key))
            .map(|(key, value)| (key.clone(), value.clone())),
    );
    // Representation changes affect shared Tcl objects, including objects whose
    // variable bindings belong to an unselected ancestor frame.
    restored
        .value_representations
        .clone_from(&child.value_representations);
    restored.representation_epoch = child.representation_epoch;
    restored.representation_epoch_high_water = restored
        .representation_epoch_high_water
        .zip(child.representation_epoch_high_water)
        .map(|(left, right)| left.max(right));
    restored.contents_kinds.retain(|key, _| !parent_key(key));
    restored.contents_kinds.extend(
        child
            .contents_kinds
            .iter()
            .filter(|(key, _)| parent_key(key))
            .map(|(key, kind)| (key.clone(), *kind)),
    );
    restored.contents_presence.retain(|key, _| !parent_key(key));
    restored.contents_presence.extend(
        child
            .contents_presence
            .iter()
            .filter(|(key, _)| parent_key(key))
            .map(|(key, &presence)| (key.clone(), presence)),
    );
    restored
        .contents_presence_slots
        .retain(|key, _| !parent_key(key));
    restored.contents_presence_slots.extend(
        child
            .contents_presence_slots
            .iter()
            .filter(|(key, _)| parent_key(key))
            .map(|(key, place)| (key.clone(), std::sync::Arc::clone(place))),
    );
    restored
        .contents_source_proofs
        .restore(&child.contents_source_proofs, parent_key);
    restored.contents_origins.retain(|key, _| !parent_key(key));
    restored.contents_origins.extend(
        child
            .contents_origins
            .iter()
            .filter(|(key, _)| parent_key(key))
            .map(|(key, origin)| (key.clone(), origin.clone())),
    );
}

fn same_execution_frame(left: &ResolveContext, right: &ResolveContext) -> bool {
    left.frame_kind == right.frame_kind
        && left.namespace == right.namespace
        && left.namespace_identity == right.namespace_identity
        && ((left.global_frame() && right.global_frame())
            || (left.activation.is_some() && left.activation == right.activation))
}

fn restore_selected_bindings(parent: &ResolveContext, child: &ResolveContext) -> ResolveContext {
    let mut parents = Vec::new();
    let mut frame = Some(parent);
    while let Some(current) = frame {
        parents.push(current);
        frame = current.caller.as_deref();
    }
    let mut restored = None;
    for parent in parents.into_iter().rev() {
        let mut current = parent.clone();
        let mut candidate = Some(child);
        while let Some(frame) = candidate {
            if same_execution_frame(parent, frame) {
                current.alias_bindings.clone_from(&frame.alias_bindings);
                current
                    .name_alias_bindings
                    .clone_from(&frame.name_alias_bindings);
                current.unknown_bindings.clone_from(&frame.unknown_bindings);
                current.globals.clone_from(&frame.globals);
                current.ns_vars.clone_from(&frame.ns_vars);
                current.upvar_aliases.clone_from(&frame.upvar_aliases);
                current.dynamic_bindings = frame.dynamic_bindings;
                current.activation_contents_world = frame.activation_contents_world;
                current.activation_observers_closed = frame.activation_observers_closed;
                break;
            }
            candidate = frame.caller.as_deref();
        }
        current.caller = restored;
        restored = Some(std::sync::Arc::new(current));
    }
    std::sync::Arc::try_unwrap(restored.expect("parent frame was retained"))
        .expect("restored root has a single owner")
}

fn hash_unordered_set<T: Hash, H: Hasher>(items: &HashSet<T>, state: &mut H) {
    let mut hashes = items
        .iter()
        .map(|item| {
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            item.hash(&mut hash);
            hash.finish()
        })
        .collect::<Vec<_>>();
    hashes.sort_unstable();
    hashes.hash(state);
}

fn hash_set<T: Hash + Ord, H: Hasher>(items: &HashSet<T>, state: &mut H) {
    let mut ordered: Vec<_> = items.iter().collect();
    ordered.sort_unstable();
    ordered.hash(state);
}

fn hash_map<K: Hash + Ord, V: Hash, H: Hasher>(items: &HashMap<K, V>, state: &mut H) {
    let mut ordered: Vec<_> = items.iter().collect();
    ordered.sort_unstable_by_key(|(key, _)| *key);
    ordered.hash(state);
}

impl Hash for NamespaceCellPresence {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.present.hash(state);
        self.possible.hash(state);
        self.closed.hash(state);
        self.closed_namespaces.hash(state);
    }
}

impl Hash for ResolveContext {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.namespace.hash(state);
        self.namespace_identity.hash(state);
        hash_set(&self.namespace_identities, state);
        hash_set(&self.namespace_addressable_identities, state);
        self.namespace_inventory.hash(state);
        self.namespace_name_protocol.hash(state);
        self.execution_name_policy.hash(state);
        hash_map(&self.namespace_objects, state);
        self.authored_tmm_static.hash(state);
        self.namespace_known.hash(state);
        self.invocation_dialect.hash(state);
        self.selected_frame.hash(state);
        self.activation.hash(state);
        hash_set(&self.globals, state);
        hash_set(&self.ns_vars, state);
        hash_map(&self.upvar_aliases, state);
        self.traced.hash(state);
        self.trace_registrations.hash(state);
        self.trace_registration_receivers.hash(state);
        self.possible_trace_registrations.hash(state);
        self.untracked_traces.hash(state);
        hash_set(&self.instance_vars, state);
        self.instance_owner.hash(state);
        self.frame_kind.hash(state);
        self.interpreter.hash(state);
        self.execution.hash(state);
        self.alias_bindings.hash(state);
        self.namespace_alias_bindings.hash(state);
        self.name_alias_bindings.hash(state);
        self.namespace_name_alias_bindings.hash(state);
        self.outward_namespace_destructions.hash(state);
        hash_map(&self.pending_namespace_retirements, state);
        self.raw_bindings.hash(state);
        self.captured_cells.hash(state);
        self.caller.hash(state);
        self.constant_values.hash(state);
        self.closed_literal_contents.hash(state);
        hash_map(&self.dictionary_scopes, state);
        self.value_representations.hash(state);
        self.representation_epoch.hash(state);
        self.representation_epoch_high_water.hash(state);
        self.contents_origins.hash(state);
        self.contents_source_proofs.hash(state);
        self.contents_presence.hash(state);
        self.contents_presence_slots.hash(state);
        self.contents_kinds.hash(state);
        self.contents_world.hash(state);
        self.activation_contents_world.hash(state);
        self.activation_observers_closed.hash(state);
        hash_unordered_set(&self.closed_observer_allocations, state);
        self.contents_unknown_namespaces.hash(state);
        self.contents_unknown_arrays.hash(state);
        self.closed_array_roots.hash(state);
        self.unknown_bindings.hash(state);
        self.dynamic_bindings.hash(state);
        hash_set(&self.known_namespaces, state);
        self.namespace_cells.hash(state);
        self.generations.hash(state);
        self.binding_identity.hash(state);
        self.dynamic_traces.hash(state);
    }
}

impl Default for ResolveContext {
    fn default() -> Self {
        Self {
            namespace: "::".to_owned(),
            namespace_identity: None,
            namespace_identities: HashSet::new(),
            namespace_addressable_identities: HashSet::new(),
            namespace_inventory: NamespaceInventoryClosure::Open,
            namespace_name_protocol: None,
            execution_name_policy: None,
            namespace_objects: HashMap::new(),
            authored_tmm_static: None,
            namespace_known: true,
            invocation_dialect: None,
            selected_frame: None,
            activation: None,
            globals: HashSet::new(),
            ns_vars: HashSet::new(),
            upvar_aliases: HashMap::new(),
            traced: VariableCellSet::default(),
            trace_registrations: VariableCellTable::default(),
            trace_registration_receivers: VariableCellTable::default(),
            possible_trace_registrations: Vec::new(),
            untracked_traces: VariableCellSet::default(),
            instance_vars: HashSet::new(),
            instance_owner: String::new(),
            frame_kind: VariableFrameKind::Local,
            interpreter: None,
            execution: None,
            raw_bindings: crate::raw_binding::RawBindingArena::default(),
            captured_cells: crate::captured_cell::CapturedCellArena::default(),
            alias_bindings: VariableCellTable::default(),
            namespace_alias_bindings: VariableCellTable::default(),
            name_alias_bindings: VariableCellTable::default(),
            namespace_name_alias_bindings: VariableCellTable::default(),
            outward_namespace_destructions: VariableNamespaceSet::default(),
            pending_namespace_retirements: HashMap::new(),
            caller: None,
            constant_values: VariableCellTable::default(),
            closed_literal_contents: VariableCellTable::default(),
            dictionary_scopes: HashMap::new(),
            value_representations: VariableCellTable::default(),
            representation_epoch: Some(0),
            representation_epoch_high_water: Some(0),
            contents_origins: VariableCellTable::default(),
            contents_source_proofs: crate::contents_source::ContentsSourceProofs::default(),
            contents_presence: VariableCellTable::default(),
            contents_presence_slots: VariableCellTable::default(),
            contents_kinds: VariableCellTable::default(),
            contents_world: ContentsWorld::Tracked,
            activation_contents_world: None,
            activation_observers_closed: false,
            closed_observer_allocations: HashSet::new(),
            contents_unknown_namespaces: VariableNamespaceSet::default(),
            contents_unknown_arrays: VariableCellSet::default(),
            closed_array_roots: VariableCellSet::default(),
            unknown_bindings: VariableCellSet::default(),
            dynamic_bindings: false,
            known_namespaces: HashSet::from(["::".to_owned()]),
            namespace_cells: NamespaceCellPresence::default(),
            generations: VariableCellTable::default(),
            binding_identity: BindingIdentity::Legacy,
            dynamic_traces: false,
        }
    }
}

impl ResolveContext {
    /// Pure purpose-selected naming advice. This supplies no cell or native slot.
    ///
    /// # Errors
    /// Missing issuers and unsupported observed inputs remain unavailable.
    pub fn execution_variable_name_projection<'a>(
        &self,
        original: tcl_syntax::naming::NativeVariableInputForm<'a>,
        purpose: tcl_syntax::naming::ObservedVariableNamePurpose,
    ) -> Result<
        tcl_syntax::naming::ExecutionVariableNameProjection<'a>,
        tcl_syntax::naming::NameProjectionUnavailable,
    > {
        self.execution_name_policy
            .ok_or(tcl_syntax::naming::NameProjectionUnavailable::PurposeNotModelled)?
            .variable_input(original, purpose)
    }

    pub(super) fn observed_variable_storage_unavailable(&self) -> bool {
        // Runtime event arenas do not issue a source/native frame allocation token.
        matches!(
            self.execution_name_policy,
            Some(tcl_syntax::naming::ExecutionNamePolicy::ObservedBigIp(_))
        )
    }

    /// Whether bare names address the current namespace table.
    #[must_use]
    pub const fn namespace_scope(&self) -> bool {
        matches!(
            self.frame_kind,
            VariableFrameKind::Namespace | VariableFrameKind::Global
        )
    }

    /// Whether numeric levels are rooted at the actual global activation.
    #[must_use]
    pub const fn global_frame(&self) -> bool {
        matches!(self.frame_kind, VariableFrameKind::Global)
    }

    /// Whether immutable cell identities are selected at this program point.
    #[must_use]
    pub const fn point_sensitive(&self) -> bool {
        matches!(self.binding_identity, BindingIdentity::Bound)
    }

    /// Namespace-variable frame, distinct from a procedure activation.
    #[must_use]
    pub fn for_namespace(namespace: impl Into<String>) -> Self {
        let namespace = namespace.into();
        Self {
            known_namespaces: HashSet::from(["::".to_owned(), namespace.clone()]),
            namespace,
            frame_kind: VariableFrameKind::Namespace,
            binding_identity: BindingIdentity::Bound,
            ..Self::default()
        }
    }

    /// Project the actual selected activation into registry alias/completion policy.
    #[must_use]
    pub const fn alias_frame(&self) -> tcl_registry::VariableAliasFrame {
        match self.frame_kind {
            VariableFrameKind::Local => tcl_registry::VariableAliasFrame::Procedure,
            VariableFrameKind::Namespace => tcl_registry::VariableAliasFrame::Namespace,
            VariableFrameKind::Global => tcl_registry::VariableAliasFrame::Global,
            VariableFrameKind::Unknown => tcl_registry::VariableAliasFrame::Unknown,
        }
    }

    /// Entry environment for a compiler function. `::top` uses the global table.
    #[must_use]
    pub fn for_function(name: &str) -> Self {
        let namespace = tcl_syntax::naming::key_holder_and_tail(name).0;
        let namespace = if namespace.is_empty() {
            "::"
        } else {
            namespace
        };
        let mut context = Self::for_namespace(namespace);
        context.frame_kind = if name == "::top" {
            VariableFrameKind::Global
        } else {
            VariableFrameKind::Local
        };
        context.activation = (name != "::top").then(|| name.to_owned());
        context
    }

    /// Select a typed execution activation while preserving shared namespace facts.
    #[must_use]
    pub fn in_frame(&self, frame: &VariableExecutionFrame) -> Self {
        if let VariableExecutionFrame::NamespaceIdentity { namespace, frame } = frame {
            let mut selected = self.in_frame(frame);
            selected.retain_namespace_world(
                namespace.clone(),
                self.namespace_identities.iter().cloned(),
                self.namespace_name_protocol,
            );
            return selected;
        }
        if let VariableExecutionFrame::Selected { selector, .. } = frame
            && let Some(selected) = self.selected_frame_context(*selector)
        {
            return selected;
        }
        let mut selected = match frame {
            VariableExecutionFrame::Global => Self::for_function("::top"),
            VariableExecutionFrame::Namespace(namespace) => Self::for_namespace(namespace.clone()),
            VariableExecutionFrame::NamespaceActivation {
                namespace,
                identity,
            } => {
                let mut selected = Self::for_namespace(namespace.clone());
                selected.activation = Some(identity.clone());
                selected
            }
            VariableExecutionFrame::Procedure {
                namespace,
                identity,
            } => {
                let mut selected = Self::for_namespace(namespace.clone());
                selected.frame_kind = VariableFrameKind::Local;
                selected.activation = Some(identity.clone());
                selected
            }
            VariableExecutionFrame::ReceiverMethod { identity } => Self {
                namespace_known: false,
                frame_kind: VariableFrameKind::Local,
                activation: Some(identity.clone()),
                binding_identity: BindingIdentity::Bound,
                ..Self::default()
            },
            VariableExecutionFrame::Selected { selector, .. } if selector.is_current_frame() => {
                return self.clone();
            }
            VariableExecutionFrame::Selected { selector, .. } if selector.is_global_frame() => {
                Self::for_function("::top")
            }
            VariableExecutionFrame::Selected {
                selector,
                namespace,
            } => Self {
                namespace: namespace.clone().unwrap_or_else(|| "::".to_owned()),
                namespace_known: namespace.is_some(),
                selected_frame: Some(*selector),
                frame_kind: VariableFrameKind::Unknown,
                binding_identity: BindingIdentity::Bound,
                ..Self::default()
            },
            VariableExecutionFrame::NamespaceIdentity { .. } => {
                unreachable!("identity wrapper handled above")
            }
            VariableExecutionFrame::Unknown => Self {
                frame_kind: VariableFrameKind::Unknown,
                namespace_known: false,
                dynamic_bindings: true,
                binding_identity: BindingIdentity::Bound,
                ..Self::default()
            },
        };
        self.inherit_namespace_world(&mut selected);
        selected
    }

    fn inherit_namespace_world(&self, selected: &mut Self) {
        let instance_keys = crate::allocated_instance::outward_cell_keys(self);
        let outward = |key: &VariableCellKey| {
            instance_keys.contains(key)
                || match key.root() {
                    VariableCellKey::Namespace { .. } | VariableCellKey::RetainedSlot(_) => true,
                    VariableCellKey::Authored(name) => name.starts_with("::"),
                    _ => false,
                }
        };
        selected
            .namespace_identities
            .clone_from(&self.namespace_identities);
        selected
            .namespace_addressable_identities
            .clone_from(&self.namespace_addressable_identities);
        selected.namespace_inventory = self.namespace_inventory;
        selected.namespace_name_protocol = self.namespace_name_protocol;
        selected.execution_name_policy = self.execution_name_policy;
        selected
            .namespace_objects
            .clone_from(&self.namespace_objects);
        selected
            .pending_namespace_retirements
            .clone_from(&self.pending_namespace_retirements);
        selected
            .authored_tmm_static
            .clone_from(&self.authored_tmm_static);
        if selected.global_frame() {
            selected.namespace_identity = self
                .namespace_identities
                .iter()
                .find(|key| {
                    key.exact_native_path()
                        .is_some_and(tcl_core_types::ByteNamespacePath::is_root)
                })
                .cloned();
        }
        selected
            .closed_observer_allocations
            .clone_from(&self.closed_observer_allocations);
        selected.raw_bindings.clone_from(&self.raw_bindings);
        selected.captured_cells.clone_from(&self.captured_cells);
        selected
            .possible_trace_registrations
            .clone_from(&self.possible_trace_registrations);
        selected
            .namespace_alias_bindings
            .clone_from(&self.namespace_alias_bindings);
        selected
            .namespace_name_alias_bindings
            .clone_from(&self.namespace_name_alias_bindings);
        selected.generations.extend(
            self.generations
                .iter()
                .filter(|(key, _)| outward(key))
                .map(|(key, generation)| (key.clone(), *generation)),
        );
        selected
            .traced
            .extend(self.traced.iter().filter(|key| outward(key)).cloned());
        selected.trace_registrations.extend(
            self.trace_registrations
                .iter()
                .filter(|(key, _)| outward(key))
                .map(|(key, registrations)| (key.clone(), registrations.clone())),
        );
        selected.trace_registration_receivers.extend(
            self.trace_registration_receivers
                .iter()
                .filter(|(key, _)| outward(key))
                .map(|(key, value)| (key.clone(), std::sync::Arc::clone(value))),
        );
        selected.untracked_traces.extend(
            self.untracked_traces
                .iter()
                .filter(|key| outward(key))
                .cloned(),
        );
        selected.dynamic_traces = self.dynamic_traces;
        selected.caller.clone_from(&self.caller);
        selected
            .value_representations
            .clone_from(&self.value_representations);
        selected.representation_epoch = self.representation_epoch;
        selected.representation_epoch_high_water = self.representation_epoch_high_water;
        self.inherit_outward_contents(selected, outward);
    }

    fn inherit_outward_contents(
        &self,
        selected: &mut Self,
        outward: impl Fn(&VariableCellKey) -> bool,
    ) {
        selected.constant_values.extend(
            self.constant_values
                .iter()
                .filter(|(key, _)| outward(key))
                .map(|(key, value)| (key.clone(), value.clone())),
        );
        selected.closed_literal_contents.extend(
            self.closed_literal_contents
                .iter()
                .filter(|(key, _)| outward(key))
                .map(|(key, value)| (key.clone(), value.clone())),
        );
        selected.contents_presence.extend(
            self.contents_presence
                .iter()
                .filter(|(key, _)| outward(key))
                .map(|(key, &presence)| (key.clone(), presence)),
        );
        selected.contents_presence_slots.extend(
            self.contents_presence_slots
                .iter()
                .filter(|(key, _)| outward(key))
                .map(|(key, place)| (key.clone(), std::sync::Arc::clone(place))),
        );
        selected
            .contents_source_proofs
            .restore(&self.contents_source_proofs, &|key| outward(key));
        selected.contents_origins.extend(
            self.contents_origins
                .iter()
                .filter(|(key, _)| outward(key))
                .map(|(key, origin)| (key.clone(), origin.clone())),
        );
        selected.contents_world = self.contents_world;
        selected
            .contents_unknown_namespaces
            .clone_from(&self.contents_unknown_namespaces);
        selected
            .contents_unknown_arrays
            .clone_from(&self.contents_unknown_arrays);
        selected
            .closed_array_roots
            .clone_from(&self.closed_array_roots);
        selected.contents_kinds.clone_from(&self.contents_kinds);
        selected.invocation_dialect = self.invocation_dialect;
        selected.namespace_cells.clone_from(&self.namespace_cells);
        selected
            .pending_namespace_retirements
            .clone_from(&self.pending_namespace_retirements);
        selected.known_namespaces.clone_from(&self.known_namespaces);
        selected
            .namespace_identities
            .clone_from(&self.namespace_identities);
        selected
            .namespace_addressable_identities
            .clone_from(&self.namespace_addressable_identities);
        selected.namespace_inventory = self.namespace_inventory;
        selected
            .authored_tmm_static
            .clone_from(&self.authored_tmm_static);
        selected.interpreter.clone_from(&self.interpreter);
        selected.execution = self.execution;
    }

    /// Enter an actually invoked frame, retaining its selected caller identity.
    #[must_use]
    pub fn enter_called_frame(&self, frame: &VariableExecutionFrame) -> Self {
        let mut selected = self.in_frame(frame);
        if matches!(
            frame.layout(),
            VariableExecutionFrame::Global | VariableExecutionFrame::Selected { .. }
        ) {
            return selected;
        }
        selected.caller = Some(std::sync::Arc::new(self.clone()));
        if matches!(
            frame.layout(),
            VariableExecutionFrame::Procedure { .. }
                | VariableExecutionFrame::ReceiverMethod { .. }
        ) {
            selected.activation_contents_world = Some(ContentsWorld::Tracked);
            selected.activation_observers_closed = true;
        }
        selected.constant_values.clone_from(&self.constant_values);
        selected
            .closed_literal_contents
            .clone_from(&self.closed_literal_contents);
        selected
            .value_representations
            .clone_from(&self.value_representations);
        selected.representation_epoch = self.representation_epoch;
        selected.representation_epoch_high_water = self.representation_epoch_high_water;
        selected.contents_origins.clone_from(&self.contents_origins);
        selected
            .contents_source_proofs
            .clone_from(&self.contents_source_proofs);
        selected
            .contents_presence
            .clone_from(&self.contents_presence);
        selected
            .contents_presence_slots
            .clone_from(&self.contents_presence_slots);
        selected.contents_world = self.contents_world;
        selected
            .contents_unknown_namespaces
            .clone_from(&self.contents_unknown_namespaces);
        selected
            .contents_unknown_arrays
            .clone_from(&self.contents_unknown_arrays);
        selected
            .closed_array_roots
            .clone_from(&self.closed_array_roots);
        selected.contents_kinds.clone_from(&self.contents_kinds);
        selected.generations.clone_from(&self.generations);
        selected.traced.clone_from(&self.traced);
        selected
            .trace_registrations
            .clone_from(&self.trace_registrations);
        selected
            .trace_registration_receivers
            .clone_from(&self.trace_registration_receivers);
        selected.untracked_traces.clone_from(&self.untracked_traces);
        selected
    }

    /// Select a proved actual frame; unknown call depth never invents one.
    #[must_use]
    pub fn selected_frame_context(&self, level: FrameLevel) -> Option<Self> {
        let distance = match level {
            FrameLevel::Relative(distance) => distance,
            FrameLevel::Absolute(target) => {
                let mut current = self;
                let mut depth = 0_u32;
                while !current.global_frame() {
                    current = current.caller.as_deref()?;
                    depth = depth.checked_add(1)?;
                }
                depth.checked_sub(target)?
            }
            FrameLevel::Dynamic => return None,
        };
        let mut current = self;
        for _ in 0..distance {
            if current.global_frame() {
                return None;
            }
            current = current.caller.as_deref()?;
        }
        let mut selected = current.clone();
        selected.captured_cells.clone_from(&self.captured_cells);
        selected
            .possible_trace_registrations
            .clone_from(&self.possible_trace_registrations);
        selected.namespace_cells.clone_from(&self.namespace_cells);
        selected
            .namespace_alias_bindings
            .clone_from(&self.namespace_alias_bindings);
        selected
            .namespace_name_alias_bindings
            .clone_from(&self.namespace_name_alias_bindings);
        selected.known_namespaces.clone_from(&self.known_namespaces);
        selected.generations.extend(
            self.generations
                .iter()
                .map(|(key, generation)| (key.clone(), *generation)),
        );
        selected.constant_values.clone_from(&self.constant_values);
        selected
            .closed_literal_contents
            .clone_from(&self.closed_literal_contents);
        selected
            .value_representations
            .clone_from(&self.value_representations);
        selected.representation_epoch = self.representation_epoch;
        selected.representation_epoch_high_water = self.representation_epoch_high_water;
        selected.contents_origins.clone_from(&self.contents_origins);
        selected
            .contents_source_proofs
            .clone_from(&self.contents_source_proofs);
        selected
            .contents_presence
            .clone_from(&self.contents_presence);
        selected
            .contents_presence_slots
            .clone_from(&self.contents_presence_slots);
        selected.contents_world = self.contents_world;
        selected
            .contents_unknown_namespaces
            .clone_from(&self.contents_unknown_namespaces);
        selected
            .contents_unknown_arrays
            .clone_from(&self.contents_unknown_arrays);
        selected
            .closed_array_roots
            .clone_from(&self.closed_array_roots);
        selected.contents_kinds.clone_from(&self.contents_kinds);
        selected.traced.clone_from(&self.traced);
        selected
            .trace_registrations
            .clone_from(&self.trace_registrations);
        selected
            .trace_registration_receivers
            .clone_from(&self.trace_registration_receivers);
        selected.untracked_traces.clone_from(&self.untracked_traces);
        selected.dynamic_traces = self.dynamic_traces;
        Some(selected)
    }

    /// Last proved contents write to this binding, independently of root recreation.
    #[must_use]
    pub fn contents_origin(&self, place: &Place) -> ContentsOrigin {
        let Some(key) = canonical_binding_value_key(place) else {
            return ContentsOrigin::Unknown;
        };
        self.contents_origins.get(&key).cloned().unwrap_or_else(|| {
            if self.contents_world_unknown_for_key(&key)
                || self.array_contents_unknown(&key)
                || self
                    .contents_unknown_namespaces
                    .iter()
                    .any(|namespace| binding_key_in_namespace(&key, namespace))
            {
                ContentsOrigin::Unknown
            } else {
                ContentsOrigin::Incoming
            }
        })
    }

    /// Contents provenance visible to a read, accounting for implicit native observers.
    #[must_use]
    pub fn read_contents_origin(
        &self,
        place: &Place,
        registry: &CommandRegistry,
    ) -> ContentsOrigin {
        if self.implicit_read_changes_value(place, registry) {
            ContentsOrigin::Unknown
        } else {
            self.contents_origin(place)
        }
    }

    pub(crate) fn implicit_read_changes_value(
        &self,
        place: &Place,
        registry: &CommandRegistry,
    ) -> bool {
        if place.index.is_some() || place.ns != "::" || !matches!(place.kind, PlaceKind::Scalar) {
            return false;
        }
        let dialect = self.invocation_dialect.or_else(|| {
            registry
                .profile()
                .map(tcl_registry::InvocationDialect::of_profile)
        });
        let Some(dialect) = dialect else {
            return false;
        };
        let variable = tcl_syntax::naming::qualify(&place.ns, &place.name);
        tcl_registry::special_vars::SPECIAL_VARS
            .iter()
            .filter_map(|spec| spec.runtime_hook)
            .any(|hook| {
                hook.changes_read_value() && hook.name_in(dialect) == Some(variable.as_str())
            })
    }

    /// Invalidate literal contents selected by a canonical mutation footprint.
    /// Bounded unknown array elements affect only the proved physical root.
    pub fn invalidate_contents_literals(&mut self, place: &Place) {
        if place.observed || (place.kind == PlaceKind::Unknown && place.ns == place::LOCAL_NS) {
            self.retain_literal_values(|_| false);
            self.invalidate_shared_representations();
            return;
        }
        if place.kind == PlaceKind::Unknown {
            let namespace = self.namespace_footprint(place);
            self.retain_literal_values(|key| {
                namespace
                    .as_ref()
                    .is_some_and(|namespace| !key.is_in_namespace(namespace))
            });
            self.value_representations.retain(|key, _| {
                namespace
                    .as_ref()
                    .is_some_and(|namespace| !key.is_in_namespace(namespace))
            });
            return;
        }
        if let Some(key) = canonical_binding_value_key(place) {
            self.forget_literal_value(&key);
            self.value_representations.remove(&key);
        }
        if place.index.is_none()
            || place
                .index
                .as_ref()
                .is_some_and(|index| index.kind != place::IndexKind::Literal)
        {
            let root = physical_array_key(place);
            if let Some(root) = root {
                self.retain_literal_values(|key| !key.is_member_of(&root));
                self.value_representations
                    .retain(|key, _| !key.is_member_of(&root));
            }
        }
        let mut root = place.clone();
        root.index = None;
        root.kind = PlaceKind::Scalar;
        if let Some(key) = canonical_binding_value_key(&root) {
            self.forget_literal_value(&key);
            self.value_representations.remove(&key);
        }
    }

    /// Publish one normal contents store or a typed unresolved write footprint.
    pub fn record_contents_write(&mut self, place: &Place, source: u32, conditional: bool) {
        let previous_presence = self.closed_contents_presence(place);
        self.record_presence_slot(place);
        self.record_contents_source(place, conditional);
        self.record_root_kind(place, conditional);
        self.invalidate_contents_literals(place);
        if place.observed || (place.kind == PlaceKind::Unknown && place.ns == place::LOCAL_NS) {
            self.closed_array_roots.clear();
            self.namespace_cells.closed_namespaces.clear();
            self.contents_world = ContentsWorld::Unknown;
            self.withdraw_activation_contents_closure();
            for origin in self.contents_origins.values_mut() {
                *origin = ContentsOrigin::Unknown;
            }
            for presence in self.contents_presence.values_mut() {
                *presence = ContentsPresence::Unknown;
            }
        } else if place.kind == PlaceKind::Unknown {
            self.closed_array_roots.clear();
            self.namespace_cells.closed_namespaces.clear();
            let Some(namespace) = self.namespace_footprint(place) else {
                self.contents_world = ContentsWorld::Unknown;
                self.withdraw_activation_contents_closure();
                for origin in self.contents_origins.values_mut() {
                    *origin = ContentsOrigin::Unknown;
                }
                for presence in self.contents_presence.values_mut() {
                    *presence = ContentsPresence::Unknown;
                }
                return;
            };
            self.contents_unknown_namespaces.insert(namespace.clone());
            for (name, presence) in &mut self.contents_presence {
                if binding_key_in_namespace(name, &namespace) {
                    *presence = ContentsPresence::Unknown;
                }
            }
            for (name, origin) in &mut self.contents_origins {
                if binding_key_in_namespace(name, &namespace) {
                    *origin = ContentsOrigin::Unknown;
                }
            }
        } else if place.kind == PlaceKind::ArrayWhole
            || place
                .index
                .as_ref()
                .is_some_and(|index| index.kind != place::IndexKind::Literal)
        {
            let Some(root) = physical_array_key(place) else {
                self.contents_world = ContentsWorld::Unknown;
                self.withdraw_activation_contents_closure();
                return;
            };
            self.contents_unknown_arrays.insert(root.clone());
            for (key, origin) in &mut self.contents_origins {
                if key.is_member_of(&root) {
                    *origin = origin.joined(&ContentsOrigin::WrittenAt(source));
                }
            }
            let presence = contents_presence_after_write(previous_presence, conditional);
            self.contents_presence.insert(cell_key(place), presence);
        } else if let Some(key) = canonical_binding_value_key(place) {
            let presence = contents_presence_after_write(previous_presence, conditional);
            self.contents_presence.insert(key.clone(), presence);
            if place.index.is_some() {
                let root = cell_key(place);
                self.contents_presence.insert(root, presence);
            }
            self.contents_origins.insert(
                key,
                if conditional {
                    self.contents_origin(place)
                        .joined(&ContentsOrigin::WrittenAt(source))
                } else {
                    ContentsOrigin::WrittenAt(source)
                },
            );
        }
    }

    /// Record a possible deletion of one unknown element while retaining its array root.
    pub fn record_unknown_element_destruction(&mut self, place: &Place) {
        self.captured_cells
            .destroy(place, Some(RootContentsKind::Array));
        let Some(root) = physical_array_key(place) else {
            self.record_contents_write(&crate::place::unknown_top(), 0, true);
            return;
        };
        self.contents_unknown_arrays.insert(root.clone());
        for (key, origin) in &mut self.contents_origins {
            if key.is_member_of(&root) {
                *origin = ContentsOrigin::Unknown;
            }
        }
        for (key, presence) in &mut self.contents_presence {
            if key.is_member_of(&root) {
                *presence = ContentsPresence::Unknown;
            }
        }
        self.retain_literal_values(|key| !key.is_member_of(&root));
        self.value_representations
            .retain(|key, _| !key.is_member_of(&root));
    }

    /// Read a literal value only from the selected proved binding slot.
    #[must_use]
    pub fn literal_value(&self, variable: &str, registry: &CommandRegistry) -> Option<&str> {
        let access = resolve_literal_access(
            variable,
            self,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        self.literal_contents_at(&access, registry)
    }

    /// Read proved contents of an already selected physical cell. This never
    /// resolves the source spelling again or executes a read observer.
    #[must_use]
    pub fn literal_contents_at<'a>(
        &'a self,
        access: &Place,
        registry: &CommandRegistry,
    ) -> Option<&'a str> {
        self.literal_contents_for_access_at(access, registry, tcl_registry::TraceOperation::Read)
    }

    /// Read stored contents at an already selected operation boundary. A write
    /// result does not execute the implicit read observer used by later getters.
    #[must_use]
    pub fn literal_contents_for_access_at(
        &self,
        access: &Place,
        registry: &CommandRegistry,
        operation: tcl_registry::TraceOperation,
    ) -> Option<&str> {
        if access.observed
            || (operation == tcl_registry::TraceOperation::Read
                && self.implicit_read_changes_value(access, registry))
            || self.contents_presence(access) != ContentsPresence::Defined
        {
            return None;
        }
        if let Some(cell) = &access.cell {
            let current = self
                .generations
                .get(&cell_key(access))
                .copied()
                .unwrap_or_default();
            if cell.generation == CellGeneration::Unknown || cell.generation != current {
                return None;
            }
        }
        let name = canonical_binding_value_key(access)?;
        self.constant_values.get(&name).map(String::as_str)
    }

    /// Resolve a substitution's literal value using its original spelling and read callbacks.
    #[must_use]
    pub fn substitution_literal_value(
        &self,
        spelling: &str,
        registry: &CommandRegistry,
    ) -> Option<&str> {
        let access = resolve_substitution_access(
            spelling,
            self,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        self.literal_contents_at(&access, registry)
    }

    /// Query contents existence without equating allocated cells with defined values.
    #[must_use]
    pub fn contents_presence(&self, place: &Place) -> ContentsPresence {
        if place.observed {
            return ContentsPresence::Unknown;
        }
        self.captured_contents_presence(place)
    }

    /// Actual presence in a captured post-operation world. This does not run a
    /// later observer; callers must retain the successful physical continuation.
    pub(crate) fn captured_contents_presence(&self, place: &Place) -> ContentsPresence {
        let key = canonical_binding_value_key(place);
        if let Some(presence) = key.as_ref().and_then(|key| self.contents_presence.get(key)) {
            return if *presence == ContentsPresence::DefinedOrUndefined {
                ContentsPresence::Unknown
            } else {
                *presence
            };
        }
        if place.index.is_some() {
            let mut root = place.clone();
            root.index = None;
            root.kind = PlaceKind::ArrayWhole;
            if self.captured_contents_presence(&root) == ContentsPresence::Undefined {
                return ContentsPresence::Undefined;
            }
        }
        let Some(key) = key else {
            return ContentsPresence::Unknown;
        };
        if place.is_global() && self.namespace_cells.captured_table_absent(&cell_key(place)) {
            return ContentsPresence::Undefined;
        }
        if place
            .index
            .as_ref()
            .is_some_and(|index| index.kind == place::IndexKind::Literal)
            && physical_array_key(place).is_some_and(|root| self.closed_array_roots.contains(&root))
            && !self.dynamic_bindings
            && !self.array_contents_unknown(&key)
            && !self
                .contents_unknown_namespaces
                .iter()
                .any(|namespace| binding_key_in_namespace(&key, namespace))
        {
            return ContentsPresence::Undefined;
        }
        if self.contents_world_unknown_for_key(&key)
            || self.array_contents_unknown(&key)
            || self
                .contents_unknown_namespaces
                .iter()
                .any(|namespace| binding_key_in_namespace(&key, namespace))
        {
            return ContentsPresence::Unknown;
        }
        if place.is_global() && self.namespace_cells.absent(&cell_key(place)) {
            return ContentsPresence::Undefined;
        }
        if matches!(
            place.cell.as_ref().map(|cell| &cell.owner),
            Some(CellOwner::Activation(identity)) if self.activation.as_ref() == Some(identity)
        ) && place.index.is_none()
            && !self.dynamic_bindings
            && !self.unknown_bindings.contains(&cell_key(place))
        {
            return ContentsPresence::Undefined;
        }
        ContentsPresence::Unknown
    }

    /// Closed contents presence for a purpose-only alternatives query.
    /// A mixed result licenses a possible missing-read diagnostic, never a
    /// value version or a definite native error. Opaque worlds remain absent.
    #[must_use]
    pub fn closed_contents_presence(&self, place: &Place) -> Option<ContentsPresence> {
        if place.observed {
            return None;
        }
        let presence = canonical_binding_value_key(place)
            .and_then(|key| self.contents_presence.get(&key).copied())
            .unwrap_or_else(|| self.contents_presence(place));
        (presence != ContentsPresence::Unknown).then_some(presence)
    }

    /// Query the originally selected cell after the native existence read boundary.
    /// Unknown callbacks and lifetimes cannot become a Boolean value proof.
    #[must_use]
    pub fn existence_after_read_at(
        &self,
        place: &Place,
        registry: &CommandRegistry,
    ) -> Option<bool> {
        if place.observed || self.implicit_read_changes_value(place, registry) {
            return None;
        }
        let cell = place.cell.as_ref()?;
        let current = self
            .generations
            .get(&cell_key(place))
            .copied()
            .unwrap_or_default();
        if cell.generation == CellGeneration::Unknown || cell.generation != current {
            return None;
        }
        match self.contents_presence(place) {
            ContentsPresence::Defined => Some(true),
            ContentsPresence::Undefined => Some(false),
            ContentsPresence::Unknown | ContentsPresence::DefinedOrUndefined => None,
        }
    }

    /// Prove a missing unobserved read without mistaking unknown values for absence.
    /// Defined array roots still need a selected scalar/array operation to prove success.
    #[must_use]
    pub fn read_completion(&self, place: &Place) -> VariableReadCompletion {
        if self.contents_presence(place) == ContentsPresence::Undefined
            || self.store_would_error(place)
        {
            VariableReadCompletion::Error
        } else {
            VariableReadCompletion::Unknown
        }
    }

    // A loop can lose the incarnation of a direct local slot while a later
    // successful scalar store still defines its current lookup contents. This
    // does not identify a captured cell or an element in a retired container.
    fn current_scalar_slot_contents(&self, place: &Place) -> bool {
        !self.dynamic_bindings
            && !place.dynamic
            && !place.observed
            && place.kind == PlaceKind::Scalar
            && place.index.is_none()
            && place.cell.as_ref().is_some_and(|cell| {
                cell.generation == CellGeneration::Unknown
                    && matches!(&cell.owner, CellOwner::Activation(identity)
                        if self.activation.as_ref() == Some(identity))
                    && self.generations.get(&cell_key(place)) == Some(&CellGeneration::Unknown)
            })
            && self.contents_presence(place) == ContentsPresence::Defined
            && matches!(self.contents_origin(place), ContentsOrigin::WrittenAt(_))
            && self.contents_kinds.get(&cell_key(place)) == Some(&RootContentsKind::Scalar)
    }

    fn current_contents_generation(&self, place: &Place) -> bool {
        place.cell.as_ref().is_some_and(|cell| {
            (cell.generation != CellGeneration::Unknown
                && cell.generation
                    == self
                        .generations
                        .get(&cell_key(place))
                        .copied()
                        .unwrap_or_default())
                || self.current_scalar_slot_contents(place)
        })
    }

    /// A missing selected value before an unobserved native read. This requires
    /// a current physical address and closed absence/observer facts; unknown
    /// tables, dynamic selections and implicit producers retain alternatives.
    #[must_use]
    pub(crate) fn read_is_definitely_missing(
        &self,
        place: &Place,
        registry: &CommandRegistry,
    ) -> bool {
        if self
            .invocation_dialect
            .is_none_or(|dialect| dialect.variable_lookup_policy.is_none())
            || place.observed
            || place.dynamic
            || place
                .cell
                .as_ref()
                .is_none_or(|cell| matches!(cell.owner, CellOwner::SelectedFrame(_)))
            || !self.current_contents_generation(place)
            || self.contents_presence(place) != ContentsPresence::Undefined
            || self.implicit_read_changes_value(place, registry)
        {
            return false;
        }
        let observers =
            self.variable_observers_at(place, tcl_registry::TraceOperation::Read, registry);
        !observers.unknown_residual
            && observers.callbacks.is_empty()
            && observers.possible_callbacks.is_empty()
    }

    /// Whether this already selected native read is guaranteed to produce a
    /// value before any observer. Presence and SSA addressability alone cannot
    /// prove scalar/array compatibility or dictionary element lookup.
    #[must_use]
    pub fn read_produces_value(&self, place: &Place, registry: &CommandRegistry) -> bool {
        use tcl_dialect::VariableContainerModel;
        let Some(dialect) = self.invocation_dialect else {
            return false;
        };
        if dialect.variable_lookup_policy.is_none()
            || place.observed
            || place.dynamic
            || place
                .cell
                .as_ref()
                .is_none_or(|cell| matches!(cell.owner, CellOwner::SelectedFrame(_)))
            || self.contents_presence(place) != ContentsPresence::Defined
            || self.implicit_read_changes_value(place, registry)
        {
            return false;
        }
        if !self.current_contents_generation(place) {
            return false;
        }
        let observers =
            self.variable_observers_at(place, tcl_registry::TraceOperation::Read, registry);
        if observers.unknown_residual
            || !observers.callbacks.is_empty()
            || !observers.possible_callbacks.is_empty()
        {
            return false;
        }
        match (dialect.variable_container_model, place.kind) {
            (Some(VariableContainerModel::DistinctArray), PlaceKind::Scalar) => {
                place.index.is_none()
                    && self.root_contents_kind(place) == Some(RootContentsKind::Scalar)
            }
            (Some(VariableContainerModel::DistinctArray), PlaceKind::ArrayElem) => {
                self.root_contents_kind(place) == Some(RootContentsKind::Array)
                    && place
                        .index
                        .as_ref()
                        .is_some_and(|index| index.kind == IndexKind::Literal)
            }
            (Some(VariableContainerModel::DictionaryValue), PlaceKind::Scalar) => {
                place.index.is_none()
            }
            (Some(VariableContainerModel::DictionaryValue), PlaceKind::ArrayElem) => {
                self.dictionary_element_read_produces_value(place, registry, dialect)
            }
            _ => false,
        }
    }

    fn dictionary_element_read_produces_value(
        &self,
        place: &Place,
        registry: &CommandRegistry,
        dialect: tcl_registry::InvocationDialect,
    ) -> bool {
        let Some(index) = place
            .index
            .as_ref()
            .filter(|index| index.kind == IndexKind::Literal)
        else {
            return false;
        };
        let mut root = place.base();
        root.kind = PlaceKind::Scalar;
        if let Some(value) = self.literal_contents_at(&root, registry) {
            return dialect.word_values.split_list(value).is_ok_and(|items| {
                items.len().is_multiple_of(2)
                    && items
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .any(|pair| pair[0] == index.value)
            });
        }
        matches!(self.array_contents_inventory(&place.base()),
            crate::array_destruction::ArrayContentsInventory::Known(members)
                if members.iter().any(|member| member.cell == place.cell && member.index == place.index))
    }

    /// Define one proven literal parameter or successfully stored value.
    pub fn define_literal(&mut self, variable: &str, value: &str, registry: &CommandRegistry) {
        let target = resolve_literal_place(variable, self, false, registry);
        if let Some(name) = canonical_binding_value_key(&target) {
            self.publish_defined_slot(&target, &name);
            self.value_representations.remove(&name);
            self.store_literal_value(name, value.to_owned());
        }
    }

    /// Withdraw namespace value and lifetime facts at an externally callable
    /// deferred entry. Fresh local slots and formal ingress remain independent;
    /// actual calls must retain their caller's authoritative namespace world.
    pub fn widen_deferred_namespace_inputs(&mut self) {
        self.closed_array_roots
            .retain(|key| !key.is_namespace_storage());
        self.namespace_cells.widen();
        self.contents_unknown_namespaces
            .extend(self.namespace_identities.iter().cloned());
        self.contents_unknown_namespaces.insert("::".to_owned());
        self.retain_literal_values(|key| !key.is_namespace_storage());
        self.value_representations
            .retain(|key, _| !key.is_namespace_storage());
        self.contents_kinds
            .retain(|key, _| !key.is_namespace_storage());
        for (key, origin) in &mut self.contents_origins {
            if key.is_namespace_storage() {
                *origin = ContentsOrigin::Unknown;
            }
        }
        for (key, presence) in &mut self.contents_presence {
            if key.is_namespace_storage() {
                *presence = ContentsPresence::Unknown;
            }
        }
        for (key, generation) in &mut self.generations {
            if key.is_namespace_storage() {
                *generation = CellGeneration::Unknown;
            }
        }
        for target in self
            .alias_bindings
            .values_mut()
            .chain(self.namespace_alias_bindings.values_mut())
        {
            if let Some(cell) = &mut target.cell
                && matches!(
                    cell.owner,
                    CellOwner::Namespace(_) | CellOwner::NamespaceIdentity(_)
                )
            {
                cell.generation = CellGeneration::Unknown;
            }
        }
    }

    /// Capture one active receiver while its physical owner remains distinguishable.
    /// Repeated recursive activations sharing one abstract allocation family do
    /// not establish that a later destruction addresses the same instance.
    pub fn capture_protected_cell(
        &mut self,
        receiver: &Place,
    ) -> Option<crate::captured_cell::CapturedCellLeaseId> {
        let owner = &receiver.cell.as_ref()?.owner;
        match owner {
            CellOwner::Namespace(_) | CellOwner::NamespaceIdentity(_) => {}
            CellOwner::AllocatedInstance(allocation)
                if allocation.incarnation
                    != crate::command_binding::AllocationIncarnation::RepeatedFresh => {}
            CellOwner::Activation(identity) => {
                let mut occurrences = 0;
                let mut frame = Some(&*self);
                while let Some(current) = frame {
                    occurrences += usize::from(current.activation.as_ref() == Some(identity));
                    frame = current.caller.as_deref();
                }
                if occurrences != 1 {
                    return None;
                }
            }
            CellOwner::RetainedSlot(_) if self.raw_bindings.active_array_member(receiver) => {}
            _ => return None,
        }
        self.captured_cells.capture(receiver)
    }

    /// Select a protected live shell after callbacks, without consulting its former alias.
    /// A contents reset may refresh scalar contents generation; retired owners never revive.
    #[must_use]
    pub fn captured_cell_receiver(
        &self,
        id: crate::captured_cell::CapturedCellLeaseId,
    ) -> Option<Place> {
        use crate::captured_cell::CapturedCellState;
        if !matches!(
            self.captured_cells.state(id),
            CapturedCellState::Live | CapturedCellState::ContentsReset
        ) {
            return None;
        }
        let mut receiver = self.captured_cells.receiver(id)?.clone();
        let generation = self
            .generations
            .get(&cell_key(&receiver))
            .copied()
            .unwrap_or_default();
        if generation == CellGeneration::Unknown {
            return None;
        }
        if receiver.index.is_none()
            && let Some(cell) = &mut receiver.cell
        {
            cell.generation = generation;
        }
        receiver.observed = false;
        Some(receiver)
    }

    /// Conversion hooks of contents selected by a completed protected read.
    /// This checks the retained physical shell and current contents generation,
    /// without asking whether a future read would execute observers again.
    #[must_use]
    pub(crate) fn captured_numeric_input_hooks_closed(
        &self,
        id: crate::captured_cell::CapturedCellLeaseId,
    ) -> bool {
        self.captured_cell_receiver(id).is_some_and(|receiver| {
            self.contents_presence(&receiver) == ContentsPresence::Defined
                && self.current_contents_generation(&receiver)
                && self.selected_contents_numeric_hooks_closed(&receiver)
        })
    }

    /// Publish a successful native store to the already captured physical receiver.
    /// The caller owns observer delivery after this store; this does not resolve
    /// a variable spelling or invoke a callback.
    pub(crate) fn publish_captured_store(
        &mut self,
        receiver: &Place,
        value: Option<&str>,
        source: u32,
    ) {
        let mut stored = receiver.clone();
        stored.observed = false;
        let mut root = stored.clone();
        root.index = None;
        root.kind = PlaceKind::ArrayWhole;
        let created_array = self
            .invocation_dialect
            .and_then(|dialect| dialect.variable_container_model)
            == Some(tcl_dialect::VariableContainerModel::DistinctArray)
            && stored.index.as_ref().is_some_and(|index| {
                index.kind == place::IndexKind::Literal
                    && self.contents_presence(&root) == ContentsPresence::Undefined
            });
        self.record_contents_write(&stored, source, false);
        if created_array && let Some(root) = physical_array_key(&root) {
            self.closed_array_roots.insert(root);
        }
        if let Some(name) = canonical_binding_value_key(&stored) {
            self.publish_defined_slot(&stored, &name);
            if let Some(value) = value {
                self.store_literal_value(name, value.to_owned());
            }
        }
    }

    pub(crate) fn record_presence_slot(&mut self, place: &Place) {
        if let Some(key) = canonical_binding_value_key(place) {
            self.contents_presence_slots
                .insert(key, std::sync::Arc::new(place.clone()));
        }
        if place.index.is_some() {
            let mut root = place.clone();
            root.index = None;
            root.kind = PlaceKind::ArrayWhole;
            if let Some(key) = canonical_binding_value_key(&root) {
                self.contents_presence_slots
                    .insert(key, std::sync::Arc::new(root));
            }
        }
    }

    fn publish_defined_slot(&mut self, target: &Place, name: &VariableCellKey) {
        self.record_presence_slot(target);
        self.record_root_kind(target, false);
        self.contents_presence
            .insert(name.to_owned(), ContentsPresence::Defined);
        if target.index.is_some() {
            self.contents_presence
                .insert(cell_key(target), ContentsPresence::Defined);
        }
        if target.is_global() && !target.dynamic {
            self.namespace_cells.present.insert(cell_key(target));
        }
    }

    /// Root kind of the same selected live physical cell, before observers run.
    #[must_use]
    pub fn root_contents_kind(&self, place: &Place) -> Option<RootContentsKind> {
        place.cell.as_ref()?;
        let key = cell_key(place);
        if place.observed || !self.current_contents_generation(place) {
            return None;
        }
        self.contents_kinds.get(&key).copied()
    }

    /// Whether an ordinary scalar/element store is proved to fail before writing.
    /// Unknown root kinds and observers cannot donate an error-only route.
    #[must_use]
    pub fn store_would_error(&self, place: &Place) -> bool {
        if let Some(index) = &place.index
            && let Some(CellOwner::RetainedSlot(identity)) =
                place.cell.as_ref().map(|cell| &cell.owner)
            && self
                .raw_bindings
                .retired_array_members
                .get(&identity.storage_key())
                .is_some_and(|members| members.contains(&index.value))
        {
            return true;
        }
        if self
            .invocation_dialect
            .and_then(|dialect| dialect.variable_container_model)
            != Some(tcl_dialect::VariableContainerModel::DistinctArray)
        {
            return false;
        }
        matches!(
            (place.kind, self.root_contents_kind(place)),
            (PlaceKind::Scalar, Some(RootContentsKind::Array))
                | (PlaceKind::ArrayElem, Some(RootContentsKind::Scalar))
        )
    }

    fn record_root_kind(&mut self, place: &Place, conditional: bool) {
        if place.observed || (place.kind == PlaceKind::Unknown && place.ns == place::LOCAL_NS) {
            self.contents_kinds.clear();
            return;
        }
        if place.kind == PlaceKind::Unknown {
            let namespace = self.namespace_footprint(place);
            self.contents_kinds.retain(|key, _| {
                namespace
                    .as_ref()
                    .is_some_and(|namespace| !key.is_in_namespace(namespace))
            });
            return;
        }
        let kind = match place.kind {
            PlaceKind::Scalar | PlaceKind::DictPath => RootContentsKind::Scalar,
            PlaceKind::ArrayElem | PlaceKind::ArrayWhole => RootContentsKind::Array,
            _ => return,
        };
        let key = cell_key(place);
        if conditional {
            if self.contents_kinds.get(&key) != Some(&kind) {
                self.contents_kinds.remove(&key);
            }
        } else {
            self.contents_kinds.insert(key, kind);
        }
    }

    /// Store known text with independently proved native representation.
    pub fn define_literal_with_representation(
        &mut self,
        variable: &str,
        value: &str,
        representation: tcl_syntax::value::ValueRepresentation,
        registry: &CommandRegistry,
    ) {
        self.define_literal(variable, value, registry);
        if let Some(name) = canonical_literal_variable_key(variable, self, registry) {
            self.value_representations
                .insert(name, representation.into());
        }
    }

    /// Representation of the same selected, currently defined physical value.
    #[must_use]
    pub fn contents_representation_at(
        &self,
        place: &Place,
    ) -> tcl_syntax::value::ValueRepresentation {
        let valid = !place.observed
            && self.contents_presence(place) == ContentsPresence::Defined
            && self.current_contents_generation(place);
        if !valid {
            return tcl_syntax::value::ValueRepresentation::Unknown;
        }
        canonical_binding_value_key(place)
            .and_then(|name| self.value_representations.get(&name))
            .map(crate::native_numeric::StoredNativeRepresentation::representation)
            .unwrap_or_default()
    }

    /// Closed possible ordinary container representations at this live read.
    /// This cost-advice projection never upgrades the strict representation query.
    pub(crate) fn container_representation_alternatives_at(
        &self,
        place: &Place,
    ) -> Option<crate::native_numeric::ClosedContainerRepresentations> {
        if place.observed
            || place.dynamic
            || self.contents_presence(place) != ContentsPresence::Defined
            || !self.current_contents_generation(place)
        {
            return None;
        }
        self.value_representations
            .get(&canonical_binding_value_key(place)?)?
            .container_alternatives()
    }

    pub(crate) fn contents_list_method_provider_at(
        &self,
        place: &Place,
        registry: &CommandRegistry,
    ) -> Option<tcl_registry::native_result::NativeListMethodProvider> {
        if !self.read_produces_value(place, registry) {
            return None;
        }
        match self
            .value_representations
            .get(&canonical_binding_value_key(place)?)?
        {
            crate::native_numeric::StoredNativeRepresentation::ReadOnlyListMethodProvider(
                provider,
            ) => Some(*provider),
            _ => None,
        }
    }

    /// Allocate a definitely bound parameter or store whose contents are unknown.
    /// This retains its cell address without inventing a literal value.
    pub fn define_unknown_contents(&mut self, variable: &str, registry: &CommandRegistry) {
        let target = resolve_literal_access(
            variable,
            self,
            false,
            registry,
            tcl_registry::TraceOperation::Write,
        );
        let Some(name) = canonical_binding_value_key(&target) else {
            return;
        };
        self.forget_literal_value(&name);
        self.value_representations.remove(&name);
        self.publish_defined_slot(&target, &name);
        self.contents_origins.insert(name, ContentsOrigin::Unknown);
    }

    /// Bind an unknown ordinary incoming activation value.
    /// Only the current physical activation's direct slot receives incoming
    /// provenance. Static, namespace, foreign and observed aliases retain unknown
    /// store provenance; this does not turn later unknown writes into live-ins.
    pub fn bind_unknown_incoming(&mut self, variable: &str, registry: &CommandRegistry) {
        self.define_unknown_contents(variable, registry);
        self.record_incoming_binding_origin(variable, registry);
    }

    /// Prove that an authored substitution reads this activation's incoming slot.
    /// The caller validates the ordinary formal binding plan separately. This
    /// preserves the physical cell and rejects later writes, aliases to other
    /// storage, uncertain observers and missing values.
    #[must_use]
    pub fn incoming_activation_slot(
        &self,
        spelling: &str,
        slot: &str,
        registry: &CommandRegistry,
    ) -> Option<CellIdentity> {
        let place = resolve_substitution_access(
            spelling,
            self,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        let cell = place.cell.as_ref()?;
        (!place.dynamic
            && !place.observed
            && place.kind == PlaceKind::Scalar
            && place.index.is_none()
            && place.name == slot
            && cell.name == slot
            && cell.generation != CellGeneration::Unknown
            && matches!(&cell.owner, CellOwner::Activation(identity)
                if self.activation.as_ref() == Some(identity))
            && self.contents_presence(&place) == ContentsPresence::Defined
            && self.read_contents_origin(&place, registry) == ContentsOrigin::Incoming)
            .then(|| cell.clone())
    }

    /// Bind a literal ordinary incoming value without inheriting an older store origin.
    /// A formal that writes static or foreign storage retains the literal bytes
    /// but cannot represent that write as this activation's SSA live-in.
    pub fn bind_literal_incoming(
        &mut self,
        variable: &str,
        value: &str,
        registry: &CommandRegistry,
    ) {
        self.define_literal(variable, value, registry);
        self.record_incoming_binding_origin(variable, registry);
    }

    fn record_incoming_binding_origin(&mut self, variable: &str, registry: &CommandRegistry) {
        let target = resolve_literal_access(
            variable,
            self,
            false,
            registry,
            tcl_registry::TraceOperation::Write,
        );
        let Some(name) = canonical_binding_value_key(&target) else {
            return;
        };
        let incoming = !target.dynamic
            && !target.observed
            && target.kind == PlaceKind::Scalar
            && target.index.is_none()
            && matches!(
                target.cell.as_ref().map(|cell| &cell.owner),
                Some(CellOwner::Activation(identity)) if self.activation.as_ref() == Some(identity)
            );
        self.contents_origins.insert(
            name,
            if incoming {
                ContentsOrigin::Incoming
            } else {
                ContentsOrigin::Unknown
            },
        );
    }

    /// Define a bound value with known representation and unknown string contents.
    pub fn define_unknown_contents_with_representation(
        &mut self,
        variable: &str,
        representation: tcl_syntax::value::ValueRepresentation,
        registry: &CommandRegistry,
    ) {
        self.define_unknown_contents(variable, registry);
        if let Some(name) = canonical_literal_variable_key(variable, self, registry) {
            self.value_representations
                .insert(name, representation.into());
        }
    }

    /// Join incoming binding states. A binding is precise only when all
    /// executable predecessors agree on the same immutable cell address.
    pub fn join(&mut self, other: &Self) {
        let integer_contents = self.joined_integer_contents(other);
        self.join_literal_contents(other);
        // Differing issuance histories at a join have no bounded next stamp.
        // In particular, abstract loop iterations must not mint an unbounded
        // succession of receipts for one repeated allocation family.
        if self.representation_epoch_high_water != other.representation_epoch_high_water {
            self.representation_epoch_high_water = None;
            self.representation_epoch = None;
        }
        if self.representation_epoch != other.representation_epoch {
            self.representation_epoch = None;
        }
        self.join_alias_bindings(other);
        self.outward_namespace_destructions
            .extend(other.outward_namespace_destructions.iter().cloned());
        self.raw_bindings.join(&other.raw_bindings);
        self.captured_cells.join(&other.captured_cells);
        self.dictionary_scopes
            .retain(|source, activation| other.dictionary_scopes.get(source) == Some(activation));
        self.contents_kinds
            .retain(|key, kind| other.contents_kinds.get(key) == Some(kind));
        self.join_contents_presence(other);
        for (key, slot) in &other.contents_presence_slots {
            self.contents_presence_slots
                .entry(key.clone())
                .or_insert_with(|| std::sync::Arc::clone(slot));
        }
        self.join_contents_origins(other);
        self.constant_values
            .retain(|name, value| other.constant_values.get(name) == Some(value));
        self.join_value_representations(other, &integer_contents);
        match (&self.caller, &other.caller) {
            (Some(left), Some(right)) if left != right => {
                let mut joined = (**left).clone();
                joined.join(right);
                self.caller = Some(std::sync::Arc::new(joined));
            }
            (Some(_), None) => self.caller = None,
            _ => {}
        }
        self.unknown_bindings
            .extend(other.unknown_bindings.iter().cloned());
        if self.interpreter != other.interpreter {
            self.interpreter = None;
        }
        if self.execution != other.execution {
            self.execution = None;
        }
        if self.activation != other.activation || self.selected_frame != other.selected_frame {
            self.activation = None;
            self.selected_frame = None;
            self.dynamic_bindings = true;
        }
        self.join_namespace_and_observer_world(other);
    }

    fn join_namespace_and_observer_world(&mut self, other: &Self) {
        self.namespace_known &= other.namespace_known
            && self.namespace == other.namespace
            && self.namespace_identity == other.namespace_identity;
        if self.namespace_identity != other.namespace_identity {
            self.namespace_identity = None;
        }
        self.namespace_identities
            .retain(|identity| other.namespace_identities.contains(identity));
        self.namespace_inventory = self.namespace_inventory.joined(
            other.namespace_inventory,
            self.namespace_addressable_identities == other.namespace_addressable_identities,
        );
        self.namespace_addressable_identities
            .retain(|identity| other.namespace_addressable_identities.contains(identity));
        if self.namespace_name_protocol != other.namespace_name_protocol {
            self.namespace_name_protocol = None;
        }
        if self.execution_name_policy != other.execution_name_policy {
            self.execution_name_policy = None;
        }
        self.namespace_objects
            .retain(|key, object| other.namespace_objects.get(key) == Some(object));
        if self.authored_tmm_static != other.authored_tmm_static {
            self.authored_tmm_static = None;
            self.mark_unenumerated_variable_observers();
        }
        for (key, pending) in &mut self.pending_namespace_retirements {
            if other.pending_namespace_retirements.get(key) != Some(pending) {
                pending.conditional = true;
            }
        }
        for (key, pending) in &other.pending_namespace_retirements {
            self.pending_namespace_retirements
                .entry(key.clone())
                .or_insert_with(|| {
                    let mut pending = pending.clone();
                    pending.conditional = true;
                    pending
                });
        }
        self.namespace_cells.join(&other.namespace_cells);
        self.dynamic_bindings |= other.dynamic_bindings;
        self.dynamic_traces |= other.dynamic_traces;
        self.join_trace_registrations(other);
        for registration in &other.possible_trace_registrations {
            let required = other
                .possible_trace_registrations
                .iter()
                .filter(|item| *item == registration)
                .count();
            let present = self
                .possible_trace_registrations
                .iter()
                .filter(|item| *item == registration)
                .count();
            self.possible_trace_registrations
                .extend(std::iter::repeat_n(
                    registration.clone(),
                    required.saturating_sub(present),
                ));
        }
        self.known_namespaces
            .retain(|name| other.known_namespaces.contains(name));
        let keys: HashSet<_> = self
            .generations
            .keys()
            .chain(other.generations.keys())
            .cloned()
            .collect();
        for key in keys {
            let mine = self.generations.get(&key).copied().unwrap_or_default();
            let theirs = other.generations.get(&key).copied().unwrap_or_default();
            if mine != theirs {
                self.generations.insert(key, CellGeneration::Unknown);
            }
        }
    }

    fn joined_integer_contents(
        &self,
        other: &Self,
    ) -> VariableCellTable<crate::native_numeric::SourceIntegerConvertibleContents> {
        // Contents/conversion alternatives use each predecessor's still-live
        // bytes and sealed stock/numeric provenance, before value intersection.
        self.value_representations
            .iter()
            .filter_map(|(key, left)| {
                let right = other.value_representations.get(key)?;
                let left_proof = left.integer_contents_in(
                    self.constant_values.get(key).map(String::as_str),
                    self.invocation_dialect,
                )?;
                let right_proof = right.integer_contents_in(
                    other.constant_values.get(key).map(String::as_str),
                    other.invocation_dialect,
                )?;
                (left_proof == right_proof).then(|| (key.clone(), left_proof))
            })
            .collect()
    }

    fn join_value_representations(
        &mut self,
        other: &Self,
        integer_contents: &VariableCellTable<
            crate::native_numeric::SourceIntegerConvertibleContents,
        >,
    ) {
        self.value_representations.retain(|name, value| {
            let Some(joined) = other
                .value_representations
                .get(name)
                .and_then(|right| {
                    if value == right
                        && (!matches!(value, crate::native_numeric::StoredNativeRepresentation::StockLiteralObject(_))
                            || self.constant_values.contains_key(name)) {
                        value.joined(right)
                    } else if let Some(joined) = value.joined_current_numeric(right) {
                        Some(joined)
                    } else if let Some(proof) = integer_contents.get(name) {
                        Some(crate::native_numeric::StoredNativeRepresentation::IntegerConvertibleContents(*proof))
                    } else {
                        value.joined(right)
                    }
                })
            else {
                return false;
            };
            *value = joined;
            true
        });
    }

    fn join_contents_presence(&mut self, other: &Self) {
        let keys: HashSet<_> = self
            .contents_presence
            .keys()
            .chain(other.contents_presence.keys())
            .cloned()
            .collect();
        for key in keys {
            let left = self
                .contents_presence
                .get(&key)
                .copied()
                .unwrap_or_else(|| {
                    other
                        .contents_presence_slots
                        .get(&key)
                        .map_or(ContentsPresence::Unknown, |slot| {
                            self.contents_presence(slot)
                        })
                });
            let right = other
                .contents_presence
                .get(&key)
                .copied()
                .unwrap_or_else(|| {
                    self.contents_presence_slots
                        .get(&key)
                        .map_or(ContentsPresence::Unknown, |slot| {
                            other.contents_presence(slot)
                        })
                });
            self.contents_presence.insert(
                key,
                if left == right {
                    left
                } else if left != ContentsPresence::Unknown && right != ContentsPresence::Unknown {
                    ContentsPresence::DefinedOrUndefined
                } else {
                    ContentsPresence::Unknown
                },
            );
        }
    }

    fn array_contents_unknown(&self, key: &VariableCellKey) -> bool {
        self.contents_unknown_arrays
            .iter()
            .any(|root| key.is_member_of(root))
    }

    fn contents_world_unknown_for_key(&self, key: &VariableCellKey) -> bool {
        if let VariableCellKey::Activation { identity, .. } = key.root()
            && self.activation.as_ref() == Some(identity)
            && let Some(world) = self.activation_contents_world
        {
            return world.is_unknown();
        }
        self.contents_world.is_unknown()
    }

    fn withdraw_activation_contents_closure(&mut self) {
        self.activation_observers_closed = false;
        self.closed_observer_allocations.clear();
        if self.activation_contents_world.is_some() {
            self.activation_contents_world = Some(ContentsWorld::Unknown);
        }
    }

    fn incoming_contents_origin(&self, key: &VariableCellKey) -> ContentsOrigin {
        if self.contents_world_unknown_for_key(key)
            || self.array_contents_unknown(key)
            || self
                .contents_unknown_namespaces
                .iter()
                .any(|namespace| binding_key_in_namespace(key, namespace))
        {
            ContentsOrigin::Unknown
        } else {
            ContentsOrigin::Incoming
        }
    }

    fn join_contents_origins(&mut self, other: &Self) {
        self.contents_source_proofs
            .joined(&other.contents_source_proofs);
        let names: HashSet<_> = self
            .contents_origins
            .keys()
            .chain(other.contents_origins.keys())
            .cloned()
            .collect();
        for name in names {
            let left = self
                .contents_origins
                .get(&name)
                .cloned()
                .unwrap_or_else(|| self.incoming_contents_origin(&name));
            let right = other
                .contents_origins
                .get(&name)
                .cloned()
                .unwrap_or_else(|| other.incoming_contents_origin(&name));
            self.contents_origins.insert(name, left.joined(&right));
        }
        self.closed_array_roots
            .retain(|key| other.closed_array_roots.contains(key));
        self.contents_world = self.contents_world.joined(other.contents_world);
        self.activation_contents_world = match (
            self.activation_contents_world,
            other.activation_contents_world,
        ) {
            (Some(left), Some(right)) => Some(left.joined(right)),
            _ => None,
        };
        self.activation_observers_closed &=
            other.activation_observers_closed && self.activation == other.activation;
        self.closed_observer_allocations
            .retain(|allocation| other.closed_observer_allocations.contains(allocation));
        self.contents_unknown_namespaces
            .extend(other.contents_unknown_namespaces.iter().cloned());
        self.contents_unknown_arrays
            .extend(other.contents_unknown_arrays.iter().cloned());
    }

    fn join_alias_bindings(&mut self, other: &Self) {
        let keys: HashSet<_> = self
            .alias_bindings
            .keys()
            .chain(other.alias_bindings.keys())
            .cloned()
            .collect();
        for key in keys {
            if self.name_alias_bindings.contains_key(&key)
                && self.name_alias_bindings.get(&key) == other.name_alias_bindings.get(&key)
            {
                continue;
            }
            if self.alias_bindings.get(&key) != other.alias_bindings.get(&key) {
                if let Some(joined) = self
                    .alias_bindings
                    .get(&key)
                    .zip(other.alias_bindings.get(&key))
                    .and_then(|(left, right)| join_alias_target(left, right))
                {
                    self.alias_bindings.insert(key, joined);
                } else {
                    self.alias_bindings.remove(&key);
                    self.unknown_bindings.insert(key);
                }
            }
        }
        self.join_name_aliases(other);
        let namespaces: HashSet<_> = self
            .namespace_alias_bindings
            .keys()
            .chain(other.namespace_alias_bindings.keys())
            .cloned()
            .collect();
        for name in namespaces {
            if self.namespace_name_alias_bindings.contains_key(&name)
                && self.namespace_name_alias_bindings.get(&name)
                    == other.namespace_name_alias_bindings.get(&name)
            {
                continue;
            }
            if self.namespace_alias_bindings.get(&name) != other.namespace_alias_bindings.get(&name)
            {
                let joined = self
                    .namespace_alias_bindings
                    .get(&name)
                    .zip(other.namespace_alias_bindings.get(&name))
                    .and_then(|(left, right)| join_alias_target(left, right))
                    .unwrap_or_else(place::unknown_top);
                self.namespace_alias_bindings.insert(name, joined);
            }
        }
    }

    fn join_name_aliases(&mut self, other: &Self) {
        let named: HashSet<_> = self
            .name_alias_bindings
            .keys()
            .chain(other.name_alias_bindings.keys())
            .cloned()
            .collect();
        for name in named {
            if self.name_alias_bindings.get(&name) != other.name_alias_bindings.get(&name) {
                self.name_alias_bindings.remove(&name);
                self.unknown_bindings.insert(name);
            }
        }
        let namespace_names: HashSet<_> = self
            .namespace_name_alias_bindings
            .keys()
            .chain(other.namespace_name_alias_bindings.keys())
            .cloned()
            .collect();
        for name in namespace_names {
            if self.namespace_name_alias_bindings.get(&name)
                != other.namespace_name_alias_bindings.get(&name)
            {
                self.namespace_name_alias_bindings
                    .insert(name, place::unknown_top());
            }
        }
    }

    fn join_trace_registrations(&mut self, other: &Self) {
        self.trace_registration_receivers
            .retain(|key, receiver| other.trace_registration_receivers.get(key) == Some(receiver));
        for target in &self.traced {
            if !self.trace_registrations.contains_key(target) {
                self.untracked_traces.insert(target.clone());
            }
        }
        for target in &other.traced {
            if !other.trace_registrations.contains_key(target) {
                self.untracked_traces.insert(target.clone());
            }
        }
        self.traced.extend(other.traced.iter().cloned());
        self.untracked_traces
            .extend(other.untracked_traces.iter().cloned());
        for (target, registrations) in &other.trace_registrations {
            let reaching = self.trace_registrations.entry(target.clone()).or_default();
            for registration in registrations {
                let required = registrations
                    .iter()
                    .filter(|item| *item == registration)
                    .count();
                let present = reaching.iter().filter(|item| *item == registration).count();
                reaching.extend(std::iter::repeat_n(
                    registration.clone(),
                    required.saturating_sub(present),
                ));
            }
        }
    }

    /// Withdraw native representation proofs after an operation may coerce shared objects.
    /// String contents, variable bindings and cell lifetimes remain authoritative.
    pub fn invalidate_shared_representations(&mut self) {
        self.invalidate_representations_for_values(None);
    }

    /// Objects with different current bytes cannot be the same native object.
    /// Unknown operand bytes withdraw every representation. Exact contents or
    /// an actual normalised numeric result-string recipe can prove disjointness.
    /// Numeric categories alone cannot preserve a representation.
    pub(crate) fn invalidate_representations_for_values(&mut self, values: Option<&[String]>) {
        self.value_representations.retain(|key, representation| {
            values.is_some_and(|values| {
                self.constant_values
                    .get(key)
                    .is_some_and(|contents| !values.contains(contents))
                    || matches!(representation,
                        crate::native_numeric::StoredNativeRepresentation::NumericShape(shape)
                        if values.iter().all(|value| !shape.string_bytes_may_equal(value.as_bytes())))
            })
        });
        if self.representation_epoch.is_some() {
            self.representation_epoch_high_water = self
                .representation_epoch_high_water
                .and_then(|epoch| epoch.checked_add(1));
            self.representation_epoch = self.representation_epoch_high_water;
        }
    }

    /// A possible ordinary-container conversion retains a closed cost domain
    /// for objects that may share the selected input. Strict intrep stays unknown.
    pub(crate) fn coerce_ordinary_container_representations(
        &mut self,
        values: Option<&[String]>,
        target: tcl_syntax::value::ValueRepresentation,
    ) {
        let Some(target_domain) = crate::native_numeric::ClosedContainerRepresentations::of(target)
        else {
            self.invalidate_representations_for_values(values);
            return;
        };
        let possible = self
            .value_representations
            .iter()
            .filter_map(|(key, value)| {
                let domain = value.container_alternatives()?;
                let disjoint = values.is_some_and(|values| {
                    self.constant_values
                        .get(key)
                        .is_some_and(|contents| !values.contains(contents))
                });
                (!disjoint).then(|| (key.clone(), domain.joined(target_domain)))
            })
            .collect::<Vec<_>>();
        let stock = self
            .value_representations
            .iter()
            .filter_map(|(key, value)| {
                let crate::native_numeric::StoredNativeRepresentation::StockLiteralObject(object) =
                    value
                else {
                    return None;
                };
                let disjoint = values.is_some_and(|values| {
                    self.constant_values
                        .get(key)
                        .is_some_and(|contents| !values.contains(contents))
                });
                (!disjoint).then(|| {
                    (
                        key.clone(),
                        crate::native_numeric::StoredNativeRepresentation::StockLiteralObject(
                            object.possibly_converted(target),
                        ),
                    )
                })
            })
            .collect::<Vec<_>>();
        self.invalidate_representations_for_values(values);
        self.value_representations.extend(
            possible
                .into_iter()
                .map(|(key, domain)| (key, domain.stored())),
        );
        self.value_representations.extend(stock);
    }

    /// A selected successful native constructor can issue a new representation
    /// receipt after an uncertain join. It cannot revive an older receipt.
    pub(crate) fn establish_created_representation_epoch(&mut self) {
        if self.representation_epoch.is_some() {
            return;
        }
        self.value_representations.clear();
        self.representation_epoch_high_water = self
            .representation_epoch_high_water
            .and_then(|epoch| epoch.checked_add(1));
        self.representation_epoch = self.representation_epoch_high_water;
    }

    /// Evaluated code may retarget bindings and change traces and lifetimes.
    pub fn widen(&mut self) {
        self.closed_array_roots.clear();
        self.namespace_inventory = NamespaceInventoryClosure::Open;
        self.captured_cells.widen();
        for slot in self.raw_bindings.slots.values_mut() {
            slot.contents = crate::raw_binding::RawBindingContents::Unknown;
        }
        self.namespace_cells.widen();
        self.retain_literal_values(|_| false);
        self.invalidate_shared_representations();
        self.contents_world = ContentsWorld::Unknown;
        self.withdraw_activation_contents_closure();
        self.contents_kinds.clear();
        for origin in self.contents_origins.values_mut() {
            *origin = ContentsOrigin::Unknown;
        }
        for presence in self.contents_presence.values_mut() {
            *presence = ContentsPresence::Unknown;
        }

        self.dynamic_bindings = true;
        self.mark_unenumerated_variable_observers();
        self.unknown_bindings
            .extend(self.alias_bindings.keys().cloned());
        self.alias_bindings.clear();
        self.name_alias_bindings.clear();
        for alias in self.namespace_name_alias_bindings.values_mut() {
            *alias = place::unknown_top();
        }
        for alias in self.namespace_alias_bindings.values_mut() {
            *alias = place::unknown_top();
        }
        self.known_namespaces
            .retain(|name| name == "::" || name == &self.namespace);
        for generation in self.generations.values_mut() {
            *generation = CellGeneration::Unknown;
        }
    }
}

// A scalar name slot survives Tcl unset. Its contents may be fresh while
// aliases still select the same slot. Element links require a proved root
// lifetime: joining different generations must never revive a dangling link.
fn join_alias_target(left: &Place, right: &Place) -> Option<Place> {
    let mut joined = left.clone();
    let mut candidate = right.clone();
    let observed = joined.observed || candidate.observed;
    joined.observed = observed;
    candidate.observed = observed;
    if matches!(left.kind, PlaceKind::Scalar | PlaceKind::ArrayWhole)
        && let (Some(left_cell), Some(right_cell)) = (&mut joined.cell, &mut candidate.cell)
        && left_cell.generation != right_cell.generation
    {
        left_cell.generation = CellGeneration::Unknown;
        right_cell.generation = CellGeneration::Unknown;
    }
    (joined == candidate).then_some(joined)
}

fn has_subst(text: &str) -> bool {
    text.contains('$') || text.contains('[')
}

/// Resolve the variables read inside a dynamic index / name *text*.
///
/// Returns the bound read-places sorted by `(ns, name)` (deterministic).
fn inner_read_places(text: &str, ctx: &ResolveContext, registry: &CommandRegistry) -> Vec<Place> {
    // Read-roles + cmd-sub recursion so `a([idx])` and `a($ns::i)` are covered.
    let mut scanner = VarReferenceScanner::for_registry(
        VarScanOptions {
            include_var_read_roles: true,
            recurse_cmd_substitutions: true,
            include_reads_before_write: false,
            element_qualified: true,
        },
        registry,
    );
    let names = scanner.scan_word(text, registry);
    let mut places: Vec<Place> = names
        .iter()
        .map(|name| {
            resolve_access(
                name,
                ctx,
                false,
                registry,
                tcl_registry::TraceOperation::Read,
            )
        })
        .collect();
    places.sort_by(|a, b| (&a.ns, &a.name).cmp(&(&b.ns, &b.name)));
    places
}

/// Turn an array index / dict key text into an [`Index`].
fn classify_index(elem: &str, ctx: &ResolveContext, registry: &CommandRegistry) -> Index {
    if has_subst(elem) {
        Index::dynamic(inner_read_places(elem, ctx, registry))
    } else {
        Index::literal(elem)
    }
}

/// Bind a (sigil-stripped, scalar) base name to a canonical [`Place`] using the
/// scope declarations in *ctx*.  `observed` overrides the trace-derived flag
/// when `Some`.
fn bind_scalar(
    base: &str,
    ctx: &ResolveContext,
    observed: Option<bool>,
    registry: &CommandRegistry,
) -> Place {
    let mut bound = bind_scalar_receiver(base, ctx, observed, registry);
    if observed.is_none() {
        bound.observed |= ctx.unenumerated_observers_may_run(&bound);
    }
    bound
}

fn bind_scalar_receiver(
    base: &str,
    ctx: &ResolveContext,
    observed: Option<bool>,
    registry: &CommandRegistry,
) -> Place {
    if ctx.observed_variable_storage_unavailable() {
        return place::unknown_top();
    }
    let obs = observed.unwrap_or_else(|| ctx.traced.contains(base));
    if ctx.unknown_bindings.contains(base) {
        return place::unknown_top();
    }
    if !ctx.raw_bindings.bindings.is_empty() {
        let slot = resolve_alias_destination_slot(base, ctx, registry);
        if let Some(key) = ctx.raw_bindings.bindings.get(&cell_key(&slot)) {
            let mut target = ctx.raw_bindings.resolve(key, ctx, registry);
            target.observed |= obs;
            return target;
        }
    }
    if let Some(mut target) = namespace_slot_alias(base, ctx, registry) {
        target.observed |= obs || ctx.traced.contains(&cell_key(&target));
        return target;
    }
    if let Some(target) = ctx.name_alias_bindings.get(base) {
        let mut target = resolve_name_alias(target, ctx);
        target.observed |= obs;
        return target;
    }
    if let Some(target) = ctx.alias_bindings.get(base) {
        let mut target = target.clone();
        target.observed |= obs || ctx.traced.contains(&cell_key(&target));
        return target;
    }
    if ctx.dynamic_bindings && !tcl_syntax::naming::is_qualified(base.as_bytes()) {
        return place::unknown_top();
    }
    if let Some(target) = ctx.upvar_aliases.get(base) {
        return place::upvar_alias(base, target.clone(), false);
    }
    bind_unaliased_scalar(base, ctx, obs, registry)
}

fn bind_unaliased_scalar(
    base: &str,
    ctx: &ResolveContext,
    obs: bool,
    registry: &CommandRegistry,
) -> Place {
    let variable_policy = ctx
        .invocation_dialect
        .and_then(|dialect| dialect.variable_lookup_policy)
        .or_else(|| {
            registry
                .profile()
                .and_then(tcl_dialect::DialectProfile::variable_lookup_policy)
        });
    if variable_policy.is_none()
        && !base.starts_with("::")
        && !ctx.global_frame()
        && (ctx.namespace_scope() || tcl_syntax::naming::is_qualified(base.as_bytes()))
    {
        return place::unknown_top();
    }
    let jim_lookup = variable_policy == Some(tcl_dialect::VariableLookupPolicy::Jim);
    let bound = if ctx.instance_vars.contains(base) {
        place::instance_var(base, &ctx.instance_owner, obs)
    } else if ctx.globals.contains(base) {
        ctx.namespace_place(base, true, obs)
    } else if ctx.ns_vars.contains(base) {
        ctx.namespace_place(base, false, obs)
    } else if base.starts_with("::") {
        ctx.namespace_place(base, true, obs)
    } else if jim_lookup && !ctx.global_frame() {
        // Jim only treats an initial :: as a global qualifier. Namespace eval
        // runs a local activation; variable declarations install explicit links.
        place::scalar(base, place::LOCAL_NS, obs)
    } else if tcl_syntax::naming::is_qualified(base.as_bytes()) || ctx.namespace_scope() {
        if !ctx.namespace_known {
            return place::unknown_top();
        }
        let current = ctx.namespace_place(base, false, obs);
        let global = ctx.namespace_place(base, true, obs);
        let fallback = ctx
            .invocation_dialect
            .and_then(|dialect| dialect.namespace_var_global_fallback)
            .or_else(|| {
                registry
                    .profile()
                    .map(tcl_dialect::DialectProfile::namespace_var_global_fallback)
            });
        if fallback == Some(false)
            || cell_key(&current) == cell_key(&global)
            || ctx.ns_vars.contains(base)
            || current.kind != PlaceKind::Unknown
                && ctx.namespace_cells.present.contains(&cell_key(&current))
        {
            current
        } else if fallback == Some(true)
            && (current.kind != PlaceKind::Unknown
                && ctx.namespace_cells.absent(&cell_key(&current))
                || ctx.namespace_candidate_absent(base, false))
            && ctx.namespace_cells.present.contains(&cell_key(&global))
        {
            global
        } else if (global.kind != PlaceKind::Unknown
            && ctx.namespace_cells.absent(&cell_key(&global))
            || ctx.namespace_candidate_absent(base, true))
            && current.cell.as_ref().is_some_and(|cell| match &cell.owner {
                CellOwner::NamespaceIdentity(identity) => {
                    ctx.namespace_identities.contains(identity.as_ref())
                }
                CellOwner::Namespace(namespace) => ctx.known_namespaces.contains(namespace),
                _ => false,
            })
        {
            current
        } else {
            return place::unknown_top();
        }
    } else {
        place::scalar(base, place::LOCAL_NS, obs)
    };
    finish_bound_scalar(bound, ctx, registry)
}

fn namespace_slot_alias(
    base: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Option<Place> {
    if (ctx.namespace_alias_bindings.is_empty() && ctx.namespace_name_alias_bindings.is_empty())
        || !(ctx.global_frame()
            || ctx.namespace_scope()
            || tcl_syntax::naming::is_qualified(base.as_bytes()))
    {
        return None;
    }
    // Namespace links belong to the slot, independently of a caller frame's
    // older spelling map retained across a selected-frame callback.
    let slot = resolve_alias_destination_slot(base, ctx, registry);
    if !slot.is_global() {
        return None;
    }
    let key = cell_key(&slot);
    ctx.namespace_name_alias_bindings
        .get(&key)
        .map(|target| resolve_name_alias(target, ctx))
        .or_else(|| ctx.namespace_alias_bindings.get(&key).cloned())
}

fn finish_bound_scalar(
    mut bound: Place,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Place {
    if ctx.point_sensitive() {
        bind_cell_identity(&mut bound, ctx, registry);
    }
    if bound.is_global()
        && let Some(target) = ctx.namespace_name_alias_bindings.get(&cell_key(&bound))
    {
        return resolve_name_alias(target, ctx);
    }
    if bound.is_global()
        && let Some(alias) = ctx.namespace_alias_bindings.get(&cell_key(&bound))
    {
        return alias.clone();
    }
    bound.observed |= ctx.traced.contains(&cell_key(&bound));
    bound
}

fn resolve_name_alias(target: &Place, context: &ResolveContext) -> Place {
    let mut current = target.clone();
    let mut visited = HashSet::new();
    loop {
        if current.kind == PlaceKind::Unknown || !visited.insert(cell_key(&current)) {
            return place::unknown_top();
        }
        let selected = match current.cell.as_ref().map(|cell| &cell.owner) {
            Some(CellOwner::Activation(identity)) => {
                let mut frame = Some(context);
                while frame.is_some_and(|frame| frame.activation.as_ref() != Some(identity)) {
                    frame = frame.and_then(|frame| frame.caller.as_deref());
                }
                let Some(frame) = frame else {
                    return place::unknown_top();
                };
                frame
            }
            Some(
                CellOwner::Namespace(_) | CellOwner::NamespaceIdentity(_) | CellOwner::CurrentFrame,
            ) => context,
            _ => return place::unknown_top(),
        };
        if current.ns == place::LOCAL_NS && selected.unknown_bindings.contains(&current.name) {
            return place::unknown_top();
        }
        let next = if current.ns == place::LOCAL_NS {
            selected.name_alias_bindings.get(&current.name)
        } else {
            context
                .namespace_name_alias_bindings
                .get(&cell_key(&current))
        };
        let Some(next) = next else {
            let generation = context
                .generations
                .get(&cell_key(&current))
                .copied()
                .unwrap_or_default();
            if let Some(cell) = &mut current.cell {
                cell.generation = generation;
            }
            return current;
        };
        let index = current.index.clone();
        current = next.clone();
        if let Some(index) = index {
            if current.index.is_some() {
                return place::unknown_top();
            }
            current.kind = PlaceKind::ArrayElem;
            current.index = Some(index);
        }
    }
}

fn bind_cell_identity(bound: &mut Place, ctx: &ResolveContext, registry: &CommandRegistry) {
    let owner = if bound.kind == PlaceKind::InstanceVar {
        CellOwner::Instance(bound.owner.clone())
    } else if bound.ns == place::LOCAL_NS {
        ctx.selected_frame.map_or_else(
            || {
                ctx.activation
                    .as_ref()
                    .map_or(CellOwner::CurrentFrame, |identity| {
                        CellOwner::Activation(identity.clone())
                    })
            },
            CellOwner::SelectedFrame,
        )
    } else {
        bound
            .cell
            .as_ref()
            .and_then(|cell| match &cell.owner {
                CellOwner::NamespaceIdentity(identity) => {
                    Some(CellOwner::NamespaceIdentity(identity.clone()))
                }
                _ => None,
            })
            .unwrap_or_else(|| CellOwner::Namespace(bound.ns.clone()))
    };
    let storage_domain = (bound.ns != place::LOCAL_NS).then(|| {
        use tcl_registry::f5::VariableStorageDomain;
        if let CellOwner::NamespaceIdentity(namespace) = &owner {
            if ctx
                .authored_tmm_static
                .as_ref()
                .is_some_and(|receipt| namespace.native_context() == Some(&receipt.namespace))
            {
                VariableStorageDomain::WorkerNamespace
            } else if registry
                .profile()
                .is_some_and(tcl_dialect::DialectProfile::is_irules)
                && namespace
                    .exact_native_path()
                    .is_some_and(tcl_core_types::ByteNamespacePath::is_root)
            {
                VariableStorageDomain::CmpGlobal
            } else {
                VariableStorageDomain::InterpreterNamespace
            }
        } else {
            tcl_registry::f5::namespace_storage_domain(
                if registry
                    .profile()
                    .is_some_and(tcl_dialect::DialectProfile::is_irules)
                {
                    tcl_registry::f5::BigIpExecutionContext::TmmIRule
                } else {
                    tcl_registry::f5::BigIpExecutionContext::HostShellTcl
                },
                &bound.ns,
            )
        }
    });
    bound.cell = Some(CellIdentity {
        owner,
        name: bound.name.clone(),
        generation: CellGeneration::Incoming,
        interpreter: ctx.interpreter.clone(),
        storage_domain,
        execution: ctx.execution.map(|mut execution| {
            if bound.ns != place::LOCAL_NS {
                execution.connection = None;
            }
            execution
        }),
    });
    let generation = ctx
        .generations
        .get(&cell_key(bound))
        .copied()
        .unwrap_or_default();
    if let Some(cell) = &mut bound.cell {
        cell.generation = generation;
    }
}

/// Resolve an alias destination slot, whose namespace creation grammar never
/// uses ordinary read fallback to a same-named global cell.
#[must_use]
pub fn resolve_alias_destination_slot(
    reference: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Place {
    let mut slot = ctx.clone();
    slot.raw_bindings = crate::raw_binding::RawBindingArena::default();
    slot.alias_bindings.clear();
    slot.namespace_alias_bindings.clear();
    slot.name_alias_bindings.clear();
    slot.namespace_name_alias_bindings.clear();
    slot.globals.clear();
    slot.ns_vars.clear();
    slot.unknown_bindings.clear();
    slot.dynamic_bindings = false;
    let jim = slot
        .invocation_dialect
        .and_then(|dialect| dialect.variable_lookup_policy)
        .or_else(|| {
            registry
                .profile()
                .and_then(tcl_dialect::DialectProfile::variable_lookup_policy)
        })
        == Some(tcl_dialect::VariableLookupPolicy::Jim);
    if !jim && (slot.namespace_scope() || tcl_syntax::naming::is_qualified(reference.as_bytes())) {
        slot.ns_vars.insert(reference.to_owned());
    }
    resolve_literal_place(reference, &slot, false, registry)
}

/// How a proved namespace access was selected from an evaluated name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespaceAccessOrigin {
    /// The literal name explicitly selected a namespace table.
    ExplicitName,
    /// An alias or lookup selected a namespace cell outside the direct slot.
    NonLocalBinding,
    /// The current namespace activation's ordinary unqualified slot.
    CurrentFrame,
    /// No namespace target or direct-slot relationship was proved.
    Unknown,
}

/// Preserve namespace selection provenance independently of contents effects.
/// The name is an evaluated literal; source quoting and substitution syntax
/// never determine this result. Unknown targets remain unknown.
#[must_use]
pub fn literal_namespace_access_origin(
    reference: &str,
    context: &ResolveContext,
    registry: &CommandRegistry,
    operation: tcl_registry::TraceOperation,
) -> NamespaceAccessOrigin {
    let target = resolve_literal_access(reference, context, false, registry, operation);
    if target.dynamic
        || !matches!(
            target.cell.as_ref().map(|cell| &cell.owner),
            Some(CellOwner::Namespace(_) | CellOwner::NamespaceIdentity(_))
        )
    {
        return NamespaceAccessOrigin::Unknown;
    }
    let (root, _) = tcl_syntax::naming::split_array_name(reference);
    if tcl_syntax::naming::is_qualified(root.as_bytes()) {
        return NamespaceAccessOrigin::ExplicitName;
    }
    let direct = resolve_alias_destination_slot(reference, context, registry);
    if target.cell == direct.cell {
        NamespaceAccessOrigin::CurrentFrame
    } else if direct.cell.is_some() && !direct.dynamic {
        NamespaceAccessOrigin::NonLocalBinding
    } else {
        NamespaceAccessOrigin::Unknown
    }
}

/// Bound the storage domain of an already evaluated literal variable name.
/// C Tcl forbids a namespace binding slot from linking to a procedure-local
/// cell, even when the target link is unknown after evaluated code. Namespace
/// aliases can still select any namespace, including host worker namespaces.
/// Do not use this result as a Must address or successful-store proof.
#[must_use]
pub fn possible_access_domain(
    reference: &str,
    context: &ResolveContext,
    registry: &CommandRegistry,
) -> PossibleVariableAccessDomain {
    if context
        .invocation_dialect
        .and_then(|dialect| dialect.variable_lookup_policy)
        != Some(tcl_dialect::VariableLookupPolicy::Tcl)
    {
        return PossibleVariableAccessDomain::Any;
    }
    let slot = resolve_alias_destination_slot(reference, context, registry);
    if !slot.dynamic
        && matches!(
            slot.cell.as_ref().map(|cell| &cell.owner),
            Some(CellOwner::Namespace(_) | CellOwner::NamespaceIdentity(_))
        )
    {
        PossibleVariableAccessDomain::NamespaceOnly
    } else {
        PossibleVariableAccessDomain::Any
    }
}

fn authored_namespace_place(name: &str, namespace: &str, observed: bool) -> Place {
    let fq = tcl_syntax::naming::qualify(namespace, name);
    let (ns, tail) = tcl_syntax::naming::key_holder_and_tail(&fq);
    place::scalar(tail, if ns.is_empty() { "::" } else { ns }, observed)
}

/// Stable name for a root's contents and lifetime state.
#[must_use]
pub fn cell_key(place: &Place) -> VariableCellKey {
    if let Some(CellIdentity {
        owner: CellOwner::NamespaceIdentity(identity),
        ..
    }) = &place.cell
    {
        return VariableCellKey::Namespace {
            identity: (**identity).clone(),
            simple: place.name.clone(),
        };
    }
    if let Some(CellIdentity {
        owner: CellOwner::AllocatedInstance(allocation),
        ..
    }) = &place.cell
    {
        return crate::allocated_instance::storage_key(allocation, &place.name);
    }
    if let Some(CellIdentity {
        owner: CellOwner::RetainedSlot(identity),
        ..
    }) = &place.cell
    {
        return VariableCellKey::RetainedSlot(identity.clone());
    }
    if let Some(CellIdentity {
        owner: CellOwner::SelectedFrame(selector),
        ..
    }) = &place.cell
    {
        VariableCellKey::SelectedFrame {
            selector: *selector,
            simple: place.name.clone(),
        }
    } else if let Some(CellIdentity {
        owner: CellOwner::Instance(identity),
        ..
    }) = &place.cell
    {
        VariableCellKey::Instance {
            identity: identity.clone(),
            simple: place.name.clone(),
        }
    } else if let Some(CellIdentity {
        owner: CellOwner::Activation(identity),
        ..
    }) = &place.cell
    {
        VariableCellKey::Activation {
            identity: identity.clone(),
            simple: place.name.clone(),
        }
    } else if place.ns == place::LOCAL_NS {
        place.name.clone().into()
    } else {
        tcl_syntax::naming::qualify(&place.ns, &place.name).into()
    }
}

/// Whether an owner-authored storage key denotes a cell under this namespace.
/// Lifetime wrappers retain the underlying namespace membership.
#[must_use]
pub(crate) fn binding_key_in_namespace<
    Q: VariableCellKeyQuery + ?Sized,
    N: VariableNamespaceQuery + ?Sized,
>(
    key: &Q,
    namespace: &N,
) -> bool {
    key.variable_cell_key()
        .is_in_namespace(namespace.variable_namespace_key().as_ref())
}

/// Project a proven bound access into the compatibility name vocabulary.
/// Unknown or foreign-frame targets never acquire a local name.
#[must_use]
pub fn canonical_variable_name(
    reference: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Option<String> {
    canonical_variable_key(reference, ctx, registry).map(|key| key.compatibility_name())
}

/// Canonical scalar key for an already-evaluated literal name operand.
#[must_use]
pub fn canonical_literal_variable_name(
    reference: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Option<String> {
    canonical_binding_value_name(&resolve_literal_place(reference, ctx, false, registry))
}

/// Exact binding-value address selected from an original written variable operand.
#[must_use]
pub fn canonical_literal_variable_key(
    reference: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Option<VariableCellKey> {
    canonical_binding_value_key(&resolve_literal_place(reference, ctx, false, registry))
}

/// Physical array-root identity, preserving captured lifetimes.
#[must_use]
pub(crate) fn physical_array_key(place: &Place) -> Option<VariableCellKey> {
    let mut root = place.clone();
    root.index = None;
    root.kind = PlaceKind::Scalar;
    canonical_place_key(&root)
}

/// Scalar contents key of the current binding slot. A successful scalar store
/// defines the slot's value even when a loop joins different physical lifetimes.
/// Captured element links remain lifetime-sensitive because deletion can dangle them.
#[must_use]
pub fn canonical_binding_value_name(bound: &Place) -> Option<String> {
    canonical_binding_value_key(bound).map(|key| key.compatibility_name())
}

/// Exact binding-value slot, retaining native namespace and member identity.
#[must_use]
pub fn canonical_binding_value_key(bound: &Place) -> Option<VariableCellKey> {
    if bound.index.is_none() && matches!(bound.kind, PlaceKind::Scalar | PlaceKind::ArrayWhole) {
        let mut binding = bound.clone();
        if let Some(cell) = &mut binding.cell {
            cell.generation = CellGeneration::Incoming;
        }
        canonical_place_key(&binding)
    } else {
        canonical_place_key(bound)
    }
}

/// Project an immutable bound access into compatibility scalar SSA names.
#[must_use]
pub fn canonical_place_name(bound: &Place) -> Option<String> {
    canonical_place_key(bound).map(|key| key.compatibility_name())
}

/// Exact physical storage slot, including selected lifetime and array member.
#[must_use]
pub fn canonical_place_key(bound: &Place) -> Option<VariableCellKey> {
    if bound
        .cell
        .as_ref()
        .is_some_and(|cell| matches!(cell.owner, CellOwner::SelectedFrame(_)))
        || bound.dynamic
        || matches!(
            bound.kind,
            PlaceKind::Unknown | PlaceKind::UpvarAlias | PlaceKind::InstanceVar
        )
    {
        return None;
    }
    let mut name = cell_key(bound);
    if let Some(cell) = &bound.cell {
        match cell.generation {
            CellGeneration::Incoming => {}
            CellGeneration::After(offset) => name = name.with_lifetime(offset),
            CellGeneration::Unknown => return None,
        }
    }
    if let Some(index) = &bound.index {
        if index.kind != place::IndexKind::Literal {
            return None;
        }
        name = name.with_index(&index.value);
    }
    Some(name)
}

/// Original simple key in the actual root namespace table, including resolved links.
/// Selected frames, locals and displayed namespace names do not grant root identity.
#[must_use]
pub fn root_namespace_variable_simple_name(place: &Place) -> Option<&str> {
    fn original(cell: &CellIdentity) -> Option<&str> {
        let root = match &cell.owner {
            CellOwner::NamespaceIdentity(namespace) => match namespace.as_ref() {
                crate::command_binding::SourceNamespaceKey::Authored(namespace) => {
                    namespace == "::"
                }
                _ => namespace
                    .exact_native_path()
                    .is_some_and(tcl_core_types::ByteNamespacePath::is_root),
            },
            CellOwner::Namespace(namespace) => namespace == "::",
            CellOwner::RetainedSlot(slot) => {
                return match slot.as_ref() {
                    crate::raw_binding::RawBindingSlotId::Variable(cell) => original(cell),
                    crate::raw_binding::RawBindingSlotId::Callable { .. } => None,
                };
            }
            _ => false,
        };
        root.then_some(cell.name.as_str())
    }
    match place.cell.as_ref() {
        Some(cell) => original(cell),
        // Legacy symbolic places have no native owner to project. The
        // already resolved namespace field is authored advice only.
        None => (place.is_global() && place.ns == "::").then_some(place.name.as_str()),
    }
}

/// Fully qualified identity for a proven namespace-variable access.
#[must_use]
pub fn canonical_namespace_variable(
    reference: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Option<String> {
    canonical_namespace_variable_key(reference, ctx, registry).map(|key| key.compatibility_name())
}

/// Original resolved namespace receiver, retaining its actual cell and storage domain.
#[must_use]
pub fn canonical_namespace_variable_place(
    reference: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Option<Place> {
    let place = resolve_place(reference, ctx, false, registry);
    (place.is_global() && !place.dynamic && canonical_place_key(&place).is_some()).then_some(place)
}

/// Exact namespace cell address; its compatibility display grants no lookup authority.
#[must_use]
pub fn canonical_namespace_variable_key(
    reference: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Option<VariableCellKey> {
    canonical_place_key(&canonical_namespace_variable_place(
        reference, ctx, registry,
    )?)
}

/// Resolve a variable reference *reference* (as written) to a canonical
/// [`Place`].
///
/// `whole_array` marks a reference the command touches as an entire array
/// (`array get a` / `array set a` / a bare `$a` passed to such), so it becomes
/// `ARRAY_WHOLE` rather than a scalar.
#[must_use]
pub fn resolve_place(
    reference: &str,
    ctx: &ResolveContext,
    whole_array: bool,
    registry: &CommandRegistry,
) -> Place {
    resolve_place_value(reference, ctx, whole_array, registry, false, false)
}

/// Exact selected storage slot for a source variable reference.
#[must_use]
pub fn canonical_variable_key(
    reference: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Option<VariableCellKey> {
    canonical_binding_value_key(&resolve_place_value(
        reference, ctx, false, registry, true, false,
    ))
}

/// Resolve an already-evaluated literal variable name, including literal dollar signs.
#[must_use]
pub fn resolve_literal_place(
    reference: &str,
    ctx: &ResolveContext,
    whole_array: bool,
    registry: &CommandRegistry,
) -> Place {
    resolve_place_value(reference, ctx, whole_array, registry, true, true)
}

/// Resolve a written variable-name word, preserving whether braces suppress
/// name and array-index substitutions. The resulting access retains reads
/// needed to form a dynamic target rather than inventing a literal cell.
#[must_use]
pub fn resolve_target_access(
    reference: &str,
    braced: bool,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
    operation: tcl_registry::TraceOperation,
) -> Place {
    project_access(
        resolve_place_value(reference, ctx, false, registry, braced, braced),
        ctx,
        operation,
    )
}

fn resolve_place_value(
    reference: &str,
    ctx: &ResolveContext,
    whole_array: bool,
    registry: &CommandRegistry,
    literal_base: bool,
    literal_index: bool,
) -> Place {
    if ctx.observed_variable_storage_unavailable() {
        return place::unknown_top();
    }
    if reference.is_empty() && !literal_base {
        return place::unknown_top();
    }

    // A target whose *name* is value-substituted (`set $X v`, `set ${tok}(k) v`)
    // denotes the variable named by the substitution's value — a place that
    // cannot be pinned down — but it *reads* the name variable(s).  Read
    // references arrive de-sigilled from the scanner, so a leading `$` here
    // always marks a write-target dynamic name.
    if !literal_base && reference.starts_with('$') {
        return place::unknown(inner_read_places(reference, ctx, registry));
    }

    let (base, elem) = tcl_syntax::naming::split_array_name_braced(reference, literal_base);

    if !literal_base && (base.is_empty() || has_subst(base)) {
        return place::unknown(inner_read_places(reference, ctx, registry));
    }

    if let Some(elem) = elem {
        let index = if literal_index {
            Index::literal(elem)
        } else {
            classify_index(elem, ctx, registry)
        };
        let bound = bind_scalar(base, ctx, None, registry);
        if bound.kind == PlaceKind::Unknown || bound.kind == PlaceKind::ArrayElem {
            return place::unknown(index.read_places);
        }
        // Carry namespace/observed/dynamic/owner from the bound scalar.
        return Place {
            kind: PlaceKind::ArrayElem,
            ns: bound.ns,
            name: bound.name,
            index: Some(index),
            keys: Vec::new(),
            owner: bound.owner,
            observed: bound.observed,
            dynamic: bound.dynamic,
            name_reads: Vec::new(),
            cell: bound.cell,
        };
    }

    if whole_array {
        let bound = bind_scalar(base, ctx, None, registry);
        if bound.kind == PlaceKind::Unknown || bound.kind == PlaceKind::ArrayElem {
            return place::unknown_top();
        }
        return Place {
            kind: PlaceKind::ArrayWhole,
            ns: bound.ns,
            name: bound.name,
            index: None,
            keys: Vec::new(),
            owner: bound.owner,
            observed: bound.observed,
            dynamic: bound.dynamic,
            name_reads: Vec::new(),
            cell: bound.cell,
        };
    }

    bind_scalar(base, ctx, None, registry)
}

/// Registration address of a trace; array elements retain their own indices.
#[must_use]
pub fn trace_key(place: &Place) -> VariableCellKey {
    let mut key = cell_key(place);
    if let Some(index) = &place.index {
        key = key.with_index(&index.value);
    }
    key
}

fn variable_observer_keys(
    bound: &Place,
    ctx: &ResolveContext,
    operation: tcl_registry::TraceOperation,
) -> Vec<VariableCellKey> {
    let root = cell_key(bound);
    let mut candidates = vec![root.clone()];
    if let Some(index) = &bound.index {
        if index.kind == place::IndexKind::Literal {
            candidates.push(trace_key(bound));
        } else {
            candidates.extend(
                ctx.traced
                    .iter()
                    .filter(|key| key.is_member_of(&root))
                    .cloned(),
            );
        }
    } else if bound.kind == PlaceKind::ArrayWhole
        || operation == tcl_registry::TraceOperation::Unset
    {
        candidates.extend(
            ctx.traced
                .iter()
                .filter(|key| key.is_member_of(&root))
                .cloned(),
        );
    }
    if bound.kind == PlaceKind::Unknown {
        candidates.extend(
            ctx.traced
                .iter()
                .filter(|key| {
                    bound.ns == place::LOCAL_NS
                        || ctx
                            .namespace_footprint(bound)
                            .is_none_or(|namespace| key.is_in_namespace(&namespace))
                })
                .cloned(),
        );
    }
    candidates
}

/// One registration selected from an actual physical variable access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VariableObserverCallback {
    pub(crate) key: VariableCellKey,
    pub(crate) prefix: String,
}

/// Ordered known callbacks plus obligations that cannot be enumerated safely.
#[derive(Debug, Default)]
pub(crate) struct VariableObserverProjection {
    pub(crate) callbacks: Vec<VariableObserverCallback>,
    pub(crate) possible_callbacks: Vec<String>,
    pub(crate) unknown_residual: bool,
}

impl ResolveContext {
    /// Complete typed registration inventory for one proved array root. Missing
    /// receiver provenance cannot become an empty member callback inventory.
    pub(crate) fn array_registration_receivers_known(&self, root: &Place) -> bool {
        variable_observer_keys(root, self, tcl_registry::TraceOperation::Unset)
            .into_iter()
            .all(|key| {
                !self.untracked_traces.contains(&key)
                    && (!self.traced.contains(&key) && !self.trace_registrations.contains_key(&key)
                        || self
                            .trace_registration_receivers
                            .get(&key)
                            .is_some_and(|receiver| receiver.cell == root.cell))
            })
    }

    /// Select only registrations on this captured cell, excluding array members
    /// and containing-root registrations. Staged teardown owns those phases.
    pub(crate) fn variable_observers_on_cell(
        &self,
        access: &Place,
        operation: tcl_registry::TraceOperation,
        registry: &CommandRegistry,
    ) -> VariableObserverProjection {
        self.variable_observers_for_keys(
            access,
            operation,
            registry,
            vec![trace_key(access)],
            false,
        )
    }

    pub(crate) fn variable_observers_at(
        &self,
        access: &Place,
        operation: tcl_registry::TraceOperation,
        registry: &CommandRegistry,
    ) -> VariableObserverProjection {
        let keys = variable_observer_keys(access, self, operation);
        let unordered_destruction = operation == tcl_registry::TraceOperation::Unset
            && access.index.is_none()
            && keys.iter().any(|key| key != &cell_key(access));
        self.variable_observers_for_keys(access, operation, registry, keys, unordered_destruction)
    }

    pub(crate) fn unenumerated_observers_may_run(&self, access: &Place) -> bool {
        let closed_owner = match access.cell.as_ref().map(|cell| &cell.owner) {
            Some(CellOwner::Activation(identity)) => {
                self.activation_observers_closed && self.activation.as_ref() == Some(identity)
            }
            Some(CellOwner::AllocatedInstance(allocation)) => self
                .closed_observer_allocations
                .contains(allocation.as_ref()),
            _ => false,
        };
        self.dynamic_traces && !closed_owner
    }

    /// An unresolved registration can select an existing caller or receiver.
    /// Revoke every affected fresh-owner receipt, not just the active frame.
    pub(crate) fn mark_unenumerated_variable_observers(&mut self) {
        self.dynamic_traces = true;
        self.activation_observers_closed = false;
        self.closed_observer_allocations.clear();
        if let Some(caller) = &mut self.caller {
            std::sync::Arc::make_mut(caller).mark_unenumerated_variable_observers();
        }
    }

    fn variable_observers_for_keys(
        &self,
        access: &Place,
        operation: tcl_registry::TraceOperation,
        registry: &CommandRegistry,
        keys: Vec<VariableCellKey>,
        unordered_destruction: bool,
    ) -> VariableObserverProjection {
        let uncertain_address = access.kind == PlaceKind::Unknown
            || access
                .cell
                .as_ref()
                .is_none_or(|cell| cell.generation == CellGeneration::Unknown)
            || access
                .index
                .as_ref()
                .is_some_and(|index| index.kind != place::IndexKind::Literal);
        let mut projection = VariableObserverProjection {
            unknown_residual: self.unenumerated_observers_may_run(access)
                || self.authored_static_observer_may_run(access, operation)
                || self.implicit_read_changes_value(access, registry)
                    && operation == tcl_registry::TraceOperation::Read,
            ..VariableObserverProjection::default()
        };
        for key in keys {
            let Some(registrations) = self.trace_registrations.get(&key) else {
                projection.unknown_residual |=
                    self.traced.contains(&key) || self.untracked_traces.contains(&key);
                continue;
            };
            projection.unknown_residual |= self.untracked_traces.contains(&key);
            for (operations, prefix) in registrations.iter().rev() {
                if operations.contains(&operation) {
                    projection.unknown_residual |= uncertain_address || unordered_destruction;
                    projection.callbacks.push(VariableObserverCallback {
                        key: key.clone(),
                        prefix: prefix.clone(),
                    });
                }
            }
        }
        projection.possible_callbacks = self
            .possible_trace_registrations
            .iter()
            .filter(|registration| {
                registration.operations.contains(&operation)
                    && place::overlap(&registration.target, access)
            })
            .map(|registration| registration.prefix.clone())
            .collect();
        if !projection.callbacks.is_empty() || !projection.possible_callbacks.is_empty() {
            let dialect = self.invocation_dialect.or_else(|| {
                registry
                    .profile()
                    .map(tcl_registry::InvocationDialect::of_profile)
            });
            projection.unknown_residual |=
                dialect.and_then(|dialect| dialect.tcl_version).is_none();
        }
        projection
    }
}

/// Observe only callbacks installed for this variable access operation.
#[must_use]
pub fn project_access(
    mut bound: Place,
    ctx: &ResolveContext,
    operation: tcl_registry::TraceOperation,
) -> Place {
    let candidates = variable_observer_keys(&bound, ctx, operation);
    bound.observed = ctx.unenumerated_observers_may_run(&bound)
        || ctx.authored_static_observer_may_run(&bound, operation)
        || ctx.possible_trace_registrations.iter().any(|registration| {
            registration.operations.contains(&operation)
                && place::overlap(&registration.target, &bound)
        })
        || candidates.iter().any(|key| {
            ctx.untracked_traces.contains(key)
                || ctx.trace_registrations.get(key).map_or_else(
                    || ctx.traced.contains(key),
                    |registrations| {
                        registrations
                            .iter()
                            .any(|(operations, _)| operations.contains(&operation))
                    },
                )
        });
    bound
}

/// Resolve a source variable read/write and select applicable trace callbacks.
#[must_use]
pub fn resolve_access(
    reference: &str,
    ctx: &ResolveContext,
    whole_array: bool,
    registry: &CommandRegistry,
    operation: tcl_registry::TraceOperation,
) -> Place {
    let read = operation == tcl_registry::TraceOperation::Read;
    project_access(
        resolve_place_value(reference, ctx, whole_array, registry, read, false),
        ctx,
        operation,
    )
}

/// Resolve one original variable-substitution spelling without losing an array index.
/// Callers retain the source spelling: canonical `${...}` display wrappers cannot
/// establish whether an original array index was braced or substituted.
#[must_use]
pub fn resolve_substitution_access(
    spelling: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
    operation: tcl_registry::TraceOperation,
) -> Place {
    let config = ctx.invocation_dialect.map_or_else(
        || tcl_lexer::LexerConfig::for_profile(registry.profile()),
        |dialect| tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
    );
    let name = tcl_syntax::naming::var_reference_for_style(spelling, config.braced_var);
    let braced = tcl_syntax::naming::split_braced_var_ref(spelling, config.braced_var).is_some();
    project_access(
        resolve_place_value(name, ctx, false, registry, true, braced),
        ctx,
        operation,
    )
}

/// Resolve the original substitution with its actually evaluated index. The
/// source evaluator freezes this value before the final read; it is not a
/// later lookup of the index's variable dependencies.
#[must_use]
pub(crate) fn resolve_evaluated_substitution_access(
    spelling: &str,
    evaluated_index: Option<&str>,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
    operation: tcl_registry::TraceOperation,
) -> Place {
    let config = ctx.invocation_dialect.map_or_else(
        || tcl_lexer::LexerConfig::for_profile(registry.profile()),
        |dialect| tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
    );
    let name = tcl_syntax::naming::var_reference_for_style(spelling, config.braced_var);
    let braced = tcl_syntax::naming::split_braced_var_ref(spelling, config.braced_var).is_some();
    let mut selected = resolve_place_value(name, ctx, false, registry, true, braced);
    if !braced
        && selected.kind == PlaceKind::ArrayElem
        && let Some(index) = evaluated_index
    {
        selected.index = Some(Index::literal(index));
    }
    project_access(selected, ctx, operation)
}

/// Resolve a literal name operand and select its applicable trace callbacks.
#[must_use]
pub fn resolve_literal_access(
    reference: &str,
    ctx: &ResolveContext,
    whole_array: bool,
    registry: &CommandRegistry,
    operation: tcl_registry::TraceOperation,
) -> Place {
    project_access(
        resolve_literal_place(reference, ctx, whole_array, registry),
        ctx,
        operation,
    )
}

/// Resolve a proved evaluated array-name shape without inventing its element value.
/// Alias lookup happens in the dispatch context, after all original words evaluated.
#[must_use]
pub fn resolve_array_element_root(
    root: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
    operation: tcl_registry::TraceOperation,
) -> Place {
    let mut bound = bind_scalar(root, ctx, None, registry);
    if matches!(bound.kind, PlaceKind::Unknown | PlaceKind::ArrayElem) {
        return place::unknown_top();
    }
    bound.kind = PlaceKind::ArrayElem;
    bound.index = Some(Index::any());
    project_access(bound, ctx, operation)
}

/// Resolve a `dict get/set $d k1 k2 …` access to a `DICT_PATH` [`Place`].
#[must_use]
pub fn resolve_dict_path(
    root_ref: &str,
    keys: &[String],
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Place {
    let (base, _) = split_array_name(root_ref);
    if base.is_empty() || has_subst(base) {
        return place::unknown(inner_read_places(root_ref, ctx, registry));
    }
    let bound = resolve_place(root_ref, ctx, false, registry);
    if bound.kind == PlaceKind::Unknown {
        return bound;
    }
    let key_indices: Vec<Index> = keys
        .iter()
        .map(|k| classify_index(k, ctx, registry))
        .collect();
    let mut path = place::dict_path(bound.name, key_indices, bound.ns);
    path.cell = bound.cell;
    path.index = bound.index;
    path.observed = bound.observed;
    path.dynamic = bound.dynamic;
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::place::{LOCAL_NS, places_read_to_form, scalar};

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn ctx() -> ResolveContext {
        ResolveContext::default()
    }

    fn called_frame_with_unknown_namespace_contents(identity: &str) -> ResolveContext {
        let mut incoming = ResolveContext::for_namespace("::");
        incoming.contents_world = ContentsWorld::Unknown;
        incoming.enter_called_frame(&VariableExecutionFrame::Procedure {
            namespace: "::".to_owned(),
            identity: identity.to_owned(),
        })
    }

    #[test]
    fn fresh_called_activation_closure_is_independent_of_incoming_namespace_contents() {
        let registry = registry();
        let state = called_frame_with_unknown_namespace_contents("actual-call");
        let local = resolve_literal_place("missing", &state, false, &registry);
        let global = resolve_literal_place("::missing", &state, false, &registry);
        assert_eq!(state.contents_presence(&local), ContentsPresence::Undefined);
        assert_eq!(state.contents_presence(&global), ContentsPresence::Unknown);
        let preview =
            ResolveContext::for_namespace("::").in_frame(&VariableExecutionFrame::Procedure {
                namespace: "::".to_owned(),
                identity: "unentered-frame".to_owned(),
            });
        assert!(preview.activation_contents_world.is_none());
    }

    #[test]
    fn fresh_activation_observers_are_independent_of_incoming_namespace_traces() {
        let registry = registry();
        let mut incoming = ResolveContext::for_namespace("::");
        incoming.dynamic_traces = true;
        let frame = VariableExecutionFrame::Procedure {
            namespace: "::".to_owned(),
            identity: "actual-call".to_owned(),
        };
        let mut state = incoming.enter_called_frame(&frame);
        let local = resolve_literal_place("missing", &state, false, &registry);
        let global = resolve_literal_place("::missing", &state, false, &registry);
        assert!(!local.observed);
        assert!(global.observed);
        assert_eq!(state.contents_presence(&local), ContentsPresence::Undefined);
        assert!(
            resolve_literal_place("missing", &incoming.in_frame(&frame), false, &registry).observed
        );
        state.traced.insert(cell_key(&local));
        assert!(resolve_literal_place("missing", &state, false, &registry).observed);
        assert!(!resolve_literal_place("other", &state, false, &registry).observed);
        state.mark_unenumerated_variable_observers();
        assert!(resolve_literal_place("other", &state, false, &registry).observed);
    }

    #[test]
    fn observed_names_project_finite_bytes_without_donating_authored_storage() {
        use tcl_syntax::naming::{
            ExecutionNamePolicy, MeasuredBigIpNameScope, NamePolicyProtocol,
            NativeVariableInputForm, ObservedBigIpNamePolicy, ObservedVariableNamePurpose,
        };
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.4").commands();
        let mut context = ResolveContext::for_function("::p");
        let original = "__tcl_lsp_2286_r2286m_nul_A\0B";
        context.execution_name_policy = Some(ExecutionNamePolicy::ObservedBigIp(
            ObservedBigIpNamePolicy::for_measured_scope(
                MeasuredBigIpNameScope::BigIp21_1_0_1Build0_0_26TmmHttpRequest,
            ),
        ));
        assert_eq!(
            context
                .execution_variable_name_projection(
                    NativeVariableInputForm::Combined(original.as_bytes()),
                    ObservedVariableNamePurpose::ScalarReceiver
                )
                .unwrap()
                .root(),
            original.as_bytes()
        );
        assert_eq!(
            resolve_literal_place(original, &context, false, registry).kind,
            PlaceKind::Unknown
        );
        assert_eq!(
            context.namespace_place(original, true, false).kind,
            PlaceKind::Unknown
        );
        assert!(
            context
                .execution_variable_name_projection(
                    NativeVariableInputForm::Combined(b"qualified::unmeasured\0name"),
                    ObservedVariableNamePurpose::ScalarReceiver
                )
                .is_err()
        );
        context.execution_name_policy = Some(ExecutionNamePolicy::NativeRecipe(
            NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4),
        ));
        assert_eq!(
            resolve_literal_place("plain", &context, false, registry).kind,
            PlaceKind::Scalar
        );
    }

    #[test]
    fn unknown_child_trace_registration_withdraws_exact_caller_observer_closure() {
        let registry = registry();
        let parent = called_frame_with_unknown_namespace_contents("parent-call");
        let mut child = parent.enter_called_frame(&VariableExecutionFrame::Procedure {
            namespace: "::".to_owned(),
            identity: "child-call".to_owned(),
        });
        let untouched = restore_execution_frame(&parent, &child);
        assert!(!resolve_literal_place("missing", &untouched, false, &registry).observed);
        child.mark_unenumerated_variable_observers();
        let restored = restore_execution_frame(&parent, &child);
        assert!(resolve_literal_place("missing", &restored, false, &registry).observed);
        let mut mixed = untouched;
        mixed.join(&restored);
        assert!(resolve_literal_place("missing", &mixed, false, &registry).observed);
    }

    #[test]
    fn called_activation_closure_withdraws_on_unknown_writes_callbacks_aliases_and_joins() {
        let registry = registry();
        let state = called_frame_with_unknown_namespace_contents("actual-call");
        let mut withdrawn = state.clone();
        withdrawn.record_contents_write(&crate::place::unknown_top(), 0, true);
        let local = resolve_literal_place("missing", &withdrawn, false, &registry);
        assert_eq!(
            withdrawn.contents_presence(&local),
            ContentsPresence::Unknown
        );
        let mut callback = state.clone();
        callback.widen();
        let local = resolve_literal_place("missing", &callback, false, &registry);
        assert_eq!(
            callback.contents_presence(&local),
            ContentsPresence::Unknown
        );
        let mut mixed = state.clone();
        mixed.join(&callback);
        let local = resolve_literal_place("missing", &mixed, false, &registry);
        assert_eq!(mixed.contents_presence(&local), ContentsPresence::Unknown);
        let mut different_frame = state.clone();
        different_frame.join(&called_frame_with_unknown_namespace_contents("other-call"));
        let local = resolve_literal_place("missing", &different_frame, false, &registry);
        assert_eq!(
            different_frame.contents_presence(&local),
            ContentsPresence::Unknown
        );
        let mut alias = state;
        alias.globals.insert("missing".to_owned());
        let global_alias = resolve_literal_place("missing", &alias, false, &registry);
        assert_eq!(
            alias.contents_presence(&global_alias),
            ContentsPresence::Unknown
        );
    }

    #[test]
    fn called_activation_closure_restores_only_its_exact_selected_frame() {
        let registry = registry();
        let parent = called_frame_with_unknown_namespace_contents("parent-call");
        let child = parent.enter_called_frame(&VariableExecutionFrame::Procedure {
            namespace: "::".to_owned(),
            identity: "child-call".to_owned(),
        });
        let restored = restore_execution_frame(&parent, &child);
        let local = resolve_literal_place("missing", &restored, false, &registry);
        assert_eq!(
            restored.contents_presence(&local),
            ContentsPresence::Undefined
        );
        let mut selected_callback = parent.clone();
        selected_callback.widen();
        let restored = restore_execution_frame(&parent, &selected_callback);
        let local = resolve_literal_place("missing", &restored, false, &registry);
        assert_eq!(
            restored.contents_presence(&local),
            ContentsPresence::Unknown
        );
    }

    #[test]
    fn namespace_presence_join_preserves_identical_definite_cells() {
        let mut cells = NamespaceCellPresence {
            present: VariableCellSet::from(["::x".to_owned()]),
            possible: VariableCellSet::default(),
            closed_namespaces: VariableNamespaceSet::default(),
            closed: true,
        };
        let original = cells.clone();
        assert!(!cells.join(&original));
        assert_eq!(cells, original);
        assert!(cells.absent("::y"));
    }

    #[test]
    fn namespace_presence_join_retains_all_may_cells_and_reports_real_changes() {
        let mut left = NamespaceCellPresence {
            present: VariableCellSet::from(["::common".to_owned(), "::left".to_owned()]),
            possible: VariableCellSet::from(["::prior".to_owned()]),
            closed_namespaces: VariableNamespaceSet::default(),
            closed: true,
        };
        let right = NamespaceCellPresence {
            present: VariableCellSet::from(["::common".to_owned(), "::right".to_owned()]),
            possible: VariableCellSet::from(["::other".to_owned()]),
            closed_namespaces: VariableNamespaceSet::default(),
            closed: false,
        };
        assert!(left.join(&right));
        assert_eq!(left.present, VariableCellSet::from(["::common".to_owned()]));
        assert_eq!(
            left.possible,
            VariableCellSet::from([
                "::left".to_owned(),
                "::right".to_owned(),
                "::prior".to_owned(),
                "::other".to_owned(),
            ])
        );
        assert!(!left.closed);
        let fixed = left.clone();
        assert!(!left.join(&right));
        assert_eq!(left, fixed);
    }

    #[test]
    fn namespace_presence_join_is_independent_of_predecessor_grouping() {
        let a = NamespaceCellPresence {
            present: VariableCellSet::from(["::x".to_owned(), "::y".to_owned()]),
            possible: VariableCellSet::default(),
            closed_namespaces: VariableNamespaceSet::default(),
            closed: true,
        };
        let b = NamespaceCellPresence {
            present: VariableCellSet::from(["::y".to_owned()]),
            possible: VariableCellSet::from(["::z".to_owned()]),
            closed_namespaces: VariableNamespaceSet::default(),
            closed: true,
        };
        let c = NamespaceCellPresence {
            present: VariableCellSet::from(["::x".to_owned()]),
            possible: VariableCellSet::from(["::x".to_owned()]),
            closed_namespaces: VariableNamespaceSet::default(),
            closed: false,
        };
        let mut left = a.clone();
        left.join(&b);
        left.join(&c);
        let mut tail = b.clone();
        tail.join(&c);
        let mut right = a.clone();
        right.join(&tail);
        assert_eq!(left, right);
        let mut reverse = c;
        reverse.join(&b);
        reverse.join(&a);
        assert_eq!(left, reverse);
    }

    #[test]
    fn local_scalar_resolves_to_local_ns() {
        let p = resolve_place("x", &ctx(), false, &registry());
        assert_eq!(p, scalar("x", LOCAL_NS, false));
    }

    #[test]
    fn absolute_scalar_splits_namespace_and_tail() {
        assert_eq!(
            resolve_place("::ns::v", &ctx(), false, &registry()),
            scalar("v", "::ns", false)
        );
        assert_eq!(
            resolve_place("::g", &ctx(), false, &registry()),
            scalar("g", "::", false)
        );
    }

    #[test]
    fn global_and_variable_declarations_change_ns() {
        let mut c = ctx();
        c.globals.insert("g".to_owned());
        assert_eq!(
            resolve_place("g", &c, false, &registry()),
            scalar("g", "::", false)
        );
        let mut c = ctx();
        c.namespace = "::ns".to_owned();
        c.ns_vars.insert("v".to_owned());
        assert_eq!(
            resolve_place("v", &c, false, &registry()),
            scalar("v", "::ns", false)
        );
    }

    #[test]
    fn literal_array_element() {
        let p = resolve_place("a(k)", &ctx(), false, &registry());
        assert_eq!(p.kind, PlaceKind::ArrayElem);
        assert_eq!(p.name, "a");
        assert_eq!(p.ns, LOCAL_NS);
        assert_eq!(p.index, Some(Index::literal("k")));
        assert_eq!(places_read_to_form(&p), [] as [crate::place::Place; 0]);
    }

    #[test]
    fn dynamic_array_element_records_index_reads() {
        // a($i) — element index reads the scalar `i`.
        let p = resolve_place("a($i)", &ctx(), false, &registry());
        assert_eq!(p.kind, PlaceKind::ArrayElem);
        assert_eq!(p.name, "a");
        assert_eq!(places_read_to_form(&p), vec![scalar("i", LOCAL_NS, false)]);
    }

    #[test]
    fn dynamic_name_target_is_unknown_but_reads_the_name_var() {
        // set $X v → the *target* is UNKNOWN, but `X` is read.
        let p = resolve_place("$X", &ctx(), false, &registry());
        assert_eq!(p.kind, PlaceKind::Unknown);
        assert_eq!(places_read_to_form(&p), vec![scalar("X", LOCAL_NS, false)]);
    }

    #[test]
    fn resolved_array_elements_feed_the_overlap_precision() {
        // Distinct literal elements of the same array resolve to
        // non-overlapping places, so a store to one is not a dead store
        // (W220) against the other; a dynamic index conservatively overlaps
        // any sibling.
        let r = registry();
        let ak = resolve_place("a(k)", &ctx(), false, &r);
        let aj = resolve_place("a(j)", &ctx(), false, &r);
        assert!(!crate::place::overlap(&ak, &aj));
        let dyn_elem = resolve_place("a($i)", &ctx(), false, &r);
        assert!(crate::place::overlap(&dyn_elem, &ak));
    }

    #[test]
    fn upvar_alias_binds_to_caller_target() {
        let mut c = ctx();
        c.upvar_aliases
            .insert("y".to_owned(), "::caller::x".to_owned());
        let p = resolve_place("y", &c, false, &registry());
        assert_eq!(p.kind, PlaceKind::UpvarAlias);
        assert_eq!(p.owner, "::caller::x");
        assert!(!p.dynamic);
        // A dynamic (empty-target) upvar alias is marked dynamic.
        let mut c = ctx();
        c.upvar_aliases.insert("z".to_owned(), String::new());
        let p = resolve_place("z", &c, false, &registry());
        assert_eq!(p.kind, PlaceKind::UpvarAlias);
        assert!(p.dynamic);
    }

    #[test]
    fn whole_array_flag_yields_array_whole() {
        let p = resolve_place("a", &ctx(), true, &registry());
        assert_eq!(p.kind, PlaceKind::ArrayWhole);
        assert_eq!(p.name, "a");
    }

    #[test]
    fn traced_name_is_observed() {
        let mut c = ctx();
        c.traced.insert("x".to_owned());
        let p = resolve_place("x", &c, false, &registry());
        assert!(p.observed);
    }

    #[test]
    fn dict_path_classifies_keys() {
        let keys = vec!["a".to_owned(), "b".to_owned()];
        let p = resolve_dict_path("d", &keys, &ctx(), &registry());
        assert_eq!(p.kind, PlaceKind::DictPath);
        assert_eq!(p.name, "d");
        assert_eq!(p.keys, vec![Index::literal("a"), Index::literal("b")]);
    }
    #[test]
    fn unresolved_array_bindings_stay_unknown_and_dict_elements_keep_indices() {
        let registry = registry();
        let mut context = ResolveContext::for_function("::f");
        context.unknown_bindings.insert("a".to_owned());
        assert_eq!(
            resolve_place("a(k)", &context, false, &registry).kind,
            PlaceKind::Unknown
        );
        assert_eq!(
            resolve_dict_path("a(k)", &["key".to_owned()], &context, &registry).kind,
            PlaceKind::Unknown
        );
        context.unknown_bindings.clear();
        let first = resolve_dict_path("a(k)", &["key".to_owned()], &context, &registry);
        let second = resolve_dict_path("a(j)", &["key".to_owned()], &context, &registry);
        assert_eq!(first.index, Some(Index::literal("k")));
        assert!(!place::overlap(&first, &second));
    }

    #[test]
    fn receiver_method_locals_do_not_imply_a_receiver_namespace() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut root = ResolveContext::for_function("::top");
        root.invocation_dialect = registry
            .profile()
            .map(tcl_registry::InvocationDialect::of_profile);
        let frame = VariableExecutionFrame::ReceiverMethod {
            identity: "receiver-method:17".to_owned(),
        };
        let method = root.in_frame(&frame);
        assert!(!method.namespace_known);
        assert_eq!(
            method.alias_frame(),
            tcl_registry::VariableAliasFrame::Procedure
        );
        let local = resolve_literal_place("x", &method, false, registry);
        assert!(matches!(local.cell.as_ref().map(|cell| &cell.owner),
            Some(CellOwner::Activation(identity)) if identity == "receiver-method:17"));
        assert_eq!(
            resolve_literal_place("N::x", &method, false, registry).kind,
            PlaceKind::Unknown
        );
        assert_eq!(
            canonical_binding_value_name(&resolve_literal_place("::x", &method, false, registry)),
            Some("::x".to_owned())
        );
        let relocation = VariableProofRelocation {
            activations: std::collections::BTreeMap::from([(
                "receiver-method:17".to_owned(),
                "receiver-method:29".to_owned(),
            )]),
            ..VariableProofRelocation::default()
        };
        assert_eq!(
            relocation.frame(&frame),
            VariableExecutionFrame::ReceiverMethod {
                identity: "receiver-method:29".to_owned(),
            }
        );
    }

    #[test]
    fn restored_namespace_alias_uses_the_latest_slot_binding() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut parent = ResolveContext::for_function("::top");
        parent.invocation_dialect = Some(tcl_registry::InvocationDialect::for_version(
            tcl_dialect::TclVersion::V8_6,
        ));
        parent.define_literal("a", "OLD", registry);
        parent.define_literal("b", "NEW", registry);
        let a = resolve_literal_place("a", &parent, false, registry);
        let b = resolve_literal_place("b", &parent, false, registry);
        let slot = resolve_alias_destination_slot("link", &parent, registry);
        parent.alias_bindings.insert("link".to_owned(), a.clone());
        parent
            .namespace_alias_bindings
            .insert(cell_key(&slot), a.clone());
        let mut child = parent.enter_called_frame(&VariableExecutionFrame::Procedure {
            namespace: "::".to_owned(),
            identity: "callback:17".to_owned(),
        });
        child.namespace_alias_bindings.insert(cell_key(&slot), b);
        let restored = restore_execution_frame(&parent, &child);
        assert_eq!(restored.literal_value("link", registry), Some("NEW"));
        assert_eq!(restored.literal_value("::link", registry), Some("NEW"));
        // A procedure-local link still selects its own activation slot.
        child.alias_bindings.insert("link".to_owned(), a);
        assert_eq!(child.literal_value("link", registry), Some("OLD"));
    }

    #[test]
    fn frame_restoration_preserves_outward_trace_and_invalidates_deleted_aliases() {
        let registry = registry();
        let mut parent = ResolveContext::for_function("::N::p");
        let target = resolve_place("::N::x", &parent, false, &registry);
        parent.alias_bindings.insert("alias".to_owned(), target);
        let mut child = parent.in_frame(&VariableExecutionFrame::NamespaceActivation {
            namespace: "::N".to_owned(),
            identity: "namespace:20".to_owned(),
        });
        child.traced.insert("::N::x".to_owned());
        let restored = restore_execution_frame(&parent, &child);
        assert!(restored.alias_bindings.contains_key("alias"));
        assert!(resolve_place("alias", &restored, false, &registry).observed);
        child.known_namespaces.remove("::N");
        let restored = restore_execution_frame(&parent, &child);
        assert_eq!(
            resolve_place("alias", &restored, false, &registry).kind,
            PlaceKind::Unknown
        );
    }

    #[test]
    fn jim_namespace_activations_have_separate_cells_from_caller_and_siblings() {
        let registry = registry();
        let mut driver = ResolveContext::default();
        let mut dialect =
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        dialect.variable_lookup_policy = Some(tcl_dialect::VariableLookupPolicy::Jim);
        dialect.namespace_var_global_fallback = Some(false);
        driver.invocation_dialect = Some(dialect);
        let caller = driver.in_frame(&VariableExecutionFrame::Procedure {
            namespace: "::N".to_owned(),
            identity: "proc:N::p".to_owned(),
        });
        let first = caller.in_frame(&VariableExecutionFrame::NamespaceActivation {
            namespace: "::N".to_owned(),
            identity: "namespace:20".to_owned(),
        });
        let second = caller.in_frame(&VariableExecutionFrame::NamespaceActivation {
            namespace: "::N".to_owned(),
            identity: "namespace:40".to_owned(),
        });
        let cells: Vec<_> = [&caller, &first, &second]
            .into_iter()
            .map(|context| resolve_place("x", context, false, &registry))
            .collect();
        assert!(cells.iter().all(|cell| matches!(
            cell.cell.as_ref().map(|cell| &cell.owner),
            Some(CellOwner::Activation(_))
        )));
        assert!(!place::overlap(&cells[0], &cells[1]));
        assert!(!place::overlap(&cells[1], &cells[2]));
        assert_ne!(
            canonical_place_name(&cells[0]),
            canonical_place_name(&cells[1])
        );
    }

    #[test]
    fn tcl8_relative_namespace_lookup_requires_cell_existence_not_namespace_alone() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut context = ResolveContext::for_function("::N::p");
        context.known_namespaces.insert("::N::R".to_owned());
        context.namespace_cells.present.insert("::R::x".to_owned());
        assert!(canonical_variable_name("R::x", &context, registry).is_none());
        context.namespace_cells.closed = true;
        assert_eq!(
            canonical_variable_name("R::x", &context, registry).as_deref(),
            Some("::R::x")
        );
        context
            .namespace_cells
            .present
            .insert("::N::R::x".to_owned());
        assert_eq!(
            canonical_variable_name("R::x", &context, registry).as_deref(),
            Some("::N::R::x")
        );
    }

    #[test]
    fn tcl9_relative_namespace_lookup_never_uses_global_variable_fallback() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.1").commands();
        let mut context = ResolveContext::for_function("::N::p");
        context.known_namespaces.insert("::N::R".to_owned());
        context.namespace_cells.present.insert("::R::x".to_owned());
        assert_eq!(
            canonical_variable_name("R::x", &context, registry).as_deref(),
            Some("::N::R::x")
        );
    }

    #[test]
    fn unknown_interpreter_policy_does_not_choose_a_qualified_variable_table() {
        let registry = CommandRegistry::build_default();
        let mut context = ResolveContext::for_function("::N::p");
        context.known_namespaces.insert("::N::R".to_owned());
        context
            .namespace_cells
            .present
            .insert("::N::R::x".to_owned());
        assert!(canonical_variable_name("R::x", &context, &registry).is_none());
        assert_eq!(
            canonical_variable_name("::N::R::x", &context, &registry).as_deref(),
            Some("::N::R::x")
        );
    }
    #[test]
    fn written_target_braces_preserve_literal_indices_and_dollar_names() {
        let registry = registry();
        let context = ResolveContext::for_function("::p");
        let dynamic = resolve_target_access(
            "a($i)",
            false,
            &context,
            &registry,
            tcl_registry::TraceOperation::Write,
        );
        assert_eq!(
            dynamic.index.as_ref().map(|index| index.kind),
            Some(place::IndexKind::Dynamic)
        );
        assert!(
            places_read_to_form(&dynamic)
                .iter()
                .any(|place| place.name == "i")
        );
        let literal = resolve_target_access(
            "a($i)",
            true,
            &context,
            &registry,
            tcl_registry::TraceOperation::Write,
        );
        assert_eq!(
            literal.index.as_ref().map(|index| index.kind),
            Some(place::IndexKind::Literal)
        );
        assert_eq!(places_read_to_form(&literal), [] as [Place; 0]);
        assert_eq!(
            resolve_target_access(
                "$x",
                true,
                &context,
                &registry,
                tcl_registry::TraceOperation::Write
            )
            .name,
            "$x"
        );
        assert_eq!(
            resolve_target_access(
                "$x",
                false,
                &context,
                &registry,
                tcl_registry::TraceOperation::Write
            )
            .kind,
            PlaceKind::Unknown
        );
    }
    #[test]
    fn namespace_access_origin_retains_explicit_selection_and_imported_aliases() {
        use tcl_registry::TraceOperation::Unset;
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let root = ResolveContext::for_function("::top");
        assert_eq!(
            literal_namespace_access_origin("bare", &root, registry, Unset),
            NamespaceAccessOrigin::CurrentFrame
        );
        assert_eq!(
            literal_namespace_access_origin("::bare", &root, registry, Unset),
            NamespaceAccessOrigin::ExplicitName
        );
        assert_eq!(
            literal_namespace_access_origin("a(::index)", &root, registry, Unset),
            NamespaceAccessOrigin::CurrentFrame
        );
        let mut local = ResolveContext::for_function("::p");
        let target = resolve_literal_place("::shared", &local, false, registry);
        local.alias_bindings.insert("alias".to_owned(), target);
        assert_eq!(
            literal_namespace_access_origin("alias", &local, registry, Unset),
            NamespaceAccessOrigin::NonLocalBinding
        );
        assert_eq!(
            literal_namespace_access_origin("ordinary", &local, registry, Unset),
            NamespaceAccessOrigin::Unknown
        );
    }

    #[test]
    fn unknown_incoming_value_has_provenance_until_a_later_unknown_store() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut frame = ResolveContext::for_function("::p");
        frame.bind_unknown_incoming("parameter", registry);
        let parameter = resolve_literal_access(
            "parameter",
            &frame,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert_eq!(
            frame.contents_presence(&parameter),
            ContentsPresence::Defined
        );
        assert_eq!(frame.contents_origin(&parameter), ContentsOrigin::Incoming);
        assert_eq!(frame.literal_contents_at(&parameter, registry), None);
        assert_eq!(
            frame.incoming_activation_slot("${parameter}", "parameter", registry),
            parameter.cell
        );
        frame.define_unknown_contents("parameter", registry);
        assert_eq!(frame.contents_origin(&parameter), ContentsOrigin::Unknown);
        assert!(
            frame
                .incoming_activation_slot("${parameter}", "parameter", registry)
                .is_none()
        );
    }

    #[test]
    fn incoming_activation_binding_does_not_donate_provenance_to_namespace_storage() {
        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let mut frame = ResolveContext::for_function("::p");
        frame.bind_unknown_incoming("::external", registry);
        let external = resolve_literal_access(
            "::external",
            &frame,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert_eq!(frame.contents_origin(&external), ContentsOrigin::Unknown);
        assert!(
            frame
                .incoming_activation_slot("${::external}", "::external", registry)
                .is_none()
        );
        assert_eq!(
            frame.contents_presence(&external),
            ContentsPresence::Defined
        );
    }

    #[test]
    fn existence_proof_distinguishes_unknown_formal_contents_from_missing_cell() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut frame = ResolveContext::for_function("::p");
        frame.define_unknown_contents("parameter", registry);
        let parameter = resolve_literal_access(
            "parameter",
            &frame,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        let missing = resolve_literal_access(
            "missing",
            &frame,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert_eq!(
            frame.existence_after_read_at(&parameter, registry),
            Some(true)
        );
        assert_eq!(
            frame.existence_after_read_at(&missing, registry),
            Some(false)
        );
        frame.define_unknown_contents("a(b)", registry);
        let array = resolve_literal_access(
            "a",
            &frame,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert_eq!(frame.existence_after_read_at(&array, registry), Some(true));
        let mut observed = parameter;
        observed.observed = true;
        assert_eq!(frame.existence_after_read_at(&observed, registry), None);
    }

    #[test]
    fn closed_container_coercion_is_cost_only_and_withdraws_unknown_inputs() {
        use tcl_syntax::value::ValueRepresentation;
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut state = ResolveContext::for_function("::f");
        state.define_literal_with_representation("d", "a 1", ValueRepresentation::Dict, registry);
        let place = resolve_literal_access(
            "d",
            &state,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        state.coerce_ordinary_container_representations(None, ValueRepresentation::List);
        let domain = state
            .container_representation_alternatives_at(&place)
            .unwrap();
        assert!(domain.contains(ValueRepresentation::List));
        assert!(domain.contains(ValueRepresentation::Dict));
        assert_eq!(
            state.contents_representation_at(&place),
            ValueRepresentation::Unknown
        );
        let mut observed = place.clone();
        observed.observed = true;
        assert!(
            state
                .container_representation_alternatives_at(&observed)
                .is_none()
        );
        state.invalidate_shared_representations();
        assert!(
            state
                .container_representation_alternatives_at(&place)
                .is_none()
        );
    }

    #[test]
    fn closed_representation_joins_keep_only_complete_ordinary_alternatives() {
        use tcl_syntax::value::ValueRepresentation;
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut left = ResolveContext::for_function("::f");
        left.define_literal_with_representation("d", "a 1", ValueRepresentation::Dict, registry);
        let mut right = left.clone();
        right.define_literal_with_representation("d", "a 1", ValueRepresentation::List, registry);
        left.join(&right);
        let place = resolve_literal_access(
            "d",
            &left,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        let domain = left
            .container_representation_alternatives_at(&place)
            .unwrap();
        assert!(domain.contains(ValueRepresentation::List));
        assert!(domain.contains(ValueRepresentation::Dict));
        assert_eq!(
            left.contents_representation_at(&place),
            ValueRepresentation::Unknown
        );
        right.invalidate_shared_representations();
        left.join(&right);
        assert!(
            left.container_representation_alternatives_at(&place)
                .is_none()
        );
    }

    #[test]
    fn known_coerced_bytes_preserve_only_disjoint_current_container_representations() {
        use tcl_syntax::value::ValueRepresentation;
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut state = ResolveContext::for_function("::f");
        state.define_literal_with_representation(
            "same",
            "a 1",
            ValueRepresentation::Dict,
            registry,
        );
        state.define_literal_with_representation(
            "different",
            "b 2",
            ValueRepresentation::Dict,
            registry,
        );
        state.define_unknown_contents_with_representation(
            "unknown",
            ValueRepresentation::List,
            registry,
        );
        let before = state.representation_epoch;
        state.invalidate_representations_for_values(Some(&["a 1".to_owned()]));
        assert_ne!(state.representation_epoch, before);
        for (name, expected) in [
            ("same", ValueRepresentation::Unknown),
            ("different", ValueRepresentation::Dict),
            ("unknown", ValueRepresentation::Unknown),
        ] {
            let place = resolve_literal_place(name, &state, false, registry);
            assert_eq!(state.contents_representation_at(&place), expected, "{name}");
        }
        let mut unknown = state.clone();
        unknown.invalidate_representations_for_values(None);
        let different = resolve_literal_place("different", &unknown, false, registry);
        assert_eq!(
            unknown.contents_representation_at(&different),
            ValueRepresentation::Unknown
        );
    }

    #[test]
    fn coercion_in_child_frame_does_not_restore_ancestor_representation() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut parent = ResolveContext::for_function("::parent");
        parent.define_literal_with_representation(
            "items",
            "1 2",
            tcl_syntax::value::ValueRepresentation::List,
            registry,
        );
        let mut child = parent.enter_called_frame(&VariableExecutionFrame::Procedure {
            namespace: "::".to_owned(),
            identity: "child".to_owned(),
        });
        child.invalidate_shared_representations();
        let selected = child
            .selected_frame_context(tcl_registry::FrameLevel::Relative(1))
            .unwrap();
        let selected_items = resolve_literal_place("items", &selected, false, registry);
        assert_eq!(
            selected.contents_representation_at(&selected_items),
            tcl_syntax::value::ValueRepresentation::Unknown
        );
        let restored = restore_execution_frame(&parent, &child);
        let items = resolve_literal_place("items", &restored, false, registry);
        assert_eq!(restored.literal_contents_at(&items, registry), Some("1 2"));
        assert_eq!(
            restored.contents_representation_at(&items),
            tcl_syntax::value::ValueRepresentation::Unknown
        );
    }

    #[test]
    fn implicit_precision_observer_blocks_raw_values_through_aliases_but_not_locals() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut state = ResolveContext::for_function("::top");
        state.define_literal("tcl_precision", "0x4", registry);
        assert!(state.literal_value("tcl_precision", registry).is_none());
        assert!(
            state
                .substitution_literal_value("$tcl_precision", registry)
                .is_none()
        );
        let stored = resolve_literal_access(
            "tcl_precision",
            &state,
            false,
            registry,
            tcl_registry::TraceOperation::Write,
        );
        assert_eq!(
            state.literal_contents_for_access_at(
                &stored,
                registry,
                tcl_registry::TraceOperation::Write
            ),
            Some("0x4")
        );
        let global = resolve_literal_place("::tcl_precision", &state, false, registry);
        state
            .alias_bindings
            .insert("alias".to_owned(), global.clone());
        assert!(state.literal_value("alias", registry).is_none());
        assert_eq!(
            state.read_contents_origin(&global, registry),
            ContentsOrigin::Unknown
        );
        let mut local = ResolveContext::for_function("::p");
        local.define_literal("tcl_precision", "0x4", registry);
        assert_eq!(local.literal_value("tcl_precision", registry), Some("0x4"));
    }

    #[test]
    fn physical_observer_projection_keeps_root_order_lifo_and_unknown_residual() {
        use tcl_registry::TraceOperation::{Read, Write};
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut state = ResolveContext::for_function("::p");
        let root = resolve_literal_place("a", &state, false, registry);
        let element = resolve_literal_place("a(k)", &state, false, registry);
        let root_key = trace_key(&root);
        let element_key = trace_key(&element);
        state.traced.extend([root_key.clone(), element_key.clone()]);
        state.trace_registrations.insert(
            root_key,
            vec![
                (vec![Write], "first".to_owned()),
                (vec![Write], "last".to_owned()),
            ],
        );
        state.trace_registrations.insert(
            element_key.clone(),
            vec![(vec![Write], "element".to_owned())],
        );
        let projection = state.variable_observers_at(&element, Write, registry);
        assert!(!projection.unknown_residual);
        assert_eq!(
            projection
                .callbacks
                .iter()
                .map(|callback| callback.prefix.as_str())
                .collect::<Vec<_>>(),
            ["last", "first", "element"]
        );
        assert_eq!(
            state
                .variable_observers_at(&element, Read, registry)
                .callbacks
                .len(),
            0
        );
        state.untracked_traces.insert(element_key);
        assert!(
            state
                .variable_observers_at(&element, Write, registry)
                .unknown_residual
        );
    }

    #[test]
    fn an_absent_physical_root_proves_its_elements_absent() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut state = ResolveContext::for_function("::p");
        let element = resolve_literal_access(
            "a(key)",
            &state,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert_eq!(
            state.contents_presence(&element),
            ContentsPresence::Undefined
        );
        let any =
            resolve_array_element_root("a", &state, registry, tcl_registry::TraceOperation::Read);
        assert_eq!(state.contents_presence(&any), ContentsPresence::Undefined);
        state.contents_world = ContentsWorld::Unknown;
        assert_eq!(state.contents_presence(&element), ContentsPresence::Unknown);
        state.contents_world = ContentsWorld::Tracked;
        state.define_unknown_contents("a", registry);
        assert_eq!(state.contents_presence(&element), ContentsPresence::Unknown);
        state.dynamic_traces = true;
        let observed = resolve_literal_access(
            "missing(key)",
            &state,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert_eq!(
            state.contents_presence(&observed),
            ContentsPresence::Unknown
        );
    }

    #[test]
    fn unknown_element_store_masks_only_its_array_and_later_named_store_recovers() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut state = ResolveContext::for_function("::p");
        state.activation = Some("frame".to_owned());
        let element = resolve_literal_place("a(x)", &state, false, registry);
        let untouched = resolve_literal_place("b(x)", &state, false, registry);
        state.record_contents_write(&element, 10, false);
        state.define_literal("a(x)", "OLD", registry);
        state.record_contents_write(&untouched, 11, false);
        state.define_literal("b(x)", "KEPT", registry);
        let any =
            resolve_array_element_root("a", &state, registry, tcl_registry::TraceOperation::Write);
        state.record_contents_write(&any, 20, false);
        assert!(state.literal_value("a(x)", registry).is_none());
        let untracked = resolve_literal_place("a(untracked)", &state, false, registry);
        assert_eq!(state.contents_origin(&untracked), ContentsOrigin::Unknown);
        assert_eq!(
            state.contents_origin(&untouched),
            ContentsOrigin::WrittenAt(11)
        );
        assert_eq!(state.literal_value("b(x)", registry), Some("KEPT"));
        state.record_contents_write(&element, 30, false);
        assert_eq!(
            state.contents_origin(&element),
            ContentsOrigin::WrittenAt(30)
        );
        let mut branch = state.clone();
        branch
            .contents_origins
            .remove(&canonical_binding_value_key(&element).unwrap());
        state.join(&branch);
        assert_eq!(state.contents_origin(&element), ContentsOrigin::Unknown);
    }

    #[test]
    fn unknown_element_destruction_keeps_root_but_not_element_existence() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut state = ResolveContext::for_function("::p");
        let element = resolve_literal_place("a(x)", &state, false, registry);
        let root = resolve_literal_place("a", &state, false, registry);
        state.record_contents_write(&element, 1, false);
        let any =
            resolve_array_element_root("a", &state, registry, tcl_registry::TraceOperation::Unset);
        state.record_unknown_element_destruction(&any);
        assert_eq!(state.contents_presence(&element), ContentsPresence::Unknown);
        assert_eq!(state.contents_presence(&root), ContentsPresence::Defined);
        assert_eq!(state.contents_origin(&element), ContentsOrigin::Unknown);
    }

    #[test]
    fn physical_proof_relocation_round_trips_aliases_contents_and_observers() {
        let registry = CommandRegistry::build_default();
        let mut context = ResolveContext::for_function("::p");
        context.activation = Some("actual-frame".to_owned());
        let mut target = resolve_literal_place("a(k)", &context, false, &registry);
        target.cell.as_mut().expect("bound cell").generation = CellGeneration::After(77);
        context
            .alias_bindings
            .insert("alias".to_owned(), target.clone());
        context
            .name_alias_bindings
            .insert("name_alias".to_owned(), target.clone());
        context
            .generations
            .insert(cell_key(&target), CellGeneration::After(77));
        context.contents_origins.insert(
            canonical_place_name(&target).unwrap(),
            ContentsOrigin::WrittenAt(77),
        );
        context
            .constant_values
            .insert(canonical_place_name(&target).unwrap(), "literal".to_owned());
        context.traced.insert(trace_key(&target));
        context.trace_registrations.insert(
            trace_key(&target),
            vec![(
                vec![tcl_registry::TraceOperation::Read],
                "callback".to_owned(),
            )],
        );
        context.caller = Some(std::sync::Arc::new(ResolveContext::for_function(
            "::external",
        )));
        context
            .contents_unknown_arrays
            .insert(physical_array_key(&target).unwrap());
        let relocation = VariableProofRelocation {
            activations: std::collections::BTreeMap::from([(
                "actual-frame".to_owned(),
                "template-frame".to_owned(),
            )]),
            source_offsets: std::collections::BTreeMap::from([(77, 7)]),
            ..VariableProofRelocation::default()
        };
        let relocated = context.relocated(&relocation);
        assert_eq!(relocated.activation.as_deref(), Some("template-frame"));
        assert_eq!(
            relocated
                .alias_bindings
                .get("alias")
                .unwrap()
                .cell
                .as_ref()
                .unwrap()
                .generation,
            CellGeneration::After(7)
        );
        assert_eq!(
            relocated.caller, context.caller,
            "unmapped caller stays an exact dependency"
        );
        assert_eq!(relocated.relocated(&relocation.inverse().unwrap()), context);
    }
}

#[cfg(test)]
mod root_contents_kind_tests {
    use super::*;

    fn context(profile: &str) -> (ResolveContext, &'static CommandRegistry) {
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let mut context = ResolveContext::for_function("::p");
        context.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
            registry.profile().unwrap(),
        ));
        (context, registry)
    }

    #[test]
    fn root_kind_admission_preserves_native_tcl_and_jim_difference() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let (mut state, registry) = context(profile);
            state.define_literal("a(k)", "OLD", registry);
            let root = resolve_literal_place("a", &state, false, registry);
            assert_eq!(
                state.root_contents_kind(&root),
                Some(RootContentsKind::Array)
            );
            assert_eq!(state.store_would_error(&root), profile != "jim");
        }
    }

    #[test]
    fn conflicting_paths_and_opaque_effects_withdraw_root_kind() {
        let (mut array, registry) = context("tcl8.6");
        array.define_literal("a(k)", "OLD", registry);
        let mut scalar = array.clone();
        crate::variable_bindings::destroy_literal_binding(&mut scalar, "a", 10, registry);
        scalar.define_literal("a", "NEW", registry);
        array.join(&scalar);
        let root = resolve_literal_place("a", &array, false, registry);
        assert_eq!(array.root_contents_kind(&root), None);
        scalar.record_contents_write(&crate::place::unknown_top(), 20, true);
        let root = resolve_literal_place("a", &scalar, false, registry);
        assert_eq!(scalar.root_contents_kind(&root), None);
    }

    #[test]
    fn retired_array_kind_cannot_follow_a_new_same_name_scalar() {
        let (mut state, registry) = context("tcl9.0");
        state.define_literal("a(k)", "OLD", registry);
        let old_root = resolve_literal_place("a", &state, false, registry);
        crate::variable_bindings::destroy_literal_binding(&mut state, "a", 10, registry);
        state.define_literal("a", "NEW", registry);
        assert_eq!(state.root_contents_kind(&old_root), None);
        let new_element = resolve_literal_place("a(k)", &state, false, registry);
        assert!(state.store_would_error(&new_element));
    }
}

#[cfg(test)]
mod worker_execution_ingress_tests {
    use super::*;

    #[test]
    fn reached_helper_and_alias_retain_the_callers_worker_namespace() {
        use tcl_registry::f5::{VariableStorageDomain, WorkerExecution};
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let mut caller = ResolveContext::for_function("::event");
        caller.known_namespaces.insert("::static".to_owned());
        caller.known_namespaces.insert("::app::static".to_owned());
        caller.execution = Some(WorkerExecution {
            initialisation_epoch: 7,
            worker: Some(2),
            connection: Some(31),
        });
        caller.define_literal("::static::counter", "OLD", registry);
        let target = resolve_literal_place("static::counter", &caller, false, registry);
        let mut helper = caller.enter_called_frame(&VariableExecutionFrame::Procedure {
            namespace: "::".to_owned(),
            identity: "helper-call:17".to_owned(),
        });
        assert_eq!(helper.execution, caller.execution);
        helper
            .alias_bindings
            .insert("alias".to_owned(), target.clone());
        let alias = resolve_literal_place("alias", &helper, false, registry);
        assert_eq!(alias.cell, target.cell);
        let cell = alias
            .cell
            .as_ref()
            .expect("selected physical namespace cell");
        assert_eq!(
            cell.storage_domain,
            Some(VariableStorageDomain::WorkerNamespace)
        );
        assert_eq!(cell.execution.unwrap().connection, None);
        assert_eq!(cell.execution.unwrap().worker, Some(2));
        helper.define_literal("alias", "NEW", registry);
        let restored = restore_execution_frame(&caller, &helper);
        assert_eq!(
            restored.literal_value("::static::counter", registry),
            Some("NEW")
        );
        let ordinary = resolve_literal_place("::global", &helper, false, registry);
        assert_eq!(ordinary.cell.as_ref().unwrap().execution, cell.execution);
        let local = resolve_literal_place("local", &helper, false, registry);
        assert_eq!(local.cell.as_ref().unwrap().execution, caller.execution);
        let nested = resolve_literal_place("::app::static::counter", &helper, false, registry);
        assert_ne!(nested.cell, target.cell);
        assert_ne!(
            nested.cell.as_ref().unwrap().storage_domain,
            cell.storage_domain
        );
    }
}

#[cfg(test)]
mod possible_access_domain_tests {
    use super::*;

    #[test]
    fn unknown_namespace_binding_retains_only_a_may_storage_domain() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut state = ResolveContext::for_function("::top");
        state.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
            registry.profile().unwrap(),
        ));
        state.widen();
        assert_eq!(
            possible_access_domain("err", &state, registry),
            PossibleVariableAccessDomain::NamespaceOnly
        );
        assert_eq!(
            resolve_literal_place("err", &state, false, registry).kind,
            PlaceKind::Unknown
        );
    }

    #[test]
    fn native_read_success_requires_contents_kind_lifetime_and_observer_proofs() {
        for profile in ["tcl8.6", "jim"] {
            let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
            let mut state = ResolveContext::for_function("::p");
            state.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
                registry.profile().unwrap(),
            ));
            state.bind_unknown_incoming("parameter", registry);
            let parameter = resolve_literal_place("parameter", &state, false, registry);
            assert!(state.read_produces_value(&parameter, registry));
            let missing = resolve_literal_place("missing", &state, false, registry);
            assert!(!state.read_produces_value(&missing, registry));
            state.define_literal("a(k)", "VALUE", registry);
            let root = resolve_literal_place("a", &state, false, registry);
            let element = resolve_literal_place("a(k)", &state, false, registry);
            assert_eq!(state.read_produces_value(&root, registry), profile == "jim");
            assert!(state.read_produces_value(&element, registry));
            let mut opaque = state.clone();
            opaque.invocation_dialect = None;
            assert!(!opaque.read_produces_value(&parameter, registry));
            let mut observed = state.clone();
            observed.trace_registrations.insert(
                trace_key(&parameter),
                vec![(
                    vec![tcl_registry::TraceOperation::Read],
                    "callback".to_owned(),
                )],
            );
            assert!(!observed.read_produces_value(&parameter, registry));
            let mut retired = parameter.clone();
            retired.cell.as_mut().unwrap().generation = CellGeneration::After(999);
            assert!(!state.read_produces_value(&retired, registry));
            if profile == "jim" {
                state.define_literal("a", "odd", registry);
                assert!(!state.read_produces_value(&element, registry));
                state.define_literal("a", "other VALUE", registry);
                assert!(!state.read_produces_value(&element, registry));
            }
        }
    }

    #[test]
    fn current_local_scalar_store_proves_contents_without_a_unique_incarnation() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut state = ResolveContext::for_function("::p");
        state.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
            registry.profile().unwrap(),
        ));
        let target = resolve_literal_place("value", &state, false, registry);
        state
            .generations
            .insert(cell_key(&target), CellGeneration::Unknown);
        let target = resolve_literal_place("value", &state, false, registry);
        state.record_contents_write(&target, 17, false);
        state.value_representations.insert(
            canonical_binding_value_name(&target).unwrap(),
            tcl_syntax::value::ValueRepresentation::Dict.into(),
        );
        assert!(state.read_produces_value(&target, registry));
        assert_eq!(
            state.contents_representation_at(&target),
            tcl_syntax::value::ValueRepresentation::Dict
        );
        assert!(canonical_place_name(&target).is_none());
        let mut uncertain = state.clone();
        uncertain.contents_origins.insert(
            canonical_binding_value_name(&target).unwrap(),
            ContentsOrigin::Unknown,
        );
        assert!(!uncertain.read_produces_value(&target, registry));
        let mut foreign = target.clone();
        foreign.cell.as_mut().unwrap().owner = CellOwner::Namespace("::".to_owned());
        assert!(!state.current_scalar_slot_contents(&foreign));
        let element = resolve_literal_place("value(k)", &state, false, registry);
        assert!(!state.read_produces_value(&element, registry));
    }

    #[test]
    fn proved_tcl_variable_policy_does_not_require_native_compiler_identity() {
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let mut state = ResolveContext::for_function("::top");
        let mut dialect = tcl_registry::InvocationDialect::of_profile(registry.profile().unwrap());
        dialect.tcl_version = None;
        assert_eq!(
            dialect.variable_lookup_policy,
            Some(tcl_dialect::VariableLookupPolicy::Tcl)
        );
        state.invocation_dialect = Some(dialect);
        state.widen();
        assert_eq!(
            possible_access_domain("err", &state, registry),
            PossibleVariableAccessDomain::NamespaceOnly
        );
        assert_eq!(
            resolve_literal_place("err", &state, false, registry).kind,
            PlaceKind::Unknown
        );
    }

    #[test]
    fn procedure_local_or_unselected_native_engine_does_not_claim_namespace_domain() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let mut state = ResolveContext::for_function("::p");
        state.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
            registry.profile().unwrap(),
        ));
        state.widen();
        assert_eq!(
            possible_access_domain("err", &state, registry),
            PossibleVariableAccessDomain::Any
        );
        assert_eq!(
            possible_access_domain("::err", &state, registry),
            PossibleVariableAccessDomain::NamespaceOnly
        );
        state.invocation_dialect = None;
        assert_eq!(
            possible_access_domain("::err", &state, registry),
            PossibleVariableAccessDomain::Any
        );
    }
    #[test]
    fn deferred_namespace_uncertainty_preserves_local_missing_and_formal_ingress() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut state = ResolveContext::for_function("::p");
        state.binding_identity = BindingIdentity::Bound;
        state.define_literal("::N::external", "old", registry);
        state.bind_unknown_incoming("parameter", registry);
        state.widen_deferred_namespace_inputs();
        let external = resolve_literal_place("::N::external", &state, false, registry);
        let parameter = resolve_literal_place("parameter", &state, false, registry);
        let missing = resolve_literal_place("missing", &state, false, registry);
        assert_eq!(
            state.contents_presence(&external),
            ContentsPresence::Unknown
        );
        assert_eq!(
            state.contents_presence(&parameter),
            ContentsPresence::Defined
        );
        assert_eq!(state.contents_origin(&parameter), ContentsOrigin::Incoming);
        assert_eq!(
            state.contents_presence(&missing),
            ContentsPresence::Undefined
        );
    }
    #[test]
    fn conditional_store_preserves_only_closed_presence_alternatives() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut state = ResolveContext::for_function("::p");
        state.binding_identity = BindingIdentity::Bound;
        state.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
            registry.profile().unwrap(),
        ));
        let target = resolve_literal_place("x", &state, false, registry);
        assert_eq!(
            state.closed_contents_presence(&target),
            Some(ContentsPresence::Undefined)
        );
        state.record_contents_write(&target, 10, true);
        assert_eq!(
            state.closed_contents_presence(&target),
            Some(ContentsPresence::DefinedOrUndefined)
        );
        assert_eq!(state.contents_presence(&target), ContentsPresence::Unknown);
        assert!(!state.read_produces_value(&target, registry));
        state.record_contents_write(&target, 20, false);
        assert_eq!(
            state.closed_contents_presence(&target),
            Some(ContentsPresence::Defined)
        );
        state.widen();
        state.record_contents_write(&target, 30, true);
        assert_eq!(state.closed_contents_presence(&target), None);
    }

    #[test]
    fn closed_presence_join_does_not_create_a_must_value_or_survive_opacity() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut missing = ResolveContext::for_function("::p");
        missing.binding_identity = BindingIdentity::Bound;
        missing.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
            registry.profile().unwrap(),
        ));
        let mut defined = missing.clone();
        defined.define_literal("x", "1", registry);
        defined.join(&missing);
        let read = resolve_literal_place("x", &defined, false, registry);
        assert_eq!(defined.contents_presence(&read), ContentsPresence::Unknown);
        assert_eq!(
            defined.closed_contents_presence(&read),
            Some(ContentsPresence::DefinedOrUndefined)
        );
        defined.widen();
        assert_eq!(defined.closed_contents_presence(&read), None);
        missing.bind_unknown_incoming("x", registry);
        let mut assigned = missing.clone();
        assigned.define_literal("x", "2", registry);
        assigned.join(&missing);
        let read = resolve_literal_place("x", &assigned, false, registry);
        assert_eq!(
            assigned.closed_contents_presence(&read),
            Some(ContentsPresence::Defined)
        );
    }
}
