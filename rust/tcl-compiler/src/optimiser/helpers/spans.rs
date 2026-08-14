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

//! Span-extension helpers for producing valid rewrite targets.
//!
//! The lexer reports representative-token spans for `${name}` /
//! `[cmd …]` words without their closing `}` / `]`. Similarly,
//! IR statement spans computed from the underlying tokens inherit
//! the same truncation — so a statement like
//! `return [gcd $b [expr {$a % $b}]]` lowers to a `Statement::Return`
//! whose span stops at the inner `}` and misses the two trailing
//! `]]`.
//!
//! Passes that emit *applicable* rewrites against these spans need
//! to target the full word / statement extent; otherwise the
//! rewrite leaves orphan closing delimiters in the output.
//!
//! This module owns the *multi-word* / statement-level extension
//! helpers used by the optimiser passes. Centralising them here keeps
//! the fix consistent across `propagation`, `tail_call`,
//! `structure_elimination`, etc.
//!
//! Widening a **single** word to its own closing delimiter is not owned
//! here: that is [`tcl_lexer::word_span_at`] (the token-free sibling of
//! `tcl_lexer::word_span`), which the passes call directly. This module
//! used to carry a `full_word_span` byte-counter of its own — a second
//! implementation under the same name as the analyser's correct
//! delegate, and one that recognised only `[…]` and `${…}` (issue
//! #1423).

use tcl_lexer::Span;

/// Extend a span through trailing closing delimiters (`]`, `}`,
/// `"`) whose matching openers lie within the covered text.
///
/// The algorithm walks the covered bytes tracking `[`/`]` and
/// `{`/`}` depth and `"…"` state (with `\` escapes skipped). If
/// any opener is unmatched at the end of the span, the span end
/// advances past the matching closer(s) immediately following.
/// Stops at the first non-matching byte so unrelated source
/// characters are never included.
///
/// Idempotent: a span that is already balanced is returned
/// unchanged. Safe against EOF / oversized spans — returns the
/// input span untouched rather than panicking.
#[must_use]
pub fn full_rewrite_span(source: &str, span: Span) -> Span {
    let bytes = source.as_bytes();
    let start = span.start() as usize;
    let mut end = span.end() as usize;
    if start > bytes.len() || end > bytes.len() || start > end {
        return span;
    }

    // Count imbalance within the covered text.
    let (mut bracket_depth, mut brace_depth, mut in_quote) = (0i32, 0i32, false);
    let mut i = start;
    while i < end {
        let b = bytes[i];
        if b == b'\\' && i + 1 < end {
            i += 2;
            continue;
        }
        match b {
            b'[' if !in_quote => bracket_depth += 1,
            b']' if !in_quote && bracket_depth > 0 => bracket_depth -= 1,
            b'{' if !in_quote => brace_depth += 1,
            b'}' if !in_quote && brace_depth > 0 => brace_depth -= 1,
            b'"' => in_quote = !in_quote,
            _ => {}
        }
        i += 1;
    }

    // Advance through matching closers immediately following.
    while (bracket_depth > 0 || brace_depth > 0 || in_quote) && end < bytes.len() {
        let b = bytes[end];
        match b {
            b']' if !in_quote && bracket_depth > 0 => {
                bracket_depth -= 1;
                end += 1;
            }
            b'}' if !in_quote && brace_depth > 0 => {
                brace_depth -= 1;
                end += 1;
            }
            b'"' if in_quote => {
                in_quote = false;
                end += 1;
            }
            _ => break,
        }
    }

    Span::new(span.start(), u32::try_from(end).unwrap_or(span.end()))
}

/// Compute the deletion range for a statement being removed, swallowing
/// the trailing run of whitespace plus one statement separator (`\n` /
/// `;`) so the surviving text closes up cleanly.
///
/// `cmd_span` is the full command span (exclusive end); `next_start` is
/// the byte offset of the next statement, or `None` at end-of-script (in
/// which case the command span is returned unchanged).
#[must_use]
pub fn statement_delete_rewrite_range(
    source: &str,
    cmd_span: Span,
    next_start: Option<usize>,
) -> Span {
    let Some(next) = next_start else {
        return cmd_span;
    };
    let end = cmd_span.end() as usize;
    if next <= end || next > source.len() {
        return cmd_span;
    }
    let bytes = source.as_bytes();
    let mut cursor = end;
    while cursor < next && matches!(bytes[cursor], b' ' | b'\t' | b'\r') {
        cursor += 1;
    }
    if cursor < next && matches!(bytes[cursor], b'\n' | b';') {
        cursor += 1;
        while cursor < next && matches!(bytes[cursor], b' ' | b'\t' | b'\r') {
            cursor += 1;
        }
        if cursor > end
            && let Ok(new_end) = u32::try_from(cursor)
        {
            return Span::new(cmd_span.start(), new_end);
        }
    }
    cmd_span
}

/// Extend an argv span that points at a `"…"` composite word to
/// cover the full quoted string.
///
/// A composite word like `"$a $b $c"` segments into many tokens
/// and `CommandTokens::argv[i]` holds only the representative
/// token (the opening `"`). To rewrite the *whole* string we need
/// the span to reach the closing quote — this is the span assembly
/// around [`tcl_lexer::close_quote_offset`], the shared scanner that
/// skips `\`-escapes and whole `[…]` command substitutions.
///
/// Returns the input span unchanged when the first byte isn't `"` or
/// no close quote is found, so a rewrite anchored on the result either
/// covers the whole string or does not fire. Scanning without the
/// command-substitution rule stopped `"a[foo "b"]c"` at the quote
/// opening the inner `"b"`, and O129's auto-fix then replaced that
/// truncated prefix and left `]c"` behind (issue #1424).
#[must_use]
pub fn full_quoted_string_span(source: &str, argv_span: Span) -> Span {
    let Some(close) = tcl_lexer::close_quote_offset(source, argv_span.start() as usize) else {
        return argv_span;
    };
    let Ok(end) = u32::try_from(close + 1) else {
        return argv_span;
    };
    Span::new(argv_span.start(), end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_rewrite_span_balanced_span_unchanged() {
        // `[cmd]` balanced → no extension.
        let source = "foo [cmd] bar";
        assert_eq!(full_rewrite_span(source, Span::new(4, 9)), Span::new(4, 9));
    }

    #[test]
    fn full_rewrite_span_single_unclosed_bracket() {
        // `[cmd` missing the `]` → extend by 1.
        let source = "foo [cmd] bar";
        assert_eq!(full_rewrite_span(source, Span::new(4, 8)), Span::new(4, 9));
    }

    #[test]
    fn full_rewrite_span_nested_missing_two_brackets() {
        // `return [gcd $b [expr {$a % $b}]]` — span covering
        // through `}` misses both trailing `]]`.
        let source = "return [gcd $b [expr {$a % $b}]]";
        let span = Span::new(0, 30);
        let extended = full_rewrite_span(source, span);
        assert_eq!(extended, Span::new(0, 32));
        assert_eq!(
            &source[extended.start() as usize..extended.end() as usize],
            "return [gcd $b [expr {$a % $b}]]",
        );
    }

    #[test]
    fn full_rewrite_span_unclosed_quoted_string() {
        // `"hi"` with span truncating before the closing quote.
        let source = "puts \"hi\"";
        assert_eq!(full_rewrite_span(source, Span::new(5, 8)), Span::new(5, 9),);
    }

    #[test]
    fn full_rewrite_span_ignores_brackets_inside_quotes() {
        // `"[x]"` — inside quotes, the `[` doesn't start a
        // CMD-subst so extension must not fire. Span covering
        // `"[x]` should extend only past the closing quote.
        let source = "\"[x]\"";
        assert_eq!(full_rewrite_span(source, Span::new(0, 4)), Span::new(0, 5),);
    }

    #[test]
    fn full_rewrite_span_respects_backslash_escape() {
        // `\[` is a literal `[` — doesn't open a subst.
        let source = "foo \\[ bar";
        assert_eq!(full_rewrite_span(source, Span::new(0, 6)), Span::new(0, 6));
    }

    #[test]
    fn full_rewrite_span_stops_at_non_closer() {
        // Extension only consumes matching closers, not arbitrary
        // following text.
        let source = "[cmd] extra";
        assert_eq!(full_rewrite_span(source, Span::new(0, 4)), Span::new(0, 5));
    }

    #[test]
    fn full_rewrite_span_at_eof_no_panic() {
        // Oversized span must not panic.
        assert_eq!(full_rewrite_span("hi", Span::new(0, 5)), Span::new(0, 5));
    }

    #[test]
    fn full_quoted_string_span_extends_through_close_quote() {
        let source = "puts \"$a $b $c\"";
        // Argv span points at just the opening `"` token (5..7
        // covering `"$`). Extension should reach the closing `"`.
        assert_eq!(
            full_quoted_string_span(source, Span::new(5, 7)),
            Span::new(5, 15),
        );
    }

    #[test]
    fn full_quoted_string_span_respects_escape() {
        // `"a\"b"` — the `\"` is an escape, not the close quote.
        let source = "\"a\\\"b\"";
        assert_eq!(
            full_quoted_string_span(source, Span::new(0, 1)),
            Span::new(0, 6),
        );
    }

    #[test]
    fn full_quoted_string_span_no_open_quote_unchanged() {
        let source = "plain";
        assert_eq!(
            full_quoted_string_span(source, Span::new(0, 3)),
            Span::new(0, 3),
        );
    }

    #[test]
    fn full_quoted_string_span_spans_quote_inside_command_substitution() {
        // Issue #1424: the `"b"` belongs to the substituted command, so
        // the span must reach the *final* `"` — stopping at the inner
        // quote would make O129's auto-fix replace `"a[foo "` and leave
        // `b"]c"` behind as a stray fragment.
        let source = "set x \"a[foo \"b\"]c\"";
        let span = full_quoted_string_span(source, Span::new(6, 8));
        assert_eq!(
            span,
            Span::new(6, u32::try_from(source.len()).unwrap()),
            "span must cover the whole quoted word",
        );
        assert_eq!(&source[span.as_range()], "\"a[foo \"b\"]c\"");
    }

    #[test]
    fn full_quoted_string_span_spans_a_comment_inside_a_substitution() {
        // The `]` in the command-position comment is inert in C Tcl, so the
        // word closes at the final `"`.  Stopping at the quote opening the
        // inner `"b"` would give O129 a truncated rewrite span and corrupt
        // otherwise valid source.
        let source = "set x \"a[\n# ] comment\nset y \"b\"\n]c\"";
        let span = full_quoted_string_span(source, Span::new(6, 8));
        assert_eq!(
            span,
            Span::new(6, u32::try_from(source.len()).unwrap()),
            "span must cover the whole multiline quoted word",
        );
    }

    #[test]
    fn full_quoted_string_span_unterminated_unchanged() {
        // No close quote at all — the span is returned untouched so no
        // rewrite anchors on a guessed offset.
        let source = "puts \"abc";
        assert_eq!(
            full_quoted_string_span(source, Span::new(5, 7)),
            Span::new(5, 7),
        );
    }

    #[test]
    fn full_rewrite_span_only_extends_as_many_closers_as_needed() {
        // Span has 1 unclosed `[` but source has 3 `]` after. We
        // should only consume 1.
        let source = "[cmd]]]";
        assert_eq!(full_rewrite_span(source, Span::new(0, 4)), Span::new(0, 5));
    }
}
