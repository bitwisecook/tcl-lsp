// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected original class-variable getter, preflight and link stages.
use super::*;
use crate::interp::WeakInterp;
use std::cell::Cell;

thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static WORDS: RefCell<Vec<obj::Owned>> = const { RefCell::new(Vec::new()) };
    static UPDATES: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static BAD_NAME: Cell<bool> = const { Cell::new(false) };
    static CLASS: Cell<OoId> = const { Cell::new(OoId(0)) };
    static REACHED: RefCell<Option<obj::Owned>> = const { RefCell::new(None) };
    static WORKER_RESULT: Cell<*mut TclObj> = const { Cell::new(std::ptr::null_mut()) };
    static WORKER_CODE: Cell<Code> = const { Cell::new(Code::Ok) };
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
    if role == 1 {
        match MODE.with(Cell::get) {
            1 => {
                interpreter.refuse_host_command("first original classvariable name");
            }
            2 => {
                let context = interpreter.runtime_context();
                let mut changed = context.clone();
                changed
                    .packages
                    .push(("classvariable-name".into(), "1.0".into()));
                interpreter.pin_context(&changed).unwrap();
                interpreter.pin_context(&context).unwrap();
            }
            3 => {
                let target = CLASS.with(Cell::get);
                let creation = interpreter.oo.borrow().objects[&target].creation_id;
                assert_eq!(
                    interpreter.eval_str(b"oo::objdefine ClassVariableClass {class ::oo::object}"),
                    Code::Ok
                );
                assert_eq!(
                    interpreter.eval_str(b"oo::objdefine ClassVariableClass {class ::oo::class}"),
                    Code::Ok
                );
                assert_eq!(
                    interpreter.oo.borrow().objects[&target].creation_id,
                    creation
                );
            }
            4 => assert_eq!(
                interpreter
                    .eval_str(b"oo::define ClassVariableClass {method extra {} {return EXTRA}}"),
                Code::Ok
            ),
            _ => {}
        }
        interpreter.set_result_bytes(b"REACHED CLASSVARIABLE NAME");
        REACHED.with(|reached| {
            *reached.borrow_mut() = Some(obj::Owned::retain(interpreter.get_obj_result()))
        });
    }
    let bytes = match role {
        0 => b"classvariable".as_slice(),
        1 => b"first",
        2 if BAD_NAME.with(Cell::get) => b"bad::second",
        2 => b"second",
        3 => b"later",
        _ => panic!("fixed classvariable original role"),
    };
    unsafe { obj::set_native_updater_string_rep(original, bytes, false) };
}
static ARGUMENT: obj::TclObjType = obj::TclObjType {
    name: c"originalClassVariableName".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
fn word(bytes: &[u8]) -> obj::Owned {
    obj::Owned::fresh(obj::new_string_bytes(bytes))
}
fn argument(role: u64) -> obj::Owned {
    obj::Owned::fresh(obj::alloc_typed(&ARGUMENT, role))
}
fn worker(interpreter: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    let words = WORDS.with(|words| {
        words
            .borrow()
            .iter()
            .map(obj::Owned::as_ptr)
            .collect::<Vec<_>>()
    });
    interpreter.set_result_bytes(b"BEFORE CLASSVARIABLE WORKER");
    let code = classvariable_cmd(interpreter, &words);
    WORKER_RESULT.with(|result| result.set(interpreter.get_obj_result()));
    WORKER_CODE.with(|result| result.set(code));
    code
}
fn native(version: &str, lazy_head: bool) -> Interp {
    let mut interpreter = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(version),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    interpreter.register_builtin(b"classvariableOriginalWorker", worker);
    assert_eq!(interpreter.eval_str(
        b"oo::class create ClassVariableClass {method probe {} {classvariableOriginalWorker}}; ClassVariableClass create original"), Code::Ok);
    CLASS.with(|class| class.set(interpreter.oo_resolve_object(b"ClassVariableClass")));
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|mode| mode.set(0));
    BAD_NAME.with(|bad| bad.set(false));
    UPDATES.with(|updates| updates.borrow_mut().clear());
    REACHED.with(|reached| *reached.borrow_mut() = None);
    WORDS.with(|words| {
        *words.borrow_mut() = vec![
            if lazy_head {
                argument(0)
            } else {
                word(b"classvariable")
            },
            argument(1),
            argument(2),
            argument(3),
        ]
    });
    interpreter
}
fn class_namespace(interpreter: &Interp) -> NsId {
    interpreter.oo.borrow().objects[&CLASS.with(Cell::get)].var_ns
}
fn finish(interpreter: &Interp) {
    assert_eq!(interpreter.entered_native_operation_count(), 0);
    assert!(interpreter.oo.borrow().call_stack.is_empty());
    assert_eq!(interpreter.frames.borrow().current_level(), 0);
    WORDS.with(|words| words.borrow_mut().clear());
    REACHED.with(|reached| *reached.borrow_mut() = None);
    OWNER.with(|owner| *owner.borrow_mut() = None);
}

#[test]
fn selected_classvariable_validates_all_names_before_any_lookup_or_link() {

    // Actual selected worker in a genuine method frame; software stage control.
    for version in ["tcl9.0", "tcl9.1"] {
        let mut interpreter = native(version, true);
        let namespace = class_namespace(&interpreter);
        BAD_NAME.with(|bad| bad.set(true));
        assert_eq!(interpreter.eval_str(b"original probe"), Code::Error);
        assert_eq!(
            UPDATES.with(|updates| updates.borrow().clone()),
            if version == "tcl9.1" {
                vec![0, 1, 2]
            } else {
                vec![1, 2]
            }
        );
        assert!(!interpreter
            .namespaces()
            .var_table(namespace)
            .has_native_namespace_cell(b"first"));
        assert_eq!(
            interpreter.snapshot_error().code_bytes(),
            b"TCL UPVAR LOCAL_ELEMENT"
        );
        assert!(interpreter.native_execution_refusal().is_none());
        WORDS.with(|words| {
            let words = words.borrow();
            assert!(!obj::has_string_rep(words[3].as_ptr()));
            assert_eq!(
                obj::obj_type_ptr(words[1].as_ptr()),
                std::ptr::addr_of!(ARGUMENT)
            );
            for original in &words[1..] {
                assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
            }
        });
        finish(&interpreter);
    }
}

#[test]
fn selected_classvariable_name_callbacks_keep_original_frame_facet_and_first_cause() {

    for version in ["tcl9.0", "tcl9.1"] {
        for mode in 1..4 {
            let mut interpreter = native(version, false);
            let namespace = class_namespace(&interpreter);
            MODE.with(|selected| selected.set(mode));
            assert_eq!(interpreter.eval_str(b"original probe"), Code::Error);
            assert_eq!(UPDATES.with(|updates| updates.borrow().clone()), vec![1]);
            if mode == 1 {
                assert!(matches!(interpreter.native_execution_refusal(),
                    Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first))
                    if first.reason == "first original classvariable name"));
            } else {
                assert!(interpreter.native_execution_refusal().is_some());
            }
            assert!(!interpreter
                .namespaces()
                .var_table(namespace)
                .has_native_namespace_cell(b"first"));
            REACHED.with(|reached| {
                assert_eq!(
                    WORKER_RESULT.with(Cell::get),
                    reached.borrow().as_ref().unwrap().as_ptr()
                );
            });
            WORDS.with(|words| {
                let words = words.borrow();
                assert!(!obj::has_string_rep(words[2].as_ptr()));
                assert!(!obj::has_string_rep(words[3].as_ptr()));
                assert!(obj::has_string_rep(words[1].as_ptr()));
                assert_eq!(unsafe { (*words[1].as_ptr()).ref_count }, 1);
            });
            finish(&interpreter);
        }
    }
}

#[test]
fn selected_classvariable_links_original_names_after_legal_method_metadata_changes() {

    for version in ["tcl9.0", "tcl9.1"] {
        let mut interpreter = native(version, false);
        let namespace = class_namespace(&interpreter);
        MODE.with(|selected| selected.set(4));
        assert_eq!(interpreter.eval_str(b"original probe"), Code::Ok);
        assert_eq!(WORKER_CODE.with(Cell::get), Code::Ok);
        assert_eq!(
            UPDATES.with(|updates| updates.borrow().clone()),
            vec![1, 2, 3]
        );
        for name in [b"first".as_slice(), b"second", b"later"] {
            assert!(interpreter
                .namespaces()
                .var_table(namespace)
                .has_native_namespace_cell(name));
        }
        REACHED.with(|reached| {
            assert_eq!(
                WORKER_RESULT.with(Cell::get),
                reached.borrow().as_ref().unwrap().as_ptr()
            );
        });
        WORDS.with(|words| {
            for (name, original) in [b"first".as_slice(), b"second", b"later"]
                .into_iter()
                .zip(&words.borrow()[1..])
            {
                assert!(interpreter
                    .namespaces()
                    .var_table(namespace)
                    .original_native_key_is(name, original.as_ptr()));
                // Caller and genuine class-namespace hash key; local frame is retired.
                assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 2);
            }
        });
        finish(&interpreter);
    }
}

#[test]
fn selected_classvariable_missing_context_reads_only_its_original_head() {

    for version in ["tcl9.0", "tcl9.1"] {
        let mut interpreter = native(version, true);
        let words = WORDS.with(|words| {
            words
                .borrow()
                .iter()
                .map(obj::Owned::as_ptr)
                .collect::<Vec<_>>()
        });
        assert_eq!(classvariable_cmd(&mut interpreter, &words), Code::Error);
        assert_eq!(UPDATES.with(|updates| updates.borrow().clone()), vec![0]);
        assert_eq!(
            interpreter.snapshot_error().code_bytes(),
            b"TCL OO CONTEXT_REQUIRED"
        );
        assert_eq!(
            interpreter.result_bytes(),
            b"classvariable may only be called from inside a method"
        );
        WORDS.with(|words| {
            for original in &words.borrow()[1..] {
                assert!(!obj::has_string_rep(original.as_ptr()));
                assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
            }
        });
        finish(&interpreter);
    }
}
