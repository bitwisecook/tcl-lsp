// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original source collection associations, independently of inferred labels.

use crate::analyser::{AnalysisResult, MemberSide};
use crate::compilation_unit::CompilationUnit;
use crate::registry_invocation::source_structure::{self, OriginalRegistryWords};
use crate::signature_scan::variable_symbol::{
    SignatureSourceVariableOccurrence, SignatureSourceVariableSymbol,
};
use tcl_lexer::{ExecutablePart, NativeScriptCommandWords, NativeWord, SourceImage};

#[derive(Debug, Clone, PartialEq, Eq)]
enum CollectionSourcePlace {
    Variable(SignatureSourceVariableSymbol),
    DeclaredField {
        class: crate::signature_scan::original_name::SourceDeclarationMetadata<
            crate::analyser::ClassDef,
        >,
        side: MemberSide,
        declaration: NativeWord,
    },
}

/// One genuine insertion schema and its whole inserted constructor syntax.
/// The association is conditional source evidence, never a stored element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalCollectionElementSource {
    writer: OriginalRegistryWords,
    variable: SignatureSourceVariableOccurrence,
    value: NativeWord,
    construction: NativeScriptCommandWords,
    effect: tcl_registry::VarElementsEffect,
}
impl OriginalCollectionElementSource {
    /// Exact effect selected by the retained writer schema. A name-concatenating
    /// contribution stays distinct from a whole-element write.
    #[must_use]
    pub const fn source_effect(&self) -> tcl_registry::VarElementsEffect {
        self.effect
    }
    /// The retained original schema for this collection writer.
    #[must_use]
    pub const fn writer(&self) -> &OriginalRegistryWords {
        &self.writer
    }
    /// The original container variable occurrence selected by the writer.
    #[must_use]
    pub const fn original_variable(&self) -> &SignatureSourceVariableOccurrence {
        &self.variable
    }
    /// The original written value operand supplied by the collection writer.
    #[must_use]
    pub const fn original_value(&self) -> &NativeWord {
        &self.value
    }
    /// The complete original constructor invocation supplying the element.
    #[must_use]
    pub const fn construction(&self) -> &NativeScriptCommandWords {
        &self.construction
    }
}

/// Actual extraction schema, original container occurrence and represented
/// insertion sources. Unrepresented contents and execution remain unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalObjectCollectionSourceCandidates {
    extraction: OriginalRegistryWords,
    variable: SignatureSourceVariableOccurrence,
    elements: Vec<OriginalCollectionElementSource>,
    path: OriginalCollectionReceiverPath,
}
impl OriginalObjectCollectionSourceCandidates {
    /// The retained original extraction schema and complete invocation.
    #[must_use]
    pub const fn extraction(&self) -> &OriginalRegistryWords {
        &self.extraction
    }
    /// The original container variable occurrence selected by extraction.
    #[must_use]
    pub const fn original_variable(&self) -> &SignatureSourceVariableOccurrence {
        &self.variable
    }
    /// Represented insertion sources, without claiming complete contents.
    #[must_use]
    pub fn elements(&self) -> &[OriginalCollectionElementSource] {
        &self.elements
    }
    /// The original receiver association selected for this extraction.
    #[must_use]
    pub const fn receiver_path(&self) -> &OriginalCollectionReceiverPath {
        &self.path
    }
    /// This source union never closes contents versions or unknown producers.
    #[must_use]
    pub const fn has_unknown_residual(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CollectionRead {
    extraction: OriginalRegistryWords,
    variable: SignatureSourceVariableOccurrence,
    place: CollectionSourcePlace,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct CollectionWrite {
    place: CollectionSourcePlace,
    element: OriginalCollectionElementSource,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct CollectionHandle {
    place: CollectionSourcePlace,
    extraction: u32,
    setter: crate::registry_invocation::OriginalSourceHandleConstruction,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct CollectionLoop {
    place: CollectionSourcePlace,
    target: SignatureSourceVariableOccurrence,
    field: tcl_syntax::word_rules::OriginalSourceListElement,
    words: OriginalRegistryWords,
    body: crate::registry_invocation::OriginalSourceScriptBody,
    variable: SignatureSourceVariableOccurrence,
}

/// Exact setter/loop association used by an element receiver. These retain
/// source syntax without claiming an executed assignment or loop iteration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginalCollectionReceiverPath {
    /// A direct original collection-element receiver expression.
    ElementRead,
    /// A receiver retained through its selected original handle assignment.
    HandleAssignment(crate::registry_invocation::OriginalSourceHandleConstruction),
    /// A receiver associated with an original loop source, without an entered iteration.
    LoopBinding {
        /// Actual selected loop schema and original full invocation.
        words: OriginalRegistryWords,
        /// Honest list-field producer; no child word or Native name is issued.
        field: tcl_syntax::word_rules::OriginalSourceListElement,
        /// Actual selected body containing the receiver occurrence.
        body: crate::registry_invocation::OriginalSourceScriptBody,
    },
}

/// One immutable original CU's source collection associations. Capture runs
/// once for a token walk; no variable/class report map supplies an association.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalObjectCollectionSources {
    input: crate::analyser::ResolvedAnalysisInput,
    realm: std::sync::Arc<crate::realm::CommandBindingRealm>,
    image: SourceImage,
    writes: Vec<CollectionWrite>,
    handles: Vec<CollectionHandle>,
    loops: Vec<CollectionLoop>,
    places: Vec<(SignatureSourceVariableSymbol, CollectionSourcePlace)>,
}
impl OriginalObjectCollectionSources {
    /// Capture associations from the same original source, analysis and compilation unit.
    #[must_use]
    pub fn capture(
        source: &str,
        analysis: &AnalysisResult,
        unit: &CompilationUnit,
    ) -> Option<Self> {
        let input = analysis.resolved_input.as_ref()?;
        let registry = analysis.resolved_registry()?;
        let realm = analysis.retained_command_realm()?;
        let image = SourceImage::document(source);
        let module = &unit.ir_module;
        if !realm.matches_resolved_analysis_input(input)
            || !analysis.matches_original_source_image(&image, input.lexer_config())
            || unit.source != source
            || module.source != image
            || module.source_metadata_input.as_ref() != Some(input)
            || module.native_lexer_config() != input.lexer_config()
            || module
                .retained_source_bindings
                .as_ref()
                .is_none_or(|held| !held.matches_module(module, registry))
        {
            return None;
        }
        let context = input.context_registry();
        let mut commands = Vec::new();
        let mut pending = vec![&module.top_level];
        pending.extend(
            module
                .original_source_procedure_bodies()
                .map(|(_, procedure)| &procedure.body),
        );
        pending.extend(module.methods.values().map(|method| &method.body));
        while let Some(script) = pending.pop() {
            for statement in &script.statements {
                pending.extend(statement.child_scripts());
                let Some(tokens) = script.retained_source_tokens_for_statement(statement) else {
                    continue;
                };
                let Some(offset) = tokens
                    .words()
                    .first()
                    .map(|word| word.source().span.start())
                else {
                    continue;
                };
                let Some(words) =
                    source_structure::source_registry_words_at(source, analysis, offset)
                else {
                    continue;
                };
                if !commands.iter().any(|(site, _)| *site == offset) {
                    commands.push((offset, words));
                }
            }
        }
        // Original evaluated command substitutions can be represented as value
        // words rather than standalone IR statements. Their schema remains the
        // retained source owner's query, never a reparsed command-name switch.
        let mut nested = Vec::new();
        for (_, words) in &commands {
            for argument in 0..words.arguments().len() {
                if let Some(child) = words
                    .original_argument_word(argument)
                    .and_then(source_structure::original_single_source_command_substitution)
                {
                    if let Some(offset) = child
                        .command()
                        .words
                        .first()
                        .map(|word| word.span().start())
                    {
                        if let Some(selected) =
                            source_structure::source_registry_words_at(source, analysis, offset)
                        {
                            nested.push((offset, selected));
                        }
                    }
                }
            }
        }
        for (offset, words) in nested {
            if !commands.iter().any(|(site, _)| *site == offset) {
                commands.push((offset, words));
            }
        }
        let mut result = Self {
            input: input.clone(),
            realm: std::sync::Arc::clone(analysis.retained_command_realm_arc()?),
            image,
            writes: Vec::new(),
            handles: Vec::new(),
            loops: Vec::new(),
            places: Vec::new(),
        };
        let owners = super::OwnerIndex::build(unit, super::FactMode::Full);
        for occurrence in &analysis.original_variable_symbols {
            if result
                .places
                .iter()
                .any(|(symbol, _)| symbol == occurrence.symbol())
            {
                continue;
            }
            if let Some(place) = collection_place(
                analysis,
                unit,
                &owners,
                occurrence,
                occurrence.span().start(),
            ) {
                result.places.push((occurrence.symbol().clone(), place));
            }
        }
        for (offset, words) in &commands {
            if !collection_lookup_is_available(words) {
                continue;
            }
            if let Some((target, values, effect)) = words
                .with_source_schema(&context, |schema| {
                    let base = schema.semantics.argument_offset;
                    if !schema
                        .authored_source_variable_name_arguments()
                        .contains(&(base, tcl_registry::ArgRole::VarWrite))
                    {
                        return None;
                    }
                    let length = schema.words.arguments().exact_argv_len()?;
                    let effect = schema.semantics.var_elements_effect?;
                    let values = match effect {
                        tcl_registry::VarElementsEffect::AppendsListElements { values_from } => {
                            (base + usize::from(values_from)..length).collect::<Vec<_>>()
                        }
                        tcl_registry::VarElementsEffect::SetsDictValue if length == base + 3 => {
                            vec![length - 1]
                        }
                        tcl_registry::VarElementsEffect::ExtendsDictValuesByName {
                            values_from,
                        } if length == base + usize::from(values_from) + 1 => vec![length - 1],
                        _ => return None,
                    };
                    Some((base, values, effect))
                })
                .flatten()
            {
                if let Some(variable) = words
                    .original_argument_word(target)
                    .and_then(|word| operand_occurrence(analysis, word))
                {
                    if let Some(place) = result.place_for(&variable) {
                        for value in values {
                            let Some(value) = words.original_argument_word(value) else {
                                continue;
                            };
                            let Some(child) =
                                source_structure::original_single_source_command_substitution(
                                    value,
                                )
                            else {
                                continue;
                            };
                            result.writes.push(CollectionWrite {
                                place: place.clone(),
                                element: OriginalCollectionElementSource {
                                    writer: words.clone(),
                                    variable: variable.clone(),
                                    value: value.clone(),
                                    construction: child.command().clone(),
                                    effect,
                                },
                            });
                        }
                    }
                }
            }
            if let Some(setter) =
                source_structure::source_handle_construction_at(source, analysis, *offset)
            {
                if let Some(variable) =
                    operand_occurrence(analysis, setter.original_variable_word())
                {
                    if let Some(place) = result.place_for(&variable) {
                        if let Some(extraction) = setter
                            .construction()
                            .words
                            .first()
                            .map(|word| word.span().start())
                        {
                            result.handles.push(CollectionHandle {
                                place,
                                extraction,
                                setter,
                            });
                        }
                    }
                }
            }
            let Some(roles) = words.roles() else {
                continue;
            };
            for &(ordinal, role) in roles {
                if role != tcl_registry::ArgRole::LoopVarList {
                    continue;
                }
                let Some(container) = words
                    .original_argument_word(ordinal + 1)
                    .and_then(|word| scalar_occurrence(analysis, word))
                else {
                    continue;
                };
                let Some(place) = result.place_for(&container) else {
                    continue;
                };
                let Some(fields) = words
                    .original_argument_word(ordinal)
                    .and_then(tcl_syntax::word_rules::original_static_word_list_elements)
                else {
                    continue;
                };
                let keyed = words.with_source_schema(&context, |schema| {
                    schema
                        .facts()
                        .argument_type_hint(ordinal + 1)
                        .is_some_and(|hint| hint.expected == Some(tcl_registry::TclType::Dict))
                }) == Some(true);
                for (index, field) in fields.into_iter().enumerate() {
                    if keyed && index != 1 {
                        continue;
                    }
                    let Some(span) = field.source_span() else {
                        continue;
                    };
                    let Some(target) = analysis.original_variable_symbol_in_source(
                        &result.image,
                        input.lexer_config(),
                        span.start(),
                    ) else {
                        continue;
                    };
                    for body in words.source_script_bodies(&context) {
                        if body.inherits_immediate_caller_source_context(&context) {
                            result.loops.push(CollectionLoop {
                                place: place.clone(),
                                target: target.clone(),
                                field: field.clone(),
                                words: words.clone(),
                                body,
                                variable: container.clone(),
                            });
                        }
                    }
                }
            }
        }
        Some(result)
    }

    fn place_for(
        &self,
        variable: &SignatureSourceVariableOccurrence,
    ) -> Option<CollectionSourcePlace> {
        self.places
            .iter()
            .find(|(symbol, _)| symbol == variable.symbol())
            .map(|(_, place)| place.clone())
    }

    fn current(&self, source: &str, analysis: &AnalysisResult) -> bool {
        self.image.bytes() == source.as_bytes()
            && analysis.resolved_input.as_ref() == Some(&self.input)
            && analysis.matches_original_source_image(&self.image, self.input.lexer_config())
            && self.realm.matches_resolved_analysis_input(&self.input)
            && analysis.retained_command_realm().is_some_and(|realm| {
                std::ptr::eq(realm, self.realm.as_ref()) || realm == self.realm.as_ref()
            })
    }

    fn read_at(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        offset: u32,
    ) -> Option<CollectionRead> {
        let extraction = source_structure::source_registry_words_at(source, analysis, offset)?;
        collection_lookup_is_available(&extraction).then_some(())?;
        let context = self.input.context_registry();
        let container = extraction
            .with_source_schema(&context, |schema| {
                let tcl_registry::ReturnElements::ElementOf { container_arg } =
                    schema.semantics.return_elements?
                else {
                    return None;
                };
                let ordinal = schema
                    .semantics
                    .argument_offset
                    .checked_add(usize::from(container_arg))?;
                (schema.words.arguments().exact_argv_len() == Some(ordinal + 2)).then_some(ordinal)
            })
            .flatten()?;
        let variable = scalar_occurrence(analysis, extraction.original_argument_word(container)?)?;
        let place = self.place_for(&variable)?;
        Some(CollectionRead {
            extraction,
            variable,
            place,
        })
    }

    fn project_read(
        &self,
        read: &CollectionRead,
        path: OriginalCollectionReceiverPath,
    ) -> OriginalObjectCollectionSourceCandidates {
        OriginalObjectCollectionSourceCandidates {
            extraction: read.extraction.clone(),
            variable: read.variable.clone(),
            elements: self
                .writes
                .iter()
                .filter(|writer| writer.place == read.place)
                .map(|writer| writer.element.clone())
                .collect(),
            path,
        }
    }

    /// Source constructor associations for one genuine whole receiver word.
    /// Alternatives retain unknown contents; a read is never a selected value.
    #[must_use]
    pub fn for_receiver(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        receiver: &NativeWord,
        offset: u32,
    ) -> Option<OriginalObjectCollectionSourceCandidates> {
        self.current(source, analysis).then_some(())?;
        (receiver.image() == &self.image).then_some(())?;
        if let Some(child) = source_structure::original_single_source_command_substitution(receiver)
        {
            let offset = child.command().words.first()?.span().start();
            let read = self.read_at(source, analysis, offset)?;
            return Some(self.project_read(&read, OriginalCollectionReceiverPath::ElementRead));
        }
        let variable = scalar_occurrence(analysis, receiver)?;
        let place = self.place_for(&variable)?;
        let handles = self
            .handles
            .iter()
            .filter(|handle| handle.place == place)
            .filter_map(|handle| {
                self.read_at(source, analysis, handle.extraction)
                    .map(|read| (read, &handle.setter))
            })
            .collect::<Vec<_>>();
        if let Some((read, setter)) = handles.first() {
            if handles.iter().any(|(other, _)| other.place != read.place) {
                return None;
            }
            return Some(self.project_read(
                read,
                OriginalCollectionReceiverPath::HandleAssignment((*setter).clone()),
            ));
        }
        let loops = self
            .loops
            .iter()
            .filter(|binding| {
                binding.target.symbol() == variable.symbol()
                    && binding.body.content_span().start() <= offset
                    && offset < binding.body.content_span().end()
            })
            .collect::<Vec<_>>();
        let binding = loops.first()?;
        if loops.iter().any(|other| other.place != binding.place) {
            return None;
        }
        Some(OriginalObjectCollectionSourceCandidates {
            extraction: binding.words.clone(),
            variable: binding.variable.clone(),
            elements: self
                .writes
                .iter()
                .filter(|writer| writer.place == binding.place)
                .map(|writer| writer.element.clone())
                .collect(),
            path: OriginalCollectionReceiverPath::LoopBinding {
                words: binding.words.clone(),
                field: binding.field.clone(),
                body: binding.body.clone(),
            },
        })
    }
}

// A genuine source constructor does not establish an unchanged extractor or
// writer. Keep unknown lookup/earlier mutation terminal on the actual schema.
fn collection_lookup_is_available(words: &OriginalRegistryWords) -> bool {
    use crate::command_binding::{
        OriginalCatalogueSourceObligation as O, SourceCommandTransitionObligation as T,
    };
    use source_structure::OriginalRegistrySource;
    match words.source() {
        OriginalRegistrySource::Selected | OriginalRegistrySource::Scoped(_) => true,
        OriginalRegistrySource::DefinitionMember(_) => false,
        OriginalRegistrySource::SourceTransitions(advice) => {
            advice.uncertain_operations().is_empty()
                && !advice.obligations().contains(&T::UnknownEarlierMutation)
        }
        OriginalRegistrySource::Conditional(schema) => {
            !schema.obligations().iter().any(|obligation| {
                matches!(
                    obligation,
                    O::UnknownCurrentLookup
                        | O::UnknownLookupCoordinates
                        | O::UnretainedHolder(_)
                        | O::EarlierBindingAlternative(_)
                        | O::SelectedBindingAlternative(_)
                )
            })
        }
        OriginalRegistrySource::ProducedPrefix(_)
        | OriginalRegistrySource::ConditionalNativeSource(_)
        | OriginalRegistrySource::Vendor(_) => false,
    }
}

fn scalar_occurrence(
    analysis: &AnalysisResult,
    word: &NativeWord,
) -> Option<SignatureSourceVariableOccurrence> {
    if word.group().expand {
        return None;
    }
    let arena = word.executable_parts();
    let [part] = arena.list(arena.root()) else {
        return None;
    };
    let ExecutablePart::Variable { name, index: None } = part.part else {
        return None;
    };
    analysis
        .original_variable_symbol_in_source(
            word.image(),
            analysis.resolved_input.as_ref()?.lexer_config(),
            name.start(),
        )
        .cloned()
}
fn operand_occurrence(
    analysis: &AnalysisResult,
    word: &NativeWord,
) -> Option<SignatureSourceVariableOccurrence> {
    if word.group().expand {
        return None;
    }
    tcl_syntax::word_rules::original_static_word_unicode_value(word)?;
    analysis
        .original_variable_symbol_in_source(
            word.image(),
            analysis.resolved_input.as_ref()?.lexer_config(),
            word.tokens().first()?.span.start(),
        )
        .cloned()
}
/// One-shot adapter for callers that already hold an exact original getter.
#[must_use]
pub fn original_object_collection_sources_at(
    source: &str,
    analysis: &AnalysisResult,
    unit: &CompilationUnit,
    offset: u32,
) -> Option<OriginalObjectCollectionSourceCandidates> {
    let sources = OriginalObjectCollectionSources::capture(source, analysis, unit)?;
    let read = sources.read_at(source, analysis, offset)?;
    Some(sources.project_read(&read, OriginalCollectionReceiverPath::ElementRead))
}
fn collection_place(
    analysis: &AnalysisResult,
    unit: &CompilationUnit,
    owners: &super::OwnerIndex,
    occurrence: &SignatureSourceVariableOccurrence,
    offset: u32,
) -> Option<CollectionSourcePlace> {
    use crate::signature_scan::variable_symbol::{
        OriginalVariableSymbolReceiver, SignatureSourceVariableSlot,
    };
    let input = analysis.resolved_input.as_ref()?;
    // An actual formal declaration remains a stronger local owner.
    if analysis.original_variable_symbols.iter().any(|candidate| {
        candidate.receiver() == OriginalVariableSymbolReceiver::FormalDeclaration
            && candidate.symbol() == occurrence.symbol()
    }) {
        return Some(CollectionSourcePlace::Variable(occurrence.symbol().clone()));
    }
    let SignatureSourceVariableSlot::Local { simple, .. } = occurrence.symbol().slot() else {
        return Some(CollectionSourcePlace::Variable(occurrence.symbol().clone()));
    };
    let scope = owners
        .spans
        .iter()
        .filter(|scope| scope.span.start() <= offset && offset < scope.span.end())
        .min_by_key(|scope| scope.span.end() - scope.span.start())?;
    let tcl_core_types::BodyFrameId::Body(id) = scope.frame else {
        return Some(CollectionSourcePlace::Variable(occurrence.symbol().clone()));
    };
    let Some(method) = unit.ir_module.methods.get(&id) else {
        return Some(CollectionSourcePlace::Variable(occurrence.symbol().clone()));
    };
    let side = if method.kind == crate::ir::MethodKind::ClassMethod {
        MemberSide::ClassObject
    } else {
        MemberSide::Instance
    };
    let method_span = method.span?;
    let context = input.context_registry();
    for class in analysis.original_class_declarations() {
        let lexical = if matches!(
            method.kind,
            crate::ir::MethodKind::Constructor | crate::ir::MethodKind::Destructor
        ) {
            class
                .metadata()
                .original_special_members
                .declarations()
                .any(|candidate| {
                    candidate.side() == side
                        && candidate.site().offset == method_span.start()
                        && candidate.metadata().body_span.start() <= offset
                        && offset < candidate.metadata().body_span.end()
                })
        } else if input.has_logical_source_name_context() {
            analysis
                .logical_source_class_declaration(class.metadata())
                .is_some_and(|declaration| {
                    let methods = match side {
                        MemberSide::Instance => &class.metadata().methods,
                        MemberSide::ClassObject => &class.metadata().class_methods,
                    };
                    methods.values().any(|candidate| {
                        method_span.start() <= candidate.name_span.start()
                            && candidate.name_span.end() <= method_span.end()
                            && candidate.body_span.start() <= offset
                            && offset < candidate.body_span.end()
                            && declaration.member(side, candidate).is_some()
                    })
                })
        } else {
            class
                .metadata()
                .original_members
                .methods(side)?
                .iter()
                .any(|candidate| {
                    candidate.declaration().site().offset == method_span.start()
                        && candidate.metadata().body_span.start() <= offset
                        && offset < candidate.metadata().body_span.end()
                })
        };
        if !lexical {
            continue;
        }
        let factory = source_structure::source_registry_words_at(
            &unit.source,
            analysis,
            class.declaration_site().offset,
        )?;
        let grammar = factory.with_source_schema(&context, |schema| {
            schema.authored_source_definition_body_grammar()
        })??;
        for body in factory.source_script_bodies(&context) {
            let Some(region) = body.definition_member_region(&context, body.content_span()) else {
                continue;
            };
            for command in region.original_commands() {
                let values = command
                    .iter()
                    .map(tcl_syntax::word_rules::original_static_word_unicode_value)
                    .collect::<Option<Vec<_>>>()?;
                let words = values
                    .iter()
                    .map(|value| tcl_registry::InvocationWord::KnownBytes(value))
                    .collect::<Vec<_>>();
                let row = grammar.member_row(
                    0,
                    tcl_registry::InvocationArguments::structured(&words),
                    Some(context.context().authoring_query()),
                )?;
                let compatible = match row.receiver {
                    tcl_registry::definer::MemberReceiver::Both => true,
                    tcl_registry::definer::MemberReceiver::Instance => side == MemberSide::Instance,
                    tcl_registry::definer::MemberReceiver::TypeObject => {
                        side == MemberSide::ClassObject
                    }
                };
                if !compatible {
                    continue;
                }
                for declaration in region.variable_argument_words(command)? {
                    let matches = if input.has_logical_source_name_context() {
                        let name = tcl_syntax::word_rules::original_static_word_unicode_value(
                            declaration,
                        )?;
                        name.is_ascii() && name == simple.as_bytes()
                    } else {
                        let policy = occurrence.symbol().policy();
                        let name = crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                            declaration, tcl_syntax::word_rules::WordValueRules::from_config(&declaration.config()), policy,
                        )?;
                        let projection = policy.recipe().combined_variable_input(name.bytes());
                        projection.element().is_none()
                            && projection.root().qualification()
                                == tcl_syntax::naming::NativeNameQualification::Unqualified
                            && projection.root().selected() == simple.as_bytes()
                    };
                    if matches {
                        return Some(CollectionSourcePlace::DeclaredField {
                            class: class.clone(),
                            side,
                            declaration: declaration.clone(),
                        });
                    }
                }
            }
        }
    }
    Some(CollectionSourcePlace::Variable(occurrence.symbol().clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;
    fn unit(source: &str, analysis: &AnalysisResult) -> CompilationUnit {
        let input = analysis.resolved_input.as_ref().unwrap();
        CompilationUnit::build_with_analysis_input(
            source,
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
        )
    }
    #[test]
    fn original_collection_transport_requires_the_actual_cu_image_and_full_input() {
        let source =
            "oo::class create C {}; dict set values key [C new]; [dict get $values key] ping";
        for dialect in ["tcl", "tcl8.6", "tcl9.0"] {
            let analysis = Analyser::new().analyse(source, dialect);
            let mut actual = unit(source, &analysis);
            let sources =
                OriginalObjectCollectionSources::capture(source, &analysis, &actual).unwrap();
            let offset = u32::try_from(source.rfind("dict get").unwrap()).unwrap();
            let read = sources.read_at(source, &analysis, offset).unwrap();
            let candidates =
                sources.project_read(&read, OriginalCollectionReceiverPath::ElementRead);
            assert_eq!(candidates.elements().len(), 1);
            assert_eq!(
                candidates
                    .extraction()
                    .original_argument_word(0)
                    .unwrap()
                    .image(),
                &SourceImage::document(source)
            );
            assert_eq!(
                candidates.elements()[0].source_effect(),
                tcl_registry::VarElementsEffect::SetsDictValue
            );
            assert!(candidates.has_unknown_residual());
            actual.source.push(' ');
            assert!(OriginalObjectCollectionSources::capture(source, &analysis, &actual).is_none());
            let mut actual = unit(source, &analysis);
            actual.ir_module.source_metadata_input = None;
            assert!(OriginalObjectCollectionSources::capture(source, &analysis, &actual).is_none());
            let changed = format!("{source} ");
            let changed_analysis = Analyser::new().analyse(&changed, dialect);
            assert!(!sources.current(&changed, &changed_analysis));
            let mut stale = analysis.clone();
            stale.body_lexer_config.as_mut().unwrap().strict_quoting =
                !analysis.body_lexer_config.unwrap().strict_quoting;
            assert!(!sources.current(source, &stale));
        }
    }
    #[test]
    fn original_collection_constructor_cannot_clear_unknown_extractor_lookup() {
        let source = "dict set values key [C new]; [dict get $values key] ping";
        for dialect in ["tcl", "tcl8.6", "tcl9.0"] {
            let analysis = Analyser::new().analyse(source, dialect);
            let offset = u32::try_from(source.rfind("dict get").unwrap()).unwrap();
            let words = source_structure::source_registry_words_at(source, &analysis, offset)
                .expect("original source getter schema");
            match words.source() {
                source_structure::OriginalRegistrySource::SourceTransitions(advice) => assert!(
                    advice.obligations().contains(&crate::command_binding::SourceCommandTransitionObligation::UnknownEarlierMutation)
                        || !advice.uncertain_operations().is_empty()),
                source_structure::OriginalRegistrySource::Conditional(schema) => assert!(schema.obligations().iter().any(|obligation| matches!(obligation,
                    crate::command_binding::OriginalCatalogueSourceObligation::UnknownCurrentLookup
                    | crate::command_binding::OriginalCatalogueSourceObligation::UnknownLookupCoordinates
                    | crate::command_binding::OriginalCatalogueSourceObligation::UnretainedHolder(_)
                    | crate::command_binding::OriginalCatalogueSourceObligation::EarlierBindingAlternative(_)
                    | crate::command_binding::OriginalCatalogueSourceObligation::SelectedBindingAlternative(_)))),
                other => panic!("{dialect}: missing genuine unknown source barrier: {other:?}"),
            }
            assert!(!collection_lookup_is_available(&words));
            let actual = unit(source, &analysis);
            let sources =
                OriginalObjectCollectionSources::capture(source, &analysis, &actual).unwrap();
            assert!(sources.read_at(source, &analysis, offset).is_none());
        }
    }
}
