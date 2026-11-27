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

//! The renaming walk over the dominator tree: version stacks, each statement's reads and definitions, and the φ operands of successor blocks.

use super::super::defs::{bound_defs, bound_names, defs_of_in_context, switch_may_defs_in_context};
use super::super::uses::uses_of_classified_with_source;
use super::super::{
    RegistryClobberNames, SourceMaySsaCall, SsaInvocationContext, SsaStatement, Symbol, UseClass,
    Version, arm_writes_host, is_unseen_call_marker, word_effects_host,
};
use super::phi_placement::{
    AuthoredScalarSourceAccess, authored_scalar_source_variable, expand_defs, expand_uses,
    original_source_may_call_from_inventory,
};
use super::read_references::{additional_read_bindings, captured_read_names, source_read_bindings};
use super::{
    ArrayElems, RenameFrame, RenameOutputs, RenamePhase, RenameWalk, SsaVariableIdentity,
    SsaVisibleIdentities, SsaVisibleVersions, StatementSourceMayMaps, VarInterner,
};
use crate::cfg;
use crate::cfg::BlockId;
use crate::ir::Statement;
use crate::var_refs::{VarReferenceScanner, VarScanOptions};
use crate::var_resolve::{ResolveContext, canonical_variable_key};
use crate::variable_bindings::PointResolveContexts;
use std::collections::{HashMap, HashSet};
use tcl_names::variable::VariableCellKey;
use tcl_registry::CommandRegistry;

pub(in super::super) fn record_destruction_predecessor(
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

impl<'a> RenameWalk<'a> {
    #[cfg(test)]
    pub(in super::super) fn new(
        func: &cfg::Function,
        config: tcl_lexer::LexerConfig,
        points: PointResolveContexts,
        metadata: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
    ) -> Self {
        Self::new_with_standalone_owner(func, config, points, metadata, false)
    }

    pub(super) fn new_with_standalone_owner(
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
            summary_links: None,
        }
    }

    pub(in super::super) fn invocation_context<'b>(
        &self,
        registry: &'b CommandRegistry,
    ) -> SsaInvocationContext<'b>
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

    pub(in super::super) fn top(&self, var: &VariableCellKey) -> Version {
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

    /// In a summary's SSA ([`RenameWalk::summary_links`]), a `set`-shaped
    /// store the walk proved no place for defines the scalar cell its literal
    /// name reads in this frame: a write through `upvar 1 $name v` stores to
    /// whatever `v` is linked to, so `v` holds the written value afterwards.
    /// Whether it did; the value refreshes it needs are the SSA builder's
    /// ([`crate::slot_allocation::summary_store_live_names`]).
    fn retain_summary_store(
        &mut self,
        stmt: &Statement,
        point: (BlockId, usize),
        registry: &CommandRegistry,
        direct_defs: &mut Vec<VariableCellKey>,
    ) -> bool {
        let (Statement::AssignConst {
            name, name_braced, ..
        }
        | Statement::AssignValue {
            name, name_braced, ..
        }
        | Statement::AssignExpr {
            name, name_braced, ..
        }
        | Statement::Incr {
            name, name_braced, ..
        }) = stmt
        else {
            return false;
        };
        if !direct_defs.is_empty()
            || !stmt.is_executable_invocation()
            || crate::ssa::is_dynamic_write_target(name, *name_braced)
            || tcl_syntax::naming::split_element_ref_bytes(name.as_bytes()).is_some()
        {
            return false;
        }
        let before = self.points.before_statement(point.0, point.1);
        let Some(cell) = canonical_variable_key(name, before, registry) else {
            return false;
        };
        direct_defs.push(cell);
        self.out.summary_stores.fallback_defs.insert(point);
        true
    }

    /// What `stmt` does to the linked locals of a summary's SSA
    /// ([`RenameWalk::summary_links`]), under the summary's own guards (its
    /// frame closed, no code it cannot see). A write or `unset` through a
    /// link — a place the walk could not name, beside a name that is a
    /// linked local — defines or destroys that local; the place it reached
    /// may be any other linked local's too. A write to a place outside the
    /// frame, or a call to a command the registry does not describe, may
    /// reach every linked place, and so may a scope-alias command that
    /// re-links a local already written. A call that names a linked local
    /// in a word defines it: its evaluation states the outcome. Each linked
    /// local a statement may reach, and does not define or destroy, may be
    /// written or unset there.
    fn summary_link_effects(
        &self,
        stmt: &Statement,
        point: (BlockId, usize),
        registry: &CommandRegistry,
        (direct_defs, fallback): (&[VariableCellKey], bool),
    ) -> SummaryLinkEffects {
        let mut effects = SummaryLinkEffects::default();
        let Some(links) = self.summary_links.as_ref() else {
            return effects;
        };
        let before = self.points.before_statement(point.0, point.1);
        let after = self.points.after_statement(point.0, point.1);
        let linked = links
            .iter()
            .filter_map(|name| canonical_variable_key(name, before, registry))
            .collect::<Vec<_>>();
        if linked.is_empty() || !stmt.is_executable_invocation() {
            return effects;
        }
        let named = || {
            defs_of_in_context(stmt, Some(self.invocation_context(registry)))
                .into_iter()
                .filter_map(|name| canonical_variable_key(&name, before, registry))
                .filter(|cell| linked.contains(cell))
                .collect::<Vec<_>>()
        };
        let unnamed = |places: &[tcl_names::place::Place]| {
            places
                .iter()
                .any(|place| crate::var_resolve::canonical_binding_value_key(place).is_none())
        };
        let written = unnamed(&crate::place_bridge::def_places_at(
            stmt,
            point.0,
            point.1,
            &self.points,
            registry,
        ));
        let destroyed = unnamed(
            &crate::place_bridge::statement_destruction_places_with_continuation_with_metadata_context(
                stmt,
                before,
                after,
                registry,
                self.points
                    .source_metadata_owner()
                    .metadata_context(registry),
            ),
        );
        if written {
            effects.defined = named();
        }
        // A call to a command the registry does not describe — a procedure
        // of the module — that names a linked local in a word: the call's
        // own evaluation states what that place becomes (its callee's
        // transfer summary, or any outcome).
        if let Statement::Call { command, args, .. } = stmt
            && registry.get(command).is_none()
        {
            for word in args {
                if let Some(cell) = canonical_variable_key(word, before, registry)
                    && linked.contains(&cell)
                    && !effects.defined.contains(&cell)
                {
                    effects.defined.push(cell);
                }
            }
        }
        if destroyed {
            effects.destroyed = named();
        }
        let through_a_link = fallback && direct_defs.iter().any(|cell| linked.contains(cell))
            || !effects.defined.is_empty()
            || !effects.destroyed.is_empty()
            || written
            || destroyed;
        let outside = direct_defs
            .iter()
            .any(|cell| !matches!(cell, VariableCellKey::Activation { .. }));
        let (unknown_command, relinks) = match stmt {
            Statement::Call { command, args, .. } | Statement::Barrier { command, args, .. } => {
                match registry.get(command) {
                    None => (true, false),
                    Some(spec)
                        if spec
                            .traits
                            .contains(tcl_registry::Traits::CREATES_SCOPE_ALIAS) =>
                    {
                        let relinks = args.iter().any(|word| {
                            canonical_variable_key(word, before, registry)
                                .is_some_and(|cell| linked.contains(&cell) && self.top(&cell) != 0)
                        });
                        (false, relinks)
                    }
                    Some(_) => (false, false),
                }
            }
            _ => (false, false),
        };
        if through_a_link || outside || unknown_command || relinks {
            effects.reached = linked
                .into_iter()
                .filter(|cell| {
                    !direct_defs.contains(cell)
                        && !effects.defined.contains(cell)
                        && !effects.destroyed.contains(cell)
                })
                .collect();
        }
        effects
    }

    /// The cells `stmt` defines (its proved places, or, in a summary's SSA,
    /// the cell a store's name reads), the ones it destroys, the ones an arm
    /// may write, and all of them with the linked locals it may reach.
    fn statement_definition_cells(
        &mut self,
        stmt: &Statement,
        point: (BlockId, usize),
        context: &ResolveContext,
        registry: &CommandRegistry,
    ) -> DefinitionCells {
        let mut direct_defs = bound_defs(
            stmt,
            point.0,
            point.1,
            &self.points,
            self.invocation_context(registry),
        );
        let fallback = self.summary_links.is_some()
            && self.retain_summary_store(stmt, point, registry, &mut direct_defs);
        let links = self.summary_link_effects(stmt, point, registry, (&direct_defs, fallback));
        direct_defs.extend(links.defined.iter().cloned());
        let mut destructions = crate::place_bridge::ssa_destruction_keys(
            stmt,
            context,
            self.points.after_statement(point.0, point.1),
            registry,
            self.points
                .source_metadata_owner()
                .metadata_context(registry),
        );
        destructions.extend(links.destroyed.iter().map(|cell| (cell.clone(), false)));
        destructions.extend(links.reached.iter().map(|cell| (cell.clone(), true)));
        direct_defs.extend(links.destroyed.iter().cloned());
        let reached_links = links.reached;
        let switch_may = bound_names(
            switch_may_defs_in_context(stmt, self.invocation_context(registry)),
            context,
            registry,
        );
        let all_defs: Vec<_> = direct_defs
            .iter()
            .chain(&switch_may)
            .chain(&reached_links)
            .cloned()
            .collect();
        DefinitionCells {
            direct: direct_defs,
            destructions,
            switch_may,
            all: all_defs,
        }
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
        let DefinitionCells {
            direct: direct_defs,
            destructions,
            switch_may,
            all: all_defs,
        } = self.statement_definition_cells(stmt, (frame.block, index), context, registry);
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

pub(super) fn rename_dominator_tree(
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

/// What one statement defines ([`RenameWalk::statement_definition_cells`]).
struct DefinitionCells {
    direct: Vec<VariableCellKey>,
    destructions: Vec<(VariableCellKey, bool)>,
    switch_may: Vec<VariableCellKey>,
    all: Vec<VariableCellKey>,
}

/// What one statement does to a summary's linked locals
/// ([`RenameWalk::summary_link_effects`]).
#[derive(Default)]
struct SummaryLinkEffects {
    /// Written through the link.
    defined: Vec<VariableCellKey>,
    /// Unset through the link.
    destroyed: Vec<VariableCellKey>,
    /// May be written or unset by another route.
    reached: Vec<VariableCellKey>,
}
