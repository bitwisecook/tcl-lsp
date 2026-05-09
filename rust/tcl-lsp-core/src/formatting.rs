//! Formatting provider — stub Rust port of
//! `lsp/features/formatting.py`.
//!
//! Returns an empty edit list — the actual formatter port lives
//! under the separate `F*` track (`tcl-formatter` crate, per
//! `core/formatting/`).  Once that crate lands, this module
//! becomes a thin LSP adapter that calls into it; until then,
//! we advertise the capability and return no-op edits so the
//! editor knows the request is supported but the document is
//! already-formatted.
//!
//! What is *deferred* (planned as `S-formatting-rich` /
//! `F-tcl-formatter`):
//!
//! * Full `tcl-formatter` crate port, mirroring
//!   `core/formatting/`.
//! * Indentation alignment, brace placement, comment-block
//!   reflow.
//! * Range-formatting (currently shares the no-op stub).

use crate::definition::LspRange;
use crate::rename::TextEdit;

/// Compute formatting edits for the entire document.
///
/// Stub implementation — always returns an empty `Vec`.
#[must_use]
pub fn formatting(_source: &str) -> Vec<TextEdit> {
    Vec::new()
}

/// Compute formatting edits for a range within the document.
///
/// Stub implementation — always returns an empty `Vec`.
#[must_use]
pub fn range_formatting(_source: &str, _range: LspRange) -> Vec<TextEdit> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting_returns_empty_edit_list() {
        assert!(formatting("proc foo {} {}\n").is_empty());
    }

    #[test]
    fn range_formatting_returns_empty_edit_list() {
        assert!(range_formatting(
            "proc foo {} {}\n",
            LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 0,
                end_character: 5,
            },
        )
        .is_empty(),);
    }
}
