// SPDX-License-Identifier: AGPL-3.0-or-later
//! Thin VM operations for the shared original Boolean/result stage worker.
//!
//! The exclusive Vm operation owns its actual interpreter, host, context and
//! selected dialect. Physical Host adapters receive no mutable Vm callback;
//! checked publication and guest settlement keep the first operational cause.
//! The poisoned speculative Interpreter guard supplies no operation authority.

use crate::{error::TclError, interp::Vm, value::Value};
use std::rc::Rc;
use tcl_cmd_core::{
    CmdError,
    native_boolean_truth::{
        NativeBooleanExpressionResultOps, NativeBooleanTruthOps,
        original_boolean_expression_result, original_boolean_truth,
    },
};
use tcl_registry::{
    InvocationDialect,
    native_boolean_truth::{
        NativeBooleanExpressionResultProduction as Production,
        NativeBooleanExpressionResultProtocol as ResultProtocol,
        NativeBooleanTruthProtocol as TruthProtocol, NativeBooleanTruthPurpose as Purpose,
    },
};
use tcl_syntax::{
    native_boolean_truth::NativeBooleanTruthProbe as Probe,
    number::Number,
    scalar_getter::{
        NativeScalarCache as Cache, NativeScalarGetterFailure as Failure,
        NativeScalarGetterKind as Getter, NativeScalarGetterValue as GetterValue,
    },
    value::ValueError,
};

struct OriginalOps<'a> {
    vm: &'a mut Vm,
    dialect: InvocationDialect,
    host: Rc<dyn tcl_platform::Host>,
    context: tcl_runtime_api::RuntimeContext,
    interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
}

fn unavailable() -> CmdError {
    ValueError::CommandProtocolUnavailable("original VM Boolean expression producer").into()
}

impl<'a> OriginalOps<'a> {
    fn entered(vm: &'a mut Vm) -> Result<Self, CmdError> {
        if let Some(first) = vm.execution_refusal.clone() {
            return Err(CmdError::from_execution_refusal(first));
        }
        Ok(Self {
            dialect: vm.actual_native_invocation_dialect(),
            host: vm.host_rc(),
            context: vm.runtime_context().clone(),
            interpreter: vm.native_interpreter_identity(),
            vm,
        })
    }
    fn current(&self) -> Result<(), CmdError> {
        if let Some(first) = self.vm.execution_refusal.clone() {
            return Err(CmdError::from_execution_refusal(first));
        }
        if self.vm.native_interpreter_identity() != self.interpreter
            || self.vm.runtime_context() != &self.context
            || self.vm.actual_native_invocation_dialect() != self.dialect
            || !Rc::ptr_eq(&self.vm.host_rc(), &self.host)
        {
            return Err(unavailable());
        }
        Ok(())
    }
    fn checked(&mut self, value: &Value, protocol: TruthProtocol) -> Result<(), CmdError> {
        self.current()?;
        value.check_native_header()?;
        if self.dialect.native_scalar_getter_protocol() != Some(protocol.scalar_protocol())
            || value
                .native_word_boolean_version()
                .is_some_and(|version| Some(version) != protocol.scalar_protocol().tcl_version())
        {
            return Err(unavailable());
        }
        if protocol.scalar_protocol().is_jim084() {
            value.bind_native_jim_context(&self.vm.native_jim_object_context()?)?;
        }
        Ok(())
    }
    fn publish(&mut self, value: Value) -> Result<(), CmdError> {
        self.current()?;
        self.vm.adopt_native_interp_result(value)?;
        self.current()
    }
    fn scalar_probe(
        &self,
        value: &Value,
        kind: Getter,
    ) -> Result<Result<GetterValue, Failure>, CmdError> {
        self.current()?;
        let result = value.native_scalar_probe_with_environment(
            self.dialect,
            kind,
            self.host.numeric_environment(),
        );
        self.current()?;
        result.map_err(Into::into)
    }
}

impl NativeBooleanTruthOps for OriginalOps<'_> {
    type Value = Value;
    fn check_host_refusal(&mut self) -> Result<(), CmdError> {
        self.current()
    }
    fn inspect_original(
        &mut self,
        value: &Value,
        protocol: TruthProtocol,
    ) -> Result<(Option<Cache>, bool), CmdError> {
        self.checked(value, protocol)?;
        Ok((
            value.native_scalar_cache(),
            value.resident_string_bytes().is_some(),
        ))
    }
    fn publish_original_operand(
        &mut self,
        value: &Value,
        protocol: TruthProtocol,
    ) -> Result<(), CmdError> {
        self.checked(value, protocol)?;
        self.publish(value.clone())
    }
    fn original_string(
        &mut self,
        value: &Value,
        protocol: TruthProtocol,
    ) -> Result<Vec<u8>, CmdError> {
        self.checked(value, protocol)?;
        Ok(value
            .native_string_bytes(
                self.dialect
                    .native_string_protocol()
                    .ok_or_else(unavailable)?,
            )
            .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)
            .map_err(ValueError::from)?
            .to_vec())
    }
    fn probe(
        &mut self,
        value: &Value,
        protocol: TruthProtocol,
        stage: Probe,
    ) -> Result<Result<GetterValue, Failure>, CmdError> {
        self.checked(value, protocol)?;
        match stage {
            Probe::Scalar { kind, .. } => self.scalar_probe(value, kind),
            Probe::ExpressionInteger84 => {
                let environment = self.host.numeric_environment();
                let before = environment
                    .map(|environment| environment.c_integer_abi())
                    .transpose()
                    .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
                self.current()?;
                let result = value.original_expression_integer84(self.dialect, environment);
                self.current()?;
                let after = environment
                    .map(|environment| environment.c_integer_abi())
                    .transpose()
                    .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
                if before != after {
                    return Err(unavailable());
                }
                result.map_err(Into::into)
            }
            Probe::ExpressionWordBoolean84(boolean) => value
                .original_expression_word_boolean84(self.dialect, boolean)
                .map(Ok)
                .map_err(Into::into),
        }
    }
    fn getter_failure(
        &mut self,
        value: &Value,
        protocol: TruthProtocol,
        kind: Getter,
        failure: Failure,
    ) -> Result<CmdError, CmdError> {
        self.checked(value, protocol)?;
        Ok(
            ValueError::NativeScalarGetter(Box::new(value.native_scalar_failure_presentation(
                self.dialect,
                kind,
                failure,
            )?))
            .into(),
        )
    }
    fn publish_intermediate_failure(&mut self, error: CmdError) -> Result<(), CmdError> {
        self.current()?;
        let completion = crate::command::completion_from_cmd_error(self.vm, error);
        self.current()?;
        self.publish(completion.result)
    }
    fn logical_number_probe(
        &mut self,
        value: &Value,
        protocol: TruthProtocol,
    ) -> Result<Result<Number, Failure>, CmdError> {
        self.checked(value, protocol)?;
        value
            .native_number_probe(
                self.dialect,
                tcl_syntax::scalar_getter::NativeNumberGetterKind::Number,
            )
            .map_err(Into::into)
    }
    fn logical_dictionary_size(
        &mut self,
        value: &Value,
        protocol: TruthProtocol,
    ) -> Result<Option<usize>, CmdError> {
        self.checked(value, protocol)?;
        value
            .original_expression_cached_lengths(
                self.dialect
                    .native_string_protocol()
                    .ok_or_else(unavailable)?,
            )
            .map(|(_, dictionary)| dictionary)
            .map_err(Into::into)
    }
    fn logical_length_hook(
        &mut self,
        value: &Value,
        protocol: TruthProtocol,
    ) -> Result<Option<usize>, CmdError> {
        self.checked(value, protocol)?;
        value
            .original_expression_cached_lengths(
                self.dialect
                    .native_string_protocol()
                    .ok_or_else(unavailable)?,
            )
            .map(|(list, _)| list)
            .map_err(Into::into)
    }
    fn logical_list_probe(
        &mut self,
        value: &Value,
        protocol: TruthProtocol,
    ) -> Result<bool, CmdError> {
        self.checked(value, protocol)?;
        match value.native_object_list_elements(
            self.dialect
                .native_string_protocol()
                .ok_or_else(unavailable)?,
        ) {
            Ok(_) => Ok(true),
            Err(ValueError::NativeListParse { .. }) => Ok(false),
            Err(error) => Err(error.into()),
        }
    }
    fn logical_spelling_double_probe(
        &mut self,
        original: &[u8],
        protocol: TruthProtocol,
    ) -> Result<bool, CmdError> {
        self.current()?;
        let diagnostic = Value::new_native_string_bytes(original);
        self.checked(&diagnostic, protocol)?;
        Ok(self.scalar_probe(&diagnostic, Getter::Double)?.is_ok())
    }
}

impl NativeBooleanExpressionResultOps for OriginalOps<'_> {
    type ResultValue = Value;
    fn original_number_probe(
        &mut self,
        value: &Value,
        _protocol: ResultProtocol,
    ) -> Result<(), CmdError> {
        self.current()?;
        let _outcome = value.native_number_probe(
            self.dialect,
            tcl_syntax::scalar_getter::NativeNumberGetterKind::Number,
        )?;
        Ok(())
    }
    fn original_is_shared(&mut self, value: &Value) -> Result<bool, CmdError> {
        self.current()?;
        value.check_native_header()?;
        Ok(value.native_object_is_shared())
    }
    fn copy_numeric_result(
        &mut self,
        value: &Value,
        cache: Cache,
        _protocol: ResultProtocol,
    ) -> Result<Value, CmdError> {
        self.current()?;
        value.check_native_header()?;
        Value::from_native_scalar_cache(cache, None, self.dialect).map_err(Into::into)
    }
    fn invalidate_numeric_string(
        &mut self,
        value: &Value,
        _protocol: ResultProtocol,
    ) -> Result<(), CmdError> {
        self.current()?;
        value
            .original_expression_numeric_invalidate()
            .map_err(Into::into)
    }
    fn retain_original_result(&mut self, value: &Value) -> Result<Value, CmdError> {
        self.current()?;
        value.check_native_header()?;
        Ok(value.clone())
    }
    fn copy_api_result(
        &mut self,
        value: &Value,
        protocol: ResultProtocol,
    ) -> Result<Value, CmdError> {
        self.current()?;
        value.check_native_header()?;
        if !protocol.copies_api_result() {
            return Err(unavailable());
        }
        Ok(value.duplicate_native_object_in(
            self.dialect
                .native_string_protocol()
                .ok_or_else(unavailable)?,
        ))
    }
    fn nonfinite_expression_failure(
        &mut self,
        value: f64,
        protocol: ResultProtocol,
    ) -> Result<Option<CmdError>, CmdError> {
        self.current()?;
        let failure =
            if protocol.scalar_protocol().tcl_version() == Some(tcl_dialect::TclVersion::V8_4) {
                tcl_cmd_core::native_numeric::c84_nonfinite_error(
                    protocol.scalar_protocol(),
                    value,
                    self.host.numeric_environment(),
                )?
            } else if value.is_nan() {
                Some(tcl_syntax::expr::errors::NativeFloatError::Domain)
            } else {
                None
            };
        Ok(failure.map(|failure| {
            let (message, code) = failure.diagnostic();
            let error = CmdError::with_error_code(message, code);
            if protocol.scalar_protocol().tcl_version() == Some(tcl_dialect::TclVersion::V8_4) {
                error.with_native_string_result(tcl_syntax::native_string::NativeStringProtocol::C(
                    tcl_dialect::TclVersion::V8_4,
                ))
            } else {
                error
            }
        }))
    }
}

fn pending(vm: &Vm) -> Result<(), TclError> {
    match vm.execution_refusal.clone() {
        Some(first) => Err(TclError::from_execution_failure(first)),
        None => Ok(()),
    }
}

fn finish<T>(vm: &mut Vm, result: Result<T, CmdError>) -> Result<T, TclError> {
    if let Some(first) = vm.execution_refusal.clone() {
        return Err(TclError::from_execution_failure(first));
    }
    match result {
        Ok(value) => Ok(value),
        Err(error) => {
            let completion = crate::command::completion_from_cmd_error(vm, error);
            match vm.execution_refusal.clone() {
                Some(first) => Err(TclError::from_execution_failure(first)),
                None => Err(TclError::from_completion(completion)),
            }
        }
    }
}

/// Reached original operand instruction. Outer-result purpose tags cannot
/// attest that normalisation has run. Authored simulation is separately selected.
pub(crate) fn boolean_for_vm(
    vm: &mut Vm,
    value: &Value,
    purpose: Purpose,
) -> Result<bool, TclError> {
    if let Some(first) = vm.execution_refusal.clone() {
        return Err(TclError::from_execution_failure(first));
    }
    if vm.numeric_context().simulation.is_some() {
        return super::authored_boolean(vm.numeric_context(), value);
    }
    let result = (|| {
        let mut ops = OriginalOps::entered(vm)?;
        if purpose.requires_expression_result() {
            return Err(unavailable());
        }
        let protocol = ops
            .dialect
            .native_boolean_truth_protocol(purpose)
            .ok_or_else(unavailable)?;
        original_boolean_truth(value, protocol, &mut ops)
    })();
    finish(vm, result)
}

pub(crate) fn expression_boolean_for_vm(
    vm: &mut Vm,
    value: &Value,
    production: Production,
) -> Result<bool, TclError> {
    pending(vm)?;
    if vm.numeric_context().simulation.is_some() {
        return super::authored_boolean(vm.numeric_context(), value);
    }
    let result = (|| {
        let mut ops = OriginalOps::entered(vm)?;
        let result_protocol = ops
            .dialect
            .native_boolean_expression_result_protocol(production)
            .ok_or_else(unavailable)?;
        let result = original_boolean_expression_result(value, result_protocol, &mut ops)?;
        let truth = ops
            .dialect
            .native_boolean_truth_protocol(result_protocol.truth_purpose())
            .ok_or_else(unavailable)?;
        original_boolean_truth(&result, truth, &mut ops)
    })();
    finish(vm, result)
}

pub(crate) fn logical_right_for_vm(
    vm: &mut Vm,
    left: &Value,
    right: &Value,
    conjunction: bool,
) -> Result<bool, TclError> {
    pending(vm)?;
    if vm.numeric_context().simulation.is_some() {
        return super::authored_boolean(vm.numeric_context(), right);
    }
    let result = (|| {
        let mut ops = OriginalOps::entered(vm)?;
        let purpose = ops
            .dialect
            .native_logical_final_operand_purpose(conjunction)
            .ok_or_else(unavailable)?;
        let protocol = ops
            .dialect
            .native_boolean_truth_protocol(purpose)
            .ok_or_else(unavailable)?;
        let left = if matches!(
            purpose,
            Purpose::LogicalAndInstruction | Purpose::LogicalOrInstruction
        ) {
            Some(original_boolean_truth(left, protocol, &mut ops)?)
        } else {
            None
        };
        let right = original_boolean_truth(right, protocol, &mut ops)?;
        Ok(left.map_or(right, |left| {
            if conjunction {
                left && right
            } else {
                left || right
            }
        }))
    })();
    finish(vm, result)
}

pub(crate) fn normalize_result_for_vm(
    vm: &mut Vm,
    value: &Value,
    production: Production,
) -> Result<Value, TclError> {
    pending(vm)?;
    if vm.numeric_context().simulation.is_some() {
        return Ok(value.clone());
    }
    let result = (|| {
        let mut ops = OriginalOps::entered(vm)?;
        let protocol = ops
            .dialect
            .native_boolean_expression_result_protocol(production)
            .ok_or_else(unavailable)?;
        original_boolean_expression_result(value, protocol, &mut ops)
    })();
    finish(vm, result)
}

pub(crate) fn normalize_numeric_instruction_for_vm(
    vm: &mut Vm,
    value: &Value,
) -> Result<Value, TclError> {
    pending(vm)?;
    if vm.numeric_context().simulation.is_some() {
        return Ok(value.clone());
    }
    let result = (|| {
        let mut ops = OriginalOps::entered(vm)?;
        let protocol = ops
            .dialect
            .native_numeric_instruction_result_protocol()
            .ok_or_else(unavailable)?;
        original_boolean_expression_result(value, protocol, &mut ops)
    })();
    finish(vm, result)
}

pub(crate) fn unary_for_vm(
    vm: &mut Vm,
    op: tcl_syntax::expr::UnaryOp,
    value: &Value,
) -> Result<Value, TclError> {
    pending(vm)?;
    if matches!(
        op,
        tcl_syntax::expr::UnaryOp::Not | tcl_syntax::expr::UnaryOp::WordNot
    ) {
        let boolean = boolean_for_vm(vm, value, Purpose::LogicalNot)?;
        return Ok(super::native_boolean_result(vm.numeric_context(), !boolean));
    }
    super::unary_in(vm.numeric_context(), op, value)
}

/// Consume a real eager logical opcode while preserving C8.4's original
/// unshared left-header result reuse after both reached operand conversions.
pub(crate) fn logical_value_for_vm(
    vm: &mut Vm,
    left: Value,
    right: &Value,
    conjunction: bool,
) -> Result<Value, TclError> {
    pending(vm)?;
    let truth = logical_right_for_vm(vm, &left, right, conjunction)?;
    let context = vm.numeric_context();
    if context.simulation.is_none()
        && context
            .dialect
            .native_scalar_getter_protocol()
            .and_then(|scalar| scalar.tcl_version())
            == Some(tcl_dialect::TclVersion::V8_4)
        && !left.native_object_is_shared()
    {
        left.store_native_expression_long84(i64::from(truth), context.dialect)?;
        Ok(left)
    } else {
        Ok(super::native_boolean_result(context, truth))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn native(environment: &str) -> Vm {
        crate::native_fixture::interpreter(crate::environment::profile_for_dialect(environment))
    }
    fn measured_jim_api(case: usize) -> (i32, i32, Vec<u8>) {
        let source = include_str!(
            "../../../tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/jim/execute-direct.stdout"
        );
        let prefix = format!("ROW\tcase={case}\tworker=1\t");
        let row = source
            .lines()
            .find(|line| line.starts_with(&prefix))
            .expect("original public ExprBool row");
        let fields = row
            .split('\t')
            .skip(1)
            .map(|field| field.split_once('=').unwrap())
            .collect::<std::collections::HashMap<_, _>>();
        let result = fields["result"]
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        (
            fields["code"].parse().unwrap(),
            fields["out"].parse().unwrap(),
            result,
        )
    }

    #[test]
    fn original_jim_public_truth_matches_two_original_api_cache_and_result_windows() {
        // naming.numeric.original-primitive-boolean-vs-expression-truth
        // docs/design/analysis/name-resolution-proofs/numeric-original-primitive-boolean-vs-expression-truth.md
        // Only direct public API out/result and pre-observer original cache/String
        // windows are compared; this does not assert a whole script or opcode.
        for (case, number, resident) in [
            (19, Number::Int(4_294_967_296), false),
            (28, Number::Double(f64::NAN), true),
        ] {
            let mut vm = native("jim");
            let seed = vm.apply_primitive_error_code(
                tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Set(b"SEEDED CODE".to_vec()),
            );
            vm.adopt_native_interp_result(Value::new_native_string_bytes(
                b"SEEDED RESULT".as_slice(),
            ))
            .unwrap();
            let value = match number {
                Number::Int(integer) => Value::int(integer),
                Number::Double(double) => Value::double(double),
                _ => unreachable!(),
            };
            assert!(value.resident_string_bytes().is_none());
            let measured = measured_jim_api(case);
            assert_eq!(measured.0, 0);
            assert_eq!(
                expression_boolean_for_vm(&mut vm, &value, Production::PublicExpressionApi)
                    .unwrap(),
                measured.1 != 0
            );
            assert_eq!(value.resident_string_bytes().is_some(), resident);
            match (number, value.native_scalar_cache()) {
                (Number::Int(expected), Some(Cache::Number(Number::Int(actual)))) => {
                    assert_eq!(actual, expected)
                }
                (Number::Double(expected), Some(Cache::Number(Number::Double(actual)))) => {
                    assert_eq!(actual.to_bits(), expected.to_bits())
                }
                other => panic!("original cache changed: {other:?}"),
            }
            let actual = vm
                .with_native_interp_result(|result| {
                    result.native_string_bytes(
                        tcl_syntax::native_string::NativeStringProtocol::Jim084,
                    )
                })
                .unwrap()
                .unwrap();
            assert_eq!(actual.as_ref(), measured.2.as_slice());
            let unchanged =
                vm.apply_primitive_error_code(tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Unchanged);
            assert!(unchanged.is_same_object(&seed));
            assert!(vm.execution_refusal.is_none());
        }
    }

    #[test]
    fn original_c84_logical_sides_preserve_distinct_selected_primary() {
        // naming.numeric.original-logical-operand-truth-sites
        // docs/design/analysis/name-resolution-proofs/numeric-original-logical-operand-truth-sites.md
        // Original465 case4 worker6/7 supplies bounded operand-stage expectations.
        // There is no whole expression result or all-provider assertion here.
        let mut vm = native("tcl8.4");
        let left = Value::new_native_string_bytes(b"4294967296".as_slice());
        assert!(!boolean_for_vm(&mut vm, &left, Purpose::LogicalAnd).unwrap());
        assert_eq!(left.native_scalar_cache(), Some(Cache::WordBoolean(false)));
        let right = Value::new_native_string_bytes(b"4294967296".as_slice());
        assert!(boolean_for_vm(&mut vm, &right, Purpose::LogicalAndInstruction).unwrap());
        assert_eq!(
            right.native_scalar_cache(),
            Some(Cache::Tcl84Long(4_294_967_296))
        );
        let string = vm
            .actual_native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        assert_eq!(
            left.native_string_bytes(string).unwrap().as_ref(),
            b"4294967296"
        );
        assert_eq!(
            right.native_string_bytes(string).unwrap().as_ref(),
            b"4294967296"
        );
    }

    #[test]
    fn original_vm_retired_operand_preserves_first_host_and_actual_prior_result() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut vm = native("tcl8.6");
        vm.adopt_native_interp_result(Value::new_native_string_bytes(b"PRIOR\0\xff".as_slice()))
            .unwrap();
        let original = Value::new_native_string_bytes(b"true".as_slice());
        let lifetime = original.native_lifetime_lease();
        drop(original);
        assert!(!lifetime.value().native_object_is_live());
        let first =
            expression_boolean_for_vm(&mut vm, lifetime.value(), Production::InlineExpression)
                .unwrap_err();
        assert!(first.is_host());
        assert!(matches!(
            first,
            TclError::Host(crate::error::TclHostFailure::Execution(
                tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "retired native object header"
                    )
                )
            ))
        ));
        let retained = vm.execution_refusal.clone().unwrap();
        let valid = Value::new_native_string_bytes(b"yes".as_slice());
        let next = boolean_for_vm(&mut vm, &valid, Purpose::ConditionalJump).unwrap_err();
        assert!(next.is_host());
        assert_eq!(vm.execution_refusal, Some(retained));
        assert!(valid.native_scalar_cache().is_none());
        assert_eq!(
            vm.with_native_interp_result(|value| value.resident_string_bytes().unwrap())
                .unwrap()
                .as_ref(),
            b"PRIOR\0\xff"
        );
    }

    #[test]
    fn reached_numeric_instruction_is_independent_of_condition_and_api_producers() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // This selected producer control asserts no original instruction capture.
        let mut vm = native("tcl8.6");
        let inline = Value::new_native_string_bytes(b"17".as_slice());
        let retained =
            normalize_result_for_vm(&mut vm, &inline, Production::InlineExpression).unwrap();
        assert!(retained.is_same_object(&inline));
        assert!(inline.native_scalar_cache().is_none());
        assert!(inline.resident_string_bytes().is_some());
        let original = Value::new_native_string_bytes(b"17".as_slice());
        let result = normalize_numeric_instruction_for_vm(&mut vm, &original).unwrap();
        assert!(result.is_same_object(&original));
        assert_eq!(
            result.native_scalar_cache(),
            Some(Cache::Number(Number::Int(17)))
        );
        assert!(result.resident_string_bytes().is_none());
        let boolean = boolean_for_vm(&mut vm, &result, Purpose::ExpressionApiResult).unwrap_err();
        assert!(boolean.is_host());
    }
}
