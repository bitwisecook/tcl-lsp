// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Readonly source targets retain their actual purpose and original owner.

use crate::analyser::{Analyser, ResolvedAnalysisInput};
use crate::command_binding::SourceAnalysisOptions;
use crate::registry_invocation::source_structure::source_class_declaration_at;

fn input(logical: bool) -> ResolvedAnalysisInput {
    let environment = tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
        .expect("known source fixture environment");
    let profile = if logical {
        tcl_dialect::DialectProfile::plain_tcl()
    } else {
        environment.analyser_profile()
    };
    ResolvedAnalysisInput::new(
        profile,
        profile,
        environment.default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    )
}

fn statement(
    source: &str,
    input: &ResolvedAnalysisInput,
    selected: &str,
    native: Option<&tcl_runtime_api::NativeCompilationEntry>,
) -> crate::ir::Statement {
    let mut lowerer = crate::lowering::Lowerer::with_config(
        input.borrowed_context_registry().commands(),
        input.lexer_config(),
    )
    .with_resolved_analysis_input(input.clone());
    if let Some(native) = native {
        lowerer.set_source_analysis_options(SourceAnalysisOptions {
            native_entry: Some(native),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                input.analyser_profile(),
            )),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        });
    }
    lowerer.lower(source);
    let module = lowerer.finish_module(source);
    assert_eq!(module.source_metadata_input.as_ref(), Some(input));
    let offset = u32::try_from(source.rfind(selected).unwrap()).unwrap();
    let mut statements = Vec::new();
    crate::ir::for_each_statement(&module.top_level, &mut |statement| {
        if statement.span().start() == offset {
            statements.push(statement.clone());
        }
    });
    assert_eq!(statements.len(), 1);
    statements.pop().unwrap()
}

#[test]
fn logical_callee_projection_retains_declaration_but_not_physical_allocation() {
    // naming.compiler.original-positioned-callee-body-availability
    // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
    let input = input(true);
    let registry = input.borrowed_context_registry().commands();
    for (source, selected) in [
        ("proc available {} {}; available VALUE", "available VALUE"),
        (
            "proc available {} {}; interp alias {} ensure {} available; ensure VALUE",
            "ensure VALUE",
        ),
    ] {
        let statement = statement(source, &input, selected, None);
        let tokens = statement.tokens().unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        let raw = binding.targets.clone();
        let targets = binding
            .original_logical_declaration_call_targets(tokens, &input, registry)
            .expect("closed original Logical procedure");
        assert_eq!(targets.len(), 1);
        assert!(raw[0].implementation_allocation.is_some());
        let mut expected = raw.clone();
        expected[0].implementation_allocation = None;
        assert_eq!(targets, expected);
        assert_eq!(binding.targets, raw);
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.proved_handler_target().is_none());
        assert!(binding.native_compiler_admission.is_none());
        for axis in 0..3 {
            let mut changed = binding.clone();
            match axis {
                0 => changed.unknown = true,
                1 => changed.may_be_absent = true,
                2 => changed.targets.clear(),
                _ => unreachable!(),
            }
            assert!(
                changed
                    .original_logical_declaration_call_targets(tokens, &input, registry)
                    .is_none()
            );
        }
    }
}

#[test]
fn native_callee_advice_keeps_its_independently_retained_allocation() {
    // naming.compiler.original-positioned-callee-body-availability
    // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
    let input = input(false);
    let captured = crate::environment_ingress::captured_native_entry(input.analyser_profile());
    let source = "proc available {} {}; available VALUE";
    let statement = statement(source, &input, "available VALUE", Some(captured.as_entry()));
    let tokens = statement.tokens().unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    let advice = binding
        .original_declared_layout_advice(tokens, true)
        .expect("actual Native source declaration layout");
    assert_eq!(advice.targets(), binding.targets.as_slice());
    assert!(advice.targets()[0].implementation_allocation.is_some());
    assert!(
        binding
            .original_logical_declaration_call_targets(
                tokens,
                &input,
                input.borrowed_context_registry().commands(),
            )
            .is_none()
    );
}

#[test]
fn source_class_projection_separates_native_name_from_logical_metadata() {
    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    let source = "oo::class create C {method ping {} {}}";
    for logical in [true, false] {
        let input = input(logical);
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, input.analyser_profile().name);
        let declaration = source_class_declaration_at(source, &analysis, 0)
            .expect("original selected class factory");
        let logical_class = declaration.logical_source_class(&analysis);
        let native_class = declaration.source_class(&analysis);
        assert_eq!(logical_class.is_some(), logical);
        assert_eq!(native_class.is_some(), !logical);
        if logical {
            assert!(declaration.name_input().native_input().is_none());
            assert!(declaration.name_input().logical_input().is_some());
            let class = logical_class.unwrap();
            assert!(class.source_name.is_some());
            assert!(analysis.original_class_declarations().any(|record| {
                declaration.matches_source_declaration(record)
                    && Some(record.name()) == class.source_name.as_ref()
            }));
        } else {
            assert!(declaration.name_input().native_input().is_some());
            assert!(declaration.name_input().logical_input().is_none());
        }
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(declaration.source_class(&missing).is_none());
        assert!(declaration.logical_source_class(&missing).is_none());
    }
}

#[test]
fn logical_class_metadata_requires_original_correspondence_and_unique_inventory() {
    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    let source = "oo::class create C {method ping {} {}}; oo::define C {method pong {} {}}";
    let input = input(true);
    let analysis = Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, input.analyser_profile().name);
    let declaration = source_class_declaration_at(source, &analysis, 0).unwrap();
    let class = declaration.logical_source_class(&analysis).unwrap();
    assert!(class.methods.contains_key("ping"));
    assert!(class.methods.contains_key("pong"));
    assert!(declaration.source_class(&analysis).is_none());
    let mut missing = analysis.clone();
    missing.original_class_metadata.clear();
    assert!(declaration.logical_source_class(&missing).is_none());
    let mut duplicate = analysis.clone();
    duplicate
        .all_classes
        .insert("presentation_only".into(), class.clone());
    assert!(declaration.logical_source_class(&duplicate).is_none());
    let mut foreign = analysis.clone();
    let other = Analyser::new()
        .with_resolved_input(input.clone())
        .analyse("oo::class create Other {}", input.analyser_profile().name);
    foreign.original_class_metadata = other.original_class_metadata;
    assert!(declaration.logical_source_class(&foreign).is_none());
    let mut changed = analysis.clone();
    let mut config = input.lexer_config();
    config.strict_quoting = !config.strict_quoting;
    changed.resolved_input = Some(ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        config,
    ));
    assert!(declaration.logical_source_class(&changed).is_none());
}
