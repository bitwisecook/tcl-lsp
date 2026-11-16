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

//! Source ingress and options: retained inputs, module snapshots, complete logical sources and refused or changed profiles.

use super::*;

#[test]
fn borrowed_lowering_retains_exact_ingress_before_finalisation() {
    let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
    let profile = registry.profile().expect("selected profile");
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let mut lowerer = Lowerer::with_config(registry, config);
    let module = lowerer.lower("set x 003");
    assert!(std::ptr::eq(
        module.resolved_profile().expect("profile"),
        profile
    ));
    assert!(module.registry_snapshot.is_some());
    assert_eq!(module.lexer_config, config);
    assert_eq!(
        module.word_values(),
        tcl_syntax::word_rules::WordValueRules::from_config(&config)
    );
}

#[test]
fn runtime_procedure_target_retains_its_actual_activation() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut lowerer = Lowerer::new(registry);
    let module = lowerer.lower_procedure_target("puts $x", "");
    let binding = module.top_level.statements[0]
        .tokens()
        .and_then(|tokens| tokens.source_binding.as_ref())
        .expect("positioned runtime procedure invocation");
    assert_eq!(
        binding.variable_frame,
        crate::var_resolve::VariableExecutionFrame::Procedure {
            namespace: "::".to_owned(),
            identity: "::top".to_owned(),
        }
    );
    assert_eq!(
        binding.variable_context.activation.as_deref(),
        Some("::top")
    );
}

#[test]
fn returned_module_snapshots_retain_their_actual_source_owner() {
    use std::sync::Arc;

    let generation = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let context = Arc::new(generation.with_command_store(Arc::clone(generation.commands())));
    let registry = generation.commands();
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    let source = "throw ERROR payload";
    for target in 0..3 {
        let mut lowerer = Lowerer::with_config(registry, config)
            .with_dialect(registry.profile())
            .with_context_registry(Arc::clone(&context));
        let module = match target {
            0 => lowerer.lower(source),
            1 => lowerer.lower_procedure_target(source, "::N"),
            _ => lowerer.lower_script_target(source, "::N"),
        };
        let owner = module
            .retained_source_bindings
            .as_ref()
            .expect("actual completed source");
        assert_eq!(module.source.bytes(), source.as_bytes());
        assert!(
            owner.positioned_for_module(module, registry).is_some(),
            "{target}"
        );
        let mut changed = module.clone();
        changed.source = tcl_lexer::SourceImage::document("throw ERROR changed");
        assert!(owner.positioned_for_module(&changed, registry).is_none());
        let mut changed = module.clone();
        changed.lexer_config.strict_quoting = !changed.lexer_config.strict_quoting;
        assert!(owner.positioned_for_module(&changed, registry).is_none());
    }
}

#[test]
fn lower_to_ir_public_api() {
    let r = reg();
    let m = lower_to_ir("set x 1\nproc foo {} {return 1}", &r);
    assert_eq!(m.top_level.statements.len(), 2);
    assert!(fixture_id_if_present(&m.procedures, "::foo").is_some());
}

// A proc body homes nested definitions to the proc's *defining*
// namespace, not the lexical namespace of the `proc` call.

#[test]
fn analyser_settings_preserve_factory_through_original_unit_rebuild() {
    let source = "set value VALUE";
    let mut analyser = crate::analyser::Analyser::new();
    let settings = analyser.source_compilation_settings();
    let analysis = analyser.analyse(source, "tcl8.6");
    let input = analysis.resolved_input.as_ref().unwrap();
    let registry = input.borrowed_context_registry().commands();
    let options = crate::compilation_unit::UnitBuildOptions {
        registry,
        defer_top_level: false,
        config: input.lexer_config(),
        dialect: Some(input.unit_profile()),
        external_call_sites: None,
        declared_commands: analyser.declared_commands.as_ref(),
    };
    let fresh = |unit: &crate::compilation_unit::CompilationUnit| {
        unit.ir_module
            .retained_source_bindings
            .as_ref()
            .unwrap()
            .fresh_native_source_analysis(&unit.ir_module, registry)
            .is_some()
    };
    let unit =
        crate::compilation_unit::CompilationUnit::build_with_analysis_input_and_source_settings(
            source, options, None, input, &settings,
        );
    assert!(fresh(&unit));
    assert_eq!(
        unit.ir_module
            .source_entry
            .analyser_source_settings
            .as_ref(),
        Some(&settings)
    );
    let rebuilt = analysis
        .original_optimiser_source_unit(source, registry, None)
        .unwrap();
    assert!(fresh(&rebuilt));
    assert_eq!(
        unit.original_optimiser_source_origin(),
        rebuilt.original_optimiser_source_origin()
    );
    let entry = unit.ir_module.source_entry.clone();
    let explicit =
        crate::compilation_unit::CompilationUnit::build_with_analysis_input_and_source_settings(
            source,
            options,
            Some(&entry),
            input,
            &settings,
        );
    assert_eq!(explicit.ir_module.source_entry, entry);
    assert!(!fresh(&explicit));
}

#[test]
fn explicit_equal_options_revoke_supplied_source_issuance() {
    let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = context.commands();
    let profile = registry.profile().unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::new(context.with_command_store(std::sync::Arc::clone(context.commands()))),
        config,
    );
    let mut lowerer = Lowerer::with_config(registry, config)
        .with_dialect(Some(profile))
        .with_resolved_analysis_input(input);
    let source = "set value VALUE";
    let original = lowerer.lower(source);
    assert!(
        original
            .retained_source_bindings
            .as_ref()
            .unwrap()
            .fresh_native_source_analysis(original, registry)
            .is_some()
    );
    let entry = original.source_entry.clone();
    lowerer.set_source_analysis_options(entry.options());
    assert!(lowerer.supplied_source.is_none());
    let explicit = lowerer.lower(source);
    assert_eq!(explicit.source_entry, entry);
    assert!(
        explicit
            .retained_source_bindings
            .as_ref()
            .unwrap()
            .fresh_native_source_analysis(explicit, registry)
            .is_none()
    );
}

#[test]
fn configured_source_ingress_retains_one_input_for_entry_and_module() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let profile = registry.profile().unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let source = "proc f {p} {return $p}";
    let module = lower_to_ir_with_dialect(source, registry, config, Some(profile));
    let input = module
        .source_metadata_input
        .as_ref()
        .expect("actual configured source input");
    assert_eq!(
        module.source_entry.metadata_context.source_analysis_input(),
        Some(input)
    );
    assert!(module.source_entry.logical_source_input.is_none());
    assert_eq!(
        module
            .source_entry
            .options()
            .execution_name_policy()
            .unwrap()
            .native_recipe()
            .unwrap()
            .authority(),
        tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
    );
    assert!(module.source_entry.native_entry.is_none());
    assert!(
        module
            .retained_source_bindings
            .as_ref()
            .unwrap()
            .matches_module(&module, registry)
    );
    let procedures = module.procedures.iter().collect::<Vec<_>>();
    assert_eq!(procedures.len(), 1);
    assert!(module.procedure_ids.owns(*procedures[0].0));
    assert_eq!(procedures[0].1.qualified_name, "::f");
}

#[test]
fn complete_logical_source_keeps_original_structure_and_native_absence() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let source = "proc subject {} {return VALUE}; set result VALUE";
    let mut current = Lowerer::with_config(context.commands(), input.lexer_config())
        .with_resolved_analysis_input(input.clone());
    current.lower(source);
    let module = current.finish_module(source);
    assert!(fixture_id_if_present(&module.procedures, "::subject").is_some());
    // Upstream keeps non-numeric bare literals as AssignValue so escape
    // spelling survives. This assertion covers IR/source ownership only.
    let statement = module.top_level.statements.iter().find(|statement| {
            matches!(statement, Statement::AssignValue { name, value, value_needs_backsubst: false, .. }
                if name == "result" && value == "VALUE")
        }).expect("unchanged literal assignment retains the upstream representation");
    let tokens = module
        .top_level
        .retained_source_tokens_for_statement(statement)
        .unwrap();
    let literal = crate::type_infer::original_literal_argument_contents(
        context.commands(),
        &module,
        tokens,
        1,
    )
    .expect("assignment retains its genuine original source");
    assert_eq!(literal.value(), "VALUE");
    assert_eq!(
        module.source_entry.logical_source_input.as_ref(),
        Some(&input)
    );
    assert!(module.source_entry.native_entry.is_none());
    assert!(module.source_entry.execution_name_policy.is_none());

    let mut stale = Lowerer::with_config(context.commands(), input.lexer_config());
    stale.config.strict_quoting = !stale.config.strict_quoting;
    stale = stale.with_resolved_analysis_input(input.clone());
    assert!(fixture_id_if_present(&stale.lower(source).procedures, "::subject").is_none());
    let foreign = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        input.lexer_config(),
    );
    let mut foreign = Lowerer::with_config(context.commands(), foreign.lexer_config())
        .with_resolved_analysis_input(foreign);
    assert!(fixture_id_if_present(&foreign.lower(source).procedures, "::subject").is_none());

    let native_profile = context.commands().profile().unwrap();
    let native = crate::analyser::ResolvedAnalysisInput::new(
        native_profile,
        native_profile,
        std::sync::Arc::clone(&context),
        tcl_lexer::LexerConfig::for_file_grammar(native_profile.grammar),
    );
    // No supplied entry: a fresh tclsh defines `::subject` (user decision
    // 2026-10-19T14:20Z). Explicit unknown history still withholds it.
    let mut fresh = Lowerer::with_config(context.commands(), native.lexer_config())
        .with_resolved_analysis_input(native.clone());
    let module = fresh.lower(source);
    assert!(fixture_id_if_present(&module.procedures, "::subject").is_some());
    assert!(module.source_entry.logical_source_input.is_none());
    assert!(module.source_entry.native_entry.is_none());
    assert!(!module.source_entry.unknown_entry);
    let mut absent = Lowerer::with_config(context.commands(), native.lexer_config())
        .with_resolved_analysis_input(native)
        .with_explicit_unknown_entry();
    let module = absent.lower(source);
    assert!(fixture_id_if_present(&module.procedures, "::subject").is_none());
    assert!(module.source_entry.logical_source_input.is_none());
    assert!(module.source_entry.native_entry.is_none());
    assert!(module.source_entry.unknown_entry);
}

#[test]
fn logical_original_declaration_layout_keeps_unicode_and_qualified_alias_heads() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // These exact UTF-8 source spellings exercise conditional Logical
    // layout selection, independently of any Native slot or execution.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    for source in [
        "rename proc ::α; ::α subject {} {return VALUE}",
        "interp alias {} ::α {} proc; ::α subject {} {return VALUE}",
        "rename proc ::α; interp alias {} ::β {} ::α; ::β subject {} {return VALUE}",
        r"rename proc ::α; \u003a\u003aα subject {} {return VALUE}",
    ] {
        let mut lowerer = Lowerer::with_config(context.commands(), input.lexer_config())
            .with_resolved_analysis_input(input.clone());
        let module = lowerer.lower(source);
        assert!(
            fixture_id_if_present(&module.procedures, "::subject").is_some(),
            "{source}"
        );
        assert!(module.source_entry.native_entry.is_none());
        assert!(module.source_entry.execution_name_policy.is_none());
        assert_eq!(
            module.source_entry.logical_source_input.as_ref(),
            Some(&input)
        );
    }
    for source in [
        "interp alias {} ::α {} proc subject; ::α {} {return VALUE}",
        "interp alias {} ::α {} proc subject {}; ::α {return VALUE}",
    ] {
        let mut lowerer = Lowerer::with_config(context.commands(), input.lexer_config())
            .with_resolved_analysis_input(input.clone());
        let module = lowerer.lower(source);
        let procedure = fixture_id_if_present(&module.procedures, "::subject")
            .and_then(|id| module.procedures.get(&id))
            .expect("captured source declaration");
        assert_eq!(procedure.params, [] as [std::string::String; 0]);
        assert_eq!(procedure.params_raw, "");
        assert!(module.source_entry.native_entry.is_none());
    }
    let source = "rename proc ::α; ::α ::α args {return PAYLOAD}; ::α subject {} {}";
    let mut replaced = Lowerer::with_config(context.commands(), input.lexer_config())
        .with_resolved_analysis_input(input);
    assert!(fixture_id_if_present(&replaced.lower(source).procedures, "::subject").is_none());
}

#[test]
fn complete_source_input_declines_changed_unit_profile_without_authoring_fallback() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    );
    let selected = Lowerer::with_config(context.commands(), input.lexer_config())
        .with_resolved_analysis_input(input.clone());
    assert!(selected.invocation_metadata_context().is_some());
    assert_eq!(selected.logical_source_input.as_ref(), Some(&input));
    let mut wrong = Lowerer::with_config(context.commands(), input.lexer_config())
        .with_dialect(Some(tcl_dialect::DialectProfile::find("tcl8.6").unwrap()))
        .with_resolved_analysis_input(input.clone());
    assert!(wrong.invocation_metadata_context().is_none());
    assert!(wrong.unknown_entry);
    let module = wrong.lower("puts VALUE");
    assert_eq!(module.source_metadata_input.as_ref(), Some(&input));
    assert!(module.source_entry.unknown_entry);
    assert!(module.source_entry.logical_source_input.is_none());
}

#[test]
fn lowered_generic_metadata_keeps_supplied_context_and_refuses_missing_driver_input() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context = tcl_registry::model::ingress::context_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let registry = context.commands();
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    let mut missing = Lowerer::with_config(registry, config).with_dialect(registry.profile());
    missing.set_source_analysis_options(SourceAnalysisOptions::default());
    missing = missing.with_dialect(registry.profile());
    assert!(missing.invocation_metadata_context().is_none());
    assert!(
        missing
            .lower("puts retained")
            .source_metadata_input
            .is_none()
    );
    let mut supplied = Lowerer::with_config(registry, config)
        .with_context_registry(std::sync::Arc::clone(&context))
        .with_dialect(registry.profile());
    supplied.set_source_analysis_options(SourceAnalysisOptions::default());
    assert!(
        supplied
            .invocation_metadata_context()
            .unwrap()
            .matches_registry(registry)
    );
    let module = supplied.lower("puts retained");
    assert!(std::sync::Arc::ptr_eq(
        &module
            .source_metadata_input
            .as_ref()
            .unwrap()
            .context_registry(),
        &context
    ));
    let foreign = tcl_registry::model::ingress::context_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
    );
    let mut wrong = Lowerer::with_config(registry, config)
        .with_context_registry(std::sync::Arc::clone(&foreign));
    wrong.set_source_analysis_options(SourceAnalysisOptions::default());
    assert!(
        !wrong
            .invocation_metadata_context()
            .unwrap()
            .matches_registry(registry)
    );
    assert!(!wrong.safe_on_uninit("incr", &["value".to_owned()]));
    let older = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(registry)),
    );
    let actual = Lowerer::with_config(registry, config)
        .with_dialect(registry.profile())
        .with_context_registry(older);
    assert!(!actual.safe_on_uninit("incr", &["value".to_owned()]));
    let current = Lowerer::with_config(registry, config)
        .with_dialect(registry.profile())
        .with_context_registry(std::sync::Arc::clone(&context));
    assert!(current.safe_on_uninit("incr", &["value".to_owned()]));
}
