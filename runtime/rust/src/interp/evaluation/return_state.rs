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

//! The pending `return` state and options, control-body completion options, and the procedure and source return boundaries.

use super::super::native_error_headers::NativeReturnOptions;
use super::super::{native_operation_currency, Code, Interp};
use crate::obj;

impl Interp {
    /// Record `return -level L -code C` state (set by the `return` command).
    pub(crate) fn set_return_state(&mut self, level: usize, code: Code) {
        self.return_level.set(level);
        self.return_code.set(code);
    }

    /// Replace byte-authored options through the same checked original-object
    /// publication owner. Generated keys grant no name or execution purpose.
    pub(crate) fn set_return_options_checked(
        &self,
        options: Vec<(Vec<u8>, Vec<u8>)>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        let scope = native_operation_currency::NativeOperationScope::enter(self)?;
        let options = options
            .into_iter()
            .map(
                |(key, value)| tcl_cmd_core::return_options::ReturnOptionPair {
                    key: obj::Owned::fresh(obj::new_string_bytes(&key)),
                    value: obj::Owned::fresh(obj::new_string_bytes(&value)),
                    key_bytes: key,
                },
            )
            .collect();
        let reached = self.set_return_option_objects_checked(options);
        scope.currency().ensure_current_or_refuse()?;
        reached
    }

    #[cfg(test)]
    pub(crate) fn set_return_options(&self, options: Vec<(Vec<u8>, Vec<u8>)>) {
        if let Err(error) = self.set_return_options_checked(options) {
            self.clone().report_cmd_error(error);
        }
    }

    /// Retain exact original options through checked Dict construction and
    /// outside-borrow retirement, under the actual entered operation.
    pub(crate) fn set_return_option_objects_checked(
        &self,
        options: Vec<tcl_cmd_core::return_options::ReturnOptionPair<obj::Owned>>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        let scope = native_operation_currency::NativeOperationScope::enter(self)?;
        let reached = NativeReturnOptions::new_checked(
            options,
            self.native_invocation_dialect()
                .native_error_objects_protocol(),
            &|| scope.currency().ensure_current_or_refuse(),
        );
        scope.currency().ensure_current_or_refuse()?;
        let options = reached?;
        self.restore_return_options_checked(options)?;
        scope.currency().ensure_current_or_refuse()?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn set_return_option_objects(
        &self,
        options: Vec<tcl_cmd_core::return_options::ReturnOptionPair<obj::Owned>>,
    ) {
        if let Err(error) = self.set_return_option_objects_checked(options) {
            self.clone().report_cmd_error(error);
        }
    }

    #[cfg(test)]
    pub(crate) fn pending_return_option_objects(
        &self,
    ) -> Vec<tcl_cmd_core::return_options::ReturnOptionPair<obj::Owned>> {
        self.return_options.borrow().pairs()
    }

    pub(in super::super) fn take_return_options(&self) -> NativeReturnOptions {
        let mut saved = std::mem::take(&mut *self.return_options.borrow_mut());
        saved.renew_incarnation();
        saved
    }

    pub(in super::super) fn restore_return_options_checked(
        &self,
        options: NativeReturnOptions,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let scope = native_operation_currency::NativeOperationScope::enter(self)?;
        let (ticket, lifetime) = self.publish_carried_return_options(options);
        scope.currency().ensure_current_or_refuse()?;
        self.check_carried_return_options_publication(&ticket, &lifetime)?;
        drop(lifetime);
        scope.currency().ensure_current_or_refuse()
    }

    // Mandatory saved-state settlement has an already attached original entry.
    // It publishes and retires even under refusal, then returns the full cause.
    pub(in super::super) fn restore_return_options_for_cleanup(
        &self,
        options: NativeReturnOptions,
        cleanup: &native_operation_currency::NativeSavedCleanupScope,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let (ticket, lifetime) = self.publish_carried_return_options(options);
        let reached = cleanup
            .check()
            .and_then(|()| self.check_carried_return_options_publication(&ticket, &lifetime));
        drop(lifetime);
        cleanup.check()?;
        reached
    }

    /// Start a new completion boundary, refusing before later work if the old
    /// owning fields retain a Host cause while retiring outside state borrows.
    pub(crate) fn clear_return_options_checked(
        &self,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        self.restore_return_options_checked(NativeReturnOptions::default())
    }

    #[cfg(test)]
    pub(crate) fn clear_return_options(&self) {
        let _ = self.clear_return_options_checked();
    }

    /// Enter a control body using the shared completion-option policy.
    pub(crate) fn begin_control_options_checked(
        &self,
        policy: tcl_runtime_api::completion_options::ControlOptionPolicy,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let scope = native_operation_currency::NativeOperationScope::enter(self)?;
        if policy.begins_fresh() {
            self.clear_return_options_checked()?;
        }
        scope.currency().ensure_current_or_refuse()
    }

    /// Settle a successful control completion before publishing later effects.
    pub(crate) fn settle_control_options_checked(
        &self,
        policy: tcl_runtime_api::completion_options::ControlOptionPolicy,
        code: Code,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let scope = native_operation_currency::NativeOperationScope::enter(self)?;
        if code == Code::Ok && policy.settles_success() {
            self.clear_return_options_checked()?;
        }
        scope.currency().ensure_current_or_refuse()
    }

    /// The pending `return` `-code`/`-level` (the options a body that completed
    /// via `return` would propagate) — for `catch`/`try`'s options dict and TIP
    /// 329 `-during` chaining.
    pub(crate) fn pending_return_code(&self) -> Code {
        self.return_code.get()
    }

    pub(crate) fn pending_return_level(&self) -> usize {
        self.return_level.get()
    }

    /// Apply a procedure/source **return boundary** to a body completion code
    /// (`TclUpdateReturnInfo`): a `Code::Return` decrements the pending
    /// `-level`; when it reaches 0 the boundary completes with the pending
    /// `-code` (so `return` → Ok, `return -code error` → Error). Other codes
    /// pass through.
    pub(in super::super) fn settle_return(&mut self, code: Code) -> Code {
        if code != Code::Return {
            return code;
        }
        self.return_level
            .set(self.return_level.get().saturating_sub(1));
        if self.return_level.get() == 0 {
            let c = self.return_code.get();
            self.return_code.set(Code::Ok);
            c
        } else {
            Code::Return
        }
    }

    /// Settle the selected native file boundary independently of procedure return.
    pub(super) fn settle_source_return(&mut self, code: Code) -> Code {
        use tcl_registry::completion::CompletionCode;
        use tcl_registry::completion_route::{
            InvocationCompletionRoute as Route, ReturnCompletionRoute,
        };
        let route = if code == Code::Return {
            Route::Return(ReturnCompletionRoute {
                eventual_code: CompletionCode::from_int(self.return_code.get().as_int() as i32),
                remaining_level: self.return_level.get() as u64,
            })
        } else {
            Route::Tcl(CompletionCode::from_int(code.as_int() as i32))
        };
        match tcl_registry::source_file::completion_route(self.native_invocation_dialect(), route) {
            Route::Return(pending) => {
                self.return_level.set(pending.remaining_level as usize);
                self.return_code
                    .set(Code::from_int(pending.eventual_code.as_int() as i32));
                Code::Return
            }
            Route::Tcl(code) => {
                if matches!(route, Route::Return(_)) {
                    self.return_level.set(0);
                    self.return_code.set(Code::Ok);
                }
                Code::from_int(code.as_int() as i32)
            }
            _ => self.set_error(b"selected native source completion is unavailable"),
        }
    }
}
