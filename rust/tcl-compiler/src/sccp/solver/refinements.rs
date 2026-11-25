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

//! Facts that hold on one branch edge: the existence and value refinements, their order, the flow while sweeping and the narrowing once settled.

use super::super::{EdgeRefinement, TraceInputs};
use super::existence::linked_elsewhere;
use super::frame_clobbers::place_base;
use super::statements::held_value;
use super::{ExistenceRun, is_externally_mutable};
use crate::analyses::{ConstValue, LatticeValue};
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::ir::Statement;
use crate::ssa::{SsaFunction, Symbol, ValueKey};
use crate::tcl_expr_eval::FoldPolicy;
use crate::value_transfer::LatticeDriver;
use std::collections::{BTreeSet, HashMap, HashSet};
use tcl_registry::CommandRegistry;
use tcl_registry::value_transfer::{
    BindingKind, DependencyEvidence, DomainFact, ExactValue, Existence, FactDomain, FactView,
};

/// The fact a refinement to `fact` leaves at a place that holds `current`:
/// `fact` where it narrows the place — a may-bound place to anything, a
/// place bound as either kind to one kind — and `current` otherwise. A
/// refinement the place contradicts rides an edge the query did not decide
/// although the place has a fact, as for a special variable the host binds,
/// so the place keeps its fact.
pub(in super::super) const fn narrowed(current: Existence, fact: Existence) -> Existence {
    match (current, fact) {
        (Existence::MayBound, _) | (Existence::Bound(BindingKind::Either), Existence::Bound(_)) => {
            fact
        }
        _ => current,
    }
}

/// One branch edge's fact about one place, from its condition's
/// `Selection` transfer ([`crate::tcl_expr_eval::condition_edge_facts`]).
pub(in super::super) struct BranchFact {
    /// The branch block and the block the edge enters.
    pub(in super::super) edge: (BlockId, BlockId),
    /// What the edge proves.
    pub(in super::super) fact: crate::tcl_expr_eval::PlaceFact,
}

/// What every branch edge of the function proves: the true and the false
/// edge of each branch whose targets differ, each condition read against
/// `registry` under the document's lexer configuration and the target's
/// fold policy, with the commands `driver` trusts.
pub(super) fn branch_facts(
    cfg: &CfgFunction,
    (registry, config, policy): (&CommandRegistry, tcl_lexer::LexerConfig, FoldPolicy),
    driver: &LatticeDriver<'_>,
) -> Vec<BranchFact> {
    let trusted = |head: &str| driver.trusted(head);
    let mut out = Vec::new();
    for (&block_id, block) in &cfg.blocks {
        let Some(Terminator::Branch {
            condition,
            true_target,
            false_target,
            condition_base,
            ..
        }) = &block.terminator
        else {
            continue;
        };
        if true_target == false_target {
            continue;
        }
        let reading = crate::tcl_expr_eval::ConditionReading {
            registry,
            config,
            policy,
            trusted: &trusted,
            existence: crate::existence_query::QueryReading::for_cfg(
                cfg,
                registry,
                config,
                cfg.source_tokens_at(block_id, usize::MAX),
                *condition_base,
            ),
        };
        let facts = crate::tcl_expr_eval::condition_edge_facts(condition, reading);
        for (target, facts) in [
            (*true_target, facts.on_true),
            (*false_target, facts.on_false),
        ] {
            out.extend(facts.into_iter().map(|fact| BranchFact {
                edge: (block_id, target),
                fact,
            }));
        }
    }
    out
}

/// The version of `symbol` `block` leaves: its exit version, or the entry's
/// for a place the function never defines.
fn exit_version(ssa: &SsaFunction, block: BlockId, symbol: Symbol) -> crate::ssa::Version {
    ssa.blocks
        .get(&block)
        .and_then(|ssa_block| ssa_block.exit_versions.get(&symbol))
        .copied()
        .unwrap_or(0)
}

/// The existence refinements of the function's guarded edges
/// ([`EdgeRefinement`]): each branch edge's existence fact about a place the
/// rung carries refines the place on that edge. An externally mutable place
/// (`external`, by slot) is never refined: a plain call to a procedure the
/// module cannot see, or to a computed head, may write or unset a global, an
/// alias, an instance variable or a connection's name without any barrier or
/// up-frame, so a refinement there would outlive the point at which another
/// actor acts.
pub(super) fn existence_refinements(
    facts: &[BranchFact],
    ssa: &SsaFunction,
    (query_only, external): (&HashMap<String, Symbol>, &[bool]),
) -> Vec<EdgeRefinement> {
    let mut out = Vec::new();
    for BranchFact { edge, fact } in facts {
        if fact.domain != FactDomain::Existence {
            continue;
        }
        let Some(symbol) = ssa
            .var_symbol(&fact.place)
            .or_else(|| query_only.get(&fact.place).copied())
        else {
            continue;
        };
        if external.get(symbol.0 as usize).copied().unwrap_or(true) {
            continue;
        }
        out.push(EdgeRefinement {
            edge: *edge,
            key: (symbol, exit_version(ssa, edge.0, symbol)),
            domain: fact.domain,
            fact: fact.fact.clone(),
            evidence: DependencyEvidence::default(),
            loop_exit: false,
        });
    }
    out.sort_by_key(|refinement| (refinement.edge.0.0, refinement.edge.1.0, refinement.key.0.0));
    out
}

/// Every place a guarded edge proves bound, with the block the edge enters,
/// whether or not the run refines the place: W210 takes a guard's word for
/// the region its edge enters ([`SccpResult::guarded`]).
pub(in super::super) fn existence_guards(facts: &[BranchFact]) -> Vec<(String, BlockId)> {
    facts
        .iter()
        .filter(|branch| {
            matches!(
                branch.fact.fact,
                FactView::Domain(DomainFact::Existence(Existence::Bound(_)))
            )
        })
        .map(|branch| (branch.fact.place.clone(), branch.edge.1))
        .collect()
}

/// Whether another actor may write a place, so that no refinement narrows
/// it and no enumerated loop writes it: one externally mutable
/// ([`is_externally_mutable`]) or an element of an escaping array, one
/// linked to state another invocation, the object or the host holds
/// ([`linked_elsewhere`], named where the run computes existence), and every
/// place once a computed name anywhere in the function may write or destroy
/// any.
pub(super) fn external_places<'a>(
    trace: &'a TraceInputs<'a>,
    escaping: &'a HashSet<String>,
    computed_writes: bool,
) -> impl Fn(&str) -> bool + 'a {
    let dialect = Some(tcl_registry::special_vars::surface_query_for_profile(
        trace.registry.profile(),
    ));
    move |name: &str| {
        computed_writes
            || is_externally_mutable(name, escaping, trace.has_dynamic_variable_trace)
            || escaping.contains(place_base(name))
            || trace
                .existence
                .is_some_and(|entry| linked_elsewhere(name, entry, trace.registry, dialect))
    }
}

/// Every other domain's refinements: each branch edge's fact about a
/// variable the SSA holds and that is not `external` — no other actor may
/// write it — narrowing the version the branch block leaves it at.
pub(super) fn value_refinements(
    facts: &[BranchFact],
    ssa: &SsaFunction,
    external: &dyn Fn(&str) -> bool,
) -> Vec<EdgeRefinement> {
    facts
        .iter()
        .filter(|branch| branch.fact.domain != FactDomain::Existence)
        .filter_map(|BranchFact { edge, fact }| {
            let symbol = ssa.var_symbol(&fact.place)?;
            if external(&fact.place) {
                return None;
            }
            Some(EdgeRefinement {
                edge: *edge,
                key: (symbol, exit_version(ssa, edge.0, symbol)),
                domain: fact.domain,
                fact: fact.fact.clone(),
                evidence: DependencyEvidence::default(),
                loop_exit: false,
            })
        })
        .collect()
}

/// Every refinement of the run in one order ([`refinement_order`]): the
/// existence rung's, which it hands over (its sweep reads its own by edge),
/// and every other domain's.
pub(in super::super) fn ordered_refinements(
    rung: Option<&mut ExistenceRun>,
    others: Vec<EdgeRefinement>,
) -> Vec<EdgeRefinement> {
    let mut refinements = rung.map_or_else(Vec::new, |run| std::mem::take(&mut run.refinements));
    refinements.extend(others);
    refinements.sort_by_key(refinement_order);
    refinements
}

/// The order refinements are kept in: by edge, then place, then domain.
fn refinement_order(refinement: &EdgeRefinement) -> (u32, u32, u32, u8) {
    let rank = match refinement.domain {
        FactDomain::Existence => 0,
        FactDomain::ExactValue => 1,
        FactDomain::Type => 2,
        FactDomain::Range => 3,
        _ => 4,
    };
    (
        refinement.edge.0.0,
        refinement.edge.1.0,
        refinement.key.0.0,
        rank,
    )
}

/// The refinements outside the existence domain as the solver reads them
/// while it sweeps (`docs/design/compiler/value-transfers.md` § *Predicate
/// refinement*): which each branch edge takes, which hold at each block it
/// has entered, and the blocks that hold none.
pub(in super::super) struct RefinementFlow {
    /// By edge, the indices into the run's refinements of those the edge
    /// takes.
    pub(super) by_edge: HashMap<(BlockId, BlockId), Vec<usize>>,
    /// The blocks holding a barrier or an up-frame, which may write any
    /// variable without a new version: they hold none and pass none on.
    overriding: HashSet<BlockId>,
    /// Per block the solver has entered, the refinements every executable
    /// edge into it carried when it last did.
    held: HashMap<BlockId, BTreeSet<usize>>,
}

impl RefinementFlow {
    /// The flow over `refinements`, or none when no refinement outside the
    /// existence domain exists. Refinements stating one fact about one
    /// version on several edges — the state an enumerated loop leaves, on
    /// every edge it may leave by — are one fact, so a block every one of
    /// those edges enters holds it; a loop's state and a branch's fact stay
    /// two, since they narrow differently ([`refined_value`]).
    pub(in super::super) fn new(cfg: &CfgFunction, refinements: &[EdgeRefinement]) -> Option<Self> {
        let mut by_edge: HashMap<(BlockId, BlockId), Vec<usize>> = HashMap::new();
        let mut alike: HashMap<(ValueKey, FactDomain, bool), Vec<usize>> = HashMap::new();
        for (index, refinement) in refinements.iter().enumerate() {
            if refinement.domain == FactDomain::Existence {
                continue;
            }
            let same = alike
                .entry((refinement.key, refinement.domain, refinement.loop_exit))
                .or_default();
            let earlier = same
                .iter()
                .copied()
                .find(|&earlier| refinements[earlier].fact == refinement.fact);
            let fact = earlier.unwrap_or_else(|| {
                same.push(index);
                index
            });
            by_edge.entry(refinement.edge).or_default().push(fact);
        }
        if by_edge.is_empty() {
            return None;
        }
        let overriding = cfg
            .blocks
            .iter()
            .filter(|(_, block)| {
                block.statements.iter().any(|statement| {
                    matches!(
                        statement,
                        Statement::Barrier { .. } | Statement::UpFrame { .. }
                    )
                })
            })
            .map(|(&id, _)| id)
            .collect();
        Some(Self {
            by_edge,
            overriding,
            held: HashMap::new(),
        })
    }

    /// Enter `block`: the refinements every executable edge into it carries
    /// now — a branch edge that takes one, or an edge from a block where it
    /// holds; an exception edge carries none, and a predecessor not entered
    /// yet carries every one, the greatest fixed point the edges into a
    /// cycle narrow — recorded, with whether that moved, and those in force
    /// at its statements and terminator, none where the block overrides.
    pub(in super::super) fn enter(
        &mut self,
        cfg: &CfgFunction,
        preds: &HashMap<BlockId, HashSet<BlockId>>,
        block: BlockId,
        edges: &HashSet<(BlockId, BlockId)>,
    ) -> (bool, BTreeSet<usize>) {
        let mut entering: Option<BTreeSet<usize>> = (block == cfg.entry).then(BTreeSet::new);
        for &pred in preds.get(&block).into_iter().flatten() {
            if !edges.contains(&(pred, block)) {
                continue;
            }
            let carried = if cfg.exception_edges.contains(&(pred, block)) {
                Some(BTreeSet::new())
            } else {
                self.held.get(&pred).map(|before| {
                    let mut carried = if self.overriding.contains(&pred) {
                        BTreeSet::new()
                    } else {
                        before.clone()
                    };
                    carried.extend(self.by_edge.get(&(pred, block)).into_iter().flatten());
                    carried
                })
            };
            entering = match (entering, carried) {
                (Some(entering), Some(carried)) => {
                    Some(entering.intersection(&carried).copied().collect())
                }
                (entering, carried) => entering.or(carried),
            };
        }
        let entering = entering.unwrap_or_default();
        let moved = self.held.get(&block) != Some(&entering);
        let in_force = if self.overriding.contains(&block) {
            BTreeSet::new()
        } else {
            entering.clone()
        };
        self.held.insert(block, entering);
        (moved, in_force)
    }

    /// Per block, the refinements in force there once the solver settles.
    pub(in super::super) fn settled(self) -> HashMap<BlockId, Vec<usize>> {
        let overriding = self.overriding;
        self.held
            .into_iter()
            .filter(|(block, held)| !held.is_empty() && !overriding.contains(block))
            .map(|(block, held)| (block, held.into_iter().collect()))
            .collect()
    }
}

/// The refinements outside the existence domain in force at each block once
/// the solver settles, with the run's refinements they index.
#[derive(Clone, Copy)]
pub(in super::super) struct Narrowing<'a> {
    pub(in super::super) refinements: &'a [EdgeRefinement],
    at: &'a HashMap<BlockId, Vec<usize>>,
}

impl<'a> Narrowing<'a> {
    /// The narrowing `at` names, over the run's `refinements`.
    pub(in super::super) fn new(
        refinements: &'a [EdgeRefinement],
        at: &'a HashMap<BlockId, Vec<usize>>,
    ) -> Self {
        Self { refinements, at }
    }

    /// Narrow, in `values`, each version a value-domain refinement in force
    /// at `block` names ([`narrow_values`]).
    pub(super) fn narrow(
        self,
        values: &mut HashMap<ValueKey, LatticeValue>,
        block: BlockId,
    ) -> Vec<(ValueKey, Option<LatticeValue>)> {
        narrow_values(
            values,
            self.refinements,
            self.at.get(&block).into_iter().flatten().copied(),
        )
    }
}

/// Narrow, in `values`, each version a value-domain refinement of
/// `in_force` names to what it holds where every such refinement holds
/// ([`refined_value`]); what each narrowed version held before, for
/// [`restore_values`]. A refinement never makes a version: it overrides the
/// version's value for the statements of one block, and the version's own
/// comes back after them.
pub(super) fn narrow_values(
    values: &mut HashMap<ValueKey, LatticeValue>,
    refinements: &[EdgeRefinement],
    in_force: impl IntoIterator<Item = usize>,
) -> Vec<(ValueKey, Option<LatticeValue>)> {
    let mut by_key: HashMap<ValueKey, Vec<&EdgeRefinement>> = HashMap::new();
    for refinement in in_force
        .into_iter()
        .filter_map(|index| refinements.get(index))
    {
        if refinement.domain == FactDomain::ExactValue {
            by_key.entry(refinement.key).or_default().push(refinement);
        }
    }
    let mut narrowed = Vec::new();
    for (key, facts) in by_key {
        let own = held_value(values, key);
        let refined = refined_value(&own, facts);
        if refined != own {
            narrowed.push((key, values.insert(key, refined)));
        }
    }
    narrowed
}

/// Give back each version [`narrow_values`] narrowed what it held before.
pub(super) fn restore_values(
    values: &mut HashMap<ValueKey, LatticeValue>,
    narrowed: Vec<(ValueKey, Option<LatticeValue>)>,
) {
    for (key, before) in narrowed {
        match before {
            Some(value) => {
                values.insert(key, value);
            }
            None => {
                values.remove(&key);
            }
        }
    }
}

/// The block-qualified values ([`SccpResult::value_entries`]): at each block,
/// each version a value-domain refinement in force there narrows
/// ([`refined_value`]), where the narrowed value differs from its own.
pub(super) fn refined_values(
    refinements: &[EdgeRefinement],
    refinements_at: &HashMap<BlockId, Vec<usize>>,
    values: &HashMap<ValueKey, LatticeValue>,
) -> HashMap<(BlockId, ValueKey), LatticeValue> {
    let mut out = HashMap::new();
    for (&block, indices) in refinements_at {
        let mut by_key: HashMap<ValueKey, Vec<&EdgeRefinement>> = HashMap::new();
        for refinement in indices.iter().filter_map(|&index| refinements.get(index)) {
            if refinement.domain == FactDomain::ExactValue {
                by_key.entry(refinement.key).or_default().push(refinement);
            }
        }
        for (key, facts) in by_key {
            let own = held_value(values, key);
            let refined = refined_value(&own, facts);
            if refined != own {
                out.insert((block, key), refined);
            }
        }
    }
    out
}

/// The blocks the loops whose exit state the settled values contradict leave
/// to: a version a loop's state narrows, at a block where that state is in
/// force, settled on a value the state rules out. The enumeration and the
/// lattice disagree about what that loop leaves, and neither is read; every
/// other loop's state stands.
pub(super) fn contradicted_exits(
    refinements: &[EdgeRefinement],
    refinements_at: &HashMap<BlockId, Vec<usize>>,
    values: &HashMap<ValueKey, LatticeValue>,
) -> HashSet<BlockId> {
    refinements_at
        .values()
        .flatten()
        .filter_map(|&index| refinements.get(index))
        .filter(|refinement| refinement.loop_exit && refinement.domain == FactDomain::ExactValue)
        .filter(|refinement| {
            let own = held_value(values, refinement.key);
            own != LatticeValue::Unknown
                && refined_value(&own, [*refinement]) == LatticeValue::Unknown
        })
        .map(|refinement| refinement.edge.1)
        .collect()
}

/// What a version holding `own` holds where every value-domain refinement of
/// `facts` holds: the members of its finite set the facts allow, or, for a
/// value the lattice cannot pin, the facts' own exact value or finite set.
/// A constant stays itself, as does a set the facts leave no member of — a
/// branch the solver could not decide although its operand is known —
/// and a version the solver has not reached stays the optimistic bottom.
/// The state an enumerated loop leaves holds of the version's settled value
/// exactly, so a constant or a set of its own the fact rules out is a value
/// the solver has not settled yet, and the version stays the optimistic
/// bottom until it has: the narrowed value then only descends, as the
/// solver's values do, and no branch decides on the unsettled one.
fn refined_value<'f>(
    own: &LatticeValue,
    facts: impl IntoIterator<Item = &'f EdgeRefinement>,
) -> LatticeValue {
    let mut allowed: Option<Vec<&'f ExactValue>> = None;
    let mut loop_exit = false;
    for refinement in facts {
        loop_exit |= refinement.loop_exit;
        let members: Vec<&ExactValue> = match &refinement.fact {
            FactView::Exact(value, _) => vec![value],
            FactView::Finite(values, _) => values.iter().collect(),
            _ => continue,
        };
        allowed = Some(match allowed {
            None => members,
            Some(before) => before
                .into_iter()
                .filter(|kept| members.iter().any(|member| member.bytes == kept.bytes))
                .collect(),
        });
    }
    let Some(allowed) = allowed else {
        return own.clone();
    };
    let permits = |constant: &ConstValue| {
        let text = crate::value_transfer::const_to_exact(constant).bytes;
        allowed.iter().any(|member| member.bytes == text)
    };
    match own {
        LatticeValue::ConstSet(members) => {
            let kept: Vec<ConstValue> = members.iter().filter(|c| permits(c)).cloned().collect();
            match kept.as_slice() {
                [] if loop_exit => LatticeValue::Unknown,
                [] => own.clone(),
                [one] => LatticeValue::Const(one.clone()),
                _ => LatticeValue::constset(kept),
            }
        }
        LatticeValue::Const(constant) if loop_exit && !permits(constant) => LatticeValue::Unknown,
        LatticeValue::Overdefined => match allowed.as_slice() {
            [] => LatticeValue::Overdefined,
            [one] => crate::value_transfer::exact_to_lattice(one),
            many => many
                .iter()
                .map(|member| crate::value_transfer::exact_to_const(member))
                .collect::<Option<Vec<_>>>()
                .map_or(LatticeValue::Overdefined, LatticeValue::constset),
        },
        LatticeValue::Unknown | LatticeValue::Const(_) => own.clone(),
    }
}
