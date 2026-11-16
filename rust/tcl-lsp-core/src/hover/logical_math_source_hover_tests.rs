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

#[test]
fn logical_math_hover_reports_conditional_headers_without_runtime_dispatch() {
    let source = "namespace eval ::tcl::mathfunc {proc custom {first} {return $first}; proc custom {second} {return $second}}; expr {custom([unknown])}";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
    let offset = u32::try_from(source.rfind("custom(").unwrap()).unwrap();
    let answer = math_function_hover(source, &analysis, offset)
        .expect("actual original source function metadata");
    assert!(answer.value.contains("first"));
    assert!(answer.value.contains("second"));
    assert!(answer.value.contains("Conditional source declaration"));
    assert!(answer.value.contains("observers remain unresolved"));
    assert!(!answer.value.contains("Dispatches to"));
    assert!(math_function_hover(&format!("{source} "), &analysis, offset).is_none());
    let native = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    assert!(math_function_hover(source, &native, offset).is_none());
}
