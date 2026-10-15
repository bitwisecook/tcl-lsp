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

//! `tcl::unsupported::representation value`, worded as C's
//! `Tcl_RepresentationCmd` (`generic/tclObj.c`) words it.
//!
//! Each engine reports the facts of its own value — its type, reference
//! count, address, internal representation and string — and this owns only
//! how a release spells them. Addresses and counts are the engine's, so they
//! never equal C's; the sentence around them does.

use std::fmt::Write as _;

use tcl_dialect::TclVersion;

/// What C's `Tcl_RepresentationCmd` reads from a `Tcl_Obj`.
#[derive(Debug, Clone, Copy)]
pub struct Representation<'a> {
    /// The `Tcl_ObjType` name; `None` is a pure string.
    pub type_name: Option<&'a str>,
    /// The value's reference count.
    pub ref_count: usize,
    /// The value's address.
    pub address: usize,
    /// The internal representation; [`Internal::None`] with no type.
    pub internal: Internal,
    /// The string representation's bytes, if the value has one.
    pub string: Option<&'a [u8]>,
}

/// A typed value's internal representation.
#[derive(Debug, Clone, Copy)]
pub enum Internal {
    /// No internal representation: a pure string.
    None,
    /// A double (`tclDoubleType`).
    Double(f64),
    /// A 64-bit value laid over `twoPtrValue` (a wide integer): one word on a
    /// 64-bit host, the low and high words on a 32-bit one.
    Bits(u64),
    /// Any other type, as `twoPtrValue`'s two words.
    Words(usize, usize),
}

/// `bits` as the two `twoPtrValue` words it occupies on this target.
fn bit_words(bits: u64) -> (usize, usize) {
    if usize::BITS >= 64 {
        (usize::try_from(bits).unwrap_or(usize::MAX), 0)
    } else {
        let low = usize::try_from(bits & 0xFFFF_FFFF).unwrap_or(usize::MAX);
        let high = usize::try_from(bits >> 32).unwrap_or(usize::MAX);
        (low, high)
    }
}

/// The command's result, as `release` writes it.
///
/// Measured on tclsh 8.6.18, 9.0.4 and 9.1.0: from 9.0 a double's internal
/// representation is its `%g` and a null word prints `0x0`; 8.6 prints the
/// double's bits as a word and a null word as glibc's `(nil)`. The string is
/// cut as `Tcl_AppendLimitedToObj(…, 16, "...")` cuts it.
#[must_use]
pub fn describe(value: &Representation<'_>, release: TclVersion) -> Vec<u8> {
    let word = |w: usize| -> String {
        if w == 0 && release < TclVersion::V9_0 {
            "(nil)".to_owned()
        } else {
            format!("{w:#x}")
        }
    };
    let mut out = format!(
        "value is a {} with a refcount of {}, object pointer at {}",
        value.type_name.unwrap_or("pure string"),
        value.ref_count,
        word(value.address),
    );
    if value.type_name.is_some() {
        match value.internal {
            Internal::Double(x) if release >= TclVersion::V9_0 => {
                if x.is_nan() {
                    // C's `Tcl_AppendPrintfToObj` refuses a NaN `%g` and
                    // appends its refusal instead.
                    out.push_str(
                        "Unable to format \", internal representation %g\" with supplied arguments: NaN",
                    );
                } else {
                    out.push_str(", internal representation ");
                    out.push_str(&crate::format::c_default_g(x));
                }
            }
            Internal::Double(x) => {
                let (a, b) = bit_words(x.to_bits());
                let _ = write!(out, ", internal representation {}:{}", word(a), word(b));
            }
            Internal::Bits(bits) => {
                let (a, b) = bit_words(bits);
                let _ = write!(out, ", internal representation {}:{}", word(a), word(b));
            }
            Internal::Words(a, b) => {
                let _ = write!(out, ", internal representation {}:{}", word(a), word(b));
            }
            Internal::None => {}
        }
    }
    let mut out = out.into_bytes();
    match value.string {
        Some(s) => {
            out.extend_from_slice(b", string representation \"");
            let kept = limited(s);
            out.extend_from_slice(kept);
            if kept.len() < s.len() {
                out.extend_from_slice(b"...");
            }
            out.push(b'"');
        }
        None => out.extend_from_slice(b", no string representation"),
    }
    out
}

/// `Tcl_AppendLimitedToObj(s, 16, "...")`'s copy: a string of more than 16
/// bytes keeps what precedes the character holding byte 13. Bytes are
/// counted as Tcl stores them, where a NUL takes two, and a character starts
/// at any byte that is not a UTF-8 continuation byte (`Tcl_UtfPrev`).
fn limited(s: &[u8]) -> &[u8] {
    const LIMIT: usize = 16;
    const KEEP: usize = 13;
    let tcl_len = |b: u8| if b == 0 { 2 } else { 1 };
    if s.iter().copied().map(tcl_len).sum::<usize>() <= LIMIT {
        return s;
    }
    let mut tcl_offset = 0;
    let mut start = 0;
    for (i, &b) in s.iter().enumerate() {
        if b & 0xC0 != 0x80 {
            start = i;
        }
        tcl_offset += tcl_len(b);
        if tcl_offset > KEEP {
            return &s[..start];
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pure(s: &str) -> Representation<'_> {
        Representation {
            type_name: None,
            ref_count: 3,
            address: 0x1000,
            internal: Internal::None,
            string: Some(s.as_bytes()),
        }
    }

    fn text(value: &Representation<'_>, release: TclVersion) -> String {
        String::from_utf8(describe(value, release)).expect("utf-8 in, utf-8 out")
    }

    /// The strings tclsh 8.6.18 and 9.0.4 cut, by the bytes they keep.
    #[test]
    fn strings_are_cut_as_tcl_append_limited_to_obj_cuts_them() {
        let rows: &[(&str, &str)] = &[
            ("abcdefghijklmnop", "abcdefghijklmnop"),
            ("abcdefghijklmnopq", "abcdefghijklm..."),
            ("", ""),
            ("abcdefghijkl\u{e9}xyz", "abcdefghijkl..."),
            ("abcdefghijklm\u{e9}xyz", "abcdefghijklm..."),
            ("abcdefghijkl\0xyzw", "abcdefghijkl..."),
            ("abcdefghijklm\0xyzw", "abcdefghijklm..."),
            ("abcdefghijk\u{1f600}xyzw", "abcdefghijk..."),
            ("abcdefghi\u{1f600}\u{1f600}", "abcdefghi\u{1f600}..."),
        ];
        for (input, kept) in rows {
            let text = text(&pure(input), TclVersion::V9_0);
            assert!(
                text.ends_with(&format!("string representation \"{kept}\"")),
                "{input:?}: {text}"
            );
        }
    }

    /// A 64-bit rep fills both words on a 32-bit target (wasm32) and one on
    /// a 64-bit host, as C's union overlays `twoPtrValue`.
    #[test]
    fn a_wide_rep_is_split_across_the_target_s_words() {
        let (low, high) = bit_words(3.0f64.to_bits());
        if usize::BITS >= 64 {
            assert_eq!((low, high), (0x4008_0000_0000_0000, 0));
        } else {
            assert_eq!((low, high), (0, 0x4008_0000));
        }
        let (low, high) = bit_words((-1i64).cast_unsigned());
        if usize::BITS >= 64 {
            assert_eq!((low, high), (usize::MAX, 0));
        } else {
            assert_eq!((low, high), (usize::MAX, usize::MAX));
        }
    }

    #[test]
    fn a_release_spells_doubles_and_null_words_as_tclsh_does() {
        let double = |x| Representation {
            type_name: Some("double"),
            ref_count: 2,
            address: 0x10,
            internal: Internal::Double(x),
            string: None,
        };
        let rows: &[(f64, &str, &str)] = &[
            (3.0, "3", "0x4008000000000000:(nil)"),
            (f64::INFINITY, "inf", "0x7ff0000000000000:(nil)"),
            (f64::NEG_INFINITY, "-inf", "0xfff0000000000000:(nil)"),
            (1.0 / 3.0, "0.333333", "0x3fd5555555555555:(nil)"),
            (-0.0, "-0", "0x8000000000000000:(nil)"),
            (1e-5, "1e-05", "0x3ee4f8b588e368f1:(nil)"),
            (123_456_789.0, "1.23457e+08", "0x419d6f3454000000:(nil)"),
        ];
        for (x, g, words) in rows {
            assert_eq!(
                text(&double(*x), TclVersion::V9_0),
                format!(
                    "value is a double with a refcount of 2, object pointer at 0x10, internal representation {g}, no string representation"
                ),
            );
            assert_eq!(
                text(&double(*x), TclVersion::V8_6),
                format!(
                    "value is a double with a refcount of 2, object pointer at 0x10, internal representation {words}, no string representation"
                ),
            );
        }
        assert_eq!(
            text(&double(f64::NAN), TclVersion::V9_1),
            "value is a double with a refcount of 2, object pointer at 0x10Unable to format \", internal representation %g\" with supplied arguments: NaN, no string representation",
        );
        let list = Representation {
            type_name: Some("list"),
            ref_count: 2,
            address: 0x20,
            internal: Internal::Words(0x30, 0),
            string: Some(b"a b"),
        };
        assert_eq!(
            text(&list, TclVersion::V9_0),
            "value is a list with a refcount of 2, object pointer at 0x20, internal representation 0x30:0x0, string representation \"a b\"",
        );
        assert_eq!(
            text(&list, TclVersion::V8_6),
            "value is a list with a refcount of 2, object pointer at 0x20, internal representation 0x30:(nil), string representation \"a b\"",
        );
        assert_eq!(
            text(&pure("1.5"), TclVersion::V9_0),
            "value is a pure string with a refcount of 3, object pointer at 0x1000, string representation \"1.5\"",
        );
    }
}
