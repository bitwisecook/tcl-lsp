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
    /// Effective priority after applying any preceding file-level `priority`
    /// declaration. Syntax-only parsing defaults this to 500; the registry
    /// wrapper applies resolved declaration state.
    pub effective_priority: u16,
    /// Decoded single-token argument words after the command head.
    pub arguments: Vec<String>,
    /// Representative lexer token for each argument word.
    ///
    /// The registry declaration owner uses these source facts to distinguish
    /// the required braced body from bare or quoted arguments.
    pub argument_tokens: Vec<Token>,
    /// Whether each argument word consists of exactly one lexer token.
    pub argument_single_tokens: Vec<bool>,
}

/// Parse complete event handlers with one lexer/naming contract.
///
/// `config` supplies the dialect grammar. iRules consumers pass its iRules
/// profile so TMM's `}{` word separator and Tcl quoting rules stay owned by
/// the lexer rather than being reimplemented by each event consumer.
#[must_use]
pub fn event_handlers(source: &str, config: LexerConfig) -> Vec<EventHandler> {
    event_handlers_with_head_predicate(source, config, |head, _| {
        head.trim_start_matches("::") == "when"
    })
}

/// Parse complete event handlers whose command head satisfies `is_handler`.
///
/// This remains a syntax-layer parser: it owns word splitting and the common
/// `EVENT ?priority N? { body }` boundary grammar, but does not decide which
/// command binding a written head denotes.  A higher layer that has resolved
/// command identity (for example after `interp alias` or `rename`) supplies
/// that decision with the head's byte offset in this script region.
#[must_use]
pub fn event_handlers_with_head_predicate(
    source: &str,
    config: LexerConfig,
    is_handler: impl Fn(&str, u32) -> bool,
) -> Vec<EventHandler> {
    let mut handlers = Vec::new();
    let sm = SourceMap::new(source);
    for command in script_commands(source, config) {
        if let Some(handler) = parse_handler(source, source, 0, &sm, &command.words, &is_handler) {
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
    is_handler: &impl Fn(&str, u32) -> bool,
) -> Option<EventHandler> {
    let [head] = words.first()?.as_slice() else {
        return None;
    };
    let canonical = crate::naming::canonical_written_command(sm.token_text(*head));
    if !is_handler(&canonical, word_span_at(text, head.span).start()) {
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
    let one = |index: usize| {
        words
            .get(index)?
            .as_slice()
            .first()
            .filter(|_| words[index].len() == 1)
    };
    let (body_index, priority) = match words.len() {
        3 => (2, None),
        5 if one(2).is_some_and(|token| sm.token_text(*token) == "priority") => {
            let value = sm.token_text(*one(3)?).parse::<i64>().ok()?;
            if !(0..=1000).contains(&value) {
                return None;
            }
            (4, Some(value))
        }
        5 if one(2).is_some_and(|token| sm.token_text(*token) == "timing")
            && one(3).is_some_and(|token| {
                matches!(sm.token_text(*token), "on" | "off" | "enable" | "disable")
            }) =>
        {
            (4, None)
        }
        7 if one(2).is_some_and(|token| sm.token_text(*token) == "priority")
            && sm
                .token_text(*one(3)?)
                .parse::<i64>()
                .is_ok_and(|priority| (0..=1000).contains(&priority))
            && one(4).is_some_and(|token| sm.token_text(*token) == "timing")
            && one(5).is_some_and(|token| {
                matches!(sm.token_text(*token), "on" | "off" | "enable" | "disable")
            }) =>
        {
            (6, sm.token_text(*one(3)?).parse::<i64>().ok())
        }
        _ => return None,
    };
    let body = one(body_index)?.to_owned();
    if body.kind != TokenType::Str {
        return None;
    }
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
        effective_priority: priority
            .and_then(|value| u16::try_from(value).ok())
            .unwrap_or(500),
        arguments: words[1..]
            .iter()
            .map(|word| sm.token_text(word[0]).to_owned())
            .collect(),
        argument_tokens: words[1..]
            .iter()
            .map(|word| shift_token(word[0], base))
            .collect(),
        argument_single_tokens: words[1..].iter().map(|word| word.len() == 1).collect(),
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

fn shift_token(mut token: Token, base: u32) -> Token {
    token.span = shift(token.span, base);
    token
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

    #[test]
    fn handler_grammar_accepts_only_complete_braced_supported_forms() {
        for source in [
            "when HTTP_REQUEST {}",
            "when HTTP_REQUEST priority 100 {}",
            "when HTTP_REQUEST timing on {}",
            "when HTTP_REQUEST timing disable {}",
            "when HTTP_REQUEST priority 100 timing enable {}",
            "when HTTP_REQUEST priority 0 {}",
            "when HTTP_REQUEST priority 1000 {}",
        ] {
            assert_eq!(event_handlers(source, irules()).len(), 1, "{source}");
        }
        for source in [
            "when HTTP_REQUEST",
            "when HTTP_REQUEST set",
            "when HTTP_REQUEST {} trailing",
            "when HTTP_REQUEST priority {}",
            "when HTTP_REQUEST priority nope {}",
            "when HTTP_REQUEST priority -1 {}",
            "when HTTP_REQUEST priority 1001 {}",
            "when HTTP_REQUEST timing maybe {}",
            "when HTTP_REQUEST timing on priority 100 {}",
        ] {
            assert!(event_handlers(source, irules()).is_empty(), "{source}");
        }
    }
}
