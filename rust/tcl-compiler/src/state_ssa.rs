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

//! Generic, versioned state-SSA records.
//!
//! Scalar SSA assigns versions to values. State SSA records the versioned
//! state of something that is *not* a scalar value: a variable cell, a command
//! binding table, a trace set, or another mutable semantic domain.  This
//! generic substrate deliberately knows nothing about Tcl command names,
//! registries, or targets. Domain-specific lowering chooses locations and
//! uses the generic [`StateOverlap`] policy to describe wildcard and alias
//! behaviour. The adapters at the end bridge typed registry data, but never
//! command spellings, into that substrate.
//!
//! The substrate records the reaching versions selected by a domain's CFG
//! renamer. It does not infer those versions from block-number order: two CFG
//! blocks with adjacent IDs need not execute consecutively. Future cell-SSA
//! and world-SSA builders should construct the data-flow solution, then use
//! these records for deterministic storage and queries.

use std::collections::BTreeMap;
use std::fmt;

use crate::cfg::BlockId;
use crate::ir::NodeId;

/// A version of a mutable state location.
///
/// Version zero is the implicit incoming state at function entry. Every
/// [`StateDef`], [`StatePhi`], and [`StateClobber`] receives a non-zero,
/// function-local version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct StateVersion(u32);

impl StateVersion {
    /// The implicit version before a function makes any state definition.
    pub const INITIAL: Self = Self(0);

    /// Construct a version from its function-local numeric representation.
    ///
    /// Callers normally obtain non-initial versions from
    /// [`StateSsaBuilder`]. This constructor is available for importers and
    /// CFG renamers that build records directly.
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Return the function-local numeric representation.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Whether this is the implicit function-entry version.
    #[must_use]
    pub fn is_initial(self) -> bool {
        self == Self::INITIAL
    }
}

/// A deterministic anchor for one state operation.
///
/// Multiple state operations generated from a single semantic node or CFG
/// statement use different `ordinal`s. This is necessary for constructs such
/// as a trace-aware load/store pair: attaching both operations only to the
/// source statement would lose their observable order.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StateSite {
    /// A semantic-IR node and an operation-local ordinal.
    Node {
        /// Stable, function-local semantic node identity.
        node: NodeId,
        /// Deterministic order among state operations at this node.
        ordinal: u32,
    },
    /// A CFG anchor and an operation-local ordinal.
    Cfg(CfgStateSite),
    /// A state operation that becomes available only on one CFG edge.
    ///
    /// Completion-qualified mutations use this rather than pretending that
    /// mutually exclusive `OK` and abrupt writes execute sequentially in the
    /// predecessor block. The origin preserves source/instruction provenance
    /// for diagnostics while `predecessor` and `successor` make availability
    /// explicit to every SSA consumer.
    Edge(CfgEdgeStateSite),
}

impl StateSite {
    /// Anchor an operation to a semantic node.
    #[must_use]
    pub fn node(node: NodeId, ordinal: u32) -> Self {
        Self::Node { node, ordinal }
    }

    /// Anchor an operation to a CFG phi position.
    #[must_use]
    pub const fn phi(block: BlockId, ordinal: u32) -> Self {
        Self::Cfg(CfgStateSite {
            block,
            position: CfgStatePosition::Phi { ordinal },
        })
    }

    /// Anchor an operation to a CFG statement.
    #[must_use]
    pub const fn statement(block: BlockId, index: u32, ordinal: u32) -> Self {
        Self::Cfg(CfgStateSite {
            block,
            position: CfgStatePosition::Statement { index, ordinal },
        })
    }

    /// Anchor an operation to a CFG terminator.
    #[must_use]
    pub const fn terminator(block: BlockId, ordinal: u32) -> Self {
        Self::Cfg(CfgStateSite {
            block,
            position: CfgStatePosition::Terminator { ordinal },
        })
    }

    /// Anchor an operation to one executable CFG edge.
    #[must_use]
    pub const fn edge(predecessor: BlockId, successor: BlockId, origin: CfgStatePosition) -> Self {
        Self::Edge(CfgEdgeStateSite {
            predecessor,
            successor,
            origin,
        })
    }
}

/// A CFG-specific state-operation anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CfgStateSite {
    /// Containing CFG block.
    pub block: BlockId,
    /// Position within the block.
    pub position: CfgStatePosition,
}

/// A state-operation anchor associated with one control-flow edge.
///
/// `origin` describes the instruction or terminator that established the
/// transition. It is intentionally separate from `predecessor`: synthetic
/// CFGs may eventually attach an edge operation to a terminator-origin while
/// retaining a different source provenance for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CfgEdgeStateSite {
    /// Block whose terminator selected this edge.
    pub predecessor: BlockId,
    /// Block reached when this edge is selected.
    pub successor: BlockId,
    /// Deterministic origin position in the predecessor's semantic action.
    pub origin: CfgStatePosition,
}

/// A deterministic position within a CFG block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CfgStatePosition {
    /// A state phi at a block entry.
    Phi {
        /// Deterministic order when a block has multiple state phis.
        ordinal: u32,
    },
    /// One operation associated with an IR statement.
    Statement {
        /// Zero-based statement index in the CFG block.
        index: u32,
        /// Deterministic order when one statement has multiple state ops.
        ordinal: u32,
    },
    /// One operation associated with a CFG terminator.
    Terminator {
        /// Deterministic order when a terminator has multiple state ops.
        ordinal: u32,
    },
}

/// Describes whether a state write can affect a state read.
///
/// State domains define their own policy. A cell domain can use structured
/// [`crate::place::Place`] overlap, while a command/trace world can use region
/// and interpreter wildcards. The direction is explicit because a future
/// domain may have an asymmetric read/write relation even though the first two
/// adapters happen to be symmetric.
pub trait StateOverlap<L> {
    /// Return true when a write to `written` may be observed by a read of
    /// `read`.
    fn may_overlap(&self, written: &L, read: &L) -> bool;

    /// Return the location denoting a clobber of every location in this state
    /// domain.
    fn wildcard(&self) -> L;
}

/// A state read together with the version reaching that read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateUse<L> {
    /// State location read by the operation.
    pub location: L,
    /// Version selected by the domain's SSA renamer.
    pub reaching_version: StateVersion,
    /// Stable source or CFG anchor.
    pub site: StateSite,
}

/// A state write that produces a new version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateDef<L> {
    /// State location written by the operation.
    pub location: L,
    /// New version produced by this write.
    pub version: StateVersion,
    /// Stable source or CFG anchor.
    pub site: StateSite,
}

/// A state phi that merges predecessor versions at a CFG block entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatePhi<L> {
    /// State location whose versions are merged.
    pub location: L,
    /// New version produced at the merge.
    pub version: StateVersion,
    /// Predecessor block to incoming state-version mapping.
    ///
    /// A [`BTreeMap`] makes serialisation, graph views, and test output
    /// deterministic regardless of predecessor insertion order.
    pub incoming: BTreeMap<BlockId, StateVersion>,
    /// Whether this phi also merges the function's implicit initial state.
    ///
    /// Ordinary phis receive state only from concrete predecessor blocks. A
    /// loop which targets the function entry has one additional conceptual
    /// predecessor: the call that entered the function before any state
    /// transition ran. `BlockId` intentionally has no sentinel value, so the
    /// entry seed is represented explicitly rather than fabricating a CFG
    /// block or losing the initial version from the merge.
    pub includes_initial: bool,
    /// Stable CFG phi anchor.
    pub site: StateSite,
}

impl<L> StatePhi<L> {
    /// Whether the implicit state at function entry participates in this phi.
    #[must_use]
    pub const fn includes_initial(&self) -> bool {
        self.includes_initial
    }

    /// Iterate all versions flowing into this phi, including the implicit
    /// entry state when [`Self::includes_initial`] is true.
    pub fn incoming_versions(&self) -> impl Iterator<Item = StateVersion> + '_ {
        self.includes_initial
            .then_some(StateVersion::INITIAL)
            .into_iter()
            .chain(self.incoming.values().copied())
    }
}

/// A conservative state write whose location is normally a policy wildcard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateClobber<L> {
    /// Region invalidated by the clobber.
    pub location: L,
    /// New version produced by this clobber.
    pub version: StateVersion,
    /// Stable source or CFG anchor.
    pub site: StateSite,
}

/// A typed state-SSA operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateOp<L> {
    /// Read state without changing it.
    Use(StateUse<L>),
    /// Write a specific state location.
    Def(StateDef<L>),
    /// Merge state versions at a CFG join.
    Phi(StatePhi<L>),
    /// Invalidate an overlapping set of state locations.
    Clobber(StateClobber<L>),
}

impl<L> StateOp<L> {
    /// The operation's stable source or CFG anchor.
    #[must_use]
    pub fn site(&self) -> &StateSite {
        match self {
            Self::Use(op) => &op.site,
            Self::Def(op) => &op.site,
            Self::Phi(op) => &op.site,
            Self::Clobber(op) => &op.site,
        }
    }

    /// The location read, written, merged, or clobbered by this operation.
    #[must_use]
    pub fn location(&self) -> &L {
        match self {
            Self::Use(op) => &op.location,
            Self::Def(op) => &op.location,
            Self::Phi(op) => &op.location,
            Self::Clobber(op) => &op.location,
        }
    }

    /// The version used or produced by this operation.
    #[must_use]
    pub const fn version(&self) -> StateVersion {
        match self {
            Self::Use(op) => op.reaching_version,
            Self::Def(op) => op.version,
            Self::Phi(op) => op.version,
            Self::Clobber(op) => op.version,
        }
    }

    /// Whether this operation produces a new state version.
    #[must_use]
    pub const fn defines_version(&self) -> bool {
        !matches!(self, Self::Use(_))
    }

    /// Return this operation as a state use, if it is one.
    #[must_use]
    pub const fn as_use(&self) -> Option<&StateUse<L>> {
        match self {
            Self::Use(op) => Some(op),
            Self::Def(_) | Self::Phi(_) | Self::Clobber(_) => None,
        }
    }
}

/// Error returned when a state-SSA record set is internally inconsistent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateSsaError {
    /// Two operations claimed the same ordered anchor.
    DuplicateSite(StateSite),
    /// Two definitions attempted to produce the same version.
    DuplicateVersion(StateVersion),
    /// A definition attempted to produce the reserved initial version.
    InitialVersionDefined,
    /// A use or phi incoming edge refers to no definition in this function.
    UnknownReachingVersion {
        /// Use or phi site holding the invalid reference.
        site: StateSite,
        /// Referenced state version.
        version: StateVersion,
    },
    /// The builder cannot allocate another non-zero state version.
    VersionExhausted,
}

impl fmt::Display for StateSsaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateSite(_) => f.write_str("duplicate state-SSA site"),
            Self::DuplicateVersion(version) => {
                write!(f, "duplicate state-SSA version {}", version.raw())
            }
            Self::InitialVersionDefined => {
                f.write_str("the initial state-SSA version cannot be defined")
            }
            Self::UnknownReachingVersion { version, .. } => {
                write!(f, "unknown reaching state-SSA version {}", version.raw())
            }
            Self::VersionExhausted => f.write_str("state-SSA version space exhausted"),
        }
    }
}

impl std::error::Error for StateSsaError {}

/// Deterministic state-SSA records for one function and one state domain.
///
/// The operation vector is sorted by [`StateSite`]. Each operation site is
/// unique, so callers can look up an exact state effect without depending on
/// `HashMap` iteration order. The `L` type supplies the state identity; it need
/// not implement ordering because sites, not location spellings, determine
/// record order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateSsa<L> {
    operations: Vec<StateOp<L>>,
    site_index: BTreeMap<StateSite, usize>,
    definition_index: BTreeMap<StateVersion, usize>,
    uses_by_reaching_version: BTreeMap<StateVersion, Vec<usize>>,
}

impl<L> StateSsa<L> {
    /// Validate and store state operations in deterministic site order.
    ///
    /// Definition versions and the reaching versions carried by uses/phis are
    /// checked here. The caller's domain-specific SSA construction is
    /// responsible for selecting the correct reaching version along CFG edges.
    pub fn new(mut operations: Vec<StateOp<L>>) -> Result<Self, StateSsaError> {
        operations.sort_by(|left, right| left.site().cmp(right.site()));

        let mut site_index = BTreeMap::new();
        let mut definition_index = BTreeMap::new();
        for (index, op) in operations.iter().enumerate() {
            let site = op.site().clone();
            if site_index.insert(site.clone(), index).is_some() {
                return Err(StateSsaError::DuplicateSite(site));
            }
            if op.defines_version() {
                let version = op.version();
                if version.is_initial() {
                    return Err(StateSsaError::InitialVersionDefined);
                }
                if definition_index.insert(version, index).is_some() {
                    return Err(StateSsaError::DuplicateVersion(version));
                }
            }
        }

        let mut uses_by_reaching_version: BTreeMap<StateVersion, Vec<usize>> = BTreeMap::new();
        for (index, op) in operations.iter().enumerate() {
            match op {
                StateOp::Use(use_op) => {
                    Self::validate_reaching_version(
                        &definition_index,
                        &use_op.site,
                        use_op.reaching_version,
                    )?;
                    uses_by_reaching_version
                        .entry(use_op.reaching_version)
                        .or_default()
                        .push(index);
                }
                StateOp::Phi(phi) => {
                    for version in phi.incoming_versions() {
                        Self::validate_reaching_version(&definition_index, &phi.site, version)?;
                    }
                }
                StateOp::Def(_) | StateOp::Clobber(_) => {}
            }
        }

        Ok(Self {
            operations,
            site_index,
            definition_index,
            uses_by_reaching_version,
        })
    }

    fn validate_reaching_version(
        definitions: &BTreeMap<StateVersion, usize>,
        site: &StateSite,
        version: StateVersion,
    ) -> Result<(), StateSsaError> {
        if version.is_initial() || definitions.contains_key(&version) {
            Ok(())
        } else {
            Err(StateSsaError::UnknownReachingVersion {
                site: site.clone(),
                version,
            })
        }
    }

    /// State operations in deterministic [`StateSite`] order.
    #[must_use]
    pub fn operations(&self) -> &[StateOp<L>] {
        &self.operations
    }

    /// Return the state operation at an exact source or CFG anchor.
    #[must_use]
    pub fn operation_at(&self, site: &StateSite) -> Option<&StateOp<L>> {
        self.site_index
            .get(site)
            .and_then(|index| self.operations.get(*index))
    }

    /// Return the operation attached to one edge and operation-local ordinal.
    #[must_use]
    pub fn operation_on_edge(
        &self,
        predecessor: BlockId,
        successor: BlockId,
        origin: CfgStatePosition,
    ) -> Option<&StateOp<L>> {
        self.operation_at(&StateSite::edge(predecessor, successor, origin))
    }

    /// Return the operation defining `version`, if it is non-initial.
    #[must_use]
    pub fn definition(&self, version: StateVersion) -> Option<&StateOp<L>> {
        self.definition_index
            .get(&version)
            .and_then(|index| self.operations.get(*index))
    }

    /// Return the reaching version recorded at a state-use site.
    #[must_use]
    pub fn reaching_version_at(&self, site: &StateSite) -> Option<StateVersion> {
        self.operation_at(site)
            .and_then(StateOp::as_use)
            .map(|use_op| use_op.reaching_version)
    }

    /// Return the reaching version at `site` when that site reads `location`.
    ///
    /// The equality test is intentionally exact. Use
    /// [`Self::definitions_overlapping`] when the domain needs an
    /// overlap-aware query.
    #[must_use]
    pub fn reaching_version_for(&self, site: &StateSite, location: &L) -> Option<StateVersion>
    where
        L: PartialEq,
    {
        self.operation_at(site)
            .and_then(StateOp::as_use)
            .filter(|use_op| &use_op.location == location)
            .map(|use_op| use_op.reaching_version)
    }

    /// Return all use records that reference `version`, in deterministic site
    /// order.
    #[must_use]
    pub fn uses_reaching(&self, version: StateVersion) -> Vec<&StateUse<L>> {
        self.uses_by_reaching_version
            .get(&version)
            .into_iter()
            .flatten()
            .filter_map(|index| self.operations.get(*index))
            .filter_map(StateOp::as_use)
            .collect()
    }

    /// Return every state definition whose written region may affect
    /// `location`, in deterministic site order.
    #[must_use]
    pub fn definitions_overlapping<'a, P>(&'a self, location: &L, policy: &P) -> Vec<&'a StateOp<L>>
    where
        P: StateOverlap<L>,
    {
        self.operations
            .iter()
            .filter(|op| op.defines_version() && policy.may_overlap(op.location(), location))
            .collect()
    }

    /// Return whether `version`'s definition may affect `location` under the
    /// supplied state-domain policy.
    #[must_use]
    pub fn definition_may_overlap<P>(&self, version: StateVersion, location: &L, policy: &P) -> bool
    where
        P: StateOverlap<L>,
    {
        self.definition(version)
            .is_some_and(|op| policy.may_overlap(op.location(), location))
    }
}

/// Incrementally builds a validated [`StateSsa`] record set.
///
/// This convenience builder allocates unique versions, but deliberately does
/// not perform dominance-frontier calculation or SSA renaming. Callers supply
/// the reaching version for every use and phi incoming edge after their own
/// CFG data-flow pass has selected it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateSsaBuilder<L> {
    next_version: u32,
    operations: Vec<StateOp<L>>,
}

impl<L> Default for StateSsaBuilder<L> {
    fn default() -> Self {
        Self {
            next_version: 1,
            operations: Vec::new(),
        }
    }
}

impl<L> StateSsaBuilder<L> {
    /// Create an empty state-SSA builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn allocate_version(&mut self) -> Result<StateVersion, StateSsaError> {
        let version = StateVersion::new(self.next_version);
        self.next_version = self
            .next_version
            .checked_add(1)
            .ok_or(StateSsaError::VersionExhausted)?;
        Ok(version)
    }

    /// Record a state read with the version selected by the caller's renamer.
    pub fn use_at(&mut self, site: StateSite, location: L, reaching_version: StateVersion) {
        self.operations.push(StateOp::Use(StateUse {
            location,
            reaching_version,
            site,
        }));
    }

    /// Record a state write and return its newly allocated version.
    pub fn def(&mut self, site: StateSite, location: L) -> Result<StateVersion, StateSsaError> {
        let version = self.allocate_version()?;
        self.operations.push(StateOp::Def(StateDef {
            location,
            version,
            site,
        }));
        Ok(version)
    }

    /// Record a state phi and return its newly allocated version.
    pub fn phi(
        &mut self,
        site: StateSite,
        location: L,
        incoming: BTreeMap<BlockId, StateVersion>,
    ) -> Result<StateVersion, StateSsaError> {
        self.phi_with_initial(site, location, incoming, false)
    }

    /// Record a state phi with an explicit implicit-entry input when needed.
    ///
    /// Most CFG joins use [`Self::phi`]. Set `includes_initial` only for a
    /// function-entry join with a reachable backedge; it records the state
    /// present before the function started without inventing a CFG block.
    pub fn phi_with_initial(
        &mut self,
        site: StateSite,
        location: L,
        incoming: BTreeMap<BlockId, StateVersion>,
        includes_initial: bool,
    ) -> Result<StateVersion, StateSsaError> {
        let version = self.allocate_version()?;
        self.operations.push(StateOp::Phi(StatePhi {
            location,
            version,
            incoming,
            includes_initial,
            site,
        }));
        Ok(version)
    }

    /// Record a clobber of `location` and return its newly allocated version.
    pub fn clobber(&mut self, site: StateSite, location: L) -> Result<StateVersion, StateSsaError> {
        let version = self.allocate_version()?;
        self.operations.push(StateOp::Clobber(StateClobber {
            location,
            version,
            site,
        }));
        Ok(version)
    }

    /// Record a policy-defined whole-domain clobber.
    pub fn clobber_all<P>(
        &mut self,
        site: StateSite,
        policy: &P,
    ) -> Result<StateVersion, StateSsaError>
    where
        P: StateOverlap<L>,
    {
        self.clobber(site, policy.wildcard())
    }

    /// Validate and finish the state-SSA record set.
    pub fn finish(self) -> Result<StateSsa<L>, StateSsaError> {
        StateSsa::new(self.operations)
    }
}

/// Domain adapters that exercise the generic substrate without encoding any
/// command recognition or backend-specific policy.
pub mod adapters {
    use crate::place::{Place, overlap, unknown_top};

    use super::StateOverlap;

    /// `Place` overlap policy for a future cell-SSA adapter.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct PlaceOverlapPolicy;

    impl StateOverlap<Place> for PlaceOverlapPolicy {
        fn may_overlap(&self, written: &Place, read: &Place) -> bool {
            overlap(written, read)
        }

        fn wildcard(&self) -> Place {
            unknown_top()
        }
    }

    /// A target-neutral mutable world-state domain.
    ///
    /// This is the compiler projection of
    /// [`tcl_registry::WorldStateDomain`]. It keeps the existing external
    /// resource taxonomy distinct while also modelling Tcl command lookup,
    /// trace, namespace, object-dispatch, package, and variable-store state.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum WorldRegionKind {
        /// Child-interpreter existence and path-to-interpreter identity.
        InterpreterTopology,
        /// Name-to-command identity bindings in an interpreter namespace.
        CommandBindings,
        /// Namespace lookup, imports, exports, and path state.
        NamespaceLookup,
        /// Namespace-specific `unknown` handler state.
        NamespaceUnknown,
        /// Command execution trace registrations.
        ExecutionTraces,
        /// Variable trace registrations.
        VariableTraces,
        /// Command rename/delete trace registrations.
        CommandTraces,
        /// `TclOO` class, object, and method dispatch state.
        ObjectDispatch,
        /// Safe-interpreter and hidden-command policy state.
        InterpreterPolicy,
        /// Package-loading state.
        PackageState,
        /// Host-provided capability state.
        HostCapabilities,
        /// Tcl variable cells when no narrower cell identity is available.
        VariableStore,
        /// An external resource kept from the existing structured
        /// side-effect model.
        ExternalResource(tcl_registry::side_effects::SideEffectTarget),
    }

    impl WorldRegionKind {
        const fn is_namespace_owned(self) -> bool {
            matches!(
                self,
                Self::CommandBindings
                    | Self::NamespaceLookup
                    | Self::NamespaceUnknown
                    | Self::ExecutionTraces
                    | Self::VariableTraces
                    | Self::CommandTraces
                    | Self::ObjectDispatch
                    | Self::VariableStore
            )
        }

        /// Project a registry mutable-world domain without command-name
        /// recognition in the compiler.
        #[must_use]
        pub const fn from_registry(domain: tcl_registry::WorldStateDomain) -> Self {
            match domain {
                tcl_registry::WorldStateDomain::InterpreterTopology => Self::InterpreterTopology,
                tcl_registry::WorldStateDomain::CommandBindings => Self::CommandBindings,
                tcl_registry::WorldStateDomain::NamespaceLookup => Self::NamespaceLookup,
                tcl_registry::WorldStateDomain::NamespaceUnknown => Self::NamespaceUnknown,
                tcl_registry::WorldStateDomain::ExecutionTraces => Self::ExecutionTraces,
                tcl_registry::WorldStateDomain::VariableTraces => Self::VariableTraces,
                tcl_registry::WorldStateDomain::CommandTraces => Self::CommandTraces,
                tcl_registry::WorldStateDomain::OoDispatch => Self::ObjectDispatch,
                tcl_registry::WorldStateDomain::InterpreterPolicy => Self::InterpreterPolicy,
                tcl_registry::WorldStateDomain::PackageState => Self::PackageState,
                tcl_registry::WorldStateDomain::HostCapabilities => Self::HostCapabilities,
                tcl_registry::WorldStateDomain::VariableStore => Self::VariableStore,
                tcl_registry::WorldStateDomain::LegacyExternal(target) => {
                    Self::ExternalResource(target)
                }
            }
        }
    }

    /// The interpreter scope of a compiler world-state location.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum WorldInterpreterScope {
        /// The interpreter current at the operation site.
        Current,
        /// One identified interpreter.
        Named(String),
        /// An unknown interpreter or every interpreter.
        Any,
    }

    impl WorldInterpreterScope {
        /// Construct a named interpreter scope.
        #[must_use]
        pub fn named(name: impl Into<String>) -> Self {
            Self::Named(name.into())
        }

        fn from_registry(scope: &tcl_registry::InterpreterScope) -> Self {
            match scope {
                tcl_registry::InterpreterScope::Current => Self::Current,
                tcl_registry::InterpreterScope::Named(name) => Self::named(name.as_ref()),
                tcl_registry::InterpreterScope::Any => Self::Any,
            }
        }
    }

    /// The namespace scope of a compiler world-state location.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum WorldNamespaceScope {
        /// The namespace current at the operation site.
        Current,
        /// One identified namespace.
        Named(String),
        /// An unknown namespace or every namespace.
        Any,
    }

    impl WorldNamespaceScope {
        /// Construct a named namespace scope.
        #[must_use]
        pub fn named(name: impl Into<String>) -> Self {
            Self::Named(name.into())
        }

        fn from_registry(scope: &tcl_registry::NamespaceScope) -> Self {
            match scope {
                tcl_registry::NamespaceScope::Current => Self::Current,
                tcl_registry::NamespaceScope::Named(name) => Self::named(name.as_ref()),
                tcl_registry::NamespaceScope::Any => Self::Any,
            }
        }
    }

    /// The domain-specific subject of a compiler world-state location.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum WorldSubjectScope {
        /// One identified command, variable, package, object, or resource.
        Named(String),
        /// Every subject in the selected domain and scopes.
        Wildcard,
    }

    impl WorldSubjectScope {
        /// Construct a named subject scope.
        #[must_use]
        pub fn named(name: impl Into<String>) -> Self {
            Self::Named(name.into())
        }

        fn from_registry(scope: &tcl_registry::SubjectScope) -> Self {
            match scope {
                tcl_registry::SubjectScope::Named(name) => Self::named(name.as_ref()),
                tcl_registry::SubjectScope::Wildcard => Self::Wildcard,
            }
        }
    }

    /// A precise or wildcard state location in the mutable interpreter world.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum WorldRegion {
        /// One state domain constrained by interpreter, namespace, and subject
        /// scopes. Any scope may itself be a wildcard.
        Scoped {
            /// State-domain category.
            kind: WorldRegionKind,
            /// Interpreter scope.
            interpreter: WorldInterpreterScope,
            /// Namespace scope.
            namespace: WorldNamespaceScope,
            /// Domain-specific subject scope.
            subject: WorldSubjectScope,
        },
        /// Every state domain in the given interpreter scope.
        InterpreterWildcard {
            /// Interpreter scope.
            interpreter: WorldInterpreterScope,
        },
        /// Every mutable namespace-owned region in one namespace and its
        /// descendants.  Recursive `namespace delete` uses this instead of
        /// discarding a known literal root into an all-world wildcard.
        NamespaceSubtree {
            /// Interpreter containing the namespace tree.
            interpreter: WorldInterpreterScope,
            /// Root namespace of the recursively affected tree.
            namespace: WorldNamespaceScope,
        },
        /// Namespace lookup state for one namespace and every ancestor on
        /// its creation path. `namespace eval ::a::b` may create `::a`
        /// before it creates `::a::b`.
        NamespaceLineage {
            /// Interpreter containing the namespace lineage.
            interpreter: WorldInterpreterScope,
            /// Deepest namespace that may be created.
            namespace: WorldNamespaceScope,
        },
        /// Every mutable state domain in every interpreter.
        Any,
    }

    impl WorldRegion {
        /// Create one scoped world-state region.
        #[must_use]
        pub fn scoped(
            kind: WorldRegionKind,
            interpreter: WorldInterpreterScope,
            namespace: WorldNamespaceScope,
            subject: WorldSubjectScope,
        ) -> Self {
            Self::Scoped {
                kind,
                interpreter,
                namespace,
                subject,
            }
        }

        /// Create one exact-subject world-state region.
        #[must_use]
        pub fn exact(
            kind: WorldRegionKind,
            interpreter: WorldInterpreterScope,
            namespace: WorldNamespaceScope,
            subject: impl Into<String>,
        ) -> Self {
            Self::scoped(
                kind,
                interpreter,
                namespace,
                WorldSubjectScope::named(subject),
            )
        }

        /// Create a wildcard-subject region in one state domain.
        #[must_use]
        pub fn domain_wildcard(
            kind: WorldRegionKind,
            interpreter: WorldInterpreterScope,
            namespace: WorldNamespaceScope,
        ) -> Self {
            Self::scoped(kind, interpreter, namespace, WorldSubjectScope::Wildcard)
        }

        /// Create a recursive namespace-tree location.
        #[must_use]
        pub fn namespace_subtree(
            interpreter: WorldInterpreterScope,
            namespace: WorldNamespaceScope,
        ) -> Self {
            Self::NamespaceSubtree {
                interpreter,
                namespace,
            }
        }

        /// Create a namespace creation-lineage location.
        #[must_use]
        pub fn namespace_lineage(
            interpreter: WorldInterpreterScope,
            namespace: WorldNamespaceScope,
        ) -> Self {
            Self::NamespaceLineage {
                interpreter,
                namespace,
            }
        }

        /// Project one registry-owned resolved effect access.
        #[must_use]
        pub fn from_effect_access(access: &tcl_registry::EffectAccess) -> Self {
            let kind = WorldRegionKind::from_registry(access.domain);
            let (interpreter, namespace) = match kind {
                // Existing external-resource descriptors are not namespaced
                // Tcl state. The legacy bridge currently supplies `Current`
                // scopes for them, so retaining that namespace would prove
                // unrelated scripts disjoint when they can observe the same
                // connection or host resource.
                WorldRegionKind::ExternalResource(_) | WorldRegionKind::HostCapabilities => {
                    (WorldInterpreterScope::Any, WorldNamespaceScope::Any)
                }
                // Package and interpreter policy state are scoped to an
                // interpreter, never to its current namespace.
                WorldRegionKind::InterpreterTopology
                | WorldRegionKind::InterpreterPolicy
                | WorldRegionKind::PackageState => (
                    WorldInterpreterScope::from_registry(&access.interpreter),
                    WorldNamespaceScope::Any,
                ),
                _ => (
                    WorldInterpreterScope::from_registry(&access.interpreter),
                    WorldNamespaceScope::from_registry(&access.namespace),
                ),
            };
            Self::scoped(
                kind,
                interpreter,
                namespace,
                WorldSubjectScope::from_registry(&access.subject),
            )
        }
    }

    /// Optional identities that make `Current` scope comparisons precise.
    ///
    /// Without this context, `Current` conservatively overlaps every named
    /// interpreter or namespace because it may resolve to that name at run
    /// time. A future function/interpreter owner can supply stable current
    /// identities and recover disjointness where they differ.
    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct WorldScopeContext {
        /// Stable identity of the current interpreter, when known.
        pub current_interpreter: Option<String>,
        /// Stable identity of the current namespace, when known.
        pub current_namespace: Option<String>,
    }

    impl WorldScopeContext {
        /// Construct a context with known current interpreter and namespace
        /// identities.
        #[must_use]
        pub fn named(interpreter: impl Into<String>, namespace: impl Into<String>) -> Self {
            Self {
                current_interpreter: Some(interpreter.into()),
                current_namespace: Some(namespace.into()),
            }
        }
    }

    /// Overlap policy for the typed [`WorldRegion`] vocabulary.
    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct WorldRegionOverlapPolicy {
        context: WorldScopeContext,
    }

    impl WorldRegionOverlapPolicy {
        /// Construct the conservative policy with no resolved current scopes.
        #[must_use]
        pub fn conservative() -> Self {
            Self::default()
        }

        /// Construct a policy using stable current interpreter and namespace
        /// identities to prove some scoped regions disjoint.
        #[must_use]
        pub fn with_context(context: WorldScopeContext) -> Self {
            Self { context }
        }

        fn interpreters_may_overlap(
            &self,
            left: &WorldInterpreterScope,
            right: &WorldInterpreterScope,
        ) -> bool {
            match (left, right) {
                (WorldInterpreterScope::Any, _)
                | (_, WorldInterpreterScope::Any)
                | (WorldInterpreterScope::Current, WorldInterpreterScope::Current) => true,
                (WorldInterpreterScope::Named(left), WorldInterpreterScope::Named(right)) => {
                    left == right
                }
                (WorldInterpreterScope::Current, WorldInterpreterScope::Named(named))
                | (WorldInterpreterScope::Named(named), WorldInterpreterScope::Current) => self
                    .context
                    .current_interpreter
                    .as_ref()
                    .is_none_or(|current| current == named),
            }
        }

        fn namespaces_may_overlap(
            &self,
            left: &WorldNamespaceScope,
            right: &WorldNamespaceScope,
        ) -> bool {
            match (left, right) {
                (WorldNamespaceScope::Any, _)
                | (_, WorldNamespaceScope::Any)
                | (WorldNamespaceScope::Current, WorldNamespaceScope::Current) => true,
                (WorldNamespaceScope::Named(left), WorldNamespaceScope::Named(right)) => {
                    left == right
                }
                (WorldNamespaceScope::Current, WorldNamespaceScope::Named(named))
                | (WorldNamespaceScope::Named(named), WorldNamespaceScope::Current) => self
                    .context
                    .current_namespace
                    .as_ref()
                    .is_none_or(|current| current == named),
            }
        }

        fn namespace_is_in_subtree(
            root: &WorldNamespaceScope,
            candidate: &WorldNamespaceScope,
        ) -> bool {
            match (root, candidate) {
                (WorldNamespaceScope::Any | WorldNamespaceScope::Current, _)
                | (_, WorldNamespaceScope::Any | WorldNamespaceScope::Current) => true,
                (WorldNamespaceScope::Named(root), WorldNamespaceScope::Named(candidate)) => {
                    root == candidate
                        || root == "::"
                        || candidate
                            .strip_prefix(root)
                            .is_some_and(|rest| rest.starts_with("::"))
                }
            }
        }

        fn subjects_may_overlap(left: &WorldSubjectScope, right: &WorldSubjectScope) -> bool {
            match (left, right) {
                (WorldSubjectScope::Wildcard, _) | (_, WorldSubjectScope::Wildcard) => true,
                (WorldSubjectScope::Named(left), WorldSubjectScope::Named(right)) => left == right,
            }
        }

        fn interpreter_scope(region: &WorldRegion) -> Option<&WorldInterpreterScope> {
            match region {
                WorldRegion::Scoped { interpreter, .. }
                | WorldRegion::InterpreterWildcard { interpreter }
                | WorldRegion::NamespaceSubtree { interpreter, .. }
                | WorldRegion::NamespaceLineage { interpreter, .. } => Some(interpreter),
                WorldRegion::Any => None,
            }
        }

        fn subtree_scoped_overlap(
            &self,
            subtree_interpreter: &WorldInterpreterScope,
            subtree_namespace: &WorldNamespaceScope,
            scoped_kind: WorldRegionKind,
            scoped_interpreter: &WorldInterpreterScope,
            scoped_namespace: &WorldNamespaceScope,
        ) -> bool {
            scoped_kind.is_namespace_owned()
                && self.interpreters_may_overlap(subtree_interpreter, scoped_interpreter)
                && Self::namespace_is_in_subtree(subtree_namespace, scoped_namespace)
        }

        fn lineage_scoped_overlap(
            &self,
            lineage_interpreter: &WorldInterpreterScope,
            lineage_namespace: &WorldNamespaceScope,
            scoped_kind: WorldRegionKind,
            scoped_interpreter: &WorldInterpreterScope,
            scoped_namespace: &WorldNamespaceScope,
        ) -> bool {
            scoped_kind == WorldRegionKind::NamespaceLookup
                && self.interpreters_may_overlap(lineage_interpreter, scoped_interpreter)
                && Self::namespace_is_in_subtree(scoped_namespace, lineage_namespace)
        }

        fn namespace_regions_overlap(
            &self,
            written_interpreter: &WorldInterpreterScope,
            written_namespace: &WorldNamespaceScope,
            read_interpreter: &WorldInterpreterScope,
            read_namespace: &WorldNamespaceScope,
        ) -> bool {
            self.interpreters_may_overlap(written_interpreter, read_interpreter)
                && (Self::namespace_is_in_subtree(written_namespace, read_namespace)
                    || Self::namespace_is_in_subtree(read_namespace, written_namespace))
        }

        fn scoped_parts(
            region: &WorldRegion,
        ) -> Option<(
            &WorldRegionKind,
            &WorldInterpreterScope,
            &WorldNamespaceScope,
            &WorldSubjectScope,
        )> {
            let WorldRegion::Scoped {
                kind,
                interpreter,
                namespace,
                subject,
            } = region
            else {
                return None;
            };
            Some((kind, interpreter, namespace, subject))
        }

        fn subtree_parts(
            region: &WorldRegion,
        ) -> Option<(&WorldInterpreterScope, &WorldNamespaceScope)> {
            let WorldRegion::NamespaceSubtree {
                interpreter,
                namespace,
            } = region
            else {
                return None;
            };
            Some((interpreter, namespace))
        }

        fn lineage_parts(
            region: &WorldRegion,
        ) -> Option<(&WorldInterpreterScope, &WorldNamespaceScope)> {
            let WorldRegion::NamespaceLineage {
                interpreter,
                namespace,
            } = region
            else {
                return None;
            };
            Some((interpreter, namespace))
        }

        fn namespace_parts(
            region: &WorldRegion,
        ) -> Option<(&WorldInterpreterScope, &WorldNamespaceScope)> {
            Self::subtree_parts(region).or_else(|| Self::lineage_parts(region))
        }

        fn scoped_overlap(&self, written: &WorldRegion, read: &WorldRegion) -> Option<bool> {
            let (written_kind, written_interpreter, written_namespace, written_subject) =
                Self::scoped_parts(written)?;
            let (read_kind, read_interpreter, read_namespace, read_subject) =
                Self::scoped_parts(read)?;
            Some(
                written_kind == read_kind
                    && self.interpreters_may_overlap(written_interpreter, read_interpreter)
                    && self.namespaces_may_overlap(written_namespace, read_namespace)
                    && Self::subjects_may_overlap(written_subject, read_subject),
            )
        }

        fn namespace_overlap(&self, written: &WorldRegion, read: &WorldRegion) -> Option<bool> {
            if let (
                Some((written_interpreter, written_namespace)),
                Some((read_interpreter, read_namespace)),
            ) = (Self::namespace_parts(written), Self::namespace_parts(read))
            {
                return Some(self.namespace_regions_overlap(
                    written_interpreter,
                    written_namespace,
                    read_interpreter,
                    read_namespace,
                ));
            }
            if let (
                Some((interpreter, namespace)),
                Some((kind, scoped_interpreter, scoped_namespace, _)),
            ) = (Self::subtree_parts(written), Self::scoped_parts(read))
            {
                return Some(self.subtree_scoped_overlap(
                    interpreter,
                    namespace,
                    *kind,
                    scoped_interpreter,
                    scoped_namespace,
                ));
            }
            if let (
                Some((kind, scoped_interpreter, scoped_namespace, _)),
                Some((interpreter, namespace)),
            ) = (Self::scoped_parts(written), Self::subtree_parts(read))
            {
                return Some(self.subtree_scoped_overlap(
                    interpreter,
                    namespace,
                    *kind,
                    scoped_interpreter,
                    scoped_namespace,
                ));
            }
            if let (
                Some((interpreter, namespace)),
                Some((kind, scoped_interpreter, scoped_namespace, _)),
            ) = (Self::lineage_parts(written), Self::scoped_parts(read))
            {
                return Some(self.lineage_scoped_overlap(
                    interpreter,
                    namespace,
                    *kind,
                    scoped_interpreter,
                    scoped_namespace,
                ));
            }
            if let (
                Some((kind, scoped_interpreter, scoped_namespace, _)),
                Some((interpreter, namespace)),
            ) = (Self::scoped_parts(written), Self::lineage_parts(read))
            {
                return Some(self.lineage_scoped_overlap(
                    interpreter,
                    namespace,
                    *kind,
                    scoped_interpreter,
                    scoped_namespace,
                ));
            }
            None
        }
    }

    impl StateOverlap<WorldRegion> for WorldRegionOverlapPolicy {
        fn may_overlap(&self, written: &WorldRegion, read: &WorldRegion) -> bool {
            let (Some(written_interpreter), Some(read_interpreter)) = (
                Self::interpreter_scope(written),
                Self::interpreter_scope(read),
            ) else {
                return true;
            };

            self.scoped_overlap(written, read)
                .or_else(|| self.namespace_overlap(written, read))
                .unwrap_or_else(|| {
                    self.interpreters_may_overlap(written_interpreter, read_interpreter)
                })
        }

        fn wildcard(&self) -> WorldRegion {
            WorldRegion::Any
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::cfg::BlockId;
    use crate::ir::NodeId;
    use crate::place::{Index, LOCAL_NS, array_elem, array_whole, scalar};

    use super::adapters::{
        PlaceOverlapPolicy, WorldInterpreterScope, WorldNamespaceScope, WorldRegion,
        WorldRegionKind, WorldRegionOverlapPolicy, WorldScopeContext,
    };
    use super::{
        CfgStatePosition, StateClobber, StateDef, StateOp, StateOverlap, StateSite, StateSsa,
        StateSsaBuilder, StateVersion,
    };

    #[test]
    fn state_records_are_deterministic_and_query_reaching_versions() {
        let location = scalar("count", LOCAL_NS, false);
        let definition_site = StateSite::node(NodeId::from_path(vec![4]), 0);
        let use_site = StateSite::node(NodeId::from_path(vec![8]), 0);
        let phi_site = StateSite::phi(BlockId(3), 0);
        let clobber_site = StateSite::statement(BlockId(3), 2, 0);

        let mut builder = StateSsaBuilder::new();
        let initial_definition = builder
            .def(definition_site.clone(), location.clone())
            .expect("first state version");
        builder.use_at(use_site.clone(), location.clone(), initial_definition);
        let phi = builder
            .phi(
                phi_site.clone(),
                location.clone(),
                BTreeMap::from([
                    (BlockId(1), initial_definition),
                    (BlockId(2), StateVersion::INITIAL),
                ]),
            )
            .expect("phi state version");
        let clobber = builder
            .clobber_all(clobber_site.clone(), &PlaceOverlapPolicy)
            .expect("clobber state version");
        let state = builder.finish().expect("valid state records");

        assert_eq!(
            state.reaching_version_at(&use_site),
            Some(initial_definition)
        );
        assert_eq!(
            state.reaching_version_for(&use_site, &location),
            Some(initial_definition)
        );
        assert_eq!(
            state.definition(phi).expect("phi definition").site(),
            &phi_site
        );
        assert_eq!(
            state
                .definition(clobber)
                .expect("clobber definition")
                .site(),
            &clobber_site
        );
        assert_eq!(state.uses_reaching(initial_definition).len(), 1);

        let sites: Vec<StateSite> = state
            .operations()
            .iter()
            .map(|op| op.site().clone())
            .collect();
        let mut expected = vec![definition_site, use_site, phi_site, clobber_site];
        expected.sort();
        assert_eq!(sites, expected);
    }

    #[test]
    fn entry_seed_is_an_explicit_phi_input_without_a_sentinel_block() {
        let location = scalar("binding", LOCAL_NS, false);
        let mut builder = StateSsaBuilder::new();
        let loop_value = builder
            .def(StateSite::statement(BlockId(1), 0, 0), location.clone())
            .expect("loop definition");
        let phi = builder
            .phi_with_initial(
                StateSite::phi(BlockId(0), 0),
                location,
                BTreeMap::from([(BlockId(1), loop_value)]),
                true,
            )
            .expect("entry phi");
        let state = builder.finish().expect("initial entry seed is valid");
        let Some(StateOp::Phi(phi)) = state.definition(phi) else {
            panic!("entry phi must remain queryable");
        };
        assert!(phi.includes_initial());
        assert_eq!(
            phi.incoming_versions().collect::<Vec<_>>(),
            vec![StateVersion::INITIAL, loop_value]
        );
    }

    #[test]
    fn place_and_world_regions_use_the_same_records_but_not_the_same_overlap() {
        let place_policy = PlaceOverlapPolicy;
        let first_element = array_elem("items", Index::literal("first"), LOCAL_NS, false);
        let second_element = array_elem("items", Index::literal("second"), LOCAL_NS, false);
        let whole_array = array_whole("items", LOCAL_NS, false);

        assert!(!place_policy.may_overlap(&first_element, &second_element));
        assert!(place_policy.may_overlap(&whole_array, &second_element));
        assert!(place_policy.may_overlap(&place_policy.wildcard(), &second_element));

        let world_policy = WorldRegionOverlapPolicy::conservative();
        let binding = WorldRegion::exact(
            WorldRegionKind::CommandBindings,
            WorldInterpreterScope::named("i1"),
            WorldNamespaceScope::named("::ns"),
            "::one",
        );
        let trace = WorldRegion::exact(
            WorldRegionKind::ExecutionTraces,
            WorldInterpreterScope::named("i1"),
            WorldNamespaceScope::named("::ns"),
            "::one",
        );
        let same_binding = WorldRegion::exact(
            WorldRegionKind::CommandBindings,
            WorldInterpreterScope::named("i1"),
            WorldNamespaceScope::named("::ns"),
            "::one",
        );
        let other_interpreter = WorldRegion::exact(
            WorldRegionKind::CommandBindings,
            WorldInterpreterScope::named("i2"),
            WorldNamespaceScope::named("::ns"),
            "::one",
        );

        assert!(world_policy.may_overlap(&binding, &same_binding));
        assert!(!world_policy.may_overlap(&binding, &trace));
        assert!(!world_policy.may_overlap(&binding, &other_interpreter));
        assert!(world_policy.may_overlap(&world_policy.wildcard(), &trace));

        let mut place_builder = StateSsaBuilder::new();
        let place_version = place_builder
            .def(
                StateSite::statement(BlockId(0), 0, 0),
                first_element.clone(),
            )
            .expect("place definition");
        place_builder.use_at(
            StateSite::statement(BlockId(0), 1, 0),
            second_element.clone(),
            StateVersion::INITIAL,
        );
        let place_state = place_builder.finish().expect("valid place records");
        assert!(
            place_state
                .definitions_overlapping(&second_element, &place_policy)
                .is_empty()
        );
        assert!(place_state.definition_may_overlap(place_version, &whole_array, &place_policy));

        let mut world_builder = StateSsaBuilder::new();
        let world_version = world_builder
            .def(StateSite::statement(BlockId(0), 0, 0), binding.clone())
            .expect("world definition");
        world_builder.use_at(
            StateSite::statement(BlockId(0), 1, 0),
            same_binding.clone(),
            world_version,
        );
        let world_state = world_builder.finish().expect("valid world records");
        assert_eq!(
            world_state
                .reaching_version_for(&StateSite::statement(BlockId(0), 1, 0), &same_binding),
            Some(world_version)
        );
        assert!(world_state.definition_may_overlap(world_version, &same_binding, &world_policy));
    }

    #[test]
    fn recursive_namespace_tree_overlap_preserves_a_known_delete_root() {
        let policy = WorldRegionOverlapPolicy::conservative();
        let deleted_tree = WorldRegion::namespace_subtree(
            WorldInterpreterScope::named("i1"),
            WorldNamespaceScope::named("::pkg"),
        );
        let descendant_command = WorldRegion::exact(
            WorldRegionKind::CommandBindings,
            WorldInterpreterScope::named("i1"),
            WorldNamespaceScope::named("::pkg::nested"),
            "command",
        );
        let sibling_command = WorldRegion::exact(
            WorldRegionKind::CommandBindings,
            WorldInterpreterScope::named("i1"),
            WorldNamespaceScope::named("::other"),
            "command",
        );
        let descendant_variable = WorldRegion::exact(
            WorldRegionKind::VariableStore,
            WorldInterpreterScope::named("i1"),
            WorldNamespaceScope::named("::pkg::nested"),
            "value",
        );
        let descendant_object = WorldRegion::exact(
            WorldRegionKind::ObjectDispatch,
            WorldInterpreterScope::named("i1"),
            WorldNamespaceScope::named("::pkg::nested"),
            "class",
        );
        let external = WorldRegion::exact(
            WorldRegionKind::ExternalResource(tcl_registry::side_effects::SideEffectTarget::FileIo),
            WorldInterpreterScope::Any,
            WorldNamespaceScope::Any,
            "file",
        );

        assert!(policy.may_overlap(&deleted_tree, &descendant_command));
        assert!(policy.may_overlap(&deleted_tree, &descendant_variable));
        assert!(policy.may_overlap(&deleted_tree, &descendant_object));
        assert!(!policy.may_overlap(&deleted_tree, &sibling_command));
        assert!(!policy.may_overlap(&deleted_tree, &external));
    }

    #[test]
    fn namespace_creation_lineage_overlaps_ancestors_not_siblings() {
        let policy = WorldRegionOverlapPolicy::conservative();
        let lineage = WorldRegion::namespace_lineage(
            WorldInterpreterScope::named("i1"),
            WorldNamespaceScope::named("::pkg::nested"),
        );
        let target = WorldRegion::domain_wildcard(
            WorldRegionKind::NamespaceLookup,
            WorldInterpreterScope::named("i1"),
            WorldNamespaceScope::named("::pkg::nested"),
        );
        let ancestor = WorldRegion::domain_wildcard(
            WorldRegionKind::NamespaceLookup,
            WorldInterpreterScope::named("i1"),
            WorldNamespaceScope::named("::pkg"),
        );
        let sibling = WorldRegion::domain_wildcard(
            WorldRegionKind::NamespaceLookup,
            WorldInterpreterScope::named("i1"),
            WorldNamespaceScope::named("::other"),
        );

        assert!(policy.may_overlap(&lineage, &target));
        assert!(policy.may_overlap(&lineage, &ancestor));
        assert!(!policy.may_overlap(&lineage, &sibling));
    }

    #[test]
    fn world_scopes_keep_current_conservative_until_context_identifies_it() {
        let current = WorldRegion::exact(
            WorldRegionKind::NamespaceUnknown,
            WorldInterpreterScope::Current,
            WorldNamespaceScope::Current,
            "handler",
        );
        let named = WorldRegion::exact(
            WorldRegionKind::NamespaceUnknown,
            WorldInterpreterScope::named("i1"),
            WorldNamespaceScope::named("::ns"),
            "handler",
        );

        assert!(WorldRegionOverlapPolicy::conservative().may_overlap(&current, &named));
        assert!(
            WorldRegionOverlapPolicy::with_context(WorldScopeContext::named("i1", "::ns"))
                .may_overlap(&current, &named)
        );
        assert!(
            !WorldRegionOverlapPolicy::with_context(WorldScopeContext::named("i2", "::other"))
                .may_overlap(&current, &named)
        );
    }

    #[test]
    fn invalid_reaching_versions_and_duplicate_sites_are_rejected() {
        let location = scalar("x", LOCAL_NS, false);
        let site = StateSite::statement(BlockId(0), 0, 0);
        let duplicate = StateSsa::new(vec![
            super::StateOp::Use(super::StateUse {
                location: location.clone(),
                reaching_version: StateVersion::INITIAL,
                site: site.clone(),
            }),
            super::StateOp::Use(super::StateUse {
                location: location.clone(),
                reaching_version: StateVersion::INITIAL,
                site: site.clone(),
            }),
        ]);
        assert!(matches!(
            duplicate,
            Err(super::StateSsaError::DuplicateSite(found)) if found == site
        ));

        let missing = StateSsa::new(vec![super::StateOp::Use(super::StateUse {
            location,
            reaching_version: StateVersion::new(9),
            site,
        })]);
        assert!(matches!(
            missing,
            Err(super::StateSsaError::UnknownReachingVersion { version, .. })
                if version == StateVersion::new(9)
        ));
    }

    #[test]
    fn edge_sites_keep_mutually_exclusive_definitions_out_of_block_order() {
        let location = scalar("binding", LOCAL_NS, false);
        let ok = StateSite::edge(
            BlockId(4),
            BlockId(5),
            CfgStatePosition::Statement {
                index: 2,
                ordinal: 0,
            },
        );
        let abrupt = StateSite::edge(
            BlockId(4),
            BlockId(6),
            CfgStatePosition::Statement {
                index: 2,
                ordinal: 0,
            },
        );
        let state = StateSsa::new(vec![
            StateOp::Def(StateDef {
                location: location.clone(),
                version: StateVersion::new(1),
                site: ok,
            }),
            StateOp::Clobber(StateClobber {
                location: location.clone(),
                version: StateVersion::new(2),
                site: abrupt,
            }),
        ])
        .expect("distinct completion edges are valid state definitions");
        assert!(matches!(
            state.operation_on_edge(
                BlockId(4),
                BlockId(5),
                CfgStatePosition::Statement {
                    index: 2,
                    ordinal: 0
                },
            ),
            Some(StateOp::Def(_))
        ));
        assert!(matches!(
            state.operation_on_edge(
                BlockId(4),
                BlockId(6),
                CfgStatePosition::Statement {
                    index: 2,
                    ordinal: 0
                },
            ),
            Some(StateOp::Clobber(_))
        ));
    }
}
