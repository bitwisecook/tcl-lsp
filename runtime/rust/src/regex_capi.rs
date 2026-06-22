//! C-ABI shim: re-export the pure-Rust [`tcl_regex`] ARE engine under the Tcl
//! regex engine's C symbols, so C Tcl code — `tclRegexp.c`, and any C extension
//! that links the engine — can compile and link against it unchanged. This is
//! the producer side that replaces the C Henry-Spencer engine entirely.
//!
//! The companion header is [`include/tcl_regex_capi.h`](../include/tcl_regex_capi.h).
//! We deliberately *hide the impedance mismatch in the shim* (the user's
//! design rule): the safe-Rust engine works in `&[u32]` codepoints, typed
//! errors, and half-open `Span`s, while C wants a `regex_t` with opaque
//! `re_guts`, a `regmatch_t[]` of `size_t` pairs with `(size_t)-1` sentinels,
//! and integer `REG_*` codes. The conversion lives here.
//!
//! ABI notes
//! - `chr` is a 32-bit codepoint (`CHRBITS == 32`, as the engine has always
//!   used); a C caller includes the shim header, which fixes that convention.
//! - Flag values (`REG_ADVANCED`, `REG_ICASE`, …, `REG_NOTBOL`/`REG_NOTEOL`)
//!   are the `regex.h` values, which `tcl-regex` already shares, so cflags /
//!   eflags pass straight through.
//! - `re_endp` (the `REG_PEND` kludge) and `re_fns` are unused; `re_guts` holds
//!   a `Box<tcl_regex::Regex>`.

use core::ffi::{c_char, c_int, c_long, c_void};
use tcl_regex::{ErrorCode, Regex};

/// 32-bit wide character (codepoint) — the engine's `chr`.
pub type Chr = u32;

const REG_OKAY: c_int = 0;
const REG_NOMATCH: c_int = 1;
const REG_INVARG: c_int = 16;
/// Magic number for a live `regex_t` (`REMAGIC` in `regguts.h`).
const REMAGIC: c_int = 0xFED7;
/// `(size_t)-1` — the engine's "subexpression did not participate" sentinel.
const NO_MATCH: usize = usize::MAX;

/// One reported (sub-)match: half-open in the engine, reported as the C
/// `regmatch_t` (`rm_so`/`rm_eo` are `size_t`).
#[repr(C)]
pub struct RegMatchT {
    pub rm_so: usize,
    pub rm_eo: usize,
}

/// `rm_detail_t` — supplementary reporting (the `REG_EXPECT` extent). Unused by
/// this engine, but kept in the ABI for source compatibility.
#[repr(C)]
pub struct RmDetailT {
    pub rm_extend: RegMatchT,
}

/// The public compiled-RE handle. Layout matches `regex.h`'s `regex_t`; only
/// `re_guts` (a boxed [`Regex`]), `re_nsub`, and `re_info` are meaningful here.
#[repr(C)]
pub struct RegexT {
    pub re_magic: c_int,
    pub re_info: c_long,
    pub re_nsub: usize,
    pub re_endp: *const Chr,
    pub re_guts: *mut c_void,
    pub re_fns: *mut c_void,
}

/// `TclReComp` — compile `pattern` (`len` codepoints) with `flags` into `re`.
/// Returns `REG_OKAY` or a `REG_*` error code.
///
/// # Safety
/// `re` must be a valid, writable `*mut RegexT`; `pattern` must point to `len`
/// readable `Chr`s (or be null with `len == 0`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn TclReComp(
    re: *mut RegexT,
    pattern: *const Chr,
    len: usize,
    flags: c_int,
) -> c_int {
    if re.is_null() || (pattern.is_null() && len != 0) {
        return REG_INVARG;
    }
    let pat: &[Chr] = if len == 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(pattern, len) }
    };
    match Regex::compile(pat, flags) {
        Ok(rx) => {
            let nsub = rx.nsub();
            let info = rx.info().bits();
            let guts = Box::into_raw(Box::new(rx)).cast::<c_void>();
            unsafe {
                (*re).re_magic = REMAGIC;
                (*re).re_info = c_long::from(info);
                (*re).re_nsub = nsub;
                (*re).re_endp = core::ptr::null();
                (*re).re_guts = guts;
                (*re).re_fns = core::ptr::null_mut();
            }
            REG_OKAY
        }
        Err(code) => code.code(),
    }
}

/// `TclReExec` — match `re` against `string` (`len` codepoints), filling up to
/// `nmatch` entries of `pmatch` with `[rm_so, rm_eo)` per group (group 0 is the
/// whole match; non-participating groups get `(size_t)-1`). `flags` carries
/// `REG_NOTBOL`/`REG_NOTEOL`. Returns `REG_OKAY` or `REG_NOMATCH`. `details` is
/// accepted for ABI compatibility and ignored (no `REG_EXPECT` support).
///
/// # Safety
/// `re` must be a `regex_t` previously filled by [`TclReComp`]; `string` must
/// point to `len` readable `Chr`s; `pmatch` must point to `nmatch` writable
/// `RegMatchT`s (or be null with `nmatch == 0`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn TclReExec(
    re: *mut RegexT,
    string: *const Chr,
    len: usize,
    _details: *mut RmDetailT,
    nmatch: usize,
    pmatch: *mut RegMatchT,
    flags: c_int,
) -> c_int {
    if re.is_null() || (string.is_null() && len != 0) {
        return REG_INVARG;
    }
    let guts = unsafe { (*re).re_guts }.cast::<Regex>();
    if guts.is_null() {
        return REG_INVARG;
    }
    let rx: &Regex = unsafe { &*guts };
    let subject: &[Chr] = if len == 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(string, len) }
    };
    match rx.exec(subject, 0, flags) {
        Some(groups) => {
            if !pmatch.is_null() && nmatch != 0 {
                let out = unsafe { core::slice::from_raw_parts_mut(pmatch, nmatch) };
                for (slot, group) in out
                    .iter_mut()
                    .zip(groups.iter().chain(core::iter::repeat(&None)))
                {
                    match group {
                        Some(span) => {
                            slot.rm_so = span.start;
                            slot.rm_eo = span.end;
                        }
                        None => {
                            slot.rm_so = NO_MATCH;
                            slot.rm_eo = NO_MATCH;
                        }
                    }
                }
            }
            REG_OKAY
        }
        None => REG_NOMATCH,
    }
}

/// `TclReFree` — release the engine state held in `re->re_guts`.
///
/// # Safety
/// `re` must be null or a `regex_t` filled by [`TclReComp`] and not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn TclReFree(re: *mut RegexT) {
    if re.is_null() {
        return;
    }
    let guts = unsafe { (*re).re_guts };
    if !guts.is_null() {
        drop(unsafe { Box::from_raw(guts.cast::<Regex>()) });
        unsafe {
            (*re).re_guts = core::ptr::null_mut();
            (*re).re_magic = 0;
        }
    }
}

/// `TclReError` — write the message for `code` into `errbuf` (NUL-terminated,
/// truncated to `errbuf_size`). Returns the full message length plus one (the
/// buffer size that would hold it), per the `regerror` convention.
///
/// # Safety
/// `errbuf` must point to `errbuf_size` writable bytes (or be null with
/// `errbuf_size == 0`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn TclReError(code: c_int, errbuf: *mut c_char, errbuf_size: usize) -> usize {
    let msg = ErrorCode::from_code(code).map_or("unknown regex error code", ErrorCode::message);
    let bytes = msg.as_bytes();
    if !errbuf.is_null() && errbuf_size != 0 {
        let n = bytes.len().min(errbuf_size - 1);
        unsafe {
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), errbuf.cast::<u8>(), n);
            *errbuf.add(n) = 0;
        }
    }
    bytes.len() + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive the C ABI exactly as a C caller would (compile → exec → free) and
    /// check the reported groups, sentinels, and error path.
    #[test]
    fn capi_roundtrip() {
        // `(\w+)@(\w+)` over "user@host" → whole + two groups.
        let pat: Vec<Chr> = r"(\w+)@(\w+)".chars().map(|c| c as u32).collect();
        let mut re = RegexT {
            re_magic: 0,
            re_info: 0,
            re_nsub: 0,
            re_endp: core::ptr::null(),
            re_guts: core::ptr::null_mut(),
            re_fns: core::ptr::null_mut(),
        };
        let rc = unsafe {
            TclReComp(
                &mut re,
                pat.as_ptr(),
                pat.len(),
                tcl_regex::defs::REG_ADVANCED,
            )
        };
        assert_eq!(rc, REG_OKAY);
        assert_eq!(re.re_nsub, 2);

        let subj: Vec<Chr> = "user@host".chars().map(|c| c as u32).collect();
        let mut m = [
            RegMatchT { rm_so: 0, rm_eo: 0 },
            RegMatchT { rm_so: 0, rm_eo: 0 },
            RegMatchT { rm_so: 0, rm_eo: 0 },
        ];
        let rc = unsafe {
            TclReExec(
                &mut re,
                subj.as_ptr(),
                subj.len(),
                core::ptr::null_mut(),
                m.len(),
                m.as_mut_ptr(),
                0,
            )
        };
        assert_eq!(rc, REG_OKAY);
        assert_eq!((m[0].rm_so, m[0].rm_eo), (0, 9)); // "user@host"
        assert_eq!((m[1].rm_so, m[1].rm_eo), (0, 4)); // "user"
        assert_eq!((m[2].rm_so, m[2].rm_eo), (5, 9)); // "host"

        // No match.
        let none: Vec<Chr> = "nope".chars().map(|c| c as u32).collect();
        let rc = unsafe {
            TclReExec(
                &mut re,
                none.as_ptr(),
                none.len(),
                core::ptr::null_mut(),
                0,
                core::ptr::null_mut(),
                0,
            )
        };
        assert_eq!(rc, REG_NOMATCH);

        unsafe { TclReFree(&mut re) };
        assert!(re.re_guts.is_null());

        // A compile error surfaces as a REG_* code with a readable message.
        let bad: Vec<Chr> = "a(".chars().map(|c| c as u32).collect();
        let rc = unsafe {
            TclReComp(
                &mut re,
                bad.as_ptr(),
                bad.len(),
                tcl_regex::defs::REG_ADVANCED,
            )
        };
        assert_ne!(rc, REG_OKAY);
        let mut buf = [0i8; 64];
        let need = unsafe { TclReError(rc, buf.as_mut_ptr(), buf.len()) };
        assert!(need > 1);
    }
}
