// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual Jim object-context roles remain separate from absent name coordinates.

use crate::interp::Interp;
use crate::obj::{self, Owned};
use std::rc::Rc;

#[test]
fn unmaterialized_namespace_retains_the_actual_frame_after_its_procedure_role_retires() {
    // Software ownership control: actual frame roles, without foreign Jim frame authority.
    let mut interp = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jimtcl"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    let context = interp.native_jim_object_context().unwrap();
    let original = Owned::fresh(obj::new_wide_int_obj(42));
    let pointer = original.as_ptr();
    crate::native_source::bind_context(pointer, &context).unwrap();
    let allocation = obj::NativeObjectLifetime::retain(pointer);
    let namespace = interp
        .namespaces_mut()
        .retain_unmaterialized_jim_namespace(original);
    let procedure = interp
        .namespaces_mut()
        .take_jim_namespace_owner(namespace)
        .unwrap();
    assert_eq!(unsafe { (*pointer).ref_count }, 1);
    assert!(interp.namespaces().jim_namespace_bytes(namespace).is_none());
    assert!(interp.namespaces().native_context_path(namespace).is_none());
    let caller = interp.current_ns();
    interp.enter_namespace_activation(namespace);
    interp.frames.borrow_mut().push(namespace);
    interp.set_current_ns(namespace);
    let frame = Rc::new(Owned::retain(pointer));
    let frame_birth = Rc::downgrade(&frame);
    interp
        .namespaces_mut()
        .remember_jim_namespace_owner(namespace, &frame)
        .unwrap();
    interp
        .frames
        .borrow_mut()
        .retain_jim_activation_objects(None, None, frame);
    assert_eq!(unsafe { (*pointer).ref_count }, 2);
    drop(procedure);
    assert_eq!(unsafe { (*pointer).ref_count }, 1);
    assert!(obj::check_native_liveness(pointer).is_ok());
    assert!(!obj::has_string_rep(pointer));
    assert!(interp.namespaces().native_context_path(namespace).is_none());
    assert!(interp
        .namespaces()
        .jim_namespace_original_owner(namespace)
        .is_some_and(|owner| std::rc::Weak::ptr_eq(&Rc::downgrade(&owner), &frame_birth)));
    assert_eq!(
        interp
            .materialize_jim_namespace(namespace)
            .unwrap()
            .as_ref(),
        b"42"
    );
    assert_eq!(unsafe { (*pointer).ref_count }, 1);
    assert!(interp.namespaces().native_context_path(namespace).is_some());
    let popped = interp.pop_native_call_frame();
    interp.set_current_ns(caller);
    interp.leave_namespace_activation(popped);
    assert_eq!(frame_birth.strong_count(), 0);
    assert!(obj::check_native_liveness(pointer).is_err());
    assert!(allocation.checked_ptr().is_err());
    assert!(interp
        .namespaces()
        .jim_namespace_original_owner(namespace)
        .is_none());
    assert!(interp.namespaces().native_context_path(namespace).is_none());
    let fresh_original = Owned::fresh(obj::new_wide_int_obj(43));
    crate::native_source::bind_context(fresh_original.as_ptr(), &context).unwrap();
    let unresolved = interp
        .namespaces_mut()
        .retain_unmaterialized_jim_namespace(fresh_original);
    interp.set_dialect_profile(crate::environment::profile_for_dialect("tcl8.6"));
    let names = interp.namespaces();
    for actual_jim_context in [namespace, unresolved] {
        assert!(names.native_context_path(actual_jim_context).is_none());
        assert!(names
            .native_c_command_context_path(actual_jim_context, tcl_dialect::TclVersion::V8_6)
            .is_none());
        assert!(names
            .namespace_name_token(interp.native_callable_interpreter(), actual_jim_context)
            .is_none());
    }
    assert!(names
        .native_c_command_context_path(super::GLOBAL, tcl_dialect::TclVersion::V8_6)
        .is_some());
    assert!(interp.native_execution_refusal().is_none());
}
