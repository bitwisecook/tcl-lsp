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

//! The unbind specialisation, derived from
//! [`Traits::DESTROYS_VARIABLE`](crate::traits::Traits::DESTROYS_VARIABLE):
//! every name the invocation's variable-writing operands resolve to is
//! unbound afterwards. The existence transfer is what the `info exists`
//! fold reads; the error on an absent place and the `-nocomplain` form's
//! completion domain arrive with the existence rung.

use tcl_dialect::TclVersion;

use crate::arg_role::ArgRole;
use crate::completion::{CompletionCode, CompletionCodeDomain};
use crate::types::TclType;

use super::CommandSemantics;
use super::answers::{
    BindingKind, CompletionOutcome, CompletionPath, DependencyEvidence, EvalAnswer, ExactValue,
    ExactValueOrUnavailable, Existence, ExistenceOutcome, ExistenceTransfer, InvocationOutcome,
    RouteIdentity, StoreOutcome, TransferAnswer, TypeFacts,
};
use super::const_ops::{Raised, TargetSemantics};
use super::context::Budget;
use super::decline::{DeclineReason, NoRouteReason};
use super::inputs::{AnalysisInputs, DomainFact, FactDomain, FactView, TargetId};
use super::route::{EvalRoute, NativeEvalId};

/// The revision of the registry-owned `unset` evaluator.
const REVISION: u64 = 1;

const NORMAL: &[CompletionCode] = &[CompletionCode::Ok];

/// The derived unbind specialisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UnbindSemantics;

impl UnbindSemantics {
    /// The error unsetting the absent variable `name` raises: `can't unset
    /// "x": no such variable` in every release, with `-errorcode` `NONE`
    /// before 8.6 and `TCL LOOKUP VARNAME x` from it (measured, 8.4 to 9.1).
    fn absent(name: &str, target: &TargetSemantics) -> Raised {
        Raised::unanimous(target, |release| {
            let code = if release >= TclVersion::V8_6 {
                format!("TCL LOOKUP VARNAME {name}")
            } else {
                "NONE".to_owned()
            };
            (format!("can't unset \"{name}\": no such variable"), code)
        })
    }
}

impl CommandSemantics for UnbindSemantics {
    fn identity(&self) -> &'static str {
        "unbind"
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: NativeEvalId::VariableUnset,
        }
    }

    /// Each named variable unbound in the command's order. A variable the
    /// rung proves unbound is the command's error unless `-nocomplain` is
    /// given, and the variables after it keep what they held: the stores
    /// before it ran (the prefix rule). A name whose existence the rung does
    /// not prove, and an element — whose existence it never states —
    /// decline.
    fn evaluate(&self, input: &dyn AnalysisInputs, _budget: &mut Budget) -> EvalAnswer {
        let view = input.invocation();
        let targets: Vec<_> = view.operands_with_role(ArgRole::VarWrite).collect();
        if targets.is_empty() {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        }
        let nocomplain = view.operands.iter().any(|operand| {
            operand.role != Some(ArgRole::VarWrite) && operand.text == "-nocomplain"
        });
        let semantics = TargetSemantics::of(input.context().profile);
        let mut stores = Vec::with_capacity(targets.len());
        let mut unset: Vec<String> = Vec::new();
        let mut failing: Option<(usize, Raised)> = None;
        for (at, id) in targets.iter().enumerate() {
            let place = match input.place(*id) {
                Ok(place) => place,
                Err(reason) => return EvalAnswer::Declined(reason),
            };
            if place.is_element() {
                return EvalAnswer::Declined(DeclineReason::NotExact);
            }
            // A name an earlier target already unset is absent by now.
            let absent = if unset.contains(&place.name) {
                true
            } else {
                match input.prior_store(&place, FactDomain::Existence) {
                    FactView::Domain(DomainFact::Existence(Existence::Unbound)) => true,
                    FactView::Domain(DomainFact::Existence(Existence::Bound(_))) => false,
                    FactView::Domain(DomainFact::Existence(Existence::Pending))
                    | FactView::Pending => return EvalAnswer::Pending,
                    FactView::Top(reason) => return EvalAnswer::Declined(reason),
                    FactView::Domain(_) | FactView::Exact(..) | FactView::Finite(..) => {
                        return EvalAnswer::Declined(DeclineReason::NotExact);
                    }
                }
            };
            let target = TargetId(*id);
            if absent && !nocomplain {
                failing.get_or_insert((at, Self::absent(&place.name, &semantics)));
                stores.push(StoreOutcome::Unbind { target });
            } else if absent {
                stores.push(StoreOutcome::Preserve { target });
            } else {
                unset.push(place.name);
                stores.push(StoreOutcome::Unbind { target });
            }
        }
        let (completion, result) = match failing {
            Some((written, raised)) => (
                CompletionOutcome::Error {
                    written,
                    message: raised.message,
                    error_code: raised.error_code,
                },
                ExactValueOrUnavailable::unproven_string(),
            ),
            None => (
                CompletionOutcome::Normal,
                ExactValueOrUnavailable::Exact(ExactValue::text("")),
            ),
        };
        EvalAnswer::Evaluated(Box::new(InvocationOutcome {
            completion,
            nested_writes: Vec::new(),
            result,
            ordered_stores: stores,
            types: TypeFacts {
                result: Some(TclType::String),
                per_target: Vec::new(),
                shapes: Vec::new(),
            },
            evidence: DependencyEvidence {
                route: Some(RouteIdentity {
                    route: self.route(),
                    implementation: self.identity(),
                    revision: REVISION,
                }),
                release: semantics.release,
                ..DependencyEvidence::default()
            },
        }))
    }

    fn transfer(
        &self,
        domain: FactDomain,
        input: &dyn AnalysisInputs,
        _budget: &mut Budget,
    ) -> TransferAnswer {
        if domain != FactDomain::Existence {
            return TransferAnswer::Generic;
        }
        let view = input.invocation();
        // A name the driver cannot resolve to a place — a computed name,
        // an escaping or traced place — is left to the owner of that
        // decline: the dynamic-name barrier already blinds the existence
        // fold for a computed destroy.
        let outcomes = view
            .operands_with_role(ArgRole::VarWrite)
            .filter(|id| input.place(*id).is_ok())
            .map(|id| (TargetId(id), ExistenceOutcome::Unbind))
            .collect();
        TransferAnswer::Existence(ExistenceTransfer {
            paths: vec![CompletionPath {
                completion: CompletionCodeDomain::Exact(NORMAL),
                outcomes,
            }],
        })
    }
}

/// `array unset arrayName ?pattern?`: the pattern-less form unsets the
/// array as `unset` does, but only an array — over a scalar or an absent
/// name the command does nothing and never raises (tclsh 8.4 to 9.1: `set
/// s 1; array unset s` leaves `s`) — so the transfer reads the place's
/// prior fact: an array is unbound, a scalar or an absent place preserved,
/// and a place that may be either keeps the generic widening. With a
/// pattern the matching elements go and the array itself stays, so its
/// place is preserved; which elements went is the elements' may-write,
/// which the SSA's element definitions carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ArrayUnsetSemantics;

/// `array unset arrayName ?pattern?`.
pub static ARRAY_UNSET: ArrayUnsetSemantics = ArrayUnsetSemantics;

impl CommandSemantics for ArrayUnsetSemantics {
    fn identity(&self) -> &'static str {
        "array-unset"
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::None {
            reason: NoRouteReason::Unauthored,
        }
    }

    fn transfer(
        &self,
        domain: FactDomain,
        input: &dyn AnalysisInputs,
        _budget: &mut Budget,
    ) -> TransferAnswer {
        if domain != FactDomain::Existence {
            return TransferAnswer::Generic;
        }
        let view = input.invocation();
        let Some(target) = view.operands_with_role(ArgRole::VarWrite).next() else {
            return TransferAnswer::Generic;
        };
        let Ok(place) = input.place(target) else {
            return TransferAnswer::Generic;
        };
        let patterned = view.operands.len() > target.0 + 1;
        let outcome = if patterned {
            ExistenceOutcome::Preserve
        } else {
            match input.prior_store(&place, FactDomain::Existence) {
                FactView::Domain(DomainFact::Existence(Existence::Bound(BindingKind::Array))) => {
                    ExistenceOutcome::Unbind
                }
                FactView::Domain(DomainFact::Existence(
                    Existence::Bound(BindingKind::Scalar)
                    | Existence::Unbound
                    | Existence::MayBound
                    | Existence::Pending,
                )) => ExistenceOutcome::Preserve,
                _ => return TransferAnswer::Generic,
            }
        };
        TransferAnswer::Existence(ExistenceTransfer {
            paths: vec![CompletionPath {
                completion: CompletionCodeDomain::Exact(NORMAL),
                outcomes: vec![(TargetId(target), outcome)],
            }],
        })
    }
}
