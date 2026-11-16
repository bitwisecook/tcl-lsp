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

//! The per-family thunks: each turns a shipped hook signature into a `HookCall` and reads the answer back.

use super::dispatch::dispatch;
use super::{
    ConstraintCallCtx, HookAnswer, HookCall, HookFamily, HookSlot, HookWord, OptionCallCtx,
    PackTransition, current_dialect,
};
use crate::arg_role::{AppendedArity, ArgRole};
use crate::clause_shape::ClauseShapeIssue;
use crate::hover::{OptionValueOutcome, ScriptTiming};
use crate::invocation_words::{
    CommandPrefixArguments, InvocationArguments, InvocationWord, InvocationWordKind,
};
use crate::literal_validation::LiteralArgumentValidation;
use crate::state_transition::{
    AliasWords, CallerFrameSelection, StateTransition, StateTransitions, TransitionSubject,
    VARIABLE_ALIAS_DOMAINS, VariableAliasDestination, VariableAliasNamePurpose,
    VariableAliasTarget, VariableCellAliasTransition,
};
use crate::value_transfer::ImplementationBudget;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use tcl_dialect::TclVersion;

/// Intern a hook-produced message as `&'static str`.
///
/// The registry's message fields are `&'static` because shipped messages live
/// in `.rodata`. A pack's are minted at load and live for the process, exactly
/// like the pack's leaked spec; the map collapses repeats so a hook emitting
/// the same rejection on every call site leaks once, not once per call.
fn intern(message: &str) -> &'static str {
    static INTERNED: LazyLock<Mutex<HashMap<String, &'static str>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    let mut table = match INTERNED.lock() {
        Ok(table) => table,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(existing) = table.get(message) {
        return existing;
    }
    let leaked: &'static str = Box::leak(message.to_owned().into_boxed_str());
    table.insert(message.to_owned(), leaked);
    leaked
}

/// Build the DSL's `words` view from the compatibility `&[&str]` a shipped
/// hook signature carries: every word is literal, which is what that view
/// means.
///
/// **Fidelity, stated plainly.** Only the families whose shipped Rust
/// signature carries [`InvocationArguments`] — the literal-argument validator,
/// command-prefix resolver and script-timing resolver — can report a word as `dynamic`,
/// `expanded`, or `opaque`; the families whose signature is a bare `&[&str]`
/// have no such information to pass on, so their `kinds` reads `literal`
/// throughout. That is exactly as sound as the shipped Rust hooks are, since
/// they receive the same strings — but a body must not read `kinds` as proof
/// of literality on those families. Widening it is a change to the registry's
/// hook signatures, not to this boundary.
pub(super) fn literal_words<'w>(args: &'w [&'w str]) -> Vec<HookWord<'w>> {
    args.iter()
        .map(|value| HookWord {
            value,
            kind: InvocationWordKind::Literal,
        })
        .collect()
}

fn structured_words(args: InvocationArguments<'_>) -> Vec<HookWord<'_>> {
    (0..args.len())
        .map(|index| {
            let word = args.get(index).unwrap_or(InvocationWord::Opaque);
            HookWord {
                value: word.literal().unwrap_or(""),
                kind: word.kind(),
            }
        })
        .collect()
}

pub(super) fn call_of<'w>(words: &'w [HookWord<'w>], version: Option<TclVersion>) -> HookCall<'w> {
    HookCall {
        words,
        version,
        in_event_body: false,
        option: None,
        constraints: None,
        dialect: current_dialect(),
        targets: &[],
        budget: ImplementationBudget::default(),
        depends: &[],
    }
}

pub(super) fn arg_role_thunk<const N: u16>(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let words = literal_words(args);
    match dispatch(
        HookSlot {
            family: HookFamily::ArgRoleResolver,
            index: N,
        },
        &call_of(&words, None),
    ) {
        HookAnswer::Roles(roles) => roles,
        _ => Vec::new(),
    }
}

pub(super) fn command_prefix_thunk<const N: u16>(
    args: CommandPrefixArguments<'_>,
) -> Vec<(u8, AppendedArity)> {
    let words = structured_words(args.words());
    match dispatch(
        HookSlot {
            family: HookFamily::CommandPrefixResolver,
            index: N,
        },
        &call_of(&words, None),
    ) {
        HookAnswer::Prefixes(prefixes) => prefixes,
        _ => Vec::new(),
    }
}

pub(super) fn script_timing_thunk<const N: u16>(
    args: InvocationArguments<'_>,
) -> Vec<(u8, ScriptTiming)> {
    let words = structured_words(args);
    match dispatch(
        HookSlot {
            family: HookFamily::ScriptTimingResolver,
            index: N,
        },
        &call_of(&words, None),
    ) {
        HookAnswer::Timings(timings) => timings,
        _ => Vec::new(),
    }
}

pub(super) fn const_fold_thunk<const N: u16>(args: &[&str]) -> Option<String> {
    let words = literal_words(args);
    match dispatch(
        HookSlot {
            family: HookFamily::ConstFold,
            index: N,
        },
        &call_of(&words, None),
    ) {
        HookAnswer::Fold(value) => Some(value),
        _ => None,
    }
}

pub(super) fn const_fold_versioned_thunk<const N: u16>(
    args: &[&str],
    version: Option<TclVersion>,
) -> Option<String> {
    let words = literal_words(args);
    match dispatch(
        HookSlot {
            family: HookFamily::ConstFoldVersioned,
            index: N,
        },
        &call_of(&words, version),
    ) {
        HookAnswer::Fold(value) => Some(value),
        _ => None,
    }
}

pub(super) fn taint_sink_thunk<const N: u16>(args: &[&str]) -> bool {
    let words = literal_words(args);
    // Silence keeps the finding alive: only an explicit `sink-suppressed`
    // turns the sink off.
    !matches!(
        dispatch(
            HookSlot {
                family: HookFamily::TaintSinkGate,
                index: N,
            },
            &call_of(&words, None),
        ),
        HookAnswer::SinkSuppressed
    )
}

pub(super) fn context_gate_thunk<const N: u16>(
    args: InvocationArguments<'_>,
    in_event_body: bool,
) -> Option<&'static str> {
    args.exact_argv_len()?;
    let words = structured_words(args);
    let call = HookCall {
        words: &words,
        version: None,
        in_event_body,
        option: None,
        constraints: None,
        dialect: current_dialect(),
        targets: &[],
        budget: ImplementationBudget::default(),
        depends: &[],
    };
    match dispatch(
        HookSlot {
            family: HookFamily::ContextGate,
            index: N,
        },
        &call,
    ) {
        HookAnswer::Reject(message) => Some(intern(&message)),
        _ => None,
    }
}

pub(super) fn literal_validator_thunk<const N: u16>(
    args: InvocationArguments<'_>,
) -> LiteralArgumentValidation {
    let words = structured_words(args);
    match dispatch(
        HookSlot {
            family: HookFamily::LiteralArgumentValidator,
            index: N,
        },
        &call_of(&words, None),
    ) {
        HookAnswer::Literal(validation) => validation,
        _ => LiteralArgumentValidation::Valid,
    }
}

pub(super) fn clause_shape_thunk<const N: u16>(
    args: InvocationArguments<'_>,
) -> Option<ClauseShapeIssue> {
    args.exact_argv_len()?;
    let words = structured_words(args);
    match dispatch(
        HookSlot {
            family: HookFamily::ClauseShapeCheck,
            index: N,
        },
        &call_of(&words, None),
    ) {
        HookAnswer::ClauseShape(error) => Some(ClauseShapeIssue::diagnostic(error)),
        _ => None,
    }
}

pub(super) fn option_arity_thunk<const N: u16>(args: &[&str], start: usize) -> OptionValueOutcome {
    let word_facts = literal_words(args);
    let option = OptionCallCtx {
        option: args.get(start.wrapping_sub(1)).copied().unwrap_or(""),
        index: start.saturating_sub(1),
        value_start: start,
    };
    let call = HookCall {
        words: &word_facts,
        version: None,
        in_event_body: false,
        option: Some(option),
        constraints: None,
        dialect: current_dialect(),
        targets: &[],
        budget: ImplementationBudget::default(),
        depends: &[],
    };
    match dispatch(
        HookSlot {
            family: HookFamily::OptionArity,
            index: N,
        },
        &call,
    ) {
        HookAnswer::Consume { words, invalid } => OptionValueOutcome {
            words,
            invalid: invalid.as_deref().map(intern),
        },
        // Silence still consumes one word — `consume 0` is a report, not an
        // abstention.
        _ => OptionValueOutcome {
            words: 1,
            invalid: None,
        },
    }
}

/// The `constraints` family's thunk (E-R14).
///
/// Reached **only** from a spec that declared a `constraints` hook, and only
/// after the declarative relations reported nothing — so a pack that declares
/// none never runs this, and neither does any shipped command.
pub(super) fn constraints_thunk<const N: u16>(
    facts: &crate::spec::OptionFacts<'_>,
) -> Vec<crate::spec::ConstraintReport> {
    // The flattened word view is what `words` / `kinds` / `nwords` answer
    // from; the structured view beside it is what the reading verbs use.
    let mut words: Vec<HookWord<'_>> = Vec::with_capacity(facts.options.len() * 2);
    for (name, value) in facts.options {
        words.push(HookWord {
            value: name,
            kind: InvocationWordKind::Literal,
        });
        if let Some(value) = value {
            words.push(HookWord {
                value,
                kind: InvocationWordKind::Literal,
            });
        }
    }
    for positional in facts.positionals {
        words.push(match positional {
            Some(value) => HookWord {
                value,
                kind: InvocationWordKind::Literal,
            },
            None => HookWord {
                value: "",
                kind: InvocationWordKind::Dynamic,
            },
        });
    }
    let call = HookCall {
        words: &words,
        version: None,
        in_event_body: false,
        option: None,
        constraints: Some(ConstraintCallCtx {
            options: facts.options,
            positionals: facts.positionals,
            complete: facts.complete,
        }),
        dialect: current_dialect(),
        targets: &[],
        budget: ImplementationBudget::default(),
        depends: &[],
    };
    match dispatch(
        HookSlot {
            family: HookFamily::Constraints,
            index: N,
        },
        &call,
    ) {
        HookAnswer::Constraints(reports) => reports,
        _ => Vec::new(),
    }
}

/// The `state_transitions` resolver family's thunk: the body's facts, read
/// against the call's own words.
pub(super) fn state_transition_thunk<const N: u16>(
    arguments: InvocationArguments<'_>,
) -> StateTransitions {
    let words = structured_words(arguments);
    match dispatch(
        HookSlot {
            family: HookFamily::StateTransitionResolver,
            index: N,
        },
        &call_of(&words, None),
    ) {
        HookAnswer::Transitions(stated) => pack_transitions(arguments, &stated),
        _ => StateTransitions::default(),
    }
}

/// The facts a resolver body's verbs state, in call order. A fact naming a
/// word past the call names nothing and is dropped; a fact naming a computed
/// word abstains and widens [`VARIABLE_ALIAS_DOMAINS`] for that word — the
/// family's contract that an abstention widens rather than narrows.
pub(super) fn pack_transitions(
    arguments: InvocationArguments<'_>,
    stated: &[PackTransition],
) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    for &fact in stated {
        match stated_alias(arguments, fact) {
            Ok(alias) => transitions.push(StateTransition::VariableCellAlias(alias)),
            Err(Some(computed)) => transitions.widen(computed, VARIABLE_ALIAS_DOMAINS),
            Err(None) => {}
        }
    }
    transitions
}

/// The alias `fact` states over `arguments`, or why it states none:
/// `Err(Some(word))` for the first computed word it names, `Err(None)` for a
/// word past the call.
fn stated_alias(
    arguments: InvocationArguments<'_>,
    fact: PackTransition,
) -> Result<VariableCellAliasTransition, Option<TransitionSubject>> {
    let literal = |index: usize| match TransitionSubject::from_argument(arguments, index) {
        Some(subject) if subject.literal().is_some() || subject.native_bytes().is_some() => {
            Ok(subject)
        }
        other => Err(other),
    };
    Ok(match fact {
        PackTransition::Alias {
            local,
            target,
            level,
        } => VariableCellAliasTransition {
            destination: VariableAliasDestination::CurrentNamespaceOrLocal,
            local: literal(local)?,
            target: VariableAliasTarget::CallerSelectedFrame {
                frame: match level {
                    None => CallerFrameSelection::DefaultCaller,
                    Some(level) => CallerFrameSelection::Explicit(literal(level)?),
                },
                variable: literal(target)?,
            },
            writes_value: false,
            words: AliasWords { local, target },
        },
        PackTransition::NamespaceVariable { name } => {
            let variable = literal(name)?;
            VariableCellAliasTransition {
                destination: VariableAliasDestination::CurrentNamespaceOrLocal,
                local: crate::state_transition::local_alias_name(
                    &variable,
                    name,
                    VariableAliasNamePurpose::NamespaceVariable,
                    arguments.dialect(),
                )
                .ok_or(None)?,
                target: VariableAliasTarget::CurrentNamespace { variable },
                writes_value: false,
                words: AliasWords::same(name),
            }
        }
    })
}
