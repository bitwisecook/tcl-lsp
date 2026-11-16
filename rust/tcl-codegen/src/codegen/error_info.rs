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

//! The command text and line `errorInfo` names: span text, substitution stamps and source lines.

use super::{CodegenCtx, cmd_subst};
use tcl_lexer::Span;

impl CodegenCtx<'_> {
    /// The surface text of the construct at `current_span`, for `errorInfo`.
    /// Empty when no span is set or no source was supplied.
    ///
    /// A command ending in a quoted (`"…"`) word has its `current_span` end at
    /// the word's inner end — [`segmenter::widen_word_end`] deliberately does not
    /// widen quoted words (other `cmd.range` consumers rely on the inner end), so
    /// the closing `"` sits one byte past `span.end()`. The `errorInfo` frame must
    /// quote the *whole* command (`"error "test error""`, eval-2.5), so include a
    /// trailing `"` here — the analogue of `widen_word_end`'s brace/bracket widen,
    /// scoped to error reporting.
    fn span_text(&self) -> tcl_lexer::SourceImage {
        match self.current_span {
            Some(sp) => {
                let (s, mut e) = (sp.start() as usize, sp.end() as usize);
                if self.source.bytes().get(e) == Some(&b'"') {
                    e += 1;
                }
                tcl_lexer::SourceImage::from_bytes(
                    self.source.get(s..e).unwrap_or_default(),
                    self.source.channel(),
                )
            }
            None => tcl_lexer::SourceImage::default(),
        }
    }

    /// C logs a command's source through any whitespace before its
    /// terminator (`error x }` names `error x `); `None` when there is none.
    pub(super) fn trailing_space_error_info(
        &self,
        text: &str,
    ) -> Option<tcl_bytecode::ErrorInfoCommand> {
        let span = self.current_span.filter(|_| !text.is_empty())?;
        let trailing = self.space_after(span);
        (!trailing.is_empty()).then(|| tcl_bytecode::ErrorInfoCommand {
            text: format!("{text}{trailing}"),
            line: self.span_line(),
            nested: false,
            enclosing: Vec::new(),
        })
    }

    /// The word separators between `span` and the terminator after it, by
    /// this dialect's grammar.
    fn space_after(&self, span: Span) -> &str {
        let separators = self.lexer_config().word_separators;
        let rest = self
            .source
            .get(self.command_end(span)..)
            .unwrap_or_default();
        let end = rest
            .iter()
            .position(|&byte| !separators.is_separator(byte))
            .unwrap_or(rest.len());
        // Word separators are ASCII, so the run is always Unicode.
        std::str::from_utf8(&rest[..end]).unwrap_or_default()
    }

    /// [`Self::source_text`] through the whitespace before the command's
    /// terminator, as C's `errorInfo` names a command. The original bytes and
    /// channel are retained, as [`Self::source_text`] retains them.
    pub(crate) fn error_info_source_text(&self, span: Span) -> tcl_lexer::SourceImage {
        let start = span.start() as usize;
        let end = self.command_end(span) + self.space_after(span).len();
        tcl_lexer::SourceImage::from_bytes(
            self.source.get(start..end).unwrap_or_default(),
            self.source.channel(),
        )
    }

    /// Where the command at `span` ends: a quoted last word's span ends inside
    /// its closing `"` (see [`Self::span_text`]).
    fn command_end(&self, span: Span) -> usize {
        let end = span.end() as usize;
        if self.source.bytes().get(end) == Some(&b'"') {
            end + 1
        } else {
            end
        }
    }

    /// Name the command inside the substitution `spelling` (`[cmd …]`) in
    /// the `errorInfo` of every instruction emitted for it from `start`, as
    /// C logs the inner command rather than the enclosing statement. A more
    /// deeply nested substitution's stamp is kept. The line is that of the
    /// inner command's first word when its `tokens` place it.
    pub(crate) fn stamp_substitution_error_info(
        &mut self,
        start: usize,
        spelling: &str,
        tokens: Option<&tcl_compiler::ir::CommandTokens>,
    ) {
        let Some(inner) = spelling
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        else {
            return;
        };
        let separators = self.lexer_config().word_separators;
        let text = inner.trim_start_matches(|c: char| {
            c == '\n' || u8::try_from(c).is_ok_and(|byte| separators.is_separator(byte))
        });
        if text.trim().is_empty() {
            return;
        }
        let statement = self.command_span_text();
        // A statement whose original bytes are not Unicode keeps its default
        // `errorInfo` rather than a stamp read from a lossy projection.
        let Ok(statement_text) = statement.try_text() else {
            return;
        };
        let before = statement_text
            .find(spelling)
            .map_or(0, |at| statement_text[..at].matches('\n').count())
            + inner[..inner.len() - text.len()].matches('\n').count();
        let line = tokens
            .and_then(|tokens| tokens.argv.first())
            .map(|word| self.source_line(*word))
            .filter(|&line| line != 0)
            .unwrap_or_else(|| self.span_line() + u32::try_from(before).unwrap_or(0));
        // Evaluated directly, each substitution is a script of its own that
        // the enclosing one logs again as the error leaves it.
        let direct = self.direct_statement_error_info();
        if let Some((text, _)) = &direct {
            self.direct_levels.push(text.clone());
        }
        // An inlined body inside the substitution names its own commands.
        for instruction in self
            .instructions
            .iter_mut()
            .skip(start)
            .filter(|instruction| instruction.source_cmd_text == statement)
        {
            match instruction.error_info_command.as_mut() {
                Some(command) if command.nested => {
                    if direct.is_some() {
                        let at = command.enclosing.len().saturating_sub(1);
                        command.enclosing.insert(at, (text.to_owned(), line));
                    }
                }
                _ => {
                    instruction.error_info_command = Some(tcl_bytecode::ErrorInfoCommand {
                        text: text.to_owned(),
                        line,
                        nested: true,
                        enclosing: direct.iter().cloned().collect(),
                    });
                }
            }
        }
        // Commands a native recipe emitted inside it keep their own text.
        self.stamp_native_substitution_except(start, inner, Some(&statement));
    }

    /// Whether the statement being emitted is one the release evaluates
    /// without compiling it.
    pub(crate) fn in_direct_statement(&self) -> bool {
        self.current_span
            .is_some_and(|span| self.current_span_is_command && self.evaluates_directly(span))
    }

    /// The command `errorInfo` names for the statement being emitted when
    /// the release evaluates it without compiling it.
    pub(super) fn direct_statement_error_info(&self) -> Option<(String, u32)> {
        self.current_span
            .filter(|_| self.in_direct_statement())
            .map(|span| {
                // `errorInfo` is presentation: bytes that are not Unicode
                // are shown lossily rather than refused.
                let text = self.error_info_source_text(span);
                (
                    String::from_utf8_lossy(text.bytes()).into_owned(),
                    self.span_line(),
                )
            })
            .or_else(|| self.direct_statement.clone())
    }

    /// Clear the enclosing levels of a nested command no directly evaluated
    /// statement ends: compiled code logs only the innermost command, so the
    /// levels a native substitution named on the way out mean nothing there.
    pub(crate) fn drop_compiled_substitution_levels(&mut self) {
        let direct: std::collections::HashSet<String> = self
            .direct_commands
            .iter()
            .chain(&self.direct_body_commands)
            .map(|span| {
                String::from_utf8_lossy(self.error_info_source_text(*span).bytes()).into_owned()
            })
            .chain(self.direct_levels.iter().cloned())
            .collect();
        for instruction in &mut self.instructions {
            if let Some(command) = instruction.error_info_command.as_mut()
                && command.nested
                && command
                    .enclosing
                    .last()
                    .is_some_and(|(text, _)| !direct.contains(text))
            {
                command.enclosing.clear();
            }
        }
    }

    /// Name the commands a bracket script compiled by a native recipe emitted
    /// from `start`, whose instructions each carry their own command's text.
    /// Each logs as a nested command; one a deeper substitution named already
    /// gains this script's command as its next enclosing level; and in a
    /// statement the release evaluates directly, the statement logs last.
    /// Instructions of the enclosing statement's own (`statement`) are left.
    fn stamp_native_substitution_except(
        &mut self,
        start: usize,
        script: &str,
        statement: Option<&tcl_lexer::SourceImage>,
    ) {
        let separators = self.lexer_config().word_separators;
        let single = !cmd_subst::has_command_separator_with_config(script, self.lexer_config());
        let text = script.trim_matches(|c: char| {
            c == '\n' || u8::try_from(c).is_ok_and(|byte| separators.is_separator(byte))
        });
        let line = self
            .instructions
            .iter()
            .skip(start)
            .find(|instruction| instruction.source_cmd_text.try_text().ok() == Some(text))
            .map_or_else(|| self.span_line(), |instruction| instruction.source_line);
        let direct = self.direct_statement_error_info();
        if let Some((text, _)) = &direct {
            self.direct_levels.push(text.clone());
        }
        for instruction in self
            .instructions
            .iter_mut()
            .skip(start)
            .filter(|instruction| statement.is_none_or(|own| instruction.source_cmd_text != *own))
        {
            match instruction.error_info_command.as_mut() {
                Some(command) if command.nested => {
                    if single && !text.is_empty() {
                        // A level named under the directly evaluated
                        // statement goes inside it.
                        let at = if direct.is_some() && command.enclosing.last() == direct.as_ref()
                        {
                            command.enclosing.len() - 1
                        } else {
                            command.enclosing.len()
                        };
                        command.enclosing.insert(at, (text.to_owned(), line));
                    }
                }
                _ => {
                    let (own, own_line) = instruction.error_info_text();
                    // A command of its own text sits deeper in this script,
                    // which logs as its next level.
                    let enclosing = if single && !text.is_empty() && own != text {
                        vec![(text.to_owned(), line)]
                    } else {
                        Vec::new()
                    };
                    instruction.error_info_command = Some(tcl_bytecode::ErrorInfoCommand {
                        text: own.to_owned(),
                        line: own_line,
                        nested: true,
                        enclosing,
                    });
                }
            }
            if let (Some(statement), Some(command)) =
                (&direct, instruction.error_info_command.as_mut())
                && command.enclosing.last() != Some(statement)
            {
                command.enclosing.push(statement.clone());
            }
        }
    }

    /// A substitution of several commands runs as a script of its own: in a
    /// directly evaluated statement, the statement logs its error again.
    pub(crate) fn stamp_substitution_script(&mut self, start: usize) {
        let Some(statement) = self.direct_statement_error_info() else {
            return;
        };
        self.direct_levels.push(statement.0.clone());
        let text = self.command_span_text();
        for instruction in self
            .instructions
            .iter_mut()
            .skip(start)
            .filter(|instruction| instruction.source_cmd_text == text)
        {
            if instruction
                .error_info_command
                .as_ref()
                .is_none_or(|command| !command.nested)
            {
                let (text, line) = instruction.error_info_text();
                instruction.error_info_command = Some(tcl_bytecode::ErrorInfoCommand {
                    text: text.to_owned(),
                    line,
                    nested: true,
                    enclosing: vec![statement.clone()],
                });
            }
        }
    }

    /// The whole Tcl command represented by [`Self::current_span`], or empty
    /// when the span belongs only to compiler-generated control machinery.
    pub(super) fn command_span_text(&self) -> tcl_lexer::SourceImage {
        if self.current_span_is_command {
            self.exact_command_source
                .clone()
                .unwrap_or_else(|| self.span_text())
        } else {
            tcl_lexer::SourceImage::default()
        }
    }

    /// The surface text of an explicit `span` within the module source — for
    /// inline-body error regions, whose enclosing command's span differs from the
    /// per-instruction `current_span`. Empty when no source was supplied.
    pub(crate) fn source_text(&self, span: Span) -> tcl_lexer::SourceImage {
        let (s, e) = (span.start() as usize, span.end() as usize);
        tcl_lexer::SourceImage::from_bytes(
            self.source.get(s..e).unwrap_or_default(),
            self.source.channel(),
        )
    }

    /// The 1-based source line of an explicit `span`'s start (its first byte).
    /// `0` when no source was supplied (the span can't be located).
    pub(crate) fn source_line(&self, span: Span) -> u32 {
        if self.source.is_empty() {
            return 0;
        }
        self.line_at(span.start())
    }

    /// The indexed 1-based line containing `offset`.
    fn line_at(&self, offset: u32) -> u32 {
        let Some(line_index) = &self.line_index else {
            return 1;
        };
        if usize::try_from(offset).map_or(true, |offset| offset > self.source.len()) {
            return 1;
        }
        line_index.position_at(offset).line.saturating_add(1)
    }

    /// The 1-based line of `current_span` within the module source — the line a
    /// command reports in `errorInfo` (`(procedure … line N)` / `("while" body
    /// line N)`). `0` when no span is available. A hand-built context with a
    /// span but no source falls back to line one.
    pub(super) fn span_line(&self) -> u32 {
        match self.current_span {
            Some(sp) => self.line_at(sp.start()),
            None => 0,
        }
    }
}
