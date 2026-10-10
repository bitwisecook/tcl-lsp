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
//! * Discovery uses the registry's top-level script-boundary walker rather
//!   than a `\bwhen\s+([A-Z_]…)` regex (the project never parses Tcl with
//!   regex). It accepts only an offset-resolved command whose active dialect
//!   registry marks it as an event handler, so a user Tcl proc named `when`
//!   is never misclassified.

use tcl_lexer::LineIndex;

/// One request-local inventory of current source event-handler boundaries.
///
/// Compatibility constructors create one complete analysis; retained-analysis
/// callers reuse its source cards and script regions. These facts supply
/// authoring context, independently of event execution or reachability.
pub struct EventHandlerFacts {
    handlers: Vec<(String, tcl_lexer::Span)>,
    line_index: LineIndex,
}

impl EventHandlerFacts {
    /// Build resolved event-handler boundaries for one request/document and
    /// canonical profile.
    #[must_use]
    pub fn for_profile(source: &str, profile: &'static tcl_dialect::DialectProfile) -> Self {
        let handlers = if profile.is_irules() {
            build_event_handlers(source, profile)
        } else {
            Vec::new()
        };
        Self {
            handlers,
            line_index: LineIndex::new(source),
        }
    }

    /// Retain current source event cards and their genuine source-script placement.
    /// This view supplies authoring context only, without event reachability.
    pub(crate) fn from_analysis(
        source: &str,
        analysis: &tcl_compiler::analyser::AnalysisResult,
    ) -> Option<Self> {
        let config = analysis.body_lexer_config?;
        let image = tcl_lexer::SourceImage::document(source);
        analysis
            .matches_original_source_image(&image, config)
            .then_some(())?;
        let registry = analysis.resolved_registry()?;
        let generation = analysis.resolved_input.as_ref()?.context_registry();
        let structure =
            crate::source_structure::SourceStructure::capture(source, Some(analysis), config)?;
        let top_level = |span: tcl_lexer::Span| {
            structure
                .scripts
                .iter()
                .filter(|(region, _)| region.start() <= span.start() && span.end() <= region.end())
                .min_by_key(|(region, _)| region.end() - region.start())
                .is_some_and(|(_, depth)| *depth == 0)
        };
        let events = tcl_registry::events::EventRegistry::build();
        if analysis.allows_lexical_declaration_advice() {
            let realm = analysis.retained_command_realm()?;
            let handlers = tcl_syntax::event_handler::event_handlers_with_head_predicate(
                source,
                config,
                |head, offset| {
                    generation
                        .context()
                        .resolve_spec(registry, realm.resolve(head, offset).spec_name())
                        .is_some_and(|spec| {
                            spec.traits.contains(tcl_registry::Traits::IS_EVENT_HANDLER)
                        })
                },
            )
            .into_iter()
            .filter(|handler| {
                let arguments = handler
                    .arguments
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>();
                tcl_registry::events::IrulesDeclarationArguments::new(
                    &arguments,
                    &handler.argument_tokens,
                    &handler.argument_single_tokens,
                    &handler.argument_closed_braced_tokens,
                )
                .and_then(|arguments| registry.irules_event_declaration(arguments, &events))
                .is_some()
            })
            .map(|handler| (handler.event, handler.span))
            .collect();
            return Some(Self {
                handlers,
                line_index: LineIndex::new(source),
            });
        }
        let mut handlers = Vec::new();
        for declaration in analysis.original_vendor_symbol_declarations() {
            let metadata = declaration.metadata();
            let input = declaration.name_input();
            if metadata.kind != tcl_registry::DefinedSymbolKind::Event
                || !input.matches_source(&image, config)
                || !top_level(metadata.name_span)
            {
                continue;
            }
            let Some(command) = structure.commands.iter().find(|command| {
                command.argv.first().is_some_and(|head| {
                    head.span.start() == declaration.original_occurrence().site().offset
                })
            }) else {
                continue;
            };
            let Some(words) =
                crate::original_invocation::source_registry_words(source, analysis, command)
            else {
                continue;
            };
            if !words
                .with_source_schema(&generation, |schema| {
                    schema
                        .semantics
                        .traits
                        .contains(tcl_registry::Traits::IS_EVENT_HANDLER)
                })
                .unwrap_or(false)
                || !words
                    .operands
                    .iter()
                    .filter_map(Option::as_ref)
                    .any(|operand| operand.word.as_ref() == Some(input.original_word()))
            {
                continue;
            }
            let Some(units) = input.literal_units(declaration.purpose()) else {
                continue;
            };
            let Ok(event) = std::str::from_utf8(units) else {
                continue;
            };
            let event = event.to_ascii_uppercase();
            if events.is_known(&event) {
                handlers.push((event, metadata.full_span));
            }
        }
        for declaration in analysis.original_symbol_declarations() {
            let metadata = declaration.metadata();
            if metadata.kind != tcl_registry::DefinedSymbolKind::Event
                || !declaration.matches_source(&image, config)
                || !declaration.matches_registry(registry)
                || !top_level(declaration.span())
            {
                continue;
            }
            let Ok(event) = std::str::from_utf8(declaration.name_input().bytes()) else {
                continue;
            };
            let event = event.to_ascii_uppercase();
            if events.is_known(&event) {
                handlers.push((event, metadata.full_span));
            }
        }
        handlers.sort_by_key(|(_, span)| (span.start(), span.end()));
        handlers.dedup();
        Some(Self {
            handlers,
            line_index: LineIndex::new(source),
        })
    }

    /// Build resolved event-handler boundaries for one request/document.
    ///
    /// This name-based convenience entry point canonicalises once, then
    /// delegates all policy to [`DialectProfile::is_irules`].
    #[must_use]
    pub fn new(source: &str, dialect: &'static tcl_dialect::DialectProfile) -> Self {
        Self::for_profile(source, dialect)
    }

    /// The top-level event handler enclosing `line` (0-based), if any.
    #[must_use]
    pub fn enclosing_event(&self, line: u32) -> Option<String> {
        self.handlers
            .iter()
            .filter(|(_, span)| {
                let start = self.line_index.line_at(span.start());
                let end = self.line_index.line_at(span.end());
                start <= line && line <= end
            })
            .min_by_key(|(_, span)| span.end() - span.start())
            .map(|(event, _)| event.clone())
    }

    /// Every distinct resolved event name, uppercased and sorted.
    #[must_use]
    pub fn file_events(&self) -> Vec<String> {
        let mut events: Vec<String> = self
            .handlers
            .iter()
            .map(|(event, _)| event.clone())
            .collect();
        events.sort();
        events.dedup();
        events
    }
}

/// One fresh current analysis for the profile compatibility entry point.
fn build_event_handlers(
    source: &str,
    profile: &'static tcl_dialect::DialectProfile,
) -> Vec<(String, tcl_lexer::Span)> {
    #[cfg(test)]
    EXPENSIVE_BUILD_COUNT.with(|count| count.set(count.get() + 1));

    let generation = tcl_registry::model::ingress::context_for_profile(profile);
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        generation,
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let analysis = tcl_compiler::analyser::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name);
    EventHandlerFacts::from_analysis(source, &analysis)
        .map_or_else(Vec::new, |facts| facts.handlers)
}

#[cfg(test)]
thread_local! {
    static EXPENSIVE_BUILD_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn expensive_build_count() -> usize {
    EXPENSIVE_BUILD_COUNT.with(std::cell::Cell::get)
}

/// Find the enclosing `when EVENT { … }` event at `line` (0-based), or
/// `None` at the top level.
#[must_use]
pub fn find_enclosing_when_event(
    source: &str,
    line: u32,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Option<String> {
    EventHandlerFacts::new(source, dialect).enclosing_event(line)
}

/// Every distinct `when EVENT` name declared in `source` (uppercased,
/// sorted), at top level only.
#[must_use]
pub fn scan_file_events(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Vec<String> {
    EventHandlerFacts::new(source, dialect).file_events()
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &str = "f5-irules";

    #[test]
    fn original_event_compatibility_constructor_uses_current_source_cards() {
        // naming.core.original-snippet-source-context
        // docs/design/analysis/name-resolution-proofs/original-snippet-source-context.md
        let profile = tcl_registry::model::ingress::resolve_environment(D).analyser_profile();
        let source = "when HTTP_REQUEST {pool selected}";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, D);
        let retained = EventHandlerFacts::from_analysis(source, &analysis).unwrap();
        let fresh = EventHandlerFacts::for_profile(source, profile);
        assert_eq!(retained.file_events(), ["HTTP_REQUEST"]);
        assert_eq!(fresh.file_events(), retained.file_events());
        assert_eq!(fresh.enclosing_event(0), Some("HTTP_REQUEST".into()));
        assert!(EventHandlerFacts::from_analysis("when HTTP_REQUEST {}", &analysis).is_none());
    }

    #[test]
    fn inert_when_text_is_neither_context_nor_file_event() {
        let src = "set payload {when CLIENT_DATA {}}\nset q \"when SERVER_DATA {}\"\nwhen HTTP_REQUEST {}";
        assert_eq!(
            scan_file_events(
                src,
                tcl_registry::model::ingress::resolve_environment(D).analyser_profile()
            ),
            ["HTTP_REQUEST"]
        );
        assert_eq!(
            find_enclosing_when_event(
                src,
                0,
                tcl_registry::model::ingress::resolve_environment(D).analyser_profile()
            ),
            None
        );
        assert_eq!(
            find_enclosing_when_event(
                src,
                1,
                tcl_registry::model::ingress::resolve_environment(D).analyser_profile()
            ),
            None
        );
    }

    #[test]
    fn unknown_event_is_not_an_editor_execution_context() {
        let src = "when BOGUS_EVENT {\n  pool /Common/inert\n}\nwhen HTTP_REQUEST {}";
        assert_eq!(
            scan_file_events(
                src,
                tcl_registry::model::ingress::resolve_environment(D).analyser_profile()
            ),
            ["HTTP_REQUEST"]
        );
        assert_eq!(
            find_enclosing_when_event(
                src,
                1,
                tcl_registry::model::ingress::resolve_environment(D).analyser_profile()
            ),
            None
        );
    }

    #[test]
    fn event_inventory_shares_exact_handler_layout_validation() {
        let src = concat!(
            "when HTTP_REQUEST {} trailing\n",
            "when CLIENT_ACCEPTED pool\n",
            "when CLIENT_DATA timing on {}\n",
            "when SERVER_DATA priority 20 timing disable {}\n",
            "when HTTP_RESPONSE timing on priority 20 {}\n",
        );
        assert_eq!(
            scan_file_events(
                src,
                tcl_registry::model::ingress::resolve_environment(D).analyser_profile()
            ),
            ["CLIENT_DATA", "SERVER_DATA"]
        );
    }

    #[test]
    fn case_and_unavailable_apply_when_words_are_not_events() {
        let src = "switch -- $x { a { when CLIENT_DATA {} } }\napply {{} { when HTTP_REQUEST {} }}";
        assert!(
            scan_file_events(
                src,
                tcl_registry::model::ingress::resolve_environment(D).analyser_profile()
            )
            .is_empty()
        );
    }

    #[test]
    fn context_inventory_ignores_unavailable_irules_mutators() {
        let src = "interp alias {} event {} when\n\
                   rename when event\n\
                   event HTTP_REQUEST { set x 1 }\n\
                   ::when CLIENT_DATA { set y 1 }\n";
        assert_eq!(
            scan_file_events(
                src,
                tcl_registry::model::ingress::resolve_environment(D).analyser_profile()
            ),
            ["CLIENT_DATA"]
        );
        assert_eq!(
            find_enclosing_when_event(
                src,
                2,
                tcl_registry::model::ingress::resolve_environment(D).analyser_profile()
            ),
            None
        );
        assert_eq!(
            find_enclosing_when_event(
                src,
                3,
                tcl_registry::model::ingress::resolve_environment(D).analyser_profile()
            ),
            Some("CLIENT_DATA".to_owned())
        );
    }

    #[test]
    fn top_level_after_close_has_no_event() {
        let src = "when HTTP_REQUEST {\n    set x 1\n}\nset top 1\n";
        // Line 3 sits past the closing brace — back at the top level.
        assert_eq!(
            find_enclosing_when_event(
                src,
                3,
                tcl_registry::model::ingress::resolve_environment(D).analyser_profile()
            ),
            None
        );
    }

    #[test]
    fn when_open_line_counts_as_inside_body() {
        // The body Str token starts at the `{` on line 0, so a cursor on
        // the `when … {` line is already "inside".
        let src = "when HTTP_REQUEST {\n    set x 1\n}\n";
        assert_eq!(
            find_enclosing_when_event(
                src,
                0,
                tcl_registry::model::ingress::resolve_environment(D).analyser_profile()
            ),
            Some("HTTP_REQUEST".to_string())
        );
    }

    #[test]
    fn cursor_inside_body_reports_event() {
        let src = "when HTTP_REQUEST {\n    set x 1\n}\n";
        assert_eq!(
            find_enclosing_when_event(
                src,
                1,
                tcl_registry::model::ingress::resolve_environment(D).analyser_profile()
            ),
            Some("HTTP_REQUEST".to_string())
        );
    }

    #[test]
    fn event_name_is_uppercased() {
        let src = "when http_request {\n    set x 1\n}\n";
        assert_eq!(
            find_enclosing_when_event(
                src,
                1,
                tcl_registry::model::ingress::resolve_environment(D).analyser_profile()
            ),
            Some("HTTP_REQUEST".to_string())
        );
    }

    #[test]
    fn nested_when_stays_in_outer_event_context() {
        // `find_enclosing` descends only into `when` bodies, so the inner
        // `when` is nested directly inside the outer one.
        let src =
            "when HTTP_REQUEST {\n    when CLIENT_DATA {\n        set y 2\n    }\n    set x 1\n}\n";
        // Line 2 sits inside the nested CLIENT_DATA body.
        assert_eq!(
            find_enclosing_when_event(
                src,
                2,
                tcl_registry::model::ingress::resolve_environment(D).analyser_profile()
            ),
            Some("HTTP_REQUEST".to_string())
        );
        // Line 4 is in the outer body, past the nested block's close.
        assert_eq!(
            find_enclosing_when_event(
                src,
                4,
                tcl_registry::model::ingress::resolve_environment(D).analyser_profile()
            ),
            Some("HTTP_REQUEST".to_string())
        );
    }

    #[test]
    fn scan_collects_all_events_sorted_deduped() {
        let src = "when HTTP_REQUEST {\n    set x 1\n}\nwhen RULE_INIT {\n    set y 2\n}\nwhen HTTP_REQUEST {\n    set z 3\n}\n";
        assert_eq!(
            scan_file_events(
                src,
                tcl_registry::model::ingress::resolve_environment(D).analyser_profile()
            ),
            vec!["HTTP_REQUEST".to_string(), "RULE_INIT".to_string()]
        );
    }

    #[test]
    fn scan_ignores_nested_when_words() {
        let src = "when HTTP_REQUEST {\n    if {1} {\n        when CLIENT_DATA { log local0. x }\n    }\n}\n";
        assert_eq!(
            scan_file_events(
                src,
                tcl_registry::model::ingress::resolve_environment(D).analyser_profile()
            ),
            vec!["HTTP_REQUEST".to_string()]
        );
    }

    #[test]
    fn rooted_nested_when_stays_in_outer_event_context() {
        let src = "::when http_request {\n  if {1} {\n    :::when client_data {\n      log local0. x\n    }\n  }\n}\n";
        assert_eq!(
            find_enclosing_when_event(
                src,
                3,
                tcl_registry::model::ingress::resolve_environment(D).analyser_profile()
            ),
            Some("HTTP_REQUEST".to_string())
        );
        assert_eq!(
            scan_file_events(
                src,
                tcl_registry::model::ingress::resolve_environment(D).analyser_profile()
            ),
            vec!["HTTP_REQUEST".to_string()]
        );
    }

    #[test]
    fn empty_source_is_top_level() {
        assert_eq!(
            find_enclosing_when_event(
                "",
                0,
                tcl_registry::model::ingress::resolve_environment(D).analyser_profile()
            ),
            None
        );
        assert!(
            scan_file_events(
                "",
                tcl_registry::model::ingress::resolve_environment(D).analyser_profile()
            )
            .is_empty()
        );
    }

    /// Top-level boundary discovery must stay bounded on pathologically deep
    /// inert nested text. It is reached
    /// by completion/code-actions on essentially every keystroke. The
    /// assertion is that both request-local views return, not their contents.
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

        let _ = find_enclosing_when_event(
            &src,
            1,
            tcl_registry::model::ingress::resolve_environment(D).analyser_profile(),
        );
        let _ = scan_file_events(
            &src,
            tcl_registry::model::ingress::resolve_environment(D).analyser_profile(),
        );
    }

    #[test]
    fn one_fact_inventory_serves_context_and_file_events() {
        let before = expensive_build_count();
        let facts = EventHandlerFacts::for_profile(
            "when HTTP_REQUEST { set x 1 }",
            tcl_dialect::DialectProfile::irules(),
        );
        assert_eq!(facts.enclosing_event(0), Some("HTTP_REQUEST".to_owned()));
        assert_eq!(facts.file_events(), ["HTTP_REQUEST"]);
        assert_eq!(
            expensive_build_count(),
            before + 1,
            "one request-local inventory performs one identity/boundary build"
        );
    }

    #[test]
    fn canonical_irules_profile_and_aliases_build_event_facts() {
        for dialect in ["f5-irules", "irules", "tcl-irule"] {
            let facts = EventHandlerFacts::new(
                "when HTTP_REQUEST {}",
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            );
            assert_eq!(facts.file_events(), ["HTTP_REQUEST"], "{dialect}");
        }
        let tcl = EventHandlerFacts::new(
            "when HTTP_REQUEST {}",
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        assert!(tcl.file_events().is_empty());
    }
}
