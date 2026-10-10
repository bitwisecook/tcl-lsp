// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Current readonly declaration identities and bounded editor wire locators.
//!
//! A wire locator reselects a retained identity; it cannot manufacture an
//! original operand, command installation, dispatch entry or edit capability.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use tcl_compiler::analyser::types::{
    MemberSide, OriginalSourceMethodMetadata, OriginalSourcePropertyMetadata,
    OriginalSourceSpecialMemberMetadata,
};
use tcl_compiler::analyser::{AnalysisResult, ClassDef, ProcDef};
use tcl_compiler::command_binding::CommandAllocationSite;
use tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata;
use tcl_compiler::signature_scan::scope::{SignatureSourceCommand, SignatureSourceNameInput};
use tcl_lexer::{LexerConfig, SourceImage, Span};

/// One independently owned current document supplied to readonly source
/// declaration, reference and hierarchy consumers. The carrier itself grants
/// neither complete inventory coverage nor native execution authority.
#[derive(Clone, Copy)]
pub struct OriginalDeclarationDocument<'a> {
    /// Owning URI, independent of an equal complete source copy.
    pub uri: &'a str,
    /// Complete current source text for this owner.
    pub source: &'a str,
    /// Its actual original declaration and consuming lookup inventories.
    pub analysis: &'a AnalysisResult,
}

/// Checked readonly class publications and their independent current owners.
/// Non-class command publications remain occupied relation candidates.
pub struct OriginalClassInventory<'a> {
    records: Vec<&'a SourceDeclarationMetadata<ClassDef>>,
    owners: Vec<OriginalDeclarationDocument<'a>>,
    occupied: Vec<&'a SignatureSourceCommand>,
}

impl<'a> OriginalClassInventory<'a> {
    /// Retain only a complete current inventory. Duplicate document ownership
    /// or stale original headers withdraws the whole relation view.
    #[must_use]
    pub fn from_documents(documents: &[OriginalDeclarationDocument<'a>]) -> Option<Self> {
        let mut inventory = Self {
            records: Vec::new(),
            owners: Vec::new(),
            occupied: Vec::new(),
        };
        let mut uris = std::collections::HashSet::new();
        for &document in documents {
            let config = document.analysis.body_lexer_config?;
            if !uris.insert(document.uri)
                || !document
                    .analysis
                    .matches_original_source_image(&SourceImage::document(document.source), config)
            {
                return None;
            }
            for record in document.analysis.original_class_declarations() {
                OriginalDeclarationIdentity::for_class(
                    document.uri,
                    document.source,
                    document.analysis,
                    record,
                )?;
                inventory.records.push(record);
                inventory.owners.push(document);
            }
            for record in document.analysis.original_procedure_declarations() {
                OriginalDeclarationIdentity::for_procedure(
                    document.uri,
                    document.source,
                    document.analysis,
                    record,
                )?;
                inventory.occupied.push(record.name());
            }
        }
        Some(inventory)
    }

    /// Original class records; indices preserve equal-byte independent owners.
    #[must_use]
    pub fn records(&self) -> &[&'a SourceDeclarationMetadata<ClassDef>] {
        &self.records
    }

    /// Complete current document owning this class record.
    #[must_use]
    pub fn owner(&self, index: usize) -> Option<OriginalDeclarationDocument<'a>> {
        self.owners.get(index).copied()
    }

    /// Other exact command publications which can block a class relation.
    #[must_use]
    pub fn occupied_non_classes(&self) -> &[&'a SignatureSourceCommand] {
        &self.occupied
    }
}

/// The readonly declaration purpose, without a runtime member-table identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalDeclarationRole {
    /// The complete document command stream.
    Document,
    /// One independently retained procedure declaration.
    Procedure,
    /// One independently retained class declaration.
    Class,
    /// One member declaration in its independently selected receiver table.
    Method(MemberSide),
    /// One property declaration in its independently selected receiver table.
    Property(MemberSide),
    /// Nameless lifecycle syntax, without an invented method-name input.
    Special(
        MemberSide,
        tcl_registry::definer::DefinitionSpecialMemberKind,
    ),
}

/// Document-owned readonly declaration metadata. Complete source and scanner
/// equality remain authoritative even if a cached source digest collides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalDeclarationIdentity {
    uri: String,
    image: SourceImage,
    config: LexerConfig,
    role: OriginalDeclarationRole,
    site: Option<CommandAllocationSite>,
    owner: Option<CommandAllocationSite>,
    object_owner: Option<tcl_compiler::command_binding::OriginalSourceObjectConfigurationTarget>,
    input: Option<SignatureSourceNameInput>,
    span: Span,
}

impl OriginalDeclarationIdentity {
    fn current(source: &str, analysis: &AnalysisResult) -> Option<(SourceImage, LexerConfig)> {
        let image = SourceImage::document(source);
        let config = analysis.body_lexer_config?;
        analysis
            .matches_original_source_image(&image, config)
            .then_some((image, config))
    }
    /// Issue a readonly identity for the actual current document stream.
    #[must_use]
    pub fn for_document(uri: &str, source: &str, analysis: &AnalysisResult) -> Option<Self> {
        let (image, config) = Self::current(source, analysis)?;
        Some(Self {
            uri: uri.to_owned(),
            image,
            config,
            role: OriginalDeclarationRole::Document,
            site: None,
            owner: None,
            object_owner: None,
            input: None,
            span: Span::new(0, 0),
        })
    }
    /// Issue a class identity only for a member of this current original inventory.
    #[must_use]
    pub fn for_class(
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        declaration: &SourceDeclarationMetadata<ClassDef>,
    ) -> Option<Self> {
        let (image, config) = Self::current(source, analysis)?;
        if !analysis
            .original_class_declarations()
            .any(|row| row == declaration)
            || declaration.name_input().source_image() != &image
            || declaration.name_input().lexer_config() != config
        {
            return None;
        }
        Some(Self {
            uri: uri.to_owned(),
            image,
            config,
            role: OriginalDeclarationRole::Class,
            site: Some(declaration.declaration_site().clone()),
            owner: None,
            object_owner: None,
            input: Some(SignatureSourceNameInput::OriginalWord(
                declaration.name_input().clone(),
            )),
            span: declaration.name_input().span(),
        })
    }
    /// Issue one procedure identity without collapsing repeated declarations.
    #[must_use]
    pub fn for_procedure(
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        declaration: &SourceDeclarationMetadata<ProcDef>,
    ) -> Option<Self> {
        let (image, config) = Self::current(source, analysis)?;
        if !analysis
            .original_procedure_declarations()
            .any(|row| row == declaration)
            || declaration.name_input().source_image() != &image
            || declaration.name_input().lexer_config() != config
        {
            return None;
        }
        Some(Self {
            uri: uri.to_owned(),
            image,
            config,
            role: OriginalDeclarationRole::Procedure,
            site: Some(declaration.declaration_site().clone()),
            owner: None,
            object_owner: None,
            input: Some(SignatureSourceNameInput::OriginalWord(
                declaration.name_input().clone(),
            )),
            span: declaration.name_input().span(),
        })
    }
    /// Issue a method identity from its actual class inventory and declaration.
    /// The source word supplies readonly geometry, independently of edits.
    #[must_use]
    pub fn for_method(
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        class: &SourceDeclarationMetadata<ClassDef>,
        method: &OriginalSourceMethodMetadata,
    ) -> Option<Self> {
        let class_identity = Self::for_class(uri, source, analysis, class)?;
        if !class.metadata().original_members.declarations().any(|row| {
            row.side() == method.side()
                && row.declaration() == method.declaration()
                && row.metadata() == method.metadata()
        }) {
            return None;
        }
        let declaration = method.declaration();
        let word = declaration.original_word();
        if word.image() != &class_identity.image || word.config() != class_identity.config {
            return None;
        }
        Some(Self {
            role: OriginalDeclarationRole::Method(method.side()),
            site: Some(declaration.site().clone()),
            owner: class_identity.site.clone(),
            input: Some(declaration.original_name_input().clone()),
            span: word.span(),
            ..class_identity
        })
    }
    /// Retain an own-object worker under its actual configuration and bounded
    /// allocation. The object's class supplies neither this table nor a name.
    #[must_use]
    pub fn for_object_method(
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        owner: &tcl_compiler::command_binding::OriginalSourceObjectConfigurationTarget,
        method: &OriginalSourceMethodMetadata,
    ) -> Option<Self> {
        let (image, config) = Self::current(source, analysis)?;
        let word = method.declaration().original_word();
        if word.image() != &image
            || word.config() != config
            || owner.site().source.source_image() != &image
            || !analysis
                .original_object_configurations()
                .any(|configuration| {
                    configuration.target() == owner
                        && configuration
                            .members()
                            .declarations()
                            .any(|row| row == method)
                })
        {
            return None;
        }
        Some(Self {
            uri: uri.to_owned(),
            image,
            config,
            role: OriginalDeclarationRole::Method(method.side()),
            site: Some(method.declaration().site().clone()),
            owner: None,
            object_owner: Some(owner.clone()),
            input: Some(method.original_name_input().clone()),
            span: word.span(),
        })
    }
    /// Issue one current readonly identity from the canonical member owner.
    #[must_use]
    pub fn for_member_candidate(
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        candidate: &crate::method_symbol::OriginalMethodCandidate,
    ) -> Option<Self> {
        if !candidate.uri().is_empty() && candidate.uri() != uri {
            return None;
        }
        if let Some(object) = candidate.own_object() {
            return Self::for_object_method(uri, source, analysis, object, candidate.method());
        }
        Self::for_method(
            uri,
            source,
            analysis,
            candidate.declaring_class_in(source, analysis)?,
            candidate.method(),
        )
    }
    /// Issue a property identity without projecting it into a generated method.
    #[must_use]
    pub fn for_property(
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        class: &SourceDeclarationMetadata<ClassDef>,
        property: &OriginalSourcePropertyMetadata,
    ) -> Option<Self> {
        let class_identity = Self::for_class(uri, source, analysis, class)?;
        if !class
            .metadata()
            .original_properties
            .declarations()
            .any(|row| row == property)
        {
            return None;
        }
        let declaration = property.declaration();
        let key = declaration.name_input();
        if key.source_image() != &class_identity.image
            || key.lexer_config() != class_identity.config
        {
            return None;
        }
        Some(Self {
            role: OriginalDeclarationRole::Property(property.side()),
            site: Some(declaration.site().clone()),
            owner: class_identity.site.clone(),
            input: Some(property.original_name_input().clone()),
            span: key.span(),
            ..class_identity
        })
    }
    /// Retain a nameless lifecycle declaration's actual keyword/body owner.
    /// This supplies source advice, without lifecycle execution or absence.
    #[must_use]
    pub fn for_special(
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        class: &SourceDeclarationMetadata<ClassDef>,
        special: &OriginalSourceSpecialMemberMetadata,
    ) -> Option<Self> {
        let class_identity = Self::for_class(uri, source, analysis, class)?;
        if !class
            .metadata()
            .original_special_members
            .declarations()
            .any(|row| row == special)
        {
            return None;
        }
        let keyword = special.keyword_word();
        if keyword.image() != &class_identity.image || keyword.config() != class_identity.config {
            return None;
        }
        Some(Self {
            role: OriginalDeclarationRole::Special(special.side(), special.kind()),
            site: Some(special.site().clone()),
            owner: class_identity.site.clone(),
            input: None,
            span: keyword.span(),
            ..class_identity
        })
    }
    /// Reselect the genuine lifecycle syntax by its class owner and worker site.
    #[must_use]
    pub fn special_metadata<'a>(
        &self,
        analysis: &'a AnalysisResult,
    ) -> Option<&'a OriginalSourceSpecialMemberMetadata> {
        let OriginalDeclarationRole::Special(side, kind) = self.role else {
            return None;
        };
        let mut rows = self
            .class_metadata(analysis)?
            .metadata()
            .original_special_members
            .declarations()
            .filter(|row| {
                row.side() == side && row.kind() == kind && Some(row.site()) == self.site.as_ref()
            });
        let first = rows.next()?;
        rows.all(|row| row == first).then_some(first)
    }
    /// Owning URI, independent of identical source bytes in another document.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }
    /// Independently retained complete source and channel.
    #[must_use]
    pub const fn image(&self) -> &SourceImage {
        &self.image
    }
    /// Complete independently selected scanner configuration.
    #[must_use]
    pub const fn config(&self) -> LexerConfig {
        self.config
    }
    /// Readonly declaration purpose and member side.
    #[must_use]
    pub const fn role(&self) -> OriginalDeclarationRole {
        self.role
    }
    /// Original word geometry. It grants no replacement recipe.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
    /// Actual independently retained declaration producer.
    #[must_use]
    pub const fn input(&self) -> Option<&SignatureSourceNameInput> {
        self.input.as_ref()
    }
    /// Original declaration allocation site, independently of installation.
    #[must_use]
    pub const fn site(&self) -> Option<&CommandAllocationSite> {
        self.site.as_ref()
    }
    /// Actual current class row belonging to a class or member identity.
    #[must_use]
    pub fn class_metadata<'a>(
        &self,
        analysis: &'a AnalysisResult,
    ) -> Option<&'a SourceDeclarationMetadata<ClassDef>> {
        if self.object_owner.is_some() {
            return None;
        }
        let site = if self.role == OriginalDeclarationRole::Class {
            self.site.as_ref()?
        } else {
            self.owner.as_ref()?
        };
        let mut rows = analysis.original_class_declarations().filter(|row| {
            row.declaration_site() == site
                && row.name_input().source_image() == &self.image
                && row.name_input().lexer_config() == self.config
        });
        let first = rows.next()?;
        rows.all(|row| row == first).then_some(first)
    }
    /// Actual current procedure row at this retained declaration site.
    #[must_use]
    pub fn procedure_metadata<'a>(
        &self,
        analysis: &'a AnalysisResult,
    ) -> Option<&'a SourceDeclarationMetadata<ProcDef>> {
        if self.role != OriginalDeclarationRole::Procedure {
            return None;
        }
        let mut rows = analysis.original_procedure_declarations().filter(|row| {
            Some(row.declaration_site()) == self.site.as_ref()
                && self.input.as_ref()
                    == Some(&SignatureSourceNameInput::OriginalWord(
                        row.name_input().clone(),
                    ))
        });
        let first = rows.next()?;
        rows.all(|row| row == first).then_some(first)
    }
    /// Actual current method row without a reporting-name lookup.
    #[must_use]
    pub fn method_metadata<'a>(
        &self,
        analysis: &'a AnalysisResult,
    ) -> Option<&'a OriginalSourceMethodMetadata> {
        let OriginalDeclarationRole::Method(side) = self.role else {
            return None;
        };
        if let Some(owner) = &self.object_owner {
            let mut rows = analysis
                .original_object_configurations()
                .filter(|configuration| configuration.target() == owner)
                .flat_map(|configuration| configuration.members().declarations())
                .filter(|row| {
                    row.side() == side
                        && Some(row.declaration().site()) == self.site.as_ref()
                        && Some(row.declaration().original_name_input()) == self.input.as_ref()
                });
            let first = rows.next()?;
            return rows.next().is_none().then_some(first);
        }
        let mut rows = self
            .class_metadata(analysis)?
            .metadata()
            .original_members
            .declarations()
            .filter(|row| {
                row.side() == side
                    && Some(row.declaration().site()) == self.site.as_ref()
                    && Some(row.declaration().original_name_input()) == self.input.as_ref()
            });
        let first = rows.next()?;
        rows.all(|row| row == first).then_some(first)
    }
    /// Recheck this retained identity against the full current source/inventory.
    #[must_use]
    pub fn is_current(&self, uri: &str, source: &str, analysis: &AnalysisResult) -> bool {
        if self.uri != uri
            || Self::current(source, analysis).as_ref() != Some(&(self.image.clone(), self.config))
        {
            return false;
        }
        match self.role {
            OriginalDeclarationRole::Document => true,
            OriginalDeclarationRole::Procedure => {
                self.procedure_metadata(analysis).is_some_and(|row| {
                    Self::for_procedure(uri, source, analysis, row).as_ref() == Some(self)
                })
            }
            OriginalDeclarationRole::Class => self.class_metadata(analysis).is_some_and(|row| {
                Self::for_class(uri, source, analysis, row).as_ref() == Some(self)
            }),
            OriginalDeclarationRole::Method(_) => {
                if let Some(owner) = &self.object_owner {
                    return self.method_metadata(analysis).is_some_and(|row| {
                        Self::for_object_method(uri, source, analysis, owner, row).as_ref()
                            == Some(self)
                    });
                }
                self.class_metadata(analysis)
                    .zip(self.method_metadata(analysis))
                    .is_some_and(|(class, row)| {
                        Self::for_method(uri, source, analysis, class, row).as_ref() == Some(self)
                    })
            }
            OriginalDeclarationRole::Special(_, _) => self
                .class_metadata(analysis)
                .zip(self.special_metadata(analysis))
                .is_some_and(|(class, row)| {
                    Self::for_special(uri, source, analysis, class, row).as_ref() == Some(self)
                }),
            OriginalDeclarationRole::Property(side) => {
                self.class_metadata(analysis).is_some_and(|class| {
                    class
                        .metadata()
                        .original_properties
                        .declarations()
                        .any(|row| {
                            row.side() == side
                                && Self::for_property(uri, source, analysis, class, row).as_ref()
                                    == Some(self)
                        })
                })
            }
        }
    }
    /// Bind the independently selected document owner while rechecking the
    /// actual current metadata. This is readonly source ownership, not loading.
    #[must_use]
    pub fn owned_by(&self, uri: &str, source: &str, analysis: &AnalysisResult) -> Option<Self> {
        self.is_current(self.uri(), source, analysis).then(|| {
            let mut owned = self.clone();
            owned.uri = uri.to_owned();
            owned
        })
    }
    /// Render only after identity selection; opaque units remain distinguishable.
    #[must_use]
    pub fn label(&self) -> String {
        if let OriginalDeclarationRole::Special(_, kind) = self.role {
            return match kind {
                tcl_registry::definer::DefinitionSpecialMemberKind::Constructor => "constructor",
                tcl_registry::definer::DefinitionSpecialMemberKind::Destructor => "destructor",
            }
            .to_owned();
        }
        self.input
            .as_ref()
            .map(|input| tcl_syntax::native_string::resident_name_label(input.bytes()))
            .unwrap_or_else(|| "<top-level>".to_owned())
    }
}

/// Byte-offset adapter for the same current readonly cursor selection.
/// Offsets are source geometry and supply no naming or dispatch authority.
#[must_use]
pub fn select_at_offset(
    uri: &str,
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> std::ops::ControlFlow<Option<OriginalDeclarationIdentity>> {
    if source.get(..offset as usize).is_none() {
        return std::ops::ControlFlow::Break(None);
    }
    let position = tcl_lexer::LineIndex::new(source).position_at_utf16(offset, source);
    select(
        uri,
        source,
        analysis,
        position.line,
        position.character.get(),
    )
}

/// Select a current original declaration at a cursor. Missing retained
/// original lookup is terminal; display-word matching is never a fallback.
#[must_use]
pub fn select(
    uri: &str,
    source: &str,
    analysis: &AnalysisResult,
    line: u32,
    character: u32,
) -> std::ops::ControlFlow<Option<OriginalDeclarationIdentity>> {
    use std::ops::ControlFlow;
    if analysis.allows_lexical_declaration_advice() {
        return ControlFlow::Continue(());
    }
    if OriginalDeclarationIdentity::current(source, analysis).is_none() {
        return ControlFlow::Break(None);
    }
    if crate::definition::original_variable_cursor_retained(source, analysis, line, character) {
        return ControlFlow::Continue(());
    }
    let index = tcl_lexer::LineIndex::new(source);
    let offset = crate::definition::byte_offset_at(&index, source, line, character);
    if let ControlFlow::Break(selected) =
        crate::method_symbol::local_candidate(source, analysis, line, character)
    {
        return ControlFlow::Break(selected.and_then(|candidate| {
            OriginalDeclarationIdentity::for_member_candidate(uri, source, analysis, &candidate)
        }));
    }
    if let ControlFlow::Break(selected) =
        crate::math_function_symbol::select_at_offset(source, analysis, offset)
    {
        return ControlFlow::Break(selected.and_then(|selected| {
            selected.procedure_metadata().and_then(|row| {
                OriginalDeclarationIdentity::for_procedure(uri, source, analysis, row)
            })
        }));
    }
    let Some(rows) = declarations(uri, source, analysis) else {
        return ControlFlow::Break(None);
    };
    let mut names = rows
        .iter()
        .filter(|row| row.span.start() <= offset && offset < row.span.end());
    if let Some(first) = names.next() {
        return ControlFlow::Break(names.next().is_none().then(|| first.clone()));
    }
    let Some(invocation) = analysis
        .command_invocations
        .iter()
        .find(|inv| inv.range.start() <= offset && offset < inv.range.end())
    else {
        return ControlFlow::Continue(());
    };
    let (Some(input), Some(lookup)) =
        (&invocation.original_name_input, &invocation.original_lookup)
    else {
        return ControlFlow::Break(None);
    };
    if input != lookup.name_input()
        || lookup.site().source.source_image() != &SourceImage::document(source)
    {
        return ControlFlow::Break(None);
    }
    if invocation.resolved_command_reference.is_some() {
        let matched = analysis
            .original_procedure_declarations()
            .filter(|row| invocation_targets_declaration(source, analysis, invocation, row, true))
            .filter_map(|row| {
                OriginalDeclarationIdentity::for_procedure(uri, source, analysis, row)
            })
            .chain(
                analysis
                    .original_class_declarations()
                    .filter(|row| {
                        invocation_targets_declaration(source, analysis, invocation, row, true)
                    })
                    .filter_map(|row| {
                        OriginalDeclarationIdentity::for_class(uri, source, analysis, row)
                    }),
            )
            .collect::<Vec<_>>();
        return ControlFlow::Break(match matched.as_slice() {
            [one] => Some(one.clone()),
            _ => None,
        });
    }
    let headers = analysis
        .original_procedure_declarations()
        .filter_map(|row| {
            OriginalDeclarationIdentity::for_procedure(uri, source, analysis, row)
                .map(|identity| (row.name(), identity))
        })
        .chain(analysis.original_class_declarations().filter_map(|row| {
            OriginalDeclarationIdentity::for_class(uri, source, analysis, row)
                .map(|identity| (row.name(), identity))
        }));
    ControlFlow::Break(lookup.matching_publications(headers).and_then(|matched| {
        match matched.as_slice() {
            [one] => Some(one.clone()),
            _ => None,
        }
    }))
}

/// Readonly local references to this exact source declaration. Terminal command
/// links are optional; original method selection uses the shared member owner.
#[must_use]
pub fn reference_spans(
    identity: &OriginalDeclarationIdentity,
    source: &str,
    analysis: &AnalysisResult,
    follow_links: bool,
) -> Vec<Span> {
    if !identity.is_current(identity.uri(), source, analysis) {
        return Vec::new();
    }
    match identity.role() {
        OriginalDeclarationRole::Procedure => {
            identity
                .procedure_metadata(analysis)
                .map_or_else(Vec::new, |declaration| {
                    let mut spans: Vec<_> = analysis
                        .command_invocations
                        .iter()
                        .filter(|invocation| {
                            invocation_targets_declaration(
                                source,
                                analysis,
                                invocation,
                                declaration,
                                follow_links,
                            )
                        })
                        .map(|invocation| invocation.range)
                        .collect();
                    for occurrence in crate::math_function_symbol::occurrences(source, analysis)
                        .into_iter()
                        .flatten()
                    {
                        if math_function_targets_declaration_in(
                            source,
                            analysis,
                            source,
                            analysis,
                            &occurrence,
                            declaration,
                            follow_links,
                        ) {
                            spans.push(occurrence.span());
                        }
                    }
                    spans.sort_by_key(|span| (span.start(), span.end()));
                    spans.dedup();
                    spans
                })
        }
        OriginalDeclarationRole::Class => {
            identity
                .class_metadata(analysis)
                .map_or_else(Vec::new, |declaration| {
                    analysis
                        .command_invocations
                        .iter()
                        .filter(|invocation| {
                            invocation_targets_declaration(
                                source,
                                analysis,
                                invocation,
                                declaration,
                                follow_links,
                            )
                        })
                        .map(|invocation| invocation.range)
                        .collect()
                })
        }
        OriginalDeclarationRole::Method(_) => crate::method_symbol::source_queries(analysis)
            .into_iter()
            .filter_map(|query| {
                let std::ops::ControlFlow::Break(Some(candidate)) =
                    crate::method_symbol::candidate_for_analysis(analysis, &query)
                else {
                    return None;
                };
                (OriginalDeclarationIdentity::for_member_candidate(
                    identity.uri(),
                    source,
                    analysis,
                    &candidate,
                )
                .as_ref()
                    == Some(identity))
                .then_some(query.span())
            })
            .collect(),
        OriginalDeclarationRole::Property(_)
        | OriginalDeclarationRole::Special(_, _)
        | OriginalDeclarationRole::Document => Vec::new(),
    }
}

/// Compact editor data locating a sealed readonly receipt. Decoding the token
/// supplies no declaration or native authority until the registry reselects it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OriginalDeclarationHandle {
    schema: u8,
    token: u64,
}

/// Bounded session-owned declaration receipt storage. Tokens never alias an
/// evicted record and full source equality is checked on every resolution.
#[derive(Debug, Default)]
pub struct OriginalDeclarationRegistry {
    next: u64,
    rows: BTreeMap<u64, OriginalDeclarationIdentity>,
    order: VecDeque<u64>,
}

impl OriginalDeclarationRegistry {
    /// Retain this issued source identity and return its readonly wire locator.
    #[must_use]
    pub fn issue(
        &mut self,
        identity: OriginalDeclarationIdentity,
    ) -> Option<OriginalDeclarationHandle> {
        if let Some((token, _)) = self.rows.iter().find(|(_, row)| *row == &identity) {
            return Some(OriginalDeclarationHandle {
                schema: 1,
                token: *token,
            });
        }
        let token = self.next.checked_add(1)?;
        self.next = token;
        while self.rows.len() >= 4096 {
            self.rows.remove(&self.order.pop_front()?);
        }
        self.rows.insert(token, identity);
        self.order.push_back(token);
        Some(OriginalDeclarationHandle { schema: 1, token })
    }
    /// Reselect the authentic current metadata by its retained full receipt.
    #[must_use]
    pub fn resolve(
        &self,
        handle: OriginalDeclarationHandle,
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
    ) -> Option<OriginalDeclarationIdentity> {
        if handle.schema != 1 {
            return None;
        }
        self.rows
            .get(&handle.token)
            .filter(|row| row.is_current(uri, source, analysis))
            .cloned()
    }
}

/// Match an actual original call/reference to one independently retained
/// declaration. Direct references compare the called slot and policy; terminal
/// call edges may follow the already retained alias/import allocation. Missing
/// original lookup remains terminal, even when a reporting name agrees.
#[must_use]
pub fn invocation_targets_declaration<T>(
    source: &str,
    analysis: &AnalysisResult,
    invocation: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    declaration: &SourceDeclarationMetadata<T>,
    follow_links: bool,
) -> bool {
    invocation_targets_declaration_in(
        source,
        analysis,
        source,
        analysis,
        invocation,
        declaration,
        follow_links,
    )
}

/// Match independent consuming and declaring source owners. An identical
/// document copy is not an owning URI join; callers must retain that separately.
#[must_use]
pub fn invocation_targets_declaration_in<T>(
    source: &str,
    analysis: &AnalysisResult,
    declaration_source: &str,
    declaration_analysis: &AnalysisResult,
    invocation: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    declaration: &SourceDeclarationMetadata<T>,
    follow_links: bool,
) -> bool {
    tcl_compiler::source_graph::invocation_targets_declaration_in(
        source,
        analysis,
        declaration_source,
        declaration_analysis,
        invocation,
        declaration,
        follow_links,
    )
}

/// Match a checked original expression-function reference to an independently
/// current declaration. Function identifiers retain their own expression
/// producer and lookup purpose; they never become synthetic command words.
/// Owning URI uniqueness remains the caller's independent obligation.
#[must_use]
pub fn math_function_targets_declaration_in<T>(
    source: &str,
    analysis: &AnalysisResult,
    declaration_source: &str,
    declaration_analysis: &AnalysisResult,
    occurrence: &tcl_compiler::command_binding::OriginalMathFunctionOccurrence,
    declaration: &SourceDeclarationMetadata<T>,
    follow_links: bool,
) -> bool {
    let Some((image, config)) = OriginalDeclarationIdentity::current(source, analysis) else {
        return false;
    };
    let Some(registry) = analysis.resolved_registry() else {
        return false;
    };
    let Some((declaring_image, declaring_config)) =
        OriginalDeclarationIdentity::current(declaration_source, declaration_analysis)
    else {
        return false;
    };
    if !occurrence.matches_source(&image, config, registry)
        || declaration.name_input().source_image() != &declaring_image
        || declaration.name_input().lexer_config() != declaring_config
    {
        return false;
    }
    let Some(reference) = occurrence.command_reference() else {
        return false;
    };
    if reference.original_name_policy() != Some(declaration.name().policy()) {
        return false;
    }
    reference_matches_declaration(reference, declaration, follow_links)
}

pub(crate) fn reference_matches_declaration<T>(
    reference: &tcl_compiler::command_binding::SourceCommandReference,
    declaration: &SourceDeclarationMetadata<T>,
    follow_links: bool,
) -> bool {
    tcl_compiler::source_graph::reference_matches_declaration(reference, declaration, follow_links)
}

/// Match an API's literal Unicode command value at global lookup scope. This
/// performs the selected document ingress only; script escapes are not run and
/// no original source operand or runtime publication is constructed.
#[must_use]
pub fn literal_name_matches_publication(
    name: &str,
    publication: &tcl_compiler::signature_scan::scope::SignatureSourceCommand,
) -> Option<bool> {
    literal_name_matches_slot(name, publication.slot(), publication.policy())
}

/// Match an explicitly authored API name against native publication geometry.
/// The Document ingress and C namespace or Jim flat-key recipe are retained by
/// `policy`. This selects no source word, current command, or implementation.
#[must_use]
pub fn literal_name_matches_slot(
    name: &str,
    slot: &tcl_core_types::ByteCommandSlot,
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> Option<bool> {
    let bytes = tcl_syntax::backslash::native_source_literal_bytes(
        name.as_bytes(),
        tcl_lexer::SourceChannel::Document,
        policy.string_protocol(),
    )
    .ok()?;
    Some(match policy.recipe() {
        recipe @ tcl_syntax::naming::NativeNameProtocol::C(_) => {
            recipe
                .command_lookup_slot(tcl_syntax::naming::NativeNameContext::root(), &bytes)
                .ok()?
                == *slot
        }
        recipe @ tcl_syntax::naming::NativeNameProtocol::Jim084 => {
            slot.namespace.is_root()
                && recipe
                    .jim_command_lookup_keys(tcl_syntax::naming::NativeNameContext::root(), &bytes)
                    .ok()?
                    .first()
                    == Some(&slot.simple)
        }
    })
}

/// Every readonly original declaration in this current document. UI maps are
/// not consulted, and repeated sites/receiver sides remain separate identities.
#[must_use]
pub fn declarations(
    uri: &str,
    source: &str,
    analysis: &AnalysisResult,
) -> Option<Vec<OriginalDeclarationIdentity>> {
    OriginalDeclarationIdentity::current(source, analysis)?;
    let mut rows = Vec::new();
    for row in analysis.original_procedure_declarations() {
        rows.push(OriginalDeclarationIdentity::for_procedure(
            uri, source, analysis, row,
        )?);
    }
    for class in analysis.original_class_declarations() {
        rows.push(OriginalDeclarationIdentity::for_class(
            uri, source, analysis, class,
        )?);
        for method in class.metadata().original_members.declarations() {
            rows.push(OriginalDeclarationIdentity::for_method(
                uri, source, analysis, class, method,
            )?);
        }
        for special in class.metadata().original_special_members.declarations() {
            rows.push(OriginalDeclarationIdentity::for_special(
                uri, source, analysis, class, special,
            )?);
        }
        for property in class.metadata().original_properties.declarations() {
            rows.push(OriginalDeclarationIdentity::for_property(
                uri, source, analysis, class, property,
            )?);
        }
    }
    for configuration in analysis.original_object_configurations() {
        for method in configuration.members().declarations() {
            rows.push(OriginalDeclarationIdentity::for_object_method(
                uri,
                source,
                analysis,
                configuration.target(),
                method,
            )?);
        }
    }
    rows.dedup();
    Some(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_class_inventory_keeps_equal_source_owners_and_current_occupied_headers() {
        // Implementation contract: naming.consumer.original-type-and-implementation-navigation
        // docs/design/analysis/name-resolution-proofs/original-type-and-implementation-navigation.md
        let source = "oo::class create C {}\nproc occupied {} {}";
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let analysis = Analyser::new().analyse(source, dialect);
            let documents = [
                OriginalDeclarationDocument {
                    uri: "file:///one.tcl",
                    source,
                    analysis: &analysis,
                },
                OriginalDeclarationDocument {
                    uri: "file:///two.tcl",
                    source,
                    analysis: &analysis,
                },
            ];
            let inventory = OriginalClassInventory::from_documents(&documents).unwrap();
            assert_eq!(inventory.records().len(), 2, "{dialect}");
            assert_eq!(inventory.occupied_non_classes().len(), 2);
            assert_eq!(inventory.owner(0).unwrap().uri, "file:///one.tcl");
            assert_eq!(inventory.owner(1).unwrap().uri, "file:///two.tcl");
            assert!(inventory.owner(2).is_none());
            assert!(
                OriginalClassInventory::from_documents(&[documents[0], documents[0]]).is_none()
            );
            assert!(
                OriginalClassInventory::from_documents(&[OriginalDeclarationDocument {
                    source: "# stale",
                    ..documents[0]
                }])
                .is_none()
            );
            let mut different_config = analysis.clone();
            let mut config = analysis.body_lexer_config.unwrap();
            config.strict_quoting = !config.strict_quoting;
            different_config.body_lexer_config = Some(config);
            assert!(
                OriginalClassInventory::from_documents(&[OriginalDeclarationDocument {
                    analysis: &different_config,
                    ..documents[0]
                }])
                .is_none()
            );
        }
    }

    #[test]
    fn original_declaration_reference_matching_keeps_opaque_slots_and_actual_lookup_owners() {
        // Implementation contract: naming.editor.original-declaration-wire-identity
        // docs/design/analysis/name-resolution-proofs/original-declaration-wire-identity.md
        let source = r"proc p\uD800 {} {}; p\uD800; proc p\uD801 {} {}; p\uD801";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let declarations = analysis
            .original_procedure_declarations()
            .collect::<Vec<_>>();
        assert_eq!(declarations.len(), 2);
        let first = analysis
            .command_invocations
            .iter()
            .find(|inv| {
                inv.original_name_input
                    .as_ref()
                    .is_some_and(|input| input.bytes() == b"p\xed\xa0\x80")
            })
            .unwrap();
        assert!(invocation_targets_declaration(
            source,
            &analysis,
            first,
            declarations[0],
            false
        ));
        assert!(!invocation_targets_declaration(
            source,
            &analysis,
            first,
            declarations[1],
            false
        ));
        let mut missing = first.clone();
        missing.original_lookup = None;
        assert!(!invocation_targets_declaration(
            source,
            &analysis,
            &missing,
            declarations[0],
            true
        ));
        assert!(!invocation_targets_declaration(
            &format!("{source} # changed"),
            &analysis,
            first,
            declarations[0],
            true
        ));
        let mut presentation = first.clone();
        presentation.name = "p\\uD801".to_owned();
        presentation.resolved_qualified_name = Some("::unrelated".to_owned());
        assert!(invocation_targets_declaration(
            source,
            &analysis,
            &presentation,
            declarations[0],
            false
        ));
    }

    #[test]
    fn original_declaration_handles_reselect_complete_source_uri_and_scanner() {
        // Implementation contract: naming.editor.original-declaration-wire-identity
        // docs/design/analysis/name-resolution-proofs/original-declaration-wire-identity.md
        let source = r"proc p\uD800 {} {}; proc p\uD801 {} {}; proc p\uD800 {a} {}";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.all_procs.clear();
        let identities = declarations("file:///a.tcl", source, &analysis).unwrap();
        assert_eq!(identities.len(), 3);
        assert_ne!(identities[0], identities[2]);
        let mut registry = OriginalDeclarationRegistry::default();
        let handle = registry.issue(identities[0].clone()).unwrap();
        let wire = serde_json::to_value(handle).unwrap();
        let decoded = serde_json::from_value(wire).unwrap();
        assert_eq!(
            registry.resolve(decoded, "file:///a.tcl", source, &analysis),
            Some(identities[0].clone())
        );
        assert!(
            registry
                .resolve(handle, "file:///b.tcl", source, &analysis)
                .is_none()
        );
        assert!(
            registry
                .resolve(
                    handle,
                    "file:///a.tcl",
                    &format!("{source}\n# changed"),
                    &analysis
                )
                .is_none()
        );
        let changed = Analyser::new().analyse(source, "tcl9.0");
        assert!(
            registry
                .resolve(handle, "file:///a.tcl", source, &changed)
                .is_none()
        );
        let forged =
            serde_json::from_value(serde_json::json!({"schema":1,"token":999999})).unwrap();
        assert!(
            registry
                .resolve(forged, "file:///a.tcl", source, &analysis)
                .is_none()
        );
    }
}

#[cfg(test)]
mod own_object_identity_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_own_object_identity_keeps_uri_currency_and_wire_reselection() {
        // Implementation contract: naming.editor.original-own-object-member-navigation
        // docs/design/analysis/name-resolution-proofs/original-own-object-member-navigation.md
        let source = r"oo::class create C {}; C create object; oo::objdefine object method p\uD800 {} {}; object p\uD800";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.all_classes.clear();
        let configuration = analysis.original_object_configurations().next().unwrap();
        let method = configuration.members().declarations().next().unwrap();
        let a = OriginalDeclarationIdentity::for_object_method(
            "file:///a.tcl",
            source,
            &analysis,
            configuration.target(),
            method,
        )
        .unwrap();
        let b = OriginalDeclarationIdentity::for_object_method(
            "file:///b.tcl",
            source,
            &analysis,
            configuration.target(),
            method,
        )
        .unwrap();
        assert_ne!(a, b);
        assert!(a.class_metadata(&analysis).is_none());
        assert_eq!(a.method_metadata(&analysis), Some(method));
        assert!(
            declarations("file:///a.tcl", source, &analysis)
                .unwrap()
                .contains(&a)
        );
        let mut registry = OriginalDeclarationRegistry::default();
        let handle = registry.issue(a.clone()).unwrap();
        assert_eq!(
            registry.resolve(handle, "file:///a.tcl", source, &analysis),
            Some(a.clone())
        );
        assert!(
            registry
                .resolve(handle, "file:///b.tcl", source, &analysis)
                .is_none()
        );
        assert!(
            registry
                .resolve(handle, "file:///a.tcl", &format!("{source} "), &analysis)
                .is_none()
        );
        assert_eq!(reference_spans(&a, source, &analysis, true).len(), 1);
        let lenses = crate::code_lens::code_lenses(
            source,
            crate::profile_for_analysis(&analysis),
            Some(&analysis),
            None,
            "file:///a.tcl",
        );
        assert!(lenses.iter().any(|lens| lens.identity.as_ref() == Some(&a)));
    }
}
