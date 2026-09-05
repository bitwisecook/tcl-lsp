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

use crate::DialectProfile;

mod reference_toolchains {
    include!(concat!(env!("OUT_DIR"), "/reference_toolchains.rs"));
}

use reference_toolchains::{REFERENCE_PATCHLEVELS, REFERENCE_SOURCE_TAGS};

/// How Tcl converts a Unicode string representation to binary bytes.
///
/// Tcl 8.x keeps the historic one-byte conversion: each character contributes
/// its low byte. Tcl 9 made that conversion checked, rejecting a code point
/// above `U+00FF`. A byte-array object itself is unaffected: its dedicated
/// byte payload is always returned verbatim. This policy concerns the separate
/// string-to-bytes shimmer used by commands such as `binary encode` and
/// `binary scan`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteStringEncoding {
    /// Tcl 8.x's legacy low-byte conversion.
    LegacyTruncate,
    /// Tcl 9.x's checked conversion, which rejects code points above `U+00FF`.
    CheckedLatin1,
}

/// Character-counting model used by Tcl string operations.
///
/// Tcl 8 stores `Tcl_UniChar` as a 16-bit unit, so a supplementary-plane
/// character occupies a surrogate pair and contributes two to `string
/// length`. Tcl 9 widened `Tcl_UniChar` and counts Unicode scalar values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringCharacterModel {
    /// Tcl 8.x: count UTF-16 code units.
    Utf16CodeUnits,
    /// Tcl 9.x: count Unicode scalar values.
    UnicodeScalars,
}

impl StringCharacterModel {
    /// The number of Tcl characters `value` holds under this model.
    ///
    /// The one place the counting rule lives, so a compile-time fold and a
    /// runtime `string length` cannot drift apart.
    #[must_use]
    pub fn count(self, value: &str) -> usize {
        match self {
            Self::Utf16CodeUnits => value.encode_utf16().count(),
            Self::UnicodeScalars => value.chars().count(),
        }
    }

    /// The character count `model` defines, or — when no release is selected —
    /// the count both models agree on, if they agree.
    ///
    /// A dialect that names no runtime release still counts every string
    /// outside the supplementary planes identically under both models, so a
    /// consumer keeps those answers and gives up only the genuinely ambiguous
    /// ones rather than declining wholesale.
    #[must_use]
    pub fn count_for(model: Option<Self>, value: &str) -> Option<usize> {
        if let Some(model) = model {
            return Some(model.count(value));
        }
        let scalars = Self::UnicodeScalars.count(value);
        (scalars == Self::Utf16CodeUnits.count(value)).then_some(scalars)
    }
}

/// One package a bare interpreter pre-provides for the core itself
/// ([`TclVersion::core_provided_packages`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CorePackage {
    /// The `package provide` name, spelled as C registers it.
    pub name: &'static str,
    /// The version C provides it at.
    pub version: &'static str,
    /// Whether C also registers a `package ifneeded` stub for it, so the
    /// name appears in `package versions`.
    ///
    /// Measured: only the `TclOO` spellings carry one — `package ifneeded
    /// TclOO 1.3.1` is `# Already present, OK?` on `tclsh9.0` while
    /// `package versions Tcl` is empty on every release (oo-0.9).
    pub ifneeded_stub: bool,
}

impl CorePackage {
    /// A core entry C provides but registers no `ifneeded` stub for
    /// (`Tcl`, `tcl`).
    const fn core(name: &'static str, version: &'static str) -> Self {
        Self {
            name,
            version,
            ifneeded_stub: false,
        }
    }

    /// A `TclOO` entry, which C's `initScript` also gives an `ifneeded` stub.
    const fn tcloo(name: &'static str, version: &'static str) -> Self {
        Self {
            name,
            version,
            ifneeded_stub: true,
        }
    }
}

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
    /// Every Tcl release line modelled by this enum, in release order.
    ///
    /// Consumers that need to enumerate supported releases (for example, to
    /// locate release-namespaced persisted data) must use this table rather
    /// than rebuilding the vocabulary beside their own parser.
    pub const ALL: [Self; 5] = [Self::V8_4, Self::V8_5, Self::V8_6, Self::V9_0, Self::V9_1];

    const fn reference_index(self) -> usize {
        match self {
            Self::V8_4 => 0,
            Self::V8_5 => 1,
            Self::V8_6 => 2,
            Self::V9_0 => 3,
            Self::V9_1 => 4,
        }
    }

    /// Compatibility parser for a dialect name at an external boundary.
    ///
    /// Typed compiler and registry paths use [`Self::from_profile`]. An
    /// unversioned (`"tcl"`), non-Tcl (`"f5-irules"`), or unknown name has no
    /// fold version, so versioned folds return only their invariant subset.
    #[must_use]
    pub fn from_dialect(dialect: Option<&str>) -> Option<Self> {
        dialect
            .and_then(DialectProfile::find)
            .and_then(Self::from_profile)
    }

    /// Return the release fact represented by an already-resolved profile.
    #[must_use]
    pub fn from_profile(profile: &DialectProfile) -> Option<Self> {
        match profile.name {
            "tcl8.4" => Some(Self::V8_4),
            "tcl8.5" => Some(Self::V8_5),
            "tcl8.6" => Some(Self::V8_6),
            "tcl9.0" => Some(Self::V9_0),
            // 9.1 must not fall through to `None` (which degrades a versioned
            // fold to the dialect-invariant subset) — it behaves as 9.0+.
            "tcl9.1" => Some(Self::V9_1),
            _ => None,
        }
    }

    /// Map a `package require Tcl` version string (`"8.6"`, `"9.0"`,
    /// `"8.6.10"`) to the release enum, or `None` for anything the enum
    /// does not model — callers treat `None` as "no floor", never as an
    /// error.
    #[must_use]
    pub fn from_package_version(version: &str) -> Option<Self> {
        let mut parts = version.split('.');
        let major = parts.next()?.parse::<u32>().ok()?;
        let minor = parts.next().and_then(|m| m.parse::<u32>().ok())?;
        match (major, minor) {
            (8, 4) => Some(Self::V8_4),
            (8, 5) => Some(Self::V8_5),
            (8, 6) => Some(Self::V8_6),
            (9, 0) => Some(Self::V9_0),
            (9, 1) => Some(Self::V9_1),
            _ => None,
        }
    }

    /// The `major.minor` string this release reports as
    /// `[package provide Tcl]`.
    ///
    /// The real interpreter reports a full patchlevel (`9.0.4`), but every
    /// requirement form compares major-then-minor first, so the two-component
    /// form answers identically for any requirement that does not name a patch
    /// level — and the enum models no patch levels to name.
    #[must_use]
    pub fn version_string(self) -> &'static str {
        match self {
            Self::V8_4 => "8.4",
            Self::V8_5 => "8.5",
            Self::V8_6 => "8.6",
            Self::V9_0 => "9.0",
            Self::V9_1 => "9.1",
        }
    }

    /// The release line `text` spells, if it is one of the ladder's own.
    ///
    /// The inverse of [`Self::version_string`], for reading a compiled
    /// surface window's bound back onto the enum.
    #[must_use]
    pub fn from_version_string(text: &str) -> Option<Self> {
        [Self::V8_4, Self::V8_5, Self::V8_6, Self::V9_0, Self::V9_1]
            .into_iter()
            .find(|version| version.version_string() == text)
    }

    /// The canonical plain-Tcl dialect profile for this release line.
    ///
    /// This is the bridge from an engine's runtime-version setting to the
    /// profile-owned expression grammar and command surface. Keeping it beside
    /// the release vocabulary avoids every consumer rebuilding a versioned
    /// dialect name when it needs registry data.
    #[must_use]
    pub const fn dialect_profile_name(self) -> &'static str {
        match self {
            Self::V8_4 => "tcl8.4",
            Self::V8_5 => "tcl8.5",
            Self::V8_6 => "tcl8.6",
            Self::V9_0 => "tcl9.0",
            Self::V9_1 => "tcl9.1",
        }
    }

    /// The full `major.minor.patch` string an engine emulating this release
    /// reports as `[info patchlevel]` / `$tcl_patchLevel`.
    ///
    /// [`Self::version_string`] answers the two-component question every
    /// `package vsatisfies` comparison actually asks; this answers the
    /// *reporting* question, which needs a third component because C Tcl
    /// always has one and scripts parse it.  The patch digit names the
    /// upstream release each of this project's engines was re-derived from —
    /// the tarballs under `tmp/` — so `tcl-vm` and `runtime/rust` cannot
    /// report different patch levels for the same emulated release by each
    /// keeping their own table (issue #1328's centralisation finding).
    ///
    /// A release line the engines have no pinned reference build for reports
    /// `.0`, which is the honest answer: the line's semantics are modelled,
    /// no specific build is.
    ///
    /// 9.1's reference build is a *beta*, and C spells its patch level
    /// `9.1b0` — a two-component version with a beta suffix, not a third
    /// numeric component (`tclsh9.1`: `info patchlevel` → `9.1b0`,
    /// `::tcl::build-info patchlevel` → `9.1b0`). `package vsatisfies 9.1b0
    /// 9.1` is `1` on every release that can parse the string (8.5+), so the
    /// suffix is a legitimate version, not a display decoration.
    #[must_use]
    pub fn patchlevel(self) -> &'static str {
        REFERENCE_PATCHLEVELS[self.reference_index()]
    }

    /// Upstream Tcl/Tk source tag pinned for this release's conformance
    /// toolchain.
    ///
    /// The language-neutral manifest beside this crate owns the tag together
    /// with [`Self::patchlevel`]. Test discovery and shell setup consume this
    /// release fact instead of maintaining their own Tcl/Tk tag maps.
    #[must_use]
    pub fn reference_source_tag(self) -> &'static str {
        REFERENCE_SOURCE_TAGS[self.reference_index()]
    }

    /// The core packages a bare interpreter of this release pre-provides —
    /// restricted to the ones this project's engines actually implement,
    /// because `package provide` is a promise that `package require` will
    /// hand back a working command surface.
    ///
    /// Measured on the reference interpreters (`package names` in a fresh
    /// `tclsh`, then `package provide`/`package ifneeded` for each):
    ///
    /// | release | `Tcl` | `tcl` | `TclOO` | `tcl::oo` |
    /// |---|---|---|---|---|
    /// | 8.4.20 | `8.4` | — | — | — |
    /// | 8.5.19 | `8.5.19` | — | — | — |
    /// | 8.6.14 | `8.6.14` | — | `1.1.0` | — |
    /// | 9.0.4 | `9.0.4` | `9.0.4` | `1.3.1` | `1.3.1` |
    /// | 9.1b0 | `9.1b0` | `9.1b0` | `1.3.1` | `1.3.1` |
    ///
    /// Three release facts sit in that table, and every one of them changes
    /// what a script sees:
    ///
    /// - The lowercase `tcl` spelling arrives with Tcl 9 (TIP 590's
    ///   all-lowercase naming). Library code such as `tm.tcl` reads
    ///   `[package provide tcl]`, so an 8.x engine that provides it takes a
    ///   different branch than `tclsh8.6` does.
    /// - Tcl 8.4 provides `TCL_VERSION`, not `TCL_PATCH_LEVEL`
    ///   (`Tcl_CreateInterp` passes `TCL_VERSION` to `Tcl_PkgProvideEx`), so
    ///   `tclsh8.4` answers `package provide Tcl` with the two-component
    ///   `8.4`. Every later release answers with the patch level.
    /// - `TclOO` is a separate 8.5-era extension: it is not pre-provided
    ///   before 8.6, and 8.6 carries `1.1.0` against 9.x's `1.3.1`, with no
    ///   lowercase `tcl::oo` co-provide before 9.
    ///
    /// The `tcl::tommath`, `zlib`, and `tcl::zlib` entries the reference
    /// interpreters also pre-provide are deliberately absent: neither engine
    /// implements those surfaces, and claiming them would turn a
    /// `package require` failure into a later `invalid command name`.
    ///
    /// The `Tcl` version below is always [`Self::patchlevel`] (8.4 aside),
    /// which is the engines' *pinned* build — 8.6.16 — while the 8.6
    /// interpreter the row above was measured on is 8.6.14. The patch digit
    /// is the one thing in the table that names a build rather than a
    /// release rule; `core_package_tracks_the_patch_level` pins the
    /// relationship so the two cannot drift apart.
    #[must_use]
    pub const fn core_provided_packages(self) -> &'static [CorePackage] {
        const TCL_8_4: &[CorePackage] = &[CorePackage::core("Tcl", "8.4")];
        const TCL_8_5: &[CorePackage] = &[CorePackage::core("Tcl", REFERENCE_PATCHLEVELS[1])];
        const TCL_8_6: &[CorePackage] = &[
            CorePackage::core("Tcl", REFERENCE_PATCHLEVELS[2]),
            CorePackage::tcloo("TclOO", "1.1.0"),
        ];
        const TCL_9_0: &[CorePackage] = &[
            CorePackage::core("Tcl", REFERENCE_PATCHLEVELS[3]),
            CorePackage::core("tcl", REFERENCE_PATCHLEVELS[3]),
            CorePackage::tcloo("TclOO", "1.3.1"),
            CorePackage::tcloo("tcl::oo", "1.3.1"),
        ];
        const TCL_9_1: &[CorePackage] = &[
            CorePackage::core("Tcl", REFERENCE_PATCHLEVELS[4]),
            CorePackage::core("tcl", REFERENCE_PATCHLEVELS[4]),
            CorePackage::tcloo("TclOO", "1.3.1"),
            CorePackage::tcloo("tcl::oo", "1.3.1"),
        ];
        match self {
            Self::V8_4 => TCL_8_4,
            Self::V8_5 => TCL_8_5,
            Self::V8_6 => TCL_8_6,
            Self::V9_0 => TCL_9_0,
            Self::V9_1 => TCL_9_1,
        }
    }

    /// Whether this core retains Tcl 8's namespace-scope lookup fallback.
    ///
    /// At namespace scope, an unqualified variable name first probes the
    /// current namespace. Through Tcl 8.6 it then probes the global namespace;
    /// Tcl 9.0 removed that second probe (TIP 278). This belongs to the
    /// version-profile vocabulary so runtimes do not each encode a release
    /// comparison beside their variable resolver.
    #[must_use]
    pub const fn namespace_var_global_fallback(self) -> bool {
        matches!(self, Self::V8_4 | Self::V8_5 | Self::V8_6)
    }

    /// Whether variable traces recover an array element the access spelling
    /// does not name.
    ///
    /// Tcl 9.0 added it in three places at once — `TclVarFindHiddenArray`
    /// (`tclInt.h` 9.0.4:866), the `part2` recovery in `TclCallVarTraces`
    /// (`tclTrace.c` 9.0.4:2560-2565) and the same recovery in
    /// `UnsetVarStruct` (`tclVar.c` 9.0.4:2638-2642); 8.4/8.5/8.6 have none of
    /// them. Two consequences are visible from script: an alias into an element
    /// (`upvar #0 a(k) e; set e 5`) fires the containing array's traces too and
    /// reports `name2 = k`, and `unset a(k)` reports `name1 = a(k)` rather than
    /// the split-off base. Measured on tclsh 8.4.20, 8.5.19, 8.6.16 (off) and
    /// 9.0.4, 9.1b0 (on). This belongs to the version-profile vocabulary so
    /// runtimes do not each encode a release comparison beside their trace
    /// dispatcher.
    #[must_use]
    pub const fn traces_recover_linked_array_element(self) -> bool {
        matches!(self, Self::V9_0 | Self::V9_1)
    }

    /// The release-defined conversion used when a string is consumed as raw
    /// binary data. See [`ByteStringEncoding`] for the Tcl 8/Tcl 9 split.
    #[must_use]
    pub const fn byte_string_encoding(self) -> ByteStringEncoding {
        match self {
            Self::V8_4 | Self::V8_5 | Self::V8_6 => ByteStringEncoding::LegacyTruncate,
            Self::V9_0 | Self::V9_1 => ByteStringEncoding::CheckedLatin1,
        }
    }

    /// The release-defined unit used by `string length` and character indices.
    #[must_use]
    pub const fn string_character_model(self) -> StringCharacterModel {
        match self {
            Self::V8_4 | Self::V8_5 | Self::V8_6 => StringCharacterModel::Utf16CodeUnits,
            Self::V9_0 | Self::V9_1 => StringCharacterModel::UnicodeScalars,
        }
    }

    /// The canonical dialect-profile name for this release (`"tcl8.6"`), the
    /// inverse of [`Self::from_dialect`].
    ///
    /// Lets an embedder that selected a release (a `--tcl-version` flag) name
    /// the same dialect to the *compiler*, so codegen targets the release the
    /// runtime was built for instead of defaulting to the permissive profile.
    #[must_use]
    pub const fn dialect_name(self) -> &'static str {
        match self {
            Self::V8_4 => "tcl8.4",
            Self::V8_5 => "tcl8.5",
            Self::V8_6 => "tcl8.6",
            Self::V9_0 => "tcl9.0",
            Self::V9_1 => "tcl9.1",
        }
    }

    /// The release's numeric-literal grammar — the single mapping every
    /// consumer of [`crate::NumberSyntax`] derives from, so the runtime, the
    /// compiler's const-folder, the analyser and the lexer grammar cannot
    /// disagree about which release accepts what.
    ///
    /// `0b`/`0o` arrive in 8.5 and `0d` / `_` separators in 9.0, which also
    /// drops octal-by-leading-zero; see [`crate::NumberSyntax`] for the
    /// per-release evidence.
    #[must_use]
    pub const fn number_syntax(self) -> crate::NumberSyntax {
        match self {
            Self::V8_4 => crate::NumberSyntax::Tcl84,
            Self::V8_5 | Self::V8_6 => crate::NumberSyntax::Tcl85,
            Self::V9_0 | Self::V9_1 => crate::NumberSyntax::Tcl90,
        }
    }

    /// Whether the core implements `namespace path` (introduced in Tcl 8.5).
    #[must_use]
    pub const fn has_namespace_path(self) -> bool {
        !matches!(self, Self::V8_4)
    }

    /// Does this release **definitely** satisfy any of `requirements` — the
    /// answer `package vsatisfies [package provide Tcl] REQ ?REQ …?` gives on
    /// every build of the release line?
    ///
    /// The decided half of [`Self::satisfies_any_ternary`]: a requirement the
    /// line's builds disagree about ([`Ternary::Inert`]) is `false` here, so
    /// this never claims more than the ternary does.  A caller that needs to
    /// tell "no" from "cannot say" — every consumer that abstains — must ask
    /// the ternary.
    ///
    /// An empty `requirements` list is `false`: real `package vsatisfies`
    /// rejects it as a wrong-argument-count error, and no caller here has a
    /// meaningful "satisfies nothing" question to ask.
    #[must_use]
    pub fn satisfies_any<S: AsRef<str>>(self, requirements: &[S]) -> bool {
        self.satisfies_any_ternary(requirements) == Ternary::Yes
    }

    /// [`Self::satisfies_any`], but [`Ternary::Inert`] when the answer really
    /// does depend on a **patch level** this enum does not model.
    ///
    /// [`TclVersion`] names a release *line*, not a build: `V9_0` stands for
    /// every `9.0.x` the user might be running.  So the honest question is not
    /// "does `9.0` satisfy this requirement" but "does **every** member of the
    /// line satisfy it, **none** of them, or some" — a requirement's satisfying
    /// set is a version interval (`package(n)`: `min` … `max`), and so is a
    /// release line, so the three answers are decided by two endpoint tests
    /// plus one containment test (issue #1126 item 3):
    ///
    /// | line vs requirement interval | answer |
    /// |---|---|
    /// | line ⊆ requirement | [`Ternary::Yes`] |
    /// | line ∩ requirement = ∅ | [`Ternary::No`] |
    /// | otherwise | [`Ternary::Inert`] |
    ///
    /// This replaces the earlier "any bound with three components is
    /// undecidable" rule, which abstained on requirements the line's *major*
    /// alone settles.  Both directions gained precision, and both are
    /// oracle-checked — `package vsatisfies` takes the candidate version as an
    /// argument, so one interpreter answers for every line, and
    /// `tests/data/package_version_oracle.txt` pins the operator itself
    /// byte-identical on 8.6.14 and 9.0.4:
    ///
    // tclsh-proof: `package vsatisfies 8.6.14 9.0.1` → 0, and so does every
    // other 8.6.x — no member of the 8.6 line can reach a 9.0 bound.
    /// * `9.0.1` against `V8_6` was `Inert`, is now `No` — no 8.6.x reaches a
    ///   9.0 bound however the patch level lands.
    // tclsh-proof: `package vsatisfies 8.6.14 8.6-8.6` → 0 while
    // `package vsatisfies 8.6 8.6-8.6` → 1: the degenerate (`-exact`) range
    // accepts only the bound itself, so a patch release of the line fails it.
    /// * `8.6-8.6` (what `package require -exact 8.6` builds) against `V8_6`
    ///   was `Yes`, is now `Inert` — `-exact 8.6` accepts `8.6`/`8.6.0` and
    ///   rejects `8.6.14`, so which it is depends on the running build.
    ///
    /// The OR short-circuits exactly as `package vsatisfies` does: a
    /// requirement the whole line satisfies settles the test [`Ternary::Yes`]
    /// however many undecidable ones sit beside it, and `No` is only reached
    /// when every requirement is refused by every member of the line.
    ///
    /// An empty `requirements` list is [`Ternary::No`], matching
    /// [`Self::satisfies_any`].
    #[must_use]
    pub fn satisfies_any_ternary<S: AsRef<str>>(self, requirements: &[S]) -> Ternary {
        let mut undecidable = false;
        for requirement in requirements {
            match self.satisfies_ternary(requirement.as_ref()) {
                Ternary::Yes => return Ternary::Yes,
                Ternary::Inert => undecidable = true,
                Ternary::No => {}
            }
        }
        if undecidable {
            Ternary::Inert
        } else {
            Ternary::No
        }
    }

    /// One requirement of [`Self::satisfies_any_ternary`], evaluated over the
    /// whole release line rather than against a single version string.
    ///
    /// The line is the closed interval `[M.m, M.m.PATCH_CEILING]`
    /// ([`PATCH_CEILING`]); `M.m` and `M.m.0` are the same version to the
    /// comparator (trailing zero components are not significant), so the low
    /// endpoint is the line's first release.
    ///
    /// A requirement's satisfying set is an interval too, which is what makes
    /// two endpoint tests sufficient for the `Yes` / `Inert` split: if both
    /// ends of the line satisfy it, so does everything between them.  When
    /// *neither* end does, the requirement interval is either entirely outside
    /// the line or strictly inside it, and the discriminator is whether the
    /// requirement's own **min bound** — always the first version its interval
    /// admits — falls within the line.
    ///
    /// A malformed requirement is [`Ternary::No`], the same conservative
    /// reading [`version_satisfies`] takes (real Tcl raises instead).
    fn satisfies_ternary(self, requirement: &str) -> Ternary {
        let low = self.version_string();
        let high = format!("{low}.{PATCH_CEILING}");
        match (
            version_satisfies(low, requirement),
            version_satisfies(&high, requirement),
        ) {
            (true, true) => Ternary::Yes,
            (false, false) => {
                let min = requirement
                    .split_once('-')
                    .map_or(requirement, |(min, _)| min);
                if ParsedVersion::parse(min).is_some()
                    && compare_versions(min, low) != core::cmp::Ordering::Less
                    && compare_versions(min, &high) != core::cmp::Ordering::Greater
                {
                    // The requirement admits only versions inside this line —
                    // some patch releases satisfy it, the endpoints do not.
                    Ternary::Inert
                } else {
                    Ternary::No
                }
            }
            _ => Ternary::Inert,
        }
    }
}

/// The patch component standing in for "the last release this `major.minor`
/// line will ever carry", the upper endpoint
/// [`TclVersion::satisfies_ternary`] tests.
///
/// Version components compare by digit count first and are never parsed into a
/// fixed integer width ([`Segment::cmp`]), so a twenty-digit component is
/// simply larger than every component any requirement or release realistically
/// writes — including the deeper components of a four-part version
/// (`8.6.16.2` orders below `8.6.<ceiling>` at the third component).
const PATCH_CEILING: &str = "99999999999999999999";

// The package version comparator — a port of C Tcl's `generic/tclPkg.c`.
//
// One implementation, shared by the bytecode VM's `package vcompare` /
// `vsatisfies` / `require`, the language server's `pkgIndex.tcl` guard
// evaluation, and the package resolver's provider selection, so none of them
// can disagree about what a version or a requirement means.
//
// The three C functions this mirrors, and where each lands here:
//
// | `tclPkg.c`                | here                       |
// |---------------------------|----------------------------|
// | `CheckVersionAndConvert`  | [`ParsedVersion::parse`]   |
// | `CompareVersions`         | [`compare_internal`]       |
// | `RequirementSatisfied`    | [`satisfies_internal`]     |
// | `SelectPackage` (best/best-stable loop) | [`select_package_version`] |

/// One element of a version's internal representation.
///
/// `CheckVersionAndConvert` rewrites a version into a space-separated list of
/// numbers in which every separator becomes a number of its own: `.` → `0`,
/// `a` → `-2`, `b` → `-1`.  Encoding the separator as a *number* is what makes
/// an alpha/beta release order below the dotted patch release at the same
/// position without a second comparison rule.
///
/// The numeric components are kept as **digit strings**, not integers, because
/// `CompareVersions` does not compute a numeric value either — its comment
/// says so outright:
///
/// > Rewritten to not compute a numeric value for the extracted version
/// > number, but do string comparison. Skip any leading zeros for that to
/// > work. This change breaks through the 32bit-limit on version numbers.
///
/// So a component of any length compares exactly: `package vcompare
/// 9223372036854775807 9223372036854775808` is `-1` on both interpreters, and
/// a forty-digit component still orders correctly.  Parsing into any fixed
/// integer width would collapse everything past that width into one value
/// (issue #1090 review, finding 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Segment<'v> {
    /// The `a` (alpha) separator — C's `-2`, below every other segment.
    Alpha,
    /// The `b` (beta) separator — C's `-1`.
    Beta,
    /// A numeric component, **leading zeros already stripped**, so `""` is
    /// zero and `"0005"` is held as `"5"`.  C strips them by advancing past
    /// `'0'` before measuring the run, which is also why a literal `0`
    /// component and a component that simply ran out compare equal.
    Number(&'v str),
}

/// Zero — the value a version that has run out of components compares as.
///
/// C reaches the same state by running off the end of the shorter string: the
/// leading-zero skip leaves an empty run, which then ties with a literal `0`,
/// sorts below any longer digit run, and sorts above the negative separator
/// markers via the sign shortcut.
const ZERO: Segment<'static> = Segment::Number("");

impl Segment<'_> {
    /// The `CompareVersions` per-segment rule.
    ///
    /// * A separator marker is negative in C, so it loses against any number
    ///   (the sign shortcut), and `a` (`-2`) loses to `b` (`-1`) — C compares
    ///   the two magnitudes and flips the result.
    /// * Two numbers compare by **digit count first**, then lexicographically
    ///   ("shorter string is smaller number", then `strcmp` on equal lengths).
    ///   With leading zeros stripped that is exact numeric ordering at any
    ///   width.
    fn cmp(self, other: Self) -> core::cmp::Ordering {
        use core::cmp::Ordering;
        match (self, other) {
            (Self::Alpha, Self::Alpha) | (Self::Beta, Self::Beta) => Ordering::Equal,
            (Self::Alpha, _) | (Self::Beta, Self::Number(_)) => Ordering::Less,
            (_, Self::Alpha) | (Self::Number(_), Self::Beta) => Ordering::Greater,
            (Self::Number(a), Self::Number(b)) => a.len().cmp(&b.len()).then_with(|| a.cmp(b)),
        }
    }
}

/// A version in C Tcl's internal representation, plus its stability flag.
///
/// `1.2` is `[1, 0, 2]`, `1.2a1` is `[1, 0, 2, a, 1]`, `1.2.3` is
/// `[1, 0, 2, 0, 3]` — the `0`s being the segments the `.` separators inject.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedVersion<'v> {
    /// The internal-rep segments.
    segments: Vec<Segment<'v>>,
    /// `false` when an `a` or `b` separator occurred — C Tcl's `hasunstable`,
    /// the flag `SelectPackage` uses to maintain its "best stable" candidate.
    stable: bool,
}

impl<'v> ParsedVersion<'v> {
    /// Validate and convert `string`, or `None` when it is not a well-formed
    /// version — a port of `CheckVersionAndConvert`'s TIP 268 rules:
    ///
    /// 1. the first character must be a digit;
    /// 2. every other character must be a digit, `.`, `a`, or `b`;
    /// 3. only one of `a` / `b` may occur;
    /// 4. neither `a`, `b`, nor `.` may sit next to a `.`;
    /// 5. the last character may not be a separator.
    ///
    /// Real Tcl raises `expected version number but got "…"` where this
    /// answers `None`; every caller here turns that into the conservative
    /// static answer (unsatisfiable / unselectable) rather than a panic.
    fn parse(string: &'v str) -> Option<Self> {
        let bytes = string.as_bytes();
        if !bytes.first().is_some_and(u8::is_ascii_digit) {
            return None;
        }
        let mut segments = Vec::new();
        let mut run_start = 0usize;
        let mut has_unstable = false;
        let mut prev = bytes[0];
        for (i, &c) in bytes.iter().enumerate().skip(1) {
            if c.is_ascii_digit() {
                prev = c;
                continue;
            }
            let separator = match c {
                b'.' => ZERO,
                b'a' => Segment::Alpha,
                b'b' => Segment::Beta,
                _ => return None,
            };
            // Rule 3 — a second `a`/`b`; rule 4 — a separator adjacent to a
            // `.`, or a `.` adjacent to any separator.
            if (has_unstable && c != b'.')
                || (matches!(prev, b'a' | b'b') && c == b'.')
                || prev == b'.'
            {
                return None;
            }
            has_unstable |= c != b'.';
            segments.push(number_segment(&string[run_start..i]));
            segments.push(separator);
            run_start = i + 1;
            prev = c;
        }
        // Rule 5 — a trailing separator.
        if matches!(prev, b'.' | b'a' | b'b') {
            return None;
        }
        segments.push(number_segment(&string[run_start..]));
        Some(Self {
            segments,
            stable: !has_unstable,
        })
    }

    /// A best-effort rep for a string [`Self::parse`] rejects, so the *total*
    /// [`compare_versions`] never has to invent an answer out of nothing: digit
    /// runs become segments, recognised separators become their markers, and
    /// any other character is skipped. Never used to decide satisfaction —
    /// only to order two strings that are not versions in the first place.
    fn lenient(string: &'v str) -> Vec<Segment<'v>> {
        let bytes = string.as_bytes();
        let mut segments = Vec::new();
        let mut run_start = 0usize;
        for (i, &c) in bytes.iter().enumerate() {
            if c.is_ascii_digit() {
                continue;
            }
            let separator = match c {
                b'.' => ZERO,
                b'a' => Segment::Alpha,
                b'b' => Segment::Beta,
                _ => {
                    // Skipped entirely: close the run before it and reopen
                    // after, so `1x2` still reads as two components.
                    segments.push(number_segment(&string[run_start..i]));
                    run_start = i + 1;
                    continue;
                }
            };
            segments.push(number_segment(&string[run_start..i]));
            segments.push(separator);
            run_start = i + 1;
        }
        segments.push(number_segment(&string[run_start..]));
        segments
    }
}

/// A digit run as a [`Segment::Number`], with C's leading-zero skip applied so
/// `"0005"`, `"5"` compare equal and `"000"`, `"0"`, `""` all read as zero.
fn number_segment(run: &str) -> Segment<'_> {
    Segment::Number(run.trim_start_matches('0'))
}

/// Compare two internal reps, returning the ordering plus whether the deciding
/// difference was in the **first** segment (C Tcl's `isMajorPtr` out-param).
///
/// A rep shorter than the other is padded with [`ZERO`] — the exact effect of
/// the C loop running off the end of the shorter string.
fn compare_internal(a: &[Segment<'_>], b: &[Segment<'_>]) -> (core::cmp::Ordering, bool) {
    use core::cmp::Ordering;
    for i in 0..a.len().max(b.len()) {
        let x = a.get(i).copied().unwrap_or(ZERO);
        let y = b.get(i).copied().unwrap_or(ZERO);
        match x.cmp(y) {
            Ordering::Equal => {}
            other => return (other, i == 0),
        }
    }
    (Ordering::Equal, false)
}

/// Compare two version numbers exactly as `package vcompare` does.
///
/// Trailing zero components are *not* significant (`1.2` == `1.2.0`), leading
/// zeros are not either (`0005` == `5`), and an alpha/beta release orders below
/// the release it leads up to but above the previous one (`1.2` < `1.2b1` is
/// false, `1.2` < `1.3b1` is true).  Numeric components compare exactly at any
/// width — `9223372036854775807` < `9223372036854775808` — because they are
/// never parsed into a fixed integer type.  All of it is pinned against
/// `tclsh8.6` and `tclsh9.0` in
/// `tcl-dialect/tests/package_version_oracle.rs`.
///
/// Total: a string that is not a well-formed version is compared through a
/// best-effort rep rather than raising, because callers order provider lists
/// that a recorder has already filtered and have no error channel.
#[must_use]
pub fn compare_versions(a: &str, b: &str) -> core::cmp::Ordering {
    let va = ParsedVersion::parse(a).map_or_else(|| ParsedVersion::lenient(a), |p| p.segments);
    let vb = ParsedVersion::parse(b).map_or_else(|| ParsedVersion::lenient(b), |p| p.segments);
    compare_internal(&va, &vb).0
}

/// Whether `version` is a **stable** release — i.e. carries no `a`/`b`
/// (alpha/beta) separator. An unparseable version is not stable.
///
/// This is C Tcl's `hasunstable` flag, the input to `package prefer`'s choice
/// between the best and the best-stable candidate.
#[must_use]
pub fn version_is_stable(version: &str) -> bool {
    ParsedVersion::parse(version).is_some_and(|p| p.stable)
}

/// Does the concrete `version` satisfy one `package vsatisfies` requirement?
///
/// The requirement forms Tcl accepts (`package(n)`, every row verified against
/// `tclsh8.6` 8.6.14 and `tclsh9.0` 9.0.4 — byte-identical):
///
/// | Written   | Means                                             | `8.6` | `9.0` |
/// |-----------|---------------------------------------------------|-------|-------|
/// | `8.5`     | `[8.5, 9)` — up to but excluding the *next major* | yes   | no    |
/// | `8.5-`    | `[8.5, ∞)`                                        | yes   | yes   |
/// | `8.5-9.0` | `[8.5, 9.0)`                                      | yes   | no    |
/// | `8.5-8.5` | exactly 8.5 (what `package require -exact` builds) | no    | no    |
///
/// Every bound that is not an exact `min-max` pair is padded with an alpha
/// segment before comparing, which is why an unstable release of the bound's
/// own version satisfies it: `package vsatisfies 1.2a1 1.2` is **1**, and so
/// is `1.2a1 1.2-1.3`. The `min == max` (exact) form is the one place that
/// padding is skipped, so `-exact 1.2` accepts `1.2` and `1.2.0` but not
/// `1.2a1`.
///
/// An ill-formed version or requirement — which real Tcl raises on — answers
/// `false`: the conservative static reading, since nothing can be shown to
/// satisfy a requirement that cannot be parsed. Nothing is trimmed first, so
/// `" 1.2"` is rejected exactly as the interpreter rejects it.
#[must_use]
pub fn version_satisfies(version: &str, requirement: &str) -> bool {
    let Some(have) = ParsedVersion::parse(version) else {
        return false;
    };
    satisfies_internal(&have.segments, requirement)
}

/// [`version_satisfies`] against an already-parsed candidate — the form
/// [`select_package_version`] needs so a candidate is converted once for the
/// whole requirement list.
fn satisfies_internal(have: &[Segment<'_>], requirement: &str) -> bool {
    use core::cmp::Ordering;
    let Some((lo, hi)) = requirement.split_once('-') else {
        // No dash: a simple version. The requirement is padded with an alpha
        // segment, and the candidate must be equal or greater *without* the
        // difference landing in the major component — which is what bounds a
        // bare `X.Y` at the next major without naming an upper bound.
        let Some(mut req) = ParsedVersion::parse(requirement).map(|p| p.segments) else {
            return false;
        };
        req.push(Segment::Alpha);
        let (ord, is_major) = compare_internal(have, &req);
        return ord == Ordering::Equal || (ord == Ordering::Greater && !is_major);
    };
    // `CheckRequirement`: at most one dash.
    if hi.contains('-') {
        return false;
    }
    let Some(min) = ParsedVersion::parse(lo).map(|p| p.segments) else {
        return false;
    };
    if hi.is_empty() {
        // `min-` — open-ended above.
        let mut min = min;
        min.push(Segment::Alpha);
        return compare_internal(have, &min).0 != Ordering::Less;
    }
    let Some(max) = ParsedVersion::parse(hi).map(|p| p.segments) else {
        return false;
    };
    if compare_internal(&min, &max).0 == Ordering::Equal {
        // `v-v` — the exact form. Compared unpadded, so an alpha/beta release
        // of `v` does *not* satisfy it.
        return compare_internal(&min, have).0 == Ordering::Equal;
    }
    let (mut min, mut max) = (min, max);
    min.push(Segment::Alpha);
    max.push(Segment::Alpha);
    compare_internal(&min, have).0 != Ordering::Greater
        && compare_internal(have, &max).0 == Ordering::Less
}

/// The requirement string `package require -exact NAME VERSION` builds:
/// `VERSION-VERSION` (`tclPkg.c`'s `PKG_REQUIRE` arm, which appends `"-"` and
/// the version to itself before handing the result to the ordinary
/// requirement machinery).
///
/// Modelling `-exact` as a requirement rather than a separate code path is why
/// there is exactly one satisfaction rule: the degenerate `min == max` range
/// *is* exactness.
#[must_use]
pub fn exact_requirement(version: &str) -> String {
    format!("{version}-{version}")
}

/// Which of the two best candidates `package require` takes — C Tcl's
/// `package prefer` interpreter state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PackagePrefer {
    /// Take the highest acceptable version, stable or not.
    Latest,
    /// Take the highest acceptable **stable** version, falling back to the
    /// highest acceptable version when nothing acceptable is stable.
    ///
    /// C Tcl's default (`tclsh8.6` and `tclsh9.0` both answer `stable` to a
    /// bare `package prefer`), unless `TCL_PKG_PREFER_LATEST` is set in the
    /// environment.
    #[default]
    Stable,
}

/// The index of the `package ifneeded` candidate `package require` would
/// select from `available` — a port of `SelectPackage`'s scan.
///
/// * A candidate whose version does not parse is skipped, exactly as
///   `SelectPackage` skips one `CheckVersionAndConvert` rejects.
/// * With a non-empty `requirements`, a candidate must satisfy **at least
///   one** of them (`SomeRequirementSatisfied` — the requirement list is an
///   OR, so `package require widget 1.2 2.0` accepts either range).
/// * An empty `requirements` accepts every candidate. This is the
///   *unconstrained* `package require NAME`, which still picks the best
///   version — not the first registered one.
/// * Two candidates with equal versions keep the **earlier** index, because C
///   replaces its running best only on a strictly greater version.
///
/// Returns `None` when nothing is acceptable — the `can't find package NAME …`
/// error case.
#[must_use]
pub fn select_package_version<S: AsRef<str>>(
    available: &[S],
    requirements: &[&str],
    prefer: PackagePrefer,
) -> Option<usize> {
    use core::cmp::Ordering;
    let mut best: Option<(usize, Vec<Segment<'_>>)> = None;
    let mut best_stable: Option<(usize, Vec<Segment<'_>>)> = None;
    for (i, candidate) in available.iter().enumerate() {
        let Some(parsed) = ParsedVersion::parse(candidate.as_ref()) else {
            continue;
        };
        if !requirements.is_empty()
            && !requirements
                .iter()
                .any(|r| satisfies_internal(&parsed.segments, r))
        {
            continue;
        }
        if best
            .as_ref()
            .is_none_or(|(_, b)| compare_internal(&parsed.segments, b).0 == Ordering::Greater)
        {
            best = Some((i, parsed.segments.clone()));
        }
        if !parsed.stable {
            continue;
        }
        if best_stable
            .as_ref()
            .is_none_or(|(_, b)| compare_internal(&parsed.segments, b).0 == Ordering::Greater)
        {
            best_stable = Some((i, parsed.segments));
        }
    }
    match prefer {
        PackagePrefer::Stable => best_stable.or(best),
        PackagePrefer::Latest => best,
    }
    .map(|(i, _)| i)
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

impl From<bool> for Ternary {
    /// A decided boolean as a [`Ternary`] — the inverse of
    /// [`Ternary::as_bool`], never producing [`Ternary::Inert`].
    fn from(value: bool) -> Self {
        if value { Self::Yes } else { Self::No }
    }
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
    use super::{StringCharacterModel, TclVersion, Ternary, exact_requirement};

    /// The core `Tcl` provide and `[info patchlevel]` are the same build
    /// fact, so the literal in [`TclVersion::core_provided_packages`] must
    /// stay equal to [`TclVersion::patchlevel`] — except at 8.4, which
    /// provides `TCL_VERSION` (`tclsh8.4`: `package provide Tcl` → `8.4`
    /// while `info patchlevel` → `8.4.20`).
    #[test]
    fn core_package_tracks_the_patch_level() {
        for version in TclVersion::ALL {
            let core = version.core_provided_packages()[0];
            assert_eq!(core.name, "Tcl", "{version:?} must provide Tcl first");
            let expected = if version == TclVersion::V8_4 {
                version.version_string()
            } else {
                version.patchlevel()
            };
            assert_eq!(core.version, expected, "{version:?} core provide");
            assert!(
                !core.ifneeded_stub,
                "{version:?}: `package versions Tcl` is empty on every release"
            );
        }
    }

    /// The lowercase `tcl` and `tcl::oo` spellings are Tcl 9 only, and
    /// `TclOO` does not exist before 8.6 — measured on the reference
    /// interpreters (8.4.20 / 8.5.19 / 8.6.14 / 9.0.4 / 9.1b0).
    #[test]
    fn lowercase_core_spellings_are_tcl9_only() {
        let names = |v: TclVersion| -> Vec<&'static str> {
            v.core_provided_packages().iter().map(|p| p.name).collect()
        };
        // TN: 8.4/8.5 pre-provide the core alone.
        assert_eq!(names(TclVersion::V8_4), ["Tcl"]);
        assert_eq!(names(TclVersion::V8_5), ["Tcl"]);
        // TP: 8.6 gains TclOO, but neither lowercase spelling.
        assert_eq!(names(TclVersion::V8_6), ["Tcl", "TclOO"]);
        // TP: 9.x co-provides both lowercase names.
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            assert_eq!(names(version), ["Tcl", "tcl", "TclOO", "tcl::oo"]);
        }
    }

    #[test]
    fn from_dialect_maps_every_versioned_tcl() {
        // Tcl9.1 must resolve to a version, not `None` (which
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

    /// Every row pinned against a live `package vsatisfies [package provide
    /// Tcl] REQ` on `tclsh8.6` (8.6.14) and `tclsh9.0` (9.0.4).
    #[test]
    fn version_satisfies_matches_the_interpreter() {
        use super::version_satisfies;
        // Bare `X.Y` is bounded by the next *major*, not the next minor.
        assert!(version_satisfies("8.6", "8.4"));
        assert!(version_satisfies("8.6", "8.5"));
        assert!(version_satisfies("8.6", "8.6"));
        assert!(!version_satisfies("8.6", "9"));
        assert!(!version_satisfies("9.0", "8.6"));
        assert!(version_satisfies("9.0", "9"));
        assert!(version_satisfies("9.0", "9.0"));
        // Open-ended.
        assert!(version_satisfies("8.6", "8.5-"));
        assert!(version_satisfies("9.0", "8.5-"));
        assert!(!version_satisfies("8.6", "9-"));
        assert!(version_satisfies("9.0", "9-"));
        // Explicit, half-open range.
        assert!(version_satisfies("8.6", "8.5-9.0"));
        assert!(!version_satisfies("9.0", "8.5-9.0"));
        // Patch levels compare component-wise.
        assert!(version_satisfies("8.5.2", "8.5-9.0"));
    }

    /// The tcllib `pkgIndex.tcl` head guard, both ways round.
    #[test]
    fn satisfies_any_is_the_multi_requirement_or() {
        // `package vsatisfies [package provide Tcl] 8.5 9` — true on both.
        assert!(TclVersion::V8_6.satisfies_any(&["8.5", "9"]));
        assert!(TclVersion::V9_0.satisfies_any(&["8.5", "9"]));
        // …but 8.4 satisfies neither requirement.
        assert!(!TclVersion::V8_4.satisfies_any(&["8.5", "9"]));
        // An empty requirement list is never satisfied.
        assert!(!TclVersion::V9_0.satisfies_any::<&str>(&[]));
    }

    /// TP — a requirement the members of a release line genuinely disagree
    /// about is undecidable, and only then.
    ///
    /// Oracle (`tclsh9.0`, `[package provide Tcl]` = 9.0.4):
    /// `vsatisfies 9.0.4 9.0.1` = 1, `vsatisfies 9.0.4 9.0.1-` = 1,
    /// `vsatisfies 9.0.4 9.0.9-` = 0 — two shipped 9.0 releases disagree, so
    /// the honest answer for the enum's `9.0` is neither.
    #[test]
    fn a_patch_level_requirement_the_line_disagrees_about_is_undecidable() {
        for requirement in ["9.0.1", "9.0.1-", "9.0-9.0.2"] {
            assert_eq!(
                TclVersion::V9_0.satisfies_any_ternary(&[requirement]),
                Ternary::Inert,
                "{requirement}"
            );
        }
    }

    /// TN — a patch-level requirement the line's *major/minor* already settles
    /// is decided, not abstained on (issue #1126 item 3).
    ///
    /// Oracle (`tclsh8.6`, `[package provide Tcl]` = 8.6.14): `vsatisfies
    /// 8.6.14 9.0.1` = 0, `vsatisfies 8.6.14 8.6.0` = 1 — and no 8.6.x
    /// disagrees with either, because the bound sits outside the line in one
    /// case and below all of it in the other.
    #[test]
    fn a_patch_level_requirement_outside_the_line_still_decides() {
        for requirement in ["9.0.1", "9.0.1-", "9.0.0-9.0.2", "8.7.0"] {
            assert_eq!(
                TclVersion::V8_6.satisfies_any_ternary(&[requirement]),
                Ternary::No,
                "{requirement}"
            );
        }
        for requirement in ["8.6.0", "8.6.0-", "8.5.1-", "8.6.0-8.7"] {
            assert_eq!(
                TclVersion::V8_6.satisfies_any_ternary(&[requirement]),
                Ternary::Yes,
                "{requirement}"
            );
        }
    }

    /// FP guard — the degenerate `V-V` range `package require -exact V`
    /// builds is *not* satisfied by the whole line.
    ///
    /// Oracle: `vsatisfies 8.6 8.6-8.6` = 1 but `vsatisfies 8.6.14 8.6-8.6` =
    /// 0 (the exact form is compared unpadded), so which answer a build gives
    /// depends on its patch level and the line as a whole abstains — where the
    /// old `major.minor`-only comparison answered a flat `Yes`.
    #[test]
    fn an_exact_requirement_on_the_line_abstains() {
        assert_eq!(
            TclVersion::V8_6.satisfies_any_ternary(&[exact_requirement("8.6")]),
            Ternary::Inert,
        );
        // …while an exact requirement naming a *different* line is still a
        // decided `No`.
        assert_eq!(
            TclVersion::V8_6.satisfies_any_ternary(&[exact_requirement("9.0")]),
            Ternary::No,
        );
        // TP — an exact requirement naming one patch release of this line is
        // undecidable: that build satisfies it, its siblings do not.
        assert_eq!(
            TclVersion::V8_6.satisfies_any_ternary(&[exact_requirement("8.6.14")]),
            Ternary::Inert,
        );
    }

    /// TN — a malformed requirement is refused rather than abstained on, the
    /// same conservative reading [`super::version_satisfies`] takes.
    #[test]
    fn a_malformed_requirement_is_not_satisfied() {
        for requirement in ["", "x", "1-2-3", " 8.6", "8.6.", "8.6c1"] {
            assert_eq!(
                TclVersion::V8_6.satisfies_any_ternary(&[requirement]),
                Ternary::No,
                "{requirement}"
            );
        }
    }

    /// TN — every two-component requirement form still decides, and the OR
    /// short-circuits on a satisfied one beside an undecidable one.
    #[test]
    fn two_component_requirements_still_decide() {
        for requirement in ["8.5", "8.5-", "8.5-9.0", "9-", "9"] {
            assert_ne!(
                TclVersion::V9_0.satisfies_any_ternary(&[requirement]),
                Ternary::Inert,
                "{requirement}"
            );
        }
        assert_eq!(
            TclVersion::V9_0.satisfies_any_ternary(&["9.0.1", "9"]),
            Ternary::Yes,
        );
        // Both requirements name the 9 line, which no 8.6.x reaches — so the
        // OR is a decided `No`, not the abstention the old three-component
        // rule produced (issue #1126 item 3).
        assert_eq!(
            TclVersion::V8_6.satisfies_any_ternary(&["9.0.1", "9"]),
            Ternary::No,
        );
        assert_eq!(TclVersion::V8_6.satisfies_any_ternary(&["9"]), Ternary::No);
        // …and an undecidable requirement beside a refused one still carries
        // the whole test to `Inert`.
        assert_eq!(
            TclVersion::V8_6.satisfies_any_ternary(&["8.6.14-", "9"]),
            Ternary::Inert,
        );
    }

    /// Per-rule TP/FP/TN/FN for the four requirement forms, issue #1090.
    ///
    /// The whole grid lives in the pinned corpus
    /// (`tests/data/package_version_oracle.txt`); these are the rows that name
    /// the rule they exercise, so a regression reads as "the min-bound rule
    /// broke" rather than "142 corpus pairs disagree".
    #[test]
    fn each_requirement_form_decides_its_own_way() {
        use super::version_satisfies as sat;
        // Min bound within the major: `[1.2, 2)`.
        assert!(sat("1.2", "1.2"), "TP — the bound itself");
        assert!(sat("1.10", "1.2"), "TP — 10 is a number, not a string tail");
        assert!(sat("1.2.3", "1.2"), "TP — a patch release of the bound");
        assert!(!sat("1.1", "1.2"), "TN — below the bound");
        assert!(!sat("2.0", "1.2"), "FP guard — the next major is excluded");
        assert!(!sat("0.9", "1.2"), "TN — a lower major");
        // …and the alpha of the bound *is* accepted (the `a0` pad), which is
        // the rule a naive `>=` comparison gets wrong in the FN direction.
        assert!(sat("1.2a1", "1.2"), "TP — `vsatisfies 1.2a1 1.2` is 1");
        assert!(!sat("1.2a1", "1.2.0"), "TN — but not of a longer bound");

        // Open-ended `min-`: no upper bound at all, so the next major counts.
        assert!(sat("2.0", "1.2-"), "TP — no major cap");
        assert!(sat("1.2a1", "1.2-"), "TP — the alpha pad applies here too");
        assert!(!sat("1.1", "1.2-"), "TN — still bounded below");

        // Half-open range `min-max`.
        assert!(sat("1.9", "1.2-2.0"), "TP — inside");
        assert!(!sat("2.0", "1.2-2.0"), "FP guard — the max is excluded");
        assert!(!sat("1.1", "1.2-2.0"), "TN — below the min");
        assert!(!sat("1.3", "1.2a1-1.3"), "FP guard — max excluded, padded");

        // Degenerate `v-v` — what `-exact` builds.
        assert!(sat("2.0", "2.0-2.0"), "TP");
        assert!(
            sat("2.0.0", "2.0-2.0"),
            "TP — trailing zeros are not a digit"
        );
        assert!(!sat("2.0a1", "2.0-2.0"), "FP guard — no alpha pad on exact");
        assert!(!sat("2.3", "2.0-2.0"), "TN — a later release");

        // Malformed: real Tcl raises, so nothing satisfies.
        for bad in ["1.2-1.3-1.4", "-1.2", "", "a-b", "1.2-x"] {
            assert!(!sat("1.2", bad), "malformed requirement {bad:?}");
        }
        for bad in ["", "1.", ".1", "1..2", "1a", "1a1b2", " 1.2"] {
            assert!(!sat(bad, "1.2"), "malformed version {bad:?}");
        }
    }

    /// `select_package_version`'s two selection axes, issue #1090: highest
    /// acceptable version, and `package prefer`'s stable-first tie-break.
    #[test]
    fn selection_takes_the_highest_acceptable_preferring_stable() {
        use super::{PackagePrefer, select_package_version as pick};
        let avail = ["1.5", "2.3", "2.0"];
        // TP — unconstrained picks the best, not index 0.
        assert_eq!(pick(&avail, &[], PackagePrefer::Stable), Some(1));
        // TP — a constraint narrows, then the best of what is left wins.
        assert_eq!(pick(&avail, &["2.0"], PackagePrefer::Stable), Some(1));
        assert_eq!(pick(&avail, &["1.2"], PackagePrefer::Stable), Some(0));
        // TP — the requirement list is an OR.
        assert_eq!(
            pick(&avail, &["1.2", "2.0"], PackagePrefer::Stable),
            Some(1)
        );
        // TN — nothing acceptable, and an empty candidate list.
        assert_eq!(pick(&avail, &["3.0"], PackagePrefer::Stable), None);
        assert_eq!(pick::<&str>(&[], &[], PackagePrefer::Stable), None);
        // TN — a candidate whose version does not parse is skipped, not
        // treated as version 0.
        assert_eq!(
            pick(&["not-a-version", "1.0"], &[], PackagePrefer::Stable),
            Some(1)
        );

        // Prefer: the prerelease is the higher version but loses by default.
        let mixed = ["1.2", "1.3b1"];
        assert_eq!(pick(&mixed, &[], PackagePrefer::Stable), Some(0));
        assert_eq!(pick(&mixed, &[], PackagePrefer::Latest), Some(1));
        // FN guard — with nothing stable to prefer, the prerelease still wins
        // under `stable`; abstaining there would resolve nothing at all.
        assert_eq!(pick(&["1.3b1"], &[], PackagePrefer::Stable), Some(0));
        // Ties keep the earlier candidate, so discovery order breaks them.
        assert_eq!(pick(&["1.0", "1.0.0"], &[], PackagePrefer::Stable), Some(0));
    }

    #[test]
    fn version_string_round_trips_through_from_package_version() {
        for v in TclVersion::ALL {
            assert_eq!(
                TclVersion::from_package_version(v.version_string()),
                Some(v)
            );
        }
    }

    #[test]
    fn release_profile_names_cover_every_runtime_choice() {
        for (version, profile) in [
            (TclVersion::V8_4, "tcl8.4"),
            (TclVersion::V8_5, "tcl8.5"),
            (TclVersion::V8_6, "tcl8.6"),
            (TclVersion::V9_0, "tcl9.0"),
            (TclVersion::V9_1, "tcl9.1"),
        ] {
            assert_eq!(version.dialect_profile_name(), profile);
        }
    }

    #[test]
    fn string_character_model_changes_at_tcl_nine() {
        assert_eq!(
            TclVersion::V8_6.string_character_model(),
            StringCharacterModel::Utf16CodeUnits
        );
        assert_eq!(
            TclVersion::V9_0.string_character_model(),
            StringCharacterModel::UnicodeScalars
        );
    }
}
