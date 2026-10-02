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

use tcl_dialect::TclVersion;

use crate::arg_role::ArgRole;
use crate::completion::{CompletionCode, CompletionCodeDomain};
use crate::types::TclType;

use super::CommandSemantics;
use super::answers::{
    BindingKind, CompletionOutcome, CompletionPath, DependencyEvidence, EvalAnswer, ExactValue,
    ExactValueOrUnavailable, Existence, ExistenceOutcome, ExistenceTransfer, InvocationOutcome,
    NumericValue, RepresentationEvidence, RouteIdentity, StoreOutcome, TransferAnswer, TypeFacts,
};
use super::const_ops::{Raised, TargetSemantics};
use super::context::Budget;
use super::decline::DeclineReason;
use super::inputs::{
    AnalysisInputs, DomainFact, FactDomain, FactView, OperandId, PlaceRef, TargetId,
};
use super::publication::{ArrayWrite, raised_outcome};
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

    /// The error reading the absent place `place` raises: `can't read "x":
    /// no such variable` in every release, with `-errorcode` `NONE` before
    /// 8.6 and `TCL LOOKUP VARNAME x` from it (measured, 8.4 to 9.1). An
    /// element's message and code say whether its array exists, which the
    /// fact does not: the error is certain and its wording unproven.
    fn read_unbound(input: &dyn AnalysisInputs, place: &PlaceRef) -> EvalAnswer {
        let target = TargetSemantics::of(input.context().profile);
        let name = &place.name;
        let raised = if place.is_element() {
            Raised::unproven()
        } else {
            Raised::unanimous(&target, |release| {
                let code = if release >= TclVersion::V8_6 {
                    format!("TCL LOOKUP VARNAME {name}")
                } else {
                    "NONE".to_owned()
                };
                (format!("can't read \"{name}\": no such variable"), code)
            })
        };
        raised_outcome(NativeEvalId::CellWrite, REVISION, &target, raised)
    }

    /// The error reading the array `name` as a scalar raises: `can't read
    /// "b": variable is array`, with `-errorcode` `NONE` before 8.6 and `TCL
    /// READ VARNAME` from it (measured, 8.4 to 9.1).
    fn read_array(input: &dyn AnalysisInputs, name: &str) -> EvalAnswer {
        let target = TargetSemantics::of(input.context().profile);
        let raised = Raised::unanimous(&target, |release| {
            let code = if release >= TclVersion::V8_6 {
                "TCL READ VARNAME"
            } else {
                "NONE"
            };
            (
                format!("can't read \"{name}\": variable is array"),
                code.to_owned(),
            )
        });
        raised_outcome(NativeEvalId::CellWrite, REVISION, &target, raised)
    }

    fn outcome(result: ExactValue, stores: Vec<StoreOutcome>) -> EvalAnswer {
        let result_type = Self::type_of(&result);
        let per_target = stores
            .iter()
            .map(|store| (store.target(), result_type))
            .collect();
        EvalAnswer::Evaluated(Box::new(InvocationOutcome {
            completion: CompletionOutcome::Normal,
            nested_writes: Vec::new(),
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
                // A place the analysis proves holds an array is the write's
                // error.
                if let Some(name) = ArrayWrite::array_place(input, target) {
                    let semantics = TargetSemantics::of(input.context().profile);
                    return raised_outcome(
                        NativeEvalId::CellWrite,
                        REVISION,
                        &semantics,
                        ArrayWrite::Set.raised(&name, &semantics),
                    );
                }
                match input.operand(value, FactDomain::ExactValue).exact() {
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
                // Reading a place the rung proves unbound, or a whole array
                // as a scalar, is the command's error, never a value.
                match input.prior_store(&place, FactDomain::Existence) {
                    FactView::Domain(DomainFact::Existence(Existence::Unbound)) => {
                        return Self::read_unbound(input, &place);
                    }
                    FactView::Domain(DomainFact::Existence(Existence::Bound(
                        BindingKind::Array,
                    ))) if !place.is_element() => {
                        return Self::read_array(input, &place.name);
                    }
                    _ => {}
                }
                match input.prior_store(&place, FactDomain::ExactValue).exact() {
                    Ok(value) => Self::outcome(value, Vec::new()),
                    Err(answer) => answer,
                }
            }
        }
    }
}

/// `const name value` (from 9.0, TIP 677): the creation of a constant,
/// whose value is the program's only where the place is absent. `const`
/// creates its variable only where none exists — over an ordinary variable
/// it raises (`can't make constant "x": variable already exists`), and
/// over a constant it keeps the old value (tclsh 9.0 and 9.1: `const c 5;
/// const c 7; set c` is 5) — so an unbound place is written and any other
/// declines. The call returns the empty string, and on its normal
/// completion the place is bound as a scalar either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstWriteSemantics;

/// `const name value`.
pub static CONST_WRITE: ConstWriteSemantics = ConstWriteSemantics;

impl ConstWriteSemantics {
    /// The written place and the value's operand: the one `VarWrite`
    /// operand and the operand after it, last.
    fn form(input: &dyn AnalysisInputs) -> Option<(TargetId, OperandId)> {
        let view = input.invocation();
        let target = view.operands_with_role(ArgRole::VarWrite).next()?;
        let value = OperandId(target.0 + 1);
        (value.0 + 1 == view.operands.len()).then_some((TargetId(target), value))
    }
}

impl CommandSemantics for ConstWriteSemantics {
    fn identity(&self) -> &'static str {
        NativeEvalId::ConstWrite.as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: NativeEvalId::ConstWrite,
        }
    }

    fn transfer(
        &self,
        domain: FactDomain,
        input: &dyn AnalysisInputs,
        _budget: &mut Budget,
    ) -> TransferAnswer {
        let Some((target, _)) = Self::form(input) else {
            return TransferAnswer::Declined(DeclineReason::Unsupported);
        };
        if domain != FactDomain::Existence {
            return TransferAnswer::Generic;
        }
        TransferAnswer::Existence(ExistenceTransfer {
            paths: vec![CompletionPath {
                completion: CompletionCodeDomain::Exact(NORMAL),
                outcomes: vec![(target, ExistenceOutcome::Bind(BindingKind::Scalar))],
            }],
        })
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, _budget: &mut Budget) -> EvalAnswer {
        let Some((target, value)) = Self::form(input) else {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        };
        let place = match input.place(target.0) {
            Ok(place) => place,
            Err(reason) => return EvalAnswer::Declined(reason),
        };
        // Only the absent place is a value: an existing variable raises and
        // an existing constant keeps its value, neither of which the route
        // models.
        match input.prior_store(&place, FactDomain::Existence) {
            FactView::Domain(DomainFact::Existence(super::answers::Existence::Unbound)) => {}
            FactView::Domain(DomainFact::Existence(super::answers::Existence::Pending)) => {
                return EvalAnswer::Pending;
            }
            _ => return EvalAnswer::Declined(DeclineReason::Unsupported),
        }
        let value = match input.operand(value, FactDomain::ExactValue).exact() {
            Ok(value) => value,
            Err(answer) => return answer,
        };
        let written = CellWriteSemantics::type_of(&value);
        EvalAnswer::Evaluated(Box::new(InvocationOutcome {
            completion: CompletionOutcome::Normal,
            nested_writes: Vec::new(),
            result: ExactValueOrUnavailable::Exact(ExactValue::text("")),
            ordered_stores: vec![StoreOutcome::Write { target, value }],
            types: TypeFacts {
                result: Some(TclType::String),
                per_target: vec![(target, written)],
                shapes: Vec::new(),
            },
            evidence: DependencyEvidence {
                route: Some(RouteIdentity {
                    route: CONST_WRITE.route(),
                    implementation: CONST_WRITE.identity(),
                    revision: REVISION,
                }),
                ..DependencyEvidence::default()
            },
        }))
    }
}
