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

//! Per-release grammars of the command kernels: `scan`'s integer width,
//! `clock`'s surface, and `binary`'s field set.
//!
//! Each axis value is a closed enum implemented natively by its kernel in
//! `tcl-cmd-core`. A release selects one value per axis through its
//! [`crate::release_policy::ReleasePolicyFacts`], measured on the pinned
//! reference builds by `cargo xtask release-facts`
//! (`docs/design/registry/model/370-dialect-grammars.md`).

/// How `scan` stores an integer conversion without an `l` size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ScanIntWidth {
    /// Tcl 8.4: C `strtol`/`strtoul` into a `long`; a value that fits 32
    /// bits unsigned is cast to a signed `int`, a wider one is kept, and a
    /// negative `%u` is spelled unsigned. `scan ffffffff %x` is `-1`.
    Wrap32Narrow,
    /// Tcl 8.5 and 8.6: the parsed number read as a C `long`, so a value
    /// within 64 bits unsigned of either sign wraps and a wider one
    /// saturates. `scan ffffffff %x` is `4294967295`.
    #[default]
    Full,
    /// Tcl 9.x: the parsed number read as a C `int`; a value outside
    /// `INT_MIN..=UINT_MAX` saturates, and `%u` reads the 32-bit result as
    /// unsigned 64-bit. `scan 1ffffffff %x` is `2147483647`.
    Saturate32,
}

/// `scan`'s grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ScanGrammar {
    /// Integer conversion width.
    pub int_width: ScanIntWidth,
}

/// `clock add`'s unit words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ClockUnits {
    /// Tcl 8.4: no `clock add`.
    None,
    /// Tcl 8.5 and 8.6: each unit's singular or plural word, no `weekdays`.
    Tcl8,
    /// Tcl 9.x: any unique prefix of the unit table, `weekdays` included.
    #[default]
    Tcl9,
}

/// `clock format` and `clock scan`'s option and specifier grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ClockFormatGrammar {
    /// Tcl 8.4: `clock format` takes `-format` and `-gmt` and hands the
    /// format to `strftime` (`%Ez` is `+0000`); `clock scan` takes only
    /// `-base` and `-gmt`; the ensemble is `clicks`, `format`, `scan` and
    /// `seconds`.
    Strftime84,
    /// Tcl 8.5 on: the clock rewrite (TIP 173) parses formats itself
    /// (`%Ez` stays `%Ez`) and adds `-locale`, `-timezone` and `clock scan
    /// -format`.
    #[default]
    Tcl85,
}

/// `clock`'s grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ClockGrammar {
    /// `clock add` units.
    pub units: ClockUnits,
    /// Option and specifier grammar.
    pub format: ClockFormatGrammar,
}

/// The field letters `binary format` and `binary scan` accept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BinaryFormats {
    /// Tcl 8.4: `a A b B h H c s S i I w W f d x X @`, and no `u` suffix.
    Tcl84,
    /// Tcl 8.5 on: adds `t n m r R q Q` and the `u` suffix (TIP 129, 275).
    #[default]
    Tcl85,
}

impl BinaryFormats {
    /// Whether `letter` is a field of this grammar.
    #[must_use]
    pub const fn accepts(self, letter: u8) -> bool {
        match letter {
            b'a' | b'A' | b'b' | b'B' | b'h' | b'H' | b'c' | b's' | b'S' | b'i' | b'I' | b'w'
            | b'W' | b'f' | b'd' | b'x' | b'X' | b'@' => true,
            b't' | b'n' | b'm' | b'r' | b'R' | b'q' | b'Q' => matches!(self, Self::Tcl85),
            _ => false,
        }
    }

    /// Whether a `u` after an integer field selects an unsigned reading.
    #[must_use]
    pub const fn has_unsigned_suffix(self) -> bool {
        matches!(self, Self::Tcl85)
    }
}

/// `binary`'s grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BinaryGrammar {
    /// Accepted field letters and suffixes.
    pub formats: BinaryFormats,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TclVersion;

    #[test]
    fn releases_select_the_measured_kernel_grammars() {
        let width = |version: TclVersion| version.release_policy().scan.int_width;
        assert_eq!(width(TclVersion::V8_4), ScanIntWidth::Wrap32Narrow);
        assert_eq!(width(TclVersion::V8_5), ScanIntWidth::Full);
        assert_eq!(width(TclVersion::V8_6), ScanIntWidth::Full);
        assert_eq!(width(TclVersion::V9_0), ScanIntWidth::Saturate32);
        assert_eq!(width(TclVersion::V9_1), ScanIntWidth::Saturate32);
        let clock = |version: TclVersion| version.release_policy().clock;
        assert_eq!(
            clock(TclVersion::V8_4),
            ClockGrammar {
                units: ClockUnits::None,
                format: ClockFormatGrammar::Strftime84
            }
        );
        assert_eq!(clock(TclVersion::V8_6).units, ClockUnits::Tcl8);
        assert_eq!(clock(TclVersion::V9_0).units, ClockUnits::Tcl9);
        assert_eq!(clock(TclVersion::V8_5).format, ClockFormatGrammar::Tcl85);
        for version in TclVersion::ALL {
            let formats = version.release_policy().binary.formats;
            assert_eq!(formats.accepts(b'm'), version >= TclVersion::V8_5);
            assert!(formats.accepts(b'w'));
            assert!(!formats.accepts(b'u'));
        }
    }
}
