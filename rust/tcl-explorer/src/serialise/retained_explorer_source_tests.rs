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
use std::sync::Arc;
use tcl_compiler::analyser::ResolvedAnalysisInput;
use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};

fn fixture(
    source: &str,
    engine: &str,
    registry: &tcl_registry::CommandRegistry,
    config: LexerConfig,
) -> ExplorerResult {
    let environment =
        tcl_registry::model::resolve_known_environment(engine).expect("known provider ingress");
    let context = Arc::new(
        environment
            .default_context_registry()
            .with_command_store(registry.snapshot().shared_registry()),
    );
    let input = ResolvedAnalysisInput::new(
        environment.analyser_profile(),
        environment.unit_profile(),
        context,
        config,
    );
    let unit = CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config,
            dialect: Some(environment.unit_profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    )
    .with_interprocedural(registry, Some(environment.unit_profile()));
    ExplorerResult {
        source: source.to_owned(),
        dialect: engine.to_owned(),
        unit,
    }
}

#[test]
fn retained_explorer_source_keeps_actual_profiles_and_ignores_display_labels() {
    // naming.source.retained-explorer-consumers
    // docs/design/analysis/name-resolution-proofs/retained-explorer-consumers.md
    // Producer/model controls, independently of C/Jim binary observations.
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let environment = tcl_registry::model::resolve_known_environment(engine).unwrap();
        let registry = environment.default_context_registry();
        let config = LexerConfig::from_grammar(environment.grammar());
        let mut result = fixture("set retained VALUE", engine, registry.commands(), config);
        result.dialect = "display-label-has-no-semantic-owner".to_owned();
        let owner = retained_explorer_source(&result, &result.source).expect(engine);
        assert_eq!(owner.config, config, "{engine}");
        assert_eq!(
            owner.dialect.unwrap().cache_key(),
            environment.unit_profile().cache_key(),
            "{engine}"
        );
        assert_eq!(
            owner.registry.snapshot().semantic_key(),
            registry.commands().snapshot().semantic_key(),
            "{engine}"
        );
        assert_eq!(
            owner.input,
            result
                .unit
                .ir_module
                .source_metadata_input
                .as_ref()
                .unwrap(),
            "{engine}"
        );
    }
}

#[test]
fn retained_event_order_uses_the_custom_full_config_and_actual_command_store() {
    // naming.source.retained-explorer-consumers
    // docs/design/analysis/name-resolution-proofs/retained-explorer-consumers.md
    // Explicit source-parser axis, not a measured BIG-IP BOM acceptance claim.
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
    let context = environment.default_context_registry();
    let config = LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        ..LexerConfig::from_grammar(environment.grammar())
    };
    let source = "\u{feff}when HTTP_REQUEST {}";
    let mut result = fixture(source, "f5-irules", context.commands(), config);
    result.dialect = "tcl8.4".to_owned();
    let rows = serialise_event_order_for_result(&result, &LineIndex::new(source));
    let rows = rows.as_array().expect("current source inventory");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["event"], "HTTP_REQUEST");
    assert_eq!(rows[0]["range"]["startOffset"], "\u{feff}when ".len());

    let commands = context.commands();
    let mut commands =
        commands.project_for_profile(commands.profile().expect("environment registry profile"));
    let mut when = commands.get("when").unwrap().clone();
    when.traits.remove(tcl_registry::Traits::IS_EVENT_HANDLER);
    commands.insert(when);
    let result = fixture(
        "when HTTP_REQUEST {}",
        "f5-irules",
        &commands,
        LexerConfig::from_grammar(environment.grammar()),
    );
    let owner = retained_explorer_source(&result, &result.source).unwrap();
    assert_eq!(
        owner.registry.snapshot().semantic_key(),
        commands.snapshot().semantic_key()
    );
    assert_eq!(
        serialise_event_order_for_result(&result, &LineIndex::new(&result.source)),
        json!([])
    );
}

fn remaining_source_views(result: &ExplorerResult, source: &str) -> Vec<(&'static str, Value)> {
    let li = LineIndex::new(source);
    vec![
        ("asm", crate::asm::serialise_asm(result, &li, source)),
        (
            "wasm",
            serialise_wasm_with_options(
                result,
                tcl_codegen::codegen::wasm::WasmCompileOptions::hosted(),
            ),
        ),
        ("shimmer", serialise_shimmer(result, &li, source)),
        ("gvn", serialise_gvn(result, &li, source)),
        ("taintWarnings", serialise_taint(result, &li, source)),
        ("taintReplays", serialise_taint_replays(result, &li, source)),
        ("cfgPostSsa", serialise_cfg_post_ssa(result, &li, source)),
        ("liveness", serialise_liveness(result)),
        ("irulesFlow", serialise_irules_flow(result, &li, source)),
        ("bounds", serialise_bounds(result)),
    ]
}

fn assert_retained_views_withdraw(result: &ExplorerResult, label: &str) {
    let line_index = LineIndex::new(&result.source);
    assert!(
        retained_explorer_source(result, &result.source).is_none(),
        "{label}"
    );
    assert!(
        serialise_event_order_for_result(result, &line_index).is_null(),
        "{label}"
    );
    assert!(
        serialise_optimisations(result, &line_index, &result.source).is_null(),
        "{label}"
    );
    assert!(
        serialise_optimiser_passes(result, &line_index, &result.source).is_null(),
        "{label}"
    );
    assert!(serialise_stats(result).is_null(), "{label}");
    assert_eq!(
        serialise_annotations(result, &line_index, &result.source),
        (Value::Null, Value::Null),
        "{label}"
    );
    assert!(optimised_result(result).is_none(), "{label}");
    for (field, value) in remaining_source_views(result, &result.source) {
        assert!(value.is_null(), "{label}/{field}");
    }
}

#[test]
fn retained_explorer_views_refuse_missing_changed_and_foreign_source_owners() {
    type Control = (&'static str, fn(&mut ExplorerResult));
    // naming.source.retained-explorer-consumers
    // docs/design/analysis/name-resolution-proofs/retained-explorer-consumers.md
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
    let context = environment.default_context_registry();
    let config = LexerConfig::from_grammar(environment.grammar());
    let source = "when HTTP_REQUEST {}";
    let original = fixture(source, "f5-irules", context.commands(), config);
    assert!(retained_explorer_source(&original, source).is_some());
    assert_eq!(
        serialise_event_order_for_result(&original, &LineIndex::new(source))
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let controls: [Control; 13] = [
        ("missing retained interpretation", |r| {
            r.unit.ir_module.retained_source_bindings = None;
        }),
        ("missing registry snapshot", |r| {
            r.unit.ir_module.registry_snapshot = None;
        }),
        ("missing full input", |r| {
            r.unit.ir_module.source_metadata_input = None;
        }),
        ("changed Module image", |r| {
            r.unit.ir_module.source = tcl_lexer::SourceImage::document("when HTTP_RESPONSE {}");
        }),
        ("changed source channel", |r| {
            r.unit.ir_module.source = tcl_lexer::SourceImage::native(r.source.as_bytes());
        }),
        ("changed CU image", |r| {
            r.unit.source.push_str("; set changed VALUE");
        }),
        ("changed presentation image", |r| {
            r.source.push_str("; set changed VALUE");
        }),
        ("changed full BOM config", |r| {
            r.unit.ir_module.lexer_config.leading_bom = tcl_lexer::LeadingBom::Skip;
        }),
        ("changed source coordinates", |r| {
            r.unit.ir_module.lexer_config.base_line += 1;
        }),
        ("changed profile", |r| {
            r.unit.ir_module.dialect_profile = Some(tcl_dialect::DialectProfile::plain_tcl());
        }),
        ("changed entry", |r| {
            r.unit.ir_module.source_entry.unknown_entry =
                !r.unit.ir_module.source_entry.unknown_entry;
        }),
        ("same-store foreign availability", |r| {
            let input = r.unit.ir_module.source_metadata_input.as_ref().unwrap();
            let commands = r
                .unit
                .ir_module
                .registry_snapshot
                .as_ref()
                .unwrap()
                .shared_registry();
            let foreign = Arc::new(
                tcl_registry::model::ingress::static_context_for("tcl8.4")
                    .with_command_store(commands),
            );
            r.unit.ir_module.source_metadata_input = Some(ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                foreign,
                input.lexer_config(),
            ));
        }),
        ("foreign command snapshot", |r| {
            r.unit.ir_module.registry_snapshot = Some(
                tcl_registry::model::ingress::static_context_for("tcl8.4")
                    .commands()
                    .snapshot(),
            );
        }),
    ];
    for (label, mutate) in controls {
        let mut changed = ExplorerResult {
            source: original.source.clone(),
            dialect: original.dialect.clone(),
            unit: original.unit.clone(),
        };
        mutate(&mut changed);
        assert_retained_views_withdraw(&changed, label);
    }
    let index = LineIndex::new(source);
    assert!(serialise_optimisations(&original, &index, "foreign source").is_null());
    assert!(serialise_optimiser_passes(&original, &index, "foreign source").is_null());
    assert_eq!(
        serialise_annotations(&original, &index, "foreign source"),
        (Value::Null, Value::Null)
    );
    assert!(retained_explorer_source(&original, source).is_some());
}

#[test]
fn rewritten_explorer_image_retains_metadata_without_reusing_old_source_ownership() {
    // naming.source.retained-explorer-consumers
    // docs/design/analysis/name-resolution-proofs/retained-explorer-consumers.md
    let environment = tcl_registry::model::resolve_known_environment("tcl8.4").unwrap();
    let context = environment.default_context_registry();
    let config = LexerConfig::from_grammar(environment.grammar());
    let original = fixture(
        "set retained ORIGINAL",
        "tcl8.4",
        context.commands(),
        config,
    );
    let owner = retained_explorer_source(&original, &original.source).unwrap();
    let rewritten = rewritten_explorer_result("set retained REWRITTEN", "display-only", &owner);
    let current = retained_explorer_source(&rewritten, &rewritten.source).unwrap();
    assert_eq!(current.input, owner.input);
    assert_eq!(current.config, config);
    assert!(std::ptr::eq(
        current.input.borrowed_context_registry(),
        owner.input.borrowed_context_registry()
    ));
    assert_ne!(
        rewritten.unit.ir_module.source,
        original.unit.ir_module.source
    );
    assert!(retained_explorer_source(&rewritten, &original.source).is_none());
}

#[test]
fn remaining_actual_cu_views_retain_profiles_and_ignore_display_labels() {
    // naming.source.retained-explorer-consumers
    // docs/design/analysis/name-resolution-proofs/retained-explorer-consumers.md
    // Source/model agreement, independently of provider execution results.
    for engine in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "jim",
        "f5-irules",
    ] {
        let environment = tcl_registry::model::resolve_known_environment(engine).unwrap();
        let registry = environment.default_context_registry();
        let config = LexerConfig {
            leading_bom: tcl_lexer::LeadingBom::Skip,
            ..LexerConfig::from_grammar(environment.grammar())
        };
        let mut result = fixture(
            "proc keep {v} {set copy $v; puts $copy}",
            engine,
            registry.commands(),
            config,
        );
        let current = remaining_source_views(&result, &result.source);
        for (field, value) in &current {
            assert!(value.is_array(), "{engine}/{field}: {value}");
        }
        result.dialect = "unrecognised-presentation-label".to_owned();
        assert_eq!(
            remaining_source_views(&result, &result.source),
            current,
            "{engine}"
        );
    }
}

#[test]
fn remaining_source_range_consumers_refuse_foreign_source_arguments() {
    let result = crate::run_pipeline("puts current", "tcl8.6");
    let source = "puts foreign";
    let li = LineIndex::new(source);
    for (field, value) in [
        ("asm", crate::asm::serialise_asm(&result, &li, source)),
        ("shimmer", serialise_shimmer(&result, &li, source)),
        ("gvn", serialise_gvn(&result, &li, source)),
        ("taintWarnings", serialise_taint(&result, &li, source)),
        ("cfgPostSsa", serialise_cfg_post_ssa(&result, &li, source)),
        ("irulesFlow", serialise_irules_flow(&result, &li, source)),
    ] {
        assert!(value.is_null(), "{field}");
    }
}

#[test]
fn retained_irules_warning_view_and_statistics_share_the_actual_count() {
    let mut result = crate::run_pipeline("when HTTP_REQUEST {set u [HTTP::uri]}", "f5-irules");
    let li = LineIndex::new(&result.source);
    let warnings = serialise_irules_flow(&result, &li, &result.source);
    let count = warnings
        .as_array()
        .expect("available warning inventory")
        .len();
    assert!(count > 0, "fixture must produce a real warning");
    assert_eq!(serialise_stats(&result)["irulesFlowWarnings"], count);
    result.dialect = "tcl9.1".to_owned();
    assert_eq!(
        serialise_irules_flow(&result, &li, &result.source),
        warnings
    );
    assert_eq!(serialise_stats(&result)["irulesFlowWarnings"], count);
}

#[test]
fn bounds_withdraws_if_a_function_loses_its_actual_module_metadata() {
    let environment = tcl_registry::model::resolve_known_environment("tcl8.6").unwrap();
    let registry = environment.default_context_registry();
    let config = LexerConfig::from_grammar(environment.grammar());
    let mut result = fixture(
        "proc keep {v} {puts $v}",
        "tcl8.6",
        registry.commands(),
        config,
    );
    assert!(serialise_bounds(&result).is_array());
    // The original Module still owns its full source. Replacing just the
    // independently supplied function withdraws the required FU join.
    let foreign = crate::run_pipeline("proc foreign {} {puts foreign}", "tcl9.1");
    result.unit.top_level = foreign.unit.top_level;
    assert!(retained_explorer_source(&result, &result.source).is_some());
    assert!(serialise_bounds(&result).is_null());
}
