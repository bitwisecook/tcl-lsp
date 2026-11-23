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

//! Argument roles: logical role projection, the Jim core layout, optional, repeating, resolver-backed and cardinality roles.

use super::*;

#[test]
fn original_logical_case_role_projection_keeps_selected_options_and_body_residuals() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    use crate::InvocationWord::{Dynamic, Expanded, KnownBytes, Literal};
    let current = crate::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    for (values, expected) in [
        (
            vec![
                Literal("--"),
                Dynamic,
                Literal("café {BODY} default {FALLBACK}"),
            ],
            Some((false, false, 2)),
        ),
        (
            vec![Literal("-exact"), Dynamic, Literal("café {BODY}")],
            Some((false, true, 2)),
        ),
        (
            vec![Dynamic, Literal("café {BODY}")],
            Some((false, true, 1)),
        ),
        (vec![Dynamic, Dynamic, Literal("café {BODY}")], None),
        (vec![Literal("--"), Dynamic, Expanded], None),
        (
            vec![
                KnownBytes(b"--"),
                Dynamic,
                KnownBytes("café {BODY}".as_bytes()),
            ],
            Some((false, true, 2)),
        ),
        (
            vec![KnownBytes(b"--"), Dynamic, KnownBytes(b"a {BODY}\0")],
            None,
        ),
        (vec![KnownBytes(b"--"), Dynamic, KnownBytes(&[0xff])], None),
    ] {
        let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            current.commands(),
            Some(current.context()),
            crate::InvocationWords::structured(Literal("switch"), &values),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        )
        .resolved()
        .unwrap();
        let native_roles = selected.facts().arg_roles_complete;
        let projection = selected.authored_logical_source_role_projection();
        assert_eq!(
            projection.as_ref().map(|roles| {
                let bodies = roles.case_body_operands().unwrap();
                (
                    bodies.selection_unknown,
                    bodies.no_match_possible,
                    bodies.bodies[0].argument,
                )
            }),
            expected,
            "{values:?}"
        );
        assert_eq!(
            selected.facts().arg_roles_complete,
            native_roles,
            "source roles never rewrite executable facts"
        );
        if let Some(roles) = projection {
            let body = roles.case_body_operands().unwrap().bodies[0];
            assert_eq!(body.list_element, Some(1));
            assert!(
                roles
                    .argument_roles()
                    .contains(&(body.argument, crate::ArgRole::Body))
            );
        }
    }
    assert_case_role_projection_by_release(&current);
}

fn assert_case_role_projection_by_release(current: &crate::model::assembly::ContextRegistry) {
    use crate::InvocationWord::{Dynamic, Literal};
    let older = crate::model::ingress::resolve_environment("tcl8.4")
        .default_context_registry()
        .with_command_store(std::sync::Arc::clone(current.commands()));
    let unmarked = [Literal("-exact"), Dynamic, Literal("café {BODY}")];
    let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
        older.commands(),
        Some(older.context()),
        crate::InvocationWords::structured(Literal("switch"), &unmarked),
        tcl_dialect::model::InvocationRealm::RuleLoader,
    )
    .resolved()
    .unwrap();
    assert!(
        selected
            .authored_logical_source_role_projection()
            .unwrap()
            .case_body_operands()
            .unwrap()
            .selection_unknown
    );
    // Tcl 8.6.18 and 9.0.4 raise "-matchvar option requires -regexp
    // option" without `-regexp`; 8.4.20 has no `-matchvar` at all.
    let values = [
        Literal("-regexp"),
        Literal("-matchvar"),
        Literal("matches"),
        Literal("--"),
        Dynamic,
        Literal("a {BODY}"),
    ];
    for (context, available) in [(current, true), (&older, false)] {
        let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            crate::InvocationWords::structured(Literal("switch"), &values),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        )
        .resolved()
        .unwrap();
        assert_eq!(
            selected.authored_logical_source_role_projection().is_some(),
            available
        );
    }
}

#[test]
fn authored_jim_core_layout_uses_source_grammar_in_shared_registry() {
    let registry = CommandRegistry::build_default();
    let dialect = crate::InvocationDialect::of_profile(
        crate::model::ingress::resolve_environment("jim").analyser_profile(),
    );
    for (arguments, body, statics) in [
        (vec!["p", "arg", "BODY"], 2, None),
        (vec!["p", "arg", "{n 0}", "BODY"], 3, Some(2)),
    ] {
        let words = arguments
            .iter()
            .map(|value| InvocationWord::Literal(value))
            .collect::<Vec<_>>();
        let selected = registry
            .resolve_structured_invocation(
                InvocationWords::structured(InvocationWord::Literal("proc"), &words)
                    .with_dialect(dialect),
                dialect.authoring_query(),
            )
            .resolved()
            .unwrap();
        assert_eq!(
            selected.authored_source_procedure_arguments(),
            Some(AuthoredSourceProcedureArguments {
                name: 0,
                parameters: 1,
                body,
            })
        );
        let (roles, complete) = selected.authored_source_argument_roles();
        assert!(complete);
        assert_eq!(
            roles
                .iter()
                .filter_map(
                    |&(at, role)| (role == ArgRole::StaticVarList).then_some(usize::from(at))
                )
                .collect::<Vec<_>>(),
            statics.into_iter().collect::<Vec<_>>()
        );
        assert_eq!(
            roles
                .iter()
                .filter_map(|&(at, role)| (role == ArgRole::Body).then_some(usize::from(at)))
                .collect::<Vec<_>>(),
            [body]
        );
    }
}

#[test]
fn authored_jim_layout_preserves_c_unknown_and_expanded_refusals() {
    let registry = CommandRegistry::build_default();
    for dialect in [
        None,
        Some(crate::InvocationDialect::for_version(
            tcl_dialect::TclVersion::V8_6,
        )),
    ] {
        let arguments = [
            InvocationWord::Literal("p"),
            InvocationWord::Literal("arg"),
            InvocationWord::Literal("STATIC"),
            InvocationWord::Literal("BODY"),
        ];
        let mut words = InvocationWords::structured(InvocationWord::Literal("proc"), &arguments);
        if let Some(dialect) = dialect {
            words = words.with_dialect(dialect);
        }
        let selected = registry
            .resolve_structured_invocation(
                words,
                dialect.and_then(crate::InvocationDialect::authoring_query),
            )
            .resolved()
            .unwrap();
        assert!(selected.authored_source_procedure_arguments().is_none());
    }
    let dialect = crate::InvocationDialect::of_profile(
        crate::model::ingress::resolve_environment("jim").analyser_profile(),
    );
    let arguments = [InvocationWord::Expanded];
    let selected = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal("proc"), &arguments)
                .with_dialect(dialect),
            dialect.authoring_query(),
        )
        .resolved()
        .unwrap();
    assert!(selected.authored_source_procedure_arguments().is_none());
    assert!(!selected.authored_source_argument_roles().1);
}

#[test]
fn authored_jim_core_layout_refuses_conflicting_selected_roles() {
    let dialect = crate::InvocationDialect::of_profile(
        crate::model::ingress::resolve_environment("jim").analyser_profile(),
    );
    let cases: &[&[(u8, ArgRole)]] = &[
        &[
            (1, ArgRole::Name),
            (0, ArgRole::ParamList),
            (2, ArgRole::Body),
        ],
        &[
            (0, ArgRole::Name),
            (1, ArgRole::ParamList),
            (2, ArgRole::Body),
            (3, ArgRole::Body),
        ],
        &[
            (0, ArgRole::Name),
            (1, ArgRole::ParamList),
            (2, ArgRole::Body),
            (1, ArgRole::StaticVarList),
        ],
        &[
            (0, ArgRole::Name),
            (1, ArgRole::ParamList),
            (2, ArgRole::Body),
            (2, ArgRole::StaticVarList),
            (2, ArgRole::StaticVarList),
        ],
    ];
    for &roles in cases {
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "source-core-definer",
            arity: Arity::exact(3),
            arg_roles: roles,
            traits: Traits::DEFINES_PROCEDURE | Traits::DEFERS_BODY,
            procedure_definition: Some(
                crate::native_procedure::NativeProcedureDefinitionSpec::Core,
            ),
            ..CommandSpec::DEFAULT
        });
        let arguments = [
            InvocationWord::Literal("p"),
            InvocationWord::Literal("arg"),
            InvocationWord::Literal("STATIC"),
            InvocationWord::Literal("BODY"),
        ];
        let selected = registry
            .resolve_structured_invocation(
                InvocationWords::structured(
                    InvocationWord::Literal("source-core-definer"),
                    &arguments,
                )
                .with_dialect(dialect),
                dialect.authoring_query(),
            )
            .resolved()
            .unwrap();
        assert!(selected.authored_source_procedure_arguments().is_none());
        assert!(!selected.authored_source_argument_roles().1);
    }
}

#[test]
fn authored_jim_layout_does_not_infer_a_core_descriptor_from_proc_hook() {
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "source-hook-definer",
        arity: Arity::exact(4),
        arg_roles: &[
            (0, ArgRole::Name),
            (1, ArgRole::ParamList),
            (3, ArgRole::Body),
        ],
        analyser_hook: Some(crate::hooks::AnalyserHookId::Proc),
        traits: Traits::DEFINES_PROCEDURE | Traits::DEFERS_BODY,
        command_table_effect: Some(CommandTableEffect::DefinesProcedure),
        ..CommandSpec::DEFAULT
    });
    let dialect = crate::InvocationDialect::of_profile(
        crate::model::ingress::resolve_environment("jim").analyser_profile(),
    );
    let arguments = [
        InvocationWord::Literal("p"),
        InvocationWord::Literal("arg"),
        InvocationWord::Literal("STATIC"),
        InvocationWord::Literal("BODY"),
    ];
    let selected = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal("source-hook-definer"), &arguments)
                .with_dialect(dialect),
            dialect.authoring_query(),
        )
        .resolved()
        .unwrap();
    assert!(selected.semantics.procedure_definition.is_none());
    assert!(selected.authored_source_procedure_arguments().is_none());
}

#[test]
fn repeating_target_roles_do_not_require_a_literal_container_value() {
    let registry = crate::model::ingress::static_context_for("tcl8.6").commands();
    let words = [
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Literal("x"),
        crate::InvocationWord::Literal("y"),
    ];
    let invocation = registry.resolve_structured_invocation(
        crate::InvocationWords::structured(crate::InvocationWord::Literal("lassign"), &words),
        registry.own_surface_query(),
    );
    let facts = invocation.resolved().unwrap().facts();
    assert!(facts.arg_roles_complete);
    assert_eq!(
        facts.arg_roles,
        vec![(1, ArgRole::VarWrite), (2, ArgRole::VarWrite)]
    );
}

#[test]
fn optional_roles_follow_proved_argv_presence() {
    let registry = CommandRegistry::build_default();
    let query = Some(SurfaceQuery::core(Family::Tcl, "8.6"));
    for (arguments, expected) in [
        (vec!["{}"], vec![(0, ArgRole::Body)]),
        (
            vec!["{}", "result"],
            vec![(0, ArgRole::Body), (1, ArgRole::VarWrite)],
        ),
        (
            vec!["{}", "result", "options"],
            vec![
                (0, ArgRole::Body),
                (1, ArgRole::VarWrite),
                (2, ArgRole::VarWrite),
            ],
        ),
    ] {
        let invocation = registry
            .resolve_invocation("catch", &arguments, query)
            .unwrap();
        assert_eq!(invocation.argument_roles().0, expected);
        assert_eq!(invocation.facts().arg_roles, expected);
        assert_eq!(invocation.facts_after_success().arg_roles, expected);
    }
    let words = [crate::InvocationWord::Expanded];
    let invocation = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(crate::InvocationWord::Literal("catch"), &words),
            query,
        )
        .resolved()
        .unwrap();
    assert!(
        invocation
            .argument_roles()
            .0
            .contains(&(2, ArgRole::VarWrite))
    );
}

fn resolver_backed_arg_roles(_: &[&str]) -> Vec<(u8, ArgRole)> {
    vec![(1, ArgRole::VarRead)]
}

#[test]
fn invocation_facts_resolve_roles_for_literal_argv() {
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "resolver-backed-roles-fixture",
        arity: Arity::any(),
        arg_roles: &[(0, ArgRole::VarWrite)],
        arg_role_resolver: Some(resolver_backed_arg_roles),
        ..CommandSpec::DEFAULT
    });
    let facts = registry
        .resolve_invocation("resolver-backed-roles-fixture", &["first", "second"], None)
        .expect("fixture command resolves")
        .facts();

    assert_eq!(facts.arg_roles, vec![(1, ArgRole::VarRead)]);
    assert!(facts.arg_roles_complete);
}

#[test]
fn sole_argument_role_index_uses_resolved_array_shape_and_dialect() {
    let registry = CommandRegistry::build_default();
    let variable_roles = [ArgRole::VarRead, ArgRole::VarWrite];

    let exists = registry
        .resolve_invocation(
            "array",
            &["exists", "items"],
            Some(SurfaceQuery::core(Family::Tcl, "9.0")),
        )
        .expect("array exists resolves")
        .facts();
    assert_eq!(
        exists.sole_argument_index_for_roles(2, &variable_roles),
        Some(1)
    );

    let default = registry
        .resolve_invocation(
            "array",
            &["def", "get", "items"],
            Some(SurfaceQuery::core(Family::Tcl, "9.0")),
        )
        .expect("Tcl 9 array default prefix resolves")
        .facts();
    assert_eq!(default.subcommand.canonical_name(), Some("default"));
    assert_eq!(
        default.sole_argument_index_for_roles(3, &variable_roles),
        Some(2)
    );
    assert_eq!(
        default.sole_argument_index_for_roles(5, &variable_roles),
        None,
        "the member's outer arity is part of target resolution"
    );

    let legacy = registry
        .resolve_invocation(
            "array",
            &["default", "get", "items"],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .expect("the array command itself resolves")
        .facts();
    assert_eq!(legacy.subcommand.canonical_name(), None);
    assert_eq!(
        legacy.sole_argument_index_for_roles(3, &variable_roles),
        None,
        "a Tcl 9-only member has no target under Tcl 8.6"
    );

    let dict_with = registry
        .resolve_invocation(
            "dict",
            &["with", "items", ""],
            Some(SurfaceQuery::core(Family::Tcl, "9.0")),
        )
        .expect("dict with resolves")
        .facts();
    assert!(dict_with.arg_roles_complete);
    assert_eq!(
        dict_with.sole_argument_index_for_roles(3, &variable_roles),
        Some(1),
        "one multi-role argument is still the sole matching operand"
    );
}

#[test]
fn invocation_facts_keep_dynamic_resolver_roles_incomplete() {
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "resolver-backed-dynamic-roles-fixture",
        arity: Arity::any(),
        arg_roles: &[(0, ArgRole::VarWrite)],
        arg_role_resolver: Some(resolver_backed_arg_roles),
        arg_role_resolver_roles: &[ArgRole::VarRead],
        ..CommandSpec::DEFAULT
    });
    let arguments = [
        crate::InvocationWord::Literal("first"),
        crate::InvocationWord::Dynamic,
    ];
    let facts = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(
                crate::InvocationWord::Literal("resolver-backed-dynamic-roles-fixture"),
                &arguments,
            ),
            None,
        )
        .resolved()
        .expect("fixture command resolves")
        .facts();

    assert_eq!(facts.arg_roles, vec![(0, ArgRole::VarWrite)]);
    assert!(!facts.arg_roles_complete);
    assert_eq!(facts.arg_role_resolver_roles, &[ArgRole::VarRead]);
}

#[test]
fn cardinality_roles_preserve_dynamic_foreach_operands_and_unknown_expansion() {
    use crate::InvocationWord::{Dynamic, Expanded, Literal};
    let registry = CommandRegistry::build_default();
    for (arguments, body, complete) in [
        (
            vec![Literal("x"), Dynamic, Literal("set y $x")],
            Some(2),
            true,
        ),
        (
            vec![Literal("x"), Dynamic, Literal("y"), Dynamic, Dynamic],
            Some(4),
            true,
        ),
        (
            vec![Literal("x"), Expanded, Literal("set y $x")],
            None,
            false,
        ),
    ] {
        let invocation = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal("foreach"), &arguments),
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .resolved()
            .expect("selected foreach");
        assert_eq!(
            invocation.semantics.arg_role_resolver_input(),
            crate::spec::ArgRoleResolverInput::Cardinality
        );
        let facts = invocation.facts();
        assert_eq!(
            facts
                .arg_roles
                .iter()
                .find_map(|(at, role)| (*role == ArgRole::Body).then_some(*at)),
            body
        );
        assert_eq!(facts.arg_roles_complete, complete);
    }
}

#[test]
fn cardinality_scan_roles_preserve_unknown_values_and_expansion_uncertainty() {
    use crate::InvocationWord::{Dynamic, Expanded, Literal};
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let registry = crate::model::ingress::static_context_for(environment).commands();
        let profile = crate::model::ingress::resolve_environment(environment).unit_profile();
        let dialect = crate::InvocationDialect::of_profile(profile);
        for command in ["scan", "binary"] {
            if command == "binary" && environment == "jim" {
                continue;
            }
            let prefix = usize::from(command == "binary");
            let mut operands = Vec::new();
            if prefix != 0 {
                operands.push(Literal("scan"));
            }
            operands.extend([Dynamic, Dynamic, Literal("first"), Literal("second")]);
            let words =
                InvocationWords::structured(Literal(command), &operands).with_dialect(dialect);
            let resolved = registry.resolve_structured_invocation(words, dialect.authoring_query());
            let facts = resolved.resolved().expect("selected native scan").facts();
            assert!(facts.arg_roles_complete, "{environment}: {command}");
            assert_eq!(
                facts.arg_roles,
                vec![
                    (1, ArgRole::ScanFormat),
                    (2, ArgRole::VarWrite),
                    (3, ArgRole::VarWrite),
                ],
                "{environment}: {command}"
            );
            operands[prefix] = Expanded;
            let words =
                InvocationWords::structured(Literal(command), &operands).with_dialect(dialect);
            let resolved = registry.resolve_structured_invocation(words, dialect.authoring_query());
            let facts = resolved
                .resolved()
                .expect("selected scan with unknown cardinality")
                .facts();
            assert!(!facts.arg_roles_complete);
            assert!(
                !facts
                    .arg_roles
                    .iter()
                    .any(|(_, role)| *role == ArgRole::VarWrite)
            );
        }
    }
}
