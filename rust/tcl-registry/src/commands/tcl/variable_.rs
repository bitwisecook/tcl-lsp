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

//! `variable` — create and initialise a namespace variable.

use crate::hooks::LoweringHookId;
use crate::prelude::*;
use crate::state_transition::local_alias_name;
use tcl_dialect::model::{SpecSurface};
use tcl_dialect::surface;
use tcl_dialect::model::Family;

const VARIABLE_TRANSITION_DOMAINS: &[StateTransitionDomain] = &[
    StateTransitionDomain::VariableCells,
    StateTransitionDomain::Namespaces,
    StateTransitionDomain::VariableTraces,
];

const VARIABLE_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    resolver: Some(variable_state_transitions),
    argument_shape: StateTransitionArgumentShape::Positional,
    dynamic_widening: &[StateTransitionWideningRule {
        operands: StateTransitionOperandLayout::Strided {
            first: 0,
            stride: 2,
        },
        domains: VARIABLE_TRANSITION_DOMAINS,
    }],
    // The transition records cell identity only. `variable name value` also
    // writes the cell's value, so its legacy variable-store effect remains.
    effect_coverage: TransitionEffectCoverage::NONE,
    // Values and later name/value pairs can fail after earlier namespace
    // aliases have been installed.
    commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
};

fn variable_state_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    for argument_index in (0..arguments.len()).step_by(2) {
        let Some(variable) = TransitionSubject::from_argument(arguments, argument_index) else {
            continue;
        };
        transitions.push(StateTransition::VariableCellAlias(
            VariableCellAliasTransition {
                local: local_alias_name(&variable),
                target: VariableAliasTarget::CurrentNamespace { variable },
            },
        ));
    }
    transitions
}

const FORMS: &[FormSpec] = &[
    // Tcl 8.6 dropped the `if (objc < 2) Tcl_WrongNumArgs(...)` guard that
    // `Tcl_VariableObjCmd` (generic/tclVar.c) has in 8.4/8.5, splitting the
    // SYNOPSIS into these two lines and making a bare `variable` call a
    // legal no-op — confirmed against the upstream source at tags
    // core-8-6-14/core-9-0-1/core-9-1-a1 (no guard) vs
    // core-8-4-20/core-8-5-19 (guard present), and empirically against
    // tclsh 8.6.14 (`catch {variable}` -> 0). Expect, Synopsys, and
    // Cadence embed that 8.6-based Tcl core
    // (`DialectSet::expr_grammar_base_version`), so they follow this pair
    // of forms too.
    FormSpec {
        synopsis: "variable name",
        surface: Some(surface![SpecSurface::core_in(Family::Tcl, &[("8.6", Some("9.2"))]), SpecSurface::package("expect")]),
        ..FormSpec::DEFAULT
    },
    FormSpec {
        synopsis: "variable ?name value...?",
        surface: Some(surface![SpecSurface::core_in(Family::Tcl, &[("8.6", Some("9.2"))]), SpecSurface::package("expect")]),
        ..FormSpec::DEFAULT
    },
    // Tcl 8.4 and 8.5 require at least one `name`: `Tcl_VariableObjCmd`
    // opens with `if (objc < 2) Tcl_WrongNumArgs(...)` in both versions,
    // so a bare `variable` is a hard "wrong # args" error there (matching
    // those versions' manpage SYNOPSIS, one combined `variable ?name
    // value...? name ?value?` line). iRules, iApps, tmsh, and the
    // Xilinx/Quartus/Mentor EDA shells embed an 8.4- or 8.5-based Tcl core
    // (`DialectSet::expr_grammar_base_version`), so they inherit the same
    // requirement.
    FormSpec {
        synopsis: "variable ?name value...? name ?value?",
        surface: Some(surface![SpecSurface::core_in(Family::Tcl, &[("8.4", Some("8.6"))]), SpecSurface::core(Family::F5Irules), SpecSurface::package("iapps"), SpecSurface::package("tmsh")]),
        ..FormSpec::DEFAULT
    },
];

/// Command spec for `variable`.
///
/// Tcl 8.4 and 8.5 share a byte-identical variable.n: a single-line
/// SYNOPSIS `variable ?name value...? name ?value?`, and
/// `Tcl_VariableObjCmd` (generic/tclVar.c) opens with `if (objc < 2)
/// Tcl_WrongNumArgs(...)`, so at least one `name` is required there. Tcl
/// 8.6 restructured the SYNOPSIS into two lines (`variable name` /
/// `variable ?name value...?`) and dropped that argument-count guard, so a
/// bare `variable` call became a legal no-op from 8.6 onward — the one
/// substantive behavioural delta found across the five versions audited
/// here; every other DESCRIPTION sentence is identical in substance across
/// all five, occasionally just reworded (e.g. how the text contrasts
/// `variable` with `global`). Tcl 9.0's variable.html differs from 8.6's
/// only in NAME-line capitalisation and doc-anchor formatting; Tcl 9.1's is
/// byte-for-byte identical to 9.0's.
/// `variable name ?value name value ...?` — the *name* sits at every even
/// argument; the interleaved values are ordinary data (issue #1185).
static REPEATED: &[RepeatedArgLayout] = &[RepeatedArgLayout::strided(ArgRole::VarWrite, 0, 2)];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "variable",
        // Present and unrestricted: iRules enables `variable`, so it
        // carries the `IRULES` bit explicitly (`ALL_TCL.union(IRULES)`) and
        // resolves under the bare `IRULES` mask; no dialect pack under
        // `tcl-registry/src/commands/` (irules/, iapps/, tk/, expect/, the
        // eda_*/ vendor packs, itcl/, bpf/) defines its own `variable`
        // spec either, so every dialect inherits this one unmodified.
        // Only the minimum argument count narrows per Tcl version —
        // captured on the individual FORMS entries above, not as a
        // whole-command dialect gate.
        surface: Some(surface![SpecSurface::core_in(Family::Tcl, &[("8.4", Some("9.2"))]), SpecSurface::core(Family::F5Irules)]),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::CREATES_BARRIER
            | Traits::CREATES_SCOPE_ALIAS
            | Traits::CREATES_DYNAMIC_BARRIER
            | Traits::FRAME_HASH_BUILTIN,
        // `variable ?name value...? name ?value?` — Tcl 8.6+ (and
        // Expect/Synopsys/Cadence, which embed that 8.6-family core)
        // accept zero args as a no-op: `Tcl_VariableObjCmd`
        // (generic/tclVar.c) has had no `Tcl_WrongNumArgs` call since 8.6
        // (confirmed against tclsh 8.6.14: `catch {variable}` -> 0, and
        // against the upstream source at
        // core-8-6-14/core-9-0-1/core-9-1-a1). Tcl 8.4 and 8.5 (and
        // iRules/iApps/tmsh/Xilinx/Quartus/Mentor, which embed that older
        // core) open with `if (objc < 2) Tcl_WrongNumArgs(...)` instead
        // (confirmed against core-8-4-20/core-8-5-19), so a bare
        // `variable` is a hard "wrong # args" error there. `Arity` has no
        // per-dialect variant, so this keeps the loosest (8.6+) bound
        // here rather than false-positiving E002 on the modern/common
        // case; see FORMS above for the version-split synopsis text.
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
            summary: "Create and initialize a namespace variable, or link to one from inside a procedure.",
            synopsis: &["variable name", "variable ?name value...?"],
            snippet: "Normally used inside a namespace eval body to create one or more variables in a namespace: each name is initialized with the value that follows it, and the value for the final name is optional. A name that does not yet exist is created — initialized if value is given, left undefined otherwise; an existing variable is updated only when value is given, and left unchanged otherwise. name is usually unqualified, creating the variable in the current namespace, but a name with namespace qualifiers creates it in the namespace it names. A declared-but-undefined variable is visible to namespace which but not to info exists. Used inside a procedure body, variable instead creates a local variable linked to the corresponding namespace variable (and so listed by info vars) — like global, but resolved relative to the current namespace rather than always the global (::) one; any value given still initializes or updates the linked namespace variable itself. name can never address a single array element: reference the whole array with no value, then populate its elements afterward with set or array set. Tcl 8.4 and 8.5 require at least one name argument — a bare variable with none is a \"wrong # args\" error there. Tcl 8.6 dropped that requirement, so a bare variable call is a legal no-op from 8.6 onward.",
            source: "Tcl variable(n)",
            examples: "namespace eval foo {\n    variable bar 12345\n}\n\nnamespace eval someNS {\n    variable someAry\n    array set someAry {\n        someName  someValue\n        otherName otherValue\n    }\n}\n\nnamespace eval foo {\n    proc spong {} {\n        variable bar\n        puts \"bar is $bar\"\n        variable ::someNS::someAry\n        parray someAry\n    }\n}",
            return_value: "The empty string.",
        }),
        lowering_hook: Some(LoweringHookId::Variable),
        forms: FORMS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::Variable),
        // A value-bearing `variable name value` can run variable traces;
        // until value writes are transition-modelled, leave its world
        // declaration open rather than falsely claiming `EMPTY`.
        state_transitions: Some(VARIABLE_TRANSITIONS),
        ..CommandSpec::DEFAULT
    }
}
