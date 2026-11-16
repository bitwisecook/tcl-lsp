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

#[test]
fn nominal_source_write_names_preserve_data_and_require_available_providers() {
    let mut registry = super::CommandRegistry::build_default();
    for name in ["", "$literal", "${literal}", "(key)"] {
        let arguments = [
            crate::InvocationWord::Literal(name),
            crate::InvocationWord::Literal("VALUE"),
        ];
        let projection = registry.variable_write_projection_for_source_words(
            crate::InvocationWords::structured(crate::InvocationWord::Literal("set"), &arguments),
        );
        assert_eq!(projection.literal_names, [name]);
        assert!(!projection.opaque_variable_frame);
    }
    let arguments = [
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Literal("VALUE"),
    ];
    let dynamic = registry.variable_write_projection_for_source_words(
        crate::InvocationWords::structured(crate::InvocationWord::Literal("set"), &arguments),
    );
    assert!(dynamic.literal_names.is_empty() && dynamic.opaque_variable_frame);
    let mut setter = registry.get("set").unwrap().clone();
    setter.name = "Source::setter";
    setter.required_package = Some("SourceUnavailable");
    registry.insert(setter);
    let arguments = [
        crate::InvocationWord::Literal(""),
        crate::InvocationWord::Literal("VALUE"),
    ];
    let unavailable =
        registry.variable_write_projection_for_source_words(crate::InvocationWords::structured(
            crate::InvocationWord::Literal("Source::setter"),
            &arguments,
        ));
    assert!(unavailable.literal_names.is_empty() && unavailable.opaque_variable_frame);
}

#[test]
fn selected_source_write_keeps_empty_scalar_and_array_names_but_refuses_dynamic_names() {
    // experiments/name-handling/results/source-empty-write-projection-20261014/oracles/observations.json
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let context =
            crate::model::ingress::resolve_environment(environment).default_context_registry();
        for (word, expected) in [
            (crate::InvocationWord::Literal(""), Some("")),
            (crate::InvocationWord::Literal("(key)"), Some("(key)")),
            (crate::InvocationWord::Dynamic, None),
        ] {
            let arguments = [word, crate::InvocationWord::Literal("VALUE")];
            let selected =
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    context.commands(),
                    Some(context.context()),
                    crate::InvocationWords::structured(
                        crate::InvocationWord::Literal("set"),
                        &arguments,
                    )
                    .with_profile(context.commands().profile()),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                );
            let projection = super::CommandRegistry::variable_write_projection_for_selected_source(
                &selected.resolved().unwrap(),
            );
            if let Some(expected) = expected {
                assert_eq!(projection.literal_names, [expected]);
                assert!(!projection.opaque_variable_frame);
            } else {
                assert_eq!(projection.literal_names.len(), 0);
                assert!(projection.opaque_variable_frame);
            }
        }
    }
}
