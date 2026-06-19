//! Token / source-range helpers for the optimiser.
//!
//! Ported from `core/compiler/optimiser/_helpers.py`:
//! `_extract_body_text` and (a simplified form of)
//! `_full_command_range`. The full Python `_full_command_range`
//! re-runs the lexer from the start of the command to discover
//! trailing whitespace and closing braces; in the Rust port the
//! IR statement `span` already covers the whole command
//! (produced by `lowering::structured`), so the helper is a
//! thin passthrough.
//!
//! Consumers: [`super::super::structure_elimination`] (body
//! extraction when replacing a dead if/while/for/switch with its
//! surviving branch), [`super::super::elimination`] (future).

use tcl_lexer::Span;

/// Return the source text enclosed by `body_span`, stripped of
/// its outer braces, leading newline, trailing whitespace, and
/// re-indented to match `stmt_span`'s column.
///
/// Matches `_extract_body_text` behaviour:
///
/// 1. Slice `source[body_span]`.
/// 2. Strip one leading `{` and one trailing `}`.
/// 3. Strip a single leading `\n` or `\r\n`.
/// 4. Rstrip spaces / tabs / CR / LF.
/// 5. Compute the target indent from `stmt_span`'s column (bytes
///    between the start of its line and `stmt_span.start()`).
/// 6. Compute the minimum common leading whitespace of non-empty
///    body lines.
/// 7. Re-indent every line (except the first, which keeps its
///    current indent) to the target column.
///
/// Returns an empty string for nonsense ranges or empty bodies.
#[must_use]
pub fn extract_body_text(source: &str, body_span: Span, stmt_span: Span) -> String {
    let body_range = body_span.as_range();
    if body_range.end > source.len() || body_range.start > body_range.end {
        return String::new();
    }
    let raw = &source[body_range];
    let mut text = raw;

    // Strip one level of braces.
    if let Some(inner) = text.strip_prefix('{') {
        text = inner;
    }
    if let Some(inner) = text.strip_suffix('}') {
        text = inner;
    }

    // Strip a single leading newline.
    let mut trimmed: String = if let Some(rest) = text.strip_prefix("\r\n") {
        rest.to_owned()
    } else if let Some(rest) = text.strip_prefix('\n') {
        rest.to_owned()
    } else {
        text.to_owned()
    };

    // Right-trim whitespace.
    let trimmed_ref = trimmed.trim_end_matches([' ', '\t', '\r', '\n']);
    if trimmed_ref.len() != trimmed.len() {
        trimmed.truncate(trimmed_ref.len());
    }
    if trimmed.is_empty() {
        return String::new();
    }

    // Target indent = byte column of stmt_span.start().
    let target_col = column_of_offset(source, stmt_span.start() as usize);
    let target_indent = " ".repeat(target_col);

    // Compute the minimum common leading whitespace of non-empty
    // body lines.
    let mut min_indent: Option<usize> = None;
    for line in trimmed.split('\n') {
        let stripped = line.trim_start_matches([' ', '\t']);
        if stripped.is_empty() {
            continue;
        }
        let leading = line.len() - stripped.len();
        min_indent = Some(match min_indent {
            Some(m) => m.min(leading),
            None => leading,
        });
    }
    let min_indent = min_indent.unwrap_or(0);

    // Re-indent.
    let lines: Vec<&str> = trimmed.split('\n').collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    for (i, line) in lines.iter().enumerate() {
        let stripped = line.trim_start_matches([' ', '\t']);
        if stripped.is_empty() {
            out.push(String::new());
        } else if i == 0 {
            out.push(line[min_indent.min(line.len())..].to_owned());
        } else {
            let dedented = &line[min_indent.min(line.len())..];
            out.push(format!("{target_indent}{dedented}"));
        }
    }
    out.join("\n")
}

/// Column (byte offset into its line) of `offset` within
/// `source`. Used to compute target-indent for
/// [`extract_body_text`].
#[must_use]
pub fn column_of_offset(source: &str, offset: usize) -> usize {
    let clamped = offset.min(source.len());
    let prefix = &source[..clamped];
    match prefix.rfind('\n') {
        Some(nl) => clamped - (nl + 1),
        None => clamped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_simple_cases() {
        assert_eq!(column_of_offset("abcdef", 3), 3);
        assert_eq!(column_of_offset("abc\ndef", 4), 0);
        assert_eq!(column_of_offset("abc\ndef", 6), 2);
        assert_eq!(column_of_offset("ab\ncd\nef", 7), 1);
        // Offset past end clamps.
        assert_eq!(column_of_offset("abc", 100), 3);
    }

    #[test]
    fn extract_body_text_strips_braces_and_newlines() {
        let source = "if {1} {\n    puts hi\n}";
        // len=22. Body span covers `{\n    puts hi\n}` → 7..22.
        let body_span = Span::new(7, 22);
        // stmt_span starts at col 0 → target indent "".
        let stmt_span = Span::new(0, 22);
        let out = extract_body_text(source, body_span, stmt_span);
        assert_eq!(out, "puts hi");
    }

    #[test]
    fn extract_body_text_preserves_multi_line_and_reindents() {
        let source = "    while {1} {\n        a\n        b\n    }";
        let body_span = Span::new(14, u32::try_from(source.len()).unwrap());
        let stmt_span = Span::new(4, u32::try_from(source.len()).unwrap());
        let out = extract_body_text(source, body_span, stmt_span);
        // Target indent = 4 spaces; min body indent = 8; each
        // subsequent line loses 4 cols of indent (from 8 → 4).
        assert_eq!(out, "a\n    b");
    }

    #[test]
    fn extract_body_text_empty_for_whitespace_only_body() {
        let source = "if {0} { \n  \n }";
        let body_span = Span::new(7, u32::try_from(source.len()).unwrap());
        let stmt_span = Span::new(0, u32::try_from(source.len()).unwrap());
        let out = extract_body_text(source, body_span, stmt_span);
        assert!(out.is_empty());
    }

    #[test]
    fn extract_body_text_handles_out_of_bounds_span() {
        let out = extract_body_text("hi", Span::new(5, 10), Span::new(0, 2));
        assert!(out.is_empty());
    }
}
