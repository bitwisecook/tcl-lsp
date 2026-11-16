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

//! Control-flow arms: `try` handler clauses and their selectors, control arm semantics and case invocations.

use super::CommandRegistry;
use crate::InvocationArguments;
use crate::arg_role::ArgRole;
use crate::hooks::LoweringHookId;
use crate::spec::CommandSpec;
use tcl_dialect::model::SurfaceQuery;

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
    let mut selected = parse_try_control_invocation_with_lists(
        &plan,
        &values,
        tcl_syntax::number::Numbers::Target(dialect.numbers),
        dialect.completion_code_policy(),
        Some((dialect.lexer_grammar, arguments.is_source_aware())),
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
    parse_try_control_invocation_with_lists(plan, args, numbers, policy, None)
}

/// Selected argument values retain their actual list grammar and whether the
/// source-aware operand producer already separated value from source syntax.
pub(crate) fn parse_selected_try_control_invocation(
    plan: &crate::ClausePlan,
    arguments: crate::InvocationArguments<'_>,
) -> Option<TryControlInvocation> {
    let dialect = arguments.dialect()?;
    parse_try_control_invocation_with_lists(
        plan,
        &arguments.literal_values()?,
        tcl_syntax::number::Numbers::Target(dialect.numbers),
        dialect.completion_code_policy(),
        Some((dialect.lexer_grammar, arguments.is_source_aware())),
    )
}

fn parse_try_control_invocation_with_lists(
    plan: &crate::ClausePlan,
    args: &[&str],
    numbers: tcl_syntax::number::Numbers,
    policy: crate::completion::CompletionCodePolicy,
    selected: Option<(tcl_dialect::LexerGrammar, bool)>,
) -> Option<TryControlInvocation> {
    let list_len = |value: &str| {
        if !selected.is_some_and(|(_, evaluated)| evaluated)
            && tcl_syntax::naming::is_dynamic_word(value)
        {
            return None;
        }
        match selected {
            Some((grammar, _)) => {
                tcl_syntax::list::split_list_with_syntax(value, grammar.list_parse, grammar.escapes)
                    .ok()
                    .map(|items| items.len())
            }
            None => tcl_syntax::list::split_list(value)
                .ok()
                .map(|items| items.len()),
        }
    };
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
                        list_len(selector)?;
                        TryClauseKind::Trap
                    }
                };
                let variable_list_index = clause.operand(ArgRole::LoopVarList)?;
                let variable_list = *args.get(variable_list_index)?;
                if list_len(variable_list)? > 2 {
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

/// Structural meaning of an already selected control operation. Operand
/// membership, grammar validity, body geometry and execution are separate.
pub(crate) const fn control_arm_semantics_for_hook(
    hook: LoweringHookId,
) -> Option<ControlArmSemantics> {
    match hook {
        LoweringHookId::Switch => Some(ControlArmSemantics::Selected),
        LoweringHookId::NamespaceEval => Some(ControlArmSemantics::FrameBoundary),
        LoweringHookId::Catch => Some(ControlArmSemantics::CompletionBoundary),
        LoweringHookId::Foreach | LoweringHookId::Lmap | LoweringHookId::ForeachLine => {
            Some(ControlArmSemantics::BoundedIteration)
        }
        LoweringHookId::For | LoweringHookId::While => Some(ControlArmSemantics::Uncertain),
        _ => None,
    }
}

fn try_control_arms(
    plan: &crate::ClausePlan,
    args: &[&str],
    numbers: tcl_syntax::number::Numbers,
) -> Option<Vec<(usize, ControlArmSemantics)>> {
    try_control_arms_for_grammar(
        plan,
        args,
        numbers,
        crate::completion::CompletionCodePolicy::for_numbers(numbers),
    )
}

pub(crate) fn try_control_arms_for_grammar(
    plan: &crate::ClausePlan,
    args: &[&str],
    numbers: tcl_syntax::number::Numbers,
    policy: crate::completion::CompletionCodePolicy,
) -> Option<Vec<(usize, ControlArmSemantics)>> {
    let invocation = parse_try_control_invocation(plan, args, numbers, policy)?;
    Some(try_control_arms_from_invocation(invocation))
}

pub(crate) fn selected_try_control_arms(
    plan: &crate::ClausePlan,
    arguments: crate::InvocationArguments<'_>,
) -> Option<Vec<(usize, ControlArmSemantics)>> {
    Some(try_control_arms_from_invocation(
        parse_selected_try_control_invocation(plan, arguments)?,
    ))
}

fn try_control_arms_from_invocation(
    invocation: TryControlInvocation,
) -> Vec<(usize, ControlArmSemantics)> {
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
    arms
}

impl CommandRegistry {
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
            LoweringHookId::Try => try_control_arms(
                &resolved.clause_plan(args, None)?,
                args,
                tcl_syntax::number::Numbers::of_profile(self.profile()),
            )?
            .into_iter()
            .find_map(|(idx, semantics)| (idx == body_index).then_some(semantics)),
            hook => control_arm_semantics_for_hook(hook),
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
}
