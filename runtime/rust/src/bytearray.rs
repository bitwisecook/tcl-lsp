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

//! Tcl's byte-array object type.
//!
//! A byte array has two intentionally different views, just like C Tcl's
//! `bytearray` `Tcl_ObjType`: an exact raw payload for `binary` commands, and a
//! Unicode string representation for ordinary string operations. Keeping both
//! avoids treating invalid UTF-8 as the string representation itself. The
//! separation matters when a string command changes a byte's Unicode character
//! into a code point that cannot be converted back to a byte under Tcl 9.

#[cfg(test)]
use tcl_dialect::ByteStringEncoding;

use crate::obj::{self, TclObj, TclObjType};

/// The backing allocation for a byte-array internal representation.
struct TclByteArray {
    bytes: Vec<u8>,
    conversion: Option<tcl_registry::native_binary_value::NativeBinaryByteConversion>,
    proper: bool,
    recipe: tcl_registry::native_string_materialization::ByteArrayStringRecipe,
}

/// C Tcl's built-in `bytearray` object type.
pub static TCL_BYTE_ARRAY_TYPE: TclObjType = TclObjType {
    name: c"bytearray".as_ptr(),
    free_int_rep_proc: Some(byte_array_free),
    dup_int_rep_proc: Some(byte_array_dup),
    update_string_proc: Some(byte_array_update_string),
    set_from_any_proc: None,
};

/// An error converting a Unicode string representation to binary bytes.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ByteConversionError {
    /// Byte offset in the UTF-8 string representation where conversion fails.
    pub(crate) byte_offset: usize,
    /// Unicode code point which does not fit in Tcl 9's byte string domain.
    pub(crate) code_point: u32,
}

unsafe fn byte_array_ref<'a>(obj: *mut TclObj) -> &'a TclByteArray {
    // SAFETY: callers establish `obj` has `TCL_BYTE_ARRAY_TYPE`, whose
    // internal rep is a live `Box<TclByteArray>` allocated below.
    unsafe { &*(obj::internal_rep(obj) as usize as *const TclByteArray) }
}

extern "C" fn byte_array_free(obj: *mut TclObj) {
    // SAFETY: invoked exactly once while freeing an object of this type.
    unsafe {
        let p = obj::internal_rep(obj) as usize as *mut TclByteArray;
        if !p.is_null() {
            drop(Box::from_raw(p));
        }
    }
}

extern "C" fn byte_array_dup(src: *mut TclObj, dup: *mut TclObj) {
    // SAFETY: `src` has this type; `dup` is a fresh object owned by
    // `Tcl_DuplicateObj`. `change_type` assumes responsibility for the box.
    unsafe {
        let copied = Box::new(TclByteArray {
            bytes: byte_array_ref(src).bytes.clone(),
            conversion: byte_array_ref(src).conversion,
            proper: byte_array_ref(src).proper,
            recipe: byte_array_ref(src).recipe,
        });
        obj::change_type(
            dup,
            &TCL_BYTE_ARRAY_TYPE,
            Box::into_raw(copied) as usize as u64,
        );
    }
}

extern "C" fn byte_array_update_string(value: *mut TclObj) {
    // The backing recipe is mandatory and sealed before allocation. The callback
    // needs no interpreter lookup and cannot guess an engine from cache names.
    unsafe {
        let backing = byte_array_ref(value);
        let string = backing
            .recipe
            .protocol()
            .materialize(tcl_syntax::native_string::NativeStringInput::PureByteArray(
                &backing.bytes,
            ))
            .expect("sealed C byte-array string recipe");
        obj::set_native_updater_string_rep(value, &string, backing.recipe.canonical_empty());
    }
}

/// Create a fresh byte-array object with an exact raw payload.
pub(crate) fn new_byte_array(
    bytes: &[u8],
    recipe: tcl_registry::native_string_materialization::ByteArrayStringRecipe,
) -> *mut TclObj {
    let backing = Box::new(TclByteArray {
        bytes: bytes.to_vec(),
        conversion: None,
        proper: true,
        recipe,
    });
    obj::alloc_typed(&TCL_BYTE_ARRAY_TYPE, Box::into_raw(backing) as usize as u64)
}

/// The mandatory native string recipe retained by this physical backing.
pub(crate) fn native_string_protocol(
    value: *mut TclObj,
) -> Option<tcl_syntax::native_string::NativeStringProtocol> {
    (obj::obj_type_ptr(value) == &TCL_BYTE_ARRAY_TYPE)
        .then(|| unsafe { byte_array_ref(value) }.recipe.protocol())
}

/// Install native append binary backing on the prepared original receiver.
pub(crate) fn set_native_append_bytes(
    value: *mut TclObj,
    bytes: &[u8],
    recipe: tcl_registry::native_string_materialization::ByteArrayStringRecipe,
) {
    let backing = Box::new(TclByteArray {
        bytes: bytes.to_vec(),
        conversion: None,
        proper: true,
        recipe,
    });
    obj::invalidate_string(value);
    obj::change_type(
        value,
        &TCL_BYTE_ARRAY_TYPE,
        Box::into_raw(backing) as usize as u64,
    );
}

/// Materialise a pure byte-array string for the selected primitive getter.
/// An existing string is original input and is never regenerated.
pub(crate) fn scalar_getter_string(
    value: *mut TclObj,
    protocol: tcl_syntax::scalar_getter::NativeScalarGetterProtocol,
) -> Option<Vec<u8>> {
    if !obj::native_string_available(value) {
        return None;
    }
    if obj::has_string_rep(value) {
        return Some(obj::bytes_of(value));
    }
    if obj::obj_type_ptr(value) != &TCL_BYTE_ARRAY_TYPE {
        let selected = protocol.tcl_version().map_or(
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
            tcl_syntax::native_string::NativeStringProtocol::C,
        );
        return crate::dict::native_object_bytes(value, selected).ok();
    }
    // SAFETY: the exact byte-array type owns this backing allocation.
    let backing = unsafe { byte_array_ref(value) };
    // The getter still independently refuses unsupported actual storage. The
    // object's retained updater recipe owns physical materialization.
    protocol.materialize(
        tcl_syntax::scalar_getter::NativeScalarStringStorage::ByteArray,
        &backing.bytes,
    )?;
    Some(obj::bytes_of(value))
}

/// Query the selected native byte-array length shortcut before generating a
/// string representation, which would change the object's purity.
pub(crate) fn native_string_length(
    value: *mut TclObj,
    policy: tcl_registry::native_string_length::NativeStringLengthRepresentation,
) -> Option<usize> {
    if obj::obj_type_ptr(value) != &TCL_BYTE_ARRAY_TYPE {
        return None;
    }
    // SAFETY: the actual byte-array type owns this live backing allocation.
    let backing = unsafe { byte_array_ref(value) };
    policy
        .counts_byte_array(!obj::has_string_rep(value), backing.proper)
        .then_some(backing.bytes.len())
}

/// Inspect the exact binary backing without generating its string cache.
pub(crate) fn native_cache_snapshot(value: *mut TclObj) -> Option<(std::rc::Rc<[u8]>, bool)> {
    if obj::obj_type_ptr(value) != &TCL_BYTE_ARRAY_TYPE {
        return None;
    }
    // SAFETY: the exact descriptor owns this live backing.
    let backing = unsafe { byte_array_ref(value) };
    Some((std::rc::Rc::from(backing.bytes.as_slice()), backing.proper))
}

/// Project a live native binary operand under the selected interpreter policy.
/// A checked failure leaves the original object and its bytes intact.
pub(crate) fn native_binary_bytes(
    value: *mut TclObj,
    policy: tcl_registry::native_binary_value::NativeBinaryByteConversion,
    cache: bool,
    recipe: tcl_registry::native_string_materialization::ByteArrayStringRecipe,
) -> Result<Vec<u8>, tcl_registry::native_binary_value::NativeBinaryByteError> {
    use tcl_registry::native_binary_value::NativeBinaryByteConversion;
    if policy == NativeBinaryByteConversion::Utf8 {
        return Ok(obj::bytes_of(value));
    }
    if obj::obj_type_ptr(value) == &TCL_BYTE_ARRAY_TYPE {
        // SAFETY: the exact type pointer owns this backing allocation.
        let backing = unsafe { byte_array_ref(value) };
        if (backing.conversion.is_none() || backing.conversion == Some(policy))
            && (backing.proper || policy != NativeBinaryByteConversion::CheckedLatin1)
        {
            return Ok(backing.bytes.clone());
        }
    }
    let original = obj::bytes_of(value);
    let units = tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(
        recipe
            .protocol()
            .tcl_version()
            .expect("sealed C byte-array recipe"),
    );
    let bytes = policy.convert_native(&original, units)?;
    if cache {
        let backing = Box::new(TclByteArray {
            bytes: bytes.clone(),
            conversion: Some(policy),
            proper: tcl_registry::native_binary_value::NativeBinaryByteConversion::CheckedLatin1
                .convert_native(&original, units)
                .is_ok(),
            recipe,
        });
        // SAFETY: native conversion replaces the live representation while
        // preserving its existing string bytes, including on shared objects.
        obj::change_type(
            value,
            &TCL_BYTE_ARRAY_TYPE,
            Box::into_raw(backing) as usize as u64,
        );
    }
    Ok(bytes)
}

/// Select decoder input from its actual object representation, before any
/// string access can materialise a pure byte array's string representation.
pub(crate) fn native_decode_input(
    value: *mut TclObj,
    policy: tcl_registry::native_binary_value::NativeBinaryDecodeSource,
    recipe: tcl_registry::native_string_materialization::ByteArrayStringRecipe,
) -> tcl_registry::native_binary_value::NativeBinaryDecodeInput {
    use tcl_registry::native_binary_value::{
        NativeBinaryByteConversion, NativeBinaryDecodeInput, NativeBinaryDecodeSource,
    };
    if obj::obj_type_ptr(value) == &TCL_BYTE_ARRAY_TYPE
        && ((policy == NativeBinaryDecodeSource::ProperByteArrayOrString
            && unsafe { byte_array_ref(value).proper })
            || (policy == NativeBinaryDecodeSource::PureByteArrayOrString
                && !obj::has_string_rep(value)))
    {
        // SAFETY: the exact type pointer owns this backing allocation.
        return NativeBinaryDecodeInput::Bytes(unsafe { byte_array_ref(value).bytes.clone() });
    }
    if policy == NativeBinaryDecodeSource::ProperByteArrayOrString {
        if let Ok(bytes) = native_binary_bytes(
            value,
            NativeBinaryByteConversion::CheckedLatin1,
            true,
            recipe,
        ) {
            return NativeBinaryDecodeInput::Bytes(bytes);
        }
    }
    let original = obj::bytes_of(value);
    match String::from_utf8(original) {
        Ok(text) => NativeBinaryDecodeInput::String(text),
        Err(error) => NativeBinaryDecodeInput::Bytes(error.into_bytes()),
    }
}

/// Return an object's bytes as a `binary` command would consume them.
///
/// A real byte-array takes the raw-payload branch. Every other object is first
/// read through its string representation, then converted by the selected Tcl
/// release policy. That keeps byte-array identity separate from a same-looking
/// Unicode string, which is the essential dual-representation distinction.
#[cfg(test)]
pub(crate) fn binary_bytes(
    obj: *mut TclObj,
    policy: ByteStringEncoding,
) -> Result<Vec<u8>, ByteConversionError> {
    if obj::obj_type_ptr(obj) == &TCL_BYTE_ARRAY_TYPE {
        // SAFETY: the type-pointer equality above establishes the backing type.
        return Ok(unsafe { byte_array_ref(obj).bytes.clone() });
    }

    let string = obj::bytes_of(obj);
    let Ok(text) = core::str::from_utf8(&string) else {
        // An embedding may hand the C ABI a non-UTF-8 plain string. Preserve
        // those bytes rather than introducing replacement characters; all
        // runtime-created binary results use the typed branch above.
        return Ok(string);
    };
    let mut out = Vec::with_capacity(text.len());
    for (byte_offset, ch) in text.char_indices() {
        let code_point = u32::from(ch);
        if code_point <= u32::from(u8::MAX) {
            out.push(u8::try_from(code_point).expect("checked byte code point"));
            continue;
        }
        match policy {
            ByteStringEncoding::LegacyTruncate => out.push(code_point.to_le_bytes()[0]),
            ByteStringEncoding::CheckedLatin1 => {
                return Err(ByteConversionError {
                    byte_offset,
                    code_point,
                });
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{binary_bytes, new_byte_array};
    use crate::counters;
    use crate::obj;
    use tcl_dialect::ByteStringEncoding;

    #[test]
    fn byte_array_keeps_raw_payload_while_generating_a_unicode_string_rep() {
        counters::reset();
        let recipe = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0)
            .byte_array_string_recipe(None)
            .unwrap();
        let obj = new_byte_array(&[b'A', 0xFF, b'B'], recipe);
        assert_eq!(
            binary_bytes(obj, ByteStringEncoding::CheckedLatin1),
            Ok(vec![b'A', 0xFF, b'B'])
        );
        assert_eq!(obj::bytes_of(obj), "A\u{00ff}B".as_bytes());
        // A duplicated object keeps both ports independent and exact.
        let dup = obj::duplicate(obj);
        assert_eq!(
            binary_bytes(dup, ByteStringEncoding::CheckedLatin1),
            Ok(vec![b'A', 0xFF, b'B'])
        );
        unsafe {
            obj::incr_ref_count(obj);
            obj::decr_ref_count(obj);
            obj::incr_ref_count(dup);
            obj::decr_ref_count(dup);
        }
        assert_eq!(counters::finalize(), 0);
    }

    #[test]
    fn byte_array_updater_retains_engine_recipe_and_actual_empty_storage_identity() {
        use tcl_syntax::value::ValueOps;
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = tcl_registry::InvocationDialect::for_version(version);
            let recipe = dialect.byte_array_string_recipe(None).unwrap();
            let value = obj::Owned::fresh(new_byte_array(&[0, 0x80, 0xff], recipe));
            assert!(!obj::has_string_rep(value.as_ptr()));
            let duplicate = obj::Owned::fresh(obj::duplicate(value.as_ptr()));
            assert_eq!(obj::bytes_of(value.as_ptr()), b"\xc0\x80\xc2\x80\xc3\xbf");
            assert_eq!(
                obj::bytes_of(duplicate.as_ptr()),
                b"\xc0\x80\xc2\x80\xc3\xbf"
            );
            assert_eq!(
                binary_bytes(value.as_ptr(), ByteStringEncoding::CheckedLatin1),
                Ok(vec![0, 0x80, 0xff])
            );
            let empty = obj::Owned::fresh(new_byte_array(b"", recipe));
            assert!(!obj::has_string_rep(empty.as_ptr()));
            assert!(obj::bytes_of(empty.as_ptr()).is_empty());
            assert_eq!(
                obj::has_canonical_empty_string(empty.as_ptr()),
                version >= tcl_dialect::TclVersion::V9_0
            );
            let copied_empty = obj::Owned::fresh(obj::duplicate(empty.as_ptr()));
            assert_eq!(
                obj::has_canonical_empty_string(copied_empty.as_ptr()),
                obj::has_canonical_empty_string(empty.as_ptr())
            );
            let mut interp = crate::interp::Interp::new();
            interp.set_runtime_version(version);
            assert_eq!(interp.list_len(&empty.as_ptr()).unwrap(), 0);
            assert_eq!(
                obj::obj_type_ptr(empty.as_ptr()) == &super::TCL_BYTE_ARRAY_TYPE,
                version >= tcl_dialect::TclVersion::V9_0
            );
            let pure_empty = obj::Owned::fresh(new_byte_array(b"", recipe));
            assert_eq!(interp.list_len(&pure_empty.as_ptr()).unwrap(), 0);
            assert_eq!(
                obj::obj_type_ptr(pure_empty.as_ptr()),
                &crate::list::TCL_LIST_TYPE as *const _
            );
        }
    }

    #[test]
    fn resident_byte_array_string_bytes_are_never_regenerated() {
        let recipe = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0)
            .byte_array_string_recipe(None)
            .unwrap();
        let value = obj::Owned::fresh(new_byte_array(b"different", recipe));
        let resident = b"A\0\xc0\x80\xff";
        unsafe {
            obj::set_string_rep(value.as_ptr(), resident);
        }
        assert_eq!(obj::bytes_of(value.as_ptr()), resident);
        let duplicate = obj::Owned::fresh(obj::duplicate(value.as_ptr()));
        assert_eq!(obj::bytes_of(duplicate.as_ptr()), resident);
        assert_eq!(
            binary_bytes(value.as_ptr(), ByteStringEncoding::CheckedLatin1),
            Ok(b"different".to_vec())
        );
    }

    #[test]
    fn canonical_and_allocated_empty_buffers_have_distinct_lifetimes() {
        let canonical = obj::Owned::fresh(obj::new_obj());
        assert!(obj::has_canonical_empty_string(canonical.as_ptr()));
        let copied = obj::Owned::fresh(obj::duplicate(canonical.as_ptr()));
        obj::string_append_inplace(canonical.as_ptr(), b"x\0\xff");
        assert_eq!(obj::bytes_of(canonical.as_ptr()), b"x\0\xff");
        assert!(obj::has_canonical_empty_string(copied.as_ptr()));
        let allocated = obj::Owned::fresh(obj::new_obj());
        unsafe {
            obj::set_native_updater_string_rep(allocated.as_ptr(), b"", false);
        }
        assert!(!obj::has_canonical_empty_string(allocated.as_ptr()));
        let duplicate = obj::Owned::fresh(obj::duplicate(allocated.as_ptr()));
        assert!(!obj::has_canonical_empty_string(duplicate.as_ptr()));
        obj::string_append_inplace(allocated.as_ptr(), b"y");
        assert_eq!(obj::bytes_of(allocated.as_ptr()), b"y");
        assert_eq!(obj::bytes_of(duplicate.as_ptr()), b"");
    }

    #[test]
    fn unavailable_byte_array_factory_refuses_without_replacing_guest_result() {
        let mut interp = crate::interp::Interp::new();
        interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
        interp.set_result_bytes(b"retained\0\xff");
        let original = interp.get_obj_result();
        assert_eq!(
            interp.set_result_byte_array(b"new"),
            crate::interp::Code::Error
        );
        assert_eq!(interp.get_obj_result(), original);
        assert_eq!(interp.result_bytes(), b"retained\0\xff");
    }

    #[test]
    fn unicode_string_to_byte_conversion_is_release_gated() {
        counters::reset();
        let obj = obj::new_string_bytes("\u{0178}".as_bytes());
        assert_eq!(
            binary_bytes(obj, ByteStringEncoding::LegacyTruncate),
            Ok(vec![b'x'])
        );
        assert_eq!(
            binary_bytes(obj, ByteStringEncoding::CheckedLatin1),
            Err(super::ByteConversionError {
                byte_offset: 0,
                code_point: 0x178,
            })
        );
        unsafe {
            obj::incr_ref_count(obj);
            obj::decr_ref_count(obj);
        }
        assert_eq!(counters::finalize(), 0);
    }
}
