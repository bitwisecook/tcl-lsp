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

//! How C Tcl 9.0 reads a byte string as characters, whatever the bytes hold.
//!
//! `Tcl_UtfToUniChar` reads a lead byte with its trail bytes as the code point
//! they encode when that is the shortest form, with `C0 80` (Tcl's NUL inside a
//! value) a NUL; a naked trail byte from `0x80` to `0x9F` as the character
//! cp1252 gives it; and any other byte, a lead byte whose sequence is cut short
//! or overlong among them, as a character of its own value. `Tcl_NumUtfChars`
//! and `Tcl_UtfNcmp` are those reads counted and compared, so the C shim and
//! the WASM runtime's C API both call this rather than decode for themselves.
//!
//! The decoder asks for bytes through a closure, so a caller holding a raw
//! pointer reads only the bytes C would, and a caller holding a slice reads
//! none past its end: a missing byte reads as NUL, which is never a trail byte.

/// The characters C Tcl reads the naked trail bytes `0x80` to `0x9F` as: the
/// ones cp1252 gives those bytes.
const CP1252: [u16; 32] = [
    0x20AC, 0x0081, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039,
    0x0152, 0x008D, 0x017D, 0x008F, 0x0090, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014,
    0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x009D, 0x017E, 0x0178,
];

/// The character at the start of the bytes `byte_at` gives, and the bytes it
/// takes. `byte_at(step)` is the byte `step` on from the start; it is asked for
/// each byte after the first only while every one before was a trail byte, and
/// never for more than the sequence the lead byte starts.
pub fn decode_with(mut byte_at: impl FnMut(usize) -> u8) -> (i32, usize) {
    let lead = byte_at(0);
    let mut trail = |step: usize| {
        let byte = byte_at(step);
        (byte & 0xC0 == 0x80).then_some(i32::from(byte & 0x3F))
    };
    let high = |mask: u8, shift: u32| i32::from(lead & mask) << shift;
    let decoded = match lead {
        0x80..=0x9F => Some((i32::from(CP1252[usize::from(lead - 0x80)]), 1)),
        // `C1` starts no sequence: its two-byte forms are all overlong, and C
        // Tcl does not read the byte after it.
        0xC0 | 0xC2..=0xDF => trail(1)
            .map(|one| (high(0x1F, 6) | one, 2))
            .filter(|&(character, _)| character == 0 || character >= 0x80),
        0xE0..=0xEF => trail(1)
            .and_then(|one| trail(2).map(|two| (high(0x0F, 12) | (one << 6) | two, 3)))
            .filter(|&(character, _)| character > 0x7FF),
        0xF0..=0xF4 => trail(1)
            .and_then(|one| {
                trail(2).and_then(|two| {
                    trail(3).map(|three| (high(0x07, 18) | (one << 12) | (two << 6) | three, 4))
                })
            })
            .filter(|&(character, _)| (0x1_0000..=0x10_FFFF).contains(&character)),
        _ => None,
    };
    decoded.unwrap_or((i32::from(lead), 1))
}

/// The first character of `bytes` and the bytes it takes, or `None` for no
/// bytes. A sequence cut short by the end of `bytes` is its bytes one each.
#[must_use]
pub fn decode(bytes: &[u8]) -> Option<(i32, usize)> {
    (!bytes.is_empty()).then(|| decode_with(|step| bytes.get(step).copied().unwrap_or(0)))
}

/// The characters in `bytes`, as `Tcl_NumUtfChars` counts a string of that many
/// bytes.
#[must_use]
pub fn count_chars(bytes: &[u8]) -> usize {
    let mut rest = bytes;
    let mut count = 0;
    while let Some((_, width)) = decode(rest) {
        rest = &rest[width..];
        count += 1;
    }
    count
}

/// Compare the first `chars` characters of two byte strings, as `Tcl_UtfNcmp`
/// does: the difference of the first pair that differs, else zero. `left_at`
/// and `right_at` give the byte at an offset into each string, and each is
/// read as far as `chars` characters take it, a NUL inside a value being a
/// character like any other. Where both strings reach a terminator at the
/// same character the rest compares equal: C would read on past both, which
/// its contract forbids and a raw pointer cannot survive.
pub fn compare_chars_with(
    chars: usize,
    mut left_at: impl FnMut(usize) -> u8,
    mut right_at: impl FnMut(usize) -> u8,
) -> i32 {
    let (mut left, mut right) = (0, 0);
    for _ in 0..chars {
        if left_at(left) == 0 && right_at(right) == 0 {
            return 0;
        }
        let (one, one_width) = decode_with(|step| left_at(left + step));
        let (two, two_width) = decode_with(|step| right_at(right + step));
        if one != two {
            return one - two;
        }
        left += one_width;
        right += two_width;
    }
    0
}

/// [`compare_chars_with`] over two slices, whose ends read as NUL.
#[must_use]
pub fn compare_chars(left: &[u8], right: &[u8], chars: usize) -> i32 {
    compare_chars_with(
        chars,
        |at| left.get(at).copied().unwrap_or(0),
        |at| right.get(at).copied().unwrap_or(0),
    )
}

/// What `Tcl_NumUtfChars` answers in C Tcl 9.0.4 for well-formed, modified
/// (`C0 80`) and malformed UTF-8: the bytes, the length given (negative for
/// up to the terminator), the count. Each answer was taken from a program
/// linked with C Tcl's own library, and each host of the C API is held to them.
pub const NUM_UTF_CHARS_VECTORS: &[(&[u8], isize, isize)] = &[
    (b"hello", -1, 5),
    (b"h\xc3\xa9llo", 6, 5),
    (b"a\xc0\x80b", 4, 3),
    (b"a\xc0\x80b", -1, 3),
    (b"\xc1\x80", 2, 2),
    (b"\x80a", 2, 2),
    (b"\x80a", -1, 2),
    (b"\xe0\x80\x41", 3, 3),
    (b"\xe0\xa0\x80", 3, 1),
    (b"\xe0\x80\x80", 3, 3),
    (b"\xf0\x90\x80\x80", 4, 1),
    (b"\xf4\x90\x80\x80", 4, 4),
    (b"\xf0\x80\x80\x80", 4, 4),
    (b"ab\xc3", 3, 3),
    (b"a\xe2\x82", 3, 3),
    (b"\xff\xfe", 2, 2),
    (b"ab\x80", 3, 3),
    (b"a\xe2\x82\xac", 3, 3),
    (b"\xf0\x9f\x98\x80", 3, 3),
    (b"a\xe2\x82\xac", 4, 2),
];

/// What `Tcl_UtfNcmp` answers in C Tcl 9.0.4, taken the same way: the two
/// strings, the characters compared, the answer.
pub const UTF_NCMP_VECTORS: &[(&[u8], &[u8], usize, i32)] = &[
    (b"\x80", b"\x81", 1, 8235),
    (b"\x9f", b"\xa0", 1, 216),
    (b"\xc0\x80", b"\x01", 1, -1),
    (b"\xc0\x80", b"", 1, 0),
    (b"\xc1\x80", b"\xc1a", 2, 8267),
    (b"\xc3\xa9", b"e", 1, 132),
    (b"\xf0\x9f\x98\x80", b"\xf0\x9f\x98\x81", 1, -1),
    (b"\xe0\x80\x41", b"\xe0\x81\x41", 3, 8235),
];

#[cfg(test)]
mod tests {
    use super::{
        NUM_UTF_CHARS_VECTORS, UTF_NCMP_VECTORS, compare_chars, compare_chars_with, count_chars,
        decode, decode_with,
    };

    fn counted(bytes: &[u8], length: isize) -> usize {
        match usize::try_from(length) {
            Ok(length) => count_chars(&bytes[..length]),
            Err(_) => count_chars(bytes.split(|&byte| byte == 0).next().unwrap_or_default()),
        }
    }

    #[test]
    fn counts_match_c_tcl() {
        for &(bytes, length, expected) in NUM_UTF_CHARS_VECTORS {
            assert_eq!(
                counted(bytes, length),
                usize::try_from(expected).unwrap(),
                "count of {bytes:x?} at {length}"
            );
        }
    }

    #[test]
    fn comparisons_match_c_tcl() {
        for &(left, right, chars, expected) in UTF_NCMP_VECTORS {
            assert_eq!(
                compare_chars(left, right, chars),
                expected,
                "compare {left:x?} with {right:x?} over {chars}"
            );
        }
    }

    #[test]
    fn a_character_reads_only_the_bytes_it_needs() {
        let mut asked = Vec::new();
        let read = decode_with(|step| {
            asked.push(step);
            [0xE2, 0x41, 0x82][step]
        });
        assert_eq!(read, (0xE2, 1));
        assert_eq!(asked, [0, 1]);
        assert_eq!(decode_with(|step| [0xC1, 0x80][step.min(1)]), (0xC1, 1));
        assert_eq!(decode(&[]), None);
        assert_eq!(decode(&[0xE2, 0x82, 0xAC, 0x80]), Some((0x20AC, 3)));
        assert_eq!(decode(&[0xC0, 0x80]), Some((0, 2)));
        assert_eq!(decode(&[0x80]), Some((0x20AC, 1)));
    }

    #[test]
    fn equal_strings_stop_at_their_shared_terminator() {
        let mut furthest = 0;
        let answer = compare_chars_with(
            8,
            |at| {
                furthest = furthest.max(at);
                b"ab\0"[at]
            },
            |at| b"ab\0"[at],
        );
        assert_eq!((answer, furthest), (0, 2));
    }
}
