// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original arithmetic borrows the existing operation owner without borrowing its policy.

use super::{NumericContext, TclError, Value};
use crate::interp::{Vm, native_numeric_ops::OriginalNumericOps};
use std::rc::Rc;
use tcl_syntax::{
    number::Number,
    scalar_getter::{NativeScalarGetterFailure, NativeScalarGetterKind, NativeScalarGetterValue},
    value::ValueError,
};

pub(crate) trait OriginalArithmeticStages {
    fn current(&self) -> Result<(), TclError>;
    fn current_for_value(&self, value: &Value) -> Result<(), TclError>;
    fn string(&self, value: &Value) -> Result<Rc<[u8]>, TclError>;
    fn prepare_integer84(&self, value: &Value) -> Result<bool, TclError>;
    fn scalar_probe(
        &self,
        value: &Value,
        kind: NativeScalarGetterKind,
    ) -> Result<Result<NativeScalarGetterValue, NativeScalarGetterFailure>, TclError>;
    fn integer(
        &self,
        value: &Value,
        input: tcl_syntax::number::NativeScalarNumericInputPolicy,
        syntax: tcl_dialect::NumberSyntax,
    ) -> Result<i64, TclError>;
    fn boolean(
        &self,
        value: &Value,
        input: tcl_syntax::number::NativeScalarNumericInputPolicy,
        syntax: tcl_dialect::NumberSyntax,
    ) -> Result<bool, TclError>;
    fn cache(&self, value: &Value, number: Number) -> Result<(), TclError>;
    fn store_long84(&self, value: &Value, integer: i64) -> Result<(), TclError>;
}

impl NumericContext<'_> {
    pub(crate) fn current(self) -> Result<(), TclError> {
        self.original
            .map_or(Ok(()), OriginalArithmeticStages::current)
    }

    pub(crate) fn current_for_value(self, value: &Value) -> Result<(), TclError> {
        match self.original {
            Some(original) => original.current_for_value(value),
            None => value.check_native_header().map_err(Into::into),
        }
    }

    pub(crate) fn string_for_value(self, value: &Value) -> Result<Rc<[u8]>, TclError> {
        match self.original {
            Some(original) => original.string(value),
            None => Ok(value.string_bytes()),
        }
    }

    pub(crate) fn native_string_for_value(self, value: &Value) -> Result<Rc<[u8]>, TclError> {
        if let Some(original) = self.original {
            return original.string(value);
        }
        let protocol = self
            .dialect
            .native_string_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        value.native_string_bytes(protocol).map_err(|error| {
            ValueError::NativeStringAccess(
                tcl_syntax::raw_string::NativeStringAccessError::Unavailable(error),
            )
            .into()
        })
    }

    pub(crate) fn prepare_integer84(self, value: &Value) -> Result<bool, TclError> {
        match self.original {
            Some(original) => original.prepare_integer84(value),
            None => value
                .prepare_native_expression_integer84_with_environment(
                    self.dialect,
                    self.environment,
                )
                .map_err(Into::into),
        }
    }

    pub(crate) fn scalar_probe(
        self,
        value: &Value,
        kind: NativeScalarGetterKind,
    ) -> Result<Result<NativeScalarGetterValue, NativeScalarGetterFailure>, TclError> {
        match self.original {
            Some(original) => original.scalar_probe(value, kind),
            None => value
                .native_scalar_probe_with_environment(self.dialect, kind, self.environment)
                .map_err(Into::into),
        }
    }

    pub(crate) fn integer(self, value: &Value) -> Result<i64, TclError> {
        let input = self
            .scalar_numeric_input_policy()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        match self.original {
            Some(original) => original.integer(value, input, self.numbers),
            None => value.native_int(input, self.numbers).map_err(Into::into),
        }
    }

    pub(crate) fn boolean(self, value: &Value) -> Result<bool, TclError> {
        let input = self
            .scalar_numeric_input_policy()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        match self.original {
            Some(original) => original.boolean(value, input, self.numbers),
            None => value.native_bool(input, self.numbers).map_err(Into::into),
        }
    }

    pub(crate) fn cache(self, value: &Value, number: Number) -> Result<(), TclError> {
        if let Some(original) = self.original {
            return original.cache(value, number);
        }
        match number {
            Number::Int(integer) => value.cache_integer_representation(integer),
            Number::Double(double) => value.cache_double_representation(double),
            _ => return Err(ValueError::ScalarNumericInputUnavailable.into()),
        }
        Ok(())
    }

    pub(crate) fn store_long84(self, value: &Value, integer: i64) -> Result<(), TclError> {
        match self.original {
            Some(original) => original.store_long84(value, integer),
            None => value
                .store_native_expression_long84(integer, self.dialect)
                .map_err(Into::into),
        }
    }
}

pub(crate) fn with_original_numeric_context<T>(
    vm: &mut Vm,
    operation: impl FnOnce(NumericContext<'_>) -> Result<T, TclError>,
) -> Result<T, TclError> {
    let inherited =
        crate::interp::native_operation_currency::NativeOperationScope::current_entered(vm)
            .map_err(TclError::from_execution_failure)?;
    let policy = vm.numeric_policy_context();
    if policy.simulation.is_some() {
        let reached = operation(policy);
        if let Err(first) = inherited
            .as_ref()
            .map_or(Ok(()), |scope| scope.check_native(vm))
        {
            drop(reached);
            let _ = inherited
                .as_ref()
                .map_or(Ok(()), |scope| scope.check_native(vm));
            return Err(TclError::from_execution_failure(first));
        }
        return reached;
    }
    let scope = crate::interp::native_operation_currency::NativeOperationScope::enter_native(vm)
        .map_err(TclError::from_execution_failure)?;
    let original = OriginalNumericOps::in_original_scope_native(vm, &scope)
        .map_err(TclError::from_execution_failure)?;
    OriginalArithmeticStages::current(&original)?;
    let actual = original.dialect();
    if policy.dialect.arithmetic().is_none()
        || policy.dialect.arithmetic() != actual.arithmetic()
        || policy.dialect.native_scalar_getter_protocol().is_none()
        || policy.dialect.native_scalar_getter_protocol() != actual.native_scalar_getter_protocol()
        || policy.dialect.native_string_protocol().is_none()
        || policy.dialect.native_string_protocol() != actual.native_string_protocol()
    {
        return Err(ValueError::ScalarNumericInputUnavailable.into());
    }
    let context = NumericContext {
        original: Some(&original),
        ..policy
    };
    let reached = operation(context);
    OriginalArithmeticStages::current(&original)?;
    reached
}

/// Authored numeric policy and actual execution currency are separate axes.
#[derive(Clone, Copy)]
pub(crate) struct OriginalNumericCommandContext<'a> {
    pub(crate) original: Option<&'a crate::interp::native_operation_currency::NativeOperationScope>,
    pub(crate) authored: Option<NumericContext<'static>>,
}

impl OriginalNumericCommandContext<'_> {
    pub(crate) fn current(self, vm: &mut Vm) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        self.original.map_or(Ok(()), |scope| scope.check_native(vm))
    }
}

pub(crate) fn with_original_numeric_command<T, E>(
    vm: &mut Vm,
    failure: fn(tcl_runtime_api::NativeExecutionError) -> E,
    operation: impl FnOnce(&mut Vm, OriginalNumericCommandContext<'_>) -> Result<T, E>,
) -> Result<T, E> {
    let inherited =
        crate::interp::native_operation_currency::NativeOperationScope::current_entered(vm)
            .map_err(failure)?;
    let authored = vm.authored_numeric_context();
    let original = if authored.is_some() {
        inherited
    } else {
        Some(
            crate::interp::native_operation_currency::NativeOperationScope::enter_native(vm)
                .map_err(failure)?,
        )
    };
    let context = OriginalNumericCommandContext {
        original: original.as_ref(),
        authored,
    };
    let reached = operation(vm, context);
    if let Err(first) = context.current(vm) {
        drop(reached);
        let _ = context.current(vm);
        return Err(failure(first));
    }
    reached
}

pub(crate) fn original_numeric_command_ops<T>(
    vm: &mut Vm,
    context: OriginalNumericCommandContext<'_>,
    purpose: tcl_registry::native_numeric_error::NativeNumericCommandPurpose,
    operation: impl FnOnce(
        &OriginalNumericOps<'_>,
        tcl_registry::native_numeric_error::NativeNumericCommandProtocol,
    ) -> Result<T, tcl_cmd_core::CmdError>,
) -> Result<T, tcl_cmd_core::CmdError> {
    use tcl_syntax::value::ValueError;
    context.current(vm)?;
    if context.authored.is_some() {
        return Err(ValueError::CommandProtocolUnavailable(
            "authored route has no primitive command getter",
        )
        .into());
    }
    let scope = context
        .original
        .ok_or(ValueError::CommandProtocolUnavailable(
            "original numeric command entry",
        ))?;
    let selected = vm
        .native_invocation_dialect()
        .native_numeric_command_protocol(purpose)
        .ok_or(ValueError::CommandProtocolUnavailable(
            "numeric command argument purpose",
        ))?;
    let selected_scalar = vm
        .native_invocation_dialect()
        .native_scalar_getter_protocol();
    let selected_string = vm.native_invocation_dialect().native_string_protocol();
    let reached = {
        let original = OriginalNumericOps::in_original_scope(vm, scope)?;
        if original.dialect().native_numeric_command_protocol(purpose) != Some(selected)
            || original.dialect().native_scalar_getter_protocol() != selected_scalar
            || selected_scalar.is_none()
            || original.dialect().native_string_protocol() != selected_string
            || selected_string.is_none()
        {
            return Err(
                ValueError::CommandProtocolUnavailable("foreign numeric command purpose").into(),
            );
        }
        let reached = operation(&original, selected);
        original.current()?;
        reached
    };
    context.current(vm)?;
    reached
}

pub(crate) fn numeric_command_guest_error(
    vm: &mut Vm,
    context: OriginalNumericCommandContext<'_>,
    error: TclError,
) -> tcl_cmd_core::CmdError {
    if let Err(first) = context.current(vm) {
        return tcl_cmd_core::CmdError::from_execution_refusal(first);
    }
    match error {
        TclError::Host(first) => {
            tcl_cmd_core::CmdError::from_execution_refusal(vm.retain_tcl_host_failure(first))
        }
        error @ TclError::Guest(_) => {
            // Only resident authored metadata is projected; no message is parsed
            // and absent or withdrawn metadata never becomes a default code.
            let fields = error.message_bytes().and_then(|message| {
                error.error_code_bytes().and_then(|code| {
                    code.map(|code| (message, code)).ok_or(crate::error::TclHostFailure::ValueAccess(
                        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("authored numeric Guest error metadata"),
                    ))
                })
            });
            match fields {
                Ok((message, code)) => {
                    tcl_cmd_core::CmdError::with_error_code_bytes(message.to_vec(), code.to_vec())
                }
                Err(first) => tcl_cmd_core::CmdError::from_execution_refusal(
                    vm.retain_tcl_host_failure(first),
                ),
            }
        }
    }
}
