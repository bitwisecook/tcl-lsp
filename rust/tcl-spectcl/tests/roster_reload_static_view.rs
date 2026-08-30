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

/// The roster store is process-wide, so the two tests here take this
/// before touching it — the same rule that gives this file its own binary,
/// one level in.
static ROSTERS: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A head Tcl 8.6 has and no `jimsh` does — so the compiled-in roster
/// withholds it, and a widened roster is a visible change.
const NOT_IN_JIM: &str = "coroutine";

/// Jim's roster with `extra` added, replacing the compiled-in one — the
/// reload these tests are about. `extra` also keeps the two tests'
/// rosters distinct, so each one's registration really does move the
/// generation whichever order they run in.
fn widened_roster(extra: &str) -> Vec<tcl_dialect::model::InheritedSurface> {
    let pack = tcl_spectcl::loader::evaluate_pack(&format!(
        "speclib widened 2.0 {{\n include from tcl into jim {{set {extra}}}\n}}"
    ));
    assert_eq!(pack.surface_rosters.len(), 1, "the row is well-formed");
    tcl_spectcl::surface_roster_conversion::to_inherited_surfaces(
        &pack.surface_rosters,
        tcl_dialect::model::Provenance::BuiltIn,
    )
}

/// Restore the compiled-in rosters, so neither test leaves the process
/// answering a `jim` question from the other's fixture.
fn restore() {
    let _ = tcl_dialect::model::register_inherited_surfaces(
        tcl_spectcl::core_surfaces::builtin_rosters(),
    );
}

#[test]
fn a_roster_reload_reaches_the_promoted_static_view() {
    let _rosters = ROSTERS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    tcl_spectcl::core_surfaces::ensure();
    restore();
    assert!(
        static_context_for("jim")
            .resolve_command(NOT_IN_JIM)
            .is_none(),
        "the compiled-in roster withholds `{NOT_IN_JIM}` from a jim document"
    );

    let outcome = tcl_dialect::model::register_inherited_surfaces(widened_roster(NOT_IN_JIM));
    assert!(outcome.changed, "{outcome:?}");

    assert!(
        static_context_for("jim")
            .resolve_command(NOT_IN_JIM)
            .is_some(),
        "the promotion served the pre-reload roster: `{NOT_IN_JIM}` is on the \
         registered roster and a jim document is still not offered it"
    );
    restore();
}

/// …and only where a roster can reach.
///
/// A roster narrows one *reimplementing* family. Keying every
/// environment's promotion on the roster axis would rebuild each of their
/// views — and, on a pack publication, the pack overlay behind each — on
/// a change that cannot move any of their answers, which is slow enough
/// to be its own defect. So the views a roster cannot reach must survive
/// one intact.
#[test]
fn a_roster_reload_leaves_the_environments_it_cannot_reach_alone() {
    let _rosters = ROSTERS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    tcl_spectcl::core_surfaces::ensure();
    restore();
    // Two with no ancestry, one fork, and one with no core of its own.
    let unreachable = ["tcl9.0", "expect", "f5-irules", "tk"];
    let before: Vec<_> = unreachable
        .iter()
        .map(|name| static_context_for(name))
        .collect();

    let outcome = tcl_dialect::model::register_inherited_surfaces(widened_roster("lassign"));
    assert!(outcome.changed, "{outcome:?}");

    for (name, was) in unreachable.iter().zip(before) {
        assert!(
            std::ptr::eq(static_context_for(name), was),
            "a roster cannot change what `{name}` offers, so its promoted \
             view must not be rebuilt"
        );
    }
    restore();
}
