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

use tcl_compiler::ir::ProcedureId;

pub trait FixtureBody {
    fn fixture_display(&self) -> &str;
}

impl FixtureBody for tcl_compiler::ir::Procedure {
    fn fixture_display(&self) -> &str {
        &self.qualified_name
    }
}
impl FixtureBody for tcl_compiler::cfg::Function {
    fn fixture_display(&self) -> &str {
        &self.name
    }
}
impl FixtureBody for tcl_bytecode::FunctionAsm {
    fn fixture_display(&self) -> &str {
        &self.name
    }
}
impl FixtureBody for tcl_compiler::compilation_unit::FunctionUnit {
    fn fixture_display(&self) -> &str {
        &self.name
    }
}
impl FixtureBody for tcl_compiler::interprocedural::ProcSummary {
    fn fixture_display(&self) -> &str {
        &self.qualified_name
    }
}
impl FixtureBody for tcl_compiler::interprocedural::MethodSummary {
    fn fixture_display(&self) -> &str {
        &self.base.qualified_name
    }
}

pub fn fixture_id_if_present<'a, T: FixtureBody + 'a>(
    inventory: impl IntoIterator<Item = (&'a ProcedureId, &'a T)>,
    display: &str,
) -> Option<ProcedureId> {
    let mut matching = inventory
        .into_iter()
        .filter(|(_, body)| body.fixture_display() == display);
    let selected = matching.next().map(|(id, _)| *id);
    assert!(
        matching.next().is_none(),
        "fixture body {display:?} ambiguous"
    );
    selected
}
