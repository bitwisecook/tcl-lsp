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
        commands: &mut Vec<tcl_source::segment::SegmentedCommand>,
    ) -> bool {
        if !commands.iter().any(|c| c.is_partial) {
            return false;
        }
        let (clean, e201) = segment_with_recovery(
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
        cmd: &tcl_source::segment::SegmentedCommand,
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
        cmd: &tcl_source::segment::SegmentedCommand,
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

/// Segment `source` with ghost-token error recovery.
///
/// Does a first plain parse, runs the E201 unterminated-`[` heuristics
/// over it to derive zero-width ghost `]` insertions (the
/// comment / known-command / brace cases), and — when any are found —
/// re-lexes with those ghosts so a swallowed following command
/// (`set x [foo bar` then `puts done`) splits into a clean
/// `[foo bar]` + `puts done` stream.  Returns the (possibly re-lexed)
/// commands **and** the E201 diagnostics for each applied ghost; an
/// empty diagnostic list means no recovery was applied and the commands
/// are exactly [`segment_commands_local`] (the caller then
/// keeps its own scan-to-next stream).
///
/// `known` is the "known command" name universe the E201 heuristics
/// consult (see `analyser::utils::recovery_known_commands`) — the active
/// registry's names plus every proc/class/alias the document itself
/// defines, so a break just before a call to a user-defined proc recovers
/// as readily as one before a builtin.
///
/// The E204-E206 lexer-warning
/// codes are emitted separately by the analyser.
fn segment_with_recovery(
    source: &str,
    config: tcl_lexer::LexerConfig,
    known: &crate::analyser::utils::RecoveryKnownCommands,
) -> (
    Vec<tcl_source::segment::SegmentedCommand>,
    Vec<crate::analyser::types::Diagnostic>,
) {
    // Cap the re-lex iterations to bound work on pathological input.
    const MAX_GHOST_RECOVERY_PASSES: usize = 32;

    let commands = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
    if commands.is_empty() {
        return (commands, Vec::new());
    }
    // Accumulated zero-width ghost `]` insertions (keyed by original-source
    // offset — ghosts don't shift later offsets, so the map stays valid
    // across re-lexes).  `diag_by_bracket` keeps one E201 per *bracket*
    // (keyed on its `[` offset), preferring a fix-bearing diagnostic over a
    // fix-less fallback so a bracket recovered by a ghost reports its
    // insertion fix, not the bare fallback the re-lexed stream would yield.
    let mut ghosts: std::collections::BTreeMap<u32, u8> = std::collections::BTreeMap::new();
    let mut diag_by_bracket: std::collections::BTreeMap<u32, crate::analyser::types::Diagnostic> =
        std::collections::BTreeMap::new();
    let sm = tcl_lexer::SourceMap::new(source);
    let mut current = commands;
    // Iterate: a single re-lex can expose a *further* unterminated `[` that
    // the previous parse had swallowed (`set a [foo` / `set c [bar` /
    // `proc …`).  Re-derive ghosts from the re-lexed stream and re-lex
    // again until a pass adds no new ghost.
    for _ in 0..MAX_GHOST_RECOVERY_PASSES {
        let mut new_ghost = false;
        for cmd in &current {
            for diag in crate::analyser::syntax_checks::unterminated_bracket_diagnostics(
                cmd, source, known, config,
            ) {
                let bracket_off = diag.span.start();
                // A heuristic case carries a `]`-insertion fix whose offset
                // is the ghost offset; a bare fallback has no fix and stays
                // unterminated after the re-lex.
                if let Some(fix) = diag.fixes.first()
                    && let std::collections::btree_map::Entry::Vacant(e) =
                        ghosts.entry(fix.span.start())
                {
                    e.insert(b']');
                    new_ghost = true;
                }
                // Keep a fix-bearing diagnostic once recorded; otherwise take
                // whatever (fix-bearing or fallback) we see first.
                let keep_existing = diag_by_bracket
                    .get(&bracket_off)
                    .is_some_and(|d| !d.fixes.is_empty());
                if !keep_existing {
                    diag_by_bracket.insert(bracket_off, diag);
                }
            }
        }
        if !new_ghost {
            break;
        }
        let (document, _warnings) =
            tcl_source::cst::build::build_document_with_ghosts(source, config, ghosts.clone());
        current = tcl_source::cst::segment::segments_from_document(document, &sm);
    }
    if ghosts.is_empty() {
        // No heuristic insertion → no re-lex.  The caller keeps its
        // scan-to-next stream and its own (fallback) E201 detector.
        return (current, Vec::new());
    }
    let diagnostics: Vec<crate::analyser::types::Diagnostic> =
        diag_by_bracket.into_values().collect();
    (current, diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::LexerConfig;
    use tcl_source::segment::{SegmentedCommand, segment_commands};
    #[test]
    fn recovery_splits_swallowed_following_command() {
        // An unterminated `[` whose next line is a
        // known command re-lexes into a clean two-command stream and
        // yields an E201 diagnostic.
        let reg = tcl_registry::CommandRegistry::build_default();
        let known: crate::analyser::utils::RecoveryKnownCommands =
            reg.command_names().map(str::to_owned).collect();
        let cfg = LexerConfig::default();
        let (rec, diags) = segment_with_recovery("set x [foo bar\nputs done\n", cfg, &known);
        assert_eq!(rec.len(), 2);
        assert_eq!(rec[0].texts, vec!["set", "x", "[foo bar]"]);
        assert_eq!(rec[1].texts, vec!["puts", "done"]);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagCode::E201);
    }

    #[test]
    fn recovery_is_a_noop_on_clean_input() {
        let reg = tcl_registry::CommandRegistry::build_default();
        let known: crate::analyser::utils::RecoveryKnownCommands =
            reg.command_names().map(str::to_owned).collect();
        let cfg = LexerConfig::default();
        let src = "set ok [foo]\nputs hi\n";
        let (rec, diags) = segment_with_recovery(src, cfg, &known);
        let plain = segment_commands(src);
        // No recoverable imbalance → byte-identical words, no diagnostics.
        let words =
            |cs: &[SegmentedCommand]| cs.iter().map(|c| c.texts.clone()).collect::<Vec<_>>();
        assert_eq!(words(&rec), words(&plain));
        assert_eq!(diags, [] as [crate::analyser::types::Diagnostic; 0]);
    }
}
