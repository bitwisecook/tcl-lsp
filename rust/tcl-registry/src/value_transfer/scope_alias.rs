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

//! The binders that link a local to a cell another frame holds — `global`,
//! `variable`, `my variable` and `sharedvar` — and `info default`, which
//! writes a procedure parameter's default
//! (`docs/design/compiler/value-transfers.md` § *Existence*, the entry
//! state, and § *Proc-level transfer summaries*).
//!
//! A linked local's existence and value are its cell's, which code outside
//! the frame may change, so the declaration states the plan — which operands
//! it links, and where the cells live — and no value: the local enters
//! may-bound. `info default` is evaluated where the analyser proves the
//! procedure and the parameter it names.

use crate::arg_role::ArgRole;
use crate::types::TclType;

use super::CommandSemantics;
use super::answers::{
    AliasFrame, CompletionOutcome, DependencyEvidence, EvalAnswer, ExactValue,
    ExactValueOrUnavailable, FactBounds, InvocationOutcome, PlanAnswer, RouteIdentity,
    ScopeAliasPlan, StoreOutcome, TypeFacts,
};
use super::answers::{BindingKind, Existence};
use super::context::Budget;
use super::decline::{DeclineReason, NoRouteReason};
use super::inputs::{AnalysisInputs, FactDomain, OperandId, ParameterDefault, TargetId};
use super::route::{EvalRoute, NativeEvalId};

/// A declaration that links each local its `VarWrite` operands name to a cell
/// in `frame`.
#[derive(Debug, Clone, Copy)]
pub struct ScopeAliasSemantics {
    /// Where the linked cells live.
    pub frame: AliasFrame,
    /// The specialisation's identity.
    pub identity: &'static str,
}

/// `global ?varName …?`.
pub static GLOBAL: ScopeAliasSemantics = ScopeAliasSemantics {
    frame: AliasFrame::Global,
    identity: "scope-alias:global",
};

/// `variable ?name value …? name ?value?`.
pub static VARIABLE: ScopeAliasSemantics = ScopeAliasSemantics {
    frame: AliasFrame::Namespace,
    identity: "scope-alias:namespace",
};

/// `my variable ?varName …?`.
pub static MY_VARIABLE: ScopeAliasSemantics = ScopeAliasSemantics {
    frame: AliasFrame::Object,
    identity: "scope-alias:object",
};

/// `sharedvar varName`.
pub const SHAREDVAR: ScopeAliasSemantics = ScopeAliasSemantics {
    frame: AliasFrame::Connection,
    identity: "scope-alias:connection",
};

impl CommandSemantics for ScopeAliasSemantics {
    fn identity(&self) -> &'static str {
        self.identity
    }

    /// The local's value is the linked cell's, which another frame holds.
    fn route(&self) -> EvalRoute {
        EvalRoute::None {
            reason: NoRouteReason::Declared,
        }
    }

    fn structure(&self, input: &dyn AnalysisInputs) -> PlanAnswer {
        PlanAnswer::ScopeAlias(ScopeAliasPlan {
            locals: input
                .invocation()
                .operands_with_role(ArgRole::VarWrite)
                .collect(),
            frame: self.frame,
        })
    }

    fn alias_frame(&self) -> Option<AliasFrame> {
        Some(self.frame)
    }
}

/// The revision of the registry-owned `info default` evaluator.
const INFO_DEFAULT_REVISION: u64 = 1;

/// `info default procname arg varname`: on its normal completion the
/// variable is written — with the parameter's default and the result 1, or
/// with the empty string and the result 0 when the parameter has none
/// (tclsh 8.4 to 9.1) — and a procedure or parameter that does not exist
/// raises with the variable untouched.
///
/// The procedure is named as a command is, from the namespace of the
/// function the call runs in, and must be a procedure: `info default` does
/// not follow an `interp alias` (`"h" isn't a procedure`), and after `rename
/// f g` it answers for `g` and raises for `f`. The analyser answers
/// [`AnalysisInputs::parameter_default`] only for a procedure of the module
/// whose binding stands — not redefined, renamed or aliased; for any other
/// the variable is written with a value the source does not give.
#[derive(Debug, Clone, Copy)]
pub struct InfoDefaultSemantics;

/// `info default procname arg varname`.
pub static INFO_DEFAULT: InfoDefaultSemantics = InfoDefaultSemantics;

impl CommandSemantics for InfoDefaultSemantics {
    fn identity(&self) -> &'static str {
        NativeEvalId::ParameterDefault.as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: NativeEvalId::ParameterDefault,
        }
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, _budget: &mut Budget) -> EvalAnswer {
        let view = input.invocation();
        // Operand 0 is the subcommand word.
        let first = view.argument_offset;
        if view.operands.len() != first + 3 {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        }
        let target = TargetId(OperandId(first + 2));
        let words = [first, first + 1].map(|index| {
            input
                .operand(OperandId(index), FactDomain::ExactValue)
                .exact()
        });
        let default = match words {
            [Ok(procedure), Ok(parameter)] => match (procedure.as_str(), parameter.as_str()) {
                (Ok(procedure), Ok(parameter)) => input.parameter_default(procedure, parameter),
                _ => ParameterDefault::Unknown,
            },
            [Err(EvalAnswer::Pending), _] | [_, Err(EvalAnswer::Pending)] => {
                return EvalAnswer::Pending;
            }
            _ => ParameterDefault::Unknown,
        };
        let (result, store) = match default {
            ParameterDefault::Value(value) => (
                ExactValueOrUnavailable::Exact(ExactValue::from_literal("1")),
                StoreOutcome::Write { target, value },
            ),
            ParameterDefault::None => (
                ExactValueOrUnavailable::Exact(ExactValue::from_literal("0")),
                StoreOutcome::Write {
                    target,
                    value: ExactValue::from_literal(""),
                },
            ),
            // The procedure or parameter the analysis cannot prove: the
            // variable is written on the normal completion all the same,
            // with a value the source alone does not give.
            ParameterDefault::Unknown => (
                ExactValueOrUnavailable::Unavailable(FactBounds {
                    existence: Existence::Bound(BindingKind::Scalar),
                    intrep: Some(TclType::Boolean),
                    shape: None,
                    segments: None,
                    taint: None,
                }),
                StoreOutcome::WriteUnavailable {
                    target,
                    facts: FactBounds {
                        existence: Existence::Bound(BindingKind::Scalar),
                        intrep: None,
                        shape: None,
                        segments: None,
                        taint: None,
                    },
                },
            ),
        };
        EvalAnswer::Evaluated(Box::new(InvocationOutcome {
            completion: CompletionOutcome::Normal,
            result,
            nested_writes: Vec::new(),
            ordered_stores: vec![store],
            types: TypeFacts {
                result: Some(TclType::Boolean),
                ..TypeFacts::default()
            },
            evidence: DependencyEvidence {
                route: Some(RouteIdentity {
                    route: self.route(),
                    implementation: NativeEvalId::ParameterDefault.as_str(),
                    revision: INFO_DEFAULT_REVISION,
                }),
                ..DependencyEvidence::default()
            },
        }))
    }
}
