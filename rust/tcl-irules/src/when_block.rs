// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! iRules `when EVENT { … }` boundary discovery.

use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::{LexerConfig, Span, TokenType, word_span_at};

/// One syntactically complete `when EVENT { … }` handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhenBlock {
    /// Event name following `when`.
    pub event: String,
    /// Whole command span, including the body's closing brace.
    pub span: Span,
    /// Body interior, excluding its outer braces.
    pub body_span: Span,
}

/// Return complete iRules event handlers in source order.
///
/// This is deliberately based on the configured Tcl segmenter, rather than a
/// `when` regex plus a brace counter. The lexer rejects commented-out headers,
/// keeps quoted and braced data intact, and uses the iRules brace-separator
/// grammar. Consumers that need handler bodies must use this entry point.
#[must_use]
pub fn when_blocks(source: &str) -> Vec<WhenBlock> {
    segment_commands_with_offset_and_config(source, 0, LexerConfig::for_file_dialect("f5-irules"))
        .into_iter()
        .filter_map(|command| {
            (command.name() == "when")
                .then_some(command)
                .and_then(|command| {
                    let event = command.args().first()?;
                    event
                        .chars()
                        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
                        .then_some(())?;
                    let body = command
                        .argv
                        .iter()
                        .rev()
                        .find(|token| token.kind == TokenType::Str)?;
                    let whole = word_span_at(source, body.span);
                    let start = whole.start().saturating_add(1);
                    let end = whole.end().saturating_sub(1);
                    let closed = usize::try_from(whole.end())
                        .ok()
                        .and_then(|end| end.checked_sub(1))
                        .and_then(|close| source.as_bytes().get(close))
                        == Some(&b'}');
                    (closed && start <= end).then(|| WhenBlock {
                        event: event.clone(),
                        span: command.span,
                        body_span: Span::new(start, end),
                    })
                })
        })
        .collect()
}

/// Whether a discovered handler body contains no executable command.  This
/// intentionally delegates comment classification to the iRules lexer.
#[must_use]
pub fn when_block_is_empty(source: &str, block: &WhenBlock) -> bool {
    source.get(block.body_span.as_range()).is_some_and(|body| {
        segment_commands_with_offset_and_config(body, 0, LexerConfig::for_file_dialect("f5-irules"))
            .is_empty()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_commented_when_and_keeps_quoted_close_brace_in_tcl_body() {
        let source =
            "# when HTTP_REQUEST { ignored }\nwhen RULE_INIT { log local0. \"legacy } path\"\n}\n";
        let blocks = when_blocks(source);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].event, "RULE_INIT");
        assert_eq!(
            &source[blocks[0].body_span.as_range()],
            " log local0. \"legacy "
        );
    }
}
