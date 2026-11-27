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

//! `scan` — parse a string under a format (the `sscanf` analogue), per
//! `tmp/tcl9.0.4/generic/tclScan.c` (`Tcl_ScanObjCmd`).
//!
//! This is the **pure matching engine**: [`scan_match`] takes the input and
//! format as code-point slices and returns the scanned values *typed*
//! (int/double/string), the conversion count, and whether EOF preceded any
//! conversion. It names no value model and no interp, so each runtime keeps a
//! thin `scan` adapter that reads argv, calls [`scan_match`], then either
//! assigns the values to `varName`s (returning the count, `-1` on EOF before
//! any conversion) or, with no vars, collects them into a list (*inline* mode).
//!
//! Conversions: `%d`/`%i`/`%u`/`%o`/`%x`/`%X`/`%b`/`%c`/`%s`/`%e`/`%E`/`%f`/
//! `%g`/`%G`/`%[...]`/`%n`/`%%`, with `*` (suppress), a field width, and
//! ignored size modifiers (`h`/`l`/`L`/…). The uppercase spellings are real
//! and case-equivalent to their lowercase twins, not dead arms — tclsh8.6.18
//! and tclsh9.0.4 both answer `scan 42 %X v` with `1` and `v` `66`, and
//! `scan 42 %E v` / `scan 42 %G v` with `42.0` (#2077). Uppercase is not a
//! blanket rule in either command, though: neither `scan` nor `format`
//! accepts `%F` (`format %F 1.5` → `bad field specifier "F"`). Operates on Unicode code points (so `%c` and widths count
//! characters). The specifier *grammar* is shared
//! ([`tcl_syntax::scan::parse_conversion`]); this module is the matcher and the
//! cross-specifier validator that build on it.

use tcl_dialect::{ScanGrammar, ScanIntWidth};
use tcl_syntax::scan::{CharSet, ScanConversion, SizeModifier, parse_conversion};

/// One scanned value, typed so each runtime builds its natural object
/// (`%d`/`%x`/…→int, `%e`/`%f`/`%g`→double, `%s`/`%[`→string, `%c`→the code
/// point as an int, `%n`→the count consumed).
#[derive(Debug, Clone, PartialEq)]
pub enum Scanned {
    /// An integer conversion (`%d`/`%i`/`%u`/`%o`/`%x`/`%b`/`%c`/`%n`).
    Int(i64),
    /// A `%u` whose value is negative as a C integer, read as unsigned 64-bit.
    Unsigned(u64),
    /// An `ll`-sized integer beyond 64 bits: its sign and digits in `radix`.
    Big {
        /// Whether a leading `-` was read.
        negative: bool,
        /// The conversion's radix.
        radix: u32,
        /// The digits read, without sign or prefix.
        digits: String,
    },
    /// A floating-point conversion (`%e`/`%f`/`%g`).
    Double(f64),
    /// A string conversion (`%s`/`%[...]`).
    Str(String),
}

impl Scanned {
    /// The decimal spelling of an integer value ([`Self::Int`],
    /// [`Self::Unsigned`] or [`Self::Big`]); `None` for a double or string.
    #[must_use]
    pub fn integer_decimal(&self) -> Option<String> {
        match self {
            Self::Int(value) => Some(value.to_string()),
            Self::Unsigned(value) => Some(value.to_string()),
            Self::Big {
                negative,
                radix,
                digits,
            } => {
                const BASE: u64 = 1_000_000_000;
                let mut limbs: Vec<u64> = vec![0];
                for digit in digits.chars() {
                    let mut carry = u64::from(digit.to_digit(*radix)?);
                    for limb in &mut limbs {
                        let next = *limb * u64::from(*radix) + carry;
                        *limb = next % BASE;
                        carry = next / BASE;
                    }
                    while carry > 0 {
                        limbs.push(carry % BASE);
                        carry /= BASE;
                    }
                }
                let mut text = String::new();
                if *negative && limbs.iter().any(|&limb| limb != 0) {
                    text.push('-');
                }
                let mut rest = limbs.iter().rev();
                text.push_str(&rest.next()?.to_string());
                for limb in rest {
                    use std::fmt::Write as _;
                    write!(text, "{limb:09}").expect("writing to a String cannot fail");
                }
                Some(text)
            }
            Self::Double(_) | Self::Str(_) => None,
        }
    }
}

/// The result of matching a format against an input.
pub struct ScanOutcome {
    /// Per non-suppressed conversion, in order: the value, or `None` if the
    /// conversion failed (inline callers pad a failure with an empty string).
    pub values: Vec<Option<Scanned>>,
    /// Successful conversions, a suppressed one and every `%n` included —
    /// C's `nconversions` (`tclScan.c`), which decides the underflow: only
    /// an input that ran out before any of them returns `-1`. The
    /// variable-mode count is the conversions assigned, the leading
    /// non-`None` [`Self::values`].
    pub nconv: usize,
    /// Whether the input reached EOF before any conversion matched (variable
    /// mode then returns `-1`).
    pub eof_before_conv: bool,
    /// A conversion C refuses after reading it: `%llu` of a negative number
    /// is `unsigned bignum scans are invalid` (`TCL FORMAT BADUNSIGNED`).
    pub error: Option<crate::CmdError>,
}

/// Validate a scan `fmt` the way C's `ValidateFormat` (`tclScan.c`) does, before
/// any matching: `num_vars` is the count of `varName` arguments (`0` for the
/// inline form). On success returns the number of variables the format requires;
/// on failure the diagnostic message (the runtime turns it into an error).
///
/// This catches the malformed-format cases (scan-3.x / scan-8.x): mixing `%` and
/// `%n$`, an out-of-range / duplicated `%n$` index, a width on `%c`, an
/// unterminated `%[`, a bad conversion character, and a variable/specifier count
/// mismatch.
pub fn validate_format(fmt: &[char], num_vars: usize) -> Result<usize, String> {
    let mut nassign: Vec<u32> = vec![0; num_vars];
    let mut got_xpg = false;
    let mut got_sequential = false;
    let mut obj_index: usize = 0;
    let mut xpg_size: usize = 0;

    let mixed = || "cannot mix \"%\" and \"%n$\" conversion specifiers".to_string();
    // `got_xpg` distinguishes the XPG index error from the plain count mismatch.
    let bad_index = |xpg: bool| {
        if xpg {
            "\"%n$\" argument index out of range".to_string()
        } else {
            "different numbers of variable names and field specifiers".to_string()
        }
    };

    let mut fi = 0;
    while fi < fmt.len() {
        if fmt[fi] != '%' {
            fi += 1;
            continue;
        }
        fi += 1;
        // Per-specifier grammar + validation (width on `%c`, bad verb, an
        // unterminated `%[`, …) is the shared parser's job.
        let conv = parse_conversion(fmt, &mut fi).map_err(|e| e.message())?;
        if conv.verb == '%' {
            continue;
        }

        // Cross-specifier rules: `%` and `%n$` cannot be mixed, and an XPG index
        // is bounded by the variable count (or grows the inline total).
        if let Some(idx) = conv.xpg_index {
            got_xpg = true;
            if got_sequential {
                return Err(mixed());
            }
            // 0 and >= INT_MAX (9.0 supports < INT_MAX args) are out of range.
            if idx == 0 || idx >= 2_147_483_647 {
                return Err(bad_index(true));
            }
            obj_index = idx as usize - 1;
            if num_vars != 0 && obj_index >= num_vars {
                return Err(bad_index(true));
            }
            if num_vars == 0 {
                xpg_size = xpg_size.max(idx as usize);
            }
        } else {
            got_sequential = true;
            if got_xpg {
                return Err(mixed());
            }
        }

        if !conv.suppress {
            if num_vars != 0 && obj_index >= num_vars {
                return Err(bad_index(got_xpg));
            }
            if obj_index >= nassign.len() {
                nassign.resize(obj_index + 1, 0);
            }
            nassign[obj_index] += 1;
            obj_index += 1;
        }
    }

    let total = if num_vars != 0 {
        num_vars
    } else if xpg_size != 0 {
        xpg_size
    } else {
        obj_index
    };
    for k in 0..total {
        let cnt = nassign.get(k).copied().unwrap_or(0);
        if cnt > 1 {
            return Err(
                "variable is assigned by multiple \"%n$\" conversion specifiers".to_string(),
            );
        } else if xpg_size == 0 && cnt == 0 {
            return Err("variable is not assigned by any conversion specifiers".to_string());
        }
    }
    Ok(total)
}

/// Match a validated sole `%c` directive using C86's original counted bytes.
/// Other formats and positional slots beyond the first retain the general matcher.
/// The caller supplies actual native C86 authority and original String getters.
///
/// # Errors
/// Refuses a decoder result outside the owned C-string prefix.
pub fn plan_c86_character_scan(
    source: &[u8],
    fmt: &[char],
) -> Result<Option<ScanOutcome>, tcl_syntax::value::ValueError> {
    use tcl_syntax::{native_tcl_utf::NativeTclUtf, value::ValueError};

    if fmt.first() != Some(&'%') {
        return Ok(None);
    }
    let mut end = 1;
    let Ok(conversion) = parse_conversion(fmt, &mut end) else {
        return Ok(None);
    };
    if end != fmt.len()
        || conversion.verb != 'c'
        || conversion.xpg_index.is_some_and(|index| index != 1)
    {
        return Ok(None);
    }
    // Tcl_ScanObjCmd stops at literal NUL; modified UTF-8 NUL is a character.
    let source = &source[..source
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(source.len())];
    let value = if source.is_empty() {
        None
    } else {
        let unavailable = || ValueError::CommandProtocolUnavailable("original C86 scan extent");
        let unit = NativeTclUtf::for_version(tcl_dialect::TclVersion::V8_6)
            .decode_c86_ucs4(source)
            .filter(|unit| unit.width != 0 && unit.width <= source.len())
            .ok_or_else(unavailable)?;
        Some(Scanned::Int(i64::from(unit.value)))
    };
    Ok(Some(ScanOutcome {
        nconv: usize::from(value.is_some()),
        eof_before_conv: value.is_none(),
        error: None,
        values: if conversion.suppress {
            Vec::new()
        } else {
            vec![value]
        },
    }))
}

/// Match `fmt` against `input` (both code-point slices) under the default
/// grammar, Tcl 8.5 and 8.6's full-width integers. Pure: no value model, no
/// interp.
#[must_use]
pub fn scan_match(input: &[char], fmt: &[char]) -> ScanOutcome {
    scan_match_in(input, fmt, ScanGrammar::default())
}

/// Match `fmt` against `input` under the release's `scan` grammar.
#[must_use]
pub fn scan_match_in(input: &[char], fmt: &[char], grammar: ScanGrammar) -> ScanOutcome {
    let mut ii = 0; // input cursor (chars)
    let mut fi = 0; // format cursor (chars)
    let mut values: Vec<Option<Scanned>> = Vec::new();
    let mut nconv = 0; // C's `nconversions`: every success, `%n` included
    let mut eof_before_conv = false;
    let mut error = None;

    while fi < fmt.len() {
        let fc = fmt[fi];
        if fc.is_whitespace() {
            while ii < input.len() && input[ii].is_whitespace() {
                ii += 1;
            }
            fi += 1;
            continue;
        }
        if fc != '%' {
            if ii < input.len() && input[ii] == fc {
                ii += 1;
                fi += 1;
                continue;
            }
            // Running out of input (rather than a wrong char) before any
            // conversion is an underflow → -1 (scan-4.15).
            if ii >= input.len() && nconv == 0 {
                eof_before_conv = true;
            }
            break; // literal mismatch
        }
        // A conversion specifier — parsed through the shared grammar (the
        // runtime rejects malformed formats before matching, so a parse error
        // here just stops the scan).
        fi += 1;
        let mut ci = fi;
        let Ok(conv) = parse_conversion(fmt, &mut ci) else {
            break;
        };
        fi = ci;

        // `%%` matches a literal `%`.
        if conv.verb == '%' {
            if ii < input.len() && input[ii] == '%' {
                ii += 1;
                continue;
            }
            break;
        }
        // `%n` reports the characters consumed so far. It consumes no input,
        // and counts as a conversion whether or not it is suppressed, as C's
        // `nconversions++` does: `scan {} %n%d n a` is 1 and `scan {} %*n%d
        // a` is 0, never the underflow's -1 (tclsh 8.4 to 9.1).
        if conv.verb == 'n' {
            if !conv.suppress {
                values.push(Some(Scanned::Int(i64::try_from(ii).unwrap_or(i64::MAX))));
            }
            nconv += 1;
            continue;
        }

        // Most conversions skip leading whitespace; `%c` and `%[` do not.
        if !matches!(conv.verb, 'c' | '[') {
            while ii < input.len() && input[ii].is_whitespace() {
                ii += 1;
            }
        }
        if ii >= input.len() && nconv == 0 {
            eof_before_conv = true;
        }

        let scanned = match scan_one(input, &mut ii, &conv, grammar.int_width) {
            Ok(scanned) => scanned,
            Err(refusal) => {
                error = Some(refusal);
                break;
            }
        };
        if let Some(v) = scanned {
            // A suppressed success counts too: `scan 5 %*d%d a` is 0, not -1.
            if !conv.suppress {
                values.push(Some(v));
            }
            nconv += 1;
        } else {
            // Conversion failed: stop. A numeric conversion that consumed only a
            // leading sign / `0x` before EOF is an underflow (-1), unlike a stop
            // on a non-matching character (scan-4.44/4.55).
            if nconv == 0 && numeric_underflow(input, ii, conv.verb) {
                eof_before_conv = true;
            }
            // Inline mode keeps a hole.
            if !conv.suppress {
                values.push(None);
            }
            break;
        }
    }

    ScanOutcome {
        values,
        nconv,
        eof_before_conv,
        error,
    }
}

/// Scan one field for the parsed `conv`, advancing `ii`. Returns the typed
/// value, or `None` if nothing valid was read.
fn scan_one(
    input: &[char],
    ii: &mut usize,
    conv: &ScanConversion,
    width_axis: ScanIntWidth,
) -> Result<Option<Scanned>, crate::CmdError> {
    // A width of 0 means "no limit" (`%0s` reads the whole word, scan-4.21/4.25).
    let width = conv.width.filter(|&w| w != 0).unwrap_or(usize::MAX);
    match conv.verb {
        'c' => {
            // One character → its code point. (No width, no whitespace skip.)
            if *ii >= input.len() {
                return Ok(None);
            }
            let cp = i64::from(input[*ii] as u32);
            *ii += 1;
            Ok(Some(Scanned::Int(cp)))
        }
        'd' | 'u' | 'i' | 'o' | 'x' | 'X' | 'b' => {
            let text = match conv.verb {
                'i' => scan_int_auto(input, ii, width),
                'o' => scan_int(input, ii, width, 8),
                'x' | 'X' => scan_int(input, ii, width, 16),
                'b' => scan_int(input, ii, width, 2),
                _ => scan_int(input, ii, width, 10),
            };
            text.map(|text| text.resolve(conv, width_axis)).transpose()
        }
        's' => {
            let start = *ii;
            let mut n = 0;
            while *ii < input.len() && !input[*ii].is_whitespace() && n < width {
                *ii += 1;
                n += 1;
            }
            if *ii == start {
                return Ok(None);
            }
            Ok(Some(Scanned::Str(input[start..*ii].iter().collect())))
        }
        'e' | 'E' | 'f' | 'g' | 'G' => Ok(scan_float(input, ii, width)),
        '[' => Ok(conv
            .charset
            .as_ref()
            .and_then(|set| scan_set(input, ii, width, set))),
        _ => Ok(None), // unknown conversion
    }
}

/// Whether a *failed* numeric conversion at `start` ran out of input — it
/// consumed only a leading sign (and, for `%x`/`%i`, a `0x` prefix) before EOF.
/// This is C's underflow (which yields -1), as opposed to stopping on a present
/// non-matching character (which yields the conversion count).
fn numeric_underflow(input: &[char], start: usize, verb: char) -> bool {
    if !matches!(
        verb,
        'd' | 'i' | 'o' | 'x' | 'X' | 'b' | 'u' | 'e' | 'E' | 'f' | 'g' | 'G'
    ) {
        return false;
    }
    let mut k = start;
    if matches!(input.get(k), Some('+' | '-')) {
        k += 1;
    }
    if matches!(verb, 'x' | 'X' | 'i')
        && input.get(k) == Some(&'0')
        && matches!(input.get(k + 1), Some('x' | 'X'))
    {
        k += 2;
    }
    k >= input.len()
}

/// The sign, radix and digits of one integer conversion, before the release
/// reads them at its width.
struct IntText {
    negative: bool,
    radix: u32,
    digits: String,
}

impl IntText {
    /// The digits' magnitude, or `None` beyond 64 bits.
    fn magnitude(&self) -> Option<u64> {
        let mut value: u64 = 0;
        for digit in self.digits.chars() {
            value = value
                .checked_mul(u64::from(self.radix))?
                .checked_add(u64::from(digit.to_digit(self.radix)?))?;
        }
        Some(value)
    }

    /// The signed value reduced modulo 2^64, or `None` beyond 64 bits.
    fn wrapped(&self) -> Option<i64> {
        self.magnitude().map(|magnitude| {
            let bits = magnitude.cast_signed();
            if self.negative {
                bits.wrapping_neg()
            } else {
                bits
            }
        })
    }

    /// The value within `i64`, or `None`.
    fn exact(&self) -> Option<i64> {
        let magnitude = self.magnitude()?;
        if self.negative {
            0i64.checked_sub_unsigned(magnitude)
        } else {
            i64::try_from(magnitude).ok()
        }
    }

    fn saturated(&self, low: i64, high: i64) -> i64 {
        if self.negative { low } else { high }
    }

    /// C's `strtol` into a 64-bit `long`: the value clamped to its range.
    fn strtol(&self) -> i64 {
        self.exact()
            .unwrap_or_else(|| self.saturated(i64::MIN, i64::MAX))
    }

    /// C's `strtoul`: the magnitude negated modulo 2^64, or `ULONG_MAX` on
    /// overflow whatever the sign.
    fn strtoul(&self) -> i64 {
        self.wrapped().unwrap_or(-1)
    }

    /// Tcl 8's `TclGetLongFromObj`: a magnitude within 64 bits unsigned wraps.
    fn tcl8_long(&self) -> i64 {
        self.wrapped()
            .unwrap_or_else(|| self.saturated(i64::MIN, i64::MAX))
    }

    /// Tcl 9's `TclGetLongFromObj`: `LONG_MIN..=ULONG_MAX`, the top wrapping.
    fn tcl9_long(&self) -> Option<i64> {
        if self.negative {
            self.exact()
        } else {
            self.wrapped()
        }
    }

    /// Read the text as `width_axis` reads `conv`'s integer (`tclScan.c`).
    fn resolve(
        self,
        conv: &ScanConversion,
        width_axis: ScanIntWidth,
    ) -> Result<Scanned, crate::CmdError> {
        let unsigned = conv.verb == 'u';
        let spell = |value: i64| {
            if unsigned && value < 0 {
                // C's `(unsigned long) value`.
                Scanned::Unsigned(value.cast_unsigned())
            } else {
                Scanned::Int(value)
            }
        };
        let size = conv.size.filter(|size| *size != SizeModifier::Short);
        Ok(match width_axis {
            // Tcl 8.4 reads `%d`/`%i` with `strtol` and the rest with
            // `strtoul`, and has no bignum size: `L` and `ll` are `long`.
            ScanIntWidth::Wrap32Narrow => {
                let value = if matches!(conv.verb, 'd' | 'i') {
                    self.strtol()
                } else {
                    self.strtoul()
                };
                // C's `(int) value` for a value within `0..=UINT_MAX`.
                match u32::try_from(value) {
                    Ok(narrow) if size.is_none() => Scanned::Int(i64::from(narrow.cast_signed())),
                    _ => spell(value),
                }
            }
            _ if size == Some(SizeModifier::Big) => {
                if unsigned && self.negative && self.magnitude() != Some(0) {
                    return Err(crate::CmdError::with_error_code(
                        "unsigned bignum scans are invalid",
                        "TCL FORMAT BADUNSIGNED",
                    ));
                }
                match self.exact() {
                    Some(value) => Scanned::Int(value),
                    None => Scanned::Big {
                        negative: self.negative,
                        radix: self.radix,
                        digits: self.digits,
                    },
                }
            }
            ScanIntWidth::Full => spell(self.tcl8_long()),
            ScanIntWidth::Saturate32 if size == Some(SizeModifier::Long) => spell(
                self.exact()
                    .unwrap_or_else(|| self.saturated(i64::MIN, i64::MAX)),
            ),
            ScanIntWidth::Saturate32 => {
                // `TclGetIntFromObj`: `INT_MIN..=UINT_MAX`, the top wrapping.
                let int = self
                    .tcl9_long()
                    .and_then(|long| {
                        i32::try_from(long)
                            .ok()
                            .or_else(|| u32::try_from(long).ok().map(u32::cast_signed))
                    })
                    .map_or_else(
                        || self.saturated(i64::from(i32::MIN), i64::from(i32::MAX)),
                        i64::from,
                    );
                spell(int)
            }
        })
    }
}

/// `%d`/`%o`/`%x`/`%b`/`%u`: an optionally-signed integer in `radix`. Every
/// conversion reads a leading sign (C's `scanf` does, even for `%x`/`%o`/`%u`).
fn scan_int(input: &[char], ii: &mut usize, width: usize, radix: u32) -> Option<IntText> {
    let start = *ii;
    let mut negative = false;
    let mut n = 0;
    if *ii < input.len() && (input[*ii] == '+' || input[*ii] == '-') {
        negative = input[*ii] == '-';
        *ii += 1;
        n += 1;
    }
    // Allow a `0x`/`0b` prefix for hex/binary.
    if (radix == 16 || radix == 2) && *ii + 1 < input.len() && input[*ii] == '0' {
        let p = input[*ii + 1].to_ascii_lowercase();
        if (radix == 16 && p == 'x') || (radix == 2 && p == 'b') {
            *ii += 2;
            n += 2;
        }
    }
    let digits_start = *ii;
    while *ii < input.len() && n < width && input[*ii].is_digit(radix) {
        *ii += 1;
        n += 1;
    }
    if *ii == digits_start {
        *ii = start;
        return None;
    }
    Some(IntText {
        negative,
        radix,
        digits: input[digits_start..*ii].iter().collect(),
    })
}

/// `%i`: an integer with C base-0 detection — `0x`/`0X` (when a hex digit
/// follows) is hex, a leading `0` is octal, else decimal. `0b` is *not*
/// recognised by scan's `%i` (scan-4.41), and a `0x` with no hex digit folds
/// back to the `0` (scan-4.45).
fn scan_int_auto(input: &[char], ii: &mut usize, width: usize) -> Option<IntText> {
    let save = *ii;
    let mut negative = false;
    let mut n = 0;
    if *ii < input.len() && (input[*ii] == '+' || input[*ii] == '-') {
        negative = input[*ii] == '-';
        *ii += 1;
        n += 1;
    }
    let radix = if input.get(*ii) == Some(&'0')
        && matches!(input.get(*ii + 1), Some('x' | 'X'))
        && input.get(*ii + 2).is_some_and(char::is_ascii_hexdigit)
    {
        *ii += 2; // consume the `0x` prefix
        n += 2;
        16u32
    } else if input.get(*ii) == Some(&'0') {
        8u32 // a leading `0` (the `0` itself is the first octal digit)
    } else {
        10u32
    };
    let ds = *ii;
    while *ii < input.len() && n < width && input[*ii].is_digit(radix) {
        *ii += 1;
        n += 1;
    }
    if *ii == ds {
        *ii = save;
        return None;
    }
    Some(IntText {
        negative,
        radix,
        digits: input[ds..*ii].iter().collect(),
    })
}

/// `%e`/`%f`/`%g`: a floating-point number.
fn scan_float(input: &[char], ii: &mut usize, width: usize) -> Option<Scanned> {
    let start = *ii;
    let mut s = String::new();
    let mut n = 0;
    let take = |c: char, ii: &mut usize, s: &mut String, n: &mut usize| {
        s.push(c);
        *ii += 1;
        *n += 1;
    };
    if *ii < input.len() && (input[*ii] == '+' || input[*ii] == '-') {
        take(input[*ii], ii, &mut s, &mut n);
    }
    let mut digits = 0;
    while *ii < input.len() && n < width && input[*ii].is_ascii_digit() {
        digits += 1;
        take(input[*ii], ii, &mut s, &mut n);
    }
    if *ii < input.len() && n < width && input[*ii] == '.' {
        take('.', ii, &mut s, &mut n);
        while *ii < input.len() && n < width && input[*ii].is_ascii_digit() {
            digits += 1;
            take(input[*ii], ii, &mut s, &mut n);
        }
    }
    if digits == 0 {
        *ii = start;
        return None;
    }
    if *ii < input.len() && n < width && matches!(input[*ii], 'e' | 'E') {
        let mut probe = *ii + 1;
        if probe < input.len() && (input[probe] == '+' || input[probe] == '-') {
            probe += 1;
        }
        if probe < input.len() && input[probe].is_ascii_digit() {
            take(input[*ii], ii, &mut s, &mut n); // 'e'
            if *ii < input.len() && (input[*ii] == '+' || input[*ii] == '-') {
                take(input[*ii], ii, &mut s, &mut n);
            }
            while *ii < input.len() && n < width && input[*ii].is_ascii_digit() {
                take(input[*ii], ii, &mut s, &mut n);
            }
        }
    }
    let val: f64 = s.parse().ok()?;
    Some(Scanned::Double(val))
}

/// `%[...]`: a run of characters matching the already-parsed `set` (its `^`
/// negation handled by [`CharSet::matches`]).
fn scan_set(input: &[char], ii: &mut usize, width: usize, set: &CharSet) -> Option<Scanned> {
    let start = *ii;
    let mut n = 0;
    while *ii < input.len() && n < width {
        if !set.matches(input[*ii]) {
            break;
        }
        *ii += 1;
        n += 1;
    }
    if *ii == start {
        return None;
    }
    Some(Scanned::Str(input[start..*ii].iter().collect()))
}

/// The result list of a literal `scan` of `input` by `format` on the portable
/// ASCII subset, or `None` outside it.
/// It admits only the conversion kinds and result representations that every
/// release shares.
#[must_use]
pub fn scan_literal(input: &str, format: &str) -> Option<String> {
    if !input.is_ascii() || !format.is_ascii() {
        return None;
    }
    let format = format.chars().collect::<Vec<_>>();
    let has_hex_prefix = input.contains("0x") || input.contains("0X");
    let mut fields = 0;
    let mut at = 0;
    while at < format.len() {
        if format[at] != '%' {
            at += 1;
            continue;
        }
        at += 1;
        let conversion = parse_conversion(&format, &mut at).ok()?;
        if conversion.suppress
            || conversion.xpg_index.is_some()
            || conversion.width.is_some()
            || conversion.size.is_some()
            || !matches!(conversion.verb, '%' | 'd' | 'o' | 'x' | 'X' | 's' | 'c')
        {
            return None;
        }
        // The sign of a prefixed hex input differs in Tcl 8.4.
        if matches!(conversion.verb, 'x' | 'X') && has_hex_prefix {
            return None;
        }
        fields += usize::from(conversion.verb != '%');
    }
    if fields == 0 {
        return None;
    }
    let chars = input.chars().collect::<Vec<_>>();
    let outcome = scan_match(&chars, &format);
    if outcome.nconv != fields {
        return None;
    }
    let elements = outcome
        .values
        .into_iter()
        .map(|value| match value? {
            Scanned::Int(value) => Some(i32::try_from(value).ok()?.to_string()),
            Scanned::Str(value) if tcl_syntax::list::list_element(&value) == value => Some(value),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    String::from_utf8(tcl_syntax::list_result::portable_list_result(&elements)?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn basic_int_string() {
        let o = scan_match(&chars("42 abc"), &chars("%d %s"));
        assert_eq!(o.nconv, 2);
        assert_eq!(
            o.values,
            vec![Some(Scanned::Int(42)), Some(Scanned::Str("abc".into()))]
        );
        assert!(!o.eof_before_conv);
    }

    #[test]
    fn hex_and_char_and_set() {
        assert_eq!(
            scan_match(&chars("0xff"), &chars("%x")).values,
            vec![Some(Scanned::Int(255))]
        );
        // `%c` is the code point of the first character.
        assert_eq!(
            scan_match(&chars("Z"), &chars("%c")).values,
            vec![Some(Scanned::Int(90))]
        );
        // Scanset runs while in the set.
        assert_eq!(
            scan_match(&chars("hello123"), &chars("%[a-z]")).values,
            vec![Some(Scanned::Str("hello".into()))]
        );
    }

    #[test]
    fn float_suppress_and_width() {
        // Suppressed conversion is not collected, but counts as C's
        // `nconversions` does; width caps the field.
        let o = scan_match(&chars("3.5 99"), &chars("%f %*d"));
        assert_eq!(o.values, vec![Some(Scanned::Double(3.5))]);
        assert_eq!(o.nconv, 2);
        assert_eq!(
            scan_match(&chars("12345"), &chars("%2d")).values,
            vec![Some(Scanned::Int(12))]
        );
    }

    #[test]
    fn eof_before_any_conversion() {
        let o = scan_match(&chars(""), &chars("%d"));
        assert!(o.eof_before_conv);
        assert_eq!(o.nconv, 0);
    }

    #[test]
    fn percent_n_reports_consumed() {
        let o = scan_match(&chars("abcde"), &chars("%3s%n"));
        assert_eq!(
            o.values,
            vec![Some(Scanned::Str("abc".into())), Some(Scanned::Int(3))]
        );
        // `%n` counts as a conversion, as C's `nconversions` does.
        assert_eq!(o.nconv, 2);
    }

    #[test]
    fn percent_n_and_a_suppressed_success_prevent_the_underflow() {
        // tclsh 8.4 to 9.1: `scan {} %n%d n a` is 1 with `n` 0; `scan {}
        // %*n%d a` and `scan 5 %*d%d a` are 0; `scan {} %*d%d a` is -1.
        let o = scan_match(&chars(""), &chars("%n%d"));
        assert_eq!(o.values, vec![Some(Scanned::Int(0)), None]);
        assert!(o.nconv > 0 || !o.eof_before_conv);
        let o = scan_match(&chars(""), &chars("%*n%d"));
        assert_eq!(o.values, vec![None]);
        assert!(o.nconv > 0 || !o.eof_before_conv);
        let o = scan_match(&chars("5"), &chars("%*d%d"));
        assert_eq!(o.values, vec![None]);
        assert!(o.nconv > 0 || !o.eof_before_conv);
        let o = scan_match(&chars(""), &chars("%*d%d"));
        assert!(o.nconv == 0 && o.eof_before_conv);
    }

    /// tclsh 8.4.20, 8.5.19 / 8.6.18 and 9.0.4 / 9.1.0, one column each.
    #[test]
    fn integer_width_follows_each_release_grammar() {
        use tcl_dialect::{ScanGrammar, ScanIntWidth};
        let read = |input: &str, conversion: &str, int_width| {
            let outcome =
                scan_match_in(&chars(input), &chars(conversion), ScanGrammar { int_width });
            outcome.values[0]
                .as_ref()
                .unwrap()
                .integer_decimal()
                .unwrap()
        };
        for (input, conversion, tcl84, tcl85, tcl90) in [
            ("ffffffff", "%x", "-1", "4294967295", "-1"),
            ("1ffffffff", "%x", "8589934591", "8589934591", "2147483647"),
            (
                "-ffffffff",
                "%x",
                "-4294967295",
                "-4294967295",
                "-2147483648",
            ),
            (
                "3000000000",
                "%u",
                "-1294967296",
                "3000000000",
                "18446744072414584320",
            ),
            (
                "-1",
                "%u",
                "18446744073709551615",
                "18446744073709551615",
                "18446744073709551615",
            ),
            (
                "-2147483649",
                "%d",
                "-2147483649",
                "-2147483649",
                "-2147483648",
            ),
            ("ffffffffffffffff", "%x", "-1", "-1", "-1"),
            (
                "-8000000000000001",
                "%x",
                "9223372036854775807",
                "9223372036854775807",
                "-2147483648",
            ),
            (
                "10000000000000000",
                "%x",
                "-1",
                "9223372036854775807",
                "2147483647",
            ),
            ("ffffffff", "%hx", "-1", "4294967295", "-1"),
            ("ffffffff", "%lx", "4294967295", "4294967295", "4294967295"),
            ("ffffffffffffffff", "%lx", "-1", "-1", "9223372036854775807"),
            (
                "18446744073709551615",
                "%lu",
                "18446744073709551615",
                "18446744073709551615",
                "9223372036854775807",
            ),
        ] {
            for (width, expected) in [
                (ScanIntWidth::Wrap32Narrow, tcl84),
                (ScanIntWidth::Full, tcl85),
                (ScanIntWidth::Saturate32, tcl90),
            ] {
                assert_eq!(
                    read(input, conversion, width),
                    expected,
                    "{input} {conversion} {width:?}"
                );
            }
        }
    }

    /// tclsh 8.5.19 to 9.1.0: `%lld` keeps every digit, and `%llu` of a
    /// negative number is refused.
    #[test]
    fn bignum_size_keeps_digits_and_refuses_negative_unsigned() {
        let grammar = tcl_dialect::ScanGrammar {
            int_width: tcl_dialect::ScanIntWidth::Saturate32,
        };
        let outcome = scan_match_in(&chars("99999999999999999999"), &chars("%lld"), grammar);
        assert_eq!(
            outcome.values[0]
                .as_ref()
                .unwrap()
                .integer_decimal()
                .unwrap(),
            "99999999999999999999"
        );
        let outcome = scan_match_in(&chars("ffffffffffffffffff"), &chars("%llx"), grammar);
        assert_eq!(
            outcome.values[0]
                .as_ref()
                .unwrap()
                .integer_decimal()
                .unwrap(),
            "4722366482869645213695"
        );
        let error = scan_match_in(&chars("-1"), &chars("%llu"), grammar)
            .error
            .unwrap();
        assert_eq!(
            error.message().unwrap(),
            "unsigned bignum scans are invalid"
        );
        assert_eq!(error.error_code().unwrap(), Some("TCL FORMAT BADUNSIGNED"));
    }

    #[test]
    fn huge_field_width_saturates_without_overflow() {
        // A giant width in the format string must not overflow the usize
        // accumulator; it saturates and behaves as an unbounded field (the input
        // length is the real cap), so the whole string scans as one `%s`.
        let o = scan_match(&chars("hello"), &chars("%999999999999999999999s"));
        assert_eq!(o.values, vec![Some(Scanned::Str("hello".into()))]);
        assert_eq!(o.nconv, 1);
    }
}
