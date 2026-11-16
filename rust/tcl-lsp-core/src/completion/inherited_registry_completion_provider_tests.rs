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
use tcl_registry::spec::ObjectClassSpec;
use tcl_registry::{CommandSpec, SubCommand};

#[test]
fn inherited_completion_uses_selected_provider_package_instead_of_receiver_package() {
    static METHODS: [SubCommand; 1] = [SubCommand {
        name: "inherited",
        detail: "actual parent provider",
        lifecycle: tcl_registry::lifecycle::Lifecycle::introduced_in("2.0"),
        ..SubCommand::DEFAULT
    }];
    static FOREIGN: [SubCommand; 1] = [SubCommand {
        name: "foreign",
        ..SubCommand::DEFAULT
    }];
    static CHILD: ObjectClassSpec = ObjectClassSpec {
        class_name: "CompletionChild",
        instance_methods: &[],
        superclasses: &["CompletionParent"],
        allow_unknown_methods: false,
        method_prefix_matching: tcl_registry::abbrev::PrefixMatching::Strict,
    };
    static PARENT: ObjectClassSpec = ObjectClassSpec {
        class_name: "CompletionParent",
        instance_methods: &METHODS,
        superclasses: &[],
        allow_unknown_methods: false,
        method_prefix_matching: tcl_registry::abbrev::PrefixMatching::Strict,
    };
    static FOREIGN_PARENT: ObjectClassSpec = ObjectClassSpec {
        class_name: "CompletionParent",
        instance_methods: &FOREIGN,
        superclasses: &[],
        allow_unknown_methods: false,
        method_prefix_matching: tcl_registry::abbrev::PrefixMatching::Strict,
    };
    let mut registry = CommandRegistry::build_default();
    for (class, package) in [(&CHILD, "ReceiverPackage"), (&PARENT, "ProviderPackage")] {
        registry.insert(CommandSpec {
            name: class.class_name,
            required_package: Some(package),
            object_class: Some(class),
            ..CommandSpec::DEFAULT
        });
    }
    registry.insert(CommandSpec {
        name: "CompletionParent",
        required_package: Some("ForeignPackage"),
        object_class: Some(&FOREIGN_PARENT),
        surface: Some(SpecSurface::JIM),
        ..CommandSpec::DEFAULT
    });
    let profile = crate::profile_for_dialect("tcl");
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    for (receiver, provider, expected) in [("1.0", "2.0", true), ("9.0", "1.0", false)] {
        let source = format!(
            "package require ReceiverPackage {receiver}\npackage require ProviderPackage {provider}\n"
        );
        let analysis =
            crate::source_structure::analyse_document(&source, profile, &registry, config);
        assert!(analysis.allows_lexical_declaration_advice());
        let items = registry_method_items(&analysis, &registry, "CompletionChild", profile)
            .unwrap_or_default();
        assert_eq!(items.iter().any(|item| item.label == "inherited"), expected);
        assert!(items.iter().all(|item| item.label != "foreign"));
        if expected {
            assert_eq!(
                items[0].documentation.as_deref(),
                Some("actual parent provider")
            );
        }
    }
}
