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

//! Raising and reporting errors: error codes, command and background errors, `errorInfo`, Jim's error stack, snapshots and error episodes.

use super::{new_string, Interp};
use crate::obj;
use crate::obj::TclObj;

mod error_info;
mod error_state;
mod jim_error_stack;
mod raising;

impl Interp {
    /// Set an authored background-handler prefix in a fixture.
    #[cfg(test)]
    pub(crate) fn set_bgerror_handler(&self, prefix: &[u8]) {
        self.invalidate_interpreter_policy();
        let replaced = self
            .bgerror
            .borrow_mut()
            .replace(obj::Owned::fresh(new_string(prefix)));
        drop(replaced);
    }

    /// Store or return the original background handler, retaining its object identity.
    pub(crate) fn bgerror_apply(
        &mut self,
        prefix: Option<*mut TclObj>,
    ) -> Result<obj::Owned, tcl_cmd_core::CmdError> {
        if let Some(prefix) = prefix {
            let valid = match tcl_syntax::value::ValueOps::list_len(self, &prefix) {
                Ok(length) => length > 0,
                Err(error) if error.native_access_refusal().is_some() => return Err(error.into()),
                Err(_) => false,
            };
            if !valid {
                let mut detail = tcl_cmd_core::CmdError::new_bytes(
                    b"cmdPrefix must be list of length >= 1".to_vec(),
                )
                .into_byte_details();
                if let Some(code) = self
                    .native_invocation_dialect()
                    .native_event_protocol()
                    .and_then(|p| p.background_prefix_format_code())
                {
                    detail.error_code = tcl_cmd_core::CmdErrorCodeUpdate::Set(code.to_vec());
                }
                return Err(tcl_cmd_core::CmdError::from_byte_details(detail));
            }
            self.invalidate_interpreter_policy();
            let replaced = self
                .bgerror
                .borrow_mut()
                .replace(obj::Owned::retain(prefix));
            drop(replaced);
        }
        Ok(self
            .bgerror
            .borrow()
            .as_ref()
            .cloned()
            .unwrap_or_else(|| obj::Owned::fresh(new_string(b"::tcl::Bgerror"))))
    }

    /// Queue a background error (a destructor failing during implicit teardown,
    /// etc.) for later processing by `update` — Tcl defers it to the event loop
    /// rather than firing at the error site.
    pub(crate) fn report_bg_error(&mut self, msg: &[u8], options: &[u8]) {
        self.bg_queue.borrow_mut().push((
            obj::Owned::fresh(new_string(msg)),
            obj::Owned::fresh(new_string(options)),
        ));
    }

    pub(crate) fn report_bg_error_original(&mut self, message: obj::Owned, options: obj::Owned) {
        self.bg_queue.borrow_mut().push((message, options));
    }

    /// Dispatch retained background-error prefixes and preserve the caller's result.
    pub(crate) fn process_bg_errors(&mut self) {
        while !self.host_refusal_pending() && !self.bg_queue.borrow().is_empty() {
            let batch = {
                let mut pending = self.bg_queue.borrow_mut();
                std::mem::take(&mut *pending)
            };
            for (message, options) in batch {
                let jim = self
                    .native_invocation_dialect()
                    .native_event_protocol()
                    .is_some_and(|p| p.after_options_exact());
                let configured = if jim {
                    None
                } else {
                    self.bgerror.borrow().clone()
                };
                let has_configured = configured.is_some();
                let handler =
                    configured.unwrap_or_else(|| obj::Owned::fresh(new_string(b"bgerror")));
                let saved = obj::Owned::retain(self.get_obj_result());
                let mut argv =
                    match tcl_syntax::value::ValueOps::list_elements(self, &handler.as_ptr()) {
                        Ok(words) => words
                            .into_iter()
                            .map(obj::Owned::retain)
                            .collect::<Vec<_>>(),
                        Err(error) => {
                            self.report_cmd_error(error.into());
                            if self.host_refusal_pending() {
                                return;
                            }
                            continue;
                        }
                    };
                if let Some(head) = argv.first() {
                    let namespace = if jim {
                        self.current_ns.get()
                    } else {
                        crate::namespace::GLOBAL
                    };
                    match self.resolve_original_command_at(namespace, head.as_ptr()) {
                        Ok(Some(_)) => {
                            argv.push(message);
                            if has_configured {
                                argv.push(options);
                            }
                            let words = argv.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
                            if jim {
                                let _ = self.dispatch(&words);
                            } else {
                                self.with_event_global_frame(|interp| {
                                    let _ = interp.dispatch(&words);
                                });
                            }
                        }
                        Ok(None) => {}
                        Err(error) => {
                            self.report_cmd_error(error.into());
                            if self.host_refusal_pending() {
                                return;
                            }
                            continue;
                        }
                    }
                }
                if self.host_refusal_pending() {
                    return;
                }
                unsafe {
                    self.set_obj_result(saved.as_ptr());
                }
            }
        }
    }
}
