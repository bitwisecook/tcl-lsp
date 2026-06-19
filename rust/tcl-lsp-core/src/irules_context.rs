//! Shared iRules `when`-context detection.
//!
//! Port of `lsp/features/irules_context.py::find_enclosing_when_event`
//! plus `core/.../namespace_data.py::scan_file_events`.  Both feed the
//! context-aware snippet templates (GAP-A9): `find_enclosing_when_event`
//! supplies the `current_event` top-level guard (event templates only
//! offer outside any `when` block) and `scan_file_events` supplies
//! `file_events` (event templates decline when their event is already
//! declared).
//!
//! Two deliberate divergences from the Python reference, both documented
//! here:
//!
//! * The conf-wrapped `embedded_rules` mode (scoping the search to the
//!   rule body containing the cursor in a BIG-IP `.conf` wrapper) is not
//!   modelled — the Rust core analyses the raw iRule body directly.
//! * `scan_file_events` walks the segmenter rather than Python's
//!   `\bwhen\s+([A-Z_]…)` regex (the project never parses Tcl with
//!   regex).  Both collect every `when EVENT` at any brace nesting; they
//!   differ only on the pathological case of the literal text `when X`
//!   sitting in a position that is *not* a command word (a comment or a
//!   non-script string), which the parse-accurate walk correctly skips.

use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::{LexerConfig, LineIndex, TokenType};

/// Find the enclosing `when EVENT { … }` event at `line` (0-based), or
/// `None` at the top level.  The innermost enclosing event wins (nested
/// `when` blocks shadow their parents), matching Python's recursive
/// `_scan_when_context`.
#[must_use]
pub fn find_enclosing_when_event(source: &str, line: u32, dialect: &str) -> Option<String> {
    if source.is_empty() {
        return None;
    }
    let line_index = LineIndex::new(source);
    scan_when_context(source, source, 0, line, dialect, &line_index)
}

/// Every distinct `when EVENT` name declared in `source` (uppercased,
/// sorted), at any brace nesting.  Mirrors `scan_file_events`.
#[must_use]
pub fn scan_file_events(source: &str, dialect: &str) -> Vec<String> {
    let mut events = Vec::new();
    collect_when_events(source, source, 0, dialect, &mut events);
    events.sort();
    events.dedup();
    events
}

/// Recursively search `text` (a slice of `full_source` starting at byte
/// `base`) for the `when` block whose braced body's line range contains
/// `cursor_line`, descending into nested bodies so the innermost match
/// wins.
fn scan_when_context(
    full_source: &str,
    text: &str,
    base: u32,
    cursor_line: u32,
    dialect: &str,
    li: &LineIndex,
) -> Option<String> {
    let mut best = None;
    let cmds =
        segment_commands_with_offset_and_config(text, base, LexerConfig::for_dialect(dialect));
    for cmd in &cmds {
        if cmd.texts.first().map(String::as_str) != Some("when") || cmd.texts.len() < 2 {
            continue;
        }
        // The braced body is the first `Str` argument word (barewords —
        // including the event name — are `Esc`).  Mirrors Python's
        // `next(tok for tok in arg_tokens if tok.type is STR)`.
        let Some(body_tok) = cmd.arg_tokens().iter().find(|t| t.kind == TokenType::Str) else {
            continue;
        };
        let start_line = li.line_at(body_tok.span.start());
        let end_line = li.line_at(body_tok.span.end());
        if cursor_line < start_line || cursor_line > end_line {
            continue;
        }
        best = Some(cmd.texts[1].to_uppercase());
        // Descend into the braced body for a deeper nested `when`.
        if let Some((inner, inner_base)) = brace_body(full_source, body_tok)
            && let Some(nested) =
                scan_when_context(full_source, inner, inner_base, cursor_line, dialect, li)
        {
            best = Some(nested);
        }
    }
    best
}

/// Walk every command in `text`, recording each `when EVENT`, and
/// recurse into every braced word so nested `when` blocks (and `when`
/// commands buried in other blocks) are all collected.
fn collect_when_events(full: &str, text: &str, base: u32, dialect: &str, out: &mut Vec<String>) {
    let cmds =
        segment_commands_with_offset_and_config(text, base, LexerConfig::for_dialect(dialect));
    for cmd in &cmds {
        if cmd.texts.first().map(String::as_str) == Some("when") && cmd.texts.len() >= 2 {
            out.push(cmd.texts[1].to_uppercase());
        }
        for tok in &cmd.argv {
            if tok.kind == TokenType::Str
                && let Some((inner, inner_base)) = brace_body(full, tok)
            {
                collect_when_events(full, inner, inner_base, dialect, out);
            }
        }
    }
}

/// Inner-content slice of a braced `Str` token plus its absolute base
/// offset, or `None` when the span is degenerate / out of bounds.  The
/// span starts at `{` (skipped via `content_offset`) and, for a
/// non-degenerate brace, ends just before the closing `}`.
fn brace_body<'a>(full: &'a str, tok: &tcl_lexer::Token) -> Option<(&'a str, u32)> {
    let inner_start = tok.span.start() + u32::from(tok.content_offset);
    let inner_end = tok.span.end();
    let (s, e) = (
        usize::try_from(inner_start).ok()?,
        usize::try_from(inner_end).ok()?,
    );
    if s > e || e > full.len() || !full.is_char_boundary(s) || !full.is_char_boundary(e) {
        return None;
    }
    Some((&full[s..e], inner_start))
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &str = "f5-irules";

    #[test]
    fn top_level_after_close_has_no_event() {
        let src = "when HTTP_REQUEST {\n    set x 1\n}\nset top 1\n";
        // Line 3 sits past the closing brace — back at the top level.
        assert_eq!(find_enclosing_when_event(src, 3, D), None);
    }

    #[test]
    fn when_open_line_counts_as_inside_body() {
        // The body Str token starts at the `{` on line 0, so a cursor on
        // the `when … {` line is already "inside" — matches Python's
        // `body_tok.start.line <= idx` containment check.
        let src = "when HTTP_REQUEST {\n    set x 1\n}\n";
        assert_eq!(
            find_enclosing_when_event(src, 0, D),
            Some("HTTP_REQUEST".to_string())
        );
    }

    #[test]
    fn cursor_inside_body_reports_event() {
        let src = "when HTTP_REQUEST {\n    set x 1\n}\n";
        assert_eq!(
            find_enclosing_when_event(src, 1, D),
            Some("HTTP_REQUEST".to_string())
        );
    }

    #[test]
    fn event_name_is_uppercased() {
        let src = "when http_request {\n    set x 1\n}\n";
        assert_eq!(
            find_enclosing_when_event(src, 1, D),
            Some("HTTP_REQUEST".to_string())
        );
    }

    #[test]
    fn innermost_nested_event_wins() {
        // `find_enclosing` descends only into `when` bodies (mirrors
        // Python's `_scan_when_context`), so the inner `when` is nested
        // directly inside the outer one.
        let src =
            "when HTTP_REQUEST {\n    when CLIENT_DATA {\n        set y 2\n    }\n    set x 1\n}\n";
        // Line 2 sits inside the nested CLIENT_DATA body.
        assert_eq!(
            find_enclosing_when_event(src, 2, D),
            Some("CLIENT_DATA".to_string())
        );
        // Line 4 is in the outer body, past the nested block's close.
        assert_eq!(
            find_enclosing_when_event(src, 4, D),
            Some("HTTP_REQUEST".to_string())
        );
    }

    #[test]
    fn scan_collects_all_events_sorted_deduped() {
        let src = "when HTTP_REQUEST {\n    set x 1\n}\nwhen RULE_INIT {\n    set y 2\n}\nwhen HTTP_REQUEST {\n    set z 3\n}\n";
        assert_eq!(
            scan_file_events(src, D),
            vec!["HTTP_REQUEST".to_string(), "RULE_INIT".to_string()]
        );
    }

    #[test]
    fn scan_finds_nested_events() {
        let src = "when HTTP_REQUEST {\n    if {1} {\n        when CLIENT_DATA { log local0. x }\n    }\n}\n";
        assert_eq!(
            scan_file_events(src, D),
            vec!["CLIENT_DATA".to_string(), "HTTP_REQUEST".to_string()]
        );
    }

    #[test]
    fn empty_source_is_top_level() {
        assert_eq!(find_enclosing_when_event("", 0, D), None);
        assert!(scan_file_events("", D).is_empty());
    }
}
