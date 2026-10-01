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

//! The completions a command raises
//! (`docs/design/compiler/value-transfers.md` § *`catch`, `try`, and
//! completion*): `error`'s `TCL_ERROR`, and the protocols by which a body's
//! completion becomes its command's.

use super::CommandSemantics;
use super::answers::{
    CompletionOutcome, DependencyEvidence, EvalAnswer, ExactValueOrUnavailable, InvocationOutcome,
    RouteIdentity, TypeFacts,
};
use super::context::Budget;
use super::decline::DeclineReason;
use super::inputs::{AnalysisInputs, FactDomain, FactView, OperandId};
use super::route::{EvalRoute, NativeEvalId};

/// The revision of the registry-owned completion evaluators.
const REVISION: u64 = 1;

/// `error message ?info? ?code?`: the `TCL_ERROR` completion. The message
/// is the command's first word and the `-errorcode` its third, `NONE` where
/// none is given (measured, 8.4 to 9.1); a word the analysis does not prove
/// leaves its field unproven, and the completion stays certain. The command
/// stores nothing, so the error is raised after no store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorSemantics;

/// `error`.
pub static ERROR: ErrorSemantics = ErrorSemantics;

impl ErrorSemantics {
    /// A word's exact text, or the field's unproven stand-in; a word the
    /// solver has not reached is the whole answer's `Pending`.
    fn field(input: &dyn AnalysisInputs, at: usize) -> Result<ExactValueOrUnavailable, EvalAnswer> {
        match input.operand(OperandId(at), FactDomain::ExactValue) {
            FactView::Exact(value, _) => Ok(value.as_str().map_or_else(
                |_| ExactValueOrUnavailable::unproven_string(),
                ExactValueOrUnavailable::exact_text,
            )),
            FactView::Pending => Err(EvalAnswer::Pending),
            FactView::Finite(..) => Err(EvalAnswer::Declined(DeclineReason::CorrelatedSets)),
            FactView::Domain(_) | FactView::Top(_) => {
                Ok(ExactValueOrUnavailable::unproven_string())
            }
        }
    }
}

impl CommandSemantics for ErrorSemantics {
    fn identity(&self) -> &'static str {
        NativeEvalId::ErrorRaise.as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: NativeEvalId::ErrorRaise,
        }
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, _budget: &mut Budget) -> EvalAnswer {
        let words = input.invocation().operands.len();
        // Another count is `wrong # args`, which the route does not word.
        if !(1..=3).contains(&words) {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        }
        let message = match Self::field(input, 0) {
            Ok(message) => message,
            Err(answer) => return answer,
        };
        let error_code = if words == 3 {
            match Self::field(input, 2) {
                Ok(code) => code,
                Err(answer) => return answer,
            }
        } else {
            ExactValueOrUnavailable::exact_text("NONE")
        };
        EvalAnswer::Evaluated(Box::new(InvocationOutcome {
            completion: CompletionOutcome::Error {
                written: 0,
                message,
                error_code,
            },
            nested_writes: Vec::new(),
            result: ExactValueOrUnavailable::unproven_string(),
            ordered_stores: Vec::new(),
            types: TypeFacts::default(),
            evidence: DependencyEvidence {
                route: Some(RouteIdentity {
                    route: self.route(),
                    implementation: self.identity(),
                    revision: REVISION,
                }),
                ..DependencyEvidence::default()
            },
        }))
    }
}
