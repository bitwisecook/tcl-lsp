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

//! Building the SSA form: φ placement, the renaming walk over the dominator tree, retained read references and the assembled function.

use super::defs::bound_defs;
use super::dominators::{
    build_dom_tree, compute_dominance_frontier, compute_idom_fast, sorted_dominance_frontier,
};
use super::instance_options::{enrich_instance_option_defs, mirror_top_level_global_option_defs};
use super::{
    Phi, RegistryClobberNames, SourceCfgBlockSnapshot, SourceMaySsaCall, SsaBlock, SsaFunction,
    SsaInvocationContext, SsaStatement, Symbol, ValueClobbers, Version, is_complexity_guarded,
};
use crate::cfg;
use crate::cfg::BlockId;
use crate::ir::Statement;
use crate::var_refs::VarReferenceScanner;
use crate::var_resolve::{ResolveContext, VariableCellKey, canonical_variable_key};
use crate::variable_bindings::PointResolveContexts;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::{HashMap, HashSet};
use tcl_registry::CommandRegistry;

mod phi_placement;
mod read_references;
mod rename;
mod statement_symbols;

#[cfg(test)]
pub(crate) use phi_placement::compute_phi_vars;
#[cfg(test)]
pub(super) use phi_placement::expand_defs;
use phi_placement::{
    authored_scalar_source_variable, collect_array_elems, compute_phi_vars_with_config,
};
#[cfg(test)]
pub(super) use read_references::invocation_read_key_in_context;
pub(super) use read_references::represented_contents_origins;
#[cfg(test)]
pub(super) use read_references::source_read_name;
use read_references::{
    original_script_source_points, retained_read_references, retained_source_read_references,
    source_read_bindings,
};
#[cfg(test)]
pub(super) use rename::record_destruction_predecessor;
use rename::rename_dominator_tree;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum SsaVariableIdentity {
    Location(VariableCellKey),
    Source(crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol),
}

// Buckets retain full authenticated identities; hashes never decide equality.
pub(super) struct SsaIdentityBuckets<V, S = rustc_hash::FxBuildHasher> {
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
    pub(super) fn contains_key(&self, key: &SsaVariableIdentity) -> bool {
        let hash = std::hash::BuildHasher::hash_one(self.buckets.hasher(), key);
        self.buckets
            .get(&hash)
            .is_some_and(|bucket| bucket.iter().any(|(stored, _)| stored == key))
    }

    pub(super) fn insert(&mut self, key: SsaVariableIdentity, value: V) {
        let hash = std::hash::BuildHasher::hash_one(self.buckets.hasher(), &key);
        let bucket = self.buckets.entry(hash).or_default();
        if let Some((_, stored_value)) = bucket.iter_mut().find(|(stored, _)| stored == &key) {
            *stored_value = value;
        } else {
            bucket.push((key, value));
        }
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = (&SsaVariableIdentity, &V)> {
        self.buckets
            .values()
            .flatten()
            .map(|(key, value)| (key, value))
    }
}

impl<V: Default, S: std::hash::BuildHasher> SsaIdentityBuckets<V, S> {
    pub(super) fn or_default(&mut self, key: SsaVariableIdentity) -> &mut V {
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

pub(super) type SsaVisibleVersions =
    SsaIdentityBuckets<Version, std::collections::hash_map::RandomState>;

type SsaIdentitySet = SsaIdentityBuckets<()>;

type SsaDefinitionSites = SsaIdentityBuckets<FxHashSet<BlockId>>;

/// Per-[`SsaFunction`] interner for location and source identities.
///
/// Each identity receives one dense [`Symbol`]; display labels and the
/// location-only inventory retain their separate projections.
#[derive(Debug, Default, Clone)]
pub(super) struct VarInterner {
    pub(super) names: Vec<String>,
    keys: Vec<VariableCellKey>,
    pub(super) cell_indices: Vec<Option<usize>>,
    pub(super) source_variables:
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

    pub(super) fn cell_key(&self, symbol: Symbol) -> Option<&VariableCellKey> {
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
    pub(super) elements: FxHashMap<VariableCellKey, HashSet<VariableCellKey>>,
    element_roots: FxHashMap<VariableCellKey, VariableCellKey>,
}

impl ArrayElems {
    pub(super) fn note(&mut self, mut target: crate::place::Place) {
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

    pub(super) fn get(&self, root: &VariableCellKey) -> Option<&HashSet<VariableCellKey>> {
        self.elements.get(root)
    }

    pub(super) fn root_for_element(&self, element: &VariableCellKey) -> Option<&VariableCellKey> {
        self.element_roots.get(element)
    }
}

/// Frame for the iterative rename walk (avoids deep recursion).
struct RenameFrame {
    pub(super) block: BlockId,
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
pub(super) struct RenameOutputs {
    phi_versions: HashMap<BlockId, HashMap<SsaVariableIdentity, Version>>,
    phi_incoming: HashMap<BlockId, HashMap<SsaVariableIdentity, HashMap<BlockId, Version>>>,
    pub(super) entry_versions: HashMap<BlockId, SsaVisibleVersions>,
    pub(super) exit_versions: HashMap<BlockId, SsaVisibleVersions>,
    stmt_infos: HashMap<BlockId, Vec<SsaStatement>>,
    pub(super) source_may_effects: HashMap<(BlockId, usize), SourceMaySsaCall>,
    /// The versions live at each call to a command the module cannot see.
    pub(super) unseen_call_versions: Vec<(SsaVariableIdentity, Version)>,
}

pub(super) fn intern_terminator_reads(
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
pub(super) struct RenameWalk<'a> {
    pub(super) metadata: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
    standalone_owner: bool,
    version_counter: HashMap<SsaVariableIdentity, Version>,
    stacks: HashMap<SsaVariableIdentity, Vec<Version>>,
    source_may_calls: HashMap<
        (usize, u32),
        Vec<crate::cfg_builder::global_write_info::OriginalSourceCallMayWrites>,
    >,
    scanner: VarReferenceScanner,
    interner: VarInterner,
    pub(super) out: RenameOutputs,
    pub(super) value_clobbers: ValueClobbers,
    points: PointResolveContexts,
    pub(super) point_symbols: HashMap<(BlockId, usize), HashMap<String, Symbol>>,
    unknown_source_names: HashSet<String>,
    /// Terminator reads the context cannot resolve but the walk's captured
    /// binding does ([`intern_terminator_reads`]).
    captured_terminator_names: HashSet<String>,
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
    pub(super) versions: HashSet<(Symbol, Version)>,
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

pub(super) fn build_ssa_inner(
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
