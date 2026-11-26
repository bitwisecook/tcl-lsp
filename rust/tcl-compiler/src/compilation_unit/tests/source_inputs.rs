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

//! The source input a unit is built under: standalone and supplied inputs, metadata currency, grammar and the selected entry.

use super::*;

fn assert_standalone_currency_withdrawals(
    source: &str,
    options: UnitBuildOptions<'_>,
    input: &crate::analyser::ResolvedAnalysisInput,
) {
    let mut changed_config = options.config;
    changed_config.leading_bom = tcl_lexer::LeadingBom::Skip;
    assert!(
        CompilationUnit::build_standalone_with_analysis_input(
            source,
            UnitBuildOptions {
                config: changed_config,
                ..options
            },
            input
        )
        .is_none()
    );
    assert!(
        CompilationUnit::build_standalone_with_analysis_input(
            source,
            UnitBuildOptions {
                dialect: tcl_dialect::DialectProfile::find("tcl8.6"),
                ..options
            },
            input
        )
        .is_none()
    );
    let foreign_store = CommandRegistry::build_default();
    assert!(
        CompilationUnit::build_standalone_with_analysis_input(
            source,
            UnitBuildOptions {
                registry: &foreign_store,
                ..options
            },
            input
        )
        .is_none()
    );
}

#[test]
fn standalone_source_input_keeps_authoring_purpose_and_exact_metadata_currency() {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let mut store = CommandRegistry::build_default();
    store.insert(tcl_registry::CommandSpec {
        name: "standaloneRetainedStoreMarker",
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let store = Arc::new(store);
    let context = tcl_registry::model::ingress::resolve_environment("tcl9.0")
        .default_context_registry()
        .with_command_store(Arc::clone(&store));
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::new(context),
        tcl_lexer::LexerConfig {
            leading_bom: tcl_lexer::LeadingBom::Content,
            ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
        },
    );
    let source = "set value [expr {1 + 2}]";
    let options = UnitBuildOptions {
        registry: &store,
        defer_top_level: false,
        config: input.lexer_config(),
        dialect: Some(profile),
        external_call_sites: None,
        declared_commands: None,
    };
    let standalone = CompilationUnit::build_standalone_with_analysis_input(source, options, &input)
        .expect("explicit Standalone source request with actual complete input");
    assert_eq!(
        standalone.ir_module.source_metadata_input.as_ref(),
        Some(&input)
    );
    assert_eq!(
        standalone.ir_module.registry_snapshot.as_ref(),
        Some(&store.snapshot())
    );
    assert_eq!(standalone.ir_module.lexer_config, input.lexer_config());
    assert!(standalone.ir_module.source_entry.native_entry.is_none());
    assert!(!standalone.ir_module.source_entry.unknown_entry);
    assert!(
        standalone
            .ir_module
            .source_entry
            .invocation_dialect
            .is_some()
    );
    assert!(
        standalone
            .ir_module
            .source_entry
            .logical_source_input
            .is_none()
    );
    // The same metadata does not promote an ordinary supplied-document
    // request to Standalone; with no entry it starts a fresh interpreter.
    let supplied = CompilationUnit::build_with_analysis_input(source, options, None, &input);
    assert_eq!(
        supplied.ir_module.source_metadata_input.as_ref(),
        Some(&input)
    );
    assert!(supplied.ir_module.source_entry.native_entry.is_none());
    assert!(!supplied.ir_module.source_entry.unknown_entry);
    assert!(
        !supplied
            .ir_module
            .source_entry
            .metadata_context
            .is_standalone()
    );
    assert_standalone_currency_withdrawals(source, options, &input);
}

#[test]
fn function_consumers_keep_the_actual_generation_and_grammar() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let registry = Arc::new(CommandRegistry::build_default());
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let actual = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(&registry)),
    );
    let config = tcl_lexer::LexerConfig {
        braced_var: tcl_dialect::BracedVarStyle::FirstClose,
        ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
    };
    let unit = CompilationUnit::build_with_context_registry(
        "proc p {x} {set y $x; return $y}; namespace eval n {set z 1}",
        UnitBuildOptions {
            registry: &registry,
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        Arc::clone(&actual),
    );
    let input = unit.ir_module.source_metadata_input.as_ref().unwrap();
    assert!(Arc::ptr_eq(&input.context_registry(), &actual));
    assert!(!unit.procedures.is_empty());
    let body_id = fixture_id(&unit.ir_module.procedures, "::p");
    assert!(std::ptr::eq(
        unit.function_for_body(Some(body_id)).unwrap(),
        std::ptr::from_ref(&unit.procedures[&body_id])
    ));
    assert!(std::ptr::eq(
        unit.function_for_body(None).unwrap(),
        std::ptr::from_ref(&unit.top_level)
    ));
    let mut wrong_stamp = unit.clone();
    wrong_stamp.procedures.get_mut(&body_id).unwrap().body_id = None;
    assert!(wrong_stamp.function_for_body(Some(body_id)).is_none());
    wrong_stamp.top_level.body_id = Some(body_id);
    assert!(wrong_stamp.function_for_body(None).is_none());
    let foreign_unit = CompilationUnit::build_with_context_registry(
        &unit.source,
        UnitBuildOptions {
            registry: &registry,
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        Arc::clone(&actual),
    );
    let foreign_id = fixture_id(&foreign_unit.ir_module.procedures, "::p");
    let mut foreign_owner = unit.clone();
    foreign_owner.ir_module.procedure_ids = foreign_unit.ir_module.procedure_ids.clone();
    for (&id, function) in unit.methods.iter().chain(unit.body_units.iter()) {
        assert!(std::ptr::eq(
            unit.function_for_body(Some(id))
                .expect("actual retained body unit"),
            function
        ));
        assert!(foreign_owner.function_for_body(Some(id)).is_none());
    }
    assert!(foreign_owner.function_for_body(Some(body_id)).is_none());
    assert!(unit.function_for_body(Some(foreign_id)).is_none());
    for function in std::iter::once(&unit.top_level)
        .chain(unit.procedures.values())
        .chain(unit.methods.values())
        .chain(unit.body_units.values())
    {
        assert_eq!(function.source_metadata_input(), Some(input));
        assert_eq!(function.source_lexer_config(), config);
        let selected = function.invocation_metadata_context(&registry).unwrap();
        assert!(std::ptr::eq(selected.context(), actual.context()));
        let foreign = CommandRegistry::build_default().project_for_profile(
            tcl_registry::model::ingress::resolve_environment("tcl9.1").unit_profile(),
        );
        assert!(function.invocation_metadata_context(&foreign).is_none());
        let mut missing = function.clone();
        missing.source_metadata_input = None;
        assert!(missing.invocation_metadata_context(&registry).is_none());
    }
}

#[test]
fn supplied_source_input_retains_axes_and_independent_logical_entry() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Source declarations and aliases are conditional authored facts, not Native entry.
    let current =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::clone(&current),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let options = UnitBuildOptions {
        registry: current.commands(),
        defer_top_level: false,
        config: input.lexer_config(),
        dialect: Some(input.unit_profile()),
        external_call_sites: None,
        declared_commands: None,
    };
    let unit = CompilationUnit::build_with_analysis_input(
        "proc aliaser {} {global ::n::shared; set shared 1}",
        options,
        None,
        &input,
    );
    assert_eq!(unit.ir_module.source_metadata_input.as_ref(), Some(&input));
    assert_eq!(
        unit.ir_module.source_entry.logical_source_input.as_ref(),
        Some(&input)
    );
    assert!(unit.ir_module.source_entry.native_entry.is_none());
    assert!(fixture_id_if_present(&unit.procedures, "::aliaser").is_some());
    let explicit = crate::command_binding::SourceAnalysisEntry::default();
    let unknown = CompilationUnit::build_with_analysis_input(
        "proc aliaser {} {global ::n::shared; set shared 1}",
        options,
        Some(&explicit),
        &input,
    );
    assert!(
        unknown
            .ir_module
            .source_entry
            .logical_source_input
            .is_none()
    );
    assert!(unknown.ir_module.source_entry.native_entry.is_none());
    assert_eq!(
        unknown.ir_module.source_metadata_input.as_ref(),
        Some(&input)
    );
    let native_profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let native = crate::analyser::ResolvedAnalysisInput::new(
        native_profile,
        native_profile,
        Arc::clone(&current),
        input.lexer_config(),
    );
    let native_unit = CompilationUnit::build_with_analysis_input(
        "puts VALUE",
        UnitBuildOptions {
            dialect: Some(native_profile),
            ..options
        },
        None,
        &native,
    );
    assert!(
        native_unit
            .ir_module
            .source_entry
            .logical_source_input
            .is_none()
    );
    assert!(native_unit.ir_module.source_entry.native_entry.is_none());
    // No supplied entry: a fresh interpreter, not unknown history.
    assert!(!native_unit.ir_module.source_entry.unknown_entry);
    assert_eq!(
        native_unit.ir_module.source_metadata_input.as_ref(),
        Some(&native)
    );
}

#[test]
fn default_unit_source_profile_keeps_explicit_standalone_authoring_purpose() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = context.commands().profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let options = UnitBuildOptions {
        registry: context.commands(),
        config,
        dialect: Some(profile),
        defer_top_level: false,
        external_call_sites: None,
        declared_commands: None,
    };
    let source = "proc local {value} {set result $value; return $result}";
    // A dialect-bearing default build is a whole document in a fresh
    // interpreter of that dialect (user decision 2026-10-19T14:20Z):
    // `tclsh8.6` defines `::local`, so the unit publishes it.
    let fresh = CompilationUnit::build_with_options(source, options);
    assert!(fresh.ir_module.source_metadata_input.is_some());
    assert!(fresh.function("::local").is_some());
    // Explicit Standalone authoring remains the dialect-blind build.
    let options = UnitBuildOptions {
        dialect: None,
        ..options
    };
    let unit = CompilationUnit::build_with_options(source, options);
    let module = &unit.ir_module;
    assert!(module.source_entry.metadata_context.is_standalone());
    assert!(module.source_metadata_input.is_none());
    assert!(module.source_entry.native_entry.is_none());
    assert_eq!(module.original_authored_declaration_body_units.len(), 1);
    assert!(module.original_native_declaration_body_units.is_empty());
    let (id, card) = module
        .original_authored_declaration_body_units
        .iter()
        .next()
        .unwrap();
    let procedure = module.body_units.get(id).unwrap();
    assert!(card.matches_procedure(module, procedure));
    assert!(procedure.publication.is_none());
    assert!(procedure.body.executed_source.is_none());
    assert!(procedure.body.native_compilation_admission.is_none());
    let header = module
        .top_level
        .retained_source_tokens_for_statement(&module.top_level.statements[0])
        .unwrap();
    assert!(
        crate::registry_invocation::admitted_native_compiler_invocation(
            context.commands(),
            None,
            header
        )
        .is_none()
    );
}

#[test]
fn default_source_purpose_does_not_replace_supplied_input_or_unknown_driver_entry() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = context.commands().profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let options = UnitBuildOptions {
        registry: context.commands(),
        config,
        dialect: Some(profile),
        defer_top_level: false,
        external_call_sites: None,
        declared_commands: None,
    };
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let source = "proc original {value} {return $value}";
    let supplied = CompilationUnit::build_with_analysis_input(source, options, None, &input);
    assert_eq!(
        supplied.ir_module.source_metadata_input.as_ref(),
        Some(&input)
    );
    assert!(
        !supplied
            .ir_module
            .source_entry
            .metadata_context
            .is_standalone()
    );
    assert!(
        supplied
            .ir_module
            .original_authored_declaration_body_units
            .is_empty()
    );
    // No supplied entry: a fresh tclsh defines `::original` (user decision
    // 2026-10-19T14:20Z); an unentered card needs explicit unknown history.
    assert!(supplied.function("::original").is_some());
    let unknown_supplied = CompilationUnit::build_with_analysis_input(
        source,
        options,
        Some(&crate::command_binding::SourceAnalysisEntry::unknown_history_for_test(&input)),
        &input,
    );
    assert_eq!(
        unknown_supplied
            .ir_module
            .original_native_declaration_body_units
            .len(),
        1
    );
    let entry = crate::command_binding::SourceAnalysisEntry {
        unknown_entry: true,
        ..Default::default()
    };
    let unknown = CompilationUnit::build_with_source_entry(source, options, &entry);
    assert!(unknown.ir_module.source_entry.unknown_entry);
    assert!(
        unknown
            .ir_module
            .original_authored_declaration_body_units
            .is_empty()
    );
}

#[test]
fn supplied_source_producer_refuses_changed_profile_and_lexer_metadata() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Actual input is retained for reporting; unrelated producer options grant no facts.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::clone(&context),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let options = UnitBuildOptions {
        registry: context.commands(),
        defer_top_level: false,
        config: input.lexer_config(),
        dialect: Some(profile),
        external_call_sites: None,
        declared_commands: None,
    };
    let selected = CompilationUnit::build_with_analysis_input("puts VALUE", options, None, &input);
    assert!(
        selected
            .top_level
            .invocation_metadata_context_for_module(context.commands(), &selected.ir_module,)
            .is_some()
    );
    let mut wrong_config = options;
    wrong_config.config.strict_quoting = !wrong_config.config.strict_quoting;
    let wrong_profile = UnitBuildOptions {
        dialect: Some(tcl_dialect::DialectProfile::find("tcl8.6").unwrap()),
        ..options
    };
    for options in [wrong_config, wrong_profile] {
        let refused =
            CompilationUnit::build_with_analysis_input("puts VALUE", options, None, &input);
        assert_eq!(
            refused.ir_module.source_metadata_input.as_ref(),
            Some(&input)
        );
        assert!(refused.ir_module.source_entry.unknown_entry);
        assert!(
            refused
                .ir_module
                .source_entry
                .logical_source_input
                .is_none()
        );
        assert!(refused.top_level.source_metadata_input().is_none());
        assert!(
            refused
                .top_level
                .invocation_metadata_context_for_module(context.commands(), &refused.ir_module,)
                .is_none()
        );
        assert!(matches!(
            refused.top_level.semantic_facts.executable(),
            crate::semantic_analysis::ExecutableAnalysisAvailability::ContextUnavailable
        ));
    }
    let mut stale_module = selected.ir_module.clone();
    stale_module.dialect_profile = wrong_profile.dialect;
    assert!(
        selected
            .top_level
            .invocation_metadata_context_for_module(context.commands(), &stale_module,)
            .is_none()
    );
}

#[test]
fn supplied_hosted_source_entry_retains_advice_without_activation() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    for environment in ["f5-irules", "f5-iapps"] {
        let environment = tcl_registry::model::ingress::resolve_environment(environment);
        let profile = environment.unit_profile();
        let context = environment.default_context_registry();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let unit = CompilationUnit::build_with_analysis_input(
            "set source_value VALUE",
            UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        );
        let entry = &unit.ir_module.source_entry;
        assert_eq!(entry.vendor_source_input.as_ref(), Some(&input));
        assert!(entry.native_entry.is_none());
        assert!(entry.logical_source_input.is_none());
        // Documented correction (user decision 2026-10-21T00:10Z,
        // "realm-aware command world"): a hosted document supplied without
        // an entry is a fresh rule load where the dialect admits the
        // authored F5 naming simulation; elsewhere history stays unknown.
        if let Some(fresh) =
            crate::command_binding::SourceAnalysisEntry::for_fresh_hosted_source(&input)
                .filter(|fresh| !fresh.unknown_entry)
        {
            assert!(!entry.unknown_entry);
            assert_eq!(
                entry.invocation_realm,
                tcl_dialect::model::InvocationRealm::RuleLoader
            );
            assert_eq!(entry.execution_name_policy, fresh.execution_name_policy);
            // The fresh load selects its environment's hosted context, so
            // an iRule's handlers address TMM's `::static`.
            assert_eq!(
                entry.hosted_execution_context,
                tcl_dialect::model::bigip_execution_context::BigIpExecutionContext::for_environment(
                    &input.availability_context().environment.id
                )
            );
            assert!(entry.native_entry.is_none());
        } else {
            assert!(entry.unknown_entry);
            assert!(entry.execution_name_policy.is_none());
            assert!(entry.hosted_execution_context.is_none());
        }
    }
}

#[test]
fn semantic_value_projection_uses_actual_jim_entry_over_c_catalogue() {
    let registry = registry();
    // The actual interpreter entry selects Jim independently of the C catalogue.
    // This software fixture observes no external provider execution.
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let context = Arc::new(
        tcl_registry::model::ingress::static_context_for("jim")
            .with_command_store(registry.snapshot().shared_registry()),
    );
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let (_runtime_owner, captured) =
        crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        metadata_context:
            crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(Some(
                &input,
            )),
        native_entry: Some(Arc::new(captured)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
            ..Default::default()
        },
        ..Default::default()
    };
    let source =
        "set x 003; set y [expr {$x}]; set z [expr {9223372036854775807 + 1}]; expr {$z + 1}";
    let compiled = CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry: &registry,
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        &input,
    );
    assert!(compiled.ir_module.source_entry.native_entry.is_some());
    assert_eq!(
        compiled.ir_module.source_metadata_input.as_ref(),
        Some(&input)
    );
    let unit = &compiled.top_level;
    let facts = unit
        .semantic_values()
        .expect("retained actual engine inputs");
    assert!(facts.contents_iter().any(|(_, value)| {
        *value
            == crate::analyses::LatticeValue::Const(crate::analyses::ConstValue::String(
                "003".into(),
            ))
    }));
    assert!(facts.expression_iter().any(|(_, analysis)| {
        analysis.evaluation.value == crate::tcl_expr_eval::TclValue::Int(i64::MIN)
    }));
    assert!(facts.expression_iter().any(|(_, analysis)| {
        analysis.evaluation.value == crate::tcl_expr_eval::TclValue::Int(i64::MIN + 1)
    }));
    assert_eq!(
        unit.semantic_value_projection,
        Arc::new(unit.semantic_value_projection.uncached())
    );
    assert!(std::ptr::eq(facts, unit.semantic_values().unwrap()));
}
