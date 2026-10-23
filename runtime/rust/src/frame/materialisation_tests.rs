// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original storage continuations; no Native callback chronology assertion.

use super::*;
use crate::{
    interp::{native_operation_currency::NativeOperationScope, Interp, WeakInterp},
    namespace::GLOBAL,
    obj::{self, Owned, TclObj},
};
use std::cell::Cell as Counter;
use tcl_cmd_core::native_array_search::{NativeArraySearchBackend, NativeArraySearchOperand};
use tcl_runtime_api::{ArrayTarget, NativeExecutionError};
use tcl_syntax::raw_string::NativeValueAccessRefusal;

thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static ARRAY: RefCell<Option<RetainedArrayCell>> = const { RefCell::new(None) };
    static MODE: Counter<u8> = const { Counter::new(0) };
    static STAGE: Counter<u8> = const { Counter::new(0) };
    static CALLS: Counter<usize> = const { Counter::new(0) };
    static INCOMING: Counter<*mut TclObj> = const { Counter::new(std::ptr::null_mut()) };
    static REFERENCES: Counter<obj::TclSize> = const { Counter::new(0) };
}
struct Cleanup;
impl Drop for Cleanup {
    fn drop(&mut self) {
        OWNER.with(|owner| owner.borrow_mut().take());
        ARRAY.with(|array| array.borrow_mut().take());
        INCOMING.with(|value| value.set(std::ptr::null_mut()));
    }
}
fn native(profile: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(profile),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn text(bytes: &[u8]) -> Owned {
    Owned::fresh(obj::new_string_bytes(bytes))
}
fn arm(interpreter: &Interp, mode: u8, stage: u8) -> Cleanup {
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|selected| selected.set(mode));
    STAGE.with(|selected| selected.set(stage));
    CALLS.with(|count| count.set(0));
    REFERENCES.with(|count| count.set(0));
    Cleanup
}
fn effect(original: *mut TclObj) {
    CALLS.with(|count| count.set(count.get() + 1));
    let Some(mut interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    else {
        return;
    };
    assert!(interpreter.original_variable_storage_borrows_are_available());
    if STAGE.with(Counter::get) == 0 {
        assert!(matches!(
            interpreter.frames.borrow().compiled_slot_var(0).as_deref(),
            Some(Var::Array(_))
        ));
        let incoming = INCOMING.with(Counter::get);
        REFERENCES.with(|count| count.set(unsafe { (*incoming).ref_count }));
    } else {
        REFERENCES.with(|count| count.set(unsafe { (*original).ref_count }));
        let array = ARRAY.with(|array| array.borrow().as_ref().unwrap().clone());
        assert!(array.contents.try_borrow_mut().is_ok());
        assert_eq!(
            array.read(b"k").map(|value| obj::bytes_of(value)),
            Some(b"V".to_vec())
        );
    }
    match MODE.with(Counter::get) {
        1 => {
            interpreter.refuse_host_command("first original materialisation stage");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("materialisation-stage".into(), "1.0".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        3 => {
            interpreter.frames.borrow_mut().pop().unwrap();
            interpreter.frames.borrow_mut().push(GLOBAL);
        }
        4 => {
            let array = ARRAY.with(|array| array.borrow().as_ref().unwrap().clone());
            let retired = array
                .contents
                .borrow_mut()
                .array_searches
                .take_handle(1)
                .unwrap();
            drop(retired);
            let replacement = text(b"s-1-a");
            array.contents.borrow_mut().array_searches.insert(
                1,
                Some(Owned::retain(replacement.as_ptr())),
                vec![b"k".to_vec()],
            );
        }
        _ => {}
    }
}
extern "C" fn retire(original: *mut TclObj) {
    effect(original);
}
extern "C" fn update(original: *mut TclObj) {
    effect(original);
    unsafe { obj::set_native_updater_string_rep(original, b"s-1-a", false) };
}
static OLD: obj::TclObjType = obj::TclObjType {
    name: c"originalMaterialisationRetirement".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
static LAZY: obj::TclObjType = obj::TclObjType {
    name: c"originalSearchHandleGetter".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
fn cause(interpreter: &Interp, mode: u8) {
    let first = interpreter
        .native_execution_refusal()
        .expect("actual original refusal");
    match mode {
        1 => assert!(
            matches!(first, NativeExecutionError::HostCommandRefusal(ref first) if first.reason == "first original materialisation stage")
        ),
        2 => assert_eq!(
            first,
            NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "stale entered native operation"
                )
            )
        ),
        _ => assert!(matches!(
            first,
            NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(_)
            )
        )),
    }
}

#[test]
fn original_formal_metadata_retires_before_adoption_under_the_same_frame_and_cell() {
    // naming.embedding.original-host-publication-and-fact-transport

    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2, 3] {
            let mut interpreter = native(profile);
            let protocol = interpreter
                .native_invocation_dialect()
                .native_compiled_variable_protocol()
                .unwrap();
            interpreter.frames.borrow_mut().push(GLOBAL);
            interpreter
                .frames
                .borrow_mut()
                .install_formal_cells(&[b"formal".to_vec()], protocol);
            let old_key = Owned::fresh(obj::alloc_typed(&OLD, 0));
            let old_value = text(b"OLD MEMBER");
            {
                let mut frames = interpreter.frames.borrow_mut();
                let (frame, slot) = frames.compiled_cell(0).unwrap();
                let table = &mut frames.frames[frame].table;
                table
                    .prepare_original_native_element_at(slot, b"k", Some(old_key.as_ptr()))
                    .unwrap();
                table
                    .store_elem(b"formal", b"k", old_value.as_ptr())
                    .unwrap();
            }
            let allocation = obj::NativeObjectLifetime::retain(old_key.as_ptr());
            drop(old_key);
            let incoming = text(b"INCOMING\0FORMAL");
            INCOMING.with(|value| value.set(incoming.as_ptr()));
            interpreter.set_result_bytes(b"PRIOR");
            let prior = interpreter.get_obj_result();
            let _cleanup = arm(&interpreter, mode, 0);
            let reached = interpreter.store_native_formal_cell_checked(0, incoming.as_ptr());
            assert_eq!(reached.is_ok(), mode == 0, "{profile}/{mode}");
            assert_eq!(CALLS.with(Counter::get), 1);
            assert_eq!(
                REFERENCES.with(Counter::get),
                2,
                "caller plus genuine incoming stored role before adoption"
            );
            assert!(obj::check_native_liveness(allocation.as_ptr()).is_err());
            assert_eq!(
                unsafe { (*incoming.as_ptr()).ref_count },
                if mode == 0 { 2 } else { 1 }
            );
            assert_eq!(interpreter.get_obj_result(), prior);
            assert_eq!(interpreter.entered_native_operation_count(), 0);
            if mode == 0 {
                assert!(
                    matches!(interpreter.frames.borrow().compiled_slot_var(0).as_deref(), Some(Var::Scalar(value)) if *value == incoming.as_ptr())
                );
            } else {
                cause(&interpreter, mode);
                if mode != 3 {
                    assert!(matches!(
                        interpreter.frames.borrow().compiled_slot_var(0).as_deref(),
                        Some(Var::Array(_))
                    ));
                }
            }
        }
    }
}

/// The actual LocateArray/trace owner supplies the backend's captured receipt.
/// A matching numeric cell identifier alone cannot issue that receipt.
fn search_with_original_target(
    interpreter: &mut Interp,
    expected: &ArrayTarget,
    sub: &str,
    name: &[u8],
    operand: Option<&NativeArraySearchOperand<'_, *mut TclObj>>,
    protocol: tcl_syntax::native_array_search::NativeArraySearchProtocol,
) -> Result<
    Result<*mut TclObj, tcl_syntax::native_array_search::NativeArraySearchFailure>,
    tcl_syntax::value::ValueError,
> {
    let mut reached = None;
    let code = interpreter.with_array_trace_target(name, |interpreter, original| {
        assert_eq!(original.cell_id(), expected.cell_id());
        reached =
            Some(interpreter.array_search_on_original(original, sub, name, operand, protocol));
        crate::interp::Code::Ok
    });
    assert_eq!(
        code,
        if interpreter.host_refusal_pending() {
            crate::interp::Code::Error
        } else {
            crate::interp::Code::Ok
        }
    );
    reached.expect("actual captured array operation reached the backend")
}

fn array(interpreter: &mut Interp) -> (RetainedArrayCell, ArrayTarget) {
    let name = text(b"a(k)");
    let value = text(b"V");
    interpreter
        .assign_original_named_variable(name.as_ptr(), value.as_ptr())
        .unwrap();
    let array = interpreter
        .namespaces()
        .var_table(GLOBAL)
        .capture_array_cell(b"a")
        .unwrap();
    let target = ArrayTarget::cell_bytes(tcl_runtime_api::FrameId(0), b"a", array.identity());
    (array, target)
}

#[test]
fn original_search_string_keeps_the_same_node_birth_without_a_native_snapshot_hold() {
    // naming.embedding.original-host-publication-and-fact-transport

    // The extension header is a software callback control, not a stock handle producer comparison.
    for profile in ["tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2, 4] {
            let mut interpreter = native(profile);
            let (array, target) = array(&mut interpreter);
            let protocol = interpreter.array_search_protocol().unwrap();
            let held = Owned::fresh(obj::alloc_typed(&LAZY, 0));
            array.contents.borrow_mut().array_searches.insert(
                1,
                Some(Owned::retain(held.as_ptr())),
                vec![b"k".to_vec()],
            );
            ARRAY.with(|selected| *selected.borrow_mut() = Some(array.clone()));
            let _cleanup = arm(&interpreter, mode, 1);
            let same_header = held.as_ptr();
            let same = NativeArraySearchOperand {
                original: &same_header,
                bytes: b"s-1-a",
                cache: None,
            };
            let direct = search_with_original_target(
                &mut interpreter,
                &target,
                "anymore",
                b"a",
                Some(&same),
                protocol,
            )
            .unwrap()
            .unwrap();
            crate::interp::drop_fresh(direct);
            assert_eq!(
                CALLS.with(Counter::get),
                0,
                "actual pointer match bypasses the retained handle getter"
            );
            let spelling = text(b"s-1-a");
            let original = spelling.as_ptr();
            let operand = NativeArraySearchOperand {
                original: &original,
                bytes: b"s-1-a",
                cache: None,
            };
            let reached = search_with_original_target(
                &mut interpreter,
                &target,
                "nextelement",
                b"a",
                Some(&operand),
                protocol,
            );
            assert_eq!(reached.is_ok(), mode == 0);
            assert_eq!(CALLS.with(Counter::get), 1);
            assert_eq!(
                REFERENCES.with(Counter::get),
                2,
                "caller and original chain only"
            );
            if mode == 0 {
                let result = reached.unwrap().unwrap();
                assert_eq!(obj::bytes_of(result), b"k");
                crate::interp::drop_fresh(result);
            } else {
                cause(&interpreter, mode);
                assert_eq!(
                    array
                        .contents
                        .borrow_mut()
                        .array_searches
                        .next_defined(1, false, |_| true),
                    Some(b"k".to_vec())
                );
            }
        }
    }
}

#[test]
fn original_done_search_retires_its_actual_handle_before_result_publication_outside_borrows() {
    // naming.embedding.original-host-publication-and-fact-transport

    for profile in ["tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2] {
            let mut interpreter = native(profile);
            let (array, target) = array(&mut interpreter);
            let protocol = interpreter.array_search_protocol().unwrap();
            let held = Owned::fresh(obj::alloc_typed(&OLD, 0));
            let original = held.as_ptr();
            let allocation = obj::NativeObjectLifetime::retain(original);
            array.contents.borrow_mut().array_searches.insert(
                1,
                Some(Owned::retain(original)),
                vec![b"k".to_vec()],
            );
            drop(held);
            ARRAY.with(|selected| *selected.borrow_mut() = Some(array.clone()));
            interpreter.set_result_bytes(b"PRIOR");
            let prior = interpreter.get_obj_result();
            let _cleanup = arm(&interpreter, mode, 1);
            let operand = NativeArraySearchOperand {
                original: &original,
                bytes: b"s-1-a",
                cache: None,
            };
            let scope = NativeOperationScope::enter(&interpreter).unwrap();
            let reached = search_with_original_target(
                &mut interpreter,
                &target,
                "donesearch",
                b"a",
                Some(&operand),
                protocol,
            );
            assert_eq!(reached.is_ok(), mode == 0);
            assert_eq!(CALLS.with(Counter::get), 1);
            assert_eq!(
                REFERENCES.with(Counter::get),
                0,
                "actual sole chain role reaches its last decrement"
            );
            assert!(!array.contents.borrow().array_searches.contains(1));
            assert!(obj::check_native_liveness(allocation.as_ptr()).is_err());
            assert_eq!(interpreter.get_obj_result(), prior);
            if mode == 0 {
                let result = reached.unwrap().unwrap();
                assert_eq!(obj::bytes_of(result), b"");
                crate::interp::drop_fresh(result);
            } else {
                cause(&interpreter, mode);
            }
            drop(scope);
            assert_eq!(interpreter.entered_native_operation_count(), 0);
        }
    }
}

#[test]
fn original_search_chain_and_formal_availability_keep_their_selected_physical_roles() {
    // naming.embedding.original-host-publication-and-fact-transport

    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut interpreter = native(profile);
        let Some(protocol) = interpreter.array_search_protocol() else {
            assert_eq!(profile, "jim");
            assert!(interpreter
                .native_invocation_dialect()
                .native_compiled_variable_protocol()
                .is_none());
            continue;
        };
        let (array, target) = array(&mut interpreter);
        let result = search_with_original_target(
            &mut interpreter,
            &target,
            "startsearch",
            b"a",
            None,
            protocol,
        )
        .unwrap()
        .unwrap();
        let returned = Owned::fresh(result);
        assert_eq!(
            unsafe { (*result).ref_count },
            if protocol.caches_handle() { 1 } else { 2 }
        );
        let snapshot = array.contents.borrow().array_searches.snapshot(|held| {
            (
                held.as_ptr(),
                obj::NativeObjectLifetime::retain(held.as_ptr()),
            )
        });
        assert_eq!(
            unsafe { (*result).ref_count },
            if protocol.caches_handle() { 1 } else { 2 },
            "metadata and allocation snapshot add no native role"
        );
        assert_eq!(
            snapshot.find_handle::<()>(|_| Ok(true)).unwrap().is_some(),
            !protocol.caches_handle()
        );
        let original = returned.as_ptr();
        let operand = NativeArraySearchOperand {
            original: &original,
            bytes: b"s-1-a",
            cache: protocol.parse(b"s-1-a").ok(),
        };
        let completed = interpreter
            .array_search_on_original(&target, "donesearch", b"a", Some(&operand), protocol)
            .unwrap()
            .unwrap();
        crate::interp::drop_fresh(completed);
        assert_eq!(
            unsafe { (*result).ref_count },
            1,
            "only the declared returned-value role remains"
        );
        assert!(!array.contents.borrow().array_searches.contains(1));
    }
}

#[test]
fn original_array_search_requires_the_actual_captured_operation_receipt() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Preserve the old fixture premise as an explicit negative: same numeric
    // cell/header observations do not replace the genuine LocateArray receipt.
    for profile in ["tcl9.0", "tcl9.1"] {
        let mut interpreter = native(profile);
        let (array, target) = array(&mut interpreter);
        let protocol = interpreter.array_search_protocol().unwrap();
        let held = Owned::fresh(obj::alloc_typed(&LAZY, 0));
        array.contents.borrow_mut().array_searches.insert(
            1,
            Some(Owned::retain(held.as_ptr())),
            vec![b"k".to_vec()],
        );
        ARRAY.with(|selected| *selected.borrow_mut() = Some(array.clone()));
        let _cleanup = arm(&interpreter, 0, 1);
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        let original = held.as_ptr();
        let operand = NativeArraySearchOperand {
            original: &original,
            bytes: b"s-1-a",
            cache: None,
        };
        let before = unsafe { (*original).ref_count };
        let error = interpreter
            .array_search_on_original(&target, "anymore", b"a", Some(&operand), protocol)
            .unwrap_err();
        assert!(matches!(
            error,
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("original array search cell")
        ));
        assert_eq!(CALLS.with(Counter::get), 0);
        assert_eq!(unsafe { (*original).ref_count }, before);
        assert!(array.contents.borrow().array_searches.contains(1));
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}
