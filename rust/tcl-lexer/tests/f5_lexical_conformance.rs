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

//! Hermetic conformance vectors for the two measured `f5-tcl` trunk axes —
//! the implicit word break (R-rules) and the brace-line continuation
//! (N-rules) — from `docs/design/bigip-irule-parser-measurements.md`
//! (§1, §2, §3, §4a; live-measured on BIG-IP 21.1.0.1 with same-host
//! stock controls).
//!
//! Every row in `tests/data/f5_lexical_vectors.txt` is lexed twice: under
//! the `f5-tcl` **trunk** grammar (which the `f5-irules` offshoot inherits
//! whole along the fork edge) and under plain **tcl8.4** (the stock
//! control), asserting each column — including that the F5 side emits
//! **zero** diagnostics on every accepted row (R6).

use tcl_dialect::model::{Family, Release, grammar};
use tcl_lexer::{Lexer, LexerConfig, SourceMap, Token, TokenType};

/// Decode the vector file's `\n` / `\t` / `\\` escapes.
fn unescape(field: &str) -> String {
    let mut out = String::with_capacity(field.len());
    let mut chars = field.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('\\') | None => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
        }
    }
    out
}

/// One expectation column: an accepted word rendering, or the first
/// diagnostic's message text.
enum Expectation {
    /// `OK=` — zero warnings, and the commands/words render to this.
    Ok(String),
    /// `ERR=` — at least one warning, the first containing this text.
    Err(String),
}

fn parse_expectation(field: &str) -> Expectation {
    if let Some(rest) = field.strip_prefix("OK=") {
        Expectation::Ok(unescape(rest))
    } else if let Some(rest) = field.strip_prefix("ERR=") {
        Expectation::Err(rest.to_owned())
    } else {
        panic!("expectation field {field:?} must start with OK= or ERR=");
    }
}

/// Render a token stream as `commands ¶ … · words`, with `∅` for an empty
/// word. Word text is the concatenation of the word's tokens'
/// `token_text` values — delimiters stripped, escapes not decoded, no
/// substitution — the lexical analogue of the measured word lists.
fn render(tokens: &[Token], map: &SourceMap<'_>) -> String {
    let mut commands: Vec<Vec<String>> = Vec::new();
    let mut words: Vec<String> = Vec::new();
    let mut current: Option<String> = None;
    let flush_word = |words: &mut Vec<String>, current: &mut Option<String>| {
        if let Some(word) = current.take() {
            words.push(word);
        }
    };
    for token in tokens {
        match token.kind {
            TokenType::Sep => flush_word(&mut words, &mut current),
            TokenType::Eol => {
                flush_word(&mut words, &mut current);
                if !words.is_empty() {
                    commands.push(std::mem::take(&mut words));
                }
            }
            TokenType::Comment => {}
            _ => {
                let text = map.token_text(*token);
                match &mut current {
                    Some(word) => word.push_str(text),
                    None => current = Some(text.to_owned()),
                }
            }
        }
    }
    flush_word(&mut words, &mut current);
    if !words.is_empty() {
        commands.push(words);
    }
    commands
        .iter()
        .map(|words| {
            words
                .iter()
                .map(|word| {
                    if word.is_empty() {
                        "∅".to_owned()
                    } else {
                        word.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join("·")
        })
        .collect::<Vec<_>>()
        .join("¶")
}

fn check(source: &str, config: LexerConfig, expectation: &Expectation, label: &str) {
    let lexer = Lexer::with_config(source, config);
    let (tokens, warnings) = lexer
        .tokenise_all_with_warnings()
        .unwrap_or_else(|err| panic!("{label}: non-strict lexing never errors, got {err}"));
    let map = SourceMap::new(source);
    match expectation {
        Expectation::Ok(expected) => {
            assert!(
                warnings.is_empty(),
                "{label}: expected zero diagnostics (R6), got {warnings:?}"
            );
            let rendered = render(&tokens, &map);
            assert_eq!(&rendered, expected, "{label}");
        }
        Expectation::Err(text) => {
            let first = warnings
                .first()
                .unwrap_or_else(|| panic!("{label}: expected a diagnostic containing {text:?}"));
            assert!(
                first.message.contains(text),
                "{label}: first diagnostic {:?} does not contain {text:?}",
                first.message
            );
        }
    }
}

/// The §1 word-formation table, the §2 continuation cases, and the §3 F3
/// matrix, each row asserted under the F5 trunk grammar and under plain
/// tcl8.4.
#[test]
fn f5_trunk_vectors_hold_under_both_grammars() {
    let f5 = LexerConfig::from_grammar(grammar(Family::F5Tcl, Release::F5_TCL_TMOS));
    assert!(f5.irules_brace_separator, "the trunk carries the R-rules");
    assert!(
        f5.brace_line_continuation.continues(),
        "the trunk carries the N-rules"
    );
    assert!(!f5.expand_syntax, "`{{*}}` is inert on the trunk");
    let stock = LexerConfig::from_grammar(grammar(Family::Tcl, Release::TCL_8_4));
    assert!(!stock.irules_brace_separator);
    assert!(!stock.brace_line_continuation.continues());
    assert!(!stock.expand_syntax);
    // The offshoot inherits the trunk grammar whole (fork-edge walk).
    assert_eq!(
        grammar(Family::F5Irules, Release::F5_IRULES_TMM),
        grammar(Family::F5Tcl, Release::F5_TCL_TMOS),
    );

    let data = include_str!("data/f5_lexical_vectors.txt");
    let mut rows = 0usize;
    for (line_number, line) in data.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('|').map(str::trim).collect();
        assert_eq!(
            fields.len(),
            3,
            "line {}: rows are `source | f5 | tcl8.4`",
            line_number + 1
        );
        let source = unescape(fields[0]);
        let f5_expected = parse_expectation(fields[1]);
        let stock_expected = parse_expectation(fields[2]);
        check(
            &source,
            f5,
            &f5_expected,
            &format!("line {} under f5-tcl: {source:?}", line_number + 1),
        );
        check(
            &source,
            stock,
            &stock_expected,
            &format!("line {} under tcl8.4: {source:?}", line_number + 1),
        );
        rows += 1;
    }
    // §1's 34-row table (23 diverging + 11 identical) plus the extra
    // measured cases; a shrunk file is a bug, not a trim.
    assert!(rows >= 55, "expected at least 55 vector rows, found {rows}");
}
