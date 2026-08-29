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

//! A reloaded `include from` roster reaches the **`&'static` promotion**,
//! not just the generation cache.
//!
//! `static_context_for` hands the LSP providers a leaked view of an
//! environment's un-overlaid generation. Its key has to carry everything
//! that generation's assembly answers under, and the surface roster moves
//! on an axis of its own: `register_inherited_surfaces` advances its
//! generation without touching any environment, so a key that named only
//! the environment would keep serving pre-reload jim availability for the
//! life of the process.
//!
//! Its own test binary, for the reason `surface_roster_trust.rs` gives:
//! the roster store is process-wide, so a test that replaces it must not
//! share a process with the ones that ask what a `jim` generation offers.

use tcl_registry::model::static_context_for;

/// A head Tcl 8.6 has and no `jimsh` does — so the compiled-in roster
/// withholds it, and a widened roster is a visible change.
const NOT_IN_JIM: &str = "coroutine";

/// The roster jim's compiled-in surface declares, plus [`NOT_IN_JIM`].
///
/// Registering it replaces the compiled-in one, which is the reload this
/// is about.
fn widened_roster() -> Vec<tcl_dialect::model::InheritedSurface> {
    let pack = tcl_spectcl::loader::evaluate_pack(&format!(
        "speclib widened 2.0 {{\n include from tcl into jim {{set {NOT_IN_JIM}}}\n}}"
    ));
    assert_eq!(pack.surface_rosters.len(), 1, "the row is well-formed");
    tcl_spectcl::surface_roster_conversion::to_inherited_surfaces(
        &pack.surface_rosters,
        tcl_dialect::model::Provenance::BuiltIn,
    )
}

#[test]
fn a_roster_reload_reaches_the_promoted_static_view() {
    tcl_spectcl::core_surfaces::ensure();
    assert!(
        static_context_for("jim")
            .resolve_command(NOT_IN_JIM)
            .is_none(),
        "the compiled-in roster withholds `{NOT_IN_JIM}` from a jim document"
    );

    let outcome = tcl_dialect::model::register_inherited_surfaces(widened_roster());
    assert!(outcome.changed, "{outcome:?}");

    assert!(
        static_context_for("jim")
            .resolve_command(NOT_IN_JIM)
            .is_some(),
        "the promotion served the pre-reload roster: `{NOT_IN_JIM}` is on the \
         registered roster and a jim document is still not offered it"
    );
}
