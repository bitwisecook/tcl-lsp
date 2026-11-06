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

//! Conditional original factory and blocking declaration headers.
use crate::analyser::{AnalysisResult, ClassDef, ProcDef, ResolvedAnalysisInput};
use crate::signature_scan::original_name::SourceDeclarationMetadata;
use crate::signature_scan::scope::SignatureSourceCommand;
use tcl_lexer::SourceImage;

#[derive(Debug, Clone, PartialEq, Eq)]
enum SourceHeader {
    Class {
        declaration: Box<SourceDeclarationMetadata<ClassDef>>,
        factory_provider: Option<Box<crate::command_binding::OriginalSourceClassRelationProvider>>,
        logical_slot: Option<tcl_core_types::ByteCommandSlot>,
        logical: Option<Box<crate::analyser::types::LogicalSourceClassDeclaration>>,
    },
    Procedure {
        declaration: Box<SourceDeclarationMetadata<ProcDef>>,
        logical_slot: Option<tcl_core_types::ByteCommandSlot>,
        logical: Option<Box<crate::analyser::types::LogicalSourceProcedureDeclaration>>,
    },
}

impl SourceHeader {
    fn name(&self) -> &SignatureSourceCommand {
        match self {
            Self::Class { declaration, .. } => declaration.name(),
            Self::Procedure { declaration, .. } => declaration.name(),
        }
    }
}

/// Authenticated conditional source headers; not a load or dispatch receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalProjectFactoryHeaders {
    input: ResolvedAnalysisInput,
    image: SourceImage,
    realm: std::sync::Arc<crate::realm::CommandBindingRealm>,
    headers: Vec<SourceHeader>,
}

impl OriginalProjectFactoryHeaders {
    /// Retain original source headers only when their analysis input and realm agree.
    #[must_use]
    pub fn capture(analysis: &AnalysisResult) -> Option<Self> {
        let input = analysis.resolved_input.as_ref()?;
        let realm = analysis.retained_command_realm()?;
        let image = realm.original_source_image()?;
        if !analysis.matches_original_source_image(image, input.lexer_config())
            || !realm.matches_resolved_analysis_input(input)
            || !analysis.command_aliases.is_empty()
        {
            return None;
        }
        let mut headers = Vec::new();
        for record in analysis.original_class_declarations() {
            if record.name_input().source_image() != image
                || record.name_input().lexer_config() != input.lexer_config()
            {
                return None;
            }
            let logical = analysis
                .logical_source_class_declaration(record.metadata())
                .cloned()
                .map(Box::new);
            let logical_slot = if input.has_logical_source_name_context() {
                Some(logical.as_ref()?.source_candidate_coordinate()?.clone())
            } else {
                None
            };
            let factory_provider = record.metadata().factory.as_ref().and_then(|factory| {
                let context = input.borrowed_context_registry();
                let provider = crate::analyser::class_hierarchy::original_metadata::original_registry_metaclass_provider(analysis, record.metadata(), context)?;
                let descriptor = provider.registry_descriptor(context)?;
                (descriptor.name == factory.root_metaclass).then_some(provider)
            });
            headers.push(SourceHeader::Class {
                declaration: Box::new(record.clone()),
                factory_provider: factory_provider.map(Box::new),
                logical_slot,
                logical,
            });
        }
        for record in analysis.original_procedure_declarations() {
            if record.name_input().source_image() != image
                || record.name_input().lexer_config() != input.lexer_config()
            {
                return None;
            }
            let logical = analysis
                .logical_source_procedure_declaration(record.metadata())
                .cloned()
                .map(Box::new);
            let logical_slot = if input.has_logical_source_name_context() {
                Some(logical.as_ref()?.source_candidate_coordinate()?)
            } else {
                None
            };
            headers.push(SourceHeader::Procedure {
                declaration: Box::new(record.clone()),
                logical_slot,
                logical,
            });
        }
        Some(Self {
            input: input.clone(),
            image: image.clone(),
            realm: std::sync::Arc::clone(analysis.retained_command_realm_arc()?),
            headers,
        })
    }

    /// Check that every header still belongs to the retained source owner.
    #[must_use]
    pub fn matches_declaring_owner(&self) -> bool {
        self.realm.matches_resolved_analysis_input(&self.input)
            && self
                .realm
                .matches_original_source_image(&self.image, self.input.lexer_config())
            && self.headers.iter().all(|header| match header {
                SourceHeader::Class {
                    declaration,
                    factory_provider,
                    logical_slot,
                    logical,
                } => {
                    factory_provider.as_ref().is_none_or(|provider| {
                        provider
                            .registry_descriptor(self.input.borrowed_context_registry())
                            .is_some()
                    }) && logical_slot.as_ref().is_none_or(|slot| {
                        logical
                            .as_deref()
                            .and_then(|owner| owner.source_candidate_coordinate())
                            == Some(slot)
                    }) && declaration.name_input().source_image() == &self.image
                        && declaration.name_input().lexer_config() == self.input.lexer_config()
                        && logical
                            .as_deref()
                            .is_none_or(|owner| owner.matches_source(&self.image, &self.input))
                }
                SourceHeader::Procedure {
                    declaration,
                    logical_slot,
                    logical,
                } => {
                    logical_slot.as_ref().is_none_or(|slot| {
                        logical
                            .as_deref()
                            .and_then(crate::analyser::types::LogicalSourceProcedureDeclaration::source_candidate_coordinate)
                            .as_ref()
                            == Some(slot)
                    }) && declaration.name_input().source_image() == &self.image
                        && declaration.name_input().lexer_config() == self.input.lexer_config()
                        && logical
                            .as_deref()
                            .is_none_or(|owner| owner.matches_source(&self.image, &self.input))
                }
            })
    }
}

// Select a unique source candidate under the caller lookup. Publication and
// unrepresented occupancy remain independent obligations.
fn select_native_factory<'a>(
    providers: &'a [(u64, OriginalProjectFactoryHeaders)],
    registry: &tcl_registry::CommandRegistry,
    tokens: &crate::ir::CommandTokens,
) -> Option<(
    u64,
    &'a OriginalProjectFactoryHeaders,
    &'a SourceDeclarationMetadata<ClassDef>,
    &'a crate::command_binding::OriginalSourceClassRelationProvider,
)> {
    if providers
        .iter()
        .any(|(_, provider)| !provider.matches_declaring_owner())
    {
        return None;
    }
    let lookup = tokens
        .source_binding
        .as_ref()?
        .original_static_head_lookup_for_tokens(tokens)?;
    let headers = providers
        .iter()
        .flat_map(|(identity, provider)| {
            provider
                .headers
                .iter()
                .map(move |header| (*identity, provider, header))
        })
        .collect::<Vec<_>>();
    let chosen = lookup.matching_external_source_advice_publications(
        registry,
        headers
            .iter()
            .enumerate()
            .map(|(index, (_, _, header))| (header.name(), index)),
    )?;
    let [index] = chosen.as_slice() else {
        return None;
    };
    let (
        identity,
        provider,
        SourceHeader::Class {
            declaration,
            factory_provider: Some(factory_provider),
            ..
        },
    ) = headers[*index]
    else {
        return None;
    };
    declaration.metadata().factory.as_ref()?;
    Some((identity, provider, declaration, factory_provider))
}

// Select the first occupied Logical source coordinate without claiming dispatch.
fn select_logical_factory<'a>(
    providers: &'a [(u64, OriginalProjectFactoryHeaders)],
    candidates: &crate::command_binding::OriginalLogicalCommandCandidates,
) -> Option<(
    u64,
    &'a OriginalProjectFactoryHeaders,
    &'a SourceDeclarationMetadata<ClassDef>,
    &'a crate::command_binding::OriginalSourceClassRelationProvider,
)> {
    if candidates.has_earlier_unknown_mutations()
        || candidates.has_lookup_operand_effects()
        || providers
            .iter()
            .any(|(_, provider)| !provider.matches_declaring_owner())
    {
        return None;
    }
    let headers = providers
        .iter()
        .flat_map(|(identity, provider)| {
            provider
                .headers
                .iter()
                .map(move |header| (*identity, provider, header))
        })
        .collect::<Vec<_>>();
    for slot in candidates.external_header_coordinates()? {
        let matching = headers
            .iter()
            .filter(|(_, _, header)| match header {
                SourceHeader::Class { logical_slot, .. }
                | SourceHeader::Procedure { logical_slot, .. } => {
                    logical_slot.as_ref() == Some(slot)
                }
            })
            .collect::<Vec<_>>();
        if matching.is_empty() {
            continue;
        }
        let [selected] = matching.as_slice() else {
            return None;
        };
        let (identity, provider, header) = **selected;
        let SourceHeader::Class {
            declaration,
            factory_provider: Some(factory_provider),
            ..
        } = header
        else {
            return None;
        };
        declaration.metadata().factory.as_ref()?;
        return Some((identity, provider, declaration, factory_provider));
    }
    None
}

/// Conditions retained on cross-document source metadata. Neither condition is
/// discharged by capturing the written call or its definition-body geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForeignFactorySourceObligation {
    /// The foreign source publication must be available at the caller horizon.
    PublicationAvailable,
    /// No unrepresented or conflicting foreign command occupies the lookup path.
    NoConflictingForeignOccupant,
    /// The selected manufacturer and its source layout must apply at the call.
    ManufacturerApplicable,
    /// Both document snapshots and the project membership must still be current.
    ProjectSnapshotCurrent,
}

/// One conditional foreign factory body under both immutable source owners.
/// It never issues an `OriginalRegistryWords`, entered worker, Native result,
/// or Normal completion. Project snapshot currentness remains a host gate.
#[derive(Debug, Clone)]
pub struct OriginalForeignFactoryBody {
    declaring_document: u64,
    caller_document: u64,
    provider: OriginalProjectFactoryHeaders,
    declaration: SourceDeclarationMetadata<ClassDef>,
    factory_provider: crate::command_binding::OriginalSourceClassRelationProvider,
    caller_input: ResolvedAnalysisInput,
    caller_image: SourceImage,
    caller_realm: std::sync::Arc<crate::realm::CommandBindingRealm>,
    projection: std::sync::Arc<crate::registry_invocation::OriginalSourceCommandProjection>,
    grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
    manufacturer: &'static tcl_registry::definer::ManufacturerMethod,
    name_ordinal: usize,
    body_ordinal: usize,
    obligations: [ForeignFactorySourceObligation; 4],
}

impl OriginalForeignFactoryBody {
    fn source_call<'a>(
        source: &str,
        analysis: &'a AnalysisResult,
        segment: &crate::segmenter::SegmentedCommand,
        caller_document: u64,
        providers: &[(u64, OriginalProjectFactoryHeaders)],
    ) -> Option<(
        &'a ResolvedAnalysisInput,
        SourceImage,
        crate::ir::CommandTokens,
        std::sync::Arc<crate::registry_invocation::OriginalSourceCommandProjection>,
    )> {
        let input = analysis.resolved_input.as_ref()?;
        let config = analysis.body_lexer_config?;
        let image = SourceImage::document(source);
        let mut caller_headers = providers
            .iter()
            .filter(|(identity, _)| *identity == caller_document);
        let (_, caller_header) = caller_headers.next()?;
        if caller_headers.next().is_some()
            || caller_header.input != *input
            || caller_header.image != image
            || !caller_header.matches_declaring_owner()
        {
            return None;
        }
        if config != input.lexer_config()
            || !analysis.matches_original_source_image(&image, config)
            || segment
                .expand_word
                .as_ref()
                .is_some_and(|words| words.iter().any(|word| *word))
        {
            return None;
        }
        let realm = analysis.retained_command_realm()?;
        realm.matches_resolved_analysis_input(input).then_some(())?;
        let offset = segment.argv.first()?.span.start();
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(&image),
            config,
            segment,
        );
        realm.stamp_original_tokens(&mut tokens);
        let binding = tokens.source_binding.as_ref()?;
        let projection = if input.has_logical_source_name_context() {
            realm
                .source_bindings_ref()
                .original_logical_source_command_at_in_source(input, &image, offset, config)?
        } else {
            binding.invocation_site()?.source.original_lexical_command(
                tokens.words(),
                offset,
                config,
            )?
        };
        if projection.command() != segment
            || projection.native_words().iter().any(|word| {
                word.group().expand
                    || word
                        .executable_parts()
                        .all_parts()
                        .any(|part| !matches!(part.part, tcl_lexer::ExecutablePart::Text(_)))
            })
            || !crate::command_binding::SourceCommandBindings::original_source_prefix_arity_debts(
                realm.source_bindings_ref(),
                input,
                projection.native_words(),
            )?
            .is_empty()
        {
            return None;
        }
        Some((input, image, tokens, projection))
    }

    fn selected_grammar(
        input: &ResolvedAnalysisInput,
        provider: &OriginalProjectFactoryHeaders,
        declaration: &SourceDeclarationMetadata<ClassDef>,
        factory_provider: &crate::command_binding::OriginalSourceClassRelationProvider,
    ) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
        let class = declaration.metadata();
        if class.source_name.as_ref() != Some(declaration.name())
            || class.source_name_ambiguous.is_observed()
            || declaration.declaration_site().source.source_image() != &provider.image
            || declaration.name_input().source_image() != &provider.image
            || declaration.name_input().lexer_config() != provider.input.lexer_config()
            || declaration
                .name_input()
                .original_word()
                .tokens()
                .first()
                .is_none_or(|token| token.span != class.name_span)
        {
            return None;
        }
        let factory = class.factory.as_ref()?;
        let context = input.borrowed_context_registry();
        let descriptor = context
            .context()
            .resolve_spec(context.commands(), &factory.root_metaclass)?;
        let declaring_descriptor =
            factory_provider.registry_descriptor(provider.input.borrowed_context_registry())?;
        if !std::ptr::eq(descriptor, declaring_descriptor) {
            return None;
        }
        if !descriptor
            .traits
            .contains(tcl_registry::Traits::IS_OO_METACLASS)
        {
            return None;
        }
        let grammar = descriptor.definition_body?;
        (grammar.family == tcl_registry::definer::DefinerFamily::TclOo).then_some(())?;
        Some(grammar)
    }

    /// Capture the original written call under a unique selected foreign
    /// header, using the caller's retained source graph and complete vector.
    fn capture(
        source: &str,
        analysis: &AnalysisResult,
        segment: &crate::segmenter::SegmentedCommand,
        caller_document: u64,
        providers: &[(u64, OriginalProjectFactoryHeaders)],
    ) -> Option<Self> {
        let (input, image, tokens, projection) =
            Self::source_call(source, analysis, segment, caller_document, providers)?;
        let config = input.lexer_config();
        let offset = segment.argv.first()?.span.start();
        let binding = tokens.source_binding.as_ref()?;
        let registry = analysis.resolved_registry()?;
        let (declaring_document, provider, declaration, factory_provider) =
            if input.has_logical_source_name_context() {
                let candidates =
                crate::registry_invocation::source_structure::source_logical_command_candidates_at(
                    source, analysis, offset,
                )?;
                (candidates.original_words() == projection.native_words()).then_some(())?;
                let selected = select_logical_factory(providers, &candidates)?;
                let policy = tcl_registry::InvocationDialect::of_profile(input.unit_profile())
                    .authored_name_policy()?;
                (selected.2.name().policy() == policy).then_some(())?;
                selected
            } else {
                select_native_factory(providers, registry, &tokens)?
            };
        if declaring_document == caller_document {
            return None;
        }
        let grammar = Self::selected_grammar(input, provider, declaration, factory_provider)?;
        let factory = declaration.metadata().factory.as_ref()?;
        let keyword = if input.has_logical_source_name_context() {
            tcl_syntax::word_rules::original_static_word_unicode_value(
                projection.native_words().get(1)?,
            )?
        } else {
            binding
                .original_written_name_input(&tokens, 1)?
                .bytes()
                .to_vec()
        };
        let keyword = std::str::from_utf8(&keyword).ok()?;
        let manufacturer = grammar.manufacturer(keyword)?;
        factory
            .exported_manufacturers
            .contains(manufacturer.keyword)
            .then_some(())?;
        let (name_argument, body_argument) =
            factory.overrides.get(manufacturer.keyword).map_or_else(
                || {
                    Some((
                        usize::from(manufacturer.names_instance_at?),
                        usize::from(manufacturer.definition_body_at?),
                    ))
                },
                |shape| Some((shape.name_arg, shape.body_arg)),
            )?;
        (name_argument != 0 && body_argument != 0 && name_argument != body_argument)
            .then_some(())?;
        let name_ordinal = name_argument.checked_add(1)?;
        let body_ordinal = body_argument.checked_add(1)?;
        let name_word = projection.native_words().get(name_ordinal)?;
        let body_word = projection.native_words().get(body_ordinal)?;
        if name_word.image() != &image
            || body_word.image() != &image
            || name_word.config() != config
            || body_word.config() != config
        {
            return None;
        }
        let content = body_word.content_span().ok()?;
        (tcl_syntax::word_rules::original_static_word_source_bytes(body_word).as_deref()
            == image.bytes().get(content.as_range()))
        .then_some(())?;
        Some(Self {
            declaring_document,
            caller_document,
            provider: provider.clone(),
            declaration: declaration.clone(),
            factory_provider: factory_provider.clone(),
            caller_input: input.clone(),
            caller_image: image,
            caller_realm: std::sync::Arc::clone(analysis.retained_command_realm_arc()?),
            projection,
            grammar,
            manufacturer,
            name_ordinal,
            body_ordinal,
            obligations: [
                ForeignFactorySourceObligation::PublicationAvailable,
                ForeignFactorySourceObligation::NoConflictingForeignOccupant,
                ForeignFactorySourceObligation::ManufacturerApplicable,
                ForeignFactorySourceObligation::ProjectSnapshotCurrent,
            ],
        })
    }

    /// Host-scoped identity of the selected declaring document.
    #[must_use]
    pub const fn declaring_document(&self) -> u64 {
        self.declaring_document
    }

    /// Host-scoped identity of the caller document.
    #[must_use]
    pub const fn caller_document(&self) -> u64 {
        self.caller_document
    }

    /// The selected source manufacturer; no dispatch follows.
    #[must_use]
    pub fn manufacturer(&self) -> &'static tcl_registry::definer::ManufacturerMethod {
        self.manufacturer
    }

    /// Original body word and source-only grammar; no body execution follows.
    #[must_use]
    pub fn body(&self) -> &tcl_lexer::NativeWord {
        &self.projection.native_words()[self.body_ordinal]
    }
    /// Original written instance name in the caller document.
    #[must_use]
    pub fn name(&self) -> &tcl_lexer::NativeWord {
        &self.projection.native_words()[self.name_ordinal]
    }
    /// Unresolved publication and applicability conditions of this source advice.
    #[must_use]
    pub fn obligations(&self) -> &[ForeignFactorySourceObligation] {
        &self.obligations
    }
    /// The unchanged declaring source graph provider, including its retained
    /// parent advice and its separate applicability obligations.
    #[must_use]
    pub fn factory_provider(&self) -> &crate::command_binding::OriginalSourceClassRelationProvider {
        &self.factory_provider
    }

    /// The current declaring analysis must still own this exact source realm,
    /// input and one unique original declaration. Snapshot generation remains
    /// independently checked by the project host.
    #[must_use]
    pub fn matches_declaring(&self, analysis: &AnalysisResult) -> bool {
        if analysis.resolved_input.as_ref() != Some(&self.provider.input)
            || !analysis
                .retained_command_realm()
                .is_some_and(|realm| std::ptr::eq(realm, self.provider.realm.as_ref()))
            || !analysis.matches_original_source_image(
                &self.provider.image,
                self.provider.input.lexer_config(),
            )
            || !self.provider.matches_declaring_owner()
        {
            return false;
        }
        let context = self.provider.input.borrowed_context_registry();
        if crate::analyser::class_hierarchy::original_metadata::original_registry_metaclass_provider(
            analysis, self.declaration.metadata(), context,
        ).as_ref() != Some(&self.factory_provider) { return false; }
        let mut matches = analysis
            .original_class_declarations()
            .filter(|record| *record == &self.declaration);
        matches.next().is_some() && matches.next().is_none()
    }
    /// Recheck the exact caller source, input and retained realm.
    #[must_use]
    pub fn matches_caller(&self, analysis: &AnalysisResult) -> bool {
        analysis.resolved_input.as_ref() == Some(&self.caller_input)
            && analysis
                .retained_command_realm()
                .is_some_and(|realm| std::ptr::eq(realm, self.caller_realm.as_ref()))
            && self
                .caller_realm
                .matches_resolved_analysis_input(&self.caller_input)
            && analysis
                .matches_original_source_image(&self.caller_image, self.caller_input.lexer_config())
            && self.name().image() == &self.caller_image
            && self.body().image() == &self.caller_image
    }
}

/// Source identity for a conditional class syntax occurrence. It retains the
/// foreign declaration and caller body without selecting a runtime class.
#[derive(Debug, Clone)]
pub struct OriginalForeignFactoryClassSyntax {
    body: std::sync::Arc<OriginalForeignFactoryBody>,
    input: crate::signature_scan::scope::SignatureSourceNameInput,
}

impl OriginalForeignFactoryClassSyntax {
    /// The exact foreign factory and caller source parents.
    #[must_use]
    pub fn body(&self) -> &OriginalForeignFactoryBody {
        &self.body
    }
    /// The original class name operand.
    #[must_use]
    pub fn name(&self) -> &tcl_lexer::NativeWord {
        self.body.name()
    }
    /// The exact caller-side source name input; it selects no namespace or cell.
    #[must_use]
    pub fn name_input(&self) -> &crate::signature_scan::scope::SignatureSourceNameInput {
        &self.input
    }
    /// Written callable candidates retain this same parent.
    #[must_use]
    pub fn callable_syntax(&self) -> Option<Vec<ForeignFactoryMemberSyntax>> {
        self.body.callable_syntax(self)
    }
}

/// One written member and its exact conditional foreign class parent.
#[derive(Debug, Clone)]
pub struct ForeignFactoryMemberSyntax {
    parent: OriginalForeignFactoryClassSyntax,
    command: Vec<tcl_lexer::NativeWord>,
    name_ordinal: usize,
    input: crate::signature_scan::scope::SignatureSourceNameInput,
    purpose: tcl_registry::definer::DefinitionMemberNamePurpose,
    name: String,
}

impl ForeignFactoryMemberSyntax {
    /// The caller's original written class source identity.
    #[must_use]
    pub fn parent(&self) -> &OriginalForeignFactoryClassSyntax {
        &self.parent
    }
    /// The complete original member command words.
    #[must_use]
    pub fn command(&self) -> &[tcl_lexer::NativeWord] {
        &self.command
    }
    /// The original member name word.
    #[must_use]
    pub fn name_word(&self) -> &tcl_lexer::NativeWord {
        &self.command[self.name_ordinal]
    }
    /// Original method source input, without any selected method table.
    #[must_use]
    pub fn name_input(&self) -> &crate::signature_scan::scope::SignatureSourceNameInput {
        &self.input
    }
    /// Registry-selected member name purpose.
    #[must_use]
    pub const fn name_purpose(&self) -> tcl_registry::definer::DefinitionMemberNamePurpose {
        self.purpose
    }
    /// A source spelling for display, with no dispatch implication.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl OriginalForeignFactoryBody {
    /// Retain the written class operand under this exact conditional parent.
    #[must_use]
    pub fn class_syntax(&self) -> Option<OriginalForeignFactoryClassSyntax> {
        let purpose =
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                &self.caller_input,
                None,
                self.declaration.name().policy(),
            );
        let key = purpose.key(
            self.name(),
            tcl_syntax::word_rules::WordValueRules::from_config(&self.name().config()),
        )?;
        Some(OriginalForeignFactoryClassSyntax {
            body: std::sync::Arc::new(self.clone()),
            input: crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(key),
        })
    }

    /// Written callable members only. This does not select an installer or
    /// prove that any member enters a method table.
    #[must_use]
    fn callable_syntax(
        &self,
        parent: &OriginalForeignFactoryClassSyntax,
    ) -> Option<Vec<ForeignFactoryMemberSyntax>> {
        use tcl_registry::definer::{CallableRole, MemberEffect};
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            self.caller_image.clone(),
            self.body().content_span().ok()?,
            self.body().config(),
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .ok()?;
        if plan.fatal_tail.is_some() {
            return None;
        }
        let mut result = Vec::new();
        for command in &plan.commands {
            if command.words.iter().any(|word| word.group().expand) {
                return None;
            }
            let values: Vec<_> = command
                .words
                .iter()
                .map(|word| {
                    tcl_syntax::word_rules::original_static_word_unicode_value(word)
                        .and_then(|bytes| String::from_utf8(bytes).ok())
                })
                .collect();
            let words: Vec<_> = values
                .iter()
                .map(|value| {
                    value.as_deref().map_or(
                        tcl_registry::InvocationWord::Dynamic,
                        tcl_registry::InvocationWord::Literal,
                    )
                })
                .collect();
            let Some(row) = self.grammar.member_row(
                0,
                tcl_registry::InvocationArguments::structured(&words),
                Some(
                    self.caller_input
                        .borrowed_context_registry()
                        .context()
                        .authoring_query(),
                ),
            ) else {
                continue;
            };
            let MemberEffect::Callable {
                role: CallableRole::Method,
                name_slot: Some(slot),
                ..
            } = row.effect
            else {
                continue;
            };
            let ordinal = row
                .keyword_index
                .checked_add(1)?
                .checked_add(usize::from(slot))?;
            let member = self
                .grammar
                .member(values.get(row.keyword_index)?.as_deref()?)?;
            let member_purpose = self.grammar.member_name_purpose(member)?;
            let word = command.words.get(ordinal)?;
            let name_purpose =
                crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                    &self.caller_input,
                    None,
                    self.declaration.name().policy(),
                );
            let key = name_purpose.key(
                word,
                tcl_syntax::word_rules::WordValueRules::from_config(&word.config()),
            )?;
            let input = crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(key);
            if !member_purpose.admits(input.policy().recipe(), input.bytes()) {
                return None;
            }
            let name = std::str::from_utf8(input.bytes()).ok()?.to_owned();
            result.push(ForeignFactoryMemberSyntax {
                parent: parent.clone(),
                command: command.words.clone(),
                name_ordinal: ordinal,
                input,
                purpose: member_purpose,
                name,
            });
        }
        Some(result)
    }

    /// The original call's complete span, without entered-command authority.
    #[must_use]
    pub fn command_span(&self) -> tcl_lexer::Span {
        self.projection.command().span
    }
}

/// Conditional original-source factory calls in the current top-level script.
/// Every result retains unresolved publication, applicability and currentness
/// obligations; no source declaration or runtime receiver follows.
#[must_use]
pub fn original_foreign_factory_bodies(
    source: &str,
    analysis: &AnalysisResult,
    caller_document: u64,
    headers: &[(u64, OriginalProjectFactoryHeaders)],
) -> Vec<OriginalForeignFactoryBody> {
    let Some(input) = analysis.resolved_input.as_ref() else {
        return Vec::new();
    };
    if crate::source_graph::current_analysis(source, analysis).is_none() {
        return Vec::new();
    }
    crate::segmenter::segment_commands_with_offset_and_config(source, 0, input.lexer_config())
        .iter()
        .filter_map(|segment| {
            OriginalForeignFactoryBody::capture(source, analysis, segment, caller_document, headers)
        })
        .collect()
}
