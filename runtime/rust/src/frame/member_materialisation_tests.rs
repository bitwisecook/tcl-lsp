// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software callback controls for the original C member continuation.

use super::*;
use crate::{
    interp::{native_operation_currency::NativeOperationScope, Code, Interp, WeakInterp},
    namespace::GLOBAL,
    obj::{self, Owned, TclObj},
};
use std::cell::Cell as Counter;
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::NativeValueAccessRefusal;

thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static ARRAY: RefCell<Option<RetainedArrayCell>> = const { RefCell::new(None) };
    static MODE: Counter<u8> = const { Counter::new(0) };
    static CALLS: Counter<usize> = const { Counter::new(0) };
    static BIRTH: Counter<Option<tcl_runtime_api::VarId>> = const { Counter::new(None) };
    static REACHED: Counter<*mut TclObj> = const { Counter::new(std::ptr::null_mut()) };
}

struct Cleanup;
impl Drop for Cleanup {
    fn drop(&mut self) {
        OWNER.with(|owner| owner.borrow_mut().take());
        ARRAY.with(|array| array.borrow_mut().take());
        REACHED.with(|result| result.set(std::ptr::null_mut()));
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
fn assign(interpreter: &mut Interp, name: &[u8], value: &[u8]) {
    let name = text(name);
    let value = text(value);
    interpreter
        .assign_original_named_variable(name.as_ptr(), value.as_ptr())
        .unwrap();
}
fn array(interpreter: &Interp, name: &[u8]) -> RetainedArrayCell {
    interpreter
        .namespaces()
        .var_table(GLOBAL)
        .capture_array_cell(name)
        .unwrap()
}
fn arm(interpreter: &Interp, original: &RetainedArrayCell, mode: u8) -> Cleanup {
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    ARRAY.with(|array| *array.borrow_mut() = Some(original.clone()));
    MODE.with(|selected| selected.set(mode));
    CALLS.with(|calls| calls.set(0));
    BIRTH.with(|birth| birth.set(None));
    Cleanup
}
extern "C" fn retire(original: *mut TclObj) {
    CALLS.with(|calls| calls.set(calls.get() + 1));
    let Some(mut interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    else {
        return;
    };
    let array = ARRAY.with(|array| array.borrow().as_ref().unwrap().clone());
    assert!(interpreter.original_variable_storage_borrows_are_available());
    assert!(array.contents.try_borrow_mut().is_ok());
    assert_eq!(
        unsafe { (*original).ref_count },
        0,
        "the chain's real last reference"
    );
    {
        let cell = array.contents.borrow();
        let member = cell.element_ids[b"new".as_slice()];
        assert!(cell.member_order.contains_key(b"new"));
        assert!(!cell.rmw_member_shells.contains(b"new".as_slice()));
        assert!(
            matches!(cell.var.as_ref(), Some(Var::Array(values)) if !values.contains_key(b"new".as_slice()))
        );
        let key = cell
            .native_member_keys
            .original_key(member, b"new")
            .unwrap();
        assert_eq!(
            unsafe { (*key).ref_count },
            if interpreter
                .native_c_variable_name_protocol()
                .unwrap()
                .version()
                < tcl_dialect::TclVersion::V9_0
            {
                2
            } else {
                3
            },
            "actual parsed/temporary/table roles at the reached search retirement"
        );
        BIRTH.with(|birth| birth.set(Some(member)));
    }
    assert_eq!(
        interpreter.var_get(b"a(k)").map(obj::bytes_of),
        Some(b"V".to_vec())
    );
    match MODE.with(Counter::get) {
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("member-retirement".into(), "1.0".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        3 => {
            assert_eq!(interpreter.eval_str(b"unset a"), Code::Ok);
            assign(&mut interpreter, b"a(k)", b"REBORN");
        }
        4 | 5 => {
            assert_eq!(interpreter.eval_str(b"upvar #0 ::b alias"), Code::Ok);
            if MODE.with(Counter::get) == 5 {
                assert_eq!(interpreter.eval_str(b"upvar #0 ::a alias"), Code::Ok);
            }
        }
        6 => {
            // Remove and recreate the reached member while keeping its parent.
            // This is genuine storage mutation, with no protective native hold.
            let (_, retired) = array.publish_remove(b"new");
            retired.finish(|| Ok::<_, ()>(())).unwrap();
            assign(&mut interpreter, b"a(new)", b"CALLBACK");
        }
        _ => {}
    }
    interpreter.set_result_bytes(b"REACHED MEMBER CALLBACK");
    REACHED.with(|result| result.set(interpreter.get_obj_result()));
    if MODE.with(Counter::get) == 1 {
        interpreter.refuse_host_command("first original member retirement");
    }
}
static SEARCH: obj::TclObjType = obj::TclObjType {
    name: c"originalMemberSearchRetirement".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn search(array: &RetainedArrayCell) -> obj::NativeObjectLifetime {
    let handle = Owned::fresh(obj::alloc_typed(&SEARCH, 0));
    let allocation = obj::NativeObjectLifetime::retain(handle.as_ptr());
    array.contents.borrow_mut().array_searches.insert(
        1,
        Some(Owned::retain(handle.as_ptr())),
        vec![b"k".to_vec()],
    );
    drop(handle);
    allocation
}
fn cause(interpreter: &Interp, mode: u8) {
    let first = interpreter
        .native_execution_refusal()
        .expect("full original refusal");
    match mode {
        1 => assert!(
            matches!(first, NativeExecutionError::HostCommandRefusal(ref first) if first.reason == "first original member retirement")
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
fn original_c_member_birth_precedes_search_retirement_and_keeps_the_selected_parent() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The extension search handle is a software control. No stock callback timing is asserted.
    for profile in ["tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2, 3, 6] {
            let mut interpreter = native(profile);
            assign(&mut interpreter, b"a(k)", b"V");
            let original = array(&interpreter, b"a");
            let allocation = search(&original);
            let _cleanup = arm(&interpreter, &original, mode);
            let name = text(b"a(new)");
            let value = text(b"OUTER");
            let reached = interpreter.assign_original_named_variable(name.as_ptr(), value.as_ptr());
            assert_eq!(reached.is_ok(), mode == 0, "{profile}/{mode}");
            assert_eq!(CALLS.with(Counter::get), 1);
            assert!(obj::check_native_liveness(allocation.as_ptr()).is_err());
            assert_eq!(
                unsafe { (*value.as_ptr()).ref_count },
                if mode == 0 { 2 } else { 1 }
            );
            assert_eq!(interpreter.get_obj_result(), REACHED.with(Counter::get));
            assert_eq!(interpreter.entered_native_operation_count(), 0);
            if mode == 0 {
                assert_eq!(original.read(b"new"), Some(value.as_ptr()));
                assert_eq!(
                    original.contents.borrow().element_ids[b"new".as_slice()],
                    BIRTH.with(Counter::get).unwrap()
                );
            } else {
                cause(&interpreter, mode);
                if mode == 6 {
                    assert_eq!(
                        original.read(b"new").map(obj::bytes_of),
                        Some(b"CALLBACK".to_vec())
                    );
                    assert_ne!(
                        original.contents.borrow().element_ids[b"new".as_slice()],
                        BIRTH.with(Counter::get).unwrap()
                    );
                } else if mode != 3 {
                    assert_eq!(original.read(b"new"), None);
                    assert!(!original
                        .contents
                        .borrow()
                        .rmw_member_shells
                        .contains(b"new".as_slice()));
                }
            }
        }
    }
}

#[test]
fn original_c_alias_selection_cannot_adopt_a_rebound_or_restored_alias_after_retirement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Weak observations supply no new alias/hash role and refuse a restored spelling's new issuer.
    for profile in ["tcl9.0", "tcl9.1"] {
        for mode in [0, 4, 5] {
            let mut interpreter = native(profile);
            assign(&mut interpreter, b"a(k)", b"V");
            assign(&mut interpreter, b"b(k)", b"OTHER");
            assert_eq!(interpreter.eval_str(b"upvar #0 ::a alias"), Code::Ok);
            let original = array(&interpreter, b"a");
            let other = array(&interpreter, b"b");
            let allocation = search(&original);
            let _cleanup = arm(&interpreter, &original, mode);
            let name = text(b"alias(new)");
            let value = text(b"OUTER");
            let reached = interpreter.assign_original_named_variable(name.as_ptr(), value.as_ptr());
            assert_eq!(reached.is_ok(), mode == 0, "{profile}/{mode}");
            assert_eq!(CALLS.with(Counter::get), 1);
            assert!(obj::check_native_liveness(allocation.as_ptr()).is_err());
            assert_eq!(interpreter.get_obj_result(), REACHED.with(Counter::get));
            assert_eq!(other.read(b"new"), None);
            assert_eq!(
                unsafe { (*value.as_ptr()).ref_count },
                if mode == 0 { 2 } else { 1 }
            );
            if mode == 0 {
                assert_eq!(original.read(b"new"), Some(value.as_ptr()));
            } else {
                cause(&interpreter, mode);
                assert_eq!(original.read(b"new"), None);
            }
            assert_eq!(interpreter.entered_native_operation_count(), 0);
        }
    }
}

#[test]
fn original_c_member_receipt_refuses_late_member_rebirth_before_receiver_capture() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for transition in [0, 1, 2] {
            let replace = transition == 1;
            let mut interpreter = native(profile);
            assign(&mut interpreter, b"a(k)", b"V");
            let original = array(&interpreter, b"a");
            let key = text(b"new");
            let scope = NativeOperationScope::enter(&interpreter).unwrap();
            let prepared = crate::vars::capture_original_c_element_materialisation(
                &mut interpreter.frames.borrow_mut(),
                &mut interpreter.namespaces_mut(),
                crate::vars::NativeOriginalNameScope {
                    current: GLOBAL,
                    namespace: Some(GLOBAL),
                },
                b"a",
                b"new",
                Some(key.as_ptr()),
                true,
                interpreter.native_c_variable_name_protocol().unwrap(),
            )
            .unwrap_or_else(|_| panic!("actual original member preparation"));
            let selected = prepared.finish(&interpreter, scope.currency()).unwrap();
            assert_eq!(
                unsafe { (*key.as_ptr()).ref_count },
                2,
                "caller plus actual member key only"
            );
            let old = original.contents.borrow().element_ids[b"new".as_slice()];
            if replace {
                let (_, retired) = original.publish_remove(b"new");
                retired.finish(|| Ok::<_, ()>(())).unwrap();
                assign(&mut interpreter, b"a(new)", b"REBORN");
                assert_ne!(
                    original.contents.borrow().element_ids[b"new".as_slice()],
                    old
                );
            }
            let scope = if transition == 2 {
                let original_context = interpreter.runtime_context();
                let mut changed = original_context.clone();
                changed
                    .packages
                    .push(("late-member-context".into(), "1.0".into()));
                interpreter.pin_context(&changed).unwrap();
                interpreter.pin_context(&original_context).unwrap();
                drop(scope);
                // A separately entered current scope cannot refresh the
                // selection's retained original currency.
                NativeOperationScope::enter(&interpreter).unwrap()
            } else {
                scope
            };
            let link = selected.capture_link_target(&interpreter, scope.currency());
            assert_eq!(link.is_ok(), transition == 0);
            let mut link = link.ok();
            if let Some(target) = &mut link {
                assert!(target.native_scalar_entry.is_none());
                assert!(target.native_element_entry.is_none());
                assert_eq!(target.array_identity, Some(original.identity()));
                assert_eq!(
                    unsafe { (*key.as_ptr()).ref_count },
                    2,
                    "original link projection takes no native key role"
                );
                crate::vars::prepare_upvar_target(
                    &mut interpreter.frames.borrow_mut(),
                    &mut interpreter.namespaces_mut(),
                    target,
                )
                .unwrap();
                assert!(target.native_element_entry.is_some());
                assert!(target.native_scalar_entry.is_none());
            }
            let reached = selected.capture_receiver(&interpreter, scope.currency());
            assert_eq!(reached.is_ok(), transition == 0);
            if replace {
                cause(&interpreter, 6);
                assert_eq!(unsafe { (*key.as_ptr()).ref_count }, 1);
                assert_eq!(
                    original.read(b"new").map(obj::bytes_of),
                    Some(b"REBORN".to_vec())
                );
            } else if transition == 2 {
                cause(&interpreter, 2);
                assert_eq!(unsafe { (*key.as_ptr()).ref_count }, 2);
                assert_eq!(original.read(b"new"), None);
            } else {
                let (receiver, home) = reached.unwrap();
                assert_eq!(home.base, b"a");
                assert_eq!(home.link_elem.as_deref(), Some(b"new".as_slice()));
                assert_eq!(receiver.trace_member(), Some((b"new".to_vec(), old)));
                assert_eq!(
                    unsafe { (*key.as_ptr()).ref_count },
                    2,
                    "late receiver adds no native key reference"
                );
                drop(receiver);
            }
            drop(link);
            drop(scope);
            assert_eq!(interpreter.entered_native_operation_count(), 0);
        }
    }
}

#[test]
fn original_c_legacy_search_cursor_retirement_adds_no_native_handle_role() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    use tcl_cmd_core::native_array_search::NativeArraySearchBackend;
    for profile in ["tcl8.5", "tcl8.6"] {
        let mut interpreter = native(profile);
        assign(&mut interpreter, b"a(k)", b"V");
        let original = array(&interpreter, b"a");
        let protocol = interpreter.array_search_protocol().unwrap();
        assert!(protocol.caches_handle());
        let target = tcl_runtime_api::ArrayTarget::cell_bytes(
            tcl_runtime_api::FrameId(0),
            b"a",
            original.identity(),
        );
        let result = interpreter
            .array_search_on_original(&target, "startsearch", b"a", None, protocol)
            .unwrap()
            .unwrap();
        let returned = Owned::fresh(result);
        assert_eq!(
            unsafe { (*result).ref_count },
            1,
            "only the actual returned-header caller"
        );
        let snapshot = original
            .contents
            .borrow()
            .array_searches
            .snapshot(Owned::as_ptr);
        assert!(snapshot.contains(1));
        assert!(snapshot.find_handle::<()>(|_| Ok(true)).unwrap().is_none());
        let name = text(b"a(new)");
        let value = text(b"OUTER");
        interpreter
            .assign_original_named_variable(name.as_ptr(), value.as_ptr())
            .unwrap();
        assert!(!original.contents.borrow().array_searches.contains(1));
        assert_eq!(
            unsafe { (*result).ref_count },
            1,
            "cursor retirement owns no original handle decrement"
        );
        assert_eq!(original.read(b"new"), Some(value.as_ptr()));
        let cell = original.contents.borrow();
        let member = cell.element_ids[b"new".as_slice()];
        let key = cell
            .native_member_keys
            .original_key(member, b"new")
            .unwrap();
        assert_eq!(
            unsafe { (*key).ref_count },
            1,
            "the actual table key after its preparation temporary retires"
        );
        drop(cell);
        drop(returned);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        assert!(interpreter.native_execution_refusal().is_none());
    }
}
