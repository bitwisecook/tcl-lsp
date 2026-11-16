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

fn assert_native_expression_child_refusals(
    retained: &CommandTokens,
    tokens: &CommandTokens,
    input: &crate::analyser::ResolvedAnalysisInput,
    config: tcl_lexer::LexerConfig,
    registry: &tcl_registry::CommandRegistry,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
) {
    let mut detached = retained.clone();
    detached.source_binding = None;
    assert!(
        original_call_context(
            &detached,
            config,
            registry,
            OriginalLiftedCallContext::Metadata(metadata)
        )
        .is_none()
    );
    let mut changed = retained.clone();
    changed.argv_texts[0] = "other".into();
    assert!(
        changed
            .source_binding
            .as_ref()
            .unwrap()
            .original_native_source_syntax_offset(&changed, input, registry)
            .is_none()
    );
    let mut missing = tokens.clone();
    missing.nested_bindings.clear();
    assert!(
        checked_original_lifted_calls_with_metadata_context(&missing, config, registry, metadata,)
            .is_none()
    );
}

#[test]
fn native_source_braced_expression_inventory_preserves_child_geometry_and_refuses_expansion() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let profile = registry.profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(registry, &input)
            .unwrap();
    for (source, expanded) in [
        ("proc p {} {expr {[helper] + 1}}\n", false),
        ("proc p {} {expr {*}{[helper] + 1}}\n", true),
    ] {
        let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry,
                config,
                dialect: Some(profile),
                defer_top_level: false,
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        );
        assert_eq!(
            unit.ir_module.original_native_declaration_body_units.len(),
            1
        );
        let (&body, _) = unit
            .ir_module
            .original_native_declaration_body_units
            .iter()
            .next()
            .unwrap();
        let script = &unit.ir_module.body_units.get(&body).unwrap().body;
        let offset =
            u32::try_from(source.find("expr").unwrap()).expect("fixture source offset fits u32");
        let originals = script
            .statements
            .iter()
            .filter_map(|statement| script.retained_source_tokens_for_statement(statement))
            .filter(|tokens| {
                tokens.synthetic.is_none()
                    && tokens
                        .words()
                        .first()
                        .is_some_and(|word| word.source().span.start() == offset)
            })
            .collect::<Vec<_>>();
        let tokens = *originals
            .first()
            .expect("actual original expression command");
        assert!(originals.iter().all(|original| *original == tokens));
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .has_original_native_source_command()
        );
        let calls =
            checked_original_lifted_calls_with_metadata_context(tokens, config, registry, metadata);
        if expanded {
            assert!(calls.is_none());
        } else {
            let calls = calls.expect("actual braced source-expression child inventory");
            assert_eq!(calls.len(), 1);
            let child = &calls[0];
            assert_eq!(&source[child.span.as_range()], "[helper]");
            let retained = child.tokens.as_ref().expect("original child word vector");
            assert_eq!(retained.argv_texts, ["helper"]);
            let proof = retained.source_binding.as_ref().unwrap();
            assert_eq!(
                proof.original_native_source_syntax_offset(retained, &input, registry),
                Some(retained.words().first().unwrap().source().span.start())
            );
            assert!(
                proof
                    .original_native_source_words(retained, &input, registry)
                    .is_none()
            );
            assert_native_expression_child_refusals(
                retained, tokens, &input, config, registry, metadata,
            );
        }
    }
}

#[test]
fn native_source_child_inventory_uses_its_own_full_word_receipt() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let profile = registry.profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        "proc p {} {set value TEXT; return $value}\n",
        crate::compilation_unit::UnitBuildOptions {
            registry,
            config,
            dialect: Some(profile),
            defer_top_level: false,
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    );
    assert_eq!(
        unit.ir_module.original_native_declaration_body_units.len(),
        1
    );
    let (&id, _) = unit
        .ir_module
        .original_native_declaration_body_units
        .iter()
        .next()
        .unwrap();
    let body = &unit.ir_module.body_units.get(&id).unwrap().body;
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(registry, &input)
            .unwrap();
    let mut count = 0;
    for site in body.command_binding_sites.iter() {
        let tokens = site.source_tokens.as_deref().unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.has_original_native_source_command());
        assert!(binding.original_lexer_config_for_tokens(tokens).is_none());
        assert!(binding.invocation_site().is_none());
        assert!(
            binding
                .original_native_source_words(tokens, &input, registry)
                .is_some()
        );
        let calls =
            checked_original_lifted_calls_with_metadata_context(tokens, config, registry, metadata)
                .expect("genuine readonly Native child syntax inventory");
        assert_eq!(calls, [] as [crate::word_subst::LiftedCall; 0]);
        count += 1;
        let mut changed = tokens.clone();
        changed.argv_texts[0].push('_');
        assert!(
            checked_original_lifted_calls_with_metadata_context(
                &changed, config, registry, metadata
            )
            .is_none()
        );
        assert!(
            checked_original_lifted_calls_with_metadata_context(
                tokens,
                config,
                &tcl_registry::CommandRegistry::build_default(),
                metadata
            )
            .is_none()
        );
        let mut changed_config = config;
        changed_config.strict_quoting = !changed_config.strict_quoting;
        let changed_input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            changed_config,
        );
        let changed_metadata =
            crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
                registry,
                &changed_input,
            )
            .unwrap();
        assert!(
            checked_original_lifted_calls_with_metadata_context(
                tokens,
                changed_config,
                registry,
                changed_metadata
            )
            .is_none()
        );
    }
    assert_eq!(count, 2);
}
