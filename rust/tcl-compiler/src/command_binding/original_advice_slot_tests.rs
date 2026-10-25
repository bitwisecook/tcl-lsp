// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software descriptor indexes retain their original lookup barriers.

use super::super::CommandToken;
use super::*;
use std::sync::Arc;
use tcl_lexer::{LexerConfig, SourceImage, Span};
use tcl_syntax::naming::NamePolicyProtocol;

fn state(
    version: tcl_dialect::TclVersion,
) -> (ModuleCommandBindings, LexerConfig, NamePolicyProtocol) {
    let dialect = tcl_registry::InvocationDialect::for_version(version);
    let registry =
        tcl_registry::model::ingress::static_context_for(version.dialect_name()).commands();
    let config = LexerConfig::from_grammar(dialect.lexer_grammar);
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

fn input(
    source: &str,
    config: LexerConfig,
    policy: NamePolicyProtocol,
) -> SignatureSourceNameInput {
    let plan = tcl_lexer::native_script_words_in(
        SourceImage::document(source),
        Span::new(0, source.len().try_into().unwrap()),
        config,
    )
    .unwrap();
    let [command] = plan.commands.as_slice() else {
        panic!("original single command")
    };
    let [word] = command.words.as_slice() else {
        panic!("original single word")
    };
    SignatureSourceNameInput::OriginalWord(
        crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
            word,
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
            policy,
        )
        .unwrap(),
    )
}

#[test]
fn advice_slot_index_preserves_original_collisions_and_map_birth() {
    // naming.command.original-world-readonly-query
    // docs/design/analysis/name-resolution-proofs/original-world-readonly-query.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let policy = tcl_registry::InvocationDialect::of_profile(registry.profile().unwrap())
            .authored_name_policy()
            .unwrap();
        let mut rows = Arc::new(AdviceRows::from([
            ("p".to_owned(), "plain".to_owned()),
            ("::p".to_owned(), "qualified".to_owned()),
            ("ns::q".to_owned(), "nested".to_owned()),
            ("opaque\0tail".to_owned(), "nul".to_owned()),
            ("café".to_owned(), "unicode".to_owned()),
        ]));
        let memo = RegistryAdviceSlotMemo::default();
        let original = memo.index(policy, &rows).unwrap();
        assert_eq!(
            Arc::strong_count(&rows),
            1,
            "index retains no map payload owner"
        );
        let collision = original_registry_metadata_slot(policy, "p").unwrap();
        assert_eq!(original.slots.get(&collision), Some(&None));
        let nested = original_registry_metadata_slot(policy, "ns::q").unwrap();
        assert_eq!(
            original.slots.get(&nested),
            Some(&Some(("ns::q".to_owned(), "nested".to_owned())))
        );
        assert_eq!(
            original.slots.len(),
            2,
            "same ASCII/NUL gate as the metadata worker"
        );
        assert!(Arc::ptr_eq(&original, &memo.index(policy, &rows).unwrap()));
        let old_birth = Arc::downgrade(&rows);
        Arc::make_mut(&mut rows).remove("::p");
        assert!(old_birth.upgrade().is_none());
        assert!(!original.matches(policy, &rows));
        let replaced = memo.index(policy, &rows).unwrap();
        assert!(!Arc::ptr_eq(&original, &replaced));
        assert_eq!(
            replaced.slots.get(&collision),
            Some(&Some(("p".to_owned(), "plain".to_owned())))
        );
        let independent = Arc::new(rows.as_ref().clone());
        let distinct = memo.index(policy, &independent).unwrap();
        assert!(!Arc::ptr_eq(&replaced, &distinct));
        assert_eq!(replaced.slots, distinct.slots);
    }
}

#[test]
fn warmed_advice_index_keeps_current_tombstones_and_document_override() {
    // naming.command.original-world-readonly-query
    // docs/design/analysis/name-resolution-proofs/original-world-readonly-query.md
    for version in tcl_dialect::TclVersion::ALL {
        let (original, config, policy) = state(version);
        let root = original.source_root_namespace_key().unwrap();
        let name = input("proc", config, policy);
        let first = original
            .original_advice_candidates_for_input(&root, &name)
            .1
            .unwrap();
        let warm = original
            .baseline
            .advice_slots
            .index(policy, &original.baseline.catalogue_commands)
            .unwrap();
        let key = original
            .original_command_paths_for_input(&root, &name)
            .unwrap()[0][0]
            .clone();
        for binding in [MayBinding::Missing, MayBinding::Unknown] {
            let mut changed = original.clone();
            changed.replace(key.clone(), BTreeSet::from([binding]));
            assert!(Arc::ptr_eq(
                &warm,
                &changed
                    .baseline
                    .advice_slots
                    .index(policy, &changed.baseline.catalogue_commands)
                    .unwrap()
            ));
            assert_eq!(
                changed.original_advice_candidates_for_input(&root, &name),
                (None, None)
            );
        }
        let mut declared = original.clone();
        let baseline = Arc::make_mut(&mut declared.baseline);
        Arc::make_mut(&mut baseline.declared_commands)
            .insert(first.slot.clone(), "document-proc".to_owned());
        let (document, catalogue) = declared.original_advice_candidates_for_input(&root, &name);
        assert_eq!(document.unwrap().name, "document-proc");
        assert!(catalogue.is_none());
        let mut collision = declared.clone();
        let baseline = Arc::make_mut(&mut collision.baseline);
        Arc::make_mut(&mut baseline.declared_commands)
            .insert("proc".to_owned(), "second-document".to_owned());
        assert_eq!(
            collision.original_advice_candidates_for_input(&root, &name),
            (None, None)
        );
        assert_eq!(
            original
                .original_advice_candidates_for_input(&root, &name)
                .1
                .unwrap(),
            first
        );
        let mut opaque = original.clone();
        opaque.opaque_domain = true;
        assert_eq!(
            opaque.original_advice_candidates_for_input(&root, &name),
            (None, None)
        );
    }
}

#[test]
fn warmed_advice_index_preserves_original_namespace_path_barriers() {
    // naming.command.original-world-readonly-query
    // docs/design/analysis/name-resolution-proofs/original-world-readonly-query.md
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let (mut original, config, policy) = state(version);
        let root = original.source_root_namespace_key().unwrap();
        let own = SourceNamespaceKey::authored("::oo");
        let name = input("class", config, policy);
        let baseline = Arc::make_mut(&mut original.baseline);
        Arc::make_mut(&mut baseline.catalogue_commands)
            .insert("::class".to_owned(), "root-proposal".to_owned());
        let nested = original
            .original_advice_candidates_for_input(&own, &name)
            .1
            .unwrap();
        assert_eq!(nested.name, "oo::class");
        assert_eq!(
            original
                .original_advice_candidates_for_input(&root, &name)
                .1
                .unwrap()
                .name,
            "root-proposal"
        );
        let key = original
            .original_command_paths_for_input(&own, &name)
            .unwrap()[0][0]
            .clone();
        let mut blocked = original.clone();
        blocked.replace(key, BTreeSet::from([MayBinding::Unknown]));
        assert!(
            blocked
                .original_advice_candidates_for_input(&own, &name)
                .1
                .is_none()
        );
        assert_eq!(
            original
                .original_advice_candidates_for_input(&own, &name)
                .1
                .unwrap(),
            nested
        );
        let foreign_policy =
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4)
                .authored_name_policy()
                .unwrap();
        let foreign = input(
            "class",
            LexerConfig::from_grammar(
                tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4)
                    .lexer_grammar,
            ),
            foreign_policy,
        );
        assert_eq!(
            original.original_advice_candidates_for_input(&own, &foreign),
            (None, None)
        );
    }
}

#[test]
fn conditional_catalogue_index_preserves_actual_live_binding_obligations() {
    // naming.compiler.conditional-registry-source-metadata
    // docs/design/analysis/name-resolution-proofs/conditional-registry-source-metadata.md
    use super::super::OriginalCatalogueSourceObligation as Obligation;
    for version in tcl_dialect::TclVersion::ALL {
        let (original, config, policy) = state(version);
        let root = original.source_root_namespace_key().unwrap();
        let scope = original.original_namespace_geometry(&root, policy).unwrap();
        let name = input("proc", config, policy);
        let mut unknown = original.clone();
        unknown.opaque_domain = true;
        let first = unknown
            .original_catalogue_source_candidate(&root, &scope, &name)
            .unwrap();
        assert_eq!(first.0.name, "proc");
        assert!(first.2.contains(&Obligation::UnknownCurrentLookup));
        let target = original
            .original_registry_metadata_target(&root, "proc", policy)
            .unwrap();
        let key = original
            .original_command_paths_for_input(&root, &name)
            .unwrap()[0][0]
            .clone();
        let mut changed = unknown.clone();
        changed.replace(key.clone(), BTreeSet::from([MayBinding::Missing]));
        assert!(
            changed
                .original_catalogue_source_candidate(&root, &scope, &name)
                .is_none()
        );
        let mut custom = target;
        custom.registry_backed = false;
        custom.kind = BindingKind::Proc;
        let mut changed = unknown;
        changed.replace(key, BTreeSet::from([MayBinding::Target(custom)]));
        assert!(
            changed
                .original_catalogue_source_candidate(&root, &scope, &name)
                .is_none()
        );
    }
}

#[test]
fn conditional_catalogue_imports_follow_retained_original_implementations() {
    for version in tcl_dialect::TclVersion::ALL {
        let (mut original, config, policy) = state(version);
        let root = original.source_root_namespace_key().unwrap();
        let scope = original.original_namespace_geometry(&root, policy).unwrap();
        let target = original
            .original_registry_metadata_target(&root, "proc", policy)
            .unwrap();
        let token = CommandToken {
            runtime: None,
            origin: "::proc".to_owned(),
            declaration: 0,
            allocation: None,
        };
        let key = SourceCommandKey::Slot {
            namespace: root.clone(),
            simple: tcl_core_types::NameBytes::from(&b"imported"[..]),
        };
        Arc::make_mut(&mut original.objects).insert(
            token.clone(),
            BTreeSet::from([MayBinding::Target(target.clone())]),
        );
        original.replace(
            key.clone(),
            BTreeSet::from([MayBinding::Imported(token.clone().into())]),
        );
        let name = input("imported", config, policy);
        let selected = original
            .original_catalogue_source_candidate(&root, &scope, &name)
            .unwrap();
        assert_eq!(selected.0.name, "proc");
        for alternative in [MayBinding::Unknown, MayBinding::Missing] {
            let mut changed = original.clone();
            Arc::make_mut(&mut changed.objects)
                .insert(token.clone(), BTreeSet::from([alternative]));
            assert!(
                changed
                    .original_catalogue_source_candidate(&root, &scope, &name)
                    .is_none()
            );
        }
        let mut custom = target;
        custom.registry_backed = false;
        custom.kind = BindingKind::Proc;
        Arc::make_mut(&mut original.objects)
            .insert(token, BTreeSet::from([MayBinding::Target(custom)]));
        assert!(
            original
                .original_catalogue_source_candidate(&root, &scope, &name)
                .is_none()
        );
    }
}

#[test]
fn advice_index_warmth_preserves_full_baseline_equality_and_bounded_fallback() {
    // naming.command.original-world-readonly-query
    // docs/design/analysis/name-resolution-proofs/original-world-readonly-query.md
    use std::hash::{Hash, Hasher};
    let (original, config, policy) = state(tcl_dialect::TclVersion::V9_1);
    let before = original.clone();
    let hash = |state: &ModuleCommandBindings| {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        state.hash(&mut hash);
        hash.finish()
    };
    let prior_hash = hash(&before);
    let root = original.source_root_namespace_key().unwrap();
    let name = input("proc", config, policy);
    let expected = original.original_advice_candidates_for_input(&root, &name);
    assert_eq!(original, before);
    assert_eq!(hash(&original), prior_hash);
    let fresh = Arc::new(original.baseline.catalogue_commands.as_ref().clone());
    assert!(!Arc::ptr_eq(&fresh, &original.baseline.catalogue_commands));
    let same = original
        .baseline
        .advice_slots
        .index(policy, &fresh)
        .unwrap();
    let current = original
        .baseline
        .advice_slots
        .index(policy, &original.baseline.catalogue_commands)
        .unwrap();
    assert!(!Arc::ptr_eq(&same, &current));
    assert_eq!(same.slots, current.slots);
    let mut fallback = original.clone();
    let baseline = Arc::make_mut(&mut fallback.baseline);
    Arc::make_mut(&mut baseline.catalogue_commands).extend(
        (0..MAX_BASELINE_METADATA_NAMES + 1)
            .map(|n| (format!("::memo_capacity_{n}"), format!("memo_capacity_{n}"))),
    );
    assert!(
        fallback
            .baseline
            .advice_slots
            .index(policy, &fallback.baseline.catalogue_commands)
            .is_none()
    );
    assert_eq!(
        fallback.original_advice_candidates_for_input(&root, &name),
        expected
    );
    assert_eq!(
        original.original_advice_candidates_for_input(&root, &name),
        expected
    );
}

#[test]
fn registry_descriptor_indexes_preserve_components_and_decline_coordinate_collisions() {
    let mut registry = tcl_registry::CommandRegistry::build_default();
    let labels = [
        ("NAME_DESCRIPTOR::", "::NAME_DESCRIPTOR::"),
        ("NAME_DESCRIPTOR:::member", "::NAME_DESCRIPTOR:::member"),
        ("::::NAME_DESCRIPTOR", "::::NAME_DESCRIPTOR"),
        (":NAME_DESCRIPTOR", ":::NAME_DESCRIPTOR"),
    ];
    for (label, _) in labels {
        registry.insert(tcl_registry::CommandSpec {
            name: label,
            ..tcl_registry::CommandSpec::DEFAULT
        });
    }
    for label in ["NAME_DESCRIPTOR_COLLISION", "::NAME_DESCRIPTOR_COLLISION"] {
        registry.insert(tcl_registry::CommandSpec {
            name: label,
            ..tcl_registry::CommandSpec::DEFAULT
        });
    }
    let baseline = super::super::BindingBaseline::for_registry(&registry);
    for (label, coordinate) in labels {
        assert_eq!(
            baseline
                .catalogue_commands
                .get(coordinate)
                .map(String::as_str),
            Some(label)
        );
    }
    for absent in [
        "::NAME_DESCRIPTOR",
        "::NAME_DESCRIPTOR::member",
        "::NAME_DESCRIPTOR_COLLISION",
    ] {
        assert!(
            !baseline.catalogue_commands.contains_key(absent),
            "{absent}"
        );
    }
    for (left, right) in [
        ("NAME_DESCRIPTOR::", "::NAME_DESCRIPTOR"),
        ("NAME_DESCRIPTOR:::member", "NAME_DESCRIPTOR::member"),
        ("::::NAME_DESCRIPTOR", "::NAME_DESCRIPTOR"),
        ("::", ""),
    ] {
        assert!(!tcl_registry::command_descriptor_labels_match(left, right));
    }
    assert!(tcl_registry::command_descriptor_labels_match(
        "NAME_DESCRIPTOR:::member",
        "::NAME_DESCRIPTOR:::member"
    ));
}

#[test]
fn declared_descriptor_indexes_preserve_components_and_decline_coordinate_collisions() {
    use tcl_dialect::model::Provenance;
    use tcl_registry::model::{DeclaredCommand, DeclaredSurface};
    let registry = tcl_registry::CommandRegistry::build_default();
    let mut declarations = DeclaredSurface::new();
    for label in [
        "NAME_DECLARED::",
        "NAME_DECLARED:::member",
        "::::NAME_DECLARED",
        "NAME_DECLARED_COLLISION",
        "::NAME_DECLARED_COLLISION",
    ] {
        declarations.declare(DeclaredCommand::new(
            label.to_owned(),
            Vec::new(),
            Provenance::Document,
        ));
    }
    let state = ModuleCommandBindings::initial_with_options(
        &registry,
        super::super::SourceAnalysisOptions {
            declared_commands: Some(&declarations),
            ..Default::default()
        },
        None,
    );
    for (coordinate, label) in [
        ("::NAME_DECLARED::", "NAME_DECLARED::"),
        ("::NAME_DECLARED:::member", "NAME_DECLARED:::member"),
        ("::::NAME_DECLARED", "::::NAME_DECLARED"),
    ] {
        assert_eq!(
            state
                .baseline
                .declared_commands
                .get(coordinate)
                .map(String::as_str),
            Some(label)
        );
    }
    for absent in [
        "::NAME_DECLARED",
        "::NAME_DECLARED::member",
        "::NAME_DECLARED_COLLISION",
    ] {
        assert!(
            !state.baseline.declared_commands.contains_key(absent),
            "{absent}"
        );
    }
}

#[test]
fn selected_baseline_roster_keeps_its_owner_and_all_other_baseline_fields() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4);
    let selected = registry.effective_semantics_for_dialect_in_realm(
        dialect,
        tcl_dialect::model::InvocationRealm::InterpreterRuntime,
    );
    let baseline = super::super::BindingBaseline::for_registry_with_semantics(
        registry,
        Some(Arc::clone(&selected)),
    );
    assert!(Arc::ptr_eq(&baseline.semantics, &selected));
    let mut previous = super::super::BindingBaseline::for_registry(registry);
    previous.semantics = selected;
    previous.refresh_fingerprint();
    assert_eq!(baseline, previous);
    assert_eq!(baseline.fingerprint, previous.fingerprint);
}

#[test]
fn source_entry_roster_obeys_requested_release_instead_of_registry_profile() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4);
    let config = LexerConfig::from_grammar(dialect.lexer_grammar);
    let state = ModuleCommandBindings::initial_with_options(
        registry,
        super::super::SourceAnalysisOptions {
            invocation_dialect: Some(dialect),
            ..Default::default()
        },
        Some(config),
    );
    assert_eq!(state.baseline.dialect, Some(dialect));
    assert_eq!(state.baseline.release, Some(tcl_dialect::TclVersion::V8_4));
    assert!(state.baseline.semantics.command("set").is_some());
    assert!(state.baseline.semantics.command("try").is_none());
    assert!(!state.baseline.semantics.binding_names().contains("::try"));
    assert_eq!(
        state.baseline.registry_snapshot,
        Some(registry.snapshot().semantic_key()),
    );
}
