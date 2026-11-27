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

use std::borrow::Cow;
use tcl_compiler::ir::ProcedureId;

pub(crate) trait FixtureBodyLabel {
    fn fixture_label(&self) -> Cow<'_, str>;
}
impl FixtureBodyLabel for tcl_bytecode::FunctionAsm {
    fn fixture_label(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.name)
    }
}
impl FixtureBodyLabel for tcl_compiler::ir::Procedure {
    fn fixture_label(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.qualified_name)
    }
}
impl FixtureBodyLabel for tcl_compiler::ir::MethodDef {
    fn fixture_label(&self) -> Cow<'_, str> {
        Cow::Owned(self.reporting_label())
    }
}
impl FixtureBodyLabel for tcl_compiler::cfg::Function {
    fn fixture_label(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.name)
    }
}
impl FixtureBodyLabel for tcl_compiler::compilation_unit::FunctionUnit {
    fn fixture_label(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.name)
    }
}
impl FixtureBodyLabel for tcl_compiler::interprocedural::ProcSummary {
    fn fixture_label(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.qualified_name)
    }
}
impl FixtureBodyLabel for tcl_compiler::interprocedural::MethodSummary {
    fn fixture_label(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.base.qualified_name)
    }
}
// A fixture label must identify one genuinely issued body before its ID is used.
pub(crate) fn fixture_id<'a, T: FixtureBodyLabel + 'a>(
    rows: impl IntoIterator<Item = (&'a ProcedureId, &'a T)>,
    label: &str,
) -> ProcedureId {
    fixture_id_if_present(rows, label)
        .unwrap_or_else(|| panic!("authored fixture body {label:?} absent"))
}
pub(crate) fn fixture_id_if_present<'a, T: FixtureBodyLabel + 'a>(
    rows: impl IntoIterator<Item = (&'a ProcedureId, &'a T)>,
    label: &str,
) -> Option<ProcedureId> {
    let mut matching = rows
        .into_iter()
        .filter(|(_, body)| body.fixture_label() == label);
    let first = matching.next().map(|(id, _)| *id);
    assert!(
        matching.next().is_none(),
        "authored fixture body {label:?} ambiguous"
    );
    first
}
