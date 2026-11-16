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

use crate::interprocedural::test_body_identity::fixture_id_if_present;

use crate::compilation_unit::CompilationUnit;
use tcl_registry::CommandRegistry;

#[test]
fn proc_body_statement_spans_are_absolute() {
    let src = "proc ::f {} { set x 1; return $x }";
    let r = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let cu = CompilationUnit::build_for(src, &r, false);
    let proc = fixture_id_if_present(&cu.ir_module.procedures, "::f")
        .and_then(|id| cu.ir_module.procedures.get(&id))
        .expect("proc");
    let first = proc.body.statements.first().expect("body stmt");
    let span = first.span();
    let text = &src[span.as_range()];
    assert!(
        text.starts_with("set"),
        "expected absolute span pointing at `set`, got {text:?}",
    );
}
