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

//! Executable values, retained callbacks, control arms, option variable scope and frame scripts in the authored source.

use super::*;

#[test]
fn source_executable_value_absence_keeps_data_role_ambiguity_separate() {
    use super::super::SourceExecutableValueCapability::{Absent, MayContain, Unproven};
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
    use super::super::SourceExecutableValueCapability::MayContain;
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
    use super::super::SourceCallbackRetention::{Absent, MayRetain};
    let registry = crate::CommandRegistry::build_default();
    let cases: &[(&str, &[&str], super::super::SourceCallbackRetention)] = &[
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
    use super::super::SourceCallbackRetention::{MayRetain, Unproven};
    use crate::InvocationWord::{Dynamic, Expanded, Literal};
    let registry = crate::CommandRegistry::build_default();
    let cases: &[(
        &str,
        &[crate::InvocationWord<'_>],
        super::super::SourceCallbackRetention,
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
    use super::super::SourceCallbackRetention::Unproven;
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
