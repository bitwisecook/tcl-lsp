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

fn input(
    actual: &str,
    store: Arc<tcl_registry::CommandRegistry>,
) -> crate::analyser::ResolvedAnalysisInput {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let context = tcl_registry::model::resolve_environment(actual)
        .default_context_registry()
        .with_command_store(store);
    crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::new(context),
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    )
}

fn fixture(
    source: &str,
    input: &crate::analyser::ResolvedAnalysisInput,
    native: &Arc<tcl_runtime_api::NativeCompilationEntry>,
) -> (Analyser, crate::compilation_unit::CompilationUnit) {
    let entry = Arc::new(crate::command_binding::SourceAnalysisEntry {
        metadata_context:
            crate::registry_invocation::OwnedInvocationMetadataContext::SuppliedSource(Box::new(
                input.clone(),
            )),
        native_entry: Some(Arc::clone(native)),
        ..Default::default()
    });
    let mut analyser = Analyser::new()
        .with_resolved_input(input.clone())
        .with_source_analysis_entry(Arc::clone(&entry));
    analyser.analyse_and_retain_result_for_test(source, input.analyser_profile().name);
    let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry: input.borrowed_context_registry().commands(),
            defer_top_level: false,
            config: input.lexer_config(),
            dialect: Some(input.unit_profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        input,
    );
    (analyser, unit)
}

fn emitted(
    analyser: &mut Analyser,
    unit: &crate::compilation_unit::CompilationUnit,
) -> Vec<crate::analyser::Diagnostic> {
    analyser
        .result
        .diagnostics
        .retain(|row| row.code != DiagCode::W250);
    analyser.emit_abstract_instantiation_diagnostics(unit);
    analyser
        .result
        .diagnostics
        .iter()
        .filter(|row| row.code == DiagCode::W250)
        .cloned()
        .collect()
}

#[test]
fn original_abstract_factory_advice_keeps_actual_context_alias_and_nested_source() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Actual registration entry remains separate from conditional factory advice.
    let catalogue = tcl_registry::model::resolve_environment("tcl9.0").default_context_registry();
    let input = input("tcl9.0", Arc::clone(catalogue.commands()));
    let (_owner, native) =
        crate::environment_ingress::captured_native_entry_with_owner(input.unit_profile());
    let native = Arc::new(native);
    for tail in [
        "Base new",
        "Base create object",
        "set result [Base new]",
        "interp alias {} makeAbstract {} Base new; makeAbstract",
    ] {
        let source = format!("oo::abstract create Base {{}}; {tail}");
        let (mut analyser, unit) = fixture(&source, &input, &native);
        let rows = emitted(&mut analyser, &unit);
        assert_eq!(rows.len(), 1, "{tail}: {rows:?}");
        assert!(source.get(rows[0].span.as_range()).is_some());
        analyser.registry = Some(Arc::new(tcl_registry::CommandRegistry::build_default()));
        assert_eq!(emitted(&mut analyser, &unit), rows);
    }
}

#[test]
fn original_abstract_factory_advice_refuses_changed_availability_and_source_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Metadata never replaces the independently retained original class/factory owner.
    let source = "oo::abstract create Base {}; Base new";
    let catalogue = tcl_registry::model::resolve_environment("tcl9.0").default_context_registry();
    let current = input("tcl9.0", Arc::clone(catalogue.commands()));
    let older = input("tcl8.6", Arc::clone(catalogue.commands()));
    assert!(Arc::ptr_eq(
        current.borrowed_context_registry().commands(),
        older.borrowed_context_registry().commands()
    ));
    let (_owner, native) =
        crate::environment_ingress::captured_native_entry_with_owner(current.unit_profile());
    let native = Arc::new(native);
    let (mut analyser, unit) = fixture(source, &current, &native);
    assert_eq!(emitted(&mut analyser, &unit).len(), 1);
    let (mut unavailable, older_unit) = fixture(source, &older, &native);
    assert_eq!(
        emitted(&mut unavailable, &older_unit),
        [] as [crate::analyser::types::Diagnostic; 0]
    );
    assert_eq!(
        emitted(&mut analyser, &older_unit),
        [] as [crate::analyser::types::Diagnostic; 0]
    );
    let mut missing = unit.clone();
    missing.ir_module.source_metadata_input = None;
    assert_eq!(
        emitted(&mut analyser, &missing),
        [] as [crate::analyser::types::Diagnostic; 0]
    );
    let mut missing = unit.clone();
    missing.ir_module.retained_source_bindings = None;
    assert_eq!(
        emitted(&mut analyser, &missing),
        [] as [crate::analyser::types::Diagnostic; 0]
    );
    let mut changed = unit.clone();
    changed.ir_module.lexer_config.strict_quoting ^= true;
    assert_eq!(
        emitted(&mut analyser, &changed),
        [] as [crate::analyser::types::Diagnostic; 0]
    );
    let mut changed = unit.clone();
    changed.ir_module.source = tcl_lexer::SourceImage::document(&format!("{source} # changed"));
    assert_eq!(
        emitted(&mut analyser, &changed),
        [] as [crate::analyser::types::Diagnostic; 0]
    );
    let foreign = input(
        "tcl9.0",
        Arc::new(tcl_registry::CommandRegistry::build_default()),
    );
    analyser.result.resolved_input = Some(foreign);
    assert_eq!(
        emitted(&mut analyser, &unit),
        [] as [crate::analyser::types::Diagnostic; 0]
    );
    analyser.result.resolved_input = None;
    assert_eq!(
        emitted(&mut analyser, &unit),
        [] as [crate::analyser::types::Diagnostic; 0]
    );
}

#[test]
fn original_abstract_factory_advice_preserves_known_class_lifecycle_barriers() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Current source incarnation and factory dependencies remain independent.
    let catalogue = tcl_registry::model::resolve_environment("tcl9.0").default_context_registry();
    let input = input("tcl9.0", Arc::clone(catalogue.commands()));
    let (_owner, native) =
        crate::environment_ingress::captured_native_entry_with_owner(input.unit_profile());
    let native = Arc::new(native);
    for mutation in [
        "rename Base Backup; proc Base args {return custom}",
        "oo::objdefine Base method new {} {return custom}",
        "oo::objdefine Base export new",
    ] {
        let source = format!("oo::abstract create Base {{}}; {mutation}; Base new");
        let (mut analyser, unit) = fixture(&source, &input, &native);
        assert!(emitted(&mut analyser, &unit).is_empty(), "{mutation}");
    }
}
