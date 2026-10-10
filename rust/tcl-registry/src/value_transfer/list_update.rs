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

//! The list cell updates — `lset`, `ledit` (from 9.0) and `lpop` (from 9.0) —
//! over the shared list cores both runtimes' adapters call
//! (`tcl_cmd_core::list::{lset, ledit, lpop}`): each reads the list one
//! variable holds and writes one value back, `lset` and `ledit` the new list as
//! their result too, `lpop` the element it removed.
//!
//! The variable must hold a value: each command reads it first and raises
//! `can't read …: no such variable` when it is absent, which is never a value,
//! so a prior the analysis proves unbound, or cannot prove, declines. The
//! release is the core's to read — `lset`'s append at an index equal to a
//! level's length is 8.6's, and an index's grammar is each release's — so
//! under a target that names no release the answer stands only where every
//! release gives it.

use tcl_dialect::TclVersion;
use tcl_dialect::model::SpecSurface;

use crate::arg_role::ArgRole;
use crate::completion::{CompletionCode, CompletionCodeDomain};
use crate::types::TclType;

use super::CommandSemantics;
use super::answers::{
    BindingKind, CompletionOutcome, CompletionPath, DependencyEvidence, EvalAnswer, ExactValue,
    ExactValueOrUnavailable, ExistenceOutcome, ExistenceTransfer, InvocationOutcome, RouteIdentity,
    StoreOutcome, TransferAnswer, TypeFacts,
};
use super::const_ops::{ConstOps, ConstValue, Needs, TargetSemantics};
use super::context::Budget;
use super::decline::{Axis, DeclineReason};
use super::destructure::unavailable;
use super::inputs::{AnalysisInputs, FactDomain, OperandId, TargetId};
use super::publication::stopped;
use super::route::{EvalRoute, NativeEvalId};

const NORMAL: &[CompletionCode] = &[CompletionCode::Ok];

/// The revision of the registry-owned list cell updates.
const REVISION: u64 = 1;

/// A list cell update.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListUpdate {
    /// `lset listVar ?index …? newValue`.
    Set,
    /// `ledit listVar first last ?element …?`.
    Edit,
    /// `lpop listVar ?index …?`.
    Pop,
}

/// The list cell update specialisation for one operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListUpdateSemantics {
    operation: ListUpdate,
}

/// `lset`.
pub static LSET: ListUpdateSemantics = ListUpdateSemantics {
    operation: ListUpdate::Set,
};

/// `ledit`.
pub static LEDIT: ListUpdateSemantics = ListUpdateSemantics {
    operation: ListUpdate::Edit,
};

/// `lpop`.
pub static LPOP: ListUpdateSemantics = ListUpdateSemantics {
    operation: ListUpdate::Pop,
};

/// What one release computes: the result and the value written back.
type Computed = Result<(ConstValue, ConstValue), DeclineReason>;

/// What one release answers: the result and the value written back, or why
/// it has none, with the raised outcome a named release reports.
type Agreed = (
    Result<(ExactValue, ExactValue), DeclineReason>,
    Option<EvalAnswer>,
);

impl ListUpdateSemantics {
    /// The operation.
    #[must_use]
    pub const fn operation(self) -> ListUpdate {
        self.operation
    }

    /// The axes the cores read: an index's grammar and how a list renders.
    pub const NEEDS: Needs = Needs::INDEX_GRAMMAR.union(Needs::LIST_RENDERING);

    /// The catalogued evaluator.
    #[must_use]
    pub const fn evaluator(self) -> NativeEvalId {
        match self.operation {
            ListUpdate::Set => NativeEvalId::ListSet,
            ListUpdate::Edit => NativeEvalId::ListEdit,
            ListUpdate::Pop => NativeEvalId::ListPop,
        }
    }

    /// The first release with the command.
    const fn since(self) -> TclVersion {
        match self.operation {
            ListUpdate::Set => TclVersion::V8_4,
            ListUpdate::Edit | ListUpdate::Pop => TclVersion::V9_0,
        }
    }

    /// The list operand and the words after it, when the invocation is a
    /// shape the operation takes.
    fn layout(self, input: &dyn AnalysisInputs) -> Option<(TargetId, Vec<OperandId>)> {
        let view = input.invocation();
        let target = view.operands_with_role(ArgRole::VarWrite).next()?;
        let rest: Vec<OperandId> = (target.0 + 1..view.operands.len()).map(OperandId).collect();
        let fits = match self.operation {
            ListUpdate::Set => !rest.is_empty(),
            ListUpdate::Edit => rest.len() >= 2,
            ListUpdate::Pop => true,
        };
        fits.then_some((TargetId(target), rest))
    }

    fn type_facts(self, target: TargetId) -> TypeFacts {
        TypeFacts {
            result: Some(match self.operation {
                ListUpdate::Set | ListUpdate::Edit => TclType::List,
                ListUpdate::Pop => TclType::String,
            }),
            per_target: vec![(target, TclType::List)],
            shapes: Vec::new(),
        }
    }

    /// The update under `release` over the list `prior` holds, with the
    /// words after it.
    fn compute(
        self,
        ops: &mut ConstOps<'_>,
        prior: &ConstValue,
        words: &[ConstValue],
        release: TclVersion,
    ) -> Computed {
        let updated = match self.operation {
            ListUpdate::Set => {
                let (value, indices) = words.split_last().ok_or(DeclineReason::Unsupported)?;
                tcl_cmd_core::list::lset(ops, prior, indices, value.clone(), release)
            }
            ListUpdate::Edit => {
                let [first, last, elements @ ..] = words else {
                    return Err(DeclineReason::Unsupported);
                };
                tcl_cmd_core::list::ledit(ops, prior, first, last, elements, release)
            }
            ListUpdate::Pop => {
                return tcl_cmd_core::list::lpop(ops, prior, words, release)
                    .map_err(|error| ops.decline(&error));
            }
        };
        let updated = updated.map_err(|error| ops.decline(&error))?;
        Ok((updated.clone(), updated))
    }

    fn evaluate_update(self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        let Some((target, rest)) = self.layout(input) else {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        };
        let place = match input.place(target.0) {
            Ok(place) => place,
            Err(reason) => return EvalAnswer::Declined(reason),
        };
        // An absent variable is the command's `can't read` error, and an
        // unknown one could be absent: only a proven value is read.
        let prior = match input.prior_store(&place, FactDomain::ExactValue).exact() {
            Ok(prior) => ConstValue::from_exact(&prior),
            Err(answer) => return answer,
        };
        let mut words = Vec::with_capacity(rest.len());
        for id in rest {
            match input.operand(id, FactDomain::ExactValue).exact() {
                Ok(value) => words.push(ConstValue::from_exact(&value)),
                Err(answer) => return answer,
            }
        }
        let target_semantics = TargetSemantics::of(input.context().profile);
        let named = target_semantics.release;
        let releases: Vec<TclVersion> = match named {
            Some(release) if release < self.since() => {
                return EvalAnswer::Declined(unavailable(SpecSurface::TCL90_PLUS));
            }
            Some(release) => vec![release],
            None if self.since() > TclVersion::V8_4 => {
                return EvalAnswer::Declined(unavailable(SpecSurface::TCL90_PLUS));
            }
            None => TclVersion::ALL.to_vec(),
        };
        // Under a target that names no release, the answer stands only where
        // every release computes it.
        let mut agreed: Option<Agreed> = None;
        for release in releases {
            let mut ops = match ConstOps::admit(input.context(), budget, Self::NEEDS) {
                Ok(ops) => ops,
                Err(reason) => return EvalAnswer::Declined(reason),
            };
            let outcome: Agreed = match self.compute(&mut ops, &prior, &words, release) {
                Ok((result, written)) => match ops.take_all(vec![result, written]) {
                    Ok(mut taken) => {
                        let written = taken.pop().expect("two values taken");
                        let result = taken.pop().expect("two values taken");
                        (Ok((result, written)), None)
                    }
                    Err(reason) => (Err(reason), None),
                },
                Err(reason) => {
                    let raised = stopped(&mut ops, reason, self.evaluator(), REVISION);
                    (Err(reason), Some(raised))
                }
            };
            match &agreed {
                None => agreed = Some(outcome),
                Some(earlier) if earlier.0 == outcome.0 => {}
                Some(_) => {
                    return EvalAnswer::Declined(DeclineReason::ReleaseAmbiguous(
                        Axis::IndexGrammar,
                    ));
                }
            }
        }
        let Some((answer, raised)) = agreed else {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        };
        let (result, written) = match answer {
            Ok(values) => values,
            // The program's own error, when the run proved one under a named
            // release; every other failure is a decline.
            Err(reason) => {
                return match (named, raised) {
                    (Some(_), Some(raised)) => raised,
                    _ => EvalAnswer::Declined(reason),
                };
            }
        };
        EvalAnswer::Evaluated(Box::new(InvocationOutcome {
            completion: CompletionOutcome::Normal,
            nested_writes: Vec::new(),
            result: ExactValueOrUnavailable::Exact(result),
            ordered_stores: vec![StoreOutcome::Write {
                target,
                value: written,
            }],
            types: self.type_facts(target),
            evidence: DependencyEvidence {
                route: Some(RouteIdentity {
                    route: self.route(),
                    implementation: self.identity(),
                    revision: REVISION,
                }),
                numerals: target_semantics.numerals,
                release: named,
                ..DependencyEvidence::default()
            },
        }))
    }
}

impl CommandSemantics for ListUpdateSemantics {
    fn identity(&self) -> &'static str {
        match self.operation {
            ListUpdate::Set => "list-update:set",
            ListUpdate::Edit => "list-update:edit",
            ListUpdate::Pop => "list-update:pop",
        }
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: self.evaluator(),
        }
    }

    /// The list operand: every list cell update reads the list it rewrites.
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
            FactDomain::Type => TransferAnswer::Type(self.type_facts(target)),
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
