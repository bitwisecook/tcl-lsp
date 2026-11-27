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

//! Selected option tables: reserved positional values, option value roles and the structured option boundary.

use super::*;

#[test]
fn selected_subcommand_options_preserve_reserved_positional_values() {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let registry = crate::CommandRegistry::build_default();
    let dialect = tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    for (arguments, expected) in [
        (vec!["equal", "-nocase", "-nocase", "other"], 1),
        (vec!["compare", "-nocase", "-nocase", "other"], 1),
        (vec!["equal", "-nocase", "-nocase", "first", "other"], 2),
        (vec!["equal", "-length", "2", "-nocase", "other"], 2),
        (vec!["equal", "first", "-nocase"], 0),
    ] {
        let words = arguments
            .iter()
            .map(|value| crate::InvocationWord::Literal(value))
            .collect::<Vec<_>>();
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("string"),
                    &words,
                )
                .with_dialect(dialect),
                dialect.authoring_query(),
            )
            .resolved()
            .unwrap();
        assert_eq!(selected.semantics.options.reserved_trailing_words, 2);
        assert_eq!(
            selected
                .semantics
                .options
                .leading_word_count(selected.words.arguments().slice_from(1)),
            Some(expected),
            "{arguments:?}"
        );
    }
}

#[test]
fn selected_option_value_roles_reach_owned_facts_without_classifying_result_data() {
    let registry = crate::CommandRegistry::build_default();
    for arguments in [
        vec!["name", "description", "-body", "expr $x+1", "-result", ""],
        vec!["name", "description", "-result", "-body"],
    ] {
        let resolved = registry
            .resolve_invocation("tcltest::test", &arguments, None)
            .expect("available declared option layout");
        let expected = (arguments.len() == 6).then_some((3, ArgRole::Body));
        for facts in [resolved.facts(), resolved.facts_after_success()] {
            let bodies: Vec<_> = facts
                .arg_roles
                .into_iter()
                .filter(|(_, role)| *role == ArgRole::Body)
                .collect();
            assert_eq!(bodies, expected.into_iter().collect::<Vec<_>>());
        }
    }
}

#[test]
fn selected_option_value_role_width_is_independent_of_unknown_payload() {
    use crate::InvocationWord::{Dynamic, Literal};
    let registry = CommandRegistry::build_default();
    let arguments = [
        Literal("name"),
        Literal("description"),
        Literal("-body"),
        Dynamic,
    ];
    let resolved = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(Literal("tcltest::test"), &arguments),
            None,
        )
        .resolved()
        .expect("selected package layout");
    assert!(resolved.facts().arg_roles.contains(&(3, ArgRole::Body)));
    let unknown = [
        Literal("name"),
        Literal("description"),
        Dynamic,
        Literal("expr $x"),
    ];
    let resolved = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(Literal("tcltest::test"), &unknown),
            None,
        )
        .resolved()
        .expect("unresolved option word remains an invocation");
    assert!(!resolved.facts().arg_roles_complete);
    assert!(!resolved.facts().arg_roles.contains(&(3, ArgRole::Body)));
}

fn structured_boundary_options() -> [OptionSpec; 2] {
    [
        OptionSpec {
            name: "-offset",
            aliases: &["-begin"],
            min_abbrev: Some(4),
            value: crate::hover::OptionValue::fixed(2, ArgRole::Value, "value"),
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "--",
            ..OptionSpec::DEFAULT
        },
    ]
}

fn structured_boundary_unknown_value() -> [crate::InvocationWord<'static>; 4] {
    use crate::InvocationWord::{Dynamic, Literal};
    [
        Literal("-offset"),
        Dynamic,
        Literal("2"),
        Literal("position"),
    ]
}

#[test]
fn structured_option_boundary_shares_alias_abbreviation_and_value_span_rules() {
    // Implementation contract: naming.studio.source-word-role-projection
    // docs/design/analysis/name-resolution-proofs/studio-source-word-role-projection.md

    use crate::InvocationWord::{Dynamic, Literal};
    let options = structured_boundary_options();
    let available = options.iter().collect::<Vec<_>>();
    let selected = InvocationOptions {
        availability: InvocationAvailability::default(),
        parent_surface: None,
        form_surface: None,
        prefix_matching: crate::abbrev::PrefixMatching::Enabled,
        placement: crate::spec::OptionPlacement::Leading,
        positional_prefix_words: 0,
        reserved_trailing_words: 0,
        case_list: None,
        base: &options,
        form: &[],
    };
    for spelling in ["-offset", "-begin", "-off", "-beg"] {
        let arguments = [
            Literal(spelling),
            Literal("1"),
            Literal("2"),
            Literal("--"),
            Dynamic,
        ];
        assert_eq!(
            crate::spec::leading_option_word_count_for_arguments(
                &available,
                InvocationArguments::structured(&arguments),
                crate::abbrev::PrefixMatching::Enabled,
                0,
            ),
            Some(4),
            "{spelling}"
        );
        let occurrences = selected
            .prefix_occurrences(InvocationArguments::structured(&arguments))
            .unwrap();
        assert_eq!(occurrences.len(), 2);
        assert_eq!(occurrences[0].argument_index, 0);
        assert_eq!(occurrences[0].option.unwrap().name, "-offset");
        assert_eq!(occurrences[0].values, 1..3);
        assert_eq!(occurrences[1].argument_index, 3);
        assert!(occurrences[1].option.is_none());
        assert!(
            crate::spec::resolve_option_prefix_with(
                &options,
                spelling,
                crate::abbrev::PrefixMatching::Enabled,
            )
            .is_some()
        );
    }
    assert!(
        crate::spec::resolve_option_prefix_with(
            &options,
            "-of",
            crate::abbrev::PrefixMatching::Enabled,
        )
        .is_none()
    );
    let unknown_value = structured_boundary_unknown_value();
    assert_eq!(
        selected
            .prefix_occurrences(InvocationArguments::structured(&unknown_value))
            .unwrap()[0]
            .values,
        1..3
    );
    assert!(
        selected
            .prefix_occurrences(InvocationArguments::structured(&[
                Dynamic,
                Literal("position")
            ]))
            .is_none()
    );
    assert_eq!(
        crate::spec::leading_option_word_count_for_arguments(
            &available,
            InvocationArguments::structured(&unknown_value),
            crate::abbrev::PrefixMatching::Enabled,
            0,
        ),
        Some(3),
        "fixed value arity retains positions independently of dynamic bytes"
    );
    let terminator = [Literal("-offset"), Literal("--"), Dynamic];
    assert_eq!(
        crate::spec::leading_option_word_count_for_arguments(
            &available,
            InvocationArguments::structured(&terminator),
            crate::abbrev::PrefixMatching::Enabled,
            0,
        ),
        Some(3),
        "a dash value is consumed before subsequent option scanning"
    );
}
