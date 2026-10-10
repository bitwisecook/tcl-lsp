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

// These algorithms always use the default RandomState hasher; making
// them generic over BuildHasher adds complexity for no real benefit.
//! Static Single-Assignment (SSA) construction over CFG blocks.
//!
//! SSA is a variable-naming discipline where every variable is assigned
//! exactly once. When control flow merges (e.g. after an `if`), a
//! synthetic *phi node* is inserted to select the correct version of a
//! variable depending on which predecessor block was executed.
//!
//! This module provides:
//!
//! 1. SSA data structures: [`Phi`], [`SsaStatement`], [`SsaBlock`],
//!    [`SsaFunction`].
//! 2. **Dominator** computation: [`compute_dominators`] and
//!    `compute_idom` for immediate dominators.
//! 3. **Dominance frontier**: [`compute_dominance_frontier`].
//! 4. **Phi placement**: [`compute_phi_vars`] using the iterated
//!    dominance frontier algorithm.
//! 5. **Variable definition extraction**: [`defs_of`] extracts variable
//!    names defined by an IR statement.
//!
//! The full SSA rename pass ([`build_ssa`]) and variable-use scanner
//! ([`uses_of`]) are now implemented, completing the SSA construction
//! pipeline. The rename pass walks the dominator tree, assigns SSA
//! versions to variable definitions and uses, and fills in phi-node
//! incoming edges.

use std::collections::{BTreeSet, HashMap, HashSet};

use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};
use tcl_registry::CommandRegistry;

use crate::cfg::{self, BlockId};
use crate::ir::{CommandTokens, SourceSite, Statement, WordExpr, WordPart};
use crate::naming::normalise_var_name;
use crate::var_refs::{VarReferenceScanner, VarScanOptions};
use crate::var_resolve::{
    ContentsOrigin, ResolveContext, VariableCellKey, VariableCellKeyQuery, canonical_variable_key,
};
use crate::variable_bindings::PointResolveContexts;
#[cfg(test)]
use crate::variable_bindings::build_point_resolve_contexts;

/// Metadata selection for a complete SSA scan. Supplied absence remains
/// unavailable; only the explicit standalone adapters select catalogue context.
#[derive(Clone, Copy)]
struct SsaInvocationContext<'a> {
    registry: &'a CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
    standalone: bool,
}

impl<'a> SsaInvocationContext<'a> {
    fn standalone(registry: &'a CommandRegistry) -> Self {
        Self {
            registry,
            metadata: registry.profile().map(|profile| {
                tcl_registry::model::semantic::SemanticContext::for_profile(profile).into()
            }),
            standalone: true,
        }
    }

    fn supplied(
        registry: &'a CommandRegistry,
        metadata: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
    ) -> Self {
        Self {
            registry,
            metadata,
            standalone: metadata.is_some_and(|context| context.source_analysis_input().is_none()),
        }
    }

    fn token_metadata(
        self,
        tokens: &CommandTokens,
    ) -> Option<crate::registry_invocation::InvocationMetadataContext<'a>> {
        if self.standalone {
            return self.metadata;
        }
        let binding = tokens.source_binding.as_ref()?;
        supplied_ssa_metadata(
            self.registry,
            binding.original_lexer_config_for_tokens(tokens)?,
            self.metadata,
        )
    }

    fn normal(
        self,
        tokens: &CommandTokens,
    ) -> Option<crate::registry_invocation::NormalTransferInvocation> {
        let metadata = self.token_metadata(tokens)?;
        crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
            self.registry,
            Some(metadata),
            tokens,
        )
    }

    fn resolve(
        self,
        tokens: &CommandTokens,
    ) -> Option<crate::registry_invocation::RegistryInvocationResolution> {
        let metadata = self.token_metadata(tokens)?;
        crate::registry_invocation::resolve_command_tokens_with_metadata_context(
            self.registry,
            Some(metadata),
            tokens,
        )
        .ok()
    }
}

fn bound_names(
    names: Vec<String>,
    context: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<VariableCellKey> {
    let mut bound = Vec::new();
    for name in names {
        if let Some(name) = canonical_variable_key(&name, context, registry)
            && !bound.contains(&name)
        {
            bound.push(name);
        }
    }
    bound
}

fn definition_source_name(
    stmt: &Statement,
    name: &str,
    after: &ResolveContext,
    registry: &CommandRegistry,
) -> Option<VariableCellKey> {
    match stmt {
        Statement::AssignConst { name_braced, .. }
        | Statement::AssignValue { name_braced, .. }
        | Statement::AssignExpr { name_braced, .. }
        | Statement::Incr { name_braced, .. } => crate::var_resolve::canonical_binding_value_key(
            &crate::var_resolve::resolve_target_access(
                name,
                *name_braced,
                after,
                registry,
                tcl_registry::TraceOperation::Write,
            ),
        ),
        _ => crate::var_resolve::canonical_literal_variable_key(name, after, registry),
    }
}

fn bound_defs(
    statement: &Statement,
    block: BlockId,
    index: usize,
    points: &PointResolveContexts,
    selection: SsaInvocationContext<'_>,
) -> Vec<VariableCellKey> {
    let registry = selection.registry;
    let context = points.before_statement(block, index);
    let after = points.after_statement(block, index);
    if matches!(
        statement,
        Statement::Call { .. }
            | Statement::AssignConst { .. }
            | Statement::AssignValue { .. }
            | Statement::AssignExpr { .. }
            | Statement::Incr { .. }
    ) {
        let mut definitions: Vec<_> =
            crate::place_bridge::def_places_at(statement, block, index, points, registry)
                .iter()
                .filter_map(crate::var_resolve::canonical_binding_value_key)
                .collect();
        definitions.extend(
            crate::place_bridge::ssa_destruction_keys(statement, context, after, registry)
                .into_iter()
                .map(|(key, _)| key),
        );
        let mut seen = FxHashSet::default();
        definitions.retain(|key| seen.insert(key.clone()));
        definitions
    } else {
        bound_names(
            defs_of_in_context(statement, Some(selection)),
            context,
            registry,
        )
    }
}

/// SSA version number — each contents transition gets a unique version.
pub type Version = u32;

/// Interned identifier for an SSA variable name.
///
/// Variable names (`"x"`, `"::ns::count"`, `"arr"`, …) are interned per
/// [`SsaFunction`] into a dense `u32` index so the hot per-statement SSA and
/// dataflow maps (`defs` / `uses`, `entry_versions` / `exit_versions`, the
/// taint / SCCP / type lattices keyed by [`ValueKey`]) key on a cheap copyable
/// id instead of hashing and cloning the name string. The `u32` reflects
/// first-seen order during SSA construction, so [`Symbol`]'s `Ord` is that
/// first-seen order — a deterministic ordering; no analysis relies on
/// variable names being in lexicographic order.
///
/// Resolve a symbol back to its display name with [`SsaFunction::var_name`],
/// and a name to its symbol (when interned) with [`SsaFunction::var_symbol`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Symbol(pub u32);

/// Closed contents-presence alternatives for a potential missing-read diagnostic.
/// This evidence supplies neither an SSA value nor a guaranteed read failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SsaReadPresenceAlternatives {
    contains_undefined: bool,
}

impl SsaReadPresenceAlternatives {
    /// At least one retained physical continuation has undefined contents.
    /// Every other continuation is also closed and has known presence.
    #[must_use]
    pub const fn may_be_undefined(self) -> bool {
        self.contains_undefined
    }
}

/// Key identifying a specific SSA value: `(variable symbol, version)`.
pub type ValueKey = (Symbol, Version);

/// Source name of one selected SSA value definition. This pairs an actual
/// definition cell with its retained written operand; it supplies no value,
/// current-read, rename or native representation capability.
#[derive(Debug, Clone)]
pub struct OriginalSsaDefinitionName {
    span: tcl_lexer::Span,
    input: crate::signature_scan::scope::SignatureSourceNameInput,
    cell: VariableCellKey,
}

impl OriginalSsaDefinitionName {
    /// Complete written receiver extent in this function's source coordinates.
    #[must_use]
    pub const fn span(&self) -> tcl_lexer::Span {
        self.span
    }

    /// Independently retained source producer at the actual receiver ordinal.
    #[must_use]
    pub const fn original_name_input(
        &self,
    ) -> &crate::signature_scan::scope::SignatureSourceNameInput {
        &self.input
    }

    /// Actual SSA cell selected at this definition, without current contents.
    #[must_use]
    pub const fn cell(&self) -> &VariableCellKey {
        &self.cell
    }
}

/// Point-owned absence of a proposed ordinary scalar in an actually entered
/// local activation. The original operation/frame and name policy are retained
/// independently; this grants no new store, value, normal completion, source
/// movement or edit permission.
#[derive(Debug, Clone)]
pub struct OriginalFreshScalarVariableProposal {
    slot: crate::var_resolve::FreshScalarVariableSlot,
    site: crate::command_binding::CommandAllocationSite,
    config: tcl_lexer::LexerConfig,
}

impl OriginalFreshScalarVariableProposal {
    /// Proposed native name units with no display re-encoding.
    #[must_use]
    pub fn name(&self) -> &[u8] {
        self.slot.name()
    }
    /// Exact selected activation slot at this original operation point.
    #[must_use]
    pub fn cell(&self) -> &VariableCellKey {
        self.slot.cell()
    }
    /// Genuine original invocation at which absence was checked.
    #[must_use]
    pub fn site(&self) -> &crate::command_binding::CommandAllocationSite {
        &self.site
    }
    /// Independently selected native naming purpose.
    #[must_use]
    pub fn policy(&self) -> tcl_syntax::naming::NamePolicyProtocol {
        self.slot.policy()
    }
    /// Full retained source parser configuration.
    #[must_use]
    pub const fn lexer_config(&self) -> tcl_lexer::LexerConfig {
        self.config
    }
}

/// Point-owned insertion naming plan retaining both independent local-slot
/// availability and the actual registered scalar setter selection. It grants
/// no store completion, native preparation, value or source motion.
#[derive(Debug, Clone)]
pub struct OriginalFreshScalarAssignmentProposal {
    variable: OriginalFreshScalarVariableProposal,
    command: crate::command_binding::OriginalScalarAssignmentCommand,
}

impl OriginalFreshScalarAssignmentProposal {
    /// Proposed unoccupied scalar slot in the actual entered local frame.
    #[must_use]
    pub const fn variable(&self) -> &OriginalFreshScalarVariableProposal {
        &self.variable
    }

    /// Independently selected current registered setter and authored word.
    #[must_use]
    pub const fn command(&self) -> &crate::command_binding::OriginalScalarAssignmentCommand {
        &self.command
    }
}

// SSA data structures

/// A phi node merging variable versions at a control-flow join.
///
/// `incoming` maps each predecessor block to the variable version that
/// flows in from that edge.
#[derive(Debug, Clone, PartialEq)]
pub struct Phi {
    /// Variable, as an interned [`Symbol`]. Resolve the display name with
    /// [`SsaFunction::var_name`].
    pub name: Symbol,
    /// SSA version assigned by this phi.
    pub version: Version,
    /// Predecessor block → incoming version.
    pub incoming: HashMap<BlockId, Version>,
}

/// An IR statement annotated with SSA version numbers.
///
/// `uses` maps each variable read by the statement to the
/// SSA version in scope. `defs` maps each contents transition, including
/// destruction, to its newly assigned version. Both key on the interned
/// variable [`Symbol`]; resolve a symbol's display name with
/// [`SsaFunction::var_name`].
#[derive(Debug, Clone, PartialEq)]
pub struct SsaStatement {
    /// The underlying IR statement.
    pub statement: Statement,
    /// Named variables read: symbol → SSA version.
    /// Unprojected reads remain in `statement.has_opaque_native_accesses()`.
    pub uses: HashMap<Symbol, Version>,
    /// Named contents transitions: symbol → SSA version.
    /// Unprojected writes remain in `statement.has_opaque_native_accesses()`.
    pub defs: HashMap<Symbol, Version>,
    /// The subset of [`Self::defs`] that are *synthetic* array-element
    /// writes, not writes the statement performs itself: the base refresh
    /// alongside an element write (`set arr(k) v` also defs `arr`), and the
    /// element fan of a dynamic-key / whole-array write (`set arr($i) v`
    /// defs every known `arr(*)`). Type inference **joins** across a
    /// may-def (old type ⊔ written value); write-sensitive passes (shimmer
    /// oscillation, dead-store) must not count one as a real write.
    pub may_defs: HashSet<Symbol>,
    /// Contents versions invalidated by destruction. These supply no value,
    /// object representation or store provenance. Conditional declaration
    /// transitions also belong to `may_defs` and retain their predecessor use.
    pub destruction_defs: HashSet<Symbol>,
    /// The subset of [`Self::uses`] that are [`UseClass::Quoted`] — carried
    /// only by a brace-quoted word this statement does not substitute. The
    /// use is real for liveness (the text may be evaluated later) but is not
    /// a read *here*, so read-before-set must ignore it. See [`UseClass`].
    pub quoted_uses: HashSet<Symbol>,
    /// The subset of [`Self::uses`] that are [`UseClass::Name`] — reached by
    /// naming the cell (`incr a`, `append a x`, `info exists a`, `unset a`)
    /// rather than by substituting a `$a` word. As real a read as any other,
    /// but with no operand for a value-forwarding pass to rewrite; carried
    /// through to [`crate::def_use::UseKind::VariableName`].
    pub name_only_uses: HashSet<Symbol>,
}

fn record_destruction_predecessor(
    symbol: Symbol,
    version: Version,
    uses: &mut HashMap<Symbol, Version>,
    name_only: &mut HashSet<Symbol>,
) {
    if let std::collections::hash_map::Entry::Vacant(entry) = uses.entry(symbol) {
        entry.insert(version);
        name_only.insert(symbol);
    }
}

/// How a statement consumes a variable reference.
///
/// Tcl substitutes `$name` in a bare or `"`-quoted word, and never in a
/// brace-quoted one: `puts {$y}` prints the two characters `$y` and reads
/// nothing (tclsh 9.0.4 / 8.6.16 agree, and `puts {$y}` succeeds with `y`
/// undefined). A braced word's contents may still be *evaluated* — by
/// `expr`, by `if`, by an `after` callback, by an unknown definer — but when
/// and in which frame is the callee's business.
///
/// The two consumers of the use set need opposite conservatism about that
/// word, which is why the use is classified rather than present-or-absent:
///
/// - liveness / dead-store (W211, W220, store elimination) must assume the
///   word **may** be evaluated, so the use must exist;
/// - read-before-set (W210) must assume it **may not** be, or may be
///   evaluated in a frame that binds the name, so it must not claim the read.
///
/// Filtering at either end breaks the other: dropping the use resurrects
/// `W211 set but never used` on `set a(k) 1; puts {$a(k)}`, and recording the
/// name as a self-initialising def deletes the feeding store outright.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UseClass {
    /// Substituted at this call site, or evaluated by the callee in this same
    /// frame (an [`ArgRole::Expr`](tcl_registry::ArgRole::Expr) /
    /// [`ArgRole::Body`](tcl_registry::ArgRole::Body) word). A genuine read,
    /// here and now.
    Substituted,
    /// Carried only inside a brace-quoted word this call site does not
    /// substitute and that nothing evaluates in this frame — see
    /// [`braced_word_class`] for the three kinds a braced word falls into and
    /// which of them land here. Kept as a use so liveness stays conservative;
    /// ignored by read-before-set.
    Quoted,
    /// Named rather than substituted: the statement reaches the cell through a
    /// variable-*name* argument — `incr a`, `append a x`, `info exists a`,
    /// `unset a` — with no `$a` word anywhere in it.
    ///
    /// As definite a read as [`Self::Substituted`], and read-before-set must
    /// treat it as one: `incr a` really does read `a`. What separates it is
    /// that there is **no operand to rewrite**. A value-forwarding pass that
    /// splices a literal over such a use destroys the statement — the reaching
    /// literal of `a` is `1`, and `incr 1` is neither an increment nor a
    /// command. [`UseKind::VariableName`] is how the def-use
    /// chains carry that distinction to those passes.
    Name,
}

/// A CFG basic block in SSA form.
///
/// `entry_versions` / `exit_versions` record which SSA version
/// of each variable is live at the start and end of the block,
/// keyed by the interned variable [`Symbol`].
#[derive(Debug, Clone, PartialEq)]
pub struct SsaBlock {
    /// Block name.
    pub name: String,
    /// Phi nodes at the start of this block.
    pub phis: Vec<Phi>,
    /// SSA-annotated statements.
    pub statements: Vec<SsaStatement>,
    /// Variable versions at block entry: symbol → version.
    pub entry_versions: HashMap<Symbol, Version>,
    /// Variable versions at block exit: symbol → version.
    pub exit_versions: HashMap<Symbol, Version>,
}

/// Complete SSA representation of one Tcl procedure or top-level script.
///
/// Includes the dominator tree and dominance frontier so that
/// downstream passes (SCCP, liveness) do not need to recompute them.
#[derive(Debug, Clone, PartialEq)]
pub struct SsaFunction {
    /// Procedure name.
    pub name: String,
    /// Entry block id.
    pub entry: BlockId,
    /// SSA blocks keyed by block id.
    pub blocks: HashMap<BlockId, SsaBlock>,
    /// Fresh scalar versions at registry boundaries. These are value effects,
    /// separate from executable writes and statement use/def evidence.
    pub value_clobbers: ValueClobbers,
    /// Immediate dominator: block → parent (None for entry).
    pub idom: HashMap<BlockId, Option<BlockId>>,
    /// Dominance frontier: block → frontier blocks.
    pub dominance_frontier: HashMap<BlockId, Vec<BlockId>>,
    /// Dominator tree: block → children.
    pub dominator_tree: HashMap<BlockId, Vec<BlockId>>,
    /// Block-name interner copied from the source CFG, so an
    /// [`SsaFunction`]-only consumer can still resolve a [`BlockId`] to its
    /// display name. Names indexed by [`BlockId`]`.0`, in creation order.
    block_names: Vec<String>,
    /// Variable-name interner: names indexed by [`Symbol`]`.0`, in first-seen
    /// order during SSA construction. Resolves a [`Symbol`] back to the
    /// display name needed for diagnostics and serialised output.
    var_names: Vec<String>,
    cell_names: Vec<String>,
    cell_keys: Vec<VariableCellKey>,
    /// Reverse interner index: variable name → its [`Symbol`].
    var_to_symbol: FxHashMap<String, Symbol>,
    cell_to_symbol: FxHashMap<VariableCellKey, Symbol>,
    point_symbols: HashMap<(BlockId, usize), HashMap<String, Symbol>>,
    /// Shared reaching frame and cell bindings used by scalar and memory consumers.
    pub point_contexts: Option<PointResolveContexts>,
    read_references: HashMap<(BlockId, usize, SourceSite, String), SsaReadReference>,
    array_root_refresh_symbols: HashSet<Symbol>,
    array_root_refresh_versions: HashSet<(Symbol, Version)>,
}

/// Canonical binding read at one exact lexical substitution.
/// A missing version retains the cell dependency while declining a value proof:
/// nested evaluation may have written contents that the containing CFG does not define.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SsaReadReference {
    /// Symbol for the canonical binding contents, never a source display name.
    pub symbol: Symbol,
    /// Matching scalar SSA contents version, if represented by this CFG.
    pub version: Option<Version>,
}

/// Captured physical representation evidence for conversion-cost advice.
/// Numeric categories come from actual current native production, independently
/// of semantic contents. Container alternatives never establish one current intrep.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SsaReadRepresentationAdvice {
    pub representation: Option<tcl_syntax::value::ValueRepresentation>,
    pub container_alternatives: Option<crate::native_numeric::ClosedContainerRepresentations>,
    pub already_numeric: bool,
    /// Unanimous current subtype, or Numeric for closed differing subtypes.
    /// Missing evidence cannot borrow a semantic type or replayed commitment.
    pub numeric_category: Option<tcl_registry::TclType>,
}

/// Ordinary incoming slot reached in one or more actual activations.
/// This is a logical input projection, not a scalar SSA value or a physical
/// alias between its cells. Callers must validate the formal binding protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SsaIncomingSlotRead {
    /// Exact authored read whose reached activations supplied these cells.
    pub source: SourceSite,
    /// Literal ordinary slot selected in every retained activation.
    pub slot: String,
    /// Distinct physical incoming cells; none is collapsed into another owner.
    pub cells: Vec<crate::place::CellIdentity>,
}

/// Retained contents provenance for a read whose possible writes need not be
/// separate scalar definitions (for example an unknown array-element index).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SsaReadContents {
    /// Exact physical binding read.
    pub reference: SsaReadReference,
    /// Source-reaching contents provenance, including explicit unknown effects.
    pub origin: ContentsOrigin,
    /// Whether entry contents are one possible source of this value.
    pub includes_incoming: bool,
    /// Represented write statements to this exact root and lifetime.
    pub writes: Vec<(BlockId, usize)>,
    /// Some possible contents are not represented by these write points.
    pub unknown_residual: bool,
}

/// Read-only SSA source-name lookup at an optional exact execution point.
/// Unpositioned compatibility queries succeed only for a unique proved binding.
#[derive(Clone, Copy)]
pub struct SsaSourceView<'a> {
    ssa: &'a SsaFunction,
    point: Option<(BlockId, usize)>,
}

#[cfg(debug_assertions)]
fn trace_original_definition_place(
    tokens: &crate::ir::CommandTokens,
    context: &crate::var_resolve::ResolveContext,
    written: usize,
    place: &crate::place::Place,
    key: &crate::var_resolve::VariableCellKey,
) {
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_SSA_DEFINITION").is_some() {
        let selected = crate::var_resolve::canonical_binding_value_key(place);
        eprintln!(
            "original_definition site={} written={} policy={} frame={:?} place={:?} dynamic={} selected={} matches={} generation={:?}",
            tokens
                .words()
                .first()
                .map_or(0, |word| word.source().span.start()),
            written,
            context.execution_name_policy.is_some(),
            context.frame_kind,
            place.kind,
            place.dynamic,
            selected.is_some(),
            selected.as_ref() == Some(key),
            place.cell.as_ref().map(|cell| cell.generation)
        );
        if let (Some(selected), crate::var_resolve::VariableCellKey::Namespace { identity, simple }) =
            (selected.as_ref(), key.root())
            && let crate::var_resolve::VariableCellKey::Namespace {
                identity: selected_identity,
                simple: selected_simple,
            } = selected.root()
        {
            eprintln!(
                "original_definition mismatch namespace_equal={} selected_name={:?} definition_name={:?} selected_lifetime={} definition_lifetime={}",
                selected_identity == identity,
                selected_simple.as_bytes(),
                simple.as_bytes(),
                matches!(
                    selected,
                    crate::var_resolve::VariableCellKey::Lifetime { .. }
                ),
                matches!(key, crate::var_resolve::VariableCellKey::Lifetime { .. }),
            );
        }
    }
}

impl<'a> SsaSourceView<'a> {
    /// Whether this query selects one operation rather than compatibility-wide bindings.
    #[must_use]
    pub const fn is_positioned(self) -> bool {
        self.point.is_some()
    }

    /// Query only bindings proved unique over the whole function.
    #[must_use]
    pub const fn unpositioned(ssa: &'a SsaFunction) -> Self {
        Self { ssa, point: None }
    }

    /// Query bindings reaching one statement's evaluation.
    #[must_use]
    pub const fn at_statement(ssa: &'a SsaFunction, block: BlockId, index: usize) -> Self {
        Self {
            ssa,
            point: Some((block, index)),
        }
    }

    /// Query bindings reaching one terminator.
    #[must_use]
    pub const fn at_terminator(ssa: &'a SsaFunction, block: BlockId) -> Self {
        Self::at_statement(ssa, block, usize::MAX)
    }

    /// Return the selected function's SSA value tables.
    #[must_use]
    pub const fn function(self) -> &'a SsaFunction {
        self.ssa
    }

    /// Iterate source spellings with their proved symbols; missing positioned facts yield no names.
    pub fn source_symbols(self) -> impl Iterator<Item = (&'a str, Symbol)> {
        self.source_names()
            .into_iter()
            .flat_map(|names| names.iter().map(|(name, &symbol)| (name.as_str(), symbol)))
            .chain(
                self.point
                    .is_none()
                    .then_some(&self.ssa.var_to_symbol)
                    .into_iter()
                    .flat_map(|names| names.iter().map(|(name, &symbol)| (name.as_str(), symbol))),
            )
    }

    /// Whether explicit external mutation dependencies cover this physical cell.
    /// Canonical storage keys and names resolved in the retained environment
    /// at this exact point can match; diagnostic display labels cannot. The
    /// sparse operand inventory is not evidence that an alias is absent.
    /// Missing point environments or unresolved dependencies retain uncertainty.
    #[must_use]
    pub fn externally_mutable_by(
        self,
        symbol: Symbol,
        names: &HashSet<String>,
        dynamic_trace: bool,
        registry: &CommandRegistry,
    ) -> Option<bool> {
        if self.ssa.has_opaque_native_accesses()
            || dynamic_trace
            || self
                .ssa
                .cell_key(symbol)
                .authored_spelling()
                .is_some_and(|name| names.contains(name))
        {
            return Some(true);
        }
        if names.is_empty() {
            return Some(false);
        }
        self.source_names()?;
        let (block, index) = self.point?;
        let points = self.ssa.point_contexts.as_ref()?;
        let context = if index == usize::MAX {
            points.before_terminator(block)
        } else {
            points.before_statement(block, index)
        };
        let mut unknown = false;
        for name in names {
            if let Some(&target) = self.ssa.cell_to_symbol.get(&VariableCellKey::from(name)) {
                if target == symbol {
                    return Some(true);
                }
                continue;
            }
            if let Some(bound) = canonical_variable_key(name, context, registry) {
                if &bound == self.ssa.cell_key(symbol) {
                    return Some(true);
                }
            } else {
                unknown = true;
            }
        }
        (!unknown).then_some(false)
    }

    /// Whether this statement's represented value survives its successful
    /// store callbacks in the same physical cell. This proves only its normal
    /// successor contents, and grants no future alias, lifetime or purity proof.
    #[must_use]
    pub fn normal_store_contents_preserved(
        self,
        symbol: Symbol,
        registry: &CommandRegistry,
    ) -> bool {
        let Some((block, index)) = self.point else {
            return false;
        };
        let Some(statement) = self
            .ssa
            .blocks
            .get(&block)
            .and_then(|body| body.statements.get(index))
        else {
            return false;
        };
        if !statement.defs.contains_key(&symbol) {
            return false;
        }
        let Some(points) = self.ssa.point_contexts.as_ref() else {
            return false;
        };
        if points.context_before(block, index).is_none() {
            return false;
        }
        let Some(tokens) = self.source_tokens() else {
            return false;
        };
        let Some(binding) = tokens.source_binding.as_ref() else {
            return false;
        };
        let Some(site) = binding.invocation_site() else {
            return false;
        };
        let captured = binding.normal_variable_continuation();
        let after = captured.unwrap_or_else(|| points.after_statement(block, index));
        crate::place_bridge::def_places_at(&statement.statement, block, index, points, registry)
            .iter()
            .any(|place| {
                if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
                    eprintln!("ORIGINAL_STORE_CONTENTS site={} captured={} cell_matches={} dynamic={} presence={:?} generation_current={} source_matches={} offset_matches={}",
                        site.offset, captured.is_some(),
                        crate::var_resolve::canonical_binding_value_key(place).as_ref() == Some(self.ssa.cell_key(symbol)),
                        place.dynamic, if captured.is_some() { after.captured_contents_presence(place) } else { after.contents_presence(place) },
                        place.cell.as_ref().is_some_and(|cell| cell.generation != crate::place::CellGeneration::Unknown
                            && after.generations.get(&crate::var_resolve::cell_key(place)).copied().unwrap_or_default() == cell.generation),
                        after.contents_have_source(place, &site.source),
                        after.contents_origin(place) == crate::var_resolve::ContentsOrigin::WrittenAt(site.offset));
                }
                crate::var_resolve::canonical_binding_value_key(place).as_ref()
                    == Some(self.ssa.cell_key(symbol))
                    && !place.dynamic
                    && if captured.is_some() {
                        after.captured_contents_presence(place)
                    } else {
                        after.contents_presence(place)
                    } == crate::var_resolve::ContentsPresence::Defined
                    && place.cell.as_ref().is_some_and(|cell| {
                        !matches!(cell.generation, crate::place::CellGeneration::Unknown)
                            && after
                                .generations
                                .get(&crate::var_resolve::cell_key(place))
                                .copied()
                                .unwrap_or(crate::place::CellGeneration::Incoming)
                                == cell.generation
                    })
                    && after.contents_have_source(place, &site.source)
                    && after.contents_origin(place)
                        == crate::var_resolve::ContentsOrigin::WrittenAt(site.offset)
            })
    }

    /// Resolve a source spelling using the selected proof context.
    #[must_use]
    pub fn symbol(self, name: &str) -> Option<Symbol> {
        self.point.map_or_else(
            || self.ssa.var_symbol(name),
            |(block, index)| self.ssa.var_symbol_at(block, index, name),
        )
    }

    /// Resolve contents reaching this boundary without inventing a lexical read.
    /// The name is a lookup input; cell identity, lifetime, observers and represented
    /// store provenance determine whether its reaching version is usable.
    #[must_use]
    pub fn reaching_binding(
        self,
        name: &str,
        registry: &CommandRegistry,
    ) -> Option<SsaReadReference> {
        self.reaching_binding_with_origins(
            name,
            registry,
            &represented_contents_origins(&self.ssa.blocks, &self.ssa.value_clobbers),
        )
    }

    fn reaching_binding_with_origins(
        self,
        name: &str,
        registry: &CommandRegistry,
        origins: &HashMap<(Symbol, Version), ContentsOrigin>,
    ) -> Option<SsaReadReference> {
        let (block_id, index) = self.point?;
        let block = self.ssa.blocks.get(&block_id)?;
        let points = self.ssa.point_contexts.as_ref()?;
        if index != usize::MAX {
            block.statements.get(index)?;
        }
        let context = points.context_before(block_id, index)?;
        let place = crate::var_resolve::resolve_literal_access(
            name,
            context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        let canonical = crate::var_resolve::canonical_binding_value_key(&place)?;
        let symbol = self.ssa.cell_to_symbol.get(&canonical).copied()?;
        let candidate = if index == usize::MAX {
            block.exit_versions.get(&symbol).copied().unwrap_or(0)
        } else {
            block.statements[..index]
                .iter()
                .enumerate()
                .rev()
                .find_map(|(index, statement)| {
                    self.ssa
                        .value_clobbers
                        .get(&block_id)
                        .and_then(|markers| markers.get(&index))
                        .and_then(|versions| versions.get(&symbol))
                        .map(|&(_, fresh)| fresh)
                        .or_else(|| statement.defs.get(&symbol).copied())
                })
                .or_else(|| block.entry_versions.get(&symbol).copied())
                .unwrap_or(0)
        };
        let origin = context.read_contents_origin(&place, registry);
        let represented = if candidate == 0 {
            Some(ContentsOrigin::Incoming)
        } else {
            origins.get(&(symbol, candidate)).cloned()
        };
        let version = (!place.observed
            && context.contents_presence(&place) == crate::var_resolve::ContentsPresence::Defined
            && !context.store_would_error(&place)
            && origin != ContentsOrigin::Unknown
            && represented.as_ref() == Some(&origin))
        .then_some(candidate);
        Some(SsaReadReference { symbol, version })
    }

    /// Candidate names from retained source operands, resolved afresh at this boundary.
    /// This inventory grants no whole-function binding or contents consensus.
    pub fn reaching_bindings(
        self,
        registry: &CommandRegistry,
    ) -> impl Iterator<Item = (&'a str, SsaReadReference)> {
        let names: BTreeSet<_> = self
            .ssa
            .point_symbols
            .values()
            .flat_map(|names| names.keys().map(String::as_str))
            .collect();
        let origins = represented_contents_origins(&self.ssa.blocks, &self.ssa.value_clobbers);
        names.into_iter().filter_map(move |name| {
            self.reaching_binding_with_origins(name, registry, &origins)
                .map(|binding| (name, binding))
        })
    }

    /// Original words retained at this exact operation; absent provenance stays absent.
    #[must_use]
    pub fn source_tokens(self) -> Option<&'a CommandTokens> {
        let (block, index) = self.point?;
        self.ssa
            .point_contexts
            .as_ref()?
            .source_tokens_at(block, index)
    }

    /// Match an SSA definition under explicitly standalone catalogue selection.
    /// Supplied-input consumers use `original_definition_name_with_metadata_context`.
    /// Definition phase, effective-to-written ordinal and byte-cell identity
    /// come from the retained normal-transfer and original operand owners.
    /// Equal names, catalogue roles or a source span cannot issue this mapping.
    #[must_use]
    pub fn original_definition_name(
        self,
        symbol: Symbol,
        registry: &CommandRegistry,
    ) -> Option<OriginalSsaDefinitionName> {
        let metadata = registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile)
            .map(crate::registry_invocation::InvocationMetadataContext::from);
        self.original_definition_name_with_metadata_context(symbol, registry, metadata)
    }

    /// Join the exact original SSA definition under supplied availability and
    /// source grammar. Missing, foreign or incompatible metadata stays absent;
    /// physical cell and successful-transfer requirements remain independent.
    #[must_use]
    pub fn original_definition_name_with_metadata_context(
        self,
        symbol: Symbol,
        registry: &CommandRegistry,
        metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    ) -> Option<OriginalSsaDefinitionName> {
        use tcl_registry::native_compilation::VariableOperandBindingPhase;
        let (block, index) = self.point?;
        let statement = self.ssa.blocks.get(&block)?.statements.get(index)?;
        statement.defs.get(&symbol)?;
        let points = self.ssa.point_contexts.as_ref()?;
        let before = points.context_before(block, index)?;
        let tokens = self.source_tokens()?;
        if tokens.synthetic.is_some() {
            return None;
        }
        let binding = tokens.source_binding.as_ref()?;
        let metadata = supplied_ssa_metadata(
            registry,
            binding.original_lexer_config_for_tokens(tokens)?,
            metadata,
        )?;
        let selected_invocation =
            crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
                registry,
                Some(metadata),
                tokens,
            );
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_SSA_DEFINITION").is_some() {
            eprintln!(
                "original_definition admission site={} normal={} operand_owner={} before_policy={} key_kind={:?}",
                tokens
                    .words()
                    .first()
                    .map_or(0, |word| word.source().span.start()),
                selected_invocation.is_some(),
                binding
                    .original_variable_operands_for_tokens(tokens)
                    .is_some(),
                before.execution_name_policy.is_some(),
                match self.ssa.cell_key(symbol).root() {
                    crate::var_resolve::VariableCellKey::Activation { .. } => "activation",
                    crate::var_resolve::VariableCellKey::Namespace { .. } => "namespace",
                    crate::var_resolve::VariableCellKey::Authored(_) => "authored",
                    _ => "other",
                }
            );
        }
        let invocation = selected_invocation?;
        let context = match invocation.variable_binding_phase() {
            VariableOperandBindingPhase::AfterArguments => before,
            VariableOperandBindingPhase::NormalContinuation => binding
                .normal_variable_continuation()
                .unwrap_or_else(|| points.after_statement(block, index)),
            VariableOperandBindingPhase::BodyProtocol => return None,
        };
        let key = self.ssa.cell_key(symbol);
        let mut found = None;
        for (written, place) in invocation.written_definition_places(context, registry) {
            #[cfg(debug_assertions)]
            trace_original_definition_place(tokens, context, written, &place, key);
            if crate::var_resolve::canonical_binding_value_key(&place).as_ref() != Some(key) {
                continue;
            }
            let written = written.checked_add(1)?;
            let input = binding.original_written_name_input(tokens, written)?;
            let span = tokens.words().get(written)?.source().span;
            if found
                .as_ref()
                .is_some_and(|(old_span, old_input)| *old_span != span || old_input != &input)
            {
                return None;
            }
            found = Some((span, input));
        }
        let (span, input) = found?;
        Some(OriginalSsaDefinitionName {
            span,
            input,
            cell: key.clone(),
        })
    }

    /// Propose an unused scalar slot at this genuine reached source operation.
    /// Complete entered local ownership and observer closure are required;
    /// namespace/global/event availability is deliberately not inferred.
    #[must_use]
    pub fn fresh_scalar_variable_proposal(
        self,
        native_name: &[u8],
        registry: &CommandRegistry,
    ) -> Option<OriginalFreshScalarVariableProposal> {
        let (block, index) = self.point?;
        let context = self
            .ssa
            .point_contexts
            .as_ref()?
            .context_before(block, index)?;
        let tokens = self.source_tokens()?;
        if tokens.synthetic.is_some() {
            return None;
        }
        let binding = tokens.source_binding.as_ref()?;
        if binding.runtime_reachability()
            != crate::command_binding::SourceRuntimeReachability::Reached
        {
            return None;
        }
        let site = binding.invocation_site()?.clone();
        let config = binding.original_lexer_config_for_tokens(tokens)?;
        let slot = context.fresh_scalar_variable_slot(native_name, registry)?;
        Some(OriginalFreshScalarVariableProposal { slot, site, config })
    }

    /// Propose an unused local scalar and independently select its current
    /// registered setter at the same original source operation. Neither
    /// receipt can replace the other's point, policy or parser configuration.
    #[must_use]
    pub fn fresh_scalar_assignment_proposal(
        self,
        native_name: &[u8],
        registry: &CommandRegistry,
    ) -> Option<OriginalFreshScalarAssignmentProposal> {
        let variable = self.fresh_scalar_variable_proposal(native_name, registry)?;
        let tokens = self.source_tokens()?;
        let binding = tokens.source_binding.as_ref()?;
        let command = binding.original_scalar_assignment_command(tokens, registry)?;
        if command.site() != variable.site()
            || command.lexer_config() != variable.lexer_config()
            || command.policy() != variable.policy()
            || !command.matches_original_point(binding)
        {
            return None;
        }
        Some(OriginalFreshScalarAssignmentProposal { variable, command })
    }

    /// Whether an activation-owned dependency belongs to this operation's
    /// actual frame. Callee summaries retain their private cells for effects;
    /// those cells do not become lexical reads in the caller. Missing frame
    /// provenance and non-activation cells provide no membership conclusion.
    #[must_use]
    pub(crate) fn owns_activation_cell(self, symbol: Symbol) -> Option<bool> {
        let VariableCellKey::Activation { identity, .. } = self.ssa.cell_key(symbol).root() else {
            return None;
        };
        let (block, index) = self.point?;
        let context = self
            .ssa
            .point_contexts
            .as_ref()?
            .context_before(block, index)?;
        Some(context.activation.as_ref()? == identity)
    }

    /// Query a substitution's canonical read without borrowing another read's context.
    #[must_use]
    pub fn read_reference(self, source: &SourceSite, spelling: &str) -> Option<SsaReadReference> {
        let (block, index) = self.point?;
        self.ssa.read_reference_at(block, index, source, spelling)
    }

    /// Exact attempted-read cell and its operation's SSA version. This can
    /// identify an undefined version for diagnostics; it proves neither a
    /// completed read nor a produced value. Missing source or cell provenance
    /// never falls back to a written variable label.
    #[must_use]
    pub(crate) fn read_occurrence_reference(
        self,
        source: &SourceSite,
        spelling: &str,
    ) -> Option<(Symbol, Version)> {
        let (block, index) = self.point?;
        let read = self.read_reference(source, spelling)?;
        let block = self.ssa.blocks.get(&block)?;
        let version = if index == usize::MAX {
            block.exit_versions.get(&read.symbol)?
        } else {
            block.statements.get(index)?.uses.get(&read.symbol)?
        };
        Some((read.symbol, *version))
    }

    /// Completion of an exact reached substitution, independently of SSA values.
    /// Every retained physical context must prove the same variable-read error.
    /// An observer, unknown residual or absent source carrier cannot supply that
    /// proof; an undefined read never needs a fabricated incoming SSA version.
    #[must_use]
    pub fn read_completion_at(
        self,
        source: &SourceSite,
        spelling: &str,
        registry: &CommandRegistry,
    ) -> Option<crate::var_resolve::VariableReadCompletion> {
        self.read_properties_at(source, spelling, registry)
            .map(|(completion, _)| completion)
    }

    /// Definedness of one exact reached read, independently of value versions.
    /// All closed physical alternatives must agree. Undefined contents are
    /// distinct from an existing array root rejected by a scalar read.
    #[must_use]
    pub fn read_contents_presence_at(
        self,
        source: &SourceSite,
        spelling: &str,
        registry: &CommandRegistry,
    ) -> Option<crate::var_resolve::ContentsPresence> {
        self.read_properties_at(source, spelling, registry)
            .map(|(_, presence)| presence)
    }

    /// Closed presence alternatives at one original reached read. Mixed
    /// Defined/Undefined alternatives can support a potential missing-read
    /// diagnostic; they cannot supply a constant or a definite execution error.
    /// Missing provenance, unknown worlds and read observers decline.
    #[must_use]
    pub fn read_contents_presence_alternatives_at(
        self,
        source: &SourceSite,
        spelling: &str,
        registry: &CommandRegistry,
    ) -> Option<SsaReadPresenceAlternatives> {
        use crate::var_resolve::ContentsPresence;
        let access = crate::command_binding::SourceVariableAccess::find_at_source(
            &self.source_tokens()?.variable_accesses,
            source,
            spelling,
        )?;
        if access.context_alternatives().is_empty()
            || access.context_residual()
                != crate::command_binding::SourceVariableReadResidual::Closed
        {
            return None;
        }
        let mut contains_undefined = false;
        for context in access.context_alternatives() {
            let place = access.place_in_context(context, registry);
            if place.observed {
                return None;
            }
            match context.closed_contents_presence(&place)? {
                ContentsPresence::Defined => {}
                ContentsPresence::Undefined | ContentsPresence::DefinedOrUndefined => {
                    contains_undefined = true;
                }
                ContentsPresence::Unknown => return None,
            }
        }
        Some(SsaReadPresenceAlternatives { contains_undefined })
    }

    fn read_properties_at(
        self,
        source: &SourceSite,
        spelling: &str,
        registry: &CommandRegistry,
    ) -> Option<(
        crate::var_resolve::VariableReadCompletion,
        crate::var_resolve::ContentsPresence,
    )> {
        use crate::var_resolve::{ContentsPresence, VariableReadCompletion};

        let access = crate::command_binding::SourceVariableAccess::find_at_source(
            &self.source_tokens()?.variable_accesses,
            source,
            spelling,
        )?;
        let contexts = access.context_alternatives();
        if contexts.is_empty()
            || access.context_residual()
                != crate::command_binding::SourceVariableReadResidual::Closed
        {
            return Some((VariableReadCompletion::Unknown, ContentsPresence::Unknown));
        }
        let mut errors = true;
        let mut presence = None;
        for context in contexts {
            let place = access.place_in_context(context, registry);
            errors &= context.read_completion(&place) == VariableReadCompletion::Error;
            let alternative = context.contents_presence(&place);
            presence = Some(match presence {
                None => alternative,
                Some(previous) if previous == alternative => previous,
                Some(_) => ContentsPresence::Unknown,
            });
        }
        Some((
            if errors {
                VariableReadCompletion::Error
            } else {
                VariableReadCompletion::Unknown
            },
            presence.unwrap_or(ContentsPresence::Unknown),
        ))
    }

    /// Resolve a word consisting solely of one substitution using its exact source site.
    #[must_use]
    pub fn read_word(self, word: &WordExpr) -> Option<SsaReadReference> {
        let access = self.word_variable_access(word)?;
        self.read_reference(&access.source, &access.original_spelling)
    }

    /// Definition supplying one captured original argv value before a nested
    /// handler runs. This contents dependency leaves the physical SSA read and
    /// every later barrier unchanged; it grants no representation or erasure.
    pub(crate) fn captured_argument_value_definition(
        self,
        word: &WordExpr,
        invocation: &crate::command_binding::CommandAllocationSite,
        registry: &CommandRegistry,
    ) -> Option<ValueKey> {
        let access = self.word_variable_access(word)?;
        if !exclusive_argument_owner(&access.owner, invocation)
            || access.context_residual()
                != crate::command_binding::SourceVariableReadResidual::Closed
            || !self.read_word_produces_value(word, registry)
        {
            return None;
        }
        let place = source_read_place(access, registry);
        let cell = place.cell.as_ref()?;
        if place.dynamic
            || place.observed
            || place.kind != crate::place::PlaceKind::Scalar
            || cell.generation == crate::place::CellGeneration::Unknown
            || access.context_alternatives().iter().any(|context| {
                let alternative = access.place_in_context(context, registry);
                alternative.cell.as_ref() != Some(cell)
                    || alternative.dynamic
                    || alternative.observed
                    || context.read_contents_origin(&alternative, registry)
                        != access
                            .variable_context
                            .read_contents_origin(&place, registry)
            })
        {
            return None;
        }
        let contents = self.read_word_contents(word, registry)?;
        if contents.unknown_residual || contents.includes_incoming {
            return None;
        }
        let [(write_block, write_index)] = contents.writes.as_slice() else {
            return None;
        };
        let (read_block, read_index) = self.point?;
        if *write_block != read_block || *write_index >= read_index {
            return None;
        }
        let statement = self
            .ssa
            .blocks
            .get(write_block)?
            .statements
            .get(*write_index)?;
        let writer = SsaSourceView::at_statement(self.ssa, *write_block, *write_index)
            .source_tokens()?
            .source_binding
            .as_ref()?
            .invocation_site()?;
        if writer.source != invocation.source {
            return None;
        }
        let version = *statement.defs.get(&contents.reference.symbol)?;
        Some((contents.reference.symbol, version))
    }

    fn word_variable_access(
        self,
        word: &WordExpr,
    ) -> Option<&'a crate::command_binding::SourceVariableAccess> {
        let (spelling, source) = word.sole_variable_substitution()?;
        self.source_tokens()?
            .variable_access_for_site(source)
            .filter(|access| access.original_spelling == spelling)
    }

    /// Every retained native context for this exact substitution proves that
    /// its read produces a value. This is separate from address, SSA version,
    /// representation and command completion evidence.
    #[must_use]
    pub fn read_word_produces_value(self, word: &WordExpr, registry: &CommandRegistry) -> bool {
        let Some(access) = self.word_variable_access(word) else {
            return false;
        };
        Self::access_produces_value(access, registry)
    }

    /// Completed value production at one retained original substitution site,
    /// including a variable component inside an interpolated word. Every
    /// physical alternative must prove the read; a label cannot supply a site.
    #[must_use]
    pub fn read_produces_value_at(
        self,
        source: &SourceSite,
        original_spelling: &str,
        registry: &CommandRegistry,
    ) -> bool {
        let Some(tokens) = self.source_tokens() else {
            return false;
        };
        let Some(access) = crate::command_binding::SourceVariableAccess::find_at_source(
            &tokens.variable_accesses,
            source,
            original_spelling,
        ) else {
            return false;
        };
        Self::access_produces_value(access, registry)
    }

    fn access_produces_value(
        access: &crate::command_binding::SourceVariableAccess,
        registry: &CommandRegistry,
    ) -> bool {
        !access.context_alternatives().is_empty()
            && access.context_residual()
                == crate::command_binding::SourceVariableReadResidual::Closed
            && access.context_alternatives().iter().all(|context| {
                let place = access.place_in_context(context, registry);
                context.read_produces_value(&place, registry)
            })
    }

    /// One represented value read which can be replaced without removing a
    /// variable observer. Every original physical context must select the same
    /// bounded scalar cell and produce a value. This grants no source extent,
    /// store deletion, alias continuity or movement of the producing expression.
    #[must_use]
    pub fn replaceable_read_at(
        self,
        source: &SourceSite,
        original_spelling: &str,
        registry: &CommandRegistry,
    ) -> Option<SsaReadReference> {
        let tokens = self.source_tokens()?;
        let access = crate::command_binding::SourceVariableAccess::find_at_source(
            &tokens.variable_accesses,
            source,
            original_spelling,
        )?;
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
            eprintln!(
                "ORIGINAL_REPLACEABLE_READ offset={} alternatives={} residual={:?} produces={}",
                source.span.start(),
                access.context_alternatives().len(),
                access.context_residual(),
                Self::access_produces_value(access, registry)
            );
            for context in access.context_alternatives() {
                let place = access.place_in_context(context, registry);
                eprintln!(
                    "ORIGINAL_REPLACEABLE_READ offset={} place_kind={:?} observed={} dynamic={} presence={:?} generation_known={}",
                    source.span.start(),
                    place.kind,
                    place.observed,
                    place.dynamic,
                    context.contents_presence(&place),
                    place.cell.as_ref().is_some_and(
                        |cell| cell.generation != crate::place::CellGeneration::Unknown
                    )
                );
            }
        }
        if !Self::access_produces_value(access, registry) {
            return None;
        }
        let reference = self.read_reference(source, original_spelling)?;
        reference.version?;
        access
            .context_alternatives()
            .iter()
            .all(|context| {
                let place = access.place_in_context(context, registry);
                !place.observed
                    && !place.dynamic
                    && place.kind == crate::place::PlaceKind::Scalar
                    && place.cell.as_ref().is_some_and(|cell| {
                        cell.generation != crate::place::CellGeneration::Unknown
                    })
                    && crate::var_resolve::canonical_binding_value_key(&place).as_ref()
                        == Some(self.ssa.cell_key(reference.symbol))
            })
            .then_some(reference)
    }

    /// Actual representation at this captured read, independently of a
    /// producer's semantic type. Unknown, observed or disagreeing native
    /// contexts cannot donate a representation from an earlier constructor.
    #[must_use]
    pub fn read_word_representation(
        self,
        word: &WordExpr,
        registry: &CommandRegistry,
    ) -> Option<tcl_syntax::value::ValueRepresentation> {
        self.read_word_representation_advice(word, registry)
            .representation
    }

    /// Closed ordinary container possibilities for conversion-cost advice.
    /// Every original read must succeed unobserved in its actual live cell;
    /// this grants no single representation, constant or erasure proof.
    #[must_use]
    pub fn read_word_representation_alternatives(
        self,
        word: &WordExpr,
        registry: &CommandRegistry,
    ) -> Option<crate::native_numeric::ClosedContainerRepresentations> {
        self.read_word_representation_advice(word, registry)
            .container_alternatives
    }

    pub(crate) fn read_word_representation_advice(
        self,
        word: &WordExpr,
        registry: &CommandRegistry,
    ) -> SsaReadRepresentationAdvice {
        self.word_variable_access(word)
            .map_or_else(SsaReadRepresentationAdvice::default, |access| {
                source_read_representations(access, registry)
            })
    }

    /// Representation advice for an actually reached native name-operand
    /// read of this physical SSA cell. This uses the retained post-argv read
    /// inventory, never the context before argument evaluation. All matching
    /// reads must succeed unobserved and agree; absent or opaque inventories
    /// donate neither a representation nor closed container possibilities.
    pub(crate) fn native_read_representation_advice(
        self,
        symbol: Symbol,
        registry: &CommandRegistry,
    ) -> SsaReadRepresentationAdvice {
        use crate::command_binding::SourceVariableReadResidual;

        let Some(reads) = self
            .source_tokens()
            .and_then(|tokens| tokens.source_binding.as_ref())
            .and_then(|binding| binding.invocation_variable_reads.as_ref())
            .filter(|reads| reads.residual == SourceVariableReadResidual::Closed)
        else {
            return SsaReadRepresentationAdvice::default();
        };
        captured_read_representations(
            reads
                .native_reads
                .iter()
                .filter(|read| {
                    crate::var_resolve::canonical_binding_value_key(&read.place).as_ref()
                        == Some(self.ssa.cell_key(symbol))
                })
                .map(|read| (read.variable_context.as_ref(), read.place.clone())),
            registry,
        )
    }

    /// Physical address selected by one retained sole variable substitution.
    /// This reports address and observer facts independently of represented
    /// scalar contents; it does not license a value or representation fold.
    #[must_use]
    pub fn read_word_place(
        self,
        word: &WordExpr,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::place::Place> {
        let access = self.word_variable_access(word)?;
        Some(access.place_in_context(&access.variable_context, registry))
    }

    /// Project the contents alternatives of one exact retained read.
    /// Writes must retain the same physical root and lifetime; source offsets
    /// alone never make an unrelated store a candidate.
    #[must_use]
    pub fn read_contents_at(
        self,
        source: &SourceSite,
        spelling: &str,
        registry: &CommandRegistry,
    ) -> Option<SsaReadContents> {
        let access = crate::command_binding::SourceVariableAccess::find_at_source(
            &self.source_tokens()?.variable_accesses,
            source,
            spelling,
        )?;
        let reference = self.read_reference(source, spelling)?;
        let target = source_read_place(access, registry);
        let origin = access
            .variable_context
            .read_contents_origin(&target, registry);
        let (includes_incoming, offsets) = match &origin {
            ContentsOrigin::Incoming => (true, &[][..]),
            ContentsOrigin::WrittenAt(offset) => (false, std::slice::from_ref(offset)),
            ContentsOrigin::Alternatives { incoming, writes } => (*incoming, writes.as_slice()),
            ContentsOrigin::Unknown => (false, &[][..]),
        };
        let mut contents = SsaReadContents {
            reference,
            origin: origin.clone(),
            includes_incoming,
            writes: Vec::new(),
            unknown_residual: origin == ContentsOrigin::Unknown || target.observed,
        };
        let points = self.ssa.point_contexts.as_ref()?;
        for offset in offsets {
            let mut found = false;
            for (&block, body) in &self.ssa.blocks {
                for (index, statement) in body.statements.iter().enumerate() {
                    if statement.statement.span().start() != *offset {
                        continue;
                    }
                    let writes = crate::place_bridge::def_places_at(
                        &statement.statement,
                        block,
                        index,
                        points,
                        registry,
                    );
                    if writes.iter().any(|write| {
                        write.cell.is_some()
                            && write.cell == target.cell
                            && crate::place::overlap(write, &target)
                    }) {
                        contents.writes.push((block, index));
                        found = true;
                    }
                }
            }
            contents.unknown_residual |= !found;
        }
        contents.writes.sort_unstable();
        contents.writes.dedup();
        Some(contents)
    }

    /// Contents alternatives for a sole variable value word, using its original
    /// captured read rather than the current value of a cached input binding.
    #[must_use]
    pub fn read_word_contents(
        self,
        word: &WordExpr,
        registry: &CommandRegistry,
    ) -> Option<SsaReadContents> {
        let access = self.word_variable_access(word)?;
        self.read_contents_at(&access.source, &access.original_spelling, registry)
    }

    /// Project a name-only query only when every retained read agrees on its contents.
    /// No reached read, a missing native grammar, or conflicting read identities
    /// leaves the projection unresolved. Exact source queries remain preferable.
    #[must_use]
    pub fn read_spelling(self, name: &str) -> Option<SsaReadReference> {
        let mut selected = None;
        for access in &self.source_tokens()?.variable_accesses {
            let dialect = access.variable_context.invocation_dialect?;
            let reference = tcl_syntax::naming::var_reference_for_style(
                &access.original_spelling,
                dialect.lexer_grammar.braced_var,
            );
            if reference != name {
                continue;
            }
            let read = self.read_reference(&access.source, &access.original_spelling)?;
            if selected.is_some_and(|previous| previous != read) {
                return None;
            }
            selected = Some(read);
        }
        selected
    }

    /// Consensus contents alternatives for evaluators accepting only a name.
    /// Differing occurrences remain unresolved; exact per-reference queries
    /// preserve more information and are preferred.
    #[must_use]
    pub fn read_spelling_contents(
        self,
        name: &str,
        registry: &CommandRegistry,
    ) -> Option<SsaReadContents> {
        let mut selected = None;
        for access in &self.source_tokens()?.variable_accesses {
            let dialect = access.variable_context.invocation_dialect?;
            if tcl_syntax::naming::var_reference_for_style(
                &access.original_spelling,
                dialect.lexer_grammar.braced_var,
            ) != name
            {
                continue;
            }
            let contents =
                self.read_contents_at(&access.source, &access.original_spelling, registry)?;
            if selected
                .as_ref()
                .is_some_and(|previous| *previous != contents)
            {
                return None;
            }
            selected = Some(contents);
        }
        selected
    }

    fn expression_variable_access(
        self,
        node: &crate::expr_ast::ExprNode,
        expr_base: Option<u32>,
    ) -> Option<&'a crate::command_binding::SourceVariableAccess> {
        let crate::expr_ast::ExprNode::Var { text, .. } = node else {
            return None;
        };
        let tokens = self.source_tokens()?;
        let dialect = tokens
            .source_binding
            .as_ref()?
            .variable_context
            .invocation_dialect?;
        let span = node.variable_source_span(
            expr_base?,
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        )?;
        let mut accesses = tokens
            .variable_accesses
            .iter()
            .filter(|access| access.source.span == span && access.original_spelling == *text);
        let access = accesses.next()?;
        accesses.next().is_none().then_some(access)
    }

    fn captured_expression_variable_access(
        self,
        node: &crate::expr_ast::ExprNode,
        expr_base: Option<u32>,
        expression: Option<&crate::command_binding::ExecutedExpressionSource>,
    ) -> Option<&'a crate::command_binding::SourceVariableAccess> {
        let Some(expression) = expression else {
            return self.expression_variable_access(node, expr_base);
        };
        let (origin, source) = expression.variable_source(node)?;
        let tokens = self.source_tokens()?;
        if tokens.source_binding.as_ref()?.source_origin()? != origin {
            return None;
        }
        let crate::expr_ast::ExprNode::Var { text, .. } = node else {
            return None;
        };
        tokens
            .variable_access_for_site(&source)
            .filter(|access| access.original_spelling == *text)
    }

    /// Representation at one original expression read, retaining the exact
    /// parser extent or evaluated-expression operand origin. Missing, observed,
    /// unknown or disagreeing physical reads yield no representation proof.
    #[must_use]
    pub fn read_expression_representation(
        self,
        node: &crate::expr_ast::ExprNode,
        expr_base: Option<u32>,
        expression: Option<&crate::command_binding::ExecutedExpressionSource>,
        registry: &CommandRegistry,
    ) -> Option<tcl_syntax::value::ValueRepresentation> {
        self.read_expression_representation_advice(node, expr_base, expression, registry)
            .representation
    }

    /// Closed List/Dict possibilities for conversion-cost advice at the exact
    /// expression read. This grants no single representation or erasure proof.
    #[must_use]
    pub fn read_expression_representation_alternatives(
        self,
        node: &crate::expr_ast::ExprNode,
        expr_base: Option<u32>,
        expression: Option<&crate::command_binding::ExecutedExpressionSource>,
        registry: &CommandRegistry,
    ) -> Option<crate::native_numeric::ClosedContainerRepresentations> {
        self.read_expression_representation_advice(node, expr_base, expression, registry)
            .container_alternatives
    }

    pub(crate) fn read_expression_representation_advice(
        self,
        node: &crate::expr_ast::ExprNode,
        expr_base: Option<u32>,
        expression: Option<&crate::command_binding::ExecutedExpressionSource>,
        registry: &CommandRegistry,
    ) -> SsaReadRepresentationAdvice {
        self.captured_expression_variable_access(node, expr_base, expression)
            .map_or_else(SsaReadRepresentationAdvice::default, |access| {
                source_read_representations(access, registry)
            })
    }

    /// Resolve an expression variable using its absolute source extent.
    /// The caller supplies the parser's original expression base; absent bases,
    /// transformed extents and conflicting read inventories yield no proof.
    #[must_use]
    pub fn read_expression_variable(
        self,
        node: &crate::expr_ast::ExprNode,
        expr_base: Option<u32>,
    ) -> Option<SsaReadReference> {
        let access = self.expression_variable_access(node, expr_base)?;
        self.read_reference(&access.source, &access.original_spelling)
    }

    /// Resolve one variable through exact evaluated-expression operand origins.
    /// A mapped source from another invocation origin cannot donate a read.
    #[must_use]
    pub fn read_executed_expression_variable(
        self,
        node: &crate::expr_ast::ExprNode,
        expression: &crate::command_binding::ExecutedExpressionSource,
    ) -> Option<SsaReadReference> {
        let (origin, source) = expression.variable_source(node)?;
        if self
            .source_tokens()?
            .source_binding
            .as_ref()?
            .source_origin()?
            != origin
        {
            return None;
        }
        let crate::expr_ast::ExprNode::Var { text, .. } = node else {
            return None;
        };
        self.read_reference(&source, text)
    }

    /// Physical address of one exact expression reference, independently of
    /// its represented value. Missing authored extents or read carriers decline.
    #[must_use]
    pub fn read_expression_variable_place(
        self,
        node: &crate::expr_ast::ExprNode,
        expr_base: Option<u32>,
        registry: &CommandRegistry,
    ) -> Option<crate::place::Place> {
        let access = self.expression_variable_access(node, expr_base)?;
        Some(source_read_place(access, registry))
    }

    /// Whether an exact read selects this activation's unobserved scalar slot.
    /// `slot` is an already validated literal slot name. Namespace, persistent,
    /// selected-frame and element bindings cannot inherit an ordinary formal's
    /// caller-supplied value evidence through a matching source spelling.
    #[must_use]
    pub fn read_expression_is_current_activation_slot(
        self,
        node: &crate::expr_ast::ExprNode,
        expr_base: Option<u32>,
        slot: &str,
        registry: &CommandRegistry,
    ) -> bool {
        let Some(access) = self.expression_variable_access(node, expr_base) else {
            return false;
        };
        let place = source_read_place(access, registry);
        !place.observed
            && !place.dynamic
            && place.kind == crate::place::PlaceKind::Scalar
            && place.index.is_none()
            && place.name == slot
            && place.cell.as_ref().is_some_and(|cell| {
                cell.name == slot
                    && matches!(&cell.owner, crate::place::CellOwner::Activation(identity)
                        if access.variable_context.activation.as_ref() == Some(identity))
            })
    }

    /// Prove an ordinary incoming slot across all reached physical activations.
    /// A compatibility join may lose a unique physical symbol while this
    /// purpose-specific input proof remains valid. Unknown residuals, changed
    /// contents, observers and static/reference targets decline atomically.
    #[must_use]
    pub fn read_expression_incoming_slot(
        self,
        node: &crate::expr_ast::ExprNode,
        expr_base: Option<u32>,
        slot: &str,
        registry: &CommandRegistry,
    ) -> Option<SsaIncomingSlotRead> {
        let access = self.expression_variable_access(node, expr_base)?;
        if access.context_residual() != crate::command_binding::SourceVariableReadResidual::Closed {
            return None;
        }
        let mut cells = Vec::new();
        for context in access.context_alternatives() {
            let cell =
                context.incoming_activation_slot(&access.original_spelling, slot, registry)?;
            if !cells.contains(&cell) {
                cells.push(cell);
            }
        }
        (!cells.is_empty()).then(|| SsaIncomingSlotRead {
            source: access.source.clone(),
            slot: slot.to_owned(),
            cells,
        })
    }

    /// Read contents alternatives for this exact expression occurrence.
    /// A physical binding without a single represented version may still have
    /// bounded reaching writes. Unknown residuals remain explicit.
    #[must_use]
    pub fn read_expression_variable_contents(
        self,
        node: &crate::expr_ast::ExprNode,
        expr_base: Option<u32>,
        registry: &CommandRegistry,
    ) -> Option<SsaReadContents> {
        let access = self.expression_variable_access(node, expr_base)?;
        self.read_contents_at(&access.source, &access.original_spelling, registry)
    }

    /// Already numeric source-produced object at this exact original read.
    /// This is representation evidence, not an invented scalar SSA version.
    #[must_use]
    pub fn read_expression_native_numeric(
        self,
        node: &crate::expr_ast::ExprNode,
        expr_base: Option<u32>,
        registry: &CommandRegistry,
    ) -> Option<crate::tcl_expr_eval::RetainedNativeOperandProof> {
        crate::native_numeric::source_read_operand(
            self.expression_variable_access(node, expr_base)?,
            registry,
        )
    }

    /// Reaching source spellings for a positioned query; absence is not a fallback.
    #[must_use]
    pub fn source_names(self) -> Option<&'a HashMap<String, Symbol>> {
        self.point
            .and_then(|(block, index)| self.ssa.source_symbols_at(block, index))
    }
}

impl<'a> From<&'a SsaFunction> for SsaSourceView<'a> {
    fn from(ssa: &'a SsaFunction) -> Self {
        Self::unpositioned(ssa)
    }
}

/// Analysis value transitions: prior and fresh versions at each marker.
pub type ValueClobbers = HashMap<BlockId, BlockValueClobbers>;

/// Per-marker prior and fresh versions within one basic block.
pub type BlockValueClobbers = HashMap<usize, HashMap<Symbol, (Version, Version)>>;

type RegistryClobberNames = HashMap<BlockId, HashMap<usize, Vec<VariableCellKey>>>;

/// Per-[`SsaFunction`] variable-name interner.
///
/// Assigns each distinct variable name a dense [`Symbol`] in first-seen order
/// and resolves a symbol back to its name. The SSA builder threads one of
/// these while renaming and hands its tables to the finished [`SsaFunction`].
#[derive(Debug, Default, Clone)]
struct VarInterner {
    names: Vec<String>,
    keys: Vec<VariableCellKey>,
    to_symbol: FxHashMap<VariableCellKey, Symbol>,
    source_symbols: FxHashMap<String, Symbol>,
}

impl VarInterner {
    fn intern(&mut self, key: &VariableCellKey) -> Symbol {
        if let Some(&symbol) = self.to_symbol.get(key) {
            return symbol;
        }
        let symbol =
            Symbol(u32::try_from(self.keys.len()).expect("SSA variable count fits in u32"));
        self.names.push(key.compatibility_name());
        self.keys.push(key.clone());
        self.to_symbol.insert(key.clone(), symbol);
        if let Some(name) = key.authored_spelling() {
            self.source_symbols.insert(name.to_owned(), symbol);
        }
        symbol
    }
}

fn register_unique_source_names(
    interner: &mut VarInterner,
    sites: &HashMap<(BlockId, usize), HashMap<String, Symbol>>,
) {
    let mut names: HashMap<&str, Option<Symbol>> = HashMap::new();
    for symbols in sites.values() {
        for (name, &symbol) in symbols {
            names
                .entry(name)
                .and_modify(|previous| {
                    if *previous != Some(symbol) {
                        *previous = None;
                    }
                })
                .or_insert(Some(symbol));
        }
    }
    for (name, symbol) in names {
        if let Some(symbol) = symbol {
            interner.source_symbols.insert(name.to_owned(), symbol);
        } else {
            interner.source_symbols.remove(name);
        }
    }
}

impl SsaFunction {
    /// Whether named SSA maps omit accesses from a retained native invocation.
    /// This residual also applies when every named use/definition map is empty.
    #[must_use]
    pub fn has_opaque_native_accesses(&self) -> bool {
        crate::ir::statements_have_opaque_native_accesses(self.blocks.values().flat_map(|block| {
            block
                .statements
                .iter()
                .map(|statement| &statement.statement)
        }))
    }

    /// A trivial SSA shell — no blocks, no dominance information — used when
    /// the complexity guard skips the expensive SSA build for an oversized
    /// body. Downstream dataflow passes run over zero blocks (a cheap no-op),
    /// and the compilation-unit builder flags the function so per-proc
    /// diagnostic passes skip it entirely.
    #[must_use]
    pub fn trivial(name: impl Into<String>, entry: BlockId, block_names: Vec<String>) -> Self {
        Self {
            name: name.into(),
            entry,
            blocks: HashMap::new(),
            value_clobbers: HashMap::new(),
            idom: HashMap::new(),
            dominance_frontier: HashMap::new(),
            dominator_tree: HashMap::new(),
            block_names,
            var_names: Vec::new(),
            cell_names: Vec::new(),
            cell_keys: Vec::new(),
            var_to_symbol: FxHashMap::default(),
            cell_to_symbol: FxHashMap::default(),
            point_symbols: HashMap::new(),
            point_contexts: None,
            read_references: HashMap::new(),
            array_root_refresh_symbols: HashSet::new(),
            array_root_refresh_versions: HashSet::new(),
        }
    }

    /// Executable version behind analysis-only scalar clobbers. Binding and
    /// provenance domains use this lineage; scalar values use the fresh key.
    #[must_use]
    pub fn binding_version(&self, symbol: Symbol, mut version: Version) -> Version {
        loop {
            let prior = self
                .value_clobbers
                .values()
                .flat_map(|markers| markers.values())
                .filter_map(|versions| versions.get(&symbol))
                .find_map(|&(prior, fresh)| (fresh == version).then_some(prior));
            let Some(prior) = prior else {
                return version;
            };
            version = prior;
        }
    }

    /// Whether this symbol is a synthetic distinct-array root refresh.
    /// Element stores refresh this aggregate dependency without writing scalar
    /// contents. Dictionary-valued array roots are ordinary value cells and
    /// are excluded from this metadata.
    #[must_use]
    pub fn is_array_root_refresh_symbol(&self, symbol: Symbol) -> bool {
        self.array_root_refresh_symbols.contains(&symbol)
    }

    /// Whether this exact value version only refreshes a distinct-array aggregate.
    /// Unlike the compatibility symbol query, this remains precise when the
    /// same binding slot later contains a recreated scalar value.
    #[must_use]
    pub fn is_array_root_refresh_version(&self, symbol: Symbol, version: Version) -> bool {
        self.array_root_refresh_versions
            .contains(&(symbol, version))
    }

    /// Relocate physical variable proofs while preserving symbols, value versions and display names.
    /// Lexical source spans are rebased by the source owner independently.
    pub fn relocate_variable_proofs(
        &mut self,
        relocation: &crate::var_resolve::VariableProofRelocation,
    ) {
        for key in &mut self.cell_keys {
            *key = relocation.cell_key(key);
        }
        self.cell_names = self
            .cell_keys
            .iter()
            .map(VariableCellKey::compatibility_name)
            .collect();
        self.cell_to_symbol = self
            .cell_to_symbol
            .iter()
            .map(|(key, &symbol)| (relocation.cell_key(key), symbol))
            .collect();
        if let Some(points) = &mut self.point_contexts {
            points.relocate_variable_proofs(relocation);
        }
    }

    /// Restore exact source ownership after instantiating an alpha-renamed body.
    /// Physical symbols and retained reference versions remain unchanged.
    pub fn restore_source_tokens(
        &mut self,
        originals: &std::collections::BTreeMap<u32, CommandTokens>,
    ) -> bool {
        self.point_contexts
            .as_mut()
            .is_none_or(|points| points.restore_source_tokens(originals))
    }

    /// Intern variable `name`, returning its [`Symbol`].
    ///
    /// Assigns the next dense id (first-seen order) the first time a name is
    /// seen and returns the existing id on re-interning, so a symbol is stable
    /// for the life of the function. Used by tests and consumers that build a
    /// fresh SSA function by hand.
    pub fn intern_var(&mut self, name: &str) -> Symbol {
        if let Some(&sym) = self.var_to_symbol.get(name) {
            return sym;
        }
        let sym = Symbol(u32::try_from(self.var_names.len()).expect("SSA var count fits in u32"));
        self.var_names.push(name.to_owned());
        self.cell_names.push(name.to_owned());
        self.cell_keys.push(name.into());
        self.var_to_symbol.insert(name.to_owned(), sym);
        self.cell_to_symbol.insert(name.into(), sym);
        sym
    }

    /// The display name of variable [`Symbol`] `sym`.
    ///
    /// # Panics
    /// Panics if `sym` was not produced by this function's variable interner.
    #[must_use]
    pub fn var_name(&self, sym: Symbol) -> &str {
        &self.var_names[sym.0 as usize]
    }

    /// Compatibility label for a cell. Use [`Self::cell_key`] for storage identity.
    ///
    /// # Panics
    /// Panics if `sym` was not produced by this function's variable interner.
    #[must_use]
    pub fn cell_name(&self, sym: Symbol) -> &str {
        &self.cell_names[sym.0 as usize]
    }

    /// Exact physical key represented by this symbol.
    #[must_use]
    pub fn cell_key(&self, symbol: Symbol) -> &VariableCellKey {
        &self.cell_keys[symbol.0 as usize]
    }

    /// Exact storage inventory, independent of diagnostic labels.
    #[must_use]
    pub fn cell_keys(&self) -> &[VariableCellKey] {
        &self.cell_keys
    }

    /// Compatibility cell labels indexed by symbol; these do not prove identity.
    #[must_use]
    pub fn cell_names(&self) -> &[String] {
        &self.cell_names
    }

    /// Whether the def of `name` recorded at (`block_name`, `stmt_idx`) is a
    /// **synthetic** array-element may-def ([`SsaStatement::may_defs`]) — the
    /// base refresh of an element write, or the element fan of a dynamic-key
    /// / whole-array write — rather than a write the statement performs
    /// itself. Unused-variable / dead-store reporting skips these: the user
    /// wrote one assignment, not one per fanned symbol.
    #[must_use]
    pub fn is_synthetic_def<Q: VariableCellKeyQuery + ?Sized>(
        &self,
        block_name: &str,
        stmt_idx: i32,
        name: &Q,
    ) -> bool {
        let key = name.variable_cell_key();
        let Some(sym) = self.cell_symbol(key.as_ref()).or_else(|| {
            key.authored_spelling()
                .and_then(|name| self.var_symbol(name))
        }) else {
            return false;
        };
        let Ok(idx) = usize::try_from(stmt_idx) else {
            return false;
        };
        self.blocks
            .values()
            .find(|b| b.name == block_name)
            .and_then(|b| b.statements.get(idx))
            .is_some_and(|st| st.may_defs.contains(&sym))
    }

    /// The [`Symbol`] a variable name resolves to, if it was interned during
    /// SSA construction. Returns `None` for a name that is not an SSA variable
    /// of this function — a lookup of such a name in any [`ValueKey`]-keyed map
    /// is a miss, which is the correct "no fact" answer.
    #[must_use]
    pub fn var_symbol(&self, name: &str) -> Option<Symbol> {
        self.var_to_symbol.get(name).copied()
    }

    /// Resolve a source spelling using the binding at its actual operation.
    #[must_use]
    pub fn var_symbol_at(&self, block: BlockId, index: usize, name: &str) -> Option<Symbol> {
        self.point_symbols
            .get(&(block, index))
            .and_then(|symbols| symbols.get(name))
            .copied()
    }

    /// Read identity and represented contents version for an exact lexical reference.
    /// `usize::MAX` selects a terminator. Missing records never use statement-name lookup.
    #[must_use]
    pub fn read_reference_at(
        &self,
        block: BlockId,
        index: usize,
        source: &SourceSite,
        spelling: &str,
    ) -> Option<SsaReadReference> {
        self.read_references
            .get(&(block, index, source.clone(), spelling.to_owned()))
            .copied()
    }

    /// Reaching source spellings and their proved symbols at one operation.
    /// The terminator uses `usize::MAX`; absent entries have no proven binding.
    #[must_use]
    pub fn source_symbols_at(
        &self,
        block: BlockId,
        index: usize,
    ) -> Option<&HashMap<String, Symbol>> {
        self.point_symbols.get(&(block, index))
    }

    /// Resolve a stored canonical cell name independently of source aliases.
    #[must_use]
    pub fn cell_symbol<Q: VariableCellKeyQuery + ?Sized>(&self, query: &Q) -> Option<Symbol> {
        let key = query.variable_cell_key();
        self.cell_to_symbol.get(key.as_ref()).copied().or_else(|| {
            let name = key.authored_spelling()?;
            let mut matches = self
                .var_names
                .iter()
                .enumerate()
                .filter(|(index, display)| {
                    *display == name && self.cell_keys[*index].authored_spelling().is_some()
                });
            let (index, _) = matches.next()?;
            matches
                .next()
                .is_none()
                .then(|| Symbol(u32::try_from(index).expect("SSA variable count fits in u32")))
        })
    }

    /// Resolve a source spelling at a terminator's reaching frame context.
    #[must_use]
    pub fn var_symbol_at_terminator(&self, block: BlockId, name: &str) -> Option<Symbol> {
        if self.point_contexts.is_none() {
            return self.var_symbol(name);
        }
        self.var_symbol_at(block, usize::MAX, name)
    }

    /// The interned variable names, indexed by [`Symbol`]`.0` (first-seen order).
    #[must_use]
    pub fn var_names(&self) -> &[String] {
        &self.var_names
    }

    /// The display name of block `id`.
    ///
    /// # Panics
    /// Panics if `id` is outside this function's interner range.
    #[must_use]
    pub fn block_name(&self, id: BlockId) -> &str {
        &self.block_names[id.0 as usize]
    }

    /// The interned block names, indexed by [`BlockId`]`.0`.
    #[must_use]
    pub fn block_names(&self) -> &[String] {
        &self.block_names
    }

    /// The [`BlockId`] a block name resolves to, if interned. Linear scan over
    /// the (per-function, small) name table.
    #[must_use]
    pub fn block_id(&self, name: &str) -> Option<BlockId> {
        self.block_names
            .iter()
            .position(|n| n == name)
            .map(|i| BlockId(u32::try_from(i).expect("block count fits in u32")))
    }

    /// Build the O(1)-query dominance index for this function
    /// ([`DominatorIntervals`]).  One O(V) walk; build it once per function
    /// and reuse it for every query.
    #[must_use]
    pub fn dominator_intervals(&self) -> DominatorIntervals {
        DominatorIntervals::build(self)
    }
}

/// Dominator-tree DFS interval numbering: answers `dominates(a, b)` in O(1).
///
/// The straightforward answer walks `b`'s immediate-dominator chain looking
/// for `a`, which is O(depth) with a hash lookup per hop — and on a flat
/// N-branch dispatch chain the idom chain *is* the whole function, so a
/// per-block-pair loop over it is O(V²).  A pre-order DFS of
/// the dominator tree instead assigns every block a half-open `[enter, exit)`
/// interval that contains exactly its dominator-tree subtree, and `a`
/// dominates `b` iff `b`'s interval nests inside `a`'s.
///
/// Blocks outside the dominator forest (unreachable, hence absent from
/// `idom`) have no interval and dominate nothing but themselves — the same
/// answer the chain walk gives, since it runs out of parents.
#[derive(Debug, Clone, Default)]
pub struct DominatorIntervals {
    /// `(enter, exit)` per block, indexed by [`BlockId`]`.0`.  `None` for a
    /// block the DFS never reached.
    intervals: Vec<Option<(u32, u32)>>,
}

impl DominatorIntervals {
    /// Number `ssa`'s dominator forest.
    ///
    /// Children are derived from `idom` rather than read off
    /// [`SsaFunction::dominator_tree`], so the index is correct for any
    /// function whose `idom` is populated (including hand-built test
    /// fixtures that never ran the tree builder).
    ///
    /// The DFS starts at the entry block and then picks up any other root
    /// (a block whose `idom` is `None`), so a forest with more than one root
    /// is numbered in full rather than silently losing a component.
    #[must_use]
    fn build(ssa: &SsaFunction) -> Self {
        let slots = ssa
            .idom
            .iter()
            .flat_map(|(id, parent)| [Some(*id), *parent])
            .flatten()
            .map(|b| b.0 as usize + 1)
            .chain(std::iter::once(ssa.block_names.len()))
            .max()
            .unwrap_or(0);
        let mut intervals = vec![None; slots];
        if ssa.idom.is_empty() {
            return Self { intervals };
        }
        let mut children: Vec<Vec<BlockId>> = vec![Vec::new(); slots];
        let mut roots: Vec<BlockId> = Vec::new();
        for (id, parent) in &ssa.idom {
            match parent {
                Some(p) => children[p.0 as usize].push(*id),
                None if *id != ssa.entry => roots.push(*id),
                None => {}
            }
        }
        for kids in &mut children {
            kids.sort_unstable();
        }
        roots.sort_unstable();
        if ssa.idom.contains_key(&ssa.entry) {
            roots.insert(0, ssa.entry);
        }

        let mut counter: u32 = 0;
        // Explicit stack of `(block, next child index)` — an iterative
        // pre-order DFS, so a deep dominator chain cannot blow the stack.
        let mut stack: Vec<(BlockId, usize)> = Vec::new();
        for root in roots {
            if intervals[root.0 as usize].is_some() {
                continue;
            }
            intervals[root.0 as usize] = Some((counter, counter));
            counter += 1;
            stack.push((root, 0));
            while let Some((node, child_idx)) = stack.pop() {
                let kids = &children[node.0 as usize];
                if child_idx < kids.len() {
                    stack.push((node, child_idx + 1));
                    let child = kids[child_idx];
                    if intervals[child.0 as usize].is_none() {
                        intervals[child.0 as usize] = Some((counter, counter));
                        counter += 1;
                        stack.push((child, 0));
                    }
                } else if let Some((_, exit)) = intervals[node.0 as usize].as_mut() {
                    *exit = counter;
                }
            }
        }
        Self { intervals }
    }

    /// True when `ancestor` dominates `node` (a block dominates itself).
    #[must_use]
    pub fn dominates(&self, ancestor: BlockId, node: BlockId) -> bool {
        if ancestor == node {
            return true;
        }
        let (Some(Some((a_enter, a_exit))), Some(Some((n_enter, n_exit)))) = (
            self.intervals.get(ancestor.0 as usize),
            self.intervals.get(node.0 as usize),
        ) else {
            return false;
        };
        *a_enter <= *n_enter && *n_exit <= *a_exit
    }
}

/// CFG block ceiling above which deep analysis (SSA / dataflow) is skipped.
///
/// A pathologically large body — almost always machine-generated, e.g. a
/// tens-of-thousands-of-block nested-if dispatch tree — would cost seconds of
/// SSA + SCCP / type / taint / liveness dataflow for near-zero useful
/// findings, and an unbounded analysis also lets interprocedural summaries
/// grow over-optimistic on adversarial input.
pub const COMPLEXITY_GUARD_BLOCKS: usize = 20_000;

/// Body-size (bytes) ceiling for the deep-analysis complexity guard. A flat
/// generated command list is block-light — so [`COMPLEXITY_GUARD_BLOCKS`]
/// never fires — yet byte-huge, and the O(blocks·vars) SSA walk plus
/// SCCP / taint / liveness still costs seconds. The ceiling is 256 KiB.
pub const DEEP_ANALYSIS_BODY_BYTES: usize = 262_144;

/// True when `func` is large enough that deep analysis (SSA / dataflow) is
/// skipped. This is the block-count half of the guard; the body-byte half is
/// applied by the compilation-unit builder, which has the body span.
#[must_use]
pub fn is_complexity_guarded(func: &cfg::Function) -> bool {
    func.blocks.len() > COMPLEXITY_GUARD_BLOCKS
}

// Variable definition extraction

/// Extract variable names defined by an IR statement.
///
/// Handles assignments (`set`, `incr`), call defs, `trace add variable`
/// (via registry roles when `registry` is supplied), and `dict for`/
/// `dict map` barriers.
///
/// Pass `Some(&CommandRegistry)` when available so barrier defs route
/// through the registry's `ArgRole::VarWrite` query; pass
/// `None` for the string-match fallback used by the unit-test
/// helpers.
#[must_use]
pub fn defs_of(stmt: &Statement) -> Vec<String> {
    if !stmt.is_executable_invocation() {
        return Vec::new();
    }
    defs_of_with_registry(stmt, None)
}

/// Whether a barrier's command is an ensemble loop subcommand whose first
/// argument is the iteration-variable list (`::tcl::dict::for` / `::map`,
/// `::tcl::array::for`). Resolved from the registry's `loop_list_header`
/// flag on the subcommand: the barrier name's last segment is the
/// subcommand, the one before it the base command (`… ::tcl::dict::for` →
/// `dict for`). Callers without a registry (test helpers) fall back to the
/// suffix heuristic, mirroring the trace fallback below.
fn barrier_is_loop_list_header(command: &str, registry: Option<&CommandRegistry>) -> bool {
    let Some(registry) = registry else {
        return command.ends_with("::for") || command.ends_with("::map");
    };
    let segments = crate::naming::qualifier_segments(command.as_bytes());
    let [.., base, sub] = segments.as_slice() else {
        return false;
    };
    let (Ok(base), Ok(sub)) = (std::str::from_utf8(base), std::str::from_utf8(sub)) else {
        return false;
    };
    registry
        .get(base)
        .and_then(|spec| spec.resolve_subcommand(sub))
        .is_some_and(|sub| sub.loop_list_header)
}

/// Whether an IR assignment target `name` (as written) is a *dynamic* write
/// target — its variable name is computed at runtime from a substitution
/// (`set $p …`, `set ${tok} …`, `set a$b(k) …`).  Such a target is opaque
/// (no static def) and *reads* the substituted name-bearing variable(s).
///
/// A leading `$`
/// marks a substituted write-target name, and a substitution in the array
/// *base* (`a$b(k)`) is dynamic too.  A bare array element (`arr($i)`) keeps
/// its static base `arr` (only the index is dynamic), so it is **not** a
/// dynamic target here.
///
/// `braced_literal` is the statement's own `name_braced` flag: a brace-quoted
/// word (`set {$n} 1`) substitutes nothing, so its `$` is part of a perfectly
/// static name and the target is **not** dynamic.  Reading the name text
/// alone calls it dynamic, withholds the def, and records a read of `n` — a
/// `W210 Variable 'n' is read before it is set` on code that never mentions
/// `n`.
#[must_use]
pub fn is_dynamic_write_target(name: &str, braced_literal: bool) -> bool {
    if braced_literal {
        return false;
    }
    if name.starts_with('$') {
        return true;
    }
    let base = match name.find('(') {
        Some(i) => &name[..i],
        None => name,
    };
    base.is_empty() || base.contains('$') || base.contains('[')
}

/// Whether a loop-variable-list word is **static** — its source performs no
/// substitution, so the word's text *is* the Tcl list of names the loop binds.
///
/// Dynamism is a property of the source word's *kind*, not of the bytes in its
/// value — the same rule the neighbouring `VarWrite` path applies via
/// [`CommandTokens::arg_is_braced_literal`].  A brace-quoted word substitutes
/// nothing, so `foreach {{$x}} {*}$spec {puts ${$x}}` binds the perfectly legal
/// variable named `$x`; testing the reconstructed word text for `$` / `[`
/// drops that name from the barrier's def set and reports a phantom
/// `W210 Variable '$x' is read before it is set` on code `tclsh` runs happily.
/// A word that really does substitute
/// (`foreach $names …`, `foreach [names] …`, `foreach {*}$spec …`) names
/// nothing statically and contributes no def.
///
/// `text` is only consulted when the statement carries no structured word for
/// this argument — a synthetic or lossy token snapshot, where the kind is
/// unknown and the conservative text test is the best available answer.
fn loop_var_list_word_is_static(word: Option<&WordExpr>, text: &str) -> bool {
    match word {
        Some(WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. }) => true,
        Some(
            WordExpr::Variable { .. }
            | WordExpr::CommandSubstitution { .. }
            | WordExpr::Expand { .. },
        ) => false,
        // A compound word is static exactly when every part is text: a quoted
        // (`"a b"`) or backslash-escaped (`a\ b`) var-list substitutes nothing,
        // while any `$` / `[` part makes the whole word dynamic.
        Some(WordExpr::Template { parts, .. }) => parts
            .iter()
            .all(|part| matches!(part, WordPart::Text { .. })),
        Some(WordExpr::Opaque { .. }) | None => !text.contains('$') && !text.contains('['),
    }
}

/// The defs a `Statement::Barrier` contributes under the registry — its
/// loop-variable lists plus its `ArgRole::VarWrite` targets.
///
/// Skips *scope-alias* commands (`global`, `variable`, `upvar`) whose variable
/// bindings are tracked separately by the `var_scoping` pass; without the skip
/// we'd produce partial defs for the vararg forms (`global x y z` would mark
/// only `x`).  The discriminator is `CREATES_SCOPE_ALIAS`, *not*
/// `CREATES_DYNAMIC_BARRIER`: `trace` is a dynamic barrier but not a scope
/// alias, so `trace add variable x` must still surface its `VarWrite` def.
fn registry_barrier_defs(
    reg: &CommandRegistry,
    command: &str,
    args: &[String],
    tokens: Option<&CommandTokens>,
) -> Vec<String> {
    use tcl_registry::{ArgRole, Traits};

    let spec = reg.get(command);
    if spec.is_some_and(|spec| spec.traits.contains(Traits::CREATES_SCOPE_ALIAS)) {
        return Vec::new();
    }
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    // Loop variables (`ArgRole::LoopVarList` — `foreach` / `lmap` var-lists,
    // `dict for` / `array for` key-value pairs).  A structured loop that stays
    // a barrier — `{*}` expansion among its words, a dynamic body — still
    // binds every variable its *literal* var-list words name, and its body is
    // scanned for reads in this same frame, so without these defs a body read
    // of a loop variable reports W210 "read before it is set".
    // A dynamic var-list word (`foreach {*}$pairs …`) names nothing statically
    // and contributes no def — see [`loop_var_list_word_is_static`] for how
    // that verdict is reached.
    //
    // A **conditionally-bound** layout is excluded: `dict update dictVar key
    // varName … body` binds `varName` only when the key is present at runtime,
    // so it is not a definite def and the key-aware read-before-set harvester
    // owns it instead (`RepeatedArgLayout::conditional_binding`).
    let loop_vars_conditional = |repeated: &[tcl_registry::RepeatedArgLayout]| {
        repeated
            .iter()
            .any(|layout| layout.conditional_binding && layout.role == ArgRole::LoopVarList)
    };
    let conditional_loop_vars = spec.is_some_and(|spec| {
        args.first()
            .and_then(|first| spec.resolve_subcommand(first))
            .map_or_else(
                || loop_vars_conditional(spec.repeated_args),
                |sub| loop_vars_conditional(sub.repeated_args),
            )
    });
    let mut defs: Vec<String> = if conditional_loop_vars {
        Vec::new()
    } else {
        reg.arg_indices_for_role(command, &arg_strs, ArgRole::LoopVarList)
            .into_iter()
            .filter_map(|idx| args.get(idx).map(|word| (idx, word)))
            .filter(|(idx, word)| {
                loop_var_list_word_is_static(tokens.and_then(|t| t.words().get(idx + 1)), word)
            })
            .filter_map(|(_, word)| {
                tcl_syntax::word_rules::WordValueRules::of_profile(reg.profile())
                    .split_list(word)
                    .ok()
            })
            .flatten()
            .map(std::borrow::Cow::into_owned)
            .filter(|name| !name.is_empty())
            .collect()
    };
    defs.extend(
        reg.arg_indices_for_role(command, &arg_strs, ArgRole::VarWrite)
            .into_iter()
            .filter_map(|idx| {
                // A brace-quoted name word is a literal name, `$` and all.
                let braced = tokens.is_some_and(|t| t.arg_is_braced_literal(idx));
                args.get(idx).map(|s| {
                    let name = crate::naming::element_var_name_braced(s, braced);
                    if reg.option_variable_scope(command, &arg_strs, idx, reg.own_surface_query())
                        == Some(tcl_registry::VariableScope::Global)
                        && !name.starts_with("::")
                    {
                        format!("::{name}")
                    } else {
                        name.to_owned()
                    }
                })
            })
            .filter(|n| !n.is_empty()),
    );
    defs
}

fn selected_barrier_defs(
    selection: SsaInvocationContext<'_>,
    tokens: &CommandTokens,
) -> Vec<String> {
    let Some(crate::registry_invocation::RegistryInvocationResolution::Resolved(facts)) =
        selection.resolve(tokens)
    else {
        return Vec::new();
    };
    if facts
        .traits
        .contains(tcl_registry::Traits::CREATES_SCOPE_ALIAS)
        || !facts.arg_roles_complete
    {
        return Vec::new();
    }
    let Some(effective) = crate::registry_invocation::effective_command_words(tokens) else {
        return Vec::new();
    };
    let Some(config) = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.original_lexer_config_for_tokens(tokens))
    else {
        return Vec::new();
    };
    let conditional = facts.repeated_args.iter().any(|layout| {
        layout.conditional_binding && layout.role == tcl_registry::ArgRole::LoopVarList
    });
    let mut defs = selection
        .normal(tokens)
        .map_or_else(Vec::new, |invocation| invocation.definition_names());
    if !conditional {
        for &(index, role) in &facts.arg_roles {
            if role != tcl_registry::ArgRole::LoopVarList {
                continue;
            }
            let Some(word) = effective
                .words
                .get(facts.argument_offset + usize::from(index) + 1)
            else {
                continue;
            };
            let Some(text) = crate::registry_invocation::invocation_word(word).literal() else {
                continue;
            };
            if let Ok(names) =
                tcl_syntax::word_rules::WordValueRules::from_config(&config).split_list(text)
            {
                defs.extend(
                    names
                        .into_iter()
                        .map(std::borrow::Cow::into_owned)
                        .filter(|name| !name.is_empty()),
                );
            }
        }
    }
    defs
}

/// Explicit standalone registry-aware `defs_of`.
///
/// Barrier defs route through
/// `ArgRole::VarWrite` instead of a hardcoded string-match.  The
/// registry-aware path also covers `::trace` and any future trace
/// alias spellings without code edits, plus skips
/// `creates_dynamic_barrier` specs (`global` / `variable` /
/// `upvar` are handled by `var_scoping`, not by SSA's per-arg
/// `VarWrite` walk).
#[must_use]
pub fn defs_of_with_registry(stmt: &Statement, registry: Option<&CommandRegistry>) -> Vec<String> {
    defs_of_in_context(stmt, registry.map(SsaInvocationContext::standalone))
}

fn defs_of_in_context(
    stmt: &Statement,
    selection: Option<SsaInvocationContext<'_>>,
) -> Vec<String> {
    let registry = selection.map(|selection| selection.registry);
    if !stmt.is_executable_invocation() || stmt.has_opaque_native_accesses() {
        return Vec::new(); // Unknown writes are clobbers, never definite named values.
    }
    match stmt {
        Statement::AssignConst {
            name, name_braced, ..
        }
        | Statement::AssignExpr {
            name, name_braced, ..
        }
        | Statement::AssignValue {
            name, name_braced, ..
        }
        | Statement::Incr {
            name, name_braced, ..
        } => {
            // A write-target whose *name* is value-substituted (`set $p …`,
            // `set ${tok}(k) …`) denotes the variable named by the
            // substitution's value — a place that cannot be pinned down, so it
            // is **not** a static def of the name-bearing variable.  `uses_of`
            // records the name read separately.  A brace-quoted target
            // (`set {$n} 1`) substitutes nothing — it is a static def of the
            // variable literally named `$n`.
            if is_dynamic_write_target(name, *name_braced) {
                return Vec::new();
            }
            // A constant-keyed array element defs its own variable
            // (`arr(k)`); a dynamic key stays on the base (the rename walk
            // fans the def over the array's known elements).
            vec![crate::naming::element_var_name_braced(name, *name_braced).to_owned()]
        }
        Statement::Call { defs, .. }
            if !defs.is_empty() && selection.is_none_or(|s| s.standalone) =>
        {
            defs.clone()
        }
        Statement::Call {
            tokens: Some(tokens),
            ..
        } => selection
            .and_then(|selection| selection.normal(tokens))
            .map_or_else(Vec::new, |invocation| {
                invocation
                    .definition_names()
                    .into_iter()
                    .map(|name| crate::naming::element_var_name_braced(&name, true).to_owned())
                    .collect()
            }),
        Statement::Barrier {
            command,
            args,
            tokens,
            ..
        } => {
            if let Some(selection) = selection.filter(|selection| !selection.standalone) {
                return tokens
                    .as_ref()
                    .map_or_else(Vec::new, |tokens| selected_barrier_defs(selection, tokens));
            }
            // Loop-header barriers (`::tcl::dict::for`/`::map`, `::tcl::array::for`
            // — any ensemble subcommand the registry marks `loop_list_header`):
            // args[0] is the iteration-variable list, so extract the names.
            // Resolved via the registry rather than a name-suffix match — a
            // user proc named `my::for` must NOT have its first argument
            // misread as loop variables.
            if !args.is_empty() && barrier_is_loop_list_header(command, registry) {
                // A loop var-list is a Tcl list, not whitespace-separated
                // source text.  Decode grouping and backslash substitutions so
                // `{one name}`, `"two name"`, and `three\ name` each define
                // one variable.  A malformed var-list makes the command fail
                // before its body runs, so it contributes no reaching defs.
                return tcl_syntax::word_rules::WordValueRules::of_profile(
                    registry.and_then(tcl_registry::CommandRegistry::profile),
                )
                .split_list(&args[0])
                .map(|names| {
                    names
                        .into_iter()
                        .map(std::borrow::Cow::into_owned)
                        .collect()
                })
                .unwrap_or_default();
            }
            if let Some(reg) = registry {
                let defs = registry_barrier_defs(reg, command, args, tokens.as_ref());
                if !defs.is_empty() {
                    return defs;
                }
            }
            // String-match fallback for callers without a registry
            // (test helpers).
            if command == "trace" && args.len() >= 3 && args[0] == "add" && args[1] == "variable" {
                return vec![crate::naming::element_var_name(&args[2]).to_owned()];
            }
            Vec::new()
        }
        // An opaque (glob/regexp/fall-through) `switch` definitely-defines a
        // variable only when *every reaching* path assigns it: there must be a
        // `default` arm (covering the no-match path) and the variable must be
        // *must-defined* in the default body and in every arm that has a body
        // (fall-through arms with no body delegate to a later body, already
        // covered). An arm that *cannot complete normally* (always
        // `return`s/`error`s/`tailcall`s/…) never reaches the code after the
        // switch, so it is excluded from the intersection (FP-RBS-14). This
        // reproduces the phi the expanded arm blocks would build, conservatively
        // — a variable only *conditionally* assigned inside an arm is not
        // claimed, so we never hide a genuine read-before-set. The expanded
        // (exact, non-fall-through) switch never reaches here; its arm defs come
        // from the real per-block statements. Shares the "flow facts"
        // definite-assignment helpers with the CFG builder (`cfg_builder`), so
        // both layers agree on "cannot complete normally".
        Statement::Switch {
            default_body: Some(_),
            ..
        } => match selection {
            Some(selection) if !selection.standalone => {
                crate::cfg_builder::switch_must_defines_with_source_input(
                    stmt,
                    selection.registry,
                    selection
                        .metadata
                        .and_then(|metadata| metadata.source_analysis_input()),
                )
                .into_iter()
                .collect()
            }
            _ => crate::cfg_builder::switch_must_defines(stmt, registry)
                .into_iter()
                .collect(),
        },
        _ => Vec::new(),
    }
}

// Dominator algorithms

/// Compute the dominator sets for all blocks in a CFG function.
///
/// Uses the iterative dataflow algorithm. Returns a map from block
/// name to the set of blocks that dominate it.
///
/// The fixpoint visits blocks in **reverse postorder** so that each
/// block is processed after the predecessors it depends on, which is
/// what makes the iteration converge in a small constant number of
/// passes for a reducible CFG instead of one pass per block.  Driving
/// the fixpoint off the reachable-block *set* (arbitrary hash order)
/// instead made convergence O(blocks) passes — pathological on a proc
/// body that lowers to a long chain of branches (e.g. a 700-way
/// `if {$x == N} {...}` dispatch), where it turned an O(N²) job into
/// O(N³) and stalled the analyser.
#[must_use]
pub fn compute_dominators(func: &cfg::Function) -> HashMap<BlockId, HashSet<BlockId>> {
    let reachable = func.reachable_blocks();
    let mut dom: HashMap<BlockId, HashSet<BlockId>> = HashMap::new();

    for id in func.blocks.keys() {
        if !reachable.contains(id) || *id == func.entry {
            dom.insert(*id, HashSet::from([*id]));
        } else {
            dom.insert(*id, reachable.clone());
        }
    }

    // Reverse postorder over blocks reachable from the entry — this is
    // exactly the reachable set, ordered so predecessors precede the
    // blocks that depend on them.
    let rpo = func.reverse_postorder();
    let preds = func.predecessors();
    let mut changed = true;
    while changed {
        changed = false;
        for id in &rpo {
            if *id == func.entry {
                continue;
            }
            let bn_preds: Vec<BlockId> = preds
                .get(id)
                .map(|p| {
                    p.iter()
                        .copied()
                        .filter(|p| reachable.contains(p))
                        .collect()
                })
                .unwrap_or_default();

            let new_dom = if bn_preds.is_empty() {
                HashSet::from([*id])
            } else {
                let mut inter = dom[&bn_preds[0]].clone();
                for p in &bn_preds[1..] {
                    inter = inter.intersection(&dom[p]).copied().collect();
                }
                inter.insert(*id);
                inter
            };

            if new_dom != dom[id] {
                dom.insert(*id, new_dom);
                changed = true;
            }
        }
    }
    dom
}

/// Compute immediate dominators directly via the Cooper-Harvey-Kennedy
/// "A Simple, Fast Dominance Algorithm".
///
/// Returns the same map shape as [`compute_idom`] (entry and
/// unreachable blocks map to `None`, every other reachable block to
/// `Some(parent)`), but **without** materialising the full dominator
/// *sets*: it works on reverse-postorder block indices and a single
/// `idom` pointer per block, so it is O(N·D) time and O(N) memory
/// rather than the O(N²) memory / O(N³) worst-case time of the
/// set-based [`compute_dominators`] + [`compute_idom`] pair.  This is
/// what keeps `build_ssa` bounded on pathologically large functions
/// (a single multi-thousand-branch generated proc would otherwise
/// exhaust memory building the dominator sets).
#[must_use]
pub(crate) fn compute_idom_fast(func: &cfg::Function) -> HashMap<BlockId, Option<BlockId>> {
    const UNDEF: usize = usize::MAX;

    // Shared iterative RPO — see `cfg::Function::reverse_postorder`.
    let rpo = func.reverse_postorder();
    let mut out: HashMap<BlockId, Option<BlockId>> = HashMap::new();
    for id in func.blocks.keys() {
        out.insert(*id, None);
    }
    if rpo.is_empty() {
        return out;
    }
    // Map block id → reverse-postorder index (entry == 0).
    let mut rpo_index: FxHashMap<BlockId, usize> =
        FxHashMap::with_capacity_and_hasher(rpo.len(), FxBuildHasher);
    for (i, n) in rpo.iter().enumerate() {
        rpo_index.insert(*n, i);
    }
    let preds = func.predecessors();

    let mut idom: Vec<usize> = vec![UNDEF; rpo.len()];
    idom[0] = 0; // entry is its own dominator (sentinel for the walk).

    // Walk up the idom tree from both nodes until they meet, using
    // RPO indices (a dominator always has a strictly smaller index).
    let intersect = |mut a: usize, mut b: usize, idom: &[usize]| -> usize {
        while a != b {
            while a > b {
                a = idom[a];
            }
            while b > a {
                b = idom[b];
            }
        }
        a
    };

    let mut changed = true;
    while changed {
        changed = false;
        // Skip the entry (index 0); process the rest in RPO order so
        // each block sees already-processed predecessors.
        for i in 1..rpo.len() {
            let mut new_idom = UNDEF;
            if let Some(ps) = preds.get(&rpo[i]) {
                for p in ps {
                    let Some(&pi) = rpo_index.get(p) else {
                        continue; // unreachable predecessor
                    };
                    if idom[pi] == UNDEF {
                        continue; // not processed yet this pass
                    }
                    new_idom = if new_idom == UNDEF {
                        pi
                    } else {
                        intersect(pi, new_idom, &idom)
                    };
                }
            }
            if new_idom != UNDEF && idom[i] != new_idom {
                idom[i] = new_idom;
                changed = true;
            }
        }
    }

    for (i, id) in rpo.iter().enumerate() {
        if i == 0 || idom[i] == UNDEF {
            out.insert(*id, None);
        } else {
            out.insert(*id, Some(rpo[idom[i]]));
        }
    }
    out
}

/// Compute immediate dominators from dominator sets.
///
/// The immediate dominator of a block is the closest strict dominator
/// (the one with the largest dominator set).
///
/// Retained as the reference set-based implementation that
/// [`compute_idom_fast`] (the production path) is cross-validated
/// against; see the `compute_idom_fast_matches_reference` test. Only
/// the fast path is used in production, so this is compiled for tests.
#[cfg(test)]
#[must_use]
pub(crate) fn compute_idom(
    func: &cfg::Function,
    dom: &HashMap<BlockId, HashSet<BlockId>>,
) -> HashMap<BlockId, Option<BlockId>> {
    let reachable = func.reachable_blocks();
    let mut idom: HashMap<BlockId, Option<BlockId>> = HashMap::new();

    for id in func.blocks.keys() {
        idom.insert(*id, None);
    }

    for id in &reachable {
        if *id == func.entry {
            continue;
        }
        let strict: HashSet<BlockId> = dom[id].iter().copied().filter(|d| d != id).collect();
        if strict.is_empty() {
            continue;
        }
        // The idom is the strict dominator with the largest dom set.
        let best = *strict.iter().max_by_key(|d| dom[*d].len()).unwrap();
        idom.insert(*id, Some(best));
    }
    idom
}

/// Compute the dominance frontier for each block.
///
/// A block `b` is in the dominance frontier of block `a` if `a`
/// dominates a predecessor of `b` but does not strictly dominate `b`.
#[must_use]
pub(crate) fn compute_dominance_frontier(
    func: &cfg::Function,
    idom: &HashMap<BlockId, Option<BlockId>>,
) -> HashMap<BlockId, HashSet<BlockId>> {
    let reachable = func.reachable_blocks();
    let preds = func.predecessors();
    let mut df: HashMap<BlockId, HashSet<BlockId>> = HashMap::new();

    for id in func.blocks.keys() {
        df.insert(*id, HashSet::new());
    }

    for id in &reachable {
        let bn_preds: Vec<BlockId> = preds
            .get(id)
            .map(|p| {
                p.iter()
                    .copied()
                    .filter(|p| reachable.contains(p))
                    .collect()
            })
            .unwrap_or_default();

        if bn_preds.len() < 2 {
            continue;
        }

        for p in &bn_preds {
            let mut runner = Some(*p);
            while let Some(r) = runner {
                if idom.get(id).and_then(|i| i.as_ref()) == Some(&r) {
                    break;
                }
                df.entry(r).or_default().insert(*id);
                runner = idom.get(&r).copied().flatten();
            }
        }
    }
    df
}

/// Build the dominator tree from immediate dominators.
///
/// Returns a map from each block to its children in the dominator tree.
/// Children are sorted by [`BlockId`] (block-creation order) for
/// deterministic traversal.
#[must_use]
pub(crate) fn build_dom_tree(
    idom: &HashMap<BlockId, Option<BlockId>>,
) -> HashMap<BlockId, Vec<BlockId>> {
    let mut tree: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
    for id in idom.keys() {
        tree.entry(*id).or_default();
    }
    for (id, parent) in idom {
        if let Some(p) = parent {
            tree.entry(*p).or_default().push(*id);
        }
    }
    for children in tree.values_mut() {
        children.sort_unstable();
    }
    tree
}

/// Compute which variables need phi nodes in each block.
///
/// Uses the iterated dominance frontier algorithm: for each variable,
/// starting from blocks where it is defined, propagate phi nodes to
/// the dominance frontier until convergence.
#[must_use]
#[cfg(test)]
pub(crate) fn compute_phi_vars(
    func: &cfg::Function,
    df: &HashMap<BlockId, HashSet<BlockId>>,
    registry: &CommandRegistry,
    elems: &ArrayElems,
) -> HashMap<BlockId, HashSet<VariableCellKey>> {
    compute_phi_vars_with_config(
        func,
        df,
        SsaInvocationContext::standalone(registry),
        elems,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
        &build_point_resolve_contexts(func, &func.name, registry),
        &HashMap::new(),
    )
}

/// [`compute_phi_vars`] under the exact lexical policy that produced `func`.
fn compute_phi_vars_with_config(
    func: &cfg::Function,
    df: &HashMap<BlockId, HashSet<BlockId>>,
    selection: SsaInvocationContext<'_>,
    elems: &ArrayElems,
    config: tcl_lexer::LexerConfig,
    points: &PointResolveContexts,
    clobbers: &RegistryClobberNames,
) -> HashMap<BlockId, HashSet<VariableCellKey>> {
    let reachable = func.reachable_blocks();
    let (nonlocal_names, mut all_defsites) =
        nonlocal_names_and_defsites(func, &reachable, selection, elems, config, points);
    for (block, markers) in clobbers {
        for name in markers.values().flatten() {
            all_defsites.entry(name.clone()).or_default().insert(*block);
        }
    }

    // Semi-pruned SSA (Briggs et al. 1998): place phis only for *non-local*
    // (upward-exposed-use) names. A phi for a purely-local name has no reader,
    // so dropping it removes only dead phis (~40% of minimal-SSA phis) without
    // changing any use/value/liveness/diagnostic result.
    let mut phi: HashMap<BlockId, HashSet<VariableCellKey>> = HashMap::new();
    for id in func.blocks.keys() {
        phi.insert(*id, HashSet::new());
    }

    for (var, sites) in &all_defsites {
        if !nonlocal_names.contains(var) {
            continue;
        }
        let mut work: Vec<BlockId> = sites.iter().copied().collect();
        work.sort_unstable();
        let mut has_phi: FxHashSet<BlockId> = FxHashSet::default();

        while let Some(nb) = work.pop() {
            for fb in df.get(&nb).into_iter().flatten() {
                if has_phi.insert(*fb) {
                    phi.entry(*fb).or_default().insert(var.clone());
                    if !sites.contains(fb) {
                        work.push(*fb);
                    }
                }
            }
        }
    }
    phi
}

/// Semi-pruned SSA support: the *non-local* names (upward-exposed uses) and
/// every variable's def-site blocks, in one pass.
///
/// A variable is *non-local* if some block reads it before (re)defining it in
/// that block — the read could observe a value flowing in from a predecessor,
/// so a phi at a merge is meaningful. A name only ever read after its in-block
/// definition (or never read) needs no phi. `defsites` is unfiltered (all
/// defined names); the caller restricts phi placement to the non-local names.
/// Physical relationships between represented literal elements and their roots.
/// These relationships come from bound places, never from parsing identity keys.
#[derive(Default)]
pub(crate) struct ArrayElems {
    elements: FxHashMap<VariableCellKey, HashSet<VariableCellKey>>,
    element_roots: FxHashMap<VariableCellKey, VariableCellKey>,
}

impl ArrayElems {
    fn note(&mut self, mut target: crate::place::Place) {
        if target.kind != crate::place::PlaceKind::ArrayElem
            || !target
                .index
                .as_ref()
                .is_some_and(|index| index.kind == crate::place::IndexKind::Literal)
        {
            return;
        }
        let Some(element) = crate::var_resolve::canonical_binding_value_key(&target) else {
            return;
        };
        target.index = None;
        target.kind = crate::place::PlaceKind::ArrayWhole;
        let Some(root) = crate::var_resolve::canonical_binding_value_key(&target) else {
            return;
        };
        self.element_roots.insert(element.clone(), root.clone());
        self.elements.entry(root).or_default().insert(element);
    }

    fn get(&self, root: &VariableCellKey) -> Option<&HashSet<VariableCellKey>> {
        self.elements.get(root)
    }

    fn root_for_element(&self, element: &VariableCellKey) -> Option<&VariableCellKey> {
        self.element_roots.get(element)
    }
}

/// Collect exact physical array elements at their own read/store boundaries.
fn collect_array_elems(
    func: &cfg::Function,
    registry: &CommandRegistry,
    points: &PointResolveContexts,
) -> ArrayElems {
    let mut elems = ArrayElems::default();
    for (&id, block) in &func.blocks {
        for (index, statement) in block.statements.iter().enumerate() {
            for target in crate::place_bridge::def_places_at(statement, id, index, points, registry)
                .into_iter()
                .chain(crate::place_bridge::read_places_at(
                    statement, id, index, points, registry,
                ))
            {
                elems.note(target);
            }
        }
        if let Some(terminator) = &block.terminator {
            for target in
                crate::place_bridge::terminator_read_places_at(terminator, id, points, registry)
            {
                elems.note(target);
            }
        }
    }
    elems
}

/// Expand a statement's direct def names with the array-element fan-out:
///
/// - an element def (`arr(k)`) also defs its base `arr`, so whole-array
///   reads (`array get arr`) see a fresh version;
/// - a base def where constant-keyed elements exist (a dynamic-key write
///   `set arr($i) v`, or a whole-array writer like `array set`) **fans**
///   over every known element — the write may have hit any of them.
///
/// The rename walk records a *use* of each fanned-only name's prior
/// version, so type inference joins the old element type with the written
/// value instead of trusting either.
fn expand_defs(direct: &[VariableCellKey], elems: &ArrayElems) -> Vec<VariableCellKey> {
    let mut out: Vec<VariableCellKey> = Vec::new();
    let push = |n: VariableCellKey, out: &mut Vec<VariableCellKey>| {
        if !out.contains(&n) {
            out.push(n);
        }
    };
    for d in direct {
        if let Some(root) = elems.root_for_element(d) {
            push(d.clone(), &mut out);
            push(root.to_owned(), &mut out);
        } else {
            push(d.clone(), &mut out);
            if let Some(els) = elems.get(d) {
                for e in els {
                    push(e.clone(), &mut out);
                }
            }
        }
    }
    out
}

/// The use-side counterpart of [`expand_defs`]: reading a base whose
/// constant-keyed elements are known (`$a($i)`, `array get a`, `parray a`)
/// reads *every* element, and a dynamic-key / whole-array write reads each
/// fanned element's prior version (the may-def join input). Both make the
/// element chains live and upward-exposed so phi placement and liveness see
/// them.
fn expand_uses(
    direct_uses: &[VariableCellKey],
    direct_defs: &[VariableCellKey],
    elems: &ArrayElems,
) -> Vec<VariableCellKey> {
    let mut out: Vec<VariableCellKey> = Vec::new();
    let push = |n: &VariableCellKey, out: &mut Vec<VariableCellKey>| {
        if !out.iter().any(|e| e == n) {
            out.push(n.to_owned());
        }
    };
    for u in direct_uses {
        if let Some(els) = elems.get(u) {
            for e in els {
                push(e, &mut out);
            }
        }
    }
    for d in direct_defs {
        if elems.root_for_element(d).is_none()
            && let Some(els) = elems.get(d)
        {
            for e in els {
                push(e, &mut out);
            }
        }
    }
    out
}

fn nonlocal_names_and_defsites(
    func: &cfg::Function,
    reachable: &HashSet<BlockId>,
    selection: SsaInvocationContext<'_>,
    elems: &ArrayElems,
    config: tcl_lexer::LexerConfig,
    points: &PointResolveContexts,
) -> (
    FxHashSet<VariableCellKey>,
    FxHashMap<VariableCellKey, FxHashSet<BlockId>>,
) {
    let registry = selection.registry;
    let mut scanner = VarReferenceScanner::with_config(
        VarScanOptions {
            include_var_read_roles: selection.standalone,
            recurse_cmd_substitutions: true,
            include_reads_before_write: false,
            element_qualified: true,
        },
        config,
    );
    let mut nonlocal_names: FxHashSet<VariableCellKey> = FxHashSet::default();
    let mut defsites: FxHashMap<VariableCellKey, FxHashSet<BlockId>> = FxHashMap::default();

    for bn in reachable {
        let Some(block) = func.blocks.get(bn) else {
            continue;
        };
        let mut defined_here: FxHashSet<VariableCellKey> = FxHashSet::default();
        for (index, stmt) in block.statements.iter().enumerate() {
            let context = points.before_statement(*bn, index);
            let mut direct_uses = bound_names(
                uses_of_classified_in_context(stmt, &mut scanner, selection)
                    .into_iter()
                    .map(|(name, _)| name)
                    .collect(),
                context,
                registry,
            );
            direct_uses.extend(
                source_read_bindings(points, *bn, index, registry)
                    .into_iter()
                    .map(|(_, bound)| bound),
            );
            direct_uses.extend(invocation_read_bindings(
                points, *bn, index, context, registry,
            ));
            let direct_defs = bound_defs(stmt, *bn, index, points, selection);
            for u in
                direct_uses
                    .iter()
                    .cloned()
                    .chain(expand_uses(&direct_uses, &direct_defs, elems))
            {
                if !defined_here.contains(&u) {
                    nonlocal_names.insert(u);
                }
            }
            for var in expand_defs(&direct_defs, elems) {
                defsites.entry(var.clone()).or_default().insert(*bn);
                defined_here.insert(var);
            }
        }
        // Terminator reads are element-qualified like statement reads (a
        // condition's `$a(k)` must place the element's phi), and a base
        // read fans over known elements.
        let mut term_uses: Vec<String> = Vec::new();
        match &block.terminator {
            Some(cfg::Terminator::Branch { condition, .. }) => {
                term_uses
                    .extend(condition.vars_element_qualified_with_config(scanner.lexer_config()));
            }
            Some(cfg::Terminator::Return { value, expr, .. }) => {
                if let Some(v) = value {
                    term_uses.extend(scanner.scan_word(v, registry));
                }
                if let Some(e) = expr {
                    term_uses.extend(e.vars_element_qualified_with_config(scanner.lexer_config()));
                }
            }
            _ => {}
        }
        let mut term_uses = bound_names(term_uses, points.before_terminator(*bn), registry);
        term_uses.extend(
            source_read_bindings(points, *bn, usize::MAX, registry)
                .into_iter()
                .map(|(_, bound)| bound),
        );
        term_uses.extend(invocation_read_bindings(
            points,
            *bn,
            usize::MAX,
            points.before_terminator(*bn),
            registry,
        ));
        for u in term_uses
            .iter()
            .cloned()
            .chain(expand_uses(&term_uses, &[], elems))
        {
            if !defined_here.contains(&u) {
                nonlocal_names.insert(u);
            }
        }
    }
    (nonlocal_names, defsites)
}

// Variable-use extraction
//
// These functions determine which variables an IR statement reads.

/// Return `true` when argument at `arg_index` is a braced literal
/// (single-token STR word).
///
/// When token info is unavailable, returns `false` so unknown
/// arguments are still scanned as ordinary inputs. We only exclude
/// bodies when we can positively identify them as single-token
/// braced literals.
fn is_braced_arg(tokens: Option<&CommandTokens>, arg_index: usize) -> bool {
    let Some(tokens) = tokens else {
        return false;
    };
    // tokens.argv includes the command name at index 0; args are 1-based.
    let tok_index = arg_index + 1;
    if tok_index >= tokens.single_token_word.len() {
        return false;
    }
    if !tokens.single_token_word[tok_index] {
        return false;
    }
    // A single-token word from a VAR or CMD token is not braced.
    if let Some(text) = tokens.argv_texts.get(tok_index) {
        !text.starts_with("${") && !text.starts_with('[')
    } else {
        true
    }
}

/// Return BODY arg indices that should be excluded from local statement uses.
///
/// We only exclude handler-style bodies that are lowered/analysed separately.
/// Dynamic evaluation commands like `eval` still need their args treated as
/// ordinary dataflow inputs (for taint and read-before-set tracking).
pub(crate) fn structural_body_indices(
    command: &str,
    args: &[String],
    tokens: Option<&CommandTokens>,
    registry: &CommandRegistry,
) -> HashSet<usize> {
    use tcl_registry::{ArgRole, BodyKind};

    // A foreign-dialect builtin disabled in the active dialect — known in some
    // dialect but absent from the active registry's `by_name` (the analyser
    // loads only the active dialect, so an iRules `when` / `log` / `session`
    // misses under plain Tcl) — is an unknown would-be user command here. Tcl
    // never substitutes inside its braced arguments, so every braced arg is
    // opaque *data*, not analysable script/expr; scanning it would read its
    // `$vars` and emit spurious findings (W210). Skip them. A command
    // unknown in *every* dialect (`get` miss *and* not known-in-any) — a real
    // user proc / TclOO body / recovery artefact — is NOT skipped: its braced
    // body still recurses.
    if registry.get(command).is_none() && registry.known_in_any_dialect(command) {
        return (0..args.len())
            .filter(|&idx| is_braced_arg(tokens, idx))
            .collect();
    }

    // The registry-declared `body_kind` on each spec /
    // subcommand tells us whether body args run in the caller's
    // frame (`Plain`) or in a separate definition / dispatch context
    // (`Structural`).  Only `Structural` body args belong in this
    // skip set — `if`, `while`, `for`, `foreach`, `catch`, `try`,
    // … bodies share the caller's frame and SSA must scan them as
    // part of the enclosing block's data flow.
    // An `ArgRole::LambdaLiteral` word is a whole anonymous procedure —
    // `{params body ?ns?}` — and C Tcl runs it in a **fresh frame**, so no
    // name written inside it is a read (or write) in the frame the call is
    // written in.  tclsh 9.0.4 / 8.6.14, identical:
    //
    //   set x 7; apply {{} {puts $x}}   ;# can't read "x": no such variable
    //
    // The lambda's own body is analysed on its own, in that frame, with its
    // parameter list bound (`lowering::lower_apply` registers it as a body
    // unit) — so scanning the literal here both mis-frames the read and
    // duplicates it, drawing a false `W210` on the lambda's own parameters
    // Unconditional, not gated on `body_kind`: the role is
    // itself the "fresh frame" statement.  A *dynamic* lambda (`apply
    // $lambda`) is not braced, so `is_braced_arg` keeps it a genuine read.
    let lambda_words: HashSet<usize> = {
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        registry
            .arg_indices_for_role(command, &arg_strs, ArgRole::LambdaLiteral)
            .into_iter()
            .filter(|&idx| idx < args.len() && is_braced_arg(tokens, idx))
            .collect()
    };

    if let Some(spec) = registry.get(command) {
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        // Subcommand body_kind (if the call dispatches to a sub).
        let sub_body_kind = if spec.subcommands.is_empty() {
            None
        } else {
            args.first()
                .and_then(|first| spec.resolve_subcommand(first))
                .map(|sub| sub.body_kind)
        };
        let body_kind = sub_body_kind.unwrap_or(spec.body_kind);
        if body_kind == BodyKind::Structural {
            let candidates = registry.arg_indices_for_role(command, &arg_strs, ArgRole::Body);
            return candidates
                .into_iter()
                .filter(|&idx| idx < args.len() && is_braced_arg(tokens, idx))
                .chain(lambda_words)
                .collect();
        }
    }

    lambda_words
}

/// Extract variable names used (read) by an IR statement.
///
/// Uses a [`VarReferenceScanner`] to find variable references in word texts
/// and expression trees.
///
/// Returns sorted variable names, excluding variables that are defined
/// by this statement (unless they exhibit read-before-write semantics).
///
/// Names only, dropping the [`UseClass`] classification — use
/// [`uses_of_classified`] when the caller must distinguish a substituted read
/// from one carried by an unevaluated brace-quoted word.
pub fn uses_of(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    registry: &CommandRegistry,
) -> Vec<String> {
    if !stmt.is_executable_invocation() {
        return Vec::new();
    }
    uses_of_classified(stmt, scanner, registry)
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

/// The classified reads a statement scan accumulates: names substituted (or
/// evaluated in this frame) by the statement, and names mentioned only inside
/// a brace-quoted word it passes through verbatim.
#[derive(Default)]
struct ClassifiedUses {
    substituted: BTreeSet<String>,
    quoted: BTreeSet<String>,
    by_name: BTreeSet<String>,
}

impl ClassifiedUses {
    /// Absorb another scan's two halves, keeping each name in its own bucket.
    /// A name that is substituted *somewhere* stays substituted — the final
    /// `quoted.retain` in [`uses_of_classified`] resolves the overlap once,
    /// at the top, so intermediate merges never have to.
    fn merge(&mut self, other: ClassifiedUses) {
        self.substituted.extend(other.substituted);
        self.quoted.extend(other.quoted);
        self.by_name.extend(other.by_name);
    }

    /// Absorb a classified name list (as [`uses_of_classified`] returns it).
    fn merge_classified(&mut self, uses: impl IntoIterator<Item = (String, UseClass)>) {
        for (name, class) in uses {
            match class {
                UseClass::Substituted => self.substituted.insert(name),
                UseClass::Quoted => self.quoted.insert(name),
                UseClass::Name => self.by_name.insert(name),
            };
        }
    }

    /// Drop every name a collapsed body defines itself, from both halves.
    fn remove_defs(&mut self, defs: &HashSet<String>) {
        self.substituted.retain(|v| !defs.contains(v));
        self.quoted.retain(|v| !defs.contains(v));
        self.by_name.retain(|v| !defs.contains(v));
    }
}

/// Explicit standalone [`uses_of`] with each name's [`UseClass`].
///
/// A name reached by both a substituted word and a brace-quoted one is
/// [`UseClass::Substituted`] — the definite read wins.
pub fn uses_of_classified(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    registry: &CommandRegistry,
) -> Vec<(String, UseClass)> {
    uses_of_classified_in_context(stmt, scanner, SsaInvocationContext::standalone(registry))
}

fn uses_of_classified_in_context(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
) -> Vec<(String, UseClass)> {
    let registry = selection.registry;
    if !stmt.is_executable_invocation() {
        return Vec::new();
    }
    if let Statement::Call {
        tokens: Some(tokens),
        ..
    }
    | Statement::Barrier {
        tokens: Some(tokens),
        ..
    } = stmt
    {
        if !tokens.evaluates_words() {
            return Vec::new();
        }
        if tokens.synthetic == Some(crate::ir::SyntheticMarker::EvaluatedArguments) {
            let mut names = Vec::new();
            for (index, text) in tokens.argv_texts.iter().enumerate() {
                if index == 0 || !tokens.arg_is_braced_literal(index - 1) {
                    for name in scanner.scan_word(text, registry) {
                        let read = (name, UseClass::Substituted);
                        if !names.contains(&read) {
                            names.push(read);
                        }
                    }
                }
            }
            return names;
        }
    }
    if stmt.has_opaque_native_accesses() {
        return Vec::new(); // The retained unknown-read flag is independent of names.
    }
    let mut found = ClassifiedUses::default();
    let mut reads_own_def: BTreeSet<String> = BTreeSet::new();

    match stmt {
        Statement::ExprEval { expr, .. } => {
            found.substituted.extend(expr_vars_for(scanner, expr));
        }

        Statement::AssignConst { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. }
        | Statement::Incr { .. } => {
            uses_in_assignment(
                stmt,
                scanner,
                selection,
                &mut found.substituted,
                &mut found.by_name,
                &mut reads_own_def,
            );
        }

        Statement::Call { .. } => {
            uses_in_call(stmt, scanner, selection, &mut found, &mut reads_own_def);
        }

        // A braced return value is literal: `proc f {} { return {$y} }`
        // returns the two characters `$y` and reads nothing.
        // tclsh-proof: tclsh8.6.14 — `proc f {} { return {$y} }; puts [f]`
        // prints `$y` with `y` undefined.
        Statement::Return {
            value,
            expr,
            braced,
            ..
        } => {
            if let Some(v) = value {
                let sink = if *braced {
                    &mut found.quoted
                } else {
                    &mut found.substituted
                };
                sink.extend(scanner.scan_word(v, registry));
            }
            if let Some(e) = expr {
                found.substituted.extend(expr_vars_for(scanner, e));
            }
        }

        Statement::Barrier { .. } => {
            uses_in_barrier(stmt, scanner, selection, &mut found, &mut reads_own_def);
        }

        // A non-lowered (glob/regexp/fall-through) `switch` is kept opaque as a
        // single `Statement::Switch` in the block. Recover the subject + arm /
        // default body reads so a variable read only as the subject or only
        // inside an arm body isn't reported unused.
        Statement::Switch {
            subject,
            arms,
            default_body,
            patterns_braced,
            ..
        } => {
            found.merge(switch_reads(
                subject,
                arms,
                default_body.as_ref(),
                *patterns_braced,
                scanner,
                selection,
            ));
            // The subject is read *before* any arm assigns, so it stays a live
            // read even when an arm also defines it (`defs_of` may now report
            // the subject var as switch-defined). Without this the read-before-
            // def of the subject would be filtered out below.
            for v in scanner.scan_word(subject, registry) {
                reads_own_def.insert(v);
            }
        }

        // Other structured IR statements (If, For, While, …) are flattened by
        // the CFG builder before SSA construction, so they never reach here.
        _ => {}
    }

    finish_classified_uses(found, &reads_own_def, stmt, selection)
}

fn finish_classified_uses(
    found: ClassifiedUses,
    reads_own_def: &BTreeSet<String>,
    stmt: &Statement,
    selection: SsaInvocationContext<'_>,
) -> Vec<(String, UseClass)> {
    // Exclude variables defined by this statement, unless they're
    // read-before-write.  Route through the registry so
    // `trace add variable` defs come from the registry's VarWrite
    // role rather than a string match.
    let defs: HashSet<String> = defs_of_in_context(stmt, Some(selection))
        .into_iter()
        .collect();
    // A name reached more than one way keeps its most definite class, and the
    // three are ordered by what a consumer may do with them: a substituted
    // word is a read *and* an operand something may rewrite; a name argument
    // is a read with nothing to rewrite; a quoted mention is neither. So
    // `lappend a $a` is Substituted (there is an operand), `lappend a {$a}` is
    // Name (the braced mention adds nothing to the name argument), and the
    // weaker classes drop the name the stronger one already asserts.
    let ClassifiedUses {
        substituted,
        mut quoted,
        mut by_name,
    } = found;
    by_name.retain(|v| !substituted.contains(v));
    quoted.retain(|v| !substituted.contains(v) && !by_name.contains(v));
    substituted
        .into_iter()
        .map(|v| (v, UseClass::Substituted))
        .chain(by_name.into_iter().map(|v| (v, UseClass::Name)))
        .chain(quoted.into_iter().map(|v| (v, UseClass::Quoted)))
        .filter(|(v, _)| !v.is_empty() && (!defs.contains(v) || reads_own_def.contains(v)))
        .collect()
}

/// Variable reads of a [`Statement::Barrier`]: its head + non-body argument
/// words, plus the scope-alias name a `dict with` / `dict update` unpacks.
/// Extracted from [`uses_of_classified`].
fn uses_in_barrier(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
    found: &mut ClassifiedUses,
    reads_own_def: &mut BTreeSet<String>,
) {
    let registry = selection.registry;
    let Statement::Barrier {
        command,
        canonical_command,
        args,
        tokens,
        ..
    } = stmt
    else {
        return;
    };
    scan_command_words(
        command,
        canonical_command.as_deref(),
        args,
        tokens.as_ref(),
        scanner,
        selection,
        found,
    );
    // Scope-alias subcommands (`dict with` / `dict update` — any
    // resolved subcommand with `creates_scope_alias`): the aliased
    // variable name is a plain string, not a $-substitution, so
    // scan_word misses it.
    //
    // The variable arg carries both VarRead and VarWrite roles.
    // When barrier defs route through the registry, the same name
    // appears in `defs` from the VarWrite query.  The closing
    // filter in `uses_of_classified`
    // (`!defs.contains(v) || reads_own_def.contains(v)`) would
    // then drop the var unless we mark it as reads-own-def here.
    // Without this, a proc whose only reference to a parameter is
    // `dict with $param {}` would produce a false unused-parameter
    // diagnostic.
    let resolution = tokens.as_ref().and_then(|tokens| selection.resolve(tokens));
    if let Some(crate::registry_invocation::RegistryInvocationResolution::Resolved(facts)) =
        resolution
    {
        if !facts
            .traits
            .contains(tcl_registry::Traits::CREATES_SCOPE_ALIAS)
        {
            return;
        }
        let Some(effective) = tokens
            .as_ref()
            .and_then(crate::registry_invocation::effective_command_words)
        else {
            return;
        };
        for &(index, role) in &facts.arg_roles {
            if role != tcl_registry::ArgRole::VarWrite {
                continue;
            }
            let Some(word) = effective
                .words
                .get(facts.argument_offset + usize::from(index) + 1)
            else {
                continue;
            };
            let Some(name) = crate::registry_invocation::invocation_word(word).literal() else {
                continue;
            };
            let name = normalise_var_name(name).to_owned();
            if !name.is_empty() {
                found.by_name.insert(name.clone());
                reads_own_def.insert(name);
            }
        }
        return;
    }
    if tokens.is_some() || !selection.standalone {
        return;
    }
    let creates_scope_alias = registry
        .get(command)
        .zip(args.first())
        .and_then(|(spec, sub)| spec.resolve_subcommand(sub))
        .is_some_and(|sub| sub.creates_scope_alias);
    if !creates_scope_alias {
        return;
    }
    let arguments: Vec<_> = args.iter().map(String::as_str).collect();
    for index in registry.arg_indices_for_role(command, &arguments, tcl_registry::ArgRole::VarWrite)
    {
        if let Some(name) = args
            .get(index)
            .map(|name| normalise_var_name(name).to_owned())
            && !name.is_empty()
        {
            found.by_name.insert(name.clone());
            reads_own_def.insert(name);
        }
    }
}

/// Variable reads of an assignment-style statement (`set` / `set =expr` /
/// `set =value` / `incr`). A *dynamic* target name (`set $p …`) reads its
/// name-bearing variable(s) (the genuine read the dead-store / unused checks
/// need — `defs_of` withholds the static def); a static target that the RHS
/// also reads is recorded as read-before-write. Extracted from [`uses_of`].
/// The value word an aliased / renamed `set` (`interp alias {} myset {} set` /
/// `rename set myset`) stores, or `None` when the `Call` is not a
/// value-passthrough store in the `set VAR VALUE` shape.
///
/// Keyed off the *canonical* command's `Set` lowering hook — the registry's
/// own "this is a value-passthrough store" fact — so the read scan matches the
/// un-aliased `set`, never the source spelling. The two-arg / single-def guard
/// restricts it to the setter shape (no `interp alias` prepended args shifting
/// the value word out of `args[1]`, and not the one-arg getter, which has no
/// def).
fn canonical_set_value<'a>(
    command: &str,
    canonical_command: Option<&str>,
    args: &'a [String],
    defs: &[String],
    selection: SsaInvocationContext<'_>,
    tokens: Option<&CommandTokens>,
) -> Option<&'a str> {
    let registry = selection.registry;
    if defs.len() != 1 {
        return None;
    }
    if let Some(tokens) = tokens {
        let invocation = selection.normal(tokens)?;
        let value_argument = invocation.stored_value_argument()?;
        return args
            .get(invocation.written_argument(value_argument)?)
            .map(String::as_str);
    }
    if !selection.standalone || args.len() != 2 {
        return None;
    }
    let canon = canonical_command.unwrap_or(command);
    let is_set = registry.get(canon).and_then(|s| s.lowering_hook)
        == Some(tcl_registry::hooks::LoweringHookId::Set);
    is_set.then(|| args[1].as_str())
}

/// Reads of a `set`-style value word, matching what the un-aliased `set`
/// lowering captures: an `[expr {…}]` value is parsed as an expression (so a
/// braced `$x`, which plain word scanning would miss, is seen); any other value
/// word is scanned for `$`-substitutions, recursing into `[...]` command
/// substitutions. Alias-derived nested expression reads already retained in
/// [`Statement::Call::reads`] complement this fallback; this path preserves
/// the scanner's requested base/element qualification for direct registry
/// spellings.
fn set_value_reads(
    value: &str,
    tokens: Option<&CommandTokens>,
    scanner: &mut VarReferenceScanner,
    registry: &CommandRegistry,
) -> BTreeSet<String> {
    // naming.variable.selected-setter-expression-reads
    // docs/design/analysis/name-resolution-proofs/variable-selected-setter-expression-reads.md
    // Queried commands already use original entered/conditional child owners
    // in scan_nested_substitution_words. A written head cannot add another
    // evaluator after that lookup has selected a different or unknown handler.
    if tokens.is_some_and(|tokens| tokens.source_binding.is_some()) {
        return scanner.scan_word(value, registry);
    }
    let trimmed = value.trim();
    // The registry carries the expression-language profile; the scanner owns
    // the exact word lexer config used to recover the `[expr …]` interior.
    let profile = registry.profile();
    if let Some(inner) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']'))
        && let Some((_, _, expr_arg, _)) =
            crate::lowering_hooks::extract_single_expr_arg_with_config(
                inner,
                &crate::alias::CommandAliasMap::new(),
                "::",
                registry,
                None,
                scanner.lexer_config(),
            )
    {
        let expr = crate::parse_expr_for_profile(&expr_arg, profile);
        return if scanner.element_qualified() {
            expr.vars_element_qualified_with_config(scanner.lexer_config())
                .into_iter()
                .collect()
        } else {
            expr.vars_with_config(scanner.lexer_config())
                .into_iter()
                .collect()
        };
    }
    scanner.scan_word(value, registry)
}

/// Variable reads of a [`Statement::Call`]: its head + non-body argument
/// words, generic lowering-provided reads (`reads`), a read-modify-write
/// target (`reads_own_defs`), and the ordinary direct-registry fallback for a
/// `set`-style value word. Mirrors [`uses_in_assignment`]'s shape; extracted
/// from [`uses_of`].
fn normal_handler_reads(
    tokens: Option<&CommandTokens>,
    defs: &[String],
    selection: SsaInvocationContext<'_>,
    found: &mut ClassifiedUses,
    reads_own_def: &mut BTreeSet<String>,
) {
    if let Some(invocation) = tokens.and_then(|tokens| selection.normal(tokens)) {
        for (index, role) in invocation.variable_roles() {
            if role != tcl_registry::ArgRole::VarRead
                && !(role == tcl_registry::ArgRole::VarWrite
                    && invocation
                        .variable_traits()
                        .contains(tcl_registry::Traits::READS_BEFORE_WRITE))
            {
                continue;
            }
            if let Some(name) = invocation.argument_literal(index) {
                let name = normalise_var_name(&name).to_owned();
                found.by_name.insert(name.clone());
                if defs.contains(&name) {
                    reads_own_def.insert(name);
                }
            }
        }
    }
}

fn uses_in_call(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
    found: &mut ClassifiedUses,
    reads_own_def: &mut BTreeSet<String>,
) {
    let registry = selection.registry;
    let Statement::Call {
        command,
        canonical_command,
        args,
        defs,
        reads,
        reads_own_defs,
        tokens,
        ..
    } = stmt
    else {
        return;
    };
    scan_command_words(
        command,
        canonical_command.as_deref(),
        args,
        tokens.as_ref(),
        scanner,
        selection,
        found,
    );
    let vars_found = &mut found.substituted;
    if let Some(value) = canonical_set_value(
        command,
        canonical_command.as_deref(),
        args,
        defs,
        selection,
        tokens.as_ref(),
    ) {
        for v in set_value_reads(value, tokens.as_ref(), scanner, registry) {
            if defs.iter().any(|d| d.as_str() == v) {
                reads_own_def.insert(v.clone());
            }
            vars_found.insert(v);
        }
    }
    let retained_roles = selection.standalone
        || tokens
            .as_ref()
            .is_some_and(|tokens| selection.normal(tokens).is_some());
    // `reads` are the registry's `ArgRole::VarRead` positions and
    // `reads_own_defs` its `READS_BEFORE_WRITE` targets: both name the cell
    // rather than substituting it, so they are `UseClass::Name` — a real read
    // with no operand word behind it.
    for name in reads.iter().filter(|_| retained_roles) {
        if name.is_empty() {
            continue;
        }
        found.by_name.insert(name.clone());
        // A name this statement both reads and defines is read *before* it is
        // written — the same rule `uses_in_barrier` applies to a `dict with`
        // scope alias. Without it the closing def-filter in
        // `uses_of_classified` drops the read, and the store feeding
        // `puts [incr n]` looks overwritten-before-read (#2050).
        if defs.contains(name) {
            reads_own_def.insert(name.clone());
        }
    }
    if *reads_own_defs && retained_roles {
        for name in defs {
            found.by_name.insert(name.clone());
            reads_own_def.insert(name.clone());
        }
    }
    normal_handler_reads(tokens.as_ref(), defs, selection, found, reads_own_def);
    // A destroying command (`unset a(k)`) consumes the target's *existence*:
    // the killed store is not dead — deleting it would make the unset error
    // on every call. Record the prior version as a read (DESTROYS_VARIABLE,
    // matching the pre-per-element behaviour the base-level use gave).
    let facts = tokens.as_ref().and_then(|tokens| selection.resolve(tokens));
    let traits = match facts.as_ref() {
        Some(crate::registry_invocation::RegistryInvocationResolution::Resolved(facts)) => {
            facts.traits
        }
        _ if tokens.is_none() && selection.standalone => registry
            .get(canonical_command.as_deref().unwrap_or(command))
            .map_or(tcl_registry::Traits::empty(), |spec| spec.traits),
        _ => tcl_registry::Traits::empty(),
    };
    let destroys = traits.contains(tcl_registry::Traits::DESTROYS_VARIABLE);
    if destroys {
        for name in defs {
            found.by_name.insert(name.clone());
            reads_own_def.insert(name.clone());
        }
    }
    // A conditional writer (`regexp`, `scan`, `binary scan`) stores into its
    // targets only on the match / conversion path; on the other path tclsh
    // leaves each previous value in place and never creates a target that did
    // not exist (8.4.20 through 9.1b0, all identical). So the definition this
    // statement appears to kill is still live, and deleting the store feeding
    // it changes the program: `set a before; regexp {(x)(y)} zz a b; puts $a`
    // prints `before`, and with the store gone it failed outright with
    // `can't read "a"` (#2051).
    //
    // Modelled exactly like the destroyer above, and for the same reason: the
    // def stays, so `emit_provably_unset_w210` still sees it and the no-match
    // prover keeps working, while the prior version becomes a read.
    //
    // `invocation_traits` rather than `get`, because `binary scan` carries the
    // trait on its *subcommand*.
    let conditionally_writes = traits.contains(tcl_registry::Traits::CONDITIONAL_VARIABLE_WRITE);
    if conditionally_writes {
        for name in defs {
            found.by_name.insert(name.clone());
            reads_own_def.insert(name.clone());
        }
    }
}

fn uses_in_assignment(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
    vars_found: &mut BTreeSet<String>,
    by_name: &mut BTreeSet<String>,
    reads_own_def: &mut BTreeSet<String>,
) {
    let registry = selection.registry;
    // A brace-quoted target's `$` is part of a literal name, not a
    // substitution: it is neither a dynamic target nor a read of the
    // `$`-less lookalike.
    let note_reads_own = |name: &str,
                          braced: bool,
                          scanner: &VarReferenceScanner,
                          vars_found: &BTreeSet<String>,
                          rod: &mut BTreeSet<String>| {
        let norm = scanner.canonical_name_braced(name, braced);
        if !norm.is_empty() && vars_found.contains(norm) {
            rod.insert(norm.to_owned());
        }
    };
    match stmt {
        Statement::AssignConst {
            name, name_braced, ..
        } => {
            if is_dynamic_write_target(name, *name_braced) {
                vars_found.extend(scanner.scan_word(name, registry));
            }
        }
        Statement::AssignExpr {
            name,
            name_braced,
            expr,
            ..
        } => {
            vars_found.extend(expr_vars_for(scanner, expr));
            if is_dynamic_write_target(name, *name_braced) {
                vars_found.extend(scanner.scan_word(name, registry));
            } else {
                note_reads_own(name, *name_braced, scanner, vars_found, reads_own_def);
            }
        }
        Statement::AssignValue {
            name,
            name_braced,
            value,
            tokens,
            ..
        } => {
            vars_found.extend(scanner.scan_word(value, registry));
            // `scan_word` declines to substitute inside braces, exactly as the
            // *word* parser does — but a nested `[expr {$x + 1}]` substitutes
            // its own braced body, and that read is real. The call path has
            // recovered it; an assignment value substitutes by the same
            // rules, so `set r [list [expr {0 in $x}]]` reads `x` just as
            // `puts [list [expr {0 in $x}]]` does. Same owner, one more
            // statement kind — not a second recovery.
            scan_nested_substitution_words(
                tokens.as_ref(),
                scanner,
                selection,
                vars_found,
                by_name,
            );
            if is_dynamic_write_target(name, *name_braced) {
                vars_found.extend(scanner.scan_word(name, registry));
            } else {
                note_reads_own(name, *name_braced, scanner, vars_found, reads_own_def);
            }
        }
        Statement::Incr {
            name,
            name_braced,
            amount,
            ..
        } => {
            if is_dynamic_write_target(name, *name_braced) {
                vars_found.extend(scanner.scan_word(name, registry));
            } else {
                // `incr a` names the cell it mutates; there is no `$a` word.
                let norm = scanner.canonical_name_braced(name, *name_braced);
                if !norm.is_empty() {
                    by_name.insert(norm.to_owned());
                    reads_own_def.insert(norm.to_owned());
                }
            }
            if let Some(amt) = amount {
                vars_found.extend(scanner.scan_word(amt, registry));
            }
        }
        _ => {}
    }
}

/// The expression-AST variable reads, named per the scanner's qualification
/// mode — element-qualified for the SSA build, base names otherwise.
fn expr_vars_for(
    scanner: &VarReferenceScanner,
    expr: &crate::expr_ast::ExprNode,
) -> BTreeSet<String> {
    if scanner.element_qualified() {
        expr.vars_element_qualified_with_config(scanner.lexer_config())
            .into_iter()
            .collect()
    } else {
        expr.vars_with_config(scanner.lexer_config())
            .into_iter()
            .collect()
    }
}

/// Scan a `Call` / `Barrier` head word plus its non-body argument words into
/// `vars_found`. `Body`-role args (loop / `if` / `catch` scripts) are skipped —
/// they are lowered into their own CFG blocks. Extracted from [`uses_of`].
fn scan_command_words(
    command: &str,
    canonical_command: Option<&str>,
    args: &[String],
    tokens: Option<&CommandTokens>,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
    out: &mut ClassifiedUses,
) {
    let registry = selection.registry;
    out.substituted.extend(scanner.scan_word(command, registry));
    let lookup = canonical_command.unwrap_or(command);
    let resolution = tokens.and_then(|tokens| selection.resolve(tokens));
    let facts = match resolution.as_ref() {
        Some(crate::registry_invocation::RegistryInvocationResolution::Resolved(facts)) => {
            Some(facts.as_ref())
        }
        _ => None,
    };
    let effective = tokens.and_then(crate::registry_invocation::effective_command_words);
    let mut body_indices = HashSet::new();
    let mut name_role_braced = HashSet::new();
    let mut in_frame_braced = HashSet::new();
    if let (Some(facts), Some(effective)) = (facts, effective.as_ref()) {
        for &(index, role) in &facts.arg_roles {
            let Some(written) =
                effective.written_argument(facts.argument_offset + usize::from(index))
            else {
                continue;
            };
            if matches!(
                role,
                tcl_registry::ArgRole::VarWrite | tcl_registry::ArgRole::VarRead
            ) && tokens.is_some_and(|tokens| tokens.arg_is_braced_literal(written))
            {
                name_role_braced.insert(written);
            }
            if role.braced_word_evaluated_in_frame() {
                if role == tcl_registry::ArgRole::Body
                    && (facts.body_kind == tcl_registry::BodyKind::Structural
                        || tokens.is_some_and(|tokens| tokens.evaluated_body().is_some()))
                {
                    body_indices.insert(written);
                } else {
                    in_frame_braced.insert(written);
                }
            }
        }
    } else if tokens.is_none() && selection.standalone {
        body_indices.extend(structural_body_indices(lookup, args, tokens, registry));
        let arguments: Vec<_> = args.iter().map(String::as_str).collect();
        in_frame_braced.extend(registry.arg_indices_evaluated_in_frame(lookup, &arguments));
    }
    let described = if tokens.is_some() {
        facts.is_some() || !selection.standalone
    } else {
        selection.standalone && registry_describes(registry, lookup)
    };
    for (idx, arg) in args.iter().enumerate() {
        if body_indices.contains(&idx) || name_role_braced.contains(&idx) {
            continue;
        }
        let braced = tokens.is_some_and(|t| t.arg_is_braced_literal(idx));
        for name in scanner.scan_word(arg, registry) {
            let class = braced_word_class(&BracedWordSite {
                braced,
                evaluated_in_frame: in_frame_braced.contains(&idx),
                described,
                word: arg,
                name: &name,
                registry,
                config: scanner.lexer_config(),
            });
            match class {
                UseClass::Quoted => out.quoted.insert(name),
                UseClass::Substituted => out.substituted.insert(name),
                // `braced_word_class` classifies a *word*, and a word is
                // never a name argument — those never reach the scanner.
                UseClass::Name => out.by_name.insert(name),
            };
        }
    }
    scan_nested_substitution_words(
        tokens,
        scanner,
        selection,
        &mut out.substituted,
        &mut out.by_name,
    );
}

/// Record the reads inside a nested `[cmd …]`'s brace-quoted words that the
/// nested command evaluates in *this* frame.
///
/// The loop above applies exactly this rule to a **statement's** words: a
/// braced word in an [`ArgRole`](tcl_registry::ArgRole) position whose
/// `braced_word_evaluated_in_frame` holds is [`UseClass::Substituted`] — "a
/// genuine read, here and now" — which is why `expr {$x + 1}` written as a
/// statement records `x`.
///
/// A nested substitution never becomes a statement, so its words never
/// reached that rule, and `$x` in `puts [expr {$x + 1}]` was recorded
/// nowhere: [`VarReferenceScanner::scan_word`] correctly declines to
/// substitute inside the braces (the *command* parser does not), and nothing
/// then asked `expr` whether it substitutes them itself (it does). The read
/// vanished from `SsaStatement::uses`, and with it from every consumer of
/// that map — SCCP, taint, type inference, def-use, GVN, interval bounds and
/// the shimmer detectors. It was observable in taint, where only a pair of
/// braces separated the two spellings:
///
/// ```tcl
/// eval [expr $tainted]      ;# T100
/// eval [expr {$tainted}]    ;# nothing, before this
/// expr {$tainted}           ;# T100 again, once it is a statement
/// ```
///
/// So this applies the same registry-driven rule at the nested positions,
/// rather than teaching any consumer about substitutions. A braced word the
/// nested command merely carries as data is left to the conservative `Quoted`
/// handling the enclosing word already got.
///
/// Writes only substituted names — a word the nested command evaluates in this
/// frame is a genuine read — so it takes that half directly rather than the
/// whole [`ClassifiedUses`], which lets the assignment path (whose collector
/// *is* the substituted set) share the one owner.
///
/// # Why `Expr` only, and not every in-frame role
///
/// [`ArgRole::braced_word_evaluated_in_frame`](tcl_registry::ArgRole::braced_word_evaluated_in_frame)
/// answers for `Body` as well as `Expr`, and for a **statement** that is right:
/// `foreach x $l {…}` runs its body in this frame, and the lowerer gives that
/// body its own CFG block, so the loop variable is *defined* where the body's
/// reads are seen.
///
/// A nested substitution gets no such block — that is the representational gap
/// this whole module works around — so a nested body's reads would arrive with
/// none of its own definitions:
///
/// ```tcl
/// set r [join [lmap x $lines {string toupper $x}] ","]
/// ```
///
/// scanning the `lmap` body records a read of `x` that nothing in the frame
/// ever writes, and `W210 read before it is set` fires on the loop variable.
/// `Expr` is the role that both substitutes here *and* binds nothing of its
/// own, which is exactly what the `[expr {$x + 1}]` case this exists for
/// needs; `Body` is left to the conservative handling until a nested
/// substitution is lowered as a command.
fn scan_nested_substitution_words(
    tokens: Option<&CommandTokens>,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
    substituted: &mut BTreeSet<String>,
    by_name: &mut BTreeSet<String>,
) {
    let registry = selection.registry;
    let config = scanner.lexer_config();
    if !selection.standalone
        && tokens.is_none_or(|tokens| selection.token_metadata(tokens).is_none())
    {
        return;
    }
    if tokens.is_some_and(|tokens| tokens.source_binding.is_some()) {
        for evaluation in
            crate::word_subst::entered_expression_evaluations(tokens, config, registry)
        {
            let config = config.with_grammar(evaluation.grammar());
            let names = if scanner.element_qualified() {
                evaluation
                    .expression()
                    .vars_element_qualified_with_config(config)
            } else {
                evaluation.expression().vars_with_config(config)
            };
            substituted.extend(names);
        }
    }
    for lifted in crate::word_subst::lifted_calls(tokens, config) {
        if let Some(child) = lifted.tokens.as_ref()
            && child.source_binding.is_some()
        {
            scan_selected_expression_roles(child, scanner, selection, substituted);
            if let Some(invocation) = selection.normal(child) {
                for (index, role) in invocation.variable_roles() {
                    if (role == tcl_registry::ArgRole::VarRead
                        || (role == tcl_registry::ArgRole::VarWrite
                            && invocation
                                .variable_traits()
                                .contains(tcl_registry::Traits::READS_BEFORE_WRITE)))
                        && let Some(name) = invocation.argument_literal(index)
                    {
                        by_name.insert(normalise_var_name(&name).to_owned());
                    }
                }
            }
            continue;
        }
        if !selection.standalone {
            continue;
        }
        let arg_strs: Vec<&str> = lifted.args.iter().map(String::as_str).collect();
        let in_frame =
            registry.arg_indices_for_role(&lifted.command, &arg_strs, tcl_registry::ArgRole::Expr);
        for idx in in_frame {
            // Only a braced word needs recovering. An unbraced one already
            // substitutes at the command level, so the enclosing word's own
            // scan has seen it. Which word is braced is the segmenter's
            // answer, not a `{`/`}` test over the argument text: `{a}{b}` and
            // a word merely *containing* braces are not brace-quoted literals.
            let braced = match lifted.arg_words.get(idx) {
                Some(WordExpr::BracedLiteral { text, .. }) => text.as_str(),
                // The structure is only absent when the word recovery
                // declined the substitution's shape, and then there is
                // nothing to recover from it either.
                _ => continue,
            };
            substituted.extend(scanner.scan_word(braced, registry));
        }
    }
}

// Non-Expr selected handlers may also declare expression arguments. Their
// possible read roles remain separate from the entered Expr evaluator receipt.
fn scan_selected_expression_roles(
    tokens: &CommandTokens,
    scanner: &VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
    substituted: &mut BTreeSet<String>,
) {
    let registry = selection.registry;
    let Some(metadata) = selection.token_metadata(tokens) else {
        return;
    };
    let Some(binding) = tokens.source_binding.as_ref() else {
        return;
    };
    if let Some(evaluation) = binding.conditional_expression_evaluation(registry, tokens) {
        let config = tcl_lexer::LexerConfig::from_grammar(evaluation.lexer_grammar());
        let names = if scanner.element_qualified() {
            evaluation.tree().vars_element_qualified_with_config(config)
        } else {
            evaluation.tree().vars_with_config(config)
        };
        substituted.extend(names);
    }
    if binding.runtime_reachability() != crate::command_binding::SourceRuntimeReachability::Reached
    {
        return;
    }
    let Some(invocation) =
        crate::registry_invocation::resolved_handler_invocation_with_metadata_context(
            registry,
            Some(metadata),
            tokens,
        )
    else {
        return;
    };
    if invocation.facts.operation
        == tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Expr,
        )
    {
        return;
    }
    let Some(dialect) = binding.variable_context.invocation_dialect else {
        return;
    };
    let parser = dialect.expression_parse_context(None);
    let config = tcl_lexer::LexerConfig::from_grammar(parser.lexer_grammar);
    for (index, role) in invocation.written_argument_roles() {
        if role != tcl_registry::ArgRole::Expr {
            continue;
        }
        let Some(WordExpr::BracedLiteral { text, .. }) = tokens.words().get(index + 1) else {
            continue;
        };
        let expression = crate::expr_parser::parse_expr_with_syntax_context(text, &parser);
        let names = if scanner.element_qualified() {
            expression.vars_element_qualified_with_config(config)
        } else {
            expression.vars_with_config(config)
        };
        substituted.extend(names);
    }
}

/// Whether the registry describes `name` — accepting a space-joined
/// *ensemble subcommand* spelling (`dict for`, `dict map`) as well as a plain
/// command name.
///
/// The CFG's synthetic loop header names itself that way
/// (`cfg_builder::cfg_lower::lower_foreach`), and `registry.get` keys only on
/// whole command names, so a plain `get` would report a `dict for` header as
/// *unclassified* and read its braced value word as a substitution.  Mirrors
/// the base/sub split `shimmer::use_site::foreach_header_expected_type`
/// already does for the same statement.
fn registry_describes(registry: &CommandRegistry, name: &str) -> bool {
    if registry.get(name).is_some() {
        return true;
    }
    name.split_once(' ').is_some_and(|(base, sub)| {
        registry
            .get(base)
            .is_some_and(|spec| spec.resolve_subcommand(sub).is_some())
    })
}

/// One `(argument word, name found in it)` pair, with the facts
/// [`braced_word_class`] decides on.
struct BracedWordSite<'a> {
    /// The word is a brace-quoted literal — Tcl substitutes nothing in it.
    braced: bool,
    /// The registry gives this position a role whose word the callee
    /// re-evaluates in the *calling* frame (`Body` / `Expr`).
    evaluated_in_frame: bool,
    /// The registry has a `CommandSpec` for this command at all.
    described: bool,
    /// The word's text (braces already stripped).
    word: &'a str,
    /// The name the scan found inside it.
    name: &'a str,
    /// The registry, asked which words of the word's own commands bind a
    /// variable — see [`crate::script_binds::script_binds_name`].
    registry: &'a CommandRegistry,
    /// The document's lexing configuration — the word is re-segmented as a
    /// script below, and that re-read must draw the same word boundaries the
    /// document's own grammar draws.
    config: tcl_lexer::LexerConfig,
}

/// How this statement consumes `site.name`.
///
/// Tcl substitutes nothing inside `{…}`, so a braced word is never read *at*
/// the call site. What the word then **is** falls into three kinds, and the
/// answer differs per kind:
///
/// - **script, this frame** — `expr {$a + $b}`, `if {$c} …`: the callee
///   re-evaluates the text where the caller's variables are in scope, so the
///   name really is read here. [`UseClass::Substituted`].
/// - **data** — a braced word of a command the registry *describes*, at a
///   role that never evaluates it: `puts {$y}`, `string match {$pat*} …`,
///   `lsort -command {cmp $x}`. The registry is the authority that nothing
///   here evaluates the word in this frame, so there is no read.
///   [`UseClass::Quoted`].
/// - **unclassified** — a braced word of a command the registry does *not*
///   describe: a user proc, an unknown definer. It may be a script, and if it
///   is it may run in this frame — a wrapper that hands it to an
///   `uplevel`-ing worker does exactly that, and tclsh then errors on an
///   unset name — so the read stands. **Unless** the word binds the name
///   itself: then the read is of that script's own local whichever frame it
///   runs in, which is the shape an un-hooked definer body takes.
fn braced_word_class(site: &BracedWordSite<'_>) -> UseClass {
    if !site.braced || site.evaluated_in_frame {
        return UseClass::Substituted;
    }
    if site.described
        || crate::script_binds::script_binds_name(
            site.word,
            site.name,
            crate::script_binds::Ownership::Bindings,
            site.registry,
            site.config,
        )
    {
        return UseClass::Quoted;
    }
    UseClass::Substituted
}

/// Reads of a non-lowered (`-glob`/`-regexp`, or `-exact` with a fall-through
/// arm) `switch` kept opaque in a CFG block: the subject word, every arm
/// pattern, and the *free* reads of each arm/default body.
///
/// The arm bodies are the *only* scripts in the pipeline that reach SSA
/// un-lowered, so this walk is the one place a `UseClass` would otherwise be
/// invented rather than derived. It is threaded through instead: a
/// brace-quoted data word inside an arm keeps the same
/// [`UseClass::Quoted`] it would carry had the arm been lowered, so
/// read-before-set skips it while liveness still honours it.
fn switch_reads(
    subject: &str,
    arms: &[crate::ir::SwitchArm],
    default_body: Option<&crate::ir::Script>,
    patterns_braced: bool,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
) -> ClassifiedUses {
    let registry = selection.registry;
    let mut reads = ClassifiedUses {
        substituted: scanner.scan_word(subject, registry),
        quoted: BTreeSet::new(),
        by_name: BTreeSet::new(),
    };
    for arm in arms {
        // A pattern from the canonical single braced `{pat body …}` block is a
        // literal list element — tclsh 9.0.4: `proc f {z} {switch -glob $z
        // {$a* {puts hit} default {puts miss}}}; f {$a}` prints `hit` with `a`
        // undefined. Supplied as separate words (`switch $s $pat {body}`) the
        // pattern does substitute. Same classification the lowered `-exact`
        // path already applies to its patterns.
        let pattern_sink = if patterns_braced {
            &mut reads.quoted
        } else {
            &mut reads.substituted
        };
        pattern_sink.extend(scanner.scan_word(&arm.pattern, registry));
        if let Some(body) = &arm.body {
            reads.merge(free_reads_in_script(body, scanner, selection));
        }
    }
    if let Some(db) = default_body {
        reads.merge(free_reads_in_script(db, scanner, selection));
    }
    reads
}

/// Reads a collapsed body consumes from the *outer* scope: its reads minus its
/// own defs (so arm-local temporaries — `set tmp 1; puts $tmp` — aren't seen as
/// outer reads). The def set is completed with the `for`-init/next and
/// if/while/for condition command-sub defs that `defs_from_ir_script` omits.
fn free_reads_in_script(
    script: &crate::ir::Script,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
) -> ClassifiedUses {
    let registry = selection.registry;
    let mut defs: HashSet<String> = crate::ir_helpers::defs_from_ir_script(script)
        .into_iter()
        .collect();
    if selection.standalone {
        defs.extend(collapsed_extra_defs(script, registry, 0));
    }
    let mut reads = reads_in_script(script, scanner, selection);
    reads.remove_defs(&defs);
    reads
}

/// Recursively collect classified variable reads from an un-lowered IR script.
fn reads_in_script(
    script: &crate::ir::Script,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
) -> ClassifiedUses {
    let mut reads = ClassifiedUses::default();
    for stmt in &script.statements {
        reads.merge(reads_in_stmt(stmt, scanner, selection));
    }
    reads
}

/// Classified variable reads of a single statement, recursing into nested
/// bodies. Leaf reads come from [`uses_of_classified`] (which resolves a
/// nested `Statement::Switch` via [`switch_reads`]); structured statements are
/// walked here because they are not lowered inside an opaque switch arm.
///
/// Every context this walk adds by hand — an `if`/`while`/`for` condition, a
/// loop's value word, a nested body — is one the enclosing frame really does
/// evaluate, so it is [`UseClass::Substituted`]; the single exception is a
/// **braced** loop value word, which is literal list text.
fn reads_in_stmt(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
) -> ClassifiedUses {
    let registry = selection.registry;
    let mut reads = ClassifiedUses::default();
    reads.merge_classified(uses_of_classified_in_context(stmt, scanner, selection));
    match stmt {
        Statement::If {
            clauses, else_body, ..
        } => {
            for clause in clauses {
                reads
                    .substituted
                    .extend(expr_vars_for(scanner, &clause.condition));
                reads.merge(reads_in_script(&clause.body, scanner, selection));
            }
            if let Some(eb) = else_body {
                reads.merge(reads_in_script(eb, scanner, selection));
            }
        }
        Statement::While {
            condition, body, ..
        } => {
            reads.substituted.extend(expr_vars_for(scanner, condition));
            reads.merge(reads_in_script(body, scanner, selection));
        }
        Statement::For {
            init,
            condition,
            next,
            body,
            ..
        } => {
            reads.merge(reads_in_script(init, scanner, selection));
            reads.substituted.extend(expr_vars_for(scanner, condition));
            reads.merge(reads_in_script(next, scanner, selection));
            reads.merge(reads_in_script(body, scanner, selection));
        }
        Statement::Foreach {
            iterators, body, ..
        } => {
            for it in iterators {
                // A braced value word is literal list text — `foreach n {a $b
                // c}` iterates the three characters `$b`, it does not read
                // `b`. The name is still *recorded*, as `Quoted`: dropping it
                // here would take its liveness use with it and draw a false
                // W220 on a store whose only mention is that word.
                // Classifying is what separates the two.
                let sink = if it.list_braced {
                    &mut reads.quoted
                } else {
                    &mut reads.substituted
                };
                sink.extend(scanner.scan_word(&it.list_arg, registry));
            }
            reads.merge(reads_in_script(body, scanner, selection));
        }
        Statement::Catch { body, .. } => {
            reads.merge(reads_in_script(body, scanner, selection));
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            reads.merge(reads_in_script(body, scanner, selection));
            for handler in handlers {
                reads.merge(reads_in_script(&handler.body, scanner, selection));
            }
            if let Some(fb) = finally_body {
                reads.merge(reads_in_script(fb, scanner, selection));
            }
        }
        _ => {}
    }
    reads
}

/// Depth cap for [`collapsed_extra_defs`]'s recursion over nested
/// `if`/`while`/`for`/`foreach`/`catch`/`try`/`switch` bodies.
/// Transitively bounded via `MAX_LOWER_NEST_DEPTH` (every `Script`
/// feeding SSA construction was built by `crate::lowering`, which already
/// caps its own construction at 256), capped here independently for
/// defence-in-depth and consistency with every other full-tree walker in
/// this crate.
const MAX_COLLAPSED_EXTRA_DEFS_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

/// `for`-init/next clause defs and if/while/for condition command-sub defs
/// (`[regexp … -> v]`) that [`crate::ir_helpers::defs_from_ir_script`] does not
/// recurse — recovered for the collapsed-body read subtraction only.
fn collapsed_extra_defs(
    script: &crate::ir::Script,
    registry: &CommandRegistry,
    depth: u32,
) -> BTreeSet<String> {
    use crate::ir_helpers::{defs_from_expr, defs_from_ir_script};
    let mut extra = BTreeSet::new();
    if MAX_COLLAPSED_EXTRA_DEFS_DEPTH.exceeded(depth) {
        return extra;
    }
    for stmt in &script.statements {
        match stmt {
            Statement::If {
                clauses, else_body, ..
            } => {
                for clause in clauses {
                    extra.extend(defs_from_expr(&clause.condition, registry));
                    extra.extend(collapsed_extra_defs(&clause.body, registry, depth + 1));
                }
                if let Some(eb) = else_body {
                    extra.extend(collapsed_extra_defs(eb, registry, depth + 1));
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                extra.extend(defs_from_expr(condition, registry));
                extra.extend(collapsed_extra_defs(body, registry, depth + 1));
            }
            Statement::For {
                init,
                condition,
                next,
                body,
                ..
            } => {
                extra.extend(defs_from_ir_script(init));
                extra.extend(defs_from_ir_script(next));
                extra.extend(defs_from_expr(condition, registry));
                extra.extend(collapsed_extra_defs(init, registry, depth + 1));
                extra.extend(collapsed_extra_defs(next, registry, depth + 1));
                extra.extend(collapsed_extra_defs(body, registry, depth + 1));
            }
            Statement::Foreach { body, .. } | Statement::Catch { body, .. } => {
                extra.extend(collapsed_extra_defs(body, registry, depth + 1));
            }
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                extra.extend(collapsed_extra_defs(body, registry, depth + 1));
                for handler in handlers {
                    extra.extend(collapsed_extra_defs(&handler.body, registry, depth + 1));
                }
                if let Some(fb) = finally_body {
                    extra.extend(collapsed_extra_defs(fb, registry, depth + 1));
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for arm in arms {
                    if let Some(body) = &arm.body {
                        extra.extend(collapsed_extra_defs(body, registry, depth + 1));
                    }
                }
                if let Some(db) = default_body {
                    extra.extend(collapsed_extra_defs(db, registry, depth + 1));
                }
            }
            _ => {}
        }
    }
    extra
}

/// Frame for the iterative rename walk (avoids deep recursion).
struct RenameFrame {
    block: BlockId,
    child_index: usize,
    pushed_vars: Vec<VariableCellKey>,
    phase: RenamePhase,
}

/// Phase within a single rename frame.
enum RenamePhase {
    /// Process phi nodes and statements for this block.
    Enter,
    /// Iterate over dominator-tree children.
    ProcessChildren,
}

/// Name-keyed per-block state produced by the rename walk and consumed when
/// assembling the final SSA blocks. Bundled so the walk and the assembly step
/// share one value instead of threading five parallel maps.
#[derive(Default)]
struct RenameOutputs {
    phi_versions: HashMap<BlockId, HashMap<VariableCellKey, Version>>,
    phi_incoming: HashMap<BlockId, HashMap<VariableCellKey, HashMap<BlockId, Version>>>,
    entry_versions: HashMap<BlockId, HashMap<VariableCellKey, Version>>,
    exit_versions: HashMap<BlockId, HashMap<VariableCellKey, Version>>,
    stmt_infos: HashMap<BlockId, Vec<SsaStatement>>,
}

/// Add variable definitions introduced through a registry-typed instance's
/// class option table (for example a widget's linked input variable).
///
/// The ordinary registry def scan sees direct command heads. Runtime object
/// commands (`.entry configure …`, `$widget configure …`) need the same scan
/// after resolving their receiver class. The method opt-in and the option's
/// external-input bit are both registry data; this layer names neither a
/// method nor an option.
fn enrich_instance_option_defs(
    func: &cfg::Function,
    registry: &CommandRegistry,
) -> Option<cfg::Function> {
    let mut enriched = func.clone();
    let defs = enrich_instance_option_defs_with_initial(
        &mut enriched,
        registry,
        &crate::taint::InstanceClassState::new(),
    );
    (!defs.is_empty()).then_some(enriched)
}

/// Mirror a registry-proven top-level global option definition under Tcl's
/// equivalent bare spelling.
///
/// A widget's deferred `-textvariable` must name a global so it outlives the
/// constructor frame.  The registry/lowerer therefore records its static
/// `VarWrite` as `::name`. At module scope, however, Tcl resolves `$name` to
/// that same global variable.  SSA otherwise interns those spellings as two
/// unrelated variables and loses the input source at the overwhelmingly
/// common top-level `$name` read.  Only the one-segment global spelling has
/// this equivalence: `::ns::name` is not a bare `name` in `::top`.
///
/// This is a scope rule, not a Tk rule. Any registry option declaring a global
/// `VarWrite` gets the same top-level alias, while an ordinary qualified write
/// (`set ::x`, `unset ::x`) does not manufacture a second SSA definition.
/// Procedure bodies retain Tcl's strict local/global distinction.
fn mirror_top_level_global_option_defs(
    func: &mut cfg::Function,
    registry: &CommandRegistry,
) -> bool {
    let mut changed = false;
    for block in func.blocks.values_mut() {
        for statement in &mut block.statements {
            let Statement::Call {
                command,
                canonical_command,
                args,
                defs,
                tokens,
                ..
            } = statement
            else {
                continue;
            };
            let name = canonical_command.as_deref().unwrap_or(command);
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            let aliases: Vec<String> = registry
                .arg_indices_for_role(name, &arg_refs, tcl_registry::ArgRole::VarWrite)
                .into_iter()
                .filter(|&index| {
                    registry.option_variable_scope(
                        name,
                        &arg_refs,
                        index,
                        registry.own_surface_query(),
                    ) == Some(tcl_registry::VariableScope::Global)
                })
                .filter_map(|index| {
                    let raw = args.get(index)?;
                    let braced = tokens
                        .as_ref()
                        .is_some_and(|tokens| tokens.arg_is_braced_literal(index));
                    let declared = crate::naming::element_var_name_braced(raw, braced);
                    let global = if declared.starts_with("::") {
                        declared.to_owned()
                    } else {
                        format!("::{declared}")
                    };
                    let bare = global.strip_prefix("::")?;
                    (!bare.is_empty() && !bare.contains("::") && defs.contains(&global))
                        .then_some(bare.to_owned())
                })
                .filter(|alias| !defs.contains(alias))
                .collect();
            changed |= !aliases.is_empty();
            defs.extend(aliases);
        }
    }
    changed
}

/// In-place form used by module CFG construction when a procedure begins with
/// proven interpreter-global receiver facts.
pub(crate) fn enrich_instance_option_defs_with_initial(
    func: &mut cfg::Function,
    registry: &CommandRegistry,
    initial: &crate::taint::InstanceClassState,
) -> HashSet<String> {
    let is_top_level = func.name == "::top";
    let classes = crate::taint::local_instance_classes_with_initial(func, registry, initial);
    let mut instance_defs = HashSet::new();
    for block in func.blocks.values_mut() {
        for statement in &mut block.statements {
            let Statement::Call {
                span,
                command,
                args,
                defs,
                ..
            } = statement
            else {
                continue;
            };
            if args.is_empty() {
                continue;
            }
            let Some(class) =
                crate::taint::unique_instance_class(command, Some(&classes), Some(span.start()))
            else {
                continue;
            };
            let invocation_args: Vec<&str> = args.iter().map(String::as_str).collect();
            let Some(invocation) = registry.resolve_instance_invocation(
                class,
                command,
                &invocation_args,
                registry.own_surface_query(),
            ) else {
                continue;
            };
            if !invocation
                .semantics
                .traits
                .contains(tcl_registry::Traits::CONFIGURES_INSTANCE_OPTIONS)
            {
                continue;
            }
            let option_args: Vec<&str> = args[1..].iter().map(String::as_str).collect();
            for (option_index, candidate) in args[1..].iter().enumerate() {
                if candidate.is_empty() || candidate.contains(['$', '[']) {
                    continue;
                }
                if tcl_registry::taint::taints_var_write(
                    registry,
                    class,
                    &option_args,
                    registry.own_surface_query(),
                    candidate,
                ) {
                    let name = crate::naming::element_var_name(candidate);
                    let name = if registry.option_variable_scope(
                        class,
                        &option_args,
                        option_index,
                        registry.own_surface_query(),
                    ) == Some(tcl_registry::VariableScope::Global)
                        && !name.starts_with("::")
                    {
                        format!("::{name}")
                    } else {
                        name.to_owned()
                    };
                    if !name.is_empty() {
                        instance_defs.insert(name.clone());
                        if !defs.contains(&name) {
                            defs.push(name.clone());
                        }
                        if is_top_level
                            && let Some(bare) = name.strip_prefix("::")
                            && !bare.is_empty()
                            && !bare.contains("::")
                        {
                            instance_defs.insert(bare.to_owned());
                            if !defs.iter().any(|defined| defined == bare) {
                                defs.push(bare.to_owned());
                            }
                        }
                    }
                }
            }
        }
    }
    instance_defs
}

/// Build SSA with dominator-based phi placement and renaming.
///
/// Computes dominators, places phi nodes, then walks the dominator
/// tree to assign SSA version numbers to every variable definition
/// and use.
///
/// This function is inherently long because the rename walk couples
/// version counters, stacks, phi versions, incoming edges, and
/// per-statement use/def maps — splitting it would just scatter the
/// state across many parameters.
// Long renumbering pass with sequential block-walk phases.
/// Intern names that appear *only* in a terminator (a `return $x` value /
/// expr, or a branch condition). The rename walk interns statement and phi
/// names but not terminator reads, so a parameter read solely in the return
/// (`proc p {x} { return $x }`) would otherwise be absent from the interner —
/// leaving `var_symbol` unable to resolve it, which breaks any consumer that
/// resolves such a name (e.g. the interprocedural O103 seed for an
/// argument-sensitive passthrough). Interning is map-membership only; no SSA
/// statement / version map changes, so every analysis result is unaffected.
fn source_read_representations(
    access: &crate::command_binding::SourceVariableAccess,
    registry: &CommandRegistry,
) -> SsaReadRepresentationAdvice {
    if access.context_residual() != crate::command_binding::SourceVariableReadResidual::Closed {
        return SsaReadRepresentationAdvice::default();
    }
    captured_read_representations(
        access.context_alternatives().iter().map(|context| {
            let place = access.place_in_context(context, registry);
            (context.as_ref(), place)
        }),
        registry,
    )
}

/// Common projection for substitutions, expression operands and native named
/// reads. An empty or opaque inventory cannot supply any positive evidence.
fn captured_read_representations<'a>(
    reads: impl Iterator<Item = (&'a crate::var_resolve::ResolveContext, crate::place::Place)>,
    registry: &CommandRegistry,
) -> SsaReadRepresentationAdvice {
    use tcl_syntax::value::ValueRepresentation;
    let mut seen = false;
    let mut representation = None;
    let mut representation_closed = true;
    let mut container_alternatives = None;
    let mut containers_closed = true;
    let mut already_numeric = true;
    let mut numeric_category = None;
    let mut numeric_categories_closed = true;
    for (context, place) in reads {
        seen = true;
        if !context.read_produces_value(&place, registry) {
            return SsaReadRepresentationAdvice::default();
        }
        let current = context.contents_representation_at(&place);
        representation_closed &= current != ValueRepresentation::Unknown
            && representation.is_none_or(|previous| previous == current);
        representation = Some(current);
        if let Some(current) = context.container_representation_alternatives_at(&place) {
            container_alternatives = Some(container_alternatives.map_or(
                current,
                |previous: crate::native_numeric::ClosedContainerRepresentations| {
                    previous.joined(current)
                },
            ));
        } else {
            containers_closed = false;
        }
        already_numeric &= context.contents_already_native_numeric_at(&place, registry);
        if let Some(current) = context.contents_native_numeric_category_at(&place, registry) {
            numeric_category = Some(numeric_category.map_or(current, |previous| {
                if previous == current {
                    current
                } else {
                    tcl_registry::TclType::Numeric
                }
            }));
        } else {
            numeric_categories_closed = false;
        }
    }
    SsaReadRepresentationAdvice {
        representation: (seen && representation_closed)
            .then_some(representation)
            .flatten(),
        container_alternatives: (seen && containers_closed)
            .then_some(container_alternatives)
            .flatten(),
        already_numeric: seen && already_numeric,
        numeric_category: (seen && numeric_categories_closed)
            .then_some(numeric_category)
            .flatten(),
    }
}

fn exclusive_argument_owner(
    owner: &crate::command_binding::SourceVariableEvaluationOwner,
    invocation: &crate::command_binding::CommandAllocationSite,
) -> bool {
    use crate::command_binding::SourceVariableEvaluationOwner;
    match owner {
        SourceVariableEvaluationOwner::InvocationArguments {
            invocation: original,
            ..
        } => original == invocation,
        SourceVariableEvaluationOwner::Alternatives(owners) => {
            !owners.is_empty()
                && owners
                    .iter()
                    .all(|owner| exclusive_argument_owner(owner, invocation))
        }
        _ => false,
    }
}

fn source_read_place(
    access: &crate::command_binding::SourceVariableAccess,
    registry: &CommandRegistry,
) -> crate::place::Place {
    access.place_in_context(&access.variable_context, registry)
}

fn source_read_name(access: &crate::command_binding::SourceVariableAccess) -> String {
    let config = access
        .variable_context
        .invocation_dialect
        .map_or_else(tcl_lexer::LexerConfig::default, |dialect| {
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar)
        });
    tcl_syntax::naming::var_reference_for_style(&access.original_spelling, config.braced_var)
        .to_owned()
}

fn captured_read_names(
    points: &PointResolveContexts,
    block: BlockId,
    index: usize,
) -> HashSet<String> {
    points
        .source_reads_at(block, index)
        .iter()
        .map(source_read_name)
        .collect()
}

fn invocation_read_bindings(
    points: &PointResolveContexts,
    block: BlockId,
    index: usize,
    context: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<VariableCellKey> {
    // Implementation contract: naming.variable.invocation-caller-ssa-frame
    // docs/design/analysis/name-resolution-proofs/variable-invocation-caller-ssa-frame.md
    points
        .source_tokens_at(block, index)
        .into_iter()
        .flat_map(|tokens| {
            crate::place_bridge::invocation_execution_read_places(tokens, registry)
                .into_iter()
                .chain(
                    crate::place_bridge::invocation_argument_execution_read_places(
                        tokens, registry,
                    ),
                )
        })
        .filter_map(|place| crate::var_resolve::canonical_binding_value_key(&place))
        .filter_map(|key| invocation_read_key_in_context(key, context))
        .collect()
}

/// Execution inventories retain all physical subtree reads. Foreign callee
/// locals are not caller SSA cells; shared namespace and caller alias cells stay.
fn invocation_read_key_in_context(
    key: VariableCellKey,
    context: &ResolveContext,
) -> Option<VariableCellKey> {
    // Implementation contract: naming.variable.invocation-caller-ssa-frame
    // docs/design/analysis/name-resolution-proofs/variable-invocation-caller-ssa-frame.md
    key.activation_identity()
        .is_none_or(|activation| context.activation.as_deref() == Some(activation))
        .then_some(key)
}

fn source_read_bindings(
    points: &PointResolveContexts,
    block: BlockId,
    index: usize,
    registry: &CommandRegistry,
) -> Vec<(String, VariableCellKey)> {
    points
        .source_reads_at(block, index)
        .iter()
        .filter_map(|access| {
            let name = crate::var_resolve::canonical_binding_value_key(&source_read_place(
                access, registry,
            ))?;
            Some((source_read_name(access), name))
        })
        .collect()
}

fn retained_read_references(
    points: &PointResolveContexts,
    blocks: &HashMap<BlockId, SsaBlock>,
    interner: &VarInterner,
    registry: &CommandRegistry,
    clobbers: &ValueClobbers,
) -> HashMap<(BlockId, usize, SourceSite, String), SsaReadReference> {
    let origins = represented_contents_origins(blocks, clobbers);
    let mut references = HashMap::new();
    for (&block_id, block) in blocks {
        for index in (0..block.statements.len()).chain(std::iter::once(usize::MAX)) {
            for access in points.source_reads_at(block_id, index) {
                let place = source_read_place(access, registry);
                let Some(name) = crate::var_resolve::canonical_binding_value_key(&place) else {
                    continue;
                };
                let Some(&symbol) = interner.to_symbol.get(&name) else {
                    continue;
                };
                let candidate = if index == usize::MAX {
                    block.exit_versions.get(&symbol).copied().unwrap_or(0)
                } else {
                    block.statements[index]
                        .uses
                        .get(&symbol)
                        .copied()
                        .unwrap_or(0)
                };
                let source_origin = access
                    .variable_context
                    .read_contents_origin(&place, registry);
                let represented = if candidate == 0 {
                    Some(&ContentsOrigin::Incoming)
                } else {
                    origins.get(&(symbol, candidate))
                };
                let version = (!place.observed
                    && access.variable_context.contents_presence(&place)
                        == crate::var_resolve::ContentsPresence::Defined
                    && !access.variable_context.store_would_error(&place)
                    && source_origin != ContentsOrigin::Unknown
                    && represented == Some(&source_origin))
                .then_some(candidate);
                references.insert(
                    (
                        block_id,
                        index,
                        access.source.clone(),
                        access.original_spelling.clone(),
                    ),
                    SsaReadReference { symbol, version },
                );
            }
        }
    }
    retain_cached_input_references(points, &mut references, registry);
    references
}

fn retain_cached_input_references(
    points: &PointResolveContexts,
    references: &mut HashMap<(BlockId, usize, SourceSite, String), SsaReadReference>,
    registry: &CommandRegistry,
) {
    type ReachedReadAt = (BlockId, usize, SsaReadReference);
    let mut by_source: HashMap<(&SourceSite, &str), Vec<ReachedReadAt>> = HashMap::new();
    for ((block, index, source, spelling), &read) in references.iter() {
        by_source
            .entry((source, spelling.as_str()))
            .or_default()
            .push((*block, *index, read));
    }
    let mut cached = Vec::new();
    for (&(block, index), tokens) in points.source_token_inventory() {
        if !matches!(
            tokens.synthetic,
            Some(crate::ir::SyntheticMarker::IterationBindings(_))
        ) {
            continue;
        }
        for access in &tokens.variable_accesses {
            let mut selected = None;
            let mut conflict = false;
            for &(origin_block, origin_index, read) in by_source
                .get(&(&access.source, access.original_spelling.as_str()))
                .into_iter()
                .flatten()
            {
                if !points
                    .source_reads_at(origin_block, origin_index)
                    .iter()
                    .any(|original| {
                        original.source == access.source
                            && original.original_spelling == access.original_spelling
                            && same_captured_read(original, access, registry)
                    })
                {
                    continue;
                }
                if read.version.is_none() || selected.is_some_and(|previous| previous != read) {
                    conflict = true;
                    break;
                }
                selected = Some(read);
            }
            if !conflict && let Some(read) = selected {
                cached.push((
                    (
                        block,
                        index,
                        access.source.clone(),
                        access.original_spelling.clone(),
                    ),
                    read,
                ));
            }
        }
    }
    references.extend(cached);
}

fn same_captured_read(
    original: &crate::command_binding::SourceVariableAccess,
    captured: &crate::command_binding::SourceVariableAccess,
    registry: &CommandRegistry,
) -> bool {
    let actual = source_read_place(original, registry);
    let retained = source_read_place(captured, registry);
    // Unrelated namespace and frame-world changes do not change a captured
    // input. Its physical address, observer state and reaching contents do.
    actual == retained
        && original
            .variable_context
            .read_contents_origin(&actual, registry)
            == captured
                .variable_context
                .read_contents_origin(&retained, registry)
}

fn represented_contents_origins(
    blocks: &HashMap<BlockId, SsaBlock>,
    clobbers: &ValueClobbers,
) -> HashMap<(Symbol, Version), ContentsOrigin> {
    let mut origins = HashMap::new();
    let phi_versions: HashSet<_> = blocks
        .values()
        .flat_map(|block| block.phis.iter().map(|phi| (phi.name, phi.version)))
        .collect();
    for block in blocks.values() {
        for statement in &block.statements {
            for (&symbol, &version) in &statement.defs {
                origins.insert(
                    (symbol, version),
                    if statement.may_defs.contains(&symbol)
                        || statement.destruction_defs.contains(&symbol)
                    {
                        ContentsOrigin::Unknown
                    } else {
                        ContentsOrigin::WrittenAt(statement.statement.span().start())
                    },
                );
            }
        }
    }
    loop {
        let mut changed = false;
        // A scalar barrier creates a widened analysis value, not an
        // executable store. Keep its exact binding's original contents
        // lineage so retained reads select the fresh lattice key rather than
        // falling back to a source store before the barrier.
        for versions in clobbers.values().flat_map(|markers| markers.values()) {
            for (&symbol, &(prior, fresh)) in versions {
                let origin = if prior == 0 {
                    Some(ContentsOrigin::Incoming)
                } else {
                    origins.get(&(symbol, prior)).cloned()
                };
                if let Some(origin) = origin
                    && origins.get(&(symbol, fresh)) != Some(&origin)
                {
                    origins.insert((symbol, fresh), origin);
                    changed = true;
                }
            }
        }
        for block in blocks.values() {
            for phi in &block.phis {
                let joined = phi
                    .incoming
                    .values()
                    .filter_map(|&version| {
                        if version == 0 {
                            Some(ContentsOrigin::Incoming)
                        } else {
                            origins.get(&(phi.name, version)).cloned().or_else(|| {
                                (!phi_versions.contains(&(phi.name, version)))
                                    .then_some(ContentsOrigin::Unknown)
                            })
                        }
                    })
                    .reduce(|left, right| left.joined(&right));
                if let Some(origin) = joined
                    && origins.get(&(phi.name, phi.version)) != Some(&origin)
                {
                    origins.insert((phi.name, phi.version), origin);
                    changed = true;
                }
            }
        }
        if !changed {
            return origins;
        }
    }
}

fn intern_terminator_reads(
    func: &cfg::Function,
    walk: &mut RenameWalk<'_>,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) {
    for (&id, block) in &func.blocks {
        let mut symbols = HashMap::new();
        for (source, _) in crate::def_use::terminator_read_vars(block.terminator.as_ref(), config) {
            if let Some(name) =
                canonical_variable_key(&source, walk.points.before_terminator(id), registry)
            {
                symbols.insert(source, walk.interner.intern(&name));
            } else {
                walk.unknown_source_names.insert(source);
            }
        }
        let captured = source_read_bindings(&walk.points, id, usize::MAX, registry);
        for (source, _) in &captured {
            symbols.remove(source);
        }
        let mut ambiguous = HashSet::new();
        for (source, bound) in captured {
            let symbol = walk.interner.intern(&bound);
            if symbols
                .insert(source.clone(), symbol)
                .is_some_and(|prior| prior != symbol)
            {
                ambiguous.insert(source);
            }
        }
        for source in ambiguous {
            symbols.remove(&source);
            walk.unknown_source_names.insert(source);
        }
        walk.point_symbols.insert((id, usize::MAX), symbols);
    }
}

/// Assemble the final [`SsaBlock`] map from the rename walk's outputs: build
/// each block's phi list and intern its name-keyed entry / exit version maps
/// onto the persisted [`Symbol`]-keyed form. Extracted from [`build_ssa`].
fn assemble_ssa_blocks(
    func: &cfg::Function,
    phi_vars: &HashMap<BlockId, HashSet<VariableCellKey>>,
    interner: &mut VarInterner,
    out: &mut RenameOutputs,
) -> HashMap<BlockId, SsaBlock> {
    let mut ssa_blocks: HashMap<BlockId, SsaBlock> = HashMap::new();
    for (bn, block) in &func.blocks {
        let mut phis: Vec<Phi> = Vec::new();
        let mut phi_var_list: Vec<VariableCellKey> = phi_vars
            .get(bn)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();
        phi_var_list.sort_by_cached_key(|key| format!("{key:?}"));

        for var in &phi_var_list {
            phis.push(Phi {
                name: interner.intern(var),
                version: out
                    .phi_versions
                    .get(bn)
                    .and_then(|m| m.get(var))
                    .copied()
                    .unwrap_or(0),
                incoming: out
                    .phi_incoming
                    .get(bn)
                    .and_then(|m| m.get(var))
                    .cloned()
                    .unwrap_or_default(),
            });
        }

        // Intern the entry / exit version maps (built name-keyed during the
        // walk) onto the persisted [`Symbol`]-keyed block.
        let intern_versions = |interner: &mut VarInterner, m: HashMap<VariableCellKey, Version>| {
            m.into_iter()
                .map(|(name, ver)| (interner.intern(&name), ver))
                .collect::<HashMap<Symbol, Version>>()
        };
        let entry_v = intern_versions(interner, out.entry_versions.remove(bn).unwrap_or_default());
        let exit_v = intern_versions(interner, out.exit_versions.remove(bn).unwrap_or_default());

        ssa_blocks.insert(
            *bn,
            SsaBlock {
                name: block.name.clone(),
                phis,
                statements: out.stmt_infos.remove(bn).unwrap_or_default(),
                entry_versions: entry_v,
                exit_versions: exit_v,
            },
        );
    }
    ssa_blocks
}

/// Mutable state threaded through the dominator-tree rename walk: the live
/// version stacks / counters keyed by variable name, the use-scanner and
/// name interner, and the accumulated per-block [`RenameOutputs`].
struct RenameWalk<'a> {
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
    version_counter: HashMap<VariableCellKey, Version>,
    stacks: HashMap<VariableCellKey, Vec<Version>>,
    scanner: VarReferenceScanner,
    interner: VarInterner,
    out: RenameOutputs,
    value_clobbers: ValueClobbers,
    points: PointResolveContexts,
    point_symbols: HashMap<(BlockId, usize), HashMap<String, Symbol>>,
    unknown_source_names: HashSet<String>,
}

impl<'a> RenameWalk<'a> {
    fn new(
        func: &cfg::Function,
        config: tcl_lexer::LexerConfig,
        points: PointResolveContexts,
        metadata: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
    ) -> Self {
        let mut out = RenameOutputs::default();
        for id in func.blocks.keys() {
            out.phi_versions.insert(*id, HashMap::new());
            out.phi_incoming.insert(*id, HashMap::new());
            out.entry_versions.insert(*id, HashMap::new());
            out.exit_versions.insert(*id, HashMap::new());
            out.stmt_infos.insert(*id, Vec::new());
        }
        Self {
            metadata,
            version_counter: HashMap::new(),
            stacks: HashMap::new(),
            scanner: VarReferenceScanner::with_config(
                VarScanOptions {
                    include_var_read_roles: metadata
                        .is_some_and(|context| context.source_analysis_input().is_none()),
                    recurse_cmd_substitutions: true,
                    include_reads_before_write: false,
                    element_qualified: true,
                },
                config,
            ),
            interner: VarInterner::default(),
            out,
            value_clobbers: HashMap::new(),
            points,
            point_symbols: HashMap::new(),
            unknown_source_names: HashSet::new(),
        }
    }

    /// Top (current) version of `var`, or `0` when none is live.
    fn top(&self, var: &VariableCellKey) -> Version {
        self.stacks
            .get(var)
            .and_then(|s| s.last().copied())
            .unwrap_or(0)
    }

    /// Allocate a fresh version for `var` and push it onto its live stack.
    fn push_new(&mut self, var: &VariableCellKey) -> Version {
        let vn = self.version_counter.get(var).copied().unwrap_or(0) + 1;
        self.version_counter.insert(var.to_owned(), vn);
        self.stacks.entry(var.to_owned()).or_default().push(vn);
        vn
    }

    /// Snapshot the currently-visible (var, version) pairs (those with a live
    /// version > 0), including this block's phi targets.
    fn visible_versions(&self, bn: BlockId) -> HashMap<VariableCellKey, Version> {
        let mut visible_vars: HashSet<VariableCellKey> = self.stacks.keys().cloned().collect();
        visible_vars.extend(self.out.phi_versions[&bn].keys().cloned());
        visible_vars
            .iter()
            .filter_map(|v| {
                let t = self.top(v);
                (t > 0).then(|| (v.clone(), t))
            })
            .collect()
    }

    fn preserve_unknown_read_liveness(
        &self,
        raw_classified: &[(String, UseClass)],
        context: &ResolveContext,
        registry: &CommandRegistry,
        classified: &mut Vec<(VariableCellKey, UseClass)>,
    ) {
        if raw_classified
            .iter()
            .any(|(name, _)| canonical_variable_key(name, context, registry).is_none())
        {
            // An unresolved read may consume any reaching current-frame value.
            // Preserve liveness without claiming a definite uninitialised read.
            let mut visible: Vec<_> = self.stacks.keys().cloned().collect();
            visible.sort_by_cached_key(|key| format!("{key:?}"));
            for name in visible {
                if !classified.iter().any(|(bound, _)| bound == &name) {
                    classified.push((name, UseClass::Quoted));
                }
            }
        }
    }

    fn retain_statement_symbols(
        &mut self,
        stmt: &Statement,
        point: (BlockId, usize),
        raw_classified: &[(String, UseClass)],
        captured: &[(String, VariableCellKey)],
        captured_names: &HashSet<String>,
        registry: &CommandRegistry,
    ) {
        let point_context = self.points.before_statement(point.0, point.1).clone();
        let context = &point_context;
        let mut source_symbols = HashMap::new();
        for (name, _) in raw_classified {
            if let Some(bound) = canonical_variable_key(name, context, registry) {
                source_symbols.insert(name.clone(), self.interner.intern(&bound));
            } else {
                self.unknown_source_names.insert(name.clone());
            }
        }
        for name in captured_names {
            source_symbols.remove(name);
        }
        let mut ambiguous = HashSet::new();
        for (name, bound) in captured {
            let symbol = self.interner.intern(bound);
            if source_symbols
                .insert(name.clone(), symbol)
                .is_some_and(|prior| prior != symbol)
            {
                ambiguous.insert(name.clone());
            }
        }
        for name in &ambiguous {
            source_symbols.remove(name);
            self.unknown_source_names.insert(name.clone());
        }
        if context.execution_name_policy.is_none() {
            for name in defs_of_in_context(
                stmt,
                Some(SsaInvocationContext::supplied(registry, self.metadata)),
            ) {
                let after = self.points.after_statement(point.0, point.1);
                let bound = definition_source_name(stmt, &name, after, registry);
                if let Some(bound) = bound {
                    source_symbols
                        .entry(name)
                        .or_insert_with(|| self.interner.intern(&bound));
                } else {
                    self.unknown_source_names.insert(name);
                }
            }
        }
        self.retain_native_operand_symbols(point, context, registry, &mut source_symbols);
        self.point_symbols
            .insert((point.0, point.1), source_symbols);
    }

    fn retain_native_operand_symbols(
        &mut self,
        point: (BlockId, usize),
        context: &ResolveContext,
        registry: &CommandRegistry,
        source_symbols: &mut HashMap<String, Symbol>,
    ) {
        if let Some(invocation) =
            self.points
                .source_tokens_at(point.0, point.1)
                .and_then(|tokens| {
                    crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
                        registry,
                        self.metadata,
                        tokens,
                    )
                })
        {
            for (argument, _) in invocation.variable_roles() {
                let Some(name) = invocation.argument_literal(argument) else {
                    continue;
                };
                let Some(target) = invocation.variable_operand_place(argument, context, registry)
                else {
                    continue;
                };
                if let Some(bound) = crate::var_resolve::canonical_binding_value_key(&target) {
                    let symbol = self.interner.intern(&bound);
                    if source_symbols
                        .insert(name.clone(), symbol)
                        .is_some_and(|old| old != symbol)
                    {
                        source_symbols.remove(&name);
                        self.unknown_source_names.insert(name.clone());
                    }
                }
            }
        }
    }

    /// Retain original read bindings and classify their exact cell identities
    /// before statement version changes.
    fn classify_statement_reads(
        &mut self,
        stmt: &Statement,
        point: (BlockId, usize),
        context: &ResolveContext,
        registry: &CommandRegistry,
    ) -> Vec<(VariableCellKey, UseClass)> {
        let raw_classified = uses_of_classified_in_context(
            stmt,
            &mut self.scanner,
            SsaInvocationContext::supplied(registry, self.metadata),
        );
        let captured = source_read_bindings(&self.points, point.0, point.1, registry);
        let captured_names = captured_read_names(&self.points, point.0, point.1);
        let mut classified: Vec<_> = raw_classified
            .iter()
            .filter(|(name, class)| {
                *class != UseClass::Substituted || !captured_names.contains(name)
            })
            .filter_map(|(name, class)| {
                canonical_variable_key(name, context, registry).map(|name| (name, *class))
            })
            .collect();
        classified.extend(
            captured
                .iter()
                .map(|(_, bound)| (bound.clone(), UseClass::Substituted)),
        );
        classified.extend(additional_read_bindings(
            &self.points,
            point.0,
            point.1,
            stmt,
            context,
            registry,
        ));
        self.preserve_unknown_read_liveness(&raw_classified, context, registry, &mut classified);
        self.retain_statement_symbols(
            stmt,
            point,
            &raw_classified,
            &captured,
            &captured_names,
            registry,
        );
        classified
    }

    /// Rename one statement's uses and defs into SSA form: look up each read's
    /// current version (carrying its [`UseClass`] through to `quoted_uses`),
    /// then push a fresh version for each def. Extracted from
    /// [`Self::enter_block`].
    fn rename_statement(
        &mut self,
        stmt: &Statement,
        frame: &mut RenameFrame,
        registry: &CommandRegistry,
        elems: &ArrayElems,
        index: usize,
    ) -> SsaStatement {
        let point_context = self.points.before_statement(frame.block, index).clone();
        let context = &point_context;
        let classified =
            self.classify_statement_reads(stmt, (frame.block, index), context, registry);
        let class_of: HashMap<&VariableCellKey, UseClass> = classified
            .iter()
            .map(|(name, class)| (name, *class))
            .collect();
        let uses_list: Vec<VariableCellKey> =
            classified.iter().map(|(name, _)| name.clone()).collect();
        let mut uses_map: HashMap<Symbol, Version> = HashMap::new();
        let mut quoted_uses: HashSet<Symbol> = HashSet::new();
        let mut name_only_uses: HashSet<Symbol> = HashSet::new();
        // A base read (`$a($i)`, `array get a`) reads every known
        // constant-keyed element — record their versions so the element chains
        // are live. A fanned element inherits its base's class: reached only
        // through a quoted base mention it is no more definite than that
        // mention.
        let fanned = expand_uses(&uses_list, &[], elems);
        for var in uses_list.iter().chain(fanned.iter()) {
            let v = self.top(var);
            let sym = self.interner.intern(var);
            if uses_map.insert(sym, v).is_some() {
                continue;
            }
            let class = class_of
                .get(var)
                .copied()
                .or_else(|| class_of.get(elems.root_for_element(var)?).copied());
            match class {
                Some(UseClass::Quoted) => {
                    quoted_uses.insert(sym);
                }
                Some(UseClass::Name) => {
                    name_only_uses.insert(sym);
                }
                Some(UseClass::Substituted) | None => {}
            }
        }

        let direct_defs = bound_defs(
            stmt,
            frame.block,
            index,
            &self.points,
            SsaInvocationContext::supplied(registry, self.metadata),
        );
        let destructions = crate::place_bridge::ssa_destruction_keys(
            stmt,
            context,
            self.points.after_statement(frame.block, index),
            registry,
        );
        let expanded = expand_defs(&direct_defs, elems);
        // A fanned element def is a *may*-write — the dynamic-key /
        // whole-array write may have hit it: record a use of its prior version
        // so type inference joins old and new rather than trusting the written
        // value alone. (The base refresh of an element write is also a may-def,
        // but reads nothing — an extra base use would make dead-store analysis
        // see every element write as an observation of the whole array.)
        for var in expanded
            .iter()
            .filter(|v| !direct_defs.contains(v) && elems.root_for_element(v).is_some())
        {
            let ver = self.top(var);
            uses_map.entry(self.interner.intern(var)).or_insert(ver);
        }
        let mut defs_map: HashMap<Symbol, Version> = HashMap::new();
        let mut may_defs: HashSet<Symbol> = HashSet::new();
        let mut destruction_defs: HashSet<Symbol> = HashSet::new();
        for var in expanded {
            let destruction = destructions
                .iter()
                .find(|(key, _)| key == &var || var.is_member_of(key));
            if destruction.is_some() {
                let symbol = self.interner.intern(&var);
                record_destruction_predecessor(
                    symbol,
                    self.top(&var),
                    &mut uses_map,
                    &mut name_only_uses,
                );
            }
            let ver = self.push_new(&var);
            frame.pushed_vars.push(var.clone());
            let sym = self.interner.intern(&var);
            defs_map.insert(sym, ver);
            if destruction.is_some() {
                destruction_defs.insert(sym);
            }
            if !direct_defs.contains(&var)
                || destruction.is_some_and(|(_, conditional)| *conditional)
            {
                may_defs.insert(sym);
            }
        }

        SsaStatement {
            statement: stmt.clone(),
            uses: uses_map,
            defs: defs_map,
            may_defs,
            destruction_defs,
            quoted_uses,
            name_only_uses,
        }
    }

    /// Process one block on first visit: assign phi versions, record entry /
    /// exit versions, rename statement uses/defs, and seed successors' phi
    /// incoming edges. Extracted from [`build_ssa`]'s rename walk.
    fn enter_block(
        &mut self,
        frame: &mut RenameFrame,
        func: &cfg::Function,
        phi_vars: &HashMap<BlockId, HashSet<VariableCellKey>>,
        registry: &CommandRegistry,
        elems: &ArrayElems,
        clobbers: &RegistryClobberNames,
    ) {
        let bn = frame.block;

        // Process phi nodes — push new versions.
        let mut phi_var_list: Vec<VariableCellKey> = phi_vars
            .get(&bn)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();
        phi_var_list.sort_by_cached_key(|key| format!("{key:?}"));

        for var in &phi_var_list {
            let ver = self.push_new(var);
            frame.pushed_vars.push(var.clone());
            self.out
                .phi_versions
                .get_mut(&bn)
                .unwrap()
                .insert(var.clone(), ver);
            self.out
                .phi_incoming
                .get_mut(&bn)
                .unwrap()
                .entry(var.clone())
                .or_default();
        }

        // Record entry versions.
        let ev = self.visible_versions(bn);
        *self.out.entry_versions.get_mut(&bn).unwrap() = ev;

        // Process statements.
        if let Some(block) = func.blocks.get(&bn) {
            let stmts: Vec<Statement> = block.statements.clone();
            for (index, stmt) in stmts.iter().enumerate() {
                let info = self.rename_statement(stmt, frame, registry, elems, index);
                self.out.stmt_infos.get_mut(&bn).unwrap().push(info);
                if let Some(names) = clobbers.get(&bn).and_then(|markers| markers.get(&index)) {
                    let mut versions = HashMap::new();
                    for name in names {
                        let prior = self.top(name);
                        let fresh = self.push_new(name);
                        frame.pushed_vars.push(name.clone());
                        versions.insert(self.interner.intern(name), (prior, fresh));
                    }
                    self.value_clobbers
                        .entry(bn)
                        .or_default()
                        .insert(index, versions);
                }
            }
        }

        // Record exit versions.
        let xv = self.visible_versions(bn);
        *self.out.exit_versions.get_mut(&bn).unwrap() = xv;

        // Fill in phi incoming edges for successors — the terminator's
        // successors plus any `try` exception-edge handler targets, so
        // a handler block's phis see this block's versions.
        for succ in func.block_successors(bn) {
            if !func.blocks.contains_key(&succ) {
                continue;
            }
            let mut succ_phis: Vec<VariableCellKey> = phi_vars
                .get(&succ)
                .map(|s| s.iter().cloned().collect())
                .unwrap_or_default();
            succ_phis.sort_by_cached_key(|key| format!("{key:?}"));
            for var in &succ_phis {
                let v = self.top(var);
                self.out
                    .phi_incoming
                    .get_mut(&succ)
                    .unwrap()
                    .entry(var.clone())
                    .or_default()
                    .insert(bn, v);
            }
        }
    }
}

/// Build SSA with dominator-based phi placement and renaming.
///
/// Computes dominators, places phi nodes, then walks the dominator
/// tree to assign SSA version numbers to every variable definition
/// and use.
#[must_use]
pub fn build_ssa(func: &cfg::Function, registry: &CommandRegistry) -> SsaFunction {
    build_ssa_with_config(
        func,
        registry,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
    )
}

/// Build SSA under the exact lexer configuration that produced `func`.
///
/// Unlike [`build_ssa`], this entry preserves environment-only grammar axes
/// (notably `JimTcl`, which deliberately has no catalogue profile) and host
/// overrides through phi placement, renaming, and terminator-read interning.
#[must_use]
pub fn build_ssa_with_config(
    func: &cfg::Function,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> SsaFunction {
    build_ssa_with_context(
        func,
        registry,
        config,
        ResolveContext::for_function(&func.name),
    )
}

fn project_ssa_source_names(walk: &mut RenameWalk<'_>, func: &cfg::Function) {
    register_unique_source_names(&mut walk.interner, &walk.point_symbols);
    for name in &walk.unknown_source_names {
        walk.interner.source_symbols.remove(name);
    }
    let mut source_names: HashMap<Symbol, Vec<&String>> = HashMap::new();
    for names in walk.point_symbols.values() {
        for (source, &symbol) in names {
            source_names.entry(symbol).or_default().push(source);
        }
    }
    for (symbol, mut names) in source_names {
        // Presentation is stable when several written aliases select one cell.
        names.sort_by_key(|name| (name.contains("::"), name.len(), *name));
        let source = names[0];
        let key = &walk.interner.keys[symbol.0 as usize];
        let display = &mut walk.interner.names[symbol.0 as usize];
        if !matches!(key, VariableCellKey::Authored(_))
            || (func.name == "::top" && !source.contains("::") && *display == format!("::{source}"))
        {
            display.clone_from(source);
        }
    }
}

#[derive(Default)]
struct ArrayRootRefreshMetadata {
    symbols: HashSet<Symbol>,
    versions: HashSet<(Symbol, Version)>,
}

fn array_root_refresh_metadata(
    entry: BlockId,
    elements: &ArrayElems,
    interner: &VarInterner,
    blocks: &HashMap<BlockId, SsaBlock>,
    points: &PointResolveContexts,
    selection: SsaInvocationContext<'_>,
) -> ArrayRootRefreshMetadata {
    let mut metadata = ArrayRootRefreshMetadata::default();
    if points
        .before_terminator(entry)
        .invocation_dialect
        .and_then(|dialect| dialect.variable_container_model)
        != Some(tcl_dialect::VariableContainerModel::DistinctArray)
    {
        return metadata;
    }
    let roots: HashSet<_> = elements
        .elements
        .keys()
        .filter_map(|root| interner.to_symbol.get(root).copied())
        .collect();
    let mut ordinary_definitions = HashSet::new();
    for (&block_id, block) in blocks {
        for (index, statement) in block.statements.iter().enumerate() {
            let direct = bound_defs(&statement.statement, block_id, index, points, selection);
            for (&symbol, &version) in &statement.defs {
                if roots.contains(&symbol)
                    && statement.may_defs.contains(&symbol)
                    && !direct.contains(&interner.keys[symbol.0 as usize])
                {
                    metadata.versions.insert((symbol, version));
                    metadata.symbols.insert(symbol);
                } else {
                    ordinary_definitions.insert(symbol);
                }
            }
        }
    }
    metadata
        .symbols
        .retain(|symbol| !ordinary_definitions.contains(symbol));
    propagate_array_refresh_phis(blocks, &mut metadata.versions);
    metadata
}

fn propagate_array_refresh_phis(
    blocks: &HashMap<BlockId, SsaBlock>,
    versions: &mut HashSet<(Symbol, Version)>,
) {
    loop {
        let mut changed = false;
        for phi in blocks.values().flat_map(|block| &block.phis) {
            if phi
                .incoming
                .values()
                .any(|&version| versions.contains(&(phi.name, version)))
                && phi
                    .incoming
                    .values()
                    .all(|&version| version == 0 || versions.contains(&(phi.name, version)))
            {
                changed |= versions.insert((phi.name, phi.version));
            }
        }
        if !changed {
            break;
        }
    }
}

fn rename_dominator_tree(
    walk: &mut RenameWalk<'_>,
    func: &cfg::Function,
    tree: &HashMap<BlockId, Vec<BlockId>>,
    phi_vars: &HashMap<BlockId, HashSet<VariableCellKey>>,
    registry: &CommandRegistry,
    elems: &ArrayElems,
    clobbers: &RegistryClobberNames,
) {
    let mut stack: Vec<RenameFrame> = Vec::new();

    if func.blocks.contains_key(&func.entry) {
        stack.push(RenameFrame {
            block: func.entry,
            child_index: 0,
            pushed_vars: Vec::new(),
            phase: RenamePhase::Enter,
        });
    }

    while let Some(frame) = stack.last_mut() {
        match frame.phase {
            RenamePhase::Enter => {
                walk.enter_block(frame, func, phi_vars, registry, elems, clobbers);
                frame.phase = RenamePhase::ProcessChildren;
            }

            RenamePhase::ProcessChildren => {
                let bn = frame.block;
                let children = tree.get(&bn).cloned().unwrap_or_default();
                let idx = frame.child_index;

                if idx < children.len() {
                    frame.child_index += 1;
                    let child = children[idx];
                    stack.push(RenameFrame {
                        block: child,
                        child_index: 0,
                        pushed_vars: Vec::new(),
                        phase: RenamePhase::Enter,
                    });
                } else {
                    // Pop versions pushed in this block.
                    let pushed = frame.pushed_vars.clone();
                    for var in pushed.iter().rev() {
                        if let Some(s) = walk.stacks.get_mut(var) {
                            s.pop();
                            if s.is_empty() {
                                walk.stacks.remove(var);
                            }
                        }
                    }
                    stack.pop();
                }
            }
        }
    }
}

/// Build SSA in an explicitly selected frame with module-proved namespace facts.
#[must_use]
pub fn build_ssa_with_context(
    func: &cfg::Function,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    entry: ResolveContext,
) -> SsaFunction {
    build_ssa_with_context_for_entry(func, registry, config, entry, None)
}

/// Preserve exact entry cells while excluding unbound version-zero clobbers.
pub(crate) fn build_ssa_with_context_for_entry(
    func: &cfg::Function,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    entry: ResolveContext,
    entry_bindings: Option<&[String]>,
) -> SsaFunction {
    let metadata = registry
        .profile()
        .map(|profile| tcl_registry::model::semantic::SemanticContext::for_profile(profile).into());
    build_ssa_with_context_for_entry_and_metadata(
        func,
        registry,
        config,
        entry,
        entry_bindings,
        metadata,
    )
}

fn supplied_ssa_metadata<'a>(
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
) -> Option<crate::registry_invocation::InvocationMetadataContext<'a>> {
    let metadata = metadata.filter(|metadata| metadata.matches_registry(registry))?;
    match metadata.source_analysis_input() {
        Some(input) => crate::registry_invocation::InvocationMetadataContext::for_source_input(
            registry,
            input,
            config,
            Some(input.unit_profile()),
        ),
        None => Some(metadata),
    }
}

/// Build source SSA with the supplied availability owner. Missing, foreign or
/// incompatible source input withholds operand metadata; it never selects a
/// standalone catalogue context. Physical cell and contents proofs are separate.
pub(crate) fn build_ssa_with_context_for_entry_and_metadata(
    func: &cfg::Function,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    entry: ResolveContext,
    entry_bindings: Option<&[String]>,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> SsaFunction {
    let metadata = supplied_ssa_metadata(registry, config, metadata);
    let initial = build_ssa_inner(
        func,
        registry,
        config,
        entry.clone(),
        &HashMap::new(),
        metadata,
    );
    let bound: Option<HashSet<_>> = entry_bindings.map(|bindings| {
        bindings
            .iter()
            .filter_map(|name| canonical_variable_key(name, &entry, registry))
            .collect()
    });
    let live = crate::slot_allocation::registry_barrier_live_names(func, &initial, registry);
    let clobbers: RegistryClobberNames = live
        .into_iter()
        .filter_map(|(block, markers)| {
            let markers: HashMap<_, _> = markers
                .into_iter()
                .filter_map(|(index, symbols)| {
                    let statements = &initial.blocks[&block].statements;
                    let preceding_defs = index
                        .checked_sub(1)
                        .and_then(|prior| statements.get(prior))
                        .filter(|prior| {
                            prior.statement.is_executable_invocation()
                                && prior.statement.span() == statements[index].statement.span()
                        })
                        .map(|prior| &prior.defs);
                    // The original invocation's outputs already own fresh versions.
                    let mut names: Vec<_> = symbols
                        .into_iter()
                        .filter(|symbol| {
                            let version = statements[..index]
                                .iter()
                                .rev()
                                .find_map(|stmt| stmt.defs.get(symbol).copied())
                                .or_else(|| {
                                    initial.blocks[&block].entry_versions.get(symbol).copied()
                                })
                                .unwrap_or(0);
                            preceding_defs.is_none_or(|defs| !defs.contains_key(symbol))
                                && bound.as_ref().is_none_or(|bindings| {
                                    version != 0 || bindings.contains(initial.cell_key(*symbol))
                                })
                        })
                        .map(|symbol| initial.cell_key(symbol).clone())
                        .collect();
                    names.sort_by_cached_key(|key| format!("{key:?}"));
                    (!names.is_empty()).then_some((index, names))
                })
                .collect();
            (!markers.is_empty()).then_some((block, markers))
        })
        .collect();
    if clobbers.is_empty() {
        initial
    } else {
        build_ssa_inner(func, registry, config, entry, &clobbers, metadata)
    }
}

fn build_ssa_inner(
    func: &cfg::Function,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    entry: ResolveContext,
    clobbers: &RegistryClobberNames,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> SsaFunction {
    // Complexity guard: skip the O(blocks·vars) phi placement + rename walk
    // for a pathologically large (usually generated) body. Returns a trivial
    // SSA; the compilation-unit builder likewise produces a trivial analysis
    // and flags the function so per-proc diagnostic passes skip it.
    if is_complexity_guarded(func) {
        return SsaFunction::trivial(func.name.clone(), func.entry, func.block_names().to_vec());
    }

    let mut enriched = enrich_instance_option_defs(func, registry);
    if func.name == "::top" {
        let target = enriched.get_or_insert_with(|| func.clone());
        let _ = mirror_top_level_global_option_defs(target, registry);
    }
    let func = enriched.as_ref().unwrap_or(func);

    // 1. Compute dominance information.  Use the Cooper-Harvey-
    //    Kennedy immediate-dominator algorithm directly — it is
    //    O(N) memory, where the set-based `compute_dominators` is
    //    O(N²) and exhausts memory on a single huge generated proc
    //    (tens of thousands of CFG blocks).
    let idom = compute_idom_fast(func);
    let df = compute_dominance_frontier(func, &idom);
    let tree = build_dom_tree(&idom);
    let config = config.nested().normalized();
    let points =
        crate::variable_bindings::build_point_resolve_contexts_with_entry(func, entry, registry);
    let elems = collect_array_elems(func, registry, &points);
    let phi_vars = compute_phi_vars_with_config(
        func,
        &df,
        SsaInvocationContext::supplied(registry, metadata),
        &elems,
        config,
        &points,
        clobbers,
    );

    // 2. Set up rename state: the transient version stacks / counters, the
    // use-scanner, name interner, and per-block outputs (keyed by variable
    // name / version, interned to `Symbol` when blocks are assembled).
    let mut walk = RenameWalk::new(func, config, points, metadata);

    rename_dominator_tree(
        &mut walk, func, &tree, &phi_vars, registry, &elems, clobbers,
    );

    // 4. Assemble SSA blocks.
    let ssa_blocks = assemble_ssa_blocks(func, &phi_vars, &mut walk.interner, &mut walk.out);

    intern_terminator_reads(func, &mut walk, registry, config);
    let read_references = retained_read_references(
        &walk.points,
        &ssa_blocks,
        &walk.interner,
        registry,
        &walk.value_clobbers,
    );
    let array_refresh = array_root_refresh_metadata(
        func.entry,
        &elems,
        &walk.interner,
        &ssa_blocks,
        &walk.points,
        SsaInvocationContext::supplied(registry, metadata),
    );
    let cell_names = walk.interner.names.clone();
    let cell_keys = walk.interner.keys.clone();
    let cell_to_symbol = walk.interner.to_symbol.clone();
    project_ssa_source_names(&mut walk, func);

    SsaFunction {
        name: func.name.clone(),
        entry: func.entry,
        blocks: ssa_blocks,
        value_clobbers: walk.value_clobbers,
        idom,
        dominance_frontier: df
            .into_iter()
            .map(|(k, v)| {
                let mut sorted: Vec<BlockId> = v.into_iter().collect();
                sorted.sort_unstable();
                (k, sorted)
            })
            .collect(),
        dominator_tree: tree,
        block_names: func.block_names().to_vec(),
        var_names: walk.interner.names,
        cell_names,
        cell_keys,
        var_to_symbol: walk.interner.source_symbols,
        cell_to_symbol,
        point_symbols: walk.point_symbols,
        point_contexts: Some(walk.points),
        read_references,
        array_root_refresh_symbols: array_refresh.symbols,
        array_root_refresh_versions: array_refresh.versions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::{Block, Function, Terminator};
    use crate::expr_ast::ExprNode;
    use tcl_lexer::Span;

    fn logical_role_metadata_fixture() -> (
        std::sync::Arc<tcl_registry::model::ContextRegistry>,
        crate::compilation_unit::CompilationUnit,
    ) {
        use std::sync::Arc;
        let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let mut registry = baseline
            .commands()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let mut info = registry.get("info").unwrap().clone();
        info.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
        registry.insert(info);
        let context = Arc::new(baseline.with_command_store(Arc::new(registry)));
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let entry = crate::command_binding::SourceAnalysisEntry::for_logical_source(&input)
            .expect("genuine positive supplied Logical input");
        let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            "info exists selected",
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&entry),
            &input,
        );
        (context, unit)
    }

    #[test]
    fn supplied_ssa_role_scans_keep_actual_availability_grammar_and_missing_input() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Selected source roles are independent of Native NamePolicy and actual execution.
        use crate::registry_invocation::InvocationMetadataContext;
        use std::sync::Arc;
        let (context, unit) = logical_role_metadata_fixture();
        let registry = context.commands();
        let function = &unit.top_level;
        let input = function.source_metadata_input().unwrap();
        let statement = function
            .cfg
            .blocks
            .values()
            .flat_map(|body| &body.statements)
            .find(|statement| statement.tokens().is_some())
            .unwrap();
        let scan = |metadata| {
            let mut scanner = VarReferenceScanner::with_config(
                VarScanOptions {
                    include_var_read_roles: false,
                    ..Default::default()
                },
                function.source_lexer_config(),
            );
            uses_of_classified_in_context(
                statement,
                &mut scanner,
                SsaInvocationContext::supplied(registry, metadata),
            )
        };
        let metadata = function.invocation_metadata_context_for_module(registry, &unit.ir_module);
        assert!(metadata.is_some());
        assert!(
            function
                .ssa
                .point_contexts
                .as_ref()
                .unwrap()
                .before_statement(function.cfg.entry, 0)
                .execution_name_policy
                .is_none()
        );
        assert_eq!(scan(metadata), vec![("selected".into(), UseClass::Name)]);
        assert!(scan(None).is_empty());
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(context.commands())),
        );
        assert!(Arc::ptr_eq(older.commands(), registry));
        let older_input = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            older,
            input.lexer_config(),
        );
        let foreign = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
            input.lexer_config(),
        );
        let mut grammar = input.lexer_config();
        grammar.strict_quoting = !grammar.strict_quoting;
        let changed = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            Arc::clone(&context),
            grammar,
        );
        for withheld in [&older_input, &foreign, &changed] {
            assert!(
                scan(InvocationMetadataContext::for_analysis_input(
                    registry, withheld
                ))
                .is_empty()
            );
        }
    }

    #[test]
    fn supplied_ssa_definition_and_lexical_reads_do_not_reconstruct_catalogue_roles() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let (context, unit) = logical_role_metadata_fixture();
        let registry = context.commands();
        let function = &unit.top_level;
        let statement = function
            .cfg
            .blocks
            .values()
            .flat_map(|body| &body.statements)
            .find(|statement| statement.tokens().is_some())
            .unwrap();
        let unavailable = SsaInvocationContext::supplied(registry, None);
        assert!(defs_of_in_context(statement, Some(unavailable)).is_empty());
        let tokens = statement.tokens().unwrap();
        assert!(unavailable.normal(tokens).is_none());
        assert!(unavailable.resolve(tokens).is_none());
        // Pure word substitution remains lexical even when command roles are withheld.
        let mut scanner = VarReferenceScanner::with_config(
            VarScanOptions {
                include_var_read_roles: false,
                ..Default::default()
            },
            function.source_lexer_config(),
        );
        assert_eq!(
            scanner.scan_word("$lexical [info exists nominal]", registry),
            BTreeSet::from(["lexical".to_owned()])
        );
    }

    fn original_operand_metadata_fixture() -> (
        std::sync::Arc<tcl_registry::model::ContextRegistry>,
        crate::environment_ingress::RetainedNativeUnit,
    ) {
        use std::sync::Arc;
        let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let mut registry = baseline.commands().project_for_profile(profile);
        let mut setter = registry.get("set").unwrap().clone();
        setter.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
        registry.insert(setter);
        let context = Arc::new(baseline.with_command_store(Arc::new(registry)));
        let (owner, captured) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        let entry = crate::command_binding::SourceAnalysisEntry {
            native_entry: Some(Arc::new(captured)),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        };
        let unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
            "set selected VALUE",
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                config: tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
                dialect: Some(profile),
                defer_top_level: false,
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&entry),
            Arc::clone(&context),
        );
        (
            context,
            crate::environment_ingress::RetainedNativeUnit::new(unit, owner),
        )
    }

    fn original_setter_definition_point(
        function: &crate::compilation_unit::FunctionUnit,
    ) -> (BlockId, usize, Symbol) {
        function
            .ssa
            .blocks
            .iter()
            .find_map(|(&block, body)| {
                body.statements
                    .iter()
                    .enumerate()
                    .find_map(|(index, statement)| {
                        statement
                            .defs
                            .keys()
                            .next()
                            .map(|&symbol| (block, index, symbol))
                    })
            })
            .expect("genuine selected original setter definition")
    }

    #[test]
    fn original_definition_subjects_keep_supplied_availability_and_missing_input_separate() {
        // naming.variable.original-ssa-definition-operand
        // docs/design/analysis/name-resolution-proofs/original-ssa-definition-operand.md
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        use crate::registry_invocation::InvocationMetadataContext;
        use std::sync::Arc;
        let (context, unit) = original_operand_metadata_fixture();
        let function = &unit.top_level;
        let (block, index, symbol) = original_setter_definition_point(function);
        let view = SsaSourceView::at_statement(&function.ssa, block, index);
        let registry = context.commands();
        let select = |metadata| {
            view.original_definition_name_with_metadata_context(symbol, registry, metadata)
        };
        let metadata = function.invocation_metadata_context_for_module(registry, &unit.ir_module);
        let original =
            select(metadata).expect("actual original receiver under current availability");
        assert_eq!(original.original_name_input().bytes(), b"selected");
        assert_eq!(original.cell(), function.ssa.cell_key(symbol));
        assert!(select(None).is_none());
        let input = function.source_metadata_input().unwrap();
        let older_context = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(context.commands())),
        );
        assert!(Arc::ptr_eq(older_context.commands(), context.commands()));
        let older = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            older_context,
            input.lexer_config(),
        );
        assert!(
            select(InvocationMetadataContext::for_analysis_input(
                registry, &older
            ))
            .is_none()
        );
        let foreign = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
            input.lexer_config(),
        );
        assert!(
            select(InvocationMetadataContext::for_analysis_input(
                registry, &foreign
            ))
            .is_none()
        );
        let mut changed_config = input.lexer_config();
        changed_config.strict_quoting = !changed_config.strict_quoting;
        assert_ne!(
            changed_config.normalized(),
            input.lexer_config().normalized()
        );
        let changed = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            Arc::clone(&context),
            changed_config,
        );
        assert!(
            select(InvocationMetadataContext::for_analysis_input(
                registry, &changed
            ))
            .is_none()
        );
        let mut unavailable_module = unit.ir_module.clone();
        unavailable_module.source_metadata_input = None;
        assert!(
            select(function.invocation_metadata_context_for_module(registry, &unavailable_module))
                .is_none()
        );
    }

    #[test]
    fn original_operand_symbols_keep_actual_availability_and_source_grammar() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        use crate::registry_invocation::InvocationMetadataContext;
        use std::sync::Arc;
        let (context, unit) = original_operand_metadata_fixture();
        let function = &unit.top_level;
        let input = function.source_metadata_input().unwrap();
        let points = function.ssa.point_contexts.as_ref().unwrap();
        let point = function
            .cfg
            .blocks
            .iter()
            .find_map(|(&block, body)| {
                body.statements
                    .iter()
                    .enumerate()
                    .find_map(|(index, statement)| {
                        statement
                            .is_executable_invocation()
                            .then_some((block, index))
                    })
            })
            .unwrap();
        let symbols = |metadata, config| {
            let metadata = supplied_ssa_metadata(context.commands(), config, metadata);
            let mut walk = RenameWalk::new(&function.cfg, config, points.clone(), metadata);
            let mut symbols = HashMap::new();
            walk.retain_native_operand_symbols(
                point,
                points.context_before(point.0, point.1).unwrap(),
                context.commands(),
                &mut symbols,
            );
            symbols
        };
        let metadata = InvocationMetadataContext::for_analysis_input(context.commands(), input);
        assert_eq!(symbols(metadata, function.source_lexer_config()).len(), 1);
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(context.commands())),
        );
        assert!(Arc::ptr_eq(older.commands(), context.commands()));
        let older_input = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            older,
            input.lexer_config(),
        );
        assert!(
            symbols(
                InvocationMetadataContext::for_analysis_input(context.commands(), &older_input,),
                function.source_lexer_config()
            )
            .is_empty()
        );
        assert!(symbols(None, function.source_lexer_config()).is_empty());
        let foreign = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
            input.lexer_config(),
        );
        assert!(
            symbols(
                InvocationMetadataContext::for_analysis_input(context.commands(), &foreign,),
                function.source_lexer_config()
            )
            .is_empty()
        );
        let mut stale_config = function.source_lexer_config();
        stale_config.strict_quoting = !stale_config.strict_quoting;
        assert!(symbols(metadata, stale_config).is_empty());
    }

    #[test]
    fn destruction_predecessor_keeps_substituted_operand_use_in_def_use() {
        for substituted in [false, true] {
            let cfg = Function::new("::top", "entry");
            let mut ssa = SsaFunction::trivial("::top", cfg.entry, cfg.block_names().to_vec());
            let symbol = ssa.intern_var("x");
            let mut statement = SsaStatement {
                statement: Statement::Call {
                    span: Span::new(0, 8),
                    command: "unset".into(),
                    canonical_command: Some("::unset".into()),
                    args: vec![if substituted { "$x" } else { "x" }.into()],
                    defs: Vec::new(),
                    reads: Vec::new(),
                    reads_own_defs: false,
                    safe_on_uninit: false,
                    tokens: None,
                    foreach_groups: None,
                },
                uses: if substituted {
                    HashMap::from([(symbol, 1)])
                } else {
                    HashMap::new()
                },
                defs: HashMap::from([(symbol, 2)]),
                may_defs: HashSet::from([symbol]),
                destruction_defs: HashSet::from([symbol]),
                quoted_uses: HashSet::new(),
                name_only_uses: HashSet::new(),
            };
            record_destruction_predecessor(
                symbol,
                1,
                &mut statement.uses,
                &mut statement.name_only_uses,
            );
            ssa.blocks.insert(
                cfg.entry,
                SsaBlock {
                    name: "entry".into(),
                    phis: Vec::new(),
                    statements: vec![statement],
                    entry_versions: HashMap::new(),
                    exit_versions: HashMap::new(),
                },
            );
            let chains =
                crate::def_use::build_def_use_chains(&ssa, None, tcl_lexer::LexerConfig::default());
            let uses = chains.uses_of("x", 1);
            assert_eq!(uses.len(), 1);
            assert_eq!(
                uses[0].class,
                if substituted {
                    UseClass::Substituted
                } else {
                    UseClass::Name
                }
            );
            assert_eq!(
                uses[0].kind,
                if substituted {
                    crate::def_use::UseKind::Operand
                } else {
                    crate::def_use::UseKind::VariableName
                }
            );
        }
    }

    /// Intern `name` into `func` and insert a fresh block for it, returning
    /// the [`BlockId`]. The shared test idiom for building a CFG by hand.
    fn block(func: &mut Function, name: &str) -> BlockId {
        let id = func.intern_block(name);
        func.blocks.insert(id, Block::new(name));
        id
    }

    fn make_goto(target: BlockId) -> Terminator {
        Terminator::Goto { target, span: None }
    }

    fn make_branch(cond: &str, t: BlockId, f: BlockId) -> Terminator {
        Terminator::Branch {
            condition: ExprNode::Raw { text: cond.into() },
            true_target: t,
            false_target: f,
            span: None,
            condition_base: None,
        }
    }

    fn make_return() -> Terminator {
        Terminator::Return {
            tokens: None,
            value: None,
            value_word: None,
            span: None,
            expr: None,
            expr_base: None,
            braced: false,
        }
    }

    fn call(span: Span, command: &str, args: &[&str], defs: &[&str]) -> Statement {
        Statement::Call {
            span,
            command: command.into(),
            canonical_command: None,
            args: args.iter().map(|arg| (*arg).into()).collect(),
            defs: defs.iter().map(|name| (*name).into()).collect(),
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        }
    }

    /// `(func, [entry, then, else, end])` for a diamond CFG:
    /// entry → branch → then/else → end → return.
    fn diamond_cfg() -> Function {
        let mut func = Function::new("::test", "entry");
        let entry = func.entry;
        let then = block(&mut func, "then");
        let els = block(&mut func, "else");
        let end = block(&mut func, "end");
        func.blocks.get_mut(&entry).unwrap().terminator = Some(make_branch("$x", then, els));
        func.blocks.get_mut(&then).unwrap().terminator = Some(make_goto(end));
        func.blocks.get_mut(&els).unwrap().terminator = Some(make_goto(end));
        func.blocks.get_mut(&end).unwrap().terminator = Some(make_return());
        func
    }

    /// Build a loop CFG: entry → header → branch → body → header / end
    fn loop_cfg() -> Function {
        let mut func = Function::new("::test", "entry");
        let entry = func.entry;
        let header = block(&mut func, "header");
        let body = block(&mut func, "body");
        let end = block(&mut func, "end");
        func.blocks.get_mut(&entry).unwrap().terminator = Some(make_goto(header));
        func.blocks.get_mut(&header).unwrap().terminator = Some(make_branch("$i < 10", body, end));
        func.blocks.get_mut(&body).unwrap().terminator = Some(make_goto(header));
        func.blocks.get_mut(&end).unwrap().terminator = Some(make_return());
        func
    }

    /// Resolve a block name to its id in `func` (test convenience).
    fn id_of(func: &Function, name: &str) -> BlockId {
        func.block_id(name).expect("block name interned")
    }

    // Data structure tests

    #[test]
    fn phi_construction() {
        let phi = Phi {
            name: Symbol(0),
            version: 3,
            incoming: HashMap::from([(BlockId(1), 1), (BlockId(2), 2)]),
        };
        assert_eq!(phi.name, Symbol(0));
        assert_eq!(phi.version, 3);
        assert_eq!(phi.incoming.len(), 2);
    }

    #[test]
    fn ssa_statement_construction() {
        let stmt = SsaStatement {
            statement: Statement::AssignConst {
                span: Span::new(0, 10),
                name: "x".into(),
                name_braced: false,
                value: "1".into(),
                value_span: None,
            },
            uses: HashMap::new(),
            defs: HashMap::from([(Symbol(0), 1)]),
            may_defs: std::collections::HashSet::new(),
            destruction_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
            name_only_uses: std::collections::HashSet::new(),
        };
        assert_eq!(stmt.defs[&Symbol(0)], 1);
        assert_eq!(stmt.uses.len(), 0);
    }

    #[test]
    fn ssa_block_construction() {
        let block = SsaBlock {
            name: "entry".into(),
            phis: vec![],
            statements: vec![],
            entry_versions: HashMap::new(),
            exit_versions: HashMap::from([(Symbol(0), 1)]),
        };
        assert_eq!(block.name, "entry");
        assert_eq!(block.phis, [] as [crate::ssa::Phi; 0]);
    }

    #[test]
    fn array_relationships_use_physical_kinds_instead_of_key_text() {
        let mut elements = ArrayElems::default();
        elements.note(crate::place::scalar("literal(parenthesis)", "::", false));
        assert!(elements.elements.is_empty());
        let element = crate::place::array_elem(
            "a",
            crate::place::Index::literal("key"),
            "::N(parenthesis)",
            false,
        );
        let element_key = crate::var_resolve::canonical_binding_value_key(&element).unwrap();
        let mut root = element.clone();
        root.kind = crate::place::PlaceKind::ArrayWhole;
        root.index = None;
        let root_key = crate::var_resolve::canonical_binding_value_key(&root).unwrap();
        elements.note(element);
        assert_eq!(elements.root_for_element(&element_key), Some(&root_key));
        assert_eq!(
            expand_defs(std::slice::from_ref(&element_key), &elements),
            vec![element_key, root_key]
        );
    }

    #[test]
    fn array_refresh_preserves_parentheses_in_its_namespace_identity() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for(
            "namespace eval {::N(parenthesis)} {proc p {} {variable a; set a(key) OLD; set a(key) NEW; return $a(key)}}",
            registry,
            false,
        );
        let function = unit.function("::N(parenthesis)::p").unwrap();
        let root = function
            .ssa
            .cell_symbol("::N(parenthesis)::a")
            .expect("qualified physical root");
        assert!(function.ssa.is_array_root_refresh_symbol(root));
        assert_eq!(function.ssa.cell_symbol("::N"), None);
    }

    #[test]
    fn scalar_recreation_does_not_inherit_array_refresh_classification() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let source = "proc f {} {set a(key) OLD; unset a; set a 5; return $a}";
        let unit = crate::compilation_unit::CompilationUnit::build_for(source, registry, false);
        let function = unit.function("::f").unwrap();
        let mut refreshes = HashSet::new();
        let mut scalar_stores = HashSet::new();
        for statement in function
            .ssa
            .blocks
            .values()
            .flat_map(|block| &block.statements)
        {
            for (&symbol, &version) in &statement.defs {
                if function.ssa.is_array_root_refresh_version(symbol, version) {
                    assert!(!function.ssa.is_array_root_refresh_symbol(symbol));
                    if source
                        .get(statement.statement.span().start() as usize..)
                        .is_some_and(|written| written.starts_with("set a(key) OLD"))
                    {
                        refreshes.insert(statement.statement.span().start());
                    }
                }
                if source
                    .get(statement.statement.span().start() as usize..)
                    .is_some_and(|written| written.starts_with("set a 5"))
                {
                    scalar_stores.insert(statement.statement.span().start());
                    assert!(!function.ssa.is_array_root_refresh_version(symbol, version));
                }
            }
        }
        // The element store refreshes its aggregate root, and recreation
        // writes a scalar. Destruction may carry a separate invalidation.
        assert_eq!(refreshes.len(), 1);
        assert_eq!(scalar_stores.len(), 1);
    }

    #[test]
    fn synthetic_array_root_refresh_is_not_dictionary_value_contents() {
        for profile in ["tcl8.6", "jim"] {
            let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
            let unit = crate::compilation_unit::CompilationUnit::build_for(
                "proc f {} {set a(key) OLD; set a(key) NEW; return $a(key)}",
                registry,
                false,
            );
            let function = unit.function("::f").unwrap();
            let refreshes: Vec<_> = function
                .ssa
                .cell_names()
                .iter()
                .enumerate()
                .filter_map(|(index, name)| {
                    let symbol = Symbol(u32::try_from(index).unwrap());
                    function
                        .ssa
                        .is_array_root_refresh_symbol(symbol)
                        .then_some(name)
                })
                .collect();
            if profile == "tcl8.6" {
                assert_eq!(refreshes.len(), 1, "{refreshes:?}");
            } else {
                assert!(refreshes.is_empty(), "{refreshes:?}");
            }
        }
    }

    #[test]
    fn ssa_function_construction() {
        let func = SsaFunction {
            name: "::test".into(),
            entry: BlockId(0),
            blocks: HashMap::new(),
            value_clobbers: HashMap::new(),
            idom: HashMap::new(),
            dominance_frontier: HashMap::new(),
            dominator_tree: HashMap::new(),
            block_names: vec!["entry".into()],
            var_names: Vec::new(),
            cell_names: Vec::new(),
            cell_keys: Vec::new(),
            var_to_symbol: rustc_hash::FxHashMap::default(),
            cell_to_symbol: rustc_hash::FxHashMap::default(),
            point_symbols: HashMap::new(),
            point_contexts: None,
            read_references: HashMap::new(),
            array_root_refresh_symbols: HashSet::new(),
            array_root_refresh_versions: HashSet::new(),
        };
        assert_eq!(func.name, "::test");
    }

    #[test]
    fn var_interner_assigns_first_seen_order_symbols() {
        let mut func = SsaFunction::trivial("::test", BlockId(0), vec!["entry".into()]);
        let x = func.intern_var("x");
        let y = func.intern_var("y");
        assert_eq!(x, Symbol(0));
        assert_eq!(y, Symbol(1));
        // Re-interning a known name returns the existing symbol.
        assert_eq!(func.intern_var("x"), x);
        assert_eq!(func.var_symbol("y"), Some(y));
        assert_eq!(func.var_symbol("missing"), None);
        assert_eq!(func.var_name(x), "x");
        assert_eq!(func.var_names(), ["x".to_string(), "y".to_string()]);
    }

    // defs_of tests

    #[test]
    fn defs_of_assign_const() {
        let stmt = Statement::AssignConst {
            span: Span::new(0, 10),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        };
        assert_eq!(defs_of(&stmt), vec!["x"]);
    }

    #[test]
    fn defs_of_dynamic_target_name_has_no_static_def() {
        // `set $p 1` / `set ${p} 1` write the variable *named by* `$p`, not
        // `p` itself — an opaque place, so there is no static def.
        for name in ["$p", "${p}", "a$b", "[gen]"] {
            let stmt = Statement::AssignValue {
                span: Span::new(0, 10),
                name: name.into(),
                name_braced: false,
                value: "1".into(),
                value_needs_backsubst: false,
                tokens: None,
            };
            assert_eq!(
                defs_of(&stmt).len(),
                0,
                "dynamic target {name:?} must not be a static def"
            );
        }
        // A constant-keyed array element defs its own per-element variable
        // (the rename walk adds the base def alongside); a dynamic key
        // stays on the base.
        let arr = Statement::AssignValue {
            span: Span::new(0, 10),
            name: "arr(idx)".into(),
            name_braced: false,
            value: "1".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        assert_eq!(defs_of(&arr), vec!["arr(idx)"]);
        let dyn_key = Statement::AssignValue {
            span: Span::new(0, 10),
            name: "arr($i)".into(),
            name_braced: false,
            value: "1".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        assert_eq!(defs_of(&dyn_key), vec!["arr"]);
        // Braces suppress substitution: the key is the literal text `$i`.
        let braced = Statement::AssignValue {
            span: Span::new(0, 10),
            name: "arr($i)".into(),
            name_braced: true,
            value: "1".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        assert_eq!(defs_of(&braced), vec!["arr($i)"]);
    }

    #[test]
    fn is_dynamic_write_target_classifies_names() {
        assert!(is_dynamic_write_target("$p", false));
        assert!(is_dynamic_write_target("${p}", false));
        assert!(is_dynamic_write_target("a$b", false));
        assert!(is_dynamic_write_target("[gen]", false));
        assert!(!is_dynamic_write_target("p", false));
        assert!(!is_dynamic_write_target("arr(idx)", false));
        assert!(!is_dynamic_write_target("ns::var", false));
    }

    #[test]
    fn brace_quoted_write_target_is_not_dynamic() {
        // `set {$n} 1` names the literal variable `$n`; the braces suppress
        // every substitution, so nothing about the target is computed.
        // tclsh 9.0.4 / 8.6.14 (identical):
        //   set {$n} v; info exists {$n} → 1 ; info exists n → 0
        assert!(!is_dynamic_write_target("$n", true));
        assert!(!is_dynamic_write_target("${n}", true));
        assert!(!is_dynamic_write_target("[gen]", true));
        assert!(!is_dynamic_write_target("$a(k)", true));
        // TN control: the same spellings *unbraced* are still dynamic.
        assert!(is_dynamic_write_target("$n", false));
        assert!(is_dynamic_write_target("[gen]", false));
    }

    #[test]
    fn defs_of_incr() {
        let stmt = Statement::Incr {
            span: Span::new(0, 10),
            name: "i".into(),
            name_braced: false,
            amount: None,
            safe_on_uninit: false,
        };
        assert_eq!(defs_of(&stmt), vec!["i"]);
    }

    #[test]
    fn defs_of_call_with_defs() {
        let stmt = Statement::Call {
            span: Span::new(0, 20),
            command: "lappend".into(),
            canonical_command: None,
            args: vec!["list".into(), "item".into()],
            defs: vec!["list".into()],
            reads: vec![],
            reads_own_defs: true,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        assert_eq!(defs_of(&stmt), vec!["list"]);
    }

    #[test]
    fn defs_of_call_no_defs() {
        let stmt = Statement::Call {
            span: Span::new(0, 10),
            command: "puts".into(),
            canonical_command: None,
            args: vec!["hello".into()],
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        assert_eq!(defs_of(&stmt), [] as [std::string::String; 0]);
    }

    #[test]
    fn defs_of_return() {
        let stmt = Statement::Return {
            tokens: None,
            span: Span::new(0, 10),
            value: Some("1".into()),
            value_word: None,
            expr: None,
            expr_base: None,
            command_binding: None,
            braced: false,
        };
        assert_eq!(defs_of(&stmt), [] as [std::string::String; 0]);
    }

    #[test]
    fn defs_of_barrier_trace() {
        let stmt = Statement::Barrier {
            span: Span::new(0, 30),
            reason: "trace".into(),
            command: "trace".into(),
            canonical_command: None,
            args: vec!["add".into(), "variable".into(), "$x".into()],
            tokens: None,
        };
        assert_eq!(defs_of(&stmt), vec!["x"]);
    }

    #[test]
    fn defs_of_barrier_dict_for() {
        let stmt = Statement::Barrier {
            span: Span::new(0, 30),
            reason: "dict for".into(),
            command: "dict::for".into(),
            canonical_command: None,
            args: vec!["k v".into(), "$d".into()],
            tokens: None,
        };
        assert_eq!(defs_of(&stmt), vec!["k", "v"]);
    }

    /// Loop-header barriers resolve via the registry's `loop_list_header`
    /// flag — the ensemble-rewritten spellings work, and a user proc that
    /// merely ENDS in `::for` does not have its first argument misread as
    /// loop variables (the old suffix match did exactly that).
    #[test]
    fn defs_of_barrier_loop_header_via_registry() {
        let reg = CommandRegistry::build_default();
        let barrier = |command: &str| Statement::Barrier {
            span: Span::new(0, 30),
            reason: "loop".into(),
            command: command.into(),
            canonical_command: None,
            args: vec!["k v".into(), "$d".into()],
            tokens: None,
        };
        for cmd in ["::tcl::dict::for", "dict::for", "::tcl::dict::map"] {
            assert_eq!(
                defs_of_with_registry(&barrier(cmd), Some(&reg)),
                vec!["k", "v"],
                "{cmd} is a registry loop-list-header subcommand"
            );
        }
        assert_eq!(
            defs_of_with_registry(&barrier("::tcl::array::for"), Some(&reg)),
            vec!["k", "v"],
            "array for shares the loop-list-header shape"
        );
        assert!(
            !defs_of_with_registry(&barrier("my::for"), Some(&reg))
                .iter()
                .any(|d| d == "k" || d == "v"),
            "a user proc ending in ::for must not be misread as a loop header"
        );
    }

    #[test]
    fn defs_of_barrier_loop_header_uses_tcl_list_grammar() {
        let reg = CommandRegistry::build_default();
        let barrier = |var_list: &str| Statement::Barrier {
            span: Span::new(0, 30),
            reason: "loop".into(),
            command: "::tcl::dict::for".into(),
            canonical_command: None,
            args: vec![var_list.into(), "$d".into()],
            tokens: None,
        };

        assert_eq!(
            defs_of_with_registry(
                &barrier(r#"{one name} "two name" three\ name plain"#),
                Some(&reg),
            ),
            vec!["one name", "two name", "three name", "plain"],
        );
        assert_eq!(
            defs_of_with_registry(&barrier("{unterminated"), Some(&reg)).len(),
            0,
            "a malformed var-list fails before defining loop variables"
        );
    }

    /// `tcltest::test` body indices come from the spec's arg-role resolver
    /// (option-keyed `-setup`/`-body`/`-cleanup` values plus the legacy
    /// positional body) via the generic `ArgRole::Body` walk, not a
    /// `command == "test"` special case.
    #[test]
    fn structural_body_indices_tcltest_via_registry() {
        use tcl_registry::ArgRole;
        // This is a role-metadata query, not proof of a loaded source command.
        let mut reg = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
        let unprovided = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
        reg.insert_ambient_package("tcltest", "2.5.8");
        // Option form: -setup and -body values are Body roles; -result's is
        // not. Both the qualified and the exported bare spelling resolve.
        let option_form = [
            "n",
            "d",
            "-setup",
            "{set x 1}",
            "-body",
            "{incr x}",
            "-result",
            "2",
        ];
        let got = reg.arg_indices_for_role("tcltest::test", &option_form, ArgRole::Body);
        assert_eq!(got, vec![3, 5], "option-form bodies via the option model");
        assert_eq!(
            unprovided.arg_indices_for_role("tcltest::test", &option_form, ArgRole::Body),
            vec![3, 5],
            "package assistance advertises roles without proving a loaded handler"
        );
        let unit = crate::compilation_unit::CompilationUnit::build_for(
            "tcltest::test n d -body {set package_only 1}",
            &unprovided,
            false,
        );
        assert!(
            unit.top_level.ssa.blocks.values().all(|block| block
                .statements
                .iter()
                .all(|statement| statement.defs.is_empty())),
            "unprovided package role inventory cannot donate physical definitions"
        );
        // The bare `test` spelling is NOT a registry key: it resolves only
        // through the lowering's namespace-import canonicalisation (which
        // stamps `canonical_command` — threaded into the SSA lookups). A
        // user proc that merely happens to be called `test` must not pick
        // up tcltest body semantics, which the old string match caused.
        assert_eq!(
            reg.arg_indices_for_role("test", &option_form, ArgRole::Body)
                .len(),
            0
        );
        // Legacy positional form: body is the penultimate argument.
        let legacy = ["n", "d", "{incr x}", "1"];
        let got = reg.arg_indices_for_role("tcltest::test", &legacy, ArgRole::Body);
        assert_eq!(got, vec![2], "legacy positional body");
        // Option form with NO body options marks nothing (and must not fall
        // back to the positional branch: `-result` is not a body).
        let no_body = ["n", "d", "-result", "2"];
        let got = reg.arg_indices_for_role("tcltest::test", &no_body, ArgRole::Body);
        assert_eq!(got.len(), 0, "no body options → no body roles: {got:?}");
        // The generic structural walk consumes the same roles (braced-token
        // filtering applies on real token streams; `None` tokens filter all,
        // so only the empty expectation is assertable here).
        let no_body_args: Vec<String> = no_body.iter().map(|s| (*s).to_string()).collect();
        assert_eq!(
            structural_body_indices("tcltest::test", &no_body_args, None, &reg).len(),
            0
        );
    }

    /// `trace add variable` defs route through the registry's
    /// `ArgRole::VarWrite` query rather than a string match.
    #[test]
    fn defs_of_barrier_trace_via_registry() {
        let reg = CommandRegistry::build_default();
        let stmt = Statement::Barrier {
            span: Span::new(0, 30),
            reason: "trace".into(),
            command: "trace".into(),
            canonical_command: None,
            args: vec!["add".into(), "variable".into(), "$x".into()],
            tokens: None,
        };
        assert_eq!(defs_of_with_registry(&stmt, Some(&reg)), vec!["x"]);
    }

    /// `trace add execution` does NOT define a variable — the
    /// command name being traced is not a `VarWrite` target.
    #[test]
    fn defs_of_barrier_trace_add_execution_no_def() {
        let reg = CommandRegistry::build_default();
        let stmt = Statement::Barrier {
            span: Span::new(0, 30),
            reason: "trace".into(),
            command: "trace".into(),
            canonical_command: None,
            args: vec!["add".into(), "execution".into(), "foo".into()],
            tokens: None,
        };
        assert_eq!(
            defs_of_with_registry(&stmt, Some(&reg)),
            [] as [std::string::String; 0]
        );
    }

    /// `global x y z` produces NO defs from the registry path
    /// (`var_scoping` handles the per-arg list).  Without the
    /// `CREATES_DYNAMIC_BARRIER` skip, the role-driven walk would
    /// only mark `x` (partial defs) and miss `y` / `z`.
    #[test]
    fn defs_of_barrier_global_vararg_no_partial_defs() {
        let reg = CommandRegistry::build_default();
        let stmt = Statement::Barrier {
            span: Span::new(0, 30),
            reason: "global".into(),
            command: "global".into(),
            canonical_command: None,
            args: vec!["x".into(), "y".into(), "z".into()],
            tokens: None,
        };
        assert_eq!(
            defs_of_with_registry(&stmt, Some(&reg)),
            [] as [std::string::String; 0]
        );
    }

    /// `variable a b c` — same vararg-list shape as
    /// `global`, same skip.
    #[test]
    fn defs_of_barrier_variable_vararg_no_partial_defs() {
        let reg = CommandRegistry::build_default();
        let stmt = Statement::Barrier {
            span: Span::new(0, 30),
            reason: "variable".into(),
            command: "variable".into(),
            canonical_command: None,
            args: vec!["a".into(), "b".into(), "c".into()],
            tokens: None,
        };
        assert_eq!(
            defs_of_with_registry(&stmt, Some(&reg)),
            [] as [std::string::String; 0]
        );
    }

    // Dominator tests

    #[test]
    fn dominators_linear() {
        // entry → b1 → b2 → return
        let mut func = Function::new("::test", "entry");
        let entry = func.entry;
        let b1 = block(&mut func, "b1");
        let b2 = block(&mut func, "b2");
        func.blocks.get_mut(&entry).unwrap().terminator = Some(make_goto(b1));
        func.blocks.get_mut(&b1).unwrap().terminator = Some(make_goto(b2));
        func.blocks.get_mut(&b2).unwrap().terminator = Some(make_return());

        let dom = compute_dominators(&func);
        assert_eq!(dom[&entry], HashSet::from([entry]));
        assert_eq!(dom[&b1], HashSet::from([entry, b1]));
        assert_eq!(dom[&b2], HashSet::from([entry, b1, b2]));
    }

    #[test]
    fn dominators_diamond() {
        let func = diamond_cfg();
        let entry = func.entry;
        let (then, els, end) = (
            id_of(&func, "then"),
            id_of(&func, "else"),
            id_of(&func, "end"),
        );
        let dom = compute_dominators(&func);

        // entry dominates everything
        for id in func.blocks.keys() {
            assert!(dom[id].contains(&entry));
        }
        // then and else are not dominated by each other
        assert!(!dom[&then].contains(&els));
        assert!(!dom[&els].contains(&then));
        // end is dominated by entry but not by then or else
        assert!(dom[&end].contains(&entry));
        assert!(!dom[&end].contains(&then));
        assert!(!dom[&end].contains(&els));
    }

    #[test]
    fn idom_diamond() {
        let func = diamond_cfg();
        let entry = func.entry;
        let (then, els, end) = (
            id_of(&func, "then"),
            id_of(&func, "else"),
            id_of(&func, "end"),
        );
        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);

        assert_eq!(idom[&entry], None);
        assert_eq!(idom[&then], Some(entry));
        assert_eq!(idom[&els], Some(entry));
        assert_eq!(idom[&end], Some(entry));
    }

    #[test]
    fn idom_loop() {
        let func = loop_cfg();
        let entry = func.entry;
        let (header, body, end) = (
            id_of(&func, "header"),
            id_of(&func, "body"),
            id_of(&func, "end"),
        );
        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);

        assert_eq!(idom[&entry], None);
        assert_eq!(idom[&header], Some(entry));
        assert_eq!(idom[&body], Some(header));
        assert_eq!(idom[&end], Some(header));
    }

    #[test]
    fn compute_idom_fast_matches_reference() {
        // The production CHK path (`compute_idom_fast`) must produce
        // exactly the same immediate dominators as the set-based
        // reference (`compute_idom` over `compute_dominators`) across
        // linear / diamond / loop shapes.
        let mut linear = Function::new("::test", "entry");
        let entry = linear.entry;
        let b1 = block(&mut linear, "b1");
        let b2 = block(&mut linear, "b2");
        linear.blocks.get_mut(&entry).unwrap().terminator = Some(make_goto(b1));
        linear.blocks.get_mut(&b1).unwrap().terminator = Some(make_goto(b2));
        linear.blocks.get_mut(&b2).unwrap().terminator = Some(make_return());

        for func in [linear, diamond_cfg(), loop_cfg()] {
            let reference = compute_idom(&func, &compute_dominators(&func));
            let fast = compute_idom_fast(&func);
            assert_eq!(fast, reference, "CHK idom diverged for {:?}", func.entry);
        }
    }

    #[test]
    fn compute_idom_fast_handles_long_chain_without_blowup() {
        // A long chain of `if`-style diamonds (the shape a big
        // generated dispatch proc lowers to) must compute quickly via
        // CHK — this is the regression for the analyser stalling /
        // OOMing on machine-generated files.
        let mut func = Function::new("::big", "b0");
        let n = 4000;
        for i in 0..n {
            let cur = func.intern_block(format!("b{i}"));
            func.blocks
                .entry(cur)
                .or_insert_with(|| Block::new(format!("b{i}")));
            let then = block(&mut func, &format!("t{i}"));
            let next = block(&mut func, &format!("b{}", i + 1));
            func.blocks.get_mut(&then).unwrap().terminator = Some(make_return());
            func.blocks.get_mut(&cur).unwrap().terminator = Some(make_branch("c", then, next));
        }
        let bn = id_of(&func, &format!("b{n}"));
        func.blocks.get_mut(&bn).unwrap().terminator = Some(make_return());
        let idom = compute_idom_fast(&func);
        // Each chain block's idom is the previous chain block.
        assert_eq!(idom[&id_of(&func, "b1")], Some(id_of(&func, "b0")));
        assert_eq!(idom[&bn], Some(id_of(&func, &format!("b{}", n - 1))));
        assert_eq!(idom[&id_of(&func, "b0")], None);
    }

    #[test]
    fn dominance_frontier_diamond() {
        let func = diamond_cfg();
        let entry = func.entry;
        let (then, els, end) = (
            id_of(&func, "then"),
            id_of(&func, "else"),
            id_of(&func, "end"),
        );
        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);
        let df = compute_dominance_frontier(&func, &idom);

        // then and else have "end" in their dominance frontier
        assert!(df[&then].contains(&end));
        assert!(df[&els].contains(&end));
        // entry has no dominance frontier
        assert_eq!(df[&entry].len(), 0);
    }

    #[test]
    fn dominance_frontier_loop() {
        let func = loop_cfg();
        let entry = func.entry;
        let (header, body) = (id_of(&func, "header"), id_of(&func, "body"));
        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);
        let df = compute_dominance_frontier(&func, &idom);

        // body has "header" in its dominance frontier (back edge)
        assert!(df[&body].contains(&header));
        // entry strictly dominates header, so header is NOT in entry's DF
        assert_eq!(
            df[&entry].len(),
            0,
            "entry's DF should be empty; got {:?}",
            df[&entry]
        );
    }

    #[test]
    fn dom_tree_diamond() {
        let func = diamond_cfg();
        let entry = func.entry;
        let (then, els, end) = (
            id_of(&func, "then"),
            id_of(&func, "else"),
            id_of(&func, "end"),
        );
        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);
        let tree = build_dom_tree(&idom);

        // entry's children include then, else, end (all directly dominated)
        let entry_children = &tree[&entry];
        assert!(entry_children.contains(&els));
        assert!(entry_children.contains(&end));
        assert!(entry_children.contains(&then));
    }

    // Phi placement tests

    #[test]
    fn phi_vars_diamond_with_defs() {
        // x defined in both then and else → phi needed at end
        let mut func = diamond_cfg();
        let entry = func.entry;
        let (then, els, end) = (
            id_of(&func, "then"),
            id_of(&func, "else"),
            id_of(&func, "end"),
        );
        func.blocks
            .get_mut(&then)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(10, 20),
                name: "x".into(),
                name_braced: false,
                value: "1".into(),
                value_span: None,
            });
        func.blocks
            .get_mut(&els)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(30, 40),
                name: "x".into(),
                name_braced: false,
                value: "2".into(),
                value_span: None,
            });

        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);
        let df = compute_dominance_frontier(&func, &idom);
        let phi = compute_phi_vars(
            &func,
            &df,
            &CommandRegistry::build_default(),
            &ArrayElems::default(),
        );

        let x = crate::var_resolve::canonical_literal_variable_key(
            "x",
            &ResolveContext::for_function(&func.name),
            &CommandRegistry::build_default(),
        )
        .unwrap();
        assert!(phi[&end].contains(&x), "x should need a phi at 'end'");
        assert!(
            !phi[&entry].contains(&x),
            "x should not need a phi at entry"
        );
    }

    #[test]
    fn phi_vars_single_def_no_phi() {
        // x defined only in entry → no phi needed anywhere
        let mut func = diamond_cfg();
        let entry = func.entry;
        func.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(0, 10),
                name: "x".into(),
                name_braced: false,
                value: "1".into(),
                value_span: None,
            });

        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);
        let df = compute_dominance_frontier(&func, &idom);
        let phi = compute_phi_vars(
            &func,
            &df,
            &CommandRegistry::build_default(),
            &ArrayElems::default(),
        );

        let x = crate::var_resolve::canonical_literal_variable_key(
            "x",
            &ResolveContext::for_function(&func.name),
            &CommandRegistry::build_default(),
        )
        .unwrap();
        for vars in phi.values() {
            assert!(!vars.contains(&x), "x should not need a phi anywhere");
        }
    }

    #[test]
    fn phi_vars_loop_def() {
        // i defined in entry and body → phi at header
        let mut func = loop_cfg();
        let entry = func.entry;
        let (header, body) = (id_of(&func, "header"), id_of(&func, "body"));
        func.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(0, 10),
                name: "i".into(),
                name_braced: false,
                value: "0".into(),
                value_span: None,
            });
        func.blocks
            .get_mut(&body)
            .unwrap()
            .statements
            .push(Statement::Incr {
                span: Span::new(30, 40),
                name: "i".into(),
                name_braced: false,
                amount: None,
                safe_on_uninit: false,
            });

        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);
        let df = compute_dominance_frontier(&func, &idom);
        let phi = compute_phi_vars(
            &func,
            &df,
            &CommandRegistry::build_default(),
            &ArrayElems::default(),
        );

        let i = crate::var_resolve::canonical_literal_variable_key(
            "i",
            &ResolveContext::for_function(&func.name),
            &CommandRegistry::build_default(),
        )
        .unwrap();
        assert!(phi[&header].contains(&i), "i should need a phi at 'header'");
    }

    #[test]
    fn phi_vars_no_defs_no_phis() {
        let func = diamond_cfg();
        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);
        let df = compute_dominance_frontier(&func, &idom);
        let phi = compute_phi_vars(
            &func,
            &df,
            &CommandRegistry::build_default(),
            &ArrayElems::default(),
        );

        for vars in phi.values() {
            assert_eq!(vars.len(), 0, "no defs → no phis");
        }
    }

    // uses_of tests

    fn default_registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    #[test]
    fn uses_of_assign_const() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let stmt = Statement::AssignConst {
            span: Span::new(0, 10),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        };
        let uses = uses_of(&stmt, &mut scanner, &reg);
        assert_eq!(uses.len(), 0, "constant assignment reads nothing");
    }

    #[test]
    fn uses_of_assign_value_with_var() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions {
            include_var_read_roles: true,
            recurse_cmd_substitutions: true,
            include_reads_before_write: false,
            element_qualified: false,
        });
        let stmt = Statement::AssignValue {
            span: Span::new(0, 15),
            name: "y".into(),
            name_braced: false,
            value: "$x".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        let uses = uses_of(&stmt, &mut scanner, &reg);
        assert!(uses.contains(&"x".to_string()), "should read $x");
        assert!(
            !uses.contains(&"y".to_string()),
            "should not read $y (it's defined)"
        );
    }

    #[test]
    fn uses_of_incr() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let stmt = Statement::Incr {
            span: Span::new(0, 10),
            name: "i".into(),
            name_braced: false,
            amount: None,
            safe_on_uninit: false,
        };
        let uses = uses_of(&stmt, &mut scanner, &reg);
        // incr reads and writes — reads_own_def
        assert!(uses.contains(&"i".to_string()), "incr reads the variable");
    }

    #[test]
    fn uses_of_return_with_value() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let stmt = Statement::Return {
            tokens: None,
            span: Span::new(0, 15),
            value: Some("$result".into()),
            value_word: None,
            expr: None,
            expr_base: None,
            command_binding: None,
            braced: false,
        };
        let uses = uses_of(&stmt, &mut scanner, &reg);
        assert!(uses.contains(&"result".to_string()));
    }

    #[test]
    fn uses_of_expr_eval() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let stmt = Statement::ExprEval {
            span: Span::new(0, 20),
            command_binding: tcl_runtime_api::CommandBindingIdentity::new("expr", "expr"),
            expr: ExprNode::Binary {
                op: crate::expr_ast::BinOp::Add,
                left: Box::new(ExprNode::Var {
                    text: "$a".into(),
                    name: "a".into(),
                    start: 0,
                    end: 2,
                }),
                right: Box::new(ExprNode::Var {
                    text: "$b".into(),
                    name: "b".into(),
                    start: 5,
                    end: 7,
                }),
            },
            expr_base: None,
        };
        let uses = uses_of(&stmt, &mut scanner, &reg);
        assert!(uses.contains(&"a".to_string()));
        assert!(uses.contains(&"b".to_string()));
    }

    // UseClass classification

    /// A `Call` whose argument words are `args`, with per-word token kinds
    /// derived from the word text: `{…}` lexes to `Str` (brace-quoted, the one
    /// form Tcl leaves wholly unsubstituted), everything else to `Esc`.
    fn call_with_words(command: &str, args: &[&str]) -> Statement {
        let source = std::iter::once(command)
            .chain(args.iter().copied())
            .collect::<Vec<_>>()
            .join(" ");
        let segment = crate::segmenter::segment_commands(&source).remove(0);
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(&source),
            tcl_lexer::LexerConfig::default(),
            &segment,
        );
        Statement::Call {
            span: segment.span,
            command: command.to_owned(),
            canonical_command: None,
            args: segment.texts.into_iter().skip(1).collect(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(tokens),
            foreach_groups: None,
        }
    }

    fn classify(stmt: &Statement, reg: &CommandRegistry, name: &str) -> Option<UseClass> {
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        uses_of_classified(stmt, &mut scanner, reg)
            .into_iter()
            .find(|(n, _)| n == name)
            .map(|(_, class)| class)
    }

    /// A braced word at a role the callee does **not** evaluate in this frame
    /// is a `Quoted` use: still recorded (liveness must assume it may be
    /// evaluated later) but not a read here.
    /// tclsh-proof: tclsh8.6.14 — `puts {$y}` prints `$y` with `y` undefined.
    #[test]
    fn uses_of_classified_braced_data_word_is_quoted() {
        let reg = default_registry();
        let stmt = call_with_words("puts", &["{$y}"]);
        assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Quoted));
    }

    /// The same name in an unbraced word is a definite read.
    #[test]
    fn uses_of_classified_unbraced_word_is_substituted() {
        let reg = default_registry();
        let stmt = call_with_words("puts", &["$y"]);
        assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Substituted));
    }

    /// A braced `Expr`-role word really does substitute — `expr` evaluates it
    /// against the caller's variables — so it stays `Substituted`.
    #[test]
    fn uses_of_classified_braced_expr_word_is_substituted() {
        let reg = default_registry();
        let stmt = call_with_words("expr", &["{$a + $b}"]);
        assert_eq!(classify(&stmt, &reg, "a"), Some(UseClass::Substituted));
        assert_eq!(classify(&stmt, &reg, "b"), Some(UseClass::Substituted));
    }

    /// A command the registry does not describe carries no role information,
    /// so its braced word is *unclassified*: it may be a script that runs in
    /// this frame. A name the word sets itself is that script's own local —
    /// the un-hooked definer shape — so it is `Quoted`.
    #[test]
    fn uses_of_classified_unknown_definer_body_local_is_quoted() {
        let reg = default_registry();
        let stmt = call_with_words(
            "mydefiner",
            &["::foo::bar", "{optlist}", "{set y 1; return $y}"],
        );
        assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Quoted));
    }

    /// TN control — a name the unclassified word does **not** set stays a
    /// read: a wrapper handing the word to an `uplevel`-ing worker really does
    /// evaluate it in this frame, and tclsh errors on the unset name.
    #[test]
    fn uses_of_classified_unknown_command_free_read_stays_substituted() {
        let reg = default_registry();
        let stmt = call_with_words("wrapper", &["myf", "{puts $myf}"]);
        assert_eq!(classify(&stmt, &reg, "myf"), Some(UseClass::Substituted));
    }

    #[test]
    fn source_ownership_classification_uses_the_retained_lexical_config() {
        // naming.variable.ssa-retained-lexical-config
        // docs/design/analysis/name-resolution-proofs/variable-ssa-retained-lexical-config.md
        let registry = default_registry();
        let tcl = tcl_lexer::LexerConfig::for_dialect("tcl9.0");
        let jim = tcl_lexer::LexerConfig {
            base_offset: 87,
            base_line: 4,
            base_col: 12,
            leading_bom: tcl_lexer::LeadingBom::Skip,
            ..tcl_lexer::LexerConfig::for_dialect("jim")
        };
        // These unchanged, brace-balanced source words exercise command
        // separators and list parsing. They are lexical classification controls,
        // not a proof that the unknown wrapper executes either body.
        for (body, tcl_class, jim_class) in [
            (
                "{set\u{000b}x 1; puts $x}",
                UseClass::Quoted,
                UseClass::Substituted,
            ),
            (
                "{foreach \"{x}y\" {1 2} {}; puts $x}",
                UseClass::Substituted,
                UseClass::Quoted,
            ),
        ] {
            let statement = call_with_words("wrapper", &[body]);
            for (config, expected) in [(tcl, tcl_class), (jim, jim_class)] {
                let mut scanner =
                    VarReferenceScanner::with_config(VarScanOptions::default(), config);
                let class = uses_of_classified(&statement, &mut scanner, &registry)
                    .into_iter()
                    .find(|(name, _)| name == "x")
                    .map(|(_, class)| class);
                assert_eq!(class, Some(expected), "{body:?}, {config:?}");
                assert_eq!(scanner.lexer_config(), config.nested().normalized());
            }
        }
    }

    #[test]
    fn nested_source_expression_roles_use_the_retained_lexical_config() {
        // naming.variable.ssa-retained-lexical-config
        // docs/design/analysis/name-resolution-proofs/variable-ssa-retained-lexical-config.md
        let registry = default_registry();
        let statement = call_with_words("puts", &["[expr\u{000b} {$nested}]"]);
        for (config, expected) in [
            (tcl_lexer::LexerConfig::for_dialect("tcl9.0"), true),
            (tcl_lexer::LexerConfig::for_dialect("jim"), false),
        ] {
            // Jim's actual head is `expr` followed by a vertical-tab byte.
            // Registry source roles cannot replace that head with `expr`.
            let mut scanner = VarReferenceScanner::with_config(VarScanOptions::default(), config);
            let names = uses_of_classified(&statement, &mut scanner, &registry);
            assert_eq!(
                names
                    .iter()
                    .any(|(name, class)| name == "nested" && *class == UseClass::Substituted),
                expected,
                "{config:?}: {names:?}",
            );
        }
    }

    /// A name reached both ways in one statement is a definite read.
    #[test]
    fn uses_of_classified_substituted_wins_over_quoted() {
        let reg = default_registry();
        let stmt = call_with_words("puts", &["{$y}", "$y"]);
        assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Substituted));
    }

    /// A braced `return` value is literal.
    /// tclsh-proof: tclsh8.6.14 — `proc f {} { return {$y} }; puts [f]` prints
    /// `$y` with `y` undefined.
    #[test]
    fn uses_of_classified_braced_return_value_is_quoted() {
        let reg = default_registry();
        let stmt = Statement::Return {
            tokens: None,
            span: Span::new(0, 15),
            value: Some("$y".into()),
            value_word: None,
            expr: None,
            expr_base: None,
            command_binding: None,
            braced: true,
        };
        assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Quoted));
    }

    /// Build a one-arm opaque `switch` around `body`, with `pattern` as the
    /// arm's pattern and the canonical braced arm block.
    fn opaque_switch(subject: &str, pattern: &str, body: crate::ir::Script) -> Statement {
        Statement::Switch {
            subject_braced: false,
            raw_arg_braced: Vec::new(),
            span: Span::new(0, 40),
            subject: subject.into(),
            subject_span: Span::new(0, 1),
            arms: vec![crate::ir::SwitchArm {
                pattern: pattern.into(),
                pattern_braced: true,
                pattern_span: Span::new(0, 1),
                body: Some(body),
                body_span: Some(Span::new(0, 1)),
                fallthrough: false,
            }],
            default_body: None,
            default_span: None,
            mode: crate::ir::SwitchMode::Glob,
            nocase: false,
            raw_args: Vec::new(),
            patterns_braced: true,
        }
    }

    /// An arm body is the one script that reaches SSA
    /// un-lowered, and its reads must arrive classified exactly as the same
    /// word would be outside the arm. A braced data word is `Quoted`.
    /// tclsh-proof: tclsh 9.0.4 — `proc f {z} { switch -glob $z { a* { puts
    /// {$b} } } }; f abc` prints `$b` with `b` undefined.
    #[test]
    fn uses_of_classified_opaque_switch_arm_braced_data_word_is_quoted() {
        let reg = default_registry();
        let mut body = crate::ir::Script::new();
        body.statements.push(call_with_words("puts", &["{$y}"]));
        let stmt = opaque_switch("$z", "a*", body);
        assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Quoted));
        // The subject is a genuine read of this frame.
        assert_eq!(classify(&stmt, &reg, "z"), Some(UseClass::Substituted));
    }

    /// TP control — the substituting spelling of the same arm word stays a
    /// definite read, so the classification is threaded, not dropped.
    #[test]
    fn uses_of_classified_opaque_switch_arm_unbraced_word_is_substituted() {
        let reg = default_registry();
        let mut body = crate::ir::Script::new();
        body.statements.push(call_with_words("puts", &["$y"]));
        let stmt = opaque_switch("$z", "a*", body);
        assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Substituted));
    }

    /// A `Statement::Foreach` inside an opaque arm is walked by
    /// `reads_in_stmt` rather than lowered, so `ForeachIterator::list_braced`
    /// is the extra fact that walk needs: a braced value word is
    /// literal list text, recorded `Quoted` so liveness still honours it.
    #[test]
    fn uses_of_classified_opaque_switch_arm_braced_foreach_list_is_quoted() {
        let reg = default_registry();
        let mut body = crate::ir::Script::new();
        body.statements.push(Statement::Foreach {
            span: Span::new(0, 20),
            iterators: vec![crate::ir::ForeachIterator {
                vars: vec!["n".into()],
                list_arg: "a $y c".into(),
                list_braced: true,
            }],
            body: crate::ir::Script::new(),
            body_span: Span::new(0, 1),
            is_lmap: false,
            raw_args: Vec::new(),
            is_dict_iteration: false,
            is_array_iteration: false,
            raw_tokens: None,
        });
        let stmt = opaque_switch("$z", "a*", body);
        assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Quoted));
    }

    /// TP control for the loop value word — the substituting spelling is a
    /// definite read even inside an opaque arm.
    #[test]
    fn uses_of_classified_opaque_switch_arm_substituted_foreach_list_is_substituted() {
        let reg = default_registry();
        let mut body = crate::ir::Script::new();
        body.statements.push(Statement::Foreach {
            span: Span::new(0, 20),
            iterators: vec![crate::ir::ForeachIterator {
                vars: vec!["n".into()],
                list_arg: "a $y c".into(),
                list_braced: false,
            }],
            body: crate::ir::Script::new(),
            body_span: Span::new(0, 1),
            is_lmap: false,
            raw_args: Vec::new(),
            is_dict_iteration: false,
            is_array_iteration: false,
            raw_tokens: None,
        });
        let stmt = opaque_switch("$z", "a*", body);
        assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Substituted));
    }

    /// A pattern from the canonical braced arm block is a
    /// literal list element; supplied as separate words it substitutes.
    #[test]
    fn uses_of_classified_switch_pattern_class_follows_patterns_braced() {
        let reg = default_registry();
        let braced = opaque_switch("$z", "$p*", crate::ir::Script::new());
        assert_eq!(classify(&braced, &reg, "p"), Some(UseClass::Quoted));
        let mut worded = opaque_switch("$z", "$p*", crate::ir::Script::new());
        if let Statement::Switch {
            patterns_braced, ..
        } = &mut worded
        {
            *patterns_braced = false;
        }
        assert_eq!(classify(&worded, &reg, "p"), Some(UseClass::Substituted));
    }

    // build_ssa tests

    #[test]
    fn build_ssa_linear() {
        let reg = default_registry();
        // entry: set x 1; set y $x; return
        let mut func = Function::new("::test", "entry");
        let entry = func.entry;
        func.blocks.get_mut(&entry).unwrap().statements = vec![
            Statement::AssignConst {
                span: Span::new(0, 7),
                name: "x".into(),
                name_braced: false,
                value: "1".into(),
                value_span: None,
            },
            Statement::AssignValue {
                span: Span::new(8, 16),
                name: "y".into(),
                name_braced: false,
                value: "$x".into(),
                value_needs_backsubst: false,
                tokens: None,
            },
        ];
        func.blocks.get_mut(&entry).unwrap().terminator = Some(make_return());

        let ssa = build_ssa(&func, &reg);
        assert_eq!(ssa.name, "::test");
        assert_eq!(ssa.entry, entry);

        let sx = ssa.var_symbol("x").expect("x interned");
        let sy = ssa.var_symbol("y").expect("y interned");
        let entry_blk = &ssa.blocks[&entry];
        // First statement (set x 1): defs x=1
        assert_eq!(entry_blk.statements[0].defs.get(&sx), Some(&1));
        // Second statement (set y $x): uses x=1, defs y=1
        assert_eq!(entry_blk.statements[1].uses.get(&sx), Some(&1));
        assert_eq!(entry_blk.statements[1].defs.get(&sy), Some(&1));
    }

    #[test]
    fn catalogue_widget_option_roles_do_not_donate_physical_store_proof() {
        let registry = tcl_registry::model::ingress::static_context_for("tk").commands();
        let arguments = [".e", "-textvariable", "value"];
        let option = registry
            .get("entry")
            .unwrap()
            .options
            .iter()
            .find(|option| option.name == "-textvariable")
            .unwrap();
        assert!(
            matches!(option.value, tcl_registry::hover::OptionValue::Takes(argument)
            if argument.role == tcl_registry::ArgRole::VarWrite)
        );
        assert_eq!(
            registry.option_variable_scope("entry", &arguments, 2, registry.own_surface_query()),
            Some(tcl_registry::VariableScope::Global)
        );
        let unit = crate::compilation_unit::CompilationUnit::build_for(
            "entry .e -textvariable value",
            registry,
            false,
        );
        assert!(
            unit.top_level
                .ssa
                .blocks
                .values()
                .flat_map(|block| &block.statements)
                .all(|statement| statement.defs.is_empty())
        );
    }

    #[test]
    fn catalogue_instance_options_do_not_manufacture_live_widget_handlers() {
        let registry = tcl_registry::model::ingress::static_context_for("tk").commands();
        let invocation = registry
            .resolve_instance_invocation(
                "entry",
                ".e",
                &["configure", "-textvariable", "value"],
                registry.own_surface_query(),
            )
            .expect("catalogue instance assistance");
        assert!(
            invocation
                .semantics
                .traits
                .contains(tcl_registry::Traits::CONFIGURES_INSTANCE_OPTIONS)
        );
        let unit = crate::compilation_unit::CompilationUnit::build_for(
            "entry .e; .e configure -textvariable value",
            registry,
            false,
        );
        assert!(
            unit.top_level
                .ssa
                .blocks
                .values()
                .flat_map(|block| &block.statements)
                .all(|statement| statement.defs.is_empty())
        );
    }

    #[test]
    fn build_ssa_top_level_ordinary_global_set_has_no_bare_alias() {
        let reg = default_registry();
        let mut func = Function::new("::top", "entry");
        let entry = func.entry;
        func.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(0, 13),
                name: "::value".into(),
                name_braced: false,
                value: "1".into(),
                value_span: None,
            });
        func.blocks.get_mut(&entry).unwrap().terminator = Some(make_return());

        let ssa = build_ssa(&func, &reg);
        let absolute = ssa.var_symbol("::value").expect("global spelling interned");
        assert_eq!(
            ssa.blocks[&entry].statements[0].defs.get(&absolute),
            Some(&1)
        );
        assert_eq!(ssa.var_symbol("value"), None);
    }

    #[test]
    fn qualified_catalogue_option_keeps_scope_assistance_without_a_store() {
        let registry = tcl_registry::model::ingress::static_context_for("tk").commands();
        let arguments = [".e", "-textvariable", "::ns::value"];
        assert_eq!(
            registry.option_variable_scope("entry", &arguments, 2, registry.own_surface_query()),
            Some(tcl_registry::VariableScope::Global)
        );
        let unit = crate::compilation_unit::CompilationUnit::build_for(
            "namespace eval ns {}; entry .e -textvariable ::ns::value",
            registry,
            false,
        );
        assert!(
            unit.top_level
                .ssa
                .blocks
                .values()
                .flat_map(|block| &block.statements)
                .all(|statement| statement.defs.is_empty())
        );
        assert_eq!(unit.top_level.ssa.var_symbol("value"), None);
    }

    #[test]
    fn build_ssa_global_option_without_absolute_def_has_no_bare_alias() {
        let reg = default_registry();
        let mut func = Function::new("::top", "entry");
        let entry = func.entry;
        func.blocks.get_mut(&entry).unwrap().statements.push(call(
            Span::new(0, 34),
            "entry",
            &[".e", "-textvariable", "value"],
            &[],
        ));
        func.blocks.get_mut(&entry).unwrap().terminator = Some(make_return());

        let ssa = build_ssa(&func, &reg);
        assert_eq!(ssa.var_symbol("::value"), None);
        assert_eq!(ssa.var_symbol("value"), None);
    }

    #[test]
    fn procedure_widget_catalogue_does_not_write_global_or_local_values() {
        let registry = tcl_registry::model::ingress::static_context_for("tk").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for(
            "proc make_entry {} {entry .e -textvariable value; .e configure -textvariable value}",
            registry,
            false,
        );
        let function = unit.function("::make_entry").unwrap();
        assert!(
            function
                .ssa
                .blocks
                .values()
                .flat_map(|block| &block.statements)
                .all(|statement| statement.defs.is_empty())
        );
    }

    #[test]
    fn build_ssa_diamond_phi() {
        let reg = default_registry();
        // entry → branch on $x → then: set x 1 → end
        //                       → else: set x 2 → end
        let mut func = diamond_cfg();
        let entry = func.entry;
        let (then, els, end) = (
            id_of(&func, "then"),
            id_of(&func, "else"),
            id_of(&func, "end"),
        );

        // Define x in entry first so it's used in the condition.
        func.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(0, 7),
                name: "x".into(),
                name_braced: false,
                value: "0".into(),
                value_span: None,
            });

        func.blocks
            .get_mut(&then)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(10, 18),
                name: "x".into(),
                name_braced: false,
                value: "1".into(),
                value_span: None,
            });
        func.blocks
            .get_mut(&els)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(20, 28),
                name: "x".into(),
                name_braced: false,
                value: "2".into(),
                value_span: None,
            });
        // Read `x` after the join so it is upward-exposed at `end` — under
        // semi-pruned SSA a phi is placed only for a name with a downstream
        // reader (a dead phi for an unread merge is correctly dropped).
        func.blocks
            .get_mut(&end)
            .unwrap()
            .statements
            .push(Statement::Call {
                span: Span::new(30, 38),
                command: "puts".into(),
                canonical_command: None,
                args: vec!["$x".into()],
                defs: vec![],
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
                foreach_groups: None,
            });

        let ssa = build_ssa(&func, &reg);

        // x is defined in both then and else and read at end, so the end block
        // should have a phi for x.
        let sx = ssa.var_symbol("x").expect("x interned");
        let end_block = &ssa.blocks[&end];
        assert!(
            end_block.phis.iter().any(|phi| phi.name == sx),
            "end block should have a phi for x"
        );

        // The phi should have incoming edges from then and else.
        if let Some(phi) = end_block.phis.iter().find(|p| p.name == sx) {
            assert!(
                phi.incoming.contains_key(&then),
                "phi should have incoming from then"
            );
            assert!(
                phi.incoming.contains_key(&els),
                "phi should have incoming from else"
            );
        }
    }

    #[test]
    fn exact_jim_config_places_and_versions_phi_for_expr_sugar_return() {
        let reg = default_registry();
        let mut func = diamond_cfg();
        let (then, els, end) = (
            id_of(&func, "then"),
            id_of(&func, "else"),
            id_of(&func, "end"),
        );
        for (block, value) in [(then, "1"), (els, "2")] {
            func.blocks
                .get_mut(&block)
                .unwrap()
                .statements
                .push(Statement::AssignConst {
                    span: Span::new(10, 20),
                    name: "a".into(),
                    name_braced: false,
                    value: value.into(),
                    value_span: None,
                });
        }
        func.blocks.get_mut(&end).unwrap().terminator = Some(Terminator::Return {
            tokens: None,
            value: Some("$($a)".into()),
            value_word: None,
            span: None,
            expr: None,
            expr_base: None,
            braced: false,
        });

        // Jim is an environment grammar without a catalogue profile. Its
        // `$()` expression sugar must therefore come from the exact config,
        // not the default registry's profile. Deliberately include file/offset
        // state: SSA re-scans detached words in local nested coordinates.
        let jim = tcl_lexer::LexerConfig {
            base_offset: 91,
            base_line: 7,
            base_col: 13,
            leading_bom: tcl_lexer::LeadingBom::Skip,
            ..tcl_lexer::LexerConfig::for_dialect("jim")
        };
        let ssa = build_ssa_with_config(&func, &reg, jim);
        let a = ssa.var_symbol("a").expect("Jim return read interns a");
        let phi = ssa.blocks[&end]
            .phis
            .iter()
            .find(|phi| phi.name == a)
            .expect("Jim-only return read keeps the merge phi live");
        assert!(phi.incoming.contains_key(&then));
        assert!(phi.incoming.contains_key(&els));

        let def_use = crate::def_use::build_def_use_chains(&ssa, Some(&func), jim);
        let chain = def_use
            .chain_for("a", phi.version)
            .expect("the phi definition has a chain");
        assert!(
            chain
                .uses
                .iter()
                .any(|use_site| use_site.kind == crate::def_use::UseKind::Terminator),
            "the Jim return must read the merged SSA version: {chain:?}"
        );

        let tcl = build_ssa_with_config(&func, &reg, tcl_lexer::LexerConfig::for_dialect("tcl9.0"));
        assert!(
            tcl.var_symbol("a").is_none_or(|symbol| {
                !tcl.blocks[&end].phis.iter().any(|phi| phi.name == symbol)
            }),
            "Tcl must not reinterpret Jim's `$()` expression sugar as a read"
        );
    }

    #[test]
    fn build_ssa_loop() {
        let reg = default_registry();
        // entry: set i 0 → header: branch $i<10 → body: incr i → header
        //                                        → end: return
        let mut func = loop_cfg();
        let entry = func.entry;
        let (header, body) = (id_of(&func, "header"), id_of(&func, "body"));

        func.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(0, 8),
                name: "i".into(),
                name_braced: false,
                value: "0".into(),
                value_span: None,
            });
        func.blocks
            .get_mut(&body)
            .unwrap()
            .statements
            .push(Statement::Incr {
                span: Span::new(20, 28),
                name: "i".into(),
                name_braced: false,
                amount: None,
                safe_on_uninit: false,
            });

        let ssa = build_ssa(&func, &reg);

        // header should have a phi for i (from entry and body).
        let si = ssa.var_symbol("i").expect("i interned");
        let header_blk = &ssa.blocks[&header];
        assert!(
            header_blk.phis.iter().any(|p| p.name == si),
            "header should have a phi for i"
        );
    }

    #[test]
    fn aliased_set_preserves_aliased_nested_expr_read_in_call_ir_for_ssa() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let reg = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let lower = |source: &str| {
            let mut lowerer = crate::lowering::Lowerer::new(&reg);
            lowerer.set_source_analysis_options(crate::command_binding::SourceAnalysisOptions {
                native_entry: Some(&entry),
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            });
            crate::lowering::lower_to_ir_with(lowerer, source)
        };
        let module = lower(
            "set x 1\ninterp alias {} myset {} set\ninterp alias {} e {} expr\nmyset y [e {$x+1}]\n",
        );
        let call = module
            .top_level
            .statements
            .last()
            .expect("aliased set call");
        assert!(
            defs_of_with_registry(call, Some(&reg)).contains(&"y".to_owned()),
            "aliased set must define y: {call:?}"
        );
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        assert!(
            uses_of(call, &mut scanner, &reg).contains(&"x".to_owned()),
            "SSA must consume the proved aliased expression read"
        );
        let failed =
            lower("interp alias {} myset {} set\ninterp alias {} e {} expr\nmyset y [e {$x+1}]\n");
        assert!(
            defs_of_with_registry(failed.top_level.statements.last().unwrap(), Some(&reg))
                .is_empty(),
            "an argument read failure cannot define the setter's output"
        );
    }

    #[test]
    fn setter_source_reads_use_selected_child_expression_owners() {
        // naming.variable.selected-setter-expression-reads
        // docs/design/analysis/name-resolution-proofs/variable-selected-setter-expression-reads.md
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(dialect).unwrap();
            let registry =
                tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
            let (_owner, entry) =
                crate::environment_ingress::captured_native_entry_with_owner(profile);
            for (source, read, expected) in [
                (
                    "set input 3; interp alias {} setter {} set; setter output [expr {$input + 1}]",
                    "input",
                    true,
                ),
                (
                    "proc expr args {return VALUE}; interp alias {} setter {} set; setter output [expr {$missing}]",
                    "missing",
                    false,
                ),
                (
                    "set input 3; interp alias {} calc {} expr; interp alias {} setter {} set; setter output [calc {$input + 1}]",
                    "input",
                    true,
                ),
            ] {
                let mut lowerer = crate::lowering::Lowerer::new(&registry);
                lowerer.set_source_analysis_options(
                    crate::command_binding::SourceAnalysisOptions {
                        native_entry: Some(&entry),
                        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                            profile,
                        )),
                        native_compilation:
                            crate::environment_ingress::authoring_native_compilation(),
                        ..Default::default()
                    },
                );
                let module = crate::lowering::lower_to_ir_with(lowerer, source);
                let call = module
                    .top_level
                    .statements
                    .last()
                    .expect("actual setter call");
                let tokens = module
                    .top_level
                    .retained_source_tokens_for_statement(call)
                    .unwrap_or_else(|| {
                        panic!("{dialect}: original aliased setter carrier missing: {call:?}")
                    });
                assert!(tokens.source_binding.is_some(), "{dialect}: {source}");
                assert!(
                    defs_of_with_registry(call, Some(&registry)).contains(&"output".to_owned()),
                    "{dialect}: actual setter definition is an independent positive: {call:?}",
                );
                let mut scanner = VarReferenceScanner::with_config(
                    VarScanOptions::default(),
                    module.native_lexer_config(),
                );
                let names = uses_of(call, &mut scanner, &registry);
                assert_eq!(
                    names.iter().any(|name| name == read),
                    expected,
                    "{dialect}: {source}: {names:?}"
                );
            }
        }
    }

    #[test]
    fn set_value_reads_preserves_scanner_element_qualification() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions {
            element_qualified: true,
            ..VarScanOptions::default()
        });
        assert!(
            set_value_reads("[::expr {$array(key) + 1}]", None, &mut scanner, &reg)
                .contains("array(key)")
        );
    }

    #[test]
    fn build_ssa_empty_function() {
        let reg = default_registry();
        let mut func = Function::new("::empty", "entry");
        let entry = func.entry;
        func.blocks.get_mut(&entry).unwrap().terminator = Some(make_return());

        let ssa = build_ssa(&func, &reg);
        assert_eq!(ssa.blocks.len(), 1);
        assert_eq!(ssa.blocks[&entry].phis, [] as [crate::ssa::Phi; 0]);
        assert_eq!(
            ssa.blocks[&entry].statements,
            [] as [crate::ssa::SsaStatement; 0]
        );
    }

    #[test]
    fn complexity_guard_skips_oversized_ssa() {
        let reg = default_registry();
        // A small function is below the ceiling and analysed normally.
        let small = Function::new("::small", "entry");
        assert!(!is_complexity_guarded(&small));

        // A function above the block ceiling is guarded: `build_ssa` returns a
        // trivial SSA without running the O(blocks·vars) dominator + phi walk
        // that would cost seconds on a pathological generated body.
        let mut big = Function::new("::big", "b0");
        let b0 = big.entry;
        for i in 0..=COMPLEXITY_GUARD_BLOCKS {
            let id = big.intern_block(format!("b{i}"));
            big.blocks.insert(id, Block::new(format!("b{i}")));
        }
        assert!(big.blocks.len() > COMPLEXITY_GUARD_BLOCKS);
        assert!(is_complexity_guarded(&big));

        let ssa = build_ssa(&big, &reg);
        assert_eq!(ssa.blocks.len(), 0, "guarded SSA must be trivial");
        assert_eq!(ssa.name, "::big");
        assert_eq!(ssa.entry, b0);
    }

    /// `collapsed_extra_defs` recurses once per nested
    /// `If`/`While`/`For`/`Foreach`/`Catch`/`Try`/`Switch` body, with no
    /// depth cap of its own. Transitively
    /// bounded to `MAX_LOWER_NEST_DEPTH` (256) by the lowering pass,
    /// so this is defence-in-depth / consistency with every other
    /// full-tree walker in this crate, not a currently-reproducible crash.
    /// 2000 levels is comfortably past this new cap; the assertion is that
    /// the call returns at all, not what it returns.
    #[test]
    fn deeply_nested_if_survives_collapsed_extra_defs() {
        use crate::ir::{IfClause, Script};

        const DEPTH: usize = 2000;
        let leaf = Statement::AssignConst {
            span: Span::new(0, 0),
            name: "leaf".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        };
        let mut script = Script::from_statements(vec![leaf]);
        for _ in 0..DEPTH {
            script = Script::from_statements(vec![Statement::If {
                span: Span::new(0, 0),
                clauses: vec![IfClause {
                    condition: ExprNode::Raw { text: "1".into() },
                    condition_span: Span::new(0, 0),
                    body: script,
                    body_span: Span::new(0, 0),
                    condition_base: None,
                }],
                else_body: None,
                else_span: None,
            }]);
        }

        let reg = CommandRegistry::build_default();
        let extra = collapsed_extra_defs(&script, &reg, 0);
        // Nothing but the leaf `AssignConst` here is a def source visible to
        // this helper (it only recovers `for`/condition-command-sub defs) —
        // the assertion is that this returns at all without overflowing the
        // stack, not what it returns.
        let _ = extra;
    }

    /// The classified uses of every `Call` in a one-statement proc, lowered
    /// from source so the statement carries the real word snapshot.
    fn classified_call_uses(body: &str) -> Vec<(String, UseClass)> {
        let reg = CommandRegistry::build_default();
        let src = format!("proc f {{tainted}} {{\n {body}\n}}");
        let cu = crate::compilation_unit::CompilationUnit::build_for(&src, &reg, false);
        let fu = cu.function("::f").expect("proc lowered");
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let mut out = Vec::new();
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                if matches!(stmt, Statement::Call { .. }) {
                    out.extend(uses_of_classified(stmt, &mut scanner, &reg));
                }
            }
        }
        out
    }

    /// A braced word of a *nested* command is read exactly as the same word of
    /// a direct call is, when the registry says the callee evaluates it in
    /// this frame. Only a pair of braces separated `[expr $tainted]` (whose
    /// read the enclosing word's own scan already saw) from
    /// `[expr {$tainted}]`, which would otherwise be invisible to every
    /// consumer of the use map.
    #[test]
    fn braced_word_of_a_nested_substitution_is_read_in_an_evaluated_role() {
        let uses = classified_call_uses("puts [expr {$tainted}]");
        assert!(
            uses.contains(&("tainted".to_owned(), UseClass::Substituted)),
            "an `Expr` role's braced word substitutes in this frame: {uses:?}"
        );
    }

    #[test]
    fn conditional_nested_expression_reads_keep_original_tokens_without_entry() {
        let registry = default_registry();
        let source = "proc f {tainted} {puts [expr {$tainted + 1}]}";
        let unit = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);
        let function = unit.function("::f").unwrap();
        let child = function
            .cfg
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .filter_map(Statement::tokens)
            .flat_map(|tokens| {
                crate::word_subst::lifted_calls(Some(tokens), unit.ir_module.lexer_config)
            })
            .find(|call| call.command == "expr")
            .and_then(|call| call.tokens)
            .expect("original nested expression");
        let binding = child.source_binding.as_ref().unwrap();
        assert_eq!(
            binding.runtime_reachability(),
            crate::command_binding::SourceRuntimeReachability::Conditional,
        );
        assert!(binding.proved_execution_target().is_none());
        let scanner = VarReferenceScanner::new(VarScanOptions::default());
        let mut reads = BTreeSet::new();
        scan_selected_expression_roles(&child, &scanner, &registry, &mut reads);
        assert_eq!(reads, BTreeSet::from(["tainted".to_owned()]));

        let mut foreign = child.clone();
        foreign.argv_texts[1] = "$other + 1".to_owned();
        foreign.word_exprs[1] = WordExpr::BracedLiteral {
            text: "$other + 1".to_owned(),
            source: foreign.word_exprs[1].source().clone(),
        };
        reads.clear();
        scan_selected_expression_roles(&foreign, &scanner, &registry, &mut reads);
        assert!(
            reads.is_empty(),
            "foreign source cannot borrow May-read topology"
        );
    }

    /// …and a braced word in a role the callee does *not* evaluate stays the
    /// literal it is. `list` builds a value; `{$tainted}` is one of its
    /// elements, not a script.
    #[test]
    fn braced_word_of_a_nested_substitution_stays_literal_in_a_data_role() {
        let uses = classified_call_uses("puts [list {$tainted}]");
        assert!(
            !uses.contains(&("tainted".to_owned(), UseClass::Substituted)),
            "`list` does not evaluate its arguments in the calling frame: {uses:?}"
        );
    }

    /// A nested command's **body** must not be read as a frame use, even
    /// though a body written on a *statement* is. The difference is the
    /// lowering: a statement's body becomes its own CFG block, so `lmap`'s
    /// loop variable is defined where the body's reads are seen, while a
    /// nested substitution gets no block at all — so its body's reads arrive
    /// with none of its own definitions and `x` looks read-before-set.
    #[test]
    fn body_word_of_a_nested_substitution_is_not_a_frame_read() {
        let uses = classified_call_uses("puts [join [lmap x $tainted {string toupper $x}] ,]");
        assert!(
            !uses.iter().any(|(name, _)| name == "x"),
            "`lmap` binds its own loop variable; the frame never reads it: {uses:?}"
        );
    }

    /// The same for a body whose locals are set inside it — `catch`'s script
    /// declares `inner`, so the enclosing frame neither reads nor defines it.
    #[test]
    fn body_locals_of_a_nested_substitution_are_not_frame_reads() {
        let uses = classified_call_uses("puts [catch {set inner 1; expr {$inner + 1}}]");
        assert!(
            !uses.iter().any(|(name, _)| name == "inner"),
            "a nested body's own local is not a read of this frame: {uses:?}"
        );
    }

    /// The classified uses of every `AssignValue` in a one-statement proc —
    /// the assignment twin of [`classified_call_uses`].
    fn classified_assign_uses(body: &str) -> Vec<(String, UseClass)> {
        let reg = CommandRegistry::build_default();
        let src = format!("proc f {{tainted}} {{\n {body}\n}}");
        let cu = crate::compilation_unit::CompilationUnit::build_for(&src, &reg, false);
        let fu = cu.function("::f").expect("proc lowered");
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let mut out = Vec::new();
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                if matches!(stmt, Statement::AssignValue { .. }) {
                    out.extend(uses_of_classified(stmt, &mut scanner, &reg));
                }
            }
        }
        out
    }

    /// An assignment's value word substitutes by exactly the rules a call's
    /// word does, so the braced body of a nested `[expr …]`
    /// is a read there too. Without it the expression's operands are absent
    /// from `uses` and every consumer of that map — the expr shimmer detector
    /// included — is blind to `set r [list [expr {$tainted + 1}]]`.
    #[test]
    fn braced_word_of_a_nested_substitution_is_read_in_an_assignment_value() {
        let uses = classified_assign_uses("set r [list [expr {$tainted + 1}]]");
        assert!(
            uses.contains(&("tainted".to_owned(), UseClass::Substituted)),
            "an `Expr` role's braced word substitutes in this frame: {uses:?}"
        );
    }

    /// …and the body exclusion holds on the assignment path too — this is the
    /// spelling the repository's own corpus actually contains.
    #[test]
    fn body_word_of_a_nested_substitution_in_an_assignment_is_not_a_frame_read() {
        let uses = classified_assign_uses("set r [join [lmap x $tainted {string toupper $x}] ,]");
        assert!(
            !uses.iter().any(|(name, _)| name == "x"),
            "`lmap` binds its own loop variable; the frame never reads it: {uses:?}"
        );
    }
}

fn additional_read_bindings(
    points: &PointResolveContexts,
    block: BlockId,
    index: usize,
    statement: &Statement,
    context: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<(VariableCellKey, UseClass)> {
    scope_read_bindings(statement, context, registry)
        .into_iter()
        .chain(
            invocation_read_bindings(points, block, index, context, registry)
                .into_iter()
                .map(|name| (name, UseClass::Name)),
        )
        .collect()
}

fn scope_read_bindings(
    statement: &Statement,
    context: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<(VariableCellKey, UseClass)> {
    crate::dictionary_bindings::scope_marker_effects(statement, context, registry)
        .into_iter()
        .flat_map(|effects| effects.reads)
        .filter_map(|place| {
            crate::var_resolve::canonical_binding_value_key(&place)
                .map(|name| (name, UseClass::Name))
        })
        .collect()
}

#[cfg(test)]
mod cell_resolution_tests {
    use super::*;
    use crate::cfg::Terminator;
    use tcl_lexer::Span;

    #[test]
    fn caller_ssa_read_key_uses_typed_activation_identity() {
        // Implementation contract: naming.variable.invocation-caller-ssa-frame
        // docs/design/analysis/name-resolution-proofs/variable-invocation-caller-ssa-frame.md
        let context = ResolveContext::default().in_frame(
            &crate::var_resolve::VariableExecutionFrame::Procedure {
                namespace: "::".to_owned(),
                identity: "caller".to_owned(),
            },
        );
        let caller = VariableCellKey::Activation {
            identity: "caller".to_owned(),
            simple: "shared".into(),
        }
        .with_lifetime(11)
        .with_index("member");
        let foreign = VariableCellKey::Activation {
            identity: "callee".to_owned(),
            simple: "local".into(),
        };
        assert_eq!(
            invocation_read_key_in_context(caller.clone(), &context),
            Some(caller)
        );
        assert!(invocation_read_key_in_context(foreign.clone(), &context).is_none());
        assert!(
            invocation_read_key_in_context(
                foreign.clone(),
                &ResolveContext::default()
                    .in_frame(&crate::var_resolve::VariableExecutionFrame::Global)
            )
            .is_none()
        );
        let written = VariableCellKey::Authored(foreign.compatibility_name());
        assert_eq!(
            invocation_read_key_in_context(written.clone(), &context),
            Some(written)
        );
        let shared = VariableCellKey::Namespace {
            identity: crate::command_binding::SourceNamespaceKey::authored("::"),
            simple: "shared".into(),
        };
        assert_eq!(
            invocation_read_key_in_context(shared.clone(), &context),
            Some(shared)
        );
    }

    #[test]
    fn invocation_read_projection_keeps_physical_inventory_and_root_alias_reads() {
        // Implementation contract: naming.variable.invocation-caller-ssa-frame
        // docs/design/analysis/name-resolution-proofs/variable-invocation-caller-ssa-frame.md
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for (source, foreign) in [
            ("proc child {} {set local 1; set copy $local}; child", true),
            (
                "set shared 1; proc child {} {upvar #0 shared alias; set copy $alias}; child",
                false,
            ),
        ] {
            let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
                source, registry, false, "tcl8.6",
            );
            let ssa = &unit.top_level.ssa;
            let mut seen = false;
            for block in ssa.blocks.values() {
                for statement in &block.statements {
                    let Some(tokens) = statement.statement.tokens() else {
                        continue;
                    };
                    for place in
                        crate::place_bridge::invocation_execution_read_places(tokens, registry)
                    {
                        let Some(key) = crate::var_resolve::canonical_binding_value_key(&place)
                        else {
                            continue;
                        };
                        if key.activation_identity().is_some() != foreign {
                            continue;
                        }
                        seen = true;
                        let used = statement
                            .uses
                            .keys()
                            .any(|symbol| ssa.cell_key(*symbol) == &key);
                        assert_eq!(used, !foreign, "{source}/{key:?}");
                    }
                }
            }
            assert!(
                seen,
                "original physical read inventory missing for {source}"
            );
        }
    }

    #[test]
    fn word_read_requires_original_spelling_as_well_as_site() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            "set constructor list; set x [$constructor a b]; set out $x",
            registry,
            false,
            "tcl8.6",
        );
        let ssa = &unit.top_level.ssa;
        let mut reached = false;
        for (&block, data) in &ssa.blocks {
            for index in 0..data.statements.len() {
                let view = SsaSourceView::at_statement(ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                for original in tokens.words().iter().filter(|word| {
                    word.sole_variable_substitution()
                        .is_some_and(|(spelling, _)| spelling == "$x")
                }) {
                    assert!(view.read_word(original).is_some());
                    assert!(view.read_word_produces_value(original, registry));
                    assert!(view.read_word_place(original, registry).is_some());
                    assert_eq!(
                        view.read_word_representation(original, registry),
                        Some(tcl_syntax::value::ValueRepresentation::List)
                    );
                    let (_, source) = original.sole_variable_substitution().unwrap();
                    let changed = WordExpr::Variable {
                        spelling: "$z".into(),
                        source: source.clone(),
                    };
                    assert!(view.read_word(&changed).is_none());
                    assert!(!view.read_word_produces_value(&changed, registry));
                    assert!(view.read_word_place(&changed, registry).is_none());
                    assert!(view.read_word_contents(&changed, registry).is_none());
                    assert!(view.read_word_representation(&changed, registry).is_none());
                    assert!(
                        view.read_word_representation_alternatives(&changed, registry)
                            .is_none()
                    );
                    assert!(
                        !view
                            .read_word_representation_advice(&changed, registry)
                            .already_numeric
                    );
                    reached = true;
                }
            }
        }
        assert!(reached, "original physical read must be present");
    }

    #[test]
    fn expression_read_requires_original_spelling_as_well_as_extent() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            "proc p {x} {expr {$x + 1}}",
            registry,
            false,
            "tcl8.6",
        );
        let ssa = &unit.procedures["::p"].ssa;
        let mut reached = false;
        for (&block, data) in &ssa.blocks {
            for index in (0..data.statements.len()).chain(std::iter::once(usize::MAX)) {
                let view = SsaSourceView::at_statement(ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                for access in tokens
                    .variable_accesses
                    .iter()
                    .filter(|read| read.original_spelling == "$x")
                {
                    let original = crate::expr_ast::ExprNode::Var {
                        text: "$x".into(),
                        name: "x".into(),
                        start: 0,
                        end: 1,
                    };
                    let base = Some(access.source.span.start());
                    assert!(
                        view.read_expression_variable_place(&original, base, registry)
                            .is_some()
                    );
                    let changed = crate::expr_ast::ExprNode::Var {
                        text: "$z".into(),
                        name: "z".into(),
                        start: 0,
                        end: 1,
                    };
                    assert!(
                        view.read_expression_variable_place(&changed, base, registry)
                            .is_none()
                    );
                    assert!(
                        view.read_expression_representation(&changed, base, None, registry)
                            .is_none()
                    );
                    assert!(
                        view.read_expression_representation_alternatives(
                            &changed, base, None, registry
                        )
                        .is_none()
                    );
                    assert!(
                        view.read_expression_variable_place(&original, None, registry)
                            .is_none()
                    );
                    reached = true;
                }
            }
        }
        assert!(reached, "the original expression read must be retained");
    }

    #[test]
    fn potential_missing_read_retains_closed_presence_alternatives() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            "proc p {flag} {if {$flag} {set x 1}; set out $x}",
            registry,
            false,
            "tcl8.6",
        );
        let ssa = &unit.procedures["::p"].ssa;
        let mut reached = false;
        for (&block, data) in &ssa.blocks {
            for index in (0..data.statements.len()).chain(std::iter::once(usize::MAX)) {
                let view = SsaSourceView::at_statement(ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                for access in tokens
                    .variable_accesses
                    .iter()
                    .filter(|read| read.original_spelling == "$x")
                {
                    assert!(
                        view.read_contents_presence_alternatives_at(&access.source, "$x", registry)
                            .is_some_and(SsaReadPresenceAlternatives::may_be_undefined),
                        "source={:?}; residual={:?}; presence={:?}",
                        access.source,
                        access.context_residual(),
                        access
                            .context_alternatives()
                            .iter()
                            .map(|context| {
                                let place = crate::var_resolve::resolve_substitution_access(
                                    "$x",
                                    context,
                                    registry,
                                    tcl_registry::TraceOperation::Read,
                                );
                                (place.observed, context.contents_presence(&place))
                            })
                            .collect::<Vec<_>>()
                    );
                    assert_eq!(
                        view.read_contents_presence_at(&access.source, "$x", registry),
                        Some(crate::var_resolve::ContentsPresence::Unknown)
                    );
                    assert_eq!(
                        view.read_completion_at(&access.source, "$x", registry),
                        Some(crate::var_resolve::VariableReadCompletion::Unknown)
                    );
                    assert!(
                        view.read_reference(&access.source, "$x")
                            .is_none_or(|read| read.version.is_none())
                    );
                    reached = true;
                }
            }
        }
        assert!(reached);
        assert!(
            SsaSourceView::unpositioned(ssa)
                .read_contents_presence_alternatives_at(
                    &SourceSite::source(Span::new(0, 2)),
                    "$x",
                    registry,
                )
                .is_none()
        );
    }

    #[test]
    fn missing_read_completion_does_not_require_a_value_version() {
        use crate::var_resolve::VariableReadCompletion;

        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            "proc p {known} {set copy $known; set copy $missing}",
            registry,
            false,
            "tcl8.6",
        );
        let ssa = &unit.procedures["::p"].ssa;
        let mut missing = false;
        let mut incoming = false;
        for (&block, data) in &ssa.blocks {
            for index in (0..data.statements.len()).chain(std::iter::once(usize::MAX)) {
                let view = SsaSourceView::at_statement(ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                for access in &tokens.variable_accesses {
                    let completion = view.read_completion_at(
                        &access.source,
                        &access.original_spelling,
                        registry,
                    );
                    match access.original_spelling.as_str() {
                        "$missing" => {
                            assert_eq!(completion, Some(VariableReadCompletion::Error));
                            assert_eq!(
                                view.read_contents_presence_at(
                                    &access.source,
                                    &access.original_spelling,
                                    registry,
                                ),
                                Some(crate::var_resolve::ContentsPresence::Undefined)
                            );
                            assert!(
                                view.read_reference(&access.source, &access.original_spelling)
                                    .is_none_or(|read| read.version.is_none())
                            );
                            missing = true;
                        }
                        "$known" => {
                            assert_eq!(completion, Some(VariableReadCompletion::Unknown));
                            incoming = true;
                        }
                        _ => {}
                    }
                }
            }
        }
        assert!(
            missing && incoming,
            "both actual read contexts are retained"
        );
        assert!(SsaSourceView::unpositioned(ssa).source_tokens().is_none());
    }

    #[test]
    fn retained_word_read_success_is_separate_from_an_address_or_version() {
        for (profile, body, expected) in [
            ("tcl8.6", "set x 1; set copy $x", true),
            ("tcl8.6", "set copy $missing", false),
            ("tcl8.6", "array set a {k VALUE}; set copy $a", false),
            ("tcl8.6", "set a(k) VALUE; set copy $a(k)", true),
            ("jim", "set d odd; set copy $d(k)", false),
        ] {
            let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
            let source = format!("proc p {{}} {{{body}}}");
            let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
                &source, registry, false, profile,
            );
            let ssa = &unit.procedures["::p"].ssa;
            let mut reached = false;
            for (&block, data) in &ssa.blocks {
                for index in 0..data.statements.len() {
                    let view = SsaSourceView::at_statement(ssa, block, index);
                    let Some(tokens) = view.source_tokens() else {
                        continue;
                    };
                    for word in tokens.words() {
                        if word.sole_variable_substitution().is_some() {
                            assert_eq!(
                                view.read_word_produces_value(word, registry),
                                expected,
                                "{profile}: {body}"
                            );
                            reached = true;
                        }
                    }
                }
            }
            assert!(reached, "actual original read retained: {profile}: {body}");
        }
    }

    #[test]
    fn scalar_read_of_an_array_is_not_a_missing_contents_proof() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            "proc p {} {array set a {k VALUE}; puts $a}",
            registry,
            false,
            "tcl8.6",
        );
        let ssa = &unit.procedures["::p"].ssa;
        let mut reached = false;
        for (&block, data) in &ssa.blocks {
            for index in (0..data.statements.len()).chain(std::iter::once(usize::MAX)) {
                let view = SsaSourceView::at_statement(ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                for access in tokens
                    .variable_accesses
                    .iter()
                    .filter(|read| read.original_spelling == "$a")
                {
                    assert_eq!(
                        view.read_completion_at(&access.source, "$a", registry),
                        Some(crate::var_resolve::VariableReadCompletion::Error)
                    );
                    assert_eq!(
                        view.read_contents_presence_at(&access.source, "$a", registry),
                        Some(crate::var_resolve::ContentsPresence::Defined)
                    );
                    assert!(
                        view.read_reference(&access.source, "$a")
                            .is_none_or(|read| read.version.is_none()),
                        "a rejected scalar read cannot supply an SSA value"
                    );
                    reached = true;
                }
            }
        }
        assert!(reached, "the failing scalar read is retained");
    }

    #[test]
    fn caught_missing_catalogue_command_does_not_hide_unknown_fallback_mutations() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.4").commands();
        let source = "set a old\ncatch {lassign {new second} a b} m\nputs $a\n";
        let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
            source,
            registry,
            false,
            registry.profile().unwrap(),
        );
        let function = &cu.top_level;
        let point = function
            .cfg
            .blocks
            .iter()
            .find_map(|(&block, body)| {
                body.statements
                    .iter()
                    .enumerate()
                    .find_map(|(index, statement)| {
                        matches!(statement, Statement::Call {command, ..} if command == "puts")
                            .then_some((block, index))
                    })
            })
            .unwrap();
        let view = SsaSourceView::at_statement(&function.ssa, point.0, point.1);
        let read = view.read_spelling("a");
        let contexts = function.ssa.point_contexts.as_ref().unwrap();
        let context = contexts.before_statement(point.0, point.1);
        assert!(
            read.is_none_or(|read| read.version.is_none()),
            "read={read:?} constants={:?} dynamic_bindings={} dynamic_traces={} source_reads={} point_names={:?}",
            context.constant_values,
            context.dynamic_bindings,
            context.dynamic_traces,
            contexts.source_reads_at(point.0, point.1).len(),
            view.source_symbols().collect::<Vec<_>>(),
        );
    }

    #[test]
    fn caught_error_only_fallback_preserves_the_reaching_cell_contents() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.4").commands();
        let source = "proc unknown {args} {return -code error absent}\nset a old\ncatch {lassign {new second} a b} m\nputs $a\n";
        let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
            source,
            registry,
            false,
            registry.profile().unwrap(),
        );
        let function = &cu.top_level;
        let read = function
            .cfg
            .blocks
            .iter()
            .find_map(|(&block, body)| {
                body.statements
                    .iter()
                    .enumerate()
                    .find_map(|(index, statement)| {
                        matches!(statement, Statement::Call {command, ..} if command == "puts")
                            .then(|| {
                                SsaSourceView::at_statement(&function.ssa, block, index)
                                    .read_spelling("a")
                            })
                    })
            })
            .flatten();
        assert!(
            read.is_some_and(|read| read.version.is_some()),
            "read={read:?}"
        );
    }

    #[test]
    fn scalar_clobber_keeps_contents_lineage_without_creating_a_store() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let mut function = crate::cfg::Function::new("::f", "entry");
        function
            .blocks
            .get_mut(&function.entry)
            .unwrap()
            .statements
            .push(assignment("x"));
        let mut ssa = build_ssa(&function, &registry);
        let symbol = *ssa.blocks[&function.entry].statements[0]
            .defs
            .keys()
            .next()
            .unwrap();
        let prior = ssa.blocks[&function.entry].statements[0].defs[&symbol];
        let fresh = prior + 1;
        ssa.value_clobbers.insert(
            function.entry,
            HashMap::from([(1, HashMap::from([(symbol, (prior, fresh))]))]),
        );
        let origins = represented_contents_origins(&ssa.blocks, &ssa.value_clobbers);
        assert_eq!(origins[&(symbol, fresh)], ContentsOrigin::WrittenAt(0));
        assert_eq!(ssa.binding_version(symbol, fresh), prior);
        assert!(
            !ssa.blocks[&function.entry]
                .statements
                .iter()
                .any(|statement| { statement.defs.get(&symbol) == Some(&fresh) })
        );
        // Destruction invalidates contents even though its predecessor has a
        // real store. A later scalar clobber cannot revive that old value.
        let statement = &mut ssa.blocks.get_mut(&function.entry).unwrap().statements[0];
        statement.destruction_defs.insert(symbol);
        let origins = represented_contents_origins(&ssa.blocks, &ssa.value_clobbers);
        assert_eq!(origins[&(symbol, fresh)], ContentsOrigin::Unknown);
    }

    #[test]
    fn computed_value_store_keeps_its_exact_following_read_origin() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let source = "proc f {} {\n set b [binary format a* hello]\n set u [string toupper $b]\n}";
        let unit = crate::compilation_unit::CompilationUnit::build_for(source, registry, false);
        let function = unit.function("::f").unwrap();
        let origins =
            represented_contents_origins(&function.ssa.blocks, &function.ssa.value_clobbers);
        let mut checked = 0;
        for (&block, body) in &function.ssa.blocks {
            for (index, statement) in body.statements.iter().enumerate() {
                let view = SsaSourceView::at_statement(&function.ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                for access in &tokens.variable_accesses {
                    if access.original_spelling != "$b" && access.original_spelling != "${b}" {
                        continue;
                    }
                    let target = source_read_place(access, registry);
                    let read = view.read_reference(&access.source, &access.original_spelling);
                    let origin = access
                        .variable_context
                        .read_contents_origin(&target, registry);
                    assert!(
                        read.is_some_and(|read| read.version.is_some()),
                        "read={read:?} source_origin={origin:?} target={target:?} uses={:?} represented={origins:?} statement_span={:?}",
                        statement.uses,
                        statement.statement.span()
                    );
                    checked += 1;
                }
            }
        }
        assert_eq!(checked, 1);
    }

    fn assignment(name: &str) -> Statement {
        Statement::AssignConst {
            span: Span::new(0, 1),
            name: name.to_owned(),
            name_braced: false,
            value: "10".to_owned(),
            value_span: None,
        }
    }

    fn variable_return(name: &str) -> Terminator {
        Terminator::Return {
            tokens: None,
            value: Some(format!("${name}")),
            value_word: None,
            span: None,
            expr: None,
            expr_base: None,
            braced: false,
        }
    }

    #[test]
    fn relative_qualified_write_and_absolute_read_share_symbol() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut cfg = cfg::Function::new("::N::p", "entry");
        let block = cfg.blocks.get_mut(&cfg.entry).expect("entry");
        block.statements.push(assignment("R::x"));
        block.terminator = Some(variable_return("::N::R::x"));
        let mut entry = ResolveContext::for_function(&cfg.name);
        entry.known_namespaces.insert("::N::R".to_owned());
        entry.namespace_cells.present.insert("::N::R::x".to_owned());
        let ssa = build_ssa_with_context(
            &cfg,
            registry,
            tcl_lexer::LexerConfig::for_profile(registry.profile()),
            entry,
        );
        let relative = ssa
            .var_symbol_at(cfg.entry, 0, "R::x")
            .expect("bound relative access");
        assert_eq!(Some(relative), ssa.var_symbol("::N::R::x"));
        assert!(
            ssa.blocks[&cfg.entry].statements[0]
                .defs
                .contains_key(&relative)
        );
    }

    #[test]
    fn top_level_global_and_bare_spelling_share_identity_and_keep_display_name() {
        let registry = CommandRegistry::build_default();
        let mut cfg = cfg::Function::new("::top", "entry");
        let block = cfg.blocks.get_mut(&cfg.entry).expect("entry");
        block.statements.push(assignment("g"));
        block.terminator = Some(variable_return("::g"));
        let ssa = build_ssa(&cfg, &registry);
        let symbol = ssa.var_symbol("g").expect("source name");
        assert_eq!(Some(symbol), ssa.var_symbol("::g"));
        assert_eq!(ssa.var_name(symbol), "g");
    }
    #[test]
    fn scalar_alias_write_after_unset_reaches_both_spellings() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let source = "proc p {} {set x OLD; upvar 0 x y; unset x; set y NEW; puts $x; puts $y}; p";
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl9.0",
        );
        let ssa = &unit.procedures["::p"].ssa;
        let mut written = None;
        let mut read = Vec::new();
        for (&block, body) in &ssa.blocks {
            for (index, statement) in body.statements.iter().enumerate() {
                match &statement.statement {
                    operation
                        if source
                            .get(operation.span().start() as usize..operation.span().end() as usize)
                            .is_some_and(|text| text.starts_with("set y NEW")) =>
                    {
                        written = ssa.var_symbol_at(block, index, "y").and_then(|symbol| {
                            statement
                                .defs
                                .get(&symbol)
                                .map(|&version| (symbol, version))
                        });
                    }
                    Statement::Call { args, .. }
                        if args.iter().any(|arg| {
                            arg == "${x}" || arg == "${y}" || arg == "$x" || arg == "$y"
                        }) =>
                    {
                        read.push((block, index, statement.uses.clone()));
                    }
                    _ => {}
                }
            }
        }
        let (symbol, version) =
            written.unwrap_or_else(|| panic!("alias store contents definition: {ssa:#?}"));
        assert_eq!(read.len(), 2, "both puts reads: {ssa:#?}");
        assert!(
            read.iter()
                .all(|(_, _, uses)| uses.get(&symbol) == Some(&version)),
            "scalar alias store must reach both spellings: {ssa:#?}"
        );
    }
    #[test]
    fn exact_reads_keep_alias_retargeting_within_one_word_separate() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let source = "proc p {} {set x OLD; set y NEW; upvar 0 x a; puts \"$a[upvar 0 y a]$a\"}; p";
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let ssa = &unit.procedures["::p"].ssa;
        let mut pairs = Vec::new();
        for (&block, body) in &ssa.blocks {
            for index in 0..body.statements.len() {
                let view = SsaSourceView::at_statement(ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                let reads: Vec<_> = tokens
                    .variable_accesses
                    .iter()
                    .filter(|access| access.original_spelling == "$a")
                    .filter_map(|access| {
                        view.read_reference(&access.source, &access.original_spelling)
                    })
                    .collect();
                if reads.len() == 2 {
                    assert!(view.is_positioned());
                    assert!(view.read_spelling("a").is_none());
                    assert!(
                        view.symbol("a").is_none(),
                        "one source name cannot select two cells"
                    );
                    pairs.push(reads);
                }
            }
        }
        assert_eq!(pairs.len(), 1, "{ssa:#?}");
        assert_ne!(pairs[0][0].symbol, pairs[0][1].symbol);
        assert!(pairs[0].iter().all(|read| read.version.is_some()));
    }

    #[test]
    fn incoming_slot_projection_keeps_distinct_call_activations() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let source = "proc p {n} {return [expr {$n + 0}]}; p 20; p 21";
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let ssa = &unit.procedures["::p"].ssa;
        let node = tcl_syntax::expr::parser::parse_expr("$n", Some("tcl8.6"));
        let base = Some(u32::try_from(source.find("$n").unwrap()).unwrap());
        let mut proofs = Vec::new();
        for (&block, data) in &ssa.blocks {
            for index in (0..data.statements.len()).chain(std::iter::once(usize::MAX)) {
                let view = SsaSourceView::at_statement(ssa, block, index);
                if let Some(proof) = view.read_expression_incoming_slot(&node, base, "n", registry)
                {
                    proofs.push(proof);
                }
            }
        }
        assert!(
            !proofs.is_empty(),
            "the reached ordinary formal has an input proof"
        );
        assert!(proofs.iter().all(|proof| proof.slot == "n"));
        assert!(
            proofs.iter().any(|proof| proof.cells.len() >= 2),
            "separate callers retain separate physical owners: {proofs:?}"
        );
    }

    #[test]
    fn incoming_slot_projection_rejects_a_formal_rebound_to_global_storage() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let source =
            "set n 100; proc p {n} {unset n; global n; return [expr {$n + 0}]}; p 20; p 21";
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let ssa = &unit.procedures["::p"].ssa;
        let node = tcl_syntax::expr::parser::parse_expr("$n", Some("tcl8.6"));
        let base = Some(u32::try_from(source.find("$n").unwrap()).unwrap());
        let mut reached = false;
        for (&block, data) in &ssa.blocks {
            for index in (0..data.statements.len()).chain(std::iter::once(usize::MAX)) {
                let view = SsaSourceView::at_statement(ssa, block, index);
                reached |= view.expression_variable_access(&node, base).is_some();
                assert!(
                    view.read_expression_incoming_slot(&node, base, "n", registry)
                        .is_none()
                );
            }
        }
        assert!(
            reached,
            "the global read is retained but is not a formal input"
        );
    }

    fn iteration_read_summary(
        function: &crate::compilation_unit::FunctionUnit,
        registry: &CommandRegistry,
    ) -> String {
        let mut original = Vec::new();
        for (&block, data) in &function.ssa.blocks {
            for (index, statement) in data.statements.iter().enumerate() {
                let view = SsaSourceView::at_statement(&function.ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                for access in tokens
                    .variable_accesses
                    .iter()
                    .filter(|access| access.original_spelling == "$l")
                {
                    let place = source_read_place(access, registry);
                    original.push((
                        block,
                        index,
                        tokens.synthetic,
                        &statement.defs,
                        &statement.uses,
                        access.source.span,
                        place.clone(),
                        access
                            .variable_context
                            .read_contents_origin(&place, registry),
                        view.read_reference(&access.source, &access.original_spelling),
                    ));
                }
            }
        }
        format!("{original:?}")
    }

    #[test]
    fn entered_iteration_store_provides_the_body_read_version() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        // The stock outer List closes iteration callbacks without normalising
        // its unknown element before the retained original body read.
        let source = "proc p {item} {set l [list $item]; foreach x $l {set y [expr {$x + 1}]}}";
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let ssa = &unit.procedures["::p"].ssa;
        let mut reads = Vec::new();
        let mut inventory = Vec::new();
        for (&block, data) in &ssa.blocks {
            for index in (0..data.statements.len()).chain(std::iter::once(usize::MAX)) {
                let view = SsaSourceView::at_statement(ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                inventory.push((
                    block,
                    index,
                    tokens.argv_texts.first().cloned(),
                    tokens.source_binding.as_ref().map(|binding| {
                        (
                            binding.unknown,
                            binding.may_be_absent,
                            binding
                                .targets
                                .iter()
                                .map(|target| target.command.as_str())
                                .collect::<Vec<_>>(),
                        )
                    }),
                    tokens
                        .nested_bindings
                        .iter()
                        .map(|(site, binding)| {
                            (
                                site,
                                binding.unknown,
                                binding.may_be_absent,
                                binding
                                    .targets
                                    .iter()
                                    .map(|target| target.command.as_str())
                                    .collect::<Vec<_>>(),
                                binding.compiled_execution_residual,
                            )
                        })
                        .collect::<Vec<_>>(),
                    tokens
                        .variable_accesses
                        .iter()
                        .map(|access| (&access.source, &access.original_spelling))
                        .collect::<Vec<_>>(),
                ));
                for access in tokens
                    .variable_accesses
                    .iter()
                    .filter(|access| source_read_name(access) == "x")
                {
                    let place = source_read_place(access, registry);
                    let origin = access
                        .variable_context
                        .read_contents_origin(&place, registry);
                    let read = view.read_reference(&access.source, &access.original_spelling);
                    assert!(
                        read.is_some_and(|read| read.version.is_some_and(|version| version > 0)),
                        "entered iterator value has a represented store: place={place:?}, origin={origin:?}, read={read:?}"
                    );
                    reads.push(read.unwrap());
                }
            }
        }
        assert!(
            !reads.is_empty(),
            "the reached expression read is retained: {inventory:?}"
        );
    }

    #[test]
    fn iteration_input_reference_keeps_preloop_version_when_body_mutates_list() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let source = "proc p {} {set l BEFORE; foreach x $l {set l AFTER}; return $x}; p";
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let function = &unit.procedures["::p"];
        let mut cached = Vec::new();
        for (&block, data) in &function.ssa.blocks {
            for (index, statement) in data.statements.iter().enumerate() {
                if statement.statement.tokens().is_none_or(|tokens| {
                    !matches!(
                        tokens.synthetic,
                        Some(crate::ir::SyntheticMarker::IterationBindings(_))
                    )
                }) {
                    continue;
                }
                assert!(function.cfg.source_tokens_at(block, index).is_none());
                assert!(function.cfg.source_tokens_at(block, usize::MAX).is_none());
                let input = function.cfg.source_input_tokens_at(block, index).unwrap();
                let view = SsaSourceView::at_statement(&function.ssa, block, index);
                cached.push(view.read_word(&input.words()[1]).unwrap_or_else(|| {
                    let accesses: Vec<_> = input
                        .variable_accesses
                        .iter()
                        .map(|access| {
                            (
                                access.source.span,
                                &access.original_spelling,
                                source_read_place(access, registry),
                                view.read_reference(&access.source, &access.original_spelling),
                            )
                        })
                        .collect();
                    let original = iteration_read_summary(function, registry);
                    panic!(
                        "original captured input read: word={:?}, accesses={accesses:?}, evaluated={original:?}",
                        input.words()[1]
                    )
                }));
                assert!(
                    !statement.uses.contains_key(&cached.last().unwrap().symbol),
                    "cached list is not reevaluated"
                );
            }
        }
        assert_eq!(cached.len(), 1);
        for (&block, data) in &function.cfg.blocks {
            if matches!(data.terminator, Some(Terminator::Goto { .. })) {
                assert!(function.cfg.source_tokens_at(block, usize::MAX).is_none());
                assert_eq!(
                    function
                        .ssa
                        .point_contexts
                        .as_ref()
                        .unwrap()
                        .source_reads_at(block, usize::MAX),
                    [] as [crate::command_binding::SourceVariableAccess; 0]
                );
            }
        }
        let version = cached[0].version.expect("represented preloop definition");
        let (block, index, _) = function
            .ssa
            .blocks
            .iter()
            .flat_map(|(&block, data)| {
                data.statements
                    .iter()
                    .enumerate()
                    .map(move |(index, statement)| (block, index, statement))
            })
            .find(|(_, _, statement)| statement.defs.get(&cached[0].symbol) == Some(&version))
            .unwrap();
        let source = SsaSourceView::at_statement(&function.ssa, block, index);
        assert!(
            matches!(source.source_tokens().and_then(|tokens| tokens.words().get(2)),
            Some(crate::ir::WordExpr::Literal { text, .. }) if text == "BEFORE")
        );
    }

    #[test]
    fn terminator_read_consensus_declines_retargeted_source_spelling() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let source =
            "proc p {} {set x OLD; set y NEW; upvar 0 x a; return \"$a[upvar 0 y a]$a\"}; p";
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let ssa = &unit.procedures["::p"].ssa;
        let mut pairs = Vec::new();
        for &block in ssa.blocks.keys() {
            let view = SsaSourceView::at_terminator(ssa, block);
            let Some(tokens) = view.source_tokens() else {
                continue;
            };
            let reads: Vec<_> = tokens
                .variable_accesses
                .iter()
                .filter(|access| access.original_spelling == "$a")
                .filter_map(|access| view.read_reference(&access.source, &access.original_spelling))
                .collect();
            if reads.len() == 2 {
                assert!(view.symbol("a").is_none());
                assert!(view.read_spelling("a").is_none());
                pairs.push(reads);
            }
        }
        assert_eq!(pairs.len(), 1);
        assert_ne!(pairs[0][0].symbol, pairs[0][1].symbol);
        assert!(pairs[0].iter().all(|read| read.version.is_some()));
    }

    #[test]
    fn reaching_binding_resolves_contents_at_a_goto_without_inventing_a_read() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.1").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            "proc a {} {set j 0; for {set k 5} {$k > 0} {incr k -1} {incr j}}",
            registry,
            false,
            registry.profile().unwrap(),
        );
        let function = unit.function("::a").unwrap();
        let node = function.cfg.loop_nodes.values().next().unwrap();
        let view = SsaSourceView::at_terminator(&function.ssa, node.entry_block);
        assert!(view.source_symbols().next().is_none());
        let reaching = view.reaching_binding("j", registry).unwrap();
        let version = reaching.version.expect("the literal store is represented");
        assert_eq!(
            function.sccp.values.get(&(reaching.symbol, version)),
            Some(&crate::analyses::LatticeValue::Const(
                crate::analyses::ConstValue::Int(0)
            )),
        );
        assert!(view.reaching_binding("missing", registry).is_none());
        assert!(
            SsaSourceView::unpositioned(&function.ssa)
                .reaching_binding("j", registry)
                .is_none()
        );
        assert!(
            SsaSourceView::at_terminator(&function.ssa, BlockId(u32::MAX))
                .reaching_binding("j", registry)
                .is_none()
        );
    }

    #[test]
    fn reaching_binding_declines_contents_after_an_unrepresented_mutation() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.1").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            "proc a {cmd} {set x OLD; $cmd}",
            registry,
            false,
            registry.profile().unwrap(),
        );
        let function = unit.function("::a").unwrap();
        let block = function
            .ssa
            .blocks
            .iter()
            .find(|(_, block)| {
                block.statements.iter().any(|statement| {
                    statement
                        .statement
                        .tokens()
                        .and_then(|tokens| tokens.words().first())
                        .is_some_and(|head| matches!(head, crate::ir::WordExpr::Variable { .. }))
                })
            })
            .unwrap()
            .0;
        let view = SsaSourceView::at_terminator(&function.ssa, *block);
        assert!(
            view.reaching_binding("x", registry)
                .is_none_or(|reaching| reaching.version.is_none()),
            "unknown contents cannot borrow the prior store",
        );
    }

    #[test]
    fn nested_contents_store_does_not_borrow_an_earlier_ssa_version() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let source = "proc p {} {set x OLD; puts \"$x[set x NEW]$x\"}; p";
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let ssa = &unit.procedures["::p"].ssa;
        let mut pairs = Vec::new();
        for (&block, body) in &ssa.blocks {
            for index in 0..body.statements.len() {
                let view = SsaSourceView::at_statement(ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                let reads: Vec<_> = tokens
                    .variable_accesses
                    .iter()
                    .filter(|access| access.original_spelling == "$x")
                    .filter_map(|access| {
                        view.read_reference(&access.source, &access.original_spelling)
                    })
                    .collect();
                if reads.len() == 2 {
                    assert!(view.read_spelling("x").is_none());
                    pairs.push(reads);
                }
            }
        }
        assert_eq!(pairs.len(), 1, "{ssa:#?}");
        assert_eq!(pairs[0][0].symbol, pairs[0][1].symbol);
        assert!(pairs[0][0].version.is_some());
        assert!(pairs[0][1].version.is_none());
    }

    #[test]
    fn name_only_read_projection_requires_reached_consensus() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            "proc p {} {set x OLD; puts \"$x$x\"}; p",
            registry,
            false,
            "tcl8.6",
        );
        let ssa = &unit.procedures["::p"].ssa;
        let mut projected = Vec::new();
        for (&block, body) in &ssa.blocks {
            for index in 0..body.statements.len() {
                let view = SsaSourceView::at_statement(ssa, block, index);
                assert!(view.read_spelling("unreached").is_none());
                if let Some(read) = view.read_spelling("x") {
                    assert!(read.version.is_some());
                    projected.push(read);
                }
            }
        }
        assert_eq!(projected.len(), 1);
        assert!(!SsaSourceView::unpositioned(ssa).is_positioned());
        assert!(
            SsaSourceView::unpositioned(ssa)
                .read_spelling("x")
                .is_none()
        );
    }

    #[test]
    fn computed_command_head_keeps_its_captured_read_inside_dictionary_iteration() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for(
            "proc demo {} {\n    set x set\n    set d [dict create a true b false c true]\n    dict for {key value} $d {\n        if {$value} {\n            $x a $key\n        }\n    }\n}\n",
            registry,
            false,
        );
        let ssa = &unit.function("::demo").unwrap().ssa;
        let mut inventory = Vec::new();
        let mut reads = Vec::new();
        for (&block, body) in &ssa.blocks {
            for index in 0..body.statements.len() {
                let view = SsaSourceView::at_statement(ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                inventory.push((
                    tokens.words().first().map(WordExpr::legacy_text),
                    tokens
                        .variable_accesses
                        .iter()
                        .map(|access| access.original_spelling.clone())
                        .collect::<Vec<_>>(),
                ));
                for access in tokens
                    .variable_accesses
                    .iter()
                    .filter(|access| source_read_name(access) == "x")
                {
                    reads.push(view.read_reference(&access.source, &access.original_spelling));
                }
            }
        }
        assert!(
            reads.iter().any(|read| {
                read.is_some_and(|read| read.version.is_some_and(|version| version > 0))
            }),
            "computed command-head reads retain their actual producer: reads={reads:?}, inventory={inventory:?}"
        );
    }
}

#[cfg(test)]
mod external_mutability_tests {
    use super::*;

    #[test]
    fn named_external_dependencies_use_actual_point_bindings_and_ignore_display_labels() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for(
            "proc p {} {set dictionary [dict create k 1]; upvar 0 dictionary view; set dictionary [dict create k 2]}",
            registry,
            false,
        );
        let mut ssa = unit.function("::p").unwrap().ssa.clone();
        let mut writes = Vec::new();
        for (&block, body) in &ssa.blocks {
            for (index, statement) in body.statements.iter().enumerate() {
                if matches!(&statement.statement, Statement::AssignValue { name, .. } if name == "dictionary")
                {
                    writes.push((
                        statement.statement.span().start(),
                        block,
                        index,
                        *statement.defs.keys().next().unwrap(),
                    ));
                }
            }
        }
        writes.sort_by_key(|(offset, ..)| *offset);
        assert_eq!(writes.len(), 2);
        assert_eq!(writes[0].3, writes[1].3);
        let symbol = writes[0].3;
        let names = HashSet::from(["view".to_owned()]);
        for label in ["view", "dictionary", "unrelated"] {
            ssa.var_names[symbol.0 as usize] = label.to_owned();
            for (point, expected) in [(writes[0], false), (writes[1], true)] {
                let source = SsaSourceView::at_statement(&ssa, point.1, point.2);
                assert_eq!(
                    source.externally_mutable_by(symbol, &names, false, registry),
                    Some(expected),
                    "{label}; canonical={}; before={:?}; carrier={:?}",
                    ssa.cell_name(symbol),
                    ssa.point_contexts
                        .as_ref()
                        .and_then(|points| points.context_before(point.1, point.2))
                        .map(|context| canonical_variable_key("view", context, registry)),
                    source
                        .source_tokens()
                        .and_then(|tokens| tokens.source_binding.as_ref())
                        .map(|binding| canonical_variable_key(
                            "view",
                            &binding.variable_context,
                            registry
                        )),
                );
                assert_eq!(
                    source.externally_mutable_by(symbol, &names, true, registry),
                    Some(true)
                );
                assert_eq!(
                    source.externally_mutable_by(
                        symbol,
                        &HashSet::from([ssa.cell_name(symbol).to_owned()]),
                        false,
                        registry
                    ),
                    Some(false)
                );
            }
        }
        assert_eq!(
            SsaSourceView::unpositioned(&ssa)
                .externally_mutable_by(symbol, &names, false, registry),
            None
        );
        assert_eq!(
            SsaSourceView::unpositioned(&ssa).externally_mutable_by(
                symbol,
                &HashSet::new(),
                false,
                registry
            ),
            Some(false)
        );
    }
}

#[cfg(test)]
mod original_definition_name_tests {
    use super::*;

    #[test]
    // Implementation contract: naming.variable.original-ssa-definition-operand
    // docs/design/analysis/name-resolution-proofs/original-ssa-definition-operand.md
    fn original_ssa_definition_names_preserve_exact_operands_and_distinct_cells() {
        let source = r"set v\uD800 42; set v\uD801 TEXT; set a(k) 7";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let function = &unit.top_level;
        let specialised = function
            .ssa
            .blocks
            .iter()
            .find_map(|(&block, body)| {
                body.statements
                    .iter()
                    .enumerate()
                    .find_map(|(index, statement)| {
                        (statement.statement.span().start() == 0).then_some((
                            block,
                            index,
                            &statement.statement,
                        ))
                    })
            })
            .expect("first original assignment");
        assert!(matches!(specialised.2, Statement::AssignConst { .. }));
        assert!(specialised.2.tokens().is_none());
        assert!(
            function
                .ssa
                .point_contexts
                .as_ref()
                .expect("source points")
                .source_tokens_at(specialised.0, specialised.1)
                .is_some()
        );
        let mut definitions = Vec::new();
        for (&block, body) in &function.ssa.blocks {
            for (index, statement) in body.statements.iter().enumerate() {
                let view = SsaSourceView::at_statement(&function.ssa, block, index);
                for &symbol in statement.defs.keys() {
                    if let Some(definition) = view.original_definition_name(symbol, registry) {
                        assert_eq!(definition.cell(), function.ssa.cell_key(symbol));
                        definitions.push(definition);
                    }
                }
            }
        }
        let select = |spelling: &str| {
            definitions
                .iter()
                .find(|definition| &source[definition.span().as_range()] == spelling)
                .unwrap_or_else(|| {
                    panic!(
                        "missing original definition for {spelling}; count={}",
                        definitions.len()
                    )
                })
        };
        let first = select(r"v\uD800");
        let second = select(r"v\uD801");
        assert_ne!(
            first.original_name_input().bytes(),
            second.original_name_input().bytes()
        );
        assert_ne!(first.cell(), second.cell());
        assert_eq!(select("a(k)").original_name_input().bytes(), b"a(k)");
    }
}
