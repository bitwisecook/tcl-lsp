// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Sealed conditional call source selections and possible-positive publication.

use super::{CallHierarchyItem, LogicalMethodProcedureSelection, logical_method_procedure_at};
use crate::definition::{LspRange, span_to_range};
use crate::original_declaration::OriginalDeclarationDocument;
use tcl_compiler::analyser::types::{
    LogicalSourceMemberDeclaration, LogicalSourceProcedureDeclaration,
};
use tcl_compiler::analyser::{AnalysisResult, MemberSide, ResolvedAnalysisInput};
use tcl_compiler::command_binding::{
    OriginalDeclaredSourceBodyFrame, OriginalLogicalAbsoluteCommandCandidate,
    OriginalLogicalCommandCandidates,
};
use tcl_lexer::{LineIndex, NativeWord, SourceImage, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Selection {
    Document(Box<OriginalLogicalCommandCandidates>),
    Procedure(Box<LogicalSourceProcedureDeclaration>),
    Member(Box<LogicalSourceMemberDeclaration>),
}

/// Current source selection for CALL hierarchy, independent of Native and TYPE
/// identities. Display fields and client ranges cannot construct this receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalCallIdentity {
    uri: String,
    image: SourceImage,
    input: ResolvedAnalysisInput,
    selected: Selection,
    provenance: Option<Box<SelectionLease>>,
    incomplete: bool,
}
impl LogicalCallIdentity {
    fn capture(document: OriginalDeclarationDocument<'_>, selected: Selection) -> Option<Self> {
        crate::original_context::CurrentSourceContext::capture(document.source, document.analysis)?;
        document
            .analysis
            .allows_retained_logical_declaration_advice()
            .then_some(())?;
        let identity = Self {
            uri: document.uri.to_owned(),
            image: SourceImage::document(document.source),
            input: document.analysis.resolved_input.as_ref()?.clone(),
            selected,
            provenance: None,
            incomplete: false,
        };
        identity
            .is_current(document.uri, document.source, document.analysis)
            .then_some(identity)
    }
    /// Real independent document owner; no rendered-name URI inverse is used.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }
    /// Retained unavailable alternatives or earlier source effects remain debt.
    #[must_use]
    pub const fn has_unresolved_applicability(&self) -> bool {
        self.incomplete
    }
    /// Reborrow the actual original header/member or genuine Document recipe
    /// under the complete current source/input/configuration/Registry/Realm.
    #[must_use]
    pub fn is_current(&self, uri: &str, source: &str, analysis: &AnalysisResult) -> bool {
        if uri != self.uri
            || self.image != SourceImage::document(source)
            || analysis.resolved_input.as_ref() != Some(&self.input)
            || crate::original_context::CurrentSourceContext::capture(source, analysis).is_none()
            || !analysis.allows_retained_logical_declaration_advice()
        {
            return false;
        }
        match &self.selected {
            Selection::Document(request) => request.original_document_source_origin().is_some()
                && request.original_head().is_some_and(|head| {
                    tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates_at(source, analysis, head.span().start()).as_ref() == Some(request.as_ref())
                }),
            Selection::Procedure(header) => {
                let mut current = analysis.logical_source_procedure_declarations().filter(|candidate| *candidate == header.as_ref());
                current.next().is_some_and(|candidate| {
                    candidate.original_name_word().is_some()
                        && candidate.original_body_word(&self.image, &self.input).is_some()
                        && candidate.source_candidate_coordinate().is_some()
                        && analysis.logical_source_procedure_declaration(candidate.metadata()) == Some(candidate)
                }) && current.next().is_none()
            }
            Selection::Member(member) => member.original_name_word()
                .and_then(tcl_syntax::word_rules::original_static_word_unicode_value).is_some()
                && member.original_body_word().is_some()
                && analysis.logical_source_member_class(member).is_some(),
        }
    }
    /// Whole wire currentness also reissues the actual selecting occurrence
    /// against its independent consuming source. A declaring file alone cannot
    /// renew a cross-file call selection.
    #[must_use]
    pub fn is_current_in(&self, documents: &[OriginalDeclarationDocument<'_>]) -> bool {
        let Some(inventory) = Inventory::capture(documents) else {
            return false;
        };
        let Some(owner) = inventory.owner(self) else {
            return false;
        };
        if !self.is_current(owner.uri, owner.source, owner.analysis) {
            return false;
        }
        let Some(provenance) = &self.provenance else {
            return true;
        };
        let Some(consumer) = documents
            .iter()
            .copied()
            .find(|document| document.uri == provenance.consumer.uri)
        else {
            return false;
        };
        provenance.consumer.image == SourceImage::document(consumer.source)
            && consumer.analysis.resolved_input.as_ref() == Some(&provenance.consumer.input)
            && if provenance.target {
                inventory.reissues_target(consumer, &provenance.evidence, self)
            } else {
                inventory.reissues_caller(consumer, &provenance.evidence, self)
            }
    }
    fn at_occurrence(
        mut self,
        document: OriginalDeclarationDocument<'_>,
        evidence: &Evidence,
        target: bool,
    ) -> Option<Self> {
        self.provenance = Some(Box::new(SelectionLease {
            consumer: Snapshot {
                uri: document.uri.to_owned(),
                image: SourceImage::document(document.source),
                input: document.analysis.resolved_input.as_ref()?.clone(),
            },
            evidence: evidence.clone(),
            target,
        }));
        Some(self)
    }
    fn same_declaration(&self, other: &Self) -> bool {
        if self.uri != other.uri || self.image != other.image || self.input != other.input {
            return false;
        }
        match (&self.selected, &other.selected) {
            (Selection::Document(a), Selection::Document(b)) => {
                a.original_document_source_origin().is_some()
                    && a.original_document_source_origin() == b.original_document_source_origin()
            }
            (a, b) => a == b,
        }
    }
    fn item(&self, document: OriginalDeclarationDocument<'_>) -> Option<LogicalCallHierarchyItem> {
        self.is_current(document.uri, document.source, document.analysis)
            .then_some(())?;
        let (name, name_span, body, detail) = match &self.selected {
            Selection::Document(_) => (
                "<top-level>".to_owned(),
                Span::new(0, 0),
                Span::new(0, u32::try_from(document.source.len()).ok()?),
                Some("Document source stream".to_owned()),
            ),
            Selection::Procedure(header) => (
                tcl_syntax::naming::authored_source_command_spelling(
                    &header.source_candidate_coordinate()?,
                )?,
                header.original_name_word()?.span(),
                header.original_body_word(&self.image, &self.input)?.span(),
                Some(format!("({} params)", header.metadata().params.len())),
            ),
            Selection::Member(member) => {
                let name = tcl_syntax::word_rules::original_static_word_unicode_value(
                    member.original_name_word()?,
                )?;
                let name = String::from_utf8(name).ok()?;
                let owner = tcl_syntax::naming::authored_source_command_spelling(
                    member.class().source_candidate_coordinate()?,
                )?;
                (
                    format!("{owner}::{name}"),
                    member.original_name_word()?.span(),
                    member.original_body_word()?.span(),
                    Some(format!("{:?} source member", member.side())),
                )
            }
        };
        let lines = LineIndex::new_lsp(document.source);
        Some(LogicalCallHierarchyItem {
            identity: self.clone(),
            item: CallHierarchyItem {
                identity: None,
                name,
                detail,
                range: span_to_range(
                    document.source,
                    &lines,
                    Span::new(name_span.start(), body.end().max(name_span.end())),
                ),
                selection_range: span_to_range(document.source, &lines, name_span),
            },
        })
    }
}

/// Display projection paired with the authentic current CALL selection.
#[derive(Debug, Clone)]
pub struct LogicalCallHierarchyItem {
    identity: LogicalCallIdentity,
    item: CallHierarchyItem,
}
impl LogicalCallHierarchyItem {
    /// Sealed source identity, separate from all presentation fields.
    #[must_use]
    pub const fn identity(&self) -> &LogicalCallIdentity {
        &self.identity
    }
    /// Presentation only, after original source selection.
    #[must_use]
    pub const fn presentation(&self) -> &CallHierarchyItem {
        &self.item
    }
}

/// Possible-positive caller/callee group; ranges belong to the consuming file.
#[derive(Debug, Clone)]
pub struct LogicalCallHierarchyEdge {
    item: LogicalCallHierarchyItem,
    ranges: Vec<LspRange>,
}
impl LogicalCallHierarchyEdge {
    /// Independently current other end of this source edge.
    #[must_use]
    pub const fn item(&self) -> &LogicalCallHierarchyItem {
        &self.item
    }
    /// Original source occurrences, without runtime call/dispatch authority.
    #[must_use]
    pub fn possible_source_ranges(&self) -> &[LspRange] {
        &self.ranges
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Snapshot {
    uri: String,
    image: SourceImage,
    input: ResolvedAnalysisInput,
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum Evidence {
    Procedure {
        retained: Option<Box<tcl_compiler::command_binding::OriginalLogicalProcedureCall>>,
        ordinary: Option<Box<OriginalLogicalCommandCandidates>>,
        absolute: Option<Box<OriginalLogicalAbsoluteCommandCandidate>>,
        head: NativeWord,
    },
    SelfMember {
        selector: NativeWord,
        member: Box<LogicalSourceMemberDeclaration>,
    },
    ClassMember {
        head: NativeWord,
        selector: NativeWord,
        member: Box<LogicalSourceMemberDeclaration>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct SelectionLease {
    consumer: Snapshot,
    evidence: Evidence,
    target: bool,
}
#[derive(Debug, Clone)]
struct Occurrence {
    caller: LogicalCallIdentity,
    target: LogicalCallIdentity,
    span: Span,
    evidence: Evidence,
}
impl Evidence {
    fn is_current(&self, consumer: OriginalDeclarationDocument<'_>) -> bool {
        match self {
            Self::Procedure {
                retained,
                ordinary,
                absolute,
                head,
            } => {
                if let Some(retained) = retained {
                    return retained.original_words().first() == Some(head)
                        && tcl_compiler::registry_invocation::source_structure::source_logical_procedure_call_at(
                            consumer.source, consumer.analysis, head.span().start()
                        ).as_ref() == Some(retained.as_ref())
                        && retained.declaration(consumer.analysis).is_some();
                }
                let current = tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates_at(consumer.source, consumer.analysis, head.span().start());
                let current_absolute = if current.is_none() {
                    tcl_compiler::registry_invocation::source_structure::source_logical_absolute_command_candidate_at(consumer.source, consumer.analysis, head.span().start())
                } else {
                    None
                };
                (ordinary.is_some() || absolute.is_some())
                    && ordinary.as_deref() == current.as_ref()
                    && absolute.as_deref() == current_absolute.as_ref()
            }
            Self::SelfMember { selector, member } => {
                consumer.analysis.body_lexer_config.is_some_and(|config| {
                    consumer
                        .analysis
                        .logical_source_member_at(
                            &SourceImage::document(consumer.source),
                            config,
                            selector.span().start(),
                        )
                        .as_ref()
                        == Some(member.as_ref())
                })
            }
            Self::ClassMember {
                head,
                selector,
                member,
            } => class_member(consumer, head.span().start())
                .as_ref()
                .is_some_and(|(current, word)| current == member.as_ref() && word == selector),
        }
    }
    fn contains(&self, cursor: u32) -> bool {
        let word = match self {
            Self::Procedure { head, .. } => head,
            Self::SelfMember { selector, .. } | Self::ClassMember { selector, .. } => selector,
        };
        word.span().as_range().contains(&(cursor as usize))
    }
}
impl Occurrence {
    fn is_current(&self, documents: &[OriginalDeclarationDocument<'_>]) -> bool {
        let Some(consumer) = documents
            .iter()
            .copied()
            .find(|document| document.uri == self.caller.uri)
        else {
            return false;
        };
        let Some(provider) = documents
            .iter()
            .find(|document| document.uri == self.target.uri)
        else {
            return false;
        };
        self.caller
            .is_current(consumer.uri, consumer.source, consumer.analysis)
            && self
                .target
                .is_current(provider.uri, provider.source, provider.analysis)
            && self.evidence.is_current(consumer)
    }
}
#[derive(Debug, Clone)]
struct Preparation {
    uri: String,
    evidence: Evidence,
    targets: Vec<LogicalCallIdentity>,
}

/// Typed conditional CALL result. Unknown/unrepresented alternatives survive
/// until the actual host's final readonly source/provider publication lease.
#[derive(Debug, Clone)]
pub struct LogicalCallAdvice {
    snapshots: Vec<Snapshot>,
    root: Option<LogicalCallIdentity>,
    items: Vec<LogicalCallHierarchyItem>,
    edges: Vec<LogicalCallHierarchyEdge>,
    occurrences: Vec<Occurrence>,
    preparations: Vec<Preparation>,
    incomplete: bool,
}
impl LogicalCallAdvice {
    /// Retained source applicability, never complete runtime call-graph closure.
    #[must_use]
    pub const fn has_unresolved_applicability(&self) -> bool {
        self.incomplete
    }
    /// Revalidate every participant and original positive occurrence.
    #[must_use]
    pub fn is_current(&self, documents: &[OriginalDeclarationDocument<'_>]) -> bool {
        let Some(inventory) = Inventory::capture(documents) else {
            return false;
        };
        self.snapshots.len() == documents.len()
            && self.snapshots.iter().all(|snapshot| {
                documents
                    .iter()
                    .find(|document| document.uri == snapshot.uri)
                    .is_some_and(|document| {
                        snapshot.image == SourceImage::document(document.source)
                            && document.analysis.resolved_input.as_ref() == Some(&snapshot.input)
                            && crate::original_context::CurrentSourceContext::capture(
                                document.source,
                                document.analysis,
                            )
                            .is_some()
                    })
            })
            && self.root.as_ref().is_none_or(|identity| {
                documents.iter().any(|document| {
                    identity.is_current(document.uri, document.source, document.analysis)
                }) && identity.is_current_in(documents)
            })
            && self
                .items
                .iter()
                .chain(self.edges.iter().map(|edge| &edge.item))
                .all(|item| {
                    documents.iter().any(|document| {
                        item.identity
                            .is_current(document.uri, document.source, document.analysis)
                    }) && item.identity.is_current_in(documents)
                })
            && self.occurrences.iter().all(|occurrence| {
                occurrence.is_current(documents) && inventory.reissues_occurrence(occurrence)
            })
            && self.preparations.iter().all(|preparation| {
                documents
                    .iter()
                    .copied()
                    .find(|document| document.uri == preparation.uri)
                    .is_some_and(|document| {
                        preparation.evidence.is_current(document)
                            && preparation.targets.iter().all(|target| {
                                inventory.reissues_target(document, &preparation.evidence, target)
                            })
                    })
            })
    }
    // Reference-purpose projection of the retained possible-call subset. Each
    // extent is the actual head/selector word, not a reported hierarchy range.
    pub(crate) fn possible_reference_name_targets(&self) -> Option<Vec<(String, Span)>> {
        let root = self.root.as_ref()?;
        Some(
            self.occurrences
                .iter()
                .filter(|occurrence| occurrence.target.same_declaration(root))
                .map(|occurrence| {
                    let span = match &occurrence.evidence {
                        Evidence::Procedure { head, .. } => head.span(),
                        Evidence::SelfMember { selector, .. }
                        | Evidence::ClassMember { selector, .. } => selector.span(),
                    };
                    (occurrence.caller.uri().to_owned(), span)
                })
                .collect(),
        )
    }
    /// Possible-positive item projection, only within the host's final lease.
    #[must_use]
    pub fn possible_source_items(&self) -> &[LogicalCallHierarchyItem] {
        &self.items
    }
    /// Possible-positive edge projection, only within the host's final lease.
    #[must_use]
    pub fn possible_source_edges(&self) -> &[LogicalCallHierarchyEdge] {
        &self.edges
    }
}

struct Inventory<'a> {
    documents: Vec<OriginalDeclarationDocument<'a>>,
    headers: Vec<(LogicalCallIdentity, OriginalDeclarationDocument<'a>)>,
    snapshots: Vec<Snapshot>,
    index: crate::workspace_index::WorkspaceIndex,
    incomplete: bool,
}
impl<'a> Inventory<'a> {
    fn capture(documents: &[OriginalDeclarationDocument<'a>]) -> Option<Self> {
        let mut inventory = Self {
            documents: documents.to_vec(),
            headers: Vec::new(),
            snapshots: Vec::new(),
            index: crate::workspace_index::WorkspaceIndex::default(),
            incomplete: false,
        };
        let mut uris = std::collections::HashSet::new();
        for &document in documents {
            crate::original_context::CurrentSourceContext::capture(
                document.source,
                document.analysis,
            )?;
            uris.insert(document.uri).then_some(())?;
            inventory.snapshots.push(Snapshot {
                uri: document.uri.to_owned(),
                image: SourceImage::document(document.source),
                input: document.analysis.resolved_input.as_ref()?.clone(),
            });
            if !document
                .analysis
                .allows_retained_logical_declaration_advice()
            {
                inventory.incomplete = true;
                continue;
            }
            let image = SourceImage::document(document.source);
            inventory.incomplete |= document
                .analysis
                .checked_logical_source_procedure_declarations(
                    &image,
                    document.analysis.body_lexer_config?,
                )
                .is_none();
            for header in document.analysis.logical_source_procedure_declarations() {
                if let Some(identity) = LogicalCallIdentity::capture(
                    document,
                    Selection::Procedure(Box::new(header.clone())),
                ) {
                    inventory.headers.push((identity, document));
                } else {
                    inventory.incomplete = true;
                }
            }
            inventory
                .index
                .add_document(document.uri, document.analysis);
        }
        Some(inventory)
    }
    fn owner(&self, identity: &LogicalCallIdentity) -> Option<OriginalDeclarationDocument<'a>> {
        self.documents
            .iter()
            .copied()
            .find(|document| document.uri == identity.uri)
    }
    fn caller(
        &self,
        document: OriginalDeclarationDocument<'a>,
        offset: u32,
        request: Option<&OriginalLogicalCommandCandidates>,
    ) -> Option<LogicalCallIdentity> {
        let image = SourceImage::document(document.source);
        if let Some(member) = document.analysis.logical_source_member_frame_at(
            &image,
            document.analysis.body_lexer_config?,
            offset,
        ) {
            return LogicalCallIdentity::capture(document, Selection::Member(Box::new(member)));
        }
        // Reborrow the actual member catalogue's full original declaration
        // vector; source body membership is independent of receiver-frame entry.
        let member_body = tcl_compiler::registry_invocation::source_structure::source_registry_words_at(
            document.source, document.analysis, offset,
        ).and_then(|words| {
            match words.source() {
                tcl_compiler::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(advice) =>
                    advice.original_definition_member_source_body().cloned(),
                _ => None,
            }
        }).or_else(|| {
            tcl_compiler::registry_invocation::source_structure::source_logical_absolute_command_candidate_at(
                document.source, document.analysis, offset,
            ).and_then(|candidate| candidate.original_member_source_body().cloned())
        });
        if let Some(body) = member_body
            && let Some(member) = document
                .analysis
                .logical_source_member_for_original_body(&body)
        {
            return LogicalCallIdentity::capture(document, Selection::Member(Box::new(member)));
        }
        let request = request?;
        if request.original_document_source_origin().is_some() {
            return LogicalCallIdentity::capture(
                document,
                Selection::Document(Box::new(request.clone())),
            );
        }
        let mut scope = request.declared_source_context()?;
        loop {
            match scope.frame_recipe() {
                OriginalDeclaredSourceBodyFrame::Procedure => {
                    let body = scope.scope_body().original_container();
                    let mut matching = self.headers.iter().filter(|(identity, owner)| {
                        owner.uri == document.uri && matches!(&identity.selected, Selection::Procedure(header) if header.original_body_word(&image, &identity.input) == Some(body))
                    });
                    let (identity, _) = matching.next()?;
                    return matching.next().is_none().then(|| identity.clone());
                }
                OriginalDeclaredSourceBodyFrame::Lambda => return None,
                OriginalDeclaredSourceBodyFrame::Namespace
                | OriginalDeclaredSourceBodyFrame::Global => {
                    scope = scope.parent_source_context()?
                }
            }
        }
    }
    fn procedure_targets(
        &self,
        document: OriginalDeclarationDocument<'a>,
        offset: u32,
    ) -> (Vec<LogicalCallIdentity>, bool) {
        if let Some(call) =
            tcl_compiler::registry_invocation::source_structure::source_logical_procedure_call_at(
                document.source,
                document.analysis,
                offset,
            )
        {
            let selected = call.declaration(document.analysis).and_then(|header| {
                LogicalCallIdentity::capture(
                    document,
                    Selection::Procedure(Box::new(header.clone())),
                )
            });
            let incomplete = selected.is_none() || call.has_unresolved_applicability();
            return (selected.into_iter().collect(), incomplete);
        }
        match logical_method_procedure_at(document.source, document.analysis, offset) {
            LogicalMethodProcedureSelection::Occupied {
                procedure,
                earlier_unknown_mutations,
            } => {
                let selected = procedure
                    .and_then(|procedure| {
                        document
                            .analysis
                            .logical_source_procedure_declaration(procedure)
                    })
                    .and_then(|header| {
                        LogicalCallIdentity::capture(
                            document,
                            Selection::Procedure(Box::new(header.clone())),
                        )
                    });
                let missing = selected.is_none();
                (
                    selected.into_iter().collect(),
                    missing || earlier_unknown_mutations,
                )
            }
            LogicalMethodProcedureSelection::Vacant {
                candidates: Some(request),
            } => {
                let candidates = self
                    .index
                    .logical_command_candidates(&request, document.uri);
                let mut incomplete =
                    candidates.len() != 1 || request.has_earlier_unknown_mutations();
                let mut selected = Vec::new();
                for candidate in candidates {
                    if !self.index.logical_command_candidate_is_current(&candidate) {
                        incomplete = true;
                        continue;
                    }
                    let Some(header) = candidate.procedure_declaration() else {
                        incomplete = true;
                        continue;
                    };
                    let Some(owner) = self
                        .documents
                        .iter()
                        .copied()
                        .find(|owner| owner.uri == candidate.uri())
                    else {
                        incomplete = true;
                        continue;
                    };
                    if let Some(identity) = LogicalCallIdentity::capture(
                        owner,
                        Selection::Procedure(Box::new(header.clone())),
                    ) {
                        selected.push(identity);
                    } else {
                        incomplete = true;
                    }
                }
                (selected, incomplete)
            }
            LogicalMethodProcedureSelection::Unavailable
            | LogicalMethodProcedureSelection::Vacant { candidates: None } => (Vec::new(), true),
        }
    }
    fn targets_for_words(
        &self,
        document: OriginalDeclarationDocument<'a>,
        words: &[NativeWord],
        command_span: Span,
    ) -> Option<(Vec<LogicalCallIdentity>, Evidence, Span, bool)> {
        let head = words.first()?;
        let ordinary = tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates_at(document.source, document.analysis, head.span().start());
        let absolute = if ordinary.is_none() {
            tcl_compiler::registry_invocation::source_structure::source_logical_absolute_command_candidate_at(document.source, document.analysis, head.span().start())
        } else {
            None
        };
        let member = words.get(1).and_then(|selector| {
            document
                .analysis
                .logical_source_member_at(
                    &SourceImage::document(document.source),
                    document.analysis.body_lexer_config?,
                    selector.span().start(),
                )
                .filter(|member| {
                    member
                        .original_name_word()
                        .is_some_and(|name| name.span() != selector.span())
                })
                .map(|member| (member, selector.clone()))
        });
        if let Some((member, selector)) = member {
            let target =
                LogicalCallIdentity::capture(document, Selection::Member(Box::new(member.clone())));
            // Own static selectors remain positive; inherited/provider closure
            // and activation are independently unrepresented applicability.
            return Some((
                target.into_iter().collect(),
                Evidence::SelfMember {
                    selector: selector.clone(),
                    member: Box::new(member),
                },
                selector.span(),
                true,
            ));
        }
        if let Some((member, selector)) = class_member(document, head.span().start()) {
            let target =
                LogicalCallIdentity::capture(document, Selection::Member(Box::new(member.clone())));
            return Some((
                target.into_iter().collect(),
                Evidence::ClassMember {
                    head: head.clone(),
                    selector: selector.clone(),
                    member: Box::new(member),
                },
                selector.span(),
                true,
            ));
        }
        let (targets, debt) = self.procedure_targets(document, head.span().start());
        Some((
            targets,
            Evidence::Procedure {
                retained: tcl_compiler::registry_invocation::source_structure::source_logical_procedure_call_at(
                    document.source, document.analysis, head.span().start(),
                ).map(Box::new),
                ordinary: ordinary.map(Box::new),
                absolute: absolute.map(Box::new),
                head: head.clone(),
            },
            command_span,
            debt,
        ))
    }

    fn reissues_target(
        &self,
        document: OriginalDeclarationDocument<'a>,
        evidence: &Evidence,
        target: &LogicalCallIdentity,
    ) -> bool {
        if !evidence.is_current(document) {
            return false;
        }
        match evidence {
            Evidence::Procedure { head, .. } => self
                .procedure_targets(document, head.span().start())
                .0
                .iter()
                .any(|current| current.same_declaration(target)),
            Evidence::SelfMember { member, .. } | Evidence::ClassMember { member, .. } => {
                LogicalCallIdentity::capture(document, Selection::Member(member.clone()))
                    .is_some_and(|current| current.same_declaration(target))
            }
        }
    }
    fn reissues_caller(
        &self,
        document: OriginalDeclarationDocument<'a>,
        evidence: &Evidence,
        caller: &LogicalCallIdentity,
    ) -> bool {
        if !evidence.is_current(document) {
            return false;
        }
        let head = match evidence {
            Evidence::Procedure { head, .. } | Evidence::ClassMember { head, .. } => head,
            Evidence::SelfMember { selector, .. } => {
                return self
                    .caller(document, selector.span().start(), None)
                    .is_some_and(|current| current.same_declaration(caller));
            }
        };
        let ordinary = tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates_at(document.source, document.analysis, head.span().start());
        self.caller(document, head.span().start(), ordinary.as_ref())
            .is_some_and(|current| current.same_declaration(caller))
    }
    fn reissues_occurrence(&self, occurrence: &Occurrence) -> bool {
        let Some(document) = self.owner(&occurrence.caller) else {
            return false;
        };
        self.reissues_caller(document, &occurrence.evidence, &occurrence.caller)
            && self.reissues_target(document, &occurrence.evidence, &occurrence.target)
    }

    fn occurrences(&mut self) -> Option<Vec<Occurrence>> {
        let mut occurrences = Vec::new();
        for document in self.documents.clone() {
            if !document
                .analysis
                .allows_retained_logical_declaration_advice()
            {
                continue;
            }
            let coverage = crate::source_structure::SourceSyntaxCoverage::capture(
                document.source,
                document.analysis,
            )?;
            self.incomplete |= coverage.applicability(document.source, document.analysis)?
                == crate::source_structure::SourceSyntaxCoverageApplicability::Incomplete;
            let walk = crate::refactor::FrameWalk::new(document.source, document.analysis)?;
            let mut seen = std::collections::HashSet::new();
            for command in coverage.commands(document.source, document.analysis)? {
                let Some(words) =
                    crate::refactor::FrameWalk::native_words(&walk, document.source, command)
                else {
                    self.incomplete = true;
                    continue;
                };
                let Some(head) = words.first() else {
                    self.incomplete = true;
                    continue;
                };
                if !seen.insert((command.span.start(), command.span.end())) {
                    continue;
                }
                let Some((targets, evidence, span, debt)) =
                    self.targets_for_words(document, &words, command.span)
                else {
                    self.incomplete = true;
                    continue;
                };
                self.incomplete |= debt;
                if targets.is_empty() {
                    continue;
                }
                let ordinary = tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates_at(document.source, document.analysis, head.span().start());
                let Some(caller) = self.caller(document, head.span().start(), ordinary.as_ref())
                else {
                    self.incomplete = true;
                    continue;
                };
                for target in targets {
                    occurrences.push(Occurrence {
                        caller: caller.clone().at_occurrence(document, &evidence, false)?,
                        target: target.at_occurrence(document, &evidence, true)?,
                        span,
                        evidence: evidence.clone(),
                    });
                }
            }
        }
        Some(occurrences)
    }
    fn reply(
        self,
        root: Option<LogicalCallIdentity>,
        mut items: Vec<LogicalCallHierarchyItem>,
        mut edges: Vec<LogicalCallHierarchyEdge>,
        occurrences: Vec<Occurrence>,
        preparations: Vec<Preparation>,
    ) -> LogicalCallAdvice {
        for item in items
            .iter_mut()
            .chain(edges.iter_mut().map(|edge| &mut edge.item))
        {
            item.identity.incomplete |= self.incomplete;
        }
        LogicalCallAdvice {
            snapshots: self.snapshots,
            root,
            items,
            edges,
            occurrences,
            preparations,
            incomplete: self.incomplete,
        }
    }
}

fn class_member(
    document: OriginalDeclarationDocument<'_>,
    offset: u32,
) -> Option<(LogicalSourceMemberDeclaration, NativeWord)> {
    let call = tcl_compiler::registry_invocation::source_structure::source_constructor_call_at(
        document.source,
        document.analysis,
        offset,
    )?;
    let class = call
        .class_declaration()
        .logical_source_class(document.analysis)?;
    let owner = document.analysis.logical_source_class_declaration(class)?;
    let selector = call.argument_word(0)?;
    let ordinal = call.written_argument(0)?;
    if call.original_words().get(ordinal + 1) != Some(selector) {
        return None;
    }
    let value = tcl_syntax::word_rules::original_static_word_unicode_value(selector)?;
    let mut selected = class
        .class_methods
        .values()
        .filter_map(|method| owner.member(MemberSide::ClassObject, method))
        .filter(|member| {
            member.metadata().visibility == "public"
                && member
                    .original_name_word()
                    .and_then(tcl_syntax::word_rules::original_static_word_unicode_value)
                    .as_ref()
                    == Some(&value)
        });
    let first = selected.next()?;
    first.original_body_word()?;
    selected.next().is_none().then(|| (first, selector.clone()))
}

// Shared declaration-only issuance for readonly lens consumers. This does not
// prepare a call, infer a cursor, or close a source-reference inventory.
pub(crate) fn declared_logical_source_items(
    selected_uri: &str,
    documents: &[OriginalDeclarationDocument<'_>],
) -> Option<Vec<LogicalCallHierarchyItem>> {
    let inventory = Inventory::capture(documents)?;
    let document = documents
        .iter()
        .copied()
        .find(|row| row.uri == selected_uri)?;
    let mut items = inventory
        .headers
        .iter()
        .filter(|(_, owner)| owner.uri == selected_uri)
        .map(|(identity, owner)| identity.item(*owner))
        .collect::<Option<Vec<_>>>()?;
    for class in document.analysis.all_classes.values() {
        let Some(owner) = document.analysis.logical_source_class_declaration(class) else {
            continue;
        };
        for (side, methods) in [
            (MemberSide::Instance, &class.methods),
            (MemberSide::ClassObject, &class.class_methods),
        ] {
            for method in methods.values() {
                let Some(member) = owner.member(side, method) else {
                    continue;
                };
                let Some(identity) =
                    LogicalCallIdentity::capture(document, Selection::Member(Box::new(member)))
                else {
                    continue;
                };
                if let Some(item) = identity.item(document) {
                    items.push(item);
                }
            }
        }
    }
    Some(items)
}

/// Genuine Logical CALL preparation, independent of report labels/ranges.
#[must_use]
pub fn prepare_logical_from_inventory(
    uri: &str,
    line: u32,
    character: u32,
    documents: &[OriginalDeclarationDocument<'_>],
) -> Option<LogicalCallAdvice> {
    let mut inventory = Inventory::capture(documents)?;
    let document = inventory
        .documents
        .iter()
        .copied()
        .find(|document| document.uri == uri)?;
    let position = crate::original_context::CurrentSourcePosition::capture(
        document.source,
        document.analysis,
        line,
        character,
    )?;
    document
        .analysis
        .allows_retained_logical_declaration_advice()
        .then_some(())?;
    let contains = |word: &NativeWord| {
        word.span()
            .as_range()
            .contains(&(position.offset() as usize))
    };
    let mut selected = inventory.headers.iter().filter(|(identity, owner)| owner.uri == uri && matches!(&identity.selected, Selection::Procedure(header) if header.original_name_word().is_some_and(contains))).map(|(identity, _)| identity.clone()).collect::<Vec<_>>();
    if let Some(member) = document.analysis.logical_source_member_at(
        &SourceImage::document(document.source),
        document.analysis.body_lexer_config?,
        position.offset(),
    ) {
        if let Some(identity) =
            LogicalCallIdentity::capture(document, Selection::Member(Box::new(member)))
        {
            selected.push(identity);
        }
    }
    let mut preparations = Vec::new();
    if selected.is_empty() {
        let coverage = crate::source_structure::SourceSyntaxCoverage::capture(
            document.source,
            document.analysis,
        )?;
        inventory.incomplete |= coverage.applicability(document.source, document.analysis)?
            == crate::source_structure::SourceSyntaxCoverageApplicability::Incomplete;
        let walk = crate::refactor::FrameWalk::new(document.source, document.analysis)?;
        for command in coverage.commands(document.source, document.analysis)? {
            let Some(words) =
                crate::refactor::FrameWalk::native_words(&walk, document.source, command)
            else {
                inventory.incomplete = true;
                continue;
            };
            let Some((targets, evidence, _, debt)) =
                inventory.targets_for_words(document, &words, command.span)
            else {
                inventory.incomplete = true;
                continue;
            };
            if !evidence.contains(position.offset()) {
                continue;
            }
            inventory.incomplete |= debt;
            let retained_targets = targets.clone();
            for target in targets {
                if !selected.iter().any(|prior| prior.same_declaration(&target)) {
                    selected.push(target.at_occurrence(document, &evidence, true)?);
                }
            }
            preparations.push(Preparation {
                uri: document.uri.to_owned(),
                evidence,
                targets: retained_targets,
            });
        }
    }
    inventory.incomplete |= selected.len() != 1;
    let items = selected
        .iter()
        .map(|identity| identity.item(inventory.owner(identity)?))
        .collect::<Option<Vec<_>>>()?;
    Some(inventory.reply(None, items, Vec::new(), Vec::new(), preparations))
}

fn edges(
    identity: &LogicalCallIdentity,
    documents: &[OriginalDeclarationDocument<'_>],
    incoming: bool,
) -> Option<LogicalCallAdvice> {
    let mut inventory = Inventory::capture(documents)?;
    let owner = inventory.owner(identity)?;
    identity
        .is_current(owner.uri, owner.source, owner.analysis)
        .then_some(())?;
    identity.is_current_in(documents).then_some(())?;
    inventory.incomplete |= identity.has_unresolved_applicability();
    let occurrences = inventory.occurrences()?;
    let mut edges: Vec<LogicalCallHierarchyEdge> = Vec::new();
    for occurrence in &occurrences {
        let (selected, other) = if incoming {
            (&occurrence.target, &occurrence.caller)
        } else {
            (&occurrence.caller, &occurrence.target)
        };
        if !selected.same_declaration(identity) {
            continue;
        }
        let consumer = inventory.owner(&occurrence.caller)?;
        let range = span_to_range(
            consumer.source,
            &LineIndex::new_lsp(consumer.source),
            occurrence.span,
        );
        if let Some(edge) = edges
            .iter_mut()
            .find(|edge| edge.item.identity.same_declaration(other))
        {
            edge.ranges.push(range);
        } else {
            edges.push(LogicalCallHierarchyEdge {
                item: other.item(inventory.owner(other)?)?,
                ranges: vec![range],
            });
        }
    }
    edges.sort_by(|a, b| {
        a.item
            .identity
            .uri
            .cmp(&b.item.identity.uri)
            .then_with(|| {
                a.item
                    .item
                    .selection_range
                    .start_line
                    .cmp(&b.item.item.selection_range.start_line)
            })
            .then_with(|| {
                a.item
                    .item
                    .selection_range
                    .start_character
                    .cmp(&b.item.item.selection_range.start_character)
            })
    });
    Some(inventory.reply(
        Some(identity.clone()),
        Vec::new(),
        edges,
        occurrences,
        Vec::new(),
    ))
}
/// Possible-positive incoming CALL sources; foreign inherited members stay debt.
#[must_use]
pub fn logical_incoming_from_inventory(
    identity: &LogicalCallIdentity,
    documents: &[OriginalDeclarationDocument<'_>],
) -> Option<LogicalCallAdvice> {
    edges(identity, documents, true)
}
/// Possible-positive outgoing CALL targets; no complete/runtime graph is supplied.
#[must_use]
pub fn logical_outgoing_from_inventory(
    identity: &LogicalCallIdentity,
    documents: &[OriginalDeclarationDocument<'_>],
) -> Option<LogicalCallAdvice> {
    edges(identity, documents, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::original_declaration::OriginalDeclarationRegistry;
    use tcl_compiler::analyser::Analyser;

    fn document<'a>(
        uri: &'a str,
        source: &'a str,
        analysis: &'a AnalysisResult,
    ) -> OriginalDeclarationDocument<'a> {
        OriginalDeclarationDocument {
            uri,
            source,
            analysis,
        }
    }
    fn prepare(
        uri: &str,
        source: &str,
        offset: usize,
        documents: &[OriginalDeclarationDocument<'_>],
    ) -> LogicalCallAdvice {
        let point =
            LineIndex::new_lsp(source).position_at_utf16(u32::try_from(offset).unwrap(), source);
        prepare_logical_from_inventory(uri, point.line, point.character.get(), documents).unwrap()
    }

    #[test]
    fn genuine_cross_file_prepare_and_both_edge_directions_keep_header_and_caller_owners() {
        let caller_source = "proc caller {} { helper }\n";
        let target_source = "proc helper {} {}\n";
        let caller = Analyser::new().analyse(caller_source, "tcl");
        let target = Analyser::new().analyse(target_source, "tcl");
        let documents = [
            document("file:///caller.tcl", caller_source, &caller),
            document("file:///target.tcl", target_source, &target),
        ];
        let prepared = prepare(
            documents[0].uri,
            caller_source,
            caller_source.find("helper").unwrap(),
            &documents,
        );
        assert!(prepared.is_current(&documents));
        assert_eq!(prepared.possible_source_items().len(), 1);
        let selected = prepared.possible_source_items()[0].identity();
        assert_eq!(selected.uri(), documents[1].uri);
        let incoming = logical_incoming_from_inventory(selected, &documents).unwrap();
        assert!(incoming.is_current(&documents));
        assert_eq!(incoming.possible_source_edges().len(), 1);
        assert_eq!(
            incoming.possible_source_edges()[0]
                .item()
                .presentation()
                .name,
            "::caller"
        );
        let caller_item = incoming.possible_source_edges()[0].item();
        let outgoing = logical_outgoing_from_inventory(caller_item.identity(), &documents).unwrap();
        assert!(outgoing.is_current(&documents));
        assert_eq!(outgoing.possible_source_edges().len(), 1);
        assert_eq!(
            outgoing.possible_source_edges()[0].item().identity().uri(),
            documents[1].uri
        );
    }

    #[test]
    fn bounded_call_kind_cannot_cast_native_type_unissued_or_rebound_consumer() {
        let source = "proc caller {} { helper }\nproc helper {} {}\n";
        let analysis = Analyser::new().analyse(source, "tcl");
        let documents = [document("file:///wire.tcl", source, &analysis)];
        let advice = prepare(
            documents[0].uri,
            source,
            source.find("helper").unwrap(),
            &documents,
        );
        let identity = advice.possible_source_items()[0].identity().clone();
        let mut registry = OriginalDeclarationRegistry::default();
        let publication = crate::original_declaration::LogicalCallHostPublication::retain(());
        let handle = registry
            .issue_logical_call(identity, publication.clone())
            .unwrap();
        assert!(
            registry
                .resolve_logical_call(handle, documents[0].uri, &documents, &publication)
                .is_some()
        );
        for schema in [1, 2, 4] {
            let mut wire = serde_json::to_value(handle).unwrap();
            wire["schema"] = serde_json::json!(schema);
            let wrong = serde_json::from_value(wire).unwrap();
            assert!(
                registry
                    .resolve_logical_call(wrong, documents[0].uri, &documents, &publication)
                    .is_none()
            );
        }
        assert!(
            registry
                .resolve(handle, documents[0].uri, source, &analysis)
                .is_none()
        );
        assert!(
            registry
                .resolve_logical_class(handle, documents[0].uri, source, &analysis)
                .is_none()
        );
        assert!(
            registry
                .resolve_logical_call(handle, "file:///other.tcl", &documents, &publication)
                .is_none()
        );
        let mut wire = serde_json::to_value(handle).unwrap();
        wire["token"] = serde_json::json!(u64::MAX);
        assert!(
            registry
                .resolve_logical_call(
                    serde_json::from_value(wire).unwrap(),
                    documents[0].uri,
                    &documents,
                    &publication
                )
                .is_none()
        );
        let other_publication =
            crate::original_declaration::LogicalCallHostPublication::retain(1_u64);
        assert!(
            registry
                .resolve_logical_call(handle, documents[0].uri, &documents, &other_publication)
                .is_none()
        );
        let changed = source.replacen("helper }", "other }", 1);
        let current = Analyser::new().analyse(&changed, "tcl");
        assert!(
            registry
                .resolve_logical_call(
                    handle,
                    documents[0].uri,
                    &[document(documents[0].uri, &changed, &current)],
                    &publication
                )
                .is_none()
        );
    }

    #[test]
    fn independent_source_input_uri_and_original_operand_currency_are_required() {
        let consumer = "proc caller {} { helper }\n";
        let provider = "proc helper {} {}\n";
        let a = Analyser::new().analyse(consumer, "tcl");
        let b = Analyser::new().analyse(provider, "tcl");
        let docs = [
            document("file:///a.tcl", consumer, &a),
            document("file:///b.tcl", provider, &b),
        ];
        let advice = prepare(
            docs[0].uri,
            consumer,
            consumer.find("helper").unwrap(),
            &docs,
        );
        let identity = advice.possible_source_items()[0].identity();
        assert!(identity.is_current_in(&docs));
        assert!(!identity.is_current_in(&docs[1..]));
        let mut missing = a.clone();
        missing.body_lexer_config = None;
        assert!(!identity.is_current_in(&[document(docs[0].uri, consumer, &missing), docs[1]]));
        assert!(!identity.is_current_in(&[document("file:///wrong.tcl", consumer, &a), docs[1]]));
        assert!(!advice.is_current(&[
            docs[0],
            document(docs[1].uri, "proc helper {} {changed}", &b)
        ]));
        assert!(prepare_logical_from_inventory(docs[0].uri, 0, 1000, &docs).is_none());
    }

    #[test]
    fn root_requires_its_genuine_document_recipe_and_data_or_dynamic_heads_do_not_select() {
        let source = "proc helper {} {}\nhelper\nset data helper\n$head\n";
        let analysis = Analyser::new().analyse(source, "tcl");
        let docs = [document("file:///root.tcl", source, &analysis)];
        let prepared = prepare(docs[0].uri, source, source.find("helper\n").unwrap(), &docs);
        let incoming =
            logical_incoming_from_inventory(prepared.possible_source_items()[0].identity(), &docs)
                .unwrap();
        assert_eq!(incoming.possible_source_edges().len(), 1);
        assert_eq!(
            incoming.possible_source_edges()[0]
                .item()
                .presentation()
                .name,
            "<top-level>"
        );
        assert!(incoming.is_current(&docs));
        let data = prepare(
            docs[0].uri,
            source,
            source.find("data helper").unwrap() + 5,
            &docs,
        );
        assert!(data.possible_source_items().is_empty());
        assert!(data.has_unresolved_applicability());
        let dynamic = prepare(docs[0].uri, source, source.find("$head").unwrap(), &docs);
        assert!(dynamic.possible_source_items().is_empty());
        assert!(dynamic.has_unresolved_applicability());
    }

    #[test]
    fn genuine_own_member_frame_and_selectors_keep_unrepresented_inherited_alternatives() {
        let source = "proc helper {} {}\noo::class create C {superclass $base; method known {} {}; method use {} {my known; my inherited; ::helper}}\n";
        let analysis = Analyser::new().analyse(source, "tcl");
        let docs = [document("file:///members.tcl", source, &analysis)];
        let prepared = prepare(docs[0].uri, source, source.find("use {}").unwrap(), &docs);
        assert_eq!(prepared.possible_source_items().len(), 1);
        let outgoing =
            logical_outgoing_from_inventory(prepared.possible_source_items()[0].identity(), &docs)
                .unwrap();
        let names = outgoing
            .possible_source_edges()
            .iter()
            .map(|edge| edge.item().presentation().name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["::helper", "::C::known"]);
        assert!(outgoing.has_unresolved_applicability());
        assert!(outgoing.is_current(&docs));
        let inherited = prepare(
            docs[0].uri,
            source,
            source.find("my inherited").unwrap() + 3,
            &docs,
        );
        assert!(inherited.possible_source_items().is_empty());
        assert!(inherited.has_unresolved_applicability());
    }

    #[test]
    fn original_unicode_cr_geometry_and_duplicate_providers_remain_possible_source_advice() {
        let caller = "# 😀\rproc caller {} { 茶 }\r";
        let provider = "# 😀\rproc 茶 {} {}\r";
        let a = Analyser::new().analyse(caller, "tcl");
        let b = Analyser::new().analyse(provider, "tcl");
        let c = Analyser::new().analyse(provider, "tcl");
        let docs = [
            document("file:///caller.tcl", caller, &a),
            document("file:///one.tcl", provider, &b),
            document("file:///two.tcl", provider, &c),
        ];
        let advice = prepare(docs[0].uri, caller, caller.find("茶").unwrap(), &docs);
        assert_eq!(advice.possible_source_items().len(), 2);
        assert!(advice.has_unresolved_applicability());
        assert!(advice.is_current(&docs));
        for item in advice.possible_source_items() {
            assert_eq!(item.presentation().selection_range.start_line, 1);
            assert_eq!(item.presentation().selection_range.start_character, 5);
            assert_eq!(item.presentation().selection_range.end_character, 6);
        }
    }
    #[test]
    fn actual_call_header_and_alias_history_survive_without_reporting_proc_maps() {
        let source = "proc p {} {}; rename p moved; interp alias {} shortcut {} moved; shortcut";
        let mut analysis = Analyser::new().analyse(source, "tcl");
        analysis.all_procs.clear();
        analysis.command_invocations.clear();
        let docs = [document("file:///actual.tcl", source, &analysis)];
        let advice = prepare(
            docs[0].uri,
            source,
            source.rfind("shortcut").unwrap(),
            &docs,
        );
        assert_eq!(advice.possible_source_items().len(), 1);
        let identity = advice.possible_source_items()[0].identity();
        assert_eq!(identity.uri(), docs[0].uri);
        assert!(identity.is_current_in(&docs));
        let Selection::Procedure(header) = &identity.selected else {
            panic!("actual source procedure required");
        };
        assert_eq!(
            &source[header.original_name_word().unwrap().span().as_range()],
            "p"
        );
        let provenance = identity.provenance.as_ref().unwrap();
        let Evidence::Procedure {
            retained: Some(call),
            ..
        } = &provenance.evidence
        else {
            panic!("original call owner required");
        };
        assert_eq!(
            call.original_words().first().unwrap().span().start(),
            u32::try_from(source.rfind("shortcut").unwrap()).unwrap()
        );
        assert!(!call.lineage().is_empty());
        let incoming = logical_incoming_from_inventory(identity, &docs).unwrap();
        assert_eq!(incoming.possible_source_edges().len(), 1);
        assert!(incoming.is_current(&docs));
    }

    #[test]
    fn retained_call_target_reissues_consumer_and_declaring_source_under_wire_identity() {
        let source = "proc p {} {}; interp alias {} shortcut {} p; shortcut";
        let analysis = Analyser::new().analyse(source, "tcl");
        let docs = [document("file:///actual.tcl", source, &analysis)];
        let advice = prepare(
            docs[0].uri,
            source,
            source.rfind("shortcut").unwrap(),
            &docs,
        );
        let identity = advice.possible_source_items()[0].identity();
        let changed_source =
            "proc p {} {}; interp alias {} shortcut {} p; rename shortcut {}; shortcut";
        let changed = Analyser::new().analyse(changed_source, "tcl");
        assert!(!identity.is_current_in(&[document(docs[0].uri, changed_source, &changed)]));
        assert!(!identity.is_current_in(&[document("file:///foreign.tcl", source, &analysis)]));
        let foreign_input = Analyser::new().analyse(source, "tcl8.6");
        assert!(!identity.is_current_in(&[document(docs[0].uri, source, &foreign_input)]));
        let mut registry = OriginalDeclarationRegistry::default();
        let publication = crate::original_declaration::LogicalCallHostPublication::retain(());
        let handle = registry
            .issue_logical_call(identity.clone(), publication.clone())
            .unwrap();
        assert!(
            registry
                .resolve_logical_call(handle, docs[0].uri, &docs, &publication)
                .is_some()
        );
        assert!(
            registry
                .resolve_logical_call(
                    handle,
                    docs[0].uri,
                    &[document(docs[0].uri, changed_source, &changed)],
                    &publication
                )
                .is_none()
        );
    }
}
