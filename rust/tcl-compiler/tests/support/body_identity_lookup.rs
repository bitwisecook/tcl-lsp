// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
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
