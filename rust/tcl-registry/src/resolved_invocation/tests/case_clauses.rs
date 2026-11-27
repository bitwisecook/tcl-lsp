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

//! Case pattern layout, presentation and clause projection, clause roles and clause-shape advice.

use super::*;

#[test]
fn original_case_pattern_layout_keeps_unknown_payloads_and_actual_option_windows() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    use crate::InvocationWord::{Dynamic, Expanded, Literal};
    let current = crate::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let older = crate::model::ingress::static_context_for("tcl8.4")
        .with_command_store(std::sync::Arc::clone(current.commands()));
    for (arguments, current_expected, older_expected) in [
        (
            vec![Literal("-regexp"), Dynamic, Literal("(a+)+ {BODY}")],
            Some((Some(2), None)),
            Some((Some(2), None)),
        ),
        (
            vec![Literal("-regexp"), Dynamic, Literal("(a+)+"), Dynamic],
            Some((None, Some(2))),
            Some((None, Some(2))),
        ),
        (
            vec![
                Literal("-regexp"),
                Literal("-matchvar"),
                Dynamic,
                Dynamic,
                Dynamic,
            ],
            Some((Some(4), None)),
            None,
        ),
        (vec![Dynamic, Dynamic, Literal("(a+)+ {BODY}")], None, None),
        (vec![Literal("-regexp"), Dynamic, Expanded], None, None),
    ] {
        for (context, expected) in [
            (current.as_ref(), current_expected),
            (&older, older_expected),
        ] {
            // Selected source layout, with no original Native dialect or argv grant.
            let selected =
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    context.commands(),
                    Some(context.context()),
                    crate::InvocationWords::structured(Literal("switch"), &arguments),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .resolved()
                .unwrap();
            let layout = selected.authored_source_case_pattern_layout();
            assert_eq!(
                layout.map(|(_, invocation)| (
                    invocation.clause_list_index,
                    invocation.inline_clause_start
                )),
                expected,
                "{arguments:?}"
            );
            if let Some((_, invocation)) = layout {
                assert_eq!(invocation.mode, crate::spec::CaseMatchMode::Regexp);
            }
        }
    }
}

#[test]
fn source_clause_roles_keep_resolver_roles_and_actual_availability() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    use crate::InvocationWord::{Expanded, Literal};
    let current = crate::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let older = crate::model::ingress::resolve_environment("tcl8.4")
        .default_context_registry()
        .with_command_store(current.commands().snapshot().shared_registry());
    let arguments = [
        Literal("update"),
        Literal("d"),
        Literal("key"),
        Literal("local"),
        Literal("BODY"),
    ];
    let words = crate::InvocationWords::structured(Literal("dict"), &arguments);
    let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
        current.commands(),
        Some(current.context()),
        words,
        tcl_dialect::model::InvocationRealm::RuleLoader,
    )
    .resolved()
    .unwrap();
    let roles = selected.arg_roles().unwrap();
    assert!(roles.contains(&(1, ArgRole::VarRead)));
    assert!(roles.contains(&(1, ArgRole::VarWrite)));
    assert!(roles.contains(&(3, ArgRole::LoopVarList)));
    assert!(roles.contains(&(4, ArgRole::Body)));
    assert!(
        crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            older.commands(),
            Some(older.context()),
            words,
            tcl_dialect::model::InvocationRealm::RuleLoader,
        )
        .resolved()
        .is_none()
    );
    let expanded = [Literal("update"), Literal("d"), Expanded, Literal("BODY")];
    let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
        current.commands(),
        Some(current.context()),
        crate::InvocationWords::structured(Literal("dict"), &expanded),
        tcl_dialect::model::InvocationRealm::RuleLoader,
    )
    .resolved()
    .unwrap();
    assert!(selected.arg_roles().is_none());
}

#[test]
fn authored_case_presentation_keeps_incomplete_list_separate_from_invocation() {
    // Implementation contract: naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    let registry = crate::CommandRegistry::build_default();
    for version in tcl_dialect::TclVersion::ALL {
        let words = [
            crate::InvocationWord::Literal("--"),
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Literal("x {puts x} dangling"),
        ];
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("switch"),
                    &words,
                )
                .with_dialect(tcl_native_model::InvocationDialect::for_version(version)),
                tcl_native_model::InvocationDialect::for_version(version).authoring_query(),
            )
            .resolved()
            .unwrap();
        assert!(selected.authored_source_case_invocation().is_none());
        assert_eq!(
            selected
                .authored_source_case_presentation()
                .map(|(_, argument)| argument),
            Some(2)
        );
        for list in ["", "x {puts x}", "x {puts x"] {
            let words = [
                crate::InvocationWord::Literal("--"),
                crate::InvocationWord::Dynamic,
                crate::InvocationWord::Literal(list),
            ];
            let selected = registry
                .resolve_structured_invocation(
                    crate::InvocationWords::structured(
                        crate::InvocationWord::Literal("switch"),
                        &words,
                    )
                    .with_dialect(tcl_native_model::InvocationDialect::for_version(version)),
                    tcl_native_model::InvocationDialect::for_version(version).authoring_query(),
                )
                .resolved()
                .unwrap();
            assert!(selected.authored_source_case_presentation().is_none());
        }
        let words = [
            crate::InvocationWord::Literal("--"),
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Expanded,
            crate::InvocationWord::Literal("x {puts x} dangling"),
        ];
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("switch"),
                    &words,
                )
                .with_dialect(tcl_native_model::InvocationDialect::for_version(version)),
                tcl_native_model::InvocationDialect::for_version(version).authoring_query(),
            )
            .resolved()
            .unwrap();
        assert!(selected.authored_source_case_presentation().is_none());
    }
}

#[test]
fn authored_case_layout_retains_dynamic_subject_and_exact_control_words() {
    // Implementation contract: naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    let registry = CommandRegistry::build_default();
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = tcl_native_model::InvocationDialect::for_version(version);
        let arguments = [
            crate::InvocationWord::KnownBytes(b"--"),
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::KnownBytes(b"x {puts x} default {puts fallback}"),
        ];
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("switch"),
                    &arguments,
                )
                .with_dialect(dialect),
                dialect.authoring_query(),
            )
            .resolved()
            .unwrap();
        let (_, layout) = selected.authored_source_case_invocation().unwrap();
        assert_eq!(layout.subject_index, Some(1));
        assert_eq!(layout.clause_list_index, Some(2));
        assert!(selected.words.arguments().literal_at(1).is_none());
        let expanded = [crate::InvocationWord::Expanded];
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("switch"),
                    &expanded,
                )
                .with_dialect(dialect),
                dialect.authoring_query(),
            )
            .resolved()
            .unwrap();
        assert!(selected.authored_source_case_invocation().is_none());
    }
}

#[test]
fn source_case_clause_projection_keeps_counted_bytes_and_unknown_option_selection() {
    use crate::InvocationWord::{Dynamic, Expanded, KnownBytes, Literal};
    for environment in ["tcl8.4", "tcl8.6", "tcl9.1", "jim", "f5-irules"] {
        let context =
            crate::model::ingress::resolve_environment(environment).default_context_registry();
        let dialect =
            tcl_native_model::InvocationDialect::of_profile(context.commands().profile().unwrap());
        let words = [
            KnownBytes(b"-exact"),
            Dynamic,
            KnownBytes(b"/a {set x 1} default {set x 2}"),
        ];
        let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            crate::InvocationWords::structured(Literal("switch"), &words).with_dialect(dialect),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        )
        .resolved()
        .unwrap();
        assert!(
            !selected.authored_source_argument_roles().1,
            "{environment}"
        );
        let bodies = selected
            .authored_source_case_clause_projection(dialect.word_values)
            .unwrap();
        let source_selected =
            crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                context.commands(),
                Some(context.context()),
                crate::InvocationWords::structured(Literal("switch"), &words),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            )
            .resolved()
            .unwrap();
        // Words written without a dialect take the registry profile's.
        assert_eq!(source_selected.words.arguments().dialect(), Some(dialect));
        assert_eq!(
            source_selected
                .authored_source_case_clause_projection(dialect.word_values)
                .unwrap(),
            bodies
        );

        assert_eq!(bodies.bodies.len(), 2, "{environment}");
        assert!(bodies.bodies.iter().all(|body| body.argument == 2));
        assert!(bodies.selection_unknown, "{environment}");
        for words in [
            vec![
                KnownBytes(b"-exact"),
                Dynamic,
                KnownBytes(b"/a {set x 1} dangling"),
            ],
            vec![
                KnownBytes(b"-exact"),
                Dynamic,
                KnownBytes(b"/a {set x 1}\0"),
            ],
            vec![KnownBytes(b"-exact"), Dynamic, Expanded],
        ] {
            let selected =
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    context.commands(),
                    Some(context.context()),
                    crate::InvocationWords::structured(Literal("switch"), &words)
                        .with_dialect(dialect),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .resolved()
                .unwrap();
            assert!(
                selected
                    .authored_source_case_clause_projection(dialect.word_values)
                    .is_none(),
                "{environment}"
            );
        }
    }
}

#[test]
fn authored_source_shape_advice_keeps_unknown_payloads_and_selected_dialect() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    use crate::InvocationWord::{Dynamic, Expanded, Literal};
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let context =
            crate::model::ingress::resolve_environment(environment).default_context_registry();
        let dialect =
            tcl_native_model::InvocationDialect::of_profile(context.commands().profile().unwrap());
        for arguments in [
            vec![Dynamic, Dynamic, Dynamic],
            vec![Literal("-regexp"), Dynamic, Dynamic, Dynamic],
        ] {
            let selected =
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    context.commands(),
                    Some(context.context()),
                    crate::InvocationWords::structured(Literal("switch"), &arguments)
                        .with_dialect(dialect),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .resolved()
                .unwrap();
            let bodies = selected.authored_source_case_body_arguments().unwrap();
            assert_eq!(bodies.len(), 1, "{environment}");
            assert_eq!(bodies[0].argument, arguments.len() - 1);
            assert_eq!(bodies[0].regexp, arguments.len() == 4);
        }
        for (arguments, expected) in [
            (vec![Literal("-nocomplain")], true),
            (vec![Literal("--")], true),
            (vec![Literal("-nocomplain"), Literal("--")], true),
            (
                vec![Literal("-nocomplain"), Literal("-nocomplain")],
                environment == "jim",
            ),
            (vec![Literal("-nocomplain"), Dynamic], false),
            (vec![Expanded], false),
        ] {
            let selected =
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    context.commands(),
                    Some(context.context()),
                    crate::InvocationWords::structured(Literal("unset"), &arguments)
                        .with_dialect(dialect),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .resolved()
                .unwrap();
            assert_eq!(
                selected
                    .authored_source_unset_option_only_arguments()
                    .is_some(),
                expected,
                "{environment}: {arguments:?}"
            );
        }
        let arguments = [Dynamic, Dynamic];
        let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            crate::InvocationWords::structured(Literal("append"), &arguments).with_dialect(dialect),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        )
        .resolved()
        .unwrap();
        let append = selected.authored_source_append_arguments().unwrap();
        assert_eq!(append.variable, 0);
        assert_eq!(append.values, 1..2);
        let arguments = [Dynamic, Expanded];
        let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            crate::InvocationWords::structured(Literal("append"), &arguments).with_dialect(dialect),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        )
        .resolved()
        .unwrap();
        assert!(selected.authored_source_append_arguments().is_none());
    }
}
