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

//! The integer, double and word-boolean object types: constructors, string updaters, double precision, and the scalar caches a getter reads and adopts.

use super::internal_representation::{change_type, change_type_using, obj_type_ptr};
use super::lookup_caches::{NATIVE_ARRAY_SEARCH_TYPE, NATIVE_COMMAND_NAME_TYPE};
use super::native_jim_lookup::{
    with_command_cache as with_jim_command_cache, with_variable_cache as with_jim_variable_cache,
};
use super::string_representations::NATIVE_STRING_TYPE;
use super::{
    bytes_of, check_native_liveness, has_string_rep, is_shared, native_end_offset, native_index,
    native_instruction_name, native_jim_enum, native_jim_subcommand, native_method_name,
    native_namespace_name, native_property_name, native_variable_name, obj_alloc,
    scalar_object_context, set_owned_string, TclObj, TclObjType, TclWideInt, DOUBLE_PRECISIONS,
    DOUBLE_STRING_POLICY, JIM_STRING_TYPE, LEGACY_EXTENSION_TYPES,
};

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

/// C Tcl 8.4's wideInt primary, independent of the selected machine Long width.
/// Only an actual selected scalar issuer/adoption installs this descriptor.
pub(crate) static TCL_WIDE84_TYPE: TclObjType = TclObjType {
    name: c"wideInt".as_ptr(),
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

pub(super) extern "C" fn int_update_string(obj: *mut TclObj) {
    // SAFETY: `obj` is a live int object whose string rep needs generating.
    unsafe {
        let s = tcl_syntax::number::native_integer_string(
            (*obj).wide(),
            tcl_syntax::number::runtime_release()
                .is_some_and(tcl_dialect::TclVersion::garbles_wide_min),
        );
        set_owned_string(obj, s.as_ptr(), s.len());
    }
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

pub(super) extern "C" fn double_update_string(obj: *mut TclObj) {
    // SAFETY: `obj` is a live double object. The release's Tcl double→string is
    // the shared `tcl_syntax::number::print_double` — integer-valued
    // doubles get `.0`, plus `Inf`/`NaN`.
    unsafe {
        let policy = DOUBLE_STRING_POLICY.get();
        let format = policy
            .format(double_precision(policy))
            .expect("validated native precision");
        let s = tcl_syntax::number::format_double_native_selected((*obj).double(), policy, format);
        set_owned_string(obj, s.as_ptr(), s.len());
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

pub(super) fn native_word_boolean_origin(
    kind: *const TclObjType,
) -> Option<tcl_dialect::TclVersion> {
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

/// Retained descriptor origin; this observation supplies no getter authority.
pub(crate) fn native_scalar_descriptor_origin(
    value: *mut TclObj,
) -> Option<tcl_dialect::TclVersion> {
    if core::ptr::eq(obj_type_ptr(value), &TCL_WIDE84_TYPE) {
        Some(tcl_dialect::TclVersion::V8_4)
    } else {
        native_word_boolean_version(value)
    }
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
    } else if core::ptr::eq(kind, &TCL_INT_TYPE) || core::ptr::eq(kind, &TCL_WIDE84_TYPE) {
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
        || native_jim_subcommand::cache(value).is_some()
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
        || core::ptr::eq(kind, &TCL_WIDE84_TYPE)
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
    if LEGACY_EXTENSION_TYPES.with(|types| types.borrow().contains(&(kind as usize))) {
        Class::LegacyExtension
    } else {
        Class::Unknown
    }
}

/// Apply the reached getter's complete cache change to the original object.
/// This conversion is observable even for shared objects and guest failures.
pub(crate) fn adopt_native_scalar_cache(
    value: *mut TclObj,
    cache: tcl_syntax::scalar_getter::NativeScalarCache,
    protocol: tcl_syntax::scalar_getter::NativeScalarGetterProtocol,
) -> Result<(), tcl_syntax::value::ValueError> {
    adopt_native_scalar_cache_using(value, cache, protocol, None)
}

pub(crate) fn adopt_native_scalar_cache_with_stage_check(
    value: *mut TclObj,
    cache: tcl_syntax::scalar_getter::NativeScalarCache,
    protocol: tcl_syntax::scalar_getter::NativeScalarGetterProtocol,
    check: &mut impl FnMut() -> Result<(), tcl_syntax::value::ValueError>,
) -> Result<(), tcl_syntax::value::ValueError> {
    let original = scalar_object_context(value)?;
    let mut current = || {
        check()?;
        let now = scalar_object_context(value)?;
        if match (&original, &now) {
            (Some(original), Some(now)) => !original.same_issuer(now),
            (None, None) => false,
            _ => true,
        } {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "changed native scalar cache issuer",
            ));
        }
        Ok(())
    };
    adopt_native_scalar_cache_using(value, cache, protocol, Some(&mut current))
}

fn adopt_native_scalar_cache_using(
    value: *mut TclObj,
    cache: tcl_syntax::scalar_getter::NativeScalarCache,
    protocol: tcl_syntax::scalar_getter::NativeScalarGetterProtocol,
    check: Option<&mut dyn FnMut() -> Result<(), tcl_syntax::value::ValueError>>,
) -> Result<(), tcl_syntax::value::ValueError> {
    use tcl_syntax::{number::Number, scalar_getter::NativeScalarCache};
    check_native_liveness(value)?;
    if let Some(issuer) = scalar_object_context(value)? {
        issuer.validate_cache(&cache, protocol)?;
    }
    match cache {
        NativeScalarCache::Tcl84Long(integer) => {
            if protocol.tcl_version() != Some(tcl_dialect::TclVersion::V8_4) {
                return Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable);
            }
            change_type_using(value, &TCL_LONG84_TYPE, integer as u64, check)?;
        }
        NativeScalarCache::Number(Number::Int(integer)) => {
            let descriptor = if protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4)
                && (check.is_some() || scalar_object_context(value)?.is_some())
            {
                &TCL_WIDE84_TYPE
            } else {
                &TCL_INT_TYPE
            };
            change_type_using(value, descriptor, integer as u64, check)?;
        }
        NativeScalarCache::Number(Number::Double(double)) => {
            change_type_using(value, &TCL_DOUBLE_TYPE, double.to_bits(), check)?;
        }
        NativeScalarCache::Number(Number::Nan { negative, payload }) => {
            let bits = (u64::from(negative) << 63)
                | 0x7ff8_0000_0000_0000
                | (payload.unwrap_or(0) & 0x0007_ffff_ffff_ffff);
            change_type_using(value, &TCL_DOUBLE_TYPE, bits, check)?;
        }
        NativeScalarCache::Number(Number::Big {
            negative,
            radix,
            digits,
        }) => {
            #[cfg(have_tommath)]
            if if let Some(check) = check {
                crate::bignum::adopt_native_big_using(value, negative, radix, &digits, Some(check))?
            } else {
                crate::bignum::adopt_native_big(value, negative, radix, &digits)
            } {
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
            change_type_using(value, descriptor, u64::from(boolean), check)?;
        }
        NativeScalarCache::JimCoercedInteger(integer) => {
            change_type_using(value, &JIM_COERCED_DOUBLE_TYPE, integer as u64, check)?;
        }
    }
    Ok(())
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
