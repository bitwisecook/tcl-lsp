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

//! `tailcall`: the request a procedure issues and dispatching its replacement after the procedure has left.

use super::super::instructions::pop;
use super::super::{Frame, OriginalExecutionOwner, TailcallReq, Tick};
use crate::interp::native_operation_currency::NativeOperationScope;
use crate::interp::{Vm, err};
use crate::value::Value;
use std::rc::Rc;
use tcl_bytecode::{FunctionAsm, Instruction, Op};
use tcl_runtime_api::{Code, Completion};

impl Vm {
    pub(crate) fn original_tailcall_request(
        &mut self,
        words: &[Value],
    ) -> Result<TailcallReq, Completion<Value>> {
        let original_scope = NativeOperationScope::enter(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            if !self.frame_owns_local_variables(self.current_level()) {
                return Err(crate::command::err_with_code(
                    "tailcall can only be called from a proc, lambda or method",
                    "TCL TAILCALL ILLEGAL",
                ));
            }
            let namespace = tcl_cmd_core::namespace::current_original(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            let protocol = self
                .object_protocol()
                .ok_or_else(|| self.refuse_host_command("original tailcall List issuer".into()))?;
            let mut members = Vec::with_capacity(words.len() + 1);
            members.push(namespace);
            members.extend_from_slice(words);
            let original = Value::native_list_constructor(members, protocol);
            let values = self
                .native_object_list_elements_in(&original, protocol)
                .map_err(|error| crate::command::completion_from_tcl_error(self, error.into()))?;
            let (namespace, words) = values.split_first().expect("original namespace prefix");
            Ok(TailcallReq {
                namespace: namespace.native_lifetime_lease().into_value(),
                words: words
                    .iter()
                    .map(|word| word.native_lifetime_lease().into_value())
                    .collect(),
                original_list: Some(original),
                original_scope: OriginalExecutionOwner::Entered(original_scope.clone()),
            })
        })();
        self.finish_original_dispatch(&original_scope, reached)
    }

    /// Preserve the real namespace-prefixed List and borrow its original members.
    pub(in super::super) fn native_tailcall_tick(&mut self, list: Value) -> Tick {
        let original_scope = match NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(error) => {
                return Tick::Return(crate::command::completion_from_cmd_error(self, error));
            }
        };
        let reached = (|| {
            let Some(protocol) = self.object_protocol() else {
                return Tick::Return(
                    self.refuse_host_command("native tailcall List issuer".into()),
                );
            };
            let members = match self.native_object_list_elements_in(&list, protocol) {
                Ok(members) => members,
                Err(error) => {
                    return Tick::Return(crate::command::completion_from_tcl_error(
                        self,
                        error.into(),
                    ));
                }
            };
            let Some((namespace, words)) = members.split_first() else {
                return Tick::Return(err("tailcall: missing namespace"));
            };
            let request = TailcallReq {
                namespace: namespace.native_lifetime_lease().into_value(),
                words: words
                    .iter()
                    .map(|value| value.native_lifetime_lease().into_value())
                    .collect(),
                original_list: Some(list),
                original_scope: OriginalExecutionOwner::Entered(original_scope.clone()),
            };
            Tick::Tailcall(request)
        })();
        if let Some(first) = self.original_scope_refusal(&original_scope) {
            drop(reached);
            let _ = original_scope.check(self);
            Tick::Return(first)
        } else {
            reached
        }
    }

    /// Dispatch a procedure-owned replacement after its frame and leave
    /// callbacks have completed. The target's own activation remains yieldable.
    pub(in super::super) fn dispatch_tailcall(
        &mut self,
        acts: &mut Vec<Frame>,
        request: TailcallReq,
    ) -> Option<Completion<Value>> {
        let TailcallReq {
            namespace,
            words,
            original_list,
            original_scope,
        } = request;
        let target =
            match crate::interp::native_operation_currency::SavedExecutionTarget::from_currency(
                original_scope.currency(),
            ) {
                Ok(target) => target,
                Err(first) => {
                    self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(
                        first.clone(),
                    ));
                    drop((namespace, words, original_list));
                    return Some(crate::command::completion_from_cmd_error(
                        self,
                        tcl_cmd_core::CmdError::from_execution_refusal(first),
                    ));
                }
            };
        let mut original_scope = original_scope;
        let original_scope = match original_scope.resume(self) {
            Ok(scope) => scope,
            Err(error) => {
                let completion = crate::command::completion_from_cmd_error(self, error);
                drop((namespace, words, original_list));
                let cleanup = self.abort_refused_activations_using(acts, Some(&target));
                drop(completion);
                return Some(cleanup);
            }
        };
        let reached = (|| {
            if let Some(first) = self.original_scope_refusal(&original_scope) {
                return self.deliver_tailcall_completion(acts, first);
            }
            let context = match self.namespace_object_lookup(&namespace) {
                Ok(Some(context)) => context,
                Ok(None) => {
                    let completion = match self.native_name_operand_bytes_checked(&namespace) {
                        Ok(bytes) => self.namespace_lookup_error_bytes(&bytes),
                        Err(error) => crate::command::completion_from_cmd_error(self, error),
                    };
                    return self.deliver_tailcall_completion(acts, completion);
                }
                Err(error) => {
                    let completion = crate::command::completion_from_cmd_error(self, error);
                    return self.deliver_tailcall_completion(acts, completion);
                }
            };
            let words = crate::NativeListItems::invocation_view(Rc::new(words));
            if acts.is_empty() {
                let asm = FunctionAsm {
                    instructions: vec![Instruction::new(Op::DONE, vec![])],
                    ..FunctionAsm::default()
                };
                let unit = self.compiled_unit(Rc::new(asm), self.source_namespace_path());
                self.push_frame(acts, Frame::new(unit, false));
            }
            let parent = acts
                .last_mut()
                .expect("replacement result activation present");
            if let Some(owner) = original_list {
                parent.tailcall_owners.push(owner);
            }
            match self.dispatch_words_entry_at(parent, &words, None, context, &[], true) {
                Ok(Some(mut tick)) => {
                    if let Tick::PushEachLoop { req, .. } = &mut tick {
                        req.invocation = Some((words.clone(), 1));
                    }
                    parent.tailcall_invocation = Some(words.clone());
                    parent.deferred_dispatch = Some(Box::new(tick));
                    None
                }
                Ok(None) => {
                    let completion = Completion::new(
                        Code::Ok,
                        pop(parent),
                        std::mem::replace(&mut parent.last_options, Value::empty()),
                    );
                    self.deliver_tailcall_completion(acts, completion)
                }
                Err(completion) => {
                    // The replacement is a command of its own level: C logs
                    // its words before the command that called the procedure.
                    if completion.code == Code::Error {
                        self.log_tailcall_invocation(&words, &completion);
                    }
                    self.deliver_tailcall_completion(acts, completion)
                }
            }
        })();
        if let Some(_first) = self.original_scope_refusal(&original_scope) {
            drop(reached);
            let completion = self.abort_refused_activations_using(acts, Some(&target));
            let _ = original_scope.check(self);
            Some(self.finish_original_completion(&original_scope, completion))
        } else {
            reached
        }
    }
}
