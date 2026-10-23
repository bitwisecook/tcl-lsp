// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Intentional mirrored source spelling, independently of semantic rename.

use super::{OriginalDeclarationDocument, OriginalDeclarationIdentity};
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::{SourceImage, Span};

/// One current declaration and its observed direct self-call source components.
/// A future name is not supplied by linked editing. The permitted operation is
/// intentional source spelling authoring; prospective collisions, observers and
/// complete rename equivalence remain unproved and require the rename planner.
pub(crate) struct OriginalLinkedSourceSpelling<'a> {
    spans: Vec<Span>,
    text: &'a str,
}

impl<'a> OriginalLinkedSourceSpelling<'a> {
    pub(crate) fn capture(
        source: &'a str,
        analysis: &AnalysisResult,
        line: u32,
        character: u32,
    ) -> Option<Self> {
        // naming.consumer.original-linked-editing

        let position = crate::original_context::CurrentSourcePosition::capture(
            source, analysis, line, character,
        )?;
        let cursor = position.offset();
        let image = SourceImage::document(source);
        if analysis
            .original_variable_root_in_source(&image, position.context().config(), cursor)
            .is_some()
        {
            return None;
        }
        let mut candidates = Vec::new();
        if analysis.allows_lexical_declaration_advice() {
            let input = analysis.resolved_input.as_ref()?;
            for declaration in analysis.logical_source_procedure_declarations() {
                if !declaration.matches_source(&image, input) {
                    continue;
                }
                let word = declaration.original_name_word()?;
                let Some(extent) = crate::original_name_edit::original_plain_source_command_tail(
                    source, analysis, word,
                ) else {
                    continue;
                };
                let text = source.get(extent.as_range())?;
                if !plain_component(text) {
                    continue;
                }
                let mut spans = vec![extent];
                for call in tcl_compiler::registry_invocation::source_structure::source_logical_procedure_calls(source, analysis)? {
                    let Some(head) = call.original_words().first() else {
                        continue;
                    };
                    if !contains(declaration.metadata().body_span, head.span().start()) {
                        continue;
                    }
                    // This intentional spelling group retains a conditional source
                    // link; runtime applicability belongs to semantic rename.
                    if call.declaration(analysis) != Some(declaration)
                        || !call.lineage().is_empty()
                        || call.captured_argument_count() != 0
                    {
                        continue;
                    }
                    let Some(head) = call.original_words().first() else {
                        continue;
                    };
                    if let Some(span) =
                        crate::original_name_edit::original_plain_source_command_tail(
                            source, analysis, head,
                        )
                        && source.get(span.as_range()) == Some(text)
                    {
                        spans.push(span);
                    }
                }
                if spans.len() > 1 && spans.iter().any(|span| contains(*span, cursor)) {
                    candidates.push(Self { spans, text });
                }
            }
        } else {
            Self::append_native_candidates(source, analysis, cursor, &mut candidates)?;
        }
        if candidates.len() != 1 {
            return None;
        }
        candidates.pop()
    }

    fn append_native_candidates(
        source: &'a str,
        analysis: &AnalysisResult,
        cursor: u32,
        candidates: &mut Vec<Self>,
    ) -> Option<()> {
        // This local owner is request-scoped; it is never a wire locator or
        // evidence that another equal-byte document has the same identity.
        let document = OriginalDeclarationDocument {
            uri: "",
            source,
            analysis,
        };
        for declaration in analysis.original_procedure_declarations() {
            let identity = OriginalDeclarationIdentity::for_procedure(
                document.uri,
                source,
                analysis,
                declaration,
            )?;
            let input = identity.input()?;
            let extent = crate::original_name_edit::original_static_name_value_span(
                input,
                input.policy().recipe().command_tail_extent(input.bytes())?,
            )?;
            let text = source.get(extent.as_range())?;
            if !plain_component(text) {
                continue;
            }
            let mut spans = vec![extent];
            for invocation in &analysis.command_invocations {
                if invocation.indirect
                    || !invocation.lookup.is_execution_site()
                    || !contains(declaration.metadata().body_span, invocation.range.start())
                    || !crate::original_declaration::invocation_targets_declaration(
                        source,
                        analysis,
                        invocation,
                        declaration,
                        false,
                    )
                {
                    continue;
                }
                let Some(input) = &invocation.original_name_input else {
                    continue;
                };
                let Some(component) = input.policy().recipe().command_tail_extent(input.bytes())
                else {
                    continue;
                };
                let Some(span) =
                    crate::original_name_edit::original_static_name_value_span(input, component)
                else {
                    continue;
                };
                if source.get(span.as_range()) == Some(text) {
                    spans.push(span);
                }
            }
            if spans.len() > 1 && spans.iter().any(|span| contains(*span, cursor)) {
                // Existing atomic preparation independently closes current
                // publication, source coverage and observer/link obligations.
                // Its current-tail proposal proves no future-name collision.
                super::original_command_prepare_span(document, &identity, cursor).ok()?;
                candidates.push(Self { spans, text });
            }
        }
        Some(())
    }

    pub(crate) fn spans(&self) -> &[Span] {
        &self.spans
    }

    pub(crate) fn text(&self) -> &str {
        self.text
    }
}

fn contains(span: Span, offset: u32) -> bool {
    span.start() <= offset && offset < span.end()
}

fn plain_component(text: &str) -> bool {
    !text.is_empty()
        && !text.chars().any(|character| {
            character.is_whitespace()
                || character.is_control()
                || matches!(
                    character,
                    '\\' | '{' | '}' | '[' | ']' | ';' | '$' | '"' | ':' | '#'
                )
        })
        && (!text.is_ascii()
            || text
                .as_bytes()
                .first()
                .is_some_and(|first| first.is_ascii_alphabetic() || *first == b'_')
                && text
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'))
}
