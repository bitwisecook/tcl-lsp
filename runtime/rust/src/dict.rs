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

//! The Tcl **dict** value type (T1.6) — an *insertion-ordered* map.
//!
//! ## Representation decision (evidence-based; experiment in
//! `experiments/dict_rep.rs`)
//!
//! A dict needs **by-key** get/set (hot, incl. `dict set` build loops) **and
//! insertion-ordered** iteration (`dict keys`/`dict for`/`dict values`, Tcl
//! 8.5+). Canonical Tcl (`tclDictObj.c`) uses a hash table + an intrusive
//! insertion-order linked list. The internal rep is **free to choose** (see the
//! C-extension note below), so the choice was made by **benchmarking five
//! candidates compiled to WASM under wasmtime** (the real target). At N=65536
//! on wasm: a linear `Vec` is out (O(n²) build = 23.5 s); sort-on-iterate
//! candidates iterate 18–60× slower than a maintained-order `Vec`; and an
//! ordered `Vec` + a fixed-hash index won on **every** axis (build 15 ms,
//! lookup 14 ms, iterate 68 µs).
//!
//! So the backing is [`TclDict`]: an **insertion-ordered `Vec` of (key, value)
//! object pairs** (O(n) ordered iteration, no sort) + a **`HashMap<key-bytes,
//! index>` with a fixed FNV hasher** (O(1) by-key). Output order == `Vec` order,
//! so it is fully **deterministic** even though the hash index is unordered (we
//! never iterate the hash for output). Zero external deps. Hung off
//! `internalRep`; the dict owns a `+1` on every key **and** value object.
//!
//! ## Compatible with C extensions
//!
//! Yes. The dict C API (`Tcl_DictObjGet`/`Put`/`Remove`/`First`/`Next`) is
//! **function-mediated**: unlike `Tcl_HashTable` (embedded by value, buckets
//! walked directly — a layout contract), no extension ever observes a dict's
//! internal structure, so the rep is free to choose (the methodology's shim
//! escape-hatch). The two ABI touch-points are honoured:
//! - keys/values cross as `Tcl_Obj *` — we store the **key objects** (not just
//!   their bytes), so `Tcl_DictObjFirst` can hand back the original key object;
//!   the byte-keyed index is just for lookup (dicts compare keys by string).
//! - `Tcl_DictObjFirst`/`Next` iterate via an opaque `Tcl_DictSearch` struct the
//!   runtime fills — when that C API lands it carries a `Vec` index + an `epoch`
//!   (added then) for modify-during-iteration detection; insertion order is
//!   exactly what this `Vec` provides.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use crate::obj::{self, TclObj, TclObjType};

/// Deterministic FNV-1a hasher (no `RandomState` — reproducible across runs).
#[derive(Default)]
struct Fnv(u64);
impl Hasher for Fnv {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        let mut h = if self.0 == 0 {
            0xcbf2_9ce4_8422_2325
        } else {
            self.0
        };
        for &b in bytes {
            h ^= b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        self.0 = h;
    }
}
type Index = HashMap<Vec<u8>, usize, BuildHasherDefault<Fnv>>;

/// The dict backing: insertion-ordered `(key, value)` object pairs + a by-key
/// (string-bytes) index into them. The dict owns a `+1` on every key and value.
struct TclDict {
    entries: Vec<(*mut TclObj, *mut TclObj)>,
    index: Index,
}

impl TclDict {
    fn position(&self, key: &[u8]) -> Option<usize> {
        self.index.get(key).copied()
    }
}

/// The `dict` type descriptor.
pub static TCL_DICT_TYPE: TclObjType = TclObjType {
    name: c"dict".as_ptr(),
    free_int_rep_proc: Some(dict_free),
    dup_int_rep_proc: Some(dict_dup),
    update_string_proc: Some(dict_update_string),
    set_from_any_proc: None,
};

// -- internalRep accessors --------------------------------------------------

unsafe fn dict_ref<'a>(obj: *mut TclObj) -> &'a TclDict {
    // SAFETY: `obj` has the dict type ⇒ its internalRep is a live `TclDict *`.
    unsafe { &*(obj::internal_rep(obj) as usize as *const TclDict) }
}

unsafe fn dict_mut<'a>(obj: *mut TclObj) -> &'a mut TclDict {
    // SAFETY: as `dict_ref`; caller holds the only reference while mutating.
    unsafe { &mut *(obj::internal_rep(obj) as usize as *mut TclDict) }
}

// -- type procs -------------------------------------------------------------

extern "C" fn dict_free(obj: *mut TclObj) {
    // SAFETY: reclaim the backing box and release the +1 on every key + value.
    unsafe {
        let p = obj::internal_rep(obj) as usize as *mut TclDict;
        if p.is_null() {
            return;
        }
        let dict = Box::from_raw(p);
        for (k, v) in &dict.entries {
            obj::decr_ref_count(*k);
            obj::decr_ref_count(*v);
        }
    }
}

extern "C" fn dict_dup(src: *mut TclObj, dup: *mut TclObj) {
    // SAFETY: deep-copy the entries + index, retaining each key + value.
    unsafe {
        let s = dict_ref(src);
        let entries = s.entries.clone();
        for (k, v) in &entries {
            obj::incr_ref_count(*k);
            obj::incr_ref_count(*v);
        }
        let index = s.index.clone();
        let boxed = Box::new(TclDict { entries, index });
        obj::change_type(dup, &TCL_DICT_TYPE, Box::into_raw(boxed) as usize as u64);
    }
}

extern "C" fn dict_update_string(obj: *mut TclObj) {
    // SAFETY: regenerate `key value key value …` with list-element quoting.
    unsafe {
        let mut buf: Vec<u8> = Vec::new();
        for (i, (k, v)) in dict_ref(obj).entries.iter().enumerate() {
            if i > 0 {
                buf.push(b' ');
            }
            // Only the very first element of the flattened list quotes a
            // leading `#` (the comment-safety rule applies to list position 0).
            crate::list::append_list_element(&mut buf, &obj::bytes_of(*k), i == 0);
            buf.push(b' ');
            crate::list::append_list_element(&mut buf, &obj::bytes_of(*v), false);
        }
        obj::set_string_rep(obj, &buf);
    }
}

// -- shimmer ----------------------------------------------------------------

/// Ensure `obj` carries the dict internal rep, parsing its string rep (an
/// even-length list `k v k v …`) if it does not. The string rep is kept.
fn ensure_dict(obj: *mut TclObj) -> Result<(), DictError> {
    if obj::obj_type_ptr(obj) == &TCL_DICT_TYPE {
        return Ok(());
    }
    let bytes = obj::bytes_of(obj);
    let pairs = scan_dict_pairs(&bytes)?;
    let mut entries: Vec<(*mut TclObj, *mut TclObj)> = Vec::with_capacity(pairs.len());
    let mut index = Index::default();
    for (k, v) in pairs {
        let ko = obj::new_string_bytes(&k);
        let vo = obj::new_string_bytes(&v);
        // SAFETY: fresh key/value objects; the dict takes the owning +1 on each.
        unsafe {
            obj::incr_ref_count(ko);
            obj::incr_ref_count(vo);
        }
        // A later duplicate key overwrites the earlier value (Tcl semantics).
        if let Some(&pos) = index.get(&k) {
            // SAFETY: release the superseded value (and the now-unused new key).
            unsafe {
                obj::decr_ref_count(entries[pos].1);
                obj::decr_ref_count(ko);
            }
            entries[pos].1 = vo;
        } else {
            index.insert(k, entries.len());
            entries.push((ko, vo));
        }
    }
    let boxed = Box::new(TclDict { entries, index });
    obj::change_type(obj, &TCL_DICT_TYPE, Box::into_raw(boxed) as usize as u64);
    Ok(())
}

// -- error ------------------------------------------------------------------

/// Why a value could not be parsed as a dict (`SetDictFromAny`/`FindElement`
/// with the "dict" type strings). Each variant carries what the C-faithful
/// message and `-errorcode` need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DictError {
    /// Odd number of elements — `missing value to go with key`
    /// (`TCL VALUE DICTIONARY`).
    MissingValue,
    /// Junk after a closing brace — `dict element in braces followed by "X"
    /// instead of space` (`TCL VALUE DICTIONARY JUNK`); carries the fragment.
    BraceJunk(Vec<u8>),
    /// Junk after a closing quote — `dict element in quotes followed by "X"
    /// instead of space` (`TCL VALUE DICTIONARY JUNK`); carries the fragment.
    QuoteJunk(Vec<u8>),
    /// Unmatched `{` — `unmatched open brace in dict` (`TCL VALUE DICTIONARY BRACE`).
    UnmatchedBrace,
    /// Unmatched `"` — `unmatched open quote in dict` (`TCL VALUE DICTIONARY QUOTE`).
    UnmatchedQuote,
    /// Input bytes were not valid UTF-8 (violates the internal-rep invariant).
    NotUtf8,
}

/// The dict-worded form of a list-grammar failure. `SetDictFromAny` walks the
/// *same* `FindElement` grammar `Tcl_SplitList` does (`tclUtil.c`) and differs
/// only in the noun it prints and the `-errorcode` it sets, so the scan itself
/// comes from the one `tcl_syntax::list` owner and this maps the outcome.
fn dict_error(err: tcl_syntax::list::ListError, src: &str) -> DictError {
    use tcl_syntax::list::ListError;
    match err {
        ListError::UnmatchedBrace => DictError::UnmatchedBrace,
        ListError::UnmatchedQuote => DictError::UnmatchedQuote,
        ListError::BraceFollowedByJunk => {
            DictError::BraceJunk(tcl_syntax::list::junk_fragment(src).into_bytes())
        }
        ListError::QuoteFollowedByJunk => {
            DictError::QuoteJunk(tcl_syntax::list::junk_fragment(src).into_bytes())
        }
    }
}

/// Decoded (key, value) byte pairs from a dict's string rep.
type BytePairs = Vec<(Vec<u8>, Vec<u8>)>;

/// Parse `bytes` into dict (key, value) byte pairs — `SetDictFromAny`'s string
/// path, over the shared list codec. A later duplicate key is *not* deduped
/// here (the caller handles it); an odd element count is `missing value to go
/// with key`.
fn scan_dict_pairs(bytes: &[u8]) -> Result<BytePairs, DictError> {
    let Ok(src) = core::str::from_utf8(bytes) else {
        return Err(DictError::NotUtf8);
    };
    let decode = |el: &tcl_syntax::list::Element| -> Vec<u8> {
        let raw = &bytes[el.value.clone()];
        if el.literal {
            raw.to_vec()
        } else {
            tcl_syntax::backslash::decode_bytes(raw).into_owned()
        }
    };
    let next =
        |pos: usize| tcl_syntax::list::find_element(src, pos).map_err(|e| dict_error(e, src));
    let mut pairs = Vec::new();
    let mut pos = 0;
    while let Some(key) = next(pos)? {
        let Some(val) = next(key.next)? else {
            return Err(DictError::MissingValue);
        };
        pairs.push((decode(&key), decode(&val)));
        pos = val.next;
    }
    Ok(pairs)
}

// -- public ops -------------------------------------------------------------

/// `Tcl_NewDictObj` from key/value object pairs (keys + values retained). A
/// later duplicate key overwrites the earlier value, keeping the first key obj.
pub fn new_dict_obj(pairs: &[(*mut TclObj, *mut TclObj)]) -> *mut TclObj {
    let mut entries: Vec<(*mut TclObj, *mut TclObj)> = Vec::with_capacity(pairs.len());
    let mut index = Index::default();
    for &(k, v) in pairs {
        let key = obj::bytes_of(k);
        if let Some(&pos) = index.get(&key) {
            // SAFETY: overwrite value (retain new, release old); keep the key.
            unsafe {
                obj::incr_ref_count(v);
                obj::decr_ref_count(entries[pos].1);
            }
            entries[pos].1 = v;
        } else {
            // SAFETY: the dict takes a +1 on both key and value.
            unsafe {
                obj::incr_ref_count(k);
                obj::incr_ref_count(v);
            }
            index.insert(key, entries.len());
            entries.push((k, v));
        }
    }
    let boxed = Box::new(TclDict { entries, index });
    obj::alloc_typed(&TCL_DICT_TYPE, Box::into_raw(boxed) as usize as u64)
}

/// `Tcl_DictObjGet` — the value for key `key` (its string bytes), borrowed.
pub fn dict_get(obj: *mut TclObj, key: &[u8]) -> Result<Option<*mut TclObj>, DictError> {
    ensure_dict(obj)?;
    // SAFETY: dict rep guaranteed.
    let d = unsafe { dict_ref(obj) };
    Ok(d.position(key).map(|i| d.entries[i].1))
}

/// `Tcl_DictObjPut` — set `key_obj`→`value` in place: update an existing key's
/// value (keeping its position + original key object), or append a new pair.
/// Both retained. Invalidates the string rep. In-place mutation is for
/// **unshared** dicts (the command layer handles copy-on-write).
pub fn dict_set(
    obj: *mut TclObj,
    key_obj: *mut TclObj,
    value: *mut TclObj,
) -> Result<(), DictError> {
    ensure_dict(obj)?;
    let key = obj::bytes_of(key_obj);
    // SAFETY: dict rep guaranteed; refcount discipline per branch.
    unsafe {
        let d = dict_mut(obj);
        if let Some(pos) = d.position(&key) {
            obj::incr_ref_count(value);
            obj::decr_ref_count(d.entries[pos].1);
            d.entries[pos].1 = value; // key object unchanged
        } else {
            obj::incr_ref_count(key_obj);
            obj::incr_ref_count(value);
            d.index.insert(key, d.entries.len());
            d.entries.push((key_obj, value));
        }
    }
    obj::invalidate_string(obj);
    Ok(())
}

/// `dict exists`.
pub fn dict_exists(obj: *mut TclObj, key: &[u8]) -> Result<bool, DictError> {
    ensure_dict(obj)?;
    // SAFETY: dict rep guaranteed.
    Ok(unsafe { dict_ref(obj) }.position(key).is_some())
}

/// `Tcl_DictObjRemove` — remove `key`, preserving the order of the rest. Returns
/// whether it existed. Releases the key's and value's `+1`.
pub fn dict_unset(obj: *mut TclObj, key: &[u8]) -> Result<bool, DictError> {
    ensure_dict(obj)?;
    // SAFETY: dict rep guaranteed.
    let existed = unsafe {
        let d = dict_mut(obj);
        match d.position(key) {
            Some(pos) => {
                let (k, v) = d.entries[pos];
                obj::decr_ref_count(k);
                obj::decr_ref_count(v);
                d.entries.remove(pos); // O(n) order-preserving shift
                d.index.remove(key);
                // Entries after `pos` shifted down by one — fix their indices.
                for idx in d.index.values_mut() {
                    if *idx > pos {
                        *idx -= 1;
                    }
                }
                true
            }
            None => false,
        }
    };
    if existed {
        obj::invalidate_string(obj);
    }
    Ok(existed)
}

/// `dict size`.
pub fn dict_size(obj: *mut TclObj) -> Result<usize, DictError> {
    ensure_dict(obj)?;
    // SAFETY: dict rep guaranteed.
    Ok(unsafe { dict_ref(obj) }.entries.len())
}

/// `dict keys` — the key objects (borrowed), in insertion order.
pub fn dict_keys(obj: *mut TclObj) -> Result<Vec<*mut TclObj>, DictError> {
    ensure_dict(obj)?;
    // SAFETY: dict rep guaranteed.
    Ok(unsafe { dict_ref(obj) }
        .entries
        .iter()
        .map(|&(k, _)| k)
        .collect())
}

/// All `(key, value)` object pairs in insertion order (`dict for`/`dict
/// values`). Borrowed (owned by the dict).
pub fn dict_pairs(obj: *mut TclObj) -> Result<Vec<(*mut TclObj, *mut TclObj)>, DictError> {
    ensure_dict(obj)?;
    // SAFETY: dict rep guaranteed.
    Ok(unsafe { dict_ref(obj) }.entries.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::counters;
    use crate::obj::new_string_bytes;

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

    fn s(b: &[u8]) -> *mut TclObj {
        new_string_bytes(b)
    }

    fn bytes(obj: *mut TclObj) -> Vec<u8> {
        obj::bytes_of(obj)
    }

    #[test]
    fn build_get_size() {
        leak_free(|| {
            let d = new_dict_obj(&[(s(b"a"), s(b"1")), (s(b"b"), s(b"2"))]);
            unsafe { obj::incr_ref_count(d) };
            assert_eq!(dict_size(d).unwrap(), 2);
            assert_eq!(bytes(dict_get(d, b"a").unwrap().unwrap()), b"1");
            assert!(dict_get(d, b"missing").unwrap().is_none());
            assert!(dict_exists(d, b"b").unwrap());
            unsafe { obj::decr_ref_count(d) }; // frees dict + its keys + values
        });
    }

    #[test]
    fn set_overwrites_in_place_preserving_order() {
        leak_free(|| {
            let d = new_dict_obj(&[(s(b"x"), s(b"1"))]);
            unsafe { obj::incr_ref_count(d) };
            dict_set(d, s(b"y"), s(b"2")).unwrap(); // new key: both retained
                                                    // Overwrite keeps the existing key object and does NOT retain the
                                                    // passed key (Tcl_DictObjPut's contract), so the caller owns the
                                                    // throwaway overwrite key — manage it here.
            let kx = s(b"x");
            unsafe { obj::incr_ref_count(kx) };
            dict_set(d, kx, s(b"9")).unwrap(); // overwrite x, keep its position
            unsafe { obj::decr_ref_count(kx) }; // overwrite didn't keep our key
            let keys: Vec<Vec<u8>> = dict_keys(d).unwrap().into_iter().map(bytes).collect();
            assert_eq!(keys, vec![b"x".to_vec(), b"y".to_vec()]);
            assert_eq!(bytes(dict_get(d, b"x").unwrap().unwrap()), b"9");
            unsafe { obj::decr_ref_count(d) };
        });
    }

    #[test]
    fn insertion_order_in_string_rep() {
        leak_free(|| {
            let d = new_dict_obj(&[(s(b"z"), s(b"1")), (s(b"a"), s(b"2")), (s(b"m"), s(b"3"))]);
            unsafe { obj::incr_ref_count(d) };
            // NOT sorted — insertion order (z a m), not alphabetical
            assert_eq!(bytes(d), b"z 1 a 2 m 3");
            unsafe { obj::decr_ref_count(d) };
        });
    }

    #[test]
    fn unset_preserves_order_and_frees() {
        leak_free(|| {
            let d = new_dict_obj(&[(s(b"a"), s(b"1")), (s(b"b"), s(b"2")), (s(b"c"), s(b"3"))]);
            unsafe { obj::incr_ref_count(d) };
            assert!(dict_unset(d, b"b").unwrap());
            let keys: Vec<Vec<u8>> = dict_keys(d).unwrap().into_iter().map(bytes).collect();
            assert_eq!(keys, vec![b"a".to_vec(), b"c".to_vec()]);
            assert_eq!(bytes(dict_get(d, b"c").unwrap().unwrap()), b"3"); // index fixed up
            assert!(!dict_unset(d, b"b").unwrap()); // already gone
            unsafe { obj::decr_ref_count(d) };
        });
    }

    #[test]
    fn string_to_dict_shimmer() {
        leak_free(|| {
            let v = new_string_bytes(b"name tcl {ver 9} 0");
            unsafe { obj::incr_ref_count(v) };
            assert_eq!(dict_size(v).unwrap(), 2);
            assert_eq!(bytes(dict_get(v, b"name").unwrap().unwrap()), b"tcl");
            assert_eq!(bytes(dict_get(v, b"ver 9").unwrap().unwrap()), b"0");
            unsafe { obj::decr_ref_count(v) };
        });
    }

    /// Issue #1608 — this runtime's native dict rep is the one binding of the
    /// canonicalisation rule that is *not* a call to the shared owner
    /// [`tcl_syntax::value::canonical_dict_slots`]: it keeps a live key index
    /// across mutation, so it canonicalises incrementally (one
    /// `Tcl_DictObjPut` per insert) rather than in one walk over the elements.
    /// That makes it exactly the shape the owner cannot enforce by
    /// construction, so it is pinned by agreement instead — the same corpus
    /// the cross-crate gate `rust/tcl-vm/tests/dict_canonicalisation_parity.rs`
    /// drives through the other bindings.
    #[test]
    fn duplicate_keys_canonicalise_like_the_shared_owner() {
        leak_free(|| {
            for source in [
                "a 1 a 2",
                "a 1 b 2 a 3",
                "x 1 x 2 y 3",
                "a 1 a 2 a 3",
                "{k k} 1 {k k} 2",
                "a b b a a c",
                "1 one 01 oh-one 1 uno",
            ] {
                // The owner's answer, from the same decoded elements.
                let elements = tcl_syntax::list::split_list_lenient(source);
                let keys: Vec<&str> = elements.iter().step_by(2).map(AsRef::as_ref).collect();
                let want: Vec<(&str, &str)> =
                    tcl_syntax::value::canonical_dict_slots(keys.iter().copied())
                        .into_iter()
                        .map(|(key_slot, value_slot)| {
                            (
                                elements[key_slot * 2].as_ref(),
                                elements[value_slot * 2 + 1].as_ref(),
                            )
                        })
                        .collect();

                let v = new_string_bytes(source.as_bytes());
                unsafe { obj::incr_ref_count(v) };
                let got: Vec<(Vec<u8>, Vec<u8>)> = dict_pairs(v)
                    .unwrap()
                    .into_iter()
                    .map(|(k, val)| (bytes(k), bytes(val)))
                    .collect();
                let want: Vec<(Vec<u8>, Vec<u8>)> = want
                    .into_iter()
                    .map(|(k, val)| (k.as_bytes().to_vec(), val.as_bytes().to_vec()))
                    .collect();
                assert_eq!(
                    got, want,
                    "native dict rep diverges from the owner on {source:?}"
                );
                unsafe { obj::decr_ref_count(v) };
            }
        });
    }

    #[test]
    fn odd_length_string_is_an_error() {
        leak_free(|| {
            let v = new_string_bytes(b"a 1 b"); // missing value for b
            unsafe { obj::incr_ref_count(v) };
            assert_eq!(dict_size(v), Err(DictError::MissingValue));
            unsafe { obj::decr_ref_count(v) };
        });
    }

    /// Issue #1429 — `\<newline>` is the line-continuation escape:
    /// `TclParseBackslash` (tclParse.c(9.0.4):884-890) collapses the backslash,
    /// the newline **and the run of spaces/tabs after it** into a single space,
    /// so that whitespace is *data* inside the element and must not terminate
    /// it. The dict shimmer carried its own `FindElement` port whose backslash
    /// arm only skipped two bytes, so it split `a\<LF> b c` into three
    /// elements where the list codec (and tclsh) see two — the mutating dict
    /// subcommands, which reach the dict through this scan rather than through
    /// the canonical codec, then disagreed with `llength` and with `dict size`.
    /// The scan now *is* the shared `tcl_syntax::list` codec.
    #[test]
    fn backslash_newline_absorbs_the_following_space_run() {
        leak_free(|| {
            // Two elements per tclsh 9.0.4: `a b` and `c` ⇒ one dict pair.
            let v = new_string_bytes(b"a\\\n b c");
            unsafe { obj::incr_ref_count(v) };
            assert_eq!(dict_size(v).unwrap(), 1);
            assert_eq!(bytes(dict_get(v, b"a b").unwrap().unwrap()), b"c");
            // The string rep re-renders the collapsed key with list quoting.
            let mut pairs = dict_pairs(v).unwrap();
            assert_eq!(pairs.len(), 1);
            let (k, val) = pairs.pop().unwrap();
            assert_eq!(bytes(k), b"a b");
            assert_eq!(bytes(val), b"c");
            unsafe { obj::decr_ref_count(v) };

            // The odd-length mirror: three elements ⇒ `missing value to go
            // with key`, exactly where tclsh errors.
            let w = new_string_bytes(b"a\\\n b c d");
            unsafe { obj::incr_ref_count(w) };
            assert_eq!(dict_size(w), Err(DictError::MissingValue));
            unsafe { obj::decr_ref_count(w) };
        });
    }

    /// The dict-worded delimiter errors still carry the offending fragment
    /// after the scan moved to the shared list codec.
    #[test]
    fn delimiter_errors_keep_their_dict_wording_and_fragment() {
        leak_free(|| {
            let v = new_string_bytes(b"a 1 {b}c d");
            unsafe { obj::incr_ref_count(v) };
            assert_eq!(dict_size(v), Err(DictError::BraceJunk(b"c".to_vec())));
            unsafe { obj::decr_ref_count(v) };

            let q = new_string_bytes(b"a 1 \"b\"c d");
            unsafe { obj::incr_ref_count(q) };
            assert_eq!(dict_size(q), Err(DictError::QuoteJunk(b"c".to_vec())));
            unsafe { obj::decr_ref_count(q) };

            let ub = new_string_bytes(b"a 1 {b");
            unsafe { obj::incr_ref_count(ub) };
            assert_eq!(dict_size(ub), Err(DictError::UnmatchedBrace));
            unsafe { obj::decr_ref_count(ub) };

            let uq = new_string_bytes(b"a 1 \"b");
            unsafe { obj::incr_ref_count(uq) };
            assert_eq!(dict_size(uq), Err(DictError::UnmatchedQuote));
            unsafe { obj::decr_ref_count(uq) };
        });
    }
}
