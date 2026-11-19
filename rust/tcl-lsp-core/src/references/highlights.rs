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

//! Document highlights: the read and write occurrences of the symbol at the cursor.

use super::call_sites::{class_reference_spans, proc_reference_spans};
use super::member_references::{find_class_member_references, method_references_for_declaration};
use super::{RefCtx, span_to_range};
use crate::definition::LspRange;
use rustc_hash::{FxHashMap, FxHashSet};
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

/// Read / write kind for a document-highlight span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightKind {
    /// The cursor's symbol appears here as a read (`$var`,
    /// command-invocation head, etc.).
    Read,
    /// The cursor's symbol is being assigned / defined here
    /// (a `set` / `variable` / `upvar` write site, a proc
    /// declaration's name span, etc.).
    Write,
    /// The match has no read / write distinction — used for
    /// command-invocation heads whose call semantics aren't
    /// surfaced as read/write by the analyser.
    Text,
}

/// The class-name highlight set at the cursor, or `None` when the cursor is
/// not on one — the class arm of [`document_highlights`], lifted out so the
/// entry point stays a readable dispatch over symbol kinds.
fn class_highlights(
    source: &str,
    line_index: &LineIndex,
    analysis: &AnalysisResult,
    line: u32,
    character: u32,
    word: &str,
    resolution: crate::definition::CallResolution<'_>,
) -> Option<Vec<(LspRange, HighlightKind)>> {
    let cursor_off = crate::definition::byte_offset_at(line_index, source, line, character);
    // Declaration-span hit, else the namespace-aware candidate resolution —
    // never a namespace-blind `c.name == word` first-hit scan (the
    // wrong-symbol drift class).
    let (qname, class_def) =
        crate::definition::resolve_class_target_at(analysis, source, resolution, cursor_off, word)?;
    // Non-variable symbols (procs / classes / methods) highlight as `Text` for
    // both declaration and uses — only variables carry the Write/Read
    // distinction.
    let mut out = vec![(
        span_to_range(source, line_index, class_def.name_span),
        HighlightKind::Text,
    )];
    for span in class_reference_spans(analysis, resolution, qname, class_def, source) {
        out.push((span_to_range(source, line_index, span), HighlightKind::Text));
    }
    Some(dedup_kinded(out))
}

/// Compute the document-highlight spans for the symbol at the
/// cursor with read / write kinds.
///
/// Variables: `VarDef.definition_span` becomes `Write`; every
/// span in `VarDef.references` becomes `Read`.  Procs and
/// classes: the name span is `Write`; every matching command-
/// invocation head is `Text` (the analyser doesn't currently
/// distinguish read vs write semantics on command-invocation
/// heads, so we conservatively emit `Text`).
#[must_use]
pub fn document_highlights(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Vec<(LspRange, HighlightKind)> {
    document_highlights_in_program(
        source,
        dialect,
        line,
        character,
        analysis,
        crate::definition::CallResolution::document_only(),
    )
}

/// Convert already selected variable declaration/read spans into highlight
/// ranges. This is a pure report conversion: source identity and selection are
/// enforced by the caller before it supplies these spans.
pub(super) fn variable_occurrence_highlights(
    source: &str,
    line_index: &LineIndex,
    sites: impl IntoIterator<Item = (tcl_lexer::Span, bool)>,
) -> Vec<(LspRange, HighlightKind)> {
    let mut highlights: Vec<_> = sites
        .into_iter()
        .map(|(span, is_declaration)| {
            (
                span_to_range(source, line_index, span),
                if is_declaration {
                    HighlightKind::Write
                } else {
                    HighlightKind::Read
                },
            )
        })
        .collect();
    highlights.sort_by_key(|(range, _)| (range.start_line, range.start_character));
    dedup_kinded(highlights)
}

/// [`document_highlights`] with the caller's whole-program export view
/// attached — the entry point a host with a workspace index should call.
///
/// Highlighting is find-references narrowed to one document, so it must pick
/// the same target: a bare call a live `namespace import -force` shadows is
/// not an occurrence of the local definition, and which definition it *does*
/// reach can only be settled with whole-program export knowledge.
#[must_use]
pub fn document_highlights_in_program(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
) -> Vec<(LspRange, HighlightKind)> {
    if crate::original_context::CurrentSourceContext::capture(source, analysis).is_none() {
        return Vec::new();
    }
    let line_index = LineIndex::new(source);
    if let std::ops::ControlFlow::Break(selected) =
        crate::variable_symbol::select(source, analysis, line, character)
    {
        return selected.map_or_else(Vec::new, |occurrence| {
            let sites = crate::variable_symbol::occurrences(analysis, occurrence.symbol())
                .map(|site| (site.span(), site.is_declaration()))
                .collect::<Vec<_>>();
            // A read the document never declares anywhere resolves to no
            // definition, so there is nothing to highlight it against.
            if !sites.iter().any(|(_, is_declaration)| *is_declaration) {
                return Vec::new();
            }
            variable_occurrence_highlights(source, &line_index, sites)
        });
    }

    let byte_offset = crate::definition::byte_offset_at(&line_index, source, line, character);
    if let std::ops::ControlFlow::Break(selected) =
        crate::namespace_symbol::select_at_offset(source, analysis, byte_offset)
    {
        return selected.map_or_else(Vec::new, |symbol| {
            original_text_highlights(
                source,
                &line_index,
                crate::namespace_symbol::original_namespace_spans(analysis, &symbol, true),
            )
        });
    }
    if let std::ops::ControlFlow::Break(selected) =
        crate::method_symbol::local_candidate(source, analysis, line, character)
    {
        return selected.map_or_else(Vec::new, |candidate| {
            original_text_highlights(
                source,
                &line_index,
                crate::method_symbol::readonly_local_reference_report(
                    source, analysis, &candidate, true,
                )
                .map_or_else(
                    Vec::new,
                    crate::original_declaration::ReadonlyReferenceReport::into_spans,
                ),
            )
        });
    }
    if let std::ops::ControlFlow::Break(selected) =
        crate::original_declaration::select("", source, analysis, line, character)
    {
        return selected.map_or_else(Vec::new, |identity| {
            let spans = crate::original_declaration::readonly_reference_report(
                &identity, source, analysis, true,
            )
            .map_or_else(
                Vec::new,
                crate::original_declaration::ReadonlyReferenceReport::into_spans,
            )
            .into_iter()
            .chain(std::iter::once(identity.span()));
            original_text_highlights(source, &line_index, spans)
        });
    }
    if !analysis.allows_lexical_declaration_advice() {
        return Vec::new();
    }
    let ctx = RefCtx {
        source,
        dialect,
        line_index: &line_index,
        line,
        character,
        analysis,
        include_declaration: true,
        resolution,
    };
    lexical_document_highlights(&ctx, byte_offset)
}

fn lexical_document_highlights(
    ctx: &RefCtx<'_>,
    byte_offset: u32,
) -> Vec<(LspRange, HighlightKind)> {
    let RefCtx {
        source,
        dialect,
        line_index,
        line,
        character,
        analysis,
        resolution,
        ..
    } = *ctx;
    if let Some(highlights) =
        selected_method_highlights(source, dialect, analysis, line_index, byte_offset)
    {
        return highlights;
    }
    if crate::receiver_identity::definition_reference_at_cursor(analysis, source, byte_offset)
        .is_some()
    {
        return Vec::new();
    }
    // Shared `$ref` gate — see `substituting_var_at_position`.
    if let Some(var_name) = crate::definition::substituting_var_at_position(
        source,
        analysis,
        line,
        character,
        byte_offset,
    ) {
        let Some(var_def) =
            crate::definition::lookup_var_read_at(analysis, source, byte_offset, &var_name)
        else {
            return Vec::new();
        };
        let Some(references) =
            crate::definition::linked_var_reference_spans(source, analysis, var_def, byte_offset)
        else {
            return Vec::new();
        };
        let mut out = Vec::with_capacity(1 + references.len());
        out.push((
            span_to_range(source, line_index, var_def.definition_span),
            HighlightKind::Write,
        ));
        // Highlight original source relationships without asserting alias execution.
        for r in references {
            out.push((span_to_range(source, line_index, r), HighlightKind::Read));
        }
        return dedup_kinded(out);
    }

    let Some(word) =
        crate::source_structure::original_cursor_word_value(source, analysis, byte_offset)
    else {
        return Vec::new();
    };

    if let Some(out) = class_highlights(
        source, line_index, analysis, line, character, &word, resolution,
    ) {
        return out;
    }

    {
        let cursor_off = crate::definition::byte_offset_at(line_index, source, line, character);
        if let Some((qname, proc_def)) = crate::definition::resolve_proc_target_at(
            analysis, source, cursor_off, &word, resolution,
        ) {
            let mut out = Vec::new();
            out.push((
                span_to_range(source, line_index, proc_def.name_span),
                HighlightKind::Text,
            ));
            for span in proc_reference_spans(analysis, resolution, qname, proc_def, source) {
                out.push((span_to_range(source, line_index, span), HighlightKind::Text));
            }
            return dedup_kinded(out);
        }
    }

    // Class-member highlights — re-segment sibling method
    // bodies via `find_class_member_references` and mark the
    // declaration as Write, every call site as Text.
    let cursor_offset = crate::definition::byte_offset_at(line_index, source, line, character);
    if let Some((decl_span, call_spans)) =
        find_class_member_references(source, dialect, &word, analysis, cursor_offset)
    {
        let mut out = Vec::new();
        out.push((
            span_to_range(source, line_index, decl_span),
            HighlightKind::Text,
        ));
        for s in call_spans {
            out.push((span_to_range(source, line_index, s), HighlightKind::Text));
        }
        return dedup_kinded(out);
    }

    Vec::new()
}

fn original_text_highlights(
    source: &str,
    line_index: &LineIndex,
    spans: impl IntoIterator<Item = tcl_lexer::Span>,
) -> Vec<(LspRange, HighlightKind)> {
    let mut entries = spans
        .into_iter()
        .map(|span| (span_to_range(source, line_index, span), HighlightKind::Text))
        .collect::<Vec<_>>();
    entries.sort_by_key(|(range, _)| {
        (
            range.start_line,
            range.start_character,
            range.end_line,
            range.end_character,
        )
    });
    dedup_kinded(entries)
}

fn selected_method_highlights(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
    byte_offset: u32,
) -> Option<Vec<(LspRange, HighlightKind)>> {
    let selected = crate::receiver_identity::method_at_cursor(analysis, source, byte_offset)?;
    let (declaration, references) = method_references_for_declaration(
        source,
        dialect,
        analysis,
        selected.class,
        selected.method,
        selected.receiver == tcl_compiler::command_binding::SourceMethodReceiver::Class,
    );
    Some(
        std::iter::once(declaration)
            .chain(references)
            .map(|span| (span_to_range(source, line_index, span), HighlightKind::Text))
            .collect(),
    )
}

/// Deduplicate kinded highlight spans by (start, end) — keeps
/// the highest-kind for each duplicate range.  Write outranks
/// Read which outranks Text, so a span that the analyser
/// records both as a write and as a Read keeps the Write
/// label.
fn dedup_kinded(mut entries: Vec<(LspRange, HighlightKind)>) -> Vec<(LspRange, HighlightKind)> {
    let mut by_key: FxHashMap<(u32, u32, u32, u32), HighlightKind> = FxHashMap::default();
    for (range, kind) in &entries {
        let key = (
            range.start_line,
            range.start_character,
            range.end_line,
            range.end_character,
        );
        let kind = *kind;
        by_key
            .entry(key)
            .and_modify(|existing| {
                if priority(kind) > priority(*existing) {
                    *existing = kind;
                }
            })
            .or_insert(kind);
    }
    let mut seen: FxHashSet<(u32, u32, u32, u32)> = FxHashSet::default();
    entries.retain_mut(|(range, kind)| {
        let key = (
            range.start_line,
            range.start_character,
            range.end_line,
            range.end_character,
        );
        if !seen.insert(key) {
            return false;
        }
        *kind = by_key[&key];
        true
    });
    entries
}

pub(super) fn priority(kind: HighlightKind) -> u8 {
    match kind {
        HighlightKind::Write => 2,
        HighlightKind::Read => 1,
        HighlightKind::Text => 0,
    }
}
