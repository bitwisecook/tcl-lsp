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

//! Ghost-token recovery and the syntax diagnostics for unterminated delimiters and lexer warnings.

use super::Analyser;
use std::collections::HashSet;
use tcl_core_types::DiagCode;
use tcl_lexer::Span;

impl Analyser {
    /// Ghost-token recovery.  When the scan-to-next
    /// recovery left a partial command (the shape that otherwise emits
    /// E200), re-lex the document with zero-width ghost `]` insertions
    /// derived from the E201 heuristics.  When any apply, the
    /// swallowed-command case (`set x [foo bar` then `puts done`) splits
    /// into a clean `[foo bar]` + `puts done` stream, `commands` is
    /// replaced with it, its E201 diagnostics are emitted, and `true` is
    /// returned (the caller then skips its own E201 detector to avoid a
    /// double-report).  Clean / fallback input has no partial command,
    /// so this never runs a second parse on the common path.
    pub(in super::super) fn apply_ghost_recovery(
        &mut self,
        source: &str,
        commands: &mut Vec<crate::segmenter::SegmentedCommand>,
    ) -> bool {
        if !commands.iter().any(|c| c.is_partial) {
            return false;
        }
        let (clean, e201) = crate::segmenter::segment_with_recovery(
            source,
            self.file_lexer_config(),
            &self.recovery_known_commands,
        );
        if e201.is_empty() {
            return false;
        }
        *commands = clean;
        self.result.diagnostics.extend(e201);
        true
    }

    /// Emit the source-recovery syntax diagnostics for one command's
    /// token stream: E100 / E102 stray closers and, unless
    /// ghost recovery already emitted them, E201 unterminated `[`
    /// command substitutions.  When `ghost_recovery_applied`,
    /// the stream is the ghost-recovered one and the E201s came from
    /// `segment_with_recovery`; a ghost-terminated command would look
    /// unterminated against the original bytes, so the detector is
    /// skipped to avoid a double-report.
    pub(super) fn emit_syntax_recovery_diagnostics(
        &mut self,
        cmd: &crate::segmenter::SegmentedCommand,
        ghost_recovery_applied: bool,
    ) {
        let stray = super::super::syntax_checks::stray_closer_diagnostics(
            cmd,
            &self.source,
            self.registry.as_deref(),
            || self.user_command_tail_names(),
        );
        self.result.diagnostics.extend(stray);
        if !ghost_recovery_applied {
            let e201 = super::super::syntax_checks::unterminated_bracket_diagnostics(
                cmd,
                &self.source,
                &self.recovery_known_commands,
                self.lexer_config(),
            );
            self.result.diagnostics.extend(e201);
        }
        // E202 / E203 fire for unterminated `"` / `{` whose token
        // wasn't split into a partial command (e.g. a quote run below
        // the segmenter's recovery line threshold).  Top-level scan,
        // so the region ends at the document end.
        self.emit_unterminated_delimiter_diagnostics(cmd, self.source.len());
    }

    /// Emit the E202 (unterminated `"`) / E203 (unterminated `{`)
    /// recovery diagnostics for one command, honouring the disable set.
    /// Returns `true` when at least one was emitted — the partial-command
    /// path uses this to suppress the generic E200.
    pub(in super::super) fn emit_unterminated_delimiter_diagnostics(
        &mut self,
        cmd: &crate::segmenter::SegmentedCommand,
        region_end: usize,
    ) -> bool {
        let diags = super::super::syntax_checks::unterminated_delimiter_diagnostics(
            cmd,
            &self.source,
            region_end,
            self.registry.as_deref(),
            &self.recovery_known_commands,
        );
        let mut emitted = false;
        for d in diags {
            if self.disabled_diagnostics.contains(d.code.as_str()) {
                continue;
            }
            self.result.diagnostics.push(d);
            emitted = true;
        }
        emitted
    }

    /// Convert the dialect-aware lexer's `LexWarning`s into recovery
    /// diagnostics.  E204 (extra characters after a close-brace), E205
    /// (after a close-quote) and E206 (missing close-brace for a
    /// `${…}` variable name) use the lexer's retained `ParseIssueKind`.
    /// Missing delimiters that overlap E201 / E202 / E203 are skipped (the
    /// recovery detectors own those with better positions + fixes);
    /// every other typed issue maps to the catch-all E200. Messages remain
    /// unchanged presentation.
    pub(super) fn emit_lexer_warning_diagnostics(&mut self) {
        let lexer = tcl_lexer::Lexer::with_source_map(
            Self::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            ),
            self.file_lexer_config(),
        );
        let Ok((tokens, warnings)) = lexer.tokenise_all_with_warnings() else {
            return;
        };
        for w in warnings {
            let code = match w.kind {
                // Owned by the E201/E202/E203 recovery heuristics.
                tcl_lexer::ParseIssueKind::MissingBracket
                | tcl_lexer::ParseIssueKind::MissingQuote
                | tcl_lexer::ParseIssueKind::MissingBrace => continue,
                tcl_lexer::ParseIssueKind::ExtraAfterBrace => DiagCode::E204,
                tcl_lexer::ParseIssueKind::ExtraAfterQuote => DiagCode::E205,
                tcl_lexer::ParseIssueKind::MissingVariableBrace => DiagCode::E206,
                // Any other typed lexer issue → catch-all E200.
                _ => DiagCode::E200,
            };
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    code,
                    Span::new(w.offset, w.offset),
                    w.message,
                    super::super::types::Severity::Error,
                ));
        }
        // Flat lexer warnings cannot see welded words inside a command
        // substitution. The shared parse-cut owner descends into those scripts
        // and keeps the original source coordinates.
        let config = self.file_lexer_config();
        let commands = tcl_lexer::group_commands(&tokens, &self.source, config);
        let Ok(cuts) = tcl_lexer::parse_cuts_in(&commands, &tokens, &self.source, config) else {
            return;
        };
        let mut emitted: HashSet<_> = self
            .result
            .diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.code, diagnostic.span.start()))
            .collect();
        for cut in cuts {
            let code = match cut.kind {
                tcl_lexer::ParseIssueKind::ExtraAfterBrace => DiagCode::E204,
                tcl_lexer::ParseIssueKind::ExtraAfterQuote => DiagCode::E205,
                _ => continue,
            };
            if !emitted.insert((code, cut.offset)) {
                continue;
            }
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    code,
                    Span::new(cut.offset, cut.offset),
                    cut.message.to_owned(),
                    super::super::types::Severity::Error,
                ));
        }
    }
}
