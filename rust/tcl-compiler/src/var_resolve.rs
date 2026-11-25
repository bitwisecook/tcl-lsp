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

use tcl_registry::CommandRegistry;
#[cfg(test)]
use tcl_registry::FrameLevel;

use crate::place::{self, CellGeneration, CellIdentity, CellOwner, Index, Place, PlaceKind};
mod cell_key;
mod fresh_scalar_proposal;
mod namespace_identity;
mod original_bytes;
mod original_name_contents;
mod original_name_write;
pub use cell_key::SharedSet;
pub use cell_key::{
    VariableCellKey, VariableCellKeyQuery, VariableCellSet, VariableCellTable,
    VariableNamespaceMembership, VariableNamespaceQuery, VariableNamespaceSet, namespace_contains,
};
pub(crate) use fresh_scalar_proposal::FreshScalarVariableSlot;
pub use original_bytes::resolve_evaluated_variable_input;
pub(crate) use original_bytes::{
    original_compiled_local_destination, original_compiled_variable_primary,
    resolve_original_alias_destination_bytes, resolve_original_compiled_primary,
    resolve_original_compiled_variable, resolve_original_element_inputs,
    resolve_original_name_input, resolve_original_namespace_variable_bytes,
};
pub(crate) use original_name_contents::{OriginalNameValueRead, OriginalNormalValueRead};
pub(crate) use original_name_write::OriginalNormalValueWrite;

mod canonical_keys;
mod contents_reads;
mod contents_stores;
mod frame_restore;
mod frame_selection;
mod incoming_bindings;
mod invalidation;
mod join;
mod observer_projection;
mod place_resolution;
mod proof_relocation;
mod scalar_binding;

pub(crate) use canonical_keys::physical_array_key;
pub use canonical_keys::{
    canonical_binding_value_key, canonical_binding_value_name, canonical_literal_variable_key,
    canonical_literal_variable_name, canonical_namespace_variable,
    canonical_namespace_variable_key, canonical_namespace_variable_place, canonical_place_key,
    canonical_place_name, canonical_variable_key, canonical_variable_name, cell_key,
    namespace_variable_simple_name, root_namespace_variable_simple_name,
};
pub use frame_restore::restore_execution_frame;
pub(crate) use observer_projection::VariableObserverProjection;
pub use observer_projection::{project_access, trace_key};
use place_resolution::authored_namespace_place;
pub use place_resolution::{
    NamespaceAccessOrigin, literal_namespace_access_origin, possible_access_domain, resolve_access,
    resolve_alias_destination_slot, resolve_array_element_root, resolve_dict_path,
    resolve_literal_access, resolve_literal_place, resolve_place, resolve_substitution_access,
    resolve_target_access,
};
pub use proof_relocation::VariableProofRelocation;
#[cfg(test)]
use scalar_binding::inner_read_places;
use scalar_binding::{bind_cell_identity, bind_scalar_receiver, resolve_name_alias};

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
    /// Whether every store this origin names is one `other` names: a value
    /// joined over `other` covers a read whose reaching stores are these
    /// (the source walk may prove a loop entered, or an arm not taken, that
    /// the graph's join keeps).
    #[must_use]
    pub fn is_within(&self, other: &Self) -> bool {
        let (incoming, writes): (bool, &[u32]) = match self {
            Self::Unknown => return false,
            Self::Incoming => (true, &[]),
            Self::WrittenAt(source) => (false, std::slice::from_ref(source)),
            Self::Alternatives { incoming, writes } => (*incoming, writes),
        };
        match other {
            Self::Unknown => false,
            Self::Incoming => writes.is_empty() && incoming,
            Self::WrittenAt(source) => !incoming && writes.iter().all(|write| write == source),
            Self::Alternatives {
                incoming: other_incoming,
                writes: other_writes,
            } => {
                (!incoming || *other_incoming)
                    && writes.iter().all(|write| other_writes.contains(write))
            }
        }
    }

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
    /// A native constant scalar of the selected cell generation.
    ConstantScalar(CellGeneration),
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
    pub prefix: VariableTracePrefix,
}

/// A copied original callback prefix or an explicitly authored compatibility
/// value. Original storage keeps complete counted bytes and producer receipts.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VariableTracePrefix {
    /// Logical authored analysis, without original native byte authority.
    Authored(String),
    /// Independently selected variable-trace copied storage.
    Original(std::sync::Arc<OriginalVariableTracePrefix>),
}

/// Sealed copied variable-prefix data. Registration liveness is owned by the
/// receiver's trace inventory, independently of this historical producer.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OriginalVariableTracePrefix {
    input: crate::signature_scan::scope::SignatureSourceNameInput,
}

impl OriginalVariableTracePrefix {
    pub(crate) fn input(&self) -> &crate::signature_scan::scope::SignatureSourceNameInput {
        &self.input
    }
}

impl From<String> for VariableTracePrefix {
    fn from(value: String) -> Self {
        Self::Authored(value)
    }
}

impl From<&str> for VariableTracePrefix {
    fn from(value: &str) -> Self {
        Self::Authored(value.to_owned())
    }
}

impl VariableTracePrefix {
    pub(crate) fn copied_original(
        input: &crate::signature_scan::scope::SignatureSourceNameInput,
        context: &ResolveContext,
    ) -> Option<Self> {
        Some(Self::Original(std::sync::Arc::new(OriginalVariableTracePrefix {
            input: crate::signature_scan::scope::SignatureSourceNameInput::OriginalValue(
                crate::signature_scan::scope::SignatureSourceNameValue::copied_variable_trace_prefix(
                    input, context,
                )?,
            ),
        })))
    }

    pub(crate) fn same_data(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Authored(left), Self::Authored(right)) => left == right,
            (Self::Original(left), Self::Original(right)) => {
                left.input.bytes() == right.input.bytes()
                    && left.input.policy() == right.input.policy()
            }
            _ => false,
        }
    }

    pub(crate) fn joined(&self, other: &Self) -> Option<Self> {
        use crate::signature_scan::scope::SignatureSourceNameInput;
        match (self, other) {
            (Self::Authored(left), Self::Authored(right)) if left == right => Some(self.clone()),
            (Self::Original(left), Self::Original(right)) => {
                let (
                    SignatureSourceNameInput::OriginalValue(left),
                    SignatureSourceNameInput::OriginalValue(right),
                ) = (&left.input, &right.input)
                else {
                    return None;
                };
                Some(Self::Original(std::sync::Arc::new(
                    OriginalVariableTracePrefix {
                        input: SignatureSourceNameInput::OriginalValue(
                            left.joined_variable_trace_prefix(right)?,
                        ),
                    },
                )))
            }
            _ => None,
        }
    }

    pub(crate) fn removal_matches(&self, other: &Self, context: &ResolveContext) -> bool {
        match (self, other) {
            (Self::Authored(left), Self::Authored(right)) => left == right,
            (Self::Original(left), Self::Original(right)) => {
                left.input.policy() == right.input.policy()
                    && left.input.is_current(context)
                    && right.input.is_current(context)
                    && context
                        .invocation_dialect
                        .and_then(tcl_registry::InvocationDialect::native_variable_trace_protocol)
                        .is_some_and(|protocol| {
                            protocol
                                .variable_prefix_matches(left.input.bytes(), right.input.bytes())
                        })
            }
            _ => false,
        }
    }
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

/// Original lexical configuration for protective source reads, independent of
/// entered-frame, command selection, or cell identity authority.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) enum SourceReadConfiguration {
    /// Explicit compatibility contexts without an original source issuer.
    #[default]
    Standalone,
    /// Configuration retained at the actual original source entry.
    Selected(tcl_lexer::LexerConfig),
    /// Differing source configurations cannot be reconstructed from a profile.
    Unavailable,
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
    pub namespace_identities: cell_key::SharedSet<crate::command_binding::SourceNamespaceKey>,
    /// Namespace incarnations reachable by a new written-address lookup.
    pub namespace_addressable_identities:
        cell_key::SharedSet<crate::command_binding::SourceNamespaceKey>,
    /// Whether the retained namespace inventory closes missing table candidates.
    pub namespace_inventory: NamespaceInventoryClosure,
    /// Selected native variable-name input recipe.
    pub namespace_name_protocol: Option<tcl_syntax::naming::NativeNameProtocol>,
    /// Name input issuer retained independently of namespace tokens and string producers.
    pub execution_name_policy: Option<tcl_syntax::naming::ExecutionNamePolicy>,
    /// Logical expression parser the source entry explicitly installed.
    pub expression_parser: Option<tcl_registry::invocation_words::LogicalExpressionParseProvider>,
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
    /// Original source lexical recipe, not a runtime or naming-policy grant.
    pub(crate) source_read_configuration: SourceReadConfiguration,
    /// Foreign frame selected for this region, if it cannot be the current frame.
    pub selected_frame: Option<tcl_registry::FrameLevel>,
    /// Proven current activation allocation site, when supplied by source execution.
    pub activation: Option<String>,
    /// Names under a `global` declaration.
    pub globals: cell_key::SharedSet<String>,
    /// Names under a `variable` declaration.
    pub ns_vars: cell_key::SharedSet<String>,
    /// Local alias → caller target (`""` = dynamic/unresolvable).
    pub upvar_aliases: HashMap<String, String>,
    /// Names with an active `trace`.
    pub traced: VariableCellSet,
    /// Known trace registrations, retaining duplicate registrations for removal.
    pub trace_registrations:
        VariableCellTable<Vec<(Vec<tcl_registry::TraceOperation>, VariableTracePrefix)>>,
    /// Selected typed registration receivers, retained independently of key encoding.
    pub trace_registration_receivers: VariableCellTable<std::sync::Arc<Place>>,
    /// Enumerated callback prefixes on unenumerated addresses; never definite registrations.
    pub possible_trace_registrations: Vec<PossibleVariableTraceRegistration>,
    /// Traces whose incoming registration or operation list is unenumerated.
    pub untracked_traces: VariableCellSet,
    /// `TclOO` instance vars in scope.
    pub instance_vars: cell_key::SharedSet<String>,
    /// Advisory declared-state scope for `instance_vars`; never an object token.
    pub instance_owner: String,
    /// Typed selection of the current variable table.
    pub frame_kind: VariableFrameKind,
    /// Known interpreter for a selected execution region.
    pub interpreter: Option<String>,
    /// Independently selected host storage context. Source-name advice and
    /// registry profiles do not supply a worker, cell or publication receipt.
    pub hosted_execution_context: Option<tcl_registry::f5::BigIpExecutionContext>,
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
    /// Authentic entered formal declaration prefix. It does not close the
    /// rest of a procedure's compiled-local inventory or authenticate a CPP.
    #[doc(hidden)]
    pub original_formal_topology:
        Option<std::sync::Arc<crate::command_binding::formal_topology::OriginalFormalTopology>>,
    /// Compiler formal slots from a live original Native procedure request.
    /// This is not authored syntax, a physical layout or an entered frame.
    pub(crate) original_native_compiler_formals:
        Option<std::sync::Arc<crate::command_binding::OriginalNativeCompilerFormalScope>>,
    /// Actual own-provider receiver inventory, queried separately by each
    /// supplied compiler-primary or runtime-root resolver purpose.
    pub(crate) original_receiver_variable_candidates:
        Option<std::sync::Arc<crate::allocated_instance::OriginalReceiverVariableCandidates>>,
    /// Authenticated original produced units at exact current value slots.
    pub(crate) original_name_values:
        VariableCellTable<original_name_contents::StoredOriginalNameValue>,
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
    /// Currency of original produced bytes, independent of cache/representation.
    pub(crate) original_contents_epoch: Option<u64>,
    /// Issuance high-water survives withdrawal; exhaustion cannot revive a receipt.
    pub(crate) original_contents_epoch_high_water: Option<u64>,
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
    activation_observer_inventory: ActivationObserverInventory,
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
    pub known_namespaces: cell_key::SharedSet<String>,
    /// Reaching namespace-cell allocation, used by Tcl8 global fallback.
    pub namespace_cells: NamespaceCellPresence,
    /// Root lifetimes changed by array or namespace destruction.
    pub generations: VariableCellTable<CellGeneration>,
    /// Cell identities are available in this context.
    pub binding_identity: BindingIdentity,
    /// Unknown trace registrations may affect any cell operation.
    pub dynamic_traces: bool,
}

/// Enumeration of observers in the independently entered private activation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
enum ActivationObserverInventory {
    #[default]
    Open,
    Closed,
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

impl ResolveContext {
    /// A variadic `args` value as this runtime renders it: a first element
    /// that starts with `#` is braced from 8.5 and bare in 8.4 (tclsh
    /// 8.4.20 prints `[list # a]` as `# a`, 8.5.19 to 9.1.0 as `{#} a`);
    /// `None` where the runtime names no release and the rendering depends
    /// on one.
    pub(crate) fn render_rest_list<S: AsRef<str>>(&self, elements: &[S]) -> Option<String> {
        let words: Vec<_> = elements
            .iter()
            .map(|word| word.as_ref().as_bytes())
            .collect();
        let bytes = match self
            .invocation_dialect
            .and_then(tcl_registry::InvocationDialect::list_result_serialization)
        {
            Some(layout) => layout.render(&words),
            None => tcl_syntax::list_result::portable_list_result(&words)?,
        };
        String::from_utf8(bytes).ok()
    }

    /// Select a frame's existing policies without claiming observer completeness.
    #[must_use]
    pub(crate) fn for_frame(
        frame_kind: VariableFrameKind,
        binding_identity: BindingIdentity,
        invocation_dialect: Option<tcl_registry::InvocationDialect>,
    ) -> Self {
        Self {
            invocation_dialect,
            frame_kind,
            binding_identity,
            ..Self::default()
        }
    }

    pub(crate) const fn activation_observers_closed(&self) -> bool {
        matches!(
            self.activation_observer_inventory,
            ActivationObserverInventory::Closed
        )
    }
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

fn hash_set<T: Hash + Ord, S, H: Hasher>(items: &HashSet<T, S>, state: &mut H) {
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
        self.expression_parser.hash(state);
        hash_map(&self.namespace_objects, state);
        self.authored_tmm_static.hash(state);
        self.namespace_known.hash(state);
        self.invocation_dialect.hash(state);
        self.source_read_configuration.hash(state);
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
        self.hosted_execution_context.hash(state);
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
        self.original_name_values.hash(state);
        self.original_formal_topology.hash(state);
        self.original_native_compiler_formals.hash(state);
        self.original_receiver_variable_candidates.hash(state);
        hash_map(&self.dictionary_scopes, state);
        self.value_representations.hash(state);
        self.original_contents_epoch.hash(state);
        self.original_contents_epoch_high_water.hash(state);
        self.representation_epoch.hash(state);
        self.representation_epoch_high_water.hash(state);
        self.contents_origins.hash(state);
        self.contents_source_proofs.hash(state);
        self.contents_presence.hash(state);
        self.contents_presence_slots.hash(state);
        self.contents_kinds.hash(state);
        self.contents_world.hash(state);
        self.activation_contents_world.hash(state);
        self.activation_observer_inventory.hash(state);
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
            namespace_identities: cell_key::SharedSet::default(),
            namespace_addressable_identities: cell_key::SharedSet::default(),
            namespace_inventory: NamespaceInventoryClosure::Open,
            namespace_name_protocol: None,
            execution_name_policy: None,
            expression_parser: None,
            namespace_objects: HashMap::new(),
            authored_tmm_static: None,
            namespace_known: true,
            invocation_dialect: None,
            source_read_configuration: SourceReadConfiguration::Standalone,
            selected_frame: None,
            activation: None,
            globals: cell_key::SharedSet::default(),
            ns_vars: cell_key::SharedSet::default(),
            upvar_aliases: HashMap::new(),
            traced: VariableCellSet::default(),
            trace_registrations: VariableCellTable::default(),
            trace_registration_receivers: VariableCellTable::default(),
            possible_trace_registrations: Vec::new(),
            untracked_traces: VariableCellSet::default(),
            instance_vars: cell_key::SharedSet::default(),
            instance_owner: String::new(),
            frame_kind: VariableFrameKind::Local,
            interpreter: None,
            hosted_execution_context: None,
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
            original_name_values: VariableCellTable::default(),
            original_formal_topology: None,
            original_native_compiler_formals: None,
            original_receiver_variable_candidates: None,
            dictionary_scopes: HashMap::new(),
            value_representations: VariableCellTable::default(),
            original_contents_epoch: Some(0),
            original_contents_epoch_high_water: Some(0),
            representation_epoch: Some(0),
            representation_epoch_high_water: Some(0),
            contents_origins: VariableCellTable::default(),
            contents_source_proofs: crate::contents_source::ContentsSourceProofs::default(),
            contents_presence: VariableCellTable::default(),
            contents_presence_slots: VariableCellTable::default(),
            contents_kinds: VariableCellTable::default(),
            contents_world: ContentsWorld::Tracked,
            activation_contents_world: None,
            activation_observer_inventory: ActivationObserverInventory::Open,
            closed_observer_allocations: HashSet::new(),
            contents_unknown_namespaces: VariableNamespaceSet::default(),
            contents_unknown_arrays: VariableCellSet::default(),
            closed_array_roots: VariableCellSet::default(),
            unknown_bindings: VariableCellSet::default(),
            dynamic_bindings: false,
            known_namespaces: cell_key::SharedSet::from(["::".to_owned()]),
            namespace_cells: NamespaceCellPresence::default(),
            generations: VariableCellTable::default(),
            binding_identity: BindingIdentity::Legacy,
            dynamic_traces: false,
        }
    }
}

impl ResolveContext {
    /// Reborrow exact original syntax; only explicit standalone utilities may
    /// select a compatibility grammar from their independently supplied profile.
    pub(crate) fn read_lexer_config(
        &self,
        registry: &CommandRegistry,
    ) -> Option<tcl_lexer::LexerConfig> {
        match self.source_read_configuration {
            SourceReadConfiguration::Selected(config) => Some(config),
            SourceReadConfiguration::Unavailable => None,
            SourceReadConfiguration::Standalone => Some(self.invocation_dialect.map_or_else(
                || tcl_lexer::LexerConfig::for_profile(registry.profile()),
                |dialect| tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
            )),
        }
    }

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
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod root_contents_kind_tests;

#[cfg(test)]
mod worker_execution_ingress_tests;

#[cfg(test)]
mod possible_access_domain_tests;

#[cfg(test)]
mod byte_cell_identity_tests;

#[cfg(test)]
mod namespace_simple_projection_tests;

#[cfg(test)]
mod original_read_configuration_tests;

#[cfg(test)]
mod selected_frame_source_read_configuration_tests;
