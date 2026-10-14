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

//! The two double spellings Tcl releases print, digit for digit.
//!
//! - [`power_of_two_digits`]: `TclDoubleDigits`' shortest mode for a normal
//!   power of two, a case Tcl 8.5–9.1 get wrong and [`super::format_double`]
//!   reproduces.
//! - [`precision_12`]: `%.12g` plus the `.0` rule, the spelling of Tcl 8.4's
//!   `Tcl_PrintDouble` at its default `tcl_precision` and of Jim's
//!   `UpdateStringOfDouble`.

/// The significant digits (no trailing zeros) and the decimal-point position
/// `TclDoubleDigits` produces in shortest mode for the normal double `2^k`.
///
/// For a power of two the next double below is a quarter-ulp away, not a
/// half, and `tclStrToD.c` (8.5.19 to 9.1.0: `ShorteningInt64Conversion`,
/// `ShorteningBignumConversionPowD`, `ShorteningBignumConversion`) tests the
/// remainder against the two half-gaps the wrong way round: the low
/// termination against `mplus`, half the gap above, and the high one against
/// `mminus`, half the gap below. The result is longer than the shortest for
/// 253 of the 2046 normal powers and does not re-read as `2^k` for 263 of
/// them (`2^64` prints `1.844674407370955e+19`). This is that loop in exact
/// integer arithmetic, so the formatter prints what tclsh prints.
pub(super) fn power_of_two_digits(k: i32) -> (Vec<u8>, i32) {
    // Every quantity is a multiple of `u`, half the gap below `2^k`
    // (`2^(k-54)`), over the decimal scale `S = 10^e`: the value is `2^54 u`,
    // Tcl's `mplus` is `2u` and its `mminus` is `u`.
    // floor(k · log10 2), off by at most one either way for |k| <= 1074.
    let mut e10 = (k * 78_913) >> 18;
    let mut unit = Big::one();
    unit.shl(u32::try_from(k - 54).unwrap_or(0));
    unit.mul_pow10(u32::try_from(-e10).unwrap_or(0));
    let mut scale = Big::one();
    scale.shl(u32::try_from(54 - k).unwrap_or(0));
    scale.mul_pow10(u32::try_from(e10).unwrap_or(0));
    let mut b = unit.clone();
    b.shl(54);
    let mut low = unit.clone();
    low.shl(1);
    let mut high = unit;
    // The estimate can be one out either way; correct it so 1 <= b/S < 10.
    if b.cmp(&scale).is_lt() {
        b.mul_small(10);
        low.mul_small(10);
        high.mul_small(10);
        e10 -= 1;
    } else {
        let mut ten_s = scale.clone();
        ten_s.mul_small(10);
        if !b.cmp(&ten_s).is_lt() {
            scale = ten_s;
            e10 += 1;
        }
    }

    let mut digits: Vec<u8> = Vec::with_capacity(20);
    loop {
        let mut digit = 0u8;
        while !b.cmp(&scale).is_lt() {
            b.sub(&scale);
            digit += 1;
        }
        // The mantissa of a power of two is even, so both tests are inclusive.
        if !b.cmp(&low).is_gt() {
            let mut twice = b.clone();
            twice.shl(1);
            let order = twice.cmp(&scale);
            if order.is_gt() || (order.is_eq() && digit % 2 == 1) {
                digit += 1;
            }
            digits.push(digit);
            break;
        }
        let mut reach = b.clone();
        reach.add(&high);
        if !reach.cmp(&scale).is_lt() {
            digits.push(digit + 1);
            break;
        }
        digits.push(digit);
        b.mul_small(10);
        low.mul_small(10);
        high.mul_small(10);
    }
    let mut decpt = e10 + 1;
    // A rounded-up digit of 10 carries into its predecessors (`BumpUp`).
    let mut i = digits.len() - 1;
    while digits[i] == 10 {
        digits[i] = 0;
        if i == 0 {
            digits.insert(0, 1);
            decpt += 1;
            break;
        }
        i -= 1;
        digits[i] += 1;
    }
    while digits.len() > 1 && digits.last() == Some(&0) {
        digits.pop();
    }
    (digits, decpt)
}

/// `f` as `sprintf("%.12g")` prints it, with `.0` appended when that output
/// has neither a `.` nor an exponent: the spelling of Tcl 8.4's
/// `Tcl_PrintDouble` (`tclUtil.c`, `tcl_precision` 12, which tests for any
/// alphabetic character) and of Jim's `UpdateStringOfDouble` (`jim.c`, which
/// tests for `e`). The two tests agree on every finite double. The exponent
/// keeps C's two-digit minimum (`1e-05`, `1e+16`). A non-finite value is
/// spelled `Inf`, `-Inf` or `NaN`, as Jim spells it; tclsh 8.4 raises before
/// an expression can produce one.
pub(super) fn precision_12(f: f64) -> String {
    // `%.12g`: twelve significant digits, scientific below 1e-4 or from 1e12.
    const PRECISION: i32 = 12;
    if f.is_nan() {
        return "NaN".to_owned();
    }
    if f.is_infinite() {
        return if f < 0.0 { "-Inf" } else { "Inf" }.to_owned();
    }
    // Rust rounds the exact binary value to 12 significant digits, half to
    // even — what glibc's `printf` does.
    let sci = format!("{:.11e}", f.abs());
    let (mantissa, exp) = sci
        .split_once('e')
        .expect("Rust's {:e} always emits an 'e' separator");
    let exp: i32 = exp.parse().expect("Rust's {:e} exponent is an integer");
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let digits = if f == 0.0 {
        "0"
    } else {
        digits.trim_end_matches('0')
    };
    let body = if !(-4..PRECISION).contains(&exp) {
        let (lead, rest) = digits.split_at(1);
        let point = if rest.is_empty() { "" } else { "." };
        let sign = if exp < 0 { '-' } else { '+' };
        format!("{lead}{point}{rest}e{sign}{:02}", exp.unsigned_abs())
    } else if exp < 0 {
        let zeros = usize::try_from(exp.unsigned_abs() - 1).unwrap_or(0);
        format!("0.{}{digits}", "0".repeat(zeros))
    } else {
        let point = usize::try_from(exp).unwrap_or(0) + 1;
        if digits.len() > point {
            let (int_part, frac) = digits.split_at(point);
            format!("{int_part}.{frac}")
        } else {
            format!("{digits}{}.0", "0".repeat(point - digits.len()))
        }
    };
    if f.is_sign_negative() {
        format!("-{body}")
    } else {
        body
    }
}

/// The few unsigned operations the power-of-two loop needs, over a fixed
/// array of little-endian 32-bit limbs: no value in it exceeds 2^1200
/// (`10^308 · 2^54` times the 10^17 the digit loop can add).
#[derive(Clone)]
struct Big([u32; LIMBS]);

const LIMBS: usize = 40;

impl Big {
    fn one() -> Self {
        let mut limbs = [0; LIMBS];
        limbs[0] = 1;
        Self(limbs)
    }

    fn shl(&mut self, bits: u32) {
        let words = usize::try_from(bits / 32).unwrap_or(LIMBS);
        let shift = bits % 32;
        for i in (0..LIMBS).rev() {
            let src = i.checked_sub(words);
            let hi = src.map_or(0, |s| self.0[s]);
            let lo = src.and_then(|s| s.checked_sub(1)).map_or(0, |s| self.0[s]);
            self.0[i] = if shift == 0 {
                hi
            } else {
                (hi << shift) | (lo >> (32 - shift))
            };
        }
    }

    fn mul_small(&mut self, factor: u32) {
        let mut carry = 0u64;
        for limb in &mut self.0 {
            let product = u64::from(*limb) * u64::from(factor) + carry;
            *limb = low_half(product);
            carry = product >> 32;
        }
        debug_assert_eq!(carry, 0, "power-of-two digit loop overflowed");
    }

    fn mul_pow10(&mut self, power: u32) {
        for _ in 0..power {
            self.mul_small(10);
        }
    }

    fn add(&mut self, other: &Self) {
        let mut carry = 0u64;
        for (limb, &addend) in self.0.iter_mut().zip(&other.0) {
            let sum = u64::from(*limb) + u64::from(addend) + carry;
            *limb = low_half(sum);
            carry = sum >> 32;
        }
        debug_assert_eq!(carry, 0, "power-of-two digit loop overflowed");
    }

    /// `self -= other`; the caller guarantees `self >= other`.
    fn sub(&mut self, other: &Self) {
        let mut borrow = 0;
        for (limb, &subtrahend) in self.0.iter_mut().zip(&other.0) {
            let (diff, under) = limb.overflowing_sub(subtrahend);
            let (diff, under_again) = diff.overflowing_sub(borrow);
            *limb = diff;
            borrow = u32::from(under || under_again);
        }
        debug_assert_eq!(borrow, 0, "power-of-two digit loop went negative");
    }

    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.iter().rev().cmp(other.0.iter().rev())
    }
}

/// The low 32 bits of `value`.
fn low_half(value: u64) -> u32 {
    let [a, b, c, d, ..] = value.to_le_bytes();
    u32::from_le_bytes([a, b, c, d])
}
