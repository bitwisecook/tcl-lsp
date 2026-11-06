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

//! Checked original operand String ingress before independently selected name purposes.

use super::{
    Vm, native_numeric_ops::OriginalNumericOps, native_operation_currency::NativeOperationScope,
};
use crate::{Value, error::TclHostFailure};
use std::rc::Rc;
use tcl_cmd_core::CmdError;
use tcl_syntax::native_string::NativeStringUnavailable;
use tcl_syntax::raw_string::NativeStringAccessError;

impl Vm {
    /// Read the original operand through its entered context and String owner.
    /// Resident String bytes need no formatter or new materialisation recipe.
    /// The caller retains its own name purpose and command-wide operation scope.
    pub(crate) fn native_name_operand_bytes_checked(
        &mut self,
        value: &Value,
    ) -> Result<Rc<[u8]>, CmdError> {
        self.native_name_operand_bytes_checked_using(value, &|_| Ok(()))
    }

    pub(crate) fn native_name_operand_bytes_checked_using(
        &mut self,
        value: &Value,
        check: &impl Fn(&mut Vm) -> Result<(), CmdError>,
    ) -> Result<Rc<[u8]>, CmdError> {
        check(self)?;
        let scope = NativeOperationScope::enter(self)?;
        let original = value.native_lifetime_lease();
        let reached = (|| {
            let operation = OriginalNumericOps::in_original_scope(self, &scope)?;
            operation.current_for_value(original.value())?;
            let reached = if let Some(bytes) = original.value().resident_string_bytes() {
                Ok(bytes)
            } else {
                let protocol = operation.dialect().native_string_protocol().ok_or(
                    NativeStringAccessError::Unavailable(
                        NativeStringUnavailable::ProtocolUnavailable,
                    ),
                )?;
                operation.string_with_original_check(original.value(), protocol, check)
            };
            operation.current_for_value(original.value())?;
            reached
        })();
        if let Err(error) = &reached
            && let Some(cause) = error.native_execution_refusal()
        {
            self.retain_tcl_host_failure(TclHostFailure::Execution(cause.clone()));
        }
        drop(original);
        scope.check(self)?;
        check(self)?;
        reached
    }
}

#[cfg(test)]
mod tests;
