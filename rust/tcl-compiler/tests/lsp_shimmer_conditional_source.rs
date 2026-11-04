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
        let matching: Vec<_> = found
            .iter()
            .filter(|warning| warning.code == DiagCode::S100 && warning.variable == "n")
            .collect();
        assert_eq!(matching.len(), 1, "{version:?}: {found:?}");
        let warning = matching[0];
        assert_eq!(
            (warning.from_type, warning.to_type),
            (TclType::Int, TclType::List)
        );
        assert_eq!(warning.span.start() as usize, source.rfind("$n").unwrap());
        assert_eq!(warning.span.end() as usize, source.rfind("$n").unwrap() + 2);
    }
}

#[test]
fn dynamic_integer_result_is_converted_by_literal_index_one() {
    let source = "set s abc; set n [string length $s]; lindex $n 1";
    for version in tcl_dialect::TclVersion::ALL {
        let found = warnings_for(source, version.dialect_profile_name());
        let matching: Vec<_> = found
            .iter()
            .filter(|warning| warning.code == DiagCode::S100 && warning.variable == "n")
            .collect();
        assert_eq!(matching.len(), 1, "{version:?}: {found:?}");
        let warning = matching[0];
        assert_eq!(
            (warning.from_type, warning.to_type),
            (TclType::Int, TclType::List)
        );
        assert_eq!(warning.span.start() as usize, source.rfind("$n").unwrap());
        assert_eq!(warning.span.end() as usize, source.rfind("$n").unwrap() + 2);
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

#[test]
fn fixed_count_dependent_increment_and_list_index_report_thunking() {
    let source = "set x 0; foreach i {1 2 3} {incr x; lindex $x 0}";
    for version in tcl_dialect::TclVersion::ALL {
        with_unit(
            source,
            version.dialect_profile_name(),
            |unit, registry, profile| {
                let warnings = tcl_compiler::shimmer::find_thunking_warnings_for_cu(unit, registry);
                let matching: Vec<_> = warnings
                    .iter()
                    .filter(|warning| warning.code == DiagCode::S102 && warning.variable == "x")
                    .collect();
                assert_eq!(matching.len(), 1, "{version:?}: {warnings:?}");
                let warning = matching[0];
                assert_eq!(
                    (warning.type_a, warning.type_b),
                    (TclType::Int, TclType::List)
                );
                assert_eq!(warning.span.start() as usize, source.rfind("$x").unwrap());
                let checks =
                    tcl_compiler::compiler_checks::run_all_checks(unit, registry, Some(profile));
                let diagnostics: Vec<_> = checks
                    .iter()
                    .filter(|diagnostic| diagnostic.code == DiagCode::S102)
                    .collect();
                assert_eq!(diagnostics.len(), 1, "{version:?}: {checks:?}");
                assert_eq!(diagnostics[0].span, warning.span);
                assert!(
                    !checks
                        .iter()
                        .any(|diagnostic| diagnostic.code == DiagCode::S100
                            && diagnostic.span == warning.span),
                    "a proved cycle must not duplicate its own single-conversion diagnostic"
                );
                assert_eq!(diagnostics[0].message, warning.message);
                assert!(diagnostics[0].replacement.is_none() && diagnostics[0].fixes.is_empty());
                assert!(
                    diagnostics[0]
                        .source_context
                        .as_ref()
                        .is_some_and(|context| !context.allows_context_action_subject())
                );
            },
        );
    }
}

#[test]
fn overwritten_or_large_valid_seed_still_reports_carried_conversion() {
    for source in [
        "set x abc; set x 0; foreach i {1 2 3} {incr x; lindex $x 0}",
        "set x 9223372036854775807; foreach i {1 2 3} {incr x; lindex $x 0}",
    ] {
        with_unit(source, "tcl8.6", |unit, registry, _| {
            let warnings = tcl_compiler::shimmer::find_thunking_warnings_for_cu(unit, registry);
            assert_eq!(
                warnings
                    .iter()
                    .filter(|warning| warning.code == DiagCode::S102)
                    .count(),
                1,
                "{source}: {warnings:?}"
            );
        });
    }
}

#[test]
fn finite_loop_without_a_carried_conversion_does_not_report_thunking() {
    for source in [
        "set x abc; foreach i {1 2 3} {incr x; lindex $x 0}",
        "set a abc; set x $a; foreach i {1 2 3} {incr x; lindex $x 0}",
        "set x Inf; foreach i {1 2 3} {incr x; lindex $x 0}",
        "set x NaN; foreach i {1 2 3} {incr x; lindex $x 0}",
        "set x {1 2}; foreach i {1 2 3} {incr x; lindex $x 0}",
        "set x 0; foreach i {} {incr x; lindex $x 0}",
        "set x 0; foreach i {1} {incr x; lindex $x 0}",
        "set x 0; foreach i $unknown {incr x; lindex $x 0}",
        "set x 0; foreach x {1 2 3} {incr x; lindex $x 0}",
        "set x 0; foreach i {1 2 3} {set x 0; incr x; lindex $x 0}",
        "set x 0; foreach i {1 2 3} {incr y; lindex $x 0}",
        "set x 0; foreach i {1 2 3} {incr x; lindex $x 0; proc incr args {}}",
        "proc observe args {}; set x 0; foreach i {1 2 3} {incr x; lindex $x 0; trace add variable x write observe}",
        "set x 0; foreach i {1 2 3} {lindex $x 0}",
        "set x 0; foreach i {1 2 3} {break; incr x; lindex $x 0}",
        "set x 0; foreach i {1 2 3} {continue; incr x; lindex $x 0}",
        "set x 0; foreach i {1 2 3} {if {$i == 2} {return}; incr x; lindex $x 0}",
        "proc incr args {}; set x 0; foreach i {1 2 3} {incr x; lindex $x 0}",
        "rename incr saved; set x 0; foreach i {1 2 3} {incr x; lindex $x 0}",
        "proc observe args {}; set x 0; trace add variable x write observe; foreach i {1 2 3} {incr x; lindex $x 0}",
    ] {
        with_unit(source, "tcl8.6", |unit, registry, _| {
            let warnings = tcl_compiler::shimmer::find_thunking_warnings_for_cu(unit, registry);
            assert!(
                !warnings
                    .iter()
                    .any(|warning| warning.code == DiagCode::S102),
                "{source}: {warnings:?}"
            );
        });
    }
}

#[test]
fn reported_conditional_list_accumulators_remain_quiet() {
    for source in [
        "set ys {}; set pts {}; foreach pt {{1 2} {3 4} {5 6}} {set y [lindex $pt 1]; if {$y > 2} {lappend ys $y; lappend pts $pt}}",
        "set ys {}; set pts {}; foreach pt {{1 2} {3 4} {5 6}} {set y [lindex $pt 1]; lappend ys $y; lappend pts $pt}",
        "set fontSizes {10.0 12.0 16.0 24.0}; foreach size $fontSizes {puts $size}",
    ] {
        with_unit(source, "tcl8.6", |unit, registry, profile| {
            let warnings = tcl_compiler::shimmer::find_thunking_warnings_for_cu(unit, registry);
            assert!(
                !warnings
                    .iter()
                    .any(|warning| warning.code == DiagCode::S102),
                "{source}: {warnings:?}"
            );
            let diagnostics =
                tcl_compiler::compiler_checks::run_all_checks(unit, registry, Some(profile));
            assert!(
                !diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == DiagCode::S102),
                "{source}: {diagnostics:?}"
            );
        });
    }
}
