// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional trace and definition-body metadata keeps its supplied owner.

use super::*;

fn supplied_input(
    context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
) -> crate::analyser::ResolvedAnalysisInput {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
    )
}

#[test]
fn trace_hazard_metadata_keeps_actual_availability_and_missing_owner_uncertainty() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // May observer inventory only; no installed physical trace is asserted.
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut commands = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut trace = commands.get("trace").unwrap().clone();
    trace.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    commands.insert(trace);
    let context = std::sync::Arc::new(baseline.with_command_store(std::sync::Arc::new(commands)));
    let source = "trace add variable value write callback";
    let lowerer = Lowerer::with_config(
        context.commands(),
        tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
    )
    .with_context_registry(std::sync::Arc::clone(&context));
    let original = lower_with(lowerer, source);
    assert!(original.traced_variables.contains("value"));
    let original_tokens = original.top_level.statements[0].tokens().unwrap().clone();
    let older = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(context.commands())),
    );
    for input in [
        Some(supplied_input(older)),
        Some(supplied_input(
            tcl_registry::model::ingress::context_for_profile(
                tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
            ),
        )),
        None,
    ] {
        let mut module = original.clone();
        module.source_metadata_input = input;
        module.traced_variables.clear();
        module.traced_commands.clear();
        module.has_dynamic_variable_trace = false;
        module.has_dynamic_trace = false;
        populate_trace_facts(&mut module, context.commands(), false);
        assert_eq!(
            module.top_level.statements[0].tokens().unwrap(),
            &original_tokens
        );
        assert!(module.traced_variables.is_empty());
        assert!(module.has_dynamic_variable_trace);
        assert!(module.has_dynamic_trace);
    }
}

#[test]
fn passive_class_body_metadata_uses_original_selection_and_supplied_availability() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Original source method units only; no class allocation or entered method.
    let context = tcl_registry::model::ingress::context_for_profile(
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
    );
    let source = "oo::class create C {method run {} {return VALUE}}";
    let lower = |availability: std::sync::Arc<tcl_registry::model::ContextRegistry>| {
        lower_with(
            Lowerer::with_config(
                context.commands(),
                tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
            )
            .with_context_registry(availability),
            source,
        )
    };
    let current = lower(std::sync::Arc::clone(&context));
    assert!(current.methods.contains_key("::C::run"));
    let older = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(context.commands())),
    );
    let foreign = tcl_registry::model::ingress::context_for_profile(
        tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
    );
    for unavailable in [older, foreign] {
        let module = lower(unavailable);
        assert!(module.methods.is_empty());
        assert!(module.oo_evidence.unretained_executable_roots);
    }
    let shadowed =
        "proc oo::class {args} {return}; oo::class create C {method run {} {return VALUE}}";
    let module = lower_with(
        Lowerer::with_config(
            context.commands(),
            tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
        )
        .with_context_registry(std::sync::Arc::clone(&context)),
        shadowed,
    );
    assert!(
        module.methods.is_empty(),
        "a source replacement cannot donate stock class grammar"
    );
    let mut missing = Lowerer::with_config(
        context.commands(),
        tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
    );
    missing.set_source_analysis_options(SourceAnalysisOptions::default());
    let module = lower_with(missing, source);
    assert!(module.methods.is_empty());
    assert!(module.oo_evidence.unretained_executable_roots);
}

#[test]
fn passive_member_layout_uses_actual_release_and_original_optional_word_shape() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Original source vocabulary/body ordinals only, no method installation.
    let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
    let current = tcl_registry::model::ingress::context_for_profile(profile);
    let older = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.6")
            .with_command_store(std::sync::Arc::clone(current.commands())),
    );
    let source = "oo::class create C {classmethod build {} {return NEW}; method run -private {arg} {return $arg}; method plain {} {return PLAIN}}";
    let config = tcl_lexer::LexerConfig::for_dialect("tcl9.1");
    let lower = |context: std::sync::Arc<tcl_registry::model::ContextRegistry>| {
        lower_with(
            Lowerer::with_config(current.commands(), config)
                .with_dialect(Some(profile))
                .with_context_registry(context),
            source,
        )
    };
    let current_module = lower(std::sync::Arc::clone(&current));
    assert!(current_module.methods.contains_key("::C::build"));
    assert_eq!(current_module.methods["::C::run"].params, ["arg"]);
    assert!(current_module.methods.contains_key("::C::plain"));
    let old_module = lower(older);
    assert!(!old_module.methods.contains_key("::C::build"));
    assert!(!old_module.methods.contains_key("::C::run"));
    assert!(old_module.methods.contains_key("::C::plain"));
    assert!(old_module.oo_evidence.unretained_executable_roots);
    let dynamic = "oo::class create C {method run $option {arg} {return $arg}; method plain {} {return PLAIN}}";
    let module = lower_with(
        Lowerer::with_config(current.commands(), config)
            .with_dialect(Some(profile))
            .with_context_registry(std::sync::Arc::clone(&current)),
        dynamic,
    );
    assert!(!module.methods.contains_key("::C::run"));
    assert!(module.methods.contains_key("::C::plain"));
    assert!(module.oo_evidence.unretained_executable_roots);
}
