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

use tcl_compiler::analyser::Analyser;
use tcl_core_types::DiagCode;

fn with_unit<T>(
    source: &str,
    run: impl FnOnce(
        &tcl_compiler::compilation_unit::CompilationUnit,
        &tcl_registry::CommandRegistry,
        &'static tcl_dialect::DialectProfile,
    ) -> T,
) -> T {
    use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
    use tcl_compiler::shimmer::ShimmerContext;
    let mut analyser = Analyser::new();
    let _analysis = analyser.analyse(source, "tcl8.6");
    let input = analyser.resolved_analysis_input();
    let settings = analyser.source_compilation_settings();
    let registry = input.borrowed_context_registry().commands();
    let profile = input.unit_profile();
    let options = UnitBuildOptions {
        registry,
        defer_top_level: false,
        config: input.lexer_config(),
        dialect: Some(profile),
        external_call_sites: None,
        declared_commands: None,
    };
    let unit = CompilationUnit::build_with_analysis_input_and_source_settings(
        source, options, None, &input, &settings,
    )
    .with_interprocedural(registry, Some(profile));
    assert!(unit.analysable_functions().next().is_some());
    assert!(
        unit.analysable_functions()
            .all(|f| ShimmerContext::for_function(f, registry).is_some())
    );
    run(&unit, registry, profile)
}

fn taint(source: &str) -> Vec<String> {
    with_unit(source, |unit, registry, profile| {
        tcl_compiler::taint::find_taint_warnings_for_cu(unit, registry, Some(profile))
            .into_iter()
            .filter(|w| w.code == DiagCode::T100)
            .map(|w| w.variable)
            .collect()
    })
}

#[test]
fn issue_2506_boolean_comparison_taint() {
    let source = "set line [gets stdin]\nset trimmed [string trimleft $line]\nset is_empty [expr {$trimmed eq \"\"}]\nset is_comment [expr {!$is_empty && [string index $trimmed 0] eq \"#\"}]\nset is_code [expr {!$is_empty && !$is_comment}]";
    let warnings = taint(source);
    assert!(
        !warnings
            .iter()
            .any(|v| v == "is_empty" || v == "is_comment"),
        "{warnings:?}"
    );
    let raw_control = taint("set raw [gets stdin]\nexpr {$raw + 1}");
    assert!(
        raw_control.iter().any(|v| v == "raw"),
        "taint analysis failed raw-input control: {raw_control:?}"
    );
    let control = taint("set n [gets stdin]\nset m [expr {$n + 1}]\nif {$m} {puts y}");
    assert!(
        control.iter().any(|v| v == "m"),
        "taint analysis failed positive control: {control:?}"
    );
}
