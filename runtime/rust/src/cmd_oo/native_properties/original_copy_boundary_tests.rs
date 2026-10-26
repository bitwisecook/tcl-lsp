// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real property-report header Copy after a reached original member getter.
use super::*;
use std::cell::Cell;
thread_local! {
    static OWNER: RefCell<Option<crate::interp::WeakInterp>> = const { RefCell::new(None) };
    static HEADER: Cell<*mut TclObj> = const { Cell::new(core::ptr::null_mut()) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static MEMBER_GETTERS: Cell<usize> = const { Cell::new(0) };
    static QUERY_GETTERS: Cell<usize> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static REFERENCES: Cell<obj::TclSize> = const { Cell::new(-1) };
    static OUTSIDE: Cell<bool> = const { Cell::new(false) };
}
struct Cleanup;
impl Drop for Cleanup {
    fn drop(&mut self) {
        MODE.with(|mode| mode.set(0));
        HEADER.with(|header| header.set(core::ptr::null_mut()));
        OWNER.with(|owner| *owner.borrow_mut() = None);
    }
}
extern "C" fn update_member(member: *mut TclObj) {
    MEMBER_GETTERS.with(|count| count.set(count.get() + 1));
    let header = HEADER.with(Cell::get);
    if !header.is_null() {
        // This is the actual cached report header reached earlier by lookup.
        // The callback removes its old List backing while its allocation lives.
        obj::change_type(header, &REPORT, 0);
        unsafe { obj::set_native_updater_string_rep(header, b"-a", false) };
    }
    unsafe { obj::set_native_updater_string_rep(member, b"-a", false) };
}
extern "C" fn update_query(query: *mut TclObj) {
    QUERY_GETTERS.with(|count| count.set(count.get() + 1));
    unsafe { obj::set_native_updater_string_rep(query, b"-a", false) };
}
extern "C" fn free_report(header: *mut TclObj) {
    FREES.with(|count| count.set(count.get() + 1));
    REFERENCES.with(|refs| refs.set(unsafe { (*header).ref_count }));
    let Some(mut interpreter) = OWNER.with(|owner| {
        owner
            .borrow()
            .as_ref()
            .and_then(crate::interp::WeakInterp::upgrade)
    }) else {
        return;
    };
    OUTSIDE.with(|outside| outside.set(interpreter.oo.try_borrow_mut().is_ok()));
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("original property Copy retirement");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("original-property-copy".into(), "1".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        _ => {}
    }
}
static MEMBER: obj::TclObjType = obj::TclObjType {
    name: c"originalPropertyCopyMember".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update_member),
    set_from_any_proc: None,
};
static QUERY: obj::TclObjType = obj::TclObjType {
    name: c"originalPropertyCopyQuery".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update_query),
    set_from_any_proc: None,
};
static REPORT: obj::TclObjType = obj::TclObjType {
    name: c"originalPropertyCopyReport".as_ptr(),
    free_int_rep_proc: Some(free_report),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
#[test]
fn original_property_table_copy_checks_retirement_after_member_getter_before_query() {

    // Software callback/currentness controls, not a native property chronology.
    for mode in [0, 1, 2] {
        let _cleanup = Cleanup;
        let mut interpreter = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("tcl9.1"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(
            interpreter.eval_str(b"oo::class create CopyClass; CopyClass create CopyObject"),
            Code::Ok
        );
        let target = interpreter.oo_resolve_object(b"CopyObject");
        let member = obj::Owned::fresh(obj::alloc_typed(&MEMBER, 0));
        unsafe { obj::set_native_updater_string_rep(member.as_ptr(), b"-a", false) };
        let incoming = obj::Owned::fresh(interpreter.new_list_object(&[member.as_ptr()]));
        set_property_slot(&mut interpreter, target, false, false, incoming.as_ptr()).unwrap();
        drop(incoming);
        let header = interpreter
            .native_all_property_header(target, false, false)
            .unwrap();
        assert_eq!(unsafe { (*header.header).ref_count }, 1);
        assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 3);
        HEADER.with(|slot| slot.set(header.header));
        OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
        MODE.with(|selected| selected.set(mode));
        MEMBER_GETTERS.with(|count| count.set(0));
        QUERY_GETTERS.with(|count| count.set(0));
        FREES.with(|count| count.set(0));
        OUTSIDE.with(|outside| outside.set(false));
        obj::invalidate_string(member.as_ptr());
        let query = obj::Owned::fresh(obj::alloc_typed(&QUERY, 0));
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        let mut table = None;
        let outcome = lookup_property(
            &mut interpreter,
            target,
            query.as_ptr(),
            false,
            &mut table,
            true,
        );
        assert_eq!(MEMBER_GETTERS.with(Cell::get), 1);
        assert_eq!(FREES.with(Cell::get), 1);
        assert_eq!(REFERENCES.with(Cell::get), 1);
        assert!(OUTSIDE.with(Cell::get));
        assert_eq!(unsafe { (*header.header).ref_count }, 1);
        assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 2);
        assert_eq!(interpreter.get_obj_result(), prior);
        if mode == 0 {
            assert!(outcome.is_ok());
            assert!(table.is_some());
            assert_eq!(QUERY_GETTERS.with(Cell::get), 1);
            assert!(interpreter.native_execution_refusal().is_none());
        } else {
            let Err(failure) = outcome else {
                panic!("original report retirement must refuse Copy");
            };
            assert_eq!(
                failure.native_execution_refusal(),
                interpreter.native_execution_refusal().as_ref()
            );
            assert!(table.is_none());
            assert_eq!(QUERY_GETTERS.with(Cell::get), 0);
            assert!(!core::ptr::eq(
                obj::obj_type_ptr(header.header),
                &crate::list::TCL_LIST_TYPE
            ));
            if mode == 1 {
                assert!(
                    matches!(interpreter.native_execution_refusal(), Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first)) if first.reason == "original property Copy retirement")
                );
            } else {
                assert!(matches!(interpreter.native_execution_refusal(), Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("stale entered native operation")))));
            }
        }
    }
}
