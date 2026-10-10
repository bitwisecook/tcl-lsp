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
//! `continue`, the protocols by which a body's completion becomes its
//! command's, and the handler chain of `try`, whose facts every consumer of
//! the handler list reads from [`HandlerChain`].

use tcl_dialect::model::SpecSurface;
use tcl_dialect::{DialectProfile, TclVersion};
use tcl_syntax::number::{Number, NumberSyntax, Numbers};
use tcl_syntax::word_rules::WordValueRules;

use crate::arg_role::ArgRole;
use crate::clause_grammar::{ClauseGrammarSpec, ClausePlan, ClauseRowId};
use crate::completion::{CompletionCode, CompletionCodeDomain, completion_code_selector};
use crate::frame_effect::FrameLevel;
use crate::invocation_words::{InvocationArguments, InvocationWord, InvocationWordKind};
use crate::types::TclType;

use super::CommandSemantics;
use super::answers::{
    Binder, BinderName, BindingKind, BodyPlan, CompletionOutcome, CompletionPath,
    CompletionProtocol, DependencyEvidence, EvalAnswer, ExactValue, ExactValueOrUnavailable,
    Existence, ExistenceOutcome, ExistenceTransfer, FactBounds, HandlerMatch, HandlerPlan,
    InvocationOutcome, PlanAnswer, Reconcile, RouteIdentity, SegmentFacts, StoreOutcome,
    TransferAnswer, TypeFacts,
};
use super::const_ops::TargetSemantics;
use super::context::Budget;
use super::decline::{DeclineReason, NoRouteReason};
use super::destructure::unavailable;
use super::inputs::{
    AnalysisInputs, DomainFact, EvaluationState, FactDomain, FactView, NestedPolicy, OperandId,
    TargetId, WrittenPlace, written_in,
};
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

/// One word of a `return` invocation, as the decoding of its options reads
/// it ([`decode_return`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnWord<'w> {
    /// A word whose value is this text.
    Text(&'w str),
    /// A computed word whose value cannot begin with `-`, so it names none of
    /// the options a release reads.
    NotAnOption,
    /// A computed word.
    Unknown,
}

/// Why [`decode_return`] gives no answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnUnknown {
    /// A word the decoding reads is computed.
    Word,
    /// `-options` merges a dictionary, which the decoding does not read.
    Options,
    /// The target's release is not named, and the releases read the words
    /// differently: 8.4 reads three options and rejects any other, 8.5 keeps
    /// any pair; or a numeral the release's integer conversion reads its own
    /// way.
    Release,
}

/// The completion a `return` whose options are accepted gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReturnCompletion {
    /// The code the completion carries once `level` frames are left. `-code
    /// return` is `ok` one level further out, as every release reads it
    /// (`catch {return -code return x} m o` leaves `o` `-code 0 -level 2`).
    pub code: CompletionCode,
    /// How many frames the completion climbs: 0 is the command's own
    /// completion. 8.4 has no `-level`, so it is 1 there.
    pub level: u32,
    /// The index of the result word, `None` for the empty result.
    pub result: Option<usize>,
    /// The index of the last `-errorcode` value word.
    pub error_code: Option<usize>,
}

impl ReturnCompletion {
    /// The code the command itself completes with: its own at level 0, and
    /// `TCL_RETURN` while a level remains to climb.
    #[must_use]
    pub fn own_code(self) -> CompletionCode {
        if self.level == 0 {
            self.code
        } else {
            CompletionCode::Return
        }
    }
}

/// What a `return` completes with, read from its words ([`decode_return`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnDecoding {
    /// The release accepts the options, and the command completes so.
    Completes(ReturnCompletion),
    /// The release rejects an option or its value: the command raises
    /// `TCL_ERROR` before it returns.
    Rejects,
    /// What the command completes with is not known.
    Unknown(ReturnUnknown),
}

/// `return`'s `count` words after its name, read as the release `profile`
/// runs reads them (`Tcl_ReturnObjCmd` and `TclMergeReturnOptions`, 8.4 to
/// 9.1): the one decoding of `return`'s options, which the lowering, the CFG,
/// the registry's completion queries and the solver's route all read.
///
/// While two words remain the first names an option and the second is its
/// value, and a last word on its own is the result, whatever it starts with
/// (`return -code` returns the text `-code`). `-code` takes one of the five
/// names or an integer, `-errorinfo` any value, and `-errorcode` a list from
/// 8.5 (8.4 takes any text); any other name is rejected by 8.4. From 8.5
/// `-level` takes a non-negative integer, `-options` merges a dictionary,
/// `-errorstack` takes a list of even length from 8.6, and any other pair is
/// kept in the options dictionary, which changes nothing here. A later pair
/// replaces an earlier one. Integers are read with the release's numeral
/// grammar and its 32-bit conversion (`010` is 8 in 8.x and 10 from 9.0).
/// Where the target names no release, only the readings every release
/// shares are answered.
pub fn decode_return<'w>(
    profile: Option<&'static DialectProfile>,
    count: usize,
    word: impl Fn(usize) -> ReturnWord<'w>,
) -> ReturnDecoding {
    let target = TargetSemantics::of(profile);
    let numbers = target.numerals.map_or(Numbers::Unknown, Numbers::Target);
    // Whether the release keeps any pair in a dictionary (8.5 on) or reads
    // three options and rejects the rest (8.4); `None` where it is unnamed.
    let dictionary = target.release.map(|release| release >= TclVersion::V8_5);
    let result = (count % 2 == 1).then(|| count - 1);
    let mut completion = ReturnCompletion {
        code: CompletionCode::Ok,
        level: 1,
        result,
        error_code: None,
    };
    for at in (0..count - usize::from(result.is_some())).step_by(2) {
        let name = match word(at) {
            ReturnWord::Text(name) => name,
            ReturnWord::NotAnOption => match dictionary {
                Some(true) => continue,
                Some(false) => return ReturnDecoding::Rejects,
                None => return ReturnDecoding::Unknown(ReturnUnknown::Release),
            },
            ReturnWord::Unknown => return ReturnDecoding::Unknown(ReturnUnknown::Word),
        };
        let value = word(at + 1);
        let read = match name {
            "-code" => read_code(value, numbers).map(|code| completion.code = code),
            "-errorinfo" => Ok(()),
            "-errorcode" => {
                completion.error_code = Some(at + 1);
                match dictionary {
                    Some(false) => Ok(()),
                    _ => read_list(value, dictionary, |_| true),
                }
            }
            _ if dictionary == Some(false) => Err(ReturnDecoding::Rejects),
            _ if dictionary.is_none() => Err(ReturnDecoding::Unknown(ReturnUnknown::Release)),
            "-level" => read_level(value, numbers).map(|level| completion.level = level),
            "-options" => Err(ReturnDecoding::Unknown(ReturnUnknown::Options)),
            "-errorstack"
                if target
                    .release
                    .is_some_and(|release| release >= TclVersion::V8_6) =>
            {
                read_list(value, dictionary, |elements| elements % 2 == 0)
            }
            _ => Ok(()),
        };
        if let Err(decoding) = read {
            return decoding;
        }
    }
    if completion.code == CompletionCode::Return {
        completion.code = CompletionCode::Ok;
        completion.level = completion.level.saturating_add(1);
    }
    ReturnDecoding::Completes(completion)
}

/// A `-code` value: one of the five names or an integer the release's
/// conversion reads.
fn read_code(value: ReturnWord<'_>, numbers: Numbers) -> Result<CompletionCode, ReturnDecoding> {
    let ReturnWord::Text(text) = value else {
        return Err(ReturnDecoding::Unknown(ReturnUnknown::Word));
    };
    completion_code_selector(text, numbers).ok_or_else(|| not_an_integer(text, numbers))
}

/// A `-level` value: an integer the release's conversion reads as
/// non-negative.
fn read_level(value: ReturnWord<'_>, numbers: Numbers) -> Result<u32, ReturnDecoding> {
    let ReturnWord::Text(text) = value else {
        return Err(ReturnDecoding::Unknown(ReturnUnknown::Word));
    };
    match crate::completion::canonical_completion_code(text, numbers) {
        Some(level) => u32::try_from(level).map_err(|_| ReturnDecoding::Rejects),
        None => Err(not_an_integer(text, numbers)),
    }
}

/// The answer for an option value the conversion did not read: an integer
/// spelling a release reads its own way, or one past the conversion's range,
/// is left to the release, and anything else is rejected.
fn not_an_integer(text: &str, numbers: Numbers) -> ReturnDecoding {
    let integer = |syntax: NumberSyntax| {
        matches!(
            Numbers::Target(syntax).parse_whole(text),
            Some(Number::Int(_) | Number::Big { .. })
        )
    };
    if numbers
        .syntax()
        .map_or_else(|| NumberSyntax::any(integer), integer)
    {
        ReturnDecoding::Unknown(ReturnUnknown::Release)
    } else {
        ReturnDecoding::Rejects
    }
}

/// [`decode_return`] over an invocation's words: a literal is its text — save,
/// in the literal compatibility view, one spelled as a substitution
/// (`$code`), which may hold any value — a substituted word that cannot begin
/// with `-` names no option, and any other word is unknown. A word an
/// expansion contributes leaves the count unknown.
#[must_use]
pub fn decode_return_words(
    profile: Option<&'static DialectProfile>,
    args: InvocationArguments<'_>,
) -> ReturnDecoding {
    let Some(count) = args.exact_argv_len() else {
        return ReturnDecoding::Unknown(ReturnUnknown::Word);
    };
    decode_return(profile, count, |at| match args.get(at) {
        Some(InvocationWord::Literal(text))
            if args.is_source_aware() || !tcl_syntax::naming::is_dynamic_word(text) =>
        {
            ReturnWord::Text(text)
        }
        Some(InvocationWord::DynamicNonOption) => ReturnWord::NotAnOption,
        _ => ReturnWord::Unknown,
    })
}

/// A value 8.5 on requires to be a list whose element count `fits`: a
/// release that requires it rejects any other, and an unnamed release may.
fn read_list(
    value: ReturnWord<'_>,
    dictionary: Option<bool>,
    fits: impl Fn(usize) -> bool,
) -> Result<(), ReturnDecoding> {
    let ReturnWord::Text(text) = value else {
        return Err(ReturnDecoding::Unknown(ReturnUnknown::Word));
    };
    match WordValueRules::TCL.split_list(text) {
        Ok(elements) if fits(elements.len()) => Ok(()),
        _ if dictionary == Some(true) => Err(ReturnDecoding::Rejects),
        _ => Err(ReturnDecoding::Unknown(ReturnUnknown::Release)),
    }
}

/// `return ?option value ...? ?result?`: the completion its options give,
/// after no store, as [`decode_return`] reads them — the decoding the
/// lowering, the CFG and the registry's completion queries read too. At a
/// positive level the completion is the pending one a procedure or `catch`
/// consumes ([`CompletionOutcome::Code`]); at level 0 it is the code itself —
/// a normal completion for `ok`, the error for `error`, carrying the
/// `-errorcode` word where one is given. Options the release rejects are an
/// error whose message the route does not word.
///
/// A word the solver has not reached makes the answer pending. An option
/// word that is not exact, `-options`, whose dictionary the decoding does not
/// merge, and a reading that depends on a release the target does not name
/// decline. The result is exact where its word is, and unproven otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReturnSemantics;

/// `return`.
pub static RETURN: ReturnSemantics = ReturnSemantics;

impl ReturnSemantics {
    fn evaluate_return(input: &dyn AnalysisInputs) -> EvalAnswer {
        let count = input.invocation().operands.len();
        // The option words, which the decoding reads: every word before a
        // lone last one.
        let mut options = Vec::with_capacity(count);
        for at in 0..count - count % 2 {
            options.push(match input.operand(OperandId(at), FactDomain::ExactValue) {
                FactView::Exact(value, _) => value.as_str().ok().map(str::to_owned),
                FactView::Pending => return EvalAnswer::Pending,
                FactView::Finite(..) => return EvalAnswer::Declined(DeclineReason::CorrelatedSets),
                FactView::Domain(_) | FactView::Top(_) => None,
            });
        }
        let text = |at: usize| options.get(at).and_then(Option::as_deref);
        let returned = match decode_return(input.context().profile, count, |at| {
            text(at).map_or(ReturnWord::Unknown, ReturnWord::Text)
        }) {
            ReturnDecoding::Completes(returned) => returned,
            ReturnDecoding::Rejects => {
                return completed(
                    NativeEvalId::ReturnComplete,
                    CompletionOutcome::error_unproven(0),
                    ExactValueOrUnavailable::unproven_string(),
                );
            }
            ReturnDecoding::Unknown(ReturnUnknown::Word) => {
                return EvalAnswer::Declined(DeclineReason::NotExact);
            }
            ReturnDecoding::Unknown(ReturnUnknown::Options) => {
                return EvalAnswer::Declined(DeclineReason::Unsupported);
            }
            ReturnDecoding::Unknown(ReturnUnknown::Release) => {
                return EvalAnswer::Declined(unavailable(SpecSurface::TCL85_PLUS));
            }
        };
        let result = match returned.result {
            Some(at) => match input.operand(OperandId(at), FactDomain::ExactValue) {
                FactView::Exact(value, _) => ExactValueOrUnavailable::Exact(value),
                FactView::Pending => return EvalAnswer::Pending,
                FactView::Finite(..) => {
                    return EvalAnswer::Declined(DeclineReason::CorrelatedSets);
                }
                FactView::Domain(_) | FactView::Top(_) => {
                    ExactValueOrUnavailable::unproven_string()
                }
            },
            None => ExactValueOrUnavailable::exact_text(""),
        };
        let completion = match (returned.level, returned.code) {
            (0, CompletionCode::Ok) => CompletionOutcome::Normal,
            (0, CompletionCode::Error) => CompletionOutcome::Error {
                written: 0,
                message: result.clone(),
                error_code: returned.error_code.map_or_else(
                    || ExactValueOrUnavailable::exact_text("NONE"),
                    |at| {
                        text(at).map_or_else(ExactValueOrUnavailable::unproven_string, |code| {
                            ExactValueOrUnavailable::exact_text(code)
                        })
                    },
                ),
            },
            (level, code) => CompletionOutcome::Code {
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
/// completion, the result variable and the options variable are written.
///
/// A closed script — brace-quoted, every command with a route of its own and
/// every value exact — is run by the nested service under the protected
/// policy ([`NestedPolicy::Protected`]), which hands back the first completion
/// that is not the normal one together with the writes that ran before it.
/// The command then completes normally with the code a caller of the script
/// observes ([`CompletionOutcome::observed_code`]), and writes the result
/// variable what the script returned — its last result, the message of its
/// error, or what `return` carried — and the options variable a dictionary.
/// The writes the script made are the command's nested writes, ahead of its
/// own two stores.
///
/// The dictionary is never an exact value. `-errorinfo`, `-errorline` and
/// `-errorstack` are the interpreter's text, and from 8.6 a success may carry
/// a stale `-errorcode` (`catch {incr absent}` leaves `TCL READ VARNAME`
/// there). The store states what holds: a dictionary, and for a completion
/// other than an error the prefix `-code N -level L` (an error's dictionary
/// lists `-errorinfo` first when `error` is given one). A script that is not
/// closed, a variable that may be an array or is an element, and a third word
/// before 8.5 decline, and the existence transfer states what holds on every
/// path: both variables are bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatchSemantics;

/// `catch`.
pub static CATCH: CatchSemantics = CatchSemantics;

impl CatchSemantics {
    /// What a variable operand's store needs of its place once the script has
    /// run: a scalar or an absent place, which the store succeeds on. The
    /// script's own writes are read first, then the fact before the command;
    /// an element, which may fail on its array, a place that may be an array
    /// and one the script left unknown decline, and a fact not yet reached is
    /// pending.
    fn writable(
        input: &dyn AnalysisInputs,
        state: &EvaluationState,
        id: OperandId,
    ) -> Result<(), EvalAnswer> {
        let place = input.place(id).map_err(EvalAnswer::Declined)?;
        if place.is_element() {
            return Err(EvalAnswer::Declined(DeclineReason::Unsupported));
        }
        match written_in(&state.writes, &place.name) {
            WrittenPlace::Exact(_) => return Ok(()),
            WrittenPlace::Unknown => {
                return Err(EvalAnswer::Declined(DeclineReason::StatefulNested));
            }
            WrittenPlace::Untouched => {}
        }
        match input.prior_store(&place, FactDomain::Existence) {
            FactView::Domain(DomainFact::Existence(
                Existence::Unbound | Existence::Bound(BindingKind::Scalar),
            )) => Ok(()),
            FactView::Pending | FactView::Domain(DomainFact::Existence(Existence::Pending)) => {
                Err(EvalAnswer::Pending)
            }
            _ => Err(EvalAnswer::Declined(DeclineReason::NotExact)),
        }
    }

    /// A value stored into a variable: the exact text as a write, an unproven
    /// one as a write whose value is unavailable and whose binding is certain.
    fn stored(target: TargetId, value: ExactValueOrUnavailable) -> StoreOutcome {
        match value {
            ExactValueOrUnavailable::Exact(value) => StoreOutcome::Write { target, value },
            ExactValueOrUnavailable::Unavailable(facts) => StoreOutcome::WriteUnavailable {
                target,
                facts: FactBounds {
                    existence: Existence::Bound(BindingKind::Scalar),
                    ..facts
                },
            },
        }
    }

    /// What the options variable holds after `completion`: a dictionary, whose
    /// text starts `-code N -level L` unless the completion is an error.
    fn options(completion: &CompletionOutcome) -> FactBounds {
        let prefix = (!matches!(completion, CompletionOutcome::Error { .. })).then(|| {
            let (code, level) = completion.options_code_and_level();
            format!("-code {code} -level {level}").into_bytes()
        });
        FactBounds {
            existence: Existence::Bound(BindingKind::Scalar),
            intrep: Some(TclType::Dict),
            shape: None,
            segments: prefix.map(|prefix| SegmentFacts {
                min_len: Some(prefix.len()),
                max_len: None,
                prefix: Some(prefix),
                suffix: None,
            }),
            taint: None,
        }
    }
}

impl CommandSemantics for CatchSemantics {
    fn identity(&self) -> &'static str {
        NativeEvalId::CatchProtected.as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: NativeEvalId::CatchProtected,
        }
    }

    fn structure(&self, input: &dyn AnalysisInputs) -> PlanAnswer {
        let words = input.invocation().operands.len();
        if !(1..=3).contains(&words) {
            return PlanAnswer::Declined(DeclineReason::Unsupported);
        }
        PlanAnswer::Body {
            binders: Vec::new(),
            body: BodyPlan {
                body: OperandId(0),
                frame: FrameLevel::Relative(0),
            },
            reconcile: Reconcile::None,
            completion: CompletionProtocol::CatchAll {
                result_var: (words >= 2).then_some(TargetId(OperandId(1))),
                options_var: (words == 3).then_some(TargetId(OperandId(2))),
            },
        }
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, _budget: &mut Budget) -> EvalAnswer {
        let words = input.invocation().operands.len();
        // Another count is `wrong # args`, which the route does not word.
        if !(1..=3).contains(&words) {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        }
        let target = TargetSemantics::of(input.context().profile);
        if words == 3 {
            match target.release {
                Some(release) if release >= TclVersion::V8_5 => {}
                Some(_) => return EvalAnswer::Declined(DeclineReason::Unsupported),
                None => return EvalAnswer::Declined(unavailable(SpecSurface::TCL85_PLUS)),
            }
        }
        let script = match input.body(OperandId(0)) {
            Ok(region) => region.script,
            Err(reason) => return EvalAnswer::Declined(reason),
        };
        let mut state = EvaluationState::new(NestedPolicy::Protected);
        let body = match input.nested(&script, &mut state) {
            EvalAnswer::Evaluated(body) => body,
            other => return other,
        };
        for at in 1..words {
            if let Err(answer) = Self::writable(input, &state, OperandId(at)) {
                return answer;
            }
        }
        let returned = match &body.completion {
            CompletionOutcome::Normal => body.result.clone(),
            CompletionOutcome::Code { result, .. } => result.clone(),
            CompletionOutcome::Error { message, .. } => message.clone(),
        };
        let mut ordered_stores = Vec::with_capacity(words - 1);
        if words >= 2 {
            ordered_stores.push(Self::stored(TargetId(OperandId(1)), returned));
        }
        if words == 3 {
            ordered_stores.push(StoreOutcome::WriteUnavailable {
                target: TargetId(OperandId(2)),
                facts: Self::options(&body.completion),
            });
        }
        let id = NativeEvalId::CatchProtected;
        EvalAnswer::Evaluated(Box::new(InvocationOutcome {
            completion: CompletionOutcome::Normal,
            result: ExactValueOrUnavailable::Exact(ExactValue::int(
                body.completion.observed_code(),
            )),
            nested_writes: state.writes,
            ordered_stores,
            types: TypeFacts {
                result: Some(TclType::Int),
                ..TypeFacts::default()
            },
            evidence: DependencyEvidence {
                route: Some(RouteIdentity {
                    route: EvalRoute::Direct { id },
                    implementation: id.as_str(),
                    revision: REVISION,
                }),
                numerals: target.numerals,
                characters: target.character_model,
                release: target.release,
                ..state.evidence
            },
        }))
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

/// One handler of a `try`, as [`HandlerChain`] reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandlerLink<'a> {
    /// How the handler's pattern word selects it.
    pub matches: HandlerMatch,
    /// The pattern word's value: the spelling of a completion code for a
    /// selection by code, unread for a selection by `-errorcode` prefix.
    pub selector: &'a str,
    /// Whether the handler's script is the fall-through marker, which runs
    /// the script of the next handler that has one.
    pub falls_through: bool,
}

/// A handler with its selector decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChainEntry {
    matches: HandlerMatch,
    code: Option<CompletionCode>,
    falls_through: bool,
}

/// The handlers of one `try`, in order, read the way `try` runs them: the
/// first handler whose selector matches the body's completion runs, a `-`
/// handler still selects its own code and runs the script of the next handler
/// that has one, and a `trap` selects only an error whose `-errorcode` starts
/// with its pattern. Every fact a consumer needs of the list — which handler
/// takes a completion, which can never run, which scripts a match may reach,
/// whether a handler selects a code — is answered here, so that the control
/// flow graph, the solvers and the optimiser read one answer
/// (`docs/design/compiler/value-transfers.md` § *`catch`, `try`, and
/// completion*).
///
/// Measured under 8.6, 9.0 and 9.1:
///
/// ```tcl
/// try {error boom} on error {} {A} on error {} {B}              ;# A
/// try {error boom} on error {} - on ok {} {X}                   ;# X
/// try {error boom} on error {} - on error {} {B} on ok {} {C}   ;# B
/// try {error boom} on error {} {E} trap {} {} {T}               ;# E
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandlerChain {
    entries: Vec<ChainEntry>,
}

impl HandlerChain {
    /// The chain of `links`, each selector decoded under `numbers`: `on 010`
    /// selects code 8 up to 8.6 and code 10 from 9.0.
    pub fn new<'a>(links: impl IntoIterator<Item = HandlerLink<'a>>, numbers: Numbers) -> Self {
        Self {
            entries: links
                .into_iter()
                .map(|link| ChainEntry {
                    matches: link.matches,
                    code: Self::selected(link.matches, link.selector, numbers),
                    falls_through: link.falls_through,
                })
                .collect(),
        }
    }

    /// The completion code a handler selects: `TCL_ERROR` for a selection by
    /// `-errorcode` prefix, whatever its pattern, and for a selection by code
    /// the one `numbers` reads the selector as — `None` for a selector that
    /// names no code, which the registry cannot decide.
    #[must_use]
    pub fn selected(
        matches: HandlerMatch,
        selector: &str,
        numbers: Numbers,
    ) -> Option<CompletionCode> {
        match matches {
            HandlerMatch::ErrorCodePrefix => Some(CompletionCode::Error),
            HandlerMatch::CompletionCode => completion_code_selector(selector, numbers),
        }
    }

    /// The number of handlers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether there are no handlers.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The completion code handler `index` selects ([`Self::selected`]).
    #[must_use]
    pub fn code(&self, index: usize) -> Option<CompletionCode> {
        self.entries[index].code
    }

    /// The handler whose script handler `index` runs: itself, or for a `-`
    /// handler the next one with a script of its own. A chain that ends on a
    /// `-`, which `try` rejects, gives the handler itself.
    #[must_use]
    pub fn owner(&self, index: usize) -> usize {
        self.entries[index..]
            .iter()
            .position(|entry| !entry.falls_through)
            .map_or(index, |offset| index + offset)
    }

    /// The first handler of the group that runs `owner`'s script: the `-`
    /// handlers just before it.
    fn group_start(&self, owner: usize) -> usize {
        self.entries[..owner]
            .iter()
            .rposition(|entry| !entry.falls_through)
            .map_or(0, |earlier| earlier + 1)
    }

    /// Whether handler `index` can never run because a handler before its
    /// group always takes its completions first: only an unconditional
    /// handler pre-empts — an `on` with the same code, a `-` one included,
    /// since it still selects its code and only delegates its script — and a
    /// `trap` never does, its pattern may not match. A selector that names no
    /// code pre-empts nothing and is pre-empted by nothing. Handlers of one
    /// group run one script, so none of them pre-empts another.
    #[must_use]
    pub fn preempted(&self, index: usize) -> bool {
        let Some(code) = self.code(index) else {
            return false;
        };
        let start = self.group_start(self.owner(index));
        self.entries[..start].iter().any(|earlier| {
            earlier.matches == HandlerMatch::CompletionCode && earlier.code == Some(code)
        })
    }

    /// The handlers whose match runs handler `index`'s own script: the
    /// handler with the `-` handlers that hand it their match, less any an
    /// earlier handler pre-empts ([`Self::preempted`]). Empty for a `-`
    /// handler, whose script is its owner's, and for a group of which every
    /// member is pre-empted.
    #[must_use]
    pub fn live_group(&self, index: usize) -> Vec<usize> {
        if self.entries[index].falls_through {
            return Vec::new();
        }
        (self.group_start(index)..=index)
            .filter(|member| !self.preempted(*member))
            .collect()
    }

    /// Whether handler `index` certainly does not select a completion with
    /// `code`: its selector names another. A selector that names no code
    /// might select it.
    #[must_use]
    pub fn misses(&self, index: usize, code: CompletionCode) -> bool {
        self.code(index).is_some_and(|selected| selected != code)
    }

    /// Whether handler `index` takes every completion with `code`: it selects
    /// by code and names it. A `trap` takes none, its pattern may not match.
    #[must_use]
    pub fn takes(&self, index: usize, code: CompletionCode) -> bool {
        let entry = &self.entries[index];
        entry.matches == HandlerMatch::CompletionCode && entry.code == Some(code)
    }

    /// The handler that certainly runs for a completion with `code`: the
    /// first whose selector is not known to name another code, when it takes
    /// the code whole. `None` when no handler selects it, and when the first
    /// that might is a `trap` or has a selector the registry cannot decide —
    /// either may or may not run it.
    #[must_use]
    pub fn first_taking(&self, code: CompletionCode) -> Option<usize> {
        let first = self
            .entries
            .iter()
            .position(|entry| entry.code.is_none_or(|selected| selected == code))?;
        self.takes(first, code).then_some(first)
    }
}

/// `try body ?handler…? ?finally script?`: the protected body, the handlers in
/// the order they are tried, and the `finally` script that runs on every path.
///
/// The structure is read from the command's clause grammar: the plan is a body
/// run in the frame the command is written in, whose completion
/// ([`CompletionProtocol::Handlers`]) runs the first handler that selects it,
/// each [`HandlerPlan`] carrying how its pattern selects (`on` by code, `trap`
/// by `-errorcode` prefix), the pattern's operand, the names its variable list
/// binds and its script — `None` for a `-` handler, which runs the next
/// script. The binders are the first two elements of the list, the result
/// variable then the options variable (a third is ignored, as `try` ignores
/// it); an empty element binds nothing and is kept as an empty name, so that
/// the position says which is which. What a plan cannot place declines: a
/// keyword or `-` that is computed, a variable list the analysis does not know
/// exactly, a word that expands into several, and a shape `try` rejects (a
/// last handler whose script is `-`, a `finally` that is not last). The
/// handlers' own facts are [`HandlerChain`]'s.
///
/// There is no evaluation route: the command's value is its body's or a
/// handler's, and nothing here runs them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrySemantics {
    /// The command's clause grammar, which says where each clause stands.
    pub grammar: &'static ClauseGrammarSpec,
}

impl TrySemantics {
    /// The binders a handler's variable-list word names: its first two
    /// elements, in order.
    fn binders(input: &dyn AnalysisInputs, list: OperandId) -> Result<Vec<Binder>, DeclineReason> {
        let text = match input.operand(list, FactDomain::ExactValue) {
            FactView::Exact(value, _) => {
                String::from_utf8(value.bytes).map_err(|_| DeclineReason::NotText)?
            }
            FactView::Top(reason) => return Err(reason),
            FactView::Pending | FactView::Finite(..) | FactView::Domain(_) => {
                return Err(DeclineReason::NotExact);
            }
        };
        // A malformed list is the command's error.
        let names =
            tcl_syntax::list::split_list(&text).map_err(|_| DeclineReason::WrongRepresentation)?;
        Ok(names
            .iter()
            .take(2)
            .map(|name| Binder {
                name: BinderName::Declared((*name).to_string()),
                kind: BindingKind::Scalar,
            })
            .collect())
    }

    /// The plan of the handler clause `at` of `plan`.
    fn handler(
        input: &dyn AnalysisInputs,
        plan: &ClausePlan,
        at: usize,
    ) -> Result<HandlerPlan, DeclineReason> {
        let clause = &plan.clauses[at];
        let (pattern, matches) = clause.handler().ok_or(DeclineReason::Unsupported)?;
        let list = clause
            .operand(ArgRole::LoopVarList)
            .ok_or(DeclineReason::Unsupported)?;
        let body = if plan.falls_through(at) {
            None
        } else {
            Some(OperandId(
                clause
                    .operand(ArgRole::Body)
                    .ok_or(DeclineReason::Unsupported)?,
            ))
        };
        Ok(HandlerPlan {
            matches,
            pattern: OperandId(pattern),
            binders: Self::binders(input, OperandId(list))?,
            body,
        })
    }
}

impl CommandSemantics for TrySemantics {
    fn identity(&self) -> &'static str {
        "body:try"
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::None {
            reason: NoRouteReason::Unauthored,
        }
    }

    fn structure(&self, input: &dyn AnalysisInputs) -> PlanAnswer {
        let view = input.invocation();
        let first = view.argument_offset;
        let words = view.operands.get(first..).unwrap_or_default();
        // A word that expands, or a region with no words, leaves the count of
        // clauses unknown.
        if words.iter().any(|word| {
            matches!(
                word.kind,
                InvocationWordKind::Expanded | InvocationWordKind::Opaque
            )
        }) {
            return PlanAnswer::Declined(DeclineReason::Unsupported);
        }
        let texts: Vec<&str> = words.iter().map(|word| word.text).collect();
        let computed: Vec<bool> = words
            .iter()
            .map(|word| word.kind != InvocationWordKind::Literal)
            .collect();
        // Tcl decides a computed keyword, or a computed `-`, by its value.
        let Ok(plan) = self
            .grammar
            .walk_words_or_abstain(&texts, &computed, &[], None)
        else {
            return PlanAnswer::Declined(DeclineReason::NotExact);
        };
        if plan.defect.is_some() {
            return PlanAnswer::Declined(DeclineReason::WrongRepresentation);
        }
        let plan = plan.offset_by(first);
        let mut body = None;
        let mut handlers = Vec::new();
        let mut finally = None;
        for (at, clause) in plan.clauses.iter().enumerate() {
            match clause.row {
                ClauseRowId::Head => body = clause.operand(ArgRole::Body).map(OperandId),
                ClauseRowId::Tail => finally = clause.operand(ArgRole::Body).map(OperandId),
                ClauseRowId::Row(_) => match Self::handler(input, &plan, at) {
                    Ok(handler) => handlers.push(handler),
                    Err(reason) => return PlanAnswer::Declined(reason),
                },
            }
        }
        let Some(body) = body else {
            return PlanAnswer::Declined(DeclineReason::WrongRepresentation);
        };
        // `try` rejects a chain whose last handler runs the script after it.
        if handlers
            .last()
            .is_some_and(|handler| handler.body.is_none())
        {
            return PlanAnswer::Declined(DeclineReason::WrongRepresentation);
        }
        PlanAnswer::Body {
            binders: Vec::new(),
            body: BodyPlan {
                body,
                frame: FrameLevel::Relative(0),
            },
            reconcile: Reconcile::None,
            completion: CompletionProtocol::Handlers { handlers, finally },
        }
    }
}
