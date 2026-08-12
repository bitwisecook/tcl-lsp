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

//! Dialect membership sets.

use bitflags::bitflags;

bitflags! {
    /// Compact set of Tcl dialects a command/subcommand is available in.
    ///
    /// `None` on a `CommandSpec` means "available in all dialects".
    /// A specific `DialectSet` restricts availability.
    ///
    /// Backed by `u64`: 12 bits are used today (5 Tcl versions + iRules, iApps,
    /// Tk, Expect, BPF, tmsh, the BIG-IP config surface); bits 8–12 were freed
    /// by the EDA-as-packages migration, leaving ample headroom for future
    /// dialect bits and the versioned-library work without a width migration.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct DialectSet: u64 {
        /// Tcl 8.4
        const TCL84     = 1 << 0;
        /// Tcl 8.5
        const TCL85     = 1 << 1;
        /// Tcl 8.6
        const TCL86     = 1 << 2;
        /// Tcl 9.0
        const TCL90     = 1 << 3;
        /// F5 iRules
        const IRULES    = 1 << 4;
        /// F5 iApps
        const IAPPS     = 1 << 5;
        /// Tk
        const TK        = 1 << 6;
        /// Expect
        const EXPECT    = 1 << 7;
        // Bits 8–12 (formerly the Synopsys/Cadence/Xilinx/Quartus/Mentor EDA
        // vendor bits) were retired: EDA shells are modelled as a base Tcl
        // version plus `required_package`-gated command libraries, not vendor
        // dialects (design doc `eda-library-packages.md`). The slots are left
        // free for future dialects.
        /// BPF-Tcl (the eBPF framework dialect)
        const BPF       = 1 << 13;
        /// Tcl 9.1
        const TCL91     = 1 << 14;
        /// F5 tmsh scripting (the tmsh shell's Tcl 8.5 host — a first-class
        /// dialect profile of its own, D8)
        const TMSH      = 1 << 15;
        /// F5 BIG-IP configuration (`bigip.conf` — a config surface with
        /// no Tcl command surface of its own; the bit exists for profile
        /// identity and the versioned-library axis)
        const BIGIP     = 1 << 16;

        /// All standard Tcl versions.
        const ALL_TCL = Self::TCL84.bits() | Self::TCL85.bits()
                      | Self::TCL86.bits() | Self::TCL90.bits() | Self::TCL91.bits();

        /// Tcl 8.5 and later.
        const TCL85_PLUS = Self::TCL85.bits() | Self::TCL86.bits()
                         | Self::TCL90.bits() | Self::TCL91.bits();

        /// Tcl 8.6 and later.
        const TCL86_PLUS = Self::TCL86.bits() | Self::TCL90.bits() | Self::TCL91.bits();

        /// The Tcl 8 series only (8.4, 8.5, 8.6), excluding 9.0+.  For
        /// commands and options dropped or renamed at the 9.0 boundary — e.g.
        /// `tcltest::bytestring` (not defined under Tcl 9.0+) and the
        /// `teststaticpkg` / `testsaveresult` test-harness commands (removed
        /// in 9.0).
        const TCL8X = Self::TCL84.bits() | Self::TCL85.bits() | Self::TCL86.bits();

        /// Tcl 9.0 and later.  A command/option gated to "9.0" persists in
        /// 9.1 (a `.1` release is additive): a `{tcl9.0}` membership is
        /// inherited under `tcl9.1`.
        const TCL90_PLUS = Self::TCL90.bits() | Self::TCL91.bits();

        /// Dialects in which the Tk widget/window commands (`button`,
        /// `pack`, `wm`, `winfo`, the `ttk::` forms, …) are available:
        /// standard Tcl (a `wish`/`package require Tk` interpreter) plus the
        /// pure-`tk` dialect.
        ///
        /// Tk is *not* part of the restricted embedded dialects (F5 iRules /
        /// iApps) or the vendor EDA shells, so its commands must never be
        /// offered or accepted there.  Membership in `ALL_TCL` (not `TK`
        /// alone) is deliberate: a plain `.tcl` file that does
        /// `package require Tk` keeps its `tcl8.6`/`tcl9.0` dialect, so the
        /// commands have to resolve under a Tcl version — the *loaded*
        /// gating (only present once `package require Tk` ran) is layered on
        /// top by the LSP, not by this dialect set.
        const TK_AND_TCL = Self::ALL_TCL.bits() | Self::TK.bits();

        // `NON_IRULES_OPERATORS` (every dialect except iRules/Tk/BPF) has
        // been retired. iRules availability is fully explicit per spec:
        // every command carries an explicit `dialects` group (universal
        // `dialects: None` was eliminated registry-wide), with the `IRULES`
        // bit present iff iRules enables it — so a K36322151-banned command
        // such as `exec` is just `ALL_TCL` and never intersects the bare
        // `IRULES` mask. The only remaining profile-level exclusion is
        // `Traits::OPERATOR_COMMAND` + `operators_as_commands` for the
        // math-operator heads.
    }
}

/// Canonical dialect profile names, in sorted order.
///
/// Kept pre-sorted so [`available_dialects`] returns them in sorted
/// order. This
/// is the single source of truth for the explorer's dialect dropdown and
/// the CLI's `--dialect` choices. Every name here resolves to its own
/// [`DialectSet::parse`] flag (`f5-tmsh` / `f5-bigip` carry their own bits
/// as first-class profiles, D8; `tk` parses to the grammar-layer `TK` bit
/// but is a library pin, not a profile — §7.2).
pub const KNOWN_DIALECTS: &[&str] = &[
    "bpf",
    "cadence-eda-tcl",
    "expect",
    "f5-bigip",
    "f5-iapps",
    "f5-irules",
    "f5-tmsh",
    "intel-quartus-eda-tcl",
    "mentor-eda-tcl",
    "synopsys-eda-tcl",
    "tcl8.4",
    "tcl8.5",
    "tcl8.6",
    "tcl9.0",
    "tcl9.1",
    "xilinx-eda-tcl",
];

/// Return the canonical dialect profile names in sorted order.
#[must_use]
pub fn available_dialects() -> &'static [&'static str] {
    KNOWN_DIALECTS
}

// Compile-time proof that `is_valid_nested_dialects` is genuinely usable in
// a `const` context (not just callable from one) — a command spec can copy
// this `const _: () = assert!(...)` shape with its own fields to get a hard
// build failure on an unreachable dialect combination, rather than relying
// solely on the registry-wide sweep test to catch it at `cargo test` time.
const _: () = assert!(DialectSet::is_valid_nested_dialects(None, None));
const _: () = assert!(DialectSet::is_valid_nested_dialects(
    None,
    Some(DialectSet::IRULES)
));
const _: () = assert!(DialectSet::is_valid_nested_dialects(
    Some(DialectSet::TCL86),
    Some(DialectSet::TCL86_PLUS)
));

impl DialectSet {
    /// Whether a nested dialect-gating declaration (`child`) is compatible
    /// with its governing declaration (`parent`) — i.e. every dialect bit
    /// `child` claims is also one `parent` claims, so `child` can actually
    /// be reached under some dialect the parent itself supports.
    ///
    /// Follows the `Option<DialectSet>` convention used throughout the
    /// registry crates: `None` means "all dialects" on either side, so a
    /// `None` on either side is always compatible; only a `Some(child)`
    /// claiming a bit `Some(parent)` lacks is invalid — e.g. an option
    /// declared `IRULES`-only under a command gated to `TCL86_PLUS` could
    /// never be reached, since the command itself is never selected
    /// outside `TCL86_PLUS`.
    ///
    /// A `const fn`, so a specific spec can self-check at compile time
    /// with a `const { assert!(...) }` block; a registry-wide sweep test
    /// covers every spec without requiring that per-file opt-in.
    #[must_use]
    pub const fn is_valid_nested_dialects(child: Option<Self>, parent: Option<Self>) -> bool {
        match (child, parent) {
            (Some(child), Some(parent)) => parent.contains(child),
            _ => true,
        }
    }

    /// Whether `name` denotes the F5 iRules dialect — resolved through the
    /// profile catalog, so the canonical `f5-irules` and every registered
    /// alias (`irules`, `tcl-irule`) agree with the profile predicates by
    /// construction (dialect-profile-model.md §2.4). This is the single
    /// source of truth for the "is this iRules?" check that compiler / LSP
    /// passes need, replacing the per-module
    /// `matches!(dialect, Some("f5-irules" | "irules"))` copies.
    #[must_use]
    pub fn is_irules_dialect(name: Option<&str>) -> bool {
        crate::profile::DialectProfile::by_opt_name(name).is_irules()
    }

    /// Whether `name`'s ensemble commands are *fixed* — the dialect
    /// ships a closed set of subcommands with no user-extensible
    /// ensembles — so the minifier may safely shorten subcommands to
    /// their unambiguous prefix.  True for the F5 dialect family
    /// (`f5-irules` / `f5-iapps` / `f5-bigip`) — resolved through the
    /// profile catalog, so alias spellings agree with the canonical name
    /// (§2.4; the old string table said `irules` was NOT fixed while
    /// `is_irules_dialect` said it WAS iRules — that disagreement is the
    /// defect this closes).
    #[must_use]
    pub fn has_fixed_ensembles(name: Option<&str>) -> bool {
        crate::profile::DialectProfile::by_opt_name(name).has_fixed_ensembles
    }

    /// Parse a dialect name string to a single-bit set.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "bpf" => Self::BPF,
            "tcl8.4" => Self::TCL84,
            "tcl8.5" => Self::TCL85,
            "tcl8.6" => Self::TCL86,
            "tcl9.0" => Self::TCL90,
            "tcl9.1" => Self::TCL91,
            "f5-irules" => Self::IRULES,
            "f5-iapps" => Self::IAPPS,
            "tk" => Self::TK,
            "expect" => Self::EXPECT,
            "f5-tmsh" => Self::TMSH,
            "f5-bigip" => Self::BIGIP,
            _ => return None,
        })
    }

    /// The canonical dialect name for a single-bit set — the inverse of
    /// [`Self::parse`]. `None` for an empty set, a combinator flag with no
    /// single dialect name (`ALL_TCL`, `TCL85_PLUS`, …), or a multi-bit set.
    #[must_use]
    pub fn canonical_name(self) -> Option<&'static str> {
        Some(match self {
            Self::BPF => "bpf",
            Self::TCL84 => "tcl8.4",
            Self::TCL85 => "tcl8.5",
            Self::TCL86 => "tcl8.6",
            Self::TCL90 => "tcl9.0",
            Self::TCL91 => "tcl9.1",
            Self::IRULES => "f5-irules",
            Self::IAPPS => "f5-iapps",
            Self::TK => "tk",
            Self::EXPECT => "expect",
            Self::TMSH => "f5-tmsh",
            Self::BIGIP => "f5-bigip",
            _ => return None,
        })
    }

    /// The canonical dialect names of every single bit set in `self`, in
    /// declaration order (`TCL84` before `TCL85` before … before `MENTOR`) —
    /// so a Tcl-version run always lists lowest-to-highest.  Combinator bits
    /// with no single name (see [`Self::canonical_name`]) contribute nothing;
    /// every *primitive* dialect this set contains is still listed via its
    /// own bit.
    #[must_use]
    pub fn member_names(self) -> Vec<&'static str> {
        const PRIMITIVES: &[DialectSet] = &[
            DialectSet::TCL84,
            DialectSet::TCL85,
            DialectSet::TCL86,
            DialectSet::TCL90,
            DialectSet::TCL91,
            DialectSet::IRULES,
            DialectSet::IAPPS,
            DialectSet::TK,
            DialectSet::EXPECT,
            DialectSet::BPF,
            DialectSet::TMSH,
            DialectSet::BIGIP,
        ];
        PRIMITIVES
            .iter()
            .filter(|&&bit| self.contains(bit))
            .filter_map(|&bit| bit.canonical_name())
            .collect()
    }

    /// The LOWEST Tcl version bit in this set, as a [`TclVersion`] — or
    /// `None` when the set carries no version bits (a pure vendor gate such
    /// as `EXPECT` or `IRULES`).
    ///
    /// This is the `min_dialect_version()` of the option-gating model
    /// (dialect-profile-model.md §5.2): a `TCL85_PLUS` gate has minimum
    /// `V8_5`, so it resolves under any profile whose `version_ceiling` is
    /// 8.5 or later, while a `TCL90_PLUS` gate (minimum `V9_0`) does not
    /// resolve under an 8.x ceiling.
    #[must_use]
    pub fn min_version(self) -> Option<crate::version::TclVersion> {
        use crate::version::TclVersion;
        [
            (Self::TCL84, TclVersion::V8_4),
            (Self::TCL85, TclVersion::V8_5),
            (Self::TCL86, TclVersion::V8_6),
            (Self::TCL90, TclVersion::V9_0),
            (Self::TCL91, TclVersion::V9_1),
        ]
        .into_iter()
        .find_map(|(bit, version)| self.intersects(bit).then_some(version))
    }

    /// Resolve `name` to the Tcl core-grammar version its `expr` evaluator
    /// behaves like, for version-gated *language grammar* features — e.g.
    /// TIP 201 (`in`/`ni`, 8.5+) and TIP 461 (`lt`/`le`/`gt`/`ge`, 9.0+).
    ///
    /// Distinct from [`Self::parse`] + `TCL85_PLUS`/`TCL90_PLUS`, which model
    /// per-*command* dialect gating and deliberately keep vendor dialects
    /// (the EDA tools, F5 iApps/tmsh) as their own bits rather than folding
    /// them into a Tcl version — most standard-library commands they don't
    /// ship are missing because of their own restricted command surface, not
    /// because of version. Expr grammar is different: every dialect here is
    /// either plain Tcl or a vendor shell embedding a real Tcl core, so it
    /// inherits that embedded core's operators wholesale. `f5-irules` is the
    /// one case where the two deliberately disagree: it advertises an
    /// 8.6-shaped command *signature* but its `expr` evaluator is a genuine
    /// embedded Tcl 8.4.6, so version-*dependent behaviour* (this included)
    /// follows 8.4, not 8.6.
    ///
    /// Source: the per-dialect base-Tcl-version table in
    /// `docs/design/compiler/dialects-events.md`. Returns `None` for
    /// `f5-bigip` (a custom config parser, not Tcl at all) and for any name
    /// this table has no documented base version for (`tk`, `bpf`) — callers
    /// should treat `None` as "can't reason about this dialect's grammar
    /// version," not "assume plain Tcl."
    #[must_use]
    pub fn expr_grammar_base_version(name: &str) -> Option<Self> {
        Some(match name {
            "tcl8.4" | "f5-irules" | "cadence-eda-tcl" => Self::TCL84,
            "tcl8.5" | "f5-iapps" | "f5-tmsh" | "xilinx-eda-tcl" | "intel-quartus-eda-tcl" => {
                Self::TCL85
            }
            "tcl8.6" | "synopsys-eda-tcl" | "mentor-eda-tcl" | "expect" => Self::TCL86,
            "tcl9.0" => Self::TCL90,
            "tcl9.1" => Self::TCL91,
            _ => return None,
        })
    }

    /// TIP 278: whether `name`'s runtime resolves an unqualified variable at
    /// **namespace scope** against the global namespace when the current
    /// namespace has no such variable — reads *and* writes (`set` / `unset` /
    /// `incr` / `info exists` all follow it; a declared-but-unset `variable`
    /// blocks it).  True through Tcl 8.6 (`tclVar.c` `TclLookupSimpleVar`
    /// consults the global table), removed in 9.0 (the lookup forces
    /// `TCL_NAMESPACE_ONLY`).  Enclosing (intermediate) namespaces are never
    /// consulted in *any* version — this knob only decides the global hop.
    ///
    /// Follows the dialect's *runtime* base version
    /// ([`Self::expr_grammar_base_version`]) so vendor shells inherit their
    /// embedded core's behaviour (an iRules script runs on a real 8.4).  A
    /// dialect with no documented base version gets `false`: without evidence
    /// of an 8.x core, the stricter 9.0 reading avoids inventing cross-scope
    /// references.
    #[must_use]
    pub fn namespace_var_global_fallback(name: &str) -> bool {
        Self::expr_grammar_base_version(name).is_some_and(|base| Self::TCL8X.contains(base))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespace_var_global_fallback_follows_the_runtime_base_version() {
        let f = DialectSet::namespace_var_global_fallback;
        // 8.x cores keep the TIP 278 fallback …
        for d in ["tcl8.4", "tcl8.5", "tcl8.6", "f5-irules", "expect"] {
            assert!(f(d), "{d} runs an 8.x core — fallback present");
        }
        // … 9.0 removed it …
        assert!(!f("tcl9.0"));
        assert!(!f("tcl9.1"));
        // … and an unknown base version takes the stricter 9.0 reading.
        assert!(!f("f5-bigip"));
        assert!(!f("tk"));
        assert!(!f("tcl"));
        assert!(!f(""));
    }

    #[test]
    fn available_dialects_is_sorted_and_complete() {
        let d = available_dialects();
        assert_eq!(d.len(), 16);
        let mut sorted = d.to_vec();
        sorted.sort_unstable();
        assert_eq!(d, sorted.as_slice(), "must be pre-sorted");
        // Spot-check the names that diverge from DialectSet::parse.
        assert!(d.contains(&"bpf"));
        assert!(d.contains(&"f5-bigip"));
        assert!(d.contains(&"f5-tmsh"));
        assert!(d.contains(&"tcl9.1"));
        assert!(!d.contains(&"tk"));
    }

    #[test]
    fn is_valid_nested_dialects_requires_child_subset_of_parent() {
        // Either side `None` ("all dialects") is always compatible.
        assert!(DialectSet::is_valid_nested_dialects(None, None));
        assert!(DialectSet::is_valid_nested_dialects(
            Some(DialectSet::IRULES),
            None
        ));
        assert!(DialectSet::is_valid_nested_dialects(
            None,
            Some(DialectSet::IRULES)
        ));
        // A child narrowing within the parent's set is compatible …
        assert!(DialectSet::is_valid_nested_dialects(
            Some(DialectSet::TCL86),
            Some(DialectSet::TCL86_PLUS)
        ));
        assert!(DialectSet::is_valid_nested_dialects(
            Some(DialectSet::TCL86_PLUS),
            Some(DialectSet::TCL86_PLUS)
        ));
        // … but a child claiming a dialect the parent lacks is not,
        // whether disjoint or only partially overlapping.
        assert!(!DialectSet::is_valid_nested_dialects(
            Some(DialectSet::IRULES),
            Some(DialectSet::TCL86_PLUS)
        ));
        assert!(!DialectSet::is_valid_nested_dialects(
            Some(DialectSet::TCL86 | DialectSet::IRULES),
            Some(DialectSet::TCL86_PLUS)
        ));
    }

    #[test]
    fn from_name_roundtrip() {
        assert_eq!(DialectSet::parse("tcl8.6"), Some(DialectSet::TCL86));
        assert_eq!(DialectSet::parse("f5-irules"), Some(DialectSet::IRULES));
        assert_eq!(DialectSet::parse("unknown"), None);
    }

    #[test]
    fn canonical_name_is_the_exact_inverse_of_parse() {
        // Every name `parse` accepts round-trips through `canonical_name`
        // back to the identical string, for every primitive dialect.
        for name in [
            "bpf",
            "tcl8.4",
            "tcl8.5",
            "tcl8.6",
            "tcl9.0",
            "tcl9.1",
            "f5-irules",
            "f5-iapps",
            "tk",
            "expect",
        ] {
            let bit = DialectSet::parse(name).unwrap_or_else(|| panic!("{name} must parse"));
            assert_eq!(bit.canonical_name(), Some(name));
        }
    }

    #[test]
    fn canonical_name_is_none_for_combinators_and_empty() {
        assert_eq!(DialectSet::empty().canonical_name(), None);
        assert_eq!(DialectSet::ALL_TCL.canonical_name(), None);
        assert_eq!(DialectSet::TCL85_PLUS.canonical_name(), None);
        assert_eq!(
            (DialectSet::TCL84 | DialectSet::TCL85).canonical_name(),
            None
        );
    }

    #[test]
    fn member_names_lists_lowest_tcl_version_first() {
        assert_eq!(
            DialectSet::ALL_TCL.member_names(),
            vec!["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"]
        );
        assert_eq!(
            DialectSet::TCL85_PLUS.member_names(),
            vec!["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"]
        );
        assert_eq!(DialectSet::TCL84.member_names(), vec!["tcl8.4"]);
        assert!(DialectSet::empty().member_names().is_empty());
    }

    #[test]
    fn expr_grammar_base_version_matches_documented_runtime_bases() {
        let base = DialectSet::expr_grammar_base_version;
        // Plain Tcl versions map to themselves.
        assert_eq!(base("tcl8.4"), Some(DialectSet::TCL84));
        assert_eq!(base("tcl9.1"), Some(DialectSet::TCL91));
        // iRules' *runtime* base (8.4.6) wins over its 8.6-shaped command
        // signature — version-dependent behaviour follows the runtime.
        assert_eq!(base("f5-irules"), Some(DialectSet::TCL84));
        // Cadence Innovus/Genus run an 8.4-safe core (owner decision).
        assert_eq!(base("cadence-eda-tcl"), Some(DialectSet::TCL84));
        // EDA / F5 vendor shells inherit their documented embedded core —
        // unlike `DialectSet::TCL85_PLUS`, which deliberately excludes them.
        for d in [
            "f5-iapps",
            "f5-tmsh",
            "xilinx-eda-tcl",
            "intel-quartus-eda-tcl",
        ] {
            assert_eq!(base(d), Some(DialectSet::TCL85), "{d}");
        }
        // Synopsys + modern Questa (Mentor) run an 8.6 core.
        for d in ["synopsys-eda-tcl", "mentor-eda-tcl", "expect"] {
            assert_eq!(base(d), Some(DialectSet::TCL86), "{d}");
        }
        // Not Tcl at all / no documented base version: stay `None` rather
        // than guessing.
        assert_eq!(base("f5-bigip"), None);
        assert_eq!(base("tk"), None);
        assert_eq!(base("bpf"), None);
        assert_eq!(base("nonsense"), None);
    }

    #[test]
    fn fixed_ensembles_cover_the_f5_family_only() {
        for d in ["f5-irules", "f5-iapps", "f5-bigip"] {
            assert!(
                DialectSet::has_fixed_ensembles(Some(d)),
                "{d} should be fixed"
            );
        }
        assert!(!DialectSet::has_fixed_ensembles(Some("tcl8.6")));
        // Alias spellings agree with the canonical name via the profile
        // catalog (§2.4) — the old string table wrongly said `irules` was
        // NOT fixed while `is_irules_dialect` said it WAS iRules.
        assert!(DialectSet::has_fixed_ensembles(Some("irules")));
        assert!(!DialectSet::has_fixed_ensembles(None));
    }

    #[test]
    fn is_irules_dialect_accepts_canonical_and_legacy_alias() {
        // The canonical `f5-irules` and the legacy `irules` alias both match;
        // every other dialect — and `None` — do not. There is no
        // active-dialect-wrapper case: that contextvar mechanism has no
        // global equivalent here by design.
        assert!(DialectSet::is_irules_dialect(Some("f5-irules")));
        assert!(DialectSet::is_irules_dialect(Some("irules")));
        assert!(!DialectSet::is_irules_dialect(Some("tcl8.6")));
        assert!(!DialectSet::is_irules_dialect(Some("f5-iapps")));
        assert!(!DialectSet::is_irules_dialect(Some("f5-bigip")));
        assert!(!DialectSet::is_irules_dialect(None));
    }

    #[test]
    fn min_version_finds_the_lowest_version_bit() {
        use crate::version::TclVersion;
        assert_eq!(DialectSet::TCL85_PLUS.min_version(), Some(TclVersion::V8_5));
        assert_eq!(DialectSet::TCL90_PLUS.min_version(), Some(TclVersion::V9_0));
        assert_eq!(DialectSet::ALL_TCL.min_version(), Some(TclVersion::V8_4));
        assert_eq!(DialectSet::TCL91.min_version(), Some(TclVersion::V9_1));
        // Pure vendor gates carry no version bits.
        assert_eq!(DialectSet::EXPECT.min_version(), None);
        assert_eq!(DialectSet::IRULES.min_version(), None);
        assert_eq!((DialectSet::IAPPS | DialectSet::EXPECT).min_version(), None);
        // Mixed gates report the lowest version bit.
        assert_eq!(
            (DialectSet::TCL86 | DialectSet::EXPECT).min_version(),
            Some(TclVersion::V8_6)
        );
        assert_eq!(DialectSet::empty().min_version(), None);
    }

    #[test]
    fn tcl85_plus_contains_86_and_90() {
        assert!(DialectSet::TCL85_PLUS.contains(DialectSet::TCL85));
        assert!(DialectSet::TCL85_PLUS.contains(DialectSet::TCL86));
        assert!(DialectSet::TCL85_PLUS.contains(DialectSet::TCL90));
        assert!(!DialectSet::TCL85_PLUS.contains(DialectSet::TCL84));
    }
}
