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
use tcl_compiler::analyser::Analyser;

fn parameter_hints(source: &str, analysis: &AnalysisResult) -> Vec<InlayHint> {
    parameter_hints_from(source, analysis, 0)
}

fn parameter_hints_from(source: &str, analysis: &AnalysisResult, start: usize) -> Vec<InlayHint> {
    assert!(source.is_ascii() && !source.contains('\n'));
    inlay_hints(
        source,
        crate::profile_for_dialect("tcl8.6"),
        LspRange {
            start_line: 0,
            start_character: u32::try_from(start).unwrap(),
            end_line: 100,
            end_character: 0,
        },
        Some(analysis),
        None,
        false,
        true,
    )
}

fn builtin_hints(source: &str, analysis: &AnalysisResult) -> Vec<InlayHint> {
    inlay_hints(
        source,
        crate::profile_for_dialect("tcl8.6"),
        LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 100,
            end_character: 0,
        },
        Some(analysis),
        Some(crate::registry_for_dialect("tcl8.6")),
        false,
        true,
    )
}

#[test]
fn original_parameter_hints_preserve_opaque_call_identity_and_source_currency() {
    let source = "proc p\\uD800 {first} {}; proc p\\uD801 {second} {}; p\\uD800 1; p\\uD801 2";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6").clone();
    analysis.all_procs.clear();
    analysis.global_scope.procs.clear();
    for invocation in &mut analysis.command_invocations {
        invocation.name = "counterfactual".to_owned();
    }
    // The requested viewport includes both tested calls. Definition
    // operands outside it have their own valid builtin synopsis hints.
    let start = source.find("; p\\uD800 1").unwrap() + 2;
    let hints = parameter_hints_from(source, &analysis, start);
    assert_eq!(
        hints
            .iter()
            .map(|hint| hint.label.as_str())
            .collect::<Vec<_>>(),
        ["first:", "second:"]
    );
    assert_eq!(
        parameter_hints(&format!("#{source}"), &analysis),
        [] as [crate::inlay_hints::InlayHint; 0]
    );
    for invocation in &mut analysis.command_invocations {
        invocation.original_lookup = None;
    }
    assert_eq!(
        parameter_hints_from(source, &analysis, start),
        [] as [crate::inlay_hints::InlayHint; 0]
    );
}

#[test]
fn original_alias_parameter_hints_map_captured_prefix_before_written_arguments() {
    let source =
        "proc target {captured written} {}; interp alias {} alias {} target FIXED; alias VALUE";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6").clone();
    analysis.all_procs.clear();
    let start = source.rfind("; alias VALUE").unwrap() + 2;
    let hints = parameter_hints_from(source, &analysis, start);
    assert_eq!(
        hints
            .iter()
            .map(|hint| hint.label.as_str())
            .collect::<Vec<_>>(),
        ["written:"]
    );
}

#[test]
fn original_builtin_alias_and_expansion_hints_keep_effective_slots_and_original_extents() {
    // Implementation contract: naming.core.original-hint-effective-origins
    // docs/design/analysis/name-resolution-proofs/core-original-hint-effective-origins.md
    let source = "interp alias {} slice {} string range CAPTURED; slice 1 2";
    let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
    let hints = builtin_hints(source, &analysis);
    let start = source.rfind("slice 1 2").unwrap();
    let hints: Vec<_> = hints
        .iter()
        .filter(|hint| hint.position_character >= u32::try_from(start).unwrap())
        .collect();
    assert_eq!(
        hints
            .iter()
            .map(|hint| hint.label.as_str())
            .collect::<Vec<_>>(),
        ["first:", "last:"]
    );
    assert_eq!(
        hints[0].position_character as usize,
        source.rfind("1 2").unwrap()
    );

    let source = "string range {*}{CAPTURED 1} 2";
    let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
    let hints = builtin_hints(source, &analysis);
    assert_eq!(
        hints
            .iter()
            .map(|hint| hint.label.as_str())
            .collect::<Vec<_>>(),
        ["string:", "first:", "last:"]
    );
    assert_eq!(
        hints
            .iter()
            .map(|hint| hint.position_character as usize)
            .collect::<Vec<_>>(),
        [
            source.find("CAPTURED").unwrap(),
            source.find("1}").unwrap(),
            source.rfind('2').unwrap()
        ]
    );
    let source = "set prefix {CAPTURED 1}; string range {*}$prefix 2";
    let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
    let hints = builtin_hints(source, &analysis);
    let hints: Vec<_> = hints
        .iter()
        .filter(|hint| hint.position_character as usize >= source.find("string range").unwrap())
        .collect();
    assert_eq!(
        hints
            .iter()
            .map(|hint| hint.label.as_str())
            .collect::<Vec<_>>(),
        ["last:"]
    );
    let source = "string range {*}$unknown 2";
    let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
    assert_eq!(
        builtin_hints(source, &analysis),
        [] as [crate::inlay_hints::InlayHint; 0]
    );
}

#[test]
fn original_format_alias_expansion_and_escape_hints_use_the_template_producer() {
    // Implementation contract: naming.core.original-format-hint-producer
    // docs/design/analysis/name-resolution-proofs/core-original-format-hint-producer.md
    let hints = |source: &str| {
        let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
        let registry = analysis.resolved_registry().unwrap();
        let unit = crate::original_invocation::source_compilation_unit(source, &analysis, registry)
            .unwrap();
        let mut out = Vec::new();
        collect_format_string_hints(
            source,
            &analysis,
            FormatHintContext {
                dialect: crate::profile_for_dialect("tcl8.6"),
                registry,
                unit: &unit,
            },
            LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 100,
                end_character: 0,
            },
            &LineIndex::new(source),
            &mut out,
        );
        out
    };
    let source = r#"interp alias {} fmt {} format; fmt "%d" 3"#;
    let out = hints(source);
    assert_eq!(
        out.iter()
            .map(|hint| hint.label.as_str())
            .collect::<Vec<_>>(),
        ["int"]
    );
    assert_eq!(
        out[0].position_character as usize,
        source.find("%d").unwrap() + 2
    );
    let source = "format {*}{%d 3}";
    let out = hints(source);
    assert_eq!(
        out.iter()
            .map(|hint| hint.label.as_str())
            .collect::<Vec<_>>(),
        ["int"]
    );
    assert_eq!(
        out[0].position_character as usize,
        source.find("%d").unwrap() + 2
    );
    let source = r"format \x25d 3";
    let out = hints(source);
    assert_eq!(
        out.iter()
            .map(|hint| hint.label.as_str())
            .collect::<Vec<_>>(),
        ["int"]
    );
    assert_eq!(
        out[0].position_character as usize,
        source.find(r"\x25d").unwrap() + 5
    );
    assert_eq!(
        hints(r#"interp alias {} fmt {} format "%d"; fmt 3"#),
        [] as [crate::inlay_hints::InlayHint; 0]
    );
    assert_eq!(
        hints("format {*}$unknown 3"),
        [] as [crate::inlay_hints::InlayHint; 0]
    );
}

#[test]
fn original_builtin_hints_use_retained_descriptor_and_do_not_borrow_shadowed_prototype() {
    // Implementation contract: naming.core.original-hint-effective-origins
    // docs/design/analysis/name-resolution-proofs/core-original-hint-effective-origins.md
    let source = "set value 1";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6").clone();
    let hints = builtin_hints(source, &analysis);
    assert!(
        hints.iter().any(|hint| hint.label == "varName:"),
        "{hints:?}"
    );
    for invocation in &mut analysis.command_invocations {
        invocation.name = "puts".to_owned();
    }
    assert_eq!(builtin_hints(source, &analysis), hints);
    let shadow = "proc set {other} {}; set value";
    let analysis = Analyser::new().analyse(shadow, "tcl8.6").clone();
    assert_eq!(
        parameter_hints_from(shadow, &analysis, shadow.rfind("; set value").unwrap() + 2)
            .iter()
            .map(|hint| hint.label.as_str())
            .collect::<Vec<_>>(),
        ["other:"]
    );
}
