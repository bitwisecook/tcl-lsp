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

//! Format-string and pattern arguments: families, source-aware layouts behind options, and the `lsearch` pattern resolver.

use super::*;

/// The format families answer from registry data alone:
/// the *position* from the `FormatString` / `ScanFormat` roles, the
/// *family* from `format_string_type`.
#[test]
fn format_string_args_cover_every_family() {
    use crate::patterns::FormatType;
    let reg = crate::model::ingress::static_context_for("tcl9.1").commands();
    let found = |name: &str, args: &[&str]| -> Vec<(usize, FormatType, bool)> {
        reg.format_string_args(name, args)
            .into_iter()
            .map(|f| (f.index, f.kind, f.scan))
            .collect()
    };
    // TP — a fixed argument index.
    assert_eq!(
        found("format", &["%d", "7"]),
        vec![(0, FormatType::Sprintf, false)]
    );
    // TP — a resolver-computed index, scan direction.
    assert_eq!(
        found("scan", &["$s", "%d", "v"]),
        vec![(1, FormatType::Sprintf, true)]
    );
    // TP — subcommand-relative indices.
    assert_eq!(
        found("binary", &["format", "c3", "$l"]),
        vec![(1, FormatType::Binary, false)]
    );
    assert_eq!(
        found("binary", &["scan", "$v", "c3", "out"]),
        vec![(2, FormatType::Binary, true)]
    );
    // TP — an *option value* position, both directions.
    assert_eq!(
        found("clock", &["format", "$t", "-format", "%Y"]),
        vec![(3, FormatType::Clock, false)]
    );
    assert_eq!(
        found("clock", &["scan", "$s", "-format", "%Y"]),
        vec![(3, FormatType::Clock, true)]
    );
    // TP — `regsub`'s replacement template, shifted past its switches.
    assert_eq!(
        found("regsub", &["-all", "--", "e", "$s", "X", "out"]),
        vec![(4, FormatType::Regsub, false)]
    );
    // FN guard — the explicitly global spellings resolve identically.
    assert_eq!(
        found("::format", &["%d", "7"]),
        found("format", &["%d", "7"])
    );
    assert_eq!(
        found("::clock", &["format", "$t", "-format", "%Y"]),
        found("clock", &["format", "$t", "-format", "%Y"])
    );
    // TN — a subcommand with no format string, and an unknown command.
    assert_eq!(found("binary", &["encode", "hex", "$d"]).len(), 0);
    assert_eq!(found("clock", &["seconds"]).len(), 0);
    assert_eq!(found("no::such::command", &["%d"]).len(), 0);
    assert_eq!(found("puts", &["%d"]).len(), 0);
    // TN — `regsub -command` makes that position a callback, not a
    // template, so it declares no replacement word.
    assert!(
        !found("regsub", &["-command", "--", "e", "$s", "cb"])
            .iter()
            .any(|(i, _, _)| *i == 3)
    );
}

#[test]
fn source_aware_format_layouts_abstain_only_while_option_selection_is_dynamic() {
    use crate::invocation_words::{InvocationArguments, InvocationWord};
    use crate::patterns::FormatType;

    let dynamic_prefix = [
        InvocationWord::Dynamic,
        InvocationWord::Literal("{a}"),
        InvocationWord::Dynamic,
        InvocationWord::Literal(r"{\1}"),
    ];
    let fixed_start_value = [
        InvocationWord::Literal("-start"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("{a}"),
        InvocationWord::Dynamic,
        InvocationWord::Literal(r"{\1}"),
    ];
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let registry = crate::model::ingress::static_context_for(dialect).commands();
        assert!(
            registry
                .format_string_args_words(
                    "regsub",
                    InvocationArguments::structured(&dynamic_prefix),
                )
                .is_empty(),
            "{dialect}: $mode could shift regsub's replacement position"
        );
        assert_eq!(
            registry.format_string_args_words(
                "regsub",
                InvocationArguments::structured(&fixed_start_value),
            ),
            vec![FormatStringArg {
                index: 4,
                kind: FormatType::Regsub,
                scan: false,
            }],
            "{dialect}: a known -start still consumes exactly its dynamic value"
        );
    }

    let invalid_command_abbrev = [
        InvocationWord::Literal("-c"),
        InvocationWord::Literal("{a}"),
        InvocationWord::Dynamic,
        InvocationWord::Literal(r"{\1}"),
    ];
    let exact_command = [
        InvocationWord::Literal("-command"),
        InvocationWord::Literal("{a}"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("callback"),
    ];
    let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
    assert!(
        registry
            .format_string_args_words(
                "regsub",
                InvocationArguments::structured(&invalid_command_abbrev),
            )
            .is_empty(),
        "-c is not a Tcl regsub abbreviation and cannot claim a replacement template"
    );
    assert!(
        registry
            .format_string_args_words("regsub", InvocationArguments::structured(&exact_command))
            .is_empty(),
        "only exact Tcl 9 -command selects a callback rather than a replacement template"
    );
}

#[test]
fn clock_format_keeps_fixed_payload_and_option_selection_separate() {
    use crate::invocation_words::{InvocationArguments, InvocationWord};
    use crate::patterns::FormatType;
    let clock = [
        InvocationWord::Literal("format"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("-format"),
        InvocationWord::Literal("%Y"),
    ];
    assert_eq!(
        crate::model::ingress::static_context_for("tcl9.0")
            .commands()
            .format_string_args_words("clock", InvocationArguments::structured(&clock),),
        vec![FormatStringArg {
            index: 3,
            kind: FormatType::Clock,
            scan: false,
        }],
        "a fixed positional time value cannot hide an after-positional option value"
    );
    let unknown_clock_option = [
        InvocationWord::Literal("format"),
        InvocationWord::Dynamic,
        InvocationWord::Dynamic,
        InvocationWord::Literal("%Y"),
    ];
    assert!(
        crate::model::ingress::static_context_for("tcl9.0")
            .commands()
            .format_string_args_words(
                "clock",
                InvocationArguments::structured(&unknown_clock_option),
            )
            .is_empty(),
        "an unknown option word does not establish the format operand"
    );
}

#[test]
fn source_aware_pattern_layouts_abstain_for_dynamic_leading_options() {
    use crate::invocation_words::{InvocationArguments, InvocationWord};

    let dynamic_prefix = [
        InvocationWord::Dynamic,
        InvocationWord::Literal("{a+}"),
        InvocationWord::Dynamic,
    ];
    for name in ["regexp", "regsub"] {
        assert!(
            crate::model::ingress::static_context_for("tcl9.0")
                .commands()
                .pattern_args_words(name, InvocationArguments::structured(&dynamic_prefix))
                .is_empty(),
            "{name}: $mode could be a leading option"
        );
    }
    let glob_dynamic_prefix = [
        InvocationWord::Dynamic,
        InvocationWord::Literal("/tmp"),
        InvocationWord::Literal("*.tcl"),
    ];
    assert!(
        crate::model::ingress::static_context_for("tcl9.0")
            .commands()
            .pattern_args_words(
                "glob",
                InvocationArguments::structured(&glob_dynamic_prefix)
            )
            .is_empty(),
        "glob $mode /tmp *.tcl cannot choose a pattern tail before $mode resolves"
    );
    let string_dynamic_prefix = [
        InvocationWord::Literal("match"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("a*"),
        InvocationWord::Literal("abc"),
    ];
    assert!(
        crate::model::ingress::static_context_for("tcl9.0")
            .commands()
            .pattern_args_words(
                "string",
                InvocationArguments::structured(&string_dynamic_prefix),
            )
            .is_empty(),
        "string match $mode a* abc cannot treat $mode as the pattern"
    );
}

#[test]
fn source_aware_pattern_layouts_accept_proven_options_and_terminators() {
    use crate::invocation_words::{InvocationArguments, InvocationWord};
    use crate::patterns::{PatternArg, PatternType};

    let regexp_start = [
        InvocationWord::Literal("-start"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("a+"),
        InvocationWord::Dynamic,
    ];
    assert_eq!(
        crate::model::ingress::static_context_for("tcl9.0")
            .commands()
            .pattern_args_words("regexp", InvocationArguments::structured(&regexp_start)),
        vec![PatternArg {
            index: 2,
            kind: PatternType::Regex,
        }],
        "a fixed value-taking option may safely carry a dynamic value"
    );
    let non_option_template = [InvocationWord::DynamicNonOption, InvocationWord::Dynamic];
    assert_eq!(
        crate::model::ingress::static_context_for("tcl9.0")
            .commands()
            .pattern_args_words(
                "regexp",
                InvocationArguments::structured(&non_option_template),
            ),
        vec![PatternArg {
            index: 0,
            kind: PatternType::Regex,
        }],
        "a template with a direct non-dash prefix remains a positional pattern"
    );
    let glob_terminator = [
        InvocationWord::Literal("--"),
        InvocationWord::Literal("-literal"),
    ];
    assert_eq!(
        crate::model::ingress::static_context_for("tcl9.0")
            .commands()
            .pattern_args_words("glob", InvocationArguments::structured(&glob_terminator)),
        vec![PatternArg {
            index: 1,
            kind: PatternType::Glob,
        }],
        "-- freezes glob's option prefix before an option-shaped pattern"
    );
}

#[test]
fn source_aware_pattern_layouts_respect_release_and_reserved_suffixes() {
    use crate::invocation_words::{InvocationArguments, InvocationWord};
    use crate::patterns::{PatternArg, PatternType};

    let strict_abbreviation = [
        InvocationWord::Literal("-sta"),
        InvocationWord::Literal("2"),
        InvocationWord::Literal("a+"),
        InvocationWord::Literal("aaa"),
    ];
    assert!(
        crate::model::ingress::static_context_for("tcl9.0")
            .commands()
            .pattern_args_words(
                "regexp",
                InvocationArguments::structured(&strict_abbreviation)
            )
            .is_empty(),
        "regexp's strict option table rejects a non-exact abbreviation"
    );
    let stride_abbreviation = [
        InvocationWord::Literal("-str"),
        InvocationWord::Literal("2"),
        InvocationWord::Literal("{a b}"),
        InvocationWord::Literal("a+"),
    ];
    assert!(
        crate::model::ingress::static_context_for("tcl8.6")
            .commands()
            .pattern_args_words(
                "lsearch",
                InvocationArguments::structured(&stride_abbreviation)
            )
            .is_empty(),
        "a Tcl 9-only option abbreviation is invalid before Tcl 9"
    );
    assert_eq!(
        crate::model::ingress::static_context_for("tcl9.0")
            .commands()
            .pattern_args_words(
                "lsearch",
                InvocationArguments::structured(&stride_abbreviation),
            ),
        vec![PatternArg {
            index: 3,
            kind: PatternType::Glob,
        }],
        "the same abbreviation follows Tcl 9's profile-filtered option table"
    );
    let option_shaped_suffix = [
        InvocationWord::Literal("-regexp"),
        InvocationWord::Literal("--"),
    ];
    assert_eq!(
        crate::model::ingress::static_context_for("tcl9.0")
            .commands()
            .pattern_args_words(
                "lsearch",
                InvocationArguments::structured(&option_shaped_suffix),
            ),
        vec![PatternArg {
            index: 1,
            kind: PatternType::Glob,
        }],
        "lsearch's reserved list/pattern suffix remains positional"
    );
}

const MUTATED_PATTERN_OPTIONS: &[crate::hover::OptionSpec] = &[crate::hover::OptionSpec {
    name: "-skip",
    value: crate::hover::OptionValue::value("count"),
    detail: "fixture value",
    surface: None,
    aliases: &[],
    lifecycle: crate::lifecycle::Lifecycle::UNSPECIFIED,
    min_abbrev: None,
    effect: None,
}];

fn mutated_pattern_literal_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let first = crate::spec::leading_option_word_count_with(
        MUTATED_PATTERN_OPTIONS,
        args,
        PrefixMatching::Strict,
    );
    [
        (first, ArgRole::Pattern),
        (first + 2, ArgRole::FormatString),
    ]
    .into_iter()
    .filter_map(|(index, role)| {
        (index < args.len())
            .then(|| u8::try_from(index).ok().map(|index| (index, role)))
            .flatten()
    })
    .collect()
}

fn mutated_pattern_layout_roles(
    args: crate::invocation_words::InvocationArguments<'_>,
    options: crate::resolved_invocation::InvocationOptions<'_, '_>,
) -> Option<Vec<(u8, ArgRole)>> {
    let first = options.leading_word_count(args)?;
    let count = args.exact_argv_len()?;
    Some(
        [
            (first, ArgRole::Pattern),
            (first + 2, ArgRole::FormatString),
        ]
        .into_iter()
        .filter_map(|(index, role)| {
            (index < count)
                .then(|| u8::try_from(index).ok().map(|index| (index, role)))
                .flatten()
        })
        .collect(),
    )
}

/// The source proof follows descriptors installed in the registry, not a
/// closed list of Tcl command names. A pack or a later registry mutation
/// can introduce the same option-dependent pattern/replacement grammar
/// and all generic consumers receive the new answer unchanged.
#[test]
fn source_aware_pattern_and_format_queries_follow_mutated_descriptors() {
    const SPEC: CommandSpec = CommandSpec {
        name: "mutated-pattern-format-owner",
        options: MUTATED_PATTERN_OPTIONS,
        prefix_matching: PrefixMatching::Strict,
        arg_role_layout_resolver: Some(mutated_pattern_layout_roles),
        pattern_type: Some(PatternType::Regex),
        format_string_type: Some(FormatType::Regsub),
        ..CommandSpec::DEFAULT
    };
    use crate::invocation_words::{InvocationArguments, InvocationWord};
    use crate::patterns::{FormatType, PatternArg, PatternType};

    assert_eq!(
        mutated_pattern_literal_roles(&["-skip", "1", "a", "s", "replacement"]),
        vec![(2, ArgRole::Pattern), (4, ArgRole::FormatString)]
    );

    let mut registry = CommandRegistry::build_default();
    registry.insert(SPEC);
    let dynamic = [
        InvocationWord::Dynamic,
        InvocationWord::Literal("{a}"),
        InvocationWord::Dynamic,
        InvocationWord::Literal(r"{\1}"),
    ];
    assert_eq!(
        registry
            .pattern_args_words(
                "mutated-pattern-format-owner",
                InvocationArguments::structured(&dynamic),
            )
            .len(),
        0
    );
    assert_eq!(
        registry
            .format_string_args_words(
                "mutated-pattern-format-owner",
                InvocationArguments::structured(&dynamic),
            )
            .len(),
        0
    );

    let fixed = [
        InvocationWord::Literal("-skip"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("{a}"),
        InvocationWord::Dynamic,
        InvocationWord::Literal(r"{\1}"),
    ];
    assert_eq!(
        registry.pattern_args_words(
            "mutated-pattern-format-owner",
            InvocationArguments::structured(&fixed),
        ),
        vec![PatternArg {
            index: 2,
            kind: PatternType::Regex,
        }]
    );
    assert_eq!(
        registry.format_string_args_words(
            "mutated-pattern-format-owner",
            InvocationArguments::structured(&fixed),
        ),
        vec![FormatStringArg {
            index: 4,
            kind: FormatType::Regsub,
            scan: false,
        }]
    );
}

/// Pattern locations come from the owning specs' option descriptors, not
/// from an LSP walk guessing where a `-start` value ends.  Exercise the
/// three shapes prone to diverging: abbreviations, value-taking
/// options, and glob's unbounded pattern tail.
#[test]
fn pattern_roles_follow_declared_option_prefixes() {
    let reg = CommandRegistry::build_default();
    assert_eq!(
        reg.arg_indices_for_role("lsearch", &["-sta", "2", "{a b c}", "b*"], ArgRole::Pattern,),
        vec![3],
    );
    assert_eq!(
        reg.arg_indices_for_role("string", &["match", "-noc", "a*", "abc"], ArgRole::Pattern,),
        vec![2],
    );
    assert_eq!(
        reg.arg_indices_for_role("glob", &["-di", "tmp", "*.tcl", "*.tm"], ArgRole::Pattern,),
        vec![2, 3],
    );
    assert_eq!(
        reg.arg_indices_for_role("glob", &["--", "-literal", "*.tcl"], ArgRole::Pattern),
        vec![1, 2],
    );
    assert_eq!(
        reg.arg_indices_for_role("regexp", &["-start", "2", "a+", "aaa"], ArgRole::Pattern,),
        vec![2],
    );
    assert_eq!(
        reg.pattern_args("lsearch", &["-regexp", "{a b}", "a+"]),
        vec![crate::patterns::PatternArg {
            index: 2,
            kind: crate::patterns::PatternType::Regex,
        }],
    );
    assert_eq!(
        reg.pattern_args("lsearch", &["-exact", "{a b}", "a+"])
            .len(),
        0
    );
}

#[test]
fn source_aware_lsearch_pattern_layout_respects_outer_option_boundary() {
    use crate::invocation_words::{InvocationArguments, InvocationWord};
    use crate::patterns::{PatternArg, PatternType};

    // Tcl's `Tcl_LsearchObjCmd` has used `for (i = 1; i < objc - 2;
    // i++)` since 8.4: it stops scanning exactly when list and pattern
    // remain. Keep the source-aware resolver aligned across every
    // supported core release, including the 9.0 -stride expansion.
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let reg = crate::model::ingress::static_context_for(dialect).commands();
        let proven_outer_option = [
            InvocationWord::Literal("-glob"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("ba*"),
        ];
        assert_eq!(
            reg.pattern_args_words(
                "lsearch",
                InvocationArguments::structured(&proven_outer_option),
            ),
            vec![PatternArg {
                index: 2,
                kind: PatternType::Glob,
            }],
            "{dialect}: lsearch -glob $list {{ba*}} has proven operands and glob mode"
        );

        let expanded_list = [
            InvocationWord::Literal("-glob"),
            InvocationWord::Expanded,
            InvocationWord::Literal("ba*"),
        ];
        assert!(
            reg.pattern_args_words("lsearch", InvocationArguments::structured(&expanded_list))
                .is_empty(),
            "{dialect}: lsearch -glob {{*}}$list {{ba*}} has no known argv suffix"
        );

        let ambiguous_option_prefix = [
            InvocationWord::Dynamic,
            InvocationWord::Literal("{a b}"),
            InvocationWord::Literal("a+"),
        ];
        // `$mode` remains inside the `i < objc - 2` option region and
        // may become a value-taking switch, so it cannot be reclassified
        // as the list operand to claim a glob pattern.
        assert!(
            reg.pattern_args_words(
                "lsearch",
                InvocationArguments::structured(&ambiguous_option_prefix),
            )
            .is_empty(),
            "{dialect}: lsearch $mode {{a b}} {{a+}} remains ambiguous"
        );
    }
}

#[test]
fn lsearch_pattern_resolver_uses_profiled_options_and_the_reserved_suffix() {
    use crate::patterns::{PatternArg, PatternType};

    // These are oracle-shaped against Tcl's `i < objc - 2` scanner.  The
    // first two words of the short form are list + pattern, even though
    // both spell like lsearch switches.
    let option_shaped_operands = ["-regexp", "-glob"];
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let registry = crate::model::ingress::static_context_for(dialect).commands();
        let patterns = registry.pattern_args("lsearch", &option_shaped_operands);
        assert_eq!(
            patterns,
            vec![PatternArg {
                index: 1,
                kind: PatternType::Glob,
            }],
            "{dialect}: lsearch -regexp -glob keeps both mandatory operands out of option scanning"
        );
        assert_eq!(
            registry.arg_indices_for_role("lsearch", &option_shaped_operands, ArgRole::Pattern),
            patterns
                .iter()
                .map(|pattern| usize::from(pattern.index))
                .collect::<Vec<_>>(),
            "{dialect}: ArgRole::Pattern must share the resolver answer"
        );
    }

    // `-str` can name only -stride in Tcl 9.  Pre-9 interpreters reject
    // it, so the resolver must abstain rather than treating an invalid
    // dash-prefixed word as the list operand and inventing a pattern.
    let stride_abbrev = ["-str", "2", "{a b}", "a+"];
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6"] {
        let registry = crate::model::ingress::static_context_for(dialect).commands();
        assert!(
            registry.pattern_args("lsearch", &stride_abbrev).is_empty(),
            "{dialect}: unavailable -stride abbreviation invalidates the invocation"
        );
        assert!(
            registry
                .arg_indices_for_role("lsearch", &stride_abbrev, ArgRole::Pattern)
                .is_empty(),
            "{dialect}: pattern roles abstain with the invalid invocation"
        );
    }
    for dialect in ["tcl9.0", "tcl9.1"] {
        let registry = crate::model::ingress::static_context_for(dialect).commands();
        let patterns = registry.pattern_args("lsearch", &stride_abbrev);
        assert_eq!(
            patterns,
            vec![PatternArg {
                index: 3,
                kind: PatternType::Glob,
            }],
            "{dialect}: -str is the available -stride abbreviation"
        );
        assert_eq!(
            registry.arg_indices_for_role("lsearch", &stride_abbrev, ArgRole::Pattern),
            vec![3],
            "{dialect}: roles, hover, and tokens share the Tcl 9 layout"
        );
    }

    // In Tcl 9 `-st` is ambiguous between -start and -stride. Unknown
    // words and unsupported `--` likewise cannot be reclassified as the
    // list operand while the mandatory suffix is still reserved.
    for args in [
        ["-st", "2", "{a b}", "a+"].as_slice(),
        ["-unknown", "{a b}", "a+"].as_slice(),
        ["--", "{a b}", "a+"].as_slice(),
    ] {
        let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
        assert!(
            registry.pattern_args("lsearch", args).is_empty(),
            "Tcl 9 invalid option prefix {args:?} must abstain"
        );
        assert!(
            registry
                .arg_indices_for_role("lsearch", args, ArgRole::Pattern)
                .is_empty(),
            "Tcl 9 invalid option prefix {args:?} must have no pattern role"
        );
    }

    // The final two words are always the mandatory list + pattern, even
    // when their spelling would otherwise be an invalid option.
    for args in [
        ["{a b}", "-unknown"].as_slice(),
        ["-regexp", "--"].as_slice(),
        ["-regexp", "-st"].as_slice(),
    ] {
        let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
        assert_eq!(
            registry.pattern_args("lsearch", args),
            vec![PatternArg {
                index: 1,
                kind: PatternType::Glob,
            }],
            "Tcl 9 final operands {args:?} stay positional"
        );
    }
}

#[test]
fn source_aware_lsearch_dynamic_prefix_and_suffix_have_distinct_obligations() {
    use crate::invocation_words::{InvocationArguments, InvocationWord};
    use crate::patterns::{PatternArg, PatternType};

    for dialect in ["tcl8.6", "tcl9.0"] {
        let registry = crate::model::ingress::static_context_for(dialect).commands();
        let dynamic_early = [
            InvocationWord::Dynamic,
            InvocationWord::Literal("{a b}"),
            InvocationWord::Literal("a+"),
        ];
        assert!(
            registry
                .pattern_args_words("lsearch", InvocationArguments::structured(&dynamic_early))
                .is_empty(),
            "{dialect}: a dynamic word before the mandatory operands can still be a switch"
        );

        let dynamic_list_operand = [
            InvocationWord::Literal("-regexp"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("a+"),
        ];
        assert_eq!(
            registry.pattern_args_words(
                "lsearch",
                InvocationArguments::structured(&dynamic_list_operand),
            ),
            vec![PatternArg {
                index: 2,
                kind: PatternType::Regex,
            }],
            "{dialect}: the dynamic mandatory list is beyond the option boundary"
        );
    }
}

/// `string is class ?-strict? ?-failindex varName? string` writes its index
/// variable (tclCmdMZ.c `StringIsCmd`, Tcl 8.5 to 9.1): the option's value is
/// a variable-write word under every release that has the option.
#[test]
fn string_is_failindex_names_a_variable_it_writes() {
    for environment in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let context = crate::model::ingress::resolve_environment(environment).default_context_registry();
        let reg = context.commands();
        for args in [
            ["is", "integer", "-failindex", "bad", "$s"].as_slice(),
            ["is", "integer", "-strict", "-failindex", "bad", "$s"].as_slice(),
        ] {
            let at = args.iter().position(|word| *word == "bad").unwrap();
            assert_eq!(
                reg.arg_indices_for_role("string", args, ArgRole::VarWrite),
                vec![at],
                "{environment}: {args:?}"
            );
        }
    }
}
