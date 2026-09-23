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

//! The lift over finite sets
//! (`docs/design/compiler/value-transfers.md` § *The correlated finite-set
//! limit*): step 4 of the lift, stated once for every specialisation.
//!
//! Per-member evaluation is admitted when exactly one *distinct SSA value*
//! among an invocation's inputs — its operands and the prior values of its
//! targets — is [`FactView::Finite`]. Two operands that read the same
//! identity are one distinct value and stay correlated; a repeated target
//! is one place. Two distinct finite inputs decline with
//! [`DeclineReason::CorrelatedSets`]: the lattice holds no pairing between
//! the sets, so no member of one may be paired with a member of the other,
//! and the cartesian product is refused as a precision limit. The
//! evaluator itself only ever sees exact inputs: the driver pins the one
//! finite value to each of its members in turn and joins the outcomes.

use super::CommandSemantics;
use super::answers::{EvalAnswer, ExactValue, InvocationOutcome};
use super::context::{AnalysisContext, BindingIdentity, Budget};
use super::decline::DeclineReason;
use super::inputs::{
    AnalysisInputs, BodyRegion, EvaluationState, FactDomain, FactView, OperandId, PlaceRef,
    ResolvedInvocationView, ValueIdentity, WordStructure,
};

/// `inner` with one finite value pinned to one of its members: every fact
/// read that carries `identity` answers that member exactly, and every
/// other read passes through.
pub struct PinnedInputs<'a> {
    inner: &'a dyn AnalysisInputs,
    identity: ValueIdentity,
    member: &'a ExactValue,
}

impl<'a> PinnedInputs<'a> {
    /// Pin `identity` to `member` over `inner`.
    #[must_use]
    pub fn new(
        inner: &'a dyn AnalysisInputs,
        identity: ValueIdentity,
        member: &'a ExactValue,
    ) -> Self {
        Self {
            inner,
            identity,
            member,
        }
    }

    fn pin(&self, view: FactView) -> FactView {
        match view {
            FactView::Finite(_, Some(identity)) if identity == self.identity => {
                FactView::Exact(self.member.clone(), Some(identity))
            }
            other => other,
        }
    }
}

impl AnalysisInputs for PinnedInputs<'_> {
    fn invocation(&self) -> &ResolvedInvocationView<'_> {
        self.inner.invocation()
    }

    fn operand(&self, id: OperandId, domain: FactDomain) -> FactView {
        self.pin(self.inner.operand(id, domain))
    }

    fn place(&self, id: OperandId) -> Result<PlaceRef, DeclineReason> {
        self.inner.place(id)
    }

    fn variable(&self, name: &str, domain: FactDomain) -> FactView {
        self.pin(self.inner.variable(name, domain))
    }

    fn prior_store(&self, place: &PlaceRef, domain: FactDomain) -> FactView {
        self.pin(self.inner.prior_store(place, domain))
    }

    fn word_structure(&self, id: OperandId) -> Result<WordStructure, DeclineReason> {
        self.inner.word_structure(id)
    }

    fn body(&self, id: OperandId) -> Result<BodyRegion, DeclineReason> {
        self.inner.body(id)
    }

    fn nested(&self, script: &str, state: &mut EvaluationState) -> EvalAnswer {
        self.inner.nested(script, state)
    }

    fn math_function(&self, name: &str) -> Result<BindingIdentity, DeclineReason> {
        self.inner.math_function(name)
    }

    fn context(&self) -> &AnalysisContext {
        self.inner.context()
    }
}

/// The distinct finite inputs of the invocation: every operand and the
/// prior value of every target its evaluation reads
/// ([`CommandSemantics::incoming_targets`]) that is a finite set,
/// deduplicated by SSA identity. An entry with no identity cannot be pinned
/// and counts as its own distinct value.
#[must_use]
pub fn finite_inputs(
    semantics: &dyn CommandSemantics,
    inputs: &dyn AnalysisInputs,
) -> Vec<(Option<ValueIdentity>, Vec<ExactValue>)> {
    let mut found: Vec<(Option<ValueIdentity>, Vec<ExactValue>)> = Vec::new();
    let mut note = |view: FactView| {
        if let FactView::Finite(members, identity) = view
            && (identity.is_none() || !found.iter().any(|(seen, _)| *seen == identity))
        {
            found.push((identity, members));
        }
    };
    let operands = inputs.invocation().operands.len();
    for index in 0..operands {
        note(inputs.operand(OperandId(index), FactDomain::ExactValue));
    }
    for target in semantics.incoming_targets(inputs) {
        if let Ok(place) = inputs.place(target.0) {
            note(inputs.prior_store(&place, FactDomain::ExactValue));
        }
    }
    found
}

/// The answer of an evaluation lifted over its finite inputs.
#[derive(Debug, Clone, PartialEq)]
pub enum LiftedAnswer {
    /// An input has not reached a usable fact.
    Pending,
    /// No exact fact, for a recorded reason.
    Declined(DeclineReason),
    /// One outcome per member of the one finite input, in member order —
    /// a single outcome when no input was finite.
    Evaluated(Vec<Box<InvocationOutcome>>),
}

/// Evaluate `semantics` over `inputs`, lifting over the one finite input
/// the limit admits: no finite input evaluates once; one evaluates per
/// member, up to `cap` members; two or more decline as correlated.
#[must_use]
pub fn evaluate_lifted(
    semantics: &dyn CommandSemantics,
    inputs: &dyn AnalysisInputs,
    budget: &mut Budget,
    cap: usize,
) -> LiftedAnswer {
    let finite = finite_inputs(semantics, inputs);
    match finite.as_slice() {
        [] => match semantics.evaluate(inputs, budget) {
            EvalAnswer::Pending => LiftedAnswer::Pending,
            EvalAnswer::Declined(reason) => LiftedAnswer::Declined(reason),
            EvalAnswer::Evaluated(outcome) => LiftedAnswer::Evaluated(vec![outcome]),
        },
        [(Some(identity), members)] => {
            if members.len() > cap {
                return LiftedAnswer::Declined(DeclineReason::TooManyMembers);
            }
            let mut outcomes = Vec::with_capacity(members.len());
            for member in members {
                let pinned = PinnedInputs::new(inputs, *identity, member);
                match semantics.evaluate(&pinned, budget) {
                    EvalAnswer::Pending => return LiftedAnswer::Pending,
                    EvalAnswer::Declined(reason) => return LiftedAnswer::Declined(reason),
                    EvalAnswer::Evaluated(outcome) => outcomes.push(outcome),
                }
            }
            LiftedAnswer::Evaluated(outcomes)
        }
        [(None, _)] => LiftedAnswer::Declined(DeclineReason::Unsupported),
        _ => LiftedAnswer::Declined(DeclineReason::CorrelatedSets),
    }
}
