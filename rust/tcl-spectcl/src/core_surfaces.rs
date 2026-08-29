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

//! **The compiled-in core surface packs** — design **Q6**.
//!
//! Q1 ruled that the shipped command surfaces stay native Rust, with one
//! carve-out: Jim's. Its surface is *authored as `SpecTcl` and loaded*,
//! because what it has to say — "of Tcl 8.6's commands, Jim implements
//! these" — is a roster, and a roster written as Rust is a second
//! catalogue to keep in step with the first.
//!
//! ## Why this is not a discoverable pack
//!
//! [`crate::bundled`]'s eight EDA loadables ship in `specs/` and are
//! *replaceable*: a distribution, `TCL_LSP_SPEC_PACK_DIR`, or a dev
//! checkout can put a different `specs/` in front of them, and the
//! embedded copies exist only for a bare binary with no directory beside
//! it. That contract is right for a vendor library and wrong for a core
//! surface — under it, an empty `specs/` directory would delete `proc`
//! from Jim.
//!
//! So the sources here are compiled in and loaded **unconditionally**,
//! ahead of and independently of discovery, at [`Provenance::BuiltIn`].
//! They are `SpecTcl` in the sense Q1 meant — authored in the vocabulary,
//! read by the one loader, exercising the same words a third-party pack
//! would — without being a file anyone can take away.
//!
//! ## Ordering
//!
//! [`ensure`] is idempotent and cheap after the first call.
//! [`crate::registration::publish_pack_set`] folds these rosters in with
//! whatever the loaded set declares, so the model-side sync (which
//! replaces the whole store) can never drop them; [`ensure`] covers the
//! callers that never publish a set at all.

use std::sync::OnceLock;

use tcl_dialect::model::{InheritedSurface, Provenance};

use crate::loader::PackSurfaceRoster;

/// The core surface packs' `SpecTcl` sources, by the name their notices
/// report against.
const CORE_SURFACES: &[(&str, &str)] =
    &[("jim.tclspec", include_str!("../core-surfaces/jim.tclspec"))];

/// The roster rows the compiled-in sources declare, parsed once.
fn rows() -> &'static [PackSurfaceRoster] {
    static ROWS: OnceLock<Vec<PackSurfaceRoster>> = OnceLock::new();
    ROWS.get_or_init(|| {
        CORE_SURFACES
            .iter()
            .flat_map(|(_, source)| crate::loader::evaluate_pack(source).surface_rosters)
            .collect()
    })
}

/// The compiled-in rosters, as the model's own data.
///
/// Recomputed per call from the parsed rows (which are cached), so a
/// caller folding these in with pack-declared rosters gets a fresh,
/// owned set to hand to the sync.
#[must_use]
pub fn builtin_rosters() -> Vec<InheritedSurface> {
    crate::surface_roster_conversion::to_inherited_surfaces(rows(), Provenance::BuiltIn)
}

/// Register the compiled-in rosters, if nothing has registered any yet.
///
/// The entry point for a process that never publishes a pack set — a
/// test, a tool reading the catalogue directly. A process that *does*
/// publish goes through [`crate::registration::publish_pack_set`], which
/// folds these in on every publication rather than racing this.
pub fn ensure() {
    static DONE: OnceLock<()> = OnceLock::new();
    DONE.get_or_init(|| {
        let _ = tcl_dialect::model::register_inherited_surfaces(builtin_rosters());
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::model::Family;

    /// The pack parses, and what it parses to is Jim's roster of Tcl's
    /// surface — not an empty one, which is the failure mode that would
    /// otherwise be silent (an empty roster still registers, and would
    /// delete Jim's entire inherited surface).
    #[test]
    fn the_compiled_in_jim_roster_loads_and_is_not_empty() {
        let surfaces = builtin_rosters();
        let [jim] = surfaces.as_slice() else {
            panic!("one compiled-in roster: {surfaces:?}");
        };
        assert_eq!(jim.target, Family::Jim);
        assert_eq!(jim.source, Family::Tcl);
        assert_eq!(jim.provenance, Provenance::BuiltIn);
        assert_eq!(
            jim.names.len(),
            84,
            "the measured jim ∩ tclsh8.6 surface across 0.76-0.84"
        );
    }

    /// The two halves the roster exists to separate, at the pack level:
    /// what `jimsh` has, and what only the inherited edge offered.
    #[test]
    fn the_roster_carries_what_jimsh_has_and_not_what_it_lacks() {
        let jim = builtin_rosters().pop().expect("the jim roster");
        for present in ["set", "proc", "if", "dict", "lmap", "lassign", "try"] {
            assert!(jim.admits(present, None), "{present}");
        }
        for absent in [
            "coroutine",
            "trace",
            "yield",
            "yieldto",
            "chan",
            "encoding",
            "fblocked",
            "fcopy",
            "unload",
            "case",
            "unknown",
            "tclLog",
            "auto_load",
        ] {
            assert!(
                !jim.admits(absent, None),
                "{absent} is in `tclsh8.6` and not in any `jimsh` 0.76-0.84"
            );
        }
    }

    /// The two names that arrived mid-ladder keep their windows through
    /// the pack → model conversion.
    #[test]
    fn the_two_mid_ladder_names_keep_their_windows() {
        use tcl_dialect::model::Version;
        let jim = builtin_rosters().pop().expect("the jim roster");
        let at = |text: &str| Version::parse(text).expect("test version");
        assert!(!jim.admits("interp", Some(&at("0.76"))));
        assert!(jim.admits("interp", Some(&at("0.77"))));
        assert!(!jim.admits("zlib", Some(&at("0.77"))));
        assert!(jim.admits("zlib", Some(&at("0.78"))));
        // Everything else spans the whole ladder.
        assert!(jim.admits("proc", Some(&at("0.76"))));
    }
}
