// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! iRules `when EVENT { … }` boundary discovery.

use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_core_types::RecursionLimit;
use tcl_lexer::{LexerConfig, Span, TokenType, word_span_at};

const MAX_WHEN_SCAN_DEPTH: RecursionLimit = RecursionLimit(256);

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
            (tcl_syntax::naming::canonical_written_command(command.name()).trim_start_matches("::")
                == "when")
                .then_some(command)
                .and_then(|command| {
                    let event = command.args().first()?.to_ascii_uppercase();
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
                        event,
                        span: command.span,
                        body_span: Span::new(start, end),
                    })
                })
        })
        .collect()
}

/// Return handlers recursively nested inside handler bodies, with spans kept
/// absolute to `source`. This is for editor context; [`when_blocks`] remains
/// the top-level execution surface.
#[must_use]
pub fn when_blocks_recursive(source: &str) -> Vec<WhenBlock> {
    let mut out = Vec::new();
    let mut pending = vec![(source, 0_u32, 0_u32)];

    while let Some((text, base, depth)) = pending.pop() {
        if MAX_WHEN_SCAN_DEPTH.exceeded(depth) {
            continue;
        }
        let commands = segment_commands_with_offset_and_config(
            text,
            base,
            LexerConfig::for_file_dialect("f5-irules").at_depth(depth),
        );
        for command in commands {
            if tcl_syntax::naming::canonical_written_command(command.name())
                .trim_start_matches("::")
                == "when"
                && let Some(event) = command
                    .args()
                    .first()
                    .map(|event| event.to_ascii_uppercase())
                && event
                    .chars()
                    .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
                && let Some(body) = command
                    .argv
                    .iter()
                    .rev()
                    .find(|token| token.kind == TokenType::Str)
            {
                let whole = word_span_at(source, body.span);
                let start = whole.start().saturating_add(1);
                let end = whole.end().saturating_sub(1);
                let closed = usize::try_from(whole.end())
                    .ok()
                    .and_then(|end| end.checked_sub(1))
                    .and_then(|close| source.as_bytes().get(close))
                    == Some(&b'}');
                if closed && start <= end {
                    out.push(WhenBlock {
                        event,
                        span: command.span,
                        body_span: Span::new(start, end),
                    });
                }
            }

            // A nested handler may occur in any script-bearing braced word,
            // for example an `if` body inside an outer handler.  Walking all
            // braced words matches the conservative editor-context contract:
            // lexical data can add a completion context, but cannot affect
            // execution or diagnostics.
            for token in command
                .argv
                .iter()
                .rev()
                .filter(|token| token.kind == TokenType::Str)
            {
                let whole = word_span_at(source, token.span);
                let start = whole.start().saturating_add(1);
                let end = whole.end().saturating_sub(1);
                let Some(inner) = source.get(start as usize..end as usize) else {
                    continue;
                };
                pending.push((inner, start, depth + 1));
            }
        }
    }
    out.sort_by_key(|block| block.span.start());
    out
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

    #[test]
    fn recognises_canonical_global_when_spellings() {
        let source =
            "::when HTTP_REQUEST { pool /Common/a }\n:::when RULE_INIT { pool /Common/b }\n";
        let blocks = when_blocks(source);
        assert_eq!(
            blocks
                .iter()
                .map(|block| block.event.as_str())
                .collect::<Vec<_>>(),
            ["HTTP_REQUEST", "RULE_INIT"]
        );
    }

    #[test]
    fn recursively_finds_nested_rooted_handlers_with_absolute_spans() {
        let source =
            "when http_request {\n  if {1} {\n    :::when client_data { pool p }\n  }\n}\n";
        let blocks = when_blocks_recursive(source);
        assert_eq!(
            blocks
                .iter()
                .map(|block| block.event.as_str())
                .collect::<Vec<_>>(),
            ["HTTP_REQUEST", "CLIENT_DATA"]
        );
        assert_eq!(
            &source[blocks[1].span.as_range()],
            ":::when client_data { pool p }"
        );
        assert_eq!(&source[blocks[1].body_span.as_range()], " pool p ");
    }
}
