// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected Wide descriptor controls; recorded Native rows are independent.
use super::*;
use crate::interp::native_operation_currency::NativeOperationScope;
use tcl_syntax::scalar_getter::{
    NativeScalarGetterFailure as Failure, NativeScalarGetterKind as Kind,
};
fn native(profile: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(profile),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
#[test]
fn selected_c84_wide_import_duplicate_getter_and_string_preserve_the_physical_descriptor() {
    let mut interp = native("tcl8.4");
    let original = scalar(&mut interp, &NativeScalarCache::Tcl84Wide(4_294_967_296)).unwrap();
    let ptr = original.as_ptr();
    assert!(core::ptr::eq(obj::obj_type_ptr(ptr), &obj::TCL_WIDE84_TYPE));
    assert!(!obj::has_string_rep(ptr));
    assert_eq!(unsafe { (*ptr).ref_count }, 1);
    let copy = Owned::fresh(obj::duplicate(ptr));
    assert!(core::ptr::eq(
        obj::obj_type_ptr(copy.as_ptr()),
        &obj::TCL_WIDE84_TYPE
    ));
    assert!(!obj::has_string_rep(copy.as_ptr()));
    assert_eq!(unsafe { (*copy.as_ptr()).ref_count }, 1);
    let scope = NativeOperationScope::enter(&interp).unwrap();
    let host = interp.host();
    let reached = crate::typed_value::native_scalar_probe_with_environment_and_currency(
        ptr,
        interp.native_invocation_dialect(),
        Kind::Int,
        host.numeric_environment(),
        Some(scope.currency()),
    )
    .unwrap();
    assert!(matches!(reached, Err(Failure::IntWidthOverflow)));
    assert!(core::ptr::eq(obj::obj_type_ptr(ptr), &obj::TCL_WIDE84_TYPE));
    assert!(!obj::has_string_rep(ptr));
    assert_eq!(
        interp.native_object_string_bytes(ptr).unwrap().as_ref(),
        b"4294967296"
    );
    assert!(core::ptr::eq(obj::obj_type_ptr(ptr), &obj::TCL_WIDE84_TYPE));
    assert_eq!(unsafe { (*ptr).ref_count }, 1);
    assert!(!obj::has_string_rep(copy.as_ptr()));
    let long = scalar(&mut interp, &NativeScalarCache::Tcl84Long(17)).unwrap();
    assert!(core::ptr::eq(
        obj::obj_type_ptr(long.as_ptr()),
        &obj::TCL_LONG84_TYPE
    ));
    let neutral = Owned::fresh(obj::new_wide_int_obj(4_294_967_296));
    assert!(core::ptr::eq(
        obj::obj_type_ptr(neutral.as_ptr()),
        &obj::TCL_INT_TYPE
    ));
}
#[test]
fn selected_c84_scalar_ingress_refuses_unqualified_or_foreign_descriptor_without_guest_publication()
{
    for (profile, facts) in [
        ("tcl8.4", NativeScalarCache::Integer(17)),
        ("tcl8.6", NativeScalarCache::Tcl84Wide(17)),
        ("jim", NativeScalarCache::Tcl84Wide(17)),
    ] {
        let mut interp = native(profile);
        interp.set_result_bytes(b"PRIOR");
        let prior = interp.get_obj_result();
        assert!(scalar(&mut interp, &facts).is_err());
        assert_eq!(interp.get_obj_result(), prior);
    }
}
