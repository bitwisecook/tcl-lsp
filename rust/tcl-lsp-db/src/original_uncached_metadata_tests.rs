// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The uncached document compiler path borrows its real analysis owner.

use super::*;
use tcl_compiler::analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput};

fn source_input(context: Arc<tcl_registry::model::ContextRegistry>) -> ResolvedAnalysisInput {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let mut config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    config.braced_var = tcl_dialect::BracedVarStyle::FirstClose;
    ResolvedAnalysisInput::new(profile, profile, context, config)
}

fn analyse(source: &str, input: &ResolvedAnalysisInput) -> AnalysisResult {
    Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, input.analyser_profile().name)
}

fn selected_call(unit: &CompilationUnit) -> bool {
    unit.top_level
        .semantic_facts
        .executable()
        .invocations()
        .any(|call| {
            matches!(
                call.resolution,
                tcl_compiler::executable_ir::InvocationResolution::Resolved(_)
            )
        })
}

#[test]
fn uncached_source_units_retain_actual_availability_and_source_currency() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional source metadata only; no runtime entry or erasure permission.
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_registry::model::resolve_environment("tcl8.6").unit_profile());
    let mut puts = registry.get("puts").unwrap().clone();
    puts.surface = registry.get("dict").unwrap().surface;
    registry.insert(puts);
    let current = Arc::new(baseline.with_command_store(Arc::new(registry)));
    let input = source_input(Arc::clone(&current));
    let source = "puts VALUE";
    let analysis = analyse(source, &input);
    let unit = compilation_unit_uncached_from_analysis(source, current.commands(), &analysis, None)
        .unwrap();
    assert_eq!(unit.ir_module.source_metadata_input.as_ref(), Some(&input));
    assert_eq!(unit.ir_module.lexer_config, input.lexer_config());
    assert!(selected_call(&unit));

    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(current.commands())),
    );
    assert!(Arc::ptr_eq(older.commands(), current.commands()));
    let unavailable = analyse(source, &source_input(older));
    let unavailable_unit =
        compilation_unit_uncached_from_analysis(source, current.commands(), &unavailable, None)
            .unwrap();
    assert!(!selected_call(&unavailable_unit));

    let mut missing = analysis.clone();
    missing.resolved_input = None;
    let mut stale = analysis.clone();
    stale.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
    let mut foreign = analysis.clone();
    foreign.resolved_input = Some(source_input(
        tcl_registry::model::resolve_environment("tcl9.1").default_context_registry(),
    ));
    let mut generation_missing = analysis.clone();
    generation_missing.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
        environment: "tcl8.6".to_owned(),
        overlay: 0x345,
    });
    for refused in [missing, stale, foreign, generation_missing] {
        assert!(
            compilation_unit_uncached_from_analysis(source, current.commands(), &refused, None,)
                .is_none()
        );
        let diagnostics = compiler_check_diagnostics_uncached_from_analysis(
            source,
            current.commands(),
            &refused,
            None,
            None,
        );
        assert!(diagnostics.checks.is_empty() && diagnostics.optimisations.is_empty());
    }
    assert!(
        compilation_unit_uncached_from_analysis(
            "puts CHANGED",
            current.commands(),
            &analysis,
            None,
        )
        .is_none()
    );
}

#[test]
fn uncached_compiler_advice_preserves_original_diagnostic_context() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context = tcl_registry::model::resolve_environment("tcl8.6").default_context_registry();
    let input = source_input(Arc::clone(&context));
    let source = "proc subject {} {if {1} {puts retained} else {puts omitted}}";
    let analysis = analyse(source, &input);
    let diagnostics = compiler_check_diagnostics_uncached_from_analysis(
        source,
        context.commands(),
        &analysis,
        None,
        None,
    );
    let branch = diagnostics
        .checks
        .iter()
        .find(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::O100)
        .expect("the selected Logical source branch supplies conditional advice");
    let original = branch
        .source_context
        .as_ref()
        .expect("original compiler source context");
    assert_eq!(original.image(), &tcl_lexer::SourceImage::document(source));
    assert_eq!(original.lexer_config(), input.lexer_config());
    assert_eq!(
        original.registry().semantic_key(),
        context.commands().snapshot().semantic_key()
    );
    let finding = tcl_lsp_core::diagnostic_policy::Finding::from(branch.clone());
    assert_eq!(finding.original_compiler_diagnostic(), Some(branch));
}
