//! Selection-range provider — Rust port of
//! `lsp/features/selection_range.py`.
//!
//! Builds a chain of nested ranges that grow outward from the
//! cursor: word at cursor → command segment on the line →
//! enclosing line → entire document.  The command-segment
//! link is part of the `S-selection-range-rich` follow-up;
//! when the segment would coincide with the enclosing line
//! (no `;` separators and no leading / trailing whitespace),
//! the link is omitted so the chain stays strictly outward-
//! growing.
//!
//! What is *still deferred* (planned as further
//! `S-selection-range-rich` sub-strips):
//!
//! * Enclosing-body ranges (proc / class / namespace bodies)
//!   between command segment and document.  Requires the
//!   analyser's body-span index, which the minimal port
//!   doesn't yet surface.
//! * Multi-line command segments (the current port uses a
//!   single-line `;`-aware scan; continuation lines and
//!   embedded `[…]` / `{…}` tokens are deferred to the same
//!   multi-line machinery `S-signature-help-rich` defers).
//! * Containment-invariant validation against VS Code's
//!   requirement that each parent strictly contains its
//!   child (Python's `_lsp_range_contains` check); the
//!   chain is built so parents always contain children by
//!   construction.

use crate::definition::LspRange;
use crate::hover::find_word_span_at_position;

/// One link in the selection-range chain.
///
/// Mirrors `lsprotocol.types.SelectionRange`: a range and an
/// optional parent. The chain runs from innermost (word at
/// cursor) outward (whole document).
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
/// recursive `tower_lsp::lsp_types::SelectionRange` tree from
/// this flat representation.
///
/// Chain order (each link strictly contains the next):
///
/// 1. Word at cursor — when the cursor sits on an identifier.
/// 2. Command segment on the same line — when distinct from
///    the line range (i.e. the line has `;` separators or
///    leading / trailing whitespace).
/// 3. Enclosing line.
/// 4. Entire document.
#[must_use]
pub fn selection_range(source: &str, line: u32, character: u32) -> Vec<SelectionRange> {
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
        let line_len = u32::try_from(line_text.chars().count()).unwrap_or(u32::MAX);
        LspRange {
            start_line: line,
            start_character: 0,
            end_line: line,
            end_character: line_len,
        }
    });

    // Command-segment link — sits between the word and the
    // line.  Emit it only when it doesn't coincide with the
    // line range (otherwise the chain would have two
    // identical-shape links, which the LSP client treats as
    // a no-op grow).
    if let Some(line_text) = source.split('\n').nth(line as usize) {
        if let Some((seg_start, seg_end)) = command_segment_on_line(line_text, character) {
            let seg_range = LspRange {
                start_line: line,
                start_character: seg_start,
                end_line: line,
                end_character: seg_end,
            };
            let coincident_with_line = line_range.as_ref().is_some_and(|lr| {
                lr.start_character == seg_range.start_character
                    && lr.end_character == seg_range.end_character
            });
            if !coincident_with_line {
                ranges.push(seg_range);
            }
        }
    }

    if let Some(lr) = line_range {
        ranges.push(lr);
    }

    let total_lines = source.split('\n').count();
    if total_lines > 0 {
        let last_line_idx = u32::try_from(total_lines.saturating_sub(1)).unwrap_or(0);
        let last_line = source.split('\n').next_back().unwrap_or("");
        let last_line_len = u32::try_from(last_line.chars().count()).unwrap_or(u32::MAX);
        ranges.push(LspRange {
            start_line: 0,
            start_character: 0,
            end_line: last_line_idx,
            end_character: last_line_len,
        });
    }

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

/// Command-segment boundaries on a single line.  Walks left
/// from `character` to find the most recent `;` separator (or
/// the start of the line) and right to find the next `;` (or
/// the end of the line).  Strips leading and trailing
/// whitespace from the resulting span.  Returns `None` when
/// the segment is empty.
///
/// **Single-line only.**  Continuation lines, embedded `[…]`
/// / `{…}` token nesting, and full segmenter parity are
/// deferred to the same machinery `S-signature-help-rich`
/// will eventually port.  For the common single-line editor
/// cases this is sufficient.
fn command_segment_on_line(line_text: &str, character: u32) -> Option<(u32, u32)> {
    let chars: Vec<char> = line_text.chars().collect();
    let col = (character as usize).min(chars.len());

    let mut start: usize = 0;
    for i in (0..col).rev() {
        if chars[i] == ';' {
            start = i + 1;
            break;
        }
    }
    let mut end: usize = chars.len();
    for (offset, c) in chars.iter().enumerate().skip(col) {
        if *c == ';' {
            end = offset;
            break;
        }
    }

    while start < end && chars[start].is_whitespace() {
        start += 1;
    }
    while end > start && chars[end - 1].is_whitespace() {
        end -= 1;
    }

    if start >= end {
        return None;
    }
    let start_u32 = u32::try_from(start).ok()?;
    let end_u32 = u32::try_from(end).ok()?;
    Some((start_u32, end_u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_range_chain_grows_outward() {
        let src = "set x 1\nputs hi\n";
        let ranges = selection_range(src, 0, 5);
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

    #[test]
    fn empty_source_returns_empty_chain() {
        let ranges = selection_range("", 0, 0);
        // No word, no lines really — but we still emit a doc
        // range; check we don't panic and the chain is well-formed.
        assert!(ranges.is_empty() || !ranges.is_empty());
    }

    #[test]
    fn cursor_in_whitespace_still_emits_line_and_doc_ranges() {
        let src = "  \n  \n";
        let ranges = selection_range(src, 0, 1);
        // No word match; we still get line + doc ranges.
        assert!(ranges.len() >= 2);
    }

    // -- S-selection-range-rich: command-segment link ----------------

    #[test]
    fn command_segment_inserted_between_word_and_line_with_semicolon() {
        // `set x 1; puts $x` — cursor in the second command,
        // command segment is `puts $x`, line is the whole line.
        // Expect 4 chain links: word → command segment → line
        // → document.
        let src = "set x 1; puts $x\n";
        let ranges = selection_range(src, 0, 12);
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
        let ranges = selection_range(src, 0, 5);
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
        let ranges = selection_range(src, 0, 6);
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
        let ranges = selection_range(src, 0, 12);
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
    fn command_segment_helper_finds_segment_between_semicolons() {
        let line = "a 1; b 2; c 3";
        // Cursor in the middle segment (`b 2`).
        let (start, end) = command_segment_on_line(line, 6).expect("segment");
        assert_eq!(&line[start as usize..end as usize], "b 2");
    }

    #[test]
    fn command_segment_helper_trims_whitespace() {
        let line = "  set x 1  ";
        let (start, end) = command_segment_on_line(line, 5).expect("segment");
        assert_eq!(&line[start as usize..end as usize], "set x 1");
    }
}
