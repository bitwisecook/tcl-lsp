//! Semantic-tokens provider — stub Rust port of
//! `lsp/features/_semantic_tokens/`.
//!
//! Returns an empty token list.  The Python implementation is
//! a 3370-LOC multi-module subsystem (`_api.py`, `_collect.py`,
//! `_format_args.py`, `_bigip.py`, `_primitives.py`,
//! `_constants.py`) that surfaces classifications for every
//! command head, argument role, format-string component,
//! `BigIP` URI segment, etc.  Porting it is a major effort
//! and is tracked under `S-semantic-tokens-rich` (the full
//! port + delta encoding).
//!
//! Advertising the capability with an empty `legend` and
//! always returning empty `data` is the LSP-spec-compliant
//! way to say "I support semantic tokens but have nothing
//! to highlight right now"; once the rich port lands the
//! same wiring grows a real legend and computes deltas.

/// Encoded semantic-tokens response.  The `data` array is
/// the LSP packed integer encoding (5 ints per token: line
/// delta, column delta, length, type, modifiers).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SemanticTokens {
    /// Packed integer data.  Empty in the stub.
    pub data: Vec<u32>,
}

/// The token-type / token-modifier legend the server
/// advertises during `initialize`.  Empty in the stub.
#[must_use]
pub fn legend_token_types() -> Vec<&'static str> {
    Vec::new()
}

/// Token-modifiers part of the legend.  Empty in the stub.
#[must_use]
pub fn legend_token_modifiers() -> Vec<&'static str> {
    Vec::new()
}

/// Compute semantic tokens for the entire document.
///
/// Stub — always returns empty `data`.
#[must_use]
pub fn full(_source: &str) -> SemanticTokens {
    SemanticTokens::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_returns_empty_data() {
        assert!(full("proc foo {} {}\n").data.is_empty());
    }

    #[test]
    fn legend_is_empty_in_stub() {
        assert!(legend_token_types().is_empty());
        assert!(legend_token_modifiers().is_empty());
    }
}
