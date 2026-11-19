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
//! Literal declaration words for the package and architecture manifest readers.
//! Both formats group Tcl words and decode backslashes, keep variable and
//! command substitutions as data, and have no evaluated argument expansion.

use std::ops::Range;

use tcl_lexer::script::{WordKind, group_commands};
use tcl_lexer::{Lexer, LexerConfig, TokenType};

/// One manifest declaration with its original command extent.
#[derive(Debug, PartialEq, Eq)]
pub struct LiteralCommand {
    /// Literal argument values, including the directive name.
    pub words: Vec<String>,
    /// Original byte extent through the byte before the terminator.
    pub span: Range<usize>,
}

/// Read data declarations using the format's own lexical configuration.
///
/// # Errors
/// Reports lexer errors or an expansion requiring command evaluation.
pub fn literal_commands(source: &str, config: LexerConfig) -> Result<Vec<LiteralCommand>, String> {
    let tokens = Lexer::with_config(source, config)
        .tokenise_all()
        .map_err(|error| error.to_string())?;
    if tokens.iter().any(|token| token.kind == TokenType::Expand) {
        return Err("manifest expansion prefix `{*}` is not permitted in safe mode".into());
    }
    let mut result = Vec::new();
    for command in group_commands(&tokens, source, config) {
        let span = command
            .source_extent(&tokens, source.len())
            .ok_or("invalid original command span")?
            .as_range();
        let mut words = Vec::with_capacity(command.words.len());
        for word in command.words {
            let start = word.span.start() as usize;
            let end = tokens
                .get(word.tokens.end)
                .map_or(source.len(), |token| token.span.start() as usize);
            let raw = source.get(start..end).ok_or("invalid lexer token span")?;
            let value = match word.kind {
                WordKind::Braced => raw
                    .strip_prefix('{')
                    .and_then(|s| s.strip_suffix('}'))
                    .ok_or("missing close-brace")?
                    .to_owned(),
                WordKind::Quoted => {
                    let body = raw
                        .strip_prefix('"')
                        .and_then(|s| s.strip_suffix('"'))
                        .ok_or("missing close-quote")?;
                    crate::backslash::decode_in(body, config.escapes).into_owned()
                }
                WordKind::Bare => crate::backslash::decode_in(raw, config.escapes).into_owned(),
            };
            words.push(value);
        }
        result.push(LiteralCommand { words, span });
    }
    Ok(result)
}
