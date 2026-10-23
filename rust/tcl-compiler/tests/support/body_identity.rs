// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use tcl_compiler::ir::ProcedureId;

#[path = "body_identity_lookup.rs"]
mod lookup;

pub use lookup::{FixtureBody, fixture_id_if_present};

// Authored fixtures identify one retained body before accessing its issued ID.
pub fn fixture_id<'a, T: FixtureBody + 'a>(
    inventory: impl IntoIterator<Item = (&'a ProcedureId, &'a T)>,
    display: &str,
) -> ProcedureId {
    fixture_id_if_present(inventory, display)
        .unwrap_or_else(|| panic!("fixture body {display:?} absent"))
}
