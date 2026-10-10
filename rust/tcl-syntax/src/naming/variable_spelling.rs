// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Literal source proposals for one exact unqualified scalar operand.

use super::{NativeNameProtocol, NativeNameQualification};

/// A literal name word and a complete variable reference that reproduce the
/// same native operand. This grants no cell, name availability, variable
/// lifetime, observer, command selection or refactoring permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeScalarSourceSpelling {
    name_word: String,
    reference: String,
}

impl NativeScalarSourceSpelling {
    #[must_use]
    pub fn name_word(&self) -> &str {
        &self.name_word
    }

    #[must_use]
    pub fn reference(&self) -> &str {
        &self.reference
    }
}

/// Render an exact local scalar name and reference under the complete source
/// grammar and native string recipe. Qualification, array interpretation,
/// clipped operands and unrepresentable source text are unavailable. The
/// shared variable scanner validates the complete reference independently.
#[must_use]
pub fn native_scalar_source_spelling(
    original: &[u8],
    channel: tcl_lexer::SourceChannel,
    config: tcl_lexer::LexerConfig,
    protocol: NativeNameProtocol,
) -> Option<NativeScalarSourceSpelling> {
    let input = protocol.combined_variable_input(original);
    if input.element().is_some()
        || input.root().qualification() != NativeNameQualification::Unqualified
        || input.root().selected() != original
    {
        return None;
    }
    let literal = crate::backslash::native_literal_source_text(
        original,
        channel,
        protocol.string_protocol(),
    )?;
    let reference = format!("${{{literal}}}");
    let parsed = tcl_lexer::word_parts::scan_var_ref(reference.as_bytes(), 0, config).ok()??;
    if parsed.next != reference.len() || parsed.index.is_some() {
        return None;
    }
    let reproduced = crate::backslash::native_source_literal_bytes(
        parsed.name,
        channel,
        protocol.string_protocol(),
    )
    .ok()?;
    if reproduced.as_ref() != original {
        return None;
    }
    let name_word = crate::backslash::native_literal_source_word(
        original,
        channel,
        config,
        protocol.string_protocol(),
    )?;
    Some(NativeScalarSourceSpelling {
        name_word,
        reference,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::DialectProfile;
    use tcl_lexer::{LexerConfig, SourceChannel};

    #[test]
    fn scalar_source_proposals_preserve_native_operand_and_complete_reference() {
        // Implementation contract: naming.source.native-scalar-source-spelling
        // docs/design/analysis/name-resolution-proofs/native-scalar-source-spelling.md
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let point = tcl_dialect::model::DialectPoint::of_dialect_name(Some(dialect)).unwrap();
            let profile = if dialect == "jim" {
                DialectProfile::projected_from_point("jim", &[], "Jim", point)
            } else {
                DialectProfile::find(dialect).unwrap().clone()
            };
            let protocol = NativeNameProtocol::for_point(point).unwrap();
            let config = LexerConfig::for_profile(Some(&profile));
            let value = crate::backslash::native_source_literal_bytes(
                "café".as_bytes(),
                SourceChannel::Document,
                protocol.string_protocol(),
            )
            .unwrap();
            let proposal =
                native_scalar_source_spelling(&value, SourceChannel::Document, config, protocol)
                    .unwrap();
            assert_eq!(proposal.reference(), "${café}");
            for unavailable in [b"::x".as_slice(), b"a(x)", b"x}suffix", b"\xed\xa0\x80"] {
                assert!(
                    native_scalar_source_spelling(
                        unavailable,
                        SourceChannel::Document,
                        config,
                        protocol,
                    )
                    .is_none(),
                    "{dialect} {unavailable:?}"
                );
            }
            assert_eq!(
                native_scalar_source_spelling(
                    b"x\0tail",
                    SourceChannel::Document,
                    config,
                    protocol,
                )
                .is_some(),
                protocol.is_jim084(),
                "{dialect}",
            );
        }
    }
}
