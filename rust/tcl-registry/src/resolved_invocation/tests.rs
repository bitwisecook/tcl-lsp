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
fn source_executable_value_absence_keeps_data_role_ambiguity_separate() {
    use super::SourceExecutableValueCapability::{Absent, MayContain, Unproven};
    use crate::InvocationWord::{Dynamic, Literal};
    let mut registry = crate::CommandRegistry::build_default();
    let single_payload = [Dynamic];
    let single = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(Literal("puts"), &single_payload),
            None,
        )
        .resolved()
        .unwrap();
    assert_eq!(single.authored_source_argument_roles(), (Vec::new(), true));
    assert_eq!(single.authored_source_executable_value_capability(), Absent);
    // With two words the first can be either the flag or a channel.
    let payload = [Dynamic, Dynamic];
    let selected = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(Literal("puts"), &payload),
            None,
        )
        .resolved()
        .unwrap();
    assert!(!selected.authored_source_argument_roles().1);
    assert_eq!(
        selected.authored_source_executable_value_capability(),
        Absent
    );
    // Public summaries do not author the private command capability.
    let mut reported = selected;
    reported.semantics.arg_roles = &[(0, crate::ArgRole::Body)];
    assert_eq!(
        reported.authored_source_executable_value_capability(),
        Absent
    );
    for (head, arguments) in [
        ("after", &["0", "set", "x", "1"][..]),
        ("apply", &["{} {set x 1}"][..]),
        ("lsort", &["-command", "compare", "a b"][..]),
        ("button", &[".b", "-command", "set x 1"][..]),
    ] {
        let selected = registry.resolve_invocation(head, arguments, None).unwrap();
        assert_eq!(
            selected.authored_source_executable_value_capability(),
            MayContain,
            "{head}"
        );
    }
    let unknown = [Dynamic, Literal("x")];
    let selected = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(Literal("namespace"), &unknown),
            None,
        )
        .resolved()
        .unwrap();
    assert_eq!(
        selected.authored_source_executable_value_capability(),
        Unproven
    );
    registry.insert(crate::CommandSpec {
        name: "unclassified_layout",
        arg_role_resolver: Some(|_| vec![(0, crate::ArgRole::Body)]),
        ..crate::CommandSpec::DEFAULT
    });
    let selected = registry
        .resolve_invocation("unclassified_layout", &["value"], None)
        .unwrap();
    assert_eq!(
        selected.authored_source_executable_value_capability(),
        Unproven
    );
}

#[test]
fn source_executable_value_capability_keeps_actual_private_instance_and_nested_options() {
    use super::SourceExecutableValueCapability::MayContain;
    const OPTIONS: &[OptionSpec] = &[OptionSpec {
        name: "-script",
        value: crate::hover::OptionValue::script(),
        ..OptionSpec::DEFAULT
    }];
    let registry = crate::CommandRegistry::build_default();
    let selected = registry
        .resolve_invocation("puts", &["data"], None)
        .unwrap();
    let member = crate::SubCommand {
        name: "run",
        arg_roles: &[(0, crate::ArgRole::Body)],
        ..crate::SubCommand::DEFAULT
    };
    let mut instance = selected;
    instance.selected.instance = true;
    instance.selected.sub = Some(&member);
    assert_eq!(
        instance.authored_source_executable_value_capability(),
        MayContain
    );
    let nested = crate::SubCommand {
        name: "nested",
        sub_subcommands: &[crate::spec::SubSubCommand {
            name: "register",
            options: Some(OPTIONS),
            ..crate::spec::SubSubCommand::DEFAULT
        }],
        ..crate::SubCommand::DEFAULT
    };
    instance.selected.sub = Some(&nested);
    assert_eq!(
        instance.authored_source_executable_value_capability(),
        MayContain
    );
    let form = crate::forms::CommandForm {
        arg_roles: &[(0, crate::ArgRole::LambdaLiteral)],
        ..crate::forms::CommandForm::DEFAULT
    };
    instance.selected.sub = None;
    instance.selected.form = Some(&form);
    assert_eq!(
        instance.authored_source_executable_value_capability(),
        MayContain
    );
    let timing = crate::SubCommand {
        name: "timing",
        script_timing_resolver: Some(|_| Vec::new()),
        ..crate::SubCommand::DEFAULT
    };
    instance.selected.form = None;
    instance.selected.sub = Some(&timing);
    assert_eq!(
        instance.authored_source_executable_value_capability(),
        MayContain
    );
    let callback = crate::SubCommand {
        name: "callback",
        callback_taint_inputs: &[(0, &[])],
        ..crate::SubCommand::DEFAULT
    };
    instance.selected.sub = Some(&callback);
    assert_eq!(
        instance.authored_source_executable_value_capability(),
        MayContain
    );
}

#[test]
fn frozen_source_retention_separates_ordinary_values_and_deferred_hooks() {
    use super::SourceCallbackRetention::{Absent, MayRetain};
    let registry = crate::CommandRegistry::build_default();
    let cases: &[(&str, &[&str], super::SourceCallbackRetention)] = &[
        ("puts", &["hi"], Absent),
        ("set", &["x", "1"], Absent),
        ("proc", &["p", "", "puts hi"], Absent),
        ("after", &["0", "puts hi"], MayRetain),
        // An empty timer script still registers; concatenated scripts
        // deliberately have no positive single-script source ordinal.
        ("after", &["0", ""], MayRetain),
        ("after", &["0", "set", "::mark", "1"], MayRetain),
        ("after", &["idle", "set", "::mark", "1"], MayRetain),
        ("fileevent", &["stdout", "writable", "puts hi"], MayRetain),
        (
            "chan",
            &["event", "stdout", "writable", "puts hi"],
            MayRetain,
        ),
        // Until selected registration metadata distinguishes these forms,
        // getter/removal calls keep the conservative deferred obligation.
        ("after", &["0"], MayRetain),
        ("after", &["info"], MayRetain),
        ("after", &["cancel", "unknown-id"], MayRetain),
        ("fileevent", &["stdout", "writable"], MayRetain),
        ("fileevent", &["stdout", "writable", ""], MayRetain),
        ("chan", &["event", "stdout", "writable"], MayRetain),
        ("chan", &["event", "stdout", "writable", ""], MayRetain),
    ];
    for &(head, arguments, expected) in cases {
        let selected = registry.resolve_invocation(head, arguments, None).unwrap();
        for facts in [selected.facts(), selected.facts_after_success()] {
            assert_eq!(
                facts.source_callback_retention(),
                expected,
                "{head} {arguments:?}"
            );
        }
    }
}

#[test]
fn frozen_source_retention_preserves_unknown_selection_and_prefix_layout() {
    use super::SourceCallbackRetention::{MayRetain, Unproven};
    use crate::InvocationWord::{Dynamic, Expanded, Literal};
    let registry = crate::CommandRegistry::build_default();
    let cases: &[(
        &str,
        &[crate::InvocationWord<'_>],
        super::SourceCallbackRetention,
    )] = &[
        ("namespace", &[Dynamic, Literal("value")], Unproven),
        ("after", &[Dynamic, Literal("puts hi")], Unproven),
        ("lsort", &[Literal("-command"), Dynamic, Expanded], Unproven),
        (
            "fileevent",
            &[Literal("stdout"), Literal("writable"), Dynamic],
            MayRetain,
        ),
    ];
    for &(head, arguments, expected) in cases {
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal(head), arguments),
                None,
            )
            .resolved()
            .unwrap();
        for facts in [selected.facts(), selected.facts_after_success()] {
            assert_eq!(
                facts.source_callback_retention(),
                expected,
                "{head} {arguments:?}"
            );
        }
    }
}

#[test]
fn frozen_source_retention_does_not_drop_body_valued_callback_options() {
    use super::SourceCallbackRetention::Unproven;
    let registry = crate::CommandRegistry::build_default();
    let selected = registry
        .resolve_invocation("button", &[".b", "-command", "puts hi"], None)
        .expect("real Tk deferred-script option descriptor");
    assert!(selected.semantics.options.available().any(
        |option| option.name == "-command" && option.value_role() == Some(crate::ArgRole::Body)
    ));
    for facts in [selected.facts(), selected.facts_after_success()] {
        assert_eq!(facts.source_callback_retention(), Unproven);
    }
}

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
fn selected_source_control_arms_require_actual_roles_and_complete_grammar() {
    use crate::InvocationWord::{Dynamic, Expanded, Literal};
    use crate::registry::ControlArmSemantics as Arm;
    let registry = crate::CommandRegistry::build_default();
    let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    for (command, arguments, ordinal, expected) in [
        (
            "if",
            vec![Dynamic, Literal("set x 1")],
            1,
            Some(Arm::Selected),
        ),
        ("if", vec![Literal("1")], 0, None),
        (
            "if",
            vec![Literal("1"), Literal("body"), Literal("garbage")],
            1,
            Some(Arm::Selected),
        ),
        (
            "if",
            vec![Literal("1"), Literal("body"), Literal("garbage")],
            2,
            Some(Arm::Selected),
        ),
        (
            "if",
            vec![
                Literal("1"),
                Literal("body"),
                Literal("else"),
                Literal("fallback"),
                Literal("extra"),
            ],
            1,
            None,
        ),
        ("if", vec![Expanded, Literal("body")], 1, None),
        (
            "catch",
            vec![Literal("body")],
            0,
            Some(Arm::CompletionBoundary),
        ),
        (
            "while",
            vec![Dynamic, Literal("body")],
            1,
            Some(Arm::Uncertain),
        ),
        (
            "foreach",
            vec![Literal("x"), Dynamic, Literal("body")],
            2,
            Some(Arm::BoundedIteration),
        ),
        (
            "apply",
            vec![Literal("{} {body}")],
            0,
            Some(Arm::FrameBoundary),
        ),
        ("list", vec![Literal("body")], 0, None),
        (
            "try",
            vec![Literal("body"), Literal("finally"), Literal("cleanup")],
            0,
            Some(Arm::Always),
        ),
        (
            "try",
            vec![Literal("body"), Literal("finally"), Literal("cleanup")],
            2,
            Some(Arm::Always),
        ),
    ] {
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal(command), &arguments)
                    .with_dialect(dialect),
                dialect.authoring_query(),
            )
            .resolved()
            .unwrap();
        assert_eq!(
            selected.authored_source_control_arm_semantics(ordinal),
            expected,
            "{command} {arguments:?}"
        );
        assert_eq!(
            selected.authored_logical_source_control_arm_semantics(ordinal),
            expected,
            "Logical {command} {arguments:?}"
        );
        assert!(
            selected
                .authored_source_control_arm_semantics(arguments.len())
                .is_none()
        );
    }
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

use super::*;
use crate::InvocationArguments;
use crate::world_effect::{
    CallbackKinds, EffectAccess, EffectAccessMode, EffectFootprint, StaticEffectAccess,
    StaticEffectFootprint, StaticInterpreterScope, StaticNamespaceScope, StaticSubjectScope,
    SubjectScope, WorldEffectComposition, WorldEffectDescriptor, WorldStateDomain,
};
use crate::{
    AbruptTransitionTransfer, CallerFrameSelection, CommandBindingDefinitionKind,
    CommandBindingTransition, CommandRegistry, CompletionCode, CompletionCodeDomain,
    CompletionDescriptor, DispatchDependencies, DispatchDependencyDescriptor,
    DispatchDependencyDomain, NamespaceTransition, NamespaceTransitionTarget, Reentrancy,
    StateTransition, StateTransitionCommit, StateTransitionDescriptor, StateTransitionDomain,
    StateTransitionKnowledge, TransitionSubject, VariableAliasTarget,
};
use tcl_dialect::model::{Family, SpecSurface, SurfaceQuery};

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
fn original_selected_completion_keeps_descriptor_and_argument_grammar_independent() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // A source completion shape is not a reached handler or Normal receipt.
    use crate::registry::InvocationCompletion;
    let context = crate::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = crate::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let dialect = crate::InvocationDialect::of_profile(profile);
    let mut registry = CommandRegistry::build_default();
    registry.insert(crate::CommandSpec {
        name: "metadata_return_fixture",
        ..registry.get("return").unwrap().clone()
    });
    for (values, expected) in [
        (&["VALUE"][..], InvocationCompletion::ReturnsResult(Some(0))),
        (
            &["-level", "0", "VALUE"][..],
            InvocationCompletion::FallsThrough,
        ),
        (
            &["-level", "2", "VALUE"][..],
            InvocationCompletion::Terminates,
        ),
        (
            &["-code", "error", "VALUE"][..],
            InvocationCompletion::Terminates,
        ),
    ] {
        let words = crate::InvocationWords::from_arguments(
            crate::InvocationWord::Literal("metadata_return_fixture"),
            InvocationArguments::literals(values).with_dialect(dialect),
        );
        let selected = registry
            .resolve_structured_invocation(words, Some(context.context().authoring_query()))
            .resolved()
            .unwrap();
        assert_eq!(selected.authored_source_invocation_completion(), expected);
    }
    let unknown = [
        crate::InvocationWord::Literal("-level"),
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Literal("VALUE"),
    ];
    let selected = registry
        .resolve_structured_invocation(
            crate::InvocationWords::from_arguments(
                crate::InvocationWord::Literal("metadata_return_fixture"),
                InvocationArguments::structured(&unknown).with_dialect(dialect),
            ),
            Some(context.context().authoring_query()),
        )
        .resolved()
        .unwrap();
    assert_eq!(
        selected.authored_source_invocation_completion(),
        InvocationCompletion::Unknown
    );
    let without_grammar = registry
        .resolve_invocation(
            "metadata_return_fixture",
            &["VALUE"],
            Some(context.context().authoring_query()),
        )
        .unwrap();
    assert_eq!(
        without_grammar.authored_source_invocation_completion(),
        InvocationCompletion::Unknown,
        "availability cannot supply the missing original argument dialect"
    );
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
fn selected_option_variable_scope_shares_availability_prefix_and_value_geometry() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    static OPTIONS: &[crate::hover::OptionSpec] = &[
        crate::hover::OptionSpec {
            name: "-global",
            value: crate::hover::OptionValue::global_var_name(),
            surface: Some(tcl_dialect::model::SpecSurface::TCL86_PLUS),
            ..crate::hover::OptionSpec::DEFAULT
        },
        crate::hover::OptionSpec {
            name: "-local",
            value: crate::hover::OptionValue::var_name(),
            ..crate::hover::OptionSpec::DEFAULT
        },
    ];
    let mut registry = CommandRegistry::build_default();
    registry.insert(crate::CommandSpec {
        name: "scope-owner",
        options: OPTIONS,
        arity: crate::Arity::new(0, 4),
        ..crate::CommandSpec::DEFAULT
    });
    let query = Some(SurfaceQuery::core(Family::Tcl, "8.6"));
    let values = [
        crate::InvocationWord::Literal("-g"),
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Literal("-local"),
        crate::InvocationWord::Literal("named"),
    ];
    let selected = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(
                crate::InvocationWord::Literal("scope-owner"),
                &values,
            ),
            query,
        )
        .resolved()
        .unwrap();
    assert_eq!(
        selected.authored_source_option_variable_scope_at(1),
        Some(crate::VariableScope::Global)
    );
    assert_eq!(
        selected.authored_source_option_variable_scope_at(3),
        Some(crate::VariableScope::CurrentFrame)
    );
    assert!(
        selected
            .authored_source_option_variable_scope_at(0)
            .is_none()
    );
    let old = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(
                crate::InvocationWord::Literal("scope-owner"),
                &values,
            ),
            Some(SurfaceQuery::core(Family::Tcl, "8.4")),
        )
        .resolved()
        .unwrap();
    assert!(old.authored_source_option_variable_scope_at(1).is_none());
    let dynamic = [
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Literal("named"),
    ];
    let unknown = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(
                crate::InvocationWord::Literal("scope-owner"),
                &dynamic,
            ),
            query,
        )
        .resolved()
        .unwrap();
    assert!(
        unknown
            .authored_source_option_variable_scope_at(1)
            .is_none()
    );
    let terminated = registry
        .resolve_invocation("scope-owner", &["--", "-global", "named"], query)
        .unwrap();
    assert!(
        terminated
            .authored_source_option_variable_scope_at(2)
            .is_none()
    );
    assert_eq!(
        registry.option_variable_scope("scope-owner", &["-g", "named"], 1, query),
        Some(crate::VariableScope::Global)
    );
}

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
fn original_logical_case_role_projection_keeps_selected_options_and_body_residuals() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    use crate::InvocationWord::{Dynamic, Expanded, KnownBytes, Literal};
    let current = crate::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let older = crate::model::ingress::resolve_environment("tcl8.4")
        .default_context_registry()
        .with_command_store(std::sync::Arc::clone(current.commands()));
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
            Some((true, true, 2)),
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
    for (context, available) in [(current.as_ref(), true), (&older, false)] {
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
fn original_frame_script_advice_keeps_selected_and_logical_layouts_separate() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    use crate::InvocationWord::{Dynamic, Expanded, Literal};
    let registry = CommandRegistry::build_default();
    let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    for (arguments, expected) in [
        (
            vec![Literal("1"), Dynamic],
            Some(std::iter::once(1..2).collect()),
        ),
        (vec![Dynamic, Dynamic], Some(vec![0..2, 1..2])),
        (vec![Literal("1")], Some(Vec::new())),
        (vec![Literal("1"), Expanded], None),
    ] {
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal("uplevel"), &arguments)
                    .with_dialect(dialect),
                dialect.authoring_query(),
            )
            .resolved()
            .unwrap();
        assert_eq!(selected.authored_source_frame_script_arguments(), expected);
        assert_eq!(
            selected.authored_logical_source_frame_script_arguments(),
            expected
        );
        if arguments == [Dynamic, Dynamic] {
            assert!(
                selected.frame_effect().is_none(),
                "possible source tails do not close the ordinary frame layout"
            );
        }
    }
    let arguments = [Literal("1"), Dynamic];
    let selected = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(Literal("uplevel"), &arguments),
            dialect.authoring_query(),
        )
        .resolved()
        .unwrap();
    assert_eq!(
        selected.authored_logical_source_frame_script_arguments(),
        Some(std::iter::once(1..2).collect())
    );
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
fn source_enum_values_and_definition_bodies_use_the_selected_invocation() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let registry = CommandRegistry::build_default();
    let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_1);
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
        super::AuthoredSourceOptionBoundary::Dynamic(1)
    );
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

#[test]
fn selected_subcommand_options_preserve_reserved_positional_values() {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let registry = crate::CommandRegistry::build_default();
    let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
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
                .with_dialect(crate::InvocationDialect::for_version(version)),
                crate::InvocationDialect::for_version(version).authoring_query(),
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
                    .with_dialect(crate::InvocationDialect::for_version(version)),
                    crate::InvocationDialect::for_version(version).authoring_query(),
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
                .with_dialect(crate::InvocationDialect::for_version(version)),
                crate::InvocationDialect::for_version(version).authoring_query(),
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
        let dialect = crate::InvocationDialect::for_version(version);
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

#[test]
fn selected_operand_coercions_preserve_shared_object_hazards() {
    use crate::InvocationWord::{Dynamic, Expanded, Literal};
    use crate::representation::RepresentationCoercionSelection as Coercions;
    let context = crate::model::ingress::static_context_for("tcl9.0");
    let registry = context.commands();
    let dialect = crate::InvocationDialect::of_profile(registry.profile().unwrap());
    let cases: &[(&str, &[crate::InvocationWord<'_>], Coercions)] = &[
        (
            "llength",
            &[Dynamic],
            Coercions::Operands { arguments: vec![0] },
        ),
        ("llength", &[Expanded], Coercions::Unknown),
        ("list", &[Dynamic], Coercions::None),
        (
            "string",
            &[Literal("length"), Dynamic],
            Coercions::Operands { arguments: vec![1] },
        ),
        (
            "expr",
            &[Literal("2 + 2")],
            Coercions::Expression {
                source_arguments: 0..1,
            },
        ),
    ];
    for (head, arguments, expected) in cases {
        let words =
            crate::InvocationWords::structured(Literal(head), arguments).with_dialect(dialect);
        let invocation = registry
            .resolve_structured_invocation(words, None)
            .resolved()
            .unwrap();
        assert_eq!(
            invocation.operand_representation_coercions(),
            *expected,
            "{head}"
        );
        assert_eq!(
            invocation.facts().operand_representation_coercions,
            *expected,
            "{head}"
        );
    }
}

#[test]
fn selected_regexp_result_uses_available_flags_before_unknown_values() {
    use crate::InvocationWord::{Dynamic, Literal};
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let profile = crate::model::ingress::static_context_for(environment)
            .commands()
            .profile()
            .unwrap();
        // The raw catalogue's latest row is Jim; actual operand ingress
        // must select the C row independently of that insertion order.
        let registry = crate::CommandRegistry::build_default();
        let dialect = crate::InvocationDialect::of_profile(profile);
        for (words, expected) in [
            (
                vec![Literal("-about"), Dynamic],
                (environment != "jim").then_some(crate::TclType::List),
            ),
            (
                vec![Literal("--"), Literal("-about"), Dynamic],
                Some(crate::TclType::Int),
            ),
            (vec![Dynamic, Dynamic], None),
            (vec![Literal("-inline"), Dynamic, Dynamic], None),
            (
                vec![Literal("-start"), Literal("-about"), Dynamic, Dynamic],
                None,
            ),
            (
                vec![Literal("-start"), Literal("--"), Dynamic, Dynamic],
                None,
            ),
        ] {
            let facts = registry
                .resolve_structured_invocation(
                    crate::InvocationWords::structured(Literal("regexp"), &words)
                        .with_dialect(dialect),
                    None,
                )
                .resolved()
                .unwrap()
                .facts();
            assert_eq!(facts.return_type, expected, "{environment}: {words:?}");
        }
    }
}

#[test]
fn source_case_clause_projection_keeps_counted_bytes_and_unknown_option_selection() {
    use crate::InvocationWord::{Dynamic, Expanded, KnownBytes, Literal};
    for environment in ["tcl8.4", "tcl8.6", "tcl9.1", "jim", "f5-irules"] {
        let context =
            crate::model::ingress::resolve_environment(environment).default_context_registry();
        let dialect = crate::InvocationDialect::of_profile(context.commands().profile().unwrap());
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
        let dialect = crate::InvocationDialect::of_profile(context.commands().profile().unwrap());
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
fn source_arity_count_layout_keeps_axis_prefix_data_and_expansion_bounds() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    use crate::InvocationWord::{Dynamic, Expanded, Literal};
    let owner = crate::model::ingress::static_context_for("tcl8.6");
    for (head, arguments, minimum, indeterminate, operands) in [
        (
            "set",
            vec![Literal("name"), Dynamic, Expanded, Literal("extra")],
            3,
            true,
            vec![0, 1, 3],
        ),
        ("string", vec![Literal("le"), Dynamic], 1, false, vec![1]),
        ("fconfigure", vec![], 0, false, vec![]),
        ("fconfigure", vec![Dynamic], 1, false, vec![0]),
    ] {
        let resolution = owner.commands().resolve_structured_invocation(
            crate::InvocationWords::structured(Literal(head), &arguments),
            Some(owner.context().authoring_query()),
        );
        let selected = resolution.resolved().unwrap();
        let count = selected.authored_source_arity().unwrap().count;
        assert_eq!(
            (count.minimum, count.indeterminate, count.operands),
            (minimum, indeterminate, operands),
            "{head}"
        );
    }
    let arguments = [Dynamic, Literal("-buffering"), Literal("none")];
    let resolution = owner.commands().resolve_structured_invocation(
        crate::InvocationWords::structured(Literal("fconfigure"), &arguments),
        Some(owner.context().authoring_query()),
    );
    let selected = resolution.resolved().unwrap();
    let all = selected
        .authored_source_count_for_arity(crate::Arity::any())
        .unwrap();
    assert_eq!(all.operands, vec![0, 1, 2]);
    let positional = selected
        .authored_source_count_for_arity(crate::Arity::any().with_positionals())
        .unwrap();
    assert_eq!(positional.operands, vec![0]);
    let arguments = [Expanded, Literal("pattern"), Dynamic];
    let resolution = owner.commands().resolve_structured_invocation(
        crate::InvocationWords::structured(Literal("regexp"), &arguments),
        Some(owner.context().authoring_query()),
    );
    assert!(
        resolution
            .resolved()
            .unwrap()
            .authored_source_count_for_arity(crate::Arity::any().with_positionals())
            .is_none()
    );
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

#[test]
fn selected_argument_hints_retain_member_options_and_transparency() {
    use crate::InvocationWord::{Dynamic, Literal};
    let context = crate::model::ingress::static_context_for("tcl9.0");
    let registry = context.commands().clone();
    let dialect = crate::InvocationDialect::of_profile(registry.profile().unwrap());
    let arguments = [Literal("map"), Literal("-nocase"), Literal("a b"), Dynamic];
    let facts = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(Literal("string"), &arguments).with_dialect(dialect),
            None,
        )
        .resolved()
        .unwrap()
        .facts();
    assert!(facts.argument_type_hint(0).is_none());
    assert!(facts.argument_type_hint(1).is_none());
    let mapping = facts.argument_type_hint(2).unwrap();
    assert_eq!(mapping.expected, Some(crate::TclType::List));
    assert!(mapping.shimmers);
    assert_eq!(mapping.transparent_from, &[crate::TclType::Dict]);
    assert_eq!(
        facts.argument_type_hint(3).unwrap().expected,
        Some(crate::TclType::String)
    );
    assert!(facts.argument_type_hint(4).is_none());
    assert_eq!(
        facts.operand_representation_coercions,
        crate::representation::RepresentationCoercionSelection::Operands {
            arguments: vec![2, 3],
        }
    );
    let length_arguments = [Literal("length"), Dynamic];
    let length = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(Literal("string"), &length_arguments)
                .with_dialect(dialect),
            None,
        )
        .resolved()
        .unwrap()
        .facts();
    let retained = length.clone();
    drop(length);
    drop(registry);
    assert_eq!(
        retained.argument_type_hint(1).unwrap().transparent_from,
        &[crate::TclType::ByteArray]
    );
}

#[test]
fn selected_argument_hints_decline_unresolved_argv_and_rejected_arity() {
    use crate::InvocationWord::{Dynamic, Expanded, Literal};
    let context = crate::model::ingress::static_context_for("tcl9.0");
    let registry = context.commands();
    let dialect = crate::InvocationDialect::of_profile(registry.profile().unwrap());
    let cases: &[&[crate::InvocationWord<'_>]] = &[
        &[Dynamic, Literal("x")],
        &[Literal("map"), Dynamic, Literal("x")],
        &[Literal("length"), Expanded],
        &[Literal("length")],
        &[Literal("length"), Literal("x"), Literal("extra")],
    ];
    for arguments in cases {
        let facts = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal("string"), arguments)
                    .with_dialect(dialect),
                None,
            )
            .resolved()
            .unwrap()
            .facts();
        for index in 0..4 {
            assert!(
                facts.argument_type_hint(index).is_none(),
                "{arguments:?}: slot {index}"
            );
        }
    }
}

#[test]
fn positional_arity_uses_proved_option_boundary_without_inventing_dynamic_values() {
    use crate::InvocationWord::{Dynamic, DynamicNonOption, Expanded, Literal};
    let registry = CommandRegistry::build_default();
    let cases: &[(&[crate::InvocationWord<'_>], Option<u16>)] = &[
        (
            &[Literal("create"), Literal("-safe"), Literal("jail")],
            Some(1),
        ),
        (
            &[Literal("create"), Literal("-safe"), Literal("-safe")],
            Some(0),
        ),
        (
            &[Literal("create"), Literal("--"), Literal("-safe")],
            Some(1),
        ),
        (&[Literal("create"), Literal("--"), Dynamic], Some(1)),
        (
            &[Literal("create"), Literal("-safe"), DynamicNonOption],
            Some(1),
        ),
        (&[Literal("create"), Dynamic], None),
        (&[Literal("create"), Literal("-safe"), Dynamic], None),
        (&[Literal("create"), Literal("-unknown")], None),
        (&[Literal("create"), Expanded], None),
        (
            &[Literal("create"), Literal("jail"), Literal("extra")],
            Some(2),
        ),
    ];
    for &(arguments, expected) in cases {
        let invocation = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal("interp"), arguments),
                None,
            )
            .resolved()
            .expect("literal interp selector resolves");
        assert_eq!(
            invocation.argument_count_for_arity(),
            expected,
            "{arguments:?}"
        );
        assert_eq!(
            invocation.facts().arity_accepts_frozen_arguments(),
            expected.map(|count| count <= 1),
            "{arguments:?}"
        );
    }
    let arguments = [Literal("-nonewline"), Dynamic];
    let invocation = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(Literal("read"), &arguments),
            None,
        )
        .resolved()
        .expect("literal read resolves");
    assert_eq!(
        invocation.argument_count_for_arity(),
        Some(2),
        "raw argument signatures retain their authored count axis"
    );
}

#[test]
fn leading_flags_are_not_positionals_and_the_final_word_never_is_one() {
    use crate::InvocationWord::Literal;
    let registry = CommandRegistry::build_default();
    // tclsh 8.4-9.0: a leading flag is not a positional, and the final
    // word never is a flag (`puts -nonewline` prints `-nonewline`).
    for (head, arguments, expected) in [
        (
            "puts",
            vec![Literal("-nonewline"), Literal("stdout"), Literal("x")],
            2,
        ),
        ("puts", vec![Literal("-nonewline")], 1),
        (
            "file",
            vec![
                Literal("link"),
                Literal("-symbolic"),
                Literal("a"),
                Literal("b"),
            ],
            2,
        ),
        ("file", vec![Literal("link"), Literal("-hard")], 1),
        (
            "tcl::prefix",
            vec![
                Literal("match"),
                Literal("-exact"),
                Literal("t"),
                Literal("s"),
            ],
            2,
        ),
        (
            "interp",
            vec![
                Literal("cancel"),
                Literal("-unwind"),
                Literal("--"),
                Literal("x"),
            ],
            1,
        ),
    ] {
        let invocation = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal(head), &arguments),
                None,
            )
            .resolved()
            .expect("literal command resolves");
        assert_eq!(
            invocation.argument_count_for_arity(),
            Some(expected),
            "{head} {arguments:?}"
        );
    }
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

const COMMAND_CODES: &[CompletionCode] = &[CompletionCode::Error];
const SUBCOMMAND_CODES: &[CompletionCode] = &[CompletionCode::Break];
const FORM_CODES: &[CompletionCode] = &[CompletionCode::Other(71)];

const COMMAND_WORLD_ACCESSES: &[StaticEffectAccess] = &[StaticEffectAccess::new(
    WorldStateDomain::PackageState,
    EffectAccessMode::Read,
    StaticInterpreterScope::Current,
    StaticNamespaceScope::Current,
    StaticSubjectScope::Wildcard,
)];
const SUBCOMMAND_WORLD_ACCESSES: &[StaticEffectAccess] = &[StaticEffectAccess::new(
    WorldStateDomain::OoDispatch,
    EffectAccessMode::Read,
    StaticInterpreterScope::Current,
    StaticNamespaceScope::Current,
    StaticSubjectScope::Wildcard,
)];

const COMMAND_WORLD: WorldEffectDescriptor = WorldEffectDescriptor {
    composition: WorldEffectComposition::Extend,
    static_footprint: StaticEffectFootprint {
        accesses: COMMAND_WORLD_ACCESSES,
        callback: crate::CallbackEffect::NONE,
    },
    resolver: None,
    source_resolver: None,
    dynamic_fallback: crate::WorldEffectDynamicFallback::ConservativeUnknownInvocation,
};
const SUBCOMMAND_WORLD: WorldEffectDescriptor = WorldEffectDescriptor {
    composition: WorldEffectComposition::Extend,
    static_footprint: StaticEffectFootprint {
        accesses: SUBCOMMAND_WORLD_ACCESSES,
        callback: crate::CallbackEffect::NONE,
    },
    resolver: None,
    source_resolver: None,
    dynamic_fallback: crate::WorldEffectDynamicFallback::ConservativeUnknownInvocation,
};

fn form_effect_resolver(args: InvocationArguments<'_>) -> EffectFootprint {
    let mut footprint = EffectFootprint::default();
    if let Some(subject) = args.literal_at(1) {
        footprint.add_access(EffectAccess::new(
            WorldStateDomain::VariableTraces,
            EffectAccessMode::Write,
            crate::InterpreterScope::Current,
            crate::NamespaceScope::Current,
            SubjectScope::named(subject),
        ));
    }
    footprint
}

const FORM_WORLD: WorldEffectDescriptor = WorldEffectDescriptor {
    composition: WorldEffectComposition::Extend,
    static_footprint: StaticEffectFootprint::EMPTY,
    resolver: Some(form_effect_resolver),
    source_resolver: None,
    dynamic_fallback: crate::WorldEffectDynamicFallback::ConservativeUnknownInvocation,
};
const REFINING_FORM_WORLD: WorldEffectDescriptor = WorldEffectDescriptor {
    composition: WorldEffectComposition::Replace,
    static_footprint: StaticEffectFootprint::EMPTY,
    resolver: Some(form_effect_resolver),
    source_resolver: None,
    dynamic_fallback: crate::WorldEffectDynamicFallback::ConservativeUnknownInvocation,
};

const WORLD_FORMS: &[CommandForm] = &[CommandForm {
    name: "argument-targeted",
    arity: Arity::exact(1),
    world_effects: Some(FORM_WORLD),
    ..CommandForm::DEFAULT
}];
const WORLD_SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "sub",
        arity: Arity::new(0, 1),
        subcommand_forms: WORLD_FORMS,
        world_effects: Some(SUBCOMMAND_WORLD),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "refine",
        arity: Arity::exact(1),
        subcommand_forms: &[CommandForm {
            name: "precise-target",
            arity: Arity::exact(1),
            world_effects: Some(REFINING_FORM_WORLD),
            ..CommandForm::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
];

const RUN_FORMS: &[CommandForm] = &[CommandForm {
    name: "custom-code-form",
    arity: Arity::exact(0),
    completion: Some(CompletionDescriptor::exact(FORM_CODES)),
    ..CommandForm::DEFAULT
}];

const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "run",
        arity: Arity::exact(0),
        completion: Some(CompletionDescriptor::exact(SUBCOMMAND_CODES)),
        subcommand_forms: RUN_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "plain",
        arity: Arity::exact(0),
        completion: Some(CompletionDescriptor::exact(SUBCOMMAND_CODES)),
        ..SubCommand::DEFAULT
    },
];

const SAFE_ON_UNINIT_SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "narrow",
        arity: Arity::exact(0),
        safe_on_uninit: Some(SpecSurface::TCL85_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "inherit",
        arity: Arity::exact(0),
        ..SubCommand::DEFAULT
    },
];

const OBJECT_DEPENDENCY: DispatchDependencies =
    DispatchDependencies::one(DispatchDependencyDomain::ObjectDispatch);
const UNKNOWN_DEPENDENCY: DispatchDependencies =
    DispatchDependencies::one(DispatchDependencyDomain::UnknownHandling);
const DISPATCH_FORMS: &[CommandForm] = &[CommandForm {
    name: "guarded-form",
    arity: Arity::exact(0),
    dispatch_dependencies: Some(DispatchDependencyDescriptor::replace(OBJECT_DEPENDENCY)),
    ..CommandForm::DEFAULT
}];
const DISPATCH_SUBCOMMANDS: &[SubCommand] = &[SubCommand {
    name: "run",
    arity: Arity::exact(0),
    subcommand_forms: DISPATCH_FORMS,
    dispatch_dependencies: Some(DispatchDependencyDescriptor::extend(UNKNOWN_DEPENDENCY)),
    ..SubCommand::DEFAULT
}];

fn registry_with_completion_fixture() -> CommandRegistry {
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "completion-fixture",
        arity: Arity::any(),
        subcommands: SUBCOMMANDS,
        completion: Some(CompletionDescriptor::exact(COMMAND_CODES)),
        ..CommandSpec::DEFAULT
    });
    registry
}

#[test]
fn retained_result_types_respect_option_forms_and_unknown_slots() {
    use crate::InvocationWord::{Dynamic, Literal};
    let registry = crate::model::ingress::static_context_for("tcl8.6").commands();
    for (command, arguments, expected) in [
        (
            "regexp",
            vec![Literal("-inline"), Literal("a"), Dynamic],
            None,
        ),
        ("regexp", vec![Literal("a"), Dynamic], Some(TclType::Int)),
        ("regexp", vec![Dynamic, Literal("a"), Dynamic], None),
        (
            "lsearch",
            vec![Literal("-all"), Dynamic, Literal("a")],
            None,
        ),
        ("lsearch", vec![Dynamic, Literal("a")], Some(TclType::Int)),
        ("lsearch", vec![Literal("--"), Dynamic, Literal("a")], None),
        (
            "regsub",
            vec![Literal("a"), Dynamic, Literal("b")],
            Some(TclType::String),
        ),
    ] {
        let invocation = registry.resolve_structured_invocation(
            crate::InvocationWords::structured(Literal(command), &arguments),
            registry.own_surface_query(),
        );
        assert_eq!(
            invocation.resolved().unwrap().facts().return_type,
            expected,
            "{command} {arguments:?}"
        );
    }
}

#[test]
fn literal_leaf_normal_world_is_separate_from_error_and_dynamic_operands() {
    let registry = crate::model::ingress::static_context_for("tcl8.6").commands();
    let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    let arguments = [
        crate::InvocationWord::Literal("format"),
        crate::InvocationWord::Literal("a*"),
        crate::InvocationWord::Literal("hello"),
    ];
    let resolve = |arguments| {
        registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("binary"),
                    arguments,
                )
                .with_dialect(dialect),
                registry.own_surface_query(),
            )
            .resolved()
            .unwrap()
    };
    let invocation = resolve(&arguments);
    assert!(invocation.facts().effects.requires_world_barrier());
    let normal = invocation.facts_after_success();
    assert!(!normal.effects.requires_world_barrier());
    assert!(
        normal
            .effects
            .accesses()
            .iter()
            .all(|access| access.domain == WorldStateDomain::InterpreterResult)
    );
    assert_eq!(
        normal.state_transitions.declared().unwrap().facts().len(),
        0
    );
    let dynamic = [
        crate::InvocationWord::Literal("format"),
        crate::InvocationWord::Literal("a*"),
        crate::InvocationWord::Dynamic,
    ];
    assert!(
        resolve(&dynamic)
            .facts_after_success()
            .effects
            .requires_world_barrier()
    );
}

#[test]
fn selected_getter_normal_contract_does_not_leak_to_mutator_or_channel_forms() {
    let registry = crate::model::ingress::static_context_for("f5-irules").commands();
    let dialect = crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
    for values in [&[][..], &["100"][..]] {
        let invocation = registry
            .resolve_structured_invocation(
                crate::InvocationWords::literals("TCP::payload", values).with_dialect(dialect),
                registry.own_surface_query(),
            )
            .resolved()
            .unwrap();
        assert_eq!(
            invocation.facts().successful_handler,
            Some(crate::native_compilation::SuccessfulHandlerSpec::Leaf)
        );
        assert!(
            !invocation
                .facts_after_success()
                .effects
                .requires_world_barrier()
        );
        assert!(invocation.facts().native_compilation.is_none());
    }
    let replace = registry
        .resolve_invocation(
            "TCP::payload",
            &["replace", "0", "1", "data"],
            registry.own_surface_query(),
        )
        .unwrap();
    assert!(replace.facts().successful_handler.is_none());
    let registry = crate::model::ingress::static_context_for("tcl8.6").commands();
    let current = registry
        .resolve_invocation("pid", &[], registry.own_surface_query())
        .unwrap();
    assert_eq!(
        current.facts().successful_handler,
        Some(crate::native_compilation::SuccessfulHandlerSpec::Leaf)
    );
    let channel = registry
        .resolve_invocation("pid", &["channel"], registry.own_surface_query())
        .unwrap();
    assert!(channel.facts().successful_handler.is_none());
}

#[test]
fn dictionary_constructor_coerces_keys_and_retains_unknown_value_objects() {
    let registry = crate::model::ingress::static_context_for("tcl8.6").commands();
    let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    for (key, value, closed) in [
        (
            crate::InvocationWord::Literal("key"),
            crate::InvocationWord::Dynamic,
            true,
        ),
        (
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Literal("value"),
            false,
        ),
    ] {
        let operands = [crate::InvocationWord::Literal("create"), key, value];
        let invocation = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("dict"),
                    &operands,
                )
                .with_dialect(dialect),
                registry.own_surface_query(),
            )
            .resolved()
            .unwrap();
        let effects = invocation.facts_after_success().effects;
        assert_eq!(!effects.requires_world_barrier(), closed);
        if closed {
            assert!(
                effects
                    .accesses()
                    .iter()
                    .all(|access| access.domain == WorldStateDomain::InterpreterResult)
            );
        }
    }
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
fn completion_descriptor_precedence_is_form_then_subcommand_then_command() {
    let registry = registry_with_completion_fixture();

    let form = registry
        .resolve_invocation("completion-fixture", &["run"], None)
        .expect("fixture command resolves");
    assert_eq!(
        form.semantics.completion.codes,
        CompletionCodeDomain::Exact(FORM_CODES),
        "the matched form owns the most-specific descriptor"
    );

    let subcommand = registry
        .resolve_invocation("completion-fixture", &["plain"], None)
        .expect("fixture command resolves");
    assert_eq!(
        subcommand.semantics.completion.codes,
        CompletionCodeDomain::Exact(SUBCOMMAND_CODES),
        "a resolved subcommand overrides its parent command"
    );

    let command = registry
        .resolve_invocation("completion-fixture", &[], None)
        .expect("fixture command resolves without a subcommand");
    assert_eq!(
        command.semantics.completion.codes,
        CompletionCodeDomain::Exact(COMMAND_CODES),
        "the command descriptor applies when no narrower form resolved"
    );
}

#[test]
fn registry_exposes_standard_code_descriptors_and_conservative_invoke_fallback() {
    let mut registry = CommandRegistry::build_default();

    let error = registry
        .resolve_invocation("error", &["boom"], None)
        .expect("error is a core command");
    assert_eq!(
        error.semantics.completion.codes,
        CompletionCodeDomain::Exact(&[CompletionCode::Error]),
        "the command registry, not a compiler spelling match, owns error's code"
    );

    registry.insert(CommandSpec {
        name: "completion-generic-fallback",
        ..CommandSpec::DEFAULT
    });
    let generic = registry
        .resolve_invocation("completion-generic-fallback", &[], None)
        .expect("unstamped fixture command resolves");
    assert_eq!(
        generic.semantics.completion,
        CompletionDescriptor::CONSERVATIVE,
        "an unstamped command must retain generic Invoke semantics"
    );
    assert_eq!(
        generic.semantics.operation,
        SemanticOperationId::Invoke,
        "the conservative descriptor does not enable a specialisation"
    );
    let generic_effects = generic.effects();
    assert!(generic_effects.callback().kinds.is_unknown());
    assert!(
        generic_effects.requires_world_barrier(),
        "unstamped generic Invoke must not be interpreted as pure"
    );
    let generic_transitions = generic.facts().state_transitions;

    registry.insert(CommandSpec {
        name: "explicit-closed-effect-fixture",
        world_effects: Some(WorldEffectDescriptor::EMPTY),
        ..CommandSpec::DEFAULT
    });
    let explicit_empty = registry
        .resolve_invocation("explicit-closed-effect-fixture", &[], None)
        .expect("explicitly effect-free fixture resolves")
        .effects();
    assert_eq!(explicit_empty.accesses().len(), 0);
    assert!(
        !explicit_empty.requires_world_barrier(),
        "the explicit EMPTY descriptor, not missing metadata, proves effect-freedom"
    );

    assert!(matches!(
        generic_transitions,
        StateTransitionKnowledge::UnknownInvocation
    ));

    registry.insert(CommandSpec {
        name: "explicit-closed-transition-fixture",
        state_transitions: Some(StateTransitionDescriptor::EMPTY),
        ..CommandSpec::DEFAULT
    });
    let explicit_transitions = registry
        .resolve_invocation("explicit-closed-transition-fixture", &[], None)
        .expect("explicitly transition-free fixture resolves")
        .facts()
        .state_transitions;
    assert!(matches!(
        explicit_transitions,
        StateTransitionKnowledge::Declared(ref transitions) if transitions.facts().is_empty()
    ));
}

#[test]
fn scalar_math_shape_requires_the_selected_native_protocol() {
    let registry = CommandRegistry::build_default();
    let values = [crate::InvocationWord::Dynamic];
    for version in [
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let dialect = crate::InvocationDialect::for_version(version);
        let words = crate::InvocationWords::structured(
            crate::InvocationWord::Literal("::tcl::mathfunc::sqrt"),
            &values,
        )
        .with_dialect(dialect);
        let resolution = registry.resolve_structured_invocation(words, dialect.authoring_query());
        let mut facts = resolution.resolved().unwrap().facts();
        let protocol = facts
            .normal_scalar_math_protocol(words.arguments())
            .unwrap();
        assert_eq!(protocol.operand_at(), 0);
        assert_eq!(
            protocol.numeric_operand_policy(),
            crate::mathfunc::NativeMathNumericOperandPolicy::PreserveCategory
        );
        assert_eq!(
            facts.normal_numeric_result_production(words.arguments()),
            Some(crate::native_result::NativeNumericResultProduction::Double)
        );
        assert_eq!(
            facts.completion.codes,
            crate::completion::CompletionCodeDomain::Exact(&[
                CompletionCode::Ok,
                CompletionCode::Error
            ])
        );
        let original_effects = facts.effects.clone();
        facts.refine_scalar_math_input_effects(words.arguments(), true);
        assert_eq!(facts.effects, original_effects);
        facts.refine_scalar_math_input_effects(words.arguments(), false);
        assert!(
            facts
                .effects
                .callback()
                .kinds
                .contains(crate::CallbackKinds::HOST)
        );
        assert!(!facts.traits.contains(Traits::PURE));
        assert_eq!(
            facts.normal_numeric_result_production(words.arguments()),
            Some(crate::native_result::NativeNumericResultProduction::Double)
        );
        facts.successful_handler = None;
        assert!(
            facts
                .normal_scalar_math_protocol(words.arguments())
                .is_none()
        );
        facts.successful_handler = Some(crate::native_compilation::SuccessfulHandlerSpec::Leaf);
        facts.native_result = None;
        assert!(
            facts
                .normal_numeric_result_production(words.arguments())
                .is_none()
        );
    }
    let expanded = [crate::InvocationWord::Expanded];
    let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_1);
    let words = crate::InvocationWords::structured(
        crate::InvocationWord::Literal("::tcl::mathfunc::sqrt"),
        &expanded,
    )
    .with_dialect(dialect);
    let facts = registry
        .resolve_structured_invocation(words, dialect.authoring_query())
        .resolved()
        .unwrap()
        .facts();
    assert!(
        facts
            .normal_scalar_math_protocol(words.arguments())
            .is_none()
    );
    assert!(
        crate::native_result::NativeResultContract::ScalarMath(
            crate::mathfunc::NativeScalarMathOperation::Sqrt
        )
        .normal_numeric_result_production(crate::InvocationArguments::literals(&["4"]), 0)
        .is_none()
    );
}

#[test]
fn native_list_length_numeric_result_requires_selected_contract_and_normal_identity() {
    let registry = CommandRegistry::build_default();
    let values = [crate::InvocationWord::Dynamic];
    let mut dialects = tcl_dialect::TclVersion::ALL
        .into_iter()
        .map(crate::InvocationDialect::for_version)
        .collect::<Vec<_>>();
    dialects.push(crate::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
    ));
    for dialect in dialects {
        let words =
            crate::InvocationWords::structured(crate::InvocationWord::Literal("llength"), &values)
                .with_dialect(dialect);
        let resolution = registry.resolve_structured_invocation(words, dialect.authoring_query());
        let mut facts = resolution.resolved().unwrap().facts();
        assert_eq!(
            facts.normal_numeric_result_production(words.arguments()),
            Some(
                crate::native_result::NativeNumericResultProduction::Integer {
                    arithmetic: dialect.arithmetic().unwrap(),
                }
            )
        );
        facts.operation = SemanticOperationId::Invoke;
        assert!(
            facts
                .normal_numeric_result_production(words.arguments())
                .is_none()
        );
        facts.operation = SemanticOperationId::Intrinsic(IntrinsicId::ListLength);
        facts.successful_handler = None;
        assert!(
            facts
                .normal_numeric_result_production(words.arguments())
                .is_none()
        );
    }
    let words = crate::InvocationWords::structured(
        crate::InvocationWord::Literal("llength"),
        &[crate::InvocationWord::Expanded],
    )
    .with_dialect(crate::InvocationDialect::for_version(
        tcl_dialect::TclVersion::V9_1,
    ));
    let resolution = registry.resolve_structured_invocation(words, None);
    assert!(
        resolution
            .resolved()
            .unwrap()
            .facts()
            .normal_numeric_result_production(words.arguments())
            .is_none()
    );
}

#[test]
fn ordinary_list_length_completion_requires_the_selected_operation_and_handler() {
    use crate::representation::OrdinaryContainerRepresentation;
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let registry = crate::model::ingress::static_context_for(environment).commands();
        let profile = crate::model::ingress::resolve_environment(environment).unit_profile();
        let dialect = crate::InvocationDialect::of_profile(profile);
        let operands = [crate::InvocationWord::Dynamic];
        let words =
            InvocationWords::structured(crate::InvocationWord::Literal("llength"), &operands)
                .with_dialect(dialect);
        let resolved = registry.resolve_structured_invocation(words, dialect.authoring_query());
        let mut facts = resolved.resolved().expect("native list length").facts();
        assert!(
            facts
                .ordinary_list_length_completion(
                    words.arguments(),
                    0,
                    OrdinaryContainerRepresentation::Dictionary,
                )
                .is_some(),
            "{environment}"
        );
        facts.operation = SemanticOperationId::Invoke;
        assert!(
            facts
                .ordinary_list_length_completion(
                    words.arguments(),
                    0,
                    OrdinaryContainerRepresentation::Dictionary,
                )
                .is_none()
        );
        facts.operation = SemanticOperationId::Intrinsic(IntrinsicId::ListLength);
        facts.successful_handler = None;
        assert!(
            facts
                .ordinary_list_length_completion(
                    words.arguments(),
                    0,
                    OrdinaryContainerRepresentation::Dictionary,
                )
                .is_none()
        );
    }
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "coercion-only",
        arity: Arity::exact(1),
        representation_effect: Some(RepresentationEffect::CoerceOrdinaryList { operand: 0 }),
        successful_handler: Some(crate::native_compilation::SuccessfulHandlerSpec::Leaf),
        ..CommandSpec::DEFAULT
    });
    let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    let words = InvocationWords::literals("coercion-only", &["a b"]).with_dialect(dialect);
    let resolved = registry.resolve_structured_invocation(words, dialect.authoring_query());
    assert!(
        resolved
            .resolved()
            .expect("metadata fixture")
            .facts()
            .ordinary_list_length_completion(
                words.arguments(),
                0,
                OrdinaryContainerRepresentation::List,
            )
            .is_none()
    );
}

#[test]
fn list_length_abstract_method_obligation_retains_native_axes() {
    use crate::representation::ListLengthObjectProtocol as Protocol;
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let registry = crate::model::ingress::static_context_for(environment).commands();
        let profile = crate::model::ingress::resolve_environment(environment).unit_profile();
        let dialect = crate::InvocationDialect::of_profile(profile);
        let operands = [crate::InvocationWord::Dynamic];
        let words =
            InvocationWords::structured(crate::InvocationWord::Literal("llength"), &operands)
                .with_dialect(dialect);
        let resolved = registry.resolve_structured_invocation(words, dialect.authoring_query());
        let mut facts = resolved.resolved().expect("native list length").facts();
        let expected = if dialect
            .tcl_version
            .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_0)
        {
            Protocol::AbstractLength { argument: 0 }
        } else {
            Protocol::Ordinary { argument: 0 }
        };
        assert_eq!(
            facts.list_length_object_protocol(words.arguments()),
            Some(expected),
            "{environment}"
        );
        assert_eq!(
            facts.list_length_object_protocol(crate::InvocationArguments::structured(&operands)),
            Some(Protocol::Unknown)
        );
        facts.operation = SemanticOperationId::Invoke;
        assert_eq!(facts.list_length_object_protocol(words.arguments()), None);
        facts.operation = SemanticOperationId::Intrinsic(IntrinsicId::ListLength);
        facts.successful_handler = None;
        assert_eq!(facts.list_length_object_protocol(words.arguments()), None);
    }
}

#[test]
fn semantic_operations_keep_structural_lowering_and_precise_leaf_intrinsics_distinct() {
    let registry = CommandRegistry::build_default();
    let dialect = Some(SurfaceQuery::core(Family::Tcl, "9.0"));

    for (command, arguments, lowering) in [
        ("incr", &["counter"][..], LoweringHookId::Incr),
        ("catch", &["body"][..], LoweringHookId::Catch),
        ("return", &[][..], LoweringHookId::Return),
        ("global", &["name"][..], LoweringHookId::Global),
    ] {
        let invocation = registry
            .resolve_invocation(command, arguments, dialect)
            .expect("core structural command resolves");
        assert_eq!(
            invocation.semantics.operation,
            SemanticOperationId::StructuredLowering(lowering)
        );
    }

    let llength = registry
        .resolve_invocation("llength", &["value"], dialect)
        .expect("llength resolves");
    assert_eq!(
        llength.semantics.operation,
        SemanticOperationId::Intrinsic(IntrinsicId::ListLength)
    );
    let lindex = registry
        .resolve_invocation("lindex", &["value", "0"], dialect)
        .expect("lindex resolves");
    assert_eq!(
        lindex.semantics.operation,
        SemanticOperationId::Intrinsic(IntrinsicId::ListIndex)
    );

    let dict_get = registry
        .resolve_invocation("dict", &["get", "value", "key"], dialect)
        .expect("dict get resolves");
    assert_eq!(
        dict_get.semantics.operation,
        SemanticOperationId::Intrinsic(IntrinsicId::DictGet)
    );

    let namespace_eval = registry
        .resolve_invocation("namespace", &["eval", "ns", "body"], dialect)
        .expect("namespace eval resolves");
    assert_eq!(
        namespace_eval.semantics.operation,
        SemanticOperationId::StructuredLowering(LoweringHookId::NamespaceEval)
    );

    let broad_string_family = registry
        .resolve_invocation("string", &["first", "a", "abc"], dialect)
        .expect("string first resolves");
    assert_eq!(
        broad_string_family.semantics.operation,
        SemanticOperationId::Invoke,
        "an unstamped emitter family is not a target-neutral intrinsic"
    );
}

#[test]
fn world_effect_descriptors_compose_by_default_and_can_explicitly_refine() {
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "world-effect-fixture",
        arity: Arity::any(),
        subcommands: WORLD_SUBCOMMANDS,
        world_effects: Some(COMMAND_WORLD),
        ..CommandSpec::DEFAULT
    });

    let command = registry
        .resolve_invocation("world-effect-fixture", &[], None)
        .expect("fixture command resolves");
    let command_effects = command.effects();
    assert_eq!(command_effects.accesses().len(), 1);
    assert_eq!(
        command_effects.accesses()[0].domain,
        WorldStateDomain::PackageState,
        "the command descriptor applies without a narrower match"
    );

    let subcommand = registry
        .resolve_invocation("world-effect-fixture", &["sub"], None)
        .expect("fixture subcommand resolves");
    let subcommand_effects = subcommand.effects();
    assert!(
        subcommand_effects
            .accesses()
            .iter()
            .any(|access| access.domain == WorldStateDomain::PackageState)
    );
    assert!(
        subcommand_effects
            .accesses()
            .iter()
            .any(|access| access.domain == WorldStateDomain::OoDispatch)
    );

    let form = registry
        .resolve_invocation("world-effect-fixture", &["sub", "targetCell"], None)
        .expect("fixture form resolves");
    let form_effects = form.effects();
    assert!(
        form_effects
            .accesses()
            .iter()
            .any(|access| access.domain == WorldStateDomain::PackageState)
    );
    assert!(
        form_effects
            .accesses()
            .iter()
            .any(|access| access.domain == WorldStateDomain::OoDispatch)
    );
    assert!(form_effects.accesses().iter().any(|access| {
        access.domain == WorldStateDomain::VariableTraces
            && access.subject == SubjectScope::named("targetCell")
    }));

    let refined = registry
        .resolve_invocation("world-effect-fixture", &["refine", "targetCell"], None)
        .expect("refining fixture form resolves");
    let refined_effects = refined.effects();
    assert_eq!(refined_effects.accesses().len(), 1);
    assert_eq!(
        refined_effects.accesses()[0].domain,
        WorldStateDomain::VariableTraces,
        "only an explicit Replace descriptor may discard inherited effects"
    );
    assert_eq!(
        refined_effects.accesses()[0].subject,
        SubjectScope::named("targetCell"),
        "the replacement descriptor is allowed to refine a wildcard parent subject"
    );
}

#[test]
fn dynamic_effect_arguments_stay_conservative_without_exposing_a_value() {
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "world-effect-fixture",
        arity: Arity::any(),
        subcommands: WORLD_SUBCOMMANDS,
        world_effects: Some(COMMAND_WORLD),
        ..CommandSpec::DEFAULT
    });
    let arguments = [
        crate::InvocationWord::Literal("sub"),
        crate::InvocationWord::Dynamic,
    ];
    let invocation = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(
                crate::InvocationWord::Literal("world-effect-fixture"),
                &arguments,
            ),
            None,
        )
        .resolved()
        .expect("literal fixture command resolves");

    assert_eq!(
        invocation.form.map(|form| form.name),
        Some("argument-targeted"),
        "a dynamic non-expanded word preserves the arity without becoming a value"
    );
    let effects = invocation.effects();
    assert!(effects.requires_world_barrier());
    assert!(effects.callback().kinds.is_unknown());
    assert!(
        !effects.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::VariableTraces
                && access.subject != SubjectScope::Wildcard
        }),
        "the resolver cannot manufacture a named target from a dynamic word"
    );
}

#[test]
fn invocation_facts_are_owned_and_target_neutral() {
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "world-effect-fixture",
        arity: Arity::any(),
        subcommands: WORLD_SUBCOMMANDS,
        world_effects: Some(COMMAND_WORLD),
        ..CommandSpec::DEFAULT
    });
    let invocation = registry
        .resolve_invocation("world-effect-fixture", &["sub", "targetCell"], None)
        .expect("literal fixture command resolves");
    let facts = invocation.facts();

    assert_eq!(facts.canonical_command, "world-effect-fixture");
    assert_eq!(facts.subcommand.canonical_name(), Some("sub"));
    assert!(matches!(
        &facts.subcommand,
        OwnedSubcommandResolution::Exact {
            spelling,
            canonical_name,
        } if spelling == "sub" && canonical_name == "sub"
    ));
    assert_eq!(facts.form.as_deref(), Some("argument-targeted"));
    assert_eq!(facts.operation, SemanticOperationId::Invoke);
    assert_eq!(facts.completion, CompletionDescriptor::CONSERVATIVE);
    assert_eq!(facts.result_stability, ResultStability::Unknown);
    assert_eq!(facts.arity, Arity::exact(1));
    assert_eq!(facts.argument_offset, 1);
    assert_eq!(facts.arg_roles.len(), 0);
    assert!(facts.arg_roles_complete);
    assert_eq!(facts.return_type, None);
    assert_eq!(facts.var_write_typing, VarWriteTyping::ReturnValue);
    assert_eq!(facts.return_elements, None);
    assert_eq!(facts.var_elements_effect, None);
    assert_eq!(facts.body_kind, BodyKind::Plain);
    assert_eq!(facts.frame_effect, None);
    assert!(facts.effects.accesses().iter().any(|access| {
        access.domain == WorldStateDomain::VariableTraces
            && access.subject == SubjectScope::named("targetCell")
    }));
}

#[test]
fn invocation_facts_expose_closed_referential_result_proofs() {
    let registry = CommandRegistry::build_default();
    let cases: &[(&str, &[&str])] = &[
        ("format", &["%s", "x"]),
        ("join", &["a b"]),
        ("lindex", &["a b", "0"]),
        ("linsert", &["a b", "0", "x"]),
        ("llength", &["a b"]),
        ("lrange", &["a b", "0", "end"]),
        ("lremove", &["a b", "0"]),
        ("lreplace", &["a b", "0", "0", "x"]),
        ("lreverse", &["a b"]),
        ("split", &["ab"]),
    ];

    for (command, arguments) in cases {
        let facts = registry
            .resolve_invocation(
                command,
                arguments,
                Some(SurfaceQuery::core(Family::Tcl, "9.1")),
            )
            .unwrap_or_else(|| panic!("{command} resolves"))
            .facts();

        assert_eq!(
            facts.result_stability,
            ResultStability::ReferentiallyTransparent,
            "{command}"
        );
        assert!(
            facts.traits.contains(Traits::PURE | Traits::CSE_CANDIDATE),
            "{command}"
        );
        assert!(facts.effects.accesses().is_empty(), "{command}");
        assert!(!facts.effects.requires_world_barrier(), "{command}");
        assert!(
            matches!(
                facts.state_transitions,
                StateTransitionKnowledge::Declared(ref transitions)
                    if transitions.facts().is_empty()
            ),
            "{command}"
        );
    }
}

#[test]
fn selected_scope_alias_traits_include_subcommand_declarations() {
    let registry = CommandRegistry::build_default();
    for (command, arguments, expected) in [
        ("namespace", &["upvar", "::ns", "source", "local"][..], true),
        ("dict", &["with", "dictionary", "puts $local"][..], true),
        (
            "dict",
            &["update", "dictionary", "key", "local", "puts $local"][..],
            true,
        ),
        ("my", &["variable", "local"][..], true),
        ("namespace", &["origin", "::p"][..], false),
    ] {
        let facts = registry
            .resolve_invocation(
                command,
                arguments,
                Some(SurfaceQuery::core(Family::Tcl, "9.1")),
            )
            .unwrap_or_else(|| panic!("{command} {arguments:?} resolves"))
            .facts();
        assert_eq!(
            facts.traits.contains(Traits::CREATES_SCOPE_ALIAS),
            expected,
            "{command} {arguments:?}"
        );
    }
}

/// The purity doors answer the authored facts for Tcl 8.4 to 9.1:
/// `llength` is pure and a CSE candidate, a braced `expr` evaluates
/// purely without being declared pure, `puts` is neither, and a host
/// callback withdraws every claim.
#[test]
fn purity_queries_answer_the_selected_invocation() {
    let registry = CommandRegistry::build_default();
    for release in ["8.4", "8.5", "8.6", "9.0", "9.1"] {
        let dialect = Some(SurfaceQuery::core(Family::Tcl, release));
        let facts = |name: &str, args: &[&str]| {
            registry
                .resolve_invocation(name, args, dialect)
                .unwrap_or_else(|| panic!("Tcl {release} resolves {name}"))
                .facts()
        };
        let mut llength = facts("llength", &["$items"]);
        assert!(llength.is_pure() && llength.is_cse_pure(), "Tcl {release}");
        assert!(llength.evaluates_purely(), "Tcl {release}");
        llength.withdraw_purity();
        assert!(
            !llength.is_pure() && !llength.evaluates_purely(),
            "Tcl {release}"
        );
        assert!(
            !llength.traits.contains(Traits::FRAMELESS_RUNTIME),
            "Tcl {release}"
        );

        let expr = facts("expr", &["{$a + 1}"]);
        assert!(expr.evaluates_purely() && !expr.is_pure(), "Tcl {release}");
        let puts = facts("puts", &["hello"]);
        assert!(
            !puts.evaluates_purely() && !puts.is_cse_pure(),
            "Tcl {release}"
        );
    }
    let llength = registry.get("llength").expect("llength");
    assert!(llength.is_pure());
    let string = registry.get("string").expect("string");
    assert!(!string.is_pure());
    assert!(
        string
            .resolve_subcommand("length")
            .is_some_and(SubCommand::is_pure)
    );
}

#[test]
fn clock_result_dependencies_are_selected_by_subcommand() {
    let registry = CommandRegistry::build_default();
    let seconds = registry
        .resolve_invocation(
            "clock",
            &["seconds"],
            Some(SurfaceQuery::core(Family::Tcl, "9.0")),
        )
        .expect("clock seconds resolves")
        .facts();
    assert_eq!(seconds.result_stability, ResultStability::Volatile);
    assert!(
        seconds.traits.contains(Traits::PURE),
        "the established subcommand pure fact must reach common semantics"
    );
    assert!(seconds.effects.requires_world_barrier());
    assert_eq!(
        seconds.state_transitions,
        StateTransitionKnowledge::UnknownInvocation,
        "a result declaration must not fabricate effect or transition closure"
    );

    let add = registry
        .resolve_invocation(
            "clock",
            &["add", "0", "1", "day"],
            Some(SurfaceQuery::core(Family::Tcl, "9.0")),
        )
        .expect("clock add resolves")
        .facts();
    assert!(matches!(
        add.result_stability,
        ResultStability::ReadsVersionedWorld(_)
    ));
    let dependencies = add.result_stability.versioned_world_dependencies();
    assert!(dependencies.contains(&WorldStateDomain::HostCapabilities));
    assert!(dependencies.contains(&WorldStateDomain::PackageState));
    assert!(dependencies.contains(&WorldStateDomain::VariableStore));
    assert!(add.effects.requires_world_barrier());
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

#[test]
fn successful_frame_facts_are_separate_from_possible_argument_error_facts() {
    let registry = CommandRegistry::build_default();
    let arguments = [
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Literal("linked"),
    ];
    let words =
        crate::InvocationWords::structured(crate::InvocationWord::Literal("upvar"), &arguments)
            .with_dialect(crate::InvocationDialect::for_version(
                tcl_dialect::TclVersion::V8_4,
            ));
    let invocation = registry
        .resolve_structured_invocation(words, Some(SurfaceQuery::core(Family::Tcl, "8.4")))
        .resolved()
        .expect("selected native upvar");
    let ordinary = invocation.facts();
    assert!(!ordinary.arg_roles_complete);
    assert!(
        !ordinary
            .state_transitions
            .declared()
            .unwrap()
            .facts()
            .iter()
            .any(|fact| { matches!(fact.transition, StateTransition::VariableCellAlias(_)) })
    );
    let normal = invocation.facts_after_success();
    assert!(normal.arg_roles_complete);
    assert_eq!(
        normal.arg_roles,
        vec![(0, ArgRole::VarCell), (1, ArgRole::VarWrite)]
    );
    assert!(
        normal
            .state_transitions
            .declared()
            .unwrap()
            .facts()
            .iter()
            .any(|fact| {
                matches!(&fact.transition, StateTransition::VariableCellAlias(alias)
                if alias.local.literal() == Some("linked")
                    && matches!(&alias.target, VariableAliasTarget::CallerSelectedFrame {
                        frame: CallerFrameSelection::DefaultCaller, ..
                    }))
            })
    );
    // Ordinary facts still retain the partial-commit error transfer; the
    // normal projection cannot replace them in caught-error states.
    assert!(
        ordinary
            .state_transitions
            .declared()
            .unwrap()
            .facts()
            .iter()
            .all(|fact| {
                fact.commit.abrupt_transfer() == AbruptTransitionTransfer::JoinWithTransition
            })
    );
}

#[test]
fn transition_facts_keep_literal_subjects_precise_and_dynamic_operands_scoped() {
    let registry = CommandRegistry::build_default();
    let arguments = [
        crate::InvocationWord::Literal("::precise"),
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Dynamic,
    ];
    let invocation = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(crate::InvocationWord::Literal("proc"), &arguments),
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .resolved()
        .expect("literal proc resolves");
    let facts = invocation.facts();

    let [transition] = facts
        .state_transitions
        .declared()
        .expect("proc declares state transitions")
        .facts()
    else {
        panic!("proc must emit exactly one definition transition");
    };
    assert_eq!(transition.commit, StateTransitionCommit::OnOkOnly);
    assert_eq!(
        transition.commit.abrupt_transfer(),
        AbruptTransitionTransfer::Unchanged,
        "a failed proc must not be treated as an unconditional definition"
    );
    assert_eq!(
        transition.transition,
        StateTransition::CommandBinding(CommandBindingTransition::Define {
            name: TransitionSubject::LocatedLiteral {
                value: "::precise".to_owned(),
                argument_index: 0
            },
            kind: CommandBindingDefinitionKind::Procedure,
        })
    );
}

#[test]
fn expanded_positional_transition_grammar_suppresses_precise_facts() {
    let registry = CommandRegistry::build_default();
    let arguments = [
        crate::InvocationWord::Literal("::maybe"),
        crate::InvocationWord::Expanded,
        crate::InvocationWord::Literal("body"),
    ];
    let invocation = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(crate::InvocationWord::Literal("proc"), &arguments),
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .resolved()
        .expect("literal proc resolves despite an expanded argument");
    let facts = invocation.facts();

    let transitions = facts
        .state_transitions
        .declared()
        .expect("proc declares state transitions");
    assert!(
        transitions.facts().iter().all(|fact| !matches!(
            fact.transition,
            StateTransition::CommandBinding(CommandBindingTransition::Define { .. })
        )),
        "an expanded argv segment can change proc's positional grammar"
    );
    assert!(transitions.facts().iter().any(|fact| {
        matches!(
            fact.transition,
            StateTransition::Widen(ref widening)
                if widening.subject
                    == TransitionSubject::Unknown {
                        argument_index: 1,
                        word_kind: crate::InvocationWordKind::Expanded,
                    }
        )
    }));
}

#[test]
fn namespace_eval_and_dynamic_delete_materialise_closed_namespace_facts() {
    let registry = CommandRegistry::build_default();
    let eval = registry
        .resolve_invocation(
            "namespace",
            &["eval", "::a", "error x"],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .expect("namespace eval resolves")
        .facts();
    let [ensure] = eval
        .state_transitions
        .declared()
        .expect("namespace eval declares transitions")
        .facts()
    else {
        panic!("namespace eval must emit exactly one ensure transition");
    };
    assert_eq!(
        ensure.commit,
        StateTransitionCommit::MayCommitBeforeAbruptCompletion
    );
    assert!(matches!(
        &ensure.transition,
        StateTransition::Namespace(NamespaceTransition::Ensure {
            namespace: NamespaceTransitionTarget::Named(TransitionSubject::LocatedLiteral { value: name, .. }),
        }) if name == "::a"
    ));
    assert!(
        eval.effects
            .callback()
            .kinds
            .contains(CallbackKinds::SCRIPT)
    );
    assert_eq!(
        eval.effects.callback().reentrancy,
        Reentrancy::CurrentInterpreter
    );

    let arguments = [
        crate::InvocationWord::Literal("delete"),
        crate::InvocationWord::Dynamic,
    ];
    let delete = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(
                crate::InvocationWord::Literal("namespace"),
                &arguments,
            ),
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .resolved()
        .expect("literal namespace delete resolves")
        .facts();
    let transitions = delete
        .state_transitions
        .declared()
        .expect("namespace delete declares transitions");
    assert!(transitions.facts().iter().any(|fact| {
        matches!(
            &fact.transition,
            StateTransition::Widen(widening)
                if widening.domains.contains(&StateTransitionDomain::ObjectDispatch)
        )
    }));
}

#[test]
fn interp_alias_query_and_delete_do_not_claim_alias_creation() {
    let registry = CommandRegistry::build_default();
    let query = registry
        .resolve_invocation(
            "interp",
            &["alias", "", "shortcut"],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .expect("interp alias query resolves")
        .facts();
    assert_eq!(
        query
            .state_transitions
            .declared()
            .expect("interp alias declares transitions")
            .facts()
            .len(),
        0
    );

    let delete = registry
        .resolve_invocation(
            "interp",
            &["alias", "", "shortcut", ""],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .expect("interp alias delete resolves")
        .facts();
    let [transition] = delete
        .state_transitions
        .declared()
        .expect("interp alias declares transitions")
        .facts()
    else {
        panic!("alias delete must emit one deletion transition");
    };
    assert!(matches!(
        &transition.transition,
        StateTransition::CommandBinding(CommandBindingTransition::Delete {
            interpreter: Some(TransitionSubject::LocatedLiteral { value: path, .. }),
            name: TransitionSubject::LocatedLiteral { value: name, .. },
        }) if path.is_empty() && name == "shortcut"
    ));
    assert_eq!(
        transition.commit,
        StateTransitionCommit::MayCommitBeforeAbruptCompletion
    );
}

#[test]
fn frame_and_namespace_alias_transitions_follow_registry_layouts() {
    let registry = CommandRegistry::build_default();
    let upvar = registry
        .resolve_invocation(
            "upvar",
            &["1", "other", "local"],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .expect("upvar resolves")
        .facts();
    assert!(matches!(
        upvar
            .state_transitions
            .declared()
            .expect("upvar declares transitions")
            .facts(),
        [transition]
            if matches!(
                &transition.transition,
                StateTransition::VariableCellAlias(alias)
                    if alias.local == TransitionSubject::LocatedLiteral { value: "local".to_owned(), argument_index: 2 }
                        && alias.target == VariableAliasTarget::CallerSelectedFrame {
                            frame: CallerFrameSelection::Explicit(
                                TransitionSubject::LocatedLiteral { value: "1".to_owned(), argument_index: 0 }
                            ),
                            variable: TransitionSubject::LocatedLiteral { value: "other".to_owned(), argument_index: 1 },
                        }
            )
    ));

    let namespace_upvar = registry
        .resolve_invocation(
            "namespace",
            &["upvar", "::scope", "other", "local"],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .expect("namespace upvar resolves")
        .facts();
    assert!(matches!(
        namespace_upvar
            .state_transitions
            .declared()
            .expect("namespace upvar declares transitions")
            .facts(),
        [transition]
            if matches!(
                &transition.transition,
                StateTransition::VariableCellAlias(alias)
                    if alias.local == TransitionSubject::LocatedLiteral { value: "local".to_owned(), argument_index: 3 }
                        && alias.target == VariableAliasTarget::Namespace {
                            namespace: TransitionSubject::LocatedLiteral { value: "::scope".to_owned(), argument_index: 1 },
                            variable: TransitionSubject::LocatedLiteral { value: "other".to_owned(), argument_index: 2 },
                        }
            )
    ));
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
fn invocation_facts_keep_conservative_dispatch_dependencies_by_default() {
    let facts = {
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "unstamped-dispatch-fixture",
            arity: Arity::exact(0),
            ..CommandSpec::DEFAULT
        });
        registry
            .resolve_invocation("unstamped-dispatch-fixture", &[], None)
            .expect("fixture command resolves")
            .facts()
    };

    assert_eq!(
        facts.dispatch_dependencies,
        DispatchDependencies::CONSERVATIVE
    );
    assert!(
        facts
            .dispatch_dependencies
            .contains(DispatchDependencyDomain::UnknownHandling),
        "static registry resolution is not itself a live-dispatch proof"
    );
}

#[test]
fn safe_on_uninit_resolves_from_the_matched_subcommand() {
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "safe-on-uninit-fixture",
        arity: Arity::any(),
        safe_on_uninit: Some(SpecSurface::ALL_TCL),
        subcommands: SAFE_ON_UNINIT_SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    });

    let narrow = registry
        .resolve_invocation("safe-on-uninit-fixture", &["narrow"], None)
        .expect("fixture subcommand resolves");
    assert_eq!(
        narrow.semantics.safe_on_uninit,
        Some(SpecSurface::TCL85_PLUS)
    );

    let inherited = registry
        .resolve_invocation("safe-on-uninit-fixture", &["inherit"], None)
        .expect("fixture subcommand resolves");
    assert_eq!(
        inherited.semantics.safe_on_uninit,
        Some(SpecSurface::ALL_TCL)
    );
}

#[test]
fn invocation_facts_own_composed_command_subcommand_and_form_dependencies() {
    let facts = {
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "dispatch-composition-fixture",
            arity: Arity::any(),
            subcommands: DISPATCH_SUBCOMMANDS,
            dispatch_dependencies: Some(DispatchDependencyDescriptor::replace(OBJECT_DEPENDENCY)),
            ..CommandSpec::DEFAULT
        });
        registry
            .resolve_invocation("dispatch-composition-fixture", &["run"], None)
            .expect("fixture command resolves")
            .facts()
    };

    // Command replace(object) is extended by the subcommand's unknown
    // requirement, then the form replaces that refinable portion with
    // object-only. The irreducible live-interpreter base remains.
    assert_eq!(
        facts.dispatch_dependencies,
        DispatchDependencies::BASE.union(OBJECT_DEPENDENCY)
    );
    assert!(
        !facts
            .dispatch_dependencies
            .contains(DispatchDependencyDomain::UnknownHandling)
    );
    assert_eq!(facts.form.as_deref(), Some("guarded-form"));
    assert_eq!(facts.subcommand.canonical_name(), Some("run"));
}

#[test]
fn transition_coverage_replaces_only_the_duplicate_legacy_write() {
    let registry = CommandRegistry::build_default();
    let invocation = registry
        .resolve_invocation(
            "rename",
            &["old", "new"],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .expect("core registry command resolves");
    let footprint = invocation.effects();

    assert!(
        !footprint.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::CommandBindings
                && matches!(
                    access.mode,
                    EffectAccessMode::Write | EffectAccessMode::ReadWrite
                )
        }),
        "the completion-edge transition, not an unconditional legacy write, owns bindings"
    );
    assert!(footprint.accesses().iter().any(|access| {
        access.domain == WorldStateDomain::CommandTraces && access.mode == EffectAccessMode::Read
    }));
    assert!(
        footprint.legacy().command_table_mutation,
        "the command-table mutation the transition states remains visible \
             to the legacy effect bridge"
    );
    assert_eq!(footprint.legacy().side_effects.len(), 1);

    let facts = invocation.facts();
    assert!(
        !facts.transition_effect_coverage.entries().is_empty(),
        "the owned hand-off retains the registry coverage contract"
    );
    assert!(
        facts
            .effects
            .callback()
            .kinds
            .contains(CallbackKinds::TRACE)
    );
}

#[test]
fn transition_coverage_does_not_hide_an_uncovered_variable_value_write() {
    let registry = CommandRegistry::build_default();
    let variable = registry
        .resolve_invocation(
            "variable",
            &["name", "value"],
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        )
        .expect("variable resolves")
        .facts();
    assert!(
        variable.transition_effect_coverage.entries().is_empty(),
        "the alias transition does not claim to model value initialisation"
    );
    assert!(variable.effects.accesses().iter().any(|access| {
        access.domain == WorldStateDomain::VariableStore
            && matches!(
                access.mode,
                EffectAccessMode::Write | EffectAccessMode::ReadWrite
            )
    }));
}
#[test]
fn no_value_append_forms_do_not_donate_mutation_contracts() {
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let registry = crate::model::ingress::static_context_for(environment).commands();
        let profile = crate::model::ingress::resolve_environment(environment).unit_profile();
        let dialect = crate::InvocationDialect::of_profile(profile);
        for command in ["append", "lappend"] {
            let words = InvocationWords::literals(command, &["x"]).with_dialect(dialect);
            let resolved = registry.resolve_structured_invocation(words, dialect.authoring_query());
            let facts = resolved.resolved().expect("selected native form").facts();
            assert_eq!(
                facts.operation,
                SemanticOperationId::Invoke,
                "{environment}: {command}"
            );
            assert!(!facts.traits.contains(Traits::UNCONDITIONAL_VARIABLE_WRITE));
            assert_eq!(facts.return_type, None);
            assert_eq!(facts.byte_array_effect, crate::ByteArrayEffect::None);
            assert_eq!(facts.var_elements_effect, None);
            assert_eq!(
                facts.native_result,
                Some(crate::native_result::NativeResultContract::VariableValue {
                    variable_at: 0,
                    phase: crate::native_result::VariableResultPhase::AfterRead,
                })
            );
            if command == "append" {
                assert_eq!(facts.arg_roles, [(0, ArgRole::VarRead)]);
                assert_eq!(
                    resolved
                        .resolved()
                        .expect("read form")
                        .semantics
                        .safe_on_uninit,
                    None
                );
                assert!(registry.arg_type_hint_words(words, 0).is_none());
            } else {
                assert!(facts.traits.contains(Traits::CONDITIONAL_VARIABLE_WRITE));
                assert_eq!(
                    facts.successful_handler,
                    Some(crate::native_compilation::SuccessfulHandlerSpec::InitialiseEmptyVariable)
                );
                assert_eq!(
                    registry.arg_type_hint_words(words, 0).is_some(),
                    environment != "jim"
                );
            }
            let mutating = registry
                .resolve_invocation(command, &["x", "value"], dialect.authoring_query())
                .expect("mutating native form")
                .facts();
            assert!(
                mutating
                    .traits
                    .contains(Traits::UNCONDITIONAL_VARIABLE_WRITE)
            );
        }
    }
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

#[test]
fn positional_count_preserves_native_reserved_tail_and_case_layout() {
    use crate::InvocationWord::{Dynamic, Literal};
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = crate::InvocationDialect::for_version(version);
        let registry = crate::CommandRegistry::build_default();
        let cases: &[(&str, &[crate::InvocationWord<'_>], u16)] = &[
            ("lsort", &[Literal("-integer"), Dynamic], 1),
            ("lsort", &[Literal("-integer")], 1),
            ("lsearch", &[Literal("-exact"), Dynamic, Dynamic], 2),
            ("lsearch", &[Literal("-exact"), Dynamic], 2),
            (
                "switch",
                &[Literal("-exact"), Literal("--"), Dynamic, Literal("x {}")],
                2,
            ),
        ];
        for &(head, arguments, expected) in cases {
            let words =
                crate::InvocationWords::structured(Literal(head), arguments).with_dialect(dialect);
            let invocation = registry
                .resolve_structured_invocation(words, dialect.authoring_query())
                .resolved()
                .expect("native literal head");
            assert_eq!(
                invocation.argument_count_for_arity(),
                Some(expected),
                "{version:?} {head}"
            );
            assert_eq!(
                invocation.facts().arity_accepts_frozen_arguments(),
                Some(true)
            );
        }
        let arguments = [Literal("-exact"), Literal("-exact {}")];
        let invocation = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal("switch"), &arguments)
                    .with_dialect(dialect),
                dialect.authoring_query(),
            )
            .resolved()
            .expect("native switch");
        assert_eq!(
            invocation.argument_count_for_arity(),
            if version == tcl_dialect::TclVersion::V8_4 {
                None
            } else {
                Some(2)
            }
        );
    }
}
