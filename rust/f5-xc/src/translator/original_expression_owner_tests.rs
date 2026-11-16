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

fn first_clause(module: &Module) -> &tcl_compiler::ir::IfClause {
    module
        .procedures
        .values()
        .flat_map(|procedure| &procedure.body.statements)
        .find_map(|statement| match statement {
            Statement::If { clauses, .. } => clauses.first(),
            _ => None,
        })
        .expect("original fixture condition")
}

#[test]
fn command_expression_operands_retain_their_complete_source_extent() {
    for getter in [
        "[HTTP::path]",
        "[HTTP::header \"X-λ\"]",
        "[string tolower [HTTP::path]]",
    ] {
        let source = format!("# λ\nwhen HTTP_REQUEST {{if {{{getter} eq \"/a\"}} {{pool route}}}}");
        let registry = irules_registry();
        let (unit, analysis) = translation_source(&source, registry);
        let clause = first_clause(&unit.ir_module);
        let context = TranslationContext::new(&source, &analysis, &unit.ir_module, registry);
        let advice = SourceExpressionAdvice::new(&context, clause.condition_base);
        let ExprNode::Binary { left, .. } = &clause.condition else {
            panic!("the fixture is a comparison");
        };
        let operand = advice
            .expression_operand(left)
            .expect("the complete original getter operand");
        assert_eq!(source.get(operand.span().as_range()), Some(getter));
        assert_eq!(operand.image(), &unit.ir_module.source);
        assert_eq!(operand.config(), unit.ir_module.lexer_config);
        assert!(advice.command_for_operand(&operand).is_some(), "{getter}");
        assert!(
            advice.command_owners_available(&clause.condition),
            "{getter}"
        );

        let mut clipped = left.as_ref().clone();
        let ExprNode::Command { end, .. } = &mut clipped else {
            panic!("the fixture's left operand is a command");
        };
        *end -= 1;
        assert!(advice.expression_operand(&clipped).is_none());
    }
}

#[test]
fn getter_source_advice_retains_original_positions_input_and_module() {
    let source = r#"when HTTP_REQUEST {if {[HTTP::path] eq "/a"} {pool route}}"#;
    let registry = irules_registry();
    let (unit, analysis) = translation_source(source, registry);
    let clause = first_clause(&unit.ir_module);
    let context = TranslationContext::new(source, &analysis, &unit.ir_module, registry);
    let advice = SourceExpressionAdvice::new(&context, clause.condition_base);
    assert_eq!(
        condition_to_path_match(&clause.condition, &advice)
            .unwrap()
            .value,
        "/a"
    );
    let unavailable = SourceExpressionAdvice::new(&context, None);
    assert!(condition_to_path_match(&clause.condition, &unavailable).is_none());
    let stale = TranslationContext::new(
        "when HTTP_REQUEST {if {[HTTP::path] eq \"/changed\"} {pool route}}",
        &analysis,
        &unit.ir_module,
        registry,
    );
    assert!(
        condition_to_path_match(
            &clause.condition,
            &SourceExpressionAdvice::new(&stale, clause.condition_base)
        )
        .is_none()
    );
    let mut missing = unit.ir_module.clone();
    missing.retained_source_bindings = None;
    let missing_context = TranslationContext::new(source, &analysis, &missing, registry);
    assert!(
        condition_to_path_match(
            &clause.condition,
            &SourceExpressionAdvice::new(&missing_context, clause.condition_base)
        )
        .is_none()
    );
    let mut changed_config = unit.ir_module.clone();
    changed_config.lexer_config.strict_quoting = !changed_config.lexer_config.strict_quoting;
    let changed_context = TranslationContext::new(source, &analysis, &changed_config, registry);
    assert!(
        condition_to_path_match(
            &clause.condition,
            &SourceExpressionAdvice::new(&changed_context, clause.condition_base)
        )
        .is_none()
    );
    let mut report_changed = unit.ir_module.clone();
    for procedure in report_changed.procedures.values_mut() {
        procedure.qualified_name = "counterfactual".into();
    }
    let report_context = TranslationContext::new(source, &analysis, &report_changed, registry);
    assert_eq!(
        condition_to_path_match(
            &clause.condition,
            &SourceExpressionAdvice::new(&report_context, clause.condition_base)
        )
        .unwrap()
        .value,
        "/a"
    );
    let foreign_registry =
        tcl_registry::model::resolve_environment("tcl8.6").default_context_registry();
    let foreign = TranslationContext::new(
        source,
        &analysis,
        &unit.ir_module,
        foreign_registry.commands(),
    );
    assert!(
        condition_to_path_match(
            &clause.condition,
            &SourceExpressionAdvice::new(&foreign, clause.condition_base)
        )
        .is_none()
    );
}

#[test]
fn replaced_getters_and_literal_switch_brackets_issue_no_getter_route() {
    let replaced = translate_irule(
        r#"proc HTTP::path {} {return /fake}; when HTTP_REQUEST {if {[HTTP::path] eq "/a"} {pool route}}"#,
    );
    assert_eq!(replaced.routes, [] as [crate::model::XCRoute; 0]);
    let literal =
        translate_irule(r#"when HTTP_REQUEST {switch {[HTTP::path]} {"/a" {pool route}}}"#);
    assert!(
        literal
            .routes
            .iter()
            .all(|route| route.path_match.is_none())
    );
    let nested = translate_irule(
        r#"when HTTP_REQUEST {if {[string tolower [HTTP::path]] eq "/a"} {pool route}}"#,
    );
    assert!(nested.routes.iter().any(|route| {
        route
            .path_match
            .as_ref()
            .is_some_and(|path| path.value == "/a")
    }));
}
