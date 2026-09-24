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
//! shared expression engine — and its result type. Every direct evaluator
//! ([`STRING_RANGE`], [`LIST_OF_ARGS`], [`LIST_LENGTH`], [`STRING_LENGTH`],
//! [`FORMAT_TEMPLATE`], [`BINARY_FORMAT`]) is registry-owned: a call into the shared core over
//! [`ConstOps`]. The expression route ([`EXPR`]) assembles its arguments
//! here ([`ExpressionRoute::assemble`]) and is evaluated by the driver's
//! engine adapter, which feeds the shared engine the analysis services.

use crate::arg_role::ArgRole;
use crate::frame_effect::FrameLevel;
use crate::types::TclType;

use super::CommandSemantics;
use super::answers::{
    Binder, BinderName, BindingKind, BodyPlan, CompletionOutcome, CompletionPath,
    CompletionProtocol, DependencyEvidence, EvalAnswer, ExactValue, ExactValueOrUnavailable,
    ExistenceOutcome, ExistenceTransfer, ExitRule, InvocationOutcome, IterableKind, IterationPlan,
    PlanAnswer, RouteIdentity, TransferAnswer, TypeFacts,
};
use super::const_ops::{ConstOps, ConstValue, Needs, TargetSemantics};
use super::context::Budget;
use super::decline::{Axis, DeclineReason, NoRouteReason};
use super::inputs::{AnalysisInputs, FactDomain, InvocationLayout, OperandId, TargetId, WordPart};
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
pub(super) fn exact_operands(
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

/// `format template ?arg …?` on the direct route: the shared format core
/// (`tcl_cmd_core::format::format_cmd_with_syntax`) over [`ConstOps`] under
/// the target release's numeral grammar. A conversion the release lacks is
/// the program's error (`%b` from 8.6, `%p` and `%llu` from 9.0; tclsh 8.4
/// raises `bad field specifier "b"`). Under a profile that names no release
/// the answer is the one every modelled release gives — each release's run
/// must agree, so an unmodified `%d` past 32 bits, the `%#d` prefix or a
/// conversion one release lacks declines with
/// `ReleaseAmbiguous(FormatVerbs)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatTemplateSemantics;

/// `format template ?arg …?`.
pub static FORMAT_TEMPLATE: FormatTemplateSemantics = FormatTemplateSemantics;

impl FormatTemplateSemantics {
    /// The axes the core reads.
    pub const NEEDS: Needs = Needs::FORMAT_VERBS
        .union(Needs::NUMERAL_GRAMMAR)
        .union(Needs::INT_TOWER);

    /// The revision of the registry-owned evaluator: 1 is the shared core,
    /// replacing the compiler's transitional fold.
    const REVISION: u64 = 1;

    /// `args` rendered as `release` renders them.
    fn render(
        input: &dyn AnalysisInputs,
        budget: &mut Budget,
        args: &[ConstValue],
        gated: &[tcl_syntax::format::VersionGatedUse],
        release: tcl_dialect::TclVersion,
    ) -> Result<(ExactValue, TargetSemantics), DeclineReason> {
        if gated.iter().any(|gated| gated.min > release) {
            return Err(DeclineReason::WrongRepresentation);
        }
        run_core(input, budget, Self::NEEDS, |ops| {
            tcl_cmd_core::format::format_cmd_with_syntax(ops, args, release.number_syntax())
                .map_err(|error| ops.decline(&error))
        })
    }
}

impl CommandSemantics for FormatTemplateSemantics {
    fn identity(&self) -> &'static str {
        NativeEvalId::FormatTemplate.as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: NativeEvalId::FormatTemplate,
        }
    }

    fn transfer(
        &self,
        domain: FactDomain,
        _input: &dyn AnalysisInputs,
        _budget: &mut Budget,
    ) -> TransferAnswer {
        result_type_transfer(domain, TclType::String)
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        let words = input.invocation().operands.len();
        if words == 0 {
            // `wrong # args`: the program's error.
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        }
        let template = match exact_operand(input, 0) {
            Ok(template) => template,
            Err(answer) => return answer,
        };
        let gated = match template.as_str() {
            Ok(text) => tcl_syntax::format::version_gated_uses(text),
            Err(reason) => return EvalAnswer::Declined(reason),
        };
        let args = match exact_operands(input, 0..words) {
            Ok(args) => args,
            Err(answer) => return answer,
        };
        let named = TargetSemantics::of(input.context().profile).release;
        let releases = named.map_or_else(
            || tcl_dialect::TclVersion::ALL.to_vec(),
            |release| vec![release],
        );
        let mut agreed: Option<Result<(ExactValue, TargetSemantics), DeclineReason>> = None;
        for release in releases {
            let rendered = Self::render(input, budget, &args, &gated, release);
            match &agreed {
                None => agreed = Some(rendered),
                Some(earlier) => {
                    let same = match (earlier, &rendered) {
                        (Ok((a, _)), Ok((b, _))) => a == b,
                        (Err(a), Err(b)) => a == b,
                        _ => false,
                    };
                    if !same {
                        return EvalAnswer::Declined(DeclineReason::ReleaseAmbiguous(
                            Axis::FormatVerbs,
                        ));
                    }
                }
            }
        }
        match agreed {
            Some(Ok((value, target))) => pure_outcome(
                NativeEvalId::FormatTemplate,
                Self::REVISION,
                value,
                TclType::String,
                DependencyEvidence {
                    numerals: target.numerals,
                    release: target.release,
                    ..DependencyEvidence::default()
                },
            ),
            Some(Err(reason)) => EvalAnswer::Declined(reason),
            None => EvalAnswer::Declined(DeclineReason::Unsupported),
        }
    }
}

/// `binary format formatString ?arg …?` on the direct route: the shared
/// packer (`tcl_cmd_core::binary::format`) over the arguments' bytes, its
/// output bound (`binary::format_size_bound`) charged before it runs, and
/// the result a byte array by construction, spelt as the string of the
/// characters `U+0000` to `U+00FF` its bytes are. Only what every release
/// the target names packs alike is evaluated, each difference measured on
/// tclsh 8.4 to 9.1:
///
/// - a field letter from its release (`t n m r R q Q` from 8.5); the `u`
///   suffix, which the packer refuses, and every other packer error decline
///   as the program's error;
/// - an integer only as a plain decimal within 64 bits: `binary format c
///   010` is `\x08` up to 8.6 and `\x0a` from 9.0, `0b` and `0o` arrive in
///   8.5 and `1_0` in 9.0, and past 64 bits 8.x raises where 9.x wraps;
/// - a float only as a plain decimal whose value every release reads alike:
///   `d 010` is 10.0 on 8.4 and 9.x and 8.0 on 8.5 and 8.6, an integer
///   spelling reads as an integer from 8.5 (`d -0` is -0.0 on 8.4 and 0.0
///   after), past the double range or below its normal range 8.4 raises, and
///   a single-precision value past `FLT_MAX` is clamped by 8.x and packed as
///   an infinity by 9.x; a float field without a count takes its whole
///   argument, where the packer would take a list's first element;
/// - a character above `U+00FF` in an argument, which crosses to bytes by
///   the release's rule;
/// - `x*` and an `@` without a count, which C Tcl refuses and the packer
///   does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinaryFormatSemantics;

/// `binary format formatString ?arg …?`.
pub static BINARY_FORMAT: BinaryFormatSemantics = BinaryFormatSemantics;

impl BinaryFormatSemantics {
    /// The axes the route reads: the field set, the crossing of characters
    /// to bytes, and the source decoding of a non-ASCII operand.
    pub const NEEDS: Needs = Needs::BINARY_FIELDS
        .union(Needs::BYTE_STRINGS)
        .union(Needs::SOURCE_ENCODING);

    /// The revision of the registry-owned evaluator.
    const REVISION: u64 = 1;

    /// Whether `text` is an integer every release reads alike: a plain
    /// decimal, optionally signed and surrounded by whitespace, with no
    /// leading zero, within 64 bits.
    fn plain_integer(text: &str) -> bool {
        let text = text.trim_matches(|c: char| c.is_ascii_whitespace());
        let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
        !digits.is_empty()
            && digits.bytes().all(|b| b.is_ascii_digit())
            && (digits == "0" || !digits.starts_with('0'))
            && text.parse::<i64>().is_ok()
    }

    /// Whether `text` is a float every release reads as one value: a plain
    /// decimal mantissa (no leading zero before another digit) with an
    /// optional exponent, whose value is zero from zero digits or a normal
    /// double — within single precision for a single-precision field — and
    /// whose integer spelling, which 8.5 on reads as an integer first, is
    /// exact in a double and not a negative zero.
    fn plain_float(text: &str, single: bool) -> bool {
        let text = text.trim_matches(|c: char| c.is_ascii_whitespace());
        let unsigned = text.strip_prefix(['+', '-']).unwrap_or(text);
        let (mantissa, exponent) = match unsigned.find(['e', 'E']) {
            Some(at) => (&unsigned[..at], Some(&unsigned[at + 1..])),
            None => (unsigned, None),
        };
        let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
        let digits = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
        let exponent_ok = exponent.is_none_or(|exponent| {
            let exponent = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
            !exponent.is_empty() && digits(exponent)
        });
        if whole.len() + fraction.len() == 0
            || !digits(whole)
            || !digits(fraction)
            || !exponent_ok
            || (whole.len() > 1 && whole.starts_with('0'))
        {
            return false;
        }
        let Ok(value) = text.parse::<f64>() else {
            return false;
        };
        let zero_digits = mantissa.bytes().all(|b| matches!(b, b'0' | b'.'));
        let in_range = if value == 0.0 {
            zero_digits
        } else {
            value.is_finite()
                && value.abs() >= f64::MIN_POSITIVE
                && (!single || value.abs() <= f64::from(f32::MAX))
        };
        let integer_spelling = !mantissa.contains('.') && exponent.is_none();
        in_range
            && (!integer_spelling
                || (!(value == 0.0 && value.is_sign_negative())
                    && value.abs() <= 9_007_199_254_740_992.0))
    }

    /// The decline for a field, or a value one takes, that the target's
    /// releases do not all pack alike, when there is one. A format or an
    /// argument the packer refuses is left for the packer to refuse.
    fn field_decline(
        release: Option<tcl_dialect::TclVersion>,
        format: &str,
        args: &[String],
    ) -> Option<DeclineReason> {
        let mut next = args.iter();
        for field in tcl_cmd_core::binary::specifiers(format.as_bytes(), false) {
            if let Some(floor) = tcl_cmd_core::binary::specifier_min_version(field.letter)
                && release.is_none_or(|release| release < floor)
            {
                return Some(DeclineReason::ReleaseAmbiguous(Axis::Availability(
                    tcl_dialect::model::SpecSurface::TCL85_PLUS[0],
                )));
            }
            let counted = field.star || field.count.is_some();
            match field.letter {
                b'x' if field.star => return Some(DeclineReason::WrongRepresentation),
                b'@' if !counted => return Some(DeclineReason::WrongRepresentation),
                b'x' | b'X' | b'@' => {}
                b'a' | b'A' | b'b' | b'B' | b'h' | b'H' => {
                    next.next();
                }
                letter => {
                    // A missing argument is the packer's error to raise.
                    let arg = next.next()?;
                    let float = matches!(letter, b'f' | b'r' | b'R' | b'd' | b'q' | b'Q');
                    let single = matches!(letter, b'f' | b'r' | b'R');
                    let plain = |value: &str| {
                        if float {
                            Self::plain_float(value, single)
                        } else {
                            Self::plain_integer(value)
                        }
                    };
                    let readable = if counted {
                        let Ok(elements) = tcl_syntax::list::split_list(arg) else {
                            return None;
                        };
                        let used = field.count.unwrap_or(elements.len()).min(elements.len());
                        elements[..used].iter().all(|element| plain(element))
                    } else {
                        plain(arg)
                    };
                    if !readable {
                        return Some(DeclineReason::ReleaseAmbiguous(Axis::NumeralGrammar));
                    }
                }
            }
        }
        None
    }

    fn evaluate_format(input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        let view = input.invocation();
        let first = view.argument_offset;
        if view.operands.len() <= first {
            // `wrong # args`: the program's error.
            return EvalAnswer::Declined(DeclineReason::WrongRepresentation);
        }
        let words = match exact_operands(input, first..view.operands.len()) {
            Ok(words) => words,
            Err(answer) => return answer,
        };
        let mut ops = match ConstOps::admit(input.context(), budget, Self::NEEDS) {
            Ok(ops) => ops,
            Err(reason) => return EvalAnswer::Declined(reason),
        };
        let target = *ops.target();
        let texts = match words
            .iter()
            .map(|word| ops.admissible_text(word).map(|text| text.to_string()))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(texts) => texts,
            Err(reason) => return EvalAnswer::Declined(reason),
        };
        let Some((format, args)) = texts.split_first() else {
            return EvalAnswer::Declined(DeclineReason::WrongRepresentation);
        };
        if let Some(reason) = Self::field_decline(target.release, format, args) {
            return EvalAnswer::Declined(reason);
        }
        let Some(bytes) = args
            .iter()
            .map(|arg| {
                arg.chars()
                    .map(|c| u8::try_from(u32::from(c)).ok())
                    .collect::<Option<Vec<u8>>>()
            })
            .collect::<Option<Vec<Vec<u8>>>>()
        else {
            return EvalAnswer::Declined(DeclineReason::ReleaseAmbiguous(Axis::ByteStrings));
        };
        let refs: Vec<&[u8]> = bytes.iter().map(Vec::as_slice).collect();
        // The output is allocated before it is packed: its bound is charged
        // first, so a count no budget pays for is never allocated.
        let bound = tcl_cmd_core::binary::format_size_bound(format.as_bytes(), &refs);
        if let Err(reason) = ops.charge_bytes(bound) {
            return EvalAnswer::Declined(reason);
        }
        let Ok(packed) = tcl_cmd_core::binary::format(format.as_bytes(), &refs) else {
            return EvalAnswer::Declined(DeclineReason::WrongRepresentation);
        };
        if let Err(reason) = ops.charge(u64::try_from(packed.len()).unwrap_or(u64::MAX)) {
            return EvalAnswer::Declined(reason);
        }
        let text: String = packed.iter().copied().map(char::from).collect();
        let value = ConstValue::bytes(text.as_bytes(), super::const_ops::Representation::ByteArray);
        match ops.take(value) {
            Ok(value) => pure_outcome(
                NativeEvalId::BinaryFormat,
                Self::REVISION,
                value,
                TclType::ByteArray,
                DependencyEvidence {
                    release: target.release,
                    ..DependencyEvidence::default()
                },
            ),
            Err(reason) => EvalAnswer::Declined(reason),
        }
    }
}

impl CommandSemantics for BinaryFormatSemantics {
    fn identity(&self) -> &'static str {
        NativeEvalId::BinaryFormat.as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: NativeEvalId::BinaryFormat,
        }
    }

    fn transfer(
        &self,
        domain: FactDomain,
        _input: &dyn AnalysisInputs,
        _budget: &mut Budget,
    ) -> TransferAnswer {
        result_type_transfer(domain, TclType::ByteArray)
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        Self::evaluate_format(input, budget)
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

/// A command declared to write its named targets with no route to
/// evaluate the written value: `evaluate` declines with the given
/// reason, and the driver's conservative fallback — the same one an
/// undeclared write already took — is what widens each target's value
/// (VT5.14; the classification is new, the lattice answer is not). Its
/// existence transfer is a may-bind of each target, as the kind the
/// command writes: the place may be bound afterwards, and never loses a
/// binding it had. A command that may also unbind its target declares no
/// kind and keeps the generic transfer, whose widening to may-bound is
/// that answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MayWriteSemantics {
    /// The roles whose operands this call may write.
    pub targets: &'static [ArgRole],
    /// Why no route reads the written value.
    pub reason: NoRouteReason,
    /// What the call binds a target as; `None` when it may unbind the
    /// target instead.
    pub kind: Option<BindingKind>,
}

/// `file stat name varName`: an array of the file's attributes.
pub static FILE_STAT: MayWriteSemantics = MayWriteSemantics {
    targets: &[ArgRole::VarWrite],
    reason: NoRouteReason::Platform,
    kind: Some(BindingKind::Array),
};

/// `file lstat name varName`: the same platform-decided array as `stat`.
pub static FILE_LSTAT: MayWriteSemantics = FILE_STAT;

/// `file tempfile ?nameVar? ?template?`: the platform names the file, a
/// scalar.
pub static FILE_TEMPFILE: MayWriteSemantics = MayWriteSemantics {
    targets: &[ArgRole::VarWrite],
    reason: NoRouteReason::Platform,
    kind: Some(BindingKind::Scalar),
};

/// `gets channelId ?varName?` / `chan gets channelId ?varName?`: the
/// channel's next line is a value the source decides.
pub static GETS: MayWriteSemantics = MayWriteSemantics {
    targets: &[ArgRole::VarWrite],
    reason: NoRouteReason::Declared,
    kind: Some(BindingKind::Scalar),
};

/// `array default exists|get|set|unset arrayName ?value?` (from 9.0): `set`
/// makes the name an array when it is absent (`array default set d 7`
/// leaves an empty array `d`), the other verbs keep the place, and none
/// writes an element's value.
pub static ARRAY_DEFAULT: MayWriteSemantics = MayWriteSemantics {
    targets: &[ArgRole::VarWrite],
    reason: NoRouteReason::Declared,
    kind: Some(BindingKind::Array),
};

/// `vwait varName`: the event loop writes `varName` from whichever event
/// fires first. The wait also ends on an unset (`Tcl_VwaitObjCmd` traces
/// `TCL_TRACE_UNSETS` too: `set x 1; after 0 {unset x}; vwait x` leaves no
/// `x` on 8.4 to 9.1), so it declares no kind and its existence transfer
/// stays generic.
pub static VWAIT: MayWriteSemantics = MayWriteSemantics {
    targets: &[ArgRole::VarWrite],
    reason: NoRouteReason::Declared,
    kind: None,
};

/// `tk_optionMenu pathName varName value ?value ...?`: the widget writes
/// `varName` from the option the user picks.
pub static TK_OPTION_MENU: MayWriteSemantics = MayWriteSemantics {
    targets: &[ArgRole::VarWrite],
    reason: NoRouteReason::Declared,
    kind: Some(BindingKind::Scalar),
};

/// `trace add|remove|variable|vdelete … commandPrefix`: the traced place
/// becomes externally mutable through the callback the call installs —
/// `NoRouteReason::Callback`, the same reason `regsub -command` declares.
/// A traced variable may be a scalar or an array.
pub static TRACE: MayWriteSemantics = MayWriteSemantics {
    targets: &[ArgRole::VarWrite],
    reason: NoRouteReason::Callback,
    kind: Some(BindingKind::Either),
};

impl CommandSemantics for MayWriteSemantics {
    fn identity(&self) -> &'static str {
        "may_write"
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::None {
            reason: self.reason,
        }
    }

    fn store_targets(&self, input: &dyn AnalysisInputs) -> Vec<TargetId> {
        let view = input.invocation();
        self.targets
            .iter()
            .flat_map(|role| view.operands_with_role(*role))
            .map(TargetId)
            .collect()
    }

    /// A may-bind of each target on the normal completion, as `unset`'s
    /// transfer is an unbind of each; every other domain, and a command
    /// that may unbind its target, is generic.
    fn transfer(
        &self,
        domain: FactDomain,
        input: &dyn AnalysisInputs,
        _budget: &mut Budget,
    ) -> TransferAnswer {
        let (FactDomain::Existence, Some(kind)) = (domain, self.kind) else {
            return TransferAnswer::Generic;
        };
        TransferAnswer::Existence(ExistenceTransfer {
            paths: vec![CompletionPath {
                completion: crate::completion::CompletionCodeDomain::Exact(&[
                    crate::completion::CompletionCode::Ok,
                ]),
                outcomes: self
                    .store_targets(input)
                    .into_iter()
                    .map(|target| (target, ExistenceOutcome::MayBind(kind)))
                    .collect(),
            }],
        })
    }
}

/// The completion codes `foreachLine`'s loop body absorbs, as `foreach`'s
/// does (TIP 670's reference implementation is a `foreach`-shaped `while`
/// over `gets`).
const FOREACH_LINE_ABSORBED: &[crate::completion::CompletionCode] = &[
    crate::completion::CompletionCode::Break,
    crate::completion::CompletionCode::Continue,
];

/// `foreachLine varName filename body`: the same loop shape `foreach`
/// declares, over the file `filename` names rather than a Tcl list — a
/// source no route reads (TIP 670). The structured lowering (`tcl-compiler`'s
/// `lower_foreach_line`) already turns every statically-bodied call into a
/// plain `Statement::Foreach`, so this plan is read only for the dynamic-body
/// fallback call the registry still resolves by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ForeachLineSemantics;

/// `foreachLine`.
pub static FOREACH_LINE: ForeachLineSemantics = ForeachLineSemantics;

impl CommandSemantics for ForeachLineSemantics {
    fn identity(&self) -> &'static str {
        "iterate:foreach_line"
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::None {
            reason: NoRouteReason::Unauthored,
        }
    }

    fn structure(&self, input: &dyn AnalysisInputs) -> PlanAnswer {
        let view = input.invocation();
        let InvocationLayout::Source = view.layout else {
            // The CFG's own synthetic loop header always names its command
            // `foreach`/`lmap`/`dict for`/`dict map` (`cfg_lower.rs`), never
            // the originating surface command, so this specialisation is
            // never asked for a `LoopHeader` plan today.
            return PlanAnswer::NoStructure;
        };
        let first = view.argument_offset;
        if view.operands.len() != first + 3 {
            return PlanAnswer::Declined(DeclineReason::WrongRepresentation);
        }
        PlanAnswer::Iterate(IterationPlan {
            binders: vec![Binder {
                name: BinderName::Operand(OperandId(first)),
                kind: BindingKind::Scalar,
            }],
            // Not `IterableKind::List`: the operand is a filename, and
            // reading it as a list would misreport the file's own name as
            // its contents. `Vendor` is the shared "opaque collection, no
            // known cardinality" shape.
            iterable: IterableKind::Vendor {
                collection: OperandId(first + 1),
                cardinality: None,
            },
            body: Some(BodyPlan {
                body: OperandId(first + 2),
                frame: FrameLevel::Relative(0),
            }),
            exit: ExitRule::Exhaustion,
            zero_iterations_bind: false,
            completion: CompletionProtocol::Absorb(FOREACH_LINE_ABSORBED),
        })
    }
}
