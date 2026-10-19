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

//! The runtime's parser, taken to the registry's model of the scripts 8.4's
//! compiler refuses (`tcl_registry::compile_84`).

use tcl_lexer::LexerConfig;
use tcl_registry::compile_84::{Command, Script, Segmenter, Word};

use crate::parse::{self, WordBody, WordPart};

/// The runtime's parser under the emulated release's grammar.
pub(crate) struct Parser {
    pub(crate) config: LexerConfig,
}

impl Segmenter for Parser {
    fn segment<'s>(&self, source: &'s str) -> Script<'s> {
        let mut commands = Vec::new();
        for command in parse::parse_script_with_config(source.as_bytes(), self.config) {
            if let Some(message) = parse::first_parse_error(&command.words, self.config) {
                return Script {
                    commands,
                    fatal: Some((command.start, message.to_owned())),
                };
            }
            let ends: Vec<usize> = command
                .words
                .iter()
                .skip(1)
                .map(|word| word.start)
                .chain(std::iter::once(command.end))
                .collect();
            let words = command
                .words
                .iter()
                .zip(ends)
                .map(|(word, end)| Word {
                    raw: word_text(source, word.start, end),
                    simple: match &word.body {
                        WordBody::Literal(text) => Some(String::from_utf8_lossy(text).into_owned()),
                        WordBody::Parts(_) => None,
                    },
                    scripts: match &word.body {
                        WordBody::Parts(parts) => parts
                            .iter()
                            .filter_map(|part| match part {
                                WordPart::Command(script) => std::str::from_utf8(script).ok(),
                                _ => None,
                            })
                            .collect(),
                        WordBody::Literal(_) => Vec::new(),
                    },
                })
                .collect();
            commands.push(Command {
                start: command.start,
                end: command.end,
                words,
            });
        }
        Script {
            commands,
            fatal: None,
        }
    }
}

/// The word as written from `start`, up to the whitespace and line
/// continuations before `end`.
fn word_text(source: &str, start: usize, end: usize) -> &str {
    let mut text = source.get(start..end).unwrap_or_default();
    loop {
        let trimmed = text.trim_end_matches([' ', '\t', '\n', '\r', '\u{b}', '\u{c}']);
        let trimmed = trimmed
            .strip_suffix('\\')
            .filter(|_| trimmed.len() < text.len())
            .unwrap_or(trimmed);
        if trimmed.len() == text.len() {
            return text;
        }
        text = trimmed;
    }
}
