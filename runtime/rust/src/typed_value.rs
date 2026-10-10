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
pub(crate) use native_boolean_truth::{
    expression_boolean_for_interp, native_boolean_for_interp, native_boolean_in,
    native_logical_right_for_interp, normalize_boolean_result_for_interp,
    normalize_boolean_result_in,
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
    use tcl_syntax::value::ValueError;
    obj::check_native_liveness(value)?;
    let protocol = dialect
        .native_scalar_getter_protocol()
        .ok_or(ValueError::ScalarNumericInputUnavailable)?;
    if obj::native_word_boolean_version(value)
        .is_some_and(|version| Some(version) != protocol.tcl_version())
    {
        return Err(ValueError::ScalarNumericInputUnavailable);
    }
    let cache = obj::native_scalar_cache(value)?;
    let target = if protocol.requires_target(kind, cache.as_ref()) {
        Some(if let Some(environment) = environment {
            tcl_cmd_core::native_numeric::scalar_getter_target(environment)?
        } else if protocol.is_jim084() {
            crate::native_source::context(value)?.scalar_getter_target()?
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
        let original = crate::bytearray::scalar_getter_string(value, protocol)?;
        if protocol.is_jim084()
            && matches!(
                kind,
                tcl_syntax::scalar_getter::NativeScalarGetterKind::Wide
                    | tcl_syntax::scalar_getter::NativeScalarGetterKind::Long
                    | tcl_syntax::scalar_getter::NativeScalarGetterKind::Double
            )
        {
            crate::native_source::context(value)
                .map_err(|_| ValueError::ScalarNumericInputUnavailable)?
                .fresh_numeric_conversion(protocol, kind, &original)?
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
        crate::bytearray::scalar_getter_string(value, protocol)?;
    }
    if let Some(cache) = cache {
        obj::adopt_native_scalar_cache(value, cache, protocol)?;
    }
    Ok(outcome)
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
    use tcl_syntax::value::ValueError;
    obj::check_native_liveness(value)?;
    let protocol = dialect
        .native_scalar_getter_protocol()
        .filter(|protocol| protocol.supports_number_getter())
        .ok_or(ValueError::ScalarNumericInputUnavailable)?;
    if obj::native_word_boolean_version(value)
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
        let original = crate::bytearray::scalar_getter_string(value, protocol)?;
        protocol
            .fresh_number_conversion(kind, &original)
            .ok_or(ValueError::ScalarNumericInputUnavailable)?
    };
    let (cache, outcome) = conversion.into_parts();
    if let Some(cache) = cache {
        obj::adopt_native_scalar_cache(value, cache, protocol)?;
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
    use tcl_syntax::value::ValueError;
    match native_scalar_probe_with_environment(value, dialect, kind, environment)? {
        Ok(result) => Ok(result),
        Err(failure) => {
            let record = native_scalar_failure_presentation(value, dialect, kind, failure)?;
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
    use tcl_syntax::value::ValueError;
    obj::check_native_liveness(value)?;
    let protocol = dialect
        .native_scalar_getter_protocol()
        .ok_or(ValueError::ScalarNumericInputUnavailable)?;
    let requires_string = protocol
        .failure_requires_original_string(kind, failure)
        .ok_or(ValueError::ScalarNumericInputUnavailable)?;
    let record = if requires_string {
        let original = crate::bytearray::scalar_getter_string(value, protocol)?;
        protocol.failure_presentation(kind, failure, &original)
    } else {
        protocol.failure_presentation_without_original_string(kind, failure)
    };
    record.ok_or(ValueError::ScalarNumericInputUnavailable)
}

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

/// A failed typed read: C's interpreter result and its `-errorcode` list text.
pub(crate) struct TypedError {
    /// The message C leaves as the interpreter result.
    pub message: Vec<u8>,
    /// The `-errorcode` list C sets alongside it.
    pub code: &'static [u8],
}

impl TypedError {
    /// `expected <what> but got <value>` + `TCL VALUE NUMBER` — the error
    /// `TclParseNumber`'s `formaterr` raises for every unparsable spelling.
    fn expected(what: &str, obj: *mut TclObj, dialect: tcl_registry::InvocationDialect) -> Self {
        let bytes = obj::bytes_of(obj);
        let mut message = format!("expected {what} but got ").into_bytes();
        if let Some(version) = dialect.native_string_protocol().and_then(|protocol| protocol.tcl_version())
            .or_else(|| dialect.byte_array_string_recipe(Some(tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation)).and_then(|recipe| recipe.protocol().tcl_version()))
        {
            message.extend_from_slice(&tcl_syntax::list::describe_bad_value_bytes_in(
                &bytes, version, dialect.lexer_grammar.list_parse, dialect.lexer_grammar.escapes,
            ));
        } else {
            // Jim's expression-specific presenter consumes the original spelling.
            message.push(b'"');
            message.extend_from_slice(&bytes);
            message.push(b'"');
        }
        TypedError {
            message,
            code: b"TCL VALUE NUMBER",
        }
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
    if obj::obj_type_ptr(value) == &obj::TCL_LONG84_TYPE {
        if dialect
            .native_scalar_getter_protocol()
            .and_then(|protocol| protocol.tcl_version())
            != Some(tcl_dialect::TclVersion::V8_4)
        {
            return Err(
                tcl_syntax::raw_string::NativeValueAccessRefusal::ScalarNumericInputUnavailable,
            );
        }
        return Ok(Some(Number::Int(obj::wide_of(value))));
    }
    if obj::obj_type_ptr(value) == &obj::TCL_DOUBLE_TYPE {
        return Ok((!integer_only).then(|| Number::Double(obj::double_of(value))));
    }
    let original = scalar_input_bytes(value, dialect)?;
    let Some(text) = core::str::from_utf8(policy.input_bytes(&original)).ok() else {
        return Ok(None);
    };
    let mut flags = ParseFlags::for_syntax(dialect.numbers);
    flags.integer_only = integer_only;
    let Some(parsed) = tcl_syntax::number::parse_whole_with(text.trim(), flags) else {
        return Ok(None);
    };
    if policy == NativeScalarNumericInputPolicy::NulTerminatedJim084 {
        use tcl_syntax::expr::mathfunc::{NumValue, jim_numeric_operand};
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
        obj::adopt_native_numeric_representation(value, representation);
        return Ok(Some(number));
    }
    Ok(Some(parsed))
}

/// Boolean words and scalar numeric conversion use the same selected byte
/// boundary; successful numeric conversion belongs to the original object.
pub(crate) fn boolean_in(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
) -> Result<Result<bool, TypedError>, tcl_syntax::raw_string::NativeValueAccessRefusal> {
    obj::check_native_liveness(value).map_err(|error| {
        error
            .native_access_refusal()
            .expect("native object lifetime refusal")
    })?;
    use tcl_syntax::number::NativeScalarNumericInputPolicy;
    let policy = dialect
        .scalar_numeric_input_policy()
        .ok_or(tcl_syntax::raw_string::NativeValueAccessRefusal::ScalarNumericInputUnavailable)?;
    let had_string = obj::has_string_rep(value);
    let original = scalar_input_bytes(value, dialect)?;
    if policy == NativeScalarNumericInputPolicy::NulTerminatedJim084 {
        if let Ok(text) = core::str::from_utf8(policy.input_bytes(&original)) {
            if let Some(boolean) = tcl_syntax::boolean::parse_boolean_word(text.trim()) {
                return Ok(Ok(boolean));
            }
        }
        if scalar_number(value, dialect, true)?.is_none() {
            let _ = scalar_number(value, dialect, false)?;
        }
    }
    if had_string
        && dialect
            .native_scalar_getter_protocol()
            .and_then(|protocol| protocol.tcl_version())
            .is_some()
    {
        if core::str::from_utf8(&original)
            .ok()
            .and_then(|text| tcl_syntax::boolean::parse_boolean_word(text.trim()))
            .is_some()
        {
            match native_boolean(value, dialect) {
                Ok(boolean) => return Ok(Ok(boolean)),
                Err(error) => {
                    if let Some(refusal) = error.native_access_refusal() {
                        return Err(refusal);
                    }
                }
            }
        }
    }
    Ok(boolean(value, dialect))
}

/// Read `obj` as C Tcl reads a 64-bit `long` (`Tcl_GetLongFromObj` on an LP64
/// host): a wide integer, or an integer past the wide range that is not
/// negative and fits 64 bits unsigned, taken modulo 2^64 as C's `(long)` of an
/// `unsigned long` takes it, so `18446744073709551615` reads -1. Past 64 bits,
/// or below the wide range, it is the overflow [`wide_int`] reports.
pub(crate) fn wide_int_modulo_unsigned(obj: *mut TclObj) -> Result<i64, TypedError> {
    match wide_int(obj) {
        Err(error) if error.code == b"ARITH IOVERFLOW" => unsigned_past_wide(obj).ok_or(error),
        read => read,
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

/// Numeric truth for the expression evaluator's operand adapter.
/// Primitive command Boolean getters use `native_boolean` instead.
pub(crate) fn boolean(
    obj: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
) -> Result<bool, TypedError> {
    let bytes = obj::bytes_of(obj);
    let text = core::str::from_utf8(&bytes).ok();
    // The words first: they are release-invariant and never numeric, so this
    // never disturbs an object's rep.
    if let Some(value) = text.and_then(|text| tcl_syntax::boolean::parse_boolean_word(text.trim()))
    {
        return Ok(value);
    }
    match read_double(obj) {
        Some(value) if value.is_nan() => Err(TypedError {
            message: b"floating point value is Not a Number".to_vec(),
            code: b"TCL VALUE DOUBLE NAN",
        }),
        // Comparing the double widening against zero is exact for every value
        // the tower produces: a bignum is non-zero by construction, and no
        // integer rounds to zero under widening.
        Some(value) => Ok(value != 0.0),
        None => Err(TypedError::expected("boolean value", obj, dialect)),
    }
}

/// The double read over the numeric tower.
#[cfg(have_tommath)]
fn read_double(obj: *mut TclObj) -> Option<f64> {
    crate::bignum::read_double(obj)
}

/// Expression numeric truth without an arbitrary-precision backend.
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

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::TclVersion;
    use tcl_syntax::scalar_getter::{
        NativeScalarCache, NativeScalarGetterKind as Kind, NativeScalarGetterValue as Value,
    };
    use tcl_syntax::value::ValueError;

    const VERSIONS: [TclVersion; 5] = [
        TclVersion::V8_4,
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ];

    fn command_cache(
        version: TclVersion,
        name: &[u8],
    ) -> tcl_runtime_api::native_command_name::NativeCommandNameCache {
        use tcl_core_types::{ByteNamespacePath, NativeByteCommandSlot};
        use tcl_runtime_api::native_command_name::{
            NativeCommandNameCache, NativeCommandNameReference,
        };
        NativeCommandNameCache {
            interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity {
                owner: 1,
                interpreter: 2,
            },
            version,
            slot: NativeByteCommandSlot::new(ByteNamespacePath::root(), name.into()),
            namespace_token: 3,
            token: 4,
            implementation_generation: 5,
            command_epoch: 6,
            reference: Some(NativeCommandNameReference {
                namespace_token: 3,
                command_reference_epoch: 7,
            }),
        }
    }

    #[test]
    fn command_cache_getters_match_all_native_original_object_rows() {
        // Native proof: naming.command-cache.original-wide-getter
        // docs/design/analysis/name-resolution-proofs/command-cache-original-wide-getter.md
        // Native proof: naming.command-cache.original-double-getter
        // docs/design/analysis/name-resolution-proofs/command-cache-original-double-getter.md
        // Native proof: naming.command-cache.original-boolean-getter
        // docs/design/analysis/name-resolution-proofs/command-cache-original-boolean-getter.md
        // Native proof: naming.command-cache.original-list-length-getter
        // docs/design/analysis/name-resolution-proofs/command-cache-original-list-length-getter.md
        // Native proof: naming.command-cache.original-dictionary-size-getter
        // docs/design/analysis/name-resolution-proofs/command-cache-original-dictionary-size-getter.md
        // Native proof: naming.command-cache.original-character-length-getter
        // docs/design/analysis/name-resolution-proofs/command-cache-original-character-length-getter.md
        use tcl_syntax::native_object::NativeObjectCacheSnapshot;
        use tcl_syntax::value::ValueOps;
        const INPUTS: [&[u8]; 6] = [b"1", b"1.0", b"true", b"1 2", b"1 x", b"bad"];
        const FIXTURES: [&str; 5] = [
            include_str!("../tests/data/native_command_cache_getters/8.4.20.tsv"),
            include_str!("../tests/data/native_command_cache_getters/8.5.19.tsv"),
            include_str!("../tests/data/native_command_cache_getters/8.6.18.tsv"),
            include_str!("../tests/data/native_command_cache_getters/9.0.4.tsv"),
            include_str!("../tests/data/native_command_cache_getters/9.1.0.tsv"),
        ];
        let mut rows = 0;
        let mut unavailable = 0;
        for (version, fixture) in VERSIONS.into_iter().zip(FIXTURES) {
            let mut interp = crate::interp::Interp::new();
            interp.set_runtime_version(version);
            let dialect = interp.native_invocation_dialect();
            for row in fixture.lines() {
                let fields: Vec<_> = row.split('\t').collect();
                assert_eq!(fields.len(), 8);
                let stage: u8 = fields[0].parse().unwrap();
                let case: usize = fields[1].parse().unwrap();
                let value = obj::Owned::fresh(obj::new_string_bytes(INPUTS[case]));
                let alias = value.clone();
                let original_bytes = unsafe { (*value.as_ptr()).bytes };
                let cache = command_cache(version, INPUTS[case]);
                obj::install_native_command_name_cache(value.as_ptr(), cache.clone(), dialect)
                    .unwrap();
                assert_eq!(
                    obj::native_command_name_cache(alias.as_ptr()),
                    Some(cache.clone())
                );
                assert_eq!(
                    obj::native_object_snapshot(value.as_ptr()).unwrap().cache,
                    NativeObjectCacheSnapshot::CommandName {
                        version,
                        resolved: true
                    }
                );
                assert_eq!(fields[2], "cmdName");
                rows += 1;
                if fields[3] == "-1" {
                    assert_eq!((version, stage), (TclVersion::V8_4, 4));
                    unavailable += 1;
                    continue;
                }
                let result = match stage {
                    0 => native_scalar_getter(value.as_ptr(), dialect, Kind::Wide).map(|v| {
                        let Value::Wide(n) = v else {
                            panic!("Wide getter return")
                        };
                        n
                    }),
                    1 => native_scalar_getter(value.as_ptr(), dialect, Kind::Double).map(|v| {
                        let Value::Double(n) = v else {
                            panic!("Double getter return")
                        };
                        n as i64
                    }),
                    2 => native_scalar_getter(value.as_ptr(), dialect, Kind::Boolean).map(|v| {
                        let Value::Boolean(n) = v else {
                            panic!("Boolean getter return")
                        };
                        i64::from(n)
                    }),
                    3 => interp.list_len(&value.as_ptr()).map(|n| n as i64),
                    4 => crate::dict::ensure_dict_native(
                        value.as_ptr(),
                        dialect.native_string_protocol().unwrap(),
                    )
                    .map(|()| crate::dict::native_cache_size(value.as_ptr()).unwrap() as i64),
                    5 => interp.native_char_len(&value.as_ptr()).map(|n| n as i64),
                    _ => unreachable!(),
                };
                assert_eq!(
                    result.is_ok(),
                    fields[3] == "0",
                    "{version:?}: {row}: {result:?}"
                );
                if let Ok(number) = result {
                    assert_eq!(
                        number,
                        fields[4].parse::<i64>().unwrap(),
                        "{version:?}: {row}"
                    );
                }
                let observed = if version == TclVersion::V8_4
                    && matches!(
                        obj::native_scalar_cache(value.as_ptr()).unwrap(),
                        Some(NativeScalarCache::Number(tcl_syntax::number::Number::Int(
                            _
                        )))
                    ) {
                    "wideInt"
                } else {
                    let descriptor = obj::obj_type_ptr(value.as_ptr());
                    assert!(
                        !descriptor.is_null(),
                        "{version:?}: {row}: missing primary descriptor"
                    );
                    unsafe { core::ffi::CStr::from_ptr((*descriptor).name) }
                        .to_str()
                        .unwrap()
                };
                assert_eq!(observed, fields[5], "{version:?}: {row}");
                assert_eq!(
                    obj::native_command_name_cache(alias.as_ptr()),
                    (fields[5] == "cmdName").then_some(cache),
                    "{version:?}: {row}"
                );
                assert_eq!(obj::has_string_rep(alias.as_ptr()), fields[6] == "1");
                assert_eq!(
                    unsafe { (*alias.as_ptr()).bytes } == original_bytes,
                    fields[7] == "1"
                );
                assert_eq!(obj::bytes_of(alias.as_ptr()), INPUTS[case]);
            }
        }
        assert_eq!(rows, 180);
        assert_eq!(unavailable, 6);
    }

    #[test]
    fn command_cache_priming_matches_every_native_original_object_row() {
        // Native proof: naming.command-cache.original-compiler-priming
        // docs/design/analysis/name-resolution-proofs/command-cache-original-compiler-priming.md
        const FIXTURES: [&str; 5] = [
            include_str!("../tests/data/native_command_cache_priming/8.4.20.tsv"),
            include_str!("../tests/data/native_command_cache_priming/8.5.19.tsv"),
            include_str!("../tests/data/native_command_cache_priming/8.6.18.tsv"),
            include_str!("../tests/data/native_command_cache_priming/9.0.4.tsv"),
            include_str!("../tests/data/native_command_cache_priming/9.1.0.tsv"),
        ];
        let mut rows = 0;
        for (version, fixture) in VERSIONS.into_iter().zip(FIXTURES) {
            let dialect = tcl_registry::InvocationDialect::for_version(version);
            let first = command_cache(version, b"A");
            let mut second = command_cache(version, b"B");
            second.token = 9;
            for row in fixture.lines() {
                let fields: Vec<_> = row.split('\t').collect();
                assert_eq!(fields.len(), 7);
                let case: u8 = fields[0].parse().unwrap();
                let value = obj::Owned::fresh(if case == 2 {
                    obj::new_wide_int_obj(7)
                } else {
                    obj::new_string_bytes(if case == 1 { b"MISSING" } else { b"A" })
                });
                let alias = value.clone();
                // Actual name lookup obtains the original string before its cache effect.
                let _ = obj::bytes_of(value.as_ptr());
                match case {
                    0 => obj::install_native_command_name_cache(
                        value.as_ptr(),
                        first.clone(),
                        dialect,
                    )
                    .unwrap(),
                    1 | 2 => {
                        obj::install_unresolved_native_command_name_cache(value.as_ptr(), dialect)
                            .unwrap()
                    }
                    3 | 4 => {
                        obj::prime_native_command_name_cache(value.as_ptr(), first.clone(), dialect)
                            .unwrap()
                    }
                    _ => unreachable!(),
                }
                let type_name = |value| {
                    let descriptor = obj::obj_type_ptr(value);
                    if descriptor.is_null() {
                        return "none";
                    }
                    // SAFETY: a non-null live descriptor owns its static name.
                    unsafe { core::ffi::CStr::from_ptr((*descriptor).name) }
                        .to_str()
                        .unwrap()
                };
                assert_eq!(type_name(value.as_ptr()), fields[1], "{version:?}: {row}");
                assert_eq!(obj::has_string_rep(value.as_ptr()), fields[2] == "1");
                if fields[1] == "cmdName" {
                    assert_eq!(
                        obj::native_object_snapshot(value.as_ptr()).unwrap().cache,
                        tcl_syntax::native_object::NativeObjectCacheSnapshot::CommandName {
                            version,
                            resolved: obj::native_command_name_cache(value.as_ptr()).is_some(),
                        }
                    );
                }
                let original_bytes = unsafe { (*value.as_ptr()).bytes };
                obj::prime_native_command_name_cache(
                    value.as_ptr(),
                    if case == 3 {
                        first.clone()
                    } else {
                        second.clone()
                    },
                    dialect,
                )
                .unwrap();
                assert_eq!(type_name(alias.as_ptr()), fields[3], "{version:?}: {row}");
                let target = obj::native_command_name_cache(alias.as_ptr()).map_or(0, |cache| {
                    if cache.token == first.token {
                        1
                    } else if cache.token == second.token {
                        2
                    } else {
                        panic!("unexpected command token")
                    }
                });
                assert_eq!(
                    target,
                    fields[4].parse::<u8>().unwrap(),
                    "{version:?}: {row}"
                );
                assert_eq!(
                    obj::native_command_name_unresolved_version(alias.as_ptr()),
                    (target == 0).then_some(version)
                );
                assert_eq!(obj::has_string_rep(alias.as_ptr()), fields[5] == "1");
                assert_eq!(
                    unsafe { (*alias.as_ptr()).bytes } == original_bytes,
                    fields[6] == "1"
                );
                rows += 1;
            }
        }
        assert_eq!(rows, 25);
    }

    #[test]
    fn command_cache_duplicate_origin_and_retirement_preserve_original_storage() {
        let version = TclVersion::V9_0;
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let value = obj::Owned::fresh(obj::new_string_bytes(b"::opaque\0\xff"));
        let alias = value.clone();
        let original_bytes = unsafe { (*value.as_ptr()).bytes };
        let cache = command_cache(version, b"::opaque\0\xff");
        obj::install_native_command_name_cache(value.as_ptr(), cache.clone(), dialect).unwrap();
        let duplicate = obj::Owned::fresh(obj::duplicate(value.as_ptr()));
        assert_eq!(
            obj::native_command_name_cache(duplicate.as_ptr()),
            Some(cache.clone())
        );
        assert_ne!(
            obj::internal_rep(duplicate.as_ptr()),
            obj::internal_rep(value.as_ptr())
        );
        assert_eq!(obj::bytes_of(duplicate.as_ptr()), b"::opaque\0\xff");
        assert!(
            obj::install_native_command_name_cache(
                value.as_ptr(),
                cache.clone(),
                tcl_registry::InvocationDialect::for_version(TclVersion::V8_6)
            )
            .is_err()
        );
        assert_eq!(obj::native_command_name_cache(alias.as_ptr()), Some(cache));
        assert_eq!(unsafe { (*alias.as_ptr()).bytes }, original_bytes);
        obj::retire_native_command_name_cache(value.as_ptr()).unwrap();
        assert!(obj::native_command_name_cache(alias.as_ptr()).is_none());
        assert_eq!(unsafe { (*alias.as_ptr()).bytes }, original_bytes);
        let plain = obj::Owned::fresh(obj::new_wide_int_obj(7));
        assert!(
            obj::install_native_command_name_cache(
                plain.as_ptr(),
                command_cache(version, b"7"),
                dialect
            )
            .is_err()
        );
        assert!(!obj::has_string_rep(plain.as_ptr()));
    }
    const WIDE: [&str; 5] = [
        include_str!("../../../rust/tcl-syntax/tests/data/native_scalar_getters/wide-8.4.txt"),
        include_str!("../../../rust/tcl-syntax/tests/data/native_scalar_getters/wide-8.5.txt"),
        include_str!("../../../rust/tcl-syntax/tests/data/native_scalar_getters/wide-8.6.txt"),
        include_str!("../../../rust/tcl-syntax/tests/data/native_scalar_getters/wide-9.0.txt"),
        include_str!("../../../rust/tcl-syntax/tests/data/native_scalar_getters/wide-9.1.txt"),
    ];
    const DOUBLE: [&str; 5] = [
        include_str!("../../../rust/tcl-syntax/tests/data/native_scalar_getters/double-8.4.txt"),
        include_str!("../../../rust/tcl-syntax/tests/data/native_scalar_getters/double-8.5.txt"),
        include_str!("../../../rust/tcl-syntax/tests/data/native_scalar_getters/double-8.6.txt"),
        include_str!("../../../rust/tcl-syntax/tests/data/native_scalar_getters/double-9.0.txt"),
        include_str!("../../../rust/tcl-syntax/tests/data/native_scalar_getters/double-9.1.txt"),
    ];
    fn field<'a>(line: &'a str, key: &str) -> &'a str {
        line.split_ascii_whitespace()
            .find_map(|word| {
                let (name, value) = word.split_once('=')?;
                (name == key).then_some(value)
            })
            .expect("measured getter field")
    }
    fn cache_kind(value: *mut TclObj) -> &'static str {
        match obj::native_scalar_cache(value).unwrap() {
            None => "string",
            Some(NativeScalarCache::Number(tcl_syntax::number::Number::Int(_))) => "integer",
            Some(NativeScalarCache::Tcl84Long(_)) => "integer",
            Some(NativeScalarCache::Number(tcl_syntax::number::Number::Big { .. })) => "bignum",
            Some(NativeScalarCache::Number(_)) => "double",
            Some(NativeScalarCache::WordBoolean(_)) => "boolean",
            Some(NativeScalarCache::JimCoercedInteger(_)) => "coerced-double",
        }
    }
    #[test]
    fn physical_fresh_getters_match_measured_c_values_and_original_caches() {
        let mut count = 0;
        for (kind, fixtures) in [(Kind::Wide, WIDE), (Kind::Double, DOUBLE)] {
            for (index, fixture) in fixtures.iter().enumerate() {
                for line in fixture.lines() {
                    count += 1;
                    let input = line.split_once('\t').unwrap().0.as_bytes();
                    let value = obj::Owned::fresh(obj::new_string_bytes(input));
                    let result = native_scalar_getter(
                        value.as_ptr(),
                        tcl_registry::InvocationDialect::for_version(VERSIONS[index]),
                        kind,
                    );
                    let native_type = field(line, "type");
                    #[cfg(not(have_tommath))]
                    if native_type == "bignum" {
                        assert_eq!(
                            result,
                            Err(ValueError::ScalarNumericInputUnavailable),
                            "{line}"
                        );
                        assert_eq!(cache_kind(value.as_ptr()), "string");
                        assert_eq!(obj::bytes_of(value.as_ptr()), input);
                        continue;
                    }
                    assert_eq!(
                        result.is_ok(),
                        field(line, "code") == "0",
                        "engine {index}: {line}: {result:?}"
                    );
                    if let Ok(returned) = result {
                        let bits = match returned {
                            Value::Wide(integer) => integer.cast_unsigned(),
                            Value::Double(double) => double.to_bits(),
                            Value::Boolean(boolean) => {
                                u64::from(boolean.returned_integer().cast_unsigned())
                            }
                        };
                        assert_eq!(
                            bits,
                            u64::from_str_radix(field(line, "bits"), 16).unwrap(),
                            "{line}"
                        );
                    }
                    let expected = match native_type {
                        "int" | "wideInt" => "integer",
                        "booleanString" | "boolean" => "boolean",
                        other => other,
                    };
                    assert_eq!(
                        cache_kind(value.as_ptr()),
                        expected,
                        "engine {index}: {line}"
                    );
                    assert_eq!(
                        obj::bytes_of(value.as_ptr()),
                        input,
                        "retained original spelling: {line}"
                    );
                }
            }
        }
        assert_eq!(count, 260);
    }
    #[test]
    fn primitive_boolean_storage_and_followup_wide_match_all_native_fixtures() {
        const FIXTURES: [&str; 6] = [
            include_str!(
                "../../../rust/tcl-syntax/tests/data/native_scalar_getters/boolean-8.4.txt"
            ),
            include_str!(
                "../../../rust/tcl-syntax/tests/data/native_scalar_getters/boolean-8.5.txt"
            ),
            include_str!(
                "../../../rust/tcl-syntax/tests/data/native_scalar_getters/boolean-8.6.txt"
            ),
            include_str!(
                "../../../rust/tcl-syntax/tests/data/native_scalar_getters/boolean-9.0.txt"
            ),
            include_str!(
                "../../../rust/tcl-syntax/tests/data/native_scalar_getters/boolean-9.1.txt"
            ),
            include_str!(
                "../../../rust/tcl-syntax/tests/data/native_scalar_getters/jim-boolean-native.txt"
            ),
        ];
        let inputs: [&[u8]; 7] = [
            b"1\0X",
            b"true\0X",
            b"false\0X",
            b"true",
            b"false",
            b"2",
            b"1.5",
        ];
        let mut count = 0;
        for (index, fixture) in FIXTURES.iter().enumerate() {
            let dialect = VERSIONS.get(index).map_or_else(
                || {
                    tcl_registry::InvocationDialect::of_profile(
                        crate::environment::profile_for_dialect("jim"),
                    )
                },
                |&version| tcl_registry::InvocationDialect::for_version(version),
            );
            let jim_context = dialect
                .native_scalar_getter_protocol()
                .filter(|protocol| protocol.is_jim084())
                .map(|_| {
                    let context = crate::native_source::NativeJimObjectContext::new(dialect)
                        .expect("selected original Jim fixture context");
                    context
                        .select_numeric_host(std::rc::Rc::new(tcl_host_native::NativeHost::new()));
                    context
                });
            for line in fixture.lines() {
                let input = inputs[field(line, "case").parse::<usize>().unwrap()];
                // The C fixture has both storage kinds; Jim's seven rows
                // are raw strings and its schema intentionally has no kind.
                let object = if index < VERSIONS.len() && field(line, "kind") == "bytearray" {
                    crate::bytearray::new_byte_array(
                        input,
                        dialect.byte_array_string_recipe(None).unwrap(),
                    )
                } else {
                    obj::new_string_bytes(input)
                };
                let value = obj::Owned::fresh(object);
                if let Some(context) = &jim_context {
                    crate::native_source::bind_context(value.as_ptr(), context)
                        .expect("original scalar belongs to selected Jim fixture");
                }
                // The fresh C84 numeric branch owns actual strtol/strtod and
                // its independently supplied C target, not a profile default.
                let environment = tcl_host_c_abi::NativeNumericEnvironment;
                let boolean = native_scalar_getter_with_environment(
                    value.as_ptr(),
                    dialect,
                    tcl_syntax::scalar_getter::NativeScalarGetterKind::Boolean,
                    Some(&environment),
                )
                .map(|returned| match returned {
                    tcl_syntax::scalar_getter::NativeScalarGetterValue::Boolean(value) => {
                        value.is_true()
                    }
                    _ => panic!("Boolean getter result"),
                });
                assert_eq!(
                    boolean.is_ok(),
                    field(line, "boolcode") == "0",
                    "engine {index}: {line}: {boolean:?}"
                );
                if let Ok(boolean) = boolean {
                    assert_eq!(u8::from(boolean).to_string(), field(line, "bool"), "{line}");
                }
                let expected_cache = match field(line, "booltype") {
                    "int" | "wideInt" => "integer",
                    "booleanString" | "boolean" => "boolean",
                    "bytearray" => "string",
                    other => other,
                };
                assert_eq!(
                    cache_kind(value.as_ptr()),
                    expected_cache,
                    "engine {index}: {line}"
                );
                let wide = native_wide_int(value.as_ptr(), dialect);
                assert_eq!(
                    wide.is_ok(),
                    field(line, "widecode") == "0",
                    "engine {index}: {line}: {wide:?}"
                );
                if let Ok(wide) = wide {
                    assert_eq!(wide, field(line, "wide").parse::<i64>().unwrap(), "{line}");
                }
                count += 1;
            }
        }
        assert_eq!(count, 77);
    }

    #[test]
    fn primitive_int_matches_native_width_cache_and_failure_on_original_objects() {
        // Native proof: naming.numeric.primitive-int-original-width-cache
        // docs/design/analysis/name-resolution-proofs/numeric-primitive-int-original-width-cache.md

        use tcl_syntax::scalar_getter::NativeScalarGetterErrorCode;
        const FIXTURES: [&str; 5] = [
            include_str!(
                "../../../rust/tcl-syntax/tests/data/native_scalar_getters/int/8.4.20.txt"
            ),
            include_str!(
                "../../../rust/tcl-syntax/tests/data/native_scalar_getters/int/8.5.19.txt"
            ),
            include_str!(
                "../../../rust/tcl-syntax/tests/data/native_scalar_getters/int/8.6.18.txt"
            ),
            include_str!("../../../rust/tcl-syntax/tests/data/native_scalar_getters/int/9.0.4.txt"),
            include_str!("../../../rust/tcl-syntax/tests/data/native_scalar_getters/int/9.1.0.txt"),
        ];
        const INPUTS: [&[u8]; 11] = [
            b"1\0X",
            b"2147483648",
            b"4294967295",
            b"4294967296",
            b"-4294967295",
            b"9223372036854775808",
            b"1.0",
            b"NaN",
            b"08",
            b"0x1",
            b"bad",
        ];
        fn decode_hex(field: &str) -> Vec<u8> {
            if field == "-" {
                return Vec::new();
            }
            field
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        }
        let mut count = 0;
        for (index, fixture) in FIXTURES.iter().enumerate() {
            let version = VERSIONS[index];
            let dialect = tcl_registry::InvocationDialect::for_version(version);
            obj::install_double_string_policy(tcl_dialect::DoubleStringPolicy::for_tcl_version(
                version,
            ));
            for line in fixture.lines() {
                let fields: Vec<_> = line.split('\t').collect();
                assert_eq!(fields.len(), 6, "{line}");
                let case: usize = fields[0].parse().unwrap();
                let value = obj::Owned::fresh(match case {
                    11 => obj::new_double_obj(1.0),
                    12 => obj::new_wide_int_obj(1),
                    13 => obj::new_double_obj(f64::NAN),
                    _ => obj::new_string_bytes(INPUTS[case]),
                });
                let alias = value.clone();
                let result = native_scalar_getter(value.as_ptr(), dialect, Kind::Int);
                count += 1;
                #[cfg(not(have_tommath))]
                if fields[3] == "bignum" {
                    assert_eq!(
                        result,
                        Err(ValueError::ScalarNumericInputUnavailable),
                        "{line}"
                    );
                    assert_eq!(cache_kind(alias.as_ptr()), "string");
                    assert_eq!(obj::bytes_of(alias.as_ptr()), INPUTS[case]);
                    continue;
                }
                assert_eq!(
                    result.is_ok(),
                    fields[1] == "0",
                    "{version:?}: {line}: {result:?}"
                );
                match result {
                    Ok(Value::Wide(integer)) => {
                        assert_eq!(integer, fields[2].parse::<i64>().unwrap(), "{line}")
                    }
                    Err(ValueError::NativeScalarGetter(error)) => {
                        assert_eq!(error.getter_kind(), Kind::Int);
                        assert_eq!(
                            error.message_bytes(),
                            decode_hex(fields[4]),
                            "{version:?}: {line}"
                        );
                        let code = match error.error_code_update() {
                            NativeScalarGetterErrorCode::Unchanged => b"SEEDED CODE".as_slice(),
                            NativeScalarGetterErrorCode::Set(bytes) => bytes.as_slice(),
                        };
                        assert_eq!(code, decode_hex(fields[5]), "{version:?}: {line}");
                    }
                    other => panic!("unexpected Int result {version:?}: {line}: {other:?}"),
                }
                let native_cache = match fields[3] {
                    "int" | "wideInt" => "integer",
                    other => other,
                };
                assert_eq!(
                    cache_kind(alias.as_ptr()),
                    native_cache,
                    "{version:?}: {line}"
                );
                if case < INPUTS.len() {
                    assert_eq!(obj::bytes_of(alias.as_ptr()), INPUTS[case], "{line}");
                }
                // Width extraction wraps independently of the complete cache.
                if (1..=4).contains(&case) {
                    let magnitude = core::str::from_utf8(INPUTS[case])
                        .unwrap()
                        .parse::<i64>()
                        .unwrap();
                    assert_eq!(
                        obj::native_scalar_cache(alias.as_ptr()).unwrap(),
                        Some(
                            if version == tcl_dialect::TclVersion::V8_4 && fields[3] == "int" {
                                NativeScalarCache::Tcl84Long(magnitude)
                            } else {
                                NativeScalarCache::Number(tcl_syntax::number::Number::Int(
                                    magnitude,
                                ))
                            }
                        ),
                        "{line}"
                    );
                }
            }
        }
        assert_eq!(count, 70);
    }

    #[test]
    fn bytearray_materialization_keeps_modified_nul_distinct_from_raw_string() {
        for version in VERSIONS {
            let dialect = tcl_registry::InvocationDialect::for_version(version);
            let raw = obj::Owned::fresh(obj::new_string_bytes(b"1\0X"));
            let bytes = obj::Owned::fresh(crate::bytearray::new_byte_array(
                b"1\0X",
                dialect.byte_array_string_recipe(None).unwrap(),
            ));
            let raw_result = native_wide_int(raw.as_ptr(), dialect);
            assert_eq!(raw_result.is_ok(), version != TclVersion::V8_4);
            assert!(matches!(
                native_wide_int(bytes.as_ptr(), dialect),
                Err(ValueError::NativeScalarGetter(_))
            ));
            assert_eq!(obj::bytes_of(raw.as_ptr()), b"1\0X");
            assert_eq!(obj::bytes_of(bytes.as_ptr()), b"1\xc0\x80X");
            assert!(core::ptr::eq(
                obj::obj_type_ptr(bytes.as_ptr()),
                &crate::bytearray::TCL_BYTE_ARRAY_TYPE
            ));
        }
    }
    #[test]
    fn reached_boolean_cache_changes_shared_container_on_the_original_object() {
        for version in VERSIONS {
            let item = obj::Owned::fresh(obj::new_string_bytes(b"true"));
            let value = obj::Owned::fresh(crate::list::new_list_obj(&[item.as_ptr()]));
            let alias = value.clone();
            assert_eq!(
                native_boolean(
                    value.as_ptr(),
                    tcl_registry::InvocationDialect::for_version(version)
                ),
                Ok(true)
            );
            assert_eq!(cache_kind(alias.as_ptr()), "boolean");
            assert_eq!(obj::bytes_of(alias.as_ptr()), b"true");
        }
    }
    #[test]
    fn native_word_boolean_descriptor_retains_release_specific_string_contract() {
        for version in VERSIONS {
            let value = obj::Owned::fresh(obj::new_string_bytes(b"true"));
            assert_eq!(
                native_boolean(
                    value.as_ptr(),
                    tcl_registry::InvocationDialect::for_version(version)
                ),
                Ok(true)
            );
            let descriptor = obj::obj_type_ptr(value.as_ptr());
            // SAFETY: native_boolean installed a live static descriptor.
            let (name, updater) = unsafe {
                (
                    core::ffi::CStr::from_ptr((*descriptor).name).to_bytes(),
                    (*descriptor).update_string_proc.is_some(),
                )
            };
            assert_eq!(
                name,
                if matches!(version, TclVersion::V8_5 | TclVersion::V8_6) {
                    b"booleanString".as_slice()
                } else {
                    b"boolean".as_slice()
                }
            );
            assert_eq!(updater, version == TclVersion::V8_4);
            if updater {
                obj::invalidate_string(value.as_ptr());
                assert_eq!(obj::bytes_of(value.as_ptr()), b"1");
            } else {
                assert_eq!(obj::bytes_of(value.as_ptr()), b"true");
            }
        }
    }

    #[test]
    fn failed_double_getter_keeps_nan_cache_and_payload() {
        let value = obj::Owned::fresh(obj::new_string_bytes(b"-NaN(123)"));
        assert!(matches!(
            native_double(
                value.as_ptr(),
                tcl_registry::InvocationDialect::for_version(TclVersion::V8_6)
            ),
            Err(ValueError::NativeScalarGetter(_))
        ));
        assert!(core::ptr::eq(
            obj::obj_type_ptr(value.as_ptr()),
            &obj::TCL_DOUBLE_TYPE
        ));
        assert_eq!(
            obj::double_of(value.as_ptr()).to_bits(),
            0xfff8_0000_0000_0123
        );
        assert_eq!(obj::bytes_of(value.as_ptr()), b"-NaN(123)");
    }
    #[test]
    fn cached_numeric_boolean_uses_its_own_string_materialization_obligation() {
        for version in VERSIONS {
            let value = obj::Owned::fresh(obj::new_wide_int_obj(2));
            assert!(!obj::has_string_rep(value.as_ptr()));
            assert_eq!(
                native_boolean(
                    value.as_ptr(),
                    tcl_registry::InvocationDialect::for_version(version)
                ),
                Ok(true)
            );
            assert_eq!(
                obj::has_string_rep(value.as_ptr()),
                version == TclVersion::V8_4
            );
            assert_eq!(
                cache_kind(value.as_ptr()),
                if version == TclVersion::V8_4 {
                    "boolean"
                } else {
                    "integer"
                }
            );
            assert_eq!(obj::bytes_of(value.as_ptr()), b"2");
        }
    }
    #[test]
    fn jim_double_cache_retains_exact_integer_and_wide_restores_it() {
        let dialect = tcl_registry::InvocationDialect::of_profile(
            crate::environment::profile_for_dialect("jim"),
        );
        let context = crate::native_source::NativeJimObjectContext::new(dialect).unwrap();
        context.select_numeric_host(std::rc::Rc::new(tcl_host_native::NativeHost::new()));
        for integer in [17, 9_007_199_254_740_993_i64] {
            let value = obj::Owned::fresh(obj::new_wide_int_obj(integer));
            crate::native_source::bind_context(value.as_ptr(), &context).unwrap();
            assert!(native_double(value.as_ptr(), dialect).is_ok());
            assert_eq!(cache_kind(value.as_ptr()), "coerced-double");
            assert_eq!(obj::wide_of(value.as_ptr()), integer);
            assert_eq!(obj::has_string_rep(value.as_ptr()), integer != 17);
            assert_eq!(native_wide_int(value.as_ptr(), dialect), Ok(integer));
            assert_eq!(cache_kind(value.as_ptr()), "integer");
        }
    }
    #[test]
    fn jim_missing_range_state_and_vendor_engine_are_host_refusals() {
        let jim = tcl_registry::InvocationDialect::of_profile(
            crate::environment::profile_for_dialect("jim"),
        );
        for input in [b"9223372036854775807".as_slice(), b"-9223372036854775808"] {
            let value = obj::Owned::fresh(obj::new_string_bytes(input));
            assert_eq!(
                native_wide_int(value.as_ptr(), jim),
                Err(ValueError::ScalarNumericInputUnavailable)
            );
            assert_eq!(cache_kind(value.as_ptr()), "string");
        }
        let vendor = tcl_registry::InvocationDialect::of_profile(
            crate::environment::profile_for_dialect("f5-irules"),
        );
        let value = obj::Owned::fresh(obj::new_wide_int_obj(3));
        assert_eq!(
            native_wide_int(value.as_ptr(), vendor),
            Err(ValueError::ScalarNumericInputUnavailable)
        );
        assert!(!obj::has_string_rep(value.as_ptr()));
    }

    #[test]
    fn neutral_probe_preserves_absent_string_and_original_nan_bits() {
        let dialect = tcl_registry::InvocationDialect::for_version(TclVersion::V8_6);
        let bits = 0xfff8_0000_0000_0042;
        let value = obj::Owned::fresh(obj::new_double_obj(f64::from_bits(bits)));
        assert!(!obj::has_string_rep(value.as_ptr()));
        assert!(
            native_scalar_probe(value.as_ptr(), dialect, Kind::Int)
                .unwrap()
                .is_err()
        );
        assert!(!obj::has_string_rep(value.as_ptr()));
        assert_eq!(obj::double_of(value.as_ptr()).to_bits(), bits);
        assert!(matches!(
            native_scalar_getter(value.as_ptr(), dialect, Kind::Int),
            Err(ValueError::NativeScalarGetter(_))
        ));
        // naming.numeric.original-capi-scalar-publication-width
        // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
        // C86 original case24/getter0 renders a constant overflow diagnostic.
        assert!(!obj::has_string_rep(value.as_ptr()));
    }

    #[test]
    fn primitive_nan_constant_failure_preserves_original_absent_string() {
        // naming.numeric.original-capi-scalar-publication-width
        // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
        // C85+ original case24/getter4 needs no original-string diagnostic.
        for version in VERSIONS.into_iter().skip(1) {
            let dialect = tcl_registry::InvocationDialect::for_version(version);
            let value = obj::Owned::fresh(obj::new_double_obj(f64::NAN));
            let Err(ValueError::NativeScalarGetter(record)) =
                native_scalar_getter(value.as_ptr(), dialect, Kind::Boolean)
            else {
                panic!("primitive Boolean failure")
            };
            assert_eq!(
                record.message_bytes(),
                b"floating point value is Not a Number"
            );
            assert!(!obj::has_string_rep(value.as_ptr()));
            assert!(obj::double_of(value.as_ptr()).is_nan());
        }
    }

    #[test]
    fn jim_completion_cache_has_no_string_updater() {
        use tcl_cmd_core::return_options::CompletionCodeCache;
        let value = obj::Owned::fresh(obj::new_wide_int_obj(7));
        adopt_completion_code_cache(value.as_ptr(), CompletionCodeCache::Jim(7)).unwrap();
        assert_eq!(
            completion_code_cache(value.as_ptr()),
            Some(CompletionCodeCache::Jim(7))
        );
        assert!(
            completion_code_string_bytes(
                value.as_ptr(),
                tcl_syntax::native_string::NativeStringProtocol::Jim084
            )
            .is_err()
        );
        assert!(!obj::has_string_rep(value.as_ptr()));
        let duplicate = obj::Owned::fresh(obj::duplicate(value.as_ptr()));
        assert_eq!(
            completion_code_cache(duplicate.as_ptr()),
            Some(CompletionCodeCache::Jim(7))
        );
        assert!(!obj::has_string_rep(duplicate.as_ptr()));
        let mut length = 9;
        assert!(unsafe { obj::get_string(value.as_ptr(), &mut length) }.is_null());
        assert_eq!(length, 0);
        assert!(!obj::has_string_rep(value.as_ptr()));
        let named = obj::Owned::fresh(obj::new_string_bytes(b"return"));
        adopt_completion_code_cache(named.as_ptr(), CompletionCodeCache::TclKeyword(2)).unwrap();
        assert_eq!(
            completion_code_string_bytes(
                named.as_ptr(),
                tcl_syntax::native_string::NativeStringProtocol::C(TclVersion::V8_6)
            )
            .unwrap(),
            b"return"
        );
    }

    #[test]
    fn native_string_count_keeps_exact_units_origin_and_duplicate_cache() {
        use tcl_registry::native_stock_list::NativeStockListInputClass;
        use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
        use tcl_syntax::native_string::NativeStringProtocol;
        for version in VERSIONS {
            let dialect = tcl_registry::InvocationDialect::for_version(version);
            let value = obj::Owned::fresh(obj::new_string_bytes(b"a\xc0\x80\xff"));
            let protocol = NativeStringProtocol::C(version);
            let representation = dialect.string_length_representation().unwrap();
            assert_eq!(
                obj::native_character_count(value.as_ptr(), protocol, representation),
                Ok(3)
            );
            assert_eq!(obj::bytes_of(value.as_ptr()), b"a\xc0\x80\xff");
            assert_eq!(
                obj::stock_list_input_class(value.as_ptr()),
                NativeStockListInputClass::String
            );
            let copied = obj::Owned::fresh(obj::duplicate(value.as_ptr()));
            assert_eq!(
                obj::native_character_count(copied.as_ptr(), protocol, representation),
                Ok(3)
            );
            assert_eq!(
                obj::native_unicode_units(copied.as_ptr(), protocol)
                    .unwrap()
                    .as_ref(),
                [97, 0, 255]
            );
            assert!(matches!(
                obj::native_object_snapshot(copied.as_ptr()).unwrap().cache,
                Cache::String {
                    unicode: Some(_),
                    num_chars: Some(3),
                    ..
                }
            ));
            assert!(matches!(
                obj::native_object_snapshot(value.as_ptr()).unwrap().cache,
                Cache::String {
                    unicode: None,
                    num_chars: Some(3),
                    ..
                }
            ));
            let other = if version == TclVersion::V9_0 {
                TclVersion::V8_6
            } else {
                TclVersion::V9_0
            };
            assert!(
                obj::native_character_count(
                    copied.as_ptr(),
                    NativeStringProtocol::C(other),
                    representation
                )
                .is_err()
            );
        }
    }
}

#[cfg(test)]
#[path = "native_number_tests.rs"]
mod native_number_tests;

#[cfg(test)]
#[path = "typed_value/native_scalar_input_tests.rs"]
mod native_scalar_input_tests;
