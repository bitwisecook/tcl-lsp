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

//! Shared Tcl value-shape helpers used across compiler passes.

use tcl_lexer::{Lexer, Span, TokenType};

/// Scan one Tcl variable reference starting at `text[at]`, returning the
/// byte index just past its end, or `None` when `text[at..]` does not
/// start with a valid reference.
///
/// Handles the three forms from the Tcl 9.0 `Tcl_ParseVar` spec:
///
/// - `$name` — bare name in `[A-Za-z0-9_:]`; `::` runs stay part of the
///   name (namespace-qualified).
/// - `${name}` — braced; any character except `}` (escapes inside the
///   braces are taken literally).
/// - `$name(index)` — array element. The index runs to the matching `)`,
///   with backslash-escaped close parens taken literally (so `$a(x\)y)`
///   is one reference whose index text is `x\)y`).
fn scan_pure_var_ref(text: &str, at: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.get(at) != Some(&b'$') {
        return None;
    }
    let mut i = at + 1;
    // Braced form `${...}`.
    if bytes.get(i) == Some(&b'{') {
        let mut j = i + 1;
        while j < bytes.len() && bytes[j] != b'}' {
            j += 1;
        }
        if j >= bytes.len() {
            return None;
        }
        return Some(j + 1);
    }
    // Bare name: alphanumeric + `_` + `:`.
    let name_start = i;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'_' | b':')) {
        i += 1;
    }
    if i == name_start {
        return None;
    }
    // Optional array index `(index)` — backslash-escape aware.
    if bytes.get(i) == Some(&b'(') {
        let mut j = i + 1;
        while j < bytes.len() {
            match bytes[j] {
                b'\\' if j + 1 < bytes.len() => j += 2, // skip escaped char
                b')' => return Some(j + 1),
                _ => j += 1,
            }
        }
        return None; // unterminated index
    }
    Some(i)
}

/// Return `true` when `text` is exactly one variable reference (`$x` /
/// `${x}` / `$ns::x` / `$arr(idx)`) with no surrounding or concatenated
/// syntax.
///
/// The check gates the uplevel/safe-eval idioms: `uplevel 1 $body` is
/// single-substitution-safe only when `$body` is one pure reference;
/// anything else double-substitutes the composed value.
#[must_use]
pub fn is_pure_var_ref(text: &str) -> bool {
    scan_pure_var_ref(text, 0) == Some(text.len())
}

/// Return `true` when `text` is a braced array-shaped var ref `${a(1)}`.
///
/// tclsh 9 loads such a reference by its whole literal name (`a(1)`
/// resolved at runtime as array element `a(1)`), so the value is a
/// *variable reference*, never a constant — analyses must treat it as
/// conservatively unknown (overdefined), the same as `${ns::y}`. A plain
/// braced scalar `${foo}` (no parens) is *not* matched.
#[must_use]
pub fn is_braced_whole_name_array_ref(text: &str) -> bool {
    let Some(inner) = text.strip_prefix("${").and_then(|s| s.strip_suffix('}')) else {
        return false;
    };
    inner.contains('(') && inner.ends_with(')') && !inner.contains('}')
}

/// Extract command name and args from `[cmd arg1 arg2 …]`.
///
/// Returns `None` when `text` is not bracket-wrapped, or when the
/// inside is empty. Arguments are split on whitespace at the *top* nesting
/// level only: `[...]`, `{...}` and `"..."` groups keep their inner
/// whitespace, so a nested command substitution like `[list [read $fd]]`
/// yields the single arg `[read $fd]` rather than the two broken halves
/// `[read` and `$fd]` (which would hide a taint source nested in an
/// argument). Callers that need full Tcl list quoting handle it upstream.
#[must_use]
pub fn parse_command_substitution(text: &str) -> Option<(String, Vec<String>)> {
    let (command, args) = parse_command_substitution_with_spans(text)?;
    Some((command, args.into_iter().map(|(text, _)| text).collect()))
}

/// Like [`parse_command_substitution`], but additionally returns each
/// argument's byte span *within `text`* (the caller adds its own base
/// offset to get an absolute source position) — for diagnostics that need
/// to anchor on the specific offending argument rather than the whole
/// substitution.
///
/// Uses the real [`Lexer`] rather than a hand-rolled bracket/brace
/// counter, so word boundaries follow Tcl's actual quoting rules (this
/// also makes it correct on inputs a naive counter gets wrong, e.g. a
/// `#` comment or an escaped delimiter at the top level). Returns `None`
/// for the same shapes as
/// [`parse_command_substitution`], plus when the bracket body doesn't lex
/// as a single command (an embedded `;`/newline followed by more words).
#[must_use]
pub fn parse_command_substitution_with_spans(text: &str) -> Option<(String, Vec<(String, Span)>)> {
    let leading_ws = u32::try_from(text.len() - text.trim_start().len()).unwrap_or(u32::MAX);
    let stripped = text.trim();
    let inner = stripped.strip_prefix('[')?.strip_suffix(']')?;
    // Offset of `inner`'s first byte within `text`: leading whitespace + `[`.
    let base = leading_ws + 1;

    let tokens = Lexer::new(inner).tokenise_all().ok()?;
    let mut words: Vec<(String, Span)> = Vec::new();
    let mut prev_kind = TokenType::Eol;
    let mut saw_terminator = false;

    for (idx, tok) in tokens.iter().enumerate() {
        match tok.kind {
            TokenType::Comment | TokenType::Sep => {
                prev_kind = tok.kind;
                continue;
            }
            TokenType::Eol => {
                if !words.is_empty() {
                    saw_terminator = true;
                }
                prev_kind = tok.kind;
                continue;
            }
            TokenType::Eof => break,
            _ => {}
        }
        if saw_terminator {
            return None;
        }
        let (word_text, tok_end) = widened_token_text(inner, tok)?;
        // Two closing delimiters the raw token stream can leave
        // unclaimed by any token's span, needing manual recovery here:
        //
        // - The `in_quote` closing `"` — unlike `Str`/`Cmd`, there is no
        //   dedicated token kind for a double-quoted word to attach
        //   `group_closer()` to, so a trailing `"` immediately after the
        //   last fragment of a quoted word (e.g. `"/api/*"`, or `"a[foo]"`
        //   right after the nested command substitution's own widened
        //   `]`) is silently dropped.
        // - A non-degenerate braced-var's closing `}` (`${p}`) — `Var`
        //   also covers the unbraced `$name` form, which has no closer at
        //   all, so `group_closer()` can't unconditionally claim `}` for
        //   the whole kind the way it does for `Str`/`Cmd`.
        //
        // Only claim the byte when the *next* token doesn't already start
        // there (e.g. `"a$b"` emits the closing `"` as its own adjacent
        // token — nothing orphaned to recover in that shape, and claiming
        // it here too would double it).
        let next_start = tokens.get(idx + 1).map(|t| t.span.start());
        let orphaned_closer = match inner.as_bytes().get(tok_end as usize) {
            Some(b'"') => true,
            Some(b'}') if tok.kind == TokenType::Var => true,
            _ => false,
        } && next_start != Some(tok_end);
        let (word_text, tok_end) = if orphaned_closer {
            let start = usize::try_from(tok.span.start()).ok()?;
            let widened_end = tok_end as usize + 1;
            (
                inner.get(start..widened_end)?.to_owned(),
                u32::try_from(widened_end).unwrap_or(u32::MAX),
            )
        } else {
            (word_text, tok_end)
        };
        let abs_span = Span::new(base + tok.span.start(), base + tok_end);
        if matches!(prev_kind, TokenType::Sep | TokenType::Eol) || words.is_empty() {
            words.push((word_text, abs_span));
        } else {
            // Adjacent tokens with no separator concatenate onto the
            // previous word (e.g. `$a$b` or `foo$x`).
            let (last_text, last_span) =
                words.last_mut().expect("words is non-empty in this branch");
            last_text.push_str(&word_text);
            *last_span = Span::new(last_span.start(), abs_span.end());
        }
        prev_kind = tok.kind;
    }

    let mut it = words.into_iter();
    let (cmd, _cmd_span) = it.next()?;
    Some((cmd, it.collect()))
}

/// Return `tok`'s text and exclusive end offset (both relative to `inner`),
/// widened to include a `{…}` / `[…]` word's closing delimiter when the raw
/// token span omits it.
///
/// The lexer follows an inner-end convention for `Str` (`{…}`) and `Cmd`
/// (`[…]`) tokens: a *non-empty* group's span ends one byte before its
/// closer (see `segmenter::widen_word_end`, the same convention this
/// mirrors). An *empty* group (`{}` / `[]`) is the exception — its span
/// already covers the closer. The discriminator used here is the extracted
/// text itself: if it doesn't already end with the closer, the group is
/// non-empty and one more byte is pulled in.
fn widened_token_text(inner: &str, tok: &tcl_lexer::Token) -> Option<(String, u32)> {
    let start = tok.span.start() as usize;
    let end = tok.span.end() as usize;
    let text = inner.get(start..end)?;
    let Some(closer) = tok.kind.group_closer() else {
        return Some((text.to_owned(), tok.span.end()));
    };
    if text.ends_with(closer) {
        return Some((text.to_owned(), tok.span.end()));
    }
    let widened_end = end + closer.len_utf8();
    let widened = inner.get(start..widened_end)?;
    Some((
        widened.to_owned(),
        u32::try_from(widened_end).unwrap_or(u32::MAX),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_var_bare_and_braced() {
        assert!(is_pure_var_ref("$x"));
        assert!(is_pure_var_ref("${x}"));
        assert!(is_pure_var_ref("$foo::bar"));
    }

    #[test]
    fn pure_var_rejects_extras() {
        assert!(!is_pure_var_ref("$x extra"));
        assert!(!is_pure_var_ref("hello$x"));
        assert!(!is_pure_var_ref("$x[nested]"));
        assert!(!is_pure_var_ref("\"$x\""));
        assert!(!is_pure_var_ref("x"));
    }

    #[test]
    fn pure_var_rejects_unbalanced_braces() {
        assert!(!is_pure_var_ref("${x}y"));
    }

    #[test]
    fn pure_var_accepts_array_index() {
        assert!(is_pure_var_ref("$arr(idx)"));
        assert!(is_pure_var_ref("$arr(key)"));
        assert!(is_pure_var_ref("${a(1)}"));
        // Backslash-escaped close paren stays inside the one index.
        assert!(is_pure_var_ref("$a(x\\)y)"));
    }

    #[test]
    fn fp_nab_12_escaped_paren_array_index_companions() {
        // FP-NAB-12: the hand-rolled parser consumes a backslash-escaped close
        // paren inside an
        // array index so the reference does not terminate at the first `)`.
        assert!(is_pure_var_ref(r"$a(x\)y)"));
        // Companion controls locking in overall parser correctness.
        assert!(is_pure_var_ref("$x"));
        assert!(is_pure_var_ref("${some name}"));
        assert!(is_pure_var_ref("$ns::x"));
        assert!(is_pure_var_ref("$arr(plain)"));
        // Inverse: an *unescaped* `)` terminates the index, leaving trailing
        // `y` as concatenated literal — NOT one pure var ref.
        assert!(!is_pure_var_ref("$a(x)y"));
    }

    #[test]
    fn pure_var_rejects_concatenation_and_trailing_text() {
        // Over-permissive cases the byte-set heuristic used to accept.
        assert!(!is_pure_var_ref("$x$y"));
        assert!(!is_pure_var_ref("$x.foo"));
        assert!(!is_pure_var_ref("$x_$y"));
        // Unterminated array index is not a pure reference.
        assert!(!is_pure_var_ref("$arr(idx"));
    }

    #[test]
    fn braced_whole_name_array_ref_detection() {
        assert!(is_braced_whole_name_array_ref("${a(1)}"));
        assert!(is_braced_whole_name_array_ref("${arr(key)}"));
        // Plain braced scalar is not an array-shaped ref.
        assert!(!is_braced_whole_name_array_ref("${foo}"));
        assert!(!is_braced_whole_name_array_ref("$arr(1)"));
        assert!(!is_braced_whole_name_array_ref("${a(1)}x"));
    }

    #[test]
    fn parse_command_substitution_basic() {
        let (cmd, args) = parse_command_substitution("[llength $x]").unwrap();
        assert_eq!(cmd, "llength");
        assert_eq!(args, vec!["$x".to_string()]);
    }

    #[test]
    fn parse_command_substitution_with_whitespace() {
        let (cmd, args) = parse_command_substitution("  [ set x 42 ]  ").unwrap();
        assert_eq!(cmd, "set");
        assert_eq!(args, vec!["x".to_string(), "42".into()]);
    }

    #[test]
    fn parse_command_substitution_rejects_non_bracketed() {
        assert!(parse_command_substitution("llength $x").is_none());
        assert!(parse_command_substitution("[empty ").is_none());
        assert!(parse_command_substitution("[]").is_none());
    }

    #[test]
    fn parse_command_substitution_no_args() {
        let (cmd, args) = parse_command_substitution("[pwd]").unwrap();
        assert_eq!(cmd, "pwd");
        assert!(args.is_empty());
    }

    #[test]
    fn spans_basic_arg_offsets_are_exact() {
        let text = "[lindex $x 0]";
        let (cmd, args) = parse_command_substitution_with_spans(text).unwrap();
        assert_eq!(cmd, "lindex");
        assert_eq!(args.len(), 2);
        let (x_text, x_span) = &args[0];
        assert_eq!(x_text, "$x");
        assert_eq!(&text[x_span.start() as usize..x_span.end() as usize], "$x");
        let (zero_text, zero_span) = &args[1];
        assert_eq!(zero_text, "0");
        assert_eq!(
            &text[zero_span.start() as usize..zero_span.end() as usize],
            "0"
        );
    }

    #[test]
    fn spans_account_for_leading_whitespace_and_inner_padding() {
        let text = "  [ set x 42 ]  ";
        let (cmd, args) = parse_command_substitution_with_spans(text).unwrap();
        assert_eq!(cmd, "set");
        let (x_text, x_span) = &args[0];
        assert_eq!(x_text, "x");
        assert_eq!(&text[x_span.start() as usize..x_span.end() as usize], "x");
        let (v_text, v_span) = &args[1];
        assert_eq!(v_text, "42");
        assert_eq!(&text[v_span.start() as usize..v_span.end() as usize], "42");
    }

    #[test]
    fn spans_none_for_non_bracketed_or_empty() {
        assert!(parse_command_substitution_with_spans("llength $x").is_none());
        assert!(parse_command_substitution_with_spans("[]").is_none());
    }

    #[test]
    fn spans_none_for_multiple_top_level_commands() {
        // A second command after a newline/`;` inside the substitution body
        // is not a single-command shape this helper models.
        assert!(parse_command_substitution_with_spans("[set x 1; set y 2]").is_none());
    }

    #[test]
    fn spans_nested_command_substitution_keeps_full_bracket_text() {
        let text = "[list [read $fd]]";
        let (cmd, args) = parse_command_substitution_with_spans(text).unwrap();
        assert_eq!(cmd, "list");
        assert_eq!(args.len(), 1);
        let (nested_text, nested_span) = &args[0];
        assert_eq!(nested_text, "[read $fd]");
        assert_eq!(
            &text[nested_span.start() as usize..nested_span.end() as usize],
            "[read $fd]"
        );
    }

    #[test]
    fn spans_braced_arg_keeps_full_brace_text() {
        let text = "[list {a b c}]";
        let (cmd, args) = parse_command_substitution_with_spans(text).unwrap();
        assert_eq!(cmd, "list");
        let (braced_text, braced_span) = &args[0];
        assert_eq!(braced_text, "{a b c}");
        assert_eq!(
            &text[braced_span.start() as usize..braced_span.end() as usize],
            "{a b c}"
        );
    }

    #[test]
    fn spans_empty_group_arg_not_overwidened() {
        let text = "[list {} x]";
        let (cmd, args) = parse_command_substitution_with_spans(text).unwrap();
        assert_eq!(cmd, "list");
        assert_eq!(args.len(), 2);
        let (empty_text, empty_span) = &args[0];
        assert_eq!(empty_text, "{}");
        assert_eq!(
            &text[empty_span.start() as usize..empty_span.end() as usize],
            "{}"
        );
        let (x_text, x_span) = &args[1];
        assert_eq!(x_text, "x");
        assert_eq!(&text[x_span.start() as usize..x_span.end() as usize], "x");
    }

    /// A non-degenerate braced variable reference (`${p}`, real name
    /// inside the braces) keeps its closing `}` — the raw `Var` token's
    /// span excludes it (same inner-end convention as `Str`/`Cmd`), and
    /// `Var` has no `group_closer()` entry since the *unbraced* `$name`
    /// form has no closer at all to unconditionally claim.
    #[test]
    fn spans_braced_var_keeps_closing_brace() {
        let text = "[file normalize ${p}]";
        let (cmd, args) = parse_command_substitution_with_spans(text).unwrap();
        assert_eq!(cmd, "file");
        assert_eq!(args[1].0, "${p}");
        assert_eq!(
            &text[args[1].1.start() as usize..args[1].1.end() as usize],
            "${p}"
        );
    }

    /// Control: an unbraced `$name` reference must not gain a spurious
    /// trailing `}` from the fix above.
    #[test]
    fn spans_unbraced_var_unaffected() {
        let text = "[file normalize $p]";
        let (cmd, args) = parse_command_substitution_with_spans(text).unwrap();
        assert_eq!(cmd, "file");
        assert_eq!(args[1].0, "$p");
    }

    /// A double-quoted word with no embedded substitution keeps its
    /// closing `"` in the returned text — the lexer's raw token span for
    /// the final content fragment excludes it (there is no dedicated
    /// token kind for a quoted word to attach `group_closer()` to, unlike
    /// `Str`/`Cmd`), so it must be recovered from the one-byte gap that
    /// widening otherwise leaves unclaimed.
    #[test]
    fn spans_quoted_word_keeps_closing_quote() {
        let text = r#"[string match "/api/*" $uri]"#;
        let (cmd, args) = parse_command_substitution_with_spans(text).unwrap();
        assert_eq!(cmd, "string");
        assert_eq!(args[1].0, "\"/api/*\"");
        assert_eq!(
            &text[args[1].1.start() as usize..args[1].1.end() as usize],
            "\"/api/*\""
        );
    }

    /// Same recovery applies when the quoted word's last fragment follows
    /// an embedded command substitution rather than being the whole word.
    #[test]
    fn spans_quoted_word_with_nested_command_sub_keeps_closing_quote() {
        let text = r#"[string match "a[foo]b" $c]"#;
        let (cmd, args) = parse_command_substitution_with_spans(text).unwrap();
        assert_eq!(cmd, "string");
        assert_eq!(args[1].0, "\"a[foo]b\"");
        assert_eq!(
            &text[args[1].1.start() as usize..args[1].1.end() as usize],
            "\"a[foo]b\""
        );
    }

    /// Control: when the closing quote directly follows an embedded
    /// `$var` substitution, the lexer already emits it as its own
    /// adjacent token (no gap) — confirms the fix above doesn't
    /// double-append in that shape.
    #[test]
    fn spans_quoted_word_after_var_substitution_keeps_closing_quote() {
        let text = r#"[string match "a$b" $c]"#;
        let (cmd, args) = parse_command_substitution_with_spans(text).unwrap();
        assert_eq!(cmd, "string");
        assert_eq!(args[1].0, "\"a$b\"");
        assert_eq!(
            &text[args[1].1.start() as usize..args[1].1.end() as usize],
            "\"a$b\""
        );
    }
}
