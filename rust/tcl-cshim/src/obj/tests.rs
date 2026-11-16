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

use super::{NativeStringStorageIdentity, Rep};
use std::cell::Cell;

use super::{Obj, ObjRef};
use tcl_engine_api::Value;

thread_local! {
    /// How many objects have been destroyed on this thread.
    pub(super) static DESTROYED: Cell<usize> = const { Cell::new(0) };
}

// Over `void *`: an `extern` block naming `Obj`, whose private state is not
// C's, would trip the FFI-safety lint.
#[cfg(cshim_c_tests)]
unsafe extern "C" {
    static tclshim_test_layout: [usize; 7];
    fn tclshim_test_bytes_of(obj: *mut std::ffi::c_void) -> *const std::ffi::c_char;
    fn tclshim_test_length_of(obj: *mut std::ffi::c_void) -> isize;
    fn tclshim_test_ref_count_of(obj: *mut std::ffi::c_void) -> isize;
    fn tclshim_test_shared_of(obj: *mut std::ffi::c_void) -> std::ffi::c_int;
    fn tclshim_test_incr_of(obj: *mut std::ffi::c_void);
    fn tclshim_test_decr_of(obj: *mut std::ffi::c_void);
}

#[cfg(cshim_c_tests)]
fn c_obj(obj: *mut Obj) -> *mut std::ffi::c_void {
    obj.cast()
}

/// The prefix the header declares is the prefix `Obj` has: `Tcl_Obj`'s fields
/// at their declared offsets, then room for the union, and only then the
/// shim's own state.
#[cfg(cshim_c_tests)]
#[test]
fn the_declared_prefix_is_the_layout_the_header_declares() {
    use std::mem::offset_of;
    // SAFETY: a constant array the C compiler emitted.
    let [
        size,
        ref_count,
        bytes,
        length,
        type_ptr,
        internal_rep,
        union_size,
    ] = unsafe { tclshim_test_layout };
    assert_eq!(offset_of!(Obj, refcount), ref_count);
    assert_eq!(offset_of!(Obj, bytes), bytes);
    assert_eq!(offset_of!(Obj, length), length);
    assert_eq!(offset_of!(Obj, type_ptr), type_ptr);
    assert_eq!(offset_of!(Obj, internal_rep), internal_rep);
    assert_eq!(
        size_of::<std::cell::Cell<[usize; 2]>>(),
        union_size,
        "room for the union"
    );
    assert_eq!(
        offset_of!(Obj, string),
        size,
        "the shim's own state starts where the declared struct ends"
    );
}

/// C reads `bytes` and `length` straight from the object: they follow the
/// string rep, and are null and zero while there is none.
#[cfg(cshim_c_tests)]
#[test]
fn the_fields_c_reads_follow_the_string_rep() {
    let raw = Obj::from_text("héllo").into_raw();
    // SAFETY: `raw` is a live object; the functions only read it.
    unsafe {
        assert_eq!(
            tclshim_test_length_of(c_obj(raw)),
            6,
            "bytes, not characters"
        );
        let bytes = std::slice::from_raw_parts(tclshim_test_bytes_of(c_obj(raw)).cast::<u8>(), 7);
        assert_eq!(bytes, "héllo\0".as_bytes());
        Obj::free(raw);
    }

    let number = Obj::int(42).into_raw();
    // SAFETY: as above.
    unsafe {
        assert!(
            tclshim_test_bytes_of(c_obj(number)).is_null(),
            "an int has no text yet"
        );
        assert_eq!(tclshim_test_length_of(c_obj(number)), 0);
        let (pointer, length) = (&*number).c_string();
        assert_eq!(tclshim_test_bytes_of(c_obj(number)), pointer);
        assert_eq!(
            tclshim_test_length_of(c_obj(number)),
            isize::try_from(length).expect("small")
        );
        assert_eq!(length, 2);
        Obj::free(number);
    }

    let list = Obj::list(vec![ObjRef::new(Obj::int(1)), ObjRef::new(Obj::int(22))]).into_raw();
    // SAFETY: as above; the element is a fresh object.
    unsafe {
        (&*list).with_string(|_| ());
        assert!(!tclshim_test_bytes_of(c_obj(list)).is_null());
        assert_eq!(tclshim_test_length_of(c_obj(list)), 4, "`1 22`");
        (&*list)
            .append_element(ObjRef::new(Obj::int(1)))
            .expect("a list takes an element");
        assert!(
            tclshim_test_bytes_of(c_obj(list)).is_null(),
            "changing the value withdrew the text C could read"
        );
        assert_eq!(tclshim_test_length_of(c_obj(list)), 0);
        Obj::free(list);
    }
}

/// The header's `Tcl_IncrRefCount`, `Tcl_DecrRefCount` and `Tcl_IsShared` are
/// macros over `refCount`: they move the count the Rust side reads, and the
/// last release frees through `TclFreeObj`.
#[cfg(cshim_c_tests)]
#[test]
fn the_header_macros_move_the_count_rust_reads_and_the_last_release_frees() {
    let raw = Obj::int(1).into_raw();
    // SAFETY: `raw` is live until the last release below; `adopt` takes the
    // first reference.
    unsafe {
        let first = ObjRef::adopt(raw);
        assert_eq!(tclshim_test_ref_count_of(c_obj(raw)), 1);
        assert_eq!(tclshim_test_shared_of(c_obj(raw)), 0);
        tclshim_test_incr_of(c_obj(raw));
        assert_eq!(first.get().refcount(), 2);
        assert_eq!(tclshim_test_shared_of(c_obj(raw)), 1);
        tclshim_test_decr_of(c_obj(raw));
        assert_eq!(first.get().refcount(), 1);
        assert_eq!(tclshim_test_shared_of(c_obj(raw)), 0);

        let freed = DESTROYED.with(Cell::get);
        std::mem::forget(first);
        tclshim_test_decr_of(c_obj(raw));
        assert_eq!(
            DESTROYED.with(Cell::get),
            freed + 1,
            "the macro called TclFreeObj"
        );
    }
}

/// Tcl's macro frees at a count of one or less, so one release of an object
/// nobody has taken a reference to frees it.
#[cfg(cshim_c_tests)]
#[test]
fn a_single_release_of_a_fresh_object_through_the_macro_frees_it() {
    let raw = Obj::int(1).into_raw();
    let before = DESTROYED.with(Cell::get);
    // SAFETY: `raw` is live, and this call frees it.
    unsafe { tclshim_test_decr_of(c_obj(raw)) };
    assert_eq!(DESTROYED.with(Cell::get), before + 1);
}

#[test]
fn a_rust_release_of_a_fresh_object_frees_it_as_in_c_tcl() {
    let raw = Obj::int(1).into_raw();
    let before = DESTROYED.with(Cell::get);
    // SAFETY: `raw` is live, and this call frees it.
    unsafe { Obj::decr_ref_count(raw) };
    assert_eq!(DESTROYED.with(Cell::get), before + 1);
}

#[test]
fn a_rust_reference_and_a_c_macro_free_once_between_them() {
    let freed = DESTROYED.with(Cell::get);
    let first = ObjRef::new(Obj::int(1));
    let second = first.clone();
    drop(first);
    assert_eq!(
        DESTROYED.with(Cell::get),
        freed,
        "one reference is still held"
    );
    drop(second);
    assert_eq!(DESTROYED.with(Cell::get), freed + 1);
}

#[test]
fn namespace_primary_duplicate_and_result_retain_opaque_authority() {
    let receipt = std::rc::Rc::new(17_u64);
    let carrier = tcl_engine_api::NativeNamespaceNameCache::new(
        tcl_engine_api::NativeCVersion::V9_0,
        (19, 0),
        receipt.clone(),
    );
    let object = Obj::with_rep(
        Rep::NamespaceName(carrier),
        Some(Box::from(b"::a:::q\0".as_slice())),
        false,
    );
    let duplicate = object.duplicate();
    assert_eq!(duplicate.bytes(), b"::a:::q");
    let result = duplicate.representation_result().unwrap();
    let tcl_engine_api::OriginalObjectResult::Shared(result) = result else {
        panic!("namespace result retains original graph identity");
    };
    let tcl_engine_api::OriginalObjectResult::Resident { value, .. } = result.as_ref() else {
        panic!("namespace primary exports its original resident allocation");
    };
    let tcl_engine_api::OriginalObjectResult::NamespaceName(carrier) = value.as_ref() else {
        panic!("namespace primary cannot become a data-only string");
    };
    assert_eq!(carrier.engine_receipt().downcast_ref::<u64>(), Some(&17));
    let primary = object.rep.borrow();
    let Rep::NamespaceName(original) = &*primary else {
        unreachable!()
    };
    assert!(std::ptr::eq(
        original.engine_receipt(),
        carrier.engine_receipt()
    ));
}

#[test]
fn native_list_duplicates_share_members_and_flag_without_copying_child_refs() {
    let original = Obj::list(vec![ObjRef::new(Obj::int(7))]);
    let duplicate = original.duplicate();
    {
        let first = original.rep.borrow();
        let second = duplicate.rep.borrow();
        let (Rep::List(first), Rep::List(second)) = (&*first, &*second) else {
            panic!("List headers");
        };
        assert!(std::rc::Rc::ptr_eq(&first.0, &second.0));
        assert_eq!(first[0].get().refcount(), 1);
        assert!(first.is_shared());
        assert!(!first.0.canonical.get());
    }
    assert_eq!(duplicate.bytes(), b"7");
    if let Rep::List(first) = &*original.rep.borrow() {
        assert!(!first.0.canonical.get());
    }
    drop(duplicate);
    assert_eq!(original.bytes(), b"7");
    if let Rep::List(first) = &*original.rep.borrow() {
        assert!(first.0.canonical.get());
    }
}

#[test]
fn native_list_copy_on_write_acquires_real_child_refs_and_separate_flag() {
    let original = Obj::list(vec![ObjRef::new(Obj::int(7))]);
    let duplicate = original.duplicate();
    duplicate.append_element(ObjRef::new(Obj::int(8))).unwrap();
    assert_eq!(original.with_list(<[ObjRef]>::len).unwrap(), 1);
    assert_eq!(duplicate.with_list(<[ObjRef]>::len).unwrap(), 2);
    assert_eq!(
        original
            .with_list(|items| items[0].get().refcount())
            .unwrap(),
        2
    );
    drop(duplicate);
    assert_eq!(
        original
            .with_list(|items| items[0].get().refcount())
            .unwrap(),
        1
    );
}

#[test]
fn cyclic_callback_list_refuses_before_exporting_an_identity_graph() {
    let object = ObjRef::new(Obj::list(vec![ObjRef::new(Obj::int(1))]));
    *object.get().rep.borrow_mut() =
        super::Rep::List(super::ListHeader::new(vec![object.clone()], false));
    let result = object.get().original_result();
    // Release the artificial cycle after observing the checked boundary.
    *object.get().rep.borrow_mut() = super::Rep::None;
    assert!(matches!(
        result,
        Err(tcl_engine_api::EngineError::ExecutionRefusal(_))
    ));
}

#[test]
fn an_int_crosses_typed_and_renders_lazily() {
    let obj = Obj::int(42);
    assert!(matches!(obj.to_value(), Value::Int(42)));
    assert_eq!(obj.bytes(), b"42");
    assert!(
        obj.to_value().as_int() == Some(42),
        "a rendered string retains its typed payload"
    );
}

#[test]
fn a_parsed_string_keeps_its_spelling() {
    let obj = Obj::from_text("0x10");
    assert_eq!(obj.get_wide().expect("hex parses"), 16);
    assert_eq!(obj.to_value().as_str(), Some("0x10"));
    assert_eq!(
        obj.get_double().expect("via the int rep").to_bits(),
        16.0_f64.to_bits()
    );
}

#[test]
fn conversions_report_c_tcl_messages() {
    let bad = Obj::from_text("abc");
    let error = bad.get_wide().expect_err("not an integer");
    assert_eq!(error.message, b"expected integer but got \"abc\"");
    assert_eq!(error.code.as_deref(), Some(b"TCL VALUE NUMBER".as_slice()));
    let big = Obj::from_text("99999999999999999999");
    assert_eq!(
        big.get_wide().expect_err("overflows").message,
        b"integer value too large to represent"
    );
    assert_eq!(
        big.get_double().expect("as a double").to_bits(),
        1e20_f64.to_bits()
    );
    let nan = Obj::from_text("NaN");
    assert_eq!(
        nan.get_double().expect_err("NaN").code.as_deref(),
        Some(b"TCL VALUE DOUBLE NAN".as_slice())
    );
    assert!(
        Obj::from_text("1.5")
            .get_boolean()
            .expect("numbers are booleans")
    );
    assert_eq!(
        Obj::from_text("NaN")
            .get_boolean()
            .expect_err("NaN")
            .code
            .as_deref(),
        Some(b"TCL VALUE DOUBLE NAN".as_slice())
    );
    assert_eq!(
        Obj::double(f64::NAN)
            .get_boolean()
            .expect_err("NaN rep")
            .message,
        b"floating point value is Not a Number"
    );
    assert_eq!(
        Obj::from_text("").get_boolean().expect_err("empty").message,
        b"expected boolean value but got \"\""
    );
}

#[test]
fn native_empty_list_constructor_preserves_canonical_null_type_length() {
    let value = Obj::list(Vec::new());
    assert!(matches!(&*value.rep.borrow(), Rep::None));
    assert_eq!(
        value.storage.get(),
        NativeStringStorageIdentity::CanonicalEmpty
    );
    assert_eq!(value.list_len().unwrap(), 0);
    assert!(matches!(&*value.rep.borrow(), Rep::None));
    value.append_element(ObjRef::new(Obj::int(7))).unwrap();
    assert_eq!(value.list_len().unwrap(), 1);
    assert_eq!(value.bytes(), b"7");
}

#[test]
fn lists_are_structural_until_text_is_authoritative() {
    let obj = Obj::list(vec![
        ObjRef::new(Obj::int(1)),
        ObjRef::new(Obj::from_text("b c")),
    ]);
    assert_eq!(obj.bytes(), b"1 {b c}");
    assert_eq!(obj.to_value().as_list().map(<[Value]>::len), Some(2));
    let parsed = Obj::from_text("a  b");
    assert_eq!(parsed.with_list(<[ObjRef]>::len).expect("a list"), 2);
    assert_eq!(parsed.to_value().as_str(), Some("a  b"), "spacing survives");
    let bad = Obj::from_text("{a}b");
    assert_eq!(
        bad.with_list(<[ObjRef]>::len).expect_err("junk").message,
        b"list element in braces followed by \"b\" instead of space"
    );
}

#[test]
fn reference_counts_follow_ownership() {
    let raw = Obj::int(1).into_raw();
    // SAFETY: `raw` is live; `adopt` takes the first reference.
    let first = unsafe { ObjRef::adopt(raw) };
    assert!(!first.get().is_shared());
    let second = first.clone();
    assert!(second.get().is_shared());
    drop(first);
    assert!(!second.get().is_shared());
}

#[test]
fn interior_nul_round_trips_through_modified_utf8() {
    let obj = Obj::from_text("a\0b");
    let (pointer, length) = obj.c_string();
    assert_eq!(length, 4);
    // SAFETY: the pointer addresses `length + 1` live bytes.
    let bytes = unsafe { std::slice::from_raw_parts(pointer.cast::<u8>(), length) };
    assert_eq!(bytes, b"a\xC0\x80b");
    assert_eq!(obj.bytes(), b"a\xC0\x80b");
    assert!(obj.text().is_err(), "native bytes are not Rust UTF-8");
}
#[test]
fn bridge_preserves_raw_strings_and_independent_binary_storage() {
    for bytes in [b"a\0z".as_slice(), b"a\xC0\x80z", b"a\xFFz"] {
        let obj = Obj::from_value(&Value::string_bytes(bytes)).unwrap();
        assert_eq!(obj.bytes(), bytes);
        assert_eq!(obj.to_value().as_bytes(), Some(bytes));
    }
    let pure = Obj::from_value(&Value::byte_array(&b"\0\xFF"[..])).unwrap();
    assert!(pure.string.borrow().is_none());
    assert_eq!(pure.to_value().as_bytes(), None);
    assert_eq!(pure.to_value().as_byte_array(), Some(b"\0\xFF".as_slice()));
    assert_eq!(pure.bytes(), b"\xC0\x80\xC3\xBF");
    assert_eq!(pure.to_value().as_byte_array(), Some(b"\0\xFF".as_slice()));
    let other = Obj::from_value(&pure.to_value()).unwrap();
    assert_eq!(other.bytes(), pure.bytes());
    assert_eq!(
        other.to_value().as_byte_array(),
        pure.to_value().as_byte_array()
    );
}

#[test]
fn scalar_carrier_keeps_full_magnitude_and_rejects_foreign_word_cache() {
    use tcl_engine_api::{NativeCVersion, NativeIntegerRadix, NativeScalarCache as Carrier};
    let carrier = Value::NativeScalar(Carrier::BigInteger {
        negative: true,
        radix: NativeIntegerRadix::Hexadecimal,
        digits: std::rc::Rc::from("10000000000000001"),
    });
    let object = Obj::from_value(&carrier).unwrap();
    assert_eq!(object.bytes(), b"-18446744073709551617");
    let exported = object.to_value();
    let Value::Resident {
        value,
        string,
        storage,
    } = exported
    else {
        panic!("resident full cache")
    };
    assert_eq!(string.as_ref(), b"-18446744073709551617");
    assert_eq!(storage, NativeStringStorageIdentity::Allocated);
    assert!(
        matches!(value.as_ref(), Value::NativeScalar(Carrier::BigInteger { digits, negative:true, .. }) if digits.as_ref()=="10000000000000001")
    );

    let word = Value::NativeScalar(Carrier::WordBoolean {
        value: true,
        origin: NativeCVersion::V9_0,
    });
    assert!(Obj::from_value(&word).is_err());
    let resident = word.with_resident_string(&b"yes\xFF"[..]);
    let object = Obj::from_value(&resident).unwrap();
    assert_eq!(object.bytes(), b"yes\xFF");
    assert!(object.get_boolean().unwrap());
    let foreign = Value::NativeScalar(Carrier::WordBoolean {
        value: true,
        origin: NativeCVersion::V8_6,
    })
    .with_resident_string("yes".as_bytes());
    assert!(Obj::from_value(&foreign).is_err());
}
#[test]
fn primitive_getters_use_actual_c9_storage_and_failure_cache() {
    use tcl_syntax::scalar_getter::NativeScalarCache;
    let raw = Obj::from_bytes(b"12\0tail");
    assert_eq!(
        raw.get_wide().expect("native raw-string numeric prefix"),
        12
    );
    assert_eq!(raw.bytes(), b"12\0tail");
    let binary = Obj::from_value(&Value::byte_array(&b"12\0tail"[..])).unwrap();
    let failure = binary
        .get_wide()
        .expect_err("native bytearray modified-NUL spelling");
    assert_eq!(binary.bytes(), b"12\xC0\x80tail");
    assert_eq!(
        failure
            .getter
            .as_ref()
            .expect("actual getter receipt")
            .getter_kind(),
        tcl_syntax::scalar_getter::NativeScalarGetterKind::Wide
    );
    let word = Obj::from_bytes(b"true");
    assert!(word.get_boolean().expect("native Boolean word"));
    assert!(matches!(
        &*word.rep.borrow(),
        super::Rep::Scalar(NativeScalarCache::WordBoolean(true))
    ));
    let magnitude = Obj::from_bytes(b"18446744073709551615");
    assert!(
        magnitude.get_wide().is_err(),
        "C9 rejects out-of-range wide return"
    );
    assert!(matches!(
        &*magnitude.rep.borrow(),
        super::Rep::Scalar(NativeScalarCache::Number(
            tcl_syntax::number::Number::Big { .. }
        ))
    ));
    assert_eq!(magnitude.bytes(), b"18446744073709551615");
}
