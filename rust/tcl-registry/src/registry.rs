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

//! Command registry — lookup facade.
//!
//! Built once at startup from command spec modules, then queried by
//! every consumer. Supports dialect filtering and trait-membership
//! queries.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock, PoisonError, RwLock};

use rustc_hash::{FxHashMap, FxHashSet};

use crate::abbrev::{Keyword, KeywordTable, PrefixMatching};
use crate::arg_role::{AppendedArity, ArgRole};
use crate::arity::Arity;
use crate::body_kind::BodyKind;
use crate::definer::DefinitionBodyGrammar;
use crate::events::{
    DataCollectionAction, DataCollectionOperation, DataCollectionProtocol, EventHandlerPriority,
};
use crate::forms::CommandForm;
use crate::hooks::{AnalyserHookId, CodegenHookId, InlineCodegenHookId, LoweringHookId};
use crate::hover::CallbackTaintInput;
use crate::invocation_words::{
    CommandPrefixArguments, InvocationWord, VariableReadProjection, VariableWriteProjection,
};
use crate::lifecycle::{Lifecycle, LifecycleState};
use crate::resolved_invocation::{
    InvocationResolutionUnresolved, ResolvedInvocation, ResolvedSubcommand,
    StructuredInvocationResolution, SubcommandResolution,
};
use crate::side_effects::SideSwitchTarget;
use crate::spec::{BytePayloadSpec, CommandSpec, SubCommand};
use crate::stamp_window::StampSelection;
use crate::state_transition::{StateTransition, StateTransitions, TransitionSubject};
use crate::traits::Traits;
use crate::types::VarWriteTyping;
use crate::value_transfer::completion::ReturnDecoding;
use crate::{InvocationArguments, InvocationWords};
use tcl_dialect::model::Family;
use tcl_dialect::model::PackageFloor;
use tcl_dialect::model::SurfaceQuery;
use tcl_dialect::model::surface_admits;

#[path = "registry_regex_hazards.rs"]
mod regex_hazards;
#[path = "registry_role_consensus.rs"]
mod role_consensus;
use tcl_dialect::model::surface_breadth;
use tcl_dialect::model::surface_nearness;
use tcl_dialect::model::{SpecProvider, SpecSurface, SurfaceLayer, surface_provided_by};
use tcl_dialect::version_satisfies;

/// Resolved metadata for an iRules event — the result of
/// [`CommandRegistry::event_info`].
#[derive(Debug, Clone)]
pub struct EventInfo {
    /// The upper-cased event name as queried.
    pub event: String,
    /// Introduction / deprecation / retirement releases on the BIG-IP axis —
    /// explicit data, with an absent introducing release inheriting the axis
    /// baseline (15.0.0). Entirely unspecified for an unknown event.
    pub lifecycle: Lifecycle,
    /// The event's lifecycle state at the queried BIG-IP release.
    pub lifecycle_state: LifecycleState,
    /// Whether the event is a recognised iRules event.
    pub known: bool,
    /// `"init"` / `"once_per_connection"` / `"per_request"` / `"unknown"`.
    pub multiplicity: &'static str,
    /// Description prose, or `""` when none is recorded.
    pub description: String,
    /// Connection-side label, or `"unknown"` for an unrecognised event.
    pub side: &'static str,
    /// Transport string (`"tcp"`, `"tcp/udp"`, `""`), or `None` for an
    /// unrecognised event.
    pub transport: Option<String>,
    /// Profile types implied by the event, sorted.
    pub implied_profiles: Vec<&'static str>,
    /// Sorted names of every command valid in this event (empty when the
    /// event is unknown).
    pub valid_commands: Vec<String>,
}

/// How a registry-described control-flow body relates to its enclosing call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlArmSemantics {
    /// The body always runs once the enclosing command is reached.
    Always,
    /// Run-time selection decides whether the body runs.
    Selected,
    /// The body runs, but in a frame that cannot inherit caller locals.
    FrameBoundary,
    /// The body runs, but its completion is contained by the enclosing call.
    CompletionBoundary,
    /// The body is repeated over a **finite** collection the invocation
    /// itself supplies — `foreach` / `lmap` / the `foreach`-line family.
    ///
    /// Like [`Self::Uncertain`] it names no *selectable* execution: which
    /// iteration (or whether any at all) a body run belongs to is not
    /// statically decidable, so nothing may be read out of the body's
    /// environment. It differs in the one fact that *is* decidable — the loop
    /// runs a bounded number of times, so the construct terminates whenever
    /// its body does. A consumer asking "does control reach the next
    /// statement?" can therefore answer from the body alone, which it cannot
    /// do for [`Self::Uncertain`].
    BoundedIteration,
    /// The body is conditional or repeated in a way this descriptor does not
    /// statically select, and whose trip count it cannot bound either — a
    /// condition-driven `for` / `while`.
    Uncertain,
}

/// Registry-parsed completion selector for an `on` clause in `try`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryCompletionSelector {
    /// Normal completion (`ok` / code 0).
    Ok,
    /// Error completion (`error` / code 1).
    Error,
    /// Procedure return completion (`return` / code 2).
    Return,
    /// Loop break completion (`break` / code 3).
    Break,
    /// Loop continue completion (`continue` / code 4).
    Continue,
    /// A valid numeric code outside the named core codes.
    Numeric(i32),
}

/// Registry-parsed kind of one `try` clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryClauseKind {
    /// `on code variableList script`.
    On(TryCompletionSelector),
    /// `trap pattern variableList script`.
    Trap,
    /// `finally script`.
    Finally,
}

/// Registry-owned word layout for one validated `try` clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TryControlClause {
    /// Typed clause kind and, for `on`, its completion selector.
    pub kind: TryClauseKind,
    /// Selector word (`code` / error-code pattern), absent for `finally`.
    pub selector_index: Option<usize>,
    /// Handler variable-list word, absent for `finally`.
    pub variable_list_index: Option<usize>,
    /// Handler or finally body word.
    pub body_index: usize,
    /// Whether this handler's body is the `-` fallthrough marker.
    pub fallthrough: bool,
}

impl TryCompletionSelector {
    /// Match a concrete captured route; dynamic routes retain both possibilities.
    #[must_use]
    pub fn matches_route(
        self,
        route: crate::completion_route::InvocationCompletionRoute,
    ) -> Option<bool> {
        let expected = match self {
            Self::Ok => crate::completion::CompletionCode::Ok,
            Self::Error => crate::completion::CompletionCode::Error,
            Self::Return => crate::completion::CompletionCode::Return,
            Self::Break => crate::completion::CompletionCode::Break,
            Self::Continue => crate::completion::CompletionCode::Continue,
            Self::Numeric(code) => crate::completion::CompletionCode::from_int(code),
        };
        route.immediate_code().map(|code| code == expected)
    }
}

/// Registry-owned parsed layout of a complete valid `try` invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TryControlInvocation {
    /// Main try-body word.
    pub body_index: usize,
    /// Handler clauses followed by an optional final `finally` clause.
    pub clauses: Vec<TryControlClause>,
}

/// The completion effect of one concrete registry invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationCompletion {
    /// Execution may continue with the following statement.
    FallsThrough,
    /// A normal procedure result, optionally naming its result argument.
    ReturnsResult(Option<usize>),
    /// A non-normal or otherwise non-result completion ends this path.
    Terminates,
    /// Dynamic operands or unknown completion grammar prevent classification.
    Unknown,
}

/// An exact control completion available from registry-declared semantics and
/// literal invocation options.
///
/// Unlike [`InvocationCompletion`], this retains the Tcl completion code a
/// surrounding `try` may match.  `ProcessExit` is intentionally separate:
/// `exit` does not produce a catchable Tcl completion and bypasses `finally`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactInvocationCompletion {
    /// A Tcl completion code propagated through enclosing control constructs.
    Tcl(crate::completion::CompletionCode),
    /// Immediate interpreter-process termination.
    ProcessExit,
}

/// Registry-owned completion knowledge for a source-aware invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationCompletionKnowledge {
    /// A fully determined Tcl completion or process exit.
    Exact(ExactInvocationCompletion),
    /// `return -code $value` can either propagate return or reject the code.
    DynamicReturnOrError,
    /// A dynamic `exit` status either terminates the process or raises
    /// catchable Tcl error; it never has a normal continuation.
    ExitOrError,
    /// Jim's dynamic exit status produces native code 6 or validation error.
    CatchableExitOrError,
    /// The invocation has a runtime-dependent completion.
    Dynamic,
}

/// Parse the literal option subset of `return` that fixes the completion code
/// visible to an enclosing `try`.  With the default `-level 1`, `return`
/// itself propagates `TCL_RETURN` even when its eventual procedure result is
/// configured as `-code error`; `-level 0` exposes that configured code to the
/// immediately enclosing script instead.
/// The literal-return parser distinguishes an invocation whose outcome is
/// genuinely runtime-dependent from one Tcl will reject before it can return.
/// The latter is still a precise `TCL_ERROR`, so an enclosing `try on error`
/// must receive it.
enum ExactReturnCompletion {
    Completion(crate::completion_route::InvocationCompletionRoute),
    StaticError,
    Dynamic,
}

/// Parse `exit ?returnCode?` after its descriptor has established the exact
/// one-word-or-omitted shape.  Tcl 8.x passes its argument through
/// `Tcl_GetIntFromObj`, which accepts `-UINT_MAX..=UINT_MAX` before the C cast;
/// Tcl 9.0+ instead uses `TclGetWideBitsFromObj`, accepting every integer
/// (including bignums) before truncating its low bits to the C exit status.
/// A literal rejected by its release's conversion is an ordinary catchable
/// error, not a process termination.
enum ExactProcessExitCompletion {
    ProcessExit,
    StaticError,
    Dynamic,
}

fn exact_process_exit_completion(
    args: crate::invocation_words::InvocationArguments<'_>,
    numbers: tcl_syntax::number::Numbers,
    conversion: Option<tcl_dialect::ProcessExitConversion>,
) -> ExactProcessExitCompletion {
    match args.get(0) {
        None => ExactProcessExitCompletion::ProcessExit,
        Some(_) => match args.literal_at(0) {
            Some(value)
                if conversion.is_some_and(|conversion| {
                    numbers.parse_exit_status(value, conversion).is_some()
                }) =>
            {
                ExactProcessExitCompletion::ProcessExit
            }
            Some(_) => ExactProcessExitCompletion::StaticError,
            None => ExactProcessExitCompletion::Dynamic,
        },
    }
}

/// Completion shape of an already selected descriptor and frozen arguments.
/// This is metadata only; runtime selection and executed completion are separate.
pub(crate) fn invocation_completion_for_selected(
    resolved: &ResolvedInvocation<'_, '_>,
    numbers: tcl_syntax::number::Numbers,
    grammar: crate::completion_route::ReturnInvocationGrammar,
) -> InvocationCompletion {
    use crate::completion::CompletionCode;
    use crate::completion_route::InvocationCompletionRoute as Route;
    let args = resolved.words.arguments();
    match resolved.argument_count_for_arity() {
        Some(count) if !resolved.semantics.arity.accepts(count) => {
            return InvocationCompletion::Terminates;
        }
        None => return InvocationCompletion::Unknown,
        Some(_) => {}
    }
    if resolved.semantics.operation
        == crate::SemanticOperationId::StructuredLowering(LoweringHookId::Return)
    {
        let mut result = crate::native_result::NativeResultSelection::Unknown;
        return match exact_return_completion_with_result(args, numbers, grammar, &mut result) {
            ExactReturnCompletion::Completion(Route::Tcl(CompletionCode::Ok)) => {
                InvocationCompletion::FallsThrough
            }
            ExactReturnCompletion::Completion(Route::Return(pending))
                if pending.eventual_code == CompletionCode::Ok && pending.remaining_level == 1 =>
            {
                InvocationCompletion::ReturnsResult(match result {
                    crate::native_result::NativeResultSelection::Argument(index) => Some(index),
                    _ => None,
                })
            }
            ExactReturnCompletion::Completion(_) | ExactReturnCompletion::StaticError => {
                InvocationCompletion::Terminates
            }
            ExactReturnCompletion::Dynamic => InvocationCompletion::Unknown,
        };
    }
    let traits = resolved.semantics.traits;
    if traits.intersects(
        Traits::TERMINATES_BLOCK
            | Traits::BREAKS_LOOP
            | Traits::CONTINUES_LOOP
            | Traits::REPLACES_FRAME,
    ) {
        InvocationCompletion::Terminates
    } else {
        InvocationCompletion::FallsThrough
    }
}

fn exact_return_completion(
    args: crate::invocation_words::InvocationArguments<'_>,
    numbers: tcl_syntax::number::Numbers,
    grammar: crate::completion_route::ReturnInvocationGrammar,
) -> ExactReturnCompletion {
    exact_return_completion_with_result(
        args,
        numbers,
        grammar,
        &mut crate::native_result::NativeResultSelection::Unknown,
    )
}

fn exact_return_completion_with_result(
    args: crate::invocation_words::InvocationArguments<'_>,
    numbers: tcl_syntax::number::Numbers,
    grammar: crate::completion_route::ReturnInvocationGrammar,
    result_argument: &mut crate::native_result::NativeResultSelection,
) -> ExactReturnCompletion {
    use crate::completion::CompletionCode;
    use crate::completion_route::{InvocationCompletionRoute as Route, ReturnCompletionRoute};
    use crate::native_result::NativeResultSelection as ResultSelection;
    use crate::value_transfer::completion::{
        ReturnDecoding, ReturnInvocationFacet, decode_return_words_in,
    };

    // These source and metadata queries retain no selected compiler artifact
    // or evaluated-worker entry. A native dialect alone cannot supply it.
    match decode_return_words_in(
        grammar,
        numbers,
        args,
        ReturnInvocationFacet::OriginalSource,
    ) {
        ReturnDecoding::Completes(completion) => {
            *result_argument = completion
                .result
                .map_or(ResultSelection::EmptyString, ResultSelection::Argument);
            let route = if completion.level == 0 && completion.code != CompletionCode::Return {
                Route::Tcl(completion.code)
            } else {
                Route::Return(ReturnCompletionRoute {
                    eventual_code: completion.code,
                    remaining_level: u64::from(completion.level),
                })
            };
            ExactReturnCompletion::Completion(route)
        }
        ReturnDecoding::Rejects => ExactReturnCompletion::StaticError,
        ReturnDecoding::Unknown(_) => ExactReturnCompletion::Dynamic,
    }
}

/// Validity of statically known native return options, using the completion
/// owner's option parser. Unknown option dictionaries stay unresolved.
pub(crate) fn native_return_result_selection(
    args: crate::InvocationArguments<'_>,
) -> crate::native_result::NativeResultSelection {
    use crate::completion_route::ReturnInvocationGrammar as Grammar;
    use crate::native_result::NativeResultSelection as Selection;
    let Some(dialect) = args.dialect() else {
        return Selection::Unknown;
    };
    let grammar = Grammar::for_dialect(dialect);
    let mut result = Selection::Unknown;
    match exact_return_completion_with_result(
        args,
        tcl_syntax::number::Numbers::Target(dialect.numbers),
        grammar,
        &mut result,
    ) {
        ExactReturnCompletion::Completion(_) => result,
        ExactReturnCompletion::StaticError => Selection::InvalidArguments,
        ExactReturnCompletion::Dynamic => Selection::Unknown,
    }
}

fn return_has_only_dynamic_code(args: crate::invocation_words::InvocationArguments<'_>) -> bool {
    let Some(len) = args.exact_argv_len() else {
        return false;
    };
    let mut index = 0;
    let mut dynamic_code = false;
    while index + 1 < len {
        match args.literal_at(index) {
            Some("-code") => {
                dynamic_code |= args.literal_at(index + 1).is_none();
            }
            Some("-errorinfo" | "-errorcode") => {}
            // Unknown option spellings may select -level 0 or -options;
            // only the authored names above prove the default level remains.
            _ => return false,
        }
        index += 2;
    }
    dynamic_code
}

/// Resolve return's state envelope through the same native completion parser.
/// An ordinary result may be dynamic without making the option grammar dynamic.
#[must_use]
pub fn native_return_state_effect(
    args: crate::InvocationArguments<'_>,
) -> crate::completion_route::ReturnStateEffect {
    use crate::completion::CompletionCode;
    use crate::completion_route::{
        InvocationCompletionRoute as Route, ReturnInvocationGrammar as Grammar,
        ReturnStateEffect as Effect,
    };
    let Some(dialect) = args.dialect() else {
        return Effect::MayMaterialiseError;
    };
    // Source-only family and numeric grammar do not select a Native handler.
    if dialect.native_name_protocol().is_none() {
        return Effect::MayMaterialiseError;
    }
    let grammar = Grammar::for_dialect(dialect);
    if grammar == Grammar::Unknown || args.exact_argv_len().is_none() {
        return Effect::MayMaterialiseError;
    }
    match exact_return_completion(
        args,
        tcl_syntax::number::Numbers::Target(dialect.numbers),
        grammar,
    ) {
        ExactReturnCompletion::Completion(Route::Tcl(CompletionCode::Ok)) => {
            Effect::ResultAndCompletion
        }
        ExactReturnCompletion::Completion(Route::Return(route))
            if route.eventual_code == CompletionCode::Ok =>
        {
            Effect::ResultAndCompletion
        }
        _ => Effect::MayMaterialiseError,
    }
}

fn native_tailcall_completion(
    args: crate::InvocationArguments<'_>,
    frame: Option<crate::VariableAliasFrame>,
) -> crate::completion_route::InvocationCompletionRoute {
    use crate::completion::CompletionCode as Code;
    use crate::completion_route::InvocationCompletionRoute as Route;
    use tcl_dialect::model::Family;
    let family = args.dialect().and_then(crate::InvocationDialect::family);
    if !matches!(family, Some(Family::Tcl | Family::Jim)) {
        return Route::Unknown;
    }
    if frame == Some(crate::VariableAliasFrame::Global) {
        return Route::Tcl(Code::Error);
    }
    if family == Some(Family::Jim) {
        return match args.exact_argv_len() {
            Some(0) if frame == Some(crate::VariableAliasFrame::Procedure) => Route::Tcl(Code::Ok),
            Some(0) => Route::TclAlternatives(&[Code::Ok, Code::Error]),
            Some(_) => Route::TailcallOrError {
                code: Code::Other(7),
            },
            None => Route::Unknown,
        };
    }
    Route::TailcallOrError { code: Code::Return }
}

fn parse_try_completion_selector(
    selector: &str,
    numbers: tcl_syntax::number::Numbers,
    policy: crate::completion::CompletionCodePolicy,
) -> Option<TryCompletionSelector> {
    let crate::completion::CompletionCodeSelection::Exact(code) =
        crate::completion::resolve_completion_code_selector(selector, numbers, policy)
    else {
        return None;
    };
    Some(match code {
        crate::completion::CompletionCode::Ok => TryCompletionSelector::Ok,
        crate::completion::CompletionCode::Error => TryCompletionSelector::Error,
        crate::completion::CompletionCode::Return => TryCompletionSelector::Return,
        crate::completion::CompletionCode::Break => TryCompletionSelector::Break,
        crate::completion::CompletionCode::Continue => TryCompletionSelector::Continue,
        crate::completion::CompletionCode::Other(code) => TryCompletionSelector::Numeric(code),
    })
}

/// Select closed try clauses from frozen native operands and the selected code grammar.
/// Unknown values or unsupported clauses retain no execution topology proof.
#[must_use]
pub fn selected_try_control_invocation(
    arguments: crate::InvocationArguments<'_>,
    offset: usize,
) -> Option<TryControlInvocation> {
    let dialect = arguments.dialect()?;
    let values = arguments.slice_from(offset).literal_values()?;
    let plan = crate::commands::tcl::NATIVE_TRY_GRAMMAR
        .walk_arguments(arguments.slice_from(offset), &[], dialect.authoring_query())?
        .ok()?;
    let mut selected = parse_try_control_invocation(
        &plan,
        &values,
        tcl_syntax::number::Numbers::Target(dialect.numbers),
        dialect.completion_code_policy(),
    )?;
    selected.body_index += offset;
    for clause in &mut selected.clauses {
        clause.body_index += offset;
        clause.selector_index = clause.selector_index.map(|index| index + offset);
        clause.variable_list_index = clause.variable_list_index.map(|index| index + offset);
    }
    Some(selected)
}

pub(crate) fn parse_try_control_invocation(
    plan: &crate::ClausePlan,
    args: &[&str],
    numbers: tcl_syntax::number::Numbers,
    policy: crate::completion::CompletionCodePolicy,
) -> Option<TryControlInvocation> {
    // The clause grammar's walk decides the chain — which words introduce a
    // handler, which is `finally`, where the chain stops making sense — and
    // this reads each clause by its timing and its handler vocabulary, never
    // by a keyword.
    if plan.defect.is_some() {
        return None;
    }
    let (head, rest) = plan.clauses.split_first()?;
    let body_index = head.operand(ArgRole::Body)?;
    let mut clauses = Vec::with_capacity(rest.len());
    for (offset, clause) in rest.iter().enumerate() {
        let body_index = clause.operand(ArgRole::Body)?;
        match clause.timing {
            crate::clause_grammar::ClauseTiming::Always => clauses.push(TryControlClause {
                kind: TryClauseKind::Finally,
                selector_index: None,
                variable_list_index: None,
                body_index,
                fallthrough: false,
            }),
            crate::clause_grammar::ClauseTiming::Selected => {
                let (selector_index, handler) = clause.handler()?;
                let selector = *args.get(selector_index)?;
                let kind = match handler {
                    crate::value_transfer::HandlerMatch::CompletionCode => {
                        TryClauseKind::On(parse_try_completion_selector(selector, numbers, policy)?)
                    }
                    crate::value_transfer::HandlerMatch::ErrorCodePrefix => {
                        if tcl_syntax::naming::is_dynamic_word(selector)
                            || tcl_syntax::list::split_list(selector).is_err()
                        {
                            return None;
                        }
                        TryClauseKind::Trap
                    }
                };
                let variable_list_index = clause.operand(ArgRole::LoopVarList)?;
                let variable_list = *args.get(variable_list_index)?;
                if tcl_syntax::naming::is_dynamic_word(variable_list) {
                    return None;
                }
                if tcl_syntax::list::split_list(variable_list).ok()?.len() > 2 {
                    return None;
                }
                // A marker with no later handler to run is Tcl's "last
                // non-finally clause must not have a body of `-`".
                let fallthrough = plan.falls_through(offset + 1);
                if fallthrough && clause.falls_through_to.is_none() {
                    return None;
                }
                clauses.push(TryControlClause {
                    kind,
                    selector_index: Some(selector_index),
                    variable_list_index: Some(variable_list_index),
                    body_index,
                    fallthrough,
                });
            }
            _ => return None,
        }
    }
    Some(TryControlInvocation {
        body_index,
        clauses,
    })
}

/// Whether a typed control invocation's clause chain is well formed: the
/// clause grammar's walk, else the `clause_shape_check` escape hatch. `None`
/// means the command states no chain grammar at all.
fn control_chain_is_well_formed(
    spec: &CommandSpec,
    args: &[&str],
    dialect: Option<SurfaceQuery<'_>>,
) -> Option<bool> {
    if let Some(plan) = spec.clause_plan(args, dialect) {
        return Some(plan.defect.is_none());
    }
    spec.clause_shape_check
        .map(|check| check(InvocationArguments::literals(args)).is_none())
}

fn try_control_arms(
    plan: &crate::ClausePlan,
    args: &[&str],
    numbers: tcl_syntax::number::Numbers,
) -> Option<Vec<(usize, ControlArmSemantics)>> {
    let invocation = parse_try_control_invocation(
        plan,
        args,
        numbers,
        crate::completion::CompletionCodePolicy::for_numbers(numbers),
    )?;
    let mut arms = vec![(invocation.body_index, ControlArmSemantics::Always)];
    arms.extend(invocation.clauses.into_iter().filter_map(|clause| {
        (!clause.fallthrough).then_some((
            clause.body_index,
            if clause.kind == TryClauseKind::Finally {
                ControlArmSemantics::Always
            } else {
                ControlArmSemantics::Selected
            },
        ))
    }));
    Some(arms)
}

impl EventInfo {
    /// Number of commands valid in this event.
    #[must_use]
    pub fn valid_command_count(&self) -> usize {
        self.valid_commands.len()
    }
}

/// Number of commands declaring a taint source — computed at compile
/// time so [`TAINT_SOURCE_INDEX`] can be a fixed-size `const` array.
const fn count_taint_sources(specs: &[CommandSpec]) -> usize {
    let mut n = 0;
    let mut i = 0;
    while i < specs.len() {
        if specs[i].taint_source.is_some() {
            n += 1;
        }
        i += 1;
    }
    n
}

/// Build the taint-source index at compile time by scanning the const
/// [`crate::commands::irules::IRULES_SPECS`] array for every spec's
/// [`crate::CommandSpec::taint_source`].
const fn build_taint_source_index()
-> [(&'static str, crate::taint::TaintColour); TAINT_SOURCE_COUNT] {
    let specs = crate::commands::irules::IRULES_SPECS;
    let mut out = [("", crate::taint::TaintColour::empty()); TAINT_SOURCE_COUNT];
    let mut i = 0;
    let mut k = 0;
    while i < specs.len() {
        if let Some(colour) = specs[i].taint_source {
            out[k] = (specs[i].name, colour);
            k += 1;
        }
        i += 1;
    }
    out
}

const TAINT_SOURCE_COUNT: usize = count_taint_sources(crate::commands::irules::IRULES_SPECS);

/// The taint-source index: command name → getter-form source colour, a
/// **compile-time** table derived from every iRules spec's
/// [`crate::CommandSpec::taint_source`] — the data's single home is each
/// `CommandSpec`, so this never drifts from the spec definitions.
///
/// Independent of which dialects a registry has loaded: a `tcl8.6`
/// document still
/// sees `HTTP::path` as a source. (The core Tcl sources `gets` / `read` /
/// `exec` / … are classified by [`crate::Traits::TAINT_SOURCE`] instead,
/// so they carry no index entry.)
const TAINT_SOURCE_INDEX: [(&str, crate::taint::TaintColour); TAINT_SOURCE_COUNT] =
    build_taint_source_index();

/// Where the words of one procedure definition sit — see
/// [`CommandRegistry::procedure_definition_words`]. Indices count words after
/// the command head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcedureWords {
    /// The procedure's name.
    pub name: usize,
    /// Its parameter list.
    pub params: usize,
    /// Its static-variable list, for a definer that takes one and a call that
    /// supplies it.
    pub statics: Option<usize>,
    /// Its body.
    pub body: usize,
}

impl ProcedureWords {
    /// Tcl's `proc name args body`, for a consumer that has no registry to ask.
    pub const TCL_PROC: Self = Self {
        name: 0,
        params: 1,
        statics: None,
        body: 2,
    };
}

/// Lookup facade over command specs.
///
/// The registry is built once from the command spec modules and then
/// queried read-only. All command-specific knowledge lives in the
/// specs — consumers never match on command name strings.
pub struct CommandRegistry {
    by_name: FxHashMap<&'static str, Vec<&'static CommandSpec>>,
    /// Specs installed through the public insertion seam, in registration
    /// order. These are authored overlays (including `SpecTcl` commands), not
    /// rows from the compiled-in command universe.
    ///
    /// Keeping that provenance beside the index lets an explicit-profile
    /// compiler projection filter the shipped surface exactly while retaining
    /// an authored `surface: None` override as the effective command binding.
    overlay_specs: Vec<&'static CommandSpec>,
    loaded_layers: Vec<SurfaceLayer>,
    /// The dialect profile this registry was built for, when it came from
    /// `registry_for_profile` / `registry_for_dialect`. `None` for
    /// hand-assembled registries (tests, ad-hoc tools), which fall back to
    /// the `loaded_layers`-derived behaviour answers.
    profile: Option<&'static tcl_dialect::DialectProfile>,
    /// Packages a `SpecTcl` pack declared **ambient** in this registry's
    /// dialect, with the version the runtime provides — `(package, version)`,
    /// in installation order.
    ///
    /// Empty for every compiled-in registry. A pack fills it through
    /// [`Self::insert_ambient_package`], which is how a package that comes
    /// *with* a dialect rather than being `package require`d gets a version
    /// floor at all.
    ///
    /// This is the pack-authored twin of [`tcl_dialect::LibraryPin`] with
    /// `ambient: true`. The profile axis is compiled in and describes the
    /// dialects this repository models; this one is authored outside it, which
    /// is the axis that has to exist before `tk` / `tcllib` / `iapps` can move
    /// to packs — a package's own version floor must not depend
    /// on whether this crate happens to know the package's name.
    ambient_packages: Vec<(&'static str, &'static str)>,
    /// The packages this registry's own point carries and the floor the
    /// registry guarantees of each — what [`Self::own_surface_query`] lends
    /// as the query's packages.
    ///
    /// Derived from [`Self::profile`] (which packages) and
    /// [`Self::package_floor`] (which release), so every seam that moves
    /// either refreshes it. Empty for a profile-less registry and for a
    /// profile whose point carries no package.
    own_packages: Vec<PackageFloor<'static>>,
    /// Special variables a `SpecTcl` pack declared with `special_var`, in
    /// installation order — the pack-authored rows [`Self::special_vars`]
    /// reads beside the shipped table.
    ///
    /// Empty for every compiled-in registry; a pack fills it through
    /// [`Self::insert_special_var`].
    special_vars: Vec<&'static crate::special_vars::SpecialVarSpec>,
    /// The member grammar of a **document** in this registry's dialect, when
    /// its command surface declares one.
    ///
    /// An authoring dialect whose file is itself a declaration body — a
    /// `.sslictcl` document, a `.tclspec` pack — has a fixed set of words that
    /// are legal at the root and nothing else. That is the same fact a
    /// [`DefinitionBodyGrammar`] states about a *nested* body, so it is stated
    /// the same way, and the generic consumers (completion, the token walker,
    /// folding) answer the root question with the machinery they already use
    /// for every level below it.
    ///
    /// `None` for an ordinary Tcl dialect, where the root is an open command
    /// position and the whole registry is the answer.
    document_grammar: Option<&'static DefinitionBodyGrammar>,
    /// Lazily-built facts for the one effective spec selected for every
    /// command in this exact registry generation.
    ///
    /// Registry construction and `SpecTcl` overlays are mutable, while all
    /// compiler consumers receive the completed registry read-only. Every
    /// mutation seam invalidates this cell, so a profile change or an authored
    /// override can never reuse facts from the preceding generation. Once the
    /// registry is frozen, command-binding and CFG consumers share the same
    /// `Arc` instead of independently walking the full command universe.
    effective_semantics: OnceLock<Arc<EffectiveRegistrySemantics>>,
    snapshot: OnceLock<RegistrySnapshot>,
    native_registrations: Arc<RwLock<NativeRegistrationCache>>,
    /// The workspace pack overlay this registry was built with
    /// ([`crate::registry_for_profile_with_overlay`]), when it was one.
    overlay: Option<u64>,
    /// The spec pack each installed pack command came from, keyed by the
    /// installed spec's address ([`Self::pack_origin`]). Every spec the
    /// registry indexes is `&'static` and never freed, so an address names
    /// one spec for the life of the process.
    pack_origins: FxHashMap<usize, crate::pack_origin::PackOrigin>,
    /// The text of the definition a `TclBody`-backed pack command's
    /// `PackageSource` pointer resolved to at load, keyed by the installed
    /// spec's address as [`Self::pack_origins`] is ([`Self::reference_body`]).
    reference_texts: FxHashMap<usize, Arc<str>>,
    /// This registry's generation: a number no other registry, and no
    /// earlier state of this one, has had. Every mutation draws a new one,
    /// so a memo keyed by it names exactly the command surface it resolved
    /// against.
    generation: u64,
}

#[derive(Default)]
struct NativeRegistrationCache {
    compilers: FxHashMap<
        (crate::InvocationDialect, String),
        Option<crate::native_compilation::NativeCompilationSpec>,
    >,
    lookups: FxHashMap<
        (crate::InvocationDialect, String),
        Option<crate::native_compilation::NativeCompilerImplementationLookup>,
    >,
}

/// Immutable registry snapshot for incremental consumers. Equality checks the
/// complete structural identity; a hash is only a table index. Specs and body
/// grammars have immutable process-lifetime identities, while registry indexes
/// and package/layer order are copied and owned by the snapshot.
#[derive(Clone, Debug)]
pub struct RegistrySnapshot(Arc<RegistrySnapshotData>);

#[derive(Debug)]
struct RegistrySnapshotData {
    semantic_key: RegistrySemanticKey,
    registry: Arc<CommandRegistry>,
}

/// Immutable structural registry identity without retained reader caches.
/// Equality checks every semantic field; fingerprints only select hash buckets.
#[derive(Clone, Debug)]
pub struct RegistrySemanticKey(Arc<RegistrySemanticKeyData>);

#[derive(Debug)]
struct RegistrySemanticKeyData {
    key: RegistrySnapshotKey,
    fingerprint: u64,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct RegistrySnapshotKey {
    commands: Vec<(&'static str, Vec<usize>)>,
    overlays: Vec<usize>,
    layers: Vec<SurfaceLayer>,
    profile: Option<tcl_dialect::DialectProfileKey>,
    ambient_packages: Vec<(&'static str, &'static str)>,
    own_packages: Vec<PackageFloor<'static>>,
    special_vars: Vec<usize>,
    overlay: Option<u64>,
    pack_origins: Vec<(usize, crate::pack_origin::PackOrigin)>,
    reference_texts: Vec<(usize, Arc<str>)>,
    document_grammar: Option<usize>,
}

impl RegistrySnapshot {
    /// Clone the cache-free identity for keys that do not need a registry reader.
    #[must_use]
    pub fn semantic_key(&self) -> RegistrySemanticKey {
        self.0.semantic_key.clone()
    }

    /// Read the retained registry rather than resolving its display name again.
    #[must_use]
    pub fn registry(&self) -> &CommandRegistry {
        &self.0.registry
    }

    /// Share the exact frozen command store with a retained analysis context.
    /// This preserves authored overrides and package/profile axes without
    /// reconstructing a catalogue or copying another registry generation.
    #[must_use]
    pub fn shared_registry(&self) -> Arc<CommandRegistry> {
        Arc::clone(&self.0.registry)
    }
}

impl PartialEq for RegistrySnapshot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0.semantic_key == other.0.semantic_key
    }
}
impl Eq for RegistrySnapshot {}
impl Hash for RegistrySnapshot {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.semantic_key.hash(state);
    }
}

impl PartialEq for RegistrySemanticKey {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0.key == other.0.key
    }
}
impl Eq for RegistrySemanticKey {}
impl Hash for RegistrySemanticKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.fingerprint.hash(state);
    }
}

/// A registry generation no registry has had.
fn next_registry_generation() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// Descriptor facts from the effective command spec selected by a registry.
///
/// This is deliberately registry-owned: consumers can ask generic trait and
/// lowering questions without constructing their own per-command indexes or
/// reproducing release/dialect selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectiveCommandSemantics {
    traits: Traits,
    lowering_hook: Option<LoweringHookId>,
}

const JIM_COMMANDS: &str = "* + - / after alarm alias append apply array binary binary::nextarg break catch cd class clock close collect concat continue curry defer dict ensemble env eof error errorInfo eval exec exists exit expr fconfigure file fileevent finalize flush for foreach format function getref gets glob glob.explode glob.glob glob.globdir global history if incr info interp join json::decode json::encode json::subencode kill lambda lambda.finalizer lappend lassign lindex linsert list llength lmap load load_ssl_certs local loop lrange lrepeat lreplace lreverse lsearch lset lsort lsubst namespace open os.fork os.gethostname os.getids os.umask os.uptime pack package parray pid pipe popen proc puts pwd rand range read readdir ref regexp regsub rename return scan seek set setref signal sleep socket source split stackdump stacktrace stderr stdin stdout string subst super switch syslog tailcall taint tcl::autocomplete tcl::prefix tcl::stdhint tell throw time timerate tree try unpack unset untaint upcall update uplevel upvar variable vwait wait while xtrace zlib";

fn jim_fresh_command_names() -> &'static BTreeSet<String> {
    static NAMES: OnceLock<BTreeSet<String>> = OnceLock::new();
    NAMES.get_or_init(|| {
        JIM_COMMANDS
            .split_whitespace()
            .map(tcl_syntax::naming::normalise_qualified_name)
            .collect()
    })
}

impl EffectiveCommandSemantics {
    /// The full effective command trait set.
    #[must_use]
    pub const fn traits(self) -> Traits {
        self.traits
    }

    /// Whether the effective command spec carries every flag in `traits`.
    #[must_use]
    pub const fn has_traits(self, traits: Traits) -> bool {
        self.traits.contains(traits)
    }

    /// The effective command's structured lowering hook, if any.
    #[must_use]
    pub const fn lowering_hook(self) -> Option<LoweringHookId> {
        self.lowering_hook
    }
}

/// Cached semantic projection of one complete [`CommandRegistry`] generation.
///
/// The projection is built in one pass over registry spellings. Its compact
/// per-command facts select the same effective row as
/// [`CommandRegistry::get_for_surface`] for the registry's own profile. Its
/// binding domain deliberately preserves the command-binding lattice's
/// historical union semantics (`command_names` plus last-registered unknown
/// handlers): profile registries retain all release rows so compiler binding
/// analysis has always treated every indexed spelling as initially bound.
/// Dynamic pack registries get their own projection; authored insertion and
/// profile changes invalidate it before the next read.
#[derive(Debug, Default)]
pub struct EffectiveRegistrySemantics {
    commands: FxHashMap<&'static str, EffectiveCommandSemantics>,
    binding_names: BTreeSet<String>,
    unresolved_command_handlers: BTreeSet<String>,
    binding_fingerprint: u64,
}

impl EffectiveRegistrySemantics {
    /// Canonical, rooted command names in this registry's indexed binding
    /// domain (the historical `command_names` union across release rows).
    #[must_use]
    pub const fn binding_names(&self) -> &BTreeSet<String> {
        &self.binding_names
    }

    /// Canonical, rooted fallback carriers for unresolved command heads.
    #[must_use]
    pub const fn unresolved_command_handlers(&self) -> &BTreeSet<String> {
        &self.unresolved_command_handlers
    }

    /// Stable fingerprint of the command-binding domain.
    ///
    /// Callers must still compare the two sets for equality; this value exists
    /// so state hashes never walk the full registry universe.
    #[must_use]
    pub const fn binding_fingerprint(&self) -> u64 {
        self.binding_fingerprint
    }

    /// Facts for `name`'s effective command spec, or `None` when the spelling
    /// is unavailable in this registry's exact release/dialect surface.
    #[must_use]
    pub fn command(&self, name: &str) -> Option<EffectiveCommandSemantics> {
        self.commands.get(name).copied()
    }

    /// Effective command names carrying `traits`.
    ///
    /// This iterator is principally useful for drift tests and reports. Hot
    /// consumers should query [`Self::command`] at the call site instead of
    /// rebuilding another complete classification set.
    pub fn command_names_with_traits(
        &self,
        traits: Traits,
    ) -> impl Iterator<Item = &'static str> + '_ {
        self.commands
            .iter()
            .filter_map(move |(&name, facts)| facts.has_traits(traits).then_some(name))
    }
}

/// Command names registered by *every* dialect. Backs
/// [`CommandRegistry::known_in_any_dialect`] — the dialect-agnostic existence
/// check over every loaded dialect. `rootable` is the subset whose bare spec
/// can also denote a rooted singleton command; method-context-only spellings
/// such as `my` are deliberately absent from it.
/// Built from the same spec functions [`CommandRegistry::build_default`]
/// and [`CommandRegistry::load_surface`] draw from, so it stays in lock-step
/// with the registry's command universe, plus the core-surface specs a crate
/// above the registry has registered ([`crate::register_core_surface_specs`]).
#[derive(Clone)]
struct AllDialectCommandNames {
    known: FxHashSet<&'static str>,
    rootable: FxHashSet<&'static str>,
    providers: FxHashMap<&'static str, NameProviders>,
}

/// Who offers a command name across the command universe — see
/// [`CommandRegistry::providers_in_any_dialect`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NameProviders {
    /// Some spec of this name states no surface, so it is offered to every
    /// dialect and no family is unrelated to it.
    pub unrestricted: bool,
    /// The providers the surface rows of every spec of this name name.
    pub providers: Vec<SpecProvider>,
    /// The surface rows of every spec of this name, for naming the dialects
    /// that offer it.
    pub rows: Vec<SpecSurface>,
}

impl NameProviders {
    fn add_spec(&mut self, spec: &CommandSpec) {
        match spec.surface {
            None => self.unrestricted = true,
            Some(rows) => {
                for row in rows {
                    if !self.providers.contains(&row.provider) {
                        self.providers.push(row.provider);
                    }
                    if !self.rows.contains(row) {
                        self.rows.push(*row);
                    }
                }
            }
        }
    }
}

impl AllDialectCommandNames {
    fn add(&mut self, spec: &CommandSpec) {
        // Normalise away a leading `::` so a spec registered only in
        // its fully-qualified spelling (e.g.
        // `::tcl::unsupported::corotype`, which has no separate bare
        // registration) still matches `known_in_any_dialect`'s
        // already-bare query — the caller strips a literal `::` head
        // from the source text before calling in, so the set must be
        // bare-normalised too or the two never agree.
        let name = spec.name.strip_prefix("::").unwrap_or(spec.name);
        self.known.insert(name);
        if name.contains("::") || !spec.traits.contains(Traits::TCLOO_METHOD_CONTEXT) {
            self.rootable.insert(name);
        }
        self.providers.entry(name).or_default().add_spec(spec);
    }
}

/// The names every compiled-in spec set offers.
fn compiled_command_names() -> &'static AllDialectCommandNames {
    static NAMES: OnceLock<AllDialectCommandNames> = OnceLock::new();
    NAMES.get_or_init(|| {
        let mut names = AllDialectCommandNames {
            known: FxHashSet::default(),
            rootable: FxHashSet::default(),
            providers: FxHashMap::default(),
        };
        let mut add = |specs: Vec<CommandSpec>| {
            for spec in &specs {
                names.add(spec);
            }
        };
        add(crate::commands::bpf::bpf_command_specs());
        add(crate::commands::tcl::tcl_command_specs());
        add(crate::commands::stdlib::stdlib_command_specs());
        add(crate::commands::tcllib::tcllib_command_specs());
        add(crate::commands::argparse::argparse_command_specs());
        add(crate::commands::ticklecharts::ticklecharts_command_specs());
        add(crate::commands::itcl::itcl_command_specs());
        add(crate::commands::tk::tk_command_specs());
        add(crate::commands::irules::irules_command_specs());
        add(crate::commands::iapps::iapps_command_specs());
        add(crate::commands::expect::expect_command_specs());
        // The EDA vendor libraries are deliberately NOT added: they ship as
        // bundled `.tclspec` loadables (`docs/design/registry/spec-packs.md`), so this
        // crate does not know their names at compile time and W002 reports an
        // EDA command outside an EDA profile as an ordinary unknown command
        // rather than as "exists, but not here". The pack is what knows.
        // SpecTcl is deliberately NOT added. This set answers "is this name a
        // command in *some* dialect", and W002 turns a `true` into "exists,
        // but not here". SpecTcl's statement words are ordinary English nouns
        // (`arity`, `traits`, `value`, `detail`) that mean nothing outside a
        // pack body, so claiming them here would rewrite an honest
        // unknown-command report on a user's `proc arity` call into a
        // misleading dialect-availability one — the exact opposite of the
        // context-sensitivity the SpecTcl grammars exist to provide.
        names
    })
}

/// The command universe: the compiled-in names plus the registered
/// core-surface specs' (a family's own commands, such as Jim's `loop`), built
/// once per registered generation.
fn all_dialect_command_names() -> &'static AllDialectCommandNames {
    static WITH_CORE_SURFACE: RwLock<Option<(u64, &'static AllDialectCommandNames)>> =
        RwLock::new(None);
    let compiled = compiled_command_names();
    let registered = crate::cache::core_surface_generation();
    if registered == 0 {
        return compiled;
    }
    let held = WITH_CORE_SURFACE
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .filter(|&(built, _)| built == registered)
        .map(|(_, names)| names);
    if let Some(names) = held {
        return names;
    }
    let (generation, specs) = crate::cache::core_surface_specs();
    let mut merged = compiled.clone();
    for spec in specs {
        merged.add(spec);
    }
    // One allocation per registered generation, held for the process.
    let merged: &'static AllDialectCommandNames = Box::leak(Box::new(merged));
    *WITH_CORE_SURFACE
        .write()
        .unwrap_or_else(PoisonError::into_inner) = Some((generation, merged));
    merged
}

/// Whether `spec` may serve as the bare-name fallback for a rooted spelling.
/// Tcl's global built-ins are authored under bare registry keys, so `::set`
/// must fall back to `set`; `TclOO` method-context commands are also authored
/// bare but do not exist as `::my`, `::self`, and so on. A namespaced spelling
/// such as `::oo::Helpers::self` remains an ordinary qualified fallback.
fn rooted_fallback_allowed(rooted: &str, spec: &CommandSpec) -> bool {
    rooted.strip_prefix("::").is_none_or(|unrooted| {
        unrooted.contains("::") || !spec.traits.contains(Traits::TCLOO_METHOD_CONTEXT)
    })
}

/// Append `(index, role)` for the `args` indices consumed by value-taking
/// options whose value role (primary or secondary) `wanted` admits.
///
/// Walks `args` from `scan_start` (1 to skip a subcommand word, else 0),
/// resolving option names, aliases, and unique abbreviations through the
/// declared table — so `-1`, `$x`, `[cmd]`, and ambiguous abbreviations are
/// treated as positionals — and advancing past each recognised option and the
/// value word(s) it consumes ([`OptionSpec::value_indices`], which honours
/// arity and the `--` terminator). The emitted indices are absolute into `args`,
/// exactly like the positional roles, so consumers map them via `argv[idx + 1]`
/// unchanged. A two-way binding (`role: VarWrite, also_role: VarRead`) emits its
/// index under both roles — the multi-role convention.
fn push_option_value_roles(
    out: &mut Vec<(usize, ArgRole)>,
    options: &[&crate::hover::OptionSpec],
    args: &[&str],
    scan_start: usize,
    wanted: &impl Fn(ArgRole) -> bool,
    prefix_matching: PrefixMatching,
) {
    let mut i = scan_start;
    while i < args.len() {
        if args[i] == "--" {
            break;
        }
        if let Some(opt) =
            crate::spec::resolve_available_option_prefix_with(options, args[i], prefix_matching)
        {
            let vals = opt.value_indices(args, i);
            for role in [opt.value_role(), opt.value_also_role()]
                .into_iter()
                .flatten()
                .filter(|&role| wanted(role))
            {
                out.extend(vals.iter().map(|&value| (value, role)));
            }
            i += 1 + vals.len();
        } else {
            i += 1;
        }
    }
}

/// Return the timing of the script-valued option that consumes `index`.
///
/// The scan deliberately mirrors [`push_option_value_roles`]: aliases and
/// unique abbreviations resolve through the same table, and value arity / `--`
/// handling comes from [`OptionSpec::value_indices`].  `None` means either that
/// `index` is not an option value or that the consuming value is not a script.
fn option_script_timing_at(
    options: &[crate::hover::OptionSpec],
    args: &[&str],
    scan_start: usize,
    index: usize,
) -> Option<crate::hover::ScriptTiming> {
    option_script_timing_at_selected(
        &options.iter().collect::<Vec<_>>(),
        args,
        scan_start,
        index,
        crate::abbrev::PrefixMatching::Enabled,
    )
}

fn option_script_timing_at_selected(
    options: &[&crate::hover::OptionSpec],
    args: &[&str],
    scan_start: usize,
    index: usize,
    prefix_matching: crate::abbrev::PrefixMatching,
) -> Option<crate::hover::ScriptTiming> {
    let mut i = scan_start;
    while i < args.len() {
        if args[i] == "--" {
            break;
        }
        if let Some(opt) =
            crate::spec::resolve_available_option_prefix_with(options, args[i], prefix_matching)
        {
            let vals = opt.value_indices(args, i);
            if vals.contains(&index) {
                return opt.value_script_timing();
            }
            i += 1 + vals.len();
        } else {
            i += 1;
        }
    }
    None
}

/// Return the external callback substitutions declared by the option value
/// that consumes `index`. The empty slice is a meaningful answer: the option
/// is a callback, but it carries no user-controlled replacement value.
fn option_callback_taint_inputs_at(
    options: &[crate::hover::OptionSpec],
    args: &[&str],
    scan_start: usize,
    index: usize,
) -> Option<&'static [CallbackTaintInput]> {
    option_callback_taint_inputs_at_selected(
        &options.iter().collect::<Vec<_>>(),
        args,
        scan_start,
        index,
        crate::abbrev::PrefixMatching::Enabled,
    )
}

fn option_callback_taint_inputs_at_selected(
    options: &[&crate::hover::OptionSpec],
    args: &[&str],
    scan_start: usize,
    index: usize,
    prefix_matching: crate::abbrev::PrefixMatching,
) -> Option<&'static [CallbackTaintInput]> {
    let mut i = scan_start;
    while i < args.len() {
        if args[i] == "--" {
            break;
        }
        if let Some(opt) =
            crate::spec::resolve_available_option_prefix_with(options, args[i], prefix_matching)
        {
            let vals = opt.value_indices(args, i);
            if vals.contains(&index) {
                return Some(opt.value_callback_taint_inputs());
            }
            i += 1 + vals.len();
        } else {
            i += 1;
        }
    }
    None
}

/// Collect option values whose role is [`ArgRole::CommandPrefix`], paired with
/// the option's [`AppendedArity`] — the option-side companion to
/// [`push_option_value_roles`], used by [`CommandRegistry::command_prefixes`].
fn push_command_prefix_options(
    out: &mut Vec<(usize, AppendedArity)>,
    options: &[crate::hover::OptionSpec],
    args: &[&str],
    scan_start: usize,
) {
    let mut i = scan_start;
    while i < args.len() {
        if args[i] == "--" {
            break;
        }
        if let Some(opt) = crate::spec::resolve_option_prefix(options, args[i]) {
            let vals = opt.value_indices(args, i);
            if opt.value_role() == Some(ArgRole::CommandPrefix) {
                let arity = opt.value_appended_arity();
                out.extend(vals.iter().map(|&v| (v, arity)));
            }
            i += 1 + vals.len();
        } else {
            i += 1;
        }
    }
}

/// The command-prefix positions of one call to `spec` (post-head
/// coordinates, the selecting word first) and the arity each receives
/// appended: `sub`'s own table when the call selected one — a subcommand, or
/// an instance method — else the command's (a resolver, else the static
/// table), plus every command-prefix option value. The one rule behind
/// [`CommandRegistry::command_prefixes`] and
/// [`crate::ResolvedInvocation::arg_roles`]; the caller selects `sub`.
pub(crate) fn command_prefixes_in(
    spec: &CommandSpec,
    sub: Option<&SubCommand>,
    args: CommandPrefixArguments<'_>,
) -> Vec<(usize, AppendedArity)> {
    if let Some(sub) = sub {
        return sub_command_prefixes(sub, args.slice_from(1))
            .into_iter()
            .map(|(index, arity)| (index + 1, arity))
            .collect();
    }
    let Some(n) = args.words().exact_argv_len() else {
        return Vec::new();
    };
    let mut out: Vec<(usize, AppendedArity)> = Vec::new();
    if let Some(resolver) = spec.command_prefix_resolver {
        out.extend(resolver(args).into_iter().map(|(i, a)| (i as usize, a)));
    } else {
        out.extend(spec.command_prefixes.iter().map(|(i, a)| (*i as usize, *a)));
    }
    if let Some(spellings) = args.spellings() {
        push_command_prefix_options(&mut out, spec.options, spellings, 0);
    }
    out.retain(|&(idx, _)| idx < n);
    out
}

/// The command-prefix positions of a subcommand's (or an instance method's)
/// own words — `args` are the words after its selecting word, and so are the
/// indices returned.
fn sub_command_prefixes(
    sub: &SubCommand,
    args: CommandPrefixArguments<'_>,
) -> Vec<(usize, AppendedArity)> {
    let Some(n) = args.words().exact_argv_len() else {
        return Vec::new();
    };
    let mut out: Vec<(usize, AppendedArity)> = Vec::new();
    if let Some(resolver) = sub.command_prefix_resolver {
        out.extend(resolver(args).into_iter().map(|(i, a)| (i as usize, a)));
    } else {
        out.extend(sub.command_prefixes.iter().map(|(i, a)| (*i as usize, *a)));
    }
    if let Some(spellings) = args.spellings() {
        push_command_prefix_options(&mut out, sub.options, spellings, 0);
    }
    out.retain(|&(idx, _)| idx < n);
    out
}

/// Put a role table in its one order — by position, and the roles one
/// position carries in [`ArgRole::ALL`] order — with each pair once.
pub(crate) fn sort_role_table(roles: &mut Vec<(usize, ArgRole)>) {
    roles.sort_by_key(|&(index, role)| {
        (
            index,
            ArgRole::ALL
                .iter()
                .position(|&known| known == role)
                .unwrap_or(usize::MAX),
        )
    });
    roles.dedup();
}

/// The pattern-bearing arguments of one call to `spec` — the one rule behind
/// [`CommandRegistry::pattern_args_for_dialect`] and
/// [`crate::ResolvedInvocation::pattern_args`]. `options` supplies the
/// option table available at `dialect`, read only by an option-selected
/// layout; `sub` is the caller's selection for `args[0]`, whose pattern
/// language overrides the command's; a static layout's positions are the
/// caller's `ArgRole::Pattern` answer.
pub(crate) fn pattern_args_in(
    spec: &CommandSpec,
    sub: Option<&SubCommand>,
    args: &[&str],
    options: impl FnOnce() -> Vec<&'static crate::hover::OptionSpec>,
    dialect: Option<SurfaceQuery<'_>>,
    static_pattern_indices: impl FnOnce() -> Vec<usize>,
) -> Vec<crate::patterns::PatternArg> {
    if spec.pattern_arg_resolver.is_some() || spec.option_selects_pattern_language() {
        let options = options();
        if let Some(resolve) = spec.pattern_arg_resolver {
            return resolve(
                args,
                crate::patterns::PatternArgResolverContext {
                    options: &options,
                    reserved_trailing_words: spec.reserved_trailing_words,
                },
            );
        }
        let effects =
            spec.option_effects_over(&options, InvocationArguments::literals(args), dialect);
        return crate::patterns::option_selected_pattern_args(
            &effects,
            spec.reserved_trailing_words,
            args.len(),
        );
    }
    let Some(kind) = sub.and_then(|sub| sub.pattern_type).or(spec.pattern_type) else {
        return Vec::new();
    };
    static_pattern_indices()
        .into_iter()
        .filter_map(|index| u8::try_from(index).ok())
        .map(|index| crate::patterns::PatternArg { index, kind })
        .collect()
}

/// Whether source words prove the option layout a resolver-derived role of
/// `spec` (or its selected `sub`) depends on — the one rule behind the
/// registry's source-aware role, pattern and format queries and the
/// resolution's derived queries. The option tables are the caller's,
/// filtered to its release.
pub(crate) fn layout_is_proven_in(
    spec: &CommandSpec,
    sub: Option<&SubCommand>,
    args: InvocationArguments<'_>,
    spec_options: impl FnOnce() -> Vec<&'static crate::hover::OptionSpec>,
    sub_options: impl FnOnce(&SubCommand) -> Vec<&'static crate::hover::OptionSpec>,
) -> bool {
    if !args.has_exact_argv_len() {
        return false;
    }
    // Static role positions and generic option-value roles remain stable
    // once expansion is excluded. Only a resolver can reinterpret a
    // source word as a positional operand, so a `clock format $time
    // -format %Y` time value does not suppress the independently-owned
    // `-format` value role.
    let resolver_depends_on_options = sub.map_or(
        (spec.arg_role_count_resolver.is_none()
            && spec.arg_role_layout_resolver.is_none()
            && spec.arg_role_resolver.is_some())
            || spec.pattern_arg_resolver.is_some()
            || spec.option_selects_pattern_language(),
        |sub| {
            sub.arg_role_count_resolver.is_none()
                && sub.arg_role_layout_resolver.is_none()
                && sub.arg_role_resolver.is_some()
        },
    );
    if !resolver_depends_on_options {
        return true;
    }
    let (options, option_args, prefix_matching, reserved_trailing_words) = match sub {
        Some(sub) => (sub_options(sub), args.slice_from(1), sub.prefix_matching, 0),
        None => (
            spec_options(),
            args,
            spec.prefix_matching,
            spec.reserved_trailing_words,
        ),
    };
    options.is_empty()
        || source_option_layout_is_proven(
            &options,
            option_args,
            prefix_matching,
            reserved_trailing_words,
        )
}

/// The options of `sub` (of `spec`) available at `dialect`, in declaration
/// order — the profile-less option table the layout proof reads for a
/// selected subcommand.
pub(crate) fn sub_options_at(
    spec: &CommandSpec,
    sub: &SubCommand,
    dialect: Option<SurfaceQuery<'_>>,
) -> Vec<&'static crate::hover::OptionSpec> {
    sub.options
        .iter()
        .filter(|option| option.supports_dialect(dialect, sub.surface.or(spec.surface)))
        .collect()
}

/// Whether source words prove a command's leading option layout.
///
/// A literal option consumes its descriptor-declared values before scanning
/// continues. A dynamic word while that scan is live might evaluate to a
/// value-taking option, so every resolver-derived positional role must
/// abstain. A literal invalid or ambiguous dash word is likewise not a
/// positional operand: treating it as one would describe an invocation Tcl
/// rejects. The profile-filtered descriptor table, its abbreviation policy,
/// and the command's reserved trailing operands are inputs, so every pattern
/// and format consumer makes this decision from the same owner data.
fn source_option_layout_is_proven(
    options: &[&crate::hover::OptionSpec],
    args: InvocationArguments<'_>,
    prefix_matching: PrefixMatching,
    reserved_trailing_words: usize,
) -> bool {
    crate::spec::leading_option_word_count_for_arguments(
        options,
        args,
        prefix_matching,
        reserved_trailing_words,
    )
    .is_some()
}

/// The shipped specs of one command group, built once and leaked.
///
/// Every group below is a `fn() -> Vec<CommandSpec>` over `const` data, so it
/// returns byte-identical specs however often it is called — and it was called
/// often: once per `build_default`, which the cache's own docs note runs per
/// CFG build on some paths, and once more per dialect layer per registry
/// generation. Building each group once and handing out `&'static` slices
/// makes a registry an index over shared data instead of an owner of a private
/// copy of it.
fn shared_specs(
    cell: &'static OnceLock<&'static [CommandSpec]>,
    build: fn() -> Vec<CommandSpec>,
) -> &'static [CommandSpec] {
    cell.get_or_init(|| &*Vec::leak(build()))
}

/// Declare a `fn() -> &'static [CommandSpec]` wrapping one group's builder in
/// its own `OnceLock`.
macro_rules! shared_group {
    ($name:ident, $build:expr) => {
        fn $name() -> &'static [CommandSpec] {
            static CELL: OnceLock<&'static [CommandSpec]> = OnceLock::new();
            shared_specs(&CELL, $build)
        }
    };
}

shared_group!(tcl_specs, crate::commands::tcl::tcl_command_specs);

/// Shipped metadata for a selected fixed expression-function registration.
/// This is independent of the command-wrapper surface filtered by a Registry
/// projection and supplies no callable command or function-table receipt.
pub(super) fn fixed_math_function_metadata(bare: &str) -> Option<&'static CommandSpec> {
    let qualified = crate::mathfunc::qualified_name(bare);
    tcl_specs().iter().find(|spec| spec.name == qualified)
}

shared_group!(stdlib_specs, crate::commands::stdlib::stdlib_command_specs);
shared_group!(tcllib_specs, crate::commands::tcllib::tcllib_command_specs);
shared_group!(
    argparse_specs,
    crate::commands::argparse::argparse_command_specs
);
shared_group!(
    ticklecharts_specs,
    crate::commands::ticklecharts::ticklecharts_command_specs
);
shared_group!(itcl_specs, crate::commands::itcl::itcl_command_specs);
shared_group!(tk_specs, crate::commands::tk::tk_command_specs);
shared_group!(bpf_specs, crate::commands::bpf::bpf_command_specs);
shared_group!(irules_specs, crate::commands::irules::irules_command_specs);
shared_group!(iapps_specs, crate::commands::iapps::iapps_command_specs);
shared_group!(tmsh_specs, crate::commands::iapps::tmsh_command_specs);
shared_group!(expect_specs, crate::commands::expect::expect_command_specs);
shared_group!(
    spectcl_specs,
    crate::commands::spectcl::spectcl_command_specs
);
shared_group!(
    sslictcl_specs,
    crate::commands::sslictcl::sslictcl_command_specs
);

/// Which authoring pack declares each shipped command, built once from the
/// same leaked `shared_group!` slices the registry inserts.
///
/// Keyed **by spec identity**, not by name. Seventeen names are declared in
/// two or three packs — `close` in `tcl`, `expect` and `irules`; `send` in
/// `tk`, `expect` and `irules` — and each dialect registers exactly one of
/// them. Asking by name could only answer with a fixed winner and would file
/// the iRules `close` under `tcl` in the iRules dialect; asking with the very
/// `&'static CommandSpec` the registry handed back cannot be wrong.
fn spec_pack_by_identity() -> &'static FxHashMap<usize, &'static str> {
    static INDEX: OnceLock<FxHashMap<usize, &'static str>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut index = FxHashMap::default();
        for (pack, specs) in authored_groups() {
            for spec in specs {
                index.insert(std::ptr::from_ref(spec) as usize, pack);
            }
        }
        index
    })
}

/// Which packs declare each name, in [`SPEC_PACKS`](crate::commands::SPEC_PACKS)
/// order. Backs the "also declared in" answer, where the question really is
/// about the name rather than about one resolved spec.
fn spec_packs_by_name() -> &'static FxHashMap<&'static str, Vec<&'static str>> {
    static INDEX: OnceLock<FxHashMap<&'static str, Vec<&'static str>>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut index: FxHashMap<&'static str, Vec<&'static str>> = FxHashMap::default();
        for (pack, specs) in authored_groups() {
            for spec in specs {
                // The same bare-normalisation `all_dialect_command_names`
                // applies, for the same reason: a caller strips a literal
                // `::` head before asking.
                let name = spec.name.strip_prefix("::").unwrap_or(spec.name);
                let packs = index.entry(name).or_default();
                if !packs.contains(&pack) {
                    packs.push(pack);
                }
            }
        }
        index
    })
}

/// Every authored group paired with the pack directory it lives in.
///
/// `tmsh_specs` is a filtered view of the same `commands/iapps/` sources, so
/// it reports `iapps` — the directory a spec author would open.
fn authored_groups() -> [(&'static str, &'static [CommandSpec]); 14] {
    [
        ("tcl", tcl_specs()),
        ("stdlib", stdlib_specs()),
        ("tcllib", tcllib_specs()),
        ("argparse", argparse_specs()),
        ("ticklecharts", ticklecharts_specs()),
        ("itcl", itcl_specs()),
        ("tk", tk_specs()),
        ("expect", expect_specs()),
        ("bpf", bpf_specs()),
        ("irules", irules_specs()),
        ("iapps", iapps_specs()),
        ("iapps", tmsh_specs()),
        ("spectcl", spectcl_specs()),
        ("sslictcl", sslictcl_specs()),
    ]
}

/// The `commands/<pack>/` module that declares `spec`.
///
/// `None` for a spec that reached the registry from a `.tclspec` pack — the
/// bundled EDA libraries, a workspace pack, the document under a spec
/// author's cursor. Provenance for those is the pack file, which the loader
/// reports; this deliberately does not guess one.
#[must_use]
pub fn spec_pack_of(spec: &'static CommandSpec) -> Option<&'static str> {
    spec_pack_by_identity()
        .get(&(std::ptr::from_ref(spec) as usize))
        .copied()
}

/// Every authoring pack that declares `name`, in browser order.
///
/// Nearly always one, and empty for a pack-loaded name. More than one means
/// a vendor or DSL surface re-declares the name with its own facts, and a
/// browser can say so instead of silently filing it under one of them.
#[must_use]
pub fn spec_packs_of(name: &str) -> &'static [&'static str] {
    spec_packs_by_name()
        .get(name.strip_prefix("::").unwrap_or(name))
        .map_or(&[][..], |packs| packs.as_slice())
}

/// The one numeral grammar `family` has at `release`, or across its whole
/// ladder when no release is pinned. `None` when the release is unknown or
/// the ladder's releases disagree.
fn point_number_syntax(
    family: Family,
    release: Option<&str>,
) -> Option<tcl_syntax::number::NumberSyntax> {
    let grammar_for = |release| tcl_dialect::model::grammar(family, release).numbers;
    if let Some(spelling) = release {
        family
            .releases()
            .iter()
            .find(|release| release.as_str() == spelling)
            .map(|&release| grammar_for(release))
            .or_else(|| {
                if family == Family::Tcl {
                    tcl_dialect::TclVersion::from_package_version(spelling)
                        .map(tcl_dialect::TclVersion::number_syntax)
                } else {
                    None
                }
            })
    } else {
        let mut releases = family.releases().iter().copied();
        let first = releases.next().map(grammar_for);
        first.filter(|&first| releases.all(|release| grammar_for(release) == first))
    }
}

impl CommandRegistry {
    /// Retain a frozen, shareable registry for caches that must preserve
    /// authored overrides, package pins, and dialect policies. Repeated calls
    /// share one snapshot; mutation creates a new snapshot while previous
    /// readers keep their original registry.
    #[must_use]
    pub fn snapshot(&self) -> RegistrySnapshot {
        self.snapshot
            .get_or_init(|| {
                // Exhaustive destructuring forces new registry axes to make an
                // explicit snapshot and identity decision at this shared seam.
                let Self {
                    by_name,
                    overlay_specs,
                    loaded_layers,
                    profile,
                    ambient_packages,
                    own_packages,
                    special_vars,
                    overlay,
                    pack_origins,
                    reference_texts,
                    generation,
                    document_grammar,
                    effective_semantics: _,
                    snapshot: _,
                    native_registrations: _,
                } = self;
                let mut commands: Vec<_> = by_name
                    .iter()
                    .map(|(&name, specs)| {
                        (
                            name,
                            specs
                                .iter()
                                .map(|&spec| std::ptr::from_ref(spec) as usize)
                                .collect(),
                        )
                    })
                    .collect();
                commands.sort_by_key(|(name, _)| *name);
                let mut pack_origins_key = pack_origins
                    .iter()
                    .map(|(&spec, origin)| (spec, origin.clone()))
                    .collect::<Vec<_>>();
                pack_origins_key.sort_by_key(|(spec, _)| *spec);
                let mut reference_texts_key = reference_texts
                    .iter()
                    .map(|(&spec, text)| (spec, Arc::clone(text)))
                    .collect::<Vec<_>>();
                reference_texts_key.sort_by_key(|(spec, _)| *spec);
                let key = RegistrySnapshotKey {
                    commands,
                    overlays: overlay_specs
                        .iter()
                        .map(|&spec| std::ptr::from_ref(spec) as usize)
                        .collect(),
                    layers: loaded_layers.clone(),
                    profile: profile.map(tcl_dialect::DialectProfile::cache_key),
                    ambient_packages: ambient_packages.clone(),
                    own_packages: own_packages.clone(),
                    special_vars: special_vars
                        .iter()
                        .map(|spec| std::ptr::from_ref(*spec).addr())
                        .collect(),
                    overlay: *overlay,
                    pack_origins: pack_origins_key,
                    reference_texts: reference_texts_key,
                    document_grammar: document_grammar
                        .map(|grammar| std::ptr::from_ref(grammar) as usize),
                };
                let registry = Self {
                    by_name: by_name.clone(),
                    overlay_specs: overlay_specs.clone(),
                    loaded_layers: loaded_layers.clone(),
                    profile: *profile,
                    ambient_packages: ambient_packages.clone(),
                    own_packages: own_packages.clone(),
                    special_vars: special_vars.clone(),
                    overlay: *overlay,
                    pack_origins: pack_origins.clone(),
                    reference_texts: reference_texts.clone(),
                    generation: *generation,
                    document_grammar: *document_grammar,
                    effective_semantics: OnceLock::from(self.effective_semantics()),
                    snapshot: OnceLock::new(),
                    native_registrations: Arc::clone(&self.native_registrations),
                };
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                key.hash(&mut hasher);
                RegistrySnapshot(Arc::new(RegistrySnapshotData {
                    semantic_key: RegistrySemanticKey(Arc::new(RegistrySemanticKeyData {
                        key,
                        fingerprint: hasher.finish(),
                    })),
                    registry: Arc::new(registry),
                }))
            })
            .clone()
    }

    /// Build the default registry with core Tcl + stdlib + tcllib commands.
    #[must_use]
    pub fn build_default() -> Self {
        let mut registry = Self {
            by_name: FxHashMap::default(),
            overlay_specs: Vec::new(),
            loaded_layers: Vec::new(),
            profile: None,
            ambient_packages: Vec::new(),
            own_packages: Vec::new(),
            special_vars: Vec::new(),
            document_grammar: None,
            effective_semantics: OnceLock::new(),
            snapshot: OnceLock::new(),
            native_registrations: Arc::default(),
            overlay: None,
            pack_origins: FxHashMap::default(),
            reference_texts: FxHashMap::default(),
            generation: next_registry_generation(),
        };
        for spec in tcl_specs() {
            registry.insert_shipped_static(spec);
        }
        for spec in stdlib_specs() {
            registry.insert_shipped_static(spec);
        }
        for spec in tcllib_specs() {
            registry.insert_shipped_static(spec);
        }
        for spec in argparse_specs() {
            registry.insert_shipped_static(spec);
        }
        for spec in ticklecharts_specs() {
            registry.insert_shipped_static(spec);
        }
        for spec in itcl_specs() {
            registry.insert_shipped_static(spec);
        }
        // Tk geometry/widget commands (`grid` / `pack` / `wm` / `button` / …)
        // are part of the always-known command universe: a `.tcl` script may
        // `package require Tk` at runtime, and the diagnostics treat them as
        // recognised under every Tcl dialect, so Tk is folded into the base
        // registry.  Mark the layer loaded so a later `load_surface(Tk)` is
        // a no-op rather than a double-insert.
        for spec in tk_specs() {
            registry.insert_shipped_static(spec);
        }
        registry.loaded_layers.push(SurfaceLayer::Package("Tk"));
        registry
    }

    /// Load a surface's commands into the registry (idempotent).
    pub fn load_surface(&mut self, layer: SurfaceLayer) {
        if self.loaded_layers.contains(&layer) {
            return;
        }
        let specs: &'static [CommandSpec] = match layer {
            SurfaceLayer::Package("bpf") => bpf_specs(),
            SurfaceLayer::Core(Family::F5Irules, _) => irules_specs(),
            SurfaceLayer::Package("iapps") => iapps_specs(),
            // The tmsh shell's own pack: the `tmsh::` surface shared with
            // iApps, without the iApp-only commands.
            SurfaceLayer::Package("tmsh") => tmsh_specs(),
            SurfaceLayer::Package("Tk") => tk_specs(),
            SurfaceLayer::Package("expect") => expect_specs(),
            // SpecTcl: the `.tclspec` DSL's own statement words. A pack file
            // is an ordinary Tcl script, so the base Tcl surface stays loaded
            // underneath (hook bodies are real Tcl); this layer adds the
            // declaration vocabulary on top of it.
            SurfaceLayer::Package("spectcl") => {
                self.document_grammar = Some(&crate::definer::SPECTCL_DOCUMENT_GRAMMAR);
                spectcl_specs()
            }
            // SslicTcl: the `.sslictcl` DSL's own declaration words. A
            // document is an ordinary Tcl script that is read and never
            // evaluated, so the base Tcl surface stays loaded underneath —
            // the grammar is what says a word is not an SslicTcl declaration.
            SurfaceLayer::Package("sslictcl") => {
                self.document_grammar = Some(&crate::definer::SSLICTCL_DOCUMENT_GRAMMAR);
                sslictcl_specs()
            }
            // A core release brings no pack of its own — it records which
            // language the registry is. The EDA shells are such a release
            // plus `required_package`-gated command libraries, which ship as
            // bundled `.tclspec` loadables (`docs/design/registry/spec-packs.md`).
            _ => &[],
        };
        for spec in specs {
            self.insert_shipped_static(spec);
        }
        self.loaded_layers.push(layer);
        self.invalidate_effective_semantics();
    }

    /// The member grammar of a document in this registry's dialect, when its
    /// command surface declares one — see the field's own documentation.
    ///
    /// A consumer asking "what may appear at the root of this file?" reads
    /// this; `None` means the root is an ordinary open command position.
    #[must_use]
    pub const fn document_grammar(&self) -> Option<&'static DefinitionBodyGrammar> {
        self.document_grammar
    }

    /// Load iRules dialect commands (convenience wrapper).
    pub fn load_irules(&mut self) {
        self.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    }

    /// Load BPF-Tcl dialect commands (convenience wrapper).
    pub fn load_bpf(&mut self) {
        self.load_surface(SurfaceLayer::Package("bpf"));
    }

    /// Whether this registry's dialect reads a bare leading-zero integer
    /// (`08`, `010`) as **octal**.
    ///
    /// Tcl 9.0 dropped the leading-zero octal rule (TIP 114): `08` parses as
    /// decimal 8 and `010` as decimal 10. Every earlier Tcl (8.4/8.5/8.6) and
    /// every 8.x-derived dialect (f5-irules ≈ 8.4, f5-iapps ≈ 8.5/8.6, the EDA
    /// dialects) keeps the octal rule, where `08`/`09` are *invalid* octal
    /// (treated as a string in `==`/`!=`) and `010` is 8.
    ///
    /// TIP 114 lands in tcl9.0 and stays in tcl9.1 (and any later 9.x), so the
    /// decimal rule applies to *every* Tcl 9 dialect, not tcl9.0 alone. The
    /// per-dialect registry built by `registry_for_dialect` records its Tcl
    /// version via [`Self::load_surface`], so a registry whose
    /// `loaded_layers` names a 9.x core release (tcl9.0, tcl9.1, …) is
    /// decimal; every other dialect (8.4/8.5/8.6 and the F5/EDA registries,
    /// which never load a Tcl-9 release row) reads leading zeros as octal.
    #[must_use]
    pub fn leading_zero_is_octal(&self) -> bool {
        self.octal_fold_policy().unwrap_or(true)
    }

    /// The release this registry's dialect runs, if it names one.
    ///
    /// The registry-level mirror of [`DialectProfile::runtime_version`], so a
    /// consumer holding a registry never spells out `profile().and_then(…)`.
    #[must_use]
    pub fn runtime_version(&self) -> Option<tcl_dialect::TclVersion> {
        self.profile()
            .and_then(tcl_dialect::DialectProfile::runtime_version)
    }

    /// The string/character model of the release this registry's dialect runs.
    #[must_use]
    pub fn character_model(&self) -> Option<tcl_dialect::StringCharacterModel> {
        self.profile()
            .and_then(tcl_dialect::DialectProfile::character_model)
    }

    /// The numeral grammar of the dialect this registry serves, or the
    /// permissive 9.x default when no profile is loaded.
    ///
    /// The single way a registry-holding consumer names its release for number
    /// parsing — the hand-written `profile().map_or(Tcl90, …)` had six copies.
    #[must_use]
    pub fn numbers(&self) -> tcl_dialect::NumberSyntax {
        tcl_dialect::NumberSyntax::of_profile(self.profile())
    }

    /// The three-valued leading-zero fold policy for this registry's
    /// dialect. A profile-built registry (`registry_for_profile` /
    /// `registry_for_dialect`) answers from the profile's runtime base:
    /// `Some(true)` = 8.x octal, `Some(false)` = 9.x decimal (`bpf`
    /// included, D7), `None` = abstain — no Tcl runtime to have an opinion
    /// (`f5-bigip`, the unknown-dialect fallback; §11.1 of the
    /// dialect-profile model). A hand-assembled registry keeps the
    /// historical `loaded_layers` derivation (a version pack records its
    /// release, so a 9.x load reads decimal).
    #[must_use]
    pub fn octal_fold_policy(&self) -> Option<bool> {
        match self.profile {
            Some(p) => p.leading_zero_is_octal.as_bool(),
            None => Some(!self.loaded_core_is_tcl9()),
        }
    }

    /// Stamp the dialect profile this registry serves. Called by the
    /// per-profile cache (`registry_for_profile`) so behaviour queries
    /// (`octal_policy`, future runtime projections) answer from the
    /// profile rather than re-deriving from loaded packs.
    pub(crate) fn set_profile(&mut self, profile: &'static tcl_dialect::DialectProfile) {
        self.profile = Some(profile);
        self.refresh_own_packages();
        self.invalidate_effective_semantics();
    }

    /// Re-derive [`Self::own_packages`] from the profile and the floors this
    /// registry now guarantees.
    fn refresh_own_packages(&mut self) {
        let carried = self
            .profile
            .map_or(&[][..], |profile| profile.surface_packages);
        self.own_packages = carried
            .iter()
            .map(|package| PackageFloor {
                name: package.name,
                version: self.package_floor(package.name),
            })
            .collect();
    }

    /// Derive an exact registry view for `profile`, preserving this registry's
    /// authored overrides.
    ///
    /// The ordinary registry deliberately retains every release row so tools
    /// can ask cross-dialect questions. Compiler passes also have a few
    /// target-neutral lookups, however, so an explicit-profile compile needs a
    /// physically exact view: every spelling indexes only the one spec selected
    /// by the profile's surface query. Authored specs are selected separately
    /// from the compiled-in universe and replace a visible shipped row even
    /// when the authored override is deliberately surface-less. This preserves
    /// the public insertion contract used by custom `SpecTcl` commands without
    /// weakening release or dialect filtering for shipped commands.
    #[must_use]
    pub fn project_for_profile(&self, profile: &'static tcl_dialect::DialectProfile) -> Self {
        let mut projected = Self::build_default();
        for &layer in profile.base_layers {
            projected.load_surface(layer);
        }
        for &(package, version) in &self.ambient_packages {
            projected.insert_ambient_package(package, version);
        }
        for &layer in &self.loaded_layers {
            if !projected.loaded_layers.contains(&layer) {
                projected.loaded_layers.push(layer);
            }
        }
        // Registered native-family rows are retained implementations too;
        // rebuilding the shipped catalogue must not discard them when the
        // driver selects an exact point. Availability still selects the row.
        for (&name, specs) in &self.by_name {
            let rows = projected.by_name.entry(name).or_default();
            for &spec in specs {
                if !rows.iter().any(|held| std::ptr::eq(*held, spec)) {
                    rows.push(spec);
                }
            }
        }
        projected.set_profile(profile);

        let own_packages = projected.own_packages.clone();
        let query = profile.surface_query().with_packages(&own_packages);
        projected.by_name = projected
            .by_name
            .iter()
            .filter_map(|(&name, specs)| {
                projected
                    .best_visible(specs, Some(query))
                    .map(|spec| (name, vec![spec]))
            })
            .collect();
        let mut overlays_by_name: FxHashMap<_, Vec<_>> = FxHashMap::default();
        for &spec in &self.overlay_specs {
            overlays_by_name.entry(spec.name).or_default().push(spec);
        }
        for (name, specs) in overlays_by_name {
            if let Some(spec) = projected.best_visible(&specs, Some(query)) {
                projected.by_name.insert(name, vec![spec]);
            }
        }
        projected.overlay_specs.clone_from(&self.overlay_specs);
        // The same authored world, projected: its overlay generation and the
        // pack each authored spec came from travel with the specs themselves.
        projected.overlay = self.overlay;
        projected.pack_origins.clone_from(&self.pack_origins);
        projected.reference_texts.clone_from(&self.reference_texts);
        projected.invalidate_effective_semantics();
        projected
    }

    /// The dialect profile this registry was built for, if any.
    #[must_use]
    pub fn profile(&self) -> Option<&'static tcl_dialect::DialectProfile> {
        self.profile
    }

    /// The point a consumer resolves hooks and calls at when honouring
    /// **this registry's own** dialect: the attached profile's, or `None`
    /// — surface-blind — for a profile-less registry.
    ///
    /// This is what lets a version-pinned compile pipeline suppress a
    /// structured lowering or codegen hook for a
    /// command the emulated release does not have — `lmap` under a tcl8.4
    /// registry resolves to no spec, so the call reaches the runtime's
    /// availability gate as a generic dispatch instead of being inlined.
    #[must_use]
    pub fn own_surface_query(&self) -> Option<SurfaceQuery<'_>> {
        self.profile
            .map(|profile| profile.surface_query().with_packages(&self.own_packages))
    }

    /// Whether a loaded core layer is a Tcl 9.x release — the derivation a
    /// profile-less registry falls back to for the octal question.
    fn loaded_core_is_tcl9(&self) -> bool {
        self.loaded_layers.iter().any(|layer| {
            matches!(layer, SurfaceLayer::Core(Family::Tcl, release)
                if version_satisfies(release, "9.0-"))
        })
    }

    /// Insert an owned command spec, **leaking it** for the process lifetime.
    ///
    /// The registry indexes `&'static CommandSpec`, so an owned spec has to be
    /// given somewhere permanent to live. That is the right trade for the two
    /// callers that need it — a test building a registry by hand, and a
    /// `.tclspec` pack whose specs are already leaked by the loader — and the
    /// wrong one for the hundreds of shipped specs, which are indexed through
    /// the registry's private compiled-universe seam.
    ///
    /// A caller in a loop over user input wants `insert_static` and an arena
    /// it controls, not this.
    pub fn insert(&mut self, spec: CommandSpec) {
        self.insert_static(Box::leak(Box::new(spec)));
    }

    /// A copy of this registry with `specs` — a family's own compiled-in
    /// surface — indexed after every shipped spec under their names, and
    /// `overlaid`'s authored overlay indexed after them.
    ///
    /// `self` is the store before any pack overlay and `overlaid` is that
    /// store with the overlay installed (the same store when there is none).
    /// The compiled-in specs are not a pack's contribution, so a pack's row
    /// for the same name outranks them where registration order breaks a
    /// tie, as it outranks the shipped data it shadows. The copy shares every
    /// `&'static CommandSpec` with its sources, records only the overlay's
    /// specs as authored, and takes every other field from `overlaid`.
    /// Availability-aware queries rank by the query's core points before
    /// registration order is consulted.
    #[must_use]
    pub(crate) fn with_core_surface(
        &self,
        specs: &[&'static CommandSpec],
        overlaid: &Self,
    ) -> Self {
        let mut by_name = self.by_name.clone();
        for spec in specs.iter().chain(&overlaid.overlay_specs) {
            by_name.entry(spec.name).or_default().push(spec);
        }
        Self {
            by_name,
            overlay_specs: overlaid.overlay_specs.clone(),
            loaded_layers: overlaid.loaded_layers.clone(),
            profile: overlaid.profile,
            ambient_packages: overlaid.ambient_packages.clone(),
            own_packages: overlaid.own_packages.clone(),
            special_vars: overlaid.special_vars.clone(),
            document_grammar: overlaid.document_grammar,
            effective_semantics: OnceLock::new(),
            snapshot: OnceLock::new(),
            native_registrations: Arc::default(),
            overlay: overlaid.overlay,
            pack_origins: overlaid.pack_origins.clone(),
            reference_texts: overlaid.reference_texts.clone(),
            // A surface neither source has had.
            generation: next_registry_generation(),
        }
    }

    /// Insert an authored overlay spec the caller already owns permanently —
    /// no copy, no leak.
    ///
    /// This is what makes a per-pack-edit registry affordable. A registry is
    /// rebuilt from scratch for every distinct pack-set content the server
    /// sees, and a `CommandSpec` is 1,296 bytes: copying the ~2,400 shipped
    /// specs into each generation cost megabytes a keystroke. Sharing one
    /// leaked copy of every shipped spec reduces a generation to its index —
    /// the names, and one pointer each.
    pub fn insert_static(&mut self, spec: &'static CommandSpec) {
        self.by_name.entry(spec.name).or_default().push(spec);
        self.overlay_specs.push(spec);
        self.invalidate_effective_semantics();
    }

    /// Record that the installed `spec` came from a spec pack — the facts a
    /// site specialised on it stamps ([`Self::pack_origin`]). The installer
    /// calls this for each pack command it inserts.
    pub fn insert_pack_origin(
        &mut self,
        spec: &'static CommandSpec,
        origin: crate::pack_origin::PackOrigin,
    ) {
        self.pack_origins
            .insert(std::ptr::from_ref(spec).addr(), origin);
        self.invalidate_effective_semantics();
    }

    /// The spec pack `spec` was installed from, or `None` for a shipped spec
    /// and for one an embedder inserted itself.
    #[must_use]
    pub fn pack_origin(&self, spec: &CommandSpec) -> Option<&crate::pack_origin::PackOrigin> {
        self.pack_origins.get(&std::ptr::from_ref(spec).addr())
    }

    /// Record the definition text a pack command's `PackageSource` backing
    /// resolved to — what the loader read from the package at load, so that
    /// nothing downstream reads a file ([`Self::reference_body`]).
    pub fn insert_reference_text(&mut self, spec: &'static CommandSpec, text: Arc<str>) {
        self.reference_texts
            .insert(std::ptr::from_ref(spec).addr(), text);
        self.invalidate_effective_semantics();
    }

    /// The Tcl definition `spec`'s backing says defines the command: the text
    /// a `PackText` backing carries, or the file a `PackageSource` backing
    /// resolved to at load. `None` for every other backing, and for a
    /// `PackageSource` the loader had no package to read.
    #[must_use]
    pub fn reference_body(&self, spec: &CommandSpec) -> Option<&str> {
        use crate::runtime_backing::{BodySource, RuntimeBacking};
        match spec.runtime_backing {
            RuntimeBacking::TclBody {
                source: BodySource::PackText { text },
                ..
            } => Some(text),
            RuntimeBacking::TclBody {
                source: BodySource::PackageSource { .. },
                ..
            } => self
                .reference_texts
                .get(&std::ptr::from_ref(spec).addr())
                .map(AsRef::as_ref),
            _ => None,
        }
    }

    /// Every spec a pack installed whose backing is a Tcl body this registry
    /// holds the text of, with the text — the commands a compile may inline
    /// the definition of. A spec a later insertion shadowed is not among them.
    pub fn reference_bodies(&self) -> impl Iterator<Item = (&'static CommandSpec, &str)> {
        self.overlay_specs.iter().filter_map(|&spec| {
            let text = self.reference_body(spec)?;
            self.pack_origin(spec)?;
            self.get_exact(spec.name)
                .is_some_and(|live| std::ptr::eq(live, spec))
                .then_some((spec, text))
        })
    }

    /// Index one row from the compiled-in command universe without recording
    /// it as an authored overlay.
    fn insert_shipped_static(&mut self, spec: &'static CommandSpec) {
        self.by_name.entry(spec.name).or_default().push(spec);
        self.invalidate_effective_semantics();
    }

    /// Record that `package` is ambient in this registry's dialect at
    /// `version` — the `ambient_package` statement of a `SpecTcl` pack.
    pub fn insert_ambient_package(&mut self, package: &'static str, version: &'static str) {
        self.ambient_packages.push((package, version));
        self.refresh_own_packages();
        self.invalidate_effective_semantics();
    }

    /// Whether `package` is **ambient** — provided by the runtime, with no
    /// `package require` needed to reach it.
    ///
    /// The union of the two things that can make that true: the dialect
    /// profile shipping it (an F5 surface is part of the runtime, §7.1 axis
    /// C), and a loaded `SpecTcl` pack declaring it with `ambient_package`.
    /// This is the one place that union is taken — every consumer that used
    /// to ask the profile alone must ask here instead, or a pack's ambient
    /// declaration registers a floor while the same package still draws a
    /// "needs a `package require`" diagnostic.
    #[must_use]
    pub fn is_ambient_package(&self, package: &str) -> bool {
        self.profile
            .is_some_and(|profile| profile.is_ambient_package(package))
            || self
                .ambient_packages
                .iter()
                .any(|(name, _)| *name == package)
    }

    /// The version floor a pack declared for `package` as ambient, if any.
    ///
    /// The **highest** declared version wins when several packs name the same
    /// package: each is a claim that the runtime provides at least that
    /// version, and the strongest such claim is the one that holds. This is
    /// the same max-composition an explicit `package require` goes through,
    /// and for the same reason — a floor is a lower bound, so combining two
    /// lower bounds takes the greater.
    #[must_use]
    pub fn ambient_package_floor(&self, package: &str) -> Option<&'static str> {
        self.ambient_packages
            .iter()
            .filter(|(name, _)| *name == package)
            .map(|(_, version)| *version)
            .max_by(|a, b| crate::version::compare(a, b))
    }

    /// Every `(package, declared floor)` row loaded packs registered with
    /// `ambient_package`, in registration order. The enumeration face of
    /// [`Self::is_ambient_package`] / [`Self::ambient_package_floor`], for
    /// the model layer's context assembly
    /// ([`crate::model::assembly::ContextRegistry`]) — the resolved context
    /// carries these rows so pack-ambient floors answer from the context
    /// rather than from a live registry borrow.
    #[must_use]
    pub fn ambient_package_rows(&self) -> &[(&'static str, &'static str)] {
        &self.ambient_packages
    }

    /// Record a special variable a `SpecTcl` pack declares — the pack's
    /// `special_var` statement.
    pub fn insert_special_var(&mut self, spec: &'static crate::special_vars::SpecialVarSpec) {
        self.special_vars.push(spec);
        self.invalidate_effective_semantics();
    }

    /// Every special variable this registry knows: the rows loaded packs
    /// declared, the latest first, then the shipped table
    /// ([`crate::special_vars::SPECIAL_VARS`]) less any name a pack row
    /// already answers for — the one door a consumer reads, so a variable a
    /// pack declares answers exactly as a shipped one does. A pack row
    /// shadows a shipped row of the same name, as an authored command spec
    /// shadows a shipped one.
    pub fn special_vars(
        &self,
    ) -> impl Iterator<Item = &'static crate::special_vars::SpecialVarSpec> + '_ {
        let declared = || self.special_vars.iter().rev().copied();
        declared().chain(
            crate::special_vars::SPECIAL_VARS
                .iter()
                .filter(move |shipped| !declared().any(|pack| pack.name == shipped.name)),
        )
    }

    /// The special variable named `name`, ignoring dialect — the registry
    /// face of [`crate::special_vars::special_var`], pack rows included.
    #[must_use]
    pub fn special_var(&self, name: &str) -> Option<&'static crate::special_vars::SpecialVarSpec> {
        self.special_vars().find(|spec| spec.name == name)
    }

    /// The special variable named `name` when `dialect` provides it — the
    /// registry face of [`crate::special_vars::special_var_in_dialect`].
    #[must_use]
    pub fn special_var_in_dialect(
        &self,
        name: &str,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&'static crate::special_vars::SpecialVarSpec> {
        self.special_var(name)
            .filter(|spec| spec.available_in(dialect))
    }

    /// The special variables `dialect` provides, pack rows first — the
    /// registry face of [`crate::special_vars::special_vars_for_dialect`].
    pub fn special_vars_for_dialect<'a>(
        &'a self,
        dialect: Option<SurfaceQuery<'a>>,
    ) -> impl Iterator<Item = &'static crate::special_vars::SpecialVarSpec> + 'a {
        self.special_vars()
            .filter(move |spec| spec.available_in(dialect))
    }

    /// Whether a bare global `name` is readable before user code in
    /// `dialect` — the registry face of
    /// [`crate::special_vars::is_readable_at_startup`], so a pack-declared
    /// startup binding answers too.
    #[must_use]
    pub fn is_readable_at_startup(&self, name: &str, dialect: Option<SurfaceQuery<'_>>) -> bool {
        self.special_var(name)
            .is_some_and(|spec| spec.readable_at_startup_in(dialect))
    }

    /// Whether `name` is eagerly bound before user code in `dialect` — the
    /// registry face of [`crate::special_vars::is_initially_bound`], so a
    /// pack-declared startup binding answers too.
    #[must_use]
    pub fn is_initially_bound(&self, name: &str, dialect: Option<SurfaceQuery<'_>>) -> bool {
        self.special_var(name)
            .is_some_and(|spec| surface_admits(spec.initially_bound, dialect.as_ref()))
    }

    /// Whether a read of `name` in `dialect` runs a declared read trace that
    /// materialises its value again after `unset` — the registry face of
    /// [`crate::special_vars::is_lazily_readable`], pack rows included.
    #[must_use]
    pub fn is_lazily_readable(&self, name: &str, dialect: Option<SurfaceQuery<'_>>) -> bool {
        self.special_var(name)
            .is_some_and(|spec| surface_admits(spec.lazily_readable, dialect.as_ref()))
    }

    /// Whether the runtime observes a write to `name` in `dialect` — the
    /// registry face of [`crate::special_vars::is_externally_read`], pack
    /// rows included.
    #[must_use]
    pub fn is_externally_read(&self, name: &str, dialect: Option<SurfaceQuery<'_>>) -> bool {
        self.special_var_in_dialect(name, dialect)
            .is_some_and(|spec| spec.externally_read)
    }

    /// Whether `name` exists as a command in *any* dialect, independent of
    /// which dialects this registry instance loaded.
    ///
    /// Looks up the global by-name index across every dialect's specs.
    /// Like [`Self::taint_source`], this is deliberately dialect-agnostic:
    /// an iRules command such as `when` is "known" even when analysing a
    /// `tcl8.6` document whose registry never loaded the iRules specs — the
    /// W002 disabled-in-dialect check needs to distinguish "exists, but not
    /// in this dialect" (→ DISALLOWED) from "exists nowhere" (→ W123's
    /// concern). A leading `::` follows [`Self::get`]'s bare-name fallback,
    /// including its exclusion for method-context-only singleton commands.
    #[must_use]
    pub fn known_in_any_dialect(&self, name: &str) -> bool {
        let names = all_dialect_command_names();
        match name.strip_prefix("::") {
            Some(unrooted) => names.rootable.contains(unrooted),
            None => names.known.contains(name),
        }
    }

    /// Canonical authored names across the compiled universe and actual store.
    /// Availability, source identity and runtime binding remain separate queries.
    pub fn command_names_in_any_dialect(&self) -> impl Iterator<Item = &'static str> {
        // naming.diagnostic.original-command-source-unavailability
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-command-source-unavailability.md
        all_dialect_command_names()
            .rootable
            .iter()
            .copied()
            .chain(
                self.by_name
                    .values()
                    .flatten()
                    .copied()
                    .chain(self.overlay_specs.iter().copied())
                    .filter(|spec| {
                        spec.name.contains("::")
                            || !spec.traits.contains(Traits::TCLOO_METHOD_CONTEXT)
                    })
                    .map(|spec| spec.name),
            )
            .map(|name| name.strip_prefix("::").unwrap_or(name))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
    }

    /// Source availability providers from the compiled universe and this
    /// actual store's authored rows. Neither collection establishes a lookup.
    #[must_use]
    pub fn source_availability_providers(&self, command: &str) -> Option<NameProviders> {
        // naming.diagnostic.original-command-source-unavailability
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-command-source-unavailability.md
        let name = command.strip_prefix("::").unwrap_or(command);
        if !all_dialect_command_names().rootable.contains(name)
            && !self
                .by_name
                .values()
                .flatten()
                .chain(self.overlay_specs.iter())
                .any(|spec| {
                    spec.name.strip_prefix("::").unwrap_or(spec.name) == name
                        && (name.contains("::")
                            || !spec.traits.contains(Traits::TCLOO_METHOD_CONTEXT))
                })
        {
            return None;
        }
        let mut providers = self.providers_in_any_dialect(command).cloned();
        for spec in self
            .by_name
            .values()
            .flatten()
            .chain(self.overlay_specs.iter())
        {
            if spec.name.strip_prefix("::").unwrap_or(spec.name) == name {
                providers
                    .get_or_insert_with(NameProviders::default)
                    .add_spec(spec);
            }
        }
        providers
    }

    /// Who offers `name` across every compiled-in dialect: the providers its
    /// specs' surface rows name, and whether any spec of the name is offered
    /// to every dialect. `None` exactly when
    /// [`Self::known_in_any_dialect`] is `false`.
    ///
    /// The other half of the W002 question. "Exists in some dialect, not this
    /// one" says the name is disabled here only when this document's
    /// environment stands in some relation to the dialect that has it; a name
    /// only an unrelated environment offers (Expect's `system` in a Jim
    /// document) is unknown here, not disabled here. The universe is the one
    /// [`Self::known_in_any_dialect`] reads, so a runtime pack's own names are
    /// absent from both.
    #[must_use]
    pub fn providers_in_any_dialect(&self, name: &str) -> Option<&'static NameProviders> {
        if !self.known_in_any_dialect(name) {
            return None;
        }
        let unrooted = name.strip_prefix("::").unwrap_or(name);
        all_dialect_command_names().providers.get(unrooted)
    }

    /// Look up a command spec by name (dialect-agnostic).
    ///
    /// A leading `::` (global qualifier) falls back to the bare name, so an
    /// explicitly-global call to a built-in (`::foreach`, `::for`, …)
    /// resolves to the same spec as its unqualified form. The fallback rejects
    /// a method-context-only singleton: Tcl has a contextual `my` but no
    /// global `::my`. Real qualified commands such as
    /// `::oo::Helpers::self` retain their namespaced fallback.
    ///
    /// The return is `&'static` because the registry stores only interned
    /// static specs — the identity [`crate::model::SpecKey`] keys the realm
    /// binding layer; the generation work re-keys dynamic pack
    /// specs when non-`'static` specs join.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&'static CommandSpec> {
        if let Some(spec) = self.by_name.get(name).and_then(|v| v.last().copied()) {
            return Some(spec);
        }
        let unrooted = name.strip_prefix("::")?;
        let spec = self.by_name.get(unrooted)?.last().copied()?;
        rooted_fallback_allowed(name, spec).then_some(spec)
    }

    /// Exact spelling lookup; unlike [`Self::get`], does not apply rooted
    /// global-name fallback.
    #[must_use]
    pub fn get_exact(&self, name: &str) -> Option<&'static CommandSpec> {
        self.by_name.get(name).and_then(|v| v.last().copied())
    }

    /// Whether a **fresh interpreter** of this registry's dialect already holds
    /// a command at exactly `qualified_name` — the question `namespace
    /// import`'s "already exists" conflict asks, and the one `info commands
    /// ::x` answers.
    ///
    /// Deliberately *not* [`Self::get`], which resolves every **spelling** a
    /// call site may legally write, including spellings that only become
    /// callable after an explicit `namespace import`.  The bare operator forms
    /// are exactly that case, and conflating the two inverts a real answer:
    ///
    /// ```text
    /// # oracle, tclsh 9.0.4 and 8.6.14, byte-identical
    /// info commands ::+            ;# -> {}          (empty!)
    /// + 1 2                        ;# -> invalid command name "+"
    /// info commands ::set          ;# -> ::set
    ///
    /// namespace eval ::Ops { proc + {a b} {…}; namespace export + }
    /// namespace import ::Ops::*    ;# -> OK, namespace origin ::+ is ::Ops::+
    ///
    /// namespace eval ::Foo { proc set {a b} {…}; namespace export set }
    /// namespace import ::Foo::*    ;# -> can't import command "set": already exists
    /// ```
    ///
    /// So `set` blocks an unforced import and `+` does not, even though
    /// [`Self::get`] answers `Some` for both.  A bare
    /// [`Traits::OPERATOR_COMMAND`] spelling is the post-`namespace import
    /// ::tcl::mathop::*` form and is excluded here; the namespaced
    /// `::tcl::mathop::+` spelling is a genuine member of a fresh
    /// interpreter's command table and is kept.
    ///
    /// The name must also *be* the spec's own canonical name, so `get`'s
    /// leading-`::` fallback cannot make a namespaced command answer for a
    /// global one.
    ///
    /// # Known imprecision
    ///
    /// A command a *package* provides (`::csv::split`, `::math::statistics::mean`)
    /// is declared here whether or not the script `package require`s it, so
    /// asking about a package's own namespace can over-claim.  The registry
    /// carries no "needs a `package require`" marker to gate on yet.  Reaching
    /// that case means importing *into* a package's own namespace, which is
    /// why it is documented rather than worked around.
    #[must_use]
    pub fn declares_command_at(&self, qualified_name: &str) -> bool {
        let bare = qualified_name.trim_start_matches("::");
        self.get(bare).is_some_and(|spec| {
            // `get` resolves spellings; this asks about one exact name, so the
            // spec has to be the one that *is* that command.
            if spec.name.trim_start_matches("::") != bare {
                return false;
            }
            if !bare.contains("::") && spec.traits.contains(Traits::TCLOO_METHOD_CONTEXT) {
                return false;
            }
            // The namespaced `::tcl::mathop::+` is in a fresh interpreter's
            // command table; the bare `+` it can be imported to is not.
            bare.contains("::") || !spec.traits.contains(Traits::OPERATOR_COMMAND)
        })
    }

    /// The typed BPF-Tcl lowering descriptor for `name`, when `name` is a
    /// BPF-dialect command (see [`crate::bpf_op`]).  The BPF-Tcl front-end
    /// dispatches on this — never on the command name.
    #[must_use]
    pub fn bpf_op(&self, name: &str) -> Option<&'static crate::bpf_op::BpfOpSpec> {
        self.get(name).and_then(|s| s.bpf_op)
    }

    /// Validated condition/body positions for a closed BPF language command.
    /// The frontend must separately retain literal original words and reject
    /// expansions. This syntax projection provides no Tcl execution evidence.
    #[must_use]
    pub fn bpf_conditional_operands(
        &self,
        name: &str,
        arguments: &[&str],
    ) -> Option<Vec<(Option<usize>, usize)>> {
        (self.bpf_op(name)?.kind == crate::bpf_op::BpfOpKind::Conditional)
            .then(|| crate::commands::tcl::bpf_conditional_operands(arguments))?
    }

    /// Find the registered spelling for a typed BPF-Tcl operation.
    ///
    /// This is the reverse lookup companion to [`Self::bpf_op`].  Tools that
    /// generate BPF-Tcl source can select an operation by its registry-owned
    /// [`crate::bpf_op::BpfOpKind`] instead of embedding a command spelling.
    /// It deliberately returns the first registered spelling: aliases must
    /// share an operation descriptor, and the canonical command specs are
    /// registered before any compatibility aliases.
    #[must_use]
    pub fn bpf_command_for(&self, kind: crate::bpf_op::BpfOpKind) -> Option<&'static str> {
        self.by_name
            .values()
            .flat_map(|specs| specs.iter())
            .find_map(|spec| (spec.bpf_op.is_some_and(|op| op.kind == kind)).then_some(spec.name))
    }

    /// Look up a command spec filtered by dialect, picking the
    /// **most-specific** visible spec (`best_visible` — §5.3's single
    /// selection rule).
    ///
    /// As with [`Self::get`], a leading `::` falls back to the bare name while
    /// rejecting method-context-only singleton spellings.
    ///
    /// A registry built for a dialect profile additionally applies that
    /// profile's operator-head exclusion ([`Self::spec_visible`]) whenever
    /// the query is about the profile's own surface — so an f5-irules query
    /// on the f5-irules registry sees exactly the specs iRules ships, no
    /// matter which consumer asks (dialect-profile-model.md §9.2).
    #[must_use]
    pub fn get_for_surface(
        &self,
        name: &str,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&'static CommandSpec> {
        if let Some(specs) = self.by_name.get(name) {
            return self.best_visible(specs, dialect);
        }
        let unrooted = name.strip_prefix("::")?;
        let spec = self.best_visible(self.by_name.get(unrooted)?, dialect)?;
        rooted_fallback_allowed(name, spec).then_some(spec)
    }

    /// Resolve `head` the way this registry's own availability rules
    /// resolve it: through [`Self::get_for_surface`] against the dialect
    /// profile this registry was built for, or through the dialect-agnostic
    /// [`Self::get`] when it has no profile.
    ///
    /// The single place the "same availability rules as `get`" promise made
    /// by the profile-aware behaviour queries is implemented, so a query
    /// added later cannot quietly answer for a command the profile's
    /// dialect does not have.
    fn spec_for_this_registry(&self, head: &str) -> Option<&CommandSpec> {
        match self.own_surface_query() {
            Some(query) => self.get_for_surface(head, Some(query)),
            None => self.get(head),
        }
    }

    /// Whether this registry's **own dialect** has a command spelled `head`.
    ///
    /// The public face of [`Self::spec_for_this_registry`], for callers that
    /// only need the yes/no: a constant-folder deciding whether it may
    /// pre-compute a call, say. Folding is a rewrite that skips the runtime's
    /// availability gate entirely, so a fold that does not ask this question
    /// silently *adds* the command to releases that never had it — a
    /// `tcl8.4` compile folded `[dict create a 1 a 2]` to a literal instead of
    /// raising `invalid command name "dict"`.
    ///
    /// A profile-less registry (`build_default`) answers dialect-agnostically,
    /// exactly as [`Self::get`] does.
    #[must_use]
    pub fn has_command_in_this_dialect(&self, head: &str) -> bool {
        self.spec_for_this_registry(head).is_some()
    }

    /// Which `TclOO` method-context keyword `head` is, if it is one.
    ///
    /// The single source of truth other consumers would otherwise duplicate as
    /// `head == "my"` / `matches!(head, "my" | "next" | "nextto")` literals.
    /// Every consumer that needs "is this word a method
    /// dispatch or introspection keyword, and which kind" asks here, so a
    /// dialect that gains or loses one of them propagates through the specs
    /// rather than through a walker edit.
    ///
    /// Only the bare, contextual spelling is a dispatch keyword. `TclOO` installs
    /// `my` in the receiving object's namespace and exposes `next` / `nextto` /
    /// `self` through that namespace's path; it does not install global
    /// `::my`, `::next`, `::nextto`, or `::self` commands. Qualified
    /// `::oo::Helpers::...` commands are separate registry entries with the
    /// dispatch traits removed. [`Self::get`] owns that distinction and does
    /// not apply its leading-`::` fallback to a method-context-only singleton.
    ///
    /// Dialect-aware through the registry instance itself: a registry built
    /// by `registry_for_dialect` / `registry_for_profile` answers at that
    /// profile's point, so all four keywords — every one of them 8.6-and-
    /// later — return `None` from a `tcl8.4` or `tcl8.5` registry. A
    /// profile-less registry (`CommandRegistry::build_default`) answers
    /// dialect-agnostically, exactly as [`Self::get`] does.
    ///
    /// `link` is **not** a keyword here and must not be added: it creates
    /// per-class bareword commands rather than dispatching one. Nor is
    /// `self`'s definer-grammar homonym — the `self` word
    /// inside an `oo::define` body is a member-grammar wrapper, resolved
    /// through [`crate::definer`], not a command head.
    #[must_use]
    pub fn method_dispatch_keyword(&self, head: &str) -> Option<MethodDispatchKind> {
        let traits = self.spec_for_this_registry(head)?.traits;
        if traits.contains(Traits::TCLOO_SELF_DISPATCH) {
            Some(MethodDispatchKind::SelfDispatch)
        } else if traits.contains(Traits::TCLOO_NEXT_CHAIN) {
            Some(MethodDispatchKind::NextChain)
        } else if traits.contains(Traits::TCLOO_INTROSPECTION) {
            Some(MethodDispatchKind::Introspection)
        } else {
            None
        }
    }

    /// Whether a bracketed command substitution `[cmd ?arg?]` denotes the
    /// current `TclOO` receiving object — so a consumer resolving what a
    /// dispatch head (`[cmd ?arg?] method`) means should treat it exactly
    /// like `my method`: the enclosing class, not a structurally-inferred
    /// type. `arg` is the substitution's own first word, `None` for a bare
    /// call (`[self]` as opposed to `[self object]`).
    ///
    /// Registry data via [`CommandSpec::self_receiver_words`]
    /// (`self`/`object` today) rather than name-matching `cmd` — a
    /// `TCLOO_INTROSPECTION` command answers `Introspection` from
    /// [`Self::method_dispatch_keyword`] (its argument is a closed
    /// subcommand set, never a method name) for every *other* word, since
    /// this is a narrower, additional fact about specific words of that
    /// same closed set, not a fourth [`MethodDispatchKind`] axis: unlike
    /// `my`, the value only dispatches once *substituted* as a command
    /// head, and unlike plain introspection, this one specific word's
    /// result is the receiver itself.
    #[must_use]
    pub fn is_self_receiver_call(&self, cmd: &str, arg: Option<&str>) -> bool {
        let Some(spec) = self.spec_for_this_registry(cmd) else {
            return false;
        };
        if spec.self_receiver_words.is_empty() {
            return false;
        }
        match arg {
            None => spec.arity.min == 0,
            Some(word) => spec.self_receiver_words.contains(&word),
        }
    }

    /// Whether `head`'s **bare** spelling resolves only from inside a
    /// `TclOO` method context; see [`Traits::TCLOO_METHOD_CONTEXT`] for the oracle
    /// transcripts.
    ///
    /// Consumers pair this with their own "is this call site inside a
    /// `TclOO` method body?" fact
    /// (`tcl_compiler::analyser::scope::innermost_scope_reaches_oo_helpers`)
    /// — the registry knows *which* commands are scoped, the call site
    /// knows *where* it is, and neither needs the other's command names.
    ///
    /// Dialect-aware exactly like [`Self::method_dispatch_keyword`]: a
    /// registry built for a profile answers at that profile's point, so a
    /// `tcl8.4` registry — which has no `TclOO` at all — answers `false`
    /// for every one of them.
    ///
    /// A qualified spelling (`oo::Helpers::link`, `::oo::Helpers::link`) is
    /// a separate, unscoped spec, so it answers `false`: the command really
    /// does exist globally, and calling it outside a method is a runtime
    /// error rather than an unknown command.
    #[must_use]
    pub fn resolves_only_in_method_context(&self, head: &str) -> bool {
        self.spec_for_this_registry(head)
            .is_some_and(|spec| spec.traits.contains(Traits::TCLOO_METHOD_CONTEXT))
    }

    /// Whether **calling** `head` needs a real `TclOO` method invocation,
    /// not merely a frame that can resolve it — see
    /// [`Traits::TCLOO_REQUIRES_METHOD_FRAME`] for the oracle transcript.
    ///
    /// The narrower companion to [`Self::resolves_only_in_method_context`],
    /// and the two answer differently in exactly one place: a Tcl 9 class
    /// `initialise` / `initialize` body, where the whole family resolves
    /// (so `W123` must stay silent) but only `my` actually runs (so
    /// completion and hover must offer only `my`).
    ///
    /// A consumer deciding "may I offer this word here / does it hover"
    /// wants **both**: the word must resolve at the call site *and*, when
    /// this answers `true`, the call site must be a method frame rather
    /// than a bare object frame. `tcl_lsp_core::oo_dispatch` pairs them
    /// once so no consumer re-derives the rule.
    ///
    /// Dialect-aware exactly like [`Self::resolves_only_in_method_context`].
    #[must_use]
    pub fn requires_oo_method_frame(&self, head: &str) -> bool {
        self.spec_for_this_registry(head)
            .is_some_and(|spec| spec.traits.contains(Traits::TCLOO_REQUIRES_METHOD_FRAME))
    }

    /// Resolve `word` against the registry-declared universal object-command
    /// surface.
    ///
    /// `TclOO` receiver commands are runtime values, so their source head is not
    /// itself a static registry command.  This query gives common consumers
    /// the inherited method metadata without encoding the registry entry that
    /// owns that surface.  A dialect may supply at most one such surface; an
    /// accidental duplicate is treated conservatively as no unique answer.
    #[must_use]
    pub fn object_command_method(&self, word: &str) -> Option<&SubCommand> {
        let mut matches = self.by_name.values().filter_map(|specs| {
            let spec = specs.last()?;
            spec.traits
                .contains(Traits::OBJECT_COMMAND_SURFACE)
                .then_some(spec)?
                .resolve_subcommand(word)
        });
        let method = matches.next()?;
        matches.next().is_none().then_some(method)
    }

    /// Whether `word` is a destructive operation on the universal object
    /// command surface.
    #[must_use]
    pub fn is_destructive_object_method(&self, word: &str) -> bool {
        self.object_command_method(word)
            .is_some_and(|method| method.destructive)
    }

    /// Whether `word` is a **manufacturer method** of any definer family the
    /// registry models — `create` / `new` / `createWithNamespace` for
    /// `TclOO`, `create` for snit.
    ///
    /// The union over every definer grammar, for the one consumer that has a
    /// class name but not the family it belongs to: a *pure consumer*
    /// document holding `set w [Widget create x]` where `Widget` is declared
    /// in another file. A consumer that knows the family must
    /// ask its grammar
    /// ([`crate::definer::DefinitionBodyGrammar::manufacturer`]) instead —
    /// that answer is exact, this one is a union.
    #[must_use]
    pub fn is_manufacturer_method(&self, word: &str) -> bool {
        self.manufacturer_methods(word)
            .any(|method| method.visibility == crate::definer::MemberVisibility::Exported)
    }

    /// Every definer family's declaration of the manufacturer method `word`
    /// — the iterator behind [`Self::is_manufacturer_method`], for a consumer
    /// that needs the layout (which argument names the instance) and not just
    /// the yes/no.
    pub fn manufacturer_methods<'a>(
        &'a self,
        word: &'a str,
    ) -> impl Iterator<Item = &'static crate::definer::ManufacturerMethod> + 'a {
        self.by_name.values().filter_map(move |specs| {
            let spec = specs.last()?;
            spec.manufacturer_methods
                .iter()
                .find(|method| method.keyword == word)
                .or_else(|| {
                    spec.definition_body
                        .and_then(|grammar| grammar.manufacturer(word))
                })
        })
    }

    /// The exact manufacturer descriptor for registered command `head` and
    /// method `word`. This covers both definition-body commands and package
    /// class factories carrying standalone manufacturer data.
    #[must_use]
    pub fn manufacturer_method(
        &self,
        head: &str,
        word: &str,
    ) -> Option<&'static crate::definer::ManufacturerMethod> {
        Self::manufacturer_method_for_descriptor(self.get(head)?, word)
    }

    /// Manufacturer vocabulary of one independently selected descriptor.
    /// The caller retains availability and original invocation ownership.
    #[must_use]
    pub(crate) fn manufacturer_method_for_descriptor(
        spec: &CommandSpec,
        word: &str,
    ) -> Option<&'static crate::definer::ManufacturerMethod> {
        if !spec.manufacturer_methods.is_empty() {
            return spec
                .manufacturer_methods
                .iter()
                .find(|method| method.keyword == word);
        }
        spec.definition_body
            .and_then(|grammar| grammar.manufacturer(word))
    }

    /// The exact manufacturer descriptor when ordinary external dispatch
    /// through registered command `head` may reach it.
    ///
    /// This is the call-classification counterpart of
    /// [`Self::manufacturer_method`]. Consumers examining a real command
    /// invocation should normally use this method; definition-body analysis
    /// that models `self export` / `self unexport` needs the unfiltered
    /// descriptor and applies the class-local visibility changes itself.
    #[must_use]
    pub fn exported_manufacturer_method(
        &self,
        head: &str,
        word: &str,
    ) -> Option<&'static crate::definer::ManufacturerMethod> {
        self.manufacturer_method(head, word)
            .filter(|method| method.visibility == crate::definer::MemberVisibility::Exported)
    }

    /// Definition grammar entered by an outer literal script form.
    /// Configuration operations select their target from the typed transition
    /// owner: an immediately following script body is a definition region,
    /// while a later method body has ordinary Tcl command context. This is
    /// structural assistance, without target existence or runtime admission.
    #[must_use]
    pub fn outer_definition_body_grammar(
        &self,
        command: &str,
        args: &[&str],
    ) -> Option<&'static DefinitionBodyGrammar> {
        let grammar = self.get(command)?.definition_body?;
        let invocation = self.resolve_invocation(command, args, None)?;
        let transitions = invocation.state_transitions();
        let target = transitions
            .facts()
            .iter()
            .find_map(|fact| match &fact.transition {
                crate::StateTransition::ObjectDispatch(
                    crate::ObjectDispatchTransition::Configure { target, .. },
                ) => target.argument_index(),
                _ => None,
            });
        if let Some(target) = target {
            return self
                .arg_indices_for_role(command, args, ArgRole::Body)
                .contains(&target.checked_add(1)?)
                .then_some(grammar);
        }
        Some(grammar)
    }

    /// Nested source vocabulary of an admitted member of this store's selected
    /// closed document grammar. The current and nested grammars must share its
    /// actual family; ordinary command execution and state transitions are not
    /// premises for a document declaration. This supplies source grammar only,
    /// without runtime dispatch, Native naming or editing authority.
    #[must_use]
    pub fn authored_document_member_grammar(
        &self,
        current: &DefinitionBodyGrammar,
        keyword: &str,
    ) -> Option<&'static DefinitionBodyGrammar> {
        let document = self.document_grammar()?;
        if current.family != document.family || !current.is_member(keyword) {
            return None;
        }
        let nested = self.get(keyword)?.definition_body?;
        (nested.family == document.family).then_some(nested)
    }

    /// The first constructor-payload argument for manufacturer `word`, when
    /// every definer family declaring that word agrees on the layout.
    ///
    /// Consumers that know the exact family should query its grammar
    /// directly. Scope-blind flow analyses use this conservative union: a
    /// future family reusing a keyword with a different layout makes the
    /// answer `None`, so parameter flow abstains instead of skipping the
    /// wrong structural words.
    #[must_use]
    pub fn uniform_manufacturer_constructor_args_from(&self, word: &str) -> Option<usize> {
        let mut methods = self.manufacturer_methods(word);
        let first = usize::from(methods.next()?.constructor_args_from);
        methods
            .all(|method| usize::from(method.constructor_args_from) == first)
            .then_some(first)
    }

    /// The instance-name argument for manufacturer `word`, when every
    /// definer family declaring that word agrees on the layout.
    ///
    /// `None` is deliberately ambiguous: the method may generate its own
    /// name, or different families may use different layouts. Consumers
    /// that need to distinguish those cases should use
    /// [`Self::manufacturer_methods`] and require agreement themselves.
    #[must_use]
    pub fn uniform_manufacturer_names_instance_at(&self, word: &str) -> Option<usize> {
        let mut methods = self.manufacturer_methods(word);
        let first = methods.next()?.names_instance_at?;
        methods
            .all(|method| method.names_instance_at == Some(first))
            .then_some(usize::from(first))
    }

    /// Whether `word` can conservatively identify a constructor call when a
    /// consumer knows only that the head names some class, not which definer
    /// family created it.
    ///
    /// Named manufacturer methods and family-specific bare-word naming hints
    /// are both registry data. Exact class-aware consumers should use the
    /// class grammar instead of this union.
    #[must_use]
    pub fn is_possible_class_construction_word(&self, word: &str) -> bool {
        self.is_manufacturer_method(word)
            || self.by_name.values().any(|specs| {
                specs.last().is_some_and(|spec| {
                    spec.definition_body
                        .and_then(|grammar| grammar.bare_word_construction_hint)
                        .is_some_and(|recognises| recognises(word))
                })
            })
    }

    /// Whether `head` binds bareword aliases for methods of the current
    /// object — `TclOO`'s `link`; see [`Traits::TCLOO_BINDS_METHOD_ALIAS`].
    ///
    /// The registry-first replacement for a `texts[0] == "link"` literal an
    /// analyser's class-body walk would otherwise carry, and dialect-aware
    /// for the same reason [`Self::method_dispatch_keyword`]
    /// is: `link` is 9.0-core / 8.6-via-`ooutil`, so an 8.5 registry answers
    /// `false`.
    #[must_use]
    pub fn binds_method_alias(&self, head: &str) -> bool {
        self.spec_for_this_registry(head)
            .is_some_and(|spec| spec.traits.contains(Traits::TCLOO_BINDS_METHOD_ALIAS))
    }

    /// The single spec-selection rule (§5.3, D6): among the specs of one
    /// name visible under `dialect`, pick the **most specific** — a
    /// dialect-scoped spec beats a catch-all (`surface: None`), a spec
    /// offered by a nearer core point of the query beats one offered only
    /// by a farther one (a document's own family over its ancestry
    /// anchor), a narrower surface beats a wider one, and among equals the
    /// *last-registered* spec wins, so curated pack overrides keep beating
    /// the data they shadow. `get_for_surface`, the iRules event
    /// cross-product, and (via `ProfileQueries::resolve_command`) the CLI
    /// snapshot all resolve through this one rule.
    fn best_visible(
        &self,
        specs: &[&'static CommandSpec],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&'static CommandSpec> {
        let mut best: Option<(usize, &CommandSpec)> = None;
        // Breadth only breaks a tie between two scoped candidates. Most names
        // have one visible spec, so calculating it before a tie is known
        // repeatedly walks their authored availability windows for no effect.
        // The best candidate's nearness is held for the same reason: it is
        // compared against every later candidate.
        let mut best_breadth: Option<u32> = None;
        let mut best_nearness: Option<usize> = None;
        // Nearness only distinguishes candidates when the query has more
        // than one core point; with one, every admitted row ranks `0`.
        let nearness_query = dialect.filter(|query| query.core.len() > 1);
        let nearness = |rows| {
            nearness_query.map_or(0, |query| {
                surface_nearness(rows, &query).unwrap_or(usize::MAX)
            })
        };

        for (index, spec) in specs.iter().copied().enumerate() {
            if !self.spec_visible(spec, dialect) {
                continue;
            }
            let Some((best_index, best_spec)) = best else {
                best = Some((index, spec));
                continue;
            };
            match (best_spec.surface, spec.surface) {
                (None, Some(_)) => {
                    best = Some((index, spec));
                    best_breadth = None;
                    best_nearness = None;
                }
                (Some(_), None) => {}
                (None, None) => {
                    // Equal catch-all scopes still use last registration.
                    best = Some((index, spec));
                }
                (Some(best_rows), Some(rows)) => {
                    let held = *best_nearness.get_or_insert_with(|| nearness(best_rows));
                    let candidate = nearness(rows);
                    match candidate.cmp(&held) {
                        Ordering::Less => {
                            best = Some((index, spec));
                            best_breadth = None;
                            best_nearness = Some(candidate);
                        }
                        Ordering::Greater => {}
                        Ordering::Equal => {
                            let old_breadth =
                                *best_breadth.get_or_insert_with(|| surface_breadth(best_rows));
                            let breadth = surface_breadth(rows);
                            if breadth < old_breadth
                                || (breadth == old_breadth && index > best_index)
                            {
                                best = Some((index, spec));
                                best_breadth = Some(breadth);
                            }
                        }
                    }
                }
            }
        }
        best.map(|(_, spec)| spec)
    }

    /// Whether `spec` is visible to any of `providers` — the coarse,
    /// version-blind twin of [`Self::spec_visible`] the static grammars use.
    ///
    /// First-paint highlighting has no resolved release to ask about, so it
    /// asks which providers the profile's language spans. The operator-head
    /// exclusion still applies: math operators are not command heads under
    /// iRules at any release.
    #[must_use]
    pub fn spec_visible_to(&self, spec: &CommandSpec, providers: &[SpecProvider]) -> bool {
        spec.surface
            .is_none_or(|rows| surface_provided_by(rows, providers))
            && self.profile.is_none_or(|profile| {
                profile.operators_as_commands
                    || !spec
                        .traits
                        .contains(crate::traits::Traits::OPERATOR_COMMAND)
            })
    }

    /// The full availability test for a point query on this registry: the
    /// spec's own surface gate, plus — when this registry was built for a
    /// profile and the query is that profile's own point — the
    /// operator-command exclusion (§9), for a profile whose math operators
    /// are not command heads.
    ///
    /// There is no disable list: availability is fully explicit in each
    /// spec's surface rows, so a sandbox-banned command such as `exec`
    /// simply never names the iRules family.
    ///
    /// Public because generators projecting a command surface for an
    /// explicit point need the same exclusion semantics `get_for_surface`
    /// applies internally.
    #[must_use]
    pub fn spec_visible(&self, spec: &CommandSpec, dialect: Option<SurfaceQuery<'_>>) -> bool {
        if !dialect.map_or_else(
            || spec.supports_dialect(None),
            |query| crate::irules_policy::runtime_surface_admits(spec, query),
        ) {
            return false;
        }
        let Some(profile) = self.profile else {
            return true;
        };
        if dialect.is_none_or(|query| !query.same_point(&profile.surface_query())) {
            // The query is about some other surface's availability; this
            // profile's operator-exclusion does not apply to it.
            return true;
        }
        // iRules availability is stated positively by each spec, so there
        // is no subtractive ban list — the only profile-level exclusion left
        // is the operator-command one (math operators are not command heads
        // under iRules).
        profile.operators_as_commands
            || !spec
                .traits
                .contains(crate::traits::Traits::OPERATOR_COMMAND)
    }

    /// Return all registered command names.
    pub fn command_names(&self) -> impl Iterator<Item = &str> {
        self.by_name.keys().copied()
    }

    /// Audited native root factory whose default construction has no user
    /// hook graph. A document binding, authored override or arbitrary `TclOO`
    /// metaclass cannot inherit this admission by sharing a grammar.
    #[must_use]
    pub fn default_construction_grammar(
        &self,
        command: &str,
    ) -> Option<&'static crate::definer::DefinitionBodyGrammar> {
        self.selected_default_construction_grammar(command, self.own_surface_query())
    }

    /// Native root-factory descriptor under the retained invocation axes.
    /// This selects metadata only; live factory, allocation and effect
    /// dependencies must independently close before construction is inferred.
    #[must_use]
    pub fn native_default_construction_grammar(
        &self,
        command: &str,
        dialect: crate::InvocationDialect,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> Option<&'static crate::definer::DefinitionBodyGrammar> {
        if dialect.family() != Some(tcl_dialect::model::Family::Tcl)
            || dialect.tcl_version.is_none()
        {
            return None;
        }
        self.selected_default_construction_grammar(
            command,
            Some(dialect.authoring_query()?.with_realm(realm)),
        )
    }

    /// Original stock class-factory constructor/bootstrap recipe under the
    /// actual retained invocation axes. Current command/support allocations,
    /// body execution and successful completion are independent obligations.
    #[must_use]
    pub fn native_class_factory_recipe(
        &self,
        command: &str,
        dialect: crate::InvocationDialect,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> Option<crate::native_tcloo_bootstrap::NativeClassFactoryRecipe> {
        let spec =
            self.get_for_surface(command, Some(dialect.authoring_query()?.with_realm(realm)))?;
        if spec_pack_of(spec) != Some("tcl")
            || !spec.traits.contains(Traits::IS_OO_METACLASS)
            || spec.definition_body.is_none()
        {
            return None;
        }
        crate::native_tcloo_bootstrap::NativeClassFactoryRecipe::select(spec.name, dialect)
    }

    fn selected_default_construction_grammar(
        &self,
        command: &str,
        query: Option<SurfaceQuery<'_>>,
    ) -> Option<&'static crate::definer::DefinitionBodyGrammar> {
        let spec = self.get_for_surface(command, query)?;
        if spec.name != "oo::class" || spec_pack_of(spec) != Some("tcl") {
            return None;
        }
        spec.definition_body
    }

    /// The one-pass semantic projection of this exact registry generation.
    ///
    /// Repeated calls return the same `Arc`. Any later authored insertion or
    /// profile change invalidates the registry's cell, so the next call builds
    /// a fresh projection while existing readers safely retain the old,
    /// immutable generation.
    #[must_use]
    pub fn effective_semantics(&self) -> Arc<EffectiveRegistrySemantics> {
        Arc::clone(self.effective_semantics.get_or_init(|| {
            let query = self.own_surface_query();
            let mut commands = FxHashMap::default();
            let mut binding_names = BTreeSet::new();
            let mut unresolved_command_handlers = BTreeSet::new();

            for (&name, specs) in &self.by_name {
                let canonical = tcl_syntax::naming::normalise_qualified_name(name);
                binding_names.insert(canonical.clone());
                if specs
                    .last()
                    .is_some_and(|spec| spec.traits.contains(Traits::UNRESOLVED_COMMAND_HANDLER))
                {
                    unresolved_command_handlers.insert(canonical);
                }
                let selected = match query {
                    Some(query) => self.best_visible(specs, Some(query)),
                    None => specs.last().copied(),
                };
                let Some(spec) = selected else {
                    continue;
                };
                commands.insert(
                    name,
                    EffectiveCommandSemantics {
                        traits: spec.traits,
                        lowering_hook: spec.lowering_hook,
                    },
                );
            }

            // These native slots belong to TclOO's definition namespace,
            // rather than ordinary callable catalogue rows. Their owner and
            // release windows come from the admitted native grammar.
            if let Some(grammar) = self.default_construction_grammar("oo::class") {
                for member in grammar.members {
                    for receiver in [
                        crate::definer::DefinitionReceiver::Instance,
                        crate::definer::DefinitionReceiver::Class,
                    ] {
                        if let Some(lookup) =
                            grammar.definition_member_lookup_for_receiver(member, receiver, query)
                        {
                            binding_names.insert(lookup.implementation);
                        }
                    }
                }
            }

            if let Some(profile) = self.profile {
                binding_names.extend(
                    self.stock_native_implementation_slots(crate::InvocationDialect::of_profile(
                        profile,
                    ))
                    .into_iter()
                    .map(|lookup| lookup.slot.to_owned()),
                );
            }

            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            if let Some(profile) = self.profile {
                let dialect = crate::InvocationDialect::of_profile(profile);
                if let Some(support) = self
                    .native_class_factory_recipe(
                        "oo::configurable",
                        dialect,
                        tcl_dialect::model::InvocationRealm::RuleLoader,
                    )
                    .and_then(crate::native_tcloo_bootstrap::NativeClassFactoryRecipe::support)
                {
                    binding_names.extend(
                        support
                            .binding_names()
                            .iter()
                            .map(|name| (*name).to_owned()),
                    );
                }
            }
            binding_names.hash(&mut hasher);
            unresolved_command_handlers.hash(&mut hasher);
            Arc::new(EffectiveRegistrySemantics {
                commands,
                binding_names,
                unresolved_command_handlers,
                binding_fingerprint: hasher.finish(),
            })
        }))
    }

    /// Descriptor roster for a separate authored source model under the
    /// actual full availability context. This supplies no native interpreter
    /// presence, compiler implementation slots or name policy. The source
    /// driver must independently retain its positively selected Logical input.
    #[must_use]
    pub fn authored_source_semantics_in_context(
        &self,
        context: &crate::model::ResolvedContext,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> Arc<EffectiveRegistrySemantics> {
        // naming.source.logical-procedure-definition-model
        // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
        let mut commands = FxHashMap::default();
        let mut binding_names = BTreeSet::new();
        let mut unresolved_command_handlers = BTreeSet::new();
        for &name in self.by_name.keys() {
            let Some(spec) = context.resolve_spec_in_realm(self, name, realm) else {
                continue;
            };
            let canonical = tcl_syntax::naming::normalise_qualified_name(name);
            // Source metadata remains known for hosted packages, but their
            // names are not initial bindings until this context activates them.
            let binding_available = spec
                .owning_package()
                .is_none_or(|package| context.package_provider_active(package));
            if binding_available && rooted_fallback_allowed(&canonical, spec) {
                binding_names.insert(canonical.clone());
            }
            if binding_available && spec.traits.contains(Traits::UNRESOLVED_COMMAND_HANDLER) {
                unresolved_command_handlers.insert(canonical);
            }
            commands.insert(
                name,
                EffectiveCommandSemantics {
                    traits: spec.traits,
                    lowering_hook: spec.lowering_hook,
                },
            );
        }
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        binding_names.hash(&mut hasher);
        unresolved_command_handlers.hash(&mut hasher);
        Arc::new(EffectiveRegistrySemantics {
            commands,
            binding_names,
            unresolved_command_handlers,
            binding_fingerprint: hasher.finish(),
        })
    }

    /// Entry command table for a concrete execution dialect. Jim's current
    /// default build has a distinct command roster; catalogue union entries
    /// cannot establish the presence of C-only commands or autoload handlers.
    #[must_use]
    pub fn effective_semantics_for_dialect(
        &self,
        dialect: crate::InvocationDialect,
    ) -> Arc<EffectiveRegistrySemantics> {
        self.effective_semantics_for_dialect_in_realm(
            dialect,
            tcl_dialect::model::InvocationRealm::RuleLoader,
        )
    }

    /// Entry command table under an explicit availability phase. Runtime
    /// presence does not license rule-loader source or native compiler hooks.
    #[must_use]
    pub fn effective_semantics_for_dialect_in_realm(
        &self,
        dialect: crate::InvocationDialect,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> Arc<EffectiveRegistrySemantics> {
        let query = dialect.authoring_query().map(|mut query| {
            if let Some(profile) = self
                .profile
                .filter(|profile| crate::InvocationDialect::of_profile(profile) == dialect)
            {
                query.packages = profile.surface_packages;
            }
            query.with_realm(realm)
        });
        let jim = dialect.family() == Some(tcl_dialect::model::Family::Jim);
        // A roster observed for one release/build cannot prove any other build.
        let audited_jim = dialect.core_point
            == Some(tcl_dialect::model::DialectPoint::canonical(
                tcl_dialect::model::Release::JIM_0_84,
            ));

        let jim_names = jim_fresh_command_names();
        let mut binding_names = if jim && audited_jim {
            jim_names.clone()
        } else {
            BTreeSet::new()
        };
        let mut commands = FxHashMap::default();
        let mut unresolved_command_handlers = BTreeSet::new();
        for (&name, specs) in &self.by_name {
            let Some(query) = query else {
                continue;
            };
            let Some(spec) = self.selected_entry_descriptor(specs, dialect, query) else {
                continue;
            };
            let canonical = tcl_syntax::naming::normalise_qualified_name(name);
            if jim {
                if !audited_jim || !jim_names.contains(&canonical) {
                    continue;
                }
            } else {
                // Unversioned C admits only commands present across the whole
                // supported ladder. A catalogue union supplies assistance only.
                if dialect.family() == Some(tcl_dialect::model::Family::Tcl)
                    && dialect.tcl_version.is_none()
                    && !tcl_dialect::TclVersion::ALL.into_iter().all(|release| {
                        self.best_visible(
                            specs,
                            Some(SurfaceQuery::core(
                                tcl_dialect::model::Family::Tcl,
                                release.version_string(),
                            )),
                        )
                        .is_some()
                    })
                {
                    continue;
                }
                if rooted_fallback_allowed(&canonical, spec) {
                    binding_names.insert(canonical.clone());
                }
            }
            commands.insert(
                name,
                EffectiveCommandSemantics {
                    traits: spec.traits,
                    lowering_hook: spec.lowering_hook,
                },
            );
            if spec.traits.contains(Traits::UNRESOLVED_COMMAND_HANDLER) {
                unresolved_command_handlers.insert(canonical);
            }
        }
        if !jim
            && let Some(spec) = self.get_for_surface("oo::class", query)
            && let Some(grammar) = spec.definition_body
        {
            for member in grammar.members {
                for receiver in [
                    crate::definer::DefinitionReceiver::Instance,
                    crate::definer::DefinitionReceiver::Class,
                ] {
                    if let Some(lookup) =
                        grammar.definition_member_lookup_for_receiver(member, receiver, query)
                    {
                        binding_names.insert(lookup.implementation);
                    }
                }
            }
        }
        binding_names.extend(
            self.stock_native_implementation_slots(dialect)
                .into_iter()
                .map(|lookup| lookup.slot.to_owned()),
        );
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.extend_native_factory_binding_names(&mut binding_names, dialect, realm);
        binding_names.hash(&mut hasher);
        unresolved_command_handlers.hash(&mut hasher);
        dialect.hash(&mut hasher);
        realm.hash(&mut hasher);
        Arc::new(EffectiveRegistrySemantics {
            commands,
            binding_names,
            unresolved_command_handlers,
            binding_fingerprint: hasher.finish(),
        })
    }

    fn extend_native_factory_binding_names(
        &self,
        names: &mut BTreeSet<String>,
        dialect: crate::InvocationDialect,
        realm: tcl_dialect::model::InvocationRealm,
    ) {
        if let Some(support) = self
            .native_class_factory_recipe("oo::configurable", dialect, realm)
            .and_then(crate::native_tcloo_bootstrap::NativeClassFactoryRecipe::support)
        {
            names.extend(support.binding_names().iter().copied().map(str::to_owned));
        }
    }

    /// Select an entry descriptor under the actual surface and realm, admitting
    /// package-owned commands only from the matching profile's ambient packages.
    fn selected_entry_descriptor(
        &self,
        specs: &[&'static CommandSpec],
        dialect: crate::InvocationDialect,
        query: SurfaceQuery<'_>,
    ) -> Option<&'static CommandSpec> {
        let spec = self.best_visible(specs, Some(query))?;
        if spec.owning_package().is_some_and(|package| {
            !self.profile.is_some_and(|profile| {
                crate::InvocationDialect::of_profile(profile) == dialect
                    && profile.is_ambient_package(package)
            })
        }) {
            return None;
        }
        Some(spec)
    }

    /// Original private compiler slots in a trusted stock entry environment.
    /// This catalogue projection supplies descriptors and fresh-entry names;
    /// a live runtime snapshot must independently supply each actual identity.
    #[must_use]
    pub fn native_compiler_implementation_slots(
        &self,
        dialect: crate::InvocationDialect,
    ) -> Vec<crate::native_compilation::NativeCompilerImplementationLookup> {
        let Some(query) = dialect.authoring_query() else {
            return Vec::new();
        };
        let mut slots = BTreeMap::new();
        for specs in self.by_name.values() {
            let Some(spec) = self.best_visible(specs, Some(query)) else {
                continue;
            };
            if spec.owning_package().is_some() {
                continue;
            }
            for compilation in
                spec.native_compilation
                    .into_iter()
                    .chain(spec.subcommands.iter().flat_map(|subcommand| {
                        subcommand.native_compilation.into_iter().chain(
                            subcommand
                                .sub_subcommands
                                .iter()
                                .filter_map(|worker| worker.native_compilation),
                        )
                    }))
            {
                for lookup in compilation
                    .implementation_prerequisites(dialect)
                    .into_iter()
                    .flatten()
                {
                    slots.insert(lookup.slot, lookup);
                }
            }
        }
        slots.into_values().collect()
    }

    /// Original private handler and compiler slots in an explicitly trusted
    /// stock entry. This bootstrap surface establishes neither live identity
    /// nor compiler-hook availability; those proofs use separate descriptors.
    #[must_use]
    pub fn stock_native_implementation_slots(
        &self,
        dialect: crate::InvocationDialect,
    ) -> Vec<crate::native_compilation::NativeCompilerImplementationLookup> {
        let mut slots: BTreeMap<_, _> = self
            .native_compiler_implementation_slots(dialect)
            .into_iter()
            .map(|lookup| (lookup.slot, lookup))
            .collect();
        if dialect.family() != Some(Family::Tcl) {
            return slots.into_values().collect();
        }
        let Some(query) = dialect.authoring_query() else {
            return slots.into_values().collect();
        };
        for specs in self.by_name.values() {
            let Some(spec) = self.best_visible(specs, Some(query)) else {
                continue;
            };
            if spec.owning_package().is_some() {
                continue;
            }
            for contract in successful_handler_contracts(spec) {
                for lookup in contract.stock_native_workers(dialect) {
                    slots.insert(lookup.slot, lookup);
                }
            }
        }
        slots.into_values().collect()
    }

    /// Native compiler descriptor of one actual stock registration identity.
    ///
    /// `identity` is an already constructed canonical command key. Private
    /// worker registrations reverse-project their authored parent and frozen
    /// prefix before selecting the member descriptor. Ambiguous mappings or
    /// unavailable descriptors retain uncertainty; this never proves that a
    /// live command has the stock implementation.
    #[must_use]
    pub fn native_compilation_for_registration(
        &self,
        identity: &str,
        dialect: crate::InvocationDialect,
    ) -> Option<crate::native_compilation::NativeCompilationSpec> {
        let key = (
            dialect,
            identity.strip_prefix("::").unwrap_or(identity).to_owned(),
        );
        if let Some(cached) = self
            .native_registrations
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .compilers
            .get(&key)
        {
            return *cached;
        }
        let selected = self.native_compilation_for_registration_uncached(&key.1, dialect);
        self.native_registrations
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .compilers
            .insert(key, selected);
        selected
    }

    fn native_compilation_for_registration_uncached(
        &self,
        identity: &str,
        dialect: crate::InvocationDialect,
    ) -> Option<crate::native_compilation::NativeCompilationSpec> {
        let key = identity.strip_prefix("::").unwrap_or(identity);
        let query = dialect.authoring_query()?;
        let rooted = format!("::{key}");
        if let Some(spec) = self
            .get_for_surface(key, Some(query))
            .or_else(|| self.get_for_surface(&rooted, Some(query)))
        {
            return spec
                .native_compilation
                .or_else(|| crate::native_tcloo_registration::compilation(key, dialect));
        }
        if let Some(compilation) = crate::native_tcloo_registration::compilation(key, dialect) {
            return Some(compilation);
        }
        let mut mapping = None;
        let mut selected = None;
        let mut found = false;
        for specs in self.by_name.values() {
            let Some(spec) = self.best_visible(specs, Some(query)) else {
                continue;
            };
            if spec.owning_package().is_some() {
                continue;
            }
            let compilations =
                spec.native_compilation
                    .into_iter()
                    .chain(spec.subcommands.iter().flat_map(|member| {
                        member.native_compilation.into_iter().chain(
                            member
                                .sub_subcommands
                                .iter()
                                .filter_map(|worker| worker.native_compilation),
                        )
                    }));
            for compilation in compilations {
                let Some(path) = compilation.implementation_prerequisites(dialect) else {
                    continue;
                };
                for lookup in &path {
                    if lookup.slot.strip_prefix("::").unwrap_or(lookup.slot) != key {
                        continue;
                    }
                    if mapping.is_some_and(|previous| previous != *lookup) {
                        return None;
                    }
                    mapping = Some(*lookup);
                    let candidate = if matches!(
                        compilation.grammar,
                        crate::native_compilation::NativeCompilationGrammar::WithImplementationPath { .. }
                    ) && path.last() == Some(lookup) {
                        // The terminal registration owns a compiler descriptor
                        // even when its semantic operand offset cannot be rebased.
                        compilation
                    } else {
                        // Registration owns this descriptor independently of
                        // invocation argc; selecting a member needs no fake argv.
                        self.native_registration_compiler_descriptor(identity, dialect)?
                    };
                    if found && selected != Some(candidate) {
                        return None;
                    }
                    selected = Some(candidate);
                    found = true;
                }
            }
        }
        selected.or_else(|| self.native_registration_compiler_descriptor(identity, dialect))
    }

    /// A normal worker roster can publish a registration independently of a
    /// compiler implementation path. Select only its authored frozen member
    /// descriptor; operand cardinality and normal-handler facts play no part.
    fn native_registration_compiler_descriptor(
        &self,
        identity: &str,
        dialect: crate::InvocationDialect,
    ) -> Option<crate::native_compilation::NativeCompilationSpec> {
        let lookup = self.native_registration_lookup(identity, dialect)?;
        let query = dialect.authoring_query()?;
        let spec = self.get_for_surface(lookup.command, Some(query))?;
        if spec.owning_package().is_some() {
            return None;
        }
        match lookup.prepended {
            [] => spec.native_compilation,
            [member] => {
                let selected = spec.resolve_subcommand_for_dialect(member, Some(query))?;
                (selected.name == *member)
                    .then_some(selected.native_compilation)
                    .flatten()
                    .map(|mut compiler| {
                        if matches!(compiler.grammar, crate::native_compilation::NativeCompilationGrammar::StringMatch(_)) {
                            compiler.grammar = crate::native_compilation::NativeCompilationGrammar::StringMatch(crate::native_string_compilation::NativeStringMatchScope::PrivateOperands);
                        }
                        compiler.grammar = match compiler.grammar {
                            crate::native_compilation::NativeCompilationGrammar::StringTrim { operation, .. } => crate::native_compilation::NativeCompilationGrammar::StringTrim { scope: crate::native_scalar_compilation::NativeScalarScope::PrivateOperands, operation },
                            crate::native_compilation::NativeCompilationGrammar::StringEqual(_) => crate::native_compilation::NativeCompilationGrammar::StringEqual(crate::native_scalar_compilation::NativeScalarScope::PrivateOperands),
                            crate::native_compilation::NativeCompilationGrammar::StringLength(_) => crate::native_compilation::NativeCompilationGrammar::StringLength(crate::native_scalar_compilation::NativeScalarScope::PrivateOperands),
                            grammar => grammar,
                        };
                        compiler
                    })
            }
            [member, operation] => {
                let selected = spec.resolve_subcommand_for_dialect(member, Some(query))?;
                if selected.name != *member {
                    return None;
                }
                let operation_spec = selected.resolve_sub_subcommand_gated(
                    operation,
                    Some(query),
                    self.package_floor_for_spec(spec),
                )?;
                (operation_spec.name == *operation)
                    .then_some(operation_spec.native_compilation)
                    .flatten()
            }
            _ => None,
        }
    }

    /// Backend metadata selected from typed original native compiler words.
    /// This query neither proves admission nor grants normal handler effects.
    #[must_use]
    pub fn native_compiler_hooks(
        &self,
        words: crate::InvocationWords<'_>,
    ) -> Option<NativeCompilerHooks> {
        let dialect = words.arguments().dialect()?;
        if self
            .native_registration_lookup(words.head_literal()?, dialect)
            .is_some()
        {
            return self.with_native_registration(words, |projected| {
                self.native_compiler_hooks(projected)
            });
        }
        let query = dialect.authoring_query()?;
        let spec = self.get_for_surface(words.head_literal()?, Some(query))?;
        let (subcommand, sub) = resolve_semantic_subcommand(
            spec,
            words.arguments(),
            Some(query),
            self.package_floor_for_spec(spec),
            Some(dialect.numbers),
        );
        if !spec.subcommands.is_empty()
            && !words.arguments().is_empty()
            && sub.is_none()
            && subcommand != SubcommandResolution::NotApplicable
        {
            return None;
        }
        let form = if let Some(sub) = sub {
            pick_form(
                spec,
                Some(sub),
                words.arguments(),
                crate::resolved_invocation::InvocationAvailability {
                    query: Some(query),
                    package_version: self.package_floor_for_spec(spec),
                },
            )
        } else {
            pick_form(
                spec,
                None,
                words.arguments(),
                crate::resolved_invocation::InvocationAvailability {
                    query: Some(query),
                    package_version: self.package_floor_for_spec(spec),
                },
            )
        };
        Some(NativeCompilerHooks {
            lowering: form
                .and_then(|form| form.lowering_hook)
                .or(sub.and_then(|sub| sub.lowering_hook))
                .or(spec.lowering_hook),
            codegen: form
                .and_then(|form| form.codegen_hook)
                .or(sub.and_then(|sub| sub.codegen_hook))
                .or(spec.codegen_hook),
            inline: sub
                .and_then(|sub| sub.inline_codegen_hook)
                .or(spec.inline_codegen_hook),
        })
    }

    /// Logical member contract of an actual stock private registration.
    /// This metadata projection proves no live slot or implementation identity.
    #[must_use]
    pub fn native_registration_invocation_facts(
        &self,
        words: crate::InvocationWords<'_>,
    ) -> Option<crate::InvocationFacts> {
        self.native_registration_facts(words, false)
    }

    /// Inert selector operands used only by a private registration's backend
    /// metadata layout. These are never runtime arguments to that private slot.
    #[must_use]
    pub fn native_registration_operand_prefix(
        &self,
        identity: &str,
        dialect: crate::InvocationDialect,
    ) -> Option<&'static [&'static str]> {
        Some(
            self.native_registration_lookup(identity, dialect)?
                .prepended,
        )
    }

    /// Original selected descriptor of a stock registration's logical source.
    /// Private registrations use the canonical registry-authored parent lookup;
    /// an explicit descriptor at the identity retains its own provenance. This
    /// supplies metadata without a live token, argument layout or completion.
    #[must_use]
    pub fn native_registration_source_descriptor(
        &self,
        identity: &str,
        dialect: crate::InvocationDialect,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> Option<&'static CommandSpec> {
        let query = dialect.authoring_query()?.with_realm(realm);
        if let Some(spec) = self.get_for_surface(identity, Some(query)) {
            return Some(spec);
        }
        let lookup = self.native_registration_lookup(identity, dialect)?;
        self.get_for_surface(lookup.command, Some(query))
    }

    /// Intrinsic descriptors authored for one native registration. This is
    /// registration metadata, independent of invocation cardinality; the caller
    /// must separately prove the live stock token and any ensemble mapping.
    #[must_use]
    pub fn native_registration_intrinsic_ids(
        &self,
        identity: &str,
        dialect: crate::InvocationDialect,
    ) -> Option<Vec<crate::IntrinsicId>> {
        let query = dialect.authoring_query()?;
        let lookup = self.native_registration_lookup(identity, dialect);
        let (command, prefix) = lookup.map_or((identity, &[][..]), |lookup| {
            (lookup.command, lookup.prepended)
        });
        let spec = self.get_for_surface(command, Some(query))?;
        if spec.owning_package().is_some() {
            return None;
        }
        let mut ids = BTreeSet::new();
        let mut add = |semantic, lowering, codegen, inline| {
            if let Some(id) =
                crate::IntrinsicId::from_descriptors(semantic, lowering, codegen, inline)
            {
                ids.insert(id);
            }
        };
        if prefix.is_empty() {
            add(
                spec.semantic_operation,
                spec.lowering_hook,
                spec.codegen_hook,
                spec.inline_codegen_hook,
            );
            for form in spec.command_forms.iter().filter(|form| {
                form.surface
                    .is_none_or(|gate| surface_admits(gate, Some(&query)))
            }) {
                add(
                    form.semantic_operation,
                    form.lowering_hook,
                    form.codegen_hook,
                    None,
                );
            }
        }
        for member in spec.subcommands.iter().filter(|member| {
            prefix.first().is_none_or(|name| *name == member.name)
                && member
                    .surface
                    .is_none_or(|gate| surface_admits(gate, Some(&query)))
        }) {
            if prefix.len() <= 1 {
                add(
                    member.semantic_operation,
                    member.lowering_hook,
                    member.codegen_hook,
                    member.inline_codegen_hook,
                );
                for form in member.subcommand_forms.iter().filter(|form| {
                    form.surface
                        .is_none_or(|gate| surface_admits(gate, Some(&query)))
                }) {
                    add(
                        form.semantic_operation,
                        form.lowering_hook,
                        form.codegen_hook,
                        None,
                    );
                }
            }
        }
        Some(ids.into_iter().collect())
    }

    /// Normal-edge facts for the same actual private registration.
    #[must_use]
    pub fn native_registration_success_facts(
        &self,
        words: crate::InvocationWords<'_>,
    ) -> Option<crate::InvocationFacts> {
        self.native_registration_facts(words, true)
    }

    fn native_registration_facts(
        &self,
        words: crate::InvocationWords<'_>,
        successful: bool,
    ) -> Option<crate::InvocationFacts> {
        let dialect = words.arguments().dialect()?;
        let lookup = self.native_registration_lookup(words.head_literal()?, dialect)?;
        let mut facts = self.with_native_registration(words, |projected| {
            let resolved = self.resolve_structured_invocation(projected, dialect.authoring_query());
            let invocation = resolved.resolved()?;
            Some(if successful {
                invocation.facts_after_success()
            } else {
                invocation.facts()
            })
        })?;
        facts.argument_offset = facts.argument_offset.checked_sub(lookup.prepended.len())?;
        if let crate::state_transition::StateTransitionKnowledge::Declared(transitions) =
            &mut facts.state_transitions
        {
            *transitions = transitions
                .project_argument_indices(|index| index.checked_sub(lookup.prepended.len()))?;
        }
        if let Some(compilation) = &mut facts.native_compilation
            && let crate::native_compilation::NativeCompilationGrammar::Dictionary {
                ensemble, ..
            } = &mut compilation.grammar
        {
            *ensemble = false;
        }
        Some(facts)
    }

    /// Completion contract of an actual private registration in its physical frame.
    /// Unknown argument values and expansions remain typed during projection.
    #[must_use]
    pub fn native_registration_completion_route(
        &self,
        words: crate::InvocationWords<'_>,
        frame: crate::VariableAliasFrame,
    ) -> Option<crate::completion_route::InvocationCompletionRoute> {
        let dialect = words.arguments().dialect()?;
        self.with_native_registration(words, |projected| {
            self.invocation_completion_route_in_frame(
                projected.head_literal()?,
                projected.arguments(),
                dialect.authoring_query(),
                frame,
            )
        })
    }

    fn with_native_registration<T>(
        &self,
        words: crate::InvocationWords<'_>,
        project: impl FnOnce(crate::InvocationWords<'_>) -> Option<T>,
    ) -> Option<T> {
        let dialect = words.arguments().dialect()?;
        let lookup = self.native_registration_lookup(words.head_literal()?, dialect)?;
        let mut arguments = lookup
            .prepended
            .iter()
            .copied()
            .map(crate::InvocationWord::Literal)
            .collect::<Vec<_>>();
        arguments
            .extend((0..words.arguments().len()).filter_map(|index| words.arguments().get(index)));
        project(
            crate::InvocationWords::structured(
                crate::InvocationWord::Literal(lookup.command),
                &arguments,
            )
            .with_dialect(dialect),
        )
    }

    fn native_registration_lookup(
        &self,
        identity: &str,
        dialect: crate::InvocationDialect,
    ) -> Option<crate::native_compilation::NativeCompilerImplementationLookup> {
        let key = (
            dialect,
            identity.strip_prefix("::").unwrap_or(identity).to_owned(),
        );
        if let Some(cached) = self
            .native_registrations
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .lookups
            .get(&key)
        {
            return *cached;
        }
        let selected = self.native_registration_lookup_uncached(&key.1, dialect);
        self.native_registrations
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .lookups
            .insert(key, selected);
        selected
    }

    fn native_registration_lookup_uncached(
        &self,
        identity: &str,
        dialect: crate::InvocationDialect,
    ) -> Option<crate::native_compilation::NativeCompilerImplementationLookup> {
        let key = identity.strip_prefix("::").unwrap_or(identity);
        let query = dialect.authoring_query()?;
        let mut selected = None;
        for specs in self.by_name.values() {
            let Some(spec) = self.best_visible(specs, Some(query)) else {
                continue;
            };
            if spec.owning_package().is_some() {
                continue;
            }
            let compiled = spec
                .native_compilation
                .into_iter()
                .chain(spec.subcommands.iter().flat_map(|member| {
                    member.native_compilation.into_iter().chain(
                        member
                            .sub_subcommands
                            .iter()
                            .filter_map(|worker| worker.native_compilation),
                    )
                }))
                .flat_map(|compilation| {
                    compilation
                        .implementation_prerequisites(dialect)
                        .into_iter()
                        .flatten()
                });
            let normal = successful_handler_contracts(spec)
                .flat_map(|contract| contract.stock_native_workers(dialect));
            for lookup in compiled.chain(normal) {
                if lookup.slot.strip_prefix("::").unwrap_or(lookup.slot) != key {
                    continue;
                }
                if selected.is_some_and(|previous| previous != lookup) {
                    return None;
                }
                selected = Some(lookup);
            }
        }
        selected
    }

    /// Measured native admission for a catalogued command. A custom binding
    /// outside this catalogue has no stock-command admission answer.
    #[must_use]
    pub fn native_command_admission(
        &self,
        name: &str,
        dialect: crate::InvocationDialect,
    ) -> Option<bool> {
        self.get(name)?;
        Self::native_stock_registration_admission(dialect, name)
    }

    /// Measured admission for an actual stock-command registration, independent
    /// of catalogue ancestry. Unknown releases/builds remain unknown.
    #[must_use]
    pub fn native_stock_registration_admission(
        dialect: crate::InvocationDialect,
        name: &str,
    ) -> Option<bool> {
        // This is a stock implementation admission query. A live custom
        // native binding is not constrained by the fresh engine's roster.
        // Catalogue absence is not permission for an original stock identity.
        // naming.mathop-binding-and-written-head-controls
        // docs/design/analysis/name-resolution-proofs/mathop-binding-and-written-head-controls.md
        (dialect.core_point
            == Some(tcl_dialect::model::DialectPoint::canonical(
                tcl_dialect::model::Release::JIM_0_84,
            )))
        .then(|| {
            jim_fresh_command_names().contains(&tcl_syntax::naming::normalise_qualified_name(name))
        })
    }

    /// Discard derived command facts after a mutation to the command surface.
    fn invalidate_effective_semantics(&mut self) {
        self.effective_semantics = OnceLock::new();
        self.snapshot = OnceLock::new();
        self.native_registrations = Arc::default();
        self.generation = next_registry_generation();
    }

    /// Record the workspace pack overlay this registry carries; `0` is none.
    pub(crate) fn set_overlay(&mut self, overlay: u64) {
        self.overlay = (overlay != 0).then_some(overlay);
        self.invalidate_effective_semantics();
    }

    /// This registry's generation: the command surface an analysis resolved
    /// against, as the value-transfer context keys it. Every mutation draws
    /// a new one, and no two registries share one.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// The workspace pack overlay this registry was built with, as its
    /// content key (`PackSet::key`), or `None` for a registry with none.
    #[must_use]
    pub const fn overlay_generation(&self) -> Option<u64> {
        self.overlay
    }

    /// Return command names whose command-level descriptor selects `operation`
    /// at some release.
    ///
    /// This uses the same target-neutral descriptor precedence as structured
    /// invocation resolution, over the stamps a command carries plain and in
    /// every window. It is intended for whole-module trust proofs that need to
    /// quantify over every registry spelling of one semantic operation, at
    /// whichever release the module is compiled for.
    pub fn command_names_for_semantic_operation(
        &self,
        operation: crate::SemanticOperationId,
    ) -> impl Iterator<Item = &str> {
        self.by_name.iter().filter_map(move |(name, specs)| {
            specs
                .iter()
                .any(|spec| {
                    spec.descriptor_combinations()
                        .any(|(semantic, codegen, inline_codegen)| {
                            crate::resolved_invocation::descriptor_operation(
                                semantic,
                                spec.lowering_hook,
                                codegen,
                                inline_codegen,
                            ) == Some(operation)
                        })
                })
                .then_some(*name)
        })
    }

    /// The [`ObjectClassSpec`] for a `TclOO` / megawidget class named
    /// `class_name`, or `None` when it is not a registry-modelled class.
    ///
    /// For a `TclOO` class the class name is the factory command name
    /// (`oo::class create Foo` binds command `Foo`), so this resolves through
    /// the ordinary command table — no separate index.  A leading `::` falls
    /// back to the bare name, as with [`Self::get`].
    #[must_use]
    pub fn object_class(&self, class_name: &str) -> Option<&crate::spec::ObjectClassSpec> {
        self.get(class_name).and_then(|s| s.object_class)
    }

    /// The version floor this registry itself guarantees for `spec`'s owning
    /// package. A profile supplies its pinned runtime library version; a
    /// `SpecTcl` ambient-package declaration supplies the pack equivalent.
    /// When both make a claim, their strongest floor wins.
    ///
    /// Request-time consumers that have `package require` facts can raise this
    /// floor through `DocumentFloor`; registry-only lookup deliberately keeps
    /// the static, resolved-profile answer. `None` remains permissive for an
    /// unpinned package or a hand-assembled registry with no ambient claim.
    fn package_floor_for_spec(&self, spec: &CommandSpec) -> Option<&'static str> {
        self.package_floor(spec.owning_package()?)
    }

    /// The version floor this registry itself guarantees for `package`: the
    /// profile's pinned runtime library version and the highest a loaded
    /// pack declared with `ambient_package`, the stronger of the two when
    /// both are stated.
    ///
    /// The one place the two claims are combined: a spec's owning package
    /// ([`Self::package_floor_for_spec`]) and a package the registry's own
    /// point carries ([`Self::own_surface_query`]) ask it alike.
    fn package_floor(&self, package: &str) -> Option<&'static str> {
        let profile_floor = self
            .profile
            .and_then(|profile| profile.library_floor_default(package));
        let ambient_floor = self.ambient_package_floor(package);
        match (profile_floor, ambient_floor) {
            (Some(profile), Some(ambient)) => (crate::version::compare(profile, ambient).is_lt())
                .then_some(ambient)
                .or(Some(profile)),
            (profile, ambient) => profile.or(ambient),
        }
    }

    /// The stronger of the registry's own package floor and a caller's
    /// document-resolved floor. `package_version` is intentionally supplied
    /// by the caller rather than inferred from a command spelling: an explicit
    /// `package require` is document data, while the profile/ambient floor is
    /// registry data.
    fn package_floor_for_spec_at<'a>(
        &self,
        spec: &CommandSpec,
        package_version: Option<&'a str>,
    ) -> Option<&'a str> {
        match (self.package_floor_for_spec(spec), package_version) {
            (Some(registry), Some(document)) => {
                if crate::version::compare(registry, document).is_lt() {
                    Some(document)
                } else {
                    Some(registry)
                }
            }
            (Some(registry), None) => Some(registry),
            (None, document) => document,
        }
    }

    /// Assemble the visible instance-method table for `class_name`, walking
    /// declared superclasses breadth-first.
    ///
    /// The owning *class command* supplies the lifecycle axis for each row,
    /// rather than assuming every inherited method belongs to the receiver's
    /// package. This is the object-method counterpart of normal subcommand
    /// filtering: a Tk 9.1-only widget operation must not become a hover,
    /// semantic-token, taint, or completion fact in a profile that resolves
    /// Tk 9.0. The filter is entirely descriptor-driven, so the registry knows
    /// no widget or method spellings.
    #[must_use]
    pub fn instance_methods(&self, class_name: &str) -> Vec<&'static SubCommand> {
        self.instance_methods_at(class_name, None, self.own_surface_query())
    }

    /// [`Self::instance_methods`] with a document-resolved floor for the
    /// owning package and an explicit surface point.
    ///
    /// A request-time caller obtains `package_version` from its document's
    /// package-require facts for the class command, which can only raise the
    /// profile/ambient floor. Passing `None` retains the registry's static
    /// profile answer. `dialect` similarly lets an unprofiled, hand-assembled
    /// registry honour a caller's resolved command surface.
    #[must_use]
    pub fn instance_methods_at(
        &self,
        class_name: &str,
        package_version: Option<&str>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Vec<&'static SubCommand> {
        let Some(class_spec) = self.get_for_surface(class_name, dialect) else {
            return Vec::new();
        };
        self.instance_methods_for_descriptor_at(class_spec, package_version, dialect)
    }

    /// Visible method metadata from an independently selected factory descriptor.
    /// The descriptor must belong to this exact Registry and surface. This
    /// source projection supplies no instance allocation or runtime dispatch.
    #[must_use]
    pub fn instance_methods_for_descriptor_at(
        &self,
        class_spec: &CommandSpec,
        package_version: Option<&str>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Vec<&'static SubCommand> {
        // naming.source.original-registered-instance-words
        // docs/design/analysis/name-resolution-proofs/source-original-registered-instance-words.md
        if !self
            .get_for_surface(class_spec.name, dialect)
            .is_some_and(|selected| std::ptr::eq(selected, class_spec))
        {
            return Vec::new();
        }
        let mut queue = std::collections::VecDeque::from([class_spec]);
        let mut seen = std::collections::HashSet::new();
        let mut method_names = std::collections::HashSet::new();
        let mut visible_methods = Vec::new();
        while let Some(owner) = queue.pop_front() {
            if !seen.insert(owner.name) {
                continue;
            }
            let Some(class) = owner.object_class else {
                continue;
            };
            let package_floor = self.package_floor_for_spec_at(owner, package_version);
            for candidate in class.instance_methods {
                let method_dialects = candidate.surface.or(owner.surface);
                if !candidate.available_for_version(package_floor)
                    || !method_dialects.is_none_or(|gate| surface_admits(gate, dialect.as_ref()))
                {
                    continue;
                }
                if method_names.insert(candidate.name) {
                    visible_methods.push(candidate);
                }
            }
            for superclass in class.superclasses {
                if let Some(owner) = self.get_for_surface(superclass, dialect) {
                    queue.push_back(owner);
                }
            }
        }
        visible_methods
    }

    /// Resolve an instance method `method` on class `class_name`, walking
    /// declared superclasses breadth-first. Lifecycle-gated methods are
    /// filtered against their owning class command's resolved package floor.
    /// Returns the owning class's [`SubCommand`] method spec, or `None` when
    /// unresolved.
    #[must_use]
    pub fn instance_method(
        &self,
        class_name: &str,
        method: &str,
    ) -> Option<&crate::spec::SubCommand> {
        self.instance_method_at(class_name, method, None, self.own_surface_query())
    }

    /// [`Self::instance_method`] with a document-resolved owning-package
    /// floor and explicit surface point. See [`Self::instance_methods_at`]
    /// for the two axes' composition.
    #[must_use]
    pub fn instance_method_at(
        &self,
        class_name: &str,
        method: &str,
        package_version: Option<&str>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&crate::spec::SubCommand> {
        self.instance_method_for_descriptor_at(
            self.get_for_surface(class_name, dialect)?,
            method,
            package_version,
            dialect,
        )
    }

    /// Select an authored instance method from the same retained factory
    /// descriptor, with the shared lifecycle and prefix-ambiguity rules.
    #[must_use]
    pub fn instance_method_for_descriptor_at(
        &self,
        class_spec: &CommandSpec,
        method: &str,
        package_version: Option<&str>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&'static SubCommand> {
        let canonical = self
            .instance_method_selection_for_descriptor_at(
                class_spec,
                method,
                package_version,
                dialect,
            )?
            .unique()?;
        self.instance_methods_for_descriptor_at(class_spec, package_version, dialect)
            .into_iter()
            .find(|candidate| candidate.name == canonical)
    }

    /// Classify a source method spelling from its independently retained
    /// descriptor, selected lifecycle and shared abbreviation owner.
    #[must_use]
    pub fn instance_method_selection_for_descriptor_at(
        &self,
        class_spec: &CommandSpec,
        method: &str,
        package_version: Option<&str>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<crate::abbrev::KeywordMatch<'static>> {
        self.get_for_surface(class_spec.name, dialect)
            .is_some_and(|selected| std::ptr::eq(selected, class_spec))
            .then_some(())?;
        let methods = self.instance_methods_for_descriptor_at(class_spec, package_version, dialect);
        let matching = class_spec.object_class?.method_prefix_matching;
        Some(
            KeywordTable::from_keywords(
                methods.iter().map(|method| Keyword {
                    name: method.name,
                    min_abbrev: method.min_abbrev,
                }),
                matching,
            )
            .resolve(method),
        )
    }

    /// Resolve a concrete object-instance invocation to the same
    /// target-neutral semantic projection used for ordinary commands.
    ///
    /// `args` starts with the method word and is followed by its arguments.
    /// The class/factory command supplies the registry-owned method table, but
    /// constructor-only command traits and effects are deliberately not
    /// inherited by the instance call. A matching [`SubCommandForm`] may
    /// replace the method row's coarse traits, mutator flag, and legacy side
    /// effects for the concrete argument shape.
    ///
    /// [`SubCommandForm`]: crate::forms::SubCommandForm
    #[must_use]
    pub fn resolve_instance_invocation<'r, 'w>(
        &'r self,
        class_name: &str,
        receiver: &'w str,
        args: &'w [&'w str],
        dialect: Option<SurfaceQuery<'w>>,
    ) -> Option<ResolvedInvocation<'r, 'w>> {
        self.resolve_structured_instance_invocation(
            class_name,
            InvocationWords::literals(receiver, args),
            dialect,
        )
    }

    /// Source-aware companion to [`Self::resolve_instance_invocation`].
    ///
    /// The receiver is the invocation head. Only a literal method word can
    /// select the class method table. Form-level literal selectors likewise
    /// inspect only known literal words; a substituted operation word keeps
    /// the conservative method-row semantics rather than guessing a form.
    #[must_use]
    pub fn resolve_structured_instance_invocation<'r, 'w>(
        &'r self,
        class_name: &str,
        words: InvocationWords<'w>,
        dialect: Option<SurfaceQuery<'w>>,
    ) -> Option<ResolvedInvocation<'r, 'w>> {
        let class_spec = if dialect.is_none() {
            self.get(class_name)?
        } else {
            self.get_for_surface(class_name, dialect)?
        };
        if !self.effect_provider_is_admitted(class_spec, dialect) {
            return None;
        }
        self.resolve_structured_instance_invocation_for_descriptor(class_spec, words, None, dialect)
    }

    /// Source-aware instance schema from an independently selected factory
    /// descriptor and its exact original effective argv. The Registry identity,
    /// lifecycle and method prefix rules remain shared with the named API.
    #[must_use]
    pub fn resolve_structured_instance_invocation_for_descriptor<'r, 'w>(
        &'r self,
        class_spec: &'r CommandSpec,
        words: InvocationWords<'w>,
        package_version: Option<&'w str>,
        dialect: Option<SurfaceQuery<'w>>,
    ) -> Option<ResolvedInvocation<'r, 'w>> {
        // naming.source.original-registered-instance-words
        // docs/design/analysis/name-resolution-proofs/source-original-registered-instance-words.md
        let words = words.with_profile(self.profile());
        let arguments = words.arguments();
        let dialect = dialect.or_else(|| {
            arguments
                .dialect()
                .and_then(crate::InvocationDialect::authoring_query)
        });
        let method_spelling = arguments.literal_at(0)?;
        self.get_for_surface(class_spec.name, dialect)
            .is_some_and(|selected| std::ptr::eq(selected, class_spec))
            .then_some(())?;
        class_spec.object_class?;
        // This is the selected source schema. Package activation, allocation
        // and actual method execution are independent of its readonly roles.
        let method = self.instance_method_for_descriptor_at(
            class_spec,
            method_spelling,
            package_version,
            dialect,
        )?;
        let availability = crate::resolved_invocation::InvocationAvailability {
            query: dialect.or_else(|| {
                words
                    .arguments()
                    .dialect()
                    .and_then(crate::InvocationDialect::authoring_query)
            }),
            package_version: self.package_floor_for_spec_at(class_spec, package_version),
        };
        let form = pick_form(class_spec, Some(method), arguments, availability);
        let resolved_method = ResolvedSubcommand {
            spelling: method_spelling,
            canonical_name: method.name,
        };
        let subcommand = if method_spelling == method.name {
            SubcommandResolution::Exact(resolved_method)
        } else {
            SubcommandResolution::UniquePrefix(resolved_method)
        };
        Some(ResolvedInvocation::new_instance(
            words,
            class_spec,
            method,
            form,
            subcommand,
            availability,
        ))
    }

    /// Resolve the [`ArgRole::CommandPrefix`] positions and appended arities for
    /// an instance-method dispatch `$obj method method_args…`.
    ///
    /// `method_args` are the words *after* the method name.  Mirrors the
    /// subcommand arm of [`Self::command_prefixes`] (static
    /// [`SubCommand::command_prefixes`] table ∪ `command_prefix_resolver` ∪
    /// command-prefix options), but keyed on the object's class + method rather
    /// than a top-level command — so `$g walk … -command cb` (option value) and
    /// `$t walkproc … cb` (trailing positional, resolver) light up the same
    /// references / call-graph / W123 / arity substrate.  Returned indices are
    /// relative to `method_args` (0 = first word after the method name).
    ///
    /// [`SubCommand::command_prefixes`]: crate::spec::SubCommand::command_prefixes
    #[must_use]
    pub fn instance_method_command_prefixes(
        &self,
        class_name: &str,
        method: &str,
        method_args: &[&str],
    ) -> Vec<(usize, AppendedArity)> {
        self.instance_method_command_prefixes_with_arguments(
            class_name,
            method,
            CommandPrefixArguments::literals(method_args),
        )
    }

    /// Source-aware companion to [`Self::instance_method_command_prefixes`].
    /// Literal-sensitive resolvers can distinguish runtime substitutions from
    /// values proved at the call site.
    #[must_use]
    pub fn instance_method_command_prefixes_structured<'w>(
        &self,
        class_name: &str,
        method: &str,
        spellings: &'w [&'w str],
        words: &'w [InvocationWord<'w>],
    ) -> Vec<(usize, AppendedArity)> {
        let Some(arguments) = CommandPrefixArguments::structured(spellings, words) else {
            return Vec::new();
        };
        self.instance_method_command_prefixes_with_arguments(class_name, method, arguments)
    }

    fn instance_method_command_prefixes_with_arguments(
        &self,
        class_name: &str,
        method: &str,
        method_args: CommandPrefixArguments<'_>,
    ) -> Vec<(usize, AppendedArity)> {
        self.instance_method(class_name, method)
            .map_or_else(Vec::new, |m| sub_command_prefixes(m, method_args))
    }

    /// Whether `pkg` is a package the registry knows about — i.e. at
    /// least one registered command declares it as its
    /// [`required_package`](crate::CommandSpec::required_package).
    ///
    /// Used by the W120 (missing-`package require`) check: a `package
    /// require` of an *unknown* third-party package may itself pull in
    /// arbitrary commands (e.g. a wrapper that `package require Tk`s
    /// internally), so the analyser cannot prove a Tk/extension command
    /// is unprovided and must suppress W120.
    #[must_use]
    pub fn provides_package(&self, pkg: &str) -> bool {
        self.by_name
            .values()
            .flat_map(|specs| specs.iter())
            .any(|spec| spec.required_package == Some(pkg))
    }

    /// Return every registered [`CommandSpec`] for `name` (all dialects),
    /// in registration order. Empty when the name is unknown.
    ///
    /// This is the raw data view — resolution (which spec *wins* under a
    /// dialect) goes through [`Self::get_for_surface`]'s most-specific
    /// rule.
    #[must_use]
    pub fn specs(&self, name: &str) -> &[&'static CommandSpec] {
        self.by_name.get(name).map_or(&[], Vec::as_slice)
    }

    /// The taint-source colour declared by `command`'s spec, or `None`
    /// when it is not a source.
    ///
    /// Reads the compile-time [`TAINT_SOURCE_INDEX`] — a table *derived*
    /// from every command spec's [`crate::CommandSpec::taint_source`],
    /// built at compile time. It is deliberately **dialect-agnostic and
    /// independent of which dialects are loaded into this registry**:
    /// an iRules
    /// getter such as `HTTP::path` is a known source even when analysing a
    /// `tcl8.6` document whose registry never loaded the iRules commands.
    #[must_use]
    pub fn taint_source(&self, command: &str) -> Option<crate::taint::TaintColour> {
        TAINT_SOURCE_INDEX
            .iter()
            .find(|(name, _)| *name == command)
            .map(|(_, colour)| *colour)
    }

    /// Return the names of `command`'s subcommands whose traits include
    /// `t` — the subcommand-level counterpart of [`Self::commands_with_trait`].
    /// Empty when the command is unknown or has no matching subcommand.
    #[must_use]
    pub fn subcommands_with_trait(&self, command: &str, t: Traits) -> Vec<&str> {
        self.get(command).map_or_else(Vec::new, |spec| {
            spec.subcommands
                .iter()
                .filter(|s| s.traits.contains(t))
                .map(|s| s.name)
                .collect()
        })
    }

    /// Return the sorted names of every f5-irules command valid in `event`.
    ///
    /// The valid-command set: a command
    /// is valid when it supports the iRules dialect, the event is not in its
    /// `excluded_events`, and either it carries no `event_requires` or the
    /// event's [`crate::events::EventProps`] satisfy them.  Returns an empty
    /// vector for an unknown event.
    #[must_use]
    pub fn valid_irules_commands_for_event<'a>(
        &'a self,
        event: &str,
        events: &crate::events::EventRegistry,
        profiles: &crate::profiles::ProfileRegistry,
        bigip_version: Option<&str>,
    ) -> Vec<&'a str> {
        let Some(props) = events.get_props(event) else {
            return Vec::new();
        };
        // The declared version range for an F5-surface spec: explicit
        // introduction/removal data, or the axis baseline (15.0) for a
        // spec with none. `bigip_version: None` keeps the pre-version
        // behaviour (no filtering) so digest-stable callers opt in.
        let version_ok = |spec: &CommandSpec| {
            let Some(version) = bigip_version else {
                return true;
            };
            spec.lifecycle
                .with_baseline(tcl_dialect::VersionKey::BigipVersion.baseline_version())
                .available_at(Some(version))
        };
        let mut names: Vec<&str> = self
            .by_name
            .iter()
            .filter_map(|(name, specs)| {
                // Best spec for the dialect — the §5.3 most-specific rule,
                // matching `get_for_surface`.
                let spec =
                    self.best_visible(specs, Some(SurfaceQuery::any_release(Family::F5Irules)))?;
                if !version_ok(spec) {
                    return None;
                }
                if spec.excluded_events.contains(&event) {
                    return None;
                }
                if let Some(req) = spec.event_requires.as_ref()
                    && !crate::events::event_satisfies(props, req, event, profiles)
                {
                    return None;
                }
                Some(*name)
            })
            .collect();
        names.sort_unstable();
        names
    }

    /// Whether `command` is legal in iRules `event` — the O(1) legality-matrix
    /// test.
    ///
    /// A command is legal when the event is known, the command supports the
    /// iRules dialect, the event is not in the command's `excluded_events`,
    /// and the command's [`crate::events::EventRequires`] (if any) are
    /// satisfied by the event's [`crate::events::EventProps`].  An unknown
    /// event is illegal for every command — an event with no props has an
    /// empty valid-command set.
    #[must_use]
    pub fn is_irules_command_legal_in_event(
        &self,
        command: &str,
        event: &str,
        events: &crate::events::EventRegistry,
        profiles: &crate::profiles::ProfileRegistry,
    ) -> bool {
        self.is_irules_call_legal_in_event(command, &[], event, events, profiles)
    }

    /// Whether a concrete iRules command call is legal in `event`.
    ///
    /// Unlike [`Self::is_irules_command_legal_in_event`], this resolves the
    /// command's registry-declared argument-prefix event forms before applying
    /// the common event matrix. Consumers pass the words after the command
    /// name; no consumer needs command-specific subcommand knowledge.
    #[must_use]
    pub fn is_irules_call_legal_in_event(
        &self,
        command: &str,
        args: &[&str],
        event: &str,
        events: &crate::events::EventRegistry,
        profiles: &crate::profiles::ProfileRegistry,
    ) -> bool {
        let Some(props) = events.get_props(event) else {
            return false;
        };
        let Some(spec) =
            self.get_for_surface(command, Some(SurfaceQuery::any_release(Family::F5Irules)))
        else {
            return false;
        };
        if spec.excluded_events.contains(&event) {
            return false;
        }
        let requirements = spec.event_requirements_for_args(args);
        if !requirements.only_in.is_empty() && !requirements.only_in.contains(&event) {
            return false;
        }
        requirements
            .requires
            .is_none_or(|req| crate::events::event_satisfies(props, req, event, profiles))
    }

    /// Sorted iRules events where `command` is legal — the inverse of
    /// [`Self::valid_irules_commands_for_event`].  Used by the IRULE1001
    /// "Available in: …" hint.
    #[must_use]
    pub fn irules_events_for_command<'a>(
        &self,
        command: &str,
        events: &'a crate::events::EventRegistry,
        profiles: &crate::profiles::ProfileRegistry,
    ) -> Vec<&'a str> {
        self.irules_events_for_call(command, &[], events, profiles)
    }

    /// Sorted iRules events where a concrete command call is legal. See
    /// [`Self::is_irules_call_legal_in_event`] for the argument-form-aware
    /// contract resolution.
    #[must_use]
    pub fn irules_events_for_call<'a>(
        &self,
        command: &str,
        args: &[&str],
        events: &'a crate::events::EventRegistry,
        profiles: &crate::profiles::ProfileRegistry,
    ) -> Vec<&'a str> {
        let mut names: Vec<&str> = events
            .all_event_names()
            .into_iter()
            .filter(|event| {
                self.is_irules_call_legal_in_event(command, args, event, events, profiles)
            })
            .collect();
        names.sort_unstable();
        names
    }

    /// Resolve full metadata for an iRules `event` — the engine behind
    /// `f5 irule event-info`.
    ///
    /// The event name is upper-cased and trimmed. `known` /
    /// `valid_commands` come from this registry (the cross-product is the
    /// same as [`Self::valid_irules_commands_for_event`]); the side /
    /// transport / implied-profiles / description / multiplicity come from
    /// `events`. `deprecated` is always `false` — the `when`-argument-value
    /// detail path carries no "deprecated" markers (independent of
    /// [`crate::events::EventProps::deprecated`]).
    #[must_use]
    pub fn event_info(
        &self,
        event: &str,
        events: &crate::events::EventRegistry,
        profiles: &crate::profiles::ProfileRegistry,
        bigip_version: Option<&str>,
    ) -> EventInfo {
        let name = event.trim().to_uppercase();
        let target = bigip_version
            .or(tcl_dialect::VersionKey::BigipVersion.default_version())
            .unwrap_or("16.1.0");
        let known =
            !name.is_empty() && events.is_known(&name) && events.event_available_at(&name, target);
        let valid_commands: Vec<String> = if known {
            self.valid_irules_commands_for_event(&name, events, profiles, Some(target))
                .into_iter()
                .map(ToOwned::to_owned)
                .collect()
        } else {
            Vec::new()
        };
        let props = events.get_props(&name);
        let lifecycle = events.event_lifecycle(&name).unwrap_or_default();
        EventInfo {
            lifecycle,
            lifecycle_state: lifecycle.state_at(Some(target)),
            known,
            multiplicity: events.multiplicity(&name),
            description: events.description(&name).unwrap_or("").to_owned(),
            side: props.map_or("unknown", crate::events::EventProps::side_label),
            // "No transport" is modelled as `None`, not an empty string.
            transport: props.and_then(|p| (!p.transport.is_empty()).then(|| p.transport.join("/"))),
            implied_profiles: {
                let mut v: Vec<&'static str> = props
                    .map(|p| p.implied_profiles.to_vec())
                    .unwrap_or_default();
                v.sort_unstable();
                v
            },
            event: name,
            valid_commands,
        }
    }

    /// The symbol-definer descriptor for `name` in `dialect`, if the command
    /// binds a navigable definition name (a `tcltest::test` case, …).
    ///
    /// A leading `::` falls back to the bare name, as with [`Self::get`].  The
    /// analyser and signature scanner consult this to record outline symbols
    /// generically — the argument index and outline category come from the
    /// [`crate::symbol_def::SymbolDef`], never from a command-name check.
    #[must_use]
    pub fn defines_symbol(
        &self,
        name: &str,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&crate::symbol_def::SymbolDef> {
        self.get_for_surface(name, dialect)
            .and_then(|s| s.defines_symbol.as_ref())
    }

    /// Every command name that declares a [`crate::symbol_def::SymbolDef`] in
    /// any registered spec, for consumers (the signature scanner) that
    /// precompute a symbol-definer lookup set rather than querying per-call.
    #[must_use]
    pub fn commands_defining_symbols(&self) -> Vec<&str> {
        self.by_name
            .iter()
            .filter_map(|(name, specs)| {
                specs
                    .iter()
                    .any(|s| s.defines_symbol.is_some())
                    .then_some(*name)
            })
            .collect()
    }

    /// Return all command specs whose traits include `t`.
    #[must_use]
    pub fn commands_with_trait(&self, t: Traits) -> Vec<&str> {
        self.by_name
            .iter()
            .filter_map(|(name, specs)| {
                specs.last().filter(|s| s.traits.contains(t)).map(|_| *name)
            })
            .collect()
    }

    /// Every instance method some registered metaclass **generates** from a
    /// class's declared `property` members — the union of
    /// [`DefinitionBodyGrammar::property_accessor_methods`] over the
    /// metaclasses that configure by property
    /// ([`Traits::CONFIGURES_BY_PROPERTY`]).
    ///
    /// The dialect-wide answer, for a consumer holding a class whose own
    /// metaclass it could not resolve to a grammar — a user metaclass derived
    /// from `oo::configurable` — but which demonstrably declares `property`
    /// members. Prefer the class's own grammar
    /// (`CommandSpec::definition_body`) whenever it resolves; this is the
    /// fallback that keeps such a class from being told its generated
    /// accessor does not exist, without any consumer spelling
    /// `configure` itself.
    ///
    /// Sorted and deduplicated. Empty for a dialect with no such metaclass
    /// (every pre-9.0 Tcl, where `oo::configurable` does not exist).
    ///
    /// [`DefinitionBodyGrammar::property_accessor_methods`]: crate::definer::DefinitionBodyGrammar::property_accessor_methods
    #[must_use]
    pub fn property_accessor_methods(&self) -> Vec<&'static str> {
        let mut out: Vec<&'static str> = self
            .commands_with_trait(Traits::CONFIGURES_BY_PROPERTY)
            .into_iter()
            .filter_map(|name| self.get(name))
            .filter_map(|spec| spec.definition_body)
            .flat_map(|grammar| grammar.property_accessor_methods.iter().copied())
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// How a call to `name` binds a variable to an **object handle**, when it
    /// does — [`CommandSpec::binds_handle`], resolved through [`Self::get`] so
    /// the explicitly global spelling (`::set`) answers identically to the
    /// bare one.
    ///
    /// The member-body-only installers a class system injects (snit's
    /// `install NAME using TYPE …`) are **not** here: they are not global
    /// commands, and live on
    /// [`crate::definer::DefinitionBodyGrammar::member_body_commands`] —
    /// enumerate them with [`Self::member_body_handle_bindings`].
    ///
    /// [`CommandSpec::binds_handle`]: crate::spec::CommandSpec::binds_handle
    #[must_use]
    pub fn handle_binding(
        &self,
        name: &str,
    ) -> Option<&'static crate::handle_binding::HandleBindingSpec> {
        self.get(name).and_then(|spec| spec.binds_handle)
    }

    /// What role `name` plays in a **cross-language RPC family**, when it plays
    /// one — [`CommandSpec::remote_method`], resolved through [`Self::get`] so
    /// `::ILX::call` answers exactly as the bare spelling does.
    ///
    /// This is also the dialect gate for the whole relation: the ILX commands
    /// are `SpecSurface::IRULES` specs, so a registry built for stock Tcl holds
    /// no command of the name and this answers `None` — an ordinary Tcl
    /// document whose own `proc ILX::call` shadows nothing gets no
    /// remote-method navigation.
    ///
    /// [`CommandSpec::remote_method`]: crate::spec::CommandSpec::remote_method
    #[must_use]
    pub fn remote_method(
        &self,
        name: &str,
    ) -> Option<&'static crate::remote_method::RemoteMethodRole> {
        self.get(name).and_then(|spec| spec.remote_method)
    }

    /// Every command in this registry that takes part in a cross-language RPC
    /// family, by name.
    ///
    /// The membership test a consumer needs *before* it walks a document: if
    /// this is empty (every non-iRules registry), there is no relation to look
    /// for and the walk is skipped entirely; if it is not, a document that
    /// mentions none of the names cannot contain a site — a rename or an
    /// `interp alias` still has to spell the target once to create the binding.
    /// Enumerating them here is what keeps that gate out of the consumers,
    /// which would otherwise have to name `ILX::call` to ask the question.
    #[must_use]
    pub fn remote_method_commands(&self) -> Vec<&'static str> {
        let mut out: Vec<&'static str> = self
            .by_name
            .values()
            .filter_map(|specs| specs.last())
            .filter(|spec| spec.remote_method.is_some())
            .map(|spec| spec.name)
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Every member-body command that binds an object handle, as
    /// `(word, layout)` pairs collected from the definition-body grammars this
    /// registry's definers carry.
    ///
    /// These words exist only inside a class system's member bodies (snit's
    /// `install`), so they deliberately have no global `CommandSpec` — a
    /// consumer builds a small lookup from this list once per document instead
    /// of naming the keyword.  Deduplicated: the snit `type` and `widget`
    /// grammars share one member-body command set.
    #[must_use]
    pub fn member_body_handle_bindings(
        &self,
    ) -> Vec<(&'static str, crate::handle_binding::HandleBindingSpec)> {
        let mut out: Vec<_> = self
            .by_name
            .values()
            .filter_map(|specs| specs.last())
            .filter_map(|spec| spec.definition_body)
            .flat_map(|grammar| grammar.member_body_commands.iter())
            .filter_map(|cmd| cmd.binds_handle.map(|layout| (cmd.name, layout)))
            .collect();
        out.sort_unstable_by_key(|(name, _)| *name);
        out.dedup_by_key(|(name, _)| *name);
        out
    }

    /// Whether `name` **writes or modifies** the variable named by its
    /// first argument (`set` / `append` / `lappend` / `incr` / `lset`):
    /// [`Traits::FIRST_ARG_VARNAME`] minus the destroy-only `unset`
    /// ([`Traits::DESTROYS_VARIABLE`]).
    ///
    /// The single membership query for the write-command consumers —
    /// loop-bound checks, dead-store cancellation, embedded-script def
    /// collection, catch-body out-vars — rather than each keeping its own
    /// hardcoded (and easily inconsistent) name set.
    #[must_use]
    pub fn writes_first_arg_variable(&self, name: &str) -> bool {
        self.get(name).is_some_and(|s| {
            s.traits.contains(Traits::FIRST_ARG_VARNAME)
                && !s.traits.contains(Traits::DESTROYS_VARIABLE)
        })
    }

    /// Whether `name` **read-modify-writes** the variable named by its
    /// first argument (`append` / `lappend` / `incr` / `lset` — not the
    /// whole-value `set`): [`Traits::FIRST_ARG_VARNAME`] ∧
    /// [`Traits::READS_BEFORE_WRITE`].
    ///
    /// Drives the minifier's RMW target protection: a name-compaction
    /// must not rename a variable whose current value an RMW command is
    /// about to fold into, while a plain `set` target is rename-safe.
    #[must_use]
    pub fn rmw_first_arg_variable(&self, name: &str) -> bool {
        self.get(name).is_some_and(|s| {
            s.traits.contains(Traits::FIRST_ARG_VARNAME)
                && s.traits.contains(Traits::READS_BEFORE_WRITE)
        })
    }

    /// Whether `name` is a core Tcl built-in carrying the
    /// [`Traits::BYTE_COMPILED`] trait — i.e. the minifier must not
    /// rewrite this command head to a `$var` alias.  Checks every
    /// registered spec for the name (not just the dialect-preferred
    /// one) so the core stamp is honoured even when a dialect layers
    /// an additional spec under the same name.
    #[must_use]
    pub fn is_byte_compiled(&self, name: &str) -> bool {
        self.by_name.get(name).is_some_and(|specs| {
            specs
                .iter()
                .any(|s| s.traits.contains(Traits::BYTE_COMPILED))
        })
    }

    /// Whether `name` is unsafe in sandboxed dialects — it allows
    /// context escalation (`uplevel`, `history`).  Drives the IRULE2003
    /// "unsafe iRules command" check.  Checks every spec registered
    /// under the name.
    #[must_use]
    pub fn is_unsafe(&self, name: &str) -> bool {
        self.by_name
            .get(name)
            .is_some_and(|specs| specs.iter().any(|s| s.unsafe_command))
    }

    /// The [`crate::traits::UNIT_LINKAGE_TRAITS`] this concrete invocation
    /// carries — the registry's answer to "does this command widen the set
    /// of callers beyond the file it appears in?".
    ///
    /// `PROVIDES_PACKAGE` (`package provide` / `ifneeded`) and
    /// `EXPORTS_COMMAND` (`namespace export`, `namespace ensemble`) say the
    /// file publishes an API surface; `LOADS_EXTERNAL_UNIT` (`source`,
    /// `load`, `package require`, `auto_load`, `auto_import`, `namespace
    /// import`) says another unit's script runs in this interpreter and can
    /// call back in.  Any of the three sinks the "every caller of this
    /// file's procs is in this file" assumption that
    /// `tcl_compiler::unit_scope`'s interprocedural call-site seed rests on.
    ///
    /// Resolved through [`Self::resolve_call`], so the subcommand word is
    /// honoured (`package provide` is a boundary, `package names` is not)
    /// and `spec.traits | sub.traits` composes exactly as
    /// [`crate::spec::SubCommand::traits`] documents. An unknown command
    /// carries no linkage — a user proc named `source` is a user proc.
    #[must_use]
    pub fn unit_linkage(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Traits {
        self.invocation_traits(name, args, dialect)
            .intersection(crate::traits::UNIT_LINKAGE_TRAITS)
    }

    /// Every trait this concrete invocation carries — `spec.traits |
    /// sub.traits`, composed exactly as [`crate::spec::SubCommand::traits`]
    /// documents.
    ///
    /// The invocation-level counterpart of `self.get(name).traits`: a
    /// subcommand's traits are *additive* over its parent's, so a consumer
    /// asking a trait question about a compound command must compose them.
    /// `namespace eval` / `namespace inscope` / `interp eval` carry the
    /// eval-family traits ([`Traits::EVALUATES_CODE`],
    /// [`Traits::SCRIPT_CONCATENATES_ARGS`]) on the **subcommand**, not on
    /// the `namespace` / `interp` spec, so a parent-only trait test silently
    /// misses them.
    ///
    /// Resolved through [`Self::resolve_call`], so the subcommand word is
    /// honoured; pass `None` to skip dialect gating (the plain
    /// [`Self::get`] lookup) when the question is "what shape is this
    /// command" rather than "is it available here". An unknown command
    /// carries no traits.
    ///
    /// A pack's `stores` row states a write class as well
    /// ([`crate::value_transfer::DeclaredStores::write_class`]): a target
    /// declared `write_or_preserve` or `may_write` is a conditional write
    /// whether or not the pack spells the trait.
    #[must_use]
    pub fn invocation_traits(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Traits {
        let Some(resolved) = self.resolve_call(name, args, dialect) else {
            return Traits::empty();
        };
        resolved.spec.traits
            | resolved.sub.map_or_else(Traits::empty, |sub| sub.traits)
            | crate::value_transfer::resolve_semantics(resolved.spec, resolved.sub, resolved.form)
                .write_class()
    }

    /// The frame the invocation's scope-alias declaration links the locals
    /// its `VarWrite` operands name into — the global namespace for
    /// `global`, the current namespace for `variable` — where the resolved
    /// command declares one
    /// ([`crate::value_transfer::CommandSemantics::alias_frame`]).
    #[must_use]
    pub fn alias_frame(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<crate::value_transfer::AliasFrame> {
        let resolved = self.resolve_call(name, args, dialect)?;
        crate::value_transfer::resolve_semantics(resolved.spec, resolved.sub, resolved.form)
            .alias_frame()
    }

    /// When the script at 0-based argument `index` runs relative to this
    /// concrete invocation.
    ///
    /// Script-valued options carry an exact per-value [`ScriptTiming`] fact, so
    /// one invocation may safely mix immediate, deferred, and reference-only
    /// executable text. A command or subcommand [`ScriptTimingResolver`]
    /// supplies the same exact fact for positional forms whose timing depends
    /// on their written arguments.
    /// Invocation-wide [`Traits::DEFERS_BODY`] remains the compatibility
    /// fallback. `None` means `index` is not a supplied executable position.
    ///
    /// [`ScriptTiming`]: crate::hover::ScriptTiming
    /// [`ScriptTimingResolver`]: crate::spec::ScriptTimingResolver
    #[must_use]
    pub fn script_timing(
        &self,
        name: &str,
        args: &[&str],
        index: usize,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<crate::hover::ScriptTiming> {
        let resolved = self.resolve_invocation(name, args, dialect)?;
        let spec = if dialect.is_none() {
            self.get(resolved.canonical_command)?
        } else {
            self.get_for_surface(resolved.canonical_command, dialect)?
        };
        let sub = resolved
            .subcommand
            .resolved()
            .and_then(|subcommand| spec.subcommand(subcommand.canonical_name));
        if !spec.subcommands.is_empty()
            && sub.is_none()
            && resolved.subcommand != SubcommandResolution::NotApplicable
        {
            return None;
        }
        let prefix = self
            .command_prefixes(name, args)
            .iter()
            .any(|(at, _)| *at == index);
        Self::script_timing_for_resolved(&resolved, spec, sub, args, index, prefix)
    }

    /// Script timing of an explicitly selected instance method. The caller
    /// supplies its receiver class; this metadata query does not prove the
    /// receiver's physical class or live command identity.
    #[must_use]
    pub fn instance_script_timing(
        &self,
        class_name: &str,
        receiver: &str,
        args: &[&str],
        index: usize,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<crate::hover::ScriptTiming> {
        let resolved = self.resolve_instance_invocation(class_name, receiver, args, dialect)?;
        let spec =
            self.get_for_surface(class_name, dialect.or_else(|| self.own_surface_query()))?;
        let selected = resolved.subcommand.resolved()?;
        let method = self.instance_method_at(
            class_name,
            selected.canonical_name,
            self.package_floor_for_spec(spec),
            dialect.or_else(|| self.own_surface_query()),
        )?;
        let prefix = self
            .instance_method_command_prefixes(
                class_name,
                selected.canonical_name,
                args.get(1..).unwrap_or_default(),
            )
            .iter()
            .any(|(at, _)| at.checked_add(1) == Some(index));
        Self::script_timing_for_resolved(&resolved, spec, Some(method), args, index, prefix)
    }

    fn script_timing_for_resolved(
        resolved: &ResolvedInvocation<'_, '_>,
        spec: &CommandSpec,
        sub: Option<&SubCommand>,
        args: &[&str],
        index: usize,
        command_prefix: bool,
    ) -> Option<crate::hover::ScriptTiming> {
        let scan_start = resolved
            .semantics
            .argument_offset
            .checked_add(if sub.is_none() {
                spec.constructor_prefix_words().unwrap_or(0)
            } else {
                0
            })?;
        let mut roles: Vec<_> = resolved
            .argument_roles()
            .0
            .into_iter()
            .map(|(position, role)| {
                (
                    resolved.semantics.argument_offset + usize::from(position),
                    role,
                )
            })
            .collect();
        push_option_value_roles(
            &mut roles,
            &resolved.semantics.options.available().collect::<Vec<_>>(),
            args,
            scan_start,
            &|_| true,
            resolved.semantics.options.prefix_matching,
        );
        let is_executable = command_prefix
            || roles
                .iter()
                .any(|&(position, role)| role.has_script_timing() && position == index);
        if !is_executable {
            return None;
        }

        let (timing_resolver, offset) = sub.map_or((spec.script_timing_resolver, 0usize), |sub| {
            (sub.script_timing_resolver, 1usize)
        });
        if let Some(resolve) = timing_resolver
            && let Some(relative) = index.checked_sub(offset)
            && let Some((_, timing)) = resolve(crate::InvocationArguments::literals(
                args.get(offset..).unwrap_or(&[]),
            ))
            .into_iter()
            .find(|(position, _)| usize::from(*position) == relative)
        {
            return Some(timing);
        }

        // An invocation-sensitive resolver is the exact fact and therefore
        // precedes an option's static default. This matters for APIs such as
        // `bibtex::parse`: the same callback option runs synchronously for an
        // in-memory input but is stored when `-channel` selects background
        // parsing. Resolver silence deliberately falls through to the option.
        let options = resolved.semantics.options.available().collect::<Vec<_>>();
        if let Some(timing) = option_script_timing_at_selected(
            &options,
            args,
            scan_start,
            index,
            resolved.semantics.options.prefix_matching,
        ) {
            return Some(timing);
        }

        Some(if resolved.semantics.traits.contains(Traits::DEFERS_BODY) {
            crate::hover::ScriptTiming::Deferred
        } else {
            crate::hover::ScriptTiming::SameInvocation
        })
    }

    /// The 0-based argument positions of this invocation that hold a script,
    /// command prefix or lambda the command stores as a **callback**: to run
    /// after it returns, at the global level or in the frame of whatever
    /// fires it, where a plain variable name can be one the registering code
    /// holds.
    ///
    /// The positions [`Self::script_timing`] calls
    /// [`Deferred`](crate::hover::ScriptTiming::Deferred), less those of a
    /// command that stores a definition
    /// ([`Traits::BODY_RUNS_IN_OWN_FRAME`]): a `proc` body is dormant too, but
    /// runs in a frame of its own. A command that runs the script now
    /// ([`SameInvocation`](crate::hover::ScriptTiming::SameInvocation)) or
    /// only names a registration (`trace remove`,
    /// [`ReferenceOnly`](crate::hover::ScriptTiming::ReferenceOnly)) has none.
    /// Indices count as [`Self::script_timing`] counts them, the resolved
    /// subcommand word included.
    #[must_use]
    pub fn callback_script_indices(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Vec<usize> {
        if self
            .invocation_traits(name, args, dialect)
            .contains(Traits::BODY_RUNS_IN_OWN_FRAME)
        {
            return Vec::new();
        }
        (0..args.len())
            .filter(|&index| {
                self.script_timing(name, args, index, dialect)
                    == Some(crate::hover::ScriptTiming::Deferred)
            })
            .collect()
    }

    /// User-controlled values that this concrete deferred callback host
    /// substitutes into argument `index` before Tcl evaluates it.
    ///
    /// This is deliberately a registry query rather than a Tk-specific
    /// compiler table. It unifies positional callback bodies (`bind`) and
    /// option values (`entry -validatecommand`), and only returns a non-empty
    /// list when the selected executable position is declared to receive an
    /// external value. Framework metadata is not inferred as taint.
    #[must_use]
    pub fn callback_taint_inputs(
        &self,
        name: &str,
        args: &[&str],
        index: usize,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> &'static [CallbackTaintInput] {
        let Some(resolved) = self.resolve_invocation(name, args, dialect) else {
            return &[];
        };
        let Some(spec) = (if dialect.is_none() {
            self.get(resolved.canonical_command)
        } else {
            self.get_for_surface(resolved.canonical_command, dialect)
        }) else {
            return &[];
        };
        let sub = resolved
            .subcommand
            .resolved()
            .and_then(|subcommand| spec.subcommand(subcommand.canonical_name));
        let (table, offset) = sub.map_or((spec.callback_taint_inputs, 0usize), |sub| {
            (sub.callback_taint_inputs, 1usize)
        });
        let options = resolved.semantics.options.base;
        let scan_start = resolved.semantics.argument_offset;
        if let Some(inputs) = option_callback_taint_inputs_at(options, args, scan_start, index) {
            return if option_script_timing_at(options, args, scan_start, index)
                == Some(crate::hover::ScriptTiming::Deferred)
            {
                inputs
            } else {
                &[]
            };
        }
        if self
            .script_timing(name, args, index, dialect)
            .is_none_or(|timing| timing != crate::hover::ScriptTiming::Deferred)
        {
            return &[];
        }
        let Some(relative) = index.checked_sub(offset) else {
            return &[];
        };
        table
            .iter()
            .find_map(|(at, inputs)| (usize::from(*at) == relative).then_some(*inputs))
            .unwrap_or(&[])
    }

    /// Callback inputs selected from evaluated word knowledge. An unknown
    /// captured value preserves its argv slot; it is never an empty literal.
    /// Positional callbacks use the complete structured role selection. Option
    /// callbacks retain the existing exact layout query when all values are
    /// available; unresolved option layout abstains.
    #[must_use]
    pub fn callback_taint_inputs_words(
        &self,
        words: InvocationWords<'_>,
        index: usize,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> &'static [CallbackTaintInput] {
        let words = words.with_profile(self.profile());
        let Some(name) = words.head_literal() else {
            return &[];
        };
        let arguments = words.arguments();
        if !arguments.has_exact_argv_len() {
            return &[];
        }
        let Some(resolved) = self
            .resolve_structured_invocation(words, dialect)
            .resolved()
        else {
            return &[];
        };
        let Some(spec) = (if dialect.is_none() {
            self.get(resolved.canonical_command)
        } else {
            self.get_for_surface(resolved.canonical_command, dialect)
        }) else {
            return &[];
        };
        if resolved
            .argument_count_for_arity()
            .is_none_or(|count| !resolved.semantics.arity.accepts(count))
        {
            return &[];
        }
        let (roles, complete) = resolved.argument_roles();
        let positional_body = complete
            && roles.iter().any(|(at, role)| {
                resolved.semantics.argument_offset + usize::from(*at) == index
                    && *role == ArgRole::Body
            });
        let sub = resolved
            .subcommand
            .resolved()
            .and_then(|subcommand| spec.subcommand(subcommand.canonical_name));
        let timing_resolver = sub.map_or(spec.script_timing_resolver, |sub| {
            sub.script_timing_resolver
        });
        if positional_body
            && resolved.semantics.traits.contains(Traits::DEFERS_BODY)
            && timing_resolver.is_none()
            && resolved.semantics.options.base.is_empty()
        {
            let (table, offset) = sub.map_or((spec.callback_taint_inputs, 0usize), |sub| {
                (sub.callback_taint_inputs, 1usize)
            });
            if let Some(relative) = index.checked_sub(offset)
                && let Some((_, inputs)) = table.iter().find(|(at, _)| usize::from(*at) == relative)
            {
                return inputs;
            }
        }
        let Some(literals) = (0..arguments.len())
            .map(|at| arguments.literal_at(at))
            .collect::<Option<Vec<_>>>()
        else {
            return &[];
        };
        self.callback_taint_inputs(name, &literals, index, dialect)
    }

    /// User-controlled values that a resolved instance method substitutes
    /// into argument `index` before evaluating its deferred callback.
    ///
    /// `args` starts with the instance method word. The query is the
    /// object-dispatch counterpart of [`Self::callback_taint_inputs`]: it
    /// resolves the receiver class and method through the same registry table,
    /// then consults the method's positional callback table or its declared
    /// options. A method form carrying [`Traits::CONFIGURES_INSTANCE_OPTIONS`]
    /// inherits the owning class's option table, which is how widget
    /// `configure` setters expose constructor options without a command-name
    /// branch. Unknown/dynamic methods and non-deferred positions abstain.
    #[must_use]
    pub fn instance_callback_taint_inputs(
        &self,
        class_name: &str,
        receiver: &str,
        args: &[&str],
        index: usize,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> &'static [CallbackTaintInput] {
        let Some(resolved) = self.resolve_instance_invocation(class_name, receiver, args, dialect)
        else {
            return &[];
        };
        let Some(method_spelling) = args.first() else {
            return &[];
        };
        let Some(method) = self.instance_method(class_name, method_spelling) else {
            return &[];
        };
        let Some(relative) = index.checked_sub(1) else {
            return &[];
        };

        let options = resolved.semantics.options.available().collect::<Vec<_>>();
        let matching = resolved.semantics.options.prefix_matching;
        let option_inputs =
            option_callback_taint_inputs_at_selected(&options, args, 1, index, matching);
        if let Some(inputs) = option_inputs {
            return if option_script_timing_at_selected(&options, args, 1, index, matching)
                == Some(crate::hover::ScriptTiming::Deferred)
            {
                inputs
            } else {
                &[]
            };
        }

        let timing = method
            .script_timing_resolver
            .and_then(|resolve| {
                resolve(crate::InvocationArguments::literals(
                    args.get(1..).unwrap_or(&[]),
                ))
                .into_iter()
                .find_map(|(at, timing)| (usize::from(at) == relative).then_some(timing))
            })
            .unwrap_or_else(|| {
                if resolved.semantics.traits.contains(Traits::DEFERS_BODY) {
                    crate::hover::ScriptTiming::Deferred
                } else {
                    crate::hover::ScriptTiming::SameInvocation
                }
            });
        if timing != crate::hover::ScriptTiming::Deferred {
            return &[];
        }
        method
            .callback_taint_inputs
            .iter()
            .find_map(|(at, inputs)| (usize::from(*at) == relative).then_some(*inputs))
            .unwrap_or(&[])
    }

    /// Deferred inputs for an explicitly selected instance with frozen argv.
    /// Unknown value bytes retain their operand slots; unknown method/option
    /// selection or expansion supplies no callback input contract.
    #[must_use]
    pub fn instance_callback_taint_inputs_words(
        &self,
        class_name: &str,
        words: InvocationWords<'_>,
        index: usize,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> &'static [CallbackTaintInput] {
        let words = words.with_profile(self.profile());
        let Some(resolved) =
            self.resolve_structured_instance_invocation(class_name, words, dialect)
        else {
            return &[];
        };
        if resolved
            .argument_count_for_arity()
            .is_none_or(|count| !resolved.semantics.arity.accepts(count))
        {
            return &[];
        }
        let Some(selected) = resolved.subcommand.resolved() else {
            return &[];
        };
        let query = dialect.or_else(|| {
            words
                .arguments()
                .dialect()
                .and_then(crate::InvocationDialect::authoring_query)
        });
        let Some(spec) = self.get_for_surface(class_name, query) else {
            return &[];
        };
        let Some(method) = self.instance_method_at(
            class_name,
            selected.canonical_name,
            self.package_floor_for_spec(spec),
            query,
        ) else {
            return &[];
        };
        let (roles, complete) = resolved.argument_roles();
        if complete
            && method.script_timing_resolver.is_none()
            && resolved.semantics.options.base.is_empty()
            && resolved.semantics.traits.contains(Traits::DEFERS_BODY)
            && roles.iter().any(|(at, role)| {
                *role == ArgRole::Body
                    && usize::from(*at) + resolved.semantics.argument_offset == index
            })
        {
            return index
                .checked_sub(resolved.semantics.argument_offset)
                .and_then(|relative| {
                    method
                        .callback_taint_inputs
                        .iter()
                        .find_map(|(at, inputs)| (usize::from(*at) == relative).then_some(*inputs))
                })
                .unwrap_or_default();
        }
        let Some(args) = (0..words.arguments().len())
            .map(|at| words.arguments().literal_at(at))
            .collect::<Option<Vec<_>>>()
        else {
            return &[];
        };
        self.instance_callback_taint_inputs(
            class_name,
            words.head_literal().unwrap_or_default(),
            &args,
            index,
            dialect,
        )
    }

    /// Variable frame of the option value at 0-based argument `index`.
    ///
    /// `None` means that `index` is not a variable-name option value. The
    /// returned scope is registry data and therefore applies uniformly to
    /// lowering, SSA, and taint analysis without command-name branches.
    #[must_use]
    pub fn option_variable_scope(
        &self,
        name: &str,
        args: &[&str],
        index: usize,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<crate::hover::VariableScope> {
        let words = crate::InvocationWords::literals(name, args).with_profile(self.profile());
        self.resolve_structured_invocation(words, dialect)
            .resolved()?
            .authored_source_option_variable_scope_at(index)
    }

    /// Classify one body argument of a typed control-flow invocation.
    /// Dynamic clause grammar stays here beside the registry hook, so a
    /// consumer never recognises a command or structural keyword by name.
    #[must_use]
    pub fn control_arm_semantics(
        &self,
        name: &str,
        args: &[&str],
        body_index: usize,
    ) -> Option<ControlArmSemantics> {
        use crate::hooks::LoweringHookId;
        let resolved = self.resolve_call(name, args, None)?;
        match resolved.lowering_hook? {
            LoweringHookId::If => {
                if !control_chain_is_well_formed(resolved.spec, args, None)? {
                    return None;
                }
                self.arg_indices_for_role(name, args, ArgRole::Body)
                    .contains(&body_index)
                    .then_some(ControlArmSemantics::Selected)
            }
            LoweringHookId::Switch => Some(ControlArmSemantics::Selected),
            LoweringHookId::NamespaceEval => Some(ControlArmSemantics::FrameBoundary),
            // `apply`'s lambda runs **now**, in a fresh procedure frame that
            // inherits none of the caller's locals — a frame boundary, and the
            // only [`ArgRole::LambdaLiteral`] position in the registry. The
            // role is one level removed from a body (the
            // argument is the `{argList body ?ns?}` *list*), so the index is
            // gated on the role rather than assumed, exactly as the `if` arm
            // above gates on `ArgRole::Body`.
            //
            // The boundary is not symmetric with `namespace eval`'s on one
            // point, and a consumer reading completion out of this descriptor
            // should know it: a `return` inside the lambda completes the
            // *lambda*, so control does reach the statement after the `apply`,
            // whereas `namespace eval`'s script propagates its `return` to the
            // enclosing procedure (tclsh 8.6.16 / 9.0.4 agree on both). Errors
            // propagate from either. The compiler's fall-through walk does not
            // separate an early *normal* completion from an abnormal one, so
            // it reads a lambda that returns as one it cannot walk past,
            // which is conservative and sound.
            LoweringHookId::Apply => self
                .arg_indices_for_role(name, args, ArgRole::LambdaLiteral)
                .contains(&body_index)
                .then_some(ControlArmSemantics::FrameBoundary),
            LoweringHookId::Catch => Some(ControlArmSemantics::CompletionBoundary),
            // A collection loop iterates a value the invocation itself
            // supplies, so it runs a *finite* number of times: it terminates
            // whenever its body does. A condition loop's trip count depends on
            // state this descriptor cannot read, so it stays uncertain.
            LoweringHookId::Foreach | LoweringHookId::Lmap | LoweringHookId::ForeachLine => {
                Some(ControlArmSemantics::BoundedIteration)
            }
            LoweringHookId::For | LoweringHookId::While => Some(ControlArmSemantics::Uncertain),
            LoweringHookId::Try => try_control_arms(
                &resolved.clause_plan(args, None)?,
                args,
                tcl_syntax::number::Numbers::of_profile(self.profile()),
            )?
            .into_iter()
            .find_map(|(idx, semantics)| (idx == body_index).then_some(semantics)),
            _ => None,
        }
    }

    /// Whether a typed control invocation's complete grammar is valid.
    /// `None` means the command has no typed control grammar.
    #[must_use]
    pub fn control_invocation_valid(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<bool> {
        use crate::hooks::LoweringHookId;
        let resolved = self.resolve_call(name, args, dialect)?;
        let hook = resolved.lowering_hook?;
        match hook {
            LoweringHookId::If => control_chain_is_well_formed(resolved.spec, args, dialect),
            LoweringHookId::Switch => Some(self.case_invocation(name, args, dialect).is_some()),
            LoweringHookId::Try => Some(
                resolved
                    .clause_plan(args, dialect)
                    .and_then(|plan| try_control_arms(&plan, args, self.control_numbers(dialect)))
                    .is_some(),
            ),
            LoweringHookId::NamespaceEval
            | LoweringHookId::Catch
            | LoweringHookId::For
            | LoweringHookId::While
            | LoweringHookId::Foreach
            | LoweringHookId::Lmap
            | LoweringHookId::ForeachLine => Some(true),
            _ => None,
        }
    }

    /// Parse the complete registry-owned `try` clause grammar and return its
    /// typed word layout. `None` means the command does not resolve to the
    /// profile's `try` lowering hook or the invocation is malformed.
    #[must_use]
    pub fn try_control_invocation(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<TryControlInvocation> {
        let resolved = self.resolve_call(name, args, dialect)?;
        if resolved.lowering_hook != Some(crate::hooks::LoweringHookId::Try) {
            return None;
        }
        parse_try_control_invocation(
            &resolved.clause_plan(args, dialect)?,
            args,
            self.control_numbers(dialect),
            self.return_invocation_grammar(crate::InvocationArguments::literals(&[]), dialect)
                .completion_code_policy(self.control_release(dialect)),
        )
    }

    /// Numeral grammar for a control invocation query.
    ///
    /// An explicit surface point is authoritative even when this registry is
    /// profile-less (the ordinary cross-dialect command universe). The
    /// nearest core point with one exact grammar answers; a query none of
    /// whose points has one abstains through
    /// [`tcl_syntax::number::Numbers::Unknown`]. Only an absent query falls
    /// back to the registry's attached profile; a profile-less catalogue is
    /// not evidence for any interpreter's control grammar.
    fn control_numbers(&self, dialect: Option<SurfaceQuery<'_>>) -> tcl_syntax::number::Numbers {
        use tcl_syntax::number::Numbers;

        let Some(query) = dialect else {
            return self.profile().map_or(Numbers::Unknown, |profile| {
                Numbers::of_profile(Some(profile))
            });
        };
        query
            .core
            .iter()
            .find_map(|(family, release)| point_number_syntax(family, release))
            .map_or(Numbers::Unknown, Numbers::Target)
    }

    /// A control handler's release is independent of its source numeral
    /// grammar. An explicit query with no release cannot borrow the registry
    /// profile's handler width.
    fn control_release(
        &self,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<tcl_dialect::TclVersion> {
        match dialect {
            Some(query) => query.core.iter().find_map(|(family, release)| {
                if family == Family::Tcl
                    && let Some(release) = release
                {
                    return tcl_dialect::TclVersion::from_package_version(release);
                }
                let version = tcl_dialect::model::DialectPoint::tcl_version_of_release;
                match release {
                    Some(spelling) => family
                        .releases()
                        .iter()
                        .find(|release| release.as_str() == spelling)
                        .and_then(|&release| version(release)),
                    None => {
                        let mut versions = family.releases().iter().copied().map(version);
                        let first = versions.next()?;
                        first.filter(|&first| versions.all(|next| next == Some(first)))
                    }
                }
            }),
            None => self.runtime_version(),
        }
    }

    /// Parse a case-list invocation using only options available in this
    /// registry profile. This is the dialect-aware entry point for consumers:
    /// the descriptor owns the layout, while [`crate::ProfileQueries`] owns
    /// option availability and value arity.
    #[must_use]
    pub fn case_invocation(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<(crate::spec::CaseListSpec, crate::spec::CaseInvocation)> {
        let resolved = self.resolve_call(name, args, dialect)?;
        let case = resolved.spec.case_list?;
        let options = self.profile().map_or_else(
            || resolved.spec.option_specs(dialect),
            |profile| {
                crate::profile_queries::ProfileQueries::available_option_specs(
                    profile,
                    resolved.spec,
                )
            },
        );
        let effective_dialect = self.own_surface_query().or(dialect);
        Some((*case, case.invocation(args, &options, effective_dialect)?))
    }

    /// Classify whether a concrete invocation falls through, returns a normal
    /// procedure result, or terminates with another completion code.
    #[must_use]
    pub fn invocation_completion(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> InvocationCompletion {
        self.invocation_completion_words(name, InvocationArguments::literals(args), dialect)
    }

    /// Classify a selected source invocation without interpreting substituted
    /// text as values. Pending returns are settled through the shared owner.
    #[must_use]
    pub fn invocation_completion_words(
        &self,
        name: &str,
        args: InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> InvocationCompletion {
        let Some(resolved) = self
            .resolve_structured_invocation(
                InvocationWords::from_arguments(InvocationWord::Literal(name), args),
                dialect,
            )
            .resolved()
        else {
            return InvocationCompletion::Unknown;
        };
        let numbers = args.dialect().map_or_else(
            || self.control_numbers(dialect),
            |native| tcl_syntax::number::Numbers::Target(native.numbers),
        );
        invocation_completion_for_selected(
            &resolved,
            numbers,
            self.return_invocation_grammar(args, dialect),
        )
    }

    /// What a `return` with `args` completes with, decoded as this
    /// registry's release reads its options
    /// ([`crate::value_transfer::completion::decode_return_words`]): the one
    /// reading of `return`'s options the lowering, the CFG, the solver's
    /// route and the completion queries here share.
    #[must_use]
    pub fn return_completion(&self, args: InvocationArguments<'_>) -> ReturnDecoding {
        crate::value_transfer::completion::decode_return_words(self.profile(), args)
    }

    /// Return an exact Tcl completion code (or process exit) for a concrete,
    /// literal invocation when registry data and return options establish one.
    ///
    /// This is intentionally narrower than [`Self::invocation_completion`]:
    /// dynamic `return -options $dict`, dynamic `-code`/`-level` values, and
    /// ordinary calls with only a conservative completion descriptor return
    /// `None` rather than inventing a handler edge.
    #[must_use]
    pub fn exact_invocation_completion(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<ExactInvocationCompletion> {
        self.exact_invocation_completion_words(
            name,
            crate::invocation_words::InvocationArguments::literals(args),
            dialect,
        )
    }

    /// Source-aware counterpart to [`Self::exact_invocation_completion`].
    /// Dynamic words remain dynamic; literal brace words retain their literal
    /// value rather than being mistaken for a variable spelling.
    #[must_use]
    pub fn exact_invocation_completion_words(
        &self,
        name: &str,
        args: crate::invocation_words::InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<ExactInvocationCompletion> {
        let dialect = dialect.or_else(|| {
            args.dialect()
                .and_then(crate::InvocationDialect::authoring_query)
        });
        // An expansion can alter the final argv shape.  A descriptor for a
        // well-formed call (notably `exit`) must not override that unknown
        // arity, because an expanded call may instead raise TCL_ERROR.
        args.exact_argv_len()?;
        let resolved = self
            .resolve_structured_invocation(
                InvocationWords::from_arguments(InvocationWord::Literal(name), args),
                dialect,
            )
            .resolved()?;
        let arity = resolved
            .semantics
            .procedure_definition
            .map_or(resolved.semantics.arity, |descriptor| {
                descriptor.arity(args.dialect())
            });
        if !arity.accepts(resolved.argument_count_for_arity()?) {
            // Signature rejection precedes the handler's declared effect.
            return Some(ExactInvocationCompletion::Tcl(
                crate::completion::CompletionCode::Error,
            ));
        }
        if resolved.semantics.operation
            == crate::SemanticOperationId::StructuredLowering(LoweringHookId::Return)
        {
            let numbers = args.dialect().map_or_else(
                || self.control_numbers(dialect),
                |dialect| tcl_syntax::number::Numbers::Target(dialect.numbers),
            );
            return match exact_return_completion(
                args,
                numbers,
                self.return_invocation_grammar(args, dialect),
            ) {
                ExactReturnCompletion::Completion(code) => {
                    code.immediate_code().map(ExactInvocationCompletion::Tcl)
                }
                ExactReturnCompletion::StaticError => Some(ExactInvocationCompletion::Tcl(
                    crate::completion::CompletionCode::Error,
                )),
                ExactReturnCompletion::Dynamic => None,
            };
        }
        let descriptor = resolved.semantics.completion;
        if descriptor.value_semantics
            == crate::completion::CompletionValueSemantics::ProcessExitStatus
        {
            return match exact_process_exit_completion(
                args,
                args.dialect().map_or_else(
                    || {
                        self.profile()
                            .map_or(tcl_syntax::number::Numbers::Unknown, |profile| {
                                tcl_syntax::number::Numbers::of_profile(Some(profile))
                            })
                    },
                    |dialect| tcl_syntax::number::Numbers::Target(dialect.numbers),
                ),
                args.dialect().map_or_else(
                    || {
                        self.profile()
                            .and_then(tcl_dialect::DialectProfile::process_exit_conversion)
                    },
                    |dialect| dialect.process_exit_conversion,
                ),
            ) {
                ExactProcessExitCompletion::ProcessExit => Some(
                    if self.return_invocation_grammar(args, dialect)
                        == crate::completion_route::ReturnInvocationGrammar::Jim
                    {
                        ExactInvocationCompletion::Tcl(crate::completion::CompletionCode::Other(6))
                    } else {
                        ExactInvocationCompletion::ProcessExit
                    },
                ),
                ExactProcessExitCompletion::StaticError => Some(ExactInvocationCompletion::Tcl(
                    crate::completion::CompletionCode::Error,
                )),
                ExactProcessExitCompletion::Dynamic => None,
            };
        }
        if resolved
            .semantics
            .traits
            .contains(Traits::TERMINATES_PROCESS)
        {
            return Some(ExactInvocationCompletion::ProcessExit);
        }
        let crate::completion::CompletionCodeDomain::Exact(codes) = descriptor.codes else {
            return None;
        };
        (codes.len() == 1).then_some(ExactInvocationCompletion::Tcl(codes[0]))
    }

    fn return_invocation_grammar(
        &self,
        args: crate::invocation_words::InvocationArguments<'_>,
        query: Option<SurfaceQuery<'_>>,
    ) -> crate::completion_route::ReturnInvocationGrammar {
        use crate::completion_route::ReturnInvocationGrammar as Grammar;
        if let Some(dialect) = args.dialect() {
            return Grammar::for_dialect(dialect);
        }
        match query {
            Some(query) => Grammar::for_family_release(
                query.core.nearest().map(|(family, _)| family),
                self.control_release(Some(query)),
            ),
            None => self.profile().map_or(Grammar::Unknown, |profile| {
                Grammar::for_dialect(crate::InvocationDialect::of_profile(profile))
            }),
        }
    }

    /// Resolve completion without losing the eventual code and unwind level of
    /// a pending return. Procedure callers reduce the route at their boundary;
    /// namespace and evaluation frames retain it. Missing descriptors remain
    /// explicit unknown routes rather than promising normal continuation.
    ///
    /// ```
    /// use tcl_registry::{CommandRegistry, InvocationArguments};
    /// let registry = CommandRegistry::build_default();
    /// let route = registry.invocation_completion_route("return", InvocationArguments::literals(&[]), None);
    /// assert!(route.is_some_and(|route| !route.normal_possible()));
    /// ```
    #[must_use]
    pub fn invocation_completion_route(
        &self,
        name: &str,
        args: crate::invocation_words::InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<crate::completion_route::InvocationCompletionRoute> {
        use crate::completion_route::InvocationCompletionRoute as Route;
        let resolved = self
            .resolve_structured_invocation(
                InvocationWords::from_arguments(InvocationWord::Literal(name), args),
                dialect,
            )
            .resolved()?;
        let descriptor = resolved.semantics.completion;
        if descriptor.value_semantics == crate::completion::CompletionValueSemantics::Tailcall {
            return Some(native_tailcall_completion(args, None));
        }
        let knowledge = self.invocation_completion_knowledge(name, args, dialect);
        if resolved.semantics.operation
            == crate::SemanticOperationId::StructuredLowering(LoweringHookId::Return)
            && !matches!(
                knowledge,
                Some(InvocationCompletionKnowledge::Exact(
                    ExactInvocationCompletion::Tcl(crate::completion::CompletionCode::Error)
                ))
            )
            && args.exact_argv_len().is_some()
        {
            let numbers = args.dialect().map_or_else(
                || self.control_numbers(dialect),
                |context| tcl_syntax::number::Numbers::Target(context.numbers),
            );
            if let ExactReturnCompletion::Completion(route) = exact_return_completion(
                args,
                numbers,
                self.return_invocation_grammar(args, dialect),
            ) {
                return Some(route);
            }
        }
        Some(match knowledge {
            Some(InvocationCompletionKnowledge::Exact(ExactInvocationCompletion::Tcl(code))) => {
                Route::Tcl(code)
            }
            Some(InvocationCompletionKnowledge::Exact(ExactInvocationCompletion::ProcessExit)) => {
                Route::ProcessExit
            }
            Some(InvocationCompletionKnowledge::DynamicReturnOrError) => Route::ReturnOrError,
            Some(InvocationCompletionKnowledge::ExitOrError) => Route::ExitOrError,
            Some(InvocationCompletionKnowledge::CatchableExitOrError) => {
                Route::CatchableExitOrError
            }
            Some(InvocationCompletionKnowledge::Dynamic) | None => match descriptor.codes {
                crate::completion::CompletionCodeDomain::Exact(codes) => {
                    Route::TclAlternatives(codes)
                }
                crate::completion::CompletionCodeDomain::Any => Route::Unknown,
            },
        })
    }

    /// Refine a selected native completion using the actual shared activation
    /// class. A catalogue or namespace spelling is not an activation proof.
    #[must_use]
    pub fn invocation_completion_route_in_frame(
        &self,
        name: &str,
        args: crate::InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
        frame: crate::VariableAliasFrame,
    ) -> Option<crate::completion_route::InvocationCompletionRoute> {
        let resolved = self
            .resolve_structured_invocation(
                InvocationWords::from_arguments(InvocationWord::Literal(name), args),
                dialect,
            )
            .resolved()?;
        let descriptor = resolved.semantics.completion;
        if descriptor.value_semantics == crate::completion::CompletionValueSemantics::Tailcall {
            return Some(native_tailcall_completion(args, Some(frame)));
        }
        self.invocation_completion_route(name, args, dialect)
    }

    /// Return source-aware completion knowledge without exposing return-option
    /// parsing to a consumer such as the diagram renderer.
    #[must_use]
    pub fn invocation_completion_knowledge(
        &self,
        name: &str,
        args: crate::invocation_words::InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<InvocationCompletionKnowledge> {
        if let Some(exact) = self.exact_invocation_completion_words(name, args, dialect) {
            return Some(InvocationCompletionKnowledge::Exact(exact));
        }
        let resolved = self
            .resolve_structured_invocation(
                InvocationWords::from_arguments(InvocationWord::Literal(name), args),
                dialect,
            )
            .resolved()?;
        let descriptor = resolved.semantics.completion;
        if descriptor.value_semantics
            == crate::completion::CompletionValueSemantics::ProcessExitStatus
        {
            // The exact path above already returned omitted / literal-valid
            // process exits and literal-invalid TCL_ERROR. Anything left is
            // source-dynamic (including `{*}` with an unknown argv length):
            // Tcl can only either parse a status and exit or reject the
            // invocation; it can never fall through normally.
            return Some(
                if self.return_invocation_grammar(args, dialect)
                    == crate::completion_route::ReturnInvocationGrammar::Jim
                {
                    InvocationCompletionKnowledge::CatchableExitOrError
                } else {
                    InvocationCompletionKnowledge::ExitOrError
                },
            );
        }
        if resolved.semantics.operation
            != crate::SemanticOperationId::StructuredLowering(LoweringHookId::Return)
        {
            return None;
        }
        Some(if return_has_only_dynamic_code(args) {
            InvocationCompletionKnowledge::DynamicReturnOrError
        } else {
            InvocationCompletionKnowledge::Dynamic
        })
    }

    /// Whether `name` is valid only at the top level of an iRule script
    /// (`when`, `proc`, `priority`, `timing`).  Drives the IRULE5006 /
    /// IRULE5007 placement checks ([`Traits::IRULES_TOP_LEVEL_ONLY`]).
    #[must_use]
    pub fn is_irules_top_level_only(&self, name: &str) -> bool {
        self.get(name)
            .is_some_and(|spec| spec.traits.contains(Traits::IRULES_TOP_LEVEL_ONLY))
    }

    /// Apply the iRules command-placement contract to one resolved head.
    ///
    /// The only legal file-level forms carry
    /// [`Traits::IRULES_TOP_LEVEL_ONLY`]. Every other invocation, including a
    /// user-defined command absent from this registry, is executable and must
    /// occur inside an event or procedure. An invalid nested top-level body is
    /// not cascaded: its enclosing executable command already carries the
    /// placement finding, while declaration forms nested there remain
    /// independently reportable.
    #[must_use]
    pub fn irules_command_placement(
        &self,
        name: &str,
        context: crate::events::IrulesExecutionContext,
    ) -> crate::events::IrulesCommandPlacement {
        use crate::events::{IrulesCommandPlacement, IrulesExecutionContext};

        if self.is_irules_top_level_only(name) {
            return if context == IrulesExecutionContext::TopLevel {
                IrulesCommandPlacement::Allowed
            } else {
                IrulesCommandPlacement::RequiresTopLevel
            };
        }
        match context {
            IrulesExecutionContext::TopLevel => IrulesCommandPlacement::RequiresEventOrProcedure,
            IrulesExecutionContext::EventBody
            | IrulesExecutionContext::ProcedureBody
            | IrulesExecutionContext::InvalidNestedBody => IrulesCommandPlacement::Allowed,
        }
    }

    /// Validate and classify one invocation on iRules' top-level declaration
    /// surface. Arity, argument layout, declaration traits, and known-event
    /// membership are decided here so compiler/tooling consumers cannot admit
    /// different executable regions.
    #[must_use]
    pub fn irules_top_level_declaration(
        &self,
        name: &str,
        arguments: crate::events::IrulesDeclarationArguments<'_>,
        events: &crate::events::EventRegistry,
    ) -> Option<crate::events::IrulesTopLevelDeclaration> {
        use crate::events::IrulesTopLevelDeclaration as Declaration;

        let declaration = self.irules_top_level_declaration_shape(name, arguments)?;
        match &declaration {
            Declaration::Event { event, .. } if !events.is_known(event) => None,
            Declaration::Event { .. }
            | Declaration::Procedure { .. }
            | Declaration::Priority { .. }
            | Declaration::Timing { .. } => Some(declaration),
        }
    }

    /// Validate an iRules file-level declaration without requiring an event
    /// selector to be known.
    ///
    /// This is the source-aware declaration owner used by lowering, symbols,
    /// diagnostics, and inventories. It deliberately accepts an unknown event
    /// with a valid declaration shape so diagnostics can report that selector;
    /// executable inventories use [`Self::irules_top_level_declaration`],
    /// which additionally requires the event registry membership.
    #[must_use]
    pub fn irules_top_level_declaration_shape(
        &self,
        name: &str,
        arguments: crate::events::IrulesDeclarationArguments<'_>,
    ) -> Option<crate::events::IrulesTopLevelDeclaration> {
        use crate::events::IrulesTopLevelDeclaration as Declaration;

        let args = arguments.values();
        let spec = self.get(name)?;
        let argument_count = u16::try_from(args.len()).ok()?;
        if !spec.arity.accepts(argument_count)
            || self.irules_command_placement(name, crate::events::IrulesExecutionContext::TopLevel)
                != crate::events::IrulesCommandPlacement::Allowed
        {
            return None;
        }
        if let Some(effect) = self.irules_top_level_effect(name, args) {
            return Some(effect);
        }
        let bodies = self.arg_indices_for_role(name, args, crate::ArgRole::Body);
        let [body_index] = bodies.as_slice() else {
            return None;
        };
        if !arguments.is_braced_literal(*body_index) {
            return None;
        }
        if spec.traits.contains(crate::Traits::IS_EVENT_HANDLER) {
            return self.irules_event_declaration_shape(arguments);
        }
        if spec.traits.contains(crate::Traits::DEFINES_PROCEDURE) {
            let names = self.arg_indices_for_role(name, args, crate::ArgRole::Name);
            let [name_index] = names.as_slice() else {
                return None;
            };
            return Some(Declaration::Procedure {
                name_index: *name_index,
                body_index: *body_index,
            });
        }
        None
    }

    /// Parse a stateful iRules file-level declaration through its spec-owned
    /// effect descriptor.
    #[must_use]
    pub fn irules_top_level_effect(
        &self,
        name: &str,
        args: &[&str],
    ) -> Option<crate::events::IrulesTopLevelDeclaration> {
        use crate::events::IrulesTopLevelDeclaration as Declaration;

        let spec = self.get(name)?;
        let argument_count = u16::try_from(args.len()).ok()?;
        if !spec.arity.accepts(argument_count) {
            return None;
        }
        match spec.irules_top_level_effect {
            Some(crate::events::IrulesTopLevelEffect::Priority) => {
                let [value] = args else { return None };
                let value = value.parse::<u16>().ok()?;
                crate::events::BIGIP_EVENT_HANDLER_PRIORITY
                    .accepts(value)
                    .then_some(Declaration::Priority { value })
            }
            Some(crate::events::IrulesTopLevelEffect::Timing) => {
                let [value] = args else { return None };
                let enabled = match *value {
                    "on" | "enable" => true,
                    "off" | "disable" => false,
                    _ => return None,
                };
                Some(Declaration::Timing { enabled })
            }
            None => None,
        }
    }

    /// Validate the shared iRules event-handler argument grammar independent
    /// of its resolved registry spelling.
    #[must_use]
    pub fn irules_event_declaration(
        &self,
        arguments: crate::events::IrulesDeclarationArguments<'_>,
        events: &crate::events::EventRegistry,
    ) -> Option<crate::events::IrulesTopLevelDeclaration> {
        let declaration = self.irules_event_declaration_shape(arguments)?;
        let crate::events::IrulesTopLevelDeclaration::Event { event, .. } = &declaration else {
            return None;
        };
        events.is_known(event).then_some(declaration)
    }

    /// Validate event-handler layout without requiring the event name to be
    /// known. Diagnostic consumers use this to report an otherwise valid
    /// top-level declaration whose event selector is unknown.
    #[must_use]
    pub fn irules_event_declaration_shape(
        &self,
        arguments: crate::events::IrulesDeclarationArguments<'_>,
    ) -> Option<crate::events::IrulesTopLevelDeclaration> {
        use crate::events::IrulesTopLevelDeclaration as Declaration;
        let args = arguments.values();
        let (body_index, priority) = match args {
            [_, _] => (1, None),
            [_, "priority", value, _] => {
                let priority = value.parse::<u16>().ok().filter(|priority| {
                    crate::events::BIGIP_EVENT_HANDLER_PRIORITY.accepts(*priority)
                })?;
                (3, Some(priority))
            }
            [_, "timing", value, _] => {
                matches!(*value, "on" | "off" | "enable" | "disable").then_some((3, None))?
            }
            [_, "priority", priority, "timing", timing, _] => {
                let priority = priority.parse::<u16>().ok().filter(|priority| {
                    crate::events::BIGIP_EVENT_HANDLER_PRIORITY.accepts(*priority)
                })?;
                matches!(*timing, "on" | "off" | "enable" | "disable")
                    .then_some((5, Some(priority)))?
            }
            _ => return None,
        };
        if !arguments.is_braced_literal(body_index) {
            return None;
        }
        let event = args.first()?.to_uppercase();
        Some(Declaration::Event {
            event,
            body_index,
            priority,
        })
    }

    /// Whether `name` should appear as a notable action node in a flow
    /// diagram ([`Traits::DIAGRAM_ACTION`]). Accepts both the bare
    /// (`HTTP::respond`) and the canonical (`::HTTP::respond`) spelling —
    /// the leading `::` stamped on `Statement::Call.canonical_command` by
    /// lowering is stripped to recover the bare registration form — and
    /// reflects the dialects loaded into this registry (the diagram-action
    /// set is part of the per-registry trait index, so a
    /// `--dialect f5-irules` registry recognises iRules actions).
    #[must_use]
    pub fn is_diagram_action(&self, name: &str) -> bool {
        let has = |n: &str| {
            self.by_name.get(n).is_some_and(|specs| {
                specs
                    .iter()
                    .any(|s| s.traits.contains(Traits::DIAGRAM_ACTION))
            })
        };
        has(name) || name.strip_prefix("::").is_some_and(has)
    }

    /// Whether `name` is explicitly marked **never** translatable to F5
    /// Distributed Cloud (XC) — any registered spec whose
    /// [`CommandSpec::xc_translatable`](crate::CommandSpec) is
    /// `Some(false)`. Consumed by the `f5-xc` iRule→XC translator.
    #[must_use]
    pub fn is_xc_never_translatable(&self, name: &str) -> bool {
        self.by_name
            .get(name)
            .is_some_and(|specs| specs.iter().any(|s| s.xc_translatable == Some(false)))
    }

    /// Whether `name` is explicitly marked translatable to XC despite an
    /// otherwise-untranslatable namespace prefix — any registered spec
    /// whose [`CommandSpec::xc_translatable`](crate::CommandSpec) is
    /// `Some(true)`.
    #[must_use]
    pub fn is_xc_translatable_override(&self, name: &str) -> bool {
        self.by_name
            .get(name)
            .is_some_and(|specs| specs.iter().any(|s| s.xc_translatable == Some(true)))
    }

    /// Whether `name` carries the [`Traits::NOT_PROC_FACTORY`] trait —
    /// a registered command head that incidentally matches the
    /// proc-factory token shape but is not a factory wrapper.  Like
    /// [`Self::is_byte_compiled`], checks every spec registered under
    /// the name.
    #[must_use]
    pub fn is_not_proc_factory(&self, name: &str) -> bool {
        self.by_name.get(name).is_some_and(|specs| {
            specs
                .iter()
                .any(|s| s.traits.contains(Traits::NOT_PROC_FACTORY))
        })
    }

    /// Whether `name` is an iRules side-switch — `clientside`,
    /// `serverside`, or `peer` — i.e. a command that evaluates its
    /// nesting-script body under a different connection-side context
    /// than the surrounding event ([`Traits::IS_SIDE_SWITCH`]).
    /// Consulted by the iRules collect/release/payload flow check when
    /// it descends into a side-switch body.  Like
    /// [`Self::is_byte_compiled`], checks every spec registered under
    /// the name.
    #[must_use]
    pub fn is_side_switch(&self, name: &str) -> bool {
        self.by_name.get(name).is_some_and(|specs| {
            specs
                .iter()
                .any(|s| s.traits.contains(Traits::IS_SIDE_SWITCH))
        })
    }

    /// Return the side context selected by `name`'s nesting-script body.
    ///
    /// This is more precise than [`Self::is_side_switch`]: callers that need
    /// to recurse into a body use the declared fixed/client/server/peer
    /// behaviour instead of matching a command spelling.
    #[must_use]
    pub fn side_switch_target(&self, name: &str) -> Option<SideSwitchTarget> {
        self.get(name)?.side_switch_target
    }

    /// Return the data-collection lifecycle fact declared by `name`.
    ///
    /// This is the only lookup consumers need for collect/release/payload
    /// analysis: command files opt in explicitly, so a similarly named custom
    /// command is never mistaken for an iRules buffer operation.
    #[must_use]
    pub fn data_collection_operation(&self, name: &str) -> Option<DataCollectionOperation> {
        self.get(name)?.data_collection
    }

    /// Return the declared protocol facts for `protocol`.
    ///
    /// A protocol can be known even when it has no `collect` command (UDP and
    /// ASM payloads are immediately available), so this scans all declared
    /// lifecycle operations rather than only collect commands.
    #[must_use]
    pub fn data_collection_protocol(&self, protocol: &str) -> Option<DataCollectionProtocol> {
        self.by_name
            .values()
            .flatten()
            .filter_map(|spec| spec.data_collection)
            .find(|operation| operation.protocol.name.eq_ignore_ascii_case(protocol))
            .map(|operation| operation.protocol)
    }

    /// Return the registered collect command for `protocol`, if it has one.
    ///
    /// Editor quick fixes use this to offer only commands that really exist;
    /// for example, UDP payload access deliberately returns `None` because
    /// BIG-IP provides each datagram without a `UDP::collect` command.
    #[must_use]
    pub fn data_collection_collect_command(&self, protocol: &str) -> Option<&CommandSpec> {
        self.by_name.values().flatten().copied().find(|spec| {
            spec.data_collection.is_some_and(|operation| {
                operation.action == DataCollectionAction::Collect
                    && operation.protocol.name.eq_ignore_ascii_case(protocol)
            })
        })
    }

    /// Return the event-handler priority policy declared by `name`.
    ///
    /// BIG-IP's `when` handler has a runtime default priority, so the policy
    /// records that omission is valid. Consumers can still honour a stricter
    /// policy for a dialect whose handler spec opts into one.
    #[must_use]
    pub fn event_handler_priority(&self, name: &str) -> Option<EventHandlerPriority> {
        self.get(name)?.event_handler_priority
    }

    /// Return the command spec that declares an event-handler priority grammar.
    ///
    /// Code generators and editor fixes use this when they need to create a
    /// handler rather than inspect an existing command. This keeps both the
    /// handler spelling and its priority grammar in the registry. Returns
    /// `None` if the dialect has no handler grammar or more than one, because
    /// choosing between multiple handlers requires additional registry data.
    #[must_use]
    pub fn event_handler_spec(&self) -> Option<&CommandSpec> {
        let mut handlers = self
            .by_name
            .values()
            .flatten()
            .filter(|spec| spec.event_handler_priority.is_some());
        let handler = handlers.next()?;
        handlers.next().is_none().then_some(handler)
    }

    /// Whether `name` is **frame-sensitive**: its meaning depends on the
    /// frame or scope it executes in, so moving a call across a proc
    /// boundary (inlining it into a caller) changes behaviour.  The union
    /// of the block-unwinding ([`Traits::TERMINATES_BLOCK`]),
    /// control-transferring ([`Traits::TRANSFERS_CONTROL`]),
    /// scope-aliasing ([`Traits::CREATES_SCOPE_ALIAS`]), and
    /// barrier-creating ([`Traits::CREATES_BARRIER`]) traits.  Like
    /// [`Self::is_byte_compiled`], checks every spec registered under the
    /// name.  Drives the inline-proc code action's safety decline and the
    /// negative half of [`Self::is_splice_safe`].
    #[must_use]
    pub fn is_frame_sensitive(&self, name: &str) -> bool {
        self.by_name.get(name).is_some_and(|specs| {
            specs
                .iter()
                .any(|s| crate::traits::is_frame_sensitive(s.traits))
        })
    }

    /// Which substitutions `name` performs over its own argument text for a
    /// call with `args`, or `None` when it performs none at all.
    ///
    /// [`Traits::PERFORMS_SUBSTITUTION`] says *that* a command substitutes;
    /// this answers *which kinds* for the call in hand, so a consumer asking
    /// "does this argument read a variable?" never has to match option
    /// spellings itself. The answer is the projection of the call's option
    /// effects onto the substitution axis
    /// ([`CommandSpec::substitutions_performed`]), under this registry's
    /// profile: a command whose options move no substitution axis performs
    /// every kind on every call, and an unreadable call — a computed switch,
    /// a spelling the release lacks, the two families mixed — answers every
    /// kind too, because assuming a substitution does not happen is the
    /// answer that loses a real read.
    #[must_use]
    pub fn substitutions_performed(
        &self,
        name: &str,
        args: &[&str],
    ) -> Option<crate::substitution::SubstitutionKinds> {
        self.get(name)?.substitutions_performed(
            InvocationArguments::literals(args),
            self.own_surface_query(),
        )
    }

    /// Every command name that is frame-sensitive ([`Self::is_frame_sensitive`]),
    /// for consumers that scan text for any member of the set (the
    /// inline-proc code action's body check) rather than querying one
    /// resolved head.
    #[must_use]
    pub fn frame_sensitive_commands(&self) -> Vec<&str> {
        self.by_name
            .iter()
            .filter_map(|(name, specs)| {
                specs
                    .iter()
                    .any(|s| crate::traits::is_frame_sensitive(s.traits))
                    .then_some(*name)
            })
            .collect()
    }

    /// Whether `name` is **splice-safe**: a call to it can be lifted out of
    /// a wrapper proc and spliced into any caller's frame without changing
    /// observable behaviour.  A splice-safe command is fully lowered
    /// ([`Traits::FRAMELESS_RUNTIME`] — it never falls back to the
    /// interpreter, so it needs no frame) **and** frame-independent: not
    /// frame-sensitive ([`Self::is_frame_sensitive`]) and not operating on
    /// variables by name ([`Traits::FIRST_ARG_VARNAME`] writers,
    /// [`Traits::FRAME_HASH_BUILTIN`] hash-bucket binders, and
    /// [`Traits::DYNAMIC_EVAL_BODY`] body evaluators all observe the frame
    /// they run in).  Drives the inliner's verbatim-splice eligibility;
    /// membership is pinned by `splice_safe_membership` in
    /// `tests/registry_commands.rs`.
    #[must_use]
    pub fn is_splice_safe(&self, name: &str) -> bool {
        self.get(name)
            .is_some_and(|spec| crate::traits::is_splice_safe(spec.traits))
    }

    /// How *name* crosses stack frames, if it does — the registry's
    /// [`FrameEffectSpec`](crate::frame_effect::FrameEffectSpec).
    ///
    /// The single membership test for "is this a frame-crossing command?".
    /// A consumer reads the returned descriptor to find the level word and
    /// the affected arguments; it never names `upvar` / `uplevel` / `eval`
    /// itself.
    #[must_use]
    pub fn frame_effect(&self, name: &str) -> Option<crate::frame_effect::FrameEffectSpec> {
        self.get(name).and_then(|spec| spec.frame_effect)
    }

    /// The behavioural traits in force for one **concrete call** — the
    /// command's own set, unioned with those of the subcommand `args[0]`
    /// resolves to (exact or unique-prefix), when it has subcommands.
    ///
    /// The trait counterpart of [`Self::arg_indices_for_role`], and it takes
    /// the same `args` shape (subcommand first) for the same reason: a
    /// consumer asking "does *this* call declare a namespace / evaluate code"
    /// must not have to know whether the fact lives on the ensemble or on one
    /// of its subcommands. `namespace eval` carries
    /// [`Traits::DECLARES_NAMESPACE`] on the subcommand while `namespace`
    /// itself does not, and `namespace inscope` — identical argument layout,
    /// same analyser hook — does not carry it at all.
    #[must_use]
    pub fn call_traits(&self, name: &str, args: &[&str]) -> Traits {
        let Some(spec) = self.get(name) else {
            return Traits::empty();
        };
        let sub = (!spec.subcommands.is_empty())
            .then(|| args.first().and_then(|word| spec.resolve_subcommand(word)))
            .flatten();
        sub.map_or(spec.traits, |sub| spec.traits.union(sub.semantic_traits()))
    }

    /// The declared [`ArgTypeHint`] for the 0-based argument `index` (after
    /// the command name) of a call to `name` with `args`.
    ///
    /// Resolves through the subcommand when the command is an ensemble and
    /// `args[0]` names one, so `dict for {k v} $d body` reports the `Dict`
    /// hint the `for` subcommand declares on its own argument 1.
    ///
    /// [`ArgTypeHint`]: crate::hooks::ArgTypeHint
    #[must_use]
    pub fn arg_type_hint(
        &self,
        name: &str,
        args: &[&str],
        index: usize,
    ) -> Option<&'static crate::hooks::ArgTypeHint> {
        let spec = self.get(name)?;
        let find = |types: &'static [(u8, crate::hooks::ArgTypeHint)], idx: usize| {
            u8::try_from(idx)
                .ok()
                .and_then(|i| types.iter().find(|(at, _)| *at == i).map(|(_, h)| h))
        };
        if !spec.subcommands.is_empty()
            && !args.is_empty()
            && let Some(sub) = spec.resolve_subcommand(args[0])
            && index >= 1
        {
            return find(sub.arg_types, index - 1);
        }
        find(spec.arg_types, index)
    }

    /// Selected argument representation hint without fabricating dynamic values.
    /// Literal subcommands select their authored positional table. Unresolved
    /// leading option operands retain uncertainty about the positional offset.
    #[must_use]
    pub fn arg_type_hint_words(
        &self,
        words: crate::InvocationWords<'_>,
        index: usize,
    ) -> Option<&'static crate::hooks::ArgTypeHint> {
        let arguments = words.arguments();
        let query = arguments
            .dialect()
            .and_then(crate::InvocationDialect::authoring_query);
        let spec = self.get_for_surface(words.head_literal()?, query)?;
        let find = |types: &'static [(u8, crate::hooks::ArgTypeHint)], index| {
            let index = u8::try_from(index).ok()?;
            types
                .iter()
                .find(|(at, _)| *at == index)
                .map(|(_, hint)| hint)
        };
        if spec.subcommands.is_empty() {
            let types = pick_form(
                spec,
                None,
                arguments,
                crate::resolved_invocation::InvocationAvailability {
                    query,
                    package_version: self.package_floor_for_spec(spec),
                },
            )
            .and_then(|form| form.arg_types)
            .unwrap_or(spec.arg_types);
            return find(types, index);
        }
        let sub = spec.resolve_subcommand(arguments.literal_at(0)?)?;
        let availability = crate::resolved_invocation::InvocationAvailability {
            query,
            package_version: self.package_floor_for_spec(spec),
        };
        let form = pick_form(spec, Some(sub), arguments, availability);
        let types = form
            .and_then(|form| form.arg_types)
            .unwrap_or(sub.arg_types);
        let relative = index.checked_sub(1)?;
        let options =
            crate::resolved_invocation::invocation_options(spec, Some(sub), form, availability);
        let skipped = options.leading_word_count(arguments.slice_from(1))?;
        find(types, relative.checked_sub(skipped)?)
    }

    /// Resolve argument indices for a given role.
    ///
    /// For subcommand-based commands (e.g. `dict create`), pass the
    /// subcommand as the first element of `args`.
    ///
    /// Four role sources feed this, in the order the registry contract
    /// documents: the [`CommandSpec::clause_grammar`] walk's flat roles (the
    /// clause structure — keywords, conditions, scripts), a dynamic
    /// `arg_role_resolver` or else the static `arg_roles` table, and — for
    /// the unbounded regular tails a fixed table cannot express — the
    /// [`RepeatedArgLayout`]s of [`CommandSpec::repeated_args`].  The
    /// grammar and the repeated layouts are *additive*: a spec may pin its
    /// leading words with `arg_roles` (`namespace upvar`'s leading namespace
    /// word) and still declare the repeating pair tail, and `catch` states
    /// its clause structure in a grammar while its `arg_roles` keep the
    /// result words' `VarWrite`.
    ///
    /// [`RepeatedArgLayout`]: crate::repeated::RepeatedArgLayout
    #[must_use]
    pub fn arg_indices_for_role(&self, name: &str, args: &[&str], role: ArgRole) -> Vec<usize> {
        // `CommandPrefix` positions (with their appended arities) are owned by
        // [`Self::command_prefixes`]; delegate so highlighting, param-trait
        // inference, and the call-reference extractor all read one source.
        if role == ArgRole::CommandPrefix {
            return self
                .command_prefixes(name, args)
                .into_iter()
                .map(|(i, _)| i)
                .collect();
        }
        self.arg_role_assignments(name, args, &[role])
            .into_iter()
            .map(|(index, _)| index)
            .collect()
    }

    /// The `(index, role)` pairs a call gives any of `wanted`, from the
    /// sources [`Self::arg_indices_for_role`] documents, through the one rule
    /// [`arg_roles_in`] states. One walk answers every wanted role, so a
    /// consumer that needs several (a definer's name, parameter list and
    /// body) asks once.
    ///
    /// [`ArgRole::CommandPrefix`] is not answered here; see
    /// [`Self::command_prefixes`].
    fn arg_role_assignments(
        &self,
        name: &str,
        args: &[&str],
        wanted: &[ArgRole],
    ) -> Vec<(usize, ArgRole)> {
        self.arg_role_assignments_words(
            name,
            InvocationArguments::literals(args).with_profile(self.profile()),
            wanted,
        )
        .unwrap_or_default()
    }

    /// Project selected roles once, retaining the execution dialect and
    /// command/subcommand/form precedence of structured resolution.
    /// Unknown source positions return `None`; computed ordinary values never
    /// become literal selectors. This is descriptor metadata, not handler
    /// binding or execution evidence. Command-prefix appended arity remains
    /// owned by [`Self::arg_indices_for_role_words`].
    #[must_use]
    pub fn arg_role_assignments_words(
        &self,
        name: &str,
        args: InvocationArguments<'_>,
        wanted: &[ArgRole],
    ) -> Option<Vec<(usize, ArgRole)>> {
        self.arg_role_assignments_words_for_dialect(name, args, wanted, self.own_surface_query())
    }

    fn arg_role_assignments_words_for_dialect(
        &self,
        name: &str,
        args: InvocationArguments<'_>,
        wanted: &[ArgRole],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<Vec<(usize, ArgRole)>> {
        let args = args.with_profile(self.profile());
        let query = args
            .dialect()
            .and_then(crate::InvocationDialect::authoring_query)
            .or(dialect);
        let words = InvocationWords::from_arguments(InvocationWord::Literal(name), args);
        let resolved = self
            .resolve_structured_invocation(words, query)
            .resolved()?;
        self.selected_argument_role_assignments(name, args, wanted, query, resolved)
    }

    fn selected_argument_role_assignments(
        &self,
        _name: &str,
        _args: InvocationArguments<'_>,
        wanted: &[ArgRole],
        _query: Option<SurfaceQuery<'_>>,
        resolved: ResolvedInvocation<'_, '_>,
    ) -> Option<Vec<(usize, ArgRole)>> {
        // naming.source.authored-registry-role-projection
        // docs/design/analysis/name-resolution-proofs/authored-registry-role-projection.md
        Some(
            resolved
                .arg_roles()?
                .into_iter()
                .filter(|(_, role)| *role != ArgRole::CommandPrefix && wanted.contains(role))
                .collect(),
        )
    }

    /// The clause plan of a call to `name` with `args` — the command's (or,
    /// when `args[0]` names one, the subcommand's) clause grammar walked under
    /// this registry's own profile, in the same post-head coordinates as
    /// [`Self::arg_indices_for_role`]. `None` when neither declares a grammar
    /// or it is unavailable at the profile.
    #[must_use]
    pub fn clause_plan(&self, name: &str, args: &[&str]) -> Option<crate::ClausePlan> {
        let spec = self.get(name)?;
        let dialect = self.own_surface_query();
        if !spec.subcommands.is_empty()
            && let Some(sub) = args.first().and_then(|word| spec.resolve_subcommand(word))
        {
            return sub
                .clause_plan(&args[1..], dialect)
                .map(|plan| plan.offset_by(1));
        }
        spec.clause_plan(args, dialect)
    }

    /// The structural defect a consumer reports for a call to `name` in place
    /// of the generic arity check — [`CommandSpec::clause_shape_defect`] under
    /// this registry's own profile (`if`'s E004).
    #[must_use]
    pub fn clause_shape_defect(
        &self,
        name: &str,
        args: &[&str],
    ) -> Option<crate::ClauseShapeError> {
        self.get(name)?
            .clause_shape_defect(args, self.own_surface_query())
    }

    /// Resolve the argument indices whose **brace-quoted** word this command
    /// nonetheless evaluates in the *calling* frame — `expr {$a} + 1`,
    /// `if {$c} …` — so a `$name` written inside one is a genuine read at
    /// the call site.
    ///
    /// Tcl substitutes nothing inside `{…}`; whether the callee then
    /// evaluates that text, and in whose frame, is the per-role fact
    /// [`ArgRole::braced_word_evaluated_in_frame`] owns. This is the one
    /// place that asks it over a call's argument words, so the SSA use
    /// classifier and the shimmer detectors share a single answer instead of
    /// each rebuilding the set.
    #[must_use]
    pub fn arg_indices_evaluated_in_frame(&self, name: &str, args: &[&str]) -> Vec<usize> {
        let roles: Vec<_> = ArgRole::ALL
            .iter()
            .copied()
            .filter(|role| role.braced_word_evaluated_in_frame())
            .collect();
        let mut out: Vec<_> = self
            .arg_role_assignments(name, args, &roles)
            .into_iter()
            .map(|(index, _)| index)
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Resolve argument-role positions from source-aware invocation words.
    ///
    /// `None` means Tcl expansion, a computed subcommand, or an option-driven
    /// layout prevents a sound source-position projection. Dynamic ordinary
    /// operands remain usable: they occupy exactly one argv position and are
    /// represented by an inert placeholder while the registry-owned layout is
    /// resolved.
    #[must_use]
    pub fn arg_indices_for_role_words(
        &self,
        name: &str,
        args: InvocationArguments<'_>,
        role: ArgRole,
    ) -> Option<Vec<usize>> {
        if role == ArgRole::CommandPrefix {
            // Preserve the callback owner's appended-arity contract after the
            // same selected descriptor has proved the source layout.
            self.arg_role_assignments_words(name, args, &[])?;
            let spellings: Vec<_> = (0..args.len())
                .map(|index| args.literal_at(index).unwrap_or(""))
                .collect();
            let words: Vec<_> = (0..args.len())
                .filter_map(|index| args.get(index))
                .collect();
            return Some(
                self.command_prefixes_structured(name, &spellings, &words)
                    .into_iter()
                    .map(|(index, _)| index)
                    .collect(),
            );
        }
        self.arg_role_assignments_words(name, args, &[role])
            .map(|roles| roles.into_iter().map(|(index, _)| index).collect())
    }

    /// Provider admission for effect queries and instance metadata. Catalogue
    /// availability and a package version alone do not establish this entry.
    fn effect_provider_is_admitted(
        &self,
        spec: &CommandSpec,
        query: Option<SurfaceQuery<'_>>,
    ) -> bool {
        spec.owning_package().is_none_or(|package| {
            self.is_ambient_package(package)
                || query
                    .is_some_and(|query| query.packages.iter().any(|floor| floor.name == package))
        })
    }

    fn effect_query(&self, words: InvocationWords<'_>) -> Option<SurfaceQuery<'_>> {
        words
            .arguments()
            .dialect()
            .and_then(crate::InvocationDialect::authoring_query)
            .or_else(|| self.own_surface_query())
    }

    fn effect_invocation_for_query<'r, 'w>(
        &'r self,
        words: InvocationWords<'w>,
        query: Option<SurfaceQuery<'w>>,
    ) -> Option<ResolvedInvocation<'r, 'w>> {
        let spec = self.get_for_surface(words.head_literal()?, query)?;
        if !self.effect_provider_is_admitted(spec, query) {
            return None;
        }
        self.resolve_structured_invocation(words, query).resolved()
    }

    /// Project the variable cells source-aware invocation words read **by
    /// name** — the read-only counterpart of
    /// [`Self::variable_write_projection`], and resolved the same way, so an
    /// alias or renamed spelling answers like the command it reaches.
    ///
    /// A name word that substitutes (`info exists $p`) denotes a cell only
    /// the runtime knows, so it widens [`VariableReadProjection::
    /// opaque_variable_frame`] rather than exposing its source spelling as a
    /// variable name.
    ///
    /// A destroyer's targets are read too (`unset x`, the
    /// [`Traits::DESTROYS_VARIABLE`] trait): an unbind reads its place's
    /// existence before it removes it — an absent place raises unless
    /// `-nocomplain` says otherwise — so the store feeding the place is
    /// observed. [`Self::variable_write_projection`] leaves a destroy out, a
    /// destroy being no value definition, so without this a nested `[unset
    /// x]` observed nothing and the store before it was deleted as dead.
    #[must_use]
    pub fn variable_read_projection(&self, words: InvocationWords<'_>) -> VariableReadProjection {
        let query = self.effect_query(words);
        self.variable_read_projection_for_query(words, query, false)
    }

    /// Source read footprint under independently retained complete availability.
    /// Unavailable metadata stays opaque; this does not prove a read or cell.
    #[must_use]
    pub fn variable_read_projection_in_resolved_context(
        &self,
        context: &crate::model::ResolvedContext,
        words: InvocationWords<'_>,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> VariableReadProjection {
        if words
            .head_literal()
            .is_none_or(|name| context.resolve_spec_in_realm(self, name, realm).is_none())
        {
            return VariableReadProjection {
                opaque_variable_frame: true,
                ..VariableReadProjection::default()
            };
        }
        self.variable_read_projection_for_query(
            words,
            Some(context.authoring_query().with_realm(realm)),
            true,
        )
    }

    fn variable_read_projection_for_query(
        &self,
        words: InvocationWords<'_>,
        query: Option<SurfaceQuery<'_>>,
        unavailable_is_opaque: bool,
    ) -> VariableReadProjection {
        let Some(name) = words.head_literal() else {
            return VariableReadProjection {
                literal_names: Vec::new(),
                opaque_variable_frame: true,
            };
        };
        let Some(invocation) = self.effect_invocation_for_query(words, query) else {
            return VariableReadProjection {
                opaque_variable_frame: unavailable_is_opaque,
                ..VariableReadProjection::default()
            };
        };
        let args = words.arguments();
        let wanted: &[ArgRole] = if invocation
            .semantics
            .traits
            .contains(Traits::DESTROYS_VARIABLE)
        {
            &[ArgRole::VarRead, ArgRole::VarWrite]
        } else {
            &[ArgRole::VarRead]
        };
        let Some(roles) =
            self.selected_argument_role_assignments(name, args, wanted, query, invocation)
        else {
            return VariableReadProjection {
                literal_names: Vec::new(),
                opaque_variable_frame: self.get_for_surface(name, query).is_some_and(|spec| {
                    wanted
                        .iter()
                        .any(|role| Self::spec_may_have_arg_role(spec, *role))
                }),
            };
        };
        let mut projection = VariableReadProjection::default();
        for (index, _) in roles {
            match args.literal_at(index) {
                Some(variable) if !variable.is_empty() => {
                    if !projection
                        .literal_names
                        .iter()
                        .any(|found| found == variable)
                    {
                        projection.literal_names.push(variable.to_owned());
                    }
                }
                _ => projection.opaque_variable_frame = true,
            }
        }
        projection
    }

    /// Project the variable-cell writes of source-aware invocation words.
    ///
    /// Command, subcommand, option, form, and repeated-tail selection stays
    /// owned by the registry. A dynamic or expanded target is never exposed
    /// as if its source spelling were its runtime variable name. A computed
    /// head is opaque because it may dispatch any variable-writing command.
    #[must_use]
    pub fn variable_write_projection(&self, words: InvocationWords<'_>) -> VariableWriteProjection {
        let query = self.effect_query(words);
        self.variable_write_projection_for_query(words, query, false)
    }

    /// Source write footprint under independently retained complete availability.
    /// Unknown or unavailable metadata stays opaque, never a successful store.
    #[must_use]
    pub fn variable_write_projection_in_resolved_context(
        &self,
        context: &crate::model::ResolvedContext,
        words: InvocationWords<'_>,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> VariableWriteProjection {
        if words
            .head_literal()
            .is_none_or(|name| context.resolve_spec_in_realm(self, name, realm).is_none())
        {
            return VariableWriteProjection {
                opaque_variable_frame: true,
                ..VariableWriteProjection::default()
            };
        }
        self.variable_write_projection_for_query(
            words,
            Some(context.authoring_query().with_realm(realm)),
            true,
        )
    }

    fn variable_write_projection_for_query(
        &self,
        words: InvocationWords<'_>,
        query: Option<SurfaceQuery<'_>>,
        unavailable_is_opaque: bool,
    ) -> VariableWriteProjection {
        let Some(name) = words.head_literal() else {
            return VariableWriteProjection {
                literal_names: Vec::new(),
                read_before_write_names: Vec::new(),
                opaque_variable_frame: true,
            };
        };
        let Some(invocation) = self.effect_invocation_for_query(words, query) else {
            return VariableWriteProjection {
                opaque_variable_frame: unavailable_is_opaque,
                ..VariableWriteProjection::default()
            };
        };

        Self::selected_variable_write_projection(&invocation, || {
            self.arg_role_variable_writes(name, words, query, invocation)
        })
    }

    fn selected_variable_write_projection(
        invocation: &ResolvedInvocation<'_, '_>,
        roles: impl FnOnce() -> VariableWriteProjection,
    ) -> VariableWriteProjection {
        // `incr` / `append` / `lappend` / `lset` / `lpop` / `ledit` fold the
        // target's current value into the one they store, so the write is
        // also a read of the same cell. Taken from the *resolved* invocation,
        // so an alias or rename spelling answers like the builtin it reaches.
        // Every VarWrite target of such a command is its read-modify-write
        // target — none of them carries a second, write-only variable role.
        let reads_before_write = invocation
            .semantics
            .traits
            .contains(Traits::READS_BEFORE_WRITE);
        let with_reads = |mut projection: VariableWriteProjection| {
            if reads_before_write {
                projection.read_before_write_names = projection.literal_names.clone();
            }
            projection
        };

        // A destroy is not a value definition. The registry's VarWrite role
        // intentionally covers both operations so name-aware consumers can
        // find their targets; this effect projection keeps their transfers
        // distinct.
        if invocation
            .semantics
            .traits
            .contains(Traits::DESTROYS_VARIABLE)
        {
            return VariableWriteProjection::default();
        }

        // A declared state-transition descriptor is the closed semantic owner
        // for variable-cell aliasing. In particular, `global`, `upvar`, trace
        // registration, and value-less `variable` forms may carry VarWrite
        // roles without writing a value. Only the descriptor's explicit
        // `writes_value` facts count here.
        if invocation.semantics.state_transitions.is_declared() {
            let mut projection = VariableWriteProjection::default();
            for fact in invocation.state_transitions().facts() {
                let StateTransition::VariableCellAlias(alias) = &fact.transition else {
                    continue;
                };
                if matches!(
                    alias.local,
                    TransitionSubject::LocatedNativeBytes { .. }
                        | TransitionSubject::Unknown { .. }
                ) {
                    // Even declaration-only aliases make later writes land in
                    // an unnameable cell, so effect consumers must widen.
                    projection.opaque_variable_frame = true;
                }
                if !alias.writes_value {
                    continue;
                }
                match &alias.local {
                    TransitionSubject::Literal(name)
                    | TransitionSubject::LocatedLiteral { value: name, .. } => {
                        if !name.is_empty() && !projection.literal_names.contains(name) {
                            projection.literal_names.push(name.clone());
                        }
                    }
                    TransitionSubject::LocatedNativeBytes { .. }
                    | TransitionSubject::Unknown { .. } => {
                        projection.opaque_variable_frame = true;
                    }
                }
            }
            return with_reads(projection);
        }

        with_reads(roles())
    }

    /// Possible output names of an already selected source descriptor.
    /// Selection/context remain owned by the caller. This never re-resolves a
    /// command spelling or proves a current variable write or entered frame.
    #[must_use]
    pub fn variable_write_projection_for_selected_source(
        invocation: &ResolvedInvocation<'_, '_>,
    ) -> VariableWriteProjection {
        Self::selected_variable_write_projection(invocation, || {
            let arguments = invocation.words.arguments();
            let Some(selected) = invocation.authored_source_arity() else {
                return VariableWriteProjection {
                    opaque_variable_frame: true,
                    ..Default::default()
                };
            };
            if arguments.exact_argv_len().is_some_and(|count| {
                selected
                    .command
                    .variable_write_min_args
                    .is_some_and(|minimum| count < usize::from(minimum))
            }) {
                return VariableWriteProjection::default();
            }
            let (roles, complete) = invocation.authored_source_argument_roles();
            if !complete {
                return VariableWriteProjection {
                    opaque_variable_frame: Self::spec_may_have_arg_role(
                        selected.command,
                        ArgRole::VarWrite,
                    ),
                    ..Default::default()
                };
            }
            let mut projection = VariableWriteProjection::default();
            for (argument, role) in roles {
                if role != ArgRole::VarWrite {
                    continue;
                }
                let argument = usize::from(argument) + invocation.semantics.argument_offset;
                let Some(name) = arguments
                    .literal_at(argument)
                    .filter(|name| !name.is_empty())
                else {
                    projection.opaque_variable_frame = true;
                    continue;
                };
                let name = if invocation.authored_source_option_variable_scope_at(argument)
                    == Some(crate::hover::VariableScope::Global)
                    && !name.starts_with("::")
                {
                    format!("::{name}")
                } else {
                    name.to_owned()
                };
                if !projection.literal_names.contains(&name) {
                    projection.literal_names.push(name);
                }
            }
            projection
        })
    }

    /// The [`ArgRole::VarWrite`] half of [`Self::variable_write_projection`]:
    /// the targets named by this invocation's argument words, once the
    /// declared-state-transition path has declined. Split out to keep each
    /// half readable on its own.
    fn arg_role_variable_writes(
        &self,
        name: &str,
        words: InvocationWords<'_>,
        query: Option<SurfaceQuery<'_>>,
        invocation: ResolvedInvocation<'_, '_>,
    ) -> VariableWriteProjection {
        let args = words.arguments();
        if args.exact_argv_len().is_some_and(|count| {
            self.get_for_surface(name, query)
                .and_then(|spec| spec.variable_write_min_args)
                .is_some_and(|minimum| count < usize::from(minimum))
        }) {
            // Option-sensitive role resolution may have to abstain when a
            // dynamic leading word could be a switch. The command descriptor
            // can nevertheless prove that this exact argv is too short to
            // contain any variable target (for example `regexp $re $s`).
            return VariableWriteProjection::default();
        }
        let global_arguments: std::collections::BTreeSet<_> = (0..args.len())
            .filter(|index| {
                invocation.authored_source_option_variable_scope_at(*index)
                    == Some(crate::hover::VariableScope::Global)
            })
            .collect();
        let Some(roles) = self.selected_argument_role_assignments(
            name,
            args,
            &[ArgRole::VarWrite],
            query,
            invocation,
        ) else {
            return VariableWriteProjection {
                literal_names: Vec::new(),
                read_before_write_names: Vec::new(),
                opaque_variable_frame: self
                    .get_for_surface(name, query)
                    .is_some_and(|spec| Self::spec_may_have_arg_role(spec, ArgRole::VarWrite)),
            };
        };
        let mut projection = VariableWriteProjection::default();
        for (index, _) in roles {
            match args.literal_at(index) {
                Some(variable) if !variable.is_empty() => {
                    let effective_name =
                        if global_arguments.contains(&index) && !variable.starts_with("::") {
                            format!("::{variable}")
                        } else {
                            variable.to_owned()
                        };
                    if !projection
                        .literal_names
                        .iter()
                        .any(|found| found == &effective_name)
                    {
                        projection.literal_names.push(effective_name);
                    }
                }
                _ => projection.opaque_variable_frame = true,
            }
        }
        projection
    }

    /// Whether any registry-owned layout for `name` can assign `role`.
    ///
    /// Dynamic role hooks are conservatively capable of every role; this is
    /// used only when source expansion makes their concrete answer unknowable.
    #[must_use]
    pub fn may_have_arg_role(&self, name: &str, role: ArgRole) -> bool {
        let Some(spec) = self.get(name) else {
            return false;
        };
        Self::spec_may_have_arg_role(spec, role)
    }

    /// Pure role capabilities of an independently selected command schema.
    /// Callers retain availability and context selection; this query does not
    /// select a spelling again or establish execution of a script operand.
    #[must_use]
    pub fn spec_may_have_arg_role(spec: &CommandSpec, role: ArgRole) -> bool {
        let options_have = |options: &[crate::hover::OptionSpec]| {
            options.iter().any(|option| {
                option.value_role() == Some(role) || option.value_also_role() == Some(role)
            })
        };
        spec.arg_role_resolver_roles.contains(&role)
            || spec.arg_roles.iter().any(|(_, found)| *found == role)
            || spec
                .clause_grammar
                .is_some_and(|grammar| grammar.may_assign(role))
            || spec.repeated_args.iter().any(|layout| layout.role == role)
            || options_have(spec.options)
            || spec.command_forms.iter().any(|form| {
                form.arg_roles.iter().any(|(_, found)| *found == role) || options_have(form.options)
            })
            || spec.subcommands.iter().any(|sub| {
                sub.arg_role_resolver_roles.contains(&role)
                    || sub.arg_roles.iter().any(|(_, found)| *found == role)
                    || sub
                        .clause_grammar
                        .is_some_and(|grammar| grammar.may_assign(role))
                    || sub.repeated_args.iter().any(|layout| layout.role == role)
                    || options_have(sub.options)
            })
    }

    /// Resolve a subcommand only when its source word has a known literal
    /// value. A dynamic selector cannot be projected into a selected
    /// descriptor's argument layout.
    fn source_selected_subcommand<'a>(
        spec: &'a CommandSpec,
        args: InvocationArguments<'_>,
    ) -> Option<&'a SubCommand> {
        (!spec.subcommands.is_empty() && spec.constructor_prefix_words().is_none())
            .then(|| {
                args.literal_at(0)
                    .and_then(|word| spec.resolve_subcommand(word))
            })
            .flatten()
    }

    /// The format-string words of **one concrete call**: which argument
    /// positions carry a conversion / field string, and which mini-language
    /// each is written in.
    ///
    /// The single registry answer, in place of matching command spellings —
    /// `match head { "format" => …, "scan" =>
    /// …, "clock" => …, "binary" => …, "regsub" => … }` — in both the
    /// semantic-token walk and the inlay-hint collector, each with its own
    /// copy of the argument layout. It combines the two facts
    /// the registry already models:
    ///
    /// * **Where** — the [`ArgRole::FormatString`] / [`ArgRole::ScanFormat`]
    ///   positions of the call, resolved through
    ///   [`Self::arg_indices_for_role`], so a fixed index (`format`), a
    ///   resolver-computed one (`scan`, `regsub` past its switches), a
    ///   subcommand-relative one (`binary format`), and an **option value**
    ///   (`clock format … -format FMT`) all answer the same way.
    /// * **Which language** — [`CommandSpec::format_string_type`], overridden
    ///   by [`SubCommand::format_string_type`] when a subcommand dispatches
    ///   (`clock format` ⇒ `Clock`, `binary scan` ⇒ `Binary`).
    ///
    /// Because the head is resolved through [`Self::get`], the explicitly
    /// global spellings (`::format`, `::clock`, …) answer identically to the
    /// bare ones — the false negative the spelling tests had. A command with
    /// no declared family answers empty, so a same-named user proc, an
    /// unknown command, or a dynamic head is never misread.
    ///
    /// `args` is the post-head argument list, subcommand first, the same
    /// shape [`Self::arg_indices_for_role`] takes; returned indices are into
    /// that list.
    #[must_use]
    pub fn format_string_args(&self, name: &str, args: &[&str]) -> Vec<FormatStringArg> {
        let Some(spec) = self.get(name) else {
            return Vec::new();
        };
        // A dispatching subcommand's family wins over the parent's.
        let sub = (!spec.subcommands.is_empty())
            .then(|| args.first().and_then(|word| spec.resolve_subcommand(word)))
            .flatten();
        let Some(kind) = sub
            .and_then(|s| s.format_string_type)
            .or(spec.format_string_type)
        else {
            return Vec::new();
        };
        let mut out: Vec<FormatStringArg> = Vec::new();
        for (role, scan) in [(ArgRole::FormatString, false), (ArgRole::ScanFormat, true)] {
            out.extend(
                self.arg_indices_for_role(name, args, role)
                    .into_iter()
                    .map(|index| FormatStringArg { index, kind, scan }),
            );
        }
        out.sort_by_key(|f| f.index);
        out
    }

    /// Source-aware counterpart to [`Self::format_string_args`].
    ///
    /// Unlike the compatibility string query, this method retains whether
    /// each source word is literal, dynamic, expanded, or opaque. The format
    /// family still comes entirely from the registry, but a dynamic leading
    /// word is never reinterpreted as a positional pattern or replacement
    /// before the profile-filtered option grammar has resolved it. This is
    /// the owner query for editors and other source-aware consumers.
    #[must_use]
    pub fn format_string_args_words(
        &self,
        name: &str,
        args: InvocationArguments<'_>,
    ) -> Vec<FormatStringArg> {
        self.format_string_args_words_for_dialect(name, args, self.own_surface_query())
    }

    /// Dialect-explicit counterpart to [`Self::format_string_args_words`].
    #[must_use]
    pub fn format_string_args_words_for_dialect(
        &self,
        name: &str,
        args: InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Vec<FormatStringArg> {
        let effective_dialect = self.own_surface_query().or(dialect);
        let spec = if effective_dialect.is_none() {
            self.get(name)
        } else {
            self.get_for_surface(name, effective_dialect)
        };
        let Some(spec) = spec else {
            return Vec::new();
        };
        // A dynamic subcommand cannot select a concrete format family.
        if !spec.subcommands.is_empty() && !args.is_empty() && args.literal_at(0).is_none() {
            return Vec::new();
        }
        let sub = Self::source_selected_subcommand(spec, args);
        let Some(kind) = sub
            .and_then(|sub| sub.format_string_type)
            .or(spec.format_string_type)
        else {
            return Vec::new();
        };
        let Some(roles) = self.arg_role_assignments_words_for_dialect(
            name,
            args,
            &[ArgRole::FormatString, ArgRole::ScanFormat],
            effective_dialect,
        ) else {
            return Vec::new();
        };
        let mut out = roles
            .into_iter()
            .map(|(index, role)| FormatStringArg {
                index,
                kind,
                scan: role == ArgRole::ScanFormat,
            })
            .collect::<Vec<_>>();
        out.sort_by_key(|format| format.index);
        out
    }

    /// The pattern-bearing arguments of one concrete invocation.
    ///
    /// This pairs each declared position with its embedded language. A static
    /// command derives the positions from [`ArgRole::Pattern`]; an
    /// option-selected command (`lsearch -regexp`) answers the projection of
    /// its option effects onto the pattern-language axis, and a layout no
    /// axis can express keeps the registry resolver escape hatch. Consumers
    /// therefore never need command-name or `-regexp` branches of their own.
    #[must_use]
    pub fn pattern_args(&self, name: &str, args: &[&str]) -> Vec<crate::patterns::PatternArg> {
        self.pattern_args_for_dialect(name, args, self.own_surface_query())
    }

    /// Dialect-explicit counterpart to [`Self::pattern_args`].
    ///
    /// A profile-bound registry always uses its attached profile.  A
    /// profile-less registry uses `dialect`, which is how editor consumers
    /// carrying one shared registry keep option-selected pattern layouts in
    /// sync with the current document's Tcl release.
    #[must_use]
    pub fn pattern_args_for_dialect(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Vec<crate::patterns::PatternArg> {
        let effective_dialect = self.own_surface_query().or(dialect);
        let spec = if effective_dialect.is_none() {
            self.get(name)
        } else {
            self.get_for_surface(name, effective_dialect)
        };
        let Some(spec) = spec else {
            return Vec::new();
        };
        pattern_args_in(
            spec,
            Self::source_selected_subcommand(spec, InvocationArguments::literals(args)),
            args,
            || {
                self.profile().map_or_else(
                    || spec.option_specs(effective_dialect),
                    |profile| {
                        crate::profile_queries::ProfileQueries::available_option_specs(
                            profile, spec,
                        )
                    },
                )
            },
            effective_dialect,
            || self.arg_indices_for_role(name, args, ArgRole::Pattern),
        )
    }

    /// Source-aware counterpart to [`Self::pattern_args`].
    ///
    /// A pattern descriptor may use a dedicated pattern resolver, a dynamic
    /// argument-role resolver, or static roles. Every option-dependent form
    /// goes through the same source-layout proof before the compatibility
    /// resolver sees a spelling projection, so `regexp $mode …`,
    /// `glob $mode …`, and `string match $mode …` all abstain consistently.
    #[must_use]
    pub fn pattern_args_words(
        &self,
        name: &str,
        args: InvocationArguments<'_>,
    ) -> Vec<crate::patterns::PatternArg> {
        self.pattern_args_words_for_dialect(name, args, self.own_surface_query())
    }

    /// Dialect-explicit counterpart to [`Self::pattern_args_words`].
    #[must_use]
    pub fn pattern_args_words_for_dialect(
        &self,
        name: &str,
        args: InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Vec<crate::patterns::PatternArg> {
        let query = self.own_surface_query().or(dialect);
        let words = InvocationWords::from_arguments(
            InvocationWord::Literal(name),
            args.with_profile(self.profile()),
        );
        self.resolve_structured_invocation(words, query)
            .resolved()
            .and_then(|resolved| resolved.authored_source_pattern_arguments())
            .unwrap_or_default()
    }

    /// How a formatter should **present** argument `index` of a call to
    /// `name` — the layout fact that refines the argument's semantic
    /// [`ArgRole`].
    ///
    /// Returns the declared override, or [`ArgPresentation::BlockScript`]
    /// (the default) when the spec says nothing. `for`'s `start` and `next`
    /// scripts answer [`ArgPresentation::InlineScript`], which is how the
    /// formatting engine keeps them on the header line without a
    /// `name == "for"` branch.
    ///
    /// Resolves through [`Self::get`], so the explicitly-global spelling
    /// `::for` answers identically to the bare `for` — the false negative
    /// the formatter's literal name comparison had. A command the registry
    /// does not know (a user proc, a dynamic head) answers the default,
    /// which is what leaves such a call formatted like any other command.
    ///
    /// `args` is the post-name argument list, in the same shape
    /// [`Self::arg_indices_for_role`] takes (subcommand first), so a
    /// subcommand-dispatching call resolves against the subcommand's own
    /// table with the same `+1` offset.
    #[must_use]
    pub fn arg_presentation(
        &self,
        name: &str,
        args: &[&str],
        index: usize,
    ) -> crate::presentation::ArgPresentation {
        use crate::presentation::ArgPresentation;
        let Some(spec) = self.get(name) else {
            return ArgPresentation::default();
        };
        // A dispatching subcommand owns the positions after its own word.
        if !spec.subcommands.is_empty()
            && let Some(sub) = args.first().and_then(|word| spec.resolve_subcommand(word))
            && index > 0
            && let Ok(sub_index) = u8::try_from(index - 1)
        {
            return sub
                .arg_presentation
                .iter()
                .find(|(i, _)| *i == sub_index)
                .map_or_else(ArgPresentation::default, |(_, p)| *p);
        }
        let Ok(idx) = u8::try_from(index) else {
            return ArgPresentation::default();
        };
        spec.arg_presentation
            .iter()
            .find(|(i, _)| *i == idx)
            .map_or_else(ArgPresentation::default, |(_, p)| *p)
    }

    /// The declared roles of the argument positions a call has **not**
    /// filled — the optional trailing words a caller could still append.
    ///
    /// [`Self::arg_indices_for_role`] answers "what role does each supplied
    /// argument play"; this answers the complementary question a
    /// splice-a-trailing-word quick-fix needs: "what would the *next* words
    /// mean if I added them".  Positions are returned in ascending order,
    /// starting at `args.len()` and stopping at the spec's arity ceiling, and
    /// only while the spec actually declares a role for the position — an
    /// unlimited-arity command with no declared tail role yields nothing
    /// rather than an unbounded run of `Value` slots.
    ///
    /// A subcommand-dispatching call resolves against the subcommand's own
    /// role table (offset by the subcommand word), exactly as
    /// [`Self::arg_indices_for_role`] does, so `chan gets`-shaped commands
    /// answer for the right spec.  Dynamic
    /// [`arg_role_resolver`](CommandSpec::arg_role_resolver) tables are
    /// consulted with the supplied argument list, which is what a resolver
    /// keyed on the *written* words can answer; positions the resolver does
    /// not name are simply absent from the result.
    #[must_use]
    pub fn unfilled_trailing_roles(&self, name: &str, args: &[&str]) -> Vec<(usize, ArgRole)> {
        self.unfilled_trailing_roles_words(
            name,
            InvocationArguments::literals(args).with_profile(self.profile()),
        )
    }

    /// Prospective trailing roles under the retained argument grammar. Added
    /// words have unknown values and ordinary cardinality; original dynamic
    /// words and expansions never become compatibility literal placeholders.
    #[must_use]
    pub fn unfilled_trailing_roles_words(
        &self,
        name: &str,
        args: InvocationArguments<'_>,
    ) -> Vec<(usize, ArgRole)> {
        let args = args.with_profile(self.profile());
        if args.exact_argv_len() != Some(args.len()) {
            return Vec::new();
        }
        let query = args
            .dialect()
            .and_then(crate::InvocationDialect::authoring_query)
            .or_else(|| self.own_surface_query());
        let resolution = self.resolve_structured_invocation(
            InvocationWords::from_arguments(InvocationWord::Literal(name), args),
            query,
        );
        let Some(resolved) = resolution.resolved() else {
            return Vec::new();
        };
        if !matches!(
            resolved.subcommand,
            SubcommandResolution::Exact(_)
                | SubcommandResolution::UniquePrefix(_)
                | SubcommandResolution::NotApplicable
        ) {
            return Vec::new();
        }
        let semantics = &resolved.semantics;
        let offset = semantics.argument_offset;
        let Some(counted) = resolved.argument_count_for_arity() else {
            return Vec::new();
        };
        let ignored = args.len().saturating_sub(usize::from(counted));
        let ceiling = usize::from(semantics.arity.max).saturating_add(ignored);
        if semantics.arg_role_layout_resolver.is_some() {
            return self.prospective_layout_roles(name, args, ceiling, query);
        }
        let declared = if let Some(resolve) = semantics.arg_role_count_resolver {
            resolve(args.len().saturating_sub(offset))
        } else if let Some(resolve) = semantics.arg_role_resolver {
            let Some(values) = args.slice_from(offset).literal_values() else {
                return Vec::new();
            };
            resolve(&values)
        } else {
            semantics.arg_roles.to_vec()
        };
        (args.len()..ceiling)
            .map_while(|position| {
                declared.iter().find_map(|&(index, role)| {
                    (usize::from(index) + offset == position).then_some((position, role))
                })
            })
            .collect()
    }

    fn prospective_layout_roles(
        &self,
        name: &str,
        original: InvocationArguments<'_>,
        ceiling: usize,
        query: Option<SurfaceQuery>,
    ) -> Vec<(usize, ArgRole)> {
        let Some(mut arguments) = (0..original.len())
            .map(|index| original.get(index))
            .collect::<Option<Vec<_>>>()
        else {
            return Vec::new();
        };
        let mut result = Vec::new();
        for position in original.len()..ceiling.min(usize::from(u8::MAX) + 1) {
            arguments.push(InvocationWord::Dynamic);
            let words = InvocationArguments::structured(&arguments);
            let words = original
                .dialect()
                .map_or(words, |dialect| words.with_dialect(dialect));
            let resolution = self.resolve_structured_invocation(
                InvocationWords::from_arguments(InvocationWord::Literal(name), words),
                query,
            );
            let Some(resolved) = resolution.resolved() else {
                break;
            };
            let (roles, complete) = resolved.argument_roles();
            if !complete {
                break;
            }
            let offset = resolved.semantics.argument_offset;
            let Some(role) = roles.iter().find_map(|&(index, role)| {
                (usize::from(index) + offset == position).then_some(role)
            }) else {
                break;
            };
            result.push((position, role));
        }
        result
    }

    /// [`Self::arg_indices_for_role`]`(name, args, `[`ArgRole::Body`]`)`,
    /// filtered to the indices whose [`BodyKind`] is
    /// [`Plain`](BodyKind::Plain) — a body that runs in the caller's own
    /// frame (`if`/`while`/`for`/`foreach`/`switch`/`try`/`catch`/`eval`, …).
    ///
    /// `Structural` bodies (`proc`, `oo::class create`, `oo::define`,
    /// `snit::method`, `uplevel`, `namespace eval`, …) execute in a
    /// definition or different-frame dispatch context, so are excluded —
    /// a command found inside one is *not* still running in the enclosing
    /// call's scope.  This is the generic test for "is a dispatch nested in
    /// this body argument still the same lexical/dispatch context as the
    /// caller": intra-class `my`/`next`/`nextto` `TclOO` dispatch and `$obj`
    /// method dispatch recursion both need this to decide which nested
    /// script regions still belong to the same enclosing method.
    ///
    /// Respects a subcommand's own `body_kind` override (e.g. a compound
    /// command whose subcommand's body is structural even though the
    /// top-level spec defaults to `Plain`), the same resolution order
    /// [`Self::arg_indices_for_role`] uses for the subcommand's other roles.
    #[must_use]
    pub fn plain_body_arg_indices(&self, name: &str, args: &[&str]) -> Vec<usize> {
        let Some(spec) = self.get(name) else {
            return Vec::new();
        };
        let sub_body_kind = if spec.subcommands.is_empty() {
            None
        } else {
            args.first()
                .and_then(|first| spec.resolve_subcommand(first))
                .map(|sub| sub.body_kind)
        };
        let body_kind = sub_body_kind.unwrap_or(spec.body_kind);
        if body_kind != BodyKind::Plain {
            return Vec::new();
        }
        self.arg_indices_for_role(name, args, ArgRole::Body)
    }

    /// Resolve [`ArgRole::CommandPrefix`] argument positions and their
    /// [`AppendedArity`] for a concrete call.
    ///
    /// The single source of truth for command-prefix callbacks — unions the
    /// three declaration mechanisms (static [`CommandSpec::command_prefixes`]
    /// table, [`CommandSpec::command_prefix_resolver`], and `command_prefix`
    /// option values), for the top-level command or its resolved subcommand.
    /// Mirrors [`Self::arg_indices_for_role`]'s subcommand offset / `--`
    /// handling. Consumers: highlighting, find-references / call-hierarchy /
    /// call-graph recording, and the callback-arity check.
    ///
    /// For subcommand-based commands pass the subcommand as `args[0]`.
    #[must_use]
    pub fn command_prefixes(&self, name: &str, args: &[&str]) -> Vec<(usize, AppendedArity)> {
        self.command_prefixes_with_arguments(name, CommandPrefixArguments::literals(args))
    }

    /// Source-aware command-prefix resolution.
    ///
    /// `spellings` supports existing position-only declarations; `words`
    /// carries the proof boundary for literal-sensitive arity resolvers. The
    /// slices must describe the same post-head source words.
    #[must_use]
    pub fn command_prefixes_structured<'w>(
        &self,
        name: &str,
        spellings: &'w [&'w str],
        words: &'w [InvocationWord<'w>],
    ) -> Vec<(usize, AppendedArity)> {
        let Some(arguments) = CommandPrefixArguments::structured(spellings, words) else {
            return Vec::new();
        };
        self.command_prefixes_with_arguments(name, arguments)
    }

    fn command_prefixes_with_arguments(
        &self,
        name: &str,
        args: CommandPrefixArguments<'_>,
    ) -> Vec<(usize, AppendedArity)> {
        let Some(spec) = self.get(name) else {
            return Vec::new();
        };
        command_prefixes_in(
            spec,
            Self::source_selected_subcommand(spec, args.words()),
            args,
        )
    }

    /// Resolve a concrete invocation to its target-neutral registry semantics.
    ///
    /// This is the common compiler entry point.  It retains the original head
    /// and argument spellings while selecting the registry command,
    /// subcommand, and form descriptors.  The returned projection contains no
    /// backend code-generation hook; target registries decide how to emit the
    /// already-resolved semantic operation later.
    ///
    /// Returns `None` when the command head is unknown or unavailable in
    /// `dialect`.
    #[must_use]
    pub fn resolve_invocation<'r, 'w>(
        &'r self,
        name: &'w str,
        args: &'w [&'w str],
        dialect: Option<SurfaceQuery<'w>>,
    ) -> Option<ResolvedInvocation<'r, 'w>> {
        self.resolve_structured_invocation(InvocationWords::literals(name, args), dialect)
            .resolved()
    }

    /// Readonly descriptor availability from this exact command store and query.
    /// The supplied descriptor has already been selected independently. Scoped
    /// schemas inherit their body owner's package floor through this same owner.
    #[must_use]
    pub fn source_descriptor_availability<'q>(
        &self,
        spec: &CommandSpec,
        query: Option<SurfaceQuery<'q>>,
    ) -> crate::resolved_invocation::InvocationAvailability<'q> {
        crate::resolved_invocation::InvocationAvailability {
            query,
            package_version: self.package_floor_for_spec(spec),
        }
    }

    /// Resolve source-aware invocation words to target-neutral registry
    /// semantics.
    ///
    /// Only a literal command head can select a registry command. Likewise,
    /// only a literal first argument can select a subcommand. Expanded and
    /// opaque arguments make form arity indeterminate, so no form is matched.
    /// A substituted non-expanded argument still contributes exactly one argv
    /// entry and may therefore participate in an arity-only form selection;
    /// its spelling is never inspected as a value.
    ///
    /// The compatibility [`Self::resolve_invocation`] adapter constructs an
    /// all-literal view without allocation.
    #[must_use]
    pub fn resolve_structured_invocation<'r, 'w>(
        &'r self,
        words: InvocationWords<'w>,
        dialect: Option<SurfaceQuery<'w>>,
    ) -> StructuredInvocationResolution<'r, 'w> {
        let numbers = words
            .arguments()
            .dialect()
            .map(|dialect| dialect.numbers)
            .or_else(|| dialect.and_then(|query| self.control_numbers(Some(query)).syntax()))
            .or_else(|| self.profile().map(|profile| profile.grammar.numbers));
        let words = words.with_profile(self.profile());
        let dialect = dialect.or_else(|| {
            words
                .arguments()
                .dialect()
                .and_then(crate::InvocationDialect::authoring_query)
        });
        let Some(name) = words.head_literal() else {
            return StructuredInvocationResolution::from_unresolved(
                InvocationResolutionUnresolved::ComputedHead {
                    word_kind: words.head().kind(),
                },
            );
        };
        let spec = if dialect.is_none() {
            self.get(name)
        } else {
            self.get_for_surface(name, dialect)
        };
        let Some(spec) = spec else {
            return StructuredInvocationResolution::from_unresolved(
                InvocationResolutionUnresolved::UnknownLiteralHead { spelling: name },
            );
        };
        let availability = self.source_descriptor_availability(spec, dialect);
        StructuredInvocationResolution {
            invocation: Some(resolve_source_descriptor_invocation(
                spec,
                words,
                availability,
                numbers,
            )),
            unresolved: None,
        }
    }

    /// The one resolution the derived-query layer projects from
    /// (`docs/design/compiler/registry-consumer-contracts.md` § *The
    /// derived-query layer*): `words` resolved under the surface query
    /// `ctx` fixes — its target profile's — so every query the resolution
    /// then answers (`clause_plan`, `option_effects`, `arg_roles`,
    /// `pattern_args`, `case_invocation`, `frame_effect`, `return_type`,
    /// `effects`, …) is asked under that one release. A context that names
    /// no profile resolves surface-blind.
    #[must_use]
    pub fn invocation<'r, 'w>(
        &'r self,
        words: InvocationWords<'w>,
        ctx: &crate::value_transfer::AnalysisContext,
    ) -> StructuredInvocationResolution<'r, 'w> {
        self.resolve_structured_invocation(words, ctx.surface_query())
    }

    /// Resolve a concrete call to its registry-described form.
    ///
    /// Given the command head `name` and the literal argument words
    /// `args`, returns a [`ResolvedCall`] describing which
    /// [`CommandSpec`] / [`SubCommand`] / [`CommandForm`] the call
    /// matches and which lowering / codegen hook applies. The
    /// returned reference borrows from the registry, so callers must
    /// not retain it across registry mutation.
    ///
    /// Returns `None` when the command is unknown to the registry.
    #[must_use]
    pub fn resolve_call<'r>(
        &'r self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'r>>,
    ) -> Option<ResolvedCall<'r>> {
        let selection = self.resolve_legacy_call_selection(name, args, dialect)?;
        let spec = selection.spec;
        let sub = selection.sub;
        let form = selection.form;

        // A stamp is read at the point the call is resolved at: a level whose
        // windows that point does not settle declines, and the call is
        // dispatched plain rather than by the level above's stamp.
        let point = dialect.as_ref();
        let command_codegen = spec.codegen_hook_selection(point);
        let command_inline = spec.inline_codegen_hook_selection(point);
        let mut resolved = ResolvedCall {
            spec,
            sub,
            form,
            availability: selection.availability,
            lowering_hook: spec.lowering_hook,
            codegen_hook: command_codegen.stamp(),
            inline_codegen_hook: command_inline.stamp(),
            analyser_hook: spec.analyser_hook,
        };

        if let Some(sub) = sub {
            resolved.lowering_hook = form
                .and_then(|f| f.lowering_hook)
                .or(sub.lowering_hook)
                .or(spec.lowering_hook);
            resolved.codegen_hook = StampSelection::stated(form.and_then(|f| f.codegen_hook))
                .or(sub.codegen_hook_selection(point))
                .or(command_codegen)
                .stamp();
            // Forms carry no inline hook — the inline emitters guard
            // their own applicability (arity / shape) at the dispatch
            // site, so subcommand-level wins over command-level.
            resolved.inline_codegen_hook = sub
                .inline_codegen_hook_selection(point)
                .or(command_inline)
                .stamp();
            // Forms carry no analyser hook either — the analyser
            // handlers keep their own shape guards, so the
            // subcommand-level stamp wins over the command-level one.
            resolved.analyser_hook = sub.analyser_hook.or(spec.analyser_hook);
            return Some(resolved);
        }

        if let Some(f) = form {
            resolved.lowering_hook = f.lowering_hook.or(spec.lowering_hook);
            resolved.codegen_hook = StampSelection::stated(f.codegen_hook)
                .or(command_codegen)
                .stamp();
            resolved.form = Some(f);
        }
        Some(resolved)
    }

    /// Select raw registry descriptors for the legacy call resolver.
    ///
    /// The exact subcommand lookup intentionally preserves the long-standing
    /// `resolve_call` compatibility behaviour.  The common
    /// [`ResolvedInvocation`] resolver instead uses
    /// [`CommandSpec::resolve_subcommand_word`] and records a typed outcome.
    fn resolve_legacy_call_selection<'r>(
        &'r self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'r>>,
    ) -> Option<InvocationSelection<'r>> {
        let spec = if dialect.is_none() {
            self.get(name)?
        } else {
            self.get_for_surface(name, dialect)?
        };

        let availability = crate::resolved_invocation::InvocationAvailability {
            query: dialect.or_else(|| self.own_surface_query()),
            package_version: self.package_floor_for_spec(spec),
        };

        if !spec.subcommands.is_empty()
            && let Some(first) = args.first()
            && let Some(sub) = spec.subcommand(first)
        {
            return Some(InvocationSelection {
                spec,
                sub: Some(sub),
                form: pick_form(
                    spec,
                    Some(sub),
                    InvocationArguments::literals(args),
                    availability,
                ),
                availability,
            });
        }

        Some(InvocationSelection {
            spec,
            sub: None,
            form: pick_form(
                spec,
                None,
                InvocationArguments::literals(args),
                availability,
            ),
            availability,
        })
    }

    /// Resolve the option-terminator profile for a command invocation.
    ///
    /// Matches the invocation's first argument against subcommands
    /// that declare an [`OptionSpec`](crate::hover::OptionSpec) with
    /// `name == "--"`, then falls back to form-level `--` declarations.
    /// Returns `None` when the command does not declare a `--`
    /// terminator at all (subcommand-scoped or form-scoped).
    ///
    /// Drives the W304 ("missing option terminator") diagnostic — the
    /// returned `scan_start` index, `subcommand` label, and
    /// `options` slice tell the caller where to start scanning for
    /// the first positional argument and which option specs to
    /// consult for value-consuming options (so they're not mistaken
    /// for positionals).  Returning a borrowed `&'static [OptionSpec]`
    /// rather than a freshly-allocated `HashSet` keeps the resolver
    /// allocation-free on the analyser hot path; per-command option
    /// counts are small (typically 1-3 value-consuming options per
    /// command), so a linear scan at the call site is cheaper than
    /// a `HashSet` build.
    ///
    #[must_use]
    pub fn resolve_option_terminator(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<ResolvedTerminator> {
        let spec = if dialect.is_none() {
            self.get(name)?
        } else {
            self.get_for_surface(name, dialect)?
        };

        // Subcommand-scoped first. Resolve the subcommand word the same way
        // ensemble dispatch does — accepting a unique prefix abbreviation
        // (`string le` ⇒ `length`) — so an abbreviated subcommand keeps its
        // `--` terminator profile, matching `arg_indices_for_role`.
        if let Some(first) = args.first()
            && let Some(sub) = if dialect.is_none() {
                spec.resolve_subcommand(first)
            } else {
                spec.resolve_subcommand_for_dialect(first, dialect)
            }
            && sub.options.iter().any(|o| o.name == "--")
        {
            return Some(ResolvedTerminator {
                scan_start: 1,
                subcommand: Some(sub.name),
                options: sub.options,
                reserved_trailing_words: 0,
            });
        }

        // Form-level fallback — option specs live at the
        // `CommandSpec.options` level (a single set per spec), so we
        // consult that directly when no subcommand match was found.
        if spec.options.iter().any(|o| o.name == "--") {
            let effective_dialect = self.own_surface_query().or(dialect);
            let reserved_trailing_words =
                spec.case_list.map_or(spec.reserved_trailing_words, |case| {
                    case.option_scan_reserved_trailing_words(
                        args,
                        effective_dialect,
                        spec.reserved_trailing_words,
                    )
                });
            return Some(ResolvedTerminator {
                scan_start: 0,
                subcommand: None,
                options: spec.options,
                reserved_trailing_words,
            });
        }

        None
    }

    /// The **command-table transitions** one invocation establishes — the
    /// one vocabulary for "this call changed the command table".
    ///
    /// Every consumer that models command-name bindings reads this: the
    /// flow-sensitive lattice, the lowerer's alias table, the analyser's
    /// rename / alias records, the realm scan, the taint walk. None of them
    /// dispatches on a coarse effect word and re-destructures the argument
    /// layout for itself any more — the layout lives with the registry
    /// resolver, once, and a dynamic operand becomes a typed
    /// [`TransitionSubject::Unknown`] or a domain widening rather than each
    /// consumer's own `is_dynamic_word` test.
    ///
    /// Selection is the ordinary
    /// [`Self::resolve_structured_invocation`] at **this registry's own
    /// point**: command-table mutations are executable behaviour, so
    /// they must use the profile-visible command surface. In particular,
    /// iRules intentionally has no `rename` or `interp`, even though the
    /// dialect-agnostic catalogue knows their Tcl specs. A profile-less
    /// registry (`build_default`) answers dialect-agnostically, exactly as
    /// [`Self::get`] does.
    ///
    /// Reusing that primitive means a **subcommand-shaped** mutator now
    /// resolves by the ordinary ensemble rule rather than by exact
    /// spelling, which is a deliberate behavioural change from the retired
    /// `command_table_effect`: `interp ali {} a {} b` really does create an
    /// alias in C Tcl (`interp` is an ensemble), and the retired resolver's
    /// exact-word lookup made it invisible to every binding consumer.
    /// `interp aliases` is its own subcommand and still states nothing.
    ///
    /// The explicitly global spellings answer
    /// identically to the bare ones — `::rename format ::origfmt`,
    /// `::interp alias {} myfmt {} ::origfmt` and `::proc ::greet {} {…}`
    /// all really do mutate the command table (tclsh 9.0.4 and 8.6.16,
    /// byte-identical; `namespace which -command ::rename` → `::rename`).
    ///
    /// [`TransitionSubject::Unknown`]: crate::TransitionSubject::Unknown
    #[must_use]
    pub fn command_binding_transitions(&self, words: InvocationWords<'_>) -> StateTransitions {
        let query = self.own_surface_query();
        self.resolve_structured_invocation(words, query)
            .resolved()
            .map_or_else(StateTransitions::default, |invocation| {
                invocation.state_transitions()
            })
    }

    /// Whether `name` (or the compound key `"cmd sub"`) produces a
    /// canonical Tcl list — a list whose elements are properly
    /// quoted so re-parsing by ``eval`` / ``uplevel`` /
    /// ``interp eval`` doesn't trigger unwanted substitution.
    ///
    /// The canonical-list-command set is
    /// derived from `return_type == TclType::List` on the command
    /// (or its subcommand entry), minus the explicit exclusion
    /// `concat` whose join-strip-grouping semantics can leave
    /// unquoted specials in the output.
    ///
    /// Drives the W101 (`eval` with string concatenation) safe-
    /// idiom suppression — `eval [list ...]`, `eval [linsert ...]`,
    /// `eval [split ...]`, etc. shouldn't fire because the inner
    /// command's output is a properly-quoted list.
    #[must_use]
    pub fn is_canonical_list_command(&self, name: &str) -> bool {
        // Exclusion: concat returns LIST but isn't canonical.
        if name == "concat" {
            return false;
        }
        // Compound form ``"cmd sub"`` — split into head + sub.
        if let Some((head, sub_name)) = name.split_once(' ') {
            if let Some(spec) = self.get(head)
                && let Some(sub) = spec.subcommand(sub_name)
            {
                return sub.return_type == Some(crate::types::TclType::List);
            }
            return false;
        }
        // Bare command name.
        self.get(name)
            .and_then(|spec| spec.return_type)
            .is_some_and(|t| t == crate::types::TclType::List)
    }

    /// Whether `name` runs one of its body arguments more than once —
    /// [`Traits::HAS_LOOP_BODY`].
    ///
    /// The single answer for a consumer that needs "is this a loop?" without
    /// naming `while` / `for` / `foreach`, so a pack-declared loop command
    /// joins loop analysis with no consumer edit. Nothing else in the model
    /// says it: `NEVER_INLINE_BODY` marks a different, non-coextensive set —
    /// `timerate` and `array for` are loops without it, and consumers read the
    /// command's bits unioned with the resolved subcommand's, so `dict for`
    /// carries it from `dict` itself.
    #[must_use]
    pub fn is_loop_command(&self, name: &str) -> bool {
        self.get(name)
            .is_some_and(|spec| spec.traits.contains(Traits::HAS_LOOP_BODY))
    }

    /// Positions selected by the definer's Native argv-shape grammar.
    /// An actual argument dialect must identify that grammar; bare catalogue
    /// roles are available separately through authored source queries.
    ///
    /// The indices count words after the head, including Jim's optional
    /// static-variable list. This shape does not validate formal contents,
    /// install a procedure, or admit its body for execution.
    #[must_use]
    pub fn procedure_definition_words(&self, head: &str, args: &[&str]) -> Option<ProcedureWords> {
        let arguments = InvocationArguments::literals(args).with_profile(self.profile());
        let selected = self
            .resolve_structured_invocation(
                InvocationWords::from_arguments(InvocationWord::Literal(head), arguments),
                self.own_surface_query(),
            )
            .resolved()?;
        if !selected
            .semantics
            .traits
            .contains(Traits::DEFINES_PROCEDURE)
        {
            return None;
        }
        let crate::native_procedure::NativeProcedureDefinitionSelection::Valid(definition) =
            selected.semantics.procedure_definition?.select(arguments)
        else {
            return None;
        };
        Some(ProcedureWords {
            name: definition.name_at,
            params: definition.parameters_at,
            statics: definition.statics_at,
            body: definition.body_at,
        })
    }

    /// `{command: BytePayloadSpec}` for every registered `<proto>::payload`
    /// byte-array command — the getter is a binary source and `<cmd> replace`
    /// a byte sink for the S110 byte-array-corruption check.
    ///
    /// Only commands actually loaded into this registry are returned, so the
    /// set is implicitly gated by the active dialect: a plain-Tcl registry
    /// (no iRules pack loaded) yields an empty map — the payload layouts
    /// are intersected with the active dialect.
    #[must_use]
    pub fn byte_array_payload_layouts(&self) -> HashMap<&'static str, BytePayloadSpec> {
        let mut out = HashMap::new();
        for specs in self.by_name.values() {
            for spec in specs {
                if let Some(layout) = spec.byte_array_payload {
                    out.insert(spec.name, layout);
                }
            }
        }
        out
    }

    /// Number of registered commands.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}

/// Resolved option-terminator profile for a command invocation.
///
/// Returned by [`CommandRegistry::resolve_option_terminator`].  Drives
/// the W304 ("missing option terminator") diagnostic.  Carries
/// borrowed references into the registry's static spec table; do
/// not retain across registry mutation.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedTerminator {
    /// Index in the `args` slice where positional-argument scanning
    /// begins.  `0` for form-level matches; `1` for subcommand-scoped
    /// matches (the first arg is the subcommand keyword).
    pub scan_start: usize,
    /// Subcommand keyword that owns the `--` declaration, if the
    /// match was subcommand-scoped.  `None` for form-level matches.
    pub subcommand: Option<&'static str>,
    /// Borrowed slice of every option declared on the matched
    /// command (or subcommand).  Callers consult [`crate::hover::OptionSpec::takes_value`]
    /// on each entry to determine whether an option name consumes a
    /// following value argument — done at the call site to avoid the
    /// per-resolve `HashSet` allocation a precomputed name set would
    /// require.  Per-command counts are small; a linear scan is
    /// cheaper than a heap-allocated set on the analyser hot path.
    pub options: &'static [crate::hover::OptionSpec],
    /// Trailing words (after the command name) that are never scanned as
    /// option candidates — see [`crate::spec::CommandSpec::reserved_trailing_words`].
    /// `0` for every subcommand-scoped match (no subcommand currently
    /// needs the reservation) and for any form without one declared.
    pub reserved_trailing_words: usize,
}

/// One format-string word of a concrete call — the answer
/// [`CommandRegistry::format_string_args`] returns.
///
/// The two facts a consumer needs are deliberately separate: `kind` names the
/// mini-language (a `clock` field string and a `format` %-string share
/// neither syntax nor version gates), while `scan` says which *direction* it
/// is written in (`format` and `scan` share the `Sprintf` family but not its
/// conversion set — `%b` is 8.6+ in both, `%p` is a `format`-only Tcl 9
/// addition). Collapsing them would make a consumer guess one from the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FormatStringArg {
    /// 0-based index into the post-head argument list.
    pub index: usize,
    /// Which mini-language the word is written in.
    pub kind: crate::patterns::FormatType,
    /// The word is a **scan**-direction spec ([`ArgRole::ScanFormat`]) rather
    /// than a format-direction one ([`ArgRole::FormatString`]).
    pub scan: bool,
}

/// Which `TclOO` method-context keyword a command word is — the answer
/// [`CommandRegistry::method_dispatch_keyword`] returns.
///
/// The three variants are three different axes, deliberately not merged: a
/// consumer that wants "does the next word name a method" wants
/// [`Self::SelfDispatch`] alone, and one that wants "is this a call into the
/// method chain" wants `SelfDispatch | NextChain` but never
/// [`Self::Introspection`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MethodDispatchKind {
    /// `my` — dispatches a method on the current object; the following word
    /// is a method name on the enclosing class
    /// ([`Traits::TCLOO_SELF_DISPATCH`]).
    SelfDispatch,
    /// `next` / `nextto` — invokes the next implementation of the
    /// *currently executing* method along the receiver's method resolution
    /// order, so no word names a method. `nextto`'s first word names the
    /// *class* to resume from, marked [`ArgRole::Name`] on the spec
    /// ([`Traits::TCLOO_NEXT_CHAIN`]).
    NextChain,
    /// `self` — introspects the current invocation and dispatches nothing;
    /// its argument is a closed subcommand set, never a method name
    /// ([`Traits::TCLOO_INTROSPECTION`]).
    Introspection,
}

/// Backend hooks of a typed native compiler invocation, without effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeCompilerHooks {
    /// Structured lowering selected by the original compiler words.
    pub lowering: Option<LoweringHookId>,
    /// Statement emitter selected by the original compiler words.
    pub codegen: Option<CodegenHookId>,
    /// Value emitter selected by the original compiler words.
    pub inline: Option<InlineCodegenHookId>,
}

/// Outcome of [`CommandRegistry::resolve_call`].
///
/// Carries borrowed references into the registry's spec table; the
/// resolved call describes the matched command spec, optionally a
/// matched subcommand and form, and the effective lowering / codegen
/// hook identifiers (form-level wins over subcommand-level wins
/// over command-level).
#[derive(Debug, Clone, Copy)]
pub struct ResolvedCall<'r> {
    /// The matched top-level command spec.
    pub spec: &'r CommandSpec,
    /// The matched subcommand, if the call has one.
    pub sub: Option<&'r SubCommand>,
    /// The matched form descriptor, if any.
    pub form: Option<&'r CommandForm>,
    /// Option availability retained by the shared selection owner.
    pub availability: crate::resolved_invocation::InvocationAvailability<'r>,
    /// Effective lowering hook identifier.
    pub lowering_hook: Option<LoweringHookId>,
    /// Effective codegen hook identifier.
    pub codegen_hook: Option<CodegenHookId>,
    /// Effective inline (value-position / catch-body) codegen hook
    /// identifier (subcommand-level wins over command-level).
    pub inline_codegen_hook: Option<InlineCodegenHookId>,
    /// Effective analyser handler-family hook identifier
    /// (subcommand-level wins over command-level).
    pub analyser_hook: Option<AnalyserHookId>,
}

/// Raw descriptor selection shared by the legacy and target-neutral
/// invocation-resolution facades.
#[derive(Debug, Clone, Copy)]
struct InvocationSelection<'r> {
    spec: &'r CommandSpec,
    sub: Option<&'r SubCommand>,
    form: Option<&'r CommandForm>,
    availability: crate::resolved_invocation::InvocationAvailability<'r>,
}

impl ResolvedCall<'_> {
    /// Effective arity for this resolved call: the form arity if a
    /// form matched, otherwise the subcommand arity, otherwise the
    /// top-level [`CommandSpec`] arity.
    #[must_use]
    pub fn arity(&self) -> Arity {
        if let Some(f) = self.form {
            return f.arity;
        }
        if let Some(s) = self.sub {
            return s.arity;
        }
        self.spec.arity
    }

    /// Effective native arity using the actual retained argument dialect.
    /// A selected procedure-definition contract may differ from the legacy
    /// catalogue synopsis, for example Jim's optional statics operand.
    #[must_use]
    pub fn arity_for_arguments(&self, arguments: crate::InvocationArguments<'_>) -> Arity {
        self.spec
            .procedure_definition
            .filter(|_| self.sub.is_none())
            .map_or_else(
                || {
                    crate::native_list_assignment::operation_arity(
                        crate::resolved_invocation::resolved_operation(
                            self.spec,
                            self.sub,
                            self.form,
                            self.availability.query.as_ref(),
                        ),
                        arguments.dialect(),
                    )
                    .unwrap_or_else(|| self.arity())
                },
                |descriptor| descriptor.arity(arguments.dialect()),
            )
    }

    /// Count selected signature operands through the common option-layout owner.
    #[must_use]
    pub fn argument_count_for_arity(
        &self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<u16> {
        let options = crate::resolved_invocation::invocation_options(
            self.spec,
            self.sub,
            self.form,
            self.availability,
        );
        crate::resolved_invocation::count_invocation_arguments(
            self.arity_for_arguments(arguments),
            arguments,
            usize::from(self.sub.is_some()),
            options,
        )
    }

    /// Effective [`VarWriteTyping`] for this resolved call: the matched
    /// subcommand's when one matched (`binary scan` destructures where the
    /// bare `binary` does not), otherwise the top-level [`CommandSpec`]'s.
    ///
    /// The compiler's type-inference pass consults this to type the
    /// variables a command writes as a side effect, rather than assuming
    /// they receive the command's return value.
    #[must_use]
    pub fn var_write_typing(&self) -> VarWriteTyping {
        self.sub
            .map_or(self.spec.var_write_typing, |s| s.var_write_typing)
    }

    /// The result↔element-structure fact for this call — the subcommand's
    /// when one matched, else the command's. See
    /// [`crate::types::ReturnElements`].
    #[must_use]
    pub fn return_elements(&self) -> Option<crate::types::ReturnElements> {
        self.sub
            .map_or(self.spec.return_elements, |s| s.return_elements)
    }

    /// The in-place element evolution of the written variable for this call —
    /// the subcommand's when one matched, else the command's. See
    /// [`crate::types::VarElementsEffect`].
    #[must_use]
    pub fn var_elements_effect(&self) -> Option<crate::types::VarElementsEffect> {
        self.sub
            .map_or(self.spec.var_elements_effect, |s| s.var_elements_effect)
    }

    /// The clause plan of this call: the matched subcommand's grammar walked
    /// over the words after the subcommand word, else the command's over
    /// `args`, in the post-head coordinates of `args` — the plan
    /// [`CommandRegistry::clause_plan`] answers for the same descriptors.
    /// `args` are source spellings (a braced `{-}` is the fall-through
    /// marker). `None` when neither declares a grammar or it is unavailable
    /// at `dialect`.
    #[must_use]
    pub fn clause_plan(
        &self,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<crate::ClausePlan> {
        match self.sub {
            Some(sub) => sub
                .clause_plan(args.get(1..).unwrap_or_default(), dialect)
                .map(|plan| plan.offset_by(1)),
            None => self.spec.clause_plan(args, dialect),
        }
    }
}

fn successful_handler_contracts(
    spec: &CommandSpec,
) -> impl Iterator<Item = crate::native_compilation::SuccessfulHandlerSpec> + '_ {
    spec.successful_handler
        .into_iter()
        .chain(
            spec.command_forms
                .iter()
                .filter_map(|form| form.successful_handler),
        )
        .chain(spec.subcommands.iter().flat_map(|member| {
            member.successful_handler.into_iter().chain(
                member
                    .subcommand_forms
                    .iter()
                    .filter_map(|form| form.successful_handler),
            )
        }))
}

/// Select the [`CommandForm`] whose arity, dialect, and optional literal
/// argument-prefix selector match the invocation.
///
/// **No lifecycle filter, because there is nothing to filter on.**
/// [`CommandForm`] — the *semantic* form, which routes lowering / codegen
/// hooks and per-form descriptors — carries no [`crate::lifecycle::Lifecycle`]
/// field; only its documentation counterpart [`crate::hover::FormSpec`] does,
/// and that is what [`CommandSpec::primary_synopsis`] and
/// [`CommandSpec::optional_trailing_arg_names`] gate. If a semantic form ever
/// gains a lifecycle, this is where the same `package_version: Option<&str>`
/// parameter belongs — the retained availability already carries the selected package floor.
fn pick_form<'r>(
    spec: &'r CommandSpec,
    sub: Option<&'r SubCommand>,
    arguments: InvocationArguments<'_>,
    availability: crate::resolved_invocation::InvocationAvailability<'_>,
) -> Option<&'r CommandForm> {
    let forms = sub.map_or(spec.command_forms, |sub| sub.subcommand_forms);
    let argument_offset = usize::from(sub.is_some());
    let argument_count = arguments.exact_argv_len()?.checked_sub(argument_offset)?;
    let dialect = availability.query;
    let accepts = |form: &CommandForm| {
        crate::resolved_invocation::count_invocation_arguments(
            form.arity,
            arguments,
            argument_offset,
            crate::resolved_invocation::invocation_options(spec, sub, Some(form), availability),
        )
        .is_some_and(|count| form.arity.accepts(count))
    };
    let admits_dialect = |form: &CommandForm| !matches!(form.surface, Some(d) if dialect.is_some() && !surface_admits(d, dialect.as_ref()));
    // Resolve each selector word as its own Tcl keyword table. This matters
    // for nested operations: `tag c ...` is ambiguous between `cell` and
    // `configure` before a later word can be inspected, while `tag cell h`
    // first resolves `cell` exactly and then uniquely prefixes `has`.
    //
    // Keep walking after a selector completes: a sibling may extend it
    // (`tag` beside `tag bind`), and the longest statically matched selector
    // wins. A dynamic word while such an extension remains viable abstains;
    // it may turn out to be the longer selector at runtime.
    let mut active: Vec<&CommandForm> = forms
        .iter()
        .filter(|form| admits_dialect(form) && form.literal_argument_prefix.is_some())
        .collect();
    let mut selected = None;
    for selector_index in 0..argument_count {
        active.retain(|form| {
            form.literal_argument_prefix
                .is_some_and(|selector| selector.words.len() > selector_index)
        });
        if active.is_empty() {
            break;
        }
        let spelling = arguments.literal_at(argument_offset + selector_index)?;
        let mut has_exact_word = false;
        let mut abbreviated_word = None;
        let mut abbreviated_ambiguous = false;

        for form in &active {
            let selector = form
                .literal_argument_prefix
                .expect("active forms have literal selectors");
            let canonical = selector.words[selector_index];
            if spelling == canonical {
                has_exact_word = true;
            } else if selector.prefix_matching.accepts_prefixes()
                && !spelling.is_empty()
                && canonical.starts_with(spelling)
            {
                if abbreviated_word.is_some_and(|word| word != canonical) {
                    abbreviated_ambiguous = true;
                } else {
                    abbreviated_word = Some(canonical);
                }
            }
        }

        let canonical = if has_exact_word {
            spelling
        } else if abbreviated_ambiguous {
            return None;
        } else if let Some(abbreviated) = abbreviated_word {
            abbreviated
        } else {
            break;
        };
        active.retain(|form| {
            let selector = form
                .literal_argument_prefix
                .expect("active forms have literal selectors");
            selector.words[selector_index] == canonical
                && (spelling == canonical || selector.prefix_matching.accepts_prefixes())
        });
        if let Some(form) = active.iter().copied().find(|form| {
            accepts(form)
                && form
                    .literal_argument_prefix
                    .is_some_and(|selector| selector.words.len() == selector_index + 1)
        }) {
            selected = Some(form);
        }
    }
    if selected.is_some() {
        return selected;
    }

    let selector_overlaps_arity = forms.iter().any(|form| {
        admits_dialect(form) && accepts(form) && form.literal_argument_prefix.is_some()
    });
    if selector_overlaps_arity {
        return None;
    }
    forms.iter().find(|form| {
        admits_dialect(form) && accepts(form) && form.literal_argument_prefix.is_none()
    })
}

/// Resolve readonly invocation metadata from an independently selected descriptor.
/// The caller owns descriptor selection and actual availability. This does not
/// select a command by name or grant a native handler, entered frame or effect.
/// Registered and scoped source descriptors share this selector/form owner.
#[must_use]
pub fn resolve_source_descriptor_invocation<'r, 'w>(
    spec: &'r CommandSpec,
    words: InvocationWords<'w>,
    availability: crate::resolved_invocation::InvocationAvailability<'w>,
    numbers: Option<tcl_dialect::NumberSyntax>,
) -> ResolvedInvocation<'r, 'w> {
    let arguments = words.arguments();
    let (subcommand, sub) = resolve_semantic_subcommand(
        spec,
        arguments,
        availability.query,
        availability.package_version,
        numbers,
    );
    let form = match (sub, spec.subcommands.is_empty(), arguments.is_empty()) {
        (Some(sub), _, _) => pick_form(spec, Some(sub), arguments, availability),
        (None, true, _) | (None, false, true) => pick_form(spec, None, arguments, availability),
        (None, false, false) if subcommand == SubcommandResolution::NotApplicable => {
            pick_form(spec, None, arguments, availability)
        }
        (None, false, false) => None,
    };
    ResolvedInvocation::new(words, spec, sub, form, subcommand, availability)
}

/// Resolve a subcommand for the common semantic path.
///
/// The registry's keyword table determines prefix legality.  The legacy
/// [`ResolvedCall`] path intentionally remains exact-only because its callers
/// preserve historical hook-routing behaviour.
fn resolve_semantic_subcommand<'r, 'w>(
    spec: &'r CommandSpec,
    arguments: InvocationArguments<'w>,
    dialect: Option<SurfaceQuery<'_>>,
    package_version: Option<&str>,
    numbers: Option<tcl_dialect::NumberSyntax>,
) -> (SubcommandResolution<'w>, Option<&'r SubCommand>) {
    if spec.subcommands.is_empty()
        || arguments.is_empty()
        || spec.constructor_prefix_words().is_some()
    {
        return (SubcommandResolution::NotApplicable, None);
    }

    let Some(spelling) = arguments.literal_at(0) else {
        let word_kind = arguments.get(0).map_or(
            crate::InvocationWordKind::Opaque,
            crate::InvocationWord::kind,
        );
        return (SubcommandResolution::Indeterminate { word_kind }, None);
    };
    if spec
        .default_form_first_word
        .is_some_and(|shape| shape.matches_syntax(spelling, numbers))
    {
        return (SubcommandResolution::NotApplicable, None);
    }
    let matched = spec.resolve_subcommand_word(spelling, dialect, package_version, None);
    match matched {
        crate::abbrev::KeywordMatch::Unique(canonical_name) => {
            // `resolve_subcommand_word` builds its table from `spec` itself,
            // so its canonical result is necessarily one of these entries.
            let sub = spec
                .subcommand(canonical_name)
                .expect("registry subcommand table and descriptor slice agree");
            let resolved = ResolvedSubcommand {
                spelling,
                canonical_name: sub.name,
            };
            let outcome = if spelling == sub.name {
                SubcommandResolution::Exact(resolved)
            } else {
                SubcommandResolution::UniquePrefix(resolved)
            };
            (outcome, Some(sub))
        }
        crate::abbrev::KeywordMatch::Ambiguous(_) => {
            (SubcommandResolution::Ambiguous { spelling }, None)
        }
        crate::abbrev::KeywordMatch::Unknown => (SubcommandResolution::Unknown { spelling }, None),
    }
}

impl std::fmt::Debug for CommandRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandRegistry")
            .field(
                "snapshot",
                &self
                    .snapshot
                    .get()
                    .map(|snapshot| snapshot.0.semantic_key.0.fingerprint),
            )
            .field("commands", &self.by_name.len())
            .field("overlay_specs", &self.overlay_specs.len())
            .field("loaded_layers", &self.loaded_layers)
            .field("profile", &self.profile.map(|p| p.name))
            .field("ambient_packages", &self.ambient_packages)
            .field("own_packages", &self.own_packages)
            .field("special_vars", &self.special_vars)
            .field(
                "document_grammar",
                &self.document_grammar.map(|g| g.members.len()),
            )
            .field(
                "effective_semantics_cached",
                &self.effective_semantics.get().is_some(),
            )
            .field("overlay", &self.overlay)
            .field("pack_origins", &self.pack_origins.len())
            .field("reference_texts", &self.reference_texts.len())
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::{CommandRegistry, MethodDispatchKind};
    use crate::ArgRole;
    use crate::forms::LiteralArgumentPrefix;
    use crate::spec::ArgRoleResolver;
    use std::collections::BTreeSet;
    use std::sync::Arc;
    use tcl_dialect::DialectProfile;
    use tcl_dialect::model::SpecProvider;
    use tcl_dialect::model::{Family, SpecSurface, SurfaceLayer, SurfaceQuery};
    use tcl_dialect::surface;

    #[test]
    fn native_registration_cache_preserves_every_dialect_and_frozen_generation() {
        let mut registry = CommandRegistry::build_default();
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = crate::InvocationDialect::for_version(version);
            for key in [
                "set",
                "tcl::info::exists",
                "tcl::namespace::origin",
                "missing::worker",
            ] {
                let expected = registry.native_compilation_for_registration_uncached(key, dialect);
                assert_eq!(
                    registry.native_compilation_for_registration(key, dialect),
                    expected
                );
                assert_eq!(
                    registry.native_compilation_for_registration(&format!("::{key}"), dialect),
                    expected
                );
                assert_eq!(
                    registry.native_registration_lookup(key, dialect),
                    registry.native_registration_lookup_uncached(key, dialect)
                );
            }
        }
        let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        let snapshot = registry.snapshot();
        let previous = registry.native_compilation_for_registration("set", dialect);
        assert!(previous.is_some());
        let mut replacement = registry.get("set").unwrap().clone();
        replacement.native_compilation = None;
        registry.insert(replacement);
        assert!(
            registry
                .native_compilation_for_registration("set", dialect)
                .is_none()
        );
        assert_eq!(
            snapshot
                .registry()
                .native_compilation_for_registration("set", dialect),
            previous
        );
        assert!(!Arc::ptr_eq(
            &registry.native_registrations,
            &snapshot.registry().native_registrations
        ));
    }

    #[test]
    fn representation_hint_requires_only_the_live_option_prefix() {
        use crate::InvocationWord::{Dynamic, Literal};
        let context = crate::model::ingress::static_context_for("tcl8.6");
        let registry = context.commands();
        for (arguments, subject) in [
            (vec![Literal("map"), Literal("a b"), Dynamic], 2),
            (
                vec![Literal("map"), Literal("-nocase"), Literal("a b"), Dynamic],
                3,
            ),
        ] {
            let words = crate::InvocationWords::structured(Literal("string"), &arguments)
                .with_dialect(crate::InvocationDialect::for_version(
                    tcl_dialect::TclVersion::V8_6,
                ));
            assert_eq!(
                registry
                    .arg_type_hint_words(words, subject)
                    .unwrap()
                    .expected,
                Some(crate::TclType::String)
            );
        }
        let arguments = [Literal("map"), Dynamic, Dynamic];
        let words = crate::InvocationWords::structured(Literal("string"), &arguments).with_dialect(
            crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        assert!(registry.arg_type_hint_words(words, 2).is_none());
    }

    #[test]
    fn form_selection_counts_available_options_on_the_declared_axis() {
        static OPTIONS: [crate::hover::OptionSpec; 2] = [
            crate::hover::OptionSpec {
                name: "-later",
                surface: Some(SpecSurface::TCL91),
                ..crate::hover::OptionSpec::DEFAULT
            },
            crate::hover::OptionSpec {
                name: "-package",
                lifecycle: crate::lifecycle::Lifecycle::introduced_in("2.0"),
                ..crate::hover::OptionSpec::DEFAULT
            },
        ];
        static FORMS: [crate::forms::CommandForm; 1] = [crate::forms::CommandForm {
            name: "positional",
            arity: crate::Arity::exact(1).with_positionals(),
            options: &OPTIONS,
            ..crate::forms::CommandForm::DEFAULT
        }];
        let mut registry = CommandRegistry::build_default();
        registry.insert(crate::CommandSpec {
            name: "counted",
            required_package: Some("Counted"),
            command_forms: &FORMS,
            ..crate::CommandSpec::DEFAULT
        });
        for (release, args, selected) in [
            ("9.0", &["-later", "x"][..], false),
            ("9.1", &["-later", "x"][..], true),
            ("9.1", &["-package", "x"][..], true),
        ] {
            let resolved = registry
                .resolve_invocation(
                    "counted",
                    args,
                    Some(SurfaceQuery::core(Family::Tcl, release)),
                )
                .unwrap();
            assert_eq!(resolved.form.is_some(), selected, "{release} {args:?}");
        }
        registry.insert_ambient_package("Counted", "1.0");
        assert!(
            registry
                .resolve_invocation(
                    "counted",
                    &["-package", "x"],
                    Some(SurfaceQuery::core(Family::Tcl, "9.1"))
                )
                .unwrap()
                .form
                .is_none()
        );
        registry.insert_ambient_package("Counted", "2.0");
        let resolved = registry
            .resolve_invocation(
                "counted",
                &["-package", "x"],
                Some(SurfaceQuery::core(Family::Tcl, "9.1")),
            )
            .unwrap();
        assert_eq!(resolved.argument_count_for_arity(), Some(1));
        assert_eq!(resolved.form.map(|form| form.name), Some("positional"));
    }

    #[test]
    fn default_form_integer_uses_the_retained_invocation_grammar() {
        static SUB: [crate::SubCommand; 1] = [crate::SubCommand {
            name: "replace",
            ..crate::SubCommand::DEFAULT
        }];
        static FORMS: [crate::forms::CommandForm; 1] = [crate::forms::CommandForm {
            name: "getter",
            arity: crate::Arity::exact(1),
            ..crate::forms::CommandForm::DEFAULT
        }];
        let mut registry = CommandRegistry::build_default();
        registry.insert(crate::CommandSpec {
            name: "payload_fixture",
            default_form_first_word: Some(crate::spec::DefaultFormFirstWord::Integer),
            subcommands: &SUB,
            command_forms: &FORMS,
            ..crate::CommandSpec::DEFAULT
        });
        for (release, spelling, selected) in [
            ("8.4", "100", true),
            ("9.1", "0d100", true),
            ("8.4", "0d100", false),
        ] {
            let args = [spelling];
            let invocation = registry
                .resolve_invocation(
                    "payload_fixture",
                    &args,
                    Some(SurfaceQuery::core(Family::Tcl, release)),
                )
                .unwrap();
            assert_eq!(
                invocation.form.map(|form| form.name),
                selected.then_some("getter")
            );
        }
        let args = [crate::InvocationWord::Dynamic];
        let unresolved = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("payload_fixture"),
                    &args,
                ),
                Some(SurfaceQuery::core(Family::Tcl, "9.1")),
            )
            .resolved()
            .unwrap();
        assert!(unresolved.form.is_none());
        assert!(matches!(
            unresolved.subcommand,
            crate::SubcommandResolution::Indeterminate { .. }
        ));
    }

    #[test]
    fn private_registration_facts_keep_actual_operand_indices() {
        // Implementation contract: naming.invocation.effective-transition-operands
        // docs/design/analysis/name-resolution-proofs/effective-transition-operands.md

        let registry = CommandRegistry::build_default();
        let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_1);
        for (head, args, canonical) in [
            ("::tcl::info::exists", vec!["x"], "info"),
            ("::tcl::string::equal", vec!["a", "b"], "string"),
            ("::tcl::dict::set", vec!["d", "k", "v"], "dict"),
            ("::tcl::array::exists", vec!["a"], "array"),
        ] {
            let words = crate::InvocationWords::literals(head, &args).with_dialect(dialect);
            let facts = registry
                .native_registration_invocation_facts(words)
                .unwrap();
            assert_eq!(facts.canonical_command, canonical, "{head}");
            assert_eq!(facts.argument_offset, 0, "{head}");
            assert!(
                registry
                    .native_registration_completion_route(
                        words,
                        crate::VariableAliasFrame::Procedure
                    )
                    .is_some(),
                "{head}"
            );
            if let crate::native_compilation::NativeCompilationGrammar::Dictionary {
                ensemble,
                ..
            } = facts.native_compilation.unwrap().grammar
            {
                assert!(!ensemble);
            }
        }
        assert!(
            registry
                .native_registration_invocation_facts(
                    crate::InvocationWords::literals("::tcl::info::missing", &[])
                        .with_dialect(dialect)
                )
                .is_none()
        );
        assert!(
            registry
                .native_registration_invocation_facts(
                    crate::InvocationWords::literals("::tcl::info::exists", &["x"]).with_dialect(
                        crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4)
                    )
                )
                .is_none()
        );
        let args = [crate::InvocationWord::DynamicNonOption];
        let words = crate::InvocationWords::structured(
            crate::InvocationWord::Literal("::tcl::info::exists"),
            &args,
        )
        .with_dialect(dialect);
        assert_eq!(
            registry
                .native_registration_invocation_facts(words)
                .unwrap()
                .argument_offset,
            0
        );
        for argument in [
            crate::InvocationWord::Literal("::A"),
            crate::InvocationWord::Dynamic,
        ] {
            let arguments = [argument];
            let words = crate::InvocationWords::structured(
                crate::InvocationWord::Literal("::tcl::namespace::path"),
                &arguments,
            )
            .with_dialect(dialect);
            let facts = registry
                .native_registration_invocation_facts(words)
                .unwrap();
            let path = facts
                .state_transitions
                .declared()
                .unwrap()
                .facts()
                .iter()
                .find_map(|fact| {
                    if let crate::StateTransition::Namespace(
                        crate::NamespaceTransition::SetPath { path, .. },
                    ) = &fact.transition
                    {
                        Some(path)
                    } else {
                        None
                    }
                })
                .unwrap();
            assert_eq!(path.argument_index(), Some(0));
            assert_eq!(path.literal(), argument.literal());
        }
    }

    #[test]
    fn native_registration_intrinsics_are_member_metadata_without_placeholder_arguments() {
        let registry = CommandRegistry::build_default();
        let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        assert_eq!(
            registry.native_registration_intrinsic_ids("tcl::string::length", dialect),
            Some(vec![crate::IntrinsicId::StringLength])
        );
        assert_eq!(
            registry.native_registration_intrinsic_ids("tcl::string::index", dialect),
            Some(vec![crate::IntrinsicId::StringIndex])
        );
        assert!(
            registry
                .native_registration_intrinsic_ids("unregistered", dialect)
                .is_none()
        );
        let root = registry
            .native_registration_intrinsic_ids("string", dialect)
            .unwrap();
        assert!(root.contains(&crate::IntrinsicId::StringLength));
        assert!(root.contains(&crate::IntrinsicId::StringIndex));
    }

    #[test]
    fn original_registration_source_descriptor_retains_parent_and_override_provenance() {
        // Implementation contract: naming.namespace.original-quiet-pattern-completion
        // docs/design/analysis/name-resolution-proofs/namespace-original-quiet-pattern-completion.md
        let mut registry = CommandRegistry::build_default();
        let realm = tcl_dialect::model::InvocationRealm::RuleLoader;
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = crate::InvocationDialect::for_version(version);
            let root = registry
                .native_registration_source_descriptor("::namespace", dialect, realm)
                .unwrap();
            assert_eq!(root.name, "namespace");
            assert_eq!(super::spec_pack_of(root), Some("tcl"));
            let private = registry.native_registration_source_descriptor(
                "::tcl::namespace::export",
                dialect,
                realm,
            );
            if version < tcl_dialect::TclVersion::V8_6 {
                assert!(private.is_none(), "{version:?}");
            } else {
                assert!(std::ptr::eq(private.unwrap(), root), "{version:?}");
            }
            assert!(
                registry
                    .native_registration_source_descriptor(
                        "tcl::namespace::unregistered",
                        dialect,
                        realm
                    )
                    .is_none()
            );
        }
        let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let surface = registry
            .get_for_surface("namespace", dialect.authoring_query())
            .unwrap()
            .surface;
        registry.insert_static(Box::leak(Box::new(crate::CommandSpec {
            name: "tcl::namespace::export",
            surface,
            ..crate::CommandSpec::DEFAULT
        })));
        let authored = registry
            .native_registration_source_descriptor("tcl::namespace::export", dialect, realm)
            .unwrap();
        assert_eq!(authored.name, "tcl::namespace::export");
        assert_eq!(
            super::spec_pack_of(authored),
            None,
            "an explicit authored helper descriptor cannot borrow its stock parent's pack"
        );
        let unknown = crate::InvocationDialect {
            native_family: None,
            core_point: None,
            tcl_version: None,
            ..dialect
        };
        assert!(
            registry
                .native_registration_source_descriptor("namespace", unknown, realm)
                .is_none()
        );
    }

    #[test]
    fn string_repeat_registration_has_its_actual_basic_worker_compiler() {
        use tcl_dialect::TclVersion;
        let registry = CommandRegistry::build_default();
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let dialect = crate::InvocationDialect::for_version(version);
            let worker =
                registry.native_compilation_for_registration("tcl::string::repeat", dialect);
            if version == TclVersion::V8_4 {
                assert!(worker.is_none());
            } else {
                assert_eq!(
                    worker.unwrap().compiler_hook_presence(dialect),
                    Some(version >= TclVersion::V8_6)
                );
            }
        }
        let mut unknown = crate::InvocationDialect::for_version(TclVersion::V9_0);
        unknown.native_family = None;
        unknown.core_point = None;
        unknown.tcl_version = None;
        assert!(
            registry
                .native_compilation_for_registration("tcl::string::repeat", unknown)
                .is_none()
        );
    }

    #[test]
    fn native_registration_compilation_keeps_private_member_and_version() {
        let registry = CommandRegistry::build_default();
        for version in [
            tcl_dialect::TclVersion::V8_4,
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let dialect = crate::InvocationDialect::of_profile(
                tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
            );
            let private =
                registry.native_compilation_for_registration("tcl::info::exists", dialect);
            if version == tcl_dialect::TclVersion::V8_4 {
                assert!(private.is_none());
            } else {
                assert_eq!(
                    private.unwrap().grammar,
                    crate::native_compilation::NativeCompilationGrammar::InfoExists
                );
                assert_eq!(private.unwrap().compiler_hook_presence(dialect), Some(true));
                assert_eq!(
                    private,
                    registry.native_compilation_for_registration("::tcl::info::exists", dialect)
                );
            }
            assert!(
                registry
                    .native_compilation_for_registration("unknown::worker", dialect)
                    .is_none()
            );
        }
    }

    #[test]
    fn normal_worker_registration_retains_its_independent_compiler_descriptor() {
        let mut registry = CommandRegistry::build_default();
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let dialect = crate::InvocationDialect::for_version(version);
            let descriptor = registry
                .native_compilation_for_registration("tcl::namespace::origin", dialect)
                .expect("the original registered worker authors its own compiler hook");
            assert_eq!(descriptor.compiler_hook_presence(dialect), Some(true));
            assert_eq!(
                descriptor,
                registry
                    .native_compilation_for_registration("::tcl::namespace::origin", dialect)
                    .unwrap()
            );
            assert!(
                registry
                    .native_compilation_for_registration("tcl::namespace::unregistered", dialect)
                    .is_none()
            );
        }
        let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let unknown = crate::InvocationDialect {
            native_family: None,
            core_point: None,
            tcl_version: None,
            ..dialect
        };
        assert!(
            registry
                .native_compilation_for_registration("tcl::namespace::origin", unknown)
                .is_none()
        );
        let replacement_surface = registry
            .get_for_surface("namespace", Some(dialect.authoring_query().unwrap()))
            .unwrap()
            .surface;
        registry.insert_static(Box::leak(Box::new(crate::CommandSpec {
            name: "namespace",
            surface: replacement_surface,
            subcommands: &[crate::SubCommand {
                name: "origin",
                ..crate::SubCommand::DEFAULT
            }],
            ..crate::CommandSpec::DEFAULT
        })));
        assert!(
            registry
                .native_compilation_for_registration("tcl::namespace::origin", dialect)
                .is_none(),
            "replacement metadata supplies no original worker registration or compiler descriptor"
        );
    }

    #[test]
    fn nested_compiler_registration_retains_each_independent_path_row() {
        let registry = CommandRegistry::build_default();
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = crate::InvocationDialect::for_version(version);
            for (identity, hook) in [
                ("tcl::binary::encode", true),
                ("tcl::binary::decode", true),
                ("tcl::binary::encode::hex", true),
                ("tcl::binary::encode::base64", false),
                ("tcl::binary::encode::uuencode", false),
                ("tcl::binary::decode::hex", true),
                ("tcl::binary::decode::base64", true),
                ("tcl::binary::decode::uuencode", true),
            ] {
                let descriptor = registry.native_compilation_for_registration(identity, dialect);
                if version < tcl_dialect::TclVersion::V8_6 {
                    assert!(descriptor.is_none(), "{version:?}: {identity}");
                } else {
                    assert_eq!(
                        descriptor.and_then(|spec| spec.compiler_hook_presence(dialect)),
                        Some(hook),
                        "{version:?}: {identity}"
                    );
                    assert_eq!(
                        descriptor,
                        registry
                            .native_compilation_for_registration(&format!("::{identity}"), dialect)
                    );
                }
            }
        }
        let owner = crate::model::ingress::static_context_for("jim");
        let dialect = crate::InvocationDialect::of_profile(
            owner.commands().profile().expect("selected Jim fixture"),
        );
        assert!(
            registry
                .native_compilation_for_registration("tcl::binary::encode::hex", dialect)
                .is_none()
        );
    }

    #[test]
    fn conflicting_private_registration_mappings_retain_uncertainty() {
        use crate::native_compilation::{
            NativeBodyCompilation, NativeCompilationGrammar, NativeCompilationSpec,
            NativeCompilerImplementationLookup,
        };
        const FIRST: NativeCompilerImplementationLookup = NativeCompilerImplementationLookup {
            ensemble: "::alpha",
            member: "x",
            slot: "::shared_worker",
            command: "alpha",
            prepended: &[],
        };
        const SECOND: NativeCompilerImplementationLookup = NativeCompilerImplementationLookup {
            ensemble: "::beta",
            member: "x",
            slot: "::shared_worker",
            command: "beta",
            prepended: &[],
        };
        let mut registry = CommandRegistry::build_default();
        for (name, lookup) in [("alpha", &FIRST), ("beta", &SECOND)] {
            registry.insert_static(Box::leak(Box::new(crate::CommandSpec {
                name,
                arity: crate::Arity::exact(0),
                native_compilation: Some(NativeCompilationSpec {
                    grammar: NativeCompilationGrammar::NamedEnsembleInvocation {
                        lookup,
                        implementation_from: tcl_dialect::TclVersion::V8_6,
                        hook_from: tcl_dialect::TclVersion::V8_6,
                        arity: crate::Arity::exact(0),
                    },
                    operation: crate::SemanticOperationId::Invoke,
                    body: NativeBodyCompilation::Inherit,
                }),
                ..crate::CommandSpec::DEFAULT
            })));
        }
        let dialect = crate::InvocationDialect::of_profile(
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
        );
        assert!(
            registry
                .native_compilation_for_registration("shared_worker", dialect)
                .is_none()
        );
        assert!(
            registry
                .native_registration_invocation_facts(
                    crate::InvocationWords::literals("shared_worker", &[]).with_dialect(dialect)
                )
                .is_none()
        );
    }

    fn body_backed(name: &'static str, backing: crate::RuntimeBacking) -> crate::CommandSpec {
        crate::CommandSpec {
            name,
            runtime_backing: backing,
            ..crate::CommandSpec::DEFAULT
        }
    }

    fn text_backing(text: &'static str) -> crate::RuntimeBacking {
        crate::RuntimeBacking::pack_text(text)
    }

    fn origin() -> crate::pack_origin::PackOrigin {
        crate::pack_origin::PackOrigin {
            pack: "vendor".to_owned(),
            content_hash: 7,
            vocabulary_version: "2".to_owned(),
        }
    }

    /// The reference bodies a registry offers are the specs a pack installed,
    /// whose backing is a Tcl body this registry holds the text of, and which
    /// are still the answer for their name: the text a `PackText` carries or a
    /// `PackageSource` resolved to at load, never an embedder's own spec, a
    /// backing of another kind, a `PackageSource` nobody read, or a spec a later
    /// insertion shadowed.
    #[test]
    fn a_registry_offers_the_live_pack_installed_bodies_it_holds_text_for() {
        let installed = |registry: &mut CommandRegistry, spec: crate::CommandSpec| {
            let spec: &'static crate::CommandSpec = Box::leak(Box::new(spec));
            registry.insert_static(spec);
            registry.insert_pack_origin(spec, origin());
            spec
        };
        let mut registry = CommandRegistry::build_default();
        let in_text = installed(
            &mut registry,
            body_backed("vendor::text", text_backing("proc vendor::text {} {}")),
        );
        let from_file = installed(
            &mut registry,
            body_backed(
                "vendor::file",
                crate::RuntimeBacking::package_source("lib.tcl"),
            ),
        );
        registry.insert_reference_text(from_file, Arc::from("proc vendor::file {} {}"));
        // Not offered: nobody read the file; the host owns the command; the
        // embedder inserted it; and a later spec shadowed it.
        installed(
            &mut registry,
            body_backed(
                "vendor::unread",
                crate::RuntimeBacking::package_source("absent.tcl"),
            ),
        );
        installed(
            &mut registry,
            body_backed("vendor::native", crate::RuntimeBacking::HostNative),
        );
        registry.insert(body_backed(
            "embedder::text",
            text_backing("proc embedder::text {} {}"),
        ));
        installed(
            &mut registry,
            body_backed(
                "vendor::shadowed",
                text_backing("proc vendor::shadowed {} {}"),
            ),
        );
        registry.insert(body_backed(
            "vendor::shadowed",
            crate::RuntimeBacking::HostNative,
        ));

        let offered: Vec<(&str, &str)> = registry
            .reference_bodies()
            .map(|(spec, text)| (spec.name, text))
            .collect();
        assert_eq!(
            offered,
            vec![
                ("vendor::text", "proc vendor::text {} {}"),
                ("vendor::file", "proc vendor::file {} {}"),
            ]
        );
        assert_eq!(
            registry.reference_body(in_text),
            Some("proc vendor::text {} {}")
        );
        assert_eq!(
            registry.reference_body(from_file),
            Some("proc vendor::file {} {}")
        );
        // A projection for a profile carries the text with the spec.
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        assert_eq!(
            registry
                .project_for_profile(profile)
                .reference_bodies()
                .count(),
            2
        );
    }

    /// Tcl 9.0.4: inside a method the bare words resolve to the receiving
    /// object's `my` command or the `::oo::Helpers` namespace path, while every
    /// global `::keyword` spelling is absent. Generic registry lookup must not
    /// turn those contextual bare specs into nonexistent global commands.
    #[test]
    fn rooted_contextual_commands_do_not_fall_back_to_bare_specs() {
        let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
        for (head, expected) in [
            ("my", MethodDispatchKind::SelfDispatch),
            ("next", MethodDispatchKind::NextChain),
            ("nextto", MethodDispatchKind::NextChain),
            ("self", MethodDispatchKind::Introspection),
        ] {
            assert_eq!(registry.method_dispatch_keyword(head), Some(expected));
            assert_eq!(
                registry.method_dispatch_keyword(&format!("::{head}")),
                None,
                "tclsh9.0.4 has no global ::{head} command"
            );
        }
        for qualified in [
            "oo::Helpers::next",
            "::oo::Helpers::next",
            "oo::Helpers::nextto",
            "::oo::Helpers::nextto",
            "oo::Helpers::self",
            "::oo::Helpers::self",
        ] {
            assert_eq!(registry.method_dispatch_keyword(qualified), None);
        }

        assert!(registry.is_self_receiver_call("self", None));
        assert!(registry.is_self_receiver_call("self", Some("object")));
        assert!(!registry.is_self_receiver_call("::self", None));
        assert!(!registry.is_self_receiver_call("::self", Some("object")));
        for qualified in ["oo::Helpers::self", "::oo::Helpers::self"] {
            assert!(
                registry.is_self_receiver_call(qualified, None),
                "no-argument {qualified} is the real qualified helper command"
            );
            assert!(
                registry.is_self_receiver_call(qualified, Some("object")),
                "{qualified} is the real qualified helper command"
            );
        }

        for head in [
            "callback",
            "classvariable",
            "link",
            "my",
            "mymethod",
            "next",
            "nextto",
            "self",
        ] {
            let rooted = format!("::{head}");
            assert!(registry.get(head).is_some(), "bare {head}");
            assert!(registry.get(&rooted).is_none(), "rooted {head}");
            assert!(
                registry
                    .get_for_surface(&rooted, registry.own_surface_query())
                    .is_none(),
                "profiled rooted {head}"
            );
            assert!(!registry.has_command_in_this_dialect(&rooted), "{rooted}");
            assert_eq!(
                registry.known_in_any_dialect(&rooted),
                head == "next",
                "only BPF-Tcl independently declares a non-contextual global {rooted}"
            );
            assert!(!registry.declares_command_at(&rooted), "{rooted}");
            assert!(
                !registry.resolves_only_in_method_context(&rooted),
                "{rooted}"
            );
            assert!(!registry.requires_oo_method_frame(&rooted), "{rooted}");
        }
        assert!(!registry.binds_method_alias("::link"));

        for (rooted, canonical) in [
            ("::set", "set"),
            ("::oo::Helpers::self", "oo::Helpers::self"),
        ] {
            assert_eq!(registry.get(rooted).map(|spec| spec.name), Some(canonical));
            assert!(registry.has_command_in_this_dialect(rooted), "{rooted}");
            assert!(registry.known_in_any_dialect(rooted), "{rooted}");
            assert!(registry.declares_command_at(rooted), "{rooted}");
        }
    }

    /// Dynamic-role capability declarations are a closed, conservative
    /// registry contract.  Source-aware callers cannot run a resolver after
    /// expansion, so every emitted role must be declared here instead of
    /// assuming a particular command's fallback shape.
    /// Every shipped surface, not just the always-loaded Tcl, stdlib, tcllib,
    /// Itcl, and Tk catalogue.  Resolver capability metadata is equally
    /// load-bearing for optional dialect/package overlays.
    fn registry_with_every_resolver_surface() -> CommandRegistry {
        let mut registry = CommandRegistry::build_default();
        for layer in [
            SurfaceLayer::Package("bpf"),
            SurfaceLayer::Core(Family::F5Irules, ""),
            SurfaceLayer::Package("iapps"),
            SurfaceLayer::Package("tmsh"),
            SurfaceLayer::Package("expect"),
            SurfaceLayer::Package("spectcl"),
        ] {
            registry.load_surface(layer);
        }
        registry
    }

    fn assert_role_inputs_exclusive(registry: &CommandRegistry) {
        for specs in registry.by_name.values() {
            for spec in specs {
                for inputs in std::iter::once([
                    spec.arg_role_resolver.is_some(),
                    spec.arg_role_count_resolver.is_some(),
                    spec.arg_role_layout_resolver.is_some(),
                ])
                .chain(spec.subcommands.iter().map(|sub| {
                    [
                        sub.arg_role_resolver.is_some(),
                        sub.arg_role_count_resolver.is_some(),
                        sub.arg_role_layout_resolver.is_some(),
                    ]
                })) {
                    assert!(
                        inputs.into_iter().filter(|present| *present).count() <= 1,
                        "{} has conflicting role input contracts",
                        spec.name
                    );
                }
            }
        }
    }

    fn assert_count_role_capabilities(registry: &CommandRegistry) {
        for specs in registry.by_name.values() {
            for spec in specs {
                for (resolver, declared) in
                    std::iter::once((spec.arg_role_count_resolver, spec.arg_role_resolver_roles))
                        .chain(
                            spec.subcommands.iter().map(|sub| {
                                (sub.arg_role_count_resolver, sub.arg_role_resolver_roles)
                            }),
                        )
                {
                    if let Some(resolver) = resolver {
                        for count in [0, 3, 5, 256] {
                            for (_, role) in resolver(count) {
                                assert!(
                                    declared.contains(&role),
                                    "count resolver emitted {role:?} for argc {count}, outside {declared:?}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    fn assert_command_role_capabilities(registry: &CommandRegistry, name: &str, args: &[&str]) {
        let spec = registry.get(name).expect("registered command");
        let roles = if let Some(resolver) = spec.arg_role_count_resolver {
            resolver(args.len())
        } else if let Some(resolver) = spec.arg_role_layout_resolver {
            let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
            let options = crate::resolved_invocation::invocation_options(
                spec,
                None,
                None,
                crate::resolved_invocation::InvocationAvailability {
                    query: Some(profile.surface_query()),
                    package_version: None,
                },
            );
            resolver(
                crate::InvocationArguments::literals(args)
                    .with_dialect(crate::InvocationDialect::of_profile(profile)),
                options,
            )
            .expect("literal native layout")
        } else if let Some(resolver) = spec.arg_role_resolver {
            resolver(args)
        } else {
            assert!(!spec.has_dynamic_argument_roles());
            return;
        };
        for (_, role) in roles {
            assert!(
                spec.arg_role_resolver_roles.contains(&role),
                "resolver emitted {role:?} for {args:?}, outside {:?}",
                spec.arg_role_resolver_roles,
            );
        }
    }

    fn assert_subcommand_role_capabilities(
        registry: &CommandRegistry,
        command_name: &str,
        sub_name: &str,
        args: &[&str],
    ) {
        let spec = registry.get(command_name).expect("registered command");
        let sub = spec.subcommand(sub_name).expect("registered subcommand");
        let roles = if let Some(resolver) = sub.arg_role_count_resolver {
            resolver(args.len())
        } else if let Some(resolver) = sub.arg_role_layout_resolver {
            let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
            let options = crate::resolved_invocation::invocation_options(
                spec,
                Some(sub),
                None,
                crate::resolved_invocation::InvocationAvailability {
                    query: Some(profile.surface_query()),
                    package_version: None,
                },
            );
            resolver(
                crate::InvocationArguments::literals(args)
                    .with_dialect(crate::InvocationDialect::of_profile(profile)),
                options,
            )
            .expect("literal native layout")
        } else {
            sub.arg_role_resolver.expect("dynamic role resolver")(args)
        };
        for (_, role) in roles {
            assert!(sub.arg_role_resolver_roles.contains(&role));
        }
    }

    #[test]
    fn dynamic_role_capabilities_cover_every_resolver_and_representative_output() {
        let registry = registry_with_every_resolver_surface();
        let mut resolver_count = 0;
        for specs in registry.by_name.values() {
            for spec in specs {
                if spec.has_dynamic_argument_roles() {
                    resolver_count += 1;
                    assert!(
                        !spec.arg_role_resolver_roles.is_empty(),
                        "{} has a dynamic role resolver without its closed capability set",
                        spec.name
                    );
                }
                for sub in spec.subcommands {
                    if sub.has_dynamic_argument_roles() {
                        resolver_count += 1;
                        assert!(
                            !sub.arg_role_resolver_roles.is_empty(),
                            "{} {} has a dynamic role resolver without its closed capability set",
                            spec.name,
                            sub.name
                        );
                    }
                }
            }
        }
        assert!(
            resolver_count >= 52,
            "the resolver catalogue unexpectedly shrank"
        );

        assert_role_inputs_exclusive(&registry);

        assert_count_role_capabilities(&registry);

        // Representative resolver families, including the multi-role and
        // discriminator-dependent rows most likely to drift.
        assert_command_role_capabilities(&registry, "set", &["name"]);
        assert_command_role_capabilities(&registry, "set", &["name", "value"]);
        assert_command_role_capabilities(
            &registry,
            "regexp",
            &["pattern", "text", "whole", "capture"],
        );
        assert_command_role_capabilities(
            &registry,
            "regsub",
            &["pattern", "text", "replacement", "out"],
        );
        assert_command_role_capabilities(
            &registry,
            "if",
            &["expr", "then", "body", "else", "fallback"],
        );
        assert_command_role_capabilities(
            &registry,
            "try",
            &["body", "on", "0", "result", "handler"],
        );
        assert_subcommand_role_capabilities(&registry, "binary", "scan", &["bytes", "a*", "out"]);
        assert_subcommand_role_capabilities(
            &registry,
            "dict",
            "update",
            &["d", "key", "local", "body"],
        );
        assert_subcommand_role_capabilities(
            &registry,
            "namespace",
            "which",
            &["-variable", "name"],
        );
        assert_subcommand_role_capabilities(&registry, "namespace", "which", &["-command", "name"]);
        assert_subcommand_role_capabilities(
            &registry,
            "trace",
            "add",
            &["variable", "name", "write", "callback"],
        );
        // A clause grammar took the retired `if` / `try` resolvers' place; its
        // walk's flat roles stay inside the same closed capability set.
        let check_grammar = |name: &str, args: &[&str]| {
            let spec = registry.get(name).expect("command");
            let plan = spec.clause_plan(args, None).expect("a clause grammar");
            for (_, role) in plan.roles {
                assert!(
                    spec.arg_role_resolver_roles.contains(&role)
                        || spec.arg_roles.iter().any(|(_, found)| *found == role),
                    "{name}'s grammar emitted {role:?} for {args:?}, outside its declared roles"
                );
            }
        };
        check_grammar("if", &["expr", "then", "body", "else", "fallback"]);
        check_grammar("try", &["body", "on", "0", "result", "handler"]);
    }

    /// A clause grammar's flat roles are part of `may_have_arg_role`'s answer,
    /// so every role a grammar can assign is one its spec already declares —
    /// in the resolver's closed set (the set a grammar that replaced a
    /// resolver keeps) or in the static table. Otherwise an expansion-blocked
    /// call would abstain differently from a literal one.
    #[test]
    fn clause_grammar_roles_are_declared_capabilities() {
        let registry = registry_with_every_resolver_surface();
        let mut grammars = 0;
        let declared = |roles: &[ArgRole], table: &[(u8, ArgRole)], role: ArgRole| {
            roles.contains(&role) || table.iter().any(|(_, found)| *found == role)
        };
        for specs in registry.by_name.values() {
            for spec in specs {
                let owners = std::iter::once((
                    spec.name.to_owned(),
                    spec.clause_grammar,
                    spec.arg_role_resolver_roles,
                    spec.arg_roles,
                ))
                .chain(spec.subcommands.iter().map(|sub| {
                    (
                        format!("{} {}", spec.name, sub.name),
                        sub.clause_grammar,
                        sub.arg_role_resolver_roles,
                        sub.arg_roles,
                    )
                }));
                for (owner, grammar, roles, table) in owners {
                    let Some(grammar) = grammar else { continue };
                    grammars += 1;
                    for &role in ArgRole::ALL {
                        assert!(
                            !grammar.may_assign(role) || declared(roles, table, role),
                            "{owner}'s clause grammar can assign {role:?}, which it does not declare"
                        );
                    }
                }
            }
        }
        assert!(
            grammars >= 11,
            "the clause-grammar catalogue unexpectedly shrank"
        );
    }

    /// The representative rows in the sibling test are hand-picked, so a
    /// resolver nobody thought to list stays unchecked — `control::do` emitted
    /// `ArgRole::Expr` without declaring it for exactly that reason (#2068).
    /// Sweep every resolver instead: feed each one the literals its own spec
    /// knows about (option names, `arg_values` words, sibling subcommand
    /// names), at every position of every arity up to the widest form a
    /// resolver in this tree inspects, and require the closed capability set
    /// to cover the result.
    #[test]
    fn dynamic_role_capabilities_cover_every_resolver_argument_shape() {
        let registry = registry_with_every_resolver_surface();
        let mut swept = 0;
        for specs in registry.by_name.values() {
            for spec in specs {
                if let Some(resolver) = spec.arg_role_count_resolver {
                    swept += 1;
                    for count in 0..=6 {
                        assert!(
                            resolver(count)
                                .iter()
                                .all(|(_, role)| spec.arg_role_resolver_roles.contains(role))
                        );
                    }
                }
                if let Some(resolver) = spec.arg_role_layout_resolver {
                    swept += 1;
                    sweep_layout_resolver(spec, None, resolver);
                }
                if let Some(resolver) = spec.arg_role_resolver {
                    swept += 1;
                    sweep_resolver(
                        resolver,
                        spec.arg_role_resolver_roles,
                        &resolver_literals(spec, None),
                        spec.name,
                    );
                }
                for sub in spec.subcommands {
                    if let Some(resolver) = sub.arg_role_count_resolver {
                        swept += 1;
                        for count in 0..=6 {
                            assert!(
                                resolver(count)
                                    .iter()
                                    .all(|(_, role)| sub.arg_role_resolver_roles.contains(role))
                            );
                        }
                    }
                    if let Some(resolver) = sub.arg_role_layout_resolver {
                        swept += 1;
                        sweep_layout_resolver(spec, Some(sub), resolver);
                    }
                    if let Some(resolver) = sub.arg_role_resolver {
                        swept += 1;
                        sweep_resolver(
                            resolver,
                            sub.arg_role_resolver_roles,
                            &resolver_literals(spec, Some(sub)),
                            &format!("{} {}", spec.name, sub.name),
                        );
                    }
                }
            }
        }
        // The same floor the capability test asserts, so the two stay in
        // step: a resolver added without a capability set fails there, and
        // one whose emitted roles drift fails here.
        assert!(swept >= 52, "the resolver catalogue unexpectedly shrank");
    }

    fn sweep_layout_resolver(
        spec: &'static crate::CommandSpec,
        sub: Option<&'static crate::spec::SubCommand>,
        resolver: crate::spec::ArgRoleLayoutResolver,
    ) {
        let declared = sub.map_or(spec.arg_role_resolver_roles, |sub| {
            sub.arg_role_resolver_roles
        });
        let options = crate::resolved_invocation::invocation_options(
            spec,
            sub,
            None,
            crate::resolved_invocation::InvocationAvailability::default(),
        );
        let literals = resolver_literals(spec, sub);
        for len in 0..=6 {
            let mut words = vec![crate::InvocationWord::Dynamic; len];
            for position in 0..len {
                for literal in &literals {
                    words[position] = crate::InvocationWord::Literal(literal);
                    if let Some(roles) =
                        resolver(crate::InvocationArguments::structured(&words), options)
                    {
                        assert!(
                            roles.iter().all(|(_, role)| declared.contains(role)),
                            "{}: {words:?} -> {roles:?}",
                            spec.name
                        );
                    }
                }
                words[position] = crate::InvocationWord::Dynamic;
            }
        }
    }

    /// The literal words a resolver is plausibly handed: the option names and
    /// `arg_values` of the spec it belongs to, plus its sibling subcommand
    /// names, which discriminator-dependent resolvers (`trace add`,
    /// `namespace which`) branch on.  Deduplicated and bounded so the sweep
    /// stays a fast unit test.
    fn resolver_literals(
        spec: &'static crate::CommandSpec,
        sub: Option<&'static crate::spec::SubCommand>,
    ) -> Vec<&'static str> {
        let mut out: BTreeSet<&'static str> = BTreeSet::new();
        let collect = |options: &'static [crate::hover::OptionSpec],
                       arg_values: &'static [(u8, &'static [crate::hover::ArgValue])],
                       out: &mut BTreeSet<&'static str>| {
            for option in options {
                out.insert(option.name);
            }
            for (_, values) in arg_values {
                for value in *values {
                    out.insert(value.value);
                }
            }
        };
        collect(spec.options, spec.arg_values, &mut out);
        if let Some(sub) = sub {
            collect(sub.options, sub.arg_values, &mut out);
        }
        for sibling in spec.subcommands {
            out.insert(sibling.name);
        }
        // A bare word and a lone dash stand in for "some value" and "an
        // option-shaped word the table does not know".
        out.insert("x");
        out.insert("-");
        out.into_iter().take(48).collect()
    }

    /// Call `resolver` with every one-literal substitution into an
    /// all-placeholder argument vector, for each arity up to `MAX_WORDS`, and
    /// assert each emitted role is declared.
    fn sweep_resolver(
        resolver: ArgRoleResolver,
        declared: &[ArgRole],
        literals: &[&'static str],
        label: &str,
    ) {
        // The widest form any resolver in this tree inspects is `trace add
        // variable name ops callback` plus a trailing word; six covers it
        // with room to spare, and a resolver that only reads a prefix is
        // exercised by the shorter arities in the same loop.
        const MAX_WORDS: usize = 6;
        let check = |args: &[&str]| {
            for (_, role) in resolver(args) {
                assert!(
                    declared.contains(&role),
                    "`{label}`'s resolver emitted {role:?} for {args:?}, outside its declared {declared:?}"
                );
            }
        };
        for len in 0..=MAX_WORDS {
            let base = vec!["x"; len];
            check(&base);
            for position in 0..len {
                for literal in literals {
                    let mut args = base.clone();
                    args[position] = literal;
                    check(&args);
                }
            }
        }
    }

    // Cross-language RPC roles.

    /// The remote-method relation is registry data on the iRules surface, so
    /// it exists there and nowhere else. A consumer asking the registry —
    /// rather than naming `ILX::call` — is what makes the dialect gate
    /// automatic.
    #[test]
    fn remote_method_commands_are_an_irules_only_surface() {
        let plain = CommandRegistry::build_default();
        assert!(
            plain.remote_method_commands().is_empty(),
            "stock Tcl has no cross-language RPC commands"
        );
        assert!(plain.remote_method("ILX::call").is_none());

        let mut irules = CommandRegistry::build_default();
        irules.load_irules();
        assert_eq!(
            irules.remote_method_commands(),
            vec!["ILX::call", "ILX::init", "ILX::notify"]
        );
        // The explicitly-global spelling is the same command.
        assert!(irules.remote_method("::ILX::call").is_some());
        assert!(
            irules
                .remote_method("ILX::init")
                .is_some_and(|role| role.opens_handle().is_some())
        );
        assert!(
            irules
                .remote_method("ILX::notify")
                .is_some_and(|role| role.calls_method().is_some())
        );
    }

    // Ambient packages.

    /// Two packs naming the same package are two claims that the runtime
    /// provides at least that version. The strongest claim is the one that
    /// holds, so the floor is the greatest — never the last declared, which
    /// would make the answer depend on pack load order.
    #[test]
    fn the_highest_declared_ambient_version_is_the_floor() {
        let mut registry = CommandRegistry::build_default();
        assert_eq!(
            registry.ambient_package_floor("Tk"),
            None,
            "a compiled-in registry declares none"
        );

        registry.insert_ambient_package("Tk", "8.6");
        registry.insert_ambient_package("Tk", "8.5");
        registry.insert_ambient_package("Itcl", "4.0");
        assert_eq!(registry.ambient_package_floor("Tk"), Some("8.6"));
        assert_eq!(registry.ambient_package_floor("Itcl"), Some("4.0"));
        assert_eq!(registry.ambient_package_floor("Expect"), None);

        // Order-independent: declaring the higher one last changes nothing.
        let mut reversed = CommandRegistry::build_default();
        reversed.insert_ambient_package("Tk", "8.5");
        reversed.insert_ambient_package("Tk", "8.6");
        assert_eq!(reversed.ambient_package_floor("Tk"), Some("8.6"));
    }

    // The complement of `arg_indices_for_role`: what the *next* words would
    // mean, which is the question a splice-a-trailing-argument quick fix asks.

    #[test]
    fn unfilled_trailing_roles_reports_the_optional_capture_variables() {
        let reg = crate::model::ingress::static_context_for("tcl8.6").commands();
        // `catch {body}` leaves both `VarWrite` slots open.
        assert_eq!(
            reg.unfilled_trailing_roles("catch", &["{body}"]),
            vec![(1, ArgRole::VarWrite), (2, ArgRole::VarWrite)]
        );
        // One supplied leaves one.
        assert_eq!(
            reg.unfilled_trailing_roles("catch", &["{body}", "res"]),
            vec![(2, ArgRole::VarWrite)]
        );
        // Fully supplied leaves none.
        assert!(
            reg.unfilled_trailing_roles("catch", &["{body}", "res", "opts"])
                .is_empty()
        );
    }

    #[test]
    fn prospective_trailing_layout_preserves_unknown_source_values() {
        let reg = crate::model::ingress::static_context_for("tcl9.0").commands();
        let ordinary = [InvocationWord::Literal("aa"), InvocationWord::Dynamic];
        let roles =
            reg.unfilled_trailing_roles_words("regexp", InvocationArguments::structured(&ordinary));
        assert_eq!(roles.first(), Some(&(2, ArgRole::VarWrite)));
        assert!(roles.iter().all(|(_, role)| *role == ArgRole::VarWrite));
        let replacement = [
            InvocationWord::Literal("-command"),
            InvocationWord::Literal("aa"),
            InvocationWord::Dynamic,
            InvocationWord::Dynamic,
        ];
        assert_eq!(
            reg.unfilled_trailing_roles_words(
                "regsub",
                InvocationArguments::structured(&replacement),
            ),
            vec![(4, ArgRole::VarWrite)]
        );
        for words in [
            vec![InvocationWord::Dynamic, InvocationWord::Dynamic],
            vec![
                InvocationWord::Literal("-inline"),
                InvocationWord::Literal("aa"),
                InvocationWord::Dynamic,
            ],
            vec![InvocationWord::Literal("aa"), InvocationWord::Expanded],
        ] {
            assert!(
                reg.unfilled_trailing_roles_words(
                    "regexp",
                    InvocationArguments::structured(&words),
                )
                .is_empty(),
                "{words:?}"
            );
        }
    }

    #[test]
    fn unfilled_trailing_roles_stops_at_the_arity_ceiling() {
        let reg = CommandRegistry::build_default();
        // Every position past `catch`'s maximum of three is absent, however
        // many are asked for.
        let roles = reg.unfilled_trailing_roles("catch", &["{body}"]);
        assert!(roles.iter().all(|(index, _)| *index < 3), "{roles:?}");
    }

    #[test]
    fn unfilled_trailing_roles_is_empty_for_an_unknown_command() {
        let reg = CommandRegistry::build_default();
        assert!(
            reg.unfilled_trailing_roles("no_such_command", &[])
                .is_empty()
        );
    }

    #[test]
    fn unfilled_trailing_roles_stops_at_an_undeclared_position() {
        let reg = CommandRegistry::build_default();
        // `puts` declares no trailing role a caller could fill with a
        // *known* meaning, so nothing is offered rather than an unbounded
        // run of `Value` slots.
        assert!(reg.unfilled_trailing_roles("puts", &["hello"]).is_empty());
    }

    /// `SAFE_INTERP_HIDDEN` and `TRANSFERS_CONTROL` were the same bit.
    ///
    /// Both were spelled `1 << 61`, so the 65th trait silently aliased the
    /// 61st. Because `FRAME_SENSITIVE_TRAITS` unions in `TRANSFERS_CONTROL`,
    /// every safe-interp-hidden command — `file`, `source`, `encoding`,
    /// `open`, … — read as frame-sensitive and had the inline-proc code
    /// action suppressed, while `break`/`continue`/`yield`/`yieldto`/
    /// `tailcall` read as safe-interp-hidden.
    ///
    /// The two are now separate enum variants, so the aliasing is
    /// unrepresentable rather than merely fixed. This pins the *behaviour*
    /// that was wrong, which a type-level guarantee alone does not cover.
    #[test]
    fn safe_interp_hidden_is_not_control_transfer() {
        let reg = CommandRegistry::build_default();
        assert_ne!(Traits::TRANSFERS_CONTROL, Traits::SAFE_INTERP_HIDDEN);

        for name in ["break", "continue", "yield", "yieldto", "tailcall"] {
            let spec = reg.get(name).expect("control-transfer command");
            assert!(
                spec.traits.contains(Traits::TRANSFERS_CONTROL),
                "{name} must transfer control"
            );
            assert!(
                !spec.traits.contains(Traits::SAFE_INTERP_HIDDEN),
                "{name} is not hidden in a safe interpreter"
            );
            assert!(reg.is_frame_sensitive(name), "{name} is frame-sensitive");
        }

        // Safe-hidden commands carrying no *other* frame-sensitive trait.
        // These are the ones the alias was wrongly marking: each had the
        // inline-proc code action suppressed on it.
        for name in [
            "file", "encoding", "open", "socket", "exec", "cd", "pwd", "glob", "load", "unload",
        ] {
            let spec = reg.get(name).expect("safe-hidden command");
            assert!(
                spec.traits.contains(Traits::SAFE_INTERP_HIDDEN),
                "{name} is hidden in a safe interpreter"
            );
            assert!(
                !spec.traits.contains(Traits::TRANSFERS_CONTROL),
                "{name} does not transfer control"
            );
            assert!(
                !reg.is_frame_sensitive(name),
                "{name} must not be frame-sensitive — the alias suppressed the \
                 inline-proc code action on it"
            );
        }

        // The correction must not swing too far: `source` and `exit` are
        // frame-sensitive on their own merits (a barrier and a block
        // terminator respectively), and stay so.
        for name in ["source", "exit"] {
            assert!(
                reg.is_frame_sensitive(name),
                "{name} is frame-sensitive independently of the trait alias"
            );
        }
    }

    use super::*;

    #[test]
    fn get_for_dialect_picks_the_most_specific_visible_spec() {
        // §5.3/D6 — the single selection rule. Three specs of one name,
        // registered deliberately most-specific-FIRST so the old
        // last-match rule would pick the catch-all:
        //   1. a TCL86-scoped spec (tightest),
        //   2. a wider TCL86|TCL90 spec,
        //   3. a catch-all (`surface: None`).
        let mut reg = CommandRegistry::build_default();
        reg.insert(CommandSpec {
            name: "d6_probe",
            surface: Some(SpecSurface::TCL86),
            ..CommandSpec::DEFAULT
        });
        reg.insert(CommandSpec {
            name: "d6_probe",
            surface: Some(surface![SpecSurface::core_in(
                Family::Tcl,
                &[("8.6", Some("9.1"))]
            )]),
            ..CommandSpec::DEFAULT
        });
        reg.insert(CommandSpec {
            name: "d6_probe",
            surface: None,
            ..CommandSpec::DEFAULT
        });

        // Scoped beats catch-all, tighter beats wider — even though the
        // catch-all was registered last.
        let under_86 =
            reg.get_for_surface("d6_probe", Some(SurfaceQuery::core(Family::Tcl, "8.6")));
        assert_eq!(under_86.and_then(|s| s.surface), Some(SpecSurface::TCL86));
        // Under 9.0 the tightest visible spec is the two-bit one.
        let under_90 =
            reg.get_for_surface("d6_probe", Some(SurfaceQuery::core(Family::Tcl, "9.0")));
        assert_eq!(
            under_90.and_then(|s| s.surface),
            Some(surface![SpecSurface::core_in(
                Family::Tcl,
                &[("8.6", Some("9.1"))]
            )])
        );
        // Where no scoped spec is visible, the catch-all still resolves.
        let under_84 =
            reg.get_for_surface("d6_probe", Some(SurfaceQuery::core(Family::Tcl, "8.4")));
        assert!(under_84.is_some_and(|s| s.surface.is_none()));
    }

    #[test]
    fn get_for_dialect_breaks_specificity_ties_by_last_registration() {
        // Curated pack overrides re-register a name at the same scope; the
        // later registration must keep winning (the old `.rev()` guarantee,
        // preserved as the D6 tie-break).
        let mut reg = CommandRegistry::build_default();
        reg.insert(CommandSpec {
            name: "d6_tie",
            surface: Some(SpecSurface::TCL86),
            arity: Arity::exact(1), // base data
            ..CommandSpec::DEFAULT
        });
        reg.insert(CommandSpec {
            name: "d6_tie",
            surface: Some(SpecSurface::TCL86),
            arity: Arity::exact(2), // curated override
            ..CommandSpec::DEFAULT
        });
        let won = reg
            .get_for_surface("d6_tie", Some(SurfaceQuery::core(Family::Tcl, "8.6")))
            .expect("d6_tie resolves");
        assert_eq!(won.arity, Arity::exact(2), "later registration wins ties");
    }

    fn reference_best_visible(
        registry: &CommandRegistry,
        specs: &[&'static CommandSpec],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&'static CommandSpec> {
        specs
            .iter()
            .enumerate()
            .filter(|(_, spec)| registry.spec_visible(spec, dialect))
            .max_by_key(|&(index, spec)| {
                let nearness = std::cmp::Reverse(
                    spec.surface
                        .zip(dialect)
                        .and_then(|(rows, query)| surface_nearness(rows, &query))
                        .unwrap_or(usize::MAX),
                );
                let scope_tightness =
                    std::cmp::Reverse(spec.surface.map_or(u32::MAX, surface_breadth));
                (spec.surface.is_some(), nearness, scope_tightness, index)
            })
            .map(|(_, spec)| *spec)
    }

    fn synthetic_spec(
        name: &'static str,
        surface: Option<&'static [SpecSurface]>,
        traits: Traits,
    ) -> &'static CommandSpec {
        Box::leak(Box::new(CommandSpec {
            name,
            traits,
            surface,
            ..CommandSpec::DEFAULT
        }))
    }

    fn assert_best_visible_matches_reference(
        registry: &CommandRegistry,
        specs: &[&'static CommandSpec],
        dialect: Option<SurfaceQuery<'_>>,
    ) {
        let expected = reference_best_visible(registry, specs, dialect);
        let actual = registry.best_visible(specs, dialect);
        assert_eq!(
            actual.map(|spec| spec.name),
            expected.map(|spec| spec.name),
            "optimised selector changed the selected name"
        );
        assert_eq!(
            actual.map(std::ptr::from_ref),
            expected.map(std::ptr::from_ref),
            "optimised selector changed the selected spec"
        );
    }

    #[test]
    fn best_visible_matches_reference_for_surface_and_registration_cases() {
        let registry = CommandRegistry::build_default();
        let scoped = synthetic_spec(
            "d6_scoped_singleton",
            Some(SpecSurface::TCL86),
            Traits::empty(),
        );
        let catch_all = synthetic_spec("d6_catch_all_singleton", None, Traits::empty());

        // A singleton scoped row and a singleton catch-all row are both
        // visible exactly as the reference selector expects.
        assert_best_visible_matches_reference(
            &registry,
            &[scoped],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        );
        assert_best_visible_matches_reference(&registry, &[catch_all], None);

        let narrow = synthetic_spec("d6_narrow", Some(SpecSurface::TCL86), Traits::empty());
        let wide = synthetic_spec("d6_wide", Some(SpecSurface::TCL86_PLUS), Traits::empty());
        let disjoint_old =
            synthetic_spec("d6_disjoint_old", Some(SpecSurface::TCL85), Traits::empty());
        let disjoint_new =
            synthetic_spec("d6_disjoint_new", Some(SpecSurface::TCL90), Traits::empty());

        // Overlapping rows choose the narrow window, while disjoint rows
        // produce no result when neither row admits the queried release.
        assert_best_visible_matches_reference(
            &registry,
            &[wide, narrow],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        );
        assert_best_visible_matches_reference(
            &registry,
            &[disjoint_old, disjoint_new],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        );

        let tie_first = synthetic_spec("d6_tie_first", Some(SpecSurface::TCL86), Traits::empty());
        let tie_last = synthetic_spec("d6_tie_last", Some(SpecSurface::TCL86), Traits::empty());
        assert_best_visible_matches_reference(
            &registry,
            &[tie_first, tie_last],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        );
    }

    #[test]
    fn best_visible_prefers_the_nearer_core_point() {
        use tcl_dialect::model::CorePoints;

        let registry = CommandRegistry::build_default();
        let jim_rows = surface![SpecSurface::core_in(Family::Jim, &[("0.80", None)])];
        let own = synthetic_spec("near_own", Some(jim_rows), Traits::empty());
        let inherited = synthetic_spec("near_inherited", Some(SpecSurface::TCL86), Traits::empty());
        let inherited_wide = synthetic_spec(
            "near_inherited_wide",
            Some(SpecSurface::ALL_TCL),
            Traits::empty(),
        );
        let catch_all = synthetic_spec("near_catch_all", None, Traits::empty());
        let own_first = SurfaceQuery {
            realm: tcl_dialect::model::InvocationRealm::RuleLoader,
            core: CorePoints::two((Family::Jim, None), (Family::Tcl, Some("8.6"))),
            packages: &[],
        };

        // The own-family row wins over an inherited row registered after
        // it and just as narrow, and over a wider one registered before it.
        for specs in [
            [own, inherited],
            [inherited, own],
            [inherited_wide, own],
            [own, inherited_wide],
        ] {
            assert_best_visible_matches_reference(&registry, &specs, Some(own_first));
            assert_eq!(
                registry
                    .best_visible(&specs, Some(own_first))
                    .map(|spec| spec.name),
                Some("near_own")
            );
        }
        // A scoped row still beats a catch-all, and the inherited row is
        // still what a query without the own-family point selects.
        assert_best_visible_matches_reference(&registry, &[catch_all, inherited], Some(own_first));
        assert_eq!(
            registry
                .best_visible(
                    &[own, inherited],
                    Some(SurfaceQuery::core(Family::Tcl, "8.6"))
                )
                .map(|spec| spec.name),
            Some("near_inherited")
        );
    }

    #[test]
    fn best_visible_matches_reference_for_package_rows() {
        let registry = CommandRegistry::build_default();
        let package = synthetic_spec("d6_package", Some(SpecSurface::EXPECT), Traits::empty());
        let core = synthetic_spec("d6_core", Some(SpecSurface::ALL_TCL), Traits::empty());
        let packages = [PackageFloor::named("expect")];
        let with_package = SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&packages);

        // Package rows are visible through the package set and are narrower
        // than the whole core row, so they win the same reference decision
        // used by the unoptimised implementation.
        assert_best_visible_matches_reference(&registry, &[core, package], Some(with_package));
        assert_eq!(
            registry
                .best_visible(&[core, package], Some(with_package))
                .map(|spec| spec.name),
            Some("d6_package")
        );
        assert_best_visible_matches_reference(
            &registry,
            &[package],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        );
    }

    #[test]
    fn best_visible_matches_reference_for_profile_operator_visibility() {
        let operator_and_core = surface![
            SpecSurface::core(Family::Tcl),
            SpecSurface::core(Family::F5Irules),
        ];
        let ordinary = synthetic_spec(
            "d6_profile_ordinary",
            Some(operator_and_core),
            Traits::empty(),
        );
        let operator = synthetic_spec(
            "d6_profile_operator",
            Some(operator_and_core),
            Traits::OPERATOR_COMMAND,
        );
        let specs = [ordinary, operator];

        // iRules does not treat math operators as command heads, so the
        // earlier ordinary row must remain selected despite the operator row
        // being registered later. Plain Tcl permits those heads, preserving
        // the later-registration tie-break.
        let irules = crate::cache::registry_for_profile(tcl_dialect::DialectProfile::irules());
        assert_best_visible_matches_reference(irules, &specs, irules.own_surface_query());
        let plain = crate::cache::registry_for_profile(tcl_dialect::DialectProfile::plain_tcl());
        assert_best_visible_matches_reference(plain, &specs, plain.own_surface_query());
        assert_eq!(
            irules
                .best_visible(&specs, irules.own_surface_query())
                .map(|spec| spec.name),
            Some("d6_profile_ordinary")
        );
        assert_eq!(
            plain
                .best_visible(&specs, plain.own_surface_query())
                .map(|spec| spec.name),
            Some("d6_profile_operator")
        );
    }

    /// A registry whose pack declared a floor for a package its point carries
    /// asks under a query that is not `==` its profile's — and is still that
    /// profile's own point, so the profile's operator exclusion applies to it.
    #[test]
    fn a_floor_on_the_own_query_leaves_it_the_profiles_own_point() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let iapps = tcl_dialect::DialectProfile::find("f5-iapps").expect("catalogue profile");
        assert!(!iapps.operators_as_commands);
        let mut registry = CommandRegistry::build_default().project_for_profile(iapps);
        registry.insert_ambient_package("iapps", "1.0");
        let own = registry.own_surface_query();
        let query = own.expect("a profiled registry has a point");
        assert_eq!(
            query.package("iapps"),
            Some(&PackageFloor::at("iapps", "1.0"))
        );
        assert_ne!(query, iapps.surface_query());
        assert!(query.same_point(&iapps.surface_query()));

        let operator = synthetic_spec("floor_probe_operator", None, Traits::OPERATOR_COMMAND);
        let ordinary = synthetic_spec("floor_probe_ordinary", None, Traits::empty());
        assert!(!registry.spec_visible(operator, own));
        assert!(registry.spec_visible(ordinary, own));
        assert!(
            registry.spec_visible(operator, Some(SurfaceQuery::core(Family::Tcl, "8.6"))),
            "asked at another point, the exclusion is not this profile's to apply"
        );
    }

    /// Whether a name is a command in this registry's own dialect is asked at
    /// the point the registry's floors make, however late a floor arrives.
    #[test]
    fn a_command_is_known_here_only_where_the_floored_point_admits_it() {
        const FROM_8_6: &[tcl_dialect::model::SpecWindow] = &[("8.6", None)];
        let gated = synthetic_spec(
            "floor_probe_from_86",
            Some(surface![SpecSurface::package_in("Tk", FROM_8_6)]),
            Traits::empty(),
        );
        for (floor, known) in [(None, true), (Some("8.5"), false), (Some("8.6"), true)] {
            let mut authored = CommandRegistry::build_default();
            authored.insert_static(gated);
            let mut registry = authored.project_for_profile(tcl_dialect::DialectProfile::tk());
            assert!(
                registry.has_command_in_this_dialect("floor_probe_from_86"),
                "indexed, and admitted while no floor is stated"
            );
            if let Some(floor) = floor {
                registry.insert_ambient_package("Tk", floor);
            }
            assert_eq!(
                registry.has_command_in_this_dialect("floor_probe_from_86"),
                known,
                "under a floor of {floor:?}"
            );
        }
    }

    // Versioned codegen-axis stamps: selected at the point a call is resolved
    // at, and declined — never guessed — where that point does not settle them.

    const HOOK_FROM_9: &[crate::stamp_window::StampWindow<crate::hooks::CodegenHookId>] =
        &[crate::stamp_window::StampWindow {
            lifecycle: crate::lifecycle::Lifecycle::introduced_in("9.0"),
            value: crate::hooks::CodegenHookId::Lassign,
        }];
    const INLINE_BEFORE_9: &[crate::stamp_window::StampWindow<
        crate::hooks::InlineCodegenHookId,
    >] = &[crate::stamp_window::StampWindow {
        lifecycle: crate::lifecycle::Lifecycle::UNSPECIFIED.retired_from("9.0"),
        value: crate::hooks::InlineCodegenHookId::Expr,
    }];

    fn points() -> [(&'static str, Option<SurfaceQuery<'static>>); 5] {
        [
            ("8.6", Some(SurfaceQuery::core(Family::Tcl, "8.6"))),
            ("9.0", Some(SurfaceQuery::core(Family::Tcl, "9.0"))),
            ("9.1", Some(SurfaceQuery::core(Family::Tcl, "9.1"))),
            (
                "the whole ladder",
                Some(SurfaceQuery::any_release(Family::Tcl)),
            ),
            ("no point", None),
        ]
    }

    #[test]
    fn a_stamp_window_is_selected_at_the_primary_release_and_declines_where_it_is_not_settled() {
        use crate::hooks::{CodegenHookId, InlineCodegenHookId};

        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "stamp::probe",
            codegen_hook_windows: HOOK_FROM_9,
            inline_codegen_hook_windows: INLINE_BEFORE_9,
            ..CommandSpec::DEFAULT
        });
        for (point, query, codegen, inline) in [
            ("8.6", points()[0].1, None, Some(InlineCodegenHookId::Expr)),
            ("9.0", points()[1].1, Some(CodegenHookId::Lassign), None),
            ("9.1", points()[2].1, Some(CodegenHookId::Lassign), None),
            ("the whole ladder", points()[3].1, None, None),
            ("no point", points()[4].1, None, None),
        ] {
            let call = registry
                .resolve_call("stamp::probe", &["$l", "a"], query)
                .expect("the probe resolves");
            assert_eq!(call.codegen_hook, codegen, "codegen hook at {point}");
            assert_eq!(call.inline_codegen_hook, inline, "inline hook at {point}");
        }
    }

    #[test]
    fn the_plain_stamp_stands_where_no_window_covers_and_a_window_wins_where_one_does() {
        use crate::hooks::CodegenHookId;

        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "stamp::beside",
            codegen_hook: Some(CodegenHookId::Llength),
            codegen_hook_windows: HOOK_FROM_9,
            ..CommandSpec::DEFAULT
        });
        for (point, query, codegen) in [
            ("8.6", points()[0].1, Some(CodegenHookId::Llength)),
            ("9.0", points()[1].1, Some(CodegenHookId::Lassign)),
            ("the whole ladder", points()[3].1, None),
        ] {
            let call = registry
                .resolve_call("stamp::beside", &["$l"], query)
                .expect("the probe resolves");
            assert_eq!(call.codegen_hook, codegen, "{point}");
        }
    }

    /// A subcommand whose windows state nothing at the release is the command's
    /// own call; one whose windows the point does not settle is dispatched
    /// plain, and the command's hook is not the answer in its place.
    #[test]
    fn a_subcommand_inherits_where_its_windows_are_silent_and_does_not_where_they_decline() {
        use crate::hooks::CodegenHookId;

        let sub = SubCommand {
            name: "get",
            codegen_hook_windows: HOOK_FROM_9,
            ..SubCommand::DEFAULT
        };
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "stamp::ensemble",
            codegen_hook: Some(CodegenHookId::Dict),
            subcommands: Box::leak(Box::new([sub])),
            ..CommandSpec::DEFAULT
        });
        for (point, query, codegen) in [
            ("8.6", points()[0].1, Some(CodegenHookId::Dict)),
            ("9.0", points()[1].1, Some(CodegenHookId::Lassign)),
            ("the whole ladder", points()[3].1, None),
            ("no point", points()[4].1, None),
        ] {
            let call = registry
                .resolve_call("stamp::ensemble", &["get", "$d", "k"], query)
                .expect("the ensemble resolves");
            assert_eq!(call.codegen_hook, codegen, "{point}");
            // The command's own call is not the subcommand's to decline.
            let own = registry
                .resolve_call("stamp::ensemble", &["$d"], query)
                .expect("the command resolves");
            assert_eq!(own.codegen_hook, Some(CodegenHookId::Dict), "{point}");
        }
    }

    #[test]
    fn a_semantic_operation_window_decides_the_operation_a_resolved_invocation_has() {
        use crate::intrinsic::IntrinsicId;
        use crate::semantic_operation::SemanticOperationId;
        use crate::stamp_window::StampWindow;

        const LENGTH_FROM_9: &[StampWindow<SemanticOperationId>] = &[StampWindow {
            lifecycle: crate::lifecycle::Lifecycle::introduced_in("9.0"),
            value: SemanticOperationId::Intrinsic(IntrinsicId::StringLength),
        }];
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "stamp::length",
            semantic_operation_windows: LENGTH_FROM_9,
            ..CommandSpec::DEFAULT
        });
        for (point, query, operation) in [
            ("8.6", points()[0].1, SemanticOperationId::Invoke),
            (
                "9.0",
                points()[1].1,
                SemanticOperationId::Intrinsic(IntrinsicId::StringLength),
            ),
            (
                "the whole ladder",
                points()[3].1,
                SemanticOperationId::Invoke,
            ),
            ("no point", points()[4].1, SemanticOperationId::Invoke),
        ] {
            let invocation = registry
                .resolve_invocation("stamp::length", &["abc"], query)
                .expect("the probe resolves");
            assert_eq!(invocation.semantics.operation, operation, "{point}");
        }
    }

    /// A subcommand whose operation windows state nothing at the release is the
    /// command's own operation; one whose windows the point does not settle is
    /// a plain invoke, and the command's operation is not the answer instead.
    #[test]
    fn a_subcommand_operation_inherits_where_silent_and_is_plain_where_it_declines() {
        use crate::intrinsic::IntrinsicId;
        use crate::semantic_operation::SemanticOperationId;
        use crate::stamp_window::StampWindow;

        const LENGTH_FROM_9: &[StampWindow<SemanticOperationId>] = &[StampWindow {
            lifecycle: crate::lifecycle::Lifecycle::introduced_in("9.0"),
            value: SemanticOperationId::Intrinsic(IntrinsicId::StringLength),
        }];
        let concat = SemanticOperationId::Intrinsic(IntrinsicId::Concat);
        let size = SubCommand {
            name: "size",
            semantic_operation_windows: LENGTH_FROM_9,
            ..SubCommand::DEFAULT
        };
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "stamp::inherits",
            semantic_operation: Some(concat),
            subcommands: Box::leak(Box::new([size])),
            ..CommandSpec::DEFAULT
        });
        for (point, query, operation) in [
            ("8.6", points()[0].1, concat),
            (
                "9.0",
                points()[1].1,
                SemanticOperationId::Intrinsic(IntrinsicId::StringLength),
            ),
            (
                "the whole ladder",
                points()[3].1,
                SemanticOperationId::Invoke,
            ),
            ("no point", points()[4].1, SemanticOperationId::Invoke),
        ] {
            let invocation = registry
                .resolve_invocation("stamp::inherits", &["size", "abc"], query)
                .expect("the ensemble resolves");
            assert_eq!(invocation.semantics.operation, operation, "{point}");
        }
    }

    /// The operation a codegen or inline hook names is read through the same
    /// selection: a window gives it where it covers the release, and a level
    /// that declines gives plain dispatch, not its command's operation.
    #[test]
    fn the_operation_a_windowed_hook_names_is_selected_and_declined_like_the_hook() {
        use crate::hooks::{CodegenHookId, InlineCodegenHookId};
        use crate::intrinsic::IntrinsicId;
        use crate::lifecycle::Lifecycle;
        use crate::semantic_operation::SemanticOperationId;
        use crate::stamp_window::StampWindow;

        const LLENGTH_FROM_9: &[StampWindow<CodegenHookId>] = &[StampWindow {
            lifecycle: Lifecycle::introduced_in("9.0"),
            value: CodegenHookId::Llength,
        }];
        const LINDEX_FROM_9: &[StampWindow<InlineCodegenHookId>] = &[StampWindow {
            lifecycle: Lifecycle::introduced_in("9.0"),
            value: InlineCodegenHookId::Lindex,
        }];
        let concat = SemanticOperationId::Intrinsic(IntrinsicId::Concat);
        let by_codegen = SubCommand {
            name: "count",
            codegen_hook_windows: LLENGTH_FROM_9,
            ..SubCommand::DEFAULT
        };
        let by_inline = SubCommand {
            name: "at",
            inline_codegen_hook_windows: LINDEX_FROM_9,
            ..SubCommand::DEFAULT
        };
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "stamp::hooked",
            semantic_operation: Some(concat),
            subcommands: Box::leak(Box::new([by_codegen, by_inline])),
            ..CommandSpec::DEFAULT
        });
        let operation = |sub: &str, query| {
            registry
                .resolve_invocation("stamp::hooked", &[sub, "abc"], query)
                .expect("the ensemble resolves")
                .semantics
                .operation
        };
        for (sub, at_9) in [
            (
                "count",
                SemanticOperationId::Intrinsic(IntrinsicId::ListLength),
            ),
            ("at", SemanticOperationId::Intrinsic(IntrinsicId::ListIndex)),
        ] {
            assert_eq!(
                operation(sub, points()[0].1),
                concat,
                "{sub} at 8.6 inherits"
            );
            assert_eq!(operation(sub, points()[1].1), at_9, "{sub} at 9.0");
            for (point, query) in [
                ("the whole ladder", points()[3].1),
                ("no point", points()[4].1),
            ] {
                assert_eq!(
                    operation(sub, query),
                    SemanticOperationId::Invoke,
                    "{sub} at {point} declines"
                );
            }
        }
    }

    #[test]
    fn a_native_lowering_window_is_read_at_the_point_and_declines_across_its_edge() {
        use crate::completion::CompletionCode;
        use crate::lifecycle::Lifecycle;
        use crate::native_lowering::NativeLowering;
        use crate::stamp_window::StampWindow;

        const BREAK_FROM_9: &[StampWindow<NativeLowering>] = &[StampWindow {
            lifecycle: Lifecycle::introduced_in("9.0"),
            value: NativeLowering::Completion(CompletionCode::Break),
        }];
        let spec = CommandSpec {
            name: "stamp::native",
            native_lowering_windows: BREAK_FROM_9,
            ..CommandSpec::DEFAULT
        };
        let shapes = points().map(|(_, query)| spec.native_lowering_at(query.as_ref()));
        assert_eq!(
            shapes,
            [
                NativeLowering::Generic,
                NativeLowering::Completion(CompletionCode::Break),
                NativeLowering::Completion(CompletionCode::Break),
                NativeLowering::Generic,
                NativeLowering::Generic,
            ]
        );
        assert_eq!(
            spec.native_lowering(),
            NativeLowering::Generic,
            "the plain accessor reads the plain field alone"
        );
    }

    /// Every operation a windowed stamp could give a command counts when the
    /// question is surface-blind: which names could be this operation.
    #[test]
    fn a_windowed_operation_is_counted_by_the_questions_that_ask_no_release() {
        use crate::intrinsic::IntrinsicId;
        use crate::semantic_operation::SemanticOperationId;
        use crate::stamp_window::StampWindow;

        const LENGTH_FROM_9: &[StampWindow<SemanticOperationId>] = &[StampWindow {
            lifecycle: crate::lifecycle::Lifecycle::introduced_in("9.0"),
            value: SemanticOperationId::Intrinsic(IntrinsicId::StringLength),
        }];
        let spec = CommandSpec {
            name: "stamp::length",
            semantic_operation_windows: LENGTH_FROM_9,
            ..CommandSpec::DEFAULT
        };
        assert!(spec.intrinsic_ids().contains(&IntrinsicId::StringLength));
        let mut registry = CommandRegistry::build_default();
        registry.insert(spec);
        assert!(
            registry
                .command_names_for_semantic_operation(SemanticOperationId::Intrinsic(
                    IntrinsicId::StringLength
                ))
                .any(|name| name == "stamp::length")
        );
        assert!(
            !CommandSpec {
                name: "stamp::plain",
                ..CommandSpec::DEFAULT
            }
            .intrinsic_ids()
            .contains(&IntrinsicId::StringLength)
        );
    }

    #[test]
    fn profile_projection_keeps_surface_less_authored_override_of_core_name() {
        let mut registry = CommandRegistry::build_default();
        let mut override_spec = registry.get("set").expect("core set spec").clone();
        override_spec.surface = None;
        override_spec.arity = Arity::exact(17);
        registry.insert(override_spec);

        let profile = crate::model::ingress::resolve_environment("tcl8.4").analyser_profile();
        let projected = registry.project_for_profile(profile);
        assert_eq!(
            projected.get("set").map(|spec| spec.arity),
            Some(Arity::exact(17)),
            "the authored catch-all is the effective binding, not Tcl's scoped row"
        );
        assert!(
            projected.get("throw").is_none(),
            "preserving the overlay must not leak Tcl 8.6+ shipped commands into Tcl 8.4"
        );
    }

    #[test]
    fn instance_method_walks_superclasses_breadth_first() {
        use crate::spec::{ObjectClassSpec, SubCommand};

        // Diamond hierarchy: Diamond → {Left, Right} → Base. Both Left and
        // Right (and Base) define `m`; a breadth-first walk in declaration
        // order must find Left's `m` first, whereas a `Vec::pop` LIFO walk
        // would reverse the siblings and return Right's.
        static M_LEFT: [SubCommand; 1] = [SubCommand {
            name: "m",
            detail: "left",
            ..SubCommand::DEFAULT
        }];
        static M_RIGHT: [SubCommand; 1] = [SubCommand {
            name: "m",
            detail: "right",
            ..SubCommand::DEFAULT
        }];
        static M_BASE: [SubCommand; 1] = [SubCommand {
            name: "m",
            detail: "base",
            ..SubCommand::DEFAULT
        }];
        static DIAMOND: ObjectClassSpec = ObjectClassSpec {
            class_name: "Diamond",
            instance_methods: &[],
            superclasses: &["Left", "Right"],
            allow_unknown_methods: false,
            method_prefix_matching: PrefixMatching::Strict,
        };
        static LEFT: ObjectClassSpec = ObjectClassSpec {
            class_name: "Left",
            instance_methods: &M_LEFT,
            superclasses: &["Base"],
            allow_unknown_methods: false,
            method_prefix_matching: PrefixMatching::Strict,
        };
        static RIGHT: ObjectClassSpec = ObjectClassSpec {
            class_name: "Right",
            instance_methods: &M_RIGHT,
            superclasses: &["Base"],
            allow_unknown_methods: false,
            method_prefix_matching: PrefixMatching::Strict,
        };
        static BASE: ObjectClassSpec = ObjectClassSpec {
            class_name: "Base",
            instance_methods: &M_BASE,
            superclasses: &[],
            allow_unknown_methods: false,
            method_prefix_matching: PrefixMatching::Strict,
        };

        let mut reg = CommandRegistry::build_default();
        for oc in [&DIAMOND, &LEFT, &RIGHT, &BASE] {
            reg.insert(CommandSpec {
                name: oc.class_name,
                object_class: Some(oc),
                ..CommandSpec::DEFAULT
            });
        }

        let resolved = reg
            .instance_method("Diamond", "m")
            .expect("method resolves");
        assert_eq!(
            resolved.detail, "left",
            "breadth-first, declaration-ordered walk visits Left before Right"
        );
    }

    #[test]
    fn instance_method_at_honours_explicit_package_and_dialect_gates() {
        use crate::spec::ObjectClassSpec;

        static METHODS: [SubCommand; 1] = [SubCommand {
            name: "later",
            surface: Some(SpecSurface::TCL91),
            lifecycle: Lifecycle::introduced_in("9.1"),
            ..SubCommand::DEFAULT
        }];
        static CLASS: ObjectClassSpec = ObjectClassSpec {
            class_name: "VersionedClass",
            instance_methods: &METHODS,
            superclasses: &[],
            allow_unknown_methods: false,
            method_prefix_matching: PrefixMatching::Strict,
        };

        let mut reg = CommandRegistry::build_default();
        reg.insert(CommandSpec {
            name: CLASS.class_name,
            required_package: Some("Demo"),
            object_class: Some(&CLASS),
            ..CommandSpec::DEFAULT
        });

        assert!(
            reg.instance_method_at(
                "VersionedClass",
                "later",
                Some("9.0"),
                Some(SurfaceQuery::core(Family::Tcl, "9.1"))
            )
            .is_none(),
            "the explicit owning-package floor rejects a future method"
        );
        assert!(
            reg.instance_method_at(
                "VersionedClass",
                "later",
                Some("9.1"),
                Some(SurfaceQuery::core(Family::Tcl, "9.0"))
            )
            .is_none(),
            "the method's own dialect gate remains independent of lifecycle"
        );
        assert!(
            reg.instance_method_at(
                "VersionedClass",
                "later",
                Some("9.1"),
                Some(SurfaceQuery::core(Family::Tcl, "9.1"))
            )
            .is_some(),
            "both registry-declared availability axes admit the method"
        );
    }

    #[test]
    fn build_default_has_commands() {
        let reg = CommandRegistry::build_default();
        assert!(!reg.is_empty());
        assert!(reg.get("for").is_some());
        assert!(reg.get("set").is_some());
        assert!(reg.get("nonexistent_command").is_none());
    }

    #[test]
    fn leading_zero_is_octal_tracks_tcl_version() {
        // Plain default registry (no Tcl release row) defaults to octal.
        assert!(CommandRegistry::build_default().leading_zero_is_octal());
        // tcl9.0 (TIP 114) reads leading zeros as decimal; everything else
        // (8.4/8.5/8.6 and the 8.x-derived F5 dialects) stays octal.
        let octal_cases = [
            SurfaceLayer::Core(Family::Tcl, "8.4"),
            SurfaceLayer::Core(Family::Tcl, "8.5"),
            SurfaceLayer::Core(Family::Tcl, "8.6"),
            SurfaceLayer::Core(Family::F5Irules, ""),
            SurfaceLayer::Package("iapps"),
        ];
        for layer in octal_cases {
            let mut reg = CommandRegistry::build_default();
            reg.load_surface(layer);
            assert!(reg.leading_zero_is_octal(), "{layer:?} should be octal");
        }
        let mut reg90 = CommandRegistry::build_default();
        reg90.load_surface(SurfaceLayer::Core(Family::Tcl, "9.0"));
        assert!(!reg90.leading_zero_is_octal(), "tcl9.0 should be decimal");
        // tcl9.1 keeps the TIP 114 decimal rule; a tcl9.1-only
        // registry (loads TCL91, not TCL90) must still read leading zeros as
        // decimal.
        let mut reg91 = CommandRegistry::build_default();
        reg91.load_surface(SurfaceLayer::Core(Family::Tcl, "9.1"));
        assert!(!reg91.leading_zero_is_octal(), "tcl9.1 should be decimal");
    }

    #[test]
    fn irules_command_legality_matrix() {
        use crate::events::EventRegistry;
        use crate::profiles::ProfileRegistry;
        let mut reg = CommandRegistry::build_default();
        reg.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
        let events = EventRegistry::build();
        let profiles = ProfileRegistry::build();
        // HTTP::respond is satisfied in HTTP_REQUEST (HTTP profile implied) but
        // not in the L4 CLIENT_ACCEPTED event.
        assert!(reg.is_irules_command_legal_in_event(
            "HTTP::respond",
            "HTTP_REQUEST",
            &events,
            &profiles
        ));
        assert!(!reg.is_irules_command_legal_in_event(
            "HTTP::respond",
            "CLIENT_ACCEPTED",
            &events,
            &profiles
        ));
        // An unknown event is illegal for every command.
        assert!(!reg.is_irules_command_legal_in_event(
            "HTTP::respond",
            "NOT_AN_EVENT",
            &events,
            &profiles
        ));
        // HA::status is explicitly excluded from RULE_INIT.
        assert!(!reg.is_irules_command_legal_in_event(
            "HA::status",
            "RULE_INIT",
            &events,
            &profiles
        ));

        // The inverse list is sorted and reflects the same matrix.
        let evs = reg.irules_events_for_command("HTTP::respond", &events, &profiles);
        assert!(evs.contains(&"HTTP_REQUEST"));
        assert!(!evs.contains(&"CLIENT_ACCEPTED"));
        assert!(
            evs.windows(2).all(|w| w[0] <= w[1]),
            "events must be sorted"
        );
    }

    #[test]
    fn get_resolves_global_qualifier_to_builtin() {
        let reg = CommandRegistry::build_default();
        assert!(reg.get("::foreach").is_some());
        assert_eq!(
            reg.get("::for").map(|s| s.name),
            reg.get("for").map(|s| s.name)
        );
        assert!(reg.get("::nonexistent_command").is_none());
    }

    #[test]
    fn switch_names_is_dialect_filtered() {
        let reg = CommandRegistry::build_default();
        let regsub = reg.get("regsub").expect("regsub spec");
        // `-command` is Tcl 9.0+ (TIP 463); the always-available
        // switches appear in every dialect.
        let in_86 = regsub.switch_names(Some(SurfaceQuery::core(Family::Tcl, "8.6")));
        assert!(in_86.contains(&"-all"), "{in_86:?}");
        assert!(in_86.contains(&"-nocase"), "{in_86:?}");
        assert!(
            !in_86.contains(&"-command"),
            "9.0-only -command leaked into 8.6: {in_86:?}",
        );
        let in_90 = regsub.switch_names(Some(SurfaceQuery::core(Family::Tcl, "9.0")));
        assert!(
            in_90.contains(&"-command"),
            "-command missing under 9.0: {in_90:?}",
        );
        // No filter → every declared option, no duplicates.
        let all = regsub.switch_names(None);
        assert!(all.contains(&"-command"));
        let mut dedup = all.clone();
        dedup.sort_unstable();
        dedup.dedup();
        assert_eq!(dedup.len(), all.len(), "switch_names returned duplicates");
    }

    #[test]
    fn tcl9_commands_from_pr_433_are_registered() {
        let reg = CommandRegistry::build_default();
        for name in [
            "foreachLine",
            "readFile",
            "writeFile",
            "lpop",
            "const",
            "tcl::idna",
            "::tcl::idna",
            "tcl::process",
            "::tcl::process",
        ] {
            assert!(
                reg.get(name).is_some(),
                "{name} not registered after SYNC-MAY19-tcl9-commands",
            );
        }
    }

    #[test]
    fn coroinject_coroprobe_registered() {
        // Verify these two commands remain registered and visible to
        // the LSP.
        let reg = CommandRegistry::build_default();
        assert!(reg.get("coroinject").is_some());
        assert!(reg.get("coroprobe").is_some());
    }

    #[test]
    fn tcl9_commands_gated_to_tcl90() {
        let reg = CommandRegistry::build_default();
        // `const` (Tcl 9.0, TIP 677) joins the list, gated `TCL90_PLUS`: it
        // does not exist in iRules' embedded Tcl 8.4.6, so it is (correctly)
        // neither pre-9.0 nor iRules-visible.
        for name in ["foreachLine", "readFile", "writeFile", "lpop", "const"] {
            let spec = reg.get(name).expect("registered");
            // A 9.0 addition is available in 9.0 *and* 9.1 (a `.1` release is
            // additive — verified against C Tcl 9.1b0 doc/*.n), so it is gated
            // `TCL90_PLUS`, not `TCL90`-only.
            assert_eq!(
                spec.surface,
                Some(SpecSurface::TCL90_PLUS),
                "{name} should be Tcl 9.0+",
            );
            assert!(spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "9.0"))));
            assert!(spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "9.1"))));
            assert!(!spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "8.6"))));
            assert!(!spec.supports_dialect(Some(SurfaceQuery::any_release(Family::F5Irules))));
        }
    }

    #[test]
    fn every_command_has_hover_with_manpage_source() {
        // Every registered command must carry a hover snippet with a non-empty
        // summary and a manpage/source attribution. A short allowlist covers
        // internal pseudo-commands and dialect placeholders that have no user
        // documentation.
        // The four regex-quote spellings are internal idiom-recognition
        // entries for the taint analyser (T103), not real Tcl commands --
        // no manpage exists to cite (re_quote.html/.htm 404s on every
        // tcl-lang.org tree for 8.4-9.1 alike; see re_quote.rs's own doc
        // comment for the full explanation).
        const HOVERLESS_OK: &[&str] = &[
            "disabled_in_irules",
            "re_quote",
            "regex_quote",
            "regex::quote",
            "regexp::quote",
        ];
        let reg = CommandRegistry::build_default();
        let mut missing_hover = Vec::new();
        let mut missing_source = Vec::new();
        for name in reg.command_names() {
            if HOVERLESS_OK.contains(&name) {
                continue;
            }
            let spec = reg.get(name).expect("registered");
            match &spec.hover {
                None => missing_hover.push(name.to_string()),
                Some(h) => {
                    if h.summary.trim().is_empty() || h.source.trim().is_empty() {
                        missing_source.push(name.to_string());
                    }
                }
            }
        }
        missing_hover.sort();
        missing_source.sort();
        assert!(
            missing_hover.is_empty(),
            "commands without a hover snippet: {missing_hover:?}",
        );
        assert!(
            missing_source.is_empty(),
            "commands with an empty hover summary or manpage source: {missing_source:?}",
        );
    }

    #[test]
    fn timerate_registered_with_body_and_int_hint() {
        // `timerate` measures the rate of execution of a script.
        use crate::arg_role::ArgRole;
        use crate::side_effects::SideEffectTarget;
        use crate::types::TclType;
        let reg = CommandRegistry::build_default();
        let spec = reg.get("timerate").expect("timerate registered");
        assert_eq!(spec.name, "timerate");
        // BODY role on arg 0, INT type hint (shimmers) on arg 1.
        assert_eq!(spec.arg_role_at(0), Some(ArgRole::Body));
        assert_eq!(
            spec.arg_types
                .iter()
                .find(|(i, _)| *i == 1)
                .map(|(_, h)| h.expected),
            Some(Some(TclType::Int)),
        );
        // Unbounded arity: at least the command word, no upper bound.
        assert!(!spec.arity.accepts(0));
        assert!(spec.arity.accepts(1));
        assert!(spec.arity.accepts(6));
        assert_eq!(spec.return_type, Some(TclType::String));
        // The body runs arbitrary code → an UNKNOWN-target read+write effect.
        assert!(
            spec.side_effects
                .iter()
                .any(|e| e.target == SideEffectTarget::Unknown && e.reads && e.writes),
            "timerate should declare an UNKNOWN read+write side effect",
        );
    }

    #[test]
    fn lookup_for_command() {
        let reg = CommandRegistry::build_default();
        let spec = reg.get("for").unwrap();
        assert_eq!(spec.name, "for");
        assert!(spec.traits.contains(Traits::CONTROL_FLOW));
        assert!(spec.traits.contains(Traits::HAS_LOOP_BODY));
        assert_eq!(spec.arity, crate::arity::Arity::exact(4));
    }

    #[test]
    fn arg_roles_for_static_command() {
        let reg = CommandRegistry::build_default();
        let bodies =
            reg.arg_indices_for_role("for", &["init", "cond", "next", "body"], ArgRole::Body);
        assert!(bodies.contains(&0)); // init
        assert!(bodies.contains(&2)); // next
        assert!(bodies.contains(&3)); // body
        assert!(!bodies.contains(&1)); // cond is Expr, not Body
    }

    #[test]
    fn arg_roles_for_expr() {
        let reg = CommandRegistry::build_default();
        let exprs =
            reg.arg_indices_for_role("for", &["init", "cond", "next", "body"], ArgRole::Expr);
        assert_eq!(exprs, vec![1]); // only the condition
    }

    #[test]
    fn uplevel_body_arg_role_skips_optional_level() {
        // `uplevel ?level? {body}` — the body word's index depends
        // on whether a leading `level` word is present. The registry resolver
        // is the single source of truth every body consumer (semantic tokens,
        // green-tree descent, SSA) queries.
        let reg = CommandRegistry::build_default()
            .project_for_profile(DialectProfile::find("tcl8.6").unwrap());
        // Literal relative level → body at 1.
        assert_eq!(
            reg.arg_indices_for_role("uplevel", &["1", "{set x 1}"], ArgRole::Body),
            vec![1]
        );
        // Absolute `#0` level → body at 1.
        assert_eq!(
            reg.arg_indices_for_role("uplevel", &["#0", "{set x 1}"], ArgRole::Body),
            vec![1]
        );
        // No level → body at 0.
        assert_eq!(
            reg.arg_indices_for_role("uplevel", &["{set x 1}"], ArgRole::Body),
            vec![0]
        );
        // The literal bytes `$lvl` are a script word, not a supplied level.
        assert_eq!(
            reg.arg_indices_for_role("uplevel", &["$lvl", "{set x 1}"], ArgRole::Body),
            vec![0]
        );
        // A lone dynamic word is the body itself (implicit level 1) → body at 0.
        assert_eq!(
            reg.arg_indices_for_role("uplevel", &["$body"], ArgRole::Body),
            vec![0]
        );
        // Bodyless `uplevel 1` (a wrong-#args error) exposes no body word — the
        // literal level must not be mis-tagged as a script.
        assert!(
            reg.arg_indices_for_role("uplevel", &["1"], ArgRole::Body)
                .is_empty()
        );
    }

    #[test]
    fn if_marks_structural_keywords() {
        let reg = CommandRegistry::build_default();
        let args = [
            "1", "then", "{a}", "elseif", "2", "then", "{b}", "else", "{c}",
        ];
        let kw = reg.arg_indices_for_role("if", &args, ArgRole::Keyword);
        // then@1, elseif@3, then@5, else@7
        assert_eq!(kw, vec![1, 3, 5, 7], "{kw:?}");
        // The bodies and exprs still resolve too.
        let bodies = reg.arg_indices_for_role("if", &args, ArgRole::Body);
        assert!(bodies.contains(&2) && bodies.contains(&6) && bodies.contains(&8));
    }

    #[test]
    fn try_marks_structural_keywords() {
        let reg = CommandRegistry::build_default();
        let args = ["{body}", "on", "error", "{e}", "{h}", "finally", "{f}"];
        let kw = reg.arg_indices_for_role("try", &args, ArgRole::Keyword);
        // on@1, finally@5
        assert_eq!(kw, vec![1, 5], "{kw:?}");
    }

    /// The format families answer from registry data alone:
    /// the *position* from the `FormatString` / `ScanFormat` roles, the
    /// *family* from `format_string_type`.
    #[test]
    fn format_string_args_cover_every_family() {
        use crate::patterns::FormatType;
        let reg = crate::model::ingress::static_context_for("tcl9.1").commands();
        let found = |name: &str, args: &[&str]| -> Vec<(usize, FormatType, bool)> {
            reg.format_string_args(name, args)
                .into_iter()
                .map(|f| (f.index, f.kind, f.scan))
                .collect()
        };
        // TP — a fixed argument index.
        assert_eq!(
            found("format", &["%d", "7"]),
            vec![(0, FormatType::Sprintf, false)]
        );
        // TP — a resolver-computed index, scan direction.
        assert_eq!(
            found("scan", &["$s", "%d", "v"]),
            vec![(1, FormatType::Sprintf, true)]
        );
        // TP — subcommand-relative indices.
        assert_eq!(
            found("binary", &["format", "c3", "$l"]),
            vec![(1, FormatType::Binary, false)]
        );
        assert_eq!(
            found("binary", &["scan", "$v", "c3", "out"]),
            vec![(2, FormatType::Binary, true)]
        );
        // TP — an *option value* position, both directions.
        assert_eq!(
            found("clock", &["format", "$t", "-format", "%Y"]),
            vec![(3, FormatType::Clock, false)]
        );
        assert_eq!(
            found("clock", &["scan", "$s", "-format", "%Y"]),
            vec![(3, FormatType::Clock, true)]
        );
        // TP — `regsub`'s replacement template, shifted past its switches.
        assert_eq!(
            found("regsub", &["-all", "--", "e", "$s", "X", "out"]),
            vec![(4, FormatType::Regsub, false)]
        );
        // FN guard — the explicitly global spellings resolve identically.
        assert_eq!(
            found("::format", &["%d", "7"]),
            found("format", &["%d", "7"])
        );
        assert_eq!(
            found("::clock", &["format", "$t", "-format", "%Y"]),
            found("clock", &["format", "$t", "-format", "%Y"])
        );
        // TN — a subcommand with no format string, and an unknown command.
        assert!(found("binary", &["encode", "hex", "$d"]).is_empty());
        assert!(found("clock", &["seconds"]).is_empty());
        assert!(found("no::such::command", &["%d"]).is_empty());
        assert!(found("puts", &["%d"]).is_empty());
        // TN — `regsub -command` makes that position a callback, not a
        // template, so it declares no replacement word.
        assert!(
            !found("regsub", &["-command", "--", "e", "$s", "cb"])
                .iter()
                .any(|(i, _, _)| *i == 3)
        );
    }

    #[test]
    fn source_aware_format_layouts_abstain_only_while_option_selection_is_dynamic() {
        use crate::invocation_words::{InvocationArguments, InvocationWord};
        use crate::patterns::FormatType;

        let dynamic_prefix = [
            InvocationWord::Dynamic,
            InvocationWord::Literal("{a}"),
            InvocationWord::Dynamic,
            InvocationWord::Literal(r"{\1}"),
        ];
        let fixed_start_value = [
            InvocationWord::Literal("-start"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("{a}"),
            InvocationWord::Dynamic,
            InvocationWord::Literal(r"{\1}"),
        ];
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let registry = crate::model::ingress::static_context_for(dialect).commands();
            assert!(
                registry
                    .format_string_args_words(
                        "regsub",
                        InvocationArguments::structured(&dynamic_prefix),
                    )
                    .is_empty(),
                "{dialect}: $mode could shift regsub's replacement position"
            );
            assert_eq!(
                registry.format_string_args_words(
                    "regsub",
                    InvocationArguments::structured(&fixed_start_value),
                ),
                vec![FormatStringArg {
                    index: 4,
                    kind: FormatType::Regsub,
                    scan: false,
                }],
                "{dialect}: a known -start still consumes exactly its dynamic value"
            );
        }

        let invalid_command_abbrev = [
            InvocationWord::Literal("-c"),
            InvocationWord::Literal("{a}"),
            InvocationWord::Dynamic,
            InvocationWord::Literal(r"{\1}"),
        ];
        let exact_command = [
            InvocationWord::Literal("-command"),
            InvocationWord::Literal("{a}"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("callback"),
        ];
        let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
        assert!(
            registry
                .format_string_args_words(
                    "regsub",
                    InvocationArguments::structured(&invalid_command_abbrev),
                )
                .is_empty(),
            "-c is not a Tcl regsub abbreviation and cannot claim a replacement template"
        );
        assert!(
            registry
                .format_string_args_words("regsub", InvocationArguments::structured(&exact_command))
                .is_empty(),
            "only exact Tcl 9 -command selects a callback rather than a replacement template"
        );
    }

    #[test]
    fn clock_format_keeps_fixed_payload_and_option_selection_separate() {
        use crate::invocation_words::{InvocationArguments, InvocationWord};
        use crate::patterns::FormatType;
        let clock = [
            InvocationWord::Literal("format"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("-format"),
            InvocationWord::Literal("%Y"),
        ];
        assert_eq!(
            crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .format_string_args_words("clock", InvocationArguments::structured(&clock),),
            vec![FormatStringArg {
                index: 3,
                kind: FormatType::Clock,
                scan: false,
            }],
            "a fixed positional time value cannot hide an after-positional option value"
        );
        let unknown_clock_option = [
            InvocationWord::Literal("format"),
            InvocationWord::Dynamic,
            InvocationWord::Dynamic,
            InvocationWord::Literal("%Y"),
        ];
        assert!(
            crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .format_string_args_words(
                    "clock",
                    InvocationArguments::structured(&unknown_clock_option),
                )
                .is_empty(),
            "an unknown option word does not establish the format operand"
        );
    }

    #[test]
    fn source_aware_pattern_layouts_abstain_for_dynamic_leading_options() {
        use crate::invocation_words::{InvocationArguments, InvocationWord};

        let dynamic_prefix = [
            InvocationWord::Dynamic,
            InvocationWord::Literal("{a+}"),
            InvocationWord::Dynamic,
        ];
        for name in ["regexp", "regsub"] {
            assert!(
                crate::model::ingress::static_context_for("tcl9.0")
                    .commands()
                    .pattern_args_words(name, InvocationArguments::structured(&dynamic_prefix))
                    .is_empty(),
                "{name}: $mode could be a leading option"
            );
        }
        let glob_dynamic_prefix = [
            InvocationWord::Dynamic,
            InvocationWord::Literal("/tmp"),
            InvocationWord::Literal("*.tcl"),
        ];
        assert!(
            crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .pattern_args_words(
                    "glob",
                    InvocationArguments::structured(&glob_dynamic_prefix)
                )
                .is_empty(),
            "glob $mode /tmp *.tcl cannot choose a pattern tail before $mode resolves"
        );
        let string_dynamic_prefix = [
            InvocationWord::Literal("match"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("a*"),
            InvocationWord::Literal("abc"),
        ];
        assert!(
            crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .pattern_args_words(
                    "string",
                    InvocationArguments::structured(&string_dynamic_prefix),
                )
                .is_empty(),
            "string match $mode a* abc cannot treat $mode as the pattern"
        );
    }

    #[test]
    fn source_aware_pattern_layouts_accept_proven_options_and_terminators() {
        use crate::invocation_words::{InvocationArguments, InvocationWord};
        use crate::patterns::{PatternArg, PatternType};

        let regexp_start = [
            InvocationWord::Literal("-start"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("a+"),
            InvocationWord::Dynamic,
        ];
        assert_eq!(
            crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .pattern_args_words("regexp", InvocationArguments::structured(&regexp_start)),
            vec![PatternArg {
                index: 2,
                kind: PatternType::Regex,
            }],
            "a fixed value-taking option may safely carry a dynamic value"
        );
        let non_option_template = [InvocationWord::DynamicNonOption, InvocationWord::Dynamic];
        assert_eq!(
            crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .pattern_args_words(
                    "regexp",
                    InvocationArguments::structured(&non_option_template),
                ),
            vec![PatternArg {
                index: 0,
                kind: PatternType::Regex,
            }],
            "a template with a direct non-dash prefix remains a positional pattern"
        );
        let glob_terminator = [
            InvocationWord::Literal("--"),
            InvocationWord::Literal("-literal"),
        ];
        assert_eq!(
            crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .pattern_args_words("glob", InvocationArguments::structured(&glob_terminator)),
            vec![PatternArg {
                index: 1,
                kind: PatternType::Glob,
            }],
            "-- freezes glob's option prefix before an option-shaped pattern"
        );
    }

    #[test]
    fn source_aware_pattern_layouts_respect_release_and_reserved_suffixes() {
        use crate::invocation_words::{InvocationArguments, InvocationWord};
        use crate::patterns::{PatternArg, PatternType};

        let strict_abbreviation = [
            InvocationWord::Literal("-sta"),
            InvocationWord::Literal("2"),
            InvocationWord::Literal("a+"),
            InvocationWord::Literal("aaa"),
        ];
        assert!(
            crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .pattern_args_words(
                    "regexp",
                    InvocationArguments::structured(&strict_abbreviation)
                )
                .is_empty(),
            "regexp's strict option table rejects a non-exact abbreviation"
        );
        let stride_abbreviation = [
            InvocationWord::Literal("-str"),
            InvocationWord::Literal("2"),
            InvocationWord::Literal("{a b}"),
            InvocationWord::Literal("a+"),
        ];
        assert!(
            crate::model::ingress::static_context_for("tcl8.6")
                .commands()
                .pattern_args_words(
                    "lsearch",
                    InvocationArguments::structured(&stride_abbreviation)
                )
                .is_empty(),
            "a Tcl 9-only option abbreviation is invalid before Tcl 9"
        );
        assert_eq!(
            crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .pattern_args_words(
                    "lsearch",
                    InvocationArguments::structured(&stride_abbreviation),
                ),
            vec![PatternArg {
                index: 3,
                kind: PatternType::Glob,
            }],
            "the same abbreviation follows Tcl 9's profile-filtered option table"
        );
        let option_shaped_suffix = [
            InvocationWord::Literal("-regexp"),
            InvocationWord::Literal("--"),
        ];
        assert_eq!(
            crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .pattern_args_words(
                    "lsearch",
                    InvocationArguments::structured(&option_shaped_suffix),
                ),
            vec![PatternArg {
                index: 1,
                kind: PatternType::Glob,
            }],
            "lsearch's reserved list/pattern suffix remains positional"
        );
    }

    /// The source proof follows descriptors installed in the registry, not a
    /// closed list of Tcl command names. A pack or a later registry mutation
    /// can introduce the same option-dependent pattern/replacement grammar
    /// and all generic consumers receive the new answer unchanged.
    #[test]
    fn source_aware_pattern_and_format_queries_follow_mutated_descriptors() {
        use crate::invocation_words::{InvocationArguments, InvocationWord};
        use crate::patterns::{FormatType, PatternArg, PatternType};

        const OPTIONS: &[crate::hover::OptionSpec] = &[crate::hover::OptionSpec {
            name: "-skip",
            value: crate::hover::OptionValue::value("count"),
            detail: "fixture value",
            surface: None,
            aliases: &[],
            lifecycle: crate::lifecycle::Lifecycle::UNSPECIFIED,
            min_abbrev: None,
            effect: None,
        }];
        fn literal_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
            let first =
                crate::spec::leading_option_word_count_with(OPTIONS, args, PrefixMatching::Strict);
            [
                (first, ArgRole::Pattern),
                (first + 2, ArgRole::FormatString),
            ]
            .into_iter()
            .filter_map(|(index, role)| {
                (index < args.len())
                    .then(|| u8::try_from(index).ok().map(|index| (index, role)))
                    .flatten()
            })
            .collect()
        }
        fn layout_roles(
            args: InvocationArguments<'_>,
            options: crate::resolved_invocation::InvocationOptions<'_, '_>,
        ) -> Option<Vec<(u8, ArgRole)>> {
            let first = options.leading_word_count(args)?;
            let count = args.exact_argv_len()?;
            Some(
                [
                    (first, ArgRole::Pattern),
                    (first + 2, ArgRole::FormatString),
                ]
                .into_iter()
                .filter_map(|(index, role)| {
                    (index < count)
                        .then(|| u8::try_from(index).ok().map(|index| (index, role)))
                        .flatten()
                })
                .collect(),
            )
        }
        assert_eq!(
            literal_roles(&["-skip", "1", "a", "s", "replacement"]),
            vec![(2, ArgRole::Pattern), (4, ArgRole::FormatString)]
        );
        const SPEC: CommandSpec = CommandSpec {
            name: "mutated-pattern-format-owner",
            options: OPTIONS,
            prefix_matching: PrefixMatching::Strict,
            arg_role_layout_resolver: Some(layout_roles),
            pattern_type: Some(PatternType::Regex),
            format_string_type: Some(FormatType::Regsub),
            ..CommandSpec::DEFAULT
        };

        let mut registry = CommandRegistry::build_default();
        registry.insert(SPEC);
        let dynamic = [
            InvocationWord::Dynamic,
            InvocationWord::Literal("{a}"),
            InvocationWord::Dynamic,
            InvocationWord::Literal(r"{\1}"),
        ];
        assert!(
            registry
                .pattern_args_words(
                    "mutated-pattern-format-owner",
                    InvocationArguments::structured(&dynamic),
                )
                .is_empty()
        );
        assert!(
            registry
                .format_string_args_words(
                    "mutated-pattern-format-owner",
                    InvocationArguments::structured(&dynamic),
                )
                .is_empty()
        );

        let fixed = [
            InvocationWord::Literal("-skip"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("{a}"),
            InvocationWord::Dynamic,
            InvocationWord::Literal(r"{\1}"),
        ];
        assert_eq!(
            registry.pattern_args_words(
                "mutated-pattern-format-owner",
                InvocationArguments::structured(&fixed),
            ),
            vec![PatternArg {
                index: 2,
                kind: PatternType::Regex,
            }]
        );
        assert_eq!(
            registry.format_string_args_words(
                "mutated-pattern-format-owner",
                InvocationArguments::structured(&fixed),
            ),
            vec![FormatStringArg {
                index: 4,
                kind: FormatType::Regsub,
                scan: false,
            }]
        );
    }

    /// Pattern locations come from the owning specs' option descriptors, not
    /// from an LSP walk guessing where a `-start` value ends.  Exercise the
    /// three shapes prone to diverging: abbreviations, value-taking
    /// options, and glob's unbounded pattern tail.
    #[test]
    fn pattern_roles_follow_declared_option_prefixes() {
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.arg_indices_for_role("lsearch", &["-sta", "2", "{a b c}", "b*"], ArgRole::Pattern,),
            vec![3],
        );
        assert_eq!(
            reg.arg_indices_for_role("string", &["match", "-noc", "a*", "abc"], ArgRole::Pattern,),
            vec![2],
        );
        assert_eq!(
            reg.arg_indices_for_role("glob", &["-di", "tmp", "*.tcl", "*.tm"], ArgRole::Pattern,),
            vec![2, 3],
        );
        assert_eq!(
            reg.arg_indices_for_role("glob", &["--", "-literal", "*.tcl"], ArgRole::Pattern),
            vec![1, 2],
        );
        assert_eq!(
            reg.arg_indices_for_role("regexp", &["-start", "2", "a+", "aaa"], ArgRole::Pattern,),
            vec![2],
        );
        assert_eq!(
            reg.pattern_args("lsearch", &["-regexp", "{a b}", "a+"]),
            vec![crate::patterns::PatternArg {
                index: 2,
                kind: crate::patterns::PatternType::Regex,
            }],
        );
        assert!(
            reg.pattern_args("lsearch", &["-exact", "{a b}", "a+"])
                .is_empty()
        );
    }

    #[test]
    fn source_aware_lsearch_pattern_layout_respects_outer_option_boundary() {
        use crate::invocation_words::{InvocationArguments, InvocationWord};
        use crate::patterns::{PatternArg, PatternType};

        // Tcl's `Tcl_LsearchObjCmd` has used `for (i = 1; i < objc - 2;
        // i++)` since 8.4: it stops scanning exactly when list and pattern
        // remain. Keep the source-aware resolver aligned across every
        // supported core release, including the 9.0 -stride expansion.
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let reg = crate::model::ingress::static_context_for(dialect).commands();
            let proven_outer_option = [
                InvocationWord::Literal("-glob"),
                InvocationWord::Dynamic,
                InvocationWord::Literal("ba*"),
            ];
            assert_eq!(
                reg.pattern_args_words(
                    "lsearch",
                    InvocationArguments::structured(&proven_outer_option),
                ),
                vec![PatternArg {
                    index: 2,
                    kind: PatternType::Glob,
                }],
                "{dialect}: lsearch -glob $list {{ba*}} has proven operands and glob mode"
            );

            let expanded_list = [
                InvocationWord::Literal("-glob"),
                InvocationWord::Expanded,
                InvocationWord::Literal("ba*"),
            ];
            assert!(
                reg.pattern_args_words("lsearch", InvocationArguments::structured(&expanded_list))
                    .is_empty(),
                "{dialect}: lsearch -glob {{*}}$list {{ba*}} has no known argv suffix"
            );

            let ambiguous_option_prefix = [
                InvocationWord::Dynamic,
                InvocationWord::Literal("{a b}"),
                InvocationWord::Literal("a+"),
            ];
            // `$mode` remains inside the `i < objc - 2` option region and
            // may become a value-taking switch, so it cannot be reclassified
            // as the list operand to claim a glob pattern.
            assert!(
                reg.pattern_args_words(
                    "lsearch",
                    InvocationArguments::structured(&ambiguous_option_prefix),
                )
                .is_empty(),
                "{dialect}: lsearch $mode {{a b}} {{a+}} remains ambiguous"
            );
        }
    }

    #[test]
    fn lsearch_pattern_resolver_uses_profiled_options_and_the_reserved_suffix() {
        use crate::patterns::{PatternArg, PatternType};

        // These are oracle-shaped against Tcl's `i < objc - 2` scanner.  The
        // first two words of the short form are list + pattern, even though
        // both spell like lsearch switches.
        let option_shaped_operands = ["-regexp", "-glob"];
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let registry = crate::model::ingress::static_context_for(dialect).commands();
            let patterns = registry.pattern_args("lsearch", &option_shaped_operands);
            assert_eq!(
                patterns,
                vec![PatternArg {
                    index: 1,
                    kind: PatternType::Glob,
                }],
                "{dialect}: lsearch -regexp -glob keeps both mandatory operands out of option scanning"
            );
            assert_eq!(
                registry.arg_indices_for_role("lsearch", &option_shaped_operands, ArgRole::Pattern),
                patterns
                    .iter()
                    .map(|pattern| usize::from(pattern.index))
                    .collect::<Vec<_>>(),
                "{dialect}: ArgRole::Pattern must share the resolver answer"
            );
        }

        // `-str` can name only -stride in Tcl 9.  Pre-9 interpreters reject
        // it, so the resolver must abstain rather than treating an invalid
        // dash-prefixed word as the list operand and inventing a pattern.
        let stride_abbrev = ["-str", "2", "{a b}", "a+"];
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6"] {
            let registry = crate::model::ingress::static_context_for(dialect).commands();
            assert!(
                registry.pattern_args("lsearch", &stride_abbrev).is_empty(),
                "{dialect}: unavailable -stride abbreviation invalidates the invocation"
            );
            assert!(
                registry
                    .arg_indices_for_role("lsearch", &stride_abbrev, ArgRole::Pattern)
                    .is_empty(),
                "{dialect}: pattern roles abstain with the invalid invocation"
            );
        }
        for dialect in ["tcl9.0", "tcl9.1"] {
            let registry = crate::model::ingress::static_context_for(dialect).commands();
            let patterns = registry.pattern_args("lsearch", &stride_abbrev);
            assert_eq!(
                patterns,
                vec![PatternArg {
                    index: 3,
                    kind: PatternType::Glob,
                }],
                "{dialect}: -str is the available -stride abbreviation"
            );
            assert_eq!(
                registry.arg_indices_for_role("lsearch", &stride_abbrev, ArgRole::Pattern),
                vec![3],
                "{dialect}: roles, hover, and tokens share the Tcl 9 layout"
            );
        }

        // In Tcl 9 `-st` is ambiguous between -start and -stride. Unknown
        // words and unsupported `--` likewise cannot be reclassified as the
        // list operand while the mandatory suffix is still reserved.
        for args in [
            ["-st", "2", "{a b}", "a+"].as_slice(),
            ["-unknown", "{a b}", "a+"].as_slice(),
            ["--", "{a b}", "a+"].as_slice(),
        ] {
            let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
            assert!(
                registry.pattern_args("lsearch", args).is_empty(),
                "Tcl 9 invalid option prefix {args:?} must abstain"
            );
            assert!(
                registry
                    .arg_indices_for_role("lsearch", args, ArgRole::Pattern)
                    .is_empty(),
                "Tcl 9 invalid option prefix {args:?} must have no pattern role"
            );
        }

        // The final two words are always the mandatory list + pattern, even
        // when their spelling would otherwise be an invalid option.
        for args in [
            ["{a b}", "-unknown"].as_slice(),
            ["-regexp", "--"].as_slice(),
            ["-regexp", "-st"].as_slice(),
        ] {
            let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
            assert_eq!(
                registry.pattern_args("lsearch", args),
                vec![PatternArg {
                    index: 1,
                    kind: PatternType::Glob,
                }],
                "Tcl 9 final operands {args:?} stay positional"
            );
        }
    }

    #[test]
    fn source_aware_lsearch_dynamic_prefix_and_suffix_have_distinct_obligations() {
        use crate::invocation_words::{InvocationArguments, InvocationWord};
        use crate::patterns::{PatternArg, PatternType};

        for dialect in ["tcl8.6", "tcl9.0"] {
            let registry = crate::model::ingress::static_context_for(dialect).commands();
            let dynamic_early = [
                InvocationWord::Dynamic,
                InvocationWord::Literal("{a b}"),
                InvocationWord::Literal("a+"),
            ];
            assert!(
                registry
                    .pattern_args_words("lsearch", InvocationArguments::structured(&dynamic_early))
                    .is_empty(),
                "{dialect}: a dynamic word before the mandatory operands can still be a switch"
            );

            let dynamic_list_operand = [
                InvocationWord::Literal("-regexp"),
                InvocationWord::Dynamic,
                InvocationWord::Literal("a+"),
            ];
            assert_eq!(
                registry.pattern_args_words(
                    "lsearch",
                    InvocationArguments::structured(&dynamic_list_operand),
                ),
                vec![PatternArg {
                    index: 2,
                    kind: PatternType::Regex,
                }],
                "{dialect}: the dynamic mandatory list is beyond the option boundary"
            );
        }
    }

    /// The repeated argument tails a fixed index table cannot
    /// express answer through the ordinary role query.
    #[test]
    fn repeated_layouts_answer_through_arg_indices_for_role() {
        let reg = CommandRegistry::build_default();
        let vars =
            |name: &str, args: &[&str]| reg.arg_indices_for_role(name, args, ArgRole::VarWrite);
        // TP — every argument of `global`.
        assert_eq!(vars("global", &["a", "b", "c"]), vec![0, 1, 2]);
        // TP — every *even* argument of `variable` (names, not values).
        assert_eq!(vars("variable", &["x", "1", "y", "2"]), vec![0, 2]);
        // TP — the local of each `namespace upvar` pair, past the namespace.
        assert_eq!(
            vars("namespace", &["upvar", "::ns", "o1", "l1", "o2", "l2"]),
            vec![3, 5]
        );
        // TP — each `dict update` varName, with the trailing body excluded.
        // Index 1 is the dictionary variable itself, which the subcommand
        // already declared (it is read on entry and written back after the
        // body).  The pair locals answer as `LoopVarList` — bound once
        // before the body, and only when the key is present — NOT as
        // `VarWrite`: an unconditional SSA def both defeated the key-aware
        // read-before-set suppression and would hide the genuine
        // absent-key warning.
        assert_eq!(
            vars("dict", &["update", "d", "k1", "v1", "k2", "v2", "{body}"]),
            vec![1]
        );
        assert_eq!(
            reg.arg_indices_for_role(
                "dict",
                &["update", "d", "k1", "v1", "k2", "v2", "{body}"],
                ArgRole::LoopVarList
            ),
            vec![3, 5]
        );
        // TP — `foreach` / `lmap` variable specs, body excluded.
        for name in ["foreach", "lmap"] {
            assert_eq!(
                reg.arg_indices_for_role(
                    name,
                    &["{a b}", "$l1", "c", "$l2", "{body}"],
                    ArgRole::LoopVarList
                ),
                vec![0, 2],
                "{name}"
            );
        }
        // FN guard — the explicitly global spellings resolve identically.
        assert_eq!(vars("::global", &["a", "b"]), vars("global", &["a", "b"]));
        // TN — a command with no repeated tail is unaffected.
        assert_eq!(vars("puts", &["a", "b"]), Vec::<usize>::new());
        // FP guard — a short call names nothing it should not.
        assert_eq!(vars("global", &[]), Vec::<usize>::new());
        assert_eq!(
            reg.arg_indices_for_role("foreach", &["{body}"], ArgRole::LoopVarList),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn option_value_arity_shifts_registry_argument_roles() {
        let reg = CommandRegistry::build_default()
            .project_for_profile(DialectProfile::find("tcl8.6").unwrap());
        assert_eq!(
            reg.arg_indices_for_role(
                "regexp",
                &["-start", "2", "pattern", "$text", "match"],
                ArgRole::VarWrite,
            ),
            vec![4]
        );
        assert_eq!(
            reg.arg_indices_for_role(
                "regsub",
                &["-start", "2", "pattern", "$text", "replacement", "result"],
                ArgRole::VarWrite,
            ),
            vec![5]
        );
        assert_eq!(
            reg.arg_indices_for_role("upvar", &["0", "remote", "local"], ArgRole::VarWrite,),
            vec![2]
        );
    }

    /// `for`'s three script arguments share the semantic
    /// [`ArgRole::Body`], and the *presentation* fact is what separates the
    /// trailing body (block-expanded) from `start` / `next` (inline).
    #[test]
    fn for_declares_inline_presentation_for_start_and_next() {
        use crate::presentation::ArgPresentation;
        let reg = CommandRegistry::build_default();
        let args = ["{set i 0}", "{$i < 3}", "{incr i}", "{puts $i}"];
        // Semantics unchanged: all three scripts are still bodies.
        assert_eq!(
            reg.arg_indices_for_role("for", &args, ArgRole::Body),
            vec![0, 2, 3]
        );
        // Presentation splits them.
        assert_eq!(
            reg.arg_presentation("for", &args, 0),
            ArgPresentation::InlineScript
        );
        assert_eq!(
            reg.arg_presentation("for", &args, 2),
            ArgPresentation::InlineScript
        );
        assert_eq!(
            reg.arg_presentation("for", &args, 3),
            ArgPresentation::BlockScript
        );
        assert!(reg.arg_presentation("for", &args, 3).is_block());
        // The absolute global spelling resolves to the same spec.
        assert_eq!(
            reg.arg_presentation("::for", &args, 0),
            ArgPresentation::InlineScript
        );
        // A command with nothing to say answers the block default, and so
        // does one the registry does not know at all.
        assert_eq!(
            reg.arg_presentation("while", &["{$x}", "{body}"], 1),
            ArgPresentation::BlockScript
        );
        assert_eq!(
            reg.arg_presentation("no::such::command", &["{a}"], 0),
            ArgPresentation::BlockScript
        );
    }

    // Option-value roles.

    fn opt(name: &'static str, value: crate::hover::OptionValue) -> crate::hover::OptionSpec {
        crate::hover::OptionSpec {
            name,
            value,
            ..crate::hover::OptionSpec::DEFAULT
        }
    }

    fn opt_with_alias(
        name: &'static str,
        aliases: &'static [&'static str],
        value: crate::hover::OptionValue,
    ) -> crate::hover::OptionSpec {
        crate::hover::OptionSpec {
            name,
            value,
            aliases,
            ..crate::hover::OptionSpec::DEFAULT
        }
    }

    fn indices(options: &[crate::hover::OptionSpec], args: &[&str], role: ArgRole) -> Vec<usize> {
        let mut out = Vec::new();
        push_option_value_roles(
            &mut out,
            &options.iter().collect::<Vec<_>>(),
            args,
            0,
            &|wanted| wanted == role,
            PrefixMatching::Enabled,
        );
        out.into_iter().map(|(index, _)| index).collect()
    }

    #[test]
    fn option_value_role_emits_body_index() {
        use crate::hover::OptionValue;
        let options = [
            opt("-command", OptionValue::script()),
            opt("-flag", OptionValue::flag()),
        ];
        let args = ["-command", "{puts hi}", "-flag", "x"];
        // The `-command` value is a Body; the flag consumes nothing.
        assert_eq!(indices(&options, &args, ArgRole::Body), vec![1]);
        // It is not a generic Value role.
        assert!(indices(&options, &args, ArgRole::Value).is_empty());
    }

    /// Explicit metadata environment; this does not install a source receiver.
    fn callback_metadata_registry() -> CommandRegistry {
        let mut registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
        for (package, version) in [("Tk", "9.1"), ("tcltest", "2.5.8"), ("bibtex", "0.8")] {
            registry.insert_ambient_package(package, version);
        }
        registry
    }

    #[test]
    fn script_timing_is_exact_for_mixed_immediate_and_deferred_arguments() {
        use crate::hover::{OptionSpec, OptionValue, ScriptTiming};

        static OPTIONS: &[OptionSpec] = &[OptionSpec {
            name: "-later",
            value: OptionValue::deferred_script(),
            detail: "stored callback",
            ..OptionSpec::DEFAULT
        }];
        let mut reg = CommandRegistry::build_default();
        reg.insert(CommandSpec {
            name: "mixed-script-timing",
            arity: crate::Arity::exact(3),
            arg_roles: &[(0, ArgRole::Body)],
            options: OPTIONS,
            ..CommandSpec::DEFAULT
        });

        let args = ["{error now}", "-later", "{error later}"];
        assert_eq!(
            reg.script_timing("mixed-script-timing", &args, 0, None),
            Some(ScriptTiming::SameInvocation),
        );
        assert_eq!(
            reg.script_timing("mixed-script-timing", &args, 2, None),
            Some(ScriptTiming::Deferred),
        );
        assert_eq!(
            reg.script_timing("mixed-script-timing", &args, 1, None),
            None,
        );
    }

    #[test]
    fn shipped_script_options_distinguish_tk_callbacks_from_tcltest_bodies() {
        use crate::hover::ScriptTiming;

        let reg = callback_metadata_registry();
        let button = [".b", "-command", "{return}"];
        assert_eq!(
            reg.script_timing("button", &button, 2, None),
            Some(ScriptTiming::Deferred),
        );

        let test = ["sample", "description", "-body", "{error failure}"];
        assert_eq!(
            reg.script_timing("tcltest::test", &test, 3, None),
            Some(ScriptTiming::SameInvocation),
        );

        let dump = ["dump", "-command", "visit", "1.0"];
        assert_eq!(
            reg.instance_script_timing("text", ".receiver", &dump, 2, None),
            Some(ScriptTiming::SameInvocation),
            "text dump calls its prefix synchronously for each returned item"
        );
        let sync = ["sync", "-command", "ready"];
        assert_eq!(
            reg.instance_script_timing("text", ".receiver", &sync, 2, None),
            Some(ScriptTiming::Deferred),
            "text sync schedules its prefix after metrics become current"
        );
    }

    #[test]
    fn callback_taint_inputs_are_exact_and_exclude_tk_bookkeeping() {
        use crate::CallbackTaintInput;

        let reg = callback_metadata_registry();
        let validation = [".entry", "-validatecommand", "{eval %P}"];
        assert_eq!(
            reg.callback_taint_inputs("entry", &validation, 2, None),
            &[
                CallbackTaintInput::TK_PROPOSED_VALUE,
                CallbackTaintInput::TK_CURRENT_VALUE,
                CallbackTaintInput::TK_EDIT_TEXT,
            ],
        );
        let bind = [".entry", "<Key>", "{eval %A}"];
        assert_eq!(
            reg.callback_taint_inputs("bind", &bind, 2, None),
            &[CallbackTaintInput::TK_EVENT_CHAR],
        );
        let plain_command = [".entry", "-command", "{eval %P}"];
        assert!(
            reg.callback_taint_inputs("button", &plain_command, 2, None)
                .is_empty(),
            "a deferred callback without user substitutions must stay clean"
        );

        let canvas_bind = ["bi", "item", "<Key>", "{eval %A}"];
        assert_eq!(
            reg.instance_callback_taint_inputs("canvas", ".canvas", &canvas_bind, 3, None),
            &[CallbackTaintInput::TK_EVENT_CHAR],
            "a unique-prefix widget subcommand must select its callback table"
        );

        let abbreviated_configure = ["conf", "-validatecommand", "{eval %P}"];
        assert_eq!(
            reg.instance_callback_taint_inputs("entry", ".entry", &abbreviated_configure, 2, None,),
            &[
                CallbackTaintInput::TK_PROPOSED_VALUE,
                CallbackTaintInput::TK_CURRENT_VALUE,
                CallbackTaintInput::TK_EDIT_TEXT,
            ],
            "a unique-prefix widget method must inherit constructor callback options"
        );

        let instance_bind = ["bind", "all", "<Key>", "{eval %A}"];
        assert_eq!(
            reg.instance_callback_taint_inputs("canvas", ".canvas", &instance_bind, 3, None,),
            &[CallbackTaintInput::TK_EVENT_CHAR],
            "a positional instance-method callback must use its method table"
        );
        assert!(
            reg.instance_callback_taint_inputs(
                "entry",
                ".entry",
                &["configure", "-validatecommand"],
                1,
                None,
            )
            .is_empty(),
            "the option word itself is not callback text"
        );
    }

    /// The registry states which stored scripts are callbacks — the words a
    /// consumer must treat as run later, outside the registering frame — and
    /// which are a definition's body or run now.
    #[test]
    fn callback_scripts_are_the_deferred_words_of_a_command_that_stores_no_definition() {
        let reg = CommandRegistry::build_default();
        let callbacks: &[(&str, &[&str], &[usize])] = &[
            ("after", &["100", "{set done 1}"], &[1]),
            ("after", &["idle", "{set done 1}"], &[1]),
            (
                "trace",
                &["add", "variable", "x", "write", "{set go 0 ;#}"],
                &[4],
            ),
            ("bind", &[".", "<Key>", "{set go 0}"], &[2]),
            ("fileevent", &["stdin", "readable", "{set go 0}"], &[2]),
            ("chan", &["event", "stdin", "readable", "{set go 0}"], &[3]),
            ("interp", &["bgerror", "{}", "{set go 0 ;#}"], &[2]),
            ("button", &[".b", "-command", "{set go 0}"], &[2]),
        ];
        for (name, args, expected) in callbacks {
            assert_eq!(
                reg.callback_script_indices(name, args, None),
                *expected,
                "{name} {args:?}"
            );
        }
        // A definition's body is dormant and runs in a frame of its own, and
        // the scripts these run now or only name are no callback.
        let none: &[(&str, &[&str])] = &[
            ("proc", &["p", "{}", "{set x 1}"]),
            ("snit::method", &["T", "m", "{}", "{set x 1}"]),
            ("lambda", &["{}", "{set x 1}"]),
            ("catch", &["{set x 1}"]),
            ("if", &["1", "{set x 1}"]),
            ("uplevel", &["#0", "{set x 1}"]),
            ("trace", &["remove", "variable", "x", "write", "cb"]),
            ("set", &["x", "1"]),
        ];
        for (name, args) in none {
            assert!(
                reg.callback_script_indices(name, args, None).is_empty(),
                "{name} {args:?}"
            );
        }
    }

    #[test]
    fn typed_callback_inputs_preserve_unknown_prefix_slots_and_decline_expansion() {
        use crate::{CallbackTaintInput, InvocationWord, InvocationWords};
        let registry = callback_metadata_registry();
        let unknown_window = [
            InvocationWord::Dynamic,
            InvocationWord::Literal("<Key>"),
            InvocationWord::Literal("eval %A"),
        ];
        let invocation =
            InvocationWords::structured(InvocationWord::Literal("bind"), &unknown_window);
        assert_eq!(
            registry.callback_taint_inputs_words(invocation, 2, None),
            &[CallbackTaintInput::TK_EVENT_CHAR]
        );
        assert_eq!(
            registry.callback_taint_inputs_words(invocation, 1, None),
            &[] as &[CallbackTaintInput]
        );
        for arguments in [&unknown_window[..1], &unknown_window[..2]] {
            assert_eq!(
                registry.callback_taint_inputs_words(
                    InvocationWords::structured(InvocationWord::Literal("bind"), arguments),
                    2,
                    None,
                ),
                &[] as &[CallbackTaintInput]
            );
        }
        let invalid = [
            InvocationWord::Dynamic,
            InvocationWord::Literal("<Key>"),
            InvocationWord::Literal("eval %A"),
            InvocationWord::Literal("extra"),
        ];
        assert_eq!(
            registry.callback_taint_inputs_words(
                InvocationWords::structured(InvocationWord::Literal("bind"), &invalid),
                2,
                None,
            ),
            &[] as &[CallbackTaintInput]
        );
        let canvas = [
            InvocationWord::Literal("bind"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("<Key>"),
            InvocationWord::Literal("eval %A"),
        ];
        assert_eq!(
            registry.instance_callback_taint_inputs_words(
                "canvas",
                InvocationWords::structured(InvocationWord::Literal(".canvas"), &canvas),
                3,
                None,
            ),
            &[CallbackTaintInput::TK_EVENT_CHAR]
        );
        let expanded_window = [InvocationWord::Expanded, InvocationWord::Literal("eval %A")];
        assert_eq!(
            registry.callback_taint_inputs_words(
                InvocationWords::structured(InvocationWord::Literal("bind"), &expanded_window),
                1,
                None,
            ),
            &[] as &[CallbackTaintInput]
        );
        let unknown_option = [
            InvocationWord::Literal(".entry"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("eval %P"),
        ];
        assert_eq!(
            registry.callback_taint_inputs_words(
                InvocationWords::structured(InvocationWord::Literal("entry"), &unknown_option),
                2,
                None,
            ),
            &[] as &[CallbackTaintInput]
        );
    }

    #[test]
    fn typed_callback_inputs_do_not_override_argument_sensitive_timing() {
        use crate::hover::ScriptTiming;
        use crate::{CallbackTaintInput, InvocationWord, InvocationWords};
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "timed-callback",
            arity: crate::Arity::exact(2),
            arg_roles: &[(1, ArgRole::Body)],
            traits: Traits::DEFERS_BODY,
            callback_taint_inputs: &[(1, &[CallbackTaintInput::TK_EVENT_CHAR])],
            script_timing_resolver: Some(|_| vec![(1, ScriptTiming::SameInvocation)]),
            ..CommandSpec::DEFAULT
        });
        let unknown = [InvocationWord::Dynamic, InvocationWord::Literal("eval %A")];
        assert_eq!(
            registry.callback_taint_inputs_words(
                InvocationWords::structured(InvocationWord::Literal("timed-callback"), &unknown),
                1,
                None,
            ),
            &[] as &[CallbackTaintInput]
        );
        let known = [
            InvocationWord::Literal("window"),
            InvocationWord::Literal("eval %A"),
        ];
        assert_eq!(
            registry.callback_taint_inputs_words(
                InvocationWords::structured(InvocationWord::Literal("timed-callback"), &known),
                1,
                None,
            ),
            &[] as &[CallbackTaintInput]
        );
    }

    #[test]
    fn argument_sensitive_timing_distinguishes_registration_and_send_forms() {
        use crate::hover::ScriptTiming;

        let reg = callback_metadata_registry();
        assert_eq!(
            reg.instance_script_timing(
                "canvas",
                ".receiver",
                &["bind", "tag", "<Button-1>", "clicked"],
                3,
                None,
            ),
            Some(ScriptTiming::Deferred),
        );
        assert_eq!(
            reg.instance_script_timing(
                "canvas",
                ".receiver",
                &["bi", "tag", "<Button-1>", "clicked"],
                3,
                None,
            ),
            Some(ScriptTiming::Deferred),
            "canvas bind's unique prefix must select the positional timing resolver",
        );
        assert_eq!(
            reg.script_timing(
                "wm",
                &["protocol", ".w", "WM_DELETE_WINDOW", "close"],
                3,
                None,
            ),
            Some(ScriptTiming::Deferred),
        );
        assert_eq!(
            reg.script_timing("wm", &["prot", ".w", "WM_DELETE_WINDOW", "close"], 3, None,),
            Some(ScriptTiming::Deferred),
            "wm protocol's unique prefix must not fall back to same-invocation timing",
        );
        assert_eq!(
            reg.script_timing("send", &["other", "work"], 1, None),
            Some(ScriptTiming::SameInvocation),
        );
        assert_eq!(
            reg.script_timing("send", &["-async", "other", "work"], 2, None,),
            Some(ScriptTiming::Deferred),
        );
        assert_eq!(
            reg.script_timing("send", &["-async", "other", "work", "argument"], 2, None,),
            None,
            "a concatenated multi-word script is not a fictional Body position",
        );
        assert_eq!(
            reg.script_timing(
                "selection",
                &["handle", "-type", "UTF8_STRING", ".w", "provide"],
                4,
                None,
            ),
            Some(ScriptTiming::Deferred),
        );
        assert_eq!(
            reg.script_timing(
                "selection",
                &["h", "-type", "UTF8_STRING", ".w", "provide"],
                4,
                None,
            ),
            Some(ScriptTiming::Deferred),
            "selection handle's unique prefix must select its positional timing resolver",
        );
        assert_eq!(
            reg.script_timing("selection", &["o", "-command", "lost", ".w"], 2, None,),
            Some(ScriptTiming::Deferred),
            "subcommand option timing must use the same unique-prefix resolution",
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn command_prefix_hosts_report_their_exact_phase() {
        use crate::hover::ScriptTiming;

        let reg = callback_metadata_registry();
        let cases: &[(&str, &[&str], usize, ScriptTiming)] = &[
            // Immediate consumers execute the prefix before returning.
            (
                "lsort",
                &["-command", "compare", "{b a}"],
                1,
                ScriptTiming::SameInvocation,
            ),
            (
                "coroprobe",
                &["worker", "inspect"],
                1,
                ScriptTiming::SameInvocation,
            ),
            (
                "generator",
                &["reduce", "combine", "0", "values"],
                1,
                ScriptTiming::SameInvocation,
            ),
            // Registration and asynchronous forms store the prefix.
            (
                "socket",
                &["-server", "accept", "0"],
                1,
                ScriptTiming::Deferred,
            ),
            (
                "fcopy",
                &["in", "out", "-command", "done"],
                3,
                ScriptTiming::Deferred,
            ),
            (
                "chan",
                &["copy", "in", "out", "-command", "done"],
                4,
                ScriptTiming::Deferred,
            ),
            (
                "interp",
                &["alias", "", "aliasName", "", "target"],
                4,
                ScriptTiming::Deferred,
            ),
            (
                "interp",
                &["bgerror", "", "handler"],
                2,
                ScriptTiming::Deferred,
            ),
            (
                "namespace",
                &["unknown", "handler"],
                1,
                ScriptTiming::Deferred,
            ),
            (
                "package",
                &["unknown", "handler"],
                1,
                ScriptTiming::Deferred,
            ),
            (
                "coroinject",
                &["worker", "handler"],
                1,
                ScriptTiming::Deferred,
            ),
            (
                "tcltest::customMatch",
                &["mode", "handler"],
                1,
                ScriptTiming::Deferred,
            ),
            (
                "trace",
                &["add", "variable", "v", "write", "handler"],
                4,
                ScriptTiming::Deferred,
            ),
            (
                "generator",
                &["map", "transform", "values"],
                1,
                ScriptTiming::Deferred,
            ),
            (
                "hook",
                &["bind", "subject", "name", "observer", "handler"],
                4,
                ScriptTiming::Deferred,
            ),
            (
                "comm::comm",
                &["send", "-command", "handler", "peer", "work"],
                2,
                ScriptTiming::Deferred,
            ),
            (
                "mime::getbody",
                &["token", "-command", "handler"],
                2,
                ScriptTiming::Deferred,
            ),
            (
                "stringprep::register",
                &["profile", "-prohibitedCommand", "handler"],
                2,
                ScriptTiming::Deferred,
            ),
            (
                "tcl::chan::halfpipe",
                &["-close-command", "handler"],
                1,
                ScriptTiming::Deferred,
            ),
            // Removal forms name executable text without running or storing it.
            (
                "trace",
                &["remove", "variable", "v", "write", "handler"],
                4,
                ScriptTiming::ReferenceOnly,
            ),
        ];

        for &(command, args, index, expected) in cases {
            assert_eq!(
                reg.script_timing(command, args, index, None),
                Some(expected),
                "wrong phase for {command} {args:?} argument {index}",
            );
        }

        assert_eq!(
            reg.script_timing("trace", &["a", "v", "v", "write", "handler"], 4, None,),
            Some(ScriptTiming::Deferred),
            "unique-prefix subcommands and trace types preserve phase metadata",
        );
    }

    #[test]
    fn legacy_trace_prefix_timing_requires_a_release_with_that_form() {
        use crate::hover::ScriptTiming;
        for (profile, available) in [
            ("tcl8.4", true),
            ("tcl8.5", true),
            ("tcl8.6", true),
            ("tcl9.0", false),
            ("tcl9.1", false),
        ] {
            let owner = crate::model::ingress::static_context_for(profile);
            let registry = owner.commands();
            for (operation, timing) in [
                ("variable", ScriptTiming::Deferred),
                ("vdelete", ScriptTiming::ReferenceOnly),
            ] {
                assert_eq!(
                    registry.script_timing("trace", &[operation, "v", "w", "handler"], 3, None),
                    available.then_some(timing),
                    "{profile}: {operation}"
                );
            }
        }
    }

    #[test]
    fn option_timing_resolver_overrides_static_default_for_bibtex_input_mode() {
        use crate::hover::ScriptTiming;

        let reg = callback_metadata_registry();
        let in_memory = ["-recordcommand", "record", "@book{x}"];
        assert_eq!(
            reg.script_timing("bibtex::parse", &in_memory, 1, None),
            Some(ScriptTiming::SameInvocation),
        );

        let channel = ["-recordcommand", "record", "-channel", "input"];
        assert_eq!(
            reg.script_timing("bibtex::parse", &channel, 1, None),
            Some(ScriptTiming::Deferred),
        );

        let completion = ["-command", "done", "-channel", "input"];
        assert_eq!(
            reg.script_timing("bibtex::parse", &completion, 1, None),
            Some(ScriptTiming::Deferred),
        );
    }

    #[test]
    fn instance_timing_requires_the_selected_provider_and_method() {
        use crate::hover::ScriptTiming;

        let provided = callback_metadata_registry();
        let arguments = ["dump", "-command", "visit", "1.0"];
        assert_eq!(
            provided.instance_script_timing("text", ".receiver", &arguments, 2, None),
            Some(ScriptTiming::SameInvocation),
        );
        let unprovided = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
        assert_eq!(
            unprovided.instance_script_timing("text", ".receiver", &arguments, 2, None),
            None,
        );
        assert_eq!(
            provided.instance_script_timing("text", ".receiver", &["unknown", "visit"], 1, None),
            None,
        );
        assert_eq!(
            provided.instance_script_timing("text", ".receiver", &arguments, 1, None),
            None,
            "an option name is not executable callback text",
        );
    }

    #[test]
    fn constructor_option_callbacks_keep_their_original_argument_positions() {
        use crate::hover::ScriptTiming;
        let registry = callback_metadata_registry();
        let constructor = [".canvas", "-xscrollcommand", "visit"];
        assert_eq!(
            registry.script_timing("canvas", &constructor, 2, None),
            Some(ScriptTiming::Deferred),
        );
        for index in [0, 1] {
            assert_eq!(
                registry.script_timing("canvas", &constructor, index, None),
                None
            );
        }
        assert_eq!(
            registry.script_timing("canvas", &[".canvas", "-xscrollcommand"], 2, None),
            None,
        );
        let method = ["configure", "-xscrollcommand", "visit"];
        assert_eq!(
            registry.instance_script_timing("canvas", ".canvas", &method, 2, None),
            Some(ScriptTiming::Deferred),
        );
        assert_eq!(
            registry.instance_script_timing("canvas", ".canvas", &method, 1, None),
            None,
        );
        for query in [vec!["configure"], vec!["configure", "-xscrollcommand"]] {
            let invocation = registry
                .resolve_instance_invocation("canvas", ".canvas", &query, None)
                .expect("the provided classic widget retains its configure query");
            assert!(!invocation.semantics.mutator);
            assert!(invocation.semantics.traits.contains(Traits::PURE));
            assert_eq!(
                registry.instance_script_timing("canvas", ".canvas", &query, 1, None),
                None,
                "query forms do not execute the stored option value",
            );
        }
        let setter = registry
            .resolve_instance_invocation("canvas", ".canvas", &method, None)
            .expect("the provided classic widget retains its configure setter");
        assert!(setter.semantics.mutator);
        assert!(
            setter
                .semantics
                .traits
                .contains(Traits::CONFIGURES_INSTANCE_OPTIONS)
        );
        let cget = ["cget", "-xscrollcommand"];
        let invocation = registry
            .resolve_instance_invocation("canvas", ".canvas", &cget, None)
            .expect("the provided classic widget retains its cget method");
        assert!(!invocation.semantics.mutator);
        assert_eq!(
            registry.instance_script_timing("canvas", ".canvas", &cget, 1, None),
            None,
        );
    }

    #[test]
    fn known_stored_tk_command_prefixes_are_deferred() {
        use crate::hover::ScriptTiming;

        let reg = callback_metadata_registry();
        let stored = [
            ("canvas", "-xscrollcommand"),
            ("canvas", "-yscrollcommand"),
            ("entry", "-xscrollcommand"),
            ("listbox", "-xscrollcommand"),
            ("listbox", "-yscrollcommand"),
            ("menu", "-tearoffcommand"),
            ("scale", "-command"),
            ("scrollbar", "-command"),
            ("spinbox", "-xscrollcommand"),
            ("text", "-xscrollcommand"),
            ("text", "-yscrollcommand"),
            ("ttk::combobox", "-xscrollcommand"),
            ("ttk::entry", "-xscrollcommand"),
            ("ttk::scale", "-command"),
            ("ttk::scrollbar", "-command"),
            ("ttk::spinbox", "-xscrollcommand"),
            ("ttk::treeview", "-xscrollcommand"),
            ("ttk::treeview", "-yscrollcommand"),
            ("tk_chooseDirectory", "-command"),
            ("tk_getOpenFile", "-command"),
            ("tk_getSaveFile", "-command"),
            ("tk_messageBox", "-command"),
        ];
        for (command, option) in stored {
            let spec = reg
                .get_for_surface(command, reg.own_surface_query())
                .unwrap();
            let arguments = if spec.constructor_prefix_words().is_some() {
                vec![".w", option, "callback"]
            } else {
                vec![option, "callback"]
            };
            assert_eq!(
                reg.script_timing(command, &arguments, arguments.len() - 1, None),
                Some(ScriptTiming::Deferred),
                "{command} {option} stores its command prefix",
            );
        }

        assert_eq!(
            reg.instance_script_timing(
                "text",
                ".receiver",
                &["dump", "-command", "visit", "1.0"],
                2,
                None,
            ),
            Some(ScriptTiming::SameInvocation),
        );
        assert_eq!(
            reg.instance_script_timing(
                "ttk::treeview",
                ".receiver",
                &["sort", "root", "-command", "compare"],
                3,
                None,
            ),
            Some(ScriptTiming::SameInvocation),
        );
    }

    #[test]
    fn option_value_fixed_arity_emits_n_indices() {
        use crate::hover::OptionValue;
        let options = [opt("-rect", OptionValue::fixed(4, ArgRole::Value, "coord"))];
        let args = ["-rect", "1", "2", "3", "4", "tail"];
        assert_eq!(indices(&options, &args, ArgRole::Value), vec![1, 2, 3, 4]);
    }

    /// An authored hook owns its own termination policy.
    fn rest_value(args: &[&str], start: usize) -> crate::hover::OptionValueOutcome {
        crate::hover::OptionValueOutcome {
            words: args[start..]
                .iter()
                .position(|word| *word == "--")
                .unwrap_or(args.len() - start),
            invalid: None,
        }
    }

    #[test]
    fn option_value_rest_arity_stops_at_terminator() {
        let rest = crate::hover::OptionSpec {
            name: "-rest",
            value: crate::hover::OptionValue::Takes(crate::hover::OptionArg {
                arity: crate::hover::OptionArity::Hook(rest_value),
                ..crate::hover::OptionArg::DEFAULT
            }),
            ..crate::hover::OptionSpec::DEFAULT
        };
        let options = [rest];
        let args = ["-rest", "a", "b", "c", "--", "d"];
        // Consumes a, b, c up to `--`, not d.
        assert_eq!(indices(&options, &args, ArgRole::Value), vec![1, 2, 3]);
    }

    #[test]
    fn option_value_terminator_stops_scan() {
        use crate::hover::OptionValue;
        let options = [opt("-command", OptionValue::script())];
        let args = ["--", "-command", "{x}"];
        assert!(indices(&options, &args, ArgRole::Body).is_empty());
    }

    #[test]
    fn option_value_two_way_var_name_emits_for_both_roles() {
        use crate::hover::OptionValue;
        let options = [opt("-textvariable", OptionValue::var_name())];
        let args = ["-textvariable", "myvar"];
        assert_eq!(indices(&options, &args, ArgRole::VarWrite), vec![1]);
        assert_eq!(indices(&options, &args, ArgRole::VarRead), vec![1]);
    }

    #[test]
    fn option_value_alias_matches_and_dynamic_flag_skipped() {
        use crate::hover::OptionValue;
        let options = [opt_with_alias("-command", &["-cmd"], OptionValue::script())];
        // Alias resolves to the same value role.
        assert_eq!(indices(&options, &["-cmd", "{x}"], ArgRole::Body), vec![1]);
        // A `$var` in flag position isn't an option name → treated as a
        // positional, so the real `-command` after it still resolves.
        assert_eq!(
            indices(&options, &["$opt", "v", "-command", "{x}"], ArgRole::Body),
            vec![3]
        );
    }

    #[test]
    fn options_do_not_leak_into_positional_role_queries() {
        // A real command with value-taking options but no role annotations
        // must not surface any option value under Body/VarWrite (inert until
        // annotated).
        let reg = CommandRegistry::build_default();
        let b = reg.arg_indices_for_role("entry", &[".e", "-width", "10"], ArgRole::Body);
        assert!(b.is_empty(), "{b:?}");
    }

    #[test]
    fn command_prefix_option_is_captured_but_never_a_body() {
        // `lsort -command cmp {a b}` — the `-command` value is a CommandPrefix,
        // so it is captured under that role but never returned for a
        // Body query, i.e. a bareword prefix is not recursed as a script.
        let reg = CommandRegistry::build_default();
        let args = ["-command", "cmp", "{a b}"];
        assert!(
            reg.arg_indices_for_role("lsort", &args, ArgRole::Body)
                .is_empty(),
            "a command prefix must not be recursed as a body",
        );
        assert_eq!(
            reg.arg_indices_for_role("lsort", &args, ArgRole::CommandPrefix),
            vec![1],
            "the -command value should carry the CommandPrefix role",
        );
    }

    #[test]
    fn command_prefixes_carry_verified_arities() {
        // Ground-truthed vs real tclsh (stable 8.6→9.0). `command_prefixes`
        // is the single source of truth for both the position and the
        // appended arity.
        let reg = CommandRegistry::build_default();
        // `lsort -command cmp {a b}` → cmp invoked as `cmp x y` (2 appended).
        assert_eq!(
            reg.command_prefixes("lsort", &["-command", "cmp", "{a b}"]),
            vec![(1, AppendedArity::Exactly(2))],
        );
        // `socket -server accept 9000` → accept invoked as `accept ch a p` (3).
        assert_eq!(
            reg.command_prefixes("socket", &["-server", "accept", "9000"]),
            vec![(1, AppendedArity::Exactly(3))],
        );
        // Positional: `tcltest::customMatch mode cmd`
        // → `cmd expected actual` (2).
        assert_eq!(
            reg.command_prefixes("tcltest::customMatch", &["exact", "cmp"]),
            vec![(1, AppendedArity::Exactly(2))],
        );
        // Dynamic resolver: `selection handle window cmd` → the
        // last arg, invoked as `cmd offset maxChars` (2).
        assert_eq!(
            reg.command_prefixes("selection", &["handle", ".w", "getData"]),
            vec![(2, AppendedArity::Exactly(2))],
        );
    }

    #[test]
    fn command_prefixes_cover_core_callback_commands() {
        // Ground-truthed vs real tclsh 8.6/9.0 (stable). Locks in the
        // coverage of trace / interp / tcllib callbacks.
        let reg = CommandRegistry::build_default();
        // `trace add variable v w cb` → cb(name1 name2 op) = 3.
        assert_eq!(
            reg.command_prefixes("trace", &["add", "variable", "v", "write", "cb"]),
            vec![(4, AppendedArity::Exactly(3))],
        );
        // `trace add command c ops cb` → cb(old new op) = 3.
        assert_eq!(
            reg.command_prefixes("trace", &["add", "command", "c", "rename", "cb"]),
            vec![(4, AppendedArity::Exactly(3))],
        );
        // Execution operations select exact callback contracts.
        assert_eq!(
            reg.command_prefixes("trace", &["add", "execution", "c", "enter", "cb"]),
            vec![(4, AppendedArity::Exactly(2))],
        );
        assert_eq!(
            reg.command_prefixes("trace", &["add", "execution", "c", "leave", "cb"]),
            vec![(4, AppendedArity::Exactly(4))],
        );
        assert_eq!(
            reg.command_prefixes("trace", &["add", "execution", "c", "enter leave", "cb"]),
            vec![(
                4,
                AppendedArity::OneOf(crate::AppendedAritySet::from_sorted_unique(&[2, 4]))
            )],
        );
        // Deprecated `trace variable v ops cb` → cb at index 3, 3 appended args.
        assert_eq!(
            reg.command_prefixes("trace", &["variable", "v", "write", "cb"]),
            vec![(3, AppendedArity::Exactly(3))],
        );
        // `interp alias {} a {} target x` (create form) → target at index 3,
        // variadic. The 2-arg query form has no target.
        assert_eq!(
            reg.command_prefixes("interp", &["alias", "{}", "a", "{}", "target"]),
            vec![(4, AppendedArity::Unknown)],
        );
        assert!(
            reg.command_prefixes("interp", &["alias", "{}", "a"])
                .is_empty(),
            "the interp-alias query form has no command prefix",
        );
        // tcllib: `struct::list filter seq cb` (1), `map seq cb` (1),
        // `fold seq init cb` (2).
        assert_eq!(
            reg.command_prefixes("struct::list", &["filter", "$s", "cb"]),
            vec![(2, AppendedArity::Exactly(1))],
        );
        assert_eq!(
            reg.command_prefixes("struct::list", &["fold", "$s", "0", "cb"]),
            vec![(3, AppendedArity::Exactly(2))],
        );
    }

    #[test]
    fn command_prefixes_cover_deferred_core_commands() {
        // Version-gated / resolver-driven callback tails wired in the deferred
        // pass. Ground-truthed vs tclsh 9.0 (all are 8.7/9.0-era surfaces).
        let reg = CommandRegistry::build_default();

        // `regsub -command re str cmdPrefix ?var?` (TIP 463, 9.0): with
        // `-command` the subSpec slot is a prefix called per match with the
        // whole match + capture groups appended (variadic ⇒ AtLeast(1)). The
        // slot index tracks the leading-switch shift.
        assert_eq!(
            reg.command_prefixes("regsub", &["-command", "re", "s", "cb"]),
            vec![(3, AppendedArity::AtLeast(1))],
        );
        assert_eq!(
            reg.command_prefixes("regsub", &["-all", "-command", "re", "s", "cb"]),
            vec![(4, AppendedArity::AtLeast(1))],
        );
        // `-c` is NOT an abbreviation of `-command`: regsub's switch table
        // resolves with Tcl_GetIndexFromObj(..., TCL_EXACT, ...), confirmed
        // live (tclsh 8.6.14: `regsub -c {a} aaa X y` -> `bad option "-c"`)
        // and against the real `Tcl_RegsubObjCmd` C source. Only the exact
        // spelling `-command` enables command-prefix mode.
        assert_eq!(
            reg.command_prefixes("regsub", &["-c", "re", "s", "cb"]),
            Vec::new(),
        );
        // Without `-command`, subSpec is a replacement template, not a prefix.
        assert!(
            reg.command_prefixes("regsub", &["re", "s", "template"])
                .is_empty(),
            "plain regsub subSpec is not a command prefix",
        );
        // `--` terminates switches, so a following `-command`-looking word is a
        // pattern, not the flag.
        assert!(
            reg.command_prefixes("regsub", &["--", "-command", "s", "template"])
                .is_empty(),
            "`--` disables -command detection",
        );

        // `namespace unknown handler` → handler(cmd ?arg...?) = AtLeast(1). The
        // zero-arg query form carries no prefix.
        assert_eq!(
            reg.command_prefixes("namespace", &["unknown", "handler"]),
            vec![(1, AppendedArity::AtLeast(1))],
        );
        assert!(
            reg.command_prefixes("namespace", &["unknown"]).is_empty(),
            "the namespace-unknown query form has no command prefix",
        );

        // `package unknown handler` → handler(name requirement ?requirement
        // ...?) = AtLeast(2): verified empirically on tclsh 8.6.14 that Tcl
        // always appends the package name *plus* at least one requirement
        // word, synthesising a "0-" placeholder when `package require` was
        // given none itself — never just the bare name, so AtLeast(2), not
        // AtLeast(1) (see the fuller note on `package_.rs`'s `unknown`
        // subcommand).
        assert_eq!(
            reg.command_prefixes("package", &["unknown", "handler"]),
            vec![(1, AppendedArity::AtLeast(2))],
        );
        assert!(
            reg.command_prefixes("package", &["unknown"]).is_empty(),
            "the package-unknown query form has no command prefix",
        );

        // `coroinject coroName command ?arg...?` — per the Tcl 9.0/9.1
        // coroutine(n) manpage, exactly two more words are appended when the
        // injected command runs: the name of the command that suspended the
        // coroutine (yield or yieldto) and its current resumption value.
        assert_eq!(
            reg.command_prefixes("coroinject", &["myCoro", "cb", "x"]),
            vec![(1, AppendedArity::Exactly(2))],
        );
        // `coroprobe coroName command ?arg...?` runs command immediately
        // inside the suspended coroutine (not deferred through a yield/
        // yieldto resumption), so no fixed extra-argument count is
        // documented -- it stays a reference-only prefix (Unknown).
        assert_eq!(
            reg.command_prefixes("coroprobe", &["myCoro", "cb"]),
            vec![(1, AppendedArity::Unknown)],
        );

        // `chan create mode cmdPrefix` / `chan push channelId cmdPrefix` — the
        // reflected-channel/transform handler is invoked as `cmdPrefix
        // subcommand handle ?args...?` ⇒ AtLeast(2).
        assert_eq!(
            reg.command_prefixes("chan", &["create", "rw", "handler"]),
            vec![(2, AppendedArity::AtLeast(2))],
        );
        assert_eq!(
            reg.command_prefixes("chan", &["push", "$ch", "xform"]),
            vec![(2, AppendedArity::AtLeast(2))],
        );
    }

    #[test]
    fn command_prefixes_cover_tcllib_callbacks() {
        // tcllib callback tails wired in the deferred pass.  Fixed arities are
        // ground-truthed against the package man pages; ambiguous/variadic tails
        // are Unknown (reference-only).
        let reg = CommandRegistry::build_default();

        // `struct::list split seq cmdprefix` — the twin of filter: cmdprefix(el).
        assert_eq!(
            reg.command_prefixes("struct::list", &["split", "$s", "cb"]),
            vec![(2, AppendedArity::Exactly(1))],
        );
        // `fileutil::find basedir filtercmd` — filtercmd(name).  Shorter forms
        // carry no prefix (idx≥argc dropped).
        assert_eq!(
            reg.command_prefixes("fileutil::find", &["/base", "flt"]),
            vec![(1, AppendedArity::Exactly(1))],
        );
        assert!(
            reg.command_prefixes("fileutil::find", &["/base"])
                .is_empty(),
            "the basedir-only fileutil::find form has no filter prefix",
        );

        // generator functional ops: reference-only (multi-value yield ⇒ Unknown).
        assert_eq!(
            reg.command_prefixes("generator", &["map", "fn", "$g"]),
            vec![(1, AppendedArity::Unknown)],
        );
        assert_eq!(
            reg.command_prefixes("generator", &["filter", "pred", "$g"]),
            vec![(1, AppendedArity::Unknown)],
        );

        // math::calculus func callbacks (man-page-pinned fixed arities).
        assert_eq!(
            reg.command_prefixes("math::calculus::integral", &["0", "1", "100", "f"]),
            vec![(3, AppendedArity::Exactly(1))],
        );
        assert_eq!(
            reg.command_prefixes("math::calculus::integral3D", &["xi", "yi", "zi", "f"]),
            vec![(3, AppendedArity::Exactly(3))],
        );
        // `newtonRaphson func deriv initval` — two prefixes, each f(x).
        assert_eq!(
            reg.command_prefixes("math::calculus::newtonRaphson", &["f", "d", "0.5"]),
            vec![
                (0, AppendedArity::Exactly(1)),
                (1, AppendedArity::Exactly(1)),
            ],
        );
        // `math::probopt::pso function bounds ?args?` — objective(coordVec).
        assert_eq!(
            reg.command_prefixes("math::probopt::pso", &["obj", "bnds", "-iter", "50"]),
            vec![(0, AppendedArity::Exactly(1))],
        );

        // log message-writer callbacks — `cmd level text` (Exactly(2)).
        assert_eq!(
            reg.command_prefixes("log::lvCmd", &["debug", "writer"]),
            vec![(1, AppendedArity::Exactly(2))],
        );
        assert_eq!(
            reg.command_prefixes("log::lvCmdForall", &["writer"]),
            vec![(0, AppendedArity::Exactly(2))],
        );
        // `uevent::bind tag event command` — command(tag event ?details?).
        assert_eq!(
            reg.command_prefixes("uevent::bind", &["tag", "ev", "cb"]),
            vec![(2, AppendedArity::AtLeast(2))],
        );

        // `hook bind subject hook observer binding` — the 4-word set form names
        // a command prefix (Unknown appended: the count is whatever the matching
        // `hook call` passes); the shorter query forms name none.
        assert_eq!(
            reg.command_prefixes("hook", &["bind", "sub", "hk", "obs", "cb"]),
            vec![(4, AppendedArity::Unknown)],
        );
        assert!(
            reg.command_prefixes("hook", &["bind", "sub", "hk", "obs"])
                .is_empty(),
            "the 3-arg `hook bind` query form has no callback prefix",
        );

        // `processman::onexit id cmd` — `cmd` is a deferred *script* (`eval $cmd`,
        // 0 appended), NOT a command prefix: it must declare none.
        assert!(
            reg.command_prefixes("processman::onexit", &["$pid", "cb"])
                .is_empty(),
            "processman::onexit cmd is a script body, not a command prefix",
        );
        assert_eq!(
            reg.get("processman::onexit")
                .and_then(|s| s.arg_roles.iter().find(|(i, _)| *i == 1))
                .map(|(_, r)| *r),
            Some(ArgRole::Body),
            "processman::onexit cmd (index 1) must carry the Body script role",
        );
    }

    #[test]
    fn command_prefixes_cover_option_value_callbacks() {
        // Option-value callbacks — the prefix is the value of a named `-flag`,
        // resolved through each command's `OptionSpec` array.  Arities are
        // ground-truthed against tcllib source (verified by re-running the exact
        // `uplevel`/`eval`/`{*}` idioms in tclsh — `uplevel`/`eval` re-split a
        // `[list …]` word, `[list {*}pfx {*}args]` keeps one word per element).
        let reg = CommandRegistry::build_default();

        // `mime::getbody token -command cb` — async body callback.  uplevel
        // re-splits `[list end]` → 1 word, `[list data $c]` → 2 ⇒ AtLeast(1).
        assert_eq!(
            reg.command_prefixes("mime::getbody", &["tok", "-command", "cb"]),
            vec![(2, AppendedArity::AtLeast(1))],
        );
        // `-decode` before `-command` is a value-less flag: the scan skips it
        // without swallowing the callback.
        assert_eq!(
            reg.command_prefixes("mime::getbody", &["tok", "-decode", "-command", "cb"]),
            vec![(3, AppendedArity::AtLeast(1))],
        );

        // `smtp::sendmessage tok -tlspolicy pol` — `eval $pol [list $code]
        // [list $diag]` ⇒ Exactly(2).  A preceding value-taking option's value
        // is skipped, not mistaken for the callback.
        assert_eq!(
            reg.command_prefixes("smtp::sendmessage", &["tok", "-tlspolicy", "pol"]),
            vec![(2, AppendedArity::Exactly(2))],
        );
        assert_eq!(
            reg.command_prefixes(
                "smtp::sendmessage",
                &["tok", "-servers", "mail.x", "-tlspolicy", "pol"],
            ),
            vec![(4, AppendedArity::Exactly(2))],
        );

        // `comm::comm send -command cb id cmd` — 7 `-key value` reply pairs ⇒
        // Exactly(14).  Subcommand-relative option scan offsets by 1.
        assert_eq!(
            reg.command_prefixes("comm::comm", &["send", "-command", "cb", "id", "cmd"]),
            vec![(2, AppendedArity::Exactly(14))],
        );

        // `bibtex::parse` — every callback prepends the parser token, then its
        // own payload words: `-command`/`-*command` ⇒ Exactly(2), except
        // `-recordcommand` ⇒ Exactly(4) (token type key recdata).
        assert_eq!(
            reg.command_prefixes("bibtex::parse", &["-recordcommand", "cb", "text"]),
            vec![(1, AppendedArity::Exactly(4))],
        );
        assert_eq!(
            reg.command_prefixes("bibtex::parse", &["-command", "cb"]),
            vec![(1, AppendedArity::Exactly(2))],
        );
        assert_eq!(
            reg.command_prefixes("bibtex::parse", &["-progresscommand", "cb"]),
            vec![(1, AppendedArity::Exactly(2))],
        );

        // `tcl::chan::halfpipe` — clean `[list {*}pfx {*}args]` idiom: write
        // appends (chan bytes) ⇒ 2, empty/close append (chan) ⇒ 1.  No
        // `-read-command` exists.
        assert_eq!(
            reg.command_prefixes("tcl::chan::halfpipe", &["-write-command", "cb"]),
            vec![(1, AppendedArity::Exactly(2))],
        );
        assert_eq!(
            reg.command_prefixes("tcl::chan::halfpipe", &["-close-command", "cb"]),
            vec![(1, AppendedArity::Exactly(1))],
        );
        assert_eq!(
            reg.command_prefixes("tcl::chan::halfpipe", &["-empty-command", "cb"]),
            vec![(1, AppendedArity::Exactly(1))],
        );
        assert!(
            reg.command_prefixes("tcl::chan::halfpipe", &["-read-command", "cb"])
                .is_empty(),
            "halfpipe has no -read-command",
        );
    }

    #[test]
    fn instance_method_command_prefixes_cover_struct_graph_and_tree() {
        // Object-instance method callbacks — the prefix is on a method of a
        // created object command (`$g walk … -command cb`, `$t walkproc … cb`),
        // resolved through the class's ObjectClassSpec.  Indices are relative to
        // the words after the method name.
        let reg = CommandRegistry::build_default();

        // struct::graph `walk node … -command cb` — option-value prefix,
        // Exactly(3) (action graphName node).
        assert_eq!(
            reg.instance_method_command_prefixes(
                "struct::graph",
                "walk",
                &["root", "-order", "pre", "-command", "cb"],
            ),
            vec![(4, AppendedArity::Exactly(3))],
        );
        assert_eq!(
            reg.instance_method_command_prefixes(
                "struct::graph",
                "walk",
                &["root", "-command", "cb"]
            ),
            vec![(2, AppendedArity::Exactly(3))],
        );

        // struct::tree `walkproc node … cmdprefix` — trailing positional prefix
        // (resolver), Exactly(3) (tree node action).  The prefix is the final
        // word regardless of intervening `-order`/`-type` options.
        assert_eq!(
            reg.instance_method_command_prefixes("struct::tree", "walkproc", &["root", "cb"]),
            vec![(1, AppendedArity::Exactly(3))],
        );
        assert_eq!(
            reg.instance_method_command_prefixes(
                "struct::tree",
                "walkproc",
                &["root", "-type", "dfs", "cb"],
            ),
            vec![(3, AppendedArity::Exactly(3))],
        );
        // `walkproc node` with no prefix word yet names none.
        assert!(
            reg.instance_method_command_prefixes("struct::tree", "walkproc", &["root"])
                .is_empty(),
            "a walkproc with only the node names no prefix",
        );
        // An unmodelled method / class resolves to nothing.
        assert!(
            reg.instance_method_command_prefixes("struct::graph", "get", &["x"])
                .is_empty(),
            "an unmodelled instance method has no command prefix",
        );
    }

    #[test]
    fn tk_command_options_classified_prefix_vs_script() {
        // The Tk `script()→command_prefix` conversion (separate commit, highest
        // risk).  Locks in the classification: appended-arg callbacks are
        // prefixes (references / call-graph / W123 / arity); verbatim scripts and
        // percent-substitution callbacks stay `script()` (Body, not recorded as a
        // reference).  Ground truth: Tk 8.6/9.0 man pages.
        let reg = CommandRegistry::build_default();

        // PREFIXES — scroll callbacks append `first last` (2).
        for widget in ["listbox", "text", "canvas", "entry", "spinbox"] {
            assert_eq!(
                reg.command_prefixes(widget, &[".w", "-xscrollcommand", "cb"]),
                vec![(2, AppendedArity::Exactly(2))],
                "{widget} -xscrollcommand must be a prefix appending 2",
            );
        }
        // scale / ttk::scale append the new value (1).
        assert_eq!(
            reg.command_prefixes("scale", &[".s", "-command", "cb"]),
            vec![(2, AppendedArity::Exactly(1))],
        );
        assert_eq!(
            reg.command_prefixes("ttk::scale", &[".s", "-command", "cb"]),
            vec![(2, AppendedArity::Exactly(1))],
        );
        // scrollbar -command: `moveto frac` (2) or `scroll n units` (3).
        assert_eq!(
            reg.command_prefixes("scrollbar", &[".sb", "-command", "cb"]),
            vec![(
                2,
                AppendedArity::OneOf(crate::AppendedAritySet::from_sorted_unique(&[2, 3]))
            )],
        );
        // menu -tearoffcommand appends the parent + torn-off menu paths (2).
        assert_eq!(
            reg.command_prefixes("menu", &[".m", "-tearoffcommand", "cb"]),
            vec![(2, AppendedArity::Exactly(2))],
        );

        // NOT prefixes — verbatim action scripts and percent-substitution
        // callbacks are `script()`, never recorded as a command reference.
        for (widget, opt) in [
            ("button", "-command"),
            ("checkbutton", "-command"),
            ("radiobutton", "-command"),
            ("menu", "-command"),
            ("menu", "-postcommand"),
            ("ttk::combobox", "-postcommand"),
            ("spinbox", "-command"), // percent-substitution (%W %s %d)
            ("spinbox", "-validatecommand"), // percent-substitution
            ("entry", "-validatecommand"), // percent-substitution
            ("entry", "-invalidcommand"), // percent-substitution
        ] {
            assert!(
                reg.command_prefixes(widget, &[".w", opt, "cb"]).is_empty(),
                "{widget} {opt} must stay a script, not a command prefix",
            );
        }
    }

    #[test]
    fn commands_naming_a_cmdprefix_declare_a_command_prefix() {
        // Drift guard: any command whose synopsis literally names a `cmdprefix`
        // argument must declare a `CommandPrefix` (static table, resolver, or
        // command-prefix option) so callbacks light up references / call-graph /
        // W123 / arity.  The allowlist holds genuinely-deferred callbacks that
        // still need modelling; it is empty now that the option-value
        // callbacks (`mime::getbody -command`, …) carry `OptionSpec` arrays.
        const DEFERRED_OPTION_PREFIX: &[&str] = &[];
        let reg = CommandRegistry::build_default();
        let mut gaps = Vec::new();
        for name in reg.command_names() {
            if DEFERRED_OPTION_PREFIX.contains(&name) {
                continue;
            }
            let spec = reg.get(name).expect("registered");
            let mut synopses: Vec<&str> = Vec::new();
            if let Some(h) = &spec.hover {
                synopses.extend(h.synopsis.iter().copied());
            }
            synopses.extend(spec.forms.iter().map(|f| f.synopsis));
            synopses.extend(spec.subcommands.iter().map(|s| s.synopsis));
            for syn in synopses {
                if !syn.to_ascii_lowercase().contains("cmdprefix") {
                    continue;
                }
                // Probe with the synopsis words after the command name; a
                // declared prefix yields a non-empty result.  Strip the `?…?`
                // optionality markers so an option-value prefix
                // (`?-command cmdprefix?`) presents its bare `-command` /
                // `cmdprefix` words to the option scanner.
                let args: Vec<&str> = syn
                    .split_whitespace()
                    .skip(1)
                    .map(|w| w.trim_matches('?'))
                    .collect();
                if reg.command_prefixes(name, &args).is_empty() {
                    gaps.push(format!("{name}: {syn}"));
                }
            }
        }
        gaps.sort();
        assert!(
            gaps.is_empty(),
            "commands whose synopsis names a cmdprefix but declare no CommandPrefix:\n{}",
            gaps.join("\n"),
        );
    }

    #[test]
    fn arg_indices_for_role_command_prefix_matches_command_prefixes() {
        // The delegation invariant: `arg_indices_for_role(CommandPrefix)` is
        // exactly the positions `command_prefixes` reports — so highlighting,
        // param-traits, and the call-reference extractor never drift.
        let reg = CommandRegistry::build_default();
        for (name, args) in [
            ("lsort", &["-command", "cmp", "{a b}"][..]),
            ("socket", &["-server", "accept", "9000"][..]),
            ("tcltest::customMatch", &["exact", "cmp"][..]),
            ("selection", &["handle", ".w", "getData"][..]),
            ("regsub", &["-command", "re", "s", "cb"][..]),
            ("namespace", &["unknown", "handler"][..]),
            ("package", &["unknown", "handler"][..]),
            ("coroinject", &["myCoro", "cb", "x"][..]),
        ] {
            let via_role = reg.arg_indices_for_role(name, args, ArgRole::CommandPrefix);
            let via_prefixes: Vec<usize> = reg
                .command_prefixes(name, args)
                .into_iter()
                .map(|(i, _)| i)
                .collect();
            assert_eq!(via_role, via_prefixes, "delegation mismatch for {name}");
        }
    }

    #[test]
    fn namespace_name_option_carries_name_role() {
        // `interp invokehidden path -namespace ns cmd` — the `-namespace` value is a
        // symbolic (namespace) name: captured declaratively for a
        // Name query, never for Body/VarWrite (not recursed, not a var def).
        let reg = CommandRegistry::build_default();
        let args = ["invokehidden", "child", "-namespace", "ns", "cmd"];
        assert_eq!(
            reg.arg_indices_for_role("interp", &args, ArgRole::Name),
            vec![3],
            "the -namespace value should carry the Name role",
        );
        assert!(
            reg.arg_indices_for_role("interp", &args, ArgRole::Body)
                .is_empty()
                && reg
                    .arg_indices_for_role("interp", &args, ArgRole::VarWrite)
                    .is_empty(),
            "a name value must not be recursed or treated as a variable",
        );
    }

    #[test]
    fn bind_script_form_recurses_only_the_trailing_script() {
        // `bind $w <KeyPress> {…}` binds a script: the third
        // argument is a deferred event-handler body and must be recursed for
        // highlighting.  The `bind tag` / `bind tag sequence` query forms carry
        // no script and must not surface a Body.
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.arg_indices_for_role("bind", &["$w", "<KeyPress>", "{…}"], ArgRole::Body),
            vec![2],
            "the trailing script of the three-argument form is a body",
        );
        // The `+script` append form is still the trailing argument.
        assert_eq!(
            reg.arg_indices_for_role("bind", &[".b", "<Enter>", "+{puts hi}"], ArgRole::Body),
            vec![2],
        );
        assert!(
            reg.arg_indices_for_role("bind", &["$w"], ArgRole::Body)
                .is_empty(),
            "the single-tag query form has no script",
        );
        assert!(
            reg.arg_indices_for_role("bind", &["$w", "<KeyPress>"], ArgRole::Body)
                .is_empty(),
            "the tag+sequence query form has no script",
        );
    }

    #[test]
    fn wm_protocol_handler_is_a_body() {
        // `wm protocol . WM_DELETE_WINDOW {script}` registers a deferred
        // handler script as its third argument; the query forms carry none.
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.arg_indices_for_role(
                "wm",
                &["protocol", ".", "WM_DELETE_WINDOW", "{exit}"],
                ArgRole::Body,
            ),
            // subcommand path offsets by +1 for the `protocol` word.
            vec![3],
            "the wm protocol handler command is a script body",
        );
        assert!(
            reg.arg_indices_for_role("wm", &["protocol", ".", "WM_DELETE_WINDOW"], ArgRole::Body)
                .is_empty(),
            "the two-argument `wm protocol window name` query form has no script",
        );
        assert!(
            reg.arg_indices_for_role("wm", &["protocol", "."], ArgRole::Body)
                .is_empty(),
            "the one-argument `wm protocol window` query form has no script",
        );
    }

    #[test]
    fn canvas_bind_subcommand_script_is_a_body() {
        let reg = callback_metadata_registry();
        for (arguments, expected) in [
            (&["bind", "item", "<Button>", "{p}"][..], vec![3]),
            (&["bind", "item", "<Button>"][..], Vec::new()),
        ] {
            let invocation = reg
                .resolve_instance_invocation("canvas", ".canvas", arguments, None)
                .expect("provided canvas instance method");
            let roles = invocation
                .argument_roles()
                .0
                .into_iter()
                .filter_map(|(at, role)| {
                    (role == ArgRole::Body)
                        .then_some(usize::from(at) + invocation.semantics.argument_offset)
                })
                .collect::<Vec<_>>();
            assert_eq!(roles, expected, "{arguments:?}");
        }
    }

    #[test]
    fn ttk_widgets_require_tk_8_5() {
        // ttk (themed Tk) widgets were introduced in Tk 8.5, so they must be
        // gated out when only an older Tk is guaranteed by `package require`.
        let reg = CommandRegistry::build_default();
        for name in [
            "ttk::button",
            "ttk::treeview",
            "ttk::notebook",
            "ttk::style",
        ] {
            let spec = reg.get(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(
                !spec.available_for_version(Some("8.4")),
                "{name} must not be available under Tk 8.4",
            );
            assert!(
                spec.available_for_version(Some("8.5")),
                "{name} must be available under Tk 8.5",
            );
            assert!(
                spec.available_for_version(None),
                "{name} must be permissive when no Tk version is pinned",
            );
        }
    }

    #[test]
    fn text_tag_bind_script_is_a_body() {
        // `pathName tag bind tagName sequence script` binds a deferred
        // event-handler script as its trailing word.
        let reg = crate::model::semantic::SemanticContext::for_environment("tk").commands();
        let roles = |arguments: &[&str]| {
            let invocation = reg
                .resolve_instance_invocation("text", ".t", arguments, reg.own_surface_query())
                .expect("provided text instance method");
            invocation
                .argument_roles()
                .0
                .into_iter()
                .filter_map(|(index, role)| {
                    (role == ArgRole::Body)
                        .then_some(usize::from(index) + invocation.semantics.argument_offset)
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(roles(&["tag", "bind", "sel", "<Key>", "{p}"]), vec![4]);
        assert!(roles(&["tag", "add", "sel", "1.0", "end"]).is_empty());
        let unprovided = CommandRegistry::build_default()
            .project_for_profile(DialectProfile::find("tcl8.6").unwrap());
        assert!(
            unprovided
                .resolve_instance_invocation(
                    "text",
                    ".t",
                    &["tag", "bind", "sel", "<Key>", "{p}"],
                    unprovided.own_surface_query()
                )
                .is_none()
        );
    }

    #[test]
    fn selection_handle_command_is_a_command_prefix() {
        // `selection handle window command` — the trailing command is a
        // prefix Tk appends offset/maxChars to, not a recursed script body.
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.arg_indices_for_role(
                "selection",
                &["handle", ".w", "getData"],
                ArgRole::CommandPrefix
            ),
            vec![2],
            "the selection handle command prefix is captured",
        );
        assert!(
            reg.arg_indices_for_role("selection", &["handle", ".w", "getData"], ArgRole::Body)
                .is_empty(),
            "a command prefix is not recursed as a script body",
        );
        // With leading option/value pairs the command is still the last arg.
        assert_eq!(
            reg.arg_indices_for_role(
                "selection",
                &["handle", "-format", "STRING", ".w", "getData"],
                ArgRole::CommandPrefix,
            ),
            vec![4],
        );
    }

    #[test]
    fn writes_first_arg_variable_membership() {
        let reg = CommandRegistry::build_default();
        // TP: the five first-arg writers.
        for cmd in ["set", "append", "lappend", "incr", "lset"] {
            assert!(reg.writes_first_arg_variable(cmd), "{cmd} writes arg 0");
        }
        // FP guards: `unset` destroys (not a write); value-taking and
        // unknown commands are out.
        for cmd in ["unset", "puts", "llength", "foreach", "nosuchcmd"] {
            assert!(!reg.writes_first_arg_variable(cmd), "{cmd} must not match");
        }
    }

    #[test]
    fn rmw_first_arg_variable_membership() {
        let reg = CommandRegistry::build_default();
        // TP: read-modify-write commands fold the current value in.
        for cmd in ["append", "lappend", "incr", "lset"] {
            assert!(reg.rmw_first_arg_variable(cmd), "{cmd} is RMW");
        }
        // FP guards: a whole-value `set` is rename-safe; `unset` destroys.
        for cmd in ["set", "unset", "puts", "nosuchcmd"] {
            assert!(!reg.rmw_first_arg_variable(cmd), "{cmd} must not match");
        }
    }

    #[test]
    fn commands_with_trait_query() {
        let reg = CommandRegistry::build_default();
        let control_flow = reg.commands_with_trait(Traits::CONTROL_FLOW);
        assert!(control_flow.contains(&"for"));
        assert!(control_flow.contains(&"if"));
        assert!(control_flow.contains(&"while"));
        assert!(!control_flow.contains(&"puts"));
    }

    #[test]
    fn native_root_factory_descriptor_uses_actual_axes() {
        let older_profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
        let mut registry = CommandRegistry::build_default();
        registry.set_profile(older_profile);
        for (version, admitted) in [
            (tcl_dialect::TclVersion::V8_4, false),
            (tcl_dialect::TclVersion::V8_6, true),
            (tcl_dialect::TclVersion::V9_0, true),
            (tcl_dialect::TclVersion::V9_1, true),
        ] {
            assert_eq!(
                registry
                    .native_default_construction_grammar(
                        "oo::class",
                        crate::InvocationDialect::for_version(version),
                        tcl_dialect::model::InvocationRealm::RuleLoader,
                    )
                    .is_some(),
                admitted,
                "the editor's older profile cannot replace the actual factory release",
            );
        }
        assert!(registry.default_construction_grammar("oo::class").is_none());
        let physically_projected =
            CommandRegistry::build_default().project_for_profile(older_profile);
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            assert!(
                physically_projected
                    .native_default_construction_grammar(
                        "oo::class",
                        crate::InvocationDialect::for_version(version),
                        tcl_dialect::model::InvocationRealm::RuleLoader,
                    )
                    .is_none(),
                "actual axes cannot restore metadata removed by physical projection"
            );
        }
        let jim = crate::InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        assert!(
            registry
                .native_default_construction_grammar(
                    "oo::class",
                    jim,
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .is_none()
        );
    }

    fn assert_effective_semantics_match_uncached_queries(registry: &CommandRegistry) {
        let index = registry.effective_semantics();
        let mut expected_bindings: BTreeSet<String> = registry
            .command_names()
            .map(tcl_syntax::naming::normalise_qualified_name)
            .collect();
        // Authored private implementation slots are bindings even when they
        // have no public command catalogue row.
        if let Some(grammar) = registry.default_construction_grammar("oo::class") {
            for member in grammar.members {
                for receiver in [
                    crate::definer::DefinitionReceiver::Instance,
                    crate::definer::DefinitionReceiver::Class,
                ] {
                    if let Some(lookup) = grammar.definition_member_lookup_for_receiver(
                        member,
                        receiver,
                        registry.own_surface_query(),
                    ) {
                        expected_bindings.insert(lookup.implementation);
                    }
                }
            }
        }
        if let Some(profile) = registry.profile() {
            expected_bindings.extend(
                registry
                    .stock_native_implementation_slots(crate::InvocationDialect::of_profile(
                        profile,
                    ))
                    .into_iter()
                    .map(|lookup| lookup.slot.to_owned()),
            );
        }
        if let Some(profile) = registry.profile()
            && let Some(support) = registry
                .native_class_factory_recipe(
                    "oo::configurable",
                    crate::InvocationDialect::of_profile(profile),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .and_then(crate::native_tcloo_bootstrap::NativeClassFactoryRecipe::support)
        {
            expected_bindings.extend(
                support
                    .binding_names()
                    .iter()
                    .map(|name| (*name).to_owned()),
            );
        }
        let expected_handlers: BTreeSet<String> = registry
            .commands_with_trait(Traits::UNRESOLVED_COMMAND_HANDLER)
            .into_iter()
            .map(tcl_syntax::naming::normalise_qualified_name)
            .collect();
        assert_eq!(index.binding_names(), &expected_bindings);
        assert_eq!(index.unresolved_command_handlers(), &expected_handlers);

        for name in registry.command_names() {
            let expected = match registry.own_surface_query() {
                Some(query) => registry.get_for_surface(name, Some(query)),
                None => registry.get(name),
            };
            let actual = index.command(name);
            assert_eq!(
                actual.map(EffectiveCommandSemantics::traits),
                expected.map(|spec| spec.traits)
            );
            assert_eq!(
                actual.and_then(EffectiveCommandSemantics::lowering_hook),
                expected.and_then(|spec| spec.lowering_hook),
                "lowering hook mismatch for {name}"
            );
        }
    }

    #[test]
    fn effective_semantics_cache_matches_default_and_exact_release_queries() {
        assert_effective_semantics_match_uncached_queries(&CommandRegistry::build_default());
        for dialect in ["tcl8.4", "tcl9.0"] {
            assert_effective_semantics_match_uncached_queries(
                crate::model::ingress::static_context_for(dialect).commands(),
            );
        }
    }

    #[test]
    fn snapshot_and_ingress_share_the_lazy_semantic_index() {
        let registry = CommandRegistry::build_default();
        let snapshot = registry.snapshot();
        assert!(Arc::ptr_eq(
            &registry.effective_semantics(),
            &snapshot.registry().effective_semantics(),
        ));
    }

    #[test]
    fn effective_semantics_cache_is_reused_and_invalidated_by_overlay() {
        let mut registry = CommandRegistry::build_default();
        let before = registry.effective_semantics();
        assert!(Arc::ptr_eq(&before, &registry.effective_semantics()));

        // `insert` is the same authored-overlay seam used by the SpecTcl
        // installer after it has specialised a pack command.
        let mut overlay = registry.get("puts").expect("puts spec").clone();
        overlay.traits |= Traits::UNRESOLVED_COMMAND_HANDLER | Traits::CATCHABLE_THROW;
        registry.insert(overlay);

        let after = registry.effective_semantics();
        assert!(!Arc::ptr_eq(&before, &after));
        assert!(after.unresolved_command_handlers().contains("::puts"));
        assert!(
            after
                .command("puts")
                .is_some_and(|facts| facts.has_traits(Traits::CATCHABLE_THROW))
        );
        assert_effective_semantics_match_uncached_queries(&registry);
    }

    #[test]
    fn effective_semantics_cache_is_invalidated_by_profile_stamp() {
        let mut registry = CommandRegistry::build_default();
        let before = registry.effective_semantics();
        let profile = tcl_dialect::DialectProfile::find("tcl8.4").expect("tcl8.4 profile");
        registry.set_profile(profile);

        let after = registry.effective_semantics();
        assert!(!Arc::ptr_eq(&before, &after));
        assert_eq!(after.command("throw"), None);
        assert_effective_semantics_match_uncached_queries(&registry);
    }

    #[test]
    fn byte_compiled_covers_the_core_builtins() {
        let reg = CommandRegistry::build_default();
        // Every registered command the minifier must never alias.
        // The former built-in skip list (minus the
        // non-command keywords `else` / `elseif` and the unregistered
        // `pwd`).
        let expected = [
            "set",
            "unset",
            "proc",
            "if",
            "while",
            "for",
            "foreach",
            "switch",
            "return",
            "break",
            "continue",
            "expr",
            "catch",
            "try",
            "throw",
            "package",
            "namespace",
            "upvar",
            "uplevel",
            "variable",
            "global",
            "append",
            "lappend",
            "incr",
            "info",
            "string",
            "list",
            "llength",
            "lindex",
            "lrange",
            "lsort",
            "lsearch",
            "lreplace",
            "linsert",
            "dict",
            "array",
            "regexp",
            "regsub",
            "scan",
            "format",
            "open",
            "close",
            "read",
            "gets",
            "eof",
            "flush",
            "seek",
            "tell",
            "fconfigure",
            "fcopy",
            "fileevent",
            "socket",
            "after",
            "update",
            "vwait",
            "rename",
            "source",
            "eval",
            "apply",
            "tailcall",
            "error",
            "cd",
            "file",
            "glob",
            "clock",
            "binary",
            "encoding",
            "interp",
            "load",
            "exit",
            "pid",
            "exec",
            "chan",
            "puts",
        ];
        for name in expected {
            assert!(
                reg.is_byte_compiled(name),
                "{name} should carry Traits::BYTE_COMPILED"
            );
        }
        // A user-proc-like name and a command outside the curated
        // skip set must not carry the trait.
        assert!(!reg.is_byte_compiled("my_helper_proc"));
        assert!(!reg.is_byte_compiled("split"));
    }

    #[test]
    fn not_proc_factory_covers_registered_skip_heads() {
        let reg = CommandRegistry::build_default();
        // Registered heads from the former `_FACTORY_SKIP_HEADS` list
        // (the four non-command heads method / classmethod /
        // itcl::class / ::itcl::class are handled by the scanner's
        // residual set, not the registry).
        let expected = [
            "proc",
            "namespace",
            "if",
            "switch",
            "while",
            "for",
            "foreach",
            "try",
            "catch",
            "eval",
            "apply",
            "expr",
            "uplevel",
            "upvar",
            "variable",
            "set",
            "lappend",
            "dict",
            "array",
            "string",
            "list",
            "lindex",
            "package",
            "source",
            "interp",
            "oo::class",
            "oo::define",
            "oo::objdefine",
        ];
        for name in expected {
            assert!(
                reg.is_not_proc_factory(name),
                "{name} should carry Traits::NOT_PROC_FACTORY"
            );
        }
        // A real factory-wrapper head must not be skipped.
        assert!(!reg.is_not_proc_factory("my_factory"));
    }

    #[test]
    fn frameless_runtime_covers_the_audited_allow_list() {
        let reg = CommandRegistry::build_default();
        let got: std::collections::HashSet<&str> = reg
            .commands_with_trait(Traits::FRAMELESS_RUNTIME)
            .into_iter()
            .collect();
        let expected: std::collections::HashSet<&str> = [
            "list",
            "lindex",
            "lrange",
            "linsert",
            "llength",
            "lsort",
            "lsearch",
            "lappend",
            "lreverse",
            "lreplace",
            // Tcl 9.0+: shares `lreplace`'s core (see
            // `tcl-vm/src/cmd_list.rs::cmd_ledit`); a flat single-variable
            // read-modify-write with no eval fallback and no sublist-index
            // descent (unlike `lset`, which deliberately stays off this
            // list) — see `commands/tcl/ledit.rs`.
            "ledit",
            "lrepeat",
            "lassign",
            // Tcl 9.0+: a flat native dispatch over `tcl-cmd-core::lseq`
            // (`tcl-vm/src/cmd_lseq.rs::cmd_lseq`) with no script-body eval
            // fallback of its own — its only recursive edge is evaluating
            // one argument word as an *expression* via `Vm::eval_expr`,
            // the same evaluator `expr` (also on this list) runs on, not a
            // general eval-a-script fallback — see `commands/tcl/lseq.rs`.
            "lseq",
            "concat",
            "split",
            "join",
            "string",
            "format",
            "expr",
            "global",
            "variable",
            "upvar",
            "namespace",
            "set",
            "incr",
            "append",
            "unset",
            "puts",
            "return",
            "error",
            "continue",
            "break",
            // `throw type message` (Tcl 8.6+) — `cmd_throw`
            // (`tcl-vm/src/cmd_try.rs`) takes its two already-substituted
            // arguments, validates `type` as a non-empty Tcl list, and
            // builds its return-options dict directly with no eval
            // fallback of any kind (it never touches `vm`), structurally
            // identical to `cmd_error` in this respect. See
            // `commands/tcl/throw_.rs`.
            "throw",
            // `::tcl::unsupported::corotype coroName` (and its
            // namespace-relative `tcl::unsupported::corotype` spelling) — a
            // flat lookup into the coroutine table
            // (`tcl-vm/src/cmd_coro.rs::cmd_corotype`): no eval fallback, no
            // `Frame` of its own. Both spellings share the same `make_spec`
            // and so the same traits. See
            // `commands/tcl/tcl_unsupported_corotype.rs`.
            "::tcl::unsupported::corotype",
            "tcl::unsupported::corotype",
        ]
        .into_iter()
        .collect();
        assert_eq!(
            got, expected,
            "FRAMELESS_RUNTIME stamps drifted from the audited allow-list"
        );
    }

    #[test]
    fn dialect_filter() {
        let reg = CommandRegistry::build_default();
        let spec = reg.get_for_surface("dict", Some(SurfaceQuery::core(Family::Tcl, "8.6")));
        assert!(spec.is_some());
        // dict is tcl8.5+ so should NOT be available in 8.4
        let spec84 = reg.get_for_surface("dict", Some(SurfaceQuery::core(Family::Tcl, "8.4")));
        assert!(spec84.is_none());
    }

    #[test]
    fn subcommand_arg_roles() {
        let reg = CommandRegistry::build_default();
        let bodies =
            reg.arg_indices_for_role("dict", &["for", "{k v}", "$d", "body"], ArgRole::Body);
        // dict for {varList} dictExpr body → body is at index 3 (subcmd=0, args 1-based +1)
        assert!(bodies.contains(&3));
    }

    #[test]
    fn variable_write_commands() {
        let reg = CommandRegistry::build_default();
        let set_vars = reg.arg_indices_for_role("set", &["x", "1"], ArgRole::VarWrite);
        assert_eq!(set_vars, vec![0]);
    }

    #[test]
    fn role_queries_share_selected_set_forms_with_owned_facts() {
        let registry = CommandRegistry::build_default()
            .project_for_profile(DialectProfile::find("tcl8.6").unwrap());
        for (args, role, expected) in [
            (vec!["x"], ArgRole::VarRead, vec![0]),
            (vec!["x"], ArgRole::VarWrite, vec![]),
            (vec!["x", "value"], ArgRole::VarRead, vec![]),
            (vec!["x", "value"], ArgRole::VarWrite, vec![0]),
        ] {
            let facts = registry
                .resolve_invocation("set", &args, registry.own_surface_query())
                .expect("native set")
                .facts();
            let positions: Vec<_> = facts
                .arg_roles
                .iter()
                .filter(|(_, found)| *found == role)
                .map(|(index, _)| facts.argument_offset + usize::from(*index))
                .collect();
            assert_eq!(positions, expected);
            assert_eq!(
                registry.arg_indices_for_role("::set", &args, role),
                expected
            );
        }
    }

    #[test]
    fn source_role_queries_select_forms_without_requiring_operand_values() {
        let registry = CommandRegistry::build_default()
            .project_for_profile(DialectProfile::find("tcl8.6").unwrap());
        let read = [InvocationWord::Dynamic];
        let write = [InvocationWord::Dynamic, InvocationWord::Dynamic];
        assert_eq!(
            registry.arg_indices_for_role_words(
                "set",
                InvocationArguments::structured(&read),
                ArgRole::VarRead
            ),
            Some(vec![0])
        );
        assert_eq!(
            registry.arg_indices_for_role_words(
                "set",
                InvocationArguments::structured(&write),
                ArgRole::VarWrite
            ),
            Some(vec![0])
        );
        let expanded = [InvocationWord::Expanded];
        assert_eq!(
            registry.arg_indices_for_role_words(
                "set",
                InvocationArguments::structured(&expanded),
                ArgRole::VarWrite
            ),
            None
        );
    }

    #[test]
    fn source_procedure_roles_do_not_require_native_definition_acceptance() {
        // naming.source.authored-registry-role-projection
        // docs/design/analysis/name-resolution-proofs/authored-registry-role-projection.md
        let registry = CommandRegistry::build_default();
        let values = [
            InvocationWord::Dynamic,
            InvocationWord::KnownBytes(b"\xff"),
            InvocationWord::Dynamic,
        ];
        let arguments = InvocationArguments::structured(&values);
        let selected = registry
            .resolve_structured_invocation(
                InvocationWords::from_arguments(InvocationWord::Literal("proc"), arguments),
                None,
            )
            .resolved()
            .unwrap();
        assert!(!selected.facts().arg_roles_complete);
        assert!(selected.facts().arg_roles.is_empty());
        assert_eq!(
            registry.arg_role_assignments_words(
                "proc",
                arguments,
                &[ArgRole::Name, ArgRole::ParamList, ArgRole::Body],
            ),
            Some(vec![
                (0, ArgRole::Name),
                (1, ArgRole::ParamList),
                (2, ArgRole::Body)
            ])
        );
        assert_eq!(
            registry.arg_indices_for_role("proc", &["p", "a b", "return"], ArgRole::Body,),
            vec![2]
        );
        let expanded = [InvocationWord::Expanded];
        assert_eq!(
            registry.arg_role_assignments_words(
                "proc",
                InvocationArguments::structured(&expanded),
                &[ArgRole::Body],
            ),
            None
        );
        let opaque = [
            InvocationWord::Dynamic,
            InvocationWord::Opaque,
            InvocationWord::Dynamic,
        ];
        assert_eq!(
            registry.arg_role_assignments_words(
                "proc",
                InvocationArguments::structured(&opaque),
                &[ArgRole::Body],
            ),
            None
        );
    }

    #[test]
    fn source_role_queries_retain_explicit_optional_frame_grammar() {
        let registry = CommandRegistry::build_default();
        let args = [
            InvocationWord::Literal("+0"),
            InvocationWord::Literal("remote"),
            InvocationWord::Literal("local"),
        ];
        let legacy = InvocationArguments::structured(&args).with_dialect(
            crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4),
        );
        let modern = InvocationArguments::structured(&args).with_dialect(
            crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        assert_eq!(
            registry.arg_indices_for_role_words("upvar", legacy, ArgRole::VarWrite),
            Some(vec![])
        );
        assert_eq!(
            registry.arg_indices_for_role_words("upvar", modern, ArgRole::VarWrite),
            Some(vec![2])
        );
    }

    /// `unset x y z` names *every* argument as a variable, not
    /// just the first, so all of them highlight as variables.
    #[test]
    fn unset_marks_every_name() {
        let reg = CommandRegistry::build_default();
        let vars = reg.arg_indices_for_role("unset", &["x", "y", "z"], ArgRole::VarWrite);
        assert_eq!(vars, vec![0, 1, 2]);
    }

    /// `arg_indices_evaluated_in_frame` is the call-site half of
    /// `ArgRole::braced_word_evaluated_in_frame`: it names the positions whose
    /// brace-quoted word the callee still substitutes against the *caller's*
    /// variables. tclsh 9.0.4: `set x 5; expr {$x} + 1` → `6`, while
    /// `lindex {$x} 0` → the two characters `$x` and
    /// `apply {{} {puts $x}}` errors with `can't read "x"` (a fresh frame).
    #[test]
    fn arg_indices_evaluated_in_frame_names_only_caller_frame_code_words() {
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.arg_indices_evaluated_in_frame("expr", &["$x", "+", "1"]),
            vec![0],
        );
        assert!(
            reg.arg_indices_evaluated_in_frame("lindex", &["$x", "0"])
                .is_empty()
        );
        assert!(
            reg.arg_indices_evaluated_in_frame("apply", &["{{} {puts $x}}"])
                .is_empty()
        );
    }

    /// `unset -nocomplain -- a b` skips the leading options and names only the
    /// real variables (`a`, `b`), mirroring `lower_unset`.
    #[test]
    fn unset_skips_leading_options() {
        let reg = CommandRegistry::build_default();
        let vars =
            reg.arg_indices_for_role("unset", &["-nocomplain", "--", "a", "b"], ArgRole::VarWrite);
        assert_eq!(vars, vec![2, 3]);
    }

    /// `unset` recognises only `-nocomplain` / `--` as options — a dash-prefixed
    /// word like `-foo` is a real variable name (verified against tclsh), so it
    /// keeps its `VarWrite` role.
    #[test]
    fn unset_dash_name_is_a_variable() {
        let reg = CommandRegistry::build_default();
        // `unset -foo bar` — both are variables (no `--` needed to reach them).
        assert_eq!(
            reg.arg_indices_for_role("unset", &["-foo", "bar"], ArgRole::VarWrite),
            vec![0, 1]
        );
        // `unset -nocomplain -foo` — `-nocomplain` is skipped, `-foo` is a name.
        assert_eq!(
            reg.arg_indices_for_role("unset", &["-nocomplain", "-foo"], ArgRole::VarWrite),
            vec![1]
        );
    }

    #[test]
    fn arg_indices_for_role_trace_add_variable_var_write() {
        let reg = CommandRegistry::build_default();
        let writes = reg.arg_indices_for_role(
            "trace",
            &["add", "variable", "x", "write", "body"],
            ArgRole::VarWrite,
        );
        // +1 for subcommand offset.  arg "x" is at idx 2 in the
        // full args list (sub args[1] + 1).
        assert!(writes.contains(&2), "VarWrite writes={writes:?}");
    }

    /// `trace add execution` does NOT declare `VarWrite`
    /// (the second arg is a command name, not a variable).
    #[test]
    fn arg_indices_for_role_trace_add_execution_no_var_write() {
        let reg = CommandRegistry::build_default();
        let writes = reg.arg_indices_for_role(
            "trace",
            &["add", "execution", "foo", "enter", "body"],
            ArgRole::VarWrite,
        );
        assert!(writes.is_empty(), "VarWrite writes={writes:?}");
    }

    /// `trace add command`/`execution` trace a command *by name*, so the name
    /// argument is a `CommandName` reference (navigation follows it), while a
    /// `variable` trace's name is not.
    #[test]
    fn arg_indices_for_role_trace_add_command_name_reference() {
        let reg = CommandRegistry::build_default();
        for kind in ["command", "execution"] {
            let names = reg.arg_indices_for_role(
                "trace",
                &["add", kind, "foo", "enter", "body"],
                ArgRole::CommandName,
            );
            // +1 subcommand offset: `foo` is at full-args index 2.
            assert!(names.contains(&2), "{kind}: CommandName names={names:?}");
        }
        let var = reg.arg_indices_for_role(
            "trace",
            &["add", "variable", "x", "write", "body"],
            ArgRole::CommandName,
        );
        assert!(var.is_empty(), "a variable trace names no command: {var:?}");
    }

    /// `trace remove variable` behaves like `trace add variable`:
    /// both alias spellings flow through the same `VarWrite` query.
    #[test]
    fn arg_indices_for_role_trace_remove_variable_var_write() {
        let reg = CommandRegistry::build_default();
        let writes = reg.arg_indices_for_role(
            "trace",
            &["remove", "variable", "y", "write", "body"],
            ArgRole::VarWrite,
        );
        assert!(writes.contains(&2), "VarWrite writes={writes:?}");
    }

    /// `global` / `variable` / `upvar` carry
    /// `CREATES_DYNAMIC_BARRIER` so SSA's barrier-def walk knows the
    /// per-arg list belongs to `var_scoping`, not the role-driven
    /// `VarWrite` query.
    #[test]
    fn creates_dynamic_barrier_trait_marks_scope_aliases() {
        let reg = CommandRegistry::build_default();
        for cmd in &["global", "variable", "upvar"] {
            assert!(
                reg.get(cmd)
                    .unwrap()
                    .traits
                    .contains(Traits::CREATES_DYNAMIC_BARRIER),
                "{cmd} should carry CREATES_DYNAMIC_BARRIER",
            );
        }
        // `set` does NOT carry the trait — its VarWrite at arg 0 is
        // a single-target def, not a vararg list.
        assert!(
            !reg.get("set")
                .unwrap()
                .traits
                .contains(Traits::CREATES_DYNAMIC_BARRIER)
        );
    }

    /// `dict with` / `dict update` arg 0 (the dict variable) plays
    /// both `VarRead` and `VarWrite` roles. This is emitted via
    /// duplicate `(idx, role)` rows in the resolver (the multi-role
    /// observable behaviour is what consumers query).
    /// `DEFERS_BODY` marks the commands that **store** a body rather than
    /// run it, and it is the fail-safe direction that matters: a consumer
    /// asking "can an unreadable body word cost me the proof that this call
    /// completes?" must get `yes` for everything that has not declared
    /// otherwise.
    ///
    /// `BodyKind` cannot answer this — `proc` and `uplevel` are both
    /// `Structural`, because that descriptor answers *which frame*, not
    /// *when* — and neither can the absence of `control_arm_semantics`, which
    /// `uplevel` / `oo::define` share with `proc`. `apply`'s lambda position
    /// is a typed frame boundary; it still declares no `DEFERS_BODY`,
    /// which is what this test checks.
    #[test]
    fn defers_body_marks_only_stored_bodies() {
        use crate::body_kind::BodyKind;
        let mut reg = CommandRegistry::build_default();
        assert!(
            reg.get("proc")
                .unwrap()
                .traits
                .contains(Traits::DEFERS_BODY),
            "`proc` stores its body for a later call"
        );
        // Every command that runs its script as part of the same call — all
        // of which lack control-arm semantics exactly as `proc` does.
        for name in ["uplevel", "apply", "oo::define", "oo::objdefine", "eval"] {
            let spec = reg.get(name).unwrap_or_else(|| panic!("{name} is known"));
            assert!(
                !spec.traits.contains(Traits::DEFERS_BODY),
                "{name} executes its script argument at this call"
            );
        }
        // The trap the flag exists to close: same `BodyKind`, opposite timing.
        assert_eq!(reg.get("proc").unwrap().body_kind, BodyKind::Structural);
        assert_eq!(reg.get("uplevel").unwrap().body_kind, BodyKind::Structural);
        // An iRules handler registration stores its body the same way.
        reg.load_irules();
        assert!(
            reg.get("when")
                .unwrap()
                .traits
                .contains(Traits::DEFERS_BODY)
        );
        // The flag is opt-in, so the silent default is the abstaining one.
        assert!(
            !CommandSpec::DEFAULT.traits.contains(Traits::DEFERS_BODY),
            "an undeclared command must never read as dormant"
        );
        assert!(
            !SubCommand::DEFAULT.traits.contains(Traits::DEFERS_BODY),
            "an undeclared subcommand must never read as dormant"
        );
    }

    /// `apply`'s lambda position is a typed **frame boundary**: the script
    /// runs as part of this call, in a fresh procedure frame.
    ///
    /// The declaration is what lets a consumer walking a statement stream
    /// treat an unreadable `apply $lambda` as the missing answer it is — the
    /// role alone never reached the question, so the walk claimed past a
    /// lambda that can raise.
    ///
    /// Gated on the role, not on the argument number: only the
    /// [`ArgRole::LambdaLiteral`] word is the script. `apply`'s trailing
    /// `arg1 arg2 …` are ordinary values bound to the lambda's parameters.
    #[test]
    fn apply_types_its_lambda_position_as_a_frame_boundary() {
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.control_arm_semantics("apply", &["{{} {puts hi}}"], 0),
            Some(ControlArmSemantics::FrameBoundary),
            "`apply`'s lambda runs now, in a fresh frame"
        );
        for index in 1..4 {
            assert_eq!(
                reg.control_arm_semantics("apply", &["{{a b} {puts hi}}", "1", "2"], index),
                None,
                "argument {index} is a value bound to a parameter, not a script"
            );
        }
        // The role is what the declaration keys on, so it survives a rename of
        // the position and answers for any future command sharing the shape.
        assert_eq!(
            reg.arg_indices_for_role("apply", &["{{} {puts hi}}"], ArgRole::LambdaLiteral),
            vec![0]
        );
    }

    /// Every shipped command carrying [`ArgRole::LambdaLiteral`] types that
    /// position, so the analyser's "read the body only where the registry says
    /// what running it means" gate is never the thing deciding a shipped
    /// command's fate.
    ///
    /// Pinned rather than assumed: `apply` is the only such
    /// command today, so dropping that gate would change nothing here — but a
    /// pack, or a future built-in, can carry the role without typing it, and
    /// then the gate is what keeps an untyped script that runs *now* from
    /// being walked into and claimed. This test says out loud that the shipped
    /// set does not exercise it.
    #[test]
    fn every_shipped_lambda_position_is_typed() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();
        let names: Vec<String> = reg.command_names().map(str::to_owned).collect();
        let mut seen = 0usize;
        for name in names {
            for argc in 1..=4 {
                let args = vec!["{{} {puts hi}}"; argc];
                for index in reg.arg_indices_for_role(&name, &args, ArgRole::LambdaLiteral) {
                    seen += 1;
                    assert!(
                        reg.control_arm_semantics(&name, &args, index).is_some(),
                        "{name}/{argc} carries a lambda at {index} without typing what running \
                         it means"
                    );
                }
            }
        }
        assert!(seen > 0, "the sweep must not be vacuous");
    }

    /// Every command that carries an untyped [`ArgRole::Body`] position and
    /// runs it *now*, pinned by name.
    ///
    /// The analyser reads the absence of `DEFERS_BODY` as "this body runs as
    /// part of this invocation" and lets such a body stop
    /// its fall-through walk. That makes a *missing* `DEFERS_BODY` a silent
    /// wrong answer rather than a missing one, so the inventory is pinned:
    /// adding a Body-role command fails this test until its author puts it on
    /// one of the two lists below, which is the moment to ask the question.
    ///
    /// Store-only forms do not appear here — they carry `DEFERS_BODY` and the
    /// sweep skips them. Commands the registry *types* do not appear either:
    /// a typed arm is answered by [`ControlArmSemantics`], not by this trait.
    ///
    /// Every entry was measured against tclsh 8.6.16 and 9.0.4, which agreed
    /// byte-for-byte, with the shape `proc p {} { FORM {error stop}; set
    /// ::reached 1 }`: `::reached` unset means the body ran here.
    const BODY_RUNS_NOW: &[&str] = &[
        // Core Tcl: script evaluators and definition scripts.
        "::tcl::dict::for",
        "::tcl::dict::map",
        "::tcl::dict::update",
        "::tcl::dict::with",
        "eval",
        "oo::define",
        "oo::objdefine",
        "time",
        "timerate",
        "uplevel",
        // Runs its body now, but *absorbs* its completion — the harness
        // catches a failing test body and reports it. That is a completion
        // boundary, which is a typed answer this trait cannot give, so it
        // sits here and the walk over-abstains on it. Tracked separately.
        "tcltest::test",
        // tcllib: iterators and definition scripts.
        "control::do",
        "fileutil::foreachLine",
        "lfilter",
        "snit::compile",
        "snit::type",
        "snit::widget",
        "snit::widgetadaptor",
        "stooop::class",
        "uri::register",
        // itcl: `class` runs its definition script; `body` and `configbody`
        // install a body against an already-declared member and look like
        // store-only forms, but no itcl is installable in this environment,
        // so they are recorded as unmeasured rather than marked on a guess.
        "itcl::body",
        "itcl::class",
        "itcl::configbody",
        // iRules: these run the script now, in a different flow context.
        "clientside",
        "peer",
        "serverside",
    ];

    /// The gate for [`BODY_RUNS_NOW`]: no command may carry an untyped
    /// `ArgRole::Body` position without an explicit stance on whether that
    /// body runs now or is stored.
    #[test]
    fn every_untyped_body_role_declares_whether_its_body_runs_now() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();
        let names: Vec<String> = reg.command_names().map(str::to_owned).collect();
        let mut unclassified: Vec<String> = Vec::new();
        let mut seen_runs_now = 0usize;
        for name in &names {
            // Role resolution is argv-shaped, so probe the argument counts a
            // grammar could take, and the leading words that select the
            // script-bearing subforms of the ensembles in this sweep. Both
            // questions are asked per shape, exactly as the analyser asks
            // them: `DEFERS_BODY` can live on a subcommand rather than the
            // command (`package ifneeded`), so a spec-level test would miss it.
            let mut untyped_body = false;
            for argc in 1..=6usize {
                for lead in ["x", "idle", "ifneeded", "0"] {
                    let mut args = vec!["x"; argc];
                    args[0] = lead;
                    // One name can carry more than one spec, resolved by
                    // dialect — `after` has a Tcl spec and an iRules one, and
                    // a sweep that looked at only one would let the other lose
                    // its trait unnoticed. Iterating the specs (rather than a
                    // list of points) also keeps every probe at a point the
                    // command actually resolves at: an unresolvable call
                    // answers with an empty trait set, which is indistinguish-
                    // able from a spec whose only trait was `DEFERS_BODY`
                    // until it was dropped — precisely the regression this
                    // gate exists to catch.
                    for spec in reg.specs(name) {
                        // Probe at a point the spec actually resolves under:
                        // its own first row, or any Tcl release when it names
                        // no provider.
                        let point = spec.surface.and_then(|rows| rows.first()).map_or(
                            SurfaceQuery::any_release(Family::Tcl),
                            |row| match row.provider {
                                SpecProvider::Core(family) => SurfaceQuery::any_release(family),
                                SpecProvider::Package(_) => SurfaceQuery::any_release(Family::Tcl),
                            },
                        );
                        let traits = reg.invocation_traits(name, &args, Some(point));
                        if spec.traits.contains(Traits::DEFERS_BODY)
                            || traits.contains(Traits::DEFERS_BODY)
                            // A `CONTROL_FLOW` command is the registry's own
                            // business: the analyser refuses to walk one whose
                            // arm it cannot type rather than reading the body,
                            // so a malformed probe shape of `if` / `try` is
                            // not a missing stance.
                            || traits.contains(Traits::CONTROL_FLOW)
                        {
                            continue;
                        }
                        let Some(invocation) = reg.resolve_invocation(name, &args, Some(point))
                        else {
                            continue;
                        };
                        for index in invocation
                            .argument_roles()
                            .0
                            .into_iter()
                            .filter(|(_, role)| *role == ArgRole::Body)
                            .map(|(position, _)| {
                                invocation.semantics.argument_offset + usize::from(position)
                            })
                        {
                            if reg.control_arm_semantics(name, &args, index).is_none()
                                && !reg
                                    .get_for_surface(name, Some(point))
                                    .is_some_and(|selected| {
                                        has_authored_body_timing(
                                            selected,
                                            &invocation,
                                            &args,
                                            index,
                                        )
                                    })
                            {
                                untyped_body = true;
                            }
                        }
                    }
                }
            }
            if !untyped_body {
                continue;
            }
            if BODY_RUNS_NOW.contains(&name.as_str()) {
                seen_runs_now += 1;
            } else {
                unclassified.push(name.clone());
            }
        }
        assert!(
            seen_runs_now > 0,
            "the sweep must not be vacuous — no runs-now command was reached"
        );
        assert!(
            unclassified.is_empty(),
            "these commands carry an untyped ArgRole::Body and declare no \
             stance: supply exact argument-sensitive script timing, store the body (give the spec \
             Traits::DEFERS_BODY, with an oracle row) or they run it now (add \
             them to BODY_RUNS_NOW, with an oracle row). Leaving them out \
             makes the analyser's fall-through walk treat their body as one \
             that can stop control — see issue #1672. Unclassified: {unclassified:?}"
        );
    }

    fn has_authored_body_timing(
        spec: &CommandSpec,
        invocation: &ResolvedInvocation<'_, '_>,
        arguments: &[&str],
        index: usize,
    ) -> bool {
        let (resolver, offset) = invocation
            .subcommand
            .resolved()
            .and_then(|selected| spec.subcommand(selected.canonical_name))
            .map_or((spec.script_timing_resolver, 0), |sub| {
                (sub.script_timing_resolver, 1)
            });
        resolver.is_some_and(|resolve| {
            resolve(crate::InvocationArguments::literals(
                arguments.get(offset..).unwrap_or(&[]),
            ))
            .iter()
            .any(|(position, _)| offset + usize::from(*position) == index)
        })
    }

    /// `DEFERS_BODY` and typed control-arm semantics are answers to different
    /// questions, and today no command gives both: everything the registry
    /// types (`if`, `switch`, `namespace eval`, `catch`, `try`, the loops,
    /// `apply`'s lambda)
    /// runs the arm it types.
    ///
    /// That is *why* a consumer cannot currently observe which of the two
    /// checks fires first — dropping either leaves every shipped command
    /// classified the same. It is a property of today's data, not of the
    /// question, so it is pinned rather than assumed: a future command that
    /// stored a body the registry also typed as an arm would make the order
    /// load-bearing, and this is the assertion that says so out loud before
    /// the analyser silently starts trusting a dormant body's arm.
    #[test]
    fn no_command_both_defers_a_body_and_types_it_as_a_control_arm() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();
        let deferring: Vec<String> = reg
            .command_names()
            .filter(|name| {
                reg.get(name)
                    .is_some_and(|spec| spec.traits.contains(Traits::DEFERS_BODY))
            })
            .map(str::to_owned)
            .collect();
        assert!(!deferring.is_empty(), "the sweep must not be vacuous");
        // Positive control: the probe below has to be able to *see* a typed
        // arm, or the sweep proves nothing. Role resolution is argv-shaped, so
        // an empty argument list makes every command answer `None` — including
        // `if`. Probe each deferring command across the argument counts its
        // grammar could take instead.
        assert_eq!(
            reg.control_arm_semantics("if", &["{$c}", "{body}"], 1),
            Some(ControlArmSemantics::Selected),
            "the probe must detect a typed arm, else the sweep is inert"
        );
        for name in deferring {
            for argc in 1..=6 {
                let args = vec!["x"; argc];
                for index in 0..argc {
                    assert_eq!(
                        reg.control_arm_semantics(&name, &args, index),
                        None,
                        "{name}/{argc} declares DEFERS_BODY but types argument {index} as a \
                         control arm; `unreadable_body_is_material`'s check order now decides \
                         its behaviour"
                    );
                }
            }
        }
    }

    /// Every spec defaults to `BodyKind::Plain` unless it
    /// opts into `Structural`.
    #[test]
    fn body_kind_default_plain() {
        use crate::body_kind::BodyKind;
        let reg = CommandRegistry::build_default();
        assert_eq!(reg.get("set").unwrap().body_kind, BodyKind::Plain);
        assert_eq!(reg.get("if").unwrap().body_kind, BodyKind::Plain);
        assert_eq!(reg.get("while").unwrap().body_kind, BodyKind::Plain);
        assert_eq!(reg.get("foreach").unwrap().body_kind, BodyKind::Plain);
    }

    /// `proc` / `oo::class` / `oo::define` / `oo::objdefine`
    /// stamp `Structural` so SSA skips their body args from the
    /// enclosing block's data flow.
    #[test]
    fn body_kind_structural_marks() {
        use crate::body_kind::BodyKind;
        let reg = CommandRegistry::build_default();
        assert_eq!(reg.get("proc").unwrap().body_kind, BodyKind::Structural);
        assert_eq!(
            reg.get("oo::class").unwrap().body_kind,
            BodyKind::Structural
        );
        assert_eq!(
            reg.get("oo::define").unwrap().body_kind,
            BodyKind::Structural
        );
        assert_eq!(
            reg.get("oo::objdefine").unwrap().body_kind,
            BodyKind::Structural
        );
        assert_eq!(
            reg.get("snit::method").unwrap().body_kind,
            BodyKind::Structural
        );
        assert_eq!(
            reg.get("snit::typemethod").unwrap().body_kind,
            BodyKind::Structural
        );
        assert_eq!(
            reg.get("uri::register").unwrap().body_kind,
            BodyKind::Structural
        );
    }

    /// iRules `when` event handler bodies are structural.
    #[test]
    fn body_kind_irules_when_structural() {
        use crate::body_kind::BodyKind;
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();
        assert_eq!(reg.get("when").unwrap().body_kind, BodyKind::Structural);
    }

    /// `plain_body_arg_indices` surfaces the same-frame control-flow /
    /// `eval` bodies a nested dispatch (e.g. `TclOO` `my method`) still
    /// executes inside — every `Plain`-kind `ArgRole::Body` argument.
    #[test]
    fn plain_body_arg_indices_covers_same_frame_bodies() {
        let reg = CommandRegistry::build_default();
        assert_eq!(reg.plain_body_arg_indices("if", &["1", "{body}"]), vec![1]);
        assert_eq!(
            reg.plain_body_arg_indices("while", &["1", "{body}"]),
            vec![1]
        );
        assert_eq!(
            reg.plain_body_arg_indices("foreach", &["v", "$list", "{body}"]),
            vec![2]
        );
        assert_eq!(reg.plain_body_arg_indices("eval", &["{body}"]), vec![0]);
        assert_eq!(
            reg.plain_body_arg_indices("catch", &["{body}", "res"]),
            vec![0]
        );
    }

    /// `Structural` bodies (`proc`, `oo::class create`, `uplevel`,
    /// `namespace eval`) never come back from `plain_body_arg_indices` —
    /// they run in a definition / different-frame context, so a dispatch
    /// found inside is not still the caller's scope.
    #[test]
    fn plain_body_arg_indices_excludes_structural_bodies() {
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.plain_body_arg_indices("proc", &["name", "args", "{body}"]),
            Vec::<usize>::new()
        );
        assert_eq!(
            reg.plain_body_arg_indices("uplevel", &["1", "{body}"]),
            Vec::<usize>::new()
        );
        assert_eq!(
            reg.plain_body_arg_indices("namespace", &["eval", "ns", "{body}"]),
            Vec::<usize>::new()
        );
    }

    /// An unknown command name (a user proc, a `TclOO` method body's own
    /// `unknownProc arg` call) has no registry spec at all — returns empty
    /// rather than panicking.
    #[test]
    fn plain_body_arg_indices_unknown_command_is_empty() {
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.plain_body_arg_indices("myUserProc", &["a", "b"]),
            Vec::<usize>::new()
        );
    }

    /// `body_arg_implicit_args` defaults to 0 and is set on
    /// `fileutil::updateInPlace` (which appends file contents to
    /// the body's first command at runtime).
    #[test]
    fn body_arg_implicit_args_defaults_zero_except_fileutil_updateinplace() {
        let reg = CommandRegistry::build_default();
        assert_eq!(reg.get("set").unwrap().body_arg_implicit_args, 0);
        assert_eq!(reg.get("proc").unwrap().body_arg_implicit_args, 0);
        assert_eq!(
            reg.get("fileutil::updateInPlace")
                .unwrap()
                .body_arg_implicit_args,
            1,
        );
    }

    #[test]
    fn procedure_definition_words_place_the_three_words_of_tcl_proc() {
        assert_eq!(
            CommandRegistry::build_default()
                .procedure_definition_words("proc", &["f", "{a b}", "{ body }"]),
            None,
            "catalogue presence supplies no native formal-argument grammar"
        );
        let reg = CommandRegistry::build_default()
            .project_for_profile(DialectProfile::find("tcl8.6").unwrap());
        assert_eq!(
            reg.procedure_definition_words("proc", &["f", "{a b}", "{ body }"]),
            Some(ProcedureWords {
                name: 0,
                params: 1,
                statics: None,
                body: 2,
            })
        );
    }

    #[test]
    fn procedure_definition_words_answers_only_for_a_procedure_definer() {
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.procedure_definition_words("set", &["x", "1", "2"]),
            None
        );
        assert_eq!(reg.procedure_definition_words("proc", &["f"]), None);
        assert_eq!(reg.procedure_definition_words("no_such_head", &[]), None);
    }

    /// The providers of a name across the compiled-in universe: Expect's
    /// `system` is offered by that package alone, `dict` by a core Tcl
    /// window, a name nothing defines by nobody.
    #[test]
    fn providers_in_any_dialect_names_who_offers_a_command() {
        let reg = CommandRegistry::build_default();
        let system = reg
            .providers_in_any_dialect("system")
            .expect("system is Expect's");
        assert!(!system.unrestricted);
        assert_eq!(system.providers, [SpecProvider::Package("expect")]);

        let dict = reg.providers_in_any_dialect("dict").expect("dict is Tcl's");
        assert!(
            dict.providers.contains(&SpecProvider::Core(Family::Tcl)),
            "{dict:?}"
        );

        assert!(
            reg.providers_in_any_dialect("no_such_command_anywhere")
                .is_none()
        );
        assert_eq!(
            reg.providers_in_any_dialect("::dict"),
            reg.providers_in_any_dialect("dict"),
            "a rooted spelling names the same providers"
        );
    }

    #[test]
    fn arg_indices_for_role_dict_with_multirole() {
        let reg = CommandRegistry::build_default();
        let reads = reg.arg_indices_for_role("dict", &["with", "$var", "body"], ArgRole::VarRead);
        let writes = reg.arg_indices_for_role("dict", &["with", "$var", "body"], ArgRole::VarWrite);
        assert!(reads.contains(&1), "VarRead reads={reads:?}");
        assert!(writes.contains(&1), "VarWrite writes={writes:?}");
    }

    #[test]
    fn arg_indices_for_role_dict_update_multirole() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let reg = CommandRegistry::build_default();
        let reads = reg.arg_indices_for_role(
            "dict",
            &["update", "$var", "k", "vname", "body"],
            ArgRole::VarRead,
        );
        let writes = reg.arg_indices_for_role(
            "dict",
            &["update", "$var", "k", "vname", "body"],
            ArgRole::VarWrite,
        );
        assert!(reads.contains(&1), "VarRead reads={reads:?}");
        assert!(writes.contains(&1), "VarWrite writes={writes:?}");
    }

    #[test]
    fn dynamic_arg_role_resolution() {
        let reg = CommandRegistry::build_default();
        // if expr body elseif expr body else body
        let roles = reg.arg_indices_for_role(
            "if",
            &[
                "$x", "then", "body1", "elseif", "$y", "body2", "else", "body3",
            ],
            ArgRole::Body,
        );
        // Bodies are at positions 2, 5, 7
        assert!(roles.contains(&2));
        assert!(roles.contains(&5));
        assert!(roles.contains(&7));
    }

    #[test]
    fn load_irules_dialect() {
        let mut reg = CommandRegistry::build_default();
        assert!(reg.get("HTTP::header").is_none()); // not loaded yet
        reg.load_irules();
        assert!(reg.get("HTTP::header").is_some());
        assert!(reg.len() > 200); // should have 1000+ commands now
    }

    #[test]
    fn irules_command_has_irules_dialect() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();
        let spec = reg.get("HTTP::header").unwrap();
        assert_eq!(spec.surface, Some(SpecSurface::IRULES));
    }

    #[test]
    fn irules_idempotent_load() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();
        let count1 = reg.len();
        reg.load_irules(); // second load should be no-op
        assert_eq!(reg.len(), count1);
    }

    #[test]
    fn default_includes_stdlib_and_tcllib() {
        let reg = CommandRegistry::build_default();
        // stdlib and tcllib are loaded by default
        assert!(reg.len() > 200);
    }

    #[test]
    fn load_tk_dialect() {
        // Tk is part of the always-known base registry now, so it is present
        // by default and a later `load_surface(Tk)` is an idempotent no-op.
        let reg = CommandRegistry::build_default();
        assert!(
            reg.get("grid").is_some(),
            "Tk commands are loaded by default"
        );
        let base_count = reg.len();
        let mut reg2 = CommandRegistry::build_default();
        reg2.load_surface(SurfaceLayer::Package("Tk"));
        assert_eq!(
            reg2.len(),
            base_count,
            "loading the Tk surface is a no-op after the default load"
        );
    }

    #[test]
    fn load_iapps_dialect() {
        let mut reg = CommandRegistry::build_default();
        reg.load_surface(SurfaceLayer::Package("iapps"));
        assert!(reg.len() > 100);
    }

    #[test]
    fn load_expect_dialect() {
        let mut reg = CommandRegistry::build_default();
        reg.load_surface(SurfaceLayer::Package("expect"));
        assert!(reg.get("expect").is_some() || reg.get("spawn").is_some());
    }

    #[test]
    fn resolve_call_unknown_command_returns_none() {
        let reg = CommandRegistry::build_default();
        assert!(reg.resolve_call("no_such_cmd", &[], None).is_none());
    }

    #[test]
    fn resolve_invocation_retains_words_and_canonicalises_the_registry_name() {
        let reg = CommandRegistry::build_default();
        let args = ["create", "key", "value"];
        let resolved = reg
            .resolve_invocation(
                "::dict",
                &args,
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("global dict spelling resolves through the registry");

        assert_eq!(resolved.words.head_literal(), Some("::dict"));
        assert_eq!(
            resolved.words.arguments(),
            InvocationArguments::literals(&args)
        );
        assert_eq!(resolved.canonical_command, "dict");
        let sub = resolved
            .subcommand
            .resolved()
            .expect("dict create subcommand");
        assert_eq!(sub.spelling, "create");
        assert_eq!(sub.canonical_name, "create");
        assert_eq!(
            resolved.subcommand.kind(),
            Some(crate::SubcommandResolutionKind::Exact)
        );
        assert_eq!(resolved.semantics.argument_offset, 1);
    }

    #[test]
    fn structured_resolution_keeps_dynamic_subcommands_indeterminate() {
        let reg = CommandRegistry::build_default();
        let arguments = [
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Literal("text"),
        ];
        let resolved = reg
            .resolve_structured_invocation(
                InvocationWords::structured(crate::InvocationWord::Literal("string"), &arguments),
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .resolved()
            .expect("a literal command head is registry-known");

        assert!(matches!(
            resolved.subcommand,
            SubcommandResolution::Indeterminate {
                word_kind: crate::InvocationWordKind::Dynamic
            }
        ));
        assert!(resolved.form.is_none());
        assert_eq!(
            resolved.semantics.operation,
            crate::SemanticOperationId::Invoke,
            "a dynamic subcommand cannot select specialised subcommand metadata"
        );
    }

    #[test]
    fn expanded_argument_does_not_match_an_arity_form() {
        let reg = CommandRegistry::build_default();
        let arguments = [crate::InvocationWord::Expanded];
        let resolved = reg
            .resolve_structured_invocation(
                InvocationWords::structured(crate::InvocationWord::Literal("incr"), &arguments),
                None,
            )
            .resolved()
            .expect("a literal command head is registry-known");

        assert!(resolved.form.is_none());
        assert_eq!(
            resolved.semantics.operation,
            crate::SemanticOperationId::StructuredLowering(LoweringHookId::Incr),
            "the command-level common operation remains visible without claiming a form"
        );
    }

    #[test]
    fn variable_effect_footprints_keep_actual_availability_over_argument_dialect() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // This is authored metadata, not a Native read/store or command result.
        let mut registry = CommandRegistry::build_default();
        registry.insert(crate::CommandSpec {
            name: "gated-write",
            surface: registry.get("dict").unwrap().surface,
            ..registry.get("set").unwrap().clone()
        });
        let store = std::sync::Arc::new(registry);
        let arguments = [
            crate::InvocationWord::Literal("$scalar(open"),
            crate::InvocationWord::Literal("VALUE"),
        ];
        let words = crate::InvocationWords::structured(
            crate::InvocationWord::Literal("gated-write"),
            &arguments,
        )
        .with_profile(tcl_dialect::DialectProfile::find("tcl8.6"));
        for (environment, available) in [("tcl8.4", false), ("tcl9.0", true)] {
            let context = crate::model::ingress::static_context_for(environment)
                .with_command_store(std::sync::Arc::clone(&store));
            let projection = store.variable_write_projection_in_resolved_context(
                context.context(),
                words,
                tcl_dialect::model::InvocationRealm::RuleLoader,
            );
            assert_eq!(
                projection
                    .literal_names
                    .contains(&"$scalar(open".to_owned()),
                available,
                "{environment}"
            );
            assert_eq!(
                projection.opaque_variable_frame, !available,
                "{environment}"
            );
        }
    }

    #[test]
    fn variable_write_projection_keeps_repeated_literal_targets() {
        let reg = CommandRegistry::build_default();
        let arguments = [
            crate::InvocationWord::Literal("{(a)(b)}"),
            crate::InvocationWord::Literal("ab"),
            crate::InvocationWord::Literal("whole"),
            crate::InvocationWord::Literal("first"),
            crate::InvocationWord::Literal("second"),
        ];
        let projection = reg.variable_write_projection(InvocationWords::structured(
            crate::InvocationWord::Literal("regexp"),
            &arguments,
        ));

        assert_eq!(projection.literal_names, ["whole", "first", "second"]);
        assert!(!projection.opaque_variable_frame);
    }

    #[test]
    fn variable_write_projection_roots_registry_global_option_targets() {
        let reg = crate::model::semantic::SemanticContext::for_environment("tk").commands();
        let arguments = [
            crate::InvocationWord::Literal(".e"),
            crate::InvocationWord::Literal("-textvariable"),
            crate::InvocationWord::Literal("value"),
        ];
        let projection = reg.variable_write_projection(InvocationWords::structured(
            crate::InvocationWord::Literal("entry"),
            &arguments,
        ));

        assert_eq!(projection.literal_names, ["::value"]);
        assert!(!projection.opaque_variable_frame);
        assert!(
            CommandRegistry::build_default()
                .variable_write_projection(InvocationWords::structured(
                    crate::InvocationWord::Literal("entry"),
                    &arguments,
                ))
                .literal_names
                .is_empty(),
            "an unprovided catalogue widget cannot establish its option targets"
        );
    }

    #[test]
    fn variable_write_projection_respects_registry_write_arity_floor() {
        let reg = CommandRegistry::build_default();
        for (head, arguments) in [
            (
                "::regexp",
                vec![
                    crate::InvocationWord::Dynamic,
                    crate::InvocationWord::Dynamic,
                ],
            ),
            (
                "::regsub",
                vec![
                    crate::InvocationWord::Dynamic,
                    crate::InvocationWord::Dynamic,
                    crate::InvocationWord::Dynamic,
                ],
            ),
        ] {
            let projection = reg.variable_write_projection(InvocationWords::structured(
                crate::InvocationWord::Literal(head),
                &arguments,
            ));
            assert_eq!(projection, VariableWriteProjection::default(), "{head}");
        }

        let arguments = [
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Dynamic,
        ];
        let projection = reg.variable_write_projection(InvocationWords::structured(
            crate::InvocationWord::Literal("::regexp"),
            &arguments,
        ));
        assert!(projection.literal_names.is_empty());
        assert!(projection.opaque_variable_frame);
    }

    #[test]
    fn variable_write_projection_marks_dynamic_targets_and_argv_opaque() {
        let reg = CommandRegistry::build_default();
        for arguments in [
            vec![
                crate::InvocationWord::Dynamic,
                crate::InvocationWord::Literal("value"),
            ],
            vec![crate::InvocationWord::Expanded],
        ] {
            let projection = reg.variable_write_projection(InvocationWords::structured(
                crate::InvocationWord::Literal("set"),
                &arguments,
            ));
            assert!(projection.literal_names.is_empty());
            assert!(projection.opaque_variable_frame);
        }
    }

    #[test]
    fn variable_write_projection_marks_computed_heads_opaque() {
        let reg = CommandRegistry::build_default();
        let no_arguments = [];
        for head in [
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Expanded,
            crate::InvocationWord::Opaque,
        ] {
            let projection =
                reg.variable_write_projection(InvocationWords::structured(head, &no_arguments));
            assert!(projection.literal_names.is_empty());
            assert!(projection.opaque_variable_frame);
        }
    }

    #[test]
    fn variable_write_projection_distinguishes_aliases_from_value_writes() {
        let reg = CommandRegistry::build_default();
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = crate::InvocationDialect::for_version(version);
            for (head, arguments) in [
                ("global", vec![crate::InvocationWord::Literal("item")]),
                (
                    "upvar",
                    vec![
                        crate::InvocationWord::Literal("1"),
                        crate::InvocationWord::Literal("outer"),
                        crate::InvocationWord::Literal("local"),
                    ],
                ),
                (
                    "trace",
                    vec![
                        crate::InvocationWord::Literal("add"),
                        crate::InvocationWord::Literal("variable"),
                        crate::InvocationWord::Literal("item"),
                        crate::InvocationWord::Literal("write"),
                        crate::InvocationWord::Literal("callback"),
                    ],
                ),
            ] {
                let projection = reg.variable_write_projection(
                    InvocationWords::structured(crate::InvocationWord::Literal(head), &arguments)
                        .with_dialect(dialect),
                );
                assert_eq!(projection, VariableWriteProjection::default(), "{head}");
            }

            let arguments = [
                crate::InvocationWord::Literal("first"),
                crate::InvocationWord::Literal("value"),
                crate::InvocationWord::Literal("declared_only"),
            ];
            let projection = reg.variable_write_projection(
                InvocationWords::structured(crate::InvocationWord::Literal("variable"), &arguments)
                    .with_dialect(dialect),
            );
            assert_eq!(projection.literal_names, ["first"]);
            assert!(!projection.opaque_variable_frame);
        }
        let unknown = [crate::InvocationWord::Literal("item")];
        let projection = reg.variable_write_projection(InvocationWords::structured(
            crate::InvocationWord::Literal("global"),
            &unknown,
        ));
        assert!(projection.literal_names.is_empty());
        assert!(
            projection.opaque_variable_frame,
            "missing alias naming policy remains unknown"
        );
    }

    #[test]
    fn variable_write_projection_does_not_infer_roles_from_an_unrelated_resolver() {
        let reg = CommandRegistry::build_default();
        let arguments = [crate::InvocationWord::Dynamic];
        let projection = reg.variable_write_projection(InvocationWords::structured(
            crate::InvocationWord::Literal("puts"),
            &arguments,
        ));
        assert_eq!(projection, VariableWriteProjection::default());
    }

    /// An unbind reads its place's existence: the
    /// read projection names a destroyer's targets, past `-nocomplain` and
    /// `--`, where the write projection names none, and a substituted target
    /// widens the frame.
    #[test]
    fn variable_read_projection_names_a_destroyers_targets() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        use crate::InvocationWord::{Dynamic, Literal};
        let reg = CommandRegistry::build_default();
        for (arguments, expected) in [
            (vec![Literal("x")], vec!["x"]),
            (
                vec![
                    Literal("-nocomplain"),
                    Literal("--"),
                    Literal("a"),
                    Literal("b"),
                ],
                vec!["a", "b"],
            ),
        ] {
            let words = InvocationWords::structured(Literal("unset"), &arguments);
            assert_eq!(reg.variable_read_projection(words).literal_names, expected);
            assert_eq!(
                reg.variable_write_projection(words),
                VariableWriteProjection::default()
            );
        }
        let computed = [Dynamic];
        let projection =
            reg.variable_read_projection(InvocationWords::structured(Literal("unset"), &computed));
        assert!(projection.literal_names.is_empty());
        assert!(projection.opaque_variable_frame);
        let query = [Literal("exists"), Literal("x")];
        assert_eq!(
            reg.variable_read_projection(InvocationWords::structured(Literal("info"), &query))
                .literal_names,
            vec!["x"]
        );
    }

    #[test]
    fn literal_adapter_matches_the_structured_literal_view() {
        let reg = CommandRegistry::build_default();
        let arguments = ["counter"];
        let adapter = reg
            .resolve_invocation("incr", &arguments, None)
            .expect("literal adapter resolves");
        let structured = reg
            .resolve_structured_invocation(InvocationWords::literals("incr", &arguments), None)
            .resolved()
            .expect("structured literal input resolves");

        assert_eq!(adapter.canonical_command, structured.canonical_command);
        assert_eq!(adapter.subcommand, structured.subcommand);
        assert_eq!(
            adapter.form.map(|form| form.name),
            structured.form.map(|form| form.name)
        );
        assert_eq!(adapter.semantics.operation, structured.semantics.operation);
    }

    #[test]
    fn structured_resolution_reports_computed_and_unknown_command_heads() {
        let reg = CommandRegistry::build_default();
        let no_arguments: [crate::InvocationWord<'_>; 0] = [];

        let computed = reg.resolve_structured_invocation(
            InvocationWords::structured(crate::InvocationWord::Dynamic, &no_arguments),
            None,
        );
        assert_eq!(
            computed.unresolved(),
            Some(crate::InvocationResolutionUnresolved::ComputedHead {
                word_kind: crate::InvocationWordKind::Dynamic,
            })
        );

        let no_literal_arguments: [&str; 0] = [];
        let unknown = reg.resolve_structured_invocation(
            InvocationWords::literals("not-a-registry-command", &no_literal_arguments),
            None,
        );
        assert_eq!(
            unknown.unresolved(),
            Some(crate::InvocationResolutionUnresolved::UnknownLiteralHead {
                spelling: "not-a-registry-command",
            })
        );
    }

    #[test]
    fn irules_placement_contract_owns_declaration_and_execution_contexts() {
        use crate::events::{IrulesCommandPlacement as Placement, IrulesExecutionContext as Ctx};

        let registry = crate::model::ingress::static_context_for("f5-irules").commands();
        for declaration in ["when", "proc", "timing", "priority"] {
            assert_eq!(
                registry.irules_command_placement(declaration, Ctx::TopLevel),
                Placement::Allowed,
                "{declaration} is an iRules top-level declaration"
            );
            assert_eq!(
                registry.irules_command_placement(declaration, Ctx::EventBody),
                Placement::RequiresTopLevel,
                "{declaration} cannot be nested in an event"
            );
            assert_eq!(
                registry.irules_command_placement(declaration, Ctx::ProcedureBody),
                Placement::RequiresTopLevel,
                "{declaration} cannot be nested in a proc"
            );
        }
        for executable in ["set", "HTTP::uri", "call", "user_proc"] {
            assert_eq!(
                registry.irules_command_placement(executable, Ctx::TopLevel),
                Placement::RequiresEventOrProcedure,
                "{executable} cannot execute at iRules top level"
            );
            assert_eq!(
                registry.irules_command_placement(executable, Ctx::EventBody),
                Placement::Allowed
            );
            assert_eq!(
                registry.irules_command_placement(executable, Ctx::ProcedureBody),
                Placement::Allowed
            );
        }
    }

    #[test]
    fn irules_declaration_owner_requires_braced_source_bodies() {
        use crate::events::IrulesTopLevelDeclaration as Declaration;

        let registry = crate::model::ingress::static_context_for("f5-irules").commands();
        let events = crate::events::EventRegistry::build();
        let declaration = |name: &str, args: &[&str]| {
            let body_index = if name == "proc" {
                2
            } else {
                args.len().saturating_sub(1)
            };
            let tokens = args
                .iter()
                .enumerate()
                .map(|(index, _)| {
                    tcl_lexer::Token::new(
                        if index == body_index {
                            tcl_lexer::TokenType::Str
                        } else {
                            tcl_lexer::TokenType::Esc
                        },
                        tcl_lexer::Span::empty(0),
                    )
                })
                .collect::<Vec<_>>();
            let single = vec![true; args.len()];
            let closed = tokens
                .iter()
                .map(|token| token.kind == tcl_lexer::TokenType::Str)
                .collect::<Vec<_>>();
            registry.irules_top_level_declaration(
                name,
                crate::events::IrulesDeclarationArguments::new(args, &tokens, &single, &closed)
                    .expect("parallel source-word views"),
                &events,
            )
        };
        assert!(matches!(
            declaration("when", &["HTTP_REQUEST", "set x 1"]),
            Some(Declaration::Event { .. })
        ));
        assert!(matches!(
            declaration("::when", &["HTTP_REQUEST", "set x 1"]),
            Some(Declaration::Event { .. })
        ));
        assert!(declaration("when", &["BOGUS_EVENT", "pool x"]).is_none());
        for valid in [
            &["HTTP_REQUEST", "body"][..],
            &["HTTP_REQUEST", "priority", "100", "body"][..],
            &["HTTP_REQUEST", "timing", "on", "body"][..],
            &[
                "HTTP_REQUEST",
                "priority",
                "100",
                "timing",
                "disable",
                "body",
            ][..],
            &["HTTP_REQUEST", "priority", "0", "body"][..],
            &["HTTP_REQUEST", "priority", "1000", "body"][..],
        ] {
            assert!(declaration("when", valid).is_some());
        }
        for invalid in [
            &["HTTP_REQUEST"][..],
            &["HTTP_REQUEST", "body", "trailing"][..],
            &["HTTP_REQUEST", "priority", "bad", "body"][..],
            &["HTTP_REQUEST", "priority", "-1", "body"][..],
            &["HTTP_REQUEST", "priority", "1001", "body"][..],
            &["HTTP_REQUEST", "timing", "maybe", "body"][..],
            &["HTTP_REQUEST", "timing", "on", "priority", "100", "body"][..],
        ] {
            assert!(declaration("when", invalid).is_none());
        }
        assert!(matches!(
            declaration("proc", &["p", "", "return"]),
            Some(Declaration::Procedure { .. })
        ));
        for value in ["0", "500", "1000"] {
            assert_eq!(
                declaration("priority", &[value]),
                Some(Declaration::Priority {
                    value: value.parse().unwrap()
                })
            );
        }
        for invalid in ["-1", "1001", "bad"] {
            assert!(declaration("priority", &[invalid]).is_none());
        }
        assert_eq!(
            declaration("timing", &["enable"]),
            Some(Declaration::Timing { enabled: true })
        );
        for malformed in [&["p", ""][..], &["p", "", "return", "extra"][..]] {
            assert!(declaration("proc", malformed).is_none());
        }
    }

    #[test]
    fn irules_declaration_owner_rejects_bare_and_quoted_source_bodies() {
        let registry = crate::model::ingress::static_context_for("f5-irules").commands();
        for (name, args, body_index) in [
            ("when", &["HTTP_REQUEST", "set x 1"][..], 1),
            ("proc", &["p", "", "return"][..], 2),
        ] {
            let mut tokens =
                vec![
                    tcl_lexer::Token::new(tcl_lexer::TokenType::Esc, tcl_lexer::Span::empty(0),);
                    args.len()
                ];
            let single = vec![true; args.len()];
            let closed = vec![false; args.len()];
            for (kind, in_quote) in [
                (tcl_lexer::TokenType::Esc, false),
                (tcl_lexer::TokenType::Esc, true),
                // Lexer recovery uses Str for `{body` at EOF; the separate
                // closed-brace fact must keep it out of declarations too.
                (tcl_lexer::TokenType::Str, false),
            ] {
                tokens[body_index] = tcl_lexer::Token {
                    kind,
                    in_quote,
                    ..tokens[body_index]
                };
                let arguments =
                    crate::events::IrulesDeclarationArguments::new(args, &tokens, &single, &closed)
                        .expect("parallel source-word views");
                assert!(
                    registry
                        .irules_top_level_declaration_shape(name, arguments)
                        .is_none(),
                    "{name} must reject a non-closed-braced body"
                );
            }
        }
    }

    #[test]
    fn literal_form_selectors_choose_the_longest_static_match() {
        const FORMS: &[CommandForm] = &[
            CommandForm {
                name: "tag-one",
                arity: Arity::exact(1),
                literal_argument_prefix: Some(LiteralArgumentPrefix::unique(&["tag"])),
                ..CommandForm::DEFAULT
            },
            CommandForm {
                name: "tag-fallback",
                arity: Arity::exact(3),
                literal_argument_prefix: Some(LiteralArgumentPrefix::unique(&["tag"])),
                ..CommandForm::DEFAULT
            },
            CommandForm {
                name: "tag-bind",
                arity: Arity::exact(3),
                literal_argument_prefix: Some(LiteralArgumentPrefix::unique(&["tag", "bind"])),
                ..CommandForm::DEFAULT
            },
        ];
        const SPEC: CommandSpec = CommandSpec {
            name: "literal-form-fixture",
            arity: Arity::any(),
            command_forms: FORMS,
            ..CommandSpec::DEFAULT
        };

        let mut registry = CommandRegistry::build_default();
        registry.insert(SPEC);
        let form = |args: &[&str]| {
            registry
                .resolve_invocation("literal-form-fixture", args, None)
                .and_then(|resolved| resolved.form)
                .map(|form| form.name)
        };

        assert_eq!(form(&["tag"]), Some("tag-one"), "arity disambiguates");
        assert_eq!(
            form(&["tag", "bind", "script"]),
            Some("tag-bind"),
            "the longer selector overrides its completed prefix"
        );
        assert_eq!(
            form(&["ta", "bi", "script"]),
            Some("tag-bind"),
            "each selector word still accepts a unique abbreviation"
        );
        assert_eq!(
            form(&["tag", "other", "value"]),
            Some("tag-fallback"),
            "a static non-match proves that the shorter selector applies"
        );

        let dynamic = [
            InvocationWord::Literal("tag"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("script"),
        ];
        let resolved = registry
            .resolve_structured_invocation(
                InvocationWords::structured(
                    InvocationWord::Literal("literal-form-fixture"),
                    &dynamic,
                ),
                None,
            )
            .resolved()
            .expect("fixture command resolves");
        assert!(
            resolved.form.is_none(),
            "a dynamic word that could extend a completed selector must abstain"
        );
    }

    #[test]
    fn unresolved_subcommands_never_fall_back_to_a_top_level_form() {
        const TOP_LEVEL_FORM: CommandForm = CommandForm {
            name: "top-level-form",
            arity: Arity::exact(1),
            ..CommandForm::DEFAULT
        };
        const SUBCOMMANDS: &[SubCommand] = &[
            SubCommand {
                name: "alpha",
                ..SubCommand::DEFAULT
            },
            SubCommand {
                name: "alpine",
                ..SubCommand::DEFAULT
            },
        ];
        const SPEC: CommandSpec = CommandSpec {
            name: "subcommand-form-fixture",
            arity: Arity::any(),
            subcommands: SUBCOMMANDS,
            command_forms: &[TOP_LEVEL_FORM],
            ..CommandSpec::DEFAULT
        };

        let mut reg = CommandRegistry::build_default();
        reg.insert(SPEC);
        let dynamic_arguments = [crate::InvocationWord::Dynamic];
        let dynamic = reg
            .resolve_structured_invocation(
                InvocationWords::structured(
                    crate::InvocationWord::Literal("subcommand-form-fixture"),
                    &dynamic_arguments,
                ),
                None,
            )
            .resolved()
            .expect("fixture command resolves");
        assert!(matches!(
            dynamic.subcommand,
            SubcommandResolution::Indeterminate { .. }
        ));
        assert!(dynamic.form.is_none());
        assert!(matches!(
            dynamic.facts().subcommand,
            crate::OwnedSubcommandResolution::Indeterminate {
                word_kind: crate::InvocationWordKind::Dynamic
            }
        ));

        let ambiguous = reg
            .resolve_invocation("subcommand-form-fixture", &["al"], None)
            .expect("fixture command resolves");
        assert!(matches!(
            ambiguous.subcommand,
            SubcommandResolution::Ambiguous { .. }
        ));
        assert!(ambiguous.form.is_none());
        assert!(matches!(
            ambiguous.facts().subcommand,
            crate::OwnedSubcommandResolution::Ambiguous { ref spelling } if spelling == "al"
        ));

        let unknown = reg
            .resolve_invocation("subcommand-form-fixture", &["missing"], None)
            .expect("fixture command resolves");
        assert!(matches!(
            unknown.subcommand,
            SubcommandResolution::Unknown { .. }
        ));
        assert!(unknown.form.is_none());
        assert!(matches!(
            unknown.facts().subcommand,
            crate::OwnedSubcommandResolution::Unknown { ref spelling } if spelling == "missing"
        ));
    }

    #[test]
    fn resolve_invocation_applies_registry_unique_prefix_subcommand_resolution() {
        let reg = CommandRegistry::build_default();
        let args = ["le", "hello"];
        let resolved = reg
            .resolve_invocation(
                "string",
                &args,
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("string is registry-known");

        let sub = resolved
            .subcommand
            .resolved()
            .expect("string le is a valid unique prefix");
        assert_eq!(sub.spelling, "le");
        assert_eq!(sub.canonical_name, "length");
        assert_eq!(
            resolved.subcommand.kind(),
            Some(crate::SubcommandResolutionKind::UniquePrefix)
        );
        assert_eq!(
            resolved.semantics.return_type,
            Some(crate::types::TclType::Int)
        );
    }

    #[test]
    fn resolve_invocation_preserves_an_ambiguous_subcommand_outcome() {
        let reg = CommandRegistry::build_default();
        let args = ["t", "hello"];
        let resolved = reg
            .resolve_invocation(
                "string",
                &args,
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("the command head remains known");

        assert!(matches!(
            resolved.subcommand,
            SubcommandResolution::Ambiguous { spelling: "t" }
        ));
        assert_eq!(
            resolved.subcommand.kind(),
            Some(crate::SubcommandResolutionKind::Ambiguous)
        );
        assert!(resolved.form.is_none());
    }

    #[test]
    fn resolve_invocation_preserves_an_unknown_subcommand_outcome() {
        let reg = CommandRegistry::build_default();
        let args = ["does-not-exist"];
        let resolved = reg
            .resolve_invocation(
                "string",
                &args,
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("the command head remains known");

        assert!(matches!(
            resolved.subcommand,
            SubcommandResolution::Unknown {
                spelling: "does-not-exist"
            }
        ));
        assert_eq!(
            resolved.subcommand.kind(),
            Some(crate::SubcommandResolutionKind::Unknown)
        );
    }

    #[test]
    fn resolve_invocation_projects_forms_without_backend_hooks() {
        let reg = CommandRegistry::build_default();
        let args = ["counter"];
        let resolved = reg
            .resolve_invocation("incr", &args, None)
            .expect("incr is registry-known");

        let form = resolved.form.expect("implicit incr form");
        assert_eq!(form.name, "implicit");
        assert_eq!(form.arity, Arity::exact(1));
        assert_eq!(form.arg_roles, &[(0, ArgRole::VarWrite)]);
        assert_eq!(resolved.semantics.arg_roles, form.arg_roles);
        assert_eq!(
            resolved.semantics.return_type,
            Some(crate::types::TclType::Int)
        );
        assert_eq!(
            resolved.semantics.lowering_hook,
            Some(LoweringHookId::Incr),
            "the common lowering descriptor survives the target-neutral projection"
        );
        assert_eq!(
            resolved.semantics.operation,
            crate::SemanticOperationId::StructuredLowering(LoweringHookId::Incr)
        );
    }

    #[test]
    fn resolve_invocation_identifies_channel_write_without_selecting_a_backend() {
        let reg = CommandRegistry::build_default();
        let args = ["hello"];
        let resolved = reg
            .resolve_invocation("puts", &args, Some(SurfaceQuery::core(Family::Tcl, "8.6")))
            .expect("puts resolves");

        assert_eq!(
            resolved.semantics.operation,
            crate::SemanticOperationId::Intrinsic(crate::IntrinsicId::ChannelWrite)
        );
        assert_eq!(
            reg.command_names_for_semantic_operation(crate::SemanticOperationId::Intrinsic(
                crate::IntrinsicId::ChannelWrite,
            ))
            .collect::<Vec<_>>(),
            vec!["puts"]
        );
    }

    #[test]
    fn semantic_operation_resolution_prefers_form_then_subcommand_then_command() {
        const FORM: CommandForm = CommandForm {
            name: "form",
            arity: Arity::exact(1),
            semantic_operation: Some(crate::SemanticOperationId::StructuredLowering(
                LoweringHookId::Set,
            )),
            ..CommandForm::DEFAULT
        };
        const SUB: SubCommand = SubCommand {
            name: "sub",
            arity: Arity::at_least(0),
            subcommand_forms: &[FORM],
            semantic_operation: Some(crate::SemanticOperationId::StructuredLowering(
                LoweringHookId::Incr,
            )),
            ..SubCommand::DEFAULT
        };
        const SPEC: CommandSpec = CommandSpec {
            name: "semantic-operation-precedence",
            arity: Arity::at_least(0),
            subcommands: &[SUB],
            semantic_operation: Some(crate::SemanticOperationId::StructuredLowering(
                LoweringHookId::Return,
            )),
            ..CommandSpec::DEFAULT
        };

        let mut reg = CommandRegistry::build_default();
        reg.insert(SPEC);

        let command = reg
            .resolve_invocation("semantic-operation-precedence", &[], None)
            .expect("command form resolves");
        assert_eq!(
            command.semantics.operation,
            crate::SemanticOperationId::StructuredLowering(LoweringHookId::Return)
        );

        let sub = reg
            .resolve_invocation("semantic-operation-precedence", &["sub"], None)
            .expect("subcommand form resolves");
        assert_eq!(
            sub.semantics.operation,
            crate::SemanticOperationId::StructuredLowering(LoweringHookId::Incr)
        );

        let form = reg
            .resolve_invocation("semantic-operation-precedence", &["sub", "argument"], None)
            .expect("subcommand form resolves");
        assert_eq!(
            form.semantics.operation,
            crate::SemanticOperationId::StructuredLowering(LoweringHookId::Set)
        );
    }

    #[test]
    fn resolve_invocation_exposes_subcommand_effects_through_one_semantic_view() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();
        let args = ["insert", "x-demo", "value"];
        let resolved = reg
            .resolve_invocation(
                "HTTP::header",
                &args,
                Some(SurfaceQuery::any_release(Family::F5Irules)),
            )
            .expect("HTTP::header insert resolves in the iRules dialect");

        assert!(resolved.semantics.traits.contains(Traits::PURE));
        assert_eq!(resolved.semantics.side_effects.len(), 1);
        let effect = resolved.semantics.side_effects[0];
        assert_eq!(
            effect.target,
            crate::side_effects::SideEffectTarget::HttpHeader
        );
        assert!(effect.reads);
        assert!(effect.writes);
    }

    #[test]
    fn resolved_call_and_resolved_invocation_share_selection() {
        let reg = CommandRegistry::build_default();
        let args = ["counter", "5"];
        let common = reg
            .resolve_invocation("incr", &args, None)
            .expect("incr resolves");
        let legacy = reg
            .resolve_call("incr", &args, None)
            .expect("legacy compatibility resolver remains available");

        assert_eq!(common.canonical_command, legacy.spec.name);
        assert_eq!(
            common.form.map(|form| form.name),
            legacy.form.map(|form| form.name)
        );
        assert_eq!(common.semantics.lowering_hook, legacy.lowering_hook);
        assert_eq!(common.semantics.arity, legacy.arity());
    }

    #[test]
    fn resolve_call_top_level_command() {
        let reg = CommandRegistry::build_default();
        let resolved = reg.resolve_call("set", &["x", "1"], None).unwrap();
        assert_eq!(resolved.spec.name, "set");
        assert!(resolved.sub.is_none());
    }

    #[test]
    fn resolve_call_subcommand() {
        let reg = CommandRegistry::build_default();
        let resolved = reg
            .resolve_call(
                "dict",
                &["create", "k", "v"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .unwrap();
        assert_eq!(resolved.spec.name, "dict");
        let sub = resolved.sub.expect("dict create resolves to a subcommand");
        assert_eq!(sub.name, "create");
    }

    #[test]
    fn resolve_call_dialect_filter_blocks_tcl84_dict() {
        let reg = CommandRegistry::build_default();
        // dict is tcl8.5+; resolving against tcl8.4 must fail.
        assert!(
            reg.resolve_call(
                "dict",
                &["create"],
                Some(SurfaceQuery::core(Family::Tcl, "8.4"))
            )
            .is_none()
        );
    }

    #[test]
    fn resolve_call_picks_arity_matched_command_form_for_incr() {
        let reg = CommandRegistry::build_default();
        // `incr counter` — arity 1 → matches the implicit form.
        let r1 = reg.resolve_call("incr", &["counter"], None).unwrap();
        let f1 = r1.form.expect("incr should match a CommandForm");
        assert_eq!(f1.name, "implicit");
        assert_eq!(f1.arity, crate::arity::Arity::exact(1));

        // `incr counter 5` — arity 2 → matches the explicit form.
        let r2 = reg.resolve_call("incr", &["counter", "5"], None).unwrap();
        let f2 = r2.form.expect("incr counter 5 should match a CommandForm");
        assert_eq!(f2.name, "explicit");
        assert_eq!(f2.arity, crate::arity::Arity::exact(2));
    }

    #[test]
    fn resolve_call_picks_arity_matched_command_form_for_lset() {
        let reg = CommandRegistry::build_default();
        // `lset lst value` — arity 2 → replace form.
        let replace = reg
            .resolve_call(
                "lset",
                &["lst", "value"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .unwrap();
        assert_eq!(replace.form.unwrap().name, "replace");

        // `lset lst 0 value` — arity 3 → single_index form.
        let single = reg
            .resolve_call(
                "lset",
                &["lst", "0", "value"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .unwrap();
        assert_eq!(single.form.unwrap().name, "single_index");

        // `lset lst 0 1 2 value` — arity 5 → flat_path form.
        let flat = reg
            .resolve_call(
                "lset",
                &["lst", "0", "1", "2", "value"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .unwrap();
        assert_eq!(flat.form.unwrap().name, "flat_path");
    }

    // -- ``resolve_option_terminator`` (W304 driver)
    //
    // Exercises option-terminator resolution for each command.
    // Each W304 fixture
    // is rooted in one of these resolver outcomes; the resolver tests
    // here pin the per-command shape, the analyser tests pin the
    // tristate-severity / two-diagnostic / code-fix behaviour.

    #[test]
    fn resolve_option_terminator_returns_none_for_unknown_command() {
        let reg = CommandRegistry::build_default();
        assert!(
            reg.resolve_option_terminator("unknownthing", &[], None)
                .is_none()
        );
    }

    #[test]
    fn resolve_option_terminator_returns_none_for_command_without_terminator() {
        let reg = CommandRegistry::build_default();
        // ``set`` does not declare a ``--`` terminator option.
        assert!(
            reg.resolve_option_terminator("set", &["x", "1"], None)
                .is_none()
        );
    }

    #[test]
    fn resolve_option_terminator_form_level_for_regexp() {
        let reg = CommandRegistry::build_default();
        let profile = reg
            .resolve_option_terminator("regexp", &[], None)
            .expect("regexp declares -- at the form level");
        assert_eq!(profile.scan_start, 0);
        assert!(profile.subcommand.is_none());
        // ``-start`` takes a value; ``-nocase`` does not.
        // ``-start`` takes a value; ``-nocase`` does not.  The
        // resolver returns the borrowed options slice; callers
        // filter via ``OptionSpec::takes_value``.
        assert!(
            profile
                .options
                .iter()
                .any(|o| o.name == "-start" && o.takes_value())
        );
        assert!(
            profile
                .options
                .iter()
                .any(|o| o.name == "-nocase" && !o.takes_value())
        );
    }

    #[test]
    fn resolve_option_terminator_subcommand_scoped_for_file_delete() {
        let reg = CommandRegistry::build_default();
        let profile = reg
            .resolve_option_terminator("file", &["delete", "$path"], None)
            .expect("file delete declares -- at the subcommand level");
        assert_eq!(profile.scan_start, 1);
        assert_eq!(profile.subcommand, Some("delete"));
    }

    #[test]
    fn resolve_option_terminator_subcommand_without_terminator_returns_none() {
        let reg = CommandRegistry::build_default();
        // ``file mtime`` has no ``--`` terminator.
        let profile = reg.resolve_option_terminator("file", &["mtime", "$p"], None);
        assert!(profile.is_none(), "got {profile:?}");
    }

    // -- ``is_canonical_list_command`` (W101 safe-idiom driver)

    #[test]
    fn is_canonical_list_command_includes_list_and_split_excludes_concat() {
        let reg = CommandRegistry::build_default();
        assert!(reg.is_canonical_list_command("list"));
        assert!(reg.is_canonical_list_command("linsert"));
        assert!(reg.is_canonical_list_command("split"));
        assert!(reg.is_canonical_list_command("lreverse"));
        // ``concat`` returns LIST but is the explicit non-canonical
        // exclusion.
        assert!(!reg.is_canonical_list_command("concat"));
        // Non-list commands (e.g. ``set``) are filtered out.
        assert!(!reg.is_canonical_list_command("set"));
        // Unknown commands return false.
        assert!(!reg.is_canonical_list_command("unknownthing"));
    }

    #[test]
    fn is_canonical_list_command_handles_compound_subcommand_keys() {
        let reg = CommandRegistry::build_default();
        // ``dict keys`` returns LIST.
        assert!(reg.is_canonical_list_command("dict keys"));
        // ``dict get`` returns String (or unspecified) — not canonical.
        assert!(!reg.is_canonical_list_command("dict froob"));
    }

    #[test]
    fn irules_sink_commands_carry_structural_options() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();

        let respond = reg.get("HTTP::respond").expect("HTTP::respond loaded");
        let opts: Vec<&str> = respond.options.iter().map(|o| o.name).collect();
        // Option set follows the reference standard:
        // -version/-content/-ifile/
        // -noserver/-reset. (`-status` is the positional status arg.)
        assert!(
            opts.contains(&"-version")
                && opts.contains(&"-content")
                && opts.contains(&"-noserver")
                && opts.contains(&"-reset"),
            "HTTP::respond options {opts:?} should include -version / -content / -noserver / -reset",
        );
        let noserver = respond
            .options
            .iter()
            .find(|o| o.name == "-noserver")
            .unwrap();
        assert!(!noserver.takes_value());
        let version = respond
            .options
            .iter()
            .find(|o| o.name == "-version")
            .unwrap();
        assert!(version.takes_value());

        let header = reg.get("HTTP::header").expect("HTTP::header loaded");
        let header_opts: Vec<&str> = header.options.iter().map(|o| o.name).collect();
        assert!(
            header_opts.contains(&"-noupdate"),
            "HTTP::header options {header_opts:?} should include -noupdate",
        );
    }

    #[test]
    fn xc_translatability_helpers_read_spec_flags() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();

        // `xc_translatable: Some(false)` → never translatable (consumed by the
        // `f5-xc` translator's XC300 branch).
        assert!(reg.is_xc_never_translatable("eval"));
        assert!(!reg.is_xc_translatable_override("eval"));

        // `xc_translatable: Some(true)` → translatable override despite an
        // otherwise-untranslatable namespace prefix (e.g. `IP::`, `ASM::`).
        assert!(reg.is_xc_translatable_override("IP::client_addr"));
        assert!(!reg.is_xc_never_translatable("IP::client_addr"));

        // Commands with no `xc_translatable` flag report neither.
        assert!(!reg.is_xc_never_translatable("set"));
        assert!(!reg.is_xc_translatable_override("set"));
        // An unknown command name is safely neither.
        assert!(!reg.is_xc_never_translatable("no_such_command_xyz"));
        assert!(!reg.is_xc_translatable_override("no_such_command_xyz"));
    }

    /// `unit_linkage` composes `spec.traits | sub.traits` and filters to the
    /// linkage union, so the answer is subcommand-precise: `package provide`
    /// publishes an API surface, `package require` pulls another unit in, and
    /// `package names` does neither.
    #[test]
    fn unit_linkage_is_subcommand_precise() {
        let reg = CommandRegistry::build_default();
        let empty = None;
        assert_eq!(
            reg.unit_linkage("package", &["provide", "mylib", "1.0"], empty),
            Traits::PROVIDES_PACKAGE
        );
        assert_eq!(
            reg.unit_linkage("package", &["ifneeded", "mylib", "1.0", "body"], empty),
            Traits::PROVIDES_PACKAGE
        );
        assert_eq!(
            reg.unit_linkage("package", &["require", "mylib"], empty),
            Traits::LOADS_EXTERNAL_UNIT
        );
        assert_eq!(
            reg.unit_linkage("package", &["names"], empty),
            Traits::empty()
        );
    }

    /// `invocation_traits` composes `spec.traits | sub.traits` for the
    /// concrete call, which is the only way the eval-family bits on the
    /// compound members are visible: `namespace eval` / `namespace inscope` /
    /// `interp eval` carry `EVALUATES_CODE` on the **subcommand**, so a
    /// parent-only `get(name).traits` test misses them.
    #[test]
    fn invocation_traits_compose_subcommand_traits() {
        let reg = CommandRegistry::build_default();
        let empty = None;
        // TP — the eval family, bare and compound.
        for (name, args) in [
            ("eval", &["{set x 1}"][..]),
            ("uplevel", &["1", "{set x 1}"][..]),
            ("namespace", &["eval", "ns", "{set x 1}"][..]),
            ("namespace", &["inscope", "ns", "{set x 1}"][..]),
            ("interp", &["eval", "slave", "{set x 1}"][..]),
        ] {
            let traits = reg.invocation_traits(name, args, empty);
            assert!(
                traits.contains(Traits::EVALUATES_CODE),
                "{name} {args:?} must carry EVALUATES_CODE"
            );
            assert!(
                traits.contains(Traits::SCRIPT_CONCATENATES_ARGS),
                "{name} {args:?} must carry SCRIPT_CONCATENATES_ARGS"
            );
        }
        // TN — the parent spec alone carries neither bit, which is exactly
        // why composing is required.
        let parent = reg.get("namespace").expect("namespace spec").traits;
        assert!(!parent.contains(Traits::EVALUATES_CODE));
        // TN — a sibling subcommand that evaluates nothing stays clean.
        assert!(
            !reg.invocation_traits("namespace", &["delete", "ns"], empty)
                .contains(Traits::EVALUATES_CODE)
        );
        // TN — an unknown command carries no traits at all.
        assert_eq!(
            reg.invocation_traits("no_such_command_xyz", &["a"], empty),
            Traits::empty()
        );
        // The parent's own traits still compose in — a subcommand's traits
        // are additive, not a replacement.
        assert!(
            reg.invocation_traits("namespace", &["eval", "ns", "{}"], empty)
                .contains(Traits::LANGUAGE_KEYWORD),
            "the parent `namespace` spec's own bits survive composition"
        );
    }

    #[test]
    fn typed_control_arms_are_registry_owned() {
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.control_arm_semantics("try", &["{}", "finally", "{}"], 2),
            Some(ControlArmSemantics::Always)
        );
        assert_eq!(
            reg.control_arm_semantics("try", &["{}", "on", "error", "{m o}", "{}"], 4,),
            Some(ControlArmSemantics::Selected)
        );
        assert_eq!(
            reg.try_control_invocation("try", &["{}", "on", "error", "{m o}", "{}"], None,),
            Some(TryControlInvocation {
                body_index: 0,
                clauses: vec![TryControlClause {
                    kind: TryClauseKind::On(TryCompletionSelector::Error),
                    selector_index: Some(2),
                    variable_list_index: Some(3),
                    body_index: 4,
                    fallthrough: false,
                }],
            })
        );
        assert_eq!(
            reg.control_arm_semantics("namespace", &["eval", "::ns", "{}"], 2),
            Some(ControlArmSemantics::FrameBoundary)
        );
        assert_eq!(
            reg.control_arm_semantics("try", &["{}", "finally", "{}", "orphan"], 0),
            None,
            "a malformed trailing clause invalidates even the main arm"
        );
        for args in [
            &["{}", "on", "bogus", "{}", "{}"][..],
            &["{}", "on", "ok", "a b c", "{}"][..],
            &["{}", "on", "ok", "{}", "-"][..],
        ] {
            assert_eq!(reg.control_arm_semantics("try", args, 0), None);
        }
        assert_eq!(
            reg.control_invocation_valid("try", &["{}", "on", "ok", "{}", "-"], None,),
            Some(false),
            "a terminal handler fallthrough has no following script"
        );
        assert_eq!(
            reg.control_arm_semantics("try", &["{}", "finally", "-"], 2),
            Some(ControlArmSemantics::Always),
            "a finally body is a script, so `-` has no fallthrough meaning there"
        );
        let chained_fallthrough = [
            "{}",
            "on",
            "error",
            "{}",
            "-",
            "on",
            "error",
            "{}",
            "{set r fell}",
        ];
        assert_eq!(
            reg.control_invocation_valid("try", &chained_fallthrough, None),
            Some(true)
        );
        assert_eq!(
            reg.control_arm_semantics("try", &chained_fallthrough, 8),
            Some(ControlArmSemantics::Selected)
        );
        assert_eq!(
            reg.control_invocation_valid("if", &["1", "then"], None),
            Some(false)
        );
        assert_eq!(
            reg.control_invocation_valid("if", &["1", "a", "b"], None),
            Some(true)
        );
        assert_eq!(
            reg.control_arm_semantics("if", &["1", "a", "b"], 2),
            Some(ControlArmSemantics::Selected)
        );
    }

    #[test]
    fn try_completion_selectors_use_the_profiled_tcl_number_and_completion_owners() {
        // `try_control_invocation` receives resolved word values. In source,
        // `{ 1 }` becomes the `" 1 "` value below; a literal value of `{1}`
        // would instead come from nested braces and is not a completion code.
        let selector = |registry: &CommandRegistry, selector: &str| {
            let invocation = registry.try_control_invocation(
                "try",
                &["{body}", "on", selector, "{message}", "{handler}"],
                None,
            )?;
            let [clause] = invocation.clauses.as_slice() else {
                return None;
            };
            match clause.kind {
                TryClauseKind::On(selector) => Some(selector),
                TryClauseKind::Trap | TryClauseKind::Finally => None,
            }
        };

        let tcl9 = crate::model::ingress::static_context_for("tcl9.0").commands();
        for spelling in ["+1", "01", "0x1", " 1 "] {
            assert_eq!(
                selector(tcl9, spelling),
                Some(TryCompletionSelector::Error),
                "Tcl 9 completion selector {spelling:?}"
            );
        }
        assert_eq!(
            selector(tcl9, "{1}"),
            None,
            "the registry must not mistake nested literal braces for source-word delimiters"
        );

        // The number owner selects grammar from the registry's target
        // profile. A leading-zero spelling is octal through Tcl 8.6 and
        // decimal from Tcl 9.0.
        let tcl8 = crate::model::ingress::static_context_for("tcl8.6").commands();
        assert_eq!(
            selector(tcl8, "010"),
            Some(TryCompletionSelector::Numeric(8))
        );
        assert_eq!(
            selector(tcl9, "010"),
            Some(TryCompletionSelector::Numeric(10))
        );

        // Completion codes use Tcl's C-compatible signed-int domain: the
        // final unsigned range wraps, while values outside it are invalid.
        assert_eq!(
            selector(tcl9, "4294967295"),
            Some(TryCompletionSelector::Numeric(-1))
        );
        for out_of_range in ["-2147483649", "4294967296", "9223372036854775808"] {
            assert_eq!(
                selector(tcl9, out_of_range),
                None,
                "out-of-range completion selector {out_of_range:?}"
            );
        }
    }

    #[test]
    fn explicit_control_surface_selects_completion_code_numeral_grammar() {
        let registry = CommandRegistry::build_default();
        let args = ["{body}", "on", "1_0", "{message}", "{handler}"];
        let tcl86 = SurfaceQuery::core(Family::Tcl, "8.6");
        let tcl90 = SurfaceQuery::core(Family::Tcl, "9.0");

        assert_eq!(
            registry.control_invocation_valid("try", &args, Some(tcl86)),
            Some(false),
            "Tcl 8.6 does not admit underscore-separated completion codes"
        );
        assert_eq!(
            registry.try_control_invocation("try", &args, Some(tcl86)),
            None
        );
        assert_eq!(
            registry.control_invocation_valid("try", &args, Some(tcl90)),
            Some(true),
            "Tcl 9 admits underscore-separated completion codes"
        );
        let invocation = registry
            .try_control_invocation("try", &args, Some(tcl90))
            .expect("Tcl 9 try invocation");
        assert!(matches!(
            invocation.clauses.as_slice(),
            [TryControlClause {
                kind: TryClauseKind::On(TryCompletionSelector::Numeric(10)),
                ..
            }]
        ));
    }

    #[test]
    fn invocation_completion_is_registry_owned() {
        let reg = crate::model::ingress::static_context_for("tcl9.0").commands();
        assert_eq!(
            reg.invocation_completion("return", &["-code", "error", "$w"], None,),
            InvocationCompletion::Terminates
        );
        assert_eq!(
            reg.invocation_completion("return", &["$w"], None),
            InvocationCompletion::ReturnsResult(Some(0))
        );
        assert_eq!(
            reg.invocation_completion("return", &["-code"], None),
            InvocationCompletion::ReturnsResult(Some(0)),
            "a trailing option-shaped word is return's result, not a missing option value"
        );
        assert_eq!(
            reg.invocation_completion("return", &["--", "x"], None),
            InvocationCompletion::ReturnsResult(None),
            "`--` is no end of options: `return -- x` is the pair `-- x` and the empty result"
        );
        assert_eq!(
            reg.invocation_completion("return", &["-level", "0", "$w"], None,),
            InvocationCompletion::FallsThrough
        );
        for args in [
            &["-level", "0", "-code", "error", "$w"][..],
            &["-code", "error", "-level", "0", "$w"][..],
            &["-level", "2", "$w"][..],
            &["-level", "-1", "$w"][..],
        ] {
            assert_eq!(
                reg.invocation_completion("return", args, None),
                InvocationCompletion::Terminates,
                "{args:?}"
            );
        }
        assert_eq!(
            reg.invocation_completion("return", &["-level", "$dynamic", "$w"], None),
            InvocationCompletion::Unknown,
        );
        for invalid in [&["-level", "-1", "value"][..]] {
            assert_eq!(
                reg.invocation_completion("return", invalid, None),
                InvocationCompletion::Terminates,
                "native validation error ends the current path"
            );
        }
        // Native worker383 ordinary_pair: C85–C91 catch=2/result empty;
        // C84 catch=1. Custom option pairs are valid in this selected C9 context.
        assert_eq!(
            reg.invocation_completion("return", &["value", "extra"], None),
            InvocationCompletion::ReturnsResult(None)
        );
        // A registry that names no release answers only what every release
        // reads alike: 8.4 rejects `-level`, which 8.5 reads.
        assert_eq!(
            CommandRegistry::build_default().invocation_completion(
                "return",
                &["-level", "0", "$w"],
                None
            ),
            InvocationCompletion::Unknown
        );
        assert_eq!(
            reg.invocation_completion("not-a-command", &[], None),
            InvocationCompletion::Unknown
        );
        assert_eq!(
            reg.invocation_completion("set", &[], None),
            InvocationCompletion::Terminates
        );
    }

    #[test]
    fn source_completion_preserves_dynamic_options_and_pending_return_depth() {
        use crate::InvocationWord::{Dynamic, Literal};
        let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
        let result = [Literal("-level"), Literal("1"), Dynamic];
        assert_eq!(
            registry.invocation_completion_words(
                "return",
                InvocationArguments::structured(&result),
                None
            ),
            InvocationCompletion::ReturnsResult(Some(2))
        );
        let deeper = [Literal("-level"), Literal("2"), Dynamic];
        assert_eq!(
            registry.invocation_completion_words(
                "return",
                InvocationArguments::structured(&deeper),
                None
            ),
            InvocationCompletion::Terminates,
            "pending return does not become a normal result at this procedure boundary"
        );
        let option = [Literal("-code"), Dynamic, Dynamic];
        assert_eq!(
            registry.invocation_completion_words(
                "return",
                InvocationArguments::structured(&option),
                None
            ),
            InvocationCompletion::Unknown
        );
        assert_eq!(
            CommandRegistry::build_default().invocation_completion(
                "return",
                &["-level", "0", "value"],
                None
            ),
            InvocationCompletion::Unknown,
            "an unselected native grammar cannot license modern return options"
        );
    }

    #[test]
    fn exact_return_options_and_process_exit_are_registry_owned() {
        use crate::completion::CompletionCode;

        let reg = crate::model::ingress::static_context_for("tcl8.6").commands();
        // Oracle (tclsh 8.6/9.0): the default -level 1 return is caught by
        // `try on return`, even when its eventual procedure result is error.
        assert_eq!(
            reg.exact_invocation_completion("return", &["-code", "error", "payload"], None,),
            Some(ExactInvocationCompletion::Tcl(CompletionCode::Return))
        );
        assert_eq!(
            reg.exact_invocation_completion(
                "return",
                &["-level", "0", "-code", "error", "payload"],
                None,
            ),
            Some(ExactInvocationCompletion::Tcl(CompletionCode::Error))
        );
        for args in [
            &["-foo", "bar", "payload"][..],
            &["-c", "error", "payload"][..],
            &["-level", "0x0", "-code", "error", "payload"][..],
        ] {
            assert_eq!(
                reg.exact_invocation_completion("return", args, None),
                Some(ExactInvocationCompletion::Tcl(if args[0] == "-level" {
                    CompletionCode::Error
                } else {
                    CompletionCode::Return
                }))
            );
        }
        for (spelling, expected) in [("2147483648", i32::MIN), ("4294967295", -1)] {
            assert_eq!(
                reg.exact_invocation_completion(
                    "return",
                    &["-level", "0", "-code", spelling, "payload"],
                    None,
                ),
                Some(ExactInvocationCompletion::Tcl(CompletionCode::Other(
                    expected
                )))
            );
        }
        for (spelling, expected) in [
            ("00", CompletionCode::Ok),
            ("+1", CompletionCode::Error),
            ("02", CompletionCode::Return),
            ("0x3", CompletionCode::Break),
            ("04", CompletionCode::Continue),
        ] {
            assert_eq!(
                reg.exact_invocation_completion(
                    "return",
                    &["-level", "0", "-code", spelling, "payload"],
                    None,
                ),
                Some(ExactInvocationCompletion::Tcl(expected)),
                "{spelling}"
            );
        }
        assert_eq!(
            reg.exact_invocation_completion("exit", &["0"], None),
            Some(ExactInvocationCompletion::ProcessExit)
        );
        assert_eq!(
            reg.exact_invocation_completion("return", &["-options", "$dynamic", "payload"], None,),
            None
        );
        assert_eq!(
            reg.exact_invocation_completion("return", &["-code", "$dynamic", "payload"], None,),
            None
        );
        assert_eq!(
            reg.exact_invocation_completion("return", &["--", "-code", "error"], None,),
            Some(ExactInvocationCompletion::Tcl(CompletionCode::Return))
        );
    }

    #[test]
    fn exit_status_completion_follows_the_release_specific_tcl_oracle() {
        use crate::completion::CompletionCode;
        use crate::invocation_words::{InvocationArguments, InvocationWord};

        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6"] {
            let reg = crate::model::ingress::static_context_for(dialect).commands();
            // Tcl 8.x's `exit` calls `Tcl_GetIntFromObj`. Despite the `int`
            // destination, its 64-bit implementation accepts the asymmetric
            // `-UINT_MAX..=UINT_MAX` range before casting to `int`.
            for args in [
                &[][..],
                &["0"][..],
                &["-7"][..],
                &["0x10"][..],
                &["-4294967295"][..],
                &["4294967295"][..],
            ] {
                assert_eq!(
                    reg.exact_invocation_completion("exit", args, None),
                    Some(ExactInvocationCompletion::ProcessExit),
                    "{dialect}: exit {args:?}"
                );
            }
            // One value beyond either Tcl 8.x conversion endpoint never gets
            // as far as process termination: it is a catchable Tcl error.
            for args in [
                &["nope"][..],
                &["1.5"][..],
                &["-4294967296"][..],
                &["4294967296"][..],
            ] {
                assert_eq!(
                    reg.exact_invocation_completion("exit", args, None),
                    Some(ExactInvocationCompletion::Tcl(CompletionCode::Error)),
                    "{dialect}: exit {args:?}"
                );
            }
        }

        for dialect in ["tcl9.0", "tcl9.1"] {
            let reg = crate::model::ingress::static_context_for(dialect).commands();
            // Tcl 9.0+ changed `exit` to `TclGetWideBitsFromObj`: every
            // integer, including a bignum beyond wide range, reaches
            // `Tcl_Exit` and has its low bits cast to `int` there.
            for args in [
                &[][..],
                &["0"][..],
                &["-4294967296"][..],
                &["4294967296"][..],
                &["18446744073709551616"][..],
            ] {
                assert_eq!(
                    reg.exact_invocation_completion("exit", args, None),
                    Some(ExactInvocationCompletion::ProcessExit),
                    "{dialect}: exit {args:?}"
                );
            }
            for args in [&["nope"][..], &["1.5"][..], &["0", "extra"][..]] {
                assert_eq!(
                    reg.exact_invocation_completion("exit", args, None),
                    Some(ExactInvocationCompletion::Tcl(CompletionCode::Error)),
                    "{dialect}: exit {args:?}"
                );
            }
        }

        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let reg = crate::model::ingress::static_context_for(dialect).commands();
            let dynamic = [InvocationWord::Dynamic];
            assert_eq!(
                reg.invocation_completion_knowledge(
                    "exit",
                    InvocationArguments::structured(&dynamic),
                    None,
                ),
                Some(InvocationCompletionKnowledge::ExitOrError),
                "{dialect}: exit $status is only process-exit or Tcl error"
            );
            let expanded = [InvocationWord::Expanded];
            assert_eq!(
                reg.invocation_completion_knowledge(
                    "exit",
                    InvocationArguments::structured(&expanded),
                    None,
                ),
                Some(InvocationCompletionKnowledge::ExitOrError),
                "{dialect}: exit {{*}}$statuses has no normal completion either"
            );
        }
    }

    #[test]
    fn exact_return_option_shapes_follow_the_tcl86_and_tcl90_oracle() {
        use crate::completion::CompletionCode;

        // Tcl parses an option only when a following argv word can supply its
        // value. A lone option-shaped word is therefore return's sole result;
        // with `-level 0`, it exposes the default TCL_OK instead.
        for dialect in ["tcl8.6", "tcl9.0"] {
            let reg = crate::model::ingress::static_context_for(dialect).commands();
            for (args, expected) in [
                (&["-code"][..], CompletionCode::Return),
                (&["-level"][..], CompletionCode::Return),
                (&["-options"][..], CompletionCode::Return),
                (&["-foo"][..], CompletionCode::Return),
                (&["-level", "0", "-code"][..], CompletionCode::Ok),
            ] {
                assert_eq!(
                    reg.exact_invocation_completion("return", args, None),
                    Some(ExactInvocationCompletion::Tcl(expected)),
                    "{dialect}: {args:?}"
                );
            }
            // Once a following word exists, the same spellings are options;
            // malformed option values remain TCL_ERROR.
            for args in [
                &["-code", "bogus", "payload"][..],
                &["-level", "-1", "payload"][..],
                &["-code", "-level"][..],
                &["-level", "-code"][..],
            ] {
                assert_eq!(
                    reg.exact_invocation_completion("return", args, None),
                    Some(ExactInvocationCompletion::Tcl(CompletionCode::Error)),
                    "{dialect}: {args:?}"
                );
            }
            // Native worker383 confirms ordinary_pair catch=2/result empty
            // for these exact C86/C90 workers. Two words are an option and its value, whatever the first
            // spells, which the options dictionary keeps: `proc p {} {return
            // payload extra}` returns the empty string (tclsh 8.6, 9.0).
            assert_eq!(
                reg.exact_invocation_completion("return", &["payload", "extra"], None),
                Some(ExactInvocationCompletion::Tcl(CompletionCode::Return)),
                "{dialect}"
            );
            // Past `INT_MIN` the conversion's range is the release's own:
            // 8.6 wraps `-2147483649` to 2147483647 and 9.0 rejects it.
            assert_eq!(
                reg.exact_invocation_completion(
                    "return",
                    &["-level", "0", "-code", "-2147483649", "payload"],
                    None,
                ),
                None,
                "{dialect}"
            );
        }
    }

    #[test]
    fn exact_invalid_arity_of_known_terminators_is_tcl_error() {
        use crate::completion::CompletionCode;

        // The descriptor of a well-formed call must never hide Tcl's prior
        // arity validation.  This table covers every fixed-arity terminating
        // core primitive, including ones whose normal effect is process exit
        // or a loop transfer rather than TCL_ERROR.
        for dialect in ["tcl8.6", "tcl9.0"] {
            let reg = crate::model::ingress::static_context_for(dialect).commands();
            for (name, args) in [
                ("error", &[][..]),
                ("error", &["message", "info", "code", "extra"][..]),
                ("throw", &[][..]),
                ("throw", &["type"][..]),
                ("throw", &["type", "message", "extra"][..]),
                ("break", &["extra"][..]),
                ("continue", &["extra"][..]),
                ("exit", &["0", "extra"][..]),
            ] {
                assert_eq!(
                    reg.exact_invocation_completion(name, args, None),
                    Some(ExactInvocationCompletion::Tcl(CompletionCode::Error)),
                    "{dialect}: {name} {args:?}"
                );
            }
        }
    }

    /// The query answers per call, and only for a command that substitutes at
    /// all — so a consumer never has to read a switch spelling itself.
    #[test]
    fn substitutions_performed_answers_per_call_and_only_for_substituting_commands() {
        let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
        assert_eq!(
            registry.substitutions_performed("set", &["x", "1"]),
            None,
            "a command that substitutes nothing has no answer to give"
        );
        let all = registry
            .substitutions_performed("subst", &["hello $name"])
            .expect("subst substitutes");
        assert!(all.variables && all.commands && all.backslashes);
        let off = registry
            .substitutions_performed("subst", &["-novariables", "hello $name"])
            .expect("subst substitutes");
        assert!(
            !off.variables && off.commands,
            "only the named kind is disabled"
        );
    }

    #[test]
    fn exact_return_source_words_distinguish_literals_from_substitution() {
        use crate::completion::CompletionCode;
        use crate::invocation_words::{InvocationArguments, InvocationWord};

        // Tcl 8.6 and 9.0 oracle: braces keep `$code` and `\\x32` literal,
        // while a bare/quoted substitution remains unknown and a decoded bare
        // escape can be supplied to this API as its effective static value.
        for dialect in ["tcl8.6", "tcl9.0"] {
            let reg = crate::model::ingress::static_context_for(dialect).commands();
            let trailing_option = [InvocationWord::Literal("-code")];
            assert_eq!(
                reg.exact_invocation_completion_words(
                    "return",
                    InvocationArguments::structured(&trailing_option),
                    None
                ),
                Some(ExactInvocationCompletion::Tcl(CompletionCode::Return)),
                "{dialect}: source-aware lone -code remains the result word"
            );
            for words in [
                vec![
                    InvocationWord::Literal("-code"),
                    InvocationWord::Literal("$code"),
                ],
                vec![
                    InvocationWord::Literal("-code"),
                    InvocationWord::Literal("\\x32"),
                ],
            ] {
                assert_eq!(
                    reg.exact_invocation_completion_words(
                        "return",
                        InvocationArguments::structured(&words),
                        None
                    ),
                    Some(ExactInvocationCompletion::Tcl(CompletionCode::Error)),
                    "{dialect}: {words:?}"
                );
            }
            let decoded = [
                InvocationWord::Literal("-code"),
                InvocationWord::Literal("2"),
            ];
            assert_eq!(
                reg.exact_invocation_completion_words(
                    "return",
                    InvocationArguments::structured(&decoded),
                    None
                ),
                Some(ExactInvocationCompletion::Tcl(CompletionCode::Return)),
            );
            for dynamic in [InvocationWord::Dynamic, InvocationWord::Dynamic] {
                let words = [InvocationWord::Literal("-code"), dynamic];
                assert_eq!(
                    reg.exact_invocation_completion_words(
                        "return",
                        InvocationArguments::structured(&words),
                        None
                    ),
                    None,
                );
            }
            let dynamic_option = [InvocationWord::Dynamic, InvocationWord::Literal("error")];
            assert_eq!(
                reg.invocation_completion_knowledge(
                    "return",
                    InvocationArguments::structured(&dynamic_option),
                    None,
                ),
                Some(InvocationCompletionKnowledge::Dynamic),
            );
        }
    }

    #[test]
    fn authored_source_model_roster_keeps_context_separate_from_native_release_evidence() {
        // naming.source.logical-procedure-definition-model
        // docs/design/analysis/name-resolution-proofs/logical-procedure-definition-model.md
        let driver =
            crate::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let older = crate::model::ingress::resolve_environment("tcl8.4").default_context_registry();
        let older = older.with_command_store(driver.commands().snapshot().shared_registry());
        for (context, lmap) in [(&*driver, true), (&older, false)] {
            let model = context.commands().authored_source_semantics_in_context(
                context.context(),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            );
            assert!(model.binding_names().contains("::proc"));
            assert_eq!(model.binding_names().contains("::lmap"), lmap);
            assert!(!model.binding_names().contains("::tcltest::test"));
            assert!(!model.binding_names().contains("::tcl::string::equal"));
        }
        let mut required = driver.context().clone();
        required.require_package("tcltest", None);
        let required_model = driver.commands().authored_source_semantics_in_context(
            &required,
            tcl_dialect::model::InvocationRealm::RuleLoader,
        );
        assert!(required_model.binding_names().contains("::tcltest::test"));
        let native =
            driver
                .commands()
                .effective_semantics_for_dialect(crate::InvocationDialect::of_point(
                    tcl_dialect::model::DialectPoint::canonical(
                        tcl_dialect::model::Release::JIM_0_79,
                    ),
                ));
        assert!(
            native.binding_names().is_empty(),
            "a source model cannot fill an unaudited Native roster"
        );
    }

    #[test]
    fn fresh_dialect_bindings_follow_availability_and_package_provenance() {
        let registry = CommandRegistry::build_default();
        for release in tcl_dialect::TclVersion::ALL {
            let semantics = registry
                .effective_semantics_for_dialect(crate::InvocationDialect::for_version(release));
            assert!(semantics.binding_names().contains("::set"));
            assert_eq!(
                semantics.binding_names().contains("::dict"),
                release >= tcl_dialect::TclVersion::V8_5
            );
            assert!(!semantics.binding_names().contains("::tcltest::test"));
        }
        let modern = registry.effective_semantics_for_dialect(crate::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        ));
        assert!(modern.binding_names().contains("::alias"));
        assert!(!modern.binding_names().contains("::trace"));
        let older = registry.effective_semantics_for_dialect(crate::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
        ));
        assert!(
            older.binding_names().is_empty(),
            "an unmeasured build cannot inherit current Jim presence"
        );
    }

    #[test]
    fn platform_bindings_require_the_selected_native_ambient_profile() {
        for (profile_name, commands) in [
            ("f5-irules", &["::HTTP2::header"][..]),
            (
                "f5-tmsh",
                &["::tmsh::create", "::tmsh::list", "::tmsh::log"][..],
            ),
        ] {
            let owner = crate::model::ingress::static_context_for(profile_name);
            let registry = owner.commands();
            let profile = registry.profile().expect("selected platform profile");
            let dialect = crate::InvocationDialect::of_profile(profile);
            let semantics = registry.effective_semantics_for_dialect(dialect);
            for command in commands {
                assert!(
                    semantics.binding_names().contains(*command),
                    "{profile_name}: {command}"
                );
            }
            assert!(!semantics.binding_names().contains("::snit::type"));
            let other = registry.effective_semantics_for_dialect(
                crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
            );
            for command in commands {
                assert!(
                    !other.binding_names().contains(*command),
                    "foreign profile: {command}"
                );
            }
        }
    }

    #[test]
    fn receiver_vocabulary_does_not_allocate_global_dispatchers() {
        let owner = crate::model::ingress::static_context_for("tcl8.6");
        let registry = owner.commands();
        let dialect = crate::InvocationDialect::of_profile(registry.profile().unwrap());
        let semantics = registry.effective_semantics_for_dialect(dialect);
        assert!(!semantics.binding_names().contains("::my"));
        assert!(!semantics.binding_names().contains("::self"));
        assert!(semantics.binding_names().contains("::oo::Helpers::self"));
    }

    #[test]
    fn exact_completion_counts_positionals_after_known_selector_and_terminator() {
        use crate::InvocationWord::{Dynamic, Literal};
        let registry = CommandRegistry::build_default();
        let extra_names = [Literal("create"), Literal("--"), Dynamic, Dynamic];
        assert_eq!(
            registry.exact_invocation_completion_words(
                "interp",
                InvocationArguments::structured(&extra_names),
                None,
            ),
            Some(ExactInvocationCompletion::Tcl(crate::CompletionCode::Error)),
            "dynamic positional bytes do not erase a known selector and excess count"
        );
        let unknown_option = [Literal("create"), Dynamic, Dynamic];
        assert_eq!(
            registry.exact_invocation_completion_words(
                "interp",
                InvocationArguments::structured(&unknown_option),
                None,
            ),
            None,
            "an unknown leading option cannot establish rejection by count"
        );
        let one_name = [Literal("create"), Literal("--"), Dynamic];
        assert_eq!(
            registry.exact_invocation_completion_words(
                "interp",
                InvocationArguments::structured(&one_name),
                None,
            ),
            None,
            "accepted arity does not prove successful creation"
        );
    }

    #[test]
    fn exact_process_exit_obeys_explicit_snapshot_without_catalogue_profile() {
        let registry = CommandRegistry::build_default();
        for release in tcl_dialect::TclVersion::ALL {
            let arguments = crate::InvocationArguments::literals(&["4294967296"])
                .with_dialect(crate::InvocationDialect::for_version(release));
            assert_eq!(
                registry.exact_invocation_completion_words("exit", arguments, None),
                Some(if release >= tcl_dialect::TclVersion::V9_0 {
                    ExactInvocationCompletion::ProcessExit
                } else {
                    ExactInvocationCompletion::Tcl(crate::completion::CompletionCode::Error)
                })
            );
        }
        let arguments = crate::InvocationArguments::literals(&["18446744073709551616"])
            .with_dialect(crate::InvocationDialect::of_point(
                tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
            ));
        assert_eq!(
            registry.exact_invocation_completion_words("exit", arguments, None),
            Some(ExactInvocationCompletion::Tcl(
                crate::completion::CompletionCode::Other(6)
            ))
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)] // descriptor layout matrix
    fn case_list_invocation_layout_is_registry_owned() {
        use crate::spec::CaseMatchMode;
        let reg = crate::model::ingress::static_context_for("tcl9.0").commands();
        let Some((_, two_arg)) = reg.case_invocation(
            "switch",
            &["-glob", "default {}"],
            Some(SurfaceQuery::core(Family::Tcl, "9.0")),
        ) else {
            panic!("two-argument case form must parse");
        };
        assert_eq!(two_arg.subject_index, Some(0));
        assert_eq!(two_arg.mode, CaseMatchMode::Exact);
        assert_eq!(two_arg.clause_list_index, Some(1));

        let Some((_, options)) = reg.case_invocation(
            "switch",
            &["-glob", "-nocase", "--", "subject", "p {}"],
            Some(SurfaceQuery::core(Family::Tcl, "9.0")),
        ) else {
            panic!("option-bearing case form must parse");
        };
        assert_eq!(options.subject_index, Some(3));
        assert_eq!(options.mode, CaseMatchMode::Glob);
        assert!(options.nocase);

        let tcl91 = crate::model::ingress::static_context_for("tcl9.1").commands();
        let Some((_, integer)) = tcl91.case_invocation(
            "switch",
            &["-integer", "--", "1", "1 {set x 1}"],
            Some(SurfaceQuery::core(Family::Tcl, "9.1")),
        ) else {
            panic!("Tcl 9.1 integer switch must retain its case-list body");
        };
        assert_eq!(integer.mode, CaseMatchMode::Other);
        assert_eq!(integer.clause_list_index, Some(3));
        for args in [
            ["-integer", "-nocase", "1", "1 {set x 1}"],
            ["-nocase", "-integer", "1", "1 {set x 1}"],
        ] {
            assert!(
                tcl91
                    .case_invocation(
                        "switch",
                        &args,
                        Some(SurfaceQuery::core(Family::Tcl, "9.1"))
                    )
                    .is_none(),
                "integer and nocase must be incompatible: {args:?}"
            );
            assert!(
                tcl91
                    .arg_indices_for_role("switch", &args, ArgRole::Body)
                    .is_empty(),
                "invalid integer/nocase form must expose no body: {args:?}"
            );
        }
        for args in [
            ["-integer", "-glob", "1", "1 {set x 1}"],
            ["-glob", "-integer", "1", "1 {set x 1}"],
            ["-integer", "-integer", "1", "1 {set x 1}"],
        ] {
            assert!(
                tcl91
                    .case_invocation(
                        "switch",
                        &args,
                        Some(SurfaceQuery::core(Family::Tcl, "9.1"))
                    )
                    .is_none(),
                "multiple match modes must be rejected: {args:?}"
            );
            assert!(
                tcl91
                    .arg_indices_for_role("switch", &args, ArgRole::Body)
                    .is_empty(),
                "multiple match modes must expose no body: {args:?}"
            );
        }
        assert_eq!(
            tcl91.arg_indices_for_role(
                "switch",
                &["-integer", "--", "1", "1 {set x 1}"],
                ArgRole::Body,
            ),
            vec![3]
        );
        assert!(
            reg.case_invocation(
                "switch",
                &["subject", "pattern", "body", "orphan"],
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .is_none()
        );
        assert!(
            reg.case_invocation(
                "switch",
                &["subject", "pattern {} orphan"],
                Some(SurfaceQuery::core(Family::Tcl, "9.0"))
            )
            .is_none()
        );

        let tcl84 = crate::model::ingress::static_context_for("tcl8.4").commands();
        for two_arg_switch in [
            ["-regexp", "default {puts hit}"],
            ["--", "default {puts hit}"],
        ] {
            assert!(
                tcl84
                    .case_invocation(
                        "switch",
                        &two_arg_switch,
                        Some(SurfaceQuery::core(Family::Tcl, "8.4"))
                    )
                    .is_none(),
                "Tcl 8.4 scans the option-like first word and rejects the missing subject: {two_arg_switch:?}"
            );
            assert!(
                tcl84
                    .arg_indices_for_role("switch", &two_arg_switch, ArgRole::Body)
                    .is_empty(),
                "Tcl 8.4 must not assign the second word a body role: {two_arg_switch:?}"
            );
            assert_eq!(
                tcl84
                    .resolve_option_terminator(
                        "switch",
                        &two_arg_switch,
                        Some(SurfaceQuery::core(Family::Tcl, "8.4"))
                    )
                    .expect("switch declares --")
                    .reserved_trailing_words,
                0,
                "Tcl 8.4 must scan both words in the option-like two-word form: {two_arg_switch:?}"
            );
        }
        for args in [
            &["subject", "default {puts hit}"][..],
            &["-regexp", "subject", "default {puts hit}"][..],
            &["--", "-subject", "default {puts hit}"][..],
        ] {
            assert!(
                tcl84
                    .case_invocation("switch", args, Some(SurfaceQuery::core(Family::Tcl, "8.4")))
                    .is_some(),
                "Tcl 8.4 accepts the unambiguous switch case-list form: {args:?}"
            );
            assert_eq!(
                tcl84.arg_indices_for_role("switch", args, ArgRole::Body),
                vec![args.len() - 1],
                "Tcl 8.4 assigns the final case-list word a body role: {args:?}"
            );
        }
        assert!(
            tcl84
                .case_invocation(
                    "switch",
                    &["-nocase", "subject", "pattern {}"],
                    Some(SurfaceQuery::core(Family::Tcl, "8.4")),
                )
                .is_none()
        );
        let tcl85 = crate::model::ingress::static_context_for("tcl8.5").commands();
        for (dialect, availability) in [
            ("tcl8.5", Some(SurfaceQuery::core(Family::Tcl, "8.5"))),
            ("tcl8.6", Some(SurfaceQuery::core(Family::Tcl, "8.6"))),
            ("tcl9.0", Some(SurfaceQuery::core(Family::Tcl, "9.0"))),
        ] {
            let registry = crate::model::ingress::static_context_for(dialect).commands();
            for two_arg_switch in [
                ["-regexp", "default {puts hit}"],
                ["--", "default {puts hit}"],
            ] {
                assert!(
                    registry
                        .case_invocation("switch", &two_arg_switch, availability)
                        .is_some(),
                    "{dialect} accepts the optionless two-word switch form: {two_arg_switch:?}"
                );
                assert_eq!(
                    registry.arg_indices_for_role("switch", &two_arg_switch, ArgRole::Body),
                    vec![1],
                    "{dialect} assigns the clause-list word a body role: {two_arg_switch:?}"
                );
                assert_eq!(
                    registry
                        .resolve_option_terminator("switch", &two_arg_switch, availability)
                        .expect("switch declares --")
                        .reserved_trailing_words,
                    2,
                    "{dialect} reserves the subject and clause-list words: {two_arg_switch:?}"
                );
            }
        }
        assert!(
            tcl85
                .case_invocation(
                    "switch",
                    &["-nocase", "subject", "pattern {}"],
                    Some(SurfaceQuery::core(Family::Tcl, "8.5")),
                )
                .is_some()
        );
        let default = CommandRegistry::build_default();
        assert!(
            default
                .case_invocation(
                    "switch",
                    &["subject", "pattern {}"],
                    Some(SurfaceQuery::core(Family::Tcl, "9.0")),
                )
                .is_some()
        );
        assert!(
            default
                .case_invocation(
                    "switch",
                    &["--", "-x", "-x {puts hit} default {puts miss}"],
                    Some(SurfaceQuery::core(Family::Tcl, "9.0")),
                )
                .is_some(),
            "the descriptor's -- terminator keeps a hyphenated switch subject positional"
        );
        assert!(
            reg.case_invocation(
                "switch",
                &["subject", "pattern -"],
                Some(SurfaceQuery::core(Family::Tcl, "9.0"))
            )
            .is_none()
        );

        let expect = crate::model::ingress::static_context_for("expect").commands();
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &["\"password:\" {send pw} -re {ye+s} {send yes} timeout {puts slow}"],
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    ),
                )
                .is_some(),
            "clause-leading flags must not break valid Expect pattern/body pairs"
        );
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &["{-re} {send literal}"],
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    ),
                )
                .is_some(),
            "a braced flag-shaped pattern is literal text, not a clause flag"
        );
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &["-re {ye+s}"],
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    )
                )
                .is_some(),
            "Expect permits a final pattern without an action"
        );
        for args in [
            ["-timeout", "5", "ready {action}"],
            ["-i", "spawn", "ready {action}"],
        ] {
            let (_, invocation) = expect
                .case_invocation(
                    "expect",
                    &args,
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")]),
                    ),
                )
                .expect("outer Expect value option followed by a final pattern");
            assert_eq!(invocation.clause_list_index, None, "{args:?}");
            assert_eq!(invocation.inline_clause_start, Some(2), "{args:?}");
            let clauses = crate::CaseListSpec::EXPECT
                .inline_clauses(&args, invocation.inline_clause_start.unwrap())
                .expect("one action-less inline clause");
            assert_eq!(clauses.len(), 1, "{args:?}");
            assert_eq!(clauses[0].pattern_index, 2, "{args:?}");
            assert_eq!(clauses[0].body_index, None, "{args:?}");
        }
        for pattern in ["#", ";"] {
            assert!(
                expect
                    .case_invocation(
                        "expect",
                        &[&format!(
                            "{pattern} {{send literal}} default {{send other}}"
                        )],
                        Some(
                            SurfaceQuery::core(Family::Tcl, "8.6")
                                .with_packages(&[PackageFloor::named("expect")])
                        ),
                    )
                    .is_some(),
                "{pattern:?} is a literal Tcl list pattern, not script syntax"
            );
        }
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &["-timeout"],
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    )
                )
                .is_none(),
            "a value-taking clause flag without a value/pattern/body is invalid"
        );
    }

    #[test]
    fn inline_expect_case_flags_and_actions_are_registry_owned() {
        let expect = crate::model::ingress::static_context_for("expect").commands();
        let args = [
            "-re",
            "{ye+s}",
            "{send yes}",
            "-timeout",
            "5",
            "timeout",
            "{send slow}",
        ];
        let (_, inline) = expect
            .case_invocation(
                "expect",
                &args,
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")]),
                ),
            )
            .expect("inline Expect flags and value flags must parse");
        let clauses = crate::CaseListSpec::EXPECT
            .inline_clauses(&args, inline.inline_clause_start.expect("inline form"))
            .expect("inline clauses");
        assert_eq!(clauses.len(), 2);
        assert_eq!(clauses[0].pattern_index, 1);
        assert_eq!(clauses[0].body_index, Some(2));
        assert_eq!(clauses[0].mode, crate::spec::CaseMatchMode::Regexp);
        assert_eq!(clauses[1].pattern_index, 5);
        assert_eq!(clauses[1].body_index, Some(6));
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &["-not", "ready", "{send ok}"],
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    )
                )
                .is_some(),
            "unique Expect flag abbreviations must retain the action body"
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)] // Expect oracle layout matrix
    fn expect_inline_flag_table_matches_the_oracle() {
        let expect = crate::model::ingress::static_context_for("expect").commands();
        for flag in [
            "-glob",
            "-regexp",
            "-exact",
            "-notransfer",
            "-nocase",
            "-i",
            "-indices",
            "-iread",
            "-timestamp",
            "-nobrace",
        ] {
            let args = if matches!(flag, "-i") {
                vec![flag, "spawn", "pattern", "{action}"]
            } else {
                vec![flag, "pattern", "{action}"]
            };
            assert!(
                expect
                    .case_invocation(
                        "expect",
                        &args,
                        Some(
                            SurfaceQuery::core(Family::Tcl, "8.6")
                                .with_packages(&[PackageFloor::named("expect")])
                        )
                    )
                    .is_some(),
                "canonical {flag} must parse"
            );
        }
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &["-timeout", "5", "pattern", "{action}"],
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    )
                )
                .is_some()
        );
        for flag in ["-gl", "-re", "-ex", "-not"] {
            assert!(
                expect
                    .case_invocation(
                        "expect",
                        &[flag, "pattern", "{action}"],
                        Some(
                            SurfaceQuery::core(Family::Tcl, "8.6")
                                .with_packages(&[PackageFloor::named("expect")])
                        )
                    )
                    .is_some(),
                "unique abbreviation {flag} must parse"
            );
        }
        for flag in ["-g", "-r", "-e", "-noc", "-nob"] {
            assert!(
                expect
                    .case_invocation(
                        "expect",
                        &[flag, "pattern", "{action}"],
                        Some(
                            SurfaceQuery::core(Family::Tcl, "8.6")
                                .with_packages(&[PackageFloor::named("expect")])
                        )
                    )
                    .is_some(),
                "unique canonical-prefix {flag} must remain an inline clause flag"
            );
        }
        for flag in ["-n", "-bogus"] {
            assert!(
                expect
                    .case_invocation(
                        "expect",
                        &[flag, "pattern", "{action}"],
                        Some(
                            SurfaceQuery::core(Family::Tcl, "8.6")
                                .with_packages(&[PackageFloor::named("expect")])
                        )
                    )
                    .is_none(),
                "ambiguous or unknown {flag} must invalidate the invocation"
            );
        }
        let args = ["--", "-re", "{action}"];
        let (_, invocation) = expect
            .case_invocation(
                "expect",
                &args,
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")]),
                ),
            )
            .expect("-- makes -re a pattern");
        let clauses = crate::CaseListSpec::EXPECT
            .inline_clauses(&args, invocation.inline_clause_start.expect("inline"))
            .expect("clause");
        assert_eq!(clauses[0].pattern_index, 1);
        assert_eq!(clauses[0].body_index, Some(2));
        let (_, omitted) = expect
            .case_invocation(
                "expect",
                &["-re", "pattern"],
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")]),
                ),
            )
            .expect("omitted final action is valid");
        assert_eq!(
            crate::CaseListSpec::EXPECT
                .inline_clauses(&["-re", "pattern"], omitted.inline_clause_start.unwrap())
                .unwrap()[0]
                .body_index,
            None
        );
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &["-nobrace", "{pattern}"],
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    )
                )
                .is_some(),
            "-nobrace makes one braced word an action-less pattern"
        );
        let (_, brace) = expect
            .case_invocation(
                "expect",
                &["-brace", "{default {return FOLDED}}"],
                Some(
                    SurfaceQuery::core(Family::Tcl, "8.6")
                        .with_packages(&[PackageFloor::named("expect")]),
                ),
            )
            .expect("exact -brace selects a clause list");
        assert_eq!(brace.clause_list_index, Some(1));
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &["-b", "{default {return FOLDED}}"],
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    ),
                )
                .is_none(),
            "-brace is exact-only, so -b is not a clause flag abbreviation"
        );
        assert!(
            expect
                .case_invocation(
                    "expect",
                    &["-brac", "{default {return FOLDED}}"],
                    Some(
                        SurfaceQuery::core(Family::Tcl, "8.6")
                            .with_packages(&[PackageFloor::named("expect")])
                    ),
                )
                .is_none(),
            "-brace must not accept a near-complete prefix either"
        );
        for args in [
            &["-timeout", "5", "-brace", "{default {return FOLDED}}"] as &[&str],
            &["-i", "spawn", "-brace", "{default {return FOLDED}}"],
            &["-brace", "{default {return FOLDED}}", "extra"],
        ] {
            assert!(
                expect
                    .case_invocation(
                        "expect",
                        args,
                        Some(
                            SurfaceQuery::core(Family::Tcl, "8.6")
                                .with_packages(&[PackageFloor::named("expect")])
                        )
                    )
                    .is_none(),
                "force-list selector must be first and have one remainder: {args:?}"
            );
        }
    }

    #[test]
    fn subjectless_case_list_defers_clause_flags_after_outer_value_options() {
        use crate::hover::{OptionSpec, OptionValue};
        use crate::spec::CaseListSpec;

        // This deliberately does not name `expect`: a future descriptor with
        // one value-taking outer option and one overlapping clause flag must
        // use the same precedence rule. The body is the observable proof that
        // `-clause` was not swallowed by the outer scan.
        let case = CaseListSpec {
            subject_args: 0,
            two_arg_optionless_surface: None,
            fallthrough_body: None,
            value_options_require_regex: &[],
            special_match_options: &[],
            clause_flags: &["-clause", "--"],
            clause_regex_flag: None,
            clause_value_flags: &[],
            clause_end_options_flag: Some("--"),
            clause_force_inline_flag: None,
            clause_force_list_flag: None,
            clause_force_list_shape: None,
            allow_omitted_final_body: true,
            keyword_patterns: &[],
            keyword_patterns_require_final: false,
            exhaustive_keyword_patterns: &[],
            optional_subject_separator: None,
            warn_unbraced_bodies: false,
            default_mode: crate::spec::CaseMatchMode::Exact,
            pattern_words: crate::spec::PatternWords::Single,
        };
        let options = [
            OptionSpec {
                name: "-outer",
                value: OptionValue::value("value"),
                ..OptionSpec::DEFAULT
            },
            OptionSpec {
                name: "-clause",
                ..OptionSpec::DEFAULT
            },
        ];
        let option_refs: Vec<&OptionSpec> = options.iter().collect();
        let args = ["-outer", "value", "-clause", "pattern", "{body}"];
        let invocation = case
            .invocation(
                &args,
                &option_refs,
                Some(SurfaceQuery::any_release(Family::Tcl)),
            )
            .expect("the outer value must precede, not consume, the clause flag");
        assert_eq!(invocation.inline_clause_start, Some(2));
        let clauses = case
            .inline_clauses(&args, invocation.inline_clause_start.unwrap())
            .expect("the deferred flag must retain the action body");
        assert_eq!(clauses[0].pattern_index, 3);
        assert_eq!(clauses[0].body_index, Some(4));
    }

    #[test]
    fn case_list_invocation_abstains_on_empty_and_truncated_shapes() {
        let switch = crate::model::ingress::static_context_for("tcl9.0").commands();
        for args in [
            &[][..],
            &["subject"][..],
            &["subject", ""][..],
            &["-regexp"][..],
            &["-regexp", "subject"][..],
            &["--"][..],
        ] {
            assert!(
                switch
                    .case_invocation("switch", args, Some(SurfaceQuery::core(Family::Tcl, "9.0")))
                    .is_none(),
                "truncated switch invocation must abstain: {args:?}",
            );
        }

        let expect = crate::model::ingress::static_context_for("expect").commands();
        for args in [
            &[][..],
            &["-brace"][..],
            &["-nobrace"][..],
            &["-timeout"][..],
            &["-timeout", "5"][..],
            &[""][..],
        ] {
            assert!(
                expect
                    .case_invocation(
                        "expect",
                        args,
                        Some(
                            SurfaceQuery::core(Family::Tcl, "8.6")
                                .with_packages(&[PackageFloor::named("expect")])
                        )
                    )
                    .is_none(),
                "truncated Expect invocation must abstain: {args:?}",
            );
        }

        // Exercise the generic selector branch independently of either
        // built-in descriptor. The selector is intentionally outside the
        // clause-flag vocabulary, so a complete call consumes it as shape
        // syntax while a missing subject/body must simply abstain.
        let custom = crate::CaseListSpec {
            clause_force_inline_flag: Some("-inline"),
            allow_omitted_final_body: false,
            ..crate::CaseListSpec::EXPECT
        };
        for args in [
            &[][..],
            &["-inline"][..],
            &["-inline", "subject"][..],
            &[""][..],
        ] {
            assert!(
                custom
                    .invocation(args, &[], Some(SurfaceQuery::any_release(Family::Tcl)))
                    .is_none(),
                "truncated custom selector invocation must abstain: {args:?}",
            );
        }
        assert!(
            custom
                .invocation(
                    &["-inline", "pattern", "{body}"],
                    &[],
                    Some(SurfaceQuery::any_release(Family::Tcl)),
                )
                .is_some(),
            "complete custom inline-selector invocation remains valid",
        );

        // A dynamic two-word switch cannot expose a static body region, but
        // asking for roles must remain a conservative abstention rather than
        // a panic. The compiler's W304 regression separately proves Tcl's
        // optionless two-word rule is retained for diagnostics.
        assert!(
            switch
                .arg_indices_for_role("switch", &["$subject", "$cases"], ArgRole::Body)
                .is_empty(),
        );
    }

    /// The whole registry-declared boundary surface, resolved by name: every
    /// command that widens a file's caller set reports its kind, a
    /// `::`-qualified spelling resolves the same, and a command that does
    /// neither reports nothing.
    #[test]
    fn unit_linkage_covers_every_declared_boundary_command() {
        let reg = CommandRegistry::build_default();
        let empty = None;
        for (name, args, want) in [
            ("source", &["lib.tcl"][..], Traits::LOADS_EXTERNAL_UNIT),
            ("::source", &["lib.tcl"][..], Traits::LOADS_EXTERNAL_UNIT),
            ("load", &["libx.so"][..], Traits::LOADS_EXTERNAL_UNIT),
            ("auto_load", &["helper"][..], Traits::LOADS_EXTERNAL_UNIT),
            (
                "auto_import",
                &["::lib::*"][..],
                Traits::LOADS_EXTERNAL_UNIT,
            ),
            // `namespace import` is deliberately not a boundary — see its
            // own spec comment.
            (
                "namespace",
                &["import", "::lib::helper"][..],
                Traits::empty(),
            ),
            (
                "namespace",
                &["export", "helper"][..],
                Traits::EXPORTS_COMMAND,
            ),
            (
                "namespace",
                &["ensemble", "create"][..],
                Traits::EXPORTS_COMMAND,
            ),
            ("namespace", &["eval", "::ns", "{}"][..], Traits::empty()),
            ("set", &["x", "1"][..], Traits::empty()),
            ("no_such_command_xyz", &["source"][..], Traits::empty()),
        ] {
            assert_eq!(
                reg.unit_linkage(name, args, empty),
                want,
                "unit_linkage({name}, {args:?})"
            );
        }
    }

    /// `SAFE_INTERP_HIDDEN` has its own bit, distinct from `TRANSFERS_CONTROL`:
    /// every `break`/`continue`/`tailcall`/`yield` reads as safe-interp-hidden
    /// and every `cd`/`exec`/`glob`/… reads as control-transferring, which
    /// `FRAME_SENSITIVE_TRAITS` consumes directly.
    #[test]
    fn safe_interp_hidden_no_longer_aliases_transfers_control() {
        let reg = CommandRegistry::build_default();
        assert_ne!(Traits::TRANSFERS_CONTROL, Traits::SAFE_INTERP_HIDDEN);
        let cd = reg.get("cd").expect("cd is registered");
        assert!(cd.traits.contains(Traits::SAFE_INTERP_HIDDEN));
        assert!(!cd.traits.contains(Traits::TRANSFERS_CONTROL));
        assert!(!reg.is_frame_sensitive("cd"));
        let brk = reg.get("break").expect("break is registered");
        assert!(brk.traits.contains(Traits::TRANSFERS_CONTROL));
        assert!(!brk.traits.contains(Traits::SAFE_INTERP_HIDDEN));
    }

    /// `declares_command_at` answers "is this in a fresh
    /// interpreter's command table", which is strictly narrower than `get`'s
    /// "is this a spelling a call site may write".
    ///
    /// Every row is oracle-verified on tclsh 9.0.4 and 8.6.14 via
    /// `info commands <name>` in a fresh `interp create`.
    #[test]
    fn declares_command_at_is_the_fresh_interpreter_command_table() {
        let reg = crate::model::ingress::static_context_for("tcl9.0").commands();
        // `info commands ::set` -> ::set
        for name in ["set", "::set", "puts", "::puts", "if", "foreach"] {
            assert!(reg.declares_command_at(name), "{name} is a global builtin");
        }
        // `info commands ::tcl::mathop::+` -> ::tcl::mathop::+
        for name in [
            "tcl::mathop::+",
            "::tcl::mathop::+",
            "oo::define",
            "::oo::define",
        ] {
            assert!(
                reg.declares_command_at(name),
                "{name} is a namespaced builtin"
            );
        }
        // `info commands ::+` -> {} and `+ 1 2` -> invalid command name "+".
        // `get` still answers `Some` for these — that is the whole point.
        for name in ["+", "::+", "eq", "::eq", "in", "::in"] {
            assert!(
                reg.get(name).is_some(),
                "{name} must stay a resolvable *spelling*",
            );
            assert!(
                !reg.declares_command_at(name),
                "{name} is only callable after `namespace import ::tcl::mathop::*`",
            );
        }
        // A name nothing declares.
        for name in ["notACommand", "::notACommand", "define"] {
            assert!(!reg.declares_command_at(name), "{name}");
        }
    }

    /// The answer is per dialect, because the command table is: `HTTP::uri`
    /// exists for iRules and nowhere else.
    #[test]
    fn declares_command_at_is_dialect_specific() {
        assert!(
            crate::model::ingress::static_context_for("f5-irules")
                .commands()
                .declares_command_at("HTTP::uri"),
            "iRules declares HTTP::uri",
        );
        assert!(
            !crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .declares_command_at("HTTP::uri"),
            "plain Tcl does not",
        );
    }

    #[test]
    fn smoke_lookup_set_command() {
        let reg = CommandRegistry::build_default();
        let spec = reg.get("set").expect("`set` must be a known command");
        assert!(
            spec.arity.min <= 1 && spec.arity.max >= 1,
            "set must accept at least a variable name: {:?}",
            spec.arity
        );
    }
}

#[cfg(test)]
mod registry_snapshot_tests {
    use super::*;

    #[test]
    fn snapshot_retains_pack_reference_overlay_and_special_variable_axes() {
        // Software cache contract: no native body or lookup is asserted.
        let mut registry = CommandRegistry::build_default();
        let spec = registry.get("set").unwrap();
        let original = registry.snapshot();
        let origin = crate::pack_origin::PackOrigin {
            pack: "SnapshotPack".to_owned(),
            content_hash: 123,
            vocabulary_version: "2.0".to_owned(),
        };
        registry.insert_pack_origin(spec, origin.clone());
        let with_origin = registry.snapshot();
        assert_ne!(original, with_origin);
        assert_eq!(original.registry().pack_origin(spec), None);
        assert_eq!(with_origin.registry().pack_origin(spec), Some(&origin));
        let address = std::ptr::from_ref(spec).addr();
        registry.insert_reference_text(spec, Arc::from("exact retained definition"));
        let with_reference = registry.snapshot();
        assert_ne!(with_origin, with_reference);
        assert!(
            !with_origin
                .registry()
                .reference_texts
                .contains_key(&address)
        );
        assert_eq!(
            with_reference.registry().reference_texts[&address].as_ref(),
            "exact retained definition",
        );
        registry.insert_special_var(&crate::special_vars::SPECIAL_VARS[0]);
        let with_variable = registry.snapshot();
        assert_ne!(with_reference, with_variable);
        assert_eq!(with_reference.registry().special_vars.len(), 0);
        assert_eq!(with_variable.registry().special_vars.len(), 1);
        registry.set_overlay(123);
        let with_overlay = registry.snapshot();
        assert_ne!(with_variable, with_overlay);
        assert_eq!(with_variable.registry().overlay, None);
        assert_eq!(with_overlay.registry().overlay, Some(123));
        assert_eq!(with_overlay.registry().generation(), registry.generation());
    }

    #[test]
    fn snapshot_retains_original_surface_across_every_mutation_kind() {
        let mut registry = CommandRegistry::build_default();
        let original = registry.snapshot();
        let retained_store = original.shared_registry();
        assert!(std::ptr::eq(retained_store.as_ref(), original.registry()));
        assert!(std::ptr::eq(
            original.registry(),
            registry.snapshot().registry()
        ));
        assert_eq!(original, CommandRegistry::build_default().snapshot());
        registry.insert_ambient_package("SnapshotPackage", "1.0");
        let ambient = registry.snapshot();
        assert_ne!(original, ambient);
        assert!(
            !original
                .registry()
                .ambient_package_rows()
                .contains(&("SnapshotPackage", "1.0"))
        );
        assert!(
            ambient
                .registry()
                .ambient_package_rows()
                .contains(&("SnapshotPackage", "1.0"))
        );
        let mut spec = registry.get("set").unwrap().clone();
        spec.traits = crate::Traits::PURE;
        registry.insert(spec);
        let overridden = registry.snapshot();
        assert_ne!(ambient, overridden);
        assert_ne!(
            ambient.registry().get("set").unwrap().traits,
            overridden.registry().get("set").unwrap().traits
        );
        registry.set_profile(tcl_dialect::DialectProfile::find("tcl8.4").unwrap());
        let profiled = registry.snapshot();
        assert_ne!(overridden, profiled);
        assert!(overridden.registry().profile().is_none());
        assert_eq!(profiled.registry().profile().unwrap().name, "tcl8.4");
        assert!(retained_store.profile().is_none());
        assert!(
            !retained_store
                .ambient_package_rows()
                .contains(&("SnapshotPackage", "1.0"))
        );
        assert_eq!(
            retained_store.get("set").unwrap().traits,
            original.registry().get("set").unwrap().traits
        );
    }

    #[test]
    fn equal_hashes_never_make_different_snapshot_contents_equal() {
        let first = CommandRegistry::build_default().snapshot();
        let mut registry = CommandRegistry::build_default();
        registry.insert_ambient_package("SnapshotCollision", "2.0");
        let mut second = registry.snapshot();
        drop(registry);
        let second_data = Arc::get_mut(&mut second.0).expect("unique test snapshot");
        Arc::get_mut(&mut second_data.semantic_key.0)
            .expect("unique semantic key")
            .fingerprint = first.0.semantic_key.0.fingerprint;
        assert_ne!(first, second);
    }
}

#[cfg(test)]
mod original_document_grammar_tests {

    #[test]
    fn original_document_member_grammar_keeps_source_vocabulary_separate_from_execution() {
        // Implementation contract: naming.consumer.typed-spec-pack-notice-actions
        // docs/design/analysis/name-resolution-proofs/typed-spec-pack-notice-actions.md
        let context = crate::model::ingress::static_context_for("spectcl");
        let registry = context.commands();
        let document = registry.document_grammar().unwrap();
        let pack = registry
            .authored_document_member_grammar(document, "speclib")
            .unwrap();
        let command = registry
            .authored_document_member_grammar(pack, "command")
            .unwrap();
        assert!(
            command
                .members
                .iter()
                .any(|member| member.keyword == "arity")
        );
        assert!(
            registry
                .authored_document_member_grammar(document, "command")
                .is_none()
        );
        assert!(
            registry
                .authored_document_member_grammar(command, "pool")
                .is_none()
        );
        let native = crate::model::ingress::static_context_for("tcl8.6").commands();
        assert!(
            native
                .authored_document_member_grammar(document, "speclib")
                .is_none()
        );
        let other = crate::model::ingress::static_context_for("sslictcl").commands();
        assert!(
            other
                .authored_document_member_grammar(document, "speclib")
                .is_none()
        );
    }
}

#[cfg(test)]
mod native_return_effect_ingress_tests {
    use super::native_return_state_effect;
    use crate::completion_route::ReturnStateEffect;
    use crate::{InvocationArguments, InvocationDialect, InvocationWord};

    #[test]
    fn native_return_effect_requires_its_independent_selected_protocol() {
        // naming.minifier.logical-formal-binding-alpha
        // docs/design/analysis/name-resolution-proofs/logical-formal-binding-alpha.md
        let arguments = [InvocationWord::Dynamic];
        let logical = InvocationDialect::of_profile(&tcl_dialect::DialectProfile::plain_tcl());
        assert!(logical.authored_name_policy().is_some());
        assert!(logical.native_name_protocol().is_none());
        for dialect in [None, Some(logical)] {
            let words = InvocationArguments::structured(&arguments);
            let words = dialect.map_or(words, |dialect| words.with_dialect(dialect));
            assert_eq!(
                native_return_state_effect(words),
                ReturnStateEffect::MayMaterialiseError
            );
        }
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            assert_eq!(
                native_return_state_effect(
                    InvocationArguments::structured(&arguments).with_dialect(dialect)
                ),
                ReturnStateEffect::ResultAndCompletion,
                "{version:?}"
            );
        }
        let jim = InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        );
        assert_eq!(
            native_return_state_effect(
                InvocationArguments::structured(&arguments).with_dialect(jim)
            ),
            ReturnStateEffect::ResultAndCompletion
        );
        let unmeasured = InvocationDialect {
            core_point: Some(tcl_dialect::model::DialectPoint::canonical(
                tcl_dialect::model::Release::JIM_0_79,
            )),
            tcl_version: None,
            ..jim
        };
        assert!(unmeasured.native_name_protocol().is_none());
        assert_eq!(
            native_return_state_effect(
                InvocationArguments::structured(&arguments).with_dialect(unmeasured)
            ),
            ReturnStateEffect::MayMaterialiseError
        );
    }
}

#[cfg(test)]
mod original_instance_descriptor_tests {
    #[test]
    fn original_instance_descriptor_keeps_registry_identity_prefixes_and_lifecycle() {
        // naming.source.original-registered-instance-words
        // docs/design/analysis/name-resolution-proofs/source-original-registered-instance-words.md
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry =
            crate::registry::CommandRegistry::build_default().project_for_profile(profile);
        let query = Some(profile.surface_query());
        let factory = registry.get_for_surface("ttk::treeview", query).unwrap();
        let method = registry
            .instance_method_for_descriptor_at(factory, "conf", None, query)
            .unwrap();
        assert_eq!(method.name, "configure");
        assert!(std::ptr::eq(
            method,
            registry
                .instance_method_at(factory.name, "conf", None, query)
                .unwrap()
        ));
        let foreign = factory.clone();
        assert!(
            registry
                .instance_methods_for_descriptor_at(&foreign, None, query)
                .is_empty()
        );
        assert!(
            registry
                .resolve_structured_instance_invocation_for_descriptor(
                    &foreign,
                    crate::InvocationWords::literals(".t", &["configure"]),
                    None,
                    query,
                )
                .is_none()
        );
        assert!(
            registry
                .resolve_structured_instance_invocation(
                    factory.name,
                    crate::InvocationWords::literals(".t", &["move", "one"]),
                    query,
                )
                .is_none(),
            "the named effect facade still requires independent package admission"
        );
        let packages = [tcl_dialect::model::PackageFloor::at("Tk", "8.6")];
        assert!(
            registry
                .resolve_structured_instance_invocation(
                    factory.name,
                    crate::InvocationWords::literals(".t", &["move", "one"]),
                    query.map(|query| query.with_packages(&packages)),
                )
                .is_some()
        );
        let selected = registry
            .resolve_structured_instance_invocation_for_descriptor(
                factory,
                crate::InvocationWords::literals(".t", &["move", "one"]),
                None,
                query,
            )
            .unwrap();
        assert_eq!(
            selected
                .authored_source_descriptors()
                .subcommand
                .unwrap()
                .name,
            "move"
        );
        assert_eq!(
            selected.facts().arity_accepts_frozen_arguments(),
            Some(false)
        );
    }
}
