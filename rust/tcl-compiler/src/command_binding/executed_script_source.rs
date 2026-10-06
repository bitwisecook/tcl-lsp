// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Executed script bytes and their truthful source relationship.

use super::{Arc, CommandAllocationSite, MaterialisedSourceKind, SourceOriginId};
use crate::ir::{Provenance, WordExpr};
use crate::registry_invocation::{EffectiveInvocationWord, effective_invocation_word};
use tcl_lexer::LexerConfig;
use tcl_syntax::word_rules::WordValueRules;

/// Mapping from executed script bytes into their owning source instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutedScriptMapping {
    /// Every byte occurs unchanged at this base in the retained source origin.
    Contiguous {
        /// First executable byte, independent of surrounding word delimiters.
        base: u32,
    },
    /// Substitution, escape decoding or concatenation created a new source.
    Materialised,
}

/// One evaluated body, shared by word, list and concatenated-script consumers.
///
/// ```
/// use std::sync::Arc;
/// use tcl_compiler::command_binding::{CommandAllocationSite, ExecutedScriptSource, SourceOriginId};
/// let parent = CommandAllocationSite {
///     source: Arc::new(SourceOriginId::authored(&Arc::from("eval $body"))),
///     offset: 0,
/// };
/// let body = ExecutedScriptSource::materialised(parent, vec![0], "set x 1");
/// assert_eq!(body.text.try_text().unwrap(), "set x 1");
/// assert_eq!(body.base(), 0);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExecutedScriptSource {
    /// Exact bytes received by the native script parser after argv evaluation.
    pub text: tcl_lexer::SourceImage,
    /// Semantic source instance; offsets never masquerade as another origin.
    pub origin: Arc<SourceOriginId>,
    /// Proved correspondence to the retained source instance.
    pub mapping: ExecutedScriptMapping,
}

impl ExecutedScriptSource {
    /// Retain an unchanged slice only after checking its owning source bytes.
    #[must_use]
    pub fn contiguous(origin: Arc<SourceOriginId>, text: &str, base: u32) -> Option<Self> {
        let start = usize::try_from(base).ok()?;
        let end = start.checked_add(text.len())?;
        (origin.source_image().bytes().get(start..end) == Some(text.as_bytes())).then(|| Self {
            text: native_image(text),
            origin,
            mapping: ExecutedScriptMapping::Contiguous { base },
        })
    }

    /// Select a frozen body value and prove a contiguous map only when the
    /// original literal word's bytes agree with its evaluated value.
    #[must_use]
    pub fn from_word(
        parent: CommandAllocationSite,
        argument: usize,
        word: &WordExpr,
        value: &str,
        config: LexerConfig,
    ) -> Self {
        let base = source_text(&parent.source)
            .and_then(|source| Self::literal_word_base(source, word, value, config));
        if let Some(base) = base {
            return Self {
                text: native_image(value),
                origin: parent.source,
                mapping: ExecutedScriptMapping::Contiguous { base },
            };
        }
        Self::materialised(parent, vec![argument], value)
    }

    /// Prove the contiguous source coordinate of an evaluated literal script.
    /// The lexer owns token extent; decoded or substituted values receive no
    /// authored coordinate. This is also the inline compiler's mapping seam.
    #[must_use]
    pub fn literal_word_base(
        source: &str,
        word: &WordExpr,
        value: &str,
        config: LexerConfig,
    ) -> Option<u32> {
        let rules = WordValueRules::from_config(&config);
        (word.source().provenance == Provenance::Source
            && effective_invocation_word(word, config.escapes, rules)
                == EffectiveInvocationWord::Literal(value.to_owned()))
        .then(|| literal_body_base(source, word, value))
        .flatten()
    }

    /// Retain a computed or concatenated value without inventing authored spans.
    #[must_use]
    pub fn materialised(parent: CommandAllocationSite, arguments: Vec<usize>, value: &str) -> Self {
        Self::materialised_image(parent, arguments, native_image(value))
    }

    /// Retain an original byte script without a Unicode projection.
    #[must_use]
    pub fn materialised_image(
        parent: CommandAllocationSite,
        arguments: Vec<usize>,
        text: tcl_lexer::SourceImage,
    ) -> Self {
        Self {
            origin: Arc::new(SourceOriginId::derived_image(
                parent,
                arguments,
                text.clone(),
                MaterialisedSourceKind::Script,
            )),
            text,
            mapping: ExecutedScriptMapping::Materialised,
        }
    }

    /// Prove an unchanged byte slice while preserving its parser input channel.
    #[must_use]
    pub fn contiguous_image(
        origin: Arc<SourceOriginId>,
        text: tcl_lexer::SourceImage,
        base: u32,
    ) -> Option<Self> {
        let start = usize::try_from(base).ok()?;
        let end = start.checked_add(text.len())?;
        (origin.source_image().bytes().get(start..end) == Some(text.bytes())).then_some(Self {
            text,
            origin,
            mapping: ExecutedScriptMapping::Contiguous { base },
        })
    }

    /// Checked advisory text; the original byte script remains authoritative.
    ///
    /// # Errors
    /// Returns the retained bytes' UTF-8 failure.
    pub fn try_text(&self) -> Result<&str, std::str::Utf8Error> {
        self.text.try_text()
    }

    /// Select a script from an evaluated list through the actual list grammar.
    /// Literal element bytes retain a contiguous source map; decoded elements
    /// receive a distinct materialised origin instead of an estimated offset.
    #[must_use]
    pub fn list_element(
        &self,
        parent: CommandAllocationSite,
        argument: usize,
        element: usize,
        rules: WordValueRules,
    ) -> Option<Self> {
        let text = self.try_text().ok()?;
        let values = rules.split_list(text).ok()?;
        let value = values.get(element)?.as_ref();
        if let ExecutedScriptMapping::Contiguous { base } = self.mapping
            && let Some(offset) = literal_list_element_offset(text, element, value)
            && let Some(base) = base.checked_add(offset)
        {
            return Some(Self {
                text: native_image(value),
                origin: Arc::clone(&self.origin),
                mapping: ExecutedScriptMapping::Contiguous { base },
            });
        }
        Some(Self::materialised(parent, vec![argument, element], value))
    }

    /// Offset used by segmentation in this actual source instance.
    #[must_use]
    pub const fn base(&self) -> u32 {
        match self.mapping {
            ExecutedScriptMapping::Contiguous { base } => base,
            ExecutedScriptMapping::Materialised => 0,
        }
    }
}

fn native_image(source: &str) -> tcl_lexer::SourceImage {
    tcl_lexer::SourceImage::native(Arc::<[u8]>::from(source.as_bytes()))
}

pub(super) fn source_text(origin: &SourceOriginId) -> Option<&str> {
    origin.try_text().ok()
}

fn literal_body_base(source: &str, word: &WordExpr, value: &str) -> Option<u32> {
    let span = tcl_lexer::word_span_at(source, word.source().span);
    let raw = source.get(span.as_range())?;
    if raw == value {
        return Some(span.start());
    }
    let interior = match raw.as_bytes().first()? {
        b'{' => tcl_syntax::word_rules::whole_braced_word(raw)?,
        b'"' if raw.ends_with('"') => raw.get(1..raw.len().checked_sub(1)?)?,
        _ => return None,
    };
    (interior == value)
        .then(|| span.start().checked_add(1))
        .flatten()
}

fn literal_list_element_offset(text: &str, element: usize, value: &str) -> Option<u32> {
    let mut next = 0;
    for index in 0..=element {
        let item = tcl_syntax::list::find_element(text, next).ok().flatten()?;
        next = item.next;
        if index == element && text.get(item.value.clone()) == Some(value) {
            return u32::try_from(item.value.start).ok();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operand(source: &str, config: LexerConfig) -> (CommandAllocationSite, WordExpr) {
        let command = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .into_iter()
            .next()
            .unwrap();
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &command,
        );
        (
            CommandAllocationSite {
                source: Arc::new(SourceOriginId::authored(&Arc::from(source))),
                offset: 0,
            },
            tokens.words()[1].clone(),
        )
    }

    #[test]
    fn inline_nested_script_mapping_retains_its_final_brace() {
        let source = "catch {expr {1 + 2}} value";
        let config = LexerConfig::for_dialect("tcl8.6");
        let (_, word) = operand(source, config);
        let body = "expr {1 + 2}";
        let base = ExecutedScriptSource::literal_word_base(source, &word, body, config).unwrap();
        assert_eq!(&source[base as usize..base as usize + body.len()], body);
        assert_eq!(source.as_bytes()[base as usize + body.len() - 1], b'}');
        assert!(
            ExecutedScriptSource::literal_word_base(source, &word, "expr {1 + 2", config).is_none()
        );
    }

    #[test]
    fn decoded_words_never_rebase_to_unmodified_authored_bytes() {
        for (source, value, contiguous) in [
            (r"catch {puts $literal}", "puts $literal", true),
            (r#"catch "puts literal""#, "puts literal", true),
            (r#"catch "puts \x41""#, "puts A", false),
            (r#"catch "$body""#, "puts A", false),
        ] {
            let config = LexerConfig::default();
            let (parent, word) = operand(source, config);
            let script = ExecutedScriptSource::from_word(parent.clone(), 0, &word, value, config);
            assert_eq!(script.text.bytes(), value.as_bytes());
            assert_eq!(
                matches!(script.mapping, ExecutedScriptMapping::Contiguous { .. }),
                contiguous
            );
            if contiguous {
                let start = usize::try_from(script.base()).unwrap();
                assert_eq!(source.get(start..start + value.len()), Some(value));
                assert_eq!(script.origin, parent.source);
            } else {
                assert_ne!(script.origin, parent.source);
                assert_eq!(script.base(), 0);
            }
        }
    }

    #[test]
    fn list_body_uses_list_grammar_and_preserves_literal_dollar_bytes() {
        let config = LexerConfig::default();
        let source = r#"switch {x "puts $literal" y "puts \x41"}"#;
        let (parent, word) = operand(source, config);
        let value = r#"x "puts $literal" y "puts \x41""#;
        let list = ExecutedScriptSource::from_word(parent.clone(), 0, &word, value, config);
        let rules = WordValueRules::from_config(&config);
        let literal = list.list_element(parent.clone(), 0, 1, rules).unwrap();
        assert_eq!(literal.text.bytes(), b"puts $literal");
        assert!(matches!(
            literal.mapping,
            ExecutedScriptMapping::Contiguous { .. }
        ));
        let escaped = list.list_element(parent, 0, 3, rules).unwrap();
        assert_eq!(escaped.text.bytes(), b"puts A");
        assert_eq!(escaped.mapping, ExecutedScriptMapping::Materialised);
    }

    #[test]
    fn brace_continuation_mapping_obeys_the_actual_word_policy() {
        let source = "catch {puts A\\
    B}";
        let c = LexerConfig::default();
        let (parent, word) = operand(source, c);
        let script = ExecutedScriptSource::from_word(parent, 0, &word, "puts A B", c);
        assert_eq!(script.mapping, ExecutedScriptMapping::Materialised);
        let jim = LexerConfig::for_profile(Some(
            tcl_registry::model::ingress::static_context_for("jim")
                .commands()
                .profile()
                .unwrap(),
        ));
        let (parent, word) = operand(source, jim);
        let value = source
            .strip_prefix("catch {")
            .unwrap()
            .strip_suffix('}')
            .unwrap();
        let script = ExecutedScriptSource::from_word(parent, 0, &word, value, jim);
        assert!(matches!(
            script.mapping,
            ExecutedScriptMapping::Contiguous { .. }
        ));
    }
    #[test]
    fn original_byte_body_keeps_native_channel_and_truthful_mapping() {
        let source = tcl_lexer::SourceImage::native(b"catch {set x \xff\0tail}".as_slice());
        let origin = Arc::new(SourceOriginId::authored_image(source.clone()));
        let body = tcl_lexer::SourceImage::native(b"set x \xff\0tail".as_slice());
        let retained =
            ExecutedScriptSource::contiguous_image(Arc::clone(&origin), body.clone(), 7).unwrap();
        assert_eq!(retained.text, body);
        assert!(retained.try_text().is_err());
        assert_eq!(retained.origin, origin);
        assert_eq!(
            retained.mapping,
            ExecutedScriptMapping::Contiguous { base: 7 }
        );
        let parent = CommandAllocationSite {
            source: origin,
            offset: 0,
        };
        let evaluated = ExecutedScriptSource::materialised_image(parent, vec![0], body.clone());
        assert_eq!(evaluated.text, body);
        assert_eq!(evaluated.origin.source_image(), &body);
        assert_eq!(evaluated.mapping, ExecutedScriptMapping::Materialised);
        assert_ne!(evaluated.origin, retained.origin);
    }

    #[test]
    fn document_origin_does_not_change_evaluated_body_parser_channel() {
        let origin = Arc::new(SourceOriginId::authored(&Arc::from("catch {set x 1}")));
        let retained = ExecutedScriptSource::contiguous(origin.clone(), "set x 1", 7).unwrap();
        assert_eq!(
            origin.source_image().channel(),
            tcl_lexer::SourceChannel::Document
        );
        assert_eq!(
            retained.text.channel(),
            tcl_lexer::SourceChannel::NativeValue
        );
        assert_eq!(retained.origin, origin);
    }
}
