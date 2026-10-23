// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original configured pack rows retain selected-word geometry, never rejection.

use crate::segmenter::SegmentedCommand;
use std::sync::Arc;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage, Span, WordKind};

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalStatement {
    image: SourceImage,
    config: LexerConfig,
    ordinal: usize,
    row_count: usize,
    words: Arc<[NativeWord]>,
    texts: Arc<[String]>,
}

/// An actual local CST word and complete original statement vector.
/// A detached fragment supplies no original document, DSL role or notice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalPackSourceWord {
    statement: Arc<LocalStatement>,
    ordinal: usize,
}
impl LocalPackSourceWord {
    /// Original local buffer/config and exact cached statement/word position.
    /// This reborrow verifies syntax provenance, not source-file or loader role.
    #[must_use]
    pub fn matches_fragment(
        &self,
        source: &str,
        config: LexerConfig,
        rows: usize,
        statement: usize,
        word: usize,
    ) -> bool {
        self.statement.image.bytes() == source.as_bytes()
            && self.statement.config == config
            && self.statement.row_count == rows
            && self.statement.ordinal == statement
            && self.ordinal == word
    }
    /// The loader's unchanged captured word text from its actual CST row.
    #[must_use]
    pub fn captured_text(&self) -> &str {
        &self.statement.texts[self.ordinal]
    }
    /// Original braced grouping, independently of evaluated value shape.
    #[must_use]
    pub fn is_braced(&self) -> bool {
        self.native().group().kind == WordKind::Braced
    }
    /// Actual local opening-token line, counting the loader's LF convention.
    #[must_use]
    pub fn local_line(&self) -> Option<u32> {
        line_at(&self.statement.image, self.native().span().start()).map(|line| line - 1)
    }
    fn native(&self) -> &NativeWord {
        &self.statement.words[self.ordinal]
    }
    /// Bind a whole document selected by the caller, without DSL/notice authority.
    #[must_use]
    pub fn in_document(&self) -> OriginalPackSourceWord {
        OriginalPackSourceWord {
            local: self.clone(),
            root: self.statement.image.clone(),
            root_config: self.statement.config,
            base: 0,
            parent: None,
            depth: 0,
        }
    }
    /// Bind only through the actual original braced parent word. Cooked, changed,
    /// detached or differently configured buffers cannot recover a full source.
    #[must_use]
    pub fn in_body(&self, parent: &OriginalPackSourceWord) -> Option<OriginalPackSourceWord> {
        let container = parent.local.native();
        if parent.depth >= 32
            || container.group().kind != WordKind::Braced
            || self.statement.config != parent.local.statement.config.nested()
            || parent.local.captured_text().as_bytes() != self.statement.image.bytes()
            || parent
                .local
                .statement
                .image
                .bytes()
                .get(container.content_span().ok()?.as_range())
                != Some(self.statement.image.bytes())
        {
            return None;
        }
        let base = parent
            .base
            .checked_add(container.content_span().ok()?.start())?;
        let candidate = OriginalPackSourceWord {
            local: self.clone(),
            root: parent.root.clone(),
            root_config: parent.root_config,
            base,
            parent: Some(Arc::new(parent.clone())),
            depth: parent.depth.checked_add(1)?,
        };
        candidate
            .matches_source(&candidate.root, candidate.root_config)
            .then_some(candidate)
    }
}

/// Full original document and the actual selected braced-container path to one
/// CST word. This is geometry only: no property/flag role, rejection, diagnostic,
/// Native execution or permission to edit is proved by a parser word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalPackSourceWord {
    local: LocalPackSourceWord,
    root: SourceImage,
    root_config: LexerConfig,
    base: u32,
    parent: Option<Arc<OriginalPackSourceWord>>,
    depth: u8,
}
impl OriginalPackSourceWord {
    /// Whole unchanged source image/channel for the selected original word.
    #[must_use]
    pub const fn image(&self) -> &SourceImage {
        &self.root
    }
    /// Complete original whole-document grammar/configuration.
    #[must_use]
    pub const fn config(&self) -> LexerConfig {
        self.root_config
    }
    /// Original word ordinal in the complete actual statement vector.
    #[must_use]
    pub const fn ordinal(&self) -> usize {
        self.local.ordinal
    }
    /// Original statement ordinal in its actual selected source container.
    #[must_use]
    pub fn statement_ordinal(&self) -> usize {
        self.local.statement.ordinal
    }
    /// Complete local original argv; positions belong to the held local image,
    /// not a fabricated absolute `NativeWord` or an evaluated command vector.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.local.statement.words
    }
    /// The actual loader/CST word text, independently of a source rejection.
    #[must_use]
    pub fn captured_text(&self) -> &str {
        self.local.captured_text()
    }
    /// Exact captured original statement head, independently of DSL vocabulary.
    #[must_use]
    pub fn captured_statement_head(&self) -> &str {
        &self.local.statement.texts[0]
    }
    /// Whether actual captured header/argument values agree with this complete
    /// original literal statement vector. This is source correspondence only,
    /// never a Native invocation, entered frame or loader-role permission.
    #[must_use]
    pub fn matches_literal_statement_capture(&self, head: &str, args: &[String]) -> bool {
        let texts = &self.local.statement.texts;
        texts.len() == args.len() + 1
            && texts.first().is_some_and(|text| text == head)
            && texts
                .iter()
                .skip(1)
                .zip(args)
                .all(|(text, value)| text == value)
            && self
                .local
                .statement
                .words
                .iter()
                .zip(texts.iter())
                .all(|(word, text)| {
                    !word.group().expand
                        && super::pack_source_word_is_literal(
                            text,
                            word.group().kind == WordKind::Braced,
                        )
                })
    }
    /// Check the actual immediate original container receipt, including its
    /// complete source/config/vector/path. None denotes only the root document.
    #[must_use]
    pub fn matches_original_parent(&self, parent: Option<&Self>) -> bool {
        self.parent.as_deref() == parent
    }
    /// Original braced grouping, without interpreting its contents as DSL.
    #[must_use]
    pub fn is_braced(&self) -> bool {
        self.local.is_braced()
    }
    /// Exact absolute original content span, with no cooked offset inverse.
    #[must_use]
    pub fn content_span(&self) -> Option<Span> {
        let span = self.local.native().content_span().ok()?;
        Some(Span::new(
            self.base.checked_add(span.start())?,
            self.base.checked_add(span.end())?,
        ))
    }
    /// Original statement's one-based LF line, independent of the selected word's line.
    #[must_use]
    pub fn statement_line(&self) -> Option<u32> {
        line_at(
            &self.root,
            self.base
                .checked_add(self.local.statement.words.first()?.span().start())?,
        )
    }
    /// Original selected word's one-based LF line.
    #[must_use]
    pub fn word_line(&self) -> Option<u32> {
        line_at(
            &self.root,
            self.base.checked_add(self.local.native().span().start())?,
        )
    }
    /// Plain ASCII content only. Expanded, braced, escaped, computed or whitespace
    /// forms cannot masquerade as a rejected property/flag spelling.
    #[must_use]
    pub fn plain_ascii(&self) -> Option<&str> {
        let word = self.local.native();
        if word.group().expand || word.group().kind == WordKind::Braced {
            return None;
        }
        let bytes = self.root.bytes().get(self.content_span()?.as_range())?;
        if bytes.is_empty()
            || !bytes.is_ascii()
            || bytes
                .iter()
                .any(|b| b.is_ascii_whitespace() || matches!(b, b'\\' | b'$' | b'[' | b']'))
        {
            return None;
        }
        std::str::from_utf8(bytes).ok()
    }
    /// Recheck complete source/configuration and every genuine selected parent.
    /// This does not discover or infer a DSL body role.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        if &self.root != image || self.root_config != config {
            return false;
        }
        if let Some(parent) = &self.parent {
            parent.matches_source(image, config)
                && parent.local.is_braced()
                && self.local.statement.config == parent.local.statement.config.nested()
                && parent.content_span().is_some_and(|span| {
                    span.start() == self.base
                        && image.bytes().get(span.as_range())
                            == Some(self.local.statement.image.bytes())
                })
        } else {
            self.base == 0
                && self.local.statement.image == self.root
                && self.local.statement.config == config
        }
    }
    /// Reparse only this exact selected original container path under another
    /// independently supplied full config. No recursive brace search or line/name
    /// match is used, and every selected word must retain the same source extent.
    #[must_use]
    pub fn reborrow(&self, image: &SourceImage, config: LexerConfig) -> Option<Self> {
        if image != &self.root {
            return None;
        }
        let source = if let Some(parent) = &self.parent {
            let parent = parent.reborrow(image, config)?;
            let text = parent.captured_text();
            let rows =
                original_pack_source_statements(text, parent.local.statement.config.nested());
            return rows
                .get(self.statement_ordinal())?
                .local_word(self.ordinal())?
                .in_body(&parent)
                .filter(|word| {
                    word.content_span() == self.content_span()
                        && word.captured_text() == self.captured_text()
                });
        } else {
            image.try_text().ok()?
        };
        original_pack_source_statements(source, config)
            .get(self.statement_ordinal())?
            .local_word(self.ordinal())
            .map(|word| word.in_document())
            .filter(|word| {
                word.content_span() == self.content_span()
                    && word.captured_text() == self.captured_text()
            })
    }
    /// One caller-selected original braced child's row/word. Selecting a body
    /// remains the caller's semantic job; this grants only checked geometry.
    #[must_use]
    pub fn body_word(&self, statement: usize, word: usize) -> Option<Self> {
        original_pack_source_statements(self.captured_text(), self.local.statement.config.nested())
            .get(statement)?
            .local_word(word)?
            .in_body(self)
    }
}

/// One actual recovered loader CST row with separately checked original words.
#[derive(Debug, Clone)]
pub struct OriginalPackSourceStatement {
    row: SegmentedCommand,
    original: Option<Arc<LocalStatement>>,
}
impl OriginalPackSourceStatement {
    /// The exact row produced by the existing configured CST parser.
    #[must_use]
    pub const fn row(&self) -> &SegmentedCommand {
        &self.row
    }
    /// Checked complete-vector local syntax, unavailable for malformed rows.
    #[must_use]
    pub fn local_word(&self, ordinal: usize) -> Option<LocalPackSourceWord> {
        let original = self.original.as_ref()?;
        original.words.get(ordinal)?;
        Some(LocalPackSourceWord {
            statement: Arc::clone(original),
            ordinal,
        })
    }
}
fn line_at(image: &SourceImage, offset: u32) -> Option<u32> {
    let prefix = image.bytes().get(..usize::try_from(offset).ok()?)?;
    let line = if u32::try_from(image.len()).is_ok() {
        image.source_map().line_index().line_at(offset)
    } else {
        tcl_lexer::LineIndex::from_bytes(prefix).line_at(offset)
    };
    line.checked_add(1)
}
/// The existing loader CST parser, with original complete row/word geometry.
/// Recovered rows stay readable but cannot lend checked word provenance beyond
/// the first real parse cut; nothing recursively treats all braced data as DSL.
#[must_use]
pub fn original_pack_source_statements(
    source: &str,
    config: LexerConfig,
) -> Vec<OriginalPackSourceStatement> {
    let image = SourceImage::document(source);
    let cut = tcl_lexer::first_parse_cut_image_checked(&image, config).ok();
    let rows = super::pack_source_statements(source, config);
    let row_count = rows.len();
    rows.into_iter()
        .enumerate()
        .map(|(ordinal, row)| {
            let original = (|| {
                let cut = cut.as_ref()?;
                if row.is_partial || cut.as_ref().is_some_and(|cut| ordinal >= cut.command) {
                    return None;
                }
                let groups =
                    tcl_lexer::group_commands_bytes(&row.all_tokens, image.bytes(), config);
                let [command] = groups.as_slice() else {
                    return None;
                };
                if command.words.len() != row.texts.len() || row.texts.len() != row.argv.len() {
                    return None;
                }
                let words = command
                    .words
                    .iter()
                    .map(|word| {
                        NativeWord::from_group(image.clone(), config, &row.all_tokens, word).ok()
                    })
                    .collect::<Option<Vec<_>>>()?;
                Some(Arc::new(LocalStatement {
                    image: image.clone(),
                    config,
                    ordinal,
                    row_count,
                    words: Arc::from(words),
                    texts: Arc::from(row.texts.clone()),
                }))
            })();
            OriginalPackSourceStatement { row, original }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{LeadingBom, SourceSegmentationPurpose};

    #[test]
    fn original_pack_word_geometry_keeps_selected_rows_parents_and_full_configuration() {
        let source = "\u{feff}speclib demo 1 {command demo {arty 1; arty 2}}";
        let config = super::super::pack_source_lexer_config(SourceSegmentationPurpose::WholeInput);
        let root = original_pack_source_statements(source, config)[0]
            .local_word(3)
            .unwrap()
            .in_document();
        let body = root.body_word(0, 2).unwrap();
        let first = body.body_word(0, 0).unwrap();
        let second = body.body_word(1, 0).unwrap();
        assert_eq!(first.plain_ascii(), Some("arty"));
        assert_eq!(second.plain_ascii(), Some("arty"));
        assert_ne!(first.content_span(), second.content_span());
        assert_eq!(first.original_words().len(), 2);
        assert!(first.matches_source(&SourceImage::document(source), config));
        assert!(!first.matches_source(
            &SourceImage::document(&source.replace("arty 2", "arty 3")),
            config
        ));
        let mut foreign = config;
        foreign.leading_bom = LeadingBom::Content;
        assert!(!first.matches_source(&SourceImage::document(source), foreign));
        let reborrowed = first
            .reborrow(&SourceImage::document(source), foreign)
            .unwrap();
        assert!(reborrowed.matches_source(&SourceImage::document(source), foreign));
        assert_eq!(reborrowed.content_span(), first.content_span());
        assert_eq!(
            second.reborrow(second.image(), config),
            Some(second.clone())
        );
    }

    #[test]
    fn original_pack_word_geometry_never_inverts_cooked_or_malformed_containers() {
        let config = super::super::pack_source_lexer_config(SourceSegmentationPurpose::WholeInput);
        let quoted = original_pack_source_statements("command demo \"arty 1\"", config)[0]
            .local_word(2)
            .unwrap()
            .in_document();
        assert!(!quoted.is_braced());
        assert!(quoted.body_word(0, 0).is_none());
        let malformed = original_pack_source_statements("arty 1; command demo {", config);
        assert!(malformed[0].local_word(0).is_some());
        assert!(
            malformed
                .iter()
                .skip(1)
                .all(|row| row.local_word(0).is_none())
        );
        let computed = original_pack_source_statements("$name 1", config)[0]
            .local_word(0)
            .unwrap()
            .in_document();
        assert!(computed.plain_ascii().is_none());
    }
}
