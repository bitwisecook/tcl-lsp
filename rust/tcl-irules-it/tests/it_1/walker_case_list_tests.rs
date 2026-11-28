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
mod case_list_tests {
    use tcl_irules::extract_irules_object_references;
    use tcl_irules::{
        extract_irules_object_references_in_closure,
        extract_original_irules_source_object_references,
    };
    use tcl_registry::CommandRegistry;

    fn reg() -> CommandRegistry {
        CommandRegistry::build_default().project_for_profile(tcl_dialect::DialectProfile::irules())
    }

    /// An object referenced only from a `switch` arm must still be found.
    ///
    /// A `switch` case list is not an `ArgRole::Body`, so the walker never
    /// descended into it: a pool used only inside a `switch` arm — an entirely
    /// ordinary iRule — looked **unreferenced**.  For highlighting that meant the
    /// name read as a plain string; for the reference graph it meant
    /// `bigip-cleanup` would have seen a live pool as an orphan.
    #[test]
    fn objects_referenced_from_a_switch_arm_are_found() {
        crate::support::install_surfaces();
        let registry = reg();
        let src = "when HTTP_REQUEST {\n\
                   \x20   pool /Common/top\n\
                   \x20   switch -glob [HTTP::uri] {\n\
                   \x20       \"/api/*\" { pool /Common/in_switch }\n\
                   \x20       default  { pool /Common/fallback }\n\
                   \x20   }\n\
                   }\n";
        let names: Vec<String> = extract_irules_object_references(src, None, &registry)
            .into_iter()
            .map(|r| r.name)
            .collect();
        for want in ["/Common/top", "/Common/in_switch", "/Common/fallback"] {
            assert!(
                names.iter().any(|n| n == want),
                "`{want}` must be a resolved reference; got {names:?}"
            );
        }
    }
    #[test]
    fn original_object_reference_consumers_ignore_dto_labels_and_runtime_value_guesses() {
        crate::support::install_surfaces();
        let source = "when HTTP_REQUEST {pool /Common/literal; set p /Common/guess; pool $p}";
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let context = tcl_irules::OriginalIrulesSourceContext::capture(source, registry).unwrap();
        let original = extract_original_irules_source_object_references(source, &context, None);
        assert_eq!(
            original
                .iter()
                .map(|reference| reference.name.as_str())
                .collect::<Vec<_>>(),
            ["/Common/literal"]
        );
        let mut displayed = context.presentation_commands(source);
        for command in &mut displayed {
            command.command = "counterfactual".to_owned();
            command.args = vec!["fake".to_owned()];
        }
        assert_eq!(
            extract_irules_object_references_in_closure(source, None, registry, &displayed),
            original
        );
        assert_eq!(
            extract_original_irules_source_object_references(
                &format!("#changed\n{source}"),
                &context,
                None
            ),
            [] as [tcl_irules::IrulesObjectReference; 0]
        );
    }
}
