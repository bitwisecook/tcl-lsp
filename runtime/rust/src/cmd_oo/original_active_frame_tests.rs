// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual active frame owner retirement and full first-cause cleanup.
use super::*;
use crate::interp::WeakInterp;
use std::cell::Cell;

thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static OUTSIDE: Cell<bool> = const { Cell::new(false) };
    static ACTIVE: Cell<usize> = const { Cell::new(usize::MAX) };
    static FREE_COUNT: Cell<obj::TclSize> = const { Cell::new(-1) };
}
struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        OWNER.with(|owner| owner.borrow_mut().take());
    }
}
extern "C" fn retired(original: *mut TclObj) {
    FREES.with(|frees| frees.set(frees.get() + 1));
    FREE_COUNT.with(|count| count.set(unsafe { (*original).ref_count }));
    let Some(mut interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    else {
        return;
    };
    let outside = interpreter.oo.try_borrow_mut().is_ok();
    OUTSIDE.with(|observed| observed.set(outside));
    if outside {
        ACTIVE.with(|active| active.set(interpreter.oo.borrow().call_stack.len()));
    }
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("first active OO frame retirement");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("active-oo-frame".into(), "1.0".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        _ => {}
    }
}
static NAME: obj::TclObjType = obj::TclObjType {
    name: c"originalActiveOoFrameName".as_ptr(),
    free_int_rep_proc: Some(retired),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn word(bytes: &[u8]) -> obj::Owned {
    obj::Owned::fresh(obj::new_string_bytes(bytes))
}
fn owning_name(bytes: &[u8]) -> obj::Owned {
    let original = obj::Owned::fresh(obj::alloc_typed(&NAME, 0));
    unsafe { obj::set_string_rep(original.as_ptr(), bytes) };
    original
}
fn native(version: &str) -> (Interp, Reset) {
    let mut interpreter = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(version),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(interpreter.eval_str(b"oo::class create FrameBase; oo::class create FrameDerived {superclass FrameBase}; FrameDerived create FrameObject"), Code::Ok);
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|mode| mode.set(0));
    FREES.with(|frees| frees.set(0));
    OUTSIDE.with(|observed| observed.set(false));
    ACTIVE.with(|active| active.set(usize::MAX));
    FREE_COUNT.with(|count| count.set(-1));
    (interpreter, Reset)
}
fn install_original_base(interpreter: &mut Interp) {
    let original = owning_name(b"m");
    let words: Vec<_> = [
        b"oo::define".as_slice(),
        b"FrameBase",
        b"method",
        b"",
        b"return UNUSED",
    ]
    .into_iter()
    .map(word)
    .collect();
    let argv = [
        words[0].as_ptr(),
        words[1].as_ptr(),
        words[2].as_ptr(),
        original.as_ptr(),
        words[3].as_ptr(),
        words[4].as_ptr(),
    ];
    assert_eq!(interpreter.dispatch(&argv), Code::Ok);
    assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 3);
    drop(original);
    assert_eq!(FREES.with(Cell::get), 0);
}
fn delete_base(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
    let code = interpreter.eval_str(b"oo::define FrameBase deletemethod m");
    if code == Code::Ok {
        interpreter.set_result_bytes(b"ACTIVE");
    }
    code
}
fn current_and_clean(interpreter: &Interp, namespace: NsId, activation: u64) {
    assert!(OUTSIDE.with(Cell::get));
    assert_eq!(FREES.with(Cell::get), 1);
    assert_eq!(FREE_COUNT.with(Cell::get), 0);
    assert_eq!(ACTIVE.with(Cell::get), 0);
    assert!(interpreter.oo.borrow().call_stack.is_empty());
    assert!(!interpreter.oo.borrow().filter_handling);
    assert!(interpreter.oo.borrow().fwd_usage.is_none());
    assert!(interpreter.ensemble_rewrite().is_none());
    assert_eq!(interpreter.current_ns(), namespace);
    assert_eq!(interpreter.frames.borrow().current_activation(), activation);
    assert_eq!(interpreter.entered_native_operation_count(), 0);
}
fn first(interpreter: &Interp, mode: u8) {
    match mode {
        0 => assert!(interpreter.native_execution_refusal().is_none()),
        1 => assert!(
            matches!(interpreter.native_execution_refusal(), Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first)) if first.reason == "first active OO frame retirement")
        ),
        2 => assert!(matches!(
            interpreter.native_execution_refusal(),
            Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "stale entered native operation"
                )
            ))
        )),
        _ => unreachable!(),
    }
}
#[test]
fn active_original_method_frames_retire_all_reached_records_outside_oo_borrow() {

    // Real authored record/name roles; the unused ancestor is held by the entered chain alone.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for forward in [false, true] {
            for mode in 0..3 {
                crate::counters::reset();
                {
                    let (mut interpreter, _reset) = native(version);
                    install_original_base(&mut interpreter);
                    if forward {
                        interpreter.register_builtin(b"frameDeleteBase", delete_base);
                        assert_eq!(
                            interpreter
                                .eval_str(b"oo::define FrameDerived forward m frameDeleteBase"),
                            Code::Ok
                        );
                    } else {
                        assert_eq!(interpreter.eval_str(b"oo::define FrameDerived method m {} {oo::define FrameBase deletemethod m; return ACTIVE}"), Code::Ok);
                    }
                    let namespace = interpreter.current_ns();
                    let activation = interpreter.frames.borrow().current_activation();
                    MODE.with(|slot| slot.set(mode));
                    assert_eq!(
                        interpreter.eval_str(b"FrameObject m"),
                        if mode == 0 { Code::Ok } else { Code::Error },
                        "{version}/{forward}/{mode}"
                    );
                    current_and_clean(&interpreter, namespace, activation);
                    first(&interpreter, mode);
                    if mode == 0 {
                        assert_eq!(interpreter.result_bytes(), b"ACTIVE");
                    }
                }
                assert_eq!(crate::counters::finalize(), 0, "{version}/{forward}/{mode}");
                assert_eq!(crate::counters::double_free_count(), 0);
            }
        }
    }
}
#[test]
fn active_eval_name_retirement_preserves_mandatory_frame_and_namespace_restoration() {

    // A private real cached-name role exercises the genuine eval frame's final owner.
    for mode in 0..3 {
        let (mut interpreter, _reset) = native("tcl8.6");
        let target = interpreter.oo_resolve_object(b"FrameObject");
        let retired = {
            let owner = interpreter.oo.borrow().objects[&target].cached_name.clone();
            let retired = owner.borrow_mut().replace(owning_name(b"::FrameObject"));
            retired
        };
        drop(retired);
        let namespace = interpreter.current_ns();
        let activation = interpreter.frames.borrow().current_activation();
        let script = word(b"rename FrameObject {}; set frameEval ACTIVE");
        MODE.with(|slot| slot.set(mode));
        let code = interpreter
            .oo_builtin_method(target, b"eval", &[script.as_ptr()], true, None)
            .unwrap();
        assert_eq!(code, if mode == 0 { Code::Ok } else { Code::Error });
        current_and_clean(&interpreter, namespace, activation);
        first(&interpreter, mode);
        assert!(!interpreter.oo.borrow().objects.contains_key(&target));
    }
}
#[test]
fn method_entry_failure_latches_original_cause_before_incoming_chain_retirement() {

    let (mut interpreter, _reset) = native("tcl8.6");
    install_original_base(&mut interpreter);
    let target = interpreter.oo_resolve_object(b"FrameObject");
    let provider = interpreter.oo_resolve_object(b"FrameBase");
    let step = CallStep {
        method_owner: None,
        provider,
        method: b"m".to_vec(),
        is_object: false,
        is_filter: false,
        synthetic_type: None,
        declaring_facet: None,
    };
    let chain = native_method_cache::retain_owners(
        &interpreter.oo.borrow(),
        vec![step],
        NativeTclooMethodReportRole::Named,
    );
    assert_eq!(
        interpreter.eval_str(b"oo::define FrameBase deletemethod m"),
        Code::Ok
    );
    assert_eq!(FREES.with(Cell::get), 0);
    interpreter.exhaust_native_operation_context_for_test();
    assert!(interpreter.native_execution_refusal().is_none());
    MODE.with(|slot| slot.set(1));
    assert_eq!(
        interpreter.oo_run(target, chain, 0, b"m", &[], MethodInvocation::internal()),
        Code::Error
    );
    assert!(matches!(
        interpreter.native_execution_refusal(),
        Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "stale entered native operation"
            )
        ))
    ));
    assert_eq!(FREES.with(Cell::get), 1);
    assert!(OUTSIDE.with(Cell::get));
    assert!(interpreter.oo.borrow().call_stack.is_empty());
    assert_eq!(interpreter.entered_native_operation_count(), 0);
}
