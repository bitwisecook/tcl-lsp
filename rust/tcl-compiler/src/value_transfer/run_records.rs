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

//! What a run records beside the lattice: explanations, folded types, preserved and raised statements, tallies and budgets.

use super::folded_type::FoldedType;
use super::{LatticeDriver, PathCompletion, PathExplanation, RouteExplanation, RouteTally};
use crate::ssa::{ValueKey, Version};
#[cfg(test)]
use crate::value_transfer::tests;
#[cfg(test)]
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use tcl_lexer::Span;
use tcl_registry::value_transfer::{
    AnalysisInputs, BindingKind, Budget, CompletionOutcome, DeclineReason, EvalRoute,
    EvaluatorOwner, ExactValue, ExistenceOutcome, InvocationOutcome, LiftedAnswer, StoreOutcome,
};

/// One solver run's request budget: [`Budget::request`], or the smaller
/// one a test sets to watch a run exhaust it.
pub(super) fn request_budget() -> Budget {
    #[cfg(test)]
    if let Some(work) = tests::REQUEST_WORK.with(Cell::get) {
        return Budget::request_of(work, Budget::REQUEST_RETAINED_BYTES);
    }
    Budget::request()
}

/// The route's spelling for an explanation.
fn route_label(route: Option<EvalRoute>) -> String {
    match route {
        None => "no semantics".to_owned(),
        Some(EvalRoute::Direct { id }) => match id.owner() {
            EvaluatorOwner::Registry => format!("direct {} (registry)", id.as_str()),
        },
        Some(EvalRoute::Expression { language }) => format!("expression {}", language.as_str()),
        Some(EvalRoute::Implementation(capability)) => {
            format!("implementation {}", capability.identity.id)
        }
        Some(EvalRoute::None { reason }) => format!("none ({})", reason.as_str()),
    }
}

/// The decline's spelling for an explanation, payload included.
pub(super) fn reason_label(reason: DeclineReason) -> String {
    match reason {
        DeclineReason::ReleaseAmbiguous(axis) => {
            format!("{}: {}", reason.as_str(), axis.as_str())
        }
        DeclineReason::NoRoute(why) => format!("{}: {}", reason.as_str(), why.as_str()),
        DeclineReason::Unavailable(tier) => format!("{}: {}", reason.as_str(), tier.as_str()),
        DeclineReason::Budget(limit) => format!("{}: {limit:?}", reason.as_str()),
        other => other.as_str().to_owned(),
    }
}

/// The completion an evaluated outcome has, for an explanation: nothing for
/// the normal one, which is what `evaluated` says, and the code or the
/// number of stores that ran before the error otherwise.
fn completion_label(completion: &CompletionOutcome) -> Option<String> {
    match PathCompletion::of(completion) {
        PathCompletion::Normal => None,
        other => Some(other.label()),
    }
}

/// One store, as an explanation spells it.
fn describe_store(name: &str, store: &StoreOutcome) -> String {
    let text = |value: &ExactValue| String::from_utf8_lossy(&value.bytes).into_owned();
    match store {
        StoreOutcome::Write { value, .. } => format!("write {name} = {}", text(value)),
        StoreOutcome::WriteElement { key, value, .. } => {
            format!("write {name}({key}) = {}", text(value))
        }
        StoreOutcome::Preserve { .. } => format!("preserve {name}"),
        StoreOutcome::Unbind { .. } => format!("unbind {name}"),
        StoreOutcome::MayWrite { .. } => format!("may-write {name}"),
        StoreOutcome::WriteUnavailable { .. } => format!("write {name} = (unavailable)"),
    }
}

/// What each outcome of an evaluated answer stores on its completion path,
/// for an explanation: the stores that ran, in execution order, a place named
/// as the invocation resolved it. Nothing is listed for a normal completion
/// that stores nothing.
pub(super) fn outcome_paths(
    outcomes: &[Box<InvocationOutcome>],
    input: Option<&dyn AnalysisInputs>,
) -> Vec<PathExplanation> {
    if outcomes.iter().all(|outcome| {
        outcome.completion.is_normal()
            && outcome.nested_writes.is_empty()
            && outcome.ordered_stores.is_empty()
    }) {
        return Vec::new();
    }
    outcomes
        .iter()
        .map(|outcome| {
            let ran = match outcome.completion {
                CompletionOutcome::Error { written, .. } => written,
                _ => usize::MAX,
            };
            let nested = outcome
                .nested_writes
                .iter()
                .map(|(place, store)| describe_store(&place.name, store));
            let ordered = outcome.ordered_stores.iter().map(|store| {
                let target = store.target().0;
                let name = input
                    .and_then(|input| input.place(target).ok())
                    .map_or_else(|| format!("operand {}", target.0), |place| place.name);
                describe_store(&name, store)
            });
            PathExplanation {
                completion: PathCompletion::of(&outcome.completion),
                stores: nested.chain(ordered).take(ran).collect(),
            }
        })
        .collect()
}

/// The paths an existence transfer lists, for an explanation: how each
/// completes and what it does to each target's existence.
pub(super) fn transfer_paths(
    transfer: &tcl_registry::value_transfer::ExistenceTransfer,
    input: &dyn AnalysisInputs,
) -> Vec<PathExplanation> {
    transfer
        .paths
        .iter()
        .map(|path| {
            let stores = path
                .outcomes
                .iter()
                .map(|(target, outcome)| {
                    let name = input
                        .place(target.0)
                        .map_or_else(|_| format!("operand {}", target.0.0), |place| place.name);
                    let kind = |kind: BindingKind| match kind {
                        BindingKind::Scalar => "scalar",
                        BindingKind::Array => "array",
                        BindingKind::Either => "either",
                    };
                    match outcome {
                        ExistenceOutcome::Bind(bound) => format!("bind {name} as {}", kind(*bound)),
                        ExistenceOutcome::Unbind => format!("unbind {name}"),
                        ExistenceOutcome::Preserve => format!("preserve {name}"),
                        ExistenceOutcome::MayBind(bound) => {
                            format!("may-bind {name} as {}", kind(*bound))
                        }
                    }
                })
                .collect();
            PathExplanation {
                completion: PathCompletion::listed(path.completion),
                stores,
            }
        })
        .collect()
}

/// The lifted answer's spelling for an explanation.
pub(super) fn answer_label(answer: &LiftedAnswer) -> String {
    match answer {
        LiftedAnswer::Pending => "pending".to_owned(),
        LiftedAnswer::Declined(reason) => format!("declined: {}", reason_label(*reason)),
        LiftedAnswer::Evaluated(outcomes) if outcomes.len() == 1 => {
            completion_label(&outcomes[0].completion).map_or_else(
                || "evaluated".to_owned(),
                |completion| format!("evaluated: {completion}"),
            )
        }
        LiftedAnswer::Evaluated(outcomes) => {
            format!("evaluated per member ({} members)", outcomes.len())
        }
    }
}

impl LatticeDriver<'_> {
    /// The statement the solver is about to evaluate, so the answers below
    /// are recorded against it; `None` between statements.
    pub(crate) fn explaining(&self, span: Option<Span>) {
        self.explaining.set(span);
    }

    /// Say whether the statements about to be evaluated sit where a throw
    /// leaves from.
    pub(crate) fn set_throwing(&self, throwing: bool) {
        self.throwing.set(throwing);
    }

    /// Whether the statement being evaluated sits where a throw leaves
    /// from.
    pub(crate) fn is_throwing(&self) -> bool {
        self.throwing.get()
    }

    /// Whether a statement that certainly raises ends its path where the
    /// statement being evaluated sits: where a throw leaves from, and
    /// anywhere in a run that reads how the procedure completes.
    pub(crate) fn raises_end_paths(&self) -> bool {
        self.throwing.get() || self.reads_exits
    }

    /// Open a solver sweep's record of the calls whose completion no
    /// re-run decided: the settled sweep's answer is the run's.
    pub(crate) fn reset_undecided_for_sweep(&self) {
        self.undecided.set(false);
    }

    /// Record that a call the run applied has a completion no re-run
    /// decided, in a run that reads how the procedure completes.
    pub(super) fn record_undecided(&self) {
        if self.reads_exits {
            self.undecided.set(true);
        }
    }

    /// Record `command`'s route and `answer` for the statement being
    /// evaluated. The last pass over a statement wins, so the record is the
    /// fixed point's.
    pub(super) fn explain(&self, command: &str, route: Option<EvalRoute>, answer: String) {
        let Some(span) = self.explaining.get() else {
            return;
        };
        self.explanations.borrow_mut().insert(
            (span.start(), span.end()),
            RouteExplanation {
                span,
                command: command.to_owned(),
                route: route_label(route),
                answer,
                paths: Vec::new(),
            },
        );
    }

    /// Attach the completion paths the statement being evaluated was found
    /// to have to the explanation recorded for it.
    pub(super) fn explain_paths(&self, paths: Vec<PathExplanation>) {
        let Some(span) = self.explaining.get() else {
            return;
        };
        if let Some(explanation) = self
            .explanations
            .borrow_mut()
            .get_mut(&(span.start(), span.end()))
        {
            explanation.paths = paths;
        }
    }

    /// Every explanation the run recorded, in statement order.
    pub(crate) fn take_explanations(&self) -> Vec<RouteExplanation> {
        std::mem::take(&mut *self.explanations.borrow_mut())
            .into_values()
            .collect()
    }

    /// Record what the latest evaluation of `key`'s statement states of its
    /// type: the solver sweeps until nothing moves, so the settled sweep's
    /// evaluation — the one over the final inputs — is the one that stays.
    /// `None` removes what an earlier sweep stated.
    pub(crate) fn record_folded(&self, key: ValueKey, folded: Option<FoldedType>) {
        let mut map = self.folded.borrow_mut();
        match folded {
            Some(folded) => {
                map.insert(key, folded);
            }
            None => {
                map.remove(&key);
            }
        }
    }

    /// The folded type recorded for `key` so far.
    pub(crate) fn folded_of(&self, key: ValueKey) -> Option<FoldedType> {
        self.folded.borrow().get(&key).cloned()
    }

    /// Forget every folded type: a barrier widens every value the run
    /// holds, and a value it widened states nothing of its type.
    pub(crate) fn forget_folded(&self) {
        self.folded.borrow_mut().clear();
    }

    /// Every folded type the run holds at its end.
    pub(crate) fn take_folded_types(&self) -> HashMap<ValueKey, FoldedType> {
        std::mem::take(&mut *self.folded.borrow_mut())
    }

    /// Record whether the latest evaluation of `key`'s statement preserved
    /// its place, and the version it preserved; the settled sweep's answer
    /// stays, as for [`Self::record_folded`]. `None` removes an earlier
    /// sweep's.
    pub(crate) fn record_preserved(&self, key: ValueKey, prior: Option<Version>) {
        let mut map = self.preserved.borrow_mut();
        match prior {
            Some(prior) => {
                map.insert(key, prior);
            }
            None => {
                map.remove(&key);
            }
        }
    }

    /// Every preserved definition the run holds at its end.
    pub(crate) fn take_preserved(&self) -> HashMap<ValueKey, Version> {
        std::mem::take(&mut *self.preserved.borrow_mut())
    }

    /// Record whether the latest evaluation of the statement at `site`, a
    /// `(block, index)`, certainly raised where a handler is thrown to; the
    /// settled sweep's answer stays, as for [`Self::record_preserved`].
    pub(crate) fn record_raised(&self, site: (crate::cfg::BlockId, usize), raised: bool) {
        let mut set = self.raised.borrow_mut();
        let mut uncaught = self.uncaught.borrow_mut();
        if raised {
            set.insert(site);
            if !self.throwing.get() {
                uncaught.insert(site);
            }
        } else {
            set.remove(&site);
            uncaught.remove(&site);
        }
    }

    /// Every statement the run's last sweep proved raises.
    pub(crate) fn take_raised(&self) -> HashSet<(crate::cfg::BlockId, usize)> {
        std::mem::take(&mut *self.raised.borrow_mut())
    }

    /// What the run recorded beside the lattice, at its end: the route
    /// explanations and tally, the folded types and the preserved
    /// definitions, in an otherwise empty result.
    pub(crate) fn take_run_facts(&self) -> crate::sccp::SccpResult {
        crate::sccp::SccpResult {
            explanations: self.take_explanations(),
            route_tally: self.take_route_tally(),
            folded_types: self.take_folded_types(),
            preserved: self.take_preserved(),
            raised: self.take_raised(),
            reads_module: self.reads_module.get(),
            completion: crate::sccp::RunCompletion {
                raises: std::mem::take(&mut *self.uncaught.borrow_mut()),
                undecided: self.undecided.get(),
            },
            ..crate::sccp::SccpResult::default()
        }
    }

    /// Count one entry into the direct-evaluator family.
    pub(super) fn enter_direct(&self) {
        let mut tally = self.tally.get();
        tally.direct += 1;
        self.tally.set(tally);
    }

    /// Count one entry into the expression-engine family.
    pub(super) fn enter_expression(&self) {
        let mut tally = self.tally.get();
        tally.expression += 1;
        self.tally.set(tally);
    }

    /// Count one entry into a declared implementation.
    pub(super) fn enter_implementation(&self) {
        let mut tally = self.tally.get();
        tally.implementation += 1;
        self.tally.set(tally);
    }

    /// The run's route-entry counts, direct, expression and implementation
    /// dispatches alike, nested entries included.
    pub(crate) fn take_route_tally(&self) -> RouteTally {
        self.tally.take()
    }

    /// Zero the tally, for the start of one full sweep over the CFG's
    /// blocks. The fixed point re-evaluates every executable statement
    /// each sweep until its answers stop changing, and once more to
    /// finalise, so counting every call would report a multiple of the
    /// true entry count; resetting at the top of each sweep keeps only the
    /// last one's, which is the settled answer's. The branch-fold pass that
    /// follows the fixed point adds to that settled count.
    pub(crate) fn reset_tally_for_sweep(&self) {
        self.tally.set(RouteTally::default());
    }

    /// Open the next solver pass: one tenth of what the request has left
    /// (`docs/design/compiler/value-evaluation.md` § *The three nested
    /// budgets*).
    pub(crate) fn open_iteration(&self) {
        let iteration = self.request.borrow_mut().iteration();
        *self.iteration.borrow_mut() = iteration;
    }

    /// The per-evaluation budget a route runs under, charging through the
    /// current pass and the run's request.
    pub(super) fn budget(&self) -> Budget {
        self.iteration.borrow().evaluation_within()
    }
}
