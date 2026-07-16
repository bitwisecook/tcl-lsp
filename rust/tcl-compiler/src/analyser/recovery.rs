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

//! Parser-error recovery heuristics.
//!
//! These helpers are invoked by the body-walk dispatcher
//! (``commands.rs::analyse_body``) immediately after segmentation
//! and before each command is dispatched.  They mutate the
//! [`SegmentedCommand`] in-place to repair common syntax errors
//! so downstream handlers (e.g. ``handle_switch_command``) see
//! the intended arg structure even when the source has a stray
//! ``]`` or a missing ``{``.
//!
//! The first two helpers:
//!
//! - [`Analyser::recover_stray_close_bracket`] — merges tokens
//!   around a stray ``]`` into a virtual ``CMD`` token so a
//!   typo like ``switch ACCESS::policy agent_id] {…}`` parses
//!   like ``switch ACCESS::policy [agent_id] {…}`` would have.
//!   Detection and insertion-point resolution are shared with the
//!   E100 diagnostic (`syntax_checks::find_first_stray_bracket` /
//!   `find_bracket_insertion_point`) so this repair only ever fires
//!   where E100 also fires, at the position E100's own quick-fix
//!   would insert ``[`` — two independent copies of this heuristic
//!   previously drifted apart and could repair (and corrupt
//!   downstream command-invocation recording for) brackets E100
//!   itself did not flag.
//! - [`looks_like_switch_case`] — peeks at a follow-on command
//!   to see if it looks like a ``pattern { body }`` pair (used
//!   by `recover_missing_open_brace`).
//!
//! The gnarlier helpers:
//!
//! - [`Analyser::recover_missing_open_brace`] — when a
//!   ``switch`` is followed by orphaned ``pattern body``
//!   command segments (because the user forgot the
//!   ``{``-body brace), the orphans are spliced into the
//!   switch's argv as additional pattern/body words and an
//!   E101 diagnostic is emitted.
//! - [`Analyser::detect_stolen_close_brace`] — when a partial
//!   command's body STR token has balanced inner braces and
//!   the inner ``{`` consumed the enclosing scope's ``}``,
//!   emits E103 instead of the generic E200.
//!
//! Both E101 and E103 emit a [`super::types::CodeFix`]
//! payload pointing at the exact insertion offset — range,
//! insertion text, and description.
//!
//! ## Quoted-context filtering
//!
//! ``]`` characters that live inside a double-quoted string
//! (e.g. ``foo "bar]"``) must be skipped.  This relies on the
//! lexer's ``Token::in_quote`` flag plus a leading-quote check
//! on adjacent ``Esc`` tokens; the segmenter already preserves
//! ``in_quote`` per-token.

use tcl_core_types::DiagCode;
use tcl_lexer::{Span, Token, TokenType};

use super::state::Analyser;
use crate::segmenter::SegmentedCommand;

impl Analyser {
    /// Repair a command whose source contains a stray ``]`` (a
    /// missing ``[``).
    ///
    /// Finds the first *trailing* stray ``]`` via
    /// [`super::syntax_checks::find_first_stray_bracket`] — the same
    /// escape/quote-aware detector the E100 diagnostic uses, so this
    /// repair only ever fires where E100 also fires — then resolves the
    /// insertion point via
    /// [`super::syntax_checks::find_bracket_insertion_point`] (the same
    /// heuristics that pick the diagnostic's own quick-fix location:
    /// own-token command-name prefix, backward scan for a known command
    /// word, arity overflow on the enclosing command).  Subsequent argv
    /// / texts / `single_token_word` entries are merged into a single
    /// virtual ``Cmd`` token so downstream dispatch sees the intended
    /// ``[name args]`` command-substitution shape.
    ///
    /// Returns silently when no resolution path succeeds; the
    /// command falls through to normal dispatch unchanged.
    pub(super) fn recover_stray_close_bracket(&self, cmd: &mut SegmentedCommand) {
        let Some((bracket_tok_idx, bracket_char_idx)) =
            super::syntax_checks::find_first_stray_bracket(&cmd.all_tokens, &self.source)
        else {
            return;
        };
        let bracket_tok = cmd.all_tokens[bracket_tok_idx];
        let bracket_argv_idx = match find_argv_index(&cmd.argv, bracket_tok) {
            Some(i) if i > 0 => i,
            _ => return,
        };
        let bracket_off = bracket_tok.span.start() + u32::try_from(bracket_char_idx).unwrap_or(0);

        let extra_known = self.user_command_tail_names();
        let registry: &tcl_registry::CommandRegistry = match self.registry {
            Some(r) => r,
            None => tcl_registry::cache::registry_for_profile(self.profile),
        };
        let Some(insert_off) = super::syntax_checks::find_bracket_insertion_point(
            cmd,
            &cmd.all_tokens,
            bracket_tok_idx,
            bracket_off,
            &self.source,
            registry,
            &extra_known,
        ) else {
            return;
        };

        let Some(cmd_start_argv_idx) = cmd.argv.iter().position(|t| t.span.start() == insert_off)
        else {
            return;
        };
        if cmd_start_argv_idx == 0 {
            return; // don't merge the enclosing command name itself
        }
        let Some(cmd_start_all_idx) = cmd
            .all_tokens
            .iter()
            .position(|t| t.span.start() == insert_off)
        else {
            return;
        };

        // Build the virtual Cmd token spanning from the start
        // of the resolved sub-command to the byte just before
        // the stray ``]``.  ``content_offset`` is **0** here
        // (not the usual ``1`` for real ``[…]`` tokens) because
        // the synthetic span doesn't include a leading ``[`` —
        // there's no ``[`` to skip in the source.  The whole
        // span is content; downstream token-text slicing
        // therefore yields the inner command directly.
        let src_start = insert_off as usize;
        let bracket_char_offset = bracket_off as usize;
        let virtual_span = Span::new(insert_off, bracket_off);
        let virtual_cmd = Token::with_content_offset(TokenType::Cmd, virtual_span, 0);

        // Splice all_tokens.
        cmd.all_tokens
            .splice(cmd_start_all_idx..=bracket_tok_idx, [virtual_cmd]);

        // Splice argv / texts / single_token_word.
        let virtual_text = format!("[{}]", &self.source[src_start..bracket_char_offset]);
        cmd.argv
            .splice(cmd_start_argv_idx..=bracket_argv_idx, [virtual_cmd]);
        cmd.texts
            .splice(cmd_start_argv_idx..=bracket_argv_idx, [virtual_text]);
        cmd.single_token_word
            .splice(cmd_start_argv_idx..=bracket_argv_idx, [true]);
    }

    /// User-declared command-like names visible so far in the walk:
    /// proc / class tail names, command-alias tail names, ensemble-
    /// namespace tail names, tclOO instance commands bound by `CLASS
    /// create NAME`, inline `# tcl-lsp: stub` declarations,
    /// `tclLsp.extraCommands`, and any explicit `unknown`-proc dispatch
    /// targets.
    ///
    /// Consulted by [`super::syntax_checks::find_bracket_insertion_point`]
    /// alongside the registry so the E100 bracket-insertion heuristic (and
    /// this module's repair, which shares it) recognises a call to an
    /// already-defined local command — a proc, a tclOO class or instance,
    /// an alias, an ensemble — not just a registry builtin.  Mirrors
    /// `Analyser::build_w123_known_names`'s candidate set; built fresh
    /// from whatever `self.result` holds *so far* (E100 runs inline
    /// during the walk, not as W123's post-pass), so a forward reference
    /// to a not-yet-declared name is out of scope here — the same
    /// backward-only visibility every other inline per-command check has.
    pub(super) fn user_command_tail_names(&self) -> std::collections::HashSet<String> {
        fn tail(qn: &str) -> Option<&str> {
            qn.rsplit_once("::")
                .map(|(_, t)| t)
                .filter(|t| !t.is_empty())
        }
        let mut names: std::collections::HashSet<String> = std::collections::HashSet::new();
        names.extend(
            self.result
                .all_procs
                .keys()
                .filter_map(|qn| tail(qn))
                .map(str::to_string),
        );
        names.extend(
            self.result
                .all_classes
                .keys()
                .filter_map(|qn| tail(qn))
                .map(str::to_string),
        );
        names.extend(
            self.result
                .command_aliases
                .keys()
                .filter_map(|qn| tail(qn))
                .map(str::to_string),
        );
        names.extend(
            self.ensemble_namespaces
                .iter()
                .filter_map(|ns| tail(ns))
                .map(str::to_string),
        );
        names.extend(self.result.created_instance_commands.iter().cloned());
        names.extend(self.extra_commands.iter().cloned());
        names.extend(super::utils::scan_stub_command_names(&self.source));
        if let Some(info) = self.result.unknown_proc_info.as_ref() {
            names.extend(info.dispatch_targets.iter().cloned());
        }
        names
    }

    /// Repair a ``switch`` command whose source is missing
    /// the ``{`` before the body, splicing orphaned
    /// ``pattern body`` segments into the switch's argv.
    ///
    /// Two shapes are recognised:
    ///
    /// - **Case A** — no whitespace after the switch's string
    ///   argument, so the EOL terminates the switch and **all**
    ///   pattern/body pairs become separate top-level commands.
    /// - **Case B** — whitespace after the string arg lets the
    ///   first pair merge as Form 1 args, leaving only the
    ///   *subsequent* pairs orphaned.
    ///
    /// In either case the orphaned commands are appended to
    /// ``cmd.argv`` / ``cmd.texts`` / ``cmd.all_tokens`` /
    /// ``cmd.single_token_word`` and an E101 diagnostic is
    /// emitted.
    /// Returns the number of consumed orphaned commands so the
    /// caller can advance its iterator past them.
    pub(super) fn recover_missing_open_brace(
        &mut self,
        cmd: &mut SegmentedCommand,
        commands: &[SegmentedCommand],
        cmd_idx: usize,
    ) -> usize {
        if cmd.texts.first().map_or("", String::as_str) != "switch" {
            return 0;
        }

        // Parse switch options.
        let args = if cmd.texts.len() > 1 {
            &cmd.texts[1..]
        } else {
            &[][..]
        };
        let mut arg_start: usize = 0;
        while arg_start < args.len() && args[arg_start].starts_with('-') {
            if args[arg_start] == "--" {
                arg_start += 1;
                break;
            }
            if matches!(args[arg_start].as_str(), "-matchvar" | "-indexvar") {
                arg_start += 2;
                continue;
            }
            arg_start += 1;
        }

        let non_option_args = if arg_start <= args.len() {
            &args[arg_start..]
        } else {
            &[][..]
        };

        // Form 2 detection: when the last non-option arg is a
        // braced body (Str token) and it's the only one after
        // the string, the switch is well-formed and recovery
        // is unnecessary.
        if non_option_args.len() >= 2 {
            let last_arg_idx = args.len().saturating_sub(1);
            let last_tok = if last_arg_idx + 1 < cmd.argv.len() {
                cmd.argv[last_arg_idx + 1]
            } else {
                *cmd.argv.last().unwrap_or(&cmd.argv[0])
            };
            if non_option_args.len() == 2
                && last_tok.kind == TokenType::Str
                && last_arg_idx == arg_start + 1
            {
                return 0;
            }
        }

        // Build a known-command set so ``looks_like_switch_case`` can
        // reject command-name-headed orphans.  Registry builtins alone
        // missed calls to procs/classes/aliases the analyser has
        // already tracked earlier in the same file — a genuine call
        // like ``renderReport { prose text }`` right after the case
        // list was silently swallowed as an extra case, corrupting the
        // switch's argv and running its braced argument text through
        // command analysis (a phantom "Unknown command" on prose).
        // Passed by reference for O(1) lookup in the per-command loop.
        let mut builtins_owned = self.builtin_command_names_const();
        builtins_owned.extend(self.user_command_tail_names());

        // Count consecutive case-like commands following the
        // switch.
        let mut case_count: usize = 0;
        for follow in commands.iter().skip(cmd_idx + 1) {
            if looks_like_switch_case(follow, &builtins_owned) {
                case_count += 1;
            } else {
                break;
            }
        }
        if case_count == 0 {
            return 0;
        }

        // Splice the orphans into the switch.
        for k in 0..case_count {
            let orphan = &commands[cmd_idx + 1 + k];
            for (text, (tok, single)) in orphan.texts.iter().zip(
                orphan
                    .argv
                    .iter()
                    .zip(orphan.single_token_word.iter().copied()),
            ) {
                cmd.argv.push(*tok);
                cmd.texts.push(text.clone());
                cmd.single_token_word.push(single);
                cmd.all_tokens.push(*tok);
            }
        }

        // Diagnostic anchor: end of the switch's string arg.
        let string_arg_argv_idx = arg_start + 1;
        let diag_span = if string_arg_argv_idx < cmd.argv.len() {
            let t = cmd.argv[string_arg_argv_idx];
            tcl_lexer::Span::new(t.span.end(), t.span.end())
        } else {
            tcl_lexer::Span::new(cmd.span.end(), cmd.span.end())
        };
        if !self.disabled_diagnostics.contains("E101") {
            // Insertion point: spans are exclusive-end, so
            // ``diag_span.end()`` is already the first byte
            // after the string-arg token — use it directly as
            // the zero-width insertion span.  Adding ``+1`` here
            // would shift the insertion one byte past the
            // intended location.
            let insert_span = tcl_lexer::Span::new(diag_span.end(), diag_span.end());
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::E101,
                span: diag_span,
                message: "Missing '{' after switch — body cases follow without braces".to_string(),
                severity: super::types::Severity::Error,
                fixes: vec![super::types::CodeFix {
                    span: insert_span,
                    new_text: " {".to_string(),
                    description: "Insert missing '{'".to_string(),
                }],
            });
        }

        case_count
    }

    /// Detect when an inner ``{`` consumed the enclosing scope's
    /// ``}`` and emit E103 instead of the generic E200.
    ///
    /// Walks the body STR token's text with a stack-based brace
    /// scan (skipping backslash-escaped pairs) and looks for the
    /// pattern where every ``{`` is matched by a ``}`` *and*
    /// the final ``}`` is the last significant content in the
    /// body.  When found, that ``}`` is the brace that "got
    /// stolen" — the missing one belongs after the inner block.
    ///
    /// Pure brace-counting cannot tell a single swallowed construct
    /// (``if {cond} {body}`` has two depth-1 pairs — condition and
    /// body — that are still one statement) from several genuinely
    /// separate ones swallowed by the same missing brace (a stray
    /// `if` block immediately followed by a sibling `proc`, which
    /// re-segments into two commands). Guessing in the latter case
    /// picks whichever brace happens to be last and offers a fix that
    /// silently nests the following statement(s) inside the unclosed
    /// body instead of closing it where the user meant — confirmed by
    /// applying the old fix and finding the "repaired" file parses
    /// clean but nests a sibling `proc` inside its neighbour. Re-
    /// segmenting the swallowed text with the real segmenter (which
    /// already understands command boundaries, unlike a byte scan)
    /// and requiring exactly one command out is the same signal
    /// `recover_missing_open_brace` already trusts elsewhere in this
    /// file, so this only fires on the unambiguous single-construct
    /// shape and abstains (falling back to the generic E200) otherwise.
    ///
    /// Returns ``true`` when E103 was emitted; the caller skips
    /// E200 in that case.
    pub(super) fn detect_stolen_close_brace(&mut self, cmd: &SegmentedCommand) -> bool {
        // Find the unclosed body STR token — last STR in argv.
        let mut body_tok: Option<Token> = None;
        for &tok in cmd.argv.iter().rev() {
            if tok.kind == TokenType::Str {
                body_tok = Some(tok);
                break;
            }
        }
        let Some(body_tok) = body_tok else {
            return false;
        };

        // Re-slice the body content.  ``content_offset`` skips
        // the opening ``{`` so ``text`` is the body's inner
        // content.
        let start = body_tok.span.start() as usize + body_tok.content_offset as usize;
        let end = body_tok.span.end() as usize;
        if start >= end || end > self.source.len() {
            return false;
        }
        let text = self.source[start..end].to_string();
        if text.is_empty() {
            return false;
        }

        // Abstain when the swallowed text spans more than one
        // top-level command — see the doc comment above.
        if crate::segmenter::segment_commands_with_offset(&text, 0).len() != 1 {
            return false;
        }

        // Stack-based brace scan, skipping backslash-escaped pairs.
        let bytes = text.as_bytes();
        let mut stack: Vec<usize> = Vec::new();
        let mut last_pop: Option<(usize, usize)> = None;
        let mut i: usize = 0;
        while i < bytes.len() {
            let ch = bytes[i];
            if ch == b'\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if ch == b'{' {
                stack.push(i);
            } else if ch == b'}' {
                let Some(open_off) = stack.pop() else {
                    return false; // more closes than opens
                };
                last_pop = Some((open_off, i));
            }
            i += 1;
        }

        if !stack.is_empty() {
            return false;
        }
        let Some((open_offset, close_offset)) = last_pop else {
            return false;
        };

        // The stolen ``}`` must be the last significant content.
        let trailing = &text[close_offset + 1..];
        if !trailing.trim().is_empty() {
            return false;
        }

        // Map body-text offsets to absolute source spans.
        let abs_close = u32::try_from(start + close_offset).expect("close offset fits in u32");
        let stolen_span = tcl_lexer::Span::new(abs_close, abs_close + 1);

        // Compute the indentation of the inner ``{`` line so the
        // CodeFix inserts a same-indent ``}`` on the next line.
        let line_start = text[..open_offset].rfind('\n').map_or(0, |i| i + 1);
        let mut indent = String::new();
        for c in text[line_start..].chars() {
            if c == ' ' || c == '\t' {
                indent.push(c);
            } else {
                break;
            }
        }
        // Insertion point: start of the line containing the
        // stolen ``}``.
        let stolen_line_start = text[..close_offset]
            .rfind('\n')
            .map_or(close_offset, |i| i + 1);
        let abs_insert =
            u32::try_from(start + stolen_line_start).expect("insert offset fits in u32");
        let insert_span = tcl_lexer::Span::new(abs_insert, abs_insert);

        if !self.disabled_diagnostics.contains("E103") {
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::E103,
                span: stolen_span,
                message: "Missing '}' — a nested body consumed this closing brace".to_string(),
                severity: super::types::Severity::Error,
                fixes: vec![super::types::CodeFix {
                    span: insert_span,
                    new_text: format!("{indent}}}\n"),
                    description: "Insert missing '}'".to_string(),
                }],
            });
        }
        true
    }

    /// Emit the generic E200 ("missing close-…") diagnostic for a
    /// partial command.
    ///
    /// Inspects the last unclosed token in the command to pick
    /// the right suffix (`brace` / `bracket` / `"`).  Always emits
    /// when E200 isn't disabled — callers gate this on
    /// `detect_stolen_close_brace` having returned ``false``.
    pub(super) fn emit_partial_command_diagnostic(&mut self, cmd: &SegmentedCommand) {
        if self.disabled_diagnostics.contains("E200") {
            return;
        }
        // The last unclosed Str/Cmd/Esc token in the command — both the
        // message suffix and the diagnostic's anchor come from it.
        let last_delim_tok = cmd
            .all_tokens
            .iter()
            .rev()
            .find(|t| matches!(t.kind, TokenType::Str | TokenType::Cmd | TokenType::Esc));
        // Prefer the delimiter the recovery segmenter recorded (from the
        // suspicious EOF-reaching token); fall back to the last
        // Str/Cmd/Esc token only when a partial wasn't produced by the
        // recovery path (so `partial_delimiter` is unset).
        let suffix = if let Some(delim) = cmd.partial_delimiter {
            delim.missing_message()
        } else {
            match last_delim_tok.map(|t| t.kind) {
                Some(TokenType::Cmd) => "missing close-bracket",
                Some(TokenType::Esc) => "missing \"",
                _ => "missing close-brace",
            }
        };
        // Anchor tightly at the unclosed delimiter's own opening position —
        // not `cmd.span`, which covers the *whole* partial command and can
        // run for many lines (the entire tail up to EOF). Zero-width,
        // matching the E201/E202/E203 convention, so the squiggle sits on
        // the actual problem instead of underlining unrelated source.
        let anchor = last_delim_tok.map_or(cmd.span.start(), |t| t.span.start());
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::E200,
            span: Span::new(anchor, anchor),
            message: suffix.to_string(),
            severity: super::types::Severity::Error,
            fixes: Vec::new(),
        });
    }

    /// Constant-folded view of [`Self::builtin_command_names`].
    ///
    /// `recover_stray_close_bracket` runs in `&self` context
    /// (it mutates the *command*, not the analyser) but
    /// `builtin_command_names` is `&mut self` because it lazily
    /// builds a cache.  This helper falls back to a fresh
    /// registry build when the cache is cold so the recovery
    /// path doesn't need exclusive access to the analyser.
    fn builtin_command_names_const(&self) -> std::collections::HashSet<String> {
        if let Some(cached) = self.builtin_names.as_ref() {
            return cached.clone();
        }
        tcl_registry::cache::registry_for_profile(self.profile)
            .command_names()
            .map(str::to_string)
            .collect()
    }
}

/// Return the index of `tok` in `argv`, or `None` if absent.
///
/// Identity is by source span — argv tokens are by-value
/// snapshots of ``all_tokens`` entries so direct equality on
/// the span start is the canonical match.
fn find_argv_index(argv: &[Token], tok: Token) -> Option<usize> {
    argv.iter().position(|t| t.span.start() == tok.span.start())
}

/// Return ``true`` when *cmd* looks like a bare ``pattern { body }``
/// or ``pattern -`` switch case.
///
/// Used by `recover_missing_open_brace` to decide whether
/// the next top-level command in a body should be merged into
/// a preceding `switch` whose ``{`` was forgotten.
///
/// A switch case has exactly two words: a pattern (which must
/// not be a known command name) followed either by a brace-
/// quoted body (`Str` token) or the literal `-` fall-through
/// marker.
///
/// `builtins` is the set of known command names — passed by
/// reference so the per-command recovery loop can use O(1)
/// lookup instead of an O(N) linear scan.
pub fn looks_like_switch_case<S: std::hash::BuildHasher>(
    cmd: &SegmentedCommand,
    builtins: &std::collections::HashSet<String, S>,
) -> bool {
    if cmd.texts.len() != 2 {
        return false;
    }
    let first = cmd.texts[0].as_str();
    if builtins.contains(first) {
        return false;
    }
    // Body must be brace-quoted (Str) or fall-through dash.
    if let Some(last_tok) = cmd.argv.last()
        && last_tok.kind == TokenType::Str
    {
        return true;
    }
    cmd.texts.last().map(String::as_str) == Some("-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segmenter::segment_commands_with_offset;

    fn analyser_with_source(source: &str) -> Analyser {
        let mut a = Analyser::new();
        a.source = source.to_string();
        a.profile = tcl_dialect::DialectProfile::by_name("tcl");
        a
    }

    fn segment(source: &str) -> SegmentedCommand {
        segment_commands_with_offset(source, 0)
            .into_iter()
            .next()
            .expect("at least one command")
    }

    #[test]
    fn recover_stray_close_bracket_repairs_typo_with_known_command() {
        // ``set x agent_id]`` — the stray ``]`` is recovered
        // when ``agent_id`` is a known command name.  Use a
        // real command name from the default registry so the
        // lookup succeeds.
        let source = "set x string]";
        let a = analyser_with_source(source);
        let mut cmd = segment(source);
        a.recover_stray_close_bracket(&mut cmd);
        // The recovery merges ``string]`` → ``[string]``.  The
        // virtual word lives at argv index 2 (after ``set`` /
        // ``x``).
        assert_eq!(cmd.texts.len(), 3);
        assert_eq!(cmd.texts[2], "[string]");
        assert_eq!(cmd.argv[2].kind, TokenType::Cmd);
    }

    #[test]
    fn recover_stray_close_bracket_no_op_when_no_stray_bracket() {
        // ``set x 1`` — no ``]`` at all, recovery is a no-op.
        let source = "set x 1";
        let a = analyser_with_source(source);
        let mut cmd = segment(source);
        let before = cmd.texts.clone();
        a.recover_stray_close_bracket(&mut cmd);
        assert_eq!(cmd.texts, before);
    }

    #[test]
    fn recover_stray_close_bracket_no_op_when_bracket_in_quoted_string() {
        // ``puts "bar]"`` — the ``]`` is inside a quoted run,
        // recovery must not fire.
        let source = r#"puts "bar]""#;
        let a = analyser_with_source(source);
        let mut cmd = segment(source);
        let before_argv_len = cmd.argv.len();
        a.recover_stray_close_bracket(&mut cmd);
        assert_eq!(cmd.argv.len(), before_argv_len);
    }

    #[test]
    fn recover_stray_close_bracket_does_not_merge_command_name() {
        // ``set x [missing arg]`` — the recovery must not eat
        // the ``set`` command name, only argv[1..].  Here the
        // stray bracket wraps ``missing arg`` at argv index >
        // 0.
        let source = "set string]";
        let a = analyser_with_source(source);
        let mut cmd = segment(source);
        a.recover_stray_close_bracket(&mut cmd);
        // ``set`` survives as cmd[0] regardless.
        assert_eq!(cmd.texts.first().map(String::as_str), Some("set"));
    }

    fn empty_builtins() -> std::collections::HashSet<String> {
        std::collections::HashSet::new()
    }

    fn builtins_with(names: &[&str]) -> std::collections::HashSet<String> {
        names.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn looks_like_switch_case_brace_body() {
        let source = "foo { puts hi }";
        let cmd = segment(source);
        assert!(looks_like_switch_case(&cmd, &empty_builtins()));
    }

    #[test]
    fn looks_like_switch_case_dash_fallthrough() {
        let source = "foo -";
        let cmd = segment(source);
        assert!(looks_like_switch_case(&cmd, &empty_builtins()));
    }

    #[test]
    fn looks_like_switch_case_rejects_known_command_at_head() {
        // ``set x`` is a real command — must not be confused
        // with a switch case.
        let source = "set x";
        let cmd = segment(source);
        assert!(!looks_like_switch_case(&cmd, &builtins_with(&["set"])));
    }

    #[test]
    fn looks_like_switch_case_rejects_three_or_more_words() {
        let source = "foo bar baz";
        let cmd = segment(source);
        assert!(!looks_like_switch_case(&cmd, &empty_builtins()));
    }

    #[test]
    fn looks_like_switch_case_rejects_single_word() {
        let source = "foo";
        let cmd = segment(source);
        assert!(!looks_like_switch_case(&cmd, &empty_builtins()));
    }

    // missing-open-brace recovery

    #[test]
    fn recover_missing_open_brace_emits_e101_and_consumes_orphans() {
        // ``switch $x\nfoo { puts hi }\nbar { puts bye }`` —
        // both ``foo`` and ``bar`` are orphaned switch cases
        // because the user forgot ``{`` after ``$x``.
        let source = "switch $x\nfoo { puts hi }\nbar { puts bye }";
        let mut a = analyser_with_source(source);
        let commands: Vec<SegmentedCommand> = segment_commands_with_offset(source, 0);
        assert!(commands.len() >= 3, "test expects 3 segmented commands");
        let mut switch_cmd = commands[0].clone();
        let consumed = a.recover_missing_open_brace(&mut switch_cmd, &commands, 0);
        assert_eq!(consumed, 2, "both orphans should be consumed");
        // The orphans were spliced in as additional argv words —
        // the switch now carries 4 pattern/body args plus the
        // string arg + the ``switch`` name.
        assert!(switch_cmd.texts.len() >= 6);
        // E101 was emitted.
        let e101 = a
            .result
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::E101)
            .expect("E101 emitted");
        assert!(e101.message.contains("Missing '{'"));
    }

    #[test]
    fn recover_missing_open_brace_no_op_for_form_2_switch() {
        // ``switch $x { a {b} c {d} }`` — already in Form 2
        // (single braced body), recovery must not fire.
        let source = "switch $x { a {b} c {d} }";
        let mut a = analyser_with_source(source);
        let commands: Vec<SegmentedCommand> = segment_commands_with_offset(source, 0);
        let mut switch_cmd = commands[0].clone();
        let consumed = a.recover_missing_open_brace(&mut switch_cmd, &commands, 0);
        assert_eq!(consumed, 0);
        assert!(
            !a.result
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::E101)
        );
    }

    #[test]
    fn recover_missing_open_brace_no_op_when_no_orphans() {
        // Plain ``switch`` followed by an unrelated command.
        let source = "switch $x\nputs hi";
        let mut a = analyser_with_source(source);
        let commands: Vec<SegmentedCommand> = segment_commands_with_offset(source, 0);
        let mut switch_cmd = commands[0].clone();
        let consumed = a.recover_missing_open_brace(&mut switch_cmd, &commands, 0);
        // ``puts hi`` is a known command so it can't be a
        // switch-case orphan.
        assert_eq!(consumed, 0);
    }

    #[test]
    fn recover_missing_open_brace_stops_at_known_user_proc() {
        // Regression: only registry builtins were excluded from
        // looking like a switch case, so a genuine call to an
        // already-declared user proc with a single braced argument
        // — ``renderReport { prose text }`` — was swallowed as an
        // extra orphaned case, corrupting the switch's argv and
        // running the braced prose through command analysis.
        let source = "switch $x\na { puts hi }\nrenderReport { prose text }";
        let mut a = analyser_with_source(source);
        a.extra_commands.insert("renderReport".to_string());
        let commands: Vec<SegmentedCommand> = segment_commands_with_offset(source, 0);
        let mut switch_cmd = commands[0].clone();
        let consumed = a.recover_missing_open_brace(&mut switch_cmd, &commands, 0);
        assert_eq!(
            consumed, 1,
            "only the genuine case should be consumed, not the renderReport call"
        );
    }

    #[test]
    fn recover_missing_open_brace_skips_non_switch() {
        let source = "set x 1";
        let mut a = analyser_with_source(source);
        let commands: Vec<SegmentedCommand> = segment_commands_with_offset(source, 0);
        let mut cmd = commands[0].clone();
        let consumed = a.recover_missing_open_brace(&mut cmd, &commands, 0);
        assert_eq!(consumed, 0);
    }

    // stolen-close-brace detection

    #[test]
    fn detect_stolen_close_brace_emits_e103_for_inner_brace_pattern() {
        // ``proc foo {} { switch $x { a { puts hi } }`` — the
        // inner ``{ a { puts hi } }`` consumed the outer
        // ``proc`` body's closing brace, leaving the proc
        // unclosed.  Build a synthetic command with a body STR
        // token whose text matches the stolen-brace pattern.
        let source = "{ switch $x {\n    a { puts hi }\n}}";
        let mut a = analyser_with_source(source);
        let commands: Vec<SegmentedCommand> = segment_commands_with_offset(source, 0);
        let cmd = &commands[0];
        let detected = a.detect_stolen_close_brace(cmd);
        assert!(detected, "stolen brace pattern should be detected");
        let e103 = a
            .result
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::E103)
            .expect("E103 emitted");
        assert!(e103.message.contains("Missing '}'"));
    }

    #[test]
    fn detect_stolen_close_brace_no_op_when_no_str_token() {
        // ``set x 1`` has no STR body token.
        let source = "set x 1";
        let mut a = analyser_with_source(source);
        let commands: Vec<SegmentedCommand> = segment_commands_with_offset(source, 0);
        let cmd = &commands[0];
        assert!(!a.detect_stolen_close_brace(cmd));
        assert!(a.result.diagnostics.is_empty());
    }

    #[test]
    fn detect_stolen_close_brace_no_op_when_unbalanced() {
        // Body whose inner content has more ``}`` than ``{`` —
        // the lexer can produce that shape when the body STR
        // contains a literal escaped close-brace via ``\}``
        // before any opening brace.  Our scan skips ``\}``
        // pairs, so the body sees zero ``{`` and zero ``}``,
        // and the early-out for "more closes than opens" or
        // "no last_pop" rejects the input.
        let source = r"{ \} a }";
        let mut a = analyser_with_source(source);
        let commands: Vec<SegmentedCommand> = segment_commands_with_offset(source, 0);
        let cmd = &commands[0];
        let detected = a.detect_stolen_close_brace(cmd);
        assert!(!detected);
    }

    #[test]
    fn detect_stolen_close_brace_no_op_when_trailing_content() {
        // Body where the last ``}`` is followed by more text —
        // it legitimately closed an inner block, the missing
        // ``}`` is the outer one.
        let source = "{ a { puts hi } extra }";
        let mut a = analyser_with_source(source);
        let commands: Vec<SegmentedCommand> = segment_commands_with_offset(source, 0);
        let cmd = &commands[0];
        // Balanced braces, so detect_stolen_close_brace will
        // return false because there's content after the last
        // ``}``.
        let detected = a.detect_stolen_close_brace(cmd);
        assert!(!detected);
    }

    #[test]
    fn detect_stolen_close_brace_no_op_when_multiple_top_level_commands_swallowed() {
        // Regression: a missing ``}`` followed by more than one
        // subsequent top-level statement (here a sibling ``proc``,
        // not just the one control-structure that stole the brace)
        // used to still fire, picking the LAST balanced closer in the
        // swallowed text — the sibling proc's own closing brace.
        // Confirmed by hand: applying that fix nested the sibling
        // proc inside the unclosed one instead of closing it where
        // the missing brace actually belongs, which parses clean but
        // silently changes the program. Re-segmenting the swallowed
        // text now shows two commands here, so this abstains in
        // favour of the generic (fix-less but not misleading) E200.
        let source = "{\n    if {1} {\n        puts hi\n    }\nproc bar {} {\n    return 1\n}\n";
        let mut a = analyser_with_source(source);
        let commands: Vec<SegmentedCommand> = segment_commands_with_offset(source, 0);
        let cmd = &commands[0];
        let detected = a.detect_stolen_close_brace(cmd);
        assert!(
            !detected,
            "ambiguous multi-statement swallow must not guess a fix location"
        );
        assert!(
            !a.result
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::E103)
        );
    }

    #[test]
    fn emit_partial_uses_stored_delimiter_for_e200_message() {
        use crate::segmenter::UnclosedDelimiter;
        // The precise E200 message comes from the recorded
        // `partial_delimiter`, not the last-token heuristic.
        for (delim, want) in [
            (UnclosedDelimiter::Brace, "missing close-brace"),
            (UnclosedDelimiter::Bracket, "missing close-bracket"),
            (UnclosedDelimiter::Quote, "missing \""),
        ] {
            let source = "x";
            let mut a = analyser_with_source(source);
            let mut cmd = crate::segmenter::segment_commands_with_offset(source, 0)[0].clone();
            cmd.is_partial = true;
            cmd.partial_delimiter = Some(delim);
            a.emit_partial_command_diagnostic(&cmd);
            let d = a
                .result
                .diagnostics
                .iter()
                .find(|d| d.code == DiagCode::E200)
                .unwrap();
            assert_eq!(d.message, want, "{delim:?}");
        }
    }

    #[test]
    fn emit_partial_command_diagnostic_emits_e200_for_unclosed_brace() {
        // Build a fake partial command via segment (a really
        // unclosed brace at end of source).
        let source = "{ unclosed";
        let mut a = analyser_with_source(source);
        let commands: Vec<SegmentedCommand> = segment_commands_with_offset(source, 0);
        let cmd = &commands[0];
        a.emit_partial_command_diagnostic(cmd);
        assert!(
            a.result
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::E200)
        );
    }

    #[test]
    fn emit_partial_command_diagnostic_anchors_at_delimiter_not_command_start() {
        // A multi-word command whose *last* word is the unclosed delimiter:
        // the E200 span must sit at that word's start, not at the
        // command's own start (which would underline the whole, possibly
        // multi-line, command through EOF — a loose, unhelpful highlight).
        let source = "oo::class create Foo {\n  method bar {} {\n    puts hi\n";
        let mut a = analyser_with_source(source);
        let commands: Vec<SegmentedCommand> = segment_commands_with_offset(source, 0);
        let cmd = &commands[0];
        // Sanity: this is genuinely one multi-word command, not already
        // split at the delimiter.
        assert!(cmd.span.start() < 21, "expected the command to start at 0");
        a.emit_partial_command_diagnostic(cmd);
        let d = a
            .result
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::E200)
            .unwrap();
        assert_eq!(
            (d.span.start(), d.span.end()),
            (21, 21),
            "expected a zero-width anchor at the unclosed `{{` (byte 21), not \
             the command start (byte {}) or a wide span",
            cmd.span.start()
        );
    }
}
