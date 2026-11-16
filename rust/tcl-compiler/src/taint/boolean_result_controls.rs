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

use crate::compilation_unit::CompilationUnit;
use tcl_core_types::DiagCode;
use tcl_registry::CommandRegistry;

fn t100_operands(src: &str) -> Vec<String> {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry: &CommandRegistry = context.commands();
    let cu = CompilationUnit::build_for_profile(src, registry, false, profile);
    super::find_taint_warnings_for_cu(&cu, registry, Some(profile))
        .into_iter()
        .filter(|warning| warning.code == DiagCode::T100)
        .map(|warning| warning.variable)
        .collect()
}

#[test]
fn a_comparison_result_is_not_tainted_2506() {
    let operands = t100_operands(
        "set line [gets stdin]\nset trimmed [string trimleft $line]\n\
             set is_empty [expr {$trimmed eq \"\"}]\n\
             set is_comment [expr {!$is_empty && [string index $trimmed 0] eq \"#\"}]\n\
             set is_code [expr {!$is_empty && !$is_comment}]\n",
    );
    assert!(
        !operands
            .iter()
            .any(|operand| operand == "is_empty" || operand == "is_comment"),
        "{operands:?}"
    );
    let control = t100_operands("set n [gets stdin]\nset m [expr {$n + 1}]\nif {$m} {puts y}\n");
    assert!(control.iter().any(|operand| operand == "m"), "{control:?}");
    let boolean = t100_operands("set n [gets stdin]\nset m [expr {$n eq {}}]\nif {$m} {puts y}\n");
    assert!(!boolean.iter().any(|operand| operand == "m"), "{boolean:?}");
}
