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
    let policy = vm.numeric_policy_context();
    if policy.simulation.is_some() {
        return operation(policy);
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
