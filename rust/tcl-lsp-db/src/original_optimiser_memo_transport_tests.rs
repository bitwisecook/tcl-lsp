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

#[test]
fn isolated_body_report_and_legacy_projection_share_the_actual_lowering_pass() {
    let log = Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = Arc::clone(&log);
    let db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(move |event: salsa::Event| {
            if let salsa::EventKind::WillExecute { database_key } = event.kind {
                sink.lock().unwrap().push(format!("{database_key:?}"));
            }
        }))),
    };
    let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = context.commands();
    let profile = tcl_lsp_core::stated_profile_for_dialect("tcl8.6").unwrap();
    let key = ProcBodyKey::new(
        &db,
        "set value 1; return $value".to_owned(),
        "::".to_owned(),
        profile.name.to_owned(),
        CompilerMemoSnapshot {
            profile: Some(profile.cache_key()),
            registry: registry.snapshot(),
        },
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    );
    let original = original_lower_proc_body(&db, key);
    let legacy = lower_proc_body(&db, key);
    assert_eq!(*legacy, *original.body());
    let repeated = original_lower_proc_body(&db, key);
    assert_eq!(original, repeated);
    assert_eq!(
        log.lock()
            .unwrap()
            .iter()
            .filter(|query| query.contains("original_lower_proc_body"))
            .count(),
        1
    );
}

#[test]
fn body_optimiser_memo_retains_the_actual_optional_worker_dialect() {
    let log = Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = Arc::clone(&log);
    let db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(move |event: salsa::Event| {
            if let salsa::EventKind::WillExecute { database_key } = event.kind {
                sink.lock().unwrap().push(format!("{database_key:?}"));
            }
        }))),
    };
    let file = SourceFile::new(
        &db,
        "proc sample {} {puts VALUE}\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let config = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let unit = document_compilation_unit_for(&db, file, config).unwrap();
    let input = unit.ir_module.source_metadata_input.as_ref().unwrap();
    let registry = input.borrowed_context_registry().commands();
    // Test-input correction (user decision 2026-10-19T14:20Z, fresh
    // interpreter): a document with no explicit entry runs `proc sample`,
    // so its body is the installed procedure, not a declaration-only
    // native body unit. The memo assertions below are unchanged.
    let body_ids: Vec<_> = unit.ir_module.procedures.keys().copied().collect();
    assert_eq!(body_ids.len(), 1, "{body_ids:?}");
    let body_id = body_ids[0];
    let capsule = tcl_compiler::body_analysis_capsule::BodyAnalysisCapsule::retain(
        Arc::clone(&unit),
        body_id,
        registry,
    )
    .unwrap();
    let none = OptDepsKey::new(
        &db,
        capsule.clone(),
        None,
        capsule.original_optimiser_analysis_context(registry, None),
    );
    let selected = OptDepsKey::new(
        &db,
        capsule.clone(),
        Some(input.unit_profile().cache_key()),
        capsule.original_optimiser_analysis_context(registry, Some(input.unit_profile())),
    );
    assert!(none != selected);
    log.lock().unwrap().clear();
    let none_report = function_optimisations(&db, none);
    let selected_report = function_optimisations(&db, selected);
    assert!(Arc::ptr_eq(
        &none_report,
        &function_optimisations(&db, none)
    ));
    assert!(Arc::ptr_eq(
        &selected_report,
        &function_optimisations(&db, selected)
    ));
    assert_eq!(
        log.lock()
            .unwrap()
            .iter()
            .filter(|query| query.contains("function_optimisations"))
            .count(),
        2
    );
    assert_eq!(
        none_report.members(),
        capsule.original_body_optimisations(registry, None)
    );
    assert_eq!(
        selected_report.members(),
        capsule.original_body_optimisations(registry, Some(input.unit_profile()))
    );
}
#[test]
fn a_legacy_lattice_key_cannot_replace_a_later_original_worker_request() {
    let db = TclDatabase::default();
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let profile = tcl_lsp_core::stated_profile_for_dialect("tcl8.6").unwrap();
    let mut checked = 0;
    let mut memo = |request: &LatticeRequest<'_>| {
        let mut upvar: Vec<_> = request
            .upvar_procs
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        let mut params: Vec<_> = request
            .proc_params
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        let mut writes: Vec<_> = request
            .global_write_procs
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        upvar.sort_by_key(|(id, _)| *id);
        params.sort_by_key(|(id, _)| *id);
        writes.sort_by_key(|(id, _)| *id);
        let context = CfgContext::new(
            &db,
            upvar,
            params,
            writes,
            BodyLatticeCommandBindings::from(request.command_bindings.clone()),
            request.procedure_summary_owners.clone(),
        );
        let snapshot = registry.snapshot();
        let legacy = lattice_request_key(
            &db,
            request,
            BodyLatticeInputs::from(request.body.clone()),
            context,
            &snapshot,
            evaluator_epoch(&db),
        );
        let legacy_report = function_lattice(&db, legacy);
        let actual_body = BodyLatticeInputs::retain_original_worker_request(request, registry)
            .expect("the real private source callback retains its own request");
        let original = lattice_request_key(
            &db,
            request,
            actual_body,
            context,
            &snapshot,
            evaluator_epoch(&db),
        );
        assert!(legacy != original);
        let original_report = function_lattice(&db, original);
        assert!(!Arc::ptr_eq(&legacy_report, &original_report));
        assert_eq!(*legacy_report, *original_report);
        assert!(Arc::ptr_eq(
            &original_report,
            &function_lattice(&db, original)
        ));
        checked += 1;
        (*original_report).clone()
    };
    let unit = CompilationUnit::build_for_memoized(
        "proc sample {value} {puts $value}\n",
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        &mut memo,
    );
    assert_eq!(checked, 1);
    assert_eq!(unit.procedures.len() + unit.body_units.len(), 1);
}
#[test]
fn legacy_equal_capsules_cannot_occupy_original_report_memo_keys() {
    let mut analyser = tcl_compiler::analyser::Analyser::new();
    let input = analyser.prepare_analysis_input("tcl8.6").unwrap();
    let registry = input.borrowed_context_registry().commands();
    let profile = input.unit_profile();
    let base = CompilationUnit::build_standalone_with_analysis_input(
        "proc sample {value} {set length [llength $value]; puts $length}\n",
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: input.lexer_config(),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        &input,
    )
    .unwrap();
    let current = Arc::new(base.clone().with_interprocedural(registry, Some(profile)));
    let mut legacy_calls = 0;
    let legacy = Arc::new(base.clone().with_interprocedural_memoized(
        registry,
        Some(profile),
        &mut |body_id, _, summary| {
            legacy_calls += 1;
            base.original_interproc_taints_for_body(Some(body_id), registry, summary)
                .map(Arc::new)
        },
    ));
    assert_eq!(legacy_calls, 1);
    assert_eq!(*legacy, *current);
    let body_id = current
        .all_body_function_units()
        .find_map(|function| function.body_id)
        .unwrap();
    let legacy =
        tcl_compiler::body_analysis_capsule::BodyAnalysisCapsule::retain(legacy, body_id, registry)
            .unwrap();
    let current = tcl_compiler::body_analysis_capsule::BodyAnalysisCapsule::retain(
        current, body_id, registry,
    )
    .unwrap();
    assert_eq!(legacy, current);
    let dialect = Some(profile);
    let legacy_context = legacy.original_optimiser_analysis_context(registry, dialect);
    let current_context = current.original_optimiser_analysis_context(registry, dialect);
    assert!(legacy_context.is_none());
    assert!(current_context.is_some());
    let db = TclDatabase::default();
    let key_profile = Some(profile.cache_key());
    let legacy_taint =
        OriginalTaintSummaryKey::new(&db, legacy.clone(), key_profile, legacy_context.clone());
    let legacy_taint_report = original_taint_cascade(&db, legacy_taint);
    let current_taint =
        OriginalTaintSummaryKey::new(&db, current.clone(), key_profile, current_context.clone());
    assert!(legacy_taint != current_taint);
    let current_taint_report = original_taint_cascade(&db, current_taint);
    assert_eq!(legacy_taint_report.taints(), current_taint_report.taints());
    assert_ne!(*legacy_taint_report, *current_taint_report);
    assert!(Arc::ptr_eq(
        &current_taint_report,
        &original_taint_cascade(&db, current_taint)
    ));
    let legacy_opts = OptDepsKey::new(&db, legacy, key_profile, legacy_context);
    let legacy_opt_report = function_optimisations(&db, legacy_opts);
    let current_opts = OptDepsKey::new(&db, current, key_profile, current_context);
    assert!(legacy_opts != current_opts);
    let current_opt_report = function_optimisations(&db, current_opts);
    assert_eq!(legacy_opt_report.members(), current_opt_report.members());
    assert_ne!(*legacy_opt_report, *current_opt_report);
    assert!(Arc::ptr_eq(
        &current_opt_report,
        &function_optimisations(&db, current_opts)
    ));
}
struct MemoAbstainHost;
impl tcl_registry::pack_hooks::PackHookHost for MemoAbstainHost {
    fn invoke(
        &self,
        _slot: tcl_registry::pack_hooks::HookSlot,
        _call: &tcl_registry::pack_hooks::HookCall<'_>,
    ) -> tcl_registry::pack_hooks::HookAnswer {
        tcl_registry::pack_hooks::HookAnswer::Abstain
    }
}
struct MemoClearHost;
impl Drop for MemoClearHost {
    fn drop(&mut self) {
        tcl_registry::pack_hooks::clear_host();
    }
}

#[test]
fn original_report_keys_keep_actual_evaluator_context_across_neutral_capsules() {
    std::thread::spawn(|| {
        let _clear = MemoClearHost;
        tcl_registry::pack_hooks::install_host(std::rc::Rc::new(MemoAbstainHost));
        let mut analyser = tcl_compiler::analyser::Analyser::new();
        let input = analyser.prepare_analysis_input("tcl8.6").unwrap();
        let registry = input.borrowed_context_registry().commands();
        let profile = input.unit_profile();
        let dialect = Some(profile);
        let build = || {
            let unit = Arc::new(
                CompilationUnit::build_standalone_with_analysis_input(
                    "proc sample {value} {set length [llength $value]; puts $length}\n",
                    UnitBuildOptions {
                        registry,
                        defer_top_level: false,
                        config: input.lexer_config(),
                        dialect,
                        external_call_sites: None,
                        declared_commands: None,
                    },
                    &input,
                )
                .unwrap()
                .with_interprocedural(registry, dialect),
            );
            let body_id = unit
                .all_body_function_units()
                .find_map(|function| function.body_id)
                .unwrap();
            tcl_compiler::body_analysis_capsule::BodyAnalysisCapsule::retain(
                unit, body_id, registry,
            )
            .unwrap()
        };
        let db = TclDatabase::default();
        let first = build();
        let first_context = first
            .original_optimiser_analysis_context(registry, dialect)
            .unwrap();
        let profile_key = Some(profile.cache_key());
        let first_opts =
            OptDepsKey::new(&db, first.clone(), profile_key, Some(first_context.clone()));
        let first_opt_report = function_optimisations(&db, first_opts);
        let first_taint = OriginalTaintSummaryKey::new(
            &db,
            first.clone(),
            profile_key,
            Some(first_context.clone()),
        );
        let first_taint_report = original_taint_cascade(&db, first_taint);
        tcl_registry::pack_hooks::install_host(std::rc::Rc::new(MemoAbstainHost));
        let second = build();
        let second_context = second
            .original_optimiser_analysis_context(registry, dialect)
            .unwrap();
        assert_ne!(
            first_context.evaluator_revision,
            second_context.evaluator_revision
        );
        assert_eq!(first, second);
        let second_opts = OptDepsKey::new(
            &db,
            second.clone(),
            profile_key,
            Some(second_context.clone()),
        );
        let second_taint =
            OriginalTaintSummaryKey::new(&db, second, profile_key, Some(second_context));
        assert!(first_opts != second_opts);
        assert!(first_taint != second_taint);
        let second_opt_report = function_optimisations(&db, second_opts);
        let second_taint_report = original_taint_cascade(&db, second_taint);
        assert_eq!(first_opt_report.members(), second_opt_report.members());
        assert_eq!(first_taint_report.taints(), second_taint_report.taints());
        assert_ne!(*first_opt_report, *second_opt_report);
        assert_ne!(*first_taint_report, *second_taint_report);
        assert!(Arc::ptr_eq(
            &second_opt_report,
            &function_optimisations(&db, second_opts)
        ));
        assert!(Arc::ptr_eq(
            &second_taint_report,
            &original_taint_cascade(&db, second_taint)
        ));
    })
    .join()
    .unwrap();
}
