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

//! The cell read-modify-write specialisation, derived from
//! [`NativeLowering::CellReadModifyWrite`](crate::native_lowering::NativeLowering):
//! one place, read then written; the possible failure is the operation's.
//!
//! The increment has a registry-owned direct route. It is the slice-one
//! arithmetic — an integer base and an integer step, canonically spelled,
//! added without widening — and declines everything else; the release
//! rules (`010` by numeral grammar, the 8.4 overflow) arrive with the shared
//! numeric owner in slice two. The append and list-append updates carry the
//! descriptor and no route yet.

use tcl_dialect::model::SpecSurface;

use crate::completion::{CompletionCode, CompletionCodeDomain};
use crate::native_lowering::CellUpdate;
use crate::types::TclType;

use super::CommandSemantics;
use super::answers::{
    BindingKind, CompletionOutcome, CompletionPath, DependencyEvidence, EvalAnswer, ExactValue,
    ExactValueOrUnavailable, ExistenceOutcome, ExistenceTransfer, InvocationOutcome, PlanAnswer,
    RangeModel, RouteIdentity, StoreOutcome, TransferAnswer, TypeFacts,
};
use super::context::Budget;
use super::decline::{DeclineReason, NoRouteReason};
use super::inputs::{AnalysisInputs, FactDomain, FactView, OperandId, TargetId};
use super::route::{EvalRoute, NativeEvalId};

const NORMAL: &[CompletionCode] = &[CompletionCode::Ok];

/// The revision of the registry-owned increment implementation.
const INCREMENT_REVISION: u64 = 1;

/// The derived cell read-modify-write specialisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellUpdateSemantics {
    /// The operation the descriptor declares.
    pub update: CellUpdate,
    /// The surfaces on which an absent cell is created.
    pub creates_absent: Option<&'static [SpecSurface]>,
}

impl CellUpdateSemantics {
    /// The one target: the first operand, the place read and written.
    const TARGET: TargetId = TargetId(OperandId(0));

    /// The type the operation writes and returns.
    #[must_use]
    pub const fn result_type(self) -> TclType {
        match self.update {
            CellUpdate::Increment => TclType::Int,
            CellUpdate::Append => TclType::String,
            CellUpdate::ListAppend => TclType::List,
        }
    }

    /// Whether `operands` post-head words are a shape the operation takes.
    const fn accepts(self, operands: usize) -> bool {
        match self.update {
            CellUpdate::Increment => operands == 1 || operands == 2,
            CellUpdate::Append | CellUpdate::ListAppend => operands >= 1,
        }
    }

    fn type_facts(self) -> TypeFacts {
        TypeFacts {
            result: Some(self.result_type()),
            per_target: vec![(Self::TARGET, self.result_type())],
            shapes: Vec::new(),
        }
    }

    /// An integer read from the value domain, or why the operation cannot
    /// use it.
    fn integer_input(view: FactView) -> Result<i64, EvalAnswer> {
        match view {
            FactView::Pending => Err(EvalAnswer::Pending),
            FactView::Exact(value, _) => value
                .as_int()
                .ok_or(EvalAnswer::Declined(DeclineReason::WrongRepresentation)),
            // Per-member evaluation over a finite set is in force from the
            // slice that evaluates over lattice inputs with the correlated
            // finite-set limit; until then a set declines.
            FactView::Finite(..) => Err(EvalAnswer::Declined(DeclineReason::Unsupported)),
            FactView::Domain(_) => Err(EvalAnswer::Declined(DeclineReason::MalformedAnswer)),
            FactView::Top(reason) => Err(EvalAnswer::Declined(reason)),
        }
    }

    fn evaluate_increment(self, input: &dyn AnalysisInputs) -> EvalAnswer {
        let operands = input.invocation().operands.len();
        if !self.accepts(operands) {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        }
        let place = match input.place(Self::TARGET.0) {
            Ok(place) => place,
            Err(reason) => return EvalAnswer::Declined(reason),
        };
        let old = match Self::integer_input(input.prior_store(&place, FactDomain::ExactValue)) {
            Ok(old) => old,
            Err(answer) => return answer,
        };
        let step = if operands == 2 {
            match Self::integer_input(input.operand(OperandId(1), FactDomain::ExactValue)) {
                Ok(step) => step,
                Err(answer) => return answer,
            }
        } else {
            1
        };
        // Widening past the wide boundary is the numeric owner's release
        // rule (8.5 onwards widens, 8.4 raises); until it lands the overflow
        // declines rather than guessing either.
        let Some(new) = old.checked_add(step) else {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        };
        let value = ExactValue::int(new);
        EvalAnswer::Evaluated(Box::new(InvocationOutcome {
            completion: CompletionOutcome::Normal,
            result: ExactValueOrUnavailable::Exact(value.clone()),
            ordered_stores: vec![StoreOutcome::Write {
                target: Self::TARGET,
                value,
            }],
            types: self.type_facts(),
            evidence: DependencyEvidence {
                route: Some(RouteIdentity {
                    route: self.route(),
                    implementation: self.identity(),
                    revision: INCREMENT_REVISION,
                }),
                ..DependencyEvidence::default()
            },
        }))
    }
}

impl CommandSemantics for CellUpdateSemantics {
    fn identity(&self) -> &'static str {
        match self.update {
            CellUpdate::Increment => "cell-update:increment",
            CellUpdate::Append => "cell-update:append",
            CellUpdate::ListAppend => "cell-update:list-append",
        }
    }

    fn route(&self) -> EvalRoute {
        match self.update {
            CellUpdate::Increment => EvalRoute::Direct {
                id: NativeEvalId::CellIncrement,
            },
            CellUpdate::Append | CellUpdate::ListAppend => EvalRoute::None {
                reason: NoRouteReason::Unauthored,
            },
        }
    }

    fn structure(&self, input: &dyn AnalysisInputs) -> PlanAnswer {
        let operands = input.invocation().operands.len();
        if !self.accepts(operands) {
            return PlanAnswer::Declined(DeclineReason::Unsupported);
        }
        PlanAnswer::CellReadModifyWrite {
            target: Self::TARGET,
            operation: self.update,
            amount: (operands >= 2).then_some(OperandId(1)),
            creates_absent: self.creates_absent,
        }
    }

    fn transfer(
        &self,
        domain: FactDomain,
        input: &dyn AnalysisInputs,
        _budget: &mut Budget,
    ) -> TransferAnswer {
        if !self.accepts(input.invocation().operands.len()) {
            return TransferAnswer::Declined(DeclineReason::Unsupported);
        }
        match domain {
            FactDomain::Type => TransferAnswer::Type(self.type_facts()),
            FactDomain::Range if self.update == CellUpdate::Increment => {
                TransferAnswer::Range(RangeModel::IntegerAdd)
            }
            FactDomain::Existence => TransferAnswer::Existence(ExistenceTransfer {
                paths: vec![CompletionPath {
                    completion: CompletionCodeDomain::Exact(NORMAL),
                    outcomes: vec![(Self::TARGET, ExistenceOutcome::Bind(BindingKind::Scalar))],
                }],
            }),
            _ => TransferAnswer::Generic,
        }
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, _budget: &mut Budget) -> EvalAnswer {
        match self.update {
            CellUpdate::Increment => self.evaluate_increment(input),
            CellUpdate::Append | CellUpdate::ListAppend => match self.route() {
                EvalRoute::None { reason } => EvalAnswer::Declined(DeclineReason::NoRoute(reason)),
                _ => EvalAnswer::Declined(DeclineReason::Unsupported),
            },
        }
    }
}
