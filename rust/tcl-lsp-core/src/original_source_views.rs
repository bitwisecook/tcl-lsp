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

    fn owns_procedure_call(&self, call: &OriginalSourceProcedureCall) -> bool {
        self.region
            .is_none_or(|region| region.owns_original_procedure_call(self.document.analysis, call))
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

/// Whether this exact current region owns the cursor through a retained command or call.
#[must_use]
pub fn region_owns_cursor(owner: &OriginalSourceViewDocument<'_>, offset: u32) -> Option<bool> {
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
    if owner.procedure_calls()?.iter().any(|call| {
        call.original_words()
            .first()
            .is_some_and(|head| contains(head.span()) && owner.owns_procedure_call(call))
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
    projected_publications(uri, offset, documents, region_owns_cursor)
}

/// Exact schema operand ownership can select an interpreted documentation view.
/// This grants no variable value, invocation target or execution authority.
#[must_use]
pub fn hover_publications(
    uri: &str,
    offset: u32,
    documents: &[OriginalSourceViewDocument<'_>],
) -> Option<Vec<OriginalSourceViewPublication>> {
    projected_publications(uri, offset, documents, |owner, offset| {
        if region_owns_cursor(owner, offset)? {
            return Some(true);
        }
        crate::hover::source_documentation_region_owns_cursor(
            owner.document.source,
            owner.document.analysis,
            offset,
            owner.region?,
        )
    })
}

fn projected_publications(
    uri: &str,
    offset: u32,
    documents: &[OriginalSourceViewDocument<'_>],
    owns_cursor: impl Fn(&OriginalSourceViewDocument<'_>, u32) -> Option<bool>,
) -> Option<Vec<OriginalSourceViewPublication>> {
    let owners = documents
        .iter()
        .filter(|row| row.document.uri == uri)
        .collect::<Vec<_>>();
    let mut claims = Vec::new();
    for owner in &owners {
        owner.current()?;
        if owner.region.is_some() && owns_cursor(owner, offset)? {
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
        let head = call.original_words().first();
        head.is_some_and(|head| {
            let span = head.span();
            let call_is_owned = owner.owns_procedure_call(call);
            call_is_owned && span.start() <= offset && offset < span.end()
        })
    });
    let call = matching.next()?;
    matching.all(|other| other == call).then_some(())?;
    let uncertainty = call.uncertain_operations();
    if !uncertainty.is_empty() {
        return None;
    }
    if call.has_lookup_operand_effects() {
        return None;
    }
    target(call, owner, documents)
}

/// Documentation from this exact selected interpretation and declaration.
/// The host must retain and recheck the physical publication ancestry.
#[must_use]
pub fn procedure_hover(
    selected: &OriginalSourceViewDeclaration,
    documents: &[OriginalSourceViewDocument<'_>],
) -> Option<crate::hover::Hover> {
    let mut owners = documents
        .iter()
        .filter(|row| row.publication == &selected.publication);
    let owner = owners.next()?;
    owners.next().is_none().then_some(())?;
    owner.current()?;
    owner.owns_declaration(&selected.identity).then_some(())?;
    (owner.document.uri == selected.identity.uri()).then_some(())?;
    crate::hover::original_source_procedure_hover(
        owner.document.source,
        owner.document.analysis,
        &selected.identity,
    )
}

/// Source-only schema or root-special-variable documentation from one exact
/// current interpretation. The host independently leases its ancestry.
#[must_use]
pub fn documentation_hover(
    publication: &OriginalSourceViewPublication,
    offset: u32,
    documents: &[OriginalSourceViewDocument<'_>],
) -> Option<crate::hover::Hover> {
    let mut owners = documents
        .iter()
        .filter(|row| row.publication == publication);
    let owner = owners.next()?;
    owners.next().is_none().then_some(())?;
    owner.current()?;
    if let Some(region) = owner.region {
        crate::hover::source_documentation_region_owns_cursor(
            owner.document.source,
            owner.document.analysis,
            offset,
            region,
        )?
        .then_some(())?;
    }
    let position = tcl_lexer::LineIndex::new_lsp(owner.document.source)
        .position_at_utf16(offset, owner.document.source);
    crate::hover::source_documentation_hover(
        owner.document.source,
        position.line,
        position.character.get(),
        owner.document.analysis,
    )
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
            if call.original_words().is_empty() || !consumer.owns_procedure_call(&call) {
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
            if !consumer.owns_procedure_call(&call)
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

/// A conditional external SOURCE header request from one exact deferred body.
/// This retains unresolved applicability debts. It is not a resolved binding,
/// runtime lookup/absence, entered frame, Normal result or edit permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalExternalAutoloadRequest {
    publication: OriginalSourceViewPublication,
    cursor: u32,
    advice:
        tcl_compiler::registry_invocation::source_structure::OriginalNativeDeferredAutoloadAdvice,
}

impl ConditionalExternalAutoloadRequest {
    /// Exact source interpretation that issued the conditional name recipe.
    #[must_use]
    pub const fn publication(&self) -> &OriginalSourceViewPublication {
        &self.publication
    }

    /// Unresolved conditional applicability; no debt becomes an execution fact.
    #[must_use]
    pub fn obligations(&self) -> &[tcl_compiler::registry_invocation::source_structure::OriginalNativeDeferredAutoloadObligation]{
        self.advice.obligations()
    }

    /// Real tclIndex source candidates in the canonical C auto-qualify order.
    /// The files are advice only; their independent current headers remain owed.
    #[must_use]
    pub fn files(
        &self,
        resolver: &crate::package_resolver::PackageResolver,
    ) -> Vec<std::path::PathBuf> {
        resolver.resolve_conditional_native_autoload_source(&self.advice)
    }
}

fn conditional_external_request(
    owner: &OriginalSourceViewDocument<'_>,
    cursor: u32,
) -> Option<ConditionalExternalAutoloadRequest> {
    owner.current()?;
    // An interrupted physical prefix cannot donate a later inherited body.
    // A region has its own genuine issuer; an ordinary root must be complete.
    if owner.region.is_none()
        && owner
            .document
            .analysis
            .original_completed_command_world()
            .is_none()
    {
        return None;
    }
    let advice = tcl_compiler::registry_invocation::source_structure::source_native_deferred_autoload_advice_at(
        owner.document.source, owner.document.analysis, cursor,
    )?;
    if !advice.is_for_analysis(owner.document.analysis) {
        return None;
    }
    let caller = advice.caller_declaration(owner.document.analysis)?;
    let head = advice.name().span();
    if !(owner.owns_command(caller.declaration_site().offset)
        && head.start() <= cursor
        && cursor < head.end())
    {
        return None;
    }
    Some(ConditionalExternalAutoloadRequest {
        publication: owner.publication.clone(),
        cursor,
        advice,
    })
}

/// Select conditional external header recipes by the genuine declaring-body
/// owner. A suffix can claim only its own sealed body; URI/ranges alone cannot
/// select or manufacture an inherited interpretation.
#[must_use]
pub fn conditional_external_autoload_requests(
    uri: &str,
    cursor: u32,
    documents: &[OriginalSourceViewDocument<'_>],
) -> Option<Vec<ConditionalExternalAutoloadRequest>> {
    let mut claimed = Vec::new();
    for owner in documents.iter().filter(|owner| owner.document.uri == uri) {
        owner.current()?;
        // Discover body ownership before checking issuance eligibility. An
        // incomplete interpretation that owns this cursor must not disappear
        // from the selected set and let a different interpretation answer.
        let commands = tcl_compiler::registry_invocation::source_structure::source_native_procedure_body_commands(
            owner.document.source, owner.document.analysis,
        )?;
        // This phase/caller membership is selection only. It grants no lookup
        // and cannot turn a refused most-specific recipe into an ancestor one.
        let owns_body = commands.iter().any(|command| {
            let words = command.original_words();
            let Some(first) = words.first() else {
                return false;
            };
            let Some(last) = words.last() else {
                return false;
            };
            let Some(caller) = command.caller_declaration(owner.document.analysis) else {
                return false;
            };
            first.span().start() <= cursor
                && cursor < last.span().end()
                && owner.owns_command(caller.declaration_site().offset)
        });
        if owns_body {
            claimed.push(owner);
        }
    }
    let mut result = Vec::new();
    for owner in &claimed {
        if claimed
            .iter()
            .any(|child| child.ancestors.contains(owner.publication))
        {
            continue;
        }
        let request = conditional_external_request(owner, cursor)?;
        if result
            .iter()
            .any(|old: &ConditionalExternalAutoloadRequest| old.publication == request.publication)
        {
            return None;
        }
        result.push(request);
    }
    (!result.is_empty()).then_some(result)
}

/// A conditional external SOURCE-header association, explicitly separate from
/// an original call's resolved binding or entered implementation allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalExternalSourceHeader {
    request: ConditionalExternalAutoloadRequest,
    declaration: OriginalDeclarationIdentity,
}

impl ConditionalExternalSourceHeader {
    /// Retained conditional purpose and its unresolved applicability debts.
    #[must_use]
    pub const fn request(&self) -> &ConditionalExternalAutoloadRequest {
        &self.request
    }

    /// Independent current source declaration only; this is no call binding.
    #[must_use]
    pub const fn declaration(&self) -> &OriginalDeclarationIdentity {
        &self.declaration
    }
}

/// Match one conditional body name to independently current external SOURCE
/// headers. This does not promote the advice to the call's runtime binding.
/// The caller must retain/rejoin both producer and provider physical leases.
#[must_use]
pub fn conditional_external_autoload_header(
    request: &ConditionalExternalAutoloadRequest,
    documents: &[OriginalSourceViewDocument<'_>],
    providers: &[crate::original_declaration::OriginalDeclarationDocument<'_>],
) -> Option<ConditionalExternalSourceHeader> {
    let mut owners = documents
        .iter()
        .filter(|owner| owner.publication == &request.publication);
    let owner = owners.next()?;
    owners.next().is_none().then_some(())?;
    if conditional_external_request(owner, request.cursor).as_ref() != Some(request) {
        return None;
    }
    let registry = owner.document.analysis.resolved_registry()?;
    let mut headers = Vec::new();
    for provider in providers {
        if provider.uri == owner.document.uri {
            return None;
        }
        crate::original_context::CurrentSourceContext::capture(provider.source, provider.analysis)?;
        for row in provider.analysis.original_procedure_declarations() {
            let identity = OriginalDeclarationIdentity::for_procedure(
                provider.uri,
                provider.source,
                provider.analysis,
                row,
            );
            headers.push((row.name().clone(), identity));
        }
        for row in provider.analysis.original_class_declarations() {
            headers.push((row.name().clone(), None));
        }
    }
    let matches = request.advice.matching_external_source_slots(
        registry,
        headers
            .iter()
            .map(|(name, identity)| (name.slot(), name.policy(), identity.clone())),
    )?;
    match matches.as_slice() {
        [Some(identity)] => Some(ConditionalExternalSourceHeader {
            request: request.clone(),
            declaration: identity.clone(),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod nested_procedure_navigation_tests {
    use super::*;
    use tcl_compiler::analyser::{Analyser, AnalysisResult};

    fn select(
        source: &str,
        analysis: &AnalysisResult,
        offset: u32,
    ) -> Option<OriginalSourceViewDeclaration> {
        let publication = OriginalSourceViewPublication::retain(1_u8);
        let inventory = [OriginalSourceViewDocument {
            publication: &publication,
            document: OriginalDeclarationDocument {
                uri: "file:///host-naming.tcl",
                source,
                analysis,
            },
            region: None,
            ancestors: &[],
        }];
        prepare(&publication, offset, &inventory)
    }

    #[test]
    fn nested_procedure_navigation_keeps_its_original_operand_horizon() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for source in [
                "proc host_old {value} { return $value }\nlist \"🧪 café\"; puts [host_old 7]\n",
                "proc host_old {value} {return $value}\nputs [list [host_old 7]]\n",
                "proc host_old {value} {return $value}\nputs [host_old 7] $later\n",
            ] {
                let analysis = Analyser::new().analyse(source, dialect);
                let offset = u32::try_from(source.find("host_old 7").unwrap()).unwrap();
                let selected = select(source, &analysis, offset).expect(source);
                assert_eq!(&source[selected.identity.span().as_range()], "host_old");
                assert_eq!(selected.identity.span().start(), 5);
                let call = tcl_compiler::registry_invocation::source_structure::source_original_procedure_call_at(
                    source, &analysis, offset,
                ).unwrap();
                assert_eq!(call.uncertain_operations(), []);
                assert!(!call.has_lookup_operand_effects());
                let independent = Analyser::new().analyse(source, dialect);
                assert!(call.declaration(&independent).is_none());
                let changed = source.replace("host_old 7", "host_old 8");
                assert!(select(&changed, &analysis, offset).is_none());
            }
        }
    }

    #[test]
    fn nested_procedure_navigation_preserves_prior_channel_write_debt() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for payload in ["\"🧪 café\"", "\"ascii\""] {
                let source = format!(
                    "proc host_old {{value}} {{ return $value }}\nputs {payload}; puts [host_old 7]\n"
                );
                let analysis = Analyser::new().analyse(&source, dialect);
                let offset = u32::try_from(source.find("host_old 7").unwrap()).unwrap();
                let call = tcl_compiler::registry_invocation::source_structure::source_original_procedure_call_at(
                    &source, &analysis, offset,
                ).unwrap();
                let declaration = call.declaration(&analysis).unwrap();
                assert_eq!(declaration.name_input().bytes(), b"host_old");
                assert!(!call.has_lookup_operand_effects());
                let [earlier] = call.uncertain_operations() else {
                    panic!("expected the earlier channel write's unresolved effects");
                };
                assert_eq!(earlier.len(), 2);
                assert_eq!(earlier[0].bytes(), b"puts");
                assert_eq!(earlier[1].bytes(), payload.as_bytes());
                let earlier_offset = source.find("\nputs ").unwrap() + 1;
                assert_eq!(
                    earlier[0].span().start(),
                    u32::try_from(earlier_offset).unwrap()
                );
                assert!(select(&source, &analysis, offset).is_none());
            }
        }
    }

    #[test]
    fn nested_procedure_navigation_retains_earlier_operand_and_parent_debts() {
        for source in [
            "proc host_old {value} {return $value}\nputs $earlier [host_old 7]\n",
            "proc host_old {value} {return $value}\nputs $earlier(index) [host_old 7]\n",
            "proc host_old {value} {return $value}\nputs [opaque] [host_old 7]\n",
            "proc host_old {value} {return $value}\nputs [rename host_old removed; host_old 7]\n",
            "proc host_old {value} {return $value}\nputs [host_old [opaque]]\n",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let offset = u32::try_from(source.rfind("host_old ").unwrap()).unwrap();
            assert!(select(source, &analysis, offset).is_none(), "{source}");
        }
        let source = "proc host_old {value} {return $value}\nputs [host_old 7]; host_old 8\n";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.rfind("host_old 8").unwrap()).unwrap();
        assert!(select(source, &analysis, offset).is_none());
    }
}

#[cfg(test)]
mod region_call_cursor_tests {
    use super::*;
    use tcl_compiler::analyser::{Analyser, AnalysisResult};
    use tcl_compiler::command_binding::OriginalSourceConditionalRegion;

    const CHILD: &str = "proc helper {} {}";

    fn fixture(
        source: &str,
    ) -> (
        AnalysisResult,
        AnalysisResult,
        OriginalSourceConditionalRegion,
        AnalysisResult,
    ) {
        let parent = Analyser::new().analyse(source, "tcl8.6");
        let load =
            tcl_compiler::registry_invocation::source_structure::original_project_source_load_at(
                source, &parent, 0,
            )
            .unwrap();
        let point = load.source_point_entry(&parent).unwrap();
        let child = Analyser::new()
            .analyse_with_source_point_entry(CHILD, "tcl8.6", &point)
            .unwrap();
        let continuation = load.conditional_continuation(&parent, &child).unwrap();
        let regions = continuation.source_regions(&parent, &child).unwrap();
        let region = regions.regions()[0].clone();
        let caller = Analyser::new()
            .analyse_source_region(&region, &parent, &child)
            .unwrap();
        (parent, child, region, caller)
    }

    #[test]
    fn special_variable_documentation_selects_its_owned_suffix_command() {
        let source = "source child.tcl; puts $::env(PATH)";
        let (parent, child, region, caller) = fixture(source);
        let root = OriginalSourceViewPublication::retain(1_u8);
        let loaded = OriginalSourceViewPublication::retain(2_u8);
        let suffix = OriginalSourceViewPublication::retain(3_u8);
        let ancestors = [loaded.clone(), root.clone()];
        let inventory = [
            OriginalSourceViewDocument {
                publication: &root,
                document: OriginalDeclarationDocument {
                    uri: "file:///main.tcl",
                    source,
                    analysis: &parent,
                },
                region: None,
                ancestors: &[],
            },
            OriginalSourceViewDocument {
                publication: &loaded,
                document: OriginalDeclarationDocument {
                    uri: "file:///child.tcl",
                    source: CHILD,
                    analysis: &child,
                },
                region: None,
                ancestors: &[],
            },
            OriginalSourceViewDocument {
                publication: &suffix,
                document: OriginalDeclarationDocument {
                    uri: "file:///main.tcl",
                    source,
                    analysis: &caller,
                },
                region: Some(&region),
                ancestors: &ancestors,
            },
        ];
        let cursor = u32::try_from(source.find("env(PATH)").unwrap() + 1).unwrap();
        assert_eq!(
            hover_publications("file:///main.tcl", cursor, &inventory),
            Some(vec![suffix.clone()])
        );
        let hover = documentation_hover(&suffix, cursor, &inventory).unwrap();
        assert!(hover.value.contains("env"));
        assert!(
            hover
                .value
                .contains("binding, availability and the current value are unresolved")
        );
        assert!(documentation_hover(&loaded, cursor, &inventory).is_none());
    }

    #[test]
    fn procedure_hover_requires_selected_publication_and_original_metadata_owner() {
        let source = "source child.tcl; helper";
        let (parent, child, region, caller) = fixture(source);
        let root = OriginalSourceViewPublication::retain(1_u8);
        let loaded = OriginalSourceViewPublication::retain(2_u8);
        let suffix = OriginalSourceViewPublication::retain(3_u8);
        let ancestors = [loaded.clone(), root.clone()];
        let mut inventory = vec![
            OriginalSourceViewDocument {
                publication: &root,
                document: OriginalDeclarationDocument {
                    uri: "file:///main.tcl",
                    source,
                    analysis: &parent,
                },
                region: None,
                ancestors: &[],
            },
            OriginalSourceViewDocument {
                publication: &loaded,
                document: OriginalDeclarationDocument {
                    uri: "file:///child.tcl",
                    source: CHILD,
                    analysis: &child,
                },
                region: None,
                ancestors: &[],
            },
            OriginalSourceViewDocument {
                publication: &suffix,
                document: OriginalDeclarationDocument {
                    uri: "file:///main.tcl",
                    source,
                    analysis: &caller,
                },
                region: Some(&region),
                ancestors: &ancestors,
            },
        ];
        let offset = u32::try_from(source.rfind("helper").unwrap()).unwrap();
        let selected = prepare(&suffix, offset, &inventory).unwrap();
        let hover = procedure_hover(&selected, &inventory).unwrap();
        assert!(hover.value.contains("Conditional source declaration"));
        assert!(hover.value.contains("proc ::helper {}"));
        let foreign = Analyser::new().analyse(CHILD, "tcl8.6");
        inventory[1].document.analysis = &foreign;
        assert!(procedure_hover(&selected, &inventory).is_none());
        inventory[1].document.analysis = &child;
        inventory[1].document.source = "proc another {} {}";
        assert!(procedure_hover(&selected, &inventory).is_none());
        inventory[1].document.source = CHILD;
        let mismatched = OriginalSourceViewDeclaration {
            publication: suffix.clone(),
            identity: selected.identity.clone(),
        };
        assert!(procedure_hover(&mismatched, &inventory).is_none());
        inventory.push(OriginalSourceViewDocument {
            publication: &loaded,
            document: OriginalDeclarationDocument {
                uri: "file:///child.tcl",
                source: CHILD,
                analysis: &child,
            },
            region: None,
            ancestors: &[],
        });
        assert!(procedure_hover(&selected, &inventory).is_none());
    }

    #[test]
    fn region_call_cursor_uses_retained_call_after_reporting_rows_clear() {
        let source = "source child.tcl; helper";
        let (parent, child, region, mut caller) = fixture(source);
        caller.command_invocations.clear();
        let root = OriginalSourceViewPublication::retain(1_u8);
        let loaded = OriginalSourceViewPublication::retain(2_u8);
        let suffix = OriginalSourceViewPublication::retain(3_u8);
        let ancestors = [loaded.clone(), root.clone()];
        let inventory = [
            OriginalSourceViewDocument {
                publication: &root,
                document: OriginalDeclarationDocument {
                    uri: "file:///main.tcl",
                    source,
                    analysis: &parent,
                },
                region: None,
                ancestors: &[],
            },
            OriginalSourceViewDocument {
                publication: &loaded,
                document: OriginalDeclarationDocument {
                    uri: "file:///child.tcl",
                    source: CHILD,
                    analysis: &child,
                },
                region: None,
                ancestors: &[],
            },
            OriginalSourceViewDocument {
                publication: &suffix,
                document: OriginalDeclarationDocument {
                    uri: "file:///main.tcl",
                    source,
                    analysis: &caller,
                },
                region: Some(&region),
                ancestors: &ancestors,
            },
        ];
        let offset = u32::try_from(source.rfind("helper").unwrap()).unwrap();
        assert_eq!(
            cursor_publications("file:///main.tcl", offset, &inventory),
            Some(vec![suffix.clone()])
        );
        let selected = prepare(&suffix, offset, &inventory).unwrap();
        assert_eq!(selected.publication, loaded);
        assert_eq!(&CHILD[selected.identity.span().as_range()], "helper");
        assert_eq!(
            cursor_publications("file:///main.tcl", 0, &inventory),
            Some(vec![root])
        );

        let changed = "source other.tcl; helper";
        let stale = [OriginalSourceViewDocument {
            publication: &suffix,
            document: OriginalDeclarationDocument {
                uri: "file:///main.tcl",
                source: changed,
                analysis: &caller,
            },
            region: Some(&region),
            ancestors: &ancestors,
        }];
        assert!(cursor_publications("file:///main.tcl", offset, &stale).is_none());
        let foreign = Analyser::new().analyse(source, "tcl8.6");
        let foreign = [OriginalSourceViewDocument {
            publication: &suffix,
            document: OriginalDeclarationDocument {
                uri: "file:///main.tcl",
                source,
                analysis: &foreign,
            },
            region: Some(&region),
            ancestors: &ancestors,
        }];
        assert!(cursor_publications("file:///main.tcl", offset, &foreign).is_none());
    }

    #[test]
    fn region_cursor_claim_cannot_donate_target_or_clear_earlier_debt() {
        for source in [
            "source child.tcl; helper",
            "source child.tcl; opaque; helper",
        ] {
            let (parent, child, region, mut caller) = fixture(source);
            caller.command_invocations.clear();
            let root = OriginalSourceViewPublication::retain(1_u8);
            let loaded = OriginalSourceViewPublication::retain(2_u8);
            let suffix = OriginalSourceViewPublication::retain(3_u8);
            let ancestors = [loaded.clone(), root];
            let load = tcl_compiler::registry_invocation::source_structure::original_project_source_load_at(source, &parent, 0).unwrap();
            let point = load.source_point_entry(&parent).unwrap();
            let foreign = Analyser::new()
                .analyse_with_source_point_entry(CHILD, "tcl8.6", &point)
                .unwrap();
            let inventory = [
                OriginalSourceViewDocument {
                    publication: &loaded,
                    document: OriginalDeclarationDocument {
                        uri: "file:///child.tcl",
                        source: CHILD,
                        analysis: &foreign,
                    },
                    region: None,
                    ancestors: &[],
                },
                OriginalSourceViewDocument {
                    publication: &suffix,
                    document: OriginalDeclarationDocument {
                        uri: "file:///main.tcl",
                        source,
                        analysis: &caller,
                    },
                    region: Some(&region),
                    ancestors: &ancestors,
                },
            ];
            let offset = u32::try_from(source.rfind("helper").unwrap()).unwrap();
            assert!(prepare(&suffix, offset, &inventory).is_none());
            if source.contains("opaque") {
                let inventory = [
                    OriginalSourceViewDocument {
                        publication: &loaded,
                        document: OriginalDeclarationDocument {
                            uri: "file:///child.tcl",
                            source: CHILD,
                            analysis: &child,
                        },
                        region: None,
                        ancestors: &[],
                    },
                    OriginalSourceViewDocument {
                        publication: &suffix,
                        document: OriginalDeclarationDocument {
                            uri: "file:///main.tcl",
                            source,
                            analysis: &caller,
                        },
                        region: Some(&region),
                        ancestors: &ancestors,
                    },
                ];
                assert!(prepare(&suffix, offset, &inventory).is_none());
            }
        }
    }

    #[test]
    fn deferred_region_body_call_retains_its_selected_caller_owner() {
        let source = "source child.tcl; proc run {} {puts [helper]}";
        let (parent, child, region, mut caller) = fixture(source);
        caller.command_invocations.clear();
        let root = OriginalSourceViewPublication::retain(1_u8);
        let loaded = OriginalSourceViewPublication::retain(2_u8);
        let suffix = OriginalSourceViewPublication::retain(3_u8);
        let ancestors = [loaded.clone(), root.clone()];
        let inventory = [
            OriginalSourceViewDocument {
                publication: &root,
                document: OriginalDeclarationDocument {
                    uri: "file:///main.tcl",
                    source,
                    analysis: &parent,
                },
                region: None,
                ancestors: &[],
            },
            OriginalSourceViewDocument {
                publication: &loaded,
                document: OriginalDeclarationDocument {
                    uri: "file:///child.tcl",
                    source: CHILD,
                    analysis: &child,
                },
                region: None,
                ancestors: &[],
            },
            OriginalSourceViewDocument {
                publication: &suffix,
                document: OriginalDeclarationDocument {
                    uri: "file:///main.tcl",
                    source,
                    analysis: &caller,
                },
                region: Some(&region),
                ancestors: &ancestors,
            },
        ];
        let offset = u32::try_from(source.rfind("helper").unwrap()).unwrap();
        assert!(!region.owns_original_command_at(&caller, offset));
        let calls = region.original_procedure_calls(&caller).unwrap();
        let call = calls
            .iter()
            .find(|call| call.original_words()[0].span().start() == offset)
            .unwrap();
        assert!(call.is_future_body());
        assert!(region.owns_original_procedure_call(&caller, call));
        assert_eq!(
            cursor_publications("file:///main.tcl", offset, &inventory),
            Some(vec![suffix.clone()])
        );
        let selected = prepare(&suffix, offset, &inventory).unwrap();
        assert_eq!(selected.publication, loaded);
        assert_eq!(&CHILD[selected.identity.span().as_range()], "helper");
        let found = references(&selected, &inventory);
        assert!(
            found
                .iter()
                .any(|row| row.publication == suffix && row.span.start() == offset)
        );
        let (_, foreign_child, foreign_region, foreign_caller) = fixture(source);
        assert!(!foreign_region.owns_original_procedure_call(&foreign_caller, call));
        assert!(!region.owns_original_procedure_call(&foreign_caller, call));
        assert!(
            call.declaration_in_region_dependency(&caller, &foreign_child)
                .is_none()
        );
    }

    #[test]
    fn deferred_region_body_call_keeps_debt_and_data_exclusion() {
        for source in [
            "source child.tcl; proc run {} {opaque; puts [helper]}",
            "source child.tcl; proc run {} {set payload {helper}}",
        ] {
            let (_, child, region, caller) = fixture(source);
            let loaded = OriginalSourceViewPublication::retain(2_u8);
            let suffix = OriginalSourceViewPublication::retain(3_u8);
            let ancestors = [loaded.clone()];
            let inventory = [
                OriginalSourceViewDocument {
                    publication: &loaded,
                    document: OriginalDeclarationDocument {
                        uri: "file:///child.tcl",
                        source: CHILD,
                        analysis: &child,
                    },
                    region: None,
                    ancestors: &[],
                },
                OriginalSourceViewDocument {
                    publication: &suffix,
                    document: OriginalDeclarationDocument {
                        uri: "file:///main.tcl",
                        source,
                        analysis: &caller,
                    },
                    region: Some(&region),
                    ancestors: &ancestors,
                },
            ];
            let offset = u32::try_from(source.rfind("helper").unwrap()).unwrap();
            assert!(prepare(&suffix, offset, &inventory).is_none(), "{source}");
            let changed = source.replace("helper", "other_");
            let stale = [OriginalSourceViewDocument {
                publication: &suffix,
                document: OriginalDeclarationDocument {
                    uri: "file:///main.tcl",
                    source: &changed,
                    analysis: &caller,
                },
                region: Some(&region),
                ancestors: &ancestors,
            }];
            assert!(cursor_publications("file:///main.tcl", offset, &stale).is_none());
        }
    }
}

#[cfg(test)]
mod conditional_external_autoload_tests {
    use super::*;
    use crate::vfs::SourceStore as _;
    use tcl_compiler::analyser::Analyser;

    const SOURCE: &str = "source lib/helpers.tcl\n\nproc run {} {\n    puts [greet_helper world]\n    puts [mypkg_autoloaded]\n}\n";
    const CHILD: &str = "proc greet_helper {who} {return \"hi $who\"}\n";
    const PROVIDER: &str = "proc mypkg_autoloaded {} {return 42}\n";

    fn with_views<R>(
        source: &str,
        use_views: impl FnOnce(&[OriginalSourceViewDocument<'_>], &OriginalSourceViewPublication) -> R,
    ) -> R {
        let prefix = Analyser::new().analyse(source, "tcl8.6");
        let load =
            tcl_compiler::registry_invocation::source_structure::original_project_source_load_at(
                source, &prefix, 0,
            )
            .unwrap();
        let point = load.source_point_entry(&prefix).unwrap();
        let child = Analyser::new()
            .analyse_with_source_point_entry(CHILD, "tcl8.6", &point)
            .unwrap();
        let regions = load
            .conditional_continuation(&prefix, &child)
            .unwrap()
            .source_regions(&prefix, &child)
            .unwrap();
        assert_eq!(regions.regions().len(), 1);
        let region = &regions.regions()[0];
        let suffix = Analyser::new()
            .analyse_source_region(region, &prefix, &child)
            .unwrap();
        let a = OriginalSourceViewPublication::retain(1_u8);
        let b = OriginalSourceViewPublication::retain(2_u8);
        let c = OriginalSourceViewPublication::retain(3_u8);
        let child_parents = [a.clone()];
        let suffix_parents = [b.clone(), a.clone()];
        let views = [
            OriginalSourceViewDocument {
                publication: &a,
                document: OriginalDeclarationDocument {
                    uri: "file:///main.tcl",
                    source,
                    analysis: &prefix,
                },
                region: None,
                ancestors: &[],
            },
            OriginalSourceViewDocument {
                publication: &b,
                document: OriginalDeclarationDocument {
                    uri: "file:///helpers.tcl",
                    source: CHILD,
                    analysis: &child,
                },
                region: None,
                ancestors: &child_parents,
            },
            OriginalSourceViewDocument {
                publication: &c,
                document: OriginalDeclarationDocument {
                    uri: "file:///main.tcl",
                    source,
                    analysis: &suffix,
                },
                region: Some(region),
                ancestors: &suffix_parents,
            },
        ];
        use_views(&views, &c)
    }

    fn cursor(source: &str) -> u32 {
        u32::try_from(source.rfind("mypkg_autoloaded").unwrap() + 4).unwrap()
    }

    #[test]
    fn deferred_body_index_headers_are_conditional_and_keep_exact_view_owners() {
        use tcl_compiler::registry_invocation::source_structure::OriginalNativeDeferredAutoloadObligation as Debt;
        with_views(SOURCE, |views, suffix| {
            let at = cursor(SOURCE);
            assert!(
                prepare(suffix, at, views).is_none(),
                "new advice must not manufacture ordinary binding"
            );
            let requests = conditional_external_autoload_requests("file:///main.tcl", at, views)
                .expect("canonical deferred source recipe");
            assert_eq!(requests.len(), 1);
            assert_eq!(requests[0].publication(), suffix);
            assert!(requests[0].obligations().contains(&Debt::BodyEntry));
            assert!(requests[0].obligations().contains(&Debt::ActualLookup));
            for debt in [
                Debt::PriorChannelWriteApplicability,
                Debt::PriorChannelWriteNormalContinuation,
                Debt::UnresolvedChannelCallbacks,
                Debt::PostChannelCallbackLookup,
            ] {
                assert!(requests[0].obligations().contains(&debt));
            }
            assert!(
                requests[0]
                    .obligations()
                    .contains(&Debt::UnknownEarlierMutation)
            );
            let store = crate::vfs::MemoryStore::new();
            store.upsert("/library/impl.tcl", PROVIDER.as_bytes().to_vec());
            let mut resolver = crate::package_resolver::PackageResolver::default();
            resolver.add_original_tcl_index("# Tcl autoload index file, version 2.0\nset auto_index(mypkg_autoloaded) [list source [file join $dir impl.tcl]]\n",std::path::Path::new("/library"),&|path|store.is_file(path));
            assert_eq!(
                requests[0].files(&resolver),
                [std::path::PathBuf::from("/library/impl.tcl")]
            );
            let provider = Analyser::new().analyse(PROVIDER, "tcl8.6");
            let headers = [OriginalDeclarationDocument {
                uri: "file:///library/impl.tcl",
                source: PROVIDER,
                analysis: &provider,
            }];
            let selected = conditional_external_autoload_header(&requests[0], views, &headers)
                .expect("conditional external source header");
            assert_eq!(selected.request(), &requests[0]);
            assert_eq!(selected.declaration().uri(), headers[0].uri);
            assert_eq!(selected.declaration().span(), Span::new(5, 21));
            assert!(prepare(suffix, at, views).is_none());
        });
    }

    #[test]
    fn deferred_body_index_headers_refuse_stale_foreign_and_ambiguous_providers() {
        with_views(SOURCE, |views, _| {
            let at = cursor(SOURCE);
            let requests =
                conditional_external_autoload_requests("file:///main.tcl", at, views).unwrap();
            let provider = Analyser::new().analyse(PROVIDER, "tcl8.6");
            let stale = [OriginalDeclarationDocument {
                uri: "file:///library/impl.tcl",
                source: "proc replaced {} {}",
                analysis: &provider,
            }];
            assert!(conditional_external_autoload_header(&requests[0], views, &stale).is_none());
            let duplicate = Analyser::new().analyse(PROVIDER, "tcl8.6");
            let headers = [
                OriginalDeclarationDocument {
                    uri: "file:///library/impl.tcl",
                    source: PROVIDER,
                    analysis: &provider,
                },
                OriginalDeclarationDocument {
                    uri: "file:///other/impl.tcl",
                    source: PROVIDER,
                    analysis: &duplicate,
                },
            ];
            assert!(conditional_external_autoload_header(&requests[0], views, &headers).is_none());
            let caller = views
                .iter()
                .find(|view| view.publication == requests[0].publication())
                .unwrap();
            let foreign = Analyser::new().analyse(SOURCE, "tcl8.6");
            let bad = [OriginalSourceViewDocument {
                publication: caller.publication,
                document: OriginalDeclarationDocument {
                    uri: caller.document.uri,
                    source: SOURCE,
                    analysis: &foreign,
                },
                region: caller.region,
                ancestors: caller.ancestors,
            }];
            assert!(conditional_external_autoload_requests("file:///main.tcl", at, &bad).is_none());
            let changed = format!("{SOURCE} ");
            let bad = [OriginalSourceViewDocument {
                publication: caller.publication,
                document: OriginalDeclarationDocument {
                    uri: caller.document.uri,
                    source: &changed,
                    analysis: caller.document.analysis,
                },
                region: caller.region,
                ancestors: caller.ancestors,
            }];
            assert!(conditional_external_autoload_requests("file:///main.tcl", at, &bad).is_none());
        });
    }

    #[test]
    fn deferred_body_index_headers_preserve_positioned_shadow_and_effect_barriers() {
        for body in [
            "proc mypkg_autoloaded {} {}; puts [mypkg_autoloaded]",
            "rename mypkg_autoloaded removed; puts [mypkg_autoloaded]",
            "mystery; puts [mypkg_autoloaded]",
            "puts [mystery]; puts [mypkg_autoloaded]",
            "puts [mypkg_autoloaded $arg]",
        ] {
            let source = format!("source lib/helpers.tcl\nproc run {{arg}} {{{body}}}\n");
            with_views(&source, |views, _| {
                assert!(
                    conditional_external_autoload_requests(
                        "file:///main.tcl",
                        cursor(&source),
                        views
                    )
                    .is_none(),
                    "{source}"
                );
            });
        }
    }
    #[test]
    fn deferred_body_index_headers_do_not_omit_an_owning_incomplete_interpretation() {
        // A real inherited child has body membership, but no ordinary complete
        // root world. It must not vanish when a second interpretation answers.
        let main = "source child.tcl";
        let child_source = "proc run {} {puts [mypkg_autoloaded]}";
        let prefix = Analyser::new().analyse(main, "tcl8.6");
        let load =
            tcl_compiler::registry_invocation::source_structure::original_project_source_load_at(
                main, &prefix, 0,
            )
            .unwrap();
        let point = load.source_point_entry(&prefix).unwrap();
        let child = Analyser::new()
            .analyse_with_source_point_entry(child_source, "tcl8.6", &point)
            .unwrap();
        assert!(child.original_completed_command_world().is_none());
        let independent = Analyser::new().analyse(child_source, "tcl8.6");
        let a = OriginalSourceViewPublication::retain(81_u8);
        let b = OriginalSourceViewPublication::retain(82_u8);
        let documents = [
            OriginalSourceViewDocument {
                publication: &a,
                document: OriginalDeclarationDocument {
                    uri: "file:///child.tcl",
                    source: child_source,
                    analysis: &independent,
                },
                region: None,
                ancestors: &[],
            },
            OriginalSourceViewDocument {
                publication: &b,
                document: OriginalDeclarationDocument {
                    uri: "file:///child.tcl",
                    source: child_source,
                    analysis: &child,
                },
                region: None,
                ancestors: &[],
            },
        ];
        let at = cursor(child_source);
        assert!(
            conditional_external_autoload_requests("file:///child.tcl", at, &documents[..1])
                .is_some(),
            "independent positive prerequisite"
        );
        let commands = tcl_compiler::registry_invocation::source_structure::source_native_procedure_body_commands(child_source, &child).expect("real inherited body inventory");
        assert!(
            commands.iter().any(|command| command
                .original_words()
                .first()
                .is_some_and(|head| head.span().start() <= at && at < head.span().end())),
            "incomplete owner genuinely claims clicked head"
        );
        assert!(
            conditional_external_autoload_requests("file:///child.tcl", at, &documents).is_none()
        );
    }
}
