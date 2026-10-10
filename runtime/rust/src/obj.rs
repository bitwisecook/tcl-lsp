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

mod native_jim_lookup;
pub(crate) use native_jim_lookup::{JimCommandCache, JimVariableCache};
pub(crate) use native_jim_lookup::{
    install_command_cache as install_jim_command_cache,
    install_variable_cache as install_jim_variable_cache,
    with_command_cache as with_jim_command_cache, with_variable_cache as with_jim_variable_cache,
};

use core::ffi::{c_char, c_void};
use std::alloc::{Layout, alloc, dealloc, realloc};
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

extern "C" fn native_string_free(value: *mut TclObj) {
    // SAFETY: this exact descriptor owns the boxed backing.
    unsafe {
        drop(Box::from_raw(
            internal_rep(value) as usize as *mut NativeStringRep
        ))
    };
}

extern "C" fn native_string_dup(value: *mut TclObj, duplicate: *mut TclObj) {
    // SAFETY: the exact descriptor owns a live backing and duplicate is fresh.
    let backing = unsafe { &*(internal_rep(value) as usize as *const NativeStringRep) };
    if !backing
        .protocol
        .string_primary_survives_duplicate(backing.count)
    {
        return;
    }
    let copied = Box::new(backing.clone());
    change_type(
        duplicate,
        &NATIVE_STRING_TYPE,
        Box::into_raw(copied) as usize as u64,
    );
}

extern "C" fn native_string_update(value: *mut TclObj) {
    // SAFETY: the exact descriptor owns the live backing. A stringless native
    // String is created only with validated retained Unicode units.
    let backing = unsafe { &*(internal_rep(value) as usize as *const NativeStringRep) };
    let Some(unicode) = &backing.unicode else {
        if backing.count == Some(0) {
            // SAFETY: the selected zero-count constructor owns a live empty String.
            unsafe { set_string_rep(value, b"") };
        }
        return;
    };
    let version = backing
        .protocol
        .tcl_version()
        .expect("native C String backing");
    let bytes = tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version)
        .encode_units(unicode)
        .expect("validated native Unicode unit width");
    // SAFETY: the owned encoded extent remains live through its buffer copy.
    unsafe { set_string_rep(value, &bytes) };
}

static NATIVE_STRING_TYPE: TclObjType = TclObjType {
    name: c"string".as_ptr(),
    free_int_rep_proc: Some(native_string_free),
    dup_int_rep_proc: Some(native_string_dup),
    update_string_proc: Some(native_string_update),
    set_from_any_proc: None,
};

/// Record a successful native string coercion after its bytes were retained.
fn retain_native_string_count(
    value: *mut TclObj,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
    count: usize,
) {
    let backing = Box::new(NativeStringRep {
        protocol,
        count: Some(count),
        unicode: None,
    });
    change_type(
        value,
        &NATIVE_STRING_TYPE,
        Box::into_raw(backing) as usize as u64,
    );
}

/// Reach the selected actual character-count protocol on the original object.
pub(crate) fn native_character_count(
    value: *mut TclObj,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
    representation: tcl_registry::native_string_length::NativeStringLengthRepresentation,
) -> Result<usize, tcl_syntax::value::ValueError> {
    use tcl_syntax::value::ValueError;
    if !native_string_available(value) {
        return Err(ValueError::CommandProtocolUnavailable(
            "native string length storage",
        ));
    }
    if let Some(length) = crate::bytearray::native_string_length(value, representation) {
        return Ok(length);
    }
    if representation.preserves_short_string()
        && has_string_rep(value)
        && unsafe { (*value).length } <= 1
    {
        return Ok(unsafe { (*value).length as usize });
    }
    if protocol.is_jim084() {
        drop(crate::dict::native_object_bytes(value, protocol)?);
        return Ok(jim_character_count(value));
    }
    if core::ptr::eq(obj_type_ptr(value), &NATIVE_STRING_TYPE) {
        // SAFETY: the exact descriptor owns this live backing.
        let backing = unsafe { &*(internal_rep(value) as usize as *const NativeStringRep) };
        if backing.protocol != protocol {
            return Err(ValueError::CommandProtocolUnavailable(
                "native string count cache origin",
            ));
        }
        if let Some(count) = backing.count {
            return Ok(count);
        }
    }
    let bytes = crate::dict::native_object_bytes(value, protocol)?;
    let version = protocol
        .tcl_version()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "native character units",
        ))?;
    let count = tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version)
        .decode_units(&bytes)
        .len();
    retain_native_string_count(value, protocol, count);
    Ok(count)
}

/// Install an actual Jim string result's native cached count without changing bytes.
pub(crate) fn retain_jim_string_count(value: *mut TclObj, count: usize) {
    change_type(
        value,
        &JIM_STRING_TYPE,
        u64::try_from(count).expect("native character count fits u64"),
    );
}

/// Install Jim's string intrep without calculating its unknown character count.
/// Native string operations preserve an existing cached count across conversion.
pub(crate) fn retain_jim_string_representation(value: *mut TclObj) {
    if !core::ptr::eq(obj_type_ptr(value), &JIM_STRING_TYPE) {
        change_type(value, &JIM_STRING_TYPE, u64::MAX);
    }
}

/// Retain the actual unknown-count String primary of an append/format producer.
/// Existing resident bytes remain owned by the same original object header.
pub(crate) fn retain_native_string_representation(
    value: *mut TclObj,
    protocol: tcl_registry::native_string_materialization::NativeStringMaterialization,
) -> Result<(), tcl_syntax::value::ValueError> {
    if !has_string_rep(value) {
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native String producer resident storage",
        ));
    }
    if protocol.protocol().is_jim084() {
        retain_jim_string_representation(value);
    } else {
        let backing = Box::new(NativeStringRep {
            protocol: protocol.protocol(),
            count: None,
            unicode: None,
        });
        change_type(
            value,
            &NATIVE_STRING_TYPE,
            Box::into_raw(backing) as usize as u64,
        );
    }
    Ok(())
}

/// Apply a uniquely owned Jim suffix cut, preserving its current count receipt.
pub(crate) fn trim_jim_string_bytes(value: *mut TclObj, bytes: &[u8]) {
    debug_assert!(!is_shared(value));
    revoke_script_location(value);
    // SAFETY: the caller holds the live, unshared object and an independent
    // byte snapshot. Its existing string intrep and cached count are retained.
    unsafe { set_string_rep(value, bytes) };
}

/// Current native Jim string count; any internal-representation conversion
/// withdraws this receipt through the central type-change door.
pub(crate) fn jim_string_count(value: *mut TclObj) -> Option<usize> {
    // SAFETY: the value is live for its caller's object operation.
    unsafe {
        (std::ptr::eq((*value).type_ptr, &JIM_STRING_TYPE) && (*value).internal_rep != u64::MAX)
            .then(|| {
                usize::try_from((*value).internal_rep).expect("native cached count fits usize")
            })
    }
}

/// Jim's actual string count conversion. Reuse the object's current cache;
/// otherwise install the count of the original native byte units.
pub(crate) fn jim_character_count(value: *mut TclObj) -> usize {
    if let Some(count) = jim_string_count(value) {
        return count;
    }
    let bytes = tcl_syntax::raw_string::RawString::from_bytes(bytes_of(value));
    let count = bytes.jim084_characters().count();
    retain_jim_string_count(value, count);
    count
}

/// The `int` (wide) type descriptor. `typePtr == &TCL_INT_TYPE` ⇒ the value is
/// in `internal_rep` as a `TclWideInt`; `bytes` may be null until shimmered.
pub static TCL_INT_TYPE: TclObjType = TclObjType {
    name: c"int".as_ptr(),
    free_int_rep_proc: None, // an int rep owns nothing
    dup_int_rep_proc: None,  // the i64 is copied with the header
    update_string_proc: Some(int_update_string),
    set_from_any_proc: None,
};
/// C Tcl 8.4 native-long primary cache, distinct from its wideInt cache.
pub(crate) static TCL_LONG84_TYPE: TclObjType = TclObjType {
    name: c"int".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(int_update_string),
    set_from_any_proc: None,
};
/// Jim's explicit double getter preserves the exact wide integer. The string
/// updater remains integer-based, and a subsequent wide getter restores Int.
pub(crate) static JIM_COERCED_DOUBLE_TYPE: TclObjType = TclObjType {
    name: c"coerced-double".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(int_update_string),
    set_from_any_proc: None,
};

/// Jim_GetWide restores the same retained integer without reparsing bytes.
pub(crate) fn restore_jim_coerced_integer(value: *mut TclObj) -> Option<i64> {
    if !core::ptr::eq(obj_type_ptr(value), &JIM_COERCED_DOUBLE_TYPE) {
        return None;
    }
    let integer = wide_of(value);
    change_type(value, &TCL_INT_TYPE, integer as u64);
    Some(integer)
}

/// The `double` type descriptor.
pub static TCL_DOUBLE_TYPE: TclObjType = TclObjType {
    name: c"double".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(double_update_string),
    set_from_any_proc: None,
};

extern "C" fn int_update_string(obj: *mut TclObj) {
    // SAFETY: `obj` is a live int object whose string rep needs generating.
    unsafe {
        let s = itoa((*obj).wide());
        set_owned_string(obj, s.as_ptr(), s.len());
    }
}

type ObjectScriptLocation =
    tcl_runtime_api::script_source_location::ScriptSourceLocation<Option<std::rc::Rc<[u8]>>>;

thread_local! {
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
    SCRIPT_LOCATIONS.with(|locations| locations.borrow_mut().remove(&(obj as usize)));
}

/// Install the native lazy double conversion of the executing engine.
pub(crate) fn install_double_string_policy(policy: tcl_dialect::DoubleStringPolicy) {
    DOUBLE_STRING_POLICY.set(policy);
    DOUBLE_PRECISIONS.with(|values| {
        values
            .borrow_mut()
            .entry(policy)
            .or_insert(policy.default_precision());
    });
}

/// C Tcl's thread-shared linked precision, independently of a variable's raw value.
pub(crate) fn double_precision(policy: tcl_dialect::DoubleStringPolicy) -> u8 {
    DOUBLE_PRECISIONS.with(|values| {
        *values
            .borrow_mut()
            .entry(policy)
            .or_insert(policy.default_precision())
    })
}

/// Commit a precision accepted by the shared native trace grammar.
pub(crate) fn set_double_precision(policy: tcl_dialect::DoubleStringPolicy, precision: u8) {
    debug_assert!(policy.format(precision).is_some());
    DOUBLE_PRECISIONS.with(|values| {
        values.borrow_mut().insert(policy, precision);
    });
}

extern "C" fn double_update_string(obj: *mut TclObj) {
    // SAFETY: `obj` is a live double object. The canonical Tcl double→string is
    // the shared `tcl_syntax::number::format_double` (also used by the compiler's
    // const-folder) — integer-valued doubles get `.0`, plus `Inf`/`NaN`.
    unsafe {
        let policy = DOUBLE_STRING_POLICY.get();
        let format = policy
            .format(double_precision(policy))
            .expect("validated native precision");
        let s = tcl_syntax::number::format_double_native_selected((*obj).double(), policy, format);
        set_owned_string(obj, s.as_ptr(), s.len());
    }
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
    live: Cell<bool>,
    retiring: Cell<bool>,
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
    if let Some(retained) = scalar_object_context(value)?
        && !retained.same_issuer(incoming)
    {
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "foreign or stale native scalar object issuer",
        ));
    }
    Ok(())
}

pub(crate) fn bind_scalar_object_context(
    value: *mut TclObj,
    incoming: Rc<crate::interp::native_scalar_context::NativeScalarObjectContext>,
) -> Result<(), tcl_syntax::value::ValueError> {
    validate_scalar_object_context(value, &incoming)?;
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

/// Memory safety lease with no native object reference. A native decr-to-zero
/// still retires the cache and its children immediately while this holds only
/// the allocation. A retired object cannot be promoted back into a native owner.
pub(crate) struct NativeObjectLifetime(*mut TclObj);
impl NativeObjectLifetime {
    pub(crate) fn as_ptr(&self) -> *mut TclObj {
        self.0
    }
    pub(crate) fn retain(value: *mut TclObj) -> Self {
        let state = auxiliary(value);
        assert!(state.live.get(), "lifetime lease on a retired object");
        state.lifetime_pins.set(
            state
                .lifetime_pins
                .get()
                .checked_add(1)
                .expect("object lifetime pins"),
        );
        Self(value)
    }
}
impl Clone for NativeObjectLifetime {
    fn clone(&self) -> Self {
        let state = auxiliary(self.0);
        state.lifetime_pins.set(
            state
                .lifetime_pins
                .get()
                .checked_add(1)
                .expect("object lifetime pins"),
        );
        Self(self.0)
    }
}

/// A borrowed object pointer provided by either a genuine reference or an
/// allocation-only procedure view. Native receivers acquire their own roles.
pub trait ObjectPointer {
    /// Borrow the selected original allocation.
    fn as_ptr(&self) -> *mut TclObj;
}

/// Memory-safe view of a procedure object without a native object reference.
/// A surviving view cannot revive resources retired by the last native role.
#[derive(Clone)]
pub struct ProcedureObject(NativeObjectLifetime);
impl ProcedureObject {
    pub(crate) fn retain_lifetime(original: &Owned) -> Self {
        Self(NativeObjectLifetime::retain(original.as_ptr()))
    }
    /// Borrow the original address; consumers must check native liveness before
    /// performing a native operation.
    #[must_use]
    pub fn as_ptr(&self) -> *mut TclObj {
        self.0.0
    }
    /// Select a live original header without acquiring a native reference.
    pub(crate) fn checked_ptr(&self) -> Result<*mut TclObj, tcl_syntax::value::ValueError> {
        allocation_is_live(self.as_ptr())
            .then_some(self.as_ptr())
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "retired native procedure object",
            ))
    }
}
impl ObjectPointer for ProcedureObject {
    fn as_ptr(&self) -> *mut TclObj {
        self.as_ptr()
    }
}
impl Drop for NativeObjectLifetime {
    fn drop(&mut self) {
        let state = auxiliary(self.0);
        let remaining = state
            .lifetime_pins
            .get()
            .checked_sub(1)
            .expect("object lifetime retirement");
        state.lifetime_pins.set(remaining);
        if remaining == 0 && !state.live.get() && !state.retiring.get() {
            // SAFETY: all native resources were retired at the actual free
            // boundary, and this is the last memory safety owner.
            unsafe { deallocate_object(self.0) };
        }
    }
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

/// An owned object reference (`rc +1`) that releases on drop — the discipline
/// that keeps the shared recursive walk leak-/double-free-safe across early
/// returns.
pub struct Owned(*mut TclObj);

impl ObjectPointer for Owned {
    fn as_ptr(&self) -> *mut TclObj {
        self.as_ptr()
    }
}

impl Owned {
    /// Take an existing owning reference without retaining the original again.
    ///
    /// # Safety
    /// The caller must transfer one live, independently owned `+1` reference.
    pub(crate) unsafe fn from_raw(o: *mut TclObj) -> Owned {
        Owned(o)
    }

    /// Take an owning `+1` on a live object (e.g. a variable's store value).
    pub(crate) fn retain(o: *mut TclObj) -> Owned {
        // SAFETY: `o` is a live object.
        unsafe { incr_ref_count(o) };
        Owned(o)
    }

    /// Adopt a freshly-minted (`rc 0`) object, taking it to `rc 1`.
    pub(crate) fn fresh(o: *mut TclObj) -> Owned {
        // SAFETY: `o` is a fresh object from a constructor / tower op.
        unsafe { incr_ref_count(o) };
        Owned(o)
    }

    #[cfg(have_tommath)]
    #[inline]
    pub(crate) fn ptr(&self) -> *mut TclObj {
        self.0
    }

    /// The borrowed object pointer (the `+1` stays with this `Owned`). Callers
    /// that retain it (e.g. `Tcl_SetObjResult`, which takes its own `+1`) read
    /// through this and let the `Owned` drop its reference normally.
    #[inline]
    #[must_use]
    pub fn as_ptr(&self) -> *mut TclObj {
        self.0
    }

    /// Hand the `+1` to the caller without releasing it here.
    pub fn into_raw(self) -> *mut TclObj {
        let o = self.0;
        core::mem::forget(self);
        o
    }

    /// Reverse the genuine interpolation reference without freeing a native
    /// refcount-zero result header. The next real receiver must retain it.
    pub(crate) fn into_native_unowned(self) -> *mut TclObj {
        let value = self.into_raw();
        // SAFETY: this consumed Owned held exactly one genuine reference.
        unsafe {
            assert!((*value).ref_count > 0, "owned interpolation reference");
            (*value).ref_count -= 1;
        }
        value
    }
}

impl Clone for Owned {
    fn clone(&self) -> Self {
        Self::retain(self.0)
    }
}

impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: `self.0` is the object we hold a `+1` on.
        unsafe { decr_ref_count(self.0) };
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
fn obj_alloc() -> *mut TclObj {
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
                live: Cell::new(true),
                retiring: Cell::new(false),
                cache: RefCell::new(None),
                scalar_context: RefCell::new(None),
            },
        );
        counters::obj_alloced();
        p
    }
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

// Typed internal-rep helpers — the shimmer keystone's plumbing, used by the
// value-type modules (`list`, future `dict`, …). pub(crate): internal only.

/// Allocate a fresh (`rc 0`) object with a typed internal rep and no string rep.
pub(crate) fn alloc_typed(type_ptr: *const TclObjType, internal_rep: u64) -> *mut TclObj {
    let obj = obj_alloc();
    if obj.is_null() {
        return obj;
    }
    // SAFETY: `obj` is a freshly owned header.
    unsafe {
        (*obj).type_ptr = type_ptr;
        (*obj).internal_rep = internal_rep;
    }
    obj
}

/// Read the raw 8-byte internal rep (a value type stores its backing pointer here).
pub(crate) fn internal_rep(obj: *mut TclObj) -> u64 {
    // SAFETY: `obj` is a live object.
    unsafe { (*obj).internal_rep }
}

/// `obj`'s current type descriptor (null for a plain string).
pub(crate) fn obj_type_ptr(obj: *mut TclObj) -> *const TclObjType {
    // SAFETY: `obj` is a live object.
    unsafe { (*obj).type_ptr }
}

/// Shimmer `obj` to a new type: free the **old** internal rep (its proc), then
/// install `new_type` + `new_rep`. The string rep is **kept** across a
/// string→typed shimmer (Tcl's dual-rep: the original spelling survives until
/// the typed value is mutated, which invalidates it via [`invalidate_string`]).
pub(crate) fn change_type(obj: *mut TclObj, new_type: *const TclObjType, new_rep: u64) {
    let retired_cache = auxiliary(obj).cache.take();
    // SAFETY: `obj` is live; free the prior rep before overwriting `internal_rep`.
    unsafe {
        let old = (*obj).type_ptr;
        if old.is_null() {
            // Leaving a *plain string*: its capacity lives in `internal_rep`,
            // which `new_type` is about to claim for its backing. Keep the bytes
            // as the cached (immutable) string rep, but first shrink the buffer
            // to exactly `length + 1` — once `type_ptr` is non-null,
            // `free_string_buffer` computes the dealloc size as `length + 1`, so
            // the buffer must match (any spare capacity from `append` would
            // otherwise be a layout mismatch). The next read returns these bytes
            // verbatim; an in-place mutation of the new rep drops them.
            shrink_string_to_exact(obj);
        } else if let Some(free) = (*old).free_int_rep_proc {
            free(obj);
        }
        (*obj).type_ptr = new_type;
        (*obj).internal_rep = new_rep;
    }
    drop(retired_cache);
}

/// Drop a reached cache while retaining exact resident storage and capacity.
pub(crate) fn discard_native_internal_representation(
    value: *mut TclObj,
) -> Result<(), tcl_syntax::value::ValueError> {
    check_native_liveness(value)?;
    if !has_string_rep(value) {
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native cache discard resident string",
        ));
    }
    if obj_type_ptr(value).is_null() {
        return Ok(());
    }
    let capacity = if has_canonical_empty_string(value) {
        0
    } else {
        // SAFETY: a typed object's resident allocation has exactly length+1 bytes.
        unsafe { (*value).length as usize + 1 }
    };
    change_type(value, core::ptr::null(), capacity as u64);
    Ok(())
}

/// Reset the same unshared C result header to canonical empty storage.
pub(crate) fn reset_native_c_result(value: *mut TclObj) {
    debug_assert!(!is_shared(value));
    // SAFETY: the interpreter owns this live unshared result. Publish its
    // canonical empty bytes before releasing the previous internal backing.
    unsafe { set_string_rep(value, b"") };
    change_type(value, core::ptr::null(), 0);
}

/// Shrink a plain string's buffer to exactly `length + 1` so its cached rep can
/// outlive a shimmer (after which `internal_rep` no longer tracks capacity and
/// `free_string_buffer` assumes the exact `length + 1` layout). No-op when there
/// is no buffer or it is already exact (set-only strings). On a rare shrink
/// failure, drop the rep (it regenerates lazily) rather than carry an unknown
/// capacity across the shimmer.
///
/// # Safety
/// `obj` must be live and a plain string (`type_ptr` null, capacity in
/// `internal_rep`).
unsafe fn shrink_string_to_exact(obj: *mut TclObj) {
    // SAFETY: caller guarantees a live plain-string `obj`; `bytes` (when
    // non-null) was allocated by `set_owned_string` with `Layout(internal_rep,1)`.
    unsafe {
        let bytes = (*obj).bytes;
        if bytes.is_null() {
            return;
        }
        let cur_cap = (*obj).internal_rep as usize; // allocated bytes incl. NUL
        let exact = (*obj).length as usize + 1;
        if cur_cap <= exact {
            return; // already exact — nothing to reclaim
        }
        let old_layout = Layout::from_size_align(cur_cap, 1).expect("buffer layout");
        let nb = realloc(bytes as *mut u8, old_layout, exact);
        if nb.is_null() {
            // Shrink failed; the original block is intact — free it (the rep
            // regenerates on next read) to avoid a later layout mismatch.
            free_string_buffer(obj);
            return;
        }
        (*obj).bytes = nb as *mut c_char;
        // `internal_rep` is left as-is here; `change_type` overwrites it with the
        // new typed rep immediately after this returns.
    }
}

/// Invalidate the string rep (drop the buffer) so it regenerates via the type's
/// `update_string_proc` on the next read — call after mutating a typed rep.
pub(crate) fn invalidate_string(obj: *mut TclObj) {
    revoke_script_location(obj);
    // SAFETY: `obj` is live; dropping its owned buffer is sound (it will be
    // regenerated lazily).
    unsafe { free_string_buffer(obj) }
}

/// Set `obj`'s string rep to a copy of `bytes` (for `update_string_proc` impls).
///
/// # Safety
/// `obj` must be live.
pub(crate) unsafe fn set_string_rep(obj: *mut TclObj, bytes: &[u8]) {
    // SAFETY: forwarded — `obj` live, slice readable.
    unsafe { set_owned_string(obj, bytes.as_ptr(), bytes.len()) }
}

/// Install the selected native updater storage without deriving canonical
/// empty identity from byte contents alone.
pub(crate) unsafe fn set_native_updater_string_rep(
    obj: *mut TclObj,
    bytes: &[u8],
    canonical_empty: bool,
) {
    unsafe {
        if bytes.is_empty() && !canonical_empty {
            set_allocated_string(obj, bytes.as_ptr(), bytes.len());
        } else {
            set_owned_string(obj, bytes.as_ptr(), bytes.len());
        }
    }
}

/// Read a `TCL_INT_TYPE` object's wide value from its internal rep.
pub(crate) fn wide_of(obj: *mut TclObj) -> TclWideInt {
    // SAFETY: caller has checked `obj`'s type is `TCL_INT_TYPE`.
    unsafe { (*obj).wide() }
}

/// Read a `TCL_DOUBLE_TYPE` object's value from its internal rep.
pub(crate) fn double_of(obj: *mut TclObj) -> f64 {
    // SAFETY: caller has checked `obj`'s type is `TCL_DOUBLE_TYPE`.
    unsafe { (*obj).double() }
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

/// Whether a just-parsed numeric internal rep may be cached back onto `obj`.
///
/// C Tcl caches unconditionally: `TclParseNumber` (`tclStrToD.c`) writes the rep
/// it built straight onto the object it parsed, whatever the refcount, because
/// the **string** rep is kept — every other holder still reads the same
/// spelling, it just no longer pays to re-parse it. That reasoning holds here
/// for the two shapes where the string rep *is* the value: a plain string (no
/// typed rep to destroy), and any unshared object. A *shared* object already
/// carrying some other typed rep — a list, a dict — is left alone instead:
/// [`change_type`] frees that rep, and the other holders would pay to rebuild
/// what they still want.
pub(crate) fn may_cache_parsed_rep(obj: *mut TclObj) -> bool {
    obj_type_ptr(obj).is_null() || !is_shared(obj)
}

/// Cache a parsed wide-integer rep onto `obj`, keeping its string rep, so the
/// next numeric use reads the rep instead of re-parsing the spelling. A no-op
/// where [`may_cache_parsed_rep`] declines.
pub(crate) fn cache_wide_rep(obj: *mut TclObj, value: TclWideInt) {
    if may_cache_parsed_rep(obj) {
        change_type(obj, &TCL_INT_TYPE, value as u64);
    }
}

/// [`cache_wide_rep`] for a parsed double.
pub(crate) fn cache_double_rep(obj: *mut TclObj, value: f64) {
    if may_cache_parsed_rep(obj) {
        change_type(obj, &TCL_DOUBLE_TYPE, value.to_bits());
    }
}

/// Numeric representation established by an actual reached native conversion.
/// This is an execution effect, independently of opportunistic parse caching.
pub(crate) enum NativeNumericRepresentation {
    Wide(TclWideInt),
    Double(f64),
}

/// Install a successful native operand conversion on the same shared object.
/// Existing bytes survive, while a prior list/dict representation is destroyed
/// exactly when the native conversion was reached. Callers must supply the
/// selected engine's successfully parsed numeric value, never an optimisation
/// approximation or a function's separately constructed result.
pub(crate) fn adopt_native_numeric_representation(
    obj: *mut TclObj,
    representation: NativeNumericRepresentation,
) {
    // A container with only an internal representation must first retain its
    // original bytes. Numeric objects retain their lazy native formatting.
    if !has_string_rep(obj)
        && !obj_type_ptr(obj).is_null()
        && !std::ptr::eq(obj_type_ptr(obj), &TCL_INT_TYPE)
        && !std::ptr::eq(obj_type_ptr(obj), &TCL_DOUBLE_TYPE)
        && !std::ptr::eq(obj_type_ptr(obj), &JIM_COERCED_DOUBLE_TYPE)
    {
        let _ = bytes_of(obj);
    }
    match representation {
        NativeNumericRepresentation::Wide(value) => {
            change_type(obj, &TCL_INT_TYPE, value as u64);
        }
        NativeNumericRepresentation::Double(value) => {
            change_type(obj, &TCL_DOUBLE_TYPE, value.to_bits());
        }
    }
}

/// Native word-boolean cache, distinct from a constructed integer boolean.
static NATIVE_WORD_BOOLEAN_84_TYPE: TclObjType = TclObjType {
    name: c"boolean".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(int_update_string),
    set_from_any_proc: None,
};
static NATIVE_WORD_BOOLEAN_85_TYPE: TclObjType = TclObjType {
    name: c"booleanString".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
static NATIVE_WORD_BOOLEAN_86_TYPE: TclObjType = TclObjType {
    name: c"booleanString".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
static NATIVE_WORD_BOOLEAN_90_TYPE: TclObjType = TclObjType {
    name: c"boolean".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
static NATIVE_WORD_BOOLEAN_91_TYPE: TclObjType = TclObjType {
    name: c"boolean".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

fn native_word_boolean_origin(kind: *const TclObjType) -> Option<tcl_dialect::TclVersion> {
    use tcl_dialect::TclVersion;
    [
        (&NATIVE_WORD_BOOLEAN_84_TYPE, TclVersion::V8_4),
        (&NATIVE_WORD_BOOLEAN_85_TYPE, TclVersion::V8_5),
        (&NATIVE_WORD_BOOLEAN_86_TYPE, TclVersion::V8_6),
        (&NATIVE_WORD_BOOLEAN_90_TYPE, TclVersion::V9_0),
        (&NATIVE_WORD_BOOLEAN_91_TYPE, TclVersion::V9_1),
    ]
    .into_iter()
    .find_map(|(descriptor, version)| core::ptr::eq(kind, descriptor).then_some(version))
}

/// Inspect the exact release of a native word-Boolean descriptor.
pub(crate) fn native_word_boolean_version(value: *mut TclObj) -> Option<tcl_dialect::TclVersion> {
    native_word_boolean_origin(obj_type_ptr(value))
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

/// Reach native C Unicode-unit storage on the original object.
pub(crate) fn new_native_unicode_obj(
    unicode: std::rc::Rc<[u32]>,
    dialect: tcl_registry::InvocationDialect,
) -> Result<*mut TclObj, tcl_syntax::value::ValueError> {
    use tcl_syntax::value::ValueError;
    let protocol =
        dialect
            .native_string_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native Unicode constructor",
            ))?;
    let version = protocol
        .tcl_version()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "native Unicode constructor",
        ))?;
    tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version)
        .encode_units(&unicode)
        .ok_or(ValueError::CommandProtocolUnavailable(
            "native Unicode unit width",
        ))?;
    let backing = Box::new(NativeStringRep {
        protocol,
        count: Some(unicode.len()),
        unicode: protocol
            .unicode_constructor_has_unicode(unicode.len())
            .then_some(unicode),
    });
    Ok(alloc_typed(
        &NATIVE_STRING_TYPE,
        Box::into_raw(backing) as usize as u64,
    ))
}

/// Reach native C Unicode-unit storage on the original object.
pub(crate) fn native_unicode_units(
    value: *mut TclObj,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) -> Result<std::rc::Rc<[u32]>, tcl_syntax::value::ValueError> {
    use tcl_syntax::value::ValueError;
    let version = protocol
        .tcl_version()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "native Unicode unit storage",
        ))?;
    if !native_string_available(value) {
        return Err(ValueError::CommandProtocolUnavailable(
            "native Unicode string updater",
        ));
    }
    if core::ptr::eq(obj_type_ptr(value), &NATIVE_STRING_TYPE) {
        // SAFETY: the exact descriptor owns this live backing.
        let backing = unsafe { &*(internal_rep(value) as usize as *const NativeStringRep) };
        if backing.protocol != protocol {
            return Err(ValueError::CommandProtocolUnavailable(
                "native Unicode cache origin",
            ));
        }
        if let Some(unicode) = &backing.unicode {
            return Ok(unicode.clone());
        }
    }
    let unicode: std::rc::Rc<[u32]> = std::rc::Rc::from(
        tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version)
            .decode_units(&crate::dict::native_object_bytes(value, protocol)?),
    );
    let backing = Box::new(NativeStringRep {
        protocol,
        count: Some(unicode.len()),
        unicode: Some(unicode.clone()),
    });
    change_type(
        value,
        &NATIVE_STRING_TYPE,
        Box::into_raw(backing) as usize as u64,
    );
    Ok(unicode)
}

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

#[derive(Clone, Copy)]
struct NativeArraySearchRep {
    cache: tcl_core_types::NativeArraySearchCache,
    version: tcl_dialect::TclVersion,
}
static NATIVE_ARRAY_SEARCH_TYPE: TclObjType = TclObjType {
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

struct NativeFrameLevelRep {
    cache: tcl_registry::NativeFrameLevelCache,
    version: tcl_dialect::TclVersion,
}

static NATIVE_FRAME_LEVEL_TYPE: TclObjType = TclObjType {
    name: c"levelReference".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

#[derive(Clone)]
struct NativeCommandNameRep {
    version: tcl_dialect::TclVersion,
    cache: Option<tcl_runtime_api::native_command_name::NativeCommandNameCache>,
}

extern "C" fn command_name_free(value: *mut TclObj) {
    // SAFETY: this exact descriptor owns the boxed immutable cache receipt.
    unsafe {
        drop(Box::from_raw(
            internal_rep(value) as usize as *mut NativeCommandNameRep
        ));
    }
}

extern "C" fn command_name_dup(source: *mut TclObj, duplicate: *mut TclObj) {
    // SAFETY: this exact source descriptor owns the live immutable record.
    let stored = unsafe { &*(internal_rep(source) as usize as *const NativeCommandNameRep) };
    change_type(
        duplicate,
        &NATIVE_COMMAND_NAME_TYPE,
        Box::into_raw(Box::new(stored.clone())) as usize as u64,
    );
}

static NATIVE_COMMAND_NAME_TYPE: TclObjType = TclObjType {
    name: c"cmdName".as_ptr(),
    free_int_rep_proc: Some(command_name_free),
    dup_int_rep_proc: Some(command_name_dup),
    update_string_proc: None,
    set_from_any_proc: None,
};

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
pub(crate) fn install_native_command_name_cache(
    value: *mut TclObj,
    cache: tcl_runtime_api::native_command_name::NativeCommandNameCache,
    dialect: tcl_registry::InvocationDialect,
) -> Result<(), tcl_syntax::value::ValueError> {
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
    change_type(
        value,
        &NATIVE_COMMAND_NAME_TYPE,
        Box::into_raw(Box::new(NativeCommandNameRep {
            version: cache.version,
            cache: Some(cache),
        })) as usize as u64,
    );
    Ok(())
}

/// Apply compile-time priming's selected early return to the original object.
pub(crate) fn prime_native_command_name_cache(
    value: *mut TclObj,
    incoming: tcl_runtime_api::native_command_name::NativeCommandNameCache,
    dialect: tcl_registry::InvocationDialect,
) -> Result<(), tcl_syntax::value::ValueError> {
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
    install_native_command_name_cache(value, incoming, dialect)
}

/// Apply the selected failed lookup's null descriptor effect; C9 keeps its primary.
pub(crate) fn install_unresolved_native_command_name_cache(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
) -> Result<(), tcl_syntax::value::ValueError> {
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
    change_type(
        value,
        &NATIVE_COMMAND_NAME_TYPE,
        Box::into_raw(Box::new(NativeCommandNameRep {
            version: protocol.version(),
            cache: None,
        })) as usize as u64,
    );
    Ok(())
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

/// Whether this original cache has a resident string or a native updater.
/// Jim completion codes with absent strings cannot manufacture a spelling.
pub(crate) fn native_string_available(value: *mut TclObj) -> bool {
    if value.is_null() || !allocation_is_live(value) {
        return false;
    }
    if has_string_rep(value) {
        return true;
    }
    let kind = obj_type_ptr(value);
    if core::ptr::eq(kind, &NATIVE_STRING_TYPE) {
        // SAFETY: the exact descriptor owns this live backing.
        let backing = unsafe { &*(internal_rep(value) as usize as *const NativeStringRep) };
        return backing.unicode.is_some() || backing.count == Some(0);
    }
    // A descriptor's actual updater, rather than its name, owns materialisation.
    kind.is_null() || unsafe { (*kind).update_string_proc.is_some() }
}

/// Inspect the physical cache without generating an object's string.
pub(crate) fn native_scalar_cache(
    value: *mut TclObj,
) -> Result<Option<tcl_syntax::scalar_getter::NativeScalarCache>, tcl_syntax::value::ValueError> {
    check_native_liveness(value)?;
    use tcl_syntax::{number::Number, scalar_getter::NativeScalarCache};
    let kind = obj_type_ptr(value);
    let cache = if core::ptr::eq(kind, &TCL_LONG84_TYPE) {
        Some(NativeScalarCache::Tcl84Long(wide_of(value)))
    } else if core::ptr::eq(kind, &TCL_INT_TYPE) {
        Some(NativeScalarCache::Number(Number::Int(wide_of(value))))
    } else if core::ptr::eq(kind, &TCL_DOUBLE_TYPE) {
        Some(NativeScalarCache::Number(Number::Double(double_of(value))))
    } else if core::ptr::eq(kind, &JIM_COERCED_DOUBLE_TYPE) {
        Some(NativeScalarCache::JimCoercedInteger(wide_of(value)))
    } else if native_word_boolean_origin(kind).is_some() {
        Some(NativeScalarCache::WordBoolean(wide_of(value) != 0))
    } else {
        #[cfg(have_tommath)]
        if core::ptr::eq(kind, &crate::bignum::TCL_BIGNUM_TYPE) {
            return crate::bignum::native_cached_number(value)
                .map(|number| Some(NativeScalarCache::Number(number)))
                .ok_or(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable);
        }
        None
    };
    Ok(cache)
}

/// Exact physical stock class for object-length dispatch. No spelling or
/// object type name authenticates a native cache.
pub(crate) fn stock_list_input_class(
    value: *mut TclObj,
) -> tcl_registry::native_stock_list::NativeStockListInputClass {
    use tcl_registry::native_stock_list::NativeStockListInputClass as Class;
    let kind = obj_type_ptr(value);
    if kind.is_null()
        || core::ptr::eq(kind, &JIM_STRING_TYPE)
        || core::ptr::eq(kind, &NATIVE_STRING_TYPE)
    {
        return Class::String;
    }
    if core::ptr::eq(kind, &crate::native_source::JIM_SOURCE_TYPE) {
        return Class::JimSource;
    }
    if core::ptr::eq(kind, &crate::list::TCL_LIST_TYPE) {
        return Class::List;
    }
    if core::ptr::eq(kind, &crate::dict::TCL_DICT_TYPE) {
        return Class::Dictionary;
    }
    if core::ptr::eq(kind, &NATIVE_COMMAND_NAME_TYPE) {
        return Class::CommandName;
    }
    if native_method_name::is_cached(value) {
        return Class::MethodName;
    }
    if native_property_name::is_cached(value) {
        return Class::PropertyName;
    }
    if with_jim_command_cache(value, |_| ()).is_some()
        || with_jim_variable_cache(value, |_| ()).is_some()
        || native_jim_enum::cache(value).is_some()
    {
        return Class::JimLookup;
    }
    if core::ptr::eq(kind, &NATIVE_ARRAY_SEARCH_TYPE) {
        return Class::ArraySearch;
    }
    if native_index::cache(value).is_some() || native_end_offset::cache(value).is_some() {
        return Class::Index;
    }
    if native_instruction_name::cache(value).is_some() {
        return Class::InstructionName;
    }
    if native_namespace_name::cache(value).is_some() {
        return Class::NamespaceName;
    }
    if native_variable_name::with_parsed(value, |_| ()).is_some() {
        return Class::ParsedVariableName;
    }
    if native_variable_name::with_local(value, |_| ()).is_some() {
        return Class::LocalVariableName;
    }
    if core::ptr::eq(kind, &crate::bytearray::TCL_BYTE_ARRAY_TYPE) {
        return Class::ByteArray;
    }
    if core::ptr::eq(kind, &TCL_INT_TYPE)
        || core::ptr::eq(kind, &TCL_LONG84_TYPE)
        || core::ptr::eq(kind, &TCL_DOUBLE_TYPE)
        || core::ptr::eq(kind, &JIM_COERCED_DOUBLE_TYPE)
    {
        return Class::Numeric;
    }
    if native_word_boolean_origin(kind).is_some() {
        return Class::Boolean;
    }
    #[cfg(have_tommath)]
    if core::ptr::eq(kind, &crate::bignum::TCL_BIGNUM_TYPE) {
        return Class::Numeric;
    }
    Class::Unknown
}

/// Apply the reached getter's complete cache change to the original object.
/// This conversion is observable even for shared objects and guest failures.
pub(crate) fn adopt_native_scalar_cache(
    value: *mut TclObj,
    cache: tcl_syntax::scalar_getter::NativeScalarCache,
    protocol: tcl_syntax::scalar_getter::NativeScalarGetterProtocol,
) -> Result<(), tcl_syntax::value::ValueError> {
    use tcl_syntax::{number::Number, scalar_getter::NativeScalarCache};
    match cache {
        NativeScalarCache::Tcl84Long(integer) => {
            if protocol.tcl_version() != Some(tcl_dialect::TclVersion::V8_4) {
                return Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable);
            }
            change_type(value, &TCL_LONG84_TYPE, integer as u64);
        }
        NativeScalarCache::Number(Number::Int(integer)) => {
            change_type(value, &TCL_INT_TYPE, integer as u64);
        }
        NativeScalarCache::Number(Number::Double(double)) => {
            change_type(value, &TCL_DOUBLE_TYPE, double.to_bits());
        }
        NativeScalarCache::Number(Number::Nan { negative, payload }) => {
            let bits = (u64::from(negative) << 63)
                | 0x7ff8_0000_0000_0000
                | (payload.unwrap_or(0) & 0x0007_ffff_ffff_ffff);
            change_type(value, &TCL_DOUBLE_TYPE, bits);
        }
        NativeScalarCache::Number(Number::Big {
            negative,
            radix,
            digits,
        }) => {
            #[cfg(have_tommath)]
            if crate::bignum::adopt_native_big(value, negative, radix, &digits) {
                return Ok(());
            }
            #[cfg(not(have_tommath))]
            let _ = (negative, radix, digits);
            return Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable);
        }
        NativeScalarCache::WordBoolean(boolean) => {
            let descriptor = match protocol.tcl_version() {
                Some(tcl_dialect::TclVersion::V8_4) => &NATIVE_WORD_BOOLEAN_84_TYPE,
                Some(tcl_dialect::TclVersion::V8_5) => &NATIVE_WORD_BOOLEAN_85_TYPE,
                Some(tcl_dialect::TclVersion::V8_6) => &NATIVE_WORD_BOOLEAN_86_TYPE,
                Some(tcl_dialect::TclVersion::V9_0) => &NATIVE_WORD_BOOLEAN_90_TYPE,
                Some(tcl_dialect::TclVersion::V9_1) => &NATIVE_WORD_BOOLEAN_91_TYPE,
                None => return Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable),
            };
            change_type(value, descriptor, u64::from(boolean));
        }
        NativeScalarCache::JimCoercedInteger(integer) => {
            change_type(value, &JIM_COERCED_DOUBLE_TYPE, integer as u64);
        }
    }
    Ok(())
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

/// `Tcl_DuplicateObj` — a fresh (`rc 0`) deep copy: the string rep (if any) plus
/// the internal rep (via the type's `dup_int_rep_proc`, or a raw copy for
/// self-contained reps like int/double).
pub(crate) fn duplicate(src: *mut TclObj) -> *mut TclObj {
    if crate::native_source::selected_string_protocol(src).is_some_and(|protocol| {
        // SAFETY: duplicate's caller supplies this live original header.
        let resident_length =
            unsafe { (!(*src).bytes.is_null()).then_some((*src).length as usize) };
        protocol.object_header_duplicate_action(resident_length)
            == tcl_syntax::native_string::NativeObjectHeaderDuplicateAction::CanonicalEmptyString
    }) {
        let duplicate = new_string_bytes(b"");
        crate::native_source::copy_context(src, duplicate);
        copy_scalar_object_context(src, duplicate);
        return duplicate;
    }
    let dup = obj_alloc();
    if dup.is_null() {
        return dup;
    }
    // SAFETY: `src` is live; `dup` is freshly owned and uniquely ours.
    unsafe {
        if !(*src).bytes.is_null() {
            if has_canonical_empty_string(src) {
                set_owned_string(dup, (*src).bytes as *const u8, (*src).length as usize);
            } else {
                set_allocated_string(dup, (*src).bytes as *const u8, (*src).length as usize);
            }
        }
        let tp = (*src).type_ptr;
        if !tp.is_null() {
            match (*tp).dup_int_rep_proc {
                Some(dup_proc) => dup_proc(src, dup), // e.g. list_dup deep-copies
                None => {
                    // self-contained rep (int/double): copy type + raw 8 bytes
                    (*dup).type_ptr = tp;
                    (*dup).internal_rep = (*src).internal_rep;
                    let cache = auxiliary(src).cache.borrow().clone();
                    auxiliary(dup).cache.replace(cache);
                }
            }
        }
    }
    crate::native_source::copy_context(src, dup);
    copy_scalar_object_context(src, dup);
    if let Some((file, line)) = script_location(src) {
        retain_script_location(dup, file, line);
    }
    dup
}

/// Replace one live object's representations with a duplicate, keeping its identity and refs.
pub(crate) fn duplicate_into(receiver: *mut TclObj, source: *mut TclObj) {
    let duplicate = duplicate(source);
    replace_scalar_object_context(receiver, source);
    // SAFETY: both objects are live, and duplicate owns independent string/type storage.
    unsafe {
        let receiver_refs = (*receiver).ref_count;
        core::ptr::swap(receiver, duplicate);
        let incoming_cache = auxiliary(duplicate).cache.take();
        let retired_cache = auxiliary(receiver).cache.replace(incoming_cache);
        auxiliary(duplicate).cache.replace(retired_cache);
        (*receiver).ref_count = receiver_refs;
        (*duplicate).ref_count = 0;
        obj_free(duplicate);
    }
    revoke_script_location(receiver);
    if let Some((file, line)) = script_location(source) {
        retain_script_location(receiver, file, line);
    }
}

/// Install the selected append String cache without invoking a previous updater.
pub(crate) fn set_native_append_string(
    value: *mut TclObj,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
    resident: Option<(
        std::rc::Rc<[u8]>,
        tcl_syntax::native_string::NativeStringStorageIdentity,
    )>,
    count: Option<usize>,
    unicode: Option<std::rc::Rc<[u32]>>,
) -> Result<(), tcl_syntax::value::ValueError> {
    use tcl_syntax::native_string::NativeStringStorageIdentity as Storage;
    use tcl_syntax::value::ValueError;
    if resident.as_ref().is_some_and(|(bytes, storage)| {
        *storage == Storage::Unknown || (*storage == Storage::CanonicalEmpty && !bytes.is_empty())
    }) {
        return Err(ValueError::CommandProtocolUnavailable(
            "native append resident storage",
        ));
    }
    if protocol.is_jim084() && (resident.is_none() || unicode.is_some()) {
        return Err(ValueError::CommandProtocolUnavailable(
            "Jim append String backing",
        ));
    }
    invalidate_string(value);
    if protocol.is_jim084() {
        change_type(
            value,
            &JIM_STRING_TYPE,
            count.map_or(u64::MAX, |count| count as u64),
        );
    } else {
        let backing = Box::new(NativeStringRep {
            protocol,
            count,
            unicode,
        });
        change_type(
            value,
            &NATIVE_STRING_TYPE,
            Box::into_raw(backing) as usize as u64,
        );
    }
    if let Some((bytes, storage)) = resident {
        // SAFETY: the original live object now owns a typed append String backing.
        unsafe { set_native_updater_string_rep(value, &bytes, storage == Storage::CanonicalEmpty) };
    }
    Ok(())
}

/// Whether `obj` is a plain string (no typed internal rep) — the precondition
/// for [`string_append_inplace`].
pub(crate) fn is_plain_string(obj: *mut TclObj) -> bool {
    // SAFETY: `obj` is a live object.
    unsafe { (*obj).type_ptr.is_null() }
}

/// Append `piece` to a **plain string** object in place, growing its buffer
/// geometrically (amortised O(1)). The caller must ensure `obj` is a
/// plain string ([`is_plain_string`]) and unshared. Refreshes `bytes`/`length`
/// and the capacity in `internal_rep`. A `realloc` keeps the single live-buffer
/// count, so the leak counters stay balanced.
pub(crate) fn string_append_inplace(obj: *mut TclObj, piece: &[u8]) {
    if piece.is_empty() {
        return;
    }
    revoke_script_location(obj);
    // SAFETY: caller guarantees a live, unshared, plain-string `obj` whose
    // buffer was allocated by `set_owned_string` (capacity in `internal_rep`).
    unsafe {
        if has_canonical_empty_string(obj) {
            set_owned_string(obj, piece.as_ptr(), piece.len());
            return;
        }
        let cur_len = (*obj).length as usize;
        let cur_cap = (*obj).internal_rep as usize; // allocated bytes incl. NUL
        let new_len = cur_len + piece.len();
        let need = new_len + 1; // + NUL terminator
        let mut buf = (*obj).bytes as *mut u8;
        if need > cur_cap {
            let new_cap = need.max(cur_cap.saturating_mul(2));
            let old_layout = Layout::from_size_align(cur_cap, 1).expect("buffer layout");
            let nb = realloc(buf, old_layout, new_cap);
            if nb.is_null() {
                counters::oom_set();
                return;
            }
            buf = nb;
            (*obj).bytes = buf as *mut c_char;
            (*obj).internal_rep = new_cap as u64;
        }
        core::ptr::copy_nonoverlapping(piece.as_ptr(), buf.add(cur_len), piece.len());
        *buf.add(new_len) = 0;
        (*obj).length = new_len as TclSize;
    }
}

/// `Tcl_NewWideIntObj` — pure int obj (no string rep yet). `fresh_zero`.
pub fn new_wide_int_obj(value: TclWideInt) -> *mut TclObj {
    let obj = obj_alloc();
    if obj.is_null() {
        return obj;
    }
    // SAFETY: freshly owned header.
    unsafe {
        (*obj).type_ptr = &TCL_INT_TYPE;
        (*obj).internal_rep = value as u64;
        (*obj).bytes = core::ptr::null_mut(); // shimmer on demand
        (*obj).length = 0;
    }
    obj
}

/// `Tcl_NewDoubleObj` — pure double obj (no string rep yet). `fresh_zero`.
pub fn new_double_obj(value: f64) -> *mut TclObj {
    let obj = obj_alloc();
    if obj.is_null() {
        return obj;
    }
    // SAFETY: freshly owned header.
    unsafe {
        (*obj).type_ptr = &TCL_DOUBLE_TYPE;
        (*obj).internal_rep = value.to_bits();
        (*obj).bytes = core::ptr::null_mut();
        (*obj).length = 0;
    }
    obj
}

/// `Tcl_NewBooleanObj` — booleans are int objs (0/1), as in Tcl. `fresh_zero`.
pub fn new_boolean_obj(value: i32) -> *mut TclObj {
    new_wide_int_obj(if value != 0 { 1 } else { 0 })
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

/// Decimal formatting of a signed 64-bit integer (Tcl's int string rep).
fn itoa(v: TclWideInt) -> Vec<u8> {
    if v == 0 {
        return vec![b'0'];
    }
    let neg = v < 0;
    let mut buf = Vec::new();
    // Work in u128 so i64::MIN's magnitude does not overflow.
    let mut n = (v as i128).unsigned_abs();
    while n > 0 {
        buf.push(b'0' + (n % 10) as u8);
        n /= 10;
    }
    if neg {
        buf.push(b'-');
    }
    buf.reverse();
    buf
}

#[cfg(test)]
mod script_location_tests {
    use super::*;

    #[test]
    fn retained_source_follows_object_lifetime_and_revokes_on_value_mutation() {
        let source = new_string_bytes(b"error BOOM");
        retain_script_location(source, Some(std::rc::Rc::from(b"source.tcl".as_slice())), 7);
        let copy = duplicate(source);
        assert_eq!(script_location(copy), script_location(source));
        string_append_inplace(copy, b" changed");
        assert!(script_location(copy).is_none());
        assert_eq!(script_location(source).unwrap().1, 7);
        // SAFETY: both fresh objects are retained once and released exactly once.
        unsafe {
            incr_ref_count(source);
            incr_ref_count(copy);
            decr_ref_count(source);
            decr_ref_count(copy);
        }
        assert!(script_location(source).is_none());
        assert!(script_location(copy).is_none());
    }
}

#[cfg(test)]
mod double_precision_tests {
    use crate::interp::{Code, Interp};
    use tcl_dialect::TclVersion;

    fn run(interp: &mut Interp, source: &str) -> String {
        assert_eq!(
            interp.eval_str(source.as_bytes()),
            Code::Ok,
            "{source}: {:?}",
            interp.result_bytes()
        );
        String::from_utf8(interp.result_bytes()).unwrap()
    }

    #[test]
    fn native_double_precision_is_lazy_shared_and_preserves_cached_strings() {
        for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
            let mut interp = Interp::new();
            interp.set_runtime_version(version);
            assert_eq!(
                run(
                    &mut interp,
                    "set ::tcl_precision 12; set x 1.0; set d [expr {$x/3}]; set ::tcl_precision 4; list $d"
                ),
                "0.3333"
            );
            assert_eq!(
                run(&mut interp, "set ::tcl_precision 12; list $d"),
                "0.3333"
            );
            assert_eq!(
                run(
                    &mut interp,
                    "interp create child; child eval {set ::tcl_precision 8}; list $::tcl_precision [expr {$x/3}]"
                ),
                "8 0.33333333"
            );
            assert_eq!(
                run(
                    &mut interp,
                    "set d [expr {$x/7}]; child eval {set ::tcl_precision 12}; list $d"
                ),
                "0.142857142857"
            );
            assert_eq!(
                run(
                    &mut interp,
                    "unset ::tcl_precision; list [info exists ::tcl_precision] $::tcl_precision"
                ),
                "1 12"
            );
            assert_eq!(
                run(
                    &mut interp,
                    "proc precision_alias {} {upvar #0 ::tcl_precision p; set p 4}; precision_alias; list $::tcl_precision"
                ),
                "4"
            );
            assert_eq!(
                run(
                    &mut interp,
                    "catch {set ::tcl_precision 18} problem; list $problem $::tcl_precision"
                ),
                "{can't set \"::tcl_precision\": improper value for precision} 4"
            );
        }
    }

    #[test]
    fn binary_scan_doubles_materialise_only_at_the_selected_precision() {
        for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
            let mut interp = Interp::new();
            interp.set_runtime_version(version);
            assert_eq!(
                run(
                    &mut interp,
                    "set ::tcl_precision 12; set x 1.0; set bytes [binary format d [expr {$x/3}]]; binary scan $bytes d d; set ::tcl_precision 4; list $d"
                ),
                "0.3333"
            );
            assert_eq!(
                run(
                    &mut interp,
                    "set ::tcl_precision 12; binary scan $bytes d* ds; set ::tcl_precision 8; list [lindex $ds 0]"
                ),
                "0.33333333"
            );
            assert_eq!(
                run(
                    &mut interp,
                    "set ::tcl_precision 4; set original [expr {$x/3}]; binary scan [binary format d $original] d copy; set ::tcl_precision 12; list $original $copy"
                ),
                "0.333333333333 0.333333333333"
            );
            assert_eq!(
                run(
                    &mut interp,
                    "set ::tcl_precision 4; set original [expr {$x/3}]; binary scan [binary format d* [list $original]] d* copies; set ::tcl_precision 12; list $original [lindex $copies 0]"
                ),
                "0.333333333333 0.333333333333"
            );
        }
    }

    #[test]
    fn native_precision_array_elements_share_the_hidden_trace_state() {
        for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
            let mut interp = Interp::new();
            interp.set_runtime_version(version);
            assert_eq!(
                run(
                    &mut interp,
                    "set ::tcl_precision 12; unset ::tcl_precision; set ::tcl_precision(x) 4; set x 1.0; list [expr {$x/3}] $::tcl_precision(x)"
                ),
                "0.3333 4"
            );
            assert_eq!(
                run(
                    &mut interp,
                    "set ::tcl_precision(x) 12; upvar #0 ::tcl_precision(x) p; set p 4; list [expr {$x/3}] $p $::tcl_precision(x)"
                ),
                "0.333333333333 4 12"
            );
            assert_eq!(
                run(
                    &mut interp,
                    "proc adjust {n1 n2 op} {set ::tcl_precision(x) 8}; trace variable ::tcl_precision(x) w adjust; set ::tcl_precision(x) 4; list [expr {$x/3}] $::tcl_precision(x)"
                ),
                "0.3333 4"
            );
        }
    }

    #[test]
    fn native_fixed_double_policies_ignore_ordinary_precision_variables() {
        for profile in ["tcl9.0", "tcl9.1", "jim"] {
            let mut interp = Interp::new();
            interp.set_dialect_profile(crate::environment::profile_for_dialect(profile));
            let expected = if profile == "jim" {
                "0.333333333333"
            } else {
                "0.3333333333333333"
            };
            assert_eq!(
                run(
                    &mut interp,
                    "set tcl_precision 4; set x 1.0; list [expr {$x/3}]"
                ),
                expected
            );
        }
    }
}

/// Genuine original Jim Enum and immediate literal cache storage.
pub(crate) mod native_jim_enum;
pub(crate) mod native_method_name;
pub(crate) mod native_property_name;
