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

//! List, enum, pattern, format and expression operands in the authored source.

use super::*;

#[test]
fn conditional_list_argument_layout_keeps_values_expansion_and_other_descriptors_distinct() {
    let registry = crate::CommandRegistry::build_default();
    let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    let arguments = [
        crate::InvocationWord::Literal("source"),
        crate::InvocationWord::Dynamic,
    ];
    let selected = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(crate::InvocationWord::Literal("list"), &arguments)
                .with_dialect(dialect),
            dialect.authoring_query(),
        )
        .resolved()
        .unwrap();
    assert_eq!(selected.authored_source_list_arguments(), Some(0..2));
    assert!(selected.authored_source_ascii_list_result().is_none());
    let arguments = [crate::InvocationWord::Expanded];
    let selected = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(crate::InvocationWord::Literal("list"), &arguments)
                .with_dialect(dialect),
            dialect.authoring_query(),
        )
        .resolved()
        .unwrap();
    assert!(selected.authored_source_list_arguments().is_none());
    let arguments = [crate::InvocationWord::Literal("VALUE")];
    let selected = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(
                crate::InvocationWord::Literal("concat"),
                &arguments,
            )
            .with_dialect(dialect),
            dialect.authoring_query(),
        )
        .resolved()
        .unwrap();
    assert!(selected.authored_source_list_arguments().is_none());
}

#[test]
fn source_list_operand_templates_keep_splices_separate_from_argv_and_results() {
    let registry = crate::CommandRegistry::build_default();
    let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    for (command, arguments, expected) in [
        (
            "list",
            vec![
                crate::InvocationWord::Literal("superclass"),
                crate::InvocationWord::Expanded,
            ],
            Some(vec![(0, false), (1, true)]),
        ),
        (
            "list",
            vec![crate::InvocationWord::Dynamic],
            Some(vec![(0, false)]),
        ),
        ("list", vec![crate::InvocationWord::Opaque], None),
        (
            "split",
            vec![crate::InvocationWord::Literal("superclass")],
            None,
        ),
        (
            "lreverse",
            vec![crate::InvocationWord::Literal("superclass")],
            None,
        ),
    ] {
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal(command),
                    &arguments,
                )
                .with_dialect(dialect),
                dialect.authoring_query(),
            )
            .resolved()
            .unwrap();
        assert_eq!(
            selected.authored_source_list_operand_templates(),
            expected,
            "{command}"
        );
        if arguments.contains(&crate::InvocationWord::Expanded) {
            assert!(selected.authored_source_list_arguments().is_none());
            assert!(selected.authored_source_ascii_list_result().is_none());
        }
    }
}

#[test]
fn source_enum_projection_keeps_literal_and_native_byte_facets_separate() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let mut registry = CommandRegistry::build_default();
    registry.insert(crate::CommandSpec {
        name: "enum_source_test",
        arg_values: &[(
            0,
            &[crate::hover::ArgValue {
                value: "ready",
                ..crate::hover::ArgValue::DEFAULT
            }],
        )],
        ..crate::CommandSpec::DEFAULT
    });
    for (value, expected) in [
        (crate::InvocationWord::Literal("ready"), vec![0]),
        (crate::InvocationWord::KnownBytes(b"ready"), vec![0]),
        (crate::InvocationWord::Literal("other"), vec![]),
        (crate::InvocationWord::KnownBytes(b"ready\0tail"), vec![]),
        (crate::InvocationWord::Dynamic, vec![]),
        (crate::InvocationWord::Expanded, vec![]),
        (crate::InvocationWord::Opaque, vec![]),
    ] {
        let arguments = [value];
        let selected = registry.resolve_structured_invocation(
            crate::InvocationWords::structured(
                crate::InvocationWord::Literal("enum_source_test"),
                &arguments,
            ),
            None,
        );
        assert_eq!(
            selected
                .resolved()
                .unwrap()
                .authored_source_enum_arguments(),
            expected
        );
    }
}

#[test]
fn authored_source_patterns_keep_selected_context_and_option_languages() {
    // naming.core.original-pattern-retained-context
    // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
    use crate::InvocationWord::{Dynamic, Literal};
    let driver = crate::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let older = crate::model::ingress::resolve_environment("tcl8.4").default_context_registry();
    let older = older.with_command_store(driver.commands().snapshot().shared_registry());
    let dialect = crate::InvocationDialect::of_profile(driver.commands().profile().unwrap());
    let arguments = [
        Literal("-regexp"),
        Literal("-stride"),
        Literal("2"),
        Dynamic,
        Literal("a+"),
    ];
    for (context, expected) in [(&*driver, Some(4)), (&older, None)] {
        let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            crate::InvocationWords::structured(Literal("lsearch"), &arguments)
                .with_dialect(dialect),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        )
        .resolved()
        .unwrap();
        let patterns = selected.authored_source_pattern_arguments();
        assert_eq!(
            patterns
                .as_ref()
                .and_then(|patterns| patterns.first().map(|pattern| pattern.index)),
            expected
        );
        if expected.is_some() {
            assert_eq!(
                patterns.unwrap(),
                vec![crate::patterns::PatternArg {
                    index: 4,
                    kind: crate::patterns::PatternType::Regex
                }]
            );
        } else {
            assert!(patterns.is_none());
        }
    }
    for (selector, expected) in [
        ("-glob", Some(crate::patterns::PatternType::Glob)),
        ("-regexp", Some(crate::patterns::PatternType::Regex)),
        ("-exact", None),
        ("-sorted", None),
    ] {
        let arguments = [Literal(selector), Dynamic, Literal("a+")];
        let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            driver.commands(),
            Some(driver.context()),
            crate::InvocationWords::structured(Literal("lsearch"), &arguments)
                .with_dialect(dialect),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        )
        .resolved()
        .unwrap();
        let patterns = selected.authored_source_pattern_arguments().unwrap();
        assert_eq!(
            patterns
                .first()
                .map(|pattern| (pattern.index, pattern.kind)),
            expected.map(|kind| (2, kind))
        );
    }
}

#[test]
fn authored_source_patterns_keep_payload_unknown_and_refuse_uncertain_layout() {
    // naming.core.original-pattern-retained-context
    // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
    use crate::InvocationWord::{Dynamic, Expanded, Literal};
    use crate::patterns::PatternType::{Glob, Regex};
    let context = crate::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let dialect = crate::InvocationDialect::of_profile(context.commands().profile().unwrap());
    for (head, arguments, expected) in [
        (
            "regexp",
            vec![Literal("-start"), Literal("0"), Literal("a+"), Dynamic],
            vec![(2, Regex)],
        ),
        (
            "regsub",
            vec![
                Literal("-all"),
                Literal("a+"),
                Dynamic,
                Literal("replacement"),
            ],
            vec![(1, Regex)],
        ),
        (
            "string",
            vec![Literal("match"), Literal("-nocase"), Literal("a*"), Dynamic],
            vec![(2, Glob)],
        ),
        (
            "glob",
            vec![Literal("-directory"), Dynamic, Literal("a*"), Dynamic],
            vec![(2, Glob), (3, Glob)],
        ),
    ] {
        let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            crate::InvocationWords::structured(Literal(head), &arguments).with_dialect(dialect),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        )
        .resolved()
        .unwrap();
        let patterns = selected.authored_source_pattern_arguments().unwrap();
        assert_eq!(
            patterns
                .iter()
                .map(|pattern| (pattern.index, pattern.kind))
                .collect::<Vec<_>>(),
            expected,
            "{head}"
        );
        assert!(selected.words.arguments().literal_values().is_none());
    }
    for (head, arguments) in [
        ("regexp", vec![Dynamic, Literal("a+"), Dynamic]),
        ("lsearch", vec![Dynamic, Dynamic, Literal("a+")]),
        ("glob", vec![Dynamic, Literal("a*")]),
        ("string", vec![Dynamic, Literal("a*"), Dynamic]),
        ("lsearch", vec![Literal("-regexp"), Expanded, Literal("a+")]),
    ] {
        let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            crate::InvocationWords::structured(Literal(head), &arguments).with_dialect(dialect),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        )
        .resolved()
        .unwrap();
        assert!(
            selected.authored_source_pattern_arguments().is_none(),
            "{head}: {arguments:?}"
        );
    }
}

#[test]
fn authored_source_formats_keep_selected_context_and_effective_ordinals() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    use crate::InvocationWord::{Dynamic, Literal};
    let driver = crate::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let older = crate::model::ingress::resolve_environment("tcl8.4").default_context_registry();
    let older = older.with_command_store(driver.commands().snapshot().shared_registry());
    let dialect = crate::InvocationDialect::of_profile(driver.commands().profile().unwrap());
    let arguments = [
        Literal("scan"),
        Literal("2020"),
        Literal("-format"),
        Literal("%Y"),
    ];
    for (context, expected) in [(&*driver, Some(3)), (&older, None)] {
        let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            crate::InvocationWords::structured(Literal("clock"), &arguments).with_dialect(dialect),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        )
        .resolved()
        .unwrap();
        let formats = selected.authored_source_format_arguments();
        assert_eq!(
            formats
                .as_ref()
                .and_then(|formats| formats.first().map(|format| format.index)),
            expected
        );
        if expected.is_some() {
            let formats = formats.unwrap();
            assert_eq!(formats.len(), 1);
            assert_eq!(formats[0].kind, crate::FormatType::Clock);
        } else {
            assert!(formats.is_none_or(|formats| formats.is_empty()));
        }
    }
    for (head, arguments, index, kind) in [
        (
            "format",
            vec![Literal("%d"), Dynamic],
            0,
            crate::FormatType::Sprintf,
        ),
        (
            "binary",
            vec![Literal("format"), Literal("c"), Dynamic],
            1,
            crate::FormatType::Binary,
        ),
    ] {
        let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            driver.commands(),
            Some(driver.context()),
            crate::InvocationWords::structured(Literal(head), &arguments).with_dialect(dialect),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        )
        .resolved()
        .unwrap();
        let formats = selected.authored_source_format_arguments().unwrap();
        assert_eq!(formats.len(), 1);
        assert_eq!(formats[0].index, index);
        assert_eq!(formats[0].kind, kind);
        assert!(!formats[0].scan);
    }
}

#[test]
fn source_expression_roles_keep_selected_tail_and_unknown_cardinality() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    use crate::InvocationWord::{Dynamic, Expanded, Literal};
    let context = crate::model::ingress::static_context_for("tcl8.6");
    for (head, arguments, expected, concatenates) in [
        ("if", vec![Dynamic, Literal("{}")], vec![0], false),
        ("expr", vec![Dynamic, Literal("+"), Dynamic], vec![0], true),
        ("puts", vec![Dynamic], vec![], false),
    ] {
        let resolution = context.commands().resolve_structured_invocation(
            crate::InvocationWords::structured(Literal(head), &arguments),
            Some(context.context().authoring_query()),
        );
        let selected = resolution.resolved().unwrap();
        let expressions = selected
            .authored_source_expression_arguments()
            .unwrap_or_else(|| panic!("source role projection declined {head}"));
        assert_eq!(expressions.arguments, expected, "{head}");
        assert_eq!(expressions.concatenates, concatenates, "{head}");
    }
    let arguments = [Expanded];
    let resolution = context.commands().resolve_structured_invocation(
        crate::InvocationWords::structured(Literal("expr"), &arguments),
        Some(context.context().authoring_query()),
    );
    assert!(
        resolution
            .resolved()
            .unwrap()
            .authored_source_expression_arguments()
            .is_none()
    );
}
