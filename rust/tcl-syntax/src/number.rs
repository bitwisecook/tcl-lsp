//! Tcl numeric-literal grammar — the shared `TclParseNumber` port.
//!
//! Classifies a string as a Tcl 9.0 number, re-derived from reference
//! `tmp/tcl9.0.3/generic/tclStrToD.c::TclParseNumber`. This is the *parse*
//! direction of the numeric tower (string → number); the *format* direction
//! (number → shortest-round-trip string) and the bignum arithmetic live with
//! each consumer (the runtime over libtommath `mp_int`, the compiler's
//! const-folder over its own value type) — both classify with this one grammar.
//!
//! The output is one of the four tower types ([`Number`]): a wide [`Int`] when
//! the integer fits `i64`, a [`Big`] (sign + radix + cleaned digits, for the
//! consumer to build a bignum from) when it overflows, a [`Double`] for floats
//! and `±Inf`, or a [`Nan`]. There is **no** boolean handling here —
//! `true`/`yes`/`on`… are `Tcl_GetBoolean`'s job, not `TclParseNumber`'s.
//!
//! ## Tcl 9.0 forms
//!
//! - Optional sign, optional surrounding whitespace (unless [`ParseFlags::no_whitespace`]).
//! - Radix prefixes `0x`/`0X`, `0o`/`0O`, `0b`/`0B`, `0d`/`0D` (case-insensitive,
//!   each needs ≥1 following digit). **A bare leading `0` is decimal** (`0755`
//!   == 755 — the 8.x octal-by-leading-zero rule is gone in 9.0).
//! - `_` digit separators between two same-base digits (`1_000`, `0xff_ff`),
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

/// Format an `f64` as Tcl's canonical double string (`Tcl_PrintDouble`-style):
/// `NaN`, `Inf`/`-Inf`, an integer-valued finite double gets a `.0` suffix
/// (so it re-parses as a double, e.g. `2.0` not `2`), else the shortest decimal
/// that round-trips (Rust's `{}` for `f64`). The one shared number→string
/// formatter for the runtime's `double` rep and the compiler's const-folder.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn format_double(f: f64) -> String {
    if f.is_nan() {
        "NaN".to_owned()
    } else if f.is_infinite() {
        if f.is_sign_negative() {
            "-Inf".to_owned()
        } else {
            "Inf".to_owned()
        }
    } else if f.fract() == 0.0 && f.abs() < 1e16 {
        format!("{}.0", f as i64)
    } else {
        format!("{f}")
    }
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
            && i + 1 < len
            && digit_val(b[i + 1], radix).is_some()
            && digit_val(b[i - 1], radix).is_some()
        {
            saw_underscore = true;
            i += 1;
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
fn parse_decimal_float(b: &[u8], start: usize, negative: bool, flags: ParseFlags) -> Option<Parsed> {
    let len = b.len();
    let mut i = start;
    let mut any_digit = false;

    let scan_run = |b: &[u8], mut i: usize| -> usize {
        while i < len {
            let is_digit = b[i].is_ascii_digit();
            let is_sep = !flags.no_underscore
                && b[i] == b'_'
                && i > start
                && i + 1 < len
                && b[i + 1].is_ascii_digit()
                && b[i - 1].is_ascii_digit();
            if is_digit || is_sep {
                i += 1;
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
        assert_eq!(n("0xff_ff"), Some(Number::Int(0xffff)));
        assert_eq!(n("1_0"), Some(Number::Int(10)));
        // misplaced underscores → not a whole number
        assert_eq!(n("_1"), None);
        assert_eq!(n("1_"), None);
        assert_eq!(n("1__0"), None);
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
}
