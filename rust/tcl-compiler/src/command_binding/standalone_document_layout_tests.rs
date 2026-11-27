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

fn original_tokens(unit: &crate::compilation_unit::CompilationUnit) -> crate::ir::CommandTokens {
    original_script_tokens(&unit.ir_module.top_level)
}

fn original_script_tokens(script: &crate::ir::Script) -> crate::ir::CommandTokens {
    script
        .statements
        .iter()
        .filter_map(|statement| script.retained_source_tokens_for_statement(statement))
        .find(|tokens| tokens.argv_texts.first().is_some_and(|head| head == "set"))
        .expect("actual complete original set vector")
        .clone()
}

fn original_head(tokens: &crate::ir::CommandTokens) -> tcl_lexer::NativeWord {
    let binding = tokens.source_binding.as_ref().unwrap();
    let site = binding.invocation_site().unwrap();
    let config = binding.original_lexer_config_for_tokens(tokens).unwrap();
    site.source
        .original_lexical_command(tokens.words(), site.offset, config)
        .unwrap()
        .native_words()
        .first()
        .unwrap()
        .clone()
}

#[test]
fn standalone_document_grammar_does_not_become_a_native_string_recipe() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for("set x value", registry, false);
    let tokens = original_tokens(&unit);
    let binding = tokens.source_binding.as_ref().unwrap();
    let snapshot = binding.lookup_state.as_ref().unwrap();
    let config = binding.original_lexer_config_for_tokens(&tokens).unwrap();
    let policy = snapshot
        .state
        .source_variables
        .execution_name_policy
        .unwrap()
        .native_recipe()
        .unwrap();
    let head = original_head(&tokens);
    assert_eq!(config.escapes, tcl_dialect::EscapeSyntax::Tcl90);
    assert_eq!(
        policy.string_protocol().escape_syntax(),
        tcl_dialect::EscapeSyntax::Tcl86
    );
    assert_eq!(
        policy.authority(),
        tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
    );
    assert!(
        tcl_names::word::SignatureSourceNameKey::from_original_native_word(
            &head,
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
            policy,
        )
        .is_none()
    );
    let projected = original_static_layout_projection(
        snapshot,
        &binding.lookup_namespace_key,
        &head,
        config,
        snapshot.state.source_variables.invocation_dialect.unwrap(),
        false,
    )
    .expect("original Standalone source metadata keeps its independent Document grammar");
    assert!(projected.targets.iter().any(|target| target.registry_backed
        && target.kind == BindingKind::Builtin
        && target.command == "set"));
    assert!(projected.proved_execution_target().is_none());
    assert!(unit.ir_module.source_entry.native_entry.is_none());
    assert!(unit.ir_module.source_metadata_input.is_none());
    let advice = binding
        .original_authored_source_layout(&tokens, registry)
        .unwrap();
    assert_eq!(
        advice.dialect().lexer_grammar,
        config.grammar_over(advice.dialect().lexer_grammar)
    );
    assert_eq!(
        advice.dialect().word_values,
        tcl_syntax::word_rules::WordValueRules::from_config(&config)
    );
}

#[test]
fn original_source_layout_keeps_missing_foreign_and_changed_vector_refusals() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_standalone_for(
        "set x value",
        registry,
        false,
    );
    let tokens = original_tokens(&unit);
    assert!(
        crate::registry_invocation::original_authored_source_structure(&tokens, registry).is_some()
    );
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    assert!(
        crate::registry_invocation::original_authored_source_structure(&tokens, foreign).is_none()
    );
    let mut changed = tokens.clone();
    changed.argv_texts[0] = "different".to_owned();
    assert!(
        crate::registry_invocation::original_authored_source_structure(&changed, registry)
            .is_none()
    );
    let mut missing = tokens;
    missing.source_binding = None;
    assert!(
        crate::registry_invocation::original_authored_source_structure(&missing, registry)
            .is_none()
    );
}

#[test]
fn supplied_source_keeps_native_word_configuration_checks() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
        "set x value",
        registry,
        false,
        registry.profile().unwrap(),
    );
    let tokens = original_tokens(&unit);
    let binding = tokens.source_binding.as_ref().unwrap();
    let snapshot = binding.lookup_state.as_ref().unwrap();
    let config = binding.original_lexer_config_for_tokens(&tokens).unwrap();
    let head = original_head(&tokens);
    let policy = snapshot
        .state
        .source_variables
        .execution_name_policy
        .unwrap()
        .native_recipe()
        .unwrap();
    assert!(!snapshot.state.baseline.metadata_context.is_standalone());
    assert_eq!(config.escapes, policy.string_protocol().escape_syntax());
    assert!(
        tcl_names::word::SignatureSourceNameKey::from_original_native_word(
            &head,
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
            policy,
        )
        .is_some()
    );
    let mismatch = tcl_lexer::LexerConfig {
        escapes: tcl_dialect::EscapeSyntax::Tcl90,
        ..config
    };
    assert!(
        original_static_layout_projection(
            snapshot,
            &binding.lookup_namespace_key,
            &head,
            mismatch,
            snapshot.state.source_variables.invocation_dialect.unwrap(),
            false,
        )
        .is_none()
    );
    assert!(
        crate::registry_invocation::original_authored_source_structure(&tokens, registry).is_none()
    );
}
#[test]
fn explicit_standalone_options_keep_original_template_argument_grammar() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let config = tcl_lexer::LexerConfig::default();
    let mut lowerer = crate::lowering::Lowerer::with_config(registry, config);
    lowerer.set_source_analysis_options(SourceAnalysisOptions::default());
    let module = crate::lowering::lower_to_ir_with(lowerer, r#"set x "\U0001F600""#);
    let tokens = original_script_tokens(&module.top_level);
    let binding = tokens.source_binding.as_ref().unwrap();
    let snapshot = binding.lookup_state.as_ref().unwrap();
    let original_dialect = snapshot.state.source_variables.invocation_dialect.unwrap();
    assert_eq!(
        original_dialect.lexer_grammar.escapes,
        tcl_dialect::EscapeSyntax::Tcl86
    );
    let advice = binding
        .original_authored_source_layout(&tokens, registry)
        .unwrap();
    assert_eq!(
        advice.dialect().lexer_grammar.escapes,
        tcl_dialect::EscapeSyntax::Tcl90
    );
    assert_eq!(advice.dialect().tcl_version, original_dialect.tcl_version);
    assert_eq!(advice.dialect().core_point, original_dialect.core_point);
    assert_eq!(
        advice.dialect().word_values,
        tcl_syntax::word_rules::WordValueRules::from_config(&config)
    );
    assert_eq!(
        snapshot.state.source_variables.invocation_dialect,
        Some(original_dialect)
    );
    let source =
        crate::registry_invocation::original_authored_source_structure(&tokens, registry).unwrap();
    assert_eq!(source.argument_literal(1).as_deref(), Some("😀"));
    assert!(module.source_entry.native_entry.is_none());
    assert!(module.source_metadata_input.is_none());
}
