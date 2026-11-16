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

//! What the descriptors select: completion descriptors and their precedence, and semantic operations.

use super::*;

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
