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
use crate::analyser::ResolvedAnalysisInput;
use crate::compilation_unit::UnitBuildOptions;
use std::sync::Arc;

fn supplied(
    config: tcl_lexer::LexerConfig,
) -> (CompilationUnit, Arc<tcl_registry::model::ContextRegistry>) {
    supplied_source("when HTTP_REQUEST {set u [HTTP::uri]}", config)
}

fn supplied_source(
    source: &str,
    config: tcl_lexer::LexerConfig,
) -> (CompilationUnit, Arc<tcl_registry::model::ContextRegistry>) {
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
    let context = environment.default_context_registry();
    let input = ResolvedAnalysisInput::new(
        environment.analyser_profile(),
        environment.unit_profile(),
        Arc::clone(&context),
        config,
    );
    let cu = CompilationUnit::build_with_analysis_input(
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
    (cu, context)
}

#[test]
fn getter_source_keeps_function_config_and_selects_nested_parser_purpose() {
    // naming.source.retained-explorer-consumers
    // docs/design/analysis/name-resolution-proofs/retained-explorer-consumers.md
    // Actual source metadata and parser-purpose control, no BIG-IP execution.
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
    let config = tcl_lexer::LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        base_line: 17,
        ..tcl_lexer::LexerConfig::from_grammar(environment.grammar())
    };
    let (cu, context) = supplied(config);
    let function = cu
        .functions()
        .find(|f| f.name.starts_with("::when::"))
        .expect("event function");
    let selected =
        unnormalised_getter_source_config(&cu.ir_module, function, context.commands(), false)
            .unwrap();
    assert_eq!(selected, function.source_lexer_config());
    assert_ne!(
        selected,
        tcl_lexer::LexerConfig::for_profile(context.commands().profile())
    );
    let local = tcl_lexer::SourceSegmentationPurpose::NestedScript.config(selected);
    assert_eq!(local, selected.nested());
    assert_eq!(local.leading_bom, tcl_lexer::LeadingBom::Content);
    assert_eq!(selected.leading_bom, tcl_lexer::LeadingBom::Skip);
    let warnings = find_unnormalised_getter_warnings(
        &cu,
        context.commands(),
        Some(environment.unit_profile().surface_query()),
    );
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, DiagCode::Irule3102);
}

#[test]
fn getter_source_keeps_original_children_and_ignores_report_values() {
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
    let query = Some(environment.unit_profile().surface_query());
    for body in [
        r#"set value "prefix[HTTP::uri]""#,
        "set value $a([HTTP::uri])",
        "set value 0; incr value [HTTP::uri]",
        "[HTTP::uri]",
    ] {
        let (unit, context) = supplied_source(&format!("when HTTP_REQUEST {{{body}}}"), config);
        let warnings = find_unnormalised_getter_warnings(&unit, context.commands(), query);
        assert_eq!(warnings.len(), 1, "{body}");
        assert!(warnings[0].message.contains("HTTP::uri -normalized"));
    }
    for source in [
        "when HTTP_REQUEST {set value {prefix[HTTP::uri]}; :HTTP::uri}",
        "proc HTTP::uri {} {return value}; when HTTP_REQUEST {HTTP::uri}",
    ] {
        let (unit, context) = supplied_source(source, config);
        assert!(
            find_unnormalised_getter_warnings(&unit, context.commands(), query).is_empty(),
            "{source}"
        );
    }
    let (mut unit, context) = supplied_source("when HTTP_REQUEST {HTTP::uri}", config);
    assert_eq!(
        find_unnormalised_getter_warnings(&unit, context.commands(), query).len(),
        1
    );
    let mut changed = false;
    for function in unit.procedures.values_mut() {
        for block in function.cfg.blocks.values_mut() {
            for statement in &mut block.statements {
                if let Statement::Call {
                    command,
                    canonical_command,
                    args,
                    ..
                }
                | Statement::Barrier {
                    command,
                    canonical_command,
                    args,
                    ..
                } = statement
                {
                    *command = "HTTP::header".to_owned();
                    *canonical_command = Some("::HTTP::header".to_owned());
                    *args = vec!["-normalized".to_owned()];
                    changed = true;
                }
            }
        }
    }
    assert!(changed);
    let warnings = find_unnormalised_getter_warnings(&unit, context.commands(), query);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].message.contains("HTTP::uri -normalized"));
}

#[test]
fn getter_source_refuses_missing_foreign_and_changed_actual_owners() {
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
    let (original, context) = supplied(config);
    let query = Some(environment.unit_profile().surface_query());
    assert_eq!(
        find_unnormalised_getter_warnings(&original, context.commands(), query).len(),
        1
    );
    for change in 0..4 {
        let mut cu = original.clone();
        match change {
            0 => cu.ir_module.source_metadata_input = None,
            1 => cu.ir_module.retained_source_bindings = None,
            2 => cu.ir_module.source = tcl_lexer::SourceImage::document("foreign source"),
            _ => cu.ir_module.lexer_config.leading_bom = tcl_lexer::LeadingBom::Skip,
        }
        assert!(
            find_unnormalised_getter_warnings(&cu, context.commands(), query).is_empty(),
            "Module axis {change}"
        );
    }
    let mut cu = original.clone();
    for function in cu.procedures.values_mut() {
        function.source_metadata_input = None;
    }
    assert_eq!(
        find_unnormalised_getter_warnings(&cu, context.commands(), query),
        [] as [crate::irules_checks::IrulesCheckWarning; 0]
    );
    let foreign = tcl_registry::model::resolve_known_environment("tcl9.1")
        .unwrap()
        .default_context_registry();
    assert_eq!(
        find_unnormalised_getter_warnings(&original, foreign.commands(), query),
        [] as [crate::irules_checks::IrulesCheckWarning; 0]
    );
    assert_eq!(
        find_unnormalised_getter_warnings(&original, context.commands(), query).len(),
        1
    );
}
