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
//! transfer consumes. The source layout's plan (the var-list grammar, the
//! body) lands with the structural plans.

use crate::completion::CompletionCode;

use super::CommandSemantics;
use super::answers::{
    Binder, BinderName, BindingKind, CompletionProtocol, ExitRule, IterableKind, IterationPlan,
    PlanAnswer,
};
use super::decline::{DeclineReason, NoRouteReason};
use super::inputs::{AnalysisInputs, InvocationLayout, OperandId};
use super::route::EvalRoute;

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
            InvocationLayout::Source => PlanAnswer::Declined(DeclineReason::Unsupported),
        }
    }
}
