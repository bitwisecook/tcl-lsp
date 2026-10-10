// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Readonly call edges from independently current original source declarations.

use crate::call_hierarchy::{CallHierarchyItem, IncomingCall, OutgoingCall};
use crate::original_declaration::{self, OriginalDeclarationIdentity, OriginalDeclarationRole};
use std::ops::ControlFlow;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::{LineIndex, Span};

/// Shared independently owned document carrier for readonly call advice.
pub type OriginalCallDocument<'a> = original_declaration::OriginalDeclarationDocument<'a>;

/// Render one current readonly source declaration as a hierarchy item.
#[must_use]
pub fn item(
    identity: &OriginalDeclarationIdentity,
    document: &OriginalCallDocument<'_>,
) -> Option<CallHierarchyItem> {
    if !identity.is_current(document.uri, document.source, document.analysis) {
        return None;
    }
    let body = body_span(identity, document.analysis).unwrap_or(identity.span());
    let index = LineIndex::new(document.source);
    Some(CallHierarchyItem {
        name: identity.label(),
        detail: Some("Source declaration".to_owned()),
        range: crate::definition::span_to_range(
            document.source,
            &index,
            Span::new(
                identity.span().start(),
                body.end().max(identity.span().end()),
            ),
        ),
        selection_range: crate::definition::span_to_range(document.source, &index, identity.span()),
        identity: Some(identity.clone()),
    })
}

/// Prepare a readonly source item using independent current document owners.
/// Original method and procedure lookup obligations are reselected centrally.
#[must_use]
pub fn prepare(
    uri: &str,
    line: u32,
    character: u32,
    documents: &[OriginalCallDocument<'_>],
    workspace: Option<&crate::workspace_index::WorkspaceIndex>,
) -> Vec<CallHierarchyItem> {
    let Some(document) = documents.iter().find(|doc| doc.uri == uri) else {
        return Vec::new();
    };
    let position = LineIndex::new(document.source);
    let offset = crate::definition::byte_offset_at(&position, document.source, line, character);
    if crate::definition::original_variable_cursor_retained(
        document.source,
        document.analysis,
        line,
        character,
    ) {
        return Vec::new();
    }
    let Some(rows) = original_declaration::declarations(uri, document.source, document.analysis)
    else {
        return Vec::new();
    };
    let mut declarations = rows
        .iter()
        .filter(|row| row.span().start() <= offset && offset < row.span().end());
    if let Some(first) = declarations.next() {
        return declarations
            .next()
            .is_none()
            .then(|| item(first, document))
            .flatten()
            .into_iter()
            .collect();
    }
    if let ControlFlow::Break(query) =
        crate::method_symbol::select(document.source, document.analysis, line, character)
    {
        let Some(query) = query else {
            return Vec::new();
        };
        let selected = if let Some(index) = workspace {
            crate::method_symbol::candidate(index, uri, &query)
        } else {
            crate::method_symbol::local_candidate(
                document.source,
                document.analysis,
                line,
                character,
            )
        };
        let ControlFlow::Break(Some(candidate)) = selected else {
            return Vec::new();
        };
        let mut targets = documents
            .iter()
            .filter(|owner| candidate.uri().is_empty() || owner.uri == candidate.uri())
            .filter_map(|owner| {
                OriginalDeclarationIdentity::for_member_candidate(
                    owner.uri,
                    owner.source,
                    owner.analysis,
                    &candidate,
                )
            });
        let Some(target) = targets.next() else {
            return Vec::new();
        };
        if targets.next().is_some() {
            return Vec::new();
        }
        return documents
            .iter()
            .find(|owner| owner.uri == target.uri())
            .and_then(|owner| item(&target, owner))
            .into_iter()
            .collect();
    }
    if crate::definition::original_variable_cursor_retained(
        document.source,
        document.analysis,
        line,
        character,
    ) {
        return Vec::new();
    }
    if let ControlFlow::Break(selected) =
        crate::math_function_symbol::select_at_offset(document.source, document.analysis, offset)
    {
        let Some(target) =
            selected.and_then(|selected| math_target(documents, document, selected.occurrence()))
        else {
            return Vec::new();
        };
        return documents
            .iter()
            .find(|owner| owner.uri == target.uri())
            .and_then(|owner| item(&target, owner))
            .into_iter()
            .collect();
    }
    let Some(invocation) = document
        .analysis
        .command_invocations
        .iter()
        .find(|inv| inv.range.start() <= offset && offset < inv.range.end())
    else {
        return Vec::new();
    };
    let Some(target) = procedure_target(documents, document, invocation) else {
        return Vec::new();
    };
    documents
        .iter()
        .find(|owner| owner.uri == target.uri())
        .and_then(|owner| item(&target, owner))
        .into_iter()
        .collect()
}

fn body_span(identity: &OriginalDeclarationIdentity, analysis: &AnalysisResult) -> Option<Span> {
    match identity.role() {
        OriginalDeclarationRole::Procedure => {
            Some(identity.procedure_metadata(analysis)?.metadata().body_span)
        }
        OriginalDeclarationRole::Method(_) => {
            Some(identity.method_metadata(analysis)?.metadata().body_span)
        }
        OriginalDeclarationRole::Special(_, _) => {
            Some(identity.special_metadata(analysis)?.body_word().span())
        }
        OriginalDeclarationRole::Document => {
            Some(Span::new(0, u32::try_from(identity.image().len()).ok()?))
        }
        OriginalDeclarationRole::Class | OriginalDeclarationRole::Property(_) => None,
    }
}

fn enclosing(
    document: &OriginalCallDocument<'_>,
    span: Span,
) -> Option<OriginalDeclarationIdentity> {
    let rows =
        original_declaration::declarations(document.uri, document.source, document.analysis)?;
    rows.into_iter()
        .filter_map(|identity| {
            let body = body_span(&identity, document.analysis)?;
            (body.start() <= span.start() && span.end() <= body.end())
                .then_some((body.len(), identity))
        })
        .min_by_key(|(length, _)| *length)
        .map(|(_, identity)| identity)
        .or_else(|| {
            OriginalDeclarationIdentity::for_document(
                document.uri,
                document.source,
                document.analysis,
            )
        })
}

fn procedure_target(
    documents: &[OriginalCallDocument<'_>],
    consumer: &OriginalCallDocument<'_>,
    invocation: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
) -> Option<OriginalDeclarationIdentity> {
    let (Some(input), Some(lookup)) =
        (&invocation.original_name_input, &invocation.original_lookup)
    else {
        return None;
    };
    if lookup.name_input() != input
        || lookup.site().source.source_image() != &tcl_lexer::SourceImage::document(consumer.source)
    {
        return None;
    }
    if invocation.resolved_command_reference.is_some() {
        let mut matches = documents.iter().flat_map(|document| {
            document
                .analysis
                .original_procedure_declarations()
                .filter_map(move |row| {
                    original_declaration::invocation_targets_declaration_in(
                        consumer.source,
                        consumer.analysis,
                        document.source,
                        document.analysis,
                        invocation,
                        row,
                        true,
                    )
                    .then(|| {
                        OriginalDeclarationIdentity::for_procedure(
                            document.uri,
                            document.source,
                            document.analysis,
                            row,
                        )
                    })
                    .flatten()
                })
        });
        let first = matches.next()?;
        return matches.next().is_none().then_some(first);
    }
    // A reached original lookup may offer a source declaration candidate.
    // Every class header occupies its slot too; ambiguous/earlier non-procedure
    // publication is terminal and never becomes a call to a later procedure.
    let headers = documents.iter().flat_map(|document| {
        document
            .analysis
            .original_procedure_declarations()
            .filter_map(move |row| {
                OriginalDeclarationIdentity::for_procedure(
                    document.uri,
                    document.source,
                    document.analysis,
                    row,
                )
                .map(|identity| (row.name(), identity))
            })
            .chain(
                document
                    .analysis
                    .original_class_declarations()
                    .filter_map(move |row| {
                        OriginalDeclarationIdentity::for_class(
                            document.uri,
                            document.source,
                            document.analysis,
                            row,
                        )
                        .map(|identity| (row.name(), identity))
                    }),
            )
    });
    let selected = lookup.matching_publications(headers)?;
    match selected.as_slice() {
        [one] if one.role() == OriginalDeclarationRole::Procedure => Some(one.clone()),
        _ => None,
    }
}

fn math_target(
    documents: &[OriginalCallDocument<'_>],
    consumer: &OriginalCallDocument<'_>,
    occurrence: &tcl_compiler::command_binding::OriginalMathFunctionOccurrence,
) -> Option<OriginalDeclarationIdentity> {
    let mut selected = documents.iter().flat_map(|owner| {
        owner
            .analysis
            .original_procedure_declarations()
            .filter_map(move |declaration| {
                original_declaration::math_function_targets_declaration_in(
                    consumer.source,
                    consumer.analysis,
                    owner.source,
                    owner.analysis,
                    occurrence,
                    declaration,
                    true,
                )
                .then(|| {
                    OriginalDeclarationIdentity::for_procedure(
                        owner.uri,
                        owner.source,
                        owner.analysis,
                        declaration,
                    )
                })
                .flatten()
            })
    });
    let first = selected.next()?;
    selected.next().is_none().then_some(first)
}

fn edges(
    documents: &[OriginalCallDocument<'_>],
    workspace: Option<&crate::workspace_index::WorkspaceIndex>,
) -> Vec<(
    OriginalDeclarationIdentity,
    OriginalDeclarationIdentity,
    Span,
)> {
    let mut edges = Vec::new();
    for document in documents {
        if original_declaration::declarations(document.uri, document.source, document.analysis)
            .is_none()
        {
            continue;
        }
        for invocation in &document.analysis.command_invocations {
            if !invocation.lookup.is_execution_site() {
                continue;
            }
            let Some(target) = procedure_target(documents, document, invocation) else {
                continue;
            };
            let Some(caller) = enclosing(document, invocation.range) else {
                continue;
            };
            let edge = (caller, target, invocation.range);
            if !edges.contains(&edge) {
                edges.push(edge);
            }
        }
        for occurrence in
            crate::math_function_symbol::occurrences(document.source, document.analysis)
                .into_iter()
                .flatten()
        {
            let Some(target) = math_target(documents, document, &occurrence) else {
                continue;
            };
            let Some(caller) = enclosing(document, occurrence.span()) else {
                continue;
            };
            let edge = (caller, target, occurrence.span());
            if !edges.contains(&edge) {
                edges.push(edge);
            }
        }
        for query in crate::method_symbol::source_queries(document.analysis) {
            if query.is_declaration() {
                continue;
            }
            let selected = if let Some(index) = workspace {
                crate::method_symbol::candidate(index, document.uri, &query)
            } else {
                crate::method_symbol::candidate_for_analysis(document.analysis, &query)
            };
            let ControlFlow::Break(Some(candidate)) = selected else {
                continue;
            };
            let mut targets = documents
                .iter()
                .filter(|owner| candidate.uri().is_empty() || owner.uri == candidate.uri())
                .filter_map(|owner| {
                    OriginalDeclarationIdentity::for_member_candidate(
                        owner.uri,
                        owner.source,
                        owner.analysis,
                        &candidate,
                    )
                });
            let Some(target) = targets.next() else {
                continue;
            };
            if targets.next().is_some() {
                continue;
            }
            let Some(caller) = enclosing(document, query.span()) else {
                continue;
            };
            let edge = (caller, target, query.span());
            if !edges.contains(&edge) {
                edges.push(edge);
            }
        }
    }
    edges
}

/// Incoming readonly source call edges, grouped by actual owning declaration.
/// Every supplied document must independently retain complete source currency.
#[must_use]
pub fn incoming(
    target: &OriginalDeclarationIdentity,
    documents: &[OriginalCallDocument<'_>],
    workspace: Option<&crate::workspace_index::WorkspaceIndex>,
) -> Vec<(String, IncomingCall)> {
    if !documents
        .iter()
        .any(|doc| target.is_current(doc.uri, doc.source, doc.analysis))
    {
        return Vec::new();
    }
    let mut groups: Vec<(OriginalDeclarationIdentity, Vec<Span>)> = Vec::new();
    for (caller, callee, span) in edges(documents, workspace) {
        if &callee != target {
            continue;
        }
        if let Some((_, spans)) = groups.iter_mut().find(|(previous, _)| previous == &caller) {
            spans.push(span);
        } else {
            groups.push((caller, vec![span]));
        }
    }
    groups
        .into_iter()
        .filter_map(|(caller, mut spans)| {
            let document = documents.iter().find(|doc| doc.uri == caller.uri())?;
            spans.sort_by_key(|span| (span.start(), span.end()));
            spans.dedup();
            let index = LineIndex::new(document.source);
            Some((
                caller.uri().to_owned(),
                IncomingCall {
                    from: item(&caller, document)?,
                    from_ranges: spans
                        .into_iter()
                        .map(|span| crate::definition::span_to_range(document.source, &index, span))
                        .collect(),
                },
            ))
        })
        .collect()
}

/// Outgoing readonly source call edges with independent target document owners.
#[must_use]
pub fn outgoing(
    caller: &OriginalDeclarationIdentity,
    documents: &[OriginalCallDocument<'_>],
    workspace: Option<&crate::workspace_index::WorkspaceIndex>,
) -> Vec<(String, OutgoingCall)> {
    let Some(consumer) = documents
        .iter()
        .find(|doc| caller.is_current(doc.uri, doc.source, doc.analysis))
    else {
        return Vec::new();
    };
    let mut groups: Vec<(OriginalDeclarationIdentity, Vec<Span>)> = Vec::new();
    for (from, target, span) in edges(documents, workspace) {
        if &from != caller {
            continue;
        }
        if let Some((_, spans)) = groups.iter_mut().find(|(previous, _)| previous == &target) {
            spans.push(span);
        } else {
            groups.push((target, vec![span]));
        }
    }
    groups
        .into_iter()
        .filter_map(|(target, mut spans)| {
            let document = documents.iter().find(|doc| doc.uri == target.uri())?;
            spans.sort_by_key(|span| (span.start(), span.end()));
            spans.dedup();
            let index = LineIndex::new(consumer.source);
            Some((
                target.uri().to_owned(),
                OutgoingCall {
                    to: item(&target, document)?,
                    from_ranges: spans
                        .into_iter()
                        .map(|span| crate::definition::span_to_range(consumer.source, &index, span))
                        .collect(),
                },
            ))
        })
        .collect()
}

/// Reference locations for one independently current source declaration.
/// Every consuming document retains its own lookup and complete source.
#[must_use]
pub fn references(
    identity: &OriginalDeclarationIdentity,
    documents: &[OriginalCallDocument<'_>],
    workspace: Option<&crate::workspace_index::WorkspaceIndex>,
) -> Vec<(String, Span)> {
    let Some(owner) = documents
        .iter()
        .find(|doc| identity.is_current(doc.uri, doc.source, doc.analysis))
    else {
        return Vec::new();
    };
    let duplicate_owner = documents
        .iter()
        .filter(|document| {
            let Some(rows) = original_declaration::declarations(
                document.uri,
                document.source,
                document.analysis,
            ) else {
                return false;
            };
            rows.iter().any(|row| {
                row.role() == identity.role()
                    && row.site() == identity.site()
                    && row.image() == identity.image()
                    && row.config() == identity.config()
            })
        })
        .count()
        != 1;
    if duplicate_owner {
        return Vec::new();
    }
    let mut locations = Vec::new();
    match identity.role() {
        OriginalDeclarationRole::Procedure => {
            if identity.procedure_metadata(owner.analysis).is_none() {
                return Vec::new();
            }
            for document in documents {
                for invocation in &document.analysis.command_invocations {
                    if procedure_target(documents, document, invocation).as_ref() == Some(identity)
                    {
                        locations.push((document.uri.to_owned(), invocation.range));
                    }
                }
                for occurrence in
                    crate::math_function_symbol::occurrences(document.source, document.analysis)
                        .into_iter()
                        .flatten()
                {
                    if math_target(documents, document, &occurrence).as_ref() == Some(identity) {
                        locations.push((document.uri.to_owned(), occurrence.span()));
                    }
                }
            }
        }
        OriginalDeclarationRole::Class => {
            let Some(declaration) = identity.class_metadata(owner.analysis) else {
                return Vec::new();
            };
            for document in documents {
                for invocation in &document.analysis.command_invocations {
                    if original_declaration::invocation_targets_declaration_in(
                        document.source,
                        document.analysis,
                        owner.source,
                        owner.analysis,
                        invocation,
                        declaration,
                        true,
                    ) {
                        locations.push((document.uri.to_owned(), invocation.range));
                    }
                }
            }
        }
        OriginalDeclarationRole::Method(_) => {
            for (caller, target, span) in edges(documents, workspace) {
                if target == *identity {
                    locations.push((caller.uri().to_owned(), span));
                }
            }
        }
        OriginalDeclarationRole::Property(_)
        | OriginalDeclarationRole::Special(_, _)
        | OriginalDeclarationRole::Document => {}
    }
    locations.sort_by(|a, b| (&a.0, a.1.start(), a.1.end()).cmp(&(&b.0, b.1.start(), b.1.end())));
    locations.dedup();
    locations
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_math_navigation_and_reference_edges_share_expression_allocations() {
        // Implementation contract: naming.core.original-math-function-selection
        // docs/design/analysis/name-resolution-proofs/core-original-math-function-selection.md
        let source = "proc ::tcl::mathfunc::abs {argument} {return OVERRIDE}\nexpr {abs(-2)}\n";
        for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut analysis = Analyser::new().analyse(source, dialect);
            analysis.all_procs.clear();
            analysis.command_invocations.clear();
            let offset = u32::try_from(source.rfind("abs(").unwrap()).unwrap();
            let position = LineIndex::new(source).position_at_utf16(offset, source);
            let documents = [OriginalCallDocument {
                uri: "file:///functions.tcl",
                source,
                analysis: &analysis,
            }];
            let prepared = prepare(
                documents[0].uri,
                position.line,
                position.character.get(),
                &documents,
                None,
            );
            assert_eq!(prepared.len(), 1, "{dialect}");
            let identity = prepared[0].identity.as_ref().unwrap();
            let occurrence = crate::math_function_symbol::at(source, &analysis, offset).unwrap();
            assert_eq!(
                references(identity, &documents, None),
                vec![(documents[0].uri.to_owned(), occurrence.span())],
                "{dialect}"
            );
            assert_eq!(incoming(identity, &documents, None).len(), 1);
            assert_eq!(
                original_declaration::reference_spans(identity, source, &analysis, true),
                vec![occurrence.span()]
            );
            assert_eq!(
                crate::definition::definition(
                    source,
                    position.line,
                    position.character.get(),
                    &analysis,
                ),
                vec![crate::definition::span_to_range(
                    source,
                    &LineIndex::new(source),
                    identity.span(),
                )]
            );
            let duplicates = [
                documents[0],
                OriginalCallDocument {
                    uri: "file:///copy.tcl",
                    ..documents[0]
                },
            ];
            assert!(
                prepare(
                    documents[0].uri,
                    position.line,
                    position.character.get(),
                    &duplicates,
                    None,
                )
                .is_empty()
            );
            assert!(references(identity, &duplicates, None).is_empty());
            assert!(crate::math_function_symbol::at("expr {abs(2)}", &analysis, 6).is_none());
        }
    }

    #[test]
    fn original_call_edges_keep_opaque_declarations_and_independent_source_owners() {
        // Implementation contract: naming.consumer.original-call-hierarchy
        // docs/design/analysis/name-resolution-proofs/original-call-hierarchy.md
        let source = r"proc p\uD800 {} {}; p\uD800; proc p\uD801 {} {}; p\uD801";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.all_procs.clear();
        let documents = [OriginalCallDocument {
            uri: "file:///a.tcl",
            source,
            analysis: &analysis,
        }];
        let identities =
            original_declaration::declarations(documents[0].uri, source, &analysis).unwrap();
        assert_eq!(identities.len(), 2);
        for target in &identities {
            let incoming = incoming(target, &documents, None);
            assert_eq!(incoming.len(), 1, "{incoming:?}");
            assert_eq!(incoming[0].1.from_ranges.len(), 1);
            assert_eq!(
                incoming[0].1.from.identity.as_ref().unwrap().role(),
                OriginalDeclarationRole::Document
            );
            assert_eq!(references(target, &documents, None).len(), 1);
        }
        let stale = [OriginalCallDocument {
            uri: documents[0].uri,
            source: "proc changed {} {}",
            analysis: &analysis,
        }];
        assert!(incoming(&identities[0], &stale, None).is_empty());
        let foreign_owner = [OriginalCallDocument {
            uri: "file:///copy.tcl",
            source,
            analysis: &analysis,
        }];
        assert!(outgoing(&identities[0], &foreign_owner, None).is_empty());
        analysis
            .command_invocations
            .iter_mut()
            .for_each(|invocation| invocation.original_lookup = None);
        let missing = [OriginalCallDocument {
            uri: "file:///a.tcl",
            source,
            analysis: &analysis,
        }];
        assert!(incoming(&identities[0], &missing, None).is_empty());
    }
}

#[cfg(test)]
mod own_object_call_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_own_object_calls_and_references_keep_the_canonical_object_worker() {
        // Implementation contract: naming.editor.original-own-object-member-navigation
        // docs/design/analysis/name-resolution-proofs/original-own-object-member-navigation.md
        let source = r"oo::class create C {method p\uD800 {} {}}; C create object; oo::objdefine object method p\uD800 {} {}; object p\uD800";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.all_classes.clear();
        let documents = [OriginalCallDocument {
            uri: "file:///a.tcl",
            source,
            analysis: &analysis,
        }];
        let offset = u32::try_from(source.rfind(r"p\uD800").unwrap()).unwrap();
        let position = LineIndex::new(source).position_at_utf16(offset, source);
        let prepared = prepare(
            "file:///a.tcl",
            position.line,
            position.character.get(),
            &documents,
            None,
        );
        let [prepared] = prepared.as_slice() else {
            panic!("one actual own method");
        };
        let identity = prepared.identity.as_ref().unwrap();
        assert!(identity.class_metadata(&analysis).is_none());
        assert_eq!(incoming(identity, &documents, None).len(), 1);
        assert_eq!(references(identity, &documents, None).len(), 1);
        let changed = format!("{source} ");
        let stale = [OriginalCallDocument {
            uri: "file:///a.tcl",
            source: &changed,
            analysis: &analysis,
        }];
        assert!(incoming(identity, &stale, None).is_empty());
    }
}
