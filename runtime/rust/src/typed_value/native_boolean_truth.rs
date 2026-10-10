// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original Boolean instruction and result producers over retained Runtime objects.
//! The shared CmdCore executor owns conversion order. This adapter supplies the
//! actual interpreter, checked object/getter, host environment and result owner.

use crate::{
    interp::{
        native_operation_currency::{CheckedNumericEnvironment, NativeOperationCurrency},
        Interp,
    },
    obj::{self, Owned, TclObj},
};
use std::{cell::Cell, rc::Rc};
use tcl_cmd_core::{
    native_boolean_truth::{
        original_boolean_expression_result, original_boolean_truth,
        NativeBooleanExpressionResultOps, NativeBooleanTruthOps,
    },
    CmdError,
};
use tcl_registry::{
    native_boolean_truth::{
        NativeBooleanExpressionResultProduction as Production,
        NativeBooleanExpressionResultProtocol as ResultProtocol,
        NativeBooleanTruthProtocol as TruthProtocol, NativeBooleanTruthPurpose as Purpose,
    },
    InvocationDialect,
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
    dialect: InvocationDialect,
    interp: Option<&'a mut Interp>,
    host: Option<Rc<dyn tcl_platform::Host>>,
    environment: Option<&'a dyn tcl_platform::NumericEnvironment>,
    currency: Option<NativeOperationCurrency>,
    abi: Cell<Option<tcl_platform::NativeCIntegerAbi>>,
}

fn unavailable() -> CmdError {
    ValueError::CommandProtocolUnavailable("original Boolean expression producer").into()
}

impl<'a> OriginalOps<'a> {
    fn entered(interp: &'a mut Interp) -> Result<Self, CmdError> {
        let currency =
            NativeOperationCurrency::issue(interp).map_err(CmdError::from_execution_refusal)?;
        let dialect = interp.native_invocation_dialect();
        let host = Some(interp.host());
        Ok(Self {
            dialect,
            interp: Some(interp),
            host,
            environment: None,
            currency: Some(currency),
            abi: Cell::new(None),
        })
    }
    fn neutral(
        dialect: InvocationDialect,
        environment: Option<&'a dyn tcl_platform::NumericEnvironment>,
    ) -> Self {
        Self {
            dialect,
            interp: None,
            host: None,
            environment,
            currency: None,
            abi: Cell::new(None),
        }
    }
    fn current(&self) -> Result<(), CmdError> {
        if let Some(currency) = &self.currency {
            currency
                .ensure_current()
                .map_err(CmdError::from_execution_refusal)?;
        }
        Ok(())
    }
    fn environment(&self) -> Result<Option<CheckedNumericEnvironment<'_>>, CmdError> {
        self.current()?;
        let actual = self.environment.or_else(|| {
            self.host
                .as_ref()
                .and_then(|host| host.numeric_environment())
        });
        self.current()?;
        Ok(actual.map(|actual| {
            CheckedNumericEnvironment::new(actual, self.currency.as_ref(), &self.abi)
        }))
    }
    fn scalar_probe(
        &self,
        value: *mut TclObj,
        kind: Getter,
    ) -> Result<Result<GetterValue, Failure>, CmdError> {
        let environment = self.environment()?;
        let result = super::native_scalar_probe_with_environment_and_currency(
            value,
            self.dialect,
            kind,
            environment
                .as_ref()
                .map(|actual| actual as &dyn tcl_platform::NumericEnvironment),
            self.currency.as_ref(),
        );
        self.current()?;
        result.map_err(Into::into)
    }
    fn checked(&mut self, value: *mut TclObj, protocol: TruthProtocol) -> Result<(), CmdError> {
        self.current()?;
        obj::check_native_liveness(value)?;
        if self.dialect.native_scalar_getter_protocol() != Some(protocol.scalar_protocol())
            || self.interp.as_ref().is_some_and(|interp| {
                interp
                    .native_invocation_dialect()
                    .native_scalar_getter_protocol()
                    != Some(protocol.scalar_protocol())
            })
            || obj::native_word_boolean_version(value)
                .is_some_and(|version| Some(version) != protocol.scalar_protocol().tcl_version())
        {
            return Err(unavailable());
        }
        // Jim's reached reporting stages require the genuine active interpreter.
        if protocol.scalar_protocol().is_jim084() {
            let interp = self.interp.as_mut().ok_or_else(unavailable)?;
            let context = interp.native_jim_object_context()?;
            crate::native_source::bind_context(value, &context)?;
        }
        Ok(())
    }
    fn expression_integer84(
        &self,
        value: *mut TclObj,
        protocol: TruthProtocol,
    ) -> Result<Result<GetterValue, Failure>, CmdError> {
        let scalar = protocol.scalar_protocol();
        if scalar.tcl_version() != Some(tcl_dialect::TclVersion::V8_4) {
            return Err(unavailable());
        }
        let current = obj::native_scalar_cache(value)?;
        let original = crate::bytearray::scalar_getter_string(value, scalar)?;
        if let Some(environment) = self.environment()? {
            tcl_cmd_core::native_numeric::scalar_getter_target(&environment)?;
            self.current()?;
            // GET_WIDE_OR_INT performs the real host integer call independently
            // of the descriptive long/wide primary conversion retained below.
            tcl_cmd_core::native_numeric::fresh_c84_conversion(
                scalar,
                Getter::Wide,
                &original,
                &environment,
            )?;
            self.current()?;
            tcl_cmd_core::native_numeric::scalar_getter_target(&environment)?;
            self.current()?;
        } else if !matches!(
            current,
            Some(Cache::Tcl84Long(_) | Cache::Number(Number::Int(_)))
        ) {
            return Err(unavailable());
        }
        let conversion = scalar
            .expression_integer_conversion84(current.as_ref(), &original)
            .ok_or_else(unavailable)?;
        let (materialize, cache, outcome) = conversion.into_parts();
        if materialize {
            crate::bytearray::scalar_getter_string(value, scalar)?;
        }
        if let Some(cache) = cache {
            obj::adopt_native_scalar_cache(value, cache, scalar)?;
        }
        Ok(outcome)
    }
}

impl NativeBooleanTruthOps for OriginalOps<'_> {
    type Value = *mut TclObj;
    fn check_host_refusal(&mut self) -> Result<(), CmdError> {
        if let Some(first) = self
            .interp
            .as_ref()
            .and_then(|interp| interp.native_execution_refusal())
        {
            return Err(CmdError::from_execution_refusal(first));
        }
        self.current()
    }
    fn inspect_original(
        &mut self,
        value: &Self::Value,
        protocol: TruthProtocol,
    ) -> Result<(Option<Cache>, bool), CmdError> {
        self.checked(*value, protocol)?;
        Ok((
            obj::native_scalar_cache(*value)?,
            obj::has_string_rep(*value),
        ))
    }
    fn publish_original_operand(
        &mut self,
        value: &Self::Value,
        protocol: TruthProtocol,
    ) -> Result<(), CmdError> {
        self.checked(*value, protocol)?;
        self.interp
            .as_mut()
            .ok_or_else(unavailable)?
            .set_result(*value);
        Ok(())
    }
    fn original_string(
        &mut self,
        value: &Self::Value,
        protocol: TruthProtocol,
    ) -> Result<Vec<u8>, CmdError> {
        self.checked(*value, protocol)?;
        crate::bytearray::scalar_getter_string(*value, protocol.scalar_protocol())
            .map_err(Into::into)
    }
    fn probe(
        &mut self,
        value: &Self::Value,
        protocol: TruthProtocol,
        stage: Probe,
    ) -> Result<Result<GetterValue, Failure>, CmdError> {
        self.checked(*value, protocol)?;
        match stage {
            Probe::Scalar { kind, .. } => self.scalar_probe(*value, kind),
            Probe::ExpressionInteger84 => self.expression_integer84(*value, protocol),
            Probe::ExpressionWordBoolean84(boolean) => {
                if protocol.scalar_protocol().tcl_version() != Some(tcl_dialect::TclVersion::V8_4)
                    || obj::has_string_rep(*value)
                    || obj::native_scalar_cache(*value)? != Some(Cache::WordBoolean(boolean))
                {
                    return Err(unavailable());
                }
                obj::adopt_native_scalar_cache(
                    *value,
                    Cache::Tcl84Long(i64::from(boolean)),
                    protocol.scalar_protocol(),
                )?;
                Ok(Ok(GetterValue::Wide(i64::from(boolean))))
            }
        }
    }
    fn getter_failure(
        &mut self,
        value: &Self::Value,
        protocol: TruthProtocol,
        kind: Getter,
        failure: Failure,
    ) -> Result<CmdError, CmdError> {
        self.checked(*value, protocol)?;
        let record =
            super::native_scalar_failure_presentation(*value, self.dialect, kind, failure)?;
        Ok(ValueError::NativeScalarGetter(Box::new(record)).into())
    }
    fn publish_intermediate_failure(&mut self, failure: CmdError) -> Result<(), CmdError> {
        self.current()?;
        self.interp
            .as_mut()
            .ok_or_else(unavailable)?
            .report_cmd_error(failure);
        self.check_host_refusal()
    }
    fn logical_number_probe(
        &mut self,
        value: &Self::Value,
        protocol: TruthProtocol,
    ) -> Result<Result<Number, Failure>, CmdError> {
        self.checked(*value, protocol)?;
        let outcome = super::native_number_probe_with_currency(
            *value,
            self.dialect,
            tcl_syntax::scalar_getter::NativeNumberGetterKind::Number,
            self.currency.as_ref(),
        );
        self.current()?;
        outcome.map_err(Into::into)
    }
    fn logical_dictionary_size(
        &mut self,
        value: &Self::Value,
        protocol: TruthProtocol,
    ) -> Result<Option<usize>, CmdError> {
        self.checked(*value, protocol)?;
        Ok(crate::dict::native_cache_size(*value))
    }
    fn logical_length_hook(
        &mut self,
        value: &Self::Value,
        protocol: TruthProtocol,
    ) -> Result<Option<usize>, CmdError> {
        self.checked(*value, protocol)?;
        if crate::native_arithseries::is_series(*value) {
            return crate::native_arithseries::length_in(
                *value,
                self.dialect
                    .native_string_protocol()
                    .ok_or_else(unavailable)?,
            )
            .map_err(Into::into);
        }
        if obj::obj_type_ptr(*value) == &crate::list::TCL_LIST_TYPE {
            return Ok(crate::list::cached_length(*value));
        }
        Ok(None)
    }
    fn logical_list_probe(
        &mut self,
        value: &Self::Value,
        protocol: TruthProtocol,
    ) -> Result<bool, CmdError> {
        self.checked(*value, protocol)?;
        let string = self
            .dialect
            .native_string_protocol()
            .ok_or_else(unavailable)?;
        match crate::list::list_elements_native_checked(*value, string) {
            Ok(_) => Ok(true),
            Err(ValueError::NativeListParse { .. }) => Ok(false),
            Err(error) => Err(error.into()),
        }
    }
    fn logical_spelling_double_probe(
        &mut self,
        original: &[u8],
        _protocol: TruthProtocol,
    ) -> Result<bool, CmdError> {
        let diagnostic = Owned::fresh(obj::new_string_bytes(original));
        Ok(self
            .scalar_probe(diagnostic.as_ptr(), Getter::Double)?
            .is_ok())
    }
}

impl NativeBooleanExpressionResultOps for OriginalOps<'_> {
    type ResultValue = Owned;
    fn original_number_probe(
        &mut self,
        value: &Self::Value,
        _protocol: ResultProtocol,
    ) -> Result<(), CmdError> {
        let outcome = super::native_number_probe_with_currency(
            *value,
            self.dialect,
            tcl_syntax::scalar_getter::NativeNumberGetterKind::Number,
            self.currency.as_ref(),
        );
        self.current()?;
        outcome?;
        Ok(())
    }
    fn original_is_shared(&mut self, value: &Self::Value) -> Result<bool, CmdError> {
        obj::check_native_liveness(*value)?;
        Ok(obj::is_shared(*value))
    }
    fn copy_numeric_result(
        &mut self,
        value: &Self::Value,
        cache: Cache,
        protocol: ResultProtocol,
    ) -> Result<Owned, CmdError> {
        obj::check_native_liveness(*value)?;
        let result = Owned::fresh(obj::new_obj());
        obj::adopt_native_scalar_cache(result.as_ptr(), cache, protocol.scalar_protocol())?;
        obj::invalidate_string(result.as_ptr());
        Ok(result)
    }
    fn invalidate_numeric_string(
        &mut self,
        value: &Self::Value,
        _protocol: ResultProtocol,
    ) -> Result<(), CmdError> {
        obj::check_native_liveness(*value)?;
        if obj::is_shared(*value)
            || !matches!(
                obj::native_scalar_cache(*value)?,
                Some(Cache::Tcl84Long(_) | Cache::Number(_))
            )
        {
            return Err(unavailable());
        }
        obj::invalidate_string(*value);
        Ok(())
    }
    fn retain_original_result(&mut self, value: &Self::Value) -> Result<Owned, CmdError> {
        obj::check_native_liveness(*value)?;
        Ok(Owned::retain(*value))
    }
    fn copy_api_result(
        &mut self,
        value: &Owned,
        protocol: ResultProtocol,
    ) -> Result<Owned, CmdError> {
        obj::check_native_liveness(value.as_ptr())?;
        if !protocol.copies_api_result() {
            return Err(unavailable());
        }
        Ok(Owned::fresh(obj::duplicate(value.as_ptr())))
    }
    fn nonfinite_expression_failure(
        &mut self,
        value: f64,
        protocol: ResultProtocol,
    ) -> Result<Option<CmdError>, CmdError> {
        let failure =
            if protocol.scalar_protocol().tcl_version() == Some(tcl_dialect::TclVersion::V8_4) {
                tcl_cmd_core::native_numeric::c84_nonfinite_error(
                    protocol.scalar_protocol(),
                    value,
                    self.environment()?
                        .as_ref()
                        .map(|actual| actual as &dyn tcl_platform::NumericEnvironment),
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

/// Reach a direct operand instruction in the actual entered interpreter.
/// Result-purpose tags cannot attest a missing evaluated-result producer.
pub(crate) fn native_boolean_for_interp(
    interp: &mut Interp,
    value: *mut TclObj,
    purpose: Purpose,
) -> Result<bool, CmdError> {
    let mut ops = OriginalOps::entered(interp)?;
    ops.check_host_refusal()?;
    if purpose.requires_expression_result() {
        return Err(unavailable());
    }
    let protocol = ops
        .dialect
        .native_boolean_truth_protocol(purpose)
        .ok_or_else(unavailable)?;
    original_boolean_truth(&value, protocol, &mut ops)
}

/// Explicit neutral C instruction adapter; Jim's interpreter effects refuse.
pub(crate) fn native_boolean_in(
    value: *mut TclObj,
    dialect: InvocationDialect,
    purpose: Purpose,
    environment: Option<&dyn tcl_platform::NumericEnvironment>,
) -> Result<bool, CmdError> {
    if purpose.requires_expression_result() {
        return Err(unavailable());
    }
    let protocol = dialect
        .native_boolean_truth_protocol(purpose)
        .ok_or_else(unavailable)?;
    original_boolean_truth(
        &value,
        protocol,
        &mut OriginalOps::neutral(dialect, environment),
    )
}

/// Perform the actual outer result conversion before Boolean extraction.
pub(crate) fn expression_boolean_for_interp(
    interp: &mut Interp,
    value: *mut TclObj,
    production: Production,
) -> Result<bool, CmdError> {
    let mut ops = OriginalOps::entered(interp)?;
    ops.check_host_refusal()?;
    let protocol = ops
        .dialect
        .native_boolean_expression_result_protocol(production)
        .ok_or_else(unavailable)?;
    let result = original_boolean_expression_result(&value, protocol, &mut ops)?;
    let truth = ops
        .dialect
        .native_boolean_truth_protocol(protocol.truth_purpose())
        .ok_or_else(unavailable)?;
    original_boolean_truth(&result.as_ptr(), truth, &mut ops)
}

/// Neutral physical result producer for an independently selected C context.
pub(crate) fn normalize_boolean_result_in(
    value: *mut TclObj,
    dialect: InvocationDialect,
    production: Production,
    environment: Option<&dyn tcl_platform::NumericEnvironment>,
) -> Result<Owned, CmdError> {
    let protocol = dialect
        .native_boolean_expression_result_protocol(production)
        .ok_or_else(unavailable)?;
    original_boolean_expression_result(
        &value,
        protocol,
        &mut OriginalOps::neutral(dialect, environment),
    )
}

/// Preserve original lhs then rhs conversion order at the selected final site.
pub(crate) fn native_logical_right_for_interp(
    interp: &mut Interp,
    left: *mut TclObj,
    right: *mut TclObj,
    conjunction: bool,
) -> Result<bool, CmdError> {
    let mut ops = OriginalOps::entered(interp)?;
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
        Some(original_boolean_truth(&left, protocol, &mut ops)?)
    } else {
        None
    };
    let right = original_boolean_truth(&right, protocol, &mut ops)?;
    ops.check_host_refusal()?;
    Ok(left.map_or(right, |left| {
        if conjunction {
            left && right
        } else {
            left || right
        }
    }))
}

/// Perform the selected genuine result production inside its active interpreter.
pub(crate) fn normalize_boolean_result_for_interp(
    interp: &mut Interp,
    value: *mut TclObj,
    production: Production,
) -> Result<Owned, CmdError> {
    let mut ops = OriginalOps::entered(interp)?;
    ops.check_host_refusal()?;
    let protocol = ops
        .dialect
        .native_boolean_expression_result_protocol(production)
        .ok_or_else(unavailable)?;
    original_boolean_expression_result(&value, protocol, &mut ops)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn native(profile: &str) -> Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            tcl_registry::model::ingress::resolve_environment(profile).unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }

    #[test]
    fn entered_number_callbacks_keep_actual_cache_and_first_host_identity() {
        // Software consumers: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        for public_result in [false, true] {
            let mut interp = native("tcl8.6");
            let original = Owned::fresh(obj::new_string_bytes(b"17"));
            let value = original.as_ptr();
            let references = unsafe { (*value).ref_count };
            let dialect = interp.native_invocation_dialect();
            {
                let mut ops = OriginalOps::entered(&mut interp).unwrap();
                if public_result {
                    let protocol = dialect
                        .native_boolean_expression_result_protocol(Production::InlineExpression)
                        .unwrap();
                    ops.original_number_probe(&value, protocol).unwrap();
                } else {
                    let protocol = dialect
                        .native_boolean_truth_protocol(Purpose::LogicalAnd)
                        .unwrap();
                    assert!(matches!(
                        ops.logical_number_probe(&value, protocol).unwrap(),
                        Ok(Number::Int(17))
                    ));
                }
            }
            assert!(matches!(
                obj::native_scalar_cache(value).unwrap(),
                Some(Cache::Number(Number::Int(17)))
            ));
            assert_eq!(obj::bytes_of(value), b"17");
            assert_eq!(unsafe { (*value).ref_count }, references);
            interp.refuse_host_command("original Number first cause");
            let first = interp.native_execution_refusal().unwrap();
            let before = obj::native_object_snapshot(value).unwrap();
            let error = match OriginalOps::entered(&mut interp) {
                Ok(_) => panic!("prior Host cannot enter either Number callback"),
                Err(error) => error,
            };
            assert_eq!(error.native_execution_refusal(), Some(&first));
            assert_eq!(obj::native_object_snapshot(value).unwrap(), before);
            assert_eq!(interp.native_execution_refusal(), Some(first));
        }
    }

    fn measured_jim_api(case: usize) -> (i32, i32, Vec<u8>) {
        let source = include_str!(
            "../../../../rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/jim/execute-direct.stdout"
        );
        let prefix = format!("ROW\tcase={case}\tworker=1\t");
        let row = source
            .lines()
            .find(|line| line.starts_with(&prefix))
            .expect("closed original public ExprBool row");
        let fields = row
            .split('\t')
            .skip(1)
            .map(|field| field.split_once('=').unwrap())
            .collect::<std::collections::HashMap<_, _>>();
        let message = fields["result"]
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        (
            fields["code"].parse().unwrap(),
            fields["out"].parse().unwrap(),
            message,
        )
    }

    struct ReentrantNumericHost {
        actual: Rc<dyn tcl_platform::Host>,
        interpreter: std::cell::RefCell<Option<Interp>>,
        calls: Cell<usize>,
    }

    impl tcl_platform::Host for ReentrantNumericHost {
        fn capabilities(&self) -> tcl_platform::Capabilities {
            self.actual.capabilities()
        }
        fn clock(&self) -> &dyn tcl_platform::Clock {
            self.actual.clock()
        }
        fn stdio(&self) -> &dyn tcl_platform::StdIo {
            self.actual.stdio()
        }
        fn env(&self) -> &dyn tcl_platform::Env {
            self.actual.env()
        }
        fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
            Some(self)
        }
    }

    impl ReentrantNumericHost {
        fn actual(&self) -> &dyn tcl_platform::NumericEnvironment {
            self.actual
                .numeric_environment()
                .expect("actual host numeric environment")
        }
    }

    impl tcl_platform::NumericEnvironment for ReentrantNumericHost {
        fn c_integer_abi(
            &self,
        ) -> Result<tcl_platform::NativeCIntegerAbi, tcl_platform::NumericEnvironmentUnavailable>
        {
            self.actual().c_integer_abi()
        }
        fn state(
            &self,
        ) -> Result<tcl_platform::NumericErrorState, tcl_platform::NumericEnvironmentUnavailable>
        {
            self.actual().state()
        }
        fn reset(&self) -> Result<(), tcl_platform::NumericEnvironmentUnavailable> {
            self.actual().reset()
        }
        fn unsigned(
            &self,
            input: &[u8],
            offset: usize,
            base: u32,
        ) -> Result<
            tcl_platform::UnsignedNumericConversion,
            tcl_platform::NumericEnvironmentUnavailable,
        > {
            self.actual().unsigned(input, offset, base)
        }
        fn signed_long(
            &self,
            input: &[u8],
            base: u32,
        ) -> Result<
            tcl_platform::SignedNumericConversion,
            tcl_platform::NumericEnvironmentUnavailable,
        > {
            let result = self.actual().signed_long(input, base);
            self.calls.set(self.calls.get() + 1);
            let mut interpreter = self.interpreter.borrow().as_ref().unwrap().clone();
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("expression-currency-control".to_owned(), "1.0".to_owned())];
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
            result
        }
        fn double(
            &self,
            input: &[u8],
            reset: bool,
        ) -> Result<
            tcl_platform::DoubleNumericConversion,
            tcl_platform::NumericEnvironmentUnavailable,
        > {
            self.actual().double(input, reset)
        }
    }

    #[test]
    fn original_host_change_and_restore_cannot_revive_entered_boolean_operation() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // The real Host conversion is reached, then a genuine same-profile
        // context change/restoration invalidates the existing guard epoch.
        // This does not claim an original C callback chronology.
        let mut interp = native("tcl8.4");
        let host = Rc::new(ReentrantNumericHost {
            actual: interp.host(),
            interpreter: std::cell::RefCell::new(None),
            calls: Cell::new(0),
        });
        interp.set_host(host.clone());
        *host.interpreter.borrow_mut() = Some(interp.clone());
        let context = interp.runtime_context();
        let profile = interp.dialect_profile();
        interp.set_result_bytes(b"PRIOR\0\xff");
        let prior = interp.get_obj_result();
        let original = Owned::fresh(obj::new_string_bytes(b"17"));
        let before = obj::native_object_snapshot(original.as_ptr()).unwrap();
        let mut out = 777;
        // SAFETY: actual interp, original object and output remain alive and
        // properly aligned; this independent test releases the ambient entry.
        unsafe {
            crate::codegen_abi::tcl_runtime_set_current_interp(&raw mut interp);
            assert_ne!(
                crate::codegen_abi::tcl_value_get_bool_for_purpose(
                    original.as_ptr(),
                    Purpose::LogicalAnd as i32,
                    &raw mut out
                ),
                0
            );
            crate::codegen_abi::tcl_runtime_set_current_interp(std::ptr::null_mut());
        }
        assert_eq!(host.calls.get(), 1);
        assert_eq!(out, 777);
        assert_eq!(interp.runtime_context(), context);
        assert!(std::ptr::eq(interp.dialect_profile(), profile));
        let first = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "stale entered native operation",
            ),
        );
        assert_eq!(interp.native_execution_refusal(), Some(first.clone()));
        assert_eq!(
            obj::native_object_snapshot(original.as_ptr()).unwrap(),
            before
        );
        assert_eq!(interp.get_obj_result(), prior);
        assert_eq!(interp.result_bytes(), b"PRIOR\0\xff");
        let error = native_boolean_for_interp(&mut interp, original.as_ptr(), Purpose::LogicalAnd)
            .unwrap_err();
        assert_eq!(error.native_execution_refusal(), Some(&first));
        assert_eq!(host.calls.get(), 1);
        // Release the test-owned callback handle; the currency receipt itself
        // keeps neither the host nor the interpreter alive.
        host.interpreter.borrow_mut().take();
    }

    #[test]
    fn original_jim_public_truth_preserves_exact_cached_value_and_intermediate_diagnostic() {
        // naming.numeric.original-primitive-boolean-vs-expression-truth
        // docs/design/analysis/name-resolution-proofs/numeric-original-primitive-boolean-vs-expression-truth.md
        // Compare only the measured direct public API out/result and original
        // post-conversion cache/String fields; no whole script/opcode claim.
        for (case, number, string) in [
            (19, Number::Int(4_294_967_296), false),
            (28, Number::Double(f64::NAN), true),
        ] {
            let mut interp = native("jim");
            interp.error_with_code(b"SEEDED RESULT", b"SEEDED CODE");
            let original = Owned::fresh(match number {
                Number::Int(integer) => obj::new_wide_int_obj(integer),
                Number::Double(double) => obj::new_double_obj(double),
                _ => unreachable!(),
            });
            assert!(!obj::has_string_rep(original.as_ptr()));
            let measured = measured_jim_api(case);
            assert_eq!(measured.0, 0);
            assert_eq!(
                expression_boolean_for_interp(
                    &mut interp,
                    original.as_ptr(),
                    Production::PublicExpressionApi
                )
                .unwrap(),
                measured.1 != 0
            );
            assert_eq!(obj::has_string_rep(original.as_ptr()), string);
            assert_eq!(interp.result_bytes(), measured.2);
            match (number, obj::native_scalar_cache(original.as_ptr()).unwrap()) {
                (Number::Int(expected), Some(Cache::Number(Number::Int(actual)))) => {
                    assert_eq!(actual, expected)
                }
                (Number::Double(expected), Some(Cache::Number(Number::Double(actual)))) => {
                    assert_eq!(actual.to_bits(), expected.to_bits())
                }
                other => panic!("original cache changed unexpectedly: {other:?}"),
            }
            assert!(!interp.host_refusal_pending());
            assert_eq!(interp.error_code_bytes_checked().unwrap(), b"SEEDED CODE");
        }
    }

    #[test]
    fn original_c84_logical_sides_keep_distinct_boolean_and_integer_primary() {
        // naming.numeric.original-logical-operand-truth-sites
        // docs/design/analysis/name-resolution-proofs/numeric-original-logical-operand-truth-sites.md
        // Bounded selected original465 case4 worker6/7 operand cache windows.
        // This stage assertion does not claim a whole expression result.
        let mut interp = native("tcl8.4");
        let left = Owned::fresh(obj::new_string_bytes(b"4294967296"));
        assert!(
            !native_boolean_for_interp(&mut interp, left.as_ptr(), Purpose::LogicalAnd).unwrap()
        );
        assert_eq!(
            obj::native_scalar_cache(left.as_ptr()).unwrap(),
            Some(Cache::WordBoolean(false))
        );
        let right = Owned::fresh(obj::new_string_bytes(b"4294967296"));
        assert!(native_boolean_for_interp(
            &mut interp,
            right.as_ptr(),
            Purpose::LogicalAndInstruction
        )
        .unwrap());
        assert_eq!(
            obj::native_scalar_cache(right.as_ptr()).unwrap(),
            Some(Cache::Tcl84Long(4_294_967_296))
        );
        assert_eq!(obj::bytes_of(left.as_ptr()), b"4294967296");
        assert_eq!(obj::bytes_of(right.as_ptr()), b"4294967296");
    }

    #[test]
    fn original_native_operand_retirement_and_first_host_stop_result_publication() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut interp = native("tcl8.6");
        interp.set_result_bytes(b"PRIOR\0\xff");
        let result = interp.get_obj_result();
        let original = Owned::fresh(obj::new_string_bytes(b"true"));
        let pointer = original.as_ptr();
        let lease = obj::NativeObjectLifetime::retain(pointer);
        drop(original);
        assert!(!obj::allocation_is_live(pointer));
        let error =
            expression_boolean_for_interp(&mut interp, pointer, Production::InlineExpression)
                .unwrap_err();
        assert_eq!(
            error.native_access_refusal(),
            Some(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "retired native object"
                )
            )
        );
        interp.report_cmd_error(error);
        let first = interp.native_execution_refusal().unwrap();
        let live = Owned::fresh(obj::new_string_bytes(b"yes"));
        let error = expression_boolean_for_interp(
            &mut interp,
            live.as_ptr(),
            Production::PublicExpressionApi,
        )
        .unwrap_err();
        assert_eq!(error.native_execution_refusal(), Some(&first));
        assert!(obj::obj_type_ptr(live.as_ptr()).is_null());
        assert_eq!(interp.get_obj_result(), result);
        assert_eq!(interp.result_bytes(), b"PRIOR\0\xff");
        drop(lease);
    }

    #[test]
    fn public_boolean_tags_cannot_attest_normalisation_or_default_unknown_purpose() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        for tag in [4, 5, 99] {
            let mut interp = native("tcl8.6");
            interp.set_result_bytes(b"PRIOR\0\xff");
            let result = interp.get_obj_result();
            let original = Owned::fresh(obj::new_string_bytes(b"on"));
            let mut out = 777;
            // SAFETY: this native interpreter and original value stay alive;
            // out is properly aligned, writable local storage. Reset only the
            // independent test entry; the getter never resets first Host.
            unsafe {
                crate::codegen_abi::tcl_runtime_set_current_interp(&raw mut interp);
                assert_ne!(
                    crate::codegen_abi::tcl_value_get_bool_for_purpose(
                        original.as_ptr(),
                        tag,
                        &raw mut out
                    ),
                    0
                );
                crate::codegen_abi::tcl_runtime_set_current_interp(std::ptr::null_mut());
            }
            assert_eq!(out, 777);
            assert!(interp.host_refusal_pending());
            assert!(obj::obj_type_ptr(original.as_ptr()).is_null());
            assert_eq!(interp.get_obj_result(), result);
            assert_eq!(interp.result_bytes(), b"PRIOR\0\xff");
        }
    }
}
