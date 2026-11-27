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

//! `tailcall`: scheduling the replacement command and draining it after the frame pops.

use super::super::{Code, Interp};
use super::CommandDispatchSelection;
use crate::namespace::{CommandBinding, NsId};
use crate::obj::TclObj;

impl Interp {
    pub(crate) fn schedule_tailcall(&mut self, words: &[*mut TclObj]) -> Code {
        if !self.in_proc() {
            return self.error(b"tailcall can only be called from a proc, lambda or method");
        }
        let Some(protocol) = self.native_invocation_dialect().tailcall_protocol() else {
            return self.error(b"native tailcall protocol is not selected");
        };
        if protocol == tcl_native_model::NativeTailcallProtocol::Tcl {
            // The generic handler retires the previous request before producing
            // a namespace operand; an empty invocation creates no replacement.
            if !self.replace_pending_tailcall(None) {
                return self.error(b"tailcall can only be called from a proc, lambda or method");
            }
            if words.is_empty() {
                self.set_result_bytes(b"");
                return Code::Return;
            }
            let namespace = match tcl_cmd_core::namespace::current_original(self) {
                Ok(original) => crate::obj::Owned::fresh(original),
                Err(error) => return self.report_cmd_error(error),
            };
            let Some(strings) = self.invocation_string_protocol() else {
                return self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original tailcall List issuer",
                    )
                    .into(),
                );
            };
            let mut members = Vec::with_capacity(words.len() + 1);
            members.push(namespace.as_ptr());
            members.extend_from_slice(words);
            let original =
                crate::obj::Owned::fresh(crate::list::new_list_obj_native(&members, strings));
            return self.schedule_original_tailcall_list(original);
        }
        self.schedule_jim_tailcall(words)
    }

    pub(crate) fn schedule_original_tailcall_list(&mut self, original: crate::obj::Owned) -> Code {
        if !self.in_proc() {
            return self.error(b"tailcall can only be called from a proc, lambda or method");
        }
        // Compiled operands already exist. Retire the old List before examining
        // the new one, and release a namespace-only cancellation at this opcode.
        if !self.replace_pending_tailcall(None) {
            return self.error(b"tailcall can only be called from a proc, lambda or method");
        }
        let Some(protocol) = self.invocation_string_protocol() else {
            return self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original tailcall List issuer",
                )
                .into(),
            );
        };
        let members = match crate::list::list_elements_native_checked(original.as_ptr(), protocol) {
            Ok(members) => members,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        if members.len() <= 1 {
            drop(original);
            self.set_result_bytes(b"");
            return Code::Return;
        }
        let request = crate::frame::PendingTailcall::OriginalNamespaceList(original);
        if !self.replace_pending_tailcall(Some(request)) {
            return self.error(b"tailcall can only be called from a proc, lambda or method");
        }
        self.set_result_bytes(b"");
        Code::Return
    }

    pub(in super::super) fn replace_pending_tailcall(
        &mut self,
        request: Option<crate::frame::PendingTailcall>,
    ) -> bool {
        let retired = { self.frames.borrow_mut().replace_tailcall(request) };
        match retired {
            Ok(previous) => {
                drop(previous);
                true
            }
            Err(rejected) => {
                drop(rejected);
                false
            }
        }
    }

    pub(in super::super) fn dispatch_tailcall(
        &mut self,
        request: crate::frame::PendingTailcall,
    ) -> Code {
        match request {
            crate::frame::PendingTailcall::OriginalNamespaceList(original) => {
                self.invoke_original_namespace_list(original.as_ptr())
            }
            crate::frame::PendingTailcall::Jim(request) => {
                self.invoke_selected_jim_tailcall(request)
            }
        }
    }

    pub(crate) fn invoke_original_namespace_list(&mut self, original: *mut TclObj) -> Code {
        let Some(protocol) = self.invocation_string_protocol() else {
            return self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original coroutine List issuer",
                )
                .into(),
            );
        };
        let members = match crate::list::list_elements_native_checked(original, protocol) {
            Ok(members) => members,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let Some((namespace, words)) = members.split_first() else {
            return self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original coroutine namespace operand",
                )
                .into(),
            );
        };
        if words.is_empty() {
            self.set_result_bytes(b"");
            return Code::Ok;
        }
        let lookup = match self.native_namespace_object_lookup(*namespace) {
            Ok(Some(namespace)) => namespace,
            Ok(None) => {
                return crate::cmd_namespace::ns_operation_not_found(
                    self,
                    *namespace,
                    tcl_syntax::naming::NativeNamespaceLookupOperation::ObjectLookup,
                );
            }
            Err(error) => return self.report_cmd_error(error.into()),
        };
        self.dispatch_original_words_at(words, lookup)
    }

    fn dispatch_original_words_at(&mut self, words: &[*mut TclObj], lookup: NsId) -> Code {
        let Some(head) = words.first() else {
            self.set_result_bytes(b"");
            return Code::Ok;
        };
        if let Err(code) = self.reset_native_ensemble_rewrite(
            tcl_native_model::native_ensemble_rewrite::EnsembleRewriteResetEvent::BeforeOrdinaryLookup,
        ) {
            return code;
        }
        let selected = match self.resolve_original_command_at(lookup, *head) {
            Ok(selected) => selected,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let selection = match selected {
            Some((command, Some(generation))) => {
                let fqn = self
                    .namespaces
                    .borrow()
                    .native_command_at_node(generation)
                    .map(|(_, fqn)| fqn);
                let Some(fqn) = fqn else {
                    return self.report_cmd_error(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "tailcall command allocation",
                        )
                        .into(),
                    );
                };
                CommandDispatchSelection::Bound(
                    fqn,
                    CommandBinding {
                        generation,
                        command,
                        jim_table_key: None,
                    },
                )
            }
            Some((command, None)) => return self.invoke_bound(command, None, words),
            None => CommandDispatchSelection::Missing {
                lookup,
                caller: self.current_ns.get(),
                origin: tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
            },
        };
        self.dispatch_prebound_inner(words, selection, true, None)
    }

    pub(super) fn drain_tailcalls(&mut self, depth: u32, mut code: Code) -> Code {
        loop {
            let request = {
                let mut requests = self.deferred_tailcalls.borrow_mut();
                if requests.last().is_some_and(|(owner, _)| *owner == depth) {
                    requests.pop().map(|(_, request)| request)
                } else {
                    None
                }
            };
            let Some(request) = request else {
                return code;
            };
            if code == Code::Ok {
                code = self.dispatch_tailcall(request);
            }
        }
    }
}
