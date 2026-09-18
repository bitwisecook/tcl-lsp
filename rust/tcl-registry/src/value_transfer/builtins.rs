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

//! The shipped value-position specialisations.
//!
//! Each declaration names its route — a catalogued direct evaluator or the
//! shared expression engine — and its result type. A registry-owned direct
//! evaluator ([`STRING_RANGE`]) is a call into the shared core over
//! [`ConstOps`]; a transitional one ([`NativeEvalId::owner`]) is run by
//! the compiler's value-transfer driver as today's fold until the shared
//! cores replace it, and the migration plan's ledger names each with its
//! expiry. The expression route is run by the driver's engine adapter by
//! construction; the registry-owned argument assembly lands with the
//! expression slice.

use crate::types::TclType;

use super::CommandSemantics;
use super::answers::{
    CompletionOutcome, DependencyEvidence, EvalAnswer, ExactValue, ExactValueOrUnavailable,
    InvocationOutcome, RouteIdentity, TransferAnswer, TypeFacts,
};
use super::const_ops::{ConstOps, ConstValue, Needs};
use super::context::Budget;
use super::decline::DeclineReason;
use super::inputs::{AnalysisInputs, FactDomain, FactView, OperandId};
use super::route::{EvalRoute, LanguageProfileId, NativeEvalId};

/// The revision of the registry-owned `string range` evaluator.
const STRING_RANGE_REVISION: u64 = 1;

/// `string range string first last` on the direct route: the shared string
/// core over [`ConstOps`], with the index numerals pre-resolved under the
/// admitted grammar (`string range abcdefghijkl 010 end` is `ijkl` up to
/// 8.6 and `kl` from 9.0, and declines with no release named) and a
/// non-ASCII operand admitted only where the target decodes source as
/// UTF-8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringRangeSemantics;

/// `string range string first last`.
pub static STRING_RANGE: StringRangeSemantics = StringRangeSemantics;

impl StringRangeSemantics {
    /// The axes the core reads.
    pub const NEEDS: Needs = Needs::INDEX_GRAMMAR
        .union(Needs::CHAR_INDEXING)
        .union(Needs::SOURCE_ENCODING);

    fn exact_operand(input: &dyn AnalysisInputs, index: usize) -> Result<ExactValue, EvalAnswer> {
        match input.operand(OperandId(index), FactDomain::ExactValue) {
            FactView::Pending => Err(EvalAnswer::Pending),
            FactView::Exact(value, _) => Ok(value),
            FactView::Finite(..) => Err(EvalAnswer::Declined(DeclineReason::CorrelatedSets)),
            FactView::Domain(_) => Err(EvalAnswer::Declined(DeclineReason::MalformedAnswer)),
            FactView::Top(reason) => Err(EvalAnswer::Declined(reason)),
        }
    }

    fn evaluate_range(input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        // Operand 0 is the subcommand word.
        if input.invocation().operands.len() != 4 {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        }
        let mut exact = Vec::with_capacity(3);
        for index in 1..4 {
            match Self::exact_operand(input, index) {
                Ok(value) => exact.push(ConstValue::from_exact(&value)),
                Err(answer) => return answer,
            }
        }
        let [subject, first, last] = exact.as_slice() else {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        };
        let mut ops = match ConstOps::admit(input.context(), budget, Self::NEEDS) {
            Ok(ops) => ops,
            Err(reason) => return EvalAnswer::Declined(reason),
        };
        let target = *ops.target();
        let computed = ops.admissible_text(subject).and_then(|text| {
            // The addressing unit is the Unicode scalar in both runtimes at
            // every release; under a release that decodes source another
            // way only ASCII reached here, where scalars and code units
            // agree.
            let len = text.chars().count();
            let first = ops.index(first, len)?;
            let last = ops.index(last, len)?;
            tcl_cmd_core::string::range(&mut ops, subject, &first, &last)
                .map_err(|error| ops.decline(&error))
        });
        let value = match computed.and_then(|value| ops.take(value)) {
            Ok(value) => value,
            Err(reason) => return EvalAnswer::Declined(reason),
        };
        EvalAnswer::Evaluated(Box::new(InvocationOutcome {
            completion: CompletionOutcome::Normal,
            result: ExactValueOrUnavailable::Exact(value),
            ordered_stores: Vec::new(),
            types: TypeFacts {
                result: Some(TclType::String),
                per_target: Vec::new(),
                shapes: Vec::new(),
            },
            evidence: DependencyEvidence {
                route: Some(RouteIdentity {
                    route: EvalRoute::Direct {
                        id: NativeEvalId::StringRange,
                    },
                    implementation: NativeEvalId::StringRange.as_str(),
                    revision: STRING_RANGE_REVISION,
                }),
                numerals: target.numerals,
                release: target.release,
                ..DependencyEvidence::default()
            },
        }))
    }
}

impl CommandSemantics for StringRangeSemantics {
    fn identity(&self) -> &'static str {
        NativeEvalId::StringRange.as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: NativeEvalId::StringRange,
        }
    }

    fn transfer(
        &self,
        domain: FactDomain,
        _input: &dyn AnalysisInputs,
        _budget: &mut Budget,
    ) -> TransferAnswer {
        match domain {
            FactDomain::Type => TransferAnswer::Type(TypeFacts {
                result: Some(TclType::String),
                per_target: Vec::new(),
                shapes: Vec::new(),
            }),
            _ => TransferAnswer::Generic,
        }
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        Self::evaluate_range(input, budget)
    }
}

/// A specialisation that declares a catalogued direct route and a result
/// type, and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectRoute {
    /// The catalogued evaluator.
    pub id: NativeEvalId,
    /// The result's internal representation.
    pub result_type: TclType,
}

/// `list ?arg …?`.
pub static LIST_OF_ARGS: DirectRoute = DirectRoute {
    id: NativeEvalId::ListOfArgs,
    result_type: TclType::List,
};

/// `format template ?arg …?`.
pub static FORMAT_TEMPLATE: DirectRoute = DirectRoute {
    id: NativeEvalId::FormatTemplate,
    result_type: TclType::String,
};

/// `llength list`.
pub static LIST_LENGTH: DirectRoute = DirectRoute {
    id: NativeEvalId::ListLength,
    result_type: TclType::Int,
};

/// `string length string`.
pub static STRING_LENGTH: DirectRoute = DirectRoute {
    id: NativeEvalId::StringLength,
    result_type: TclType::Int,
};

impl CommandSemantics for DirectRoute {
    fn identity(&self) -> &'static str {
        self.id.as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct { id: self.id }
    }

    fn transfer(
        &self,
        domain: FactDomain,
        _input: &dyn AnalysisInputs,
        _budget: &mut Budget,
    ) -> TransferAnswer {
        match domain {
            FactDomain::Type => TransferAnswer::Type(TypeFacts {
                result: Some(self.result_type),
                per_target: Vec::new(),
                shapes: Vec::new(),
            }),
            _ => TransferAnswer::Generic,
        }
    }
}

/// A specialisation that declares the shared expression engine under a
/// language profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpressionRoute {
    /// The language whose arithmetic the engine runs under.
    pub language: LanguageProfileId,
}

/// `expr arg ?arg …?`.
pub static EXPR: ExpressionRoute = ExpressionRoute {
    language: LanguageProfileId::TclExpr,
};

impl CommandSemantics for ExpressionRoute {
    fn identity(&self) -> &'static str {
        match self.language {
            LanguageProfileId::TclExpr => "expression:tcl",
            LanguageProfileId::BpfExpr => "expression:bpf",
        }
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Expression {
            language: self.language,
        }
    }
}
