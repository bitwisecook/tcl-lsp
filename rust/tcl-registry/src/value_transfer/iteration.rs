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

//! The list-iteration protocol `foreach` and `lmap` declare explicitly, and
//! the loop rule every iteration plan shares.
//!
//! Iteration is never derived: `VarWriteTyping::ElementsOf` states a type
//! relationship and `LOOP_LIST_HEADER` a CFG shape, and neither says how
//! the binders are bound, padded, or what ends the loop. The declaration
//! here answers the synthetic loop header the CFG builder emits — its
//! operands are the iterable words in group order and its binders the
//! names defined per iteration — which is what the solver's per-element
//! transfer consumes — and the source layout: the var-list word's names,
//! the one list, and the body.
//!
//! The loop rule is what a loop does with each completion of its body and
//! what it yields when it completes normally ([`IterationPlan::step`],
//! [`LoopResult::of`]). Measured under tclsh 8.4 to 9.1 (`lmap` from 8.6,
//! `-level` from 8.5):
//!
//! ```tcl
//! foreach x {1 2 3} {set y $x; break}               ;# 0; y is 1, x is 1
//! foreach x {1 2 3} {set y $x; continue; set y no}  ;# 0; y is 3
//! foreach x {1 2} {set y $x; error boom}            ;# 1, boom; y is 1
//! foreach x {1 2} {return -level 0 -code 5 v}       ;# 5, v
//! foreach x {1 2} {return -code break}              ;# 2 in a procedure
//! lmap x {1 2 3 4} {if {$x == 2} continue; if {$x == 4} break; set x}   ;# 1 3
//! ```

use crate::completion::CompletionCode;

use super::CommandSemantics;
use super::answers::{
    Binder, BinderName, BindingKind, BodyPlan, CompletionOutcome, CompletionProtocol,
    ExactValueOrUnavailable, Existence, ExitRule, FactBounds, IterableKind, IterationPlan,
    PlanAnswer,
};
use super::const_ops::TargetSemantics;
use super::decline::{DeclineReason, NoRouteReason};
use super::inputs::{AnalysisInputs, FactDomain, FactView, InvocationLayout, OperandId};
use super::route::EvalRoute;
use crate::frame_effect::FrameLevel;
use crate::types::TclType;

/// The completion codes a loop body absorbs: `break` ends the loop and
/// `continue` the iteration, and every other code the body completes with is
/// the loop's own. Every loop the registry declares absorbs these two.
pub const LOOP_ABSORBED: &[CompletionCode] = &[CompletionCode::Break, CompletionCode::Continue];

/// What a loop does after one run of its body, by how the body completed
/// ([`IterationPlan::step`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopStep {
    /// The body completed normally: the iteration yields the body's result
    /// and the loop goes on.
    Next,
    /// The body completed with an absorbed code that ends the iteration
    /// alone (`continue`): the iteration yields nothing and the loop goes on.
    Skip,
    /// The body completed with an absorbed `break`: the loop completes
    /// normally with what the iterations before it yielded.
    Exit,
    /// Any other completion is the loop's own: an error, a `return`, a code
    /// the loop does not absorb, and a `break` or `continue` at a level above
    /// 0, which the loop sees as a `return` (`return -code break` in a
    /// procedure's loop leaves the procedure).
    Leave,
}

/// What a loop yields when it completes normally ([`LoopResult::of`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoopResult {
    /// The empty string, whatever its body did (`foreach`, `foreachLine`).
    Empty,
    /// The list of the results of the iterations whose body completed
    /// normally (`lmap`): an iteration that ends in `continue` contributes
    /// nothing, and `break` ends the collection with what was collected.
    Collected,
    /// A result the declaration does not state: a pack's loop.
    Unstated,
}

impl IterationPlan {
    /// What the loop does after its body completes with `completion`: the
    /// plan's completion protocol read for that completion. A normal
    /// completion goes on to the next iteration; a code the protocol absorbs,
    /// seen at level 0, is `continue`'s step or, for `break`, the loop's end;
    /// everything else leaves the loop with the body's completion. A plan
    /// whose protocol absorbs nothing leaves on every completion but the
    /// normal one.
    #[must_use]
    pub fn step(&self, completion: &CompletionOutcome) -> LoopStep {
        let absorbed: &[CompletionCode] = match &self.completion {
            CompletionProtocol::Absorb(codes) => codes,
            _ => &[],
        };
        match completion {
            CompletionOutcome::Normal => LoopStep::Next,
            CompletionOutcome::Code { code, level: 0, .. } if absorbed.contains(code) => {
                if *code == CompletionCode::Break {
                    LoopStep::Exit
                } else {
                    LoopStep::Skip
                }
            }
            CompletionOutcome::Code { .. } | CompletionOutcome::Error { .. } => LoopStep::Leave,
        }
    }
}

impl LoopResult {
    /// The loop's result once it completes normally, given what its
    /// iterations yielded in order — the body's result of each iteration
    /// that stepped [`LoopStep::Next`] — rendered as `target` renders a list.
    /// A collection is exact when every yielded value is, and the target
    /// decides the rendering (a first element that starts with `#` is quoted
    /// from 8.5); otherwise it is an unproven list.
    #[must_use]
    pub fn of(
        self,
        yielded: &[ExactValueOrUnavailable],
        target: &TargetSemantics,
    ) -> ExactValueOrUnavailable {
        match self {
            Self::Empty => ExactValueOrUnavailable::exact_text(""),
            Self::Collected => yielded
                .iter()
                .map(|value| match value {
                    ExactValueOrUnavailable::Exact(value) => value.as_str().ok(),
                    ExactValueOrUnavailable::Unavailable(_) => None,
                })
                .collect::<Option<Vec<&str>>>()
                .and_then(|elements| target.render_list(&elements))
                .map_or_else(
                    || {
                        ExactValueOrUnavailable::Unavailable(FactBounds {
                            existence: Existence::Bound(BindingKind::Scalar),
                            intrep: Some(TclType::List),
                            shape: None,
                            segments: None,
                            taint: None,
                        })
                    },
                    ExactValueOrUnavailable::exact_text,
                ),
            Self::Unstated => ExactValueOrUnavailable::unproven_string(),
        }
    }
}

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
                    completion: CompletionProtocol::Absorb(LOOP_ABSORBED),
                    result: self.result(),
                })
            }
            InvocationLayout::Source => self.source_plan(input),
        }
    }
}

impl IterationSemantics {
    /// What the loop yields: `lmap` collects its body's results, `foreach`
    /// yields the empty string.
    const fn result(self) -> LoopResult {
        if self.collects {
            LoopResult::Collected
        } else {
            LoopResult::Empty
        }
    }

    /// The plan of `foreach varList list body` in its source layout: one
    /// binder per name of the var-list word, padded with the empty string
    /// past the list's end, over the one list, the body run in the caller's
    /// frame with `break` and `continue` absorbed, and no binder bound when
    /// the list is empty. Several var-list and list pairs step several
    /// iterables in lockstep, which one plan does not describe; a var-list
    /// the analysis does not know exactly names no binders.
    fn source_plan(self, input: &dyn AnalysisInputs) -> PlanAnswer {
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
            completion: CompletionProtocol::Absorb(LOOP_ABSORBED),
            result: self.result(),
        })
    }
}

#[cfg(test)]
mod tests {
    use tcl_dialect::DialectProfile;

    use super::*;

    fn plan(completion: CompletionProtocol, result: LoopResult) -> IterationPlan {
        IterationPlan {
            binders: Vec::new(),
            iterable: IterableKind::List(OperandId(0)),
            body: None,
            exit: ExitRule::Exhaustion,
            zero_iterations_bind: false,
            completion,
            result,
        }
    }

    fn code(code: CompletionCode, level: u32) -> CompletionOutcome {
        CompletionOutcome::Code {
            code,
            level,
            result: ExactValueOrUnavailable::exact_text(""),
        }
    }

    /// A loop goes on after a normal completion and after `continue`, ends
    /// normally at `break`, and leaves with every other completion of its
    /// body: an error, a `return`, a code of the body's own, and a `break` or
    /// `continue` at a level above 0, which the loop sees as a `return`. A
    /// protocol that absorbs nothing leaves on every code.
    #[test]
    fn the_loop_rule_reads_each_completion_of_its_body() {
        let foreach = plan(CompletionProtocol::Absorb(LOOP_ABSORBED), LoopResult::Empty);
        assert_eq!(foreach.step(&CompletionOutcome::Normal), LoopStep::Next);
        assert_eq!(
            foreach.step(&code(CompletionCode::Continue, 0)),
            LoopStep::Skip
        );
        assert_eq!(
            foreach.step(&code(CompletionCode::Break, 0)),
            LoopStep::Exit
        );
        for (left, level) in [
            (CompletionCode::Break, 1),
            (CompletionCode::Continue, 1),
            (CompletionCode::Return, 1),
            (CompletionCode::Ok, 1),
            (CompletionCode::from_int(5), 0),
        ] {
            assert_eq!(
                foreach.step(&code(left, level)),
                LoopStep::Leave,
                "{left:?} at level {level}"
            );
        }
        assert_eq!(
            foreach.step(&CompletionOutcome::error_unproven(0)),
            LoopStep::Leave
        );
        let none = plan(CompletionProtocol::TclBody, LoopResult::Empty);
        assert_eq!(none.step(&CompletionOutcome::Normal), LoopStep::Next);
        assert_eq!(none.step(&code(CompletionCode::Break, 0)), LoopStep::Leave);
        assert_eq!(
            none.step(&code(CompletionCode::Continue, 0)),
            LoopStep::Leave
        );
    }

    /// `foreach` yields the empty string whatever its iterations yielded;
    /// `lmap` the list of them, rendered as the target renders a list — a
    /// first element that starts with `#` is quoted from 8.5 and bare in 8.4,
    /// which a profile naming no release cannot say — and an unproven list
    /// when one of them is unproven; a pack's loop states none.
    #[test]
    fn a_loop_yields_what_its_result_rule_states() {
        let exact = ExactValueOrUnavailable::exact_text;
        let tcl86 = TargetSemantics::of(DialectProfile::find("tcl8.6"));
        let tcl84 = TargetSemantics::of(DialectProfile::find("tcl8.4"));
        let unnamed = TargetSemantics::of(Some(DialectProfile::plain_tcl()));
        assert_eq!(LoopResult::Empty.of(&[exact("a")], &tcl86), exact(""));
        assert_eq!(
            LoopResult::Collected.of(&[exact("1"), exact("a b"), exact("")], &tcl86),
            exact("1 {a b} {}")
        );
        assert_eq!(LoopResult::Collected.of(&[], &tcl86), exact(""));
        assert_eq!(
            LoopResult::Collected.of(&[exact("#x"), exact("y")], &tcl86),
            exact("{#x} y")
        );
        assert_eq!(
            LoopResult::Collected.of(&[exact("#x"), exact("y")], &tcl84),
            exact("#x y")
        );
        for unproven in [
            LoopResult::Collected.of(&[exact("#x")], &unnamed),
            LoopResult::Collected.of(
                &[exact("1"), ExactValueOrUnavailable::unproven_string()],
                &tcl86,
            ),
            LoopResult::Unstated.of(&[], &tcl86),
        ] {
            assert!(
                matches!(unproven, ExactValueOrUnavailable::Unavailable(_)),
                "{unproven:?}"
            );
        }
    }
}
