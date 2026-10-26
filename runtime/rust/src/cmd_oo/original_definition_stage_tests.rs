// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original definition operands, target withdrawal and reached flag effects.
use super::*;
use crate::interp::WeakInterp;
use std::cell::Cell;

thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static CLASS: Cell<bool> = const { Cell::new(false) };
    static TARGET: Cell<OoId> = const { Cell::new(OoId(0)) };
    static UPDATES: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
    static FLAG_OPERATION: Cell<bool> = const { Cell::new(true) };
    static FLAG_OBSERVATION: Cell<Option<bool>> = const { Cell::new(None) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static REACHED: RefCell<Option<obj::Owned>> = const { RefCell::new(None) };
}
fn effect(interpreter: &mut Interp) {
    interpreter.set_result_bytes(b"REACHED DEFINITION OPERAND");
    REACHED.with(|result| {
        *result.borrow_mut() = Some(obj::Owned::retain(interpreter.get_obj_result()))
    });
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("first original definition operand");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("definition-operand".into(), "1.0".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        3 => {
            if CLASS.with(Cell::get) {
                interpreter.oo_destroy_class(TARGET.with(Cell::get));
            } else {
                assert_eq!(interpreter.oo_destroy(TARGET.with(Cell::get)), Code::Ok);
            }
        }
        4 => assert_eq!(
            interpreter.eval_str(b"oo::define DefinitionClass {method extra {} {return EXTRA}}"),
            Code::Ok
        ),
        _ => {}
    }
}
fn owner() -> Interp {
    OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap())
}
extern "C" fn update(original: *mut TclObj) {
    let role = obj::internal_rep(original);
    UPDATES.with(|updates| updates.borrow_mut().push(role));
    let mut interpreter = owner();
    assert!(interpreter.oo.try_borrow_mut().is_ok());
    assert!(interpreter.frames.try_borrow_mut().is_ok());
    if role == 5 {
        FLAG_OBSERVATION.with(|seen| {
            seen.set(Some(visible(
                &interpreter,
                b"first",
                FLAG_OPERATION.with(Cell::get),
            )))
        });
    }
    effect(&mut interpreter);
    let bytes = match role {
        1 => b"::oo::class".as_slice(),
        2 => b"-instance",
        3 => b"::",
        4 => b"first",
        5 => b"second",
        6 => b"later",
        _ => panic!("fixed original operand role"),
    };
    unsafe { obj::set_native_updater_string_rep(original, bytes, false) };
}
extern "C" fn retire(_original: *mut TclObj) {
    FREES.with(|frees| frees.set(frees.get() + 1));
    let mut interpreter = owner();
    assert!(interpreter.oo.try_borrow_mut().is_ok());
    effect(&mut interpreter);
}
static ARGUMENT: obj::TclObjType = obj::TclObjType {
    name: c"originalDefinitionOperand".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
static BODY: obj::TclObjType = obj::TclObjType {
    name: c"originalDefinitionRetiredBody".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn word(bytes: &[u8]) -> obj::Owned {
    obj::Owned::fresh(obj::new_string_bytes(bytes))
}
fn argument(role: u64) -> obj::Owned {
    obj::Owned::fresh(obj::alloc_typed(&ARGUMENT, role))
}
fn native(version: &str, class: bool) -> Interp {
    let mut interpreter = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(version),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(
        interpreter
            .eval_str(b"oo::class create DefinitionClass; DefinitionClass create DefinitionObject"),
        Code::Ok
    );
    let target = interpreter.oo_resolve_object(if class {
        b"DefinitionClass"
    } else {
        b"DefinitionObject"
    });
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|mode| mode.set(0));
    CLASS.with(|selected| selected.set(class));
    TARGET.with(|selected| selected.set(target));
    UPDATES.with(|updates| updates.borrow_mut().clear());
    FREES.with(|frees| frees.set(0));
    FLAG_OPERATION.with(|selected| selected.set(true));
    FLAG_OBSERVATION.with(|seen| seen.set(None));
    REACHED.with(|result| *result.borrow_mut() = None);
    interpreter
}
fn enter(interpreter: &mut Interp) -> NsId {
    let target = if CLASS.with(Cell::get) {
        DefTarget::Class(TARGET.with(Cell::get))
    } else {
        DefTarget::Object(TARGET.with(Cell::get))
    };
    let tape = [word(b"oo::define"), word(b"DefinitionClass"), word(b"")];
    let argv: Vec<_> = tape.iter().map(obj::Owned::as_ptr).collect();
    interpreter.oo_enter_definition(&target, &argv).unwrap()
}
fn finish(interpreter: &mut Interp, caller: NsId) {
    interpreter.oo_leave_definition(caller);
    assert_eq!(interpreter.entered_native_operation_count(), 0);
    OWNER.with(|owner| *owner.borrow_mut() = None);
    REACHED.with(|result| *result.borrow_mut() = None);
}
fn assert_first_refusal(interpreter: &Interp, mode: u8) {
    assert!(interpreter.native_execution_refusal().is_some());
    if mode == 1 {
        assert!(matches!(interpreter.native_execution_refusal(),
        Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first))
            if first.reason == "first original definition operand"));
    }
    REACHED.with(|result| {
        assert_eq!(
            interpreter.get_obj_result(),
            result.borrow().as_ref().unwrap().as_ptr()
        )
    });
}
fn assert_withdrawn(interpreter: &Interp) {
    let state = interpreter.oo.borrow();
    assert!(!state.objects.contains_key(&TARGET.with(Cell::get)));
    assert!(!state.classes.contains_key(&TARGET.with(Cell::get)));
    drop(state);
    assert!(interpreter.native_execution_refusal().is_none());
    assert_eq!(
        interpreter.snapshot_error().code_bytes(),
        b"TCL OO MONKEY_BUSINESS"
    );
    assert!(interpreter
        .result_bytes()
        .windows(b"deleted".len())
        .any(|part| part == b"deleted"));
}
fn visible(interpreter: &Interp, name: &[u8], export: bool) -> bool {
    let state = interpreter.oo.borrow();
    if CLASS.with(Cell::get) {
        let owner = &state.classes[&TARGET.with(Cell::get)];
        if export {
            owner.exported.contains(name)
        } else {
            owner.unexported.contains(name)
        }
    } else {
        let owner = &state.objects[&TARGET.with(Cell::get)];
        if export {
            owner.exported.contains(name)
        } else {
            owner.unexported.contains(name)
        }
    }
}

#[test]
fn class_operand_withdrawal_cannot_recreate_a_class_facet_on_the_original_object() {

    // Software target/currency controls; C9.1 source supplies the context/getter/effect join.
    for mode in 0..4 {
        let mut interpreter = native("tcl9.1", false);
        let caller = enter(&mut interpreter);
        let head = word(b"class");
        let class = argument(1);
        MODE.with(|selected| selected.set(mode));
        assert_eq!(
            def_class(&mut interpreter, &[head.as_ptr(), class.as_ptr()]),
            if mode == 0 { Code::Ok } else { Code::Error }
        );
        assert_eq!(UPDATES.with(|updates| updates.borrow().clone()), [1]);
        assert!(obj::has_string_rep(class.as_ptr()));
        assert_eq!(unsafe { (*class.as_ptr()).ref_count }, 1);
        match mode {
            0 => {
                assert!(interpreter
                    .oo
                    .borrow()
                    .classes
                    .contains_key(&TARGET.with(Cell::get)));
                assert_eq!(interpreter.result_bytes(), b"");
            }
            3 => assert_withdrawn(&interpreter),
            _ => assert_first_refusal(&interpreter, mode),
        }
        finish(&mut interpreter, caller);
    }
}

#[test]
fn definition_namespace_checks_original_target_after_kind_and_namespace_getters() {

    for stage in [2, 3] {
        for mode in 0..5 {
            let mut interpreter = native("tcl9.1", true);
            let caller = enter(&mut interpreter);
            let head = word(b"definitionnamespace");
            let kind = if stage == 2 {
                argument(2)
            } else {
                word(b"-instance")
            };
            let namespace = if stage == 3 { argument(3) } else { argument(6) };
            if stage == 2 && (mode == 0 || mode == 4) {
                unsafe { obj::set_native_updater_string_rep(namespace.as_ptr(), b"::", false) };
            }
            MODE.with(|selected| selected.set(mode));
            assert_eq!(
                def_definitionnamespace(
                    &mut interpreter,
                    &[head.as_ptr(), kind.as_ptr(), namespace.as_ptr()]
                ),
                if mode == 0 || mode == 4 {
                    Code::Ok
                } else {
                    Code::Error
                }
            );
            assert_eq!(UPDATES.with(|updates| updates.borrow().clone()), [stage]);
            assert_eq!(unsafe { (*kind.as_ptr()).ref_count }, 1);
            assert_eq!(unsafe { (*namespace.as_ptr()).ref_count }, 1);
            match mode {
                0 | 4 => {
                    assert_eq!(
                        interpreter.oo.borrow().classes[&TARGET.with(Cell::get)]
                            .def_ns
                            .as_deref(),
                        Some(b"::".as_slice())
                    );
                    assert_eq!(interpreter.result_bytes(), b"");
                    assert!(interpreter.native_execution_refusal().is_none());
                }
                3 => assert_withdrawn(&interpreter),
                _ => assert_first_refusal(&interpreter, mode),
            }
            if stage == 2 && mode != 0 && mode != 4 {
                assert!(!obj::has_string_rep(namespace.as_ptr()));
            }
            finish(&mut interpreter, caller);
        }
    }
}

#[test]
fn method_deletion_distinguishes_withdrawn_target_before_later_lookup_or_retirement() {

    for class in [false, true] {
        for after_retirement in [false, true] {
            let mut interpreter = native("tcl9.1", class);
            let caller = enter(&mut interpreter);
            if after_retirement {
                let body = obj::Owned::fresh(obj::alloc_typed(&BODY, 17));
                unsafe { obj::set_native_updater_string_rep(body.as_ptr(), b"return", false) };
                let method = native_method_body(&interpreter, Vec::new(), body, None);
                let retired = {
                    let mut state = interpreter.oo.borrow_mut();
                    if class {
                        state
                            .classes
                            .get_mut(&TARGET.with(Cell::get))
                            .unwrap()
                            .methods
                            .insert_generated(b"first".to_vec(), method)
                    } else {
                        state
                            .objects
                            .get_mut(&TARGET.with(Cell::get))
                            .unwrap()
                            .methods
                            .insert_generated(b"first".to_vec(), method)
                    }
                };
                assert!(retired.is_none());
            }
            let head = word(b"deletemethod");
            let name = if after_retirement {
                word(b"first")
            } else {
                argument(4)
            };
            let later = argument(6);
            MODE.with(|selected| selected.set(3));
            assert_eq!(
                def_deletemethod(
                    &mut interpreter,
                    &[head.as_ptr(), name.as_ptr(), later.as_ptr()]
                ),
                Code::Error
            );
            assert_withdrawn(&interpreter);
            assert_eq!(FREES.with(Cell::get), usize::from(after_retirement));
            assert_eq!(
                UPDATES.with(|updates| updates.borrow().clone()),
                if after_retirement { vec![] } else { vec![4] }
            );
            assert!(!obj::has_string_rep(later.as_ptr()));
            assert_eq!(unsafe { (*later.as_ptr()).ref_count }, 1);
            finish(&mut interpreter, caller);
        }
    }
}

#[test]
fn selected_c91_visibility_keeps_reached_first_flag_when_a_later_operand_refuses() {

    // C9.1 source orders each hash getter and flag effect before the next operand.
    // Host/ABA outcomes are software controls, not measured native callback chronology.
    for class in [false, true] {
        for export in [false, true] {
            for mode in [1, 2] {
                let mut interpreter = native("tcl9.1", class);
                let caller = enter(&mut interpreter);
                let head = word(if export { b"export" } else { b"unexport" });
                let first = word(b"first");
                assert_eq!(
                    def_export(&mut interpreter, &[head.as_ptr(), first.as_ptr()], !export),
                    Code::Ok
                );
                FLAG_OPERATION.with(|selected| selected.set(export));
                let second = argument(5);
                let later = argument(6);
                MODE.with(|selected| selected.set(mode));
                assert_eq!(
                    def_export(
                        &mut interpreter,
                        &[
                            head.as_ptr(),
                            first.as_ptr(),
                            second.as_ptr(),
                            later.as_ptr()
                        ],
                        export
                    ),
                    Code::Error
                );
                assert_eq!(FLAG_OBSERVATION.with(Cell::get), Some(true));
                assert!(visible(&interpreter, b"first", export));
                assert!(!visible(&interpreter, b"second", export));
                assert!(!visible(&interpreter, b"later", export));
                assert_eq!(UPDATES.with(|updates| updates.borrow().clone()), [5]);
                assert!(obj::has_string_rep(second.as_ptr()));
                assert!(!obj::has_string_rep(later.as_ptr()));
                assert_eq!(unsafe { (*second.as_ptr()).ref_count }, 1);
                assert_eq!(unsafe { (*later.as_ptr()).ref_count }, 1);
                assert_first_refusal(&interpreter, mode);
                finish(&mut interpreter, caller);
            }
        }
    }
}

#[test]
fn visibility_compatibility_and_ordinary_metadata_changes_keep_their_selected_stages() {

    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 4] {
            let mut interpreter = native(version, true);
            let caller = enter(&mut interpreter);
            let head = word(b"export");
            let first = word(b"first");
            let second = argument(5);
            MODE.with(|selected| selected.set(mode));
            assert_eq!(
                def_export(
                    &mut interpreter,
                    &[head.as_ptr(), first.as_ptr(), second.as_ptr()],
                    true
                ),
                if mode == 1 { Code::Error } else { Code::Ok }
            );
            assert_eq!(
                visible(&interpreter, b"first", true),
                mode != 1 || version == "tcl9.1"
            );
            assert_eq!(visible(&interpreter, b"second", true), mode != 1);
            assert_eq!(FLAG_OBSERVATION.with(Cell::get), Some(version == "tcl9.1"));
            assert_eq!(UPDATES.with(|updates| updates.borrow().clone()), [5]);
            if mode == 1 {
                assert_first_refusal(&interpreter, mode);
            } else {
                assert_eq!(interpreter.result_bytes(), b"");
                assert!(interpreter.native_execution_refusal().is_none());
            }
            finish(&mut interpreter, caller);
        }
    }
}
