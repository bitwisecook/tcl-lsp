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

//! `global` — access global variables.

use crate::hooks::{CodegenHookId, LoweringHookId};
use crate::prelude::*;
use crate::state_transition::local_alias_name;
use tcl_dialect::model::{SpecSurface};
use tcl_dialect::surface;
use tcl_dialect::model::Family;

const GLOBAL_TRANSITION_DOMAINS: &[StateTransitionDomain] = &[
    StateTransitionDomain::VariableCells,
    StateTransitionDomain::Namespaces,
    StateTransitionDomain::VariableTraces,
];

const GLOBAL_EFFECT_COVERAGE: &[TransitionEffectCoverage] = &[TransitionEffectCoverage {
    source: WorldEffectWriteSource::LegacySideEffect(SideEffectTarget::Variable),
    domains: &[WorldStateDomain::VariableStore],
}];

const GLOBAL_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    resolver: Some(global_state_transitions),
    argument_shape: StateTransitionArgumentShape::Independent,
    dynamic_widening: &[StateTransitionWideningRule {
        operands: StateTransitionOperandLayout::EveryArgument,
        domains: GLOBAL_TRANSITION_DOMAINS,
    }],
    effect_coverage: GLOBAL_EFFECT_COVERAGE,
    // Each alias is installed in turn, so a later argument may fail after a
    // preceding link has become observable.
    commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
};

fn global_state_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    for argument_index in 0..arguments.len() {
        let Some(variable) = TransitionSubject::from_argument(arguments, argument_index) else {
            continue;
        };
        transitions.push(StateTransition::VariableCellAlias(
            VariableCellAliasTransition {
                local: local_alias_name(&variable),
                target: VariableAliasTarget::Global { variable },
            },
        ));
    }
    transitions
}

const FORMS: &[FormSpec] = &[
    // Tcl 8.6 dropped the `Tcl_WrongNumArgs` check that `Tcl_GlobalObjCmd`
    // (generic/tclVar.c) has in 8.4/8.5, so every varname became optional
    // — the manpage's own SYNOPSIS changed from `global varname ?varname
    // ...?` to `global ?varname ...?` at that same boundary (8.6 TclCmd
    // manpage vs. 8.4/8.5). Expect, Synopsys, and Cadence embed an
    // 8.6-based Tcl core (`DialectSet::expr_grammar_base_version`), so
    // they follow this form too.
    FormSpec {
        synopsis: "global ?varname ...?",
        surface: Some(surface![SpecSurface::core_in(Family::Tcl, &[("8.6", Some("9.2"))]), SpecSurface::package("expect")]),
        ..FormSpec::DEFAULT
    },
    // Tcl 8.4 and 8.5 require at least one varname: `Tcl_GlobalObjCmd`
    // opens with `if (objc < 2) Tcl_WrongNumArgs(...)` in both versions,
    // so a bare `global` is a hard "wrong # args" error there (matching
    // those versions' manpage SYNOPSIS, `global varname ?varname ...?`).
    // iRules, iApps, tmsh, and the Xilinx/Quartus/Mentor EDA shells embed
    // an 8.4- or 8.5-based Tcl core (`DialectSet::expr_grammar_base_version`),
    // so they inherit the same requirement.
    FormSpec {
        synopsis: "global varname ?varname ...?",
        surface: Some(surface![SpecSurface::core_in(Family::Tcl, &[("8.4", Some("8.6"))]), SpecSurface::core(Family::F5Irules), SpecSurface::package("iapps"), SpecSurface::package("tmsh")]),
        ..FormSpec::DEFAULT
    },
];

/// Command spec for `global`.
/// `global name ?name ...?` declares *every* argument, not just the first:
/// the unbounded tail a fixed index table cannot express (issue #1185).
static REPEATED: &[RepeatedArgLayout] = &[RepeatedArgLayout::every(ArgRole::VarWrite, 0)];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "global",
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::CREATES_BARRIER
            | Traits::CREATES_SCOPE_ALIAS
            | Traits::ALIASES_GLOBAL
            | Traits::CREATES_DYNAMIC_BARRIER
            | Traits::FRAME_HASH_BUILTIN,
        // `global ?varName ...?` in Tcl 8.6/9.0/9.1 (and Expect/Synopsys/
        // Cadence, which embed that 8.6-family core) — zero args is a
        // valid no-op there: `Tcl_GlobalObjCmd` (generic/tclVar.c) has no
        // `Tcl_WrongNumArgs` call from 8.6 onward, so its `for (i=1; …)`
        // loop simply doesn't run.
        //
        // Tcl 8.4 and 8.5 (and iRules/iApps/tmsh/Xilinx/Quartus/Mentor,
        // which embed that older core) are stricter: `Tcl_GlobalObjCmd`
        // opens with `if (objc < 2) Tcl_WrongNumArgs(...)`, so a bare
        // `global` is a hard "wrong # args" error there — even outside a
        // proc body, since that check runs before the "not in a proc"
        // early return. `Arity` has no per-dialect variant, so this keeps
        // the loosest (8.6+) bound here rather than false-positiving E002
        // on the common case; see FORMS above for the version-split
        // synopsis text.
        arity: Arity::any(),
        arg_roles: &[(0, ArgRole::VarWrite)],
        repeated_args: REPEATED,
        assigns_variable_at: Some(0),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Access global variables",
            synopsis: &["global ?varname ...?"],
            snippet: "Executing global outside a proc body has no effect there (a no-op), once its arity requirement, if any, is met. Inside a proc body it creates local variables linked to the corresponding global (::-namespace) variables; like upvar-created links, these do not appear in info locals. A namespace-qualified varname is linked under its unqualified tail name, per namespace tail. Tcl 8.4 and 8.5 require at least one varname — a bare global with none is a \"wrong # args\" error even outside a proc body; Tcl 8.6 dropped that requirement, making global alone a legal no-op in every context. Since Tcl 8.5, varname must be a plain scalar name: one that looks like an array element (e.g. a(b)) is rejected as an error.",
            source: "Tcl global(n)",
            examples: "proc reset {} {\n    global a::x\n    set x 0\n}",
            return_value: "The empty string.",
        }),
        lowering_hook: Some(LoweringHookId::Global),
        codegen_hook: Some(CodegenHookId::Global),
        forms: FORMS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::Global),
        world_effects: Some(WorldEffectDescriptor::EMPTY),
        state_transitions: Some(GLOBAL_TRANSITIONS),
        ..CommandSpec::DEFAULT
    }
}
