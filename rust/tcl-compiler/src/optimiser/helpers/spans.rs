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
//! This module owns the extension helpers used by the optimiser
//! passes. Centralising them here keeps the fix consistent across
//! `propagation`, `tail_call`, `structure_elimination`, etc.

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

/// Extend an argv span that points at a `"…"` composite word to
/// cover the full quoted string.
///
/// A composite word like `"$a $b $c"` segments into many tokens
/// and `CommandTokens::argv[i]` holds only the representative
/// token (the opening `"`). To rewrite the *whole* string we need
/// the span to reach the closing quote — this helper scans the
/// source forward from the argv start through `\"` escapes until
/// the matching close quote. Returns the input span unchanged
/// when the first byte isn't `"` or no close quote is found.
#[must_use]
pub fn full_quoted_string_span(source: &str, argv_span: Span) -> Span {
    let bytes = source.as_bytes();
    let start = argv_span.start() as usize;
    if start >= bytes.len() || bytes[start] != b'"' {
        return argv_span;
    }
    let mut i = start + 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            i += 2;
            continue;
        }
        if bytes[i] == b'"' {
            return Span::new(
                argv_span.start(),
                u32::try_from(i + 1).unwrap_or(argv_span.end()),
            );
        }
        i += 1;
    }
    argv_span
}

/// Simpler variant for single-word argv spans. The lexer reports
/// the representative token for `${name}` / `[cmd]` words without
/// the trailing `}` / `]`; this helper extends by exactly one
/// byte when the next source byte is the expected closer.
///
/// Used by passes that rewrite a single argv word (not a full
/// statement). Equivalent to [`full_rewrite_span`] on those shapes
/// but faster — no bracket/brace counting.
#[must_use]
pub fn full_word_span(source: &str, argv_span: Span) -> Span {
    let bytes = source.as_bytes();
    let start = argv_span.start() as usize;
    let end = argv_span.end() as usize;
    if start >= bytes.len() || end >= bytes.len() || start > end {
        return argv_span;
    }
    let Some(&first) = bytes.get(start) else {
        return argv_span;
    };
    if end == start {
        return argv_span;
    }
    let second = bytes.get(start + 1).copied();
    let next = bytes[end];
    let needs_extend =
        (first == b'[' && next == b']') || (first == b'$' && second == Some(b'{') && next == b'}');
    if needs_extend {
        Span::new(argv_span.start(), argv_span.end() + 1)
    } else {
        argv_span
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_word_span_bare_var() {
        // `$x` — no extension.
        assert_eq!(full_word_span("$x", Span::new(0, 2)), Span::new(0, 2));
    }

    #[test]
    fn full_word_span_braced_var_extends_closing_brace() {
        // `${x}` — argv span 0..3 covers `${x`, extend to 0..4.
        assert_eq!(full_word_span("${x}", Span::new(0, 3)), Span::new(0, 4));
    }

    #[test]
    fn full_word_span_cmd_subst_extends_closing_bracket() {
        // `[cmd]` — argv span 0..4 covers `[cmd`, extend to 0..5.
        assert_eq!(full_word_span("[cmd]", Span::new(0, 4)), Span::new(0, 5));
    }

    #[test]
    fn full_word_span_plain_identifier_no_extension() {
        assert_eq!(full_word_span("plain ", Span::new(0, 5)), Span::new(0, 5));
    }

    #[test]
    fn full_word_span_at_eof_no_extension() {
        assert_eq!(full_word_span("[x", Span::new(0, 2)), Span::new(0, 2));
    }

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
    fn full_rewrite_span_only_extends_as_many_closers_as_needed() {
        // Span has 1 unclosed `[` but source has 3 `]` after. We
        // should only consume 1.
        let source = "[cmd]]]";
        assert_eq!(full_rewrite_span(source, Span::new(0, 4)), Span::new(0, 5));
    }
}
