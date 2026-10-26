// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use crate::codegen_abi::{self, TclCompletionAbi};
use std::cell::{Cell, RefCell};

const BODY: &[u8] = b"return [expr {$a + $b}]";

fn publish(
    interpreter: &mut Interp,
    parameters: &[u8],
    candidate_body: &[u8],
) -> Option<Rc<ProcDef>> {
    publish_original(interpreter, b"a b", BODY, parameters, candidate_body)
}

fn publish_original(
    interpreter: &mut Interp,
    original_parameters: &[u8],
    original_body: &[u8],
    parameters: &[u8],
    candidate_body: &[u8],
) -> Option<Rc<ProcDef>> {
    let words = [
        fixture::string(b"proc"),
        fixture::string(b"add"),
        fixture::string(original_parameters),
        fixture::string(original_body),
    ];
    let pointers = words.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
    let identity = tcl_registry::NativeLowering::Definition
        .definition_guard_identity(interpreter.native_invocation_dialect())
        .unwrap();
    let mut out = TclCompletionAbi {
        code: -1,
        result: core::ptr::null_mut(),
        options: core::ptr::null_mut(),
    };
    // SAFETY: all argv objects and counted immutable candidate ranges are live.
    let status = unsafe {
        codegen_abi::tcl_codegen_proc_define_guarded_i64_add_argv(
            pointers.as_ptr(),
            4,
            identity.value(),
            parameters.as_ptr(),
            i32::try_from(parameters.len()).unwrap(),
            candidate_body.as_ptr(),
            i32::try_from(candidate_body.len()).unwrap(),
            &mut out,
        )
    };
    assert_eq!(status, codegen_abi::TCL_INVOKE_ABI_OK);
    assert_eq!(out.code, 0);
    // SAFETY: the ABI returned these owned completion references exactly once.
    unsafe { codegen_abi::tcl_completion_release(&mut out) };
    interpreter.proc_def(b"add")
}

fn result(interpreter: &Interp) -> Vec<u8> {
    obj::bytes_of(interpreter.result_obj())
}

#[test]
fn guarded_future_body_preserves_real_publication_and_runs_primitive_add() {
    let mut interpreter = fixture::native("tcl9.0");
    let _current = fixture::install(&mut interpreter, 0);
    let declaration = publish(&mut interpreter, b"a b", BODY).unwrap();
    assert!(declaration.future_i64_add.get().is_some());
    assert!(declaration.native.is_none());
    assert_eq!(interpreter.eval_str(b"info args add"), Code::Ok);
    assert_eq!(result(&interpreter), b"a b");
    assert_eq!(interpreter.eval_str(b"info body add"), Code::Ok);
    assert_eq!(result(&interpreter), BODY);
    assert_eq!(
        interpreter.eval_str(b"set d 2; set e 3; add $d $e"),
        Code::Ok
    );
    assert_eq!(result(&interpreter), b"5");
    assert_eq!(interpreter.native_proc_dispatches(), 1);
    assert_eq!(interpreter.eval_str(b"list $d $e"), Code::Ok);
    assert_eq!(result(&interpreter), b"2 3");
    let artifact = cache(declaration.body.checked_ptr().unwrap()).unwrap();
    assert!(
        interpreter.capture_entered_i64_body(&artifact).is_none(),
        "source and declaration alone grant no entered frame"
    );
    assert_eq!(
        interpreter.eval_str(b"rename add moved; moved 4 5"),
        Code::Ok
    );
    assert_eq!(result(&interpreter), b"9");
    assert_eq!(interpreter.native_proc_dispatches(), 2);
    assert_eq!(
        interpreter.eval_str(b"rename proc original; original proc args {return REPLACED}"),
        Code::Ok
    );
    assert!(publish(&mut interpreter, b"a b", BODY).is_none());
    assert_eq!(interpreter.native_proc_dispatches(), 2);
}

#[test]
fn future_recipe_keeps_number_trace_and_replacement_fallbacks() {
    for (parameters, candidate, setup, invocation, expected) in [
        (
            &b"a b"[..],
            BODY,
            &b""[..],
            &b"add 9223372036854775807 1"[..],
            &b"9223372036854775808"[..],
        ),
        (b"a b", BODY, b"", b"add 2.5 1", b"3.5"),
        (b"x y", BODY, b"", b"add 2 3", b"5"),
        (
            b"a b",
            &b"return [expr {$a - $b}]"[..],
            b"",
            b"add 2 3",
            b"5",
        ),
        (
            b"a b",
            BODY,
            b"set observed 0; trace add execution add enter {apply {{args} {incr ::observed}}}",
            b"list [add 2 3] $observed",
            b"5 1",
        ),
        (
            b"a b",
            BODY,
            b"proc add {a b} {return REPLACED}",
            b"add 2 3",
            b"REPLACED",
        ),
    ] {
        let mut interpreter = fixture::native("tcl9.0");
        let _current = fixture::install(&mut interpreter, 0);
        publish(&mut interpreter, parameters, candidate).unwrap();
        assert_eq!(interpreter.eval_str(setup), Code::Ok);
        assert_eq!(interpreter.eval_str(invocation), Code::Ok);
        assert_eq!(result(&interpreter), expected);
        assert_eq!(interpreter.native_proc_dispatches(), 0);
    }
}

#[test]
fn shared_original_recompile_cannot_copy_the_future_recipe() {
    let mut interpreter = fixture::native("tcl8.5");
    let _current = fixture::install(&mut interpreter, 0);
    let original = publish(&mut interpreter, b"a b", BODY).unwrap();
    let execution =
        tcl_runtime_api::native_procedure_roles::NativeProcedureReference::acquire(&original)
            .unwrap();
    assert_eq!(interpreter.eval_str(b"add 2 3"), Code::Ok);
    assert_eq!(result(&interpreter), b"5");
    let current = interpreter.proc_def(b"add").unwrap();
    assert!(!Rc::ptr_eq(&current, &original));
    assert!(current.future_i64_add.get().is_none());
    assert_eq!(interpreter.native_proc_dispatches(), 0);
    drop(execution);
}

struct EpochHost {
    inner: Rc<dyn tcl_platform::Host>,
    target: RefCell<Option<Interp>>,
    fired: Cell<usize>,
    input: RefCell<Option<obj::NativeObjectLifetime>>,
    raw_before_host: Cell<Option<bool>>,
}
impl tcl_platform::Host for EpochHost {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        self.inner.capabilities()
    }
    fn clock(&self) -> &dyn tcl_platform::Clock {
        self.inner.clock()
    }
    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        self.inner.stdio()
    }
    fn env(&self) -> &dyn tcl_platform::Env {
        self.inner.env()
    }
    fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
        let target = self.target.borrow().as_ref().cloned();
        if let Some(mut target) = target {
            let selected = target.cmd_frames.try_borrow().ok().is_some_and(|frames| {
                frames
                    .last()
                    .is_some_and(|frame| frame.cmd == b"expr {$a + $b}")
            });
            if selected && self.fired.get() == 0 {
                self.raw_before_host.set(
                    self.input
                        .borrow()
                        .as_ref()
                        .map(|input| obj::obj_type_ptr(input.as_ptr()).is_null()),
                );
                self.fired.set(1);
                self.target.borrow_mut().take();
                target.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            }
        }
        self.inner.numeric_environment()
    }
    fn native_list_sorting_environment(
        &self,
    ) -> Option<&dyn tcl_platform::NativeListSortingEnvironment> {
        self.inner.native_list_sorting_environment()
    }
    fn native_integer_formatter(&self) -> Option<&dyn tcl_platform::NativeIntegerFormatter> {
        self.inner.native_integer_formatter()
    }
    fn system_encoding(&self) -> tcl_platform::SystemEncoding {
        self.inner.system_encoding()
    }
    fn encoding_catalogue(&self) -> Option<&dyn tcl_platform::EncodingCatalogue> {
        self.inner.encoding_catalogue()
    }
    fn filesystem(&self) -> Option<&dyn tcl_platform::Filesystem> {
        self.inner.filesystem()
    }
    fn sockets(&self) -> Option<&dyn tcl_platform::Sockets> {
        self.inner.sockets()
    }
    fn process(&self) -> Option<&dyn tcl_platform::Process> {
        self.inner.process()
    }
}

#[test]
fn reached_numeric_context_change_is_terminal_without_body_replay() {
    let host = Rc::new(EpochHost {
        inner: crate::interp::default_host(),
        target: RefCell::new(None),
        fired: Cell::new(0),
        input: RefCell::new(None),
        raw_before_host: Cell::new(None),
    });
    let mut interpreter = Interp::with_native_core(
        host.clone(),
        crate::environment::profile_for_dialect("tcl9.0"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    let _current = fixture::install(&mut interpreter, 0);
    publish(&mut interpreter, b"a b", BODY).unwrap();
    assert_eq!(
        interpreter.eval_str(b"set d 2; set e 3; add $d $e"),
        Code::Ok
    );
    assert_eq!(result(&interpreter), b"5");
    assert_eq!(interpreter.native_proc_dispatches(), 1);
    let left = obj::Owned::fresh(obj::new_string_bytes(b"2"));
    let right = obj::Owned::fresh(obj::new_string_bytes(b"3"));
    interpreter.var_set(b"d", left.as_ptr()).unwrap();
    interpreter.var_set(b"e", right.as_ptr()).unwrap();
    assert!(obj::obj_type_ptr(left.as_ptr()).is_null());
    host.input
        .replace(Some(obj::NativeObjectLifetime::retain(left.as_ptr())));
    host.target.replace(Some(interpreter.clone()));
    let code = interpreter.eval_str(b"add $d $e");
    host.target.borrow_mut().take();
    assert_eq!(host.fired.get(), 1);
    assert_eq!(host.raw_before_host.get(), Some(true));
    assert!(obj::obj_type_ptr(left.as_ptr()).is_null());
    host.input.borrow_mut().take();
    assert_eq!(code, Code::Error);
    assert!(interpreter.native_execution_refusal().is_some());
    assert_eq!(interpreter.native_proc_dispatches(), 1);
}

#[test]
// Matched C8.6.18/C9.0.4: proc add {b b} {return [expr {$b + $b}]} ; add 1 2 -> 2.
// Retained oracle receipt: 0bdff94191089813811e61b592742e455d432edbda734ed8a5a9d3e90919b271.
fn duplicate_original_formals_follow_original_first_slot_without_a_native_strategy() {
    let mut interpreter = fixture::native("tcl9.0");
    let _current = fixture::install(&mut interpreter, 0);
    let body = b"return [expr {$b + $b}]";
    let declaration = publish_original(&mut interpreter, b"b b", body, b"b b", body).unwrap();
    assert_eq!(declaration.params[0].name, declaration.params[1].name);
    assert_eq!(interpreter.eval_str(b"add 1 2"), Code::Ok);
    assert_eq!(result(&interpreter), b"2");
    assert_eq!(interpreter.native_proc_dispatches(), 0);
}
