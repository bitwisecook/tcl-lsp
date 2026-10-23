// SPDX-License-Identifier: AGPL-3.0-or-later
//! Membership borrows original List and String stages without numeric conversion.

use super::{TclError, Value, with_original_numeric_command};
use crate::interp::{Vm, native_numeric_ops::OriginalNumericOps};
use std::cell::RefCell;
use tcl_syntax::value::ValueError;

pub(super) fn in_list(vm: &mut Vm, needle: &Value, list: &Value) -> Result<bool, TclError> {
    with_original_numeric_command(vm, TclError::from_execution_failure, |vm, context| {
        let selected = vm.native_invocation_dialect();
        let model = selected
            .characters
            .ok_or(tcl_syntax::raw_string::NativeValueAccessRefusal::CharacterModelUnavailable)?;
        let needle = needle.native_lifetime_lease();
        let list = list.native_lifetime_lease();
        let reached = if context.authored.is_some() {
            // An inherited operation is currency only. The authored grammar
            // continues through the configured parser and pure String route.
            let vm = RefCell::new(&mut *vm);
            let current = |actual: &Value| -> Result<(), TclError> {
                context
                    .current(&mut vm.borrow_mut())
                    .map_err(TclError::from_execution_failure)?;
                if let Err(refusal) = actual.original_native_header_currentness() {
                    let first = vm.borrow_mut().retain_tcl_host_failure(
                        crate::error::TclHostFailure::ValueAccess(refusal),
                    );
                    return Err(TclError::from_execution_failure(first));
                }
                Ok(())
            };
            let string = |actual: &Value| -> Result<std::rc::Rc<[u8]>, TclError> {
                current(actual)?;
                let reached = actual.string_bytes();
                current(actual)?;
                Ok(reached)
            };
            let result = (|| {
                current(list.value())?;
                current(needle.value())?;
                let items = list.value().native_list_elements_with_strings(
                    selected.lexer_grammar.list_parse,
                    selected.lexer_grammar.escapes,
                    &mut || current(list.value()),
                    &mut |actual| string(actual),
                )?;
                for item in &*items {
                    current(list.value())?;
                    items.check_generation()?;
                    let left = string(needle.value())?;
                    let right = string(item)?;
                    current(list.value())?;
                    items.check_generation()?;
                    if left == right {
                        return Ok(true);
                    }
                }
                Ok(false)
            })();
            current(list.value())?;
            result
        } else {
            let scope = context
                .original
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original expression List entry",
                ))?;
            let original = OriginalNumericOps::in_original_scope_native(vm, scope)
                .map_err(TclError::from_execution_failure)?;
            let actual = original.dialect();
            let protocol =
                selected
                    .native_string_protocol()
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "expression List String protocol",
                    ))?;
            let native_parse = if protocol.is_jim084() {
                tcl_dialect::ListParse::Lenient
            } else {
                tcl_dialect::ListParse::Strict
            };
            if actual.native_string_protocol() != Some(protocol)
                || actual.characters != Some(model)
                || selected.lexer_grammar.list_parse != native_parse
                || selected.lexer_grammar.escapes != protocol.escape_syntax()
            {
                return Err(ValueError::CommandProtocolUnavailable(
                    "foreign expression List or character grammar",
                )
                .into());
            }
            let result = (|| {
                let items = original.list_for_expression(list.value(), protocol)?;
                for item in &*items {
                    super::OriginalArithmeticStages::current_for_value(&original, list.value())?;
                    items.check_generation()?;
                    let left = super::OriginalArithmeticStages::string(&original, needle.value())?;
                    let right = super::OriginalArithmeticStages::string(&original, item)?;
                    super::OriginalArithmeticStages::current_for_value(&original, list.value())?;
                    items.check_generation()?;
                    if left == right {
                        return Ok(true);
                    }
                }
                Ok(false)
            })();
            super::OriginalArithmeticStages::current_for_value(&original, list.value())?;
            result
        };
        // Allocation-only operands grant no Native role; their final retirement
        // stays inside the original command boundary before it checks currency.
        drop(list);
        drop(needle);
        context
            .current(vm)
            .map_err(TclError::from_execution_failure)?;
        reached
    })
}
