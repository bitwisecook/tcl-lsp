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

fn codes(source: &str, code: DiagCode) -> Vec<String> {
    Analyser::new()
        .analyse(source, "tcl8.6")
        .diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.code == code)
        .map(|diagnostic| diagnostic.message)
        .collect()
}

#[test]
fn issue_2288_prefixed_tk_bind_script() {
    let source = "package require Tk\nbind $frameName <Destroy> +[list [self] destroy]";
    let unsafe_source = "package require Tk\nproc f {y} {bind .x <Destroy> \"puts $y\"}";
    assert!(codes(source, DiagCode::W105).is_empty());
    assert!(!codes(unsafe_source, DiagCode::W105).is_empty());
}

#[test]
fn issue_2499_dynamic_variable_read() {
    let source = "proc f {sides} {foreach side $sides {set value [set $side]}}";
    assert!(codes(source, DiagCode::W212).is_empty());
    assert!(!codes("proc f {name} {set $name 1}", DiagCode::W212).is_empty());
}

#[test]
fn issue_2503_nested_expr_inside_quoted_result() {
    let source =
        "set progress \"\"\nexpr {$progress eq \"\" ? \"\" : \"[expr {round($progress * 100)}]%\"}";
    assert!(codes(source, DiagCode::W114).is_empty());
    assert!(!codes("expr {[expr {1+1}] + 1}", DiagCode::W114).is_empty());
}

#[test]
fn issue_2505_multiline_list_uplevel() {
    let source = "uplevel #0 [list \\\n    puts hello \\\n]";
    assert!(codes(source, DiagCode::W301).is_empty());
    assert!(codes("uplevel #0 [list puts hello]", DiagCode::W301).is_empty());
    assert!(!codes("proc f {x} {uplevel #0 \"puts $x\"}", DiagCode::W301).is_empty());
}
