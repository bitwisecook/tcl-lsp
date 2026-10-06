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

//! Dictionary values retain original key and value objects in insertion order.
//! The byte-keyed index provides lookup; the retained native hash-order state
//! describes table growth independently of that lookup index. A dictionary
//! owns one native reference to each key and value. Prepared mutation receipts
//! select COW before retaining a working owner and preserve that decision
//! throughout a nested path update.
//!
//! See `list.rs` for the pointer-argument safety contract.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};

use tcl_cmd_core::namespace::TclStringHashOrder;
use tcl_syntax::native_string::NativeStringProtocol;
use tcl_syntax::value::ValueError;

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
    hash_order: TclStringHashOrder,
    string_protocol: Cell<Option<NativeStringProtocol>>,
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

// internalRep accessors

unsafe fn dict_ref<'a>(obj: *mut TclObj) -> &'a TclDict {
    // SAFETY: `obj` has the dict type ⇒ its internalRep is a live `TclDict *`.
    unsafe { &*(obj::internal_rep(obj) as usize as *const TclDict) }
}

unsafe fn dict_mut<'a>(obj: *mut TclObj) -> &'a mut TclDict {
    // SAFETY: as `dict_ref`; caller holds the only reference while mutating.
    unsafe { &mut *(obj::internal_rep(obj) as usize as *mut TclDict) }
}

// type procs

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
        // Tcl's DupDictInternalRep creates a fresh Tcl_HashTable and reinserts
        // the live entries. Capacity history belongs to the particular table,
        // not to a COW duplicate.
        let mut hash_order = TclStringHashOrder::default();
        for (key, _) in &entries {
            hash_order.insert(&obj::bytes_of(*key));
        }
        let boxed = Box::new(TclDict {
            entries,
            index,
            hash_order,
            string_protocol: s.string_protocol.clone(),
        });
        obj::change_type(dup, &TCL_DICT_TYPE, Box::into_raw(boxed) as usize as u64);
    }
}

extern "C" fn dict_update_string(value: *mut TclObj) {
    if let Some(protocol) = native_string_protocol(value) {
        let _ = native_object_bytes(value, protocol);
    }
}

/// Selected recipe retained by this actual Dictionary backing.
pub(crate) fn native_string_protocol(value: *mut TclObj) -> Option<NativeStringProtocol> {
    (obj::obj_type_ptr(value) == &TCL_DICT_TYPE)
        .then(|| unsafe { dict_ref(value) }.string_protocol.get())
        .flatten()
}

/// Retain a selected recipe before any original-member string conversion.
pub(crate) fn seal_string_protocol(
    value: *mut TclObj,
    protocol: NativeStringProtocol,
) -> Result<(), ValueError> {
    if obj::obj_type_ptr(value) != &TCL_DICT_TYPE {
        return Ok(());
    }
    let retained = &unsafe { dict_ref(value) }.string_protocol;
    if retained.get().is_some_and(|existing| existing != protocol) {
        return Err(ValueError::CommandProtocolUnavailable(
            "native Dictionary string recipe origin",
        ));
    }
    retained.set(Some(protocol));
    Ok(())
}

/// Original member pointers; copying this inventory adds no native references.
fn compound_members(value: *mut TclObj) -> Result<Option<Vec<*mut TclObj>>, ValueError> {
    if let Some(backing) = crate::list::native_list_backing(value) {
        return Ok(Some(backing.elements()?.to_vec()));
    }
    if obj::obj_type_ptr(value) == &TCL_DICT_TYPE {
        return Ok(Some(
            unsafe { dict_ref(value) }
                .entries
                .iter()
                .flat_map(|(key, value)| [*key, *value])
                .collect(),
        ));
    }
    Ok(None)
}

// shimmer

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
    let mut hash_order = TclStringHashOrder::default();
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
            hash_order.insert(&k);
            index.insert(k, entries.len());
            entries.push((ko, vo));
        }
    }
    let boxed = Box::new(TclDict {
        entries,
        index,
        hash_order,
        string_protocol: Cell::new(None),
    });
    obj::change_type(obj, &TCL_DICT_TYPE, Box::into_raw(boxed) as usize as u64);
    Ok(())
}

/// Checked original string access through the physical object's retained updater.
pub(crate) fn native_object_bytes(
    value: *mut TclObj,
    protocol: NativeStringProtocol,
) -> Result<Vec<u8>, ValueError> {
    native_object_bytes_with_integer_formatter(value, protocol, None)
}

/// Checked actual-build formatting, propagated through original compounds.
pub(crate) fn native_object_bytes_with_integer_formatter(
    value: *mut TclObj,
    protocol: NativeStringProtocol,
    formatter: Option<&dyn tcl_platform::NativeIntegerFormatter>,
) -> Result<Vec<u8>, ValueError> {
    let mut pending = vec![(value, false)];
    while let Some((current, expanded)) = pending.pop() {
        obj::check_native_liveness(current)?;
        if obj::native_instruction_name::cache(current)
            .is_some_and(|name| protocol != NativeStringProtocol::C(name.version()))
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "foreign instruction-name updater",
            ));
        }
        if obj::has_string_rep(current) {
            continue;
        }
        if let Some(members) = compound_members(current)? {
            crate::list::seal_string_protocol(current, protocol)?;
            seal_string_protocol(current, protocol)?;
            if !expanded {
                pending.push((current, true));
                pending.extend(members.into_iter().rev().map(|member| (member, false)));
                continue;
            }
            let elements: Vec<_> = members.into_iter().map(obj::bytes_of).collect();
            let bytes =
                tcl_syntax::list_result::NativeListResultSerialization::for_string_protocol(
                    protocol,
                )
                .render(&elements);
            if obj::obj_type_ptr(current) == &crate::list::TCL_LIST_TYPE {
                crate::list::install_native_string(current, &bytes, protocol);
            } else {
                // SAFETY: the original Dictionary still owns its live backing.
                unsafe {
                    obj::set_native_updater_string_rep(current, &bytes, protocol.compound_updater_storage() == tcl_syntax::native_string::NativeStringStorageIdentity::CanonicalEmpty)
                };
            }
            continue;
        }
        if crate::native_arithseries::is_series(current) {
            drop(crate::native_arithseries::materialize_string(
                current, protocol,
            )?);
            continue;
        }
        if protocol == NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4) {
            let scalar = match obj::native_scalar_cache(current)? {
                Some(tcl_syntax::scalar_getter::NativeScalarCache::Tcl84Long(value)) => {
                    Some((tcl_platform::NativeIntegerKind::Long, value))
                }
                Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
                    tcl_syntax::number::Number::Int(value),
                )) => Some((tcl_platform::NativeIntegerKind::Wide, value)),
                _ => None,
            };
            if let Some((kind, value)) = scalar {
                if let Some(formatter) = formatter {
                    if formatter.build().version[..2] != [8, 4] {
                        return Err(ValueError::CommandProtocolUnavailable(
                            "native integer formatter build",
                        ));
                    }
                    let bytes = formatter.format(kind, value).map_err(|_| {
                        ValueError::CommandProtocolUnavailable("native integer formatter operation")
                    })?;
                    // SAFETY: current is the original live object and the native
                    // updater supplied its complete counted bytes. C84 allocates.
                    unsafe { obj::set_native_updater_string_rep(current, &bytes, false) };
                    continue;
                }
                if value == i64::MIN {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "C Tcl 8.4 minimum integer string formatter",
                    ));
                }
            }
        }
        if !obj::native_string_available(current) {
            return Err(ValueError::CommandProtocolUnavailable(
                "native object string updater",
            ));
        }
        if obj::obj_type_ptr(current) == &crate::bytearray::TCL_BYTE_ARRAY_TYPE
            && crate::bytearray::native_string_protocol(current) != Some(protocol)
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "native binary string recipe origin",
            ));
        }
        if core::ptr::eq(obj::obj_type_ptr(current), &obj::TCL_DOUBLE_TYPE) {
            let policy = match protocol {
                NativeStringProtocol::C(version) => {
                    tcl_dialect::DoubleStringPolicy::for_tcl_version(version)
                }
                NativeStringProtocol::Jim084 => tcl_dialect::DoubleStringPolicy::JimTwelve,
            };
            let format = policy
                .format(obj::double_precision(policy))
                .expect("validated native precision");
            let bytes = tcl_syntax::number::format_double_native_selected(
                obj::double_of(current),
                policy,
                format,
            );
            // SAFETY: the selected updater installs this live Double's spelling.
            unsafe { obj::set_native_updater_string_rep(current, bytes.as_bytes(), false) };
        }
        drop(obj::bytes_of(current));
        if !obj::has_string_rep(current) {
            return Err(ValueError::CommandProtocolUnavailable(
                "native object string updater",
            ));
        }
    }
    Ok(obj::bytes_of(value))
}

/// Convert using the actual native dictionary door, preserving member objects
/// and the cache reached before a parse failure.
pub(crate) fn ensure_dict_native(
    value: *mut TclObj,
    protocol: NativeStringProtocol,
) -> Result<(), ValueError> {
    seal_string_protocol(value, protocol)?;
    if obj::obj_type_ptr(value) == &TCL_DICT_TYPE {
        return Ok(());
    }
    let cached_list = obj::obj_type_ptr(value) == &crate::list::TCL_LIST_TYPE;
    if protocol.is_jim084() && cached_list && obj::is_shared(value) {
        native_object_bytes(value, protocol)?;
    }
    let members: Vec<obj::Owned> = if cached_list || protocol.is_jim084() {
        // Jim converts to List before validating an even count; C reuses an
        // existing List but does not install one on its string-parse path.
        if !cached_list {
            native_object_bytes(value, protocol)?;
        }
        crate::list::list_elements_native_checked(value, protocol)?
            .into_iter()
            .map(obj::Owned::retain)
            .collect()
    } else {
        let bytes = native_object_bytes(value, protocol)?;
        match tcl_syntax::list::split_native_list_bytes(&bytes, protocol) {
            Ok(members) => members
                .iter()
                .map(|bytes| obj::Owned::fresh(obj::new_string_bytes(bytes)))
                .collect(),
            Err(error) => {
                return Err(ValueError::DictionaryParse {
                    error,
                    source: bytes,
                });
            }
        }
    };
    if members.len() % 2 != 0 {
        return Err(ValueError::MissingDictionaryValue);
    }
    let mut entries: Vec<(*mut TclObj, *mut TclObj)> = Vec::with_capacity(members.len() / 2);
    let mut index = Index::default();
    let mut hash_order = TclStringHashOrder::default();
    let mut keys = Vec::with_capacity(members.len() / 2);
    for pair in members.chunks_exact(2) {
        keys.push(native_object_bytes(pair[0].as_ptr(), protocol)?);
    }
    let duplicate_keys = keys.iter().collect::<HashSet<_>>().len() != keys.len();
    if cached_list && !protocol.is_jim084() && duplicate_keys {
        native_object_bytes(value, protocol)?;
    }
    for (pair, key) in members.chunks_exact(2).zip(keys) {
        let (k, v) = (pair[0].as_ptr(), pair[1].as_ptr());
        if let Some(&position) = index.get(&key) {
            unsafe {
                obj::incr_ref_count(v);
                obj::decr_ref_count(entries[position].1);
            }
            entries[position].1 = v;
        } else {
            unsafe {
                obj::incr_ref_count(k);
                obj::incr_ref_count(v);
            }
            hash_order.insert(&key);
            index.insert(key, entries.len());
            entries.push((k, v));
        }
    }
    let backing = Box::new(TclDict {
        entries,
        index,
        hash_order,
        string_protocol: Cell::new(Some(protocol)),
    });
    obj::change_type(
        value,
        &TCL_DICT_TYPE,
        Box::into_raw(backing) as usize as u64,
    );
    Ok(())
}

/// Prepared native dictionary owner, retaining its original COW decision.
pub(crate) struct PreparedNativeDictionary {
    value: obj::Owned,
    protocol: NativeStringProtocol,
}

impl PreparedNativeDictionary {
    pub(crate) fn original(&self) -> *mut TclObj {
        self.value.as_ptr()
    }
    pub(crate) fn prepare(
        original: Option<*mut TclObj>,
        protocol: NativeStringProtocol,
    ) -> Result<Self, ValueError> {
        // Observe actual ownership before creating the receipt's reference.
        let value = match original {
            Some(value) if obj::is_shared(value) => obj::Owned::fresh(obj::duplicate(value)),
            Some(value) => {
                ensure_dict_native(value, protocol)?;
                obj::Owned::retain(value)
            }
            None => obj::Owned::fresh(new_dict_obj(&[])),
        };
        ensure_dict_native(value.as_ptr(), protocol)?;
        Ok(Self { value, protocol })
    }
    pub(crate) fn prepare_after_conversion(
        original: Option<*mut TclObj>,
        protocol: NativeStringProtocol,
    ) -> Result<Self, ValueError> {
        if let Some(original) = original {
            ensure_dict_native(original, protocol)?;
        }
        Self::prepare(original, protocol)
    }

    pub(crate) fn prepare_for_increment(
        original: Option<*mut TclObj>,
        protocol: NativeStringProtocol,
    ) -> Result<Self, ValueError> {
        let shared = original.is_some_and(obj::is_shared);
        let prepared = Self::prepare_after_conversion(original, protocol)?;
        if shared && protocol.tcl_version().is_some() {
            obj::invalidate_string(prepared.value.as_ptr());
        }
        Ok(prepared)
    }

    pub(crate) fn with_member<R>(
        &self,
        key: *mut TclObj,
        operation: impl FnOnce(Option<*mut TclObj>) -> R,
    ) -> Result<R, ValueError> {
        let key = native_object_bytes(key, self.protocol)?;
        let backing = unsafe { dict_ref(self.value.as_ptr()) };
        Ok(operation(
            backing.position(&key).map(|i| backing.entries[i].1),
        ))
    }
    pub(crate) fn set_member(
        &mut self,
        key: *mut TclObj,
        value: *mut TclObj,
    ) -> Result<(), ValueError> {
        native_object_bytes(key, self.protocol)?;
        dict_set(self.value.as_ptr(), key, value).expect("prepared native Dictionary cache");
        Ok(())
    }
    pub(crate) fn remove_member(&mut self, key: *mut TclObj) -> Result<bool, ValueError> {
        let key = native_object_bytes(key, self.protocol)?;
        Ok(dict_unset(self.value.as_ptr(), &key).expect("prepared native Dictionary cache"))
    }
    pub(crate) fn into_value(self) -> obj::Owned {
        self.value
    }
}

/// Actual native dictionary members, borrowed from the converted original.
pub(crate) fn native_dict_pairs(
    value: *mut TclObj,
    protocol: NativeStringProtocol,
) -> Result<Vec<(*mut TclObj, *mut TclObj)>, ValueError> {
    ensure_dict_native(value, protocol)?;
    Ok(unsafe { dict_ref(value) }.entries.clone())
}

// error

/// Why a value could not be parsed as a dict (`SetDictFromAny`/`FindElement`
/// with the "dict" type strings). Each variant carries what the C-faithful
/// message and `-errorcode` need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DictError {
    /// Odd number of elements — `missing value to go with key`
    /// (`TCL VALUE DICTIONARY`).
    MissingValue,
    /// Typed shared byte-element parser failure and the original dictionary bytes.
    Parse {
        /// Shared failure identity.
        error: tcl_syntax::list::ListError,
        /// Original dictionary string representation.
        source: Vec<u8>,
    },
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
}

impl DictError {
    /// Canonical public message bytes for this dictionary conversion failure.
    #[must_use]
    pub fn message_bytes(&self) -> Vec<u8> {
        match self {
            Self::Parse { error, source } => {
                tcl_syntax::value::dictionary_parse_message(*error, source)
            }
            Self::MissingValue => b"missing value to go with key".to_vec(),
            Self::BraceJunk(fragment) | Self::QuoteJunk(fragment) => {
                let kind = if matches!(self, Self::BraceJunk(_)) {
                    b"dict element in braces followed by \"".as_slice()
                } else {
                    b"dict element in quotes followed by \"".as_slice()
                };
                let mut message = kind.to_vec();
                message.extend_from_slice(fragment);
                message.extend_from_slice(b"\" instead of space");
                message
            }
            Self::UnmatchedBrace => b"unmatched open brace in dict".to_vec(),
            Self::UnmatchedQuote => b"unmatched open quote in dict".to_vec(),
        }
    }

    /// Checked Unicode presentation of this dictionary conversion failure.
    pub fn message(&self) -> Result<String, tcl_syntax::raw_string::UnicodeAccessError> {
        let bytes = self.message_bytes();
        String::from_utf8(bytes).map_err(|error| {
            let error = error.utf8_error();
            tcl_syntax::raw_string::UnicodeAccessError {
                valid_up_to: error.valid_up_to(),
                error_len: error.error_len(),
            }
        })
    }
}

/// Decoded (key, value) byte pairs from a dict's string rep.
type BytePairs = Vec<(Vec<u8>, Vec<u8>)>;

/// Parse `bytes` into dict (key, value) byte pairs — `SetDictFromAny`'s string
/// path, over the shared list codec. A later duplicate key is *not* deduped
/// here (the caller handles it); an odd element count is `missing value to go
/// with key`.
fn scan_dict_pairs(bytes: &[u8]) -> Result<BytePairs, DictError> {
    let elements = crate::parse::split_list(bytes).map_err(|error| DictError::Parse {
        error: error.shared(),
        source: bytes.to_vec(),
    })?;
    if elements.len() % 2 != 0 {
        return Err(DictError::MissingValue);
    }
    let mut elements = elements.into_iter();
    let mut pairs = Vec::new();
    while let Some(key) = elements.next() {
        let value = elements.next().expect("even dictionary element count");
        pairs.push((key, value));
    }
    Ok(pairs)
}

// public ops

/// `Tcl_NewDictObj` from key/value object pairs (keys + values retained). A
/// later duplicate key overwrites the earlier value, keeping the first key obj.
pub fn new_dict_obj(pairs: &[(*mut TclObj, *mut TclObj)]) -> *mut TclObj {
    new_dict_obj_with_hash_bucket_count(pairs, None)
}

/// `Tcl_NewDictObj` with a copied table's initial bucket-array size.
pub fn new_dict_obj_with_hash_bucket_count(
    pairs: &[(*mut TclObj, *mut TclObj)],
    bucket_count: Option<usize>,
) -> *mut TclObj {
    let mut entries: Vec<(*mut TclObj, *mut TclObj)> = Vec::with_capacity(pairs.len());
    let mut index = Index::default();
    let mut hash_order = TclStringHashOrder::default();
    if let Some(bucket_count) = bucket_count {
        hash_order.retain_bucket_count(bucket_count);
    }
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
            hash_order.insert(&key);
            index.insert(key, entries.len());
            entries.push((k, v));
        }
    }
    let boxed = Box::new(TclDict {
        entries,
        index,
        hash_order,
        string_protocol: Cell::new(None),
    });
    obj::alloc_typed(&TCL_DICT_TYPE, Box::into_raw(boxed) as usize as u64)
}

/// Construct original Dictionary members after checked native key conversion.
pub(crate) fn new_dict_obj_native(
    pairs: &[(*mut TclObj, *mut TclObj)],
    bucket_count: Option<usize>,
    protocol: NativeStringProtocol,
) -> Result<*mut TclObj, ValueError> {
    for &(key, _) in pairs {
        drop(native_object_bytes(key, protocol)?);
    }
    let value = new_dict_obj_with_hash_bucket_count(pairs, bucket_count);
    seal_string_protocol(value, protocol)?;
    Ok(value)
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
            d.hash_order.insert(&key);
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
                d.hash_order.remove(key);
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
pub(crate) fn native_cache_size(value: *mut TclObj) -> Option<usize> {
    if obj::obj_type_ptr(value) != &TCL_DICT_TYPE {
        return None;
    }
    // SAFETY: the exact descriptor owns this live backing.
    Some(unsafe { dict_ref(value) }.entries.len())
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

/// Retained bucket-array size of the native dictionary hash table.
pub fn dict_hash_bucket_count(obj: *mut TclObj) -> Result<usize, DictError> {
    ensure_dict(obj)?;
    // SAFETY: dict rep guaranteed.
    Ok(unsafe { dict_ref(obj) }.hash_order.bucket_count())
}

#[cfg(test)]
mod tests {
    #[test]
    fn actual_c84_updater_retains_original_long_and_wide_minimum_cache() {
        use tcl_platform::{NativeIntegerFormatter, NativeIntegerKind};
        use tcl_syntax::{number::Number, scalar_getter::NativeScalarCache};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let formatter =
            tcl_test_support::native_integer_formatter::load_pinned_c84_integer_formatter(&root)
                .expect("explicit pinned native formatter");
        let dialect = tcl_registry::InvocationDialect::of_profile(
            tcl_registry::model::ingress::resolve_environment("tcl8.4").unit_profile(),
        );
        let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4);
        for (kind, cache) in [
            (
                NativeIntegerKind::Long,
                NativeScalarCache::Tcl84Long(i64::MIN),
            ),
            (
                NativeIntegerKind::Wide,
                NativeScalarCache::Number(Number::Int(i64::MIN)),
            ),
        ] {
            let original = obj::Owned::fresh(obj::new_wide_int_obj(i64::MIN));
            obj::adopt_native_scalar_cache(
                original.as_ptr(),
                cache.clone(),
                dialect.native_scalar_getter_protocol().unwrap(),
            )
            .unwrap();
            assert!(native_object_bytes(original.as_ptr(), protocol).is_err());
            assert!(!obj::has_string_rep(original.as_ptr()));
            let expected = formatter.format(kind, i64::MIN).unwrap();
            let bytes = native_object_bytes_with_integer_formatter(
                original.as_ptr(),
                protocol,
                Some(&formatter),
            )
            .unwrap();
            assert_eq!(bytes, expected);
            assert_eq!(
                obj::native_scalar_cache(original.as_ptr()).unwrap(),
                Some(cache)
            );
            assert!(obj::has_string_rep(original.as_ptr()));
        }
    }

    fn test_dict(pairs: &[(*mut TclObj, *mut TclObj)]) -> *mut TclObj {
        let value = super::new_dict_obj(pairs);
        super::seal_string_protocol(
            value,
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5),
        )
        .unwrap();
        value
    }
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
            let d = test_dict(&[(s(b"a"), s(b"1")), (s(b"b"), s(b"2"))]);
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
            let d = test_dict(&[(s(b"x"), s(b"1"))]);
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
            let d = test_dict(&[(s(b"z"), s(b"1")), (s(b"a"), s(b"2")), (s(b"m"), s(b"3"))]);
            unsafe { obj::incr_ref_count(d) };
            // NOT sorted — insertion order (z a m), not alphabetical
            assert_eq!(bytes(d), b"z 1 a 2 m 3");
            unsafe { obj::decr_ref_count(d) };
        });
    }

    #[test]
    fn unset_preserves_order_and_frees() {
        leak_free(|| {
            let d = test_dict(&[(s(b"a"), s(b"1")), (s(b"b"), s(b"2")), (s(b"c"), s(b"3"))]);
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

    /// This runtime's native dict rep is the one binding of the
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

    /// `\<newline>` is the line-continuation escape:
    /// `TclParseBackslash` (tclParse.c(9.0.4):884-890) collapses the backslash,
    /// the newline **and the run of spaces/tabs after it** into a single space,
    /// so that whitespace is *data* inside the element and must not terminate
    /// it. The dict scan is the shared `tcl_syntax::list` codec rather than a
    /// separate `FindElement` port: a port whose backslash arm only skips two
    /// bytes would split `a\<LF> b c` into three elements where the list
    /// codec (and tclsh) see two — and the mutating dict subcommands, which
    /// reach the dict through this scan rather than through the canonical
    /// codec, would then disagree with `llength` and with `dict size`.
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
            assert_eq!(
                dict_size(v),
                Err(DictError::Parse {
                    error: tcl_syntax::list::ListError::BraceFollowedByJunk,
                    source: b"a 1 {b}c d".to_vec()
                })
            );
            unsafe { obj::decr_ref_count(v) };

            let q = new_string_bytes(b"a 1 \"b\"c d");
            unsafe { obj::incr_ref_count(q) };
            assert_eq!(
                dict_size(q),
                Err(DictError::Parse {
                    error: tcl_syntax::list::ListError::QuoteFollowedByJunk,
                    source: b"a 1 \"b\"c d".to_vec()
                })
            );
            unsafe { obj::decr_ref_count(q) };

            let ub = new_string_bytes(b"a 1 {b");
            unsafe { obj::incr_ref_count(ub) };
            assert_eq!(
                dict_size(ub),
                Err(DictError::Parse {
                    error: tcl_syntax::list::ListError::UnmatchedBrace,
                    source: b"a 1 {b".to_vec()
                })
            );
            unsafe { obj::decr_ref_count(ub) };

            let uq = new_string_bytes(b"a 1 \"b");
            unsafe { obj::incr_ref_count(uq) };
            assert_eq!(
                dict_size(uq),
                Err(DictError::Parse {
                    error: tcl_syntax::list::ListError::UnmatchedQuote,
                    source: b"a 1 \"b".to_vec()
                })
            );
            unsafe { obj::decr_ref_count(uq) };
        });
    }
}
