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

//! A braced argument a proved procedure runs as a script is analysed in the
//! frame the procedure's contract names. Each wrapper is one measured shape
//! from `script-parameters-design.md` (tclsh 8.4.20 to 9.1.0 agree).

use crate::analyser::types::Scope;
use tcl_core_types::DiagCode;

/// `c05`'s wrapper: the script runs in the caller's frame inside `catch`.
const WRAPPER: &str = "proc t {label script} {if {[catch {uplevel 1 $script} r]} {return \"ERR $r\"} else {return $r}}\n";

fn analyse(source: &str) -> crate::analyser::AnalysisResult {
    crate::analyser::Analyser::new().analyse(source, "tcl8.6")
}

fn reports(result: &crate::analyser::AnalysisResult, code: DiagCode, line: &str) -> bool {
    result
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == code && diagnostic.message.contains(line))
}

fn script_scope(root: &Scope) -> Option<&Scope> {
    root.children
        .iter()
        .find(|scope| scope.name.starts_with("script@"))
}

#[test]
fn a_caller_frame_argument_reports_an_unknown_subcommand() {
    let result = analyse(&format!("{WRAPPER}t a {{string frobnicate x}}\n"));
    assert!(
        reports(&result, DiagCode::W001, "frobnicate"),
        "{:?}",
        result.diagnostics
    );
}

#[test]
fn a_caller_frame_argument_reports_a_bad_option() {
    let result = analyse(&format!("{WRAPPER}t a {{close stdout sideways}}\n"));
    assert!(
        reports(&result, DiagCode::W127, "sideways"),
        "{:?}",
        result.diagnostics
    );
}

#[test]
fn a_caller_frame_argument_writes_the_calling_frame() {
    let result = analyse(&format!("{WRAPPER}t a {{set x 1}}\n"));
    assert!(result.global_scope.variables.contains_key("x"));
}

/// `uplevel #0 $s`: the script sets `::z`.
#[test]
fn a_global_argument_writes_the_global_frame() {
    let result = analyse("proc g {s} {uplevel #0 $s}\ng {set z 3}\n");
    assert!(result.global_scope.variables.contains_key("z"));
}

/// `eval $s`: the script runs in `e`'s own frame, without the caller's `w`.
#[test]
fn an_own_frame_argument_neither_writes_nor_sees_the_caller() {
    let result = analyse("proc e {s} {eval $s}\nset w 1\ne {set y $w}\n");
    assert!(!result.global_scope.variables.contains_key("y"));
    let script = script_scope(&result.global_scope).expect("the script's own activation");
    assert!(script.variables.contains_key("y"));
    assert!(!script.variables.contains_key("w"));
}

#[test]
fn an_argument_the_procedure_does_not_run_is_left_alone() {
    for source in [
        "proc n {label script} {return $script}\nn a {string frobnicate x}\n".to_owned(),
        format!("{WRAPPER}set s {{string frobnicate x}}\nt a $s\n"),
    ] {
        assert!(
            !reports(&analyse(&source), DiagCode::W001, "frobnicate"),
            "{source}"
        );
    }
}
