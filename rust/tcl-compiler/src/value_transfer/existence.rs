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

//! The existence rung: the per-place existence state at the current point, the steps stores and write classes take, and `info exists` queries.

use super::argument_words::call_arguments;
use super::{ExistenceStep, LatticeDriver};
use crate::ir::Statement;
use crate::ssa::{SsaFunction, Symbol};
use std::cell::RefCell;
use std::collections::HashMap;
use tcl_registry::value_transfer::{
    BindingKind, CompletionOutcome, DeclineReason, DependencyEvidence, DomainFact, ExactValue,
    ExactValueOrUnavailable, Existence, FactView, InvocationOutcome, LiftedAnswer, PlaceRef,
    StoreOutcome, TypeFacts, WrittenPlace, written_in,
};
use tcl_registry::{ArgRole, InvocationWord, TclType};

/// The existence step one store takes on its place: a write binds a
/// scalar (an element is one), an unbind unbinds, a preserve keeps the
/// fact, and a may-write joins it with the kind its bounds state.
pub(super) fn store_existence(store: &StoreOutcome) -> ExistenceStep {
    match store {
        StoreOutcome::Write { .. } | StoreOutcome::WriteElement { .. } => {
            ExistenceStep::Set(Existence::Bound(BindingKind::Scalar))
        }
        StoreOutcome::WriteUnavailable { facts, .. } => {
            ExistenceStep::Set(Existence::Bound(match facts.existence {
                Existence::Bound(kind) => kind,
                Existence::Pending | Existence::Unbound | Existence::MayBound => {
                    BindingKind::Either
                }
            }))
        }
        StoreOutcome::Preserve { .. } => ExistenceStep::PRESERVE,
        StoreOutcome::Unbind { .. } => ExistenceStep::Set(Existence::Unbound),
        StoreOutcome::MayWrite { facts, .. } => ExistenceStep::Join(Existence::Bound(match facts
            .existence
        {
            Existence::Bound(kind) => kind,
            Existence::Pending | Existence::Unbound | Existence::MayBound => BindingKind::Either,
        })),
    }
}

/// The existence step a definition of `name` takes from an outcome's
/// ordered steps per place: its own place's steps in execution order; for
/// the array that holds an element the outcome writes, unbinds or may
/// write, an array binding (an element exists only in an array, and
/// unsetting one leaves the array); for an element of an array the outcome
/// unbinds whole, an unbinding. `None` when no step names the definition.
pub(super) fn existence_from(
    steps: &[(PlaceRef, ExistenceStep)],
    name: &str,
) -> Option<ExistenceStep> {
    let own = steps
        .iter()
        .filter(|(place, _)| place.name == name)
        .map(|(_, step)| *step)
        .reduce(ExistenceStep::then);
    if own.is_some() {
        return own;
    }
    let array = steps
        .iter()
        .filter(|(place, _)| place.is_element() && place.base() == name)
        .filter_map(|(_, step)| match step {
            ExistenceStep::Set(_) => Some(ExistenceStep::Set(Existence::Bound(BindingKind::Array))),
            ExistenceStep::Join(Existence::Pending) => None,
            ExistenceStep::Join(_) => {
                Some(ExistenceStep::Join(Existence::Bound(BindingKind::Array)))
            }
        })
        .reduce(ExistenceStep::then);
    if array.is_some() {
        return array;
    }
    let (base, _) = tcl_syntax::naming::split_element_ref(name)?;
    steps
        .iter()
        .any(|(place, step)| place.name == base && *step == ExistenceStep::Set(Existence::Unbound))
        .then_some(ExistenceStep::Set(Existence::Unbound))
}

impl LatticeDriver<'_> {
    /// Position the existence rung at a block's entry: `state` holds one
    /// fact per symbol, and the statements the solver evaluates next read
    /// and advance it.
    pub(crate) fn existence_enter(&self, state: Vec<Existence>) {
        *self.existence.borrow_mut() = Some(state);
    }

    /// The existence state at the current point, taken: the block's exit
    /// once its statements have run. `None` when the run computes none.
    pub(crate) fn existence_leave(&self) -> Option<Vec<Existence>> {
        self.existence.borrow_mut().take()
    }

    /// The existence fact `symbol` holds at the current point, when the
    /// run computes existence.
    pub(crate) fn existence_now(&self, symbol: Symbol) -> Option<Existence> {
        self.existence
            .borrow()
            .as_ref()
            .and_then(|state| state.get(symbol.0 as usize).copied())
    }

    /// The existence state at the current point, copied, when the run
    /// computes existence.
    pub(crate) fn existence_state(&self) -> Option<Vec<Existence>> {
        self.existence.borrow().clone()
    }

    /// The existence state the scripts of the statement about to be
    /// evaluated start from ([`LatticeDriver::body_entry`]), held while the
    /// returned scope lives: dropping it clears the state, so no read after
    /// the statement answers from it.
    pub(crate) fn body_entry_scope(&self, state: Option<Vec<Existence>>) -> BodyEntryScope<'_> {
        *self.body_entry.borrow_mut() = state;
        BodyEntryScope {
            entry: &self.body_entry,
        }
    }

    /// Advance `symbol`'s fact at the current point by `step`, returning
    /// the fact afterwards.
    pub(crate) fn existence_step(&self, symbol: Symbol, step: ExistenceStep) -> Option<Existence> {
        let mut state = self.existence.borrow_mut();
        let fact = state.as_mut()?.get_mut(symbol.0 as usize)?;
        *fact = step.apply(*fact);
        Some(*fact)
    }

    /// Replace every fact at the current point by `clobber`'s answer for
    /// it: a barrier's, or a computed name's.
    pub(crate) fn existence_clobber(&self, clobber: impl Fn(Existence) -> Existence) {
        if let Some(state) = self.existence.borrow_mut().as_mut() {
            for fact in state.iter_mut() {
                *fact = clobber(*fact);
            }
        }
    }

    /// The existence fact the place `name` holds at the current point, as
    /// an input view: `Unavailable` when the run computes none, or the name
    /// is no symbol of the function. Where the statement's scripts start from
    /// another state ([`LatticeDriver::body_entry`]) the fact is that state's.
    pub(super) fn existence_fact(&self, ssa: &SsaFunction, name: &str) -> FactView {
        ssa.var_symbol(name)
            .or_else(|| self.existence_places.borrow().get(name).copied())
            .and_then(|symbol| match self.body_entry.borrow().as_ref() {
                Some(state) => state.get(symbol.0 as usize).copied(),
                None => self.existence_now(symbol),
            })
            .map_or(
                FactView::Top(DeclineReason::Unavailable(self.context.tier)),
                |fact| FactView::Domain(DomainFact::Existence(fact)),
            )
    }

    /// Hand the driver the rung's slots past the SSA's symbols: each place
    /// only an existence query names, and the slot the run keeps it in.
    pub(crate) fn existence_places(&self, places: HashMap<String, Symbol>) {
        *self.existence_places.borrow_mut() = places;
    }

    /// Hand the driver, per slot, whether the rung holds the place
    /// externally mutable.
    pub(crate) fn existence_external(&self, external: Vec<bool>) {
        *self.existence_external.borrow_mut() = external;
    }

    /// Whether the place `name` is one the rung holds externally mutable: a
    /// call to a procedure the module cannot see may bind or unset it, so
    /// no fact about it is proven at a point.
    fn existence_is_external(&self, ssa: &SsaFunction, name: &str) -> bool {
        ssa.var_symbol(name)
            .or_else(|| self.existence_places.borrow().get(name).copied())
            .is_some_and(|symbol| {
                self.existence_external
                    .borrow()
                    .get(symbol.0 as usize)
                    .copied()
                    .unwrap_or(false)
            })
    }

    /// The existence query `kind` over its source words: a literal
    /// name reads its place, and an element name its array; a computed key
    /// leaves a bareword array fixed, so `Params($k)` asks about `Params` as
    /// a literal element does. Any other computed name decides nothing.
    pub(super) fn existence_answer(
        &self,
        kind: crate::existence_query::ExistenceKind,
        (words, texts): (&[InvocationWord<'_>], &[&str]),
        (ssa, prior): (&SsaFunction, &[(PlaceRef, StoreOutcome)]),
    ) -> LiftedAnswer {
        match (words, texts) {
            ([_, InvocationWord::Literal(name)], _) => {
                let base = crate::sccp::place_base(name);
                self.existence_of(kind, base, base != *name, (ssa, prior))
            }
            ([_, InvocationWord::Dynamic], [_, text]) => {
                crate::existence_query::computed_element_base(text)
                    .map_or(LiftedAnswer::Declined(DeclineReason::NotExact), |base| {
                        self.existence_of(kind, base, true, (ssa, prior))
                    })
            }
            _ => LiftedAnswer::Declined(DeclineReason::NotExact),
        }
    }

    /// `info exists NAME` / `array exists NAME` over the existence rung at
    /// the current point, for the place `base` — an `element` query
    /// names its array: a bound place exists, one bound as an array is an
    /// array, one bound as a scalar is no array, and an unbound one is
    /// neither; an element exists only in an array, so an unbound array
    /// holds none, whatever the key. Anything else decides nothing —
    /// `MayBound`, `Bound(Either)` for `array exists`, an element of an
    /// array that may exist, a run with no rung (`Unavailable`, never read
    /// as unbound), a special variable in the initial global frame, which
    /// the host rather than the script binds, and any other externally
    /// mutable place, which a call the module cannot see may bind
    /// or unset.
    fn existence_of(
        &self,
        kind: crate::existence_query::ExistenceKind,
        base: &str,
        element: bool,
        (ssa, prior): (&SsaFunction, &[(PlaceRef, StoreOutcome)]),
    ) -> LiftedAnswer {
        use crate::existence_query::ExistenceKind;
        // An evaluation that has already written the place, or its array,
        // has moved it off the rung's point: the query decides nothing.
        if !matches!(written_in(prior, base), WrittenPlace::Untouched) {
            return LiftedAnswer::Declined(DeclineReason::StatefulNested);
        }
        let host_bound = self.existence_initial_global
            && self
                .registry
                .special_var_in_dialect(
                    base,
                    Some(tcl_registry::special_vars::surface_query_for_profile(
                        self.registry.profile(),
                    )),
                )
                .is_some();
        if host_bound || self.existence_is_external(ssa, base) {
            return LiftedAnswer::Declined(DeclineReason::EscapingPlace);
        }
        let fact = match self.existence_fact(ssa, base) {
            FactView::Domain(DomainFact::Existence(fact)) => fact,
            FactView::Top(reason) => return LiftedAnswer::Declined(reason),
            _ => return LiftedAnswer::Declined(DeclineReason::Unsupported),
        };
        let exists = match (kind, element, fact) {
            (_, _, Existence::Pending) => return LiftedAnswer::Pending,
            (_, _, Existence::Unbound)
            | (ExistenceKind::Array, false, Existence::Bound(BindingKind::Scalar)) => false,
            (ExistenceKind::AnyVariable, false, Existence::Bound(_))
            | (ExistenceKind::Array, false, Existence::Bound(BindingKind::Array)) => true,
            _ => return LiftedAnswer::Declined(DeclineReason::Unsupported),
        };
        LiftedAnswer::Evaluated(vec![Box::new(InvocationOutcome {
            completion: CompletionOutcome::Normal,
            nested_writes: Vec::new(),
            result: ExactValueOrUnavailable::Exact(ExactValue::int(i64::from(exists))),
            ordered_stores: Vec::new(),
            types: TypeFacts {
                result: Some(TclType::Boolean),
                per_target: Vec::new(),
                shapes: Vec::new(),
            },
            evidence: DependencyEvidence::default(),
        })])
    }

    /// The existence step the registry's write class gives a call's
    /// definition `name` where the step its declared transfer took,
    /// `declared`, states less: a may-bind, or the generic widening of a
    /// command that declares no existence transfer or whose transfer
    /// declined. A target a command writes whenever it completes
    /// (`UNCONDITIONAL_VARIABLE_WRITE`: `gets`, `lassign`, `file tempfile`) is
    /// bound afterwards, as the kind a may-write declaration names or a
    /// scalar; one it writes only on a match or reads before writing
    /// (`CONDITIONAL_VARIABLE_WRITE`, `READS_BEFORE_WRITE`: `regexp`, `lset`)
    /// holds what the place held joined with a scalar binding. A target is a
    /// word at one of the invocation's own variable-write positions; an
    /// alias's prepended words are not on the call, so through one any word
    /// is, unless a word runs a substitution that could define a name of its
    /// own. `None` keeps `declared`: a head the module may rebind, a command
    /// that may destroy a variable (`unset`'s class, or an irreversible
    /// descriptor such as `array unset`'s), any other declared step, and any
    /// definition no write class reaches.
    pub(crate) fn existence_by_write_class(
        &self,
        statement: &Statement,
        name: &str,
        declared: ExistenceStep,
    ) -> Option<ExistenceStep> {
        let Statement::Call {
            canonical_command,
            args,
            tokens,
            foreach_groups: None,
            ..
        } = statement
        else {
            return None;
        };
        let may_bind = match declared {
            ExistenceStep::Join(Existence::Bound(kind)) => Some(kind),
            step if step == ExistenceStep::UNKNOWN => None,
            _ => return None,
        };
        let head = statement.canonical_command_or_source();
        if !self.trusted(head) {
            return None;
        }
        let cooked = call_arguments(args, tokens.as_ref(), &self.lexer_config);
        let texts: Vec<&str> = cooked.iter().map(|arg| arg.text.as_ref()).collect();
        let surface = self.registry.own_surface_query();
        let traits = self.registry.invocation_traits(head, &texts, surface);
        if traits.contains(tcl_registry::Traits::DESTROYS_VARIABLE)
            || self
                .registry
                .resolve_call(head, &texts, surface)
                .is_some_and(|call| call.sub.is_some_and(|sub| sub.destructive))
        {
            return None;
        }
        let step = if traits.contains(tcl_registry::Traits::UNCONDITIONAL_VARIABLE_WRITE) {
            ExistenceStep::Set(Existence::Bound(may_bind.unwrap_or(BindingKind::Scalar)))
        } else if may_bind.is_none()
            && traits.intersects(
                tcl_registry::Traits::CONDITIONAL_VARIABLE_WRITE
                    .union(tcl_registry::Traits::READS_BEFORE_WRITE),
            )
        {
            ExistenceStep::Join(Existence::Bound(BindingKind::Scalar))
        } else {
            return None;
        };
        let target = if canonical_command.is_some() {
            !args.iter().any(|word| word.contains('[')) && texts.contains(&name)
        } else {
            self.registry
                .arg_indices_for_role(head, &texts, ArgRole::VarWrite)
                .into_iter()
                .any(|index| texts.get(index).is_some_and(|&text| text == name))
        };
        target.then_some(step)
    }
}

/// The existence state a statement's scripts start from, set on the driver
/// while this lives ([`LatticeDriver::body_entry_scope`]).
pub(crate) struct BodyEntryScope<'d> {
    pub(super) entry: &'d RefCell<Option<Vec<Existence>>>,
}

impl Drop for BodyEntryScope<'_> {
    fn drop(&mut self) {
        *self.entry.borrow_mut() = None;
    }
}
