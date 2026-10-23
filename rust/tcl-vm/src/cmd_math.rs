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

//! `expr` math functions, registered as `tcl::mathfunc::*` (the names the
//! compiler invokes and `ExprEval::call` routes to).

use num_bigint::BigInt;
use num_traits::Signed;
use tcl_runtime_api::Completion;
use tcl_syntax::expr::mathfunc::{
    IntWidth, NativeMathProtocol, NumValue, try_dispatch_with_backend_protocol,
};
use tcl_syntax::number::{self, Number};

use crate::command::err_with_code;
use crate::error::TclError;
use crate::expr::NumericContext;
use crate::interp::native_operation_currency::NativeOperationScope;
use crate::interp::{Vm, err, ok};
use crate::value::Value;
use std::rc::Rc;

type Stages<'a> = Option<NumericContext<'a>>;

fn quiet_guest<T>(reached: Result<T, TclError>) -> Result<Option<T>, TclError> {
    match reached {
        Ok(value) => Ok(Some(value)),
        Err(TclError::Guest(_)) => Ok(None),
        Err(error @ TclError::Host(_)) => Err(error),
    }
}

fn math_current(value: &Value, original: Stages<'_>) -> Result<(), TclError> {
    original.map_or(Ok(()), |context| context.current_for_value(value))
}

fn math_text(value: &Value, original: Stages<'_>) -> Result<Rc<str>, TclError> {
    if original.is_none() {
        return Ok(value.to_str());
    }
    value.numeric_conversion_text(original, tcl_syntax::value::ValueError::NotDoubleBytes)
}

/// Coerce `v` to a double for the classification predicates (`isnan` /
/// `isunordered` / …), which deliberately accept a literal `NaN` — inspecting
/// NaN/Inf is their purpose — even though a bare `NaN` is a domain error as an
/// ordinary operand.
/// `fpclassify floatValue` — the top-level command (`tclBasic.c`
/// `FloatClassifyObjCmd`): classify a number as zero / subnormal / normal /
/// infinite / nan. An integer coerces to its double value first (so `0` is
/// `zero`, any other integer `normal`); a non-number errors.
fn cmd_fpclassify(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [value] = args else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"fpclassify floatValue\"",
        );
    };
    let reached = crate::expr::with_original_numeric_context(vm, |context| {
        let number = num_or_nan_using(value, Some(context))?;
        Ok(Value::string(match number.classify() {
            std::num::FpCategory::Nan => "nan",
            std::num::FpCategory::Infinite => "infinite",
            std::num::FpCategory::Zero => "zero",
            std::num::FpCategory::Subnormal => "subnormal",
            std::num::FpCategory::Normal => "normal",
        }))
    });
    match reached {
        Ok(value) => ok(value),
        Err(error) => crate::command::completion_from_tcl_error(vm, error),
    }
}

fn num_or_nan_using(value: &Value, original: Stages<'_>) -> Result<f64, TclError> {
    if let Some(number) = quiet_guest(value.as_double_with_stages(original))? {
        return Ok(number);
    }
    let text = math_text(value, original)?;
    match Value::numeric_conversion_parse(text.trim(), original) {
        Some(Number::Nan { .. }) => Ok(f64::NAN),
        Some(Number::Big { .. }) => Ok(crate::expr::value_as_bigint_with_stages(value, original)?
            .map_or(f64::NAN, |integer| crate::expr::big_to_f64(&integer))),
        _ => Err(TclError::new(format!("expected number but got \"{text}\""))),
    }
}

/// The message and `errorCode` C raises (`tclExecute.c`, errno `EDOM`) when a
/// math function's argument is out of range — `sqrt(-1)`, `acos(2)`, `fmod(x,0)`,
/// … . `isqrt` of a negative reuses the same code with its own message.
const DOMAIN_MSG: &str = "domain error: argument not in valid range";
const DOMAIN_CODE: &str = "ARITH DOMAIN {domain error: argument not in valid range}";

/// The `-errorcode` of a math function's non-numeric argument
/// (`expected number but got "x"`), from 8.5.
const VALUE_NUMBER_CODE: &str = "TCL VALUE NUMBER";

fn domain_err() -> Completion<Value> {
    err_with_code(DOMAIN_MSG, DOMAIN_CODE)
}

/// A shared math-function refusal as a VM completion, carrying C's verbatim
/// message and `-errorcode`: `ARITH IOVERFLOW` for an infinity
/// reaching an integer conversion, `TCL VALUE DOUBLE NAN` for a NaN operand,
/// `ARITH DOMAIN` otherwise. `Abstain` cannot occur here — the VM's backend
/// has an arbitrary-precision rung and its release is resolved — so it falls
/// back to the generic domain error, as do the two refusals the caller words
/// itself.
fn math_func_err(e: tcl_syntax::expr::mathfunc::MathFuncError) -> Completion<Value> {
    let message = e.message();
    if message.is_empty() {
        return domain_err();
    }
    err_with_code(message, e.error_code())
}

/// A [`num_or_nan_using`] failure as a completion, with C's `-errorcode`. A
/// message-only completion would have the right text for a NaN and an
/// infinity but leave `errorCode` as `NONE`.
fn num_err(message: String) -> Completion<Value> {
    use tcl_syntax::expr::errors;
    if message == errors::NAN_MESSAGE {
        err_with_code(message, errors::NAN_CODE)
    } else if message == errors::IOVERFLOW_MESSAGE {
        err_with_code(message, errors::IOVERFLOW_CODE)
    } else if message.starts_with("expected ") {
        release_err_with_code(message, VALUE_NUMBER_CODE)
    } else {
        err(message)
    }
}

fn shared_math_using(
    name: &str,
    args: &[Value],
    int_width: IntWidth,
    protocol: NativeMathProtocol,
    numbers: tcl_dialect::NumberSyntax,
    original: Stages<'_>,
) -> Result<Completion<Value>, TclError> {
    let Some(spec) = tcl_syntax::expr::mathfunc::spec(name) else {
        return Ok(err(format!(
            "invalid command name \"tcl::mathfunc::{name}\""
        )));
    };
    let min = usize::from(spec.arity.min);
    if args.len() < min
        || spec
            .arity
            .max
            .is_some_and(|maximum| args.len() > usize::from(maximum))
    {
        return Ok(arity_err(name, args.len() < min));
    }
    let operands = args
        .iter()
        .map(|value| {
            math_current(value, original)?;
            if protocol == NativeMathProtocol::Jim084 {
                return jim_math_operand_using(value, numbers, original);
            }
            if let Some(number) = value.double_representation() {
                return Ok(NumValue::Float(number));
            }
            if let Some(integer) = quiet_guest(value.as_int_with_stages(original))? {
                return Ok(NumValue::Int(integer));
            }
            if let Some(integer) = crate::expr::value_as_bigint_with_stages(value, original)? {
                return Ok(NumValue::Big(integer));
            }
            if let Some(number) = quiet_guest(value.as_double_with_stages(original))? {
                return Ok(NumValue::Float(number));
            }
            let text = math_text(value, original)?;
            if matches!(
                Value::numeric_conversion_parse(text.trim(), original),
                Some(Number::Nan { .. })
            ) {
                return Ok(NumValue::Float(f64::NAN));
            }
            // 8.4 has one message and no code; from 8.5 the reader's own
            // words with `TCL VALUE NUMBER`.
            let (message, code) = tcl_syntax::expr::mathfunc::non_numeric_argument_error(
                name,
                &text,
                tcl_syntax::expr::errors::ambient_release(),
            );
            Err(TclError::Guest(err_with_code(message, code)))
        })
        .collect::<Result<Vec<_>, TclError>>()?;
    Ok(
        match try_dispatch_with_backend_protocol(name, &operands, int_width, protocol) {
            Ok(NumValue::Int(integer)) => ok(Value::int(integer)),
            Ok(NumValue::Big(integer)) => ok(crate::expr::big_value(&integer)),
            Ok(NumValue::Float(number)) => ok(Value::double(number)),
            Err(error) => math_func_err(error),
        },
    )
}

fn jim_math_operand_using(
    value: &Value,
    numbers: tcl_dialect::NumberSyntax,
    original: Stages<'_>,
) -> Result<NumValue<BigInt>, TclError> {
    math_current(value, original)?;
    if value.existing_string_representation().is_none()
        && let Some(double) = value.double_representation()
    {
        return Ok(NumValue::Float(double));
    }
    let text = math_text(value, original)?;
    let parsed = number::parse_whole_with(text.trim(), number::ParseFlags::for_syntax(numbers))
        .and_then(|parsed| tcl_syntax::expr::mathfunc::jim_numeric_operand(&parsed))
        .ok_or_else(|| TclError::Guest(err(format!("expected number but got \"{text}\""))))?;
    match (&parsed, original) {
        (NumValue::Int(integer), Some(original)) => original.cache(value, Number::Int(*integer))?,
        (NumValue::Float(double), Some(original)) => {
            original.cache(value, Number::Double(*double))?;
        }
        (NumValue::Int(integer), None) => value.cache_integer_representation(*integer),
        (NumValue::Float(double), None) => value.cache_double_representation(*double),
        (NumValue::Big(_), _) => {
            unreachable!("Jim numeric operand has no arbitrary-precision rung")
        }
    }
    math_current(value, original)?;
    Ok(parsed)
}

/// The one builtin behind every `tcl::mathfunc::NAME`.
///
/// The invoked word's tail selects the function, exactly as
/// `runtime/rust`'s `cmd_mathfunc::mathfunc` reads it off `argv[0]` — so a
/// single fn pointer serves all of them and the registration loop below can
/// be driven straight off the shared name table. Both the ordinary command
/// path and the 8.4 fixed-table path
/// ([`Vm::invoke_fixed_math_builtin`](crate::interp::Vm)) set the invoked
/// name before calling, and the fixed-table path sets the *canonical*
/// registry key, so the tail is the function's own spelling either way.
///
/// Most functions fall through to [`shared_math_using`], which drives
/// `tcl_syntax::expr::mathfunc::dispatch_with_backend` — the same shared
/// implementation `expr` itself uses. The integer conversions
/// (`int`/`wide`/`entier`/`round`/`isqrt`) are shared arms too: the shared
/// seam has an exact float-to-bignum operation (`BigIntOps::from_f64_trunc`),
/// so `int(1e300)` and `round(1e300)` keep their exact answers there.
///
/// The arms named here are the ones whose VM bodies are deliberately *not*
/// the shared ones: `abs` takes an i128 fast path before the bignum rung,
/// `double` is the VM's own value coercion, `bool` accepts a boolean *word*
/// operand (`bool(tru)` is `1`, which `shared_math_using`'s numeric operand
/// conversion would refuse), and `rand`/`srand` carry interpreter state.
fn m_mathfunc(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(identity) = vm.invoked_builtin_identity() else {
        return vm.refuse_host_command(
            "math handler has no selected stock implementation identity".into(),
        );
    };
    let Some(name) = tcl_registry::mathfunc::global_command_bare_name(identity).map(str::to_owned)
    else {
        return vm.refuse_host_command(
            "math handler stock identity is not a direct global math function".into(),
        );
    };
    let int_width = IntWidth::for_tcl_version(vm.runtime_version());
    let dialect = vm.native_invocation_dialect();
    if dialect.scalar_numeric_input_policy().is_none() && vm.authored_numeric_context().is_none() {
        return vm.refuse_host_command(
            "logical math evaluation requires an explicit numeric simulation provider".into(),
        );
    }
    let protocol = tcl_registry::mathfunc::native_math_protocol(dialect)
        .expect("native math handler dispatch must retain its selected protocol");
    if vm.authored_numeric_context().is_some() {
        let mut completion = logical_mathfunc(vm, &name, args, int_width, protocol);
        completion.result = completion.result.with_native_double_format(dialect);
        return completion;
    }
    let scope = match NativeOperationScope::enter_native(vm) {
        Ok(scope) => scope,
        Err(error) => {
            return crate::command::completion_from_tcl_error(
                vm,
                TclError::from_execution_failure(error),
            );
        }
    };
    let reached = if name == "rand" {
        Ok(m_rand(vm, args))
    } else if name == "srand" {
        Ok(m_srand(vm, args))
    } else {
        crate::expr::with_original_numeric_context(vm, |context| {
            if protocol == NativeMathProtocol::Jim084 {
                return shared_math_using(
                    &name,
                    args,
                    int_width,
                    protocol,
                    dialect.numbers,
                    Some(context),
                );
            }
            match name.as_str() {
                "abs" => m_abs_using(context, args),
                "double" => m_double_using(args, Some(context)),
                "bool" => m_bool_using(args, Some(context)),
                name => match m_integer_conversion_using(name, args, int_width, Some(context))? {
                    Some(completion) => Ok(completion),
                    None => shared_math_using(
                        name,
                        args,
                        int_width,
                        protocol,
                        dialect.numbers,
                        Some(context),
                    ),
                },
            }
        })
    };
    if let Err(first) = scope.check_native(vm) {
        drop(reached);
        let _ = scope.check_native(vm);
        return crate::command::completion_from_tcl_error(
            vm,
            TclError::from_execution_failure(first),
        );
    }
    let mut completion = match reached {
        Ok(completion) => completion,
        Err(error) => return crate::command::completion_from_tcl_error(vm, error),
    };
    completion.result = completion.result.with_native_double_format(dialect);
    if let Err(first) = scope.check_native(vm) {
        drop(completion);
        let _ = scope.check_native(vm);
        return crate::command::completion_from_tcl_error(
            vm,
            TclError::from_execution_failure(first),
        );
    }
    completion
}

fn logical_mathfunc(
    vm: &mut Vm,
    name: &str,
    args: &[Value],
    int_width: IntWidth,
    protocol: NativeMathProtocol,
) -> Completion<Value> {
    if name == "rand" {
        return m_rand(vm, args);
    }
    if name == "srand" {
        return m_srand(vm, args);
    }
    if name == "bool" {
        return m_bool(vm, args);
    }
    let Some(context) = vm.authored_numeric_context() else {
        return vm.refuse_host_command("selected authored numeric policy is unavailable".into());
    };
    let mut prepared = Vec::with_capacity(args.len());
    for argument in args {
        match crate::expr::numeric_value_in(context, argument) {
            Ok(value) => prepared.push(value),
            Err(error) => return crate::command::completion_from_tcl_error(vm, error),
        }
    }
    match shared_math_using(name, &prepared, int_width, protocol, context.numbers, None) {
        Ok(completion) => completion,
        Err(error) => crate::command::completion_from_tcl_error(vm, error),
    }
}

pub(crate) fn invoke_authored_function(
    vm: &mut Vm,
    name: &str,
    args: &[Value],
) -> Completion<Value> {
    let Some(provider) = vm.authored_math_provider() else {
        return vm.refuse_host_command("authored fixed-function provider is unavailable".into());
    };
    let Some(arity) = tcl_registry::authored_math_functions::arity(provider, name) else {
        return err(format!("unknown math function \"{name}\""));
    };
    if args.len() != arity {
        return err(format!(
            "too {} arguments for math function",
            if args.len() < arity { "few" } else { "many" }
        ));
    }
    if name == "rand" {
        return vm.authored_math_random(None);
    }
    if name == "srand" {
        return vm.authored_math_random(args.first());
    }
    let mut completion =
        logical_mathfunc(vm, name, args, IntWidth::Windowed, NativeMathProtocol::Tcl);
    completion.result = vm.format_authored_math_result(completion.result);
    completion
}

/// Whether `v` is already an *integer* object — the operand class C's
/// `Tcl_GetNumberFromObj` reports as `TCL_NUMBER_INT` or `TCL_NUMBER_BIG`.
/// A double (even an integral one such as `4.0`) is not: C converts it, so it
/// has no integer object to hand back.
fn is_integer_value_using(value: &Value, original: Stages<'_>) -> Result<bool, TclError> {
    if quiet_guest(value.as_int_with_stages(original))?.is_some() {
        return Ok(true);
    }
    Ok(matches!(
        Value::numeric_conversion_parse(math_text(value, original)?.trim(), original),
        Some(Number::Int(_) | Number::Big { .. })
    ))
}

/// The object-preserving integer path shared by `int`, `wide`, and `entier`
/// (#1936), the same seam `runtime/rust`'s `cmd_mathfunc` takes.
///
/// C's `ExprEntierFunc` hands an already-integral operand **back as the same
/// object**, so the result keeps the operand's own string rep:
/// `::tcl::mathfunc::entier 0x10` is `0x10`, not `16` (tclsh 8.5-9.1). Routing
/// it through the shared numeric dispatch instead rebuilds a canonical decimal
/// and loses that rep. Which of the three preserves and which windows is not
/// decided here: it is
/// [`tcl_syntax::expr::mathfunc::integer_conversion`], the one owner both
/// engines read, over the release's [`IntWidth`] axis. A windowing conversion
/// has to build a new integer, so only the unbounded one can preserve the
/// object.
///
/// `None` falls through to [`shared_math_using`]: a float operand (which C genuinely
/// converts, and whose exact float-to-bignum truncation lives in the shared
/// seam), a non-number (whose refusal wording is the shared seam's), and any
/// wrong argument count (whose arity error is too).
///
/// `expr` itself is unaffected: it coerces its result to numeric, so
/// `expr {entier($h)}` stays `16` for `$h` of `0x10` exactly as C Tcl does —
/// the preservation is visible only through the command form.
fn m_integer_conversion_using(
    name: &str,
    args: &[Value],
    int_width: IntWidth,
    original: Stages<'_>,
) -> Result<Option<Completion<Value>>, TclError> {
    let Some(conversion) = tcl_syntax::expr::mathfunc::integer_conversion(name, int_width) else {
        return Ok(None);
    };
    let [value] = args else {
        return Ok(None);
    };
    if !is_integer_value_using(value, original)? {
        return Ok(None);
    }
    Ok(match conversion {
        tcl_syntax::expr::mathfunc::IntegerConversion::Window => {
            quiet_guest(value.as_wide_with_stages(original))?.map(|integer| ok(Value::int(integer)))
        }
        tcl_syntax::expr::mathfunc::IntegerConversion::Preserve => Some(ok(value.clone())),
    })
}

pub(crate) fn register(
    vm: &mut Vm,
    native: Option<tcl_registry::special_vars::NativeBootstrapProtocol>,
) {
    // `::tcl::mathfunc` is a real namespace in C Tcl, so a user
    // `proc tcl::mathfunc::square {x} {…}` (TIP 232's custom-function
    // mechanism) must find it existing — declare it alongside the builtin
    // registrations (which only create flat command-table keys).
    vm.declare_namespace("tcl::mathfunc");
    // A measured native constructor roster precedes library procedure births.
    // Other constructors retain the shared implementation inventory.
    let names = native
        .and_then(tcl_registry::special_vars::NativeBootstrapProtocol::core_mathfunc_roster)
        .map_or_else(
            || {
                tcl_syntax::expr::mathfunc::all()
                    .into_iter()
                    .map(|spec| spec.name)
                    .collect::<Vec<_>>()
            },
            <[_]>::to_vec,
        );
    for name in names {
        vm.register_stock_builtin(&format!("tcl::mathfunc::{name}"), m_mathfunc);
    }
    // `fpclassify` is a top-level command, not a math function.
    vm.register_stock_builtin("fpclassify", cmd_fpclassify);
}

fn one<'a>(args: &'a [Value], name: &str) -> Result<&'a Value, Completion<Value>> {
    match args {
        [x] => Ok(x),
        _ => Err(arity_err(name, args.is_empty())),
    }
}

/// A math function called with the wrong number of arguments, worded for
/// the release (`too few`/`not enough`), with `TCL WRONGARGS` from 8.6.
fn arity_err(name: &str, short: bool) -> Completion<Value> {
    let release = tcl_syntax::expr::errors::ambient_release();
    let words = if short {
        release.math_arity_shortfall()
    } else {
        "too many arguments"
    };
    release_err_with_code(
        format!("{words} for math function \"{name}\""),
        "TCL WRONGARGS",
    )
}

/// [`err_with_code`] where the emulated release sets `code`
/// ([`tcl_dialect::TclVersion::sets_error_code`]); a bare error otherwise.
fn release_err_with_code(message: impl AsRef<[u8]>, code: &str) -> Completion<Value> {
    if tcl_syntax::expr::errors::ambient_release().sets_error_code(code.as_bytes()) {
        err_with_code(message, code)
    } else {
        err(message)
    }
}

fn m_abs_using(
    context: crate::expr::NumericContext<'_>,
    args: &[Value],
) -> Result<Completion<Value>, TclError> {
    let value = match one(args, "abs") {
        Ok(value) => value,
        Err(guest) => return Ok(guest),
    };
    let original = Some(context);
    math_current(value, original)?;
    let dialect = context.dialect;
    if dialect.arithmetic() == Some(tcl_dialect::NativeArithmetic::Tcl84Wide) {
        if !context.prepare_integer84(value)? {
            return Ok(num_err(
                "argument to math function didn't have numeric value".to_owned(),
            ));
        }
        if let Some(integer) = value.integer_representation() {
            let Some(result) = integer.checked_abs() else {
                return Ok(num_err(
                    tcl_syntax::expr::errors::IOVERFLOW_MESSAGE.to_owned(),
                ));
            };
            let cache = dialect
                .native_scalar_getter_protocol()
                .and_then(|protocol| {
                    protocol.expression_integer_result84(result, &[value.native_scalar_cache()])
                })
                .expect("selected C84 abs result");
            return Value::from_native_scalar_cache(cache, None, dialect)
                .map(ok)
                .map_err(Into::into);
        }
    }
    if let Some(number) = value.double_representation() {
        return Ok(if number.is_nan() {
            num_err(tcl_syntax::expr::errors::NAN_MESSAGE.to_string())
        } else {
            ok(Value::double(number.abs()))
        });
    }
    if let Some(integer) = quiet_guest(value.as_int_with_stages(original))? {
        return Ok(ok(crate::expr::int_value(i128::from(integer).abs())));
    }
    if let Some(integer) = value.as_i128_with_stages(original)?
        && integer != i128::MIN
    {
        return Ok(ok(crate::expr::int_value(integer.abs())));
    }
    if let Some(integer) = crate::expr::value_as_bigint_with_stages(value, original)? {
        return Ok(ok(crate::expr::big_value(&integer.abs())));
    }
    match num_or_nan_using(value, original) {
        Ok(number) if number.is_nan() => {
            Ok(num_err(tcl_syntax::expr::errors::NAN_MESSAGE.to_string()))
        }
        Ok(number) => Ok(ok(Value::double(number.abs()))),
        Err(error @ TclError::Host(_)) => Err(error),
        Err(TclError::Guest(guest)) => Ok(guest),
    }
}

fn m_double_using(args: &[Value], original: Stages<'_>) -> Result<Completion<Value>, TclError> {
    let value = match one(args, "double") {
        Ok(value) => value,
        Err(guest) => return Ok(guest),
    };
    match value.as_double_with_stages(original) {
        Ok(number) if number.is_nan() => {
            Ok(num_err(tcl_syntax::expr::errors::NAN_MESSAGE.to_string()))
        }
        Ok(number) => Ok(ok(Value::double(number))),
        Err(error @ TclError::Host(_)) => Err(error),
        Err(error @ TclError::Guest(_)) => {
            if let Some(integer) = crate::expr::value_as_bigint_with_stages(value, original)? {
                return Ok(ok(Value::double(crate::expr::big_to_f64(&integer))));
            }
            if let Some(text) = value.numeric_conversion_optional_text(original)?
                && matches!(
                    Value::numeric_conversion_parse(text.trim(), original),
                    Some(Number::Nan { .. })
                )
            {
                return Ok(num_err(tcl_syntax::expr::errors::NAN_MESSAGE.to_string()));
            }
            Err(error)
        }
    }
}

fn m_bool_using(args: &[Value], original: Stages<'_>) -> Result<Completion<Value>, TclError> {
    let value = match one(args, "bool") {
        Ok(value) => value,
        Err(guest) => return Ok(guest),
    };
    value
        .as_bool_with_stages(original)
        .map(|boolean| ok(Value::bool(boolean)))
}

fn m_bool(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let value = match one(args, "bool") {
        Ok(value) => value,
        Err(guest) => return guest,
    };
    match tcl_syntax::value::ValueOps::as_bool(vm, value) {
        Ok(boolean) => ok(Value::bool(boolean)),
        Err(error) => crate::command::completion_from_tcl_error(vm, error.into()),
    }
}

/// `srand(seed)` — reseed the `expr rand()` generator and return its first draw.
/// C (`ExprSrandFunc`) coerces the argument to a wide integer (falling back to
/// truncating a double), installs it as the seed, then tail-calls `rand()`; so
/// `srand` is deterministic and itself yields a number in `[0, 1)`.
fn m_srand(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if let Err(refused) = vm.confine_generator("srand") {
        return refused;
    }
    let value = match one(args, "srand") {
        Ok(value) => value,
        Err(guest) => return guest,
    };
    let reached = if vm.authored_numeric_context().is_some() {
        match tcl_syntax::value::ValueOps::as_int(vm, value) {
            Ok(seed) => Ok(seed),
            Err(error) => srand_seed_failure(value, None, error.into()),
        }
    } else {
        crate::expr::with_original_numeric_context(vm, |context| {
            match value.as_wide_with_stages(Some(context)) {
                Ok(seed) => Ok(seed),
                Err(error) => srand_seed_failure(value, Some(context), error),
            }
        })
    };
    let seed = match reached {
        Ok(seed) => seed,
        Err(error) => return crate::command::completion_from_tcl_error(vm, error),
    };
    if let Some(first) = vm.refused_completion() {
        return first;
    }
    vm.rand_seed_set(seed);
    ok(Value::native_double(
        vm.rand_next(),
        vm.native_invocation_dialect(),
    ))
}

fn srand_seed_failure(
    value: &Value,
    original: Stages<'_>,
    error: TclError,
) -> Result<i64, TclError> {
    if matches!(error, TclError::Host(_)) {
        return Err(error);
    }
    let text = math_text(value, original)?;
    let floating = quiet_guest(value.as_double_with_stages(original))?.is_some();
    Err(TclError::Guest(err_with_code(
        format!("expected integer but got \"{text}\""),
        if floating {
            "TCL VALUE INTEGER"
        } else {
            "TCL VALUE NUMBER"
        },
    )))
}

/// `rand()` — the next draw from the Park–Miller minimal-standard generator, a
/// `double` in `[0, 1)`. Takes no arguments.
fn m_rand(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if let Err(refused) = vm.confine_generator("rand") {
        return refused;
    }
    if !args.is_empty() {
        return arity_err("rand", false);
    }
    ok(Value::native_double(
        vm.rand_next(),
        vm.native_invocation_dialect(),
    ))
}
