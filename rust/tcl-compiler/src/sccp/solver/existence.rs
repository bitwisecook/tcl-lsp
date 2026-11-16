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

//! The existence rung: the fact each place enters with, its forward propagation through statements and handler regions, and the places only a query names.

use super::super::{ExistenceEntry, ExistenceInitialState};
use super::frame_clobbers::{place_base, statement_clobber, terminator_clobber};
use super::refinements::{BranchFact, existence_guards, existence_refinements, narrowed};
use super::{Clobber, ExistenceAt, ExistenceRun, RungResults};
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::expr_ast::ExprNode;
use crate::ir::Statement;
use crate::ssa::{SsaFunction, SsaStatement, Symbol, ValueKey};
use crate::value_transfer::LatticeDriver;
use std::collections::{HashMap, HashSet};
use tcl_registry::CommandRegistry;
use tcl_registry::value_transfer::{BindingKind, DomainFact, Existence, FactView};

impl ExistenceAt<'_> {
    /// Record the facts the statement at `index` finds at each place it
    /// reads: the settled sweep's stay.
    pub(super) fn record_reads(
        &mut self,
        index: usize,
        stmt_ssa: &SsaStatement,
        driver: &LatticeDriver<'_>,
    ) {
        let Ok(index) = u32::try_from(index) else {
            return;
        };
        for &symbol in stmt_ssa.uses.keys() {
            let fact = driver.existence_now(symbol).unwrap_or(Existence::Pending);
            self.run.reads.insert((self.block, index, symbol), fact);
        }
    }

    /// Join the current point into the block's points, for a region block.
    fn note_point(&mut self, driver: &LatticeDriver<'_>) {
        if let Some(points) = self.through.as_mut() {
            for (fact, symbol) in points.iter_mut().zip(0u32..) {
                if let Some(now) = driver.existence_now(Symbol(symbol)) {
                    *fact = fact.join(now);
                }
            }
        }
    }

    /// Apply the clobber the statement at `index` performs, after its own
    /// storage outcomes, and join the point into the block's points.
    pub(super) fn finish_statement(&mut self, index: usize, driver: &LatticeDriver<'_>) {
        if let Some(clobber) = self.run.clobbers.get(&(self.block, index)) {
            driver.existence_clobber(|fact| clobber.applied(fact));
            for &symbol in &clobber.touched {
                driver.existence_step(
                    symbol,
                    crate::value_transfer::ExistenceStep::Set(Existence::MayBound),
                );
            }
        }
        self.note_point(driver);
    }

    /// A barrier or a static-body `uplevel` at `index`: every place is
    /// may-bound afterwards, its own definitions included; whether a
    /// version's fact moved.
    pub(in super::super) fn barrier(
        &mut self,
        index: usize,
        stmt_ssa: &SsaStatement,
        driver: &LatticeDriver<'_>,
    ) -> bool {
        self.finish_statement(index, driver);
        let mut changed = false;
        for (&var, &ver) in &stmt_ssa.defs {
            changed |= self.run.record_version((var, ver), Existence::MayBound);
        }
        changed
    }

    /// The step the definition `var` takes: an externally mutable place
    /// stays may-bound, a typed assignment binds, and any other statement's
    /// comes from its evaluation.
    pub(super) fn step_for(
        &self,
        stmt_ssa: &SsaStatement,
        (var, name, element_write_base): (Symbol, &str, Option<Symbol>),
        driver: &LatticeDriver<'_>,
        (kept, evaluated): (bool, impl FnOnce() -> crate::value_transfer::ExistenceStep),
    ) -> crate::value_transfer::ExistenceStep {
        if self
            .run
            .mutable
            .get(var.0 as usize)
            .copied()
            .unwrap_or(true)
        {
            return crate::value_transfer::ExistenceStep::Set(Existence::MayBound);
        }
        // A statement that raised before it stored leaves a place as it was,
        // a typed assignment included.
        if kept {
            return evaluated();
        }
        let step = assignment_existence(stmt_ssa, var, element_write_base, driver)
            .unwrap_or_else(evaluated);
        // The registry's write class states what a call does to the places
        // it names where its declared transfer states less.
        driver
            .existence_by_write_class(&stmt_ssa.statement, name, step)
            .unwrap_or(step)
    }

    /// Advance the definition `key`'s place by `step` and record the fact
    /// its version is established with; whether it moved.
    pub(super) fn advance(
        &mut self,
        key: ValueKey,
        step: crate::value_transfer::ExistenceStep,
        driver: &LatticeDriver<'_>,
    ) -> bool {
        let fact = driver
            .existence_step(key.0, step)
            .unwrap_or(Existence::Pending);
        self.run.record_version(key, fact)
    }

    /// Apply a computed name in the terminator's words, and the places a
    /// body nested in one of its substitutions may touch, before the branch
    /// reads its condition.
    pub(super) fn before_terminator(&self, driver: &LatticeDriver<'_>) {
        if let Some(clobber) = self.run.terminator_clobbers.get(&self.block) {
            driver.existence_clobber(|fact| clobber.applied(fact));
            for &symbol in &clobber.touched {
                driver.existence_step(
                    symbol,
                    crate::value_transfer::ExistenceStep::Set(Existence::MayBound),
                );
            }
        }
    }

    /// Leave the block: record its exit, and for a region block the join
    /// of its points, its entry among them where a throw can leave from it
    /// (`entry_throws`); whether either moved.
    pub(in super::super) fn leave(self, driver: &LatticeDriver<'_>, entry_throws: bool) -> bool {
        let Some(exit) = driver.existence_leave() else {
            return false;
        };
        let through = self.through.map(|mut points| {
            if let Some(entry) = self.entry.as_deref().filter(|_| entry_throws) {
                join_state(&mut points, entry);
            }
            join_state(&mut points, &exit);
            points
        });
        self.run.finish_block(self.block, exit, through)
    }
}

/// Join `incoming` into `state`, place by place.
fn join_state(state: &mut [Existence], incoming: &[Existence]) {
    for (fact, other) in state.iter_mut().zip(incoming) {
        *fact = fact.join(*other);
    }
}

/// Join `state` into what `states` holds for `block`; whether it moved.
fn join_into(
    states: &mut HashMap<BlockId, Vec<Existence>>,
    block: BlockId,
    state: Vec<Existence>,
) -> bool {
    if let Some(old) = states.get_mut(&block) {
        let before = old.clone();
        join_state(old, &state);
        return *old != before;
    }
    states.insert(block, state);
    true
}

impl ExistenceRun {
    /// The run for `cfg` / `ssa` entered under `entry`; `escaping` is the
    /// value lattice's externally-mutable set, and `facts` what each branch
    /// edge's condition proves, of which the run takes the existence facts.
    pub(in super::super) fn new(
        cfg: &CfgFunction,
        ssa: &SsaFunction,
        entry: ExistenceEntry<'_>,
        (escaping, facts, caller_frame): (
            &HashSet<String>,
            &[BranchFact],
            crate::dynamic_names::DynamicNameBarrier,
        ),
        registry: &CommandRegistry,
    ) -> Self {
        // The SSA's symbols, then a slot per place only an existence query
        // names, in that order, so a symbol's slot is its index.
        let symbols = ssa.var_names().len();
        let extra = query_only_places(cfg, ssa, registry, entry.config);
        let mut names: Vec<String> = ssa.var_names().to_vec();
        names.extend(extra.iter().cloned());
        let query_only: HashMap<String, Symbol> = extra
            .into_iter()
            .zip(symbols..)
            .filter_map(|(name, slot)| Some((name, Symbol(u32::try_from(slot).ok()?))))
            .collect();
        let mutable: Vec<bool> = names
            .iter()
            .enumerate()
            .map(|(slot, name)| {
                u32::try_from(slot)
                    .is_ok_and(|slot| ssa.source_variable_symbol(Symbol(slot)).is_some())
                    || entry.dynamic_trace
                    || tcl_syntax::naming::is_qualified(name.as_bytes())
                    || escaping.contains(name.as_str())
                    || escaping.contains(place_base(name))
            })
            .collect();
        let dialect = Some(tcl_registry::special_vars::surface_query_for_profile(
            registry.profile(),
        ));
        let external: Vec<bool> = names
            .iter()
            .zip(&mutable)
            .map(|(name, &mutable)| mutable || linked_elsewhere(name, entry, registry, dialect))
            .collect();
        let caller_frame = Clobber::of_names(caller_frame);
        let entry_state: Vec<Existence> = names
            .iter()
            .zip(&mutable)
            .map(|(name, &mutable)| {
                let fact = if mutable {
                    Existence::MayBound
                } else {
                    entry_fact(name, entry, registry, dialect)
                };
                caller_frame.applied(fact)
            })
            .collect();
        let mut clobbers = HashMap::new();
        let mut terminator_clobbers = HashMap::new();
        for (&block_id, block) in &cfg.blocks {
            for (index, statement) in block.statements.iter().enumerate() {
                let at = (cfg, block_id, index);
                let clobber = statement_clobber(statement, ssa, at, registry, entry.config);
                if !clobber.is_empty() {
                    clobbers.insert((block_id, index), clobber);
                }
            }
            if let Some(terminator) = &block.terminator {
                let clobber = terminator_clobber(terminator, ssa, cfg, registry, entry.config);
                if !clobber.is_empty() {
                    terminator_clobbers.insert(block_id, clobber);
                }
            }
        }
        let refinements = existence_refinements(facts, ssa, (&query_only, &external));
        let guards = existence_guards(facts);
        let mut by_edge: HashMap<(BlockId, BlockId), Vec<(usize, Existence)>> = HashMap::new();
        for refinement in &refinements {
            if let FactView::Domain(DomainFact::Existence(fact)) = &refinement.fact {
                by_edge
                    .entry(refinement.edge)
                    .or_default()
                    .push((refinement.key.0.0 as usize, *fact));
            }
        }
        let entry_edges: HashSet<(BlockId, BlockId)> = cfg
            .region_entries
            .iter()
            .map(|entry| (entry.source, entry.handler))
            .collect();
        let handler_regions = handler_regions(cfg, &entry_edges);
        let in_regions = handler_regions.values().flatten().copied().collect();
        // Version 0 of each SSA place is what the frame enters with; a
        // query-only slot has no version.
        let versions = (0u32..)
            .zip(entry_state.iter().take(symbols))
            .map(|(symbol, fact)| ((Symbol(symbol), 0), *fact))
            .collect();
        Self {
            entry: entry_state,
            mutable,
            external,
            clobbers,
            terminator_clobbers,
            entry_edges,
            handler_regions,
            in_regions,
            exits: HashMap::new(),
            through: HashMap::new(),
            versions,
            reads: HashMap::new(),
            query_only,
            refinements,
            by_edge,
            guards,
            entry_states: HashMap::new(),
        }
    }

    /// What the run leaves in the result once the solver settles.
    pub(in super::super) fn finish(self, ssa: &SsaFunction) -> RungResults {
        RungResults {
            entries: self.block_qualified(ssa),
            query_places: self.query_places(),
            versions: self.versions,
            reads: self.reads,
            exits: self.exits,
            guards: self.guards,
        }
    }

    /// The places only an existence query names, by their slot past the
    /// SSA's symbols.
    pub(in super::super) fn query_places(&self) -> Vec<String> {
        let mut places: Vec<(&String, Symbol)> = self
            .query_only
            .iter()
            .map(|(name, &slot)| (name, slot))
            .collect();
        places.sort_by_key(|&(_, slot)| slot);
        places.into_iter().map(|(name, _)| name.clone()).collect()
    }

    /// Enter `block` for a sweep: its entry state, which each φ takes, at
    /// the driver's cursor; `changed` is set when a φ's fact moved.
    pub(in super::super) fn enter<'r>(
        &'r mut self,
        cfg: &CfgFunction,
        (block, ssa_block): (BlockId, &crate::ssa::SsaBlock),
        incoming: &[BlockId],
        driver: &LatticeDriver<'_>,
        changed: &mut bool,
    ) -> ExistenceAt<'r> {
        let state = self.block_entry(cfg, block, incoming);
        self.entry_states.insert(block, state.clone());
        for phi in &ssa_block.phis {
            let fact = state
                .get(phi.name.0 as usize)
                .copied()
                .unwrap_or(Existence::Pending);
            *changed |= self.record_version((phi.name, phi.version), fact);
        }
        let in_region = self.in_regions.contains(&block);
        let through = in_region.then(|| vec![Existence::Pending; state.len()]);
        let entry = in_region.then(|| state.clone());
        driver.existence_enter(state);
        ExistenceAt {
            run: self,
            block,
            through,
            entry,
        }
    }

    /// The state `block` enters with: the entry rules for the function's
    /// entry, joined with every executable incoming edge's contribution —
    /// a predecessor's exit, or across an exception edge every point of the
    /// handler's region. A predecessor the sweep has not reached yet
    /// contributes nothing.
    fn block_entry(
        &self,
        cfg: &CfgFunction,
        block: BlockId,
        incoming: &[BlockId],
    ) -> Vec<Existence> {
        let mut state = if block == cfg.entry {
            self.entry.clone()
        } else {
            vec![Existence::Pending; self.entry.len()]
        };
        for &pred in incoming {
            let normal = cfg
                .blocks
                .get(&pred)
                .is_some_and(|b| b.successors().contains(&block));
            if normal && let Some(exit) = self.exits.get(&pred) {
                join_state(&mut state, &self.arriving((pred, block), exit));
            }
            if self.entry_edges.contains(&(pred, block)) {
                // The state before the body is the block before it, at its
                // exit.
                if let Some(exit) = self.exits.get(&pred) {
                    join_state(&mut state, exit);
                }
            } else if cfg.exception_edges.contains(&(pred, block)) {
                for region in self.handler_regions.get(&block).into_iter().flatten() {
                    if let Some(points) = self.through.get(region) {
                        join_state(&mut state, points);
                    }
                }
            }
        }
        state
    }

    /// `exit`, the state a predecessor leaves with, as it arrives along
    /// `edge`: each refinement of the edge narrows its place.
    fn arriving<'e>(
        &self,
        edge: (BlockId, BlockId),
        exit: &'e [Existence],
    ) -> std::borrow::Cow<'e, [Existence]> {
        let Some(facts) = self.by_edge.get(&edge) else {
            return std::borrow::Cow::Borrowed(exit);
        };
        let mut state = exit.to_vec();
        for &(slot, fact) in facts {
            if let Some(place) = state.get_mut(slot) {
                *place = narrowed(*place, fact);
            }
        }
        std::borrow::Cow::Owned(state)
    }

    /// The block-qualified facts: for each executable block and the version
    /// of each place live at its entry, the fact the place holds there
    /// where it differs from the fact the version was established with. A
    /// slot past the SSA's symbols has no version, so it has none.
    fn block_qualified(&self, ssa: &SsaFunction) -> HashMap<(BlockId, ValueKey), Existence> {
        let mut out = HashMap::new();
        for (&block, state) in &self.entry_states {
            let live = ssa
                .blocks
                .get(&block)
                .map(|ssa_block| &ssa_block.entry_versions);
            for (slot, &here) in (0u32..).zip(state) {
                let symbol = Symbol(slot);
                let version = live
                    .and_then(|versions| versions.get(&symbol))
                    .copied()
                    .unwrap_or(0);
                let key = (symbol, version);
                if here != Existence::Pending
                    && self.versions.get(&key).is_some_and(|&own| own != here)
                {
                    out.insert((block, key), here);
                }
            }
        }
        out
    }

    /// Record `fact` for `key`, joined with what an earlier sweep recorded;
    /// whether the record moved.
    fn record_version(&mut self, key: ValueKey, fact: Existence) -> bool {
        let old = self
            .versions
            .get(&key)
            .copied()
            .unwrap_or(Existence::Pending);
        let joined = old.join(fact);
        if joined == old && self.versions.contains_key(&key) {
            return false;
        }
        self.versions.insert(key, joined);
        true
    }

    /// Record a block's exit and, for a region block, the join of its
    /// points; whether either moved.
    fn finish_block(
        &mut self,
        block: BlockId,
        exit: Vec<Existence>,
        through: Option<Vec<Existence>>,
    ) -> bool {
        let mut changed = join_into(&mut self.exits, block, exit);
        if let Some(points) = through {
            changed |= join_into(&mut self.through, block, points);
        }
        changed
    }
}

/// The places an existence query names that no SSA symbol holds, sorted: a
/// name the function only asks `info exists` or `array exists` about — `if
/// {[info exists x]}` with no other mention of `x` — or the array an
/// element query asks about, which the query reads (an element exists only
/// in an array). Each takes a slot past the SSA's symbols, so the rung
/// carries it from the entry through every clobber like any other place.
fn query_only_places(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Vec<String> {
    let mut names = Vec::new();
    for (&block_id, block) in &cfg.blocks {
        for (index, statement) in block.statements.iter().enumerate() {
            let parent = cfg.source_tokens_at(block_id, index);
            match statement {
                Statement::AssignExpr {
                    expr, expr_base, ..
                } => {
                    collect_query_names(
                        expr,
                        crate::existence_query::QueryReading::for_cfg(
                            cfg, registry, config, parent, *expr_base,
                        ),
                        &mut names,
                    );
                }
                Statement::AssignValue { value, .. } => {
                    let reading = crate::existence_query::QueryReading::for_cfg(
                        cfg, registry, config, parent, None,
                    );
                    if cfg.metadata_context.is_standalone() {
                        names.extend(reading.in_text(value).map(|query| query.var));
                    } else if let Some(calls) = parent
                        .and_then(|parent| crate::word_subst::checked_lifted_calls(parent, config))
                    {
                        for call in calls {
                            names.extend(
                                call.tokens
                                    .as_ref()
                                    .or(call.words.as_ref())
                                    .and_then(|tokens| reading.in_tokens(tokens))
                                    .map(|query| query.var),
                            );
                        }
                    }
                }
                _ => {}
            }
        }
        if let Some(Terminator::Branch {
            condition,
            condition_base,
            ..
        }) = &block.terminator
        {
            collect_query_names(
                condition,
                crate::existence_query::QueryReading::for_cfg(
                    cfg,
                    registry,
                    config,
                    cfg.source_tokens_at(block_id, usize::MAX),
                    *condition_base,
                ),
                &mut names,
            );
        }
    }
    let mut places: Vec<String> = names
        .iter()
        .map(|name| place_base(name).to_owned())
        .filter(|place| ssa.var_symbol(place).is_none())
        .collect();
    places.sort_unstable();
    places.dedup();
    places
}

/// Keep command leaves attached to the same original expression site while
/// walking its tree. The explicit stack avoids native recursion depth limits.
fn collect_query_names(
    expression: &ExprNode,
    reading: crate::existence_query::QueryReading<'_>,
    names: &mut Vec<String>,
) {
    let mut pending = vec![expression];
    while let Some(node) = pending.pop() {
        match node {
            ExprNode::Command { .. } => {
                names.extend(reading.in_expr(node).map(|query| query.var));
            }
            ExprNode::Binary { left, right, .. } => pending.extend([&**left, &**right]),
            ExprNode::Unary { operand, .. } => pending.push(operand),
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                pending.extend([&**condition, &**true_branch, &**false_branch]);
            }
            ExprNode::Call { args, .. } => pending.extend(args.iter()),
            _ => {}
        }
    }
}

/// The fact a place named `name` enters an unaliased frame with: a
/// parameter bound as a scalar; an element whose array the frame links or
/// holds from elsewhere, a `TclOO` instance variable, a connection-scoped
/// name, and a special variable startup does not bind all may-bound; a
/// special variable startup binds bound as its kind; anything else unbound.
fn entry_fact(
    name: &str,
    entry: ExistenceEntry<'_>,
    registry: &CommandRegistry,
    dialect: Option<tcl_dialect::model::SurfaceQuery<'static>>,
) -> Existence {
    if entry.initial_state == ExistenceInitialState::InheritedFrame {
        return Existence::MayBound;
    }
    let linked = |name: &str| linked_to_state(name, entry);
    let special = |name: &str| special_at_entry(name, entry, registry, dialect);
    let base = place_base(name);
    if base != name {
        // Which elements an array holds is its own fact: an element of a
        // parameter, a linked array or a special one is not provably
        // absent.
        let held_elsewhere = entry.params.iter().any(|param| param == base)
            || linked(base)
            || special(base).is_some()
            || entry.caller_places.is_some_and(|places| places.links(base));
        return if held_elsewhere {
            Existence::MayBound
        } else {
            Existence::Unbound
        };
    }
    if entry.params.iter().any(|param| param == name) {
        return Existence::Bound(BindingKind::Scalar);
    }
    if let Some(&fact) = entry
        .caller_places
        .and_then(|places| places.entries.get(name))
    {
        return fact;
    }
    if linked(name) {
        return Existence::MayBound;
    }
    if let Some(spec) = special(name) {
        let bound = registry.is_initially_bound(name, dialect);
        return match spec.kind {
            tcl_registry::special_vars::SpecialVarKind::Scalar if bound => {
                Existence::Bound(BindingKind::Scalar)
            }
            tcl_registry::special_vars::SpecialVarKind::Array if bound => {
                Existence::Bound(BindingKind::Array)
            }
            _ => Existence::MayBound,
        };
    }
    Existence::Unbound
}

/// Whether `name` is a `TclOO` instance variable (not shadowed by a
/// parameter) or a name an iRules `when` handler's connection binds: state
/// another invocation holds.
fn linked_to_state(name: &str, entry: ExistenceEntry<'_>) -> bool {
    entry.object_state.is_some_and(|state| state.contains(name))
        && !entry.params.iter().any(|param| param == name)
        || entry
            .connection_scoped
            .is_some_and(|scoped| scoped.holds(name))
}

/// The registry's special variable `name`, when the body is the document's
/// initial global frame, where the host rather than the script binds it.
fn special_at_entry(
    name: &str,
    entry: ExistenceEntry<'_>,
    registry: &CommandRegistry,
    dialect: Option<tcl_dialect::model::SurfaceQuery<'static>>,
) -> Option<&'static tcl_registry::special_vars::SpecialVarSpec> {
    entry
        .initial_global
        .then(|| registry.special_var_in_dialect(name, dialect))
        .flatten()
}

/// Whether the place `name`, or the array it is an element of, is linked
/// to state another invocation, the object or the host holds: a `TclOO`
/// instance variable, a connection-scoped name, or a special variable of
/// the initial global frame. Beside the qualified, aliased and traced
/// places the rung already holds may-bound, these are the externally
/// mutable places a refinement never narrows.
pub(in super::super) fn linked_elsewhere(
    name: &str,
    entry: ExistenceEntry<'_>,
    registry: &CommandRegistry,
    dialect: Option<tcl_dialect::model::SurfaceQuery<'static>>,
) -> bool {
    let base = place_base(name);
    linked_to_state(base, entry) || special_at_entry(base, entry, registry, dialect).is_some()
}

/// Per handler block, the region its exception edges leave from: every
/// block on a normal path from one of its edges' sources to another — a
/// `try` or `catch` body between its first command and its tail — sources
/// included. Any command in the region may throw, so the handler sees every
/// point of it. The block before the body is no part of it: its edge
/// (`entry_edges`) carries the state at its exit alone.
pub(in super::super) fn handler_regions(
    cfg: &CfgFunction,
    entry_edges: &HashSet<(BlockId, BlockId)>,
) -> HashMap<BlockId, Vec<BlockId>> {
    let mut sources: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
    for &(from, to) in &cfg.exception_edges {
        if entry_edges.contains(&(from, to)) {
            continue;
        }
        let listed = sources.entry(to).or_default();
        if !listed.contains(&from) {
            listed.push(from);
        }
    }
    if sources.is_empty() {
        return HashMap::new();
    }
    let mut predecessors: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
    for (&id, block) in &cfg.blocks {
        for successor in block.successors() {
            predecessors.entry(successor).or_default().push(id);
        }
    }
    let reach = |starts: &[BlockId], next: &dyn Fn(BlockId) -> Vec<BlockId>| {
        let mut seen: HashSet<BlockId> = starts.iter().copied().collect();
        let mut work: Vec<BlockId> = starts.to_vec();
        while let Some(block) = work.pop() {
            for other in next(block) {
                if seen.insert(other) {
                    work.push(other);
                }
            }
        }
        seen
    };
    sources
        .into_iter()
        .map(|(handler, starts)| {
            let forward = reach(&starts, &|block| {
                cfg.blocks
                    .get(&block)
                    .map_or_default(crate::cfg::Block::successors)
            });
            let backward = reach(&starts, &|block| {
                predecessors.get(&block).cloned().unwrap_or_default()
            });
            let mut region: Vec<BlockId> = forward.intersection(&backward).copied().collect();
            region.sort_unstable_by_key(|block| block.0);
            (handler, region)
        })
        .collect()
}

/// The existence step a typed assignment (`set`'s lowering) takes on the
/// definition `var`: the written place binds as a scalar, the array an
/// element write refreshes binds as an array, and an element the SSA fans a
/// dynamic-key write over may have been bound. `None` for any other
/// statement, whose step is its evaluation's; a typed assignment whose
/// `set` the module rebinds takes the generic widening.
fn assignment_existence(
    stmt_ssa: &SsaStatement,
    var: Symbol,
    element_write_base: Option<Symbol>,
    driver: &LatticeDriver<'_>,
) -> Option<crate::value_transfer::ExistenceStep> {
    use crate::value_transfer::ExistenceStep;
    if !matches!(
        stmt_ssa.statement,
        Statement::AssignConst { .. }
            | Statement::AssignExpr { .. }
            | Statement::AssignValue { .. }
    ) {
        return None;
    }
    Some(if !driver.typed_assignment_trusted() {
        ExistenceStep::UNKNOWN
    } else if element_write_base == Some(var) {
        ExistenceStep::Set(Existence::Bound(BindingKind::Array))
    } else if stmt_ssa.may_defs.contains(&var) {
        ExistenceStep::Join(Existence::Bound(BindingKind::Scalar))
    } else {
        ExistenceStep::Set(Existence::Bound(BindingKind::Scalar))
    })
}
