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

//! How an invocation completes: completion codes, exact `return` and `exit` completions, completion routes and what is known of them.

use super::CommandRegistry;
use crate::hooks::LoweringHookId;
use crate::invocation_words::InvocationWord;
use crate::resolved_invocation::ResolvedInvocation;
use crate::traits::Traits;
use crate::value_transfer::completion::ReturnDecoding;
use crate::{InvocationArguments, InvocationWords};
use tcl_dialect::model::SurfaceQuery;

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
    use crate::completion_route::ReturnStateEffect as Effect;
    let Some(dialect) = args.dialect() else {
        return Effect::MayMaterialiseError;
    };
    // Source-only family and numeric grammar do not select a Native handler.
    // The authored F5 Tcl 8.4 core (TMM's interpreter, simulated) selects the
    // Tcl 8.4 handler; its own name protocol is the authored simulation.
    if dialect.native_name_protocol().is_none() && dialect.authored_f5_tcl84_core().is_none() {
        return Effect::MayMaterialiseError;
    }
    return_state_effect_for_source_grammar(args, dialect, false)
}

/// Conditional Return storage under the independently selected source grammar.
/// This describes the authored operation, not a Native handler or completion.
#[must_use]
pub fn authored_source_return_state_effect(
    args: crate::InvocationArguments<'_>,
) -> crate::completion_route::ReturnStateEffect {
    let Some(dialect) = args.dialect() else {
        return crate::completion_route::ReturnStateEffect::MayMaterialiseError;
    };
    return_state_effect_for_source_grammar(args, dialect, true)
}

fn return_state_effect_for_source_grammar(
    args: crate::InvocationArguments<'_>,
    dialect: crate::InvocationDialect,
    allow_unanimous_unknown_grammar: bool,
) -> crate::completion_route::ReturnStateEffect {
    use crate::completion::CompletionCode;
    use crate::completion_route::{
        InvocationCompletionRoute as Route, ReturnInvocationGrammar as Grammar,
        ReturnStateEffect as Effect,
    };
    let grammar = Grammar::for_dialect(dialect);
    if (grammar == Grammar::Unknown && !allow_unanimous_unknown_grammar)
        || args.exact_argv_len().is_none()
    {
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

impl CommandRegistry {
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
        let descriptor = resolved.semantics.completion.or_conservative();
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
        let descriptor = resolved.semantics.completion.or_conservative();
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
        let descriptor = resolved.semantics.completion.or_conservative();
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
        let descriptor = resolved.semantics.completion.or_conservative();
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
}
