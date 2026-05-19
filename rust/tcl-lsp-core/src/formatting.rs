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
/// document.
///
/// True range-aware formatting: only the line slice
/// `[range.start_line, range.end_line]` (extended to whole
/// lines) is re-normalised, with the brace depth at the
/// start of the slice computed from the source prefix above
/// it.  Emits a single `TextEdit` that replaces the slice
/// with its formatted form, or an empty `Vec` when the
/// slice is already normalised.
///
/// Range-formatting only touches the line range requested;
/// edits outside it are left untouched.  Editors that
/// invoke `textDocument/rangeFormatting` (eg. `format
/// selection`) only need the selected slice to change.
#[must_use]
pub fn range_formatting(source: &str, range: LspRange) -> Vec<TextEdit> {
    let lines: Vec<&str> = source.split('\n').collect();
    if lines.is_empty() {
        return Vec::new();
    }
    let line_count = u32::try_from(lines.len()).unwrap_or(u32::MAX);
    let start_line = range.start_line.min(line_count.saturating_sub(1));
    let end_line = range
        .end_line
        .min(line_count.saturating_sub(1))
        .max(start_line);

    // Brace depth at the start of `start_line` — count
    // running `{` / `}` over every line before it.  Matches
    // the same string / comment skip rules `brace_delta`
    // uses inside the formatter.
    let mut prefix_depth: i32 = 0;
    for prior in lines.iter().take(start_line as usize) {
        prefix_depth = (prefix_depth + brace_delta(prior)).max(0);
    }

    // Slice of lines we re-format.
    let slice_end = (end_line as usize) + 1;
    let slice_lines: Vec<&str> = lines[start_line as usize..slice_end].to_vec();
    let formatted_slice = format_lines(&slice_lines, prefix_depth);
    let original_slice = slice_lines.join("\n");
    // The slice has no trailing newline (line ranges are
    // half-open in LSP, but our split here gave us all the
    // lines `start_line..=end_line`).  The formatter appends
    // a trailing newline.  Compare against the joined slice
    // plus the formatter's expected trailing newline to skip
    // edits when nothing changed.
    let original_with_nl = if formatted_slice.ends_with('\n') {
        format!("{original_slice}\n")
    } else {
        original_slice.clone()
    };
    if formatted_slice == original_with_nl {
        return Vec::new();
    }

    // Replacement range covers the full slice, line-anchored
    // (column 0 of `start_line` to column 0 of the line
    // *after* `end_line`).  When `end_line` is the last line
    // of the document, anchor the end at the post-final-char
    // position so editors interpret the edit correctly.
    let edit_range = if (end_line + 1) < line_count {
        LspRange {
            start_line,
            start_character: 0,
            end_line: end_line + 1,
            end_character: 0,
        }
    } else {
        let last_line_len =
            u32::try_from(lines[end_line as usize].chars().count()).unwrap_or(u32::MAX);
        LspRange {
            start_line,
            start_character: 0,
            end_line,
            end_character: last_line_len,
        }
    };
    vec![TextEdit {
        range: edit_range,
        new_text: formatted_slice,
    }]
}

/// Format an explicit line slice, given the brace depth at
/// the start of the slice.  Shared core between
/// [`format_source`] (depth 0, all lines) and
/// [`range_formatting`] (depth from prefix walk, partial
/// slice).
fn format_lines(lines: &[&str], initial_depth: i32) -> String {
    let mut out = String::new();
    let mut depth = initial_depth;
    let mut prev_blank = false;
    for line in lines {
        let trimmed = line.trim_end();
        let stripped = trimmed.trim_start();
        if stripped.is_empty() {
            if !prev_blank && !out.is_empty() {
                out.push('\n');
                prev_blank = true;
            }
            continue;
        }
        prev_blank = false;
        let leading_close = leading_close_braces(stripped);
        let line_depth = (depth - leading_close).max(0);
        for _ in 0..line_depth {
            out.push_str("    ");
        }
        out.push_str(stripped);
        out.push('\n');
        depth = (depth + brace_delta(stripped)).max(0);
    }
    out
}

/// Whitespace-normalise `source`.  Pure function so tests
/// can exercise the algorithm without invoking the LSP
/// edit-emission layer.
#[must_use]
pub fn format_source(source: &str) -> String {
    let lines: Vec<&str> = source.split('\n').collect();
    // The final `split('\n')` element is the empty string
    // after a trailing newline — drop it so we emit exactly
    // one trailing newline at the end.
    let trailing_empty = lines.last().is_some_and(|s| s.is_empty());
    let effective: &[&str] = if trailing_empty {
        &lines[..lines.len() - 1]
    } else {
        &lines[..]
    };
    let mut out = format_lines(effective, 0);
    // Ensure exactly one trailing newline when the source
    // had any content at all — `format_lines` always appends
    // a newline after the last non-blank line, but an
    // entirely blank input produces an empty string.
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
    fn range_formatting_emits_edit_for_dirty_range() {
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
        assert!(edits[0].new_text.contains("set x 1"), "{edits:?}");
        // Trailing whitespace stripped.
        assert!(!edits[0].new_text.contains("   "), "{edits:?}");
    }

    #[test]
    fn range_formatting_no_edits_when_slice_is_clean() {
        // Whole document is already formatted — range over a
        // clean slice should emit no edits.
        let src = "proc foo {} {\n    set x 1\n}\n";
        let edits = range_formatting(
            src,
            LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 2,
                end_character: 0,
            },
        );
        assert!(edits.is_empty(), "{edits:?}");
    }

    #[test]
    fn range_formatting_preserves_brace_depth_from_prefix() {
        // Inside a proc body, the lines should be indented
        // 4 spaces.  Format only line 1 (the body's `set x`
        // line) — the formatter must pick up `depth = 1`
        // from the prefix walk.
        let src = "proc foo {} {\nset x 1\n}\n";
        let edits = range_formatting(
            src,
            LspRange {
                start_line: 1,
                start_character: 0,
                end_line: 1,
                end_character: 100,
            },
        );
        assert_eq!(edits.len(), 1, "{edits:?}");
        // Inside the proc body — should be indented 4 spaces.
        assert!(
            edits[0].new_text.starts_with("    set x 1"),
            "expected indented set; got {:?}",
            edits[0].new_text,
        );
    }

    #[test]
    fn range_formatting_clamps_end_at_eof() {
        // Source has 2 lines; request a range whose end
        // extends past EOF.  Should still emit one valid
        // edit anchored at the final line's end.
        let src = "set x 1   \nset y 2\n";
        let edits = range_formatting(
            src,
            LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 99,
                end_character: 0,
            },
        );
        assert_eq!(edits.len(), 1, "{edits:?}");
    }
}
