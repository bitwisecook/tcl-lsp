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

//! How a release counts characters and converts strings to bytes: the character model and its unit iterator.

/// How Tcl converts a Unicode string representation to binary bytes.
///
/// Tcl 8.x keeps the historic one-byte conversion: each character contributes
/// its low byte. Tcl 9 made that conversion checked, rejecting a code point
/// above `U+00FF`. A byte-array object itself is unaffected: its dedicated
/// byte payload is always returned verbatim. This policy concerns the separate
/// string-to-bytes shimmer used by commands such as `binary encode` and
/// `binary scan`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ByteStringEncoding {
    /// Tcl 8.x's legacy low-byte conversion.
    LegacyTruncate,
    /// Tcl 9.x's checked conversion, which rejects code points above `U+00FF`.
    CheckedLatin1,
}

/// Character-counting model used by Tcl string operations.
///
/// Three-valued, because the releases are. Measured with
/// `string length [encoding convertfrom utf-8 …]` on tclsh 8.4.20, 8.5.19,
/// 8.6.18, 9.0.4 and 9.1b0:
///
/// | sequence | 8.4, 8.5 | 8.6 | 9.x |
/// |---|---|---|---|
/// | 2-byte (`é`, `U+00E9`) | 1 | 1 | 1 |
/// | 3-byte (`€`, `U+20AC`) | 1 | 1 | 1 |
/// | 4-byte (`😀`, `U+1F600`) | **4** | 2 | 1 |
///
/// Tcl 8.6 stores `Tcl_UniChar` as a 16-bit unit, so a supplementary-plane
/// character occupies a surrogate pair and contributes two. Tcl 9 widened
/// `Tcl_UniChar` and counts Unicode scalar values.
///
/// 8.4 and 8.5 are neither: their internal UTF-8 caps a character at three
/// bytes (`TCL_UTF_MAX` 3), so a supplementary code point is never assembled
/// into a character at all and each of its four bytes counts as one. That is
/// *not* byte counting — `é` and `€` still count as one apiece, which is why
/// this is its own variant rather than the `Bytes` model a Jim build without
/// `JIM_UTF8` would need (there `é` would count two).
///
/// `string index` shows the same split from the other side: at index 0 of
/// that four-byte string, 8.4/8.5 answer code point 240 (`0xF0`, the raw
/// UTF-8 lead byte), 8.6 answers 55357 (`0xD83D`, the high surrogate), and
/// 9.x answers 128512 (`U+1F600`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StringCharacterModel {
    /// Tcl 8.4-8.5: count BMP characters, but a supplementary code point
    /// counts as its four UTF-8 bytes (`TCL_UTF_MAX` 3).
    BmpCharsElseUtf8Bytes,
    /// Tcl 8.6: count UTF-16 code units.
    Utf16CodeUnits,
    /// Tcl 9.x: count Unicode scalar values.
    UnicodeScalars,
    /// Pinned Jim 0.84 UTF8 numeric units, retaining surrogate and invalid bytes.
    /// Byte-valued counting is implemented by the shared raw-string owner.
    Jim084Utf8,
}

impl StringCharacterModel {
    /// Every admitted model for cross-release agreement checks.
    pub const ALL: &'static [Self] = &[
        Self::BmpCharsElseUtf8Bytes,
        Self::Utf16CodeUnits,
        Self::UnicodeScalars,
        Self::Jim084Utf8,
    ];

    /// The number of Tcl characters `value` holds under this model: the
    /// [`Self::units`] it spans.
    ///
    /// The one place the counting rule lives, so a compile-time fold and a
    /// runtime `string length` cannot drift apart.
    #[must_use]
    pub fn count(self, value: &str) -> usize {
        match self {
            Self::UnicodeScalars | Self::Jim084Utf8 => value.chars().count(),
            Self::BmpCharsElseUtf8Bytes | Self::Utf16CodeUnits => {
                self.units(value).map(|run| run.units).sum()
            }
        }
    }

    /// `value`'s characters as this model addresses them, in order: each
    /// run is a character a Rust string holds and the number of the model's
    /// units it spans.
    ///
    /// - 8.4/8.5 never assemble a supplementary scalar (`TCL_UTF_MAX` 3):
    ///   its units are its four UTF-8 bytes, each an ordinary character
    ///   `U+0000`–`U+00FF`, so it yields four one-unit runs. Measured with
    ///   `scan [string index $s $i] %c` over a literal U+1D11E on tclsh
    ///   8.4.20 and 8.5.19: 240, 157, 132, 158.
    /// - 8.6 splits a supplementary scalar into a UTF-16 surrogate pair
    ///   (55348, 56606 on tclsh 8.6.18). A lone surrogate is not a value a
    ///   Rust string holds, so the pair stays one run spanning two units
    ///   (the boundary in `semantic-aot-optimisation.md` § *Tcl 8
    ///   supplementary-character boundary*).
    /// - 9.x and Jim 0.84 UTF8: one run per scalar.
    ///
    /// Every other character is one run of one unit under every model.
    #[must_use]
    pub fn units(self, value: &str) -> CharacterUnits<'_> {
        CharacterUnits {
            model: self,
            chars: value.chars(),
            bytes: [0; 4],
            next_byte: 0,
            byte_len: 0,
        }
    }

    /// The character count `model` defines, or — when no release is selected —
    /// the count **every** model agrees on, if they all agree.
    ///
    /// A dialect that names no runtime release still counts every string
    /// outside the supplementary planes identically under all three models, so
    /// a consumer keeps those answers and gives up only the genuinely
    /// ambiguous ones rather than declining wholesale.
    ///
    /// Unanimity is over `ALL`, not over a hardcoded pair: a rule written from
    /// the 8.6/9.0 pair alone answers 2 for a supplementary character under a
    /// profile that also spans 8.4, where the real answer is 4.
    #[must_use]
    pub fn count_for(model: Option<Self>, value: &str) -> Option<usize> {
        if let Some(model) = model {
            return Some(model.count(value));
        }
        let mut counts = Self::ALL.iter().map(|m| m.count(value));
        let first = counts.next()?;
        counts.all(|c| c == first).then_some(first)
    }
}

/// One run of [`StringCharacterModel::units`]: a character a Rust string
/// holds and the number of the model's units it spans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitRun {
    /// The character, as the model's units materialise it.
    pub held: char,
    /// 1, or more where the model's units for `held` cannot be held apart.
    pub units: usize,
}

/// The iterator [`StringCharacterModel::units`] returns.
#[derive(Debug, Clone)]
pub struct CharacterUnits<'a> {
    model: StringCharacterModel,
    chars: core::str::Chars<'a>,
    /// The UTF-8 bytes of a supplementary scalar 8.4/8.5 never assembled.
    bytes: [u8; 4],
    next_byte: usize,
    byte_len: usize,
}

impl Iterator for CharacterUnits<'_> {
    type Item = UnitRun;

    fn next(&mut self) -> Option<UnitRun> {
        if self.next_byte < self.byte_len {
            let byte = self.bytes[self.next_byte];
            self.next_byte += 1;
            return Some(UnitRun {
                held: char::from(byte),
                units: 1,
            });
        }
        let held = self.chars.next()?;
        if u32::from(held) <= 0xFFFF {
            return Some(UnitRun { held, units: 1 });
        }
        Some(match self.model {
            StringCharacterModel::BmpCharsElseUtf8Bytes => {
                self.byte_len = held.encode_utf8(&mut self.bytes).len();
                self.next_byte = 1;
                UnitRun {
                    held: char::from(self.bytes[0]),
                    units: 1,
                }
            }
            StringCharacterModel::Utf16CodeUnits => UnitRun {
                held,
                units: held.len_utf16(),
            },
            StringCharacterModel::UnicodeScalars | StringCharacterModel::Jim084Utf8 => {
                UnitRun { held, units: 1 }
            }
        })
    }
}
