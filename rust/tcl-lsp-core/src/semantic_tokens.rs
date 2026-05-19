//! Semantic-tokens provider — Rust port of
//! `lsp/features/_semantic_tokens/`.
//!
//! Produces an LSP-encoded semantic-tokens stream covering
//! the common Tcl token categories:
//!
//! * **Keyword** — control-flow command heads (`if`, `while`,
//!   `for`, `foreach`, `switch`, `return`, `break`,
//!   `continue`, `try`, `catch`, `eval`, `uplevel`, `upvar`,
//!   `expr`, `subst`, `proc`, `namespace`, `set`, `unset`,
//!   `global`, `variable`, `lmap`, `lappend`, `incr`,
//!   `append`).
//! * **Function** — every other command-head token (user
//!   procs + built-in commands).
//! * **Variable** — `$name` / `${name}` substitutions.
//! * **String** — braced literals (`{...}`) and double-quoted
//!   strings.
//! * **Number** — integer / float literals.
//! * **Comment** — `# ...` comment lines.
//! * **Namespace** — namespace-qualified names containing
//!   `::`.
//!
//! The legend is exposed via [`legend_token_types`] and
//! [`legend_token_modifiers`] so the server advertises it in
//! the LSP `initialize` capabilities response.
//!
//! What is *deferred* (planned as further
//! `S-semantic-tokens-rich` sub-strips):
//!
//! * Format-string component highlighting (`%Y` /
//!   `\1` / `*.tcl` inside `clock format` / `regsub` /
//!   `glob`).  Each format helper already has a hover; the
//!   semantic-token side needs the same cursor-context
//!   detection plus per-component classification.
//! * `BigIP` URI segments / iRules-specific event names.
//! * Delta encoding (`semanticTokens/full/delta`) — the
//!   minimal port returns a fresh full stream on every
//!   request.
//! * `semanticTokens/range` — same encoding limited to a
//!   range; defer until the full stream has good UX.

use tcl_compiler::segmenter::segment_commands;
use tcl_lexer::{LineIndex, Token, TokenType};

/// Encoded semantic-tokens response.  The `data` array is
/// the LSP packed integer encoding (5 ints per token: line
/// delta, column delta, length, type, modifiers).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SemanticTokens {
    /// Packed integer data.
    pub data: Vec<u32>,
}

/// Indexed enum for the token types we emit.  Numeric
/// values must align with the order returned by
/// [`legend_token_types`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
enum TokenKind {
    Keyword = 0,
    Function = 1,
    Variable = 2,
    String = 3,
    Number = 4,
    Comment = 5,
    Namespace = 6,
}

/// The token-type / token-modifier legend the server
/// advertises during `initialize`.
#[must_use]
pub fn legend_token_types() -> Vec<&'static str> {
    vec![
        "keyword",
        "function",
        "variable",
        "string",
        "number",
        "comment",
        "namespace",
    ]
}

/// Token-modifiers part of the legend.  The minimal rich
/// port doesn't use modifiers; this list reserves the
/// position for future deprecation / readonly / etc.
#[must_use]
pub fn legend_token_modifiers() -> Vec<&'static str> {
    Vec::new()
}

/// Tcl control-flow / structural keywords.  Mirrors Python's
/// `_CONTROL_FLOW_KEYWORDS` in `_constants.py`.
const KEYWORD_COMMANDS: &[&str] = &[
    "if",
    "elseif",
    "else",
    "while",
    "for",
    "foreach",
    "switch",
    "return",
    "break",
    "continue",
    "try",
    "catch",
    "eval",
    "uplevel",
    "upvar",
    "expr",
    "subst",
    "proc",
    "namespace",
    "set",
    "unset",
    "global",
    "variable",
    "lmap",
    "lappend",
    "incr",
    "append",
    "default",
    "on",
    "trap",
    "finally",
];

/// Classify a command-head token name.  Mirrors Python's
/// `_classify_command_head`.
fn classify_command_head(name: &str) -> TokenKind {
    if KEYWORD_COMMANDS.contains(&name) {
        TokenKind::Keyword
    } else if name.contains("::") {
        TokenKind::Namespace
    } else {
        TokenKind::Function
    }
}

/// Compute semantic tokens for the entire document.
#[must_use]
pub fn full(source: &str) -> SemanticTokens {
    let entries = collect_entries(source);
    encode_entries(&entries)
}

/// Compute semantic tokens for `range` within the document.
/// Tokens whose start position falls outside the range are
/// dropped.  Delta encoding starts from the first surviving
/// token rather than the document origin, matching the LSP
/// spec for `semanticTokens/range`.
#[must_use]
pub fn range(source: &str, range: crate::definition::LspRange) -> SemanticTokens {
    let mut entries = collect_entries(source);
    entries.retain(|(line, col, _, _)| {
        let starts_after_or_at_range_start = (*line, *col)
            >= (range.start_line, range.start_character);
        let starts_before_range_end = (*line, *col)
            <= (range.end_line, range.end_character);
        starts_after_or_at_range_start && starts_before_range_end
    });
    encode_entries(&entries)
}

/// Walk the segmenter + comment scan and return raw
/// `(line, col, length, kind)` tuples sorted by position.
/// Shared by `full` and `range`.
fn collect_entries(source: &str) -> Vec<(u32, u32, u32, TokenKind)> {
    let mut entries: Vec<(u32, u32, u32, TokenKind)> = Vec::new();
    let line_index = LineIndex::new(source);

    // Walk every segmented command and classify each token.
    for seg in segment_commands(source) {
        if seg.argv.is_empty() {
            continue;
        }
        // Classify the command-head token.
        let head_tok = seg.argv[0];
        let head_text = &seg.texts[0];
        let head_kind = classify_command_head(head_text);
        push_token(&line_index, source, head_tok, head_kind, &mut entries);

        // Walk the remaining tokens (arg-position tokens
        // + nested tokens).  Each contributes a classification
        // based on its `TokenType`.
        for tok in &seg.all_tokens {
            // Skip the head token (already pushed).
            if tok.span == head_tok.span {
                continue;
            }
            if let Some(kind) = classify_arg_token(*tok, source) {
                push_token(&line_index, source, *tok, kind, &mut entries);
            }
        }
    }

    // Comments aren't in the segmenter's command stream
    // (it strips them).  Scan the source for `#` comments
    // separately.
    push_comment_tokens(source, &line_index, &mut entries);

    // Sort by (line, column) so the delta encoding works.
    entries.sort_by_key(|(line, col, _, _)| (*line, *col));
    entries
}

/// Classify a non-head token by its lexer-assigned kind.
fn classify_arg_token(tok: Token, source: &str) -> Option<TokenKind> {
    let span = tok.span;
    let len = (span.end() - span.start()) as usize;
    if len == 0 {
        return None;
    }
    match tok.kind {
        TokenType::Var => Some(TokenKind::Variable),
        TokenType::Str => Some(TokenKind::String),
        TokenType::Esc => {
            // Quoted strings vs barewords vs numbers.  The
            // segmenter doesn't differentiate; peek at the
            // source byte to disambiguate.
            let start = span.start() as usize;
            let bytes = source.as_bytes();
            if start < bytes.len() && bytes[start] == b'"' {
                Some(TokenKind::String)
            } else {
                let text = source
                    .get(start..(start + len).min(source.len()))
                    .unwrap_or("");
                if is_number_literal(text) {
                    Some(TokenKind::Number)
                } else if text.contains("::") {
                    Some(TokenKind::Namespace)
                } else {
                    None
                }
            }
        }
        _ => None,
    }
}

/// `true` when `text` is a Tcl number literal — integer
/// (optionally signed, hex `0x...` or binary `0b...`) or
/// floating-point.
fn is_number_literal(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    let trimmed = text.trim_start_matches(['+', '-']);
    if trimmed.is_empty() {
        return false;
    }
    if let Some(rest) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return !rest.is_empty() && rest.chars().all(|c| c.is_ascii_hexdigit() || c == '_');
    }
    if let Some(rest) = trimmed
        .strip_prefix("0b")
        .or_else(|| trimmed.strip_prefix("0B"))
    {
        return !rest.is_empty() && rest.chars().all(|c| matches!(c, '0' | '1' | '_'));
    }
    // Integer or float.  Use Rust's parsers for simplicity.
    text.parse::<i64>().is_ok() || text.parse::<f64>().is_ok()
}

/// Scan `source` for `#` comment lines and push each one as
/// a Comment-kind entry.  Mirrors Python's `_collect_comments`.
fn push_comment_tokens(
    source: &str,
    line_index: &LineIndex,
    entries: &mut Vec<(u32, u32, u32, TokenKind)>,
) {
    let mut byte_pos: u32 = 0;
    let mut line_start = true;
    for c in source.chars() {
        let len = u32::try_from(c.len_utf8()).unwrap_or(1);
        if c == '\n' {
            line_start = true;
            byte_pos += len;
            continue;
        }
        if c.is_whitespace() {
            byte_pos += len;
            continue;
        }
        if line_start && c == '#' {
            // Find the end of the comment line.
            let comment_start = byte_pos;
            let mut p = byte_pos;
            let bytes = source.as_bytes();
            while (p as usize) < bytes.len() && bytes[p as usize] != b'\n' {
                p += 1;
            }
            let comment_end = p;
            let pos = line_index.position_at(comment_start);
            let len_chars = u32::try_from(
                source[comment_start as usize..comment_end as usize]
                    .chars()
                    .count(),
            )
            .unwrap_or(0);
            entries.push((pos.line, pos.character, len_chars, TokenKind::Comment));
            // Skip past the comment line.
            byte_pos = comment_end;
            line_start = false;
            continue;
        }
        line_start = false;
        byte_pos += len;
    }
}

/// Push a single token into the entries list, computing
/// (line, column, length-in-chars, kind).
fn push_token(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    kind: TokenKind,
    entries: &mut Vec<(u32, u32, u32, TokenKind)>,
) {
    let span = tok.span;
    let len_bytes = span.end() - span.start();
    if len_bytes == 0 {
        return;
    }
    let pos = line_index.position_at(span.start());
    let text = source
        .get(span.start() as usize..span.end() as usize)
        .unwrap_or("");
    // Skip multi-line tokens — LSP encoding wants per-line
    // entries; multi-line tokens would need splitting.
    // For the minimal rich port, drop them.
    if text.contains('\n') {
        return;
    }
    let len_chars = u32::try_from(text.chars().count()).unwrap_or(0);
    entries.push((pos.line, pos.character, len_chars, kind));
}

/// Encode entries into the LSP packed integer stream:
/// `[deltaLine, deltaCol, length, type, modifiers]` per token.
fn encode_entries(entries: &[(u32, u32, u32, TokenKind)]) -> SemanticTokens {
    let mut data: Vec<u32> = Vec::with_capacity(entries.len() * 5);
    let mut prev_line: u32 = 0;
    let mut prev_col: u32 = 0;
    for (line, col, len, kind) in entries {
        let delta_line = line.saturating_sub(prev_line);
        let delta_col = if delta_line == 0 {
            col.saturating_sub(prev_col)
        } else {
            *col
        };
        data.push(delta_line);
        data.push(delta_col);
        data.push(*len);
        data.push(*kind as u32);
        data.push(0); // No modifiers.
        prev_line = *line;
        prev_col = *col;
    }
    SemanticTokens { data }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_returns_non_empty_data_for_simple_proc() {
        let s = full("proc foo {} {}\n");
        // Should have at least: `proc` (keyword), `foo`
        // (function), `{}` (string), `{}` (string).
        assert!(!s.data.is_empty(), "{:?}", s.data);
        // 5 ints per token.
        assert_eq!(s.data.len() % 5, 0);
    }

    #[test]
    fn legend_has_expected_entries() {
        let types = legend_token_types();
        assert_eq!(types[TokenKind::Keyword as usize], "keyword");
        assert_eq!(types[TokenKind::Function as usize], "function");
        assert_eq!(types[TokenKind::Variable as usize], "variable");
        assert_eq!(types[TokenKind::String as usize], "string");
        assert_eq!(types[TokenKind::Number as usize], "number");
        assert_eq!(types[TokenKind::Comment as usize], "comment");
        assert_eq!(types[TokenKind::Namespace as usize], "namespace");
    }

    #[test]
    fn keywords_classified_as_keyword() {
        let s = full("if {1} { puts hi }\n");
        // First token's type index should be 0 (Keyword) for `if`.
        // The encoded data: [deltaLine, deltaCol, length, type, modifiers].
        assert_eq!(s.data[3], TokenKind::Keyword as u32, "{:?}", s.data);
    }

    #[test]
    fn comments_classified_as_comment() {
        let s = full("# this is a comment\nset x 1\n");
        // The first token should be the comment.
        assert_eq!(s.data[3], TokenKind::Comment as u32, "{:?}", s.data);
    }

    #[test]
    fn variables_classified_as_variable() {
        let s = full("set $x 1\n");
        // The `$x` token kind should be Variable.
        let kinds: Vec<u32> = s.data.chunks(5).map(|c| c[3]).collect();
        assert!(
            kinds.contains(&(TokenKind::Variable as u32)),
            "expected Variable in kinds; got {kinds:?}",
        );
    }

    #[test]
    fn is_number_literal_recognises_integers_and_floats() {
        assert!(is_number_literal("42"));
        assert!(is_number_literal("-7"));
        assert!(is_number_literal("3.14"));
        assert!(is_number_literal("0xff"));
        assert!(is_number_literal("0b1010"));
        assert!(!is_number_literal("abc"));
        assert!(!is_number_literal(""));
        assert!(!is_number_literal("1.2.3"));
    }

    #[test]
    fn empty_source_returns_empty_data() {
        assert!(full("").data.is_empty());
    }

    #[test]
    fn classify_command_head_picks_namespace_for_qualified() {
        assert_eq!(classify_command_head("::myns::greet"), TokenKind::Namespace,);
        assert_eq!(classify_command_head("greet"), TokenKind::Function);
        assert_eq!(classify_command_head("if"), TokenKind::Keyword);
    }

    // -- S-semantic-tokens-rich: range variant -----------------------

    #[test]
    fn range_filters_tokens_outside_window() {
        // Three commands on three lines.  Range covers only
        // line 1 — the line-0 and line-2 tokens should drop.
        let src = "set a 1\nset b 2\nset c 3\n";
        let full_data = full(src);
        let line1_only = range(
            src,
            crate::definition::LspRange {
                start_line: 1,
                start_character: 0,
                end_line: 1,
                end_character: 10,
            },
        );
        // Each tcl line emits at least one classified token.
        // The range result must be strictly smaller than the
        // full result.
        assert!(line1_only.data.len() < full_data.data.len());
        assert!(line1_only.data.len() % 5 == 0);
        assert!(!line1_only.data.is_empty(), "{:?}", line1_only.data);
    }

    #[test]
    fn range_keeps_entire_document_when_range_covers_it() {
        let src = "proc foo {} { puts hi }\n";
        let full_data = full(src);
        let wide = range(
            src,
            crate::definition::LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 99,
                end_character: 0,
            },
        );
        assert_eq!(wide.data, full_data.data);
    }
}
