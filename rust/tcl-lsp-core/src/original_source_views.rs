// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional procedure relationships retaining exact interpretation owners.
//! The host supplies current physical publication chains and rechecks them at
//! rendering. These positive relationships grant no execution or edit proof.

use crate::original_declaration::{
    OriginalDeclarationDocument, OriginalDeclarationIdentity, OriginalSourceViewPublication,
};
use tcl_compiler::command_binding::OriginalSourceProcedureCall;
use tcl_lexer::Span;

/// One exact retained interpretation and its genuine physical ancestor chain.
pub struct OriginalSourceViewDocument<'a> {
    /// Typed host publication locator, independently checked by the host.
    pub publication: &'a OriginalSourceViewPublication,
    /// The actual producer's source and analysis, never a URI reconstruction.
    pub document: OriginalDeclarationDocument<'a>,
    /// Exact issuer-owned partial view, separately selected from a whole input.
    /// Its command inventory never widens whole-document/CU completion.
    pub region: Option<&'a tcl_compiler::command_binding::OriginalSourceConditionalRegion>,
    /// Current retained ancestor locators in nearest-parent-first order.
    pub ancestors: &'a [OriginalSourceViewPublication],
}

impl OriginalSourceViewDocument<'_> {
    fn current(&self) -> Option<()> {
        let analysis = self.document.analysis;
        let input = analysis.resolved_input.as_ref()?;
        if analysis.analysis_context_unavailable.is_some()
            || analysis.body_lexer_config != Some(input.lexer_config())
            || !analysis
                .retained_command_realm()?
                .matches_resolved_analysis_input(input)
        {
            return None;
        }
        match self.region {
            None => crate::original_context::CurrentSourceContext::capture(
                self.document.source,
                self.document.analysis,
            )
            .map(|_| ()),
            Some(region) => {
                let analysis = self.document.analysis;
                let config = analysis.body_lexer_config?;
                (region.matches_region_analysis(analysis)
                    && analysis.original_source_region() == Some(region)
                    && analysis.matches_original_source_image(
                        &tcl_lexer::SourceImage::document(self.document.source),
                        config,
                    ))
                .then_some(())
            }
        }
    }

    fn procedure_calls(&self) -> Option<Vec<OriginalSourceProcedureCall>> {
        self.current()?;
        match self.region {
            Some(region) => region.original_procedure_calls(self.document.analysis),
            None => {
                tcl_compiler::registry_invocation::source_structure::source_original_procedure_calls(
                    self.document.source,
                    self.document.analysis,
                )
            }
        }
    }

    fn owns_command(&self, offset: u32) -> bool {
        self.region
            .is_none_or(|region| region.owns_original_command_at(self.document.analysis, offset))
    }

    fn owns_declaration(&self, identity: &OriginalDeclarationIdentity) -> bool {
        identity
            .procedure_metadata(self.document.analysis)
            .is_some_and(|row| self.owns_command(row.declaration_site().offset))
    }
}

/// A selected declaration retaining the view used for metadata and lifting.
#[derive(Clone)]
pub struct OriginalSourceViewDeclaration {
    /// Same typed publication belonging to the selected producer.
    pub publication: OriginalSourceViewPublication,
    /// Genuine compiler-selected original declaration identity.
    pub identity: OriginalDeclarationIdentity,
}

/// One positive conditional call relationship, with separate actual owners.
pub struct OriginalSourceViewCallEdge {
    /// Actual calling declaration and interpretation.
    pub caller: OriginalSourceViewDeclaration,
    /// Actual declaring interpretation selected by the retained call.
    pub target: OriginalSourceViewDeclaration,
    /// Original written call head in the caller's complete physical image.
    pub span: Span,
}

fn target(
    call: &OriginalSourceProcedureCall,
    consumer: &OriginalSourceViewDocument<'_>,
    documents: &[OriginalSourceViewDocument<'_>],
) -> Option<OriginalSourceViewDeclaration> {
    let mut selected = documents.iter().filter_map(|owner| {
        let record = if owner.publication == consumer.publication {
            call.declaration(consumer.document.analysis)
        } else {
            consumer
                .ancestors
                .contains(owner.publication)
                .then_some(())?;
            call.declaration_in_parent(consumer.document.analysis, owner.document.analysis)
                .or_else(|| {
                    consumer.region?;
                    call.declaration_in_region_dependency(
                        consumer.document.analysis,
                        owner.document.analysis,
                    )
                })
        }?;
        owner.current()?;
        owner
            .owns_command(record.declaration_site().offset)
            .then_some(())?;
        let identity = OriginalDeclarationIdentity::for_procedure(
            owner.document.uri,
            owner.document.source,
            owner.document.analysis,
            record,
        )?;
        Some(OriginalSourceViewDeclaration {
            publication: owner.publication.clone(),
            identity,
        })
    });
    let first = selected.next()?;
    selected.next().is_none().then_some(first)
}

fn region_owns_cursor(owner: &OriginalSourceViewDocument<'_>, offset: u32) -> Option<bool> {
    owner.current()?;
    let region = owner.region?;
    let analysis = owner.document.analysis;
    let contains = |span: Span| span.start() <= offset && offset < span.end();
    if analysis.original_procedure_declarations().any(|row| {
        contains(row.name_input().span())
            && region.owns_original_command_at(analysis, row.declaration_site().offset)
    }) {
        return Some(true);
    }
    let image = tcl_lexer::SourceImage::document(owner.document.source);
    let config = analysis.body_lexer_config?;
    Some(analysis.command_invocations.iter().any(|call| {
        contains(call.range)
            && region.owns_original_command_at(analysis, call.range.start())
            && matches!(call.original_name_input.as_ref(),
                Some(tcl_compiler::signature_scan::scope::SignatureSourceNameInput::OriginalWord(word))
                    if word.source_image() == &image
                        && word.lexer_config() == config
                        && word.span() == call.range)
    }))
}

/// Exact command/declaration ownership chooses appended suffix views. A region
/// can replace only its own retained prefix's answer at an actually owned
/// cursor; unrelated physical interpretations remain independently required.
/// This projection supplies no target or lookup permission.
#[must_use]
pub fn cursor_publications(
    uri: &str,
    offset: u32,
    documents: &[OriginalSourceViewDocument<'_>],
) -> Option<Vec<OriginalSourceViewPublication>> {
    let owners = documents
        .iter()
        .filter(|row| row.document.uri == uri)
        .collect::<Vec<_>>();
    let mut claims = Vec::new();
    for owner in &owners {
        owner.current()?;
        if owner.region.is_some() && region_owns_cursor(owner, offset)? {
            claims.push(*owner);
        }
    }
    Some(
        owners
            .into_iter()
            .filter(|owner| {
                (owner.region.is_none()
                    || claims
                        .iter()
                        .any(|claim| claim.publication == owner.publication))
                    && !claims.iter().any(|claim| {
                        claim.publication != owner.publication
                            && claim.ancestors.contains(owner.publication)
                    })
            })
            .map(|owner| owner.publication.clone())
            .collect(),
    )
}

/// Select a local declaration or exact conditional procedure target. The
/// selected publication is explicit; equal URI/text cannot pick another view.
#[must_use]
pub fn prepare(
    selected: &OriginalSourceViewPublication,
    offset: u32,
    documents: &[OriginalSourceViewDocument<'_>],
) -> Option<OriginalSourceViewDeclaration> {
    let mut owners = documents.iter().filter(|row| row.publication == selected);
    let owner = owners.next()?;
    owners.next().is_none().then_some(())?;
    let doc = &owner.document;
    owner.current()?;
    let rows = crate::original_declaration::declarations(doc.uri, doc.source, doc.analysis)?;
    let mut local = rows.into_iter().filter(|row| {
        owner.owns_declaration(row) && row.span().start() <= offset && offset < row.span().end()
    });
    if let Some(identity) = local.next() {
        return local
            .next()
            .is_none()
            .then(|| OriginalSourceViewDeclaration {
                publication: selected.clone(),
                identity,
            });
    }
    let calls = owner.procedure_calls()?;
    let mut matching = calls.iter().filter(|call| {
        call.original_words().first().is_some_and(|head| {
            owner.owns_command(head.span().start())
                && head.span().start() <= offset
                && offset < head.span().end()
        })
    });
    let call = matching.next()?;
    matching.all(|other| other == call).then_some(())?;
    if !call.uncertain_operations().is_empty() || call.has_lookup_operand_effects() {
        return None;
    }
    target(call, owner, documents)
}

/// Positive conditional procedure edges. Missing coverage is unavailable; an
/// empty result cannot certify absence of calls or edit coverage.
#[must_use]
pub fn procedure_edges(
    documents: &[OriginalSourceViewDocument<'_>],
) -> Vec<OriginalSourceViewCallEdge> {
    let mut result = Vec::new();
    for consumer in documents {
        let doc = &consumer.document;
        if consumer.current().is_none() {
            continue;
        }
        let Some(calls) = consumer.procedure_calls() else {
            continue;
        };
        for call in calls {
            if !call
                .original_words()
                .first()
                .is_some_and(|head| consumer.owns_command(head.span().start()))
            {
                continue;
            }
            if !call.uncertain_operations().is_empty() || call.has_lookup_operand_effects() {
                continue;
            }
            let Some(caller) = call.caller_declaration(doc.analysis).and_then(|record| {
                OriginalDeclarationIdentity::for_procedure(
                    doc.uri,
                    doc.source,
                    doc.analysis,
                    record,
                )
            }) else {
                continue;
            };
            if !consumer.owns_declaration(&caller) {
                continue;
            }
            let Some(target) = target(&call, consumer, documents) else {
                continue;
            };
            let Some(head) = call.original_words().first() else {
                continue;
            };
            result.push(OriginalSourceViewCallEdge {
                caller: OriginalSourceViewDeclaration {
                    publication: consumer.publication.clone(),
                    identity: caller,
                },
                target,
                span: head.span(),
            });
        }
    }
    result
}

/// A positive exact source reference, including top-level calls without a
/// procedure caller. It supplies no complete absence or edit-coverage proof.
pub struct OriginalSourceViewReference {
    /// Actual physical/source publication containing this call.
    pub publication: OriginalSourceViewPublication,
    /// Genuine original written call-head geometry.
    pub span: Span,
}

/// Positive calls to one actual declaration across the supplied current views.
/// Call hierarchy's caller-definition requirement cannot remove file-root
/// references from this separate inventory.
#[must_use]
pub fn references(
    selected: &OriginalSourceViewDeclaration,
    documents: &[OriginalSourceViewDocument<'_>],
) -> Vec<OriginalSourceViewReference> {
    let mut result = Vec::<OriginalSourceViewReference>::new();
    for consumer in documents {
        let Some(calls) = consumer.procedure_calls() else {
            continue;
        };
        for call in calls {
            let Some(head) = call.original_words().first() else {
                continue;
            };
            if !consumer.owns_command(head.span().start())
                || !call.uncertain_operations().is_empty()
                || call.has_lookup_operand_effects()
            {
                continue;
            }
            let Some(target) = target(&call, consumer, documents) else {
                continue;
            };
            if target.publication == selected.publication
                && target.identity == selected.identity
                && !result
                    .iter()
                    .any(|old| &old.publication == consumer.publication && old.span == head.span())
            {
                result.push(OriginalSourceViewReference {
                    publication: consumer.publication.clone(),
                    span: head.span(),
                });
            }
        }
    }
    result
}
