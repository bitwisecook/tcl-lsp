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

//! Property rows of a command body: source-configuration references, representation effects, `DEFERS_BODY`, `refine` forms, open surfaces, variable receivers and typed notice subjects.

use super::*;
use tcl_registry::abbrev::PrefixMatching;
use tcl_registry::arity::Arity;
use tcl_registry::definer::MemberVisibility;
use tcl_registry::knowledge::Knowledge;
use tcl_registry::representation::RepresentationEffect;
use tcl_registry::side_effects::SideEffect;
use tcl_registry::side_effects::SideEffectTarget;
use tcl_registry::traits::Traits;

#[test]
fn subcommand_source_configuration_reference_loads_and_omission_abstains() {
    let pack = evaluate_pack(
        "speclib probe 2.0 { command sample { \
             subcommand compare { \
               source_configuration_reference DataGroupComparison \
             }; \
             subcommand plain { arity 0 } } }",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = pack.command("sample").unwrap().spec;
    assert_eq!(
        spec.subcommands[0].source_configuration_reference,
        Some(tcl_registry::SourceConfigurationReferenceLayout::DataGroupComparison)
    );
    assert!(spec.subcommands[1].source_configuration_reference.is_none());
}

#[test]
fn invalid_source_configuration_reference_withdraws_the_descriptor() {
    let pack = evaluate_pack(
        "speclib probe 2.0 { command sample { subcommand compare { \
             source_configuration_reference Invented } } }",
    );
    assert!(
        pack.command("sample").unwrap().spec.subcommands[0]
            .source_configuration_reference
            .is_none()
    );
    assert!(
        pack.notices.iter().any(|notice| {
            notice.message == "unknown source configuration reference `Invented` dropped"
        }),
        "{:?}",
        pack.notices
    );
}

#[test]
fn source_configuration_reference_has_static_and_evaluated_loader_parity() {
    let source = "speclib probe 2.0 { command sample { subcommand compare { \
                      source_configuration_reference DataGroupComparison } } }";
    let fast = evaluate_pack(source);
    let evaluated = evaluate_pack_with(
        source,
        &EvalOptions {
            static_fast_path: false,
            ..EvalOptions::default()
        },
    );
    let templated = evaluate_pack(
        "speclib probe 2.0 { set layout DataGroupComparison; \
             command sample { subcommand compare { \
               source_configuration_reference $layout } } }",
    );
    for pack in [&fast, &evaluated, &templated] {
        assert!(pack.load_error.is_none(), "{:?}", pack.load_error);
        assert!(pack.notices.is_empty(), "{:?}", pack.notices);
        assert_eq!(
            pack.command("sample").unwrap().spec.subcommands[0].source_configuration_reference,
            Some(tcl_registry::SourceConfigurationReferenceLayout::DataGroupComparison)
        );
    }
    assert_eq!(
        format!("{:?}", fast.command("sample").unwrap().spec),
        format!("{:?}", evaluated.command("sample").unwrap().spec)
    );
    assert_eq!(
        format!("{:?}", fast.command("sample").unwrap().spec),
        format!("{:?}", templated.command("sample").unwrap().spec)
    );
}

#[test]
fn original_abstract_definition_grammar_loads_without_changing_factory_visibility() {
    // naming.tcloo.original-abstract-manufacturer-public-visibility
    // docs/design/analysis/name-resolution-proofs/tcloo-original-abstract-manufacturer-public-visibility.md
    let pack = evaluate_pack(
        "speclib probe 1.1 { command makeAbstract { definition_body tcloo-abstract; \
             manufacturer create -names-instance-at 1 -definition-body-at 2 -constructor-args-from 2 } }",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = &pack.command("makeAbstract").unwrap().spec;
    let grammar = spec.definition_body.unwrap();
    for selector in ["create", "new", "createWithNamespace"] {
        assert_eq!(
            grammar.manufacturer(selector).unwrap().visibility,
            MemberVisibility::Unexported
        );
    }
    assert_eq!(
        spec.manufacturer_methods[0].visibility,
        MemberVisibility::Exported
    );
    assert!(spec.native_result.is_none());
}

#[test]
fn expression_representation_coercion_loads_without_a_native_hook() {
    let pack = evaluate_pack(
        "speclib probe 1.1 { command evaluate { \
             representation_effect {CoerceExpressionValues 0} } }",
    );
    assert_eq!(
        pack.command("evaluate").unwrap().spec.representation_effect,
        Some(RepresentationEffect::CoerceExpressionValues { arguments_from: 0 })
    );
}

#[test]
fn numeric_object_conversion_requirements_load_without_a_result_contract() {
    let pack = evaluate_pack(
        "speclib probe 1.1 { command convert { \
             representation_effect {CoerceNumericValues 0} } }",
    );
    let spec = &pack.command("convert").unwrap().spec;
    assert_eq!(
        spec.representation_effect,
        Some(RepresentationEffect::CoerceNumericValues { arguments_from: 0 })
    );
    assert!(spec.native_result.is_none());
}

/// A pack can declare `DEFERS_BODY` for its own definer, and — the half
/// that matters — a pack that says nothing leaves the trait **unset**.
///
/// The flag tells a static walk that an unreadable body word costs it
/// nothing about the call's completion, so the silent
/// default has to be the abstaining one: unset means "this body may run
/// here". Traits resolve by name against the registry's own flag list, so
/// there is no loader table to keep in step — but there is also nothing
/// stopping a future edit from inventing one, which is what this pins.
#[test]
fn a_pack_declares_defers_body_and_omitting_it_stays_abstaining() {
    let declared = evaluate_pack(
        "speclib probe 1.1 { command mydefiner { \
             arity 3; traits {DEFERS_BODY} } }",
    );
    assert!(
        declared
            .command("mydefiner")
            .unwrap()
            .spec
            .traits
            .contains(tcl_registry::Traits::DEFERS_BODY),
        "a pack must be able to declare its own definer dormant: {:?}",
        declared.notices
    );
    assert!(declared.notices.is_empty(), "{:?}", declared.notices);

    let silent = evaluate_pack("speclib probe 1.1 { command mydefiner { arity 3 } }");
    assert!(
        !silent
            .command("mydefiner")
            .unwrap()
            .spec
            .traits
            .contains(tcl_registry::Traits::DEFERS_BODY),
        "an undeclared body must stay material, never dormant-by-default"
    );
}

/// `refine` states one invocation form (Q12/D2): the words are the
/// owning scope's own, and every overlay it omits inherits.
#[test]
fn a_refine_block_states_one_invocation_form() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n command demo {\n arity 0..2\n \
             traits {CSE_CANDIDATE MUTATES_ARGUMENT}\n \
             refine query {\n arity 0\n traits {CSE_CANDIDATE}\n mutator no\n \
             side_effects none\n }\n \
             refine set {\n arity 2\n selector {put}\n mutator yes\n \
             side_effect InterpState -writes\n }\n }\n}",
    );
    let spec = pack.command("demo").expect("demo loads").spec;
    let [query, set] = spec.command_forms else {
        panic!("two forms, got {:?}", spec.command_forms);
    };
    assert_eq!(query.name, "query");
    assert_eq!(query.arity, Arity::exact(0));
    assert_eq!(query.traits, Some(Traits::CSE_CANDIDATE));
    assert_eq!(query.mutator, Some(false));
    assert_eq!(
        query.side_effects,
        Some(Knowledge::Partial(&[] as &[SideEffect]))
    );
    assert_eq!(set.name, "set");
    assert_eq!(set.arity, Arity::exact(2));
    // An unmentioned overlay is inherited, not blanked.
    assert_eq!(set.traits, None);
    assert_eq!(set.mutator, Some(true));
    let selector = set.literal_argument_prefix.expect("a selector");
    assert_eq!(selector.words, ["put"]);
    assert_eq!(selector.prefix_matching, PrefixMatching::Enabled);
    let [effect] = set.side_effects.expect("declared effects").lower_bound() else {
        panic!("one effect");
    };
    assert_eq!(effect.target, SideEffectTarget::InterpState);
    assert!(effect.writes);
}

/// A subcommand refines its own forms, through the same reader — which is
/// the shape Tk's widget methods need: one query arity and one mutation
/// arity under a single method word.
#[test]
fn a_subcommand_refines_its_own_forms() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n command widget {\n arity 1..\n \
             subcommand cget {\n arity 0..1\n \
             refine query {\n arity 0\n traits {CSE_CANDIDATE}\n mutator no\n }\n \
             refine set {\n arity 1\n mutator yes\n }\n }\n }\n}",
    );
    let spec = pack.command("widget").expect("widget loads").spec;
    let sub = spec.subcommand("cget").expect("the cget subcommand");
    assert_eq!(
        sub.subcommand_forms
            .iter()
            .map(|form| form.name)
            .collect::<Vec<_>>(),
        ["query", "set"]
    );
    assert_eq!(sub.subcommand_forms[1].mutator, Some(true));
}

/// A `refine` body takes only the fields a form carries: a fact that
/// belongs on the parent is an unknown property here, not a silent loss.
#[test]
fn a_refine_block_refuses_a_fact_the_form_cannot_hold() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n command demo {\n arity 1\n \
             refine only {\n arity 1\n return_type List\n }\n }\n}",
    );
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("return_type")),
        "{:?}",
        pack.notices
    );
}

/// `dynamic_surface` / `unknown_members` set the open-surface fact on a
/// command, and the object-class flags set it on a class.
#[test]
fn dynamic_surface_opens_a_commands_member_set() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n command demo {\n arity 1\n \
             dynamic_surface\n }\n command duo {\n arity 1\n \
             unknown_members\n }\n command trio { arity 1 }\n \
             command quad {\n arity 1\n \
             object_class ::probe::tree -dynamic-surface {\n \
             method walk { arity 0 }\n }\n }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    assert!(
        pack.command("demo")
            .expect("demo")
            .spec
            .allow_unknown_subcommands
    );
    assert!(
        pack.command("duo")
            .expect("duo")
            .spec
            .allow_unknown_subcommands
    );
    assert!(
        !pack
            .command("trio")
            .expect("trio")
            .spec
            .allow_unknown_subcommands,
        "saying nothing keeps the closed default"
    );
}

#[test]
// Implementation contract: naming.variable.registry-receiver-authoring-parity
// docs/design/analysis/name-resolution-proofs/registry-variable-receiver-authoring-parity.md
fn combined_variable_receiver_positions_preserve_scope_and_explicit_withdrawal() {
    use tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined;
    let pack = evaluate_pack(
        "speclib probe 2.0 {command demo {arity 1; arg 0 -role VarWrite; variable_receivers {0}; refine read {arity 1; variable_receivers {}}; subcommand nested {arity 1; arg 0 -role VarRead; variable_receivers {0}; refine ordinary {arity 1; variable_receivers inherit}}}}",
    );
    let spec = &pack.command("demo").expect("pure naming metadata").spec;
    assert_eq!(spec.variable_receivers, Some(&[(0, Combined)][..]));
    assert_eq!(spec.command_forms[0].variable_receivers, Some(&[][..]));
    assert_eq!(
        spec.subcommands[0].variable_receivers,
        Some(&[(0, Combined)][..])
    );
    assert_eq!(
        spec.subcommands[0].subcommand_forms[0].variable_receivers,
        None
    );
    assert!(spec.successful_handler.is_none());
    assert!(spec.native_compilation.is_none());
    for bad in ["0 0", "256", "RootOnly", "-1"] {
        let rejected = evaluate_pack(&format!(
            "speclib probe 2.0 {{command demo {{variable_receivers {{{bad}}}}}; command neighbour {{arity 0}}}}"
        ));
        assert!(rejected.command("demo").is_none(), "{bad}");
        assert!(rejected.command("neighbour").is_some());
    }
}

#[test]
fn original_loader_notices_retain_their_typed_property_and_flag_subjects() {
    // Implementation contract: naming.consumer.typed-spec-pack-notice-actions
    // docs/design/analysis/name-resolution-proofs/typed-spec-pack-notice-actions.md
    let source = "speclib demo 1 {\n command demo::x {\n arty 1; arg 0 -rle Body\n }\n}\n";
    let pack = evaluate_pack(source);
    let subjects: Vec<_> = pack
        .notices
        .iter()
        .filter_map(|notice| notice.subject.as_ref())
        .collect();
    assert!(subjects.iter().any(|subject| subject.word() == "arty"
        && matches!(
            subject.kind(),
            tcl_lsp_core::code_actions::SpecPackNoticeKind::Property
        )));
    assert!(subjects.iter().any(|subject|subject.word()=="-rle" && matches!(subject.kind(),tcl_lsp_core::code_actions::SpecPackNoticeKind::Flag{row} if row=="arg")));
    assert!(
        subjects
            .iter()
            .all(|subject| subject.image().bytes() == source.as_bytes())
    );
}

#[test]
// Implementation contract: naming.variable.trace-source-receiver-purpose
// docs/design/analysis/name-resolution-proofs/trace-source-receiver-purpose.md
fn trace_subject_receiver_metadata_preserves_form_and_rejects_ambiguous_fields() {
    use tcl_registry::resolved_invocation::VariableReceiverOperandForm::{Combined, TraceSubject};
    let pack = evaluate_pack(
        "speclib probe 2.0 {command demo {arity 2; arg 0 -role VarRead; arg 1 -role VarRead; variable_receivers {0 {1 trace-subject}}; refine nested {arity 2; variable_receivers {{0 trace-subject}}}; subcommand query {arity 1; arg 0 -role VarRead; variable_receivers {{0 trace-subject}}}}}",
    );
    let spec = &pack.command("demo").unwrap().spec;
    assert_eq!(
        spec.variable_receivers,
        Some(&[(0, Combined), (1, TraceSubject)][..])
    );
    assert_eq!(
        spec.command_forms[0].variable_receivers,
        Some(&[(0, TraceSubject)][..])
    );
    assert_eq!(
        spec.subcommands[0].variable_receivers,
        Some(&[(0, TraceSubject)][..])
    );
    assert!(spec.successful_handler.is_none());
    assert!(spec.native_compilation.is_none());
    for bad in [
        "{0 unsupported}",
        "{0 trace-subject} 0",
        "{0 trace-subject extra}",
        "{256 trace-subject}",
    ] {
        let pack = evaluate_pack(&format!(
            "speclib probe 2.0 {{command demo {{variable_receivers {{{bad}}}}}; command neighbour {{arity 0}}}}"
        ));
        assert!(pack.command("demo").is_none(), "{bad}");
        assert!(pack.command("neighbour").is_some());
    }
}
