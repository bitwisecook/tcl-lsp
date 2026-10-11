// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual concat operation stays entered through all selected getter/cleanup stages.
use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use crate::obj::TclObjType;
use std::cell::Cell;
thread_local! {
    static MODE: Cell<u8> = const { Cell::new(0) };
    static FIRST_GETTERS: Cell<usize> = const { Cell::new(0) };
}
extern "C" fn original_string(original: *mut TclObj) {
    FIRST_GETTERS.with(|count| count.set(count.get() + 1));
    let mut interpreter = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone();
    unsafe {
        obj::set_native_updater_string_rep(original, b"A", false);
    }
    if MODE.with(Cell::get) == 1 {
        interpreter.refuse_host_command("original concat getter");
    } else if MODE.with(Cell::get) == 2 {
        let original = interpreter.runtime_context();
        let mut changed = original.clone();
        changed.packages = vec![("concat-original-context".to_owned(), "1".to_owned())];
        interpreter.pin_context(&changed).unwrap();
        interpreter.pin_context(&original).unwrap();
    }
}
static ORIGINAL_STRING: TclObjType = TclObjType {
    name: c"originalConcatGetterControl".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(original_string),
    set_from_any_proc: None,
};
#[test]
fn original_concat_first_getter_refusal_stops_later_getters_and_result_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software original updater; native concat fixture windows remain independent.
    for mode in [1, 2] {
        let mut interpreter = fixture::native("tcl9.0");
        let _current = fixture::install(&mut interpreter, 0);
        MODE.with(|selected| selected.set(mode));
        FIRST_GETTERS.with(|count| count.set(0));
        let first = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL_STRING, 0));
        let later = fixture::later();
        let argv = [first.as_ptr(), later.as_ptr()];
        let arguments = OriginalArguments::enter(&interpreter, &argv).unwrap();
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        let before = crate::counters::live_objs();
        let failure = interpreter.concat_original_arguments(&argv).unwrap_err();
        assert_eq!(
            failure.native_execution_refusal(),
            interpreter.native_execution_refusal().as_ref()
        );
        if mode == 1 {
            assert!(matches!(failure.native_execution_refusal().unwrap(),
                tcl_runtime_api::NativeExecutionError::HostCommandRefusal(reason)
                if reason.reason == "original concat getter"));
        } else {
            assert_eq!(
                failure,
                tcl_cmd_core::CmdError::from(ValueError::CommandProtocolUnavailable(
                    "stale entered native operation"
                ))
            );
        }
        assert!(arguments.check_for(&interpreter).is_err());
        assert_eq!(FIRST_GETTERS.with(Cell::get), 1);
        assert_eq!(fixture::GETTERS.with(Cell::get), 0);
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_eq!(unsafe { (*first.as_ptr()).ref_count }, 1);
        assert_eq!(crate::counters::live_objs(), before);
    }
}
#[test]
fn original_concat_checked_source_conversion_retains_first_cause_before_working_cleanup() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Direct real append stage: the caller owns its existing working List.
    let mut interpreter = fixture::native("tcl9.0");
    let _current = fixture::install(&mut interpreter, 1);
    let target = obj::Owned::fresh(list::new_list_obj_native(
        &[],
        NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
    ));
    let source = fixture::original(b"A B");
    let argv = [target.as_ptr(), source.as_ptr()];
    let arguments = OriginalArguments::enter(&interpreter, &argv).unwrap();
    interpreter.set_result_bytes(b"PRIOR");
    let prior = interpreter.get_obj_result();
    let before = crate::counters::live_objs();
    let failure = append(&interpreter, target.as_ptr(), source.as_ptr()).unwrap_err();
    fixture::host(&interpreter.native_execution_refusal().unwrap());
    assert!(failure.native_access_refusal().is_some());
    assert!(arguments.check_for(&interpreter).is_err());
    assert_eq!(interpreter.get_obj_result(), prior);
    assert_eq!(fixture::FREES.with(Cell::get), 1);
    assert_eq!(fixture::REFS.with(Cell::get), 1);
    assert_eq!(unsafe { (*source.as_ptr()).ref_count }, 1);
    assert_eq!(
        list::native_list_backing(target.as_ptr())
            .unwrap()
            .elements()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(crate::counters::live_objs(), before);
}
#[test]
fn original_concat_authored_route_keeps_selected_portable_algorithm_without_native_entry() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Native capability is absent; pure operation currency is no capability grant.
    let mut interpreter = Interp::new();
    let first = fixture::string(b" a ");
    let second = fixture::string(b"{b c}");
    let result = obj::Owned::fresh(
        interpreter
            .concat_original_arguments(&[first.as_ptr(), second.as_ptr()])
            .unwrap(),
    );
    assert_eq!(obj::bytes_of(result.as_ptr()), b"a {b c}");
    assert!(interpreter.native_execution_refusal().is_none());
    assert_eq!(unsafe { (*first.as_ptr()).ref_count }, 1);
    assert_eq!(unsafe { (*second.as_ptr()).ref_count }, 1);
}
