// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original native name operands for the canonical source command table.
//! Geometry is conditional source metadata; installations occur only in the
//! existing selected operation transfer, never by a publication-side replay.

use super::{
    BTreeSet, BindingKind, MayBinding, ModuleCommandBindings, ResolvedCommandTarget,
    SourceCommandKey, SourceExecutionContext, SourceNamespaceKey, SourceNativeInvocation,
};
use crate::signature_scan::scope::{SignatureNamespaceScope, SignatureSourceNameInput};
use tcl_registry::{
    CommandBindingDefinitionKind, CommandBindingTransition, InvocationFacts, StateTransition,
    TransitionSubject,
};

#[cfg(any(test, debug_assertions))]
std::thread_local! {
    static BASELINE_LOOKUP_TOTALS: std::cell::Cell<(u64, u128)> = const {
        std::cell::Cell::new((0, 0))
    };
}

#[cfg(any(test, debug_assertions))]
pub(super) fn baseline_lookup_totals() -> (u64, u128) {
    BASELINE_LOOKUP_TOTALS.with(std::cell::Cell::get)
}

#[cfg(any(test, debug_assertions))]
struct BaselineLookupTiming(std::time::Instant);

#[cfg(any(test, debug_assertions))]
impl BaselineLookupTiming {
    fn at_query() -> Option<Self> {
        std::env::var_os("TCL_LSP_TRACE_SOURCE_PHASES")
            .is_some()
            .then(|| Self(std::time::Instant::now()))
    }
}

#[cfg(any(test, debug_assertions))]
impl Drop for BaselineLookupTiming {
    fn drop(&mut self) {
        BASELINE_LOOKUP_TOTALS.with(|totals| {
            let (calls, nanos) = totals.get();
            totals.set((
                calls.saturating_add(1),
                nanos.saturating_add(self.0.elapsed().as_nanos()),
            ));
        });
    }
}

// Fixed Registry descriptors are independently authored baseline metadata.
// C registration and Jim's flat global table use different naming purposes;
// neither projection manufactures an original source operand or native entry.
pub(super) fn original_registry_metadata_slot(
    policy: tcl_syntax::naming::NamePolicyProtocol,
    name: &str,
) -> Option<tcl_core_types::ByteCommandSlot> {
    use tcl_syntax::naming::{NativeNameContext, NativeNameProtocol};
    if !name.is_ascii() || name.as_bytes().contains(&0) {
        return None;
    }
    match policy.recipe() {
        NativeNameProtocol::C(_) => policy
            .recipe()
            .command_c_api_publication_slot(NativeNameContext::root(), name.as_bytes())
            .ok(),
        NativeNameProtocol::Jim084 => {
            let input = policy
                .recipe()
                .command_lookup_input(NativeNameContext::root(), name.as_bytes())
                .ok()?;
            Some(tcl_core_types::ByteCommandSlot::new(
                tcl_core_types::ByteNamespacePath::root(),
                tcl_core_types::NameBytes::from(input.jim_flat_key()?),
            ))
        }
    }
}

const MAX_BASELINE_METADATA_NAMES: usize = 4096;

/// Derived spelling projection over one immutable Registry baseline. Current
/// table occupancy, namespace geometry and native absence are checked outside.
#[derive(Debug, Clone)]
pub(super) struct RegistryMetadataSlotIndex {
    policy: tcl_syntax::naming::NamePolicyProtocol,
    semantics: std::sync::Arc<tcl_registry::registry::EffectiveRegistrySemantics>,
    names: BTreeSet<String>,
    slots: std::collections::HashMap<tcl_core_types::ByteCommandSlot, Option<String>>,
}

impl RegistryMetadataSlotIndex {
    fn new(
        policy: tcl_syntax::naming::NamePolicyProtocol,
        semantics: &std::sync::Arc<tcl_registry::registry::EffectiveRegistrySemantics>,
    ) -> Option<Self> {
        let names = semantics.binding_names();
        if names.len() > MAX_BASELINE_METADATA_NAMES {
            return None;
        }
        Some(Self {
            policy,
            semantics: std::sync::Arc::clone(semantics),
            names: names.clone(),
            slots: metadata_slots(policy, names),
        })
    }

    fn matches(
        &self,
        policy: tcl_syntax::naming::NamePolicyProtocol,
        semantics: &std::sync::Arc<tcl_registry::registry::EffectiveRegistrySemantics>,
    ) -> bool {
        self.policy == policy
            && (std::sync::Arc::ptr_eq(&self.semantics, semantics)
                || self.names == *semantics.binding_names())
    }
}

fn metadata_slots(
    policy: tcl_syntax::naming::NamePolicyProtocol,
    names: &BTreeSet<String>,
) -> std::collections::HashMap<tcl_core_types::ByteCommandSlot, Option<String>> {
    let mut slots = std::collections::HashMap::new();
    for name in names {
        if let Some(slot) = original_registry_metadata_slot(policy, name) {
            slots
                .entry(slot)
                .and_modify(|row| *row = None)
                .or_insert_with(|| Some(name.clone()));
        }
    }
    slots
}

/// One barrier classifier shared by separately owned Native and vendor source
/// coordinates. It grants schema applicability only, never a live lookup.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CatalogueSourceCell {
    Pass,
    Alternative,
    Barrier,
}
pub(super) fn classify_catalogue_source_cell(
    bindings: &[&MayBinding],
    catalogue: Option<&str>,
) -> CatalogueSourceCell {
    #[cfg(test)]
    if std::env::var_os("TCL_LSP_TRACE_SOURCE_CATALOGUE").is_some() && !bindings.is_empty() {
        eprintln!("SOURCE_CATALOGUE candidate={catalogue:?} cells={bindings:?}");
    }
    let unknown = bindings
        .iter()
        .any(|binding| matches!(binding, MayBinding::Unknown));
    let missing = bindings
        .iter()
        .any(|binding| matches!(binding, MayBinding::Missing));
    let compatible = |binding: &&MayBinding| {
        matches!(binding,MayBinding::Target(target)
        if catalogue.is_some_and(|name| target.registry_backed && target.kind==BindingKind::Builtin
            && target.prepended.is_empty() && target.command==name))
    };
    if bindings.is_empty() {
        return CatalogueSourceCell::Pass;
    }
    if catalogue.is_some() {
        if !unknown && bindings.iter().any(|binding| !compatible(binding)) {
            CatalogueSourceCell::Barrier
        } else if unknown || missing || bindings.iter().any(|binding| !compatible(binding)) {
            CatalogueSourceCell::Alternative
        } else {
            CatalogueSourceCell::Pass
        }
    } else if !unknown && !missing {
        CatalogueSourceCell::Barrier
    } else if unknown
        || bindings
            .iter()
            .any(|binding| !matches!(binding, MayBinding::Missing))
    {
        CatalogueSourceCell::Alternative
    } else {
        CatalogueSourceCell::Pass
    }
}

pub(super) struct OriginalCommandOperands {
    inputs: Vec<(TransitionSubject, SignatureSourceNameInput)>,
    procedure_body: bool,
    namespace_ensure: Option<TransitionSubject>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signature_scan::scope::SignatureSourceNameKey;

    #[test]
    fn original_initial_table_selects_the_registered_handler() {
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = tcl_registry::InvocationDialect::for_version(version);
            let registry =
                tcl_registry::model::ingress::static_context_for(version.dialect_name()).commands();
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
                tcl_lexer::native_script_words_in(image, tcl_lexer::Span::new(0, 12), config)
                    .unwrap();
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
        let dialect = tcl_registry::InvocationDialect::of_point(
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
        let c = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
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
            assert!(!paths.is_empty());
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
            assert!(state.original_definition_key(
                &root, &input, CommandBindingDefinitionKind::Procedure,
            ).is_none());
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
            let dialect = tcl_registry::InvocationDialect::of_profile(registry.profile().unwrap());
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
            let other = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_5)
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
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let registry =
            tcl_registry::model::ingress::static_context_for(version.dialect_name()).commands();
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
        let binding =
            super::super::source_binding_from_original_input(&state, &input, &root).unwrap();
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
            super::super::Arc::make_mut(&mut declared.baseline)
                .declared_commands
                .insert(candidate.slot, "document-proc".to_owned());
            let (document, catalogue) =
                declared.original_advice_candidates_for_input(&root, &input);
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
        super::super::Arc::make_mut(&mut proposal.baseline)
            .catalogue_commands
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
            let dialect = tcl_registry::InvocationDialect::for_version(version);
            let registry =
                tcl_registry::model::ingress::static_context_for(version.dialect_name()).commands();
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
            let dialect = tcl_registry::InvocationDialect::for_version(version);
            let registry =
                tcl_registry::model::ingress::static_context_for(version.dialect_name()).commands();
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
}

pub(super) struct OriginalCommandTargetSelection {
    pub(super) targets: BTreeSet<ResolvedCommandTarget>,
    pub(super) original_prefixes:
        super::BTreeMap<ResolvedCommandTarget, Option<Vec<SignatureSourceNameInput>>>,
    pub(super) may_be_absent: bool,
    pub(super) unknown: bool,
}

impl OriginalCommandTargetSelection {
    fn insert(
        &mut self,
        target: ResolvedCommandTarget,
        inputs: Option<Vec<SignatureSourceNameInput>>,
    ) {
        self.original_prefixes
            .entry(target.clone())
            .and_modify(|previous| {
                if *previous != inputs {
                    *previous = None;
                }
            })
            .or_insert(inputs);
        self.targets.insert(target);
    }
}

fn command_operand_subjects(facts: &InvocationFacts) -> Vec<&TransitionSubject> {
    let mut subjects = Vec::new();
    if let Some(transitions) = facts.state_transitions.declared() {
        for fact in transitions.facts() {
            if let StateTransition::Namespace(
                tcl_registry::NamespaceTransition::Ensure {
                    namespace: tcl_registry::NamespaceTransitionTarget::Named(subject),
                }
                | tcl_registry::NamespaceTransition::Delete {
                    namespace: tcl_registry::NamespaceTransitionTarget::Named(subject),
                },
            ) = &fact.transition
            {
                subjects.push(subject);
            }
            if let StateTransition::Namespace(
                tcl_registry::NamespaceTransition::Export { patterns, .. }
                | tcl_registry::NamespaceTransition::Import { patterns, .. }
                | tcl_registry::NamespaceTransition::Forget { patterns, .. },
            ) = &fact.transition
            {
                subjects.extend(patterns);
            }
            if let StateTransition::Namespace(tcl_registry::NamespaceTransition::SetPath {
                path,
                ..
            }) = &fact.transition
            {
                subjects.push(path);
            }
            let StateTransition::CommandBinding(transition) = &fact.transition else {
                continue;
            };
            match transition {
                CommandBindingTransition::Define { name, .. } => subjects.push(name),
                CommandBindingTransition::Move { from, to } => subjects.extend([from, to]),
                CommandBindingTransition::Delete { interpreter, name } => {
                    subjects.extend(interpreter.as_ref());
                    subjects.push(name);
                }
                CommandBindingTransition::Alias {
                    source_interpreter,
                    alias,
                    target_interpreter,
                    target,
                    arguments,
                    ..
                } => {
                    subjects.extend([source_interpreter, alias, target_interpreter, target]);
                    subjects.extend(arguments);
                }
                CommandBindingTransition::Unknown { .. } => {}
            }
        }
    }
    subjects
}

fn namespace_ensure_subject(facts: &InvocationFacts) -> Option<TransitionSubject> {
    (facts.operation
        == tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::NamespaceEval,
        )
        && facts.analyser_hook == Some(tcl_registry::hooks::AnalyserHookId::NamespaceEval))
    .then(|| {
        let transitions = facts.state_transitions.declared()?;
        let [fact] = transitions.facts() else {
            return None;
        };
        let StateTransition::Namespace(tcl_registry::NamespaceTransition::Ensure {
            namespace: tcl_registry::NamespaceTransitionTarget::Named(subject),
        }) = &fact.transition
        else {
            return None;
        };
        facts
            .arg_roles
            .iter()
            .any(|(index, role)| {
                *role == tcl_registry::ArgRole::NamespaceName
                    && facts.argument_offset.checked_add(usize::from(*index))
                        == subject.argument_index()
            })
            .then(|| subject.clone())
    })
    .flatten()
}

#[derive(Clone, Copy)]
struct OriginalAliasOperation<'a> {
    source_interpreter: &'a TransitionSubject,
    alias: &'a TransitionSubject,
    target_interpreter: &'a TransitionSubject,
    target: &'a TransitionSubject,
    arguments: &'a [TransitionSubject],
    target_lookup: tcl_registry::AliasTargetLookup,
    declaration: u32,
}

impl OriginalCommandOperands {
    pub(super) fn capture(
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Self {
        let subjects = command_operand_subjects(facts);
        let inputs = subjects
            .into_iter()
            .filter_map(|subject| {
                original_operand(native, subject, state, context)
                    .map(|input| (subject.clone(), input))
            })
            .collect();
        let namespace_ensure = namespace_ensure_subject(facts);
        let formal_topology =
            super::formal_topology::from_invocation(native, facts, state, context);
        let procedure_body = matches!(
            facts.procedure_definition,
            Some(tcl_registry::native_procedure::NativeProcedureDefinitionSelection::Valid(_))
        ) && formal_topology.is_some()
            && facts
                .arg_roles
                .iter()
                .filter(|(_, role)| *role == tcl_registry::ArgRole::Body)
                .all(|(index, _)| {
                    super::retained_script_operand(
                        facts.argument_offset + usize::from(*index),
                        native.script_operands(),
                        state,
                        native.segment.span.start(),
                        context.config,
                    )
                    .is_some()
                })
            && facts
                .arg_roles
                .iter()
                .any(|(_, role)| *role == tcl_registry::ArgRole::Body);
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_COMMAND_CAPTURE offset={} facts={:?} procedure={:?} roles={:?} argument_offset={} inputs={:?} formals={} body={} opaque={} namespace={:?} baseline_policy={:?} variable_dialect={:?} variable_policy={:?}",
                native.segment.span.start(),
                facts.operation,
                facts.procedure_definition,
                facts.arg_roles,
                facts.argument_offset,
                inputs,
                formal_topology.is_some(),
                procedure_body,
                state.has_opaque_domain(),
                context.namespace_identity(),
                state.baseline.execution_name_policy,
                state.source_variables.invocation_dialect,
                state.source_variables.execution_name_policy
            );
        }
        Self {
            inputs,
            procedure_body,
            namespace_ensure,
        }
    }

    fn input(&self, subject: &TransitionSubject) -> Option<&SignatureSourceNameInput> {
        let mut selected = self
            .inputs
            .iter()
            .filter(|(owned, _)| owned == subject)
            .map(|(_, input)| input);
        let first = selected.next()?;
        selected.all(|input| input == first).then_some(first)
    }

    /// Retained actual Ensure operand; the Registry ordinal and the complete
    /// original invocation independently supply this producer.
    pub(super) fn namespace_input(
        &self,
        subject: &TransitionSubject,
    ) -> Option<&SignatureSourceNameInput> {
        self.input(subject)
    }

    /// A whole `NamespaceName` operand of the independently selected eval
    /// recipe. A qualifier derived from a command name is a different input.
    pub(super) fn namespace_ensure_input(
        &self,
        subject: &TransitionSubject,
    ) -> Option<&SignatureSourceNameInput> {
        (self.namespace_ensure.as_ref() == Some(subject)).then_some(())?;
        self.input(subject)
    }

    pub(super) fn procedure_key(
        &self,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        current: &SourceNamespaceKey,
    ) -> Option<SourceCommandKey> {
        let mut names = facts
            .state_transitions
            .declared()?
            .command_bindings()
            .filter_map(|transition| match transition {
                CommandBindingTransition::Define {
                    name,
                    kind: CommandBindingDefinitionKind::Procedure,
                } => Some(name),
                _ => None,
            });
        let name = names.next()?;
        if names.next().is_some() || !self.procedure_body {
            return None;
        }
        state.original_definition_key(
            current,
            self.input(name)?,
            CommandBindingDefinitionKind::Procedure,
        )
    }

    fn apply_alias(
        &self,
        state: &mut ModuleCommandBindings,
        operation: OriginalAliasOperation<'_>,
    ) -> Option<()> {
        let OriginalAliasOperation {
            source_interpreter,
            alias,
            target_interpreter,
            target,
            arguments,
            target_lookup,
            declaration,
        } = operation;
        if !self.input(source_interpreter)?.bytes().is_empty() {
            return Some(());
        }
        let root = state.source_root_namespace_key()?;
        let key = state.original_publication_key(
            &root,
            self.input(alias)?,
            super::namespace_slots::PublicationPurpose::Alias,
        )?;
        let binding = if self.input(target_interpreter)?.bytes().is_empty() {
            let target = self.input(target)?;
            let prepended = arguments
                .iter()
                .map(|argument| self.input(argument).map(effective_input))
                .collect::<Option<Vec<_>>>()?;
            MayBinding::Target(ResolvedCommandTarget {
                command: std::str::from_utf8(target.bytes())
                    .unwrap_or_default()
                    .to_owned(),
                prepended,
                registry_backed: false,
                kind: BindingKind::Alias,
                implementation_generation: declaration,
                terminal: false,
                target_lookup,
                token: None,
                implementation_allocation: None,
            })
        } else {
            MayBinding::Unknown
        };
        state.rebound_names.insert(key.clone());
        state.install(key, binding);
        Some(())
    }

    /// One selected operation mutates the canonical command table. Its source
    /// operands never authorize execution, Normal completion or native headers.
    pub(super) fn apply(
        &self,
        state: &mut ModuleCommandBindings,
        transition: &CommandBindingTransition,
        current: &SourceNamespaceKey,
        declaration: u32,
    ) -> Option<()> {
        match transition {
            CommandBindingTransition::Define { name, kind } => {
                let key = state.original_definition_key(current, self.input(name)?, *kind)?;
                if *kind == CommandBindingDefinitionKind::Procedure && !self.procedure_body {
                    return None;
                }
                let label = ModuleCommandBindings::command_key_label(&key).unwrap_or_default();
                let target = ResolvedCommandTarget {
                    command: label,
                    prepended: Vec::new(),
                    registry_backed: false,
                    kind: match kind {
                        CommandBindingDefinitionKind::Procedure => BindingKind::Proc,
                        CommandBindingDefinitionKind::Object => BindingKind::Class,
                        CommandBindingDefinitionKind::Command => BindingKind::Command,
                    },
                    implementation_generation: declaration,
                    terminal: true,
                    target_lookup: tcl_registry::AliasTargetLookup::Global,
                    token: None,
                    implementation_allocation: None,
                };
                state.install(key, MayBinding::Target(target));
                #[cfg(test)]
                if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
                    eprintln!(
                        "ORIGINAL_COMMAND_INSTALLED offset={declaration} bindings={:?}",
                        state.bindings
                    );
                }
            }
            CommandBindingTransition::Move { from, to } => {
                let from = self.input(from)?;
                let to = self.input(to)?;
                let (source, bindings) =
                    state.original_occupied_command_for_input(current, from)?;
                let scope = state.original_namespace_geometry(current, to.policy())?;
                let selected = to
                    .policy()
                    .recipe()
                    .rename_destination_input(scope.context()?, to.bytes())
                    .ok()?;
                if selected.selected().is_empty() {
                    state.remove(source);
                    return Some(());
                }
                let destination = state.original_publication_key(
                    current,
                    to,
                    super::namespace_slots::PublicationPurpose::Rename,
                )?;
                let prior = state.original_bindings_for_key(&destination)?;
                if prior != BTreeSet::from([MayBinding::Missing]) {
                    return None;
                }
                state.replace(source.clone(), BTreeSet::from([MayBinding::Missing]));
                state.rebound_names.insert(source.clone());
                state.rebound_names.insert(destination.clone());
                state.replace(destination, bindings);
            }
            CommandBindingTransition::Delete { interpreter, name } => {
                if let Some(interpreter) = interpreter
                    && !self.input(interpreter)?.bytes().is_empty()
                {
                    return Some(());
                }
                let current = if interpreter.is_some() {
                    state.source_root_namespace_key()?
                } else {
                    current.clone()
                };
                let (key, _) =
                    state.original_occupied_command_for_input(&current, self.input(name)?)?;
                state.remove(key);
            }
            CommandBindingTransition::Alias {
                source_interpreter,
                alias,
                target_interpreter,
                target,
                arguments,
                target_lookup,
            } => {
                self.apply_alias(
                    state,
                    OriginalAliasOperation {
                        source_interpreter,
                        alias,
                        target_interpreter,
                        target,
                        arguments,
                        target_lookup: *target_lookup,
                        declaration,
                    },
                )?;
            }
            CommandBindingTransition::Unknown { .. } => return None,
        }
        Some(())
    }
}

fn effective_input(
    input: &SignatureSourceNameInput,
) -> crate::registry_invocation::EffectiveInvocationWord {
    match std::str::from_utf8(input.bytes()) {
        Ok(text) => crate::registry_invocation::EffectiveInvocationWord::Literal(text.to_owned()),
        Err(_) => {
            crate::registry_invocation::EffectiveInvocationWord::ByteLiteral(input.bytes().into())
        }
    }
}

pub(super) fn original_operand(
    native: SourceNativeInvocation<'_>,
    subject: &TransitionSubject,
    state: &ModuleCommandBindings,
    _context: SourceExecutionContext<'_>,
) -> Option<SignatureSourceNameInput> {
    native
        .original_variable_operands
        .input(subject.argument_index()?, &state.source_variables)
        .cloned()
}

enum OriginalLookupSlot {
    Retained(SourceCommandKey),
    MissingHolder,
}

impl OriginalLookupSlot {
    fn key(self) -> Option<SourceCommandKey> {
        match self {
            Self::Retained(key) => Some(key),
            Self::MissingHolder => None,
        }
    }
}

#[derive(Clone, Copy)]
struct OriginalTargetLookupContext<'a> {
    policy: tcl_syntax::naming::NamePolicyProtocol,
    namespace: &'a SourceNamespaceKey,
    lookup: super::CommandTargetLookup,
}

type OriginalAliasLoopPaths = Vec<Vec<(tcl_core_types::ByteCommandSlot, Option<SourceCommandKey>)>>;

impl ModuleCommandBindings {
    fn original_definition_key(
        &self,
        current: &SourceNamespaceKey,
        input: &SignatureSourceNameInput,
        kind: CommandBindingDefinitionKind,
    ) -> Option<SourceCommandKey> {
        let scope = if input.bytes().starts_with(b"::") {
            SignatureNamespaceScope::root(Some(input.policy()))
        } else {
            self.original_namespace_geometry(current, input.policy())?
        };
        let slot = super::source_command_world::publication_slot(&scope, input, kind)?;
        self.original_command_key_for_slot(&slot, input.policy())
    }

    /// Conditional declaration namespace geometry from a genuine original
    /// operand. The original observation/frame owner must admit the caller.
    /// Current table existence, installation and Normal completion remain
    /// separate; this facade must never be used for an operation transfer.
    pub(super) fn original_conditional_procedure_key(
        &self,
        current: &SourceNamespaceKey,
        input: &SignatureSourceNameInput,
    ) -> Option<SourceCommandKey> {
        let policy = input.policy();
        if self.baseline.execution_name_policy?.native_recipe()? != policy {
            return None;
        }
        if let SourceNamespaceKey::Native(context) = current {
            let entry = self.baseline.native_entry.as_ref()?;
            if entry.command_name_policy()? != policy
                || entry.retained_namespace_context(context.token).ok()? != *context
            {
                return None;
            }
        }
        let scope = if input.bytes().starts_with(b"::") || current.is_root() {
            SignatureNamespaceScope::root(Some(policy))
        } else {
            self.original_command_world
                .conditional_scope(current, policy)?
        };
        let slot = super::source_command_world::publication_slot(
            &scope,
            input,
            tcl_registry::CommandBindingDefinitionKind::Procedure,
        )?;
        let simple = slot.simple;
        let key = |namespace| SourceCommandKey::slot(namespace, simple.clone());
        let holder = match policy.recipe() {
            tcl_syntax::naming::NativeNameProtocol::C(_) => {
                SignatureNamespaceScope::C(slot.namespace)
            }
            tcl_syntax::naming::NativeNameProtocol::Jim084 => return Some(key(current.clone())),
        };
        if holder == scope && !input.bytes().starts_with(b"::") {
            return Some(key(current.clone()));
        }
        if matches!(&holder, SignatureNamespaceScope::C(path) if path.is_root()) {
            return self.source_root_namespace_key().map(key);
        }
        self.original_command_world
            .conditional_namespace_for_scope(&holder, policy)
            .map(key)
    }

    pub(super) fn original_publication_key(
        &self,
        current: &SourceNamespaceKey,
        input: &SignatureSourceNameInput,
        purpose: super::namespace_slots::PublicationPurpose,
    ) -> Option<SourceCommandKey> {
        let scope = if input.bytes().starts_with(b"::") {
            SignatureNamespaceScope::root(Some(input.policy()))
        } else {
            self.original_namespace_geometry(current, input.policy())?
        };
        let recipe = input.policy().recipe();
        let slot = match purpose {
            super::namespace_slots::PublicationPurpose::Procedure => {
                recipe.command_publication_slot(scope.context()?, input.bytes())
            }
            super::namespace_slots::PublicationPurpose::Rename => {
                recipe.rename_destination_slot(scope.context()?, input.bytes())
            }
            super::namespace_slots::PublicationPurpose::Alias => {
                recipe.alias_publication_slot(scope.context()?, input.bytes())
            }
            super::namespace_slots::PublicationPurpose::Command => {
                recipe.command_c_api_publication_slot(scope.context()?, input.bytes())
            }
        }
        .ok()?;
        if matches!(
            purpose,
            super::namespace_slots::PublicationPurpose::Procedure
        ) && let tcl_syntax::naming::NativeNameProtocol::C(version) = recipe
        {
            tcl_registry::native_procedure::procedure_name_creation_error(
                tcl_registry::InvocationDialect::for_version(version),
                slot.namespace.is_root(),
                slot.simple.as_bytes(),
            )?
            .ok()?;
        }
        self.original_command_key_for_slot(&slot, input.policy())
    }

    /// Select a modeled table identity from exact retained geometry. Registry
    /// ASCII metadata is a separate baseline purpose, never a source producer.
    pub(super) fn original_command_key_for_slot(
        &self,
        slot: &tcl_core_types::ByteCommandSlot,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<SourceCommandKey> {
        let mut world = (*self.original_command_world).clone();
        if world.select_policy(self)? != policy {
            return None;
        }
        let mut namespaces = self.namespaces.iter().filter(|namespace| {
            world
                .scope(namespace, policy)
                .is_some_and(|scope| match scope {
                    SignatureNamespaceScope::C(path) => path == slot.namespace,
                    SignatureNamespaceScope::Jim(_) => {
                        slot.namespace.is_root()
                            && **namespace == self.source_root_namespace_key().unwrap_or_default()
                    }
                    SignatureNamespaceScope::Symbolic(_) => false,
                })
        });
        let namespace = namespaces.next()?.clone();
        if namespaces.next().is_some() {
            return None;
        }
        Some(SourceCommandKey::slot(namespace, slot.simple.clone()))
    }

    pub(super) fn original_command_paths_for_input(
        &self,
        current: &SourceNamespaceKey,
        input: &SignatureSourceNameInput,
    ) -> Option<Vec<Vec<SourceCommandKey>>> {
        self.original_command_paths_for_bytes(current, input.bytes(), input.policy())
    }

    /// Alias loop topology may end at a known missing target namespace.
    /// None for a cell key means that modeled holder is absent; it grants no
    /// namespace, callable target, or later dispatch. Unknown geometry declines.
    pub(super) fn original_alias_loop_paths_for_input(
        &self,
        current: &SourceNamespaceKey,
        input: &SignatureSourceNameInput,
    ) -> Option<OriginalAliasLoopPaths> {
        self.original_command_slot_paths_for_bytes(current, input.bytes(), input.policy())?
            .into_iter()
            .map(|path| {
                path.into_iter()
                    .map(|slot| {
                        let key = self
                            .original_lookup_key_for_slot(&slot, input.policy())?
                            .key();
                        Some((slot, key))
                    })
                    .collect()
            })
            .collect()
    }

    /// Explicit fixed Registry metadata proposal. It shares the original
    /// table path geometry without manufacturing an original source operand.
    pub(super) fn original_registry_command_paths(
        &self,
        current: &SourceNamespaceKey,
        name: &str,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<Vec<Vec<SourceCommandKey>>> {
        if !name.is_ascii() || name.as_bytes().contains(&0) {
            return None;
        }
        self.original_command_paths_for_bytes(current, name.as_bytes(), policy)
    }

    fn original_command_paths_for_bytes(
        &self,
        current: &SourceNamespaceKey,
        bytes: &[u8],
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<Vec<Vec<SourceCommandKey>>> {
        self.original_command_slot_paths_for_bytes(current, bytes, policy)?
            .into_iter()
            .map(|path| {
                let mut retained = Vec::new();
                for slot in path {
                    match self.original_lookup_key_for_slot(&slot, policy)? {
                        OriginalLookupSlot::Retained(key) => retained.push(key),
                        OriginalLookupSlot::MissingHolder => {}
                    }
                }
                Some(retained)
            })
            .collect()
    }

    fn original_command_slot_paths_for_bytes(
        &self,
        current: &SourceNamespaceKey,
        bytes: &[u8],
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<Vec<Vec<tcl_core_types::ByteCommandSlot>>> {
        use tcl_syntax::naming::{NativeNameContext, NativeNameProtocol, NativeNameQualification};
        let mut world = (*self.original_command_world).clone();
        if world.select_policy(self)? != policy || self.has_opaque_domain() {
            return None;
        }
        let scope = if bytes.starts_with(b"::") {
            SignatureNamespaceScope::root(Some(policy))
        } else {
            world.scope(current, policy)?
        };
        let recipe = policy.recipe();
        let mut paths = Vec::new();
        match recipe {
            NativeNameProtocol::Jim084 => {
                paths.push(
                    recipe
                        .jim_command_lookup_keys(scope.context()?, bytes)
                        .ok()?
                        .into_iter()
                        .map(|simple| tcl_core_types::ByteCommandSlot {
                            namespace: tcl_core_types::ByteNamespacePath::root(),
                            simple,
                        })
                        .collect::<Vec<_>>(),
                );
            }
            NativeNameProtocol::C(version) => {
                let absolute = recipe
                    .command_lookup_input(scope.context()?, bytes)
                    .ok()?
                    .qualification()
                    == NativeNameQualification::Absolute;
                if !absolute && self.unknown_lookup_namespaces.contains(current) {
                    return None;
                }
                let local = (!absolute)
                    .then(|| recipe.command_lookup_slot(scope.context()?, bytes).ok())
                    .flatten();
                let occupied = |slot: &tcl_core_types::ByteCommandSlot| {
                    self.original_command_key_for_slot(slot, policy)
                        .and_then(|key| self.original_bindings_for_key(&key))
                        .is_some_and(|bindings| {
                            !bindings.is_empty() && !bindings.contains(&MayBinding::Missing)
                        })
                };
                // Later namespace-path uncertainty is consulted only after a
                // genuine current cell permits fallback. It cannot spoil an
                // independently occupied earlier cell.
                if let Some(local) = &local
                    && occupied(local)
                {
                    return Some(vec![vec![local.clone()]]);
                }
                if !absolute
                    && version.has_namespace_path()
                    && self.unknown_namespace_paths.contains(current)
                {
                    return None;
                }
                let empty = BTreeSet::from([Vec::new()]);
                for path in self.namespace_paths.get(current).unwrap_or(&empty) {
                    let mut slots = Vec::new();
                    let mut stopped = false;
                    if let Some(local) = &local {
                        slots.push(local.clone());
                        if version.has_namespace_path() {
                            for namespace in path {
                                if !self.namespaces.contains(namespace) {
                                    continue;
                                }
                                let selected = world.scope(namespace, policy)?;
                                let slot = recipe
                                    .command_lookup_slot(selected.context()?, bytes)
                                    .ok()?;
                                stopped = occupied(&slot);
                                if !slots.contains(&slot) {
                                    slots.push(slot);
                                }
                                if stopped {
                                    break;
                                }
                            }
                        }
                    }
                    if !stopped {
                        let root = recipe
                            .command_lookup_slot(NativeNameContext::root(), bytes)
                            .ok()?;
                        if !slots.contains(&root) {
                            slots.push(root);
                        }
                    }
                    paths.push(slots);
                }
            }
        }
        Some(paths)
    }

    fn original_catalogue_slot_for_key(
        &self,
        key: &SourceCommandKey,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<tcl_core_types::ByteCommandSlot> {
        match key {
            SourceCommandKey::Authored(name) => original_registry_metadata_slot(policy, name),
            SourceCommandKey::Slot { namespace, simple } => {
                let scope = self
                    .original_command_world
                    .retained_scope_for_source_advice(namespace, policy)?;
                match scope {
                    SignatureNamespaceScope::C(namespace) => {
                        Some(tcl_core_types::ByteCommandSlot {
                            namespace,
                            simple: simple.clone(),
                        })
                    }
                    SignatureNamespaceScope::Jim(_) => Some(tcl_core_types::ByteCommandSlot {
                        namespace: tcl_core_types::ByteNamespacePath::root(),
                        simple: simple.clone(),
                    }),
                    SignatureNamespaceScope::Symbolic(_) => None,
                }
            }
        }
    }

    pub(super) fn original_catalogue_bindings_for_slot(
        &self,
        slot: &tcl_core_types::ByteCommandSlot,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Vec<&MayBinding> {
        self.bindings
            .iter()
            .filter(|(key, _)| {
                self.original_catalogue_slot_for_key(key, policy).as_ref() == Some(slot)
            })
            .flat_map(|(_, alternatives)| alternatives.iter())
            .collect()
    }

    fn original_catalogue_path_candidate(
        &self,
        current: &SourceNamespaceKey,
        path: &[tcl_core_types::ByteCommandSlot],
        policy: tcl_syntax::naming::NamePolicyProtocol,
        obligations: &mut Vec<super::OriginalCatalogueSourceObligation>,
    ) -> Option<super::DeclaredCommandCandidate> {
        use super::OriginalCatalogueSourceObligation as Obligation;
        let mut selected = None;
        for slot in path {
            let mut candidates = self.baseline.catalogue_commands.iter().filter(|(name, _)| {
                original_registry_metadata_slot(policy, name).as_ref() == Some(slot)
            });
            let candidate = candidates.next();
            if candidates.next().is_some() {
                return None;
            }
            let bindings = self.original_catalogue_bindings_for_slot(slot, policy);
            match classify_catalogue_source_cell(
                &bindings,
                candidate.map(|(_, name)| name.as_str()),
            ) {
                CatalogueSourceCell::Barrier => return None,
                CatalogueSourceCell::Alternative if candidate.is_some() => {
                    obligations.push(Obligation::SelectedBindingAlternative(slot.clone()));
                }
                CatalogueSourceCell::Alternative => {
                    obligations.push(Obligation::EarlierBindingAlternative(slot.clone()));
                }
                CatalogueSourceCell::Pass => {}
            }
            if self
                .baseline
                .declared_commands
                .keys()
                .any(|name| original_registry_metadata_slot(policy, name).as_ref() == Some(slot))
            {
                return None;
            }
            if let Some((metadata, name)) = candidate {
                if self.original_command_key_for_slot(slot, policy).is_none() {
                    obligations.push(Obligation::UnretainedHolder(slot.clone()));
                }
                selected = Some(super::DeclaredCommandCandidate {
                    name: name.clone(),
                    slot: metadata.clone(),
                    lookup_namespace: current.display().unwrap_or_default(),
                });
                break;
            }
        }
        selected
    }

    /// Conditional source catalogue matching is separate from closed lookup.
    /// Existing exact cells, tombstones and document overrides remain barriers;
    /// unknown state is retained on the returned readonly source schema.
    pub(super) fn original_catalogue_source_candidate(
        &self,
        current: &SourceNamespaceKey,
        namespace: &SignatureNamespaceScope,
        input: &SignatureSourceNameInput,
    ) -> Option<(
        super::DeclaredCommandCandidate,
        Vec<Vec<tcl_core_types::ByteCommandSlot>>,
        Vec<super::OriginalCatalogueSourceObligation>,
    )> {
        use super::OriginalCatalogueSourceObligation as Obligation;
        let policy = input.policy();
        if self
            .baseline
            .execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
            != Some(policy)
        {
            return None;
        }
        let mut obligations = Vec::new();
        if self.has_opaque_domain()
            || self.baseline.unknown_entry
            || self.unknown_lookup_namespaces.contains(current)
        {
            obligations.push(Obligation::UnknownCurrentLookup);
        }
        if self
            .original_command_world
            .retained_scope_for_source_advice(current, policy)
            .as_ref()
            != Some(namespace)
        {
            obligations.push(Obligation::UnknownExecutingNamespace);
        }
        let paths = if let Some(paths) =
            self.original_command_slot_paths_for_bytes(current, input.bytes(), policy)
        {
            paths
        } else {
            // A missing path owner cannot donate an empty namespace path. This
            // bounded schema accepts only independently known empty path rows.
            if self.unknown_namespace_paths.contains(current)
                || self
                    .namespace_paths
                    .get(current)
                    .is_some_and(|paths| paths.iter().any(|path| !path.is_empty()))
            {
                return None;
            }
            obligations.push(Obligation::UnknownLookupCoordinates);
            vec![
                crate::signature_scan::scope::SignatureSourceLookup::from_input(
                    namespace.clone(),
                    input,
                )?
                .candidates()?,
            ]
        };
        let mut unanimous = None;
        for path in &paths {
            let selected =
                self.original_catalogue_path_candidate(current, path, policy, &mut obligations)?;
            if unanimous.as_ref().is_some_and(|prior| prior != &selected) {
                return None;
            }
            unanimous = Some(selected);
        }
        Some((unanimous?, paths, obligations))
    }

    /// Retained source-only namespace geometry under the independently selected
    /// naming policy. Authored labels never supply bytes or native incarnation.
    pub(crate) fn original_namespace_geometry(
        &self,
        namespace: &SourceNamespaceKey,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<SignatureNamespaceScope> {
        if !self.namespaces.contains(namespace) {
            return None;
        }
        let mut world = (*self.original_command_world).clone();
        (world.select_policy(self)? == policy).then_some(())?;
        world.scope(namespace, policy)
    }

    /// Readonly catalogue or document advice at genuine original geometry.
    /// A missing holder may support an unloaded catalogue candidate; it never
    /// supplies a runtime namespace, table entry or implementation.
    pub(super) fn original_advice_candidates_for_input(
        &self,
        current: &SourceNamespaceKey,
        input: &SignatureSourceNameInput,
    ) -> (
        Option<super::DeclaredCommandCandidate>,
        Option<super::DeclaredCommandCandidate>,
    ) {
        if !input.is_current(&self.source_variables) {
            return (None, None);
        }
        self.original_advice_candidates_for_bytes(current, input.bytes(), input.policy())
    }

    /// Explicit future/document declaration metadata at an original operand
    /// and genuine namespace identity. The subset supplies no runtime binding.
    pub(super) fn original_declared_candidate_from_input(
        &self,
        current: &SourceNamespaceKey,
        input: &SignatureSourceNameInput,
        declarations: &super::BTreeMap<String, String>,
    ) -> Option<super::DeclaredCommandCandidate> {
        input.is_current(&self.source_variables).then_some(())?;
        let paths =
            self.original_command_slot_paths_for_bytes(current, input.bytes(), input.policy())?;
        self.original_advice_candidate_from_paths(
            current,
            &paths,
            input.policy(),
            declarations,
            false,
        )
    }

    /// Fixed metadata proposals retain their authored provenance. No original
    /// source word or value is reconstructed from this compatibility argument.
    pub(super) fn original_advice_candidates_for_metadata(
        &self,
        current: &SourceNamespaceKey,
        name: &str,
    ) -> (
        Option<super::DeclaredCommandCandidate>,
        Option<super::DeclaredCommandCandidate>,
    ) {
        if !name.is_ascii() || name.as_bytes().contains(&0) {
            return (None, None);
        }
        let mut world = (*self.original_command_world).clone();
        let Some(policy) = world.select_policy(self) else {
            return (None, None);
        };
        self.original_advice_candidates_for_bytes(current, name.as_bytes(), policy)
    }

    fn original_advice_candidates_for_bytes(
        &self,
        current: &SourceNamespaceKey,
        bytes: &[u8],
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> (
        Option<super::DeclaredCommandCandidate>,
        Option<super::DeclaredCommandCandidate>,
    ) {
        let Some(paths) = self.original_command_slot_paths_for_bytes(current, bytes, policy) else {
            return (None, None);
        };
        (
            self.original_advice_candidate_from_paths(
                current,
                &paths,
                policy,
                &self.baseline.declared_commands,
                false,
            ),
            self.original_advice_candidate_from_paths(
                current,
                &paths,
                policy,
                &self.baseline.catalogue_commands,
                true,
            ),
        )
    }

    fn original_advice_candidate_from_paths(
        &self,
        current: &SourceNamespaceKey,
        paths: &[Vec<tcl_core_types::ByteCommandSlot>],
        policy: tcl_syntax::naming::NamePolicyProtocol,
        declarations: &super::BTreeMap<String, String>,
        catalogue: bool,
    ) -> Option<super::DeclaredCommandCandidate> {
        let mut unanimous = None;
        for path in paths {
            let mut selected = None;
            for slot in path {
                let key = self.original_lookup_key_for_slot(slot, policy)?.key();
                if key
                    .as_ref()
                    .is_some_and(|key| self.bindings.contains_key(key))
                {
                    return None;
                }
                let metadata_slot = |name: &str| {
                    original_registry_metadata_slot(policy, name).as_ref() == Some(slot)
                };
                if self
                    .bindings
                    .keys()
                    .any(|key| key.authored_spelling().is_some_and(metadata_slot))
                {
                    return None;
                }
                if catalogue
                    && self
                        .baseline
                        .declared_commands
                        .keys()
                        .any(|name| metadata_slot(name))
                {
                    return None;
                }
                let mut matching = declarations.iter().filter(|(name, _)| metadata_slot(name));
                if let Some((metadata, name)) = matching.next() {
                    if matching.next().is_some() {
                        return None;
                    }
                    selected = Some(super::DeclaredCommandCandidate {
                        name: name.clone(),
                        lookup_namespace: current.display().unwrap_or_default(),
                        slot: metadata.clone(),
                    });
                    break;
                }
                if let Some(key) = key {
                    let bindings = self.original_bindings_for_key(&key)?;
                    if bindings != BTreeSet::from([MayBinding::Missing]) {
                        return None;
                    }
                }
            }
            let selected = selected?;
            if unanimous
                .as_ref()
                .is_some_and(|previous| previous != &selected)
            {
                return None;
            }
            unanimous = Some(selected);
        }
        unanimous
    }

    // None is unknown geometry; MissingHolder is a known absent modelled holder.
    // Readonly lookup may omit a known absent holder. Publication still
    // requires original_command_key_for_slot and its actual retained owner.
    fn original_lookup_key_for_slot(
        &self,
        slot: &tcl_core_types::ByteCommandSlot,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<OriginalLookupSlot> {
        if let Some(key) = self.original_command_key_for_slot(slot, policy) {
            if let SourceCommandKey::Slot { namespace, .. } = &key
                && self.unknown_lookup_namespaces.contains(namespace)
            {
                return None;
            }
            return Some(OriginalLookupSlot::Retained(key));
        }
        let mut world = (*self.original_command_world).clone();
        if world.select_policy(self)? != policy || self.baseline.unknown_entry {
            return None;
        }
        for namespace in self.namespaces.iter() {
            let matches = match world.scope(namespace, policy)? {
                SignatureNamespaceScope::C(path) => path == slot.namespace,
                SignatureNamespaceScope::Jim(_) => slot.namespace.is_root(),
                SignatureNamespaceScope::Symbolic(_) => return None,
            };
            if matches {
                return None;
            }
        }
        Some(OriginalLookupSlot::MissingHolder)
    }

    /// Actual target selected by an explicit fixed Registry name proposal.
    /// Original source/value producers remain separate. Alias wrappers cannot
    /// supply the immutable direct worker, even when their bytes agree.
    pub(super) fn original_registry_metadata_target(
        &self,
        current: &SourceNamespaceKey,
        name: &str,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<super::ResolvedCommandTarget> {
        let paths = self.original_registry_command_paths(current, name, policy)?;
        let mut unanimous = None;
        for path in paths {
            let mut selected = None;
            let mut absent = true;
            for key in path {
                let bindings = self.original_bindings_for_key(&key)?;
                if bindings.is_empty() {
                    return None;
                }
                for binding in &bindings {
                    let implementations = match binding {
                        MayBinding::Target(target) => vec![target],
                        MayBinding::Imported(imported) => self
                            .objects
                            .get(&imported.origin)?
                            .iter()
                            .map(|implementation| match implementation {
                                MayBinding::Target(target) => Some(target),
                                _ => None,
                            })
                            .collect::<Option<Vec<_>>>()?,
                        MayBinding::Missing => continue,
                        MayBinding::Unknown => return None,
                    };
                    if implementations.is_empty() {
                        return None;
                    }
                    for target in implementations {
                        if !target.terminal
                            || !target.registry_backed
                            || target.kind == super::BindingKind::Alias
                            || !target.prepended.is_empty()
                        {
                            return None;
                        }
                        if selected.as_ref().is_some_and(|previous| previous != target) {
                            return None;
                        }
                        selected = Some(target.clone());
                    }
                }
                if !bindings.contains(&MayBinding::Missing) {
                    absent = false;
                    break;
                }
            }
            if absent {
                return None;
            }
            let selected = selected?;
            if unanimous
                .as_ref()
                .is_some_and(|previous| previous != &selected)
            {
                return None;
            }
            unanimous = Some(selected);
        }
        unanimous
    }

    /// Readonly pre-fallback presence for a genuine original name. Known
    /// absent namespace holders can support absence advice but never produce
    /// a table identity or dispatch target.
    pub(super) fn original_slot_presence_for_input(
        &self,
        input: &SignatureSourceNameInput,
        current: &SourceNamespaceKey,
    ) -> super::SourceCommandSlotPresence {
        use super::SourceCommandSlotPresence as Presence;
        if !input.is_current(&self.source_variables)
            || self.baseline.unknown_entry
            || (!input.bytes().starts_with(b"::")
                && matches!(current, SourceNamespaceKey::Authored(_))
                && !self.source_variables.namespace_known)
        {
            return Presence::Unknown;
        }
        let Some(paths) =
            self.original_command_slot_paths_for_bytes(current, input.bytes(), input.policy())
        else {
            return Presence::Unknown;
        };
        self.original_slot_presence_in_paths(paths, input.policy())
    }

    fn original_slot_presence_in_paths(
        &self,
        paths: Vec<Vec<tcl_core_types::ByteCommandSlot>>,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> super::SourceCommandSlotPresence {
        use super::SourceCommandSlotPresence as Presence;
        let mut present = false;
        let mut absent = false;
        for path in paths {
            let mut fallthrough = true;
            for slot in path {
                let Some(key) = self.original_lookup_key_for_slot(&slot, policy) else {
                    return Presence::Unknown;
                };
                let Some(key) = key.key() else {
                    continue;
                };
                let Some(bindings) = self.original_bindings_for_key(&key) else {
                    return Presence::Unknown;
                };
                if bindings.is_empty() || bindings.contains(&MayBinding::Unknown) {
                    return Presence::Unknown;
                }
                present |= bindings.iter().any(|binding| {
                    matches!(binding, MayBinding::Target(_) | MayBinding::Imported(_))
                });
                fallthrough = bindings.contains(&MayBinding::Missing);
                if !fallthrough {
                    break;
                }
            }
            absent |= fallthrough;
        }
        match (present, absent) {
            (true, false) => Presence::Present,
            (false, true) => Presence::Absent,
            (true, true) => Presence::MayPresent,
            (false, false) => Presence::Unknown,
        }
    }

    /// Readonly presence for a sealed original expression function-name value.
    /// The source owner separately closes operands, observers and pool effects.
    /// No command-head word or runtime function invocation is manufactured.
    pub(super) fn original_function_diagnostic_presence(
        &self,
        current: &SourceNamespaceKey,
        name: &tcl_registry::mathfunc::NativeExpressionFunctionCommandName,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> super::SourceCommandSlotPresence {
        // Proof: naming.expression.original-function-navigation
        // docs/design/analysis/name-resolution-proofs/original-function-navigation.md
        use super::SourceCommandSlotPresence as Presence;
        if name.protocol() != policy.recipe()
            || self
                .baseline
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                != Some(policy)
            || self.baseline.unknown_entry
            || (!self.source_variables.namespace_known
                && matches!(current, SourceNamespaceKey::Authored(_)))
        {
            return Presence::Unknown;
        }
        let Some(paths) =
            self.original_command_slot_paths_for_bytes(current, name.command_bytes(), policy)
        else {
            return Presence::Unknown;
        };
        let qualifier_retained = !paths.is_empty()
            && paths.iter().all(|path| {
                path.iter().any(|slot| {
                    self.original_lookup_key_for_slot(slot, policy)
                        .is_some_and(|key| matches!(key, OriginalLookupSlot::Retained(_)))
                })
            });
        let presence = self.original_slot_presence_in_paths(paths, policy);
        self.original_diagnostic_presence_after_absence(
            current,
            policy,
            presence,
            qualifier_retained,
        )
    }

    pub(super) fn original_diagnostic_presence_after_absence(
        &self,
        current: &SourceNamespaceKey,
        policy: tcl_syntax::naming::NamePolicyProtocol,
        presence: super::SourceCommandSlotPresence,
        qualifier_retained: bool,
    ) -> super::SourceCommandSlotPresence {
        use super::SourceCommandSlotPresence as Presence;
        if presence != Presence::Absent {
            return presence;
        }
        if !qualifier_retained
            || self.unknown_lookup_namespaces.contains(current)
            || self.namespace_unknown_handlers.contains(current)
        {
            return Presence::Unknown;
        }
        let Some(fallback) = self.baseline.dialect.and_then(|dialect| {
            tcl_registry::command_lookup::native_lookup_fallback_policy(
                dialect,
                tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
            )
        }) else {
            return Presence::Unknown;
        };
        let Some(paths) =
            self.original_registry_command_paths(current, fallback.default_handler, policy)
        else {
            return Presence::Unknown;
        };
        if paths.is_empty() {
            return Presence::Unknown;
        }
        for handler in paths.into_iter().flatten() {
            let Some(expected) = self.original_initial_bindings_for_key(&handler) else {
                return Presence::Unknown;
            };
            let Some(actual) = self.original_bindings_for_key(&handler) else {
                return Presence::Unknown;
            };
            if actual != expected && actual != BTreeSet::from([MayBinding::Missing]) {
                return Presence::Unknown;
            }
        }
        presence
    }

    /// Each original lookup path must reach a retained qualifier before an
    /// absent command can become a diagnostic. Missing holders grant no
    /// runtime namespace or fallback identity.
    pub(super) fn original_diagnostic_qualifier_is_retained(
        &self,
        input: &SignatureSourceNameInput,
        current: &SourceNamespaceKey,
    ) -> bool {
        self.original_command_slot_paths_for_bytes(current, input.bytes(), input.policy())
            .is_some_and(|paths| {
                !paths.is_empty()
                    && paths.iter().all(|path| {
                        path.iter().any(|slot| {
                            self.original_lookup_key_for_slot(slot, input.policy())
                                .is_some_and(|key| matches!(key, OriginalLookupSlot::Retained(_)))
                        })
                    })
            })
    }

    /// Actual binding alternatives for one retained original byte key. Fixed
    /// ASCII Registry metadata is selected separately from source publication;
    /// an earlier occupied builtin therefore blocks a later source fallback.
    pub(super) fn original_bindings_for_key(
        &self,
        key: &SourceCommandKey,
    ) -> Option<BTreeSet<MayBinding>> {
        if let Some(bindings) = self.bindings.get(key) {
            return Some(bindings.clone());
        }
        self.original_baseline_bindings_for_key(key, true)
    }

    pub(super) fn original_initial_bindings_for_key(
        &self,
        key: &SourceCommandKey,
    ) -> Option<BTreeSet<MayBinding>> {
        if let Some(bindings) = self.original_entry_bindings.get(key) {
            return Some(bindings.clone());
        }
        self.original_baseline_bindings_for_key(key, false)
    }

    fn original_baseline_bindings_for_key(
        &self,
        key: &SourceCommandKey,
        retained_authored_changes: bool,
    ) -> Option<BTreeSet<MayBinding>> {
        #[cfg(any(test, debug_assertions))]
        let _timing = BaselineLookupTiming::at_query();
        let SourceCommandKey::Slot { namespace, simple } = key else {
            return None;
        };
        let mut world = (*self.original_command_world).clone();
        let policy = world.select_policy(self)?;
        let scope = world.scope(namespace, policy)?;
        let selected = match policy.recipe() {
            tcl_syntax::naming::NativeNameProtocol::C(_) => {
                let SignatureNamespaceScope::C(path) = scope else {
                    return None;
                };
                tcl_core_types::ByteCommandSlot {
                    namespace: path,
                    simple: simple.clone(),
                }
            }
            tcl_syntax::naming::NativeNameProtocol::Jim084 => tcl_core_types::ByteCommandSlot {
                namespace: tcl_core_types::ByteNamespacePath::root(),
                simple: simple.clone(),
            },
        };
        // Native entry command rows already occupy exact keys. An absent row
        // cannot gain a Registry implementation from authored metadata.
        if self.baseline.native_entry.is_some() {
            return Some(BTreeSet::from([MayBinding::Missing]));
        }
        let index = self
            .baseline
            .metadata_slots
            .get_or_init(|| RegistryMetadataSlotIndex::new(policy, &self.baseline.semantics));
        let name = if let Some(index) = index
            .as_ref()
            .filter(|index| index.matches(policy, &self.baseline.semantics))
        {
            let Some(row) = index.slots.get(&selected) else {
                return Some(BTreeSet::from([MayBinding::Missing]));
            };
            row.as_ref()?
        } else {
            let mut metadata = self
                .baseline
                .semantics
                .binding_names()
                .iter()
                .filter(|name| {
                    original_registry_metadata_slot(policy, name).as_ref() == Some(&selected)
                });
            let Some(name) = metadata.next() else {
                return Some(BTreeSet::from([MayBinding::Missing]));
            };
            if metadata.next().is_some() {
                return None;
            }
            name
        };
        let authored = SourceCommandKey::from(name.clone());
        if retained_authored_changes && let Some(bindings) = self.bindings.get(&authored) {
            return Some(bindings.clone());
        }
        Some(Self::unmodified_bindings(
            &authored,
            self.baseline.semantics.binding_names(),
        ))
    }

    /// Existing modeled keys share one binding projection. Original byte
    /// cells use retained namespace geometry and independently selected
    /// baseline metadata; explicitly authored keys retain their legacy role.
    pub(super) fn binding_alternatives(
        &self,
        key: &(impl super::CommandKeyQuery + ?Sized),
    ) -> BTreeSet<MayBinding> {
        let key = key.command_key();
        if matches!(key.as_ref(), SourceCommandKey::Slot { .. }) {
            return self
                .original_bindings_for_key(key.as_ref())
                .unwrap_or_else(|| BTreeSet::from([MayBinding::Unknown]));
        }
        self.bindings.get(key.as_ref()).cloned().unwrap_or_else(|| {
            Self::unmodified_bindings(key.as_ref(), self.baseline.semantics.binding_names())
        })
    }

    /// Candidate table cells that can win one of the genuine namespace paths.
    /// Missing alternatives continue to later cells; occupied cells stop that
    /// path. This projection retains original table keys for compiler owners.
    pub(super) fn original_command_keys_for_input(
        &self,
        current: &SourceNamespaceKey,
        input: &SignatureSourceNameInput,
    ) -> Option<Vec<SourceCommandKey>> {
        let mut keys = Vec::new();
        for path in self.original_command_paths_for_input(current, input)? {
            for key in path {
                let bindings = self.original_bindings_for_key(&key)?;
                if !keys.contains(&key) {
                    keys.push(key);
                }
                if !bindings.contains(&MayBinding::Missing) {
                    break;
                }
            }
        }
        Some(keys)
    }

    /// Actual occupied raw cell for one original operand. Unlike the compiler
    /// traversal, earlier proven Missing cells are not returned as targets.
    /// Every possible path must reach the same complete current cell; alias
    /// resolution and imported implementation selection remain separate.
    pub(super) fn original_occupied_command_for_input(
        &self,
        current: &SourceNamespaceKey,
        input: &SignatureSourceNameInput,
    ) -> Option<(SourceCommandKey, BTreeSet<MayBinding>)> {
        if !input.is_current(&self.source_variables) || self.has_opaque_domain() {
            return None;
        }
        self.original_occupied_command_in_paths(
            self.original_command_paths_for_input(current, input)?,
        )
    }

    /// Current occupied cell for explicit fixed Registry metadata. This retains
    /// table occupancy only; bootstrap roles and handlers are independent.
    pub(super) fn original_occupied_registry_metadata_command(
        &self,
        current: &SourceNamespaceKey,
        name: &str,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<(SourceCommandKey, BTreeSet<MayBinding>)> {
        if self.has_opaque_domain() {
            return None;
        }
        self.original_occupied_command_in_paths(
            self.original_registry_command_paths(current, name, policy)?,
        )
    }

    fn original_occupied_command_in_paths(
        &self,
        paths: Vec<Vec<SourceCommandKey>>,
    ) -> Option<(SourceCommandKey, BTreeSet<MayBinding>)> {
        let mut unanimous = None;
        for path in paths {
            let mut selected = None;
            for key in path {
                let bindings = self.original_bindings_for_key(&key)?;
                if bindings == BTreeSet::from([MayBinding::Missing]) {
                    continue;
                }
                if bindings.is_empty()
                    || bindings.contains(&MayBinding::Missing)
                    || bindings.contains(&MayBinding::Unknown)
                {
                    return None;
                }
                selected = Some((key, bindings));
                break;
            }
            let selected = selected?;
            if unanimous
                .as_ref()
                .is_some_and(|previous| previous != &selected)
            {
                return None;
            }
            unanimous = Some(selected);
        }
        unanimous
    }

    /// Exact source table geometry, independently of its reporting label.
    pub(super) fn original_byte_slot_for_key(
        &self,
        key: &SourceCommandKey,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<tcl_core_types::ByteCommandSlot> {
        let SourceCommandKey::Slot { namespace, simple } = key else {
            return None;
        };
        let scope = self.original_namespace_geometry(namespace, policy)?;
        let namespace = match scope {
            SignatureNamespaceScope::C(path) => path,
            SignatureNamespaceScope::Jim(_) => tcl_core_types::ByteNamespacePath::root(),
            SignatureNamespaceScope::Symbolic(_) => return None,
        };
        Some(tcl_core_types::ByteCommandSlot {
            namespace,
            simple: simple.clone(),
        })
    }

    pub(super) fn original_targets_for_input(
        &self,
        input: &SignatureSourceNameInput,
        namespace: &SourceNamespaceKey,
        lookup: super::CommandTargetLookup,
    ) -> Option<OriginalCommandTargetSelection> {
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            let paths = self.original_command_paths_for_input(namespace, input);
            eprintln!(
                "ORIGINAL_COMMAND_TARGET_QUERY input={:?} policy={:?} namespace={:?} opaque={} paths={:?} cells={:?}",
                input.bytes(),
                input.policy(),
                namespace,
                self.has_opaque_domain(),
                paths,
                paths.as_ref().map(|paths| paths
                    .iter()
                    .flat_map(|path| path.iter())
                    .map(|key| (key, self.original_bindings_for_key(key)))
                    .collect::<Vec<_>>())
            );
        }
        self.original_targets_for_input_in(input, namespace, lookup, &mut BTreeSet::new())
    }

    fn original_targets_for_input_in(
        &self,
        input: &SignatureSourceNameInput,
        namespace: &SourceNamespaceKey,
        lookup: super::CommandTargetLookup,
        visiting: &mut BTreeSet<SourceCommandKey>,
    ) -> Option<OriginalCommandTargetSelection> {
        let mut selected = OriginalCommandTargetSelection {
            targets: BTreeSet::new(),
            original_prefixes: super::BTreeMap::new(),
            may_be_absent: false,
            unknown: false,
        };
        for path in self.original_command_paths_for_input(namespace, input)? {
            let mut absent = true;
            for key in path {
                if !visiting.insert(key.clone()) {
                    selected.unknown = true;
                    absent = false;
                    break;
                }
                let bindings = self.original_bindings_for_key(&key)?;
                for binding in &bindings {
                    match binding {
                        MayBinding::Target(target) => self.original_resolve_target(
                            target,
                            &key,
                            OriginalTargetLookupContext {
                                policy: input.policy(),
                                namespace,
                                lookup,
                            },
                            visiting,
                            &mut selected,
                        )?,
                        MayBinding::Imported(import) => {
                            let Some(implementations) = self.objects.get(&import.origin) else {
                                selected.unknown = true;
                                continue;
                            };
                            for implementation in implementations {
                                match implementation {
                                    MayBinding::Target(target) => self.original_resolve_target(
                                        target,
                                        &key,
                                        OriginalTargetLookupContext {
                                            policy: input.policy(),
                                            namespace,
                                            lookup,
                                        },
                                        visiting,
                                        &mut selected,
                                    )?,
                                    MayBinding::Missing => selected.may_be_absent = true,
                                    MayBinding::Imported(_) | MayBinding::Unknown => {
                                        selected.unknown = true;
                                    }
                                }
                            }
                        }
                        MayBinding::Missing => {}
                        MayBinding::Unknown => selected.unknown = true,
                    }
                }
                visiting.remove(&key);
                absent = bindings.contains(&MayBinding::Missing);
                if !absent {
                    break;
                }
            }
            selected.may_be_absent |= absent;
        }
        // No fallback implementation is invented from an opaque report name.
        // The generic driver keeps the independently possible unknown handler.
        selected.unknown |=
            selected.may_be_absent && matches!(lookup, super::CommandTargetLookup::WithFallback);
        Some(selected)
    }

    fn original_resolve_target(
        &self,
        target: &ResolvedCommandTarget,
        key: &SourceCommandKey,
        context: OriginalTargetLookupContext<'_>,
        visiting: &mut BTreeSet<SourceCommandKey>,
        selected: &mut OriginalCommandTargetSelection,
    ) -> Option<()> {
        let OriginalTargetLookupContext {
            policy,
            namespace,
            lookup,
        } = context;
        if self.runtime_execution_observed(target.token.as_ref()) {
            selected.unknown = true;
        }
        if target.terminal {
            selected.insert(target.clone(), target.prepended.is_empty().then(Vec::new));
            return Some(());
        }
        let slot = self.original_slot_for_command_key(key, policy)?;
        let publication = self.original_publication_at(&slot, policy)?;
        if !publication.has_implementation_allocation(target.implementation_allocation.as_ref()?) {
            return None;
        }
        let alias = publication.alias_target()?;
        let target_namespace = match alias.lookup() {
            tcl_registry::AliasTargetLookup::Global => self.source_root_namespace_key()?,
            tcl_registry::AliasTargetLookup::CallerNamespace => namespace.clone(),
        };
        let result = self.original_targets_for_input_in(
            alias.name_input(),
            &target_namespace,
            lookup,
            visiting,
        )?;
        selected.may_be_absent |= result.may_be_absent;
        selected.unknown |= result.unknown;
        for mut terminal in result.targets {
            let inputs = result
                .original_prefixes
                .get(&terminal)
                .cloned()
                .flatten()
                .and_then(|mut inputs| {
                    if alias.arguments().len() != target.prepended.len()
                        || alias.arguments().iter().any(|input| {
                            input.policy() != policy || !input.is_current(&self.source_variables)
                        })
                    {
                        return None;
                    }
                    inputs.extend_from_slice(alias.arguments());
                    Some(inputs)
                });
            terminal.prepended.extend(target.prepended.iter().cloned());
            selected.insert(terminal, inputs);
        }
        Some(())
    }

    pub(super) fn original_slot_for_command_key(
        &self,
        key: &SourceCommandKey,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<tcl_core_types::ByteCommandSlot> {
        let SourceCommandKey::Slot { namespace, simple } = key else {
            return None;
        };
        let mut world = (*self.original_command_world).clone();
        if world.select_policy(self)? != policy {
            return None;
        }
        let namespace = match world.scope(namespace, policy)? {
            SignatureNamespaceScope::C(path) => path,
            SignatureNamespaceScope::Jim(_) => tcl_core_types::ByteNamespacePath::root(),
            SignatureNamespaceScope::Symbolic(_) => return None,
        };
        Some(tcl_core_types::ByteCommandSlot {
            namespace,
            simple: simple.clone(),
        })
    }
}

#[cfg(test)]
mod conditional_catalogue_source_tests {
    #[test]
    fn original_conditional_source_catalogue_requires_current_custom_cell_barriers() {
        // naming.compiler.conditional-registry-source-metadata
        // docs/design/analysis/name-resolution-proofs/conditional-registry-source-metadata.md
        for (dialect, source, target) in [
            (
                "f5-irules",
                "proc call {args} {}\nwhen HTTP_REQUEST {call Lib::one}",
                "call Lib",
            ),
            (
                "tcl8.6",
                "namespace eval ::report {}\nproc ::report::defstyle {args} {}\n::report::defstyle st {} {top}",
                "::report::defstyle st",
            ),
        ] {
            let analysis = crate::analyser::Analyser::new().analyse(source, dialect);
            let config = analysis.body_lexer_config.unwrap();
            let mut actual = None;
            let start = u32::try_from(source.find(target).unwrap()).unwrap();
            // The actual whole source word-vector owner chooses a nested call
            // before metadata; no outer script's role can fill an inner miss.
            if analysis.has_original_vendor_source_names() {
                actual = crate::registry_invocation::source_structure::selected_vendor_registry_words_at(
                    source, &analysis, start,
                ).map(|(metadata, _)| metadata.shape().command().to_owned());
            } else if let Some(metadata) = analysis
                .original_conditional_registry_metadata_in_source(
                    &tcl_lexer::SourceImage::document(source),
                    config,
                    start,
                )
            {
                actual = Some(metadata.command().to_owned());
            }
            assert!(
                actual.is_none(),
                "{dialect}: custom source cell admitted {actual:?}"
            );
        }
    }
}
