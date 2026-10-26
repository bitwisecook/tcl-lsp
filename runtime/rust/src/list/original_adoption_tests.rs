// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software bounds at the original List primary retirement/adoption stage.
use super::*;
use crate::interp::{native_arguments::OriginalArguments, Interp};
use tcl_cmd_core::CmdError;
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::NativeValueAccessRefusal;

thread_local! {
    static MODE: Cell<u8> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static MEMBER_FREES: Cell<usize> = const { Cell::new(0) };
    static MEMBER_REFS: Cell<obj::TclSize> = const { Cell::new(0) };
}
fn native() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl8.6"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
struct CurrentInterp;
impl Drop for CurrentInterp {
    fn drop(&mut self) {
        crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
    }
}
extern "C" fn free_original(value: *mut TclObj) {
    let previous = FREES.with(|calls| {
        let previous = calls.get();
        calls.set(previous + 1);
        previous
    });
    if previous != 0 {
        return;
    }
    let mode = MODE.with(Cell::get);
    if mode >= 2 {
        obj::change_type(value, &obj::TCL_INT_TYPE, 777);
        if mode == 3 {
            obj::change_type(value, &ORIGINAL, 0);
        }
    }
    if mode == 1 || mode == 4 {
        let mut interpreter = unsafe { crate::codegen_abi::current_interp().as_ref() }
            .unwrap()
            .clone();
        interpreter.refuse_host_command("original List primary retirement");
    }
}
static ORIGINAL: TclObjType = TclObjType {
    name: c"originalListAdoption".as_ptr(),
    free_int_rep_proc: Some(free_original),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn original() -> obj::Owned {
    let original = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
    unsafe { obj::set_native_updater_string_rep(original.as_ptr(), b"a b", false) };
    original
}
fn refuse(interpreter: &Interp, error: tcl_syntax::value::ValueError) -> CmdError {
    let error = CmdError::from(error);
    if let Some(first) = error.native_execution_refusal() {
        interpreter.clone().refuse_native_execution(first.clone());
        return CmdError::from_execution_refusal(interpreter.native_execution_refusal().unwrap());
    }
    error
}
#[test]
fn original_list_current_retirement_adopts_once_without_an_extra_header_role() {
    let interpreter = native();
    let value = original();
    let arguments = OriginalArguments::enter(&interpreter, &[value.as_ptr()]).unwrap();
    MODE.with(|mode| mode.set(0));
    FREES.with(|calls| calls.set(0));
    let members = list_elements_native_using(
        value.as_ptr(),
        NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6),
        &|| arguments.check_for(&interpreter),
        &|error| refuse(&interpreter, error),
    )
    .unwrap();
    assert_eq!(members.len(), 2);
    assert_eq!(obj::bytes_of(members[0]), b"a");
    assert_eq!(obj::bytes_of(members[1]), b"b");
    assert_eq!(FREES.with(Cell::get), 1);
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
    assert_eq!(unsafe { (*members[0]).ref_count }, 1);
    assert_eq!(unsafe { (*members[1]).ref_count }, 1);
    assert_eq!(obj::bytes_of(value.as_ptr()), b"a b");
    assert!(interpreter.native_execution_refusal().is_none());
}
#[test]
fn original_list_reentrant_primary_and_restore_refuse_before_backing_adoption() {
    // Real primary callbacks; these are software bounds, not C callback parity.
    for mode in [1, 2, 3, 4] {
        let mut interpreter = native();
        let value = original();
        let arguments = OriginalArguments::enter(&interpreter, &[value.as_ptr()]).unwrap();
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        MODE.with(|selected| selected.set(mode));
        FREES.with(|calls| calls.set(0));
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
        let _current = CurrentInterp;
        let objects_before = crate::counters::live_objs();
        let failure = list_elements_native_using(
            value.as_ptr(),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6),
            &|| arguments.check_for(&interpreter),
            &|error| refuse(&interpreter, error),
        )
        .unwrap_err();
        assert_eq!(crate::counters::live_objs(), objects_before);
        let first = interpreter.native_execution_refusal().unwrap();
        assert_eq!(failure.native_execution_refusal(), Some(&first));
        if mode == 1 || mode == 4 {
            assert!(
                matches!(first, NativeExecutionError::HostCommandRefusal(ref reason)
                if reason.reason == "original List primary retirement")
            );
        } else {
            assert_eq!(
                first,
                NativeExecutionError::ValueAccessRefusal(
                    NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "changed native primary publication slot",
                    ),
                )
            );
        }
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_eq!(obj::bytes_of(value.as_ptr()), b"a b");
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
        assert!(!core::ptr::eq(
            obj::obj_type_ptr(value.as_ptr()),
            &TCL_LIST_TYPE
        ));
        if mode == 1 {
            assert!(obj::obj_type_ptr(value.as_ptr()).is_null());
            assert_eq!(FREES.with(Cell::get), 1);
        } else if mode == 3 {
            assert!(core::ptr::eq(obj::obj_type_ptr(value.as_ptr()), &ORIGINAL));
            assert_eq!(FREES.with(Cell::get), 2);
        } else {
            assert!(core::ptr::eq(
                obj::obj_type_ptr(value.as_ptr()),
                &obj::TCL_INT_TYPE
            ));
            assert_eq!(obj::internal_rep(value.as_ptr()), 777);
            assert_eq!(FREES.with(Cell::get), 2);
        }
        assert_eq!(
            arguments
                .check_for(&interpreter)
                .unwrap_err()
                .native_execution_refusal(),
            Some(&first)
        );
    }
}
extern "C" fn free_member(value: *mut TclObj) {
    MEMBER_FREES.with(|calls| calls.set(calls.get() + 1));
    MEMBER_REFS.with(|refs| refs.set(unsafe { (*value).ref_count }));
    let mut interpreter = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone();
    interpreter.refuse_host_command("mandatory unused List member retirement");
}
static MEMBER: TclObjType = TclObjType {
    name: c"originalListUnusedMember".as_ptr(),
    free_int_rep_proc: Some(free_member),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
#[test]
fn original_list_failed_adoption_retires_the_actual_prepared_member_before_settlement() {
    let mut interpreter = native();
    let value = original();
    let member = obj::alloc_typed(&MEMBER, 0);
    let member_lifetime = obj::NativeObjectLifetime::retain(member);
    // This is the prepared backing's actual member role, not a guard hold.
    unsafe { obj::incr_ref_count(member) };
    let backing = Box::new(TclList {
        elems: NativeListStorage::new(vec![member]),
        canonical: Rc::new(Cell::new(false)),
        string_protocol: Cell::new(Some(NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6))),
        string_typed: false,
    });
    let arguments = OriginalArguments::enter(&interpreter, &[value.as_ptr()]).unwrap();
    MODE.with(|mode| mode.set(1));
    FREES.with(|calls| calls.set(0));
    MEMBER_FREES.with(|calls| calls.set(0));
    MEMBER_REFS.with(|refs| refs.set(-1));
    crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
    let _current = CurrentInterp;
    let failure = with_native_conversion_check(
        &|| arguments.check_for(&interpreter),
        &|error| refuse(&interpreter, error),
        |stage, retain| {
            install_list_backing(value.as_ptr(), backing, None, Some(stage), Some(retain))
        },
    )
    .unwrap_err();
    let first = interpreter.native_execution_refusal().unwrap();
    assert!(
        matches!(first, NativeExecutionError::HostCommandRefusal(ref reason)
        if reason.reason == "original List primary retirement")
    );
    assert_eq!(failure.native_execution_refusal(), Some(&first));
    assert_eq!(MEMBER_FREES.with(Cell::get), 1);
    assert_eq!(MEMBER_REFS.with(Cell::get), 0);
    assert!(!obj::allocation_is_live(member_lifetime.as_ptr()));
    assert!(obj::obj_type_ptr(value.as_ptr()).is_null());
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
}

#[test]
fn original_list_primary_guard_precedes_the_unused_member_host_retirement() {
    let mut interpreter = native();
    let value = original();
    let member = obj::alloc_typed(&MEMBER, 0);
    let member_lifetime = obj::NativeObjectLifetime::retain(member);
    // This is the prepared backing's actual member role, not a guard hold.
    unsafe { obj::incr_ref_count(member) };
    let backing = Box::new(TclList {
        elems: NativeListStorage::new(vec![member]),
        canonical: Rc::new(Cell::new(false)),
        string_protocol: Cell::new(Some(NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6))),
        string_typed: false,
    });
    let arguments = OriginalArguments::enter(&interpreter, &[value.as_ptr()]).unwrap();
    MODE.with(|mode| mode.set(2));
    FREES.with(|calls| calls.set(0));
    MEMBER_FREES.with(|calls| calls.set(0));
    MEMBER_REFS.with(|refs| refs.set(-1));
    crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
    let _current = CurrentInterp;
    let failure = with_native_conversion_check(
        &|| arguments.check_for(&interpreter),
        &|error| refuse(&interpreter, error),
        |stage, retain| {
            install_list_backing(value.as_ptr(), backing, None, Some(stage), Some(retain))
        },
    )
    .unwrap_err();
    let first = interpreter.native_execution_refusal().unwrap();
    assert_eq!(
        first,
        NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::CommandProtocolUnavailable(
                "changed native primary publication slot"
            ),
        )
    );
    assert_eq!(failure.native_execution_refusal(), Some(&first));
    assert_eq!(MEMBER_FREES.with(Cell::get), 1);
    assert_eq!(MEMBER_REFS.with(Cell::get), 0);
    assert!(!obj::allocation_is_live(member_lifetime.as_ptr()));
    assert!(core::ptr::eq(
        obj::obj_type_ptr(value.as_ptr()),
        &obj::TCL_INT_TYPE
    ));
    assert_eq!(obj::internal_rep(value.as_ptr()), 777);
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
}

#[test]
fn original_dictionary_current_and_reentrant_primary_keep_the_same_stage_cause() {
    for mode in [0, 1, 2, 3, 4] {
        let mut interpreter = native();
        let value = original();
        let arguments = OriginalArguments::enter(&interpreter, &[value.as_ptr()]).unwrap();
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        MODE.with(|selected| selected.set(mode));
        FREES.with(|calls| calls.set(0));
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
        let _current = CurrentInterp;
        let objects_before = crate::counters::live_objs();
        let reached = crate::dict::native_dict_pairs_using(
            value.as_ptr(),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6),
            &|| arguments.check_for(&interpreter),
            &|error| refuse(&interpreter, error),
        );
        if mode == 0 {
            let pairs = reached.unwrap();
            assert_eq!(pairs.len(), 1);
            assert_eq!(obj::bytes_of(pairs[0].0), b"a");
            assert_eq!(obj::bytes_of(pairs[0].1), b"b");
            assert_eq!(unsafe { (*pairs[0].0).ref_count }, 1);
            assert_eq!(unsafe { (*pairs[0].1).ref_count }, 1);
            assert!(core::ptr::eq(
                obj::obj_type_ptr(value.as_ptr()),
                &crate::dict::TCL_DICT_TYPE
            ));
            assert_eq!(FREES.with(Cell::get), 1);
            assert!(interpreter.native_execution_refusal().is_none());
        } else {
            let failure = reached.unwrap_err();
            let first = interpreter.native_execution_refusal().unwrap();
            assert_eq!(failure.native_execution_refusal(), Some(&first));
            assert_eq!(crate::counters::live_objs(), objects_before);
            assert!(!core::ptr::eq(
                obj::obj_type_ptr(value.as_ptr()),
                &crate::dict::TCL_DICT_TYPE
            ));
            if mode == 1 || mode == 4 {
                assert!(
                    matches!(first, NativeExecutionError::HostCommandRefusal(ref reason)
                    if reason.reason == "original List primary retirement")
                );
            } else {
                assert_eq!(
                    first,
                    NativeExecutionError::ValueAccessRefusal(
                        NativeValueAccessRefusal::CommandProtocolUnavailable(
                            "changed native primary publication slot"
                        ),
                    )
                );
            }
            if mode == 1 {
                assert!(obj::obj_type_ptr(value.as_ptr()).is_null());
            } else if mode == 3 {
                assert!(core::ptr::eq(obj::obj_type_ptr(value.as_ptr()), &ORIGINAL));
            } else {
                assert!(core::ptr::eq(
                    obj::obj_type_ptr(value.as_ptr()),
                    &obj::TCL_INT_TYPE
                ));
                assert_eq!(obj::internal_rep(value.as_ptr()), 777);
            }
        }
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
        assert_eq!(obj::bytes_of(value.as_ptr()), b"a b");
        assert_eq!(interpreter.get_obj_result(), prior);
    }
}

struct ReentrantPrimary {
    root: *mut TclObj,
    calls: Rc<Cell<usize>>,
}
impl Drop for ReentrantPrimary {
    fn drop(&mut self) {
        self.calls.set(self.calls.get() + 1);
        obj::change_type(self.root, &obj::TCL_INT_TYPE, 777);
    }
}

#[test]
fn original_operational_conversion_retains_slot_failure_before_unused_member_host() {
    // Actual List/Dict converters and the return-instruction adapter retain the
    // earlier slot failure. Software cache/free callbacks confer no Native chronology.
    for operation in 0..3 {
        let mut interpreter = native();
        let key = obj::new_string_bytes(b"K");
        let member = obj::alloc_typed(&MEMBER, 0);
        unsafe { obj::set_native_updater_string_rep(member, b"V", false) };
        let key_lifetime = obj::NativeObjectLifetime::retain(key);
        let member_lifetime = obj::NativeObjectLifetime::retain(member);
        // Only the genuine input compound owns the original member roles.
        let root = obj::Owned::fresh(if operation == 0 {
            crate::dict::new_dict_obj(&[(key, member)])
        } else {
            new_list_obj(&[key, member])
        });
        assert_eq!(unsafe { (*key).ref_count }, 1);
        assert_eq!(unsafe { (*member).ref_count }, 1);
        let callbacks = Rc::new(Cell::new(0));
        obj::set_allocation_cache(
            root.as_ptr(),
            Rc::new(ReentrantPrimary {
                root: root.as_ptr(),
                calls: Rc::clone(&callbacks),
            }),
        );
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        let arguments = OriginalArguments::enter(&interpreter, &[root.as_ptr()]).unwrap();
        MEMBER_FREES.with(|calls| calls.set(0));
        MEMBER_REFS.with(|refs| refs.set(-1));
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
        let _current = CurrentInterp;
        let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6);
        let reached = match operation {
            0 => list_elements_native_using(
                root.as_ptr(),
                protocol,
                &|| arguments.check_for(&interpreter),
                &|error| arguments.retain_refusal(error.into()),
            )
            .map(drop),
            1 => crate::dict::native_dict_pairs_using(
                root.as_ptr(),
                protocol,
                &|| arguments.check_for(&interpreter),
                &|error| arguments.retain_refusal(error.into()),
            )
            .map(drop),
            _ => interpreter
                .process_original_c_return_options(
                    tcl_registry::native_return_options::NativeReturnOptionsApplication::Immediate,
                    0,
                    0,
                    root.as_ptr(),
                )
                .map(drop),
        };
        let first = NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::CommandProtocolUnavailable(
                "changed native primary publication slot",
            ),
        );
        assert_eq!(
            reached.unwrap_err().native_execution_refusal(),
            Some(&first)
        );
        assert_eq!(interpreter.native_execution_refusal(), Some(first.clone()));
        assert_eq!(callbacks.get(), 1);
        assert_eq!(MEMBER_FREES.with(Cell::get), 1);
        assert_eq!(MEMBER_REFS.with(Cell::get), 0);
        assert!(!obj::allocation_is_live(member_lifetime.as_ptr()));
        assert!(!obj::allocation_is_live(key_lifetime.as_ptr()));
        assert!(core::ptr::eq(
            obj::obj_type_ptr(root.as_ptr()),
            &obj::TCL_INT_TYPE,
        ));
        assert_eq!(obj::internal_rep(root.as_ptr()), 777);
        assert_eq!(unsafe { (*root.as_ptr()).ref_count }, 1);
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_eq!(
            arguments
                .check_for(&interpreter)
                .unwrap_err()
                .native_execution_refusal(),
            Some(&first),
        );
    }
}

fn original_copy_native() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl9.1"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn original_copy_series(length: usize, protocol: NativeStringProtocol) -> obj::Owned {
    let extent = length.to_string();
    let Ok(plan) = tcl_cmd_core::lseq::decode(&[extent.as_bytes()]) else {
        panic!("actual arithmetic series plan");
    };
    let series = tcl_cmd_core::lseq::prepare_series(&plan).unwrap().unwrap();
    obj::Owned::fresh(crate::native_arithseries::new_series(series, protocol).unwrap())
}

#[test]
fn original_ordinary_list_projection_keeps_fresh_series_members_and_captured_header_roles() {
    let interpreter = original_copy_native();
    let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1);
    let value = original_copy_series(3, protocol);
    let cached = crate::native_arithseries::elements(value.as_ptr(), protocol).unwrap();
    let old_members: Vec<_> = cached
        .iter()
        .map(|&member| obj::NativeObjectLifetime::retain(member))
        .collect();
    let arguments = OriginalArguments::enter(&interpreter, &[value.as_ptr()]).unwrap();
    let members = ordinary_list_elements_native_using(
        value.as_ptr(),
        protocol,
        &|| arguments.check_for(&interpreter),
        &|error| arguments.retain_refusal(error.into()),
    )
    .unwrap();
    assert_eq!(members.len(), 3);
    assert!(core::ptr::eq(
        obj::obj_type_ptr(value.as_ptr()),
        &TCL_LIST_TYPE
    ));
    assert!(!obj::has_string_rep(value.as_ptr()));
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
    for (coordinate, (&member, old)) in members.iter().zip(&old_members).enumerate() {
        assert_ne!(member, old.as_ptr());
        assert!(!obj::allocation_is_live(old.as_ptr()));
        assert_eq!(obj::internal_rep(member), coordinate as u64);
        assert_eq!(unsafe { (*member).ref_count }, 1);
    }
    arguments.check_for(&interpreter).unwrap();
    assert!(interpreter.native_execution_refusal().is_none());
}

#[test]
fn original_ordinary_list_projection_retirement_refuses_before_new_backing() {
    let mut interpreter = original_copy_native();
    let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1);
    let value = original_copy_series(2, protocol);
    let cached = crate::native_arithseries::elements(value.as_ptr(), protocol).unwrap();
    let old_members: Vec<_> = cached
        .iter()
        .map(|&member| obj::NativeObjectLifetime::retain(member))
        .collect();
    obj::change_type(cached[0], &MEMBER, 0);
    let arguments = OriginalArguments::enter(&interpreter, &[value.as_ptr()]).unwrap();
    interpreter.set_result_bytes(b"PRIOR");
    let prior = interpreter.get_obj_result();
    MEMBER_FREES.with(|calls| calls.set(0));
    MEMBER_REFS.with(|refs| refs.set(-1));
    crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
    let _current = CurrentInterp;
    let objects_before = crate::counters::live_objs();
    let failure = ordinary_list_elements_native_using(
        value.as_ptr(),
        protocol,
        &|| arguments.check_for(&interpreter),
        &|error| arguments.retain_refusal(error.into()),
    )
    .unwrap_err();
    let first = interpreter.native_execution_refusal().unwrap();
    assert!(
        matches!(&first, NativeExecutionError::HostCommandRefusal(reason)
        if reason.reason == "mandatory unused List member retirement")
    );
    assert_eq!(failure.native_execution_refusal(), Some(&first));
    assert_eq!(MEMBER_FREES.with(Cell::get), 1);
    assert_eq!(MEMBER_REFS.with(Cell::get), 0);
    assert!(old_members
        .iter()
        .all(|member| !obj::allocation_is_live(member.as_ptr())));
    assert_eq!(
        crate::counters::live_objs(),
        objects_before - cached.len() as i64
    );
    assert!(obj::obj_type_ptr(value.as_ptr()).is_null());
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
    assert!(!obj::has_string_rep(value.as_ptr()));
    assert_eq!(interpreter.get_obj_result(), prior);
    assert_eq!(
        arguments
            .check_for(&interpreter)
            .unwrap_err()
            .native_execution_refusal(),
        Some(&first)
    );
}

#[test]
fn original_list_copy_projection_keeps_shared_members_and_lazy_abstract_length() {
    let interpreter = original_copy_native();
    let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1);
    {
        let value = original();
        let arguments = OriginalArguments::enter(&interpreter, &[value.as_ptr()]).unwrap();
        MODE.with(|mode| mode.set(0));
        FREES.with(|calls| calls.set(0));
        let copy = native_list_copy_using(
            value.as_ptr(),
            protocol,
            &|| arguments.check_for(&interpreter),
            &|error| arguments.retain_refusal(error.into()),
        )
        .unwrap();
        let original_members = list_elements_native_checked(value.as_ptr(), protocol).unwrap();
        let copied_members = list_elements_native_checked(copy.as_ptr(), protocol).unwrap();
        assert_ne!(value.as_ptr(), copy.as_ptr());
        assert_eq!(original_members, copied_members);
        assert_eq!(FREES.with(Cell::get), 1);
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
        assert_eq!(unsafe { (*copy.as_ptr()).ref_count }, 1);
        assert!(original_members
            .iter()
            .all(|&member| unsafe { (*member).ref_count == 1 }));
        assert_eq!(obj::bytes_of(value.as_ptr()), b"a b");
        assert!(!obj::has_string_rep(copy.as_ptr()));
        drop(copy);
        assert!(original_members
            .iter()
            .all(|&member| unsafe { (*member).ref_count == 1 }));
        arguments.check_for(&interpreter).unwrap();
    }
    let length = tcl_cmd_core::lseq::MAX_MATERIALIZE as usize + 1;
    let value = original_copy_series(length, protocol);
    let arguments = OriginalArguments::enter(&interpreter, &[value.as_ptr()]).unwrap();
    let copy = native_list_copy_using(
        value.as_ptr(),
        protocol,
        &|| arguments.check_for(&interpreter),
        &|error| arguments.retain_refusal(error.into()),
    )
    .unwrap();
    assert_ne!(copy.as_ptr(), value.as_ptr());
    assert_eq!(
        crate::native_arithseries::physical_state(value.as_ptr()),
        Some((length, 2, false))
    );
    assert_eq!(
        crate::native_arithseries::physical_state(copy.as_ptr()),
        Some((length, 2, false))
    );
    assert!(!obj::has_string_rep(value.as_ptr()));
    assert!(!obj::has_string_rep(copy.as_ptr()));
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
    assert_eq!(unsafe { (*copy.as_ptr()).ref_count }, 1);
    drop(copy);
    assert_eq!(
        crate::native_arithseries::physical_state(value.as_ptr()),
        Some((length, 1, false))
    );
    arguments.check_for(&interpreter).unwrap();
    assert!(interpreter.native_execution_refusal().is_none());
}

#[test]
fn original_list_copy_projection_refuses_reached_retirement_before_fresh_header() {
    for mode in [1, 2, 3] {
        let mut interpreter = original_copy_native();
        let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1);
        let value = original();
        let arguments = OriginalArguments::enter(&interpreter, &[value.as_ptr()]).unwrap();
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        MODE.with(|selected| selected.set(mode));
        FREES.with(|calls| calls.set(0));
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
        let _current = CurrentInterp;
        let objects_before = crate::counters::live_objs();
        let Err(failure) = native_list_copy_using(
            value.as_ptr(),
            protocol,
            &|| arguments.check_for(&interpreter),
            &|error| arguments.retain_refusal(error.into()),
        ) else {
            panic!("real original primary retirement must refuse");
        };
        let first = interpreter.native_execution_refusal().unwrap();
        assert_eq!(failure.native_execution_refusal(), Some(&first));
        if mode == 1 {
            assert!(
                matches!(&first, NativeExecutionError::HostCommandRefusal(reason)
                if reason.reason == "original List primary retirement")
            );
            assert!(obj::obj_type_ptr(value.as_ptr()).is_null());
        } else {
            assert_eq!(
                first,
                NativeExecutionError::ValueAccessRefusal(
                    NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "changed native primary publication slot"
                    )
                )
            );
            assert!(!core::ptr::eq(
                obj::obj_type_ptr(value.as_ptr()),
                &TCL_LIST_TYPE
            ));
        }
        assert_eq!(crate::counters::live_objs(), objects_before);
        assert_eq!(obj::bytes_of(value.as_ptr()), b"a b");
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_eq!(
            arguments
                .check_for(&interpreter)
                .unwrap_err()
                .native_execution_refusal(),
            Some(&first)
        );
    }
}
