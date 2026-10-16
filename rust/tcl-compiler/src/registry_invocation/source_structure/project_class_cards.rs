// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional project class display from original factories and source slots.

use crate::analyser::{AnalysisResult, ClassDef, ResolvedAnalysisInput};
use crate::command_binding::{
    OriginalSourceClassDeclaration, OriginalSourceClassInstance, SourceAdviceNameInput,
};
use std::sync::Arc;
use tcl_core_types::ByteCommandSlot;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage};
use tcl_registry::model::{ContextRegistry, ResolvedContext};
use tcl_syntax::naming::NamePolicyProtocol;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProjectClassCell {
    Class(Arc<OriginalSourceClassDeclaration>),
    Instance(Arc<OriginalSourceClassInstance>),
    Blocker,
}

/// Sealed final conditional graph transport, independent of Native publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalProjectClassCells {
    image: SourceImage,
    config: LexerConfig,
    context: ResolvedContext,
    registry: tcl_registry::RegistrySemanticKey,
    policy: Option<NamePolicyProtocol>,
    input: Option<ResolvedAnalysisInput>,
    cells: Vec<(ByteCommandSlot, ProjectClassCell)>,
    complete: bool,
}
impl OriginalProjectClassCells {
    pub(crate) fn new(
        image: SourceImage,
        config: LexerConfig,
        context: &ContextRegistry,
        policy: Option<NamePolicyProtocol>,
        cells: Vec<(ByteCommandSlot, ProjectClassCell)>,
        complete: bool,
    ) -> Self {
        Self {
            image,
            config,
            context: context.context().clone(),
            registry: context.commands().snapshot().semantic_key(),
            policy,
            input: None,
            cells,
            complete,
        }
    }
    pub(crate) fn with_retained_input(
        &self,
        image: &SourceImage,
        input: &ResolvedAnalysisInput,
    ) -> Option<Self> {
        let context = input.context_registry();
        (image == &self.image
            && input.lexer_config() == self.config
            && context.context() == &self.context
            && context.commands().snapshot().semantic_key() == self.registry
            && input.has_logical_source_name_context() == self.policy.is_none())
        .then_some(())?;
        let mut retained = self.clone();
        retained.input = Some(input.clone());
        Some(retained)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClassCard {
    factory: OriginalSourceClassDeclaration,
    metadata: ClassDef,
    members: crate::analyser::class_hierarchy::original_metadata::OriginalInstanceClassMetadata,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct DisplayRow {
    slot: ByteCommandSlot,
    class: Option<ClassCard>,
    instance: Option<Arc<OriginalSourceClassInstance>>,
}

/// Original factories, member owners and conditional named construction.
/// No report map, allocation, entered method or runtime occupation is supplied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalProjectClassCards {
    cells: OriginalProjectClassCells,
    realm: Arc<crate::realm::CommandBindingRealm>,
    rows: Vec<DisplayRow>,
}
impl OriginalProjectClassCards {
    fn matches_declaring_owner(&self) -> bool {
        self.cells.input.as_ref().is_some_and(|input| {
            self.realm.matches_resolved_analysis_input(input)
                && self
                    .realm
                    .matches_original_source_image(&self.cells.image, input.lexer_config())
        })
    }

    #[must_use]
    pub fn capture(analysis: &AnalysisResult) -> Option<Self> {
        let input = analysis.resolved_input.as_ref()?;
        let realm = analysis.retained_command_realm()?;
        let image = realm.original_source_image()?;
        analysis
            .matches_original_source_image(image, input.lexer_config())
            .then_some(())?;
        let cells = realm.original_class_display_publications(input)?;
        let mut rows = Vec::new();
        for (slot, cell) in &cells.cells {
            let (factory, instance) = match cell {
                ProjectClassCell::Class(class) => (Some(class.as_ref()), None),
                ProjectClassCell::Instance(instance) => (
                    Some(instance.class_declaration()),
                    Some(Arc::clone(instance)),
                ),
                ProjectClassCell::Blocker => (None, None),
            };
            let class = factory.and_then(|factory| {
                if !factory.factory().uncertain_operations().is_empty() {
                    return None;
                }
                if let Some(instance) = &instance {
                    if input.has_logical_source_name_context() {
                        instance.constructor().logical_constructor_shape(analysis)?;
                    } else {
                        instance.constructor().constructor_shape(analysis)?;
                    }
                }
                capture_class(factory, analysis)
            });
            rows.push(DisplayRow {
                slot: slot.clone(),
                class,
                instance,
            });
        }
        Some(Self {
            cells,
            realm: Arc::clone(analysis.retained_command_realm_arc()?),
            rows,
        })
    }
}

/// A selected original class and its independent current caller input.
/// Source-only constructor/member display cannot become Native dispatch.
pub struct OriginalProjectReceiverClass {
    class: ClassCard,
    input: ResolvedAnalysisInput,
    declaring_input: ResolvedAnalysisInput,
    policy: Option<NamePolicyProtocol>,
    owners: Vec<ClassCard>,
    hierarchy: crate::analyser::ClassHierarchy,
    method: NativeWord,
    collection: Option<crate::object_types::OriginalObjectCollectionSourceCandidates>,
}
impl OriginalProjectReceiverClass {
    /// Actual collection source association, when this is an element hint.
    /// The retained source union explicitly preserves unknown contents.
    #[must_use]
    pub const fn collection_source(
        &self,
    ) -> Option<&crate::object_types::OriginalObjectCollectionSourceCandidates> {
        self.collection.as_ref()
    }
    #[must_use]
    pub fn metadata(&self) -> &ClassDef {
        &self.class.metadata
    }
    #[must_use]
    pub fn hierarchy(&self) -> &crate::analyser::ClassHierarchy {
        &self.hierarchy
    }
    #[must_use]
    pub fn grammar(&self) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
        self.class
            .factory
            .grammar(&self.declaring_input.context_registry())
    }
    /// Exact original member correspondence precedes presentation. The shared
    /// hierarchy resolves original relation rows; report maps cannot add owners.
    #[must_use]
    pub fn provides_method(&self) -> bool {
        let original = &self.method;
        let Some(name) =
            SourceAdviceNameInput::conditional_display_word(original, &self.input, self.policy)
        else {
            return false;
        };
        let Ok(text) = std::str::from_utf8(name.bytes()) else {
            return false;
        };
        let Some(grammar) = self.grammar() else {
            return false;
        };
        let owners = self
            .owners
            .iter()
            .map(|owner| owner.members.clone())
            .collect::<Vec<_>>();
        match crate::analyser::class_hierarchy::original_metadata::original_instance_method_display(
            &owners, 0, &name,
        ) {
            Some(Some(_member)) => return true,
            None => return false,
            Some(None) => {}
        }
        grammar.property_accessor_methods.contains(&text)
            || grammar
                .builtin_object_method(text, tcl_registry::definer::MethodReach::ObjectCommand)
                .is_some_and(|method| {
                    method.receiver == tcl_registry::definer::BuiltinMethodReceiver::AnyObject
                })
    }
}

fn capture_class(
    factory: &OriginalSourceClassDeclaration,
    analysis: &AnalysisResult,
) -> Option<ClassCard> {
    if !factory.factory().uncertain_operations().is_empty() {
        return None;
    }
    let members = crate::analyser::class_hierarchy::original_metadata::OriginalInstanceClassMetadata::capture(analysis, factory)?;
    Some(ClassCard {
        factory: factory.clone(),
        metadata: members.metadata().clone(),
        members,
    })
}

fn finish(
    class: ClassCard,
    input: &ResolvedAnalysisInput,
    policy: Option<NamePolicyProtocol>,
    declaring_input: &ResolvedAnalysisInput,
    declaring_provider: &OriginalProjectClassCards,
    providers: &[OriginalProjectClassCards],
    method: &NativeWord,
) -> Option<OriginalProjectReceiverClass> {
    use std::collections::{HashMap, HashSet};
    let mut owners = vec![class.clone()];
    let project_complete = providers
        .iter()
        .filter(|provider| !std::ptr::eq(*provider, declaring_provider))
        .all(|provider| {
            provider.cells.complete
                && provider.matches_declaring_owner()
                && provider.rows.iter().all(|row| row.class.is_some())
        });
    // Provider occurrences remain independent even when complete source bytes,
    // sites, inputs and metadata compare equal. Only this exact selected
    // declaring inventory's immutable factory is already present as root.
    for provider in std::iter::once(declaring_provider).chain(
        providers
            .iter()
            .filter(|provider| !std::ptr::eq(*provider, declaring_provider)),
    ) {
        if !project_complete
            || !provider.cells.complete
            || !provider.matches_declaring_owner()
            || provider.rows.iter().any(|row| row.class.is_none())
            || provider.cells.policy != policy
        {
            continue;
        }
        for card in provider
            .rows
            .iter()
            .filter(|row| row.instance.is_none())
            .filter_map(|row| row.class.as_ref())
        {
            if std::ptr::eq(provider, declaring_provider)
                && std::ptr::eq(card.factory.factory(), class.factory.factory())
            {
                continue;
            }
            owners.push(card.clone());
        }
    }
    let mut classes = HashMap::new();
    let mut ambiguous = HashSet::new();
    for owner in &owners {
        let name = &owner.metadata.qualified_name;
        if classes
            .insert(name.clone(), owner.metadata.clone())
            .is_some()
        {
            ambiguous.insert(name.clone());
        }
    }
    classes.retain(|name, _| !ambiguous.contains(name));
    (classes.get(&class.metadata.qualified_name) == Some(&class.metadata)).then_some(())?;
    let hierarchy = crate::analyser::build_class_hierarchy(classes);
    Some(OriginalProjectReceiverClass {
        class,
        input: input.clone(),
        declaring_input: declaring_input.clone(),
        policy,
        owners,
        hierarchy,
        method: method.clone(),
        collection: None,
    })
}

/// Whole receiver command and genuine selected external source slot.
/// Stronger occupants, aliases, tombstones and unknown effects stay terminal.
/// Missing representation supplies neither absence nor class identity.
/// Providers are independent foreign-document inventories: the caller must
/// exclude its actual document owner before transport, never by source bytes.
#[must_use]
pub fn original_project_receiver_class(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    providers: &[OriginalProjectClassCards],
) -> Option<OriginalProjectReceiverClass> {
    original_project_receiver_class_in_unit(source, analysis, segment, providers, None)
}

/// Source display with an independently retained original CU for collection
/// insertion/extraction associations. Missing CU supplies no element hint.
#[must_use]
pub fn original_project_receiver_class_in_unit(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    providers: &[OriginalProjectClassCards],
    unit: Option<&crate::compilation_unit::CompilationUnit>,
) -> Option<OriginalProjectReceiverClass> {
    let sources = unit.and_then(|unit| {
        crate::object_types::OriginalObjectCollectionSources::capture(source, analysis, unit)
    });
    original_project_receiver_class_with_sources(
        source,
        analysis,
        segment,
        providers,
        sources.as_ref(),
    )
}

/// The same source display over a once-captured actual CU collection inventory.
#[must_use]
pub fn original_project_receiver_class_with_sources(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    providers: &[OriginalProjectClassCards],
    sources: Option<&crate::object_types::OriginalObjectCollectionSources>,
) -> Option<OriginalProjectReceiverClass> {
    let offset = segment.argv.first()?.span.start();
    let (input, realm, words) = super::registered_original_words_at(source, analysis, offset)?;
    let binding = realm.invocation_at_source("", offset);
    let (actual, _) = binding.original_recorded_command()?;
    if actual.span != segment.span
        || actual.argv != segment.argv
        || actual.texts != segment.texts
        || actual.word_fragments != segment.word_fragments
        || actual.single_token_word != segment.single_token_word
        || actual.expand_word != segment.expand_word
        || actual.is_partial != segment.is_partial
        || actual.partial_delimiter != segment.partial_delimiter
        || segment
            .expand_word
            .as_ref()
            .is_some_and(|expanded| expanded.iter().any(|expanded| *expanded))
    {
        return None;
    }
    // Operands can mutate tables before dispatch; this purpose has no effect
    // proof for dynamic operands and keeps them unavailable.
    if words
        .iter()
        .skip(1)
        .any(|word| tcl_syntax::word_rules::original_static_word_unicode_value(word).is_none())
    {
        return None;
    }
    if let Some(receipt) = realm.original_source_class_instance_words(input, &words) {
        if receipt.obligations().contains(
            &crate::command_binding::SourceCommandTransitionObligation::UnknownEarlierMutation,
        ) {
            return None;
        }
        let class = capture_class(receipt.instance().class_declaration(), analysis)?;
        if input.has_logical_source_name_context() {
            receipt
                .instance()
                .constructor()
                .logical_constructor_shape(analysis)?;
        } else {
            receipt
                .instance()
                .constructor()
                .constructor_shape(analysis)?;
        }
        let policy = receipt
            .instance()
            .class_declaration()
            .name_input()
            .native_input()
            .map(|input| input.policy());
        let local = OriginalProjectClassCards::capture(analysis)?;
        return finish(
            class,
            input,
            policy,
            input,
            &local,
            providers,
            words.get(1)?,
        );
    }
    if let Some(sources) = sources
        && let Some(collection) = sources.for_receiver(source, analysis, words.first()?, offset)
    {
        let mut selected: Option<OriginalProjectReceiverClass> = None;
        for element in collection.elements() {
            let candidate = constructor_receiver_class(
                source,
                analysis,
                &element.construction().words,
                providers,
                words.get(1)?,
            )?;
            if selected.as_ref().is_some_and(|previous| {
                previous.class.factory.factory() != candidate.class.factory.factory()
                    || previous.class.metadata != candidate.class.metadata
            }) {
                return None;
            }
            selected = Some(candidate);
        }
        let mut selected = selected?;
        selected.collection = Some(collection);
        return Some(selected);
    }
    let child = super::original_single_source_command_substitution(words.first()?);
    let (lookup_words, construction) = if let Some(child) = &child {
        (child.command().words.as_slice(), true)
    } else {
        tcl_syntax::word_rules::original_static_word_unicode_value(words.first()?)?;
        (words.as_slice(), false)
    };
    let lookup_offset = lookup_words.first()?.span().start();
    if construction {
        return constructor_receiver_class(
            source,
            analysis,
            lookup_words,
            providers,
            words.get(1)?,
        );
    }
    let selected = selected_row(source, analysis, lookup_offset, lookup_words, providers)?;
    selected.1.instance.as_ref()?;
    finish(
        selected.1.class.as_ref()?.clone(),
        input,
        selected.0.cells.policy,
        selected.0.cells.input.as_ref()?,
        selected.0,
        providers,
        words.get(1)?,
    )
}

fn constructor_receiver_class(
    source: &str,
    analysis: &AnalysisResult,
    lookup_words: &[NativeWord],
    providers: &[OriginalProjectClassCards],
    method: &NativeWord,
) -> Option<OriginalProjectReceiverClass> {
    let input = analysis.resolved_input.as_ref()?;
    if lookup_words
        .iter()
        .any(|word| tcl_syntax::word_rules::original_static_word_unicode_value(word).is_none())
    {
        return None;
    }
    let lookup_offset = lookup_words.first()?.span().start();
    if let Some(call) = super::source_constructor_call_at(source, analysis, lookup_offset) {
        (call.original_words() == lookup_words
            && !call.obligations().contains(
                &crate::command_binding::SourceCommandTransitionObligation::UnknownEarlierMutation,
            ))
        .then_some(())?;
        let shape = if input.has_logical_source_name_context() {
            call.logical_constructor_shape(analysis)?
        } else {
            call.constructor_shape(analysis)?
        };
        (lookup_words.len().checked_sub(1)? >= shape.constructor_args_from()).then_some(())?;
        let class = capture_class(call.class_declaration(), analysis)?;
        let final_cards = OriginalProjectClassCards::capture(analysis)?;
        final_cards
            .rows
            .iter()
            .any(|row| {
                row.instance.is_none()
                    && row.class.as_ref().is_some_and(|current| {
                        std::ptr::eq(current.factory.factory(), class.factory.factory())
                    })
            })
            .then_some(())?;
        let policy = call
            .class_declaration()
            .name_input()
            .native_input()
            .map(|input| input.policy());
        return finish(class, input, policy, input, &final_cards, providers, method);
    }
    let selected = selected_row(source, analysis, lookup_offset, lookup_words, providers)?;
    selected.1.instance.is_none().then_some(())?;
    let class = selected.1.class.as_ref()?;
    let selector = SourceAdviceNameInput::conditional_display_word(
        lookup_words.get(1)?,
        input,
        selected.0.cells.policy,
    )?;
    let shape = class.factory.display_constructor_shape(
        &class.metadata,
        &selector,
        &selected.0.cells.input.as_ref()?.context_registry(),
        None,
    )?;
    (lookup_words.len().checked_sub(1)? >= shape.constructor_args_from()).then_some(())?;
    finish(
        class.clone(),
        input,
        selected.0.cells.policy,
        selected.0.cells.input.as_ref()?,
        selected.0,
        providers,
        method,
    )
}

fn selected_row<'a>(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
    words: &[NativeWord],
    providers: &'a [OriginalProjectClassCards],
) -> Option<(&'a OriginalProjectClassCards, &'a DisplayRow)> {
    let (input, realm, retained) = super::registered_original_words_at(source, analysis, offset)?;
    (retained == words).then_some(())?;
    if providers
        .iter()
        .any(|provider| !provider.cells.complete || !provider.matches_declaring_owner())
    {
        return None;
    }
    let rows = providers
        .iter()
        .filter(|provider| {
            provider.cells.input.as_ref().is_some_and(|declaring| {
                declaring.has_logical_source_name_context()
                    == input.has_logical_source_name_context()
            })
        })
        .flat_map(|provider| provider.rows.iter().map(move |row| (provider, row)))
        .collect::<Vec<_>>();
    let selected = if input.has_logical_source_name_context() {
        let candidates = super::source_logical_command_candidates_at(source, analysis, offset)?;
        if candidates.has_earlier_unknown_mutations()
            || candidates.has_lookup_operand_effects()
            || candidates.original_words() != words
        {
            return None;
        }
        let mut selected = None;
        for slot in candidates.external_header_coordinates()? {
            let matching = rows
                .iter()
                .copied()
                .filter(|(_, row)| &row.slot == slot)
                .collect::<Vec<_>>();
            if !matching.is_empty() {
                selected = Some(matching);
                break;
            }
        }
        selected?
    } else {
        let binding = realm.invocation_at_source("", offset);
        let (_, mut tokens) = binding.original_recorded_command()?;
        realm.stamp_original_tokens(&mut tokens);
        let lookup = tokens
            .source_binding
            .as_ref()?
            .original_static_head_lookup_for_tokens(&tokens)?;
        let publications = rows
            .iter()
            .map(|(provider, row)| Some((&row.slot, provider.cells.policy?, (*provider, *row))))
            .collect::<Option<Vec<_>>>()?;
        lookup.matching_external_source_advice_slots(analysis.resolved_registry()?, publications)?
    };
    let [selected] = selected.as_slice() else {
        return None;
    };
    Some(*selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;

    fn receiver(
        source: &str,
        dialect: &str,
        providers: &[OriginalProjectClassCards],
    ) -> Option<OriginalProjectReceiverClass> {
        let analysis = Analyser::new().analyse(source, dialect);
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            analysis.body_lexer_config?,
        )
        .pop()?;
        original_project_receiver_class(source, &analysis, &command, providers)
    }
    fn cards(source: &str, dialect: &str) -> OriginalProjectClassCards {
        OriginalProjectClassCards::capture(&Analyser::new().analyse(source, dialect))
            .expect(dialect)
    }

    fn collection_receiver(
        source: &str,
        dialect: &str,
        providers: &[OriginalProjectClassCards],
    ) -> Option<OriginalProjectReceiverClass> {
        let analysis = Analyser::new().analyse(source, dialect);
        let input = analysis.resolved_input.as_ref()?;
        let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: analysis.resolved_registry()?,
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(input.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            input,
        );
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            input.lexer_config(),
        )
        .pop()?;
        original_project_receiver_class_in_unit(source, &analysis, &command, providers, Some(&unit))
    }

    #[test]
    fn project_class_display_uses_instance_side_and_inherited_visibility() {
        for dialect in ["tcl", "tcl8.6", "tcl9.0"] {
            let homonym = vec![cards(
                "oo::class create Base {method ping {} {}}; oo::class create C {superclass Base; self method ping {} {}}",
                dialect,
            )];
            assert!(
                receiver("[C new] ping", dialect, &homonym)
                    .expect(dialect)
                    .provides_method()
            );
            for source in [
                "oo::class create Base {method ping {} {}}; oo::class create C {superclass Base; unexport ping}",
                "oo::class create Base {method ping {} {}; unexport ping}; oo::class create C {superclass Base}",
                "oo::class create Base {private method ping {} {}}; oo::class create C {superclass Base; export ping}",
            ] {
                let providers = vec![cards(source, dialect)];
                assert!(
                    receiver("[C new] ping", dialect, &providers)
                        .is_none_or(|class| !class.provides_method()),
                    "{dialect}: {source}"
                );
            }
            let exported = vec![cards(
                "oo::class create Base {method ping {} {}; unexport ping}; oo::class create C {superclass Base; export ping}",
                dialect,
            )];
            assert!(
                receiver("[C new] ping", dialect, &exported)
                    .expect(dialect)
                    .provides_method()
            );
            let base = cards("oo::class create Base {method ping {} {}}", dialect);
            let unknown = cards("unknown_effect", dialect);
            let source = "oo::class create C {superclass Base}; [C new] ping";
            assert!(
                !receiver(source, dialect, &[base, unknown])
                    .expect(dialect)
                    .provides_method()
            );
        }
    }

    #[test]
    fn project_class_inherited_display_does_not_bypass_final_non_class_occupants() {
        for dialect in ["tcl", "tcl8.6", "tcl9.0"] {
            for blocker in [
                "proc Base {args} {}",
                "interp alias {} Base {} list",
                "rename Base {}",
            ] {
                let source = format!(
                    "oo::class create ::Base {{method ping {{}} {{}}}}; namespace eval ns {{oo::class create C {{superclass Base}}; {blocker}}}"
                );
                let providers = [cards(&source, dialect)];
                assert!(
                    receiver("[::ns::C new] ping", dialect, &providers)
                        .is_none_or(|class| !class.provides_method()),
                    "{dialect}: {source}"
                );
            }
        }
    }

    #[test]
    fn project_class_collection_keeps_original_insertion_extraction_and_unknown_contents() {
        for dialect in ["tcl", "tcl8.6", "tcl9.0"] {
            for storage in [
                "dict set values key [C new]; [dict get $values key] ping",
                "lappend values [C new]; [lindex $values 0] ping",
                "dict set values key [C new]; rename C D; [dict get $values key] ping",
            ] {
                let source = format!("oo::class create C {{method ping {{}} {{}}}}; {storage}");
                let selected = collection_receiver(&source, dialect, &[]).expect(&source);
                assert!(selected.provides_method(), "{dialect}: {source}");
                let association = selected
                    .collection_source()
                    .expect("original element association");
                assert!(association.has_unknown_residual());
                assert_eq!(association.elements().len(), 1);
                assert_eq!(
                    association.elements()[0].construction().words[0]
                        .try_text()
                        .unwrap(),
                    "C"
                );
                assert!(
                    receiver(&source, dialect, &[]).is_none(),
                    "missing CU cannot recover source from inferred labels"
                );
            }
            for storage in [
                "dict set values outer inner [C new]; [dict get $values outer] ping",
                "dict lappend values key [C new]; [dict get $values key] ping",
                "dict set values key UNKNOWN; [dict get $values key] ping",
                "dict set other key [C new]; [dict get $values key] ping",
            ] {
                let source = format!("oo::class create C {{method ping {{}} {{}}}}; {storage}");
                assert!(
                    collection_receiver(&source, dialect, &[]).is_none(),
                    "{dialect}: {source}"
                );
            }
        }
    }

    #[test]
    fn project_class_inherited_display_preserves_independent_equal_content_providers() {
        for dialect in ["tcl", "tcl8.6", "tcl9.0"] {
            let provider = "oo::class create Base {method ping {} {}}";
            let source = "oo::class create C {superclass Base}; [C new] ping";
            let a = cards(provider, dialect);
            let b = cards(provider, dialect);
            assert!(
                receiver(source, dialect, &[a, b]).is_none_or(|class| !class.provides_method()),
                "{dialect}"
            );
        }
    }

    #[test]
    fn project_class_collection_handles_and_loop_fields_retain_their_source_owner() {
        for dialect in ["tcl", "tcl8.6", "tcl9.0"] {
            let source = "oo::class create C {method ping {} {}}; dict set values key [C new]; set item [dict get $values key]; $item ping";
            let selected = collection_receiver(source, dialect, &[]).expect(dialect);
            assert!(selected.provides_method());
            assert!(matches!(
                selected.collection_source().unwrap().receiver_path(),
                crate::object_types::OriginalCollectionReceiverPath::HandleAssignment(_)
            ));
            for (storage, loop_source) in [
                (
                    "lappend values [C new]",
                    "foreach item $values {$item ping}",
                ),
                (
                    "dict set values key [C new]",
                    "dict for {key item} $values {$item ping}",
                ),
            ] {
                let source = format!(
                    "oo::class create C {{method ping {{}} {{}}}}; {storage}; {loop_source}"
                );
                let analysis = Analyser::new().analyse(&source, dialect);
                let input = analysis.resolved_input.as_ref().unwrap();
                let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
                    &source,
                    crate::compilation_unit::UnitBuildOptions {
                        registry: analysis.resolved_registry().unwrap(),
                        defer_top_level: false,
                        config: input.lexer_config(),
                        dialect: Some(input.unit_profile()),
                        external_call_sites: None,
                        declared_commands: None,
                    },
                    None,
                    input,
                );
                let offset = u32::try_from(source.rfind("$item ping").unwrap()).unwrap();
                let binding = analysis
                    .retained_command_realm()
                    .unwrap()
                    .invocation_at_source("", offset);
                let (command, _) = binding
                    .original_recorded_command()
                    .expect("actual retained loop body");
                let selected = original_project_receiver_class_in_unit(
                    &source,
                    &analysis,
                    command,
                    &[],
                    Some(&unit),
                )
                .expect(&source);
                assert!(selected.provides_method(), "{dialect}: {source}");
                let collection = selected.collection_source().unwrap();
                assert!(collection.has_unknown_residual());
                assert!(matches!(
                    collection.receiver_path(),
                    crate::object_types::OriginalCollectionReceiverPath::LoopBinding { .. }
                ));
                assert_eq!(collection.elements().len(), 1);
            }
        }
    }

    #[test]
    fn project_class_collection_keeps_lookup_barriers_and_final_factory_occupants() {
        for dialect in ["tcl", "tcl8.6", "tcl9.0"] {
            for suffix in [
                "rename C {}",
                "rename C {}; oo::class create C {}",
                "unknown_effect",
                "proc dict {args} {}",
            ] {
                let source = format!(
                    "oo::class create C {{method ping {{}} {{}}}}; dict set values key [C new]; {suffix}; [dict get $values key] ping"
                );
                assert!(
                    collection_receiver(&source, dialect, &[]).is_none(),
                    "{dialect}: {source}"
                );
            }
            let library = cards("oo::class create C {method ping {} {}}", dialect);
            let source = "dict set values key [C new]; [dict get $values key] ping";
            // The foreign header can describe the construction, but its call
            // leaves a genuine earlier-mutation barrier on the getter lookup.
            assert!(
                receiver("[C new] ping", dialect, std::slice::from_ref(&library))
                    .unwrap()
                    .provides_method()
            );
            assert!(
                collection_receiver(source, dialect, &[library]).is_none(),
                "{dialect}"
            );
        }
    }

    #[test]
    fn project_class_cards_do_not_use_native_report_membership_as_instance_identity() {
        let source = "oo::class create C {method ping {} {}}; C create obj";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.all_classes.clear();
        analysis.instance_classes.clear();
        analysis.created_instance_commands.clear();
        let cards = OriginalProjectClassCards::capture(&analysis).unwrap();
        assert!(
            receiver("obj ping", "tcl8.6", &[cards])
                .unwrap()
                .provides_method()
        );
        assert!(OriginalProjectClassCards::capture(&AnalysisResult::default()).is_none());
        let mut unavailable = Analyser::new().analyse(source, "tcl8.6");
        unavailable.resolved_input = None;
        assert!(OriginalProjectClassCards::capture(&unavailable).is_none());
    }

    #[test]
    fn project_class_cards_keep_factory_member_and_named_instance_owners() {
        for dialect in ["tcl", "tcl8.6", "tcl9.0"] {
            let providers = vec![cards(
                "oo::class create ::C {method ping {} {}}; ::C create ::obj",
                dialect,
            )];
            for source in ["[::C new] ping", "::obj ping"] {
                let class = receiver(source, dialect, &providers).expect(source);
                assert_eq!(class.metadata().qualified_name, "::C");
                assert!(class.provides_method(), "{dialect}: {source}");
            }
            let class = receiver("[::C new] missing", dialect, &providers).expect(dialect);
            assert!(!class.provides_method());
            for source in [
                "oo::class create ::C {method ping {} {}}; ::C create ::obj; ::obj ping",
                "oo::class create ::C {method ping {} {}}; set handle [::C new]; $handle ping",
            ] {
                assert!(
                    receiver(source, dialect, &[])
                        .expect(source)
                        .provides_method()
                );
            }
        }
    }

    #[test]
    fn project_class_cards_preserve_caller_occupants_and_unknown_operand_barriers() {
        for dialect in ["tcl", "tcl8.6", "tcl9.0"] {
            let providers = vec![cards(
                "oo::class create ::C {method ping {} {}}; ::C create ::obj",
                dialect,
            )];
            for source in [
                "proc C {args} {}; [C new] ping",
                "proc C {args} {}; rename C {}; [C new] ping",
                "interp alias {} C {} list; [C new] ping",
                "unknown_effect; [C new] ping",
                "[C new] ping [unknown_operand]",
                "[C {*}{new}] ping",
                "[C new; C new] ping",
                "prefix[C new] ping",
                "proc obj {args} {}; obj ping",
                "proc obj {args} {}; rename obj {}; obj ping",
                "interp alias {} obj {} list; obj ping",
            ] {
                assert!(
                    receiver(source, dialect, &providers).is_none(),
                    "{dialect}: {source}"
                );
            }
        }
    }

    #[test]
    fn project_class_cards_keep_provider_tombstones_aliases_incarnations_and_conflicts() {
        for dialect in ["tcl", "tcl8.6", "tcl9.0"] {
            for library in [
                "oo::class create C {method ping {} {}}; rename C {}",
                "oo::class create C {method ping {} {}}; rename C Other",
                "oo::class create C {method ping {} {}}; unknown_effect",
                "oo::class create Other {method ping {} {}}; interp alias {} C {} Other",
                "oo::class create C {method ping {} {}}; C create obj; rename C {}; oo::class create C {}",
            ] {
                let providers = vec![cards(library, dialect)];
                assert!(
                    receiver("[C new] ping", dialect, &providers)
                        .is_none_or(|class| !class.provides_method()),
                    "{dialect}: {library}"
                );
                assert!(
                    receiver("obj ping", dialect, &providers).is_none(),
                    "{dialect}: {library}"
                );
            }
            let removed_instance = vec![cards(
                "oo::class create C {method ping {} {}}; C create obj; rename obj {}",
                dialect,
            )];
            assert!(receiver("obj ping", dialect, &removed_instance).is_none());
            assert!(
                receiver("[C new] ping", dialect, &removed_instance)
                    .expect(dialect)
                    .provides_method()
            );
            let providers = vec![
                cards("oo::class create C {method ping {} {}}", dialect),
                cards("oo::class create C {method pong {} {}}", dialect),
            ];
            assert!(receiver("[C new] ping", dialect, &providers).is_none());
            let providers = vec![cards(
                "oo::class create C {method ping {} {}}; rename C D",
                dialect,
            )];
            assert!(
                receiver("[D new] ping", dialect, &providers)
                    .expect(dialect)
                    .provides_method()
            );
        }
    }

    #[test]
    fn project_class_cards_require_selected_constructor_shape_and_compatible_inputs() {
        for dialect in ["tcl", "tcl8.6", "tcl9.0"] {
            for body in ["self method new {} {}"] {
                let providers = vec![cards(
                    &format!("oo::class create C {{{body}; method ping {{}} {{}}}}"),
                    dialect,
                )];
                assert!(
                    receiver("[C new] ping", dialect, &providers).is_none(),
                    "{dialect}: {body}"
                );
            }
            let hidden = vec![cards(
                "oo::class create C {method ping {} {}}; oo::objdefine C unexport new",
                dialect,
            )];
            assert!(receiver("[C new] ping", dialect, &hidden).is_none());
            let providers = vec![cards("oo::class create C {method ping {} {}}", dialect)];
            assert!(receiver("[C missing] ping", dialect, &providers).is_none());
            assert!(receiver("[C create] ping", dialect, &providers).is_none());
            let other = if dialect == "tcl9.0" {
                "tcl8.6"
            } else {
                "tcl9.0"
            };
            assert!(receiver("[C new] ping", other, &providers).is_none());
        }
    }
}
