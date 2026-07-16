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

//! The versioned-library axis (design doc §7/§7.1, decision D5): which
//! library packages a dialect profile ships ambiently, and at what version
//! floor.
//!
//! Command/option specs already carry `required_package` + `min_version`
//! (the introducing package version) and gate through
//! `available_for_version` — this module adds the *profile* side: a static
//! pin per shipped library so a floor exists even when the file never
//! writes `package require`. There is deliberately **no parallel version
//! machinery**: pins resolve to the same version-string floors the
//! `package require` path produces.

use crate::version::TclVersion;

/// An externally-supplied version axis a [`LibraryVersion::Keyed`] pin
/// resolves through: the BIG-IP (TMOS) release for the F5 surfaces, the
/// EDA tool release, or the SDC standard revision.
///
/// Per **D5** the default for every key is the **oldest supported
/// version** — the conservative choice: by default only floor-version
/// commands are offered, and commands introduced later stay hidden until
/// the file/session pins a newer version. A default of "latest" would
/// silently mark genuinely-unavailable commands as known on older
/// targets; "oldest" never over-reports availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VersionKey {
    /// The BIG-IP TMOS release (F5 iRules / iApps / tmsh / config schema).
    BigipVersion,
    /// The EDA tool release (Vivado, Quartus, Design Compiler, Genus,
    /// Questa …).
    ToolVersion,
    /// The SDC (Synopsys Design Constraints) standard revision.
    SdcVersion,
}

impl VersionKey {
    /// The **oldest supported** version this key defaults to when nothing
    /// pins it explicitly (D5).
    ///
    /// - [`VersionKey::BigipVersion`] → `16.1.0`, the oldest F5-supported
    ///   TMOS release at ratification time. Registry data introduced at or
    ///   below this floor is offered by default; later introductions
    ///   (`Introduced in BIG-IP x.y.z` > 16.1.0) need an explicit pin.
    /// - [`VersionKey::ToolVersion`] / [`VersionKey::SdcVersion`] → `None`
    ///   (permissive): no registry pack carries keyed tool/SDC
    ///   introduction data yet, so there is no evidence to pick a floor
    ///   from; the first data backfill must set one.
    #[must_use]
    pub const fn default_version(self) -> Option<&'static str> {
        match self {
            Self::BigipVersion => Some("16.1.0"),
            Self::ToolVersion | Self::SdcVersion => None,
        }
    }
}

/// How a profile pins one shipped library's version (§7 "Libraries"
/// column).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryVersion {
    /// The library version follows the profile's embedded Tcl runtime —
    /// tcllib/Tk on the plain Tcl profiles (`wish` 8.6 ships Tk 8.6).
    TracksBase,
    /// A fixed shipped version (Expect `5.45.4`, Itcl `3.4`/`4.2`).
    Pinned(&'static str),
    /// Resolved through an external [`VersionKey`] (BIG-IP / EDA tool /
    /// SDC), defaulting to the key's oldest supported version (D5).
    Keyed(VersionKey),
}

/// One library a profile models: the package name spec data uses
/// (`required_package` / `tcllib_package`), its version pin, and whether
/// it is **ambient** in this profile's runtime.
///
/// Two distinct roles share the pin machinery:
///
/// - **Ambient** (`ambient: true`) — the package *is* part of the
///   modelled interpreter (the F5 command surfaces, an EDA shell's own
///   tool commands, Expect's commands in an `expect` script). It needs no
///   `package require`, never draws the missing-require diagnostic, stays
///   in the ambient completion / static-highlight surface, and the pin
///   supplies the version floor its specs' `min_version` gates compare
///   against.
/// - **Hosted** (`ambient: false`) — the package is loadable on this
///   profile (Tk / Itcl on plain Tcl) and still needs its
///   `package require`; the pin only refines the *floor* used when the
///   file requires it without a version (Tk on `tcl8.6` guarantees 8.6,
///   so an 8.7-introduced option is not offered).
///
/// Either way an explicit versioned `package require` can only *raise*
/// the floor, never lower it below the pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LibraryPin {
    /// The package name as spec data spells it.
    pub package: &'static str,
    /// The version pin.
    pub version: LibraryVersion,
    /// Whether the package is part of the modelled runtime (no
    /// `package require` needed) rather than merely hostable.
    pub ambient: bool,
}

/// Per-session/per-file overrides for the [`VersionKey`] axes — the
/// resolved `--bigip-version`-style pins. Absent keys fall back to
/// [`VersionKey::default_version`] (oldest supported, D5).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LibraryVersionOverrides {
    /// Pinned BIG-IP TMOS release, e.g. `17.1.0`.
    pub bigip_version: Option<String>,
    /// Pinned EDA tool release.
    pub tool_version: Option<String>,
    /// Pinned SDC standard revision.
    pub sdc_version: Option<String>,
}

impl LibraryVersionOverrides {
    /// The override for `key`, if any.
    #[must_use]
    pub fn get(&self, key: VersionKey) -> Option<&str> {
        match key {
            VersionKey::BigipVersion => self.bigip_version.as_deref(),
            VersionKey::ToolVersion => self.tool_version.as_deref(),
            VersionKey::SdcVersion => self.sdc_version.as_deref(),
        }
    }
}

impl TclVersion {
    /// This release as the package-version string the tracking libraries
    /// (Tk, tcllib on `TracksBase` pins) report — the floor a
    /// `min_version: "8.7"` option compares against.
    #[must_use]
    pub const fn as_package_version(self) -> &'static str {
        match self {
            Self::V8_4 => "8.4",
            Self::V8_5 => "8.5",
            Self::V8_6 => "8.6",
            Self::V9_0 => "9.0",
            Self::V9_1 => "9.1",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LibraryVersionOverrides, VersionKey};
    use crate::version::TclVersion;

    #[test]
    fn bigip_defaults_to_the_oldest_supported_tmos() {
        // D5: oldest supported, not latest — the conservative floor.
        assert_eq!(
            VersionKey::BigipVersion.default_version(),
            Some("16.1.0"),
            "BigipVersion default is the oldest F5-supported TMOS"
        );
        // No keyed tool/SDC introduction data exists in any pack yet, so
        // those keys stay permissive until the first backfill.
        assert_eq!(VersionKey::ToolVersion.default_version(), None);
        assert_eq!(VersionKey::SdcVersion.default_version(), None);
    }

    #[test]
    fn overrides_win_over_defaults() {
        let mut o = LibraryVersionOverrides::default();
        assert_eq!(o.get(VersionKey::BigipVersion), None);
        o.bigip_version = Some("17.1.0".to_owned());
        assert_eq!(o.get(VersionKey::BigipVersion), Some("17.1.0"));
    }

    #[test]
    fn tcl_versions_render_as_package_floors() {
        assert_eq!(TclVersion::V8_6.as_package_version(), "8.6");
        assert_eq!(TclVersion::V9_1.as_package_version(), "9.1");
    }
}
