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

//! Parsing a script into commands, their arguments and the comments between them.

use super::{ArgKind, CommandArg, CommentLine, ParsedCommand};
use tcl_lexer::{SourceMap, Token, TokenType};

// Command parsing

/// Count blank lines represented by an EOL token's text (each
/// newline beyond the first is a blank line).
fn count_newlines(text: &str) -> usize {
    text.matches('\n').count().saturating_sub(1)
}

/// The leading whitespace on the source line containing `offset`, but only when
/// everything before `offset` on that line is whitespace — i.e. the comment
/// stands alone at line start. Returns `None` for a mid-line (inline `;#`)
/// comment, which has no standalone column to preserve.
fn comment_orig_indent(source: &str, offset: usize) -> Option<String> {
    let line_start = source.get(..offset)?.rfind('\n').map_or(0, |p| p + 1);
    let prefix = source.get(line_start..offset)?;
    if prefix.bytes().all(|b| b == b' ' || b == b'\t') {
        Some(prefix.to_owned())
    } else {
        None
    }
}

/// Parse Tcl source into structured commands plus any trailing
/// comments.
pub(super) fn parse_commands(
    source: &str,
    sm: &SourceMap,
    tokens: &[Token],
    config: tcl_lexer::LexerConfig,
) -> Option<(Vec<ParsedCommand>, Vec<CommentLine>)> {
    if sm.base_offset() != 0 || sm.source_bytes() != source.as_bytes() {
        return None;
    }
    let image = tcl_lexer::SourceImage::document(source);
    let mut words = std::collections::HashMap::new();
    for command in tcl_lexer::group_commands(tokens, source, config) {
        for group in command.words {
            let word =
                tcl_lexer::NativeWord::from_group(image.clone(), config, tokens, &group).ok()?;
            if words.insert(word.span().start(), word).is_some() {
                return None;
            }
        }
    }
    let mut commands: Vec<ParsedCommand> = Vec::new();
    let mut pending_comments: Vec<CommentLine> = Vec::new();
    let mut pending_blank_lines = 0usize;

    let mut argv: Vec<CommandArg> = Vec::new();
    let mut prev_type = TokenType::Eol;

    for &tok in tokens {
        match tok.kind {
            TokenType::Eof => break,
            TokenType::Comment => {
                pending_comments.push(CommentLine {
                    orig_indent: comment_orig_indent(source, tok.span.start() as usize),
                    text: sm.text(tok.span).to_owned(),
                });
                continue;
            }
            TokenType::Sep => {
                prev_type = TokenType::Sep;
                continue;
            }
            TokenType::Eol => {
                let newlines = count_newlines(sm.text(tok.span));
                if argv.is_empty() {
                    pending_blank_lines += newlines;
                } else {
                    // A pure `;` separator (no newline in the Eol run) means
                    // this command shared a line with the next.
                    flush_parsed_command(
                        &mut argv,
                        &mut pending_comments,
                        pending_blank_lines,
                        newlines == 0,
                        &mut commands,
                    );
                    pending_blank_lines = newlines;
                }
                prev_type = TokenType::Eol;
                continue;
            }
            _ => {}
        }

        // segmentation-drift-ok: the formatter's own
        // `parse_commands` is a word grouper that must also keep the trivia
        // (comments, blank-line runs) `WordSpan` deliberately does not carry.
        let is_start_of_new_arg = matches!(prev_type, TokenType::Sep | TokenType::Eol);
        let detected_quoted =
            is_start_of_new_arg && source.as_bytes().get(tok.span.start() as usize) == Some(&b'"');
        let text = sm.token_text(tok).to_owned();

        if is_start_of_new_arg || argv.is_empty() {
            argv.push(CommandArg {
                kind: ArgKind::Word,
                tokens: vec![tok],
                text,
                is_braced: tok.kind == TokenType::Str,
                is_quoted: detected_quoted,
                formatted_body: None,
                member_body_presentation: None,
                lexical_word: words.get(&tok.span.start())?.clone(),
                original_word: None,
                original_input: None,
            });
        } else {
            let last = argv.last_mut().expect("argv non-empty");
            last.tokens.push(tok);
            last.text.push_str(&text);
        }
        prev_type = tok.kind;
    }

    flush_parsed_command(
        &mut argv,
        &mut pending_comments,
        pending_blank_lines,
        false,
        &mut commands,
    );
    for arg in commands.iter().flat_map(|command| &command.args) {
        if !arg
            .tokens
            .iter()
            .filter(|token| token.kind != TokenType::Expand)
            .eq(arg.lexical_word.tokens().iter())
            || arg
                .tokens
                .iter()
                .any(|token| token.kind == TokenType::Expand)
                != arg.lexical_word.group().expand
        {
            return None;
        }
    }
    Some((commands, pending_comments))
}

fn flush_parsed_command(
    argv: &mut Vec<CommandArg>,
    pending_comments: &mut Vec<CommentLine>,
    pending_blank_lines: usize,
    terminated_by_semicolon: bool,
    commands: &mut Vec<ParsedCommand>,
) {
    if argv.is_empty() {
        return;
    }
    let taken_args = std::mem::take(argv);
    let head_start = taken_args
        .first()
        .and_then(|arg| arg.tokens.first())
        .map_or(0, |token| token.span.start());
    commands.push(ParsedCommand {
        resolved_name: String::new(),
        source_roles: None,
        logical_roles: None,
        logical_context: None,
        source_traits: None,
        source_words: None,
        source_case: None,
        expression_bracing: None,
        head_start,
        args: taken_args,
        preceding_comments: std::mem::take(pending_comments),
        preceding_blank_lines: pending_blank_lines,
        terminated_by_semicolon,
    });
}
