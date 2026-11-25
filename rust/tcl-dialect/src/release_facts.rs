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

//! Release catalogues measured on the pinned C Tcl source builds.
//!
//! These describe the reference distribution, not a running interpreter's
//! installed packages, encoding search path or mutable command namespace.

use crate::TclVersion;

/// Source-derived and execution-checked facts for one pinned reference build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseFacts {
    /// Release line whose pinned build supplied these facts.
    pub version: TclVersion,
    /// Matching native C cache release, derived from the same release identifier.
    pub native_c_version: tcl_core_types::NativeCVersion,
    /// Canonical two-component release spelling from the reference manifest.
    pub version_string: &'static str,
    /// Plain Tcl profile for this release line.
    pub profile_name: &'static str,
    /// Full reference build version, checked against its C header and binary.
    pub patchlevel: &'static str,
    /// Upstream source tag from the reference-toolchain manifest.
    pub source_tag: &'static str,
    /// Builtin expression functions in the default reference build.
    pub math_functions: &'static [&'static str],
    /// Encoding files and builtins, checked against `encoding names`.
    pub encodings: &'static [&'static str],
    /// Name, version and ifneeded-stub presence for startup package provides.
    pub provided_packages: &'static [(&'static str, &'static str, bool)],
    /// The source-observed core and `TclOO` subset the engines already support.
    pub core_packages: &'static [crate::CorePackage],
    /// The source header and startup variables' `TclOO` version and patchlevel.
    pub tcloo_version: Option<(&'static str, &'static str)>,
    /// Directory-to-documentation-name rows in `pkgs/package.list.txt`.
    ///
    /// These are distribution mappings, not installed package provisions.
    /// Releases without that upstream file have an empty slice.
    pub bundled_packages: &'static [(&'static str, &'static str)],
    /// Qualified command names in a fresh reference interpreter, recursively.
    ///
    /// No packages or autoloadable commands are loaded for this survey.
    pub commands: &'static [&'static str],
}

include!("generated_release_facts.rs");

/// Observed command windows in the default Linux reference distribution.
///
/// This describes that reference build, not universal availability: optional
/// and platform commands may differ in another build. Windows stop at the end
/// of the supported release ladder. Rooted observed names and their exact
/// unrooted catalogue aliases are accepted, such as `::timerate` and `timerate`.
#[must_use]
pub fn reference_command_surface(name: &str) -> Option<&'static [crate::model::SpecSurface]> {
    REFERENCE_COMMAND_SURFACES
        .binary_search_by_key(&name, |row| row.0)
        .ok()
        .map(|index| REFERENCE_COMMAND_SURFACES[index].1)
}

impl TclVersion {
    /// Immutable facts from this release's pinned reference source and binary.
    #[must_use]
    pub const fn release_facts(self) -> &'static ReleaseFacts {
        &RELEASE_FACTS[self as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_catalogues_follow_the_release_ladder() {
        for version in TclVersion::ALL {
            let facts = version.release_facts();
            assert_eq!(facts.version, version);
            assert_eq!(facts.version_string, version.version_string());
            assert_eq!(facts.patchlevel, version.patchlevel());
            assert_eq!(facts.source_tag, version.reference_source_tag());
            for names in [facts.math_functions, facts.encodings, facts.commands] {
                assert!(names.windows(2).all(|pair| pair[0] < pair[1]));
            }
        }
    }

    #[test]
    fn native_release_enum_and_generated_catalogue_share_the_ladder() {
        for version in TclVersion::ALL {
            assert_eq!(
                version.release_facts().native_c_version as usize,
                version as usize
            );
        }
    }

    #[test]
    fn reference_rosters_preserve_release_boundaries() {
        let tcl86 = TclVersion::V8_6.release_facts();
        let tcl90 = TclVersion::V9_0.release_facts();
        assert!(!tcl86.commands.contains(&"::timerate"));
        assert!(tcl86.commands.contains(&"::tcl::unsupported::timerate"));
        assert!(tcl90.commands.contains(&"::timerate"));
        assert!(!tcl86.math_functions.contains(&"isfinite"));
        assert!(tcl90.math_functions.contains(&"isfinite"));
        assert!(!tcl86.encodings.contains(&"utf-32"));
        assert!(tcl90.encodings.contains(&"utf-32"));
    }

    #[test]
    fn command_surfaces_keep_observed_windows_and_root_spelling() {
        assert!(
            REFERENCE_COMMAND_SURFACES
                .windows(2)
                .all(|pair| pair[0].0 < pair[1].0)
        );
        let timerate = reference_command_surface("timerate").unwrap();
        assert_eq!(timerate, reference_command_surface("::timerate").unwrap());
        assert_eq!(timerate[0].windows, &[("9.0", Some("9.2"))]);
        assert_eq!(
            reference_command_surface("tcl::unsupported::timerate").unwrap()[0].windows,
            &[("8.6", Some("9.0"))]
        );
        let abs = reference_command_surface("tcl::mathfunc::abs").unwrap();
        assert_eq!(
            abs,
            reference_command_surface("::tcl::mathfunc::abs").unwrap()
        );
        for name in [
            "not_a_reference_command",
            "::not_a_reference_command",
            "::::timerate",
            "::tcl::::mathfunc::abs",
        ] {
            assert!(reference_command_surface(name).is_none(), "{name}");
        }
    }
}
