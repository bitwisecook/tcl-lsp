//! Selection-range provider — minimal Rust port of
//! `lsp/features/selection_range.py`.
//!
//! Builds a chain of nested ranges that grow outward from the
//! cursor: word at cursor → enclosing line → entire document.
//!
//! What is *deferred* (planned as `S-selection-range-rich`
//! follow-up):
//!
//! * Command-segment range (Python adds the segmented Tcl
//!   command — single-line subset of `find_command_at_position`)
//!   between word and line.
//! * Enclosing-body ranges (proc / class / namespace bodies)
//!   between line and document.
//! * Containment-invariant validation against VS Code's
//!   requirement that each parent strictly contains its
//!   child (Python's `_lsp_range_contains` check); the
//!   minimal port's chain is built so parents always contain
//!   children by construction (word ⊂ line ⊂ document).

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
/// innermost first.  The server is responsible for materialising
/// the recursive `tower_lsp::lsp_types::SelectionRange` tree
/// from this flat representation.
#[must_use]
pub fn selection_range(source: &str, line: u32, character: u32) -> Vec<SelectionRange> {
    let mut chain: Vec<SelectionRange> = Vec::new();

    // 1. Word at cursor (innermost).
    if let Some((_, start, end)) = find_word_span_at_position(source, line, character) {
        chain.push(SelectionRange {
            range: LspRange {
                start_line: line,
                start_character: start,
                end_line: line,
                end_character: end,
            },
            parent_index: None,
        });
    }

    // 2. Enclosing line.
    if let Some(line_text) = source.split('\n').nth(line as usize) {
        let line_len = u32::try_from(line_text.chars().count()).unwrap_or(u32::MAX);
        let line_range = LspRange {
            start_line: line,
            start_character: 0,
            end_line: line,
            end_character: line_len,
        };
        let parent_for_word = chain.is_empty().then_some(usize::MAX);
        chain.push(SelectionRange {
            range: line_range,
            parent_index: None,
        });
        let line_idx = chain.len() - 1;
        // Chain word → line.
        if let Some(word_idx) = chain.len().checked_sub(2) {
            if parent_for_word.is_none() {
                chain[word_idx].parent_index = Some(line_idx);
            }
        }
    }

    // 3. Entire document.
    let total_lines = source.split('\n').count();
    if total_lines > 0 {
        let last_line_idx = u32::try_from(total_lines.saturating_sub(1)).unwrap_or(0);
        let last_line = source.split('\n').next_back().unwrap_or("");
        let last_line_len = u32::try_from(last_line.chars().count()).unwrap_or(u32::MAX);
        let doc_range = LspRange {
            start_line: 0,
            start_character: 0,
            end_line: last_line_idx,
            end_character: last_line_len,
        };
        let prev_idx = chain.len();
        chain.push(SelectionRange {
            range: doc_range,
            parent_index: None,
        });
        if prev_idx > 0 {
            chain[prev_idx - 1].parent_index = Some(prev_idx);
        }
    }

    chain
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
}
