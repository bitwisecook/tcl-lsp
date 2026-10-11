// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original eval-object getters retain the entered context through restoration.

use super::{Code, Interp};
use crate::obj::{self, Owned, TclObj};
use std::{cell::Cell, rc::Rc};
use tcl_platform::{
    Capabilities, Clock, Env, Host, NativeIntegerFormatter, NumericEnvironment, StdIo,
};

thread_local! {
    static SCRIPT_GETTER_MODE: Cell<u8> = const { Cell::new(0) };
    static SCRIPT_GETTER_CALLS: Cell<usize> = const { Cell::new(0) };
    static SCRIPT_RESTORED: Cell<bool> = const { Cell::new(false) };
}

struct CurrentEvalInterp(*mut Interp);
impl CurrentEvalInterp {
    fn enter(interp: &mut Interp) -> Self {
        let previous = crate::codegen_abi::current_interp();
        crate::codegen_abi::tcl_runtime_set_current_interp(interp);
        Self(previous)
    }
}
impl Drop for CurrentEvalInterp {
    fn drop(&mut self) {
        crate::codegen_abi::tcl_runtime_set_current_interp(self.0);
    }
}

fn change_and_restore(interp: &mut Interp) {
    let original = interp.runtime_context();
    let mut changed = original.clone();
    changed.packages = vec![("original-eval-object".to_owned(), "1.0".to_owned())];
    interp.pin_context(&changed).unwrap();
    interp.pin_context(&original).unwrap();
}

struct EvalHost {
    actual: Rc<dyn Host>,
    armed: Cell<bool>,
    mode: Cell<u8>,
    formatter_calls: Cell<usize>,
}
impl Host for EvalHost {
    fn capabilities(&self) -> Capabilities {
        self.actual.capabilities()
    }
    fn clock(&self) -> &dyn Clock {
        self.actual.clock()
    }
    fn stdio(&self) -> &dyn StdIo {
        self.actual.stdio()
    }
    fn env(&self) -> &dyn Env {
        self.actual.env()
    }
    fn numeric_environment(&self) -> Option<&dyn NumericEnvironment> {
        self.actual.numeric_environment()
    }
    fn native_integer_formatter(&self) -> Option<&dyn NativeIntegerFormatter> {
        if self.armed.get() {
            self.formatter_calls.set(self.formatter_calls.get() + 1);
            let mut interp = unsafe { crate::codegen_abi::current_interp().as_ref() }
                .unwrap()
                .clone();
            match self.mode.replace(0) {
                1 => {
                    interp.refuse_host_command("first original eval formatter refusal");
                }
                2 => change_and_restore(&mut interp),
                _ => {}
            }
        }
        self.actual.native_integer_formatter()
    }
}

extern "C" fn script_updater(original: *mut TclObj) {
    SCRIPT_GETTER_CALLS.with(|calls| calls.set(calls.get() + 1));
    let mut interp = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone();
    match SCRIPT_GETTER_MODE.with(Cell::get) {
        1 => {
            interp.refuse_host_command("first original eval updater refusal");
        }
        2 => change_and_restore(&mut interp),
        _ => {}
    }
    // SAFETY: the reached callback owns the same live original script header.
    unsafe { obj::set_native_updater_string_rep(original, b"set reachedBody 1", false) };
}
static SCRIPT_TYPE: obj::TclObjType = obj::TclObjType {
    name: c"originalEvalScript".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(script_updater),
    set_from_any_proc: None,
};

fn original_body_probe(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let before = (interp.current_level(), interp.current_ns.get());
    assert_eq!(before.0, 1);
    assert_ne!(before.1, crate::namespace::GLOBAL);
    let code = interp.eval_uplevel_obj(0, argv[1]);
    SCRIPT_RESTORED.with(|restored| {
        restored.set((interp.current_level(), interp.current_ns.get()) == before);
    });
    code
}

fn fixture() -> (Interp, Rc<EvalHost>) {
    let mut interp = Interp::with_native_core(
        super::default_host(),
        crate::environment::profile_for_dialect("tcl8.4"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    let host = Rc::new(EvalHost {
        actual: interp.host(),
        armed: Cell::new(false),
        mode: Cell::new(0),
        formatter_calls: Cell::new(0),
    });
    interp.set_host(host.clone());
    interp.register_builtin(b"originalBodyProbe", original_body_probe);
    assert_eq!(
        interp.eval_str(b"namespace eval ::evalOwner {proc p {body} {originalBodyProbe $body}}"),
        Code::Ok,
    );
    SCRIPT_GETTER_MODE.with(|mode| mode.set(0));
    SCRIPT_GETTER_CALLS.with(|calls| calls.set(0));
    SCRIPT_RESTORED.with(|restored| restored.set(false));
    (interp, host)
}

#[test]
fn original_eval_formatter_refuses_later_getter_and_restores_actual_frame() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual C84 eval-host String purpose; software Host stages, no native equivalence.
    for mode in 0..=2 {
        let (mut interp, host) = fixture();
        let original = Owned::fresh(obj::alloc_typed(&SCRIPT_TYPE, 0));
        let command = Owned::fresh(obj::new_string_bytes(b"::evalOwner::p"));
        let before = unsafe { (*original.as_ptr()).ref_count };
        host.mode.set(mode);
        host.armed.set(true);
        let _installed = CurrentEvalInterp::enter(&mut interp);
        let code = interp.dispatch(&[command.as_ptr(), original.as_ptr()]);
        host.armed.set(false);
        assert_eq!(host.formatter_calls.get(), 1);
        assert!(SCRIPT_RESTORED.with(Cell::get));
        assert_eq!(interp.current_level(), 0);
        assert_eq!(interp.current_ns.get(), crate::namespace::GLOBAL);
        assert_eq!(interp.entered_native_operation_count(), 0);
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, before);
        let reached = interp
            .namespaces
            .borrow()
            .var_table(crate::namespace::GLOBAL)
            .binding_id(b"reachedBody")
            .is_some();
        if mode == 0 {
            assert_eq!(code, Code::Ok);
            assert_eq!(SCRIPT_GETTER_CALLS.with(Cell::get), 1);
            assert!(reached);
            assert!(interp.native_execution_refusal().is_none());
        } else {
            assert_eq!(code, Code::Error);
            assert_eq!(SCRIPT_GETTER_CALLS.with(Cell::get), 0);
            assert!(!obj::has_string_rep(original.as_ptr()));
            assert!(!reached);
            assert!(interp.native_execution_refusal().is_some());
        }
    }
}

#[test]
fn original_eval_updater_keeps_reached_bytes_and_refuses_script_entry() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Reached updater effects remain; current entered context precedes every body command.
    for mode in 1..=2 {
        let (mut interp, host) = fixture();
        let original = Owned::fresh(obj::alloc_typed(&SCRIPT_TYPE, 0));
        let command = Owned::fresh(obj::new_string_bytes(b"::evalOwner::p"));
        SCRIPT_GETTER_MODE.with(|value| value.set(mode));
        host.armed.set(true);
        let _installed = CurrentEvalInterp::enter(&mut interp);
        assert_eq!(
            interp.dispatch(&[command.as_ptr(), original.as_ptr()]),
            Code::Error
        );
        host.armed.set(false);
        assert_eq!(host.formatter_calls.get(), 1);
        assert_eq!(SCRIPT_GETTER_CALLS.with(Cell::get), 1);
        assert_eq!(obj::bytes_of(original.as_ptr()), b"set reachedBody 1");
        assert!(SCRIPT_RESTORED.with(Cell::get));
        assert_eq!(interp.current_level(), 0);
        assert_eq!(interp.current_ns.get(), crate::namespace::GLOBAL);
        assert_eq!(interp.entered_native_operation_count(), 0);
        assert!(
            interp
                .namespaces
                .borrow()
                .var_table(crate::namespace::GLOBAL)
                .binding_id(b"reachedBody")
                .is_none()
        );
        let retained = interp.native_execution_refusal().unwrap();
        assert!(interp.eval_object_bytes(original.as_ptr()).is_err());
        assert_eq!(interp.native_execution_refusal(), Some(retained));
        assert_eq!(SCRIPT_GETTER_CALLS.with(Cell::get), 1);
    }
}
