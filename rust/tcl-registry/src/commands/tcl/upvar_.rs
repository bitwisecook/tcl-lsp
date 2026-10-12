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

//! `upvar` — create link to variable in a different stack frame.

use crate::StateTransitionWidening;
use crate::hooks::{CodegenHookId, LoweringHookId};
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const UPVAR_FRAME_EFFECT: FrameEffectSpec = FrameEffectSpec::UPVAR;

const UPVAR_REPEATED_ARGS: &[RepeatedArgLayout] = &[RepeatedArgLayout {
    optional_leading_word: true,
    ..RepeatedArgLayout::strided(ArgRole::VarWrite, 1, 2)
}];

const UPVAR_TRANSITION_DOMAINS: &[StateTransitionDomain] = &[
    StateTransitionDomain::VariableCells,
    StateTransitionDomain::VariableTraces,
];

const UPVAR_EFFECT_COVERAGE: &[TransitionEffectCoverage] = &[
    TransitionEffectCoverage {
        source: WorldEffectWriteSource::LegacyFrame,
        domains: &[WorldStateDomain::VariableStore],
    },
    TransitionEffectCoverage {
        source: WorldEffectWriteSource::LegacySideEffect(SideEffectTarget::Variable),
        domains: &[WorldStateDomain::VariableStore],
    },
];

const UPVAR_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    success_resolver: Some(upvar_success_transitions),
    resolver: Some(upvar_state_transitions),
    argument_shape: StateTransitionArgumentShape::Positional,
    // Each alias transition already carries its unknown source/destination.
    // Registering a link does not write other cells or erase their traces.
    dynamic_widening: &[],
    effect_coverage: UPVAR_EFFECT_COVERAGE,
    // Alias pairs are processed in order and can be observed through traces
    // before a later pair fails.
    commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
};

fn upvar_state_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    upvar_transitions_for_layout(arguments, UPVAR_FRAME_EFFECT.resolve_arguments(arguments))
}

fn upvar_success_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    upvar_transitions_for_layout(
        arguments,
        UPVAR_FRAME_EFFECT.successful_layout(arguments).layout,
    )
}

fn upvar_transitions_for_layout(
    arguments: InvocationArguments<'_>,
    layout: crate::frame_effect::FrameArgumentResolution,
) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    let level_word_len = match layout {
        crate::frame_effect::FrameArgumentResolution::Valid { level_word_len, .. } => {
            level_word_len
        }
        crate::frame_effect::FrameArgumentResolution::Invalid => return transitions,
        crate::frame_effect::FrameArgumentResolution::Unknown => {
            if let Some(subject) = TransitionSubject::from_argument(arguments, 0) {
                transitions.push(StateTransition::Widen(StateTransitionWidening {
                    domains: UPVAR_TRANSITION_DOMAINS.to_vec(),
                    subject,
                }));
            }
            return transitions;
        }
    };

    let frame = match level_word_len {
        0 => CallerFrameSelection::DefaultCaller,
        1 => match TransitionSubject::from_argument(arguments, 0) {
            Some(level) => CallerFrameSelection::Explicit(level),
            None => return transitions,
        },
        _ => return transitions,
    };

    for other_index in (level_word_len..arguments.len()).step_by(2) {
        let (Some(variable), Some(local)) = (
            TransitionSubject::from_argument(arguments, other_index),
            TransitionSubject::from_argument(arguments, other_index + 1),
        ) else {
            continue;
        };
        transitions.push(StateTransition::VariableCellAlias(
            VariableCellAliasTransition {
                destination:
                    crate::state_transition::VariableAliasDestination::CurrentNamespaceOrLocal,
                local,
                target: VariableAliasTarget::CallerSelectedFrame {
                    frame: frame.clone(),
                    variable,
                },
                writes_value: false,
                words: crate::state_transition::AliasWords {
                    local: other_index + 1,
                    target: other_index,
                },
            },
        ));
    }
    transitions
}

// `upvar ?level? otherVar myVar ?otherVar myVar ...?` is the exact
// SYNOPSIS text in every one of the Tcl 8.4, 8.5, 8.6, 9.0, and 9.1
// upvar.n manpages — unlike `global`/`return`/`open`, this command's
// invocation shape has never changed, so a single dialect-unrestricted
// FormSpec covers every version (9.0's and 9.1's upvar.html bodies are
// themselves byte-for-byte identical, bar the doc-anchor version string
// in the page banner).
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "upvar ?level? otherVar myVar ?otherVar myVar ...?",
    ..FormSpec::DEFAULT
}];

/// Command spec for `upvar`.
///
/// `upvar`'s SYNOPSIS and minimum arity are unchanged across Tcl 8.4,
/// 8.5, 8.6, 9.0, and 9.1, so `FORMS`/`arity` need no per-version gate.
/// Two real behavioural facts do differ by version; both are folded into
/// the hover snippet's prose rather than a dialect gate, since the
/// command and its argument shape stay universal and only a property of
/// an existing argument changes:
///   - A `myVar` that looks like an array element (`a(b)`) is a hard
///     error from Tcl 8.5 onward ("can't create a scalar variable that
///     looks like an array element" — confirmed on tclsh 8.6.14); the
///     Tcl 8.4 manpage instead documents that "a regular variable is
///     created" there, silently.
///   - When `otherVar` names an array element, a whole-array trace on
///     that array does not fire for accesses through `myVar` on Tcl 8.4
///     through 8.6 (confirmed on tclsh 8.6.14: only a trace on the
///     specific element fires). The Tcl 9.0 and 9.1 manpages instead
///     document that the element name is passed as the trace
///     procedure's second argument "in case of traces set on an entire
///     array" — i.e. such a whole-array trace now does fire.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "upvar",
        native_result: Some(crate::native_result::NativeResultContract::EmptyString),
        // Native compileProc registration: pinned C Tcl 8.4.20–9.1.0 tclBasic.c.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::Upvar,
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        completion: Some(crate::completion::CompletionDescriptor::exact(&[
            crate::completion::CompletionCode::Ok,
            crate::completion::CompletionCode::Error,
        ])),
        runtime_backing: RuntimeBacking::shipped("upvar"),
        // A pure variable-scoping primitive — no filesystem, process, or
        // network access — so every dialect that hosts a real Tcl core carries
        // it unmodified, the same reasoning `global`/`variable` use for their
        // own unrestricted `dialects`. iRules enables it, so it carries an
        // iRules row explicitly (`ALL_TCL.union(IRULES)`) and resolves under
        // the bare `IRULES` mask; and no dialect command pack (irules/,
        // iapps/, itcl/, tk/, expect/, the eda_*/ vendor directories) defines
        // its own "upvar" spec to add or restrict a form.
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::CREATES_BARRIER
            | Traits::CREATES_SCOPE_ALIAS
            | Traits::CREATES_DYNAMIC_BARRIER
            | Traits::FRAME_HASH_BUILTIN
            // The aliased frame is picked by the runtime level argument
            // (default 1), so any enclosing proc's locals can be read or
            // written by name — renaming transforms must treat every scope
            // as observable while an `upvar` exists.
            | Traits::ALIASES_CALLER_FRAME,
        // At least one otherVar/myVar pair is required — `upvar` and
        // `upvar x` (0 or 1 trailing word) both raise "wrong # args",
        // `upvar x y` does not (tclsh 8.6.14). No upper bound: any
        // number of additional pairs is legal. Identical in every
        // version, since the SYNOPSIS itself never changed (see
        // `FORMS`).
        arity: Arity::at_least(2),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Create link to variable in a different stack frame",
            synopsis: &["upvar ?level? otherVar myVar ?otherVar myVar ...?"],
            snippet: "Links each myVar in the current procedure to the variable named otherVar in the call frame named by level (or the global scope, when level is #0); afterwards, reading, writing, or unsetting myVar reads, writes, or unsets otherVar directly. otherVar need not exist beforehand — it is created, just like an ordinary variable, the first time myVar is referenced. myVar must not already exist as a variable when upvar runs, and is always treated as a plain variable name, never an array element: since Tcl 8.5, a myVar that looks like an array element (e.g. a(b)) is a hard error (\"can't create a scalar variable that looks like an array element\"); Tcl 8.4 instead silently created an ordinary scalar variable literally named that. otherVar itself may be a scalar, a whole array, or a single array element. level takes any uplevel-style form — a plain integer counts call frames up from the current one (each namespace eval body also counts as one frame), #N is an absolute frame number, and it defaults to 1 (the immediate caller) when level is omitted (argument parity selects it in Tcl 8.6+, while Tcl 8.4/8.5 probe a leading digit or #); a level outside the current call stack is a \"bad level\" error. There is no way to remove an upvar link short of leaving the procedure that created it, though a later upvar call can retarget myVar to a different otherVar. A variable trace on otherVar fires on accesses through myVar but is passed myVar's name, not otherVar's; when otherVar names one element of an array, Tcl 8.4 through 8.6 do not fire a whole-array trace on that array for accesses through myVar (only a trace on that specific element fires), while Tcl 9.0 and 9.1 pass the element name as the trace procedure's second argument, so a whole-array trace does observe the access.",
            source: "Tcl upvar(n)",
            examples: "proc add2 name {\n    upvar $name x\n    set x [expr {$x + 2}]\n}\nset n 5\nadd2 n\nputs $n\n\n# level defaults to 1 (the caller); an explicit level reaches further up the stack\nproc decr {varName {decrement 1}} {\n    upvar 1 $varName var\n    incr var [expr {-$decrement}]\n}\n\n# level #0 links straight to the global scope, regardless of call depth\nproc bumpCounter {} {\n    upvar #0 counter c\n    incr c\n}",
            return_value: "The empty string.",
        }),
        // Presence is dialect-aware: pre-8.6 probes the leading word,
        // while 8.6+ and Jim use count parity. All role/transition consumers
        // share the registry frame grammar.
        frame_effect: Some(UPVAR_FRAME_EFFECT),
        repeated_args: UPVAR_REPEATED_ARGS,
        lowering_hook: Some(LoweringHookId::Upvar),
        native_lowering: Some(NativeLowering::Scope(ScopeKind::Upvar)),
        codegen_hook: Some(CodegenHookId::Upvar),
        forms: FORMS,
        xc_translatable: Some(false),
        world_effects: Some(WorldEffectDescriptor::EMPTY),
        state_transitions: Some(UPVAR_TRANSITIONS),
        ..CommandSpec::DEFAULT
    }
}
