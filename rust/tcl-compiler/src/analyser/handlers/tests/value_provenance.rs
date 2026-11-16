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

//! Values a handler reads through checked logical provenance: dynamic words, constant folds, interpreter handles and dictionary keys.

use super::*;

fn original_value_use_token(analyser: &Analyser, needle: &str) -> Token {
    let start = u32::try_from(analyser.source.rfind(needle).unwrap()).unwrap();
    let commands = crate::segmenter::segment_commands_with_offset_and_config(
        analyser
            .source
            .get(usize::try_from(start).unwrap()..)
            .unwrap(),
        start,
        analyser.file_lexer_config().nested(),
    );
    commands.first().unwrap().argv[0]
}

#[test]
fn logical_dynamic_word_keeps_exact_braced_name_and_interpolation_sources() {
    let source =
        "set n WRONG; set {$n} RIGHT; set {scalar(open} SCALAR; puts ${n}:${$n}:${scalar(open}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = original_value_use_token(&analyser, "${n}:${$n}:${scalar(open}");
    assert_eq!(
        analyser.resolve_dynamic_word("${n}:${$n}:${scalar(open}", Some(token), false, &[]),
        Some("WRONG:RIGHT:SCALAR".to_owned())
    );
    analyser.source = source.replace("RIGHT", "OTHER").into();
    assert!(
        analyser
            .resolve_dynamic_word("${n}:${$n}:${scalar(open}", Some(token), false, &[])
            .is_none()
    );
}

#[test]
fn logical_nested_constant_fold_uses_original_variable_components() {
    let source = "set n WRONG; set {$n} RIGHT; set result [list $n [list ${$n}]]; puts $result";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = original_value_use_token(&analyser, "$result");
    assert_eq!(
        analyser
            .lookup_logical_const_substitution(token)
            .map(|(value, _)| value),
        Some("WRONG RIGHT")
    );
}

#[test]
fn logical_regex_value_metadata_uses_original_scope_and_source_provenance() {
    let source = "set pattern {ROOT}; proc p {} {set pattern {LOCAL}; regexp $pattern text}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let source_use = source.rfind("$pattern").unwrap();
    assert!(analyser.result.regex_patterns.iter().any(|pattern| {
        pattern.pattern == "LOCAL" && usize::try_from(pattern.range.start()).unwrap() == source_use
    }));
    assert!(
        !analyser
            .result
            .regex_patterns
            .iter()
            .any(|pattern| pattern.pattern == "ROOT")
    );
}

#[test]
fn logical_interpreter_handle_value_keeps_distinct_path_provenance_and_snapshot() {
    let source = "set child [interp create named]; interp eval $child {set x VALUE}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = original_value_use_token(&analyser, "$child");
    assert_eq!(
        analyser.resolve_dynamic_interp_path(Some(token)).as_deref(),
        Some("named")
    );
    let snapshot = analyser.snapshot();
    analyser.source = source.replace("VALUE", "OTHER").into();
    assert!(analyser.resolve_dynamic_interp_path(Some(token)).is_none());
    analyser.source = source.to_owned().into();
    analyser.restore(snapshot);
    assert_eq!(
        analyser.resolve_dynamic_interp_path(Some(token)).as_deref(),
        Some("named")
    );
    let source = "set child named; interp eval $child {set x VALUE}";
    let mut scalar = Analyser::new();
    scalar.analyse_and_retain_result_for_test(source, "tcl");
    let token = original_value_use_token(&scalar, "$child");
    assert!(scalar.resolve_dynamic_interp_path(Some(token)).is_none());
}

#[test]
fn logical_dictionary_keys_have_metadata_and_data_geometry_without_variable_declarations() {
    let source = "set d {alpha A beta B}; dict with d {puts $alpha}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let scope = &analyser.result.global_scope;
    assert!(!scope.variables.contains_key("alpha"));
    assert!(!scope.variables.contains_key("beta"));
    let alpha = scope.readonly_dictionary_keys.get("alpha").unwrap();
    assert_eq!(
        &source[alpha.original_key_span().unwrap().as_range()],
        "alpha"
    );
    let image = tcl_lexer::SourceImage::document(source);
    let advice = analyser
        .result
        .logical_variable_scope_advice(
            &image,
            analyser.file_lexer_config(),
            u32::try_from(source.rfind("$alpha").unwrap()).unwrap(),
        )
        .unwrap();
    assert!(advice.readonly_variable_names().contains(&"alpha"));
    assert!(advice.variable_definition("alpha").is_none());
}

#[test]
fn logical_dictionary_metadata_retains_an_empty_variable_name() {
    let source = "set d {{} EMPTY}; dict with d {puts ${}}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let scope = &analyser.result.global_scope;
    assert!(scope.readonly_dictionary_keys.contains_key(""));
    assert!(!scope.variables.contains_key(""));
}

#[test]
fn logical_dictionary_write_invalidates_the_previous_constant_value() {
    let source = "set x OLD; set d {x NEW}; dict with d {puts $x}; puts $x";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = original_value_use_token(&analyser, "$x");
    assert!(analyser.lookup_logical_const_substitution(token).is_none());
    assert!(analyser.result.global_scope.variables.contains_key("x"));
}

#[test]
fn logical_dictionary_write_invalidates_interpreter_handle_provenance() {
    let source = "set child [interp create named]; set d {child other}; dict with d {}; interp eval $child {set x VALUE}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = original_value_use_token(&analyser, "$child");
    assert!(analyser.resolve_dynamic_interp_path(Some(token)).is_none());
}

#[test]
fn logical_dictionary_metadata_masks_an_outer_procedure_name() {
    let source = "set x ROOT; proc p {} {set d {x LOCAL}; dict with d {puts $x}}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = original_value_use_token(&analyser, "$x");
    let image = tcl_lexer::SourceImage::document(source);
    let advice = analyser
        .result
        .logical_variable_scope_advice(&image, analyser.file_lexer_config(), token.span.start())
        .unwrap();
    assert!(advice.readonly_variable_names().contains(&"x"));
    assert!(advice.variable_definition("x").is_none());
    assert!(analyser.lookup_logical_const_substitution(token).is_none());
}

#[test]
fn dynamic_apply_value_uses_checked_logical_variable_declaration() {
    let source = "namespace eval N {set lambda {{p} {puts $p} ::N}}; apply $::N::lambda VALUE";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = tcl_lexer::Lexer::with_config(source, analyser.file_lexer_config())
        .tokenise_all()
        .unwrap()
        .into_iter()
        .find(|token| token.kind == TokenType::Var)
        .unwrap();
    let elements = analyser
        .resolve_dynamic_apply_lambda(token)
        .expect("original Logical namespace value");
    assert_eq!(
        elements
            .iter()
            .map(|(_, value)| value.as_str())
            .collect::<Vec<_>>(),
        vec!["p", "puts $p", "::N"]
    );
    analyser.source = source.replace("VALUE", "OTHER").into();
    assert!(analyser.resolve_dynamic_apply_lambda(token).is_none());
}

#[test]
fn logical_defined_symbol_word_uses_current_value_provenance() {
    let source = "set name SYMBOL; puts $name";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = tcl_lexer::Lexer::with_config(source, analyser.file_lexer_config())
        .tokenise_all()
        .unwrap()
        .into_iter()
        .find(|token| token.kind == TokenType::Var)
        .unwrap();
    assert_eq!(
        analyser.defined_symbol_reporting_name(None, None, "$name", token, true, &[]),
        Some("SYMBOL".to_owned())
    );
    let original = analyser.snapshot();
    analyser.result.resolved_input = None;
    assert!(
        analyser
            .defined_symbol_reporting_name(None, None, "$name", token, true, &[])
            .is_none()
    );
    analyser.restore(original);
    assert_eq!(
        analyser.resolve_const_word("$name", token, true),
        Some("SYMBOL".to_owned())
    );
}

#[test]
fn dict_with_value_uses_checked_logical_scope() {
    let source =
        "set d {root ROOT}; namespace eval N {set d {local LOCAL}; dict with d {puts $local}}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let namespace = analyser
        .result
        .global_scope
        .children
        .iter()
        .find(|scope| scope.kind == super::super::super::types::ScopeKind::Namespace)
        .unwrap();
    assert!(namespace.readonly_dictionary_keys.contains_key("local"));
    assert!(!namespace.readonly_dictionary_keys.contains_key("root"));
    assert!(!namespace.variables.contains_key("local"));
    let key = &namespace.readonly_dictionary_keys["local"];
    assert_eq!(
        &source[key.original_key_span().unwrap().as_range()],
        "local"
    );
    let source = "set d {root ROOT}; proc p {} {dict with d {puts $root}}";
    let mut procedure = Analyser::new();
    procedure.analyse_and_retain_result_for_test(source, "tcl");
    let local = procedure
        .result
        .global_scope
        .children
        .iter()
        .find(|scope| scope.kind == super::super::super::types::ScopeKind::Proc)
        .unwrap();
    assert!(!local.variables.contains_key("root"));
}

#[test]
fn dynamic_apply_scanner_uses_irules_brace_boundary_config() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // An original Logical builder with the supplied lexer overlay, without
    // any Native/F5 list execution or lambda-frame claim.
    let source = "set lambda [list {p} {set y}{set z}]";
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_dialect("f5-irules");
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let input = super::super::super::ResolvedAnalysisInput::new(profile, profile, context, config);
    let mut analyser = Analyser::new().with_resolved_input(input);
    analyser.result = analyser.analyse(source, "tcl");
    let token = tcl_lexer::Lexer::with_source_map(tcl_lexer::SourceMap::new(source), config)
        .tokenise_all()
        .expect("valid command substitution")
        .into_iter()
        .find(|token| token.kind == tcl_lexer::TokenType::Cmd)
        .expect("command token");
    let elements = analyser
        .resolve_dynamic_apply_lambda(token)
        .expect("original literal list lambda");
    assert_eq!(elements.len(), 3, "iRules list elements: {elements:?}");
    let mut missing = Analyser::new();
    missing.source = source.to_owned().into();
    missing.profile = tcl_dialect::DialectProfile::irules();
    assert!(
        missing.resolve_dynamic_apply_lambda(token).is_none(),
        "a profile and authored source do not supply the original builder"
    );
}
