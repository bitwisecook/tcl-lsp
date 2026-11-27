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

use rustc_hash::FxHashMap;
use tcl_registry::CommandRegistry;

use crate::cfg::{self, BlockId};
use crate::ir::CommandTokens;
use crate::ir::Statement;
use crate::var_refs::VarReferenceScanner;
#[cfg(test)]
use crate::var_refs::VarScanOptions;
#[cfg(test)]
use crate::var_resolve::ContentsOrigin;
use crate::var_resolve::{ResolveContext, VariableCellKey, canonical_variable_key};
use crate::variable_bindings::PointResolveContexts;
use tcl_source::words::SourceSite;
#[cfg(test)]
use tcl_source::words::WordExpr;
mod construction;
mod defs;
mod dominators;
mod function_queries;
mod instance_options;
mod source_may_effects;
mod source_view;
mod uses;

#[cfg(test)]
pub(crate) use construction::ArrayElems;
use construction::build_ssa_inner;
#[cfg(test)]
pub(crate) use construction::compute_phi_vars;
#[cfg(test)]
use construction::represented_contents_origins;
#[cfg(test)]
use construction::{
    RenameWalk, expand_defs, invocation_read_key_in_context, record_destruction_predecessor,
    source_read_name,
};
#[cfg(test)]
use defs::{collapsed_extra_defs, defs_of_in_context, switch_may_defs_in_context};
pub use defs::{defs_of, defs_of_with_registry};
pub(crate) use defs::{defs_of_with_metadata_context, nested_writes, refreshed_bases};
#[cfg(test)]
use defs::{nested_writes_in_context, selected_barrier_defs};
#[cfg(test)]
pub(crate) use dominators::compute_idom;
pub use dominators::{DominatorIntervals, compute_dominators};
#[cfg(test)]
pub(crate) use dominators::{build_dom_tree, compute_dominance_frontier, compute_idom_fast};
#[cfg(test)]
use instance_options::mirror_top_level_global_option_defs;
#[cfg(test)]
pub(crate) use instance_options::original_command_option_writes;
#[cfg(test)]
use instance_options::original_global_root_frame;
pub(crate) use instance_options::{
    OriginalLinkedOptionWrite, enrich_instance_option_defs_with_initial,
    linked_option_definition_name, original_command_taint_option_writes,
    original_instance_option_writes,
};
pub(crate) use source_may_effects::SourceVariableSsaSnapshot;
#[cfg(test)]
use source_view::source_read_place;
pub use source_view::{
    OriginalAnalysisSsaDefinition, OriginalAnalysisSsaWords, OriginalFreshScalarAssignmentProposal,
    OriginalFreshScalarVariableProposal, OriginalSourceSsaDefinition,
    OriginalSourceSsaDefinitionName, OriginalSourceSsaRead, OriginalSsaDefinitionName,
    SsaIncomingSlotRead, SsaReadContents, SsaReadPresenceAlternatives, SsaSourceView,
    original_analysis_definition_at_unit, original_analysis_words_for_statement,
    original_source_definition_at_unit, original_source_read_at_unit,
};
pub(crate) use source_view::{SsaReadRepresentationAdvice, current_authored_analysis_function};
pub(crate) use uses::structural_body_indices;
#[cfg(test)]
use uses::{
    BracedWordSite, braced_word_owns_name, scan_selected_expression_roles, set_value_reads,
};
pub use uses::{uses_of, uses_of_classified};
#[cfg(test)]
use uses::{uses_of_classified_in_context, uses_of_classified_with_source};

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

/// Key identifying a specific SSA value: `(variable symbol, version)`.
pub type ValueKey = (Symbol, Version);

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

/// Analysis value transitions: prior and fresh versions at each marker.
pub type ValueClobbers = HashMap<BlockId, BlockValueClobbers>;

/// Per-marker prior and fresh versions within one basic block.
pub type BlockValueClobbers = HashMap<usize, HashMap<Symbol, (Version, Version)>>;

type RegistryClobberNames = HashMap<BlockId, HashMap<usize, Vec<VariableCellKey>>>;

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

pub use tcl_syntax::naming::unattributed_dynamic_written_variable_target as is_dynamic_write_target;

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
    build_ssa_for_entry(
        func,
        (registry, config),
        entry,
        entry_bindings,
        metadata,
        (standalone_owner, None),
    )
}

/// The SSA a transfer summary runs on: as
/// [`build_ssa_with_context_for_entry_and_metadata`], except that a store
/// the walk proved no place for defines the cell its name reads, so a write
/// through a local linked to a caller's place defines that local, with the
/// refreshes that keep it sound; every statement that may reach one of the
/// `linked` locals' places by another route may write or unset that local.
pub(crate) fn build_summary_ssa(
    func: &cfg::Function,
    (registry, config): (&CommandRegistry, tcl_lexer::LexerConfig),
    entry: ResolveContext,
    entry_bindings: Option<&[String]>,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    standalone_owner: bool,
    linked: &[String],
) -> SsaFunction {
    build_ssa_for_entry(
        func,
        (registry, config),
        entry,
        entry_bindings,
        metadata,
        (standalone_owner, Some(linked)),
    )
}

fn build_ssa_for_entry(
    func: &cfg::Function,
    (registry, config): (&CommandRegistry, tcl_lexer::LexerConfig),
    entry: ResolveContext,
    entry_bindings: Option<&[String]>,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    (standalone_owner, summary): (bool, Option<&[String]>),
) -> SsaFunction {
    let metadata = supplied_ssa_metadata(registry, config, metadata);
    let (initial, summary_stores) = build_ssa_inner(
        func,
        registry,
        config,
        entry.clone(),
        &HashMap::new(),
        metadata,
        (standalone_owner, summary),
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
    let mut clobbers: RegistryClobberNames =
        shared_barrier_live_names(func, &initial, registry, config)
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
    if summary.is_some() {
        retain_summary_store_refreshes(func, &initial, registry, &summary_stores, &mut clobbers);
    }
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
            (standalone_owner, summary),
        )
        .0
    }
}

/// The values live across each statement the shared SSA refreshes them at:
/// a call to unseen code, an operation whose metadata owner is unavailable,
/// and a store the walk proved no place for.
fn shared_barrier_live_names(
    func: &cfg::Function,
    initial: &SsaFunction,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> HashMap<BlockId, HashMap<usize, HashSet<Symbol>>> {
    let mut live = crate::slot_allocation::unseen_code_live_names(func, initial, registry);
    for (block, barriers) in
        crate::slot_allocation::unavailable_metadata_effect_live_names(func, initial, registry)
            .into_iter()
            .chain(crate::slot_allocation::unproved_store_live_names(
                func, initial, registry,
            ))
    {
        let target = live.entry(block).or_default();
        for (index, symbols) in barriers {
            target.entry(index).or_default().extend(symbols);
        }
    }
    for (block, symbols) in crate::slot_allocation::unavailable_metadata_terminator_live_names(
        func, initial, registry, config,
    ) {
        live.entry(block).or_default().insert(usize::MAX, symbols);
    }
    live
}

/// Add a summary's refreshes across its fallback stores
/// ([`crate::slot_allocation::summary_store_live_names`]) to `clobbers`.
/// They are added after the shared filter, which sees only the shared SSA.
fn retain_summary_store_refreshes(
    func: &cfg::Function,
    initial: &SsaFunction,
    registry: &CommandRegistry,
    stores: &construction::SummaryStores,
    clobbers: &mut RegistryClobberNames,
) {
    for (block, markers) in crate::slot_allocation::summary_store_live_names(
        func,
        initial,
        registry,
        &stores.fallback_defs,
    ) {
        let target = clobbers.entry(block).or_default();
        for (index, symbols) in markers {
            let names = target.entry(index).or_default();
            for key in symbols
                .into_iter()
                .filter_map(|symbol| initial.cell_key(symbol).cloned())
            {
                if !names.contains(&key) {
                    names.push(key);
                }
            }
            names.sort_by_cached_key(|key| format!("{key:?}"));
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
