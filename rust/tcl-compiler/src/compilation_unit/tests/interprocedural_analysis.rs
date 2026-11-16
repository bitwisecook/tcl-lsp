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

//! The interprocedural pass: original taint replay under its owners, and results retired on a refused module realm.

use super::*;

fn assert_original_taint_owner_withdrawals(
    unit: &CompilationUnit,
    registry: &CommandRegistry,
    analysis: &InterproceduralAnalysis,
    body: crate::ir::ProcedureId,
    profile: &'static tcl_dialect::DialectProfile,
    source: &str,
) {
    let mut missing = unit.clone();
    missing.ir_module.retained_source_bindings = None;
    assert!(
        missing
            .original_interproc_taints_for_body(Some(body), registry, analysis)
            .is_none()
    );
    let mut missing = unit.clone();
    missing.ir_module.source_metadata_input = None;
    assert!(
        missing
            .original_interproc_taints_for_body(Some(body), registry, analysis)
            .is_none()
    );
    let mut missing = unit.clone();
    missing.top_level.source_metadata_input = None;
    assert!(
        missing
            .original_interproc_taints_for_body(None, registry, analysis)
            .is_none()
    );
    let mut changed = unit.clone();
    changed.ir_module.source = tcl_lexer::SourceImage::document(&format!("{source}; puts CHANGED"));
    assert!(
        changed
            .original_interproc_taints_for_body(Some(body), registry, analysis)
            .is_none()
    );
    let mut foreign_registry = registry.project_for_profile(profile);
    let mut extra = registry.get("puts").unwrap().clone();
    extra.name = "taint_source_owner_control";
    foreign_registry.insert(extra);
    assert!(
        unit.original_interproc_taints_for_body(Some(body), &foreign_registry, analysis)
            .is_none()
    );
    let mut missing_analysis = analysis.clone();
    missing_analysis.global_instance_class_owner = None;
    assert!(
        unit.original_interproc_taints_for_body(Some(body), registry, &missing_analysis)
            .is_none()
    );
}

#[test]
fn original_module_taint_replay_preserves_receivers_and_refuses_foreign_owners() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tk").default_context_registry();
    let registry = context.commands();
    let profile = tcl_registry::model::ingress::resolve_environment("tk").unit_profile();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let source = "entry .e; proc danger {} {set value [.e get]; exec $value}";
    let build = |source| {
        CompilationUnit::build_with_analysis_input(
            source,
            UnitBuildOptions {
                registry,
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        )
    };
    let unit = build(source);
    let body = *unit
        .ir_module
        .original_source_procedure_bodies()
        .next()
        .unwrap()
        .0;
    let direct = unit.clone().with_interprocedural(registry, Some(profile));
    let analysis = direct.interproc.as_ref().unwrap();
    assert!(analysis.global_instance_classes[&body].contains_key(".e"));
    let expected = direct.function_for_body(Some(body)).unwrap();
    assert!(expected.taints.values().any(|taint| taint.is_tainted()));
    assert_eq!(
        unit.original_interproc_taints_for_body(Some(body), registry, analysis),
        Some(expected.taints.as_ref().clone()),
    );
    let mut observed = 0;
    let memoised = unit.clone().with_interprocedural_memoized(
        registry,
        Some(profile),
        &mut |actual_body, _, _| {
            assert_eq!(actual_body, body);
            observed += 1;
            None
        },
    );
    assert_eq!(observed, 1);
    assert_eq!(memoised.top_level.taints, direct.top_level.taints);
    assert_eq!(
        memoised.function_for_body(Some(body)).unwrap().taints,
        expected.taints
    );

    let other = build("entry .other; proc danger {} {set value [.other get]; exec $value}");
    let foreign_body = *other
        .ir_module
        .original_source_procedure_bodies()
        .next()
        .unwrap()
        .0;
    assert!(
        unit.original_interproc_taints_for_body(Some(foreign_body), registry, analysis)
            .is_none()
    );
    let other = other.with_interprocedural(registry, Some(profile));
    assert!(
        unit.original_interproc_taints_for_body(
            Some(body),
            registry,
            other.interproc.as_ref().unwrap()
        )
        .is_none()
    );
    assert_original_taint_owner_withdrawals(&unit, registry, analysis, body, profile, source);
}

#[test]
fn module_realm_refusal_withholds_caller_seeds_and_retires_interprocedural_results() {
    // naming.source.retained-positioned-command-realm
    // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let source = "proc retained {x} {list $x}; retained VALUE";
    let options = UnitBuildOptions {
        registry,
        defer_top_level: false,
        config,
        dialect: Some(profile),
        external_call_sites: None,
        declared_commands: None,
    };
    let (module, cfg, _, prepared) =
        lower_and_build_cfg(source, options, (None, None), None, None, None, None);
    assert!(crate::realm::document_realm_bindings_for_module(&module, registry).is_some());
    let (actual, _, _) = resolve_unit_scope(&module, &cfg, Some(&prepared), options);
    assert_eq!(
        actual.get("::retained").unwrap().uniform_literal_at(0),
        Some("VALUE")
    );
    let mut missing = module.clone();
    missing.retained_source_bindings = None;
    let mut changed = module.clone();
    changed.source = tcl_lexer::SourceImage::document("proc other {} {};");
    let mut missing_snapshot = module;
    missing_snapshot.registry_snapshot = None;
    for refused in [missing, changed, missing_snapshot] {
        let (unknown, _, _) = resolve_unit_scope(&refused, &cfg, Some(&prepared), options);
        let caller = unknown.get("::retained").unwrap();
        assert!(caller.opaque_caller);
        assert_eq!(caller.uniform_literal_at(0), None);
    }
    let completed = CompilationUnit::build_for_profile(source, registry, false, profile)
        .with_interprocedural(registry, Some(profile));
    assert!(completed.interproc.is_some());
    let mut missing = completed.clone();
    missing.ir_module.retained_source_bindings = None;
    let ordinary = missing
        .clone()
        .with_interprocedural(registry, Some(profile));
    assert!(ordinary.interproc.is_none());
    let mut callbacks = 0;
    let mut callback = |_: crate::ir::ProcedureId, _: &str, _: &InterproceduralAnalysis| {
        callbacks += 1;
        None
    };
    let memoised = missing.with_interprocedural_memoized(registry, Some(profile), &mut callback);
    assert!(memoised.interproc.is_none());
    assert_eq!(callbacks, 0);
}
