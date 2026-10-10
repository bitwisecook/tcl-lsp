// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! List-field source geometry from complete original Document words.

use std::sync::Arc;
use tcl_lexer::{ExecutablePart, ExecutableText, NativeWord, SourceChannel, Span, WordKind};

/// One lexical list field retains its complete original parent and grammar.
/// Its source extent names the whole original field, including escapes. No
/// Native list object, child word key, variable binding or value is issued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceListElement {
    parent: Arc<NativeWord>,
    value: String,
    source: Option<Span>,
    unchanged: bool,
}

impl OriginalSourceListElement {
    /// Complete original Document container; no child word is manufactured.
    #[must_use]
    pub fn original_word(&self) -> &NativeWord {
        &self.parent
    }
    /// Checked Unicode lexical presentation under the original escape grammar.
    /// This is separate from a Native engine's string units or naming policy.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
    /// Honest whole-field extent. Cooked intermediate values cannot supply
    /// invented interior coordinates.
    #[must_use]
    pub const fn source_span(&self) -> Option<Span> {
        self.source
    }
    /// Split this field under the same original list/escape grammar. Unchanged
    /// fields retain child extents; a cooked field can retain only a child
    /// whose value occupies its whole input, never an invented subrange.
    #[must_use]
    pub fn elements(&self) -> Option<Vec<Self>> {
        list_fields(&self.parent, self.value.as_bytes(), |range| {
            let span = self.source?;
            if self.unchanged {
                Some(Span::new(
                    span.start().checked_add(u32::try_from(range.start).ok()?)?,
                    span.start().checked_add(u32::try_from(range.end).ok()?)?,
                ))
            } else {
                (range == (0..self.value.len())).then_some(span)
            }
        })
    }
}

/// Source-only list fields of a static complete original Document word.
/// The word's full grammar controls values and geometry. Expanded, computed,
/// non-Document and non-Unicode lexical values decline. No source substring is
/// recaptured as a word or given Native list/name authority.
#[must_use]
pub fn original_static_word_list_elements(
    word: &NativeWord,
) -> Option<Vec<OriginalSourceListElement>> {
    // naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    let value = super::original_static_word_source_bytes(word)?;
    std::str::from_utf8(&value).ok()?;
    list_fields(&Arc::new(word.clone()), &value, |range| {
        Some(Span::new(
            source_offset(word, &value, range.start)?,
            source_offset(word, &value, range.end)?,
        ))
    })
}

fn list_fields(
    parent: &Arc<NativeWord>,
    bytes: &[u8],
    source: impl Fn(std::ops::Range<usize>) -> Option<Span>,
) -> Option<Vec<OriginalSourceListElement>> {
    let mut fields = Vec::new();
    let mut next = 0;
    while let Some(element) =
        crate::list::find_element_bytes_with_syntax(bytes, next, parent.config().list_parse).ok()?
    {
        let raw = bytes.get(element.value.clone())?;
        let value = if element.literal {
            std::borrow::Cow::Borrowed(raw)
        } else {
            lexical_field_value(raw, parent.config().escapes)?
        };
        fields.push(OriginalSourceListElement {
            parent: Arc::clone(parent),
            value: std::str::from_utf8(&value).ok()?.to_owned(),
            source: source(element.value),
            unchanged: raw == value.as_ref(),
        });
        if element.next <= next {
            return None;
        }
        next = element.next;
    }
    Some(fields)
}

// The shared scanner, rather than a String replacement conversion, decides
// whether every escaped numeric unit has a Unicode lexical view.
fn lexical_field_value(
    raw: &[u8],
    escapes: tcl_dialect::EscapeSyntax,
) -> Option<std::borrow::Cow<'_, [u8]>> {
    let mut offset = 0;
    while offset < raw.len() {
        if raw[offset] != b'\\' {
            offset += 1;
            continue;
        }
        let fragment = tcl_lexer::source_backslash_fragment_in(
            raw,
            offset,
            SourceChannel::NativeValue,
            escapes,
            |_| tcl_lexer::EscapedInputUnit {
                width: 1,
                value: tcl_lexer::EscapedInputValue::CopyOriginal,
            },
        )?;
        if let tcl_lexer::BackslashFragmentValue::Codepoint(value) = fragment.value {
            char::from_u32(value)?;
        }
        if fragment.end <= offset {
            return None;
        }
        offset = fragment.end;
    }
    Some(crate::backslash::decode_bytes_in(raw, escapes))
}

fn source_offset(word: &NativeWord, value: &[u8], offset: usize) -> Option<u32> {
    if offset > value.len() {
        return None;
    }
    if value.is_ascii() {
        return super::original_static_word_ascii_source_offset(word, offset);
    }
    if word.group().kind == WordKind::Braced {
        let content = word.content_span().ok()?;
        (word.image().bytes().get(content.as_range())? == value).then_some(())?;
        return content.start().checked_add(u32::try_from(offset).ok()?);
    }
    let arena = word.executable_parts();
    let mut produced = 0_usize;
    let mut selected = None;
    for component in arena.list(arena.root()) {
        let bytes: &[u8] = match &component.part {
            ExecutablePart::Text(ExecutableText::Original) => {
                let raw = arena.bytes(component.span)?;
                (crate::backslash::source_literal_bytes(raw, SourceChannel::Document).as_ref()
                    == raw)
                    .then_some(())?;
                raw
            }
            ExecutablePart::Text(ExecutableText::Decoded(bytes)) => bytes,
            _ => return None,
        };
        let end = produced.checked_add(bytes.len())?;
        if (produced..=end).contains(&offset) {
            let local = offset - produced;
            let at = match &component.part {
                ExecutablePart::Text(ExecutableText::Original) => component
                    .span
                    .start()
                    .checked_add(u32::try_from(local).ok()?)?,
                _ if local == 0 => component.span.start(),
                _ if local == bytes.len() => component.span.end(),
                _ => {
                    let raw = arena.bytes(component.span)?;
                    component.span.start().checked_add(
                        u32::try_from(super::decoded_ascii_source_offset(
                            raw,
                            bytes,
                            word.config(),
                            local,
                        )?)
                        .ok()?,
                    )?
                }
            };
            if selected.is_some_and(|previous| previous != at) {
                return None;
            }
            selected = Some(at);
        }
        produced = end;
    }
    (produced == value.len()).then_some(selected).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn word(source: &str, config: tcl_lexer::LexerConfig) -> NativeWord {
        tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap()
        .commands[0]
            .words[1]
            .clone()
    }

    #[test]
    fn original_lexical_list_fields_keep_unicode_escapes_and_literal_ancestry() {
        // naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        let source = "list {é {lo\\u006eg 5} escaped\\ name}";
        let original = word(source, tcl_lexer::LexerConfig::default());
        let fields = original_static_word_list_elements(&original).unwrap();
        assert_eq!(fields[0].value(), "é");
        assert_eq!(&source[fields[0].source_span().unwrap().as_range()], "é");
        let pair = fields[1].elements().unwrap();
        assert_eq!(pair[0].value(), "long");
        assert_eq!(
            &source[pair[0].source_span().unwrap().as_range()],
            r"lo\u006eg"
        );
        let cooked = fields[2].elements().unwrap();
        assert_eq!(cooked.len(), 2);
        assert!(cooked.iter().all(|field| field.source_span().is_none()));
        assert!(
            fields
                .iter()
                .all(|field| field.original_word() == &original)
        );
    }

    #[test]
    fn original_lexical_list_geometry_uses_full_grammar_and_declines_opaque_values() {
        // naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        let config = tcl_lexer::LexerConfig {
            expand_syntax: true,
            ..tcl_lexer::LexerConfig::default()
        };
        for source in [
            r"list {\uD800}",
            "list $dynamic",
            "list prefix[list value]",
            "list {*}{one two}",
        ] {
            assert!(
                original_static_word_list_elements(&word(source, config)).is_none(),
                "{source}"
            );
        }
        let malformed = "list {one \"two}";
        assert!(original_static_word_list_elements(&word(malformed, config)).is_none());
        let lenient = tcl_lexer::LexerConfig {
            list_parse: tcl_dialect::ListParse::Lenient,
            ..config
        };
        assert_eq!(
            original_static_word_list_elements(&word(malformed, lenient))
                .unwrap()
                .len(),
            2
        );
        let original = word("list {one two}", config);
        let native = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::native(b"list {one two}".as_slice()),
            Span::new(0, 14),
            config,
        )
        .unwrap()
        .commands[0]
            .words[1]
            .clone();
        assert!(original_static_word_list_elements(&native).is_none());
        assert!(original_static_word_list_elements(&original).is_some());
    }
}
