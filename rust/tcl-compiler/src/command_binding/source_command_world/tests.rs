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

use super::*;
use crate::signature_scan::scope::SignatureNamespaceChild;
use tcl_core_types::{ByteNamespacePath, NameBytes};

fn analyse(source: &str, version: tcl_dialect::TclVersion) -> SourceCommandBindings {
    analyse_with_compilation(
        source,
        version,
        crate::environment_ingress::authoring_native_compilation(),
    )
}

fn analyse_with_compilation(
    source: &str,
    version: tcl_dialect::TclVersion,
    compilation: tcl_registry::native_compilation::NativeCompilationContext,
) -> SourceCommandBindings {
    let dialect = tcl_native_model::InvocationDialect::for_version(version);
    let registry = tcl_registry::model::ingress::static_context_for(match version {
        tcl_dialect::TclVersion::V8_4 => "tcl8.4",
        tcl_dialect::TclVersion::V8_5 => "tcl8.5",
        tcl_dialect::TclVersion::V8_6 => "tcl8.6",
        tcl_dialect::TclVersion::V9_0 => "tcl9.0",
        tcl_dialect::TclVersion::V9_1 => "tcl9.1",
    })
    .commands();
    SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        registry,
        super::super::SourceAnalysisOptions {
            invocation_dialect: Some(dialect),
            native_compilation: compilation,
            ..super::super::SourceAnalysisOptions::default()
        },
    )
}

fn slot(namespace: &[&[u8]], simple: &[u8]) -> ByteCommandSlot {
    ByteCommandSlot {
        namespace: ByteNamespacePath::from_segments(namespace.iter().copied()),
        simple: NameBytes::from(simple),
    }
}

#[test]
fn evaluated_method_and_publication_operands_refuse_lexical_variable_roots() {
    let bindings = analyse(
        "oo::class create C {method ping {} {}}",
        tcl_dialect::TclVersion::V8_6,
    );
    let world = bindings.original_completed_command_world().unwrap();
    let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
    let publication = world.declaration_at(&slot(&[], b"C"), policy).unwrap();
    let roster = world
        .class_instance_method_roster(publication.definition().unwrap())
        .unwrap();
    let entry = roster.entries.values().next().unwrap();
    let input = entry.original_name_input();
    let authored_word = input.original_word_key().unwrap().original_word();
    let value = SignatureSourceNameInput::OriginalValue(
        crate::signature_scan::name_value::SignatureSourceNameValue::from_original_static_word(
            authored_word,
            tcl_syntax::word_rules::WordValueRules::from_config(&authored_word.config()),
            policy,
        )
        .unwrap(),
    );
    let root_scope = SignatureNamespaceScope::root(Some(policy));
    for operand in [input, &value] {
        assert_eq!(roster.source_method_for_input(operand), Some(entry));
        assert_eq!(
            publication_slot(
                &root_scope,
                operand,
                CommandBindingDefinitionKind::Procedure
            ),
            Some(slot(&[], b"ping")),
        );
        assert!(root_scope.child_from_input(operand).is_some());
    }
    let plan = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document("$ping"),
        tcl_lexer::Span::new(0, 5),
        authored_word.config(),
    )
    .unwrap();
    let variable = &plan.commands[0].words[0];
    let root =
        crate::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_word(
            variable,
            variable.span(),
            tcl_syntax::word_rules::WordValueRules::from_config(&variable.config()),
            policy,
        )
        .unwrap();
    let root = SignatureSourceNameInput::OriginalVariableRoot(root);
    assert_eq!(root.bytes(), input.bytes());
    assert_eq!(root.policy(), input.policy());
    assert!(roster.source_method_for_input(&root).is_none());
    assert!(
        super::super::SourceReceiverMethodEntry::for_original_input(
            &roster.entries,
            super::super::SourceMethodReceiver::Instance,
            &root,
        )
        .is_none()
    );
    assert!(
        publication_slot(&root_scope, &root, CommandBindingDefinitionKind::Procedure,).is_none()
    );
    assert!(root_scope.child_from_input(&root).is_none());
    let mut transfer = (*world.state.original_command_world).clone();
    let before = transfer.clone();
    assert!(
        transfer
            .import(&root_scope, false, std::slice::from_ref(&root))
            .is_none()
    );
    assert_eq!(transfer, before);
    assert!(
        transfer
            .forget(&root_scope, std::slice::from_ref(&root))
            .is_none()
    );
    assert_eq!(transfer, before);
}

#[test]
fn original_opaque_procedure_completion_and_body_keep_distinct_allocations() {
    // Implementation contract: naming.command.original-completed-cell-occupancy
    // docs/design/analysis/name-resolution-proofs/command-original-completed-cell-occupancy.md
    let source = r"proc P\uD800 {} {}; proc P\uD801 {} {}; P\uD800";
    for version in tcl_dialect::TclVersion::ALL {
        let bindings = analyse(source, version);
        let world = bindings
            .original_completed_command_world()
            .unwrap_or_else(|| {
                panic!(
                    "{}: opaque installed procedure and body",
                    version.dialect_profile_name()
                )
            });
        let procedures = world
            .declarations()
            .filter(|entry| entry.kind() == OriginalCommandPublicationKind::Procedure)
            .collect::<Vec<_>>();
        assert_eq!(procedures.len(), 2, "{}", version.dialect_profile_name());
        assert_ne!(procedures[0].slot(), procedures[1].slot());
        let point = bindings.invocation_at_source(
            "",
            u32::try_from(source.rfind(r"P\uD800").unwrap()).unwrap(),
        );
        let reference = point
            .original_evaluated_command_reference()
            .unwrap_or_else(|| {
                panic!("{}: opaque call allocation", version.dialect_profile_name())
            });
        assert_eq!(reference.original_slot(), Some(procedures[0].slot()));
        assert!(
            analyse(r"proc P\uD800 {{arg default extra}} {}", version)
                .original_completed_command_world()
                .is_none()
        );
    }
}

#[test]
fn original_class_instance_roster_includes_builtin_and_distinct_source_names() {
    // Implementation contract: naming.tcloo.original-source-instance-method-roster
    // docs/design/analysis/name-resolution-proofs/tcloo-original-source-instance-method-roster.md
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let source = r"oo::class create C {method m\uD800 {} {return OLD}; method m\uD800 {} {}}";
        let bindings = analyse(source, version);
        let world = bindings
            .original_completed_command_world()
            .expect("closed source factory");
        let publication = world
            .declaration_at(&slot(&[], b"C"), NamePolicyProtocol::authored_tcl(version))
            .unwrap();
        let class = publication.definition().expect("genuine class declaration");
        let roster = world
            .class_instance_method_roster(class)
            .expect("closed own source inventory");
        assert_eq!(roster.class_definition(), class);
        assert_eq!(roster.publication(), publication);
        assert!(
            roster.contains(b"destroy"),
            "selected builtin blocks a new member"
        );
        assert!(roster.contains(b"m\xed\xa0\x80"));
        assert!(!roster.contains(b"m\xed\xa0\x81"));
        assert!(roster.default_exported(b"m\xed\xa0\x80"));
        assert!(!roster.default_exported(b"Upper"));
        assert!(!roster.default_exported(b"\xed\xa0\x80"));
        let entry = roster
            .entries
            .values()
            .next()
            .expect("current own source entry");
        assert_eq!(
            roster.source_method_for_input(entry.original_name_input()),
            Some(entry),
        );
        assert_eq!(
            entry.declaration().offset,
            u32::try_from(source.rfind("method").unwrap()).unwrap(),
            "latest declaration owns the surviving name",
        );
        assert!(roster.names().any(|name| name.as_bytes() == b"destroy"));
        let foreign = analyse("oo::class create C {}", version)
            .original_completed_command_world()
            .unwrap();
        assert!(foreign.class_instance_method_roster(class).is_none());
        let mut unknown = world.clone();
        let state = Arc::make_mut(&mut unknown.state);
        let token = roster.class_target().identity.as_ref().unwrap();
        Arc::make_mut(&mut state.class_definitions)
            .get_mut(token)
            .unwrap()
            .instance_methods = None;
        assert!(unknown.class_instance_method_roster(class).is_none());
    }
}

#[test]
fn original_completed_occupancy_keeps_baseline_source_and_unknown_holders_separate() {
    // Implementation contract: naming.command.original-completed-cell-occupancy
    // docs/design/analysis/name-resolution-proofs/command-original-completed-cell-occupancy.md
    for version in tcl_dialect::TclVersion::ALL {
        let policy = NamePolicyProtocol::authored_tcl(version);
        let world = analyse("", version)
            .original_completed_command_world()
            .unwrap();
        assert_eq!(
            world.command_slot_occupied(&slot(&[], b"proc"), policy),
            Some(true)
        );
        assert_eq!(
            world.command_slot_occupied(&slot(&[], b"missing"), policy),
            Some(false)
        );
        assert_eq!(
            world.command_slot_occupied(&slot(&[b"Missing"], b"p"), policy),
            None
        );
        let other = if version == tcl_dialect::TclVersion::V8_4 {
            tcl_dialect::TclVersion::V8_5
        } else {
            tcl_dialect::TclVersion::V8_4
        };
        assert_eq!(
            world.command_slot_occupied(
                &slot(&[], b"proc"),
                NamePolicyProtocol::authored_tcl(other)
            ),
            None
        );
        let world = analyse(r"proc P\uD800 {} {}; rename P\uD800 Q\uD801", version)
            .original_completed_command_world()
            .unwrap_or_else(|| panic!("{}: opaque procedure move", version.dialect_profile_name()));
        assert_eq!(
            world.command_slot_occupied(&slot(&[], b"P\xed\xa0\x80"), policy),
            Some(false)
        );
        assert_eq!(
            world.command_slot_occupied(&slot(&[], b"Q\xed\xa0\x81"), policy),
            Some(true)
        );
        assert!(
            analyse("proc P {} {}; eval $unknown", version)
                .original_completed_command_world()
                .is_none()
        );
    }
}

#[test]
fn original_opaque_jim_callable_name_does_not_gate_ascii_static_capture() {
    // Implementation contract: naming.procedure.original-jim-static-list-capture-inputs
    // docs/design/analysis/name-resolution-proofs/procedure-original-jim-static-list-capture-inputs.md
    let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
    let point = tcl_dialect::model::DialectPoint::of_dialect_name(Some("jim")).unwrap();
    let dialect = tcl_native_model::InvocationDialect::of_point(point);
    let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
    let analyse = |source: &str| {
        super::super::SourceCommandBindings::analyse_with_options(
            source,
            config,
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                ..Default::default()
            },
        )
    };
    let bindings = analyse(r"proc P\uD800 {} {{counter 1}} {}; proc P\uD801 {} {{counter 2}} {}");
    let world = bindings
        .original_completed_command_world()
        .expect("exact installed callable allocation owns statics independent of reporting name");
    let declarations = world
        .declarations()
        .filter(|declaration| declaration.kind() == OriginalCommandPublicationKind::Procedure)
        .collect::<Vec<_>>();
    assert_eq!(declarations.len(), 2);
    assert_ne!(declarations[0].slot(), declarations[1].slot());
    assert!(
        declarations
            .iter()
            .all(|declaration| declaration.definition().is_some())
    );
    assert!(
        analyse(r"proc P\uD800 {} {counter &counter} {}")
            .original_completed_command_world()
            .is_none()
    );
}

#[test]
fn publication_snapshots_share_original_rows_and_detach_actual_moves_and_replacements() {
    // naming.command.original-publication-row-sharing
    // docs/design/analysis/name-resolution-proofs/original-publication-row-sharing.md
    // Actual source ledger snapshots only, with no external evaluation,
    // Native freshness, selected handler or source-edit claim.
    let source = "proc alpha {} {}; proc beta {} {}; rename alpha gamma; rename gamma beta";
    for version in tcl_dialect::TclVersion::ALL {
        let bindings = analyse(source, version);
        let first_site = u32::try_from(source.find("rename alpha").unwrap()).unwrap();
        let second_site = u32::try_from(source.find("rename gamma").unwrap()).unwrap();
        let first = &bindings
            .dispatch_points_at(first_site)
            .next()
            .unwrap()
            .state
            .original_command_world;
        let second = &bindings
            .dispatch_points_at(second_site)
            .next()
            .unwrap()
            .state
            .original_command_world;
        let complete = bindings
            .original_completed_command_world()
            .expect("completed original rename sequence");
        let final_world = &complete.state.original_command_world;
        let row = |world: &OriginalSourceCommandWorld, name: &[u8]| {
            world
                .publications
                .iter()
                .find(|entry| entry.slot.simple.as_bytes() == name)
                .cloned()
                .expect("actual publication")
        };
        let alpha = row(first, b"alpha");
        let beta = row(first, b"beta");
        let gamma = row(second, b"gamma");
        assert!(
            Arc::ptr_eq(&beta, &row(second, b"beta")),
            "an unchanged row retains its actual producer"
        );
        assert!(
            !Arc::ptr_eq(&alpha, &gamma),
            "a moved publication detaches its own row"
        );
        assert_eq!(alpha.name_input().bytes(), b"alpha");
        assert_eq!(gamma.name_input().bytes(), b"gamma");
        assert_eq!(alpha.declaration_site(), gamma.declaration_site());
        assert_eq!(alpha.definition(), gamma.definition());
        let replaced = row(final_world, b"beta");
        assert!(!Arc::ptr_eq(&beta, &replaced));
        assert!(!Arc::ptr_eq(&gamma, &replaced));
        assert_eq!(replaced.declaration_site(), alpha.declaration_site());
        assert_eq!(replaced.name_input().bytes(), b"beta");
        assert_eq!(beta.name_input().bytes(), b"beta");
        assert_ne!(beta.declaration_site(), replaced.declaration_site());
        assert_eq!(first.publications.len(), 2);
        assert_eq!(second.publications.len(), 2);
        assert_eq!(final_world.publications.len(), 1);
        let branch = first.as_ref().clone();
        assert_eq!(&branch, first.as_ref());
        for (original, shared) in first.publications.iter().zip(&branch.publications) {
            assert!(Arc::ptr_eq(original, shared));
        }
        let mut changed = branch;
        let altered = Arc::make_mut(&mut changed.publications[0]);
        altered.definition = None;
        assert_eq!(alpha.name_input().bytes(), b"alpha");
        assert!(
            alpha.definition().is_some(),
            "changing a private branch cannot erase the original definition"
        );
        assert_ne!(&changed, first.as_ref());
    }
}

#[test]
fn publication_row_sharing_preserves_complete_semantic_equality_hash_and_order() {
    // naming.command.original-publication-row-sharing
    // docs/design/analysis/name-resolution-proofs/original-publication-row-sharing.md
    use std::hash::{Hash, Hasher};
    let hash = |world: &OriginalSourceCommandWorld| {
        let mut state = std::collections::hash_map::DefaultHasher::new();
        world.hash(&mut state);
        state.finish()
    };
    let source = "proc alpha {} {}; proc beta {} {}; rename alpha gamma";
    for version in tcl_dialect::TclVersion::ALL {
        let first = analyse(source, version)
            .original_completed_command_world()
            .unwrap();
        let independent = analyse(source, version)
            .original_completed_command_world()
            .unwrap();
        let left = first.state.original_command_world.as_ref();
        let right = independent.state.original_command_world.as_ref();
        assert_eq!(left, right);
        assert_eq!(hash(left), hash(right));
        let names = first
            .declarations()
            .map(|publication| publication.slot.simple.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            [
                NameBytes::from(b"beta".as_slice()),
                NameBytes::from(b"gamma".as_slice())
            ]
        );
        assert!(
            first
                .declarations()
                .zip(independent.declarations())
                .all(|(a, b)| a == b)
        );
    }
}

#[test]
fn readonly_original_world_queries_borrow_exact_selected_baselines_and_publications() {
    // naming.command.original-world-readonly-query
    // docs/design/analysis/name-resolution-proofs/original-world-readonly-query.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let state = ModuleCommandBindings::initial(registry);
        let original = state.original_command_world.as_ref();
        let query = original.query_view(&state);
        let policy = original.policy.expect("actual selected baseline recipe");
        assert!(matches!(&query.world, std::borrow::Cow::Borrowed(_)));
        assert!(std::ptr::eq(query.world(), original));
        assert_eq!(query.selected_policy(), Some(policy));
        assert_eq!(query.clone_nanos(), None);
        assert_eq!(
            query
                .world()
                .scope(&state.source_root_namespace_key().unwrap(), policy),
            Some(SignatureNamespaceScope::root(Some(policy)))
        );
    }
    let bindings = analyse(
        "proc original {} {}; interp alias {} forwarding {} original; rename original moved",
        tcl_dialect::TclVersion::V9_1,
    );
    let state = bindings.original_completed_root_state.as_ref().unwrap();
    let original = state.original_command_world.as_ref();
    let query = original.query_view(state);
    assert!(std::ptr::eq(query.world(), original));
    assert!(query.world().publications.iter().any(|publication| {
        publication.slot == slot(&[], b"moved")
            && publication.kind == OriginalCommandPublicationKind::Procedure
    }));
    assert!(query.world().publications.iter().any(|publication| {
        publication.slot == slot(&[], b"forwarding")
            && publication.kind == OriginalCommandPublicationKind::Alias
            && publication
                .alias_target
                .as_ref()
                .unwrap()
                .name_input()
                .bytes()
                == b"original"
    }));
    assert!(
        query
            .world()
            .publications
            .iter()
            .all(|publication| { publication.slot != slot(&[], b"original") })
    );
}

#[test]
fn readonly_original_world_queries_prepare_only_owned_missing_policy_or_root() {
    // naming.command.original-world-readonly-query
    // docs/design/analysis/name-resolution-proofs/original-world-readonly-query.md
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.1").commands();
    let state = ModuleCommandBindings::initial(registry);
    let original = state.original_command_world.as_ref();
    let policy = original.policy.unwrap();
    let root = state.source_root_namespace_key().unwrap();
    let mut missing_root = original.clone();
    missing_root
        .namespaces
        .retain(|namespace| namespace.key != root);
    let query = missing_root.query_view(&state);
    assert!(matches!(&query.world, std::borrow::Cow::Owned(_)));
    assert_eq!(query.selected_policy(), Some(policy));
    assert_eq!(
        query.world().scope(&root, policy),
        Some(SignatureNamespaceScope::root(Some(policy)))
    );
    assert!(
        missing_root
            .namespaces
            .iter()
            .all(|namespace| namespace.key != root)
    );
    let unselected = OriginalSourceCommandWorld::default();
    let query = unselected.query_view(&state);
    assert!(matches!(&query.world, std::borrow::Cow::Owned(_)));
    assert_eq!(query.selected_policy(), Some(policy));
    assert_eq!(query.world(), original);
    assert_eq!(unselected, OriginalSourceCommandWorld::default());
}

#[test]
fn readonly_original_world_queries_keep_policy_refusal_and_cow_withdrawal_current() {
    // naming.command.original-world-readonly-query
    // docs/design/analysis/name-resolution-proofs/original-world-readonly-query.md
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.1").commands();
    let state = ModuleCommandBindings::initial(registry);
    let original = state.original_command_world.as_ref();
    let earlier_query = original.query_view(&state);
    let foreign_registry = tcl_registry::model::ingress::static_context_for("tcl8.4").commands();
    let foreign_state = ModuleCommandBindings::initial(foreign_registry);
    let refused = original.query_view(&foreign_state);
    assert!(matches!(&refused.world, std::borrow::Cow::Owned(_)));
    assert_eq!(refused.selected_policy(), None);
    assert_eq!(refused.world(), original);
    let mut current = state.clone();
    current.mark_opaque_binding_mutation();
    assert!(!Arc::ptr_eq(
        &state.original_command_world,
        &current.original_command_world
    ));
    let withdrawn = current.original_command_world.query_view(&current);
    assert!(matches!(&withdrawn.world, std::borrow::Cow::Owned(_)));
    assert_eq!(withdrawn.selected_policy(), None);
    assert!(withdrawn.world().unavailable);
    assert!(!earlier_query.world().unavailable);
    assert_eq!(earlier_query.selected_policy(), original.policy);
    let unknown = ModuleCommandBindings::initial_with_options(
        registry,
        super::super::SourceAnalysisOptions {
            unknown_entry: true,
            ..Default::default()
        },
        None,
    );
    let unavailable = unknown.original_command_world.query_view(&unknown);
    assert_eq!(unavailable.selected_policy(), None);
    assert!(unavailable.world().unavailable);
}

#[test]
fn original_world_baseline_is_selected_before_branching() {
    // Implementation contract: naming.command.original-world-baseline-bootstrap
    // docs/design/analysis/name-resolution-proofs/original-world-baseline-bootstrap.md
    for version in tcl_dialect::TclVersion::ALL {
        let registry =
            tcl_registry::model::ingress::static_context_for(version.dialect_profile_name())
                .commands();
        let state = ModuleCommandBindings::initial(registry);
        let policy = NamePolicyProtocol::authored_tcl(version);
        assert_eq!(state.original_command_world.policy, Some(policy));
        let mut invoked = (*state.original_command_world).clone();
        assert_eq!(invoked.select_policy(&state), Some(policy));
        assert!(!invoked.join(&state.original_command_world));
        assert!(!invoked.unavailable);
        invoked.withdraw();
        assert_eq!(invoked.select_policy(&state), None);
        assert!(invoked.unavailable);
        let unknown = ModuleCommandBindings::initial_with_options(
            registry,
            super::super::SourceAnalysisOptions {
                unknown_entry: true,
                ..super::super::SourceAnalysisOptions::default()
            },
            None,
        );
        assert!(unknown.original_command_world.unavailable);
        assert_eq!(unknown.original_command_world.policy, None);
    }
}

#[test]
fn original_simple_procedure_installs_before_following_command() {
    for version in tcl_dialect::TclVersion::ALL {
        let source = "proc p {} {}; set checkpoint READY; p";
        let bindings = analyse(source, version);
        let point = bindings
            .invocation_at_source("", u32::try_from(source.rfind("; p").unwrap() + 2).unwrap());
        let reference = point
            .original_evaluated_command_reference()
            .expect("original procedure is installed in the actual following lookup state");
        assert_eq!(reference.original_slot(), Some(&slot(&[], b"p")));
        assert_eq!(reference.definition().unwrap().allocation().site.offset, 0);
    }
}

#[test]
fn original_alias_prefix_inputs_follow_actual_allocations_and_held_values() {
    // Implementation contract: naming.alias.captured-prefix-allocation-contract
    // docs/design/analysis/name-resolution-proofs/alias-captured-prefix-allocation-contract.md
    for version in tcl_dialect::TclVersion::ALL {
        let source = "set name first; interp alias {} inner {} set $name; interp alias {} outer {} inner; set name second; outer VALUE; set checkpoint $first";
        let bindings = analyse(source, version);
        let offset = u32::try_from(source.find("outer VALUE").unwrap()).unwrap();
        let point = bindings.invocation_at_source("outer", offset);
        let target = point
            .proved_handler_target()
            .expect("one current alias implementation chain");
        let inputs = target
            .original_prepended_name_inputs()
            .expect("captured producer survives a known later cell write");
        assert_eq!(
            inputs
                .iter()
                .map(crate::signature_scan::scope::SignatureSourceNameInput::bytes)
                .collect::<Vec<_>>(),
            vec![b"first".as_slice()]
        );
        assert!(
            inputs[0].original_word_key().is_none(),
            "a captured read is not a written call operand"
        );
        let last = bindings.invocation_at_source(
            "set",
            u32::try_from(source.rfind("set checkpoint").unwrap()).unwrap(),
        );
        assert_eq!(
            last.evaluated_written_argument_value(2),
            Some("VALUE"),
            "the captured input reaches the actual Set operand owner"
        );
        let replaced = "interp alias {} write {} set first; interp alias {} write {} set second; write VALUE; set checkpoint $second";
        let bindings = analyse(replaced, version);
        let point = bindings.invocation_at_source(
            "write",
            u32::try_from(replaced.find("write VALUE").unwrap()).unwrap(),
        );
        let target = point
            .proved_handler_target()
            .expect("replacement has its own current allocation");
        assert_eq!(
            target.original_prepended_name_inputs().unwrap()[0].bytes(),
            b"second"
        );
    }
}

#[test]
fn completed_original_publications_follow_moves_and_namespace_lifetimes() {
    for version in tcl_dialect::TclVersion::ALL {
        let bindings = analyse(
            "proc P {} {}; rename P Q; namespace eval N {proc p {} {}}",
            version,
        );
        let world = bindings
            .original_completed_command_world()
            .expect("closed genuine Normal root");
        let policy = NamePolicyProtocol::authored_tcl(version);
        assert!(world.declaration_at(&slot(&[], b"P"), policy).is_none());
        let moved = world
            .declaration_at(&slot(&[], b"Q"), policy)
            .expect("retained original move");
        assert_eq!(moved.kind(), OriginalCommandPublicationKind::Procedure);
        assert!(
            moved.definition().is_some(),
            "emitted implementation allocation joins independently"
        );
        assert!(world.declaration_at(&slot(&[b"N"], b"p"), policy).is_some());
        let deleted = analyse(
            "namespace eval N {proc p {} {}}; namespace delete N; namespace eval N {}",
            version,
        );
        assert!(
            deleted
                .original_completed_command_world()
                .expect("completed delete and fresh empty namespace")
                .declaration_at(&slot(&[b"N"], b"p"), policy)
                .is_none()
        );
    }
}

#[test]
fn completed_world_requires_normal_root_and_closed_mutations() {
    for version in tcl_dialect::TclVersion::ALL {
        for source in [
            "proc P {} {}; error BOOM",
            "proc P {} {}; eval $unknown",
            "proc P {} {}; rename $unknown Q",
        ] {
            assert!(
                analyse(source, version)
                    .original_completed_command_world()
                    .is_none(),
                "{version:?}: {source}"
            );
        }
        let bindings = analyse("proc rename args {}; proc P {} {}; rename P Q", version);
        let world = bindings
            .original_completed_command_world()
            .expect("source replacement body completes normally");
        let policy = NamePolicyProtocol::authored_tcl(version);
        assert!(world.declaration_at(&slot(&[], b"P"), policy).is_some());
        assert!(
            world.declaration_at(&slot(&[], b"Q"), policy).is_none(),
            "replaced Registry worker cannot donate a move"
        );
    }
}

#[test]
fn original_qualified_forget_joins_current_origin_after_source_and_alias_moves() {
    // Implementation contract: naming.namespace.original-qualified-forget-publications
    // docs/design/analysis/name-resolution-proofs/namespace-original-qualified-forget-publications.md
    let source = r"rename auto_import savedPrelude; namespace eval A {proc p\uD800 {} {}; proc p\uD801 {} {}; namespace export p*}; namespace eval B {namespace import ::A::*; rename p\uD800 alias\uD800}; rename ::A::p\uD800 ::A::q\uD800; namespace eval B {namespace forget ::A::p*}";
    for version in tcl_dialect::TclVersion::ALL {
        let bindings = analyse(source, version);
        let world = bindings
            .original_completed_command_world()
            .unwrap_or_else(|| panic!("{}", version.dialect_profile_name()));
        let policy = NamePolicyProtocol::authored_tcl(version);
        let retained = world
            .declaration_at(&slot(&[b"B"], b"alias\xed\xa0\x80"), policy)
            .unwrap();
        let origin = world
            .declaration_at(&slot(&[b"A"], b"q\xed\xa0\x80"), policy)
            .unwrap();
        assert_eq!(retained.kind(), OriginalCommandPublicationKind::Imported);
        assert_eq!(retained.imported_origin(), Some(origin.declaration_site()));
        assert_eq!(retained.definition(), origin.definition());
        assert!(
            world
                .declaration_at(&slot(&[b"B"], b"p\xed\xa0\x81"), policy)
                .is_none()
        );
        assert!(
            world
                .declaration_at(&slot(&[b"A"], b"p\xed\xa0\x81"), policy)
                .is_some()
        );
    }
}

#[test]
fn original_namespace_world_requires_the_explicit_source_compilation_mode() {
    // Implementation contract: naming.namespace.original-qualified-forget-publications
    // docs/design/analysis/name-resolution-proofs/namespace-original-qualified-forget-publications.md
    for version in tcl_dialect::TclVersion::ALL {
        let bindings = analyse_with_compilation(
            "namespace eval N {proc p {} {}}",
            version,
            tcl_registry::native_compilation::NativeCompilationContext::default(),
        );
        assert!(
            bindings.original_completed_command_world().is_none(),
            "{version:?}"
        );
    }
}

#[test]
fn original_namespace_import_keeps_the_source_declaration_allocation() {
    for version in tcl_dialect::TclVersion::ALL {
        let bindings = analyse(
            "rename auto_import savedPrelude; namespace eval A {proc p {} {}; namespace export p}; namespace eval B {namespace import ::A::p}",
            version,
        );
        let world = bindings
            .original_completed_command_world()
            .expect("closed original namespace operations");
        let policy = NamePolicyProtocol::authored_tcl(version);
        let origin = world
            .declaration_at(&slot(&[b"A"], b"p"), policy)
            .expect("source declaration");
        let imported = world
            .declaration_at(&slot(&[b"B"], b"p"), policy)
            .expect("exact original import");
        assert_eq!(imported.kind(), OriginalCommandPublicationKind::Imported);
        assert_eq!(imported.imported_origin(), Some(origin.declaration_site()));
        assert_eq!(imported.definition(), origin.definition());
    }
}
