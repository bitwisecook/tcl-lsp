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

//! Loop argument and binding stages do not invent opaque callee arguments.

use tcl_compiler::analyser::Analyser;
use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
use tcl_compiler::ir::SyntheticMarker;
use tcl_core_types::DiagCode;

#[test]
fn declared_empty_loop_reads_keep_real_entry_and_unknown_writer_controls() {
    let cases: &[(&str, &str, &[&str])] = &[
        (
            "empty-loop-before-top-level-missing-read",
            "proc p {} {foreach x {} {}; puts $x}; foreach y {} {}; puts $y",
            &["$x", "$y"],
        ),
        (
            "empty-loop-before-independent-top-level-error",
            "proc p {} {foreach x {} {}; return $x}; error fail",
            &["$x"],
        ),
        (
            "nonempty-loop-is-bound",
            "proc p {} {foreach x {7} {}; return $x}; error fail",
            &[],
        ),
        (
            "prior-binding-survives-empty-loop",
            "proc p {} {set x 7; foreach x {} {}; return $x}; error fail",
            &[],
        ),
        (
            "formal-binding-survives-empty-loop",
            "proc p {x} {foreach x {} {}; return $x}; error fail",
            &[],
        ),
        (
            "unknown-prefix-retains-initialisation-possibility",
            "proc p {} {operation x; foreach x {} {}; return $x}; error fail",
            &[],
        ),
        (
            "by-name-writer-retains-binding",
            "proc operation {name} {upvar 1 $name cell; set cell 7}; proc p {} {operation x; foreach x {} {}; return $x}; error fail",
            &[],
        ),
        (
            "nested-opaque-word-effect-keeps-binding-possibility",
            "proc p {} {foreach y [operation x] {}; return $x}; error fail",
            &[],
        ),
        (
            "nested-by-name-writer-keeps-its-word-effects",
            "proc operation {} {upvar 1 x cell; set cell 7; return {}}; proc p {} {foreach y [operation] {}; return $x}; error fail",
            &[],
        ),
        (
            "global-alias-keeps-other-frame-possibility",
            "proc p {} {global x; foreach x {} {}; return $x}; error fail",
            &[],
        ),
        (
            "formal-also-passed-to-opaque-call",
            "proc p {x} {operation x; foreach x {} {}; return $x}; error fail",
            &[],
        ),
    ];
    let mut failures = Vec::new();
    for &(label, source, expected) in cases {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let analysis = Analyser::new().analyse(source, dialect);
            assert!(analysis.resolved_input.is_some(), "{label} under {dialect}");
            assert!(analysis.analysis_context_unavailable.is_none());
            let mut actual: Vec<_> = analysis
                .diagnostics
                .iter()
                .filter(|finding| finding.code == DiagCode::W210)
                .map(|finding| (finding.span.start(), finding.span.end()))
                .collect();
            actual.sort_unstable();
            let mut wanted: Vec<_> = expected
                .iter()
                .map(|&spelling| {
                    let start = u32::try_from(source.rfind(spelling).unwrap()).unwrap();
                    (start, start + u32::try_from(spelling.len()).unwrap())
                })
                .collect();
            wanted.sort_unstable();
            println!("EMPTY_LOOP {label} {dialect} actual={actual:?} wanted={wanted:?}");
            if actual != wanted {
                failures.push(format!(
                    "{label} under {dialect}: {actual:?}, expected {wanted:?}"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn conditional_empty_loop_guards_keep_original_and_later_read_spans() {
    let cases: &[(&str, &str, &[&str])] = &[
        (
            "exact-original",
            "proc p {} {\n    foreach x {} {}\n    if {[info exists x]} { puts yes }\n    puts $x\n}\nforeach y {} {}\nputs $y\n",
            &["$x", "$y"],
        ),
        (
            "guarded-only",
            "proc p {} {foreach x {} {}; if {[info exists x]} {puts $x}; return quiet}; error fail",
            &[],
        ),
        (
            "guarded-then-later-read",
            "proc p {} {foreach x {} {}; if {[info exists x]} {puts $x}; puts $x}; error fail",
            &["$x"],
        ),
        (
            "unsafe-or",
            "proc p {} {foreach x {} {}; if {[info exists x] || $x > 0} {puts yes}}; error fail",
            &["$x"],
        ),
        (
            "unsafe-negated-and",
            "proc p {} {foreach x {} {}; if {![info exists x] && $x > 0} {puts yes}}; error fail",
            &["$x"],
        ),
        (
            "global-query-does-not-guard-local-read",
            "set ::x 1; proc p {} {foreach x {} {}; if {[info exists ::x] && $x > 0} {puts yes}}; error fail",
            &["$x"],
        ),
        (
            "may-bound-conditional-guard",
            "proc p {c} {if {$c} {set x 7}; foreach x {} {}; if {[info exists x] && $x > 0} {puts yes}}; error fail",
            &[],
        ),
        (
            "may-bound-conditional-unsafe-or",
            "proc p {c} {if {$c} {set x 7}; foreach x {} {}; if {[info exists x] || $x > 0} {puts yes}}; error fail",
            &["$x"],
        ),
        (
            "may-bound-guard-and-later-read",
            "proc p {c} {if {$c} {set x 7}; foreach x {} {}; if {[info exists x] && $x > 0} {puts yes}; puts $x}; error fail",
            &["$x"],
        ),
    ];
    let mut failures = Vec::new();
    for &(label, source, expected) in cases {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let analysis = Analyser::new().analyse(source, dialect);
            assert!(analysis.resolved_input.is_some(), "{label} under {dialect}");
            assert!(analysis.analysis_context_unavailable.is_none());
            let mut actual: Vec<_> = analysis
                .diagnostics
                .iter()
                .filter(|finding| finding.code == DiagCode::W210)
                .map(|finding| (finding.span.start(), finding.span.end()))
                .collect();
            actual.sort_unstable();
            let mut wanted: Vec<_> = expected
                .iter()
                .map(|&spelling| {
                    let start = u32::try_from(source.rfind(spelling).unwrap()).unwrap();
                    (start, start + u32::try_from(spelling.len()).unwrap())
                })
                .collect();
            wanted.sort_unstable();
            println!("EMPTY_LOOP {label} {dialect} actual={actual:?} wanted={wanted:?}");
            if actual != wanted {
                failures.push(format!(
                    "{label} under {dialect}: {actual:?}, expected {wanted:?}"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn opaque_callee_names_exclude_actual_loop_stages_and_keep_real_unknown_calls() {
    for (source, expected) in [
        ("proc p {} {foreach x {} {}; puts $x}; error fail", false),
        (
            "proc p {} {operation sentinel; foreach x {} {}; puts $x}; error fail",
            true,
        ),
    ] {
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(source, "tcl8.6");
        analyser.result = analysis;
        let input = analyser.resolved_analysis_input();
        let settings = analyser.source_compilation_settings();
        let registry = input.borrowed_context_registry().commands();
        let profile = input.unit_profile();
        let unit = CompilationUnit::build_with_analysis_input_and_source_settings(
            source,
            UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
            &settings,
        )
        .with_interprocedural(registry, Some(profile));
        let mut arguments = std::collections::HashSet::new();
        let mut unseen_arguments = std::collections::HashSet::new();
        let mut evaluated_arguments = 0;
        let mut iteration_bindings = 0;
        let mut residual_wrappers = 0;
        for function in unit.analysable_functions() {
            arguments.extend(
                tcl_compiler::interprocedural::collect_positioned_opaque_callee_name_args(
                    &function.cfg,
                    &|_| false,
                ),
            );
            unseen_arguments.extend(
                tcl_compiler::interprocedural::collect_positioned_opaque_callee_name_args(
                    &function.cfg,
                    &|_| true,
                ),
            );
            for block in function.cfg.blocks.values() {
                for statement in &block.statements {
                    let Some(tokens) = statement.tokens() else {
                        continue;
                    };
                    match tokens.synthetic {
                        Some(SyntheticMarker::EvaluatedArguments) => {
                            evaluated_arguments += 1;
                            assert!(tokens.evaluates_words());
                            assert!(!tokens.may_dispatch_source_command());
                        }
                        Some(SyntheticMarker::IterationBindings(_)) => {
                            iteration_bindings += 1;
                            assert!(!tokens.may_dispatch_source_command());
                        }
                        Some(SyntheticMarker::EvaluatedWrapper) => {
                            residual_wrappers += 1;
                            assert!(tokens.may_dispatch_source_command());
                            assert!(!tokens.evaluates_words());
                        }
                        _ => {}
                    }
                }
            }
        }
        assert!(evaluated_arguments > 0);
        assert_eq!(iteration_bindings > 0, !expected);
        assert_eq!(residual_wrappers > 0, expected);
        assert_eq!(arguments.contains("x"), expected, "{source}: {arguments:?}");
        assert_eq!(
            arguments.contains("sentinel"),
            expected,
            "{source}: {arguments:?}"
        );
        assert_eq!(
            unseen_arguments.contains("sentinel"),
            expected,
            "{source}: {unseen_arguments:?}"
        );
        assert!(
            !unseen_arguments.contains("x"),
            "{source}: {unseen_arguments:?}"
        );
        assert!(unit.ir_module.source_entry.native_entry.is_none());
    }
}
