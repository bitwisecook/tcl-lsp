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

//! Definition bodies, procedure parameter roles, static lists and the `proc` hook declaration.

use super::*;

#[test]
fn source_enum_values_and_definition_bodies_use_the_selected_invocation() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let registry = CommandRegistry::build_default();
    let dialect = tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_1);
    let query = Some(SurfaceQuery::core(Family::Tcl, "9.1"));
    let args = [
        crate::InvocationWord::Literal("is"),
        crate::InvocationWord::Literal("alnum"),
        crate::InvocationWord::Dynamic,
    ];
    let selected = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(crate::InvocationWord::Literal("string"), &args)
                .with_dialect(dialect),
            query,
        )
        .resolved()
        .unwrap();
    assert_eq!(selected.authored_source_enum_arguments(), vec![1]);
    for (args, definition_body) in [
        (vec!["C", "method pick {} {}"], true),
        (vec!["C", "method", "pick", "{}", "{}"], false),
    ] {
        let words = args
            .iter()
            .map(|word| crate::InvocationWord::Literal(word))
            .collect::<Vec<_>>();
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("oo::define"),
                    &words,
                )
                .with_dialect(dialect),
                query,
            )
            .resolved()
            .unwrap();
        assert_eq!(
            selected.authored_source_definition_body_grammar().is_some(),
            definition_body,
            "{args:?}"
        );
    }
    let args = [
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Literal("one"),
        crate::InvocationWord::Literal("set x 1"),
        crate::InvocationWord::Literal("default"),
        crate::InvocationWord::Literal("set x 2"),
    ];
    let selected = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(crate::InvocationWord::Literal("switch"), &args)
                .with_dialect(dialect),
            query,
        )
        .resolved()
        .unwrap();
    let clauses = selected.authored_source_inline_case_clauses().unwrap();
    assert_eq!(clauses.len(), 2);
    assert_eq!(clauses[0].pattern_index, 1);
    assert_eq!(clauses[0].body_index, Some(2));
    assert_eq!(clauses[1].body_index, Some(4));
}

#[test]
fn authored_source_procedure_roles_do_not_supply_native_definition_acceptance() {
    // Implementation contract: naming.vendor.original-registry-metadata
    // docs/design/analysis/name-resolution-proofs/vendor-original-registry-metadata.md
    let registry = CommandRegistry::build_default();
    let arguments = [
        crate::InvocationWord::Literal("p"),
        crate::InvocationWord::Literal("arg"),
        crate::InvocationWord::Dynamic,
    ];
    let selected = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(crate::InvocationWord::Literal("proc"), &arguments),
            None,
        )
        .resolved()
        .unwrap();
    let (roles, complete) = selected.authored_source_argument_roles();
    assert!(complete);
    assert!(roles.contains(&(1, ArgRole::ParamList)));
    assert!(roles.contains(&(2, ArgRole::Body)));
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    assert_eq!(
        selected.authored_source_procedure_arguments(),
        Some(AuthoredSourceProcedureArguments {
            name: 0,
            parameters: 1,
            body: 2,
        })
    );
    for facts in [selected.facts(), selected.facts_after_success()] {
        assert!(!facts.arg_roles_complete);
        assert_eq!(facts.arg_roles.len(), 0);
    }
    let opaque = [
        crate::InvocationWord::Literal("p"),
        crate::InvocationWord::Literal("arg"),
        crate::InvocationWord::Opaque,
    ];
    let selected = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(crate::InvocationWord::Literal("proc"), &opaque),
            None,
        )
        .resolved()
        .unwrap();
    assert!(selected.authored_source_procedure_arguments().is_none());
    let expanded = [crate::InvocationWord::Expanded];
    let selected = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(crate::InvocationWord::Literal("proc"), &expanded),
            None,
        )
        .resolved()
        .unwrap();
    assert!(selected.words.arguments().exact_argv_len().is_none());
    assert!(selected.authored_source_procedure_arguments().is_none());
    assert!(!selected.facts().arg_roles_complete);
}

#[test]
fn authored_source_static_list_position_rejects_duplicate_or_overlapping_roles() {
    for (roles, arguments, expected) in [
        (
            &[
                (0, ArgRole::Name),
                (1, ArgRole::ParamList),
                (2, ArgRole::StaticVarList),
                (3, ArgRole::Body),
            ][..],
            &["p", "arg", "{n 0}", "BODY"][..],
            Some(Some(2)),
        ),
        (
            &[
                (0, ArgRole::Name),
                (1, ArgRole::ParamList),
                (2, ArgRole::StaticVarList),
                (3, ArgRole::StaticVarList),
                (4, ArgRole::Body),
            ][..],
            &["p", "arg", "{n 0}", "{m 0}", "BODY"][..],
            None,
        ),
        (
            &[
                (0, ArgRole::Name),
                (1, ArgRole::ParamList),
                (1, ArgRole::StaticVarList),
                (2, ArgRole::Body),
            ][..],
            &["p", "arg", "BODY"][..],
            None,
        ),
    ] {
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "source-static-definer",
            arity: Arity::exact(u16::try_from(arguments.len()).unwrap()),
            arg_roles: roles,
            traits: Traits::DEFINES_PROCEDURE | Traits::DEFERS_BODY,
            procedure_definition: Some(
                crate::native_procedure::NativeProcedureDefinitionSpec::Core,
            ),
            ..CommandSpec::DEFAULT
        });
        let selected = registry
            .resolve_invocation("source-static-definer", arguments, None)
            .unwrap();
        assert_eq!(
            selected
                .authored_source_procedure_static_list()
                .map(|shape| match shape {
                    AuthoredSourceProcedureStaticList::Absent => None,
                    AuthoredSourceProcedureStaticList::Argument(at) => Some(at),
                }),
            expected
        );
        assert!(selected.authored_source_procedure_arguments().is_some());
        assert!(!matches!(
            selected.facts().procedure_definition,
            Some(crate::native_procedure::NativeProcedureDefinitionSelection::Valid(_))
        ));
    }
}

const PROC_SOURCE_ROLES: &[(u8, ArgRole)] = &[
    (0, ArgRole::Name),
    (1, ArgRole::ParamList),
    (2, ArgRole::Body),
];

const PROC_CONFLICTING_SOURCE_ROLES: &[(u8, ArgRole)] = &[
    (0, ArgRole::Name),
    (1, ArgRole::Body),
    (2, ArgRole::ParamList),
];

type ProcSourceCase = (
    Option<crate::hooks::AnalyserHookId>,
    Option<CommandTableEffect>,
    &'static [(u8, ArgRole)],
    bool,
);

fn authored_proc_source_cases() -> [ProcSourceCase; 4] {
    [
        (
            Some(crate::hooks::AnalyserHookId::Proc),
            Some(CommandTableEffect::DefinesProcedure),
            PROC_SOURCE_ROLES,
            true,
        ),
        (
            None,
            Some(CommandTableEffect::DefinesProcedure),
            PROC_SOURCE_ROLES,
            false,
        ),
        (
            Some(crate::hooks::AnalyserHookId::Proc),
            None,
            PROC_SOURCE_ROLES,
            false,
        ),
        (
            Some(crate::hooks::AnalyserHookId::Proc),
            Some(CommandTableEffect::DefinesProcedure),
            PROC_CONFLICTING_SOURCE_ROLES,
            false,
        ),
    ]
}

#[test]
fn authored_proc_hook_declaration_requires_exact_source_roles_and_transition() {
    for (hook, transition, roles, expected) in authored_proc_source_cases() {
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "source-definer",
            arity: Arity::exact(3),
            arg_roles: roles,
            analyser_hook: hook,
            command_table_effect: transition,
            ..CommandSpec::DEFAULT
        });
        let selected = registry
            .resolve_invocation("source-definer", &["provided", "one", ""], None)
            .unwrap();
        assert_eq!(
            selected.authored_source_procedure_arguments().is_some(),
            expected
        );
        if expected {
            assert_eq!(
                selected.authored_source_script_timing_at(2),
                Some(crate::ScriptTiming::Deferred)
            );
            assert_eq!(
                selected.semantics.script_metadata.timing_at(&selected, 2),
                Some(crate::ScriptTiming::SameInvocation)
            );
        }
        assert!(selected.semantics.procedure_definition.is_none());
        assert!(selected.facts().procedure_definition.is_none());
        assert!(
            selected
                .facts_after_success()
                .procedure_definition
                .is_none()
        );
        assert!(selected.semantics.native_compilation.is_none());
        let wrong_arity = registry
            .resolve_invocation("source-definer", &["provided", "one"], None)
            .unwrap();
        assert!(wrong_arity.authored_source_procedure_arguments().is_none());
        let arguments = [crate::InvocationWord::Expanded];
        let expanded = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("source-definer"),
                    &arguments,
                ),
                None,
            )
            .resolved()
            .unwrap();
        assert!(expanded.authored_source_procedure_arguments().is_none());
    }
    let registry = {
        fn immediate(_: crate::InvocationArguments<'_>) -> Vec<(u8, crate::ScriptTiming)> {
            vec![(2, crate::ScriptTiming::SameInvocation)]
        }
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "conflicting-source-definer",
            arity: Arity::exact(3),
            arg_roles: PROC_SOURCE_ROLES,
            analyser_hook: Some(crate::hooks::AnalyserHookId::Proc),
            command_table_effect: Some(CommandTableEffect::DefinesProcedure),
            script_timing_resolver: Some(immediate),
            ..CommandSpec::DEFAULT
        });
        registry
    };
    let selected = registry
        .resolve_invocation("conflicting-source-definer", &["provided", "one", ""], None)
        .unwrap();
    assert!(selected.authored_source_procedure_arguments().is_none());
    assert_eq!(
        selected.authored_source_script_timing_at(2),
        Some(crate::ScriptTiming::SameInvocation)
    );
}
