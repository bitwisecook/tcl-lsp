// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original C9 insertion preserves getter order and the selected List header.

use super::{NativeOperationScope, Op, Value, Vm};
use crate::interp::native_numeric_ops::OriginalNumericOps;
use tcl_cmd_core::CmdError;
use tcl_registry::native_list_operations_compilation::NativeListInsertionInstruction;
use tcl_syntax::value::{ValueError, ValueOps};

impl Vm {
    pub(super) fn execute_native_list_insertion(
        &mut self,
        mut operands: Vec<Value>,
        instruction: NativeListInsertionInstruction,
        opcode: Op,
    ) -> Result<Value, CmdError> {
        let version = match (instruction, opcode) {
            (NativeListInsertionInstruction::ReplaceFour, Op::LREPLACE4) => {
                tcl_dialect::TclVersion::V9_0
            }
            (NativeListInsertionInstruction::Replace, Op::LREPLACE) => {
                tcl_dialect::TclVersion::V9_1
            }
            _ => {
                return Err(ValueError::CommandProtocolUnavailable(
                    "original List insertion opcode",
                )
                .into());
            }
        };
        if self.actual_native_invocation_dialect().tcl_version != Some(version) {
            return Err(
                ValueError::CommandProtocolUnavailable("original List insertion release").into(),
            );
        }
        let scope = NativeOperationScope::enter(self)?;
        let numeric = OriginalNumericOps::in_original_scope(self, &scope)?;
        let protocol = numeric.dialect().native_string_protocol().ok_or(
            ValueError::CommandProtocolUnavailable("original List insertion issuer"),
        )?;
        let original = operands.remove(0);
        let index = operands.remove(0);
        let reached = (|| {
            let length = numeric.with_vm_for_value(&original, |vm| {
                vm.list_len(&original).map_err(CmdError::from)
            })?;
            let end = i64::try_from(length).map_err(|_| {
                ValueError::CommandProtocolUnavailable("original List insertion length")
            })?;
            let at = numeric
                .with_vm_for_value(&index, |vm| {
                    tcl_cmd_core::native_list_index::original_index_using(
                        vm, &index, end, true, None,
                    )
                })?
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original List insertion coordinate",
                ))?;
            numeric.current_for_value(&original)?;
            let at = usize::try_from(at.max(0)).unwrap_or(length).min(length);
            let result = if original.native_object_is_shared() {
                original.duplicate_native_object_checked(
                    protocol,
                    &mut || numeric.current_for_value(&original),
                    &mut |value| numeric.string(value, protocol),
                    &mut |error| numeric.retain_failure(error),
                )?
            } else {
                original
            };
            for value in &operands {
                numeric.current_for_value(value)?;
            }
            result.native_list_replace_prepared_range_checked(
                at,
                0,
                &operands,
                protocol,
                &mut || numeric.current_for_value(&result),
                &mut |value| numeric.string(value, protocol),
            )?;
            numeric.current_for_value(&result)?;
            Ok(result)
        })();
        if let Err(first) = &reached {
            numeric.retain_failure(first);
        }
        numeric.current()?;
        reached
    }
}
