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

//! Sealed static original naming words, independent of display and execution.

use tcl_core_types::NameBytes;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage, Span};
use tcl_syntax::{naming::NamePolicyProtocol, word_rules::WordValueRules};

/// One complete original static naming word and its independently formed units.
///
/// Its private producer preserves the source channel, complete lexical
/// configuration and independently selected string protocol. This receipt
/// grants no command publication, native activation or writable edit recipe.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SignatureSourceNameKey {
    original: NativeWord,
    bytes: NameBytes,
    rules: WordValueRules,
    policy: NamePolicyProtocol,
    provenance: SourceNameProvenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SourceNameProvenance {
    Native,
    Logical,
}

impl SignatureSourceNameKey {
    /// Capture a complete static lexical word through the canonical byte owner.
    /// Substitution, expansion or conflicting word/string rules abstain.
    #[must_use]
    pub fn from_original_native_word(
        original: &NativeWord,
        rules: WordValueRules,
        policy: NamePolicyProtocol,
    ) -> Option<Self> {
        if original.group().expand
            || rules != WordValueRules::from_config(&original.config())
            || original.config().escapes != policy.string_protocol().escape_syntax()
        {
            return None;
        }
        let words = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
            std::slice::from_ref(original),
            policy.string_protocol(),
        )
        .ok()?;
        Some(Self {
            original: original.clone(),
            bytes: NameBytes::from(words.literal(0)?),
            rules,
            policy,
            provenance: SourceNameProvenance::Native,
        })
    }

    /// Capture readonly Unicode source units under the actual Document grammar.
    /// The authored naming abstraction is independent of that grammar and does
    /// not supply a Native string, literal pool, cell or execution capability.
    #[doc(hidden)]
    #[must_use]
    pub fn from_original_logical_source_word(
        original: &NativeWord,
        rules: WordValueRules,
        policy: NamePolicyProtocol,
    ) -> Option<Self> {
        if policy.authority() != tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
            || rules != WordValueRules::from_config(&original.config())
        {
            return None;
        }
        let bytes = tcl_syntax::word_rules::original_static_word_unicode_value(original)?;
        Some(Self {
            original: original.clone(),
            bytes: bytes.into(),
            rules,
            policy,
            provenance: SourceNameProvenance::Logical,
        })
    }

    /// Whether this key came from the selected Logical document producer.
    #[must_use]
    pub const fn is_logical_source(&self) -> bool {
        matches!(self.provenance, SourceNameProvenance::Logical)
    }

    /// Source extent of byte-identical literal Logical units in the complete word.
    #[must_use]
    pub fn logical_source_literal_extent(&self, extent: std::ops::Range<usize>) -> Option<Span> {
        let word = &self.original;
        if !self.is_logical_source()
            || word.image().channel() != tcl_lexer::SourceChannel::Document
            || word.group().expand
        {
            return None;
        }
        let plan =
            tcl_lexer::native_script_words_in(word.image().clone(), word.span(), word.config())
                .ok()?;
        let [command] = plan.commands.as_slice() else {
            return None;
        };
        let [reparsed] = command.words.as_slice() else {
            return None;
        };
        if reparsed != word {
            return None;
        }
        let content = word.content_span().ok()?;
        let raw = word.image().bytes().get(content.as_range())?;
        if raw != self.bytes()
            || tcl_syntax::word_rules::original_static_word_unicode_value(word)?.as_slice() != raw
        {
            return None;
        }
        let text = std::str::from_utf8(raw).ok()?;
        if extent.start > extent.end
            || extent.end > raw.len()
            || !text.is_char_boundary(extent.start)
            || !text.is_char_boundary(extent.end)
        {
            return None;
        }
        Some(Span::new(
            content
                .start()
                .checked_add(u32::try_from(extent.start).ok()?)?,
            content
                .start()
                .checked_add(u32::try_from(extent.end).ok()?)?,
        ))
    }

    /// Actual produced units; Unicode reporting cannot reconstruct this value.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.bytes.as_bytes()
    }

    /// The independently selected name provider, including its authority.
    #[must_use]
    pub const fn policy(&self) -> NamePolicyProtocol {
        self.policy
    }

    /// Original producer image and input channel.
    #[must_use]
    pub fn source_image(&self) -> &SourceImage {
        self.original.image()
    }

    /// Complete grouped original word extent, without an edit grant.
    #[must_use]
    pub fn span(&self) -> Span {
        self.original.span()
    }

    /// Retained complete lexical configuration.
    #[must_use]
    pub fn lexer_config(&self) -> LexerConfig {
        self.original.config()
    }

    /// Retained value/list rules, independently of command semantics.
    #[must_use]
    pub const fn word_value_rules(&self) -> WordValueRules {
        self.rules
    }

    /// Sealed original lexical producer.
    #[must_use]
    pub fn original_word(&self) -> &NativeWord {
        &self.original
    }

    /// Exact UTF-8 advice only; unavailable display does not withdraw identity.
    #[must_use]
    pub fn display(&self) -> Option<&str> {
        self.bytes.try_utf8().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(source: &[u8], policy: NamePolicyProtocol) -> Option<SignatureSourceNameKey> {
        let config = match policy.recipe() {
            tcl_syntax::naming::NativeNameProtocol::C(version) => {
                LexerConfig::from_grammar(tcl_dialect::grammar_of_dialect_name(Some(&format!(
                    "tcl{}",
                    version.version_string()
                ))))
            }
            tcl_syntax::naming::NativeNameProtocol::Jim084 => {
                LexerConfig::from_grammar(tcl_dialect::grammar_of_dialect_name(Some("jimtcl")))
            }
        };
        let plan = tcl_lexer::native_script_words_in(
            SourceImage::native(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        SignatureSourceNameKey::from_original_native_word(
            &plan.commands[0].words[0],
            WordValueRules::from_config(&config),
            policy,
        )
    }

    #[test]
    fn original_logical_name_units_keep_actual_grammar_without_native_word_admission() {
        // naming.variable.original-alias-source-template
        // docs/design/analysis/name-resolution-proofs/original-alias-source-template.md
        let config =
            LexerConfig::for_file_grammar(tcl_dialect::DialectProfile::plain_tcl().grammar);
        let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        let image = SourceImage::document(r"name\U0001F600");
        let plan = tcl_lexer::native_script_words_in(
            image.clone(),
            Span::new(0, u32::try_from(image.len()).unwrap()),
            config,
        )
        .unwrap();
        let original = &plan.commands[0].words[0];
        let key = SignatureSourceNameKey::from_original_logical_source_word(
            original,
            WordValueRules::from_config(&config),
            policy,
        )
        .expect("actual Logical document value");
        assert_eq!(key.bytes(), "name😀".as_bytes());
        assert_eq!(key.source_image(), &image);
        assert_eq!(key.lexer_config(), config);
        assert_eq!(key.policy(), policy);
        assert!(
            SignatureSourceNameKey::from_original_native_word(
                original,
                WordValueRules::from_config(&config),
                policy,
            )
            .is_none(),
            "source advice cannot supply the mismatched Native recipe"
        );
        for source in ["$name", "{*}{name}", r"name\uD800"] {
            let image = SourceImage::document(source);
            let plan = tcl_lexer::native_script_words_in(
                image.clone(),
                Span::new(0, u32::try_from(image.len()).unwrap()),
                config,
            )
            .unwrap();
            assert!(
                SignatureSourceNameKey::from_original_logical_source_word(
                    &plan.commands[0].words[0],
                    WordValueRules::from_config(&config),
                    policy,
                )
                .is_none(),
                "{source}"
            );
        }
        let native = NamePolicyProtocol::for_native_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some("tcl9.0")).unwrap(),
        )
        .unwrap();
        assert!(
            SignatureSourceNameKey::from_original_logical_source_word(
                original,
                WordValueRules::from_config(&config),
                native,
            )
            .is_none()
        );
        assert!(
            SignatureSourceNameKey::from_original_logical_source_word(
                original,
                WordValueRules::JIM,
                policy,
            )
            .is_none()
        );
        let native_image = SourceImage::native(b"name".as_slice());
        let native_plan =
            tcl_lexer::native_script_words_in(native_image.clone(), Span::new(0, 4), config)
                .unwrap();
        assert!(
            SignatureSourceNameKey::from_original_logical_source_word(
                &native_plan.commands[0].words[0],
                WordValueRules::from_config(&config),
                policy,
            )
            .is_none()
        );
    }

    #[test]
    fn logical_literal_extent_requires_the_complete_unchanged_word() {
        let config =
            LexerConfig::for_file_grammar(tcl_dialect::DialectProfile::plain_tcl().grammar);
        let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        for (source, expected) in [
            ("café🙂", Some(Span::new(0, 9))),
            ("{café🙂}", Some(Span::new(1, 10))),
            (r"caf\u00e9", None),
        ] {
            let image = SourceImage::document(source);
            let plan = tcl_lexer::native_script_words_in(
                image,
                Span::new(0, u32::try_from(source.len()).unwrap()),
                config,
            )
            .unwrap();
            let key = SignatureSourceNameKey::from_original_logical_source_word(
                &plan.commands[0].words[0],
                WordValueRules::from_config(&config),
                policy,
            )
            .unwrap();
            assert!(key.is_logical_source());
            assert_eq!(
                key.logical_source_literal_extent(0..key.bytes().len()),
                expected
            );
            if expected.is_some() {
                assert!(key.logical_source_literal_extent(4..5).is_none());
                assert!(
                    key.logical_source_literal_extent(0..key.bytes().len() + 1)
                        .is_none()
                );
            }
        }
    }

    #[test]
    fn logical_literal_extent_keeps_document_offsets_for_command_words() {
        let source = "set {café🙂} 1";
        let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        let plain = LexerConfig::for_file_grammar(tcl_dialect::DialectProfile::plain_tcl().grammar);
        for base_offset in [0, 77] {
            let config = LexerConfig {
                base_offset,
                ..plain
            };
            let image = SourceImage::document(source);
            let script_words = tcl_lexer::native_script_words_in(
                image,
                Span::new(0, u32::try_from(source.len()).unwrap()),
                config,
            )
            .unwrap();
            let key = SignatureSourceNameKey::from_original_logical_source_word(
                &script_words.commands[0].words[1],
                WordValueRules::from_config(&config),
                policy,
            )
            .unwrap();
            assert_eq!(
                key.logical_source_literal_extent(0..key.bytes().len()),
                Some(Span::new(5, 14))
            );
        }
    }

    #[test]
    fn native_key_does_not_grant_logical_source_extent() {
        let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        let config =
            LexerConfig::from_grammar(tcl_dialect::grammar_of_dialect_name(Some("tcl8.6")));
        let image = SourceImage::document("ascii");
        let plan = tcl_lexer::native_script_words_in(image, Span::new(0, 5), config).unwrap();
        let word = &plan.commands[0].words[0];
        let rules = WordValueRules::from_config(&config);
        let native =
            SignatureSourceNameKey::from_original_native_word(word, rules, policy).unwrap();
        let logical =
            SignatureSourceNameKey::from_original_logical_source_word(word, rules, policy).unwrap();
        assert!(!native.is_logical_source());
        assert!(logical.is_logical_source());
        assert_ne!(native, logical);
        assert_eq!(native.logical_source_literal_extent(0..5), None);
        assert_eq!(
            logical.logical_source_literal_extent(0..5),
            Some(Span::new(0, 5))
        );
    }

    #[test]
    fn static_name_key_keeps_raw_zero_distinct_from_escaped_native_zero() {
        for version in tcl_dialect::TclVersion::ALL {
            let policy = NamePolicyProtocol::authored_tcl(version);
            let raw = key(b"p\0tail", policy).unwrap();
            let escaped = key(br"p\u0000tail", policy).unwrap();
            assert_eq!(raw.bytes(), b"p\0tail");
            assert_eq!(escaped.bytes(), b"p\xc0\x80tail");
            assert_ne!(raw, escaped);
        }
    }

    #[test]
    fn opaque_static_units_retain_identity_without_unicode_advice() {
        let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        let key = key(br"p\uD800", policy).unwrap();
        assert_eq!(key.bytes(), b"p\xed\xa0\x80");
        assert!(key.display().is_none());
    }

    #[test]
    fn name_key_declines_dynamic_expanded_or_conflicting_protocol() {
        let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        assert!(key(b"$name", policy).is_none());
        assert!(key(b"{*}{p}", policy).is_none());
        let original = key(b"p", policy).unwrap();
        assert!(
            SignatureSourceNameKey::from_original_native_word(
                original.original_word(),
                WordValueRules::JIM,
                policy,
            )
            .is_none()
        );
        assert!(
            SignatureSourceNameKey::from_original_native_word(
                original.original_word(),
                WordValueRules::TCL,
                NamePolicyProtocol::authored_jim084(),
            )
            .is_none()
        );
    }
}
