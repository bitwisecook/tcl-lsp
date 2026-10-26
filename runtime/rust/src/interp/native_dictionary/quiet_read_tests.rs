// SPDX-License-Identifier: AGPL-3.0-or-later
//! Quiet Guest lookup and original first execution refusal remain separate.
use super::*;
use crate::obj::{self, Owned, TclObj};
use std::{cell::Cell, rc::Rc};
use tcl_runtime_api::native_variable_trace::{
    NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
};
thread_local! {
    static UPDATES: Cell<usize> = const { Cell::new(0) };
}
extern "C" fn key_string(value: *mut TclObj) {
    UPDATES.with(|value| value.set(value.get() + 1));
    unsafe { obj::set_native_updater_string_rep(value, b"missing", false) };
}
static KEY: obj::TclObjType = obj::TclObjType {
    name: c"originalQuietDictionaryKey".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(key_string),
    set_from_any_proc: None,
};
struct Read {
    changed: bool,
    calls: Rc<Cell<usize>>,
}
impl NativeVariableObserver<Interp> for Read {
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        interpreter: &mut Interp,
        access: NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        assert_eq!(access.operation, NativeVariableTraceOperation::Read);
        assert!(interpreter.frames.try_borrow_mut().is_ok());
        assert!(interpreter.namespaces.try_borrow_mut().is_ok());
        self.calls.set(self.calls.get() + 1);
        if self.changed {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("quiet-dictionary-read".into(), "1.0".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        } else {
            interpreter.refuse_host_command("original quiet dictionary read observer");
        }
        Ok(())
    }
}
#[test]
fn quiet_dictionary_initial_read_stops_first_refusal_before_key_getter_or_publication() {

    // Real software native read callbacks; the unchanged 15-window comparison
    // retains the independent C/Jim public missing-variable observations.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for changed in [false, true] {
            crate::counters::reset();
            {
                let mut interpreter = Interp::with_native_core(
                    super::super::default_host(),
                    crate::environment::profile_for_dialect(profile),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap();
                assert_eq!(
                    interpreter.eval_str(b"set quietDict {kept VALUE}"),
                    Code::Ok
                );
                let stored = interpreter.var_get(b"quietDict").unwrap();
                let name = Owned::fresh(obj::new_string_bytes(b"quietDict"));
                let command = Owned::fresh(obj::new_string_bytes(b"dict"));
                let sub = Owned::fresh(obj::new_string_bytes(b"unset"));
                let key = Owned::fresh(obj::alloc_typed(&KEY, 0));
                let calls = Rc::new(Cell::new(0));
                interpreter
                    .add_native_variable_observer(
                        name.as_ptr(),
                        &[NativeVariableTraceOperation::Read],
                        Rc::new(Read {
                            changed,
                            calls: calls.clone(),
                        }),
                    )
                    .unwrap();
                interpreter.set_result_bytes(b"QUIET_READ_PRIOR");
                let prior = interpreter.result_obj();
                UPDATES.with(|value| value.set(0));
                assert_eq!(
                    interpreter.dispatch(&[
                        command.as_ptr(),
                        sub.as_ptr(),
                        name.as_ptr(),
                        key.as_ptr(),
                    ]),
                    Code::Error
                );
                assert_eq!(calls.get(), 1);
                assert_eq!(UPDATES.with(Cell::get), 0);
                assert!(!obj::has_string_rep(key.as_ptr()));
                assert_eq!(unsafe { (*key.as_ptr()).ref_count }, 1);
                assert_eq!(interpreter.result_obj(), prior);
                assert_eq!(
                    crate::vars::get(
                        &interpreter.frames.borrow(),
                        &interpreter.namespaces(),
                        interpreter.current_ns(),
                        b"quietDict",
                    ),
                    Some(stored)
                );
                let first = interpreter.native_execution_refusal().unwrap();
                if changed {
                    assert_eq!(first, tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("stale entered native operation")
                    ));
                } else {
                    assert!(
                        matches!(first, tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first) if first.reason == "original quiet dictionary read observer")
                    );
                }
                assert_eq!(interpreter.entered_native_operation_count(), 0);
            }
            assert_eq!(crate::counters::finalize(), 0);
            assert_eq!(crate::counters::double_free_count(), 0);
        }
    }
}
