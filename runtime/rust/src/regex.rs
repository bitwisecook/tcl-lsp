//! Safe wrapper over Tcl 9's Henry-Spencer regex engine (the **same** ARE
//! engine `tclsh` uses), linked in by `build.rs` (gated `have_regex`).
//!
//! The engine operates on `Tcl_UniChar` (32-bit codepoint) arrays, so we
//! decode the UTF-8 byte strings the runtime carries into a codepoint vector,
//! run the match, and map the codepoint offsets the engine reports back into
//! byte offsets when slicing substrings (Tcl indices are *character* indices).
//!
//! Driving model mirrors `tclRegexp.c` `RegExpExecUniChar` / `Tcl_RegExpExec`:
//! to search from a character `offset` we advance the codepoint pointer and
//! shorten the length, set `REG_NOTBOL` so `^` doesn't match at the new start,
//! and add `offset` back to the reported match positions.
//!
//! See `regex_shim/` for the C host hooks (heap, ASCII char classes, the
//! `Tcl_UniChar*` case/encode helpers) and `docs/design/runtime/tcltest-bringup.md`
//! (M3 — the regex wall).

use core::ffi::{c_char, c_int, c_long, c_void};

// `regex_t` — must mirror `generic/regex.h` exactly. On a 64-bit host:
// int(4)+pad(4) + long(8) + size_t(8) + ptr(8) + ptr(8) + ptr(8). `#[repr(C)]`
// reproduces the C field order and padding.
#[repr(C)]
struct RegexT {
    re_magic: c_int,
    re_info: c_long,
    re_nsub: usize,
    re_endp: *const c_char,
    re_guts: *mut c_void,
    re_fns: *mut c_void,
}

/// One reported (sub-)match: half-open `[so, eo)` in **character** offsets.
/// `so == NO_MATCH` means the subexpression did not participate.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RegMatch {
    pub so: usize,
    pub eo: usize,
}

/// `(size_t)-1` — the engine's "did not participate" sentinel.
pub const NO_MATCH: usize = usize::MAX;

// Compile flags (`generic/regex.h`). `REG_ADVANCED` (= EXTENDED|ADVF) is the
// Tcl ARE default; the rest are OR'd per the command's options.
pub const REG_ADVANCED: c_int = 0o3;
pub const REG_ICASE: c_int = 0o10; // -nocase
pub const REG_NOSUB: c_int = 0o20;
pub const REG_EXPANDED: c_int = 0o40; // -expanded
pub const REG_NLSTOP: c_int = 0o100; // -linestop  (. and [^] stop at \n)
pub const REG_NLANCH: c_int = 0o200; // -lineanchor (^/$ match at \n)
pub const REG_NEWLINE: c_int = 0o300; // -line (both of the above)

// Exec flags.
const REG_NOTBOL: c_int = 0o1;

// Return codes.
const REG_OKAY: c_int = 0;

// `regerror` selector for "format this status into errbuf".
extern "C" {
    fn TclReComp(re: *mut RegexT, pattern: *const i32, len: usize, flags: c_int) -> c_int;
    fn TclReExec(
        re: *mut RegexT,
        string: *const i32,
        len: usize,
        details: *mut c_void,
        nmatch: usize,
        pmatch: *mut RegMatch,
        flags: c_int,
    ) -> c_int;
    fn TclReFree(re: *mut RegexT);
    fn TclReError(code: c_int, errbuf: *mut c_char, errbuf_size: usize) -> usize;
}

/// A compiled regular expression. Owns the engine's internal state; `Drop`
/// runs `TclReFree`.
pub struct Regex {
    re: Box<RegexT>,
}

impl Regex {
    /// Compile `pattern` (UTF-8 bytes) with `REG_ADVANCED | flags`. On failure
    /// returns the Tcl error detail (already prefixed-ready) for the status.
    pub fn compile(pattern: &[u8], flags: c_int) -> Result<Regex, Vec<u8>> {
        let pat = decode_utf8(pattern).0;
        // SAFETY: zeroed `regex_t` is the documented pre-compile state.
        let mut re: Box<RegexT> = Box::new(unsafe { core::mem::zeroed() });
        // SAFETY: `pat` is a valid readable codepoint slice; `re` is writable.
        let rc = unsafe { TclReComp(re.as_mut(), pat.as_ptr(), pat.len(), REG_ADVANCED | flags) };
        if rc != REG_OKAY {
            return Err(error_detail(rc));
        }
        Ok(Regex { re })
    }

    /// Number of capturing subexpressions (so the whole match plus this many).
    pub fn nsub(&self) -> usize {
        self.re.re_nsub
    }

    /// Match `text` (decoded codepoints) starting at character `offset`.
    /// `notbol` requests `REG_NOTBOL` (the character at `offset` is not a line
    /// start, so `^` won't match there) — the caller computes it per Tcl's
    /// rule (set unless `offset` is 0 or follows a newline). Returns the match
    /// vector (index 0 = whole match, then each subexpression), with positions
    /// as **absolute** character offsets. `None` on no match. Match/error are
    /// not distinguished here: a malformed pattern is caught at `compile` time,
    /// and the engine cannot fail at exec for a successfully compiled pattern
    /// in our usage.
    pub fn exec(&mut self, text: &[i32], offset: usize, notbol: bool) -> Option<Vec<RegMatch>> {
        let nm = self.re.re_nsub + 1;
        let mut matches = vec![RegMatch { so: 0, eo: 0 }; nm];
        let off = offset.min(text.len());
        let slice = &text[off..];
        let flags = if notbol { REG_NOTBOL } else { 0 };
        // SAFETY: `slice` is readable for `slice.len()` codepoints; `matches`
        // is writable for `nm` entries; `re` was successfully compiled.
        let rc = unsafe {
            TclReExec(
                self.re.as_mut(),
                slice.as_ptr(),
                slice.len(),
                core::ptr::null_mut(),
                nm,
                matches.as_mut_ptr(),
                flags,
            )
        };
        if rc != REG_OKAY {
            return None;
        }
        for m in matches.iter_mut() {
            if m.so != NO_MATCH {
                m.so += off;
                m.eo += off;
            }
        }
        Some(matches)
    }
}

impl Drop for Regex {
    fn drop(&mut self) {
        // SAFETY: `re` was filled by a successful `TclReComp`; `TclReFree`
        // tears down its internal allocations. (The wasm-only call_indirect
        // hazard the Zig oracle documented does not apply to the native link.)
        unsafe { TclReFree(self.re.as_mut()) };
    }
}

/// Format an engine status code into the Tcl error detail text.
fn error_detail(code: c_int) -> Vec<u8> {
    let mut buf = [0u8; 128];
    // SAFETY: `buf` is writable for its length; `TclReError` writes a
    // NUL-terminated message and never reads `buf`.
    let n = unsafe { TclReError(code, buf.as_mut_ptr() as *mut c_char, buf.len()) };
    let end = n.min(buf.len()).saturating_sub(1); // drop the NUL
    buf[..end].to_vec()
}

/// Decode UTF-8 `bytes` into codepoints plus a parallel byte-offset table.
/// The returned `offsets` has length `codepoints.len() + 1`: `offsets[i]` is
/// the byte index where codepoint `i` starts, and the final entry is
/// `bytes.len()` — so a character range `[so, eo)` slices the original bytes as
/// `bytes[offsets[so]..offsets[eo]]`. Invalid sequences decode to U+FFFD,
/// matching the C shim's `Tcl_UniCharToUtfDString`.
pub fn decode_utf8(bytes: &[u8]) -> (Vec<i32>, Vec<usize>) {
    let mut cps = Vec::with_capacity(bytes.len());
    let mut offs = Vec::with_capacity(bytes.len() + 1);
    let mut i = 0;
    while i < bytes.len() {
        offs.push(i);
        let b0 = bytes[i];
        let (cp, n) = if b0 < 0x80 {
            (b0 as u32, 1)
        } else if b0 & 0xE0 == 0xC0 {
            (u32::from(b0 & 0x1F), 2)
        } else if b0 & 0xF0 == 0xE0 {
            (u32::from(b0 & 0x0F), 3)
        } else if b0 & 0xF8 == 0xF0 {
            (u32::from(b0 & 0x07), 4)
        } else {
            (0xFFFD, 1)
        };
        let mut cp = cp;
        let mut taken = 1;
        if n > 1 && i + n <= bytes.len() {
            let mut ok = true;
            for j in 1..n {
                let b = bytes[i + j];
                if b & 0xC0 != 0x80 {
                    ok = false;
                    break;
                }
                cp = (cp << 6) | u32::from(b & 0x3F);
                taken += 1;
            }
            if !ok || taken != n {
                cp = 0xFFFD;
                taken = 1;
            }
        } else if n > 1 {
            cp = 0xFFFD;
            taken = 1;
        }
        cps.push(cp as i32);
        i += taken;
    }
    offs.push(bytes.len());
    (cps, offs)
}
