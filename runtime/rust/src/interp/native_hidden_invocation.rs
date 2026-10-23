// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original hidden-command context selection and mandatory frame retirement.

use super::native_operation_currency::{NativeOperationScope, SavedNativeOperationCurrency};
use super::{Code, Interp, TclObj};
use std::rc::Weak;
use tcl_runtime_api::NativeExecutionError;

struct SelectedHiddenFrame {
    previous_namespace: crate::namespace::NsId,
    caller_level: usize,
    caller: Weak<()>,
    namespace_frame: Option<(usize, usize, Weak<()>)>,
}

impl Interp {
    pub(crate) fn invoke_hidden_in_namespace(
        &mut self,
        original_namespace: &[u8],
        command: &[u8],
        argv: &[*mut TclObj],
    ) -> Code {
        let Some(grammar) = self
            .native_invocation_dialect()
            .native_name_protocol()
            .and_then(|protocol| protocol.hidden_invocation_grammar().ok())
        else {
            return self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original hidden-invocation frame policy",
                )
                .into(),
            );
        };
        let original = SavedNativeOperationCurrency::capture(self);
        if let Err(first) = original.check(self) {
            return self.refuse_native_execution(first);
        }
        let target = match self.ensure_global_namespace_address(original_namespace) {
            Ok(target) => target,
            Err(error) => return self.report_cmd_error(error),
        };
        let (caller_level, caller) = {
            let frames = self.frames.borrow();
            (frames.current_level(), frames.current_activation_owner())
        };
        let mut selected = SelectedHiddenFrame {
            previous_namespace: self.current_ns(),
            caller_level,
            caller,
            namespace_frame: None,
        };
        if grammar.uses_global_variable_frame() {
            if target != crate::namespace::GLOBAL {
                return self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original hidden-invocation global frame",
                    )
                    .into(),
                );
            }
            self.frames.borrow_mut().set_active_level(0);
        } else {
            self.enter_namespace_activation(target);
            let mut frames = self.frames.borrow_mut();
            let level = frames.push_namespace(target);
            selected.namespace_frame = Some((
                level,
                frames.current_frame_index(),
                frames.current_activation_owner(),
            ));
        }
        self.current_ns.set(target);
        let code = self.invoke_hidden(command, argv);
        match self.retire_hidden_invocation_frame(selected, &original) {
            Ok(()) => code,
            Err(first) => self.refuse_native_execution(first),
        }
    }

    fn retire_hidden_invocation_frame(
        &mut self,
        selected: SelectedHiddenFrame,
        original: &SavedNativeOperationCurrency,
    ) -> Result<(), NativeExecutionError> {
        let cleanup = NativeOperationScope::enter_saved_cleanup(original, self)?;
        if let Some((level, index, owner)) = selected.namespace_frame {
            let same_frame = {
                let mut frames = self.frames.borrow_mut();
                let previous = frames.set_active_level(level);
                let same = frames.top_level() == level
                    && frames.current_frame_index() == index
                    && owner.upgrade().is_some()
                    && owner.ptr_eq(&frames.current_activation_owner());
                if !same {
                    frames.set_active_level(previous);
                }
                same
            };
            if same_frame {
                let popped = self.pop_native_call_frame();
                self.current_ns.set(selected.previous_namespace);
                self.leave_namespace_activation(popped);
            } else {
                self.refuse_host_command("original hidden-invocation frame is unavailable");
            }
        }
        let same_caller = self
            .frames
            .borrow()
            .original_activation_owner_at(selected.caller_level)
            .is_some_and(|owner| {
                selected.caller.upgrade().is_some() && owner.ptr_eq(&selected.caller)
            });
        if same_caller {
            self.frames
                .borrow_mut()
                .set_active_level(selected.caller_level);
        } else {
            self.refuse_host_command("original hidden-invocation caller is unavailable");
        }
        self.current_ns.set(selected.previous_namespace);
        drop(cleanup);
        original.check(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    thread_local! {
        static FIRST: std::cell::RefCell<Option<NativeExecutionError>> = const { std::cell::RefCell::new(None) };
    }

    fn withdraw_hidden_frame(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
        interpreter.refuse_host_command("first hidden namespace frame withdrawal");
        FIRST.with(|first| *first.borrow_mut() = interpreter.native_execution_refusal());
        let popped = interpreter.pop_native_call_frame();
        interpreter.leave_namespace_activation(popped);
        interpreter.set_current_ns(crate::namespace::GLOBAL);
        Code::Error
    }

    #[test]
    fn consumed_hidden_frame_restores_the_actual_caller_and_preserves_first_cause() {
        let mut interpreter = Interp::with_native_core(
            super::super::default_host(),
            crate::environment::profile_for_dialect("tcl8.6"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        interpreter.register_builtin(b"withdraw", withdraw_hidden_frame);
        assert!(matches!(
            interpreter.hide_command(b"withdraw", b"held"),
            crate::interp::CommandVisibilityOutcome::Moved,
        ));
        let caller = interpreter.ensure_namespace(b"Caller");
        interpreter.enter_namespace_activation(caller);
        interpreter.frames.borrow_mut().push_namespace(caller);
        interpreter.set_current_ns(caller);
        let original = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"held"));
        assert_eq!(
            interpreter.invoke_hidden_in_namespace(b"Chosen", b"held", &[original.as_ptr()]),
            Code::Error,
        );
        assert_eq!(interpreter.current_level(), 1);
        assert_eq!(interpreter.current_ns(), caller);
        let first = FIRST.with(|first| first.borrow().clone()).unwrap();
        assert_eq!(interpreter.native_execution_refusal(), Some(first));
    }

    #[test]
    fn hidden_global_frame_levels_match_original_c_releases() {
        for version in tcl_dialect::TclVersion::ALL {
            let mut interpreter = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect(version.dialect_profile_name()),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let source = br#"
interp hide {} info hiddenInfo
proc observe {} {
    list [interp invokehidden {} -global hiddenInfo level] [interp invokehidden {} hiddenInfo level]
}
list [observe] [interp invokehidden {} -global hiddenInfo level]
"#;
            assert_eq!(interpreter.eval_str(source), Code::Ok);
            assert_eq!(
                interpreter.result_bytes(),
                if version == tcl_dialect::TclVersion::V8_4 {
                    b"{0 1} 0".as_slice()
                } else {
                    b"{2 1} 1".as_slice()
                },
            );
            assert_eq!(interpreter.current_level(), 0);
            assert_eq!(interpreter.current_ns(), crate::namespace::GLOBAL);
        }
    }

    #[test]
    fn hidden_namespace_store_uses_its_actual_frame_and_restores_the_proc_caller() {
        for version in tcl_dialect::TclVersion::ALL {
            if version == tcl_dialect::TclVersion::V8_4 {
                continue;
            }
            let mut interpreter = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect(version.dialect_profile_name()),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let source = br#"
interp hide {} set hiddenSet
proc observe {} {
    interp invokehidden {} hiddenSet item LOCAL
    list [interp invokehidden {} -namespace chosen hiddenSet item NAMESPACE] [interp invokehidden {} hiddenSet item]
}
list [observe] [interp invokehidden {} hiddenSet ::chosen::item]
"#;
            assert_eq!(interpreter.eval_str(source), Code::Ok);
            assert_eq!(interpreter.result_bytes(), b"{NAMESPACE LOCAL} NAMESPACE");
            assert_eq!(interpreter.current_level(), 0);
            assert_eq!(interpreter.current_ns(), crate::namespace::GLOBAL);
        }
    }
}
