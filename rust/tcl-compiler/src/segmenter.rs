//! Command segmentation for Tcl token streams.
//!
//! Splits a flat token stream into per-command structures at EOL
//! boundaries. Both the analyser and lowerer consume these structures
//! instead of running their own token-iteration loops.
//!
//! Ports `core/parsing/command_segmenter.py`.

use tcl_lexer::{Lexer, LexerConfig, SourceMap, Span, Token, TokenType};

/// A single Tcl command parsed from the token stream.
#[derive(Debug, Clone)]
pub struct SegmentedCommand {
    /// Byte span covering the whole command.
    pub span: Span,
    /// Per-word representative tokens (one per argv entry).
    pub argv: Vec<Token>,
    /// Per-word reconstructed text.
    pub texts: Vec<String>,
    /// Whether each word is a single token.
    pub single_token_word: Vec<bool>,
    /// All tokens in the command (including separators).
    pub all_tokens: Vec<Token>,
    /// Whether the command is incomplete (unclosed delimiter).
    pub is_partial: bool,
    /// `{*}` expansion markers per word, if any word uses expansion.
    pub expand_word: Option<Vec<bool>>,
    /// Concatenated text of comment line(s) immediately preceding
    /// the command (without the leading ``#`` and trim-leading-
    /// whitespace per Python's `command_segmenter.py`); ``None``
    /// when no comment precedes.  Used by `_handle_proc` /
    /// `_handle_oo_class_command` for `ProcDef.doc` / `ClassDef.doc`.
    pub preceding_comment: Option<String>,
}

impl SegmentedCommand {
    /// Command name (first word).
    #[must_use]
    pub fn name(&self) -> &str {
        self.texts.first().map_or("", String::as_str)
    }

    /// Arguments (words after the command name).
    #[must_use]
    pub fn args(&self) -> &[String] {
        if self.texts.len() > 1 {
            &self.texts[1..]
        } else {
            &[]
        }
    }

    /// Per-arg representative tokens.
    #[must_use]
    pub fn arg_tokens(&self) -> &[Token] {
        if self.argv.len() > 1 {
            &self.argv[1..]
        } else {
            &[]
        }
    }

    /// Per-arg single-token flags.
    #[must_use]
    pub fn arg_single_token(&self) -> &[bool] {
        if self.single_token_word.len() > 1 {
            &self.single_token_word[1..]
        } else {
            &[]
        }
    }

    /// Return a copy of `self` with every span shifted by
    /// `base_offset`. Used by
    /// [`segment_commands_with_offset`] to relocate a body
    /// script's spans into the outer source buffer's offset
    /// space.
    #[must_use]
    pub fn shifted_by(mut self, base_offset: u32) -> Self {
        self.span = shift_span(self.span, base_offset);
        for tok in &mut self.argv {
            tok.span = shift_span(tok.span, base_offset);
        }
        for tok in &mut self.all_tokens {
            tok.span = shift_span(tok.span, base_offset);
        }
        self
    }
}

fn shift_span(span: Span, by: u32) -> Span {
    Span::new(span.start() + by, span.end() + by)
}

/// Return the source-level text fragment for a single token.
///
/// Variables are prefixed with `$` and command substitutions are
/// wrapped in `[...]` so that the result mirrors what the user wrote.
#[must_use]
pub fn word_piece(sm: &SourceMap<'_>, tok: Token) -> String {
    let text = sm.token_text(tok);
    match tok.kind {
        TokenType::Var => {
            if text.contains('}') {
                format!("${text}")
            } else {
                format!("${{{text}}}")
            }
        }
        TokenType::Cmd => format!("[{text}]"),
        _ => text.to_owned(),
    }
}

/// Compute the span covering all tokens.
fn span_from_tokens(tokens: &[Token]) -> Span {
    if tokens.is_empty() {
        return Span::new(0, 0);
    }
    let start = tokens.first().unwrap().span.start();
    let end = tokens.last().unwrap().span.end();
    Span::new(start, end)
}

/// Segment a token stream into per-command structures at EOL boundaries.
///
/// The core segmentation loop. Use
/// [`segment_commands_with_recovery`] when a known-commands set is
/// available and the caller wants Python-parity error recovery
/// (scanning past unclosed `{` / `[` / `"` for the next known
/// command name).
#[must_use]
pub fn segment_commands(source: &str) -> Vec<SegmentedCommand> {
    segment_commands_with_offset(source, 0)
}

/// Segment with a base byte offset (for body scripts inside braces).
///
/// The lexer tokenises `source` starting at local offset `0`.
/// Segmentation runs in local-offset space so the `SourceMap`
/// can slice text via [`Token::span`]; immediately before
/// returning, every `SegmentedCommand` has its spans relocated
/// by `base_offset` so downstream IR / optimiser / def-use
/// consumers see absolute offsets into the outer source buffer.
#[must_use]
pub fn segment_commands_with_offset(source: &str, base_offset: u32) -> Vec<SegmentedCommand> {
    let commands = segment_commands_local(source);
    if base_offset == 0 {
        return commands;
    }
    commands
        .into_iter()
        .map(|c| c.shifted_by(base_offset))
        .collect()
}

/// Segment with Python-parity error recovery.
///
/// Mirrors `segment_commands(source, recovery=True)` in
/// `core/parsing/command_segmenter.py`. After the raw
/// segmentation, the last command is inspected for a token that
/// looks like an unclosed delimiter (a `Str` / `Cmd` / `Esc` token
/// reaching EOF; for `Str` and `Esc` also requiring a line span of
/// at least three lines to avoid mistaking legitimate multi-line
/// strings for run-aways). If found and the suspicious token's
/// inner text contains a line whose first word is in
/// `known_commands`, the broken command is marked
/// `is_partial = true` and the source is re-segmented from the
/// recovery point; the recovered commands are appended.
///
/// Recovery is intended for top-level segmentation only — body
/// scripts (inside `{…}` blocks) should call [`segment_commands`]
/// or [`segment_commands_with_offset`] without recovery to avoid
/// false positives on legitimate multi-line braced strings.
#[must_use]
pub fn segment_commands_with_recovery<S>(
    source: &str,
    known_commands: &std::collections::HashSet<&str, S>,
) -> Vec<SegmentedCommand>
where
    S: std::hash::BuildHasher,
{
    let mut commands = segment_commands_local(source);
    let Some(last_cmd) = commands.last_mut() else {
        return commands;
    };
    let Some(suspicious_tok) = find_suspicious_token(source, &last_cmd.all_tokens) else {
        return commands;
    };
    // Strip the opening delimiter (`{` / `[` / `"`) so the inner
    // text + offset arithmetic matches the Python reference, whose
    // `Token.text` excludes the delimiter.
    let content_start =
        suspicious_tok.span.start() as usize + suspicious_tok.content_offset as usize;
    let content_end = suspicious_tok.span.end() as usize;
    let token_text = &source[content_start..content_end];
    let Some(recovery_offset) = find_recovery_offset(token_text, content_start, known_commands)
    else {
        return commands;
    };
    last_cmd.is_partial = true;
    // Re-segment the slice starting at the recovery point. The
    // recovered slice's spans are relative to the slice; shift them
    // back into the outer source buffer's offset space.
    let remaining = &source[recovery_offset..];
    if !remaining.trim().is_empty() {
        let base = u32::try_from(recovery_offset).expect("recovery_offset fits in u32");
        let recovered = segment_commands_with_offset(remaining, base);
        commands.extend(recovered);
    }
    commands
}

/// Minimum line span for a `Str` / `Esc` token to be treated as a
/// run-away unclosed delimiter rather than a legitimate multi-line
/// literal. Tuned (in the Python implementation) to avoid false
/// positives; mirrored here verbatim.
const RECOVERY_LINE_THRESHOLD: usize = 3;

/// Return the first token in `tokens` that looks like an unclosed
/// delimiter (`{` / `[` / `"`).
///
/// A `Cmd` token reaching EOF is always suspicious — a valid
/// `[…]` always closes — so no line-span threshold applies. `Str`
/// (`{…}`) and `Esc` (best-effort marker for unclosed `"…"` runs)
/// must also span at least [`RECOVERY_LINE_THRESHOLD`] lines.
fn find_suspicious_token(source: &str, tokens: &[Token]) -> Option<Token> {
    let source_len = source.len();
    for &tok in tokens {
        let is_brace_or_quote = matches!(tok.kind, TokenType::Str | TokenType::Esc);
        let is_bracket = matches!(tok.kind, TokenType::Cmd);
        if !is_brace_or_quote && !is_bracket {
            continue;
        }
        // Token must reach EOF — properly closed delimiters end
        // before EOF. `Span::end()` is exclusive, so a token
        // reaches EOF only when ``end == source_len``. The
        // previous ``end + 1 < source_len`` check incorrectly
        // accepted tokens ending at ``source_len - 1`` (e.g. a
        // closed multi-line ``{...}`` followed by a trailing
        // newline), which spuriously triggered recovery on
        // valid input — most visibly for ``Cmd`` tokens, which
        // have no line-span check to filter the false positive.
        let end = tok.span.end() as usize;
        if end < source_len {
            continue;
        }
        if is_bracket {
            return Some(tok);
        }
        // Brace / quote: also require line span ≥ threshold.
        let token_text = &source[tok.span.as_range()];
        let line_span = token_text.bytes().filter(|&b| b == b'\n').count();
        if line_span >= RECOVERY_LINE_THRESHOLD {
            return Some(tok);
        }
    }
    None
}

/// Find a byte offset in the original source where parsing can
/// resume after an unclosed delimiter.
///
/// Scans the inner text of a suspiciously large token line by
/// line, skipping line 0 (which is part of the broken command).
/// Returns the source offset of the first non-blank line whose
/// leading word is a known command name, or `None` if no recovery
/// point is found.
///
/// `token_text` is the content of the suspicious token with its
/// opening delimiter stripped (`{` / `[` / `"`); `content_start`
/// is the source offset of `token_text`'s first byte. Both follow
/// the Python `_find_recovery_offset` contract.
fn find_recovery_offset<S>(
    token_text: &str,
    content_start: usize,
    known_commands: &std::collections::HashSet<&str, S>,
) -> Option<usize>
where
    S: std::hash::BuildHasher,
{
    let mut inner_offset: usize = 0;
    for (i, line) in token_text.split('\n').enumerate() {
        if i == 0 {
            // Skip the first line — part of the broken command.
            inner_offset += line.len() + 1;
            continue;
        }
        let stripped = line.trim_start_matches([' ', '\t']);
        if !stripped.is_empty() {
            // First word: up to whitespace, ';', '{', or '['.
            let word_end = stripped
                .find([' ', '\t', '\n', '\r', ';', '{', '['])
                .unwrap_or(stripped.len());
            let first_word = &stripped[..word_end];
            if known_commands.contains(first_word) {
                let leading_ws = line.len() - stripped.len();
                return Some(content_start + inner_offset + leading_ws);
            }
        }
        inner_offset += line.len() + 1;
    }
    None
}

fn segment_commands_local(source: &str) -> Vec<SegmentedCommand> {
    // word_piece only needs token_text (source indexing), no base offset.
    let sm = SourceMap::new(source);
    let config = LexerConfig::default();
    let lexer_sm = SourceMap::new(source);
    let lexer = Lexer::with_source_map(lexer_sm, config);
    let Ok(tokens) = lexer.tokenise_all() else {
        return Vec::new();
    };

    let mut commands = Vec::new();
    let mut argv: Vec<Token> = Vec::new();
    let mut texts: Vec<String> = Vec::new();
    let mut single: Vec<bool> = Vec::new();
    let mut expand: Vec<bool> = Vec::new();
    let mut all_tokens: Vec<Token> = Vec::new();
    let mut prev_type = TokenType::Eol;
    let mut next_expand = false;
    let mut has_expand = false;
    // Accumulator for ``# ...`` lines preceding the next command.
    // Mirrors `last_comment` in
    // ``core/parsing/command_segmenter.py``: each new comment line
    // is appended (joined by ``\n``); the leading ``#`` and a
    // single space are stripped per Python's
    // ``line.lstrip('#').strip()`` shape.  Reset to ``None`` once a
    // command consumes it, or whenever a non-comment / non-EOL
    // token interrupts the run.
    let mut last_comment: Option<String> = None;

    for &tok in &tokens {
        match tok.kind {
            TokenType::Comment => {
                let raw = sm.token_text(tok);
                let line = raw.trim_start_matches('#').trim().to_string();
                last_comment = Some(match last_comment.take() {
                    Some(prev) => format!("{prev}\n{line}"),
                    None => line,
                });
                continue;
            }

            TokenType::Sep => {
                prev_type = tok.kind;
                continue;
            }

            // Backslash-newline continuation between words is whitespace.
            TokenType::Esc if sm.token_text(tok) == "\\\n" => {
                prev_type = TokenType::Sep;
                continue;
            }

            TokenType::Expand => {
                all_tokens.push(tok);
                next_expand = true;
                has_expand = true;
                prev_type = TokenType::Sep;
                continue;
            }

            TokenType::Eol | TokenType::Eof => {
                if !argv.is_empty() {
                    commands.push(SegmentedCommand {
                        span: span_from_tokens(&all_tokens),
                        argv: std::mem::take(&mut argv),
                        texts: std::mem::take(&mut texts),
                        single_token_word: std::mem::take(&mut single),
                        all_tokens: std::mem::take(&mut all_tokens),
                        is_partial: false,
                        expand_word: if has_expand {
                            Some(std::mem::take(&mut expand))
                        } else {
                            expand.clear();
                            None
                        },
                        preceding_comment: last_comment.take(),
                    });
                } else if matches!(tok.kind, TokenType::Eol) {
                    // Blank-line EOL: clear any accumulated
                    // preceding-comment state so a comment block
                    // separated from its following declaration
                    // by a blank line doesn't get attached to
                    // that declaration.  Mirrors Python's
                    // ``command_segmenter.py`` lines 257-261:
                    // ``elif tok.text.count("\n") > 1:
                    //         last_comment = None``.  An EOL
                    // token whose text contains more than one
                    // ``\n`` is the lexer's encoding of a
                    // run-of-empty-lines.  Class docs (now
                    // sourced from Rust on the merged result)
                    // are the most visible affected case.
                    let eol_text = sm.token_text(tok);
                    if eol_text.bytes().filter(|&b| b == b'\n').count() > 1 {
                        last_comment = None;
                    }
                }
                has_expand = false;
                next_expand = false;
                prev_type = tok.kind;
                continue;
            }

            _ => {}
        }

        all_tokens.push(tok);
        let piece = word_piece(&sm, tok);

        if prev_type == TokenType::Sep || prev_type == TokenType::Eol {
            argv.push(tok);
            texts.push(piece);
            single.push(true);
            expand.push(next_expand);
            next_expand = false;
        } else if let Some(last_text) = texts.last_mut() {
            last_text.push_str(&piece);
            if let Some(s) = single.last_mut() {
                *s = false;
            }
            // Widen argv[-1] to cover the full Tcl word, matching
            // Python's command_segmenter so multi-token words like
            // ``$var/literal.tcl`` carry one argv token whose span
            // ends at the last sub-token.
            if let Some(prev) = argv.last_mut() {
                prev.span = Span::new(prev.span.start(), tok.span.end());
            }
        } else {
            argv.push(tok);
            texts.push(piece);
            single.push(true);
            expand.push(next_expand);
            next_expand = false;
        }

        prev_type = tok.kind;
    }

    // Trailing command without final EOL.
    if !argv.is_empty() {
        commands.push(SegmentedCommand {
            span: span_from_tokens(&all_tokens),
            argv,
            texts,
            single_token_word: single,
            all_tokens,
            is_partial: false,
            expand_word: if has_expand { Some(expand) } else { None },
            preceding_comment: last_comment.take(),
        });
    }

    commands
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_source() {
        assert!(segment_commands("").is_empty());
    }

    #[test]
    fn single_command() {
        let cmds = segment_commands("puts hello");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name(), "puts");
        assert_eq!(cmds[0].args(), &["hello"]);
    }

    #[test]
    fn two_commands() {
        let cmds = segment_commands("set x 1\nputs $x");
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].name(), "set");
        assert_eq!(cmds[0].texts.len(), 3); // set, x, 1
        assert_eq!(cmds[1].name(), "puts");
    }

    #[test]
    fn semicolon_separator() {
        let cmds = segment_commands("set x 1; set y 2");
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn variable_word() {
        let cmds = segment_commands("puts $name");
        assert_eq!(cmds.len(), 1);
        // Variable should be wrapped as ${name}.
        assert_eq!(cmds[0].texts[1], "${name}");
    }

    #[test]
    fn command_substitution() {
        let cmds = segment_commands("puts [expr 1+2]");
        assert_eq!(cmds.len(), 1);
        // Command substitution wrapped in brackets.
        assert!(cmds[0].texts[1].starts_with('['));
        assert!(cmds[0].texts[1].ends_with(']'));
    }

    #[test]
    fn braced_string() {
        let cmds = segment_commands("if {$x > 0} {puts yes}");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name(), "if");
        assert_eq!(cmds[0].texts.len(), 3);
    }

    #[test]
    fn single_token_tracking() {
        let cmds = segment_commands("set x {hello world}");
        assert_eq!(cmds.len(), 1);
        // "set" is single, "x" is single, "{hello world}" is single.
        assert!(cmds[0].single_token_word.iter().all(|&s| s));
    }

    #[test]
    fn multi_token_word() {
        let cmds = segment_commands("puts $a$b");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].texts.len(), 2); // "puts", "${a}${b}"
        assert!(!cmds[0].single_token_word[1]); // multi-token word
    }

    #[test]
    fn multi_token_word_argv_spans_full_word() {
        // Mirrors the Python segmenter's argv-widening behaviour:
        // for a multi-token word the representative argv token's
        // span must cover the whole reconstructed word, not just
        // the first sub-token.
        let src = "source $script_dir/init.tcl";
        let cmds = segment_commands(src);
        assert_eq!(cmds.len(), 1);
        let arg_span = cmds[0].argv[1].span;
        assert_eq!(
            &src[arg_span.as_range()],
            "$script_dir/init.tcl",
            "argv[1].span must cover the whole multi-token word",
        );
        assert!(!cmds[0].single_token_word[1]);
    }

    #[test]
    fn comment_ignored() {
        let cmds = segment_commands("# this is a comment\nputs hello");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name(), "puts");
    }

    #[test]
    fn arg_tokens_and_arg_single() {
        let cmds = segment_commands("set x 1");
        assert_eq!(cmds[0].arg_tokens().len(), 2); // x, 1
        assert_eq!(cmds[0].arg_single_token().len(), 2);
    }

    #[test]
    fn blank_lines_between_commands() {
        let cmds = segment_commands("set x 1\n\nset y 2");
        assert_eq!(cmds.len(), 2);
    }
}

#[cfg(test)]
mod recovery_tests {
    //! Tests for [`segment_commands_with_recovery`].
    //!
    //! Mirrors the Python `command_segmenter` behaviour ported in
    //! Seg2: when an unclosed `{` / `[` / `"` causes the lexer to
    //! consume the rest of the file as one giant token, the
    //! segmenter scans the inner text for a line whose first word
    //! is a known command, marks the broken command `is_partial`,
    //! and re-segments from the recovery point so later commands
    //! still surface to consumers (workspace index, signature
    //! scan).

    use super::*;
    use std::collections::HashSet;

    fn known<const N: usize>(names: [&'static str; N]) -> HashSet<&'static str> {
        HashSet::from(names)
    }

    #[test]
    fn unclosed_brace_recovers_at_known_command() {
        // The Python segmenter recovers from an unclosed brace by
        // scanning forward for the next line whose first word is in
        // the known-commands set. Without recovery the Rust
        // segmenter would yield a single command — `proc early` —
        // whose body STR token swallows everything to EOF.
        let src = "proc early {} {\n    # missing close brace\n\nproc late {} {}\n";
        let cmds = segment_commands_with_recovery(src, &known(["proc"]));
        assert_eq!(cmds.len(), 2, "expected partial + recovered, got {cmds:?}");
        assert!(cmds[0].is_partial, "first command should be marked partial");
        assert_eq!(cmds[1].name(), "proc");
        // Verify the recovered command's argv span points at the
        // *outer* source buffer's offset of `proc late`, not the
        // local-to-slice offset.
        let argv0 = cmds[1].argv[0];
        assert_eq!(&src[argv0.span.as_range()], "proc");
    }

    #[test]
    fn unclosed_brace_without_known_command_keeps_swallowed_input() {
        // No recovery point reachable — the broken command stays
        // as the only command and is *not* marked partial (recovery
        // never fires).
        let src = "proc early {} {\n    line one\n    line two\n";
        let cmds = segment_commands_with_recovery(src, &known(["proc"]));
        assert_eq!(cmds.len(), 1);
        assert!(!cmds[0].is_partial);
    }

    #[test]
    fn unclosed_brace_below_line_threshold_skipped() {
        // Multi-line braced literals shorter than the line
        // threshold (3 lines) are NOT treated as suspicious — they
        // are likely legitimate string content.
        let src = "set x {\n  line two\n";
        let cmds = segment_commands_with_recovery(src, &known(["proc", "set"]));
        // Recovery does not fire; we still get the one (over-large)
        // command without a partial split.
        assert_eq!(cmds.len(), 1);
        assert!(!cmds[0].is_partial);
    }

    #[test]
    fn unclosed_bracket_recovers_without_line_threshold() {
        // Unclosed `[` is always suspicious (no line span check) —
        // a valid `[…]` always closes. Note: braced empty bodies
        // `{}` round-trip through `word_piece` as empty strings
        // because `SourceMap::token_text` strips the opening `{`,
        // leaving the brace's content (empty) behind.
        let src = "set x [foo\nproc late {} {}\n";
        let cmds = segment_commands_with_recovery(src, &known(["proc"]));
        assert!(
            cmds.len() >= 2,
            "expected partial + recovered, got {cmds:?}"
        );
        let recovered = cmds.last().unwrap();
        assert_eq!(recovered.name(), "proc");
        assert_eq!(recovered.args(), ["late", "", ""]);
    }

    #[test]
    fn empty_known_commands_set_never_recovers() {
        let src = "proc early {} {\n    # missing close brace\n\nproc late {} {}\n";
        let cmds = segment_commands_with_recovery(src, &HashSet::new());
        assert_eq!(cmds.len(), 1);
        assert!(!cmds[0].is_partial);
    }

    #[test]
    fn closed_input_is_unaffected_by_recovery() {
        // Recovery must not perturb well-formed inputs — same
        // commands as `segment_commands(src)` would produce.
        let src = "proc foo {} {}\nproc bar {} {}";
        let kc = known(["proc"]);
        let recovered = segment_commands_with_recovery(src, &kc);
        let raw = segment_commands(src);
        assert_eq!(recovered.len(), raw.len());
        for (r, p) in recovered.iter().zip(raw.iter()) {
            assert_eq!(r.name(), p.name());
            assert_eq!(r.args(), p.args());
            assert_eq!(r.is_partial, p.is_partial);
        }
    }

    #[test]
    fn closed_command_sub_followed_by_trailing_newline_not_suspicious() {
        // Regression for the EOF off-by-one: ``Span::end()`` is
        // exclusive, so a closed ``[cmd]`` whose ``]`` sits at
        // ``source_len - 2`` (followed by a single ``\n``) ends
        // at ``source_len - 1``. The previous ``end + 1 < source_len``
        // check incorrectly accepted such tokens as "reaches EOF",
        // which spuriously triggered recovery on ``Cmd`` tokens
        // (no line-span check) when their inner text contained a
        // known command name like ``proc``.
        //
        // After the fix: only ``end == source_len`` qualifies as
        // EOF, so this input segments cleanly without recovery.
        let src = "set x [proc foo {} {}]\n";
        let kc = known(["proc"]);
        let cmds = segment_commands_with_recovery(src, &kc);
        // Should be exactly one command (`set x [...]`); no
        // partial split, no spurious extra recovered commands.
        assert_eq!(cmds.len(), 1, "expected one command, got {cmds:?}");
        assert_eq!(cmds[0].name(), "set");
        assert!(!cmds[0].is_partial);
    }

    #[test]
    fn closed_multiline_brace_followed_by_trailing_newline_not_suspicious() {
        // Same regression but for the ``Str`` (brace) path. A
        // closed multi-line braced string spanning more than the
        // ``RECOVERY_LINE_THRESHOLD`` lines, with a trailing
        // newline, must not be treated as suspicious.
        let src = "set x {\n  line1\n  line2\n  line3\n}\n";
        let kc = known(["set"]);
        let cmds = segment_commands_with_recovery(src, &kc);
        assert_eq!(cmds.len(), 1, "expected one command, got {cmds:?}");
        assert_eq!(cmds[0].name(), "set");
        assert!(!cmds[0].is_partial);
    }

    #[test]
    fn recovery_picks_first_matching_line_not_later_ones() {
        // Two `set` commands appear in the broken body. The
        // segmenter must recover at the first match — the line
        // starting with `set y 1` — not skip ahead to the later
        // `set z 2`. The recovered slice is then re-segmented on
        // its own, picking up `foo bar` and the second `set` too.
        let src = "proc early {} {\n    set y 1\nfoo bar\nset z 2\n";
        let cmds = segment_commands_with_recovery(src, &known(["set"]));
        assert!(cmds.len() >= 2);
        assert!(cmds[0].is_partial);
        // First recovered command points at `set y 1`, not `set z 2`.
        assert_eq!(cmds[1].name(), "set");
        assert_eq!(cmds[1].args(), ["y", "1"]);
        // The later `set z 2` command is also reachable in the
        // recovered slice.
        let last_set = cmds
            .iter()
            .rev()
            .find(|c| c.name() == "set")
            .expect("at least one set");
        assert_eq!(last_set.args(), ["z", "2"]);
    }

    #[test]
    fn recovery_skips_indented_lines_and_finds_unindented_match() {
        // Recovery scans for the first line whose first word is
        // known, regardless of indentation — the line lookup
        // strips leading whitespace before the word match.
        let src = "proc early {} {\n    not_known foo\n    proc indented {} {}\n";
        let cmds = segment_commands_with_recovery(src, &known(["proc"]));
        assert_eq!(cmds.len(), 2);
        assert!(cmds[0].is_partial);
        assert_eq!(cmds[1].name(), "proc");
        // `{}` empty bodies become empty strings in `texts` (the
        // segmenter strips the opening delimiter before emitting
        // the word piece).
        assert_eq!(cmds[1].args(), ["indented", "", ""]);
    }

    #[test]
    fn recovery_from_offset_into_source_uses_absolute_offsets() {
        // The recovered slice must use absolute (outer-source)
        // offsets so downstream consumers (signature_scan, the
        // workspace index) can slice the recovered token text from
        // the original buffer without further bookkeeping.
        let src = "proc early {} {\n    # broken\n\nproc late {} {}\n";
        let cmds = segment_commands_with_recovery(src, &known(["proc"]));
        let late = cmds.iter().find(|c| !c.is_partial).expect("recovered cmd");
        let argv0 = late.argv[0];
        assert_eq!(&src[argv0.span.as_range()], "proc");
        // Confirm the recovered command's argv tokens span the
        // outer-source bytes (the `late` name token text matches
        // the source byte range).
        let name_tok = late.argv[1];
        assert_eq!(&src[name_tok.span.as_range()], "late");
    }
}

#[cfg(test)]
mod span_absolute_tests {
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    #[test]
    fn proc_body_statement_spans_are_absolute() {
        let src = "proc ::f {} { set x 1; return $x }";
        let r = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(src, &r, false);
        let proc = cu.ir_module.procedures.get("::f").expect("proc");
        let first = proc.body.statements.first().expect("body stmt");
        let span = first.span();
        let text = &src[span.as_range()];
        assert!(
            text.starts_with("set"),
            "expected absolute span pointing at `set`, got {text:?}",
        );
    }
}
