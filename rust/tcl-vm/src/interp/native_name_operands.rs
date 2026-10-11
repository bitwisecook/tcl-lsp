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
                operation.string(original.value(), protocol)
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
        reached
    }
}

#[cfg(test)]
mod tests;
