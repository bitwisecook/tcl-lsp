// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Current readonly declaration identities and bounded editor wire locators.
//!
//! A wire locator reselects a retained identity; it cannot manufacture an
//! original operand, command installation, dispatch entry or edit capability.

use crate::wire_session::EditorWireSession;
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
        let image = analysis.retained_command_realm()?.original_source_image()?;
        if image.channel() != tcl_lexer::SourceChannel::Document
            || image.bytes() != source.as_bytes()
        {
            return None;
        }
        let image = image.clone();
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
        let (SignatureSourceNameInput::OriginalWord(_)
        | SignatureSourceNameInput::OriginalValue(_)) = method.original_name_input()
        else {
            return None;
        };
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
        let (SignatureSourceNameInput::OriginalWord(_)
        | SignatureSourceNameInput::OriginalValue(_)) = method.original_name_input()
        else {
            return None;
        };
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
        let (SignatureSourceNameInput::OriginalWord(_)
        | SignatureSourceNameInput::OriginalValue(_)) = property.original_name_input()
        else {
            return None;
        };
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
            uri.clone_into(&mut owned.uri);
            owned
        })
    }
    /// Compatibility presentation key from the already selected original slot.
    /// It supplies no lookup, invocation or editing identity.
    pub(crate) fn reporting_key(&self, analysis: &AnalysisResult) -> Option<String> {
        let slot = match self.role {
            OriginalDeclarationRole::Procedure => self.procedure_metadata(analysis)?.name().slot(),
            OriginalDeclarationRole::Document => return None,
            _ => self.class_metadata(analysis)?.name().slot(),
        };
        let owner = tcl_syntax::native_string::resident_name_label(
            &tcl_syntax::naming::native_command_full_name_bytes(slot),
        );
        Some(match self.role {
            OriginalDeclarationRole::Procedure | OriginalDeclarationRole::Class => owner,
            OriginalDeclarationRole::Method(side) => tcl_compiler::analyser::class_member_key(
                &owner,
                &self.label(),
                side == MemberSide::ClassObject,
            ),
            OriginalDeclarationRole::Property(_) => {
                tcl_compiler::analyser::class_property_key(&owner, &self.label())
            }
            OriginalDeclarationRole::Special(_, kind) => match kind {
                tcl_registry::definer::DefinitionSpecialMemberKind::Constructor => {
                    tcl_compiler::analyser::class_constructor_key(&owner)
                }
                tcl_registry::definer::DefinitionSpecialMemberKind::Destructor => {
                    tcl_compiler::analyser::class_destructor_key(&owner)
                }
            },
            OriginalDeclarationRole::Document => return None,
        })
    }

    /// Compare presentation keys from the current original class snapshot.
    /// A reported member need not carry a Native method identity.
    pub(crate) fn class_reports_member_key(&self, analysis: &AnalysisResult, key: &str) -> bool {
        if self.role != OriginalDeclarationRole::Class
            || !self
                .image
                .try_text()
                .is_ok_and(|source| self.is_current(self.uri(), source, analysis))
        {
            return false;
        }
        let Some(class) = self.class_metadata(analysis) else {
            return false;
        };
        let owner = tcl_syntax::native_string::resident_name_label(
            &tcl_syntax::naming::native_command_full_name_bytes(class.name().slot()),
        );
        let metadata = class.metadata();
        metadata.methods.values().any(|member| {
            !member.name_span.is_empty()
                && tcl_compiler::analyser::class_member_key(&owner, &member.name, false) == key
        }) || metadata.class_methods.values().any(|member| {
            !member.name_span.is_empty()
                && tcl_compiler::analyser::class_member_key(&owner, &member.name, true) == key
        }) || metadata.properties.values().any(|member| {
            !member.name_span.is_empty()
                && tcl_compiler::analyser::class_property_key(&owner, &member.name) == key
        }) || (metadata
            .constructors
            .last()
            .is_some_and(|member| !member.name_span.is_empty())
            && tcl_compiler::analyser::class_constructor_key(&owner) == key)
            || (metadata
                .destructor
                .as_ref()
                .is_some_and(|member| !member.name_span.is_empty())
                && tcl_compiler::analyser::class_destructor_key(&owner) == key)
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
        self.input.as_ref().map_or_else(
            || "<top-level>".to_owned(),
            |input| tcl_syntax::native_string::resident_name_label(input.bytes()),
        )
    }
}

/// `superclass` / `mixin` operands in this document's original class bodies
/// that select `identity`'s class, through the same first-publication relation
/// rule the type hierarchy walks (`original_direct_metadata_relations_for_records`).
/// Reporting only: these operands are class-name references, not invocations.
fn class_relation_reference_spans(
    identity: &OriginalDeclarationIdentity,
    source: &str,
    analysis: &AnalysisResult,
) -> Vec<Span> {
    use tcl_compiler::analyser::types::OriginalSourceClassRelationKind;
    let Some(target) = identity.class_metadata(analysis) else {
        return Vec::new();
    };
    let Some((image, config)) = OriginalDeclarationIdentity::current(source, analysis) else {
        return Vec::new();
    };
    let records = analysis.original_class_declarations().collect::<Vec<_>>();
    let Some(root) = records
        .iter()
        .position(|record| std::ptr::eq(*record, target))
    else {
        return Vec::new();
    };
    let occupied = analysis
        .original_procedure_declarations()
        .map(SourceDeclarationMetadata::name)
        .collect::<Vec<_>>();
    let mut spans = Vec::new();
    for record in &records {
        for effect in record.metadata().original_relations.effects() {
            if effect.side() != MemberSide::Instance
                || !matches!(
                    effect.kind(),
                    OriginalSourceClassRelationKind::Superclass
                        | OriginalSourceClassRelationKind::Mixin
                )
            {
                continue;
            }
            for relation in effect.values().unwrap_or_default() {
                let selected = relation.lookup().first_matching_publications(
                    records
                        .iter()
                        .enumerate()
                        .map(|(index, record)| (record.name(), Some(index)))
                        .chain(occupied.iter().map(|name| (*name, None))),
                );
                if selected.as_slice() != [Some(root)] {
                    continue;
                }
                // The operand word itself, written in this document under
                // its current grammar (the same check a procedure name gets).
                if let Some(key) = relation.name_input().original_word_key()
                    && key.source_image() == &image
                    && key.lexer_config() == config
                {
                    spans.push(key.span());
                }
            }
        }
    }
    spans
}

/// Byte-offset adapter for the same current readonly cursor selection.
/// Offsets are source geometry and supply no naming or dispatch authority.
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
    // A cursor resting just past a declaration name (the editor position
    // after its last character) still names that declaration, as the
    // lexical word lookup always allowed.
    let mut ends = rows
        .iter()
        .filter(|row| !row.span.is_empty() && row.span.end() == offset);
    if let Some(first) = ends.next() {
        return ControlFlow::Break(ends.next().is_none().then(|| first.clone()));
    }
    select_invocation(uri, source, analysis, offset)
}

fn select_invocation(
    uri: &str,
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> std::ops::ControlFlow<Option<OriginalDeclarationIdentity>> {
    use std::ops::ControlFlow;
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
    let Some(registry) = analysis.resolved_registry() else {
        return ControlFlow::Break(None);
    };
    let headers = analysis
        .original_procedure_declarations()
        .filter_map(|row| {
            OriginalDeclarationIdentity::for_procedure(uri, source, analysis, row)
                .map(|identity| (row.name(), row.declaration_site(), identity))
        })
        .chain(analysis.original_class_declarations().filter_map(|row| {
            OriginalDeclarationIdentity::for_class(uri, source, analysis, row)
                .map(|identity| (row.name(), row.declaration_site(), identity))
        }));
    ControlFlow::Break(
        lookup
            .matching_source_advice_publications(registry, headers)
            .and_then(|matched| match matched.as_slice() {
                [one] => Some(one.clone()),
                _ => None,
            }),
    )
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
            procedure_reference_spans(identity, source, analysis, follow_links)
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
        OriginalDeclarationRole::Property(_) => {
            tcl_compiler::analyser::class_hierarchy::original_metadata::
                original_source_self_property_references(source, analysis)
                .into_iter()
                .flatten()
                .filter_map(|reference| {
                    (OriginalDeclarationIdentity::for_property(
                        identity.uri(),
                        source,
                        analysis,
                        reference.declaring_class(),
                        reference.property(),
                    )
                    .as_ref()
                        == Some(identity))
                    .then_some(reference.selector_span())
                })
                .collect()
        }
        OriginalDeclarationRole::Special(_, _) => identity
            .class_metadata(analysis)
            .zip(identity.special_metadata(analysis))
            .map_or_else(Vec::new, |(class, special)| {
                crate::references::original_special_next_chain_spans(
                    source, analysis, class, special,
                )
            }),
        OriginalDeclarationRole::Document => Vec::new(),
    }
}

fn procedure_reference_spans(
    identity: &OriginalDeclarationIdentity,
    source: &str,
    analysis: &AnalysisResult,
    follow_links: bool,
) -> Vec<Span> {
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

/// Coverage of the independent conditional source-reference facet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadonlyReferenceCoverage {
    /// This role uses its existing inventory; no member source facet was requested.
    ExistingInventory,
    /// The checked source walk and typed member tape cover every selector,
    /// and no other command in this document may dispatch or capture the
    /// member. Other documents remain outside this census.
    CompleteSourceOccurrenceCensus,
    /// Genuine whole-source ownership retains partial positive occurrences.
    Partial,
    /// The independent source facet is unavailable; existing positives remain.
    Unavailable,
}

/// Readonly occurrences with explicit independent source coverage.
/// Neither variant proves an occurrence census, absence or edit coverage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadonlyReferenceReport {
    spans: Vec<Span>,
    coverage: ReadonlyReferenceCoverage,
}

impl ReadonlyReferenceReport {
    /// Positive source extents only, independently of editable occurrences.
    #[must_use]
    pub fn into_spans(self) -> Vec<Span> {
        self.spans
    }
    /// Borrow the positive readonly source extents.
    #[must_use]
    pub fn spans(&self) -> &[Span] {
        &self.spans
    }
    /// Partial source availability stays separate from its positive count.
    #[must_use]
    pub const fn coverage(&self) -> ReadonlyReferenceCoverage {
        self.coverage
    }
    pub(crate) fn extend(&mut self, spans: impl IntoIterator<Item = Span>) {
        self.spans.extend(spans);
        self.spans.sort_by_key(|span| (span.start(), span.end()));
        self.spans.dedup();
    }
}

/// Exact property occurrences require a complete member source census.
/// Applicability remains separate.
fn complete_property_source_occurrences(
    identity: &OriginalDeclarationIdentity,
    source: &str,
    analysis: &AnalysisResult,
    properties: &[tcl_compiler::analyser::class_hierarchy::original_metadata::OriginalSourceSelfPropertyReference],
) -> Option<Vec<Span>> {
    let methods = tcl_compiler::analyser::class_hierarchy::original_metadata::
        original_source_self_method_references(source, analysis)?;
    complete_member_source_census(identity, source, analysis, &methods, properties).then_some(())?;
    Some(
        properties
            .iter()
            .filter_map(|reference| {
                (OriginalDeclarationIdentity::for_property(
                    identity.uri(),
                    source,
                    analysis,
                    reference.declaring_class(),
                    reference.property(),
                )
                .as_ref()
                    == Some(identity))
                .then_some(reference.selector_span())
            })
            .collect(),
    )
}

/// One classified command of a readonly member census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemberCensusCommand {
    /// An immediate self-dispatch whose selector the typed tape must record.
    SelfDispatch,
    /// A command whose proven layout dispatches and captures no member.
    Other,
}

/// Inputs shared by every command of one readonly member census.
struct MemberCensus<'a> {
    source: &'a str,
    analysis: &'a AnalysisResult,
    context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
    syntax: crate::source_structure::SourceSyntaxStructure,
    name: &'a [u8],
    /// Literal operand text naming the member, with its whole word extent.
    mentions: Vec<(Span, Span)>,
}

/// A complete readonly census of one member's source occurrences. The
/// checked source walk must observe every command of every reachable region,
/// and every command must be classified with a proven layout:
/// - a definition member routes by its selected effect, and forwards,
///   filters, visibility, retraction, mixin relations, method references and
///   command-prefix, command-name or opaque-script roles stay incomplete;
/// - an immediate self-dispatch keeps the Registry selector layout and must
///   correspond one-to-one with the compiler's typed method/property tape;
/// - any other command needs complete selected roles, no retained callback
///   (stored script or command prefix), no substitution template, no
///   unrepresented definer form, no descriptor declaring an implementation
///   namespace (qualified `oo::Helpers` spellings and every `dict` call) and
///   no method-context helper without a declared dispatch kind;
/// - no other literal operand text may name the member outside a walked
///   nested script, since a captured selector (`[list my m]`, `callback m`,
///   an object command call) can dispatch without a self-dispatch site.
///
/// Unknown or dynamic heads, unavailable member geometry and dynamic
/// selectors stay incomplete. The census covers this one document only.
fn complete_member_source_census(
    identity: &OriginalDeclarationIdentity,
    source: &str,
    analysis: &AnalysisResult,
    methods: &[tcl_compiler::analyser::class_hierarchy::original_metadata::OriginalSourceSelfMethodReference],
    properties: &[tcl_compiler::analyser::class_hierarchy::original_metadata::OriginalSourceSelfPropertyReference],
) -> bool {
    let Some(name) = identity
        .input
        .as_ref()
        .map(SignatureSourceNameInput::bytes)
        .filter(|name| !name.is_empty() && name.is_ascii())
    else {
        return false;
    };
    let Some(input) = analysis.resolved_input.as_ref() else {
        return false;
    };
    let Some(syntax) = crate::source_structure::SourceSyntaxStructure::capture(source, analysis)
    else {
        return false;
    };
    let mut census = MemberCensus {
        source,
        analysis,
        context: input.context_registry(),
        syntax,
        name,
        mentions: Vec::new(),
    };
    let mut walked = Vec::new();
    let mut uncertain = false;
    let mut regions = Vec::new();
    let status = crate::executable_regions::visit_checked_analysis_executable_commands_with_regions(
        source,
        analysis,
        &mut |command, _head, execution_context| {
            match census.classify(command) {
                Some(MemberCensusCommand::Other) => {}
                Some(MemberCensusCommand::SelfDispatch)
                    if execution_context
                        == crate::executable_regions::ExecutableContext::PotentialBody =>
                {
                    walked.push(command.span);
                }
                Some(MemberCensusCommand::SelfDispatch) | None => uncertain = true,
            }
            false
        },
        &mut regions,
    );
    if status != crate::executable_regions::PotentialSourceVisit::Complete || uncertain {
        return false;
    }
    // A mention is excused only inside a walked script nested in its own
    // word, whose commands were classified one by one.
    if census.mentions.iter().any(|&(mention, word)| {
        !regions.iter().any(|&(region, _)| {
            word.start() <= region.start()
                && region.end() <= word.end()
                && region.start() <= mention.start()
                && mention.end() <= region.end()
        })
    }) {
        return false;
    }
    walked.sort_by_key(|span| (span.start(), span.end()));
    let Some(mut recorded) = methods
        .iter()
        .map(tcl_compiler::analyser::class_hierarchy::original_metadata::OriginalSourceSelfMethodReference::command_span)
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    recorded.extend(properties.iter().map(tcl_compiler::analyser::class_hierarchy::original_metadata::OriginalSourceSelfPropertyReference::command_span));
    recorded.sort_by_key(|span| (span.start(), span.end()));
    walked == recorded && !recorded.windows(2).any(|pair| pair[0] == pair[1])
}

impl MemberCensus<'_> {
    fn classify(
        &mut self,
        command: &tcl_compiler::segmenter::SegmentedCommand,
    ) -> Option<MemberCensusCommand> {
        use tcl_registry::{AuthoredSourceSelfDispatch, Traits};
        let head = command.argv.first()?;
        // Selected definition members have their own scoped vocabulary; a
        // global name lookup cannot classify them, and unavailable member
        // geometry has no global fallback.
        if let Some(member) = self.syntax.definition_member_at(head.span.start()).ok()? {
            return (census_member_matches(&member, command) && census_admits_member(&member))
                .then_some(MemberCensusCommand::Other);
        }
        let words =
            crate::original_invocation::source_registry_words(self.source, self.analysis, command)?;
        let selected = words
            .with_source_schema(&self.context, |schema| {
                if let Some(dispatch) = schema.authored_source_self_dispatch() {
                    return (dispatch != AuthoredSourceSelfDispatch::Unproven)
                        .then_some(MemberCensusCommand::SelfDispatch);
                }
                // `next`, `link`, `callback`/`mymethod` and any other
                // method-context helper without a declared dispatch kind can
                // reach a member without a self-dispatch site.
                let traits = schema.semantics.traits;
                if traits
                    .intersects(Traits::TCLOO_NEXT_CHAIN.union(Traits::TCLOO_BINDS_METHOD_ALIAS))
                    || (traits.contains(Traits::TCLOO_METHOD_CONTEXT)
                        && !traits.contains(Traits::TCLOO_INTROSPECTION))
                {
                    return None;
                }
                // A substitution template (`subst`) evaluates command
                // substitutions that neither the walker nor syntax coverage
                // enters, so an escaped or computed selector stays unseen.
                if traits.contains(Traits::PERFORMS_SUBSTITUTION)
                    || matches!(
                        schema.semantics.body_execution,
                        Some(tcl_registry::body_execution::BodyExecutionSpec::SubstitutionTemplate)
                    )
                {
                    return None;
                }
                // A descriptor that declares an implementation namespace is
                // not classified here: the qualified `::oo::Helpers::next` and
                // `::oo::Helpers::link` spellings drop the dispatch traits of
                // their contextual twins. Every `dict` call is also excluded,
                // because the bare `dict` descriptor declares `::tcl::dict`.
                if schema
                    .authored_source_descriptors()
                    .command
                    .implementation_namespace
                    .is_some()
                {
                    return None;
                }
                // A definer form without a selected definition body (an inline
                // `oo::define C forward …`) installs members no region shows.
                if schema
                    .authored_source_descriptors()
                    .command
                    .definition_body
                    .is_some()
                    && schema.authored_source_definition_body_grammar().is_none()
                {
                    return None;
                }
                (schema.authored_source_retains_callback() == Some(false))
                    .then_some(MemberCensusCommand::Other)
            })
            .flatten()?;
        if selected == MemberCensusCommand::SelfDispatch {
            // The selector is reconciled with the typed tape; passed values
            // may still carry a captured selector.
            return self
                .record_mentions(&words.operands, |position| position == 0)
                .then_some(selected);
        }
        let roles = words.roles.as_deref()?;
        if roles
            .iter()
            .any(|(_, role)| *role == tcl_registry::ArgRole::OpaqueScript)
        {
            return None;
        }
        // Variable-name operands cannot carry a selector.
        let exempt = roles
            .iter()
            .filter(|(_, role)| {
                matches!(
                    role,
                    tcl_registry::ArgRole::VarRead
                        | tcl_registry::ArgRole::VarWrite
                        | tcl_registry::ArgRole::LoopVarList
                        | tcl_registry::ArgRole::StaticVarList
                        | tcl_registry::ArgRole::ParamList
                )
            })
            .map(|(position, _)| *position)
            .collect::<Vec<_>>();
        self.record_mentions(&words.operands, |position| exempt.contains(&position))
            .then_some(selected)
    }

    /// Record literal operand text naming the member. This is a conservative
    /// lexical token scan of literal parts, not a list or script parse;
    /// command substitutions are classified as commands of their own. An
    /// operand without an original word (a captured or expanded argument)
    /// cannot be scanned, so the command stays unclassified.
    fn record_mentions(
        &mut self,
        operands: &[Option<crate::original_invocation::OriginalOperandSource>],
        skip: impl Fn(usize) -> bool,
    ) -> bool {
        for (position, operand) in operands.iter().enumerate() {
            if skip(position) {
                continue;
            }
            let Some(word) = operand.as_ref().and_then(|operand| operand.word.as_ref()) else {
                return false;
            };
            for part in word.executable_parts().all_parts() {
                let tcl_lexer::ExecutablePart::Text(text) = &part.part else {
                    continue;
                };
                if let tcl_lexer::ExecutableText::Decoded(bytes) = text {
                    if census_tokens(bytes).any(|(_, token)| census_names_member(token, self.name))
                    {
                        self.mentions.push((part.span, word.span()));
                    }
                    continue;
                }
                let Some(raw) = self.source.get(part.span.as_range()) else {
                    return false;
                };
                for (offset, token) in census_tokens(raw.as_bytes()) {
                    if !census_names_member(token, self.name) {
                        continue;
                    }
                    let mention = u32::try_from(offset)
                        .ok()
                        .and_then(|offset| part.span.start().checked_add(offset))
                        .zip(u32::try_from(token.len()).ok())
                        .and_then(|(start, len)| Some(Span::new(start, start.checked_add(len)?)))
                        .unwrap_or(part.span);
                    self.mentions.push((mention, word.span()));
                }
            }
        }
        true
    }
}

/// The selected member vector is exactly the walked command: equal extents
/// and word tokens, without the separators a segmented command also keeps.
/// An expanded member keeps its `Expand` token and stays unmatched.
fn census_member_matches(
    member: &crate::source_structure::SourceSyntaxDefinitionMember<'_>,
    command: &tcl_compiler::segmenter::SegmentedCommand,
) -> bool {
    let words = member.original_words();
    words
        .first()
        .is_some_and(|word| word.span().start() == command.span.start())
        && words
            .last()
            .is_some_and(|word| word.span().end() == command.span.end())
        && words
            .iter()
            .flat_map(|word| word.tokens().iter())
            .copied()
            .eq(command.all_tokens.iter().copied().filter(|token| {
                !matches!(
                    token.kind,
                    tcl_lexer::TokenType::Sep | tcl_lexer::TokenType::Eol
                )
            }))
}

/// A member admitted by the census declares or relates only: its effect
/// cannot dispatch or name a method later, and it carries no method
/// reference, command prefix, command name or opaque script. Routing uses
/// the selected member row's effect, never its keyword.
fn census_admits_member(
    member: &crate::source_structure::SourceSyntaxDefinitionMember<'_>,
) -> bool {
    use tcl_registry::ArgRole;
    use tcl_registry::definer::{MemberEffect, MemberRefKind, RelationSlot};
    let region = member.region();
    let words = member.original_words();
    region.checked_member_row(words).is_some_and(|row| {
        matches!(
            row.effect,
            MemberEffect::Callable { .. }
                | MemberEffect::StateDeclaration { .. }
                | MemberEffect::InitScript { .. }
                | MemberEffect::Configuration
                | MemberEffect::Relation {
                    slot: RelationSlot::Superclass
                }
        )
    }) && region.checked_references(words).is_some_and(|references| {
        references
            .iter()
            .all(|reference| reference.kind() == MemberRefKind::Class)
    }) && [
        ArgRole::CommandPrefix,
        ArgRole::CommandName,
        ArgRole::OpaqueScript,
    ]
    .into_iter()
    .all(|role| {
        region
            .argument_words_for_role(words, role)
            .is_some_and(|words| words.is_empty())
    })
}

/// Tokens of literal text split at whitespace and Tcl word, script, list and
/// substitution punctuation, with their byte offsets.
fn census_tokens(bytes: &[u8]) -> impl Iterator<Item = (usize, &[u8])> {
    let mut start = 0;
    bytes
        .split(|byte| byte.is_ascii_whitespace() || b"[]{}\";$\\()".contains(byte))
        .map(move |token| {
            let offset = start;
            start += token.len() + 1;
            (offset, token)
        })
        .filter(|(_, token)| !token.is_empty())
}

/// Whether a literal token may name the member: the name itself, a unique
/// `-` option prefix of it (property `configure`/`cget`), or an internal
/// accessor method name such as `<ReadProp-name>`.
fn census_names_member(token: &[u8], name: &[u8]) -> bool {
    token == name
        || token
            .strip_prefix(b"-")
            .is_some_and(|prefix| !prefix.is_empty() && name.starts_with(prefix))
        || (token.contains(&b'<') && token.windows(name.len()).any(|window| window == name))
}

/// Add conditional original member-body source correspondences for reporting.
/// Existing declaration/reference and transformation inventories are unchanged.
#[must_use]
pub fn readonly_reference_report(
    identity: &OriginalDeclarationIdentity,
    source: &str,
    analysis: &AnalysisResult,
    follow_links: bool,
) -> Option<ReadonlyReferenceReport> {
    identity
        .is_current(identity.uri(), source, analysis)
        .then_some(())?;
    crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let mut report = ReadonlyReferenceReport {
        spans: reference_spans(identity, source, analysis, follow_links),
        coverage: ReadonlyReferenceCoverage::ExistingInventory,
    };
    if matches!(identity.role(), OriginalDeclarationRole::Property(_)) {
        match tcl_compiler::analyser::class_hierarchy::original_metadata::
            original_source_self_property_references(source, analysis)
        {
            Some(references) => {
                if let Some(occurrences) = complete_property_source_occurrences(
                    identity, source, analysis, &references,
                ) {
                    report.coverage = ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus;
                    report.extend(occurrences);
                } else {
                    report.coverage = ReadonlyReferenceCoverage::Partial;
                    report.extend(references.into_iter().filter_map(|reference| {
                        (OriginalDeclarationIdentity::for_property(
                            identity.uri(),
                            source,
                            analysis,
                            reference.declaring_class(),
                            reference.property(),
                        )
                        .as_ref()
                            == Some(identity))
                        .then_some(reference.selector_span())
                    }));
                }
            }
            None => report.coverage = ReadonlyReferenceCoverage::Unavailable,
        }
        return Some(report);
    }
    if identity.role() == OriginalDeclarationRole::Class {
        report.extend(class_relation_reference_spans(identity, source, analysis));
    }
    if identity.role() != OriginalDeclarationRole::Method(MemberSide::Instance)
        || identity.object_owner.is_some()
    {
        return Some(report);
    }
    let references = tcl_compiler::analyser::class_hierarchy::original_metadata::
        original_source_self_method_references(source, analysis);
    match references {
        Some(references) => {
            let complete = tcl_compiler::analyser::class_hierarchy::original_metadata::
                original_source_self_property_references(source, analysis)
                .is_some_and(|properties| {
                    complete_member_source_census(
                        identity,
                        source,
                        analysis,
                        &references,
                        &properties,
                    )
                });
            report.coverage = if complete {
                ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus
            } else {
                ReadonlyReferenceCoverage::Partial
            };
            report.extend(references.into_iter().filter_map(|reference| {
                (OriginalDeclarationIdentity::for_method(
                    identity.uri(),
                    source,
                    analysis,
                    reference.declaring_class(),
                    reference.method(),
                )
                .as_ref()
                    == Some(identity))
                .then_some(reference.selector_span())
            }));
        }
        None => report.coverage = ReadonlyReferenceCoverage::Unavailable,
    }
    Some(report)
}

/// Compact editor data locating a sealed readonly receipt. Decoding the token
/// supplies no declaration or native authority until the registry reselects it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OriginalDeclarationHandle {
    schema: u8,
    token: u64,
    // Lossless JSON bytes identify this registry allocation only. Legacy
    // Native/TYPE wire objects retain exactly their existing two fields.
    session: Option<EditorWireSession>,
}

impl Serialize for OriginalDeclarationHandle {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Wire {
            schema: u8,
            token: u64,
            #[serde(skip_serializing_if = "Option::is_none")]
            session: Option<EditorWireSession>,
            #[serde(rename = "wirePurpose", skip_serializing_if = "Option::is_none")]
            purpose: Option<&'static str>,
        }
        Wire {
            schema: self.schema,
            token: self.token,
            session: self.session,
            purpose: self.session.map(|_| {
                if self.schema == 5 {
                    "originalSourceViewRelationship"
                } else {
                    "logicalSourceRelationship"
                }
            }),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OriginalDeclarationHandle {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Legacy {
            schema: u8,
            token: u64,
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Session {
            schema: u8,
            token: u64,
            #[serde(rename = "session")]
            allocation: EditorWireSession,
            #[serde(rename = "wirePurpose")]
            purpose: RelationshipPurpose,
        }
        #[derive(Deserialize)]
        enum RelationshipPurpose {
            #[serde(rename = "logicalSourceRelationship")]
            LogicalSourceRelationship,
            #[serde(rename = "originalSourceViewRelationship")]
            OriginalSourceViewRelationship,
        }
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Session(Session),
            Legacy(Legacy),
        }
        match Wire::deserialize(deserializer)? {
            Wire::Legacy(wire) => Ok(Self {
                schema: wire.schema,
                token: wire.token,
                session: None,
            }),
            // The explicit transport variant is independent of the claimed
            // semantic schema. Wrong schemas decode as inert locators; every
            // actual resolver still enforces its own kind and retained owner.
            Wire::Session(wire) => {
                let purpose_matches = match wire.purpose {
                    RelationshipPurpose::OriginalSourceViewRelationship => wire.schema == 5,
                    RelationshipPurpose::LogicalSourceRelationship => wire.schema != 5,
                };
                if !purpose_matches {
                    return Err(serde::de::Error::custom(
                        "source relationship wire purpose mismatch",
                    ));
                }
                Ok(Self {
                    schema: wire.schema,
                    token: wire.token,
                    session: Some(wire.allocation),
                })
            }
        }
    }
}

/// Session-only host publication payload. This stores and compares an actual
/// host-owned typed stamp; it cannot issue semantic/source authority and is
/// never serialised into editor data. Different host stamp types never match.
#[derive(Debug, Clone)]
pub struct LogicalCallHostPublication(std::sync::Arc<dyn HostPublicationValue>);
trait HostPublicationValue: std::fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
    fn same_value(&self, other: &dyn HostPublicationValue) -> bool;
}
impl<T: std::fmt::Debug + Eq + Send + Sync + 'static> HostPublicationValue for T {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn same_value(&self, other: &dyn HostPublicationValue) -> bool {
        other
            .as_any()
            .downcast_ref::<T>()
            .is_some_and(|other| self == other)
    }
}
impl LogicalCallHostPublication {
    /// Retain a host-owned stamp independently of the source selection. This
    /// constructor checks no host currency; its actual issuer must do so under
    /// the host's publication lease before issuing or resolving a readonly handle.
    #[must_use]
    pub fn retain<T: std::fmt::Debug + Eq + Send + Sync + 'static>(stamp: T) -> Self {
        Self(std::sync::Arc::new(stamp))
    }
}
impl PartialEq for LogicalCallHostPublication {
    fn eq(&self, other: &Self) -> bool {
        self.0.same_value(other.0.as_ref())
    }
}
impl Eq for LogicalCallHostPublication {}

/// A source view's typed host publication stamp. The host must validate its
/// actual physical parent, path, source and publication leases independently.
/// This equality-only payload grants no semantic or Native authority and is
/// never serialized into editor data.
#[derive(Debug, Clone)]
pub struct OriginalSourceViewPublication(std::sync::Arc<dyn HostPublicationValue>);

impl OriginalSourceViewPublication {
    /// Retain a host-owned stamp without asserting that it is current.
    #[must_use]
    pub fn retain<T: std::fmt::Debug + Eq + Send + Sync + 'static>(stamp: T) -> Self {
        Self(std::sync::Arc::new(stamp))
    }
}
impl PartialEq for OriginalSourceViewPublication {
    fn eq(&self, other: &Self) -> bool {
        self.0.same_value(other.0.as_ref())
    }
}
impl Eq for OriginalSourceViewPublication {}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RetainedDeclaration {
    Original(OriginalDeclarationIdentity),
    OriginalInSourceView {
        identity: OriginalDeclarationIdentity,
        publication: OriginalSourceViewPublication,
    },
    LogicalClass(Box<crate::type_hierarchy::LogicalHierarchyClass>),
    LogicalCall {
        identity: crate::call_hierarchy::LogicalCallIdentity,
        publication: LogicalCallHostPublication,
    },
    LogicalLens {
        identity: crate::code_lens::LogicalLensIdentity,
        publication: LogicalCallHostPublication,
    },
}

/// Bounded session-owned declaration receipt storage. Tokens never alias an
/// evicted record and full source equality is checked on every resolution.
#[derive(Debug)]
pub struct OriginalDeclarationRegistry {
    next: u64,
    rows: BTreeMap<u64, RetainedDeclaration>,
    order: VecDeque<u64>,
    // Real OS/Web Crypto allocation randomness, never source/name authority.
    // Failure withholds the new session-scoped kinds independently of legacy
    // Native/TYPE issuance and resolution.
    session: Option<EditorWireSession>,
}
impl Default for OriginalDeclarationRegistry {
    fn default() -> Self {
        Self {
            next: 0,
            rows: BTreeMap::new(),
            order: VecDeque::new(),
            session: EditorWireSession::allocate(),
        }
    }
}

impl OriginalDeclarationRegistry {
    /// Retain this issued source identity and return its readonly wire locator.
    #[must_use]
    pub fn issue(
        &mut self,
        identity: OriginalDeclarationIdentity,
    ) -> Option<OriginalDeclarationHandle> {
        self.issue_retained(RetainedDeclaration::Original(identity), 1)
    }
    /// Bind a genuine readonly declaration to its independently checked host
    /// source view. Issue only inside the host's current publication lease.
    #[must_use]
    pub fn issue_in_source_view(
        &mut self,
        identity: OriginalDeclarationIdentity,
        publication: OriginalSourceViewPublication,
    ) -> Option<OriginalDeclarationHandle> {
        self.issue_retained(
            RetainedDeclaration::OriginalInSourceView {
                identity,
                publication,
            },
            5,
        )
    }

    /// Read the saved typed host stamp to select its exact live publication.
    /// The returned stamp is inert: the host must revalidate it and call
    /// `resolve_in_source_view` before using a declaration. No URI/text fallback
    /// may renew a missing or retired saved publication.
    #[must_use]
    pub fn source_view_publication<T: 'static>(
        &self,
        handle: OriginalDeclarationHandle,
    ) -> Option<&T> {
        if handle.schema != 5 || !self.matches_logical_session(handle) {
            return None;
        }
        let RetainedDeclaration::OriginalInSourceView { publication, .. } =
            self.rows.get(&handle.token)?
        else {
            return None;
        };
        publication.0.as_any().downcast_ref::<T>()
    }

    /// Reselect this exact current source-view publication and original owner.
    /// A legacy locator cannot cast to this kind; neither can CALL/TYPE/LENS.
    #[must_use]
    pub fn resolve_in_source_view(
        &self,
        handle: OriginalDeclarationHandle,
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        current_publication: &OriginalSourceViewPublication,
    ) -> Option<OriginalDeclarationIdentity> {
        if handle.schema != 5 || !self.matches_logical_session(handle) {
            return None;
        }
        let RetainedDeclaration::OriginalInSourceView {
            identity,
            publication,
        } = self.rows.get(&handle.token)?
        else {
            return None;
        };
        (publication == current_publication && identity.is_current(uri, source, analysis))
            .then(|| identity.clone())
    }

    /// Retain a sealed Logical class selection under a separate wire schema.
    #[must_use]
    pub fn issue_logical_class(
        &mut self,
        identity: crate::type_hierarchy::LogicalHierarchyClass,
    ) -> Option<OriginalDeclarationHandle> {
        self.issue_retained(RetainedDeclaration::LogicalClass(Box::new(identity)), 2)
    }
    /// Retain a genuine Logical CALL selection under its own wire kind.
    #[must_use]
    pub fn issue_logical_call(
        &mut self,
        identity: crate::call_hierarchy::LogicalCallIdentity,
        publication: LogicalCallHostPublication,
    ) -> Option<OriginalDeclarationHandle> {
        self.issue_retained(
            RetainedDeclaration::LogicalCall {
                identity,
                publication,
            },
            3,
        )
    }
    /// Retain a genuine Logical source lens under its own wire schema.
    /// The equality-only host payload must be minted inside the actual source,
    /// provider and index publication lease; it supplies no semantic authority.
    #[must_use]
    pub fn issue_logical_lens(
        &mut self,
        identity: crate::code_lens::LogicalLensIdentity,
        publication: LogicalCallHostPublication,
    ) -> Option<OriginalDeclarationHandle> {
        self.issue_retained(
            RetainedDeclaration::LogicalLens {
                identity,
                publication,
            },
            4,
        )
    }
    fn issue_retained(
        &mut self,
        identity: RetainedDeclaration,
        schema: u8,
    ) -> Option<OriginalDeclarationHandle> {
        let session = if matches!(schema, 3..=5) {
            Some(self.session?)
        } else {
            None
        };
        if let Some((token, _)) = self.rows.iter().find(|(_, row)| *row == &identity) {
            return Some(OriginalDeclarationHandle {
                schema,
                token: *token,
                session,
            });
        }
        let token = self.next.checked_add(1)?;
        self.next = token;
        while self.rows.len() >= 4096 {
            self.rows.remove(&self.order.pop_front()?);
        }
        self.rows.insert(token, identity);
        self.order.push_back(token);
        Some(OriginalDeclarationHandle {
            schema,
            token,
            session,
        })
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
        if handle.schema != 1 || handle.session.is_some() {
            return None;
        }
        let RetainedDeclaration::Original(identity) = self.rows.get(&handle.token)? else {
            return None;
        };
        identity
            .is_current(uri, source, analysis)
            .then(|| identity.clone())
    }
    /// Reselect one current Logical class; Native and forged wire schemas refuse.
    #[must_use]
    pub fn resolve_logical_class(
        &self,
        handle: OriginalDeclarationHandle,
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
    ) -> Option<crate::type_hierarchy::LogicalHierarchyClass> {
        if handle.schema != 2 || handle.session.is_some() {
            return None;
        }
        let RetainedDeclaration::LogicalClass(identity) = self.rows.get(&handle.token)? else {
            return None;
        };
        identity
            .is_current(uri, source, analysis)
            .then(|| identity.as_ref().clone())
    }
    /// Session allocation is a transport discriminator, independent of source
    /// equality and genuine publication revalidation. Missing entropy refuses
    /// before any row can be selected, including in a fresh registry.
    fn matches_logical_session(&self, handle: OriginalDeclarationHandle) -> bool {
        self.session.is_some() && handle.session == self.session
    }

    /// Reborrow the current source CALL receipt. Native and TYPE tokens cannot
    /// cast to CALL, and client labels/ranges do not participate in selection.
    #[must_use]
    pub fn resolve_logical_call(
        &self,
        handle: OriginalDeclarationHandle,
        uri: &str,
        documents: &[OriginalDeclarationDocument<'_>],
        current_publication: &LogicalCallHostPublication,
    ) -> Option<crate::call_hierarchy::LogicalCallIdentity> {
        if handle.schema != 3 || !self.matches_logical_session(handle) {
            return None;
        }
        let RetainedDeclaration::LogicalCall {
            identity,
            publication,
        } = self.rows.get(&handle.token)?
        else {
            return None;
        };
        (publication == current_publication
            && identity.uri() == uri
            && identity.is_current_in(documents))
        .then(|| identity.clone())
    }
    /// Reselect a current Logical lens without reading client labels/ranges.
    /// Native, TYPE and CALL handles cannot cast to this separate schema.
    #[must_use]
    pub fn resolve_logical_lens(
        &self,
        handle: OriginalDeclarationHandle,
        uri: &str,
        documents: &[OriginalDeclarationDocument<'_>],
        current_publication: &LogicalCallHostPublication,
    ) -> Option<crate::code_lens::LogicalLensIdentity> {
        if handle.schema != 4 || !self.matches_logical_session(handle) {
            return None;
        }
        let RetainedDeclaration::LogicalLens {
            identity,
            publication,
        } = self.rows.get(&handle.token)?
        else {
            return None;
        };
        (publication == current_publication
            && identity.uri() == uri
            && identity.is_current_in(documents))
        .then(|| identity.clone())
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
        for method in class
            .metadata()
            .original_members
            .declarations()
            .filter(|method| method.is_primary_source_declaration())
        {
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
mod readonly_source_reference_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn readonly_member_reports_keep_source_spans_out_of_edit_inventories() {
        let source = "oo::class create C {method ping {} {}; method body {} {my ping; my ping}}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let class = analysis.original_class_declarations().next().unwrap();
        let method = class
            .metadata()
            .original_members
            .declarations()
            .find(|method| method.original_name_input().bytes() == b"ping")
            .unwrap();
        let identity =
            OriginalDeclarationIdentity::for_method("", source, &analysis, class, method).unwrap();
        let old = reference_spans(&identity, source, &analysis, true);
        let report = readonly_reference_report(&identity, source, &analysis, true).unwrap();
        assert_eq!(
            report.coverage(),
            ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus
        );
        assert_eq!(report.spans().len(), 2);
        assert!(
            report
                .spans()
                .iter()
                .all(|span| &source[span.as_range()] == "ping")
        );
        assert!(
            old.is_empty(),
            "the pre-existing reference kernel does not acquire the source facet"
        );
        let (_, strong) = crate::references::method_references_for_declaration(
            source,
            analysis.resolved_profile().unwrap(),
            &analysis,
            class.metadata(),
            method.metadata(),
            false,
        );
        assert!(
            strong.is_empty(),
            "source-only selectors cannot enter a transformation inventory"
        );
        assert!(
            crate::references::method_references_for_class(
                source,
                analysis.resolved_profile().unwrap(),
                &analysis,
                "::C",
                "ping",
                false,
            )
            .is_none()
        );
        assert!(
            crate::rename::method_spans_in_document(
                source,
                analysis.resolved_profile().unwrap(),
                &analysis,
                "::C",
                "ping",
                false,
            )
            .is_empty()
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(readonly_reference_report(&identity, source, &missing, true).is_none());
    }

    fn member_report(source: &str, dialect: &str, name: &[u8]) -> ReadonlyReferenceReport {
        let analysis = Analyser::new().analyse(source, dialect);
        let class = analysis
            .original_class_declarations()
            .find(|class| {
                class
                    .metadata()
                    .original_members
                    .declarations()
                    .any(|method| method.original_name_input().bytes() == name)
                    || class
                        .metadata()
                        .original_properties
                        .declarations()
                        .any(|property| property.original_name_input().bytes() == name)
            })
            .unwrap();
        let identity = class
            .metadata()
            .original_members
            .declarations()
            .find(|method| method.original_name_input().bytes() == name)
            .map_or_else(
                || {
                    let property = class
                        .metadata()
                        .original_properties
                        .declarations()
                        .find(|property| property.original_name_input().bytes() == name)
                        .unwrap();
                    OriginalDeclarationIdentity::for_property(
                        "", source, &analysis, class, property,
                    )
                },
                |method| {
                    OriginalDeclarationIdentity::for_method("", source, &analysis, class, method)
                },
            )
            .unwrap();
        readonly_reference_report(&identity, source, &analysis, true).unwrap()
    }

    #[test]
    fn readonly_method_census_completes_only_proven_self_dispatch_shapes() {
        for (source, spans) in [
            ("oo::class create C {method ping {} {}}", 0),
            (
                "oo::class create C {method ping {} {}; method body {} {my ping; my ping}}",
                2,
            ),
            (
                "oo::class create Base {method ping {} {}}; oo::class create Child {superclass Base; method body {} {my ping}}",
                1,
            ),
        ] {
            let report = member_report(source, "tcl8.6", b"ping");
            assert_eq!(
                report.coverage(),
                ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus,
                "{source}"
            );
            assert_eq!(report.spans().len(), spans, "{source}");
        }
    }

    #[test]
    fn readonly_member_census_fails_closed_for_deferred_and_unproven_dispatch() {
        let body = |tail: &str| {
            format!(
                "oo::class create C {{method ping {{}} {{}}; method body {{script cmd}} {{my ping; {tail}}}}}"
            )
        };
        let member = |member: &str| {
            format!(
                "oo::class create M {{}}; oo::class create C {{method ping {{}} {{}}; {member}; method body {{}} {{my ping}}}}"
            )
        };
        let mut cases = vec![
            // Retained callbacks: a stored script, a command prefix or a
            // wrapped prefix may dispatch later without a self-dispatch site.
            ("tcl8.6", body("after 0 [list my ping]")),
            ("tcl8.6", body("after idle {my ping}")),
            (
                "tcl8.6",
                body("trace add variable ::v write [list my ping]"),
            ),
            ("tcl8.6", body("set handler [namespace code {my ping}]")),
            ("tcl8.6", body("lsort -command {my ping} {b a}")),
            // A captured selector that leaves the document.
            ("tcl8.6", body("return [list my ping]")),
            ("tcl8.6", body("set captured {my ping}")),
            // Dynamic selectors, heads and scripts.
            ("tcl8.6", body("my $script")),
            ("tcl8.6", body("{*}$cmd")),
            ("tcl8.6", body("[self] ping")),
            ("tcl8.6", body("eval $script")),
            ("tcl8.6", body("uplevel 1 $script")),
            ("tcl8.6", body("mystery")),
            // Substitution templates evaluate escaped or computed selectors.
            ("tcl8.6", body(r"subst {[my p\x69ng]}")),
            ("tcl9.0", body(r"subst {[my p\x69ng]}")),
            ("tcl8.6", body("set s [string cat pi ng]; subst {[my $s]}")),
            ("tcl9.0", body("set s [string cat pi ng]; subst {[my $s]}")),
            // Method-chain and alias helpers, bare and namespaced.
            ("tcl8.6", body("next")),
            (
                "tcl8.6",
                "oo::class create Base {method ping {} {}}; oo::class create Child {superclass Base; method ping {} {::oo::Helpers::next}}".to_owned(),
            ),
            (
                "tcl9.0",
                "oo::class create Base {method ping {} {}}; oo::class create Child {superclass Base; method ping {} {::oo::Helpers::next}}".to_owned(),
            ),
            (
                "tcl8.6",
                "oo::class create Base {method ping {} {}}; oo::class create Child {superclass Base; method ping {} {::oo::Helpers::nextto Base}}".to_owned(),
            ),
            (
                "tcl9.0",
                "oo::class create C {method ping {} {}; constructor {} {::oo::Helpers::link [list set [string cat pi ng]]}; method body {} {set a 1}}".to_owned(),
            ),
            // Definition members that dispatch, name or reorder methods.
            ("tcl8.6", member("forward fw my ping")),
            ("tcl8.6", member("filter ping")),
            ("tcl8.6", member("export ping")),
            ("tcl8.6", member("mixin M")),
            // Members installed outside a selected definition body, and an
            // object command call.
            (
                "tcl8.6",
                format!("{}; oo::define C forward fw my ping", body("return")),
            ),
            ("tcl8.6", format!("{}; C create c; c ping", body("return"))),
        ];
        for tail in [
            "callback ping",
            "mymethod ping",
            "oo::Helpers::callback ping",
            "link ping",
        ] {
            cases.push(("tcl9.0", body(tail)));
        }
        for (dialect, source) in &cases {
            let report = member_report(source, dialect, b"ping");
            assert_eq!(
                report.coverage(),
                ReadonlyReferenceCoverage::Partial,
                "{dialect}: {source}"
            );
        }
        for source in [
            "oo::class create C {property size; method body {} {my size; my configure -size 3}}",
            "oo::class create C {property size; method body {} {my size}}; C create c; c configure -si 4",
        ] {
            let report = member_report(source, "tcl9.0", b"size");
            assert_eq!(
                report.coverage(),
                ReadonlyReferenceCoverage::Partial,
                "{source}"
            );
        }
    }

    #[test]
    fn readonly_member_census_stays_partial_for_other_class_families() {
        // A snit method dispatches through `$self`, which has no typed
        // self-dispatch tape. itcl is not covered: nothing establishes that
        // `itcl::class` publishes original method declarations, and a
        // control that checks no identity proves nothing.
        let source = "snit::type C {method ping {} {}; method body {} {$self ping}}";
        for dialect in ["tcl8.5", "tcl8.6"] {
            let analysis = Analyser::new().analyse(source, dialect);
            let mut checked = 0;
            for class in analysis.original_class_declarations() {
                for method in class.metadata().original_members.declarations() {
                    let Some(identity) = OriginalDeclarationIdentity::for_method(
                        "", source, &analysis, class, method,
                    ) else {
                        continue;
                    };
                    checked += 1;
                    assert_ne!(
                        readonly_reference_report(&identity, source, &analysis, true)
                            .map(|report| report.coverage()),
                        Some(ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus),
                        "{dialect}: {}",
                        identity.label()
                    );
                }
            }
            assert!(
                checked > 0,
                "{dialect}: no snit method identity was checked"
            );
        }
    }

    #[test]
    fn readonly_property_reports_promote_only_a_complete_typed_source_census() {
        let source = "oo::class create C {property size; method body {} {my size}}";
        let analysis = Analyser::new().analyse(source, "tcl9.0");
        let class = analysis.original_class_declarations().next().unwrap();
        let property = class
            .metadata()
            .original_properties
            .declarations()
            .find(|property| property.original_name_input().bytes() == b"size")
            .unwrap();
        let identity =
            OriginalDeclarationIdentity::for_property("", source, &analysis, class, property)
                .unwrap();
        let report = readonly_reference_report(&identity, source, &analysis, true).unwrap();
        assert_eq!(
            report.coverage(),
            ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus
        );
        assert_eq!(report.spans().len(), 1);
        assert_eq!(&source[report.spans()[0].as_range()], "size");
        let unused_source = "oo::class create C {property size; method body {} {return ok}}";
        let unused_analysis = Analyser::new().analyse(unused_source, "tcl9.0");
        let unused_class = unused_analysis
            .original_class_declarations()
            .next()
            .unwrap();
        let unused_property = unused_class
            .metadata()
            .original_properties
            .declarations()
            .find(|property| property.original_name_input().bytes() == b"size")
            .unwrap();
        let unused_identity = OriginalDeclarationIdentity::for_property(
            "",
            unused_source,
            &unused_analysis,
            unused_class,
            unused_property,
        )
        .unwrap();
        let unused_report =
            readonly_reference_report(&unused_identity, unused_source, &unused_analysis, true)
                .unwrap();
        assert_eq!(
            unused_report.coverage(),
            ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus
        );
        assert!(unused_report.spans().is_empty());
        let dynamic_source = "oo::class create C {property size; method body {} {my $selector}}";
        let dynamic_analysis = Analyser::new().analyse(dynamic_source, "tcl9.0");
        let dynamic_class = dynamic_analysis
            .original_class_declarations()
            .next()
            .unwrap();
        let dynamic_property = dynamic_class
            .metadata()
            .original_properties
            .declarations()
            .find(|property| property.original_name_input().bytes() == b"size")
            .unwrap();
        let dynamic_identity = OriginalDeclarationIdentity::for_property(
            "",
            dynamic_source,
            &dynamic_analysis,
            dynamic_class,
            dynamic_property,
        )
        .unwrap();
        let dynamic_report =
            readonly_reference_report(&dynamic_identity, dynamic_source, &dynamic_analysis, true)
                .unwrap();
        assert_eq!(
            dynamic_report.coverage(),
            ReadonlyReferenceCoverage::Partial
        );
        assert!(dynamic_report.spans().is_empty());
        let mut foreign = analysis.clone();
        foreign.resolved_input = Analyser::new()
            .analyse("puts foreign", "tcl9.0")
            .resolved_input;
        assert!(readonly_reference_report(&identity, source, &foreign, true).is_none());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_class_inventory_keeps_equal_source_owners_and_current_occupied_headers() {
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
            serde_json::from_value(serde_json::json!({"schema":1,"token":999_999})).unwrap();
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

#[cfg(test)]
mod logical_session_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn missing_random_allocation_withholds_logical_call_without_changing_legacy_wire() {
        let source = "proc helper {} {}\nhelper\n";
        let analysis = Analyser::new().analyse(source, "tcl");
        let document = OriginalDeclarationDocument {
            uri: "file:///session-failure.tcl",
            source,
            analysis: &analysis,
        };
        let advice =
            crate::call_hierarchy::prepare_logical_from_inventory(document.uri, 0, 5, &[document])
                .unwrap();
        let identity = advice.possible_source_items()[0].identity().clone();
        let publication = LogicalCallHostPublication::retain(());
        let mut registry = OriginalDeclarationRegistry {
            session: None,
            ..OriginalDeclarationRegistry::default()
        };
        assert!(
            registry
                .issue_logical_call(identity.clone(), publication.clone())
                .is_none()
        );
        assert!(registry.rows.is_empty());
        let legacy = registry
            .issue(
                OriginalDeclarationIdentity::for_document(document.uri, source, &analysis).unwrap(),
            )
            .unwrap();
        assert_eq!(
            serde_json::to_value(legacy).unwrap(),
            serde_json::json!({"schema":1,"token":1})
        );
        assert!(
            registry
                .resolve(legacy, document.uri, source, &analysis)
                .is_some()
        );
        // A genuine retained CALL from an entropy-available issuer cannot be
        // reborrowed once the current allocation owner is unavailable.
        let mut issuer = OriginalDeclarationRegistry::default();
        let handle = issuer
            .issue_logical_call(identity, publication.clone())
            .unwrap();
        assert!(
            issuer
                .resolve_logical_call(handle, document.uri, &[document], &publication)
                .is_some()
        );
        issuer.session = None;
        assert!(
            issuer
                .resolve_logical_call(handle, document.uri, &[document], &publication)
                .is_none()
        );
    }

    #[test]
    fn session_wire_bytes_are_lossless_and_legacy_shapes_stay_strict() {
        let nonce: EditorWireSession =
            serde_json::from_value(serde_json::json!(vec![255; 16])).unwrap();
        for schema in [3, 4] {
            let handle = OriginalDeclarationHandle {
                schema,
                token: 1,
                session: Some(nonce),
            };
            let value = serde_json::to_value(handle).unwrap();
            assert_eq!(value["session"], serde_json::json!(vec![255; 16]));
            assert_eq!(
                serde_json::from_value::<OriginalDeclarationHandle>(value).unwrap(),
                handle
            );
        }
        for schema in [1, 2] {
            let legacy = serde_json::json!({"schema":schema,"token":1});
            let handle: OriginalDeclarationHandle = serde_json::from_value(legacy.clone()).unwrap();
            assert_eq!(serde_json::to_value(handle).unwrap(), legacy);
            for session in [serde_json::json!(nonce), serde_json::Value::Null] {
                assert!(
                    serde_json::from_value::<OriginalDeclarationHandle>(
                        serde_json::json!({"schema":schema,"token":1,"session":session})
                    )
                    .is_err()
                );
            }
        }
        for session in [
            serde_json::json!([0]),
            serde_json::json!(vec![256; 16]),
            serde_json::Value::Null,
        ] {
            assert!(
                serde_json::from_value::<OriginalDeclarationHandle>(
                    serde_json::json!({"schema":3,"token":1,"session":session})
                )
                .is_err()
            );
        }
    }
}

#[cfg(test)]
mod relationship_transport_purpose_tests {
    use super::*;
    #[test]
    fn tagged_wrong_kind_decodes_without_becoming_legacy_authority() {
        let source = "proc p {} {}";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        let document = OriginalDeclarationDocument {
            uri: "file:///tagged-call.tcl",
            source,
            analysis: &analysis,
        };
        let identity =
            OriginalDeclarationIdentity::for_document(document.uri, source, &analysis).unwrap();
        let mut registry = OriginalDeclarationRegistry::default();
        let native = registry.issue(identity).unwrap();
        let nonce: EditorWireSession =
            serde_json::from_value(serde_json::json!(vec![255; 16])).unwrap();
        for schema in [1, 2, 4] {
            let wire = serde_json::json!({"schema":schema,"token":1,"session":nonce,"wirePurpose":"logicalSourceRelationship"});
            let inert: OriginalDeclarationHandle = serde_json::from_value(wire).unwrap();
            assert!(
                registry
                    .resolve(inert, document.uri, source, &analysis)
                    .is_none()
            );
            assert!(
                registry
                    .resolve_logical_class(inert, document.uri, source, &analysis)
                    .is_none()
            );
        }
        assert!(
            registry
                .resolve(native, document.uri, source, &analysis)
                .is_some()
        );
        assert_eq!(
            serde_json::to_value(native).unwrap(),
            serde_json::json!({"schema":1,"token":1})
        );
    }
}

#[cfg(test)]
mod property_source_census_unclassified_receiver_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn unclassified_object_receivers_and_unknown_heads_preserve_partial_counts() {
        for suffix in [
            "; set obj [C new]; $obj size",
            "; mystery",
            "; set selector size; my $selector",
        ] {
            let source = format!(
                "oo::class create C {{property size; method body {{}} {{my size}}}}{suffix}"
            );
            let analysis = Analyser::new().analyse(&source, "tcl9.0");
            let class = analysis.original_class_declarations().next().unwrap();
            let property = class
                .metadata()
                .original_properties
                .declarations()
                .find(|property| property.original_name_input().bytes() == b"size")
                .unwrap();
            let identity =
                OriginalDeclarationIdentity::for_property("", &source, &analysis, class, property)
                    .unwrap();
            let report = readonly_reference_report(&identity, &source, &analysis, true).unwrap();
            assert_eq!(
                report.coverage(),
                ReadonlyReferenceCoverage::Partial,
                "{source}"
            );
            assert!(
                !report.spans().is_empty(),
                "existing positive source selector remains"
            );
        }
    }
}

#[cfg(test)]
mod source_view_publication_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    // A storage-layer control, not a genuine server publication lease. The
    // protocol/host tests must retire actual physical-parent publications.
    #[derive(Debug, PartialEq, Eq)]
    struct TestView {
        parent: &'static str,
        epoch: u64,
    }

    #[test]
    fn equal_source_identity_cannot_renew_another_saved_view_stamp() {
        let uri = "file:///shared.tcl";
        let source = "proc run {} {}\n";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let identity = OriginalDeclarationIdentity::for_document(uri, source, &analysis).unwrap();
        let a = OriginalSourceViewPublication::retain(TestView {
            parent: "file:///A/root.tcl",
            epoch: 1,
        });
        let b = OriginalSourceViewPublication::retain(TestView {
            parent: "file:///B/root.tcl",
            epoch: 1,
        });
        let restored_a = OriginalSourceViewPublication::retain(TestView {
            parent: "file:///A/root.tcl",
            epoch: 3,
        });
        let mut registry = OriginalDeclarationRegistry::default();
        let old = registry
            .issue_in_source_view(identity.clone(), a.clone())
            .unwrap();
        let current = registry
            .issue_in_source_view(identity.clone(), b.clone())
            .unwrap();
        let restored = registry
            .issue_in_source_view(identity.clone(), restored_a.clone())
            .unwrap();
        assert_ne!(old, current);
        assert_ne!(old, restored);
        assert_eq!(
            registry.source_view_publication::<TestView>(old),
            Some(&TestView {
                parent: "file:///A/root.tcl",
                epoch: 1
            })
        );
        assert!(registry.source_view_publication::<String>(old).is_none());
        assert!(
            registry
                .resolve_in_source_view(old, uri, source, &analysis, &b)
                .is_none()
        );
        assert!(
            registry
                .resolve_in_source_view(old, uri, source, &analysis, &restored_a)
                .is_none()
        );
        assert_eq!(
            registry.resolve_in_source_view(current, uri, source, &analysis, &b),
            Some(identity.clone())
        );
        assert!(registry.resolve(old, uri, source, &analysis).is_none());
        let legacy = registry.issue(identity).unwrap();
        assert!(
            registry
                .source_view_publication::<TestView>(legacy)
                .is_none()
        );
        assert!(
            registry
                .resolve_in_source_view(legacy, uri, source, &analysis, &a)
                .is_none()
        );
        let wire = serde_json::to_value(old).unwrap();
        assert_eq!(wire.as_object().unwrap().len(), 4);
        assert_eq!(wire["schema"], 5);
        assert_eq!(wire["wirePurpose"], "originalSourceViewRelationship");
        assert_eq!(
            serde_json::from_value::<OriginalDeclarationHandle>(wire.clone()).unwrap(),
            old
        );
        assert_eq!(
            wire["session"],
            serde_json::to_value(old.session.unwrap()).unwrap()
        );
        for schema in [1, 2, 3, 4] {
            let wrong = OriginalDeclarationHandle { schema, ..old };
            assert!(
                registry
                    .source_view_publication::<TestView>(wrong)
                    .is_none()
            );
            assert!(
                registry
                    .resolve_in_source_view(wrong, uri, source, &analysis, &a)
                    .is_none()
            );
        }
    }

    #[test]
    fn source_view_wire_rejects_restarted_registry_and_missing_session() {
        // Real wire-session allocation protects counter collisions only; it
        // grants no source, parent-publication or declaration authority.
        let uri = "file:///shared.tcl";
        let source = "proc run {} {}\n";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let identity = OriginalDeclarationIdentity::for_document(uri, source, &analysis).unwrap();
        let stamp = OriginalSourceViewPublication::retain(TestView {
            parent: "file:///A/root.tcl",
            epoch: 1,
        });
        let mut first = OriginalDeclarationRegistry::default();
        let old = first
            .issue_in_source_view(identity.clone(), stamp.clone())
            .unwrap();
        let mut restarted = OriginalDeclarationRegistry::default();
        let new = restarted
            .issue_in_source_view(identity.clone(), stamp.clone())
            .unwrap();
        assert_eq!(
            old.token, new.token,
            "exercise an actual reset-counter collision"
        );
        assert_ne!(
            old.session, new.session,
            "independent genuine wire allocations"
        );
        assert!(restarted.source_view_publication::<TestView>(old).is_none());
        assert!(
            restarted
                .resolve_in_source_view(old, uri, source, &analysis, &stamp)
                .is_none()
        );
        assert!(
            restarted
                .resolve_in_source_view(new, uri, source, &analysis, &stamp)
                .is_some()
        );
        let missing = OriginalDeclarationHandle {
            session: None,
            ..old
        };
        assert!(first.source_view_publication::<TestView>(missing).is_none());
        assert!(
            first
                .resolve_in_source_view(missing, uri, source, &analysis, &stamp)
                .is_none()
        );
        let mut unavailable = OriginalDeclarationRegistry {
            session: None,
            next: 0,
            rows: BTreeMap::new(),
            order: VecDeque::new(),
        };
        assert!(
            unavailable
                .issue_in_source_view(identity.clone(), stamp)
                .is_none()
        );
        assert!(
            unavailable.issue(identity).is_some(),
            "legacy Native issuance stays independent"
        );
    }

    #[test]
    fn source_view_transport_keeps_its_exact_session_relationship_purpose() {
        let session = EditorWireSession::allocate().unwrap();
        let locator = OriginalDeclarationHandle {
            schema: 5,
            token: 1,
            session: Some(session),
        };
        let wire = serde_json::to_value(locator).unwrap();
        assert_eq!(wire.as_object().unwrap().len(), 4);
        assert_eq!(
            serde_json::from_value::<OriginalDeclarationHandle>(wire.clone()).unwrap(),
            locator
        );
        let mut changed = wire.clone();
        changed["wirePurpose"] = serde_json::json!("logicalSourceRelationship");
        assert!(serde_json::from_value::<OriginalDeclarationHandle>(changed).is_err());
        let mut changed = wire.clone();
        changed.as_object_mut().unwrap().remove("session");
        assert!(serde_json::from_value::<OriginalDeclarationHandle>(changed).is_err());
        let mut changed = wire.clone();
        changed.as_object_mut().unwrap().remove("wirePurpose");
        assert!(serde_json::from_value::<OriginalDeclarationHandle>(changed).is_err());
        for schema in [1, 2, 3, 4] {
            let mut changed = wire.clone();
            changed["schema"] = serde_json::json!(schema);
            assert!(serde_json::from_value::<OriginalDeclarationHandle>(changed).is_err());
        }
    }
}
