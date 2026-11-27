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

//! World-effect composition, state-transition facts, alias transitions and transition coverage.

use super::*;

#[test]
fn world_effect_descriptors_compose_by_default_and_can_explicitly_refine() {
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "world-effect-fixture",
        arity: Arity::any(),
        subcommands: WORLD_SUBCOMMANDS,
        world_effects: crate::knowledge::Knowledge::Partial(COMMAND_WORLD),
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
        world_effects: crate::knowledge::Knowledge::Partial(COMMAND_WORLD),
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
        let dialect = tcl_native_model::InvocationDialect::of_profile(profile);
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
