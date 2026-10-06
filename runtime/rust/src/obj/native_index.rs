// SPDX-License-Identifier: AGPL-3.0-or-later
//! Genuine original Index primary and retained canonical-table updater.

use super::{TclObj, TclObjType};
use tcl_core_types::NativeIndexCache;
use tcl_dialect::TclVersion;
use tcl_syntax::value::ValueError;

#[derive(Clone)]
struct Representation {
    cache: NativeIndexCache,
    version: TclVersion,
}

extern "C" fn free(object: *mut TclObj) {
    // SAFETY: exact descriptor identity owns this boxed retained table receipt.
    unsafe {
        drop(Box::from_raw(
            super::internal_rep(object) as usize as *mut Representation
        ));
    }
}
extern "C" fn duplicate(source: *mut TclObj, target: *mut TclObj) {
    // SAFETY: the exact source descriptor owns this live representation.
    let stored = unsafe { &*(super::internal_rep(source) as usize as *const Representation) };
    install_unchecked(target, stored.clone());
}
extern "C" fn update(object: *mut TclObj) {
    // SAFETY: the exact descriptor owns the retained guarded table reader.
    let stored = unsafe { &*(super::internal_rep(object) as usize as *const Representation) };
    let word = stored
        .cache
        .word()
        .expect("retained native Index table entry");
    // C9 Index uses Tcl_InitStringRep; older releases allocate even empty words.
    let canonical_empty =
        word.is_empty() && matches!(stored.version, TclVersion::V9_0 | TclVersion::V9_1);
    // SAFETY: the retained table owns the bytes throughout their buffer copy.
    unsafe { super::set_native_updater_string_rep(object, &word, canonical_empty) };
}
static TYPE: TclObjType = TclObjType {
    name: c"index".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: Some(duplicate),
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
fn install_unchecked(object: *mut TclObj, stored: Representation) {
    super::change_type(
        object,
        &TYPE,
        Box::into_raw(Box::new(stored)) as usize as u64,
    );
}
pub(crate) fn cache(object: *mut TclObj) -> Option<(NativeIndexCache, TclVersion)> {
    if !core::ptr::eq(super::obj_type_ptr(object), &TYPE) {
        return None;
    }
    // SAFETY: exact descriptor identity authenticates the boxed primary.
    let stored = unsafe { &*(super::internal_rep(object) as usize as *const Representation) };
    Some((stored.cache.clone(), stored.version))
}
pub(crate) fn install(
    object: *mut TclObj,
    cache: NativeIndexCache,
    protocol: tcl_registry::native_index_lookup::NativeIndexLookupProtocol,
) -> Result<(), ValueError> {
    if !super::has_string_rep(object) {
        return Err(ValueError::CommandProtocolUnavailable(
            "native Index resident storage",
        ));
    }
    install_unchecked(
        object,
        Representation {
            cache,
            version: protocol.version(),
        },
    );
    Ok(())
}
