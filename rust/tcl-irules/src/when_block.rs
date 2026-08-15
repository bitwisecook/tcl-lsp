// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! iRules `when EVENT { … }` boundary discovery.

use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::LexerConfig;
use tcl_syntax::event_handler::{EventHandlerTraversal, event_handlers};

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
    event_handlers(
        source,
        LexerConfig::for_file_dialect("f5-irules"),
        EventHandlerTraversal::TopLevel,
    )
}

/// Return handlers recursively nested inside handler bodies, with spans kept
/// absolute to `source`. This is for editor context; [`when_blocks`] remains
/// the top-level execution surface.
#[must_use]
pub fn when_blocks_recursive(source: &str) -> Vec<WhenBlock> {
    event_handlers(
        source,
        LexerConfig::for_file_dialect("f5-irules"),
        EventHandlerTraversal::Recursive,
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
    fn ignores_commented_when_and_keeps_quoted_close_brace_in_tcl_body() {
        let source =
            "# when HTTP_REQUEST { ignored }\nwhen RULE_INIT { log local0. \"legacy } path\"\n}\n";
        let blocks = when_blocks(source);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].event, "RULE_INIT");
        assert_eq!(
            &source[blocks[0].body_span.as_range()],
            " log local0. \"legacy "
        );
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
    fn recursively_finds_nested_rooted_handlers_with_absolute_spans() {
        let source =
            "when http_request {\n  if {1} {\n    :::when client_data { pool p }\n  }\n}\n";
        let blocks = when_blocks_recursive(source);
        assert_eq!(
            blocks
                .iter()
                .map(|block| block.event.as_str())
                .collect::<Vec<_>>(),
            ["HTTP_REQUEST", "CLIENT_DATA"]
        );
        assert_eq!(
            &source[blocks[1].span.as_range()],
            ":::when client_data { pool p }"
        );
        assert_eq!(&source[blocks[1].body_span.as_range()], " pool p ");
    }
}
