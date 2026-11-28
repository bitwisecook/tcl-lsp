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

#[cfg(test)]
mod tests {
    use tcl_irules::*;

    #[test]
    fn original_when_inventory_keeps_event_words_bodies_and_conditional_cards() {
        crate::support::install_surfaces();
        let source =
            "when HTTP_REQUEST { pool /Common/a }\nwhen CLIENT_ACCEPTED { log local0. ok }";
        let tcl_irules::IrulesWhenSourceInventory::Available(inventory) =
            when_source_inventory(source)
        else {
            panic!("actual complete source inventory");
        };
        assert!(inventory.is_complete());
        assert_eq!(
            inventory
                .declarations()
                .iter()
                .map(tcl_irules::OriginalIrulesWhenDeclaration::event)
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
            tcl_irules::IrulesWhenSourceInventory::Unavailable
        ));
    }

    #[test]
    fn original_when_inventory_distinguishes_empty_and_unavailable_source_sites() {
        crate::support::install_surfaces();
        for source in ["", "# no source commands\n", "set value 1"] {
            let tcl_irules::IrulesWhenSourceInventory::Available(inventory) =
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
            if let tcl_irules::IrulesWhenSourceInventory::Available(inventory) =
                when_source_inventory(source)
            {
                assert!(!inventory.is_complete(), "{source:?}");
                assert!(inventory.declarations().is_empty(), "{source:?}");
            }
        }
    }

    #[test]
    fn original_when_inventory_preserves_preceding_unknown_source_debt() {
        crate::support::install_surfaces();
        let source = "unmodelled_source_effect; when HTTP_REQUEST {}";
        let tcl_irules::IrulesWhenSourceInventory::Available(inventory) =
            when_source_inventory(source)
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
        crate::support::install_surfaces();
        let source =
            "# when HTTP_REQUEST { ignored }\nwhen RULE_INIT { log local0. \"legacy } path\"\n}\n";
        let blocks = when_blocks(source);
        assert_eq!(blocks, [] as [tcl_syntax::event_handler::EventHandler; 0]);
    }

    #[test]
    fn recognises_canonical_global_when_spellings() {
        crate::support::install_surfaces();
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
        crate::support::install_surfaces();
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
        crate::support::install_surfaces();
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
        crate::support::install_surfaces();
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
        crate::support::install_surfaces();
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
        crate::support::install_surfaces();
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
        crate::support::install_surfaces();
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
