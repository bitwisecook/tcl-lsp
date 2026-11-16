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
fn logical_alias_source_layout_keeps_original_operands_without_native_transition_grants() {
    // experiments/name-handling/results/logical-frame-alias-projection-20261014/oracles/observations.json
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = crate::model::ingress::context_for_profile(profile);
    for arguments in [
        vec!["#0", "g", "local"],
        vec!["1", "$literal", ""],
        vec!["g", "local"],
    ] {
        let arguments = arguments
            .iter()
            .map(|value| crate::InvocationWord::Literal(value))
            .collect::<Vec<_>>();
        let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            crate::InvocationWords::structured(crate::InvocationWord::Literal("upvar"), &arguments)
                .with_profile(Some(profile)),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        );
        let schema = selected.resolved().unwrap();
        let ordinary = schema.state_transitions();
        let aliases = schema.authored_logical_source_variable_aliases().unwrap();
        assert_eq!(aliases.len(), 1);
        let offset = usize::from(arguments.len() % 2 == 1);
        assert_eq!(aliases[0].local.literal(), arguments[offset + 1].literal());
        assert_eq!(aliases[0].words.local, offset + 1);
        assert_eq!(aliases[0].words.target, offset);
        assert!(!aliases[0].writes_value);
        assert_eq!(schema.state_transitions(), ordinary);
    }
    for arguments in [
        vec![
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Literal("g"),
            crate::InvocationWord::Literal("local"),
        ],
        vec![
            crate::InvocationWord::Literal("010"),
            crate::InvocationWord::Literal("g"),
            crate::InvocationWord::Literal("local"),
        ],
        vec![crate::InvocationWord::Expanded],
    ] {
        let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            crate::InvocationWords::structured(crate::InvocationWord::Literal("upvar"), &arguments)
                .with_profile(Some(profile)),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        );
        assert!(
            selected
                .resolved()
                .unwrap()
                .authored_logical_source_variable_aliases()
                .is_none()
        );
    }
}
#[test]
fn conditional_no_level_alias_card_keeps_generic_layout_and_target_debt() {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = crate::model::ingress::context_for_profile(profile);
    for arguments in [
        vec![
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Literal("local"),
        ],
        vec![
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Literal("target"),
            crate::InvocationWord::Literal("local"),
        ],
        vec![
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Dynamic,
        ],
    ] {
        let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            crate::InvocationWords::structured(crate::InvocationWord::Literal("upvar"), &arguments)
                .with_profile(Some(profile)),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        );
        let schema = selected.resolved().unwrap();
        let ordinary = schema.state_transitions();
        assert!(schema.authored_logical_source_state_transitions().is_none());
        assert!(schema.authored_logical_source_variable_aliases().is_none());
        let card = schema.conditional_no_level_source_alias_layout();
        if arguments.len() == 2 && arguments[1].literal() == Some("local") {
            let card = card.expect("actual typed registered conditional no-level producer");
            assert!(card.may_argument_error());
            assert_eq!(card.pairs().len(), 1);
            assert!(matches!(
                card.pairs()[0].0,
                crate::TransitionSubject::Unknown {
                    argument_index: 0,
                    ..
                }
            ));
            assert_eq!(card.pairs()[0].1.literal(), Some("local"));
            assert!(
                card.residual()
                    .facts()
                    .iter()
                    .any(|fact| matches!(fact.transition, crate::StateTransition::Widen(_)))
            );
            let mut replaced = schema;
            replaced.semantics.state_transitions.form = Some(crate::StateTransitionDescriptor {
                composition: crate::StateTransitionComposition::Replace,
                ..crate::StateTransitionDescriptor::EMPTY
            });
            assert!(
                replaced
                    .conditional_no_level_source_alias_layout()
                    .is_none()
            );
            let mut extended = schema;
            extended.semantics.state_transitions.form = Some(crate::StateTransitionDescriptor {
                composition: crate::StateTransitionComposition::Extend,
                ..crate::StateTransitionDescriptor::EMPTY
            });
            assert!(
                extended
                    .conditional_no_level_source_alias_layout()
                    .is_some()
            );
            let mut custom = schema;
            let mut descriptor = custom.semantics.state_transitions.command.unwrap();
            descriptor.logical_source_resolver = Some(
                crate::state_transition::LogicalSourceStateTransitionResolver::FrameAliases(
                    crate::state_transition::logical_source_alias_pairs_resolver(
                        crate::frame_effect::FrameLevelWord::Upvar,
                    ),
                ),
            );
            custom.semantics.state_transitions.command = Some(descriptor);
            assert!(custom.conditional_no_level_source_alias_layout().is_none());
        } else {
            assert!(card.is_none());
        }
        assert_eq!(schema.state_transitions(), ordinary);
    }
}

#[test]
fn logical_alias_destination_applicability_retains_source_purpose_and_unknown_frames() {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = crate::model::ingress::context_for_profile(profile);
    let dialect = crate::InvocationDialect::of_profile(profile);
    assert_eq!(dialect.variable_lookup_policy, None);
    let arguments = [crate::InvocationWord::Dynamic];
    let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
        context.commands(),
        Some(context.context()),
        crate::InvocationWords::structured(crate::InvocationWord::Literal("global"), &arguments)
            .with_dialect(dialect),
        tcl_dialect::model::InvocationRealm::RuleLoader,
    );
    let schema = selected.resolved().unwrap();
    let original = schema.state_transitions();
    let aliases = schema.authored_logical_source_variable_aliases().unwrap();
    let [alias] = aliases.as_slice() else {
        panic!("actual selected source alias");
    };
    assert_eq!(
        alias.destination,
        crate::VariableAliasDestination::ProcedureLocal
    );
    assert_eq!(
        alias
            .destination
            .is_active_in_frame(crate::VariableAliasFrame::Namespace, dialect),
        None
    );
    for (frame, expected) in [
        (crate::VariableAliasFrame::Global, Some(false)),
        (crate::VariableAliasFrame::Namespace, Some(false)),
        (crate::VariableAliasFrame::Procedure, Some(true)),
        (crate::VariableAliasFrame::Unknown, None),
    ] {
        assert_eq!(
            schema.authored_logical_source_alias_destination_is_active_in_frame(
                alias.destination,
                frame,
            ),
            expected
        );
    }
    assert_eq!(
        schema.authored_logical_source_alias_destination_is_active_in_frame(
            crate::VariableAliasDestination::CurrentNamespaceOrLocal,
            crate::VariableAliasFrame::Namespace,
        ),
        None,
        "a different destination has no selected descriptor receipt"
    );
    assert_eq!(schema.state_transitions(), original);
    assert_eq!(dialect.variable_lookup_policy, None);

    let native = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    let selected_native = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
        context.commands(),
        Some(context.context()),
        crate::InvocationWords::structured(crate::InvocationWord::Literal("global"), &arguments)
            .with_dialect(native),
        tcl_dialect::model::InvocationRealm::RuleLoader,
    );
    assert_eq!(
        selected_native
            .resolved()
            .unwrap()
            .authored_logical_source_alias_destination_is_active_in_frame(
                alias.destination,
                crate::VariableAliasFrame::Namespace,
            ),
        None,
        "a Native dialect cannot enter the Logical-only facade"
    );
}

#[test]
fn dictionary_source_phase_keeps_generic_unknown_and_additional_effects() {
    // An unknown callback and a variable write: the callback alone
    // carries no access for the additional effects to show.
    fn extra(arguments: crate::InvocationArguments<'_>) -> crate::world_effect::EffectFootprint {
        let mut footprint = crate::world_effect::EffectFootprint::conservative_unknown_invocation();
        footprint.extend(
            crate::world_effect::WorldEffectDescriptor::VARIABLE_READ_MODIFY_WRITE
                .resolve(arguments),
        );
        footprint
    }
    // The lenient profile spans 8.4, which has no `dict`.
    let ingress = crate::model::ingress::resolve_environment("tcl8.6");
    let context = ingress.default_context_registry();
    let profile = ingress.unit_profile();
    let arguments = [
        crate::InvocationWord::Literal("with"),
        crate::InvocationWord::Literal("$dict"),
        crate::InvocationWord::Literal(""),
    ];
    let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
        context.commands(),
        Some(context.context()),
        crate::InvocationWords::structured(crate::InvocationWord::Literal("dict"), &arguments)
            .with_profile(Some(profile)),
        tcl_dialect::model::InvocationRealm::RuleLoader,
    );
    let mut schema = selected.resolved().unwrap();
    assert!(schema.facts().effects.callback().kinds.is_unknown());
    let phase = schema.authored_dictionary_scope_source_phase().unwrap();
    assert!(
        phase
            .additional_effects()
            .accesses()
            .iter()
            .all(|access| access.mode == crate::world_effect::EffectAccessMode::Read)
    );
    assert_eq!(phase.plan().dictionary_argument, 1);
    assert_eq!(phase.plan().body_argument, 2);
    assert_eq!(
        phase.effects().callback().kinds,
        crate::world_effect::CallbackKinds::TRACE
    );
    assert!(schema.facts().effects.callback().kinds.is_unknown());
    schema.semantics.world_effects.form = Some(crate::world_effect::WorldEffectDescriptor {
        resolver: Some(extra),
        ..crate::world_effect::WorldEffectDescriptor::EMPTY
    });
    let expanded = schema.authored_dictionary_scope_source_phase().unwrap();
    assert!(expanded.effects().callback().kinds.is_unknown());
    assert!(
        expanded
            .additional_effects()
            .accesses()
            .iter()
            .any(|access| access.mode != crate::world_effect::EffectAccessMode::Read)
    );
    schema.semantics.body_execution = None;
    assert!(schema.authored_dictionary_scope_source_phase().is_none());
}

#[test]
fn dictionary_source_phase_rejects_wrong_arity_and_scripted_helpers() {
    for dialect in ["tcl8.6", "jim"] {
        let ingress = crate::model::ingress::resolve_environment(dialect);
        let context = ingress.default_context_registry();
        let profile = ingress.unit_profile();
        let arguments = [
            crate::InvocationWord::Literal("with"),
            crate::InvocationWord::Literal("d"),
            crate::InvocationWord::Literal(""),
        ];
        let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            crate::InvocationWords::structured(crate::InvocationWord::Literal("dict"), &arguments)
                .with_profile(Some(profile)),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        );
        let mut schema = selected.resolved().unwrap();
        assert!(
            schema.authored_dictionary_scope_source_phase().is_some(),
            "{dialect}"
        );
        assert!(schema.facts().effects.callback().kinds.is_unknown());
        schema.semantics.arity = crate::Arity::exact(0);
        assert!(
            schema.authored_dictionary_scope_source_phase().is_none(),
            "{dialect}"
        );
        assert!(schema.facts().effects.callback().kinds.is_unknown());
    }
    let ingress = crate::model::ingress::resolve_environment("jim");
    let context = ingress.default_context_registry();
    let arguments = [
        crate::InvocationWord::Literal("update"),
        crate::InvocationWord::Literal("d"),
        crate::InvocationWord::Literal("key"),
        crate::InvocationWord::Literal("mapped"),
        crate::InvocationWord::Literal(""),
    ];
    let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
        context.commands(),
        Some(context.context()),
        crate::InvocationWords::structured(crate::InvocationWord::Literal("dict"), &arguments)
            .with_profile(Some(ingress.unit_profile())),
        tcl_dialect::model::InvocationRealm::RuleLoader,
    );
    let schema = selected.resolved().unwrap();
    assert_eq!(schema.facts().arity_accepts_frozen_arguments(), Some(true));
    assert!(schema.authored_dictionary_scope_source_phase().is_none());
    assert!(schema.facts().effects.callback().kinds.is_unknown());
}

#[test]
fn source_descriptor_replacement_withdraws_parent_layout_requirement() {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = crate::model::ingress::context_for_profile(profile);
    let arguments = [
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Literal("source"),
        crate::InvocationWord::Literal("local"),
    ];
    let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
        context.commands(),
        Some(context.context()),
        crate::InvocationWords::structured(crate::InvocationWord::Literal("upvar"), &arguments)
            .with_profile(Some(profile)),
        tcl_dialect::model::InvocationRealm::RuleLoader,
    );
    let mut schema = selected.resolved().unwrap();
    assert!(schema.authored_logical_source_state_transitions().is_none());
    schema.semantics.state_transitions.form = Some(crate::StateTransitionDescriptor {
        composition: crate::StateTransitionComposition::Replace,
        ..crate::StateTransitionDescriptor::EMPTY
    });
    assert_eq!(
        schema
            .authored_logical_source_state_transitions()
            .unwrap()
            .facts()
            .len(),
        0
    );
    assert_eq!(schema.state_transitions().facts().len(), 0);
}
#[test]
fn conditional_list_ordinals_do_not_borrow_ascii_or_runtime_authority() {
    use crate::InvocationWord;
    let context = crate::model::ingress::static_context_for("tcl");
    let arguments = [InvocationWord::Dynamic];
    let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
        context.commands(),
        Some(context.context()),
        crate::InvocationWords::structured(InvocationWord::Literal("list"), &arguments),
        tcl_dialect::model::InvocationRealm::RuleLoader,
    )
    .resolved()
    .unwrap();
    assert_eq!(selected.authored_source_list_arguments(), Some(0..1));
    assert_eq!(selected.authored_source_ascii_list_result(), None);
    let environment = crate::model::ingress::resolve_known_environment("jim").unwrap();
    let context = crate::model::ingress::static_context_for("jim");
    let arguments = [InvocationWord::Literal("literal")];
    let selected = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
        context.commands(),
        Some(context.context()),
        crate::InvocationWords::structured(InvocationWord::Literal("list"), &arguments)
            .with_dialect(crate::InvocationDialect::of_profile(
                environment.unit_profile(),
            )),
        tcl_dialect::model::InvocationRealm::RuleLoader,
    )
    .resolved()
    .unwrap();
    assert_eq!(selected.authored_source_list_arguments(), Some(0..1));
    assert_eq!(selected.authored_source_ascii_list_result(), None);
}
