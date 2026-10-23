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
    let index = LineIndex::new_lsp(document.source);
    Some(CallHierarchyItem {
        name: if matches!(identity.role(), OriginalDeclarationRole::Method(_)) {
            // The selected declaration retains its side independently. Internal
            // member keys include that side for identity; CALL presentation has
            // always shown the actual class spelling followed by the member.
            identity.class_metadata(document.analysis).map_or_else(
                || identity.label(),
                |class| {
                    let owner = tcl_syntax::native_string::resident_name_label(
                        &tcl_syntax::naming::native_command_full_name_bytes(class.name().slot()),
                    );
                    format!("{owner}::{}", identity.label())
                },
            )
        } else {
            identity.label()
        },
        detail: original_item_detail(identity, document.analysis),
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

/// The item detail the lexical provider renders for the same declaration:
/// `(N params)` for a procedure with parameters (none without), and
/// `KIND of CLASS (N params)` for a class member. A declaration whose
/// current metadata is unavailable keeps the generic source label.
fn original_item_detail(
    identity: &OriginalDeclarationIdentity,
    analysis: &tcl_compiler::analyser::AnalysisResult,
) -> Option<String> {
    if let Some(procedure) = identity.procedure_metadata(analysis) {
        let params = procedure.metadata().params.len();
        return (params > 0).then(|| format!("({params} params)"));
    }
    if let (Some(method), Some(class)) = (
        identity.method_metadata(analysis),
        identity.class_metadata(analysis),
    ) {
        let method = method.metadata();
        return Some(format!(
            "{} of {} ({} params)",
            method.kind,
            class.metadata().qualified_name,
            method.params.len(),
        ));
    }
    Some("Source declaration".to_owned())
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
    let Some(position) = crate::original_context::CurrentSourcePosition::capture(
        document.source,
        document.analysis,
        line,
        character,
    ) else {
        return Vec::new();
    };
    let offset = position.offset();
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
    if let ControlFlow::Break(items) =
        prepare_method_item(uri, document, (line, character), documents, workspace)
    {
        return items;
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
    let Some(calls) = OriginalProcedureCalls::capture(*document) else {
        return Vec::new();
    };
    let Some(command) = calls.at_cursor(offset) else {
        return Vec::new();
    };
    let Some(head) = command.argv.first() else {
        return Vec::new();
    };
    let mut invocations = document
        .analysis
        .command_invocations
        .iter()
        .filter(|invocation| invocation.range == head.span);
    let Some(invocation) = invocations.next() else {
        return Vec::new();
    };
    if invocations.next().is_some() {
        return Vec::new();
    }
    let Some(target) = procedure_target(documents, &calls, invocation) else {
        return Vec::new();
    };
    documents
        .iter()
        .find(|owner| owner.uri == target.uri())
        .and_then(|owner| item(&target, owner))
        .into_iter()
        .collect()
}

fn prepare_method_item(
    uri: &str,
    document: &OriginalCallDocument<'_>,
    position: (u32, u32),
    documents: &[OriginalCallDocument<'_>],
    workspace: Option<&crate::workspace_index::WorkspaceIndex>,
) -> ControlFlow<Vec<CallHierarchyItem>> {
    let (line, character) = position;
    let ControlFlow::Break(query) =
        crate::method_symbol::select(document.source, document.analysis, line, character)
    else {
        return ControlFlow::Continue(());
    };

    let Some(query) = query else {
        return ControlFlow::Break(Vec::new());
    };
    let selected = if let Some(index) = workspace {
        crate::method_symbol::candidate(index, uri, &query)
    } else {
        crate::method_symbol::local_candidate(document.source, document.analysis, line, character)
    };
    let ControlFlow::Break(Some(candidate)) = selected else {
        return ControlFlow::Break(Vec::new());
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
        return ControlFlow::Break(Vec::new());
    };
    if targets.next().is_some() {
        return ControlFlow::Break(Vec::new());
    }
    ControlFlow::Break(
        documents
            .iter()
            .find(|owner| owner.uri == target.uri())
            .and_then(|owner| item(&target, owner))
            .into_iter()
            .collect(),
    )
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

struct OriginalProcedureCalls<'a> {
    document: OriginalCallDocument<'a>,
    walk: crate::refactor::FrameWalk<'a>,
    commands: Vec<tcl_compiler::segmenter::SegmentedCommand>,
}
impl<'a> OriginalProcedureCalls<'a> {
    fn capture(document: OriginalCallDocument<'a>) -> Option<Self> {
        let walk = crate::refactor::FrameWalk::new(document.source, document.analysis)?;
        let commands = crate::source_structure::SourceSyntaxStructure::capture_commands(
            document.source,
            document.analysis,
        )?;
        Some(Self {
            document,
            walk,
            commands,
        })
    }
    fn at_cursor(&self, cursor: u32) -> Option<&tcl_compiler::segmenter::SegmentedCommand> {
        let mut selected = self.commands.iter().filter(|command| {
            crate::refactor::FrameWalk::native_words(&self.walk, self.document.source, command)
                .and_then(|words| words.first().map(tcl_lexer::NativeWord::span))
                .is_some_and(|span| span.start() <= cursor && cursor < span.end())
        });
        let command = selected.next()?;
        selected.all(|other| other == command).then_some(command)
    }
    fn tokens(
        &self,
        invocation: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    ) -> Option<tcl_compiler::ir::CommandTokens> {
        let lookup = invocation.original_lookup.as_ref()?;
        let mut commands = self.commands.iter().filter(|command| {
            command.argv.first().is_some_and(|head| {
                head.span == invocation.range && head.span.start() == lookup.site().offset
            })
        });
        let command = commands.next()?;
        commands.all(|other| other == command).then_some(())?;
        Some(crate::refactor::FrameWalk::tokens(
            &self.walk,
            self.document.source,
            command,
        ))
    }
}

fn procedure_target(
    documents: &[OriginalCallDocument<'_>],
    calls: &OriginalProcedureCalls<'_>,
    invocation: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
) -> Option<OriginalDeclarationIdentity> {
    let consumer = &calls.document;
    let tokens = calls.tokens(invocation)?;
    let binding = tokens.source_binding.as_ref()?;
    let actual_input = binding.original_head_name_input(&tokens)?;
    let actual_lookup = binding.original_command_lookup(&tokens, &actual_input)?;
    if invocation.original_name_input.as_ref() != Some(&actual_input)
        || invocation.original_lookup.as_ref() != Some(&actual_lookup)
        || invocation.resolved_command_reference != binding.original_command_reference(&tokens)
    {
        return None;
    }
    for document in documents {
        crate::original_context::CurrentSourceContext::capture(document.source, document.analysis)?;
    }
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
        return resolved_procedure_target(documents, consumer, invocation);
    }
    // Positioned source advice preserves occupants and unknown observations.
    // Header coordinates alone cannot supply the missing reference.
    let registry = consumer.analysis.resolved_registry()?;
    let headers = documents
        .iter()
        .flat_map(|document| {
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
                    .map(|identity| (row.name(), row.declaration_site(), identity))
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
                            .map(|identity| (row.name(), row.declaration_site(), identity))
                        }),
                )
        })
        .collect::<Vec<_>>();
    let selected = lookup.matching_source_advice_publications(
        registry,
        headers
            .iter()
            .map(|(name, site, identity)| (*name, *site, identity.clone())),
    )?;
    let selected = if selected.is_empty() {
        lookup.matching_external_source_advice_publications(
            registry,
            headers
                .into_iter()
                .filter(|(_, _, identity)| identity.image() != lookup.site().source.source_image())
                .map(|(name, _, identity)| (name, identity)),
        )?
    } else {
        selected
    };
    match selected.as_slice() {
        [one] if one.role() == OriginalDeclarationRole::Procedure => Some(one.clone()),
        _ => None,
    }
}

fn resolved_procedure_target(
    documents: &[OriginalCallDocument<'_>],
    consumer: &OriginalCallDocument<'_>,
    invocation: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
) -> Option<OriginalDeclarationIdentity> {
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
    matches.next().is_none().then_some(first)
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

// Conditional source relations are issued at the original call horizon.
// They do not renew the independent Native execution-site branch.
fn source_procedure_edges(
    document: &OriginalCallDocument<'_>,
) -> Vec<(
    OriginalDeclarationIdentity,
    OriginalDeclarationIdentity,
    Span,
)> {
    let mut edges = Vec::new();
    for call in
        tcl_compiler::registry_invocation::source_structure::source_original_procedure_calls(
            document.source,
            document.analysis,
        )
        .into_iter()
        .flatten()
    {
        // Earlier unknown mutations and pre-lookup operand effects are
        // retained debts of this conditional relation; they may delete or
        // replace the header, so they cannot issue a call edge.
        if !call.uncertain_operations().is_empty() || call.has_lookup_operand_effects() {
            continue;
        }
        let Some((caller_record, target_record)) = call
            .caller_declaration(document.analysis)
            .zip(call.declaration(document.analysis))
        else {
            continue;
        };
        let Some((caller, target)) = OriginalDeclarationIdentity::for_procedure(
            document.uri,
            document.source,
            document.analysis,
            caller_record,
        )
        .zip(OriginalDeclarationIdentity::for_procedure(
            document.uri,
            document.source,
            document.analysis,
            target_record,
        )) else {
            continue;
        };
        let Some(head) = call.original_words().first() else {
            continue;
        };
        let edge = (caller, target, head.span());
        if !edges.contains(&edge) {
            edges.push(edge);
        }
    }
    edges
}

// Direct `my` selectors inside authentic member bodies are conditional source
// relations retained by the compiler's member-body catalogue. Body entry,
// receiver, lookup and inheritance obligations stay with that relation.
fn source_self_method_edges(
    document: &OriginalCallDocument<'_>,
) -> Vec<(
    OriginalDeclarationIdentity,
    OriginalDeclarationIdentity,
    Span,
)> {
    let mut edges = Vec::new();
    for reference in
        tcl_compiler::analyser::class_hierarchy::original_metadata::original_source_self_method_references(
            document.source,
            document.analysis,
        )
        .into_iter()
        .flatten()
    {
        let Some((caller, target)) = OriginalDeclarationIdentity::for_method(
            document.uri,
            document.source,
            document.analysis,
            reference.body_class(),
            reference.body_method(),
        )
        .zip(OriginalDeclarationIdentity::for_method(
            document.uri,
            document.source,
            document.analysis,
            reference.declaring_class(),
            reference.method(),
        )) else {
            continue;
        };
        let edge = (caller, target, reference.selector_span());
        if !edges.contains(&edge) {
            edges.push(edge);
        }
    }
    edges
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
        let calls = OriginalProcedureCalls::capture(*document);
        for invocation in &document.analysis.command_invocations {
            let Some(calls) = calls.as_ref() else {
                continue;
            };
            if !invocation.lookup.is_execution_site() {
                continue;
            }
            let Some(target) = procedure_target(documents, calls, invocation) else {
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
        for edge in source_procedure_edges(document) {
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

// Call-hierarchy edges add direct `my` selector relations. Member reference
// inventories stay with the readonly member report, so `references` keeps the
// plain edge set.
fn hierarchy_edges(
    documents: &[OriginalCallDocument<'_>],
    workspace: Option<&crate::workspace_index::WorkspaceIndex>,
) -> Vec<(
    OriginalDeclarationIdentity,
    OriginalDeclarationIdentity,
    Span,
)> {
    let mut edges = edges(documents, workspace);
    for document in documents {
        for edge in source_self_method_edges(document) {
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
    for (caller, callee, span) in hierarchy_edges(documents, workspace) {
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
            let index = LineIndex::new_lsp(document.source);
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
    for (from, target, span) in hierarchy_edges(documents, workspace) {
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
            let index = LineIndex::new_lsp(consumer.source);
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
                let calls = OriginalProcedureCalls::capture(*document);
                for invocation in &document.analysis.command_invocations {
                    let Some(calls) = calls.as_ref() else {
                        continue;
                    };
                    if procedure_target(documents, calls, invocation).as_ref() == Some(identity) {
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

#[cfg(test)]
mod positioned_procedure_advice_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn call<'a>(
        analysis: &'a AnalysisResult,
        source: &str,
        written: &str,
    ) -> &'a tcl_compiler::signature_scan::types::SignatureCommandInvocation {
        let offset = u32::try_from(source.rfind(written).unwrap()).unwrap();
        analysis
            .command_invocations
            .iter()
            .find(|invocation| invocation.range.start() == offset)
            .unwrap()
    }

    #[test]
    fn original_call_fallback_uses_genuine_positioned_source_advice_for_deferred_headers() {
        let source = "proc caller {} {target value}";
        let provider_source = "proc target {value} {}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let provider = Analyser::new().analyse(provider_source, "tcl8.6");
        let documents = [
            OriginalCallDocument {
                uri: "file:///caller.tcl",
                source,
                analysis: &analysis,
            },
            OriginalCallDocument {
                uri: "file:///target.tcl",
                source: provider_source,
                analysis: &provider,
            },
        ];
        let invocation = call(&analysis, source, "target value");
        assert!(invocation.resolved_command_reference.is_none());
        assert!(invocation.original_lookup.is_some());
        let calls = OriginalProcedureCalls::capture(documents[0]).unwrap();
        let selected = procedure_target(&documents, &calls, invocation).unwrap();
        assert_eq!(selected.uri(), "file:///target.tcl");
        assert_eq!(selected.role(), OriginalDeclarationRole::Procedure);
        assert_eq!(selected.input().unwrap().bytes(), b"target");
        let mut presentation = invocation.clone();
        presentation.name = "untrusted report".into();
        presentation.resolved_qualified_name = Some("::different".into());
        assert_eq!(
            procedure_target(&documents, &calls, &presentation),
            Some(selected)
        );
    }

    #[test]
    fn original_call_fallback_does_not_restore_deleted_or_unknown_occupied_headers() {
        for source in [
            "proc ::target args {}; rename ::target {}; proc caller {} {::target value}",
            "proc ::target args {}; proc caller {} {mystery; ::target value}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let documents = [OriginalCallDocument {
                uri: "file:///caller.tcl",
                source,
                analysis: &analysis,
            }];
            let invocation = call(&analysis, source, "::target value");
            assert!(invocation.resolved_command_reference.is_none(), "{source}");
            let lookup = invocation.original_lookup.as_ref().unwrap();
            let geometric = lookup
                .matching_publications(
                    analysis
                        .original_procedure_declarations()
                        .map(|row| (row.name(), row.declaration_site().clone())),
                )
                .unwrap();
            assert!(
                !geometric.is_empty(),
                "the old geometric fallback could nominate this header: {source}"
            );
            let calls = OriginalProcedureCalls::capture(documents[0]).unwrap();
            assert!(
                procedure_target(&documents, &calls, invocation).is_none(),
                "{source}"
            );
            let target = analysis
                .original_procedure_declarations()
                .find(|row| row.name().slot().simple.as_bytes() == b"target")
                .unwrap();
            let identity = OriginalDeclarationIdentity::for_procedure(
                documents[0].uri,
                source,
                &analysis,
                target,
            )
            .unwrap();
            assert!(incoming(&identity, &documents, None).is_empty(), "{source}");
            assert!(
                references(&identity, &documents, None).is_empty(),
                "{source}"
            );
        }
    }

    #[test]
    fn original_call_prepare_requires_written_head_and_strict_current_position() {
        let source = "proc target {value} {}; target target";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let documents = [OriginalCallDocument {
            uri: "file:///calls.tcl",
            source,
            analysis: &analysis,
        }];
        let head = u32::try_from(source.find("target target").unwrap()).unwrap();
        assert_eq!(
            prepare(documents[0].uri, 0, head, &documents, None).len(),
            1
        );
        let argument = u32::try_from(source.rfind("target").unwrap()).unwrap();
        assert!(prepare(documents[0].uri, 0, argument, &documents, None).is_empty());
        assert!(prepare(documents[0].uri, u32::MAX, 0, &documents, None).is_empty());
        let stale = [OriginalCallDocument {
            source: "proc changed {} {}",
            ..documents[0]
        }];
        assert!(prepare(documents[0].uri, 0, head, &stale, None).is_empty());
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        let missing = [OriginalCallDocument {
            analysis: &missing,
            ..documents[0]
        }];
        assert!(prepare(documents[0].uri, 0, head, &missing, None).is_empty());
    }

    #[test]
    fn original_call_lookup_projection_must_match_the_genuine_current_call() {
        let source = "proc target {} {}; target";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let foreign = Analyser::new().analyse(source, "tcl9.0");
        let documents = [OriginalCallDocument {
            uri: "file:///calls.tcl",
            source,
            analysis: &analysis,
        }];
        let invocation = call(&analysis, source, "target").clone();
        let calls = OriginalProcedureCalls::capture(documents[0]).unwrap();
        assert!(procedure_target(&documents, &calls, &invocation).is_some());
        let mut changed = invocation.clone();
        let foreign_call = foreign.command_invocations.last().unwrap();
        changed
            .original_name_input
            .clone_from(&foreign_call.original_name_input);
        changed
            .original_lookup
            .clone_from(&foreign_call.original_lookup);
        changed
            .resolved_command_reference
            .clone_from(&foreign_call.resolved_command_reference);
        assert!(procedure_target(&documents, &calls, &changed).is_none());
        let mut missing = invocation;
        missing.original_lookup = None;
        assert!(procedure_target(&documents, &calls, &missing).is_none());
    }

    #[test]
    fn original_call_ranges_follow_lsp_lone_cr_positions_after_exact_selection() {
        let source = "proc target {} {};\rtarget";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let documents = [OriginalCallDocument {
            uri: "file:///calls.tcl",
            source,
            analysis: &analysis,
        }];
        let selected = prepare(documents[0].uri, 1, 0, &documents, None);
        assert_eq!(selected.len(), 1);
        let identity = selected[0].identity.as_ref().unwrap();
        let incoming = incoming(identity, &documents, None);
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0].1.from_ranges[0].start_line, 1);
        assert_eq!(incoming[0].1.from_ranges[0].end_line, 1);
    }
}

#[cfg(test)]
mod positioned_procedure_strong_reference_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_call_reference_keeps_local_alias_target_and_builtin_occupancy_terminal() {
        for (source, header, selected) in [
            (
                "proc local args {}; interp alias {} forwarded {} local; forwarded value",
                "proc forwarded args {}",
                Some(b"local".as_slice()),
            ),
            (
                "proc target args {}; target value",
                "proc target args {}",
                Some(b"target".as_slice()),
            ),
            ("puts value", "proc puts args {}", None),
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let provider = Analyser::new().analyse(header, "tcl8.6");
            let documents = [
                OriginalCallDocument {
                    uri: "file:///caller.tcl",
                    source,
                    analysis: &analysis,
                },
                OriginalCallDocument {
                    uri: "file:///foreign.tcl",
                    source: header,
                    analysis: &provider,
                },
            ];
            let invocation = analysis.command_invocations.last().unwrap();
            assert!(invocation.resolved_command_reference.is_some(), "{source}");
            let calls = OriginalProcedureCalls::capture(documents[0]).unwrap();
            let actual = procedure_target(&documents, &calls, invocation);
            assert_eq!(
                actual
                    .as_ref()
                    .and_then(|identity| identity.input())
                    .map(tcl_compiler::signature_scan::scope::SignatureSourceNameInput::bytes),
                selected,
                "{source}"
            );
            if let Some(identity) = actual {
                assert_eq!(identity.uri(), documents[0].uri);
            }
        }
    }
}

#[cfg(test)]
mod external_procedure_advice_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_root_call_reborrows_head_and_conditional_external_header() {
        for (source, provider_source) in [
            ("shared_helper\n", "proc shared_helper {} {}\n"),
            ("shared_helper retained\n", "proc shared_helper args {}\n"),
        ] {
            let provider = Analyser::new().analyse(provider_source, "tcl8.6");
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let documents = [
                OriginalCallDocument {
                    uri: "file:///main.tcl",
                    source,
                    analysis: &analysis,
                },
                OriginalCallDocument {
                    uri: "file:///lib.tcl",
                    source: provider_source,
                    analysis: &provider,
                },
            ];
            let calls = OriginalProcedureCalls::capture(documents[0]).unwrap();
            let command = calls.at_cursor(0).unwrap();
            let invocation = analysis.command_invocations.first().unwrap();
            assert_eq!(invocation.range, command.argv[0].span);
            if command.argv.len() > 1 {
                assert_ne!(invocation.range, command.span);
            }
            assert!(invocation.resolved_command_reference.is_none());
            assert!(calls.tokens(invocation).is_some());
            let selected = prepare(documents[0].uri, 0, 0, &documents, None);
            assert_eq!(selected.len(), 1, "{source}");
            let identity = selected[0].identity.as_ref().unwrap();
            assert_eq!(identity.uri(), documents[1].uri);
            assert!(identity.is_current(documents[1].uri, provider_source, &provider));
            assert_eq!(identity.input().unwrap().bytes(), b"shared_helper");
            let duplicates = [
                documents[0],
                documents[1],
                OriginalCallDocument {
                    uri: "file:///other-lib.tcl",
                    ..documents[1]
                },
            ];
            assert!(prepare(documents[0].uri, 0, 0, &duplicates, None).is_empty());
            if command.argv.len() > 1 {
                let mut whole_command = invocation.clone();
                whole_command.range = command.span;
                assert!(procedure_target(&documents, &calls, &whole_command).is_none());
            }
            let stale = [
                documents[0],
                OriginalCallDocument {
                    source: "proc changed args {}\n",
                    ..documents[1]
                },
            ];
            assert!(prepare(documents[0].uri, 0, 0, &stale, None).is_empty());
        }
    }

    #[test]
    fn external_root_header_preserves_occupants_deletions_and_unknowns() {
        for (source, provider_source) in [
            ("list retained", "proc list args {}"),
            (
                "interp alias {} shared_helper {} list; shared_helper retained",
                "proc shared_helper args {}",
            ),
            (
                "proc shared_helper args {}; rename shared_helper {}; shared_helper retained",
                "proc shared_helper args {}",
            ),
            (
                "mystery; shared_helper retained",
                "proc shared_helper args {}",
            ),
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let provider = Analyser::new().analyse(provider_source, "tcl8.6");
            let documents = [
                OriginalCallDocument {
                    uri: "file:///main.tcl",
                    source,
                    analysis: &analysis,
                },
                OriginalCallDocument {
                    uri: "file:///lib.tcl",
                    source: provider_source,
                    analysis: &provider,
                },
            ];
            let invocation = analysis.command_invocations.last().unwrap();
            let position =
                LineIndex::new_lsp(source).position_at_utf16(invocation.range.start(), source);
            assert!(
                prepare(
                    documents[0].uri,
                    position.line,
                    position.character.get(),
                    &documents,
                    None
                )
                .is_empty(),
                "{source}"
            );
        }
        let source = "shared_helper retained; proc shared_helper args {}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let documents = [OriginalCallDocument {
            uri: "file:///main.tcl",
            source,
            analysis: &analysis,
        }];
        assert!(prepare(documents[0].uri, 0, 0, &documents, None).is_empty());
    }
}
