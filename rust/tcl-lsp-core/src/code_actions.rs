//! Code-actions provider — stub Rust port of
//! `lsp/features/code_actions.py`.
//!
//! Returns an empty list.  The Python provider surfaces
//! quick-fixes for analyser diagnostics (insert missing brace,
//! replace command with similar, etc.); porting requires the
//! cached-analysis surface that `S-diagnostics` lands later
//! (deferred to `S-code-actions-rich`).

use crate::definition::LspRange;

/// One code-action entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeAction {
    /// Title shown in the editor.
    pub title: String,
    /// Edits the action would apply.
    pub edits: Vec<crate::rename::TextEdit>,
}

/// Compute code actions for a document range.
///
/// Stub — always returns empty.
#[must_use]
pub fn code_actions(_source: &str, _range: LspRange) -> Vec<CodeAction> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_actions_for_simple_source() {
        assert!(code_actions(
            "proc foo {} {}\n",
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
