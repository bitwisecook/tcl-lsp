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

//! WASM binary encoding helpers — pure functions, no emitter state.
//!
//! LEB128 for integers and length-prefixed UTF-8 for strings: the input
//! side of every byte the module serialiser writes.
//!
//! The `& 0x7F` masking makes the low-byte extraction exact, and lengths
//! are bounded by source size.

/// Encode an unsigned integer as unsigned LEB128.
#[must_use]
pub fn leb128_unsigned(mut value: u64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        // `& 0x7F` keeps only the low 7 bits, so the value always fits in `u8`.
        let mut byte = u8::try_from(value & 0x7F).expect("7-bit value fits u8");
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
    out
}

/// Encode a signed integer as signed LEB128.
#[must_use]
pub fn leb128_signed(mut value: i64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        // `& 0x7F` masks to the low 7 bits (0..=127), so it fits in `u8`.
        let byte = u8::try_from(value & 0x7F).expect("7-bit value fits u8");
        // Arithmetic shift keeps the sign bit.
        value >>= 7;
        let sign_bit_set = byte & 0x40 != 0;
        if (value == 0 && !sign_bit_set) || (value == -1 && sign_bit_set) {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
    out
}

/// Decode an unsigned LEB128 from the front of `bytes` (ignores trailing
/// bytes). Returns 0 for an empty slice.
#[must_use]
pub fn decode_leb128_unsigned(bytes: &[u8]) -> u64 {
    let mut result: u64 = 0;
    let mut shift = 0;
    for &b in bytes {
        result |= u64::from(b & 0x7F) << shift;
        if b & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    result
}

/// Decode a signed LEB128 from the front of `bytes`.
#[must_use]
pub fn decode_leb128_signed(bytes: &[u8]) -> i64 {
    let mut result: i64 = 0;
    let mut shift = 0u32;
    let mut last = 0u8;
    for &b in bytes {
        result |= i64::from(b & 0x7F) << shift;
        shift += 7;
        last = b;
        if b & 0x80 == 0 {
            break;
        }
    }
    // Sign-extend if the sign bit of the last byte is set and we have room.
    if shift < 64 && last & 0x40 != 0 {
        result |= -1i64 << shift;
    }
    result
}

/// Encode a UTF-8 string with an unsigned-LEB128 length prefix.
#[must_use]
pub fn encode_string(s: &str) -> Vec<u8> {
    let bytes = s.as_bytes();
    let mut out = leb128_unsigned(bytes.len() as u64);
    out.extend_from_slice(bytes);
    out
}

/// Encode a WASM vector: an unsigned-LEB128 count followed by the
/// concatenated item encodings.
#[must_use]
pub fn encode_vector(items: &[Vec<u8>]) -> Vec<u8> {
    let mut out = leb128_unsigned(items.len() as u64);
    for item in items {
        out.extend_from_slice(item);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsigned_leb128_roundtrips() {
        for v in [
            0u64,
            1,
            63,
            64,
            127,
            128,
            300,
            16384,
            624_485,
            u64::from(u32::MAX),
        ] {
            assert_eq!(decode_leb128_unsigned(&leb128_unsigned(v)), v, "value {v}");
        }
    }

    #[test]
    fn signed_leb128_roundtrips() {
        for v in [
            0i64, 1, -1, 63, -63, 64, -64, 127, -128, 128, -129, 624_485, -624_485,
        ] {
            assert_eq!(decode_leb128_signed(&leb128_signed(v)), v, "value {v}");
        }
    }

    #[test]
    fn known_leb128_encodings() {
        // Canonical examples from the LEB128 spec / WASM tests.
        assert_eq!(leb128_unsigned(624_485), vec![0xE5, 0x8E, 0x26]);
        assert_eq!(leb128_signed(-123_456), vec![0xC0, 0xBB, 0x78]);
        assert_eq!(leb128_unsigned(0), vec![0x00]);
    }

    #[test]
    fn string_and_vector_encoding() {
        assert_eq!(encode_string("tcl"), vec![3, b't', b'c', b'l']);
        let items = vec![vec![0xAu8], vec![0xB, 0xC]];
        assert_eq!(encode_vector(&items), vec![2, 0xA, 0xB, 0xC]);
    }
}
