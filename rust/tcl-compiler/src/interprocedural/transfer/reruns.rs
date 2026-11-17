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

//! Re-running a callee's body under what one call gives it: the seeds, the cache key and the value and places its normal exits leave.

use super::ModuleProcedures;
use super::parameter_roles::{exit_fact, normal_exits};
use crate::cfg::Function as CfgFunction;
use crate::sccp::{
    BuiltinFoldInputs, CallerPlaces, ExistenceEntry, FoldTrust, ModuleLevel, ModuleRun, SccpResult,
    SolveInputs, TraceInputs,
};
use crate::ssa::SsaFunction;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::rc::Rc;
use tcl_registry::value_transfer::{ExactValue, Existence};

/// A re-run's identity: the callee, the bytes each parameter and place is
/// seeded with and each place's existence, and whether the run takes the
/// whole-module trust. The fold policy and the dialect a re-run is solved
/// under are not in it: both are fixed for one [`ModuleProcedures`] — one
/// per unit build, one per seedless stage and one per O103 fold — so every
/// re-run it holds shares them.
pub(super) type RerunKey = (
    crate::ir::ProcedureId,
    Vec<Option<Vec<u8>>>,
    Vec<(Option<Vec<u8>>, Existence)>,
    bool,
);

/// What one call gives a re-run of its callee: each parameter's value, where
/// exact, and each `Name` link's place — its value, where exact, and its
/// existence — in [`TransferSummary::links`] order.
pub(crate) type RerunSeeds<'s> = (
    &'s [Option<ExactValue>],
    &'s [(Option<ExactValue>, Existence)],
);

struct RerunEntrySeeds {
    seed: HashMap<(String, crate::ssa::Version), crate::analyses::LatticeValue>,
    entries: HashMap<String, Existence>,
}

fn rerun_entry_seeds(
    declared_params: &[String],
    links: &[(usize, String)],
    (params, places): RerunSeeds<'_>,
) -> RerunEntrySeeds {
    let mut seed: HashMap<(String, crate::ssa::Version), crate::analyses::LatticeValue> =
        HashMap::new();
    for (name, value) in declared_params.iter().zip(params) {
        if let Some(value) = value {
            seed.insert(
                (name.clone(), 0),
                crate::value_transfer::exact_to_lattice(value),
            );
        }
    }
    let mut entries: HashMap<String, Existence> = HashMap::new();
    for ((_, local), (value, existence)) in links.iter().zip(places) {
        if let Some(value) = value {
            seed.insert(
                (local.clone(), 0),
                crate::value_transfer::exact_to_lattice(value),
            );
        }
        entries.insert(local.clone(), *existence);
    }
    RerunEntrySeeds { seed, entries }
}

/// How deep re-runs nest: a call a re-run evaluates re-runs its callee in
/// turn.
const MAX_RERUN_DEPTH: usize = 32;

/// How many re-runs one module's procedures make in all.
const MAX_RERUNS: u32 = 4096;

/// A re-run of a procedure's body under what one call gives it
/// ([`ModuleProcedures::rerun`]).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Rerun {
    /// The value every normal exit returns, where they agree.
    pub(crate) result: Option<ExactValue>,
    /// Whether a normal exit is reached: where none is, the call never
    /// completes normally.
    pub(crate) completes: bool,
    /// Whether the run decided that a call so seeded completes at those
    /// exits: every parameter and every place it was seeded with exact, no
    /// statement raising beside them and every call it applied decided in
    /// turn ([`crate::sccp::RunCompletion::is_decided`]).
    pub(crate) decided: bool,
    /// Each `Name` link's place where the normal exits leave it, in
    /// [`TransferSummary::links`] order.
    pub(crate) places: Vec<PlaceAfter>,
}

/// One caller place where a re-run's normal exits leave it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlaceAfter {
    /// Its existence, joined over the exits; `None` where no exit states
    /// it.
    pub(crate) existence: Option<Existence>,
    /// The value every exit leaves in it, where they agree.
    pub(crate) value: Option<ExactValue>,
}

/// What a re-run is solved under: its caller's value semantics, and the
/// registry, mutation facts and trust a fold is gated by.
#[derive(Clone, Copy)]
pub(crate) struct RerunStance<'s> {
    /// The value semantics.
    pub(crate) policy: crate::tcl_expr_eval::FoldPolicy,
    /// The fold inputs.
    pub(crate) folds: BuiltinFoldInputs<'s>,
}

impl<'a> ModuleProcedures<'a> {
    /// A re-run of `callee`'s body with its parameters holding `params` and
    /// the places its `Name` parameters name holding `places`, in
    /// [`TransferSummary::links`] order, under `stance`: its result,
    /// whether it completes normally, and where it leaves each place. Each
    /// re-run is made once. `None` for a callee with no summary or no flow
    /// graph to solve, one whose re-run with these seeds is already under
    /// way — a recursion its seeds do not end — and past the depth and
    /// count bounds.
    pub(crate) fn rerun(
        &self,
        callee: &crate::ir::ProcedureId,
        (params, places): RerunSeeds<'_>,
        stance: RerunStance<'_>,
    ) -> Option<Rc<Rerun>> {
        let bytes = |value: &Option<ExactValue>| value.as_ref().map(|value| value.bytes.clone());
        let key: RerunKey = (
            callee.to_owned(),
            params.iter().map(bytes).collect(),
            places
                .iter()
                .map(|(value, existence)| (bytes(value), *existence))
                .collect(),
            stance.folds.trust == FoldTrust::WholeModule,
        );
        if let Some(made) = self.reruns.borrow().get(&key) {
            return made.clone();
        }
        if self.active.borrow().contains(&key)
            || self.active.borrow().len() >= MAX_RERUN_DEPTH
            || self.spent.get() >= MAX_RERUNS
        {
            return None;
        }
        self.spent.set(self.spent.get() + 1);
        self.active.borrow_mut().push(key.clone());
        let made = self
            .solve_rerun(callee, (params, places), stance)
            .map(Rc::new);
        self.active.borrow_mut().pop();
        self.reruns.borrow_mut().insert(key, made.clone());
        made
    }

    /// [`Self::rerun`]'s run, uncached.
    fn solve_rerun(
        &self,
        callee: &crate::ir::ProcedureId,
        (params, places): RerunSeeds<'_>,
        stance: RerunStance<'_>,
    ) -> Option<Rerun> {
        let declared = self.source.as_ref()?.procedure(callee)?;
        let links = self.links(callee)?;
        let (cfg, ssa) = self.flow_of(callee)?;
        let RerunEntrySeeds { seed, entries } =
            rerun_entry_seeds(&declared.params, &links, (params, places));
        let owned: HashSet<String> = entries.keys().cloned().collect();
        let caller_places = CallerPlaces { entries };
        let trace = self.trace;
        let source_metadata_input = self.source_metadata_input_for(callee);
        if source_metadata_input.is_none() && !cfg.metadata_context.is_standalone() {
            return None;
        }
        let config = self.units.and_then(|units| units.get(callee)).map_or(
            self.config,
            crate::compilation_unit::FunctionUnit::source_lexer_config,
        );
        let callee_folds = BuiltinFoldInputs {
            source_metadata_input,
            dialect: source_metadata_input
                .map(crate::analyser::ResolvedAnalysisInput::unit_profile)
                .or_else(|| {
                    cfg.metadata_context
                        .is_standalone()
                        .then_some(stance.folds.dialect)
                        .flatten()
                }),
            proven_pure_parameters: false,
            ..stance.folds
        };
        let result = crate::sccp::sccp_in_module(&SolveInputs {
            cfg,
            ssa: &ssa,
            param_constants: Some(&seed),
            policy: stance.policy,
            extra_escaping: &HashSet::new(),
            trace: TraceInputs {
                source_metadata_input,
                registry: self.registry,
                traced_variables: trace.traced_variables,
                has_dynamic_variable_trace: trace.has_dynamic_variable_trace,
                deferred_writes: trace.deferred_writes,
                analysis_context: self.analysis_context,
                existence: Some(ExistenceEntry {
                    initial_state: crate::sccp::ExistenceInitialState::DeclaredFrame,
                    params: &declared.params,
                    object_state: None,
                    initial_global: false,
                    connection_scoped: None,
                    dynamic_trace: trace.has_dynamic_variable_trace || trace.deferred_writes.any,
                    config,
                    caller_places: Some(&caller_places),
                }),
            },
            folds: Some(callee_folds),
            module: ModuleRun {
                analysis_body: Some(crate::sccp::AnalysisBody::Procedure(*callee)),
                procedures: Some(self),
                owned: Some(&owned),
                level: ModuleLevel::Results,
                reads_exits: true,
            },
        });
        let exits = normal_exits(cfg, &result);
        let seeded_exactly = params.iter().all(Option::is_some)
            && places.iter().all(|(value, existence)| match existence {
                Existence::Unbound => true,
                Existence::Bound(_) => value.is_some(),
                Existence::Pending | Existence::MayBound => false,
            });
        let reading = super::super::ExitReading {
            policy: stance.policy,
            grammar: config.grammar_over(
                callee_folds
                    .dialect
                    .map_or_else(tcl_dialect::LexerGrammar::default, |profile| {
                        profile.grammar
                    }),
            ),
            folds: callee_folds,
            module: Some((self, callee)),
        };
        Some(Rerun {
            result: super::super::exit_value(
                super::super::ExitBody { cfg, ssa: &ssa },
                &result,
                reading,
            ),
            completes: !exits.is_empty(),
            decided: seeded_exactly && result.completion.is_decided(),
            places: links
                .iter()
                .map(|(_, local)| PlaceAfter {
                    existence: exit_fact(&ssa, &result, &exits, local),
                    value: exit_constant(cfg, &ssa, &result, &exits, local),
                })
                .collect(),
        })
    }

    /// `callee`'s flow graph and SSA: the built unit's where there is one,
    /// else the module's flow graph with its SSA built once. `None` for a
    /// body the complexity guard stops.
    pub(super) fn flow_of(
        &self,
        callee: &crate::ir::ProcedureId,
    ) -> Option<(&'a CfgFunction, Rc<SsaFunction>)> {
        let declared = self.procedures.get(callee)?;
        let declared_frame = self.cfg.declared_frame_procedures.get(callee);
        let unit = self
            .units
            .and_then(|units| units.get(callee))
            .filter(|_| declared_frame.is_none());
        let cfg = match (declared_frame, unit) {
            (Some(cfg), _) => cfg,
            (None, Some(unit)) => &unit.cfg,
            (None, None) => self.cfg.procedures.get(callee)?,
        };
        if crate::ssa::is_complexity_guarded(cfg)
            || unit.is_some_and(|unit| unit.complexity_guarded)
        {
            return None;
        }
        let ssa = self
            .ssa
            .borrow_mut()
            .entry(callee.to_owned())
            .or_insert_with(|| {
                Rc::new(unit.map_or_else(
                    || {
                        crate::ssa::build_ssa_with_context_for_entry_and_metadata(
                            cfg,
                            self.registry,
                            self.config,
                            crate::compilation_unit::function_source_entry(cfg, None),
                            Some(&declared.params),
                            crate::registry_invocation::InvocationMetadataContext::for_module(
                                self.registry,
                                self.ir,
                            ),
                            cfg.metadata_context.is_standalone(),
                        )
                    },
                    |unit| unit.ssa.clone(),
                ))
            })
            .clone();
        Some((cfg, ssa))
    }
}

/// The constant `local` holds at every one of a run's normal `exits`: the
/// value of each version that reaches an exit along the executable edges,
/// where every one agrees and may be written into source.
fn exit_constant(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    result: &SccpResult,
    exits: &[crate::cfg::BlockId],
    local: &str,
) -> Option<ExactValue> {
    let symbol = ssa.var_symbol(local)?;
    let mut found: Option<ExactValue> = None;
    for &exit in exits {
        for version in reaching_versions(cfg, ssa, result, exit, symbol) {
            let key = (symbol, version);
            let Some(crate::analyses::LatticeValue::Const(value)) = result.value_at(exit, key)
            else {
                return None;
            };
            if !result.materialises(key) {
                return None;
            }
            let value = crate::value_transfer::const_to_exact(value);
            match &found {
                Some(held) if held.bytes != value.bytes => return None,
                Some(_) => {}
                None => found = Some(value),
            }
        }
    }
    found
}

/// The versions of `symbol` that reach the end of `block`: the block's own
/// last definition or φ, else each executable predecessor's, back to the
/// version the function enters with.
fn reaching_versions(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    result: &SccpResult,
    block: crate::cfg::BlockId,
    symbol: crate::ssa::Symbol,
) -> BTreeSet<crate::ssa::Version> {
    let preds = cfg.predecessors();
    let mut versions = BTreeSet::new();
    let mut seen: HashSet<crate::cfg::BlockId> = HashSet::new();
    let mut work = vec![block];
    while let Some(at) = work.pop() {
        if !seen.insert(at) {
            continue;
        }
        let own = ssa.blocks.get(&at).and_then(|ssa_block| {
            ssa_block
                .statements
                .iter()
                .rev()
                .find_map(|statement| statement.defs.get(&symbol).copied())
                .or_else(|| {
                    ssa_block
                        .phis
                        .iter()
                        .find(|phi| phi.name == symbol)
                        .map(|phi| phi.version)
                })
        });
        if let Some(version) = own {
            versions.insert(version);
            continue;
        }
        let incoming: Vec<crate::cfg::BlockId> = preds
            .get(&at)
            .into_iter()
            .flatten()
            .copied()
            .filter(|pred| result.executable_edges.contains(&(*pred, at)))
            .collect();
        if incoming.is_empty() {
            versions.insert(0);
        }
        work.extend(incoming);
    }
    versions
}
