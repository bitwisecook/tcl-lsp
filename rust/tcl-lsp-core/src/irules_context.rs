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
//! [`EventHandlerFacts`] feeds the context-aware snippet templates:
//! `enclosing_event` supplies the `current_event` top-level guard (event
//! templates only offer outside any `when` block) and `file_events` supplies
//! `file_events` (event templates decline when their event is already
//! declared). The compatibility helpers below each construct a short-lived
//! inventory, while callers that need both facts reuse one inventory.
//!
//! Two deliberate design choices, both documented here:
//!
//! * The conf-wrapped `embedded_rules` mode (scoping the search to the
//!   rule body containing the cursor in a BIG-IP `.conf` wrapper) is not
//!   modelled — the raw iRule body is analysed directly.
//! * Discovery uses the registry's recursive script-boundary walker rather
//!   than a `\bwhen\s+([A-Z_]…)` regex (the project never parses Tcl with
//!   regex). It accepts only an offset-resolved command whose active dialect
//!   registry marks it as an event handler, so a user Tcl proc named `when`
//!   is never misclassified.

use tcl_lexer::LineIndex;

/// One request-local inventory of resolved iRules event-handler boundaries.
///
/// The constructor builds the document's [`HeadIdentityMap`]
/// (`tcl_compiler::head_identity::HeadIdentityMap`) exactly once, then hands
/// it to the registry's recursive boundary owner. Callers can derive both the
/// enclosing event and the deduplicated file inventory without a global cache
/// or a second identity scan.
pub struct EventHandlerFacts {
    handlers: Vec<tcl_irules::WhenBlock>,
    line_index: LineIndex,
}

impl EventHandlerFacts {
    /// Build resolved event-handler boundaries for one request/document.
    #[must_use]
    pub fn new(source: &str, dialect: &str) -> Self {
        #[cfg(test)]
        FACT_BUILD_COUNT.with(|count| count.set(count.get() + 1));

        let handlers = if dialect == "f5-irules" {
            let config = tcl_lexer::LexerConfig::for_file_dialect(dialect);
            let registry = tcl_registry::registry_for_dialect(dialect);
            let identities = tcl_compiler::head_identity::command_head_identities_with_config(
                source, config, registry,
            );
            tcl_registry::events::recursive_when_handlers_with_registry_and_head_resolver(
                source,
                registry,
                &identities,
            )
        } else {
            Vec::new()
        };
        Self {
            handlers,
            line_index: LineIndex::new(source),
        }
    }

    /// The innermost event handler enclosing `line` (0-based), if any.
    #[must_use]
    pub fn enclosing_event(&self, line: u32) -> Option<String> {
        self.handlers
            .iter()
            .filter(|block| {
                let start = self.line_index.line_at(block.span.start());
                let end = self.line_index.line_at(block.span.end());
                start <= line && line <= end
            })
            .min_by_key(|block| block.span.end() - block.span.start())
            .map(|block| block.event.clone())
    }

    /// Every distinct resolved event name, uppercased and sorted.
    #[must_use]
    pub fn file_events(&self) -> Vec<String> {
        let mut events: Vec<String> = self
            .handlers
            .iter()
            .map(|block| block.event.clone())
            .collect();
        events.sort();
        events.dedup();
        events
    }
}

#[cfg(test)]
thread_local! {
    static FACT_BUILD_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Find the enclosing `when EVENT { … }` event at `line` (0-based), or
/// `None` at the top level.  The innermost enclosing event wins (nested
/// `when` blocks shadow their parents).
#[must_use]
pub fn find_enclosing_when_event(source: &str, line: u32, dialect: &str) -> Option<String> {
    EventHandlerFacts::new(source, dialect).enclosing_event(line)
}

/// Every distinct `when EVENT` name declared in `source` (uppercased,
/// sorted), at any brace nesting.
#[must_use]
pub fn scan_file_events(source: &str, dialect: &str) -> Vec<String> {
    EventHandlerFacts::new(source, dialect).file_events()
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
    fn context_inventory_uses_resolved_event_handler_identity() {
        let src = "interp alias {} event {} when\n\
                   event HTTP_REQUEST { set x 1 }\n\
                   proc when {args} {}\n\
                   when CLIENT_DATA { set y 1 }\n";
        assert_eq!(scan_file_events(src, D), ["HTTP_REQUEST"]);
        assert_eq!(
            find_enclosing_when_event(src, 1, D),
            Some("HTTP_REQUEST".to_owned())
        );
        assert_eq!(find_enclosing_when_event(src, 3, D), None);
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

    #[test]
    fn one_fact_inventory_serves_context_and_file_events() {
        let before = FACT_BUILD_COUNT.with(std::cell::Cell::get);
        let facts = EventHandlerFacts::new("when HTTP_REQUEST { set x 1 }", D);
        assert_eq!(facts.enclosing_event(0), Some("HTTP_REQUEST".to_owned()));
        assert_eq!(facts.file_events(), ["HTTP_REQUEST"]);
        assert_eq!(
            FACT_BUILD_COUNT.with(std::cell::Cell::get),
            before + 1,
            "one request-local inventory performs one identity/boundary build"
        );
    }
}
