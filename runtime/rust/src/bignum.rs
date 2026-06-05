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
}
