//! Code-lens provider — stub Rust port of
//! `lsp/features/code_lens.py`.
//!
//! Returns an empty list.  The Python provider surfaces
//! per-proc reference counts above each definition; porting
//! requires the workspace-wide reference index (deferred to
//! `S-code-lens-rich`).

use crate::definition::LspRange;

/// One code-lens entry — anchor range plus a command label
/// (Python uses commands like `Show Type`/`Show References`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeLens {
    /// Anchor range for the lens.
    pub range: LspRange,
    /// Command label shown to the user.
    pub command_title: String,
    /// Command identifier sent on click.
    pub command: String,
}

/// Compute code lenses for a document.
///
/// Stub — always returns empty.
#[must_use]
pub fn code_lenses(_source: &str) -> Vec<CodeLens> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_lenses_for_simple_source() {
        assert!(code_lenses("proc foo {} {}\n").is_empty());
    }
}
