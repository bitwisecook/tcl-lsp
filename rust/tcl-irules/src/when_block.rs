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
    let Some(identities) = fresh_irules_realm(source, config, registry) else {
        return Vec::new();
    };
    tcl_registry::events::top_level_when_handlers_with_registry_and_head_resolver(
        source,
        registry,
        &identities,
    )
}

/// Readonly original source event declarations under the actual iRules store.
/// Unknown lookup, namespace, observer and source mutation premises stay on the
/// retained cards. Unavailable or partial source coverage is never empty advice.
#[must_use]
pub fn when_source_inventory(source: &str) -> crate::IrulesWhenSourceInventory {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    crate::OriginalIrulesSourceContext::capture(source, registry)
        .map_or(crate::IrulesWhenSourceInventory::Unavailable, |context| {
            context.when_source_inventory(source)
        })
}

/// Return syntactically valid top-level handler candidates, including unknown
/// event names, for the unknown-event diagnostic only.
#[must_use]
pub fn when_block_candidates(source: &str) -> Vec<WhenBlock> {
    let config = LexerConfig::for_file_dialect("f5-irules");
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let Some(identities) = fresh_irules_realm(source, config, registry) else {
        return Vec::new();
    };
    tcl_registry::events::top_level_when_handler_candidates_with_registry_and_head_resolver(
        source,
        registry,
        &identities,
    )
}

fn fresh_irules_realm(
    source: &str,
    config: LexerConfig,
    registry: &tcl_registry::CommandRegistry,
) -> Option<tcl_compiler::realm::CommandBindingRealm> {
    let profile = registry
        .profile()
        .unwrap_or_else(tcl_dialect::DialectProfile::irules);
    let context = std::sync::Arc::new(
        tcl_registry::model::ingress::context_for_profile(profile)
            .with_command_store(registry.snapshot().shared_registry()),
    );
    let input =
        tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
    tcl_compiler::realm::document_realm_bindings_for_fresh_hosted_source(source, &input)
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
    // naming.consumer.original-irules-source-context
    // docs/design/analysis/name-resolution-proofs/original-irules-source-context.md
    fn original_when_inventory_keeps_event_words_bodies_and_conditional_cards() {
        let source =
            "when HTTP_REQUEST { pool /Common/a }\nwhen CLIENT_ACCEPTED { log local0. ok }";
        let crate::IrulesWhenSourceInventory::Available(inventory) = when_source_inventory(source)
        else {
            panic!("actual complete source inventory");
        };
        assert!(inventory.is_complete());
        assert_eq!(
            inventory
                .declarations()
                .iter()
                .map(super::super::source_context::OriginalIrulesWhenDeclaration::event)
                .collect::<Vec<_>>(),
            ["HTTP_REQUEST", "CLIENT_ACCEPTED"]
        );
        for declaration in inventory.declarations() {
            let tcl_compiler::registry_invocation::source_structure::OriginalRegistrySource::Vendor(
                metadata,
            ) = declaration.words().source()
            else {
                panic!("original hosted source card")
            };
            assert!(metadata.obligations().contains(
                &tcl_compiler::command_binding::VendorCatalogueSourceObligation::AuthoredSourceCoordinates));
            assert_eq!(
                metadata.shape().original_words().get(1),
                declaration.words().original_argument_word(0)
            );
            assert!(metadata.matches_source(
                &tcl_lexer::SourceImage::document(source),
                inventory.source_context().expression_syntax().0
            ));
        }
        assert!(matches!(
            inventory.source_context().when_source_inventory(
                "when HTTP_RESPONSE { pool /Common/a }\nwhen CLIENT_ACCEPTED { log local0. ok }"
            ),
            crate::IrulesWhenSourceInventory::Unavailable
        ));
    }

    #[test]
    // naming.consumer.original-irules-source-context
    // docs/design/analysis/name-resolution-proofs/original-irules-source-context.md
    fn original_when_inventory_distinguishes_empty_and_unavailable_source_sites() {
        for source in ["", "# no source commands\n", "set value 1"] {
            let crate::IrulesWhenSourceInventory::Available(inventory) =
                when_source_inventory(source)
            else {
                panic!("owned source inventory: {source:?}");
            };
            assert!(inventory.is_complete(), "{source:?}");
            assert!(inventory.declarations().is_empty());
        }
        for source in [
            "when HTTP_REQUEST {",
            "proc when args {}; when HTTP_REQUEST {}",
        ] {
            if let crate::IrulesWhenSourceInventory::Available(inventory) =
                when_source_inventory(source)
            {
                assert!(!inventory.is_complete(), "{source:?}");
                assert!(inventory.declarations().is_empty(), "{source:?}");
            }
        }
    }

    #[test]
    // naming.consumer.original-irules-source-context
    // docs/design/analysis/name-resolution-proofs/original-irules-source-context.md
    fn original_when_inventory_preserves_preceding_unknown_source_debt() {
        let source = "unmodelled_source_effect; when HTTP_REQUEST {}";
        let crate::IrulesWhenSourceInventory::Available(inventory) = when_source_inventory(source)
        else {
            panic!("conditional source inventory");
        };
        assert!(!inventory.is_complete());
        assert_ne!(inventory.unavailable_sites(), []);
        let declaration = inventory
            .declarations()
            .first()
            .expect("conditional source declaration");
        let tcl_compiler::registry_invocation::source_structure::OriginalRegistrySource::Vendor(
            metadata,
        ) = declaration.words().source()
        else {
            panic!("original hosted card")
        };
        assert!(metadata.authored_barriers().has_unknown_transitions());
        assert_ne!(metadata.obligations(), []);
    }

    #[test]
    fn ignores_commented_when_and_rejects_trailing_text_after_brace_close() {
        let source =
            "# when HTTP_REQUEST { ignored }\nwhen RULE_INIT { log local0. \"legacy } path\"\n}\n";
        let blocks = when_blocks(source);
        assert_eq!(blocks, [] as [tcl_syntax::event_handler::EventHandler; 0]);
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
