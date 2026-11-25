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

    /// Read a supported release line from a package version, including alpha,
    /// beta and patch versions. Package version validation owns the grammar.
    #[must_use]
    pub fn parse(version: &str) -> Option<Self> {
        if !validate_version(version) {
            return None;
        }
        let prefix = version.split(['a', 'b']).next()?;
        let mut parts = prefix.split('.');
        let major = parts.next()?.parse::<u32>().ok()?;
        let minor = parts.next()?.parse::<u32>().ok()?;
        Self::ALL.into_iter().find(|release| {
            let mut expected = release.version_string().split('.');
            expected.next().and_then(|part| part.parse::<u32>().ok()) == Some(major)
                && expected.next().and_then(|part| part.parse::<u32>().ok()) == Some(minor)
        })
    }

    /// The release floor named by a `package require Tcl` version.
    #[must_use]
    pub fn from_package_version(version: &str) -> Option<Self> {
        Self::parse(version)
    }

    /// The `major.minor` string this release reports as
    /// `[package provide Tcl]`.
    ///
    /// The real interpreter reports a full patchlevel (`9.0.4`), but every
    /// requirement form compares major-then-minor first, so the two-component
    /// form answers identically for any requirement that does not name a patch
    /// level — and the enum models no patch levels to name.
    #[must_use]
    pub const fn version_string(self) -> &'static str {
        self.release_facts().version_string
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
        self.release_facts().profile_name
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

    /// Source-observed core package provisions implemented by both engines.
    /// The generated reference roster supplies versions, aliases and ifneeded
    /// stubs. The projection retains interpreter and `TclOO` subjects; tommath
    /// and zlib provisions require independent engine backing.
    #[must_use]
    pub const fn core_provided_packages(self) -> &'static [CorePackage] {
        self.release_facts().core_packages
    }

    /// `TclOO`'s header- and oracle-checked version and patchlevel, absent before
    /// it joined the core. These seed the namespace's version variables.
    #[must_use]
    pub const fn tcloo_version(self) -> Option<(&'static str, &'static str)> {
        self.release_facts().tcloo_version
    }

    /// The release-defined unit used by `string length` and character indices.
    #[must_use]
    pub const fn string_character_model(self) -> StringCharacterModel {
        self.release_policy().characters
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
        self.release_policy().numbers
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod core_package_subject_tests;
