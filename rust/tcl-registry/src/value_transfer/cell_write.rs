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

//! The exact-value write behind `set`
//! (`docs/design/compiler/value-transfers.md` § *Storage-writing
//! commands*): the direct route's one-target write.
//!
//! `set name value` writes the value, byte for byte, and returns it; `set
//! name` returns the value the place holds. The written place is the
//! operand the resolver gives the `VarWrite` role, the read one the operand
//! it gives `VarRead`, so the declaration never names an operand position.
//! Reading an absent variable is the program's error, never a value: a
//! prior the solver cannot prove declines. In statement position the
//! solver transfers the typed assignment natively; this route serves the
//! invocation-shaped uses, `[set x]` first.

use crate::arg_role::ArgRole;
use crate::completion::{CompletionCode, CompletionCodeDomain};
use crate::types::TclType;

use super::CommandSemantics;
use super::answers::{
    BindingKind, CompletionOutcome, CompletionPath, DependencyEvidence, EvalAnswer, ExactValue,
    ExactValueOrUnavailable, ExistenceOutcome, ExistenceTransfer, InvocationOutcome, NumericValue,
    RepresentationEvidence, RouteIdentity, StoreOutcome, TransferAnswer, TypeFacts,
};
use super::context::Budget;
use super::decline::DeclineReason;
use super::inputs::{AnalysisInputs, FactDomain, FactView, OperandId, TargetId};
use super::route::{EvalRoute, NativeEvalId};

const NORMAL: &[CompletionCode] = &[CompletionCode::Ok];

/// The revision of the registry-owned cell-write evaluator.
const REVISION: u64 = 1;

/// `set name ?value?`: the write of an exact value, or the read of the
/// prior one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellWriteSemantics;

/// `set name ?value?`.
pub static CELL_WRITE: CellWriteSemantics = CellWriteSemantics;

/// The form an invocation takes.
enum Form {
    /// `set name value`: the written place and the value's operand.
    Write { target: TargetId, value: OperandId },
    /// `set name`: the read place.
    Read { target: TargetId },
}

impl CellWriteSemantics {
    /// The invocation's form, from the roles the resolver gave its
    /// operands: a write names one `VarWrite` operand, the value the
    /// operand after it and last; a read names one `VarRead` operand alone.
    fn form(input: &dyn AnalysisInputs) -> Option<Form> {
        let view = input.invocation();
        if let Some(target) = view.operands_with_role(ArgRole::VarWrite).next() {
            let value = OperandId(target.0 + 1);
            return (value.0 + 1 == view.operands.len()).then_some(Form::Write {
                target: TargetId(target),
                value,
            });
        }
        let target = view.operands_with_role(ArgRole::VarRead).next()?;
        (view.operands.len() == 1).then_some(Form::Read {
            target: TargetId(target),
        })
    }

    /// An exact input, or the answer that stands in for one that is not.
    /// A finite set that reaches the evaluator is one the driver's lift
    /// could not pin, so it is the correlated case.
    fn exact_input(view: FactView) -> Result<ExactValue, EvalAnswer> {
        match view {
            FactView::Pending => Err(EvalAnswer::Pending),
            FactView::Exact(value, _) => Ok(value),
            FactView::Finite(..) => Err(EvalAnswer::Declined(DeclineReason::CorrelatedSets)),
            FactView::Domain(_) => Err(EvalAnswer::Declined(DeclineReason::MalformedAnswer)),
            FactView::Top(reason) => Err(EvalAnswer::Declined(reason)),
        }
    }

    /// The semantic type an exact value carries: its representation
    /// evidence, else its numeric classification, else a string.
    fn type_of(value: &ExactValue) -> TclType {
        match (value.representation, value.numeric) {
            (RepresentationEvidence::Constructed(t), _) => t,
            (RepresentationEvidence::Unknown, Some(NumericValue::Int(_))) => TclType::Int,
            (RepresentationEvidence::Unknown, Some(NumericValue::Float(_))) => TclType::Double,
            (RepresentationEvidence::Unknown, Some(NumericValue::Bool(_))) => TclType::Boolean,
            (RepresentationEvidence::Unknown, None) => TclType::String,
        }
    }

    fn outcome(result: ExactValue, stores: Vec<StoreOutcome>) -> EvalAnswer {
        let result_type = Self::type_of(&result);
        let per_target = stores
            .iter()
            .map(|store| (store.target(), result_type))
            .collect();
        EvalAnswer::Evaluated(Box::new(InvocationOutcome {
            completion: CompletionOutcome::Normal,
            result: ExactValueOrUnavailable::Exact(result),
            ordered_stores: stores,
            types: TypeFacts {
                result: Some(result_type),
                per_target,
                shapes: Vec::new(),
            },
            evidence: DependencyEvidence {
                route: Some(RouteIdentity {
                    route: CELL_WRITE.route(),
                    implementation: CELL_WRITE.identity(),
                    revision: REVISION,
                }),
                ..DependencyEvidence::default()
            },
        }))
    }
}

impl CommandSemantics for CellWriteSemantics {
    fn identity(&self) -> &'static str {
        "cell-write"
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: NativeEvalId::CellWrite,
        }
    }

    /// The read form's target: the write form reads nothing it stores.
    fn incoming_targets(&self, input: &dyn AnalysisInputs) -> Vec<TargetId> {
        match Self::form(input) {
            Some(Form::Read { target }) => vec![target],
            Some(Form::Write { .. }) | None => Vec::new(),
        }
    }

    fn transfer(
        &self,
        domain: FactDomain,
        input: &dyn AnalysisInputs,
        _budget: &mut Budget,
    ) -> TransferAnswer {
        let Some(form) = Self::form(input) else {
            return TransferAnswer::Declined(DeclineReason::Unsupported);
        };
        match (domain, form) {
            (FactDomain::Existence, Form::Write { target, .. }) => {
                TransferAnswer::Existence(ExistenceTransfer {
                    paths: vec![CompletionPath {
                        completion: CompletionCodeDomain::Exact(NORMAL),
                        outcomes: vec![(target, ExistenceOutcome::Bind(BindingKind::Scalar))],
                    }],
                })
            }
            (FactDomain::Type, Form::Write { target, value }) => {
                match input.operand(value, FactDomain::ExactValue) {
                    FactView::Exact(value, _) => {
                        let written = Self::type_of(&value);
                        TransferAnswer::Type(TypeFacts {
                            result: Some(written),
                            per_target: vec![(target, written)],
                            shapes: Vec::new(),
                        })
                    }
                    _ => TransferAnswer::Generic,
                }
            }
            _ => TransferAnswer::Generic,
        }
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, _budget: &mut Budget) -> EvalAnswer {
        let Some(form) = Self::form(input) else {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        };
        match form {
            Form::Write { target, value } => {
                if let Err(reason) = input.place(target.0) {
                    return EvalAnswer::Declined(reason);
                }
                match Self::exact_input(input.operand(value, FactDomain::ExactValue)) {
                    Ok(value) => {
                        let store = StoreOutcome::Write {
                            target,
                            value: value.clone(),
                        };
                        Self::outcome(value, vec![store])
                    }
                    Err(answer) => answer,
                }
            }
            Form::Read { target } => {
                let place = match input.place(target.0) {
                    Ok(place) => place,
                    Err(reason) => return EvalAnswer::Declined(reason),
                };
                match Self::exact_input(input.prior_store(&place, FactDomain::ExactValue)) {
                    Ok(value) => Self::outcome(value, Vec::new()),
                    Err(answer) => answer,
                }
            }
        }
    }
}
