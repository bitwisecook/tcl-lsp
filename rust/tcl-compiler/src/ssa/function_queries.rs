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

//! Queries on a built `SsaFunction`: its outline, cells, symbols, names and blocks, and the trivial function of a guarded body.

use super::dominators::DominatorIntervals;
use super::{SsaFunction, SsaReadReference, Symbol, Version};
use crate::cfg;
use crate::cfg::BlockId;
use crate::ir::{CommandTokens, SourceSite};
use crate::var_resolve::{VariableCellKey, VariableCellKeyQuery};
use rustc_hash::FxHashMap;
use std::collections::{HashMap, HashSet};

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
