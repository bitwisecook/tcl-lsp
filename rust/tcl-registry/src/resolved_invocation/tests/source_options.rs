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

//! Teardown and destructive-path targets, possible and available options, file arguments and diagnostic options.

use super::*;

#[test]
fn original_teardown_source_layout_uses_selected_effective_argv() {
    use crate::InvocationWord::{Expanded, Literal};
    let owner = crate::model::ingress::resolve_environment("tk").default_context_registry();
    let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
        owner.commands(),
        Some(owner.context()),
        crate::InvocationWords::structured(
            Literal("destroy"),
            &[Literal(".root"), Literal(".root.child")],
        ),
        tcl_dialect::model::InvocationRealm::RuleLoader,
    )
    .resolved()
    .expect("selected Tk teardown descriptor");
    assert_eq!(
        selected
            .authored_source_teardown_arguments()
            .map(|layout| layout.arguments),
        Some(0..2)
    );

    let expanded = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
        owner.commands(),
        Some(owner.context()),
        crate::InvocationWords::structured(Literal("destroy"), &[Expanded]),
        tcl_dialect::model::InvocationRealm::RuleLoader,
    )
    .resolved()
    .expect("selected expanded Tk teardown descriptor");
    assert!(expanded.authored_source_teardown_arguments().is_none());
}

#[test]
fn original_destructive_source_paths_keep_dynamic_and_captured_option_boundaries() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    use crate::InvocationWord::{Dynamic, Literal};
    let owner = crate::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    for (arguments, expected, unknown) in [
        (
            vec![Literal("delete"), Dynamic, Literal("later")],
            1..2,
            true,
        ),
        (
            vec![
                Literal("delete"),
                Literal("-force"),
                Literal("--"),
                Dynamic,
                Literal("later"),
            ],
            3..5,
            false,
        ),
        (vec![Literal("mkdir"), Dynamic, Dynamic], 1..3, false),
    ] {
        let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            owner.commands(),
            Some(owner.context()),
            crate::InvocationWords::structured(Literal("file"), &arguments),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        )
        .resolved()
        .unwrap();
        let paths = selected
            .authored_source_destructive_path_arguments()
            .unwrap();
        assert_eq!(paths.arguments, expected);
        assert_eq!(paths.selection_unknown, unknown);
        assert_eq!(paths.subcommands.len(), 1);
        assert!(matches!(arguments[0], Literal(name) if paths.subcommands[0] == name));
    }
}

#[test]
fn original_destructive_source_paths_refuse_unknown_widths_and_unselected_descriptors() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    use crate::InvocationWord::{Dynamic, Expanded, Literal, Opaque};
    let owner = crate::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    for arguments in [
        vec![Literal("delete"), Literal("-unknown"), Dynamic],
        vec![Literal("delete"), Expanded, Dynamic],
        vec![Literal("delete"), Opaque, Dynamic],
        vec![Dynamic, Literal("--"), Dynamic],
        vec![Literal("exists"), Dynamic],
    ] {
        let resolution = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            owner.commands(),
            Some(owner.context()),
            crate::InvocationWords::structured(Literal("file"), &arguments),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        );
        assert!(
            resolution
                .resolved()
                .and_then(|selected| selected.authored_source_destructive_path_arguments())
                .is_none(),
            "{arguments:?}"
        );
    }
}

#[test]
fn source_possible_options_branch_over_admitted_widths_and_terminators() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Pure selected descriptor/source layout, without handler execution.
    static OPTIONS: &[crate::hover::OptionSpec] = &[
        crate::hover::OptionSpec {
            name: "--",
            surface: Some(SpecSurface::TCL86_PLUS),
            ..crate::hover::OptionSpec::DEFAULT
        },
        crate::hover::OptionSpec {
            name: "-flag",
            ..crate::hover::OptionSpec::DEFAULT
        },
        crate::hover::OptionSpec {
            name: "-value",
            value: crate::hover::OptionValue::value("value"),
            ..crate::hover::OptionSpec::DEFAULT
        },
    ];
    let mut registry = CommandRegistry::build_default();
    registry.insert(crate::CommandSpec {
        name: "source-options",
        options: OPTIONS,
        ..crate::CommandSpec::DEFAULT
    });
    let dynamic = crate::InvocationWord::Dynamic;
    let literal = crate::InvocationWord::Literal;
    for (arguments, expected) in [
        (vec![dynamic, literal("data"), dynamic], vec![0, 2]),
        (vec![literal("-v"), dynamic, dynamic], vec![0, 2]),
        (vec![literal("--"), dynamic], vec![]),
        (vec![literal("data"), dynamic], vec![]),
        (vec![literal("-missing"), dynamic], vec![0]),
    ] {
        let words = crate::InvocationWords::structured(literal("source-options"), &arguments);
        let selected = registry
            .resolve_structured_invocation(words, Some(SurfaceQuery::core(Family::Tcl, "8.6")))
            .resolved()
            .unwrap();
        assert_eq!(
            selected
                .authored_source_possible_option_arguments()
                .unwrap()
                .arguments,
            expected
        );
        let older = registry
            .resolve_structured_invocation(words, Some(SurfaceQuery::core(Family::Tcl, "8.4")))
            .resolved()
            .unwrap();
        assert!(older.authored_source_possible_option_arguments().is_none());
    }
    let expanded = [crate::InvocationWord::Expanded];
    let selected = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(literal("source-options"), &expanded),
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .resolved()
        .unwrap();
    assert!(
        selected
            .authored_source_possible_option_arguments()
            .is_none()
    );
}

#[test]
// naming.core.original-registry-context-completion
// docs/design/analysis/name-resolution-proofs/original-registry-context-completion.md
fn source_completion_options_keep_selected_form_and_surface() {
    use crate::InvocationWord::Literal;
    static BASE: &[crate::hover::OptionSpec] = &[crate::hover::OptionSpec {
        name: "-base",
        ..crate::hover::OptionSpec::DEFAULT
    }];
    static FIRST: &[crate::hover::OptionSpec] = &[
        crate::hover::OptionSpec {
            name: "-first",
            ..crate::hover::OptionSpec::DEFAULT
        },
        crate::hover::OptionSpec {
            name: "-new",
            surface: Some(SpecSurface::TCL90_PLUS),
            ..crate::hover::OptionSpec::DEFAULT
        },
    ];
    static SECOND: &[crate::hover::OptionSpec] = &[crate::hover::OptionSpec {
        name: "-second",
        ..crate::hover::OptionSpec::DEFAULT
    }];
    static FORMS: &[crate::forms::CommandForm] = &[
        crate::forms::CommandForm {
            name: "first",
            arity: crate::Arity::exact(3),
            literal_argument_prefix: Some(crate::forms::LiteralArgumentPrefix::exact(&["first"])),
            options: FIRST,
            ..crate::forms::CommandForm::DEFAULT
        },
        crate::forms::CommandForm {
            name: "second",
            arity: crate::Arity::exact(3),
            literal_argument_prefix: Some(crate::forms::LiteralArgumentPrefix::exact(&["second"])),
            options: SECOND,
            ..crate::forms::CommandForm::DEFAULT
        },
    ];
    let mut registry = CommandRegistry::build_default();
    registry.insert(crate::CommandSpec {
        name: "source-completion-forms",
        options: BASE,
        command_forms: FORMS,
        ..crate::CommandSpec::DEFAULT
    });
    for (selector, version, expected) in [
        ("first", "8.6", vec!["-base", "-first"]),
        ("second", "8.6", vec!["-base", "-second"]),
        ("first", "9.0", vec!["-base", "-first", "-new"]),
    ] {
        let arguments = [Literal(selector), Literal("-value"), Literal("")];
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal("source-completion-forms"), &arguments),
                Some(SurfaceQuery::core(Family::Tcl, version)),
            )
            .resolved()
            .unwrap();
        assert_eq!(
            selected
                .authored_source_available_options()
                .unwrap()
                .into_iter()
                .map(|option| option.name)
                .collect::<Vec<_>>(),
            expected,
        );
    }
    for selector in [Literal("missing"), crate::InvocationWord::Dynamic] {
        let arguments = [selector, Literal("-value"), Literal("")];
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal("string"), &arguments),
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .resolved()
            .unwrap();
        assert!(selected.authored_source_available_options().is_none());
    }
}

#[test]
// naming.core.original-registry-context-completion
// docs/design/analysis/name-resolution-proofs/original-registry-context-completion.md
fn source_completion_options_require_actual_nested_selector() {
    use crate::InvocationWord::{Dynamic, Literal};
    let registry = CommandRegistry::build_default();
    for operation in [Literal("configure"), Literal("create")] {
        let arguments = [
            Literal("ensemble"),
            operation,
            Literal("-prefixes"),
            Literal(""),
        ];
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal("namespace"), &arguments),
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .resolved()
            .unwrap();
        assert!(
            selected
                .authored_source_available_options()
                .unwrap()
                .iter()
                .any(|option| option.name == "-prefixes")
        );
    }
    for arguments in [
        vec![
            Literal("ensemble"),
            Dynamic,
            Literal("-prefixes"),
            Literal(""),
        ],
        vec![
            Literal("ensemble"),
            Literal("missing"),
            Literal("-prefixes"),
            Literal(""),
        ],
        vec![Literal("ensemble")],
    ] {
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal("namespace"), &arguments),
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .resolved()
            .unwrap();
        assert!(selected.authored_source_available_options().is_none());
        assert!(selected.authored_source_diagnostic_options().is_none());
    }
}

#[test]
fn original_source_file_argument_keeps_available_forms_and_unknown_selectors() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    // Selected source form only; no native file or accepted handler follows.
    use crate::InvocationWord::{Dynamic, Expanded, Literal};
    let current = crate::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let older = crate::model::ingress::static_context_for("tcl8.4")
        .with_command_store(std::sync::Arc::clone(current.commands()));
    let newer = crate::model::ingress::static_context_for("tcl9.0")
        .with_command_store(std::sync::Arc::clone(current.commands()));
    for (arguments, expected_current, expected_older, expected_newer) in [
        (vec![Dynamic], Some(0), Some(0), Some(0)),
        (
            vec![Literal("-encoding"), Dynamic, Dynamic],
            Some(2),
            None,
            Some(2),
        ),
        (vec![Literal("-nopkg"), Dynamic], None, None, Some(1)),
        (vec![Literal("-enc"), Dynamic, Dynamic], None, None, None),
        (vec![Dynamic, Dynamic, Dynamic], None, None, None),
        (vec![Expanded], None, None, None),
    ] {
        for (context, expected) in [
            (current.as_ref(), expected_current),
            (&older, expected_older),
            (&newer, expected_newer),
        ] {
            // No original Native dialect is donated from this context.
            let selected =
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    context.commands(),
                    Some(context.context()),
                    crate::InvocationWords::structured(Literal("source"), &arguments),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .resolved()
                .unwrap();
            assert_eq!(
                selected.authored_source_file_argument(),
                expected,
                "{arguments:?}"
            );
        }
    }
    let arguments = [Literal("literal")];
    let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
        current.commands(),
        Some(current.context()),
        crate::InvocationWords::structured(Literal("puts"), &arguments),
        tcl_dialect::model::InvocationRealm::RuleLoader,
    )
    .resolved()
    .unwrap();
    assert!(selected.authored_source_file_argument().is_none());
}

#[test]
fn original_invokehidden_options_keep_the_path_prefix_and_actual_availability() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    // Native source anchor: native_event_original/source/8.6.18/tclInterp.c
    // OPT_INVOKEHID scans i=3 while the path remains original objv[2].
    use crate::InvocationWord::{Dynamic, Literal};
    let current = crate::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let older = crate::model::ingress::static_context_for("tcl8.4")
        .with_command_store(std::sync::Arc::clone(current.commands()));
    let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    let arguments = [
        Literal("invokehidden"),
        Dynamic,
        Literal("-namespace"),
        Dynamic,
        Dynamic,
    ];
    for (context, available) in [(current.as_ref(), true), (&older, false)] {
        let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            crate::InvocationWords::structured(Literal("interp"), &arguments).with_dialect(dialect),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        )
        .resolved()
        .unwrap();
        let scan = selected.authored_source_diagnostic_options().unwrap();
        assert_eq!(selected.semantics.options.positional_prefix_words, 1);
        let [option] = scan.options.as_slice() else {
            panic!("selected original option")
        };
        assert_eq!(option.argument, 2);
        assert_eq!(option.values, Some(3..4));
        assert_eq!(option.available, available);
        assert_eq!(scan.boundary, AuthoredSourceOptionBoundary::Dynamic(4));
    }
}

#[test]
fn source_diagnostic_options_keep_nested_prefixes_and_unknown_boundaries() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    let registry = crate::CommandRegistry::build_default();
    let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    for (words, expected) in [
        (vec!["ensemble", "create", "-command", "::E"], 2),
        (vec!["ensemble", "configure", "-map", "-prefixes", "0"], 3),
    ] {
        let args = words
            .iter()
            .map(|word| crate::InvocationWord::Literal(word))
            .collect::<Vec<_>>();
        let invocation = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("namespace"),
                    &args,
                )
                .with_dialect(dialect),
                Some(tcl_dialect::model::SurfaceQuery::core(
                    tcl_dialect::model::Family::Tcl,
                    "8.6",
                )),
            )
            .resolved()
            .unwrap();
        let scan = invocation.authored_source_diagnostic_options().unwrap();
        assert_eq!(scan.options.len(), 1, "{words:?}: {scan:?}");
        assert_eq!(scan.options[0].argument, expected);
    }
    let args = [
        crate::InvocationWord::Literal("-nocase"),
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Literal("-all"),
    ];
    let invocation = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(crate::InvocationWord::Literal("regexp"), &args)
                .with_dialect(dialect),
            Some(tcl_dialect::model::SurfaceQuery::core(
                tcl_dialect::model::Family::Tcl,
                "8.6",
            )),
        )
        .resolved()
        .unwrap();
    let scan = invocation.authored_source_diagnostic_options().unwrap();
    assert_eq!(scan.options.len(), 1);
    assert_eq!(
        scan.boundary,
        super::super::AuthoredSourceOptionBoundary::Dynamic(1)
    );
}
