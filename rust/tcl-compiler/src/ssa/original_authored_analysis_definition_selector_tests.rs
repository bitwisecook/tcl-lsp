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

fn unit(source: &str, registry: &CommandRegistry) -> crate::compilation_unit::CompilationUnit {
    crate::compilation_unit::CompilationUnit::build_for_with_config(
        source,
        registry,
        false,
        tcl_lexer::LexerConfig::default(),
    )
}

#[test]
fn authored_source_definition_owner_requires_current_module_body_and_standalone_purpose() {
    let registry = CommandRegistry::build_default();
    let source = "set value TOP\nproc p {} {set value BODY}\n";
    let original = unit(source, &registry);
    assert!(original.ir_module.source_metadata_input.is_none());
    assert!(
        original
            .ir_module
            .source_entry
            .metadata_context
            .is_standalone()
    );
    assert!(current_authored_analysis_function(
        &original,
        &original.top_level,
        &registry
    ));
    assert_eq!(
        original
            .ir_module
            .original_authored_declaration_body_units
            .len(),
        1
    );
    let id = *original
        .ir_module
        .original_authored_declaration_body_units
        .keys()
        .next()
        .unwrap();
    let function = original.function_for_body(Some(id)).unwrap();
    assert!(current_authored_analysis_function(
        &original, function, &registry
    ));
    let mut missing = original.clone();
    missing.ir_module.source_entry.metadata_context =
        crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(None);
    assert!(!current_authored_analysis_function(
        &missing,
        &missing.top_level,
        &registry
    ));
    let mut stale = original.clone();
    stale.source.push(' ');
    assert!(!current_authored_analysis_function(
        &stale,
        &stale.top_level,
        &registry
    ));
    let mut changed = original.clone();
    changed
        .ir_module
        .body_units
        .get_mut(&id)
        .unwrap()
        .params
        .push("foreign".into());
    assert!(!current_authored_analysis_function(
        &changed,
        changed.function_for_body(Some(id)).unwrap(),
        &registry,
    ));
    let other = unit(source, &registry);
    assert!(!current_authored_analysis_function(
        &original,
        &other.top_level,
        &registry
    ));
}

#[test]
fn authored_body_tokens_keep_the_actual_original_header_attachment() {
    let registry = CommandRegistry::build_default();
    let source = "proc p {} {puts BODY}\n";
    let original = unit(source, &registry);
    let module = &original.ir_module;
    assert!(module.source_metadata_input.is_none());
    let retained = module.retained_source_bindings.as_deref().unwrap();
    let (id, card) = module
        .original_authored_declaration_body_units
        .iter()
        .next()
        .unwrap();
    let body = &module.body_units.get(id).unwrap().body;
    assert!(card.matches_procedure(module, module.body_units.get(id).unwrap()));
    let tokens = body
        .retained_source_tokens_for_statement(body.statements.first().unwrap())
        .unwrap();
    let scope = tokens
        .source_binding
        .as_ref()
        .unwrap()
        .original_authored_source_body_scope_for_name_roles(tokens, &registry)
        .unwrap();
    assert!(scope.matches_module(module));
    assert!(card.owns_original_source_scope(scope));
    assert!(retained.owns_original_analysis_attachment(tokens));
    let mut missing = tokens.clone();
    missing.source_binding = Some(crate::command_binding::SourceInvocationBinding::default());
    assert!(!retained.owns_original_analysis_attachment(&missing));
    let foreign = unit(source, &registry);
    let (_, foreign_card) = foreign
        .ir_module
        .original_authored_declaration_body_units
        .iter()
        .next()
        .unwrap();
    let foreign_id = *foreign
        .ir_module
        .original_authored_declaration_body_units
        .keys()
        .next()
        .unwrap();
    let foreign_body = &foreign.ir_module.body_units.get(&foreign_id).unwrap().body;
    let foreign_tokens = foreign_body
        .retained_source_tokens_for_statement(foreign_body.statements.first().unwrap())
        .unwrap();
    let foreign_scope = foreign_tokens
        .source_binding
        .as_ref()
        .unwrap()
        .original_authored_source_body_scope_for_name_roles(foreign_tokens, &registry)
        .unwrap();
    assert!(foreign_card.owns_original_source_scope(foreign_scope));
    assert!(!card.owns_original_source_scope(foreign_scope));
    assert!(!retained.owns_original_analysis_attachment(foreign_tokens));
}

#[test]
fn authored_source_definitions_select_exact_represented_versions() {
    let registry = CommandRegistry::build_default();
    let source = "set value 123\nset value TEXT\n";
    let original = unit(source, &registry);
    assert!(original.ir_module.source_metadata_input.is_none());
    let definitions = source
        .match_indices("set value")
        .map(|(offset, _)| {
            let span = tcl_lexer::Span::new(
                u32::try_from(offset).expect("fixture source offset fits u32") + 4,
                u32::try_from(offset).expect("fixture source offset fits u32") + 9,
            );
            original_analysis_definition_at_unit(&original, span, &registry)
                .expect("authentic represented Authored source definition")
        })
        .collect::<Vec<_>>();
    assert_eq!(definitions.len(), 2);
    assert!(definitions.iter().all(|definition| std::ptr::eq(
        definition.function(),
        std::ptr::from_ref(&original.top_level)
    )));
    assert_eq!(definitions[0].symbol(), definitions[1].symbol());
    assert_ne!(definitions[0].version(), definitions[1].version());
    let span = definitions[1].span();
    assert!(
        original_analysis_definition_at_unit(
            &original,
            tcl_lexer::Span::new(span.start() + 1, span.end()),
            &registry,
        )
        .is_none()
    );
    let mut malformed = original.clone();
    malformed.top_level.source_config.strict_quoting =
        !malformed.top_level.source_config.strict_quoting;
    assert!(original_analysis_definition_at_unit(&malformed, span, &registry).is_none());
}
