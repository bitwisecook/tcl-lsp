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

//! The `DialectProfile` catalog — one interned profile per canonical dialect.
//!
//! A profile is resolved from a dialect-name string **once at ingest**
//! (LSP `dialect_for_open`, CLI `effective_dialect`, `detect_dialect`) and
//! the `&'static DialectProfile` is threaded from there, so consumers stop
//! re-parsing dialect strings per query.
//!
//! This is the seed of the compositional model in
//! `docs/design/dialect-profile-model.md`: the identity axis lands first
//! (Milestone 1); the availability axis (masks + disable lists), the
//! behaviour axis (base versions, octal, grammar), and the versioned-library
//! axis are added by the following milestones.

/// One resolved dialect. `'static`, interned in [`DialectProfile::all`],
/// keyed by canonical name.
///
/// Equality of profiles is pointer identity — there is exactly one profile
/// per canonical dialect, plus the [`DialectProfile::plain_tcl`] fallback
/// every unknown name resolves to.
#[derive(Debug)]
pub struct DialectProfile {
    /// The canonical dialect name (`"tcl8.6"`, `"f5-irules"`, …). Stable:
    /// this is the string that round-trips through configuration
    /// (`tclLsp.selectDialect`, `folderDialects`), the registry-dump JSON
    /// schema, and `DialectSet::canonical_name`.
    pub name: &'static str,
    /// Legacy / editor spellings that resolve to this profile
    /// (`"irules"` → `f5-irules`). Populated when [`DialectProfile::by_name`]
    /// becomes the ingest resolver (Milestone 2) so the alias unification
    /// lands together with its consumers and tests; empty until then.
    pub aliases: &'static [&'static str],
}

/// The catalog: one profile per canonical dialect, in
/// [`KNOWN_DIALECTS`](crate::KNOWN_DIALECTS) (sorted) order.
static CATALOG: [DialectProfile; 16] = [
    DialectProfile {
        name: "bpf",
        aliases: &[],
    },
    DialectProfile {
        name: "cadence-eda-tcl",
        aliases: &[],
    },
    DialectProfile {
        name: "expect",
        aliases: &[],
    },
    DialectProfile {
        name: "f5-bigip",
        aliases: &[],
    },
    DialectProfile {
        name: "f5-iapps",
        aliases: &[],
    },
    DialectProfile {
        name: "f5-irules",
        aliases: &[],
    },
    DialectProfile {
        name: "f5-tmsh",
        aliases: &[],
    },
    DialectProfile {
        name: "intel-quartus-eda-tcl",
        aliases: &[],
    },
    DialectProfile {
        name: "mentor-eda-tcl",
        aliases: &[],
    },
    DialectProfile {
        name: "synopsys-eda-tcl",
        aliases: &[],
    },
    DialectProfile {
        name: "tcl8.4",
        aliases: &[],
    },
    DialectProfile {
        name: "tcl8.5",
        aliases: &[],
    },
    DialectProfile {
        name: "tcl8.6",
        aliases: &[],
    },
    DialectProfile {
        name: "tcl9.0",
        aliases: &[],
    },
    DialectProfile {
        name: "tcl9.1",
        aliases: &[],
    },
    DialectProfile {
        name: "xilinx-eda-tcl",
        aliases: &[],
    },
];

/// The single sink for every unparseable / typo / unset dialect string.
/// Deliberately permissive so an unknown dialect never flags valid code —
/// the availability/behaviour axes stay maximally permissive here.
static PLAIN_TCL: DialectProfile = DialectProfile {
    name: "tcl",
    aliases: &[],
};

impl DialectProfile {
    /// The full catalog of canonical dialect profiles, in sorted-name order
    /// (the [`KNOWN_DIALECTS`](crate::KNOWN_DIALECTS) order). Excludes the
    /// [`Self::plain_tcl`] fallback — it is a resolution sink, not a
    /// selectable dialect.
    #[must_use]
    pub fn all() -> &'static [DialectProfile] {
        &CATALOG
    }

    /// Resolve a dialect-name string to its interned profile,
    /// alias-normalised; every unknown name resolves to the permissive
    /// [`Self::plain_tcl`] fallback (never fails).
    #[must_use]
    pub fn by_name(name: &str) -> &'static DialectProfile {
        Self::find(name).unwrap_or(&PLAIN_TCL)
    }

    /// Resolve an *optional* dialect-name string: `None` and unknown names
    /// both land on [`Self::plain_tcl`]. The ingest-boundary form of
    /// [`Self::by_name`] for callers holding `Option<&str>`.
    #[must_use]
    pub fn by_opt_name(name: Option<&str>) -> &'static DialectProfile {
        name.map_or(&PLAIN_TCL, Self::by_name)
    }

    /// Like [`Self::by_name`] but distinguishing "unknown" from a real
    /// profile: returns `None` for a name that is neither a canonical
    /// dialect name nor a registered alias.
    #[must_use]
    pub fn find(name: &str) -> Option<&'static DialectProfile> {
        CATALOG
            .iter()
            .find(|p| p.name == name || p.aliases.contains(&name))
    }

    /// The `f5-irules` profile — an explicit handle for the hardcoded
    /// iRules lookups (event checks, taint, the iRules test framework).
    #[must_use]
    pub fn irules() -> &'static DialectProfile {
        Self::find("f5-irules").expect("f5-irules is in the catalog")
    }

    /// The permissive fallback profile every unknown / unset dialect
    /// resolves to (§8 of the design doc: one sink, `ALL_TCL`-permissive).
    #[must_use]
    pub fn plain_tcl() -> &'static DialectProfile {
        &PLAIN_TCL
    }
}

#[cfg(test)]
mod tests {
    use super::DialectProfile;
    use crate::dialect_set::KNOWN_DIALECTS;

    #[test]
    fn catalog_matches_known_dialects_exactly() {
        // One profile per canonical name, in the same (sorted) order —
        // KNOWN_DIALECTS and the catalog can never drift apart.
        let names: Vec<&str> = DialectProfile::all().iter().map(|p| p.name).collect();
        assert_eq!(names.as_slice(), KNOWN_DIALECTS);
    }

    #[test]
    fn by_name_resolves_canonical_names_to_themselves() {
        for &name in KNOWN_DIALECTS {
            assert_eq!(DialectProfile::by_name(name).name, name);
        }
    }

    #[test]
    fn unknown_names_sink_to_plain_tcl() {
        for unknown in ["", "nonsense", "tcl8.7", "TCL8.6"] {
            let p = DialectProfile::by_name(unknown);
            assert!(
                std::ptr::eq(p, DialectProfile::plain_tcl()),
                "{unknown:?} must resolve to the PLAIN_TCL sink"
            );
        }
        assert!(DialectProfile::find("nonsense").is_none());
    }

    #[test]
    fn by_opt_name_treats_none_as_plain_tcl() {
        assert!(std::ptr::eq(
            DialectProfile::by_opt_name(None),
            DialectProfile::plain_tcl()
        ));
        assert_eq!(DialectProfile::by_opt_name(Some("expect")).name, "expect");
    }

    #[test]
    fn irules_handle_is_the_catalog_entry() {
        let via_handle = DialectProfile::irules();
        let via_name = DialectProfile::by_name("f5-irules");
        assert!(std::ptr::eq(via_handle, via_name));
        assert_eq!(via_handle.name, "f5-irules");
    }

    #[test]
    fn profiles_are_interned_pointer_identities() {
        assert!(std::ptr::eq(
            DialectProfile::by_name("tcl8.6"),
            DialectProfile::by_name("tcl8.6")
        ));
        assert!(!std::ptr::eq(
            DialectProfile::by_name("tcl8.6"),
            DialectProfile::by_name("tcl9.0")
        ));
    }
}
