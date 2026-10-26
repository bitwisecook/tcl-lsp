// SPDX-License-Identifier: AGPL-3.0-or-later
//! Genuine Runtime declaration references and callbacks, separate from C source chronology.
use super::*;
use crate::interp::WeakInterp;
use std::cell::Cell;
thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static TARGET: Cell<OoId> = const { Cell::new(OoId(0)) };
    static PRIVATE: Cell<bool> = const { Cell::new(false) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static INCOMING: RefCell<Vec<*mut TclObj>> = const { RefCell::new(Vec::new()) };
    static KEYS: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static OLD_FULL: Cell<*mut TclObj> = const { Cell::new(std::ptr::null_mut()) };
}
extern "C" fn key(original: *mut TclObj) {
    let value = obj::internal_rep(original);
    KEYS.with(|calls| calls.borrow_mut().push(value));
    let bytes = if value == 42 {
        b"key".as_slice()
    } else {
        b"later".as_slice()
    };
    unsafe {
        obj::set_native_updater_string_rep(original, bytes, false);
    }
}
static KEY: obj::TclObjType = obj::TclObjType {
    name: c"mappingOriginalKey".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(key),
    set_from_any_proc: None,
};
extern "C" fn old(original: *mut TclObj) {
    let mut interpreter = OWNER
        .with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
        .unwrap();
    FREES.with(|calls| calls.set(calls.get() + 1));
    assert!(interpreter.oo.try_borrow_mut().is_ok());
    assert_eq!(unsafe { (*original).ref_count }, 0);
    assert!(!obj::allocation_is_live(original));
    let key = (TARGET.with(Cell::get), true, PRIVATE.with(Cell::get));
    {
        let state = interpreter.oo.borrow();
        let visible = slot(&state, key).unwrap();
        assert_eq!(
            visible.len(),
            1,
            "old roster stays visible at the real decrement"
        );
        assert_eq!(visible[0].original.as_ptr(), original);
        assert!(matches!(
            visible[0].original,
            DeclaredVariableHeader::Observed(_)
        ));
        if key.2 {
            let full = visible[0].full_name.as_ref().unwrap().original.as_ptr();
            assert_eq!(full, OLD_FULL.with(Cell::get));
            assert_eq!(
                unsafe { (*full).ref_count },
                1,
                "original role retires before stored fullName"
            );
        }
    }
    INCOMING.with(|values| {
        for &value in values.borrow().iter() {
            assert_eq!(
                unsafe { (*value).ref_count },
                3,
                "caller, List member, actual incoming mapping role"
            );
            obj::invalidate_string(value);
        }
    });
    match MODE.with(Cell::get) {
        1 => {
            interpreter.set_result_bytes(b"REACHED MAPPING RETIREMENT");
            interpreter.refuse_host_command("first mapping retirement");
        }
        2 => {
            interpreter.set_result_bytes(b"REACHED MAPPING RETIREMENT");
            let before = interpreter.runtime_context();
            let mut changed = before.clone();
            changed
                .packages
                .push(("mapping-retirement".into(), "1.0".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&before).unwrap();
        }
        3 => {
            interpreter.oo_destroy_class(TARGET.with(Cell::get));
        }
        5 => {
            let slot = interpreter.oo_resolve_object(b"::oo::define::variable");
            assert_eq!(slot_inst_get(&mut interpreter, slot, &[]), Code::Error);
            assert!(interpreter.native_execution_refusal().is_some());
        }
        6 | 7 => {
            interpreter
                .frames
                .borrow_mut()
                .push(crate::namespace::GLOBAL);
            if MODE.with(Cell::get) == 7 {
                let popped = interpreter.frames.borrow_mut().pop();
                assert_eq!(popped, Some(crate::namespace::GLOBAL));
            }
        }
        4 => {
            let replacement = obj::Owned::fresh(obj::new_string_bytes(b"nested"));
            assert_eq!(publish(&mut interpreter, &[replacement.as_ptr()]), Code::Ok);
            assert_eq!(unsafe { (*replacement.as_ptr()).ref_count }, 2);
        }
        _ => {}
    }
}
static OLD: obj::TclObjType = obj::TclObjType {
    name: c"mappingOriginalRetirement".as_ptr(),
    free_int_rep_proc: Some(old),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn provided(engine: &str, private: bool) -> Interp {
    let mut interpreter = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(engine),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(
        interpreter.eval_str(b"oo::class create MappingClass {}"),
        Code::Ok
    );
    let target = interpreter.oo_resolve_object(b"MappingClass");
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    TARGET.with(|value| value.set(target));
    PRIVATE.with(|value| value.set(private));
    MODE.with(|mode| mode.set(0));
    FREES.with(|calls| calls.set(0));
    OLD_FULL.with(|value| value.set(std::ptr::null_mut()));
    INCOMING.with(|values| values.borrow_mut().clear());
    KEYS.with(|calls| calls.borrow_mut().clear());
    interpreter
}
fn publish(interpreter: &mut Interp, values: &[*mut TclObj]) -> Code {
    let head = obj::Owned::fresh(obj::new_string_bytes(b"oo::define"));
    let name = obj::Owned::fresh(obj::new_string_bytes(b"MappingClass"));
    let list = obj::Owned::fresh(interpreter.new_list_object(values));
    let argv = [head.as_ptr(), name.as_ptr(), list.as_ptr()];
    let target = DefTarget::Class(TARGET.with(Cell::get));
    let caller = interpreter.oo_enter_definition(&target, &argv).unwrap();
    let definition_level = interpreter.frames.borrow().current_level();
    let old_private = interpreter.oo.borrow().private_depth;
    interpreter.oo.borrow_mut().private_depth = usize::from(PRIVATE.with(Cell::get));
    let slot = interpreter.oo_resolve_object(b"::oo::define::variable");
    let originals = OriginalArguments::enter(interpreter, &[list.as_ptr()]).unwrap();
    let code = slot_inst_set_original(interpreter, slot, &[list.as_ptr()], &originals);
    interpreter.oo.borrow_mut().private_depth = old_private;
    if MODE.with(Cell::get) == 6 {
        assert_eq!(
            interpreter.frames.borrow().current_level(),
            definition_level + 1
        );
        let popped = interpreter.frames.borrow_mut().pop();
        assert_eq!(popped, Some(crate::namespace::GLOBAL));
    }
    interpreter.oo_leave_definition(caller);
    drop(list);
    drop(name);
    drop(head);
    originals.finish(code)
}
fn install_old(
    interpreter: &mut Interp,
) -> (obj::NativeObjectLifetime, Option<obj::NativeObjectLifetime>) {
    let original = obj::Owned::fresh(obj::alloc_typed(&OLD, 0));
    unsafe {
        obj::set_string_rep(original.as_ptr(), b"old");
    }
    assert_eq!(publish(interpreter, &[original.as_ptr()]), Code::Ok);
    let lease = obj::NativeObjectLifetime::retain(original.as_ptr());
    let full = {
        let state = interpreter.oo.borrow();
        slot(
            &state,
            (TARGET.with(Cell::get), true, PRIVATE.with(Cell::get)),
        )
        .unwrap()[0]
            .full_name
            .as_ref()
            .map(|full| {
                OLD_FULL.with(|pointer| pointer.set(full.original.as_ptr()));
                obj::NativeObjectLifetime::retain(full.original.as_ptr())
            })
    };
    drop(original);
    assert_eq!(FREES.with(Cell::get), 0);
    (lease, full)
}
fn operand(number: u64) -> obj::Owned {
    let original = obj::Owned::fresh(obj::alloc_typed(&KEY, number));
    unsafe {
        obj::set_string_rep(
            original.as_ptr(),
            if number == 42 { b"key" } else { b"later" },
        );
    }
    original
}
fn reset() {
    OWNER.with(|owner| *owner.borrow_mut() = None);
    INCOMING.with(|values| values.borrow_mut().clear());
}
#[test]
fn original_runtime_mapping_retires_old_visible_roles_before_unique_original_keys() {

    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interpreter = provided(engine, false);
        let (old, _) = install_old(&mut interpreter);
        let first = operand(42);
        let equal = operand(42);
        let later = operand(43);
        let values = [first.as_ptr(), equal.as_ptr(), later.as_ptr()];
        INCOMING.with(|incoming| *incoming.borrow_mut() = values.to_vec());
        assert_eq!(publish(&mut interpreter, &values), Code::Ok);
        assert_eq!(KEYS.with(|calls| calls.borrow().clone()), [42, 42, 43]);
        assert_eq!(FREES.with(Cell::get), 1);
        assert!(!obj::allocation_is_live(old.as_ptr()));
        let state = interpreter.oo.borrow();
        let map = slot(&state, (TARGET.with(Cell::get), true, false)).unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map[0].original.as_ptr(), first.as_ptr());
        assert_eq!(map[1].original.as_ptr(), later.as_ptr());
        assert_eq!(unsafe { (*first.as_ptr()).ref_count }, 2);
        assert_eq!(unsafe { (*equal.as_ptr()).ref_count }, 1);
        assert!(state.variable_mapping_retirements.is_empty());
        drop(state);
        reset();
    }
}
#[test]
fn original_runtime_mapping_first_retirement_refusal_stops_keys_and_cleans_matching_visibility() {

    for private in [false, true] {
        for mode in [1, 2, 3, 5] {
            let mut interpreter = provided("tcl9.1", private);
            let (old, full) = install_old(&mut interpreter);
            let first = operand(42);
            let later = operand(43);
            let values = [first.as_ptr(), later.as_ptr()];
            INCOMING.with(|incoming| *incoming.borrow_mut() = values.to_vec());
            MODE.with(|value| value.set(mode));
            assert_eq!(publish(&mut interpreter, &values), Code::Error);
            assert_eq!(FREES.with(Cell::get), 1);
            assert!(KEYS.with(|calls| calls.borrow().is_empty()));
            assert!(!obj::allocation_is_live(old.as_ptr()));
            if let Some(full) = full {
                assert!(!obj::allocation_is_live(full.as_ptr()));
                assert_eq!(unsafe { (*full.as_ptr()).ref_count }, 0);
            }
            assert_eq!(unsafe { (*first.as_ptr()).ref_count }, 1);
            assert_eq!(unsafe { (*later.as_ptr()).ref_count }, 1);
            assert!(interpreter
                .oo
                .borrow()
                .variable_mapping_retirements
                .is_empty());
            let first = interpreter.native_execution_refusal();
            if mode != 3 {
                assert!(first.is_some());
            }
            if mode == 1 || mode == 2 {
                assert_eq!(
                    obj::bytes_of(interpreter.get_obj_result()),
                    b"REACHED MAPPING RETIREMENT"
                );
            }
            reset();
        }
    }
}
#[test]
fn original_runtime_private_mapping_copy_keeps_both_stored_headers_and_original_storage_identity() {

    let mut interpreter = provided("tcl9.1", true);
    let original = obj::Owned::fresh(obj::new_string_bytes(b"p\0tail"));
    assert_eq!(publish(&mut interpreter, &[original.as_ptr()]), Code::Ok);
    let target = TARGET.with(Cell::get);
    let epoch = interpreter.oo.borrow().objects[&target].creation_id;
    let full = {
        let state = interpreter.oo.borrow();
        let declared = &state.classes[&target].private_variables[0];
        assert_eq!(declared.name, b"p\0tail");
        let full = declared.full_name.as_ref().unwrap();
        assert_eq!(full.name, format!("{epoch} : p").as_bytes());
        assert_eq!(unsafe { (*full.original.as_ptr()).ref_count }, 1);
        let observation = observation(declared);
        assert_eq!(
            observation.full_name.as_ref().unwrap().original.as_ptr(),
            full.original.as_ptr()
        );
        assert_eq!(unsafe { (*full.original.as_ptr()).ref_count }, 1);
        obj::NativeObjectLifetime::retain(full.original.as_ptr())
    };
    let copy = interpreter.oo.borrow().classes[&target].clone();
    assert_eq!(
        copy.private_variables[0]
            .full_name
            .as_ref()
            .unwrap()
            .original
            .as_ptr(),
        full.as_ptr()
    );
    assert_eq!(unsafe { (*full.as_ptr()).ref_count }, 2);
    assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 3);
    drop(copy);
    assert_eq!(unsafe { (*full.as_ptr()).ref_count }, 1);
    assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 2);
    assert_eq!(publish(&mut interpreter, &[]), Code::Ok);
    assert!(!obj::allocation_is_live(full.as_ptr()));
    assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
    reset();
}
#[test]
fn original_runtime_mapping_preserves_recursive_roster_until_outer_publication() {

    let mut interpreter = provided("tcl9.1", false);
    let (old, _) = install_old(&mut interpreter);
    let outer = operand(42);
    INCOMING.with(|incoming| *incoming.borrow_mut() = vec![outer.as_ptr()]);
    MODE.with(|mode| mode.set(4));
    assert_eq!(publish(&mut interpreter, &[outer.as_ptr()]), Code::Ok);
    assert!(!obj::allocation_is_live(old.as_ptr()));
    assert_eq!(FREES.with(Cell::get), 1);
    let state = interpreter.oo.borrow();
    let map = slot(&state, (TARGET.with(Cell::get), true, false)).unwrap();
    assert_eq!(map.len(), 1);
    assert_eq!(map[0].original.as_ptr(), outer.as_ptr());
    assert_eq!(unsafe { (*outer.as_ptr()).ref_count }, 2);
    assert!(state.variable_mapping_retirements.is_empty());
    drop(state);
    reset();
}

#[test]
fn original_runtime_private_link_uses_the_stored_full_name_header_for_its_actual_cache() {

    let mut interpreter = provided("tcl9.1", true);
    let original = obj::Owned::fresh(obj::new_string_bytes(b"p"));
    assert_eq!(publish(&mut interpreter, &[original.as_ptr()]), Code::Ok);
    let target = TARGET.with(Cell::get);
    let full = {
        let state = interpreter.oo.borrow();
        let full = state.classes[&target].private_variables[0]
            .full_name
            .as_ref()
            .unwrap();
        obj::NativeObjectLifetime::retain(full.original.as_ptr())
    };
    assert!(matches!(
        obj::native_object_snapshot(full.as_ptr()).unwrap().cache,
        tcl_syntax::native_object::NativeObjectCacheSnapshot::None
    ));
    assert_eq!(interpreter.eval_str(b"oo::define MappingClass method touch {} {my variable p; set p VALUE}; MappingClass create c; c touch"), Code::Ok);
    let state = interpreter.oo.borrow();
    assert_eq!(
        state.classes[&target].private_variables[0]
            .full_name
            .as_ref()
            .unwrap()
            .original
            .as_ptr(),
        full.as_ptr()
    );
    assert!(matches!(
        obj::native_object_snapshot(full.as_ptr()).unwrap().cache,
        tcl_syntax::native_object::NativeObjectCacheSnapshot::ParsedVariableName { .. }
    ));
    drop(state);
    assert_eq!(obj::bytes_of(interpreter.get_obj_result()), b"VALUE");
    reset();
}

#[test]
fn original_runtime_mapping_requires_current_definition_frame_and_allows_balanced_nested_frame() {

    // Real FrameStack births check software continuation, not Native frame chronology.
    for mode in [6, 7] {
        let mut interpreter = provided("tcl9.1", false);
        let baseline = interpreter.frames.borrow().current_level();
        let (old, _) = install_old(&mut interpreter);
        let incoming = operand(42);
        INCOMING.with(|values| *values.borrow_mut() = vec![incoming.as_ptr()]);
        MODE.with(|value| value.set(mode));
        let reached = publish(&mut interpreter, &[incoming.as_ptr()]);
        assert_eq!(interpreter.frames.borrow().current_level(), baseline);
        assert_eq!(FREES.with(Cell::get), 1);
        assert!(!obj::allocation_is_live(old.as_ptr()));
        assert!(interpreter
            .oo
            .borrow()
            .variable_mapping_retirements
            .is_empty());
        if mode == 6 {
            assert_eq!(reached, Code::Error);
            assert!(KEYS.with(|keys| keys.borrow().is_empty()));
            assert_eq!(unsafe { (*incoming.as_ptr()).ref_count }, 1);
            assert!(matches!(
                interpreter.native_execution_refusal(),
                Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(_))
            ));
        } else {
            assert_eq!(reached, Code::Ok);
            assert_eq!(KEYS.with(|keys| keys.borrow().clone()), [42]);
            assert_eq!(unsafe { (*incoming.as_ptr()).ref_count }, 2);
            assert!(interpreter.native_execution_refusal().is_none());
            assert_eq!(
                interpreter.oo.borrow().classes[&TARGET.with(Cell::get)].variables[0]
                    .original
                    .as_ptr(),
                incoming.as_ptr()
            );
        }
        reset();
    }
}
