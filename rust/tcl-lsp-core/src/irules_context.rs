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

//! Shared iRules `when`-context detection.
//!
//! Both `find_enclosing_when_event` and `scan_file_events` feed the
//! context-aware snippet templates: `find_enclosing_when_event`
//! supplies the `current_event` top-level guard (event templates only
//! offer outside any `when` block) and `scan_file_events` supplies
//! `file_events` (event templates decline when their event is already
//! declared).
//!
//! Two deliberate design choices, both documented here:
//!
//! * The conf-wrapped `embedded_rules` mode (scoping the search to the
//!   rule body containing the cursor in a BIG-IP `.conf` wrapper) is not
//!   modelled — the raw iRule body is analysed directly.
//! * `scan_file_events` walks the segmenter rather than a
//!   `\bwhen\s+([A-Z_]…)` regex (the project never parses Tcl with
//!   regex).  Both collect every `when EVENT` at any brace nesting; they
//!   differ only on the pathological case of the literal text `when X`
//!   sitting in a position that is *not* a command word (a comment or a
//!   non-script string), which the parse-accurate walk correctly skips.

use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_core_types::RecursionLimit;
use tcl_lexer::{LexerConfig, LineIndex, TokenType};

/// Cap on brace-nesting depth [`scan_when_context`]/[`collect_when_events`]
/// will descend into, mirroring the compiler analyser's `MAX_BODY_DEPTH` so
/// deeply (but validly) nested code still resolves `when`-context / event
/// lists correctly while pathological/adversarial nesting can't blow the
/// native stack — see
/// `docs/design/compiler/recursive-descent-depth-limits.md`.
const MAX_WHEN_SCAN_DEPTH: RecursionLimit = RecursionLimit(256);

/// Find the enclosing `when EVENT { … }` event at `line` (0-based), or
/// `None` at the top level.  The innermost enclosing event wins (nested
/// `when` blocks shadow their parents).
#[must_use]
pub fn find_enclosing_when_event(source: &str, line: u32, dialect: &str) -> Option<String> {
    if source.is_empty() {
        return None;
    }
    let line_index = LineIndex::new(source);
    scan_when_context(source, source, 0, line, dialect, &line_index, 0)
}

/// Every distinct `when EVENT` name declared in `source` (uppercased,
/// sorted), at any brace nesting.
#[must_use]
pub fn scan_file_events(source: &str, dialect: &str) -> Vec<String> {
    let mut events = Vec::new();
    collect_when_events(source, source, 0, dialect, &mut events, 0);
    events.sort();
    events.dedup();
    events
}

/// Recursively search `text` (a slice of `full_source` starting at byte
/// `base`) for the `when` block whose braced body's line range contains
/// `cursor_line`, descending into nested bodies so the innermost match
/// wins.
///
/// `depth` is the nesting level of this call (0 at the top); past
/// [`MAX_WHEN_SCAN_DEPTH`] this stops descending into nested bodies rather
/// than recursing further.
fn scan_when_context(
    full_source: &str,
    text: &str,
    base: u32,
    cursor_line: u32,
    dialect: &str,
    li: &LineIndex,
    depth: u32,
) -> Option<String> {
    let mut best = None;
    let cmds = segment_commands_with_offset_and_config(
        text,
        base,
        LexerConfig::for_file_dialect(dialect).at_depth(depth),
    );
    for cmd in &cmds {
        if cmd.texts.first().map(String::as_str) != Some("when") || cmd.texts.len() < 2 {
            continue;
        }
        // The braced body is the first `Str` argument word (barewords —
        // including the event name — are `Esc`).
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
        if !MAX_WHEN_SCAN_DEPTH.exceeded(depth)
            && let Some((inner, inner_base)) = brace_body(full_source, body_tok)
            && let Some(nested) = scan_when_context(
                full_source,
                inner,
                inner_base,
                cursor_line,
                dialect,
                li,
                depth + 1,
            )
        {
            best = Some(nested);
        }
    }
    best
}

/// Walk every command in `text`, recording each `when EVENT`, and
/// recurse into every braced word so nested `when` blocks (and `when`
/// commands buried in other blocks) are all collected.
///
/// `depth` is the nesting level of this call (0 at the top); past
/// [`MAX_WHEN_SCAN_DEPTH`] this stops descending into nested bodies rather
/// than recursing further.
fn collect_when_events(
    full: &str,
    text: &str,
    base: u32,
    dialect: &str,
    out: &mut Vec<String>,
    depth: u32,
) {
    if MAX_WHEN_SCAN_DEPTH.exceeded(depth) {
        return;
    }
    let cmds = segment_commands_with_offset_and_config(
        text,
        base,
        LexerConfig::for_file_dialect(dialect).at_depth(depth),
    );
    for cmd in &cmds {
        if cmd.texts.first().map(String::as_str) == Some("when") && cmd.texts.len() >= 2 {
            out.push(cmd.texts[1].to_uppercase());
        }
        for tok in &cmd.argv {
            if tok.kind == TokenType::Str
                && let Some((inner, inner_base)) = brace_body(full, tok)
            {
                collect_when_events(full, inner, inner_base, dialect, out, depth + 1);
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
        // the `when … {` line is already "inside".
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
        // `find_enclosing` descends only into `when` bodies, so the inner
        // `when` is nested directly inside the outer one.
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

    /// Regression coverage for issue #996: `scan_when_context` and
    /// `collect_when_events` recurse once per nested braced body, with no
    /// depth cap before this fix. Reachable from completion/code-actions
    /// on essentially every keystroke, so pathologically deep `when`/`if`
    /// nesting must not crash the server. The assertion is that both
    /// return at all, not what they return.
    #[test]
    fn deeply_nested_bodies_survive_when_scanning() {
        const DEPTH: usize = 2000;
        let mut src = String::from("when HTTP_REQUEST {\n");
        for _ in 0..DEPTH {
            src.push_str("if {1} {\n");
        }
        src.push_str("when CLIENT_DATA { log local0. deep }\n");
        for _ in 0..DEPTH {
            src.push_str("}\n");
        }
        src.push_str("}\n");

        let _ = find_enclosing_when_event(&src, 1, D);
        let _ = scan_file_events(&src, D);
    }
}
