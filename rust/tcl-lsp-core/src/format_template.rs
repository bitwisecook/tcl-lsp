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

//! Map a selected format string's value offsets back to the written Tcl word.

pub(crate) struct FormatOperand {
    pub(crate) original: crate::original_invocation::OriginalOperandSource,
    logical: Option<
        std::sync::Arc<tcl_compiler::command_binding::OriginalSourceCommandTransitionAdvice>,
    >,
}

enum FormatTemplateMapping {
    Native {
        channel: tcl_lexer::SourceChannel,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        escapes: Option<tcl_dialect::EscapeSyntax>,
    },
    Logical(Box<tcl_lexer::NativeWord>),
}

pub(crate) struct FormatTemplate<'a> {
    pub(crate) content: std::borrow::Cow<'a, str>,
    raw: &'a [u8],
    span: tcl_lexer::Span,
    mapping: FormatTemplateMapping,
}

impl<'a> FormatTemplate<'a> {
    pub(crate) fn from_operand(source: &'a str, operand: &FormatOperand) -> Option<Self> {
        let original = &operand.original;
        let Some(input) = &original.input else {
            let logical = operand.logical.as_ref()?.logical_source_input()?;
            let word = original.word.as_ref()?;
            if word.image() != &tcl_lexer::SourceImage::document(source)
                || word.config() != logical.lexer_config()
            {
                return None;
            }
            let value = tcl_syntax::word_rules::original_static_word_ascii_presentation(word)?;
            return Some(Self {
                content: std::borrow::Cow::Owned(String::from_utf8(value).ok()?),
                raw: source.as_bytes().get(word.span().as_range())?,
                span: word.span(),
                mapping: FormatTemplateMapping::Logical(Box::new(word.clone())),
            });
        };
        let protocol = input.policy().string_protocol();
        let (span, channel, escapes) = if let Some(key) = input.original_word_key() {
            let word = key.original_word();
            let escapes =
                (word.group().kind != tcl_lexer::WordKind::Braced).then_some(word.config().escapes);
            (word.content_span().ok()?, word.image().channel(), escapes)
        } else {
            // Readonly list children can have exact literal extents. Produced
            // values without such an extent cannot supply a template coordinate.
            input.original_static_list_container()?;
            (original.span, tcl_lexer::SourceChannel::Document, None)
        };
        let raw = source.as_bytes().get(span.as_range())?;
        if escapes.is_none()
            && tcl_syntax::backslash::native_source_literal_bytes(raw, channel, protocol)
                .ok()?
                .as_ref()
                != input.bytes()
        {
            return None;
        }
        Some(Self {
            content: std::borrow::Cow::Owned(std::str::from_utf8(input.bytes()).ok()?.to_owned()),
            raw,
            span,
            mapping: FormatTemplateMapping::Native {
                channel,
                protocol,
                escapes,
            },
        })
    }

    pub(crate) fn original_offset(&self, offset: usize) -> Option<usize> {
        let (channel, protocol, escapes) = match &self.mapping {
            FormatTemplateMapping::Native {
                channel,
                protocol,
                escapes,
            } => (channel, protocol, escapes),
            FormatTemplateMapping::Logical(word) => {
                return usize::try_from(
                    tcl_syntax::word_rules::original_static_word_ascii_source_offset(word, offset)?,
                )
                .ok();
            }
        };
        let end = if let Some(escapes) = escapes {
            tcl_syntax::backslash::native_source_string_extent(
                self.raw,
                *channel,
                *escapes,
                *protocol,
                0..offset,
            )?
            .end
        } else {
            tcl_syntax::backslash::native_source_literal_extent(
                self.raw,
                *channel,
                *protocol,
                0..offset,
            )?
            .end
        };
        (self.span.start() as usize).checked_add(end)
    }
}

impl FormatOperand {
    pub(crate) fn capture(
        selected: &crate::original_invocation::OriginalRegistryWords,
        input: &tcl_compiler::analyser::ResolvedAnalysisInput,
        index: usize,
    ) -> Option<Self> {
        let operand = selected.operands.get(index)?.as_ref()?;
        let actual = selected.arguments.get(index)?.literal_bytes()?;
        let logical = if let Some(native) = operand.input.as_ref() {
            (native.bytes() == actual).then_some(())?;
            None
        } else {
            let crate::original_invocation::OriginalRegistrySource::SourceTransitions(advice) =
                &selected.source
            else {
                return None;
            };
            (advice.logical_source_input() == Some(input)).then_some(())?;
            let word = operand.word.as_ref()?;
            (tcl_syntax::word_rules::original_static_word_ascii_presentation(word)?.as_slice()
                == actual)
                .then_some(())?;
            Some(std::sync::Arc::clone(advice))
        };
        Some(Self {
            original: operand.clone(),
            logical,
        })
    }
}
