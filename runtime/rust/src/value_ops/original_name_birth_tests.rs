// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fresh ValueOps names transfer no owning reference to their raw receiver.

use crate::{
    interp::Interp,
    obj::{self, Owned},
};
use tcl_syntax::{native_object::NativeObjectCacheSnapshot, value::ValueOps};

#[test]
fn original_full_command_name_birth_is_unowned_until_its_actual_list_or_result_receiver() {

    // Runtime software role control, without a Native getter/count observation.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interpreter = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(profile),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let name =
            ValueOps::native_command_full_name_result(&mut interpreter, b"::qualified::name")
                .unwrap();
        let allocation = obj::NativeObjectLifetime::retain(name);
        assert_eq!(
            unsafe { (*name).ref_count },
            0,
            "{profile}: fresh raw result"
        );
        assert!(matches!(
            obj::native_object_snapshot(name).unwrap().cache,
            NativeObjectCacheSnapshot::String { .. }
        ));
        let list = Owned::fresh(ValueOps::new_list(&mut interpreter, vec![name]));
        assert_eq!(
            unsafe { (*name).ref_count },
            1,
            "{profile}: actual List member"
        );
        assert_eq!(crate::list::cached_length(list.as_ptr()), Some(1));
        drop(list);
        assert!(
            obj::check_native_liveness(allocation.as_ptr()).is_err(),
            "{profile}: final member retired"
        );

        let name =
            ValueOps::native_command_full_name_result(&mut interpreter, b"::result::name").unwrap();
        let allocation = obj::NativeObjectLifetime::retain(name);
        assert_eq!(unsafe { (*name).ref_count }, 0);
        interpreter.set_result(name);
        assert_eq!(interpreter.get_obj_result(), name);
        assert_eq!(
            unsafe { (*name).ref_count },
            1,
            "{profile}: actual result role"
        );
        interpreter.set_result_bytes(b"NEXT");
        assert!(
            obj::check_native_liveness(allocation.as_ptr()).is_err(),
            "{profile}: final result retired"
        );
    }
    let mut jim = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jim"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert!(ValueOps::native_command_full_name_result(&mut jim, b"::unavailable").is_err());
}
