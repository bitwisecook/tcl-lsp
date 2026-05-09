//! Inlay-hints provider — stub Rust port of
//! `lsp/features/inlay_hints.py`.
//!
//! Returns an empty list.  The Python provider surfaces
//! parameter-name hints at proc call sites; porting requires
//! the same command-segmenter / argument-resolver machinery as
//! signature-help (deferred to `S-inlay-hints-rich`).

use crate::definition::LspRange;

/// One inlay-hint entry — position plus label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlayHint {
    /// Anchor position for the hint (typically the start of
    /// an argument token).
    pub position_line: u32,
    /// Anchor character.
    pub position_character: u32,
    /// Hint label (e.g. `name:`).
    pub label: String,
}

/// Compute inlay hints for a document range.
///
/// Stub — always returns empty.
#[must_use]
pub fn inlay_hints(_source: &str, _range: LspRange) -> Vec<InlayHint> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_hints_for_simple_source() {
        assert!(inlay_hints(
            "set x 1\n",
            LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 1,
                end_character: 0,
            },
        )
        .is_empty());
    }
}
