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
//! All three operations have a registry-owned direct route, and each is
//! the value computation the runtime adapters call — `ValueOps::int_add`,
//! `var::append_bytes`, `var::lappend_value` — over [`ConstOps`], with the
//! lattice write as the compile-time store. The release rules are the
//! adapter's: the increment's base and step are read under the target's
//! numeral grammar (`010` is 9 up to 8.6 and 11 from 9.0), an overflow
//! widens from 8.5 and declines under 8.4 or an unnamed release, and a
//! list append over a value that is not a list is the program's error,
//! which is never a value.

use tcl_dialect::model::SpecSurface;
use tcl_syntax::value::ValueOps;

use crate::completion::{CompletionCode, CompletionCodeDomain};
use crate::native_lowering::CellUpdate;
use crate::types::TclType;

use super::CommandSemantics;
use super::answers::{
    BindingKind, CompletionOutcome, CompletionPath, DependencyEvidence, EvalAnswer,
    ExactValueOrUnavailable, ExistenceOutcome, ExistenceTransfer, InvocationOutcome, PlanAnswer,
    RangeModel, RouteIdentity, StoreOutcome, TransferAnswer, TypeFacts,
};
use super::const_ops::{ConstOps, ConstValue, Needs};
use super::context::Budget;
use super::decline::DeclineReason;
use super::inputs::{AnalysisInputs, FactDomain, OperandId, TargetId};
use super::route::{EvalRoute, NativeEvalId};

const NORMAL: &[CompletionCode] = &[CompletionCode::Ok];

/// The revision of the registry-owned cell-update evaluators: 2 is the
/// shared cores over `ConstOps`; 1 was the slice-one checked arithmetic.
const REVISION: u64 = 2;

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

    /// The release axes the operation's core reads.
    #[must_use]
    pub const fn needs(self) -> Needs {
        match self.update {
            CellUpdate::Increment => Needs::NUMERAL_GRAMMAR.union(Needs::INT_TOWER),
            CellUpdate::Append => Needs::NONE,
            CellUpdate::ListAppend => Needs::LIST_RENDERING,
        }
    }

    /// The catalogued evaluator.
    #[must_use]
    pub const fn evaluator(self) -> NativeEvalId {
        match self.update {
            CellUpdate::Increment => NativeEvalId::CellIncrement,
            CellUpdate::Append => NativeEvalId::CellAppend,
            CellUpdate::ListAppend => NativeEvalId::CellListAppend,
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

    fn evaluate_update(self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        let operands = input.invocation().operands.len();
        if !self.accepts(operands) {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        }
        let place = match input.place(Self::TARGET.0) {
            Ok(place) => place,
            Err(reason) => return EvalAnswer::Declined(reason),
        };
        let prior = match input.prior_store(&place, FactDomain::ExactValue).exact() {
            Ok(prior) => prior,
            Err(answer) => return answer,
        };
        let mut values = Vec::with_capacity(operands.saturating_sub(1));
        for index in 1..operands {
            match input
                .operand(OperandId(index), FactDomain::ExactValue)
                .exact()
            {
                Ok(value) => values.push(ConstValue::from_exact(&value)),
                Err(answer) => return answer,
            }
        }
        let mut ops = match ConstOps::admit(input.context(), budget, self.needs()) {
            Ok(ops) => ops,
            Err(reason) => return EvalAnswer::Declined(reason),
        };
        let target = *ops.target();
        let current = ConstValue::from_exact(&prior);
        let computed = match self.update {
            CellUpdate::Increment => {
                let step = values
                    .first()
                    .cloned()
                    .unwrap_or_else(|| ConstValue::int(1));
                ops.int_add(Some(&current), &step)
                    .map_err(|error| ops.decline_value(&error))
            }
            CellUpdate::Append => Ok(tcl_cmd_core::var::append_bytes(
                &mut ops,
                Some(current),
                &values,
            )),
            CellUpdate::ListAppend => {
                tcl_cmd_core::var::lappend_value(&mut ops, Some(current), &values)
                    .map_err(|error| ops.decline(&error))
            }
        };
        let value = match computed.and_then(|value| ops.take(value)) {
            Ok(value) => value,
            Err(reason) => return EvalAnswer::Declined(reason),
        };
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
                    revision: REVISION,
                }),
                numerals: match self.update {
                    CellUpdate::Increment => target.numerals,
                    CellUpdate::Append | CellUpdate::ListAppend => None,
                },
                release: target.release,
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
        EvalRoute::Direct {
            id: self.evaluator(),
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

    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        self.evaluate_update(input, budget)
    }
}
