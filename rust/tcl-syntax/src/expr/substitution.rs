// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Complete command-substitution spans in a Tcl expression.
//!
//! The expression lexer intentionally makes a double-quoted operand one
//! opaque `String` token.  That is right for expression parsing, but a
//! consumer that needs to execute or inventory the embedded Tcl commands must
//! still see substitutions in that operand.  This module owns that bridge: it
//! keeps expression-level braces and comments inert, while delegating every
//! actual `[…]` closer to the lexer range owner.

use tcl_lexer::{Span, backslash_escape_end, command_substitution_end};

/// Return the complete, outermost command substitutions directly evaluated by
/// `source` as a Tcl expression.
///
/// Each returned [`Span`] is a byte-exact, half-open range including both
/// brackets.  An escaped bracket, a bracket inside a braced literal, and an
/// unterminated command substitution are absent.  Substitutions in a
/// double-quoted expression operand are included.  A substitution nested in a
/// returned command is intentionally not repeated: it belongs to that
/// command's Tcl-script body and its script consumer will recurse into it.
///
/// `dialect` controls TIP 582 expression comments in the same way as the
/// expression lexer.  The closer itself is owned by
/// [`tcl_lexer::command_substitution_end`], so quotes, braces, nested command
/// substitutions, escapes, and command-position comments *inside* a returned
/// command use Tcl script grammar rather than a duplicate expression scan.
#[must_use]
pub fn command_substitution_spans(source: &str, dialect: Option<&str>) -> Vec<Span> {
    let comments = tcl_dialect::DialectProfile::by_opt_name(dialect)
        .grammar
        .expr_comments
        .comments();
    let mut spans = Vec::new();
    scan_expression(source, 0, source.len(), comments, &mut spans);
    spans
}

fn scan_expression(
    source: &str,
    mut pos: usize,
    end: usize,
    comments: bool,
    spans: &mut Vec<Span>,
) {
    let bytes = source.as_bytes();
    while pos < end {
        match bytes[pos] {
            b'\\' => pos = backslash_escape_end(source, pos).min(end),
            b'#' if comments => {
                pos = source[pos..end]
                    .find('\n')
                    .map_or(end, |relative| pos + relative + 1);
            }
            b'{' => match braced_literal_end(source, pos, end) {
                Some(next) => pos = next,
                // A malformed braced operand gives us no sound outer grammar
                // in which to prove a later `[` live.  Fail closed.
                None => return,
            },
            b'"' => {
                pos = scan_quoted_operand(source, pos, end, spans).unwrap_or(end);
            }
            b'[' => match command_substitution_end(source, pos).filter(|&next| next <= end) {
                Some(next) => {
                    spans.push(span(pos, next));
                    pos = next;
                }
                // Do not manufacture a span for an editor-buffer fragment
                // whose `]` has not yet been written.
                None => return,
            },
            _ => pos += 1,
        }
    }
}

/// Skip a braced expression operand.  Braces are literal syntax: neither
/// substitutions nor comments run in them.  This mirrors the brace balance
/// rule used by the expression lexer.
fn braced_literal_end(source: &str, open: usize, end: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut pos = open + 1;
    let mut depth = 1_u32;
    while pos < end {
        match bytes[pos] {
            b'\\' => pos = backslash_escape_end(source, pos).min(end),
            b'{' => {
                depth += 1;
                pos += 1;
            }
            b'}' => {
                depth -= 1;
                pos += 1;
                if depth == 0 {
                    return Some(pos);
                }
            }
            _ => pos += 1,
        }
    }
    None
}

/// Scan a quoted expression operand for substitutions; braces and `#` are
/// ordinary characters here.  Spans are committed only after the closing
/// quote, so an incomplete quoted operand does not claim that its inner
/// command can execute.
fn scan_quoted_operand(
    source: &str,
    open: usize,
    end: usize,
    spans: &mut Vec<Span>,
) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut pos = open + 1;
    let mut quoted_spans = Vec::new();
    while pos < end {
        match bytes[pos] {
            b'\\' => pos = backslash_escape_end(source, pos).min(end),
            b'"' => {
                spans.extend(quoted_spans);
                return Some(pos + 1);
            }
            b'[' => {
                let next = command_substitution_end(source, pos).filter(|&next| next <= end)?;
                quoted_spans.push(span(pos, next));
                pos = next;
            }
            _ => pos += 1,
        }
    }
    None
}

fn span(start: usize, end: usize) -> Span {
    Span::new(
        u32::try_from(start).expect("expression offset fits u32"),
        u32::try_from(end).expect("expression offset fits u32"),
    )
}

#[cfg(test)]
mod tests {
    use super::command_substitution_spans;

    fn texts(source: &str, dialect: Option<&str>) -> Vec<String> {
        command_substitution_spans(source, dialect)
            .into_iter()
            .map(|span| source[span.as_range()].to_owned())
            .collect()
    }

    #[test]
    fn reports_direct_live_substitutions_with_exact_unicode_offsets() {
        let source = "☃ + \"[HTTP::host]\" + [HTTP::uri]";
        let spans = command_substitution_spans(source, Some("f5-irules"));
        assert_eq!(
            texts(source, Some("f5-irules")),
            ["[HTTP::host]", "[HTTP::uri]"]
        );
        assert_eq!(
            spans[0].start() as usize,
            source.find("[HTTP::host]").unwrap()
        );
        assert_eq!(
            spans[0].end() as usize,
            source.find("[HTTP::host]").unwrap() + 12
        );
    }

    #[test]
    fn excludes_escaped_braced_and_unterminated_brackets() {
        assert_eq!(
            texts(r"\[escaped] + {[braced]} + [live]", Some("f5-irules")),
            ["[live]"]
        );
        assert!(texts("[unterminated", Some("f5-irules")).is_empty());
        assert!(texts("\"[complete_but_unquoted]", Some("f5-irules")).is_empty());
    }

    #[test]
    fn respects_expression_and_nested_script_comments() {
        let source = "# [inert]\n[set x 1; # ] in command\nHTTP::host]";
        assert_eq!(
            texts(source, Some("tcl9.0")),
            ["[set x 1; # ] in command\nHTTP::host]"]
        );
        assert_eq!(
            texts(source, Some("tcl8.6")),
            ["[inert]", "[set x 1; # ] in command\nHTTP::host]"]
        );
    }

    #[test]
    fn nested_command_is_owned_by_its_outer_script() {
        assert_eq!(
            texts("[expr {[HTTP::host] ne \"\"}]", Some("f5-irules")),
            ["[expr {[HTTP::host] ne \"\"}]"]
        );
    }
}
