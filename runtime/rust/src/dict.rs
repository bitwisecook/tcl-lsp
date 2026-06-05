//! The Tcl **dict** value type (T1.6) — an *insertion-ordered* map.
//!
//! ## Representation decision (evidence-based; experiment in
//! `experiments/dict_rep.rs`, numbers in `rust-runtime-port.md`)
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
use crate::parse::{self, ListError};

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
            crate::list::append_list_element(&mut buf, &obj::bytes_of(*k));
            buf.push(b' ');
            crate::list::append_list_element(&mut buf, &obj::bytes_of(*v));
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
    let elems = parse::split_list(&bytes).map_err(DictError::List)?;
    if elems.len() % 2 != 0 {
        return Err(DictError::OddLength);
    }
    let mut entries: Vec<(*mut TclObj, *mut TclObj)> = Vec::with_capacity(elems.len() / 2);
    let mut index = Index::default();
    let mut it = elems.into_iter();
    while let (Some(k), Some(v)) = (it.next(), it.next()) {
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

/// Why a value could not be used as a dict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictError {
    /// The string rep is not a valid list.
    List(ListError),
    /// The list has an odd number of elements (missing a value for a key).
    OddLength,
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

    #[test]
    fn odd_length_string_is_an_error() {
        leak_free(|| {
            let v = new_string_bytes(b"a 1 b"); // missing value for b
            unsafe { obj::incr_ref_count(v) };
            assert_eq!(dict_size(v), Err(DictError::OddLength));
            unsafe { obj::decr_ref_count(v) };
        });
    }
}
