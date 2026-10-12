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

//! Selection-range provider.
//!
//! Builds a chain of nested ranges that grow outward from the
//! cursor: word at cursor → command segment on the line →
//! enclosing line → entire document.  The command-segment
//! link is omitted when the segment would coincide with the
//! enclosing line (no `;` separators and no leading / trailing
//! whitespace), so the chain stays strictly outward-growing.
//!
//! Enclosing-body ranges: when the
//! caller threads an [`AnalysisResult`] through, the chain
//! grows with one link per containing proc / class body,
//! ordered innermost first.  This makes `Ctrl-Shift-Right`
//! step from a statement to its proc body, then to the
//! enclosing class body, then to the document.
//!
//! Limitations:
//!
//! * Namespace bodies contribute no enclosing link: their spans
//!   live only in the analyser's scope tree, which this provider
//!   does not walk.
//! * Partial commands use the segmenter's recovery span; an unfinished edit
//!   may therefore have a wider command link until its closer is written.

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::{LineIndex, Span};

use crate::definition::{LspRange, byte_offset_at, utf16_len};
use crate::hover::find_word_span_at_position;

/// One link in the selection-range chain.
///
/// A range and an optional parent. The chain runs from
/// innermost (word at cursor) outward (whole document).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionRange {
    /// Range covered by this link.
    pub range: LspRange,
    /// Index of the parent link in the same `Vec`, or `None`
    /// for the outermost element.
    pub parent_index: Option<usize>,
}

/// Compute the selection-range chain for a position.
///
/// The returned vector lists every link in the chain ordered
/// innermost first.  Parent links are wired so each child's
/// `parent_index` points to the next outward link in the
/// chain.  The server is responsible for materialising the
/// recursive `ls_types::SelectionRange` tree from
/// this flat representation.
///
/// Chain order (each link strictly contains the next):
///
/// 1. Word at cursor — when the cursor sits on an identifier.
/// 2. Segmented command — when distinct from the enclosing line.
/// 3. Enclosing line.
/// 4. Enclosing proc / class bodies — one link per containing
///    body, innermost first.  Only present when `analysis`
///    is `Some`.
/// 5. Entire document.
#[must_use]
pub fn selection_range(
    source: &str,
    line: u32,
    character: u32,
    analysis: Option<&AnalysisResult>,
) -> Vec<SelectionRange> {
    selection_range_for_dialect(
        source,
        line,
        character,
        analysis,
        analysis
            .and_then(AnalysisResult::resolved_profile)
            .unwrap_or_else(|| crate::profile_for_dialect("tcl9.0")),
    )
}

/// Compute the selection-range chain using the document's resolved dialect.
///
/// The command link is a segmented CST command span, not a source scan for
/// semicolons, so literal separators inside Tcl words cannot truncate it.
#[must_use]
pub fn selection_range_for_dialect(
    source: &str,
    line: u32,
    character: u32,
    analysis: Option<&AnalysisResult>,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Vec<SelectionRange> {
    // Implementation contract: naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md

    let mut ranges: Vec<LspRange> = Vec::new();

    if let Some((_, start, end)) = find_word_span_at_position(source, line, character) {
        ranges.push(LspRange {
            start_line: line,
            start_character: start,
            end_line: line,
            end_character: end,
        });
    }

    let line_range = source.split('\n').nth(line as usize).map(|line_text| {
        let line_len = utf16_len(line_text);
        LspRange {
            start_line: line,
            start_character: 0,
            end_line: line,
            end_character: line_len,
        }
    });

    // The command-segment link normally sits between the word and the line.
    // It is dropped when it coincides with the line range, which would leave
    // the chain with two identical links — a no-op grow to the LSP client.
    // A multiline command instead becomes the direct parent; a physical line
    // cannot contain it.
    let line_index = LineIndex::new(source);
    let cursor_offset = byte_offset_at(&line_index, source, line, character);
    let mut command_is_multiline = false;
    let owned_analysis = analysis.is_none().then(|| {
        crate::source_structure::analyse_document(
            source,
            dialect,
            crate::registry_for_dialect_profile(dialect),
            tcl_lexer::LexerConfig::for_profile(Some(dialect)),
        )
    });
    let current_analysis = analysis.or(owned_analysis.as_ref()).filter(|analysis| {
        analysis
            .resolved_profile()
            .is_some_and(|profile| profile.cache_key() == dialect.cache_key())
            && analysis.body_lexer_config.is_some_and(|config| {
                analysis.matches_original_source_image(
                    &tcl_lexer::SourceImage::document(source),
                    config,
                )
            })
    });
    let command_config = current_analysis
        .and_then(|analysis| analysis.body_lexer_config)
        .unwrap_or_else(|| tcl_lexer::LexerConfig::for_file_grammar(dialect.grammar));
    if let Some(span) =
        crate::source_structure::SourceStructure::capture(source, current_analysis, command_config)
            .and_then(|structure| structure.command_at(cursor_offset))
    {
        let seg_range = span_to_range(source, &line_index, span);
        command_is_multiline = seg_range.start_line != seg_range.end_line;
        let coincident_with_line = line_range.as_ref().is_some_and(|lr| {
            lr.start_line == seg_range.start_line
                && lr.end_line == seg_range.end_line
                && lr.start_character == seg_range.start_character
                && lr.end_character == seg_range.end_character
        });
        if !coincident_with_line {
            ranges.push(seg_range);
        }
    }

    // A physical line cannot contain a multiline command.  Omitting it keeps
    // every LSP parent link a true containing range.
    if !command_is_multiline && let Some(lr) = line_range {
        ranges.push(lr);
    }

    // Enclosing-body links — one per proc / class body whose
    // span contains the cursor's byte offset.  Order is
    // innermost first so the chain stays outward-growing.
    if let Some(analysis) = current_analysis {
        for span in enclosing_body_spans(source, analysis, cursor_offset).unwrap_or_default() {
            ranges.push(span_to_range(source, &line_index, span));
        }
    }

    let total_lines = source.split('\n').count();
    if total_lines > 0 {
        let last_line_idx = u32::try_from(total_lines.saturating_sub(1)).unwrap_or(0);
        let last_line = source.split('\n').next_back().unwrap_or("");
        let last_line_len = utf16_len(last_line);
        ranges.push(LspRange {
            start_line: 0,
            start_character: 0,
            end_line: last_line_idx,
            end_character: last_line_len,
        });
    }

    // Enforce the LSP parent-contains-child invariant: every range must lie
    // within its outward neighbour. A full-width *line* range can stick out of
    // an enclosing body that starts/ends mid-line (e.g. `proc foo {} {\n set x 1
    // }` — the line's end column exceeds the body's), which VS Code rejects.
    // Clamp each range to its parent, processing
    // outermost→innermost so the chain stays strictly nested. Every range
    // contains the cursor, so the intersection is never empty (no inversion).
    for i in (0..ranges.len().saturating_sub(1)).rev() {
        let parent_start = {
            let p = &ranges[i + 1];
            (p.start_line, p.start_character)
        };
        let parent_end = {
            let p = &ranges[i + 1];
            (p.end_line, p.end_character)
        };
        let r = &mut ranges[i];
        if (r.start_line, r.start_character) < parent_start {
            r.start_line = parent_start.0;
            r.start_character = parent_start.1;
        }
        if (r.end_line, r.end_character) > parent_end {
            r.end_line = parent_end.0;
            r.end_character = parent_end.1;
        }
    }

    // Clamping can collapse a command/body link onto its parent. LSP ranges
    // must grow strictly at every parent edge, so discard equal neighbours.
    ranges.dedup();

    // Wire `parent_index` so each link points to its outward
    // neighbour.  The outermost link has `None`.
    let len = ranges.len();
    ranges
        .into_iter()
        .enumerate()
        .map(|(i, range)| SelectionRange {
            range,
            parent_index: (i + 1 < len).then_some(i + 1),
        })
        .collect()
}

/// Current readonly declaration-body geometry. Each original declaration owns
/// its own source site; names, installed tables and later routes supply no join.
/// Missing currency or non-nested containing spans declines semantic links.
fn enclosing_body_spans(
    source: &str,
    analysis: &AnalysisResult,
    cursor_offset: u32,
) -> Option<Vec<Span>> {
    let config = analysis.body_lexer_config?;
    let image = tcl_lexer::SourceImage::document(source);
    analysis
        .matches_original_source_image(&image, config)
        .then_some(())?;
    let contains = |s: Span| s.start() < cursor_offset && cursor_offset < s.end();
    let mut spans: Vec<Span> = Vec::new();
    let mut retain = |span: Span| {
        if span.start() > span.end() || source.get(span.as_range()).is_none() {
            return None;
        }
        if contains(span) {
            spans.push(span);
        }
        Some(())
    };
    if !analysis.allows_lexical_declaration_advice() {
        use crate::original_declaration::OriginalDeclarationIdentity;
        for declaration in analysis.original_procedure_declarations() {
            OriginalDeclarationIdentity::for_procedure("", source, analysis, declaration)?;
            retain(declaration.metadata().body_span)?;
        }
        for class in analysis.original_class_declarations() {
            OriginalDeclarationIdentity::for_class("", source, analysis, class)?;
            retain(class.metadata().body_span)?;
            for method in class.metadata().original_members.declarations() {
                OriginalDeclarationIdentity::for_method("", source, analysis, class, method)?;
                if method.forward_prefix().is_none() {
                    retain(method.metadata().body_span)?;
                }
            }
            for special in class.metadata().original_special_members.declarations() {
                OriginalDeclarationIdentity::for_special("", source, analysis, class, special)?;
                let body = special.body_word();
                (body.image() == &image && body.config() == config).then_some(())?;
                retain(body.content_span().ok()?)?;
            }
        }
        // Independently selected definition workers own their body syntax even
        // when the target remains an external declaration obligation. Their
        // body spans do not select a class, object allocation or native entry.
        for configuration in analysis.original_class_configurations() {
            (configuration.target().site().source.source_image() == &image).then_some(())?;
            retain_original_member_bodies(
                configuration.members(),
                configuration.special_members(),
                &image,
                config,
                &mut retain,
            )?;
        }
        for configuration in analysis.original_object_configurations() {
            (configuration.target().site().source.source_image() == &image).then_some(())?;
            retain_original_member_bodies(
                configuration.members(),
                configuration.special_members(),
                &image,
                config,
                &mut retain,
            )?;
        }
    } else {
        // Explicit logical compatibility, independently selected at ingress.
        // A missing original/native inventory cannot enable this branch.
        retain_logical_body_spans(analysis, &mut retain)?;
    }
    // Innermost first — sort by span width ascending.
    spans.sort_by_key(|s| s.end() - s.start());
    spans.dedup();
    if spans
        .windows(2)
        .any(|pair| pair[1].start() > pair[0].start() || pair[1].end() < pair[0].end())
    {
        return None;
    }
    Some(spans)
}

fn retain_logical_body_spans(
    analysis: &AnalysisResult,
    retain: &mut impl FnMut(Span) -> Option<()>,
) -> Option<()> {
    for proc_def in analysis.all_procs.values() {
        retain(proc_def.body_span)?;
    }
    for class_def in analysis.all_classes.values() {
        retain(class_def.body_span)?;
        for method in class_def
            .methods
            .values()
            .chain(class_def.class_methods.values())
            .chain(class_def.constructors.iter())
            .chain(class_def.destructor.iter())
        {
            retain(method.body_span)?;
        }
    }
    Some(())
}

fn retain_original_member_bodies(
    methods: &tcl_compiler::analyser::types::OriginalSourceMemberLedger,
    specials: &tcl_compiler::analyser::types::OriginalSourceSpecialMemberLedger,
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    retain: &mut impl FnMut(Span) -> Option<()>,
) -> Option<()> {
    for method in methods.declarations() {
        let declaration = method.declaration();
        let word = declaration.original_word();
        (word.image() == image
            && word.config() == config
            && declaration.site().source.source_image() == image)
            .then_some(())?;
        if method.forward_prefix().is_none() {
            retain(method.metadata().body_span)?;
        }
    }
    for special in specials.declarations() {
        let body = special.body_word();
        (body.image() == image
            && body.config() == config
            && special.site().source.source_image() == image)
            .then_some(())?;
        retain(body.content_span().ok()?)?;
    }
    Some(())
}

fn span_to_range(source: &str, line_index: &LineIndex, span: Span) -> LspRange {
    let start = line_index.position_at_utf16(span.start(), source);
    let end = line_index.position_at_utf16(span.end(), source);
    LspRange {
        start_line: start.line,
        start_character: start.character.get(),
        end_line: end.line,
        end_character: end.character.get(),
    }
}

/// Explicit test/API compatibility ingress with the same actual full grammar
/// and store as public analysis-free selection. Geometry never grants dispatch.
#[cfg(test)]
fn command_span_at(
    source: &str,
    cursor_offset: u32,
    config: tcl_lexer::LexerConfig,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Option<Span> {
    let analysis = crate::source_structure::analyse_document(
        source,
        dialect,
        crate::registry_for_dialect_profile(dialect),
        config,
    );
    crate::source_structure::SourceStructure::capture(source, Some(&analysis), config)?
        .command_at(cursor_offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_selection_uses_retained_bodies_and_declines_stale_body_roles() {
        // Implementation contract: naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        let source = "proc p {} {\n puts [list inner]\n}\n";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        let ranges = selection_range(source, 1, 13, Some(&analysis));
        assert!(ranges.iter().any(|range| range.range.start_line == 1
            && range.range.start_character == 7
            && range.range.end_character == 17));
        let stale = source.replace("inner", "other");
        let ranges = selection_range(&stale, 1, 13, Some(&analysis));
        assert!(!ranges.iter().any(|range| range.range.start_line == 1
            && range.range.start_character == 7
            && range.range.end_character == 17));
    }

    #[test]
    fn selection_range_chain_grows_outward() {
        let src = "set x 1\nputs hi\n";
        let ranges = selection_range(src, 0, 5, None);
        assert!(!ranges.is_empty());
        // Innermost should be the word `1` (or `x` if cursor was earlier).
        let inner = &ranges[0];
        assert_eq!(inner.range.start_line, 0);
        // Each subsequent range must strictly contain its child.
        for w in ranges.windows(2) {
            let child = &w[0];
            let parent = &w[1];
            assert!(parent.range.start_line <= child.range.start_line);
            assert!(parent.range.end_line >= child.range.end_line);
        }
    }

    /// An empty document yields exactly one link, not none — the old name
    /// said the opposite, and its `x || !x` assertion was true of every
    /// value, so it tested only that the call returned (#2072).
    ///
    /// One link rather than two is the load-bearing part: the line range and
    /// the whole-document range both come out as `(0,0)-(0,0)` here, and
    /// `dedup` collapses them. Without that collapse the client is handed a
    /// chain whose "grow" step does not grow — commenting out the `dedup`
    /// call makes this fail with `left: 2, right: 1`.
    #[test]
    fn empty_source_still_yields_a_document_range() {
        let ranges = selection_range("", 0, 0, None);
        assert_eq!(ranges.len(), 1, "{ranges:?}");
        let doc = &ranges[0];
        assert_eq!(
            (
                doc.range.start_line,
                doc.range.start_character,
                doc.range.end_line,
                doc.range.end_character
            ),
            (0, 0, 0, 0),
            "the one range covers the whole (empty) document"
        );
        assert!(doc.parent_index.is_none(), "it is the outermost range");
    }

    #[test]
    fn cursor_in_whitespace_still_emits_line_and_doc_ranges() {
        let src = "  \n  \n";
        let ranges = selection_range(src, 0, 1, None);
        // No word match; we still get line + doc ranges.
        assert!(ranges.len() >= 2);
    }

    #[test]
    fn command_segment_inserted_between_word_and_line_with_semicolon() {
        // `set x 1; puts $x` — cursor in the second command,
        // command segment is `puts $x`, line is the whole line.
        // Expect 4 chain links: word → command segment → line
        // → document.
        let src = "set x 1; puts $x\n";
        let ranges = selection_range(src, 0, 12, None);
        assert!(ranges.len() >= 4, "chain too short: {ranges:?}");
        // Same-line links (word, segment, line) must
        // nest by character span; the outermost document
        // link spans multiple lines, so we check it with
        // line-aware containment instead.
        for w in ranges.windows(2) {
            let child = &w[0];
            let parent = &w[1];
            let contains = if parent.range.start_line == parent.range.end_line
                && child.range.start_line == child.range.end_line
                && parent.range.start_line == child.range.start_line
            {
                parent.range.start_character <= child.range.start_character
                    && parent.range.end_character >= child.range.end_character
            } else {
                parent.range.start_line <= child.range.start_line
                    && parent.range.end_line >= child.range.end_line
            };
            assert!(
                contains,
                "parent {parent:?} doesn't contain child {child:?}",
            );
        }
        // Command segment should start at column 9 (after
        // `; `) and end before any trailing whitespace.
        let seg = &ranges[1];
        assert_eq!(seg.range.start_character, 9, "{seg:?}");
    }

    #[test]
    fn command_segment_omitted_when_coincident_with_line() {
        // `puts hi\n` — no `;`, no whitespace around the
        // command.  The command segment would equal the line
        // range, so the rich link is suppressed and the chain
        // is just word → line → doc.
        let src = "puts hi\n";
        let ranges = selection_range(src, 0, 5, None);
        // No duplicate ranges by start/end char.
        for w in ranges.windows(2) {
            assert!(
                w[0].range.start_character != w[1].range.start_character
                    || w[0].range.end_character != w[1].range.end_character,
                "duplicate range pair: {ranges:?}",
            );
        }
    }

    #[test]
    fn command_segment_emitted_with_leading_whitespace() {
        // `    set x 1\n` — leading whitespace.  Command
        // segment starts at column 4, line range starts at 0.
        // The rich link should be present.
        let src = "    set x 1\n";
        let ranges = selection_range(src, 0, 6, None);
        // Find the command segment (between word and line).
        let starts: Vec<u32> = ranges.iter().map(|r| r.range.start_character).collect();
        // Expect at least one range starting at column 4
        // (segment) and one starting at column 0 (line + doc).
        assert!(
            starts.contains(&4),
            "expected segment starting at col 4; got starts={starts:?}",
        );
        assert!(
            starts.contains(&0),
            "expected line / doc range starting at col 0; got starts={starts:?}",
        );
    }

    #[test]
    fn parent_indices_form_outward_chain() {
        let src = "set x 1; puts $x\n";
        let ranges = selection_range(src, 0, 12, None);
        // Every link except the outermost should have its
        // `parent_index` pointing to the next link in the
        // Vec.
        for (i, r) in ranges.iter().enumerate() {
            if i + 1 < ranges.len() {
                assert_eq!(r.parent_index, Some(i + 1), "link {i}: {r:?}");
            } else {
                assert_eq!(r.parent_index, None, "outermost link {i}: {r:?}");
            }
        }
    }

    #[test]
    fn command_segment_uses_segmenter_between_semicolons() {
        let line = "a 1; b 2; c 3";
        let spans = selection_range(line, 0, 6, None);
        assert!(
            spans.iter().any(|range| {
                range.range.start_character == 5 && range.range.end_character == 8
            })
        );
    }

    #[test]
    fn command_segment_trims_whitespace() {
        let line = "  set x 1  ";
        let spans = selection_range(line, 0, 5, None);
        assert!(
            spans.iter().any(|range| {
                range.range.start_character == 2 && range.range.end_character == 9
            })
        );
    }

    #[test]
    fn command_segment_does_not_split_literal_semicolon_in_braces() {
        let src = "foo {a; b} c";
        let spans = selection_range(src, 0, 6, None);
        assert!(spans.iter().any(|range| {
            range.range.start_character == 0 && range.range.end_character == utf16_len(src)
        }));
    }

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = tcl_compiler::analyser::Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    #[test]
    fn original_body_ranges_keep_opaque_and_repeated_declaration_owners() {
        // Implementation contract: naming.editor.original-selection-body-ranges
        // docs/design/analysis/name-resolution-proofs/original-selection-body-ranges.md
        let source = "proc p\\uD800 {} {\n set first 1\n}\nproc p\\uD801 {} {\n set second 2\n}\nproc repeated {} {\n set earlier 3\n}\nproc repeated {} {\n set later 4\n}\n";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, dialect);
            let bodies = analysis
                .original_procedure_declarations()
                .map(|row| row.metadata().body_span)
                .collect::<Vec<_>>();
            assert_eq!(bodies.len(), 4, "{dialect}");
            analysis.all_procs.clear();
            analysis.superseded_procs.clear();
            analysis.proc_declaration_sites.clear();
            for (marker, body) in ["first", "second", "earlier", "later"]
                .into_iter()
                .zip(bodies)
            {
                let cursor = u32::try_from(source.find(marker).unwrap()).unwrap();
                assert_eq!(
                    enclosing_body_spans(source, &analysis, cursor),
                    Some(vec![body]),
                    "{dialect} {marker}"
                );
                let index = LineIndex::new(source);
                let position = index.position_at_utf16(cursor, source);
                let ranges = selection_range(
                    source,
                    position.line,
                    position.character.get(),
                    Some(&analysis),
                );
                assert!(
                    ranges
                        .iter()
                        .any(|range| range.range == span_to_range(source, &index, body)),
                    "{dialect} {marker} {ranges:?}"
                );
            }
        }
    }

    #[test]
    fn original_body_ranges_keep_named_and_nameless_class_workers() {
        // Implementation contract: naming.editor.original-selection-body-ranges
        // docs/design/analysis/name-resolution-proofs/original-selection-body-ranges.md
        let source = "oo::class create C\\uD800 {\n method m\\uD800 {} {\n  set methodBody 1\n }\n constructor {} {\n  set constructorBody 2\n }\n destructor {\n  set destructorBody 3\n }\n}\n";
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, dialect);
            let class = analysis.original_class_declarations().next().unwrap();
            let class_body = class.metadata().body_span;
            let mut workers = class
                .metadata()
                .original_members
                .declarations()
                .map(|row| row.metadata().body_span)
                .collect::<Vec<_>>();
            workers.extend(
                class
                    .metadata()
                    .original_special_members
                    .declarations()
                    .map(|row| row.body_word().content_span().unwrap()),
            );
            assert_eq!(workers.len(), 3, "{dialect}");
            analysis.all_classes.clear();
            analysis.superseded_classes.clear();
            for (marker, worker) in ["methodBody", "constructorBody", "destructorBody"]
                .into_iter()
                .zip(workers)
            {
                let cursor = u32::try_from(source.find(marker).unwrap()).unwrap();
                let spans = enclosing_body_spans(source, &analysis, cursor).unwrap();
                assert_eq!(spans, [worker, class_body], "{dialect} {marker}");
            }
        }
    }

    #[test]
    fn original_body_ranges_refuse_stale_configuration_and_foreign_headers() {
        // Implementation contract: naming.editor.original-selection-body-ranges
        // docs/design/analysis/name-resolution-proofs/original-selection-body-ranges.md
        let source = "proc p {} {\n set value 1\n}\n";
        let analysis = analyse(source);
        let cursor = u32::try_from(source.find("value").unwrap()).unwrap();
        assert_eq!(
            enclosing_body_spans(source, &analysis, cursor)
                .unwrap()
                .len(),
            1
        );
        assert!(
            enclosing_body_spans(&source.replace("value", "other"), &analysis, cursor).is_none()
        );
        let mut changed_config = analysis.clone();
        let mut config = changed_config.body_lexer_config.unwrap();
        config.strict_quoting = !config.strict_quoting;
        changed_config.body_lexer_config = Some(config);
        assert!(enclosing_body_spans(source, &changed_config, cursor).is_none());
        let foreign = analyse("proc foreign {} {set value 9}");
        let mut contaminated = analysis.clone();
        contaminated
            .original_procedure_metadata
            .extend(foreign.original_procedure_declarations().cloned());
        assert!(enclosing_body_spans(source, &contaminated, cursor).is_none());
        let mut assistance = AnalysisResult::default();
        assistance.all_procs = analysis.all_procs.clone();
        assert!(enclosing_body_spans(source, &assistance, cursor).is_none());
    }

    #[test]
    fn chain_is_strictly_nested_for_mid_line_body() {
        // A single-line-ish proc whose body ends mid-line
        // (`set x 1 }` on line 1) must still yield a strictly-nested chain —
        // every range contained by its outward neighbour.
        let src = "proc foo {} {\n    set x 1 }\n";
        let analysis = analyse(src);
        let ranges = selection_range(src, 1, 5, Some(&analysis));
        assert!(ranges.len() >= 2, "{ranges:?}");
        // Each range must be contained by its parent (the next outward link).
        let key = |l: u32, c: u32| (l, c);
        for r in &ranges {
            let Some(pi) = r.parent_index else { continue };
            let p = &ranges[pi];
            assert!(
                key(p.range.start_line, p.range.start_character)
                    <= key(r.range.start_line, r.range.start_character),
                "parent must start at/before child: parent={:?} child={:?}",
                p.range,
                r.range,
            );
            assert!(
                key(r.range.end_line, r.range.end_character)
                    <= key(p.range.end_line, p.range.end_character),
                "parent must end at/after child: parent={:?} child={:?}",
                p.range,
                r.range,
            );
        }
    }

    #[test]
    fn analysis_chain_adds_enclosing_proc_body() {
        // Cursor inside the proc body — should add a link
        // covering the body, between line and document.
        let src = "proc greet {} {\n    set x 1\n}\n";
        let analysis = analyse(src);
        let ranges = selection_range(src, 1, 8, Some(&analysis));
        // Strict containment removes a line/command link when clamping makes
        // it equal to the proc body: word + body + document is the minimal
        // valid chain here.
        assert!(ranges.len() >= 3, "{ranges:?}");
        // Find the body link — its start_line should be 0
        // (the opening `{` line) and end_line >= 2.
        let body_link = ranges
            .iter()
            .find(|r| r.range.start_line == 0 && r.range.end_line >= 2);
        assert!(body_link.is_some(), "expected body link; got {ranges:?}");
    }

    #[test]
    fn analysis_chain_skips_body_when_cursor_outside() {
        // Cursor on the line AFTER the proc body.  No
        // enclosing-body link should appear.
        let src = "proc greet {} {\n    set x 1\n}\nset z 9\n";
        let analysis = analyse(src);
        let ranges = selection_range(src, 3, 5, Some(&analysis));
        // No range covers lines 0–2 inclusive (the body).
        let has_body = ranges
            .iter()
            .any(|r| r.range.start_line == 0 && r.range.end_line == 2);
        assert!(!has_body, "unexpected body link: {ranges:?}");
    }

    #[test]
    fn analysis_chain_orders_inner_body_before_outer_class() {
        // Method body inside a class body.  Cursor inside the
        // method body should yield the method body first, then
        // the class body.
        let src = "oo::class create C {\n    method m {} {\n        set x 1\n    }\n}\n";
        let analysis = analyse(src);
        let ranges = selection_range(src, 2, 16, Some(&analysis));
        // Multi-line links are (in chain order, innermost first):
        // method body, class body, document.  The document link
        // is always the very last one — drop it before checking
        // the body ordering.
        let multi_line: Vec<&SelectionRange> = ranges
            .iter()
            .filter(|r| r.range.start_line != r.range.end_line)
            .collect();
        assert!(
            multi_line.len() >= 3,
            "expected method-body + class-body + doc; got {ranges:?}",
        );
        // Drop the document link (the last one).
        let bodies = &multi_line[..multi_line.len() - 1];
        assert!(
            bodies.len() >= 2,
            "expected ≥2 enclosing-body links; got {bodies:?}",
        );
        // Each body's span should be no wider than the next.
        let width = |r: &SelectionRange| r.range.end_line - r.range.start_line;
        for win in bodies.windows(2) {
            assert!(
                width(win[0]) <= width(win[1]),
                "expected innermost-first body ordering, got {bodies:?}",
            );
        }
    }

    #[test]
    fn continuation_and_multiline_bracket_cursor_keep_command_parent_nested() {
        let src = "set result [format %s \\\n    [string toupper value]]\n";
        // Cursor on the second physical line, inside the nested bracket word.
        // The generic substitution traversal selects that innermost command,
        // not the outer multiline `set` command.
        let ranges = selection_range(src, 1, 19, None);
        assert!(ranges.len() >= 3, "{ranges:?}");
        let command = &ranges[1].range;
        assert_eq!((command.start_line, command.end_line), (1, 1), "{ranges:?}");
        // The immediate parent is the innermost command, followed by its
        // physical line; every parent remains a true containment range.
        for link in &ranges {
            if let Some(parent) = link.parent_index {
                let parent = &ranges[parent].range;
                assert!(
                    (parent.start_line, parent.start_character)
                        <= (link.range.start_line, link.range.start_character)
                        && (link.range.end_line, link.range.end_character)
                            <= (parent.end_line, parent.end_character),
                    "non-nested selection chain: {ranges:?}"
                );
            }
        }
        assert!(ranges.windows(2).all(|pair| pair[0].range != pair[1].range));
    }

    #[test]
    fn nested_proc_command_is_innermost_with_and_without_analysis() {
        let src = "proc p {} {\n :::if {1} {\n  set value [string toupper \\\n    local]\n }\n}\n";
        let cursor = u32::try_from(src.find("local").unwrap()).unwrap();
        let source_len = u32::try_from(src.len()).unwrap();
        let analysed = analyse(src);
        for analysis in [None, Some(&analysed)] {
            let span = command_span_at(
                src,
                cursor,
                tcl_lexer::LexerConfig::for_file_dialect("tcl9.0"),
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            )
            .expect("inner command");
            assert!(span.start() > 0 && span.end() - span.start() < source_len);
            let ranges = selection_range(src, 3, 5, analysis);
            assert!(
                ranges.windows(2).all(|pair| {
                    (pair[1].range.start_line, pair[1].range.start_character)
                        < (pair[0].range.start_line, pair[0].range.start_character)
                        || (pair[0].range.end_line, pair[0].range.end_character)
                            < (pair[1].range.end_line, pair[1].range.end_character)
                }),
                "{ranges:?}"
            );
        }
    }

    #[test]
    fn command_substitutions_choose_the_innermost_multiline_command_once() {
        let src = "set result [list [string toupper \\\n    local]]\n";
        let cursor = u32::try_from(src.find("local").unwrap()).unwrap();
        let span = command_span_at(
            src,
            cursor,
            tcl_lexer::LexerConfig::for_file_dialect("tcl9.0"),
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        )
        .expect("nested command substitution");
        assert_eq!(
            &src[span.start() as usize..span.end() as usize],
            "string toupper \\\n    local"
        );

        let ranges = selection_range(src, 1, 5, None);
        assert!(ranges.windows(2).all(|pair| pair[0].range != pair[1].range));
    }

    #[test]
    fn aliased_registry_body_and_nested_substitution_choose_inner_command() {
        let src = concat!(
            "interp alias {} branch {} if\n",
            "branch {1} {\n",
            " set result [list [string toupper local]]\n",
            "}\n",
        );
        let cursor = u32::try_from(src.find("local").unwrap()).unwrap();
        let span = command_span_at(
            src,
            cursor,
            tcl_lexer::LexerConfig::for_file_dialect("tcl9.0"),
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        )
        .expect("inner command through aliased Body role");
        assert_eq!(
            &src[span.start() as usize..span.end() as usize],
            "string toupper local"
        );
        let ranges = selection_range(src, 2, 39, None);
        assert!(ranges.windows(2).all(|pair| pair[0].range != pair[1].range));
    }
}
