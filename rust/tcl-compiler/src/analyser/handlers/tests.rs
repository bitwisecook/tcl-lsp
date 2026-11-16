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

use super::*;
use tcl_lexer::Span;

mod classes;
mod code_loading;
mod command_aliases;
mod control_flow;
mod definition_warnings;
mod ensembles;
mod interpreters;
mod name_resolution;
mod namespaces;
mod procedures;
mod safe_interpreters;
mod scope_aliases;
mod value_provenance;
mod written_variables;

fn esc_tok(span: Span) -> Token {
    Token::new(TokenType::Esc, span)
}

fn str_tok(span: Span) -> Token {
    Token {
        kind: TokenType::Str,
        span,
        content_offset: 1,
        in_quote: false,
    }
}

fn span(start: u32, end: u32) -> Span {
    Span::new(start, end)
}

// Explicit standalone word-shape controls use the shared lexer owner.
fn elements(text: &str) -> Option<Vec<String>> {
    let (head, args) = crate::value_shapes::parse_command_substitution_with_config(
        text,
        tcl_lexer::LexerConfig::default(),
    )?;
    Some(std::iter::once(head).chain(args).collect())
}

// `set`'s binding through the generic dispatch tail — the role binding
// and `bind_value_word_assignment`; `set` has no analyser hook.

/// Dispatch one command whose words carry the given tokens and
/// single-token flags, through `process_command` exactly as a walked
/// command reaches it.
fn dispatch_tokens(a: &mut Analyser, words: &[(&str, Token, bool)]) {
    let owned: Vec<String> = words.iter().map(|(w, _, _)| (*w).to_string()).collect();
    let toks: Vec<Token> = words.iter().map(|&(_, tok, _)| tok).collect();
    let single: Vec<bool> = words.iter().map(|&(_, _, single)| single).collect();
    let expanded = vec![false; owned.len()];
    a.process_command(&owned, &toks, &single, &expanded, &[]);
}

/// Analyse `source` as Tcl 8.6 and keep the walk's state: the handlers
/// read each word from the analysed source, which a bare token list lacks.
fn analysed(source: &str) -> Analyser {
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl8.6");
    analyser
}

// Scope aliases through the generic consumer — the dispatch tail's
// `apply_state_transitions`, reached by `process_command` exactly as a
// walked command reaches it.

/// Dispatch one command whose words are bare (`Esc`) tokens four bytes
/// apart — a word carrying `$` or `[` reads as computed, as it would in
/// source.
fn dispatch_words(a: &mut Analyser, words: &[&str]) {
    let owned: Vec<String> = words.iter().map(|s| (*s).to_string()).collect();
    let toks: Vec<Token> = (0..owned.len())
        .map(|i| {
            let at = u32::try_from(i).unwrap() * 4;
            esc_tok(span(at, at + 1))
        })
        .collect();
    let single = vec![true; owned.len()];
    let expanded = vec![false; owned.len()];
    a.process_command(&owned, &toks, &single, &expanded, &[]);
}

// handle_namespace_eval_command
