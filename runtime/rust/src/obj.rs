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

//! The `Tcl_Obj` value model + refcount discipline.
//!
//! Every allocation is balanced by a free driven by the refcount reaching
//! zero, and the alloc/free counters (`crate::counters`) prove it.
//!
//! ## Layout — the C-extension ABI requires `#[repr(C)]`
//!
//! `docs/design/runtime/c-extension-abi.md` §4.2 fixes the layout: extensions
//! read `objPtr->refCount` / `objPtr->bytes` directly through `tcl.h` macros,
//! so the struct must be `#[repr(C)]` with the exact field order
//! `tcl.h` declares — `{ refCount, bytes, length, typePtr, internalRep }`. On
//! `wasm32` that is `{ isize, ptr, isize, ptr, <8-byte union> }`. This is the
//! single canonical obj model for the runtime (the codegen's 32-byte
//! handle/tagged-immediate layout is a separate codegen detail; the runtime
//! serves the same `tcl_*`/`obj_*` codegen primitives over the
//! ABI-faithful struct, with the immediate/inline-string optimisations layered
//! on later).
//!
//! ## Refcount semantics — faithful to `tclObj.c`
//!
//! Constructors return an object at **refCount 0** (the `fresh_zero` C-API
//! convention, `c-api-ownership-contract.md`): the caller owns nothing until
//! it `Tcl_IncrRefCount`s or hands the object to a consumer that retains it.
//! `Tcl_DecrRefCount` frees immediately when the count reaches zero (Tcl's
//! `TclFreeObj`). The runtime-internal *deferred* free queue
//! (`tcl_obj_drain_pending`, for the eval-loop aliasing case in
//! `memory-management.md` MM-B.6) lands with the eval loop; the C-API
//! boundary is immediate, which is what `Tcl_DecrRefCount` documents.

mod jim_object_pool;
pub(crate) use jim_object_pool::{
    validate_allocator as validate_jim_allocator, JimAllocationScope, JimCommandNameObservation,
    JimPool,
};
mod native_jim_lookup;
pub(crate) use native_jim_lookup::{
    install_command_cache_with_stage_check as install_jim_command_cache_with_stage_check,
    install_variable_cache_with_stage_check as install_jim_variable_cache_with_stage_check,
    with_command_cache as with_jim_command_cache, with_variable_cache as with_jim_variable_cache,
};
pub(crate) use native_jim_lookup::{JimCommandCache, JimVariableCache};

use core::ffi::{c_char, c_void};
use std::alloc::{alloc, dealloc, Layout};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::Rc,
};

use crate::counters;

pub(crate) mod native_end_offset;
pub(crate) mod native_index;
pub(crate) mod native_instruction_name;
pub(crate) mod native_lambda_expression;
pub(crate) mod native_namespace_name;
pub(crate) mod native_variable_name;

/// `Tcl_Size` — `ptrdiff_t` in `tcl.h` (Tcl 9 width-agnostic size type).
pub type TclSize = isize;
/// `Tcl_WideInt` — always 64-bit.
pub type TclWideInt = i64;

/// `Tcl_ObjType` — the registered type descriptor (`tcl.h`). The four procs are
/// the **shimmer keystone** (value-kinds): the runtime
/// dispatches free / dup / string-generation through `typePtr`, so built-in
/// types (int, double, list, …) and extension-registered custom types share one
/// mechanism. Signatures match `tcl.h` so an extension's
/// `Tcl_ObjType` slots in unchanged.
pub type FreeInternalRepProc = extern "C" fn(*mut TclObj);
pub type DupInternalRepProc = extern "C" fn(*mut TclObj, *mut TclObj);
pub type UpdateStringProc = extern "C" fn(*mut TclObj);
pub type SetFromAnyProc = extern "C" fn(*mut c_void, *mut TclObj) -> core::ffi::c_int;

#[repr(C)]
pub struct TclObjType {
    pub name: *const c_char,
    pub free_int_rep_proc: Option<FreeInternalRepProc>,
    pub dup_int_rep_proc: Option<DupInternalRepProc>,
    pub update_string_proc: Option<UpdateStringProc>,
    pub set_from_any_proc: Option<SetFromAnyProc>,
}

// SAFETY: these are immortal `'static` descriptors; the raw `name` pointer is
// to a `'static` NUL-terminated byte string. They are read-only and shared.
unsafe impl Sync for TclObjType {}

/// Register an actual legacy five-field descriptor without abstract List methods.
/// Its ordinary String/List conversion remains effectful and independently checked.
/// Registration grants no stock-hook closure, successful conversion or List cache.
pub fn register_legacy_extension_type(descriptor: &'static TclObjType) {
    LEGACY_EXTENSION_TYPES.with(|types| {
        types
            .borrow_mut()
            .insert(descriptor as *const TclObjType as usize);
    });
}

/// Jim's native string internal representation retains the prescribed character
/// count independently of byte length. Bytes always exist for this type.
pub(crate) static JIM_STRING_TYPE: TclObjType = TclObjType {
    name: c"string".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

/// Native C string representation. Count is computed under the current selected
/// model, so repinning an interpreter cannot reuse a different model's cache.
#[derive(Clone)]
struct NativeStringRep {
    protocol: tcl_syntax::native_string::NativeStringProtocol,
    count: Option<usize>,
    unicode: Option<std::rc::Rc<[u32]>>,
}

type ObjectScriptLocation =
    tcl_runtime_api::script_source_location::ScriptSourceLocation<Option<std::rc::Rc<[u8]>>>;

thread_local! {
    static LEGACY_EXTENSION_TYPES: std::cell::RefCell<std::collections::HashSet<usize>> =
        std::cell::RefCell::new(std::collections::HashSet::new());
    // Metadata follows the value object, without changing the public C header.
    // Retiring the header removes its entry before its address can be reused.
    static SCRIPT_LOCATIONS: std::cell::RefCell<std::collections::HashMap<usize, ObjectScriptLocation>> = std::cell::RefCell::new(std::collections::HashMap::new());
    // C's UpdateStringProc has no interpreter argument. Like Tcl_PrintDouble,
    // the selected engine owns the thread's string conversion and shared
    // precision. Child interpreters inherit the existing precision entry.
    static DOUBLE_STRING_POLICY: std::cell::Cell<tcl_dialect::DoubleStringPolicy> =
        const { std::cell::Cell::new(tcl_dialect::DoubleStringPolicy::Shortest) };
    static DOUBLE_PRECISIONS: std::cell::RefCell<std::collections::HashMap<tcl_dialect::DoubleStringPolicy, u8>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

mod duplication;
mod internal_representation;
mod lifetime_handles;
mod lookup_caches;
mod scalar_types;
mod string_representations;

pub(crate) use duplication::{
    duplicate, duplicate_into, duplicate_into_with_stage_check, duplicate_with_stage_check,
};
pub(crate) use internal_representation::{
    alloc_typed, change_type, change_type_with_stage_check, discard_native_internal_representation,
    discard_native_internal_representation_with_stage_check, internal_rep, invalidate_string,
    obj_type_ptr, reset_native_c_result, set_native_updater_string_rep, set_string_rep,
};
pub(crate) use lifetime_handles::{NativeObjectLifetime, RetiringResultHeader};
pub use lifetime_handles::{NativePrimaryReceipt, ObjectPointer, Owned, ProcedureObject};
pub(crate) use lookup_caches::{
    adopt_completion_code_cache, completion_code_cache, install_native_array_search_cache,
    install_native_command_name_cache_with_stage_check, install_native_frame_level_cache,
    install_unresolved_native_command_name_cache_with_stage_check, native_array_search_cache_in,
    native_command_name_cache, native_frame_level_cache_in,
    prime_native_command_name_cache_with_stage_check,
};
#[cfg(test)]
pub(crate) use lookup_caches::{
    install_native_command_name_cache, install_unresolved_native_command_name_cache,
    native_command_name_unresolved_version, prime_native_command_name_cache,
    retire_native_command_name_cache,
};
use lookup_caches::{NATIVE_COMMAND_NAME_TYPE, NATIVE_FRAME_LEVEL_TYPE};
#[cfg(have_tommath)]
pub(crate) use scalar_types::may_cache_parsed_rep;
use scalar_types::native_word_boolean_origin;
pub(crate) use scalar_types::{
    adopt_native_numeric_representation, adopt_native_scalar_cache,
    adopt_native_scalar_cache_with_stage_check, cache_double_rep, cache_wide_rep, double_of,
    double_precision, install_double_string_policy, native_scalar_cache,
    native_scalar_descriptor_origin, restore_jim_coerced_integer, set_double_precision,
    stock_list_input_class, wide_of, NativeNumericRepresentation, JIM_COERCED_DOUBLE_TYPE,
    TCL_LONG84_TYPE, TCL_WIDE84_TYPE,
};
pub use scalar_types::{
    new_boolean_obj, new_double_obj, new_wide_int_obj, TCL_DOUBLE_TYPE, TCL_INT_TYPE,
};
#[cfg(have_tommath)]
pub(crate) use string_representations::jim_character_count;
use string_representations::NATIVE_STRING_TYPE;
pub(crate) use string_representations::{
    is_plain_string, jim_string_count, native_c_string_storage_checked, native_character_count,
    native_character_count_checked, native_string_available, native_unicode_units,
    native_unicode_units_checked, new_native_unicode_obj, retain_jim_string_count,
    retain_jim_string_representation, retain_native_string_representation,
    set_native_append_string, set_native_append_string_checked, string_append_inplace,
    trim_jim_string_bytes,
};

/// Retain the actual original literal's byte-preserving file and creation line.
pub(crate) fn retain_script_location(obj: *mut TclObj, file: Option<std::rc::Rc<[u8]>>, line: u32) {
    SCRIPT_LOCATIONS.with(|locations| {
        locations
            .borrow_mut()
            .insert(obj as usize, ObjectScriptLocation { file, line })
    });
}

/// Source metadata of the same live value object, never a lookup by its bytes.
pub(crate) fn script_location(obj: *mut TclObj) -> Option<(Option<std::rc::Rc<[u8]>>, u32)> {
    SCRIPT_LOCATIONS.with(|locations| {
        locations
            .borrow()
            .get(&(obj as usize))
            .map(|location| (location.file.clone(), location.line))
    })
}

fn revoke_script_location(obj: *mut TclObj) {
    // The object is already retired; an unavailable TLS map owns no live metadata.
    let _ = SCRIPT_LOCATIONS.try_with(|locations| locations.borrow_mut().remove(&(obj as usize)));
}

/// `Tcl_Obj` — ABI-faithful to `tcl.h` (§4.2). `internal_rep` models the
/// 8-byte `Tcl_ObjInternalRep` union; core-API extensions never touch its
/// variants, so we keep the raw 8 bytes and reinterpret for `wide`/`double`.
#[repr(C)]
pub struct TclObj {
    pub ref_count: TclSize,
    pub bytes: *mut c_char,
    pub length: TclSize,
    pub type_ptr: *const TclObjType,
    pub internal_rep: u64,
}

// The exported header remains exactly TclObj. Allocation bookkeeping follows
// it and is never part of the extension ABI or a native cache descriptor.
#[repr(C)]
struct ObjectAllocation {
    object: TclObj,
    auxiliary: ObjectAuxiliary,
}
struct ObjectAuxiliary {
    lifetime_pins: Cell<usize>,
    incarnation: RefCell<tcl_runtime_api::native_object_incarnation::NativeObjectIncarnation>,
    jim_allocation: RefCell<Option<jim_object_pool::JimAllocation>>,
    jim_pool: RefCell<Option<std::rc::Weak<JimPool>>>,
    live: Cell<bool>,
    retiring: Cell<bool>,
    primary_epoch: Cell<Option<u64>>,
    cache: RefCell<Option<Rc<dyn Any>>>,
    scalar_context: RefCell<
        Option<
            Result<
                Rc<crate::interp::native_scalar_context::NativeScalarObjectContext>,
                tcl_syntax::raw_string::NativeValueAccessRefusal,
            >,
        >,
    >,
}

fn auxiliary(value: *mut TclObj) -> &'static ObjectAuxiliary {
    // SAFETY: all runtime object headers originate at this sole allocator and
    // TclObj is the first field. The caller must retain a native or lifetime owner.
    unsafe { &(*(value.cast::<ObjectAllocation>())).auxiliary }
}

pub(crate) fn allocation_cache<T: Any>(value: *mut TclObj) -> Option<Rc<T>> {
    auxiliary(value)
        .cache
        .borrow()
        .as_ref()?
        .clone()
        .downcast()
        .ok()
}

pub(crate) fn set_allocation_cache<T: Any>(value: *mut TclObj, cache: Rc<T>) {
    assert!(
        allocation_is_live(value),
        "cache installation on a retired object"
    );
    let retired = auxiliary(value).cache.replace(Some(cache));
    drop(retired);
}

pub(crate) fn scalar_object_context(
    value: *mut TclObj,
) -> Result<
    Option<Rc<crate::interp::native_scalar_context::NativeScalarObjectContext>>,
    tcl_syntax::value::ValueError,
> {
    check_native_liveness(value)?;
    auxiliary(value)
        .scalar_context
        .borrow()
        .clone()
        .transpose()
        .map_err(Into::into)
}

pub(crate) fn validate_scalar_object_context(
    value: *mut TclObj,
    incoming: &crate::interp::native_scalar_context::NativeScalarObjectContext,
) -> Result<(), tcl_syntax::value::ValueError> {
    if let Some(retained) = scalar_object_context(value)? {
        if !retained.same_issuer(incoming) {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "foreign or stale native scalar object issuer",
            ));
        }
    }
    Ok(())
}

pub(crate) fn bind_scalar_object_context(
    value: *mut TclObj,
    incoming: Rc<crate::interp::native_scalar_context::NativeScalarObjectContext>,
) -> Result<(), tcl_syntax::value::ValueError> {
    validate_scalar_object_context(value, &incoming)?;
    if let Some(cache) = native_scalar_cache(value)? {
        incoming.validate_bound_cache(&cache)?;
    }
    if scalar_object_context(value)?.is_some() {
        return Ok(());
    }
    let retired = auxiliary(value).scalar_context.replace(Some(Ok(incoming)));
    drop(retired);
    Ok(())
}

fn copy_scalar_object_context(original: *mut TclObj, duplicate: *mut TclObj) {
    let incoming = auxiliary(original).scalar_context.borrow().clone();
    let retired = auxiliary(duplicate).scalar_context.replace(incoming);
    drop(retired);
}

fn replace_scalar_object_context(receiver: *mut TclObj, source: *mut TclObj) {
    let retained = auxiliary(receiver).scalar_context.borrow().clone();
    let incoming = auxiliary(source).scalar_context.borrow().clone();
    replace_scalar_object_context_from(receiver, retained, incoming);
}

type ScalarContextReceipt = Option<
    Result<
        Rc<crate::interp::native_scalar_context::NativeScalarObjectContext>,
        tcl_syntax::raw_string::NativeValueAccessRefusal,
    >,
>;
fn replace_scalar_object_context_from(
    receiver: *mut TclObj,
    retained: ScalarContextReceipt,
    incoming: ScalarContextReceipt,
) {
    let selected = match (retained, incoming) {
        (Some(Ok(retained)), Some(Ok(incoming))) if !retained.same_issuer(&incoming) => Some(Err(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "foreign native scalar replacement issuer",
            ),
        )),
        (Some(Err(cause)), _) | (_, Some(Err(cause))) => Some(Err(cause)),
        (Some(retained), _) => Some(retained),
        (None, incoming) => incoming,
    };
    let retired = auxiliary(receiver).scalar_context.replace(selected);
    drop(retired);
}

pub(crate) fn allocation_is_live(value: *mut TclObj) -> bool {
    auxiliary(value).live.get()
}

/// Validate a retained allocation before reading or changing its native header.
pub(crate) fn check_native_liveness(
    value: *mut TclObj,
) -> Result<(), tcl_syntax::value::ValueError> {
    if value.is_null() || !allocation_is_live(value) {
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "retired native object",
        ));
    }
    Ok(())
}

// The header's `Tcl_Obj` (`include/tcl.h`): the fields in this order at
// consecutive word offsets, then the internal representation, which makes the
// object 24 bytes on wasm32. An extension compiled against the header reads and
// writes `refCount` and reads `bytes` and `length` at these offsets, so the build
// fails if the struct stops matching them.
const _: () = {
    use core::mem::{offset_of, size_of};
    const WORD: usize = size_of::<usize>();
    assert!(offset_of!(TclObj, ref_count) == 0);
    assert!(offset_of!(TclObj, bytes) == WORD);
    assert!(offset_of!(TclObj, length) == 2 * WORD);
    assert!(offset_of!(TclObj, type_ptr) == 3 * WORD);
    assert!(offset_of!(TclObj, internal_rep) == 4 * WORD);
    #[cfg(target_pointer_width = "32")]
    assert!(size_of::<TclObj>() == 24);
};

impl TclObj {
    #[inline]
    fn wide(&self) -> TclWideInt {
        self.internal_rep as i64
    }

    #[inline]
    fn double(&self) -> f64 {
        f64::from_bits(self.internal_rep)
    }
}

// Allocation — the single allocator (§4.4). Natively this is the Rust global
// allocator; on wasm it becomes the one shared-memory allocator. Every
// header and every owned string buffer is counted so the leak gate can prove
// balance.

fn obj_layout() -> Layout {
    Layout::new::<ObjectAllocation>()
}

/// Allocate a zeroed `TclObj` header at refCount 0 with no string rep.
fn allocate_object_memory() -> *mut TclObj {
    // SAFETY: `obj_layout()` is non-zero-sized and well-formed; we initialise
    // every field before returning, and the pointer is freed exactly once by
    // `free_obj`.
    unsafe {
        let p = alloc(obj_layout()) as *mut TclObj;
        if p.is_null() {
            counters::oom_set();
            return core::ptr::null_mut();
        }
        (*p).ref_count = 0;
        (*p).bytes = core::ptr::null_mut();
        (*p).length = 0;
        (*p).type_ptr = core::ptr::null();
        (*p).internal_rep = 0;
        core::ptr::write(
            core::ptr::addr_of_mut!((*p.cast::<ObjectAllocation>()).auxiliary),
            ObjectAuxiliary {
                lifetime_pins: Cell::new(0),
                incarnation: RefCell::new(
                    tcl_runtime_api::native_object_incarnation::NativeObjectIncarnation::fresh(),
                ),
                jim_allocation: RefCell::new(None),
                jim_pool: RefCell::new(None),
                live: Cell::new(true),
                retiring: Cell::new(false),
                primary_epoch: Cell::new(Some(0)),
                cache: RefCell::new(None),
                scalar_context: RefCell::new(None),
            },
        );
        p
    }
}

fn obj_alloc() -> *mut TclObj {
    let pointer = match jim_object_pool::allocate() {
        Ok(Some((allocation, pool))) => {
            let pointer = jim_object_pool::pointer(&allocation);
            let state = auxiliary(pointer);
            *state.incarnation.borrow_mut() = allocation.incarnation().clone();
            state.live.set(true);
            state.retiring.set(false);
            state.primary_epoch.set(
                state
                    .primary_epoch
                    .get()
                    .and_then(|epoch| epoch.checked_add(1)),
            );
            *state.jim_allocation.borrow_mut() = Some(allocation);
            *state.jim_pool.borrow_mut() = Some(pool);
            // SAFETY: a free pool slot has retired all former Native resources.
            unsafe {
                (*pointer).ref_count = 0;
                (*pointer).bytes = core::ptr::null_mut();
                (*pointer).length = 0;
                (*pointer).type_ptr = core::ptr::null();
                (*pointer).internal_rep = 0;
            }
            pointer
        }
        Ok(None) => allocate_object_memory(),
        Err(()) => return core::ptr::null_mut(),
    };
    if !pointer.is_null() {
        counters::obj_alloced();
    }
    pointer
}

/// Free a `TclObj` and its owned string buffer (if any). `TclFreeObj`: what
/// the header's `Tcl_DecrRefCount` macro calls once it has lowered the count of
/// the last reference, whatever count that left.
///
/// # Safety
/// `obj` must be a live header previously returned by `obj_alloc` and not yet
/// freed; no other reference may use it after this returns.
pub unsafe fn free_obj(obj: *mut TclObj) {
    if obj.is_null() {
        return;
    }
    let state = auxiliary(obj);
    if !state.live.replace(false) {
        counters::double_free();
        return;
    }
    state.retiring.set(true);
    let retired_cache = state.cache.take();
    let retired_scalar_context = state.scalar_context.take();
    revoke_script_location(obj);
    crate::native_source::forget_context(obj);
    // SAFETY: caller guarantees `obj` is a live, uniquely-owned header.
    unsafe {
        // Dispatch the type's free-internal-rep proc (releases list elements,
        // frees the list/dict backing, runs an extension's freeIntRepProc, …).
        let tp = (*obj).type_ptr;
        if !tp.is_null() {
            if let Some(free) = (*tp).free_int_rep_proc {
                free(obj);
            }
        }
        free_string_buffer(obj);
        (*obj).type_ptr = core::ptr::null();
        (*obj).internal_rep = 0;
    }
    drop(retired_cache);
    drop(retired_scalar_context);
    counters::obj_freed();
    state.retiring.set(false);
    let allocation = state.jim_allocation.borrow_mut().take();
    let pool = state
        .jim_pool
        .borrow_mut()
        .take()
        .and_then(|pool| pool.upgrade());
    if let Some(allocation) = allocation {
        if let Some(pool) = pool {
            let _ = pool.release(allocation);
        } else {
            drop(allocation);
        }
        return;
    }
    if state.lifetime_pins.get() == 0 {
        // SAFETY: no remaining native or lifetime owner can use this allocation.
        unsafe { deallocate_object(obj) };
    }
}

unsafe fn deallocate_object(obj: *mut TclObj) {
    // SAFETY: caller owns the final allocation lease; its header has already
    // released all native resources. Drop only the initialized auxiliary state.
    unsafe {
        core::ptr::drop_in_place(core::ptr::addr_of_mut!(
            (*obj.cast::<ObjectAllocation>()).auxiliary
        ));
        dealloc(obj.cast(), obj_layout());
    }
}

/// Whether `obj` already carries a materialised string rep.
///
/// A type whose `update_string_proc` is `None` can only be attached to an
/// object that has one, since there would otherwise be no way back to a
/// spelling.
pub(crate) fn has_string_rep(obj: *mut TclObj) -> bool {
    // SAFETY: `obj` is a live object.
    !unsafe { (*obj).bytes }.is_null()
}

/// Inspect original physical storage and cache without reaching any updater.
pub(crate) fn native_object_snapshot(
    value: *mut TclObj,
) -> Result<tcl_syntax::native_object::NativeObjectSnapshot, tcl_syntax::value::ValueError> {
    check_native_liveness(value)?;
    use std::rc::Rc;
    use tcl_syntax::native_object::{NativeObjectCacheSnapshot as Cache, NativeObjectSnapshot};
    use tcl_syntax::native_string::NativeStringStorageIdentity as Storage;
    let kind = obj_type_ptr(value);
    let resident = has_string_rep(value).then(|| Rc::from(bytes_of(value)));
    let storage = resident.as_ref().map(|_| {
        if has_canonical_empty_string(value) {
            Storage::CanonicalEmpty
        } else {
            Storage::Allocated
        }
    });
    let cache = if kind.is_null() {
        Cache::None
    } else if core::ptr::eq(kind, &NATIVE_STRING_TYPE) {
        // SAFETY: the exact descriptor owns the live backing.
        let backing = unsafe { &*(internal_rep(value) as usize as *const NativeStringRep) };
        Cache::String {
            protocol: backing.protocol,
            num_chars: backing.count,
            unicode: backing.unicode.clone(),
        }
    } else if let Some(epoch) = with_jim_command_cache(value, |cache| cache.epoch) {
        Cache::JimCommand {
            procedure_epoch: epoch,
        }
    } else if let Some((frame, global)) =
        with_jim_variable_cache(value, |cache| (cache.frame, cache.global))
    {
        Cache::JimVariable { frame, global }
    } else if let Some(cache) = native_jim_subcommand::cache(value) {
        Cache::JimSubcommand {
            index: cache.index(),
        }
    } else if let Some(cache) = native_jim_enum::cache(value) {
        match cache {
            tcl_core_types::NativeJimOptionCache::Enum { entry, flags } => Cache::JimEnum {
                flags,
                index: entry.index(),
            },
            tcl_core_types::NativeJimOptionCache::ComparedString { .. } => Cache::JimComparedString,
        }
    } else if let Some(cache) = crate::interp::native_body_artifact::cache_snapshot(value) {
        cache
    } else if let Some(cache) = crate::native_script::cache_snapshot(value) {
        cache
    } else if let Some(cache) = crate::native_substitution::cache_snapshot(value) {
        cache
    } else if core::ptr::eq(kind, &JIM_STRING_TYPE) {
        Cache::JimString {
            num_chars: jim_string_count(value),
        }
    } else if let Some((bytes, proper)) = crate::bytearray::native_cache_snapshot(value) {
        Cache::ByteArray { bytes, proper }
    } else if let Some(version) = native_word_boolean_origin(kind) {
        Cache::WordBoolean {
            value: wide_of(value) != 0,
            version,
        }
    } else if let Some((length, canonical)) = crate::list::native_cache_snapshot(value) {
        Cache::List { length, canonical }
    } else if let Some(size) = crate::dict::native_cache_size(value) {
        Cache::Dictionary {
            size,
            pure: resident.is_none(),
        }
    } else if core::ptr::eq(kind, &NATIVE_COMMAND_NAME_TYPE) {
        // SAFETY: the exact descriptor owns its immutable primary receipt.
        let stored = unsafe { &*(internal_rep(value) as usize as *const NativeCommandNameRep) };
        Cache::CommandName {
            version: stored.version,
            resolved: stored.cache.is_some(),
        }
    } else if let Some(name) = native_instruction_name::cache(value) {
        Cache::InstructionName {
            version: name.version(),
            opcode: name.opcode(),
        }
    } else if core::ptr::eq(kind, &NATIVE_FRAME_LEVEL_TYPE) {
        let stored = allocation_cache::<NativeFrameLevelRep>(value).ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native frame cache allocation",
            ),
        )?;
        let (relative, level) = match stored.cache {
            tcl_registry::NativeFrameLevelCache::Relative(level) => (true, level),
            tcl_registry::NativeFrameLevelCache::Absolute(level) => (false, level),
        };
        Cache::FrameReference {
            version: stored.version,
            relative,
            level,
        }
    } else if let Some((cache, version)) = native_index::cache(value) {
        Cache::Index {
            version,
            index: cache.index(),
            stride: cache.stride(),
        }
    } else if let Some(cache) = native_namespace_name::cache(value) {
        Cache::NamespaceName {
            version: cache.version(),
            resolved: cache.namespace().is_some(),
        }
    } else if let Some((version, array)) = native_variable_name::with_parsed(value, |cache| {
        (cache.protocol.version(), cache.array.is_some())
    }) {
        Cache::ParsedVariableName { version, array }
    } else if let Some((version, index)) =
        native_variable_name::with_local(value, |cache| (cache.protocol.version(), cache.index))
    {
        Cache::LocalVariableName { version, index }
    } else if let Some(number) = native_scalar_cache(value)? {
        Cache::Numeric(number)
    } else {
        Cache::Other
    };
    Ok(NativeObjectSnapshot {
        resident,
        storage,
        cache,
    })
}

#[derive(Clone, Copy)]
struct NativeArraySearchRep {
    cache: tcl_core_types::NativeArraySearchCache,
    version: tcl_dialect::TclVersion,
}

struct NativeFrameLevelRep {
    cache: tcl_registry::NativeFrameLevelCache,
    version: tcl_dialect::TclVersion,
}

#[derive(Clone)]
struct NativeCommandNameRep {
    version: tcl_dialect::TclVersion,
    cache: Option<tcl_runtime_api::native_command_name::NativeCommandNameCache>,
}

/// Copy `obj`'s string rep (shimmering via `update_string_proc` if needed).
pub(crate) fn bytes_of(obj: *mut TclObj) -> Vec<u8> {
    // SAFETY: `obj` is a live object; `get_string` returns a borrowed pointer
    // into its (possibly just-generated) string rep, copied immediately.
    unsafe {
        let mut len: TclSize = 0;
        let p = get_string(obj, &mut len);
        if p.is_null() {
            return Vec::new();
        }
        core::slice::from_raw_parts(p as *const u8, len as usize).to_vec()
    }
}

/// Native canonical empty-string storage identity, separate from byte length.
static CANONICAL_EMPTY_STRING: [u8; 1] = [0];

/// Whether the original resident representation uses canonical empty storage.
pub(crate) fn has_canonical_empty_string(value: *mut TclObj) -> bool {
    unsafe { (*value).bytes == CANONICAL_EMPTY_STRING.as_ptr().cast::<c_char>().cast_mut() }
}

/// Allocate an owned, NUL-terminated buffer holding `src[..len]` and attach it
/// to `obj` as its string rep. Replaces any prior owned buffer.
///
/// # Safety
/// `obj` must be live; `src` must point to at least `len` readable bytes (or be
/// null when `len == 0`).
unsafe fn set_owned_string(obj: *mut TclObj, src: *const u8, len: usize) {
    unsafe {
        if len == 0 {
            free_string_buffer(obj);
            (*obj).bytes = CANONICAL_EMPTY_STRING.as_ptr().cast::<c_char>().cast_mut();
            (*obj).length = 0;
            if (*obj).type_ptr.is_null() {
                (*obj).internal_rep = 0;
            }
            return;
        }
        set_allocated_string(obj, src, len);
    }
}

/// Attach allocated storage even for zero bytes, preserving C 8 byte-array
/// updater identity. This is independent of the representation's contents.
unsafe fn set_allocated_string(obj: *mut TclObj, src: *const u8, len: usize) {
    // SAFETY: see fn-doc; `obj` is live and we own its `bytes` slot.
    unsafe {
        free_string_buffer(obj);
        let cap = len + 1; // + NUL terminator (Tcl keeps string reps NUL-term)
        let layout = Layout::from_size_align(cap, 1).expect("buffer layout");
        let buf = alloc(layout);
        if buf.is_null() {
            counters::oom_set();
            (*obj).bytes = core::ptr::null_mut();
            (*obj).length = 0;
            return;
        }
        if len > 0 {
            core::ptr::copy_nonoverlapping(src, buf, len);
        }
        *buf.add(len) = 0;
        (*obj).bytes = buf as *mut c_char;
        (*obj).length = len as TclSize;
        // For a plain string (no typed rep), track the buffer's allocated
        // capacity in `internal_rep` (unused otherwise) so `string`/`append` can
        // grow it amortised and `free_string_buffer` frees the right size.
        // A typed obj's `internal_rep` is its backing pointer — never touch it.
        if (*obj).type_ptr.is_null() {
            (*obj).internal_rep = cap as u64;
        }
        counters::buf_alloced();
    }
}

/// Free `obj`'s owned string buffer if it owns one. A null `bytes` (a "pure"
/// int/double obj that has never been shimmered) owns nothing.
///
/// # Safety
/// `obj` must be live.
unsafe fn free_string_buffer(obj: *mut TclObj) {
    // SAFETY: `obj` is live per caller; `bytes`, when non-null, was allocated
    // by `set_owned_string` with `Layout(length + 1, 1)` and `length` is
    // immutable for these obj kinds — strings are not appended-to here.
    unsafe {
        let bytes = (*obj).bytes;
        if bytes.is_null() {
            return;
        }
        if has_canonical_empty_string(obj) {
            (*obj).bytes = core::ptr::null_mut();
            (*obj).length = 0;
            return;
        }
        // Capacity: a plain string's allocated size lives in `internal_rep`; a
        // typed obj's cached string rep is exact (`length + 1`). Freeing with
        // the exact allocation size is required (Rust dealloc layout must match).
        let cap = if (*obj).type_ptr.is_null() {
            (*obj).internal_rep as usize
        } else {
            (*obj).length as usize + 1
        };
        let layout = Layout::from_size_align(cap, 1).expect("buffer layout");
        dealloc(bytes as *mut u8, layout);
        (*obj).bytes = core::ptr::null_mut();
        (*obj).length = 0;
        counters::buf_freed();
    }
}

// Constructors — all `fresh_zero` (refCount 0).

/// `Tcl_NewObj` — a fresh empty-string object at refCount 0.
pub fn new_obj() -> *mut TclObj {
    let obj = obj_alloc();
    if obj.is_null() {
        return obj;
    }
    // SAFETY: `obj` is a freshly allocated live header we uniquely own.
    unsafe {
        set_owned_string(obj, core::ptr::null(), 0);
    }
    obj
}

/// `Tcl_NewStringObj(bytes, length)` — copies the bytes. `length < 0` means
/// "NUL-terminated, use `strlen`". Result is `fresh_zero`.
///
/// # Safety
/// `bytes` must point to at least `length` readable bytes (or be a valid
/// NUL-terminated C string when `length < 0`).
pub unsafe fn new_string_obj(bytes: *const c_char, length: TclSize) -> *mut TclObj {
    let obj = obj_alloc();
    if obj.is_null() {
        return obj;
    }
    // SAFETY: caller guarantees `bytes`/`length`; `obj` is freshly owned.
    unsafe {
        let len = if length < 0 {
            if bytes.is_null() {
                0
            } else {
                libc_strlen(bytes)
            }
        } else {
            length as usize
        };
        set_owned_string(obj, bytes as *const u8, len);
    }
    obj
}

/// A fresh (`rc 0`) string object holding `bytes` (the internal byte-slice
/// constructor the value-type modules use).
pub(crate) fn new_string_bytes(bytes: &[u8]) -> *mut TclObj {
    // SAFETY: `bytes` is a valid readable slice.
    unsafe { new_string_obj(bytes.as_ptr() as *const c_char, bytes.len() as TclSize) }
}

/// `Tcl_IsShared` — does more than one reference hold `obj`? Mutation in place is
/// only sound on an unshared object; otherwise copy-on-write.
pub(crate) fn is_shared(obj: *mut TclObj) -> bool {
    // SAFETY: `obj` is a live object.
    unsafe { (*obj).ref_count > 1 }
}

// Refcount — `Tcl_IncrRefCount` / `Tcl_DecrRefCount` (faithful to the macros).

/// `Tcl_IncrRefCount`. Null-safe.
///
/// # Safety
/// `obj` must be null or a live header.
pub unsafe fn incr_ref_count(obj: *mut TclObj) {
    if obj.is_null() {
        return;
    }
    // SAFETY: caller guarantees `obj` is live.
    unsafe {
        assert!(
            allocation_is_live(obj),
            "native reference to a retired object"
        );
        (*obj).ref_count += 1;
    }
}

/// `Tcl_DecrRefCount`. Frees the object immediately when the count reaches
/// zero (`TclFreeObj`). Null-safe. Increments the double-free counter if called
/// on an object already at refCount 0 (a contract violation — see the MM-B
/// double-free guard).
///
/// # Safety
/// `obj` must be null or a live header to which the caller holds a reference.
pub unsafe fn decr_ref_count(obj: *mut TclObj) {
    if obj.is_null() {
        return;
    }
    // SAFETY: caller guarantees `obj` is live and holds a reference.
    unsafe {
        if (*obj).ref_count <= 0 {
            // Releasing an object with no outstanding reference: a double-free
            // / contract violation. Count it and refuse to free again.
            counters::double_free();
            return;
        }
        (*obj).ref_count -= 1;
        if (*obj).ref_count <= 0 {
            free_obj(obj);
        }
    }
}

// String rep — `Tcl_GetStringFromObj` / `Tcl_GetString` (shimmer on demand).

/// `Tcl_GetStringFromObj` — returns a borrowed pointer into the object's string
/// rep, generating it on demand for pure int objects (shimmer). The pointer is
/// valid until the object is modified or freed.
///
/// # Safety
/// `obj` must be a live header. `length_out`, if non-null, must be writable.
pub unsafe fn get_string(obj: *mut TclObj, length_out: *mut TclSize) -> *mut c_char {
    // SAFETY: caller guarantees `obj` is live; we own its `bytes` slot for the
    // shimmer write.
    unsafe {
        if (*obj).bytes.is_null() {
            if !native_string_available(obj) {
                if !length_out.is_null() {
                    *length_out = 0;
                }
                return core::ptr::null_mut();
            }
            // Generate the string rep via the type's update_string_proc (int,
            // double, list, an extension's custom type, …). A typed obj with no
            // proc, or an untyped obj, gets the empty string rep.
            let tp = (*obj).type_ptr;
            if !tp.is_null() {
                if let Some(update) = (*tp).update_string_proc {
                    update(obj);
                }
            }
            if (*obj).bytes.is_null() {
                set_owned_string(obj, core::ptr::null(), 0);
            }
        }
        if !length_out.is_null() {
            *length_out = (*obj).length;
        }
        (*obj).bytes
    }
}

// Small helpers (no libc dependency in the native build).

/// `strlen` over a NUL-terminated C string.
///
/// # Safety
/// `s` must point to a NUL-terminated string.
unsafe fn libc_strlen(s: *const c_char) -> usize {
    // SAFETY: caller guarantees NUL termination.
    unsafe {
        let mut n = 0usize;
        while *s.add(n) != 0 {
            n += 1;
        }
        n
    }
}

#[cfg(test)]
mod script_location_tests;

#[cfg(test)]
mod double_precision_tests;

/// Genuine original Jim Enum and immediate literal cache storage.
pub(crate) mod native_jim_enum;
pub(crate) mod native_jim_subcommand;
pub(crate) mod native_method_name;
pub(crate) mod native_property_name;

#[cfg(test)]
#[path = "obj/native_scalar_adoption_tests.rs"]
mod native_scalar_adoption_tests;

#[cfg(test)]
#[path = "obj/native_duplicate_controls.rs"]
mod native_duplicate_controls;

#[cfg(test)]
#[path = "obj/tls_retirement_tests.rs"]
mod tls_retirement_tests;
