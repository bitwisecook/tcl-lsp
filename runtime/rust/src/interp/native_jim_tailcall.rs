// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Actual Jim command references and selected tailcall argument Lists.

use super::{
    native_arguments::{OriginalArgumentFrame, OriginalArguments},
    native_operation_currency::{NativeOperationScope, SavedNativeOperationCurrency},
    Code, Command, Interp, WeakInterp,
};
use crate::obj::{self, Owned, TclObj};
use std::rc::Rc;
use tcl_syntax::value::ValueError;

/// One genuine command reference, shared without acquiring another native role.
#[derive(Clone)]
pub(super) struct JimCommandReference(Rc<JimCommandReferenceData>);

pub(super) struct JimCommandReferenceData {
    interpreter: WeakInterp,
    currency: SavedNativeOperationCurrency,
    token: u64,
    command: Option<Command>,
}
impl Drop for JimCommandReferenceData {
    fn drop(&mut self) {
        let Some(mut interpreter) = self.interpreter.upgrade() else {
            drop(self.command.take());
            return;
        };
        let _cleanup = NativeOperationScope::enter_saved_cleanup(&self.currency, &interpreter);
        // Command delete hooks precede previous-command edge retirement.
        drop(self.command.take());
        interpreter
            .namespaces
            .borrow_mut()
            .leave_jim_command(self.token);
        if let Err(first) = interpreter.retire_pending_native_ensemble_roles() {
            interpreter.refuse_native_execution(first);
        }
    }
}
impl JimCommandReference {
    pub(super) fn token(&self) -> u64 {
        self.0.token
    }
    pub(super) fn command(&self) -> Command {
        self.0
            .command
            .as_ref()
            .expect("live actual Jim command reference")
            .clone()
    }
    pub(super) fn check(&self, actual: &Interp) -> Result<(), ValueError> {
        self.0.currency.check(actual).map_err(|_| {
            ValueError::CommandProtocolUnavailable("selected Jim command original operation")
        })
    }
    pub(super) fn same_reference(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

enum JimTailcallRetirement {
    Pending,
    Entered,
}

/// The original argument List and already selected physical command allocation.
/// Neither a namespace spelling nor a coordinate is needed after selection.
pub(crate) struct JimSelectedTailcall {
    original: Option<Owned>,
    selected: Option<JimCommandReference>,
    retirement: JimTailcallRetirement,
}
impl Drop for JimSelectedTailcall {
    fn drop(&mut self) {
        let interpreter = self
            .selected
            .as_ref()
            .and_then(|selected| selected.0.interpreter.upgrade());
        let _cleanup = interpreter.as_ref().and_then(|interpreter| {
            self.selected.as_ref().and_then(|selected| {
                NativeOperationScope::enter_saved_cleanup(&selected.0.currency, interpreter).ok()
            })
        });
        match self.retirement {
            JimTailcallRetirement::Pending => {
                // Uninvoked frame cleanup releases tailcallCmd before tailcallObj.
                drop(self.selected.take());
                drop(self.original.take());
            }
            JimTailcallRetirement::Entered => {
                // The selected evaluation releases tailcallObj before its Jim_Cmd.
                drop(self.original.take());
                drop(self.selected.take());
            }
        }
    }
}

impl Interp {
    pub(super) fn retained_jim_command_reference(&self, token: u64) -> Option<JimCommandReference> {
        self.jim_command_references
            .borrow()
            .iter()
            .rev()
            .find_map(|reference| {
                let reference = reference.upgrade()?;
                (reference.token == token).then_some(JimCommandReference(reference))
            })
    }

    pub(super) fn capture_jim_command_reference(
        &self,
        command: &Command,
        token: u64,
    ) -> Result<JimCommandReference, ValueError> {
        self.check_entered_native_operation().map_err(|_| {
            ValueError::CommandProtocolUnavailable("Jim command reference original operation")
        })?;
        if self
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_none()
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "Jim command reference issuer",
            ));
        }
        let published = self
            .namespaces
            .borrow()
            .jim_node_command(token)
            .or_else(|| {
                self.hidden
                    .borrow()
                    .values()
                    .find(|binding| binding.generation == token)
                    .map(|binding| binding.command.clone())
            });
        let actual = match published {
            Some(command) => Some(command),
            None => match self.retained_jim_command_reference(token) {
                Some(reference) => {
                    reference.check(self)?;
                    Some(reference.command())
                }
                None => None,
            },
        };
        if !actual.is_some_and(|actual| actual.is_same_binding(command)) {
            return Err(ValueError::CommandProtocolUnavailable(
                "Jim command reference physical worker",
            ));
        }
        let currency = SavedNativeOperationCurrency::capture(self);
        currency.check(self).map_err(|_| {
            ValueError::CommandProtocolUnavailable("Jim command reference currency")
        })?;
        let reference: Rc<JimCommandReferenceData> = Rc::new(JimCommandReferenceData {
            interpreter: self.downgrade(),
            currency,
            token,
            command: Some(command.clone()),
        });
        self.namespaces.borrow_mut().enter_jim_command(token);
        let mut references = self.jim_command_references.borrow_mut();
        references.retain(|reference| reference.strong_count() != 0);
        references.push(Rc::downgrade(&reference));
        Ok(JimCommandReference(reference))
    }

    pub(super) fn schedule_jim_tailcall(&mut self, words: &[*mut TclObj]) -> Code {
        let Some(&head) = words.first() else {
            return Code::Ok;
        };
        let arguments = match OriginalArguments::enter(self, words) {
            Ok(arguments) => arguments,
            Err(error) => return self.report_cmd_error(error),
        };
        let frame = OriginalArgumentFrame::capture(self);
        let namespace = self.current_ns();
        let code = (|| {
            let reached = self.resolve_original_jim_command(namespace, head);
            if let Err(error) = frame.check(self, &arguments) {
                return self.report_cmd_error(error);
            }
            if self.current_ns() != namespace {
                return self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable(
                        "Jim tailcall original namespace changed",
                    )
                    .into(),
                );
            }
            let (command, token) = match reached {
                Ok(Some((command, Some(token)))) => (command, token),
                Ok(None) => {
                    let name = arguments.bytes(0);
                    if let Err(error) = frame.check(self, &arguments) {
                        return self.report_cmd_error(error);
                    }
                    if self.current_ns() != namespace {
                        return self.report_cmd_error(
                            ValueError::CommandProtocolUnavailable(
                                "Jim tailcall original namespace changed",
                            )
                            .into(),
                        );
                    }
                    return match name {
                        Ok(name) => self.invalid_command_result(&name),
                        Err(error) => self.report_cmd_error(error),
                    };
                }
                Ok(Some((_, None))) => {
                    return self.report_cmd_error(
                        ValueError::CommandProtocolUnavailable("Jim tailcall command allocation")
                            .into(),
                    )
                }
                Err(error) => return self.report_cmd_error(error.into()),
            };
            let selected = match self.capture_jim_command_reference(&command, token) {
                Ok(selected) => selected,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            drop(command);
            let original = Owned::fresh(self.new_list_object(words));
            if let Err(error) = frame.check(self, &arguments) {
                return self.report_cmd_error(error);
            }
            if self.current_ns() != namespace {
                return self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable(
                        "Jim tailcall original namespace changed",
                    )
                    .into(),
                );
            }
            let request = JimSelectedTailcall {
                original: Some(original),
                selected: Some(selected),
                retirement: JimTailcallRetirement::Pending,
            };
            if !self.replace_pending_tailcall(Some(crate::frame::PendingTailcall::Jim(request))) {
                return self.error(b"tailcall can only be called from a proc, lambda or method");
            }
            Code::from_int(
                tcl_registry::invocation_words::NativeTailcallProtocol::Jim084.pending_code(),
            )
        })();
        arguments.finish(code)
    }

    pub(super) fn invoke_selected_jim_tailcall(
        &mut self,
        mut request: JimSelectedTailcall,
    ) -> Code {
        let selected = request
            .selected
            .as_ref()
            .expect("actual selected Jim command");
        if let Err(error) = selected.check(self) {
            return self.report_cmd_error(error.into());
        }
        let original = request.original.as_ref().expect("actual Jim tailcall List");
        let words = crate::list::list_elements_native_checked(
            original.as_ptr(),
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        );
        if let Err(error) = selected.check(self) {
            return self.report_cmd_error(error.into());
        }
        let words = match words {
            Ok(words) => words,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        request.retirement = JimTailcallRetirement::Entered;
        self.invoke_bound_with_jim_reference(
            selected.command(),
            Some(selected.token()),
            &words,
            Some(selected.clone()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::namespace::GLOBAL;
    use std::{
        cell::RefCell,
        ffi::{c_int, c_void},
    };

    fn jim() -> Interp {
        Interp::with_native_core(
            super::super::default_host(),
            crate::environment::profile_for_dialect("jimtcl"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }
    fn word(interpreter: &mut Interp, bytes: &[u8]) -> Owned {
        let original = Owned::fresh(obj::new_string_bytes(bytes));
        crate::native_source::bind_context(
            original.as_ptr(),
            &interpreter.native_jim_object_context().unwrap(),
        )
        .unwrap();
        original
    }
    fn oracle(kind: &str, label: &str) -> Vec<&'static str> {
        include_str!("../../tests/data/native_jim_tailcall/selection-20261015.tsv")
            .lines()
            .find(|line| {
                let mut fields = line.split('|');
                fields.next() == Some(kind) && fields.next() == Some(label)
            })
            .unwrap()
            .split('|')
            .collect()
    }

    #[test]
    fn original_jim_tailcall_keeps_same_object_hot_namespace_and_return_completion() {
        for missing in [false, true] {
            let mut interpreter = jim();
            let namespace = Owned::fresh(obj::new_wide_int_obj(42));
            crate::native_source::bind_context(
                namespace.as_ptr(),
                &interpreter.native_jim_object_context().unwrap(),
            )
            .unwrap();
            let original_type = obj::obj_type_ptr(namespace.as_ptr());
            let formals = word(&mut interpreter, b"target");
            let body = word(&mut interpreter, b"tailcall $target VALUE");
            let lambda = Owned::fresh(interpreter.new_list_object(&[
                formals.as_ptr(),
                body.as_ptr(),
                namespace.as_ptr(),
            ]));
            let head = word(&mut interpreter, b"apply");
            let target = word(
                &mut interpreter,
                if missing { b"::missing" } else { b"::return" },
            );
            for phase in 0..if missing { 1 } else { 2 } {
                let label = if missing {
                    "MISSING_TARGET"
                } else if phase == 0 {
                    "COLD_CACHED"
                } else {
                    "HOT_CACHED"
                };
                if phase != 0 {
                    obj::invalidate_string(namespace.as_ptr());
                }
                assert_eq!(
                    obj::has_string_rep(namespace.as_ptr()),
                    oracle("NAMESPACE_BEFORE", label)[2] == "1"
                );
                let code = interpreter.eval_original_object_vector(&[
                    head.as_ptr(),
                    lambda.as_ptr(),
                    target.as_ptr(),
                ]);
                // Observe actual headers before any result getter.
                assert_eq!(
                    obj::has_string_rep(namespace.as_ptr()),
                    oracle("NAMESPACE_AFTER", label)[2] == "1",
                    "{label}"
                );
                assert_eq!(obj::obj_type_ptr(namespace.as_ptr()), original_type);
                assert_eq!(code.as_int().to_string(), oracle("RESULT", label)[2]);
                let result = oracle("RESULT", label)[3]
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|bytes| {
                        u8::from_str_radix(std::str::from_utf8(bytes).unwrap(), 16).unwrap()
                    })
                    .collect::<Vec<_>>();
                assert_eq!(interpreter.result_bytes(), result, "{label}");
                assert!(interpreter.native_execution_refusal().is_none());
                assert!(interpreter.jim_active_command_workers.borrow().is_empty());
            }
        }
    }

    thread_local! {
        static EVENTS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
        static ARGUMENT_CLEANUP: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    }
    extern "C" fn release_argument(_: *mut TclObj) {
        EVENTS.with(|events| events.borrow_mut().push("argument"));
        let target =
            ARGUMENT_CLEANUP.with(|target| target.borrow().as_ref().and_then(WeakInterp::upgrade));
        if let Some(mut target) = target {
            let first = target
                .native_execution_refusal()
                .expect("original withdrawn operation");
            assert!(target.check_entered_native_operation().is_err());
            assert_eq!(target.eval_str(b"set revived TRUE"), Code::Error);
            assert_eq!(target.native_execution_refusal(), Some(first));
        }
    }
    static ARGUMENT: obj::TclObjType = obj::TclObjType {
        name: c"tailcall-original-argument".as_ptr(),
        free_int_rep_proc: Some(release_argument),
        dup_int_rep_proc: None,
        update_string_proc: None,
        set_from_any_proc: None,
    };
    unsafe extern "C" fn selected_worker(
        _: *mut c_void,
        interpreter: *mut Interp,
        argc: c_int,
        _: *const *mut TclObj,
    ) -> c_int {
        assert_eq!(argc, 2);
        EVENTS.with(|events| {
            let mut events = events.borrow_mut();
            assert!(events.is_empty());
            events.push("invoke");
        });
        // SAFETY: the actual synchronous callback receives its live interpreter.
        unsafe { (*interpreter).set_result_bytes(b"VALUE") };
        0
    }
    unsafe extern "C" fn delete_selected(_: *mut c_void) {
        EVENTS.with(|events| events.borrow_mut().push("delete"));
    }

    fn pending(interpreter: &mut Interp) -> (crate::frame::PendingTailcall, Owned) {
        interpreter
            .create_obj_command(
                b"selected",
                super::super::ObjCommand::new(
                    selected_worker,
                    std::ptr::null_mut(),
                    Some(delete_selected),
                ),
            )
            .unwrap();
        let head = word(interpreter, b"::selected");
        let argument = Owned::fresh(obj::alloc_typed(&ARGUMENT, 0));
        crate::native_source::bind_context(
            argument.as_ptr(),
            &interpreter.native_jim_object_context().unwrap(),
        )
        .unwrap();
        interpreter.frames.borrow_mut().push(GLOBAL);
        assert_eq!(
            interpreter.schedule_tailcall(&[head.as_ptr(), argument.as_ptr()]),
            Code::from_int(7)
        );
        let request = interpreter.frames.borrow_mut().take_tailcall().unwrap();
        interpreter.frames.borrow_mut().pop();
        drop(argument);
        (request, head)
    }

    #[test]
    fn selected_jim_tailcall_survives_real_table_removal_then_retires_list_before_worker() {
        let mut interpreter = jim();
        EVENTS.with(|events| events.borrow_mut().clear());
        let (request, head) = pending(&mut interpreter);
        let generation = obj::with_jim_command_cache(head.as_ptr(), |cache| cache.token).unwrap();
        assert!(interpreter
            .namespaces
            .borrow_mut()
            .delete(GLOBAL, b"selected"));
        assert!(EVENTS.with(|events| events.borrow().is_empty()));
        // The genuine original cached pointer still names the retained worker.
        assert_eq!(
            interpreter
                .resolve_original_command(head.as_ptr())
                .unwrap()
                .unwrap()
                .1,
            Some(generation)
        );
        let fresh = word(&mut interpreter, b"::selected");
        assert!(interpreter
            .resolve_original_command(fresh.as_ptr())
            .unwrap()
            .is_none());
        assert_eq!(interpreter.dispatch_tailcall(request), Code::Ok);
        assert_eq!(interpreter.result_bytes(), b"VALUE");
        assert_eq!(
            EVENTS.with(|events| events.borrow().clone()),
            ["invoke", "argument", "delete"]
        );
        assert!(interpreter
            .resolve_original_command(head.as_ptr())
            .unwrap()
            .is_none());
        assert!(interpreter.jim_active_command_workers.borrow().is_empty());
        assert!(interpreter
            .retained_jim_command_reference(generation)
            .is_none());
    }

    #[test]
    fn withdrawn_jim_tailcall_refuses_invocation_and_releases_its_actual_list_and_worker() {
        let mut interpreter = jim();
        EVENTS.with(|events| events.borrow_mut().clear());
        let (request, head) = pending(&mut interpreter);
        let generation = obj::with_jim_command_cache(head.as_ptr(), |cache| cache.token).unwrap();
        assert!(interpreter
            .namespaces
            .borrow_mut()
            .delete(GLOBAL, b"selected"));
        ARGUMENT_CLEANUP.with(|target| *target.borrow_mut() = Some(interpreter.downgrade()));
        interpreter.set_dialect_profile(crate::environment::profile_for_dialect("tcl8.6"));
        assert_eq!(interpreter.dispatch_tailcall(request), Code::Error);
        ARGUMENT_CLEANUP.with(|target| *target.borrow_mut() = None);
        let first = interpreter
            .native_execution_refusal()
            .expect("original target withdrawal");
        assert_eq!(
            EVENTS.with(|events| events.borrow().clone()),
            ["delete", "argument"]
        );
        assert!(interpreter
            .retained_jim_command_reference(generation)
            .is_none());
        assert!(interpreter.jim_active_command_workers.borrow().is_empty());
        assert_eq!(interpreter.native_execution_refusal(), Some(first));
    }
    #[test]
    fn last_live_jim_interpreter_releases_deferred_tailcall_without_a_handle_cycle() {
        EVENTS.with(|events| events.borrow_mut().clear());
        let mut interpreter = jim();
        let (request, head) = pending(&mut interpreter);
        assert!(interpreter
            .namespaces
            .borrow_mut()
            .delete(GLOBAL, b"selected"));
        interpreter
            .deferred_tailcalls
            .borrow_mut()
            .push((1, request));
        drop(head);
        let birth = interpreter.downgrade();
        drop(interpreter);
        assert!(birth.upgrade().is_none());
        assert_eq!(
            EVENTS.with(|events| events.borrow().clone()),
            ["delete", "argument"]
        );
    }
    #[test]
    fn nested_jim_tailcalls_keep_selected_local_and_previous_command_owners() {
        let mut interpreter = jim();
        assert_eq!(interpreter.eval_str(
            b"proc target {} {return BASE}; proc issuer {} {local proc target {} {tailcall ::return VALUE}; tailcall target}"
        ), Code::Ok);
        let issuer = word(&mut interpreter, b"issuer");
        assert_eq!(
            interpreter.eval_original_object_vector(&[issuer.as_ptr()]),
            Code::Return
        );
        assert_eq!(interpreter.result_bytes(), b"VALUE");
        assert_eq!(interpreter.eval_str(b"target"), Code::Ok);
        assert_eq!(interpreter.result_bytes(), b"BASE");
        assert!(interpreter.jim_active_command_workers.borrow().is_empty());
        assert!(interpreter
            .jim_command_references
            .borrow()
            .iter()
            .all(|owner| owner.strong_count() == 0));
        assert!(interpreter.native_execution_refusal().is_none());
    }
}
