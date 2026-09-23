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
//! evaluator ([`STRING_RANGE`], [`LIST_OF_ARGS`], [`LIST_LENGTH`],
//! [`STRING_LENGTH`]) is a call into the shared core over [`ConstOps`]; a
//! transitional one ([`NativeEvalId::owner`], [`FORMAT_TEMPLATE`] until
//! slice 3) is run by the compiler's value-transfer driver as today's fold
//! until the shared cores replace it, and the migration plan's ledger names
//! it with its expiry. The expression route ([`EXPR`]) assembles its
//! arguments here ([`ExpressionRoute::assemble`]) and is evaluated by the
//! driver's engine adapter, which feeds the shared engine the analysis
//! services.

use crate::types::TclType;

use super::CommandSemantics;
use super::answers::{
    CompletionOutcome, DependencyEvidence, EvalAnswer, ExactValue, ExactValueOrUnavailable,
    InvocationOutcome, RouteIdentity, TransferAnswer, TypeFacts,
};
use super::const_ops::{ConstOps, ConstValue, Needs, TargetSemantics};
use super::context::Budget;
use super::decline::DeclineReason;
use super::inputs::{AnalysisInputs, FactDomain, OperandId, WordPart};
use super::route::{EvalRoute, LanguageProfileId, NativeEvalId};

/// The revision of the registry-owned `string range` evaluator.
const STRING_RANGE_REVISION: u64 = 1;

/// The revision of the registry-owned list and length evaluators: 1 is the
/// shared cores over `ConstOps`, replacing the compiler's transitional
/// folds.
const LIST_AND_LENGTH_REVISION: u64 = 1;

/// Operand `index`'s exact value, or the answer that stands in for one that
/// is not ([`super::inputs::FactView::exact`]).
fn exact_operand(input: &dyn AnalysisInputs, index: usize) -> Result<ExactValue, EvalAnswer> {
    input
        .operand(OperandId(index), FactDomain::ExactValue)
        .exact()
}

/// Operands `range`'s exact values, in order.
fn exact_operands(
    input: &dyn AnalysisInputs,
    range: std::ops::Range<usize>,
) -> Result<Vec<ConstValue>, EvalAnswer> {
    range
        .map(|index| exact_operand(input, index).map(|value| ConstValue::from_exact(&value)))
        .collect()
}

/// The outcome of a pure direct route: its result, no store, and the
/// evidence it rests on.
fn pure_outcome(
    id: NativeEvalId,
    revision: u64,
    value: ExactValue,
    result_type: TclType,
    evidence: DependencyEvidence,
) -> EvalAnswer {
    EvalAnswer::Evaluated(Box::new(InvocationOutcome {
        completion: CompletionOutcome::Normal,
        result: ExactValueOrUnavailable::Exact(value),
        ordered_stores: Vec::new(),
        types: TypeFacts {
            result: Some(result_type),
            per_target: Vec::new(),
            shapes: Vec::new(),
        },
        evidence: DependencyEvidence {
            route: Some(RouteIdentity {
                route: EvalRoute::Direct { id },
                implementation: id.as_str(),
                revision,
            }),
            ..evidence
        },
    }))
}

/// Run `compute` over a value model admitted for `needs`: the exact value
/// it returns, or the first recorded fault.
fn run_core(
    input: &dyn AnalysisInputs,
    budget: &mut Budget,
    needs: Needs,
    compute: impl FnOnce(&mut ConstOps<'_>) -> Result<ConstValue, DeclineReason>,
) -> Result<(ExactValue, TargetSemantics), DeclineReason> {
    let mut ops = ConstOps::admit(input.context(), budget, needs)?;
    let target = *ops.target();
    let value = compute(&mut ops)?;
    ops.take(value).map(|value| (value, target))
}

/// The type transfer of a route that writes nothing: its result type.
fn result_type_transfer(domain: FactDomain, result_type: TclType) -> TransferAnswer {
    match domain {
        FactDomain::Type => TransferAnswer::Type(TypeFacts {
            result: Some(result_type),
            per_target: Vec::new(),
            shapes: Vec::new(),
        }),
        _ => TransferAnswer::Generic,
    }
}

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

    fn evaluate_range(input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        // Operand 0 is the subcommand word.
        if input.invocation().operands.len() != 4 {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        }
        let exact = match exact_operands(input, 1..4) {
            Ok(exact) => exact,
            Err(answer) => return answer,
        };
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

/// `list ?arg …?` on the direct route: the list core over [`ConstOps`],
/// the arguments rendered as one canonical list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListOfArgsSemantics;

/// `list ?arg …?`.
pub static LIST_OF_ARGS: ListOfArgsSemantics = ListOfArgsSemantics;

impl ListOfArgsSemantics {
    /// The axis the core reads: how a list result is quoted.
    pub const NEEDS: Needs = Needs::LIST_RENDERING;
}

impl CommandSemantics for ListOfArgsSemantics {
    fn identity(&self) -> &'static str {
        NativeEvalId::ListOfArgs.as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: NativeEvalId::ListOfArgs,
        }
    }

    fn transfer(
        &self,
        domain: FactDomain,
        _input: &dyn AnalysisInputs,
        _budget: &mut Budget,
    ) -> TransferAnswer {
        result_type_transfer(domain, TclType::List)
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        let args = match exact_operands(input, 0..input.invocation().operands.len()) {
            Ok(args) => args,
            Err(answer) => return answer,
        };
        match run_core(input, budget, Self::NEEDS, |ops| {
            Ok(tcl_cmd_core::list::list(ops, &args))
        }) {
            Ok((value, target)) => pure_outcome(
                NativeEvalId::ListOfArgs,
                LIST_AND_LENGTH_REVISION,
                value,
                TclType::List,
                DependencyEvidence {
                    release: target.release,
                    ..DependencyEvidence::default()
                },
            ),
            Err(reason) => EvalAnswer::Declined(reason),
        }
    }
}

/// `llength list` on the direct route: the list core's element count over
/// the list parse, charged per element. A value that is not a list is the
/// program's error, never a count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListLengthSemantics;

/// `llength list`.
pub static LIST_LENGTH: ListLengthSemantics = ListLengthSemantics;

impl ListLengthSemantics {
    /// No axis: counting a list's elements reads nothing release-dependent.
    pub const NEEDS: Needs = Needs::NONE;
}

impl CommandSemantics for ListLengthSemantics {
    fn identity(&self) -> &'static str {
        NativeEvalId::ListLength.as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: NativeEvalId::ListLength,
        }
    }

    fn transfer(
        &self,
        domain: FactDomain,
        _input: &dyn AnalysisInputs,
        _budget: &mut Budget,
    ) -> TransferAnswer {
        result_type_transfer(domain, TclType::Int)
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        if input.invocation().operands.len() != 1 {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        }
        let list = match exact_operands(input, 0..1) {
            Ok(mut args) => args.remove(0),
            Err(answer) => return answer,
        };
        match run_core(input, budget, Self::NEEDS, |ops| {
            tcl_cmd_core::list::llength(ops, &list).map_err(|error| ops.decline(&error))
        }) {
            Ok((value, target)) => pure_outcome(
                NativeEvalId::ListLength,
                LIST_AND_LENGTH_REVISION,
                value,
                TclType::Int,
                DependencyEvidence {
                    release: target.release,
                    ..DependencyEvidence::default()
                },
            ),
            Err(reason) => EvalAnswer::Declined(reason),
        }
    }
}

/// `string length string` on the direct route: the string core's character
/// count under the target's character model, with a non-ASCII operand
/// admitted only where the target decodes source as UTF-8 (`string length
/// héllo` from a UTF-8 file is 6 up to 8.6 and 5 from 9.0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringLengthSemantics;

/// `string length string`.
pub static STRING_LENGTH: StringLengthSemantics = StringLengthSemantics;

impl StringLengthSemantics {
    /// The axes the core reads.
    pub const NEEDS: Needs = Needs::CHAR_MODEL.union(Needs::SOURCE_ENCODING);
}

impl CommandSemantics for StringLengthSemantics {
    fn identity(&self) -> &'static str {
        NativeEvalId::StringLength.as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: NativeEvalId::StringLength,
        }
    }

    fn transfer(
        &self,
        domain: FactDomain,
        _input: &dyn AnalysisInputs,
        _budget: &mut Budget,
    ) -> TransferAnswer {
        result_type_transfer(domain, TclType::Int)
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        // Operand 0 is the subcommand word.
        if input.invocation().operands.len() != 2 {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        }
        let subject = match exact_operands(input, 1..2) {
            Ok(mut args) => args.remove(0),
            Err(answer) => return answer,
        };
        match run_core(input, budget, Self::NEEDS, |ops| {
            ops.admissible_text(&subject)?;
            Ok(tcl_cmd_core::string::length(ops, &subject))
        }) {
            Ok((value, target)) => pure_outcome(
                NativeEvalId::StringLength,
                LIST_AND_LENGTH_REVISION,
                value,
                TclType::Int,
                DependencyEvidence {
                    characters: target.character_model,
                    release: target.release,
                    ..DependencyEvidence::default()
                },
            ),
            Err(reason) => EvalAnswer::Declined(reason),
        }
    }
}

/// A specialisation that declares a catalogued direct route and a result
/// type, and nothing else: the route's evaluator is the compiler's
/// transitional handler ([`NativeEvalId::owner`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectRoute {
    /// The catalogued evaluator.
    pub id: NativeEvalId,
    /// The result's internal representation.
    pub result_type: TclType,
}

/// `format template ?arg …?`.
pub static FORMAT_TEMPLATE: DirectRoute = DirectRoute {
    id: NativeEvalId::FormatTemplate,
    result_type: TclType::String,
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
        result_type_transfer(domain, self.result_type)
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

/// A BPF-Tcl expression. The route never evaluates in the analyser: see
/// [`ExpressionRoute::assemble`].
pub static BPF_EXPR: ExpressionRoute = ExpressionRoute {
    language: LanguageProfileId::BpfExpr,
};

/// What `expr`'s argument words assemble to, as the command specifies:
/// one braced word is the expression text, whose `$name` reads the
/// engine performs through `variable`; any other word reaches the
/// engine already substituted; several words join with one space.
#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionSource {
    /// One braced word: its text, and where the text starts in the word's
    /// source (one past the opening brace), for anchoring a position of
    /// the parsed expression back to the word.
    Braced {
        /// The expression text.
        text: String,
        /// The text's byte offset in the operand's source word.
        base: usize,
    },
    /// Any other shape: every word's substituted value, joined with one
    /// space. `expr` parses the result as an expression, so its `$name`
    /// reads and `[…]` scripts run a second time — Tcl's double
    /// substitution of an unbraced argument.
    Substituted(ExactValue),
}

impl ExpressionSource {
    /// The expression text the engine parses.
    ///
    /// # Errors
    ///
    /// `NotText` when an assembled value's bytes are not text.
    pub fn text(&self) -> Result<&str, DeclineReason> {
        match self {
            Self::Braced { text, .. } => Ok(text),
            Self::Substituted(value) => value.as_str(),
        }
    }
}

impl ExpressionRoute {
    /// The revision of the expression route's evaluation: 1 is the shared
    /// engine's full value under the analysis services.
    pub const REVISION: u64 = 1;

    /// Assemble the invocation's argument words into the expression the
    /// engine evaluates.
    ///
    /// Only Tcl's own arithmetic is evaluated here: under
    /// [`LanguageProfileId::BpfExpr`] the route declines `Unsupported`,
    /// because BPF-Tcl's signed division truncates towards zero (`-7 / 2`
    /// is `-3`) where Tcl floors (`-4` on every release from 8.4 to 9.1),
    /// and the BPF arithmetic adapter is the BPF frontend's
    /// (`docs/design/compiler/ebpf-backend.md`): a Tcl engine result is
    /// never a BPF constant.
    ///
    /// # Errors
    ///
    /// `Pending` while an argument word is pending; `Declined` with the
    /// reason an argument word is not an exact value (a finite set the
    /// lift could not pin is `CorrelatedSets`), or `Unsupported` for no
    /// argument at all — the program's `wrong # args` — and for a language
    /// the engine does not evaluate.
    pub fn assemble(&self, input: &dyn AnalysisInputs) -> Result<ExpressionSource, EvalAnswer> {
        if self.language != LanguageProfileId::TclExpr {
            return Err(EvalAnswer::Declined(DeclineReason::Unsupported));
        }
        let words = input.invocation().operands.len();
        if words == 0 {
            return Err(EvalAnswer::Declined(DeclineReason::Unsupported));
        }
        let braced = if words == 1 {
            input
                .word_structure(OperandId(0))
                .ok()
                .filter(|structure| structure.braced)
        } else {
            None
        };
        if let Some(structure) = braced {
            let value = exact_operand(input, 0)?;
            let text = value.as_str().map_err(EvalAnswer::Declined)?.to_owned();
            let base = match structure.parts.as_slice() {
                [WordPart::Literal { span, .. }] => span.start() as usize,
                _ => 1,
            };
            return Ok(ExpressionSource::Braced { text, base });
        }
        let mut values = (0..words).map(|index| exact_operand(input, index));
        let first = values
            .next()
            .unwrap_or(Err(EvalAnswer::Declined(DeclineReason::Unsupported)))?;
        let mut rest = values.peekable();
        if rest.peek().is_none() {
            return Ok(ExpressionSource::Substituted(first));
        }
        let mut bytes = first.bytes;
        for value in rest {
            bytes.push(b' ');
            bytes.extend_from_slice(&value?.bytes);
        }
        Ok(ExpressionSource::Substituted(ExactValue {
            bytes,
            numeric: None,
            representation: super::answers::RepresentationEvidence::Unknown,
        }))
    }
}

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

    /// The `$name` operands of the assembled expression, element-qualified
    /// (`a(k)` for a constant key), parsed under the context's profile. An
    /// argument that is not yet an exact value reads nothing here: the
    /// lift counts that operand itself.
    fn variable_reads(&self, input: &dyn AnalysisInputs) -> Vec<String> {
        let Ok(source) = self.assemble(input) else {
            return Vec::new();
        };
        let Ok(text) = source.text() else {
            return Vec::new();
        };
        let profile = input.context().profile;
        let node = tcl_syntax::expr::parser::parse_expr_for_profile(text, profile);
        let mut reads: Vec<String> = node
            .vars_element_qualified_with_config(tcl_lexer::LexerConfig::for_profile(profile))
            .into_iter()
            .collect();
        reads.sort_unstable();
        reads
    }
}
