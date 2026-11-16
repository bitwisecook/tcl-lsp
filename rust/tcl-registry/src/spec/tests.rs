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

#[test]
fn option_prefix_requires_all_declared_fixed_value_words() {
    use crate::InvocationWord::{Dynamic, Expanded, Literal};
    use crate::hover::{OptionArg, OptionArity, OptionSpec, OptionValue};
    let one = OptionSpec {
        name: "-one",
        value: OptionValue::value("value"),
        ..OptionSpec::DEFAULT
    };
    let pair = OptionSpec {
        name: "-pair",
        value: OptionValue::Takes(OptionArg {
            arity: OptionArity::Fixed(2),
            ..OptionArg::DEFAULT
        }),
        ..OptionSpec::DEFAULT
    };
    let options = [&one, &pair];
    for (words, expected) in [
        (vec![Literal("-one")], None),
        (vec![Literal("-one"), Expanded], None),
        (vec![Literal("-pair")], None),
        (vec![Literal("-pair"), Dynamic], None),
        (vec![Literal("-one"), Dynamic], Some(2)),
        (vec![Literal("-pair"), Dynamic, Literal("-one")], Some(3)),
        (
            vec![Literal("-pair"), Dynamic, Dynamic, Literal("body")],
            Some(3),
        ),
    ] {
        assert_eq!(
            super::leading_option_word_count_for_arguments(
                &options,
                crate::InvocationArguments::structured(&words),
                crate::abbrev::PrefixMatching::Enabled,
                0,
            ),
            expected,
            "{words:?}"
        );
    }
}

#[test]
fn option_prefix_keeps_zero_width_terminator_hook_and_reserved_suffix_rules() {
    use crate::InvocationWord::{Dynamic, Literal};
    use crate::hover::{OptionArg, OptionArity, OptionSpec, OptionValue, OptionValueOutcome};
    fn rest(args: &[&str], start: usize) -> OptionValueOutcome {
        OptionValueOutcome {
            words: args.len() - start,
            invalid: None,
        }
    }
    let options = [
        OptionSpec {
            name: "-zero",
            value: OptionValue::fixed(0, crate::ArgRole::Value, ""),
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-one",
            value: OptionValue::value("value"),
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-rest",
            value: OptionValue::Takes(OptionArg {
                arity: OptionArity::Hook(rest),
                ..OptionArg::DEFAULT
            }),
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "--",
            ..OptionSpec::DEFAULT
        },
    ];
    let options = options.iter().collect::<Vec<_>>();
    for (words, reserved, expected) in [
        (vec![Literal("-zero")], 0, Some(1)),
        (vec![Literal("--"), Dynamic], 0, Some(1)),
        (
            vec![Literal("-rest"), Literal("a"), Literal("b")],
            0,
            Some(3),
        ),
        (vec![Literal("-rest"), Dynamic], 0, None),
        (vec![Literal("-one"), Dynamic, Literal("body")], 1, Some(2)),
        (vec![Dynamic], 1, Some(0)),
    ] {
        assert_eq!(
            super::leading_option_word_count_for_arguments(
                &options,
                crate::InvocationArguments::structured(&words),
                crate::abbrev::PrefixMatching::Enabled,
                reserved,
            ),
            expected,
            "{words:?} reserved={reserved}"
        );
    }
}

#[test]
fn case_option_effects_preserve_the_selected_value_width() {
    use super::{CaseListSpec, CaseMatchMode};
    use crate::hover::{OptionSpec, OptionValue};
    use crate::option_effect::{EffectAxis, OptionEffect, OptionEffectKind};

    // Descriptor/source projection: the unknown option payload has one
    // authentic ordinal. Its effect cannot promote it to the subject.
    let option = OptionSpec {
        name: "-mode",
        value: OptionValue::value("value"),
        effect: Some(OptionEffect {
            kind: OptionEffectKind::Selects(EffectAxis::Selection(CaseMatchMode::Exact)),
            family: "matching",
        }),
        ..OptionSpec::DEFAULT
    };
    let words = [
        Some("-mode"),
        None,
        Some("subject"),
        Some("pattern"),
        Some("body"),
    ];
    let invocation = CaseListSpec::SWITCH
        .source_invocation_values(&words, &[&option], None)
        .unwrap();
    assert_eq!(invocation.subject_index, Some(2));
    assert_eq!(invocation.inline_clause_start, Some(3));
    assert_eq!(invocation.mode, CaseMatchMode::Exact);
    assert!(
        CaseListSpec::SWITCH
            .source_invocation_values(&[Some("-mode")], &[&option], None)
            .is_none()
    );
}
use tcl_dialect::model::{Family, SurfaceQuery};

use super::*;
use crate::registry::CommandRegistry;

#[test]
fn byte_subcommand_resolution_retains_dialect_and_operand_boundaries() {
    // Native proof: naming.selector.selected-byte-descriptor-lookup
    // docs/design/analysis/name-resolution-proofs/selected-byte-descriptor-lookup.md
    let registry = CommandRegistry::build_default();
    let spec = registry.get("string").expect("registered string");
    let old = Some(SurfaceQuery::core(Family::Tcl, "8.6"));
    let current = Some(SurfaceQuery::core(Family::Tcl, "9.1"));
    assert_eq!(
        spec.resolve_subcommand_bytes_for_dialect(b"in", old)
            .map(|s| s.name),
        Some("index")
    );
    assert!(
        spec.resolve_subcommand_bytes_for_dialect(b"insert", old)
            .is_none()
    );
    assert!(
        spec.resolve_subcommand_bytes_for_dialect(b"in", current)
            .is_none()
    );
    assert_eq!(
        spec.resolve_subcommand_bytes_for_dialect(b"insert", current)
            .map(|s| s.name),
        Some("insert")
    );
    for word in [
        b"index\0tail".as_slice(),
        b"index\xff",
        b"\xc0\x80",
        b"\xed\xa0\x80",
    ] {
        assert!(
            spec.resolve_subcommand_bytes_for_dialect(word, current)
                .is_none()
        );
    }
    assert_eq!(
        spec.resolve_subcommand_for_dialect("index", current)
            .map(|s| s.name),
        spec.resolve_subcommand_bytes_for_dialect(b"index", current)
            .map(|s| s.name)
    );
}

#[test]
fn a_writer_declared_only_by_its_assigned_variable_index_must_say_how_it_writes() {
    let spec = CommandSpec {
        name: "w",
        assigns_variable_at: Some(0),
        ..CommandSpec::DEFAULT
    };
    assert_eq!(spec.unclassified_variable_writers(), vec!["w".to_owned()]);
    let classified = CommandSpec {
        traits: Traits::UNCONDITIONAL_VARIABLE_WRITE,
        ..spec
    };
    assert_eq!(classified.unclassified_variable_writers().len(), 0);
}

// Optional trailing argument names.
//
// A quick fix that appends a documented optional argument learns both how
// many words it may append and what to call them from the synopsis, so a
// dialect whose form documents fewer words gets a narrower offer.

#[test]
fn optional_trailing_placeholders_takes_the_trailing_run() {
    assert_eq!(
        optional_trailing_placeholders("catch script ?resultVarName? ?optionsVarName?"),
        vec!["resultVarName", "optionsVarName"]
    );
    // A required word terminates the run — these are the words a caller
    // may append *without* also having to supply something in between.
    assert_eq!(
        optional_trailing_placeholders("cmd ?a? required ?b?"),
        vec!["b"]
    );
    // No optional tail at all.
    assert_eq!(optional_trailing_placeholders("puts string").len(), 0);
    // A variadic tail is not a fixed slot list.
    assert_eq!(optional_trailing_placeholders("cmd ?arg ...?").len(), 0);
    assert_eq!(optional_trailing_placeholders("cmd ?...?").len(), 0);
}

#[test]
fn optional_trailing_arg_names_narrows_with_the_dialect() {
    let registry = CommandRegistry::build_default();
    // The C Tcl row, selected by release: Jim has a row of its own.
    let names = |release| {
        let query = Some(SurfaceQuery::core(Family::Tcl, release));
        registry
            .get_for_surface("catch", query)
            .expect("catch is a registry command")
            .optional_trailing_arg_names(query, None)
    };
    // Tcl 8.5 onward documents `catch script ?resultVarName? ?optionsVarName?`.
    assert_eq!(names("9.0"), vec!["resultVarName", "optionsVarName"]);
    // Tcl 8.4's `catch script ?varName?` has no options dictionary, so a
    // consumer running under that dialect never offers a word the release
    // has no argument slot for.
    assert_eq!(names("8.4"), vec!["varName"]);
}

// `FormSpec` lifecycle in the form-selection queries.
//
// A `FormSpec` carries its own lifecycle, so a synopsis a later release
// added must not be shown to — nor its optional words offered to — a
// document pinned below it.  The widest form is declared first, matching
// the shipped convention (`lassign`'s 8.6+ shape leads its 8.5 one).

const VERSIONED_FORMS: &[crate::hover::FormSpec] = &[
    crate::hover::FormSpec {
        synopsis: "fauxform target ?resultVar? ?optionsVar?",
        lifecycle: Lifecycle::introduced_in("2.0"),
        ..crate::hover::FormSpec::DEFAULT
    },
    crate::hover::FormSpec {
        synopsis: "fauxform target ?resultVar?",
        ..crate::hover::FormSpec::DEFAULT
    },
];

fn versioned_form_spec() -> CommandSpec {
    CommandSpec {
        name: "fauxform",
        forms: VERSIONED_FORMS,
        ..CommandSpec::DEFAULT
    }
}

#[test]
fn primary_synopsis_skips_a_form_the_floor_predates() {
    let spec = versioned_form_spec();
    // At 1.0 the two-optional-word form does not exist yet, so the usage
    // hint shows the shape that release actually has.
    assert_eq!(
        spec.primary_synopsis(Some("1.0")),
        Some("fauxform target ?resultVar?")
    );
    // From 2.0 the newer form leads, exactly as declaration order says.
    assert_eq!(
        spec.primary_synopsis(Some("2.0")),
        Some("fauxform target ?resultVar? ?optionsVar?")
    );
    // No floor resolved = permissive, i.e. the pre-existing behaviour.
    assert_eq!(
        spec.primary_synopsis(None),
        Some("fauxform target ?resultVar? ?optionsVar?")
    );
}

#[test]
fn optional_trailing_arg_names_skips_a_form_the_floor_predates() {
    let spec = versioned_form_spec();
    // A quick fix must not offer to append `optionsVar` to a document
    // whose Fauxpkg is 1.x: that release has no argument slot for it.
    assert_eq!(
        spec.optional_trailing_arg_names(Some(SurfaceQuery::core(Family::Tcl, "9.0")), Some("1.0")),
        vec!["resultVar"]
    );
    assert_eq!(
        spec.optional_trailing_arg_names(Some(SurfaceQuery::core(Family::Tcl, "9.0")), Some("2.0")),
        vec!["resultVar", "optionsVar"]
    );
    assert_eq!(
        spec.optional_trailing_arg_names(Some(SurfaceQuery::core(Family::Tcl, "9.0")), None),
        vec!["resultVar", "optionsVar"]
    );
}

#[test]
fn a_retired_form_stops_being_documented() {
    const RETIRED: &[crate::hover::FormSpec] = &[crate::hover::FormSpec {
        synopsis: "fauxform target ?resultVar?",
        lifecycle: Lifecycle::UNSPECIFIED.retired_from("3.0"),
        ..crate::hover::FormSpec::DEFAULT
    }];
    let spec = CommandSpec {
        name: "fauxform",
        forms: RETIRED,
        ..CommandSpec::DEFAULT
    };
    assert!(spec.primary_synopsis(Some("3.0")).is_none());
    assert_eq!(
        spec.optional_trailing_arg_names(Some(SurfaceQuery::core(Family::Tcl, "9.0")), Some("3.0"))
            .len(),
        0
    );
    // Retirement is exclusive, so 2.9 still documents it.
    assert_eq!(
        spec.primary_synopsis(Some("2.9")),
        Some("fauxform target ?resultVar?")
    );
}

// Command-level argument-value gates.
//
// A literal value can be gated by its own `ArgValue::lifecycle`, by a
// positional `versioned_arg_values` entry, or by both; the accessor must
// apply every gate that names it.

const GATED_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "old",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "new",
        lifecycle: Lifecycle::introduced_in("2.0"),
        ..ArgValue::DEFAULT
    },
];

const UNDECLARED_GATE: &[VersionedArgValue] = &[VersionedArgValue {
    index: 0,
    value: "undeclared",
    lifecycle: Lifecycle::introduced_in("3.0"),
}];

fn gated_spec() -> CommandSpec {
    CommandSpec {
        name: "widget",
        arg_values: &[(0, GATED_VALUES)],
        versioned_arg_values: UNDECLARED_GATE,
        ..CommandSpec::DEFAULT
    }
}

#[test]
fn command_arg_value_availability_honours_both_gates() {
    let spec = gated_spec();
    // The value's own lifecycle …
    assert!(!spec.arg_value_available_for_version(0, "new", Some("1.0")));
    assert!(spec.arg_value_available_for_version(0, "new", Some("2.0")));
    // … a positional gate for a value the table does not declare …
    assert!(!spec.arg_value_available_for_version(0, "undeclared", Some("2.0")));
    assert!(spec.arg_value_available_for_version(0, "undeclared", Some("3.0")));
    // … an ungated value, an unknown value, and an unknown target are
    // all permissive.
    assert!(spec.arg_value_available_for_version(0, "old", Some("1.0")));
    assert!(spec.arg_value_available_for_version(0, "anything", Some("1.0")));
    assert!(spec.arg_value_available_for_version(0, "new", None));
    // A different argument index shares no gates.
    assert!(spec.arg_value_available_for_version(1, "new", Some("1.0")));
}

#[test]
fn available_arg_values_filters_the_declared_table() {
    let spec = gated_spec();
    let names = |version: Option<&str>| -> Vec<&str> {
        spec.available_arg_values_at(0, version)
            .iter()
            .map(|value| value.value)
            .collect()
    };
    assert_eq!(names(Some("1.0")), vec!["old"]);
    assert_eq!(names(Some("2.0")), vec!["old", "new"]);
    assert_eq!(names(None), vec!["old", "new"]);
}

// Second-level subcommand gates.

const OPS: &[SubSubCommand] = &[
    SubSubCommand {
        name: "classic",
        ..SubSubCommand::DEFAULT
    },
    SubSubCommand {
        name: "modern",
        lifecycle: Lifecycle::introduced_in("2.0"),
        ..SubSubCommand::DEFAULT
    },
];

const OP_HOLDER: SubCommand = SubCommand {
    name: "op",
    sub_subcommands: OPS,
    ..SubCommand::DEFAULT
};

#[test]
fn nested_keyword_resolution_honours_the_authored_prefix_policy() {
    let exact = SubCommand {
        prefix_matching: PrefixMatching::Strict,
        ..OP_HOLDER
    };
    assert_eq!(
        OP_HOLDER.resolve_sub_subcommand("cla").map(|sub| sub.name),
        Some("classic")
    );
    assert!(exact.resolve_sub_subcommand("cla").is_none());
    assert_eq!(
        exact.resolve_sub_subcommand("classic").map(|sub| sub.name),
        Some("classic")
    );
    assert!(
        exact
            .resolve_sub_subcommand_gated("mod", None, Some("2.0"))
            .is_none()
    );
    assert_eq!(
        exact
            .resolve_sub_subcommand_gated("modern", None, Some("2.0"))
            .map(|sub| sub.name),
        Some("modern")
    );
    assert!(
        exact
            .resolve_sub_subcommand_gated("modern", None, Some("1.0"))
            .is_none()
    );
}

#[test]
fn sub_subcommand_resolution_honours_the_package_floor() {
    // Below the introduction the operation does not exist, so it neither
    // resolves nor competes for a prefix …
    assert!(
        OP_HOLDER
            .resolve_sub_subcommand_gated("modern", None, Some("1.0"))
            .is_none()
    );
    assert_eq!(
        OP_HOLDER
            .resolve_sub_subcommand_gated("modern", None, Some("2.0"))
            .map(|op| op.name),
        Some("modern")
    );
    // … and an unknown target stays permissive, matching every other
    // availability query.
    assert_eq!(
        OP_HOLDER
            .resolve_sub_subcommand_gated("modern", None, None)
            .map(|op| op.name),
        Some("modern")
    );
    let names = |version: Option<&str>| -> Vec<&str> {
        OP_HOLDER
            .available_sub_subcommands(None, version)
            .iter()
            .map(|op| op.name)
            .collect()
    };
    assert_eq!(names(Some("1.0")), vec!["classic"]);
    assert_eq!(names(Some("2.0")), vec!["classic", "modern"]);
}
