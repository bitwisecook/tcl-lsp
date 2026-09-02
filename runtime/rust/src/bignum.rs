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

//! The bignum rung of the numeric tower: the `TCL_BIGNUM_TYPE` obj rep over
//! libtommath `mp_int`, the representation chosen + validated in EXP-BIGNUM.
//!
//! `mp_int` **is** our bignum — the same representation C extensions get via
//! `Tcl_GetBignumFromObj` (we ship `tclTomMath.h` + export the `TclBN_*` stubs,
//! Track 2/3), so there is no second bignum and no boundary conversion. The
//! obj's 8-byte `internal_rep` holds a heap pointer to the `mp_int`; on wasm32
//! this can later pack inline (`dp` + packed header in the two i32 words, exactly
//! C Tcl's scheme) — deferred as a non-observable optimisation.
//!
//! Built only when `build.rs` links libtommath (the `have_tommath` cfg); the
//! C-extension boundary (`Tcl_GetBignumFromObj` + the TomMath stubs table) lands
//! with the C-API track.
//!
//! This module is the one place raw `mp_*` FFI is reviewed.

#![allow(clippy::not_unsafe_ptr_arg_deref)] // the obj-procs take `*mut TclObj` by the C ABI

use core::ffi::{c_char, c_int};

use crate::obj::{self, TclObj, TclObjType};
use tcl_syntax::expr::NumericCompare;
use tcl_syntax::number::Radix;

/// libtommath's `mp_int` (pristine, `MP_64BIT`): `{ int used, alloc; mp_sign
/// sign; mp_digit *dp; }` — see `tommath.h:257`. `mp_sign` and `mp_err` are
/// `int`-sized; `mp_digit` is `uint64_t` under `MP_64BIT` (the digit *array* is
/// heap, so this struct is 16 B on wasm32 / 24 B on 64-bit native either way).
#[repr(C)]
struct MpInt {
    used: c_int,
    alloc: c_int,
    sign: c_int,
    dp: *mut u64,
}

const MP_OKAY: c_int = 0;

// SAFETY: thin declarations of the pristine libtommath C API that `build.rs`
// compiles + links (`-DTCL_WITH_EXTERNAL_TOMMATH -DLTM_ALL -DMP_64BIT`).
extern "C" {
    fn mp_init(a: *mut MpInt) -> c_int;
    fn mp_clear(a: *mut MpInt);
    fn mp_init_copy(a: *mut MpInt, b: *const MpInt) -> c_int;
    fn mp_read_radix(a: *mut MpInt, s: *const c_char, radix: c_int) -> c_int;
    fn mp_to_radix(
        a: *const MpInt,
        s: *mut c_char,
        maxlen: usize,
        written: *mut usize,
        radix: c_int,
    ) -> c_int;
    fn mp_radix_size(a: *const MpInt, radix: c_int, size: *mut c_int) -> c_int;
    fn mp_count_bits(a: *const MpInt) -> c_int;
    fn mp_get_i64(a: *const MpInt) -> i64;
    fn mp_init_i64(a: *mut MpInt, b: i64) -> c_int;
    fn mp_add(a: *const MpInt, b: *const MpInt, c: *mut MpInt) -> c_int;
    fn mp_sub(a: *const MpInt, b: *const MpInt, c: *mut MpInt) -> c_int;
    fn mp_mul(a: *const MpInt, b: *const MpInt, c: *mut MpInt) -> c_int;
    fn mp_neg(a: *const MpInt, b: *mut MpInt) -> c_int;
    fn mp_get_double(a: *const MpInt) -> f64;
    fn mp_div(a: *const MpInt, b: *const MpInt, c: *mut MpInt, d: *mut MpInt) -> c_int;
    fn mp_sub_d(a: *const MpInt, b: u64, c: *mut MpInt) -> c_int; // b: mp_digit (MP_64BIT)
    fn mp_cmp(a: *const MpInt, b: *const MpInt) -> c_int; // mp_ord: -1/0/1
    fn mp_expt_n(a: *const MpInt, b: c_int, c: *mut MpInt) -> c_int;
    fn mp_and(a: *const MpInt, b: *const MpInt, c: *mut MpInt) -> c_int;
    fn mp_or(a: *const MpInt, b: *const MpInt, c: *mut MpInt) -> c_int;
    fn mp_xor(a: *const MpInt, b: *const MpInt, c: *mut MpInt) -> c_int;
    fn mp_complement(a: *const MpInt, b: *mut MpInt) -> c_int;
    fn mp_mul_2d(a: *const MpInt, b: c_int, c: *mut MpInt) -> c_int;
    fn mp_signed_rsh(a: *const MpInt, b: c_int, c: *mut MpInt) -> c_int;
}

// ---------------------------------------------------------------------------
// Tower arithmetic — the integer rung (wide → bignum, with demote-when-fits)
// plus double promotion. Follows `tclExecute.c`'s overflow-checked wide fast
// path → `ExecuteExtendedBinaryMathOp` bignum path → canonical demote. Operands
// are `TclObj`s; results are fresh (`rc 0`) `TclObj`s (int / bignum / double).
// Covers +/-/*/neg, floor `/`/`%` (sign-of-divisor), comparison, `**` (TIP 123),
// the bitwise ops `& | ^ ~`, and shifts `<< >>`. The `expr` walker builds on this.
// ---------------------------------------------------------------------------

/// An RAII libtommath integer: owns its `mp_int`, clearing it on drop.
struct Mp(MpInt);

impl Mp {
    /// A fresh `mp_int` initialised to 0.
    fn zero() -> Option<Mp> {
        let mut m = zeroed_mp();
        // SAFETY: `mp_init` initialises `m`'s fields + allocates its digit array.
        (unsafe { mp_init(&mut m) } == MP_OKAY).then_some(Mp(m))
    }

    /// An `mp_int` holding the wide `v`.
    fn from_i64(v: i64) -> Option<Mp> {
        let mut m = zeroed_mp();
        // SAFETY: initialise `m` from the 64-bit value.
        (unsafe { mp_init_i64(&mut m, v) } == MP_OKAY).then_some(Mp(m))
    }

    /// A deep copy of the live `mp_int` at `src`.
    fn copy_of(src: *const MpInt) -> Option<Mp> {
        let mut m = zeroed_mp();
        // SAFETY: `src` is a live mp_int; `mp_init_copy` deep-copies it.
        (unsafe { mp_init_copy(&mut m, src) } == MP_OKAY).then_some(Mp(m))
    }

    #[inline]
    fn ptr(&self) -> *const MpInt {
        &self.0
    }

    /// Move the inner `mp_int` out without clearing (the caller takes ownership).
    fn into_inner(self) -> MpInt {
        let m = core::mem::ManuallyDrop::new(self);
        // SAFETY: `m` is not dropped (ManuallyDrop), so the mp_int is not cleared
        // here; ownership moves to the returned value.
        unsafe { core::ptr::read(&m.0) }
    }
}

impl Drop for Mp {
    fn drop(&mut self) {
        // SAFETY: `self.0` is a live, owned mp_int.
        unsafe { mp_clear(&mut self.0) }
    }
}

/// An operand read off a `TclObj` for arithmetic — one tower rung.
enum NumVal {
    /// A wide integer.
    Wide(i64),
    /// A bignum (owned).
    Big(Mp),
    /// A floating-point value.
    Float(f64),
}

/// Read a numeric operand from an object: its typed rep when it has one, else
/// parse its string via the shared [`tcl_syntax::number`] grammar. Returns
/// `None` for a non-numeric string or a NaN operand (the caller raises the
/// "can't use … as operand" error).
fn read(obj: *mut TclObj) -> Option<NumVal> {
    let tp = obj::obj_type_ptr(obj);
    if tp == &obj::TCL_INT_TYPE {
        return Some(NumVal::Wide(obj::wide_of(obj)));
    }
    if tp == &obj::TCL_DOUBLE_TYPE {
        return Some(NumVal::Float(obj::double_of(obj)));
    }
    if tp == &TCL_BIGNUM_TYPE {
        return Some(NumVal::Big(Mp::copy_of(mp_ptr(obj))?));
    }
    // Untyped (or other): classify the string rep, then cache what we parsed
    // back onto the object so the next use reads a rep instead of the spelling.
    let value = parse_string_rep(obj)?;
    cache_parsed_rep(obj, &value);
    Some(value)
}

/// Classify an object's string rep through the shared [`tcl_syntax::number`]
/// grammar, without touching the object's internal rep.
fn parse_string_rep(obj: *mut TclObj) -> Option<NumVal> {
    let bytes = obj::bytes_of(obj);
    let s = core::str::from_utf8(&bytes).ok()?;
    use tcl_syntax::number::Number;
    match tcl_syntax::number::parse_whole(s)? {
        Number::Int(v) => Some(NumVal::Wide(v)),
        Number::Double(d) => Some(NumVal::Float(d)),
        Number::Big {
            negative,
            radix,
            digits,
        } => {
            let mut m = zeroed_mp();
            let mut c = Vec::with_capacity(digits.len() + 2);
            if negative {
                c.push(b'-');
            }
            c.extend_from_slice(digits.as_bytes());
            c.push(0);
            // SAFETY: init then parse the cleaned digits into `m`.
            unsafe {
                if mp_init(&mut m) != MP_OKAY {
                    return None;
                }
                if mp_read_radix(&mut m, c.as_ptr() as *const c_char, radix as c_int) != MP_OKAY {
                    mp_clear(&mut m);
                    return None;
                }
            }
            Some(NumVal::Big(Mp(m)))
        }
        Number::Nan { .. } => None,
    }
}

/// Write the freshly parsed tower value back onto `obj` as its internal rep.
///
/// This is C's numeric write-back (`TclParseNumber` stores the rep it built on
/// the object it parsed), and without it `incr x` / `expr {$x+1}` in a loop
/// re-lex the same spelling on every iteration. The **string** rep is kept by
/// [`obj::change_type`], so the object's spelling is unchanged: `"0x10"` reads
/// as 16 and still prints `0x10`, exactly as in C Tcl.
/// [`obj::may_cache_parsed_rep`] owns the "may we" question.
fn cache_parsed_rep(obj: *mut TclObj, value: &NumVal) {
    match value {
        NumVal::Wide(w) => obj::cache_wide_rep(obj, *w),
        NumVal::Float(f) => obj::cache_double_rep(obj, *f),
        NumVal::Big(m) => {
            if !obj::may_cache_parsed_rep(obj) {
                return;
            }
            // The caller keeps the operand it was handed, so the object gets its
            // own deep copy of the digits.
            let Some(copy) = Mp::copy_of(m.ptr()) else {
                return;
            };
            let boxed = Box::into_raw(Box::new(copy.into_inner()));
            obj::change_type(obj, &TCL_BIGNUM_TYPE, boxed as u64);
        }
    }
}

/// How an object read as a `Tcl_WideInt` (`Tcl_GetWideIntFromObj`) came out.
pub(crate) enum WideRead {
    /// The value, as a wide integer.
    Wide(i64),
    /// An integer past the wide range — C's `integer value too large to
    /// represent` / `ARITH IOVERFLOW`.
    Overflow,
    /// A number that is not an integer (a double, or NaN).
    NotInteger,
    /// Not a number at all.
    NotNumeric,
}

/// Read `obj` as a wide integer with `Tcl_GetWideIntFromObj`'s classification,
/// caching the parsed rep back onto the object like every other tower read.
///
/// A bignum that still fits a wide narrows here, exactly as C's bignum branch
/// does even with auto-narrowing enabled.
pub(crate) fn read_wide(obj: *mut TclObj) -> WideRead {
    match read(obj) {
        Some(NumVal::Wide(w)) => WideRead::Wide(w),
        Some(NumVal::Big(m)) => {
            // SAFETY: `m` owns a live mp_int.
            if unsafe { mp_count_bits(m.ptr()) } <= 63 {
                // SAFETY: the magnitude fits, so the signed read is exact.
                WideRead::Wide(unsafe { mp_get_i64(m.ptr()) })
            } else {
                WideRead::Overflow
            }
        }
        Some(NumVal::Float(_)) => WideRead::NotInteger,
        None => {
            // A NaN spelling is a number the tower declines, not a non-number:
            // `Tcl_GetWideIntFromObj` still reports it as "expected integer".
            if is_nan_operand(obj) {
                WideRead::NotInteger
            } else {
                WideRead::NotNumeric
            }
        }
    }
}

/// Read `obj` as a double with `Tcl_GetDoubleFromObj`'s widening (an integer or
/// bignum promotes), caching the parsed rep back onto the object. `None` is a
/// non-numeric value; a NaN spelling yields `Some(NaN)`, which the boolean
/// context rejects and the double context accepts.
pub(crate) fn read_double(obj: *mut TclObj) -> Option<f64> {
    match read(obj) {
        Some(NumVal::Wide(w)) => Some(w as f64),
        Some(NumVal::Float(f)) => Some(f),
        // SAFETY: `m` owns a live mp_int; C promotes a bignum to the nearest
        // double (±Inf past the range).
        Some(NumVal::Big(m)) => Some(unsafe { mp_get_double(m.ptr()) }),
        None => is_nan_operand(obj).then_some(f64::NAN),
    }
}

/// Materialise a tower value as a fresh object, demoting a bignum that fits.
fn to_obj(v: NumVal) -> *mut TclObj {
    match v {
        NumVal::Wide(w) => obj::new_wide_int_obj(w),
        NumVal::Float(f) => obj::new_double_obj(f),
        NumVal::Big(mp) => store(mp.into_inner()),
    }
}

/// The integer binary operators routed through the bignum path.
#[derive(Clone, Copy)]
enum IntOp {
    Add,
    Sub,
    Mul,
}

/// Apply an integer op to two bignums (promoting the wide fast path's overflow).
fn int_big(op: IntOp, a: *const MpInt, b: *const MpInt) -> Option<NumVal> {
    let mut out = Mp::zero()?;
    let f = match op {
        IntOp::Add => mp_add,
        IntOp::Sub => mp_sub,
        IntOp::Mul => mp_mul,
    };
    // SAFETY: `a`/`b` are live mp_ints; `out` is freshly initialised.
    (unsafe { f(a, b, &mut out.0) } == MP_OKAY).then_some(NumVal::Big(out))
}

/// Why a tower op could not produce a value — mapped to Tcl's verbatim error
/// strings by the `expr`/`mathop` layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithError {
    /// An operand was not a number (`can't use non-numeric string … as operand`).
    NonNumeric,
    /// An integer-only op (bit-op, `<<`, `>>`) got a float operand
    /// (`can't use floating-point value as operand of …`).
    NonInteger,
    /// Integer `/` or `%` by zero (`divide by zero`, `-errorcode ARITH
    /// DIVZERO`). **Not** `0 ** negative` — C classes that as a domain
    /// error, [`ArithError::ZeroToNegativePower`], with its own message and
    /// `-errorcode` (`tclExecute.c`'s `EXPON_OF_ZERO`).
    DivideByZero,
    /// `0 ** negative` on either the integer or the float tier
    /// (`exponentiation of zero by negative power`, `-errorcode ARITH
    /// DOMAIN`) — C's `EXPON_OF_ZERO`, a domain error rather than a division
    /// by zero.
    ZeroToNegativePower,
    /// A negative shift count (`negative shift argument`).
    NegativeShift,
    /// An exponent too large to compute (`exponent too large`).
    ExponentTooLarge,
    /// A left-shift count too large to compute (`integer value too large to
    /// represent`) — C raises this, not "exponent too large", for `<<`.
    TooLargeToRepresent,
    /// A bignum allocation failed (out of memory).
    Alloc,
}

/// Read an operand as a number, mapping a non-numeric/parse failure to the
/// `NonNumeric` error.
fn num(obj: *mut TclObj) -> Result<NumVal, ArithError> {
    read(obj).ok_or(ArithError::NonNumeric)
}

/// The shared dispatch for `+`/`-`/`*`: wide fast path with overflow→bignum, the
/// bignum path when either operand is big, and double promotion when either is a
/// float.
fn arith(op: IntOp, a: *mut TclObj, b: *mut TclObj) -> Result<*mut TclObj, ArithError> {
    let x = num(a)?;
    let y = num(b)?;
    let v = match (x, y) {
        // Float promotion: mixed or both float → double result.
        (NumVal::Float(p), q) => NumVal::Float(float_op(op, p, as_f64(q))),
        (p, NumVal::Float(q)) => NumVal::Float(float_op(op, as_f64(p), q)),
        // Wide fast path: checked op, overflow → bignum.
        (NumVal::Wide(p), NumVal::Wide(q)) => match wide_op(op, p, q) {
            Some(w) => NumVal::Wide(w),
            None => {
                let pm = Mp::from_i64(p).ok_or(ArithError::Alloc)?;
                let qm = Mp::from_i64(q).ok_or(ArithError::Alloc)?;
                int_big(op, pm.ptr(), qm.ptr()).ok_or(ArithError::Alloc)?
            }
        },
        // At least one bignum → bignum path.
        (p, q) => {
            let pm = into_mp(p).ok_or(ArithError::Alloc)?;
            let qm = into_mp(q).ok_or(ArithError::Alloc)?;
            int_big(op, pm.ptr(), qm.ptr()).ok_or(ArithError::Alloc)?
        }
    };
    Ok(to_obj(v))
}

#[inline]
fn wide_op(op: IntOp, a: i64, b: i64) -> Option<i64> {
    match op {
        IntOp::Add => a.checked_add(b),
        IntOp::Sub => a.checked_sub(b),
        IntOp::Mul => a.checked_mul(b),
    }
}

#[inline]
fn float_op(op: IntOp, a: f64, b: f64) -> f64 {
    match op {
        IntOp::Add => a + b,
        IntOp::Sub => a - b,
        IntOp::Mul => a * b,
    }
}

#[inline]
fn as_f64(v: NumVal) -> f64 {
    match v {
        NumVal::Wide(w) => w as f64,
        NumVal::Float(f) => f,
        // SAFETY: `v` is a live bignum; libtommath's double conversion.
        NumVal::Big(mp) => unsafe { mp_get_double(mp.ptr()) },
    }
}

/// Promote a wide/bignum operand to an owned `mp_int` (errors on a float).
fn into_mp(v: NumVal) -> Option<Mp> {
    match v {
        NumVal::Wide(w) => Mp::from_i64(w),
        NumVal::Big(mp) => Some(mp),
        NumVal::Float(_) => None,
    }
}

/// `a + b` over the tower.
pub fn add(a: *mut TclObj, b: *mut TclObj) -> Result<*mut TclObj, ArithError> {
    arith(IntOp::Add, a, b)
}

/// `a - b` over the tower.
pub fn sub(a: *mut TclObj, b: *mut TclObj) -> Result<*mut TclObj, ArithError> {
    arith(IntOp::Sub, a, b)
}

/// `a * b` over the tower.
pub fn mul(a: *mut TclObj, b: *mut TclObj) -> Result<*mut TclObj, ArithError> {
    arith(IntOp::Mul, a, b)
}

/// `-a` over the tower (wide negation promotes only at `i64::MIN`).
pub fn neg(a: *mut TclObj) -> Result<*mut TclObj, ArithError> {
    let v = match num(a)? {
        NumVal::Wide(w) => match w.checked_neg() {
            Some(n) => NumVal::Wide(n),
            None => NumVal::Big(mp_negate(&Mp::from_i64(w).ok_or(ArithError::Alloc)?)?),
        },
        NumVal::Float(f) => NumVal::Float(-f),
        NumVal::Big(mp) => NumVal::Big(mp_negate(&mp)?),
    };
    Ok(to_obj(v))
}

/// `-x` as a fresh bignum.
fn mp_negate(x: &Mp) -> Result<Mp, ArithError> {
    let mut out = Mp::zero().ok_or(ArithError::Alloc)?;
    // SAFETY: `x` and `out` are live mp_ints.
    if unsafe { mp_neg(x.ptr(), &mut out.0) } != MP_OKAY {
        return Err(ArithError::Alloc);
    }
    Ok(out)
}

/// Integer floor `a / b` (quotient toward −∞, sign of divisor) over the tower —
/// the `tclExecute.c` rule. A float operand makes it float division. Errors
/// `DivideByZero` on a zero integer divisor.
pub fn div(a: *mut TclObj, b: *mut TclObj) -> Result<*mut TclObj, ArithError> {
    divmod(a, b, true)
}

/// Integer floor `a % b` (remainder takes the sign of the divisor) over the
/// tower. Errors `DivideByZero` on a zero divisor.
pub fn mod_(a: *mut TclObj, b: *mut TclObj) -> Result<*mut TclObj, ArithError> {
    divmod(a, b, false)
}

fn divmod(a: *mut TclObj, b: *mut TclObj, want_quotient: bool) -> Result<*mut TclObj, ArithError> {
    let x = num(a)?;
    let y = num(b)?;
    // `/` divides as floats when either operand is a float; `%` is integer-only,
    // so a float operand is an error (`cannot use floating-point value ...`).
    if matches!(x, NumVal::Float(_)) || matches!(y, NumVal::Float(_)) {
        if !want_quotient {
            return Err(ArithError::NonInteger);
        }
        let (p, q) = (as_f64(x), as_f64(y));
        return Ok(obj::new_double_obj(p / q));
    }
    // The integer tier is the shared tower's (`int_div` / `int_mod` over the
    // libtommath adapter): the floor quotient, the divisor-signed remainder,
    // and the zero-divisor refusal all come from that one owner (#1428).
    let (p, q) = (tower_of(x)?, tower_of(y)?);
    let r = if want_quotient {
        tcl_syntax::number_tower::int_div(&p, &q)
    } else {
        tcl_syntax::number_tower::int_mod(&p, &q)
    };
    Ok(tower_obj(r.ok_or(ArithError::DivideByZero)?))
}

/// Floor division for bignums via `mp_div` (C-truncation) + the floor adjust:
/// when the remainder is non-zero and its sign differs from the divisor's,
/// `q -= 1; r += divisor`.
fn mp_floor_divmod(a: &Mp, b: &Mp) -> Result<(Mp, Mp), ArithError> {
    let mut q = Mp::zero().ok_or(ArithError::Alloc)?;
    let mut r = Mp::zero().ok_or(ArithError::Alloc)?;
    // SAFETY: all live mp_ints.
    unsafe {
        if mp_div(a.ptr(), b.ptr(), &mut q.0, &mut r.0) != MP_OKAY {
            return Err(ArithError::Alloc);
        }
        // `mp_iszero` is `used == 0`; `sign` is 0 (ZPOS) / 1 (NEG). The
        // floor-adjust (`q -= 1; r += b`) only runs when signs differ — the `&&`
        // short-circuits before the mutating calls otherwise.
        let needs_adjust = r.0.used != 0 && r.0.sign != b.0.sign;
        if needs_adjust
            && (mp_sub_d(q.ptr(), 1, &mut q.0) != MP_OKAY
                || mp_add(r.ptr(), b.ptr(), &mut r.0) != MP_OKAY)
        {
            return Err(ArithError::Alloc);
        }
    }
    Ok((q, r))
}

/// Numeric three-way comparison over the tower (for `< > <= >= == !=`). `None`
/// Whether `obj` reads as an integer (a wide or a bignum, not a float or a
/// non-number) — the `incr` / bit-op operand check.
#[must_use]
pub fn is_integer(obj: *mut TclObj) -> bool {
    matches!(read(obj), Some(NumVal::Wide(_) | NumVal::Big(_)))
}

/// Whether `obj` reads as any number (integer, bignum, or float) — used by
/// `expr`'s operand-type error to tell a non-numeric operand from a float one.
#[must_use]
pub fn is_numeric(obj: *mut TclObj) -> bool {
    read(obj).is_some()
}

/// The low 64 bits of an **integer** `obj` as a signed wide — C's `wide()`
/// truncation (a bignum that overflows `i64` wraps, e.g. `wide(2**63)` ⇒
/// `i64::MIN`). The caller guards with [`is_integer`]; a non-integer yields 0.
#[must_use]
pub fn truncate_to_wide(obj: *mut TclObj) -> i64 {
    match read(obj) {
        Some(NumVal::Wide(w)) => w,
        // SAFETY: `mp` is a live mp_int; `mp_get_i64` returns its low 64 bits.
        Some(NumVal::Big(mp)) => unsafe { mp_get_i64(mp.ptr()) },
        _ => 0,
    }
}

/// Read `obj` as a [`mathfunc::Num`](tcl_syntax::expr::mathfunc::Num) for the
/// shared math-function dispatch — `None` if non-numeric. A bignum widens to a
/// double (math functions compute on the double rung, as in C Tcl).
#[must_use]
pub fn as_math_num(obj: *mut TclObj) -> Option<tcl_syntax::expr::mathfunc::NumValue<TowerMp>> {
    use tcl_syntax::expr::mathfunc::NumValue;
    Some(match read(obj)? {
        NumVal::Wide(w) => NumValue::Int(w),
        NumVal::Float(f) => NumValue::Float(f),
        NumVal::Big(mp) => NumValue::Big(TowerMp(mp)),
    })
}

/// Materialise a result from the shared math-function dispatch, demoting a
/// bignum that fits the wide representation exactly as the arithmetic tower
/// does.
pub fn math_num_to_obj(num: tcl_syntax::expr::mathfunc::NumValue<TowerMp>) -> *mut TclObj {
    match num {
        tcl_syntax::expr::mathfunc::NumValue::Int(i) => obj::new_wide_int_obj(i),
        tcl_syntax::expr::mathfunc::NumValue::Float(f) => obj::new_double_obj(f),
        tcl_syntax::expr::mathfunc::NumValue::Big(mp) => to_obj(NumVal::Big(mp.0)),
    }
}

/// on a non-numeric operand (NaN compares as the IEEE result via `f64`).
#[must_use]
pub fn compare(a: *mut TclObj, b: *mut TclObj) -> Option<NumericCompare> {
    use core::cmp::Ordering;
    use NumericCompare::{Ordered, Unordered};
    // NaN is numeric-but-unordered for comparisons (C Tcl: `!=` true, every
    // other comparison false), while [`read`] deliberately folds a NaN
    // *string* to `None` because the arithmetic paths must raise on it — so
    // detect NaN here first. A typed double NaN is caught the same way.
    if is_nan_operand(a) || is_nan_operand(b) {
        return Some(Unordered);
    }
    let x = read(a)?;
    let y = read(b)?;
    Some(match (x, y) {
        (NumVal::Wide(p), NumVal::Wide(q)) => Ordered(p.cmp(&q)),
        // Integer-vs-double compares exactly — a both-as-`f64` comparison
        // merges distinct wides above 2⁵³ (`20000000000000003` vs
        // `20000000000000004.0` — the case C's TclCompareTwoNumbers cites).
        (NumVal::Wide(p), NumVal::Float(d)) => {
            NumericCompare::from_partial(tcl_syntax::number::compare_int_double(i128::from(p), d))
        }
        (NumVal::Float(d), NumVal::Wide(q)) => {
            match tcl_syntax::number::compare_int_double(i128::from(q), d) {
                Some(ord) => Ordered(ord.reverse()),
                None => Unordered,
            }
        }
        (NumVal::Float(p), NumVal::Float(q)) => NumericCompare::from_partial(p.partial_cmp(&q)),
        (NumVal::Big(m), NumVal::Float(d)) => big_vs_double(&m, d),
        (NumVal::Float(d), NumVal::Big(m)) => match big_vs_double(&m, d) {
            Ordered(ord) => Ordered(ord.reverse()),
            unordered => unordered,
        },
        // Both integers, at least one bignum → mp_cmp.
        (p, q) => {
            let pm = into_mp(p)?;
            let qm = into_mp(q)?;
            // SAFETY: live mp_ints; mp_cmp returns -1/0/1.
            Ordered(match unsafe { mp_cmp(pm.ptr(), qm.ptr()) } {
                n if n < 0 => Ordering::Less,
                0 => Ordering::Equal,
                _ => Ordering::Greater,
            })
        }
    })
}

/// Whether the operand is a NaN — a typed double NaN, or an untyped string
/// spelling one (`NaN`, `nan(...)`) per the shared number grammar.
fn is_nan_operand(obj: *mut TclObj) -> bool {
    let tp = obj::obj_type_ptr(obj);
    if tp == &obj::TCL_DOUBLE_TYPE {
        return obj::double_of(obj).is_nan();
    }
    if tp == &obj::TCL_INT_TYPE || tp == &TCL_BIGNUM_TYPE {
        return false;
    }
    let bytes = obj::bytes_of(obj);
    let Ok(s) = core::str::from_utf8(&bytes) else {
        return false;
    };
    matches!(
        tcl_syntax::number::parse_whole(s),
        Some(tcl_syntax::number::Number::Nan { .. })
    )
}

/// Exact bignum-vs-finite-double comparison (NaN was filtered by the caller):
/// split the double into its exact integer part and fraction, compare the
/// integer parts with `mp_cmp`, and let a non-zero fraction break a tie.
fn big_vs_double(m: &Mp, d: f64) -> NumericCompare {
    use core::cmp::Ordering;
    use NumericCompare::Ordered;
    if d == f64::INFINITY {
        return Ordered(Ordering::Less);
    }
    if d == f64::NEG_INFINITY {
        return Ordered(Ordering::Greater);
    }
    const MANTISSA_BITS: u64 = 52;
    const EXPONENT_BIAS_AND_SHIFT: u64 = 1023 + MANTISSA_BITS;
    let bits = d.to_bits();
    let negative = (bits >> 63) == 1;
    let stored_exponent = (bits >> MANTISSA_BITS) & 0x7ff;
    let fraction_bits = bits & ((1 << MANTISSA_BITS) - 1);
    // value = mantissa × 2^(stored − 1075); subnormal/zero ⇒ integer part 0.
    let mantissa = if stored_exponent == 0 {
        fraction_bits
    } else {
        (1 << MANTISSA_BITS) | fraction_bits
    };
    let (int_magnitude, scale_up, has_fraction) =
        match stored_exponent.checked_sub(EXPONENT_BIAS_AND_SHIFT) {
            // Scale up: purely integral, worth `mantissa << left` (left ≤ 971,
            // applied via mp_mul_2d below).
            Some(left) => (mantissa, left, false),
            None => {
                let right = EXPONENT_BIAS_AND_SHIFT - stored_exponent;
                if right >= MANTISSA_BITS + 2 {
                    // |d| < 1 (incl. zero/subnormal): integer part 0.
                    (0, 0, mantissa != 0)
                } else {
                    (mantissa >> right, 0, mantissa & ((1 << right) - 1) != 0)
                }
            }
        };
    // The integer part as an mp: |int| = int_magnitude × 2^scale_up, signed.
    let signed = i64::try_from(int_magnitude).map_or(i64::MAX, |v| if negative { -v } else { v });
    let Some(mut dm) = Mp::from_i64(signed) else {
        // Allocation failure: degrade to the (lossy) float compare rather
        // than panic — vanishingly rare and still totally ordered.
        return NumericCompare::from_partial(
            // SAFETY: live bignum; libtommath's double conversion.
            unsafe { mp_get_double(m.ptr()) }.partial_cmp(&d),
        );
    };
    if scale_up > 0 {
        let mut out = zeroed_mp();
        // SAFETY: init `out`, then shift the live `dm` left into it.
        unsafe {
            if mp_init(&mut out) != MP_OKAY {
                return NumericCompare::from_partial(mp_get_double(m.ptr()).partial_cmp(&d));
            }
            if mp_mul_2d(
                dm.ptr(),
                c_int::try_from(scale_up).unwrap_or(c_int::MAX),
                &mut out,
            ) != MP_OKAY
            {
                mp_clear(&mut out);
                return NumericCompare::from_partial(mp_get_double(m.ptr()).partial_cmp(&d));
            }
        }
        dm = Mp(out);
    }
    // SAFETY: both live mp_ints.
    let cmp = match unsafe { mp_cmp(m.ptr(), dm.ptr()) } {
        n if n < 0 => Ordering::Less,
        0 => Ordering::Equal,
        _ => Ordering::Greater,
    };
    Ordered(match (cmp, has_fraction) {
        // Equal integer parts, fraction on the double: truncation is toward
        // zero, so positive d sits above its integer part, negative below.
        (Ordering::Equal, true) if !negative => Ordering::Less,
        (Ordering::Equal, true) => Ordering::Greater,
        (ord, _) => ord,
    })
}

// ---------------------------------------------------------------------------
// Exponentiation, bitwise ops, and shifts (integer-only except `**` on floats).
// ---------------------------------------------------------------------------

/// An integer operand (rejecting floats) for the bit-ops / shifts.
enum IntVal {
    Wide(i64),
    Big(Mp),
}

/// Read an operand that must be an integer (bit-ops / shifts). A float operand
/// is `NonInteger`; a non-number is `NonNumeric`.
fn read_int(obj: *mut TclObj) -> Result<IntVal, ArithError> {
    match num(obj)? {
        NumVal::Wide(w) => Ok(IntVal::Wide(w)),
        NumVal::Big(m) => Ok(IntVal::Big(m)),
        NumVal::Float(_) => Err(ArithError::NonInteger),
    }
}

fn int_to_mp(v: IntVal) -> Result<Mp, ArithError> {
    match v {
        IntVal::Wide(w) => Mp::from_i64(w).ok_or(ArithError::Alloc),
        IntVal::Big(m) => Ok(m),
    }
}

#[inline]
fn mp_is_neg(m: &Mp) -> bool {
    m.0.sign != 0 // MP_ZPOS == 0, MP_NEG == 1 (libtommath normalises 0 to ZPOS)
}

#[inline]
fn mp_is_even(m: &Mp) -> bool {
    // SAFETY: `m` is a live mp_int; `used == 0` is zero (even), else the low
    // digit's bit 0 gives parity.
    m.0.used == 0 || unsafe { *m.0.dp & 1 == 0 }
}

/// The `i64` stand-in for a beyond-wide exponent: the tower's `**` rules past
/// `MAX_EXPONENT` depend only on the exponent's **sign and parity** (`(-1) **
/// 10**20` is `1`; `2 ** 10**20` is "exponent too large"; `2 ** -10**20` is
/// `0`), so folding a bignum exponent to an equal-sign, equal-parity wide is
/// exact. The same fold `tcl-vm`'s `big_pow` uses.
fn saturating_exponent(eb: &Mp) -> i64 {
    use tcl_syntax::number_tower::MAX_EXPONENT;
    match (mp_is_neg(eb), mp_is_even(eb)) {
        (true, true) => -2,
        (true, false) => -1,
        (false, true) => MAX_EXPONENT + 1,
        (false, false) => MAX_EXPONENT + 2,
    }
}

/// `a ** b` over the tower (TIP 123 integer rules + float `pow`).
///
/// The integer tier is the shared owner's ([`tcl_syntax::number_tower::int_pow`]
/// over the [`TowerMp`] adapter): the zero/`±1` base collapses, the
/// negative-exponent floor, and C's `2^28` exponent ceiling all live there, so
/// `3 ** 268435456` is an instant "exponent too large" rather than a
/// multi-hundred-megabit allocation (#1428).
pub fn pow(a: *mut TclObj, b: *mut TclObj) -> Result<*mut TclObj, ArithError> {
    let base = num(a)?;
    let exp = num(b)?;
    // Float exponentiation when either operand is a float.
    if matches!(base, NumVal::Float(_)) || matches!(exp, NumVal::Float(_)) {
        let (p, q) = (as_f64(base), as_f64(exp));
        if p == 0.0 && q < 0.0 {
            return Err(ArithError::ZeroToNegativePower);
        }
        return Ok(obj::new_double_obj(p.powf(q)));
    }
    let e = match exp {
        NumVal::Wide(e) => e,
        NumVal::Big(eb) => saturating_exponent(&eb),
        NumVal::Float(_) => unreachable!("float exponents took the branch above"),
    };
    let base = tower_of(base)?;
    match tcl_syntax::number_tower::int_pow(&base, e) {
        Some(r) => Ok(tower_obj(r)),
        // `int_pow` declines exactly two cases, which C reports differently: a
        // zero base with a negative exponent is the `ARITH DOMAIN`
        // "exponentiation of zero by negative power"; anything else is the
        // exponent ceiling.
        None if tcl_syntax::number_tower::BigIntOps::is_zero(&base) => {
            Err(ArithError::ZeroToNegativePower)
        }
        None => Err(ArithError::ExponentTooLarge),
    }
}

/// The bitwise binary ops.
#[derive(Clone, Copy)]
enum BitOp {
    And,
    Or,
    Xor,
}

fn bitwise(op: BitOp, a: *mut TclObj, b: *mut TclObj) -> Result<*mut TclObj, ArithError> {
    let x = read_int(a)?;
    let y = read_int(b)?;
    // Wide fast path — `&`/`|`/`^` of two wides always fits a wide.
    if let (IntVal::Wide(p), IntVal::Wide(q)) = (&x, &y) {
        let r = match op {
            BitOp::And => p & q,
            BitOp::Or => p | q,
            BitOp::Xor => p ^ q,
        };
        return Ok(obj::new_wide_int_obj(r));
    }
    let pm = int_to_mp(x)?;
    let qm = int_to_mp(y)?;
    let mut out = Mp::zero().ok_or(ArithError::Alloc)?;
    let f = match op {
        BitOp::And => mp_and,
        BitOp::Or => mp_or,
        BitOp::Xor => mp_xor,
    };
    // SAFETY: live mp_ints; `out` freshly initialised.
    if unsafe { f(pm.ptr(), qm.ptr(), &mut out.0) } != MP_OKAY {
        return Err(ArithError::Alloc);
    }
    Ok(store(out.into_inner()))
}

/// `a & b` over the tower (integer-only).
pub fn band(a: *mut TclObj, b: *mut TclObj) -> Result<*mut TclObj, ArithError> {
    bitwise(BitOp::And, a, b)
}
/// `a | b` over the tower (integer-only).
pub fn bor(a: *mut TclObj, b: *mut TclObj) -> Result<*mut TclObj, ArithError> {
    bitwise(BitOp::Or, a, b)
}
/// `a ^ b` over the tower (integer-only).
pub fn bxor(a: *mut TclObj, b: *mut TclObj) -> Result<*mut TclObj, ArithError> {
    bitwise(BitOp::Xor, a, b)
}

/// `~a` over the tower (integer-only; `~a == -a-1`).
pub fn bnot(a: *mut TclObj) -> Result<*mut TclObj, ArithError> {
    match read_int(a)? {
        IntVal::Wide(w) => Ok(obj::new_wide_int_obj(!w)),
        IntVal::Big(m) => {
            let mut out = Mp::zero().ok_or(ArithError::Alloc)?;
            // SAFETY: live mp_ints.
            if unsafe { mp_complement(m.ptr(), &mut out.0) } != MP_OKAY {
                return Err(ArithError::Alloc);
            }
            Ok(store(out.into_inner()))
        }
    }
}

/// A shift count folded to a wide: a beyond-wide count keeps only its **sign**,
/// which is all either direction needs (`>>` collapses to the operand's sign at
/// any count past its width; `<<` refuses any count past `INT_MAX` anyway), so
/// `i64::MAX` / `-1` are exact stand-ins.
fn shift_count(b: *mut TclObj) -> Result<i64, ArithError> {
    Ok(match read_int(b)? {
        IntVal::Wide(c) => c,
        IntVal::Big(m) => {
            if mp_is_neg(&m) {
                -1
            } else {
                i64::MAX
            }
        }
    })
}

/// `a << b` over the tower (integer-only). The operand is read **before** the
/// count (C reports a float left operand rather than the count problem:
/// `expr {1.5 << -1}` is the operand-type error), a zero base short-circuits at
/// any count (`0 << 10**20` is `0`), and the shift itself is the tower
/// adapter's `mp_mul_2d`; `store` demotes a small result back to a wide.
pub fn shl(a: *mut TclObj, b: *mut TclObj) -> Result<*mut TclObj, ArithError> {
    let base = int_tower(read_int(a)?);
    let count = shift_count(b)?;
    if count < 0 {
        return Err(ArithError::NegativeShift);
    }
    if tcl_syntax::number_tower::BigIntOps::is_zero(&base) {
        return Ok(obj::new_wide_int_obj(0));
    }
    // C's `mp_mul_2d` count is an `int`; past `INT_MAX` it raises the overflow
    // error rather than attempting an astronomic result.
    let count = u32::try_from(count)
        .ok()
        .filter(|&c| i32::try_from(c).is_ok())
        .ok_or(ArithError::TooLargeToRepresent)?;
    Ok(tower_obj(tcl_syntax::number_tower::BigIntOps::shl(
        &base, count,
    )))
}

/// `a >> b` over the tower (integer-only, arithmetic/sign-extending) — the
/// shared owner's [`int_shr`](tcl_syntax::number_tower::int_shr), including the
/// width collapse (`2 >> 10**20` is `0`, `-2 >> 10**20` is `-1`) and the
/// negative-count refusal.
pub fn shr(a: *mut TclObj, b: *mut TclObj) -> Result<*mut TclObj, ArithError> {
    let value = int_tower(read_int(a)?);
    let count = shift_count(b)?;
    let r = tcl_syntax::number_tower::int_shr(&value, count).ok_or(ArithError::NegativeShift)?;
    Ok(tower_obj(r))
}

/// The `bignum` type descriptor (the shimmer keystone for arbitrary-precision
/// integers). Free clears the `mp_int` + frees its box; dup deep-copies it;
/// update-string renders the canonical decimal.
pub static TCL_BIGNUM_TYPE: TclObjType = TclObjType {
    name: c"bignum".as_ptr(),
    free_int_rep_proc: Some(bignum_free),
    dup_int_rep_proc: Some(bignum_dup),
    update_string_proc: Some(bignum_update_string),
    set_from_any_proc: None,
};

/// The heap `mp_int` a bignum obj points to (read from `internal_rep`).
#[inline]
fn mp_ptr(obj: *mut TclObj) -> *mut MpInt {
    obj::internal_rep(obj) as *mut MpInt
}

extern "C" fn bignum_free(obj: *mut TclObj) {
    let p = mp_ptr(obj);
    if !p.is_null() {
        // SAFETY: `p` is a box we created in `store`; clear the mp_int's digit
        // array, then drop the box (frees the struct).
        unsafe {
            mp_clear(p);
            drop(Box::from_raw(p));
        }
    }
}

extern "C" fn bignum_dup(src: *mut TclObj, dup: *mut TclObj) {
    let mut copy = zeroed_mp();
    // SAFETY: `src` holds a live mp_int; copy it into a fresh box for `dup`.
    unsafe {
        if mp_init_copy(&mut copy, mp_ptr(src)) != MP_OKAY {
            return; // OOM: leave `dup` typeless (a benign empty value)
        }
        let boxed = Box::into_raw(Box::new(copy));
        (*dup).type_ptr = &TCL_BIGNUM_TYPE;
        (*dup).internal_rep = boxed as u64;
    }
}

extern "C" fn bignum_update_string(obj: *mut TclObj) {
    let p = mp_ptr(obj);
    // SAFETY: `p` is the live mp_int; render its canonical base-10 string.
    unsafe {
        let mut size: c_int = 0;
        if mp_radix_size(p, 10, &mut size) != MP_OKAY || size <= 0 {
            obj::set_string_rep(obj, b"0");
            return;
        }
        let mut buf = vec![0u8; size as usize];
        let mut written: usize = 0;
        if mp_to_radix(
            p,
            buf.as_mut_ptr() as *mut c_char,
            buf.len(),
            &mut written,
            10,
        ) != MP_OKAY
        {
            obj::set_string_rep(obj, b"0");
            return;
        }
        // `written` counts the trailing NUL; the string is the bytes before it.
        let end = written.saturating_sub(1).min(buf.len());
        obj::set_string_rep(obj, &buf[..end]);
    }
}

#[inline]
fn zeroed_mp() -> MpInt {
    MpInt {
        used: 0,
        alloc: 0,
        sign: 0,
        dp: core::ptr::null_mut(),
    }
}

/// Build a numeric object from a parsed [`Number::Big`](tcl_syntax::number::Number)
/// — `digits` is the magnitude in `radix` (no sign/prefix/separators). Applies
/// the tower's **demote-when-fits** canonicalisation: a value that fits a wide
/// returns a `TCL_INT_TYPE` object instead (so equality/hashing/string stay
/// stable). Returns null on allocation/parse failure.
#[must_use]
pub fn from_big_digits(negative: bool, radix: Radix, digits: &str) -> *mut TclObj {
    // libtommath's `mp_read_radix` consumes a leading '-', so build a signed,
    // NUL-terminated C string.
    let mut s = Vec::with_capacity(digits.len() + 2);
    if negative {
        s.push(b'-');
    }
    s.extend_from_slice(digits.as_bytes());
    s.push(0);

    let mut mp = zeroed_mp();
    // SAFETY: initialise then parse into a stack mp_int; on any failure clear it.
    unsafe {
        if mp_init(&mut mp) != MP_OKAY {
            return core::ptr::null_mut();
        }
        if mp_read_radix(&mut mp, s.as_ptr() as *const c_char, radix as c_int) != MP_OKAY {
            mp_clear(&mut mp);
            return core::ptr::null_mut();
        }
    }
    store(mp)
}

/// Install a (stack) `mp_int` as a bignum object, demoting to a wide when it
/// fits. Takes ownership of `mp` (clears it on the demote path).
fn store(mut mp: MpInt) -> *mut TclObj {
    // SAFETY: `mp` is a live, owned mp_int.
    let bits = unsafe { mp_count_bits(&mp) };
    if bits <= 63 {
        // Fits a wide (magnitude < 2^63) — demote. (i64::MIN, a 64-bit
        // magnitude, conservatively stays bignum for now; correctness-safe.)
        let v = unsafe { mp_get_i64(&mp) };
        unsafe { mp_clear(&mut mp) };
        return obj::new_wide_int_obj(v);
    }
    let boxed = Box::into_raw(Box::new(mp));
    obj::alloc_typed(&TCL_BIGNUM_TYPE, boxed as u64)
}

// ---------------------------------------------------------------------------
// The shared-tower backend adapter: `BigIntOps` over the real `mp_int`.
// ---------------------------------------------------------------------------

/// Lift an integer operand onto the shared tower's backend value. A float
/// operand is `NonInteger` (the integer tiers of `**`/`/`/`%`/`<<`/`>>` are
/// reached only once the float promotion has been ruled out).
fn tower_of(v: NumVal) -> Result<TowerMp, ArithError> {
    Ok(match v {
        NumVal::Wide(w) => tcl_syntax::number_tower::BigIntOps::from_i64(w),
        NumVal::Big(mp) => TowerMp(mp),
        NumVal::Float(_) => return Err(ArithError::NonInteger),
    })
}

/// Lift an already-validated integer operand ([`read_int`]) onto the tower.
fn int_tower(v: IntVal) -> TowerMp {
    match v {
        IntVal::Wide(w) => tcl_syntax::number_tower::BigIntOps::from_i64(w),
        IntVal::Big(m) => TowerMp(m),
    }
}

/// Materialise a tower result as a fresh object, demoting when it fits a wide
/// (`$big - $big` is `0`, never a one-word bignum).
fn tower_obj(v: TowerMp) -> *mut TclObj {
    to_obj(NumVal::Big(v.0))
}

/// The libtommath backend for the shared tower semantics
/// (`tcl_syntax::number_tower::BigIntOps`) — the adapter that closes the
/// tower's backend seam over the real `mp_int`. The pure-Rust adopters (the
/// compiler's const-folder, the VM) run the identical oracle-conformance
/// corpus over `num-bigint`; this type runs it over libtommath, so the two
/// backends cannot drift on any covered semantic. The trait is infallible,
/// so allocation failure — libtommath's only failure mode on these
/// operations — panics, matching Rust's global-allocator convention.
pub struct TowerMp(Mp);

impl TowerMp {
    /// Run `f` into a fresh `mp_int` and wrap it.
    fn build(f: impl FnOnce(*mut MpInt) -> c_int) -> Self {
        let mut out = Mp::zero().expect("libtommath alloc");
        assert!(f(&mut out.0) == MP_OKAY, "libtommath alloc");
        TowerMp(out)
    }

    /// Three-way compare via `mp_cmp`.
    fn cmp_mp(&self, other: &Self) -> core::cmp::Ordering {
        // SAFETY: both live mp_ints; mp_cmp returns -1/0/1.
        match unsafe { mp_cmp(self.0.ptr(), other.0.ptr()) } {
            n if n < 0 => core::cmp::Ordering::Less,
            0 => core::cmp::Ordering::Equal,
            _ => core::cmp::Ordering::Greater,
        }
    }
}

impl Clone for TowerMp {
    fn clone(&self) -> Self {
        TowerMp(Mp::copy_of(self.0.ptr()).expect("libtommath alloc"))
    }
}

impl PartialEq for TowerMp {
    fn eq(&self, other: &Self) -> bool {
        self.cmp_mp(other) == core::cmp::Ordering::Equal
    }
}
impl Eq for TowerMp {}
impl PartialOrd for TowerMp {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for TowerMp {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.cmp_mp(other)
    }
}

impl core::fmt::Debug for TowerMp {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Decimal via mp_to_radix (small values only appear in test output).
        let p = self.0.ptr();
        // SAFETY: live mp_int; buffer sized by mp_radix_size (incl. NUL).
        unsafe {
            let mut size: c_int = 0;
            if mp_radix_size(p, 10, &mut size) != MP_OKAY || size <= 0 {
                return write!(f, "<mp?>");
            }
            let mut buf = vec![0u8; size as usize];
            let mut written: usize = 0;
            if mp_to_radix(
                p,
                buf.as_mut_ptr() as *mut c_char,
                buf.len(),
                &mut written,
                10,
            ) != MP_OKAY
            {
                return write!(f, "<mp?>");
            }
            let end = written.saturating_sub(1).min(buf.len());
            write!(f, "{}", String::from_utf8_lossy(&buf[..end]))
        }
    }
}

impl tcl_syntax::number_tower::BigIntOps for TowerMp {
    fn from_i64(v: i64) -> Self {
        TowerMp(Mp::from_i64(v).expect("libtommath alloc"))
    }
    fn to_i64(&self) -> Option<i64> {
        // Exact range check (`store`'s demote is deliberately conservative
        // about i64::MIN; the trait contract is exact).
        let min = Self::from_i64(i64::MIN);
        let max = Self::from_i64(i64::MAX);
        (self.cmp_mp(&min) != core::cmp::Ordering::Less
            && self.cmp_mp(&max) != core::cmp::Ordering::Greater)
            // SAFETY: live mp_int within i64 range.
            .then(|| unsafe { mp_get_i64(self.0.ptr()) })
    }
    fn to_i64_wrapping(&self) -> i64 {
        // SAFETY: live mp_int; libtommath exposes Tcl's low-64-bit fold.
        unsafe { mp_get_i64(self.0.ptr()) }
    }
    fn is_zero(&self) -> bool {
        self.0 .0.used == 0
    }
    fn is_negative(&self) -> bool {
        self.0 .0.used != 0 && self.0 .0.sign != 0
    }
    fn add(&self, other: &Self) -> Self {
        // SAFETY (each op below): live operand mp_ints; out is initialised.
        Self::build(|out| unsafe { mp_add(self.0.ptr(), other.0.ptr(), out) })
    }
    fn sub(&self, other: &Self) -> Self {
        Self::build(|out| unsafe { mp_sub(self.0.ptr(), other.0.ptr(), out) })
    }
    fn mul(&self, other: &Self) -> Self {
        Self::build(|out| unsafe { mp_mul(self.0.ptr(), other.0.ptr(), out) })
    }
    fn div_floor(&self, other: &Self) -> Self {
        let (q, _) = mp_floor_divmod(&self.0, &other.0).expect("libtommath alloc");
        TowerMp(q)
    }
    fn mod_floor(&self, other: &Self) -> Self {
        let (_, r) = mp_floor_divmod(&self.0, &other.0).expect("libtommath alloc");
        TowerMp(r)
    }
    fn neg(&self) -> Self {
        Self::build(|out| unsafe { mp_neg(self.0.ptr(), out) })
    }
    fn pow_u32(&self, exp: u32) -> Self {
        let e = c_int::try_from(exp).expect("exponent fits c_int (tower-capped)");
        Self::build(|out| unsafe { mp_expt_n(self.0.ptr(), e, out) })
    }
    fn shl(&self, count: u32) -> Self {
        let c = c_int::try_from(count).expect("shift count fits c_int");
        Self::build(|out| unsafe { mp_mul_2d(self.0.ptr(), c, out) })
    }
    fn shr(&self, count: usize) -> Self {
        // The tower only calls this with `count <= bit_len` (the collapse
        // guard), so the count always fits c_int.
        let c = c_int::try_from(count).expect("shift count fits c_int");
        Self::build(|out| unsafe { mp_signed_rsh(self.0.ptr(), c, out) })
    }
    fn bitand(&self, other: &Self) -> Self {
        Self::build(|out| unsafe { mp_and(self.0.ptr(), other.0.ptr(), out) })
    }
    fn bitor(&self, other: &Self) -> Self {
        Self::build(|out| unsafe { mp_or(self.0.ptr(), other.0.ptr(), out) })
    }
    fn bitxor(&self, other: &Self) -> Self {
        Self::build(|out| unsafe { mp_xor(self.0.ptr(), other.0.ptr(), out) })
    }
    fn bit_len(&self) -> u64 {
        // SAFETY: live mp_int; count is non-negative.
        u64::try_from(unsafe { mp_count_bits(self.0.ptr()) }).unwrap_or(0)
    }
    fn to_f64(&self) -> f64 {
        // SAFETY: live mp_int; libtommath performs the correctly-rounded
        // bignum-to-double conversion used by Tcl's numeric tower.
        unsafe { mp_get_double(self.0.ptr()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obj;

    /// The real libtommath backend passes the shared tower conformance
    /// corpus — the differential proof that `mp_int` and the pure-Rust
    /// `num-bigint` backend (compiler/VM) implement identical semantics.
    #[test]
    fn tower_conformance_libtommath() {
        tcl_syntax::number_tower::conformance::assert_backend::<TowerMp>();
    }

    fn string_of(obj: *mut TclObj) -> Vec<u8> {
        // Force the string rep and read it back.
        let mut len = 0isize;
        // SAFETY: `obj` is a live object; Tcl_GetStringFromObj shimmers + borrows.
        unsafe {
            let p = crate::capi::Tcl_GetStringFromObj(obj, &mut len);
            core::slice::from_raw_parts(p as *const u8, len as usize).to_vec()
        }
    }

    fn type_name(obj: *mut TclObj) -> &'static str {
        let tp = obj::obj_type_ptr(obj);
        if tp == &TCL_BIGNUM_TYPE {
            "bignum"
        } else if tp.is_null() {
            "string"
        } else {
            "int/other"
        }
    }

    #[test]
    fn big_value_stays_bignum_and_stringifies() {
        crate::counters::reset();
        // 2**100 — well past a wide.
        let digits = "1267650600228229401496703205376";
        let o = from_big_digits(false, Radix::Dec, digits);
        assert!(!o.is_null());
        // SAFETY: take an owning ref then release it.
        unsafe { obj::incr_ref_count(o) };
        assert_eq!(type_name(o), "bignum");
        assert_eq!(string_of(o), digits.as_bytes());
        unsafe { obj::decr_ref_count(o) };
        assert_eq!(crate::counters::finalize(), 0, "leak");
    }

    #[test]
    fn negative_bignum_round_trips() {
        crate::counters::reset();
        let digits = "1267650600228229401496703205376";
        let o = from_big_digits(true, Radix::Dec, digits);
        unsafe { obj::incr_ref_count(o) };
        let mut expected = b"-".to_vec();
        expected.extend_from_slice(digits.as_bytes());
        assert_eq!(string_of(o), expected);
        unsafe { obj::decr_ref_count(o) };
        assert_eq!(crate::counters::finalize(), 0);
    }

    #[test]
    fn fits_wide_demotes_to_int() {
        crate::counters::reset();
        // A value the parser would call "Big" only if it overflowed; here we feed
        // a small magnitude and confirm `store` demotes it to a plain int.
        let o = from_big_digits(false, Radix::Dec, "42");
        unsafe { obj::incr_ref_count(o) };
        assert_eq!(type_name(o), "int/other"); // demoted to TCL_INT_TYPE
        assert_eq!(string_of(o), b"42");
        unsafe { obj::decr_ref_count(o) };
        assert_eq!(crate::counters::finalize(), 0);
    }

    #[test]
    fn hex_radix_and_dup() {
        crate::counters::reset();
        // 0xffff_ffff_ffff_ffff_f magnitude (the number-grammar test's Big case).
        let o = from_big_digits(false, Radix::Hex, "fffffffffffffffff");
        unsafe { obj::incr_ref_count(o) };
        assert_eq!(type_name(o), "bignum");
        // Duplicate must deep-copy (independent mp_int).
        let d = obj::duplicate(o);
        unsafe { obj::incr_ref_count(d) };
        assert_eq!(string_of(d), string_of(o));
        unsafe {
            obj::decr_ref_count(d);
            obj::decr_ref_count(o);
        }
        assert_eq!(crate::counters::finalize(), 0);
    }

    // ---- tower arithmetic ----
    //
    // Helpers own (`rc 1`) every operand + result and release them, so each test
    // ends leak-clean. The ops *borrow* operands (never consume them).

    fn rc1(o: *mut TclObj) -> *mut TclObj {
        unsafe { obj::incr_ref_count(o) };
        o
    }
    fn drop1(o: *mut TclObj) {
        unsafe { obj::decr_ref_count(o) };
    }
    fn int_obj(v: i64) -> *mut TclObj {
        rc1(obj::new_wide_int_obj(v))
    }

    /// Apply a binary op to two operands, returning `(string, type)`; releases
    /// the operands and the result.
    fn binop(
        f: fn(*mut TclObj, *mut TclObj) -> Result<*mut TclObj, ArithError>,
        a: *mut TclObj,
        b: *mut TclObj,
    ) -> (Vec<u8>, &'static str) {
        let r = rc1(f(a, b).expect("numeric"));
        let out = (string_of(r), type_name(r));
        drop1(r);
        drop1(a);
        drop1(b);
        out
    }

    /// A fresh (`rc 1`) `2**63` bignum object (just past a wide).
    fn two_pow_63() -> *mut TclObj {
        let a = int_obj(i64::MAX);
        let b = int_obj(1);
        let r = rc1(add(a, b).expect("add"));
        drop1(a);
        drop1(b);
        r
    }

    #[test]
    fn wide_overflows_to_bignum_then_demotes() {
        crate::counters::reset();
        // i64::MAX + 1 → bignum
        let (s, t) = binop(add, int_obj(i64::MAX), int_obj(1));
        assert_eq!(s, b"9223372036854775808");
        assert_eq!(t, "bignum");
        // (2**63) - 1 → demotes back to a wide
        let (s, t) = binop(sub, two_pow_63(), int_obj(1));
        assert_eq!(s, b"9223372036854775807");
        assert_eq!(t, "int/other");
        assert_eq!(crate::counters::finalize(), 0);
    }

    #[test]
    fn wide_fast_path_stays_wide() {
        crate::counters::reset();
        assert_eq!(binop(add, int_obj(2), int_obj(40)).0, b"42");
        assert_eq!(
            binop(mul, int_obj(6), int_obj(7)),
            (b"42".to_vec(), "int/other")
        );
        assert_eq!(crate::counters::finalize(), 0);
    }

    #[test]
    fn bignum_times_bignum() {
        crate::counters::reset();
        // (2**63) * (2**63) = 2**126
        let (s, t) = binop(mul, two_pow_63(), two_pow_63());
        assert_eq!(s, b"85070591730234615865843651857942052864");
        assert_eq!(t, "bignum");
        assert_eq!(crate::counters::finalize(), 0);
    }

    #[test]
    fn double_promotion_and_neg() {
        crate::counters::reset();
        // 2 + 0.5 → 2.5 (double)
        let (s, t) = binop(add, int_obj(2), rc1(obj::new_double_obj(0.5)));
        assert_eq!(s, b"2.5");
        assert_eq!(t, "int/other"); // a double (non-bignum, non-string)
                                    // -i64::MIN promotes to bignum
        let m = int_obj(i64::MIN);
        let r = rc1(neg(m).expect("neg"));
        assert_eq!(string_of(r), b"9223372036854775808");
        assert_eq!(type_name(r), "bignum");
        drop1(r);
        drop1(m);
        assert_eq!(crate::counters::finalize(), 0);
    }

    #[test]
    fn reads_numeric_strings_via_grammar() {
        crate::counters::reset();
        // plain-string operands get classified by the shared `tcl_syntax::number`
        let (s, _) = binop(
            add,
            rc1(obj::new_string_bytes(b"0xff")),
            rc1(obj::new_string_bytes(b"1")),
        );
        assert_eq!(s, b"256");
        assert_eq!(crate::counters::finalize(), 0);
    }

    #[test]
    fn floor_div_and_mod() {
        crate::counters::reset();
        // -7 / 2 = -4 (floor), -7 % 2 = 1 (sign of divisor)
        assert_eq!(binop(div, int_obj(-7), int_obj(2)).0, b"-4");
        assert_eq!(binop(mod_, int_obj(-7), int_obj(2)).0, b"1");
        // 7 / -2 = -4, 7 % -2 = -1
        assert_eq!(binop(div, int_obj(7), int_obj(-2)).0, b"-4");
        assert_eq!(binop(mod_, int_obj(7), int_obj(-2)).0, b"-1");
        // positive case unchanged
        assert_eq!(binop(div, int_obj(7), int_obj(2)).0, b"3");
        assert_eq!(binop(mod_, int_obj(7), int_obj(2)).0, b"1");
        assert_eq!(crate::counters::finalize(), 0);
    }

    #[test]
    fn bignum_floor_div() {
        crate::counters::reset();
        // floor(-2**63 / 2**63) = -1 via the bignum path
        let big = two_pow_63();
        let neg_big = rc1(neg(big).expect("neg"));
        drop1(big);
        let (q, _) = binop(div, neg_big, two_pow_63()); // binop frees both operands
        assert_eq!(q, b"-1");
        assert_eq!(crate::counters::finalize(), 0);
    }

    #[test]
    fn divide_by_zero_errors() {
        crate::counters::reset();
        let a = int_obj(5);
        let z = int_obj(0);
        assert_eq!(div(a, z), Err(ArithError::DivideByZero));
        assert_eq!(mod_(a, z), Err(ArithError::DivideByZero));
        drop1(a);
        drop1(z);
        assert_eq!(crate::counters::finalize(), 0);
    }

    #[test]
    fn compares_across_rungs() {
        use core::cmp::Ordering;
        crate::counters::reset();
        let big = two_pow_63();
        let small = int_obj(5);
        assert_eq!(
            compare(big, small),
            Some(NumericCompare::Ordered(Ordering::Greater))
        ); // bignum > wide
        assert_eq!(
            compare(small, big),
            Some(NumericCompare::Ordered(Ordering::Less))
        );
        let two = int_obj(2);
        let half = rc1(obj::new_double_obj(2.5));
        assert_eq!(
            compare(two, half),
            Some(NumericCompare::Ordered(Ordering::Less))
        ); // 2 < 2.5
        let two_eq = int_obj(2);
        let two_b = int_obj(2);
        assert_eq!(
            compare(two_eq, two_b),
            Some(NumericCompare::Ordered(Ordering::Equal))
        );
        for o in [big, small, two, half, two_eq, two_b] {
            drop1(o);
        }
        assert_eq!(crate::counters::finalize(), 0);
    }

    #[test]
    fn exponentiation() {
        crate::counters::reset();
        assert_eq!(
            binop(pow, int_obj(2), int_obj(10)),
            (b"1024".to_vec(), "int/other")
        );
        assert_eq!(binop(pow, int_obj(5), int_obj(0)).0, b"1");
        assert_eq!(binop(pow, int_obj(0), int_obj(5)).0, b"0");
        assert_eq!(binop(pow, int_obj(-1), int_obj(3)).0, b"-1");
        assert_eq!(binop(pow, int_obj(-1), int_obj(4)).0, b"1");
        // |base|>=2 with a negative exponent floors to 0 (TIP 123)
        assert_eq!(binop(pow, int_obj(2), int_obj(-1)).0, b"0");
        // 2**64 overflows a wide → bignum
        let (s, t) = binop(pow, int_obj(2), int_obj(64));
        assert_eq!(s, b"18446744073709551616");
        assert_eq!(t, "bignum");
        // 2**128 (bignum)
        assert_eq!(
            binop(pow, int_obj(2), int_obj(128)).0,
            b"340282366920938463463374607431768211456"
        );
        // 0 ** -1 is C's *domain* error, not a division by zero (tclsh
        // 8.6.16/9.0.4: `exponentiation of zero by negative power`,
        // `-errorcode ARITH DOMAIN`) — #1428.
        let z = int_obj(0);
        let m1 = int_obj(-1);
        assert_eq!(pow(z, m1), Err(ArithError::ZeroToNegativePower));
        drop1(z);
        drop1(m1);
        assert_eq!(crate::counters::finalize(), 0);
    }

    #[test]
    fn bitwise_ops() {
        crate::counters::reset();
        assert_eq!(binop(band, int_obj(0xff), int_obj(0x0f)).0, b"15");
        assert_eq!(binop(bor, int_obj(12), int_obj(3)).0, b"15");
        assert_eq!(binop(bxor, int_obj(5), int_obj(3)).0, b"6");
        // ~a == -a-1
        let a = int_obj(5);
        let r = rc1(bnot(a).expect("bnot"));
        assert_eq!(string_of(r), b"-6");
        drop1(r);
        drop1(a);
        // float operand → NonInteger
        let f = rc1(obj::new_double_obj(1.5));
        let two = int_obj(2);
        assert_eq!(band(f, two), Err(ArithError::NonInteger));
        drop1(f);
        drop1(two);
        assert_eq!(crate::counters::finalize(), 0);
    }

    #[test]
    fn shifts() {
        crate::counters::reset();
        assert_eq!(binop(shl, int_obj(1), int_obj(4)).0, b"16");
        assert_eq!(binop(shr, int_obj(256), int_obj(4)).0, b"16");
        assert_eq!(binop(shr, int_obj(-8), int_obj(1)).0, b"-4"); // arithmetic
                                                                  // 1 << 64 overflows a wide → bignum 2**64
        let (s, t) = binop(shl, int_obj(1), int_obj(64));
        assert_eq!(s, b"18446744073709551616");
        assert_eq!(t, "bignum");
        // negative shift → error
        let a = int_obj(1);
        let n = int_obj(-1);
        assert_eq!(shl(a, n), Err(ArithError::NegativeShift));
        drop1(a);
        drop1(n);
        assert_eq!(crate::counters::finalize(), 0);
    }

    /// A beyond-wide positive shift count: `>>` collapses to the operand's
    /// sign while `<<` is C's "integer value too large to represent" —
    /// `expr {2 >> 10**20}` is `0`, `{-2 >> 10**20}` is `-1` (tclsh
    /// 8.6/9.0 verified), never "exponent too large".
    #[test]
    fn huge_shift_counts() {
        crate::counters::reset();
        let huge = rc1(from_big_digits(false, Radix::Dec, "100000000000000000000"));
        let two = int_obj(2);
        let neg_two = int_obj(-2);
        let r = rc1(shr(two, huge).expect("2 >> huge collapses"));
        assert_eq!(string_of(r), b"0");
        drop1(r);
        let r = rc1(shr(neg_two, huge).expect("-2 >> huge collapses"));
        assert_eq!(string_of(r), b"-1");
        drop1(r);
        assert_eq!(shl(two, huge), Err(ArithError::TooLargeToRepresent));
        // A wide count past c_int is the same error, not "exponent".
        let wide_count = int_obj(4_294_967_296);
        assert_eq!(shl(two, wide_count), Err(ArithError::TooLargeToRepresent));
        drop1(wide_count);
        drop1(two);
        drop1(neg_two);
        drop1(huge);
        assert_eq!(crate::counters::finalize(), 0);
    }
}
