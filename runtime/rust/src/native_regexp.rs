// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original C RegExp descriptors retain real compiled ARE artifacts.

use crate::obj::{self, TclObj, TclObjType};
use std::{cell::RefCell, rc::Rc};
use tcl_syntax::{
    native_regex::{NativeRegexpCache, NativeRegexpRecipe},
    value::ValueError,
};
type Cache = NativeRegexpCache<tcl_regex::cmd_core::CompiledRegex>;

pub(crate) fn cached(
    original: *mut TclObj,
    recipe: NativeRegexpRecipe,
    flags: u32,
) -> Result<Option<Rc<RefCell<tcl_regex::cmd_core::CompiledRegex>>>, ValueError> {
    obj::check_native_liveness(original)?;
    if obj::obj_type_ptr(original) == &JIM_REGEXP_TYPE {
        return Err(ValueError::CommandProtocolUnavailable(
            "foreign Jim regexp cache in C",
        ));
    }
    if obj::obj_type_ptr(original) != &REGEXP_TYPE {
        return Ok(None);
    }
    // SAFETY: this exact descriptor owns the live Cache allocation.
    let cache = unsafe { &*(obj::internal_rep(original) as usize as *const Cache) };
    if cache.recipe() != recipe {
        return Err(ValueError::CommandProtocolUnavailable(
            "native regexp cache origin",
        ));
    }
    Ok(cache.compiled_for(recipe, flags))
}

pub(crate) fn glob(
    original: *mut TclObj,
    recipe: NativeRegexpRecipe,
    flags: u32,
) -> Result<Option<Rc<[u8]>>, ValueError> {
    obj::check_native_liveness(original)?;
    if obj::obj_type_ptr(original) != &REGEXP_TYPE {
        return Ok(None);
    }
    // SAFETY: the exact live descriptor owns this retained compiled cache.
    Ok(unsafe { &*(obj::internal_rep(original) as usize as *const Cache) }.glob_for(recipe, flags))
}

pub(crate) fn install(
    original: *mut TclObj,
    recipe: NativeRegexpRecipe,
    flags: u32,
    compiled: Rc<RefCell<tcl_regex::cmd_core::CompiledRegex>>,
) -> Result<(), ValueError> {
    obj::check_native_liveness(original)?;
    if !obj::has_string_rep(original) {
        return Err(ValueError::CommandProtocolUnavailable(
            "native regexp counted pattern storage",
        ));
    }
    let cache = Box::new(Cache::new(
        recipe,
        flags,
        compiled,
        &obj::bytes_of(original),
    ));
    obj::change_type(original, &REGEXP_TYPE, Box::into_raw(cache) as usize as u64);
    Ok(())
}
extern "C" fn free(original: *mut TclObj) {
    // SAFETY: descriptor teardown releases exactly its own executable allocation.
    unsafe {
        drop(Box::from_raw(
            obj::internal_rep(original) as usize as *mut Cache
        ));
    }
}
extern "C" fn duplicate(original: *mut TclObj, copy: *mut TclObj) {
    // SAFETY: the retained source descriptor owns the original Cache.
    let cache = unsafe { &*(obj::internal_rep(original) as usize as *const Cache) };
    let copy_cache = Box::new(cache.clone());
    obj::change_type(
        copy,
        &REGEXP_TYPE,
        Box::into_raw(copy_cache) as usize as u64,
    );
}
pub(crate) static REGEXP_TYPE: TclObjType = TclObjType {
    name: c"regexp".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: Some(duplicate),
    update_string_proc: None,
    set_from_any_proc: None,
};

type JimCache = tcl_syntax::native_regex::JimRegexpCache<tcl_regex::cmd_core::CompiledRegex>;
pub(crate) fn cached_jim(
    original: *mut TclObj,
    flags: u32,
) -> Result<
    Option<tcl_syntax::native_regex::JimRegexpArtifact<tcl_regex::cmd_core::CompiledRegex>>,
    ValueError,
> {
    obj::check_native_liveness(original)?;
    if obj::obj_type_ptr(original) == &REGEXP_TYPE {
        return Err(ValueError::CommandProtocolUnavailable(
            "foreign C regexp cache in Jim",
        ));
    }
    if obj::obj_type_ptr(original) != &JIM_REGEXP_TYPE {
        return Ok(None);
    }
    // SAFETY: the exact descriptor owns this original live primary payload.
    unsafe { &*(obj::internal_rep(original) as usize as *const JimCache) }.compiled_for(flags)
}
pub(crate) fn install_jim(
    original: *mut TclObj,
    flags: u32,
    program: tcl_regex::cmd_core::CompiledRegex,
) -> Result<
    tcl_syntax::native_regex::JimRegexpArtifact<tcl_regex::cmd_core::CompiledRegex>,
    ValueError,
> {
    obj::check_native_liveness(original)?;
    if !obj::has_string_rep(original) {
        return Err(ValueError::CommandProtocolUnavailable(
            "Jim regexp resident CString source",
        ));
    }
    let cache = Box::new(JimCache::new(flags, program));
    let artifact = cache
        .compiled_for(flags)?
        .expect("live published Jim program");
    obj::change_type(
        original,
        &JIM_REGEXP_TYPE,
        Box::into_raw(cache) as usize as u64,
    );
    Ok(artifact)
}
extern "C" fn free_jim(original: *mut TclObj) {
    // SAFETY: only this exact primary owns this payload; retirement withdraws
    // the shared shallow program before another native primary can use it.
    unsafe {
        drop(Box::from_raw(
            obj::internal_rep(original) as usize as *mut JimCache
        ));
    }
}
extern "C" fn duplicate_jim(original: *mut TclObj, copy: *mut TclObj) {
    // SAFETY: duplicate preserves Jim's same-program shallow ownership while
    // keeping distinct payload allocations safe for checked retirement.
    let cache = unsafe { &*(obj::internal_rep(original) as usize as *const JimCache) };
    let copy_cache = Box::new(cache.duplicate());
    obj::change_type(
        copy,
        &JIM_REGEXP_TYPE,
        Box::into_raw(copy_cache) as usize as u64,
    );
}
pub(crate) static JIM_REGEXP_TYPE: TclObjType = TclObjType {
    name: c"regexp".as_ptr(),
    free_int_rep_proc: Some(free_jim),
    dup_int_rep_proc: Some(duplicate_jim),
    update_string_proc: None,
    set_from_any_proc: None,
};

pub(crate) fn cached_jim_index(original: *mut TclObj) -> Result<Option<i32>, ValueError> {
    obj::check_native_liveness(original)?;
    Ok((obj::obj_type_ptr(original) == &JIM_INDEX_TYPE)
        .then(|| obj::internal_rep(original) as u32 as i32))
}
pub(crate) fn install_jim_index(
    original: *mut TclObj,
    index: tcl_syntax::native_jim_index::JimIndex,
) {
    obj::change_type(original, &JIM_INDEX_TYPE, index.0 as u32 as u64);
}
extern "C" fn update_jim_index(original: *mut TclObj) {
    let index = tcl_syntax::native_jim_index::JimIndex(obj::internal_rep(original) as u32 as i32);
    // SAFETY: this live scalar descriptor has no child allocation or callback.
    unsafe {
        obj::set_string_rep(original, &index.string_bytes());
    }
}
static JIM_INDEX_TYPE: TclObjType = TclObjType {
    name: c"index".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update_jim_index),
    set_from_any_proc: None,
};
