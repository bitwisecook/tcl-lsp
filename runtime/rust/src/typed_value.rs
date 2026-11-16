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

//! Physical scalar getters and expression numeric preparation on live objects.
//!
//! `native_scalar_getter` owns primitive Int, Wide, Double and Boolean conversion.
//! It selects the actual engine, inspects the original cache before string
//! access, and applies reached cache changes even when conversion fails.
//! Its failure retains exact bytes and the update to private interpreter state.
//!
//! Expression numeric truth and arithmetic preparation use separate entry
//! points. Their conversion purposes cannot borrow primitive getter behaviour.

mod native_boolean_truth;
pub(crate) use native_boolean_truth::{expression_boolean_for_interp, native_boolean_for_interp};
#[cfg(have_tommath)]
pub(crate) use native_boolean_truth::{
    native_boolean_in, native_logical_right_for_interp, normalize_boolean_result_for_interp,
    normalize_boolean_result_in, normalize_numeric_instruction_for_interp,
    normalize_numeric_instruction_in,
};

use crate::obj::{self, TclObj};

/// Inspect the original completion-code cache, retaining its engine-specific kind.
pub(crate) fn completion_code_cache(
    value: *mut TclObj,
) -> Option<tcl_cmd_core::return_options::CompletionCodeCache> {
    obj::completion_code_cache(value)
}

/// Install the selected completion getter's original-object cache effect.
pub(crate) fn adopt_completion_code_cache(
    value: *mut TclObj,
    cache: tcl_cmd_core::return_options::CompletionCodeCache,
) -> Result<(), tcl_syntax::value::ValueError> {
    obj::adopt_completion_code_cache(value, cache)
}

/// Obtain an original completion operand's available native spelling.
/// Jim's stringless return-code representation has no native updater.
pub(crate) fn completion_code_string_bytes(
    value: *mut TclObj,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) -> Result<Vec<u8>, tcl_syntax::value::ValueError> {
    obj::check_native_liveness(value)?;
    if matches!(
        completion_code_cache(value),
        Some(tcl_cmd_core::return_options::CompletionCodeCache::Jim(_))
    ) && !obj::has_string_rep(value)
    {
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "Jim return-code string updater",
        ));
    }
    crate::dict::native_object_bytes(value, protocol)
}

/// Reach an error-neutral native getter, retaining cache effects without
/// rendering a failure or forcing a cached object's string representation.
pub(crate) fn native_scalar_probe(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    kind: tcl_syntax::scalar_getter::NativeScalarGetterKind,
) -> Result<
    Result<
        tcl_syntax::scalar_getter::NativeScalarGetterValue,
        tcl_syntax::scalar_getter::NativeScalarGetterFailure,
    >,
    tcl_syntax::value::ValueError,
> {
    native_scalar_probe_with_environment(value, dialect, kind, None)
}
pub(crate) fn native_scalar_probe_with_environment(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    kind: tcl_syntax::scalar_getter::NativeScalarGetterKind,
    environment: Option<&dyn tcl_platform::NumericEnvironment>,
) -> Result<
    Result<
        tcl_syntax::scalar_getter::NativeScalarGetterValue,
        tcl_syntax::scalar_getter::NativeScalarGetterFailure,
    >,
    tcl_syntax::value::ValueError,
> {
    native_scalar_probe_with_environment_and_currency(value, dialect, kind, environment, None)
}

/// The entered adapter checks reached String stages before later conversion
/// and cache publication. Neutral utilities explicitly supply no currency.
pub(crate) fn native_scalar_probe_with_environment_and_currency(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    kind: tcl_syntax::scalar_getter::NativeScalarGetterKind,
    environment: Option<&dyn tcl_platform::NumericEnvironment>,
    currency: Option<&crate::interp::native_operation_currency::NativeOperationCurrency>,
) -> Result<
    Result<
        tcl_syntax::scalar_getter::NativeScalarGetterValue,
        tcl_syntax::scalar_getter::NativeScalarGetterFailure,
    >,
    tcl_syntax::value::ValueError,
> {
    native_scalar_probe_with_stage_check(value, dialect, kind, environment, currency, None)
}

/// Same error-neutral scalar worker, with the original caller's independent
/// frame/storage check at reached callbacks and primary cache adoption.
pub(crate) fn native_scalar_probe_with_stage_check(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    kind: tcl_syntax::scalar_getter::NativeScalarGetterKind,
    environment: Option<&dyn tcl_platform::NumericEnvironment>,
    currency: Option<&crate::interp::native_operation_currency::NativeOperationCurrency>,
    stage_check: Option<&dyn Fn() -> Result<(), tcl_syntax::value::ValueError>>,
) -> Result<
    Result<
        tcl_syntax::scalar_getter::NativeScalarGetterValue,
        tcl_syntax::scalar_getter::NativeScalarGetterFailure,
    >,
    tcl_syntax::value::ValueError,
> {
    let original_lifetime = if stage_check.is_some() {
        obj::check_native_liveness(value)?;
        Some(obj::NativeObjectLifetime::retain(value))
    } else {
        None
    };
    let reached = scalar_with_original_lifetime(value, currency, || {
        let abi = std::cell::Cell::new(None);
        let check = || {
            scalar_object_stage_current(value, currency)
                .and_then(|()| stage_check.map_or(Ok(()), |check| check()))
                .map_err(|_| tcl_platform::NumericEnvironmentUnavailable::Target)
        };
        let checked = environment
            .filter(|_| currency.is_some())
            .map(|actual| tcl_platform::CheckedNumericEnvironment::new(actual, &check, &abi));
        let environment = checked
            .as_ref()
            .map(|actual| actual as &dyn tcl_platform::NumericEnvironment)
            .or(environment);
        scalar_probe_using(
            value,
            dialect,
            kind,
            environment,
            currency,
            &abi,
            stage_check,
        )
    });
    drop(original_lifetime);
    reached
}

// An outer environment supplies no Jim issuer. Query the retained context's
// actual Host only when the existing recipe reaches an environment stage.
fn scalar_numeric_environment_using<T>(
    value: *mut TclObj,
    protocol: tcl_syntax::scalar_getter::NativeScalarGetterProtocol,
    environment: Option<&dyn tcl_platform::NumericEnvironment>,
    currency: Option<&crate::interp::native_operation_currency::NativeOperationCurrency>,
    abi: &std::cell::Cell<Option<tcl_platform::NativeCIntegerAbi>>,
    operation: impl FnOnce(
        &dyn tcl_platform::NumericEnvironment,
    ) -> Result<T, tcl_syntax::value::ValueError>,
) -> Result<T, tcl_syntax::value::ValueError> {
    if currency.is_some() && protocol.is_jim084() {
        let context = crate::native_source::context(value)?;
        let check = || scalar_jim_stage_current(value, currency, &context);
        context.with_numeric_environment_checked(&check, abi, operation)
    } else {
        operation(environment.ok_or(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)?)
    }
}

fn scalar_probe_using(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    kind: tcl_syntax::scalar_getter::NativeScalarGetterKind,
    environment: Option<&dyn tcl_platform::NumericEnvironment>,
    currency: Option<&crate::interp::native_operation_currency::NativeOperationCurrency>,
    abi: &std::cell::Cell<Option<tcl_platform::NativeCIntegerAbi>>,
    stage_check: Option<&dyn Fn() -> Result<(), tcl_syntax::value::ValueError>>,
) -> Result<
    Result<
        tcl_syntax::scalar_getter::NativeScalarGetterValue,
        tcl_syntax::scalar_getter::NativeScalarGetterFailure,
    >,
    tcl_syntax::value::ValueError,
> {
    use tcl_syntax::value::ValueError;
    let stage = || -> Result<(), ValueError> {
        scalar_object_stage_current(value, currency)?;
        if let Some(check) = stage_check {
            obj::check_native_liveness(value)?;
            check()?;
        }
        Ok(())
    };
    stage()?;
    obj::check_native_liveness(value)?;
    let protocol = dialect
        .native_scalar_getter_protocol()
        .ok_or(ValueError::ScalarNumericInputUnavailable)?;
    if currency.is_some() && protocol.is_jim084() {
        let context = crate::native_source::context(value)?;
        scalar_jim_stage_current(value, currency, &context)?;
    }
    if obj::native_scalar_descriptor_origin(value)
        .is_some_and(|version| Some(version) != protocol.tcl_version())
    {
        return Err(ValueError::ScalarNumericInputUnavailable);
    }
    let cache = obj::native_scalar_cache(value)?;
    if let (Some(environment), Some(cache)) = (environment, cache.as_ref()) {
        let admitted = scalar_numeric_environment_using(
            value,
            protocol,
            Some(environment),
            currency,
            abi,
            |actual| {
                tcl_cmd_core::native_numeric::scalar_getter_recipe_abi(
                    protocol,
                    kind,
                    tcl_syntax::scalar_getter::NativeScalarGetterStage::Cached(cache),
                    actual,
                )
            },
        );
        stage()?;
        admitted?;
    }
    let target = if protocol.requires_target(kind, cache.as_ref()) {
        Some(if currency.is_some() && protocol.is_jim084() {
            let result = scalar_numeric_environment_using(
                value,
                protocol,
                environment,
                currency,
                abi,
                |actual| tcl_cmd_core::native_numeric::scalar_getter_target(actual),
            );
            stage()?;
            result?
        } else if let Some(environment) = environment {
            let result = tcl_cmd_core::native_numeric::scalar_getter_target(environment);
            stage()?;
            result?
        } else if protocol.is_jim084() {
            let context = crate::native_source::context(value)?;
            let result = context.scalar_getter_target();
            stage()?;
            result?
        } else {
            return Err(ValueError::ScalarNumericInputUnavailable);
        })
    } else {
        None
    };
    let cached = cache
        .as_ref()
        .map(|cache| protocol.cached_conversion(kind, cache, target))
        .transpose()
        .map_err(|_| ValueError::ScalarNumericInputUnavailable)?
        .flatten();
    let conversion = if let Some(conversion) = cached {
        conversion
    } else {
        if kind != tcl_syntax::scalar_getter::NativeScalarGetterKind::Boolean {
            if let Some(environment) = environment {
                let admitted = scalar_numeric_environment_using(
                    value,
                    protocol,
                    Some(environment),
                    currency,
                    abi,
                    |actual| {
                        tcl_cmd_core::native_numeric::scalar_getter_recipe_abi(
                            protocol,
                            kind,
                            tcl_syntax::scalar_getter::NativeScalarGetterStage::Fresh,
                            actual,
                        )
                    },
                );
                stage()?;
                admitted?;
            }
        }
        let source = stage_check
            .map(|_| obj::NativePrimaryReceipt::capture(value))
            .transpose()?;
        let original = crate::bytearray::scalar_getter_string(value, protocol);
        stage()?;
        if let Some(source) = source {
            source.check_for(value, "changed original scalar String primary")?;
        }
        let original = original?;
        if protocol.is_jim084()
            && matches!(
                kind,
                tcl_syntax::scalar_getter::NativeScalarGetterKind::Wide
                    | tcl_syntax::scalar_getter::NativeScalarGetterKind::Long
                    | tcl_syntax::scalar_getter::NativeScalarGetterKind::Double
            )
        {
            let context = crate::native_source::context(value)
                .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
            let result = if currency.is_some() {
                scalar_numeric_environment_using(
                    value,
                    protocol,
                    environment,
                    currency,
                    abi,
                    |actual| {
                        tcl_cmd_core::native_numeric::fresh_jim_conversion(
                            protocol, kind, &original, actual,
                        )
                    },
                )
            } else if let Some(environment) = environment {
                tcl_cmd_core::native_numeric::fresh_jim_conversion(
                    protocol,
                    kind,
                    &original,
                    environment,
                )
            } else {
                context.fresh_numeric_conversion(protocol, kind, &original)
            };
            stage()?;
            result?
        } else if let Some(environment) =
            environment.filter(|_| protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4))
        {
            tcl_cmd_core::native_numeric::fresh_c84_conversion(
                protocol,
                kind,
                &original,
                environment,
            )?
        } else {
            protocol
                .fresh_conversion_with_target(kind, &original, target)
                .map_err(|_| ValueError::ScalarNumericInputUnavailable)?
                .ok_or(ValueError::ScalarNumericInputUnavailable)?
        }
    };
    let (materialize, cache, outcome) = conversion.into_parts();
    if materialize {
        let source = stage_check
            .map(|_| obj::NativePrimaryReceipt::capture(value))
            .transpose()?;
        let original = crate::bytearray::scalar_getter_string(value, protocol);
        stage()?;
        if let Some(source) = source {
            source.check_for(value, "changed original scalar String primary")?;
        }
        original?;
    }
    stage()?;
    if let Some(cache) = cache {
        adopt_scalar_cache_for_original_using(value, cache, protocol, currency, stage_check)?;
    }
    Ok(outcome)
}

fn scalar_stage_current(
    currency: Option<&crate::interp::native_operation_currency::NativeOperationCurrency>,
) -> Result<(), tcl_syntax::value::ValueError> {
    if let Some(currency) = currency {
        currency
            .ensure_current_or_refuse()
            .map_err(|_| tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)?;
    }
    Ok(())
}

/// Check the actual operand header and retained operation before another stage.
/// Compatibility ValueError never replaces the full first cause retained here.
pub(crate) fn scalar_object_stage_current(
    value: *mut TclObj,
    currency: Option<&crate::interp::native_operation_currency::NativeOperationCurrency>,
) -> Result<(), tcl_syntax::value::ValueError> {
    scalar_stage_current(currency)?;
    if currency.is_none() {
        return Ok(());
    }
    if let Err(error) = obj::check_native_liveness(value) {
        if let (Some(currency), Some(refusal)) = (currency, error.native_access_refusal()) {
            currency.retain_refusal(&tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                refusal,
            ));
        }
        return Err(error);
    }
    Ok(())
}

fn adopt_scalar_cache_for_original_using(
    value: *mut TclObj,
    cache: tcl_syntax::scalar_getter::NativeScalarCache,
    protocol: tcl_syntax::scalar_getter::NativeScalarGetterProtocol,
    currency: Option<&crate::interp::native_operation_currency::NativeOperationCurrency>,
    stage_check: Option<&dyn Fn() -> Result<(), tcl_syntax::value::ValueError>>,
) -> Result<(), tcl_syntax::value::ValueError> {
    if currency.is_some() || stage_check.is_some() {
        let mut check = || {
            scalar_object_stage_current(value, currency)?;
            stage_check.map_or(Ok(()), |check| check())
        };
        let result =
            obj::adopt_native_scalar_cache_with_stage_check(value, cache, protocol, &mut check);
        check()?;
        if let Err(error) = &result {
            if let (Some(currency), Some(refusal)) = (currency, error.native_access_refusal()) {
                currency.retain_refusal(
                    &tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(refusal),
                );
            }
        }
        result
    } else {
        obj::adopt_native_scalar_cache(value, cache, protocol)
    }
}

fn scalar_with_original_lifetime<T>(
    value: *mut TclObj,
    currency: Option<&crate::interp::native_operation_currency::NativeOperationCurrency>,
    operation: impl FnOnce() -> Result<T, tcl_syntax::value::ValueError>,
) -> Result<T, tcl_syntax::value::ValueError> {
    if currency.is_none() {
        return operation();
    }
    scalar_object_stage_current(value, currency)?;
    let lease = obj::NativeObjectLifetime::retain(value);
    let result = operation();
    if let Err(error) = &result {
        retain_scalar_refusal(currency, error);
    }
    let current = scalar_object_stage_current(value, currency);
    drop(lease);
    current?;
    // After final allocation-only retirement the old header may no longer
    // exist. Only the genuine operation receipt is queried at this boundary.
    scalar_stage_current(currency)?;
    result
}

fn retain_scalar_refusal(
    currency: Option<&crate::interp::native_operation_currency::NativeOperationCurrency>,
    error: &tcl_syntax::value::ValueError,
) {
    if let (Some(currency), Some(refusal)) = (currency, error.native_access_refusal()) {
        currency.retain_refusal(&tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            refusal,
        ));
    }
}

fn scalar_jim_stage_current(
    value: *mut TclObj,
    currency: Option<&crate::interp::native_operation_currency::NativeOperationCurrency>,
    context: &crate::native_source::NativeJimObjectContext,
) -> Result<(), tcl_syntax::value::ValueError> {
    scalar_object_stage_current(value, currency)?;
    if !context.is_live() {
        let error =
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("retired Jim interpreter");
        retain_scalar_refusal(currency, &error);
        return Err(error);
    }
    Ok(())
}

/// Probe the original C Number/Bignum primitive without guest error publication.
/// Cache changes apply before completion; unsupported engines refuse before string access.
pub(crate) fn native_number_probe(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    kind: tcl_syntax::scalar_getter::NativeNumberGetterKind,
) -> Result<
    Result<tcl_syntax::number::Number, tcl_syntax::scalar_getter::NativeScalarGetterFailure>,
    tcl_syntax::value::ValueError,
> {
    native_number_probe_with_currency(value, dialect, kind, None)
}

/// Entered Number probes use the same primitive recipe and preserve reached
/// String effects before refusing later conversion or cache publication.
pub(crate) fn native_number_probe_with_currency(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    kind: tcl_syntax::scalar_getter::NativeNumberGetterKind,
    currency: Option<&crate::interp::native_operation_currency::NativeOperationCurrency>,
) -> Result<
    Result<tcl_syntax::number::Number, tcl_syntax::scalar_getter::NativeScalarGetterFailure>,
    tcl_syntax::value::ValueError,
> {
    native_number_probe_with_stage_check(value, dialect, kind, currency, None)
}

/// Same Number/Bignum worker with the captured original frame/storage guard
/// before reached String, numeric conversion and primary cache adoption.
pub(crate) fn native_number_probe_with_stage_check(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    kind: tcl_syntax::scalar_getter::NativeNumberGetterKind,
    currency: Option<&crate::interp::native_operation_currency::NativeOperationCurrency>,
    stage_check: Option<&dyn Fn() -> Result<(), tcl_syntax::value::ValueError>>,
) -> Result<
    Result<tcl_syntax::number::Number, tcl_syntax::scalar_getter::NativeScalarGetterFailure>,
    tcl_syntax::value::ValueError,
> {
    scalar_with_original_lifetime(value, currency, || {
        native_number_probe_using(value, dialect, kind, currency, stage_check)
    })
}

fn native_number_probe_using(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    kind: tcl_syntax::scalar_getter::NativeNumberGetterKind,
    currency: Option<&crate::interp::native_operation_currency::NativeOperationCurrency>,
    stage_check: Option<&dyn Fn() -> Result<(), tcl_syntax::value::ValueError>>,
) -> Result<
    Result<tcl_syntax::number::Number, tcl_syntax::scalar_getter::NativeScalarGetterFailure>,
    tcl_syntax::value::ValueError,
> {
    use tcl_syntax::value::ValueError;
    let stage = || {
        scalar_object_stage_current(value, currency)?;
        stage_check.map_or(Ok(()), |check| check())
    };
    stage()?;
    obj::check_native_liveness(value)?;
    let protocol = dialect
        .native_scalar_getter_protocol()
        .filter(|protocol| protocol.supports_number_getter())
        .ok_or(ValueError::ScalarNumericInputUnavailable)?;
    if obj::native_scalar_descriptor_origin(value)
        .is_some_and(|version| Some(version) != protocol.tcl_version())
    {
        return Err(ValueError::ScalarNumericInputUnavailable);
    }
    let current = obj::native_scalar_cache(value)?;
    let cached = if kind == tcl_syntax::scalar_getter::NativeNumberGetterKind::IncrementNumber {
        protocol.increment_number_preflight(
            current.as_ref(),
            obj::has_string_rep(value).then(|| obj::bytes_of(value).len()),
            obj::obj_type_ptr(value).is_null(),
        )
    } else {
        current
            .as_ref()
            .and_then(|cache| protocol.cached_number_conversion(kind, cache))
    };
    let conversion = if let Some(conversion) = cached {
        conversion
    } else {
        let primary = stage_check
            .map(|_| obj::NativePrimaryReceipt::capture(value))
            .transpose()?;
        let original = crate::bytearray::scalar_getter_string(value, protocol);
        stage()?;
        if let Some(primary) = primary {
            primary.check_for(value, "changed original Number String primary")?;
        }
        let original = original?;
        protocol
            .fresh_number_conversion(kind, &original)
            .ok_or(ValueError::ScalarNumericInputUnavailable)?
    };
    let (cache, outcome) = conversion.into_parts();
    stage()?;
    if let Some(cache) = cache {
        adopt_scalar_cache_for_original_using(value, cache, protocol, currency, stage_check)?;
    }
    Ok(outcome)
}

/// Execute a selected physical getter and render its reached guest failure.
/// Guest publication belongs to the consuming command adapter.
pub(crate) fn native_scalar_getter(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    kind: tcl_syntax::scalar_getter::NativeScalarGetterKind,
) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterValue, tcl_syntax::value::ValueError> {
    native_scalar_getter_with_environment(value, dialect, kind, None)
}

pub(crate) fn native_scalar_getter_with_environment(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    kind: tcl_syntax::scalar_getter::NativeScalarGetterKind,
    environment: Option<&dyn tcl_platform::NumericEnvironment>,
) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterValue, tcl_syntax::value::ValueError> {
    native_scalar_getter_with_environment_and_currency(value, dialect, kind, environment, None)
}

pub(crate) fn native_scalar_getter_with_environment_and_currency(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    kind: tcl_syntax::scalar_getter::NativeScalarGetterKind,
    environment: Option<&dyn tcl_platform::NumericEnvironment>,
    currency: Option<&crate::interp::native_operation_currency::NativeOperationCurrency>,
) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterValue, tcl_syntax::value::ValueError> {
    native_scalar_getter_with_stage_check(value, dialect, kind, environment, currency, None)
}

/// Reuse the same probe and reached failure presentation with an original
/// consumer's independent physical frame/storage guard at every stage.
pub(crate) fn native_scalar_getter_with_stage_check(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    kind: tcl_syntax::scalar_getter::NativeScalarGetterKind,
    environment: Option<&dyn tcl_platform::NumericEnvironment>,
    currency: Option<&crate::interp::native_operation_currency::NativeOperationCurrency>,
    stage_check: Option<&dyn Fn() -> Result<(), tcl_syntax::value::ValueError>>,
) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterValue, tcl_syntax::value::ValueError> {
    use tcl_syntax::value::ValueError;
    match native_scalar_probe_with_stage_check(
        value,
        dialect,
        kind,
        environment,
        currency,
        stage_check,
    )? {
        Ok(result) => Ok(result),
        Err(failure) => {
            let record = native_scalar_failure_presentation_with_stage_check(
                value,
                dialect,
                kind,
                failure,
                currency,
                stage_check,
            )?;
            scalar_object_stage_current(value, currency)?;
            stage_check.map_or(Ok(()), |check| check())?;
            Err(ValueError::NativeScalarGetter(Box::new(record)))
        }
    }
}

/// Render the already reached primitive failure without replaying its probe.
/// Constant diagnostics retain absent String storage; dependent diagnostics
/// use the selected checked original getter. Publication is caller-owned.
pub(crate) fn native_scalar_failure_presentation(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    kind: tcl_syntax::scalar_getter::NativeScalarGetterKind,
    failure: tcl_syntax::scalar_getter::NativeScalarGetterFailure,
) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterError, tcl_syntax::value::ValueError> {
    native_scalar_failure_presentation_with_currency(value, dialect, kind, failure, None)
}

pub(crate) fn native_scalar_failure_presentation_with_currency(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    kind: tcl_syntax::scalar_getter::NativeScalarGetterKind,
    failure: tcl_syntax::scalar_getter::NativeScalarGetterFailure,
    currency: Option<&crate::interp::native_operation_currency::NativeOperationCurrency>,
) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterError, tcl_syntax::value::ValueError> {
    native_scalar_failure_presentation_with_stage_check(
        value, dialect, kind, failure, currency, None,
    )
}

/// Present the same reached scalar failure while retaining its original
/// physical frame/storage guard through dependent String materialisation.
pub(crate) fn native_scalar_failure_presentation_with_stage_check(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    kind: tcl_syntax::scalar_getter::NativeScalarGetterKind,
    failure: tcl_syntax::scalar_getter::NativeScalarGetterFailure,
    currency: Option<&crate::interp::native_operation_currency::NativeOperationCurrency>,
    stage_check: Option<&dyn Fn() -> Result<(), tcl_syntax::value::ValueError>>,
) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterError, tcl_syntax::value::ValueError> {
    scalar_with_original_lifetime(value, currency, || {
        scalar_failure_presentation_using(value, dialect, kind, failure, currency, stage_check)
    })
}

fn scalar_failure_presentation_using(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    kind: tcl_syntax::scalar_getter::NativeScalarGetterKind,
    failure: tcl_syntax::scalar_getter::NativeScalarGetterFailure,
    currency: Option<&crate::interp::native_operation_currency::NativeOperationCurrency>,
    stage_check: Option<&dyn Fn() -> Result<(), tcl_syntax::value::ValueError>>,
) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterError, tcl_syntax::value::ValueError> {
    use tcl_syntax::value::ValueError;
    let stage = || {
        scalar_object_stage_current(value, currency)?;
        stage_check.map_or(Ok(()), |check| check())
    };
    stage()?;
    obj::check_native_liveness(value)?;
    let protocol = dialect
        .native_scalar_getter_protocol()
        .ok_or(ValueError::ScalarNumericInputUnavailable)?;
    let requires_string = protocol
        .failure_requires_original_string(kind, failure)
        .ok_or(ValueError::ScalarNumericInputUnavailable)?;
    let record = if requires_string {
        let primary = stage_check
            .map(|_| obj::NativePrimaryReceipt::capture(value))
            .transpose()?;
        let original = crate::bytearray::scalar_getter_string(value, protocol);
        stage()?;
        if let Some(primary) = primary {
            primary.check_for(value, "changed original scalar failure String primary")?;
        }
        let original = original?;
        protocol.failure_presentation(kind, failure, &original)
    } else {
        protocol.failure_presentation_without_original_string(kind, failure)
    };
    record.ok_or(ValueError::ScalarNumericInputUnavailable)
}

#[cfg(test)]
pub(crate) fn native_wide_int(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
) -> Result<i64, tcl_syntax::value::ValueError> {
    use tcl_syntax::scalar_getter::{NativeScalarGetterKind, NativeScalarGetterValue};
    match native_scalar_getter(value, dialect, NativeScalarGetterKind::Wide)? {
        NativeScalarGetterValue::Wide(integer) => Ok(integer),
        _ => Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable),
    }
}

#[cfg(test)]
pub(crate) fn native_double(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
) -> Result<f64, tcl_syntax::value::ValueError> {
    use tcl_syntax::scalar_getter::{NativeScalarGetterKind, NativeScalarGetterValue};
    match native_scalar_getter(value, dialect, NativeScalarGetterKind::Double)? {
        NativeScalarGetterValue::Double(double) => Ok(double),
        _ => Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable),
    }
}

pub(crate) fn native_boolean(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
) -> Result<bool, tcl_syntax::value::ValueError> {
    use tcl_syntax::scalar_getter::{NativeScalarGetterKind, NativeScalarGetterValue};
    match native_scalar_getter(value, dialect, NativeScalarGetterKind::Boolean)? {
        NativeScalarGetterValue::Boolean(boolean) => Ok(boolean.is_true()),
        _ => Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable),
    }
}

/// Obtain expression input through the selected original object's updater.
/// This physical String producer has no Guest parser or completion publisher.
fn scalar_input_bytes(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
) -> Result<Vec<u8>, tcl_syntax::raw_string::NativeValueAccessRefusal> {
    let recipe = dialect
        .native_string_materialization(Some(
            tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
        ))
        .ok_or(tcl_syntax::raw_string::NativeValueAccessRefusal::ScalarNumericInputUnavailable)?;
    crate::dict::native_object_bytes(value, recipe.protocol()).map_err(|error| {
        error
            .native_access_refusal()
            .expect("physical original String access has only operational failures")
    })
}

/// Prepare a scalar getter on its original object under the actual engine's
/// byte boundary. This never changes the retained spelling or parses source.
pub(crate) fn scalar_number(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    integer_only: bool,
) -> Result<Option<tcl_syntax::number::Number>, tcl_syntax::raw_string::NativeValueAccessRefusal> {
    scalar_number_impl(value, dialect, integer_only, || Ok(()))
}

/// Original arithmetic preparation checks its entered owner after reached
/// String access and before parsing or publishing a numeric primary.
/// Neutral mathematical callers explicitly supply no entered interpreter.
pub(crate) fn scalar_number_with_currency(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    integer_only: bool,
    currency: Option<&crate::interp::native_operation_currency::NativeOperationCurrency>,
) -> Result<Option<tcl_syntax::number::Number>, tcl_runtime_api::NativeExecutionError> {
    scalar_number_impl(value, dialect, integer_only, || {
        currency.map_or(Ok(()), |currency| currency.ensure_current_or_refuse())
    })
}

// One preparation algorithm serves the neutral utility and the entered owner.
// The entered check retains its full first Host cause; it is not an access-only
// projection and cannot panic when a reached updater refuses execution.
fn scalar_number_impl<E: From<tcl_syntax::raw_string::NativeValueAccessRefusal>>(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
    integer_only: bool,
    current: impl Fn() -> Result<(), E>,
) -> Result<Option<tcl_syntax::number::Number>, E> {
    current()?;
    obj::check_native_liveness(value).map_err(|error| {
        error
            .native_access_refusal()
            .expect("native object lifetime refusal")
    })?;
    use tcl_syntax::number::{NativeScalarNumericInputPolicy, Number, ParseFlags};
    let policy = dialect
        .scalar_numeric_input_policy()
        .ok_or(tcl_syntax::raw_string::NativeValueAccessRefusal::ScalarNumericInputUnavailable)?;
    if policy == NativeScalarNumericInputPolicy::NulTerminatedJim084 && integer_only {
        if let Some(integer) = obj::restore_jim_coerced_integer(value) {
            return Ok(Some(Number::Int(integer)));
        }
    }
    if policy == NativeScalarNumericInputPolicy::NulTerminatedJim084
        && obj::obj_type_ptr(value) == &obj::JIM_COERCED_DOUBLE_TYPE
    {
        return Ok((!integer_only).then(|| Number::Double(obj::wide_of(value) as f64)));
    }
    if obj::obj_type_ptr(value) == &obj::TCL_INT_TYPE {
        return Ok(Some(Number::Int(obj::wide_of(value))));
    }
    if obj::obj_type_ptr(value) == &obj::TCL_LONG84_TYPE
        || obj::obj_type_ptr(value) == &obj::TCL_WIDE84_TYPE
    {
        if dialect
            .native_scalar_getter_protocol()
            .and_then(|protocol| protocol.tcl_version())
            != Some(tcl_dialect::TclVersion::V8_4)
        {
            return Err(
                tcl_syntax::raw_string::NativeValueAccessRefusal::ScalarNumericInputUnavailable
                    .into(),
            );
        }
        return Ok(Some(Number::Int(obj::wide_of(value))));
    }
    if obj::obj_type_ptr(value) == &obj::TCL_DOUBLE_TYPE {
        return Ok((!integer_only).then(|| Number::Double(obj::double_of(value))));
    }
    let original = scalar_input_bytes(value, dialect);
    current()?;
    let original = original?;
    let Some(text) = core::str::from_utf8(policy.input_bytes(&original)).ok() else {
        return Ok(None);
    };
    if dialect.numbers == tcl_dialect::NumberSyntax::Tcl84
        && tcl_syntax::number::is_out_of_range_double(text, dialect.numbers)
    {
        return Ok(None);
    }
    let mut flags = ParseFlags::for_syntax(dialect.numbers);
    flags.integer_only = integer_only;
    let Some(parsed) = tcl_syntax::number::parse_whole_with(text.trim(), flags) else {
        return Ok(None);
    };
    if policy == NativeScalarNumericInputPolicy::NulTerminatedJim084 {
        use tcl_syntax::expr::mathfunc::{jim_numeric_operand, NumValue};
        let Some(number) = jim_numeric_operand::<tcl_syntax::expr::mathfunc::NoBig>(&parsed) else {
            return Ok(None);
        };
        let (number, representation) = match number {
            NumValue::Int(integer) => (
                Number::Int(integer),
                obj::NativeNumericRepresentation::Wide(integer),
            ),
            NumValue::Float(double) => (
                Number::Double(double),
                obj::NativeNumericRepresentation::Double(double),
            ),
            NumValue::Big(uninhabited) => match uninhabited {},
        };
        current()?;
        obj::adopt_native_numeric_representation(value, representation);
        current()?;
        return Ok(Some(number));
    }
    Ok(Some(parsed))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "native_number_tests.rs"]
mod native_number_tests;

#[cfg(test)]
#[path = "typed_value/native_scalar_input_tests.rs"]
mod native_scalar_input_tests;

// ---------------------------------------------------------------------------
// The C typed reads (`Tcl_Get{Int,Long,WideInt,Double,Boolean}FromObj`) over a
// live object, for the commands whose argument reads C makes through these
// calls (`interp limit`, `after`, `time`, `file mtime`, `info level`, `seek`,
// `read`, `incr`, `dict incr`, `zlib`, `string repeat`, `return -level`, ...).
// Each read writes the parsed rep back as C does and fails with C's message
// and `-errorcode` ([`TypedError`]); the native scalar getters above keep the
// selected engine's primitive conversions.
// ---------------------------------------------------------------------------

/// A failed typed read: C's interpreter result and its `-errorcode` list text.
pub(crate) struct TypedError {
    /// The message C leaves as the interpreter result.
    pub message: Vec<u8>,
    /// The `-errorcode` list C sets alongside it.
    pub code: &'static [u8],
}

impl TypedError {
    /// C's integer overflow: `integer value too large to represent`, with the
    /// message as the third word of `ARITH IOVERFLOW {…}`.
    pub(crate) fn overflow() -> Self {
        Self {
            message: tcl_syntax::expr::errors::IOVERFLOW_MESSAGE
                .as_bytes()
                .to_vec(),
            code: tcl_syntax::expr::errors::IOVERFLOW_CODE.as_bytes(),
        }
    }

    pub(crate) fn is_overflow(&self) -> bool {
        self.code == tcl_syntax::expr::errors::IOVERFLOW_CODE.as_bytes()
    }

    /// `expected <what> but got <value>` + `TCL VALUE NUMBER` — the error
    /// `TclParseNumber`'s `formaterr` raises for every unparsable spelling.
    fn expected(what: &str, obj: *mut TclObj) -> Self {
        let bytes = obj::bytes_of(obj);
        let text = String::from_utf8_lossy(&bytes);
        let mut message = format!("expected {what} but got ").into_bytes();
        message.extend_from_slice(tcl_syntax::list::describe_bad_value(&text).as_bytes());
        TypedError {
            message,
            code: b"TCL VALUE NUMBER",
        }
    }
}

/// The code `tclObj.c` sets when an integer read meets an object holding a
/// double.
pub(crate) const HELD_DOUBLE_CODE: &[u8] = b"TCL VALUE INTEGER";

/// Read `obj` as a Tcl wide integer — `Tcl_GetWideIntFromObj`.
///
/// A double-typed object is refused under C's own wording and `TCL VALUE
/// INTEGER` code (`tclObj.c`); an integer past the wide range is C's
/// [`TypedError::overflow`]; anything else unparsable is `TclParseNumber`'s
/// `TCL VALUE NUMBER`.
pub(crate) fn wide_int(obj: *mut TclObj) -> Result<i64, TypedError> {
    if let Some(error) = held_double(obj) {
        return Err(error);
    }
    read_wide_int(obj)
}

/// The refusal every integer read makes of an object already holding a
/// double, before any reparse of its spelling (`tclObj.c`).
pub(crate) fn held_double(obj: *mut TclObj) -> Option<TypedError> {
    if obj::obj_type_ptr(obj) != &obj::TCL_DOUBLE_TYPE {
        return None;
    }
    let bytes = obj::bytes_of(obj);
    let mut message = b"expected integer but got \"".to_vec();
    message.extend_from_slice(&bytes);
    message.push(b'"');
    Some(TypedError {
        message,
        code: HELD_DOUBLE_CODE,
    })
}

/// Read `obj` as C Tcl reads a 64-bit `long` (`Tcl_GetLongFromObj` on an LP64
/// host): a wide integer, or an integer past the wide range that is not
/// negative and fits 64 bits unsigned, taken modulo 2^64 as C's `(long)` of an
/// `unsigned long` takes it, so `18446744073709551615` reads -1. Past 64 bits,
/// or below the wide range, it is the overflow [`wide_int`] reports.
pub(crate) fn wide_int_modulo_unsigned(obj: *mut TclObj) -> Result<i64, TypedError> {
    match wide_int(obj) {
        Err(error) if error.is_overflow() => unsigned_past_wide(obj).ok_or(error),
        read => read,
    }
}

/// Read `obj` as C Tcl reads a `long` (`Tcl_GetLongFromObj`) on this target,
/// as a wide integer for the caller to narrow. Where `long` is 32 bits
/// (`wasm32`), an integer from `LONG_MIN` to `ULONG_MAX`, which the caller
/// truncates; where it is 64 bits, [`wide_int_modulo_unsigned`]. Anything else
/// is the overflow.
pub(crate) fn long(obj: *mut TclObj) -> Result<i64, TypedError> {
    if core::mem::size_of::<core::ffi::c_long>() == 4 {
        let wide = wide_int(obj)?;
        if (i64::from(i32::MIN)..=i64::from(u32::MAX)).contains(&wide) {
            Ok(wide)
        } else {
            Err(TypedError::overflow())
        }
    } else {
        wide_int_modulo_unsigned(obj)
    }
}

/// Read `obj` as a C `int` — `Tcl_GetIntFromObj`, the read behind every
/// `int`-typed count, level and option value: a [`long`] narrowed as C narrows
/// it ([`tcl_syntax::value::narrow_to_c_int`]), so `4294967295` reads -1 and
/// `4294967296` is the overflow.
pub(crate) fn int(obj: *mut TclObj) -> Result<i32, TypedError> {
    tcl_syntax::value::narrow_to_c_int(long(obj)?).map_err(|_| TypedError::overflow())
}

/// Check `obj` is an integer of any size — `Tcl_GetBignumFromObj`'s acceptance,
/// with [`wide_int`]'s refusal for anything else.
pub(crate) fn integer(obj: *mut TclObj) -> Result<(), TypedError> {
    match wide_int(obj) {
        Err(error) if !error.is_overflow() => Err(error),
        _ => Ok(()),
    }
}

/// Check `obj` is an `incr` operand the way `TclIncrObj` checks one: it reads
/// the value as a number first, so a non-integer number is refused as the
/// double it then holds (`TCL VALUE INTEGER`) and anything else as unparsable.
pub(crate) fn incr_operand(obj: *mut TclObj) -> Result<(), TypedError> {
    integer(obj).or_else(|error| match double(obj) {
        Ok(_) => integer(obj),
        Err(_) => Err(error),
    })
}

impl From<TypedError> for tcl_cmd_core::CmdError {
    fn from(error: TypedError) -> Self {
        let code = String::from_utf8_lossy(error.code).into_owned();
        Self::with_error_code(String::from_utf8_lossy(&error.message).into_owned(), code)
    }
}

/// The bits of `obj`'s integer spelling as an `i64`, when the value is past the
/// wide range, not negative, and fits 64 bits unsigned.
fn unsigned_past_wide(obj: *mut TclObj) -> Option<i64> {
    let bytes = obj::bytes_of(obj);
    let text = core::str::from_utf8(&bytes).ok()?;
    match tcl_syntax::number::parse_whole(text)? {
        tcl_syntax::number::Number::Big {
            negative: false,
            radix,
            digits,
        } => u64::from_str_radix(&digits, radix as u32)
            .ok()
            // C's `(long)` of an `unsigned long`: the same 64 bits.
            .map(|value| value as i64),
        _ => None,
    }
}

/// Read `obj` as a Tcl double — `Tcl_GetDoubleFromObj`. An integer or bignum
/// widens; `NaN` is a value here (the boolean context is where it is an error).
pub(crate) fn double(obj: *mut TclObj) -> Result<f64, TypedError> {
    read_double(obj).ok_or_else(|| TypedError::expected("floating-point number", obj))
}

/// The wide-integer classification over the numeric tower.
#[cfg(have_tommath)]
fn read_wide_int(obj: *mut TclObj) -> Result<i64, TypedError> {
    use crate::bignum::WideRead;
    match crate::bignum::read_wide(obj) {
        WideRead::Wide(value) => Ok(value),
        WideRead::Overflow => Err(TypedError::overflow()),
        WideRead::NotInteger | WideRead::NotNumeric => Err(TypedError::expected("integer", obj)),
    }
}

/// The double read over the numeric tower.
#[cfg(have_tommath)]
fn read_double(obj: *mut TclObj) -> Option<f64> {
    crate::bignum::read_double(obj)
}

/// Without the tower (a wasm build whose libtommath cross-compile was
/// unavailable) the same classification runs over the shared number grammar
/// alone: the typed reps still short-circuit, a beyond-wide integer is the same
/// overflow, and the write-back still happens for the two reps this build has.
#[cfg(not(have_tommath))]
fn read_wide_int(obj: *mut TclObj) -> Result<i64, TypedError> {
    use tcl_syntax::number::Number;
    if obj::obj_type_ptr(obj) == &obj::TCL_INT_TYPE {
        return Ok(obj::wide_of(obj));
    }
    match parse_string_rep(obj) {
        Some(Number::Int(value)) => {
            obj::cache_wide_rep(obj, value);
            Ok(value)
        }
        Some(Number::Big { .. }) => Err(TypedError::overflow()),
        _ => Err(TypedError::expected("integer", obj)),
    }
}

/// [`read_double`] without the numeric tower — see [`read_wide_int`]'s note.
#[cfg(not(have_tommath))]
fn read_double(obj: *mut TclObj) -> Option<f64> {
    use tcl_syntax::number::Number;
    let type_ptr = obj::obj_type_ptr(obj);
    if type_ptr == &obj::TCL_INT_TYPE {
        return Some(obj::wide_of(obj) as f64);
    }
    if type_ptr == &obj::TCL_DOUBLE_TYPE {
        return Some(obj::double_of(obj));
    }
    match parse_string_rep(obj)? {
        Number::Int(value) => {
            obj::cache_wide_rep(obj, value);
            Some(value as f64)
        }
        Number::Double(value) => {
            obj::cache_double_rep(obj, value);
            Some(value)
        }
        // A parsed `Big` is beyond `i64`, but it still has a real double
        // widening: C answers `1e23` for `99999999999999999999999`, not an
        // infinity, and only overflows past `f64`'s range. Reporting ±Inf
        // for every `Big` would be correct for the boolean read above, which
        // only wants sign and non-zeroness, but wrong here: this is what
        // `tcl_codegen_value_get_double` returns.
        Number::Big {
            negative,
            radix,
            digits,
        } => {
            let magnitude = big_magnitude_as_double(radix, &digits);
            Some(if negative { -magnitude } else { magnitude })
        }
        Number::Nan { .. } => Some(f64::NAN),
    }
}

/// Widen a beyond-wide magnitude to `f64` without a bignum — the tower-less
/// stand-in for `mp_get_double`.
///
/// A decimal magnitude goes through Rust's own parser, which is correctly
/// rounded and answers infinity exactly where the value really is past
/// `f64::MAX`. A power-of-two radix folds digit by digit: every scaling step
/// is an exact binary shift, so only the digit additions round, and each of
/// those is below the running value's ULP once the magnitude passes `2^53`.
#[cfg(not(have_tommath))]
fn big_magnitude_as_double(radix: tcl_syntax::number::Radix, digits: &str) -> f64 {
    let base = radix as u32;
    if base == 10 {
        return digits.parse::<f64>().unwrap_or(f64::INFINITY);
    }
    let scale = f64::from(base);
    digits.chars().fold(0.0_f64, |acc, digit| {
        acc * scale + f64::from(digit.to_digit(base).unwrap_or(0))
    })
}

/// The shared number grammar over an object's string rep (tower-less build).
#[cfg(not(have_tommath))]
fn parse_string_rep(obj: *mut TclObj) -> Option<tcl_syntax::number::Number> {
    let bytes = obj::bytes_of(obj);
    let text = core::str::from_utf8(&bytes).ok()?;
    tcl_syntax::number::parse_whole(text)
}
