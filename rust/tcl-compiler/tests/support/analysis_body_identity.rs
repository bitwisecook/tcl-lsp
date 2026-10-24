// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
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
