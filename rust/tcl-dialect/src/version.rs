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
    // tcl-lsp-no-smoke-include
    include!(concat!(env!("OUT_DIR"), "/reference_toolchains.rs"));
}

use reference_toolchains::{REFERENCE_PATCHLEVELS, REFERENCE_SOURCE_TAGS};

mod character_model;
mod engine_policies;
mod package_versions;
mod release_behaviour;

pub use character_model::{ByteStringEncoding, CharacterUnits, StringCharacterModel, UnitRun};
pub use engine_policies::{
    ConcatPolicy, DirectPackageFile, DirectPackageFileBytes, DirectPackageFileKind,
    FrameLevelPresence, IndexGrammar, IndexIntegerWidth, IndexSyntax, InterpreterProtocol,
    ListSetBounds, NamespaceImportBinding, NativeProcedureHeaderCompilation, PackageProtocol,
    ParameterGrammar, ProcessExitConversion, RegexStartGrammar, Ternary, VariableContainerModel,
    VariableLinkBinding, VariableLookupPolicy,
};
pub use package_versions::{
    PackageDiagnostic, PackagePrefer, RequirementBytesValidationError, RequirementValidationError,
    VersionOrderKey, compare_versions, compare_versions_bytes_for, compare_versions_for,
    exact_requirement, select_package_version, select_package_version_bytes_for,
    select_package_version_exact_bytes_for, select_package_version_exact_for,
    select_package_version_for, validate_requirement, validate_requirement_bytes_for,
    validate_requirement_for, validate_version, validate_version_bytes_for, validate_version_for,
    version_in_any_window, version_is_stable, version_is_stable_bytes_for, version_is_stable_for,
    version_matches_exact_bytes_for, version_matches_exact_for, version_satisfies,
    version_satisfies_bytes_for, version_satisfies_for,
};

const TCLOO_8_6: (&str, &str) = ("1.1.0", "1.1.0");
const TCLOO_9: (&str, &str) = ("1.3", "1.3.1");

/// Semantic subject of a core package row, independent of its spelling and
/// pinned reference-build version. This classifies an explicit hypothetical
/// release survey, never a current interpreter's provision database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorePackageRole {
    /// The interpreter release itself (including its authored aliases).
    InterpreterRelease,
    /// The separately versioned `TclOO` package.
    ObjectSystem,
}

/// One package a bare interpreter pre-provides for the core itself
/// ([`TclVersion::core_provided_packages`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CorePackage {
    /// Authored semantic subject; not inferred from the display name.
    pub role: CorePackageRole,
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
            role: CorePackageRole::InterpreterRelease,
        }
    }

    /// A `TclOO` entry, which C's `initScript` also gives an `ifneeded` stub.
    const fn tcloo(name: &'static str, version: &'static str) -> Self {
        Self {
            name,
            version,
            ifneeded_stub: true,
            role: CorePackageRole::ObjectSystem,
        }
    }
}

/// A specific Tcl release whose **compile-time** semantics a constant fold may
/// depend on — e.g. `string is integer` is unbounded on 9.0 but caps at
/// `2³²-1` on 8.x, and `string is wideinteger` / `entier` / `dict` and
/// `format %b` don't exist (they *raise*) before a given release.  Ordered, so
/// a fold can test `version >= TclVersion::V8_5`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    /// unversioned (`"tcl"`) or unknown name, and a profile whose release nothing
    /// measured, has no fold version, so versioned folds return only their
    /// invariant subset.
    #[must_use]
    pub fn from_dialect(dialect: Option<&str>) -> Option<Self> {
        dialect
            .and_then(DialectProfile::find)
            .and_then(Self::from_profile)
    }

    /// Return the release fact represented by an already-resolved profile: its
    /// [`DialectProfile::evaluation_point`], the release its measurement stands
    /// behind, never one read off its name.
    #[must_use]
    pub fn from_profile(profile: &DialectProfile) -> Option<Self> {
        profile.evaluation_point()
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
    /// keeping their own table.
    ///
    /// A release line the engines have no pinned reference build for reports
    /// `.0`, which is the honest answer: the line's semantics are modelled,
    /// no specific build is.
    ///
    /// A pinned pre-release is spelt the way C spells it — the 9.1 beta was
    /// `9.1b0`, a two-component version with a beta suffix rather than a third
    /// numeric component — and `package vsatisfies 9.1b0 9.1` is `1` on every
    /// release that can parse the string (8.5+), so such a suffix is a
    /// legitimate version, not a display decoration.
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
    /// | 9.1.0 | `9.1.0` | `9.1.0` | `1.3.1` | `1.3.1` |
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
    /// which is the engines' *pinned* build, while the 8.6
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
            CorePackage::tcloo("TclOO", TCLOO_8_6.1),
        ];
        const TCL_9_0: &[CorePackage] = &[
            CorePackage::core("Tcl", REFERENCE_PATCHLEVELS[3]),
            CorePackage::core("tcl", REFERENCE_PATCHLEVELS[3]),
            CorePackage::tcloo("TclOO", TCLOO_9.1),
            CorePackage::tcloo("tcl::oo", TCLOO_9.1),
        ];
        const TCL_9_1: &[CorePackage] = &[
            CorePackage::core("Tcl", REFERENCE_PATCHLEVELS[4]),
            CorePackage::core("tcl", REFERENCE_PATCHLEVELS[4]),
            CorePackage::tcloo("TclOO", TCLOO_9.1),
            CorePackage::tcloo("tcl::oo", TCLOO_9.1),
        ];
        match self {
            Self::V8_4 => TCL_8_4,
            Self::V8_5 => TCL_8_5,
            Self::V8_6 => TCL_8_6,
            Self::V9_0 => TCL_9_0,
            Self::V9_1 => TCL_9_1,
        }
    }

    /// The core's `TCLOO_VERSION` and `TCLOO_PATCHLEVEL` (`tclOO.h`), which
    /// seed `::oo::version` and `::oo::patchlevel`; `None` before `TclOO`
    /// joined the core in 8.6. 8.6 spells both `1.1.0`; 9.x has `1.3` and
    /// `1.3.1`.
    #[must_use]
    pub const fn tcloo_version(self) -> Option<(&'static str, &'static str)> {
        match self {
            Self::V8_4 | Self::V8_5 => None,
            Self::V8_6 => Some(TCLOO_8_6),
            Self::V9_0 | Self::V9_1 => Some(TCLOO_9),
        }
    }

    /// The release-defined unit used by `string length` and character indices.
    #[must_use]
    pub const fn string_character_model(self) -> StringCharacterModel {
        match self {
            // 8.4/8.5 cap a character at three UTF-8 bytes, so a
            // supplementary code point is never assembled and counts as its
            // four bytes — measured 4, where the 8.6 surrogate model says 2.
            Self::V8_4 | Self::V8_5 => StringCharacterModel::BmpCharsElseUtf8Bytes,
            Self::V8_6 => StringCharacterModel::Utf16CodeUnits,
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
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod core_package_subject_tests;
