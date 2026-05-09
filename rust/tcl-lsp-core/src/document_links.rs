//! Document-links provider — stub Rust port of
//! `lsp/features/document_links.py`.
//!
//! Returns an empty list. The Python provider surfaces
//! `source` / `package require` arguments as clickable links;
//! porting that behaviour requires a workspace path-resolver
//! (deferred to `S-document-links-rich`).

/// Compute document links for a document.
///
/// Stub — always returns empty.
#[must_use]
pub fn document_links(_source: &str) -> Vec<DocumentLink> {
    Vec::new()
}

/// One link in a document — target URI plus the source range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentLink {
    /// Source-range start line.
    pub start_line: u32,
    /// Source-range start character.
    pub start_character: u32,
    /// Source-range end line.
    pub end_line: u32,
    /// Source-range end character.
    pub end_character: u32,
    /// Target URI string.
    pub target: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_links_for_simple_source() {
        assert!(document_links("set x 1\n").is_empty());
    }
}
