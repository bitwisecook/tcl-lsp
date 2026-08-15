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

use tcl_lexer::LineIndex;

/// Find the enclosing `when EVENT { … }` event at `line` (0-based), or
/// `None` at the top level.  The innermost enclosing event wins (nested
/// `when` blocks shadow their parents).
#[must_use]
pub fn find_enclosing_when_event(source: &str, line: u32, _dialect: &str) -> Option<String> {
    let line_index = LineIndex::new(source);
    tcl_irules::when_blocks_recursive(source)
        .into_iter()
        .filter(|block| {
            let start = line_index.line_at(block.span.start());
            let end = line_index.line_at(block.span.end());
            start <= line && line <= end
        })
        .min_by_key(|block| block.span.end() - block.span.start())
        .map(|block| block.event)
}

/// Every distinct `when EVENT` name declared in `source` (uppercased,
/// sorted), at any brace nesting.
#[must_use]
pub fn scan_file_events(source: &str, _dialect: &str) -> Vec<String> {
    let mut events: Vec<String> = tcl_irules::when_blocks_recursive(source)
        .into_iter()
        .map(|block| block.event)
        .collect();
    events.sort();
    events.dedup();
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &str = "f5-irules";

    #[test]
    fn inert_when_text_is_neither_context_nor_file_event() {
        let src = "set payload {when CLIENT_DATA {}}\nset q \"when SERVER_DATA {}\"\nwhen HTTP_REQUEST {}";
        assert_eq!(scan_file_events(src, D), ["HTTP_REQUEST"]);
        assert_eq!(find_enclosing_when_event(src, 0, D), None);
        assert_eq!(find_enclosing_when_event(src, 1, D), None);
    }

    #[test]
    fn semantic_case_and_lambda_regions_supply_file_events() {
        let src = "switch -- $x { a { when CLIENT_DATA {} } }\napply {{} { when HTTP_REQUEST {} }}";
        assert_eq!(scan_file_events(src, D), ["CLIENT_DATA", "HTTP_REQUEST"]);
    }

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
    fn rooted_nested_handlers_use_canonical_names_and_normalised_events() {
        let src = "::when http_request {\n  if {1} {\n    :::when client_data {\n      log local0. x\n    }\n  }\n}\n";
        assert_eq!(
            find_enclosing_when_event(src, 3, D),
            Some("CLIENT_DATA".to_string())
        );
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
