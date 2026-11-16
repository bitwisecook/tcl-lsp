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

use super::*;
use tcl_registry::{CommandSpec, Traits};

const SELECTED_DOC: tcl_registry::hover::HoverSnippet =
    tcl_registry::hover::HoverSnippet::brief("selected source provider", &[], "source metadata");
const FOREIGN_DOC: tcl_registry::hover::HoverSnippet = tcl_registry::hover::HoverSnippet::brief(
    "unavailable foreign provider",
    &[],
    "source metadata",
);

fn selected_builtin_filter_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::build_default();
    for (name, selected_traits, foreign_traits, foreign_package) in [
        (
            "catalogue-operator",
            Traits::empty(),
            Traits::OPERATOR_COMMAND,
            None,
        ),
        (
            "catalogue-member",
            Traits::empty(),
            Traits::DEFINITION_BODY_MEMBER_ONLY,
            None,
        ),
        (
            "catalogue-frame",
            Traits::empty(),
            Traits::TCLOO_METHOD_CONTEXT | Traits::TCLOO_REQUIRES_METHOD_FRAME,
            None,
        ),
        (
            "catalogue-package",
            Traits::empty(),
            Traits::empty(),
            Some("Tk"),
        ),
        (
            "catalogue-version",
            Traits::empty(),
            Traits::empty(),
            Some("http"),
        ),
        (
            "catalogue-hidden-member",
            Traits::DEFINITION_BODY_MEMBER_ONLY,
            Traits::empty(),
            None,
        ),
        (
            "catalogue-hidden-frame",
            Traits::TCLOO_METHOD_CONTEXT | Traits::TCLOO_REQUIRES_METHOD_FRAME,
            Traits::empty(),
            None,
        ),
    ] {
        registry.insert(CommandSpec {
            name,
            traits: selected_traits,
            hover: Some(SELECTED_DOC),
            ..CommandSpec::DEFAULT
        });
        registry.insert(CommandSpec {
            name,
            traits: foreign_traits,
            surface: Some(SpecSurface::TCL90),
            required_package: foreign_package,
            lifecycle: tcl_registry::lifecycle::Lifecycle {
                introduced: Some("99.0"),
                ..tcl_registry::lifecycle::Lifecycle::UNSPECIFIED
            },
            hover: Some(FOREIGN_DOC),
            ..CommandSpec::DEFAULT
        });
    }
    registry
}

#[test]
fn builtin_filters_keep_the_actual_selected_provider_through_every_gate() {
    // naming.consumer.selected-catalogue-completion-provider
    // docs/design/analysis/name-resolution-proofs/selected-catalogue-completion-provider.md
    let profile = crate::profile_for_dialect("tcl8.6");
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let source = "package require http 2.0\n";
    let registry = selected_builtin_filter_registry();
    let analysis = crate::source_structure::analyse_document(source, profile, &registry, config);
    let context = crate::document_floor::DocumentFloor::new(&analysis, profile)
        .context()
        .unwrap();
    for name in registry
        .command_names()
        .filter(|name| name.starts_with("catalogue-"))
    {
        let selected = context.resolve_spec(&registry, name).unwrap();
        assert!(!std::ptr::eq(selected, registry.get(name).unwrap()));
        assert_eq!(selected.hover.unwrap().summary, SELECTED_DOC.summary);
    }
    let frame = crate::oo_dispatch::OoFrame {
        resolves: false,
        method: false,
    };
    let items = builtin_completions(
        &registry,
        profile,
        "catalogue-",
        &FxHashMap::default(),
        false,
        &analysis,
        frame,
    );
    let labels = items
        .iter()
        .map(|item| item.label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        [
            "catalogue-frame",
            "catalogue-member",
            "catalogue-operator",
            "catalogue-package",
            "catalogue-version"
        ]
    );
    assert!(
        items
            .iter()
            .all(|item| item.documentation.as_deref() == Some(SELECTED_DOC.summary))
    );
    assert!(
        items
            .iter()
            .all(|item| item.detail.as_deref() == Some("built-in"))
    );
    let selected_frame = context
        .resolve_spec(&registry, "catalogue-hidden-frame")
        .unwrap();
    assert!(!frame.admits_selected_spec(selected_frame));
    assert!(
        !crate::oo_dispatch::OoFrame {
            resolves: true,
            method: false
        }
        .admits_selected_spec(selected_frame)
    );
    assert!(
        crate::oo_dispatch::OoFrame {
            resolves: true,
            method: true
        }
        .admits_selected_spec(selected_frame)
    );
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert_eq!(
        builtin_completions(
            &registry,
            profile,
            "catalogue-",
            &FxHashMap::default(),
            false,
            &missing,
            frame
        ),
        [] as [crate::completion::CompletionItem; 0]
    );
    let mut changed = analysis.clone();
    changed.body_lexer_config = None;
    assert_eq!(
        builtin_completions(
            &registry,
            profile,
            "catalogue-",
            &FxHashMap::default(),
            false,
            &changed,
            frame
        ),
        [] as [crate::completion::CompletionItem; 0]
    );
}

#[test]
fn grammar_member_details_use_the_same_current_availability_context() {
    // naming.consumer.selected-catalogue-completion-provider
    // docs/design/analysis/name-resolution-proofs/selected-catalogue-completion-provider.md
    use tcl_registry::definer::{DefinitionBodyGrammar, MemberSpec, TCLOO_GRAMMAR};
    static MEMBERS: &[MemberSpec] = &[
        MemberSpec {
            keyword: "catalogue-member",
            surface: None,
            ..TCLOO_GRAMMAR.members[0]
        },
        MemberSpec {
            keyword: "grammar-only",
            surface: None,
            ..TCLOO_GRAMMAR.members[0]
        },
        MemberSpec {
            keyword: "hidden-member",
            surface: Some(SpecSurface::TCL90),
            ..TCLOO_GRAMMAR.members[0]
        },
    ];
    static GRAMMAR: DefinitionBodyGrammar = DefinitionBodyGrammar {
        members: MEMBERS,
        ..TCLOO_GRAMMAR
    };
    let profile = crate::profile_for_dialect("tcl8.6");
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "catalogue-member",
        hover: Some(SELECTED_DOC),
        ..CommandSpec::DEFAULT
    });
    registry.insert(CommandSpec {
        name: "catalogue-member",
        surface: Some(SpecSurface::TCL90),
        hover: Some(FOREIGN_DOC),
        ..CommandSpec::DEFAULT
    });
    let analysis = crate::source_structure::analyse_document("", profile, &registry, config);
    let context = crate::document_floor::DocumentFloor::new(&analysis, profile)
        .context()
        .unwrap();
    let items = grammar_member_completions(&GRAMMAR, "", &registry, config, context);
    assert_eq!(
        items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        ["catalogue-member", "grammar-only"]
    );
    assert_eq!(
        items[0].documentation.as_deref(),
        Some(SELECTED_DOC.summary)
    );
    assert_eq!(items[0].detail.as_deref(), Some("built-in"));
    assert!(items[1].documentation.is_none() && items[1].detail.is_none());
}
