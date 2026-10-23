// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected math operands over the entered value and pure function owners.

use super::{
    native_numeric::{kernel_operand, NumericOperation},
    ExprCtx, ExprError, Owned,
};
use crate::obj::TclObj;
use tcl_syntax::expr::mathfunc::{NativeMathProtocol, NumValue};
use tcl_syntax::number::Number;

pub(crate) struct MathOperation {
    original: NumericOperation,
    protocol: NativeMathProtocol,
}
impl MathOperation {
    pub(crate) fn enter(context: &mut dyn ExprCtx) -> Result<Self, ExprError> {
        let original = NumericOperation::enter(context)?;
        let protocol = tcl_registry::mathfunc::native_math_protocol(original.dialect())
            .ok_or_else(|| {
                ExprError::from_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original math function protocol",
                    )
                    .into(),
                )
            })?;
        Ok(Self { original, protocol })
    }
    pub(crate) fn check(&self) -> Result<(), ExprError> {
        self.original.check()
    }
    pub(crate) fn after<T>(&self, result: Result<T, ExprError>) -> Result<T, ExprError> {
        self.original.after(result)
    }
    pub(crate) fn string(&self, value: *mut TclObj) -> Result<Vec<u8>, ExprError> {
        self.original.string(value)
    }
    pub(crate) fn dialect(&self) -> tcl_registry::InvocationDialect {
        self.original.dialect()
    }
    /// Complete selected classification before a subsequent operand is reached.
    pub(crate) fn number(&self, value: *mut TclObj) -> Result<Option<Number>, ExprError> {
        let result = self.original.with_environment(|environment| {
            if self.protocol == NativeMathProtocol::Tcl
                && self.dialect().arithmetic() == Some(tcl_dialect::NativeArithmetic::Tcl84Wide)
            {
                if let Some(integer) = super::fixed_integer_with_currency(
                    value,
                    self.dialect(),
                    environment,
                    self.original.currency(),
                )? {
                    return Ok(Some(Number::Int(integer)));
                }
            }
            self.original.number(value)
        });
        self.after(result)
    }
    fn mathematical_operand(
        &self,
        number: Number,
    ) -> Result<Option<NumValue<crate::bignum::TowerMp>>, ExprError> {
        if self.protocol == NativeMathProtocol::Jim084 {
            return Ok(tcl_syntax::expr::mathfunc::jim_numeric_operand(&number));
        }
        let kernel = kernel_operand(number)?;
        // This existing kernel sees a fresh accepted numeric primary. It does
        // not read or reparse the original source spelling.
        Ok(crate::bignum::as_math_num(kernel.ptr()))
    }
    fn boolean_operand(&self, value: *mut TclObj) -> Result<Number, ExprError> {
        self.original.with_environment(|environment| {
            use tcl_syntax::scalar_getter::{
                NativeScalarGetterKind as Getter, NativeScalarGetterValue as Value,
            };
            let outcome = crate::typed_value::native_scalar_probe_with_environment_and_currency(
                value,
                self.dialect(),
                Getter::Boolean,
                environment,
                self.original.currency(),
            );
            self.check()?;
            match outcome.map_err(|error| ExprError::from_cmd_error(error.into()))? {
                Ok(Value::Boolean(value)) => Ok(Number::Int(i64::from(value.is_true()))),
                Ok(_) => Err(ExprError::from_cmd_error(
                    tcl_syntax::value::ValueError::ScalarNumericInputUnavailable.into(),
                )),
                Err(failure) => {
                    let presentation = crate::typed_value::native_scalar_failure_presentation(
                        value,
                        self.dialect(),
                        Getter::Boolean,
                        failure,
                    );
                    self.check()?;
                    Err(ExprError::from_cmd_error(
                        tcl_syntax::value::ValueError::NativeScalarGetter(Box::new(
                            presentation
                                .map_err(|error| ExprError::from_cmd_error(error.into()))?,
                        ))
                        .into(),
                    ))
                }
            }
        })
    }
    /// The error the release raises for an argument of the math function
    /// `name` that is no number
    /// ([`tcl_syntax::expr::mathfunc::non_numeric_argument_error`]): 8.4's one
    /// message, and from 8.5 the reader's own words with `TCL VALUE NUMBER`.
    fn non_numeric_argument(&self, name: &str, argument: *mut TclObj) -> ExprError {
        let release = self
            .dialect()
            .tcl_version
            .unwrap_or_else(tcl_syntax::expr::errors::ambient_release);
        let unread = crate::obj::bytes_of(argument);
        let (message, code) = tcl_syntax::expr::mathfunc::non_numeric_argument_error(
            &name.to_ascii_lowercase(),
            &String::from_utf8_lossy(&unread),
            release,
        );
        ExprError::from_parts(message.into_bytes(), code.as_bytes().to_vec())
    }

    pub(crate) fn dispatch(&self, name: &str, arguments: &[Owned]) -> Result<Owned, ExprError> {
        let result = (|| {
            let mut numbers = Vec::with_capacity(arguments.len());
            for argument in arguments {
                let number = if tcl_syntax::expr::mathfunc::accepts_boolean_operand(name) {
                    Some(self.boolean_operand(argument.ptr())?)
                } else {
                    self.number(argument.ptr())?
                };
                let number =
                    number.ok_or_else(|| self.non_numeric_argument(name, argument.ptr()))?;
                numbers.push(number);
            }
            self.dispatch_numbers(
                name,
                numbers,
                &arguments.iter().map(Owned::as_ptr).collect::<Vec<_>>(),
            )
        })();
        self.after(result)
    }
    /// Dispatch already accepted mathematical magnitudes once; identity-forward
    /// callers separately retain the original operand and its resident spelling.
    pub(crate) fn dispatch_numbers(
        &self,
        name: &str,
        numbers: Vec<Number>,
        originals: &[*mut TclObj],
    ) -> Result<Owned, ExprError> {
        let result = (|| {
            let mut operands = Vec::with_capacity(numbers.len());
            for (index, number) in numbers.into_iter().enumerate() {
                operands.push(self.mathematical_operand(number)?.ok_or_else(|| {
                    match originals.get(index) {
                        Some(&original) => self.non_numeric_argument(name, original),
                        None => {
                            ExprError::msg(b"argument to math function didn't have numeric value")
                        }
                    }
                })?);
            }
            self.check()?;
            match tcl_syntax::expr::mathfunc::try_dispatch_with_backend_protocol(
                &name.to_ascii_lowercase(),
                &operands,
                super::int_width_for_dialect(self.dialect()),
                self.protocol,
            ) {
                Ok(NumValue::Int(integer))
                    if name == "abs"
                        && self.dialect().arithmetic()
                            == Some(tcl_dialect::NativeArithmetic::Tcl84Wide) =>
                {
                    super::native_integer_result(self.dialect(), integer, originals)
                }
                Ok(number) => super::native_math_result(number, self.dialect()).map(Owned::fresh),
                Err(tcl_syntax::expr::mathfunc::MathFuncError::UnknownFunction) => {
                    let mut message = b"unknown math function \"".to_vec();
                    message.extend_from_slice(name.as_bytes());
                    message.push(b'"');
                    Err(ExprError::from_bytes(message))
                }
                Err(error) => Err(super::math_func_err(error)),
            }
        })();
        self.after(result)
    }
    /// Convert an accepted integer magnitude through the existing bit-window
    /// kernel. No rejected original spelling reaches that kernel.
    pub(crate) fn integer_window(&self, number: Number) -> Result<i64, ExprError> {
        let kernel = kernel_operand(number)?;
        let result = crate::bignum::truncate_to_wide(kernel.ptr());
        self.after(Ok(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interp::{Code, Interp};
    use crate::obj;
    use std::cell::Cell;
    use tcl_runtime_api::NativeExecutionError;
    thread_local! { static CALLS: Cell<usize> = const { Cell::new(0) }; }
    extern "C" fn updater(value: *mut TclObj) {
        CALLS.with(|calls| calls.set(calls.get() + 1));
        let mut interpreter = unsafe { crate::codegen_abi::current_interp().as_ref() }
            .unwrap()
            .clone();
        interpreter.refuse_host_command("first original math operand updater");
        unsafe { obj::set_native_updater_string_rep(value, b"17", false) };
    }
    static ORIGINAL: obj::TclObjType = obj::TclObjType {
        name: c"originalMathOperandRefusal".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(updater),
        set_from_any_proc: None,
    };
    fn native(profile: &str) -> crate::interp::Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            tcl_registry::model::ingress::resolve_environment(profile).unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }
    #[test]
    fn original_math_operand_first_host_stops_before_later_conversion_and_result() {
        // naming.embedding.original-host-publication-and-fact-transport

        // Reached software getter effects, not a native math function/errno observation.
        let mut interpreter = native("tcl8.6");
        interpreter.set_result_bytes(b"PRIOR\0\xff");
        let prior = interpreter.get_obj_result();
        let first = Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
        let later = Owned::fresh(obj::new_string_bytes(b"2"));
        let snapshot = obj::native_object_snapshot(later.ptr()).unwrap();
        CALLS.with(|calls| calls.set(0));
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
        let operation = crate::builtins::math_operation(&mut interpreter).unwrap();
        let error = operation
            .dispatch("pow", &[first.clone(), later.clone()])
            .err()
            .expect("original math operation must refuse");
        let cause = interpreter.native_execution_refusal().unwrap();
        assert!(
            matches!(&cause, NativeExecutionError::HostCommandRefusal(first) if first.reason == "first original math operand updater")
        );
        assert_eq!(error.native_execution_refusal, Some(cause));
        assert_eq!(CALLS.with(Cell::get), 1);
        assert_eq!(obj::bytes_of(first.ptr()), b"17");
        assert_eq!(obj::obj_type_ptr(first.ptr()), &ORIGINAL as *const _);
        assert_eq!(obj::native_object_snapshot(later.ptr()).unwrap(), snapshot);
        assert_eq!(unsafe { (*first.ptr()).ref_count }, 1);
        assert_eq!(unsafe { (*later.ptr()).ref_count }, 1);
        assert_eq!(interpreter.get_obj_result(), prior);
        drop(operation);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
    }
    #[test]
    fn original_math_rejection_never_reparses_selected_spelling_or_reaches_later_operand() {
        // naming.embedding.original-host-publication-and-fact-transport

        // A selected C84 software rejection and unreached right updater; no full-release comparator.
        let mut interpreter = native("tcl8.4");
        let rejected = Owned::fresh(obj::new_string_bytes(b"0b10"));
        let later = Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
        let snapshot = obj::native_object_snapshot(later.ptr()).unwrap();
        CALLS.with(|calls| calls.set(0));
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
        let operation = crate::builtins::math_operation(&mut interpreter).unwrap();
        let error = operation
            .dispatch("pow", &[rejected.clone(), later.clone()])
            .err()
            .expect("original math operation must refuse");
        assert!(error.native_execution_refusal.is_none());
        assert!(error.native_access_refusal.is_none());
        assert_eq!(CALLS.with(Cell::get), 0);
        assert_eq!(obj::native_object_snapshot(later.ptr()).unwrap(), snapshot);
        assert_eq!(obj::bytes_of(rejected.ptr()), b"0b10");
        assert!(obj::native_scalar_cache(rejected.ptr()).unwrap().is_none());
        assert!(!interpreter.host_refusal_pending());
        drop(operation);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
    }
    #[test]
    fn original_math_forwarding_keeps_original_header_and_boolean_uses_primitive_purpose() {
        // naming.embedding.original-host-publication-and-fact-transport

        // Current software function/result producers, no external math-function equivalence claim.
        let mut interpreter = native("tcl8.6");
        let head = Owned::fresh(obj::new_string_bytes(b"::tcl::mathfunc::entier"));
        let integer = Owned::fresh(obj::new_string_bytes(b"0x10"));
        let code = crate::cmd_mathfunc::mathfunc_identity(
            &mut interpreter,
            &[head.ptr(), integer.ptr()],
            b"::tcl::mathfunc::entier",
        );
        assert_eq!(code, Code::Ok);
        assert_eq!(interpreter.get_obj_result(), integer.ptr());
        assert_eq!(obj::bytes_of(integer.ptr()), b"0x10");
        assert_eq!(unsafe { (*integer.ptr()).ref_count }, 2);
        let boolean = Owned::fresh(obj::new_string_bytes(b"true"));
        let operation = crate::builtins::math_operation(&mut interpreter).unwrap();
        let result = operation.dispatch("bool", &[boolean.clone()]).unwrap();
        assert_eq!(obj::bytes_of(result.ptr()), b"1");
        assert_eq!(obj::bytes_of(boolean.ptr()), b"true");
        assert_eq!(unsafe { (*boolean.ptr()).ref_count }, 1);
        drop(operation);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        assert!(!interpreter.host_refusal_pending());
    }
    #[test]
    fn original_stateful_math_seed_first_host_preserves_prior_result_and_generator_state() {
        // naming.embedding.original-host-publication-and-fact-transport

        // Original software seed getter stops before the real PRNG mutation.
        let mut interpreter = native("tcl8.6");
        let control = native("tcl8.6");
        let _ = interpreter.srand(123);
        let _ = control.srand(123);
        interpreter.set_result_bytes(b"PRIOR\0\xff");
        let prior = interpreter.get_obj_result();
        let head = Owned::fresh(obj::new_string_bytes(b"::tcl::mathfunc::srand"));
        let seed = Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
        CALLS.with(|calls| calls.set(0));
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
        let code = crate::cmd_mathfunc::mathfunc_identity(
            &mut interpreter,
            &[head.ptr(), seed.ptr()],
            b"::tcl::mathfunc::srand",
        );
        assert_eq!(code, Code::Error);
        assert!(
            matches!(interpreter.native_execution_refusal(), Some(NativeExecutionError::HostCommandRefusal(first)) if first.reason == "first original math operand updater")
        );
        assert_eq!(CALLS.with(Cell::get), 1);
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_eq!(
            interpreter.rand_next().to_bits(),
            control.rand_next().to_bits()
        );
        assert_eq!(unsafe { (*seed.ptr()).ref_count }, 1);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
    }
}
