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
use crate::signature_scan::scope::SignatureSourceNameKey;

#[test]
fn original_initial_table_selects_the_registered_handler() {
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = tcl_native_model::InvocationDialect::for_version(version);
        let registry =
            tcl_registry::model::ingress::static_context_for(version.dialect_profile_name())
                .commands();
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        let state = ModuleCommandBindings::initial_with_options(
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                ..Default::default()
            },
            Some(config),
        );
        let image = tcl_lexer::SourceImage::document("proc p {} {}");
        let words =
            tcl_lexer::native_script_words_in(image, tcl_lexer::Span::new(0, 12), config).unwrap();
        let key = SignatureSourceNameKey::from_original_native_word(
            &words.commands[0].words[0],
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
            dialect.authored_name_policy().unwrap(),
        )
        .unwrap();
        let selected = state
            .original_targets_for_input(
                &SignatureSourceNameInput::OriginalWord(key),
                &state.source_root_namespace_key().unwrap(),
                super::super::CommandTargetLookup::NamedSlots,
            )
            .expect("the modeled baseline retains its exact namespace and table policy");
        assert!(!selected.unknown && !selected.may_be_absent, "{version:?}");
        let targets = selected.targets.into_iter().collect::<Vec<_>>();
        let [target] = targets.as_slice() else {
            panic!("one actual modeled Registry handler");
        };
        assert!(target.registry_backed && target.terminal);
        assert_eq!(
            registry.get(&target.command).unwrap().lowering_hook,
            Some(tcl_registry::hooks::LoweringHookId::Proc)
        );
    }
}

#[test]
fn original_jim_baseline_uses_the_selected_flat_registry_name_purpose() {
    let dialect = tcl_native_model::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
    );
    let registry = tcl_registry::model::ingress::static_context_for("jimtcl").commands();
    let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
    let policy = dialect.authored_name_policy().unwrap();
    let state = ModuleCommandBindings::initial_with_options(
        registry,
        super::super::SourceAnalysisOptions {
            invocation_dialect: Some(dialect),
            ..Default::default()
        },
        Some(config),
    );
    let root = state.source_root_namespace_key().unwrap();
    for name in ["proc", "set", "info", "lsort"] {
        let input = catalogue_input(name, config, policy);
        let selected = state
            .original_targets_for_input(
                &input,
                &root,
                super::super::CommandTargetLookup::NamedSlots,
            )
            .expect("actual selected Jim Registry cell");
        assert!(!selected.unknown && !selected.may_be_absent, "{name}");
        let targets = selected.targets.into_iter().collect::<Vec<_>>();
        let [target] = targets.as_slice() else {
            panic!("one Jim handler")
        };
        assert!(target.registry_backed && target.terminal);
        assert_eq!(registry.get(&target.command).unwrap().name, name);
        let qualified = catalogue_input(&format!("::{name}"), config, policy);
        assert_eq!(
            state
                .original_targets_for_input(
                    &qualified,
                    &root,
                    super::super::CommandTargetLookup::NamedSlots
                )
                .unwrap()
                .targets,
            BTreeSet::from([target.clone()])
        );
        let mut changed = state.clone();
        let key = changed
            .original_command_paths_for_input(&root, &input)
            .unwrap()[0][0]
            .clone();
        changed.replace(key, BTreeSet::from([MayBinding::Unknown]));
        assert!(
            changed
                .original_registry_metadata_target(&root, name, policy)
                .is_none()
        );
    }
    let unknown = ModuleCommandBindings::initial_with_options(
        registry,
        super::super::SourceAnalysisOptions {
            invocation_dialect: Some(dialect),
            unknown_entry: true,
            ..Default::default()
        },
        Some(config),
    );
    assert!(
        unknown
            .original_targets_for_input(
                &catalogue_input("proc", config, policy),
                &unknown.source_root_namespace_key().unwrap(),
                super::super::CommandTargetLookup::NamedSlots
            )
            .is_none()
    );
    let c = tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    assert!(
        state
            .original_targets_for_input(
                &catalogue_input(
                    "proc",
                    tcl_lexer::LexerConfig::from_grammar(c.lexer_grammar),
                    c.authored_name_policy().unwrap()
                ),
                &root,
                super::super::CommandTargetLookup::NamedSlots
            )
            .is_none()
    );
}

#[test]
fn closed_original_lookup_retains_missing_holder_absence_without_a_slot() {
    // Software contract only: a closed retained namespace roster proves
    // no named target, but does not supply an unknown-handler result.
    // Provider bootstrap evidence: bootstrap-original-mathop-constructor-publication.md.
    for version in tcl_dialect::TclVersion::ALL {
        let (state, config, policy) = catalogue_state(version);
        let root = state.source_root_namespace_key().unwrap();
        let input = catalogue_input("::unregistered_holder::missing", config, policy);
        let paths = state
            .original_command_paths_for_input(&root, &input)
            .unwrap();
        assert_ne!(
            paths,
            [] as [std::vec::Vec<crate::command_binding::namespace_context::SourceCommandKey>; 0]
        );
        assert!(paths.iter().all(Vec::is_empty));
        let selected = state
            .original_targets_for_input(
                &input,
                &root,
                super::super::CommandTargetLookup::NamedSlots,
            )
            .unwrap();
        assert!(selected.targets.is_empty() && selected.may_be_absent && !selected.unknown);
        let fallback = state
            .original_targets_for_input(
                &input,
                &root,
                super::super::CommandTargetLookup::WithFallback,
            )
            .unwrap();
        assert!(fallback.targets.is_empty() && fallback.may_be_absent && fallback.unknown);
        assert!(
            state
                .original_definition_key(&root, &input, CommandBindingDefinitionKind::Procedure,)
                .is_none()
        );
        let mut open = state.clone();
        super::super::Arc::make_mut(&mut open.baseline).unknown_entry = true;
        assert!(
            open.original_command_paths_for_input(&root, &input)
                .is_none()
        );
        let mut opaque = state.clone();
        opaque.opaque_domain = true;
        assert!(
            opaque
                .original_command_paths_for_input(&root, &input)
                .is_none()
        );
    }
}

#[test]
fn original_conditional_catalogue_keeps_unknowns_but_blocks_known_cells() {
    // Implementation contract: naming.compiler.conditional-registry-source-metadata
    // docs/design/analysis/name-resolution-proofs/conditional-registry-source-metadata.md
    use super::super::OriginalCatalogueSourceObligation as Obligation;
    for version in tcl_dialect::TclVersion::ALL {
        let (state, config, policy) = catalogue_state(version);
        let root = state.source_root_namespace_key().unwrap();
        let scope = state.original_namespace_geometry(&root, policy).unwrap();
        let input = catalogue_input("proc", config, policy);
        let key = state
            .original_command_paths_for_input(&root, &input)
            .unwrap()[0][0]
            .clone();
        let mut unknown = state.clone();
        unknown.opaque_domain = true;
        let (candidate, _, obligations) = unknown
            .original_catalogue_source_candidate(&root, &scope, &input)
            .expect("unknown execution may retain a conditional source schema");
        assert_eq!(candidate.name, "proc");
        assert!(obligations.contains(&Obligation::UnknownCurrentLookup));
        assert!(
            unknown
                .original_advice_candidates_for_input(&root, &input)
                .1
                .is_none()
        );
        for replacement in [
            MayBinding::Missing,
            MayBinding::Target({
                let mut custom = state
                    .original_registry_metadata_target(&root, "proc", policy)
                    .unwrap();
                custom.registry_backed = false;
                custom.kind = BindingKind::Proc;
                custom
            }),
        ] {
            let mut blocked = unknown.clone();
            blocked.replace(key.clone(), BTreeSet::from([replacement]));
            assert!(
                blocked
                    .original_catalogue_source_candidate(&root, &scope, &input)
                    .is_none(),
                "{version:?}"
            );
        }
        let mut mixed = unknown.clone();
        mixed.replace(key.clone(), BTreeSet::from([MayBinding::Unknown]));
        let (_, _, obligations) = mixed
            .original_catalogue_source_candidate(&root, &scope, &input)
            .unwrap();
        assert!(
            obligations
                .iter()
                .any(|item| matches!(item, Obligation::SelectedBindingAlternative(_)))
        );
        let mut paths = unknown;
        super::super::Arc::make_mut(&mut paths.unknown_namespace_paths).insert(root.clone());
        assert!(
            paths
                .original_catalogue_source_candidate(&root, &scope, &input)
                .is_none()
        );
    }
}

#[test]
fn original_metadata_index_keeps_collisions_and_complete_baseline_keys() {
    for profile in ["tcl8.4", "tcl8.6", "tcl9.1", "jimtcl"] {
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let dialect = tcl_native_model::InvocationDialect::of_profile(registry.profile().unwrap());
        let policy = dialect.authored_name_policy().unwrap();
        let names = BTreeSet::from(["p".to_owned(), "::p".to_owned()]);
        let slots = metadata_slots(policy, &names);
        let slot = original_registry_metadata_slot(policy, "p").unwrap();
        assert_eq!(slots.get(&slot), Some(&None));
        let state = ModuleCommandBindings::initial(registry);
        let mut before = state.clone();
        std::sync::Arc::make_mut(&mut before.baseline);
        let hash = |value: &ModuleCommandBindings| {
            use std::hash::{Hash, Hasher};
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            value.hash(&mut hash);
            hash.finish()
        };
        let before_hash = hash(&before);
        let slot = original_registry_metadata_slot(policy, "set").unwrap();
        let key = state.original_command_key_for_slot(&slot, policy).unwrap();
        let bindings = state.original_initial_bindings_for_key(&key).unwrap();
        assert!(
            matches!(bindings.first(), Some(MayBinding::Target(target)) if target.registry_backed)
        );
        assert_eq!(state, before);
        assert_eq!(hash(&state), before_hash);
        let index = state
            .baseline
            .metadata_slots
            .get()
            .unwrap()
            .as_ref()
            .unwrap();
        assert!(index.matches(policy, &state.baseline.semantics));
        let mut changed_registry = tcl_registry::CommandRegistry::build_default()
            .project_for_profile(registry.profile().unwrap());
        changed_registry.insert(tcl_registry::CommandSpec {
            name: "added",
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let mut changed = state.clone();
        std::sync::Arc::make_mut(&mut changed.baseline).semantics =
            changed_registry.effective_semantics();
        assert!(!index.matches(policy, &changed.baseline.semantics));
        let added = original_registry_metadata_slot(policy, "added").unwrap();
        let added = changed
            .original_command_key_for_slot(&added, policy)
            .unwrap();
        assert!(
            matches!(changed.original_initial_bindings_for_key(&added).unwrap().first(),
                Some(MayBinding::Target(target)) if target.registry_backed)
        );
        assert_eq!(
            state.original_initial_bindings_for_key(&added),
            Some(BTreeSet::from([MayBinding::Missing]))
        );
        let other = tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_5)
            .authored_name_policy()
            .unwrap();
        if other != policy {
            assert!(!index.matches(other, &state.baseline.semantics));
        }
    }
}

fn catalogue_state(
    version: tcl_dialect::TclVersion,
) -> (
    ModuleCommandBindings,
    tcl_lexer::LexerConfig,
    tcl_syntax::naming::NamePolicyProtocol,
) {
    let dialect = tcl_native_model::InvocationDialect::for_version(version);
    let registry =
        tcl_registry::model::ingress::static_context_for(version.dialect_profile_name()).commands();
    let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
    (
        ModuleCommandBindings::initial_with_options(
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                ..Default::default()
            },
            Some(config),
        ),
        config,
        dialect.authored_name_policy().unwrap(),
    )
}

fn catalogue_input(
    source: &str,
    config: tcl_lexer::LexerConfig,
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> SignatureSourceNameInput {
    let image = tcl_lexer::SourceImage::document(source);
    let plan = tcl_lexer::native_script_words_in(
        image,
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        config,
    )
    .unwrap();
    let [command] = plan.commands.as_slice() else {
        panic!("one complete original command")
    };
    let [word] = command.words.as_slice() else {
        panic!("one complete original head")
    };
    SignatureSourceNameInput::OriginalWord(
        SignatureSourceNameKey::from_original_native_word(
            word,
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
            policy,
        )
        .unwrap(),
    )
}

fn assert_evaluated_command_operand(
    state: &ModuleCommandBindings,
    root_namespace: &SourceNamespaceKey,
    input: &SignatureSourceNameInput,
) {
    assert!(
        state
            .original_command_paths_for_input(root_namespace, input)
            .is_some()
    );
    assert!(
        state
            .original_alias_loop_paths_for_input(root_namespace, input)
            .is_some()
    );
    assert!(
        state
            .original_targets_for_input(
                input,
                root_namespace,
                super::super::CommandTargetLookup::CompilerSlots,
            )
            .is_some()
    );
    assert!(
        state
            .original_publication_key(
                root_namespace,
                input,
                super::super::namespace_slots::PublicationPurpose::Command,
            )
            .is_some()
    );
}

#[test]
fn evaluated_command_roles_do_not_consume_lexical_root_units() {
    let (state, config, policy) = catalogue_state(tcl_dialect::TclVersion::V8_6);
    let root_namespace = state.source_root_namespace_key().unwrap();
    let operand = catalogue_input("proc", config, policy);
    let word = operand.original_word_key().unwrap().original_word();
    let value = SignatureSourceNameInput::OriginalValue(
        crate::signature_scan::scope::SignatureSourceNameValue::from_original_static_word(
            word,
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
            policy,
        )
        .unwrap(),
    );
    let plan = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document("$proc"),
        tcl_lexer::Span::new(0, 5),
        config,
    )
    .unwrap();
    let word = &plan.commands[0].words[0];
    let root = SignatureSourceNameInput::OriginalVariableRoot(
        crate::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_word(
            word,
            word.span(),
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
            policy,
        )
        .unwrap(),
    );
    assert_eq!(root.bytes(), operand.bytes());
    assert_eq!(root.policy(), operand.policy());
    for input in [&operand, &value] {
        assert_evaluated_command_operand(&state, &root_namespace, input);
    }
    assert!(
        state
            .original_command_paths_for_input(&root_namespace, &root)
            .is_none()
    );
    assert!(
        state
            .original_alias_loop_paths_for_input(&root_namespace, &root)
            .is_none()
    );
    assert!(
        state
            .original_targets_for_input(
                &root,
                &root_namespace,
                super::super::CommandTargetLookup::CompilerSlots,
            )
            .is_none()
    );
    assert!(
        state
            .original_publication_key(
                &root_namespace,
                &root,
                super::super::namespace_slots::PublicationPurpose::Command,
            )
            .is_none()
    );
    assert!(!state.original_diagnostic_qualifier_is_retained(&root, &root_namespace));
    let subject = TransitionSubject::LocatedLiteral {
        value: "proc".into(),
        argument_index: 0,
    };
    let mut operands = OriginalCommandOperands {
        inputs: vec![(subject.clone(), operand)],
        procedure_body: false,
        namespace_ensure: None,
    };
    assert!(operands.input(&subject).is_some());
    operands.inputs[0].1 = root;
    assert!(operands.input(&subject).is_none());
}

#[test]
fn original_catalogue_advice_retains_unloaded_metadata_without_runtime_holder() {
    let (state, config, policy) = catalogue_state(tcl_dialect::TclVersion::V8_6);
    let root = state.source_root_namespace_key().unwrap();
    let input = catalogue_input("::struct::tree", config, policy);
    let (declared, catalogue) = state.original_advice_candidates_for_input(&root, &input);
    assert!(declared.is_none());
    assert_eq!(
        catalogue.as_ref().map(|candidate| candidate.name.as_str()),
        Some("struct::tree")
    );
    assert!(
        state
            .original_targets_for_input(
                &input,
                &root,
                super::super::CommandTargetLookup::NamedSlots
            )
            .is_none()
    );
    let binding = super::super::source_binding_from_original_input(&state, &input, &root).unwrap();
    assert!(binding.unknown && binding.may_be_absent && binding.targets.is_empty());
    assert_eq!(binding.catalogue_command, catalogue);
    assert!(binding.proved_target().is_none());
    assert!(
        state
            .original_advice_candidates_for_input(
                &root,
                &catalogue_input("::external::factory", config, policy)
            )
            .1
            .is_none()
    );
}

#[test]
fn original_catalogue_advice_obeys_current_slots_and_document_overrides() {
    for version in tcl_dialect::TclVersion::ALL {
        let (state, config, policy) = catalogue_state(version);
        let root = state.source_root_namespace_key().unwrap();
        let input = catalogue_input("proc", config, policy);
        let candidate = state
            .original_advice_candidates_for_input(&root, &input)
            .1
            .unwrap();
        let key = state
            .original_command_paths_for_input(&root, &input)
            .unwrap()[0][0]
            .clone();
        for replacement in [MayBinding::Unknown, MayBinding::Missing] {
            let mut changed = state.clone();
            changed.replace(key.clone(), BTreeSet::from([replacement]));
            assert!(
                changed
                    .original_advice_candidates_for_input(&root, &input)
                    .1
                    .is_none()
            );
        }
        let mut declared = state.clone();
        let baseline = super::super::Arc::make_mut(&mut declared.baseline);
        super::super::Arc::make_mut(&mut baseline.declared_commands)
            .insert(candidate.slot, "document-proc".to_owned());
        let (document, catalogue) = declared.original_advice_candidates_for_input(&root, &input);
        assert_eq!(
            document.as_ref().map(|candidate| candidate.name.as_str()),
            Some("document-proc")
        );
        assert!(catalogue.is_none());
        let mut unknown = state.clone();
        unknown.opaque_domain = true;
        assert_eq!(
            unknown.original_advice_candidates_for_input(&root, &input),
            (None, None)
        );
    }
}

#[test]
fn original_catalogue_advice_stops_at_earlier_occupied_namespace_cells() {
    let (state, config, policy) = catalogue_state(tcl_dialect::TclVersion::V8_6);
    let own = SourceNamespaceKey::authored("::oo");
    let input = catalogue_input("class", config, policy);
    assert!(state.namespaces.contains(&own));
    let mut proposal = state.clone();
    let baseline = super::super::Arc::make_mut(&mut proposal.baseline);
    super::super::Arc::make_mut(&mut baseline.catalogue_commands)
        .insert("::class".to_owned(), "root-proposal".to_owned());
    assert_eq!(
        proposal
            .original_advice_candidates_for_input(&own, &input)
            .1
            .unwrap()
            .name,
        "oo::class"
    );
    let key = proposal
        .original_command_paths_for_input(&own, &input)
        .unwrap()[0][0]
        .clone();
    proposal.replace(key, BTreeSet::from([MayBinding::Unknown]));
    assert!(
        proposal
            .original_advice_candidates_for_input(&own, &input)
            .1
            .is_none()
    );
    let mut unknown_path = state;
    super::super::Arc::make_mut(&mut unknown_path.unknown_namespace_paths).insert(own.clone());
    // The occupied local slot is selected before consulting the path.
    assert_eq!(
        unknown_path
            .original_advice_candidates_for_input(&own, &input)
            .1
            .unwrap()
            .name,
        "oo::class"
    );
    assert!(
        unknown_path
            .original_advice_candidates_for_metadata(&own, "class\u{d7ff}")
            .1
            .is_none()
    );
}

#[test]
fn original_default_factory_dependencies_use_the_current_exact_registry_cells() {
    let grammar = &tcl_registry::definer::TCLOO_GRAMMAR;
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let dialect = tcl_native_model::InvocationDialect::for_version(version);
        let registry =
            tcl_registry::model::ingress::static_context_for(version.dialect_profile_name())
                .commands();
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        let state = ModuleCommandBindings::initial_with_options(
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                ..Default::default()
            },
            Some(config),
        );
        assert!(
            state.default_construction_dependencies_hold(grammar),
            "{version:?}"
        );
        let root = state.source_root_namespace_key().unwrap();
        let policy = dialect.authored_name_policy().unwrap();
        for dependency in grammar.default_construction_lookup_dependencies().unwrap() {
            let key = state
                .original_registry_command_paths(&root, dependency, policy)
                .unwrap()[0][0]
                .clone();
            let mut shadowed = state.clone();
            shadowed.replace(key.clone(), BTreeSet::from([MayBinding::Unknown]));
            assert!(!shadowed.default_construction_dependencies_hold(grammar));
            let mut wrapper = state
                .original_registry_metadata_target(&root, dependency, policy)
                .unwrap();
            wrapper.kind = BindingKind::Alias;
            let mut aliased = state.clone();
            aliased.replace(key, BTreeSet::from([MayBinding::Target(wrapper)]));
            assert!(!aliased.default_construction_dependencies_hold(grammar));
        }
    }
}

#[test]
fn original_registry_metadata_target_requires_the_current_exact_cell() {
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = tcl_native_model::InvocationDialect::for_version(version);
        let registry =
            tcl_registry::model::ingress::static_context_for(version.dialect_profile_name())
                .commands();
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        let mut state = ModuleCommandBindings::initial_with_options(
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                ..Default::default()
            },
            Some(config),
        );
        let root = state.source_root_namespace_key().unwrap();
        let policy = dialect.authored_name_policy().unwrap();
        let target = state
            .original_registry_metadata_target(&root, "proc", policy)
            .unwrap();
        let baseline = state.clone();
        assert!(target.registry_backed && target.terminal);
        let key = state
            .original_registry_command_paths(&root, "proc", policy)
            .unwrap()[0][0]
            .clone();
        state.replace(key.clone(), BTreeSet::from([MayBinding::Unknown]));
        assert!(
            state
                .original_registry_metadata_target(&root, "proc", policy)
                .is_none()
        );
        state = baseline;
        let mut wrapper = target;
        wrapper.kind = BindingKind::Alias;
        state.replace(key, BTreeSet::from([MayBinding::Target(wrapper)]));
        assert!(
            state
                .original_registry_metadata_target(&root, "proc", policy)
                .is_none()
        );
        assert!(
            state
                .original_registry_metadata_target(&root, "p\u{d7ff}", policy)
                .is_none()
        );
    }
}
#[test]
fn original_mutation_selector_stops_at_the_first_occupied_cell() {
    // Implementation contract: naming.command.original-occupied-mutation-transfer
    // docs/design/analysis/name-resolution-proofs/original-occupied-mutation-transfer.md
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let (mut state, config, policy) = catalogue_state(version);
        let root = state.source_root_namespace_key().unwrap();
        let own = super::super::SourceNamespaceKey::authored("::oo");
        let input = catalogue_input("proc", config, policy);
        let root_key = state
            .original_command_paths_for_input(&root, &input)
            .unwrap()[0][0]
            .clone();
        assert!(
            state
                .original_command_keys_for_input(&own, &input)
                .unwrap()
                .len()
                >= 2
        );
        let (selected, bindings) = state
            .original_occupied_command_for_input(&own, &input)
            .unwrap();
        assert_eq!(selected, root_key);
        let own_key = state
            .original_command_paths_for_input(&own, &input)
            .unwrap()[0][0]
            .clone();
        state.replace(own_key.clone(), bindings.clone());
        assert_eq!(
            state
                .original_occupied_command_for_input(&own, &input)
                .unwrap()
                .0,
            own_key
        );
        state.replace(own_key.clone(), BTreeSet::from([MayBinding::Unknown]));
        assert!(
            state
                .original_occupied_command_for_input(&own, &input)
                .is_none()
        );
        state.replace(
            own_key,
            bindings.into_iter().chain([MayBinding::Missing]).collect(),
        );
        assert!(
            state
                .original_occupied_command_for_input(&own, &input)
                .is_none()
        );
    }
}
