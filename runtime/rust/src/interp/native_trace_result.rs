// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected native ownership of the result across variable trace callbacks.

use super::Interp;
use crate::obj;

pub(super) struct NativeTraceResultState {
    currency: super::native_operation_currency::SavedNativeOperationCurrency,
    result: obj::Owned,
    interpreter: Option<NativeTraceInterpreterState>,
    reset_before_restore: bool,
    return_code: Option<super::Code>,
}

struct NativeTraceInterpreterState {
    error: super::native_error_variables::NativeErrorTraceState,
    code: super::Code,
    level: usize,
    options: super::native_error_headers::NativeReturnOptions,
    stack: Option<super::native_error_headers::NativeErrorStack>,
}

impl Interp {
    pub(super) fn save_native_variable_trace_result(
        &self,
        script: bool,
    ) -> Result<Option<NativeTraceResultState>, tcl_runtime_api::NativeExecutionError> {
        use tcl_registry::native_variable_trace::NativeTraceStatePurpose;
        self.save_native_trace_state(if script {
            NativeTraceStatePurpose::VariableScript
        } else {
            NativeTraceStatePurpose::VariableChain
        })
    }

    pub(super) fn save_native_command_trace_result(
        &self,
        script: bool,
    ) -> Result<Option<NativeTraceResultState>, tcl_runtime_api::NativeExecutionError> {
        use tcl_registry::native_variable_trace::NativeTraceStatePurpose;
        self.save_native_trace_state(if script {
            NativeTraceStatePurpose::CommandScript
        } else {
            NativeTraceStatePurpose::CommandChain
        })
    }

    fn save_native_trace_state(
        &self,
        purpose: tcl_registry::native_variable_trace::NativeTraceStatePurpose,
    ) -> Result<Option<NativeTraceResultState>, tcl_runtime_api::NativeExecutionError> {
        use tcl_registry::native_variable_trace::NativeTraceStateRecipe;
        let currency =
            super::native_operation_currency::SavedNativeOperationCurrency::capture(self);
        currency.check(self)?;
        let Some(recipe) = self
            .native_invocation_dialect()
            .native_trace_state_recipe(purpose)
        else {
            return Ok(None);
        };
        if recipe == NativeTraceStateRecipe::None {
            return Ok(None);
        }
        if matches!(
            recipe,
            NativeTraceStateRecipe::MoveResult | NativeTraceStateRecipe::MoveResultAndCode
        ) {
            let replacement = obj::Owned::fresh(obj::new_string_bytes(b""));
            let original = self.result.replace(replacement.into_raw());
            // SAFETY: SaveResult transfers the interpreter's existing +1.
            let result = unsafe { obj::Owned::from_raw(original) };
            return Ok(Some(NativeTraceResultState {
                currency,
                result,
                interpreter: None,
                reset_before_restore: true,
                return_code: (recipe == NativeTraceStateRecipe::MoveResultAndCode)
                    .then(|| self.return_code.get()),
            }));
        }
        Ok(Some(NativeTraceResultState {
            currency,
            result: obj::Owned::retain(self.result.get()),
            interpreter: Some(NativeTraceInterpreterState {
                error: self
                    .save_native_error_trace_state()
                    .expect("selected C interpreter error state"),
                code: self.return_code.get(),
                level: self.return_level.get(),
                options: self.return_options.borrow().clone(),
                stack: self
                    .runtime_version()
                    .has_error_stack()
                    .then(|| self.error_stack.borrow().clone()),
            }),
            reset_before_restore: false,
            return_code: None,
        }))
    }

    pub(super) fn restore_native_variable_trace_result(
        &self,
        saved: NativeTraceResultState,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let cleanup = super::native_operation_currency::NativeOperationScope::enter_saved_cleanup(
            &saved.currency,
            self,
        )?;
        if saved.reset_before_restore {
            let current = self.result.get();
            if obj::is_shared(current) {
                let empty = obj::Owned::fresh(obj::new_string_bytes(b""));
                let retired = self.result.replace(empty.into_raw());
                // SAFETY: the interpreter transfers its old owning reference.
                unsafe { obj::decr_ref_count(retired) };
            } else {
                obj::reset_native_c_result(current);
            }
            self.settle_native_global_error_episode();
        }
        let mut options_outcome = Ok(());
        if let Some(state) = saved.interpreter {
            self.return_code.set(state.code);
            self.return_level.set(state.level);
            let _ = self.restore_native_error_trace_state(state.error);
            if let Some(stack) = state.stack {
                let mut stack = stack;
                stack.renew_incarnation();
                let retired = self.error_stack.replace(stack);
                drop(retired);
            }
            options_outcome = self.restore_return_options_for_cleanup(state.options, &cleanup);
        }
        let retired = self.result.replace(saved.result.into_raw());
        // SAFETY: the interpreter transfers its current +1 to this retirement;
        // the saved +1 is already installed before any object release.
        unsafe { obj::decr_ref_count(retired) };
        if let Some(code) = saved.return_code {
            self.return_code.set(code);
        }

        cleanup.check()?;
        options_outcome
    }
}

#[cfg(test)]
mod tests {
    use super::Interp;
    use tcl_dialect::TclVersion;

    #[test]
    fn selected_callback_save_transfers_the_original_result_owner() {
        for release in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let mut interp = Interp::new();
            interp.set_runtime_version(release);
            interp.set_result_bytes(b"BEFORE\xff\0TAIL");
            let original = interp.result_obj();
            let script = release == TclVersion::V8_4;
            let absent = interp.save_native_variable_trace_result(!script).unwrap();
            assert!(absent.is_none());
            // SAFETY: the interpreter owns this live original result.
            assert_eq!(unsafe { (*original).ref_count }, 1);
            let saved = interp
                .save_native_variable_trace_result(script)
                .unwrap()
                .unwrap();
            // SAFETY: the selected state owns the live original. C84 moves
            // that reference; modern C also keeps the interpreter reference.
            assert_eq!(unsafe { (*original).ref_count }, if script { 1 } else { 2 });
            if script {
                assert_ne!(interp.result_obj(), original);
                assert_eq!(interp.result_bytes(), b"");
            }
            interp.set_result_bytes(b"CALLBACK");
            // SAFETY: the saved state still owns the retired result.
            assert_eq!(unsafe { (*original).ref_count }, 1);
            interp.restore_native_variable_trace_result(saved).unwrap();
            assert_eq!(interp.result_obj(), original);
            assert_eq!(interp.result_bytes(), b"BEFORE\xff\0TAIL");
            // SAFETY: restoration transfers the saved reference to the interpreter.
            assert_eq!(unsafe { (*original).ref_count }, 1);
        }
    }

    #[test]
    fn c84_script_purposes_preserve_different_return_state() {
        let mut interp = Interp::new();
        interp.set_runtime_version(TclVersion::V8_4);
        for command in [false, true] {
            interp.set_return_state(3, super::super::Code::from_int(7));
            interp.set_result_bytes(b"BEFORE");
            let original = interp.result_obj();
            let saved = if command {
                interp.save_native_command_trace_result(true)
            } else {
                interp.save_native_variable_trace_result(true)
            }
            .unwrap()
            .unwrap();
            // SAFETY: the selected saved state owns this original +1.
            assert_eq!(unsafe { (*original).ref_count }, 1);
            interp.set_return_state(0, super::super::Code::Error);
            interp.set_result_bytes(b"CALLBACK");
            interp.restore_native_variable_trace_result(saved).unwrap();
            assert_eq!(interp.result_obj(), original);
            assert_eq!(interp.pending_return_level(), 0);
            assert_eq!(
                interp.pending_return_code(),
                if command {
                    super::super::Code::from_int(7)
                } else {
                    super::super::Code::Error
                }
            );
        }
    }

    #[test]
    fn modern_state_restores_original_private_objects_and_shared_options() {
        use crate::obj;
        for release in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let mut interp = Interp::new();
            interp.set_runtime_version(release);
            interp.set_return_state(3, super::super::Code::from_int(7));
            let key = obj::Owned::fresh(obj::new_string_bytes(b"-custom\xff"));
            let value = obj::Owned::fresh(obj::new_string_bytes(b"VALUE\0TAIL"));
            let key_ptr = key.as_ptr();
            let value_ptr = value.as_ptr();
            interp.set_return_option_objects(vec![
                tcl_cmd_core::return_options::ReturnOptionPair {
                    key,
                    value,
                    key_bytes: b"-custom\xff".to_vec(),
                },
            ]);
            let info = obj::Owned::fresh(obj::new_string_bytes(b"ORIGINAL INFO"));
            let info_ptr = info.as_ptr();
            {
                let mut exc = interp.exc.borrow_mut();
                exc.native.info = Some(info);
                exc.native.info_len = 13;
                exc.already_logged = true;
            }
            if release.has_error_stack() {
                let mut stack = interp.error_stack.borrow_mut();
                stack.begin_inner(b"INNER".to_vec(), b"ORIGINAL".to_vec());
                stack.mark_reset();
            }
            let saved = interp
                .save_native_variable_trace_result(false)
                .unwrap()
                .unwrap();
            // SAFETY: shared option backing retains the originals without
            // acquiring extra references to its key or value children.
            assert_eq!(unsafe { (*key_ptr).ref_count }, 1);
            assert_eq!(unsafe { (*value_ptr).ref_count }, 1);
            assert_eq!(unsafe { (*info_ptr).ref_count }, 2);
            interp.set_return_state(0, super::super::Code::Error);
            interp.clear_return_options();
            {
                let mut exc = interp.exc.borrow_mut();
                exc.native.info = None;
                exc.native.info_len = 0;
                exc.already_logged = false;
                exc.native.legacy_copy = true;
            }
            if release.has_error_stack() {
                interp
                    .error_stack
                    .borrow_mut()
                    .begin_inner(b"INNER".to_vec(), b"CALLBACK".to_vec());
            }
            interp.restore_native_variable_trace_result(saved).unwrap();
            assert_eq!(interp.pending_return_level(), 3);
            assert_eq!(
                interp.pending_return_code(),
                super::super::Code::from_int(7)
            );
            assert_eq!(
                interp.pending_return_option_objects()[0].key.as_ptr(),
                key_ptr
            );
            assert_eq!(
                interp.pending_return_option_objects()[0].value.as_ptr(),
                value_ptr
            );
            let exc = interp.exc.borrow();
            assert_eq!(exc.native.info.as_ref().unwrap().as_ptr(), info_ptr);
            assert_eq!(exc.native.info_len, 13);
            assert!(exc.already_logged);
            assert!(exc.native.legacy_copy);
            if release.has_error_stack() {
                let stack = interp.error_stack.borrow();
                assert!(stack.is_reset());
                assert_eq!(stack.entries()[1], b"ORIGINAL");
            }
        }
    }
}

#[cfg(test)]
mod settlement_tests {
    use super::Interp;
    use crate::{frame::VarError, interp::Code, obj};
    use std::{cell::Cell, rc::Rc};
    use tcl_runtime_api::{
        native_variable_trace::{
            NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
        },
        NativeExecutionError,
    };
    use tcl_syntax::raw_string::NativeValueAccessRefusal;

    thread_local! {
        static MODE: Cell<u8> = const { Cell::new(0) };
        static CALLS: Cell<usize> = const { Cell::new(0) };
    }

    fn original(profile: &str) -> Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(profile),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }

    fn change_and_restore(interp: &mut Interp) {
        let original = interp.runtime_context();
        let mut changed = original.clone();
        changed.packages = vec![("trace-settlement".into(), "1.0".into())];
        interp.pin_context(&changed).unwrap();
        interp.pin_context(&original).unwrap();
    }

    fn callback(interp: &mut Interp, mode: u8) -> Result<(), tcl_cmd_core::CmdError> {
        CALLS.with(|calls| calls.set(calls.get() + 1));
        interp.set_result_bytes(b"CALLBACK\0RESULT");
        interp.set_return_state(0, Code::Error);
        match mode {
            1 => Err(tcl_cmd_core::CmdError::new("TRACE GUEST")),
            2 => {
                interp.refuse_host_command("first original trace callback");
                Ok(())
            }
            3 => {
                change_and_restore(interp);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn script(interp: &mut Interp, _: &[*mut obj::TclObj]) -> Code {
        let result = callback(interp, MODE.with(Cell::get));
        match result {
            Ok(()) => Code::Ok,
            Err(error) => interp.report_cmd_error(error),
        }
    }

    struct Direct(u8);
    impl NativeVariableObserver<Interp> for Direct {
        type Error = tcl_cmd_core::CmdError;
        fn observe(
            &self,
            interp: &mut Interp,
            access: NativeVariableTraceAccess<'_>,
        ) -> Result<(), Self::Error> {
            assert_eq!(access.operation, NativeVariableTraceOperation::Write);
            assert_eq!(access.name1, b"x");
            callback(interp, self.0)
        }
    }

    fn register(interp: &mut Interp, script_callback: bool, mode: u8) {
        if script_callback {
            interp.register_builtin(b"original_trace_callback", script);
            assert_eq!(
                interp.eval_str(b"trace add variable x write original_trace_callback"),
                Code::Ok,
            );
        } else {
            let name = obj::Owned::fresh(obj::new_string_bytes(b"x"));
            interp
                .add_native_variable_observer(
                    name.as_ptr(),
                    &[NativeVariableTraceOperation::Write],
                    Rc::new(Direct(mode)),
                )
                .unwrap();
        }
    }

    fn assert_refusal(interp: &Interp, mode: u8) {
        match mode {
            2 => {
                let Some(NativeExecutionError::HostCommandRefusal(first)) =
                    interp.native_execution_refusal()
                else {
                    panic!("actual original trace Host cause was not retained");
                };
                assert_eq!(first.reason, "first original trace callback");
            }
            3 => assert_eq!(
                interp.native_execution_refusal(),
                Some(NativeExecutionError::ValueAccessRefusal(
                    NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "stale entered native operation",
                    ),
                )),
            ),
            _ => assert!(interp.native_execution_refusal().is_none()),
        }
    }

    #[test]
    fn original_script_and_native_trace_settlement_separate_host_from_guest_failure() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Real reached trace/result ownership, not an external callback-equivalence claim.
        for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for script_callback in [false, true] {
                for mode in 0..4 {
                    let mut interp = original(profile);
                    MODE.with(|value| value.set(mode));
                    register(&mut interp, script_callback, mode);
                    interp.set_result_bytes(b"PRIOR\0\xffRESULT");
                    interp.set_return_state(3, Code::Other(7));
                    let prior = obj::Owned::retain(interp.result_obj());
                    let value = obj::Owned::fresh(obj::new_string_bytes(b"REACHED\0VALUE"));
                    CALLS.with(|calls| calls.set(0));
                    let scope =
                        crate::interp::native_operation_currency::NativeOperationScope::enter(
                            &interp,
                        )
                        .unwrap();
                    let result = interp.var_set(b"x", value.as_ptr());
                    assert_eq!(
                        CALLS.with(Cell::get),
                        1,
                        "{profile}/{script_callback}/{mode}"
                    );
                    assert_eq!(
                        result,
                        match mode {
                            0 => Ok(()),
                            1 => Err(VarError::TraceError),
                            // Host/ABA is the neutral legacy unwind carrier;
                            // assert_refusal below checks the full original cause.
                            _ => Err(VarError::NameProtocolUnavailable),
                        }
                    );
                    // This direct storage projection starts no new name operation.
                    assert_eq!(
                        crate::vars::get(
                            &interp.frames.borrow(),
                            &interp.namespaces(),
                            interp.current_ns(),
                            b"x",
                        ),
                        Some(value.as_ptr()),
                    );
                    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 2);
                    assert!(interp.active_var_trace_scopes.borrow().is_empty());
                    assert_eq!(interp.entered_native_operation_count(), 1);
                    assert_refusal(&interp, mode);
                    if mode == 1 {
                        assert_eq!(interp.result_bytes(), b"TRACE GUEST");
                        assert_ne!(interp.result_obj(), prior.as_ptr());
                        assert_eq!(unsafe { (*prior.as_ptr()).ref_count }, 1);
                        assert_eq!(
                            interp.traces.borrow().pending_err.as_deref(),
                            Some(b"TRACE GUEST".as_slice())
                        );
                    } else {
                        assert_eq!(interp.result_obj(), prior.as_ptr());
                        assert_eq!(unsafe { (*prior.as_ptr()).ref_count }, 2);
                        assert_eq!(interp.pending_return_level(), 3);
                        assert_eq!(interp.pending_return_code(), Code::Other(7));
                        assert!(interp.traces.borrow().pending_err.is_none());
                    }
                    if mode >= 2 {
                        let first = interp.native_execution_refusal();
                        assert!(interp.var_get(b"x").is_none());
                        assert_eq!(interp.native_execution_refusal(), first);
                    }
                    drop(scope);
                    assert_eq!(interp.entered_native_operation_count(), 0);
                }
            }
        }
    }

    #[test]
    fn c84_host_trace_settlement_uses_only_the_selected_script_saved_state() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // C84 has Script SaveResult and no chain saved state; neither grants the other purpose.
        for script_callback in [false, true] {
            let mut interp = original("tcl8.4");
            MODE.with(|value| value.set(2));
            register(&mut interp, script_callback, 2);
            interp.set_result_bytes(b"PRIOR\0\xffRESULT");
            let prior = obj::Owned::retain(interp.result_obj());
            let value = obj::Owned::fresh(obj::new_string_bytes(b"REACHED"));
            CALLS.with(|calls| calls.set(0));
            let scope =
                crate::interp::native_operation_currency::NativeOperationScope::enter(&interp)
                    .unwrap();
            assert_eq!(
                interp.var_set(b"x", value.as_ptr()),
                Err(VarError::NameProtocolUnavailable)
            );
            assert_eq!(CALLS.with(Cell::get), 1);
            assert_refusal(&interp, 2);
            assert!(interp.active_var_trace_scopes.borrow().is_empty());
            if script_callback {
                assert_eq!(interp.result_obj(), prior.as_ptr());
                assert_eq!(unsafe { (*prior.as_ptr()).ref_count }, 2);
            } else {
                assert_ne!(interp.result_obj(), prior.as_ptr());
                assert_eq!(interp.result_bytes(), b"CALLBACK\0RESULT");
                assert_eq!(unsafe { (*prior.as_ptr()).ref_count }, 1);
            }
            assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 2);
            drop(scope);
            assert_eq!(interp.entered_native_operation_count(), 0);
        }
    }

    struct Retirement {
        interpreter: crate::interp::WeakInterp,
        calls: Rc<Cell<usize>>,
        observed: Rc<Cell<*mut obj::TclObj>>,
        mode: u8,
        original: super::super::native_operation_currency::NativeOperationCurrency,
    }

    extern "C" fn retire(value: *mut obj::TclObj) {
        // SAFETY: the actual extension owns this single boxed backing.
        let state = unsafe { Box::from_raw(obj::internal_rep(value) as usize as *mut Retirement) };
        state.calls.set(state.calls.get() + 1);
        let mut interp = state.interpreter.upgrade().unwrap();
        state.observed.set(interp.result_obj());
        assert!(interp.active_var_trace_scopes.borrow().is_empty());
        // The original outer owner remains current while write/chain/saved
        // cleanup entries are also live. Their actual count is not authority.
        state.original.ensure_current_or_refuse().unwrap();
        if state.mode == 2 {
            interp.refuse_host_command("first original trace callback");
        } else {
            change_and_restore(&mut interp);
        }
    }

    static CALLBACK_RESULT: obj::TclObjType = obj::TclObjType {
        name: c"originalTraceSettlementResult".as_ptr(),
        free_int_rep_proc: Some(retire),
        dup_int_rep_proc: None,
        update_string_proc: None,
        set_from_any_proc: None,
    };

    struct RetirementObserver {
        calls: Rc<Cell<usize>>,
        observed: Rc<Cell<*mut obj::TclObj>>,
        mode: u8,
        original: Rc<
            std::cell::RefCell<
                Option<super::super::native_operation_currency::NativeOperationCurrency>,
            >,
        >,
    }
    impl NativeVariableObserver<Interp> for RetirementObserver {
        type Error = tcl_cmd_core::CmdError;
        fn observe(
            &self,
            interp: &mut Interp,
            _: NativeVariableTraceAccess<'_>,
        ) -> Result<(), Self::Error> {
            let state = Box::new(Retirement {
                interpreter: interp.downgrade(),
                calls: self.calls.clone(),
                observed: self.observed.clone(),
                mode: self.mode,
                original: self.original.borrow().as_ref().unwrap().clone(),
            });
            let value = obj::alloc_typed(&CALLBACK_RESULT, Box::into_raw(state) as usize as u64);
            // SAFETY: independently resident String; no getter is forced by the observer.
            unsafe { obj::set_string_rep(value, b"CALLBACK") };
            interp.set_result(value);
            Ok(())
        }
    }

    #[test]
    fn original_trace_result_retirement_is_checked_after_saved_owner_restoration() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Actual extension free hook sees publication outside borrows and retains its first cause.
        for mode in [2, 3] {
            let mut interp = original("tcl8.6");
            let calls = Rc::new(Cell::new(0));
            let observed = Rc::new(Cell::new(std::ptr::null_mut()));
            let original = Rc::new(std::cell::RefCell::new(None));
            let name = obj::Owned::fresh(obj::new_string_bytes(b"x"));
            interp
                .add_native_variable_observer(
                    name.as_ptr(),
                    &[NativeVariableTraceOperation::Write],
                    Rc::new(RetirementObserver {
                        calls: calls.clone(),
                        observed: observed.clone(),
                        mode,
                        original: original.clone(),
                    }),
                )
                .unwrap();
            interp.set_result_bytes(b"PRIOR");
            let prior = obj::Owned::retain(interp.result_obj());
            let value = obj::Owned::fresh(obj::new_string_bytes(b"REACHED"));
            let scope =
                crate::interp::native_operation_currency::NativeOperationScope::enter(&interp)
                    .unwrap();
            *original.borrow_mut() = Some(scope.currency().clone());
            assert_eq!(
                interp.var_set(b"x", value.as_ptr()),
                Err(VarError::NameProtocolUnavailable)
            );
            assert_eq!(calls.get(), 1);
            assert_eq!(observed.get(), prior.as_ptr());
            assert_eq!(interp.result_obj(), prior.as_ptr());
            assert_eq!(unsafe { (*prior.as_ptr()).ref_count }, 2);
            assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 2);
            assert_refusal(&interp, mode);
            assert!(interp.traces.borrow().pending_err.is_none());
            assert_eq!(interp.entered_native_operation_count(), 1);
            drop(scope);
            assert_eq!(interp.entered_native_operation_count(), 0);
        }
    }
}
