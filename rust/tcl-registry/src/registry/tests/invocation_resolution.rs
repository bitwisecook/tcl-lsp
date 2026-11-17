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

//! Resolving calls and invocations: form selection, subcommand outcomes, semantic operations and option terminators.

use super::*;

#[test]
fn form_selection_counts_available_options_on_the_declared_axis() {
    static OPTIONS: [crate::hover::OptionSpec; 2] = [
        crate::hover::OptionSpec {
            name: "-later",
            surface: Some(SpecSurface::TCL91),
            ..crate::hover::OptionSpec::DEFAULT
        },
        crate::hover::OptionSpec {
            name: "-package",
            lifecycle: crate::lifecycle::Lifecycle::introduced_in("2.0"),
            ..crate::hover::OptionSpec::DEFAULT
        },
    ];
    static FORMS: [crate::forms::CommandForm; 1] = [crate::forms::CommandForm {
        name: "positional",
        arity: crate::Arity::exact(1).with_positionals(),
        options: &OPTIONS,
        ..crate::forms::CommandForm::DEFAULT
    }];
    let mut registry = CommandRegistry::build_default();
    registry.insert(crate::CommandSpec {
        name: "counted",
        required_package: Some("Counted"),
        command_forms: &FORMS,
        ..crate::CommandSpec::DEFAULT
    });
    for (release, args, selected) in [
        ("9.0", &["-later", "x"][..], false),
        ("9.1", &["-later", "x"][..], true),
        ("9.1", &["-package", "x"][..], true),
    ] {
        let resolved = registry
            .resolve_invocation(
                "counted",
                args,
                Some(SurfaceQuery::core(Family::Tcl, release)),
            )
            .unwrap();
        assert_eq!(resolved.form.is_some(), selected, "{release} {args:?}");
    }
    registry.insert_ambient_package("Counted", "1.0");
    assert!(
        registry
            .resolve_invocation(
                "counted",
                &["-package", "x"],
                Some(SurfaceQuery::core(Family::Tcl, "9.1"))
            )
            .unwrap()
            .form
            .is_none()
    );
    registry.insert_ambient_package("Counted", "2.0");
    let resolved = registry
        .resolve_invocation(
            "counted",
            &["-package", "x"],
            Some(SurfaceQuery::core(Family::Tcl, "9.1")),
        )
        .unwrap();
    assert_eq!(resolved.argument_count_for_arity(), Some(1));
    assert_eq!(resolved.form.map(|form| form.name), Some("positional"));
}

#[test]
fn default_form_integer_uses_the_retained_invocation_grammar() {
    static SUB: [crate::SubCommand; 1] = [crate::SubCommand {
        name: "replace",
        ..crate::SubCommand::DEFAULT
    }];
    static FORMS: [crate::forms::CommandForm; 1] = [crate::forms::CommandForm {
        name: "getter",
        arity: crate::Arity::exact(1),
        ..crate::forms::CommandForm::DEFAULT
    }];
    let mut registry = CommandRegistry::build_default();
    registry.insert(crate::CommandSpec {
        name: "payload_fixture",
        default_form_first_word: Some(crate::spec::DefaultFormFirstWord::Integer),
        subcommands: &SUB,
        command_forms: &FORMS,
        ..crate::CommandSpec::DEFAULT
    });
    for (release, spelling, selected) in [
        ("8.4", "100", true),
        ("9.1", "0d100", true),
        ("8.4", "0d100", false),
    ] {
        let args = [spelling];
        let invocation = registry
            .resolve_invocation(
                "payload_fixture",
                &args,
                Some(SurfaceQuery::core(Family::Tcl, release)),
            )
            .unwrap();
        assert_eq!(
            invocation.form.map(|form| form.name),
            selected.then_some("getter")
        );
    }
    let args = [crate::InvocationWord::Dynamic];
    let unresolved = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(
                crate::InvocationWord::Literal("payload_fixture"),
                &args,
            ),
            Some(SurfaceQuery::core(Family::Tcl, "9.1")),
        )
        .resolved()
        .unwrap();
    assert!(unresolved.form.is_none());
    assert!(matches!(
        unresolved.subcommand,
        crate::SubcommandResolution::Indeterminate { .. }
    ));
}

#[test]
fn resolve_call_unknown_command_returns_none() {
    let reg = CommandRegistry::build_default();
    assert!(reg.resolve_call("no_such_cmd", &[], None).is_none());
}

#[test]
fn resolve_invocation_retains_words_and_canonicalises_the_registry_name() {
    let reg = CommandRegistry::build_default();
    let args = ["create", "key", "value"];
    let resolved = reg
        .resolve_invocation(
            "::dict",
            &args,
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .expect("global dict spelling resolves through the registry");

    assert_eq!(resolved.words.head_literal(), Some("::dict"));
    assert_eq!(
        resolved.words.arguments(),
        InvocationArguments::literals(&args)
    );
    assert_eq!(resolved.canonical_command, "dict");
    let sub = resolved
        .subcommand
        .resolved()
        .expect("dict create subcommand");
    assert_eq!(sub.spelling, "create");
    assert_eq!(sub.canonical_name, "create");
    assert_eq!(
        resolved.subcommand.kind(),
        Some(crate::SubcommandResolutionKind::Exact)
    );
    assert_eq!(resolved.semantics.argument_offset, 1);
}

#[test]
fn structured_resolution_keeps_dynamic_subcommands_indeterminate() {
    let reg = CommandRegistry::build_default();
    let arguments = [
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Literal("text"),
    ];
    let resolved = reg
        .resolve_structured_invocation(
            InvocationWords::structured(crate::InvocationWord::Literal("string"), &arguments),
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .resolved()
        .expect("a literal command head is registry-known");

    assert!(matches!(
        resolved.subcommand,
        SubcommandResolution::Indeterminate {
            word_kind: crate::InvocationWordKind::Dynamic
        }
    ));
    assert!(resolved.form.is_none());
    assert_eq!(
        resolved.semantics.operation,
        crate::SemanticOperationId::Invoke,
        "a dynamic subcommand cannot select specialised subcommand metadata"
    );
}

#[test]
fn expanded_argument_does_not_match_an_arity_form() {
    let reg = CommandRegistry::build_default();
    let arguments = [crate::InvocationWord::Expanded];
    let resolved = reg
        .resolve_structured_invocation(
            InvocationWords::structured(crate::InvocationWord::Literal("incr"), &arguments),
            None,
        )
        .resolved()
        .expect("a literal command head is registry-known");

    assert!(resolved.form.is_none());
    assert_eq!(
        resolved.semantics.operation,
        crate::SemanticOperationId::StructuredLowering(LoweringHookId::Incr),
        "the command-level common operation remains visible without claiming a form"
    );
}

#[test]
fn literal_adapter_matches_the_structured_literal_view() {
    let reg = CommandRegistry::build_default();
    let arguments = ["counter"];
    let adapter = reg
        .resolve_invocation("incr", &arguments, None)
        .expect("literal adapter resolves");
    let structured = reg
        .resolve_structured_invocation(InvocationWords::literals("incr", &arguments), None)
        .resolved()
        .expect("structured literal input resolves");

    assert_eq!(adapter.canonical_command, structured.canonical_command);
    assert_eq!(adapter.subcommand, structured.subcommand);
    assert_eq!(
        adapter.form.map(|form| form.name),
        structured.form.map(|form| form.name)
    );
    assert_eq!(adapter.semantics.operation, structured.semantics.operation);
}

#[test]
fn structured_resolution_reports_computed_and_unknown_command_heads() {
    let reg = CommandRegistry::build_default();
    let no_arguments: [crate::InvocationWord<'_>; 0] = [];

    let computed = reg.resolve_structured_invocation(
        InvocationWords::structured(crate::InvocationWord::Dynamic, &no_arguments),
        None,
    );
    assert_eq!(
        computed.unresolved(),
        Some(crate::InvocationResolutionUnresolved::ComputedHead {
            word_kind: crate::InvocationWordKind::Dynamic,
        })
    );

    let no_literal_arguments: [&str; 0] = [];
    let unknown = reg.resolve_structured_invocation(
        InvocationWords::literals("not-a-registry-command", &no_literal_arguments),
        None,
    );
    assert_eq!(
        unknown.unresolved(),
        Some(crate::InvocationResolutionUnresolved::UnknownLiteralHead {
            spelling: "not-a-registry-command",
        })
    );
}

#[test]
fn literal_form_selectors_choose_the_longest_static_match() {
    const FORMS: &[CommandForm] = &[
        CommandForm {
            name: "tag-one",
            arity: Arity::exact(1),
            literal_argument_prefix: Some(LiteralArgumentPrefix::unique(&["tag"])),
            ..CommandForm::DEFAULT
        },
        CommandForm {
            name: "tag-fallback",
            arity: Arity::exact(3),
            literal_argument_prefix: Some(LiteralArgumentPrefix::unique(&["tag"])),
            ..CommandForm::DEFAULT
        },
        CommandForm {
            name: "tag-bind",
            arity: Arity::exact(3),
            literal_argument_prefix: Some(LiteralArgumentPrefix::unique(&["tag", "bind"])),
            ..CommandForm::DEFAULT
        },
    ];
    const SPEC: CommandSpec = CommandSpec {
        name: "literal-form-fixture",
        arity: Arity::any(),
        command_forms: FORMS,
        ..CommandSpec::DEFAULT
    };

    let mut registry = CommandRegistry::build_default();
    registry.insert(SPEC);
    let form = |args: &[&str]| {
        registry
            .resolve_invocation("literal-form-fixture", args, None)
            .and_then(|resolved| resolved.form)
            .map(|form| form.name)
    };

    assert_eq!(form(&["tag"]), Some("tag-one"), "arity disambiguates");
    assert_eq!(
        form(&["tag", "bind", "script"]),
        Some("tag-bind"),
        "the longer selector overrides its completed prefix"
    );
    assert_eq!(
        form(&["ta", "bi", "script"]),
        Some("tag-bind"),
        "each selector word still accepts a unique abbreviation"
    );
    assert_eq!(
        form(&["tag", "other", "value"]),
        Some("tag-fallback"),
        "a static non-match proves that the shorter selector applies"
    );

    let dynamic = [
        InvocationWord::Literal("tag"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("script"),
    ];
    let resolved = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal("literal-form-fixture"), &dynamic),
            None,
        )
        .resolved()
        .expect("fixture command resolves");
    assert!(
        resolved.form.is_none(),
        "a dynamic word that could extend a completed selector must abstain"
    );
}

#[test]
fn unresolved_subcommands_never_fall_back_to_a_top_level_form() {
    const TOP_LEVEL_FORM: CommandForm = CommandForm {
        name: "top-level-form",
        arity: Arity::exact(1),
        ..CommandForm::DEFAULT
    };
    const SUBCOMMANDS: &[SubCommand] = &[
        SubCommand {
            name: "alpha",
            ..SubCommand::DEFAULT
        },
        SubCommand {
            name: "alpine",
            ..SubCommand::DEFAULT
        },
    ];
    const SPEC: CommandSpec = CommandSpec {
        name: "subcommand-form-fixture",
        arity: Arity::any(),
        subcommands: SUBCOMMANDS,
        command_forms: &[TOP_LEVEL_FORM],
        ..CommandSpec::DEFAULT
    };

    let mut reg = CommandRegistry::build_default();
    reg.insert(SPEC);
    let dynamic_arguments = [crate::InvocationWord::Dynamic];
    let dynamic = reg
        .resolve_structured_invocation(
            InvocationWords::structured(
                crate::InvocationWord::Literal("subcommand-form-fixture"),
                &dynamic_arguments,
            ),
            None,
        )
        .resolved()
        .expect("fixture command resolves");
    assert!(matches!(
        dynamic.subcommand,
        SubcommandResolution::Indeterminate { .. }
    ));
    assert!(dynamic.form.is_none());
    assert!(matches!(
        dynamic.facts().subcommand,
        crate::OwnedSubcommandResolution::Indeterminate {
            word_kind: crate::InvocationWordKind::Dynamic
        }
    ));

    let ambiguous = reg
        .resolve_invocation("subcommand-form-fixture", &["al"], None)
        .expect("fixture command resolves");
    assert!(matches!(
        ambiguous.subcommand,
        SubcommandResolution::Ambiguous { .. }
    ));
    assert!(ambiguous.form.is_none());
    assert!(matches!(
        ambiguous.facts().subcommand,
        crate::OwnedSubcommandResolution::Ambiguous { ref spelling } if spelling == "al"
    ));

    let unknown = reg
        .resolve_invocation("subcommand-form-fixture", &["missing"], None)
        .expect("fixture command resolves");
    assert!(matches!(
        unknown.subcommand,
        SubcommandResolution::Unknown { .. }
    ));
    assert!(unknown.form.is_none());
    assert!(matches!(
        unknown.facts().subcommand,
        crate::OwnedSubcommandResolution::Unknown { ref spelling } if spelling == "missing"
    ));
}

#[test]
fn resolve_invocation_applies_registry_unique_prefix_subcommand_resolution() {
    let reg = CommandRegistry::build_default();
    let args = ["le", "hello"];
    let resolved = reg
        .resolve_invocation(
            "string",
            &args,
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .expect("string is registry-known");

    let sub = resolved
        .subcommand
        .resolved()
        .expect("string le is a valid unique prefix");
    assert_eq!(sub.spelling, "le");
    assert_eq!(sub.canonical_name, "length");
    assert_eq!(
        resolved.subcommand.kind(),
        Some(crate::SubcommandResolutionKind::UniquePrefix)
    );
    assert_eq!(
        resolved.semantics.return_type,
        Some(crate::types::TclType::Int)
    );
}

#[test]
fn resolve_invocation_preserves_an_ambiguous_subcommand_outcome() {
    let reg = CommandRegistry::build_default();
    let args = ["t", "hello"];
    let resolved = reg
        .resolve_invocation(
            "string",
            &args,
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .expect("the command head remains known");

    assert!(matches!(
        resolved.subcommand,
        SubcommandResolution::Ambiguous { spelling: "t" }
    ));
    assert_eq!(
        resolved.subcommand.kind(),
        Some(crate::SubcommandResolutionKind::Ambiguous)
    );
    assert!(resolved.form.is_none());
}

#[test]
fn resolve_invocation_preserves_an_unknown_subcommand_outcome() {
    let reg = CommandRegistry::build_default();
    let args = ["does-not-exist"];
    let resolved = reg
        .resolve_invocation(
            "string",
            &args,
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .expect("the command head remains known");

    assert!(matches!(
        resolved.subcommand,
        SubcommandResolution::Unknown {
            spelling: "does-not-exist"
        }
    ));
    assert_eq!(
        resolved.subcommand.kind(),
        Some(crate::SubcommandResolutionKind::Unknown)
    );
}

#[test]
fn resolve_invocation_projects_forms_without_backend_hooks() {
    let reg = CommandRegistry::build_default();
    let args = ["counter"];
    let resolved = reg
        .resolve_invocation("incr", &args, None)
        .expect("incr is registry-known");

    let form = resolved.form.expect("implicit incr form");
    assert_eq!(form.name, "implicit");
    assert_eq!(form.arity, Arity::exact(1));
    assert_eq!(form.arg_roles, &[(0, ArgRole::VarWrite)]);
    assert_eq!(resolved.semantics.arg_roles, form.arg_roles);
    assert_eq!(
        resolved.semantics.return_type,
        Some(crate::types::TclType::Int)
    );
    assert_eq!(
        resolved.semantics.lowering_hook,
        Some(LoweringHookId::Incr),
        "the common lowering descriptor survives the target-neutral projection"
    );
    assert_eq!(
        resolved.semantics.operation,
        crate::SemanticOperationId::StructuredLowering(LoweringHookId::Incr)
    );
}

#[test]
fn resolve_invocation_identifies_channel_write_without_selecting_a_backend() {
    let reg = CommandRegistry::build_default();
    let args = ["hello"];
    let resolved = reg
        .resolve_invocation("puts", &args, Some(SurfaceQuery::core(Family::Tcl, "8.6")))
        .expect("puts resolves");

    assert_eq!(
        resolved.semantics.operation,
        crate::SemanticOperationId::Intrinsic(crate::IntrinsicId::ChannelWrite)
    );
    assert_eq!(
        reg.command_names_for_semantic_operation(crate::SemanticOperationId::Intrinsic(
            crate::IntrinsicId::ChannelWrite,
        ))
        .collect::<Vec<_>>(),
        vec!["puts"]
    );
}

#[test]
fn semantic_operation_resolution_prefers_form_then_subcommand_then_command() {
    const FORM: CommandForm = CommandForm {
        name: "form",
        arity: Arity::exact(1),
        semantic_operation: Some(crate::SemanticOperationId::StructuredLowering(
            LoweringHookId::Set,
        )),
        ..CommandForm::DEFAULT
    };
    const SUB: SubCommand = SubCommand {
        name: "sub",
        arity: Arity::at_least(0),
        subcommand_forms: &[FORM],
        semantic_operation: Some(crate::SemanticOperationId::StructuredLowering(
            LoweringHookId::Incr,
        )),
        ..SubCommand::DEFAULT
    };
    const SPEC: CommandSpec = CommandSpec {
        name: "semantic-operation-precedence",
        arity: Arity::at_least(0),
        subcommands: &[SUB],
        semantic_operation: Some(crate::SemanticOperationId::StructuredLowering(
            LoweringHookId::Return,
        )),
        ..CommandSpec::DEFAULT
    };

    let mut reg = CommandRegistry::build_default();
    reg.insert(SPEC);

    let command = reg
        .resolve_invocation("semantic-operation-precedence", &[], None)
        .expect("command form resolves");
    assert_eq!(
        command.semantics.operation,
        crate::SemanticOperationId::StructuredLowering(LoweringHookId::Return)
    );

    let sub = reg
        .resolve_invocation("semantic-operation-precedence", &["sub"], None)
        .expect("subcommand form resolves");
    assert_eq!(
        sub.semantics.operation,
        crate::SemanticOperationId::StructuredLowering(LoweringHookId::Incr)
    );

    let form = reg
        .resolve_invocation("semantic-operation-precedence", &["sub", "argument"], None)
        .expect("subcommand form resolves");
    assert_eq!(
        form.semantics.operation,
        crate::SemanticOperationId::StructuredLowering(LoweringHookId::Set)
    );
}

#[test]
fn resolve_invocation_exposes_subcommand_effects_through_one_semantic_view() {
    let mut reg = CommandRegistry::build_default();
    reg.load_irules();
    let args = ["insert", "x-demo", "value"];
    let resolved = reg
        .resolve_invocation(
            "HTTP::header",
            &args,
            Some(SurfaceQuery::any_release(Family::F5Irules)),
        )
        .expect("HTTP::header insert resolves in the iRules dialect");

    // No source evidence closes an iRules command (issue #1712).
    assert!(!resolved.facts().is_pure());
    assert_eq!(resolved.semantics.side_effects.lower_bound().len(), 1);
    let effect = resolved.semantics.side_effects.lower_bound()[0];
    assert_eq!(
        effect.target,
        crate::side_effects::SideEffectTarget::HttpHeader
    );
    assert!(effect.reads);
    assert!(effect.writes);
}

#[test]
fn resolved_call_and_resolved_invocation_share_selection() {
    let reg = CommandRegistry::build_default();
    let args = ["counter", "5"];
    let common = reg
        .resolve_invocation("incr", &args, None)
        .expect("incr resolves");
    let legacy = reg
        .resolve_call("incr", &args, None)
        .expect("legacy compatibility resolver remains available");

    assert_eq!(common.canonical_command, legacy.spec.name);
    assert_eq!(
        common.form.map(|form| form.name),
        legacy.form.map(|form| form.name)
    );
    assert_eq!(common.semantics.lowering_hook, legacy.lowering_hook);
    assert_eq!(common.semantics.arity, legacy.arity());
}

#[test]
fn resolve_call_top_level_command() {
    let reg = CommandRegistry::build_default();
    let resolved = reg.resolve_call("set", &["x", "1"], None).unwrap();
    assert_eq!(resolved.spec.name, "set");
    assert!(resolved.sub.is_none());
}

#[test]
fn resolve_call_subcommand() {
    let reg = CommandRegistry::build_default();
    let resolved = reg
        .resolve_call(
            "dict",
            &["create", "k", "v"],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .unwrap();
    assert_eq!(resolved.spec.name, "dict");
    let sub = resolved.sub.expect("dict create resolves to a subcommand");
    assert_eq!(sub.name, "create");
}

#[test]
fn resolve_call_dialect_filter_blocks_tcl84_dict() {
    let reg = CommandRegistry::build_default();
    // dict is tcl8.5+; resolving against tcl8.4 must fail.
    assert!(
        reg.resolve_call(
            "dict",
            &["create"],
            Some(SurfaceQuery::core(Family::Tcl, "8.4"))
        )
        .is_none()
    );
}

#[test]
fn resolve_call_picks_arity_matched_command_form_for_incr() {
    let reg = CommandRegistry::build_default();
    // `incr counter` — arity 1 → matches the implicit form.
    let r1 = reg.resolve_call("incr", &["counter"], None).unwrap();
    let f1 = r1.form.expect("incr should match a CommandForm");
    assert_eq!(f1.name, "implicit");
    assert_eq!(f1.arity, crate::arity::Arity::exact(1));

    // `incr counter 5` — arity 2 → matches the explicit form.
    let r2 = reg.resolve_call("incr", &["counter", "5"], None).unwrap();
    let f2 = r2.form.expect("incr counter 5 should match a CommandForm");
    assert_eq!(f2.name, "explicit");
    assert_eq!(f2.arity, crate::arity::Arity::exact(2));
}

#[test]
fn resolve_call_picks_arity_matched_command_form_for_lset() {
    let reg = CommandRegistry::build_default();
    // `lset lst value` — arity 2 → replace form.
    let replace = reg
        .resolve_call(
            "lset",
            &["lst", "value"],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .unwrap();
    assert_eq!(replace.form.unwrap().name, "replace");

    // `lset lst 0 value` — arity 3 → single_index form.
    let single = reg
        .resolve_call(
            "lset",
            &["lst", "0", "value"],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .unwrap();
    assert_eq!(single.form.unwrap().name, "single_index");

    // `lset lst 0 1 2 value` — arity 5 → flat_path form.
    let flat = reg
        .resolve_call(
            "lset",
            &["lst", "0", "1", "2", "value"],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .unwrap();
    assert_eq!(flat.form.unwrap().name, "flat_path");
}

// -- ``resolve_option_terminator`` (W304 driver)
//
// Exercises option-terminator resolution for each command.
// Each W304 fixture
// is rooted in one of these resolver outcomes; the resolver tests
// here pin the per-command shape, the analyser tests pin the
// tristate-severity / two-diagnostic / code-fix behaviour.

#[test]
fn resolve_option_terminator_returns_none_for_unknown_command() {
    let reg = CommandRegistry::build_default();
    assert!(
        reg.resolve_option_terminator("unknownthing", &[], None)
            .is_none()
    );
}

#[test]
fn resolve_option_terminator_returns_none_for_command_without_terminator() {
    let reg = CommandRegistry::build_default();
    // ``set`` does not declare a ``--`` terminator option.
    assert!(
        reg.resolve_option_terminator("set", &["x", "1"], None)
            .is_none()
    );
}

#[test]
fn resolve_option_terminator_form_level_for_regexp() {
    let reg = CommandRegistry::build_default();
    let profile = reg
        .resolve_option_terminator("regexp", &[], None)
        .expect("regexp declares -- at the form level");
    assert_eq!(profile.scan_start, 0);
    assert!(profile.subcommand.is_none());
    // ``-start`` takes a value; ``-nocase`` does not.
    // ``-start`` takes a value; ``-nocase`` does not.  The
    // resolver returns the borrowed options slice; callers
    // filter via ``OptionSpec::takes_value``.
    assert!(
        profile
            .options
            .iter()
            .any(|o| o.name == "-start" && o.takes_value())
    );
    assert!(
        profile
            .options
            .iter()
            .any(|o| o.name == "-nocase" && !o.takes_value())
    );
}

#[test]
fn resolve_option_terminator_subcommand_scoped_for_file_delete() {
    let reg = CommandRegistry::build_default();
    let profile = reg
        .resolve_option_terminator("file", &["delete", "$path"], None)
        .expect("file delete declares -- at the subcommand level");
    assert_eq!(profile.scan_start, 1);
    assert_eq!(profile.subcommand, Some("delete"));
}

#[test]
fn resolve_option_terminator_subcommand_without_terminator_returns_none() {
    let reg = CommandRegistry::build_default();
    // ``file mtime`` has no ``--`` terminator.
    let profile = reg.resolve_option_terminator("file", &["mtime", "$p"], None);
    assert!(profile.is_none(), "got {profile:?}");
}
