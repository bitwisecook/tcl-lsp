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

//! `ValueOps` for `Interp`: the shared command logic's view of the runtime's `*mut TclObj` values.

use super::append_objects::RuntimeAppendObjects;
#[cfg(have_tommath)]
use super::integer_object;
use super::native_jim_cat::OriginalJimStringCatConstruction;
use super::{
    integer_operand, native_concat, native_jim_index, value_error, RuntimeAppendValue,
    ValueCoercion,
};
use crate::interp::{obj_bytes, Interp};
use crate::obj::TclObj;
use crate::{list, obj};
use std::rc::Rc;
use tcl_syntax::number::Number;
#[cfg(have_tommath)]
use tcl_syntax::number::{self, Radix};
#[cfg(have_tommath)]
use tcl_syntax::value::IntegerMagnitude;
use tcl_syntax::value::{ValueError, ValueOps};

#[cfg(have_tommath)]
fn index_expression_error(source: &str, error: &crate::expr_error::ExprError) -> ValueError {
    error.native_access_refusal.map_or_else(
        || ValueError::NotInteger(source.to_owned()),
        ValueError::from,
    )
}

impl ValueOps for Interp {
    fn string_character_model(&self) -> Option<tcl_dialect::StringCharacterModel> {
        self.native_invocation_dialect().characters
    }
    type Value = *mut TclObj;

    fn name_policy_protocol(&self) -> Option<tcl_syntax::naming::NamePolicyProtocol> {
        Interp::name_policy_protocol(self)
    }

    fn original_option_index(
        &mut self,
        original: &Self::Value,
        words: &'static [&'static str],
        exact: bool,
        noun: &'static str,
    ) -> Result<Option<tcl_syntax::value::OriginalOptionLookup>, ValueError> {
        use tcl_syntax::value::OriginalOptionLookup;
        obj::check_native_liveness(*original)?;
        let dialect = self.native_invocation_dialect();
        if dialect.native_jim_enum_protocol().is_some() {
            let table =
                tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(words);
            let flags = tcl_registry::native_jim_enum::NativeJimEnumFlags::options(exact);
            return Ok(Some(
                match self.native_jim_enum_from_original(
                    *original,
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
            match self.native_index_from_original(*original, &table, exact, noun)? {
                Ok(index) => OriginalOptionLookup::Index(index),
                Err(message) => {
                    let bytes = self.native_string_bytes(original)?;
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

    fn same_object(&self, left: &Self::Value, right: &Self::Value) -> Option<bool> {
        (obj::allocation_is_live(*left) && obj::allocation_is_live(*right))
            .then_some(*left == *right)
    }

    fn concat_policy(&self) -> Option<tcl_dialect::ConcatPolicy> {
        self.native_invocation_dialect().concat_policy()
    }
    fn has_list_representation(&self, value: &Self::Value) -> bool {
        core::ptr::eq(obj::obj_type_ptr(*value), &crate::list::TCL_LIST_TYPE)
    }

    fn native_concat_list_shape(
        &self,
        value: &Self::Value,
    ) -> Result<tcl_syntax::value::NativeConcatListShape, ValueError> {
        native_concat::shape(self, *value)
    }
    fn native_concat_empty_list(&mut self) -> Result<Self::Value, ValueError> {
        native_concat::empty(self)
    }
    fn native_concat_copy_list(&mut self, value: &Self::Value) -> Result<Self::Value, ValueError> {
        native_concat::copy(self, *value)
    }
    fn native_concat_append_list(
        &mut self,
        target: &Self::Value,
        source: &Self::Value,
    ) -> Result<(), ValueError> {
        native_concat::append(self, *target, *source)
    }
    fn native_concat_first_bytes(
        &mut self,
        value: &Self::Value,
    ) -> Result<Option<tcl_syntax::value::NativeConcatFirstElement<Self::Value>>, ValueError> {
        native_concat::first(self, *value)
    }
    fn release_native_concat_first(
        &mut self,
        first: tcl_syntax::value::NativeConcatFirstElement<Self::Value>,
    ) {
        if let Some(original) = first.temporary {
            drop(obj::Owned::fresh(original));
        }
    }
    fn native_concat_string_bytes(&mut self, value: &Self::Value) -> Result<Rc<[u8]>, ValueError> {
        self.native_object_string_bytes(*value)
    }
    fn native_concat_string_result(&mut self, bytes: &[u8]) -> Result<Self::Value, ValueError> {
        native_concat::string(self, bytes)
    }
    fn discard_native_concat_result(&mut self, value: Self::Value) {
        drop(obj::Owned::fresh(value));
    }

    fn native_jim_string_cat(
        &mut self,
        args: &[*mut TclObj],
    ) -> Result<Option<*mut TclObj>, ValueError> {
        use tcl_registry::native_string_materialization::LogicalStringProvider;
        let dialect = self.native_invocation_dialect();
        let policy = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "selected string cat object purpose",
            ))?;
        let provider = (policy.authority()
            == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation)
            .then_some(LogicalStringProvider::Tcl84CoreSimulation);
        let protocol = dialect
            .native_string_materialization(provider)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "selected string cat object materialization",
            ))?
            .protocol();
        if !protocol.is_jim084() {
            return Ok(None);
        }
        let operation = crate::interp::native_operation_currency::NativeOperationScope::enter(self)
            .map_err(|_| {
                ValueError::CommandProtocolUnavailable("original Jim string cat operation")
            })?;
        let source = self.original_source_construction()?;
        let lifetimes = args
            .iter()
            .map(|&original| {
                obj::check_native_liveness(original)?;
                Ok(obj::NativeObjectLifetime::retain(original))
            })
            .collect::<Result<Vec<_>, ValueError>>()?;
        let check = |original: &RuntimeAppendValue| -> Result<(), ValueError> {
            operation
                .currency()
                .ensure_current_or_refuse()
                .map_err(|_| {
                    ValueError::CommandProtocolUnavailable("original Jim string cat operation")
                })?;
            if let Some(source) = &source {
                source.check(self)?;
            }
            for original in &lifetimes {
                original.checked_ptr()?;
            }
            obj::check_native_liveness(original.as_ptr())
        };
        let originals: Vec<_> = args
            .iter()
            .copied()
            .map(RuntimeAppendValue::borrowed)
            .collect();
        let objects = RuntimeAppendObjects {
            dialect,
            binary_recipe: None,
        };
        let retain = |error: &ValueError| {
            if let Some(refusal) = error.native_access_refusal() {
                operation.currency().retain_refusal(
                    &tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(refusal),
                );
            }
        };
        let result = tcl_cmd_core::native_cat::concatenate_jim_checked(
            &objects,
            protocol,
            &originals,
            &mut |original: &RuntimeAppendValue| {
                self.native_object_string_bytes_using(
                    original.as_ptr(),
                    &|| check(original),
                    &|error| error,
                )
            },
            &mut |original| check(original),
            &mut |error| retain(error),
        )?;
        let construction = OriginalJimStringCatConstruction::completed(
            &lifetimes,
            result.as_ptr(),
            source.clone(),
        );
        let recorded = self.record_original_namespace_info_cat(&construction);
        if let Err(error) = &recorded {
            retain(error);
        }
        recorded?;
        Ok(Some(result.into_native_unowned()))
    }

    fn index_syntax(&self) -> Option<tcl_dialect::IndexSyntax> {
        self.native_invocation_dialect().index_syntax()
    }

    fn index_error_string_protocol(
        &self,
    ) -> Result<Option<tcl_syntax::native_string::NativeStringProtocol>, ValueError> {
        let materialization = self
            .native_invocation_dialect()
            .native_string_materialization(None)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native index error String producer",
            ))?;
        Ok((!materialization.protocol().is_jim084()).then_some(materialization.protocol()))
    }

    #[cfg(not(have_tommath))]
    fn eval_index_expression(&mut self, _source: &str) -> Result<i64, ValueError> {
        Err(ValueError::ExpressionEngineUnavailable)
    }

    #[cfg(have_tommath)]
    fn eval_index_expression(&mut self, source: &str) -> Result<i64, ValueError> {
        let context = self
            .native_invocation_dialect()
            .expression_parse_context(Some(self.dialect_profile()));
        let node =
            if tcl_registry::runtime_expr_validation::requires_fixed_function_preparation(&context)
            {
                crate::builtins::prepare_runtime_fixed_expression(self, source)
                    .map_err(|error| index_expression_error(source, &error))?
            } else {
                let node = tcl_syntax::expr::parser::parse_expr_for_profile(
                    source,
                    Some(self.dialect_profile()),
                );
                tcl_registry::expr_surface::RuntimeExprSurface::for_profile(self.dialect_profile())
                    .validate(&node)
                    .map_err(|_| ValueError::NotInteger(source.to_owned()))?;
                node.map_text(String::into_bytes)
            };
        let value = crate::expr::eval_index_expression(&node, self.dialect_profile())
            .map_err(|error| index_expression_error(source, &error))?;
        match self.as_int(&value.as_ptr()) {
            Ok(value) => Ok(value),
            Err(error)
                if error.native_access_refusal().is_none()
                    && self
                        .native_invocation_dialect()
                        .index_syntax()
                        .is_some_and(|syntax| {
                            syntax.width == tcl_dialect::IndexIntegerWidth::Tcl64
                        }) =>
            {
                let bytes = obj_bytes(value.as_ptr());
                let text = std::str::from_utf8(&bytes)
                    .map_err(|_| ValueError::NotIntegerBytes(bytes.clone()))?;
                match number::parse_whole_with(
                    text,
                    number::ParseFlags::for_syntax(self.dialect_profile().grammar.numbers),
                ) {
                    Some(Number::Big { negative, .. }) => {
                        Ok(if negative { i64::MIN } else { i64::MAX })
                    }
                    _ => Err(error),
                }
            }
            Err(error) => Err(error),
        }
    }

    fn value_size_limit(&self) -> Option<u64> {
        self.value_limit()
    }

    fn measure_value(&mut self, v: &*mut TclObj, cap: u64) -> u64 {
        crate::budget::measured_bytes(*v, cap)
    }

    fn admit_value_bytes(&mut self, bytes: u64) -> bool {
        self.charge_allocation(bytes).is_none()
    }

    fn new_str(&mut self, s: &str) -> *mut TclObj {
        self.new_host_text(s)
    }

    fn new_string(&mut self, s: String) -> *mut TclObj {
        self.new_host_text(&s)
    }

    fn new_int(&mut self, n: i64) -> *mut TclObj {
        obj::new_wide_int_obj(n)
    }

    fn new_double(&mut self, f: f64) -> *mut TclObj {
        obj::new_double_obj(f)
    }

    fn new_jim_string(&mut self, bytes: &[u8], character_count: usize) -> *mut TclObj {
        let value = self.new_bytes(bytes);
        obj::retain_jim_string_count(value, character_count);
        value
    }

    fn jim_string_trim_result(
        &mut self,
        value: &Self::Value,
        plan: tcl_syntax::raw_string::JimStringTrimPlan,
    ) -> Self::Value {
        let bytes = obj_bytes(*value);
        let start = plan.byte_start();
        let end = plan.byte_end();
        if start > 0 {
            // The left-trim producer creates an unshared original working
            // header. Right trim converts and mutates that header in place.
            let working = self.new_bytes(&bytes[start..]);
            if plan.right_conversion() {
                obj::retain_jim_string_representation(working);
                if end == start {
                    crate::interp::drop_fresh(working);
                    return self.new_bytes(b"");
                }
                if end < bytes.len() {
                    obj::trim_jim_string_bytes(working, &bytes[start..end]);
                }
            }
            return working;
        }
        if plan.right_conversion() {
            obj::retain_jim_string_representation(*value);
            if end == 0 {
                return self.new_bytes(b"");
            }
        }
        if end == bytes.len() {
            return *value;
        }
        if end == 0 || obj::is_shared(*value) {
            return self.new_bytes(&bytes[..end]);
        }
        obj::trim_jim_string_bytes(*value, &bytes[..end]);
        *value
    }

    fn array_existence_result(&mut self, present: bool) -> Result<*mut TclObj, ValueError> {
        use tcl_registry::native_array_compilation::{
            native_array_existence_result, NativeArrayExistenceResult,
        };
        if self.observed_name_policy_selected() {
            return Ok(self.new_bool(present));
        }
        match native_array_existence_result(self.native_invocation_dialect()) {
            Some(NativeArrayExistenceResult::ExecutionBooleanConstant) => {
                self.native_execution_boolean_constant(present)
            }
            Some(NativeArrayExistenceResult::FreshInteger) => Ok(self.new_bool(present)),
            None => Err(ValueError::CommandProtocolUnavailable(
                "array existence result producer",
            )),
        }
    }

    fn new_bool(&mut self, b: bool) -> *mut TclObj {
        obj::new_boolean_obj(i32::from(b))
    }

    fn new_list(&mut self, items: Vec<*mut TclObj>) -> *mut TclObj {
        self.new_list_object(&items)
    }

    fn pin_value(&mut self, value: &*mut TclObj) {
        unsafe { obj::incr_ref_count(*value) };
    }

    fn unpin_value(&mut self, value: &*mut TclObj) {
        unsafe { obj::decr_ref_count(*value) };
    }

    fn try_as_str(
        &mut self,
        v: &*mut TclObj,
    ) -> Result<Rc<str>, tcl_syntax::raw_string::UnicodeAccessError> {
        tcl_syntax::raw_string::RawString::from_bytes(obj_bytes(*v)).unicode()
    }

    fn checked_character_view(&mut self, value: &*mut TclObj) -> Result<Rc<str>, ValueError> {
        if !self.has_native_character_view_entry() {
            return self.as_chars(value).map_err(Into::into);
        }
        // A ValueOps value is caller-owned live allocation storage. The lease
        // preserves that storage across callbacks without adding a Native ref.
        obj::check_native_liveness(*value)?;
        let original = obj::NativeObjectLifetime::retain(*value);
        self.native_character_view(&original)
    }

    /// A character-indexed read converts the value to a string (C's
    /// `Tcl_GetUnicodeFromObj`): a canonical list is one no longer.
    fn as_chars(
        &mut self,
        v: &*mut TclObj,
    ) -> Result<Rc<str>, tcl_syntax::raw_string::UnicodeAccessError> {
        list::read_as_string(*v);
        self.try_as_str(v)
    }

    fn is_untyped_or_string(&mut self, v: &*mut TclObj) -> bool {
        list::is_untyped_or_string(*v)
    }

    fn native_char_len(&mut self, value: &Self::Value) -> Result<usize, ValueError> {
        obj::check_native_liveness(*value)?;
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
        obj::native_character_count(*value, protocol, representation)
    }

    fn native_object_snapshot(
        &self,
        value: &Self::Value,
    ) -> Result<tcl_syntax::native_object::NativeObjectSnapshot, ValueError> {
        obj::native_object_snapshot(*value)
    }

    fn native_unicode_units(&mut self, value: &Self::Value) -> Result<Rc<[u32]>, ValueError> {
        obj::check_native_liveness(*value)?;
        let protocol = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native Unicode unit recipe",
            ))?
            .string_protocol();
        obj::native_unicode_units(*value, protocol)
    }

    fn native_unicode_string_result(
        &mut self,
        units: Rc<[u32]>,
        version: tcl_dialect::TclVersion,
    ) -> Result<Self::Value, ValueError> {
        let dialect = self.native_invocation_dialect();
        if dialect.tcl_version != Some(version) || dialect.native_error_log_protocol().is_none() {
            return Err(ValueError::CommandProtocolUnavailable(
                "native Unicode result issuer",
            ));
        }
        obj::new_native_unicode_obj(units, dialect)
    }

    fn native_external_utf8_result(
        &mut self,
        bytes: &[u8],
        version: tcl_dialect::TclVersion,
    ) -> Result<Self::Value, ValueError> {
        let dialect = self.native_invocation_dialect();
        let recipe = dialect
            .byte_array_string_recipe(None)
            .filter(|recipe| {
                recipe.protocol() == tcl_syntax::native_string::NativeStringProtocol::C(version)
            })
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native external UTF-8 binary result issuer",
            ))?;
        Ok(crate::bytearray::new_byte_array(bytes, recipe))
    }

    fn discard_native_internal_representation(
        &mut self,
        value: &Self::Value,
    ) -> Result<(), ValueError> {
        obj::discard_native_internal_representation(*value)
    }

    fn as_int(&mut self, v: &*mut TclObj) -> Result<i64, ValueError> {
        let operation = ValueCoercion::enter(self, *v)?;
        match operation.scalar(tcl_syntax::scalar_getter::NativeScalarGetterKind::Wide)? {
            tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(value) => Ok(value),
            _ => Err(ValueCoercion::value_error(
                self,
                ValueError::ScalarNumericInputUnavailable,
            )),
        }
    }

    /// `Tcl_GetIntFromObj` through the runtime's one typed-read owner
    /// (`typed_value::int`), under the same coercion currency as `as_int`.
    fn as_c_int(&mut self, v: &*mut TclObj) -> Result<i32, ValueError> {
        let operation = ValueCoercion::enter(self, *v)?;
        let read = crate::typed_value::int(*v).map_err(|error| value_error(*v, &error));
        operation.check()?;
        read
    }

    #[cfg(have_tommath)]
    fn integer_magnitude(
        &mut self,
        v: &*mut TclObj,
        radix: Radix,
        syntax: tcl_dialect::NumberSyntax,
    ) -> Result<IntegerMagnitude, ValueError> {
        // A held double is refused before its spelling is read.
        if let Some(error) = crate::typed_value::held_double(*v) {
            return Err(value_error(*v, &error));
        }
        let operation = ValueCoercion::enter(self, *v)?;
        let number = match operation.integer_number()? {
            Some(number @ (Number::Int(_) | Number::Big { .. })) => number,
            _ => return Err(ValueError::NotIntegerBytes(operation.original_string()?)),
        };
        // Render the accepted Number through the existing integer digit kernel.
        // The original's bytes and the caller's format syntax cannot reselect
        // input grammar or install a numeric primary onto a rejected original.
        let integer = integer_object(number);
        if integer.is_null() {
            return Err(ValueCoercion::value_error(
                self,
                ValueError::ScalarNumericInputUnavailable,
            ));
        }
        let integer = obj::Owned::fresh(integer);
        let magnitude = crate::bignum::integer_magnitude(integer.as_ptr(), radix, syntax);
        drop(integer);
        operation.check()?;
        magnitude
            .map(|(negative, digits)| IntegerMagnitude { negative, digits })
            .ok_or_else(|| {
                ValueCoercion::value_error(self, ValueError::ScalarNumericInputUnavailable)
            })
    }

    fn string_compare_length(&mut self, v: &*mut TclObj) -> Result<Option<usize>, ValueError> {
        let operation = ValueCoercion::enter(self, *v)?;
        match operation.integer_number()? {
            Some(Number::Int(integer)) => Ok(usize::try_from(integer).ok()),
            Some(Number::Big { .. }) => Err(ValueError::IntegerOverflow),
            _ => Err(ValueError::NotIntegerBytes(operation.original_string()?)),
        }
    }

    fn as_double(&mut self, v: &*mut TclObj) -> Result<f64, ValueError> {
        let operation = ValueCoercion::enter(self, *v)?;
        // A held double is its own value (`Tcl_GetDoubleFromObj`), not its
        // spelling, which 8.4 rounds to twelve digits.
        if obj::obj_type_ptr(*v) == &obj::TCL_DOUBLE_TYPE && !obj::double_of(*v).is_nan() {
            let value = obj::double_of(*v);
            operation.check()?;
            return Ok(value);
        }
        match operation.scalar(tcl_syntax::scalar_getter::NativeScalarGetterKind::Double)? {
            tcl_syntax::scalar_getter::NativeScalarGetterValue::Double(value) => Ok(value),
            _ => Err(ValueCoercion::value_error(
                self,
                ValueError::ScalarNumericInputUnavailable,
            )),
        }
    }

    fn as_bool(&mut self, v: &*mut TclObj) -> Result<bool, ValueError> {
        let operation = ValueCoercion::enter(self, *v)?;
        match operation.scalar(tcl_syntax::scalar_getter::NativeScalarGetterKind::Boolean)? {
            tcl_syntax::scalar_getter::NativeScalarGetterValue::Boolean(value) => {
                Ok(value.is_true())
            }
            _ => Err(ValueCoercion::value_error(
                self,
                ValueError::ScalarNumericInputUnavailable,
            )),
        }
    }

    /// Integer addition follows the selected native tower, independently of
    /// whether this build has an arbitrary-precision backend.
    fn int_add(
        &mut self,
        a: Option<&*mut TclObj>,
        b: &*mut TclObj,
    ) -> Result<*mut TclObj, ValueError> {
        if let Some(left) = a {
            obj::check_native_liveness(*left)?;
        }
        obj::check_native_liveness(*b)?;
        use tcl_dialect::NativeArithmetic;
        let dialect = self.native_invocation_dialect();
        let policy = dialect.arithmetic().ok_or(ValueError::IntegerOverflow)?;
        // Preserve native coercion order: current value, then increment.
        let left = a
            .map(|value| integer_operand(*value, dialect))
            .transpose()?;
        let right = integer_operand(*b, dialect)?;
        if policy != NativeArithmetic::TclBignum {
            let left = left
                .as_ref()
                .map(|value| tcl_syntax::expr::wide::parsed_literal(policy, value))
                .transpose()
                .map_err(|_| ValueError::IntegerOverflow)?
                .unwrap_or(0);
            let right = tcl_syntax::expr::wide::parsed_literal(policy, &right)
                .map_err(|_| ValueError::IntegerOverflow)?;
            let sum =
                tcl_syntax::expr::wide::binary(policy, tcl_syntax::expr::BinOp::Add, left, right)
                    .map_err(|_| ValueError::IntegerOverflow)?;
            return Ok(obj::new_wide_int_obj(sum));
        }
        #[cfg(have_tommath)]
        {
            let left = integer_object(left.unwrap_or(Number::Int(0)));
            let right = integer_object(right);
            let result = crate::bignum::add(left, right).map_err(|_| ValueError::IntegerOverflow);
            crate::interp::drop_fresh(left);
            crate::interp::drop_fresh(right);
            result
        }
        #[cfg(not(have_tommath))]
        {
            let (Some(Number::Int(left)), Number::Int(right)) =
                (left.or(Some(Number::Int(0))), right)
            else {
                return Err(ValueError::IntegerOverflow);
            };
            left.checked_add(right)
                .map(obj::new_wide_int_obj)
                .ok_or(ValueError::IntegerOverflow)
        }
    }

    fn list_len(&mut self, value: &Self::Value) -> Result<usize, ValueError> {
        native_jim_index::list_length_using(
            self,
            value,
            &mut |_, _| Ok(()),
            |interpreter, original| interpreter.list_elements(original),
        )
    }

    fn list_index(
        &mut self,
        value: &Self::Value,
        index: usize,
    ) -> Result<Option<Self::Value>, ValueError> {
        obj::check_native_liveness(*value)?;
        if crate::native_arithseries::is_series(*value) {
            let protocol =
                self.invocation_string_protocol()
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "abstract native Index",
                    ))?;
            return crate::native_arithseries::index_in(*value, index, protocol);
        }
        Ok(self.list_elements(value)?.into_iter().nth(index))
    }

    fn list_elements(&mut self, v: &*mut TclObj) -> Result<Vec<*mut TclObj>, ValueError> {
        obj::check_native_liveness(*v)?;
        let protocol = self.native_invocation_dialect().native_string_materialization(
            Some(tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation),
        ).ok_or(ValueError::CommandProtocolUnavailable("native object list conversion"))?.protocol();
        if protocol.is_jim084() {
            crate::native_source::bind_context(*v, &self.native_jim_object_context()?)?;
        }
        list::list_elements_native_checked(*v, protocol)
    }

    fn dict_pairs(
        &mut self,
        v: &*mut TclObj,
    ) -> Result<Vec<(*mut TclObj, *mut TclObj)>, ValueError> {
        obj::check_native_liveness(*v)?;
        let protocol = self.native_invocation_dialect().native_string_materialization(
            Some(tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation),
        ).ok_or(ValueError::CommandProtocolUnavailable("native object dictionary conversion"))?.protocol();
        if protocol.is_jim084() {
            crate::native_source::bind_context(*v, &self.native_jim_object_context()?)?;
        }
        crate::dict::native_dict_pairs(*v, protocol)
    }

    fn dict_hash_bucket_count(&mut self, v: &*mut TclObj) -> Result<Option<usize>, ValueError> {
        drop(self.dict_pairs(v)?);
        Ok(Some(
            crate::dict::dict_hash_bucket_count(*v).expect("native Dictionary cache"),
        ))
    }

    fn new_dict_checked(
        &mut self,
        pairs: Vec<(*mut TclObj, *mut TclObj)>,
    ) -> Result<*mut TclObj, ValueError> {
        self.new_dict_with_hash_bucket_count_checked(pairs, 4)
    }

    fn new_dict_with_hash_bucket_count_checked(
        &mut self,
        pairs: Vec<(*mut TclObj, *mut TclObj)>,
        bucket_count: usize,
    ) -> Result<*mut TclObj, ValueError> {
        let recipe = self.native_invocation_dialect().native_string_materialization(Some(tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation)).ok_or(ValueError::CommandProtocolUnavailable("native dictionary construction"))?;
        let buckets = Some(bucket_count);
        crate::dict::new_dict_obj_native(&pairs, buckets, recipe.protocol())
    }

    fn new_dict(&mut self, pairs: Vec<(*mut TclObj, *mut TclObj)>) -> *mut TclObj {
        let value = crate::dict::new_dict_obj(&pairs);
        if let Some(recipe) = self
            .native_invocation_dialect()
            .native_string_materialization(Some(
            tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
        )) {
            crate::dict::seal_string_protocol(value, recipe.protocol())
                .expect("fresh native Dictionary recipe");
        }
        value
    }

    fn new_dict_with_hash_bucket_count(
        &mut self,
        pairs: Vec<(*mut TclObj, *mut TclObj)>,
        bucket_count: usize,
    ) -> *mut TclObj {
        let value = crate::dict::new_dict_obj_with_hash_bucket_count(&pairs, Some(bucket_count));
        if let Some(recipe) = self
            .native_invocation_dialect()
            .native_string_materialization(Some(
            tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
        )) {
            crate::dict::seal_string_protocol(value, recipe.protocol())
                .expect("fresh native Dictionary recipe");
        }
        value
    }

    /// Byte-exact, and simpler than `as_str`'s Unicode round trip — this is why
    /// `append` (which never needs character semantics) routes through the
    /// shared core without any of `bytes_to_str`/`str_to_bytes`'s trade-offs.
    fn as_bytes(&mut self, v: &*mut TclObj) -> Rc<[u8]> {
        Rc::from(obj_bytes(*v).as_slice())
    }

    fn native_string_bytes(&mut self, value: &Self::Value) -> Result<Rc<[u8]>, ValueError> {
        self.native_object_string_bytes(*value)
    }

    fn native_split(
        &mut self,
        value: &Self::Value,
        chars: Option<&Self::Value>,
    ) -> Result<Option<Self::Value>, ValueError> {
        use tcl_syntax::{naming::NamePolicyAuthority, native_string::NativeStringProtocol};
        let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6);
        if !self.name_policy_protocol().is_some_and(|policy| {
            policy.authority() == NamePolicyAuthority::Native
                && policy.string_protocol() == protocol
        }) {
            return Ok(None);
        }
        let original = ValueCoercion::enter(self, *value)?;
        let delimiter = chars
            .map(|&chars| {
                obj::check_native_liveness(chars)?;
                Ok::<_, ValueError>(obj::NativeObjectLifetime::retain(chars))
            })
            .transpose()?;
        let characters = if let Some(delimiter) = &delimiter {
            let bytes = self.native_object_string_bytes(delimiter.as_ptr());
            original.check()?;
            obj::check_native_liveness(delimiter.as_ptr())?;
            bytes?
        } else {
            Rc::from(&b" \n\t\r"[..])
        };
        let source = self.native_object_string_bytes(*value);
        original.check()?;
        if let Some(delimiter) = &delimiter {
            obj::check_native_liveness(delimiter.as_ptr())?;
        }
        let source = source?;
        let parts = tcl_cmd_core::list::plan_c86_split(&source, &characters)?;
        let result = obj::Owned::fresh(obj::new_obj());
        let mut reused = std::collections::HashMap::<u32, *mut TclObj>::new();
        for part in parts {
            let existing = part.reuse.and_then(|key| reused.get(&key).copied());
            let fresh = existing
                .is_none()
                .then(|| obj::Owned::fresh(obj::new_string_bytes(&source[part.bytes])));
            let child =
                existing.unwrap_or_else(|| fresh.as_ref().expect("fresh split child").as_ptr());
            let appended = crate::list::append_native_element_with_currency(
                result.as_ptr(),
                child,
                protocol,
                original.scope.currency(),
            );
            original.check()?;
            appended.map_err(|error| ValueCoercion::value_error(self, error))?;
            if let Some(key) = part.reuse {
                reused.insert(key, child);
            }
        }
        original.check()?;
        Ok(Some(result.into_native_unowned()))
    }

    /// Byte-exact construction (the `obj::new_string_bytes` path).
    fn new_bytes(&mut self, bytes: &[u8]) -> *mut TclObj {
        obj::new_string_bytes(bytes)
    }

    fn native_command_full_name_result(&mut self, bytes: &[u8]) -> Result<*mut TclObj, ValueError> {
        Ok(self
            .native_namespace_origin_result(bytes)?
            .into_native_unowned())
    }

    fn try_append_bytes_in_place(&mut self, v: &mut *mut TclObj, bytes: &[u8]) -> bool {
        // Amortised O(1) growth when the object is an unshared plain string —
        // an in-place path the VM cannot take (it always copies).
        if obj::is_plain_string(*v) && !obj::is_shared(*v) {
            obj::string_append_inplace(*v, bytes);
            true
        } else {
            false
        }
    }

    fn try_list_append_in_place(
        &mut self,
        list: &mut *mut TclObj,
        item: &*mut TclObj,
    ) -> Result<bool, ValueError> {
        // naming.list.original-value-append-storage-currency
        // docs/design/analysis/name-resolution-proofs/list-original-value-append-storage-currency.md
        obj::check_native_liveness(*list)?;
        obj::check_native_liveness(*item)?;
        if obj::is_shared(*list) {
            return Ok(false);
        }
        let protocol = self.native_invocation_dialect().native_string_materialization(
            Some(tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation),
        ).ok_or(ValueError::CommandProtocolUnavailable("native object list append"))?.protocol();
        if protocol.is_jim084() {
            crate::native_source::bind_context(*list, &self.native_jim_object_context()?)?;
        }
        list::append_prepared_native_elements(*list, &[*item], protocol)?;
        Ok(true)
    }
}
