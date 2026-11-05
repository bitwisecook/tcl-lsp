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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original external binding reports, with complete linked source snapshots.

use tcl_compiler::analyser::Analyser;
use tcl_compiler::analyser::types::Diagnostic;
use tcl_core_types::DiagCode;

const DIALECTS: [&str; 5] = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"];

fn findings(label: &str, source: &str, dialect: &str) -> Vec<Diagnostic> {
    let diagnostics = Analyser::new().analyse(source, dialect).diagnostics;
    let compact: Vec<_> = diagnostics
        .iter()
        .map(|finding| {
            serde_json::json!({
                "code": finding.code.to_string(),
                "start": finding.span.start(),
                "end": finding.span.end(),
                "message": finding.message,
            })
        })
        .collect();
    println!(
        "BINDING_EXTERNAL {}",
        serde_json::json!({
            "input": label,
            "dialect": dialect,
            "bytes": source.len(),
            "diagnostics": compact,
        })
    );
    diagnostics
}

fn absent(diagnostics: &[Diagnostic], code: DiagCode, name: &str, label: &str, dialect: &str) {
    assert!(
        diagnostics
            .iter()
            .all(|finding| finding.code != code || !finding.message.contains(name)),
        "{label} under {dialect}: {code} for {name}; see BINDING_EXTERNAL output"
    );
}

#[test]
fn binding_diagnostics_have_positive_controls() {
    for dialect in DIALECTS {
        let missing_variable = findings(
            "W210-positive-control",
            "proc missingVariable {} {puts $__binding_missing}; missingVariable\n",
            dialect,
        );
        assert!(missing_variable.iter().any(|finding| {
            finding.code == DiagCode::W210 && finding.message.contains("__binding_missing")
        }));
        let missing_interpreter = findings(
            "W140-positive-control",
            "interp eval vectorCoreChild {set child}\n",
            dialect,
        );
        assert!(missing_interpreter.iter().any(|finding| {
            finding.code == DiagCode::W140 && finding.message.contains("vectorCoreChild")
        }));
    }
}

#[test]
fn issue_2286_tcltest_setup_body_and_cleanup() {
    let linked = include_str!("../data/lsp-external-bindings/2286-linked.test");
    let snippets = [
        include_str!("../data/lsp-external-bindings/2286-issue-1.tcl"),
        include_str!("../data/lsp-external-bindings/2286-issue-2.tcl"),
        include_str!("../data/lsp-external-bindings/2286-issue-3.tcl"),
        include_str!("../data/lsp-external-bindings/2286-issue-4.tcl"),
    ];
    for dialect in DIALECTS {
        let diagnostics = findings("2286-linked.test", linked, dialect);
        for name in ["'optimizer'", "'pdata'", "'par0'", "'par1'", "'pars'"] {
            absent(
                &diagnostics,
                DiagCode::W210,
                name,
                "2286-linked.test",
                dialect,
            );
        }
        for (index, snippet) in snippets.iter().enumerate() {
            let source =
                format!("package require tcltest\nnamespace import -force ::tcltest::*\n{snippet}");
            let label = format!("2286-issue-{}-with-tcltest-context", index + 1);
            let diagnostics = findings(&label, &source, dialect);
            assert!(
                diagnostics
                    .iter()
                    .all(|finding| finding.code != DiagCode::W210)
            );
        }
    }
}

#[test]
fn issue_2289_child_interpreter_created_in_new() {
    let linked = include_str!("../data/lsp-external-bindings/2289-linked.test");
    let creator = include_str!("../data/lsp-external-bindings/2289-issue-1.tcl");
    let called = format!("{creator}\nNew\ninterp eval vectorCoreChild {{set value VALUE}}\n");
    for dialect in DIALECTS {
        for (label, source) in [
            ("2289-linked.test", linked),
            ("2289-creator-called", &called),
        ] {
            let diagnostics = findings(label, source, dialect);
            absent(
                &diagnostics,
                DiagCode::W140,
                "vectorCoreChild",
                label,
                dialect,
            );
        }
    }
}

#[test]
fn issue_2290_current_complete_tclmeasure_source() {
    let source = include_str!("../data/lsp-external-bindings/2290-linked.tcl");
    for dialect in DIALECTS {
        let diagnostics = findings("2290-linked.tcl", source, dialect);
        for name in ["'data'", "'type'"] {
            absent(
                &diagnostics,
                DiagCode::W210,
                name,
                "2290-linked.tcl",
                dialect,
            );
        }
    }
}

#[test]
fn issue_2501_original_lassign_loop() {
    let source = include_str!("../data/lsp-external-bindings/2501-issue-1.tcl");
    for dialect in DIALECTS {
        let diagnostics = findings("2501-issue-1.tcl", source, dialect);
        absent(
            &diagnostics,
            DiagCode::W210,
            "'start'",
            "2501-issue-1.tcl",
            dialect,
        );
    }
}

#[test]
fn issue_2502_original_dictionary_unpack() {
    let source = include_str!("../data/lsp-external-bindings/2502-issue-1.tcl");
    for dialect in DIALECTS {
        let diagnostics = findings("2502-issue-1.tcl", source, dialect);
        for name in ["'radius'", "'bg'"] {
            absent(
                &diagnostics,
                DiagCode::W210,
                name,
                "2502-issue-1.tcl",
                dialect,
            );
        }
    }
}
