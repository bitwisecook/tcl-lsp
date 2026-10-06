// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual Jim Enum/ComparedString headers; neither descriptor has a string updater.
use super::{change_type, has_string_rep, internal_rep, obj_type_ptr, TclObj, TclObjType};
use tcl_core_types::NativeJimOptionCache;
use tcl_syntax::value::ValueError;
extern "C" fn free(value: *mut TclObj) {
    unsafe {
        drop(Box::from_raw(
            internal_rep(value) as usize as *mut NativeJimOptionCache
        ));
    }
}
extern "C" fn duplicate(source: *mut TclObj, copy: *mut TclObj) {
    let cache = unsafe { &*(internal_rep(source) as usize as *const NativeJimOptionCache) };
    change_type(
        copy,
        obj_type_ptr(source),
        Box::into_raw(Box::new(cache.clone())) as usize as u64,
    );
}
pub(crate) static ENUM: TclObjType = TclObjType {
    name: c"get-enum".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: Some(duplicate),
    update_string_proc: None,
    set_from_any_proc: None,
};
pub(crate) static COMPARED: TclObjType = TclObjType {
    name: c"compared-string".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: Some(duplicate),
    update_string_proc: None,
    set_from_any_proc: None,
};
pub(crate) fn cache(value: *mut TclObj) -> Option<NativeJimOptionCache> {
    let ty = obj_type_ptr(value);
    if !core::ptr::eq(ty, &ENUM) && !core::ptr::eq(ty, &COMPARED) {
        return None;
    }
    Some(unsafe { (&*(internal_rep(value) as usize as *const NativeJimOptionCache)).clone() })
}
pub(crate) fn install(
    value: *mut TclObj,
    cache: NativeJimOptionCache,
    dialect: tcl_registry::InvocationDialect,
) -> Result<(), ValueError> {
    super::check_native_liveness(value)?;
    dialect
        .native_jim_enum_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "Jim Enum primary issuer",
        ))?;
    if !has_string_rep(value) {
        return Err(ValueError::CommandProtocolUnavailable(
            "Jim Enum resident spelling",
        ));
    }
    let ty = if matches!(cache, NativeJimOptionCache::Enum { .. }) {
        &ENUM
    } else {
        &COMPARED
    };
    change_type(value, ty, Box::into_raw(Box::new(cache)) as usize as u64);
    Ok(())
}
