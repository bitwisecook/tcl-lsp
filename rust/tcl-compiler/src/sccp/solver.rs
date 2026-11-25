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

//! The solver: its rounds, the sweep over the blocks, the existence rung, refinements, loop enumeration and the settled post-passes.

use super::{
    BranchFold, BuiltinFoldInputs, EdgeRefinement, EnumeratedLoop, ModuleRun, SccpResult,
    TraceInputs, cfg_order, compute_predecessors, entry_block_set,
};
use crate::analyses::LatticeValue;
use crate::cfg::{BlockId, Function as CfgFunction};
use crate::ssa::{SsaFunction, Symbol, ValueKey};
use crate::tcl_expr_eval::FoldPolicy;
use crate::value_transfer::LatticeDriver;
use std::collections::{HashMap, HashSet};
use tcl_registry::value_transfer::Existence;

mod branches;
mod existence;
mod frame_clobbers;
mod loop_enumeration;
mod refinements;
mod statements;
mod sweep;

#[cfg(test)]
pub(super) use branches::branch_deferrable;
pub(super) use branches::collect_constant_branches;
use branches::settled_statements;
pub(crate) use frame_clobbers::place_base;
use loop_enumeration::enumerate_loops;
pub(super) use refinements::{Narrowing, RefinementFlow, ordered_refinements};
use refinements::{
    branch_facts, contradicted_exits, external_places, refined_values, value_refinements,
};
pub(crate) use statements::prior_version;
#[cfg(test)]
pub(super) use statements::sccp_process_statements;
#[cfg(test)]
pub(super) use statements::typed_element_write_base;
pub(super) use sweep::sccp_process_phis;
pub(super) use sweep::{seed_live_in_roots, seeded_values};

/// What a run must assume of the whole function when only the blocks an
/// `executable` filter admits can run: the names it keeps overdefined
/// ([`escaping_names`]) and whether a computed name may write or destroy any
/// place ([`crate::dynamic_names::dynamic_name_barrier_in`]).
#[derive(Clone, PartialEq, Eq)]
pub(super) struct RunHazards {
    escaping: HashSet<String>,
    computed_writes: bool,
    caller_frame: crate::dynamic_names::DynamicNameBarrier,
}

impl RunHazards {
    pub(super) fn of(inputs: &SolveInputs<'_>, executable: &dyn Fn(&BlockId) -> bool) -> Self {
        let trace = inputs.trace.with_callback_writes_as_traces();
        let names = crate::dynamic_names::dynamic_name_barrier_in(
            inputs.cfg,
            trace.registry,
            solve_lexer_config(trace),
            executable,
        );
        Self {
            escaping: escaping_names(
                inputs.cfg,
                &trace,
                inputs.extra_escaping,
                inputs.module.owned,
                executable,
            ),
            computed_writes: names.writes
                || names.destroys
                || callback_writes_any(inputs.cfg, trace.deferred_writes, executable),
            caller_frame: inputs.cfg.caller_frame_barrier_in(executable),
        }
    }

    fn covers(&self, other: &Self) -> bool {
        other.escaping.is_subset(&self.escaping)
            && (self.computed_writes || !other.computed_writes)
            && self.caller_frame.union(other.caller_frame) == self.caller_frame
    }

    pub(super) fn absorb(&mut self, other: Self) {
        self.escaping.extend(other.escaping);
        self.computed_writes |= other.computed_writes;
        self.caller_frame = self.caller_frame.union(other.caller_frame);
    }
}

/// Whether a callback a statement registers may write any variable, leaving
/// out the statements at sites this function holds in blocks `executable`
/// rejects: those never run, so they register nothing. A site this function
/// does not hold is another function's, which may run.
pub(super) fn callback_writes_any(
    cfg: &CfgFunction,
    deferred: &crate::ir::DeferredWrites,
    executable: &dyn Fn(&BlockId) -> bool,
) -> bool {
    if !deferred.any || deferred.any_sites.is_empty() {
        return deferred.any;
    }
    let skipped: HashSet<&crate::command_binding::CommandAllocationSite> = cfg
        .blocks
        .iter()
        .filter(|(id, _)| !executable(id))
        .flat_map(|(_, block)| &block.statements)
        .filter_map(|statement| {
            statement
                .tokens()?
                .source_binding
                .as_ref()?
                .invocation_site()
        })
        .collect();
    deferred.any_beyond(|site| skipped.contains(site))
}

/// [`drive`] under the whole-function hazards its own executable blocks
/// carry.
///
/// A block that never runs aliases nothing and writes no computed name, but
/// which blocks run is what the solve decides, and a hazard makes values
/// overdefined. So the hazards start from what no block contributes and grow
/// by what the blocks each solve leaves executable contribute, until a
/// solve's blocks contribute nothing new. Larger hazards only make more
/// values overdefined and more blocks executable, so they rise and the loop
/// ends; the last solve ran under every hazard its executable blocks carry.
/// Without this, one arm the source walk proves dead (lowered as an opaque
/// call because the walk binds nothing in it) made every variable of the
/// function external, so not even the condition that kills the arm could be
/// decided.
pub(super) fn drive_escaping(inputs: &SolveInputs<'_>) -> SccpResult {
    let every_block = RunHazards::of(inputs, &|_| true);
    let mut hazards = RunHazards::of(inputs, &|_| false);
    if hazards.covers(&every_block) {
        return drive(|round| solve(inputs, round, &every_block));
    }
    loop {
        let result = drive(|round| solve(inputs, round, &hazards));
        let needed = RunHazards::of(inputs, &|id| result.executable_blocks.contains(id));
        if hazards.covers(&needed) {
            return result;
        }
        hazards.absorb(needed);
    }
}

/// The solver's rounds over `solve`, one run of the solver each. A loop the
/// first run proves the start state of is run to its exit, and the run is
/// made again with the state it leaves stated on the edges it leaves by: the
/// values only descend. The second run enumerates nothing. Where its settled
/// values contradict the state some loops left, those loops' states are
/// dropped and the run made again without them; every other loop's state
/// stands. Each such run drops at least one loop, so the rounds end.
pub(super) fn drive(mut solve: impl FnMut(&Round) -> Solved) -> SccpResult {
    let mut round = Round::First;
    let mut loops = Vec::new();
    loop {
        match solve(&round) {
            Solved::Settled(result) => {
                return SccpResult {
                    loop_enumerations: loops,
                    ..*result
                };
            }
            Solved::Enumerated(enumerated, exits) => {
                loops = enumerated;
                round = Round::Published(exits);
            }
            Solved::Contradicted(ends) => {
                loops.retain(|record| !ends.contains(&record.exit_block));
                if let Round::Published(exits) = &mut round {
                    exits.retain(|refinement| !ends.contains(&refinement.edge.1));
                }
            }
        }
    }
}

/// The inputs of one solver run.
#[derive(Clone, Copy)]
pub(crate) struct SolveInputs<'a> {
    pub(crate) cfg: &'a CfgFunction,
    pub(crate) ssa: &'a SsaFunction,
    pub(crate) param_constants: Option<&'a HashMap<(String, crate::ssa::Version), LatticeValue>>,
    pub(crate) policy: FoldPolicy,
    pub(crate) extra_escaping: &'a HashSet<String>,
    pub(crate) trace: TraceInputs<'a>,
    pub(crate) folds: Option<BuiltinFoldInputs<'a>>,
    pub(crate) module: ModuleRun<'a>,
}

impl<'a> SolveInputs<'a> {
    /// The run's driver over `trace` and the escaping set, reading what the
    /// run reads of the module.
    pub(super) fn driver(
        &self,
        trace: TraceInputs<'a>,
        escaping: &HashSet<String>,
    ) -> LatticeDriver<'a> {
        LatticeDriver::new(trace, self.folds, self.policy, escaping)
            .with_cfg_metadata(&self.cfg.metadata_context)
            .in_module(self.cfg, self.module)
    }
}

/// Which run of the solver this is.
pub(super) enum Round {
    /// The first: the loops whose start state it proves are enumerated.
    First,
    /// The second, with the refinements the first run's loops published.
    Published(Vec<EdgeRefinement>),
}

/// What a run of the solver answers.
pub(super) enum Solved {
    /// The settled result.
    Settled(Box<SccpResult>),
    /// The first run enumerated loops whose exits publish state: the loops
    /// and the refinements their exits state, for the second run.
    Enumerated(Vec<EnumeratedLoop>, Vec<EdgeRefinement>),
    /// The second run's settled values contradict the state the loops that
    /// leave to these blocks left.
    Contradicted(HashSet<BlockId>),
}

fn solve_grammar(trace: TraceInputs<'_>) -> tcl_dialect::LexerGrammar {
    trace.source_metadata_input.map_or_else(
        || {
            trace
                .registry
                .profile()
                .map_or_else(tcl_dialect::LexerGrammar::default, |p| p.grammar)
        },
        |input| input.unit_profile().grammar,
    )
}

fn solve_lexer_config(trace: TraceInputs<'_>) -> tcl_lexer::LexerConfig {
    trace.existence.map_or_else(
        || {
            trace.source_metadata_input.map_or_else(
                || crate::dynamic_names::lexer_config_for(trace.registry),
                crate::analyser::ResolvedAnalysisInput::lexer_config,
            )
        },
        |entry| entry.config,
    )
}

/// One run of the solver ([`sccp_with_builtin_folds`]).
pub(super) fn solve(inputs: &SolveInputs<'_>, round: &Round, hazards: &RunHazards) -> Solved {
    let SolveInputs {
        cfg,
        ssa,
        param_constants,
        policy,
        trace,
        ..
    } = *inputs;
    let trace = trace.with_callback_writes_as_traces();
    let preds = compute_predecessors(cfg);
    let mut values = seeded_values(param_constants, |name| ssa.var_symbol(name));

    let grammar = solve_grammar(trace);
    seed_live_in_roots(cfg, ssa, &mut values, grammar, |_, name| {
        ssa.var_symbol(name)
    });

    let escaping = hazards.escaping.clone();
    // Every command-specific answer below comes from the registry's
    // declaration for the resolved invocation, through one driver whose
    // context is this run's identity.
    let driver = inputs.driver(trace, &escaping);
    driver.catch_ends(cfg);
    // What every branch edge's condition proves, in every domain, read
    // under the document's grammar.
    let config = solve_lexer_config(trace);
    let facts = branch_facts(cfg, (trace.registry, config, policy), &driver);
    // The existence rung runs beside the values, over the same executable
    // blocks and edges, when the caller asks for it.
    let mut existence = trace.existence.map(|entry| {
        ExistenceRun::new(
            cfg,
            ssa,
            entry,
            (&escaping, &facts, hazards.caller_frame),
            trace.registry,
        )
    });
    if let Some(run) = &existence {
        driver.existence_places(run.query_only.clone());
        driver.existence_external(run.external.clone());
    }
    // Every refinement, in order: the existence rung's, which it carries
    // itself, and every other domain's, which the sweep reads through the
    // blocks they hold in, the state enumerated loops leave among them.
    let external = external_places(&trace, &escaping, hazards.computed_writes);
    let mut others = value_refinements(&facts, ssa, &external);
    if let Round::Published(exits) = round {
        others.extend_from_slice(exits);
    }
    let refinements = ordered_refinements(existence.as_mut(), others);

    let executable_blocks = entry_block_set(cfg);
    let order = cfg_order(cfg);
    let regions = RegionShape::of(cfg);
    let sweep = SweepContext {
        cfg,
        ssa,
        preds: &preds,
        escaping: &escaping,
        // A computed name any executable block writes or destroys may be any
        // place, as a trace on a computed name may observe any.
        has_dynamic_variable_trace: trace.has_dynamic_variable_trace || hazards.computed_writes,
        grammar,
        driver: &driver,
        regions: &regions,
        refinements: &refinements,
        external: &external,
    };
    let mut state = SweepState {
        values,
        executable_blocks,
        executable_edges: HashSet::new(),
        existence,
        refined: RefinementFlow::new(cfg, &refinements),
    };

    sweep.settle(&order, &mut state);

    let SweepState {
        mut values,
        executable_blocks,
        executable_edges,
        existence,
        refined,
    } = state;
    let refinements_at = refined.map_or_default(RefinementFlow::settled);
    if matches!(round, Round::Published(_)) {
        let ends = contradicted_exits(&refinements, &refinements_at, &values);
        if !ends.is_empty() {
            return Solved::Contradicted(ends);
        }
    }
    if matches!(round, Round::First) {
        let (loops, exits) = enumerate_loops(
            (cfg, ssa, &preds),
            (&mut values, Narrowing::new(&refinements, &refinements_at)),
            (&executable_blocks, &executable_edges),
            existence.as_ref().map(|run| &run.exits),
            (&driver, &external),
        );
        if !exits.is_empty() {
            return Solved::Enumerated(loops, exits);
        }
    }
    Solved::Settled(Box::new(settled_result(
        (cfg, ssa, &order),
        &driver,
        SettledRun {
            values,
            executable_blocks,
            executable_edges,
            existence,
            refinements,
            refinements_at,
        },
    )))
}

/// What a run's sweep settled on, before the post-passes read it.
struct SettledRun {
    pub(super) values: HashMap<ValueKey, LatticeValue>,
    pub(super) executable_blocks: HashSet<BlockId>,
    pub(super) executable_edges: HashSet<(BlockId, BlockId)>,
    pub(super) existence: Option<ExistenceRun>,
    pub(super) refinements: Vec<EdgeRefinement>,
    pub(super) refinements_at: HashMap<BlockId, Vec<usize>>,
}

/// A settled run's result: the constant branches and the post-passes over
/// the settled lattice, each block under the refinements in force there, and
/// every fact the run recorded.
fn settled_result(
    (cfg, ssa, order): (&CfgFunction, &SsaFunction, &[BlockId]),
    driver: &LatticeDriver<'_>,
    run: SettledRun,
) -> SccpResult {
    let SettledRun {
        mut values,
        executable_blocks,
        executable_edges,
        existence,
        refinements,
        refinements_at,
    } = run;
    let narrowing = Narrowing::new(&refinements, &refinements_at);
    let mut constant_branches = collect_constant_branches(
        cfg,
        ssa,
        (&mut values, narrowing),
        &executable_blocks,
        order,
        BranchFold { driver },
        existence.as_ref().map(|run| &run.exits),
    );
    let rung = existence.map_or_else(RungResults::default, |run| run.finish(ssa));
    // The post-passes read the settled lattice, each block under the
    // refinements in force there, before it moves in.
    let (template_plans, selections, unreached_arms) = settled_statements(
        (driver, cfg, ssa),
        (&mut values, narrowing),
        &executable_blocks,
    );
    constant_branches.extend(unreached_arms);
    let value_entries = refined_values(&refinements, &refinements_at, &values);
    SccpResult {
        template_plans,
        selections,
        values,
        executable_blocks,
        executable_edges,
        constant_branches,
        existence: rung.versions,
        existence_reads: rung.reads,
        existence_exits: rung.exits,
        existence_entries: rung.entries,
        refinements,
        refinements_at,
        value_entries,
        query_places: rung.query_places,
        existence_guards: rung.guards,
        ..driver.take_run_facts()
    }
}

/// The names whose value is never a compile-time constant.
///
/// Global / namespace / upvar-aliased / traced variables are shared mutable
/// state observable and writable from other scopes, traces, and source
/// files, so folding through one would be unsound across any opaque call
/// (`set ::g 5; mut; expr {$::g + 1}` must NOT fold to 6 — `mut` may have
/// rewritten `::g`). SCCP forces every such definition to `Overdefined` so it
/// never propagates a constant through one; the read is still tracked for
/// liveness. The set is the whole-function (flow-insensitive) view of the
/// `var_observability` alias/trace lattice, widened by any whole-module fact
/// the caller supplies (`extra_escaping`), by the whole-module
/// `traced_variables` fact — which also catches a trace installed by a
/// *called* proc, which the single-`CfgFunction` view cannot see — and by the
/// names a callback script of the module writes (`deferred_writes`), whose
/// write no statement of this function shows. A name the run owns
/// ([`ModuleRun::owned`]) is the run's own local, save one a trace or a
/// callback script may reach.
fn escaping_names(
    cfg: &CfgFunction,
    trace: &TraceInputs<'_>,
    extra_escaping: &HashSet<String>,
    owned: Option<&HashSet<String>>,
    executable: &dyn Fn(&BlockId) -> bool,
) -> HashSet<String> {
    let mut escaping = crate::var_observability::analyse_executable_var_observability(
        cfg,
        trace.registry,
        executable,
    )
    .escaping_var_names();
    if let Some(owned) = owned {
        escaping.retain(|name| !owned.contains(name));
    }
    escaping.extend(extra_escaping.iter().cloned());
    trace.extend_module_escaping(&mut escaping);
    escaping
}

/// The read-only context one solver sweep runs each block under.
struct SweepContext<'a> {
    pub(super) cfg: &'a CfgFunction,
    pub(super) ssa: &'a SsaFunction,
    preds: &'a HashMap<BlockId, HashSet<BlockId>>,
    escaping: &'a HashSet<String>,
    pub(super) has_dynamic_variable_trace: bool,
    pub(super) grammar: tcl_dialect::LexerGrammar,
    pub(super) driver: &'a LatticeDriver<'a>,
    /// Where the function's exception edges leave from.
    regions: &'a RegionShape,
    /// The run's refinements, which the refinement flow indexes.
    pub(super) refinements: &'a [EdgeRefinement],
    /// Which places another actor may write.
    external: &'a dyn Fn(&str) -> bool,
}

/// Where a function's exception edges leave from, which decides what a
/// statement that certainly raises does to its block: inside a handler's
/// region the block's normal way out is closed and the exception edge carries
/// the state the statement left, and outside one nothing is claimed.
struct RegionShape {
    /// The blocks a handler's region holds the commands of: a source of an
    /// exception edge that is no region entry.
    throwing: HashSet<BlockId>,
    /// The region entries, by the block that holds the body's first command.
    pub(super) entries: HashMap<BlockId, Vec<crate::cfg::RegionEntry>>,
    /// The region entries' edges.
    entry_edges: HashSet<(BlockId, BlockId)>,
}

impl RegionShape {
    pub(super) fn of(cfg: &CfgFunction) -> Self {
        let entry_edges: HashSet<(BlockId, BlockId)> = cfg
            .region_entries
            .iter()
            .map(|entry| (entry.source, entry.handler))
            .collect();
        let throwing = cfg
            .exception_edges
            .iter()
            .filter(|edge| !entry_edges.contains(edge))
            .map(|&(from, _)| from)
            .collect();
        let mut entries: HashMap<BlockId, Vec<crate::cfg::RegionEntry>> = HashMap::new();
        for entry in &cfg.region_entries {
            entries.entry(entry.first).or_default().push(*entry);
        }
        Self {
            throwing,
            entries,
            entry_edges,
        }
    }
}

/// What a block's statements leave of its way out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct BlockExit {
    /// Whether the block can complete normally: no statement of it is
    /// certain to raise.
    completes: bool,
    /// Whether the state before the block is one a throw can leave from:
    /// false where its first statement raises only after a store, so the
    /// handler never sees what held before it.
    entry_throws: bool,
}

impl BlockExit {
    const NORMAL: Self = Self {
        completes: true,
        entry_throws: true,
    };
}

/// What the sweeps advance: the lattice values, the executable blocks and
/// edges, and the existence rung when the run computes it.
pub(super) struct SweepState {
    pub(super) values: HashMap<ValueKey, LatticeValue>,
    pub(super) executable_blocks: HashSet<BlockId>,
    pub(super) executable_edges: HashSet<(BlockId, BlockId)>,
    pub(super) existence: Option<ExistenceRun>,
    /// The refinements outside the existence domain as the sweep reads
    /// them, when the function has one.
    refined: Option<RefinementFlow>,
}

/// The existence rung at one block during a sweep: the run, the block, and
/// — for a block some handler region holds — the join of the points passed
/// so far.
pub(super) struct ExistenceAt<'r> {
    run: &'r mut ExistenceRun,
    pub(super) block: BlockId,
    /// The join of the points passed so far after the block's entry.
    through: Option<Vec<Existence>>,
    /// The state the block entered with, for a region block: a point a
    /// throw leaves from unless the block's first statement raises only
    /// after a store.
    pub(super) entry: Option<Vec<Existence>>,
}

/// A whole-frame effect on the existence rung that no storage outcome
/// states: a barrier's, a computed name's, or a nested body's.
#[derive(Debug, Clone, Default)]
pub(super) struct Clobber {
    /// A barrier or a static-body `uplevel`: any place may have been
    /// written or destroyed, so every place is may-bound afterwards.
    all: bool,
    /// A computed name written: an unbound place may be bound afterwards.
    pub(super) writes: bool,
    /// A computed name destroyed: a bound place may be unbound afterwards.
    pub(super) destroys: bool,
    /// The places a nested body the statement keeps inline (a non-lowered
    /// `switch`'s arms) may write or destroy.
    touched: Vec<Symbol>,
}

impl Clobber {
    /// The clobber a statement's computed-name facts state.
    fn of_names(barrier: crate::dynamic_names::DynamicNameBarrier) -> Self {
        Self {
            writes: barrier.writes,
            destroys: barrier.destroys,
            ..Self::default()
        }
    }

    /// Whether the clobber changes nothing.
    pub(super) fn is_empty(&self) -> bool {
        !self.all && !self.writes && !self.destroys && self.touched.is_empty()
    }

    /// The fact a place holds after the clobber, when it held `fact`.
    pub(super) const fn applied(&self, fact: Existence) -> Existence {
        match fact {
            _ if self.all => Existence::MayBound,
            Existence::Unbound if self.writes => Existence::MayBound,
            Existence::Bound(_) if self.destroys => Existence::MayBound,
            other => other,
        }
    }
}

/// One run's existence rung (`docs/design/compiler/value-transfers.md`
/// § *Existence*): a forward fact per place, flowing through the same
/// executable blocks and edges the value lattice runs over. A block enters
/// with the join of its executable predecessors' exits — an exception edge
/// contributes every point of the region its handler covers, since any
/// command in it may throw — each statement's storage outcomes advance the
/// places it defines, and a barrier, a computed name or an inline nested
/// body clobbers what it may touch. The per-version facts, the facts each
/// statement reads, and each block's exit are the run's answer.
pub(super) struct ExistenceRun {
    /// The fact each symbol holds on entry to the function.
    pub(super) entry: Vec<Existence>,
    /// Whether each symbol's place is writable from outside the function —
    /// qualified, aliased, traced, or under a computed trace — and so
    /// may-bound wherever it is read.
    mutable: Vec<bool>,
    /// Whether each slot's place is externally mutable: a `mutable` one, or
    /// one linked to state another invocation, the object or the host
    /// holds ([`linked_elsewhere`]). Such a place is never refined, and an
    /// existence query about one decides nothing: whatever holds it can
    /// change it through any call, not only across a barrier.
    external: Vec<bool>,
    /// The clobber each statement performs, by `(block, index)`.
    clobbers: HashMap<(BlockId, usize), Clobber>,
    /// The clobber each block's terminator performs.
    terminator_clobbers: HashMap<BlockId, Clobber>,
    /// The exception edges that leave from the block before a body: they
    /// carry that block's exit, never the points inside it.
    entry_edges: HashSet<(BlockId, BlockId)>,
    /// Per handler block, the blocks whose every point its exception edges
    /// may leave from.
    handler_regions: HashMap<BlockId, Vec<BlockId>>,
    /// The blocks some handler region holds, whose inner points are kept.
    in_regions: HashSet<BlockId>,
    /// Each executable block's exit state.
    exits: HashMap<BlockId, Vec<Existence>>,
    /// The join of every point in each region block.
    through: HashMap<BlockId, Vec<Existence>>,
    /// The fact each SSA version is established with.
    versions: HashMap<ValueKey, Existence>,
    /// The fact each statement finds at each place it reads.
    reads: HashMap<(BlockId, u32, Symbol), Existence>,
    /// The slots past the SSA's symbols: each place only an existence
    /// query names, which the run carries like any other.
    query_only: HashMap<String, Symbol>,
    /// The guarded edges' refinements, until the run takes them
    /// ([`ordered_refinements`]); the sweep reads `by_edge`.
    pub(super) refinements: Vec<EdgeRefinement>,
    /// The refinements by edge: each place's slot and its refined fact.
    by_edge: HashMap<(BlockId, BlockId), Vec<(usize, Existence)>>,
    /// Each existence guard of the function: a place a guarded edge proves
    /// bound, and the block the edge enters ([`SccpResult::existence_guards`]).
    guards: Vec<(String, BlockId)>,
    /// Each executable block's entry state.
    entry_states: HashMap<BlockId, Vec<Existence>>,
}

/// What the existence rung leaves in the run's result: the fact each SSA
/// version is established with, the fact each statement finds at each place
/// it reads, each block's exit, the block-qualified facts, the places only
/// an existence query names and the existence guards. Its guarded edges'
/// refinements join the run's before the sweep ([`ordered_refinements`]).
#[derive(Default)]
pub(super) struct RungResults {
    versions: HashMap<ValueKey, Existence>,
    reads: HashMap<(BlockId, u32, Symbol), Existence>,
    exits: HashMap<BlockId, Vec<Existence>>,
    pub(super) entries: HashMap<(BlockId, ValueKey), Existence>,
    pub(super) query_places: Vec<String>,
    /// The existence guards ([`SccpResult::existence_guards`]), which the
    /// rung's run collects, so they exist only where the rung runs: the deep
    /// tier (`AnalysisTier::Deep`). A run without the rung leaves none, and
    /// W210 then has no guard's word — a tier below the deep one that keeps
    /// the dataflow diagnostics must collect them itself
    /// (`existence_guards`) or lose every `info exists` guard.
    guards: Vec<(String, BlockId)>,
}

/// A name is externally mutable (and so never a constant) when it is global /
/// namespace-qualified, escapes via alias / trace *within this function* (or
/// is traced *anywhere in the module* — a `trace add variable` installed by
/// a different proc, unioned into `escaping` by the caller; see [`sccp`]'s
/// docs), or the module installs a variable trace on a non-literal
/// (dynamic) target — in which case *every* name is potentially traced and
/// none can be trusted, mirroring
/// [`crate::gvn::is_pure_command_with_traces`]'s handling of
/// `has_dynamic_trace`.
///
/// `pub(crate)`: also consulted by [`crate::optimiser::propagation`]'s
/// def-use-chain-based load-forwarding (O102), which does not otherwise run
/// through this module's lattice and so needs the same predicate applied
/// directly.
pub(crate) fn is_externally_mutable(
    name: &str,
    escaping: &HashSet<String>,
    has_dynamic_variable_trace: bool,
) -> bool {
    has_dynamic_variable_trace || tcl_syntax::naming::is_absolute(name) || escaping.contains(name)
}

/// What [`sccp_process_statements`] reads beside the block: the block's id,
/// the names that escape the function, whether a dynamic variable trace may
/// fire, and the fresh versions each marker for a call to code the module
/// cannot see in the block gives the names live after it
/// ([`crate::ssa::SsaFunction::value_clobbers`]).
#[derive(Clone, Copy)]
pub(super) struct StatementInputs<'a> {
    pub(super) block: BlockId,
    pub(super) escaping: &'a HashSet<String>,
    pub(super) has_dynamic_variable_trace: bool,
    pub(super) clobbers: Option<&'a crate::ssa::BlockValueClobbers>,
    /// The opaque `catch` bodies the enumeration may run, when the run has
    /// them.
    pub(super) catches: Option<CatchBodies<'a>>,
}

/// The bodies of a function's opaque `catch` calls, by span, and which
/// places another actor may write, which a body the enumeration runs writes
/// none of.
#[derive(Clone, Copy)]
pub(super) struct CatchBodies<'a> {
    bodies: &'a HashMap<tcl_lexer::Span, crate::ir::Script>,
    external: &'a dyn Fn(&str) -> bool,
}

/// What a block's statements did: whether anything moved, and the block's way
/// out.
pub(super) struct StatementsRun {
    pub(super) changed: bool,
    exit: BlockExit,
}

/// Read-only inputs shared by [`sccp_process_terminator`].
pub(super) struct TerminatorInputs<'a> {
    pub(super) cfg: &'a CfgFunction,
    pub(super) ssa: &'a SsaFunction,
    pub(super) values: &'a HashMap<ValueKey, LatticeValue>,
    /// The document's lexer grammar — the branch condition's `Raw` operand
    /// texts are re-read under it when their variables are collected.
    pub(super) grammar: tcl_dialect::LexerGrammar,
    /// The run's value-transfer driver: a condition's nested commands and
    /// finite inputs are evaluated through it.
    pub(super) driver: &'a LatticeDriver<'a>,
    /// The block's way out, from its statements: a block that certainly
    /// raises opens no normal edge.
    pub(super) exit: BlockExit,
    /// The exception edges that leave from the block before a body, which
    /// open when the body's first command can fail ahead of any store.
    pub(super) entry_edges: &'a HashSet<(BlockId, BlockId)>,
}
