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

//! `lsort`/`lsearch` comparison core — the pure, value-model-free comparison
//! modes shared by both runtimes' list sort and search.
//!
//! The `-command` mode invokes a Tcl proc (Family-B) and stays in each adapter;
//! the modes here are pure `&[u8] -> Ordering`. `lsort` uses the lenient
//! [`key_compare`] (a non-numeric value under `-integer`/`-real` sorts as 0);
//! `lsearch` reuses the same primitives but reports its own coercion errors, so
//! it calls [`parse_wide`]/[`parse_real`]/[`dictionary_compare`] directly.
//!
//! `dictionary_compare` in particular reproduces the fiddly logic of C's
//! `DictionaryCompare` (`tclCmdIL.c`) — exactly the kind of subtle logic worth
//! writing once.

use core::cmp::Ordering;

use crate::string::{self, utf8_char_at};

/// A non-command comparison mode for `lsort`/`lsearch`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    /// Byte (optionally case-folded) comparison — `-ascii`.
    Ascii,
    /// Dictionary comparison — `-dictionary`.
    Dictionary,
    /// Integer comparison — `-integer`.
    Integer,
    /// Floating-point comparison — `-real`.
    Real,
}

/// Parse a Tcl integer for `-integer` keys — the shared 9.0-first
/// [`tcl_syntax::number`] grammar (optional sign + surrounding whitespace,
/// `0x`/`0o`/`0b`/`0d` radix prefixes, `_` digit separators) in its
/// whole-string, integer-only shape (`Tcl_GetWideIntFromObj` via
/// `TCL_PARSE_INTEGER_ONLY`). `i128` so a key just past `i64` still orders
/// correctly; values beyond `i128` (and floats, `Inf`, `NaN`) are `None`.
#[must_use]
pub fn parse_wide(b: &[u8]) -> Option<i128> {
    use tcl_syntax::number::{Number, ParseFlags, parse_whole_with};
    let s = core::str::from_utf8(b).ok()?;
    let flags = ParseFlags {
        integer_only: true,
        ..ParseFlags::default()
    };
    match parse_whole_with(s, flags)? {
        Number::Int(v) => Some(i128::from(v)),
        Number::Big {
            negative,
            radix,
            digits,
        } => {
            let mag = i128::from_str_radix(&digits, radix as u32).ok()?;
            Some(if negative { -mag } else { mag })
        }
        // `integer_only` classifies only Int/Big; named specials (`Inf`,
        // `NaN`) are not `-integer` keys.
        Number::Double(_) | Number::Nan { .. } => None,
    }
}

/// Parse a Tcl floating-point value for `-real` sort keys.
#[must_use]
pub fn parse_real(b: &[u8]) -> Option<f64> {
    core::str::from_utf8(b).ok()?.trim().parse::<f64>().ok()
}

/// Dictionary comparison (`lsort -dictionary`): case-insensitive, with embedded
/// decimal runs compared as numbers; a run with more leading zeros sorts later
/// only as a secondary tiebreak. Follows C's `DictionaryCompare`.
#[must_use]
pub fn dictionary_compare(left: &[u8], right: &[u8]) -> Ordering {
    let (mut li, mut ri) = (0usize, 0usize);
    let mut secondary: i64 = 0;
    loop {
        let l_is_digit = left.get(li).is_some_and(u8::is_ascii_digit);
        let r_is_digit = right.get(ri).is_some_and(u8::is_ascii_digit);
        if l_is_digit && r_is_digit {
            // Skip and tally leading zeros (more zeros → later, as a secondary).
            let mut zeros: i64 = 0;
            while left.get(li) == Some(&b'0') && left.get(li + 1).is_some_and(u8::is_ascii_digit) {
                li += 1;
                zeros += 1;
            }
            while right.get(ri) == Some(&b'0') && right.get(ri + 1).is_some_and(u8::is_ascii_digit)
            {
                ri += 1;
                zeros -= 1;
            }
            if secondary == 0 {
                secondary = zeros;
            }
            // Compare the digit runs by length, then by value.
            let mut diff: i64 = 0;
            loop {
                if diff == 0 {
                    diff = left.get(li).map_or(0, |&c| i64::from(c))
                        - right.get(ri).map_or(0, |&c| i64::from(c));
                }
                li += 1;
                ri += 1;
                let ld = left.get(li).is_some_and(u8::is_ascii_digit);
                let rd = right.get(ri).is_some_and(u8::is_ascii_digit);
                if !rd {
                    if ld {
                        return Ordering::Greater;
                    }
                    if diff != 0 {
                        return diff.cmp(&0);
                    }
                    break;
                } else if !ld {
                    return Ordering::Less;
                }
            }
            continue;
        }
        // Outside a digit run C steps a *character*, not a byte, and folds it
        // with `Tcl_UniCharToLower` (#2125): tclsh 8.4.20 onwards sort
        // `[list \u00c0 \u00e1 \u00c2]` as `\u00c0 \u00e1 \u00c2`, which is
        // folded order (\u00e0, \u00e1, \u00e2) — byte order would give `\u00c0 \u00c2 \u00e1`.
        let (Some((l, ln)), Some((r, rn))) = (utf8_char_at(left, li), utf8_char_at(right, ri))
        else {
            // One side (or both) has run out. C leaves the character loop here
            // and compares the *bytes* at the two cursors — one of them the
            // terminating NUL — so this exit stays byte-wise.
            let byte = |b: Option<&u8>| b.map_or(0i64, |&c| i64::from(c));
            let diff = byte(left.get(li)) - byte(right.get(ri));
            if diff != 0 {
                return diff.cmp(&0);
            }
            return secondary.cmp(&0);
        };
        let (ll, rl) = (string::simple_lower(l), string::simple_lower(r));
        if ll != rl {
            return ll.cmp(&rl);
        }
        // C's secondary is the *case* verdict, not the code-point difference:
        // "upper on the left" sorts first whichever way the two code points
        // happen to run. They only differ for a script whose uppercase sits
        // above its lowercase (Georgian Mtavruli), but C's rule is the one to
        // reproduce.
        if secondary == 0 {
            if l.is_uppercase() && r.is_lowercase() {
                secondary = -1;
            } else if r.is_uppercase() && l.is_lowercase() {
                secondary = 1;
            }
        }
        li += ln;
        ri += rn;
    }
}

/// Compare two keys under `mode` (lenient: a non-numeric value under `Integer`/
/// `Real` sorts as 0, matching `lsort`'s comparison). `lsearch`, which must
/// report a coercion error instead, uses the primitives above directly.
#[must_use]
pub fn key_compare(mode: SortMode, nocase: bool, a: &[u8], b: &[u8]) -> Ordering {
    match mode {
        SortMode::Dictionary => dictionary_compare(a, b),
        SortMode::Integer => parse_wide(a).unwrap_or(0).cmp(&parse_wide(b).unwrap_or(0)),
        SortMode::Real => parse_real(a)
            .unwrap_or(0.0)
            .partial_cmp(&parse_real(b).unwrap_or(0.0))
            .unwrap_or(Ordering::Equal),
        SortMode::Ascii => {
            if nocase {
                // Full-range fold, as C's `TclUtfCasecmp` (#2125): tclsh
                // 8.5.19 onwards sort `[list \u00c9 \u00e9 a B]` as
                // `a B \u00c9 \u00e9` — the two E-acutes compare *equal* and
                // keep input order. An ASCII fold leaves them unequal.
                string::fold_lower_bytes(a).cmp(&string::fold_lower_bytes(b))
            } else {
                a.cmp(b)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_wide_shares_the_canonical_integer_grammar() {
        // Whitespace, sign, radix prefixes, and the i128 headroom past i64.
        assert_eq!(parse_wide(b" 42 "), Some(42));
        assert_eq!(parse_wide(b"+7"), Some(7));
        assert_eq!(parse_wide(b"-0x10"), Some(-16));
        assert_eq!(parse_wide(b"0o17"), Some(15));
        assert_eq!(parse_wide(b"0b101"), Some(5));
        assert_eq!(
            parse_wide(b"0x7FFFFFFFFFFFFFFFF"), // one nibble past i64
            Some(0x0007_FFFF_FFFF_FFFF_FFFF_i128)
        );
        // The Tcl 9.0 spellings: `0d` decimal prefixes and `_` digit
        // separators. (tclsh8.6 rejects both — they are 9.0 syntax; the shared
        // grammar is 9.0-first by design.)
        assert_eq!(parse_wide(b"0d5"), Some(5));
        assert_eq!(parse_wide(b"1_000"), Some(1000));
        // A doubled sign is not an integer, as in tclsh.
        assert_eq!(parse_wide(b"--5"), None);
        assert_eq!(parse_wide(b"0x-5"), None);
        // Non-integers stay rejected.
        assert_eq!(parse_wide(b""), None);
        assert_eq!(parse_wide(b"1.5"), None);
        assert_eq!(parse_wide(b"Inf"), None);
        assert_eq!(parse_wide(b"12x"), None);
    }

    #[test]
    fn dictionary_orders_embedded_numbers() {
        // numeric runs compare by value, not lexically: x9 < x10.
        assert_eq!(dictionary_compare(b"x9", b"x10"), Ordering::Less);
        assert_eq!(dictionary_compare(b"x100", b"x99"), Ordering::Greater);
        // case-insensitive primary, with case as a secondary tiebreak.
        assert_eq!(dictionary_compare(b"abc", b"ABC"), Ordering::Greater);
        assert_eq!(dictionary_compare(b"Foo", b"foo"), Ordering::Less);
        // leading zeros only break ties.
        assert_eq!(dictionary_compare(b"a01", b"a1"), Ordering::Greater);
    }

    #[test]
    fn key_compare_modes() {
        // integer compares numerically (not lexically): 9 < 10.
        assert_eq!(
            key_compare(SortMode::Integer, false, b"9", b"10"),
            Ordering::Less
        );
        // hex/decimal mix.
        assert_eq!(
            key_compare(SortMode::Integer, false, b"0x10", b"15"),
            Ordering::Greater
        );
        // real.
        assert_eq!(
            key_compare(SortMode::Real, false, b"1.5", b"1.25"),
            Ordering::Greater
        );
        // ascii vs nocase.
        assert_eq!(
            key_compare(SortMode::Ascii, false, b"B", b"a"),
            Ordering::Less
        );
        assert_eq!(
            key_compare(SortMode::Ascii, true, b"B", b"a"),
            Ordering::Greater
        );
        // a non-numeric integer key is lenient (sorts as 0).
        assert_eq!(
            key_compare(SortMode::Integer, false, b"x", b"0"),
            Ordering::Equal
        );
    }

    #[test]
    fn nocase_and_dictionary_fold_the_full_unicode_range() {
        // Regression (#2125): both folds were `to_ascii_lowercase`, so every
        // non-ASCII letter compared by raw bytes.
        //
        // tclsh 8.5.19 / 8.6.18 / 9.0.4 / 9.1b0:
        //   % lsort -nocase [list É é a B]   ;# a B É é
        // i.e. the two E-acutes compare *equal* and keep input order.
        assert_eq!(
            key_compare(
                SortMode::Ascii,
                true,
                "\u{c9}".as_bytes(),
                "\u{e9}".as_bytes()
            ),
            Ordering::Equal
        );
        assert_eq!(
            key_compare(
                SortMode::Ascii,
                true,
                "\u{410}".as_bytes(),
                "\u{430}".as_bytes()
            ),
            Ordering::Equal
        );
        // `İ` folds to `i` (`Tcl_UniCharToLower`), so the fold must be the
        // simple 1:1 mapping, not Rust's full one. tclsh 8.5.19 onwards:
        //   % lsort -nocase [list İ i]   ;# İ i  (equal, input order)
        assert_eq!(
            key_compare(SortMode::Ascii, true, "\u{130}".as_bytes(), b"i"),
            Ordering::Equal
        );
        // A folded difference still orders by the folded code points, not the
        // raw ones: tclsh sorts [list À á Â] as À á
        // Â under both -nocase and -dictionary (à < á < â),
        // where a byte fold would put Â (0xc2) before á (0xe1).
        assert_eq!(
            key_compare(
                SortMode::Ascii,
                true,
                "\u{e1}".as_bytes(),
                "\u{c2}".as_bytes()
            ),
            Ordering::Less
        );
        assert_eq!(
            dictionary_compare("\u{e1}".as_bytes(), "\u{c2}".as_bytes()),
            Ordering::Less
        );
        assert_eq!(
            dictionary_compare("\u{c9}".as_bytes(), "\u{e9}".as_bytes()),
            Ordering::Less // equal folded; `-dictionary`'s upper-first secondary
        );
        assert_eq!(
            dictionary_compare("\u{130}".as_bytes(), b"i"),
            Ordering::Less // equal folded, upper on the left
        );
        // Case-sensitive comparison is untouched.
        assert_eq!(
            key_compare(
                SortMode::Ascii,
                false,
                "\u{c9}".as_bytes(),
                "\u{e9}".as_bytes()
            ),
            Ordering::Less
        );
        // Non-UTF-8 keys still compare instead of derailing the walk.
        assert_eq!(
            key_compare(SortMode::Ascii, true, &[0xffu8], &[0xffu8]),
            Ordering::Equal
        );
        assert_eq!(
            dictionary_compare(&[0xffu8, b'a'], &[0xffu8, b'A']),
            Ordering::Greater
        );
    }
}
