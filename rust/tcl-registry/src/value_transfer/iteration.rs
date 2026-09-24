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

//! The list-iteration protocol `foreach` and `lmap` declare explicitly.
//!
//! Iteration is never derived: `VarWriteTyping::ElementsOf` states a type
//! relationship and `LOOP_LIST_HEADER` a CFG shape, and neither says how
//! the binders are bound, padded, or what ends the loop. The declaration
//! here answers the synthetic loop header the CFG builder emits — its
//! operands are the iterable words in group order and its binders the
//! names defined per iteration — which is what the solver's per-element
//! transfer consumes — and the source layout: the var-list word's names,
//! the one list, and the body.

use crate::completion::CompletionCode;

use super::CommandSemantics;
use super::answers::{
    Binder, BinderName, BindingKind, BodyPlan, CompletionProtocol, ExitRule, IterableKind,
    IterationPlan, PlanAnswer,
};
use super::decline::{DeclineReason, NoRouteReason};
use super::inputs::{AnalysisInputs, FactDomain, FactView, InvocationLayout, OperandId};
use super::route::EvalRoute;
use crate::frame_effect::FrameLevel;

/// The completion codes a loop body absorbs.
const ABSORBED: &[CompletionCode] = &[CompletionCode::Break, CompletionCode::Continue];

/// The list-iteration specialisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IterationSemantics {
    /// Whether the loop collects each iteration's body result (`lmap`).
    pub collects: bool,
}

/// `foreach varList list ?varList list …? body`.
pub static FOREACH: IterationSemantics = IterationSemantics { collects: false };

/// `lmap varList list ?varList list …? body`.
pub static LMAP: IterationSemantics = IterationSemantics { collects: true };

impl CommandSemantics for IterationSemantics {
    fn identity(&self) -> &'static str {
        if self.collects {
            "iterate:lmap"
        } else {
            "iterate:foreach"
        }
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::None {
            reason: NoRouteReason::Unauthored,
        }
    }

    fn structure(&self, input: &dyn AnalysisInputs) -> PlanAnswer {
        let view = input.invocation();
        match view.layout {
            InvocationLayout::LoopHeader { binders } => {
                // One iterable per plan; a multi-list header is several
                // iterables stepping in lockstep, which the plan does not
                // yet describe.
                if view.operands.len() != 1 {
                    return PlanAnswer::Declined(DeclineReason::Unsupported);
                }
                PlanAnswer::Iterate(IterationPlan {
                    binders: binders
                        .iter()
                        .map(|name| Binder {
                            name: BinderName::Declared(name.clone()),
                            kind: BindingKind::Scalar,
                        })
                        .collect(),
                    iterable: IterableKind::List(OperandId(0)),
                    body: None,
                    exit: ExitRule::Exhaustion,
                    zero_iterations_bind: false,
                    completion: CompletionProtocol::Absorb(ABSORBED),
                })
            }
            InvocationLayout::Source => Self::source_plan(input),
        }
    }
}

impl IterationSemantics {
    /// The plan of `foreach varList list body` in its source layout: one
    /// binder per name of the var-list word, padded with the empty string
    /// past the list's end, over the one list, the body run in the caller's
    /// frame with `break` and `continue` absorbed, and no binder bound when
    /// the list is empty. Several var-list and list pairs step several
    /// iterables in lockstep, which one plan does not describe; a var-list
    /// the analysis does not know exactly names no binders.
    fn source_plan(input: &dyn AnalysisInputs) -> PlanAnswer {
        let view = input.invocation();
        let first = view.argument_offset;
        let words = view.operands.len().saturating_sub(first);
        // One pair or more and the body: the command's own arity.
        if words < 3 || words.is_multiple_of(2) {
            return PlanAnswer::Declined(DeclineReason::WrongRepresentation);
        }
        if words > 3 {
            return PlanAnswer::Declined(DeclineReason::Unsupported);
        }
        let names = match input.operand(OperandId(first), FactDomain::ExactValue) {
            FactView::Exact(value, _) => match String::from_utf8(value.bytes) {
                Ok(text) => tcl_syntax::list::split_list(&text)
                    .map(|names| names.iter().map(ToString::to_string).collect::<Vec<_>>()),
                Err(_) => return PlanAnswer::Declined(DeclineReason::NotText),
            },
            FactView::Top(reason) => return PlanAnswer::Declined(reason),
            FactView::Pending | FactView::Finite(..) | FactView::Domain(_) => {
                return PlanAnswer::Declined(DeclineReason::NotExact);
            }
        };
        // An empty or malformed var-list is the command's error.
        let Some(names) = names.ok().filter(|names| !names.is_empty()) else {
            return PlanAnswer::Declined(DeclineReason::WrongRepresentation);
        };
        PlanAnswer::Iterate(IterationPlan {
            binders: names
                .into_iter()
                .map(|name| Binder {
                    name: BinderName::Declared(name),
                    kind: BindingKind::Scalar,
                })
                .collect(),
            iterable: IterableKind::List(OperandId(first + 1)),
            body: Some(BodyPlan {
                body: OperandId(first + 2),
                frame: FrameLevel::Relative(0),
            }),
            exit: ExitRule::Exhaustion,
            zero_iterations_bind: false,
            completion: CompletionProtocol::Absorb(ABSORBED),
        })
    }
}
