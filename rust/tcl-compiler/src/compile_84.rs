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

//! The scripts Tcl 8.4's compiler refuses, as this compiler's segmenter
//! takes them apart (`tcl_registry::compile_84`).

use tcl_dialect::DialectProfile;
use tcl_lexer::{LexerConfig, TokenType};
use tcl_registry::CommandRegistry;
use tcl_registry::compile_84::{Command, Compile, Script, Segmenter, Word};
use tcl_runtime_api::{CompileFailure, CompileUnit};

use crate::segmenter::SegmentedCommand;

/// Why 8.4's compiler refuses `source` compiled as `unit` (only in a
/// procedure body do `foreach`, `lappend` and a `catch` with a variable
/// compile), or `None` when it compiles. A literal expression is parsed as
/// stock 8.4 parses it only where `profile` adds no operators of its own
/// ([`tcl_dialect::DialectProfile::f5_core_expr_grammar`]: F5's fork adds
/// its word operators to 8.4's parser).
#[must_use]
pub(crate) fn compile_failure(
    source: &str,
    registry: &CommandRegistry,
    profile: &DialectProfile,
    unit: CompileUnit,
) -> Option<CompileFailure> {
    let segmenter = Segments {
        config: LexerConfig::from_grammar(profile.grammar),
    };
    tcl_registry::compile_84::compile_failure(
        source,
        &Compile {
            registry,
            point: registry.own_surface_query(),
            segmenter: &segmenter,
            in_procedure: unit == CompileUnit::ProcedureBody,
            parses_expressions: profile.f5_core_expr_grammar().is_none(),
        },
    )
}

/// This compiler's segmenter under a profile's grammar.
struct Segments {
    config: LexerConfig,
}

impl Segmenter for Segments {
    fn segment<'s>(&self, source: &'s str) -> Script<'s> {
        // Without the lexer's ownership of the source there is no command
        // boundary to follow, so no 8.4 compile verdict: the runtime's own
        // admission decides the script.
        let Ok(parsed) = crate::lowering::command_at_time_script_with_config(source, self.config)
        else {
            return Script {
                commands: Vec::new(),
                fatal: None,
            };
        };
        Script {
            commands: parsed
                .commands
                .iter()
                .map(|command| Command {
                    start: command.span.start() as usize,
                    end: command_end(source, command),
                    words: words(source, command),
                })
                .collect(),
            fatal: parsed
                .fatal_tail
                .map(|(start, message, _)| (start, message)),
        }
    }
}

/// The words of `command`, read from `src`.
fn words<'s>(src: &'s str, command: &SegmentedCommand) -> Vec<Word<'s>> {
    command
        .word_fragments
        .iter()
        .zip(&command.texts)
        .map(|(fragments, text)| {
            let start = fragments
                .first()
                .map_or(0, |fragment| fragment.token.span.start() as usize);
            let mut end = fragments
                .last()
                .map_or(start, |fragment| fragment.token.span.end() as usize);
            // A brace word's token, and a quoted word's last, stops short of
            // its close.
            let close = match src.as_bytes().get(start) {
                Some(b'{') => Some(b'}'),
                Some(b'"') => Some(b'"'),
                _ => None,
            };
            if close.is_some() && src.as_bytes().get(end) == close.as_ref() && end > start + 1 {
                end += 1;
            }
            let raw = src.get(start..end).unwrap_or_default();
            let simple = match fragments.as_slice() {
                [fragment] => match fragment.token.kind {
                    TokenType::Str => Some(text.clone()),
                    TokenType::Esc if !raw.contains('\\') => Some(text.clone()),
                    _ => None,
                },
                [] => Some(String::new()),
                _ => None,
            };
            let scripts = fragments
                .iter()
                .filter(|fragment| fragment.token.kind == TokenType::Cmd)
                .filter_map(|fragment| {
                    // The token spans the `[` and stops short of the `]`.
                    let span = fragment.token.span;
                    let open = span.start() as usize + usize::from(fragment.token.content_offset);
                    src.get(open..(span.end() as usize).max(open))
                })
                .collect();
            Word {
                raw,
                simple,
                scripts,
            }
        })
        .collect()
}

/// Where C ends the text it quotes for `command`: at its terminator, any
/// whitespace before it kept, or at the end of the script.
fn command_end(src: &str, command: &SegmentedCommand) -> usize {
    let end = command.span.end() as usize;
    let rest = src.get(end..).unwrap_or_default();
    rest.find(['\n', ';', ']'])
        .map_or(src.len(), |offset| end + offset)
}
