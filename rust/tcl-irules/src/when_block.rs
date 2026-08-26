// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! iRules `when EVENT { … }` boundary discovery.

use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::LexerConfig;

/// One syntactically complete `when EVENT { … }` handler.
pub type WhenBlock = tcl_syntax::event_handler::EventHandler;

/// Return complete iRules event handlers in source order.
///
/// This is the iRules-configured wrapper around the dependency-low shared
/// syntax owner. The lexer rejects commented-out headers, keeps quoted and
/// braced data intact, and uses the iRules brace-separator grammar. Consumers
/// that need handler bodies must use this entry point.
#[must_use]
pub fn when_blocks(source: &str) -> Vec<WhenBlock> {
    let config = LexerConfig::for_file_dialect("f5-irules");
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let identities =
        tcl_compiler::head_identity::command_head_identities_with_config(source, config, registry);
    tcl_registry::events::top_level_when_handlers_with_registry_and_head_resolver(
        source,
        registry,
        &identities,
    )
}

/// Return syntactically valid top-level handler candidates, including unknown
/// event names, for the unknown-event diagnostic only.
#[must_use]
pub fn when_block_candidates(source: &str) -> Vec<WhenBlock> {
    let config = LexerConfig::for_file_dialect("f5-irules");
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let identities =
        tcl_compiler::head_identity::command_head_identities_with_config(source, config, registry);
    tcl_registry::events::top_level_when_handler_candidates_with_registry_and_head_resolver(
        source,
        registry,
        &identities,
    )
}

/// Whether a discovered handler body contains no executable command.  This
/// intentionally delegates comment classification to the iRules lexer.
#[must_use]
pub fn when_block_is_empty(source: &str, block: &WhenBlock) -> bool {
    source.get(block.body_span.as_range()).is_some_and(|body| {
        segment_commands_with_offset_and_config(body, 0, LexerConfig::for_file_dialect("f5-irules"))
            .is_empty()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_commented_when_and_rejects_trailing_text_after_brace_close() {
        let source =
            "# when HTTP_REQUEST { ignored }\nwhen RULE_INIT { log local0. \"legacy } path\"\n}\n";
        let blocks = when_blocks(source);
        assert!(blocks.is_empty());
    }

    #[test]
    fn recognises_canonical_global_when_spellings() {
        let source =
            "::when HTTP_REQUEST { pool /Common/a }\n:::when RULE_INIT { pool /Common/b }\n";
        let blocks = when_blocks(source);
        assert_eq!(
            blocks
                .iter()
                .map(|block| block.event.as_str())
                .collect::<Vec<_>>(),
            ["HTTP_REQUEST", "RULE_INIT"]
        );
    }

    #[test]
    fn unavailable_irules_mutators_do_not_change_when_identity() {
        // F5 K36322151: iRules disables `interp`, `rename`, and `namespace`.
        // These Tcl-looking statements therefore cannot alias, move, or import
        // the event-handler command.
        let source = "interp alias {} event {} when\n\
                      rename when event\n\
                      namespace import ::x::*\n\
                      event HTTP_REQUEST {}\n\
                      ::when CLIENT_DATA {}\n";
        let blocks = when_blocks(source);
        assert_eq!(
            blocks
                .iter()
                .map(|block| block.event.as_str())
                .collect::<Vec<_>>(),
            ["CLIENT_DATA"],
            "only the rooted, registry-declared iRules event handler is valid"
        );
    }

    #[test]
    fn a_rebound_when_spelling_is_not_an_event_handler() {
        let source = "when HTTP_REQUEST {}\n\
                      proc when {args} {}\n\
                      when CLIENT_DATA {}\n";
        let blocks = when_blocks(source);
        assert_eq!(
            blocks
                .iter()
                .map(|block| block.event.as_str())
                .collect::<Vec<_>>(),
            ["HTTP_REQUEST"]
        );
    }

    #[test]
    fn nested_when_is_not_an_irules_event_handler() {
        let source =
            "when http_request {\n  if {1} {\n    :::when client_data { pool p }\n  }\n}\n";
        let blocks = when_blocks(source);
        assert_eq!(
            blocks
                .iter()
                .map(|block| block.event.as_str())
                .collect::<Vec<_>>(),
            ["HTTP_REQUEST"]
        );
    }

    #[test]
    fn nested_when_remains_the_only_valid_event_after_unavailable_aliases() {
        let source = "interp alias {} branch {} if\n\
                      interp alias {} event {} ::when\n\
                      branch {1} { ::event HTTP_REQUEST {} }\n\
                      if {1} { ::when HTTP_REQUEST {} }\n\
                      ::when CLIENT_DATA {}\n";
        let blocks = when_blocks(source);
        assert_eq!(
            blocks
                .iter()
                .map(|block| block.event.as_str())
                .collect::<Vec<_>>(),
            ["CLIENT_DATA"],
            "only a top-level rooted handler is valid"
        );

        let rebound = "proc if {args} {}\nif {1} { when CLIENT_DATA {} }\n";
        assert!(
            when_blocks(rebound).is_empty(),
            "a user command named `if` does not inherit the registry body grammar"
        );
    }

    #[test]
    fn top_level_discovery_does_not_execute_braced_or_quoted_data() {
        let source = r#"set payload {when CLIENT_DATA {}}
set quoted "when SERVER_DATA {}"
# when RULE_INIT {}
when HTTP_REQUEST {}"#;
        assert_eq!(
            when_blocks(source)
                .iter()
                .map(|block| block.event.as_str())
                .collect::<Vec<_>>(),
            ["HTTP_REQUEST"]
        );
    }

    #[test]
    fn nested_case_and_unavailable_apply_when_words_are_not_events() {
        let source = r"
switch -- $x { a { when CLIENT_DATA {} } }
apply {{} { when HTTP_REQUEST {} }}
set inert {when RULE_INIT {}}
";
        assert_eq!(
            when_blocks(source)
                .iter()
                .map(|block| block.event.as_str())
                .collect::<Vec<_>>(),
            Vec::<&str>::new()
        );
    }
}
