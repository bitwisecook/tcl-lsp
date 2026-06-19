//! `lsort`/`lsearch` comparison core — the pure, value-model-free comparison
//! modes shared by both runtimes' list sort and search.
//!
//! The `-command` mode invokes a Tcl proc (Family-B) and stays in each adapter;
//! the modes here are pure `&[u8] -> Ordering`. `lsort` uses the lenient
//! [`key_compare`] (a non-numeric value under `-integer`/`-real` sorts as 0);
//! `lsearch` reuses the same primitives but reports its own coercion errors, so
//! it calls [`parse_wide`]/[`parse_real`]/[`dictionary_compare`] directly.
//!
//! `dictionary_compare` in particular is a faithful, fiddly port of C's
//! `DictionaryCompare` (`tclCmdIL.c`) — exactly the kind of subtle logic worth
//! writing once. (The bytecode VM had no dictionary comparison at all before
//! this; its `lsort -dictionary` fell back to a plain byte compare.)

use core::cmp::Ordering;

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

/// Parse a Tcl integer (optional sign + `0x`/`0o`/`0b`/decimal) for `-integer`
/// keys. `i128` so a key just past `i64` still orders correctly.
#[must_use]
pub fn parse_wide(b: &[u8]) -> Option<i128> {
    let s = core::str::from_utf8(b).ok()?.trim();
    let (neg, body) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    if body.is_empty() {
        return None;
    }
    let v: i128 = if let Some(h) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
        i128::from_str_radix(h, 16).ok()?
    } else if let Some(o) = body.strip_prefix("0o").or_else(|| body.strip_prefix("0O")) {
        i128::from_str_radix(o, 8).ok()?
    } else if let Some(bb) = body.strip_prefix("0b").or_else(|| body.strip_prefix("0B")) {
        i128::from_str_radix(bb, 2).ok()?
    } else {
        body.parse::<i128>().ok()?
    };
    Some(if neg { -v } else { v })
}

/// Parse a Tcl floating-point value for `-real` sort keys.
#[must_use]
pub fn parse_real(b: &[u8]) -> Option<f64> {
    core::str::from_utf8(b).ok()?.trim().parse::<f64>().ok()
}

/// Dictionary comparison (`lsort -dictionary`): case-insensitive, with embedded
/// decimal runs compared as numbers; a run with more leading zeros sorts later
/// only as a secondary tiebreak. Ported from C's `DictionaryCompare`.
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
        match (left.get(li).copied(), right.get(ri).copied()) {
            (Some(l), Some(r)) => {
                let (ll, rl) = (l.to_ascii_lowercase(), r.to_ascii_lowercase());
                if ll != rl {
                    return ll.cmp(&rl);
                }
                if secondary == 0 && l != r {
                    secondary = i64::from(l) - i64::from(r);
                }
                li += 1;
                ri += 1;
            }
            (l, r) => {
                let diff = l.map_or(0i64, i64::from) - r.map_or(0i64, i64::from);
                if diff != 0 {
                    return diff.cmp(&0);
                }
                return secondary.cmp(&0);
            }
        }
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
                a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase())
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
}
