// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
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
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum OriginalVariableRootOwner {
    Word(NativeWord),
    Executable(ExecutablePartArena),
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
        )
    }

    fn from_owner(
        owner: OriginalVariableRootOwner,
        exact_part: Span,
        rules: WordValueRules,
        policy: NamePolicyProtocol,
    ) -> Option<Self> {
        let arena = match &owner {
            OriginalVariableRootOwner::Word(word) => word.executable_parts(),
            OriginalVariableRootOwner::Executable(arena) => arena,
        };
        if rules != WordValueRules::from_config(&arena.config())
            || arena.config().escapes != policy.string_protocol().escape_syntax()
        {
            return None;
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
        let bytes = tcl_syntax::backslash::native_source_literal_bytes(
            arena.bytes(name)?,
            arena.image().channel(),
            policy.string_protocol(),
        )
        .ok()?
        .into_owned();
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
        })
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

    /// Native source-produced root units; variable names do not evaluate escapes.
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
