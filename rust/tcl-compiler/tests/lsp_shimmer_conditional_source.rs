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
use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
use tcl_compiler::shimmer::{ShimmerWarning, find_shimmer_warnings_for_cu};
use tcl_core_types::DiagCode;
use tcl_registry::TclType;

fn warnings(source: &str) -> Vec<ShimmerWarning> {
    warnings_for(source, "tcl8.6")
}

fn warnings_for(source: &str, dialect: &str) -> Vec<ShimmerWarning> {
    with_unit(source, dialect, |unit, registry, _| {
        find_shimmer_warnings_for_cu(unit, registry)
    })
}

fn with_unit<T>(
    source: &str,
    dialect: &str,
    check: impl FnOnce(
        &CompilationUnit,
        &tcl_registry::CommandRegistry,
        &'static tcl_dialect::DialectProfile,
    ) -> T,
) -> T {
    let mut analyser = Analyser::new();
    let _analysis = analyser.analyse(source, dialect);
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
    check(&unit, registry, profile)
}

#[test]
fn compiler_checks_preserve_conditional_shimmer_without_action_authority() {
    for (source, expected) in [
        ("set s abc; set n [string length $s]; lindex $n 0", 1),
        ("set n {1 2}; lindex $n 0", 0),
    ] {
        with_unit(source, "tcl8.6", |unit, registry, profile| {
            let warnings = find_shimmer_warnings_for_cu(unit, registry);
            let checks =
                tcl_compiler::compiler_checks::run_all_checks(unit, registry, Some(profile));
            let source_warnings: Vec<_> = warnings
                .iter()
                .filter(|warning| warning.code == DiagCode::S100 && warning.variable == "n")
                .collect();
            assert_eq!(source_warnings.len(), expected, "{source}: {warnings:?}");
            let diagnostics: Vec<_> = checks
                .iter()
                .filter(|diagnostic| diagnostic.code == DiagCode::S100)
                .collect();
            assert_eq!(diagnostics.len(), expected, "{source}: {checks:?}");
            if let Some(warning) = source_warnings.first() {
                let diagnostic = diagnostics
                    .iter()
                    .find(|diagnostic| diagnostic.span == warning.span)
                    .expect("same original use span");
                assert_eq!(diagnostic.message, warning.message);
                assert!(diagnostic.replacement.is_none() && diagnostic.fixes.is_empty());
                assert!(
                    diagnostic
                        .source_context
                        .as_ref()
                        .is_some_and(|context| !context.allows_context_action_subject())
                );
            }
        });
    }
}

#[test]
fn dynamic_length_result_is_converted_by_list_index() {
    let source = "set s abc; set n [string length $s]; lindex $n 0";
    for version in tcl_dialect::TclVersion::ALL {
        let found = warnings_for(source, version.dialect_profile_name());
        let warning = found
            .iter()
            .find(|warning| warning.code == DiagCode::S100 && warning.variable == "n")
            .unwrap_or_else(|| panic!("{version:?}: {found:?}"));
        assert_eq!(
            (warning.from_type, warning.to_type),
            (TclType::Int, TclType::List)
        );
        assert_eq!(warning.span.start() as usize, source.rfind("$n").unwrap());
    }
}

#[test]
fn pure_and_pooled_literals_do_not_supply_integer_representation() {
    for source in [
        "set n [string length abc]; lindex $n 0",
        "set n {1 2}; lindex $n 0",
        "set n 123; lindex $n 0",
    ] {
        for version in tcl_dialect::TclVersion::ALL {
            let found = warnings_for(source, version.dialect_profile_name());
            assert!(
                !found.iter().any(|warning| warning.variable == "n"),
                "{version:?} {source}: {found:?}"
            );
        }
    }
}

#[test]
fn forwarded_copy_warns_before_later_scope_refusal() {
    let source = "set s abc; set n [string length $s]; set m $n; lindex $m 0; lindex $n 0";
    let found = warnings(source);
    let converted: Vec<_> = found
        .iter()
        .filter(|warning| {
            warning.code == DiagCode::S100 && (warning.variable == "m" || warning.variable == "n")
        })
        .collect();
    assert_eq!(converted.len(), 1, "{found:?}");
    assert_eq!(converted[0].variable, "m", "{found:?}");
    assert_eq!(
        (converted[0].from_type, converted[0].to_type),
        (TclType::Int, TclType::List)
    );
    assert_eq!(
        converted[0].span.start() as usize,
        source.find("$m").unwrap()
    );
}

#[test]
fn unknown_nested_result_does_not_reuse_inner_integer_result() {
    let source = "set s abc; set n [lindex [string length $s] 0]; lindex $n 0";
    let found = warnings(source);
    assert!(
        !found.iter().any(|warning| warning.variable == "n"),
        "{found:?}"
    );
}

#[test]
fn shadowed_handlers_and_traces_withdraw_source_assumptions() {
    for source in [
        "proc lindex args {return 0}; set s abc; set n [string length $s]; lindex $n 0",
        "rename lindex saved; set s abc; set n [string length $s]; lindex $n 0",
        "proc observe args {}; set s abc; trace add variable n write observe; set n [string length $s]; lindex $n 0",
    ] {
        let found = warnings(source);
        assert!(
            !found.iter().any(|warning| warning.variable == "n"),
            "{source}: {found:?}"
        );
    }
}
