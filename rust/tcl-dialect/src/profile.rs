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
//! This is the compositional model of `docs/design/dialect-profile-model.md`
//! landing milestone by milestone: identity (Milestone 1), the availability
//! axis — masks, disable list, load layers, grammar unions (Milestone 2) —
//! with the behaviour axis and the versioned-library axis following.

use crate::dialect_set::DialectSet;

/// The commands F5's TMM interpreter removes from iRules: the K36322151
/// bans (file system, process, channel-config, event-loop, and
/// introspection commands the data-plane sandbox strips) plus the
/// project-modelled iRules-excluded internals (`tcl::build-info`,
/// `tcl_findLibrary`, the regex-quote helpers).
///
/// This is the **subtractive** half of the iRules profile: the
/// [`DialectProfile::availability_mask`] is the *bare* `IRULES` bit and every
/// availability consumer applies this disable list **after** the mask query
/// (`is_available` = mask ∧ ¬disabled). Never fold these into the mask as a
/// `TCL84|IRULES` union — the union re-admits exactly these commands (see
/// §9 of the design doc, the subtractive-iRules trap).
///
/// Membership is data-bound, not hand-curated: until the Milestone 5 data
/// retag the per-spec `NON_IRULES_OPERATORS` tags encode the same exclusion,
/// and `tcl-registry`'s `irules_disabled_commands_match_the_spec_data`
/// contract test keeps this list and the spec data in lock-step from both
/// sides. (8.5+/8.6 core such as `dict`/`lmap` is deliberately NOT here —
/// that is version-gating on the pinned 8.4 base, a different axis.)
const IRULES_DISABLED_COMMANDS: &[&str] = &[
    "::tcl::build-info",
    "::tcl::unsupported::corotype",
    "auto_execok",
    "auto_import",
    "auto_load",
    "auto_mkindex",
    "auto_mkindex_old",
    "auto_qualify",
    "auto_reset",
    "bgerror",
    "cd",
    "eof",
    "exec",
    "exit",
    "fblocked",
    "fconfigure",
    "fcopy",
    "file",
    "fileevent",
    "filename",
    "flush",
    "gets",
    "glob",
    "http",
    "interp",
    "load",
    "memory",
    "namespace",
    "open",
    "package",
    "pid",
    "pkg_mkindex",
    "pwd",
    "re_quote",
    "regex::quote",
    "regex_quote",
    "regexp::quote",
    "rename",
    "seek",
    "socket",
    "source",
    "tcl::build-info",
    "tcl_findLibrary",
    "tell",
    "time",
    "timerate",
    "unknown",
    "unload",
    "update",
    "vwait",
];

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
    /// (`"irules"` → `f5-irules`). Resolution through [`Self::by_name`]
    /// canonicalises them, so profile predicates can never disagree with
    /// the canonical spelling the way the string-keyed tables used to
    /// (design doc §2.4).
    pub aliases: &'static [&'static str],
    /// AXIS A: the **precise** availability mask commands / subcommands /
    /// options / special variables are membership-tested against
    /// (`spec.supports_dialect(intersects)`). Composed as
    /// `(signature-base Tcl version bits) | (vendor bit)` for the additive
    /// vendor dialects (`TCL85|IAPPS`, `TCL86|EXPECT`, …).
    ///
    /// For **subtractive** profiles (iRules) this is the *bare* vendor bit
    /// and [`Self::disabled_commands`] carries the exclusions — never a
    /// version|vendor union (§9 of the design doc).
    pub availability_mask: DialectSet,
    /// Subtractive disable list, applied AFTER the mask query by every
    /// availability consumer. Non-empty only for `f5-irules` (the 42
    /// K36322151 commands). Empty for additive dialects.
    pub disabled_commands: &'static [&'static str],
    /// The registry command packs `load_dialect` applies for this profile,
    /// in order. Empty for the plain Tcl versions (the default build) and
    /// for the config-only dialects that have no pack yet (`f5-tmsh`,
    /// `f5-bigip` — first-class in Milestone 6).
    pub base_layers: &'static [DialectSet],
    /// Coarse over-approximating union for **static** grammars only
    /// (tree-sitter / tmLanguage first-paint highlighting). Deliberately
    /// wider than [`Self::availability_mask`] — precise per-version
    /// correctness is the LSP semantic-token layer's job (§10). iRules is
    /// the exception: its static grammar is scoped to the bare `IRULES`
    /// bit (the shipped highlight fix this model preserves).
    pub grammar_union: DialectSet,
}

/// The catalog: one profile per canonical dialect, in
/// [`KNOWN_DIALECTS`](crate::KNOWN_DIALECTS) (sorted) order.
///
/// Mask values follow the per-dialect table in
/// `docs/design/dialect-profile-model.md` §7. Interim values that the
/// milestone plan tightens later are commented at the entry.
static CATALOG: [DialectProfile; 16] = [
    // bpf embeds a genuine Tcl 9.0 (design doc D7). Until Milestone 6 gives
    // it the precise `TCL90|BPF` mask (with its reverse-regression golden
    // budget), the mask over-approximates with every Tcl version so no core
    // command is falsely unknown — strictly fewer false positives than the
    // pre-profile bare-`BPF` view, never more.
    DialectProfile {
        name: "bpf",
        aliases: &[],
        availability_mask: DialectSet::ALL_TCL.union(DialectSet::BPF),
        disabled_commands: &[],
        base_layers: &[DialectSet::BPF],
        grammar_union: DialectSet::ALL_TCL.union(DialectSet::BPF),
    },
    DialectProfile {
        name: "cadence-eda-tcl",
        aliases: &[],
        availability_mask: DialectSet::TCL86.union(DialectSet::CADENCE),
        disabled_commands: &[],
        base_layers: &[DialectSet::CADENCE],
        grammar_union: DialectSet::ALL_TCL.union(DialectSet::CADENCE),
    },
    DialectProfile {
        name: "expect",
        aliases: &[],
        availability_mask: DialectSet::TCL86.union(DialectSet::EXPECT),
        disabled_commands: &[],
        base_layers: &[DialectSet::EXPECT],
        grammar_union: DialectSet::ALL_TCL.union(DialectSet::EXPECT),
    },
    // f5-bigip is a config parser, not a Tcl surface; it has no command
    // pack. Until Milestone 6 models it first-class, resolution stays as
    // permissive as the pre-profile fallback so nothing regresses.
    DialectProfile {
        name: "f5-bigip",
        aliases: &[],
        availability_mask: DialectSet::ALL_TCL,
        disabled_commands: &[],
        base_layers: &[],
        grammar_union: DialectSet::ALL_TCL,
    },
    // iApps run a real Tcl 8.5.13 *host* interpreter (not the TMM sandbox):
    // full 8.5 core (dict, lassign, apply) plus the iApp surface; nothing
    // disabled. `TCL85|IAPPS` is the W123/W002 fix this milestone lands.
    DialectProfile {
        name: "f5-iapps",
        aliases: &[],
        availability_mask: DialectSet::TCL85.union(DialectSet::IAPPS),
        disabled_commands: &[],
        base_layers: &[DialectSet::IAPPS],
        grammar_union: DialectSet::ALL_TCL.union(DialectSet::IAPPS),
    },
    // iRules is SUBTRACTIVE (§9): a genuine embedded Tcl 8.4.6 whose F5
    // command surface carries the IRULES tag, minus the 42 K36322151
    // disables. The mask stays the BARE bit — `TCL84|IRULES` would re-admit
    // exactly the disabled commands through their (until Milestone 5)
    // `NON_IRULES_OPERATORS` tags.
    DialectProfile {
        name: "f5-irules",
        aliases: &["irules", "tcl-irule"],
        availability_mask: DialectSet::IRULES,
        disabled_commands: IRULES_DISABLED_COMMANDS,
        base_layers: &[DialectSet::IRULES],
        grammar_union: DialectSet::IRULES,
    },
    // f5-tmsh has no DialectSet bit or command pack yet (first-class in
    // Milestone 6, D8, with the reverse-regression budget); until then it
    // resolves as permissively as the pre-profile fallback did.
    DialectProfile {
        name: "f5-tmsh",
        aliases: &[],
        availability_mask: DialectSet::ALL_TCL,
        disabled_commands: &[],
        base_layers: &[],
        grammar_union: DialectSet::ALL_TCL,
    },
    DialectProfile {
        name: "intel-quartus-eda-tcl",
        aliases: &[],
        availability_mask: DialectSet::TCL85.union(DialectSet::QUARTUS),
        disabled_commands: &[],
        base_layers: &[DialectSet::QUARTUS],
        grammar_union: DialectSet::ALL_TCL.union(DialectSet::QUARTUS),
    },
    DialectProfile {
        name: "mentor-eda-tcl",
        aliases: &[],
        availability_mask: DialectSet::TCL85.union(DialectSet::MENTOR),
        disabled_commands: &[],
        base_layers: &[DialectSet::MENTOR],
        grammar_union: DialectSet::ALL_TCL.union(DialectSet::MENTOR),
    },
    DialectProfile {
        name: "synopsys-eda-tcl",
        aliases: &[],
        availability_mask: DialectSet::TCL86.union(DialectSet::SYNOPSYS),
        disabled_commands: &[],
        base_layers: &[DialectSet::SYNOPSYS],
        grammar_union: DialectSet::ALL_TCL.union(DialectSet::SYNOPSYS),
    },
    DialectProfile {
        name: "tcl8.4",
        aliases: &[],
        availability_mask: DialectSet::TCL84,
        disabled_commands: &[],
        base_layers: &[],
        grammar_union: DialectSet::ALL_TCL,
    },
    DialectProfile {
        name: "tcl8.5",
        aliases: &[],
        availability_mask: DialectSet::TCL85,
        disabled_commands: &[],
        base_layers: &[],
        grammar_union: DialectSet::ALL_TCL,
    },
    DialectProfile {
        name: "tcl8.6",
        aliases: &[],
        availability_mask: DialectSet::TCL86,
        disabled_commands: &[],
        base_layers: &[],
        grammar_union: DialectSet::ALL_TCL,
    },
    DialectProfile {
        name: "tcl9.0",
        aliases: &[],
        availability_mask: DialectSet::TCL90,
        disabled_commands: &[],
        base_layers: &[],
        grammar_union: DialectSet::ALL_TCL,
    },
    // Tag-level `TCL90_PLUS` unions already give 9.1 its 9.0 inheritance,
    // so the exact bit keeps per-version gating precise.
    DialectProfile {
        name: "tcl9.1",
        aliases: &[],
        availability_mask: DialectSet::TCL91,
        disabled_commands: &[],
        base_layers: &[],
        grammar_union: DialectSet::ALL_TCL,
    },
    DialectProfile {
        name: "xilinx-eda-tcl",
        aliases: &[],
        availability_mask: DialectSet::TCL85.union(DialectSet::XILINX),
        disabled_commands: &[],
        base_layers: &[DialectSet::XILINX],
        grammar_union: DialectSet::ALL_TCL.union(DialectSet::XILINX),
    },
];

/// The single sink for every unparseable / typo / unset dialect string.
/// Deliberately permissive so an unknown dialect never flags valid code
/// (design doc §8): full `ALL_TCL` availability, nothing disabled, no pack.
static PLAIN_TCL: DialectProfile = DialectProfile {
    name: "tcl",
    aliases: &[],
    availability_mask: DialectSet::ALL_TCL,
    disabled_commands: &[],
    base_layers: &[],
    grammar_union: DialectSet::ALL_TCL,
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

    /// Whether this profile's disable list bans `name` (the subtractive
    /// filter every availability consumer applies after its mask query —
    /// §9 of the design doc). Compares the *registry spec name*; callers
    /// normalise any `::` qualification before asking.
    #[must_use]
    pub fn is_command_disabled(&self, name: &str) -> bool {
        // Binary search is possible (the list is sorted) but a 42-entry
        // linear scan of `&str`s is already cheaper than the hash lookups
        // around it, and only iRules has a non-empty list at all.
        self.disabled_commands.contains(&name)
    }
}

#[cfg(test)]
mod tests {
    use super::DialectProfile;
    use crate::dialect_set::{DialectSet, KNOWN_DIALECTS};

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

    #[test]
    fn irules_aliases_canonicalise_to_the_same_profile() {
        // §2.4: `irules` / `tcl-irule` must resolve to the f5-irules
        // profile so profile predicates can never disagree with the
        // canonical spelling.
        for alias in ["irules", "tcl-irule"] {
            assert!(
                std::ptr::eq(DialectProfile::by_name(alias), DialectProfile::irules()),
                "{alias:?} must canonicalise to f5-irules"
            );
        }
    }

    #[test]
    fn irules_mask_is_the_bare_vendor_bit_with_disables() {
        // The subtractive-iRules trap (§9): the mask must stay the bare
        // IRULES bit — a TCL84|IRULES union would re-admit the disabled
        // commands — and the disable list must be present and sorted.
        let p = DialectProfile::irules();
        assert_eq!(p.availability_mask, DialectSet::IRULES);
        assert_eq!(p.disabled_commands.len(), 50);
        let mut sorted = p.disabled_commands.to_vec();
        sorted.sort_unstable();
        assert_eq!(p.disabled_commands, sorted.as_slice(), "list stays sorted");
        assert!(p.is_command_disabled("exec"));
        assert!(p.is_command_disabled("file"));
        assert!(p.is_command_disabled("socket"));
        assert!(!p.is_command_disabled("pool"));
        assert!(!p.is_command_disabled("set"));
    }

    #[test]
    fn only_irules_is_subtractive() {
        for p in DialectProfile::all() {
            if p.name == "f5-irules" {
                continue;
            }
            assert!(
                p.disabled_commands.is_empty(),
                "{}: only iRules carries a disable list",
                p.name
            );
        }
        assert!(
            DialectProfile::plain_tcl().disabled_commands.is_empty(),
            "the fallback sink disables nothing"
        );
    }

    #[test]
    fn additive_vendor_masks_compose_base_version_and_vendor_bit() {
        // §7: the composed (version|vendor) masks the W123/W002 fix rests on.
        let cases: &[(&str, DialectSet, DialectSet)] = &[
            ("f5-iapps", DialectSet::TCL85, DialectSet::IAPPS),
            ("expect", DialectSet::TCL86, DialectSet::EXPECT),
            ("synopsys-eda-tcl", DialectSet::TCL86, DialectSet::SYNOPSYS),
            ("cadence-eda-tcl", DialectSet::TCL86, DialectSet::CADENCE),
            ("xilinx-eda-tcl", DialectSet::TCL85, DialectSet::XILINX),
            (
                "intel-quartus-eda-tcl",
                DialectSet::TCL85,
                DialectSet::QUARTUS,
            ),
            ("mentor-eda-tcl", DialectSet::TCL85, DialectSet::MENTOR),
        ];
        for &(name, base, vendor) in cases {
            let p = DialectProfile::by_name(name);
            assert_eq!(p.availability_mask, base | vendor, "{name}");
        }
    }

    #[test]
    fn plain_tcl_versions_keep_their_exact_bit() {
        for (name, bit) in [
            ("tcl8.4", DialectSet::TCL84),
            ("tcl8.5", DialectSet::TCL85),
            ("tcl8.6", DialectSet::TCL86),
            ("tcl9.0", DialectSet::TCL90),
            ("tcl9.1", DialectSet::TCL91),
        ] {
            assert_eq!(DialectProfile::by_name(name).availability_mask, bit);
        }
    }

    #[test]
    fn config_only_dialects_stay_permissive_until_first_class() {
        // f5-tmsh / f5-bigip (Milestone 6, D8) must not regress before their
        // reverse-regression golden budget lands.
        for name in ["f5-tmsh", "f5-bigip"] {
            assert_eq!(
                DialectProfile::by_name(name).availability_mask,
                DialectSet::ALL_TCL,
                "{name}"
            );
        }
        // bpf keeps a strictly-widening interim mask until D7 lands in M6.
        assert_eq!(
            DialectProfile::by_name("bpf").availability_mask,
            DialectSet::ALL_TCL | DialectSet::BPF
        );
    }

    #[test]
    fn grammar_union_is_never_narrower_than_the_mask() {
        // §10: the static-grammar union over-approximates (or equals, for
        // subtractive iRules) the precise mask — never under-approximates.
        for p in DialectProfile::all() {
            assert!(
                p.grammar_union.contains(p.availability_mask),
                "{}: grammar_union must cover availability_mask",
                p.name
            );
        }
    }

    #[test]
    fn irules_grammar_union_is_the_shipped_bare_bit_fix() {
        // The shipped iRules highlight fix is literally
        // `grammar_union == availability_mask == IRULES` (§9.1).
        let p = DialectProfile::irules();
        assert_eq!(p.grammar_union, DialectSet::IRULES);
    }
}
