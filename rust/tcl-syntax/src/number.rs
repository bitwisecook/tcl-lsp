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

//! Tcl numeric-literal grammar — the shared `TclParseNumber` port.
//!
//! Classifies a string as a Tcl 9.0 number, re-derived from reference
//! `tmp/tcl9.0.3/generic/tclStrToD.c::TclParseNumber`. This is the *parse*
//! direction of the numeric tower (string → number); the *format* direction
//! (number → shortest-round-trip string) and the bignum arithmetic live with
//! each consumer (the runtime over libtommath `mp_int`, the compiler's
//! const-folder over its own value type) — both classify with this one grammar.
//!
//! The output is one of the four tower types ([`Number`]): a wide
//! [`Number::Int`] when the integer fits `i64`, a [`Number::Big`] (sign +
//! radix + cleaned digits, for the consumer to build a bignum from) when it
//! overflows, a [`Number::Double`] for floats and `±Inf`, or a [`Number::Nan`].
//! There is **no** boolean handling here —
//! `true`/`yes`/`on`… are `Tcl_GetBoolean`'s job, not `TclParseNumber`'s.
//!
//! ## Tcl 9.0 forms
//!
//! - Optional sign, optional surrounding whitespace (unless [`ParseFlags::no_whitespace`]).
//! - Radix prefixes `0x`/`0X`, `0o`/`0O`, `0b`/`0B`, `0d`/`0D` (case-insensitive,
//!   each needs ≥1 following digit). **A bare leading `0` is decimal** (`0755`
//!   == 755 — the 8.x octal-by-leading-zero rule is gone in 9.0).
//! - `_` digit separators between same-base digits (`1__000`, `0xff__ff`),
//!   unless [`ParseFlags::no_underscore`]. Not leading/trailing a digit run.
//! - Decimal floats: `1.5`, `.5`, `1.`, `1e9`, `1.5E-3` (no hex/oct/bin floats).
//! - `Inf`/`Infinity` and `NaN`/`NaN(hexpayload)` — case-insensitive.

use std::borrow::Cow;

/// Integer radix (base) for a [`Number::Big`]'s digit string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Radix {
    /// Base 2 (`0b…`).
    Bin = 2,
    /// Base 8 (`0o…`).
    Oct = 8,
    /// Base 10 (the default / `0d…`).
    Dec = 10,
    /// Base 16 (`0x…`).
    Hex = 16,
}

/// A classified Tcl number — one rung of the numeric tower.
#[derive(Debug, Clone, PartialEq)]
pub enum Number {
    /// An integer that fits in a wide (`i64`).
    Int(i64),
    /// An integer too large for a wide: build the bignum from `digits` (sign,
    /// radix prefix, and `_` separators already stripped) in `radix`, negating
    /// when `negative`. (E.g. feed `digits` to libtommath `mp_read_radix`.)
    Big {
        /// The value's sign.
        negative: bool,
        /// The base of `digits`.
        radix: Radix,
        /// Magnitude digits in `radix`, cleaned (no sign/prefix/underscores).
        digits: String,
    },
    /// A floating-point value (including `±Inf`).
    Double(f64),
    /// IEEE NaN, with the optional `NaN(hexpayload)` mantissa payload.
    Nan {
        /// Whether the literal was sign-negated (`-NaN`).
        negative: bool,
        /// The `NaN(hex)` payload, if one was given.
        payload: Option<u64>,
    },
}

/// Knobs mirroring `TclParseNumber`'s flag bits (the subset used so far).
#[derive(Debug, Clone, Copy, Default)]
pub struct ParseFlags {
    /// Reject a fractional part / exponent (`TCL_PARSE_INTEGER_ONLY`).
    pub integer_only: bool,
    /// Reject leading/trailing whitespace (`TCL_PARSE_NO_WHITESPACE`).
    pub no_whitespace: bool,
    /// Reject `_` digit separators (`TCL_PARSE_NO_UNDERSCORE`).
    pub no_underscore: bool,
}

/// A successful parse: the classified [`Number`] and the byte offset just past
/// the consumed number (the `endPtr` of `TclParseNumber`).
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    /// The classified value.
    pub number: Number,
    /// Offset in the input just past the number (before any trailing space).
    pub end: usize,
}

/// Format an `f64` as Tcl's canonical double string, byte-for-byte with
/// `Tcl_PrintDouble` (`tclStrToD.c`): `NaN`, `Inf`/`-Inf`, signed zero
/// (`0.0`/`-0.0`), and otherwise the shortest decimal that round-trips, laid out
/// `%g`-style — fixed notation when the decimal point sits in `[-3, 17]`, else
/// scientific (`d.ddde±NN`). An integer-valued result in fixed notation keeps a
/// `.0` suffix so it re-parses as a `Double`, not an `Int` (`2.0` not `2`,
/// `1e16` as `10000000000000000.0` not `10000000000000000`). The one shared
/// number→string formatter for the runtime's `double` rep and the compiler's
/// const-folder.
///
/// We must NOT use Rust's bare `{}` here: for large/small magnitudes it picks a
/// different fixed/scientific cutover than C Tcl (`{}` prints `1e300` as 301
/// digits and `1e17` as `100000000000000000`, the latter re-parsing as an
/// integer), so the `.0`-vs-Int round-trip and the canonical text both break.
/// Instead we take Rust's shortest scientific form (`{:e}`, which selects the
/// same minimal digits as C's `dtoa`) and re-lay it out under Tcl's rule.
#[must_use]
pub fn format_double(f: f64) -> String {
    if f.is_nan() {
        return "NaN".to_owned();
    }
    if f.is_infinite() {
        return if f.is_sign_negative() {
            "-Inf".to_owned()
        } else {
            "Inf".to_owned()
        };
    }
    // Signed zero must keep its sign: `Tcl_PrintDouble` renders `-0.0` as `-0.0`,
    // and `f == 0.0` is true for both zeros (so the sign bit is the only tell).
    if f == 0.0 {
        return if f.is_sign_negative() {
            "-0.0".to_owned()
        } else {
            "0.0".to_owned()
        };
    }

    // `{:e}` gives the shortest round-tripping mantissa + a base-10 exponent,
    // e.g. `1.5e2`, `1e17`, `6.022e23`. Split them to drive Tcl's layout.
    let neg = f < 0.0;
    let sci = format!("{:e}", f.abs());
    let (mantissa, exp_str) = sci
        .split_once('e')
        .expect("Rust's {:e} always emits an 'e' separator");
    let exp: i32 = exp_str
        .parse()
        .expect("Rust's {:e} exponent is a valid integer");
    // `digits` are the significant digits (point removed); `decpt` is the
    // position of the decimal point measured from the first digit, so the value
    // is `0.<digits> * 10^decpt` (matching `dtoa`'s `decpt`).
    let digits: String = mantissa.chars().filter(|&c| c != '.').collect();
    let ndigits = digits.len();
    let decpt = exp + 1;

    // Tcl's `%g` cutover: scientific outside `decpt ∈ [-3, 17]`, else fixed.
    let out = if !(-3..=17).contains(&decpt) {
        // Scientific: the shortest mantissa verbatim (a single digit stays bare,
        // no `.0`), then `e`, an explicit sign, and the natural-width exponent.
        let e = decpt - 1;
        format!("{mantissa}e{}{}", if e < 0 { '-' } else { '+' }, e.abs())
    } else if decpt <= 0 {
        // Leading-zero fraction: `0.000<digits>` (`-decpt` is ≥ 0 here).
        let lead = decpt.unsigned_abs() as usize;
        let zeros = "0".repeat(lead);
        format!("0.{zeros}{digits}")
    } else {
        // `decpt > 0` here, so it converts to a length cleanly.
        let point = usize::try_from(decpt).unwrap_or(usize::MAX);
        if point >= ndigits {
            // Integer-valued in fixed notation: pad to the point, then the `.0`
            // that keeps it parsing as a Double rather than an Int.
            let zeros = "0".repeat(point - ndigits);
            format!("{digits}{zeros}.0")
        } else {
            // A point inside the digits: `<int>.<frac>` (`0 < point < ndigits`).
            let (int_part, frac_part) = digits.split_at(point);
            format!("{int_part}.{frac_part}")
        }
    };
    if neg { format!("-{out}") } else { out }
}

/// Exactly compare an integer against a double, the way C Tcl's
/// `TclCompareTwoNumbers` (`tclExecute.c`) does for its wide/double arm —
/// **without** first rounding the integer to a double, which above 2⁵³ merges
/// distinct values (`expr {9007199254740993 > 9007199254740992.0}` is 1; a
/// both-as-`f64` comparison calls them equal). `None` means the double is NaN
/// (unordered — C Tcl's rule for a NaN comparison operand is "`!=` is true,
/// every other comparison false", which the caller applies).
///
/// Takes `i128` so the runtime's `Big` tier shares it; `i64` callers widen.
#[must_use]
pub fn compare_int_double(w: i128, d: f64) -> Option<core::cmp::Ordering> {
    use core::cmp::Ordering;
    // 2¹²⁷ — the smallest double at or above every i128 (`i128::MAX` = 2¹²⁷−1).
    // At or beyond it (incl. +Inf) every integer is smaller; strictly below
    // −2¹²⁷ (incl. −Inf) every integer is greater. −2¹²⁷ itself is exactly
    // `i128::MIN`, so it falls through to the exact path.
    const TWO_POW_127: f64 = 170_141_183_460_469_231_731_687_303_715_884_105_728.0;
    if d.is_nan() {
        return None;
    }
    if d >= TWO_POW_127 {
        return Some(Ordering::Less);
    }
    if d < -TWO_POW_127 {
        return Some(Ordering::Greater);
    }
    // Finite with |d| ≤ 2¹²⁷: the integer part is exactly representable, so
    // compare integer parts first and let a non-zero fraction break the tie.
    let (int_part, has_fraction) = split_double_exact(d);
    Some(match w.cmp(&int_part) {
        Ordering::Equal if !has_fraction => Ordering::Equal,
        // Same integer part but `d` carries a fraction: truncation is toward
        // zero, so a positive `d` sits above its integer part (w < d) and a
        // negative `d` below it (w > d).
        Ordering::Equal if d > 0.0 => Ordering::Less,
        Ordering::Equal => Ordering::Greater,
        unequal => unequal,
    })
}

/// Split a finite double with |d| ≤ 2¹²⁷ into its exact truncated integer part
/// and whether a non-zero fractional part was cut off. Pure bit arithmetic on
/// the IEEE-754 representation — a `d as i128` cast would truncate silently on
/// out-of-range input and trip `clippy::cast_possible_truncation`; this stays
/// exact by construction.
fn split_double_exact(d: f64) -> (i128, bool) {
    const MANTISSA_BITS: u64 = 52;
    const MANTISSA_MASK: u64 = (1 << MANTISSA_BITS) - 1;
    const EXPONENT_MASK: u64 = 0x7ff;
    // IEEE-754 binary64 bias for the stored exponent, plus the 52 mantissa
    // fraction bits: value = mantissa × 2^(stored − 1023 − 52).
    const EXPONENT_BIAS_AND_SHIFT: u64 = 1023 + MANTISSA_BITS;

    let bits = d.to_bits();
    let negative = (bits >> 63) == 1;
    let stored_exponent = (bits >> MANTISSA_BITS) & EXPONENT_MASK;
    let fraction_bits = bits & MANTISSA_MASK;
    if stored_exponent == 0 {
        // Zero (fraction 0) or subnormal (|d| < 2⁻¹⁰²²): integer part is 0
        // either way; only a subnormal leaves a fraction behind.
        return (0, fraction_bits != 0);
    }
    // Normal number: an implicit leading 1 above the 52 stored fraction bits.
    let mantissa = (1 << MANTISSA_BITS) | fraction_bits;
    let (magnitude, has_fraction) =
        if let Some(left) = stored_exponent.checked_sub(EXPONENT_BIAS_AND_SHIFT) {
            // 2^left scales the mantissa up: purely integral. The caller's range
            // bound (|d| ≤ 2¹²⁷) keeps the result within a u128; saturate rather
            // than panic if the bound is ever violated.
            let scaled = u32::try_from(left)
                .ok()
                .and_then(|shift| u128::from(mantissa).checked_shl(shift))
                .unwrap_or(u128::MAX);
            (scaled, false)
        } else {
            // Scaling down by 2^right: the low `right` bits are the fraction.
            let right = EXPONENT_BIAS_AND_SHIFT - stored_exponent;
            if right > MANTISSA_BITS {
                // The whole mantissa is fractional (|d| < 1).
                (0, true)
            } else {
                (
                    u128::from(mantissa >> right),
                    mantissa & ((1 << right) - 1) != 0,
                )
            }
        };
    // Magnitude ≤ 2¹²⁷ by the caller's bound. Only −2¹²⁷ (`i128::MIN`) uses the
    // top bit, and only on the negative side; every other value converts.
    let int_part = if negative {
        i128::try_from(magnitude).map_or(i128::MIN, |m| -m)
    } else {
        i128::try_from(magnitude).unwrap_or(i128::MAX)
    };
    (int_part, has_fraction)
}

/// Tcl numeric whitespace: space, tab, newline, VT, FF, CR.
#[inline]
fn is_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

#[inline]
fn digit_val(c: u8, radix: u32) -> Option<u32> {
    let v = match c {
        b'0'..=b'9' => u32::from(c - b'0'),
        b'a'..=b'f' => u32::from(c - b'a') + 10,
        b'A'..=b'F' => u32::from(c - b'A') + 10,
        _ => return None,
    };
    (v < radix).then_some(v)
}

/// Parse a number at the start of `s` (after optional leading whitespace),
/// returning the classified value and where it ended. Returns `None` if no valid
/// number begins there. This is the partial form (the lexer/`scan` entry); use
/// [`parse_whole`] when the entire string must be a number.
#[must_use]
pub fn parse(s: &str, flags: ParseFlags) -> Option<Parsed> {
    let b = s.as_bytes();
    let len = b.len();
    let mut i = 0;

    if !flags.no_whitespace {
        while i < len && is_ws(b[i]) {
            i += 1;
        }
    }

    // Optional sign.
    let mut negative = false;
    if i < len && (b[i] == b'+' || b[i] == b'-') {
        negative = b[i] == b'-';
        i += 1;
    }

    // Inf / NaN (alphabetic forms).
    if i < len && matches!(b[i], b'i' | b'I' | b'n' | b'N') {
        return parse_inf_nan(b, i, negative);
    }

    // Radix prefix (`0x`/`0o`/`0b`/`0d`), each requiring a following digit. A
    // bare leading `0` falls through to the decimal path (9.0: not octal).
    let mut radix = Radix::Dec;
    let mut int_only_prefix = flags.integer_only;
    if i + 1 < len && b[i] == b'0' {
        let (r, only) = match b[i + 1] {
            b'x' | b'X' => (Some(Radix::Hex), true),
            b'o' | b'O' => (Some(Radix::Oct), true),
            b'b' | b'B' => (Some(Radix::Bin), true),
            b'd' | b'D' => (Some(Radix::Dec), true),
            _ => (None, false),
        };
        if let Some(r) = r {
            // Commit to the prefix only if a valid digit follows it.
            if i + 2 < len && digit_val(b[i + 2], r as u32).is_some() {
                radix = r;
                int_only_prefix = only; // `0x`/`0o`/`0b`/`0d` are integer forms
                i += 2;
            }
        }
    }

    // Scan the integer magnitude digits (with `_` separators).
    let int_start = i;
    let scan = scan_digits(b, i, radix as u32, flags.no_underscore);
    i = scan.end;

    // Decimal floats: a `.` or exponent (and not integer-only) makes it a double.
    // Also a leading `.` with no integer digits (`.5`).
    let has_int_digits = i > int_start;
    let dot = i < len && b[i] == b'.';
    let exp = i < len && (b[i] == b'e' || b[i] == b'E');
    if radix == Radix::Dec && !int_only_prefix && (dot || exp || !has_int_digits) {
        return parse_decimal_float(b, int_start, negative, flags);
    }
    if !has_int_digits {
        return None; // no digits and not a float → not a number
    }

    // Integer: a wide `Int` when the magnitude fits, else `Big` from the cleaned
    // digits (the wide `u64` accumulator can still exceed `i64`'s range without
    // overflowing — e.g. magnitudes in `(i64::MAX, u64::MAX]`).
    let number = match (scan.overflow, to_i64(scan.magnitude, negative)) {
        (false, Some(v)) => Number::Int(v),
        _ => Number::Big {
            negative,
            radix,
            digits: scan.digits.into_owned(),
        },
    };
    Some(Parsed { number, end: i })
}

/// Apply `negative` to a `u64` magnitude, yielding `Some(i64)` when it fits a
/// wide (`i64::MIN`'s magnitude is `2^63`, representable only when negated).
fn to_i64(mag: u64, negative: bool) -> Option<i64> {
    if negative {
        if mag == 1u64 << 63 {
            Some(i64::MIN)
        } else {
            i64::try_from(mag).ok().map(|v| -v)
        }
    } else {
        i64::try_from(mag).ok()
    }
}

/// Parse `s` as a complete Tcl number: optional surrounding whitespace, then the
/// whole remainder must be the number (the `Tcl_GetWideIntFromObj`/`…Double…`
/// shape). Returns `None` on trailing junk.
#[must_use]
pub fn parse_whole(s: &str) -> Option<Number> {
    let flags = ParseFlags::default();
    let p = parse(s, flags)?;
    let mut i = p.end;
    let b = s.as_bytes();
    while i < b.len() && is_ws(b[i]) {
        i += 1;
    }
    (i == b.len()).then_some(p.number)
}

struct IntScan<'s> {
    end: usize,
    /// Accumulated magnitude (valid only when `!overflow`).
    magnitude: u64,
    /// The wide magnitude overflowed `u64` — use `digits` for a bignum.
    overflow: bool,
    /// Magnitude digits with `_` separators removed (borrowed when none).
    digits: Cow<'s, str>,
}

/// Scan a run of base-`radix` digits with optional `_` separators (only between
/// two digits). Accumulates the magnitude into `u64`, flagging overflow.
fn scan_digits(b: &[u8], start: usize, radix: u32, no_underscore: bool) -> IntScan<'_> {
    let len = b.len();
    let mut i = start;
    let mut mag: u64 = 0;
    let mut overflow = false;
    let mut saw_underscore = false;
    while i < len {
        if let Some(d) = digit_val(b[i], radix) {
            if !overflow {
                match mag
                    .checked_mul(u64::from(radix))
                    .and_then(|m| m.checked_add(u64::from(d)))
                {
                    Some(m) => mag = m,
                    None => overflow = true,
                }
            }
            i += 1;
        } else if !no_underscore
            && b[i] == b'_'
            && i > start
            && digit_val(b[i - 1], radix).is_some()
        {
            let mut j = i + 1;
            while j < len && b[j] == b'_' {
                j += 1;
            }
            if j < len && digit_val(b[j], radix).is_some() {
                saw_underscore = true;
                i = j;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    // Cleaned digits (for the bignum path): strip `_`. Borrow when there were none.
    let raw = std::str::from_utf8(&b[start..i]).unwrap_or("");
    let digits = if saw_underscore {
        Cow::Owned(raw.replace('_', ""))
    } else {
        Cow::Borrowed(raw)
    };
    IntScan {
        end: i,
        magnitude: mag,
        overflow,
        digits,
    }
}

/// Parse a decimal float lexeme starting at `start` (after the sign), via Rust's
/// correctly-rounded `f64` parser once `_` separators are stripped.
fn parse_decimal_float(
    b: &[u8],
    start: usize,
    negative: bool,
    flags: ParseFlags,
) -> Option<Parsed> {
    let len = b.len();
    let mut i = start;
    let mut any_digit = false;

    let scan_run = |b: &[u8], mut i: usize| -> usize {
        while i < len {
            if b[i].is_ascii_digit() {
                i += 1;
            } else if !flags.no_underscore && b[i] == b'_' && i > start && b[i - 1].is_ascii_digit()
            {
                let mut j = i + 1;
                while j < len && b[j] == b'_' {
                    j += 1;
                }
                if j < len && b[j].is_ascii_digit() {
                    i = j;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        i
    };

    let after_int = scan_run(b, i);
    if after_int > i {
        any_digit = true;
    }
    i = after_int;
    if i < len && b[i] == b'.' {
        i += 1;
        let after_frac = scan_run(b, i);
        if after_frac > i {
            any_digit = true;
        }
        i = after_frac;
    }
    if !any_digit {
        return None;
    }
    if i < len && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < len && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        let after_exp = scan_run(b, j);
        if after_exp > j {
            i = after_exp; // a valid exponent — consume it
        }
        // else: the `e` is not part of the number; stop before it.
    }

    let lexeme = std::str::from_utf8(&b[start..i]).ok()?;
    let cleaned = if lexeme.contains('_') {
        Cow::Owned(lexeme.replace('_', ""))
    } else {
        Cow::Borrowed(lexeme)
    };
    let mag: f64 = cleaned.parse().ok()?;
    let value = if negative { -mag } else { mag };
    Some(Parsed {
        number: Number::Double(value),
        end: i,
    })
}

/// Parse the alphabetic `Inf`/`Infinity`/`NaN`/`NaN(hex)` forms (case-insensitive).
fn parse_inf_nan(b: &[u8], start: usize, negative: bool) -> Option<Parsed> {
    let len = b.len();
    // Case-insensitive prefix match; returns the end offset if matched.
    let matches_ci = |word: &[u8], at: usize| -> bool {
        at + word.len() <= len && b[at..at + word.len()].eq_ignore_ascii_case(word)
    };

    if matches_ci(b"inf", start) {
        let mut end = start + 3;
        if matches_ci(b"inity", end) {
            end += 5; // the full "Infinity"
        }
        let v = if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
        return Some(Parsed {
            number: Number::Double(v),
            end,
        });
    }

    if matches_ci(b"nan", start) {
        let mut end = start + 3;
        let mut payload = None;
        // Optional `(hexdigits)` payload.
        if end < len && b[end] == b'(' {
            let p0 = end + 1;
            let mut p = p0;
            let mut acc: u64 = 0;
            while p < len && p - p0 < 16 {
                match digit_val(b[p], 16) {
                    Some(d) => {
                        acc = (acc << 4) | u64::from(d);
                        p += 1;
                    }
                    None => break,
                }
            }
            if p > p0 && p < len && b[p] == b')' {
                payload = Some(acc);
                end = p + 1;
            }
            // A malformed payload leaves `end` at the bare "NaN".
        }
        return Some(Parsed {
            number: Number::Nan { negative, payload },
            end,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(s: &str) -> Option<Number> {
        parse_whole(s)
    }

    #[test]
    fn decimal_integers() {
        assert_eq!(n("0"), Some(Number::Int(0)));
        assert_eq!(n("42"), Some(Number::Int(42)));
        assert_eq!(n("-42"), Some(Number::Int(-42)));
        assert_eq!(n("+7"), Some(Number::Int(7)));
        // leading zero is DECIMAL in 9.0, not octal
        assert_eq!(n("0755"), Some(Number::Int(755)));
        assert_eq!(n("  13  "), Some(Number::Int(13))); // surrounding whitespace ok
    }

    #[test]
    fn radix_prefixes() {
        assert_eq!(n("0xff"), Some(Number::Int(255)));
        assert_eq!(n("0XFF"), Some(Number::Int(255)));
        assert_eq!(n("0o17"), Some(Number::Int(15)));
        assert_eq!(n("0b1010"), Some(Number::Int(10)));
        assert_eq!(n("0d0755"), Some(Number::Int(755)));
        assert_eq!(n("-0x10"), Some(Number::Int(-16)));
        // a bare `0x` (no hex digit) is not a whole number
        assert_eq!(n("0x"), None);
    }

    #[test]
    fn underscores() {
        assert_eq!(n("1_000_000"), Some(Number::Int(1_000_000)));
        assert_eq!(n("1__000___000"), Some(Number::Int(1_000_000)));
        assert_eq!(n("0xff_ff"), Some(Number::Int(0xffff)));
        assert_eq!(n("0xff__ff"), Some(Number::Int(0xffff)));
        assert_eq!(n("0b1__0"), Some(Number::Int(2)));
        assert_eq!(n("0o7__7"), Some(Number::Int(63)));
        assert_eq!(n("1_0"), Some(Number::Int(10)));
        // misplaced underscores → not a whole number
        assert_eq!(n("_1"), None);
        assert_eq!(n("1_"), None);
        assert_eq!(n("1__"), None);
        assert_eq!(n("0x_f"), None);
    }

    #[test]
    fn i64_bounds_and_bignum() {
        assert_eq!(n("9223372036854775807"), Some(Number::Int(i64::MAX)));
        assert_eq!(n("-9223372036854775808"), Some(Number::Int(i64::MIN)));
        // one past i64::MAX → Big
        assert_eq!(
            n("9223372036854775808"),
            Some(Number::Big {
                negative: false,
                radix: Radix::Dec,
                digits: "9223372036854775808".to_owned()
            })
        );
        // a clearly huge value → Big
        assert_eq!(
            n("123456789012345678901234567890"),
            Some(Number::Big {
                negative: false,
                radix: Radix::Dec,
                digits: "123456789012345678901234567890".to_owned()
            })
        );
        // huge hex with separators → Big with cleaned digits
        assert_eq!(
            n("0xffff_ffff_ffff_ffff_f"),
            Some(Number::Big {
                negative: false,
                radix: Radix::Hex,
                digits: "fffffffffffffffff".to_owned()
            })
        );
    }

    #[test]
    fn floats() {
        assert_eq!(n("1.5"), Some(Number::Double(1.5)));
        assert_eq!(n(".5"), Some(Number::Double(0.5)));
        assert_eq!(n("1."), Some(Number::Double(1.0)));
        assert_eq!(n("-2.5e3"), Some(Number::Double(-2500.0)));
        assert_eq!(n("1E-3"), Some(Number::Double(0.001)));
        assert_eq!(n("1_0.5"), Some(Number::Double(10.5)));
        assert_eq!(n("1__0.5"), Some(Number::Double(10.5)));
        assert_eq!(n("1.0__2"), Some(Number::Double(1.02)));
        assert_eq!(n("1_.0"), None);
        assert_eq!(n("1._0"), None);
    }

    #[test]
    fn inf_and_nan() {
        assert_eq!(n("Inf"), Some(Number::Double(f64::INFINITY)));
        assert_eq!(n("infinity"), Some(Number::Double(f64::INFINITY)));
        assert_eq!(n("-Inf"), Some(Number::Double(f64::NEG_INFINITY)));
        assert_eq!(
            n("NaN"),
            Some(Number::Nan {
                negative: false,
                payload: None
            })
        );
        assert_eq!(
            n("NaN(1ff)"),
            Some(Number::Nan {
                negative: false,
                payload: Some(0x1ff)
            })
        );
    }

    #[test]
    fn integer_only_flag() {
        // integer_only stops at the `.` so a whole-parse of a float fails
        let f = ParseFlags {
            integer_only: true,
            ..Default::default()
        };
        let p = parse("12.5", f).unwrap();
        assert_eq!(p.number, Number::Int(12));
        assert_eq!(p.end, 2); // stopped before the `.`
    }

    #[test]
    fn not_numbers() {
        assert_eq!(n(""), None);
        assert_eq!(n("   "), None);
        assert_eq!(n("abc"), None);
        assert_eq!(n("12x"), None);
        assert_eq!(n("0x1.5"), None); // no hex floats
        assert_eq!(n("1.2.3"), None);
    }

    #[test]
    fn partial_parse_reports_end() {
        let p = parse("42 rest", ParseFlags::default()).unwrap();
        assert_eq!(p.number, Number::Int(42));
        assert_eq!(p.end, 2);
    }

    #[test]
    fn format_double_matches_tcl_print_double() {
        // Specials and signed zero (the `-0.0` sign must survive — regression).
        assert_eq!(format_double(f64::NAN), "NaN");
        assert_eq!(format_double(f64::INFINITY), "Inf");
        assert_eq!(format_double(f64::NEG_INFINITY), "-Inf");
        assert_eq!(format_double(0.0), "0.0");
        assert_eq!(format_double(-0.0), "-0.0");
        // Ordinary fixed-notation values keep the distinguishing `.0`.
        assert_eq!(format_double(2.0), "2.0");
        assert_eq!(format_double(1.5), "1.5");
        assert_eq!(format_double(0.5), "0.5");
        assert_eq!(format_double(0.0001), "0.0001");
        // Integer-valued doubles ≥ 1e16 keep `.0` (re-parse as Double, not Int):
        // tclsh renders these with the trailing `.0`, not as a bare integer.
        assert_eq!(format_double(1e16), "10000000000000000.0");
        assert_eq!(format_double(1.5e16), "15000000000000000.0");
        assert_eq!(format_double(-1e16), "-10000000000000000.0");
        // At 1e17 tclsh switches to scientific (`1e+17`, single-digit mantissa
        // stays bare, exponent has an explicit sign and natural width).
        assert_eq!(format_double(1e17), "1e+17");
        assert_eq!(format_double(-1e17), "-1e+17");
        assert_eq!(format_double(1e-5), "1e-5");
        assert_eq!(format_double(6.022e23), "6.022e+23");
    }

    #[test]
    fn format_double_round_trips_to_double_not_int() {
        // The point of the `.0`: every `format_double` output must re-parse as a
        // `Double` (never an `Int`), so the numeric tower stays put across a
        // string round-trip. 1e16/1e17 are the regression cases.
        for &v in &[2.0, 1e16, 1.5e16, 1e17, -1e16, -1e17, 1e-5, 1.0, 0.0, -0.0] {
            let s = format_double(v);
            match parse_whole(&s) {
                Some(Number::Double(_)) => {}
                other => panic!("format_double({v}) = {s:?} re-parsed as {other:?}, not Double"),
            }
        }
    }

    #[test]
    fn compare_int_double_is_exact_past_2_pow_53() {
        use core::cmp::Ordering::{Equal, Greater, Less};
        // The motivating cases (tclsh oracle): a both-as-f64 comparison calls
        // 9007199254740993 equal to 9007199254740992.0; Tcl compares exactly.
        //   expr {9007199254740993 == 9007199254740992.0} → 0
        //   expr {9007199254740993 >  9007199254740992.0} → 1
        assert_eq!(
            compare_int_double(9_007_199_254_740_993, 9_007_199_254_740_992.0),
            Some(Greater)
        );
        // The float literal ...993.0 itself rounds to ...992.0, so the wide
        // still orders above it: expr {9007199254740993 > 9007199254740993.0} → 1.
        assert_eq!(
            compare_int_double(9_007_199_254_740_993, 9_007_199_254_740_993.0),
            Some(Greater)
        );
        assert_eq!(
            compare_int_double(9_007_199_254_740_992, 9_007_199_254_740_992.0),
            Some(Equal)
        );
        // 20000000000000003 < 20000000000000004.0 — the exact case the C
        // comment in TclCompareTwoNumbers cites.
        assert_eq!(
            compare_int_double(20_000_000_000_000_003, 20_000_000_000_000_004.0),
            Some(Less)
        );
    }

    #[test]
    fn compare_int_double_small_and_fractional() {
        use core::cmp::Ordering::{Equal, Greater, Less};
        assert_eq!(compare_int_double(2, 2.5), Some(Less));
        assert_eq!(compare_int_double(3, 2.5), Some(Greater));
        assert_eq!(compare_int_double(-2, -2.5), Some(Greater));
        assert_eq!(compare_int_double(-3, -2.5), Some(Less));
        assert_eq!(compare_int_double(0, 0.0), Some(Equal));
        assert_eq!(compare_int_double(0, -0.0), Some(Equal));
        assert_eq!(compare_int_double(0, -0.5), Some(Greater));
        assert_eq!(compare_int_double(0, 0.5), Some(Less));
        assert_eq!(compare_int_double(5, 5.0), Some(Equal));
        assert_eq!(compare_int_double(-5, -5.0), Some(Equal));
        // Subnormals: integer part 0, fraction non-zero.
        assert_eq!(compare_int_double(0, f64::MIN_POSITIVE / 2.0), Some(Less));
        assert_eq!(
            compare_int_double(0, -f64::MIN_POSITIVE / 2.0),
            Some(Greater)
        );
    }

    #[test]
    fn compare_int_double_extremes() {
        use core::cmp::Ordering::{Equal, Greater, Less};
        // ±2¹²⁷: i128::MAX = 2¹²⁷−1 sits below 2¹²⁷; i128::MIN is exactly
        // −2¹²⁷. 2⁶³ marks the i64-boundary sliver.
        const TWO_POW_127: f64 = 170_141_183_460_469_231_731_687_303_715_884_105_728.0;
        const TWO_POW_63: f64 = 9_223_372_036_854_775_808.0;
        // NaN is unordered — the caller applies Tcl's "!= true, rest false".
        assert_eq!(compare_int_double(1, f64::NAN), None);
        // Infinities order around every integer.
        assert_eq!(compare_int_double(i128::MAX, f64::INFINITY), Some(Less));
        assert_eq!(
            compare_int_double(i128::MIN, f64::NEG_INFINITY),
            Some(Greater)
        );
        assert_eq!(compare_int_double(i128::MAX, TWO_POW_127), Some(Less));
        assert_eq!(compare_int_double(i128::MIN, -TWO_POW_127), Some(Equal));
        assert_eq!(compare_int_double(0, -TWO_POW_127), Some(Greater));
        // 2⁶³ exactly vs the neighbouring wides (the i64-boundary sliver).
        assert_eq!(
            compare_int_double(i128::from(i64::MAX), TWO_POW_63),
            Some(Less)
        );
        assert_eq!(
            compare_int_double(i128::from(i64::MIN), -TWO_POW_63),
            Some(Equal)
        );
        // A wide whose f64 rounding lands ON 2⁶³ still orders exactly.
        assert_eq!(
            compare_int_double(9_223_372_036_854_775_807, 9.3e18),
            Some(Less)
        );
        assert_eq!(
            compare_int_double(-9_223_372_036_854_775_807, -9.3e18),
            Some(Greater)
        );
    }
}
