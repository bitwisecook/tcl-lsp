// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual C91 property-name descriptor and original accessor child owners.
use super::*;
use tcl_syntax::value::ValueError;
#[derive(Clone)]
struct PropertyName {
    reader: Owned,
    writer: Owned,
}
extern "C" fn free(value: *mut TclObj) {
    unsafe {
        drop(Box::from_raw(
            internal_rep(value) as usize as *mut PropertyName
        ));
    }
}
extern "C" fn duplicate(source: *mut TclObj, copy: *mut TclObj) {
    let cache = unsafe { &*(internal_rep(source) as usize as *const PropertyName) };
    change_type(
        copy,
        &TYPE,
        Box::into_raw(Box::new(cache.clone())) as usize as u64,
    );
}
pub(crate) static TYPE: TclObjType = TclObjType {
    name: c"tcl::oo property name".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: Some(duplicate),
    update_string_proc: None,
    set_from_any_proc: None,
};
pub(crate) fn is_cached(value: *mut TclObj) -> bool {
    core::ptr::eq(obj_type_ptr(value), &TYPE)
}
pub(crate) fn accessor(
    value: *mut TclObj,
    recipe: tcl_registry::native_property_lookup::NativePropertyLookupProtocol,
    writable: bool,
) -> Result<*mut TclObj, ValueError> {
    check_native_liveness(value)?;
    if !is_cached(value) {
        let bytes = crate::dict::native_object_bytes(value, recipe.strings())?;
        let (reader, writer) = recipe.accessor_names(&bytes);
        let reader = Owned::fresh(new_string_bytes(&reader));
        let writer = Owned::fresh(new_string_bytes(&writer));
        let materialization = recipe.materialization();
        retain_native_string_representation(reader.as_ptr(), materialization)?;
        retain_native_string_representation(writer.as_ptr(), materialization)?;
        change_type(
            value,
            &TYPE,
            Box::into_raw(Box::new(PropertyName { reader, writer })) as usize as u64,
        );
    }
    let cache = unsafe { &*(internal_rep(value) as usize as *const PropertyName) };
    Ok(if writable {
        cache.writer.as_ptr()
    } else {
        cache.reader.as_ptr()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_property_name_duplicate_reuses_and_releases_accessor_children() {
        let dialect = tcl_registry::InvocationDialect::of_profile(
            crate::environment::profile_for_dialect("tcl9.1"),
        );
        let recipe = dialect.native_property_lookup_protocol().unwrap();
        let original = Owned::fresh(new_string_bytes(b"-y\xff\0tail"));
        let reader = accessor(original.as_ptr(), recipe, false).unwrap();
        let writer = accessor(original.as_ptr(), recipe, true).unwrap();
        assert_eq!(bytes_of(reader), b"<ReadProp-y\xff>");
        assert_eq!(bytes_of(writer), b"<WriteProp-y\xff>");
        assert_eq!(unsafe { (*reader).ref_count }, 1);
        assert_eq!(unsafe { (*writer).ref_count }, 1);
        let copy = Owned::fresh(super::super::duplicate(original.as_ptr()));
        assert_eq!(accessor(copy.as_ptr(), recipe, false).unwrap(), reader);
        assert_eq!(unsafe { (*reader).ref_count }, 2);
        assert_eq!(unsafe { (*writer).ref_count }, 2);
        drop(copy);
        assert_eq!(unsafe { (*reader).ref_count }, 1);
        assert_eq!(unsafe { (*writer).ref_count }, 1);
        invalidate_string(original.as_ptr());
        assert_eq!(accessor(original.as_ptr(), recipe, false).unwrap(), reader);
        assert!(!native_string_available(original.as_ptr()));
        let snapshot = native_object_snapshot(original.as_ptr()).unwrap();
        assert!(snapshot.resident.is_none());
    }
}
