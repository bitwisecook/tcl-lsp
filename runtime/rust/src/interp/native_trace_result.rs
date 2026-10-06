// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected native ownership of the result across variable trace callbacks.

use super::Interp;
use crate::obj;

pub(super) struct NativeTraceResultState {
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
    ) -> Option<NativeTraceResultState> {
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
    ) -> Option<NativeTraceResultState> {
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
    ) -> Option<NativeTraceResultState> {
        use tcl_registry::native_variable_trace::NativeTraceStateRecipe;
        let recipe = self
            .native_invocation_dialect()
            .native_trace_state_recipe(purpose)?;
        if recipe == NativeTraceStateRecipe::None {
            return None;
        }
        if matches!(
            recipe,
            NativeTraceStateRecipe::MoveResult | NativeTraceStateRecipe::MoveResultAndCode
        ) {
            let replacement = obj::Owned::fresh(obj::new_string_bytes(b""));
            let original = self.result.replace(replacement.into_raw());
            // SAFETY: SaveResult transfers the interpreter's existing +1.
            let result = unsafe { obj::Owned::from_raw(original) };
            return Some(NativeTraceResultState {
                result,
                interpreter: None,
                reset_before_restore: true,
                return_code: (recipe == NativeTraceStateRecipe::MoveResultAndCode)
                    .then(|| self.return_code.get()),
            });
        }
        Some(NativeTraceResultState {
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
        })
    }

    pub(super) fn restore_native_variable_trace_result(&self, saved: NativeTraceResultState) {
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
            self.reset_native_global_error_episode();
        }
        if let Some(state) = saved.interpreter {
            self.return_code.set(state.code);
            self.return_level.set(state.level);
            self.restore_native_error_trace_state(state.error);
            if let Some(stack) = state.stack {
                let retired = self.error_stack.replace(stack);
                drop(retired);
            }
            self.restore_return_options(state.options);
        }
        let retired = self.result.replace(saved.result.into_raw());
        // SAFETY: the interpreter transfers its current +1 to this retirement;
        // the saved +1 is already installed before any object release.
        unsafe { obj::decr_ref_count(retired) };
        if let Some(code) = saved.return_code {
            self.return_code.set(code);
        }
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
            let absent = interp.save_native_variable_trace_result(!script);
            assert!(absent.is_none());
            // SAFETY: the interpreter owns this live original result.
            assert_eq!(unsafe { (*original).ref_count }, 1);
            let saved = interp.save_native_variable_trace_result(script).unwrap();
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
            interp.restore_native_variable_trace_result(saved);
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
            .unwrap();
            // SAFETY: the selected saved state owns this original +1.
            assert_eq!(unsafe { (*original).ref_count }, 1);
            interp.set_return_state(0, super::super::Code::Error);
            interp.set_result_bytes(b"CALLBACK");
            interp.restore_native_variable_trace_result(saved);
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
            let saved = interp.save_native_variable_trace_result(false).unwrap();
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
            interp.restore_native_variable_trace_result(saved);
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
