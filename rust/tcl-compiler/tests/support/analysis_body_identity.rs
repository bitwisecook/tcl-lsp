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

//! Original body selection for tests that only inspect source metadata.
//! A reporting label selects an issued original body, never a callable,
//! entered frame, inlining permission or executable replacement.

use tcl_compiler::ir::{Module, Procedure, ProcedureId};

pub fn original_body_for_analysis<'a>(
    module: &'a Module,
    display: &str,
) -> Option<(ProcedureId, &'a Procedure)> {
    let registry = module.resolved_registry();
    let mut primary = module
        .procedures
        .iter()
        .filter(|(_, procedure)| procedure.qualified_name == display);
    if let Some((&id, procedure)) = primary.next() {
        assert!(
            primary.next().is_none(),
            "fixture body {display:?} ambiguous"
        );
        // A present primary body with unavailable original ownership is terminal.
        return (module.original_procedure_report_label(id, registry) == Some(display))
            .then_some((id, procedure));
    }
    let mut original = module
        .body_lowerings()
        .filter(|(id, _)| module.original_procedure_report_label(**id, registry) == Some(display));
    let (&id, procedure) = original.next()?;
    assert!(
        original.next().is_none(),
        "fixture body {display:?} ambiguous"
    );
    Some((id, procedure))
}
