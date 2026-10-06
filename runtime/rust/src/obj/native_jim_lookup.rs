// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pinned Jim lookup primaries. Private payloads retain actual original owners.

use super::{change_type, has_string_rep, internal_rep, obj_type_ptr, Owned, TclObj, TclObjType};
use tcl_runtime_api::native_compilation::NativeInterpreterIdentity;
use tcl_syntax::value::ValueError;

#[derive(Clone)]
pub(crate) struct JimCommandCache {
    pub interpreter: NativeInterpreterIdentity,
    pub epoch: u64,
    pub token: u64,
    pub namespace: Owned,
}

#[derive(Clone)]
pub(crate) struct JimVariableCache {
    pub interpreter: NativeInterpreterIdentity,
    pub frame: u64,
    pub global: bool,
    pub cell: crate::frame::WeakJimVariableCell,
}

macro_rules! descriptor {
    ($ty:ident, $record:ty, $free:ident, $dup:ident, $name:expr) => {
        extern "C" fn $free(value: *mut TclObj) {
            // SAFETY: the exact descriptor owns this boxed private payload.
            unsafe {
                drop(Box::from_raw(internal_rep(value) as usize as *mut $record));
            }
        }
        extern "C" fn $dup(source: *mut TclObj, duplicate: *mut TclObj) {
            // SAFETY: the exact source descriptor owns the live private payload.
            let record = unsafe { &*(internal_rep(source) as usize as *const $record) };
            change_type(
                duplicate,
                &$ty,
                Box::into_raw(Box::new(record.clone())) as usize as u64,
            );
        }
        pub(crate) static $ty: TclObjType = TclObjType {
            name: $name.as_ptr(),
            free_int_rep_proc: Some($free),
            dup_int_rep_proc: Some($dup),
            update_string_proc: None,
            set_from_any_proc: None,
        };
    };
}
// The two native primary types have no string updater. The command duplicate
// owns one namespace reference; the variable duplicate owns only a weak cell.
descriptor!(
    JIM_COMMAND_TYPE,
    JimCommandCache,
    command_free,
    command_dup,
    c"command"
);
descriptor!(
    JIM_VARIABLE_TYPE,
    JimVariableCache,
    variable_free,
    variable_dup,
    c"variable"
);

pub(crate) fn with_command_cache<R>(
    value: *mut TclObj,
    read: impl FnOnce(&JimCommandCache) -> R,
) -> Option<R> {
    if !core::ptr::eq(obj_type_ptr(value), &JIM_COMMAND_TYPE) {
        return None;
    }
    // SAFETY: exact descriptor validation precedes borrowing its payload.
    Some(read(unsafe {
        &*(internal_rep(value) as usize as *const JimCommandCache)
    }))
}
pub(crate) fn with_variable_cache<R>(
    value: *mut TclObj,
    read: impl FnOnce(&JimVariableCache) -> R,
) -> Option<R> {
    if !core::ptr::eq(obj_type_ptr(value), &JIM_VARIABLE_TYPE) {
        return None;
    }
    // SAFETY: exact descriptor validation precedes borrowing its payload.
    Some(read(unsafe {
        &*(internal_rep(value) as usize as *const JimVariableCache)
    }))
}
fn require_original(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
) -> Result<(), ValueError> {
    dialect
        .native_jim_lookup_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "Jim lookup primary issuer",
        ))?;
    if !has_string_rep(value) {
        return Err(ValueError::CommandProtocolUnavailable(
            "Jim lookup resident spelling",
        ));
    }
    Ok(())
}
pub(crate) fn install_command_cache(
    value: *mut TclObj,
    cache: JimCommandCache,
    dialect: tcl_registry::InvocationDialect,
) -> Result<(), ValueError> {
    require_original(value, dialect)?;
    change_type(
        value,
        &JIM_COMMAND_TYPE,
        Box::into_raw(Box::new(cache)) as usize as u64,
    );
    Ok(())
}
pub(crate) fn install_variable_cache(
    value: *mut TclObj,
    cache: JimVariableCache,
    dialect: tcl_registry::InvocationDialect,
) -> Result<(), ValueError> {
    require_original(value, dialect)?;
    change_type(
        value,
        &JIM_VARIABLE_TYPE,
        Box::into_raw(Box::new(cache)) as usize as u64,
    );
    Ok(())
}
