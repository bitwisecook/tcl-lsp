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

//! Target version ranges, and the registry packs one spans (issue #1257).
//!
//! A document is written for a release, but it may be *run* on a later one.
//! Anything that rewrites a word whose meaning depends on a keyword table has
//! to reason over the whole range, not just the release in front of it: a
//! prefix that is unique today can become ambiguous when a later Tcl adds a
//! keyword (`string cat` arrived in 8.6.2 and shortened what `string c…`
//! could mean).
//!
//! [`DialectSet`] is already the project's range type — a set of core-Tcl
//! version bits — so this module is the small bridge from a range to the
//! **registry packs** it spans, plus the one rule that turns a single target
//! dialect into its forward-compatibility range.
//!
//! Consumers ask here rather than loading packs by dialect name themselves:
//! the minifier's ad-hoc "every later core-Tcl registry" walk and the
//! formatter's version-blind resolution were two copies of the same list, and
//! a new core release had to be remembered in both.


use crate::CommandRegistry;

/// Every core-Tcl release the registry ships a pack for, oldest first.
///
/// The single source of truth for release *order*. `DialectSet`'s bit order
/// agrees with it by construction (see [`DialectSet::member_names`]); this
/// list is what maps those bits back to loadable pack names.
pub const CORE_TCL_RELEASES: &[&str] = &["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"];

/// The core-Tcl release names a target `range` spans, oldest first.
///
/// Non-core bits (`IRULES`, `TK`, `EXPECT`, …) contribute nothing: they are
/// vendor gates, not releases on the core version axis, and the keyword
/// tables that change between releases are the core ones.
#[must_use]
pub fn core_releases_in(range: &[&'static str]) -> Vec<&'static str> {
    CORE_TCL_RELEASES
        .iter()
        .copied()
        .filter(|name| range.contains(name))
        .collect()
}

/// The registry packs a target `range` spans, oldest release first.
///
/// This is the answer to "what keyword tables does this document's target
/// range span" — the one helper the formatter, the minifier, and the
/// analyser share. An empty result means the range names no core release
/// (a pure vendor dialect), and a caller should fall back to the single
/// registry it was handed.
#[must_use]
pub fn registries_over_range(range: &[&'static str]) -> Vec<&'static CommandRegistry> {
    core_releases_in(range)
        .into_iter()
        .map(|name| {
            crate::model::ingress::static_context_for(name)
                .commands()
                .as_ref()
        })
        .collect()
}

/// The forward-compatibility range for a document targeting `dialect`: that
/// release and **every later** core-Tcl release.
///
/// This is the range a rewrite must be safe across, because a script written
/// for 8.6 may well be run under 9.0. A vendor dialect with no core version
/// A name that is not a core release (`f5-irules` names its own runtime)
/// yields the empty range — nothing to widen over, so a consumer keeps the
/// single table it holds, which is the pre-existing conservative behaviour.
#[must_use]
pub fn forward_range(dialect: &str) -> &'static [&'static str] {
    CORE_TCL_RELEASES
        .iter()
        .position(|name| *name == dialect)
        .map_or(&[], |pos| &CORE_TCL_RELEASES[pos..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_forward_range_is_the_target_and_every_later_release() {
        assert_eq!(
            core_releases_in(forward_range("tcl8.6")),
            vec!["tcl8.6", "tcl9.0", "tcl9.1"]
        );
        assert_eq!(
            core_releases_in(forward_range("tcl8.4")),
            CORE_TCL_RELEASES.to_vec()
        );
        // The newest release has nothing after it — the range is itself.
        assert_eq!(core_releases_in(forward_range("tcl9.1")), vec!["tcl9.1"]);
    }

    #[test]
    fn a_vendor_dialect_has_no_core_range() {
        // `f5-irules` names a runtime, not a core release: nothing to widen
        // over, so the consumer keeps the single table it holds.
        assert!(forward_range("f5-irules").is_empty());
        assert!(registries_over_range(forward_range("f5-irules")).is_empty());
        assert!(forward_range("nonsense").is_empty());
    }

    #[test]
    fn an_explicit_range_maps_to_its_packs_oldest_first() {
        let names = core_releases_in(SpecSurface::TCL85_PLUS);
        assert_eq!(names, vec!["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"]);
        assert_eq!(
            registries_over_range(SpecSurface::TCL85_PLUS).len(),
            names.len()
        );
        assert!(core_releases_in(None).is_empty());
    }

    #[test]
    fn a_mixed_range_keeps_only_the_core_releases() {
        // A vendor bit alongside core bits contributes no pack of its own.
        let range = SpecSurface::TCL86 | SpecSurface::TCL90 | SpecSurface::IRULES;
        assert_eq!(core_releases_in(range), vec!["tcl8.6", "tcl9.0"]);
    }
}
