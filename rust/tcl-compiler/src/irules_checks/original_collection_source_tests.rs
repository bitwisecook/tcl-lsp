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
use crate::compilation_unit::UnitBuildOptions;
use std::sync::Arc;

#[test]
fn conditional_leaf_census_does_not_treat_selected_lambda_roles_as_empty_scripts() {
    let environment = tcl_registry::model::resolve_known_environment("tcl8.6").unwrap();
    let context = environment.default_context_registry();
    let dialect = tcl_native_model::InvocationDialect::of_profile(environment.analyser_profile());
    let arguments = [
        tcl_registry::InvocationWord::Literal("value"),
        tcl_registry::InvocationWord::Literal("literal"),
    ];
    let selected = context.commands().resolve_structured_invocation(
        tcl_registry::InvocationWords::structured(
            tcl_registry::InvocationWord::Literal("set"),
            &arguments,
        )
        .with_dialect(dialect),
        Some(context.context().authoring_query()),
    );
    assert_eq!(
        leaf_handler_source_completion(&selected.resolved().unwrap()),
        Some(tcl_registry::registry::InvocationCompletion::FallsThrough)
    );
    for argument in [
        tcl_registry::InvocationWord::Literal("{} {drop}"),
        tcl_registry::InvocationWord::Dynamic,
    ] {
        let arguments = [argument];
        let selected = context.commands().resolve_structured_invocation(
            tcl_registry::InvocationWords::structured(
                tcl_registry::InvocationWord::Literal("apply"),
                &arguments,
            )
            .with_dialect(dialect),
            Some(context.context().authoring_query()),
        );
        let schema = selected
            .resolved()
            .expect("actual available Tcl8.6 Apply schema");
        assert!(schema.authored_source_lambda_call().is_some());
        assert_eq!(schema.authored_source_script_arguments(), Some(vec![0]));
        assert!(leaf_handler_source_completion(&schema).is_none());
    }
}

#[test]
fn conditional_source_flow_retains_phase_and_normal_continuation_obligations() {
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
    let source = "when HTTP_REQUEST {set value \"prefix[drop]\"; HTTP::respond 200; HTTP::header insert X yes}";
    let (unit, context) = supplied(source, config);
    let surface = Some(environment.unit_profile().surface_query());
    let advice = find_conditional_irule_source_flow_advice(&unit, context.commands(), surface)
        .expect("complete leaf source retains possible actions without executing them");
    for code in [DiagCode::Irule5002, DiagCode::Irule1201] {
        assert!(
            advice
                .findings()
                .iter()
                .any(|finding| finding.code() == code)
        );
    }
    assert!(advice.findings().iter().all(|finding| {
        finding
            .message()
            .contains("runtime reachability and completion unproved")
            && finding.warning.replacement.is_none()
            && finding.warning.fixes.is_empty()
    }));
    for obligation in [
        CollectionSourceObligationKind::UnknownTarget,
        CollectionSourceObligationKind::NormalContinuation,
    ] {
        assert!(advice.obligations().any(|current| *current == obligation));
    }
    assert!(
        advice
            .obligations()
            .any(|current| matches!(current, CollectionSourceObligationKind::EffectOnly(_)))
    );
    assert_eq!(
        find_unguarded_drop_warnings(&unit, surface),
        [] as [crate::irules_checks::IrulesCheckWarning; 0]
    );
    assert_eq!(
        find_http_flow_warnings(&unit, surface),
        [] as [crate::irules_checks::IrulesCheckWarning; 0]
    );
    for source in [
        "when HTTP_REQUEST {set value [return]suffix[drop]}",
        "when HTTP_REQUEST {if {$condition} {drop}}",
        "when HTTP_REQUEST {for {} {$condition} {drop} {HTTP::respond 200}}",
        "when HTTP_REQUEST {eval {drop}}",
        "when HTTP_REQUEST {HTTP::respond 200; return; HTTP::header insert X yes}",
    ] {
        let (unit, context) = supplied(source, config);
        assert!(
            find_conditional_irule_source_flow_advice(&unit, context.commands(), surface).is_none(),
            "{source}"
        );
    }
    let mut missing = unit.clone();
    for function in missing.procedures.values_mut() {
        for block in function.cfg.blocks.values_mut() {
            for statement in &mut block.statements {
                if let Some(tokens) = statement.tokens_mut()
                    && tokens.synthetic.is_none()
                {
                    tokens.source_binding = None;
                }
            }
        }
    }
    assert!(
        find_conditional_irule_source_flow_advice(&missing, context.commands(), surface).is_none()
    );
}

#[test]
fn conditional_collection_pairings_require_a_closed_source_census() {
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
    let source = "when HTTP_REQUEST {HTTP::release}";
    let (unit, context) = supplied(source, config);
    {
        let advice = find_conditional_collection_source_pairing_advice(
            &unit,
            context.commands(),
            Some(environment.unit_profile().surface_query()),
        )
        .expect("the complete original leaf handler has conditional source advice");
        assert!(
            advice
                .pairings()
                .iter()
                .any(|pairing| pairing.code() == DiagCode::Irule1008)
        );
        assert!(
            advice
                .pairings()
                .iter()
                .all(|pairing| pairing.message().contains("Conditional source pairing"))
        );
        assert!(
            advice
                .obligations()
                .any(|obligation| *obligation == CollectionSourceObligationKind::UnknownTarget)
        );
        assert!(
            advice
                .obligations()
                .any(|obligation| *obligation == CollectionSourceObligationKind::EventActivation)
        );
        assert!(
            advice.obligations().any(|obligation| *obligation
                == CollectionSourceObligationKind::SourceSchemaApplicability)
        );
    }
    for source in [
        "when HTTP_REQUEST {HTTP::release}\nwhen $event {HTTP::collect}",
        "when HTTP_REQUEST priority [unknown_header] {HTTP::release}",
        "when HTTP_REQUEST priority [eval {when HTTP_RESPONSE {HTTP::collect}}] {HTTP::release}",
        "when HTTP_REQUEST {eval {HTTP::collect}; HTTP::release}",
        "when HTTP_REQUEST {expr {[HTTP::collect]}; HTTP::release}",
        "when HTTP_REQUEST {after 0 {HTTP::collect}; HTTP::release}",
        "unknown_constructor\nwhen HTTP_REQUEST {HTTP::release}",
        "when HTTP_REQUEST {HTTP::release}\nwhen HTTP_RESPONSE {",
        "when HTTP_REQUEST {HTTP::release; return; HTTP::collect}",
    ] {
        let (unit, context) = supplied(source, config);
        assert!(
            find_conditional_collection_source_pairing_advice(
                &unit,
                context.commands(),
                Some(environment.unit_profile().surface_query()),
            )
            .is_none(),
            "{source}"
        );
    }
    for axis in 0..3 {
        let mut stale = unit.clone();
        match axis {
            0 => stale.ir_module.source = tcl_lexer::SourceImage::document("foreign source"),
            1 => stale.ir_module.lexer_config.leading_bom = tcl_lexer::LeadingBom::Skip,
            _ => {
                for function in stale.procedures.values_mut() {
                    for block in function.cfg.blocks.values_mut() {
                        for statement in &mut block.statements {
                            if let Some(tokens) = statement.tokens_mut()
                                && tokens.synthetic.is_none()
                            {
                                tokens.source_binding = None;
                            }
                        }
                    }
                }
            }
        }
        assert!(
            find_conditional_collection_source_pairing_advice(
                &stale,
                context.commands(),
                Some(environment.unit_profile().surface_query()),
            )
            .is_none(),
            "axis {axis}"
        );
    }
}

#[test]
fn unknown_collection_effects_keep_source_calls_without_certifying_absence() {
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
    // Unknown effects need unknown prior history, which a supplied
    // document no longer implies: a fresh rule load resolves
    // `HTTP::release` (user decision 2026-10-21T00:10Z). The explicit
    // unknown-history entry keeps this contract's premise.
    let (mut unit, context) =
        supplied_with_history("when HTTP_REQUEST {HTTP::release}", config, true);
    let query = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
    let state = collect_flow_state(&query).unwrap().0;
    assert_eq!(state.release_calls.len(), 1);
    assert!(
        state
            .obligations
            .contains(&CollectionSourceObligationKind::UnknownTarget)
    );
    assert_eq!(
        find_collect_flow_warnings(
            &unit,
            context.commands(),
            Some(environment.unit_profile().surface_query())
        ),
        [] as [crate::irules_checks::IrulesCheckWarning; 0]
    );
    let mut changed = false;
    for function in unit.procedures.values_mut() {
        for block in function.cfg.blocks.values_mut() {
            for statement in &mut block.statements {
                if matches!(
                    statement.original_argv_inventory_disposition(),
                    crate::ir::OriginalArgvInventoryDisposition::EffectOnly(
                        crate::ir::SyntheticMarker::GlobalFrameScript
                            | crate::ir::SyntheticMarker::CallerFrameOpaque
                    )
                ) {
                    statement
                        .tokens_mut()
                        .unwrap()
                        .argv_texts
                        .push("unowned argv".to_owned());
                    changed = true;
                    break;
                }
            }
            if changed {
                break;
            }
        }
        if changed {
            break;
        }
    }
    assert!(changed);
    let query = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
    assert!(collect_flow_state(&query).is_none());
}

fn supplied(
    source: &str,
    config: tcl_lexer::LexerConfig,
) -> (CompilationUnit, Arc<tcl_registry::model::ContextRegistry>) {
    supplied_with_history(source, config, false)
}

/// `unknown_history` supplies the explicit unknown-history entry; without
/// it a supplied document is a fresh rule load (user decision
/// 2026-10-21T00:10Z).
fn supplied_with_history(
    source: &str,
    config: tcl_lexer::LexerConfig,
    unknown_history: bool,
) -> (CompilationUnit, Arc<tcl_registry::model::ContextRegistry>) {
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
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
        unknown_history
            .then(|| crate::command_binding::SourceAnalysisEntry::unknown_history_for_test(&input))
            .as_ref(),
        &input,
    );
    (unit, context)
}

#[test]
fn collection_query_uses_actual_function_config_and_separates_standalone() {
    // naming.source.original-collection-body-owner
    // docs/design/analysis/name-resolution-proofs/original-collection-body-owner.md
    // Actual source/config + extracted syntax purpose, not BIG-IP execution.
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
    let config = tcl_lexer::LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        base_line: 17,
        ..tcl_lexer::LexerConfig::from_grammar(environment.grammar())
    };
    let (unit, context) = supplied("when HTTP_REQUEST {HTTP::collect; HTTP::release}", config);
    let source = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
    assert!(!source.standalone);
    assert!(source.metadata().is_some());
    let function = unit
        .procedures
        .values()
        .find(|fu| fu.name.starts_with("::when::"))
        .unwrap();
    let actual = source.function_config(function).unwrap();
    assert_eq!(actual, function.source_lexer_config());
    let local = tcl_lexer::SourceSegmentationPurpose::NestedScript.config(actual);
    assert_eq!(local.leading_bom, tcl_lexer::LeadingBom::Content);
    let (head, _) =
        parse_command_substitution_with_config("[\u{feff}HTTP::payload]", local).unwrap();
    assert_eq!(head, "\u{feff}HTTP::payload");
    let (head, _) = parse_command_substitution_with_config("[HTTP::payload]", local).unwrap();
    assert_eq!(head, "HTTP::payload");
    let standalone = CompilationUnit::build_for_with_config(
        "when HTTP_REQUEST {HTTP::collect; HTTP::release}",
        context.commands(),
        false,
        config,
    );
    let separate = IrulesSourceQuery::capture(&standalone, context.commands()).unwrap();
    assert!(separate.standalone);
    assert!(separate.metadata().is_none());
    assert!(collect_flow_state(&separate).is_some());
}

#[test]
fn collection_source_metadata_keeps_original_names_and_ignores_report_labels() {
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
    let (mut unit, context) = supplied(
        "when HTTP_REQUEST {HTTP::release; :HTTP::collect; set value {[HTTP::collect]}}",
        config,
    );
    let query = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
    let original = collect_flow_state(&query).unwrap().0;
    assert!(original.collect_calls.is_empty());
    assert_eq!(original.release_calls.len(), 1);
    let mut changed = false;
    for function in unit.procedures.values_mut() {
        for block in function.cfg.blocks.values_mut() {
            for statement in &mut block.statements {
                if let Statement::Call {
                    command,
                    canonical_command,
                    tokens: Some(tokens),
                    ..
                }
                | Statement::Barrier {
                    command,
                    canonical_command,
                    tokens: Some(tokens),
                    ..
                } = statement
                    && tokens
                        .argv_texts
                        .first()
                        .is_some_and(|word| word == "HTTP::release")
                {
                    *command = "HTTP::collect".to_owned();
                    *canonical_command = Some("::HTTP::collect".to_owned());
                    changed = true;
                }
            }
        }
    }
    assert!(changed);
    let query = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
    let actual = collect_flow_state(&query).unwrap().0;
    assert!(actual.collect_calls.is_empty());
    assert_eq!(actual.release_calls.len(), 1);
}

#[test]
fn collection_source_keeps_original_template_index_and_head_substitutions() {
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
    for body in [
        r#"set value "prefix[HTTP::collect]""#,
        "set value $a([HTTP::collect])",
        "[HTTP::collect]",
    ] {
        let (unit, context) = supplied(
            &format!("when HTTP_REQUEST {{{body}; HTTP::release}}"),
            config,
        );
        let query = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
        let state = collect_flow_state(&query).unwrap().0;
        assert_eq!(state.collect_calls.len(), 1, "{body}");
        assert_eq!(state.release_calls.len(), 1, "{body}");
    }
    let (unit, context) = supplied(
        "when HTTP_REQUEST {set value {prefix[HTTP::collect]}; HTTP::release}",
        config,
    );
    let query = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
    let state = collect_flow_state(&query).unwrap().0;
    assert!(state.collect_calls.is_empty());
    assert_eq!(state.release_calls.len(), 1);
}

#[test]
fn collection_source_retains_admitted_nonregistry_commands() {
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
    let (unit, context) = supplied(
        "proc ordinary {} {return value}; when HTTP_REQUEST {ordinary; HTTP::release}",
        config,
    );
    let query = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
    let state = collect_flow_state(&query).unwrap().0;
    assert!(state.collect_calls.is_empty());
    assert_eq!(state.release_calls.len(), 1);
}

#[test]
fn assigned_collection_substitution_uses_its_original_child_once() {
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
    let (mut unit, context) = supplied(
        "when HTTP_REQUEST {set value [HTTP::collect]; HTTP::release}",
        config,
    );
    let query = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
    let state = collect_flow_state(&query).unwrap().0;
    assert_eq!(state.collect_calls.len(), 1);
    assert_eq!(state.release_calls.len(), 1);
    let mut removed = false;
    for function in unit.procedures.values_mut() {
        for block in function.cfg.blocks.values_mut() {
            for statement in &mut block.statements {
                if statement.tokens().is_some_and(|tokens| {
                    tokens.argv_texts.first().is_some_and(|head| head == "set")
                }) {
                    match statement {
                        Statement::AssignValue { tokens, .. }
                        | Statement::Call { tokens, .. }
                        | Statement::Barrier { tokens, .. } => {
                            *tokens = None;
                            removed = true;
                        }
                        _ => panic!("unexpected original set carrier"),
                    }
                }
            }
        }
    }
    assert!(removed);
    let query = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
    assert!(collect_flow_state(&query).is_none());
}

#[test]
fn collection_withdrawal_is_unavailable_instead_of_an_empty_pairing_inventory() {
    // naming.source.original-collection-body-owner
    // docs/design/analysis/name-resolution-proofs/original-collection-body-owner.md
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
    let (unit, context) = supplied(
        "when HTTP_REQUEST {clientside {HTTP::collect}; HTTP::release}",
        config,
    );
    let source = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
    assert!(collect_flow_state(&source).is_some());
    for axis in 0..6 {
        let mut changed = unit.clone();
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
        let query = IrulesSourceQuery::capture(&changed, context.commands());
        assert!(
            query
                .as_ref()
                .is_none_or(|query| collect_flow_state(query).is_none()),
            "axis {axis}"
        );
        assert!(
            find_collect_flow_warnings(
                &changed,
                context.commands(),
                Some(environment.unit_profile().surface_query())
            )
            .is_empty(),
            "axis {axis}"
        );
    }
    let mut wrong = unit.clone();
    for function in wrong.procedures.values_mut() {
        for block in function.cfg.blocks.values_mut() {
            for statement in &mut block.statements {
                if let Some(tokens) = statement.tokens_mut()
                    && tokens
                        .argv_texts
                        .first()
                        .is_some_and(|head| head == "clientside")
                {
                    tokens.argv_texts[1].push_str("foreign");
                }
            }
        }
    }
    let source = IrulesSourceQuery::capture(&wrong, context.commands()).unwrap();
    assert!(collect_flow_state(&source).is_none());
}
