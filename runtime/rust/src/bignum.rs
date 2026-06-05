//! The bignum rung of the numeric tower: the `TCL_BIGNUM_TYPE` obj rep over
//! libtommath `mp_int`, the representation chosen + validated in EXP-BIGNUM
//! (`docs/design/runtime/rust-runtime-port.md`).
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
}

// ---------------------------------------------------------------------------
// Tower arithmetic — the integer rung (wide → bignum, with demote-when-fits)
// plus double promotion. Follows `tclExecute.c`'s overflow-checked wide fast
// path → `ExecuteExtendedBinaryMathOp` bignum path → canonical demote. Operands
// are `TclObj`s; results are fresh (`rc 0`) `TclObj`s (int / bignum / double).
// Covers +/-/*/neg, floor `/`/`%` (sign-of-divisor), and numeric comparison;
// `**`, bit-ops, and the `expr` walker build on this next.
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
    // Untyped (or other): classify the string rep.
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
    /// Integer `/` or `%` by zero (`divide by zero`).
    DivideByZero,
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
    // Float division when either operand is a float (`%` on a float is an error
    // in Tcl, but that surfaces at the expr layer; here `/` floats, `%` would
    // already be rejected upstream).
    if matches!(x, NumVal::Float(_)) || matches!(y, NumVal::Float(_)) {
        let (p, q) = (as_f64(x), as_f64(y));
        let r = if want_quotient { p / q } else { p % q };
        return Ok(obj::new_double_obj(r));
    }
    // Wide fast path.
    if let (NumVal::Wide(p), NumVal::Wide(q)) = (&x, &y) {
        let (p, q) = (*p, *q);
        if q == 0 {
            return Err(ArithError::DivideByZero);
        }
        // i64::MIN / -1 overflows the wide → bignum path below.
        if !(p == i64::MIN && q == -1) {
            let (quot, rem) = wide_floor_divmod(p, q);
            return Ok(obj::new_wide_int_obj(if want_quotient {
                quot
            } else {
                rem
            }));
        }
    }
    // Bignum path.
    let pm = into_mp(x).ok_or(ArithError::Alloc)?;
    let qm = into_mp(y).ok_or(ArithError::Alloc)?;
    // SAFETY: `qm` is live; a zero divisor is `used == 0`.
    if qm.0.used == 0 {
        return Err(ArithError::DivideByZero);
    }
    let (quot, rem) = mp_floor_divmod(&pm, &qm)?;
    Ok(store((if want_quotient { quot } else { rem }).into_inner()))
}

/// Floor division for wides: quotient toward −∞, remainder with the divisor's
/// sign (`a == (a/b)*b + (a%b)`). `b != 0` and not the `MIN/-1` overflow.
fn wide_floor_divmod(a: i64, b: i64) -> (i64, i64) {
    let mut q = a / b;
    let mut r = a % b;
    if r != 0 && (r < 0) != (b < 0) {
        q -= 1;
        r += b;
    }
    (q, r)
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
/// on a non-numeric operand (NaN compares as the IEEE result via `f64`).
#[must_use]
pub fn compare(a: *mut TclObj, b: *mut TclObj) -> Option<core::cmp::Ordering> {
    use core::cmp::Ordering;
    let x = read(a)?;
    let y = read(b)?;
    Some(match (x, y) {
        (NumVal::Wide(p), NumVal::Wide(q)) => p.cmp(&q),
        // Any float operand → compare as f64 (partial_cmp; NaN → treat as
        // Greater so it sorts consistently — the expr layer handles NaN rules).
        (p, q) if matches!(p, NumVal::Float(_)) || matches!(q, NumVal::Float(_)) => as_f64(p)
            .partial_cmp(&as_f64(q))
            .unwrap_or(Ordering::Greater),
        // At least one bignum → mp_cmp.
        (p, q) => {
            let pm = into_mp(p)?;
            let qm = into_mp(q)?;
            // SAFETY: live mp_ints; mp_cmp returns -1/0/1.
            match unsafe { mp_cmp(pm.ptr(), qm.ptr()) } {
                n if n < 0 => Ordering::Less,
                0 => Ordering::Equal,
                _ => Ordering::Greater,
            }
        }
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obj;

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
        assert_eq!(compare(big, small), Some(Ordering::Greater)); // bignum > wide
        assert_eq!(compare(small, big), Some(Ordering::Less));
        let two = int_obj(2);
        let half = rc1(obj::new_double_obj(2.5));
        assert_eq!(compare(two, half), Some(Ordering::Less)); // 2 < 2.5
        let two_eq = int_obj(2);
        let two_b = int_obj(2);
        assert_eq!(compare(two_eq, two_b), Some(Ordering::Equal));
        for o in [big, small, two, half, two_eq, two_b] {
            drop1(o);
        }
        assert_eq!(crate::counters::finalize(), 0);
    }
}
