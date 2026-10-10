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

/// One value read by the return-options decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnWord<'w> {
    /// An exact logical value.
    Text(&'w str),
    /// Exact counted bytes, which expose ASCII option syntax only. Opaque
    /// non-ASCII bytes retain an unknown value rather than a text projection.
    Bytes(&'w [u8]),
    /// A computed value whose nonempty prefix cannot begin with `-`.
    NotAnOption,
    /// A computed value with no selected option syntax.
    Unknown,
}

/// Why the return-options decoder cannot determine a completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnUnknown {
    /// A word whose value the selected grammar reads is not exact.
    Word,
    /// An options dictionary lacks exact contents.
    Options,
    /// An engine, release, numeric width or source/compiler distinction remains.
    Release,
}

/// The admitted facet of a return invocation, independently of its values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnInvocationFacet {
    /// Original-source advice without a selected native compiler artifact.
    /// A source/worker disagreement remains unknown.
    OriginalSource,
    /// Already evaluated argv consumed by the selected original worker.
    /// This facet does not itself establish that worker's availability.
    EvaluatedArguments,
}

/// The completion carried by accepted return options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnCompletion {
    /// Eventual code. C Tcl normalises `return` to `ok` one level further out;
    /// Jim retains code two and the supplied level.
    pub code: CompletionCode,
    /// Pending unwind count. Wider Jim levels remain unresolved because this
    /// public representation does not carry Jim's signed-wide catch field.
    pub level: u32,
    /// Original argv ordinal of the result, or no result word.
    pub result: Option<usize>,
    /// Original argv ordinal of the selected direct `-errorcode` value.
    pub error_code: Option<usize>,
    /// Exact selected `-errorcode` from an options dictionary. A nested member
    /// is a value, not an invented original argv ordinal.
    pub merged_error_code: Option<String>,
}

impl ReturnCompletion {
    /// Immediate completion, before a procedure consumes any unwind level.
    #[must_use]
    pub fn own_code(&self) -> CompletionCode {
        if self.level == 0 {
            self.code
        } else {
            CompletionCode::Return
        }
    }
}

/// A selected return grammar's completion or explicit unresolved premise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReturnDecoding {
    /// The selected grammar accepts these values.
    Completes(ReturnCompletion),
    /// The selected grammar rejects a known option or value.
    Rejects,
    /// A required value, grammar or invocation facet is unavailable.
    Unknown(ReturnUnknown),
}

#[derive(Debug, Clone, Copy)]
struct ReturnTarget {
    grammar: crate::completion_route::ReturnInvocationGrammar,
    numbers: Numbers,
    release: Option<TclVersion>,
    facet: ReturnInvocationFacet,
}

impl ReturnTarget {
    fn for_profile(profile: Option<&DialectProfile>) -> Self {
        let dialect = profile.map(crate::InvocationDialect::of_profile);
        Self {
            grammar: dialect.map_or(
                crate::completion_route::ReturnInvocationGrammar::Unknown,
                crate::completion_route::ReturnInvocationGrammar::for_dialect,
            ),
            numbers: dialect.map_or(Numbers::Unknown, |dialect| Numbers::Target(dialect.numbers)),
            release: dialect.and_then(|dialect| dialect.tcl_version),
            facet: ReturnInvocationFacet::OriginalSource,
        }
    }

    fn option_name(self, text: &str) -> &str {
        if self.grammar == crate::completion_route::ReturnInvocationGrammar::Jim {
            text.split_once('\0').map_or(text, |(prefix, _)| prefix)
        } else {
            text
        }
    }
}

/// Explicit standalone original-source advice under the supplied profile.
/// The last odd word is the result even when it resembles an option. Actual
/// retained invocations use [`decode_return_words_in`] with their real facet.
#[must_use]
pub fn decode_return<'w>(
    profile: Option<&'static DialectProfile>,
    count: usize,
    word: impl Fn(usize) -> ReturnWord<'w>,
) -> ReturnDecoding {
    decode_return_in(ReturnTarget::for_profile(profile), count, &word)
}

/// Standalone source-word adapter. An actual retained dialect is consumed
/// directly; the profile is only this explicit compatibility entry's input.
#[must_use]
pub fn decode_return_words(
    profile: Option<&'static DialectProfile>,
    args: InvocationArguments<'_>,
) -> ReturnDecoding {
    let mut target = ReturnTarget::for_profile(profile);
    if let Some(dialect) = args.dialect() {
        target.grammar = crate::completion_route::ReturnInvocationGrammar::for_dialect(dialect);
        target.numbers = Numbers::Target(dialect.numbers);
        target.release = dialect.tcl_version;
    }
    decode_argument_words(target, args)
}

/// Decode the selected original grammar, numeric policy, actual release and
/// invocation facet. Source advice cannot establish a worker, compiler entry,
/// result object or frame. Expansion and opaque operand syntax stay unknown.
#[must_use]
pub fn decode_return_words_in(
    grammar: crate::completion_route::ReturnInvocationGrammar,
    numbers: Numbers,
    args: InvocationArguments<'_>,
    facet: ReturnInvocationFacet,
) -> ReturnDecoding {
    decode_argument_words(
        ReturnTarget {
            grammar,
            numbers,
            release: args.dialect().and_then(|dialect| dialect.tcl_version),
            facet,
        },
        args,
    )
}

fn decode_argument_words(target: ReturnTarget, args: InvocationArguments<'_>) -> ReturnDecoding {
    let Some(count) = args.exact_argv_len() else {
        return ReturnDecoding::Unknown(ReturnUnknown::Word);
    };
    decode_return_in(target, count, &|at| match args.get(at) {
        Some(InvocationWord::Literal(text))
            if args.is_source_aware() || !tcl_syntax::naming::is_dynamic_word(text) =>
        {
            ReturnWord::Text(text)
        }
        Some(InvocationWord::KnownBytes(bytes)) => ReturnWord::Bytes(bytes),
        Some(InvocationWord::DynamicNonOption) => ReturnWord::NotAnOption,
        _ => ReturnWord::Unknown,
    })
}

fn unanimous_return(answers: impl IntoIterator<Item = ReturnDecoding>) -> ReturnDecoding {
    let mut answers = answers.into_iter();
    let Some(first) = answers.next() else {
        return ReturnDecoding::Unknown(ReturnUnknown::Release);
    };
    if answers.all(|answer| answer == first) {
        first
    } else {
        ReturnDecoding::Unknown(ReturnUnknown::Release)
    }
}

fn decode_return_in<'w>(
    target: ReturnTarget,
    count: usize,
    word: &impl Fn(usize) -> ReturnWord<'w>,
) -> ReturnDecoding {
    use crate::completion_route::ReturnInvocationGrammar as Grammar;
    if target.grammar == Grammar::Unknown {
        return unanimous_return(
            [Grammar::LegacyTcl, Grammar::OptionsTcl, Grammar::Jim]
                .map(|grammar| decode_return_in(ReturnTarget { grammar, ..target }, count, word)),
        );
    }
    if target.grammar == Grammar::OptionsTcl && target.release.is_none() {
        return unanimous_return([TclVersion::V8_5, TclVersion::V8_6, TclVersion::V9_0].map(
            |release| {
                decode_return_in(
                    ReturnTarget {
                        release: Some(release),
                        ..target
                    },
                    count,
                    word,
                )
            },
        ));
    }
    if target.grammar == Grammar::OptionsTcl
        && target.release == Some(TclVersion::V8_5)
        && target.facet == ReturnInvocationFacet::OriginalSource
    {
        // Native source379/380 and worker383 measure different nested-option
        // readings in C85. No selected compiler artifact is present here.
        return unanimous_return(
            [false, true].map(|inline| decode_return_selected(target, count, word, inline)),
        );
    }
    decode_return_selected(
        target,
        count,
        word,
        target.release != Some(TclVersion::V8_5),
    )
}

#[derive(Debug, Clone)]
struct ReturnOption<'w> {
    value: Option<std::borrow::Cow<'w, str>>,
    ordinal: Option<usize>,
}

impl<'w> ReturnOption<'w> {
    fn from_word(value: ReturnWord<'w>, ordinal: usize) -> Self {
        Self {
            value: return_word_text(value).map(std::borrow::Cow::Borrowed),
            ordinal: Some(ordinal),
        }
    }

    fn text(&self) -> Result<&str, ReturnDecoding> {
        self.value
            .as_deref()
            .ok_or(ReturnDecoding::Unknown(ReturnUnknown::Word))
    }
}

fn return_word_text(word: ReturnWord<'_>) -> Option<&str> {
    match word {
        ReturnWord::Text(text) => Some(text),
        ReturnWord::Bytes(bytes) if bytes.is_ascii() => std::str::from_utf8(bytes).ok(),
        ReturnWord::Bytes(_) | ReturnWord::NotAnOption | ReturnWord::Unknown => None,
    }
}

#[derive(Debug, Default)]
struct ReturnOptions<'w> {
    code: Option<ReturnOption<'w>>,
    level: Option<ReturnOption<'w>>,
    error_code: Option<ReturnOption<'w>>,
    error_stack: Option<ReturnOption<'w>>,
}

impl<'w> ReturnOptions<'w> {
    fn select(&mut self, name: &str, value: ReturnOption<'w>) {
        match name {
            "-code" => self.code = Some(value),
            "-level" => self.level = Some(value),
            "-errorcode" => self.error_code = Some(value),
            "-errorstack" => self.error_stack = Some(value),
            _ => {}
        }
    }

    fn direct(
        &mut self,
        target: ReturnTarget,
        name: &str,
        value: ReturnOption<'w>,
    ) -> Result<(), ReturnDecoding> {
        use crate::completion_route::ReturnInvocationGrammar as Grammar;
        match name {
            "-code" => {
                read_return_code(value.text()?, target)?;
            }
            "-level" if target.grammar == Grammar::Jim => {
                read_return_level(value.text()?, target)?;
            }
            "-errorinfo" | "-errorcode" => {}
            _ => return Err(ReturnDecoding::Rejects),
        }
        self.select(name, value);
        Ok(())
    }

    fn finish(
        self,
        target: ReturnTarget,
        result: Option<usize>,
    ) -> Result<ReturnCompletion, ReturnDecoding> {
        use crate::completion_route::ReturnInvocationGrammar as Grammar;
        let mut code = self.code.as_ref().map_or(Ok(CompletionCode::Ok), |value| {
            read_return_code(value.text()?, target)
        })?;
        let mut level = self
            .level
            .as_ref()
            .map_or(Ok(1), |value| read_return_level(value.text()?, target))?;
        if target.grammar == Grammar::OptionsTcl {
            if let Some(value) = &self.error_code {
                validate_return_list(value.text()?, false)?;
            }
            if target
                .release
                .is_some_and(|release| release >= TclVersion::V8_6)
                && let Some(value) = &self.error_stack
            {
                validate_return_list(value.text()?, true)?;
            }
        }
        if code == CompletionCode::Return && target.grammar != Grammar::Jim {
            code = CompletionCode::Ok;
            if level == i32::MAX.unsigned_abs() {
                return Err(ReturnDecoding::Unknown(ReturnUnknown::Release));
            }
            level += 1;
        }
        let error_code = self.error_code.as_ref().and_then(|value| value.ordinal);
        let merged_error_code = self
            .error_code
            .filter(|value| value.ordinal.is_none())
            .and_then(|value| value.value.map(std::borrow::Cow::into_owned));
        Ok(ReturnCompletion {
            code,
            level,
            result,
            error_code,
            merged_error_code,
        })
    }
}

fn decode_return_selected<'w>(
    target: ReturnTarget,
    count: usize,
    word: &impl Fn(usize) -> ReturnWord<'w>,
    inline: bool,
) -> ReturnDecoding {
    use crate::completion_route::ReturnInvocationGrammar as Grammar;
    let result = (count % 2 == 1).then(|| count - 1);
    let mut options = ReturnOptions::default();
    for at in (0..count - count % 2).step_by(2) {
        let name_word = word(at);
        let Some(name) = return_word_text(name_word) else {
            if name_word == ReturnWord::NotAnOption {
                if target.grammar == Grammar::OptionsTcl {
                    continue;
                }
                return ReturnDecoding::Rejects;
            }
            return ReturnDecoding::Unknown(ReturnUnknown::Word);
        };
        let name = target.option_name(name);
        let value = ReturnOption::from_word(word(at + 1), at + 1);
        let selected = if target.grammar != Grammar::OptionsTcl {
            options.direct(target, name, value)
        } else if name == "-options" {
            merge_return_options(&mut options, value, inline)
        } else {
            options.select(name, value);
            Ok(())
        };
        if let Err(answer) = selected {
            return answer;
        }
    }
    options
        .finish(target, result)
        .map_or_else(|answer| answer, ReturnDecoding::Completes)
}

/// C85 expands the selected nested dictionary after merging its surrounding
/// entries. C86+ expands every nested list pair in place. The iterative stack
/// preserves that order without imposing a source-depth or host-stack limit.
fn merge_return_options<'w>(
    options: &mut ReturnOptions<'w>,
    value: ReturnOption<'w>,
    inline: bool,
) -> Result<(), ReturnDecoding> {
    let text = value
        .value
        .ok_or(ReturnDecoding::Unknown(ReturnUnknown::Options))?;
    let mut pending = vec![return_option_pairs(&text)?.into_iter()];
    let mut deferred: Vec<Option<String>> = Vec::new();
    while let Some(pairs) = pending.last_mut() {
        let Some((name, value)) = pairs.next() else {
            pending.pop();
            if !inline && let Some(value) = deferred.pop().flatten() {
                pending.push(return_option_pairs(&value)?.into_iter());
                deferred.push(None);
            }
            continue;
        };
        if name == "-options" {
            if inline {
                pending.push(return_option_pairs(&value)?.into_iter());
            } else if let Some(nested) = deferred.last_mut() {
                *nested = Some(value);
            } else {
                deferred.push(Some(value));
            }
        } else {
            options.select(
                &name,
                ReturnOption {
                    value: Some(std::borrow::Cow::Owned(value)),
                    ordinal: None,
                },
            );
        }
    }
    Ok(())
}

fn return_option_pairs(text: &str) -> Result<Vec<(String, String)>, ReturnDecoding> {
    let elements = WordValueRules::TCL
        .split_list(text)
        .map_err(|_| ReturnDecoding::Rejects)?;
    if elements.len() % 2 != 0 {
        return Err(ReturnDecoding::Rejects);
    }
    Ok(elements
        .chunks_exact(2)
        .map(|pair| (pair[0].to_string(), pair[1].to_string()))
        .collect())
}

fn read_return_code(text: &str, target: ReturnTarget) -> Result<CompletionCode, ReturnDecoding> {
    use crate::completion::{CompletionCodeSelection, resolve_completion_code_selector};
    match resolve_completion_code_selector(
        target.option_name(text),
        target.numbers,
        target.grammar.completion_code_policy(target.release),
    ) {
        CompletionCodeSelection::Exact(code) => Ok(code),
        CompletionCodeSelection::Invalid => Err(ReturnDecoding::Rejects),
        CompletionCodeSelection::Unknown => Err(ReturnDecoding::Unknown(ReturnUnknown::Release)),
    }
}

fn read_return_level(text: &str, target: ReturnTarget) -> Result<u32, ReturnDecoding> {
    use crate::completion::CompletionCodePolicy as Policy;
    let text = target.option_name(text);
    let Some(value) = target.numbers.parse_wide(text) else {
        return Err(not_an_integer(text, target.numbers));
    };
    let policy = target.grammar.completion_code_policy(target.release);
    if policy == Policy::Jim {
        if value < 0 {
            return Err(ReturnDecoding::Rejects);
        }
        return u32::try_from(value)
            .ok()
            .filter(|level| *level <= i32::MAX.unsigned_abs())
            .ok_or(ReturnDecoding::Unknown(ReturnUnknown::Release));
    }
    let lower = match policy {
        Policy::Tcl8 => -i64::from(u32::MAX),
        Policy::Tcl9 | Policy::Unknown => i64::from(i32::MIN),
        Policy::Jim => unreachable!("Jim levels use their separate width contract"),
    };
    if value < lower || value > i64::from(u32::MAX) {
        return Err(
            if policy == Policy::Unknown && value >= -i64::from(u32::MAX) && value < lower {
                ReturnDecoding::Unknown(ReturnUnknown::Release)
            } else {
                ReturnDecoding::Rejects
            },
        );
    }
    let [a, b, c, d, ..] = value.to_le_bytes();
    u32::try_from(i32::from_le_bytes([a, b, c, d])).map_err(|_| ReturnDecoding::Rejects)
}

fn not_an_integer(text: &str, numbers: Numbers) -> ReturnDecoding {
    let integer = |syntax: NumberSyntax| {
        matches!(
            Numbers::Target(syntax).parse_whole(text),
            Some(Number::Int(_) | Number::Big { .. })
        )
    };
    if numbers.syntax().is_none() && NumberSyntax::any(integer) {
        ReturnDecoding::Unknown(ReturnUnknown::Release)
    } else {
        ReturnDecoding::Rejects
    }
}

fn validate_return_list(text: &str, even: bool) -> Result<(), ReturnDecoding> {
    match WordValueRules::TCL.split_list(text) {
        Ok(elements) if !even || elements.len() % 2 == 0 => Ok(()),
        _ => Err(ReturnDecoding::Rejects),
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
/// A word the solver has not reached makes the answer pending. Exact options
/// dictionaries are merged by the shared decoder. A required computed word,
/// an unresolved native width or a source/compiler disagreement declines. The
/// result is exact where its selected original word is, and unproven otherwise.
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
        let mut target = ReturnTarget::for_profile(input.context().profile);
        if input.context().profile.is_some() {
            target.numbers = Numbers::Target(input.context().grammar.numbers);
        }
        let returned = match decode_return_in(target, count, &|at| {
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
                    || {
                        returned.merged_error_code.as_deref().map_or_else(
                            || ExactValueOrUnavailable::exact_text("NONE"),
                            ExactValueOrUnavailable::exact_text,
                        )
                    },
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

#[cfg(test)]
mod return_decoder_tests {
    use super::*;
    use crate::completion_route::ReturnInvocationGrammar as Grammar;
    use crate::invocation_words::InvocationWord::{KnownBytes as B, Literal as L};

    // Post-evaluation values of the exact native request cases. Counted NUL
    // values are binary-produced in the probe, not source NUL characters.
    const CASES: &[(&str, &[InvocationWord<'static>])] = &[
        ("empty", &[]),
        ("result", &[L("RESULT")]),
        ("ordinary_pair", &[L("foo"), L("bar")]),
        ("ordinary_pair_result", &[L("foo"), L("bar"), L("RESULT")]),
        ("ordinary_pairs", &[L("foo"), L("bar"), L("baz"), L("quux")]),
        ("lone_code", &[L("-code")]),
        ("lone_level", &[L("-level")]),
        ("lone_options", &[L("-options")]),
        ("lone_dashdash", &[L("--")]),
        ("dashdash_pair", &[L("--"), L("VALUE")]),
        ("code_ok", &[L("-code"), L("ok"), L("RESULT")]),
        ("code_error", &[L("-code"), L("error"), L("RESULT")]),
        ("code_return", &[L("-code"), L("return"), L("RESULT")]),
        ("code_break", &[L("-code"), L("break"), L("RESULT")]),
        ("code_continue", &[L("-code"), L("continue"), L("RESULT")]),
        ("code_signal", &[L("-code"), L("signal"), L("RESULT")]),
        ("code_exit", &[L("-code"), L("exit"), L("RESULT")]),
        ("code_eval", &[L("-code"), L("eval"), L("RESULT")]),
        ("code_prefix", &[L("-code"), L("o"), L("RESULT")]),
        ("code_010", &[L("-code"), L("010"), L("RESULT")]),
        ("code_uintmax", &[L("-code"), L("4294967295"), L("RESULT")]),
        (
            "code_negative_uintmax",
            &[L("-code"), L("-4294967295"), L("RESULT")],
        ),
        (
            "code_over_uintmax",
            &[L("-code"), L("4294967296"), L("RESULT")],
        ),
        ("code_intmin", &[L("-code"), L("-2147483648"), L("RESULT")]),
        (
            "code_bad_then_ok",
            &[L("-code"), L("BAD"), L("-code"), L("ok"), L("RESULT")],
        ),
        (
            "code_ok_then_bad",
            &[L("-code"), L("ok"), L("-code"), L("BAD"), L("RESULT")],
        ),
        ("level_zero", &[L("-level"), L("0"), L("RESULT")]),
        ("level_two", &[L("-level"), L("2"), L("RESULT")]),
        ("level_negative", &[L("-level"), L("-1"), L("RESULT")]),
        ("level_010", &[L("-level"), L("010"), L("RESULT")]),
        ("level_intmax", &[L("-level"), L("2147483647"), L("RESULT")]),
        (
            "level_over_intmax",
            &[L("-level"), L("2147483648"), L("RESULT")],
        ),
        (
            "level_uintmax",
            &[L("-level"), L("4294967295"), L("RESULT")],
        ),
        (
            "level_bad_then_zero",
            &[L("-level"), L("BAD"), L("-level"), L("0"), L("RESULT")],
        ),
        (
            "level_negative_then_zero",
            &[L("-level"), L("-1"), L("-level"), L("0"), L("RESULT")],
        ),
        (
            "errorcode_valid",
            &[
                L("-code"),
                L("error"),
                L("-errorcode"),
                L("A B"),
                L("RESULT"),
            ],
        ),
        ("errorcode_bad", &[L("-errorcode"), L("{"), L("RESULT")]),
        (
            "errorcode_bad_then_good",
            &[
                L("-errorcode"),
                L("{"),
                L("-errorcode"),
                L("A B"),
                L("RESULT"),
            ],
        ),
        ("errorinfo_any", &[L("-errorinfo"), L("{"), L("RESULT")]),
        (
            "errorstack_odd",
            &[L("-errorstack"), L("INNER"), L("RESULT")],
        ),
        (
            "errorstack_even",
            &[L("-errorstack"), L("INNER command"), L("RESULT")],
        ),
        ("errorstack_bad", &[L("-errorstack"), L("{"), L("RESULT")]),
        (
            "errorstack_bad_then_good",
            &[
                L("-errorstack"),
                L("{"),
                L("-errorstack"),
                L("INNER command"),
                L("RESULT"),
            ],
        ),
        ("custom_dash", &[L("-custom"), L("VALUE"), L("RESULT")]),
        ("custom_bare", &[L("custom"), L("VALUE"), L("RESULT")]),
        ("custom_dynamic_value", &[L("-custom"), L("V"), L("RESULT")]),
        ("options_empty", &[L("-options"), L(""), L("RESULT")]),
        (
            "options_code_ok",
            &[L("-options"), L("-code ok -level 0"), L("RESULT")],
        ),
        ("options_odd", &[L("-options"), L("-code"), L("RESULT")]),
        (
            "options_bad_code",
            &[L("-options"), L("-code BAD"), L("RESULT")],
        ),
        (
            "options_bad_overridden",
            &[
                L("-options"),
                L("-code BAD"),
                L("-code"),
                L("ok"),
                L("RESULT"),
            ],
        ),
        (
            "options_bad_level_overridden",
            &[
                L("-options"),
                L("-level -1"),
                L("-level"),
                L("0"),
                L("RESULT"),
            ],
        ),
        (
            "options_overrides_bad_code",
            &[
                L("-code"),
                L("BAD"),
                L("-options"),
                L("-code ok"),
                L("RESULT"),
            ],
        ),
        (
            "options_nested",
            &[
                L("-options"),
                L("-options {-code ok -level 0}"),
                L("RESULT"),
            ],
        ),
        (
            "options_code_key_bare",
            &[L("-options"), L("code error"), L("RESULT")],
        ),
        (
            "code_return_level_zero",
            &[L("-code"), L("return"), L("-level"), L("0"), L("RESULT")],
        ),
        (
            "code_return_level_two",
            &[L("-code"), L("return"), L("-level"), L("2"), L("RESULT")],
        ),
        (
            "code_break_level_zero",
            &[L("-code"), L("break"), L("-level"), L("0"), L("RESULT")],
        ),
        (
            "nul_key_code",
            &[B(b"-code\0tail"), L("error"), L("RESULT")],
        ),
        ("nul_code_ok", &[L("-code"), B(b"ok\0tail"), L("RESULT")]),
        (
            "nul_custom_key",
            &[B(b"custom\0tail"), L("VALUE"), L("RESULT")],
        ),
        (
            "level_negative_uintmax",
            &[L("-level"), L("-4294967295"), L("RESULT")],
        ),
        (
            "level_negative_over_intmin",
            &[L("-level"), L("-2147483649"), L("RESULT")],
        ),
        (
            "level_negative_over_uintmax",
            &[L("-level"), L("-4294967296"), L("RESULT")],
        ),
        (
            "level_over_uintmax",
            &[L("-level"), L("4294967296"), L("RESULT")],
        ),
        ("level_named_ok", &[L("-level"), L("ok"), L("RESULT")]),
        (
            "options_nested_before_outer_code",
            &[
                L("-options"),
                L("-options {-code error} -code ok"),
                L("RESULT"),
            ],
        ),
        (
            "options_nested_after_outer_code",
            &[
                L("-options"),
                L("-code ok -options {-code error}"),
                L("RESULT"),
            ],
        ),
        (
            "options_repeated_nested_bad_then_good",
            &[
                L("-options"),
                L("-options {-code BAD} -options {-code ok}"),
                L("RESULT"),
            ],
        ),
        (
            "options_repeated_nested_invalid_then_good",
            &[
                L("-options"),
                L("-options \\{ -options {-code ok}"),
                L("RESULT"),
            ],
        ),
        (
            "options_repeated_code_bad_then_good",
            &[L("-options"), L("-code BAD -code ok"), L("RESULT")],
        ),
        (
            "options_odd_then_empty",
            &[L("-options"), L("-code"), L("-options"), L(""), L("RESULT")],
        ),
        (
            "options_bad_list_then_empty",
            &[L("-options"), L("{"), L("-options"), L(""), L("RESULT")],
        ),
        (
            "options_errorcode_origin",
            &[
                L("-options"),
                L("-code error -level 0 -errorcode {MERGED CODE}"),
                L("RESULT"),
            ],
        ),
        (
            "nul_numeric_code",
            &[L("-code"), B(b"1\0tail"), L("RESULT")],
        ),
        (
            "nul_numeric_level",
            &[L("-level"), B(b"1\0tail"), L("RESULT")],
        ),
    ];

    struct Provider {
        profile: &'static str,
        source: &'static str,
        extension: &'static str,
        worker: &'static str,
    }

    const PROVIDERS: &[Provider] = &[
        Provider {
            profile: "tcl8.4",
            source: include_str!(
                "../../tests/data/native_return_option_pair_grammar379/8.4.20/stdout"
            ),
            extension: include_str!(
                "../../tests/data/native_return_option_pair_grammar380/8.4.20/stdout"
            ),
            worker: include_str!(
                "../../tests/data/native_return_evaluated_worker383/8.4.20/stdout"
            ),
        },
        Provider {
            profile: "tcl8.5",
            source: include_str!(
                "../../tests/data/native_return_option_pair_grammar379/8.5.19/stdout"
            ),
            extension: include_str!(
                "../../tests/data/native_return_option_pair_grammar380/8.5.19/stdout"
            ),
            worker: include_str!(
                "../../tests/data/native_return_evaluated_worker383/8.5.19/stdout"
            ),
        },
        Provider {
            profile: "tcl8.6",
            source: include_str!(
                "../../tests/data/native_return_option_pair_grammar379/8.6.18/stdout"
            ),
            extension: include_str!(
                "../../tests/data/native_return_option_pair_grammar380/8.6.18/stdout"
            ),
            worker: include_str!(
                "../../tests/data/native_return_evaluated_worker383/8.6.18/stdout"
            ),
        },
        Provider {
            profile: "tcl9.0",
            source: include_str!(
                "../../tests/data/native_return_option_pair_grammar379/9.0.4/stdout"
            ),
            extension: include_str!(
                "../../tests/data/native_return_option_pair_grammar380/9.0.4/stdout"
            ),
            worker: include_str!("../../tests/data/native_return_evaluated_worker383/9.0.4/stdout"),
        },
        Provider {
            profile: "tcl9.1",
            source: include_str!(
                "../../tests/data/native_return_option_pair_grammar379/9.1.0/stdout"
            ),
            extension: include_str!(
                "../../tests/data/native_return_option_pair_grammar380/9.1.0/stdout"
            ),
            worker: include_str!("../../tests/data/native_return_evaluated_worker383/9.1.0/stdout"),
        },
        Provider {
            profile: "jim",
            source: include_str!(
                "../../tests/data/native_return_option_pair_grammar379/jim/stdout"
            ),
            extension: include_str!(
                "../../tests/data/native_return_option_pair_grammar380/jim/stdout"
            ),
            worker: include_str!("../../tests/data/native_return_evaluated_worker383/jim/stdout"),
        },
    ];

    fn decoded(
        provider: &Provider,
        words: &[InvocationWord<'_>],
        facet: ReturnInvocationFacet,
    ) -> ReturnDecoding {
        let profile = DialectProfile::find(provider.profile).expect("measured provider grammar");
        let dialect = crate::InvocationDialect::of_profile(profile);
        let numbers = if provider.profile == "jim" {
            Numbers::Target(NumberSyntax::Jim)
        } else {
            Numbers::Target(dialect.numbers)
        };
        decode_return_words_in(
            Grammar::for_dialect(dialect),
            numbers,
            InvocationArguments::structured(words).with_dialect(dialect),
            facet,
        )
    }

    fn hex(text: &str) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        for byte in text.bytes() {
            write!(out, "{byte:02x}").expect("String formatting");
        }
        out
    }

    fn compare_public_fields(provider: &Provider, row: &str, facet: ReturnInvocationFacet) -> bool {
        let fields: Vec<_> = row.split('|').collect();
        assert_eq!(fields.len(), 8, "{}: {row}", provider.profile);
        let label = fields[1];
        let words = CASES
            .iter()
            .find(|(name, _)| *name == label)
            .expect("exact native case label")
            .1;
        let answer = decoded(provider, words, facet);
        let context = format!("{} {label} {facet:?}", provider.profile);
        if provider.profile == "jim"
            && matches!(
                label,
                "level_over_intmax" | "level_uintmax" | "level_over_uintmax"
            )
        {
            // The original public Jim rows are retained in full. This API has
            // no signed-wide level field, so none is fabricated from the u32
            // field, the observed catch cast or a C recipe.
            assert_eq!(
                answer,
                ReturnDecoding::Unknown(ReturnUnknown::Release),
                "{context}"
            );
            return false;
        }
        if provider.profile == "tcl8.5"
            && facet == ReturnInvocationFacet::OriginalSource
            && matches!(
                label,
                "options_nested_before_outer_code" | "options_repeated_nested_invalid_then_good"
            )
        {
            // Source380 and original-worker383 disagree at these exact rows.
            // Source values do not identify a selected compiler artifact.
            assert_eq!(
                answer,
                ReturnDecoding::Unknown(ReturnUnknown::Release),
                "{context}"
            );
            return false;
        }
        if provider.profile != "tcl8.4"
            && provider.profile != "jim"
            && label == "options_errorcode_origin"
        {
            // An accepted level-zero error also has catch code one. Its
            // supplied result and errorcode distinguish acceptance from a
            // grammar rejection; code one alone must not pass this control.
            assert!(matches!(answer, ReturnDecoding::Completes(_)), "{context}");
        }
        match answer {
            ReturnDecoding::Rejects => assert_eq!(fields[2], "1", "{context}"),
            ReturnDecoding::Unknown(reason) => {
                panic!("unresolved measured fields {context}: {reason:?}")
            }
            ReturnDecoding::Completes(returned) => {
                assert_eq!(
                    returned.own_code().as_int().to_string(),
                    fields[2],
                    "{context}"
                );
                if fields[4] != "ABSENT" {
                    assert_eq!(returned.code.as_int().to_string(), fields[4], "{context}");
                    assert_eq!(returned.level.to_string(), fields[5], "{context}");
                }
                let result = returned.result.map_or("", |at| {
                    return_word_text(match words[at] {
                        L(text) => ReturnWord::Text(text),
                        B(bytes) => ReturnWord::Bytes(bytes),
                        _ => panic!("exact result fixture"),
                    })
                    .expect("known result")
                });
                assert_eq!(hex(result), fields[3], "{context}");
                if returned.code == CompletionCode::Error
                    && fields[6] != "ABSENT"
                    && fields[4] != "ABSENT"
                {
                    let error_code = returned
                        .error_code
                        .and_then(|at| match words[at] {
                            L(text) => Some(text),
                            _ => None,
                        })
                        .or(returned.merged_error_code.as_deref())
                        .unwrap_or("NONE");
                    assert_eq!(hex(error_code), fields[6], "{context}");
                }
            }
        }
        true
    }

    #[test]
    fn source_return_decoder_matches_measured_public_fields_with_explicit_refusals() {
        // naming.completion.return-option-pair-grammar
        // docs/design/analysis/name-resolution-proofs/completion-return-option-pair-grammar.md
        // Original source379/380 only: own code, accepted result, and available
        // catch options. No compiler instruction, object, frame or message claim.
        for provider in PROVIDERS {
            let mut compared = 0;
            let mut count = 0;
            for row in provider
                .source
                .lines()
                .chain(provider.extension.lines())
                .filter(|line| line.starts_with("ROW|"))
            {
                compared += usize::from(compare_public_fields(
                    provider,
                    row,
                    ReturnInvocationFacet::OriginalSource,
                ));
                count += 1;
            }
            assert_eq!(count, 76);
            assert_eq!(
                compared,
                if provider.profile == "jim" {
                    73
                } else if provider.profile == "tcl8.5" {
                    74
                } else {
                    76
                }
            );
        }
    }

    #[test]
    fn evaluated_return_decoder_matches_original_worker_public_fields() {
        // naming.completion.return-evaluated-argument-worker-grammar
        // docs/design/analysis/name-resolution-proofs/completion-return-evaluated-argument-worker-grammar.md
        // Alias-to-original-worker383 keeps every original operand. This compares
        // public fields only, independently of source379/380 compilation.
        for provider in PROVIDERS {
            let rows: Vec<_> = provider
                .worker
                .lines()
                .filter(|line| line.starts_with("ROW|"))
                .collect();
            assert_eq!(rows.len(), 76);
            let compared = rows
                .into_iter()
                .filter(|row| {
                    compare_public_fields(provider, row, ReturnInvocationFacet::EvaluatedArguments)
                })
                .count();
            assert_eq!(compared, if provider.profile == "jim" { 73 } else { 76 });
        }
    }

    #[test]
    fn return_decoder_preserves_unknown_values_count_and_original_ordinals() {
        // naming.completion.return-option-pair-grammar
        // docs/design/analysis/name-resolution-proofs/completion-return-option-pair-grammar.md
        // Software contract: constructed source carriers test uncertainty and
        // provenance. They are not independently observed Native entry/header facts.
        let profile = DialectProfile::find("tcl8.6").expect("C86 grammar");
        let dialect = crate::InvocationDialect::of_profile(profile);
        let read = |words: &[InvocationWord<'_>]| {
            decode_return_words_in(
                Grammar::OptionsTcl,
                Numbers::Target(dialect.numbers),
                InvocationArguments::structured(words).with_dialect(dialect),
                ReturnInvocationFacet::OriginalSource,
            )
        };
        assert_eq!(
            read(&[L("-code"), InvocationWord::Dynamic]),
            ReturnDecoding::Unknown(ReturnUnknown::Word)
        );
        assert_eq!(
            read(&[InvocationWord::Expanded]),
            ReturnDecoding::Unknown(ReturnUnknown::Word)
        );
        assert_eq!(
            read(&[B(&[0xff]), L("ok")]),
            ReturnDecoding::Unknown(ReturnUnknown::Word)
        );
        assert_eq!(
            read(&[L("-options"), InvocationWord::Dynamic]),
            ReturnDecoding::Unknown(ReturnUnknown::Options)
        );
        let ReturnDecoding::Completes(custom) = read(&[
            InvocationWord::DynamicNonOption,
            InvocationWord::Dynamic,
            L("RESULT"),
        ]) else {
            panic!("non-reserved custom C pair");
        };
        assert_eq!(custom.result, Some(2));
        let ReturnDecoding::Completes(overridden) = read(&[
            L("-code"),
            InvocationWord::Dynamic,
            L("-code"),
            L("ok"),
            L("-errorcode"),
            L("A B"),
            L("RESULT"),
        ]) else {
            panic!("known final values");
        };
        assert_eq!(overridden.code, CompletionCode::Ok);
        assert_eq!(overridden.error_code, Some(5));
        assert_eq!(overridden.merged_error_code, None);
        assert_eq!(overridden.result, Some(6));
        let ReturnDecoding::Completes(merged) = read(&[
            L("-options"),
            L("-code error -level 0 -errorcode {MERGED CODE}"),
            L("RESULT"),
        ]) else {
            panic!("exact dictionary");
        };
        assert_eq!(merged.error_code, None);
        assert_eq!(merged.merged_error_code.as_deref(), Some("MERGED CODE"));
        assert_eq!(merged.result, Some(2));
    }

    #[test]
    fn return_value_transfer_keeps_actual_numeric_context_and_merged_error_code() {
        // naming.completion.return-option-pair-grammar
        // docs/design/analysis/name-resolution-proofs/completion-return-option-pair-grammar.md
        // Software overlay/route contract. A retained mixed numeral grammar is
        // not a newly measured C90 engine or a Native object/handler receipt.
        let profile = DialectProfile::find("tcl9.0").expect("selected C90 profile");
        let mut context = super::super::context::AnalysisContext::detached(Some(profile));
        context.grammar.numbers = NumberSyntax::Tcl85;
        let inputs = super::super::literal::LiteralInputs::new(
            "return",
            None,
            &["-code", "010", "RESULT"],
            Some(profile),
        )
        .with_context(context);
        let EvalAnswer::Evaluated(outcome) = RETURN.evaluate(&inputs, &mut Budget::evaluation())
        else {
            panic!("exact source value route");
        };
        assert_eq!(outcome.completion.options_code_and_level(), (8, 1));
        let inputs = super::super::literal::LiteralInputs::new(
            "return",
            None,
            &[
                "-options",
                "-code error -level 0 -errorcode {MERGED CODE}",
                "RESULT",
            ],
            Some(profile),
        );
        let EvalAnswer::Evaluated(outcome) = RETURN.evaluate(&inputs, &mut Budget::evaluation())
        else {
            panic!("exact dictionary route");
        };
        let CompletionOutcome::Error { error_code, .. } = outcome.completion else {
            panic!("selected level-zero error");
        };
        assert_eq!(
            error_code,
            ExactValueOrUnavailable::exact_text("MERGED CODE")
        );
    }

    #[test]
    fn return_decoder_release_and_invocation_facet_are_independent_premises() {
        // naming.completion.return-option-pair-grammar
        // docs/design/analysis/name-resolution-proofs/completion-return-option-pair-grammar.md
        // Software contract: a missing release/entry facet cannot be replaced by
        // numeral grammar or a source word. Public disagreement is in380/383.
        let values = [L("-errorstack"), L("INNER"), L("RESULT")];
        assert_eq!(
            decode_return_words_in(
                Grammar::OptionsTcl,
                Numbers::Target(NumberSyntax::Tcl85),
                InvocationArguments::structured(&values),
                ReturnInvocationFacet::OriginalSource
            ),
            ReturnDecoding::Unknown(ReturnUnknown::Release)
        );
        let nested = [
            L("-options"),
            L("-options {-code error} -code ok"),
            L("RESULT"),
        ];
        let dialect = crate::InvocationDialect::for_version(TclVersion::V8_5);
        let args = InvocationArguments::structured(&nested).with_dialect(dialect);
        assert_eq!(
            decode_return_words_in(
                Grammar::OptionsTcl,
                Numbers::Target(NumberSyntax::Tcl85),
                args,
                ReturnInvocationFacet::OriginalSource
            ),
            ReturnDecoding::Unknown(ReturnUnknown::Release)
        );
        let ReturnDecoding::Completes(worker) = decode_return_words_in(
            Grammar::OptionsTcl,
            Numbers::Target(NumberSyntax::Tcl85),
            args,
            ReturnInvocationFacet::EvaluatedArguments,
        ) else {
            panic!("explicit worker reading");
        };
        assert_eq!(worker.code, CompletionCode::Error);
        assert_eq!(worker.result, Some(2));
    }

    #[test]
    fn return_conversion_width_keeps_release_independent_of_numeral_overlay() {
        // naming.completion.return-option-pair-grammar
        // docs/design/analysis/name-resolution-proofs/completion-return-option-pair-grammar.md
        // Source379/380 measures the C8/C9 conversion-width distinction.
        // These mixed carriers separately test the software axis contract;
        // they do not claim a newly observed mixed Native engine or entry.
        let numbers = Numbers::Target(NumberSyntax::Tcl85);
        let c90 = crate::InvocationDialect::for_version(TclVersion::V9_0);
        let c86 = crate::InvocationDialect::for_version(TclVersion::V8_6);
        for option in ["-code", "-level"] {
            let values = [L(option), L("-4294967295"), L("RESULT")];
            let arguments = InvocationArguments::structured(&values);
            assert_eq!(
                decode_return_words_in(
                    Grammar::OptionsTcl,
                    numbers,
                    arguments.with_dialect(c90),
                    ReturnInvocationFacet::OriginalSource,
                ),
                ReturnDecoding::Rejects,
            );
            assert!(matches!(
                decode_return_words_in(
                    Grammar::OptionsTcl,
                    numbers,
                    arguments.with_dialect(c86),
                    ReturnInvocationFacet::OriginalSource,
                ),
                ReturnDecoding::Completes(_),
            ));
            assert_eq!(
                decode_return_words_in(
                    Grammar::OptionsTcl,
                    numbers,
                    arguments,
                    ReturnInvocationFacet::OriginalSource,
                ),
                ReturnDecoding::Unknown(ReturnUnknown::Release),
            );
        }
        let values = [L("-code"), L("010"), L("RESULT")];
        let ReturnDecoding::Completes(parsed) = decode_return_words_in(
            Grammar::OptionsTcl,
            numbers,
            InvocationArguments::structured(&values).with_dialect(c90),
            ReturnInvocationFacet::OriginalSource,
        ) else {
            panic!("the retained numeral overlay still reads octal");
        };
        assert_eq!(parsed.code, CompletionCode::Other(8));
    }

    #[test]
    fn registry_return_routes_keep_release_grammar_and_numerals_independent() {
        use crate::completion_route::{InvocationCompletionRoute as Route, ReturnCompletionRoute};

        // naming.completion.return-option-pair-grammar
        // docs/design/analysis/name-resolution-proofs/completion-return-option-pair-grammar.md
        // Actual Registry consumer contracts under explicit mixed carriers.
        // These source advice queries prove no Native entry or object state.
        let values = [L("-options"), L("-code ok -level 0"), L("RESULT")];
        for (name, version, numbers, expected) in [
            (
                "tcl9.0",
                TclVersion::V9_0,
                NumberSyntax::Tcl84,
                Route::Tcl(CompletionCode::Ok),
            ),
            (
                "tcl8.4",
                TclVersion::V8_4,
                NumberSyntax::Tcl90,
                Route::Tcl(CompletionCode::Error),
            ),
        ] {
            let registry = crate::model::ingress::static_context_for(name).commands();
            let mut dialect = crate::InvocationDialect::for_version(version);
            dialect.numbers = numbers;
            let arguments = InvocationArguments::structured(&values).with_dialect(dialect);
            assert_eq!(
                registry.invocation_completion_route("return", arguments, None),
                Some(expected),
            );
            assert_eq!(
                matches!(
                    registry.return_completion(arguments),
                    ReturnDecoding::Completes(_)
                ),
                version != TclVersion::V8_4,
            );
        }
        let values = [L("-code"), L("-4294967295"), L("RESULT")];
        for (name, version, numbers, expected) in [
            (
                "tcl9.0",
                TclVersion::V9_0,
                NumberSyntax::Tcl85,
                Route::Tcl(CompletionCode::Error),
            ),
            (
                "tcl8.6",
                TclVersion::V8_6,
                NumberSyntax::Tcl90,
                Route::Return(ReturnCompletionRoute {
                    eventual_code: CompletionCode::Error,
                    remaining_level: 1,
                }),
            ),
        ] {
            let registry = crate::model::ingress::static_context_for(name).commands();
            let mut dialect = crate::InvocationDialect::for_version(version);
            dialect.numbers = numbers;
            let arguments = InvocationArguments::structured(&values).with_dialect(dialect);
            assert_eq!(
                registry.invocation_completion_route("return", arguments, None),
                Some(expected),
            );
        }
    }

    #[test]
    fn return_decoder_keeps_unknown_grammar_and_numbers_as_separate_premises() {
        // naming.completion.return-option-pair-grammar
        // docs/design/analysis/name-resolution-proofs/completion-return-option-pair-grammar.md
        // Software consensus contract: provider grammar and numeral knowledge
        // are independent; agreement supplies no worker or compiler entry.
        let values = [L("RESULT")];
        let ReturnDecoding::Completes(common) = decode_return_words_in(
            Grammar::Unknown,
            Numbers::Unknown,
            InvocationArguments::structured(&values),
            ReturnInvocationFacet::OriginalSource,
        ) else {
            panic!("a result-only return needs neither numeric nor option selection");
        };
        assert_eq!(common.code, CompletionCode::Ok);
        assert_eq!(common.level, 1);
        assert_eq!(common.result, Some(0));
        let values = [L("-code"), L("return"), L("RESULT")];
        assert_eq!(
            decode_return_words_in(
                Grammar::Unknown,
                Numbers::Target(NumberSyntax::Tcl85),
                InvocationArguments::structured(&values),
                ReturnInvocationFacet::OriginalSource,
            ),
            ReturnDecoding::Unknown(ReturnUnknown::Release),
        );
        let values = [L("-code"), L("010"), L("RESULT")];
        assert_eq!(
            decode_return_words_in(
                Grammar::OptionsTcl,
                Numbers::Unknown,
                InvocationArguments::structured(&values)
                    .with_dialect(crate::InvocationDialect::for_version(TclVersion::V8_6)),
                ReturnInvocationFacet::OriginalSource,
            ),
            ReturnDecoding::Unknown(ReturnUnknown::Release),
        );
        let ReturnDecoding::Completes(common) = decode_return_words_in(
            Grammar::Unknown,
            Numbers::Target(NumberSyntax::Tcl85),
            InvocationArguments::structured(&values),
            ReturnInvocationFacet::OriginalSource,
        ) else {
            panic!("the explicit numeral overlay agrees at this narrow code");
        };
        assert_eq!(common.code, CompletionCode::Other(8));
    }
}
