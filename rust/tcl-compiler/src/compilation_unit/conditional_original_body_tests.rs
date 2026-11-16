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

fn supplied(
    source: &str,
    dialect: &str,
    config: tcl_lexer::LexerConfig,
) -> (CompilationUnit, Arc<tcl_registry::model::ContextRegistry>) {
    let environment = tcl_registry::model::resolve_known_environment(dialect).unwrap();
    let context = environment.default_context_registry();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        environment.analyser_profile(),
        environment.unit_profile(),
        Arc::clone(&context),
        config,
    );
    let unit = CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(environment.unit_profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    );
    (unit, context)
}

fn side_parent(unit: &CompilationUnit) -> (&FunctionUnit, &crate::ir::CommandTokens) {
    let function = unit
        .procedures
        .values()
        .find(|fu| fu.name.starts_with("::when::"))
        .expect("actual event function");
    let tokens = function
        .cfg
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .filter_map(|statement| statement.tokens())
        .find(|tokens| {
            tokens
                .argv_texts
                .first()
                .is_some_and(|head| head == "clientside")
        })
        .expect("original side-switch vector");
    (function, tokens)
}

#[test]
fn conditional_original_child_keeps_source_config_namespace_and_has_no_event_grant() {
    // naming.source.original-collection-body-owner
    // docs/design/analysis/name-resolution-proofs/original-collection-body-owner.md
    // Conditional original syntax/CFG only; no BIG-IP entry or worker claim.
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
    let config = tcl_lexer::LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        base_line: 11,
        ..tcl_lexer::LexerConfig::from_grammar(environment.grammar())
    };
    let (unit, context) = supplied(
        "when HTTP_REQUEST {clientside {HTTP::collect}; HTTP::release}",
        "f5-irules",
        config,
    );
    let (function, tokens) = side_parent(&unit);
    let script = crate::lowering::Lowerer::original_body_for_module(
        &unit.ir_module,
        context.commands(),
        tokens,
        0,
    )
    .expect("genuine retained child source");
    assert!(script.namespace_context.is_some());
    let original_source = script.executed_source.as_ref().unwrap();
    assert_eq!(original_source.try_text().unwrap(), "HTTP::collect");
    let child = unit
        .conditional_original_body_unit(context.commands(), function, tokens, 0)
        .expect("conditional original child");
    assert_eq!(
        child.source_lexer_config(),
        function.source_lexer_config().nested()
    );
    assert_eq!(
        child.source_metadata_input(),
        Some(
            &unit
                .ir_module
                .source_metadata_input
                .as_ref()
                .unwrap()
                .for_nested_source()
        )
    );
    assert_eq!(child.cfg.executed_source.as_ref(), Some(original_source));
    assert!(child.irules_event_body.is_none());
    assert!(
        child
            .invocation_metadata_context(context.commands())
            .is_some()
    );
    assert!(
        child
            .invocation_metadata_context_for_module(context.commands(), &unit.ir_module)
            .is_some()
    );
    assert_eq!(child.cfg.namespace_context, script.namespace_context);
    assert!(
        unit.conditional_original_body_unit(context.commands(), &child, tokens, 0)
            .is_none()
    );
    let detached = function.clone();
    assert!(
        unit.conditional_original_body_unit(context.commands(), &detached, tokens, 0)
            .is_none()
    );
    let mut changed = tokens.clone();
    changed.argv_texts[1].push_str("foreign");
    assert!(
        unit.conditional_original_body_unit(context.commands(), function, &changed, 0)
            .is_none()
    );
    assert!(
        unit.conditional_original_body_unit(context.commands(), function, tokens, 1)
            .is_none()
    );
}

#[test]
fn nested_original_children_share_the_actual_root_function_owner() {
    // naming.source.original-collection-body-owner
    // docs/design/analysis/name-resolution-proofs/original-collection-body-owner.md
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
    let (unit, context) = supplied(
        "when HTTP_REQUEST {clientside {serverside {TCP::collect}}}",
        "f5-irules",
        config,
    );
    let (original_function, parent) = side_parent(&unit);
    let child = unit
        .conditional_original_body_unit(context.commands(), original_function, parent, 0)
        .expect("first original child");
    let nested = child
        .cfg
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .filter_map(|statement| statement.tokens())
        .find(|tokens| {
            tokens
                .argv_texts
                .first()
                .is_some_and(|head| head == "serverside")
        })
        .expect("actual nested parent vector");
    let grandchild = unit
        .conditional_original_body_unit(context.commands(), original_function, nested, 0)
        .expect("nested original child");
    assert_eq!(
        grandchild
            .cfg
            .executed_source
            .as_ref()
            .unwrap()
            .try_text()
            .unwrap(),
        "TCP::collect"
    );
    assert_eq!(
        grandchild.source_metadata_input(),
        original_function.source_metadata_input()
    );
    assert!(grandchild.irules_event_body.is_none());
    assert!(
        unit.conditional_original_body_unit(context.commands(), &child, nested, 0)
            .is_none()
    );
}

#[test]
fn original_child_withdraws_missing_foreign_and_changed_complete_owners() {
    // naming.source.original-collection-body-owner
    // docs/design/analysis/name-resolution-proofs/original-collection-body-owner.md
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
    let source = "when HTTP_REQUEST {clientside {HTTP::collect}}";
    let (original, context) = supplied(source, "f5-irules", config);
    let (function, tokens) = side_parent(&original);
    assert!(
        original
            .conditional_original_body_unit(context.commands(), function, tokens, 0)
            .is_some()
    );
    for axis in 0..6 {
        let mut changed = original.clone();
        match axis {
            0 => changed.source.push_str("foreign"),
            1 => changed.ir_module.source_metadata_input = None,
            2 => changed.ir_module.retained_source_bindings = None,
            3 => changed.ir_module.lexer_config.leading_bom = tcl_lexer::LeadingBom::Skip,
            4 => changed.source_class_facts = None,
            _ => {
                for function in changed.procedures.values_mut() {
                    function.source_metadata_input = None;
                }
            }
        }
        let (function, tokens) = side_parent(&changed);
        assert!(
            changed
                .conditional_original_body_unit(context.commands(), function, tokens, 0)
                .is_none(),
            "axis {axis}"
        );
    }
    let foreign = tcl_registry::model::resolve_known_environment("tcl9.1")
        .unwrap()
        .default_context_registry();
    assert!(
        original
            .conditional_original_body_unit(foreign.commands(), function, tokens, 0)
            .is_none()
    );
    let foreign_config = tcl_lexer::LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        ..config
    };
    let (fresh, _) = supplied(source, "f5-irules", foreign_config);
    let mut donated = original.clone();
    donated.source_class_facts = fresh.source_class_facts;
    let (function, tokens) = side_parent(&donated);
    assert!(
        donated
            .conditional_original_body_unit(context.commands(), function, tokens, 0)
            .is_none()
    );
}

#[test]
fn genuine_roster_retains_empty_classes_and_refuses_owner_donation() {
    // naming.source.original-collection-body-owner
    // docs/design/analysis/name-resolution-proofs/original-collection-body-owner.md
    // Descriptive class spelling only; no Native class allocation/constructor.
    let environment = tcl_registry::model::resolve_known_environment("tcl").unwrap();
    let config = tcl_lexer::LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        base_line: 9,
        ..tcl_lexer::LexerConfig::from_grammar(environment.grammar())
    };
    let source = "oo::class create Empty {}";
    let (unit, context) = supplied(source, "tcl", config);
    assert!(unit.ir_module.methods.is_empty());
    let facts = unit.source_class_facts.as_ref().expect("actual roster");
    assert!(
        facts
            .for_module(&unit.ir_module, context.commands())
            .unwrap()
            .contains("::Empty")
    );
    let same = unit.clone();
    assert!(Arc::ptr_eq(
        facts,
        same.source_class_facts.as_ref().unwrap()
    ));
    assert!(
        facts
            .for_module(&same.ir_module, context.commands())
            .is_some()
    );
    // A semantic Eq source owner is descriptive, not an allocation or
    // lifetime receipt. Fresh-vs-memo CU equality must not depend on Arc addresses.
    let fresh = CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(environment.unit_profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        unit.ir_module.source_metadata_input.as_ref().unwrap(),
    );
    assert_eq!(
        unit.ir_module.retained_source_bindings,
        fresh.ir_module.retained_source_bindings
    );
    assert_eq!(unit.source_class_facts, fresh.source_class_facts);
    assert!(
        facts
            .for_module(&fresh.ir_module, context.commands())
            .is_some()
    );
    let (foreign, _) = supplied("oo::class create Foreign {}", "tcl", config);
    assert!(
        facts
            .for_module(&foreign.ir_module, context.commands())
            .is_none()
    );
    let mut changed = unit.ir_module.clone();
    changed.lexer_config.base_line += 1;
    assert!(facts.for_module(&changed, context.commands()).is_none());
    changed = unit.ir_module.clone();
    changed.source = tcl_lexer::SourceImage::native(source.as_bytes());
    assert!(facts.for_module(&changed, context.commands()).is_none());
    changed = unit.ir_module.clone();
    changed.source_metadata_input = None;
    assert!(facts.for_module(&changed, context.commands()).is_none());
}

#[test]
fn conditional_original_child_inherits_unknown_presence_and_refines_local_stores() {
    // naming.source.original-collection-body-owner
    // docs/design/analysis/name-resolution-proofs/original-collection-body-owner.md
    // Conditional source/lattice facts only; no provider or physical frame claim.
    let environment = tcl_registry::model::resolve_known_environment("tcl8.6").unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
    let source = "proc p {} {set inherited prior; eval {set copy $inherited; set own fresh; set before $own; unset own; set after $own}}; p";
    let (unit, context) = supplied(source, "tcl8.6", config);
    let original = unit.function("::p").expect("actual caller function");
    let tokens = original
        .cfg
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .filter_map(|statement| statement.tokens())
        .find(|tokens| tokens.argv_texts.first().is_some_and(|head| head == "eval"))
        .expect("original eval parent");
    let child = unit
        .conditional_original_body_unit(context.commands(), original, tokens, 0)
        .expect("actual conditional eval body");
    let inherited = child.ssa.var_symbol("inherited").expect("inherited read");
    let own = child.ssa.var_symbol("own").expect("written local");
    assert_eq!(
        child.sccp.existence.get(&(inherited, 0)),
        Some(&tcl_registry::value_transfer::Existence::MayBound)
    );
    assert_eq!(
        child.sccp.existence.get(&(own, 0)),
        Some(&tcl_registry::value_transfer::Existence::MayBound)
    );
    let mut reads: Vec<_> = child
        .sccp
        .existence_reads
        .iter()
        .filter(|((_, _, symbol), _)| *symbol == own)
        .filter_map(|(&(block, index, _), &fact)| {
            let statement = &child.cfg.blocks[&block].statements[index as usize];
            matches!(statement, crate::ir::Statement::AssignValue { value, .. } if value == "$own")
                .then_some((statement.span().start(), fact))
        })
        .collect();
    reads.sort_by_key(|(offset, _)| *offset);
    assert_eq!(
        reads.iter().map(|(_, fact)| *fact).collect::<Vec<_>>(),
        vec![
            tcl_registry::value_transfer::Existence::Bound(
                tcl_registry::value_transfer::BindingKind::Scalar
            ),
            tcl_registry::value_transfer::Existence::Unbound,
        ]
    );
    assert!(child.irules_event_body.is_none());
    assert!(
        !child
            .sccp
            .values
            .get(&(inherited, 0))
            .is_some_and(|value| matches!(value, crate::analyses::LatticeValue::Const(_)))
    );
}

#[test]
fn conditional_empty_child_has_no_report_selected_frame_or_parent_entry_donation() {
    // naming.source.original-collection-body-owner
    // docs/design/analysis/name-resolution-proofs/original-collection-body-owner.md
    // Source entry/API control only; no entered caller, event or Native frame.
    let environment = tcl_registry::model::resolve_known_environment("tcl8.6").unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
    let (unit, context) = supplied("eval {}", "tcl8.6", config);
    let tokens = unit
        .top_level
        .cfg
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .filter_map(|statement| statement.tokens())
        .find(|tokens| tokens.argv_texts.first().is_some_and(|head| head == "eval"))
        .expect("original empty body parent");
    let child = unit
        .conditional_original_body_unit(context.commands(), &unit.top_level, tokens, 0)
        .expect("genuine empty conditional body");
    let fallback = conditional_function_source_entry(&child.cfg, child.source_metadata_input());
    assert_eq!(
        fallback.frame_kind,
        crate::var_resolve::VariableFrameKind::Unknown
    );
    assert!(fallback.activation.is_none());
    assert!(fallback.execution.is_none());
    assert!(fallback.instance_vars.is_empty());
    assert!(fallback.execution_name_policy.is_none());
    assert_eq!(
        fallback.namespace_identity.as_ref(),
        child.cfg.namespace_context.as_deref()
    );
    let selected = function_source_entry(&child.cfg, Some(&fallback));
    assert_eq!(
        selected.frame_kind,
        crate::var_resolve::VariableFrameKind::Unknown
    );
    assert!(selected.activation.is_none());
    let mut renamed = child.cfg.clone();
    renamed.name = "::invented::procedure".to_owned();
    assert_eq!(function_source_entry(&renamed, Some(&fallback)), selected);
    let mut changed = unit.clone();
    changed.ir_module.source_entry.unknown_entry = true;
    assert!(
        changed
            .conditional_original_body_unit(context.commands(), &changed.top_level, tokens, 0)
            .is_none()
    );
}
