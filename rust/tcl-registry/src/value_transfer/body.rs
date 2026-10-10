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

//! The body commands of `dict`: `dict with` and `dict update` as structural
//! plans (`docs/design/compiler/value-transfers.md` § *Storage-writing
//! commands*: "`dict with` and `dict update` bind keys, run a body, and
//! reconcile").
//!
//! Each answers `PlanAnswer::Body`: the names bound on body entry, the body
//! in the caller's frame, the write-back of the bound keys into the
//! dictionary variable, and the body's completion as the command's. The
//! binders are a projection on body entry — the proven keys of the
//! dictionary for `dict with`, the declared variables for `dict update` —
//! never the command's value, which is the body's result and has no route.

use crate::frame_effect::FrameLevel;

use super::CommandSemantics;
use super::answers::{
    Binder, BinderName, BindingKind, BodyPlan, CompletionProtocol, PlanAnswer, Reconcile,
};
use super::decline::{DeclineReason, NoRouteReason};
use super::inputs::{AnalysisInputs, FactDomain, FactView, OperandId};
use super::route::EvalRoute;

/// The distinct keys of the dictionary `text` spells, in first-occurrence
/// order, or `None` when it is not a dictionary (an odd element count, or
/// not a list).
fn dict_keys(text: &str) -> Option<Vec<String>> {
    let elements = tcl_syntax::list::split_list(text).ok()?;
    if elements.len() % 2 != 0 {
        return None;
    }
    let mut keys: Vec<String> = Vec::with_capacity(elements.len() / 2);
    for [key, _] in elements.as_chunks::<2>().0 {
        if !keys.iter().any(|held| held == key.as_ref()) {
            keys.push(key.to_string());
        }
    }
    Some(keys)
}

/// The value the dictionary at `path` holds within `text`, the last value
/// of a repeated key winning, or `None` when a step is not a dictionary or
/// lacks its key — which the command raises on.
fn dict_at(text: &str, path: &[String]) -> Option<String> {
    let mut current = text.to_owned();
    for key in path {
        let elements = tcl_syntax::list::split_list(&current).ok()?;
        if elements.len() % 2 != 0 {
            return None;
        }
        let value = elements
            .as_chunks::<2>()
            .0
            .iter()
            .rev()
            .find(|[held, _]| held.as_ref() == key)
            .map(|[_, value]| value.to_string())?;
        current = value;
    }
    Some(current)
}

/// The text of the exact value `view` holds, or the decline that stands in
/// for one it does not: an input the solver has not reached, a finite set
/// or an unknown value is no proven key set.
fn exact_text(view: FactView) -> Result<String, DeclineReason> {
    match view {
        FactView::Exact(value, _) => {
            String::from_utf8(value.bytes).map_err(|_| DeclineReason::NotText)
        }
        FactView::Top(reason) => Err(reason),
        FactView::Pending | FactView::Finite(..) | FactView::Domain(_) => {
            Err(DeclineReason::NotExact)
        }
    }
}

/// The body plan both commands share: the body as the last operand, run in
/// the caller's frame, the bound keys written back into the dictionary
/// operand, and the body's completion as the command's.
fn body_plan(binders: Vec<Binder>, dict: OperandId, body: OperandId) -> PlanAnswer {
    PlanAnswer::Body {
        binders,
        body: BodyPlan {
            body,
            frame: FrameLevel::Relative(0),
        },
        reconcile: Reconcile::WriteBackKeys(dict),
        completion: CompletionProtocol::TclBody,
    }
}

/// `dict with dictionaryVariable ?key …? body`: every key of the dictionary
/// the variable holds (or of the one the key path names within it) is a
/// variable of the body's scope on entry, and each is written back into the
/// dictionary when the body completes. The binders are the dictionary's
/// proven keys; a dictionary the analysis does not know exactly binds keys
/// no plan can name, so the plan declines and the generic transfer stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DictWithSemantics;

/// `dict with` and `::tcl::dict::with`.
pub static DICT_WITH: DictWithSemantics = DictWithSemantics;

impl CommandSemantics for DictWithSemantics {
    fn identity(&self) -> &'static str {
        "body:dict-with"
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::None {
            reason: NoRouteReason::Unauthored,
        }
    }

    fn structure(&self, input: &dyn AnalysisInputs) -> PlanAnswer {
        let view = input.invocation();
        let first = view.argument_offset;
        let Some(last) = view
            .operands
            .len()
            .checked_sub(1)
            .filter(|&last| last > first)
        else {
            return PlanAnswer::Declined(DeclineReason::WrongRepresentation);
        };
        let dict = OperandId(first);
        let place = match input.place(dict) {
            Ok(place) => place,
            Err(reason) => return PlanAnswer::Declined(reason),
        };
        let path = match (first + 1..last)
            .map(|index| exact_text(input.operand(OperandId(index), FactDomain::ExactValue)))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(path) => path,
            Err(reason) => return PlanAnswer::Declined(reason),
        };
        let value = match exact_text(input.prior_store(&place, FactDomain::ExactValue)) {
            Ok(value) => value,
            Err(reason) => return PlanAnswer::Declined(reason),
        };
        let Some(keys) = dict_at(&value, &path).as_deref().and_then(dict_keys) else {
            return PlanAnswer::Declined(DeclineReason::WrongRepresentation);
        };
        body_plan(
            keys.into_iter()
                .map(|key| Binder {
                    name: BinderName::Declared(key),
                    kind: BindingKind::Scalar,
                })
                .collect(),
            dict,
            OperandId(last),
        )
    }
}

/// `dict update dictionaryVariable key varName ?key varName …? body`: each
/// `varName` is a variable of the body's scope on entry — bound to its
/// key's value, or left unbound where the dictionary lacks the key — and
/// each is written back into the dictionary when the body completes. The
/// binders are the declared variables, whatever the dictionary holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DictUpdateSemantics;

/// `dict update` and `::tcl::dict::update`.
pub static DICT_UPDATE: DictUpdateSemantics = DictUpdateSemantics;

impl CommandSemantics for DictUpdateSemantics {
    fn identity(&self) -> &'static str {
        "body:dict-update"
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::None {
            reason: NoRouteReason::Unauthored,
        }
    }

    fn structure(&self, input: &dyn AnalysisInputs) -> PlanAnswer {
        let view = input.invocation();
        let first = view.argument_offset;
        // The dictionary, one key and variable pair at least, and the body.
        let words = view.operands.len().saturating_sub(first);
        if words < 4 || !words.is_multiple_of(2) {
            return PlanAnswer::Declined(DeclineReason::WrongRepresentation);
        }
        let last = view.operands.len() - 1;
        body_plan(
            (first + 2..last)
                .step_by(2)
                .map(|index| Binder {
                    name: BinderName::Operand(OperandId(index)),
                    kind: BindingKind::Scalar,
                })
                .collect(),
            OperandId(first),
            OperandId(last),
        )
    }
}
