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

use tcl_dialect::TclVersion;

use crate::arg_role::ArgRole;
use crate::completion::{CompletionCode, CompletionCodeDomain};
use crate::types::TclType;

use super::answers::{
    BindingKind, CompletionOutcome, CompletionPath, DependencyEvidence, EvalAnswer,
    ExactValueOrUnavailable, Existence, ExistenceOutcome, ExistenceTransfer, InvocationOutcome,
    RepresentationEvidence, RouteIdentity, StoreOutcome, TransferAnswer, TypeFacts,
};
use super::builtins::exact_operands;
use super::const_ops::{ConstOps, ConstValue, Needs, Raised, TargetSemantics};
use super::context::Budget;
use super::decline::DeclineReason;
use super::inputs::{AnalysisInputs, DomainFact, FactDomain, FactView, OperandId, TargetId};
use super::route::{EvalRoute, NativeEvalId};

/// What writing a scalar to a place that holds an array raises, by the
/// command making the write and the release: `set`, `append`, `lappend`,
/// `lassign` and `binary scan` say `can't set "b": variable is array`; `incr`
/// says `can't read` before 8.5; `scan`, `regexp` and `regsub` say `couldn't
/// set variable "b"` before 8.6. From 8.6 the `-errorcode` is `TCL WRITE
/// VARNAME`, before it `NONE`. Every command but `scan` raises at the write
/// it cannot make; `scan` makes every other and raises after the last, worded
/// by its first failure from 8.6 and by each of them in turn before (measured,
/// 8.4 to 9.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ArrayWrite {
    /// `set`, `append`, `lappend`, `lassign`, `binary scan`.
    Set,
    /// `incr`, which reads the variable first before 8.5.
    Incr,
    /// `regexp`, `regsub`.
    Match,
    /// `scan`, which goes on past a write it cannot make.
    Scan,
}

impl ArrayWrite {
    /// The message and `-errorcode` one write to the array `name` raises
    /// under `release`.
    fn worded(self, name: &str, release: TclVersion) -> (String, String) {
        let code = if release >= TclVersion::V8_6 {
            "TCL WRITE VARNAME"
        } else {
            "NONE"
        };
        let message = match self {
            Self::Incr if release < TclVersion::V8_5 => {
                format!("can't read \"{name}\": variable is array")
            }
            Self::Match | Self::Scan if release < TclVersion::V8_6 => {
                format!("couldn't set variable \"{name}\"")
            }
            Self::Set | Self::Incr | Self::Match | Self::Scan => {
                format!("can't set \"{name}\": variable is array")
            }
        };
        (message, code.to_owned())
    }

    /// The error the write to the array `name` raises.
    pub(super) fn raised(self, name: &str, target: &TargetSemantics) -> Raised {
        Raised::unanimous(target, |release| self.worded(name, release))
    }

    /// Whether the command goes on past a write it cannot make: `scan`
    /// assigns every variable it converted and reports after the last.
    const fn goes_on(self) -> bool {
        matches!(self, Self::Scan)
    }

    /// The error a command that goes on raises after its writes, where the
    /// writes it could not make were to the arrays `names`, in order: from 8.6
    /// the first failure's, and before it each failure's message in turn, run
    /// together (`couldn't set variable "a"couldn't set variable "c"`).
    fn raised_after(self, names: &[String], target: &TargetSemantics) -> Raised {
        Raised::unanimous(target, |release| {
            let mut worded = names.iter().map(|name| self.worded(name, release));
            let (mut message, code) = worded.next().unwrap_or_default();
            if release < TclVersion::V8_6 {
                for (next, _) in worded {
                    message.push_str(&next);
                }
            }
            (message, code)
        })
    }

    /// The array the target names, when the analysis proves the place holds
    /// one: a scalar write to it raises. Anything else — a place of another
    /// kind, an element, a fact the rung does not state — is no proof of an
    /// error, and the write stays the normal completion's.
    pub(super) fn array_place(input: &dyn AnalysisInputs, target: TargetId) -> Option<String> {
        let place = input.place(target.0).ok()?;
        if place.is_element() {
            return None;
        }
        matches!(
            input.prior_store(&place, FactDomain::Existence),
            FactView::Domain(DomainFact::Existence(Existence::Bound(BindingKind::Array)))
        )
        .then_some(place.name)
    }
}

const NORMAL: &[CompletionCode] = &[CompletionCode::Ok];
const ERROR: &[CompletionCode] = &[CompletionCode::Error];

/// The existence transfer of a command that stores to its `VarWrite`
/// operands in order, each store failing where its place holds an array
/// (`docs/design/compiler/value-transfers.md` § *`catch`, `try`, and
/// completion*). A place the rung proves unbound or a scalar takes the store;
/// one it proves an array raises there; any other kind — and an element,
/// which may fail on its array — may. The normal path binds every target, and
/// is absent where one is certainly an array. The error path is present where
/// a store may fail: every target before the first that is not proven
/// writable is bound, that target and the rest are may-bound, or untouched
/// where the first is certainly an array, which stops the command there.
pub(super) fn ordered_writes_transfer(input: &dyn AnalysisInputs) -> TransferAnswer {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Kind {
        Writable,
        Array,
        Unproven,
    }
    let mut targets: Vec<(TargetId, Kind)> = Vec::new();
    for id in input.invocation().operands_with_role(ArgRole::VarWrite) {
        let Ok(place) = input.place(id) else {
            return TransferAnswer::Generic;
        };
        let kind = if place.is_element() {
            Kind::Unproven
        } else {
            match input.prior_store(&place, FactDomain::Existence) {
                FactView::Domain(DomainFact::Existence(
                    Existence::Unbound | Existence::Bound(BindingKind::Scalar),
                )) => Kind::Writable,
                FactView::Domain(DomainFact::Existence(Existence::Bound(BindingKind::Array))) => {
                    Kind::Array
                }
                _ => Kind::Unproven,
            }
        };
        targets.push((TargetId(id), kind));
    }
    if targets.is_empty() {
        return TransferAnswer::Generic;
    }
    let mut paths = Vec::new();
    if targets.iter().all(|(_, kind)| *kind != Kind::Array) {
        paths.push(CompletionPath {
            completion: CompletionCodeDomain::Exact(NORMAL),
            outcomes: targets
                .iter()
                .map(|(target, _)| (*target, ExistenceOutcome::Bind(BindingKind::Scalar)))
                .collect(),
        });
    }
    if let Some(failing) = targets.iter().position(|(_, kind)| *kind != Kind::Writable) {
        let certain = targets[failing].1 == Kind::Array;
        let outcomes = targets
            .iter()
            .enumerate()
            .filter_map(|(at, (target, _))| {
                if at < failing {
                    Some((*target, ExistenceOutcome::Bind(BindingKind::Scalar)))
                } else if certain {
                    None
                } else {
                    Some((*target, ExistenceOutcome::MayBind(BindingKind::Scalar)))
                }
            })
            .collect();
        paths.push(CompletionPath {
            completion: CompletionCodeDomain::Exact(ERROR),
            outcomes,
        });
    }
    TransferAnswer::Existence(ExistenceTransfer { paths })
}

/// How a publication checks the kind of the places it writes: the inputs
/// that say what each holds, and the command's own wording of the failure.
#[derive(Clone, Copy)]
pub(super) struct Checked<'i> {
    /// The inputs the target kinds are read from.
    pub(super) input: &'i dyn AnalysisInputs,
    /// What a write to an array raises, for this command.
    pub(super) write: ArrayWrite,
}

/// What a route whose core failed answers: the error the program raises
/// ([`ConstOps::raised`]) as a completion that ran no store, or the decline
/// when the failure was a fault — an inadmissible axis, a budget — or no
/// core's error at all.
pub(super) fn stopped(
    ops: &mut ConstOps<'_>,
    reason: DeclineReason,
    id: NativeEvalId,
    revision: u64,
) -> EvalAnswer {
    let target = *ops.target();
    match ops.raised() {
        Some(raised) => raised_outcome(id, revision, &target, raised),
        None => EvalAnswer::Declined(reason),
    }
}

/// The outcome of a route that raised before any store ran: the error its
/// core reported, with no value and nothing written.
pub(super) fn raised_outcome(
    id: NativeEvalId,
    revision: u64,
    target: &TargetSemantics,
    raised: Raised,
) -> EvalAnswer {
    EvalAnswer::Evaluated(Box::new(InvocationOutcome {
        completion: CompletionOutcome::Error {
            written: 0,
            message: raised.message,
            error_code: raised.error_code,
        },
        nested_writes: Vec::new(),
        result: ExactValueOrUnavailable::unproven_string(),
        ordered_stores: Vec::new(),
        types: TypeFacts::default(),
        evidence: DependencyEvidence {
            route: Some(RouteIdentity {
                route: EvalRoute::Direct { id },
                implementation: id.as_str(),
                revision,
            }),
            numerals: None,
            characters: target.character_model,
            release: target.release,
            ..DependencyEvidence::default()
        },
    }))
}

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

/// The error `stores` raise where one writes a place `checked` proves holds
/// an array, as the count of the stores that ran and the error. A command
/// raises at the first such store, so the stores before it ran; `scan` goes on
/// ([`ArrayWrite::goes_on`]), so every store ran, each failing one a
/// `Preserve` now, and it raises after the last.
fn failing_stores(
    checked: Option<Checked<'_>>,
    stores: &mut [StoreOutcome],
    target: &TargetSemantics,
) -> Option<(usize, Raised)> {
    let checked = checked?;
    let failing: Vec<(usize, String)> = stores
        .iter()
        .enumerate()
        .filter_map(|(at, store)| {
            let StoreOutcome::Write { target: place, .. } = store else {
                return None;
            };
            Some((at, ArrayWrite::array_place(checked.input, *place)?))
        })
        .collect();
    let (first, name) = failing.first()?;
    if !checked.write.goes_on() {
        return Some((*first, checked.write.raised(name, target)));
    }
    let mut names = Vec::with_capacity(failing.len());
    for (at, name) in failing {
        let place = stores[at].target();
        stores[at] = StoreOutcome::Preserve { target: place };
        names.push(name);
    }
    Some((stores.len(), checked.write.raised_after(&names, target)))
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
    ///
    /// With `checked`, the first write to a place the analysis proves
    /// holds an array is the command's error: the stores before it ran and
    /// the rest did not, so the outcome is `Error { written }` with every
    /// store listed in order and the targets after the failing one left as
    /// they were (the prefix rule). `scan` goes on past such a write: every
    /// store ran, the failing ones preserve their places, and the error is
    /// raised after the last.
    pub(super) fn publish(
        self,
        mut ops: ConstOps<'_>,
        checked: Option<Checked<'_>>,
        id: NativeEvalId,
        revision: u64,
    ) -> EvalAnswer {
        let target_semantics = *ops.target();
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
        let failing = failing_stores(checked, &mut ordered_stores, &target_semantics);
        let (completion, result, result_type) = match failing {
            Some((written, raised)) => {
                per_target.retain(|(target, _)| {
                    ordered_stores[..written].iter().any(|store| {
                        store.target() == *target && !matches!(store, StoreOutcome::Preserve { .. })
                    })
                });
                (
                    CompletionOutcome::Error {
                        written,
                        message: raised.message,
                        error_code: raised.error_code,
                    },
                    ExactValueOrUnavailable::unproven_string(),
                    None,
                )
            }
            None => (
                CompletionOutcome::Normal,
                ExactValueOrUnavailable::Exact(result),
                Some(self.result_type),
            ),
        };
        EvalAnswer::Evaluated(Box::new(InvocationOutcome {
            completion,
            nested_writes: Vec::new(),
            result,
            ordered_stores,
            types: TypeFacts {
                result: result_type,
                per_target,
                shapes: Vec::new(),
            },
            evidence: DependencyEvidence {
                route: Some(RouteIdentity {
                    route: EvalRoute::Direct { id },
                    implementation: id.as_str(),
                    revision,
                }),
                characters: target_semantics.character_model,
                release: target_semantics.release,
                ..DependencyEvidence::default()
            },
        }))
    }
}
