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

//! What the writing routes share: opening an evaluation over exact words,
//! and publishing a result with its ordered stores — the values charged,
//! taken from one value model, and typed as they were built.

use crate::arg_role::ArgRole;
use crate::types::TclType;

use super::answers::{
    CompletionOutcome, DependencyEvidence, EvalAnswer, ExactValueOrUnavailable, InvocationOutcome,
    RepresentationEvidence, RouteIdentity, StoreOutcome, TypeFacts,
};
use super::builtins::exact_operands;
use super::const_ops::{ConstOps, ConstValue, Needs};
use super::context::Budget;
use super::decline::DeclineReason;
use super::inputs::{AnalysisInputs, OperandId, TargetId};
use super::route::{EvalRoute, NativeEvalId};

/// Every operand's exact word as text the target reads alike, the operands
/// the resolver gives the `VarWrite` role, and a value model admitted for
/// `needs` — or the answer that stands in for an input that is not exact.
pub(super) fn open_words<'b>(
    input: &dyn AnalysisInputs,
    budget: &'b mut Budget,
    needs: Needs,
) -> Result<(ConstOps<'b>, Vec<String>, Vec<OperandId>), EvalAnswer> {
    let words = exact_operands(input, 0..input.invocation().operands.len())?;
    let targets: Vec<OperandId> = input
        .invocation()
        .operands_with_role(ArgRole::VarWrite)
        .collect();
    let mut ops = ConstOps::admit(input.context(), budget, needs).map_err(EvalAnswer::Declined)?;
    let texts = words
        .iter()
        .map(|word| ops.admissible_text(word).map(|text| text.to_string()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(EvalAnswer::Declined)?;
    Ok((ops, texts, targets))
}

/// Whether the resolver's `VarWrite` operands are exactly the operands from
/// `first` on, `count` of them, in order: the places a destructuring core
/// writes by position. When they are not, the stores would land on other
/// places than the command's, so the route declines.
pub(super) fn targets_are(targets: &[OperandId], first: usize, count: usize) -> bool {
    targets.len() == count
        && targets
            .iter()
            .enumerate()
            .all(|(at, id)| id.0 == first + at)
}

/// One store before its value is taken.
pub(super) enum PendingStore {
    /// The target's place holds the value afterwards.
    Write(TargetId, ConstValue),
    /// The element `key` of the target's array holds the value afterwards.
    WriteElement(TargetId, String, ConstValue),
    /// The target's place is untouched.
    Preserve(TargetId),
}

impl PendingStore {
    fn value(&self) -> Option<&ConstValue> {
        match self {
            Self::Write(_, value) | Self::WriteElement(_, _, value) => Some(value),
            Self::Preserve(_) => None,
        }
    }
}

/// What one evaluation publishes, before its values are taken: the result
/// and each store in execution order.
pub(super) struct Publication {
    /// The command result.
    pub(super) result: ConstValue,
    /// The result's type.
    pub(super) result_type: TclType,
    /// The stores, in execution order.
    pub(super) stores: Vec<PendingStore>,
}

impl Publication {
    /// A result and a `Preserve` for every declared target: the call wrote
    /// nothing.
    pub(super) fn preserving(
        result: ConstValue,
        result_type: TclType,
        targets: &[OperandId],
    ) -> Self {
        Self {
            result,
            result_type,
            stores: targets
                .iter()
                .map(|id| PendingStore::Preserve(TargetId(*id)))
                .collect(),
        }
    }

    /// Charge the published bytes as work — one unit per byte a result or a
    /// store carries — take every value, and build the outcome on the route
    /// `id` at `revision`: each write typed as the value it built.
    pub(super) fn publish(
        self,
        mut ops: ConstOps<'_>,
        id: NativeEvalId,
        revision: u64,
    ) -> EvalAnswer {
        let target = *ops.target();
        let bytes = self.result.bytes.len()
            + self
                .stores
                .iter()
                .filter_map(PendingStore::value)
                .map(|value| value.bytes.len())
                .sum::<usize>();
        if let Err(reason) = ops.charge(u64::try_from(bytes).unwrap_or(u64::MAX)) {
            return EvalAnswer::Declined(reason);
        }
        let written: Vec<ConstValue> = self
            .stores
            .iter()
            .filter_map(PendingStore::value)
            .cloned()
            .collect();
        let mut taken = match ops.take_all(std::iter::once(self.result).chain(written).collect()) {
            Ok(taken) => taken.into_iter(),
            Err(reason) => return EvalAnswer::Declined(reason),
        };
        let Some(result) = taken.next() else {
            return EvalAnswer::Declined(DeclineReason::MalformedAnswer);
        };
        let mut ordered_stores = Vec::with_capacity(self.stores.len());
        let mut per_target = Vec::new();
        for store in self.stores {
            let (target, key) = match store {
                PendingStore::Preserve(target) => {
                    ordered_stores.push(StoreOutcome::Preserve { target });
                    continue;
                }
                PendingStore::Write(target, _) => (target, None),
                PendingStore::WriteElement(target, key, _) => (target, Some(key)),
            };
            let Some(value) = taken.next() else {
                return EvalAnswer::Declined(DeclineReason::MalformedAnswer);
            };
            match key {
                None => {
                    if let RepresentationEvidence::Constructed(built) = value.representation {
                        per_target.push((target, built));
                    }
                    ordered_stores.push(StoreOutcome::Write { target, value });
                }
                // The type facts are per target, and an element write's
                // target is the whole array: the value's own evidence
                // speaks for the element.
                Some(key) => ordered_stores.push(StoreOutcome::WriteElement { target, key, value }),
            }
        }
        EvalAnswer::Evaluated(Box::new(InvocationOutcome {
            completion: CompletionOutcome::Normal,
            result: ExactValueOrUnavailable::Exact(result),
            ordered_stores,
            types: TypeFacts {
                result: Some(self.result_type),
                per_target,
                shapes: Vec::new(),
            },
            evidence: DependencyEvidence {
                route: Some(RouteIdentity {
                    route: EvalRoute::Direct { id },
                    implementation: id.as_str(),
                    revision,
                }),
                characters: target.character_model,
                release: target.release,
                ..DependencyEvidence::default()
            },
        }))
    }
}
