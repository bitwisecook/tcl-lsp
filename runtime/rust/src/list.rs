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

//! The Tcl **list** value type (T1.6) — the first user of the typed-internal-rep
//! machinery (`obj::change_type` / `free`/`dup`/`update_string` via `typePtr`).
//!
//! ## Representation decision (re-derived)
//!
//! Canonical Tcl (`tclListObj.c`) backs a list obj with a `List` struct holding
//! a **contiguous, growable `Tcl_Obj *` array**. The op-profile forces this:
//! `lindex`/`lset` want O(1) random access, `lappend` amortised O(1) append,
//! `lrange`/`lreplace` block operations, `foreach` a sequential scan — all array
//! operations, not a linked list. And the **ABI forces it too**:
//! `Tcl_ListObjGetElements` hands an extension a `Tcl_Obj **` it indexes
//! directly, so the rep must *be* (or cheaply materialise) a contiguous array.
//! So the Rust backing is [`TclList`] = `Vec<*mut TclObj>` — that contiguous
//! array. (No WASM experiment needed: the rep is forced by the ABI; the only
//! tunable, share-on-write `lappend`, is an rc==1 in-place fast path noted below.)
//!
//! The backing is hung off `Tcl_Obj.internalRep` (`design levers`: keep the
//! 24-byte header exact, per-type data behind `internalRep`). The list owns a
//! `+1` on every element; `list_free` releases them all.
//!
//! Module-level `allow(not_unsafe_ptr_arg_deref)`: like the rest of the runtime
//! these ops take live `*mut TclObj` ubiquitously (the Tcl C model); the
//! liveness invariant is upheld by the eval loop's refcount discipline, so
//! threading `unsafe` through every signature would add noise, not safety.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::obj::{self, TclObj, TclObjType};
use crate::parse::{self, ListError};

/// The list backing: the contiguous, growable element array. Each element is a
/// `*mut TclObj` the list owns a `+1` of.
struct TclList {
    elems: Vec<*mut TclObj>,
    /// Whether the value's canonical form **is** its elements (C's
    /// `TclListObjIsCanonical`): `true` for a list built from objects
    /// (`new_list_obj`/append/replace), `false` for one shimmered from a string
    /// (whose kept string rep may differ from the canonical list form). The eval
    /// loop dispatches a canonical list by element identity; a non-canonical one
    /// is re-parsed from its string ([`is_pure_list`]).
    canonical: bool,
}

/// The `list` type descriptor — free/dup/update-string procs wired to the list
/// backing (`update_string` regenerates the canonical list string form).
pub static TCL_LIST_TYPE: TclObjType = TclObjType {
    name: c"list".as_ptr(),
    free_int_rep_proc: Some(list_free),
    dup_int_rep_proc: Some(list_dup),
    update_string_proc: Some(list_update_string),
    set_from_any_proc: None,
};

// -- internalRep accessors --------------------------------------------------

unsafe fn list_ref<'a>(obj: *mut TclObj) -> &'a TclList {
    // SAFETY: `obj` has the list type, so its internalRep is a live `TclList *`.
    unsafe { &*(obj::internal_rep(obj) as usize as *const TclList) }
}

unsafe fn list_mut<'a>(obj: *mut TclObj) -> &'a mut TclList {
    // SAFETY: as `list_ref`, and the caller holds the only reference while mutating.
    unsafe { &mut *(obj::internal_rep(obj) as usize as *mut TclList) }
}

// -- type procs -------------------------------------------------------------

extern "C" fn list_free(obj: *mut TclObj) {
    // SAFETY: `obj` is a live list obj being freed; reclaim the backing box and
    // release the +1 the list held on each element.
    unsafe {
        let p = obj::internal_rep(obj) as usize as *mut TclList;
        if p.is_null() {
            return;
        }
        let list = Box::from_raw(p);
        for &e in &list.elems {
            obj::decr_ref_count(e);
        }
    }
}

extern "C" fn list_dup(src: *mut TclObj, dup: *mut TclObj) {
    // SAFETY: deep-copy the element vector and retain each element for the copy.
    unsafe {
        let src_ref = list_ref(src);
        let elems = src_ref.elems.clone();
        let canonical = src_ref.canonical;
        for &e in &elems {
            obj::incr_ref_count(e);
        }
        let boxed = Box::new(TclList { elems, canonical });
        obj::change_type(dup, &TCL_LIST_TYPE, Box::into_raw(boxed) as usize as u64);
    }
}

extern "C" fn list_update_string(obj: *mut TclObj) {
    // SAFETY: `obj` is a live list obj whose string rep needs (re)generating.
    unsafe {
        let mut buf: Vec<u8> = Vec::new();
        for (i, &e) in list_ref(obj).elems.iter().enumerate() {
            if i > 0 {
                buf.push(b' ');
            }
            append_list_element(&mut buf, &obj::bytes_of(e), i == 0);
        }
        obj::set_string_rep(obj, &buf);
    }
}

// -- shimmer ----------------------------------------------------------------

/// Ensure `obj` carries the list internal rep, parsing its string rep into
/// elements if it does not (string → list shimmer). The string rep is kept.
fn ensure_list(obj: *mut TclObj) -> Result<(), ListError> {
    if obj::obj_type_ptr(obj) == &TCL_LIST_TYPE {
        return Ok(());
    }
    let bytes = obj::bytes_of(obj);
    let elem_bytes = parse::split_list(&bytes)?;
    let mut elems = Vec::with_capacity(elem_bytes.len());
    for eb in &elem_bytes {
        let eo = obj::new_string_bytes(eb);
        // SAFETY: `eo` is fresh; the list takes the owning +1.
        unsafe { obj::incr_ref_count(eo) };
        elems.push(eo);
    }
    // Shimmered from a string: the kept string rep is the source of truth, so
    // the value is *not* canonical (its string may not match the list form).
    let boxed = Box::new(TclList {
        elems,
        canonical: false,
    });
    obj::change_type(obj, &TCL_LIST_TYPE, Box::into_raw(boxed) as usize as u64);
    Ok(())
}

// -- public ops -------------------------------------------------------------

/// `Tcl_NewListObj` — a fresh (`rc 0`) list of the given elements (each retained).
pub fn new_list_obj(elems: &[*mut TclObj]) -> *mut TclObj {
    let v: Vec<*mut TclObj> = elems.to_vec();
    for &e in &v {
        // SAFETY: each element is live; the list takes a +1.
        unsafe { obj::incr_ref_count(e) };
    }
    let boxed = Box::new(TclList {
        elems: v,
        canonical: true,
    });
    obj::alloc_typed(&TCL_LIST_TYPE, Box::into_raw(boxed) as usize as u64)
}

/// `Tcl_ListObjLength`. Shimmers a string to a list if needed.
pub fn list_length(obj: *mut TclObj) -> Result<usize, ListError> {
    ensure_list(obj)?;
    // SAFETY: `ensure_list` guarantees the list rep.
    Ok(unsafe { list_ref(obj) }.elems.len())
}

/// `Tcl_ListObjIndex` — the element at `i` (borrowed), or `None` if out of range.
pub fn list_index(obj: *mut TclObj, i: usize) -> Result<Option<*mut TclObj>, ListError> {
    ensure_list(obj)?;
    // SAFETY: list rep guaranteed.
    Ok(unsafe { list_ref(obj) }.elems.get(i).copied())
}

/// Every element (borrowed pointers), e.g. for `Tcl_ListObjGetElements` /
/// `foreach`. The returned objects are owned by the list.
/// Whether `obj` is a **pure** list — it has the list internal rep and no
/// string rep, so its canonical form is its elements (C's
/// `TclListObjIsCanonical`). Such a value can be evaluated as a single command
/// by element *identity* (preserving each element obj's TIP 280 source
/// location), instead of being stringified and re-parsed.
#[must_use]
pub fn is_pure_list(obj: *mut TclObj) -> bool {
    obj::obj_type_ptr(obj) == &TCL_LIST_TYPE && unsafe { list_ref(obj) }.canonical
}

pub fn list_elements(obj: *mut TclObj) -> Result<Vec<*mut TclObj>, ListError> {
    ensure_list(obj)?;
    // SAFETY: list rep guaranteed.
    Ok(unsafe { list_ref(obj) }.elems.clone())
}

/// `Tcl_ListObjAppendElement` — append `elem` (retained) in place and invalidate
/// the string rep.
///
/// In-place mutation is correct only when `obj` is **unshared** (`refCount <= 1`)
/// — exactly Tcl's contract for this call. The `lappend` command (T1.6b) is
/// responsible for copy-on-write when the value is shared.
pub fn list_append(obj: *mut TclObj, elem: *mut TclObj) -> Result<(), ListError> {
    ensure_list(obj)?;
    // SAFETY: list rep guaranteed; the list takes a +1 on `elem`.
    unsafe {
        obj::incr_ref_count(elem);
        list_mut(obj).elems.push(elem);
    }
    obj::invalidate_string(obj); // regenerate the string form on next read
    Ok(())
}

// -- list-element string quoting --------------------------------------------

/// Tcl list whitespace (the bytes `TclFindElement` treats as separators).
#[inline]
fn is_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// Byte-oriented, backslash-aware trim for one `Tcl_ConcatObj` part —
/// same semantics as `tcl_cmd_core::list::trim_concat_element`
/// (`TclTrimLeft`/`TclTrimRight`: the right trim keeps a trailing
/// whitespace byte escaped by an odd run of backslashes), but operating
/// directly on bytes rather than `&str`. This runtime treats Tcl strings
/// as arbitrary byte slices — not necessarily UTF-8 — so a caller
/// concatenating raw command/argument bytes (e.g. `namespace inscope`'s
/// script half, `cmd_namespace::inscope_script`) must not round-trip
/// through `String::from_utf8_lossy`, which would mangle a non-UTF-8 byte
/// into the 3-byte U+FFFD replacement sequence and change the value.
#[must_use]
pub(crate) fn trim_concat_element_bytes(s: &[u8]) -> &[u8] {
    let mut start = 0;
    while start < s.len() && is_ws(s[start]) {
        start += 1;
    }
    let mut end = s.len();
    while end > start && is_ws(s[end - 1]) {
        let backslashes = s[start..end - 1]
            .iter()
            .rev()
            .take_while(|&&b| b == b'\\')
            .count();
        if backslashes % 2 == 1 {
            break; // escaped whitespace: part of the element.
        }
        end -= 1;
    }
    &s[start..end]
}

/// Append `elem` to `buf` in canonical Tcl list-element form — a thin binding
/// of the shared `tcl_syntax::list` codec's **byte** entry point
/// (`TclScanElement` / `TclConvertElement`, tclUtil.c:1056 / :1422).
///
/// `quote_hash` (the first element of a list) forces a leading `#` to be quoted
/// so the rendered list cannot be misread as starting a comment
/// (`TCL_DONT_QUOTE_HASH` inverted). Shared with `dict` (key/value quoting).
///
/// This runtime used to carry its own port of the same four `CONVERT_*` modes
/// (issue #1439). The two agreed on every one of ~13k probed inputs, but they
/// were separate code with disjoint parity tables and no drift gate — and two
/// of the runtime port's flag settings on the trailing-`\` and `\<newline>`
/// arms already differed from C (harmless only because `require_escape`
/// dominates them). One implementation, one parity table.
pub(crate) fn append_list_element(buf: &mut Vec<u8>, elem: &[u8], quote_hash: bool) {
    tcl_syntax::list::append_list_element(buf, elem, quote_hash);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::counters;
    use crate::obj::{new_string_bytes, TclObj};

    fn leak_free(body: impl FnOnce()) {
        counters::reset();
        body();
        assert_eq!(
            counters::finalize(),
            0,
            "residual: {} objs, {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0);
    }

    fn release(obj: *mut TclObj) {
        unsafe {
            obj::incr_ref_count(obj);
            obj::decr_ref_count(obj);
        }
    }

    #[test]
    fn build_index_length() {
        leak_free(|| {
            let a = new_string_bytes(b"a");
            let b = new_string_bytes(b"b");
            let c = new_string_bytes(b"c");
            let list = new_list_obj(&[a, b, c]);
            unsafe { obj::incr_ref_count(list) };
            assert_eq!(list_length(list).unwrap(), 3);
            assert_eq!(obj::bytes_of(list_index(list, 1).unwrap().unwrap()), b"b");
            assert!(list_index(list, 9).unwrap().is_none());
            // the bare element objs (a,b,c) are owned by the list; free our
            // construction refs (they were rc 0, list took them to rc 1)
            unsafe { obj::decr_ref_count(list) }; // frees list + its 3 elements
        });
    }

    #[test]
    fn list_to_string_quotes() {
        leak_free(|| {
            let plain = new_string_bytes(b"a");
            let spaced = new_string_bytes(b"b c");
            let empty = new_string_bytes(b"");
            let list = new_list_obj(&[plain, spaced, empty]);
            unsafe { obj::incr_ref_count(list) };
            // a {b c} {}
            assert_eq!(obj::bytes_of(list), b"a {b c} {}");
            unsafe { obj::decr_ref_count(list) };
        });
    }

    #[test]
    fn string_to_list_shimmer() {
        leak_free(|| {
            let s = new_string_bytes(b"x {y z} w");
            unsafe { obj::incr_ref_count(s) };
            // reading length shimmers the string into a list
            assert_eq!(list_length(s).unwrap(), 3);
            assert_eq!(obj::bytes_of(list_index(s, 1).unwrap().unwrap()), b"y z");
            unsafe { obj::decr_ref_count(s) };
        });
    }

    #[test]
    fn append_invalidates_string_rep() {
        leak_free(|| {
            let list = new_list_obj(&[new_string_bytes(b"a")]);
            unsafe { obj::incr_ref_count(list) };
            assert_eq!(obj::bytes_of(list), b"a");
            list_append(list, new_string_bytes(b"b")).unwrap();
            // string rep regenerated to include the new element
            assert_eq!(obj::bytes_of(list), b"a b");
            assert_eq!(list_length(list).unwrap(), 2);
            unsafe { obj::decr_ref_count(list) };
        });
    }

    #[test]
    fn list_element_quoting_modes() {
        let q = |e: &[u8]| {
            let mut buf = Vec::new();
            append_list_element(&mut buf, e, true);
            buf
        };
        // escape mode (unbalanced open brace) — braces escaped too.
        assert_eq!(q(b"a{b"), b"a\\{b");
        // bare — balanced braces need no quoting.
        assert_eq!(q(b"a{b}c"), b"a{b}c");
        // brace mode — `[`/`$`/`;` force quoting but braces round-trip.
        assert_eq!(q(b"[append"), b"{[append}");
        assert_eq!(q(b"a$b"), b"{a$b}");
        assert_eq!(q(b"a;b"), b"{a;b}");
        assert_eq!(q(b"a[b]c"), b"{a[b]c}");
        // mask mode — a lone `]`/`"` escapes (braces left literal).
        assert_eq!(q(b".b]"), b".b\\]");
        assert_eq!(q(b"x]y"), b"x\\]y");
        // leading `#` (first element) → brace.
        assert_eq!(q(b"#c"), b"{#c}");
        // whitespace → brace.
        assert_eq!(q(b"a b"), b"{a b}");
        let _ = release; // silence unused in this pure test
    }
}
