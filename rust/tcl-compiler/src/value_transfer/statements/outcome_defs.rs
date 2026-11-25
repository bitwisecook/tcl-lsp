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

//! The lattice values a resolved invocation's outcomes leave in a statement's definitions: placed stores, raised paths and existence transfers.

use super::super::existence::{existence_from, store_existence};
use super::super::folded_type::FoldedType;
use super::super::lattice_inputs::exact_to_lattice;
use super::super::run_records::{answer_label, outcome_paths, reason_label, transfer_paths};
use super::super::{
    DefAnswer, ExistenceStep, LatticeDriver, PathCompletion, completes_normally, element_place,
    fact_to_lattice, place_named, widened,
};
use crate::analyses::{LatticeValue, MAX_CONSTSET_SIZE};
use crate::sccp::{DefValues, RaisedDefs};
use crate::ssa::ValueKey;
use tcl_registry::value_transfer::{
    AnalysisInputs, CommandSemantics, CompletionOutcome, DeclineReason, EvalRoute, EvaluatorOwner,
    FactDomain, InvocationOutcome, LiftedAnswer, OperandId, PlaceRef, StoreOutcome, TargetId,
    TransferAnswer, evaluate_lifted, validate_outcome,
};

impl LatticeDriver<'_> {
    /// The lattice values a resolved invocation leaves in `defs`, the
    /// variables its statement defines: the registry's evaluator over the
    /// lattice inputs, lifted over one finite input, on the route the
    /// registry owns; each outcome validated against the invocation's
    /// targets ([`validate_outcome`]) and applied per place
    /// ([`Self::apply_outcome`]), and the members joined per definition.
    pub(super) fn call_defs(
        &self,
        head: &str,
        semantics: &dyn tcl_registry::value_transfer::CommandSemantics,
        defs: &[(String, ValueKey)],
        inputs: &dyn AnalysisInputs,
    ) -> DefValues {
        let widen = || DefValues::PerDef(self.transferred(semantics, defs, inputs, widened(defs)));
        let route = semantics.route();
        match route {
            EvalRoute::Direct { id } if id.owner() == EvaluatorOwner::Registry => {
                self.enter_direct();
            }
            // A declared implementation is registry-owned too: the
            // specialisation's `evaluate` runs it in the bounded host, on
            // the bindings it declares it rests on.
            EvalRoute::Implementation(capability) => {
                if self.explains_an_unheld_binding(head, route, &capability) {
                    return widen();
                }
                self.enter_implementation();
            }
            _ => {
                self.explain(
                    head,
                    Some(route),
                    "not evaluated: the route is not registry-owned".to_owned(),
                );
                return widen();
            }
        }
        self.word_error.set(false);
        let answer = evaluate_lifted(semantics, inputs, &mut self.budget(), MAX_CONSTSET_SIZE);
        self.explain(head, Some(route), answer_label(&answer));
        if let LiftedAnswer::Evaluated(outcomes) = &answer {
            self.explain_paths(outcome_paths(outcomes, Some(inputs)));
        }
        self.explain_transfer_paths(semantics, inputs);
        // A word that raises is an error before the command runs, whatever
        // the route would answer from the words it reads: nothing it would
        // store is stored, and the state before it is the one a handler is
        // thrown to.
        if self.word_error.replace(false) {
            if self.raises_end_paths() {
                return DefValues::Raised(Box::new(RaisedDefs {
                    prefix: None,
                    written: 0,
                }));
            }
            return widen();
        }
        let outcomes = match answer {
            LiftedAnswer::Pending => {
                let pending = defs
                    .iter()
                    .map(|(_, key)| DefAnswer {
                        existence: ExistenceStep::PENDING,
                        ..DefAnswer::untyped(*key, LatticeValue::Unknown)
                    })
                    .collect();
                return DefValues::PerDef(self.transferred(semantics, defs, inputs, pending));
            }
            LiftedAnswer::Declined(_) => return widen(),
            LiftedAnswer::Evaluated(outcomes) => outcomes,
        };
        let plan = semantics.structure(inputs);
        let targets = semantics.store_targets(inputs);
        if let Some(reason) = outcomes
            .iter()
            .find_map(|outcome| validate_outcome(&plan, &targets, outcome).err())
        {
            self.explain(
                head,
                Some(route),
                format!("declined: {}", reason_label(reason)),
            );
            return widen();
        }
        // A completion that is not the normal one publishes no value for the
        // statement's definitions: where a raise ends the path, they take what
        // the stores that ran left, and elsewhere they widen. The explanation
        // already says what the route proved.
        if outcomes
            .iter()
            .any(|outcome| !outcome.completion.is_normal())
        {
            if self.raises_end_paths()
                && let Some(raised) = self.raised_defs(&outcomes, defs, inputs)
            {
                return DefValues::Raised(Box::new(raised));
            }
            return widen();
        }
        let mut joined: Option<Vec<DefAnswer>> = None;
        for outcome in &outcomes {
            let applied = match self.apply_outcome(outcome, defs, inputs) {
                Ok(applied) => applied,
                Err(reason) => {
                    self.explain(
                        head,
                        Some(route),
                        format!("declined: {}", reason_label(reason)),
                    );
                    return widen();
                }
            };
            joined = Some(match joined {
                None => applied,
                Some(earlier) => earlier
                    .into_iter()
                    .zip(&applied)
                    .map(|(left, right)| left.join(right))
                    .collect(),
            });
        }
        DefValues::PerDef(joined.unwrap_or_else(|| widened(defs)))
    }

    /// The statement's definitions when every outcome is an error: each
    /// after the stores that ran before it, the places the rest name left as
    /// they were, joined over the members of a finite input. `None` where an
    /// outcome is another completion, carries writes of its words, or
    /// stores to places the evaluation cannot place — the statement then
    /// widens as before.
    fn raised_defs(
        &self,
        outcomes: &[Box<InvocationOutcome>],
        defs: &[(String, ValueKey)],
        input: &dyn AnalysisInputs,
    ) -> Option<RaisedDefs> {
        let mut joined: Option<Vec<DefAnswer>> = None;
        let mut written = usize::MAX;
        for outcome in outcomes {
            let CompletionOutcome::Error { written: ran, .. } = outcome.completion else {
                return None;
            };
            if !outcome.nested_writes.is_empty() {
                return None;
            }
            let placed = self.placed_stores(outcome, input).ok()?;
            let ran_stores = placed.get(..ran)?;
            // A place the stores that ran did not reach keeps what it held,
            // an element of an untouched array included.
            let untouched = StoreOutcome::Preserve {
                target: TargetId(OperandId(0)),
            };
            let mut prefix: Vec<(PlaceRef, &StoreOutcome)> = ran_stores.to_vec();
            for (name, _) in defs {
                let place = place_named(name);
                if !ran_stores
                    .iter()
                    .any(|(seen, _)| seen.shares_storage_with(&place))
                {
                    prefix.push((place, &untouched));
                }
            }
            let answers = self.defs_from_placed(&prefix, Some(outcome), defs, input);
            written = written.min(ran);
            joined = Some(match joined {
                None => answers,
                Some(earlier) => earlier
                    .into_iter()
                    .zip(&answers)
                    .map(|(left, right)| left.join(right))
                    .collect(),
            });
        }
        Some(RaisedDefs {
            prefix: Some(joined?),
            written,
        })
    }

    /// Add to the statement's explanation the paths the declaration's
    /// existence transfer lists, when it lists an error path beside the
    /// normal one, for each kind of completion the evaluation did not
    /// already report: a kind the analysis cannot place the failing step of
    /// is still a path the statement may take.
    fn explain_transfer_paths(
        &self,
        semantics: &dyn CommandSemantics,
        inputs: &dyn AnalysisInputs,
    ) {
        let TransferAnswer::Existence(transfer) =
            semantics.transfer(FactDomain::Existence, inputs, &mut self.budget())
        else {
            return;
        };
        if transfer.paths.len() < 2 {
            return;
        }
        let Some(span) = self.explaining.get() else {
            return;
        };
        let mut explanations = self.explanations.borrow_mut();
        let Some(explanation) = explanations.get_mut(&(span.start(), span.end())) else {
            return;
        };
        for path in transfer_paths(&transfer, inputs) {
            let reported = explanation.paths.iter().any(|known| {
                known.completion == path.completion
                    || (path.completion == PathCompletion::Error(None)
                        && known.completion.is_error())
            });
            if !reported {
                explanation.paths.push(path);
            }
        }
    }

    /// `answers` with the existence each definition takes from the
    /// declaration's own existence transfer, when it states one: the
    /// normal completion's outcome for each target's place
    /// ([`existence_from`]). A definition the transfer does not name, and
    /// every definition of a command whose transfer is generic, keeps the
    /// step its answer carries — a pending evaluation's, or the generic
    /// widening.
    fn transferred(
        &self,
        semantics: &dyn CommandSemantics,
        defs: &[(String, ValueKey)],
        inputs: &dyn AnalysisInputs,
        mut answers: Vec<DefAnswer>,
    ) -> Vec<DefAnswer> {
        let TransferAnswer::Existence(transfer) =
            semantics.transfer(FactDomain::Existence, inputs, &mut self.budget())
        else {
            return answers;
        };
        let Some(normal) = transfer
            .paths
            .iter()
            .find(|path| completes_normally(path.completion))
        else {
            return answers;
        };
        let steps: Vec<(PlaceRef, ExistenceStep)> = normal
            .outcomes
            .iter()
            .filter_map(|(target, outcome)| {
                inputs
                    .place(target.0)
                    .ok()
                    .map(|place| (place, ExistenceStep::of(*outcome)))
            })
            .collect();
        for (answer, (name, _)) in answers.iter_mut().zip(defs) {
            if let Some(step) = existence_from(&steps, name) {
                answer.existence = step;
            }
        }
        answers
    }

    /// The value one outcome leaves in each of `defs`, the statement's
    /// definitions (`docs/design/compiler/value-transfers.md` § *Storage-
    /// writing commands*). Every store's target resolves to its place
    /// first — an element write's to the element of its target's array —
    /// so two spellings of one cell are one place and a repeated target
    /// composes in execution order: a `Write` is its value, the last write
    /// winning; a `Preserve` keeps what the place holds at that
    /// point, the prior version's value when nothing earlier wrote it (a
    /// pending prior stays pending); a `MayWrite` and an `Unbind` widen,
    /// their facts being the type domain's and the existence rung's. A
    /// definition no store names widens.
    ///
    /// # Errors
    ///
    /// The whole answer declines, and every definition widens, for a
    /// completion other than the normal one (the stores made before an error
    /// are not applied); a target that is no place (`DynamicName`, `EscapingPlace`
    /// from the resolver); an element write beside its array's base write
    /// (`OverlappingTargets`); and a traced place (`TracedPlace`), whose
    /// trace runs on the write and can observe or rewrite the others.
    pub(super) fn apply_outcome(
        &self,
        outcome: &InvocationOutcome,
        defs: &[(String, ValueKey)],
        input: &dyn AnalysisInputs,
    ) -> Result<Vec<DefAnswer>, DeclineReason> {
        if outcome.completion != CompletionOutcome::Normal {
            return Err(DeclineReason::Unsupported);
        }
        let placed = self.placed_stores(outcome, input)?;
        Ok(self.defs_from_placed(&placed, Some(outcome), defs, input))
    }

    /// The value each of `defs` takes from `placed`, the stores an outcome
    /// makes in execution order with the place each names: the last write to
    /// a place wins, a preserve keeps what the place holds at that point, a
    /// may-write or an unbind widens, and a definition no store names
    /// widens. `outcome` supplies the folded type its stores state, when the
    /// stores are its own.
    pub(super) fn defs_from_placed(
        &self,
        placed: &[(PlaceRef, &StoreOutcome)],
        outcome: Option<&InvocationOutcome>,
        defs: &[(String, ValueKey)],
        input: &dyn AnalysisInputs,
    ) -> Vec<DefAnswer> {
        let steps: Vec<(PlaceRef, ExistenceStep)> = placed
            .iter()
            .map(|(place, store)| (place.clone(), store_existence(store)))
            .collect();
        defs.iter()
            .map(|(name, key)| {
                let existence = existence_from(&steps, name).unwrap_or(ExistenceStep::UNKNOWN);
                let named: Vec<&(PlaceRef, &StoreOutcome)> = placed
                    .iter()
                    .filter(|(place, _)| place.name == *name)
                    .collect();
                let Some((place, _)) = named.first() else {
                    return DefAnswer {
                        existence,
                        ..DefAnswer::untyped(*key, LatticeValue::Overdefined)
                    };
                };
                if self.is_escaping(place) {
                    return DefAnswer {
                        existence,
                        ..DefAnswer::untyped(*key, LatticeValue::Overdefined)
                    };
                }
                let mut held: Option<LatticeValue> = None;
                for (_, store) in &named {
                    match store {
                        StoreOutcome::Write { value, .. }
                        | StoreOutcome::WriteElement { value, .. } => {
                            held = Some(exact_to_lattice(value));
                        }
                        StoreOutcome::Preserve { .. } => {}
                        StoreOutcome::Unbind { .. }
                        | StoreOutcome::MayWrite { .. }
                        | StoreOutcome::WriteUnavailable { .. } => {
                            held = Some(LatticeValue::Overdefined);
                        }
                    }
                }
                let value = held.unwrap_or_else(|| {
                    fact_to_lattice(&input.prior_store(place, FactDomain::ExactValue))
                });
                DefAnswer {
                    key: *key,
                    value,
                    folded: outcome.and_then(|outcome| {
                        FoldedType::after_stores(outcome, named.iter().map(|(_, store)| *store))
                    }),
                    stated: true,
                    preserved: named
                        .iter()
                        .all(|(_, store)| matches!(store, StoreOutcome::Preserve { .. })),
                    existence,
                }
            })
            .collect()
    }

    /// Each of an outcome's ordered stores with the place it names, in
    /// execution order: an element write's target resolves to the element
    /// of its array, so two spellings of one cell are one place.
    ///
    /// # Errors
    ///
    /// A target that is no place (`DynamicName`, `EscapingPlace` from the
    /// resolver); an element write beside its array's base write
    /// (`OverlappingTargets`); and a traced place (`TracedPlace`), whose
    /// trace runs on the write and can observe or rewrite the others.
    pub(in super::super) fn placed_stores<'o>(
        &self,
        outcome: &'o InvocationOutcome,
        input: &dyn AnalysisInputs,
    ) -> Result<Vec<(PlaceRef, &'o StoreOutcome)>, DeclineReason> {
        let mut placed: Vec<(PlaceRef, &StoreOutcome)> =
            Vec::with_capacity(outcome.ordered_stores.len());
        for store in &outcome.ordered_stores {
            let place = match store {
                StoreOutcome::WriteElement { target, key, .. } => {
                    element_place(&input.place(target.0)?, key)?
                }
                _ => input.place(store.target().0)?,
            };
            if placed
                .iter()
                .any(|(seen, _)| seen.overlaps_as_element_and_base(&place))
            {
                return Err(DeclineReason::OverlappingTargets);
            }
            if self.may_be_traced(&place) {
                return Err(DeclineReason::TracedPlace);
            }
            placed.push((place, store));
        }
        Ok(placed)
    }

    /// Whether the ordered evaluation state can own a write to `place`:
    /// local to the frame, so no other frame or trace can write or observe
    /// it while the evaluation runs — not traced, not escaping, and not an
    /// element of an array that is.
    pub(in super::super) fn state_owns(&self, place: &PlaceRef) -> bool {
        !self.may_be_traced(place)
            && !self.is_escaping(place)
            && !self.is_escaping(&PlaceRef::scalar(place.base()))
    }

    /// Whether the trace owner says a trace may observe `place` or its base.
    fn may_be_traced(&self, place: &PlaceRef) -> bool {
        [place.name.as_str(), place.base()].into_iter().any(|name| {
            crate::command_binding::trace_registrations::variable_may_be_traced(
                &self.context.traced_variables,
                self.context.has_dynamic_variable_trace,
                name,
            )
        })
    }

    /// Whether `place` is writable from outside the function — the
    /// solver's own rule ([`crate::sccp::is_externally_mutable`]), so a
    /// definition it widens before any transfer runs is widened here too.
    fn is_escaping(&self, place: &PlaceRef) -> bool {
        let escaping = &self.context.escaping;
        self.context.has_dynamic_variable_trace
            || tcl_syntax::naming::is_absolute(&place.name)
            || escaping.contains(&place.name)
    }
}
