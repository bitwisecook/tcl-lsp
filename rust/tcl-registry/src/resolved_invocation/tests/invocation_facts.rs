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

//! `InvocationFacts`: coercions, result types, argument hints, scalar-math and list-length protocols, purity and dispatch dependencies.

use super::*;

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
        assert!(!facts.is_pure());
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
fn invocation_facts_are_owned_and_target_neutral() {
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "world-effect-fixture",
        arity: Arity::any(),
        subcommands: WORLD_SUBCOMMANDS,
        world_effects: crate::knowledge::Knowledge::Partial(COMMAND_WORLD),
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
        // Closed by the Tcl 9.1 C evidence, and a CSE candidate.
        assert!(facts.is_cse_pure(), "{command}");
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
    // Purity derives from the C evidence of each row at the release.
    let at = tcl_dialect::model::SurfaceQuery::core(tcl_dialect::model::Family::Tcl, "9.1");
    let llength = registry.get("llength").expect("llength");
    assert!(llength.is_pure_at(Some(&at)));
    let string = registry.get("string").expect("string");
    assert!(!string.is_pure_at(Some(&at)));
    assert!(
        string
            .resolve_subcommand("length")
            .is_some_and(|length| length.is_pure_at(string, Some(&at)))
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
    // It reads the clock: `ClockSecondsObjCmd` reaches `Tcl_GetTime` (8.5 to
    // 9.0) or `Tcl_GetDayTime` (9.1), and 8.4's `clock seconds` reaches
    // `TclpGetSeconds`, so no release's evidence closes it (issue #1712).
    assert!(!seconds.is_pure());
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
