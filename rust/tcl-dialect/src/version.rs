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

//! The ordered Tcl release enum behaviour semantics key off, and the
//! three-valued policy type for behaviours a non-Tcl profile has no
//! opinion on.

/// A specific Tcl release whose **compile-time** semantics a constant fold may
/// depend on — e.g. `string is integer` is unbounded on 9.0 but caps at
/// `2³²-1` on 8.x, and `string is wideinteger` / `entier` / `dict` and
/// `format %b` don't exist (they *raise*) before a given release.  Ordered, so
/// a fold can test `version >= TclVersion::V8_5`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TclVersion {
    /// Tcl 8.4.
    V8_4,
    /// Tcl 8.5.
    V8_5,
    /// Tcl 8.6.
    V8_6,
    /// Tcl 9.0.
    V9_0,
    /// Tcl 9.1. Shares 9.0's compile-time fold semantics (a superset release);
    /// ordered after `V9_0` so `>= V9_0` gates include it.
    V9_1,
}

impl TclVersion {
    /// Map an optimiser dialect string (`"tcl8.4"` … `"tcl9.1"`) to a version,
    /// or `None` for an unversioned (`"tcl"`), non-Tcl (`"f5-irules"`), or
    /// unknown dialect — in which case a versioned fold must return only the
    /// dialect-invariant subset every release shares.
    #[must_use]
    pub fn from_dialect(dialect: Option<&str>) -> Option<Self> {
        match dialect {
            Some("tcl8.4") => Some(Self::V8_4),
            Some("tcl8.5") => Some(Self::V8_5),
            Some("tcl8.6") => Some(Self::V8_6),
            Some("tcl9.0") => Some(Self::V9_0),
            // 9.1 must not fall through to `None` (which degrades a versioned
            // fold to the dialect-invariant subset) — it behaves as 9.0+
            // (RUST_ISSUE_083).
            Some("tcl9.1") => Some(Self::V9_1),
            _ => None,
        }
    }
}

/// A three-valued behaviour policy, so a non-Tcl profile (`f5-bigip`) and
/// the permissive unknown-dialect fallback are **inert** — "no opinion" —
/// rather than silently defaulted to one of the real behaviours
/// (dialect-profile-model.md §11.1). Consumers short-circuit on
/// [`Ternary::Inert`] instead of guessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ternary {
    /// The behaviour applies (e.g. leading-zero integers read as octal).
    Yes,
    /// The behaviour does not apply (e.g. Tcl 9.x dropped bare-leading-zero
    /// octal, TIP 114/472).
    No,
    /// No opinion — the profile is not a Tcl runtime, or is the permissive
    /// unknown-dialect sink. Validators and const-folders abstain.
    Inert,
}

impl Ternary {
    /// The policy as an `Option<bool>`: `Inert` is `None`, so callers that
    /// already model "undecided" as `None` (the expr const-folder's octal
    /// input) consume it directly.
    #[must_use]
    pub fn as_bool(self) -> Option<bool> {
        match self {
            Self::Yes => Some(true),
            Self::No => Some(false),
            Self::Inert => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TclVersion, Ternary};

    #[test]
    fn from_dialect_maps_every_versioned_tcl() {
        // RUST_ISSUE_083: tcl9.1 must resolve to a version, not `None` (which
        // silently degrades versioned folds to the dialect-invariant subset).
        assert_eq!(
            TclVersion::from_dialect(Some("tcl8.4")),
            Some(TclVersion::V8_4)
        );
        assert_eq!(
            TclVersion::from_dialect(Some("tcl8.5")),
            Some(TclVersion::V8_5)
        );
        assert_eq!(
            TclVersion::from_dialect(Some("tcl8.6")),
            Some(TclVersion::V8_6)
        );
        assert_eq!(
            TclVersion::from_dialect(Some("tcl9.0")),
            Some(TclVersion::V9_0)
        );
        assert_eq!(
            TclVersion::from_dialect(Some("tcl9.1")),
            Some(TclVersion::V9_1)
        );
        // Unversioned / non-Tcl / unknown → None.
        assert_eq!(TclVersion::from_dialect(Some("tcl")), None);
        assert_eq!(TclVersion::from_dialect(Some("f5-irules")), None);
        assert_eq!(TclVersion::from_dialect(None), None);
    }

    #[test]
    fn v9_1_orders_at_or_above_v9_0() {
        // A `>= V9_0` gate must include 9.1.
        assert!(TclVersion::V9_1 >= TclVersion::V9_0);
        assert!(TclVersion::V9_1 > TclVersion::V8_6);
    }

    #[test]
    fn ternary_maps_inert_to_none() {
        assert_eq!(Ternary::Yes.as_bool(), Some(true));
        assert_eq!(Ternary::No.as_bool(), Some(false));
        assert_eq!(Ternary::Inert.as_bool(), None);
    }
}
