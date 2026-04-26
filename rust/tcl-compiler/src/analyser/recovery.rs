//! Parser-error recovery heuristics — Rust port of
//! `core/analysis/_analyser/_recovery.py`.
//!
//! These helpers are invoked by the body-walk dispatcher
//! (``commands.rs::analyse_body``) immediately after segmentation
//! and before each command is dispatched.  They mutate the
//! [`SegmentedCommand`] in-place to repair common syntax errors
//! so downstream handlers (e.g. ``handle_switch_command``) see
//! the intended arg structure even when the source has a stray
//! ``]`` or a missing ``{``.
//!
//! **C41e4** lands the first two helpers:
//!
//! - [`Analyser::recover_stray_close_bracket`] — merges tokens
//!   around a stray ``]`` into a virtual ``CMD`` token so a
//!   typo like ``switch ACCESS::policy agent_id] {…}`` parses
//!   like ``switch ACCESS::policy [agent_id] {…}`` would have.
//! - [`looks_like_switch_case`] — peeks at a follow-on command
//!   to see if it looks like a ``pattern { body }`` pair (used
//!   by C41e5's ``_recover_missing_open_brace``).
//!
//! **C41e5** adds the gnarlier helpers:
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
//! **Deferred:** the Python helpers attach a [`CodeFix`] to the
//! E101 / E103 diagnostics that points at the exact insertion
//! offset.  The Rust ``Diagnostic`` struct doesn't carry
//! ``CodeFix`` yet (chunk-log carry-over: "W123 'did you
//! mean…?' suggestions + CodeFix payload — needs ``CodeFix``
//! field on ``Diagnostic``").  When the field lands the
//! emitter call sites in ``recover_missing_open_brace`` /
//! ``detect_stolen_close_brace`` populate it; for now they
//! emit code + message + range only.
//!
//! ## Quoted-context filtering
//!
//! The Python port uses
//! ``core.parsing.token_positions.classify_quoted_contexts``
//! to skip ``]`` characters that live inside a double-quoted
//! string (e.g. ``foo "bar]"``).  The Rust port relies on the
//! lexer's ``Token::in_quote`` flag plus a leading-quote check
//! on adjacent ``Esc`` tokens — same signal sources, simpler
//! plumbing because the segmenter already preserves
//! ``in_quote`` per-token.

#![allow(clippy::doc_markdown)]

use tcl_lexer::{Span, Token, TokenType};

use super::state::Analyser;
use crate::segmenter::SegmentedCommand;

impl Analyser {
    /// Repair a command whose source contains a stray ``]`` (a
    /// missing ``[``).
    ///
    /// Mirrors `_recover_stray_close_bracket` in
    /// `core/analysis/_analyser/_recovery.py:30-156`.  Walks
    /// `cmd.all_tokens` looking for an `Esc` token that ends in
    /// ``]`` (and isn't inside a double-quoted string), then
    /// scans backward for a known command name — that's the
    /// position the missing ``[`` should have been at.
    /// Subsequent argv / texts / single_token_word entries are
    /// merged into a single virtual ``Cmd`` token so
    /// downstream dispatch sees the intended ``[name args]``
    /// command-substitution shape.
    ///
    /// Three resolution paths, in priority order:
    ///
    /// 1. The bracket token's own text starts with a known
    ///    command name (e.g. ``foo]``) — the bracket itself
    ///    is the start.
    /// 2. A preceding `Esc` token is exactly a known command
    ///    name — the merge starts there.
    /// 3. Arity fallback — when the enclosing command has a
    ///    bounded ``max`` and the argv overflows, the missing
    ///    ``[`` belongs before the ``max``-th argument.
    ///
    /// Returns silently when no resolution path succeeds; the
    /// command falls through to normal dispatch unchanged.
    pub(super) fn recover_stray_close_bracket(&self, cmd: &mut SegmentedCommand) {
        let Some((bracket_tok_idx, bracket_char_idx)) =
            find_stray_close_bracket(&cmd.all_tokens, &self.source)
        else {
            return;
        };
        let bracket_tok = cmd.all_tokens[bracket_tok_idx];
        let bracket_argv_idx = match find_argv_index(&cmd.argv, bracket_tok) {
            Some(i) if i > 0 => i,
            _ => return,
        };

        let known = self.builtin_command_names_const();
        let prefix = if bracket_char_idx > 0 {
            let span = bracket_tok.span;
            let start = span.start() as usize;
            let end = start + bracket_char_idx;
            &self.source[start..end]
        } else {
            ""
        };

        let mut cmd_start_all_idx: Option<usize> = None;
        let mut cmd_start_argv_idx: Option<usize> = None;

        if known.contains(prefix) {
            cmd_start_all_idx = Some(bracket_tok_idx);
            cmd_start_argv_idx = Some(bracket_argv_idx);
        } else {
            for i in (1..bracket_tok_idx).rev() {
                let t = cmd.all_tokens[i];
                if t.kind != TokenType::Esc {
                    continue;
                }
                let text = self.token_text(t);
                if known.contains(text) {
                    cmd_start_all_idx = Some(i);
                    cmd_start_argv_idx = find_argv_index(&cmd.argv, t);
                    break;
                }
            }
        }

        // Arity-based fallback.
        if cmd_start_all_idx.is_none() || cmd_start_argv_idx.is_none() {
            let cmd_name = cmd.texts.first().map_or("", String::as_str);
            if let Some(spec_max) = lookup_max_arity(cmd_name) {
                let nargs = cmd.argv.len().saturating_sub(1);
                if nargs > spec_max && spec_max >= 1 {
                    let target_argv_idx = spec_max;
                    if target_argv_idx < cmd.argv.len() {
                        let target_tok = cmd.argv[target_argv_idx];
                        if target_tok.span.start() < bracket_tok.span.start() {
                            cmd_start_argv_idx = Some(target_argv_idx);
                            for (j, t) in cmd.all_tokens.iter().enumerate() {
                                if t.span.start() == target_tok.span.start() {
                                    cmd_start_all_idx = Some(j);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        let Some(cmd_start_all_idx) = cmd_start_all_idx else {
            return;
        };
        let Some(cmd_start_argv_idx) = cmd_start_argv_idx else {
            return;
        };
        if cmd_start_argv_idx == 0 {
            return; // don't merge the enclosing command name itself
        }

        // Build the virtual Cmd token spanning from the start
        // of the resolved sub-command to the byte just before
        // the stray ``]``.  ``content_offset = 1`` mirrors a
        // real ``[…]`` token where the ``[`` is the leading
        // delimiter byte.
        let src_start = cmd.argv[cmd_start_argv_idx].span.start() as usize;
        let bracket_char_offset = bracket_tok.span.start() as usize + bracket_char_idx;
        let virtual_span = Span::new(
            u32::try_from(src_start).expect("src_start fits in u32"),
            u32::try_from(bracket_char_offset).expect("bracket end fits in u32"),
        );
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

    /// Repair a ``switch`` command whose source is missing
    /// the ``{`` before the body, splicing orphaned
    /// ``pattern body`` segments into the switch's argv.
    ///
    /// Mirrors `_recover_missing_open_brace` in
    /// `core/analysis/_analyser/_recovery.py:179-324`.  Two
    /// shapes Python recognises:
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
    /// emitted (no ``CodeFix`` payload yet — see module docs).
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

        // Parse switch options (mirrors Python's option scan).
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

        // Build a builtins set so ``looks_like_switch_case``
        // can reject command-name-headed orphans.
        let builtins_owned = self.builtin_command_names_const();
        let builtins_view: Vec<&str> = builtins_owned.iter().map(String::as_str).collect();

        // Count consecutive case-like commands following the
        // switch.
        let mut case_count: usize = 0;
        for follow in commands.iter().skip(cmd_idx + 1) {
            if looks_like_switch_case(follow, &builtins_view) {
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
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "E101".to_string(),
                span: diag_span,
                message: "Missing '{' after switch — body cases follow without braces".to_string(),
                severity: super::types::Severity::Error,
            });
        }

        case_count
    }

    /// Detect when an inner ``{`` consumed the enclosing scope's
    /// ``}`` and emit E103 instead of the generic E200.
    ///
    /// Mirrors `_detect_stolen_close_brace` in
    /// `core/analysis/_analyser/_recovery.py:326-449`.  Walks
    /// the body STR token's text with a stack-based brace scan
    /// (skipping backslash-escaped pairs) and looks for the
    /// pattern where every ``{`` is matched by a ``}`` *and*
    /// the final ``}`` is the last significant content in the
    /// body.  When found, that ``}`` is the brace that "got
    /// stolen" — the missing one belongs after the inner block.
    ///
    /// Returns ``true`` when E103 was emitted; the caller skips
    /// E200 in that case.  ``CodeFix`` payload deferred — see
    /// module docs.
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
        // the opening ``{`` so ``text`` mirrors Python's
        // ``body_tok.text``.
        let start = body_tok.span.start() as usize + body_tok.content_offset as usize;
        let end = body_tok.span.end() as usize;
        if start >= end || end > self.source.len() {
            return false;
        }
        let text = self.source[start..end].to_string();
        if text.is_empty() {
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
        let Some((_open_offset, close_offset)) = last_pop else {
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

        if !self.disabled_diagnostics.contains("E103") {
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "E103".to_string(),
                span: stolen_span,
                message: "Missing '}' — a nested body consumed this closing brace".to_string(),
                severity: super::types::Severity::Error,
            });
        }
        true
    }

    /// Emit the generic E200 ("missing close-…") diagnostic for a
    /// partial command.
    ///
    /// Mirrors the E200 emission in
    /// `core/analysis/_analyser/_core.py:476-493`.  Inspects the
    /// last unclosed token in the command to pick the right
    /// suffix (`brace` / `bracket` / `"`).  Always emits when
    /// E200 isn't disabled — callers gate this on
    /// `detect_stolen_close_brace` having returned ``false``.
    pub(super) fn emit_partial_command_diagnostic(&mut self, cmd: &SegmentedCommand) {
        if self.disabled_diagnostics.contains("E200") {
            return;
        }
        // Pick the last token whose span reaches EOF / the
        // command end; that's the unclosed delimiter.
        let kind = cmd
            .all_tokens
            .iter()
            .rev()
            .find(|t| matches!(t.kind, TokenType::Str | TokenType::Cmd | TokenType::Esc))
            .map(|t| t.kind);
        let suffix = match kind {
            Some(TokenType::Cmd) => "missing close-bracket",
            Some(TokenType::Esc) => "missing \"",
            _ => "missing close-brace",
        };
        self.result.diagnostics.push(super::types::Diagnostic {
            code: "E200".to_string(),
            span: cmd.span,
            message: suffix.to_string(),
            severity: super::types::Severity::Error,
        });
    }

    /// Lookup a token's text by re-slicing the analyser's source
    /// buffer.
    ///
    /// `Token` stores a span only — text is materialised on
    /// demand. The leading delimiter (``$`` / ``${`` / ``[`` /
    /// ``{`` / ``"``) is stripped via ``content_offset`` so the
    /// returned slice mirrors Python's ``Token.text``.
    fn token_text(&self, tok: Token) -> &str {
        let start = tok.span.start() as usize + tok.content_offset as usize;
        let end = tok.span.end() as usize;
        &self.source[start..end.min(self.source.len())]
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
        use tcl_registry::prelude::DialectSet;
        use tcl_registry::CommandRegistry;
        if let Some(cached) = self.builtin_names.as_ref() {
            return cached.clone();
        }
        let mut registry = CommandRegistry::build_default();
        if let Some(d) = DialectSet::parse(&self.dialect) {
            registry.load_dialect(d);
        }
        registry.command_names().map(str::to_string).collect()
    }
}

/// Locate the first stray ``]`` ESC token in a command's token
/// stream, ignoring brackets inside double-quoted runs.
///
/// Returns ``Some((token_index, char_index_within_token))`` when
/// a stray bracket is found.  The bracket must be the *last*
/// character of its token text (``foo]``, not ``foo]bar``) to
/// match the Python contract — partial mid-token brackets are
/// usually not the recovery shape we're after.
fn find_stray_close_bracket(tokens: &[Token], source: &str) -> Option<(usize, usize)> {
    // Track quoted context across the token stream.  Mirrors
    // `classify_quoted_contexts` in
    // `core/parsing/token_positions.py:14-56`.
    let mut prev_in_quote = false;
    for (ti, &tok) in tokens.iter().enumerate() {
        if matches!(tok.kind, TokenType::Sep | TokenType::Eol) {
            prev_in_quote = false;
            continue;
        }
        let self_opens = tok.kind == TokenType::Esc && tok.content_offset > 0;
        let in_quoted_word = prev_in_quote || self_opens;
        prev_in_quote = tok.in_quote;
        if tok.kind != TokenType::Esc {
            continue;
        }
        if in_quoted_word {
            continue;
        }
        let span = tok.span;
        let start = span.start() as usize + tok.content_offset as usize;
        let end = span.end() as usize;
        if start >= end || end > source.len() {
            continue;
        }
        let text = &source[start..end];
        // ``]`` must be the trailing character.
        if let Some(idx) = text.rfind(']') {
            if idx == text.len() - 1 {
                // Char index within the token (relative to
                // span start, not content start) — the Python
                // code computes ``idx == len(tok.text) - 1``
                // and then offsets from ``tok.start.offset``;
                // mirror that arithmetic by adding the content
                // offset back.
                let char_idx = idx + tok.content_offset as usize;
                return Some((ti, char_idx));
            }
        }
    }
    None
}

/// Return the index of `tok` in `argv`, or `None` if absent.
///
/// Identity is by source span — argv tokens are by-value
/// snapshots of ``all_tokens`` entries so direct equality on
/// the span start is the canonical match.
fn find_argv_index(argv: &[Token], tok: Token) -> Option<usize> {
    argv.iter().position(|t| t.span.start() == tok.span.start())
}

/// Look up the maximum argv arity for a command from the
/// default registry.
///
/// Returns ``None`` for unknown commands or for commands with
/// an unbounded upper bound.  `max` of ``0`` is also treated as
/// "no useful constraint" — the recovery only fires when there
/// are *excess* args beyond a known cap.
fn lookup_max_arity(cmd_name: &str) -> Option<usize> {
    use tcl_registry::CommandRegistry;
    if cmd_name.is_empty() {
        return None;
    }
    let registry = CommandRegistry::build_default();
    let spec = registry.get(cmd_name)?;
    if spec.arity.is_unlimited() {
        return None;
    }
    Some(spec.arity.max as usize)
}

/// Return ``true`` when *cmd* looks like a bare ``pattern { body }``
/// or ``pattern -`` switch case.
///
/// Mirrors `_looks_like_switch_case` in
/// `core/analysis/_analyser/_recovery.py:160-177`.  Used by
/// **C41e5**'s `recover_missing_open_brace` to decide whether
/// the next top-level command in a body should be merged into
/// a preceding `switch` whose ``{`` was forgotten.
///
/// A switch case has exactly two words: a pattern (which must
/// not be a known command name) followed either by a brace-
/// quoted body (`Str` token) or the literal `-` fall-through
/// marker.
pub fn looks_like_switch_case(cmd: &SegmentedCommand, builtins: &[&str]) -> bool {
    if cmd.texts.len() != 2 {
        return false;
    }
    let first = cmd.texts[0].as_str();
    if builtins.contains(&first) {
        return false;
    }
    // Body must be brace-quoted (Str) or fall-through dash.
    if let Some(last_tok) = cmd.argv.last() {
        if last_tok.kind == TokenType::Str {
            return true;
        }
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
        a.dialect = "tcl".to_string();
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

    #[test]
    fn looks_like_switch_case_brace_body() {
        let source = "foo { puts hi }";
        let cmd = segment(source);
        assert!(looks_like_switch_case(&cmd, &[]));
    }

    #[test]
    fn looks_like_switch_case_dash_fallthrough() {
        let source = "foo -";
        let cmd = segment(source);
        assert!(looks_like_switch_case(&cmd, &[]));
    }

    #[test]
    fn looks_like_switch_case_rejects_known_command_at_head() {
        // ``set x`` is a real command — must not be confused
        // with a switch case.
        let source = "set x";
        let cmd = segment(source);
        assert!(!looks_like_switch_case(&cmd, &["set"]));
    }

    #[test]
    fn looks_like_switch_case_rejects_three_or_more_words() {
        let source = "foo bar baz";
        let cmd = segment(source);
        assert!(!looks_like_switch_case(&cmd, &[]));
    }

    #[test]
    fn looks_like_switch_case_rejects_single_word() {
        let source = "foo";
        let cmd = segment(source);
        assert!(!looks_like_switch_case(&cmd, &[]));
    }

    // -- C41e5: missing-open-brace recovery --------------------------

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
            .find(|d| d.code == "E101")
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
        assert!(!a.result.diagnostics.iter().any(|d| d.code == "E101"));
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
    fn recover_missing_open_brace_skips_non_switch() {
        let source = "set x 1";
        let mut a = analyser_with_source(source);
        let commands: Vec<SegmentedCommand> = segment_commands_with_offset(source, 0);
        let mut cmd = commands[0].clone();
        let consumed = a.recover_missing_open_brace(&mut cmd, &commands, 0);
        assert_eq!(consumed, 0);
    }

    // -- C41e5: stolen-close-brace detection -------------------------

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
            .find(|d| d.code == "E103")
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
    fn emit_partial_command_diagnostic_emits_e200_for_unclosed_brace() {
        // Build a fake partial command via segment (a really
        // unclosed brace at end of source).
        let source = "{ unclosed";
        let mut a = analyser_with_source(source);
        let commands: Vec<SegmentedCommand> = segment_commands_with_offset(source, 0);
        let cmd = &commands[0];
        a.emit_partial_command_diagnostic(cmd);
        assert!(a.result.diagnostics.iter().any(|d| d.code == "E200"));
    }
}
