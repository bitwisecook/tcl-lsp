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

//! Internal representations that cache a lookup: array searches, frame levels, command names and completion codes.

use super::internal_representation::{
    change_type, discard_native_internal_representation, internal_rep, obj_type_ptr,
};
use super::scalar_types::wide_of;
use super::{
    allocation_cache, bytes_of, check_native_liveness, has_string_rep, set_allocation_cache,
    NativeArraySearchRep, NativeCommandNameRep, NativeFrameLevelRep, TclObj, TclObjType,
};
use std::rc::Rc;

/// The exact completion-keyword table's index representation.
/// Its original keyword spelling remains resident after the native conversion.
static COMPLETION_KEYWORD_TYPE: TclObjType = TclObjType {
    name: c"index".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

/// Jim's completion-code representation has no native string updater.
static JIM_RETURN_CODE_TYPE: TclObjType = TclObjType {
    name: c"return-code".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

pub(super) static NATIVE_ARRAY_SEARCH_TYPE: TclObjType = TclObjType {
    name: c"array search".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

pub(crate) fn native_array_search_cache_in(
    value: *mut TclObj,
    protocol: tcl_syntax::native_array_search::NativeArraySearchProtocol,
) -> Result<Option<tcl_core_types::NativeArraySearchCache>, tcl_syntax::value::ValueError> {
    if !core::ptr::eq(obj_type_ptr(value), &NATIVE_ARRAY_SEARCH_TYPE) {
        return Ok(None);
    }
    let stored = allocation_cache::<NativeArraySearchRep>(value).ok_or(
        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native array-search allocation origin",
        ),
    )?;
    if !protocol.accepts_cache_origin(stored.version) || !has_string_rep(value) {
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native array-search cache origin",
        ));
    }
    Ok(Some(stored.cache))
}

pub(crate) fn install_native_array_search_cache(
    value: *mut TclObj,
    cache: tcl_core_types::NativeArraySearchCache,
    protocol: tcl_syntax::native_array_search::NativeArraySearchProtocol,
) -> Result<(), tcl_syntax::value::ValueError> {
    if !protocol.caches_handle()
        || !has_string_rep(value)
        || cache.name_offset > bytes_of(value).len()
    {
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native array-search cache storage",
        ));
    }
    let stored = Rc::new(NativeArraySearchRep {
        cache,
        version: protocol.version(),
    });
    // The native descriptor has no hooks. Rust auxiliary storage belongs to
    // the actual object allocation and is copied by its NULL-hook path.
    change_type(value, &NATIVE_ARRAY_SEARCH_TYPE, 0);
    set_allocation_cache(value, stored);
    Ok(())
}

pub(super) static NATIVE_FRAME_LEVEL_TYPE: TclObjType = TclObjType {
    name: c"levelReference".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

pub(super) extern "C" fn command_name_free(value: *mut TclObj) {
    // SAFETY: this exact descriptor owns the boxed immutable cache receipt.
    unsafe {
        drop(Box::from_raw(
            internal_rep(value) as usize as *mut NativeCommandNameRep
        ));
    }
}

pub(super) extern "C" fn command_name_dup(source: *mut TclObj, duplicate: *mut TclObj) {
    // SAFETY: this exact source descriptor owns the live immutable record.
    let stored = unsafe { &*(internal_rep(source) as usize as *const NativeCommandNameRep) };
    change_type(
        duplicate,
        &NATIVE_COMMAND_NAME_TYPE,
        Box::into_raw(Box::new(stored.clone())) as usize as u64,
    );
}

pub(super) static NATIVE_COMMAND_NAME_TYPE: TclObjType = TclObjType {
    name: c"cmdName".as_ptr(),
    free_int_rep_proc: Some(command_name_free),
    dup_int_rep_proc: Some(command_name_dup),
    update_string_proc: None,
    set_from_any_proc: None,
};

fn command_name_stage_current(
    value: *mut TclObj,
    stage_check: Option<&dyn Fn() -> Result<(), tcl_syntax::value::ValueError>>,
) -> Result<(), tcl_syntax::value::ValueError> {
    if let Some(check) = stage_check {
        check()?;
        check_native_liveness(value)?;
    }
    Ok(())
}

fn retire_command_name_primary(
    value: *mut TclObj,
    stage_check: Option<&dyn Fn() -> Result<(), tcl_syntax::value::ValueError>>,
) -> Result<(), tcl_syntax::value::ValueError> {
    if stage_check.is_some() && !obj_type_ptr(value).is_null() {
        // The reached old primary retires before the selected cache is adopted.
        let retired = discard_native_internal_representation(value);
        command_name_stage_current(value, stage_check)?;
        retired?;
    }
    command_name_stage_current(value, stage_check)
}

/// Original command-name primary receipt; inspecting it grants no live cache hit.
pub(crate) fn native_command_name_cache(
    value: *mut TclObj,
) -> Option<tcl_runtime_api::native_command_name::NativeCommandNameCache> {
    if !core::ptr::eq(obj_type_ptr(value), &NATIVE_COMMAND_NAME_TYPE) {
        return None;
    }
    // SAFETY: this exact descriptor owns the live immutable cache receipt.
    unsafe { &*(internal_rep(value) as usize as *const NativeCommandNameRep) }
        .cache
        .clone()
}

/// Actual origin of C8's null command-name descriptor after a failed lookup.
pub(crate) fn native_command_name_unresolved_version(
    value: *mut TclObj,
) -> Option<tcl_dialect::TclVersion> {
    if !core::ptr::eq(obj_type_ptr(value), &NATIVE_COMMAND_NAME_TYPE) {
        return None;
    }
    // SAFETY: this exact descriptor owns the live immutable record.
    let stored = unsafe { &*(internal_rep(value) as usize as *const NativeCommandNameRep) };
    stored.cache.is_none().then_some(stored.version)
}

/// Install the selected original lookup's primary effect without changing bytes.
/// The live world separately validates command and reference identities on use.
#[cfg(test)]
pub(crate) fn install_native_command_name_cache(
    value: *mut TclObj,
    cache: tcl_runtime_api::native_command_name::NativeCommandNameCache,
    dialect: tcl_registry::InvocationDialect,
) -> Result<(), tcl_syntax::value::ValueError> {
    install_native_command_name_cache_with_stage_check(value, cache, dialect, None)
}

/// The same selected cache worker with original-operation stage checks.
pub(crate) fn install_native_command_name_cache_with_stage_check(
    value: *mut TclObj,
    cache: tcl_runtime_api::native_command_name::NativeCommandNameCache,
    dialect: tcl_registry::InvocationDialect,
    stage_check: Option<&dyn Fn() -> Result<(), tcl_syntax::value::ValueError>>,
) -> Result<(), tcl_syntax::value::ValueError> {
    command_name_stage_current(value, stage_check)?;
    use tcl_syntax::value::ValueError;
    let protocol =
        dialect
            .native_command_name_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native command-name cache",
            ))?;
    if !protocol.accepts_cache_origin(cache.version) {
        return Err(ValueError::CommandProtocolUnavailable(
            "native command-name cache origin",
        ));
    }
    if !has_string_rep(value) {
        return Err(ValueError::CommandProtocolUnavailable(
            "native command-name resident string",
        ));
    }
    retire_command_name_primary(value, stage_check)?;
    change_type(
        value,
        &NATIVE_COMMAND_NAME_TYPE,
        Box::into_raw(Box::new(NativeCommandNameRep {
            version: cache.version,
            cache: Some(cache),
        })) as usize as u64,
    );
    command_name_stage_current(value, stage_check)
}

/// Apply compile-time priming's selected early return to the original object.
#[cfg(test)]
pub(crate) fn prime_native_command_name_cache(
    value: *mut TclObj,
    incoming: tcl_runtime_api::native_command_name::NativeCommandNameCache,
    dialect: tcl_registry::InvocationDialect,
) -> Result<(), tcl_syntax::value::ValueError> {
    prime_native_command_name_cache_with_stage_check(value, incoming, dialect, None)
}

/// The same selected cache worker with original-operation stage checks.
pub(crate) fn prime_native_command_name_cache_with_stage_check(
    value: *mut TclObj,
    incoming: tcl_runtime_api::native_command_name::NativeCommandNameCache,
    dialect: tcl_registry::InvocationDialect,
    stage_check: Option<&dyn Fn() -> Result<(), tcl_syntax::value::ValueError>>,
) -> Result<(), tcl_syntax::value::ValueError> {
    command_name_stage_current(value, stage_check)?;
    use tcl_syntax::value::ValueError;
    let protocol =
        dialect
            .native_command_name_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native command-name priming",
            ))?;
    let existing = native_command_name_cache(value);
    let unresolved = native_command_name_unresolved_version(value);
    if !protocol.accepts_cache_origin(incoming.version)
        || existing
            .as_ref()
            .is_some_and(|cache| !protocol.accepts_cache_origin(cache.version))
        || unresolved.is_some_and(|version| !protocol.accepts_cache_origin(version))
    {
        return Err(ValueError::CommandProtocolUnavailable(
            "native command-name priming origin",
        ));
    }
    if protocol.preserves_primed_cache(existing.as_ref(), unresolved.is_some(), &incoming) {
        return Ok(());
    }
    install_native_command_name_cache_with_stage_check(value, incoming, dialect, stage_check)
}

/// Apply the selected failed lookup's null descriptor effect; C9 keeps its primary.
#[cfg(test)]
pub(crate) fn install_unresolved_native_command_name_cache(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
) -> Result<(), tcl_syntax::value::ValueError> {
    install_unresolved_native_command_name_cache_with_stage_check(value, dialect, None)
}

/// The same selected cache worker with original-operation stage checks.
pub(crate) fn install_unresolved_native_command_name_cache_with_stage_check(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    stage_check: Option<&dyn Fn() -> Result<(), tcl_syntax::value::ValueError>>,
) -> Result<(), tcl_syntax::value::ValueError> {
    command_name_stage_current(value, stage_check)?;
    use tcl_syntax::value::ValueError;
    let protocol =
        dialect
            .native_command_name_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native command-name lookup miss",
            ))?;
    if !protocol.installs_unresolved_on_miss() {
        return Ok(());
    }
    if !has_string_rep(value) {
        return Err(ValueError::CommandProtocolUnavailable(
            "native command-name resident string",
        ));
    }
    retire_command_name_primary(value, stage_check)?;
    change_type(
        value,
        &NATIVE_COMMAND_NAME_TYPE,
        Box::into_raw(Box::new(NativeCommandNameRep {
            version: protocol.version(),
            cache: None,
        })) as usize as u64,
    );
    command_name_stage_current(value, stage_check)
}

/// Retire only this reached command-name primary, preserving resident storage.
#[cfg(test)]
pub(crate) fn retire_native_command_name_cache(
    value: *mut TclObj,
) -> Result<(), tcl_syntax::value::ValueError> {
    if core::ptr::eq(obj_type_ptr(value), &NATIVE_COMMAND_NAME_TYPE) {
        discard_native_internal_representation(value)?;
    }
    Ok(())
}

/// Inspect the original frame cache under its authentic actual-engine issuer.
pub(crate) fn native_frame_level_cache_in(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
) -> Result<Option<tcl_registry::NativeFrameLevelCache>, tcl_syntax::value::ValueError> {
    use tcl_syntax::value::ValueError;
    check_native_liveness(value)?;
    if !core::ptr::eq(obj_type_ptr(value), &NATIVE_FRAME_LEVEL_TYPE) {
        return Ok(None);
    }
    let protocol = dialect
        .native_frame_level_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable("native frame cache"))?;
    let stored = allocation_cache::<NativeFrameLevelRep>(value).ok_or(
        ValueError::CommandProtocolUnavailable("native frame cache allocation"),
    )?;
    if protocol.tcl_version() != Some(stored.version) || !protocol.accepts_cache(stored.cache) {
        return Err(ValueError::CommandProtocolUnavailable(
            "native frame cache origin",
        ));
    }
    Ok(Some(stored.cache))
}

/// Install an actually reached frame-reference conversion, preserving spelling.
pub(crate) fn install_native_frame_level_cache(
    value: *mut TclObj,
    cache: tcl_registry::NativeFrameLevelCache,
    dialect: tcl_registry::InvocationDialect,
) -> Result<(), tcl_syntax::value::ValueError> {
    use tcl_syntax::value::ValueError;
    check_native_liveness(value)?;
    let protocol = dialect
        .native_frame_level_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable("native frame cache"))?;
    let version = protocol
        .tcl_version()
        .ok_or(ValueError::CommandProtocolUnavailable("native frame cache"))?;
    if !has_string_rep(value) || !protocol.accepts_cache(cache) {
        return Err(ValueError::CommandProtocolUnavailable(
            "native frame cache storage",
        ));
    }
    // Native levelReference has NULL hooks and retains a parsed integer, not
    // a frame pointer. The sole allocation owner copies/retires this receipt
    // independently of native descriptor hooks, just as its inline payload.
    let stored = Rc::new(NativeFrameLevelRep { cache, version });
    change_type(value, &NATIVE_FRAME_LEVEL_TYPE, 0);
    set_allocation_cache(value, stored);
    Ok(())
}

/// Inspect only authenticated completion-code descriptors, never their names.
pub(crate) fn completion_code_cache(
    value: *mut TclObj,
) -> Option<tcl_cmd_core::return_options::CompletionCodeCache> {
    use tcl_cmd_core::return_options::CompletionCodeCache;
    let kind = obj_type_ptr(value);
    if core::ptr::eq(kind, &COMPLETION_KEYWORD_TYPE) {
        Some(CompletionCodeCache::TclKeyword(wide_of(value) as i32))
    } else if core::ptr::eq(kind, &JIM_RETURN_CODE_TYPE) {
        Some(CompletionCodeCache::Jim(wide_of(value) as i32))
    } else {
        None
    }
}

/// Adopt a reached completion conversion on its original object.
pub(crate) fn adopt_completion_code_cache(
    value: *mut TclObj,
    cache: tcl_cmd_core::return_options::CompletionCodeCache,
) -> Result<(), tcl_syntax::value::ValueError> {
    use tcl_cmd_core::return_options::CompletionCodeCache;
    let (descriptor, code) = match cache {
        CompletionCodeCache::TclKeyword(code) => {
            if !has_string_rep(value) {
                return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "completion keyword resident string",
                ));
            }
            (&COMPLETION_KEYWORD_TYPE, code)
        }
        CompletionCodeCache::Jim(code) => (&JIM_RETURN_CODE_TYPE, code),
    };
    change_type(value, descriptor, i64::from(code) as u64);
    Ok(())
}
