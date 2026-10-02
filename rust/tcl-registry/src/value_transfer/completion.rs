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
//! completion*): `error`'s `TCL_ERROR`, the codes of `return`, `break` and
//! `continue`, and the protocols by which a body's completion becomes its
//! command's.

use tcl_dialect::TclVersion;
use tcl_dialect::model::SpecSurface;

use crate::arg_role::ArgRole;
use crate::completion::{CompletionCode, CompletionCodeDomain};

use super::CommandSemantics;
use super::answers::{
    BindingKind, CompletionOutcome, CompletionPath, DependencyEvidence, EvalAnswer,
    ExactValueOrUnavailable, ExistenceOutcome, ExistenceTransfer, InvocationOutcome, RouteIdentity,
    TransferAnswer, TypeFacts,
};
use super::const_ops::TargetSemantics;
use super::context::Budget;
use super::decline::{DeclineReason, NoRouteReason};
use super::destructure::unavailable;
use super::inputs::{AnalysisInputs, FactDomain, FactView, OperandId, TargetId};
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
        completed(
            NativeEvalId::ErrorRaise,
            CompletionOutcome::Error {
                written: 0,
                message,
                error_code,
            },
            ExactValueOrUnavailable::unproven_string(),
        )
    }
}

/// The answer of a route whose whole effect is its completion: no store,
/// the completion and the result it gives.
fn completed(
    id: NativeEvalId,
    completion: CompletionOutcome,
    result: ExactValueOrUnavailable,
) -> EvalAnswer {
    EvalAnswer::Evaluated(Box::new(InvocationOutcome {
        completion,
        nested_writes: Vec::new(),
        result,
        ordered_stores: Vec::new(),
        types: TypeFacts::default(),
        evidence: DependencyEvidence {
            route: Some(RouteIdentity {
                route: EvalRoute::Direct { id },
                implementation: id.as_str(),
                revision: REVISION,
            }),
            ..DependencyEvidence::default()
        },
    }))
}

/// `break` and `continue`: the completion with the code of the same name
/// at level 0 and the empty result, after no store. The commands take no
/// word, and a word is `wrong # args`, which the route does not word.
fn loop_control(input: &dyn AnalysisInputs, id: NativeEvalId, code: CompletionCode) -> EvalAnswer {
    if !input.invocation().operands.is_empty() {
        return EvalAnswer::Declined(DeclineReason::Unsupported);
    }
    completed(
        id,
        CompletionOutcome::Code {
            code,
            level: 0,
            result: ExactValueOrUnavailable::exact_text(""),
        },
        ExactValueOrUnavailable::exact_text(""),
    )
}

/// `break`: `TCL_BREAK` at level 0, which `catch` reports as 3 and a loop
/// absorbs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreakSemantics;

/// `break`.
pub static BREAK: BreakSemantics = BreakSemantics;

impl CommandSemantics for BreakSemantics {
    fn identity(&self) -> &'static str {
        NativeEvalId::BreakComplete.as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: NativeEvalId::BreakComplete,
        }
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, _budget: &mut Budget) -> EvalAnswer {
        loop_control(input, NativeEvalId::BreakComplete, CompletionCode::Break)
    }
}

/// `continue`: `TCL_CONTINUE` at level 0, which `catch` reports as 4 and a
/// loop absorbs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContinueSemantics;

/// `continue`.
pub static CONTINUE: ContinueSemantics = ContinueSemantics;

impl CommandSemantics for ContinueSemantics {
    fn identity(&self) -> &'static str {
        NativeEvalId::ContinueComplete.as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: NativeEvalId::ContinueComplete,
        }
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, _budget: &mut Budget) -> EvalAnswer {
        loop_control(
            input,
            NativeEvalId::ContinueComplete,
            CompletionCode::Continue,
        )
    }
}

/// `return ?-code code? ?-level level? ?result?`: the completion its options
/// give, after no store. The options are read as Tcl reads them: while two
/// words remain the first names an option and the second is its value, and
/// a last word on its own is the result, whatever it starts with. `-code`
/// is one of the five names or an integer and the default is `ok`; `-level`
/// is a non-negative integer from 8.5, default 1. At a positive level the
/// completion is the pending one a procedure or `catch` consumes
/// ([`CompletionOutcome::Code`]); at level 0 it is the code itself — a
/// normal completion for `ok`, the error for `error`.
///
/// An option the route does not read (`-errorcode`, `-errorinfo`,
/// `-errorstack`, `-options`, and from 8.5 any other pair, which the
/// options dictionary keeps), a code or level that is not spelled in
/// canonical decimal (`010` is 8 before 9.0 and 10 from it), a word that is
/// not exact, and `-level` where the target is not proven to have it
/// decline. The result is exact where its word is, and unproven otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReturnSemantics;

/// `return`.
pub static RETURN: ReturnSemantics = ReturnSemantics;

impl ReturnSemantics {
    /// A `-code` value: one of the five names, or an integer spelled the
    /// one way every release reads it.
    fn code(word: &str) -> Option<CompletionCode> {
        match word {
            "ok" => Some(CompletionCode::Ok),
            "error" => Some(CompletionCode::Error),
            "return" => Some(CompletionCode::Return),
            "break" => Some(CompletionCode::Break),
            "continue" => Some(CompletionCode::Continue),
            number => number
                .parse::<i32>()
                .ok()
                .filter(|parsed| parsed.to_string() == number)
                .map(CompletionCode::from_int),
        }
    }

    /// A `-level` value: a non-negative integer spelled the one way every
    /// release reads it.
    fn level(word: &str) -> Option<u32> {
        word.parse::<u32>()
            .ok()
            .filter(|parsed| parsed.to_string() == word)
    }

    /// The text of the word at `at`, which an option name or value must
    /// have: a word the solver has not reached makes the answer pending.
    fn text(input: &dyn AnalysisInputs, at: usize) -> Result<String, EvalAnswer> {
        match input.operand(OperandId(at), FactDomain::ExactValue) {
            FactView::Exact(value, _) => value
                .as_str()
                .map(str::to_owned)
                .map_err(EvalAnswer::Declined),
            FactView::Pending => Err(EvalAnswer::Pending),
            FactView::Finite(..) => Err(EvalAnswer::Declined(DeclineReason::CorrelatedSets)),
            FactView::Domain(_) | FactView::Top(_) => {
                Err(EvalAnswer::Declined(DeclineReason::NotExact))
            }
        }
    }

    fn evaluate_return(input: &dyn AnalysisInputs) -> EvalAnswer {
        let count = input.invocation().operands.len();
        let has_result = count % 2 == 1;
        let (mut code, mut level) = (CompletionCode::Ok, 1_u32);
        for name_at in (0..count - usize::from(has_result)).step_by(2) {
            let (name, value) = match (Self::text(input, name_at), Self::text(input, name_at + 1)) {
                (Ok(name), Ok(value)) => (name, value),
                (Err(answer), _) | (_, Err(answer)) => return answer,
            };
            match name.as_str() {
                "-code" => {
                    let Some(chosen) = Self::code(&value) else {
                        return EvalAnswer::Declined(DeclineReason::Unsupported);
                    };
                    code = chosen;
                }
                "-level" => {
                    match TargetSemantics::of(input.context().profile).release {
                        Some(release) if release >= TclVersion::V8_5 => {}
                        Some(_) => return EvalAnswer::Declined(DeclineReason::Unsupported),
                        None => {
                            return EvalAnswer::Declined(unavailable(SpecSurface::TCL85_PLUS));
                        }
                    }
                    let Some(chosen) = Self::level(&value) else {
                        return EvalAnswer::Declined(DeclineReason::Unsupported);
                    };
                    level = chosen;
                }
                _ => return EvalAnswer::Declined(DeclineReason::Unsupported),
            }
        }
        let result = if has_result {
            match input.operand(OperandId(count - 1), FactDomain::ExactValue) {
                FactView::Exact(value, _) => ExactValueOrUnavailable::Exact(value),
                FactView::Pending => return EvalAnswer::Pending,
                FactView::Finite(..) => {
                    return EvalAnswer::Declined(DeclineReason::CorrelatedSets);
                }
                FactView::Domain(_) | FactView::Top(_) => {
                    ExactValueOrUnavailable::unproven_string()
                }
            }
        } else {
            ExactValueOrUnavailable::exact_text("")
        };
        let completion = match (level, code) {
            (0, CompletionCode::Ok) => CompletionOutcome::Normal,
            (0, CompletionCode::Error) => CompletionOutcome::Error {
                written: 0,
                message: result.clone(),
                error_code: ExactValueOrUnavailable::exact_text("NONE"),
            },
            _ => CompletionOutcome::Code {
                code,
                level,
                result: result.clone(),
            },
        };
        completed(NativeEvalId::ReturnComplete, completion, result)
    }
}

impl CommandSemantics for ReturnSemantics {
    fn identity(&self) -> &'static str {
        NativeEvalId::ReturnComplete.as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: NativeEvalId::ReturnComplete,
        }
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, _budget: &mut Budget) -> EvalAnswer {
        Self::evaluate_return(input)
    }
}

/// `catch script ?resultVarName? ?optionsVarName?`: whatever the script's
/// completion, the result variable and the options variable are written, and
/// what they hold is the script's to say — nothing the analysis knows of an
/// opaque body, so each is bound with a value that is not available. The
/// script's own writes are the body's, stated where the body is lowered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatchSemantics;

/// `catch`.
pub static CATCH: CatchSemantics = CatchSemantics;

impl CommandSemantics for CatchSemantics {
    fn identity(&self) -> &'static str {
        "catch"
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
        let outcomes: Vec<_> = input
            .invocation()
            .operands_with_role(ArgRole::VarWrite)
            .filter(|id| input.place(*id).is_ok())
            .map(|id| (TargetId(id), ExistenceOutcome::Bind(BindingKind::Scalar)))
            .collect();
        if outcomes.is_empty() {
            return TransferAnswer::Generic;
        }
        TransferAnswer::Existence(ExistenceTransfer {
            paths: vec![CompletionPath {
                completion: CompletionCodeDomain::Any,
                outcomes,
            }],
        })
    }
}
