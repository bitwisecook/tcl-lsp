// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software controls for original List conversion before Each continuations.
use super::*;
use crate::obj::{self, Owned, TclObjType};
use std::cell::Cell;
use tcl_runtime_api::NativeExecutionError;

thread_local! {
    pub(crate) static MODE: Cell<u8> = const { Cell::new(0) };
    pub(crate) static FREES: Cell<usize> = const { Cell::new(0) };
    pub(crate) static REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
    pub(crate) static GETTERS: Cell<usize> = const { Cell::new(0) };
}
pub(crate) fn native(dialect: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(dialect),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
pub(crate) struct Current;
impl Drop for Current {
    fn drop(&mut self) {
        MODE.with(|mode| mode.set(0));
        crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
    }
}
pub(crate) fn install(interpreter: &mut Interp, mode: u8) -> Current {
    MODE.with(|selected| selected.set(mode));
    FREES.with(|count| count.set(0));
    REFS.with(|refs| refs.set(-1));
    GETTERS.with(|count| count.set(0));
    crate::codegen_abi::tcl_runtime_set_current_interp(interpreter);
    Current
}
extern "C" fn free_original(original: *mut TclObj) {
    let first = FREES.with(|count| {
        let first = count.get() == 0;
        count.set(count.get() + 1);
        first
    });
    if !first {
        return;
    }
    REFS.with(|refs| refs.set(unsafe { (*original).ref_count }));
    let mut interpreter = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone();
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("original List consumer retirement");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("original-list-consumer".to_owned(), "1".to_owned())];
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        3 => {
            let mut frames = interpreter.frames.borrow_mut();
            assert!(frames.pop().is_some());
            frames.push(crate::namespace::GLOBAL);
        }
        _ => {}
    }
}
extern "C" fn later_string(original: *mut TclObj) {
    GETTERS.with(|count| count.set(count.get() + 1));
    unsafe {
        obj::set_native_updater_string_rep(original, b"LATER", false);
    }
}
static ORIGINAL: TclObjType = TclObjType {
    name: c"originalListConsumerRetirement".as_ptr(),
    free_int_rep_proc: Some(free_original),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
static LATER: TclObjType = TclObjType {
    name: c"originalListConsumerLaterGetter".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(later_string),
    set_from_any_proc: None,
};
pub(crate) fn original(bytes: &[u8]) -> Owned {
    let owner = Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
    unsafe {
        obj::set_native_updater_string_rep(owner.as_ptr(), bytes, false);
    }
    owner
}
pub(crate) fn later() -> Owned {
    Owned::fresh(obj::alloc_typed(&LATER, 0))
}
pub(crate) fn string(bytes: &[u8]) -> Owned {
    Owned::fresh(obj::new_string_bytes(bytes))
}
pub(crate) fn host(first: &NativeExecutionError) {
    assert!(
        matches!(first, NativeExecutionError::HostCommandRefusal(reason)
        if reason.reason == "original List consumer retirement")
    );
}
#[test]
fn original_each_variable_copy_retirement_refuses_before_value_and_body_getters() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software free callback, not a native C/Jim callback chronology claim.
    for mode in [1, 2] {
        let mut interpreter = native("tcl8.6");
        let _current = install(&mut interpreter, mode);
        let command = string(b"foreach");
        let names = original(b"v");
        let values = later();
        let body = later();
        let argv = [
            command.as_ptr(),
            names.as_ptr(),
            values.as_ptr(),
            body.as_ptr(),
        ];
        let arguments = OriginalArguments::enter(&interpreter, &argv).unwrap();
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        let before = crate::counters::live_objs();
        assert_eq!(each_loop(&mut interpreter, &argv, false), Code::Error);
        let first = interpreter.native_execution_refusal().unwrap();
        if mode == 1 {
            host(&first);
        } else {
            assert!(matches!(
                first,
                NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "stale entered native operation"
                    )
                )
            ));
        }
        assert!(arguments.check_for(&interpreter).is_err());
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_eq!(GETTERS.with(Cell::get), 0);
        assert_eq!(FREES.with(Cell::get), 1);
        assert_eq!(REFS.with(Cell::get), 1);
        assert_eq!(unsafe { (*names.as_ptr()).ref_count }, 1);
        assert!(!core::ptr::eq(
            obj::obj_type_ptr(names.as_ptr()),
            &crate::list::TCL_LIST_TYPE
        ));
        assert_eq!(crate::counters::live_objs(), before);
    }
}
#[test]
fn original_each_value_copy_retirement_keeps_prepared_variable_owner_and_first_cause() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interpreter = native("tcl8.6");
    let _current = install(&mut interpreter, 1);
    let command = string(b"foreach");
    let names = string(b"v");
    let values = original(b"A");
    let body = later();
    let argv = [
        command.as_ptr(),
        names.as_ptr(),
        values.as_ptr(),
        body.as_ptr(),
    ];
    let arguments = OriginalArguments::enter(&interpreter, &argv).unwrap();
    interpreter.set_result_bytes(b"PRIOR");
    let prior = interpreter.get_obj_result();
    assert_eq!(each_loop(&mut interpreter, &argv, false), Code::Error);
    host(&interpreter.native_execution_refusal().unwrap());
    assert!(arguments.check_for(&interpreter).is_err());
    assert_eq!(interpreter.get_obj_result(), prior);
    assert_eq!(GETTERS.with(Cell::get), 0);
    assert_eq!(FREES.with(Cell::get), 1);
    assert_eq!(REFS.with(Cell::get), 1);
    assert_eq!(unsafe { (*values.as_ptr()).ref_count }, 1);
    assert!(core::ptr::eq(
        obj::obj_type_ptr(names.as_ptr()),
        &crate::list::TCL_LIST_TYPE
    ));
}
