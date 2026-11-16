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
//! The SSA rename pass ([`build_ssa`]) and variable-use scanner ([`uses_of`])
//! walk the dominator tree, assigning SSA versions to definitions and uses
//! and filling phi-node incoming edges.

use std::collections::{BTreeSet, HashMap, HashSet};

use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};
use tcl_registry::CommandRegistry;

use crate::cfg::{self, BlockId};
use crate::ir::{CommandTokens, SourceSite, Statement, WordExpr, WordPart};
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
        Self::supplied_with_standalone_owner(registry, metadata, false)
    }

    fn supplied_with_standalone_owner(
        registry: &'a CommandRegistry,
        metadata: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
        standalone_owner: bool,
    ) -> Self {
        Self {
            registry,
            metadata,
            standalone: metadata
                .is_some_and(crate::registry_invocation::InvocationMetadataContext::is_standalone)
                || (standalone_owner && metadata.is_none()),
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
        if let Some(metadata) = self.metadata
            && let Some(input) = metadata.source_analysis_input()
            && metadata.matches_registry(self.registry)
            && binding
                .original_native_source_words(tokens, input, self.registry)
                .is_some()
        {
            return Some(metadata);
        }
        supplied_ssa_metadata(
            self.registry,
            binding.original_lexer_config_for_tokens(tokens)?,
            self.metadata,
        )
    }

    /// Supplied source retains its own role owner; detached requested roles
    /// are confined to the independently selected standalone utility route.
    fn word_reads(self, scanner: &mut VarReferenceScanner, text: &str) -> BTreeSet<String> {
        if self.standalone && scanner.standalone_roles_requested() {
            scanner.scan_standalone_word(text, self.registry)
        } else {
            scanner.scan_lexical_word(text, self.registry)
        }
    }

    /// Conditional source roles retain their original Logical owner. They
    /// cannot replace an unavailable Native handler or Normal transfer.
    fn logical_structure(
        self,
        tokens: &CommandTokens,
    ) -> Option<crate::registry_invocation::LogicalStructuredInvocation> {
        let metadata = self.token_metadata(tokens)?;
        metadata.source_analysis_input()?;
        (tokens
            .source_binding
            .as_ref()?
            .original_source_advice_purpose(tokens, metadata, self.registry)?
            == crate::command_binding::OriginalInvocationSourcePurpose::Logical)
            .then_some(())?;
        crate::registry_invocation::logical_structured_invocation_with_metadata_context(
            self.registry,
            metadata,
            tokens,
            None,
        )
    }

    fn source_structure(
        self,
        tokens: &CommandTokens,
    ) -> Option<crate::registry_invocation::SourceStructuredInvocation> {
        let metadata = self.token_metadata(tokens)?;
        crate::registry_invocation::original_native_source_structured_invocation(
            self.registry,
            metadata,
            tokens,
        )
        .or_else(|| {
            self.logical_structure(tokens).map(|selected| {
                crate::registry_invocation::SourceStructuredInvocation::Logical(Box::new(selected))
            })
        })
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

/// The existing definition producer owns its operand-binding phase. Source
/// name correspondence cannot resolve that definition again after its effects.
fn original_structural_definition_key_at(
    statement: &Statement,
    block: BlockId,
    index: usize,
    points: &PointResolveContexts,
    registry: &CommandRegistry,
) -> Option<VariableCellKey> {
    if !matches!(
        statement,
        Statement::AssignConst { .. }
            | Statement::AssignValue { .. }
            | Statement::AssignExpr { .. }
            | Statement::Incr { .. }
    ) {
        return None;
    }
    let places = crate::place_bridge::def_places_at(statement, block, index, points, registry);
    let mut keys = places
        .iter()
        .map(crate::var_resolve::canonical_binding_value_key);
    let key = keys.next()??;
    keys.all(|other| other.as_ref() == Some(&key))
        .then_some(key)
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
            crate::place_bridge::ssa_destruction_keys(
                statement,
                context,
                after,
                registry,
                points.source_metadata_owner().metadata_context(registry),
            )
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
    contains_defined: bool,
}

impl SsaReadPresenceAlternatives {
    /// At least one retained physical continuation has undefined contents.
    /// Every other continuation is also closed and has known presence.
    #[must_use]
    pub const fn may_be_undefined(self) -> bool {
        self.contains_undefined
    }

    /// At least one retained continuation may hold defined contents, so the
    /// read is not a proved absence on every path.
    #[must_use]
    pub const fn may_be_defined(self) -> bool {
        self.contains_defined
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

#[derive(Debug, Clone, PartialEq)]
struct SourceMaySsaCall {
    statement: Statement,
    original: Option<crate::cfg_builder::global_write_info::OriginalSourceCallMayWrites>,
    definitions: Vec<(Symbol, Version)>,
    clobber: Option<SourceMayClobber>,
}

#[derive(Debug, Clone, PartialEq)]
struct SourceMayClobber {
    marker_index: usize,
    marker: Statement,
    versions: HashMap<Symbol, (Version, Version)>,
    locations: std::sync::Arc<Vec<(Symbol, VariableCellKey)>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OriginalScriptSourcePoint {
    symbol: Symbol,
    access: crate::registry_invocation::AuthoredSourceVariableAccess,
    used: Option<Version>,
    defined: Option<Version>,
}

#[derive(Debug, Clone)]
struct SourceCfgBlockSnapshot {
    terminator: Option<cfg::Terminator>,
    successors: Vec<BlockId>,
    standalone_statements: Option<std::sync::Arc<[Statement]>>,
}

// Source statement custody is checked separately from topology equality.
impl PartialEq for SourceCfgBlockSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.terminator == other.terminator && self.successors == other.successors
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SourceVariableSsaSnapshot {
    identity: crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
    definitions: Vec<(
        BlockId,
        usize,
        Version,
        crate::cfg_builder::global_write_info::OriginalSourceCallMayWrites,
    )>,
    graph: Vec<SourceVariableBlockSnapshot>,
    topology: HashMap<BlockId, SourceCfgBlockSnapshot>,
    entry: BlockId,
    idom: HashMap<BlockId, Option<BlockId>>,
    dominance_frontier: HashMap<BlockId, Vec<BlockId>>,
    dominator_tree: HashMap<BlockId, Vec<BlockId>>,
}

#[derive(Debug, Clone, PartialEq)]
struct SourceVariableBlockSnapshot {
    block: BlockId,
    phis: Vec<Phi>,
    entry: Option<Version>,
    exit: Option<Version>,
    statements: Vec<SourceVariableStatementSnapshot>,
}

#[derive(Debug, Clone, PartialEq)]
struct SourceVariableReadFlags {
    quoted: bool,
    name_only: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct SourceVariableStatementSnapshot {
    index: usize,
    statement: Statement,
    used: Option<Version>,
    defined: Option<Version>,
    may: bool,
    reads: SourceVariableReadFlags,
    destroyed: bool,
}

impl SourceVariableSsaSnapshot {
    pub(crate) fn same_owner(&self, other: &Self) -> bool {
        self.identity == other.identity
            && self.graph == other.graph
            && self.topology == other.topology
            && self.entry == other.entry
            && self.idom == other.idom
            && self.dominance_frontier == other.dominance_frontier
            && self.dominator_tree == other.dominator_tree
            && self.definitions.len() == other.definitions.len()
            && self
                .definitions
                .iter()
                .zip(&other.definitions)
                .all(|(left, right)| {
                    left.0 == right.0
                        && left.1 == right.1
                        && left.2 == right.2
                        && left.3.same_owner(&right.3)
                })
    }
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
    /// Fresh scalar versions where a call to code the module cannot see sits
    /// ([`crate::ir::SyntheticMarker::UnseenCall`]), for the names live past
    /// it. These are value effects, separate from executable writes and
    /// statement use/def evidence: the binding behind a fresh version is the
    /// one before the marker ([`Self::binding_version`]).
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
    cell_indices: Vec<Option<usize>>,
    source_variables:
        FxHashMap<Symbol, crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol>,
    source_may_effects: HashMap<(BlockId, usize), SourceMaySsaCall>,
    // Positive RootScript issuer rows; never inferred from a slot shape.
    original_script_points: HashMap<(BlockId, usize), OriginalScriptSourcePoint>,
    source_topology: HashMap<BlockId, SourceCfgBlockSnapshot>,
    /// Reverse interner index: variable name → its [`Symbol`].
    var_to_symbol: FxHashMap<String, Symbol>,
    /// Versions live at an authentic unseen-call effect marker.
    unseen_call_versions: HashSet<(Symbol, Version)>,
    cell_to_symbol: FxHashMap<VariableCellKey, Symbol>,
    point_symbols: HashMap<(BlockId, usize), HashMap<String, Symbol>>,
    /// Shared reaching frame and cell bindings used by scalar and memory consumers.
    pub point_contexts: Option<PointResolveContexts>,
    read_references: HashMap<(BlockId, usize, SourceSite, String), SsaReadReference>,
    source_read_references: HashMap<(BlockId, usize, SourceSite, String), SsaReadReference>,
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

/// One conditional source definition in its current actual declared body.
/// Its authored SSA slot supplies no physical cell, successful store or edit permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceSsaDefinitionName {
    span: tcl_lexer::Span,
    input: crate::signature_scan::scope::SignatureSourceNameInput,
    body: crate::ir::ProcedureId,
}
impl OriginalSourceSsaDefinitionName {
    /// Complete original written variable operand extent in the document.
    #[must_use]
    pub const fn span(&self) -> tcl_lexer::Span {
        self.span
    }
    /// Genuine original operand under the retained Native source policy.
    #[must_use]
    pub fn original_name_input(&self) -> &crate::signature_scan::scope::SignatureSourceNameInput {
        &self.input
    }
    /// Current issued body whose conditional source owns this occurrence.
    #[must_use]
    pub const fn body_id(&self) -> crate::ir::ProcedureId {
        self.body
    }
}

/// Current analysis unit and authored SSA symbol selected by original definition geometry.
/// This readonly source projection grants no reached cell, Native value or edit permission.
pub struct OriginalSourceSsaDefinition<'a> {
    function: &'a crate::compilation_unit::FunctionUnit,
    name: OriginalSourceSsaDefinitionName,
    symbol: Symbol,
    version: Version,
}
impl<'a> OriginalSourceSsaDefinition<'a> {
    /// Actual function from the supplied current compilation unit.
    #[must_use]
    pub const fn function(&self) -> &'a crate::compilation_unit::FunctionUnit {
        self.function
    }
    /// Exact original source definition and its issued body owner.
    #[must_use]
    pub const fn name(&self) -> &OriginalSourceSsaDefinitionName {
        &self.name
    }
    /// Authored analysis symbol under this function's own interner.
    #[must_use]
    pub const fn symbol(&self) -> Symbol {
        self.symbol
    }
    /// Version at the represented conditional source definition.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.version
    }
}

/// Select a unique current source definition by its original document extent.
/// Equal variable labels and function display names never select the owner.
#[must_use]
pub fn original_source_definition_at_unit<'a>(
    unit: &'a crate::compilation_unit::CompilationUnit,
    span: tcl_lexer::Span,
    registry: &CommandRegistry,
) -> Option<OriginalSourceSsaDefinition<'a>> {
    let mut selected: Option<OriginalSourceSsaDefinition<'a>> = None;
    for function in unit.all_body_function_units() {
        let Some(body) = function.body_id else {
            continue;
        };
        for (&block, statements) in &function.ssa.blocks {
            for (index, statement) in statements.statements.iter().enumerate() {
                let view = SsaSourceView::at_statement(&function.ssa, block, index);
                for (&symbol, &version) in &statement.defs {
                    let Some(name) =
                        view.original_source_definition_for_body(symbol, unit, body, registry)
                    else {
                        continue;
                    };
                    if name.span() != span {
                        continue;
                    }
                    if selected.as_ref().is_some_and(|old| {
                        old.name != name
                            || old.symbol != symbol
                            || old.version != version
                            || !std::ptr::eq(old.function, function)
                    }) {
                        return None;
                    }
                    selected = Some(OriginalSourceSsaDefinition {
                        function,
                        name,
                        symbol,
                        version,
                    });
                }
            }
        }
    }
    selected
}

/// An original read mapped to its current function's own SSA symbol.
/// This readonly association supplies no successful read, physical value or edit authority.
pub struct OriginalSourceSsaRead<'a> {
    function: &'a crate::compilation_unit::FunctionUnit,
    span: tcl_lexer::Span,
    reference: SsaReadReference,
}
impl<'a> OriginalSourceSsaRead<'a> {
    /// Actual current compilation owner of this read.
    #[must_use]
    pub const fn function(&self) -> &'a crate::compilation_unit::FunctionUnit {
        self.function
    }
    /// Complete unchanged original substitution extent.
    #[must_use]
    pub const fn span(&self) -> tcl_lexer::Span {
        self.span
    }
    /// Canonical symbol retained by this function's original read producer.
    #[must_use]
    pub const fn symbol(&self) -> Symbol {
        self.reference.symbol
    }
    /// Represented contents version, when independently available.
    #[must_use]
    pub const fn version(&self) -> Option<Version> {
        self.reference.version
    }
}

/// Select an original read by exact document geometry and actual compilation ownership.
/// Missing read provenance never falls back to a variable or function display name.
#[must_use]
pub fn original_source_read_at_unit<'a>(
    unit: &'a crate::compilation_unit::CompilationUnit,
    span: tcl_lexer::Span,
    registry: &CommandRegistry,
) -> Option<OriginalSourceSsaRead<'a>> {
    let module = &unit.ir_module;
    if module.source.try_text().ok()? != unit.source {
        return None;
    }
    let input = module.source_metadata_input.as_ref();
    let retained = module.retained_source_bindings.as_deref()?;
    let positioned = retained.positioned_for_module(module, registry)?;
    let origin = positioned.source_origin()?;
    let mut selected: Option<OriginalSourceSsaRead<'a>> = None;
    for function in unit.all_body_function_units() {
        let current = if input.is_some() {
            current_analysis_function(unit, function, registry)
        } else {
            current_authored_analysis_function(unit, function, registry)
        };
        if !current {
            continue;
        }
        for ((block, index, source, spelling), reference) in function
            .ssa
            .read_references
            .iter()
            .chain(function.ssa.source_read_references.iter())
        {
            if source.provenance != crate::ir::Provenance::Source {
                continue;
            }
            let start =
                u32::try_from(i64::from(source.span.start()).checked_add(function.base_offset)?)
                    .ok()?;
            let end =
                u32::try_from(i64::from(source.span.end()).checked_add(function.base_offset)?)
                    .ok()?;
            if tcl_lexer::Span::new(start, end) != span
                || unit.source.get(span.as_range()) != Some(spelling.as_str())
            {
                continue;
            }
            let view = SsaSourceView::at_statement(&function.ssa, *block, *index);
            let Some(tokens) = view.source_tokens() else {
                continue;
            };
            let mut original = tokens.clone();
            crate::lattice_rebase::rebase_command_tokens(&mut original, function.base_offset);
            let Some(binding) = original.source_binding.as_ref() else {
                continue;
            };
            if !retained.owns_original_analysis_attachment(&original) {
                continue;
            }
            let owner = if input.is_none() {
                !binding.has_original_native_source_command()
                    && !binding.has_original_logical_source_advice()
                    && owns_original_authored_analysis_tokens(unit, function, &original, registry)
            } else if binding.has_original_native_source_command() {
                function.body_id.is_some_and(|body| {
                    binding.original_native_source_owner_for_body(&original, module, body, registry)
                }) && binding
                    .original_native_source_words(&original, input?, registry)
                    .is_some()
            } else if binding.has_original_logical_source_advice() {
                binding.original_logical_source_words(&original, input?, registry).is_some_and(|words| {
                    matches!(words.source(), crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(advice)
                        if std::sync::Arc::ptr_eq(&advice.site().source, origin))
                })
            } else {
                function.body_id.map_or_else(
                    || {
                        retained.owns_original_tokens(&original)
                            && binding
                                .invocation_site()
                                .is_some_and(|site| std::sync::Arc::ptr_eq(&site.source, origin))
                    },
                    |body| retained.owns_original_body_tokens(module, body, &original, registry),
                )
            };
            if !owner {
                continue;
            }
            let reference = function
                .ssa
                .read_references
                .get(&(*block, *index, source.clone(), spelling.clone()))
                .copied()
                .unwrap_or(*reference);
            if selected.as_ref().is_some_and(|old| {
                !std::ptr::eq(old.function, function) || old.reference != reference
            }) {
                return None;
            }
            selected = Some(OriginalSourceSsaRead {
                function,
                span,
                reference,
            });
        }
    }
    selected
}

/// One exact source definition in the current compilation owner's analysis.
/// Its SSA version supplies no physical cell, completed store or edit authority.
pub struct OriginalAnalysisSsaDefinition<'a> {
    function: &'a crate::compilation_unit::FunctionUnit,
    span: tcl_lexer::Span,
    symbol: Symbol,
    version: Version,
    source_name: Vec<u8>,
}
impl<'a> OriginalAnalysisSsaDefinition<'a> {
    /// Actual current function owning this definition.
    #[must_use]
    pub const fn function(&self) -> &'a crate::compilation_unit::FunctionUnit {
        self.function
    }
    /// Exact original written variable operand extent.
    #[must_use]
    pub const fn span(&self) -> tcl_lexer::Span {
        self.span
    }
    /// Symbol from this function's own source operation.
    #[must_use]
    pub const fn symbol(&self) -> Symbol {
        self.symbol
    }
    /// Version at this represented definition, independently of subsequent writes.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.version
    }
    /// Exact literal receiver value retained by this original definition's
    /// source producer. It is not a reference spelling or a physical cell.
    #[must_use]
    pub fn source_name_bytes(&self) -> &[u8] {
        &self.source_name
    }
}

fn original_written_source_variable_name(
    words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
    written: usize,
    span: tcl_lexer::Span,
) -> Option<&str> {
    let mut selected = words
        .origins()
        .iter()
        .enumerate()
        .filter_map(|(effective, origin)| {
            (*origin
                == crate::registry_invocation::InvocationWordOrigin::Written(
                    written.checked_add(1)?,
                ))
            .then(|| effective.checked_sub(1))
            .flatten()
        });
    let argument = selected.next()?;
    if selected.next().is_some() || words.operands().get(argument)?.as_ref()?.span() != span {
        return None;
    }
    std::str::from_utf8(words.arguments().get(argument)?.literal_bytes()?).ok()
}

/// Exact source word geometry at one current represented analysis operation.
/// This supplies no read value, completed invocation, Normal or edit authority.
pub struct OriginalAnalysisSsaWords {
    words: Vec<tcl_lexer::NativeWord>,
    config: tcl_lexer::LexerConfig,
}

impl OriginalAnalysisSsaWords {
    /// Complete original lexical vector in document coordinates.
    #[must_use]
    pub fn words(&self) -> &[tcl_lexer::NativeWord] {
        &self.words
    }

    /// Actual grammar used to authenticate this vector.
    #[must_use]
    pub const fn config(&self) -> tcl_lexer::LexerConfig {
        self.config
    }
}

/// Authenticate the actual function, body, input and whole original word vector.
/// Present readonly child purposes remain terminal when their owner refuses.
#[must_use]
pub fn original_analysis_words_for_statement(
    unit: &crate::compilation_unit::CompilationUnit,
    function: &crate::compilation_unit::FunctionUnit,
    block: BlockId,
    index: usize,
    registry: &CommandRegistry,
) -> Option<OriginalAnalysisSsaWords> {
    if !current_analysis_function(unit, function, registry) {
        return None;
    }
    let module = &unit.ir_module;
    let input = module.source_metadata_input.as_ref()?;
    let retained = module.retained_source_bindings.as_deref()?;
    let positioned = retained.positioned_for_module(module, registry)?;
    let origin = positioned.source_origin()?;
    let tokens = SsaSourceView::at_statement(&function.ssa, block, index).source_tokens()?;
    let mut original = tokens.clone();
    crate::lattice_rebase::rebase_command_tokens(&mut original, function.base_offset);
    let binding = original.source_binding.as_ref()?;
    retained
        .owns_original_analysis_attachment(&original)
        .then_some(())?;
    let config = if binding.has_original_native_source_command() {
        let body = function.body_id?;
        if !binding.original_native_source_owner_for_body(&original, module, body, registry) {
            return None;
        }
        binding.original_native_source_words(&original, input, registry)?;
        input.lexer_config()
    } else if binding.has_original_logical_source_advice() {
        let words = binding.original_logical_source_words(&original, input, registry)?;
        if !matches!(words.source(), crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(advice)
            if std::sync::Arc::ptr_eq(&advice.site().source, origin))
        {
            return None;
        }
        input.lexer_config()
    } else {
        let owned = function.body_id.map_or_else(
            || {
                retained.owns_original_tokens(&original)
                    && binding
                        .invocation_site()
                        .is_some_and(|site| std::sync::Arc::ptr_eq(&site.source, origin))
            },
            |body| retained.owns_original_body_tokens(module, body, &original, registry),
        );
        if !owned {
            return None;
        }
        binding.original_lexer_config_for_tokens(&original)?
    };
    let offset = original.words().first()?.source().span.start();
    let words = crate::registry_invocation::original_native_compiler_words(
        &module.source,
        original.words(),
        offset,
        config,
    )?;
    Some(OriginalAnalysisSsaWords { words, config })
}

fn current_analysis_function(
    unit: &crate::compilation_unit::CompilationUnit,
    function: &crate::compilation_unit::FunctionUnit,
    registry: &CommandRegistry,
) -> bool {
    let module = &unit.ir_module;
    let Some(input) = module.source_metadata_input.as_ref() else {
        return false;
    };
    let Some(retained) = module.retained_source_bindings.as_deref() else {
        return false;
    };
    if module.source.try_text().ok() != Some(unit.source.as_str())
        || retained.positioned_for_module(module, registry).is_none()
        || unit
            .function_for_body(function.body_id)
            .is_none_or(|current| !std::ptr::eq(current, function))
        || function
            .invocation_metadata_context_for_module(registry, module)
            .is_none()
        || function
            .source_metadata_input()
            .is_none_or(|held| held != input && held != &input.for_nested_source())
    {
        return false;
    }
    match function.body_id {
        Some(body) => unit
            .original_procedure_for_body(body)
            .is_some_and(|procedure| {
                retained.matches_original_procedure_summary(body, procedure)
                    && unit.original_procedure_cfg_for_body(body) == Some(&function.cfg)
            }),
        None => function.cfg == unit.cfg_module.top_level,
    }
}

/// Current original authored analysis graph; no execution or frame authority.
pub(crate) fn current_authored_analysis_function(
    unit: &crate::compilation_unit::CompilationUnit,
    function: &crate::compilation_unit::FunctionUnit,
    registry: &CommandRegistry,
) -> bool {
    let module = &unit.ir_module;
    let Some(retained) = module.retained_source_bindings.as_deref() else {
        return false;
    };
    if module.source_metadata_input.is_some()
        || !module.source_entry.metadata_context.is_standalone()
        || module.source_entry.native_entry.is_some()
        || module.source_entry.logical_source_input.is_some()
        || module.source_entry.vendor_source_input.is_some()
        || module.source_entry.hosted_execution_context.is_some()
        || module.source_entry.unknown_entry
        || function.source_metadata_input().is_some()
        || function.source_lexer_config().nested().normalized()
            != module.native_lexer_config().nested().normalized()
        || module.source.try_text().ok() != Some(unit.source.as_str())
        || retained.positioned_for_module(module, registry).is_none()
        || unit
            .function_for_body(function.body_id)
            .is_none_or(|current| !std::ptr::eq(current, function))
    {
        return false;
    }
    match function.body_id {
        Some(body) => unit
            .original_procedure_for_body(body)
            .is_some_and(|procedure| {
                module
                    .original_authored_declaration_body_units
                    .get(&body)
                    .is_some_and(|card| card.matches_procedure(module, procedure))
                    && retained.matches_original_procedure_summary(body, procedure)
                    && unit.original_procedure_cfg_for_body(body) == Some(&function.cfg)
            }),
        None => function.cfg == unit.cfg_module.top_level,
    }
}

fn owns_original_authored_analysis_tokens(
    unit: &crate::compilation_unit::CompilationUnit,
    function: &crate::compilation_unit::FunctionUnit,
    original: &CommandTokens,
    registry: &CommandRegistry,
) -> bool {
    let module = &unit.ir_module;
    let Some(retained) = module.retained_source_bindings.as_deref() else {
        return false;
    };
    let Some(binding) = original.source_binding.as_ref() else {
        return false;
    };
    if !retained.owns_original_analysis_attachment(original)
        || original.synthetic.is_some()
        || !original.original_words_match_argv_text()
        || binding.has_original_native_source_command()
        || binding.has_original_logical_source_advice()
    {
        return false;
    }
    if binding.has_original_authored_source_body_name_role_owner() {
        let Some(scope) = binding
            .original_authored_scalar_body_scope(original, registry)
            .or_else(|| {
                binding.original_authored_source_body_scope_for_name_roles(original, registry)
            })
        else {
            return false;
        };
        let Some(body) = function.body_id else {
            return false;
        };
        scope.matches_module(module)
            && scope.original_words(original, registry).is_some()
            && module
                .original_authored_declaration_body_units
                .get(&body)
                .is_some_and(|card| card.owns_original_source_scope(scope))
    } else {
        function.body_id.is_none() && retained.owns_original_tokens(original)
    }
}

fn original_authored_definition_operands(
    unit: &crate::compilation_unit::CompilationUnit,
    function: &crate::compilation_unit::FunctionUnit,
    original: &CommandTokens,
    registry: &CommandRegistry,
) -> Option<Vec<(tcl_lexer::Span, String)>> {
    let module = &unit.ir_module;
    let binding = original.source_binding.as_ref()?;
    if binding.has_original_native_source_command()
        || binding.has_original_logical_source_advice()
        || original.synthetic.is_some()
        || !original.original_words_match_argv_text()
    {
        return None;
    }
    let roles = if binding.has_original_authored_source_body_name_role_owner() {
        let scope = binding
            .original_authored_scalar_body_scope(original, registry)
            .or_else(|| {
                binding.original_authored_source_body_scope_for_name_roles(original, registry)
            })?;
        if !scope.matches_module(module) || scope.has_unclosed_declaration_source_coverage() {
            return None;
        }
        let body = function.body_id?;
        module
            .original_authored_declaration_body_units
            .get(&body)?
            .owns_original_source_scope(scope)
            .then_some(())?;
        let words = scope.original_words(original, registry)?;
        if words.iter().any(|word| word.group().expand) {
            return None;
        }
        scope.with_source_schema(original, registry, |schema| {
            let (roles, complete) = schema.authored_source_argument_roles();
            (complete && schema.semantics.frame_effect.is_none()).then(|| {
                roles
                    .into_iter()
                    .map(|(argument, role)| {
                        (schema.facts().argument_offset + usize::from(argument), role)
                    })
                    .collect::<Vec<_>>()
            })
        })?
    } else {
        function.body_id.is_none().then_some(())?;
        let retained = module.retained_source_bindings.as_deref()?;
        retained.owns_original_tokens(original).then_some(())?;
        let grammar =
            crate::registry_invocation::original_authored_source_structure(original, registry)?
                .into_closed_source_grammar()?;
        if grammar.has_original_word_expansion()
            || grammar.config().nested().normalized()
                != function.source_lexer_config().nested().normalized()
        {
            return None;
        }
        grammar.with_source_schema(registry, |schema| {
            schema.semantics.frame_effect.is_none().then_some(())
        })?;
        grammar.written_roles()
    };
    let offset = original.words().first()?.source().span.start();
    let written = crate::registry_invocation::original_native_compiler_words(
        &module.source,
        original.words(),
        offset,
        module.native_lexer_config(),
    )?;
    roles
        .into_iter()
        .filter(|(_, role)| *role == tcl_registry::ArgRole::VarWrite)
        .map(|(argument, _)| {
            let word = written.get(argument.checked_add(1)?)?;
            if word.group().expand {
                return None;
            }
            let value = tcl_syntax::word_rules::original_static_word_unicode_value(word)?;
            let name = std::str::from_utf8(&value).ok()?.to_owned();
            Some((word.span(), name))
        })
        .collect()
}

fn original_authored_analysis_definition_at_unit<'a>(
    unit: &'a crate::compilation_unit::CompilationUnit,
    span: tcl_lexer::Span,
    registry: &CommandRegistry,
) -> Option<OriginalAnalysisSsaDefinition<'a>> {
    let mut selected: Option<OriginalAnalysisSsaDefinition<'a>> = None;
    for function in unit.all_body_function_units() {
        if !current_authored_analysis_function(unit, function, registry) {
            continue;
        }
        for (&block, body) in &function.ssa.blocks {
            for (index, statement) in body.statements.iter().enumerate() {
                let view = SsaSourceView::at_statement(&function.ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                let mut original = tokens.clone();
                crate::lattice_rebase::rebase_command_tokens(&mut original, function.base_offset);
                if unit
                    .ir_module
                    .retained_source_bindings
                    .as_deref()
                    .is_none_or(|retained| !retained.owns_original_analysis_attachment(&original))
                {
                    continue;
                }
                let Some(operands) =
                    original_authored_definition_operands(unit, function, &original, registry)
                else {
                    continue;
                };
                for (operand_span, name) in operands {
                    if operand_span != span {
                        continue;
                    }
                    let Some(symbol) = view.symbol(&name) else {
                        continue;
                    };
                    let Some(&version) = statement.defs.get(&symbol) else {
                        continue;
                    };
                    if statement.may_defs.contains(&symbol) {
                        continue;
                    }
                    if selected.as_ref().is_some_and(|old| {
                        !std::ptr::eq(old.function, function)
                            || old.symbol != symbol
                            || old.version != version
                            || old.source_name != name.as_bytes()
                    }) {
                        return None;
                    }
                    selected = Some(OriginalAnalysisSsaDefinition {
                        function,
                        span,
                        symbol,
                        version,
                        source_name: name.into_bytes(),
                    });
                }
            }
        }
    }
    selected
}

fn merge_analysis_definitions<'a>(
    selected: &mut Option<OriginalAnalysisSsaDefinition<'a>>,
    function: &'a crate::compilation_unit::FunctionUnit,
    span: tcl_lexer::Span,
    definitions: AnalysisDefinitionRows,
) -> Option<()> {
    for (definition_span, symbol, version, source_name) in definitions {
        if definition_span != span {
            continue;
        }
        if selected.as_ref().is_some_and(|old| {
            !std::ptr::eq(old.function, function)
                || old.symbol != symbol
                || old.version != version
                || old.source_name != source_name
        }) {
            return None;
        }
        *selected = Some(OriginalAnalysisSsaDefinition {
            function,
            span,
            symbol,
            version,
            source_name,
        });
    }
    Some(())
}

type AnalysisDefinitionRows = Vec<(tcl_lexer::Span, Symbol, Version, Vec<u8>)>;

struct LogicalAnalysisDefinitionContext<'a> {
    module: &'a crate::ir::Module,
    function: &'a crate::compilation_unit::FunctionUnit,
    registry: &'a CommandRegistry,
    input: &'a crate::analyser::ResolvedAnalysisInput,
    retained: &'a crate::command_binding::RetainedSourceModuleBindings,
    positioned: &'a crate::command_binding::SourceCommandBindings,
    origin: &'a std::sync::Arc<crate::command_binding::SourceOriginId>,
}

fn logical_analysis_definitions(
    context: &LogicalAnalysisDefinitionContext<'_>,
    binding: &crate::command_binding::SourceInvocationBinding,
    original: &crate::ir::CommandTokens,
    view: SsaSourceView<'_>,
    statement: &SsaStatement,
) -> Option<AnalysisDefinitionRows> {
    let LogicalAnalysisDefinitionContext {
        module,
        function,
        registry,
        input,
        retained,
        positioned,
        origin,
    } = *context;
    let mut definitions = Vec::new();
    if binding.has_original_logical_source_advice() {
        let words = binding.original_logical_source_words(original, input, registry)?;
        if !matches!(words.source(), crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(advice)
            if std::sync::Arc::ptr_eq(&advice.site().source, origin))
        {
            return None;
        }
        collect_logical_advice_definitions(
            &words,
            original,
            module,
            view,
            statement,
            &mut definitions,
        );
    } else {
        let owned = function.body_id.map_or_else(
            || retained.owns_original_tokens(original),
            |body| retained.owns_original_body_tokens(module, body, original, registry),
        );
        if !owned {
            return None;
        }
        let metadata = function.invocation_metadata_context_for_module(registry, module)?;
        let invocation =
            crate::registry_invocation::logical_structured_invocation_with_metadata_context(
                registry,
                metadata,
                original,
                Some(positioned),
            )?;
        for (argument, role) in invocation.written_roles() {
            if role != tcl_registry::ArgRole::VarWrite {
                continue;
            }
            let Some(word) = original.words().get(argument + 1) else {
                continue;
            };
            let Some(name) = invocation.written_variable_name_operand(argument) else {
                continue;
            };
            let Some(symbol) = view.symbol(&name) else {
                continue;
            };
            let Some(&version) = statement.defs.get(&symbol) else {
                continue;
            };
            if !statement.may_defs.contains(&symbol) {
                definitions.push((
                    word.source().span,
                    symbol,
                    version,
                    name.as_bytes().to_vec(),
                ));
            }
        }
    }
    Some(definitions)
}

fn collect_logical_advice_definitions(
    words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
    original: &crate::ir::CommandTokens,
    module: &crate::ir::Module,
    view: SsaSourceView<'_>,
    statement: &SsaStatement,
    definitions: &mut AnalysisDefinitionRows,
) {
    for (argument, role) in words.written_argument_roles() {
        if role != tcl_registry::ArgRole::VarWrite {
            continue;
        }
        let Some(word) = original.words().get(argument + 1) else {
            continue;
        };
        let Some(receiver_span) = word.original_written_span(module.source.bytes()) else {
            continue;
        };
        let Some(name) = original_written_source_variable_name(words, argument, receiver_span)
        else {
            continue;
        };
        let Some(symbol) = view.symbol(name) else {
            continue;
        };
        let Some(&version) = statement.defs.get(&symbol) else {
            continue;
        };
        if !statement.may_defs.contains(&symbol) {
            definitions.push((receiver_span, symbol, version, name.as_bytes().to_vec()));
        }
    }
}

/// Select a represented source definition by its exact current written occurrence.
/// Source body purposes remain separate from physical Native definition receipts.
#[must_use]
pub fn original_analysis_definition_at_unit<'a>(
    unit: &'a crate::compilation_unit::CompilationUnit,
    span: tcl_lexer::Span,
    registry: &CommandRegistry,
) -> Option<OriginalAnalysisSsaDefinition<'a>> {
    let module = &unit.ir_module;
    if module.source_metadata_input.is_none() {
        return original_authored_analysis_definition_at_unit(unit, span, registry);
    }
    let input = module.source_metadata_input.as_ref()?;
    let retained = module.retained_source_bindings.as_deref()?;
    let positioned = retained.positioned_for_module(module, registry)?;
    let origin = positioned.source_origin()?;
    let mut selected: Option<OriginalAnalysisSsaDefinition<'a>> = None;
    for function in unit.all_body_function_units() {
        if !current_analysis_function(unit, function, registry) {
            continue;
        }
        for (&block, body) in &function.ssa.blocks {
            for (index, statement) in body.statements.iter().enumerate() {
                let view = SsaSourceView::at_statement(&function.ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                let mut original = tokens.clone();
                crate::lattice_rebase::rebase_command_tokens(&mut original, function.base_offset);
                let Some(binding) = original.source_binding.as_ref() else {
                    continue;
                };
                if !retained.owns_original_analysis_attachment(&original) {
                    continue;
                }
                let mut definitions = Vec::new();
                if binding.has_original_native_source_command() {
                    let Some(body) = function.body_id else {
                        continue;
                    };
                    for (&symbol, &version) in &statement.defs {
                        if let Some(name) =
                            view.original_source_definition_for_body(symbol, unit, body, registry)
                        {
                            definitions.push((
                                name.span(),
                                symbol,
                                version,
                                name.original_name_input().bytes().to_vec(),
                            ));
                        }
                    }
                } else if input.has_logical_source_name_context() {
                    let context = LogicalAnalysisDefinitionContext {
                        module,
                        function,
                        registry,
                        input,
                        retained,
                        positioned,
                        origin,
                    };
                    let Some(logical_definitions) =
                        logical_analysis_definitions(&context, binding, &original, view, statement)
                    else {
                        continue;
                    };
                    definitions = logical_definitions;
                } else {
                    let metadata =
                        function.invocation_metadata_context_for_module(registry, module)?;
                    for (&symbol, &version) in &statement.defs {
                        if statement.may_defs.contains(&symbol) {
                            continue;
                        }
                        if let Some(name) = view.original_definition_name_with_metadata_context(
                            symbol,
                            registry,
                            Some(metadata),
                        ) {
                            let offset = function.base_offset;
                            let start =
                                u32::try_from(i64::from(name.span().start()).checked_add(offset)?)
                                    .ok()?;
                            let end =
                                u32::try_from(i64::from(name.span().end()).checked_add(offset)?)
                                    .ok()?;
                            definitions.push((
                                tcl_lexer::Span::new(start, end),
                                symbol,
                                version,
                                name.original_name_input().bytes().to_vec(),
                            ));
                        }
                    }
                }
                merge_analysis_definitions(&mut selected, function, span, definitions)?;
            }
        }
    }
    selected
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
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_SSA_DEFINITION").is_some() {
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

    /// Readonly literal source fact, not a represented cell or raw SSA value.
    /// Missing graph custody refuses; incomplete source coverage stays unknown.
    pub(crate) fn original_script_literal_read_coverage(
        self,
        key: Option<ValueKey>,
    ) -> Option<bool> {
        let (block, index) = self.point?;
        let body = self.ssa.blocks.get(&block)?;
        let held = self
            .ssa
            .source_topology
            .get(&block)?
            .standalone_statements
            .as_ref()?;
        if body.statements.len() != held.len()
            || body
                .statements
                .iter()
                .zip(held.iter())
                .any(|(current, original)| &current.statement != original)
        {
            return None;
        }
        let Some((symbol, version)) = key else {
            return Some(false);
        };
        let Some(read) = self.ssa.original_script_points.get(&(block, index)) else {
            return Some(false);
        };
        let current = body.statements.get(index)?;
        if read.symbol != symbol
            || read.used != Some(version)
            || read.access.is_definition()
            || current.uses.get(&symbol) != read.used.as_ref()
        {
            return None;
        }
        for prior in (0..index).rev() {
            let Some(write) = self.ssa.original_script_points.get(&(block, prior)) else {
                return Some(false);
            };
            // Only closed, quiet, entirely literal Set source steps survive.
            // Any read/callback/unknown/array/multi-name/synthetic step stops.
            if write.access != crate::registry_invocation::AuthoredSourceVariableAccess::Write {
                return Some(false);
            }
            let statement = body.statements.get(prior)?;
            if write.defined.is_none()
                || statement.defs.get(&write.symbol) != write.defined.as_ref()
                || statement.may_defs.contains(&write.symbol)
            {
                return None;
            }
            if write.symbol == symbol {
                return Some(write.defined == Some(version));
            }
            // Distinct interned source identities have different counted
            // namespace slots, under each step's proved alias/trace closure.
        }
        Some(false)
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
        self.externally_mutable_given(
            symbol,
            names,
            dynamic_trace,
            self.ssa.has_opaque_native_accesses(),
            registry,
        )
    }

    /// [`Self::externally_mutable_by`] with whether the function's running
    /// code has opaque native accesses already decided by the caller.
    #[must_use]
    pub fn externally_mutable_given(
        self,
        symbol: Symbol,
        names: &HashSet<String>,
        dynamic_trace: bool,
        opaque_native_accesses: bool,
        registry: &CommandRegistry,
    ) -> Option<bool> {
        let key = self.ssa.cell_key(symbol)?;
        if opaque_native_accesses
            || dynamic_trace
            || key
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
                if &bound == key {
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
        let Some(key) = self.ssa.cell_key(symbol) else {
            return false;
        };
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
                if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
                    eprintln!("ORIGINAL_STORE_CONTENTS site={} captured={} cell_matches={} dynamic={} presence={:?} generation_current={} source_matches={} offset_matches={}",
                        site.offset, captured.is_some(),
                        crate::var_resolve::canonical_binding_value_key(place).as_ref() == Some(key),
                        place.dynamic, if captured.is_some() { after.captured_contents_presence(place) } else { after.contents_presence(place) },
                        place.cell.as_ref().is_some_and(|cell| cell.generation != crate::place::CellGeneration::Unknown
                            && after.generations.get(&crate::var_resolve::cell_key(place)).copied().unwrap_or_default() == cell.generation),
                        after.contents_have_source(place, &site.source),
                        after.contents_origin(place) == crate::var_resolve::ContentsOrigin::WrittenAt(site.offset));
                }
                crate::var_resolve::canonical_binding_value_key(place).as_ref()
                    == Some(key)
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

    /// Conditional contents of this exact represented Logical scalar setter.
    /// The actual source continuation owns the literal, cell and write site;
    /// this grants no Native store, completed read, Normal or erasure receipt.
    pub(crate) fn conditional_logical_scalar_store_contents(
        self,
        symbol: Symbol,
        registry: &CommandRegistry,
    ) -> Option<String> {
        let key = self.ssa.cell_key(symbol)?;
        let (block, index) = self.point?;
        let statement = self.ssa.blocks.get(&block)?.statements.get(index)?;
        if statement.defs.len() != 1 || !statement.defs.contains_key(&symbol) {
            return None;
        }
        let points = self.ssa.point_contexts.as_ref()?;
        let store = points.conditional_logical_scalar_store_at(block, index, registry)?;
        let place = store.definition_place(registry)?;
        (crate::var_resolve::canonical_binding_value_key(&place).as_ref() == Some(key))
            .then(|| store.value().map(str::to_owned))
            .flatten()
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
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_SSA_DEFINITION").is_some() {
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
                match self.ssa.cell_key(symbol)?.root() {
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
        let key = self.ssa.cell_key(symbol)?;
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

    /// Readonly definition geometry from the actual Native source body/schema.
    /// This separate source projection cannot replace a physical definition query.
    #[must_use]
    pub fn original_source_definition_for_body(
        self,
        symbol: Symbol,
        unit: &crate::compilation_unit::CompilationUnit,
        body: crate::ir::ProcedureId,
        registry: &CommandRegistry,
    ) -> Option<OriginalSourceSsaDefinitionName> {
        let function = unit.function_for_body(Some(body))?;
        if !std::ptr::eq(self.ssa, std::ptr::from_ref(&function.ssa)) {
            return None;
        }
        let module = &unit.ir_module;
        let card = module.original_native_declaration_body_units.get(&body)?;
        let procedure = module.body_units.get(&body)?;
        if !card.matches_procedure(module, procedure) {
            return None;
        }
        let input = module.source_metadata_input.as_ref()?;
        let metadata = function.invocation_metadata_context_for_module(registry, module)?;
        let function_input = metadata.source_analysis_input()?;
        if (function_input != input && function_input != &input.for_nested_source())
            || function.source_lexer_config().nested().normalized()
                != input.lexer_config().nested().normalized()
        {
            return None;
        }
        let (block, index) = self.point?;
        let statement = self.ssa.blocks.get(&block)?.statements.get(index)?;
        if !statement.defs.contains_key(&symbol) || statement.may_defs.contains(&symbol) {
            return None;
        }
        let slot = self.ssa.cell_key(symbol)?.authored_spelling()?;
        let mut tokens = self.source_tokens()?.clone();
        crate::lattice_rebase::rebase_command_tokens(&mut tokens, function.base_offset);
        let binding = tokens.source_binding.as_ref()?;
        if !binding.original_native_source_owner_for_body(&tokens, module, body, registry) {
            return None;
        }
        let words = binding.original_native_source_words(&tokens, input, registry)?;
        let mut found = None;
        for (argument, role) in words.written_argument_roles() {
            if role != tcl_registry::ArgRole::VarWrite {
                continue;
            }
            let operand = words.operands().get(argument)?.as_ref()?;
            let name = operand.input()?;
            let selected = name.policy().recipe().combined_variable_input(name.bytes());
            if selected.element().is_some()
                || selected.root().qualification()
                    != tcl_syntax::naming::NativeNameQualification::Unqualified
                || selected.root().selected() != slot.as_bytes()
            {
                continue;
            }
            let current = (operand.span(), name.clone());
            if found.as_ref().is_some_and(|previous| previous != &current) {
                return None;
            }
            found = Some(current);
        }
        let (span, input) = found?;
        Some(OriginalSourceSsaDefinitionName { span, input, body })
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
        let VariableCellKey::Activation { identity, .. } = self.ssa.cell_key(symbol)?.root() else {
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
        let mut contains_defined = false;
        for context in access.context_alternatives() {
            let place = access.place_in_context(context, registry);
            if place.observed {
                return None;
            }
            match context.closed_contents_presence(&place)? {
                ContentsPresence::Defined => contains_defined = true,
                ContentsPresence::Undefined => contains_undefined = true,
                ContentsPresence::DefinedOrUndefined => {
                    contains_undefined = true;
                    contains_defined = true;
                }
                ContentsPresence::Unknown => return None,
            }
        }
        Some(SsaReadPresenceAlternatives {
            contains_undefined,
            contains_defined,
        })
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
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
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
        let key = self.ssa.cell_key(reference.symbol)?;
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
                    && crate::var_resolve::canonical_binding_value_key(&place).as_ref() == Some(key)
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
        let Some(key) = self.ssa.cell_key(symbol) else {
            return SsaReadRepresentationAdvice::default();
        };
        captured_read_representations(
            reads
                .native_reads
                .iter()
                .filter(|read| {
                    crate::var_resolve::canonical_binding_value_key(&read.place).as_ref()
                        == Some(key)
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
        let config = tokens
            .source_binding
            .as_ref()?
            .original_argument_lexer_config(tokens)?;
        let span = node.variable_source_span(expr_base?, config)?;
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

fn original_source_may_call(
    function: &cfg::Function,
    statement: &Statement,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    registry: &CommandRegistry,
) -> Option<crate::cfg_builder::global_write_info::OriginalSourceCallMayWrites> {
    original_source_may_call_from_inventory(
        &function.source_may_calls,
        statement,
        metadata,
        registry,
    )
}

fn original_source_may_call_from_inventory(
    inventory: &HashMap<
        (usize, u32),
        Vec<crate::cfg_builder::global_write_info::OriginalSourceCallMayWrites>,
    >,
    statement: &Statement,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    registry: &CommandRegistry,
) -> Option<crate::cfg_builder::global_write_info::OriginalSourceCallMayWrites> {
    let site = statement
        .tokens()?
        .source_binding
        .as_ref()?
        .invocation_site()?;
    let candidates =
        inventory.get(&(std::sync::Arc::as_ptr(&site.source) as usize, site.offset))?;
    let mut matching = candidates
        .iter()
        .filter(|receipt| receipt.matches_statement(statement, metadata, registry));
    let first = matching.next()?;
    matching
        .all(|next| first.same_owner(next))
        .then(|| first.clone())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SsaVariableIdentity {
    Location(VariableCellKey),
    Source(crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol),
}

// Buckets retain full authenticated identities; hashes never decide equality.
struct SsaIdentityBuckets<V, S = rustc_hash::FxBuildHasher> {
    buckets: HashMap<u64, Vec<(SsaVariableIdentity, V)>, S>,
}

impl<V, S: Default> Default for SsaIdentityBuckets<V, S> {
    fn default() -> Self {
        Self {
            buckets: HashMap::with_hasher(S::default()),
        }
    }
}

impl<V, S: std::hash::BuildHasher> SsaIdentityBuckets<V, S> {
    fn contains_key(&self, key: &SsaVariableIdentity) -> bool {
        let hash = std::hash::BuildHasher::hash_one(self.buckets.hasher(), key);
        self.buckets
            .get(&hash)
            .is_some_and(|bucket| bucket.iter().any(|(stored, _)| stored == key))
    }

    fn insert(&mut self, key: SsaVariableIdentity, value: V) {
        let hash = std::hash::BuildHasher::hash_one(self.buckets.hasher(), &key);
        let bucket = self.buckets.entry(hash).or_default();
        if let Some((_, stored_value)) = bucket.iter_mut().find(|(stored, _)| stored == &key) {
            *stored_value = value;
        } else {
            bucket.push((key, value));
        }
    }

    fn iter(&self) -> impl Iterator<Item = (&SsaVariableIdentity, &V)> {
        self.buckets
            .values()
            .flatten()
            .map(|(key, value)| (key, value))
    }
}

impl<V: Default, S: std::hash::BuildHasher> SsaIdentityBuckets<V, S> {
    fn or_default(&mut self, key: SsaVariableIdentity) -> &mut V {
        let hash = std::hash::BuildHasher::hash_one(self.buckets.hasher(), &key);
        let bucket = self.buckets.entry(hash).or_default();
        let position = bucket.iter().position(|(stored, _)| stored == &key);
        let index = if let Some(position) = position {
            position
        } else {
            bucket.push((key, V::default()));
            bucket.len() - 1
        };
        &mut bucket[index].1
    }
}

impl<V, S> FromIterator<(SsaVariableIdentity, V)> for SsaIdentityBuckets<V, S>
where
    S: std::hash::BuildHasher + Default,
{
    fn from_iter<T: IntoIterator<Item = (SsaVariableIdentity, V)>>(iter: T) -> Self {
        let mut result = Self::default();
        for (key, value) in iter {
            result.insert(key, value);
        }
        result
    }
}

impl<V, S> IntoIterator for SsaIdentityBuckets<V, S> {
    type Item = (SsaVariableIdentity, V);
    type IntoIter = std::iter::Flatten<
        std::collections::hash_map::IntoValues<u64, Vec<(SsaVariableIdentity, V)>>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.buckets.into_values().flatten()
    }
}

type SsaVisibleIdentities = SsaIdentityBuckets<(), std::collections::hash_map::RandomState>;
type SsaVisibleVersions = SsaIdentityBuckets<Version, std::collections::hash_map::RandomState>;
type SsaIdentitySet = SsaIdentityBuckets<()>;
type SsaDefinitionSites = SsaIdentityBuckets<FxHashSet<BlockId>>;

/// Per-[`SsaFunction`] interner for location and source identities.
///
/// Each identity receives one dense [`Symbol`]; display labels and the
/// location-only inventory retain their separate projections.
#[derive(Debug, Default, Clone)]
struct VarInterner {
    names: Vec<String>,
    keys: Vec<VariableCellKey>,
    cell_indices: Vec<Option<usize>>,
    source_variables:
        FxHashMap<Symbol, crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol>,
    source_to_symbol:
        FxHashMap<crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol, Symbol>,
    to_symbol: FxHashMap<VariableCellKey, Symbol>,
    source_symbols: FxHashMap<String, Symbol>,
}

impl VarInterner {
    fn intern(&mut self, key: &VariableCellKey) -> Symbol {
        if let Some(&symbol) = self.to_symbol.get(key) {
            return symbol;
        }
        let symbol =
            Symbol(u32::try_from(self.names.len()).expect("SSA variable count fits in u32"));
        self.names.push(key.compatibility_name());
        self.cell_indices.push(Some(self.keys.len()));
        self.keys.push(key.clone());
        self.to_symbol.insert(key.clone(), symbol);
        if let Some(name) = key.authored_spelling() {
            self.source_symbols.insert(name.to_owned(), symbol);
        }
        symbol
    }
    fn intern_identity(&mut self, identity: &SsaVariableIdentity) -> Symbol {
        match identity {
            SsaVariableIdentity::Location(key) => self.intern(key),
            SsaVariableIdentity::Source(source) => {
                if let Some(&symbol) = self.source_to_symbol.get(source) {
                    return symbol;
                }
                let symbol = Symbol(
                    u32::try_from(self.names.len()).expect("SSA variable count fits in u32"),
                );
                self.names.push(
                    source
                        .checked_c_namespace_key()
                        .unwrap_or_else(|| "source variable".to_owned()),
                );
                self.cell_indices.push(None);
                self.source_variables.insert(symbol, source.clone());
                self.source_to_symbol.insert(source.clone(), symbol);
                symbol
            }
        }
    }

    fn cell_key(&self, symbol: Symbol) -> Option<&VariableCellKey> {
        self.keys.get(
            self.cell_indices
                .get(symbol.0 as usize)
                .copied()
                .flatten()?,
        )
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
    /// One line per statement: its block, index, span, uses and defs. Failure
    /// messages print this, since the full `Debug` form repeats every shared
    /// token tree and can run to gigabytes.
    #[cfg(test)]
    pub(crate) fn outline(&self) -> String {
        use std::fmt::Write as _;
        let mut blocks: Vec<_> = self.blocks.iter().collect();
        blocks.sort_by_key(|(id, _)| **id);
        let mut out = String::new();
        for (id, block) in blocks {
            let _ = writeln!(out, "{id:?} {}", block.name);
            for (index, statement) in block.statements.iter().enumerate() {
                let mut uses: Vec<_> = statement.uses.iter().collect();
                uses.sort();
                let mut defs: Vec<_> = statement.defs.iter().collect();
                defs.sort();
                let _ = writeln!(
                    out,
                    "  #{index} {:?} uses={uses:?} defs={defs:?}",
                    statement.statement.span()
                );
            }
        }
        out
    }
}

impl SsaFunction {
    /// Source identities positively issued by actual standalone SCRIPT points.
    /// Duplicates are harmless; no physical location or contents is returned.
    pub(crate) fn original_script_source_symbols(&self) -> impl Iterator<Item = Symbol> + '_ {
        self.original_script_points
            .values()
            .map(|point| point.symbol)
    }

    /// Compare current CFG topology with the private original SSA source snapshot.
    /// Empty snapshots provide no correspondence proof.
    pub(crate) fn has_current_source_cfg_topology(&self, cfg: &cfg::Function) -> bool {
        !self.source_topology.is_empty()
            && self.entry == cfg.entry
            && self.source_topology.len() == cfg.blocks.len()
            && self.source_topology.iter().all(|(block, held)| {
                cfg.blocks.get(block).is_some_and(|current| {
                    held.standalone_statements.as_deref() == Some(current.statements.as_slice())
                        && current.terminator == held.terminator
                        && cfg.block_successors(*block) == held.successors
                })
            })
    }

    /// Whether named SSA maps omit native accesses or an incomplete original
    /// argument inventory. Empty named maps cannot establish absence of reads.
    #[must_use]
    pub fn has_opaque_native_accesses(&self) -> bool {
        self.has_opaque_native_accesses_in(&|_| true)
    }

    /// [`Self::has_opaque_native_accesses`] over only the blocks `admit`
    /// accepts: a block that never runs accesses nothing.
    #[must_use]
    pub fn has_opaque_native_accesses_in(&self, admit: &dyn Fn(BlockId) -> bool) -> bool {
        self.point_contexts
            .as_ref()
            .is_some_and(|points| points.has_unknown_source_arguments_in(admit))
            || crate::ir::statements_have_opaque_native_accesses(
                self.blocks
                    .iter()
                    .filter(|(id, _)| admit(**id))
                    .flat_map(|(_, block)| {
                        block
                            .statements
                            .iter()
                            .map(|statement| &statement.statement)
                    }),
            )
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
            cell_indices: Vec::new(),
            source_variables: FxHashMap::default(),
            source_may_effects: HashMap::new(),
            original_script_points: HashMap::new(),
            source_topology: HashMap::new(),
            var_to_symbol: FxHashMap::default(),
            cell_to_symbol: FxHashMap::default(),
            point_symbols: HashMap::new(),
            point_contexts: None,
            read_references: HashMap::new(),
            source_read_references: HashMap::new(),
            array_root_refresh_symbols: HashSet::new(),
            array_root_refresh_versions: HashSet::new(),
            unseen_call_versions: HashSet::new(),
        }
    }

    /// Whether version `version` of `sym` is the one the name holds at a call
    /// to code the module cannot see: that code may have read it, so no store
    /// to it is dead before the call. What the code may have written is the
    /// fresh version the marker leaves ([`Self::value_clobbers`]).
    #[must_use]
    pub fn is_observed_by_unseen_call(&self, sym: Symbol, version: Version) -> bool {
        self.unseen_call_versions.contains(&(sym, version))
    }

    /// [`Self::is_observed_by_unseen_call`] for a name spelled out.
    #[must_use]
    pub fn name_is_observed_by_unseen_call(&self, name: &str, version: Version) -> bool {
        self.var_to_symbol
            .get(name)
            .is_some_and(|&sym| self.is_observed_by_unseen_call(sym, version))
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
        self.cell_indices.push(Some(self.cell_keys.len()));
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

    /// Display label of this symbol's location, without currentness or Native permission.
    #[must_use]
    pub fn cell_name(&self, symbol: Symbol) -> Option<&str> {
        self.cell_names
            .get(
                self.cell_indices
                    .get(symbol.0 as usize)
                    .copied()
                    .flatten()?,
            )
            .map(String::as_str)
    }

    /// Location identity represented by this symbol, including conditional authored locations.
    /// Source-only variable geometry supplies no such projection.
    #[must_use]
    pub fn cell_key(&self, symbol: Symbol) -> Option<&VariableCellKey> {
        self.cell_keys.get(
            self.cell_indices
                .get(symbol.0 as usize)
                .copied()
                .flatten()?,
        )
    }

    /// Dense location inventory, independent of source-only symbols and display labels.
    /// Inventory positions are not SSA symbols; use [`Self::cell_symbols`] for pairs.
    #[must_use]
    pub fn cell_keys(&self) -> &[VariableCellKey] {
        &self.cell_keys
    }

    /// Actual SSA symbols paired with their location identity.
    pub fn cell_symbols(&self) -> impl Iterator<Item = (Symbol, &VariableCellKey)> {
        self.cell_indices
            .iter()
            .enumerate()
            .filter_map(|(symbol, index)| {
                let symbol = Symbol(u32::try_from(symbol).ok()?);
                Some((symbol, self.cell_keys.get((*index)?)?))
            })
    }

    /// Exact source identity, without a location, successful read or Normal completion.
    #[must_use]
    pub fn source_variable_symbol(
        &self,
        symbol: Symbol,
    ) -> Option<&crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol> {
        self.source_variables.get(&symbol)
    }

    /// Original source-only variable identities paired with their own SSA symbols.
    pub fn source_variable_symbols(
        &self,
    ) -> impl Iterator<
        Item = (
            Symbol,
            &crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
        ),
    > {
        self.source_variables
            .iter()
            .map(|(&symbol, variable)| (symbol, variable))
    }

    fn current_source_may_call(&self, block: BlockId, index: usize) -> Option<&SourceMaySsaCall> {
        let effect = self.source_may_effects.get(&(block, index))?;
        let statement = self.blocks.get(&block)?.statements.get(index)?;
        let original = effect.original.as_ref()?;
        let input = self
            .point_contexts
            .as_ref()?
            .source_metadata_owner()
            .source_analysis_input();
        if statement.statement != effect.statement
            || !original.matches_source_statement(&statement.statement, input)
            || effect.definitions.len() != original.writes().len()
            || effect.definitions.iter().zip(original.writes()).any(
                |(&(symbol, version), place)| {
                    statement.defs.get(&symbol) != Some(&version)
                        || !statement.may_defs.contains(&symbol)
                        || self.source_variable_symbol(symbol) != Some(place.symbol())
                },
            )
        {
            return None;
        }
        Some(effect)
    }

    /// Hidden source MAY effects cannot issue a call outcome or a location relation.
    pub(crate) fn source_may_requires_unknown_at(&self, block: BlockId, index: usize) -> bool {
        let logical = self
            .point_contexts
            .as_ref()
            .and_then(|points| points.source_metadata_owner().source_analysis_input())
            .is_some_and(crate::analyser::ResolvedAnalysisInput::has_logical_source_name_context);
        let applicable = self.blocks.get(&block).and_then(|body| body.statements.get(index)).is_some_and(|statement| {
            matches!(&statement.statement, Statement::Call { tokens, foreach_groups: None, .. } if tokens.as_ref().is_none_or(|tokens| tokens.synthetic.is_none()))
        });
        logical
            && applicable
            && self
                .current_source_may_call(block, index)
                .is_none_or(|effect| {
                    effect
                        .original
                        .as_ref()
                        .is_none_or(|original| !original.writes().is_empty())
                })
    }

    fn seal_source_may_clobbers(&mut self) {
        if self.source_may_effects.is_empty() {
            return;
        }
        let locations = std::sync::Arc::new(
            self.cell_symbols()
                .map(|(symbol, key)| (symbol, key.clone()))
                .collect::<Vec<_>>(),
        );
        for (&(block, index), effect) in &mut self.source_may_effects {
            let Some(body) = self.blocks.get(&block) else {
                continue;
            };
            let Some((marker_index, marker)) = body
                .statements
                .iter()
                .enumerate()
                .skip(index + 1)
                .take_while(|(_, statement)| statement.statement.synthetic_marker().is_some())
                .find(|(_, statement)| is_unseen_call_marker(&statement.statement))
            else {
                continue;
            };
            let Some(versions) = self
                .value_clobbers
                .get(&block)
                .and_then(|markers| markers.get(&marker_index))
                .filter(|versions| !versions.is_empty())
            else {
                continue;
            };
            effect.clobber = Some(SourceMayClobber {
                marker_index,
                marker: marker.statement.clone(),
                versions: versions.clone(),
                locations: std::sync::Arc::clone(&locations),
            });
        }
    }

    /// Reuse only the marker and location clobbers emitted by this SSA producer.
    /// Missing correspondence supplies no closed effect coverage.
    pub(crate) fn source_may_clobber_is_current_at(&self, block: BlockId, index: usize) -> bool {
        let Some(effect) = self.source_may_effects.get(&(block, index)) else {
            return false;
        };
        let Some(held) = effect.clobber.as_ref() else {
            return false;
        };
        let Some(body) = self.blocks.get(&block) else {
            return false;
        };
        body.statements
            .get(index)
            .is_some_and(|statement| statement.statement == effect.statement)
            && body
                .statements
                .get(held.marker_index)
                .is_some_and(|statement| statement.statement == held.marker)
            && body
                .statements
                .get(index + 1..held.marker_index)
                .is_some_and(|statements| {
                    statements
                        .iter()
                        .all(|statement| statement.statement.synthetic_marker().is_some())
                })
            && self
                .value_clobbers
                .get(&block)
                .and_then(|markers| markers.get(&held.marker_index))
                == Some(&held.versions)
            && self
                .cell_symbols()
                .eq(held.locations.iter().map(|(symbol, key)| (*symbol, key)))
    }

    /// Correspondence to the graph held by this SSA owner. A current external
    /// FunctionUnit/CFG still requires its independent owner checks.
    pub(crate) fn source_variable_snapshot(
        &self,
        symbol: Symbol,
    ) -> Option<SourceVariableSsaSnapshot> {
        let identity = self.source_variable_symbol(symbol)?.clone();
        let mut definitions = Vec::new();
        for (&(block, index), effect) in &self.source_may_effects {
            for &(defined, version) in &effect.definitions {
                if defined == symbol {
                    let current = self.current_source_may_call(block, index)?;
                    definitions.push((block, index, version, current.original.as_ref()?.clone()));
                }
            }
        }
        if definitions.is_empty() {
            return None;
        }
        definitions.sort_by_key(|(block, index, version, _)| (*block, *index, *version));
        let mut graph = Vec::new();
        for (&block, body) in &self.blocks {
            let phis: Vec<_> = body
                .phis
                .iter()
                .filter(|phi| phi.name == symbol)
                .cloned()
                .collect();
            let statements: Vec<_> = body
                .statements
                .iter()
                .enumerate()
                .filter_map(|(index, statement)| {
                    let used = statement.uses.get(&symbol).copied();
                    let defined = statement.defs.get(&symbol).copied();
                    (used.is_some() || defined.is_some()).then(|| SourceVariableStatementSnapshot {
                        index,
                        statement: statement.statement.clone(),
                        used,
                        defined,
                        may: statement.may_defs.contains(&symbol),
                        reads: SourceVariableReadFlags {
                            quoted: statement.quoted_uses.contains(&symbol),
                            name_only: statement.name_only_uses.contains(&symbol),
                        },
                        destroyed: statement.destruction_defs.contains(&symbol),
                    })
                })
                .collect();
            let entry = body.entry_versions.get(&symbol).copied();
            let exit = body.exit_versions.get(&symbol).copied();
            if !phis.is_empty() || !statements.is_empty() || entry.is_some() || exit.is_some() {
                graph.push(SourceVariableBlockSnapshot {
                    block,
                    phis,
                    entry,
                    exit,
                    statements,
                });
            }
        }
        graph.sort_by_key(|block| block.block);
        Some(SourceVariableSsaSnapshot {
            identity,
            definitions,
            graph,
            topology: self.source_topology.clone(),
            entry: self.entry,
            idom: self.idom.clone(),
            dominance_frontier: self.dominance_frontier.clone(),
            dominator_tree: self.dominator_tree.clone(),
        })
    }

    /// Dense location labels, in [`Self::cell_keys`] inventory order, never identity.
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
                    *display == name
                        && self
                            .cell_key(Symbol(
                                u32::try_from(*index).expect("SSA variable count fits in u32"),
                            ))
                            .is_some_and(|key| key.authored_spelling().is_some())
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

    /// Every symbol in this function's actual interner, independent of identity domain.
    pub fn variable_symbols(&self) -> impl Iterator<Item = Symbol> {
        (0..self.var_names.len())
            .map(|index| Symbol(u32::try_from(index).expect("SSA variable count fits in u32")))
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
        let tail = crate::naming::written_command_tail(command.as_bytes());
        return crate::naming::is_qualified(command.as_bytes())
            && (tail == b"for" || tail == b"map");
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

pub use tcl_syntax::naming::unattributed_dynamic_written_variable_target as is_dynamic_write_target;

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
                        && !tcl_syntax::naming::is_absolute(name)
                    {
                        tcl_syntax::naming::root_unrooted_key(name)
                    } else {
                        name.to_owned()
                    }
                })
            }),
    );
    defs
}

fn selected_barrier_defs(
    selection: SsaInvocationContext<'_>,
    tokens: &CommandTokens,
) -> Vec<String> {
    if let Some(source) = selection.source_structure(tokens) {
        let Some(traits) = source.traits() else {
            return Vec::new();
        };
        if traits.contains(tcl_registry::Traits::CREATES_SCOPE_ALIAS)
            || source.loop_bindings_conditional()
        {
            return Vec::new();
        }
        let Some(config) = tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.original_lexer_config_for_tokens(tokens))
        else {
            return Vec::new();
        };
        return source
            .written_roles()
            .into_iter()
            .filter(|(_, role)| *role == tcl_registry::ArgRole::LoopVarList)
            .filter_map(|(argument, _)| source.argument_literal(argument))
            .filter_map(|text| {
                tcl_syntax::word_rules::WordValueRules::from_config(&config)
                    .split_list(&text)
                    .ok()
                    .map(|names| {
                        names
                            .into_iter()
                            .map(std::borrow::Cow::into_owned)
                            .collect::<Vec<_>>()
                    })
            })
            .flatten()
            .collect();
    }
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
                defs.extend(names.into_iter().map(std::borrow::Cow::into_owned));
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

/// Definition projection under the caller's retained complete source input.
/// Supplied absence never reopens standalone descriptor selection; original
/// typed structural writes and Normal-transfer checks retain their own purpose.
pub(crate) fn defs_of_with_metadata_context(
    stmt: &Statement,
    registry: &CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Vec<String> {
    defs_of_in_context(
        stmt,
        Some(SsaInvocationContext::supplied(registry, metadata)),
    )
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
        Statement::Call { .. } if is_effect_marker(stmt) => Vec::new(),
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
                .map_or_default(|names| {
                    names
                        .into_iter()
                        .map(std::borrow::Cow::into_owned)
                        .collect()
                });
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
                        .and_then(crate::registry_invocation::InvocationMetadataContext::source_analysis_input),
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

/// The names an opaque `switch`'s arm bodies may write, destroy or declare
/// beyond the ones every path through it defines
/// ([`defs_of_with_registry`]).
///
/// A `-glob`, `-regexp`, `-nocase`, fall-through or `case` statement keeps its
/// arm bodies inline, so no block of the CFG holds a write inside one, and
/// this walk is the only place that says `q* { set go 0 }` may leave `go` at
/// 0 where the statement began with 1. A name written anywhere in an arm —
/// however deeply nested, in a command's own words or in a `[…]` substitution
/// one of them carries, and whichever command writes it (`incr`, `append`,
/// `lappend`, `lset`, `dict set`, `array set`, `unset`, a `regexp`'s match
/// variable) — is a *may*-def: the statement leaves it as it was when no arm
/// runs, so the rename walk gives it a version that reads its
/// prior one, the solver joins the two, and existence is unknown afterwards.
/// The statement the CFG builder puts after such a `switch` for the names a
/// callee its arms call writes ([`crate::ir::SyntheticMarker::ArmWrites`])
/// states them as its own definitions, and they are may-definitions too, as
/// are the callee-summary names merged onto a call under supplied metadata.
/// Every other statement kind returns nothing.
pub(crate) fn switch_may_defs(stmt: &Statement, registry: &CommandRegistry) -> Vec<String> {
    switch_may_defs_in_context(stmt, SsaInvocationContext::standalone(registry))
}

/// Named possible arm writes under the same retained selection as SSA itself.
/// This is not a closed inventory of unknown writes or an executed store.
fn switch_may_defs_in_context(
    stmt: &Statement,
    selection: SsaInvocationContext<'_>,
) -> Vec<String> {
    if let Statement::Call { defs, .. } = stmt
        && is_effect_marker(stmt)
    {
        return defs
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
    }
    // Under supplied metadata a call's definite definitions come from its
    // resolved invocation, so the callee-summary names the CFG builder merged
    // onto it (`upvar 1 g x; set x 9`, `global g`) are possible writes the
    // statement leaves as they were when the callee does not reach them.
    if let Statement::Call { defs, .. } = stmt
        && !selection.standalone
        && !defs.is_empty()
    {
        let definite = defs_of_in_context(stmt, Some(selection));
        return defs
            .iter()
            .filter(|name| !definite.contains(name))
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
    }
    let Statement::Switch {
        arms, default_body, ..
    } = stmt
    else {
        return Vec::new();
    };
    let mut written = BTreeSet::new();
    for body in arms
        .iter()
        .filter_map(|arm| arm.body.as_ref())
        .chain(default_body.as_ref())
    {
        nested_writes_in_context(body, selection, &mut written);
    }
    for name in defs_of_in_context(stmt, Some(selection)) {
        written.remove(&name);
    }
    written.into_iter().collect()
}

/// The bases a statement with arm may-definitions refreshes only because it
/// may write one of their elements: the array `set a(k) …` names in an
/// opaque `switch` arm or `catch` body, where the statement states no write
/// of the base itself ([`switch_may_defs`]). The rename walk gives each a
/// quoted read of its prior version, for liveness and the solver's join;
/// read-before-set reads such a base as the definition it is, never as a
/// name the statement may leave unset.
pub(crate) fn refreshed_bases(
    stmt: &Statement,
    function: &crate::compilation_unit::FunctionUnit,
    registry: &CommandRegistry,
) -> Vec<String> {
    if function.source_metadata_input().is_none() && function.cfg.metadata_context.is_standalone() {
        return bases_refreshed_by(&switch_may_defs(stmt, registry));
    }
    bases_refreshed_by(&switch_may_defs_in_context(
        stmt,
        SsaInvocationContext::supplied(registry, function.invocation_metadata_context(registry)),
    ))
}

/// The bases of the elements of `stated` that `stated` does not name itself.
fn bases_refreshed_by(stated: &[String]) -> Vec<String> {
    let mut bases: Vec<String> = stated
        .iter()
        .filter_map(|name| crate::naming::split_element_ref(name).map(|(root, _)| root.to_owned()))
        .filter(|base| !stated.contains(base))
        .collect();
    bases.sort();
    bases.dedup();
    bases
}

/// Every name any statement of `script`, however deeply nested, or any `[…]`
/// substitution in one of its words, may write.
pub(crate) fn nested_writes(
    script: &crate::ir::Script,
    registry: &CommandRegistry,
    out: &mut BTreeSet<String>,
) -> bool {
    nested_writes_in_context(script, SsaInvocationContext::standalone(registry), out)
}

fn nested_writes_in_context(
    script: &crate::ir::Script,
    selection: SsaInvocationContext<'_>,
    out: &mut BTreeSet<String>,
) -> bool {
    let registry = selection.registry;
    let mut opaque = false;
    let mut pending = vec![(script, 0_u32)];
    while let Some((script, depth)) = pending.pop() {
        if MAX_COLLAPSED_EXTRA_DEFS_DEPTH.exceeded(depth) {
            opaque = true;
            continue;
        }
        for statement in &script.statements {
            let structural = crate::ir_helpers::structural_variable_write_projection(statement);
            opaque |= structural.opaque_variable_frame;
            out.extend(structural.literal_names);
            if let Statement::Call { defs, .. } = statement {
                // Existing callee-summary names remain possible writes, not completed stores.
                out.extend(defs.iter().cloned());
            }
            if let Some(tokens) = script.retained_source_tokens_for_statement(statement) {
                let source_purpose = selection.token_metadata(tokens).and_then(|metadata| {
                    tokens
                        .source_binding
                        .as_ref()?
                        .original_source_advice_purpose(tokens, metadata, registry)
                });
                if !selection.standalone && source_purpose.is_none() {
                    opaque = true;
                } else {
                    let names = crate::ir_helpers::original_statement_possible_name_writes(
                        statement, tokens, registry,
                    );
                    opaque |= names.opaque;
                    out.extend(names.names);
                }
            } else if statement.is_executable_invocation() && statement.synthetic_marker().is_none()
            {
                opaque |= !selection.standalone
                    || !crate::ir_helpers::standalone_typed_statement_writes_are_covered(
                        script, statement, registry,
                    );
            }
            for body in statement.child_scripts() {
                pending.push((body, depth + 1));
            }
        }
    }
    opaque
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
            let bn_preds: Vec<BlockId> = preds.get(id).map_or_default(|p| {
                p.iter()
                    .copied()
                    .filter(|p| reachable.contains(p))
                    .collect()
            });

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
        let bn_preds: Vec<BlockId> = preds.get(id).map_or_default(|p| {
            p.iter()
                .copied()
                .filter(|p| reachable.contains(p))
                .collect()
        });

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
    .into_iter()
    .map(|(block, identities)| {
        (
            block,
            identities
                .into_iter()
                .filter_map(|identity| match identity {
                    SsaVariableIdentity::Location(key) => Some(key),
                    SsaVariableIdentity::Source(_) => None,
                })
                .collect(),
        )
    })
    .collect()
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
) -> HashMap<BlockId, HashSet<SsaVariableIdentity>> {
    let reachable = func.reachable_blocks();
    let (nonlocal_names, mut all_defsites) =
        nonlocal_names_and_defsites(func, &reachable, selection, elems, config, points);
    for (block, markers) in clobbers {
        for name in markers.values().flatten() {
            all_defsites
                .or_default(SsaVariableIdentity::Location(name.clone()))
                .insert(*block);
        }
    }

    // Semi-pruned SSA (Briggs et al. 1998): place phis only for *non-local*
    // (upward-exposed-use) names. A phi for a purely-local name has no reader,
    // so dropping it removes only dead phis (~40% of minimal-SSA phis) without
    // changing any use/value/liveness/diagnostic result.
    let mut phi: HashMap<BlockId, HashSet<SsaVariableIdentity>> = HashMap::new();
    for id in func.blocks.keys() {
        phi.insert(*id, HashSet::new());
    }

    for (var, sites) in all_defsites.iter() {
        if !nonlocal_names.contains_key(var) {
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

type AuthoredScalarSourceAccess = (
    crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
    String,
    crate::registry_invocation::AuthoredSourceVariableAccess,
);

fn authored_scalar_source_variable(
    points: &PointResolveContexts,
    block: BlockId,
    index: usize,
    registry: &CommandRegistry,
) -> Option<(
    crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
    String,
    crate::registry_invocation::AuthoredSourceVariableAccess,
)> {
    let tokens = points.source_tokens_at(block, index)?;
    let site = tokens.source_binding.as_ref()?.invocation_site()?;
    let offset = tokens.words().first()?.source().span.start();
    let mut original = tokens.clone();
    crate::lattice_rebase::rebase_command_tokens(
        &mut original,
        i64::from(site.offset) - i64::from(offset),
    );
    let invocation =
        crate::registry_invocation::original_authored_scalar_invocation(&original, registry);
    let scalar = invocation.map(|invocation| invocation.scalar_source_variable(registry));
    scalar
        .flatten()
        .map(|(symbol, name, definition)| {
            let access = if definition {
                crate::registry_invocation::AuthoredSourceVariableAccess::Write
            } else {
                crate::registry_invocation::AuthoredSourceVariableAccess::Read
            };
            (symbol, name, access)
        })
        .or_else(|| {
            crate::registry_invocation::original_authored_script_scalar_variable(
                &original, registry,
            )
        })
}

fn record_authored_terminator_use(
    points: &PointResolveContexts,
    bn: BlockId,
    registry: &CommandRegistry,
    defined_here: &SsaIdentitySet,
    nonlocal_names: &mut SsaIdentitySet,
) {
    if let Some((symbol, _, crate::registry_invocation::AuthoredSourceVariableAccess::Read)) =
        authored_scalar_source_variable(points, bn, usize::MAX, registry)
    {
        let identity = SsaVariableIdentity::Source(symbol);
        if !defined_here.contains_key(&identity) {
            nonlocal_names.or_default(identity);
        }
    }
}

fn nonlocal_terminator_uses(
    (bn, block): (BlockId, &cfg::Block),
    selection: SsaInvocationContext<'_>,
    scanner: &mut VarReferenceScanner,
    points: &PointResolveContexts,
) -> Vec<VariableCellKey> {
    let registry = selection.registry;
    // Terminator reads are element-qualified like statement reads (a
    // condition's `$a(k)` must place the element's phi), and a base
    // read fans over known elements.
    let mut term_uses: Vec<String> = Vec::new();
    match &block.terminator {
        Some(cfg::Terminator::Branch { condition, .. }) => {
            term_uses.extend(condition.vars_element_qualified_with_config(scanner.lexer_config()));
        }
        Some(cfg::Terminator::Return { value, expr, .. }) => {
            if let Some(v) = value {
                term_uses.extend(selection.word_reads(scanner, v));
            }
            if let Some(e) = expr {
                term_uses.extend(e.vars_element_qualified_with_config(scanner.lexer_config()));
            }
        }
        _ => {}
    }
    let mut term_uses = bound_names(term_uses, points.before_terminator(bn), registry);
    term_uses.extend(
        source_read_bindings(points, bn, usize::MAX, registry)
            .into_iter()
            .map(|(_, bound)| bound),
    );
    term_uses.extend(invocation_read_bindings(
        points,
        bn,
        usize::MAX,
        points.before_terminator(bn),
        registry,
    ));
    term_uses
}

fn nonlocal_names_and_defsites(
    func: &cfg::Function,
    reachable: &HashSet<BlockId>,
    selection: SsaInvocationContext<'_>,
    elems: &ArrayElems,
    config: tcl_lexer::LexerConfig,
    points: &PointResolveContexts,
) -> (SsaIdentitySet, SsaDefinitionSites) {
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
    let mut nonlocal_names = SsaIdentitySet::default();
    let mut defsites = SsaDefinitionSites::default();

    for bn in reachable {
        let Some(block) = func.blocks.get(bn) else {
            continue;
        };
        let mut defined_here = SsaIdentitySet::default();
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
            let may_defs = bound_names(
                switch_may_defs_in_context(stmt, selection),
                context,
                registry,
            );
            let all_defs: Vec<_> = direct_defs.iter().chain(&may_defs).cloned().collect();
            for u in direct_uses
                .iter()
                .cloned()
                .chain(expand_uses(&direct_uses, &all_defs, elems))
                .chain(may_defs.iter().cloned())
            {
                let u = SsaVariableIdentity::Location(u);
                if !defined_here.contains_key(&u) {
                    nonlocal_names.or_default(u);
                }
            }
            for var in expand_defs(&all_defs, elems) {
                let var = SsaVariableIdentity::Location(var);
                defsites.or_default(var.clone()).insert(*bn);
                defined_here.or_default(var);
            }
            if let Some((symbol, _, definition)) =
                authored_scalar_source_variable(points, *bn, index, registry)
            {
                let identity = SsaVariableIdentity::Source(symbol);
                if definition.is_definition() {
                    if definition.is_may_write() {
                        nonlocal_names.or_default(identity.clone());
                    }
                    defsites.or_default(identity.clone()).insert(*bn);
                    defined_here.or_default(identity);
                } else if !defined_here.contains_key(&identity) {
                    nonlocal_names.or_default(identity);
                }
            }
            if let Some(receipt) =
                original_source_may_call(func, stmt, selection.metadata, registry)
            {
                for place in receipt.writes() {
                    let identity = SsaVariableIdentity::Source(place.symbol().clone());
                    // Source effects may be observed without a successful
                    // lexical read; retain their joins conservatively.
                    nonlocal_names.or_default(identity.clone());
                    defsites.or_default(identity.clone()).insert(*bn);
                    defined_here.or_default(identity);
                }
            }
        }
        record_authored_terminator_use(points, *bn, registry, &defined_here, &mut nonlocal_names);
        let term_uses = nonlocal_terminator_uses((*bn, block), selection, &mut scanner, points);
        for u in term_uses
            .iter()
            .cloned()
            .chain(expand_uses(&term_uses, &[], elems))
        {
            let u = SsaVariableIdentity::Location(u);
            if !defined_here.contains_key(&u) {
                nonlocal_names.or_default(u);
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
                    for name in selection.word_reads(scanner, text) {
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
                sink.extend(selection.word_reads(scanner, v));
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
            for v in selection.word_reads(scanner, subject) {
                reads_own_def.insert(v);
            }
        }

        // Other structured IR statements (If, For, While, …) are flattened by
        // the CFG builder before SSA construction, so they never reach here.
        _ => {}
    }

    finish_classified_uses(found, &reads_own_def, stmt, selection)
}

/// Consumed typed statements retain their original roles at the exact source point.
fn uses_of_classified_with_source(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
    tokens: Option<&CommandTokens>,
) -> Vec<(String, UseClass)> {
    let mut reads = uses_of_classified_in_context(stmt, scanner, selection);
    if !matches!(
        stmt,
        Statement::AssignConst { .. }
            | Statement::AssignExpr { .. }
            | Statement::AssignValue { .. }
            | Statement::Incr { .. }
    ) {
        return reads;
    }
    let mut selected = ClassifiedUses::default();
    selected_handler_name_reads(tokens, &[], selection, &mut selected, &mut BTreeSet::new());
    for name in selected.by_name {
        if let Some((_, class)) = reads.iter_mut().find(|(existing, _)| *existing == name) {
            if *class == UseClass::Quoted {
                *class = UseClass::Name;
            }
        } else {
            reads.push((name, UseClass::Name));
        }
    }
    reads
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
        .filter(|(v, _)| !defs.contains(v) || reads_own_def.contains(v))
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
    if tokens
        .as_ref()
        .is_some_and(|tokens| selection.source_structure(tokens).is_some())
    {
        selected_handler_name_reads(tokens.as_ref(), &[], selection, found, reads_own_def);
        return;
    }
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
            let name = tcl_syntax::naming::normalise_var_name_braced(name, true).to_owned();
            found.by_name.insert(name.clone());
            reads_own_def.insert(name);
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
            .map(|name| crate::naming::normalise_var_name(name).to_owned())
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
    selection: SsaInvocationContext<'_>,
) -> BTreeSet<String> {
    let registry = selection.registry;
    // naming.variable.selected-setter-expression-reads
    // docs/design/analysis/name-resolution-proofs/variable-selected-setter-expression-reads.md
    // Queried commands already use original entered/conditional child owners
    // in scan_nested_substitution_words. A written head cannot add another
    // evaluator after that lookup has selected a different or unknown handler.
    if tokens.is_some_and(|tokens| tokens.source_binding.is_some()) {
        return scanner.scan_lexical_word(value, registry);
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
    selection.word_reads(scanner, value)
}

/// Named reads from the independently selected Normal or conditional Logical
/// role owner. Literal operand data keeps empty names and dollar sigils.
fn selected_handler_name_reads(
    tokens: Option<&CommandTokens>,
    defs: &[String],
    selection: SsaInvocationContext<'_>,
    found: &mut ClassifiedUses,
    reads_own_def: &mut BTreeSet<String>,
) {
    if let Some(source) = tokens.and_then(|tokens| selection.source_structure(tokens)) {
        let Some(traits) = source.traits() else {
            return;
        };
        for (argument, role) in source.written_roles() {
            if role != tcl_registry::ArgRole::VarRead
                && !(role == tcl_registry::ArgRole::VarWrite
                    && traits.contains(tcl_registry::Traits::READS_BEFORE_WRITE))
            {
                continue;
            }
            if let Some(name) = source.argument_literal(argument) {
                let name = tcl_syntax::naming::normalise_var_name_braced(&name, true).to_owned();
                found.by_name.insert(name.clone());
                if defs.contains(&name) {
                    reads_own_def.insert(name);
                }
            }
        }
        return;
    }
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
                let name = tcl_syntax::naming::normalise_var_name_braced(&name, true).to_owned();
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
        for v in set_value_reads(value, tokens.as_ref(), scanner, selection) {
            if defs.iter().any(|d| d.as_str() == v) {
                reads_own_def.insert(v.clone());
            }
            vars_found.insert(v);
        }
    }
    let retained_roles = selection.standalone
        || tokens.as_ref().is_some_and(|tokens| {
            tokens.carries_proved_names() || selection.normal(tokens).is_some()
        });
    // `reads` are the registry's `ArgRole::VarRead` positions and
    // `reads_own_defs` its `READS_BEFORE_WRITE` targets: both name the cell
    // rather than substituting it, so they are `UseClass::Name` — a real read
    // with no operand word behind it.
    for name in reads.iter().filter(|_| retained_roles) {
        found.by_name.insert(name.clone());
        // A name this statement both reads and defines is read *before* it is
        // written — the same rule `uses_in_barrier` applies to a `dict with`
        // scope alias. Without it the closing def-filter in
        // `uses_of_classified` drops the read, and the store feeding
        // `puts [incr n]` looks overwritten-before-read (#2050).
        if defs.contains(name) {
            reads_own_def.insert(name.clone());
            // …and the call's embedded commands write it: its words read it
            // before, between and after that write, and which version a word
            // names cannot be told, so no word of it is an operand to
            // forward. The read stays one by name, of the version before the
            // write.
            found.substituted.remove(name);
        }
    }
    if *reads_own_defs && retained_roles {
        for name in defs {
            found.by_name.insert(name.clone());
            reads_own_def.insert(name.clone());
        }
    }
    selected_handler_name_reads(tokens.as_ref(), defs, selection, found, reads_own_def);
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
    // def stays, and the prior version becomes a read. A def whose evaluated
    // outcome preserves its place holds that prior version
    // (`SccpResult::preserved`), which is how W210 sees a no-match read.
    // The same commands nested in a word or a condition take the same read
    // from `ir_helpers::variable_write_effects_from_commands`.
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
    // A brace-quoted target's `$` is part of a literal name, not a
    // substitution: it is neither a dynamic target nor a read of the
    // `$`-less lookalike.
    let note_reads_own = |name: &str,
                          braced: bool,
                          scanner: &VarReferenceScanner,
                          vars_found: &BTreeSet<String>,
                          rod: &mut BTreeSet<String>| {
        let norm = scanner.canonical_name_braced(name, braced);
        if vars_found.contains(norm) {
            rod.insert(norm.to_owned());
        }
    };
    match stmt {
        Statement::AssignConst {
            name, name_braced, ..
        } => {
            if is_dynamic_write_target(name, *name_braced) {
                vars_found.extend(selection.word_reads(scanner, name));
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
                vars_found.extend(selection.word_reads(scanner, name));
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
            vars_found.extend(selection.word_reads(scanner, value));
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
                vars_found.extend(selection.word_reads(scanner, name));
            } else {
                note_reads_own(name, *name_braced, scanner, vars_found, reads_own_def);
            }
        }
        Statement::Incr {
            name,
            name_braced,
            amount,
            amount_braced,
            ..
        } => {
            if is_dynamic_write_target(name, *name_braced) {
                vars_found.extend(selection.word_reads(scanner, name));
            } else {
                // `incr a` names the cell it mutates; there is no `$a` word.
                let norm = scanner.canonical_name_braced(name, *name_braced);
                by_name.insert(norm.to_owned());
                reads_own_def.insert(norm.to_owned());
            }
            // A braced amount substitutes nothing: `incr x {$n}` reads no
            // `n` (it raises `expected integer but got "$n"`).
            if let Some(amt) = amount.as_ref().filter(|_| !*amount_braced) {
                vars_found.extend(selection.word_reads(scanner, amt));
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
    out.substituted
        .extend(selection.word_reads(scanner, command));
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
    if let Some(source) = tokens.and_then(|tokens| selection.source_structure(tokens)) {
        for (written, role) in source.written_roles() {
            if matches!(
                role,
                tcl_registry::ArgRole::VarWrite | tcl_registry::ArgRole::VarRead
            ) && tokens.is_some_and(|tokens| tokens.arg_is_braced_literal(written))
            {
                name_role_braced.insert(written);
            }
        }
    }
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
        for name in selection.word_reads(scanner, arg) {
            let class = braced_word_class(&BracedWordSite {
                braced,
                evaluated_in_frame: in_frame_braced.contains(&idx),
                described,
                word: arg,
                name: &name,
                selection,
                tokens,
                written_argument: idx,
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
                        by_name.insert(
                            tcl_syntax::naming::normalise_var_name_braced(&name, true).to_owned(),
                        );
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
            substituted.extend(selection.word_reads(scanner, braced));
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
    let Some(config) = binding.original_lexer_config_for_tokens(tokens) else {
        return;
    };
    if let Some(evaluation) = binding.conditional_expression_evaluation(registry, tokens) {
        let config = config.with_grammar(evaluation.lexer_grammar());
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
    let config = config.with_grammar(parser.lexer_grammar);
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
    /// Actual availability and the explicit standalone utility boundary.
    selection: SsaInvocationContext<'a>,
    /// Original whole invocation, including the body word's written ordinal.
    tokens: Option<&'a CommandTokens>,
    written_argument: usize,
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
    if site.described || braced_word_owns_name(site) {
        return UseClass::Quoted;
    }
    UseClass::Substituted
}

/// An ownership exclusion consumes a complete original body query. MAY names
/// in an opaque residual cannot prove that a read belongs to that body.
fn braced_word_owns_name(site: &BracedWordSite<'_>) -> bool {
    if site.selection.standalone {
        return crate::script_binds::script_binds_name(
            site.word,
            site.name,
            crate::script_binds::Ownership::Bindings,
            site.selection.registry,
            site.config,
        );
    }
    let Some(tokens) = site.tokens else {
        return false;
    };
    let Some(ownership) = crate::script_binds::original_literal_body_ownership(
        site.selection.registry,
        site.selection.token_metadata(tokens),
        tokens,
        site.written_argument,
        crate::script_binds::Ownership::Bindings,
    ) else {
        return false;
    };
    let root = tcl_syntax::naming::split_element_ref(site.name).map_or(site.name, |(root, _)| root);
    !ownership.opaque && ownership.names.iter().any(|name| name == root)
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
    let mut reads = ClassifiedUses {
        substituted: selection.word_reads(scanner, subject),
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
        pattern_sink.extend(selection.word_reads(scanner, &arm.pattern));
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
                sink.extend(selection.word_reads(scanner, &it.list_arg));
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

/// Explicit standalone condition/init/next write candidates for collapsed reads.
/// Source-owned callers use their complete retained statement inventories.
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
    pushed_vars: Vec<SsaVariableIdentity>,
    phase: RenamePhase,
}

/// Phase within a single rename frame.
enum RenamePhase {
    /// Process phi nodes and statements for this block.
    Enter,
    /// Iterate over dominator-tree children.
    ProcessChildren,
}

/// Identity-keyed per-block state produced by the rename walk and consumed when
/// assembling the final SSA blocks. Bundled so the walk and the assembly step
/// share one value instead of threading five parallel maps.
#[derive(Default)]
struct RenameOutputs {
    phi_versions: HashMap<BlockId, HashMap<SsaVariableIdentity, Version>>,
    phi_incoming: HashMap<BlockId, HashMap<SsaVariableIdentity, HashMap<BlockId, Version>>>,
    entry_versions: HashMap<BlockId, SsaVisibleVersions>,
    exit_versions: HashMap<BlockId, SsaVisibleVersions>,
    stmt_infos: HashMap<BlockId, Vec<SsaStatement>>,
    source_may_effects: HashMap<(BlockId, usize), SourceMaySsaCall>,
    /// The versions live at each call to a command the module cannot see.
    unseen_call_versions: Vec<(SsaVariableIdentity, Version)>,
}

/// Retain linked-input definitions from a registry-selected instance class.
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

pub(crate) fn linked_option_definition_name(
    tokens: &CommandTokens,
    index: usize,
    global: bool,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    registry: &CommandRegistry,
) -> Option<String> {
    let binding = tokens.source_binding.as_ref()?;
    let purpose = binding.original_source_advice_purpose(tokens, metadata, registry)?;
    let input = binding.original_written_name_input(tokens, index.checked_add(1)?)?;
    let key = input.original_word_key()?;
    // Empty linked-input options disconnect; dollar signs in static values are data.
    let value = key.bytes();
    if value.is_empty() {
        return None;
    }
    let variable = (purpose != crate::command_binding::OriginalInvocationSourcePurpose::Logical)
        .then(|| key.policy().recipe().combined_variable_input(value));
    // Logical words retain Unicode values; they are not a Native string pool.
    let root = variable.as_ref().map_or_else(
        || tcl_syntax::naming::split_element_ref_bytes(value).map_or(value, |(root, _)| root),
        |variable| variable.root().selected(),
    );
    let name = std::str::from_utf8(root).ok()?;
    Some(
        if global && tcl_syntax::naming::unroot_rooted_key(name).is_none() {
            tcl_syntax::naming::root_unrooted_key(name)
        } else {
            name.to_owned()
        },
    )
}

/// Selected source option metadata keeps its effective ordinal and genuine
/// written operand separately. This grants no physical definition or store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OriginalLinkedOptionWrite {
    pub(crate) effective_argument: usize,
    pub(crate) written_argument: usize,
    pub(crate) scope: tcl_registry::VariableScope,
}

fn selected_source_option_writes(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<Vec<(usize, tcl_registry::VariableScope)>> {
    schema.authored_source_argument_roles().1.then_some(())?;
    schema.semantics.options.prefix_occurrences(
        schema
            .words
            .arguments()
            .slice_from(schema.semantics.argument_offset),
    )?;
    Some(
        schema
            .authored_source_variable_name_arguments()
            .into_iter()
            .filter(|(_, role)| *role == tcl_registry::ArgRole::VarWrite)
            .filter_map(|(argument, _)| {
                Some((
                    argument,
                    schema.authored_source_option_variable_scope_at(argument)?,
                ))
            })
            .collect(),
    )
}

pub(crate) fn original_command_option_writes(
    tokens: &CommandTokens,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    registry: &CommandRegistry,
) -> Option<Vec<OriginalLinkedOptionWrite>> {
    tokens
        .source_binding
        .as_ref()?
        .original_source_advice_purpose(tokens, metadata, registry)?;
    let advice =
        crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
            registry,
            Some(metadata),
            tokens,
        )?;
    advice.unanimous_command_words()?;
    let mut selected = None;
    for shape in &advice.candidates {
        let writes =
            shape.with_source_schema(registry, metadata, tokens, selected_source_option_writes)?;
        let writes = writes
            .into_iter()
            .filter_map(|(effective_argument, scope)| {
                let crate::registry_invocation::InvocationWordOrigin::Written(written) = shape
                    .effective
                    .origins
                    .get(effective_argument.checked_add(1)?)?
                else {
                    return None;
                };
                Some(OriginalLinkedOptionWrite {
                    effective_argument,
                    written_argument: written.checked_sub(1)?,
                    scope,
                })
            })
            .collect::<Vec<_>>();
        if selected
            .as_ref()
            .is_some_and(|previous| previous != &writes)
        {
            return None;
        }
        selected = Some(writes);
    }
    selected
}

/// Selected command options whose current Registry schema specifically
/// declares both variable writes and tainted external input. Generic SSA
/// variable-write consumers keep using `original_command_option_writes`.
pub(crate) fn original_command_taint_option_writes(
    tokens: &CommandTokens,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    registry: &CommandRegistry,
) -> Option<Vec<OriginalLinkedOptionWrite>> {
    fn selected(
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<Vec<(usize, tcl_registry::VariableScope)>> {
        if !schema
            .semantics
            .traits
            .contains(tcl_registry::Traits::TAINTS_VAR_WRITES)
        {
            return Some(Vec::new());
        }
        let offset = schema.semantics.argument_offset;
        let options = schema
            .semantics
            .options
            .prefix_occurrences(schema.words.arguments().slice_from(offset))?;
        let variable_writes = selected_source_option_writes(schema)?;
        Some(
            variable_writes
                .into_iter()
                .filter(|(argument, _)| {
                    argument.checked_sub(offset).is_some_and(|relative| {
                        options.iter().any(|occurrence| {
                            occurrence.values.contains(&relative)
                                && occurrence
                                    .option
                                    .is_some_and(tcl_registry::hover::OptionSpec::taints_var_write)
                        })
                    })
                })
                .collect(),
        )
    }

    tokens
        .source_binding
        .as_ref()?
        .original_source_advice_purpose(tokens, metadata, registry)?;
    let advice =
        crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
            registry,
            Some(metadata),
            tokens,
        )?;
    advice.unanimous_command_words()?;
    let mut selected_writes = None;
    for shape in &advice.candidates {
        let writes = shape.with_source_schema(registry, metadata, tokens, selected)?;
        let writes = writes
            .into_iter()
            .filter_map(|(effective_argument, scope)| {
                let crate::registry_invocation::InvocationWordOrigin::Written(written) = shape
                    .effective
                    .origins
                    .get(effective_argument.checked_add(1)?)?
                else {
                    return None;
                };
                Some(OriginalLinkedOptionWrite {
                    effective_argument,
                    written_argument: written.checked_sub(1)?,
                    scope,
                })
            })
            .collect::<Vec<_>>();
        if selected_writes
            .as_ref()
            .is_some_and(|previous| previous != &writes)
        {
            return None;
        }
        selected_writes = Some(writes);
    }
    selected_writes
}

pub(crate) fn original_instance_option_writes(
    tokens: &CommandTokens,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    registry: &CommandRegistry,
) -> Option<Vec<OriginalLinkedOptionWrite>> {
    let source = tokens
        .source_binding
        .as_ref()?
        .original_registered_instance_source_for_tokens(tokens, metadata, registry)?;
    let context = metadata
        .source_analysis_input()?
        .borrowed_context_registry();
    source
        .instance()
        .descriptor(context)?
        .traits
        .contains(tcl_registry::Traits::TAINTS_VAR_WRITES)
        .then_some(())?;
    let writes = source.with_source_schema(context, |schema| {
        schema
            .semantics
            .traits
            .contains(tcl_registry::Traits::CONFIGURES_INSTANCE_OPTIONS)
            .then_some(())?;
        let offset = schema.semantics.argument_offset;
        let options = schema
            .semantics
            .options
            .prefix_occurrences(schema.words.arguments().slice_from(offset))?;
        Some(
            selected_source_option_writes(schema)?
                .into_iter()
                .filter(|(argument, _)| {
                    argument.checked_sub(offset).is_some_and(|relative| {
                        options.iter().any(|occurrence| {
                            occurrence.values.contains(&relative)
                                && occurrence
                                    .option
                                    .is_some_and(tcl_registry::hover::OptionSpec::taints_var_write)
                        })
                    })
                })
                .collect::<Vec<_>>(),
        )
    })??;
    let mut selected = Vec::new();
    for (effective_argument, scope) in writes {
        let word = source.argument_word(effective_argument)?;
        if word.group().expand {
            return None;
        }
        // A captured prefix keeps its earlier source word. Only the actual
        // current complete vector can supply this call's written operand.
        let mut written = source
            .original_words()
            .iter()
            .enumerate()
            .filter_map(|(written, original)| (original == word).then_some(written));
        let Some(index) = written.next() else {
            continue;
        };
        if written.next().is_some() {
            return None;
        }
        selected.push(OriginalLinkedOptionWrite {
            effective_argument,
            written_argument: index.checked_sub(1)?,
            scope,
        });
    }
    Some(selected)
}

fn original_global_root_frame(
    tokens: &CommandTokens,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    registry: &CommandRegistry,
) -> Option<()> {
    let binding = tokens.source_binding.as_ref()?;
    binding.original_source_advice_purpose(tokens, metadata, registry)?;
    if binding.variable_frame.layout() != &crate::var_resolve::VariableExecutionFrame::Global {
        return None;
    }
    let namespace = binding.variable_context.namespace_identity.as_ref()?;
    namespace.exact_native_path()?.is_root().then_some(())
}

// Bare aliases require the original global frame and root namespace, not a body label.
fn mirror_top_level_global_option_defs(
    func: &mut cfg::Function,
    registry: &CommandRegistry,
) -> bool {
    let Some(crate::registry_invocation::InvocationMetadataSelection::Retained(metadata)) =
        func.metadata_context.metadata_context(registry)
    else {
        return false;
    };
    let mut changed = false;
    for block in func.blocks.values_mut() {
        for statement in &mut block.statements {
            let Statement::Call {
                command,
                args,
                defs,
                tokens,
                ..
            } = statement
            else {
                continue;
            };
            let Some(tokens) = tokens.as_ref() else {
                continue;
            };
            if tokens.argv_texts.first() != Some(command)
                || tokens.argv_texts.get(1..) != Some(args.as_slice())
            {
                continue;
            }
            if original_global_root_frame(tokens, metadata, registry).is_none() {
                continue;
            }
            let Some(writes) = original_command_option_writes(tokens, metadata, registry) else {
                continue;
            };
            let aliases: Vec<String> = writes
                .into_iter()
                .filter(|write| write.scope == tcl_registry::VariableScope::Global)
                .filter_map(|write| {
                    let global = linked_option_definition_name(
                        tokens,
                        write.written_argument,
                        true,
                        metadata,
                        registry,
                    )?;
                    let bare = tcl_syntax::naming::unroot_rooted_key(&global)?;
                    (!tcl_syntax::naming::is_qualified(bare.as_bytes()) && defs.contains(&global))
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

/// Retain source option definitions under the genuine selected instance schema.
/// Physical SSA definitions still require the independent Normal projection in
/// `defs_of_in_context`; this source metadata supplies no such permission.
pub(crate) fn enrich_instance_option_defs_with_initial(
    func: &mut cfg::Function,
    registry: &CommandRegistry,
    _initial: &crate::taint::InstanceClassState,
) -> HashSet<String> {
    let Some(crate::registry_invocation::InvocationMetadataSelection::Retained(metadata)) =
        func.metadata_context.metadata_context(registry)
    else {
        return HashSet::new();
    };
    let mut instance_defs = HashSet::new();
    for block in func.blocks.values_mut() {
        for statement in &mut block.statements {
            let Statement::Call {
                command,
                args,
                defs,
                tokens,
                ..
            } = statement
            else {
                continue;
            };
            let Some(tokens) = tokens.as_ref() else {
                continue;
            };
            if tokens.argv_texts.first() != Some(command)
                || tokens.argv_texts.get(1..) != Some(args.as_slice())
            {
                continue;
            }
            let Some(writes) = original_instance_option_writes(tokens, metadata, registry) else {
                continue;
            };
            for write in writes {
                let global = write.scope == tcl_registry::VariableScope::Global;
                let Some(name) = linked_option_definition_name(
                    tokens,
                    write.written_argument,
                    global,
                    metadata,
                    registry,
                ) else {
                    continue;
                };
                instance_defs.insert(name.clone());
                if !defs.contains(&name) {
                    defs.push(name.clone());
                }
                if global
                    && original_global_root_frame(tokens, metadata, registry).is_some()
                    && let Some(bare) = tcl_syntax::naming::unroot_rooted_key(&name)
                    && !tcl_syntax::naming::is_qualified(bare.as_bytes())
                {
                    instance_defs.insert(bare.to_owned());
                    if !defs.iter().any(|defined| defined == bare) {
                        defs.push(bare.to_owned());
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

/// Exact read spelling projection; unavailable original syntax cannot relabel
/// the retained read or suppress an independently scanned protective dependency.
fn source_read_name_with_config(
    access: &crate::command_binding::SourceVariableAccess,
    registry: &CommandRegistry,
) -> Option<String> {
    let config = access.variable_context.read_lexer_config(registry)?;
    Some(
        tcl_syntax::naming::var_reference_for_style(&access.original_spelling, config.braced_var)
            .to_owned(),
    )
}

/// Existing test probe delegates the same actual configured source projection.
#[cfg(test)]
fn source_read_name(access: &crate::command_binding::SourceVariableAccess) -> String {
    source_read_name_with_config(access, &CommandRegistry::build_default())
        .unwrap_or_else(|| access.original_spelling.clone())
}

fn captured_read_names(
    points: &PointResolveContexts,
    block: BlockId,
    index: usize,
    registry: &CommandRegistry,
) -> HashSet<String> {
    points
        .source_reads_at(block, index)
        .iter()
        .filter_map(|access| source_read_name_with_config(access, registry))
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
            // A callee's static storage belongs to the callable, not to any
            // caller frame: the caller cannot reach it, so the callee's reads
            // of it are not caller SSA reads.
            crate::place_bridge::invocation_execution_read_places(tokens, registry)
                .into_iter()
                .filter_map(|place| crate::var_resolve::canonical_binding_value_key(&place))
                .filter(|key| !is_callable_storage(key))
                .chain(
                    crate::place_bridge::invocation_argument_execution_read_places_at(
                        points, block, index, registry,
                    )
                    .into_iter()
                    .filter_map(|place| crate::var_resolve::canonical_binding_value_key(&place)),
                )
        })
        .filter_map(|key| invocation_read_key_in_context(key, context))
        .collect()
}

/// Whether `key` is a callable's own static storage (Jim `proc … {{n 0}}`).
fn is_callable_storage(key: &VariableCellKey) -> bool {
    matches!(
        key,
        VariableCellKey::RetainedSlot(slot)
            if matches!(slot.as_ref(), crate::raw_binding::RawBindingSlotId::Callable { .. })
    )
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
    // A captured read the walk made inside a callee it followed is of the
    // callee's own frame: like an invocation's execution reads, a cell of
    // another activation is no SSA cell of this function.
    let context = if index == usize::MAX {
        points.before_terminator(block)
    } else {
        points.before_statement(block, index)
    };
    points
        .source_reads_at(block, index)
        .iter()
        .filter_map(|access| {
            let name = crate::var_resolve::canonical_binding_value_key(&source_read_place(
                access, registry,
            ))?;
            let name = invocation_read_key_in_context(name, context)?;
            Some((source_read_name_with_config(access, registry)?, name))
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
                    && represented.is_some_and(|represented| source_origin.is_within(represented)))
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

fn original_script_source_points(
    points: &PointResolveContexts,
    blocks: &HashMap<BlockId, SsaBlock>,
    interner: &VarInterner,
    registry: &CommandRegistry,
) -> HashMap<(BlockId, usize), OriginalScriptSourcePoint> {
    let mut rows = HashMap::new();
    for (&(block, index), tokens) in points.source_token_inventory() {
        let Some(binding) = tokens.source_binding.as_ref() else {
            continue;
        };
        if binding
            .original_standalone_script_variable_scope(tokens, registry)
            .is_none()
        {
            continue;
        }
        let Some(site) = binding.invocation_site() else {
            continue;
        };
        let Some(offset) = tokens
            .words()
            .first()
            .map(|word| word.source().span.start())
        else {
            continue;
        };
        let mut original = tokens.clone();
        crate::lattice_rebase::rebase_command_tokens(
            &mut original,
            i64::from(site.offset) - i64::from(offset),
        );
        let Some((identity, _, access)) =
            crate::registry_invocation::original_authored_script_scalar_variable(
                &original, registry,
            )
        else {
            continue;
        };
        let Some(&symbol) = interner.source_to_symbol.get(&identity) else {
            continue;
        };
        let Some(body) = blocks.get(&block) else {
            continue;
        };
        let (used, defined) = if index == usize::MAX {
            (body.exit_versions.get(&symbol).copied(), None)
        } else {
            let Some(statement) = body.statements.get(index) else {
                continue;
            };
            (
                statement.uses.get(&symbol).copied(),
                statement.defs.get(&symbol).copied(),
            )
        };
        rows.insert(
            (block, index),
            OriginalScriptSourcePoint {
                symbol,
                access,
                used,
                defined,
            },
        );
    }
    rows
}

fn retained_source_read_references(
    points: &PointResolveContexts,
    blocks: &HashMap<BlockId, SsaBlock>,
    symbols: &HashMap<(BlockId, usize), HashMap<String, Symbol>>,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    interner: &VarInterner,
) -> HashMap<(BlockId, usize, SourceSite, String), SsaReadReference> {
    let mut references = HashMap::new();
    let mut scanner = VarReferenceScanner::with_config(
        VarScanOptions {
            include_var_read_roles: false,
            recurse_cmd_substitutions: false,
            include_reads_before_write: false,
            element_qualified: true,
        },
        config,
    );
    for (&(block, index), tokens) in points.source_token_inventory() {
        if tokens.synthetic.is_some() || !tokens.evaluates_words() {
            continue;
        }
        let Some(names) = symbols.get(&(block, index)) else {
            continue;
        };
        let Some(body) = blocks.get(&block) else {
            continue;
        };
        let authored = authored_scalar_source_variable(points, block, index, registry);
        let mut retain = |spelling: &str, source: &SourceSite| {
            if source.provenance != crate::ir::Provenance::Source {
                return;
            }
            let read_names = scanner.scan_lexical_word(spelling, registry);
            let mut selected = read_names.iter();
            let Some(name) = selected.next() else {
                return;
            };
            if selected.next().is_some() {
                return;
            }
            let Some(&symbol) = names.get(name) else {
                return;
            };
            let represented = if index == usize::MAX {
                body.exit_versions.contains_key(&symbol)
            } else {
                body.statements
                    .get(index)
                    .is_some_and(|statement| statement.uses.contains_key(&symbol))
            };
            if represented {
                references.insert(
                    (block, index, source.clone(), spelling.to_owned()),
                    SsaReadReference {
                        symbol,
                        version: authored
                            .as_ref()
                            .filter(|(held, held_name, definition)| {
                                !definition.is_definition()
                                    && held_name == name
                                    && interner.source_variables.get(&symbol) == Some(held)
                            })
                            .and_then(|_| {
                                if index == usize::MAX {
                                    body.exit_versions.get(&symbol).copied()
                                } else {
                                    body.statements.get(index)?.uses.get(&symbol).copied()
                                }
                            }),
                    },
                );
            }
        };
        for word in tokens.words() {
            match word {
                WordExpr::Variable { spelling, source } => retain(spelling, source),
                WordExpr::Template {
                    parts,
                    rejected: None,
                    ..
                } => {
                    for part in parts {
                        if let WordPart::Variable { spelling, source } = part {
                            retain(spelling, source);
                        }
                    }
                }
                _ => {}
            }
        }
    }
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
    // Both merges and analysis-only scalar copies start at bottom until a
    // predecessor contributes. Missing copy results are not unknown stores.
    let derived_versions: HashSet<_> = blocks
        .values()
        .flat_map(|block| block.phis.iter().map(|phi| (phi.name, phi.version)))
        .chain(
            clobbers
                .values()
                .flat_map(|markers| markers.values())
                .flat_map(|versions| {
                    versions
                        .iter()
                        .map(|(&symbol, &(_, fresh))| (symbol, fresh))
                }),
        )
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
                    origins.get(&(symbol, prior)).cloned().or_else(|| {
                        (!derived_versions.contains(&(symbol, prior)))
                            .then_some(ContentsOrigin::Unknown)
                    })
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
                                (!derived_versions.contains(&(phi.name, version)))
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
        let authored = authored_scalar_source_variable(&walk.points, id, usize::MAX, registry);
        // A read only the walk's captured binding resolves names the
        // function-wide symbol only where a statement agrees with it
        // ([`project_ssa_source_names`]).
        let captured = source_read_bindings(&walk.points, id, usize::MAX, registry);
        for (source, _) in crate::def_use::terminator_read_vars(block.terminator.as_ref(), config) {
            if let Some(name) =
                canonical_variable_key(&source, walk.points.before_terminator(id), registry)
            {
                symbols.insert(source, walk.interner.intern(&name));
            } else if captured.iter().any(|(held, _)| *held == source) {
                walk.captured_terminator_names.insert(source);
            } else if !authored
                .as_ref()
                .is_some_and(|(_, name, definition)| !definition.is_definition() && name == &source)
            {
                walk.unknown_source_names.insert(source);
            }
        }
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
        if let Some((
            symbol,
            name,
            crate::registry_invocation::AuthoredSourceVariableAccess::Read,
        )) = authored
        {
            let symbol = walk
                .interner
                .intern_identity(&SsaVariableIdentity::Source(symbol));
            symbols.insert(name.clone(), symbol);
        }
        walk.point_symbols.insert((id, usize::MAX), symbols);
    }
}

/// Assemble the final [`SsaBlock`] map from the rename walk's outputs: build
/// each block's phi list and intern its identity-keyed entry / exit version maps
/// onto the persisted [`Symbol`]-keyed form. Extracted from [`build_ssa`].
fn assemble_ssa_blocks(
    func: &cfg::Function,
    phi_vars: &HashMap<BlockId, HashSet<SsaVariableIdentity>>,
    interner: &mut VarInterner,
    out: &mut RenameOutputs,
) -> HashMap<BlockId, SsaBlock> {
    let mut ssa_blocks: HashMap<BlockId, SsaBlock> = HashMap::new();
    for (bn, block) in &func.blocks {
        let mut phis: Vec<Phi> = Vec::new();
        let mut phi_var_list: Vec<SsaVariableIdentity> = phi_vars
            .get(bn)
            .map_or_default(|s| s.iter().cloned().collect());
        phi_var_list.sort_by_cached_key(|key| format!("{key:?}"));

        for var in &phi_var_list {
            phis.push(Phi {
                name: interner.intern_identity(var),
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

        // Intern the entry / exit version maps (built identity-keyed during the
        // walk) onto the persisted [`Symbol`]-keyed block.
        let intern_versions = |interner: &mut VarInterner, m: SsaVisibleVersions| {
            m.into_iter()
                .map(|(name, ver)| (interner.intern_identity(&name), ver))
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

type StatementSourceMayMaps<'a> = (
    &'a mut HashMap<Symbol, Version>,
    &'a mut HashSet<Symbol>,
    &'a mut HashSet<Symbol>,
    &'a mut HashMap<Symbol, Version>,
);

/// Mutable state threaded through the dominator-tree rename walk: the live
/// version stacks / counters keyed by variable identity, the use-scanner and
/// name interner, and the accumulated per-block [`RenameOutputs`].
struct RenameWalk<'a> {
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
    standalone_owner: bool,
    version_counter: HashMap<SsaVariableIdentity, Version>,
    stacks: HashMap<SsaVariableIdentity, Vec<Version>>,
    source_may_calls: HashMap<
        (usize, u32),
        Vec<crate::cfg_builder::global_write_info::OriginalSourceCallMayWrites>,
    >,
    scanner: VarReferenceScanner,
    interner: VarInterner,
    out: RenameOutputs,
    value_clobbers: ValueClobbers,
    points: PointResolveContexts,
    point_symbols: HashMap<(BlockId, usize), HashMap<String, Symbol>>,
    unknown_source_names: HashSet<String>,
    /// Terminator reads the context cannot resolve but the walk's captured
    /// binding does ([`intern_terminator_reads`]).
    captured_terminator_names: HashSet<String>,
}

impl<'a> RenameWalk<'a> {
    #[cfg(test)]
    fn new(
        func: &cfg::Function,
        config: tcl_lexer::LexerConfig,
        points: PointResolveContexts,
        metadata: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
    ) -> Self {
        Self::new_with_standalone_owner(func, config, points, metadata, false)
    }

    fn new_with_standalone_owner(
        func: &cfg::Function,
        config: tcl_lexer::LexerConfig,
        points: PointResolveContexts,
        metadata: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
        standalone_owner: bool,
    ) -> Self {
        let mut out = RenameOutputs::default();
        for id in func.blocks.keys() {
            out.phi_versions.insert(*id, HashMap::new());
            out.phi_incoming.insert(*id, HashMap::new());
            out.entry_versions
                .insert(*id, SsaVisibleVersions::default());
            out.exit_versions.insert(*id, SsaVisibleVersions::default());
            out.stmt_infos.insert(*id, Vec::new());
        }
        Self {
            metadata,
            standalone_owner,
            version_counter: HashMap::new(),
            stacks: HashMap::new(),
            source_may_calls: func.source_may_calls.clone(),
            scanner: VarReferenceScanner::with_config(
                VarScanOptions {
                    include_var_read_roles: (standalone_owner && metadata.is_none())
                        || metadata
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
            captured_terminator_names: HashSet::new(),
        }
    }

    fn invocation_context<'b>(&self, registry: &'b CommandRegistry) -> SsaInvocationContext<'b>
    where
        'a: 'b,
    {
        SsaInvocationContext::supplied_with_standalone_owner(
            registry,
            self.metadata,
            self.standalone_owner,
        )
    }

    /// Top version in the same rename engine, independently of identity domain.
    fn top_identity(&self, var: &SsaVariableIdentity) -> Version {
        self.stacks
            .get(var)
            .and_then(|stack| stack.last().copied())
            .unwrap_or(0)
    }

    fn top(&self, var: &VariableCellKey) -> Version {
        self.top_identity(&SsaVariableIdentity::Location(var.clone()))
    }

    fn push_identity(&mut self, var: &SsaVariableIdentity) -> Version {
        let version = self.version_counter.get(var).copied().unwrap_or(0) + 1;
        self.version_counter.insert(var.clone(), version);
        self.stacks.entry(var.clone()).or_default().push(version);
        version
    }

    fn push_new(&mut self, var: &VariableCellKey) -> Version {
        self.push_identity(&SsaVariableIdentity::Location(var.clone()))
    }

    fn visible_versions(&self, block: BlockId) -> SsaVisibleVersions {
        let mut visible: SsaVisibleIdentities = self
            .stacks
            .keys()
            .cloned()
            .map(|identity| (identity, ()))
            .collect();
        for identity in self.out.phi_versions[&block].keys().cloned() {
            visible.or_default(identity);
        }
        visible
            .into_iter()
            .filter_map(|(identity, ())| {
                let version = self.top_identity(&identity);
                (version > 0).then_some((identity, version))
            })
            .collect()
    }

    fn preserve_unknown_read_liveness(
        &self,
        raw_classified: &[(String, UseClass)],
        context: &ResolveContext,
        registry: &CommandRegistry,
        unknown_arguments: bool,
        classified: &mut Vec<(VariableCellKey, UseClass)>,
    ) {
        if unknown_arguments
            || raw_classified
                .iter()
                .any(|(name, _)| canonical_variable_key(name, context, registry).is_none())
        {
            // An unresolved read may consume any reaching current-frame value.
            // Preserve liveness without claiming a definite uninitialised read.
            let mut visible: Vec<_> = self
                .stacks
                .keys()
                .filter_map(|identity| match identity {
                    SsaVariableIdentity::Location(key) => Some(key.clone()),
                    SsaVariableIdentity::Source(_) => None,
                })
                .collect();
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
        let authored = authored_scalar_source_variable(&self.points, point.0, point.1, registry);
        for (name, _) in raw_classified {
            if let Some(bound) = canonical_variable_key(name, context, registry) {
                source_symbols.insert(name.clone(), self.interner.intern(&bound));
            } else if authored.as_ref().is_none_or(|(_, held, _)| held != name) {
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
        if let Some(store) = self
            .points
            .conditional_logical_scalar_store_at(point.0, point.1, registry)
            && let Some(place) = store.definition_place(registry)
            && let Some(bound) = crate::var_resolve::canonical_binding_value_key(&place)
        {
            let name = store.source_name().to_owned();
            let symbol = self.interner.intern(&bound);
            if source_symbols
                .insert(name.clone(), symbol)
                .is_some_and(|previous| previous != symbol)
            {
                source_symbols.remove(&name);
                self.unknown_source_names.insert(name);
            }
        } else if context.execution_name_policy.is_none()
            && self.invocation_context(registry).standalone
        {
            for name in defs_of_in_context(stmt, Some(self.invocation_context(registry))) {
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
        self.retain_source_word_symbols(stmt, point, registry, &mut source_symbols);
        self.retain_assignment_symbols(stmt, point, registry, &mut source_symbols);
        self.retain_native_operand_symbols(point, context, registry, &mut source_symbols);
        self.point_symbols
            .insert((point.0, point.1), source_symbols);
    }

    fn retain_source_word_symbols(
        &mut self,
        stmt: &Statement,
        point: (BlockId, usize),
        registry: &CommandRegistry,
        source_symbols: &mut HashMap<String, Symbol>,
    ) {
        if let Some(input) = self
            .metadata
            .and_then(crate::registry_invocation::InvocationMetadataContext::source_analysis_input)
            && let Some(tokens) = self.points.source_tokens_at(point.0, point.1)
            && let Some(binding) = tokens.source_binding.as_ref()
        {
            let words = if binding.has_original_logical_source_advice() {
                binding
                    .original_logical_source_input_for_name_roles(tokens, registry)
                    .filter(|held| *held == input || &held.for_nested_source() == input)
                    .and_then(|held| binding.original_logical_source_words(tokens, held, registry))
            } else if binding.has_original_native_source_command() {
                binding
                    .original_native_source_input_for_name_roles(tokens, registry)
                    .filter(|held| *held == input || &held.for_nested_source() == input)
                    .and_then(|held| binding.original_native_source_words(tokens, held, registry))
            } else {
                None
            };
            if let Some(words) = words {
                for (argument, role) in words.written_argument_roles() {
                    if role != tcl_registry::ArgRole::VarWrite {
                        continue;
                    }
                    let Some(word) = tokens.words().get(argument + 1) else {
                        continue;
                    };
                    let Some(receiver_span) = binding.invocation_site().and_then(|site| {
                        word.original_written_span(site.source.source_image().bytes())
                    }) else {
                        continue;
                    };
                    let Some(name) =
                        original_written_source_variable_name(&words, argument, receiver_span)
                    else {
                        continue;
                    };
                    if !defs_of_in_context(stmt, Some(self.invocation_context(registry)))
                        .iter()
                        .any(|defined| defined == name)
                    {
                        continue;
                    }
                    let after = self.points.after_statement(point.0, point.1);
                    if let Some(bound) = definition_source_name(stmt, name, after, registry) {
                        let symbol = self.interner.intern(&bound);
                        if source_symbols
                            .insert(name.to_owned(), symbol)
                            .is_some_and(|old| old != symbol)
                        {
                            source_symbols.remove(name);
                            self.unknown_source_names.insert(name.to_owned());
                        }
                    }
                }
            }
        }
    }

    fn retain_assignment_symbols(
        &mut self,
        stmt: &Statement,
        point: (BlockId, usize),
        registry: &CommandRegistry,
        source_symbols: &mut HashMap<String, Symbol>,
    ) {
        if let Statement::AssignConst { name, .. }
        | Statement::AssignValue { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::Incr { name, .. } = stmt
            && let Some(tokens) = self.points.source_tokens_at(point.0, point.1)
        {
            let original_name = if let Some(metadata) = self.metadata
                && metadata.source_analysis_input().is_some_and(
                    crate::analyser::ResolvedAnalysisInput::has_logical_source_name_context,
                )
                && tokens.source_binding.as_ref().is_some_and(|binding| {
                    !binding.has_original_logical_source_advice()
                        && !binding.has_original_native_source_command()
                }) {
                crate::registry_invocation::logical_structured_invocation_with_metadata_context(
                    registry, metadata, tokens, None,
                )
                .is_some_and(|invocation| {
                    invocation.written_roles().iter().any(|&(argument, role)| {
                        role == tcl_registry::ArgRole::VarWrite
                            && invocation
                                .written_variable_name_operand(argument)
                                .as_deref()
                                == Some(name.as_str())
                    })
                })
            } else {
                crate::registry_invocation::original_assignment_name_operand(registry, tokens, name)
                    .is_some()
            };
            if original_name
                && let Some(bound) = original_structural_definition_key_at(
                    stmt,
                    point.0,
                    point.1,
                    &self.points,
                    registry,
                )
            {
                let symbol = self.interner.intern(&bound);
                if source_symbols
                    .insert(name.clone(), symbol)
                    .is_some_and(|old| old != symbol)
                {
                    source_symbols.remove(name);
                    self.unknown_source_names.insert(name.clone());
                }
            }
        }
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
        let selection = self.invocation_context(registry);
        let raw_classified = uses_of_classified_with_source(
            stmt,
            &mut self.scanner,
            selection,
            self.points.source_tokens_at(point.0, point.1),
        );
        let captured = source_read_bindings(&self.points, point.0, point.1, registry);
        let captured_names = captured_read_names(&self.points, point.0, point.1, registry);
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
        self.preserve_unknown_read_liveness(
            &raw_classified,
            context,
            registry,
            self.points
                .source_arguments_unknown_at(point.0, point.1, registry),
            &mut classified,
        );
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

    fn rename_statement_definitions(
        &mut self,
        frame: &mut RenameFrame,
        expanded: Vec<VariableCellKey>,
        (direct_defs, destructions): (&[VariableCellKey], &[(VariableCellKey, bool)]),
        (uses_map, name_only_uses): (&mut HashMap<Symbol, Version>, &mut HashSet<Symbol>),
    ) -> (HashMap<Symbol, Version>, HashSet<Symbol>, HashSet<Symbol>) {
        let mut defs_map: HashMap<Symbol, Version> = HashMap::new();
        let mut may_defs: HashSet<Symbol> = HashSet::new();
        let mut destruction_defs: HashSet<Symbol> = HashSet::new();
        for var in expanded {
            let destruction = destructions
                .iter()
                .find(|(key, _)| key == &var || var.is_member_of(key));
            if destruction.is_some() {
                let symbol = self.interner.intern(&var);
                record_destruction_predecessor(symbol, self.top(&var), uses_map, name_only_uses);
            }
            let ver = self.push_new(&var);
            frame
                .pushed_vars
                .push(SsaVariableIdentity::Location(var.clone()));
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

        (defs_map, may_defs, destruction_defs)
    }

    fn record_statement_read_versions(
        &mut self,
        (uses_list, fanned): (&[VariableCellKey], &[VariableCellKey]),
        class_of: &HashMap<&VariableCellKey, UseClass>,
        elems: &ArrayElems,
        (uses_map, quoted_uses, name_only_uses): (
            &mut HashMap<Symbol, Version>,
            &mut HashSet<Symbol>,
            &mut HashSet<Symbol>,
        ),
    ) {
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
    }

    fn record_statement_may_uses(
        &mut self,
        switch_may: &[VariableCellKey],
        uses_map: &mut HashMap<Symbol, Version>,
        quoted_uses: &mut HashSet<Symbol>,
    ) {
        let mut may_bases: Vec<_> = switch_may
            .iter()
            .filter_map(|key| {
                let root = key.root();
                (root != key && !switch_may.contains(root)).then(|| root.clone())
            })
            .collect();
        may_bases.sort_by_cached_key(|key| format!("{key:?}"));
        may_bases.dedup();
        for var in switch_may.iter().cloned().chain(may_bases) {
            let sym = self.interner.intern(&var);
            if let std::collections::hash_map::Entry::Vacant(slot) = uses_map.entry(sym) {
                slot.insert(self.top(&var));
                quoted_uses.insert(sym);
            }
        }
        for (identity, versions) in &self.stacks {
            if matches!(identity, SsaVariableIdentity::Source(_))
                && let Some(&version) = versions.last().filter(|version| **version > 0)
            {
                let symbol = self.interner.intern_identity(identity);
                uses_map.entry(symbol).or_insert(version);
                quoted_uses.insert(symbol);
            }
        }
    }

    fn retain_statement_source_may_effects(
        &mut self,
        stmt: &Statement,
        frame: &mut RenameFrame,
        registry: &CommandRegistry,
        index: usize,
        (uses_map, quoted_uses, may_defs, defs_map): StatementSourceMayMaps<'_>,
    ) {
        if self
            .metadata
            .and_then(crate::registry_invocation::InvocationMetadataContext::source_analysis_input)
            .is_some_and(crate::analyser::ResolvedAnalysisInput::has_logical_source_name_context)
            && matches!(stmt, Statement::Call { tokens, foreach_groups: None, .. } if tokens.as_ref().is_none_or(|tokens| tokens.synthetic.is_none()))
        {
            let original = original_source_may_call_from_inventory(
                &self.source_may_calls,
                stmt,
                self.metadata,
                registry,
            );
            let mut definitions = Vec::new();
            if let Some(receipt) = &original {
                for place in receipt.writes() {
                    let identity = SsaVariableIdentity::Source(place.symbol().clone());
                    let symbol = self.interner.intern_identity(&identity);
                    let prior = self.top_identity(&identity);
                    let version = self.push_identity(&identity);
                    frame.pushed_vars.push(identity);
                    uses_map.insert(symbol, prior);
                    quoted_uses.insert(symbol);
                    may_defs.insert(symbol);
                    defs_map.insert(symbol, version);
                    definitions.push((symbol, version));
                }
            }
            self.out.source_may_effects.insert(
                (frame.block, index),
                SourceMaySsaCall {
                    statement: stmt.clone(),
                    original,
                    definitions,
                    clobber: None,
                },
            );
        }
    }

    fn record_authored_scalar_source_reads(
        &mut self,
        authored: Option<&AuthoredScalarSourceAccess>,
        point: (BlockId, usize),
        uses_map: &mut HashMap<Symbol, Version>,
        quoted_uses: &mut HashSet<Symbol>,
    ) {
        if let Some((symbol, name, definition)) = authored {
            let identity = SsaVariableIdentity::Source(symbol.clone());
            let symbol = self.interner.intern_identity(&identity);
            self.point_symbols
                .entry(point)
                .or_default()
                .insert(name.clone(), symbol);
            if !definition.is_definition() || definition.is_may_write() {
                uses_map.insert(symbol, self.top_identity(&identity));
                if definition.is_may_write() {
                    quoted_uses.insert(symbol);
                }
            }
        }
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
        self.record_statement_read_versions(
            (&uses_list, &fanned),
            &class_of,
            elems,
            (&mut uses_map, &mut quoted_uses, &mut name_only_uses),
        );

        let authored = authored_scalar_source_variable(&self.points, frame.block, index, registry);
        self.record_authored_scalar_source_reads(
            authored.as_ref(),
            (frame.block, index),
            &mut uses_map,
            &mut quoted_uses,
        );
        let direct_defs = bound_defs(
            stmt,
            frame.block,
            index,
            &self.points,
            self.invocation_context(registry),
        );
        let destructions = crate::place_bridge::ssa_destruction_keys(
            stmt,
            context,
            self.points.after_statement(frame.block, index),
            registry,
            self.points
                .source_metadata_owner()
                .metadata_context(registry),
        );
        let switch_may = bound_names(
            switch_may_defs_in_context(stmt, self.invocation_context(registry)),
            context,
            registry,
        );
        let all_defs: Vec<_> = direct_defs.iter().chain(&switch_may).cloned().collect();
        let expanded = expand_defs(&all_defs, elems);
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
        // An arm's write does not replace the prior value on the paths where
        // no arm runs, so the statement uses it: liveness keeps the store that
        // feeds it and the solver joins the two. The use is `Quoted` — real
        // for liveness, but not a read the source states, so read-before-set
        // never reports it. An element's base refresh is a may-def of the
        // same kind: where no arm writes the element, the array — or the
        // scalar the write would have raised on — holds what it held, so the
        // store before it is used too ([`refreshed_bases`]).
        self.record_statement_may_uses(&switch_may, &mut uses_map, &mut quoted_uses);
        let (mut defs_map, mut may_defs, destruction_defs) = self.rename_statement_definitions(
            frame,
            expanded,
            (&direct_defs, &destructions),
            (&mut uses_map, &mut name_only_uses),
        );

        if let Some((symbol, _, access)) = authored
            && access.is_definition()
        {
            let identity = SsaVariableIdentity::Source(symbol);
            let symbol = self.interner.intern_identity(&identity);
            let version = self.push_identity(&identity);
            if access.is_may_write() {
                may_defs.insert(symbol);
            }
            frame.pushed_vars.push(identity);
            defs_map.insert(symbol, version);
        }

        self.retain_statement_source_may_effects(
            stmt,
            frame,
            registry,
            index,
            (
                &mut uses_map,
                &mut quoted_uses,
                &mut may_defs,
                &mut defs_map,
            ),
        );

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

    fn rename_block_statements(
        &mut self,
        frame: &mut RenameFrame,
        func: &cfg::Function,
        registry: &CommandRegistry,
        elems: &ArrayElems,
        clobbers: &RegistryClobberNames,
    ) {
        let bn = frame.block;
        // Process statements.
        if let Some(block) = func.blocks.get(&bn) {
            let stmts: Vec<Statement> = block.statements.clone();
            for (index, stmt) in stmts.iter().enumerate() {
                let info = self.rename_statement(stmt, frame, registry, elems, index);
                // The versions held where the call runs, before the fresh
                // ones it leaves are pushed below: the code it reaches may
                // read each of them.
                if is_unseen_call_marker(stmt) {
                    let live = self.visible_versions(bn);
                    self.out.unseen_call_versions.extend(live);
                }
                self.out.stmt_infos.get_mut(&bn).unwrap().push(info);
                if let Some(names) = clobbers.get(&bn).and_then(|markers| markers.get(&index)) {
                    let mut versions = HashMap::new();
                    for name in names {
                        let prior = self.top(name);
                        let fresh = self.push_new(name);
                        frame
                            .pushed_vars
                            .push(SsaVariableIdentity::Location(name.clone()));
                        versions.insert(self.interner.intern(name), (prior, fresh));
                    }
                    self.value_clobbers
                        .entry(bn)
                        .or_default()
                        .insert(index, versions);
                }
            }
            if let Some(infos) = self.out.stmt_infos.get_mut(&bn) {
                demote_reads_beside_writes(infos);
                drop_reads_of_own_writes(infos);
            }
            if self
                .points
                .source_arguments_unknown_at(bn, usize::MAX, registry)
            {
                // The terminator may consume each reaching value. Retain
                // liveness beside the final statement without a definite read.
                let visible = self.visible_versions(bn);
                if let Some(last) = self
                    .out
                    .stmt_infos
                    .get_mut(&bn)
                    .and_then(|infos| infos.last_mut())
                {
                    for (key, version) in visible {
                        let symbol = self.interner.intern_identity(&key);
                        if let std::collections::hash_map::Entry::Vacant(slot) =
                            last.uses.entry(symbol)
                        {
                            slot.insert(version);
                            last.quoted_uses.insert(symbol);
                        }
                    }
                }
            }
        }
    }

    /// Process one block on first visit: assign phi versions, record entry /
    /// exit versions, rename statement uses/defs, and seed successors' phi
    /// incoming edges. Extracted from [`build_ssa`]'s rename walk.
    fn enter_block(
        &mut self,
        frame: &mut RenameFrame,
        func: &cfg::Function,
        phi_vars: &HashMap<BlockId, HashSet<SsaVariableIdentity>>,
        registry: &CommandRegistry,
        elems: &ArrayElems,
        clobbers: &RegistryClobberNames,
    ) {
        let bn = frame.block;

        // Process phi nodes — push new versions.
        let mut phi_var_list: Vec<SsaVariableIdentity> = phi_vars
            .get(&bn)
            .map_or_default(|s| s.iter().cloned().collect());
        phi_var_list.sort_by_cached_key(|key| format!("{key:?}"));

        for var in &phi_var_list {
            let ver = self.push_identity(var);
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

        self.rename_block_statements(frame, func, registry, elems, clobbers);

        // Preserve the versions visible after statements but before any
        // terminator-only effects. Terminator reads and block exits refer to
        // this snapshot; clobbers below affect successor edges only.
        let exit_versions = self.visible_versions(bn);
        *self.out.exit_versions.get_mut(&bn).unwrap() = exit_versions;

        // Terminator reads use the versions recorded above; unknown effects
        // in its nested command substitutions get separate versions for
        // outgoing edges only.
        if let Some(names) = clobbers
            .get(&bn)
            .and_then(|markers| markers.get(&usize::MAX))
        {
            let mut versions = HashMap::new();
            for name in names {
                let prior = self.top(name);
                let fresh = self.push_new(name);
                frame
                    .pushed_vars
                    .push(SsaVariableIdentity::Location(name.clone()));
                versions.insert(self.interner.intern(name), (prior, fresh));
            }
            self.value_clobbers
                .entry(bn)
                .or_default()
                .insert(usize::MAX, versions);
        }

        // Fill in phi incoming edges for successors — the terminator's
        // successors plus any `try` exception-edge handler targets, so
        // a handler block's phis see this block's versions.
        for succ in func.block_successors(bn) {
            if !func.blocks.contains_key(&succ) {
                continue;
            }
            let mut succ_phis: Vec<SsaVariableIdentity> = phi_vars
                .get(&succ)
                .map_or_default(|s| s.iter().cloned().collect());
            succ_phis.sort_by_cached_key(|key| format!("{key:?}"));
            for var in &succ_phis {
                let v = self.top_identity(var);
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
    let config = func.metadata_context.source_analysis_input().map_or_else(
        || tcl_lexer::LexerConfig::for_profile(registry.profile()),
        crate::analyser::ResolvedAnalysisInput::lexer_config,
    );
    build_ssa_with_config(func, registry, config)
}

/// Build SSA under the exact lexer configuration that produced `func`.
///
/// An explicit configuration also supplies environment-only grammar axes for
/// standalone CFGs, which retain no complete source input. Supplied CFGs keep
/// their original metadata checks when an explicit configuration differs.
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
        crate::compilation_unit::function_source_entry(func, None),
    )
}

fn project_ssa_source_names(walk: &mut RenameWalk<'_>, func: &cfg::Function) {
    register_unique_source_names(&mut walk.interner, &walk.point_symbols);
    for name in &walk.unknown_source_names {
        walk.interner.source_symbols.remove(name);
    }
    for name in &walk.captured_terminator_names {
        let symbol = walk.interner.source_symbols.get(name).copied();
        let agreed = symbol.is_some_and(|symbol| {
            walk.point_symbols.iter().any(|(&(_, index), symbols)| {
                index != usize::MAX && symbols.get(name) == Some(&symbol)
            })
        });
        if !agreed {
            walk.interner.source_symbols.remove(name);
        }
    }
    let mut source_names: HashMap<Symbol, Vec<&String>> = HashMap::new();
    for names in walk.point_symbols.values() {
        for (source, &symbol) in names {
            source_names.entry(symbol).or_default().push(source);
        }
    }
    for (symbol, mut names) in source_names {
        // Presentation is stable when several written aliases select one cell.
        names.sort_by_key(|name| {
            (
                tcl_syntax::naming::is_qualified(name.as_bytes()),
                name.len(),
                *name,
            )
        });
        let source = names[0];
        let Some(key) = walk.interner.cell_key(symbol) else {
            continue;
        };
        let authored = matches!(key, VariableCellKey::Authored(_));
        let display = &mut walk.interner.names[symbol.0 as usize];
        if !authored
            || (func.name == "::top"
                && !tcl_syntax::naming::is_qualified(source.as_bytes())
                && *display == crate::naming::rooted_member_key("::", source))
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
                    && interner
                        .cell_key(symbol)
                        .is_none_or(|key| !direct.contains(key))
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
    phi_vars: &HashMap<BlockId, HashSet<SsaVariableIdentity>>,
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
    let standalone_owner = func.metadata_context.is_standalone();
    let metadata = func
        .metadata_context
        .metadata_context(registry)
        .and_then(crate::registry_invocation::InvocationMetadataSelection::context);
    build_ssa_with_context_for_entry_and_metadata(
        func,
        registry,
        config,
        entry,
        entry_bindings,
        metadata,
        standalone_owner,
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
    standalone_owner: bool,
) -> SsaFunction {
    let metadata = supplied_ssa_metadata(registry, config, metadata);
    let initial = build_ssa_inner(
        func,
        registry,
        config,
        entry.clone(),
        &HashMap::new(),
        metadata,
        standalone_owner,
    );
    let bound: Option<HashSet<_>> = entry_bindings.map(|bindings| {
        bindings
            .iter()
            .filter_map(|name| canonical_variable_key(name, &entry, registry))
            .collect()
    });
    let unavailable_metadata = initial.point_contexts.as_ref().is_some_and(|points| {
        let owner = points.source_metadata_owner();
        !owner.is_standalone() && owner.metadata_context(registry).is_none()
    });
    let mut live = crate::slot_allocation::unseen_call_live_names(func, &initial, registry);
    for (block, barriers) in
        crate::slot_allocation::unavailable_metadata_effect_live_names(func, &initial, registry)
    {
        let target = live.entry(block).or_default();
        for (index, symbols) in barriers {
            target.entry(index).or_default().extend(symbols);
        }
    }
    for (block, symbols) in crate::slot_allocation::unavailable_metadata_terminator_live_names(
        func, &initial, registry, config,
    ) {
        live.entry(block).or_default().insert(usize::MAX, symbols);
    }
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
                            let preceding = if index == usize::MAX {
                                statements.as_slice()
                            } else {
                                &statements[..index]
                            };
                            let version = preceding
                                .iter()
                                .rev()
                                .find_map(|stmt| stmt.defs.get(symbol).copied())
                                .or_else(|| {
                                    initial.blocks[&block].entry_versions.get(symbol).copied()
                                })
                                .unwrap_or(0);
                            preceding_defs.is_none_or(|defs| !defs.contains_key(symbol))
                                && (unavailable_metadata
                                    || bound.as_ref().is_none_or(|bindings| {
                                        version != 0
                                            || initial
                                                .cell_key(*symbol)
                                                .is_some_and(|key| bindings.contains(key))
                                    }))
                        })
                        .filter_map(|symbol| initial.cell_key(symbol).cloned())
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
        build_ssa_inner(
            func,
            registry,
            config,
            entry,
            &clobbers,
            metadata,
            standalone_owner,
        )
    }
}

fn intern_unseen_call_versions(walk: &mut RenameWalk<'_>) -> HashSet<(Symbol, Version)> {
    std::mem::take(&mut walk.out.unseen_call_versions)
        .into_iter()
        .map(|(name, version)| (walk.interner.intern_identity(&name), version))
        .collect()
}

fn source_cfg_topology(
    func: &cfg::Function,
    interner: &VarInterner,
    standalone_owner: bool,
) -> HashMap<BlockId, SourceCfgBlockSnapshot> {
    if interner.source_variables.is_empty() && !standalone_owner {
        HashMap::new()
    } else {
        func.blocks
            .iter()
            .map(|(&block, body)| {
                (
                    block,
                    SourceCfgBlockSnapshot {
                        standalone_statements: standalone_owner
                            .then(|| std::sync::Arc::<[Statement]>::from(body.statements.clone())),
                        terminator: body.terminator.clone(),
                        successors: func.block_successors(block),
                    },
                )
            })
            .collect()
    }
}

fn sorted_dominance_frontier(
    frontier: HashMap<BlockId, HashSet<BlockId>>,
) -> HashMap<BlockId, Vec<BlockId>> {
    frontier
        .into_iter()
        .map(|(k, v)| {
            let mut sorted: Vec<BlockId> = v.into_iter().collect();
            sorted.sort_unstable();
            (k, sorted)
        })
        .collect()
}

fn build_ssa_inner(
    func: &cfg::Function,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    entry: ResolveContext,
    clobbers: &RegistryClobberNames,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    standalone_owner: bool,
) -> SsaFunction {
    // Complexity guard: skip the O(blocks·vars) phi placement + rename walk
    // for a pathologically large (usually generated) body. Returns a trivial
    // SSA; the compilation-unit builder likewise produces a trivial analysis
    // and flags the function so per-proc diagnostic passes skip it.
    if is_complexity_guarded(func) {
        return SsaFunction::trivial(func.name.clone(), func.entry, func.block_names().to_vec());
    }

    let mut enriched = enrich_instance_option_defs(func, registry);
    let target = enriched.get_or_insert_with(|| func.clone());
    let _ = mirror_top_level_global_option_defs(target, registry);
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
        SsaInvocationContext::supplied_with_standalone_owner(registry, metadata, standalone_owner),
        &elems,
        config,
        &points,
        clobbers,
    );

    // 2. Set up rename state: the transient version stacks / counters, the
    // use-scanner, name interner, and per-block outputs (keyed by variable
    // name / version, interned to `Symbol` when blocks are assembled).
    let mut walk =
        RenameWalk::new_with_standalone_owner(func, config, points, metadata, standalone_owner);

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
    let source_read_references = retained_source_read_references(
        &walk.points,
        &ssa_blocks,
        &walk.point_symbols,
        registry,
        config,
        &walk.interner,
    );
    let array_refresh = array_root_refresh_metadata(
        func.entry,
        &elems,
        &walk.interner,
        &ssa_blocks,
        &walk.points,
        SsaInvocationContext::supplied_with_standalone_owner(registry, metadata, standalone_owner),
    );
    let cell_names = walk
        .interner
        .keys
        .iter()
        .map(VariableCellKey::compatibility_name)
        .collect();
    let cell_keys = walk.interner.keys.clone();
    let cell_to_symbol = walk.interner.to_symbol.clone();
    project_ssa_source_names(&mut walk, func);

    let unseen_call_versions = intern_unseen_call_versions(&mut walk);
    let source_topology = source_cfg_topology(func, &walk.interner, standalone_owner);
    let original_script_points =
        original_script_source_points(&walk.points, &ssa_blocks, &walk.interner, registry);
    let mut result = SsaFunction {
        name: func.name.clone(),
        entry: func.entry,
        blocks: ssa_blocks,
        value_clobbers: walk.value_clobbers,
        idom,
        dominance_frontier: sorted_dominance_frontier(df),
        dominator_tree: tree,
        block_names: func.block_names().to_vec(),
        var_names: walk.interner.names,
        cell_names,
        cell_indices: walk.interner.cell_indices,
        source_variables: walk.interner.source_variables,
        source_may_effects: walk.out.source_may_effects,
        original_script_points,
        source_topology,
        cell_keys,
        var_to_symbol: walk.interner.source_symbols,
        cell_to_symbol,
        point_symbols: walk.point_symbols,
        point_contexts: Some(walk.points),
        read_references,
        source_read_references,
        array_root_refresh_symbols: array_refresh.symbols,
        array_root_refresh_versions: array_refresh.versions,
        unseen_call_versions,
    };
    result.seal_source_may_clobbers();
    result
}

/// A statement whose `[…]` substitutions write a name reads it before,
/// between and after the write, and the statement's word effects, the
/// definition point the CFG builder pairs with it, define the name, so the
/// statement's own use names the version after the write. No word of it is an
/// operand something may rewrite: each such use becomes a read by name, and
/// the definition point reads the version before the write on the
/// statement's behalf.
fn demote_reads_beside_writes(infos: &mut [SsaStatement]) {
    for call in 0..infos.len() {
        let Some(host) = word_effects_host(infos, call) else {
            continue;
        };
        let written: Vec<Symbol> = infos[call].defs.keys().copied().collect();
        for symbol in written {
            if infos[host].uses.contains_key(&symbol) && !infos[host].quoted_uses.contains(&symbol)
            {
                infos[host].name_only_uses.insert(symbol);
            }
        }
    }
}

/// A script a statement keeps inside itself reads a name it writes as its
/// own: what it consumes of the frame is its reads less its writes, the rule
/// an opaque `switch`'s arms are read by ([`free_reads_in_script`]). An
/// opaque `catch` or a deferred `try` runs its scripts before the call
/// assigns its result and options variables, so the CFG builder states what
/// they write in the marker ahead of the call ([`arm_writes_host`]), and the
/// call's reads of those names are dropped: `catch {set i 0; puts $i}` reads
/// nothing of `i` from the frame. A read after the call reads the marker's
/// may-definition, so a name the scripts only may write is still undefined
/// there on the path where they did not. Like the arms' rule this is blind to
/// order inside the script: a read before the script's own write of the name
/// is dropped too.
fn drop_reads_of_own_writes(infos: &mut [SsaStatement]) {
    for marker in 0..infos.len() {
        let Some(host) = arm_writes_host(infos, marker) else {
            continue;
        };
        let written: Vec<Symbol> = infos[marker].defs.keys().copied().collect();
        for symbol in written {
            infos[host].uses.remove(&symbol);
            infos[host].quoted_uses.remove(&symbol);
            infos[host].name_only_uses.remove(&symbol);
        }
    }
}

/// The statement the [`crate::ir::SyntheticMarker::ArmWrites`] marker at
/// `marker` states the writes of when it stands ahead of it — an opaque
/// `catch` or a deferred `try`: the first statement after it that is no
/// marker, when it shares the marker's span, as the CFG builder puts them.
/// `None` for any other statement, and for the marker that follows an opaque
/// `switch`, whose arms' reads are already free of their own writes.
pub(crate) fn arm_writes_host(infos: &[SsaStatement], marker: usize) -> Option<usize> {
    let statement = &infos.get(marker)?.statement;
    if !is_arm_writes_marker(statement) {
        return None;
    }
    let host = (marker + 1..infos.len()).find(|&i| !is_synthetic_statement(&infos[i].statement))?;
    (infos[host].statement.span() == statement.span()).then_some(host)
}

/// The statement whose words the definition point at `index` states the
/// effects of ([`crate::ir::SyntheticMarker::WordEffects`]): the statement
/// right after it, which the CFG builder pairs with it
/// ([`crate::ir::WordEffectsHost::Statement`]). `None` for any other
/// statement, and for a control statement's words, which no statement holds.
pub(crate) fn word_effects_host(infos: &[SsaStatement], index: usize) -> Option<usize> {
    match infos.get(index)?.statement.word_effects_host()? {
        crate::ir::WordEffectsHost::Statement => {
            let host = index + 1;
            (host < infos.len()).then_some(host)
        }
        crate::ir::WordEffectsHost::Dispatch => None,
    }
}

/// Whether `stmt` is the marker the CFG builder puts where a call to a
/// command the module cannot see runs ([`crate::ir::SyntheticMarker::
/// UnseenCall`]).
pub(crate) fn is_unseen_call_marker(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::Call { tokens: Some(tokens), .. }
            if tokens.synthetic == Some(crate::ir::SyntheticMarker::UnseenCall)
    )
}

/// Whether `stmt` is the marker the CFG builder puts beside an opaque
/// `switch`, `catch` or `try` for the names its scripts write
/// ([`crate::ir::SyntheticMarker::ArmWrites`]).
pub(crate) fn is_arm_writes_marker(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::Call { tokens: Some(tokens), .. }
            if tokens.synthetic == Some(crate::ir::SyntheticMarker::ArmWrites)
    )
}

/// Whether `stmt` is a marker that only states an effect the statement beside
/// it has: where code the module cannot see runs, or what the scripts it keeps
/// inside itself write. It shares its host's span and is no command to run or
/// plan.
#[doc(hidden)]
#[must_use]
pub fn is_effect_marker(stmt: &Statement) -> bool {
    is_unseen_call_marker(stmt) || is_arm_writes_marker(stmt)
}

/// Whether `stmt` states may-definitions of the kind [`switch_may_defs`]
/// returns: an opaque `switch`, a marker, or a call carrying callee-summary
/// names.
pub(crate) fn has_arm_may_defs(stmt: &Statement) -> bool {
    matches!(stmt, Statement::Switch { .. })
        || is_effect_marker(stmt)
        || matches!(stmt, Statement::Call { defs, .. } if !defs.is_empty())
}

/// Whether `stmt` is a statement the CFG builder synthesised to carry an
/// effect ([`crate::ir::SyntheticMarker`]).
fn is_synthetic_statement(stmt: &Statement) -> bool {
    match stmt {
        Statement::Call { tokens, .. } | Statement::Barrier { tokens, .. } => tokens
            .as_ref()
            .is_some_and(|tokens| tokens.synthetic.is_some()),
        _ => false,
    }
}

#[cfg(test)]
mod tests;

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
mod cell_resolution_tests;

#[cfg(test)]
mod external_mutability_tests;

#[cfg(test)]
mod original_definition_name_tests;

#[cfg(test)]
mod native_source_definition_tests;

#[cfg(test)]
mod original_source_read_selector_tests;

#[cfg(test)]
mod original_analysis_definition_selector_tests;

#[cfg(test)]
mod original_authored_analysis_definition_selector_tests;

#[cfg(test)]
mod retained_host_grammar_tests;

#[cfg(test)]
mod unavailable_metadata_value_effect_tests;
