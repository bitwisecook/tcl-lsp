// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Shared boundary parser for Tcl-shaped event-handler commands.

use std::collections::HashSet;

use tcl_lexer::{Lexer, LexerConfig, SourceMap, Span, Token, TokenType, word_span_at};

/// Whether event-handler discovery is limited to the supplied script or also
/// descends into its braced words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventHandlerTraversal {
    /// Inspect only commands at the supplied script level.
    TopLevel,
    /// Inspect commands at every braced-word script level.
    Recursive,
}

/// One syntactically complete `when EVENT ?priority N? { ... }` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventHandler {
    /// Upper-case event identity.
    pub event: String,
    /// Whole handler command span, including the body's closing brace.
    pub span: Span,
    /// Event word span.
    pub event_span: Span,
    /// Body interior span, excluding its braces.
    pub body_span: Span,
    /// Explicit handler priority, when it is a valid integer.
    pub priority: Option<i64>,
}

/// Parse complete event handlers with one lexer/naming contract.
///
/// `config` supplies the dialect grammar. iRules consumers pass its iRules
/// profile so TMM's `}{` word separator and Tcl quoting rules stay owned by
/// the lexer rather than being reimplemented by each event consumer.
#[must_use]
pub fn event_handlers(
    source: &str,
    config: LexerConfig,
    traversal: EventHandlerTraversal,
) -> Vec<EventHandler> {
    let mut handlers = Vec::new();
    let mut pending = vec![(source, 0_u32, 0_u32)];
    let mut visited = HashSet::new();

    while let Some((text, base, depth)) = pending.pop() {
        if depth > 256 || !visited.insert((base, text.len())) {
            continue;
        }
        let Ok(tokens) = Lexer::with_config(text, config.at_depth(depth)).tokenise_all() else {
            continue;
        };
        let sm = SourceMap::new(text);
        for command in command_words(&tokens) {
            if let Some(handler) = parse_handler(source, text, base, &sm, &command) {
                handlers.push(handler);
            }
            if traversal == EventHandlerTraversal::Recursive {
                for word in command.iter().rev() {
                    let [token] = word.as_slice() else { continue };
                    if token.kind != TokenType::Str {
                        continue;
                    }
                    let whole = word_span_at(text, token.span);
                    let start = whole.start().saturating_add(1);
                    let end = whole.end().saturating_sub(1);
                    if !closed_brace(text, whole) || start > end {
                        continue;
                    }
                    let Some(inner) = text.get(start as usize..end as usize) else {
                        continue;
                    };
                    pending.push((inner, base + start, depth + 1));
                }
            }
        }
    }
    handlers.sort_by_key(|handler| handler.span.start());
    handlers
}

fn command_words(tokens: &[Token]) -> Vec<Vec<Vec<Token>>> {
    let mut commands = Vec::new();
    let mut command = Vec::new();
    let mut word = Vec::new();
    for &token in tokens {
        match token.kind {
            TokenType::Sep => finish_word(&mut command, &mut word),
            TokenType::Eol | TokenType::Eof => {
                finish_word(&mut command, &mut word);
                if !command.is_empty() {
                    commands.push(std::mem::take(&mut command));
                }
            }
            TokenType::Comment => {}
            _ => word.push(token),
        }
    }
    finish_word(&mut command, &mut word);
    if !command.is_empty() {
        commands.push(command);
    }
    commands
}

fn finish_word(command: &mut Vec<Vec<Token>>, word: &mut Vec<Token>) {
    if !word.is_empty() {
        command.push(std::mem::take(word));
    }
}

fn parse_handler(
    full: &str,
    text: &str,
    base: u32,
    sm: &SourceMap<'_>,
    words: &[Vec<Token>],
) -> Option<EventHandler> {
    let [head] = words.first()?.as_slice() else {
        return None;
    };
    let canonical = crate::naming::canonical_written_command(sm.token_text(*head));
    if canonical.trim_start_matches("::") != "when" {
        return None;
    }
    let [event_token] = words.get(1)?.as_slice() else {
        return None;
    };
    let event = sm.token_text(*event_token).to_ascii_uppercase();
    if event.is_empty()
        || !event
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
    {
        return None;
    }
    let body = words.get(2..)?.iter().rev().find_map(|word| {
        let [token] = word.as_slice() else {
            return None;
        };
        (token.kind == TokenType::Str).then_some(token)
    })?;
    let body_whole = word_span_at(text, body.span);
    if !closed_brace(text, body_whole) {
        return None;
    }
    let first = *words.first()?.first()?;
    let command_start = word_span_at(text, first.span).start();
    let command_end = body_whole.end();
    let body_start = body_whole.start().saturating_add(1);
    let body_end = body_whole.end().saturating_sub(1);
    if body_start > body_end {
        return None;
    }
    let event_whole = word_span_at(text, event_token.span);
    let priority = words
        .get(2)
        .and_then(|word| word.as_slice().first().filter(|_| word.len() == 1))
        .filter(|token| sm.token_text(**token) == "priority")
        .and_then(|_| words.get(3))
        .and_then(|word| word.as_slice().first().filter(|_| word.len() == 1))
        .and_then(|token| sm.token_text(*token).parse::<i64>().ok());

    let span = shift(Span::new(command_start, command_end), base);
    let event_span = shift(event_whole, base);
    let body_span = shift(Span::new(body_start, body_end), base);
    // Keep absolute spans honest even for an overflowed/fabricated base.
    (span.end() as usize <= full.len()).then_some(EventHandler {
        event,
        span,
        event_span,
        body_span,
        priority,
    })
}

fn closed_brace(source: &str, whole: Span) -> bool {
    usize::try_from(whole.end())
        .ok()
        .and_then(|end| end.checked_sub(1))
        .and_then(|close| source.as_bytes().get(close))
        == Some(&b'}')
}

fn shift(span: Span, base: u32) -> Span {
    Span::new(base + span.start(), base + span.end())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn irules() -> LexerConfig {
        LexerConfig::for_file_dialect("f5-irules")
    }

    #[test]
    fn top_level_and_recursive_modes_share_root_and_event_semantics() {
        let source = "::when http_request { if {1} { :::when client_data {} } }";
        let top = event_handlers(source, irules(), EventHandlerTraversal::TopLevel);
        assert_eq!(
            top.iter().map(|h| h.event.as_str()).collect::<Vec<_>>(),
            ["HTTP_REQUEST"]
        );
        let all = event_handlers(source, irules(), EventHandlerTraversal::Recursive);
        assert_eq!(
            all.iter().map(|h| h.event.as_str()).collect::<Vec<_>>(),
            ["HTTP_REQUEST", "CLIENT_DATA"]
        );
        assert_eq!(&source[all[1].span.as_range()], ":::when client_data {}");
    }

    #[test]
    fn comments_and_string_data_are_not_handlers() {
        let source = "# when BAD {}\nset x {when DATA {}}\nwhen {NO_BODY}\nwhen good {}";
        let top = event_handlers(source, irules(), EventHandlerTraversal::TopLevel);
        assert_eq!(
            top.iter().map(|h| h.event.as_str()).collect::<Vec<_>>(),
            ["GOOD"]
        );
    }

    #[test]
    fn parses_priority_and_body_spans() {
        let source = "when HTTP_REQUEST priority 100 { pool p }";
        let handlers = event_handlers(source, irules(), EventHandlerTraversal::TopLevel);
        assert_eq!(handlers[0].priority, Some(100));
        assert_eq!(&source[handlers[0].body_span.as_range()], " pool p ");
    }
}
