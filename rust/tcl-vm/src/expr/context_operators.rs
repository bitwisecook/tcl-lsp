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

//! Operators under the numeric policy an activation selected (`NumericContext`): native operand conversion, `arith_in`, `unary_in`, `compare_in` and their operand errors.

use super::arithmetic::{
    Num, arith_numbers, big_value, dbl_arith, divzero, int_value, is_int_only, num_f,
    zero_to_negative_power,
};
#[cfg(test)]
use super::boolean_context_err;
use super::comparison::{compare_values_numeric, compare_with_numeric_using, string_comparison};
use super::{NumericContext, unary};
use crate::error::TclError;
use crate::value::Value;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use std::cmp::Ordering;
use std::rc::Rc;
use tcl_syntax::expr::{BinOp, NumericCompare, UnaryOp, errors};
use tcl_syntax::number;
use tcl_syntax::number::Number;

/// Convert one operand with the selected native grammar and integer tower.
fn native_num(dialect: NumericContext<'_>, value: &Value) -> Result<Num, TclError> {
    dialect.current_for_value(value)?;
    if let Some(simulation) = dialect.simulation {
        let number = simulation
            .parse_number(
                &value.string_bytes(),
                tcl_syntax::logical_numeric_simulation::LogicalNumericInputStage::Number,
            )
            .map_err(|_| TclError::new("non-numeric operand"))?;
        return logical_number(dialect, number);
    }

    if dialect.arithmetic() == Some(tcl_dialect::NativeArithmetic::Tcl84Wide)
        && !dialect.prepare_integer84(value)?
    {
        return Err(TclError::new("non-numeric operand"));
    }

    let input = dialect.scalar_numeric_input_policy().ok_or_else(|| {
        TclError::from(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)
    })?;
    if input == number::NativeScalarNumericInputPolicy::NulTerminatedJim084
        && matches!(
            value.native_scalar_cache(),
            Some(
                tcl_syntax::scalar_getter::NativeScalarCache::JimCoercedInteger(_)
                    | tcl_syntax::scalar_getter::NativeScalarCache::Number(Number::Int(_))
            )
        )
    {
        match dialect.integer(value) {
            Ok(integer) => return Ok(Num::Int(integer)),
            Err(error) if error.is_host() => return Err(error),
            Err(_) => {}
        }
    }
    if let Some(value) = value.integer_representation() {
        return Ok(Num::Int(value));
    }
    if let Some(value) = value.double_representation() {
        return Ok(Num::Dbl(value));
    }
    if dialect.arithmetic() == Some(tcl_dialect::NativeArithmetic::Tcl84Wide)
        && (dialect.original.is_some() || dialect.environment.is_some())
    {
        return match dialect.scalar_probe(
            value,
            tcl_syntax::scalar_getter::NativeScalarGetterKind::Double,
        )? {
            Ok(tcl_syntax::scalar_getter::NativeScalarGetterValue::Double(value)) => {
                Ok(Num::Dbl(value))
            }
            _ => Err(TclError::new("non-numeric operand")),
        };
    }

    let policy = dialect
        .arithmetic()
        .ok_or_else(|| TclError::new("native arithmetic policy is not selected"))?;
    if let Some(number @ Number::Big { .. }) = value.number_representation() {
        if policy != tcl_dialect::NativeArithmetic::TclBignum {
            return tcl_syntax::expr::wide::parsed_literal(policy, &number)
                .map(Num::Int)
                .map_err(|error| native_wide_error(dialect, policy, error));
        }
        let Number::Big {
            negative,
            radix,
            digits,
        } = number
        else {
            unreachable!("matched full integer cache")
        };
        let magnitude = BigInt::parse_bytes(digits.as_bytes(), radix as u32)
            .expect("validated native Big magnitude");
        let integer = if negative { -magnitude } else { magnitude };
        return Ok(integer
            .to_i128()
            .map_or_else(|| Num::Huge(integer), Num::Big));
    }
    let original = dialect.native_string_for_value(value)?;
    let text = match std::str::from_utf8(input.input_bytes(&original)) {
        Ok(text) => text,
        Err(_) if input == number::NativeScalarNumericInputPolicy::NulTerminatedJim084 => {
            return Err(TclError::new("non-numeric operand"));
        }
        Err(error) => {
            return Err(tcl_syntax::raw_string::UnicodeAccessError {
                valid_up_to: error.valid_up_to(),
                error_len: error.error_len(),
            }
            .into());
        }
    };
    parsed_native_number(dialect, value, policy, text)
}

fn parsed_native_number(
    dialect: NumericContext<'_>,
    value: &Value,
    policy: tcl_dialect::NativeArithmetic,
    text: &str,
) -> Result<Num, TclError> {
    match number::parse_whole_with(text, number::ParseFlags::for_syntax(dialect.numbers)) {
        Some(Number::Int(integer)) => {
            dialect.cache(value, Number::Int(integer))?;
            Ok(Num::Int(integer))
        }
        Some(Number::Double(double)) => {
            dialect.cache(value, Number::Double(double))?;
            Ok(Num::Dbl(double))
        }
        Some(Number::Nan { .. }) => Ok(Num::Dbl(f64::NAN)),
        Some(Number::Big {
            negative,
            radix,
            digits,
        }) => {
            let magnitude = BigInt::parse_bytes(digits.as_bytes(), radix as u32)
                .ok_or_else(|| TclError::new("invalid integer literal"))?;
            let integer = if negative { -magnitude } else { magnitude };
            if policy == tcl_dialect::NativeArithmetic::TclBignum {
                return Ok(integer
                    .to_i128()
                    .map_or_else(|| Num::Huge(integer), Num::Big));
            }
            let integer = tcl_syntax::expr::wide::literal(policy, &integer)
                .map_err(|error| native_wide_error(dialect, policy, error))?;
            dialect.cache(value, Number::Int(integer))?;
            Ok(Num::Int(integer))
        }
        None => Err(TclError::new("non-numeric operand")),
    }
}

fn logical_number(dialect: NumericContext<'_>, number: Number) -> Result<Num, TclError> {
    match number {
        Number::Int(value) => Ok(Num::Int(value)),
        Number::Double(value) => Ok(Num::Dbl(value)),
        Number::Nan { .. } => Ok(Num::Dbl(f64::NAN)),
        integer @ Number::Big { .. } => {
            let policy = dialect
                .arithmetic()
                .ok_or_else(|| TclError::new("logical arithmetic policy is not selected"))?;
            tcl_syntax::expr::wide::parsed_literal(policy, &integer)
                .map(Num::Int)
                .map_err(|error| native_wide_error(dialect, policy, error))
        }
    }
}

/// Prepare a numeric value under an explicit reached invocation authority.
pub(crate) fn numeric_value_in(
    context: NumericContext<'_>,
    value: &Value,
) -> Result<Value, TclError> {
    native_num(context, value).map(native_num_value)
}

fn native_num_value(number: Num) -> Value {
    match number {
        Num::Int(value) => Value::int(value),
        Num::Big(value) => int_value(value),
        Num::Huge(value) => big_value(&value),
        Num::Dbl(value) => Value::double(value),
    }
}

/// Expression Boolean operators use `Tcl_NewLongObj` in C8.4, independently
/// of the operand's word-Boolean or wideInt primary.
pub(crate) fn native_boolean_result<'a>(
    context: impl Into<NumericContext<'a>>,
    value: bool,
) -> Value {
    let context = context.into();
    if let Some(cache) = context
        .native_scalar_getter_protocol()
        .and_then(|protocol| protocol.expression_integer_result84(i64::from(value), &[]))
    {
        Value::from_native_scalar_cache(cache, None, context.dialect)
            .expect("selected C84 Boolean result")
    } else {
        Value::bool(value)
    }
}

fn native_wide_error(
    dialect: NumericContext<'_>,
    policy: tcl_dialect::NativeArithmetic,
    error: tcl_syntax::expr::wide::WideError,
) -> TclError {
    use tcl_syntax::expr::wide::WideError;
    let error = match error {
        WideError::DivisionByZero if policy == tcl_dialect::NativeArithmetic::JimWide => {
            TclError::new("Division by zero")
        }
        WideError::DivisionByZero => divzero(),
        WideError::LiteralOverflow => TclError::with_error_code(
            "integer value too large to represent",
            "ARITH IOVERFLOW {integer value too large to represent}",
        ),
        WideError::ZeroToNegativePower if policy == tcl_dialect::NativeArithmetic::JimWide => {
            TclError::new("exponentiation of zero by negative power")
        }
        WideError::ZeroToNegativePower => zero_to_negative_power(),
        WideError::UndefinedNativeOperation => TclError::new("undefined native integer operation"),
        WideError::Unsupported => TclError::new("unsupported native integer operator"),
    };
    native_numeric_error84(dialect, error)
}

fn native_numeric_error84(dialect: NumericContext<'_>, error: TclError) -> TclError {
    if let Err(first) = dialect.current() {
        return first;
    }
    if dialect
        .native_scalar_getter_protocol()
        .is_none_or(|protocol| protocol.tcl_version() != Some(tcl_dialect::TclVersion::V8_4))
    {
        return error;
    }
    let Some(materialization) = dialect.dialect.native_string_materialization(None) else {
        return tcl_syntax::value::ValueError::ScalarNumericInputUnavailable.into();
    };
    if let Some(completion) = error.guest_completion()
        && let Err(refusal) = completion
            .result
            .retain_native_string_representation(materialization)
    {
        return refusal.into();
    }
    match dialect.current() {
        Ok(()) => error,
        Err(first) => first,
    }
}

/// Arithmetic used by both native expression evaluation and inline opcodes.
pub(crate) fn arith_in<'a>(
    dialect: impl Into<NumericContext<'a>>,
    op: BinOp,
    a: &Value,
    b: &Value,
) -> Result<Value, TclError> {
    let dialect = dialect.into();
    let policy = dialect
        .arithmetic()
        .ok_or_else(|| TclError::new("native arithmetic policy is not selected"))?;
    let x = native_operand(dialect, a, errors::OperandSide::Left, op)?;
    let y = native_operand(dialect, b, errors::OperandSide::Right, op)?;
    // A NaN operand is TCL_NUMBER_NAN, which every arithmetic operator
    // reports through IllegalExprOperandType.
    if matches!(
        op,
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Pow
    ) {
        for (number, value, side) in [
            (&x, a, errors::OperandSide::Left),
            (&y, b, errors::OperandSide::Right),
        ] {
            if matches!(number, Num::Dbl(double) if double.is_nan()) {
                return Err(selected_operand_error_code(
                    dialect,
                    context_operand_error(dialect, value, side, op.as_str()),
                ));
            }
        }
    }
    if is_int_only(op) && (matches!(x, Num::Dbl(_)) || matches!(y, Num::Dbl(_))) {
        let failed = if matches!(x, Num::Dbl(_)) { a } else { b };
        if let Some(error) = expression_operand_error(
            dialect,
            tcl_registry::native_numeric_error::NativeExpressionOperandStage::Integer,
            failed,
        ) {
            return Err(error);
        }
        return Err(selected_operand_error_code(
            dialect,
            if matches!(x, Num::Dbl(_)) {
                context_operand_error(dialect, a, errors::OperandSide::Left, op.as_str())
            } else {
                context_operand_error(dialect, b, errors::OperandSide::Right, op.as_str())
            },
        ));
    }
    if policy == tcl_dialect::NativeArithmetic::TclBignum {
        return arith_numbers(op, a, b, &x, &y);
    }
    if let (Num::Int(x), Num::Int(y)) = (&x, &y) {
        let integer = tcl_syntax::expr::wide::binary(policy, op, *x, *y)
            .map_err(|error| native_wide_error(dialect, policy, error))?;
        if let Some(cache) = dialect
            .native_scalar_getter_protocol()
            .and_then(|protocol| {
                protocol.expression_integer_result84(
                    integer,
                    &[a.native_scalar_cache(), b.native_scalar_cache()],
                )
            })
        {
            return Value::from_native_scalar_cache(cache, None, dialect.dialect)
                .map_err(Into::into);
        }
        return Ok(Value::int(integer));
    }
    // 8.4 refuses a double division by zero before dividing
    // (`tclExecute.c` 8.4 `INST_DIV`), where later releases return `Inf`.
    if op == BinOp::Div
        && num_f(&y) == 0.0
        && context_release(dialect).is_some_and(tcl_dialect::TclVersion::raises_float_range_errors)
    {
        return Err(TclError::with_error_code(
            errors::DIVZERO_MESSAGE,
            errors::DIVZERO_CODE,
        ));
    }
    dbl_arith(op, num_f(&x), num_f(&y))
}

/// The release whose numeric semantics `context` selects, if any.
fn context_release(context: NumericContext<'_>) -> Option<tcl_dialect::TclVersion> {
    if let Some(simulation) = context.simulation {
        return Some(match simulation {
            tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core => {
                tcl_dialect::TclVersion::V8_4
            }
        });
    }
    context
        .native_scalar_getter_protocol()
        .and_then(tcl_syntax::scalar_getter::NativeScalarGetterProtocol::tcl_version)
}

fn expression_operand_error(
    dialect: NumericContext<'_>,
    stage: tcl_registry::native_numeric_error::NativeExpressionOperandStage,
    value: &Value,
) -> Option<TclError> {
    let presenter = dialect.expression_operand_error_presentation()?;
    let original = match dialect.string_for_value(value) {
        Ok(original) => original,
        Err(cause) => return Some(cause),
    };
    Some(TclError::with_error_code(
        presenter.message(stage, &original),
        presenter.error_code(),
    ))
}

/// Present an incompatible arithmetic operand using only the selected numeric
/// authority. Authored logical simulation does not borrow the physical engine's
/// error wording, primitive error state or cache transition.
fn context_operand_error(
    context: NumericContext<'_>,
    value: &Value,
    side: errors::OperandSide,
    operator: &str,
) -> TclError {
    let release = if let Some(simulation) = context.simulation {
        match simulation {
            tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core => {
                tcl_dialect::TclVersion::V8_4
            }
        }
    } else if let Some(release) = context
        .native_scalar_getter_protocol()
        .and_then(tcl_syntax::scalar_getter::NativeScalarGetterProtocol::tcl_version)
    {
        release
    } else {
        return TclError::from(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable);
    };
    let original = match context.string_for_value(value) {
        Ok(original) => original,
        Err(cause) => return cause,
    };
    let parsed = if let Some(simulation) = context.simulation {
        simulation
            .parse_number(
                &original,
                tcl_syntax::logical_numeric_simulation::LogicalNumericInputStage::Number,
            )
            .ok()
    } else if let Some(double) = value.double_representation() {
        Some(Number::Double(double))
    } else {
        std::str::from_utf8(&original).ok().and_then(|text| {
            number::parse_whole_with(text, number::ParseFlags::for_syntax(context.numbers))
        })
    };
    let range_error = release == tcl_dialect::TclVersion::V8_4
        && std::str::from_utf8(&original).is_ok_and(|text| {
            number::is_out_of_range_double(text, tcl_dialect::NumberSyntax::Tcl84)
        });
    let description = match parsed {
        Some(Number::Double(double)) if double.is_nan() => {
            errors::OperandDesc::NonNumericFloatingPointValue
        }
        Some(Number::Double(double)) if range_error => {
            errors::OperandDesc::for_c84_range_error(double)
        }
        Some(Number::Double(_)) => errors::OperandDesc::FloatingPointValue,
        Some(Number::Nan { .. }) => errors::OperandDesc::NonNumericFloatingPointValue,
        _ if release >= tcl_dialect::TclVersion::V9_0
            && tcl_syntax::list::split_list_bytes_in(
                &original,
                context.lexer_grammar.list_parse,
                context.lexer_grammar.escapes,
            )
            .is_ok_and(|elements| elements.len() > 1) =>
        {
            errors::OperandDesc::List
        }
        _ => errors::OperandDesc::NonNumericString,
    };
    // Before 9.0 the message never spells the operand, but its text still
    // decides the description (`empty string`, `invalid octal number`).
    let message =
        if release >= tcl_dialect::TclVersion::V9_0 && description != errors::OperandDesc::List {
            match tcl_syntax::raw_string::RawString::from_bytes(Rc::clone(&original)).unicode() {
                Ok(text) => {
                    errors::illegal_operand_message(description, &text, side, operator, release)
                        .into_bytes()
                }
                Err(error) => return error.into(),
            }
        } else {
            errors::illegal_operand_message_bytes(
                description,
                &original,
                side,
                operator.as_bytes(),
                release,
            )
        };
    TclError::with_error_code(
        message,
        errors::illegal_operand_error_code(
            description,
            &String::from_utf8_lossy(&original),
            release,
        ),
    )
}

fn selected_operand_error_code(dialect: NumericContext<'_>, mut error: TclError) -> TclError {
    if let Err(first) = dialect.current() {
        return first;
    }
    let policy = dialect.simulation.map_or_else(
        || dialect.expression_operand_error_code_policy(),
        |simulation| match simulation {
            tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core => {
                Some(tcl_registry::native_numeric_error::NativeExpressionOperandErrorCodePolicy::None)
            }
        },
    );
    if error.is_guest_error()
        && let Some(policy) = policy
    {
        let existing = match error.error_code_bytes() {
            Ok(code) => code,
            Err(cause) => return cause.into(),
        };
        if let Err(cause) = error
            .set_error_code(policy.select_invalid_type_code(existing.as_deref().unwrap_or(b"NONE")))
        {
            return cause.into();
        }
    }
    match dialect.current() {
        Ok(()) => error,
        Err(first) => first,
    }
}

#[cfg(test)]
pub(crate) fn boolean_operand_error<'a>(
    dialect: impl Into<NumericContext<'a>>,
    error: TclError,
) -> TclError {
    let dialect = dialect.into();
    if !error.is_guest_error()
        || dialect.simulation.is_some()
        || dialect.expression_operand_error_presentation().is_some()
    {
        error
    } else {
        let message = match error.message_bytes() {
            Ok(bytes) => bytes,
            Err(cause) => return cause.into(),
        };
        let error = boolean_context_err(message.to_vec());
        let code = match error.error_code_bytes() {
            Ok(code) => code,
            Err(cause) => return cause.into(),
        };
        if code.as_deref() == Some(errors::NAN_CODE.as_bytes()) {
            error
        } else {
            selected_operand_error_code(dialect, error)
        }
    }
}

pub(crate) fn authored_boolean<'a>(
    dialect: impl Into<NumericContext<'a>>,
    value: &Value,
) -> Result<bool, TclError> {
    let dialect = dialect.into();
    value.check_native_header()?;
    if let Some(simulation) = dialect.simulation {
        let stage = tcl_syntax::logical_numeric_simulation::LogicalBooleanInputStage::NumericTruth;
        if let Some(integer) = value.integer_representation() {
            return Ok(simulation
                .current_number_boolean(Number::Int(integer), stage)
                .value());
        }
        if let Some(double) = value.double_representation() {
            return Ok(simulation
                .current_number_boolean(Number::Double(double), stage)
                .value());
        }
        return simulation
            .parse_boolean(&value.string_bytes(), stage)
            .map(|value| value.value())
            .map_err(|_| {
                selected_operand_error_code(
                    dialect,
                    tcl_syntax::value::ValueError::NotBooleanBytes(value.string_bytes().to_vec())
                        .into(),
                )
            });
    }
    Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
        "explicit authored Boolean simulation",
    )
    .into())
}

fn native_operand(
    dialect: NumericContext<'_>,
    value: &Value,
    side: errors::OperandSide,
    op: BinOp,
) -> Result<Num, TclError> {
    native_num(dialect, value).map_err(|error| {
        if let Err(first) = dialect.current_for_value(value) {
            return first;
        }
        if error.is_host() {
            return error;
        }
        let stage = if is_int_only(op) {
            tcl_registry::native_numeric_error::NativeExpressionOperandStage::Integer
        } else {
            tcl_registry::native_numeric_error::NativeExpressionOperandStage::FloatingPoint
        };
        let message = match error.message_bytes() {
            Ok(message) => message,
            Err(cause) => return cause.into(),
        };
        let error = if let Some(error) = expression_operand_error(dialect, stage, value) {
            error
        } else if message.as_ref() == b"integer value too large to represent" {
            // C8.4's `IllegalExprOperandType` sets the overflow code itself.
            return TclError::with_error_code(
                format!(
                    "can't use integer value too large to represent as operand of \"{}\"",
                    op.as_str()
                ),
                b"ARITH IOVERFLOW {integer value too large to represent}",
            );
        } else {
            context_operand_error(dialect, value, side, op.as_str())
        };
        selected_operand_error_code(dialect, error)
    })
}

/// The error a unary operator raises for an operand that is not an integer.
fn unary_integer_operand_error(
    dialect: NumericContext<'_>,
    value: &Value,
    op: UnaryOp,
) -> TclError {
    expression_operand_error(
        dialect,
        tcl_registry::native_numeric_error::NativeExpressionOperandStage::Integer,
        value,
    )
    .unwrap_or_else(|| {
        selected_operand_error_code(
            dialect,
            context_operand_error(dialect, value, errors::OperandSide::Unary, op.as_str()),
        )
    })
}

/// Unary arithmetic under the entered native command's selected policy.
pub(crate) fn unary_in<'a>(
    dialect: impl Into<NumericContext<'a>>,
    op: UnaryOp,
    value: &Value,
) -> Result<Value, TclError> {
    let dialect = dialect.into();
    let policy = dialect
        .arithmetic()
        .ok_or_else(|| TclError::new("native arithmetic policy is not selected"))?;
    if matches!(op, UnaryOp::Not | UnaryOp::WordNot) {
        // INST_LNOT reports a failed boolean conversion through
        // IllegalExprOperandType, not the getter's own wording.
        return authored_boolean(dialect, value)
            .map(|truth| native_boolean_result(dialect, !truth))
            .map_err(|error| {
                if error.is_host() {
                    return error;
                }
                if let Err(first) = dialect.current_for_value(value) {
                    return first;
                }
                expression_operand_error(
                    dialect,
                    tcl_registry::native_numeric_error::NativeExpressionOperandStage::Boolean,
                    value,
                )
                .unwrap_or_else(|| {
                    selected_operand_error_code(
                        dialect,
                        context_operand_error(
                            dialect,
                            value,
                            errors::OperandSide::Unary,
                            op.as_str(),
                        ),
                    )
                })
            });
    }
    let number = native_num(dialect, value).map_err(|error| {
        if let Err(first) = dialect.current_for_value(value) {
            return first;
        }
        if error.is_host() {
            return error;
        }
        if let Some(presenter) = dialect.expression_operand_error_presentation()
            && matches!(op, UnaryOp::Pos | UnaryOp::Neg)
            && dialect.boolean(value).is_ok()
            && let Some(message) = presenter.non_numeric_unary_message(op)
        {
            return TclError::with_error_code(message, presenter.error_code());
        }
        if let Err(first) = dialect.current_for_value(value) {
            return first;
        }
        let stage = if op == UnaryOp::BitNot {
            tcl_registry::native_numeric_error::NativeExpressionOperandStage::Integer
        } else {
            tcl_registry::native_numeric_error::NativeExpressionOperandStage::Boolean
        };
        expression_operand_error(dialect, stage, value).unwrap_or_else(|| {
            selected_operand_error_code(
                dialect,
                context_operand_error(dialect, value, errors::OperandSide::Unary, op.as_str()),
            )
        })
    })?;
    if matches!(op, UnaryOp::Pos | UnaryOp::Neg)
        && matches!(number, Num::Dbl(double) if double.is_nan())
    {
        return Err(selected_operand_error_code(
            dialect,
            context_operand_error(dialect, value, errors::OperandSide::Unary, op.as_str()),
        ));
    }
    if op == UnaryOp::BitNot && matches!(number, Num::Dbl(_)) {
        return Err(unary_integer_operand_error(dialect, value, op));
    }
    if policy == tcl_dialect::NativeArithmetic::TclBignum {
        return unary(op, &native_num_value(number));
    }
    match number {
        Num::Int(integer) => {
            let result = tcl_syntax::expr::wide::unary(policy, op, integer)
                .map_err(|error| native_wide_error(dialect, policy, error))?;
            if let Some(cache) = dialect
                .native_scalar_getter_protocol()
                .and_then(|protocol| {
                    protocol.expression_integer_result84(result, &[value.native_scalar_cache()])
                })
            {
                Value::from_native_scalar_cache(cache, None, dialect.dialect).map_err(Into::into)
            } else {
                Ok(Value::int(result))
            }
        }
        Num::Dbl(value) if op == UnaryOp::Neg => Ok(Value::double(-value)),
        Num::Dbl(value) if op == UnaryOp::Pos => Ok(Value::double(value)),
        _ => Err(unary_integer_operand_error(dialect, value, op)),
    }
}

pub(super) fn native_literal(dialect: NumericContext<'_>, text: &str) -> Result<Value, TclError> {
    let policy = dialect
        .arithmetic()
        .ok_or_else(|| TclError::new("native arithmetic policy is not selected"))?;
    if policy == tcl_dialect::NativeArithmetic::TclBignum {
        return Ok(
            match number::parse_whole_with(text, number::ParseFlags::for_syntax(dialect.numbers)) {
                Some(Number::Int(value)) => Value::int(value),
                Some(Number::Double(value)) => Value::double(value),
                _ => Value::string(text),
            },
        );
    }
    match native_num(dialect, &Value::string(text)) {
        Ok(Num::Int(value)) => Ok(Value::int(value)),
        Ok(Num::Dbl(value)) if !value.is_nan() => Ok(Value::double(value)),
        Err(error) if error.is_host() => Err(error),
        Err(error) => match error.message_bytes() {
            Ok(message) if message.as_ref() == b"integer value too large to represent" => {
                Err(error)
            }
            Ok(_) => Ok(Value::string(text)),
            Err(cause) => Err(cause.into()),
        },
        _ => Ok(Value::string(text)),
    }
}

pub(super) fn compare_values_numeric_checked_in(
    dialect: NumericContext<'_>,
    a: &Value,
    b: &Value,
) -> Result<Option<NumericCompare>, TclError> {
    dialect.arithmetic().ok_or_else(|| {
        TclError::from(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)
    })?;
    let numeric = |value| match native_num(dialect, value) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.is_host() => Err(error),
        Err(_) => Ok(None),
    };
    let Some(left) = numeric(a)? else {
        return Ok(None);
    };
    let Some(right) = numeric(b)? else {
        return Ok(None);
    };
    if dialect.arithmetic() == Some(tcl_dialect::NativeArithmetic::TclBignum) {
        return Ok(compare_values_numeric(
            &native_num_value(left),
            &native_num_value(right),
        ));
    }
    Ok(Some(match (left, right) {
        (Num::Int(a), Num::Int(b)) => NumericCompare::Ordered(a.cmp(&b)),
        (Num::Int(a), Num::Dbl(b)) => {
            NumericCompare::from_partial(number::compare_int_double(i128::from(a), b))
        }
        (Num::Dbl(a), Num::Int(b)) => NumericCompare::from_partial(
            number::compare_int_double(i128::from(b), a).map(Ordering::reverse),
        ),
        (Num::Dbl(a), Num::Dbl(b)) => NumericCompare::from_partial(a.partial_cmp(&b)),
        _ => return Ok(None),
    }))
}

/// Native comparisons preserve wide conversion before comparing numeric values.
pub(crate) fn compare_in<'a>(
    dialect: impl Into<NumericContext<'a>>,
    op: BinOp,
    a: &Value,
    b: &Value,
) -> Result<bool, TclError> {
    let dialect = dialect.into();
    let numeric = if string_comparison(op) {
        None
    } else {
        compare_values_numeric_checked_in(dialect, a, b)?
    };
    compare_with_numeric_using(
        op,
        a,
        b,
        numeric,
        dialect.characters,
        dialect.original.map(|_| dialect),
    )
}
