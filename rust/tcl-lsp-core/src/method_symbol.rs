// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Readonly original method operands and independently owned source candidates.
//!
//! A call-point receiver receipt and a declaration candidate are different
//! purposes. Both retain exact inputs and canonical declarations; declaration
//! metadata supplies no live object, native dispatch, edit or execution grant.

use std::ops::ControlFlow;
use tcl_compiler::analyser::types::{
    MemberSide, OriginalSourceMemberDeclaration, OriginalSourceMethodMetadata,
};
use tcl_compiler::analyser::{AnalysisResult, ClassDef, ProcDef};
use tcl_compiler::command_binding::{
    CommandAllocationSite, OriginalCommandLookup, OriginalSourceConstructorCall,
};
use tcl_compiler::ir::CommandTokens;
use tcl_compiler::segmenter::SegmentedCommand;
use tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata;
use tcl_compiler::signature_scan::scope::SignatureSourceNameInput;
use tcl_lexer::{LineIndex, SourceImage, SourceMap, Span};

/// Readonly navigation purpose, independent of lookup or edit authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalMethodNavigation {
    /// The independently owned canonical declaration.
    Declaration,
    /// Authentic selector occurrences, with an explicit declaration policy.
    References {
        /// Include the canonical declaring operand among the source occurrences.
        include_declaration: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OwnObjectMethod {
    configuration: tcl_compiler::command_binding::OriginalSourceObjectConfigurationTarget,
    method: OriginalSourceMethodMetadata,
    entry: Option<(
        tcl_compiler::command_binding::SourceReceiverMethodEntry,
        u64,
    )>,
    call_point: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Target {
    Declaration {
        class: CommandAllocationSite,
        method: OriginalSourceMemberDeclaration,
        call_point: bool,
    },
    OwnObject(Box<OwnObjectMethod>),
    SourceClassCall(Box<OriginalSourceConstructorCall>),
    ClassLookup {
        lookup: OriginalCommandLookup,
        known_class: bool,
    },
}

/// Authentic queried member operand. Construction is confined to the current
/// source scanner or an independently retained temporal receiver selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalMethodQuery {
    class_input: Option<SignatureSourceNameInput>,
    selector_input: SignatureSourceNameInput,
    side: MemberSide,
    span: Span,
    is_declaration: bool,
    consumer_image: SourceImage,
    consumer_config: tcl_lexer::LexerConfig,
    target: Target,
}

impl OriginalMethodQuery {
    fn without_call_point(&mut self) {
        match &mut self.target {
            Target::Declaration { call_point, .. } => *call_point = false,
            Target::OwnObject(own) => own.call_point = false,
            Target::SourceClassCall(_) | Target::ClassLookup { .. } => {}
        }
    }

    /// Class operand or selected canonical class declaration producer. An
    /// object-value head never supplies a reconstructed class command name.
    #[must_use]
    pub const fn class_input(&self) -> Option<&SignatureSourceNameInput> {
        self.class_input.as_ref()
    }
    /// Original queried route, independently of the canonical body name.
    #[must_use]
    pub const fn selector_input(&self) -> &SignatureSourceNameInput {
        &self.selector_input
    }
    /// Independently retained receiver side.
    #[must_use]
    pub const fn side(&self) -> MemberSide {
        self.side
    }
    /// Readonly operand extent; it supplies no replacement-source authority.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
    /// Whether this occurrence is the canonical declaration word.
    #[must_use]
    pub const fn is_declaration(&self) -> bool {
        self.is_declaration
    }
    /// Complete independently retained consumer source, not a member producer image.
    #[must_use]
    pub const fn consumer_image(&self) -> &SourceImage {
        &self.consumer_image
    }
    /// Every scanner override belonging to that consumer.
    #[must_use]
    pub const fn consumer_config(&self) -> tcl_lexer::LexerConfig {
        self.consumer_config
    }
}

/// A document-owned source member. Canonical identity includes the owning URI
/// even when two documents contain exactly the same bytes and sites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalMethodCandidate {
    uri: String,
    class_input: Option<SignatureSourceNameInput>,
    class: Option<CommandAllocationSite>,
    object: Option<tcl_compiler::command_binding::OriginalSourceObjectConfigurationTarget>,
    method: OriginalSourceMethodMetadata,
    call_point: bool,
}

impl OriginalMethodCandidate {
    /// Own-object declaration owner; this source receipt grants no class table.
    #[must_use]
    pub const fn own_object(
        &self,
    ) -> Option<&tcl_compiler::command_binding::OriginalSourceObjectConfigurationTarget> {
        self.object.as_ref()
    }

    /// Owning source document.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }
    /// Exact canonical declaration geometry.
    #[must_use]
    pub fn declaration_span(&self) -> Span {
        self.method.declaration().original_word().span()
    }
    /// Canonical source metadata, separate from the actual queried route.
    #[must_use]
    pub const fn method(&self) -> &OriginalSourceMethodMetadata {
        &self.method
    }
    /// Complete source owning the canonical declaration.
    #[must_use]
    pub fn declaration_image(&self) -> &SourceImage {
        self.method.declaration().original_word().image()
    }
    /// Selected declaration scanner configuration.
    #[must_use]
    pub fn declaration_config(&self) -> tcl_lexer::LexerConfig {
        self.method.declaration().original_word().config()
    }
    /// This occurrence had its own retained temporal receiver entry. False
    /// means declaration advice; it never means a fabricated native dispatch.
    #[must_use]
    pub const fn has_call_point_entry(&self) -> bool {
        self.call_point
    }
    /// Full source declaration identity, independent of reporting labels.
    #[must_use]
    pub fn same_declaration(&self, other: &Self) -> bool {
        self.uri == other.uri
            && self.class == other.class
            && self.object == other.object
            && self.method.side() == other.method.side()
            && self.method.declaration() == other.method.declaration()
    }
    /// Canonical declaring class in this complete current source inventory.
    /// This source join grants navigation, independently of runtime ancestry.
    #[must_use]
    pub fn declaring_class_in<'a>(
        &self,
        source: &str,
        analysis: &'a AnalysisResult,
    ) -> Option<&'a SourceDeclarationMetadata<ClassDef>> {
        if !current(source, analysis) || self.declaration_image() != &SourceImage::document(source)
        {
            return None;
        }
        let mut records = analysis.original_class_declarations().filter(|record| {
            Some(record.declaration_site()) == self.class.as_ref()
                && record
                    .metadata()
                    .original_members
                    .declarations()
                    .any(|method| {
                        method.side() == self.method.side()
                            && method.declaration() == self.method.declaration()
                            && method.metadata() == self.method.metadata()
                    })
        });
        let first = records.next()?;
        records.next().is_none().then_some(first)
    }
    /// Render names only after selection. No label participates in lookup.
    #[must_use]
    pub fn hover(&self) -> Option<crate::hover::Hover> {
        if let Some(object) = &self.object {
            let method = crate::original_oo::method_label(&self.method)?;
            let owner = object
                .name_input()
                .map(|input| tcl_syntax::native_string::resident_name_label(input.bytes()));
            let label = owner.map_or_else(
                || format!("**Own method** `{method}`"),
                |owner| format!("**Own method** `{owner} {method}`"),
            );
            let purpose = if self.call_point {
                "Resolved at this call"
            } else {
                "Possible source declaration"
            };
            return Some(crate::hover::Hover {
                kind: crate::hover::HoverKind::Markdown,
                value: format!("{label}\n\n{purpose}."),
            });
        }
        let input = self.class_input.as_ref()?;
        let class = tcl_syntax::native_string::resident_name_label(input.bytes());
        let method = crate::original_oo::method_label(&self.method)?;
        let kind = match self.method.side() {
            MemberSide::Instance => "method",
            MemberSide::ClassObject => "classmethod",
        };
        let purpose = if self.call_point {
            "Resolved at this call"
        } else {
            "Possible source declaration"
        };
        let parameters = if self.method.metadata().params_computed {
            "unknown parameters".to_owned()
        } else {
            format!("{} parameter(s)", self.method.metadata().params.len())
        };
        Some(crate::hover::Hover {
            value: format!("**{kind}** `{class}::{method}` ({parameters})\n\n{purpose}."),
            kind: crate::hover::HoverKind::Markdown,
        })
    }
}

fn current(source: &str, analysis: &AnalysisResult) -> bool {
    analysis.body_lexer_config.is_some_and(|config| {
        analysis.matches_original_source_image(&SourceImage::document(source), config)
    })
}

fn declaration_query(
    class: &SourceDeclarationMetadata<ClassDef>,
    method: &OriginalSourceMethodMetadata,
    input: SignatureSourceNameInput,
    span: Span,
    is_declaration: bool,
    call_point: bool,
    currency: (SourceImage, tcl_lexer::LexerConfig),
) -> OriginalMethodQuery {
    OriginalMethodQuery {
        class_input: Some(SignatureSourceNameInput::OriginalWord(
            class.name_input().clone(),
        )),
        selector_input: input,
        side: method.side(),
        span,
        is_declaration,
        consumer_image: currency.0,
        consumer_config: currency.1,
        target: Target::Declaration {
            class: class.declaration_site().clone(),
            method: method.declaration().clone(),
            call_point,
        },
    }
}

fn consumer_currency(analysis: &AnalysisResult) -> Option<(SourceImage, tcl_lexer::LexerConfig)> {
    Some((
        analysis
            .retained_command_realm()?
            .original_source_image()?
            .clone(),
        analysis.body_lexer_config?,
    ))
}

fn own_query(
    configuration: &tcl_compiler::command_binding::OriginalSourceObjectConfigurationTarget,
    method: &OriginalSourceMethodMetadata,
    selector: (SignatureSourceNameInput, Span),
    is_declaration: bool,
    entry: Option<(
        tcl_compiler::command_binding::SourceReceiverMethodEntry,
        u64,
    )>,
    call_point: bool,
    currency: (SourceImage, tcl_lexer::LexerConfig),
) -> OriginalMethodQuery {
    let (selector_input, span) = selector;
    let (consumer_image, consumer_config) = currency;
    OriginalMethodQuery {
        class_input: None,
        selector_input,
        side: method.side(),
        span,
        is_declaration,
        consumer_image,
        consumer_config,
        target: Target::OwnObject(Box::new(OwnObjectMethod {
            configuration: configuration.clone(),
            method: method.clone(),
            entry,
            call_point,
        })),
    }
}

fn own_candidate(
    uri: &str,
    query: &OriginalMethodQuery,
) -> ControlFlow<Option<OriginalMethodCandidate>> {
    let Target::OwnObject(own) = &query.target else {
        return ControlFlow::Break(None);
    };
    let word = own.method.declaration().original_word();
    if word.image() != query.consumer_image()
        || word.config() != query.consumer_config()
        || own.configuration.site().source.source_image() != query.consumer_image()
        || own.method.side() != query.side()
        || own.call_point && own.entry.is_none()
        || own.entry.as_ref().is_some_and(|(entry, _)| {
            entry.declaring_object() != Some(own.configuration.instance().allocation())
                || entry.declaration() != own.method.declaration().site()
                || entry.original_name_input() != own.method.original_name_input()
        })
    {
        return ControlFlow::Break(None);
    }
    ControlFlow::Break(Some(OriginalMethodCandidate {
        uri: uri.to_owned(),
        class_input: None,
        class: None,
        object: Some(own.configuration.clone()),
        method: own.method.clone(),
        call_point: own.call_point,
    }))
}

fn temporal_query(
    analysis: &AnalysisResult,
    selected: &crate::receiver_identity::RetainedReceiverMethod<'_>,
) -> Option<OriginalMethodQuery> {
    if let Some((entry, generation)) = &selected.own_entry {
        let allocation = entry.declaring_object()?;
        let mut configurations =
            analysis
                .original_object_configurations()
                .filter(|configuration| {
                    configuration.target().instance().allocation() == allocation
                        && configuration
                            .members()
                            .declarations()
                            .any(|method| method == selected.metadata)
                });
        let configuration = configurations.next()?;
        if configurations.next().is_some() {
            return None;
        }
        let (consumer_image, consumer_config) = consumer_currency(analysis)?;
        return Some(own_query(
            configuration.target(),
            selected.metadata,
            (
                selected.original_selector_input().clone(),
                selected.selector,
            ),
            false,
            Some((entry.clone(), *generation)),
            true,
            (consumer_image, consumer_config),
        ));
    }
    let side = match selected.receiver {
        tcl_compiler::command_binding::SourceMethodReceiver::Instance => MemberSide::Instance,
        tcl_compiler::command_binding::SourceMethodReceiver::Class => MemberSide::ClassObject,
    };
    let mut classes = analysis
        .original_class_declarations()
        .filter(|class| std::ptr::eq(class.metadata(), selected.class));
    let class = classes.next()?;
    if classes.next().is_some() {
        return None;
    }
    let mut methods = class
        .metadata()
        .original_members
        .declarations()
        .filter(|method| method.side() == side && std::ptr::eq(method.metadata(), selected.method));
    let method = methods.next()?;
    if methods.next().is_some() {
        return None;
    }
    Some(declaration_query(
        class,
        method,
        selected.original_selector_input().clone(),
        selected.selector,
        false,
        true,
        consumer_currency(analysis)?,
    ))
}

fn command_query(
    source: &str,
    analysis: &AnalysisResult,
    command: &SegmentedCommand,
) -> ControlFlow<Option<OriginalMethodQuery>> {
    if let Some(selected) = crate::receiver_identity::method_at_command(analysis, source, command) {
        return ControlFlow::Break(temporal_query(analysis, &selected));
    }
    // A known object has no declaration-only substitute for a missing entry.
    if crate::receiver_identity::class_at_command_head(analysis, source, command).is_some() {
        return ControlFlow::Break(None);
    }
    if let Some(call) =
        tcl_compiler::registry_invocation::source_structure::source_constructor_call_at(
            source,
            analysis,
            command.argv[0].span.start(),
        )
        && call
            .class_declaration()
            .name_input()
            .native_input()
            .is_some()
    {
        return ControlFlow::Break(source_class_call_query(analysis, call));
    }
    let Some(selector) = command.argv.get(1) else {
        return ControlFlow::Continue(());
    };
    // A variable receiver with a static selector needs its own object
    // receipt. Its reporting class name cannot fill missing dispatch proof.
    if let (Some(config), Some(realm)) = (
        analysis.body_lexer_config,
        analysis.retained_command_realm(),
    ) {
        let tokens = CommandTokens::from_segmented(&SourceMap::new(source), config, command);
        let binding = realm.invocation_at_source(command.name(), command.argv[0].span.start());
        if tokens
            .word_exprs
            .first()
            .is_some_and(|head| head.sole_variable_substitution().is_some())
            && binding
                .original_written_name_input(&tokens, 1)
                .is_some_and(|input| input.original_word_key().is_some())
        {
            return ControlFlow::Break(None);
        }
    }
    unresolved_class_lookup_query(source, analysis, command, selector.span)
}

fn unresolved_class_lookup_query(
    source: &str,
    analysis: &AnalysisResult,
    command: &SegmentedCommand,
    selector_span: Span,
) -> ControlFlow<Option<OriginalMethodQuery>> {
    let Some(invocation) = analysis
        .command_invocations
        .iter()
        .find(|invocation| invocation.range.start() == command.argv[0].span.start())
    else {
        return ControlFlow::Continue(());
    };
    let Some(head) = invocation.original_name_input.as_ref() else {
        return ControlFlow::Continue(());
    };
    let known_class = if let Some(reference) = &invocation.resolved_command_reference {
        let Some(definition) = reference.definition() else {
            return ControlFlow::Continue(());
        };
        if definition.kind() != tcl_compiler::command_binding::SourceCommandDefinitionKind::Class {
            return ControlFlow::Continue(());
        }
        true
    } else {
        false
    };
    if known_class {
        return ControlFlow::Break(None);
    }
    // This retained original head cannot borrow a reporting lookup or
    // selector producer when its independent owner is unavailable.
    let unavailable = || ControlFlow::Break(None);
    let Some(config) = analysis.body_lexer_config else {
        return unavailable();
    };
    let Some(lookup) = invocation
        .original_lookup
        .as_ref()
        .filter(|lookup| lookup.name_input() == head)
    else {
        return unavailable();
    };
    if lookup
        .matching_publications(
            analysis
                .original_class_declarations()
                .map(|record| (record.name(), ())),
        )
        .is_some_and(|selected| !selected.is_empty())
    {
        return unavailable();
    }
    let Some(realm) = analysis.retained_command_realm() else {
        return unavailable();
    };
    let tokens = CommandTokens::from_segmented(&SourceMap::new(source), config, command);
    let binding = realm.invocation_at_source(command.name(), command.argv[0].span.start());
    let Some(input) = binding.original_written_name_input(&tokens, 1) else {
        return unavailable();
    };
    let Some((consumer_image, consumer_config)) = consumer_currency(analysis) else {
        return unavailable();
    };
    ControlFlow::Break(Some(OriginalMethodQuery {
        class_input: Some(head.clone()),
        selector_input: input,
        side: MemberSide::ClassObject,
        span: selector_span,
        is_declaration: false,
        consumer_image,
        consumer_config,
        target: Target::ClassLookup {
            lookup: lookup.clone(),
            known_class,
        },
    }))
}

fn source_class_call_query(
    analysis: &AnalysisResult,
    call: OriginalSourceConstructorCall,
) -> Option<OriginalMethodQuery> {
    call.class_declaration().source_class(analysis)?;
    let selector = call.argument_input(0)?.native_input()?.clone();
    let span = selector.original_word_key()?.span();
    let (consumer_image, consumer_config) = consumer_currency(analysis)?;
    Some(OriginalMethodQuery {
        class_input: call
            .class_declaration()
            .name_input()
            .native_input()
            .cloned(),
        selector_input: selector,
        side: MemberSide::ClassObject,
        span,
        is_declaration: false,
        consumer_image,
        consumer_config,
        target: Target::SourceClassCall(Box::new(call)),
    })
}

/// Select a genuine current-source declaration or selector. A retained OO
/// operand with unavailable provenance is terminal. Non-OO arguments continue
/// through their own providers after workspace command occupation is checked.
#[must_use]
pub fn select(
    source: &str,
    analysis: &AnalysisResult,
    line: u32,
    character: u32,
) -> ControlFlow<Option<OriginalMethodQuery>> {
    // A lexical variable root remains a variable editor query even when its
    // produced value occupies an OO selector. Retained unknowns also stay with
    // the variable facade's terminal refusal rather than a class guess.
    let offset =
        crate::definition::byte_offset_at(&LineIndex::new(source), source, line, character);
    if analysis.body_lexer_config.is_some_and(|config| {
        analysis
            .original_variable_root_in_source(&SourceImage::document(source), config, offset)
            .is_some()
    }) {
        return ControlFlow::Continue(());
    }
    select_at_offset(source, analysis, offset)
}

fn select_at_offset(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> ControlFlow<Option<OriginalMethodQuery>> {
    if !current(source, analysis) {
        return if analysis.original_class_declarations().next().is_some()
            || analysis
                .command_invocations
                .iter()
                .any(|invocation| invocation.original_name_input.is_some())
        {
            ControlFlow::Break(None)
        } else {
            ControlFlow::Continue(())
        };
    }
    let mut own_declarations = analysis
        .original_object_configurations()
        .flat_map(|configuration| {
            configuration
                .members()
                .declarations()
                .map(move |method| (configuration.target(), method))
        })
        .filter(|(_, method)| {
            method
                .declaration()
                .static_occurrence()
                .is_some_and(|declaration| {
                    declaration
                        .name_input()
                        .span()
                        .as_range()
                        .contains(&(offset as usize))
                })
        });
    if let Some((configuration, method)) = own_declarations.next() {
        if own_declarations.next().is_some() {
            return ControlFlow::Break(None);
        }
        let Some(declaration) = method.declaration().static_occurrence() else {
            return ControlFlow::Break(None);
        };
        let Some((image, config)) = consumer_currency(analysis) else {
            return ControlFlow::Break(None);
        };
        return ControlFlow::Break(Some(own_query(
            configuration,
            method,
            (
                SignatureSourceNameInput::OriginalWord(declaration.name_input().clone()),
                declaration.name_input().span(),
            ),
            true,
            None,
            false,
            (image, config),
        )));
    }
    let mut declarations = analysis
        .original_class_declarations()
        .flat_map(|class| {
            class
                .metadata()
                .original_members
                .declarations()
                .map(move |method| (class, method))
        })
        .filter(|(_, method)| {
            method
                .declaration()
                .static_occurrence()
                .is_some_and(|declaration| {
                    declaration
                        .name_input()
                        .span()
                        .as_range()
                        .contains(&(offset as usize))
                })
        });
    if let Some((class, method)) = declarations.next() {
        if declarations.next().is_some() {
            return ControlFlow::Break(None);
        }
        let Some(declaration) = method.declaration().static_occurrence() else {
            return ControlFlow::Break(None);
        };
        let key = declaration.name_input();
        let Some(currency) = consumer_currency(analysis) else {
            return ControlFlow::Break(None);
        };
        return ControlFlow::Break(Some(declaration_query(
            class,
            method,
            SignatureSourceNameInput::OriginalWord(key.clone()),
            key.span(),
            true,
            false,
            currency,
        )));
    }
    if let Some(selected) = crate::receiver_identity::method_at_cursor(analysis, source, offset) {
        let query = temporal_query(analysis, &selected).map(|mut query| {
            // Cursor selection may be a captured prefix; source references
            // retain its declaration without attributing future execution.
            query.without_call_point();
            query
        });
        return ControlFlow::Break(query);
    }
    if crate::receiver_identity::definition_reference_at_cursor(analysis, source, offset).is_some()
    {
        return ControlFlow::Break(None);
    }
    let mut selected = ControlFlow::Continue(());
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            if command
                .argv
                .get(1)
                .is_some_and(|word| word.span.as_range().contains(&(offset as usize)))
            {
                selected = command_query(source, analysis, command);
                return selected.is_break();
            }
            false
        },
    );
    selected
}

/// Retain authentic selector occurrences for workspace readonly reference
/// queries. The owning original source is mandatory and independently checked.
#[must_use]
pub fn source_queries(analysis: &AnalysisResult) -> Vec<OriginalMethodQuery> {
    let Some(realm) = analysis.retained_command_realm() else {
        return Vec::new();
    };
    let Some(image) = realm.original_source_image() else {
        return Vec::new();
    };
    let Ok(source) = std::str::from_utf8(image.bytes()) else {
        return Vec::new();
    };
    if !current(source, analysis) {
        return Vec::new();
    }
    let mut queries = Vec::new();
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            if let ControlFlow::Break(Some(query)) = command_query(source, analysis, command) {
                queries.push(query);
            }
            for selected in
                crate::receiver_identity::captured_methods_at_command(analysis, source, command)
            {
                if let Some(mut query) = temporal_query(analysis, &selected) {
                    query.without_call_point();
                    queries.push(query);
                }
            }
            false
        },
    );
    queries.sort_by_key(|query| (query.span.start(), query.span.end()));
    queries.dedup();
    queries
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Header {
    Class(usize),
    Procedure,
}

fn unique_class_records<'a>(
    inventory: impl Iterator<Item = (&'a str, &'a SourceDeclarationMetadata<ClassDef>)>,
) -> Vec<(&'a str, &'a SourceDeclarationMetadata<ClassDef>)> {
    let mut seen = std::collections::HashMap::<
        (&str, &CommandAllocationSite),
        Vec<&SourceDeclarationMetadata<ClassDef>>,
    >::new();
    let mut records = Vec::new();
    for (uri, record) in inventory {
        let at_site = seen.entry((uri, record.declaration_site())).or_default();
        // Repeated identical views of one document are one source record.
        // Different URIs or disagreeing complete metadata remain independent.
        if !at_site.contains(&record) {
            at_site.push(record);
            records.push((uri, record));
        }
    }
    records
}

/// Select a document-owned source candidate through exact original command
/// occupation. Earlier proc headers, duplicate class owners, missing relation
/// coverage and unavailable own ledgers cannot donate a later member.
#[must_use]
pub fn candidate(
    index: &crate::workspace_index::WorkspaceIndex,
    caller_uri: &str,
    query: &OriginalMethodQuery,
) -> ControlFlow<Option<OriginalMethodCandidate>> {
    if matches!(query.target, Target::OwnObject(_)) {
        if !index
            .diagnostic_source_context(caller_uri)
            .is_some_and(|context| {
                context.image() == query.consumer_image()
                    && context.config() == query.consumer_config()
            })
        {
            return ControlFlow::Break(None);
        }
        return own_candidate(caller_uri, query);
    }
    if matches!(query.target, Target::SourceClassCall(_))
        && !index
            .diagnostic_source_context(caller_uri)
            .is_some_and(|context| {
                context.image() == query.consumer_image()
                    && context.config() == query.consumer_config()
            })
    {
        return ControlFlow::Break(None);
    }
    let records = unique_class_records(index.original_class_declarations());
    let procedures = index
        .original_procedure_declarations()
        .map(|(_, record)| record)
        .collect::<Vec<_>>();
    if matches!(query.target, Target::ClassLookup { .. }) {
        return workspace_class_lookup_candidate(index, query, &records, &procedures);
    }
    candidate_for_records(caller_uri, query, &records, &procedures)
}

fn workspace_class_lookup_candidate(
    index: &crate::workspace_index::WorkspaceIndex,
    query: &OriginalMethodQuery,
    records: &[(&str, &SourceDeclarationMetadata<ClassDef>)],
    procedures: &[&SourceDeclarationMetadata<ProcDef>],
) -> ControlFlow<Option<OriginalMethodCandidate>> {
    let Target::ClassLookup {
        lookup,
        known_class,
    } = &query.target
    else {
        return ControlFlow::Break(None);
    };
    let headers = index
        .original_class_source_candidates()
        .filter_map(|(uri, class, slot, policy)| {
            records
                .iter()
                .position(|(owner, record)| *owner == uri && *record == class)
                .map(|root| (slot, policy, Header::Class(root)))
        })
        .chain(
            index
                .original_procedure_source_candidates()
                .map(|(_, _, slot, policy)| (slot, policy, Header::Procedure)),
        );
    match lookup.matching_slot_publications(headers).as_deref() {
        Some([Header::Class(root)]) => ControlFlow::Break(metadata_candidate(
            procedures,
            records,
            *root,
            query.selector_input(),
            query.side(),
        )),
        Some([] | [Header::Procedure]) if !*known_class => ControlFlow::Continue(()),
        _ => ControlFlow::Break(None),
    }
}

fn candidate_for_records(
    caller_uri: &str,
    query: &OriginalMethodQuery,
    records: &[(&str, &SourceDeclarationMetadata<ClassDef>)],
    procedures: &[&SourceDeclarationMetadata<ProcDef>],
) -> ControlFlow<Option<OriginalMethodCandidate>> {
    match &query.target {
        Target::OwnObject(_) => ControlFlow::Break(None),
        Target::Declaration {
            class,
            method,
            call_point,
        } => {
            let mut selected = records
                .iter()
                .filter(|(uri, record)| *uri == caller_uri && record.declaration_site() == class);
            let Some((uri, record)) = selected.next() else {
                return ControlFlow::Break(None);
            };
            if selected.next().is_some() {
                return ControlFlow::Break(None);
            }
            let mut methods =
                record
                    .metadata()
                    .original_members
                    .declarations()
                    .filter(|declaration| {
                        declaration.side() == query.side && declaration.declaration() == method
                    });
            let Some(method) = methods.next() else {
                return ControlFlow::Break(None);
            };
            if methods.next().is_some() {
                return ControlFlow::Break(None);
            }
            ControlFlow::Break(Some(owned_candidate(
                uri,
                record,
                method.clone(),
                *call_point,
            )))
        }
        Target::SourceClassCall(call) => {
            let mut selected = records.iter().enumerate().filter(|(_, (uri, record))| {
                *uri == caller_uri && call.class_declaration().matches_source_declaration(record)
            });
            let Some((root, _)) = selected.next() else {
                return ControlFlow::Break(None);
            };
            if selected.next().is_some() {
                return ControlFlow::Break(None);
            }
            ControlFlow::Break(metadata_candidate(
                procedures,
                records,
                root,
                query.selector_input(),
                query.side(),
            ))
        }
        // A local call with no authentic source-class owner cannot select a
        // canonical header. Sibling source suggestions use the separate
        // workspace current-publication inventory above.
        Target::ClassLookup { .. } => ControlFlow::Break(None),
    }
}

fn owned_candidate(
    uri: &str,
    class: &SourceDeclarationMetadata<ClassDef>,
    method: OriginalSourceMethodMetadata,
    call_point: bool,
) -> OriginalMethodCandidate {
    OriginalMethodCandidate {
        uri: uri.to_owned(),
        class_input: Some(SignatureSourceNameInput::OriginalWord(
            class.name_input().clone(),
        )),
        class: Some(class.declaration_site().clone()),
        object: None,
        method,
        call_point,
    }
}

fn metadata_candidate(
    procedures: &[&SourceDeclarationMetadata<ProcDef>],
    records: &[(&str, &SourceDeclarationMetadata<ClassDef>)],
    root: usize,
    selector: &SignatureSourceNameInput,
    side: MemberSide,
) -> Option<OriginalMethodCandidate> {
    // Class-object own metadata supplies no maker or object-mixin order.
    let order = match side {
        MemberSide::ClassObject => vec![root],
        MemberSide::Instance => tcl_compiler::analyser::class_hierarchy::original_metadata::
            original_instance_metadata_order_for_inventory(
                &records.iter().map(|(_, record)| *record).collect::<Vec<_>>(),
                root,
                &procedures.iter().map(|record| record.name()).collect::<Vec<_>>(),
            )?,
    };
    for position in order {
        let (uri, record) = *records.get(position)?;
        let methods = record.metadata().original_members.methods(side)?;
        if let Some(method) = methods.into_iter().find(|method| {
            method.original_name_input().policy() == selector.policy()
                && method.original_name_input().bytes() == selector.bytes()
                && method
                    .name_purpose()
                    .admits(selector.policy().recipe(), selector.bytes())
        }) {
            return Some(owned_candidate(uri, record, method, false));
        }
    }
    None
}

/// Explicit source-declaration advice for an independently selected class and
/// member side. Instance advice uses the common exact-record MRO; class-object
/// advice uses only this source object's own table. No dispatch follows.
#[must_use]
pub fn member_metadata_candidate(
    index: &crate::workspace_index::WorkspaceIndex,
    owner_uri: &str,
    class: &SourceDeclarationMetadata<ClassDef>,
    selector: &SignatureSourceNameInput,
    side: MemberSide,
) -> Option<OriginalMethodCandidate> {
    let records = unique_class_records(index.original_class_declarations());
    let mut roots = records
        .iter()
        .enumerate()
        .filter(|(_, (uri, record))| *uri == owner_uri && **record == *class);
    let root = roots.next()?.0;
    if roots.next().is_some() {
        return None;
    }
    let procedures = index
        .original_procedure_declarations()
        .map(|(_, record)| record)
        .collect::<Vec<_>>();
    metadata_candidate(&procedures, &records, root, selector, side)
}

/// Select a local readonly candidate directly from genuine analysis
/// inventories. This shares the Workspace kernel without constructing an index
/// or reconstructing a class/selector from a reporting string.
#[must_use]
pub fn local_candidate(
    source: &str,
    analysis: &AnalysisResult,
    line: u32,
    character: u32,
) -> ControlFlow<Option<OriginalMethodCandidate>> {
    match select(source, analysis, line, character) {
        ControlFlow::Continue(()) => ControlFlow::Continue(()),
        ControlFlow::Break(None) => ControlFlow::Break(None),
        ControlFlow::Break(Some(query)) => candidate_for_analysis(analysis, &query),
    }
}

pub(crate) fn candidate_for_analysis(
    analysis: &AnalysisResult,
    query: &OriginalMethodQuery,
) -> ControlFlow<Option<OriginalMethodCandidate>> {
    if let Target::OwnObject(own) = &query.target {
        if !analysis.body_lexer_config.is_some_and(|config| {
            config == query.consumer_config()
                && analysis.matches_original_source_image(query.consumer_image(), config)
        }) || !analysis
            .original_object_configurations()
            .any(|configuration| {
                configuration.target() == &own.configuration
                    && configuration
                        .members()
                        .declarations()
                        .any(|method| method == &own.method)
            })
        {
            return ControlFlow::Break(None);
        }
        return own_candidate("", query);
    }
    if let Target::SourceClassCall(call) = &query.target
        && call.class_declaration().source_class(analysis).is_none()
    {
        return ControlFlow::Break(None);
    }
    let records = unique_class_records(
        analysis
            .original_class_declarations()
            .map(|record| ("", record)),
    );
    let procedures = analysis
        .original_procedure_declarations()
        .collect::<Vec<_>>();
    candidate_for_records("", query, &records, &procedures)
}

/// Local source selector references to the same canonical declaration. Each
/// consumer retains its own complete current image/configuration; captured
/// prefix identity remains separate from an entered future callback.
#[must_use]
pub fn local_reference_spans(
    source: &str,
    analysis: &AnalysisResult,
    target: &OriginalMethodCandidate,
    include_declaration: bool,
) -> Vec<Span> {
    if !target.uri().is_empty() || !current(source, analysis) {
        return Vec::new();
    }
    let mut spans = Vec::new();
    if include_declaration {
        spans.push(target.declaration_span());
    }
    for query in source_queries(analysis) {
        if let ControlFlow::Break(Some(selected)) = candidate_for_analysis(analysis, &query)
            && selected.same_declaration(target)
        {
            spans.push(query.span());
        }
    }
    spans.sort_by_key(|span| (span.start(), span.end()));
    spans.dedup();
    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_index::WorkspaceIndex;
    use tcl_compiler::analyser::Analyser;

    fn analysis(source: &str) -> AnalysisResult {
        let mut result = Analyser::new().analyse(source, "tcl8.6").clone();
        result.all_classes.clear();
        result.superseded_classes.clear();
        result
    }

    #[test]
    fn logical_class_calls_keep_their_separate_source_consumer_domain() {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        let source = "oo::class create C {self method ping {} {}}; C ping";
        let result = Analyser::new().analyse(source, "tcl");
        assert!(result.allows_retained_logical_declaration_advice());
        assert!(result.original_class_declarations().next().is_none());
        let commands = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            result.body_lexer_config.unwrap(),
        );
        let command = commands.last().unwrap();
        let call = tcl_compiler::registry_invocation::source_structure::source_constructor_call_at(
            source,
            &result,
            command.argv[0].span.start(),
        )
        .unwrap();
        assert!(
            call.class_declaration()
                .name_input()
                .native_input()
                .is_none()
        );
        assert!(
            call.class_declaration()
                .logical_source_class(&result)
                .is_some()
        );
        assert!(matches!(
            command_query(source, &result, command),
            ControlFlow::Continue(())
        ));
    }

    #[test]
    fn original_class_call_method_candidates_keep_call_horizons_and_captured_selectors() {
        // naming.source.original-class-publications
        // docs/design/analysis/name-resolution-proofs/source-original-class-publications.md
        let source = "oo::class create C {self method ping {} {}}; C ping; rename C M; C ping; M ping; interp alias {} A {} M ping; A; rename M {}; M ping";
        let result = analysis(source);
        assert!(result.original_completed_command_world().is_none());
        let commands = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            result.body_lexer_config.unwrap(),
        );
        let mut selected = Vec::new();
        for command in &commands {
            if !matches!(command.name(), "C" | "M" | "A") {
                continue;
            }
            let query = command_query(source, &result, command);
            let candidate = match query {
                ControlFlow::Break(Some(query)) => {
                    if command.name() == "A" {
                        let Target::SourceClassCall(call) = &query.target else {
                            panic!("original captured call");
                        };
                        assert_eq!(call.written_argument(0), None);
                        assert_eq!(query.selector_input().bytes(), b"ping");
                        assert!(query.span().start() < command.argv[0].span.start());
                        assert!(!call.obligations().is_empty());
                    }
                    candidate_for_analysis(&result, &query)
                }
                _ => ControlFlow::Break(None),
            };
            let found = matches!(candidate, ControlFlow::Break(Some(_)));
            if let ControlFlow::Break(Some(candidate)) = candidate {
                assert_eq!(candidate.method().original_name_input().bytes(), b"ping");
                assert!(!candidate.has_call_point_entry());
            }
            selected.push(found);
        }
        assert_eq!(selected, [true, false, true, true, false]);
    }

    #[test]
    fn original_workspace_class_method_candidates_use_current_slots_and_canonical_targets() {
        // naming.source.original-class-publications
        // docs/design/analysis/name-resolution-proofs/source-original-class-publications.md
        let library_source = "oo::class create C {self method ping {} {}}; rename C M";
        let library = analysis(library_source);
        let caller_source = "M ping; C ping";
        let caller = analysis(caller_source);
        assert!(library.original_completed_command_world().is_none());
        let library_uri = "file:///classes.tcl";
        let caller_uri = "file:///caller.tcl";
        let mut index =
            WorkspaceIndex::from_documents([(library_uri, &library), (caller_uri, &caller)]);
        let queries = source_queries(&caller);
        assert_eq!(queries.len(), 2);
        let ControlFlow::Break(Some(selected)) = candidate(&index, caller_uri, &queries[0]) else {
            panic!("current moved source suggestion");
        };
        assert_eq!(selected.uri(), library_uri);
        assert_eq!(selected.class_input.as_ref().unwrap().bytes(), b"C");
        assert_eq!(
            selected.class.as_ref().unwrap(),
            library
                .original_class_declarations()
                .next()
                .unwrap()
                .declaration_site()
        );
        assert!(!selected.has_call_point_entry());
        assert!(matches!(
            candidate(&index, caller_uri, &queries[1]),
            ControlFlow::Continue(())
        ));
        for source in [
            "oo::class create C {self method ping {} {}}; rename C {}",
            "oo::class create C {self method ping {} {}}; proc C {} {}",
        ] {
            index.replace_document(library_uri, &analysis(source));
            for query in &queries {
                assert!(matches!(
                    candidate(&index, caller_uri, query),
                    ControlFlow::Continue(())
                ));
            }
        }
        let mut missing = library.clone();
        missing.resolved_input = None;
        index.replace_document(library_uri, &missing);
        assert!(matches!(
            candidate(&index, caller_uri, &queries[0]),
            ControlFlow::Continue(())
        ));
    }

    #[test]
    fn original_local_method_providers_share_opaque_declaration_and_whole_source_currency() {
        // Implementation contract: naming.editor.original-method-candidates (docs/design/analysis/name-resolution-proofs/original-method-candidates.md).
        let source = "namespace eval N {oo::class create C {method m\\uD800 {a} {}; method m\\uD801 {a b} {}}}";
        let result = analysis(source);
        let cursor = u32::try_from(source.find("m\\uD800").unwrap()).unwrap();
        let ControlFlow::Break(Some(selected)) = local_candidate(source, &result, 0, cursor) else {
            panic!("genuine original nested method declaration");
        };
        assert_eq!(
            selected.method().original_name_input().bytes(),
            b"m\xed\xa0\x80"
        );
        assert_eq!(selected.declaration_image(), &SourceImage::document(source));
        assert_eq!(
            crate::definition::definition(source, 0, cursor, &result).len(),
            1
        );
        assert_eq!(
            crate::references::references(
                source,
                crate::profile_for_dialect("tcl8.6"),
                0,
                cursor,
                &result,
                true
            )
            .len(),
            1
        );
        assert!(
            crate::hover::hover(source, 0, cursor, &result, None)
                .unwrap()
                .value
                .contains("Possible source declaration")
        );
        let stale = source.replacen("N", "M", 1);
        assert!(matches!(
            local_candidate(&stale, &result, 0, cursor),
            ControlFlow::Break(None)
        ));
        assert!(crate::definition::definition(&stale, 0, cursor, &result).is_empty());
        assert!(
            crate::references::references(
                &stale,
                crate::profile_for_dialect("tcl8.6"),
                0,
                cursor,
                &result,
                true
            )
            .is_empty()
        );
        assert!(crate::hover::hover(&stale, 0, cursor, &result, None).is_none());
    }

    #[test]
    fn original_local_method_providers_refuse_reporting_receiver_class_guesses() {
        // Implementation contract: naming.editor.original-method-candidates (docs/design/analysis/name-resolution-proofs/original-method-candidates.md).
        let source = "oo::class create C {method m {} {}}\nset obj unknown\n$obj m\n";
        let mut result = Analyser::new().analyse(source, "tcl8.6").clone();
        result
            .instance_classes
            .insert("obj".to_owned(), "::C".to_owned());
        assert!(matches!(
            local_candidate(source, &result, 2, 5),
            ControlFlow::Break(None)
        ));
        assert!(crate::definition::definition(source, 2, 5, &result).is_empty());
        assert!(
            crate::references::references(
                source,
                crate::profile_for_dialect("tcl8.6"),
                2,
                5,
                &result,
                true
            )
            .is_empty()
        );
        assert!(crate::hover::hover(source, 2, 5, &result, None).is_none());
    }

    #[test]
    fn original_method_selection_leaves_lexical_variable_reads_with_the_variable_owner() {
        // Implementation contract: naming.editor.original-method-candidates (docs/design/analysis/name-resolution-proofs/original-method-candidates.md).
        for (source, line, character) in [
            (
                "namespace eval N {}\nset ::N::v VALUE\nunknown $::N::v\n",
                2,
                13,
            ),
            ("set method m\nC $method\n", 1, 4),
        ] {
            let mut result = analysis(source);
            let ControlFlow::Break(Some(variable)) =
                crate::variable_symbol::select(source, &result, line, character)
            else {
                panic!("genuine lexical variable read");
            };
            let retained_root = variable.span();
            assert!(matches!(
                select(source, &result, line, character),
                ControlFlow::Continue(())
            ));
            // Keep the authentic retained read while its selected symbol is
            // unavailable. Its own provider must refuse instead of borrowing OO.
            result.original_variable_symbols.clear();
            result
                .original_variable_symbol_conflicts
                .push(retained_root);
            assert!(matches!(
                crate::variable_symbol::select(source, &result, line, character),
                ControlFlow::Break(None)
            ));
            assert!(matches!(
                select(source, &result, line, character),
                ControlFlow::Continue(())
            ));
        }
    }

    #[test]
    fn original_method_lookup_owner_absence_is_terminal_before_reporting_assistance() {
        // Implementation contract: naming.editor.original-method-candidates (docs/design/analysis/name-resolution-proofs/original-method-candidates.md).
        let source = "C m\n";
        let mut result = analysis(source);
        assert!(matches!(
            select(source, &result, 0, 2),
            ControlFlow::Break(Some(_))
        ));
        let invocation = result
            .command_invocations
            .iter_mut()
            .find(|invocation| invocation.range.start() == 0)
            .unwrap();
        assert!(invocation.original_name_input.is_some());
        assert!(invocation.original_lookup.take().is_some());
        assert!(matches!(
            select(source, &result, 0, 2),
            ControlFlow::Break(None)
        ));
        assert!(matches!(
            local_candidate(source, &result, 0, 2),
            ControlFlow::Break(None)
        ));
        assert!(crate::definition::definition(source, 0, 2, &result).is_empty());
        assert!(
            crate::references::references(
                source,
                crate::profile_for_dialect("tcl8.6"),
                0,
                2,
                &result,
                true
            )
            .is_empty()
        );
        assert!(crate::hover::hover(source, 0, 2, &result, None).is_none());
        let foreign = analysis("D m\n")
            .command_invocations
            .into_iter()
            .find_map(|invocation| invocation.original_lookup)
            .unwrap();
        result.command_invocations[0].original_lookup = Some(foreign);
        assert!(matches!(
            select(source, &result, 0, 2),
            ControlFlow::Break(None)
        ));
    }

    #[test]
    fn original_workspace_member_candidates_keep_opaque_inputs_and_document_owners() {
        // Implementation contract: naming.editor.original-method-candidates (docs/design/analysis/name-resolution-proofs/original-method-candidates.md).
        let provider = analysis(
            "oo::class create C\\uD800 {self method m\\uD800 {} {}; self method m\\uD801 {} {}}\n",
        );
        let consumer_source = "C\\uD800 m\\uD800\n";
        let consumer = analysis(consumer_source);
        let ControlFlow::Break(Some(query)) = select(consumer_source, &consumer, 0, 11) else {
            panic!("retained original selector");
        };
        assert_eq!(query.class_input().unwrap().bytes(), b"C\xed\xa0\x80");
        assert_eq!(query.selector_input().bytes(), b"m\xed\xa0\x80");
        let mut index = WorkspaceIndex::from_documents([
            ("file:///provider.tcl", &provider),
            ("file:///consumer.tcl", &consumer),
        ]);
        let ControlFlow::Break(Some(selected)) = candidate(&index, "file:///consumer.tcl", &query)
        else {
            panic!("exact own-table source candidate");
        };
        assert_eq!(selected.uri(), "file:///provider.tcl");
        assert_eq!(
            selected.method().original_name_input().bytes(),
            b"m\xed\xa0\x80"
        );
        assert!(!selected.has_call_point_entry());
        assert!(
            selected
                .hover()
                .unwrap()
                .value
                .contains("Possible source declaration")
        );
        index.add_document("file:///provider.tcl", &provider);
        assert!(matches!(
            candidate(&index, "file:///consumer.tcl", &query),
            ControlFlow::Break(Some(_))
        ));
        index.add_document("file:///duplicate.tcl", &provider);
        assert!(matches!(
            candidate(&index, "file:///consumer.tcl", &query),
            ControlFlow::Break(None)
        ));
        assert!(matches!(
            select(&format!("#{consumer_source}"), &consumer, 0, 11),
            ControlFlow::Continue(()) | ControlFlow::Break(None)
        ));
    }

    #[test]
    fn original_instance_member_advice_uses_complete_relations_and_never_class_side_mro() {
        // Implementation contract: naming.editor.original-method-candidates (docs/design/analysis/name-resolution-proofs/original-method-candidates.md).
        let base =
            analysis("oo::class create B\\uD800 {method m\\uD800 {} {}; self method cm {} {}}\n");
        let child = analysis("oo::class create C {superclass B\\uD800}\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///base.tcl", &base),
            ("file:///child.tcl", &child),
        ]);
        let root = child.original_class_declarations().next().unwrap();
        let parent = base.original_class_declarations().next().unwrap();
        let input = parent
            .metadata()
            .original_members
            .declarations()
            .find(|method| method.side() == MemberSide::Instance)
            .unwrap()
            .original_name_input();
        let selected = member_metadata_candidate(
            &index,
            "file:///child.tcl",
            root,
            input,
            MemberSide::Instance,
        )
        .unwrap();
        assert_eq!(selected.uri(), "file:///base.tcl");
        assert!(
            member_metadata_candidate(
                &index,
                "file:///child.tcl",
                root,
                input,
                MemberSide::ClassObject
            )
            .is_none()
        );
        let missing = WorkspaceIndex::from_documents([("file:///child.tcl", &child)]);
        assert!(
            member_metadata_candidate(
                &missing,
                "file:///child.tcl",
                root,
                input,
                MemberSide::Instance
            )
            .is_none()
        );
        let mut duplicate = index.clone();
        duplicate.add_document("file:///other-base.tcl", &base);
        assert!(
            member_metadata_candidate(
                &duplicate,
                "file:///child.tcl",
                root,
                input,
                MemberSide::Instance
            )
            .is_none()
        );
    }

    #[test]
    fn original_member_advice_stops_at_earlier_occupied_non_class_headers() {
        // Implementation contract: naming.editor.original-method-candidates (docs/design/analysis/name-resolution-proofs/original-method-candidates.md).
        let provider = analysis("oo::class create C {self method m {} {}}\n");
        let blocker = analysis("namespace eval N {proc C {} {}}\n");
        let source = "namespace eval N {C m}\n";
        let consumer = analysis(source);
        let index = WorkspaceIndex::from_documents([
            ("file:///provider.tcl", &provider),
            ("file:///blocker.tcl", &blocker),
            ("file:///consumer.tcl", &consumer),
        ]);
        let cursor = u32::try_from(source.find("C m").unwrap() + 2).unwrap();
        let ControlFlow::Break(Some(query)) = select_at_offset(source, &consumer, cursor) else {
            panic!("authentic pending class operand");
        };
        assert!(matches!(
            candidate(&index, "file:///consumer.tcl", &query),
            ControlFlow::Continue(())
        ));
    }
}

#[cfg(test)]
mod own_object_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn at(
        source: &str,
        analysis: &AnalysisResult,
        needle: &str,
    ) -> ControlFlow<Option<OriginalMethodCandidate>> {
        let offset = u32::try_from(source.rfind(needle).unwrap()).unwrap();
        let position = LineIndex::new(source).position_at_utf16(offset, source);
        local_candidate(source, analysis, position.line, position.character.get())
    }

    #[test]
    fn original_own_object_candidates_keep_allocation_and_canonical_worker_without_class_tables() {
        // Implementation contract: naming.editor.original-own-object-member-navigation
        // docs/design/analysis/name-resolution-proofs/original-own-object-member-navigation.md
        let source = r"oo::class create C {method shared {} {return CLASS}}; C create object; C create sibling; oo::objdefine object {method shared {} {}; method p\uD800 {} {}; method p\uD801 {} {}}; object shared; object p\uD800; object p\uD801; sibling shared";
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut analysis = Analyser::new().analyse(source, dialect);
            analysis.all_classes.clear();
            analysis.global_scope.classes.clear();
            analysis.instance_classes.clear();
            analysis.created_instance_commands.clear();
            let ControlFlow::Break(Some(own)) = at(source, &analysis, "shared; object") else {
                panic!("own method has its own source owner");
            };
            assert!(own.own_object().is_some());
            assert!(own.has_call_point_entry());
            assert!(own.declaring_class_in(source, &analysis).is_none());
            let ControlFlow::Break(Some(sibling)) = at(source, &analysis, "shared") else {
                panic!("sibling keeps the class-owned declaration");
            };
            assert!(sibling.own_object().is_none());
            assert!(!own.same_declaration(&sibling));
            let ControlFlow::Break(Some(a)) = at(source, &analysis, r"p\uD800;") else {
                panic!("first opaque own key");
            };
            let ControlFlow::Break(Some(b)) = at(source, &analysis, r"p\uD801;") else {
                panic!("second opaque own key");
            };
            assert!(!a.same_declaration(&b));
            assert!(a.hover().unwrap().value.contains("Own method"));
            let changed = format!("{source} ");
            assert!(matches!(
                at(&changed, &analysis, r"p\uD800;"),
                ControlFlow::Break(None)
            ));
            let mut changed_config = analysis.clone();
            changed_config
                .body_lexer_config
                .as_mut()
                .unwrap()
                .strict_quoting ^= true;
            assert!(matches!(
                at(source, &changed_config, r"p\uD800;"),
                ControlFlow::Break(None)
            ));
        }
    }

    #[test]
    fn original_own_object_generation_and_private_entries_block_class_advice() {
        // Implementation contract: naming.editor.original-own-object-member-navigation
        // docs/design/analysis/name-resolution-proofs/original-own-object-member-navigation.md
        let source = "oo::class create C {method shared {} {return CLASS}; method Hidden {} {}; export Hidden}; C create object; oo::objdefine object {method shared {} {return FIRST}; method Hidden {} {}}; object shared; oo::objdefine object method shared {} {return SECOND}; object shared; object Hidden";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let first = u32::try_from(source.find("object shared;").unwrap()).unwrap() + 7;
        let position = LineIndex::new(source).position_at_utf16(first, source);
        let ControlFlow::Break(Some(first)) =
            local_candidate(source, &analysis, position.line, position.character.get())
        else {
            panic!("first own generation");
        };
        let ControlFlow::Break(Some(last)) = at(source, &analysis, "shared; object Hidden") else {
            panic!("replacement own worker");
        };
        assert!(!first.same_declaration(&last));
        assert_eq!(
            first.own_object().unwrap().instance().allocation(),
            last.own_object().unwrap().instance().allocation()
        );
        assert!(matches!(
            at(source, &analysis, "Hidden"),
            ControlFlow::Break(None)
        ));
        let complete_prefix = source.strip_suffix("object Hidden").unwrap();
        let ambiguous = format!("{complete_prefix} unknown; object shared");
        let analysis = Analyser::new().analyse(&ambiguous, "tcl8.6");
        assert!(matches!(
            at(&ambiguous, &analysis, "shared"),
            ControlFlow::Break(None)
        ));
    }
}
