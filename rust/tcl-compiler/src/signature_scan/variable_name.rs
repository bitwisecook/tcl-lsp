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

//! Exact original variable roots, distinct from complete naming words.

use tcl_core_types::NameBytes;
use tcl_lexer::{
    ExecutablePart, ExecutablePartArena, NativeWord, SourceImage, Span, SpannedExecutablePart,
};
use tcl_syntax::{naming::NamePolicyProtocol, word_rules::WordValueRules};

/// A variable component of one authentic original word. Its root units and
/// separate-index flag are lexical facts; compiler versus runtime selection,
/// cell currency, observer closure and normal completion remain independent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SignatureSourceVariableRoot {
    owner: OriginalVariableRootOwner,
    part: SpannedExecutablePart,
    source_span: Span,
    bytes: NameBytes,
    rules: WordValueRules,
    policy: NamePolicyProtocol,
    formation: VariableRootFormation,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum OriginalVariableRootOwner {
    Word(NativeWord),
    Executable(ExecutablePartArena),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VariableRootFormation {
    NativeRecipe,
    LogicalSource,
}

impl SignatureSourceVariableRoot {
    /// Select an exact raw or mapped component extent from the retained arena.
    /// A substring, partial root, different grammar or string protocol declines.
    #[must_use]
    pub fn from_original_word(
        word: &NativeWord,
        exact_part: Span,
        rules: WordValueRules,
        policy: NamePolicyProtocol,
    ) -> Option<Self> {
        Self::from_owner(
            OriginalVariableRootOwner::Word(word.clone()),
            exact_part,
            rules,
            policy,
            VariableRootFormation::NativeRecipe,
        )
    }

    /// Retain an original expression/runtime-template component without
    /// inventing a complete word. Image and full scanner configuration agree.
    #[must_use]
    pub fn from_original_executable(
        arena: &ExecutablePartArena,
        image: &SourceImage,
        config: tcl_lexer::LexerConfig,
        exact_part: Span,
        rules: WordValueRules,
        policy: NamePolicyProtocol,
    ) -> Option<Self> {
        (arena.image() == image && arena.config() == config).then_some(())?;
        Self::from_owner(
            OriginalVariableRootOwner::Executable(arena.clone()),
            exact_part,
            rules,
            policy,
            VariableRootFormation::NativeRecipe,
        )
    }

    /// Exact readonly Document root under its original source grammar. This
    /// keeps the authored name abstraction separate from Native object units.
    pub(crate) fn from_original_logical_source_executable(
        arena: &ExecutablePartArena,
        image: &SourceImage,
        config: tcl_lexer::LexerConfig,
        exact_part: Span,
        rules: WordValueRules,
        policy: NamePolicyProtocol,
    ) -> Option<Self> {
        (arena.image() == image && arena.config() == config).then_some(())?;
        Self::from_owner(
            OriginalVariableRootOwner::Executable(arena.clone()),
            exact_part,
            rules,
            policy,
            VariableRootFormation::LogicalSource,
        )
    }

    fn from_owner(
        owner: OriginalVariableRootOwner,
        exact_part: Span,
        rules: WordValueRules,
        policy: NamePolicyProtocol,
        formation: VariableRootFormation,
    ) -> Option<Self> {
        let arena = match &owner {
            OriginalVariableRootOwner::Word(word) => word.executable_parts(),
            OriginalVariableRootOwner::Executable(arena) => arena,
        };
        if rules != WordValueRules::from_config(&arena.config()) {
            return None;
        }
        match formation {
            VariableRootFormation::NativeRecipe => {
                (arena.config().escapes == policy.string_protocol().escape_syntax())
                    .then_some(())?;
            }
            VariableRootFormation::LogicalSource => {
                (policy.authority() == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
                    && arena.image().channel() == tcl_lexer::SourceChannel::Document)
                    .then_some(())?;
                arena.image().try_text().ok()?;
            }
        }
        let mut selected = arena.all_parts().filter(|part| {
            matches!(part.part, ExecutablePart::Variable { .. })
                && (part.span == exact_part || arena.source_span(part) == Some(exact_part))
        });
        let part = selected.next()?;
        if selected.next().is_some() {
            return None;
        }
        let ExecutablePart::Variable { name, .. } = part.part else {
            return None;
        };
        let bytes = match formation {
            VariableRootFormation::NativeRecipe => {
                tcl_syntax::backslash::native_source_literal_bytes(
                    arena.bytes(name)?,
                    arena.image().channel(),
                    policy.string_protocol(),
                )
                .ok()?
                .into_owned()
            }
            VariableRootFormation::LogicalSource => tcl_syntax::backslash::source_literal_bytes(
                arena.bytes(name)?,
                tcl_lexer::SourceChannel::Document,
            )
            .into_owned(),
        };
        let part = part.clone();
        let source_span = arena.source_span(&part)?;
        drop(selected);
        Some(Self {
            owner,
            part,
            source_span,
            bytes: bytes.into(),
            rules,
            policy,
            formation,
        })
    }

    /// Reparse an exact component using this root's retained source authority.
    #[must_use]
    pub fn reparse_original_component(
        &self,
        arena: &ExecutablePartArena,
        image: &SourceImage,
        exact_part: Span,
    ) -> Option<Self> {
        (arena.image() == image
            && image.channel() == self.source_image().channel()
            && arena.config() == self.lexer_config())
        .then_some(())?;
        Self::from_owner(
            OriginalVariableRootOwner::Executable(arena.clone()),
            exact_part,
            self.rules,
            self.policy,
            self.formation,
        )
    }

    /// Literal root bytes under the same source authority as this root.
    #[must_use]
    pub fn source_literal_bytes<'a>(&self, raw: &'a [u8]) -> Option<std::borrow::Cow<'a, [u8]>> {
        match self.formation {
            VariableRootFormation::NativeRecipe => {
                tcl_syntax::backslash::native_source_literal_bytes(
                    raw,
                    self.source_image().channel(),
                    self.policy.string_protocol(),
                )
                .ok()
            }
            VariableRootFormation::LogicalSource => {
                std::str::from_utf8(raw).ok()?;
                Some(tcl_syntax::backslash::source_literal_bytes(
                    raw,
                    self.source_image().channel(),
                ))
            }
        }
    }

    /// Map a literal value extent through this root's retained source authority.
    #[must_use]
    pub fn source_literal_extent(
        &self,
        raw: &[u8],
        extent: std::ops::Range<usize>,
    ) -> Option<std::ops::Range<usize>> {
        match self.formation {
            VariableRootFormation::NativeRecipe => {
                tcl_syntax::backslash::native_source_literal_extent(
                    raw,
                    self.source_image().channel(),
                    self.policy.string_protocol(),
                    extent,
                )
            }
            VariableRootFormation::LogicalSource => tcl_syntax::backslash::source_literal_extent(
                raw,
                self.source_image().channel(),
                extent,
            ),
        }
    }

    pub(super) fn arena(&self) -> &ExecutablePartArena {
        match &self.owner {
            OriginalVariableRootOwner::Word(word) => word.executable_parts(),
            OriginalVariableRootOwner::Executable(arena) => arena,
        }
    }

    pub(super) fn index_parts(&self) -> Option<&[SpannedExecutablePart]> {
        let ExecutablePart::Variable {
            index: Some(index), ..
        } = self.part.part
        else {
            return None;
        };
        Some(self.arena().list(index))
    }

    /// Static original index data retains the independently selected index
    /// list. A nested read/command needs its actual evaluated producer instead.
    #[must_use]
    pub fn static_index_input(&self) -> Option<super::scope::SignatureSourceNameInput> {
        Some(super::scope::SignatureSourceNameInput::OriginalValue(
            super::scope::SignatureSourceNameValue::from_original_substitution_index(self)?,
        ))
    }

    /// Original root units; variable names do not evaluate escapes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.bytes.as_bytes()
    }
    /// Independently selected string/name recipe and authority.
    #[must_use]
    pub const fn policy(&self) -> NamePolicyProtocol {
        self.policy
    }
    /// Original source owner and its input channel.
    #[must_use]
    pub fn source_image(&self) -> &SourceImage {
        self.arena().image()
    }
    /// Exact complete variable component, including its substitution delimiters.
    #[must_use]
    pub fn part_span(&self) -> Span {
        self.part.span
    }
    /// Exact original lexical name extent, excluding substitution delimiters
    /// and the independently evaluated array index. This is readonly geometry,
    /// not a complete naming word or an edit grant.
    #[must_use]
    pub fn name_span(&self) -> Option<Span> {
        let ExecutablePart::Variable { name, .. } = self.part.part else {
            return None;
        };
        self.arena().bytes(name)?;
        Some(name)
    }
    /// The shared scanner's mapped token convention for this same component.
    #[must_use]
    pub const fn source_span(&self) -> Span {
        self.source_span
    }
    /// Full retained lexical configuration, including independent overrides.
    #[must_use]
    pub fn lexer_config(&self) -> tcl_lexer::LexerConfig {
        self.arena().config()
    }
    /// Retained original word owner; this is not a complete naming-word key.
    #[must_use]
    pub fn original_word(&self) -> Option<&NativeWord> {
        match &self.owner {
            OriginalVariableRootOwner::Word(word) => Some(word),
            OriginalVariableRootOwner::Executable(_) => None,
        }
    }
    /// A separately evaluated index exists. Its value is an independent producer.
    #[must_use]
    pub fn is_separate_array_root(&self) -> bool {
        matches!(
            self.part.part,
            ExecutablePart::Variable { index: Some(_), .. }
        )
    }
    /// Checked reporting only, never a byte-name reconstruction bridge.
    #[must_use]
    pub fn display(&self) -> Option<&str> {
        self.bytes.try_utf8().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::SubstFlags;

    #[test]
    fn original_root_literal_extents_retain_logical_and_native_units() {
        let mut config = tcl_lexer::LexerConfig::for_file_grammar(
            tcl_dialect::DialectProfile::plain_tcl().grammar,
        );
        let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        config.escapes = policy.string_protocol().escape_syntax();
        let image = SourceImage::document("${café🙂}");
        let arena = ExecutablePartArena::decompose(
            image.clone(),
            Span::new(0, u32::try_from(image.len()).unwrap()),
            SubstFlags::default(),
            config,
        )
        .unwrap();
        let part = &arena.list(arena.root())[0];
        let rules = WordValueRules::from_config(&config);
        let logical = SignatureSourceVariableRoot::from_original_logical_source_executable(
            &arena, &image, config, part.span, rules, policy,
        )
        .unwrap();
        let native = SignatureSourceVariableRoot::from_original_executable(
            &arena, &image, config, part.span, rules, policy,
        )
        .unwrap();
        let raw = "café🙂".as_bytes();
        assert_eq!(logical.bytes(), raw);
        assert_ne!(native.bytes(), raw);
        assert_eq!(
            logical.source_literal_extent(raw, 0..raw.len()),
            Some(0..raw.len())
        );
        assert_eq!(
            native.source_literal_extent(raw, 0..native.bytes().len()),
            Some(0..raw.len())
        );
        assert_eq!(logical.source_literal_extent(raw, 0..raw.len() - 1), None);
        assert_eq!(native.source_literal_extent(raw, 0..raw.len()), None);
    }

    #[test]
    fn original_logical_roots_keep_exact_document_geometry_and_native_refusal() {
        // naming.variable.original-alias-source-template
        // docs/design/analysis/name-resolution-proofs/original-alias-source-template.md
        let config = tcl_lexer::LexerConfig::for_file_grammar(
            tcl_dialect::DialectProfile::plain_tcl().grammar,
        );
        let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        let image = SourceImage::document("$::ns::café($index)");
        let arena = ExecutablePartArena::decompose(
            image.clone(),
            Span::new(0, u32::try_from(image.len()).unwrap()),
            SubstFlags::default(),
            config,
        )
        .unwrap();
        let part = arena.list(arena.root()).first().unwrap();
        let rules = WordValueRules::from_config(&config);
        let root = SignatureSourceVariableRoot::from_original_logical_source_executable(
            &arena, &image, config, part.span, rules, policy,
        )
        .expect("authentic Logical lexical root");
        // C Tcl's `$` scan stops at the first non-ASCII byte: tclsh 8.6.18,
        // 9.0.4 and 9.1.0 read `$::ns::café($index)` as the scalar
        // `::ns::caf` followed by literal `é(...)` (`can't read "::ns::caf":
        // variable is array` when `::ns::caf` is an array). A Logical root
        // keeps that same lexical extent; it is not an array reference.
        assert_eq!(root.bytes(), "::ns::caf".as_bytes());
        assert_eq!(root.source_image(), &image);
        assert_eq!(root.lexer_config(), config);
        assert!(!root.is_separate_array_root());
        assert!(root.static_index_input().is_none());
        assert!(
            SignatureSourceVariableRoot::from_original_executable(
                &arena, &image, config, part.span, rules, policy,
            )
            .is_none()
        );
        for bad_span in [
            Span::new(part.span.start() + 1, part.span.end()),
            Span::new(0, 2),
        ] {
            assert!(
                SignatureSourceVariableRoot::from_original_logical_source_executable(
                    &arena, &image, config, bad_span, rules, policy,
                )
                .is_none()
            );
        }
        let mut changed = config;
        changed.strict_quoting = !changed.strict_quoting;
        assert!(
            SignatureSourceVariableRoot::from_original_logical_source_executable(
                &arena, &image, changed, part.span, rules, policy,
            )
            .is_none()
        );
        assert!(
            SignatureSourceVariableRoot::from_original_logical_source_executable(
                &arena,
                &SourceImage::document("$other"),
                config,
                part.span,
                rules,
                policy,
            )
            .is_none()
        );
        let native_image = SourceImage::native(b"$foo".as_slice());
        let native_arena = ExecutablePartArena::decompose(
            native_image.clone(),
            Span::new(0, 4),
            SubstFlags::default(),
            config,
        )
        .unwrap();
        assert!(
            SignatureSourceVariableRoot::from_original_logical_source_executable(
                &native_arena,
                &native_image,
                config,
                Span::new(0, 4),
                rules,
                policy,
            )
            .is_none()
        );
    }

    #[test]
    fn original_executable_root_keeps_channel_configuration_and_separate_index() {
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let dialect = tcl_registry::InvocationDialect::of_point(
                tcl_dialect::model::DialectPoint::of_dialect_name(Some(name)).unwrap(),
            );
            let policy = dialect.authored_name_policy().unwrap();
            let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
            let rules = WordValueRules::from_config(&config);
            let bytes = b"${v\0tail}";
            let image = SourceImage::native(bytes.as_slice());
            let arena = ExecutablePartArena::decompose(
                image.clone(),
                Span::new(0, 9),
                SubstFlags::default(),
                config,
            )
            .unwrap();
            let part = arena.list(arena.root()).first().unwrap();
            let root = SignatureSourceVariableRoot::from_original_executable(
                &arena, &image, config, part.span, rules, policy,
            )
            .unwrap();
            assert_eq!(root.bytes(), b"v\0tail");
            assert_eq!(root.name_span(), Some(Span::new(2, 8)));
            assert!(root.original_word().is_none());
            assert!(!root.is_separate_array_root());
            assert!(root.static_index_input().is_none());
            assert!(
                SignatureSourceVariableRoot::from_original_executable(
                    &arena,
                    &SourceImage::document(std::str::from_utf8(bytes).unwrap()),
                    config,
                    part.span,
                    rules,
                    policy
                )
                .is_none()
            );
            let mut mismatch = config;
            mismatch.escapes = if config.escapes == tcl_dialect::EscapeSyntax::Tcl84 {
                tcl_dialect::EscapeSyntax::Tcl86
            } else {
                tcl_dialect::EscapeSyntax::Tcl84
            };
            assert!(
                SignatureSourceVariableRoot::from_original_executable(
                    &arena,
                    &image,
                    mismatch,
                    part.span,
                    WordValueRules::from_config(&mismatch),
                    policy
                )
                .is_none()
            );
            assert!(
                SignatureSourceVariableRoot::from_original_executable(
                    &arena,
                    &image,
                    config,
                    Span::new(part.span.start() + 1, part.span.end()),
                    rules,
                    policy
                )
                .is_none()
            );
            for (source, expected) in [
                (b"$a(k\0tail)".as_slice(), Some(b"k\0tail".as_slice())),
                (b"$a()".as_slice(), Some(b"".as_slice())),
                (b"$a($k)".as_slice(), None),
            ] {
                let image = SourceImage::native(source);
                let arena = ExecutablePartArena::decompose(
                    image.clone(),
                    Span::new(0, u32::try_from(source.len()).unwrap()),
                    SubstFlags::default(),
                    config,
                )
                .unwrap();
                let part = arena.list(arena.root()).first().unwrap();
                let root = SignatureSourceVariableRoot::from_original_executable(
                    &arena, &image, config, part.span, rules, policy,
                )
                .unwrap();
                assert!(root.is_separate_array_root());
                assert_eq!(
                    root.static_index_input()
                        .as_ref()
                        .map(|input| input.bytes().to_vec()),
                    expected.map(<[u8]>::to_vec)
                );
            }
        }
    }
}
