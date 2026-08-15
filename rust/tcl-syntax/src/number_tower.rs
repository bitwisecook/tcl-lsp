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

//! The numeric-tower **operator semantics**, written once, generic over the
//! big-integer backend.
//!
//! This crate already owns the tower's two string boundaries — the parse
//! direction ([`crate::number::parse`], the `TclParseNumber` port) and the
//! format direction ([`crate::number::format_double`], `Tcl_PrintDouble`).
//! This module adds the third piece: the **integer operator semantics** of
//! `tclExecute.c` (`INST_ADD` … `INST_EXPON`), so the promotion ladder,
//! floor division/modulus, the `**` special cases, shift sign-collapse, and
//! demote-when-fits live in exactly one place.
//!
//! ## The backend seam
//!
//! [`BigIntOps`] abstracts the arbitrary-precision integer. Adopters plug in
//! whichever backend they are built on:
//!
//! - the **faithful runtime** can implement it over the real libtommath
//!   `mp_int` (its bignum *is* `TCL_BIGNUM_TYPE`, the C-extension ABI — see
//!   `runtime/rust/src/bignum.rs`);
//! - the **compiler's const-folder** implements it over `num-bigint`
//!   (`tcl_compiler::tcl_expr_eval`);
//! - the **VM** (`tcl_vm::expr`) is the same `num-bigint` shape and is the
//!   next planned adopter.
//!
//! Because every operation here takes and returns backend values (never a
//! second bignum representation), swapping libtommath for a pure-Rust
//! equivalent — or back — is a per-adopter choice with no semantic drift:
//! the semantics are these functions, verified against the tclsh oracle
//! corpus (`docs/design/compiler/type-tracking.md`) and C Tcl 9's
//! `tclExecute.c`.
//!
//! ## What lives here vs the adopter
//!
//! Here: everything value-shaped — promotion, floor rules, `**` collapses,
//! shift-count edge cases, two's-complement bitwise identities. In the
//! adopter: value wrapping (demote-when-fits via [`BigIntOps::to_i64`]),
//! error surfaces (divide-by-zero message text), and float contamination
//! (any double operand routes the whole operation to `f64` *before* these
//! functions are reached, mirroring `Tcl_GetNumberFromObj`).

/// Maximum exponent for integer `**` — C Tcl's `INST_EXPON` limit
/// (`exponent < 2^28`); larger exponents raise "exponent too large" at
/// runtime, so no adopter should compute them.
pub const MAX_EXPONENT: i64 = (1 << 28) - 1;

/// The big-integer backend: the operations the tower semantics need, no
/// more. Implementations must be exact (arbitrary precision) and use
/// two's-complement semantics for the bitwise trio on negative values —
/// both libtommath (`mp_and`/`mp_or`/`mp_xor` under `TCL_BIGNUM_TYPE`'s
/// sign handling) and `num-bigint` already do.
pub trait BigIntOps: Sized + Clone + Ord {
    /// Construct from a wide.
    #[must_use]
    fn from_i64(v: i64) -> Self;
    /// Narrow to a wide when the value fits — the demote-when-fits step
    /// (`$big - $big` is `Int(0)`, never a one-word bignum).
    fn to_i64(&self) -> Option<i64>;
    /// Tcl's low-64-bit `int`/`wide` conversion.
    fn to_i64_wrapping(&self) -> i64;
    /// Whether the value is zero.
    fn is_zero(&self) -> bool;
    /// Whether the value is negative.
    fn is_negative(&self) -> bool;
    /// Exact sum.
    #[must_use]
    fn add(&self, other: &Self) -> Self;
    /// Exact difference.
    #[must_use]
    fn sub(&self, other: &Self) -> Self;
    /// Exact product.
    #[must_use]
    fn mul(&self, other: &Self) -> Self;
    /// **Floor** quotient (rounds toward −∞ — Tcl's `/`, not truncation).
    /// Callers guarantee a non-zero divisor.
    #[must_use]
    fn div_floor(&self, other: &Self) -> Self;
    /// **Floor** remainder (sign follows the divisor — Tcl's `%`).
    /// Callers guarantee a non-zero divisor.
    #[must_use]
    fn mod_floor(&self, other: &Self) -> Self;
    /// Exact negation.
    #[must_use]
    fn neg(&self) -> Self;
    /// Exact power by a small non-negative exponent.
    #[must_use]
    fn pow_u32(&self, exp: u32) -> Self;
    /// Left shift by a small count.
    #[must_use]
    fn shl(&self, count: u32) -> Self;
    /// Arithmetic right shift by a count (sign-extending).
    #[must_use]
    fn shr(&self, count: usize) -> Self;
    /// Two's-complement AND.
    #[must_use]
    fn bitand(&self, other: &Self) -> Self;
    /// Two's-complement OR.
    #[must_use]
    fn bitor(&self, other: &Self) -> Self;
    /// Two's-complement XOR.
    #[must_use]
    fn bitxor(&self, other: &Self) -> Self;
    /// Number of bits in the magnitude (for shift-count collapse).
    fn bit_len(&self) -> u64;
    /// Correctly rounded conversion to an IEEE-754 double.
    fn to_f64(&self) -> f64;
    /// Exact magnitude.
    #[must_use]
    fn abs(&self) -> Self {
        if self.is_negative() {
            self.neg()
        } else {
            self.clone()
        }
    }
}

/// A generic floor square root for backends that provide only exact integer
/// arithmetic. The initial upper bound is `2^ceil(bit_len/2)`; binary search
/// then avoids importing a backend-specific root implementation into the
/// shared semantics.
#[must_use]
pub fn int_sqrt<B: BigIntOps>(value: &B) -> Option<B> {
    if value.is_negative() {
        return None;
    }
    if value.is_zero() {
        return Some(B::from_i64(0));
    }
    let mut lo = B::from_i64(0);
    let mut hi = B::from_i64(1).shl(u32::try_from(value.bit_len().div_ceil(2)).ok()?);
    while lo < hi {
        let mid = lo.add(&hi).add(&B::from_i64(1)).shr(1);
        if mid.mul(&mid) <= *value {
            lo = mid;
        } else {
            hi = mid.sub(&B::from_i64(1));
        }
    }
    Some(lo)
}

/// Integer `**` on the tower — C's `INST_EXPON` rules, exactly:
///
/// - `y == 0` → `1`; `y == 1` → the base.
/// - base `0`: negative exponent is the "exponentiation of zero by negative
///   power" domain error (`None`); otherwise `0`.
/// - base `1` → `1`; base `-1` → `±1` by exponent parity — these collapse
///   for *any* exponent magnitude.
/// - any other base with a negative exponent → `0` (integer power rounds
///   to zero).
/// - exponents past [`MAX_EXPONENT`] are the "exponent too large" runtime
///   error (`None`) — no adopter should compute them.
#[must_use]
pub fn int_pow<B: BigIntOps>(base: &B, exponent: i64) -> Option<B> {
    let one = B::from_i64(1);
    let minus_one = B::from_i64(-1);
    if exponent == 0 {
        return Some(one);
    }
    if exponent == 1 {
        return Some(base.clone());
    }
    if base.is_zero() {
        return if exponent < 0 {
            None
        } else {
            Some(B::from_i64(0))
        };
    }
    if *base == one {
        return Some(one);
    }
    if *base == minus_one {
        return Some(if exponent % 2 == 0 { one } else { minus_one });
    }
    if exponent < 0 {
        return Some(B::from_i64(0));
    }
    if exponent > MAX_EXPONENT {
        return None;
    }
    let exp = u32::try_from(exponent).ok()?;
    Some(base.pow_u32(exp))
}

/// Arithmetic right shift with the width collapse: a count at or past the
/// operand's bit length yields the replicated sign (`0` / `-1`), matching
/// C's wide and bignum paths. A negative count is the caller's "negative
/// shift argument" error (`None`).
#[must_use]
pub fn int_shr<B: BigIntOps>(value: &B, count: i64) -> Option<B> {
    if count < 0 {
        return None;
    }
    match usize::try_from(count) {
        Ok(c) if u64::try_from(c).is_ok_and(|c64| c64 <= value.bit_len()) => Some(value.shr(c)),
        _ => Some(B::from_i64(if value.is_negative() { -1 } else { 0 })),
    }
}

/// Floor division on the tower. `None` on a zero divisor (the adopter's
/// "divide by zero" error surface).
#[must_use]
pub fn int_div<B: BigIntOps>(a: &B, b: &B) -> Option<B> {
    if b.is_zero() {
        return None;
    }
    Some(a.div_floor(b))
}

/// Floor modulus on the tower (sign follows the divisor). `None` on a zero
/// divisor.
#[must_use]
pub fn int_mod<B: BigIntOps>(a: &B, b: &B) -> Option<B> {
    if b.is_zero() {
        return None;
    }
    Some(a.mod_floor(b))
}

/// The backend conformance suite: every semantic row of the tclsh oracle
/// corpus, generic over the backend. Each adopter's test suite calls
/// [`conformance::assert_backend`] with its own [`BigIntOps`] type — the
/// crate's unit tests run it over an `i128` toy backend and (feature
/// `num-bigint`) `BigInt`; the faithful runtime runs it over the real
/// libtommath `mp_int`. A backend that passes cannot drift from the others
/// on any covered semantic.
pub mod conformance {
    use super::{BigIntOps, MAX_EXPONENT, int_div, int_mod, int_pow, int_shr};

    /// Panics (with the failing row) unless `B` reproduces the full oracle
    /// corpus. `wide` must build values well beyond `i64` (e.g. `2^64`) —
    /// backends narrower than ~130 bits cannot conform.
    #[expect(
        clippy::missing_panics_doc,
        reason = "panicking on a failed row is this function's contract"
    )]
    pub fn assert_backend<B: BigIntOps + core::fmt::Debug>() {
        let v = B::from_i64;

        // Round-trip and demote-when-fits.
        assert_eq!(v(i64::MAX).to_i64(), Some(i64::MAX));
        assert_eq!(v(i64::MIN).to_i64(), Some(i64::MIN));
        let two64 = v(2).pow_u32(64); // 18446744073709551616
        assert_eq!(two64.to_i64(), None, "2^64 must not narrow");
        assert!(!two64.is_negative());
        assert!(!two64.is_zero());
        assert!(v(0).is_zero());
        assert!(v(-1).is_negative());

        // Exact ring ops at the promotion boundary (i64::MAX + 1, -MAX - 2).
        let big = v(i64::MAX).add(&v(1));
        assert_eq!(big.to_i64(), None);
        assert_eq!(big.sub(&v(1)).to_i64(), Some(i64::MAX), "demote on re-fit");
        assert_eq!(
            v(i64::MIN).add(&v(-1)).add(&v(1)).to_i64(),
            Some(i64::MIN),
            "-2^63 - 1 + 1 re-fits"
        );
        assert_eq!(v(3).mul(&v(-4)).to_i64(), Some(-12));
        assert_eq!(big.neg().add(&big).to_i64(), Some(0), "x + (-x) = 0");

        // Floor division and modulus (sign follows the divisor), including
        // the beyond-wide oracle rows 2^64 % 7 = 2 and 2^64 / -3.
        assert_eq!(int_div(&v(7), &v(2)).unwrap().to_i64(), Some(3));
        assert_eq!(int_div(&v(-7), &v(2)).unwrap().to_i64(), Some(-4));
        assert_eq!(int_mod(&v(-7), &v(2)).unwrap().to_i64(), Some(1));
        assert_eq!(int_mod(&v(7), &v(-2)).unwrap().to_i64(), Some(-1));
        assert!(int_div(&v(1), &v(0)).is_none(), "divide by zero");
        assert!(int_mod(&v(1), &v(0)).is_none());
        assert_eq!(int_mod(&two64, &v(7)).unwrap().to_i64(), Some(2));
        assert_eq!(
            int_div(&two64, &v(-3)).unwrap().to_i64(),
            Some(-6_148_914_691_236_517_206),
            "2^64 / -3 floors toward -inf"
        );

        // `**` collapses and guards (`INST_EXPON`).
        assert_eq!(int_pow(&v(2), 10).unwrap().to_i64(), Some(1024));
        assert_eq!(int_pow(&v(5), 0).unwrap().to_i64(), Some(1));
        assert!(int_pow(&v(0), -1).is_none(), "zero by negative power");
        assert_eq!(int_pow(&v(1), i64::MAX).unwrap().to_i64(), Some(1));
        assert_eq!(int_pow(&v(-1), 5).unwrap().to_i64(), Some(-1));
        assert_eq!(int_pow(&v(-1), 6).unwrap().to_i64(), Some(1));
        assert_eq!(int_pow(&v(7), -3).unwrap().to_i64(), Some(0));
        assert!(
            int_pow(&v(2), MAX_EXPONENT + 1).is_none(),
            "exponent too large"
        );
        assert_eq!(int_pow(&v(2), 64).unwrap(), two64, "2**64 exact");

        // Shifts: exactness and the sign-collapse past the width.
        assert_eq!(v(1).shl(63).to_i64(), None, "1<<63 exceeds i64");
        assert_eq!(
            v(1).shl(63).neg().to_i64(),
            Some(i64::MIN),
            "-(1<<63) is i64::MIN"
        );
        assert_eq!(int_shr(&v(-8), 1).unwrap().to_i64(), Some(-4));
        assert_eq!(
            int_shr(&v(-8), 1000).unwrap().to_i64(),
            Some(-1),
            "sign collapse"
        );
        assert_eq!(int_shr(&v(8), 1000).unwrap().to_i64(), Some(0));
        assert!(int_shr(&v(8), -1).is_none(), "negative count");
        assert_eq!(int_shr(&two64, 64).unwrap().to_i64(), Some(1), "2^64 >> 64");
        assert_eq!(
            int_shr(&two64.neg(), 200).unwrap().to_i64(),
            Some(-1),
            "-2^64 >> 200 collapses to -1"
        );

        // Two's-complement bitwise trio on negatives (the oracle rows
        // -5 & 3 = 3, -5 | 3 = -5, -5 ^ 3 = -8) and the beyond-wide row
        // 2^64 & 0xFF = 0.
        assert_eq!(v(-5).bitand(&v(3)).to_i64(), Some(3));
        assert_eq!(v(-5).bitor(&v(3)).to_i64(), Some(-5));
        assert_eq!(v(-5).bitxor(&v(3)).to_i64(), Some(-8));
        assert_eq!(two64.bitand(&v(0xFF)).to_i64(), Some(0));

        // bit_len feeds the shift collapse: 2^64 needs 65 bits.
        assert_eq!(two64.bit_len(), 65);
        assert_eq!(v(0).bit_len(), 0);

        // Total order (drives numeric comparisons).
        assert!(v(-1) < v(0) && v(0) < v(1));
        assert!(two64 > v(i64::MAX), "2^64 > i64::MAX");
        assert!(two64.neg() < v(i64::MIN), "-2^64 < i64::MIN");
    }
}

/// The `num-bigint` backend adapter (feature `num-bigint`) — shared by the
/// pure-Rust adopters (the compiler's const-folder today, the VM next). The
/// faithful runtime implements [`BigIntOps`] over the real libtommath
/// `mp_int` instead; both backends run these identical semantics.
#[cfg(feature = "num-bigint")]
impl BigIntOps for num_bigint::BigInt {
    fn from_i64(v: i64) -> Self {
        num_bigint::BigInt::from(v)
    }
    fn to_i64(&self) -> Option<i64> {
        num_traits::ToPrimitive::to_i64(self)
    }
    fn to_i64_wrapping(&self) -> i64 {
        let low = self & ((num_bigint::BigInt::from(1u8) << 64) - 1u8);
        num_traits::ToPrimitive::to_u64(&low)
            .unwrap_or(0)
            .cast_signed()
    }
    fn is_zero(&self) -> bool {
        num_traits::Zero::is_zero(self)
    }
    fn is_negative(&self) -> bool {
        num_traits::Signed::is_negative(self)
    }
    fn add(&self, other: &Self) -> Self {
        self + other
    }
    fn sub(&self, other: &Self) -> Self {
        self - other
    }
    fn mul(&self, other: &Self) -> Self {
        self * other
    }
    fn div_floor(&self, other: &Self) -> Self {
        num_integer::Integer::div_floor(self, other)
    }
    fn mod_floor(&self, other: &Self) -> Self {
        num_integer::Integer::mod_floor(self, other)
    }
    fn neg(&self) -> Self {
        -self
    }
    fn pow_u32(&self, exp: u32) -> Self {
        self.pow(exp)
    }
    fn shl(&self, count: u32) -> Self {
        self << count
    }
    fn shr(&self, count: usize) -> Self {
        self >> count
    }
    fn bitand(&self, other: &Self) -> Self {
        self & other
    }
    fn bitor(&self, other: &Self) -> Self {
        self | other
    }
    fn bitxor(&self, other: &Self) -> Self {
        self ^ other
    }
    fn bit_len(&self) -> u64 {
        self.bits()
    }
    fn to_f64(&self) -> f64 {
        num_traits::ToPrimitive::to_f64(self).unwrap_or_else(|| {
            if self.is_negative() {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny exact backend over `i128` — wide enough for every semantic
    /// case the tests exercise, proving the semantics are backend-agnostic.
    #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
    struct I128(i128);

    impl BigIntOps for I128 {
        fn from_i64(v: i64) -> Self {
            I128(i128::from(v))
        }
        fn to_i64(&self) -> Option<i64> {
            i64::try_from(self.0).ok()
        }
        fn to_i64_wrapping(&self) -> i64 {
            let low = self.0 & i128::from(u64::MAX);
            u64::try_from(low).unwrap().cast_signed()
        }
        fn is_zero(&self) -> bool {
            self.0 == 0
        }
        fn is_negative(&self) -> bool {
            self.0 < 0
        }
        fn add(&self, other: &Self) -> Self {
            I128(self.0 + other.0)
        }
        fn sub(&self, other: &Self) -> Self {
            I128(self.0 - other.0)
        }
        fn mul(&self, other: &Self) -> Self {
            I128(self.0 * other.0)
        }
        fn div_floor(&self, other: &Self) -> Self {
            I128(
                self.0.div_euclid(other.0)
                    - i128::from(other.0 < 0 && self.0.rem_euclid(other.0) != 0),
            )
        }
        fn mod_floor(&self, other: &Self) -> Self {
            let r = self.0.rem_euclid(other.0.abs());
            I128(if other.0 < 0 && r != 0 {
                r - other.0.abs()
            } else {
                r
            })
        }
        fn neg(&self) -> Self {
            I128(-self.0)
        }
        fn pow_u32(&self, exp: u32) -> Self {
            I128(self.0.pow(exp))
        }
        fn shl(&self, count: u32) -> Self {
            I128(self.0 << count)
        }
        fn shr(&self, count: usize) -> Self {
            I128(self.0 >> count)
        }
        fn bitand(&self, other: &Self) -> Self {
            I128(self.0 & other.0)
        }
        fn bitor(&self, other: &Self) -> Self {
            I128(self.0 | other.0)
        }
        fn bitxor(&self, other: &Self) -> Self {
            I128(self.0 ^ other.0)
        }
        fn bit_len(&self) -> u64 {
            u64::from(128 - self.0.unsigned_abs().leading_zeros())
        }
        fn to_f64(&self) -> f64 {
            num_traits::ToPrimitive::to_f64(&self.0).unwrap()
        }
    }

    /// The toy backend itself passes the full conformance corpus — proving
    /// the suite (and the semantics) are backend-agnostic.
    #[test]
    fn i128_backend_conforms() {
        conformance::assert_backend::<I128>();
    }

    /// The `num-bigint` adapter passes the full conformance corpus.
    #[cfg(feature = "num-bigint")]
    #[test]
    fn num_bigint_backend_conforms() {
        conformance::assert_backend::<num_bigint::BigInt>();
    }

    /// Floor semantics, oracle values (`-7/2 → -4`, `-7%2 → 1`).
    #[test]
    fn floor_div_mod_oracle() {
        let v = |n: i64| I128::from_i64(n);
        assert_eq!(int_div(&v(7), &v(2)), Some(v(3)));
        assert_eq!(int_div(&v(-7), &v(2)), Some(v(-4)));
        assert_eq!(int_mod(&v(-7), &v(2)), Some(v(1)));
        assert_eq!(int_mod(&v(7), &v(-2)), Some(v(-1)));
        assert_eq!(int_div(&v(1), &v(0)), None);
        assert_eq!(int_mod(&v(1), &v(0)), None);
    }

    /// `**` collapses and guards (`INST_EXPON`).
    #[test]
    fn pow_rules() {
        let v = |n: i64| I128::from_i64(n);
        assert_eq!(int_pow(&v(2), 10), Some(v(1024)));
        assert_eq!(int_pow(&v(5), 0), Some(v(1)));
        assert_eq!(int_pow(&v(5), 1), Some(v(5)));
        assert_eq!(int_pow(&v(0), -1), None, "zero by negative power");
        assert_eq!(int_pow(&v(0), 3), Some(v(0)));
        assert_eq!(int_pow(&v(1), i64::MAX), Some(v(1)));
        assert_eq!(int_pow(&v(-1), 5), Some(v(-1)));
        assert_eq!(int_pow(&v(-1), 6), Some(v(1)));
        assert_eq!(int_pow(&v(7), -3), Some(v(0)), "integer power rounds to 0");
        assert_eq!(int_pow(&v(2), MAX_EXPONENT + 1), None, "exponent too large");
    }

    /// Shift sign-collapse past the operand width.
    #[test]
    fn shr_collapse() {
        let v = |n: i64| I128::from_i64(n);
        assert_eq!(int_shr(&v(-8), 1), Some(v(-4)));
        assert_eq!(int_shr(&v(-8), 1000), Some(v(-1)));
        assert_eq!(int_shr(&v(8), 1000), Some(v(0)));
        assert_eq!(int_shr(&v(8), -1), None, "negative count is an error");
    }
}
