// SPDX-License-Identifier: AGPL-3.0-or-later
//! Genuine C namespace-name object headers and descriptor ownership.

use super::{TclObj, TclObjType};
use tcl_runtime_api::native_namespace_name::NativeNamespaceNameCache;
use tcl_syntax::native_namespace_name::NativeNamespaceNameRecipe;
use tcl_syntax::value::ValueError;

extern "C" fn free(value: *mut TclObj) {
    // SAFETY: the exact descriptor owns this boxed cache lease.
    unsafe {
        drop(Box::from_raw(
            super::internal_rep(value) as usize as *mut NativeNamespaceNameCache
        ));
    }
}

extern "C" fn duplicate(source: *mut TclObj, target: *mut TclObj) {
    // SAFETY: the original descriptor owns the live shared namespace receipt.
    let cache =
        unsafe { &*(super::internal_rep(source) as usize as *const NativeNamespaceNameCache) };
    install_unchecked(target, cache.clone());
}

extern "C" fn update(value: *mut TclObj) {
    // SAFETY: this is the exact C8.4 descriptor's updater and live backing.
    let cache =
        unsafe { &*(super::internal_rep(value) as usize as *const NativeNamespaceNameCache) };
    let recipe = NativeNamespaceNameRecipe::for_tcl_version(cache.version());
    let bytes = cache
        .string_update_bytes(recipe)
        .expect("C8.4 namespace-name string updater");
    // SAFETY: bytes remain owned by this descriptor during their buffer copy.
    unsafe { super::set_native_updater_string_rep(value, bytes, bytes.is_empty()) };
}

static C84_TYPE: TclObjType = TclObjType {
    name: c"nsName".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: Some(duplicate),
    update_string_proc: Some(update),
    set_from_any_proc: None,
};

static C_TYPE: TclObjType = TclObjType {
    name: c"nsName".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: Some(duplicate),
    update_string_proc: None,
    set_from_any_proc: None,
};

pub(crate) fn cache(value: *mut TclObj) -> Option<NativeNamespaceNameCache> {
    let kind = super::obj_type_ptr(value);
    if !core::ptr::eq(kind, &C84_TYPE) && !core::ptr::eq(kind, &C_TYPE) {
        return None;
    }
    // SAFETY: descriptor identity authenticates this exact boxed primary.
    Some(
        unsafe { &*(super::internal_rep(value) as usize as *const NativeNamespaceNameCache) }
            .clone(),
    )
}

pub(crate) fn install(
    value: *mut TclObj,
    cache: NativeNamespaceNameCache,
    protocol: tcl_registry::native_namespace_name::NativeNamespaceNameProtocol,
) -> Result<(), ValueError> {
    if cache.version() != protocol.recipe().version() || !super::has_string_rep(value) {
        return Err(ValueError::CommandProtocolUnavailable(
            "native namespace-name primary origin and resident storage",
        ));
    }
    install_unchecked(value, cache);
    Ok(())
}

fn install_unchecked(value: *mut TclObj, cache: NativeNamespaceNameCache) {
    let descriptor = if cache.version() == tcl_dialect::TclVersion::V8_4 {
        &C84_TYPE
    } else {
        &C_TYPE
    };
    super::change_type(
        value,
        descriptor,
        Box::into_raw(Box::new(cache)) as usize as u64,
    );
}

pub(crate) fn retire(value: *mut TclObj) -> Result<(), ValueError> {
    if cache(value).is_some() {
        if !super::has_string_rep(value) {
            super::change_type(value, core::ptr::null(), 0);
            return Err(ValueError::CommandProtocolUnavailable(
                "retired namespace-name resident storage",
            ));
        }
        super::discard_native_internal_representation(value)?;
    }
    Ok(())
}

/// Tcl 8.4's namespace-current append produces an unknown-count String primary.
pub(crate) fn current_string_84(value: *mut TclObj) {
    let backing = Box::new(super::NativeStringRep {
        protocol: tcl_syntax::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4),
        count: None,
        unicode: None,
    });
    super::change_type(
        value,
        &super::NATIVE_STRING_TYPE,
        Box::into_raw(backing) as usize as u64,
    );
}
