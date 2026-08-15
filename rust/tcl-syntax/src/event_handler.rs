// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Shared boundary parser for Tcl-shaped event-handler commands.

use tcl_lexer::{Lexer, LexerConfig, SourceMap, Span, Token, TokenType, word_span_at};

/// One command split into lexer-owned words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptCommand {
    /// Words in source order; each word retains all of its lexer tokens.
    pub words: Vec<Vec<Token>>,
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
pub fn event_handlers(source: &str, config: LexerConfig) -> Vec<EventHandler> {
    let mut handlers = Vec::new();
    let sm = SourceMap::new(source);
    for command in script_commands(source, config) {
        if let Some(handler) = parse_handler(source, source, 0, &sm, &command.words) {
            handlers.push(handler);
        }
    }
    handlers
}

/// Split one supplied script region into commands and words.
///
/// This function deliberately does not infer that arbitrary braced values are
/// scripts. Higher layers may recurse only after their semantic owner has
/// identified an executable script surface.
#[must_use]
pub fn script_commands(source: &str, config: LexerConfig) -> Vec<ScriptCommand> {
    let Ok(tokens) = Lexer::with_config(source, config).tokenise_all() else {
        return Vec::new();
    };
    command_words(&tokens)
        .into_iter()
        .map(|words| ScriptCommand { words })
        .collect()
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
    fn top_level_parser_normalises_rooted_event_semantics() {
        let source = "::when http_request { if {1} { :::when client_data {} } }";
        let top = event_handlers(source, irules());
        assert_eq!(
            top.iter().map(|h| h.event.as_str()).collect::<Vec<_>>(),
            ["HTTP_REQUEST"]
        );
    }

    #[test]
    fn comments_and_string_data_are_not_handlers() {
        let source = "# when BAD {}\nset x {when DATA {}}\nwhen {NO_BODY}\nwhen good {}";
        let top = event_handlers(source, irules());
        assert_eq!(
            top.iter().map(|h| h.event.as_str()).collect::<Vec<_>>(),
            ["GOOD"]
        );
    }

    #[test]
    fn supplied_script_region_does_not_promote_braced_data() {
        let source = "set payload {when CLIENT_DATA {}}\nwhen HTTP_REQUEST {}";
        let handlers = event_handlers(source, irules());
        assert_eq!(
            handlers
                .iter()
                .map(|h| h.event.as_str())
                .collect::<Vec<_>>(),
            ["HTTP_REQUEST"]
        );
    }

    #[test]
    fn parses_priority_and_body_spans() {
        let source = "when HTTP_REQUEST priority 100 { pool p }";
        let handlers = event_handlers(source, irules());
        assert_eq!(handlers[0].priority, Some(100));
        assert_eq!(&source[handlers[0].body_span.as_range()], " pool p ");
    }
}
