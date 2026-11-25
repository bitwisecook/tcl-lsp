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

//! RFC 4648 base32, as tcllib 2.0's `base32` and `base32::hex` packages read
//! it: `base32::encode` and `base32::decode` over the standard alphabet, and
//! their `base32::hex` twins over the extended-hex one.
//!
//! [`encode`] is the packages' encoding of a byte string, `=`-padded to a
//! multiple of eight characters. [`decode`] answers only a canonical
//! encoding — a length that is a multiple of eight, the alphabet in either
//! case, a trailing run of one, three, four or six `=` and the bits the last
//! character carries past the data zero — which both of the packages'
//! implementations (pure Tcl and `tcllibc`) decode alike. Everything else is
//! a [`DecodeError`]: the Tcl implementation raises for each, and the C one
//! reads a set trailing bit as data, so the two part there.

/// The two alphabets the packages ship.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alphabet {
    /// `A`–`Z`, `2`–`7`: the `base32` package.
    Standard,
    /// `0`–`9`, `A`–`V`: the `base32::hex` package.
    ExtendedHex,
}

impl Alphabet {
    const fn encoding(self) -> data_encoding::Encoding {
        match self {
            Self::Standard => data_encoding::BASE32,
            Self::ExtendedHex => data_encoding::BASE32HEX,
        }
    }
}

/// Why [`decode`] gives no bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// The length is not a multiple of eight.
    Length,
    /// A character outside the alphabet and `=`, or padding with data after
    /// it.
    Character,
    /// A run of padding of a length no encoding ends in.
    Padding,
    /// The last character carries a set bit past the data.
    NonCanonical,
}

/// `data` encoded: eight characters for every five bytes, the last group
/// padded with `=`.
#[must_use]
pub fn encode(data: &[u8], alphabet: Alphabet) -> String {
    alphabet.encoding().encode(data)
}

/// `text` decoded, where it is a canonical encoding over `alphabet`.
///
/// # Errors
///
/// [`DecodeError`] for any other text.
pub fn decode(text: &str, alphabet: Alphabet) -> Result<Vec<u8>, DecodeError> {
    let symbols = text.as_bytes();
    if !symbols.len().is_multiple_of(8) {
        return Err(DecodeError::Length);
    }
    let data = symbols
        .iter()
        .rposition(|&symbol| symbol != b'=')
        .map_or(0, |last| last + 1);
    let padding = symbols.len() - data;
    if !matches!(padding, 0 | 1 | 3 | 4 | 6) {
        return Err(DecodeError::Padding);
    }
    // Tcllib accepts case-insensitive symbols and only a final padding run.
    if symbols[..data].contains(&b'=') {
        return Err(DecodeError::Character);
    }
    alphabet
        .encoding()
        .decode(text.to_ascii_uppercase().as_bytes())
        .map_err(|error| match error.kind {
            data_encoding::DecodeKind::Length => DecodeError::Length,
            data_encoding::DecodeKind::Symbol => DecodeError::Character,
            data_encoding::DecodeKind::Padding => DecodeError::Padding,
            data_encoding::DecodeKind::Trailing => DecodeError::NonCanonical,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encodings as tcllib 2.0's `base32` and `base32::hex` give them under
    /// tclsh 8.5.19, 8.6.18, 9.0.4 and 9.1.0, every release alike.
    const ENCODINGS: &[(&[u8], &str, &str)] = &[
        (b"", "", ""),
        (b"a", "ME======", "C4======"),
        (b"ab", "MFRA====", "C5H0===="),
        (b"abc", "MFRGG===", "C5H66==="),
        (b"abcd", "MFRGGZA=", "C5H66P0="),
        (b"abcde", "MFRGGZDF", "C5H66P35"),
        (b"abcdef", "MFRGGZDFMY======", "C5H66P35CO======"),
        (b"\xff\x00\x80", "74AIA===", "VS080==="),
        (b"foobar", "MZXW6YTBOI======", "CPNMUOJ1E8======"),
    ];

    #[test]
    fn each_encoding_is_the_packages() {
        for &(data, standard, hex) in ENCODINGS {
            assert_eq!(encode(data, Alphabet::Standard), standard, "{data:?}");
            assert_eq!(encode(data, Alphabet::ExtendedHex), hex, "{data:?}");
            assert_eq!(decode(standard, Alphabet::Standard).as_deref(), Ok(data));
            assert_eq!(decode(hex, Alphabet::ExtendedHex).as_deref(), Ok(data));
        }
    }

    /// What tcllib's Tcl implementation decodes, and what it raises for:
    /// each raise is a decline here, and so is a set trailing bit, which
    /// `tcllibc` reads as data.
    #[test]
    fn decode_answers_only_a_canonical_encoding() {
        let standard = |text| decode(text, Alphabet::Standard);
        assert_eq!(standard("MY======").as_deref(), Ok(&b"f"[..]));
        assert_eq!(standard("my======").as_deref(), Ok(&b"f"[..]));
        assert_eq!(standard("AA======").as_deref(), Ok(&b"\0"[..]));
        assert_eq!(
            standard("ABCDEFGH").as_deref(),
            Ok(&b"\x00\x44\x32\x14\xc7"[..])
        );
        assert_eq!(standard("mzxw6ytboi======").as_deref(), Ok(&b"foobar"[..]));
        assert_eq!(standard("MZ======"), Err(DecodeError::NonCanonical));
        assert_eq!(standard("ABCDEFG="), Err(DecodeError::NonCanonical));
        assert_eq!(standard("MZXW7==="), Err(DecodeError::NonCanonical));
        assert_eq!(standard("A======="), Err(DecodeError::Padding));
        assert_eq!(standard("========"), Err(DecodeError::Padding));
        assert_eq!(standard("ABCDEFG"), Err(DecodeError::Length));
        assert_eq!(standard("MZ======MZ======"), Err(DecodeError::Character));
        assert_eq!(standard("01234567"), Err(DecodeError::Character));
        assert_eq!(standard("MZXW 6YT"), Err(DecodeError::Character));
        let hex = |text| decode(text, Alphabet::ExtendedHex);
        assert_eq!(hex("c4======").as_deref(), Ok(&b"a"[..]));
        assert_eq!(hex("0123456V").as_deref(), Ok(&b"\x00\x44\x32\x14\xdf"[..]));
        assert_eq!(hex("V8======").as_deref(), Ok(&b"\xfa"[..]));
        assert_eq!(hex("W======="), Err(DecodeError::Padding));
        assert_eq!(hex("WA======"), Err(DecodeError::Character));
    }
}
