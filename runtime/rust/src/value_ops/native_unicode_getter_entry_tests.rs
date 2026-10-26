// SPDX-License-Identifier: AGPL-3.0-or-later
//! Constructor-issued getter authority is distinct from profile.

use super::{change_and_restore, leak_free, original};
use crate::{interp::obj_bytes, obj};
use tcl_syntax::value::ValueOps;

#[test]
fn a_reached_native_unicode_view_preserves_the_original_invalid_byte_observation() {
    for environment in ["tcl8.6", "tcl9.0"] {
        let mut retained = None;
        leak_free(|| {
            let mut interpreter = original(environment);
            let value = obj::Owned::fresh(obj::new_string_bytes(b"A\xffB"));
            assert!(obj::obj_type_ptr(value.as_ptr()).is_null());
            let text = interpreter.checked_character_view(&value.as_ptr()).unwrap();
            let units = interpreter.native_unicode_units(&value.as_ptr()).unwrap();
            assert_eq!(&*units, &[0x0041, 0x00ff, 0x0042]);
            assert_eq!(text.chars().count(), 3);
            assert_eq!(text.chars().nth(1), Some('\u{00ff}'));
            assert_eq!(
                interpreter
                    .try_as_str(&value.as_ptr())
                    .unwrap_err()
                    .valid_up_to,
                1
            );
            assert_eq!(obj_bytes(value.as_ptr()), b"A\xffB");
            retained = Some(text);
        });
        assert_eq!(&**retained.as_ref().unwrap(), "A\u{00ff}B");
    }
}

#[test]
fn a_foreign_or_unpaired_native_cache_cannot_borrow_a_generic_ascii_view() {
    leak_free(|| {
        let mut interpreter = original("tcl8.6");
        let ascii = obj::Owned::fresh(obj::new_string_bytes(b"A"));
        interpreter.native_unicode_units(&ascii.as_ptr()).unwrap();
        let mut other = original("tcl9.0");
        let error = other.checked_character_view(&ascii.as_ptr()).unwrap_err();
        assert!(error.native_access_refusal().is_some());
        assert_eq!(&*other.try_as_str(&ascii.as_ptr()).unwrap(), "A");
    });
    leak_free(|| {
        let mut interpreter = original("tcl8.6");
        let value = obj::Owned::fresh(
            obj::new_native_unicode_obj(
                std::rc::Rc::from([0xd800]),
                interpreter.native_invocation_dialect(),
            )
            .unwrap(),
        );
        assert!(!obj::has_string_rep(value.as_ptr()));
        let error = interpreter
            .checked_character_view(&value.as_ptr())
            .unwrap_err();
        assert!(error.native_access_refusal().is_some());
        assert!(!obj::has_string_rep(value.as_ptr()));
    });
}

#[test]
fn well_formed_native_utf16_is_a_lossless_host_view() {
    leak_free(|| {
        let mut interpreter = original("tcl8.6");
        let value = obj::Owned::fresh(
            obj::new_native_unicode_obj(
                std::rc::Rc::from([0xd83d, 0xde00]),
                interpreter.native_invocation_dialect(),
            )
            .unwrap(),
        );
        let text = interpreter.checked_character_view(&value.as_ptr()).unwrap();
        assert_eq!(&*text, "\u{1f600}");
        assert_eq!(text.encode_utf16().collect::<Vec<_>>(), [0xd83d, 0xde00]);
        assert!(!obj::has_string_rep(value.as_ptr()));
    });
}

#[test]
fn a_stale_entered_operation_cannot_consume_an_existing_unicode_cache() {
    leak_free(|| {
        let mut interpreter = original("tcl8.6");
        let value = obj::Owned::fresh(obj::new_string_bytes(b"A\xffB"));
        interpreter.native_unicode_units(&value.as_ptr()).unwrap();
        let entered =
            crate::interp::native_operation_currency::NativeOperationScope::enter(&interpreter)
                .unwrap();
        change_and_restore(&mut interpreter);
        assert!(interpreter.checked_character_view(&value.as_ptr()).is_err());
        assert!(interpreter.host_refusal_pending());
        assert_eq!(obj_bytes(value.as_ptr()), b"A\xffB");
        drop(entered);
    });
}

#[test]
fn generic_or_profile_only_construction_cannot_issue_a_native_getter() {
    for environment in ["tcl8.6", "tcl9.0", "f5-irules"] {
        leak_free(|| {
            let mut interpreter = crate::Interp::new();
            let profile = tcl_registry::model::resolve_environment(environment).unit_profile();
            assert_eq!(profile.name, environment);
            interpreter.set_dialect_profile(profile);
            let value = obj::Owned::fresh(obj::new_string_bytes(b"A\xffB"));
            let header = obj::obj_type_ptr(value.as_ptr());
            assert!(interpreter.checked_character_view(&value.as_ptr()).is_err());
            assert_eq!(obj::obj_type_ptr(value.as_ptr()), header);
            assert_eq!(obj_bytes(value.as_ptr()), b"A\xffB");
            assert_eq!(
                interpreter
                    .try_as_str(&value.as_ptr())
                    .unwrap_err()
                    .valid_up_to,
                1
            );
        });
    }
}

#[test]
fn getter_callbacks_cannot_refresh_the_original_constructor_entry() {
    leak_free(|| {
        let mut interpreter = original("tcl8.6");
        let value = obj::Owned::fresh(obj::alloc_typed(&super::ORIGINAL_TYPE, 0));
        super::UPDATER_OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
        super::UPDATER_CALLS.with(|calls| calls.set(0));
        super::UPDATER_HOST.with(|host| host.set(false));
        let error = interpreter
            .checked_character_view(&value.as_ptr())
            .unwrap_err();
        assert!(error.native_access_refusal().is_some());
        assert!(interpreter.host_refusal_pending());
        super::UPDATER_CALLS.with(|calls| assert_eq!(calls.get(), 1));
        assert_eq!(obj_bytes(value.as_ptr()), b"17");
        super::UPDATER_OWNER.with(|owner| *owner.borrow_mut() = None);
    });
}

#[test]
fn a_retired_original_object_cannot_reenter_under_a_retained_allocation_lease() {
    leak_free(|| {
        let mut interpreter = original("tcl8.6");
        let value = obj::Owned::fresh(obj::new_string_bytes(b"A\xffB"));
        let lease = obj::NativeObjectLifetime::retain(value.as_ptr());
        drop(value);
        assert!(lease.checked_ptr().is_err());
        assert!(interpreter.checked_character_view(&lease.as_ptr()).is_err());
        assert!(interpreter.native_character_view(&lease).is_err());
    });
}

thread_local! {
    static FREE_OWNER: std::cell::RefCell<Option<crate::interp::WeakInterp>> = const { std::cell::RefCell::new(None) };
    static FREE_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

extern "C" fn update_before_reentrant_free(value: *mut obj::TclObj) {
    // SAFETY: the original reached updater receives its caller-owned live header.
    unsafe { obj::set_native_updater_string_rep(value, b"17", false) };
}

extern "C" fn reentrant_free(_value: *mut obj::TclObj) {
    FREE_CALLS.with(|calls| calls.set(calls.get() + 1));
    let owner = FREE_OWNER.with(|owner| {
        owner
            .borrow()
            .as_ref()
            .and_then(crate::interp::WeakInterp::upgrade)
    });
    if let Some(mut interpreter) = owner {
        change_and_restore(&mut interpreter);
    }
}

static REENTRANT_FREE_TYPE: obj::TclObjType = obj::TclObjType {
    name: c"nativeUnicodeViewReentrantFree".as_ptr(),
    free_int_rep_proc: Some(reentrant_free),
    dup_int_rep_proc: None,
    update_string_proc: Some(update_before_reentrant_free),
    set_from_any_proc: None,
};

#[test]
fn original_free_callback_cannot_publish_a_stale_unicode_view() {
    leak_free(|| {
        let mut interpreter = original("tcl8.6");
        let value = obj::Owned::fresh(obj::alloc_typed(&REENTRANT_FREE_TYPE, 0));
        FREE_OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
        FREE_CALLS.with(|calls| calls.set(0));
        let error = interpreter
            .checked_character_view(&value.as_ptr())
            .unwrap_err();
        assert!(error.native_access_refusal().is_some());
        FREE_CALLS.with(|calls| assert_eq!(calls.get(), 1));
        assert!(interpreter.host_refusal_pending());
        FREE_OWNER.with(|owner| *owner.borrow_mut() = None);
    });
}
