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

//! Hexadecimal conversion advice and its actual editor actions.

use tcl_compiler::analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput};
use tcl_core_types::DiagCode;
use tcl_lexer::{LexerConfig, LineIndex, Utf16Col};
use tcl_lsp_core::code_actions::{CodeAction, DiagnosticEditSource, code_actions};
use tcl_lsp_core::definition::LspRange;
use tcl_lsp_core::diagnostic_policy::{Finding, Policy, Report};
use tcl_lsp_core::diagnostic_report::{DocumentSource, SourcePass, document_report_with_analysis};

const RGB_SOURCE: &str = "proc rgb {hex_clean} {\nset r [expr 0x[string range $hex_clean 0 1]]\nset g [expr 0x[string range $hex_clean 2 3]]\nset b [expr 0x[string range $hex_clean 4 5]]\nreturn [list $r $g $b]\n}\n";

fn analyse(source: &str, dialect: &str) -> AnalysisResult {
    let environment = tcl_registry::model::ingress::resolve_environment(dialect);
    let context = environment.default_context_registry();
    let profile = environment.analyser_profile();
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        LexerConfig::for_file_grammar(profile.grammar),
    );
    let analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse(source, dialect);
    assert!(analysis.analysis_context_unavailable.is_none());
    assert!(analysis.resolved_input.is_some());
    assert!(DiagnosticEditSource::for_analysis(source, &analysis).is_some());
    analysis
}

fn report(source: &str, analysis: &AnalysisResult) -> Report {
    let mut produced: Vec<_> = analysis
        .diagnostics
        .iter()
        .cloned()
        .map(Finding::from)
        .collect();
    let hints = tcl_lsp_core::diagnostic_report::brace_expr_hints(&produced);
    produced.extend(hints);
    let document = DocumentSource {
        text: source,
        analysis_text: source,
        decode: None,
        dialect: analysis.resolved_profile().unwrap(),
        pass: SourcePass::Tcl { line_length: 120 },
    };
    document_report_with_analysis(&document, produced, &Policy::unrestricted(), Some(analysis))
}

fn actions(source: &str, analysis: &AnalysisResult, report: &Report) -> Vec<CodeAction> {
    code_actions(
        source,
        LspRange {
            start_line: 0,
            start_character: 0,
            end_line: u32::try_from(source.lines().count()).unwrap(),
            end_character: 0,
        },
        Some(analysis),
        report,
    )
}

fn observe(tclsh: &std::path::Path, source: &str) -> tcl_test_support::ScriptOutcome {
    let word = tcl_codegen::codegen::helpers::tcl_list_element(source);
    let script =
        format!("set completion [catch {word} message]\nputs [list $completion $message]\n");
    tcl_test_support::run_script(tclsh, script.as_bytes()).unwrap()
}

fn jimsh() -> Option<tcl_test_support::Jimsh> {
    if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
        Some(tcl_test_support::require_jimsh().expect("required current Jim oracle"))
    } else {
        tcl_test_support::locate_jimsh().expect("validated Jim oracle")
    }
}

fn apply(source: &str, action: &CodeAction) -> String {
    let index = LineIndex::new(source);
    let mut edits: Vec<_> = action
        .edits
        .iter()
        .map(|edit| {
            let start = index.offset_at_utf16(
                edit.range.start_line,
                Utf16Col::new(edit.range.start_character),
                source,
            );
            let end = index.offset_at_utf16(
                edit.range.end_line,
                Utf16Col::new(edit.range.end_character),
                source,
            );
            (start as usize..end as usize, edit.new_text.as_str())
        })
        .collect();
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut output = source.to_owned();
    for (range, replacement) in edits {
        output.replace_range(range, replacement);
    }
    output
}

#[test]
fn recommendation_preserves_w100_and_o111() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let analysis = analyse(RGB_SOURCE, dialect);
        let report = report(RGB_SOURCE, &analysis);
        for code in [DiagCode::W100, DiagCode::O111] {
            assert_eq!(
                report
                    .shown()
                    .filter(|shown| shown.finding.code == code)
                    .count(),
                3,
                "{dialect}: {code}"
            );
        }
        assert_eq!(
            report
                .shown()
                .filter(|shown| shown.finding.code == DiagCode::W100
                    && shown.finding.message.contains("scan $hex %x var"))
                .count(),
            3,
            "{dialect}"
        );
    }
}

#[test]
fn dynamic_hex_recommendation_offers_no_invalid_brace_or_scan_edit() {
    let analysis = analyse(RGB_SOURCE, "tcl8.6");
    let report = report(RGB_SOURCE, &analysis);
    let current = DiagnosticEditSource::for_analysis(RGB_SOURCE, &analysis).unwrap();
    let warnings: Vec<_> = report
        .shown()
        .filter(|shown| shown.finding.code == DiagCode::W100)
        .collect();
    assert_eq!(warnings.len(), 3);
    assert!(
        warnings
            .iter()
            .all(|shown| current.matches_finding(shown.finding))
    );
    let offered = actions(RGB_SOURCE, &analysis, &report);
    let invalid = offered
        .iter()
        .filter(|action| {
            matches!(
                action.title.as_str(),
                "Wrap expression in braces" | "Brace expr for safety and performance"
            )
        })
        .count();
    assert_eq!(invalid, 0, "a braced 0x substitution is invalid Tcl");
    assert!(
        offered
            .iter()
            .flat_map(|action| &action.edits)
            .all(|edit| !edit.new_text.contains("scan "))
    );
}

#[test]
fn positive_and_negative_source_patterns() {
    for (expression, expected_warning, expected_scan) in [
        ("expr 0x$hex", 1, 1),
        ("expr 0X$hex", 1, 1),
        ("expr \"0x$hex\"", 1, 1),
        ("expr 0x[string range $hex 0 1]", 1, 1),
        ("expr $hex + 1", 1, 0),
        ("expr 0x$hex + 1", 1, 0),
        ("expr 0x$left$right", 1, 0),
        ("expr 0x[string range $hex 0 1]+1", 1, 0),
        ("expr -0x$hex", 1, 0),
        ("expr 0xff", 0, 0),
        ("expr {0x$hex}", 0, 0),
        ("$command 0x$hex", 0, 0),
    ] {
        let source = format!("proc f {{hex left right command}} {{ {expression} }}\n");
        let analysis = analyse(&source, "tcl8.6");
        let warnings: Vec<_> = analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::W100)
            .collect();
        assert_eq!(warnings.len(), expected_warning, "{expression}");
        assert_eq!(
            warnings
                .iter()
                .filter(|diagnostic| diagnostic.message.contains("scan $hex %x var"))
                .count(),
            expected_scan,
            "{expression}"
        );
    }
}

#[test]
fn ordinary_expression_brace_action_still_preserves_execution() {
    let source = "proc f {x} {return [expr $x + 1]}\nputs [f 5]\n";
    let analysis = analyse(source, "tcl8.6");
    let report = report(source, &analysis);
    let offered = actions(source, &analysis, &report);
    let brace = offered
        .iter()
        .find(|action| action.title == "Brace expr for safety and performance")
        .expect("ordinary arithmetic keeps its brace action");
    let rewritten = apply(source, brace);
    assert!(rewritten.contains("expr {$x + 1}"));
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = tcl_test_support::witness_tclsh(version) else {
            continue;
        };
        assert_eq!(
            observe(&tclsh.path, source),
            observe(&tclsh.path, &rewritten),
            "{version:?}"
        );
    }
}

#[test]
fn scan_example_preserves_valid_rgb_components() {
    let scan = "proc rgb {hex_clean} {\nscan [string range $hex_clean 0 1] %x r\nscan [string range $hex_clean 2 3] %x g\nscan [string range $hex_clean 4 5] %x b\nreturn [list $r $g $b]\n}\n";
    let jim = jimsh();
    for (colour, expected) in [
        ("000000", "0 0 0"),
        ("ffffff", "255 255 255"),
        ("aBcD09", "171 205 9"),
        ("000080", "0 0 128"),
    ] {
        let call = format!("puts [rgb {colour}]\n");
        let original = format!("{RGB_SOURCE}{call}");
        let recommended = format!("{scan}{call}");
        for version in tcl_dialect::TclVersion::ALL {
            let Some(tclsh) = tcl_test_support::witness_tclsh(version) else {
                continue;
            };
            let observed = observe(&tclsh.path, &original);
            assert_eq!(observed.stdout, format!("{expected}\n0 {{}}\n").as_bytes());
            assert_eq!(
                observed,
                observe(&tclsh.path, &recommended),
                "{version:?} {colour}"
            );
        }
        if let Some(jim) = &jim {
            let jimsh = &jim.path;
            let observed = observe(jimsh, &original);
            assert_eq!(observed.stdout, format!("{expected}\n0 {{}}\n").as_bytes());
            assert_eq!(observed, observe(jimsh, &recommended));
        }
    }
}

#[test]
fn a_user_command_named_expr_has_no_conversion_recommendation() {
    let source = "proc expr {value} {return $value}\nproc f {hex} {expr 0x$hex}\n";
    let analysis = analyse(source, "tcl8.6");
    assert!(
        analysis
            .diagnostics
            .iter()
            .all(|diagnostic| { !diagnostic.message.contains("scan $hex %x var") })
    );
}

#[test]
fn unknown_hex_text_does_not_justify_an_automatic_scan_rewrite() {
    let jim = jimsh();
    for value in ["-ff", "ff+1", "zz"] {
        let word = tcl_codegen::codegen::helpers::tcl_list_element(value);
        let original = format!("set hex {word}\nexpr 0x$hex\n");
        let scanned = format!("set hex {word}\nset value UNSET\nscan $hex %x value\nset value\n");
        for version in tcl_dialect::TclVersion::ALL {
            let Some(tclsh) = tcl_test_support::witness_tclsh(version) else {
                continue;
            };
            assert_ne!(
                observe(&tclsh.path, &original),
                observe(&tclsh.path, &scanned),
                "{version:?}: {value}"
            );
        }
        if let Some(jim) = &jim {
            let jimsh = &jim.path;
            assert_ne!(observe(jimsh, &original), observe(jimsh, &scanned));
        }
    }
}
