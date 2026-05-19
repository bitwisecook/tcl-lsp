//! Formatting provider — Rust port of
//! `lsp/features/formatting.py` (basic shape).
//!
//! Produces a single full-document `TextEdit` that
//! replaces the source with a normalised form:
//!
//! * Each line is trimmed of trailing whitespace.
//! * Tabs at the start of a line are converted to four
//!   spaces.
//! * Indentation tracks the brace nesting depth (4 spaces
//!   per level).  Continuation lines inside an open brace
//!   pick up one level of indentation.
//! * Multiple consecutive blank lines collapse to a single
//!   blank line.
//! * The final line is followed by a single trailing
//!   newline.
//!
//! The formatter is intentionally conservative — it only
//! rewrites whitespace, never the order or shape of
//! command words.  Subtle multi-line Tcl forms (`if {1}
//! { … } else { … }` on one line, switch-case bodies,
//! complex `proc` arg lists) keep their existing layout.
//!
//! What is *deferred* (planned as further `S-formatting-
//! rich` / `F-tcl-formatter` sub-strips):
//!
//! * Brace-placement normalisation (K&R / Allman /
//!   per-file-config).
//! * Comment-block reflow.
//! * Range-formatting (currently emits a whole-document
//!   replacement when given a range).
//! * Configurable tab width / indentation style via
//!   `FormatterConfig`.
//! * `lsp/features/code_actions.py`-driven indentation
//!   adjustments for partial edits.

use crate::definition::LspRange;
use crate::rename::TextEdit;
use tcl_lexer::LineIndex;

/// Compute formatting edits for the entire document.
///
/// Returns a single `TextEdit` that replaces the whole
/// document with its normalised form, or an empty `Vec`
/// when the document is already normalised.
#[must_use]
pub fn formatting(source: &str) -> Vec<TextEdit> {
    let formatted = format_source(source);
    if formatted == source {
        return Vec::new();
    }
    let line_index = LineIndex::new(source);
    let end_pos = line_index.position_at(u32::try_from(source.len()).unwrap_or(0));
    vec![TextEdit {
        range: LspRange {
            start_line: 0,
            start_character: 0,
            end_line: end_pos.line,
            end_character: end_pos.character,
        },
        new_text: formatted,
    }]
}

/// Compute formatting edits for a range within the
/// document.  The current implementation falls back to a
/// whole-document format (the LSP allows this — the
/// editor handles applying overlapping edits gracefully).
#[must_use]
pub fn range_formatting(source: &str, _range: LspRange) -> Vec<TextEdit> {
    formatting(source)
}

/// Whitespace-normalise `source`.  Pure function so tests
/// can exercise the algorithm without invoking the LSP
/// edit-emission layer.
#[must_use]
pub fn format_source(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut depth: i32 = 0;
    let mut prev_blank = false;
    // We need to keep newline-only lines but collapse runs.
    let lines: Vec<&str> = source.split('\n').collect();
    let total = lines.len();
    for (idx, line) in lines.iter().enumerate() {
        // The final `split('\n')` element is the empty
        // string after a trailing newline.  Drop it so we
        // emit exactly one trailing newline at the end.
        if idx + 1 == total && line.is_empty() {
            break;
        }
        let trimmed = line.trim_end();
        let stripped = trimmed.trim_start();
        if stripped.is_empty() {
            // Blank line — emit once even if multiple
            // adjacent blanks.
            if !prev_blank && !out.is_empty() {
                out.push('\n');
                prev_blank = true;
            }
            continue;
        }
        prev_blank = false;
        // Compute leading-brace adjustment: a line that
        // starts with `}` decreases indent before the line;
        // a line ending with `{` increases indent for the
        // next line.  We use the running brace-delta of
        // the *stripped* content (excluding string /
        // braced literals — best-effort).
        let leading_close = leading_close_braces(stripped);
        let line_depth = (depth - leading_close).max(0);
        for _ in 0..line_depth {
            out.push_str("    ");
        }
        out.push_str(stripped);
        out.push('\n');
        depth = (depth + brace_delta(stripped)).max(0);
    }
    // Ensure exactly one trailing newline when the source
    // had any content at all.
    if !out.ends_with('\n') && !out.is_empty() {
        out.push('\n');
    }
    out
}

/// Count the number of `}` characters at the start of a
/// line (ignoring leading whitespace).  Used to dedent the
/// closing-brace line itself.
fn leading_close_braces(line: &str) -> i32 {
    let mut n = 0;
    for c in line.chars() {
        if c == '}' {
            n += 1;
        } else {
            break;
        }
    }
    n
}

/// Net brace delta for a logical line.  Ignores braces
/// inside `"..."` strings and inside the body of a brace-
/// literal that fully nests in the line (e.g. `proc f {}
/// {body}` is depth-neutral).  Conservative — we count
/// every `{` / `}` outside double-quoted strings.
fn brace_delta(line: &str) -> i32 {
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escaped = false;
    let mut in_comment = false;
    for c in line.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if in_comment {
            // Tcl comments run to end of line.
            continue;
        }
        if in_string {
            if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '#' if depth == 0 && line.trim_start().starts_with('#') => {
                in_comment = true;
            }
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
    }
    depth
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn already_formatted_returns_no_edits() {
        let src = "proc foo {} {\n    set x 1\n}\n";
        assert!(formatting(src).is_empty(), "{:?}", formatting(src));
    }

    #[test]
    fn collapses_multiple_blank_lines() {
        let src = "set x 1\n\n\n\nset y 2\n";
        let out = format_source(src);
        assert_eq!(out, "set x 1\n\nset y 2\n");
    }

    #[test]
    fn trims_trailing_whitespace() {
        let src = "set x 1   \nset y 2\t\n";
        let out = format_source(src);
        assert_eq!(out, "set x 1\nset y 2\n");
    }

    #[test]
    fn indents_proc_body() {
        let src = "proc f {} {\nset x 1\n}\n";
        let out = format_source(src);
        assert_eq!(out, "proc f {} {\n    set x 1\n}\n");
    }

    #[test]
    fn dedents_closing_brace() {
        let src = "if {1} {\nset x 1\n}\n";
        let out = format_source(src);
        assert_eq!(out, "if {1} {\n    set x 1\n}\n");
    }

    #[test]
    fn handles_nested_braces() {
        let src = "proc f {} {\nif {1} {\nset x 1\n}\n}\n";
        let out = format_source(src);
        assert_eq!(
            out,
            "proc f {} {\n    if {1} {\n        set x 1\n    }\n}\n",
        );
    }

    #[test]
    fn preserves_single_line_brace_blocks() {
        // `proc f {} {body}` is depth-neutral — the formatter
        // shouldn't add indentation to subsequent lines.
        let src = "proc f {} {body}\nset x 1\n";
        let out = format_source(src);
        assert_eq!(out, "proc f {} {body}\nset x 1\n");
    }

    #[test]
    fn ensures_trailing_newline() {
        let src = "set x 1";
        let out = format_source(src);
        assert_eq!(out, "set x 1\n");
    }

    #[test]
    fn empty_source_stays_empty() {
        assert_eq!(format_source(""), "");
    }

    #[test]
    fn brace_delta_ignores_string_contents() {
        assert_eq!(brace_delta(r#"set x "}{"; # comment"#), 0);
    }

    #[test]
    fn brace_delta_counts_nested() {
        assert_eq!(brace_delta("foo { bar { baz"), 2);
        assert_eq!(brace_delta("} }"), -2);
    }

    #[test]
    fn range_formatting_returns_whole_document_edit() {
        // For now the range variant falls back to a whole-
        // document format; the edit should be non-empty
        // for an unnormalised source.
        let src = "set x 1   \n";
        let edits = range_formatting(
            src,
            LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 0,
                end_character: 5,
            },
        );
        assert_eq!(edits.len(), 1, "{edits:?}");
    }
}
