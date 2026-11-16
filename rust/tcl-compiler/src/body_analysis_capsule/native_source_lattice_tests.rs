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

// Unentered native declaration cards exist only under explicitly supplied
// unknown runtime history; with no entry a fresh interpreter defines each
// procedure (user decision 2026-10-19T14:20Z).
use super::*;
use crate::compilation_unit::{FunctionLatticeInputs, LatticeRequest, UnitDialect};

fn build_current(req: &LatticeRequest<'_>, registry: &CommandRegistry) -> FunctionUnit {
    let metadata = req.original_metadata_context(registry);
    let prepared = crate::cfg_builder::PreparedCfgContext::from_metadata_input(
        (
            req.upvar_procs.clone(),
            req.proc_params.clone(),
            req.global_write_procs.clone(),
            req.command_bindings.clone(),
            req.procedure_summary_owners.clone(),
        ),
        registry,
        crate::registry_invocation::InvocationMetadataInput::Retained(&metadata),
    );
    let cfg = crate::cfg_builder::build_cfg_function_with_prepared_context(
        req.qname,
        req.body,
        true,
        registry,
        req.plain_command_dispatch,
        &prepared,
        req.lexer_config,
    );
    let known_classes = req.known_classes.iter().cloned().collect();
    let traced_variables = req.traced_variables.iter().cloned().collect();
    let parameters = crate::compilation_unit::decode_param_constants(req.param_constants);
    FunctionUnit::build_for_original_declaration_lattice(
        req.body_id,
        req.qname,
        cfg,
        req.params,
        FunctionLatticeInputs {
            dialect: UnitDialect {
                registry,
                config: req.lexer_config,
                source_metadata_input: req.source_metadata_input,
            },
            param_constants: parameters.as_ref(),
            known_classes: &known_classes,
            trace_facts: crate::compilation_unit::ModuleTraceFacts {
                traced_variables: &traced_variables,
                has_dynamic_variable_trace: req.has_dynamic_variable_trace,
                deferred_writes: &req.analysis_context.deferred_writes,
            },
            analysis_context: req.analysis_context,
            command_trust: req.command_trust,
            event_body: None,
        },
    )
    .with_retained_semantic_analysis(
        registry,
        Some(req.body),
        crate::dispatch_proof::DispatchEntryAssumption::UnknownWorld,
    )
}

#[test]
fn logical_declared_body_retains_raw_inputs_without_certifying_neutral_reuse() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let foreign_context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let foreign_input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::clone(&foreign_context),
        config,
    );
    let mut visited = 0;
    let mut memo = |request: &LatticeRequest<'_>| {
        visited += 1;
        assert!(
            request
                .module
                .original_logical_declaration_body_units
                .contains_key(&request.body_id)
        );
        let retained = BodyLatticeInputs::retain(request, context.commands())
            .expect("authentic current Logical declaration body");
        assert_eq!(retained.body(), request.body);
        assert!(retained.certificate.is_none());
        assert!(retained.command_bindings().is_none());
        assert!(retained.certified_native_source_body_id().is_none());
        let missing = LatticeRequest {
            source_metadata_input: None,
            ..*request
        };
        assert!(BodyLatticeInputs::retain(&missing, context.commands()).is_none());
        let foreign = LatticeRequest {
            source_metadata_input: Some(&foreign_input),
            ..*request
        };
        assert!(BodyLatticeInputs::retain(&foreign, context.commands()).is_none());
        let mut changed = request.module.clone();
        changed
            .body_units
            .get_mut(&request.body_id)
            .unwrap()
            .params
            .push("extra".into());
        let changed = LatticeRequest {
            module: &changed,
            ..*request
        };
        assert!(BodyLatticeInputs::retain(&changed, context.commands()).is_none());
        build_current(request, context.commands())
    };
    let no_isolated_body = |_: &str, _: &str| -> crate::ir::Script { panic!("source-only body") };
    let unit = CompilationUnit::build_memoized_with_analysis_input(
        "proc absent::logical {x} {return $x}\n",
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            config,
            dialect: Some(profile),
            defer_top_level: false,
            external_call_sites: None,
            declared_commands: None,
        },
        &mut memo,
        &no_isolated_body,
        None,
        &input,
    );
    drop(memo);
    assert_eq!(visited, 1);
    assert_eq!(
        unit.ir_module.original_logical_declaration_body_units.len(),
        1
    );
}

#[test]
fn readonly_source_restoration_preserves_absence_of_execution_and_refuses_missing_carriers() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = context.commands().profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let source = "proc p {} {set value TEXT; return $value}\n";
    let options = crate::compilation_unit::UnitBuildOptions {
        registry: context.commands(),
        config,
        dialect: Some(profile),
        defer_top_level: false,
        external_call_sites: None,
        declared_commands: None,
    };
    let unit = CompilationUnit::build_with_analysis_input(
        source,
        options,
        Some(&crate::command_binding::SourceAnalysisEntry::unknown_history_for_test(&input)),
        &input,
    );
    let (&id, card) = unit
        .ir_module
        .original_native_declaration_body_units
        .iter()
        .next()
        .unwrap();
    let procedure = unit.original_procedure_for_body(id).unwrap();
    assert!(card.matches_procedure(&unit.ir_module, procedure));
    let function = unit.function_for_body(Some(id)).unwrap();
    let proofs = crate::command_binding::BodySourceProofs::from_body(&procedure.body);
    let restored = function
        .restored_readonly_source_proofs(&proofs)
        .expect("current conditional source carriers without execution");
    assert!(restored.cfg.executed_source.is_none());
    assert!(restored.cfg.native_compilation_admission.is_none());
    assert!(restored.cfg.statement_sources.values().all(Option::is_none));
    assert!(Arc::ptr_eq(&restored.def_use, &function.def_use));
    assert!(function.restored_source_proofs(&proofs).is_none());
    let mut missing = procedure.body.clone();
    missing.command_binding_sites = crate::ir::CommandBindingSites::default();
    for statement in &mut missing.statements {
        if let Some(tokens) = statement.tokens_mut() {
            tokens.source_binding = None;
        }
    }
    let missing = crate::command_binding::BodySourceProofs::from_body(&missing);
    assert!(function.restored_readonly_source_proofs(&missing).is_none());
    let logical_profile = tcl_dialect::DialectProfile::plain_tcl();
    let logical_input = crate::analyser::ResolvedAnalysisInput::new(
        logical_profile,
        logical_profile,
        Arc::clone(&context),
        tcl_lexer::LexerConfig::for_file_grammar(logical_profile.grammar),
    );
    let logical = CompilationUnit::build_with_analysis_input(
        source,
        crate::compilation_unit::UnitBuildOptions {
            config: logical_input.lexer_config(),
            dialect: Some(logical_profile),
            ..options
        },
        None,
        &logical_input,
    );
    let entered = logical
        .ir_module
        .procedures
        .values()
        .next()
        .unwrap()
        .body
        .executed_source
        .as_ref()
        .unwrap();
    let mut changed = function.clone();
    changed.cfg.executed_source = Some(Arc::clone(entered));
    assert!(changed.restored_readonly_source_proofs(&proofs).is_none());
    let mut changed_body = procedure.body.clone();
    changed_body.executed_source = Some(Arc::clone(entered));
    let mixed = crate::command_binding::BodySourceProofs::from_body(&changed_body);
    assert!(function.restored_readonly_source_proofs(&mixed).is_none());
}

#[test]
fn native_source_capsules_retain_current_owners_under_certified_unrelated_edits() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = context.commands().profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let build = |other: &str| {
        Arc::new(CompilationUnit::build_with_analysis_input(
            &format!(
                "proc kept {{x}} {{set value $x; return $value}}\nproc other {{}} {{{other}}}\n"
            ),
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&crate::command_binding::SourceAnalysisEntry::unknown_history_for_test(&input)),
            &input,
        ))
    };
    let left = build("return BEFORE");
    let right = build("return AFTER");
    let select = |unit: &CompilationUnit| {
        *unit
            .ir_module
            .original_native_declaration_body_units
            .iter()
            .find(|(_, card)| card.original_name_input().bytes() == b"kept")
            .unwrap()
            .0
    };
    let left_id = select(&left);
    let right_id = select(&right);
    assert_ne!(left_id, right_id);
    let a = BodyAnalysisCapsule::retain(Arc::clone(&left), left_id, context.commands()).unwrap();
    let b = BodyAnalysisCapsule::retain(Arc::clone(&right), right_id, context.commands()).unwrap();
    assert!(a.has_closed_analysis_correspondence());
    assert!(a.has_closed_readonly_source_correspondence());
    assert!(!a.has_closed_native_correspondence());
    assert_eq!(a, b);
    assert!(std::ptr::eq(b.unit(), right.as_ref()));
    assert_eq!(b.body_id(), right_id);
    assert!(std::ptr::eq(
        b.function(),
        right.function_for_body(Some(right_id)).unwrap()
    ));
    assert!(BodyAnalysisCapsule::retain(Arc::clone(&right), left_id, context.commands()).is_none());
    let mut missing_input = (*right).clone();
    missing_input.ir_module.source_metadata_input = None;
    assert!(
        BodyAnalysisCapsule::retain(Arc::new(missing_input), right_id, context.commands())
            .is_none()
    );
}

fn check_native_source_request_dependencies(
    req: &LatticeRequest<'_>,
    context: &tcl_registry::model::ContextRegistry,
    foreign_input: &crate::analyser::ResolvedAnalysisInput,
    foreign_context: &tcl_registry::model::ContextRegistry,
    foreign_script: &crate::ir::Script,
) -> FunctionUnit {
    let source_card = req
        .module
        .original_native_declaration_body_units
        .get(&req.body_id)
        .unwrap();
    assert!(
        source_card.matches_procedure(req.module, req.module.body_units.get(&req.body_id).unwrap())
    );
    if req.body_source == Some("set value $x; return $value") {
        assert!(BodyLatticeInputs::retain(req, context.commands()).is_some());
        let foreign_source_input = LatticeRequest {
            source_metadata_input: Some(foreign_input),
            ..*req
        };
        assert!(BodyLatticeInputs::retain(&foreign_source_input, context.commands()).is_none());
        assert!(BodyLatticeInputs::retain(req, foreign_context.commands()).is_none());
        let mut changed_store = req.module.clone();
        changed_store.body_units.get_mut(&req.body_id).unwrap().body = foreign_script.clone();
        let foreign_store_request = LatticeRequest {
            module: &changed_store,
            ..*req
        };
        assert!(BodyLatticeInputs::retain(&foreign_store_request, context.commands()).is_none());
    } else {
        assert!(
            BodyLatticeInputs::retain(req, context.commands()).is_none(),
            "{}",
            req.body_source.unwrap()
        );
    }
    build_current(req, context.commands())
}

fn check_logical_source_request_without_publication(
    req: &LatticeRequest<'_>,
    context: &tcl_registry::model::ContextRegistry,
) -> FunctionUnit {
    let input = BodyLatticeInputs::retain(req, context.commands()).unwrap();
    assert!(input.certified_native_source_body_id().is_none());
    assert!(
        req.command_bindings
            .prepare_native_source_body_correspondence(
                req.body,
                crate::command_binding::BodyProofScope {
                    source: req.source,
                    body_source: req.body_source.unwrap(),
                    original_body_offset: req.original_body_offset,
                    executable_body_offset: req.executable_body_offset,
                    metadata: crate::registry_invocation::InvocationMetadataInput::SuppliedSource(
                        req.source_metadata_input
                    ),
                    config: req.lexer_config,
                },
                req.module,
                req.body_id,
                context.commands()
            )
            .is_none()
    );
    build_current(req, context.commands())
}

fn capsule_test_options<'a>(
    registry: &'a CommandRegistry,
    config: tcl_lexer::LexerConfig,
    profile: &'static tcl_dialect::DialectProfile,
) -> crate::compilation_unit::UnitBuildOptions<'a> {
    crate::compilation_unit::UnitBuildOptions {
        registry,
        defer_top_level: false,
        config,
        dialect: Some(profile),
        external_call_sites: None,
        declared_commands: None,
    }
}

#[test]
fn native_source_lattice_certificate_refuses_genuine_dependency_and_owner_changes() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = context.commands().profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let foreign_context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let foreign_profile = foreign_context.commands().profile().unwrap();
    let foreign_input = crate::analyser::ResolvedAnalysisInput::new(
        foreign_profile,
        foreign_profile,
        Arc::clone(&foreign_context),
        tcl_lexer::LexerConfig::for_file_grammar(foreign_profile.grammar),
    );
    let foreign_store = CompilationUnit::build_with_analysis_input(
        "proc a {x} {return FOREIGN}\n",
        capsule_test_options(context.commands(), config, profile),
        Some(&crate::command_binding::SourceAnalysisEntry::unknown_history_for_test(&input)),
        &input,
    );
    let foreign_script = foreign_store
        .ir_module
        .body_units
        .values()
        .next()
        .unwrap()
        .body
        .clone();
    let mut visited = 0;
    let mut memo = |req: &LatticeRequest<'_>| {
        visited += 1;
        check_native_source_request_dependencies(
            req,
            &context,
            &foreign_input,
            &foreign_context,
            &foreign_script,
        )
    };
    let unavailable_body_cache =
        |_: &str, _: &str| -> crate::ir::Script { panic!("source-only body cache") };
    let source = "proc a {x} {set value $x; return $value}\nproc store {x} {upvar 0 outside local; return $local}\nproc observer {x} {trace add execution return enter hook; return $x}\nproc tracing {x} {trace add variable x read hook; return $x}\nproc document {x} {helper $x}\n";
    let unit = CompilationUnit::build_memoized_with_analysis_input(
        source,
        capsule_test_options(context.commands(), config, profile),
        &mut memo,
        &unavailable_body_cache,
        None,
        &input,
    );
    drop(memo);
    assert_eq!(visited, 5);
    assert_eq!(
        unit.ir_module.original_native_declaration_body_units.len(),
        5
    );
    let logical_profile = tcl_dialect::DialectProfile::plain_tcl();
    let logical_input = crate::analyser::ResolvedAnalysisInput::new(
        logical_profile,
        logical_profile,
        Arc::clone(&context),
        tcl_lexer::LexerConfig::for_file_grammar(logical_profile.grammar),
    );
    let mut logical_seen = 0;
    let mut logical_memo = |req: &LatticeRequest<'_>| {
        logical_seen += 1;
        check_logical_source_request_without_publication(req, &context)
    };
    let _ = CompilationUnit::build_memoized_with_analysis_input(
        "proc logical {x} {return $x}\n",
        capsule_test_options(
            context.commands(),
            logical_input.lexer_config(),
            logical_profile,
        ),
        &mut logical_memo,
        &unavailable_body_cache,
        None,
        &logical_input,
    );
    drop(logical_memo);
    assert_eq!(logical_seen, 1);
}

#[test]
fn logical_named_source_retains_raw_lattice_without_native_publication() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::clone(&context),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let mut visited = 0;
    let mut memo = |request: &LatticeRequest<'_>| {
        visited += 1;
        let procedure = request.original_source_procedure().unwrap();
        assert_eq!(procedure.body_kind, crate::ir::ProcedureBodyKind::Named);
        assert!(procedure.publication.is_none());
        assert!(
            !request
                .module
                .original_logical_declaration_body_units
                .contains_key(&request.body_id)
        );
        let raw = BodyLatticeInputs::retain(request, context.commands()).unwrap();
        assert!(raw.command_bindings().is_none());
        assert!(raw.certified_native_source_body_id().is_none());
        assert!(raw.certificate.is_none());
        let missing = LatticeRequest {
            source_metadata_input: None,
            ..*request
        };
        assert!(BodyLatticeInputs::retain(&missing, context.commands()).is_none());
        let mut changed = request.module.clone();
        changed
            .procedures
            .get_mut(&request.body_id)
            .unwrap()
            .params
            .push("foreign".to_owned());
        let altered = LatticeRequest {
            module: &changed,
            ..*request
        };
        assert!(BodyLatticeInputs::retain(&altered, context.commands()).is_none());
        build_current(request, context.commands())
    };
    let unavailable_body_cache =
        |_: &str, _: &str| -> crate::ir::Script { panic!("source-only body cache") };
    let _unit = CompilationUnit::build_memoized_with_analysis_input(
        "proc logical {x} {return $x}\n",
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config: input.lexer_config(),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        &mut memo,
        &unavailable_body_cache,
        None,
        &input,
    );
    drop(memo);
    assert_eq!(visited, 1);
}

#[test]
fn authored_source_capsules_keep_independent_standalone_analysis_purpose() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = context.commands().profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let options = crate::compilation_unit::UnitBuildOptions {
        registry: context.commands(),
        config,
        dialect: Some(profile),
        defer_top_level: false,
        external_call_sites: None,
        declared_commands: None,
    };
    let build = |other: &str| {
        Arc::new(CompilationUnit::build_with_options(
            &format!(
                "proc kept {{x}} {{set value $x; return $value}}\nproc other {{}} {{return {other}}}\n"
            ),
            options,
        ))
    };
    let left = build("BEFORE");
    let right = build("AFTER");
    let select = |unit: &CompilationUnit| {
        *unit
            .ir_module
            .original_authored_declaration_body_units
            .iter()
            .find(|(_, card)| card.name() == "kept")
            .unwrap()
            .0
    };
    let a =
        BodyAnalysisCapsule::retain(Arc::clone(&left), select(&left), context.commands()).unwrap();
    let b = BodyAnalysisCapsule::retain(Arc::clone(&right), select(&right), context.commands())
        .unwrap();
    assert!(left.ir_module.source_entry.metadata_context.is_standalone());
    assert!(left.ir_module.source_metadata_input.is_none());
    assert!(left.ir_module.source_entry.native_entry.is_none());
    assert!(a.has_closed_analysis_correspondence());
    assert!(a.has_closed_readonly_source_correspondence());
    assert!(!a.has_closed_native_correspondence());
    assert!(
        a == b,
        "same certified standalone body after unrelated source edit"
    );
    let foreign = CommandRegistry::build_default();
    assert!(BodyAnalysisCapsule::retain(Arc::clone(&left), select(&left), &foreign).is_none());
    let mut missing = (*left).clone();
    missing
        .ir_module
        .original_authored_declaration_body_units
        .clear();
    assert!(
        BodyAnalysisCapsule::retain(Arc::new(missing), select(&left), context.commands()).is_none()
    );
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let supplied = Arc::new(CompilationUnit::build_with_analysis_input(
        "proc kept {x} {set value $x; return $value}",
        options,
        Some(&crate::command_binding::SourceAnalysisEntry::unknown_history_for_test(&input)),
        &input,
    ));
    assert!(
        supplied
            .ir_module
            .original_authored_declaration_body_units
            .is_empty()
    );
    assert!(
        !supplied
            .ir_module
            .original_native_declaration_body_units
            .is_empty()
    );
}

#[test]
fn authored_source_lattice_certificate_rebinds_actual_ids_and_refuses_missing_owner() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = context.commands().profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let options = crate::compilation_unit::UnitBuildOptions {
        registry: context.commands(),
        config,
        dialect: Some(profile),
        defer_top_level: false,
        external_call_sites: None,
        declared_commands: None,
    };
    let mut previous: Option<(BodyLatticeInputs, FunctionUnit)> = None;
    let mut computations = 0;
    let mut ids = Vec::new();
    let mut memo = |request: &LatticeRequest<'_>| {
        assert!(request.module.source_entry.metadata_context.is_standalone());
        let inputs = BodyLatticeInputs::retain(request, context.commands())
            .expect("authentic standalone scalar source dependencies");
        assert_eq!(
            inputs.certified_readonly_source_body_id(),
            Some(request.body_id)
        );
        assert!(inputs.certified_native_source_body_id().is_none());
        let mut missing = request.module.clone();
        missing.original_authored_declaration_body_units.clear();
        assert!(
            BodyLatticeInputs::retain(
                &LatticeRequest {
                    module: &missing,
                    ..*request
                },
                context.commands()
            )
            .is_none()
        );
        let mut wrong_config = LatticeRequest { ..*request };
        wrong_config.lexer_config.strict_quoting = !wrong_config.lexer_config.strict_quoting;
        assert!(BodyLatticeInputs::retain(&wrong_config, context.commands()).is_none());
        if request.qname == "::kept" {
            ids.push(request.body_id);
            if let Some((old, cached)) = &previous {
                assert!(
                    &inputs == old,
                    "same complete standalone source dependencies"
                );
                let rebound = inputs
                    .rebind_cached(old, cached)
                    .expect("same typed source analysis, current issued owner");
                assert_eq!(rebound.body_id, Some(request.body_id));
                assert!(Arc::ptr_eq(&rebound.def_use, &cached.def_use));
                assert!(rebound.cfg.executed_source.is_none());
                assert!(rebound.cfg.native_compilation_admission.is_none());
                return rebound;
            }
            computations += 1;
            let current = build_current(request, context.commands());
            previous = Some((inputs, current.clone()));
            current
        } else {
            build_current(request, context.commands())
        }
    };
    for source in [
        "proc kept {x} {set value $x; return $value}\nproc other {} {return BEFORE}\n",
        "\nproc kept {x} {set value $x; return $value}\nproc other {} {return AFTER}\n",
    ] {
        let unit = CompilationUnit::build_for_memoized(source, options, &mut memo);
        assert!(unit.ir_module.source_metadata_input.is_none());
        assert!(unit.ir_module.source_entry.native_entry.is_none());
    }
    drop(memo);
    assert_eq!(computations, 1);
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
}

#[test]
fn native_source_lattice_correspondence_rebinds_actual_ids_after_an_unrelated_edit() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = context.commands().profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let mut retained: Option<(BodyLatticeInputs, FunctionUnit)> = None;
    let mut a_computations = 0;
    let mut a_ids = Vec::new();
    let mut memo = |req: &LatticeRequest<'_>| {
        assert!(req.module.source_entry.native_entry.is_none());
        assert!(
            req.module
                .original_native_declaration_body_units
                .contains_key(&req.body_id)
        );
        let inputs = BodyLatticeInputs::retain(req, context.commands())
            .expect("actual Native source dependency certificate");
        assert_eq!(inputs.certified_native_source_body_id(), Some(req.body_id));
        let mut unavailable = req.module.clone();
        unavailable.source_metadata_input = None;
        let missing = LatticeRequest {
            module: &unavailable,
            ..*req
        };
        assert!(BodyLatticeInputs::retain(&missing, context.commands()).is_none());
        if req.qname == "::a" {
            a_ids.push(req.body_id);
            if let Some((previous, cached)) = &retained {
                assert_eq!(inputs, *previous);
                let rebound = inputs
                    .rebind_cached(previous, cached)
                    .expect("current genuine source restoration");
                assert_eq!(rebound.body_id, Some(req.body_id));
                assert!(Arc::ptr_eq(&rebound.def_use, &cached.def_use));
                return rebound;
            }
            a_computations += 1;
            let computed = build_current(req, context.commands());
            retained = Some((inputs, computed.clone()));
            computed
        } else {
            build_current(req, context.commands())
        }
    };
    let unavailable_body_cache = |_: &str, _: &str| -> crate::ir::Script {
        panic!("source-only declarations cannot enter the executable body cache")
    };
    for source in [
        "proc a {x} {set y $x; return $y}\nproc b {} {return TWO}\n",
        "\nproc a {x} {set y $x; return $y}\nproc b {} {return THREE}\n",
    ] {
        let unit = CompilationUnit::build_memoized_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            &mut memo,
            &unavailable_body_cache,
            None,
            &input,
        );
        assert_eq!(
            unit.ir_module.original_native_declaration_body_units.len(),
            2
        );
        assert!(
            unit.ir_module
                .original_native_declaration_body_units
                .keys()
                .all(|id| unit.function_for_body(Some(*id)).is_some())
        );
    }
    drop(memo);
    assert_eq!(a_computations, 1);
    assert_eq!(a_ids.len(), 2);
    assert_ne!(a_ids[0], a_ids[1]);
}
