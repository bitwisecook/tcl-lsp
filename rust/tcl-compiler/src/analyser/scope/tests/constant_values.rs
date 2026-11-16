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

//! Constant strings: the Logical lookup through frames and namespaces, and the per-scope tables.

use super::*;

fn logical_value_token(analyser: &Analyser, written: &str) -> Token {
    let start = analyser.source.rfind(written).expect("retained source use");
    let mut token = tcl_lexer::Lexer::with_config(
        &analyser.source[start..],
        analyser.file_lexer_config().nested(),
    )
    .tokenise_all()
    .expect("valid retained source")
    .into_iter()
    .next()
    .unwrap();
    token.span = Span::new(
        token.span.start() + u32::try_from(start).unwrap(),
        token.span.end() + u32::try_from(start).unwrap(),
    );
    token
}

#[test]
fn logical_const_value_uses_variable_frame_not_command_namespace() {
    let source = "set x ROOT\nnamespace eval N {set x NAMESPACE; proc p {} {set x LOCAL; uplevel 0 {puts $x}; uplevel #0 {puts $x}; uplevel 1 {puts $x}}}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    for (context, expected) in [
        ("uplevel 0", Some("LOCAL")),
        ("uplevel #0", Some("ROOT")),
        ("uplevel 1", None),
    ] {
        let offset = source.find(context).unwrap();
        let start = source[offset..].find("$x").unwrap() + offset;
        let token = Token::with_content_offset(
            TokenType::Var,
            Span::new(
                u32::try_from(start).unwrap(),
                u32::try_from(start + 2).unwrap(),
            ),
            1,
        );
        assert_eq!(
            analyser
                .lookup_logical_const_substitution(token)
                .map(|(value, _)| value),
            expected,
            "{context}"
        );
    }
}

#[test]
fn logical_const_value_preserves_relative_qualified_namespace() {
    let source = "namespace eval N {namespace eval child {set x VALUE}; puts $child::x}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = logical_value_token(&analyser, "$child::x");
    assert_eq!(
        analyser
            .lookup_logical_const_substitution(token)
            .map(|(value, _)| value),
        Some("VALUE")
    );
}

#[test]
fn logical_const_value_requires_original_source_context_and_snapshot() {
    let source = "set x VALUE; puts $x";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = logical_value_token(&analyser, "$x");
    assert_eq!(
        analyser
            .lookup_logical_const_substitution(token)
            .map(|(value, _)| value),
        Some("VALUE")
    );
    let snapshot = analyser.snapshot();
    let input = analyser.result.resolved_input.as_ref().unwrap().clone();
    let mut changed_config = input.lexer_config();
    changed_config.strict_quoting = !changed_config.strict_quoting;
    analyser.result.resolved_input = Some(super::super::super::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        changed_config,
    ));
    assert!(analyser.lookup_logical_const_substitution(token).is_none());
    let foreign_store = std::sync::Arc::new(tcl_registry::CommandRegistry::build_default());
    let foreign = std::sync::Arc::new(
        input
            .borrowed_context_registry()
            .with_command_store(foreign_store),
    );
    analyser.result.resolved_input = Some(super::super::super::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        foreign,
        input.lexer_config(),
    ));
    assert!(analyser.lookup_logical_const_substitution(token).is_none());
    analyser.result.resolved_input = Some(input);
    analyser.source = source.replace("VALUE", "OTHER").into();
    assert!(analyser.lookup_logical_const_substitution(token).is_none());
    analyser.source = source.to_owned().into();
    analyser.result.resolved_input = None;
    assert!(analyser.lookup_logical_const_substitution(token).is_none());
    analyser.restore(snapshot);
    assert_eq!(
        analyser
            .lookup_logical_const_substitution(token)
            .map(|(value, _)| value),
        Some("VALUE")
    );
    analyser
        .const_strings
        .get_mut(&Vec::new())
        .unwrap()
        .get_mut("x")
        .unwrap()
        .0 = "OTHER".to_owned();
    assert!(analyser.lookup_logical_const_substitution(token).is_none());
}

#[test]
fn logical_const_value_does_not_resolve_array_receiver_from_literal_element() {
    let source = "set which live; set {targets($which)} FALSE; set targets(live) CORRECT; puts $targets($which)";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = logical_value_token(&analyser, "$targets($which)");
    assert!(analyser.lookup_logical_const_substitution(token).is_none());
    let source = "set {scalar(open} SCALAR; puts ${scalar(open}";
    let mut scalar = Analyser::new();
    scalar.analyse_and_retain_result_for_test(source, "tcl");
    let token = logical_value_token(&scalar, "${scalar(open}");
    assert_eq!(
        scalar
            .lookup_logical_const_substitution(token)
            .map(|(value, _)| value),
        Some("SCALAR")
    );
}

#[test]
fn logical_const_value_does_not_borrow_native_or_linked_cell_provenance() {
    let source = "set x ROOT; proc p {} {global x; puts $x}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = logical_value_token(&analyser, "$x");
    assert!(analyser.lookup_logical_const_substitution(token).is_none());
    let source = "set x VALUE; puts $x";
    let mut native = Analyser::new();
    native.analyse_and_retain_result_for_test(source, "tcl8.6");
    let token = logical_value_token(&native, "$x");
    assert!(!native.result.allows_lexical_declaration_advice());
    assert!(native.lookup_logical_const_substitution(token).is_none());
}

#[test]
fn logical_const_value_does_not_guess_an_object_method_namespace() {
    let source = "namespace eval lexical {namespace eval lexical {set x DEFINING}; oo::class create C {method read {} {puts $lexical::x}}}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = logical_value_token(&analyser, "$lexical::x");
    let image = tcl_lexer::SourceImage::document(source);
    let advice = analyser
        .result
        .logical_variable_scope_advice(&image, analyser.file_lexer_config(), token.span.start())
        .expect("actual method source frame");
    assert_eq!(
        advice.eligible_scopes().last().unwrap().kind,
        ScopeKind::Method
    );
    assert!(advice.namespace_variable_coordinate("lexical::x").is_none());
    assert!(analyser.lookup_logical_const_substitution(token).is_none());
}

#[test]
fn logical_const_value_refuses_non_dominating_and_future_writes() {
    for source in [
        "if {$flag} {set x BRANCH}; puts $x",
        "puts $x; set x FUTURE",
    ] {
        let mut analyser = Analyser::new();
        analyser.analyse_and_retain_result_for_test(source, "tcl");
        let token = logical_value_token(&analyser, "$x");
        assert!(
            analyser.lookup_logical_const_substitution(token).is_none(),
            "{source}"
        );
    }
}

#[test]
fn set_const_string_retains_the_value() {
    let mut a = Analyser::new();
    a.set_const_string("x", "hello".to_string(), span(0, 5), &[]);
    assert_eq!(
        a.const_strings
            .get(&Vec::new())
            .and_then(|values| values.get("x"))
            .map(|(value, _)| value.as_str()),
        Some("hello")
    );
}

#[test]
fn const_string_tables_keep_independent_values() {
    let mut a = make_analyser_with_namespace("ns1");
    a.set_const_string("x", "outer".to_string(), span(0, 5), &[]);
    a.set_const_string("x", "inner".to_string(), span(10, 15), &[0]);
    assert_eq!(
        a.const_strings
            .get(&vec![0])
            .and_then(|values| values.get("x"))
            .map(|(value, _)| value.as_str()),
        Some("inner")
    );
    assert_eq!(
        a.const_strings
            .get(&Vec::new())
            .and_then(|values| values.get("x"))
            .map(|(value, _)| value.as_str()),
        Some("outer")
    );
}

#[test]
fn clear_const_string_removes_entry() {
    let mut a = Analyser::new();
    a.set_const_string("x", "hello".to_string(), span(0, 5), &[]);
    a.clear_const_string("x", &[]);
    assert_eq!(
        a.const_strings
            .get(&Vec::new())
            .and_then(|values| values.get("x"))
            .map(|(value, _)| value.as_str()),
        None
    );
}

#[test]
fn const_string_table_starts_empty() {
    let a = Analyser::new();
    assert_eq!(
        a.const_strings
            .get(&Vec::new())
            .and_then(|values| values.get("x"))
            .map(|(value, _)| value.as_str()),
        None
    );
}
