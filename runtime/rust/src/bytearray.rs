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

use tcl_dialect::ByteStringEncoding;

use crate::obj::{self, TclObj, TclObjType};

/// The backing allocation for a byte-array internal representation.
struct TclByteArray {
    bytes: Vec<u8>,
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
        });
        obj::change_type(
            dup,
            &TCL_BYTE_ARRAY_TYPE,
            Box::into_raw(copied) as usize as u64,
        );
    }
}

extern "C" fn byte_array_update_string(obj: *mut TclObj) {
    // Tcl exposes one U+00XX character for each byte when a byte-array is used
    // as a string. The raw payload remains in the internal rep; this UTF-8
    // buffer is only the lazily generated string representation.
    unsafe {
        let string: String = byte_array_ref(obj)
            .bytes
            .iter()
            .copied()
            .map(char::from)
            .collect();
        obj::set_string_rep(obj, string.as_bytes());
    }
}

/// Create a fresh byte-array object with an exact raw payload.
pub(crate) fn new_byte_array(bytes: &[u8]) -> *mut TclObj {
    let backing = Box::new(TclByteArray {
        bytes: bytes.to_vec(),
    });
    obj::alloc_typed(&TCL_BYTE_ARRAY_TYPE, Box::into_raw(backing) as usize as u64)
}

/// Return an object's bytes as a `binary` command would consume them.
///
/// A real byte-array takes the raw-payload branch. Every other object is first
/// read through its string representation, then converted by the selected Tcl
/// release policy. That keeps byte-array identity separate from a same-looking
/// Unicode string, which is the essential dual-representation distinction.
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
        let obj = new_byte_array(&[b'A', 0xFF, b'B']);
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
