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

//! `ValueOps` for the VM — binds the portable `tcl-cmd-core` command logic to
//! the VM's `Rc<Obj>` value model.
//!
//! Value construction needs no interpreter state, so the seam is implemented
//! directly on [`Vm`] (the natural `ops` object a builtin already holds). The
//! copy-on-write asymmetry is explicit: the `Rc`-handle model cannot grow a
//! buffer in place: [`ValueOps::try_append_bytes_in_place`] returns `false`,
//! while [`ValueOps::try_list_append_in_place`] returns `Ok(false)`. Callers
//! therefore build a fresh value. The WASM runtime supports amortised in-place
//! growth through those capabilities. The VM retains exact
//! string bytes independently of a checked Unicode projection; `as_bytes`
//! uses that byte owner directly.

#[path = "value_ops/native_concat.rs"]
mod native_concat;
#[cfg(test)]
#[path = "value_ops/native_concat_tests.rs"]
mod native_concat_tests;
#[path = "value_ops/native_list_index.rs"]
mod native_list_index;

use std::rc::Rc;

use tcl_syntax::number::{self, Number, Radix};
use tcl_syntax::value::{IntegerMagnitude, ValueError, ValueOps, string_char_len};

use crate::interp::Vm;
use crate::value::Value;

/// Integer addition through the selected native numeral and arithmetic owners.
/// `dict incr` captures the same dialect before its callback borrows the VM.
pub(crate) fn int_add<'a>(
    dialect: impl Into<crate::expr::NumericContext<'a>>,
    a: Option<&Value>,
    b: &Value,
) -> Result<Value, ValueError> {
    use tcl_dialect::NativeArithmetic;
    let context = dialect.into();
    let dialect = context.dialect;
    let policy = dialect.arithmetic().ok_or(ValueError::IntegerOverflow)?;
    let input = dialect.scalar_numeric_input_policy();
    let parse = |value: &Value| {
        if let Some(simulation) = context.simulation {
            return logical_integer_number(simulation, value);
        }
        let input = input.ok_or(ValueError::ScalarNumericInputUnavailable)?;
        if let Some(number @ (Number::Int(_) | Number::Big { .. })) = value.number_representation()
        {
            return Ok(number);
        }
        let protocol = dialect
            .native_string_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let original = value
            .native_string_bytes(protocol)
            .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
        let text = std::str::from_utf8(input.input_bytes(&original))
            .map_err(|_| ValueError::NotIntegerBytes(original.to_vec()))?;
        let mut flags = number::ParseFlags::for_syntax(dialect.numbers);
        flags.integer_only = true;
        match number::parse_whole_with(text, flags) {
            Some(parsed @ (Number::Int(_) | Number::Big { .. })) => {
                if let Number::Int(integer) = parsed {
                    value.cache_integer_representation(integer);
                }
                Ok(parsed)
            }
            _ => Err(ValueError::NotIntegerBytes(original.to_vec())),
        }
    };
    let left = a.map(parse).transpose()?.unwrap_or(Number::Int(0));
    let right = parse(b)?;
    if policy != NativeArithmetic::TclBignum {
        let left = tcl_syntax::expr::wide::parsed_literal(policy, &left)
            .map_err(|_| ValueError::IntegerOverflow)?;
        let right = tcl_syntax::expr::wide::parsed_literal(policy, &right)
            .map_err(|_| ValueError::IntegerOverflow)?;
        let sum = tcl_syntax::expr::wide::binary(policy, tcl_syntax::expr::BinOp::Add, left, right)
            .map_err(|_| ValueError::IntegerOverflow)?;
        return Ok(Value::int(sum));
    }
    let to_big = |value: Number| match value {
        Number::Int(value) => num_bigint::BigInt::from(value),
        Number::Big {
            negative,
            radix,
            digits,
        } => {
            let value = num_bigint::BigInt::parse_bytes(digits.as_bytes(), radix as u32)
                .expect("number owner retains valid magnitude digits");
            if negative { -value } else { value }
        }
        Number::Double(_) | Number::Nan { .. } => unreachable!("integer-only parser"),
    };
    Ok(crate::expr::big_value(&(to_big(left) + to_big(right))))
}

/// Authenticated modern C increment operations on borrowed physical members.
pub(crate) struct VmIncrementObjects {
    dialect: tcl_registry::InvocationDialect,
    string: tcl_syntax::native_string::NativeStringProtocol,
}

impl VmIncrementObjects {
    pub(crate) fn selected(
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<Self, tcl_cmd_core::CmdError> {
        dialect
            .native_scalar_getter_protocol()
            .filter(|protocol| protocol.supports_number_getter())
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let string = dialect
            .native_string_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        Ok(Self { dialect, string })
    }
}

impl tcl_cmd_core::native_increment::NativeIncrementObjects for VmIncrementObjects {
    type Value = Value;
    type Prepared = Value;

    fn prepare(&self, original: Option<&Value>) -> Result<Value, tcl_cmd_core::CmdError> {
        Ok(match original {
            Some(value) if value.native_object_is_shared() => {
                value.duplicate_native_object_in(self.string)
            }
            Some(value) => value.clone(),
            None => Value::int(0),
        })
    }
    fn current<'a>(&self, prepared: &'a Value) -> &'a Value {
        prepared
    }
    fn probe(
        &self,
        value: &Value,
        kind: tcl_syntax::scalar_getter::NativeNumberGetterKind,
    ) -> Result<
        Result<Number, tcl_syntax::scalar_getter::NativeScalarGetterFailure>,
        tcl_cmd_core::CmdError,
    > {
        value
            .native_number_probe(self.dialect, kind)
            .map_err(Into::into)
    }
    fn integer_failure(
        &self,
        value: &Value,
        kind: tcl_syntax::scalar_getter::NativeNumberGetterKind,
    ) -> Result<tcl_cmd_core::CmdError, tcl_cmd_core::CmdError> {
        let getter = if kind == tcl_syntax::scalar_getter::NativeNumberGetterKind::Bignum {
            tcl_syntax::scalar_getter::NativeScalarGetterKind::Wide
        } else {
            tcl_syntax::scalar_getter::NativeScalarGetterKind::Int
        };
        match value.native_scalar_getter(self.dialect, getter) {
            Err(error) => Ok(error.into()),
            Ok(_) => Err(ValueError::CommandProtocolUnavailable(
                "native increment integer failure stage",
            )
            .into()),
        }
    }
    fn add(&self, current: Number, amount: Number) -> Result<Number, tcl_cmd_core::CmdError> {
        fn integer(value: Number) -> num_bigint::BigInt {
            match value {
                Number::Int(value) => value.into(),
                Number::Big {
                    negative,
                    radix,
                    digits,
                } => {
                    let value = num_bigint::BigInt::parse_bytes(digits.as_bytes(), radix as u32)
                        .expect("native numeric owner validates magnitude");
                    if negative { -value } else { value }
                }
                _ => unreachable!("increment kernel validates integers"),
            }
        }
        tcl_cmd_core::native_increment::add_integer_numbers(current, amount, |current, amount| {
            use tcl_syntax::number_tower::BigIntOps;
            let sum = integer(current).add(&integer(amount));
            Ok(sum.to_i64().map_or_else(
                || Number::Big {
                    negative: sum.is_negative(),
                    radix: Radix::Dec,
                    digits: sum.abs().to_string(),
                },
                Number::Int,
            ))
        })
    }
    fn store(&self, prepared: &mut Value, sum: Number) -> Result<(), tcl_cmd_core::CmdError> {
        prepared
            .store_native_increment_number(sum, self.dialect)
            .map_err(Into::into)
    }
    fn finish(&self, prepared: Value) -> Value {
        prepared
    }
}

/// Physical legacy receiver operations use the independently selected native handler.
pub(crate) struct VmLegacyIncrementObjects {
    dialect: tcl_registry::InvocationDialect,
    string: tcl_syntax::native_string::NativeStringProtocol,
    recipe: tcl_syntax::scalar_getter::NativeLegacyIncrementRecipe,
    jim_context: Option<Rc<crate::value::NativeJimObjectContext>>,
}
impl VmLegacyIncrementObjects {
    pub(crate) fn selected(vm: &Vm) -> Result<Self, tcl_cmd_core::CmdError> {
        let dialect = vm.native_invocation_dialect();
        let recipe = dialect
            .native_legacy_increment_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?
            .recipe();
        let string = dialect
            .native_string_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        Ok(Self {
            dialect,
            string,
            recipe,
            jim_context: if string.is_jim084() {
                Some(crate::interp::InterpState::native_jim_object_context(vm)?)
            } else {
                None
            },
        })
    }
}
impl tcl_cmd_core::native_increment::LegacyIncrementObjects for VmLegacyIncrementObjects {
    type Value = Value;
    type Prepared = Value;
    fn recipe(&self) -> tcl_syntax::scalar_getter::NativeLegacyIncrementRecipe {
        self.recipe
    }
    fn cache(
        &self,
        value: &Value,
    ) -> Result<Option<tcl_syntax::scalar_getter::NativeScalarCache>, tcl_cmd_core::CmdError> {
        Ok(value.native_scalar_cache())
    }
    fn wide(&self, value: &Value) -> Result<i64, tcl_cmd_core::CmdError> {
        if let Some(context) = &self.jim_context {
            value.bind_native_jim_context(context)?;
        }
        match value.native_scalar_getter(
            self.dialect,
            tcl_syntax::scalar_getter::NativeScalarGetterKind::Wide,
        )? {
            tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(value) => Ok(value),
            _ => Err(ValueError::ScalarNumericInputUnavailable.into()),
        }
    }
    fn prepare(&self, original: Option<&Value>) -> Result<Value, tcl_cmd_core::CmdError> {
        Ok(match original {
            Some(value) if value.native_object_is_shared() => {
                value.duplicate_native_object_in(self.string)
            }
            Some(value) => value.clone(),
            None => Value::int(0),
        })
    }
    fn current<'a>(&self, prepared: &'a Value) -> &'a Value {
        prepared
    }
    fn store(
        &self,
        prepared: &mut Value,
        cache: tcl_syntax::scalar_getter::NativeScalarCache,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        prepared
            .store_native_legacy_increment_cache(cache, self.dialect)
            .map_err(Into::into)
    }
    fn finish(&self, prepared: Value) -> Value {
        prepared
    }
}

pub(crate) struct VmLegacyIncrementAmountOps<'a>(pub(crate) &'a mut Vm);
impl tcl_cmd_core::native_increment::LegacyIncrementAmountOps for VmLegacyIncrementAmountOps<'_> {
    type Value = Value;
    fn c84_long(&mut self, original: &Value) -> Result<i64, tcl_cmd_core::CmdError> {
        match original.native_scalar_getter(
            self.0.native_invocation_dialect(),
            tcl_syntax::scalar_getter::NativeScalarGetterKind::Long,
        )? {
            tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(value) => Ok(value),
            _ => Err(ValueError::ScalarNumericInputUnavailable.into()),
        }
    }
    fn jim_wide_expression(&mut self, original: &Value) -> Result<i64, tcl_cmd_core::CmdError> {
        self.0.native_jim_wide_expression(original)
    }
}

fn logical_integer_number(
    simulation: tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation,
    value: &Value,
) -> Result<Number, ValueError> {
    simulation
        .parse_number(
            &value.string_bytes(),
            tcl_syntax::logical_numeric_simulation::LogicalNumericInputStage::Integer,
        )
        .map_err(|_| ValueError::NotIntegerBytes(value.string_bytes().to_vec()))
}

fn logical_integer(
    context: crate::expr::NumericContext<'_>,
    simulation: tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation,
    value: &Value,
) -> Result<i64, ValueError> {
    let parsed = logical_integer_number(simulation, value)?;
    let arithmetic = context
        .arithmetic()
        .ok_or(ValueError::ScalarNumericInputUnavailable)?;
    tcl_syntax::expr::wide::parsed_literal(arithmetic, &parsed)
        .map_err(|_| ValueError::IntegerOverflow)
}

impl Vm {
    fn safe_index_integer(
        &mut self,
        value: &Value,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<i64, ValueError> {
        match self.as_int(value) {
            Err(error)
                if error.native_access_refusal().is_none()
                    && dialect.index_syntax().is_some_and(|syntax| {
                        syntax.width == tcl_dialect::IndexIntegerWidth::Tcl64
                    }) =>
            {
                match value.native_scalar_cache() {
                    Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(Number::Big {
                        negative,
                        ..
                    })) => Ok(if negative { i64::MIN } else { i64::MAX }),
                    _ => Err(error),
                }
            }
            outcome => outcome,
        }
    }
}

impl ValueOps for Vm {
    type Value = Value;

    fn name_policy_protocol(&self) -> Option<tcl_syntax::naming::NamePolicyProtocol> {
        crate::interp::InterpState::name_policy_protocol(self)
    }

    fn original_option_index(
        &mut self,
        original: &Self::Value,
        words: &'static [&'static str],
        exact: bool,
        noun: &'static str,
    ) -> Result<Option<tcl_syntax::value::OriginalOptionLookup>, ValueError> {
        use tcl_syntax::value::OriginalOptionLookup;
        original.check_native_header()?;
        let dialect = self.actual_native_invocation_dialect();
        if dialect.native_jim_enum_protocol().is_some() {
            let table =
                tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(words);
            let flags = tcl_registry::native_jim_enum::NativeJimEnumFlags::options(exact);
            return Ok(Some(
                match self.native_jim_enum_from_original(
                    original,
                    &table,
                    flags,
                    Some(noun.as_bytes()),
                )? {
                    Ok(index) => OriginalOptionLookup::Index(index),
                    Err(message) => OriginalOptionLookup::Failure {
                        message: message.unwrap_or_default(),
                        error_code: b"NONE".to_vec(),
                        string_result: None,
                    },
                },
            ));
        }
        let protocol = dialect.native_index_lookup_protocol().ok_or(
            ValueError::CommandProtocolUnavailable("original static option lookup"),
        )?;
        let table =
            tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(words);
        Ok(Some(
            match self.native_index_from_original(original, &table, exact, noun)? {
                Ok(index) => OriginalOptionLookup::Index(index),
                Err(message) => {
                    let bytes = original
                        .native_string_bytes(dialect.native_string_protocol().ok_or(
                            ValueError::CommandProtocolUnavailable("original option string getter"),
                        )?)
                        .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)?;
                    OriginalOptionLookup::Failure {
                        message,
                        error_code: protocol.error_code(noun.as_bytes(), &bytes),
                        string_result: Some(tcl_syntax::native_string::NativeStringProtocol::C(
                            protocol.version(),
                        )),
                    }
                }
            },
        ))
    }

    fn same_object(&self, left: &Value, right: &Value) -> Option<bool> {
        (left.native_object_is_live() && right.native_object_is_live())
            .then(|| left.is_same_object(right))
    }

    fn native_object_snapshot(
        &self,
        value: &Value,
    ) -> Result<tcl_syntax::native_object::NativeObjectSnapshot, ValueError> {
        value.check_native_header()?;
        Ok(value.native_object_snapshot())
    }

    fn discard_native_internal_representation(&mut self, value: &Value) -> Result<(), ValueError> {
        value.discard_native_internal_representation()
    }

    fn native_unicode_units(&mut self, value: &Value) -> Result<Rc<[u32]>, ValueError> {
        let policy = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native original object Unicode issuer",
            ))?;
        value.native_unicode_units(policy.string_protocol())
    }

    fn native_unicode_string_result(
        &mut self,
        units: Rc<[u32]>,
        version: tcl_dialect::TclVersion,
    ) -> Result<Value, ValueError> {
        let dialect = self.actual_native_invocation_dialect();
        if dialect.tcl_version != Some(version) || dialect.native_error_log_protocol().is_none() {
            return Err(ValueError::CommandProtocolUnavailable(
                "native Unicode result issuer",
            ));
        }
        Value::from_native_unicode_units(units, dialect)
    }

    fn native_external_utf8_result(
        &mut self,
        bytes: &[u8],
        version: tcl_dialect::TclVersion,
    ) -> Result<Value, ValueError> {
        let dialect = self.actual_native_invocation_dialect();
        if dialect.native_string_protocol() != Some(tcl_syntax::native_string::NativeStringProtocol::C(version)) {
            return Err(ValueError::CommandProtocolUnavailable("native external UTF-8 binary result issuer"));
        }
        Value::from_native_byte_array(Rc::from(bytes), dialect)
    }

    fn concat_policy(&self) -> Option<tcl_dialect::ConcatPolicy> {
        self.actual_native_invocation_dialect().concat_policy()
    }

    fn has_list_representation(&self, value: &Value) -> bool {
        value.has_list_representation()
    }

    fn native_concat_list_shape(
        &self,
        value: &Value,
    ) -> Result<tcl_syntax::value::NativeConcatListShape, ValueError> {
        native_concat::shape(self, value)
    }
    fn native_concat_empty_list(&mut self) -> Result<Self::Value, ValueError> {
        native_concat::empty(self)
    }
    fn native_concat_copy_list(&mut self, value: &Value) -> Result<Self::Value, ValueError> {
        native_concat::copy(self, value)
    }
    fn native_concat_append_list(
        &mut self,
        target: &Value,
        source: &Value,
    ) -> Result<(), ValueError> {
        native_concat::append(self, target, source)
    }
    fn native_concat_first_bytes(
        &mut self,
        value: &Value,
    ) -> Result<Option<tcl_syntax::value::NativeConcatFirstElement<Self::Value>>, ValueError> {
        native_concat::first(self, value)
    }
    fn native_concat_string_bytes(&mut self, value: &Value) -> Result<Rc<[u8]>, ValueError> {
        native_concat::bytes(self, value)
    }
    fn native_concat_string_result(&mut self, bytes: &[u8]) -> Result<Self::Value, ValueError> {
        native_concat::string(self, bytes)
    }

    fn index_syntax(&self) -> Option<tcl_dialect::IndexSyntax> {
        self.native_invocation_dialect().index_syntax()
    }

    fn index_error_string_protocol(
        &self,
    ) -> Result<Option<tcl_syntax::native_string::NativeStringProtocol>, ValueError> {
        let materialization = self
            .actual_native_invocation_dialect()
            .native_string_materialization(None)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native index error String producer",
            ))?;
        Ok((!materialization.protocol().is_jim084()).then_some(materialization.protocol()))
    }

    fn eval_index_expression(&mut self, source: &str) -> Result<i64, ValueError> {
        use tcl_registry::runtime_expr_validation::{
            ExpressionPreparationProof, prepare_expression_witness,
            requires_fixed_function_preparation,
        };
        use tcl_syntax::expr::eval::{ExprEvalRequest, ExprEvalState, ExprEvalStep};
        let dialect = self.native_invocation_dialect();
        let context = dialect.expression_parse_context(Some(self.source_profile()));
        let expression = if self.authored_math_provider().is_some() {
            self.prepare_expression(source).map_err(|error| {
                error.native_access_refusal().map_or_else(
                    || ValueError::NotInteger(source.to_owned()),
                    ValueError::from,
                )
            })?
        } else if requires_fixed_function_preparation(&context) {
            let table = self.native_fixed_math_prerequisite();
            match prepare_expression_witness(source, &context, table.as_ref()) {
                ExpressionPreparationProof::Prepared(prepared) => prepared.tree().clone(),
                ExpressionPreparationProof::Rejected(_) => {
                    return Err(ValueError::NotInteger(source.to_owned()));
                }
                ExpressionPreparationProof::Unknown => {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native safe index expression preparation",
                    ));
                }
            }
        } else {
            tcl_syntax::expr::parser::parse_expr_with_grammar(source, &dialect.lexer_grammar)
        };
        let surface = tcl_registry::expr_surface::RuntimeExprSurface::for_profile(
            self.native_execution_profile(),
        );
        surface
            .validate(&expression)
            .map_err(|_| ValueError::NotInteger(source.to_owned()))?;
        let mut evaluation = ExprEvalState::new(expression);
        loop {
            let step = evaluation
                .advance(&mut crate::expr::ExprEval::new(self))
                .map_err(|error| {
                    error.native_access_refusal().map_or_else(
                        || ValueError::NotInteger(source.to_owned()),
                        ValueError::from,
                    )
                })?;
            match step {
                ExprEvalStep::Complete(value) => {
                    return self.safe_index_integer(&value, dialect);
                }
                ExprEvalStep::Request(ExprEvalRequest::Call { function, args, .. }) => {
                    if self.authored_math_provider().is_some() {
                        let completion =
                            crate::cmd_math::invoke_authored_function(self, &function, &args);
                        if completion.code != tcl_core_types::Code::Ok {
                            return Err(ValueError::NotInteger(source.to_owned()));
                        }
                        evaluation.resume(completion.result);
                        continue;
                    }
                    if tcl_registry::native_expression_program::expression_function_dispatch(
                        self.expression_evaluation_policy().as_ref(),
                        self.actual_native_invocation_dialect(),
                    ) != Some(tcl_registry::mathfunc::NativeMathFunctionDispatch::FixedTable)
                    {
                        return Err(ValueError::CommandProtocolUnavailable(
                            "expression fixed-function evaluation policy",
                        ));
                    }
                    let tcl_registry::expr_surface::MathFunctionCallTarget::FixedBuiltin(spec) =
                        surface.math_function_call_target(&function)
                    else {
                        return Err(ValueError::NotInteger(source.to_owned()));
                    };
                    let completion = self.invoke_fixed_math_builtin(spec, &args);
                    if completion.code != tcl_core_types::Code::Ok {
                        return Err(ValueError::NotInteger(source.to_owned()));
                    }
                    evaluation.resume(completion.result);
                }
                ExprEvalStep::Request(
                    ExprEvalRequest::Variable { .. }
                    | ExprEvalRequest::Command { .. }
                    | ExprEvalRequest::SubstitutedString { .. },
                ) => {
                    return Err(ValueError::NotInteger(source.to_owned()));
                }
            }
        }
    }

    fn new_str(&mut self, s: &str) -> Value {
        Value::string(s)
    }

    fn new_bytes(&mut self, bytes: &[u8]) -> Value {
        Value::new_native_string_bytes(bytes.to_vec())
    }

    fn new_string(&mut self, s: String) -> Value {
        Value::string(s)
    }

    fn new_int(&mut self, n: i64) -> Value {
        Value::int(n)
    }

    fn new_double(&mut self, f: f64) -> Value {
        Value::native_double(f, self.native_invocation_dialect())
    }

    fn array_existence_result(&mut self, present: bool) -> Result<Value, ValueError> {
        use tcl_registry::native_array_compilation::{
            NativeArrayExistenceResult, native_array_existence_result,
        };
        if crate::interp::InterpState::name_policy_protocol(self).is_some_and(|policy| {
            policy.authority() == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
        }) {
            return Ok(self.new_bool(present));
        }
        let dialect = self.actual_native_invocation_dialect();
        match native_array_existence_result(dialect) {
            Some(NativeArrayExistenceResult::ExecutionBooleanConstant) => self
                .native_c_execution_boolean(
                    present,
                    dialect
                        .tcl_version
                        .ok_or(ValueError::CommandProtocolUnavailable(
                            "array existence execution constant release",
                        ))?,
                ),
            Some(NativeArrayExistenceResult::FreshInteger) => Ok(self.new_bool(present)),
            None => Err(ValueError::CommandProtocolUnavailable(
                "array existence result producer",
            )),
        }
    }

    fn new_bool(&mut self, b: bool) -> Value {
        Value::bool(b)
    }

    fn new_list(&mut self, items: Vec<Value>) -> Value {
        match self
            .native_invocation_dialect()
            .native_string_materialization(Some(
            tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
        )) {
            Some(recipe) => Value::native_list_constructor(items, recipe.protocol()),
            None => Value::list(items),
        }
    }

    fn string_character_model(&self) -> Option<tcl_dialect::StringCharacterModel> {
        self.native_invocation_dialect().characters
    }

    fn new_jim_string(&mut self, bytes: &[u8], character_count: usize) -> Value {
        Value::native_jim_string(bytes, character_count)
    }

    fn jim_string_trim_result(
        &mut self,
        value: &Value,
        plan: tcl_syntax::raw_string::JimStringTrimPlan,
    ) -> Value {
        value.jim_string_trim_result(plan)
    }

    fn try_as_str(
        &mut self,
        value: &Value,
    ) -> Result<Rc<str>, tcl_syntax::raw_string::UnicodeAccessError> {
        value.try_to_str()
    }

    fn try_char_len(
        &mut self,
        value: &Value,
    ) -> Result<usize, tcl_syntax::raw_string::UnicodeAccessError> {
        match self.native_invocation_dialect().characters {
            Some(model) => value.native_character_count(
                model,
                self.native_invocation_dialect()
                    .string_length_representation(),
            ),
            None => Ok(string_char_len(
                &value.try_to_str()?,
                self.runtime_version(),
            )),
        }
    }

    fn as_bytes(&mut self, value: &Value) -> Rc<[u8]> {
        value.string_bytes()
    }

    fn native_char_len(&mut self, value: &Value) -> Result<usize, ValueError> {
        use tcl_registry::native_string_materialization::LogicalStringProvider;
        let dialect = self.native_invocation_dialect();
        let provider = Some(LogicalStringProvider::Tcl84CoreSimulation);
        let protocol = dialect
            .native_string_materialization(provider)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native string length",
            ))?
            .protocol();
        let representation = dialect
            .string_length_representation_with_provider(provider)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native string length representation",
            ))?;
        value.native_character_count_with_protocol(protocol, representation)
    }

    fn native_string_bytes(&mut self, value: &Value) -> Result<Rc<[u8]>, ValueError> {
        self.native_name_operand_bytes(value)
            .map_err(|_| ValueError::CommandProtocolUnavailable("native string materialisation"))
    }

    fn as_int(&mut self, v: &Value) -> Result<i64, ValueError> {
        let context = self.numeric_context();
        if let Some(simulation) = context.simulation {
            return logical_integer(context, simulation, v);
        }
        let dialect = context.dialect;
        if dialect.native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            v.bind_native_jim_context(&crate::interp::InterpState::native_jim_object_context(
                self,
            )?)?;
        }
        match v.native_scalar_getter_with_environment(
            dialect,
            tcl_syntax::scalar_getter::NativeScalarGetterKind::Wide,
            self.host().numeric_environment(),
        )? {
            tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(value) => Ok(value),
            _ => Err(ValueError::ScalarNumericInputUnavailable),
        }
    }

    fn integer_magnitude(
        &mut self,
        v: &Value,
        radix: Radix,
        syntax: tcl_dialect::NumberSyntax,
    ) -> Result<IntegerMagnitude, ValueError> {
        let context = self.numeric_context();
        let parsed = if let Some(simulation) = context.simulation {
            logical_integer_number(simulation, v)?
        } else {
            let input = context
                .scalar_numeric_input_policy()
                .ok_or(ValueError::ScalarNumericInputUnavailable)?;
            if let Some(integer @ (Number::Int(_) | Number::Big { .. })) = v.number_representation()
            {
                integer
            } else {
                let protocol = context
                    .dialect
                    .native_string_protocol()
                    .ok_or(ValueError::ScalarNumericInputUnavailable)?;
                let original = v
                    .native_string_bytes(protocol)
                    .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
                let text = std::str::from_utf8(input.input_bytes(&original))
                    .map_err(|_| ValueError::NotIntegerBytes(original.to_vec()))?;
                let mut flags = number::ParseFlags::for_syntax(syntax);
                flags.integer_only = true;
                number::parse_whole_with(text, flags)
                    .ok_or_else(|| ValueError::NotIntegerBytes(original.to_vec()))?
            }
        };
        let value = match parsed {
            Number::Int(value) => num_bigint::BigInt::from(value),
            Number::Big {
                negative,
                radix,
                digits,
            } => {
                let value = num_bigint::BigInt::parse_bytes(digits.as_bytes(), radix as u32)
                    .ok_or_else(|| ValueError::NotIntegerBytes(v.string_bytes().to_vec()))?;
                if negative { -value } else { value }
            }
            Number::Double(_) | Number::Nan { .. } => {
                return Err(ValueError::NotIntegerBytes(v.string_bytes().to_vec()));
            }
        };
        Ok(IntegerMagnitude {
            negative: value.sign() == num_bigint::Sign::Minus,
            digits: value.magnitude().to_str_radix(radix as u32),
        })
    }

    /// Native addition shared with `dict incr` under the actual interpreter.
    fn int_add(&mut self, a: Option<&Value>, b: &Value) -> Result<Value, ValueError> {
        int_add(self.numeric_context(), a, b)
    }

    fn as_double(&mut self, v: &Value) -> Result<f64, ValueError> {
        let context = self.numeric_context();
        if let Some(simulation) = context.simulation {
            let number = simulation
                .parse_number(
                    &v.string_bytes(),
                    tcl_syntax::logical_numeric_simulation::LogicalNumericInputStage::Number,
                )
                .map_err(|_| ValueError::NotDoubleBytes(v.string_bytes().to_vec()))?;
            return match number {
                Number::Int(value) => Ok(num_traits::ToPrimitive::to_f64(&value)
                    .expect("wide integer fits double exponent")),
                Number::Double(value) => Ok(value),
                Number::Nan { .. } => Ok(f64::NAN),
                integer @ Number::Big { .. } => {
                    let policy = context
                        .arithmetic()
                        .ok_or(ValueError::ScalarNumericInputUnavailable)?;
                    tcl_syntax::expr::wide::parsed_literal(policy, &integer)
                        .map(|value| {
                            num_traits::ToPrimitive::to_f64(&value)
                                .expect("wide integer fits double exponent")
                        })
                        .map_err(|_| ValueError::IntegerOverflow)
                }
            };
        }
        let dialect = context.dialect;
        if dialect.native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            v.bind_native_jim_context(&crate::interp::InterpState::native_jim_object_context(
                self,
            )?)?;
        }
        match v.native_scalar_getter_with_environment(
            dialect,
            tcl_syntax::scalar_getter::NativeScalarGetterKind::Double,
            self.host().numeric_environment(),
        )? {
            tcl_syntax::scalar_getter::NativeScalarGetterValue::Double(value) => Ok(value),
            _ => Err(ValueError::ScalarNumericInputUnavailable),
        }
    }

    fn as_bool(&mut self, v: &Value) -> Result<bool, ValueError> {
        let context = self.numeric_context();
        if let Some(simulation) = context.simulation {
            let stage =
                tcl_syntax::logical_numeric_simulation::LogicalBooleanInputStage::BooleanValue;
            if let Some(integer) = v.integer_representation() {
                return Ok(simulation
                    .current_number_boolean(Number::Int(integer), stage)
                    .value());
            }
            if let Some(double) = v.double_representation() {
                return Ok(simulation
                    .current_number_boolean(Number::Double(double), stage)
                    .value());
            }
            return simulation
                .parse_boolean(&v.string_bytes(), stage)
                .map(|value| value.value())
                .map_err(|_| ValueError::NotBooleanBytes(v.string_bytes().to_vec()));
        }
        match v.native_scalar_getter(
            context.dialect,
            tcl_syntax::scalar_getter::NativeScalarGetterKind::Boolean,
        )? {
            tcl_syntax::scalar_getter::NativeScalarGetterValue::Boolean(value) => Ok(value),
            _ => Err(ValueError::ScalarNumericInputUnavailable),
        }
    }

    fn list_len(&mut self, value: &Value) -> Result<usize, ValueError> {
        use tcl_registry::native_stock_list::{
            LogicalListLengthProvider, NativeObjectLengthAction,
        };
        let protocol = self
            .native_invocation_dialect()
            .object_length_protocol(Some(LogicalListLengthProvider::Tcl84CoreSimulation))
            .ok_or(ValueError::CommandProtocolUnavailable("object list length"))?;
        let class = value.stock_list_input_class();
        let canonical_empty = match value.resident_string_storage_identity() {
            Some(tcl_syntax::native_string::NativeStringStorageIdentity::CanonicalEmpty) => true,
            Some(tcl_syntax::native_string::NativeStringStorageIdentity::Unknown)
                if value
                    .resident_string_bytes()
                    .is_some_and(|bytes| bytes.is_empty()) =>
            {
                if protocol.action(class, true) != protocol.action(class, false) {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "object list length storage identity",
                    ));
                }
                false
            }
            _ => false,
        };
        match protocol.action(class, canonical_empty).ok_or(
            ValueError::CommandProtocolUnavailable("object list length storage"),
        )? {
            NativeObjectLengthAction::CachedList => {
                value
                    .cached_list_length()
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "object list length storage",
                    ))
            }
            NativeObjectLengthAction::Constant(length) => Ok(length),
            NativeObjectLengthAction::ConvertToList => {
                self.list_elements(value).map(|elements| elements.len())
            }
        }
    }

    fn list_elements(&mut self, v: &Value) -> Result<Vec<Value>, ValueError> {
        let dialect = self.native_invocation_dialect();
        let protocol = dialect.native_string_materialization(Some(tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation)).ok_or(ValueError::CommandProtocolUnavailable("native object list conversion"))?.protocol();
        self.native_object_list_elements_in(v, protocol)
            .map(|items| items.as_ref().clone())
    }

    fn dict_pairs(&mut self, v: &Value) -> Result<Vec<(Value, Value)>, ValueError> {
        let dialect = self.native_invocation_dialect();
        let protocol = dialect.native_string_materialization(Some(tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation)).ok_or(ValueError::CommandProtocolUnavailable("native object dictionary conversion"))?.protocol();
        self.native_object_dict_pairs_in(v, protocol)
    }

    fn dict_hash_bucket_count(&mut self, v: &Value) -> Result<Option<usize>, ValueError> {
        <Self as ValueOps>::dict_pairs(self, v)?;
        v.dict_hash_bucket_count().map(Some)
    }

    fn new_dict_checked(&mut self, pairs: Vec<(Value, Value)>) -> Result<Value, ValueError> {
        self.new_dict_with_hash_bucket_count_checked(pairs, 4)
    }

    fn new_dict_with_hash_bucket_count_checked(
        &mut self,
        pairs: Vec<(Value, Value)>,
        bucket_count: usize,
    ) -> Result<Value, ValueError> {
        let recipe = self.native_invocation_dialect().native_string_materialization(Some(tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation)).ok_or(ValueError::CommandProtocolUnavailable("native dictionary construction"))?;
        let buckets = Some(bucket_count);
        Value::native_dictionary_constructor(pairs, buckets, recipe.protocol())
    }

    fn new_dict(&mut self, pairs: Vec<(Value, Value)>) -> Value {
        let value = Value::dict(pairs);
        if let Some(recipe) = self
            .native_invocation_dialect()
            .native_string_materialization(Some(
            tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
        )) {
            value
                .seal_compound_string_protocol(recipe.protocol())
                .expect("fresh native Dictionary recipe");
        }
        value
    }

    fn new_dict_with_hash_bucket_count(
        &mut self,
        pairs: Vec<(Value, Value)>,
        bucket_count: usize,
    ) -> Value {
        let value = Value::dict_with_hash_bucket_count(pairs, Some(bucket_count));
        if let Some(recipe) = self
            .native_invocation_dialect()
            .native_string_materialization(Some(
            tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
        )) {
            value
                .seal_compound_string_protocol(recipe.protocol())
                .expect("fresh native Dictionary recipe");
        }
        value
    }
}

#[cfg(test)]
mod raw_value_tests {
    use super::*;

    #[test]
    fn native_safe_index_preparation_keeps_eager_functions_and_lazy_substitutions() {
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let profile = tcl_registry::model::ingress::static_context_for(engine)
                .commands()
                .profile()
                .unwrap();
            let mut vm = crate::native_fixture::core(profile);
            for source in [
                "0 && absent(1)",
                "1 ? 0 : absent(1)",
                "0 && sqrt()",
                "1 ? 0 : sqrt(1,2)",
                "sqrt(0)",
                "$absent",
                "[error REACHED]",
                "\"$absent\"",
            ] {
                let error =
                    tcl_cmd_core::index::resolve_for_ops(&mut vm, source, 2).expect_err(source);
                assert!(
                    error.native_access_refusal().is_none(),
                    "{engine}: {source}"
                );
                if engine == "jim" {
                    assert_eq!(
                        error.into_byte_details().message,
                        format!("bad index \"{source}\": must be intexpr or end?[+-]intexpr?")
                            .as_bytes(),
                    );
                }
            }
            for source in [
                "0 && sqrt(1)",
                "1 ? 0 : $absent",
                "0 && [error REACHED]",
                "1 ? 0 : \"$absent\"",
                "int(0)",
            ] {
                let result = tcl_cmd_core::index::resolve_for_ops(&mut vm, source, 2);
                if engine == "jim" {
                    assert_eq!(result.unwrap(), 0, "{source}");
                } else {
                    let error = result.expect_err(source);
                    assert!(
                        error.native_access_refusal().is_none(),
                        "{engine}: {source}"
                    );
                }
            }
            let result = tcl_cmd_core::index::resolve_for_ops(&mut vm, "0+1", 2);
            if engine == "tcl8.4" {
                assert!(result.is_err());
            } else {
                assert_eq!(result.unwrap(), 1, "{engine}");
            }
            assert!(vm.execution_refusal.is_none(), "{engine}");
        }
    }

    #[test]
    fn safe_index_functions_require_the_actual_fixed_table() {
        let current = tcl_registry::model::ingress::static_context_for("tcl9.0")
            .commands()
            .profile()
            .unwrap();
        let jim = tcl_registry::model::ingress::static_context_for("jim")
            .commands()
            .profile()
            .unwrap();
        let mut vm = crate::native_fixture::core(current);
        vm.set_dialect_profile(jim);
        assert!(vm.native_fixed_math_prerequisite().is_none());
        let error = tcl_cmd_core::index::resolve_for_ops(&mut vm, "0 && sqrt(1)", 2).unwrap_err();
        assert!(error.native_access_refusal().is_some());
        let completion = crate::command::completion_from_cmd_error(&mut vm, error);
        assert_eq!(completion.code, tcl_runtime_api::Code::Error);
        assert!(matches!(
            vm.execution_refusal,
            Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(_))
        ));
    }

    #[test]
    fn native_increment_overflow_retains_the_full_integer_primary() {
        use tcl_cmd_core::native_increment::NativeIncrementObjects;
        for engine in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = Vm::new();
            vm.set_dialect_profile(
                tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
            );
            let objects =
                VmIncrementObjects::selected(vm.actual_native_invocation_dialect()).unwrap();
            for (current, amount, negative, digits) in [
                (i64::MAX, 1, false, "9223372036854775808"),
                (i64::MIN, -1, true, "9223372036854775809"),
            ] {
                let initial = Number::Int(current);
                let current = Value::int(current);
                let result = tcl_cmd_core::native_increment::increment(
                    &objects,
                    Some(&current),
                    &Value::int(amount),
                )
                .unwrap();
                assert_eq!(
                    result.number_representation(),
                    Some(Number::Big {
                        negative,
                        radix: Radix::Dec,
                        digits: digits.into(),
                    }),
                    "{engine}",
                );
                assert_eq!(
                    objects
                        .add(
                            result.number_representation().unwrap(),
                            Number::Int(-amount)
                        )
                        .unwrap(),
                    initial,
                    "{engine}",
                );
            }
        }
    }

    #[test]
    fn selected_jim_byte_values_keep_native_units_list_objects_and_numeric_errors() {
        let mut vm = Vm::new();
        vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("jim").analyser_profile(),
        );
        let value = Value::from_string_bytes(vec![0xff, 0xed, 0xa0, 0x80]);
        assert_eq!(vm.native_char_len(&value), Ok(2));
        assert!(vm.try_as_str(&value).is_err());
        assert_eq!(
            vm.as_int(&value),
            Err(ValueError::NotIntegerBytes(vec![0xff, 0xed, 0xa0, 0x80]))
        );
        let list = Value::list(vec![value.clone()]);
        assert_eq!(
            vm.list_elements(&list).unwrap()[0].string_bytes(),
            value.string_bytes()
        );
        let source = Value::from_string_bytes(vec![b'{', 0xff]);
        assert_eq!(
            vm.list_elements(&source).unwrap()[0]
                .string_bytes()
                .as_ref(),
            &[0xff]
        );
        let dictionary = vm.new_dict(vec![(value.clone(), Value::int(1))]);
        assert_eq!(
            <Vm as ValueOps>::dict_pairs(&mut vm, &dictionary).unwrap()[0]
                .0
                .string_bytes(),
            value.string_bytes()
        );
    }

    #[test]
    fn portable_error_adapter_separates_raw_guest_result_from_unicode_refusal() {
        let mut vm = Vm::new();
        let error = tcl_cmd_core::CmdError::new_bytes(vec![0xff]);
        let completion = crate::command::completion_from_cmd_error(&mut vm, error);
        assert_eq!(completion.code, tcl_runtime_api::Code::Error);
        assert_eq!(completion.result.string_bytes().as_ref(), &[0xff]);
        assert!(vm.execution_refusal.is_none());
        let value = Value::from_string_bytes(vec![0xff]);
        let error = tcl_cmd_core::CmdError::from(value.try_to_str().unwrap_err());
        let _ = crate::command::completion_from_cmd_error(&mut vm, error);
        assert!(matches!(
            vm.execution_refusal,
            Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(_))
        ));
        assert_eq!(value.string_bytes().as_ref(), &[0xff]);
    }

    #[test]
    fn reached_host_refusal_cannot_initialise_or_write_an_increment_receiver() {
        fn refuse(vm: &mut Vm, _: &[Value]) -> tcl_runtime_api::Completion<Value> {
            vm.refuse_host_command("test native-access refusal".to_owned())
        }
        for version in tcl_dialect::TclVersion::ALL {
            let mut vm = Vm::new();
            vm.set_dialect_profile(
                tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
            );
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::default(),
            ));
            vm.register("refuse", refuse);
            vm.set_var("x", Value::int(10)).unwrap();
            vm.add_var_trace("x", vec!["read".to_owned()], "refuse".to_owned(), false);
            assert!(matches!(
                vm.try_eval_source("incr x"),
                Err(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(_))
            ));
            assert_eq!(vm.get_var("x").unwrap().string_bytes().as_ref(), b"10");
        }
    }

    #[test]
    fn jim_array_read_bridge_preserves_distinct_raw_dictionary_keys() {
        use tcl_runtime_api::{ArrayElementRead, FrameId, VarStore};
        let mut vm = Vm::new();
        vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("jim").analyser_profile(),
        );
        vm.set_var(
            "a",
            Value::dict(vec![
                (Value::from_string_bytes(vec![0xff]), Value::string("raw")),
                (Value::string("ÿ"), Value::string("unicode")),
            ]),
        )
        .unwrap();
        let target = vm.array_target(FrameId(0), "a");
        assert_eq!(
            vm.array_key_bytes_checked_at(&target).unwrap().unwrap(),
            [vec![0xff], "ÿ".as_bytes().to_vec()]
        );
        let ArrayElementRead::Value(raw) = vm.array_read_elem_bytes_at(&target, &[0xff]).unwrap()
        else {
            panic!("existing raw key must be readable");
        };
        assert_eq!(raw.string_bytes().as_ref(), b"raw");
        let ArrayElementRead::Value(unicode) = vm
            .array_read_elem_bytes_at(&target, "ÿ".as_bytes())
            .unwrap()
        else {
            panic!("distinct Unicode key must be readable");
        };
        assert_eq!(unicode.string_bytes().as_ref(), b"unicode");
        assert!(
            vm.array_keys_checked_at(&target)
                .unwrap_err()
                .native_access_refusal()
                .is_some()
        );
        assert!(vm.execution_refusal.is_none());
        assert!(vm.unset_elem_bytes_at(&target, &[0xff]).unwrap());
        assert_eq!(
            vm.array_key_bytes_checked_at(&target).unwrap().unwrap(),
            ["ÿ".as_bytes().to_vec()]
        );
        assert!(!vm.unset_elem_bytes_at(&target, &[0xff]).unwrap());
        vm.unset_var("a");
        vm.set_var("a", Value::dict(vec![(Value::string("ÿ"), Value::int(1))]))
            .unwrap();
        assert!(!vm.unset_elem_bytes_at(&target, "ÿ".as_bytes()).unwrap());
        assert_eq!(
            vm.get_var("a")
                .unwrap()
                .native_dict_pairs(
                    vm.native_invocation_dialect().lexer_grammar.list_parse,
                    vm.native_invocation_dialect().lexer_grammar.escapes,
                )
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn selected_jim_string_operations_match_original_native_byte_extents() {
        let mut vm = Vm::new();
        vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("jim").analyser_profile(),
        );
        // Pinned Jim084 observations: FF, ASCII A, surrogate D800, scalar é.
        let raw = vec![0xff, b'A', 0xed, 0xa0, 0x80, 0xc3, 0xa9];
        let value = Value::from_string_bytes(raw.clone());
        assert_eq!(
            tcl_cmd_core::string::index(&mut vm, &value, &Value::int(0))
                .unwrap()
                .string_bytes()
                .as_ref(),
            &[0xff]
        );
        assert_eq!(
            tcl_cmd_core::string::index(&mut vm, &value, &Value::int(2))
                .unwrap()
                .string_bytes()
                .as_ref(),
            &[0xed, 0xa0, 0x80]
        );
        assert_eq!(
            tcl_cmd_core::string::range(&mut vm, &value, &Value::int(1), &Value::string("end"))
                .unwrap()
                .string_bytes()
                .as_ref(),
            &[b'A', 0xed, 0xa0, 0x80, 0xc3, 0xa9]
        );
        assert_eq!(
            tcl_cmd_core::string::reverse(&mut vm, &value)
                .unwrap()
                .string_bytes()
                .as_ref(),
            &[0xc3, 0xa9, 0xed, 0xa0, 0x80, b'A', 0xff]
        );
        assert_eq!(
            tcl_cmd_core::string::repeat(&mut vm, &value, &Value::int(2))
                .unwrap()
                .string_bytes()
                .as_ref(),
            raw.repeat(2)
        );
        assert_eq!(
            tcl_cmd_core::string::repeat(&mut vm, &value, &Value::string("1+1"))
                .unwrap()
                .string_bytes()
                .as_ref(),
            raw.repeat(2)
        );
        let error =
            tcl_cmd_core::string::repeat(&mut vm, &value, &Value::string("[set x 1]")).unwrap_err();
        assert_eq!(
            error.message_bytes(),
            b"expected integer expression but got \"[set x 1]\""
        );
        assert!(error.native_access_refusal().is_none());
        let error =
            tcl_cmd_core::string::repeat(&mut vm, &value, &Value::from_string_bytes(vec![0xff]))
                .unwrap_err();
        let mut expected = b"expected integer expression but got \"".to_vec();
        expected.extend_from_slice(&[0xff, b'"']);
        assert_eq!(error.message_bytes(), expected);
        assert!(error.native_access_refusal().is_none());
        assert!(vm.execution_refusal.is_none());
        assert_eq!(value.string_bytes().as_ref(), raw);

        let unicode = Value::string("ÿ");
        assert_eq!(vm.native_char_len(&unicode), Ok(1));
        assert_ne!(unicode.string_bytes().as_ref(), &[0xff]);
    }

    #[test]
    fn jim_range_retains_native_seek_and_cached_count_until_conversion() {
        let mut vm = Vm::new();
        vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("jim").analyser_profile(),
        );
        let source = Value::from_string_bytes(vec![0xc3, b'A', 0xc3, 0xa9]);
        assert_eq!(vm.native_char_len(&source), Ok(3));
        assert_eq!(
            tcl_cmd_core::string::index(&mut vm, &source, &Value::int(1))
                .unwrap()
                .string_bytes()
                .as_ref(),
            &[0xc3, 0xa9],
        );
        assert_eq!(
            tcl_cmd_core::string::index(&mut vm, &source, &Value::int(2))
                .unwrap()
                .string_bytes()
                .as_ref(),
            &[0],
        );
        let range =
            tcl_cmd_core::string::range(&mut vm, &source, &Value::int(0), &Value::int(1)).unwrap();
        assert_eq!(range.string_bytes(), source.string_bytes());
        assert_eq!(vm.native_char_len(&range), Ok(2));
        let reconstruction = Value::from_string_bytes(range.string_bytes());
        assert_eq!(vm.native_char_len(&reconstruction), Ok(3));
        assert_eq!(vm.list_elements(&range).unwrap().len(), 1);
        assert_eq!(vm.native_char_len(&range), Ok(3));
        let suffix =
            tcl_cmd_core::string::range(&mut vm, &source, &Value::int(1), &Value::string("end"))
                .unwrap();
        assert_eq!(suffix.string_bytes().as_ref(), &[0xc3, 0xa9, 0]);
        assert_eq!(vm.native_char_len(&suffix), Ok(2));

        let unowned = Value::from_string_bytes(vec![0xe0, 0xe0, 0xe0, 0xc3, 0xa9]);
        assert_eq!(
            tcl_cmd_core::string::index(&mut vm, &unowned, &Value::int(2))
                .unwrap()
                .string_bytes()
                .as_ref(),
            &[0]
        );
        let error = tcl_cmd_core::string::index(&mut vm, &unowned, &Value::int(3)).unwrap_err();
        assert!(matches!(
            error.native_access_refusal(),
            Some(tcl_syntax::raw_string::NativeValueAccessRefusal::StringAccess(_))
        ));
        let _ = crate::command::completion_from_cmd_error(&mut vm, error);
        assert!(matches!(
            vm.execution_refusal,
            Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(_))
        ));
    }
}

#[cfg(test)]
mod native_equality_tests {
    use super::*;
    use tcl_syntax::{native_equality::full_native_equality, native_string::NativeStringProtocol};

    #[test]
    fn byte_and_unicode_equality_follow_the_original_native_cache() {
        let mut vm = Vm::new();
        vm.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        let raw = Value::from_string_bytes(b"\xff".as_slice());
        let encoded = Value::from_string_bytes(b"\xc3\xbf".as_slice());
        assert_eq!(full_native_equality(&mut vm, &raw, &encoded), Ok(false));
        raw.native_unicode_units(NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6))
            .unwrap();
        encoded
            .native_unicode_units(NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6))
            .unwrap();
        assert_eq!(full_native_equality(&mut vm, &raw, &encoded), Ok(true));
    }

    #[test]
    fn c_identity_shortcut_keeps_an_unmaterialized_integer() {
        let mut vm = Vm::new();
        vm.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        let value = Value::int(5);
        assert_eq!(full_native_equality(&mut vm, &value, &value), Ok(true));
        assert!(value.resident_string_bytes().is_none());
    }

    #[test]
    fn jim_identity_still_reaches_original_string_and_count_getters() {
        let mut vm = Vm::new();
        vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("jim").analyser_profile(),
        );
        let value = Value::int(5);
        assert_eq!(full_native_equality(&mut vm, &value, &value), Ok(true));
        assert_eq!(
            value.resident_string_bytes().as_deref(),
            Some(b"5".as_slice())
        );
        assert!(matches!(
            value.native_object_snapshot().cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::JimString { num_chars: Some(1) }
        ));
    }
}

#[cfg(test)]
mod native_equality_fixture_tests {
    use super::*;
    use tcl_syntax::{
        native_equality::full_native_equality, native_object::NativeObjectCacheSnapshot as Cache,
    };

    fn object(vm: &mut Vm, bytes: &[u8], representation: usize, jim: bool) -> Value {
        let value = if representation >= 2 {
            Value::byte_array(Rc::<[u8]>::from(bytes))
        } else {
            Value::from_string_bytes(bytes)
        };
        match representation {
            1 if jim => {
                vm.native_char_len(&value).unwrap();
            }
            1 => {
                vm.native_unicode_units(&value).unwrap();
            }
            3 => {
                vm.native_string_bytes(&value).unwrap();
            }
            _ => {}
        }
        value
    }

    fn physical(value: &Value, expected_type: &str, expected_resident: &str, case: &str) {
        let snapshot = value.native_object_snapshot();
        let actual = match snapshot.cache {
            Cache::None => "none",
            Cache::String { .. } | Cache::JimString { .. } => "string",
            Cache::ByteArray { .. } => "bytearray",
            other => panic!("{case}: unexpected physical cache {other:?}"),
        };
        assert_eq!(actual, expected_type, "{case}");
        assert_eq!(
            snapshot.resident.is_some(),
            expected_resident == "1",
            "{case}"
        );
    }

    #[test]
    fn original_object_equality_and_cache_effects_match_728_native_calls() {
        let pairs: &[(&str, &[u8], &[u8])] = &[
            ("nul-tail", b"a\0x", b"a\0y"),
            ("raw-modified-nul", b"a\0x", b"a\xc0\x80x"),
            ("raw-modified-ff", b"a\xffx", b"a\xc3\xbfx"),
            ("raw-cp1252-euro", b"a\x80x", b"a\xe2\x82\xacx"),
            ("utf8-two-byte", b"\xc3\xa9x", b"\xc3\xaax"),
            ("astral", b"\xf0\x9f\x98\x80x", b"\xf0\x9f\x98\x81x"),
            ("invalid-leading", b"\xffa", b"\xffb"),
        ];
        let engines = [
            (
                "tcl8.6",
                include_str!("../tests/data/native_equality/8.6.18.tsv"),
            ),
            (
                "tcl9.0",
                include_str!("../tests/data/native_equality/9.0.4.tsv"),
            ),
            (
                "tcl9.1",
                include_str!("../tests/data/native_equality/9.1.0.tsv"),
            ),
            ("jim", include_str!("../tests/data/native_equality/Jim.tsv")),
        ];
        let mut compared = 0;
        for (engine, expected) in engines {
            let jim = engine == "jim";
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut rows = expected.lines();
            let reps = if jim { 2 } else { 4 };
            for &(name, left, right) in pairs {
                for left_rep in 0..reps {
                    for right_rep in 0..reps {
                        for same in 0..2 {
                            let row: Vec<_> = rows.next().unwrap().split('\t').collect();
                            let case = format!("{name}-L{left_rep}-R{right_rep}-same{same}");
                            assert_eq!(row[0], case);
                            let mut vm = Vm::new();
                            vm.set_dialect_profile(profile);
                            let left = object(&mut vm, left, left_rep, jim);
                            let right = if same == 1 {
                                left.clone()
                            } else {
                                object(&mut vm, right, right_rep, jim)
                            };
                            physical(&left, row[2], row[3], &case);
                            physical(&right, row[4], row[5], &case);
                            let equal = full_native_equality(&mut vm, &left, &right).unwrap();
                            assert_eq!(equal, row[1] == "1", "{engine}/{case}");
                            physical(&left, row[6], row[7], &case);
                            physical(&right, row[8], row[9], &case);
                            compared += 1;
                        }
                    }
                }
            }
            assert!(rows.next().is_none());
        }
        assert_eq!(compared, 728);
    }
}
