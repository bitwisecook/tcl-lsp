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

//! The keyed updates of the dictionary one variable holds — `dict set`,
//! `unset`, `incr`, `append`, `lappend`, and their `::tcl::dict::`
//! spellings — over the shared dict cores
//! (`docs/design/compiler/value-transfers-migration.md` § *Third-party
//! commands*, Tier 1).
//!
//! The dictionary operand is the one the resolver gives the `VarWrite`
//! role, so the subcommand form (operand 0 is the subcommand word) and the
//! qualified command share one declaration. The key path and the value
//! follow it. Each update reads the prior dictionary, rebuilds it through
//! `tcl_cmd_core::dict` over [`ConstOps`], and writes it back: the new
//! dictionary is the result and the one store, rendered canonically. A
//! prior that is not a dictionary, a missing intermediate key of `dict
//! unset`, and an increment that is not an integer are the program's
//! errors, never values. An absent variable is the empty dictionary — all
//! five create it — but only once the inputs prove it absent: a prior the
//! solver cannot prove declines.

use tcl_syntax::value::ValueOps;

use crate::arg_role::ArgRole;
use crate::completion::{CompletionCode, CompletionCodeDomain};
use crate::types::TclType;

use super::CommandSemantics;
use super::answers::{
    BindingKind, CompletionOutcome, CompletionPath, DependencyEvidence, EvalAnswer, ExactValue,
    ExactValueOrUnavailable, Existence, ExistenceOutcome, ExistenceTransfer, InvocationOutcome,
    RouteIdentity, StoreOutcome, TransferAnswer, TypeFacts,
};
use super::const_ops::{ConstOps, ConstValue, Needs};
use super::context::Budget;
use super::decline::DeclineReason;
use super::inputs::{AnalysisInputs, DomainFact, FactDomain, FactView, OperandId, TargetId};
use super::route::{EvalRoute, NativeEvalId};

const NORMAL: &[CompletionCode] = &[CompletionCode::Ok];

/// The revision of the registry-owned keyed-update evaluators.
const REVISION: u64 = 1;

/// A keyed update of the dictionary one variable holds, over
/// `tcl_cmd_core::dict`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyedUpdate {
    /// `dict set dictionaryVariable key ?key …? value`.
    Set,
    /// `dict unset dictionaryVariable key ?key …?`.
    Unset,
    /// `dict incr dictionaryVariable key ?increment?`.
    Increment,
    /// `dict append dictionaryVariable key ?string …?`.
    Append,
    /// `dict lappend dictionaryVariable key ?value …?`.
    ListAppend,
}

/// The keyed-update specialisation for one operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyedUpdateSemantics {
    operation: KeyedUpdate,
}

/// `dict set`.
pub static DICT_SET: KeyedUpdateSemantics = KeyedUpdateSemantics {
    operation: KeyedUpdate::Set,
};

/// `dict unset`.
pub static DICT_UNSET: KeyedUpdateSemantics = KeyedUpdateSemantics {
    operation: KeyedUpdate::Unset,
};

/// `dict incr`.
pub static DICT_INCR: KeyedUpdateSemantics = KeyedUpdateSemantics {
    operation: KeyedUpdate::Increment,
};

/// `dict append`.
pub static DICT_APPEND: KeyedUpdateSemantics = KeyedUpdateSemantics {
    operation: KeyedUpdate::Append,
};

/// `dict lappend`.
pub static DICT_LAPPEND: KeyedUpdateSemantics = KeyedUpdateSemantics {
    operation: KeyedUpdate::ListAppend,
};

impl KeyedUpdateSemantics {
    /// The operation.
    #[must_use]
    pub const fn operation(self) -> KeyedUpdate {
        self.operation
    }

    /// The release axes the operation's cores read: the dictionary's
    /// canonical order and rendering, and for an increment the numeral
    /// grammar and the integer tower.
    #[must_use]
    pub const fn needs(self) -> Needs {
        let dict = Needs::DICT_ORDER.union(Needs::LIST_RENDERING);
        match self.operation {
            KeyedUpdate::Increment => dict.union(Needs::NUMERAL_GRAMMAR).union(Needs::INT_TOWER),
            KeyedUpdate::Set
            | KeyedUpdate::Unset
            | KeyedUpdate::Append
            | KeyedUpdate::ListAppend => dict,
        }
    }

    /// The catalogued evaluator.
    #[must_use]
    pub const fn evaluator(self) -> NativeEvalId {
        match self.operation {
            KeyedUpdate::Set => NativeEvalId::DictSet,
            KeyedUpdate::Unset => NativeEvalId::DictUnset,
            KeyedUpdate::Increment => NativeEvalId::DictIncr,
            KeyedUpdate::Append => NativeEvalId::DictAppend,
            KeyedUpdate::ListAppend => NativeEvalId::DictListAppend,
        }
    }

    /// The dictionary operand and the operands after it, when the
    /// invocation is a shape the operation takes.
    fn layout(self, input: &dyn AnalysisInputs) -> Option<(TargetId, Vec<OperandId>)> {
        let view = input.invocation();
        let target = view.operands_with_role(ArgRole::VarWrite).next()?;
        let rest: Vec<OperandId> = (target.0 + 1..view.operands.len()).map(OperandId).collect();
        let fits = match self.operation {
            KeyedUpdate::Set => rest.len() >= 2,
            KeyedUpdate::Increment => rest.len() == 1 || rest.len() == 2,
            KeyedUpdate::Unset | KeyedUpdate::Append | KeyedUpdate::ListAppend => !rest.is_empty(),
        };
        fits.then_some((TargetId(target), rest))
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

    /// The dictionary the variable holds before the update: its exact
    /// value, or the empty dictionary when the inputs prove the variable
    /// unbound. An unknown prior is never taken for an absent one.
    fn prior_dictionary(
        input: &dyn AnalysisInputs,
        target: TargetId,
    ) -> Result<Option<ExactValue>, EvalAnswer> {
        let place = input.place(target.0).map_err(EvalAnswer::Declined)?;
        let unbound = |view: &FactView| {
            matches!(
                view,
                FactView::Domain(DomainFact::Existence(Existence::Unbound))
            )
        };
        match input.prior_store(&place, FactDomain::ExactValue) {
            view if unbound(&view) => Ok(None),
            FactView::Top(reason) => {
                if unbound(&input.prior_store(&place, FactDomain::Existence)) {
                    Ok(None)
                } else {
                    Err(EvalAnswer::Declined(reason))
                }
            }
            view => Self::exact_input(view).map(Some),
        }
    }

    fn evaluate_update(self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        let Some((target, rest)) = self.layout(input) else {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        };
        let prior = match Self::prior_dictionary(input, target) {
            Ok(prior) => prior,
            Err(answer) => return answer,
        };
        let mut words = Vec::with_capacity(rest.len());
        for id in rest {
            match Self::exact_input(input.operand(id, FactDomain::ExactValue)) {
                Ok(value) => words.push(ConstValue::from_exact(&value)),
                Err(answer) => return answer,
            }
        }
        let mut ops = match ConstOps::admit(input.context(), budget, self.needs()) {
            Ok(ops) => ops,
            Err(reason) => return EvalAnswer::Declined(reason),
        };
        let target_semantics = *ops.target();
        let dictionary = prior.map_or_else(|| ConstValue::text(""), |v| ConstValue::from_exact(&v));
        let computed = self.apply(&mut ops, dictionary, &words);
        let value = match computed.and_then(|value| ops.take(value)) {
            Ok(value) => value,
            Err(reason) => return EvalAnswer::Declined(reason),
        };
        EvalAnswer::Evaluated(Box::new(InvocationOutcome {
            completion: CompletionOutcome::Normal,
            result: ExactValueOrUnavailable::Exact(value.clone()),
            ordered_stores: vec![StoreOutcome::Write { target, value }],
            types: Self::type_facts(target),
            evidence: DependencyEvidence {
                route: Some(RouteIdentity {
                    route: self.route(),
                    implementation: self.identity(),
                    revision: REVISION,
                }),
                numerals: match self.operation {
                    KeyedUpdate::Increment => target_semantics.numerals,
                    KeyedUpdate::Set
                    | KeyedUpdate::Unset
                    | KeyedUpdate::Append
                    | KeyedUpdate::ListAppend => None,
                },
                release: target_semantics.release,
                ..DependencyEvidence::default()
            },
        }))
    }

    /// The new dictionary: the operation applied to `dictionary` with the
    /// words after the dictionary operand.
    fn apply(
        self,
        ops: &mut ConstOps<'_>,
        dictionary: ConstValue,
        words: &[ConstValue],
    ) -> Result<ConstValue, DeclineReason> {
        match self.operation {
            KeyedUpdate::Set => {
                let (value, keys) = words.split_last().ok_or(DeclineReason::Unsupported)?;
                set_path(ops, dictionary, keys, value.clone())
            }
            KeyedUpdate::Unset => unset_path(ops, dictionary, words),
            KeyedUpdate::Increment | KeyedUpdate::Append | KeyedUpdate::ListAppend => {
                let (key, values) = words.split_first().ok_or(DeclineReason::Unsupported)?;
                let mut pairs = ops
                    .dict_pairs(&dictionary)
                    .map_err(|error| ops.decline_value(&error))?;
                let key_text = ops.as_str(key);
                let old = tcl_cmd_core::dict::lookup(ops, &pairs, &key_text);
                let new_value = self.update_value(ops, old, values)?;
                tcl_cmd_core::dict::upsert(ops, &mut pairs, key, new_value);
                Ok(ops.new_dict(pairs))
            }
        }
    }

    /// The value one key holds after an increment, append, or list append,
    /// from the value it held (`None` when absent).
    fn update_value(
        self,
        ops: &mut ConstOps<'_>,
        old: Option<ConstValue>,
        values: &[ConstValue],
    ) -> Result<ConstValue, DeclineReason> {
        match self.operation {
            KeyedUpdate::Increment => match (old, values.first()) {
                (Some(old), step) => {
                    let step = step.cloned().unwrap_or_else(|| ConstValue::int(1));
                    ops.int_add(Some(&old), &step)
                        .map_err(|error| ops.decline_value(&error))
                }
                // An absent key takes the increment as written once it
                // proves to be an integer: `dict incr d k 010` stores
                // `010` in every release.
                (None, Some(step)) => ops
                    .int_add(None, step)
                    .map(|_| step.clone())
                    .map_err(|error| ops.decline_value(&error)),
                (None, None) => Ok(ConstValue::int(1)),
            },
            KeyedUpdate::Append => Ok(tcl_cmd_core::var::append_bytes(ops, old, values)),
            KeyedUpdate::ListAppend => tcl_cmd_core::var::lappend_value(ops, old, values)
                .map_err(|error| ops.decline(&error)),
            KeyedUpdate::Set | KeyedUpdate::Unset => Err(DeclineReason::Unsupported),
        }
    }

    fn type_facts(target: TargetId) -> TypeFacts {
        TypeFacts {
            result: Some(TclType::Dict),
            per_target: vec![(target, TclType::Dict)],
            shapes: Vec::new(),
        }
    }
}

/// `dict set`'s key path: each intermediate level is the dictionary under
/// its key (an absent key an empty one), and the levels rebuild bottom-up
/// with the value at the last key.
fn set_path(
    ops: &mut ConstOps<'_>,
    dictionary: ConstValue,
    keys: &[ConstValue],
    value: ConstValue,
) -> Result<ConstValue, DeclineReason> {
    let mut levels = Vec::with_capacity(keys.len());
    let mut node = dictionary;
    for key in keys {
        let pairs = ops
            .dict_pairs(&node)
            .map_err(|error| ops.decline_value(&error))?;
        let key_text = ops.as_str(key);
        let next =
            tcl_cmd_core::dict::lookup(ops, &pairs, &key_text).unwrap_or_else(|| ops.empty());
        levels.push((pairs, key));
        node = next;
    }
    Ok(rebuild(ops, levels, value))
}

/// `dict unset`'s key path: every intermediate key must be present — a
/// missing one is the program's error (`key "x" not known in
/// dictionary`) — and the last key is removed, present or not.
fn unset_path(
    ops: &mut ConstOps<'_>,
    dictionary: ConstValue,
    keys: &[ConstValue],
) -> Result<ConstValue, DeclineReason> {
    let (last, intermediate) = keys.split_last().ok_or(DeclineReason::Unsupported)?;
    let mut levels = Vec::with_capacity(intermediate.len());
    let mut node = dictionary;
    for key in intermediate {
        let pairs = ops
            .dict_pairs(&node)
            .map_err(|error| ops.decline_value(&error))?;
        let key_text = ops.as_str(key);
        let Some(next) = tcl_cmd_core::dict::lookup(ops, &pairs, &key_text) else {
            return Err(DeclineReason::WrongRepresentation);
        };
        levels.push((pairs, key));
        node = next;
    }
    let innermost = tcl_cmd_core::dict::remove(ops, &node, std::slice::from_ref(last))
        .map_err(|error| ops.decline(&error))?;
    Ok(rebuild(ops, levels, innermost))
}

/// Rebuild a key path bottom-up: each level takes the rebuilt level below
/// under its key.
fn rebuild(
    ops: &mut ConstOps<'_>,
    levels: Vec<(Vec<(ConstValue, ConstValue)>, &ConstValue)>,
    innermost: ConstValue,
) -> ConstValue {
    let mut value = innermost;
    for (mut pairs, key) in levels.into_iter().rev() {
        tcl_cmd_core::dict::upsert(ops, &mut pairs, key, value);
        value = ops.new_dict(pairs);
    }
    value
}

impl CommandSemantics for KeyedUpdateSemantics {
    fn identity(&self) -> &'static str {
        match self.operation {
            KeyedUpdate::Set => "keyed-update:set",
            KeyedUpdate::Unset => "keyed-update:unset",
            KeyedUpdate::Increment => "keyed-update:incr",
            KeyedUpdate::Append => "keyed-update:append",
            KeyedUpdate::ListAppend => "keyed-update:lappend",
        }
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: self.evaluator(),
        }
    }

    /// The dictionary operand: every keyed update reads the dictionary it
    /// rewrites.
    fn incoming_targets(&self, input: &dyn AnalysisInputs) -> Vec<TargetId> {
        self.layout(input)
            .map(|(target, _)| vec![target])
            .unwrap_or_default()
    }

    fn transfer(
        &self,
        domain: FactDomain,
        input: &dyn AnalysisInputs,
        _budget: &mut Budget,
    ) -> TransferAnswer {
        let Some((target, _)) = self.layout(input) else {
            return TransferAnswer::Declined(DeclineReason::Unsupported);
        };
        match domain {
            FactDomain::Type => TransferAnswer::Type(Self::type_facts(target)),
            FactDomain::Existence => TransferAnswer::Existence(ExistenceTransfer {
                paths: vec![CompletionPath {
                    completion: CompletionCodeDomain::Exact(NORMAL),
                    outcomes: vec![(target, ExistenceOutcome::Bind(BindingKind::Scalar))],
                }],
            }),
            _ => TransferAnswer::Generic,
        }
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        self.evaluate_update(input, budget)
    }
}
