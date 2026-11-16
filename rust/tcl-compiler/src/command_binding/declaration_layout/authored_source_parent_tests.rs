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

#[test]
fn authored_source_parent_reborrow_keeps_allocated_and_uninstalled_body_owners() {
    let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
    let context = environment.default_context_registry();
    let registry = context.commands();
    let profile = environment.unit_profile();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    for (prefix, allocated) in [("", true), ("rename $old decl\n", false)] {
        let source = format!("{prefix}proc kept {{x}} {{list retained}}");
        let unit = crate::compilation_unit::CompilationUnit::build_for_with_config(
            &source, registry, false, config,
        );
        let module = &unit.ir_module;
        let (&id, _) = module
            .original_authored_declaration_body_units
            .iter()
            .find(|(_, card)| card.name() == "kept")
            .unwrap();
        let procedure = &module.body_units[&id];
        let tokens = procedure
            .body
            .retained_source_tokens_for_statement(&procedure.body.statements[0])
            .unwrap();
        let scope = tokens
            .source_binding
            .as_ref()
            .unwrap()
            .original_authored_source_body_scope_for_name_roles(tokens, registry)
            .unwrap();
        let bindings = module
            .retained_source_bindings
            .as_ref()
            .unwrap()
            .positioned_for_module(module, registry)
            .unwrap();
        let site = CommandAllocationSite {
            source: Arc::clone(bindings.source_origin().unwrap()),
            offset: tokens.words()[0].source().span.start(),
        };
        let observations = bindings.declaration_layouts.get(&site).unwrap();
        for row in original_declaration_layouts(observations).unwrap() {
            match row.entry.as_ref() {
                OriginalDiagnosticFrameEntry::Body(body) => {
                    assert!(allocated, "{prefix:?}");
                    assert_eq!(
                        &body.allocation().site,
                        scope.original_parent_site().unwrap()
                    );
                }
                OriginalDiagnosticFrameEntry::DeclaredProcedure(body) => {
                    assert!(!allocated, "{prefix:?}");
                    assert_eq!(
                        body.declaration_site(),
                        scope.original_parent_site().unwrap()
                    );
                }
                _ => panic!("actual procedure source owner"),
            }
        }
        let selected = bindings
            .original_authored_declaration_source_tokens(tokens, scope, registry)
            .expect("genuine original parent/source row");
        let binding = selected.source_binding.as_ref().unwrap();
        assert!(
            binding
                .original_authored_declaration_scalar_layout(&selected, registry)
                .is_some()
        );
        assert!(binding.execution_is_unknown());
        assert!(binding.proved_execution_target().is_none());
        assert!(
            crate::registry_invocation::normal_representation_invocation_with_metadata_context(
                registry, None, &selected,
            )
            .is_none()
        );
        assert!(
            crate::registry_invocation::admitted_native_compiler_invocation(
                registry, None, &selected,
            )
            .is_none()
        );
        let mut withdrawn = bindings.clone();
        withdrawn.declaration_layouts.remove(&site);
        assert!(
            withdrawn
                .original_authored_declaration_source_tokens(tokens, scope, registry,)
                .is_none()
        );
    }
}
