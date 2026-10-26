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

//! Entered arithmetic/comparison ingress, separate from numeric recipes.
//! The scope retains the actual interpreter through callbacks and cleanup;
//! typed temporary kernel operands carry accepted magnitudes, not authority.

use super::{ExprCtx, ExprError, Owned};
use crate::interp::native_operation_currency::{
    CheckedNumericEnvironment, NativeOperationCurrency, NativeOperationScope,
};
use crate::obj::{self, TclObj};
use std::{cell::Cell, rc::Rc};
use tcl_syntax::expr::{BinOp, UnaryOp};
use tcl_syntax::number::Number;

pub(super) struct NumericOperation {
    scope: Option<NativeOperationScope>,
    interpreter: Option<crate::interp::Interp>,
    dialect: tcl_registry::InvocationDialect,
    host: Option<Rc<dyn tcl_platform::Host>>,
    abi: Cell<Option<tcl_platform::NativeCIntegerAbi>>,
}
impl NumericOperation {
    pub(super) fn enter(ctx: &mut dyn ExprCtx) -> Result<Self, ExprError> {
        let operation = Self::enter_values(ctx)?;
        operation.dialect.arithmetic().ok_or_else(|| {
            ExprError::host_refusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::ScalarNumericInputUnavailable,
            )
        })?;
        Ok(operation)
    }
    /// Retain the genuine entered owner without granting a numeric purpose.
    pub(super) fn enter_values(ctx: &mut dyn ExprCtx) -> Result<Self, ExprError> {
        let interpreter = ctx.boolean_interpreter().map(|interp| interp.clone());
        let scope = interpreter
            .as_ref()
            .map(NativeOperationScope::enter)
            .transpose()
            .map_err(ExprError::from_execution_refusal)?;
        let operation = Self {
            scope,
            interpreter,
            dialect: ctx.invocation_dialect(),
            host: ctx.numeric_host(),
            abi: Cell::new(None),
        };
        operation.check()?;
        Ok(operation)
    }
    pub(super) fn interpreter(&self) -> Option<&crate::interp::Interp> {
        self.interpreter.as_ref()
    }
    pub(super) fn dialect(&self) -> tcl_registry::InvocationDialect {
        self.dialect
    }
    pub(super) fn currency(&self) -> Option<&NativeOperationCurrency> {
        self.scope.as_ref().map(NativeOperationScope::currency)
    }
    pub(super) fn check(&self) -> Result<(), ExprError> {
        self.currency().map_or(Ok(()), |currency| {
            currency
                .ensure_current_or_refuse()
                .map_err(ExprError::from_execution_refusal)
        })
    }
    pub(super) fn after<T>(&self, result: Result<T, ExprError>) -> Result<T, ExprError> {
        self.check()?;
        result
    }
    pub(super) fn with_environment<T>(
        &self,
        operation: impl FnOnce(Option<&dyn tcl_platform::NumericEnvironment>) -> Result<T, ExprError>,
    ) -> Result<T, ExprError> {
        self.check()?;
        let environment = self
            .host
            .as_ref()
            .and_then(|host| host.numeric_environment());
        self.check()?;
        let checked = environment
            .map(|actual| CheckedNumericEnvironment::new(actual, self.currency(), &self.abi));
        let result = operation(
            checked
                .as_ref()
                .map(|checked| checked as &dyn tcl_platform::NumericEnvironment),
        );
        self.after(result)
    }
    pub(super) fn string(&self, value: *mut TclObj) -> Result<Vec<u8>, ExprError> {
        super::original_operand_bytes(value, self.dialect, self.currency())
    }
    pub(super) fn number(&self, value: *mut TclObj) -> Result<Option<Number>, ExprError> {
        self.check()?;
        let number = if self
            .dialect
            .native_scalar_getter_protocol()
            .is_some_and(|protocol| protocol.supports_number_getter())
        {
            let result = crate::typed_value::native_number_probe_with_currency(
                value,
                self.dialect,
                tcl_syntax::scalar_getter::NativeNumberGetterKind::Number,
                self.currency(),
            );
            self.check()?;
            result
                .map(|outcome| outcome.ok())
                .map_err(|error| ExprError::from_cmd_error(error.into()))
        } else {
            let result = crate::typed_value::scalar_number_with_currency(
                value,
                self.dialect,
                false,
                self.currency(),
            );
            self.check()?;
            result.map_err(ExprError::from_execution_refusal)
        };
        self.after(number)
    }
}

/// Physical arithmetic kernel inputs are already classified by the selected
/// owner. Their fresh numeric primaries prevent a second ambient String parse.
pub(super) fn kernel_operand(number: Number) -> Result<Owned, ExprError> {
    let value = match number {
        Number::Int(value) => obj::new_wide_int_obj(value),
        Number::Double(value) => obj::new_double_obj(value),
        Number::Big {
            negative,
            radix,
            digits,
        } => crate::bignum::from_big_digits(negative, radix, &digits),
        Number::Nan { negative, payload } => obj::new_double_obj(f64::from_bits(
            (u64::from(negative) << 63)
                | 0x7ff8_0000_0000_0000
                | (payload.unwrap_or(0) & 0x0007_ffff_ffff_ffff),
        )),
    };
    if value.is_null() {
        return Err(super::arith_err(crate::bignum::ArithError::Alloc));
    }
    Ok(Owned::fresh(value))
}

fn accepted_number(
    operation: &NumericOperation,
    value: *mut TclObj,
    fixed: Option<i64>,
) -> Result<Option<Number>, ExprError> {
    match fixed {
        Some(integer) => Ok(Some(Number::Int(integer))),
        None => operation.number(value),
    }
}
fn illegal_binary<T>(
    operation: &NumericOperation,
    value: *mut TclObj,
    side: tcl_syntax::expr::errors::OperandSide,
    op: BinOp,
    stage: tcl_registry::native_numeric_error::NativeExpressionOperandStage,
    floating_operand: bool,
) -> Result<T, ExprError> {
    let bytes = operation.string(value)?;
    let error = match operation.dialect().expression_operand_error_presentation() {
        Some(presentation) => ExprError::from_bytes(presentation.message(stage, &bytes)),
        None => super::operand_type_err(
            super::operand_desc(value, floating_operand, &bytes, operation.dialect()),
            &bytes,
            side,
            super::binop_sym(op),
            operation.dialect(),
            stage,
        ),
    };
    operation.check()?;
    Err(error)
}
fn binary_operand(
    operation: &NumericOperation,
    op: BinOp,
    value: *mut TclObj,
    side: tcl_syntax::expr::errors::OperandSide,
    environment: Option<&dyn tcl_platform::NumericEnvironment>,
    fixed: bool,
) -> Result<(Option<i64>, Number), ExprError> {
    let integer = if fixed {
        super::fixed_integer_with_currency(
            value,
            operation.dialect(),
            environment,
            operation.currency(),
        )
        .map_err(|error| super::operand_overflow(error, super::binop_sym(op)))?
    } else {
        None
    };
    use tcl_registry::native_numeric_error::NativeExpressionOperandStage as Stage;
    let integer_required = matches!(
        op,
        BinOp::Mod | BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::LShift | BinOp::RShift
    );
    let Some(number) = accepted_number(operation, value, integer)? else {
        let stage = if integer_required {
            Stage::Integer
        } else {
            Stage::FloatingPoint
        };
        return illegal_binary(operation, value, side, op, stage, false);
    };
    if integer_required && matches!(number, Number::Double(_) | Number::Nan { .. }) {
        return illegal_binary(operation, value, side, op, Stage::Integer, true);
    }
    Ok((integer, number))
}

fn binary_kernel(op: BinOp, left: &Owned, right: &Owned) -> Result<Owned, ExprError> {
    let result = match op {
        BinOp::Add => crate::bignum::add(left.ptr(), right.ptr()),
        BinOp::Sub => crate::bignum::sub(left.ptr(), right.ptr()),
        BinOp::Mul => crate::bignum::mul(left.ptr(), right.ptr()),
        BinOp::Div => crate::bignum::div(left.ptr(), right.ptr()),
        BinOp::Mod => crate::bignum::mod_(left.ptr(), right.ptr()),
        BinOp::Pow => crate::bignum::pow(left.ptr(), right.ptr()),
        BinOp::BitAnd => crate::bignum::band(left.ptr(), right.ptr()),
        BinOp::BitOr => crate::bignum::bor(left.ptr(), right.ptr()),
        BinOp::BitXor => crate::bignum::bxor(left.ptr(), right.ptr()),
        BinOp::LShift => crate::bignum::shl(left.ptr(), right.ptr()),
        BinOp::RShift => crate::bignum::shr(left.ptr(), right.ptr()),
        _ => return Err(ExprError::msg(b"unsupported operator")),
    };
    result.map(Owned::fresh).map_err(super::arith_err)
}

/// Enter selected operand conversion once, then give the magnitude-only
/// arithmetic kernel fresh accepted primaries. A rejected original String
/// cannot be reparsed under another ambient release.
pub(super) fn binary(
    ctx: &mut dyn ExprCtx,
    op: BinOp,
    left: Owned,
    right: Owned,
) -> Result<Owned, ExprError> {
    let operation = NumericOperation::enter(ctx)?;
    let result = operation.with_environment(|environment| {
        let dialect = operation.dialect();
        let fixed = dialect
            .arithmetic()
            .filter(|policy| *policy != tcl_dialect::NativeArithmetic::TclBignum);
        let (left_integer, left_number) = binary_operand(
            &operation,
            op,
            left.ptr(),
            tcl_syntax::expr::errors::OperandSide::Left,
            environment,
            fixed.is_some(),
        )?;
        let (right_integer, right_number) = binary_operand(
            &operation,
            op,
            right.ptr(),
            tcl_syntax::expr::errors::OperandSide::Right,
            environment,
            fixed.is_some(),
        )?;
        if let (Some(policy), Some(left_integer), Some(right_integer)) =
            (fixed, left_integer, right_integer)
        {
            let integer = tcl_syntax::expr::wide::binary(policy, op, left_integer, right_integer)
                .map_err(|error| super::wide_error(error, policy))?;
            operation.check()?;
            return super::native_integer_result(dialect, integer, &[left.ptr(), right.ptr()]);
        }
        let left_kernel = kernel_operand(left_number)?;
        let right_kernel = kernel_operand(right_number)?;
        operation.check()?;
        binary_kernel(op, &left_kernel, &right_kernel)
    });
    drop(left);
    drop(right);
    operation.after(result)
}
/// Arithmetic unary stages share entered operand conversion and retirement.
/// Logical NOT uses its independently selected Boolean operand owner.
pub(super) fn unary(ctx: &mut dyn ExprCtx, op: UnaryOp, value: Owned) -> Result<Owned, ExprError> {
    let operation = NumericOperation::enter(ctx)?;
    let result = operation.with_environment(|environment| {
        let dialect = operation.dialect();
        if let Some(policy) = dialect
            .arithmetic()
            .filter(|policy| *policy != tcl_dialect::NativeArithmetic::TclBignum)
        {
            if let Some(integer) = super::fixed_integer_with_currency(
                value.ptr(),
                dialect,
                environment,
                operation.currency(),
            )? {
                let integer = tcl_syntax::expr::wide::unary(policy, op, integer)
                    .map_err(|error| super::wide_error(error, policy))?;
                operation.check()?;
                return super::native_integer_result(dialect, integer, &[value.ptr()]).map(Some);
            }
        }
        let Some(number) = operation.number(value.ptr())? else {
            return Err(super::selected_unary_error(
                crate::bignum::ArithError::NonNumeric,
                op,
                value.ptr(),
                dialect,
                environment,
                operation.currency(),
            ));
        };
        if op == UnaryOp::BitNot && matches!(number, Number::Double(_) | Number::Nan { .. }) {
            return Err(super::selected_unary_error(
                crate::bignum::ArithError::NonInteger,
                op,
                value.ptr(),
                dialect,
                environment,
                operation.currency(),
            ));
        }
        let kernel = kernel_operand(number)?;
        operation.check()?;
        let result = match op {
            UnaryOp::Pos => {
                if !matches!(
                    crate::bignum::compare(kernel.ptr(), kernel.ptr()),
                    Some(tcl_syntax::expr::NumericCompare::Ordered(_))
                ) {
                    return Err(super::selected_unary_error(
                        crate::bignum::ArithError::NonNumeric,
                        op,
                        value.ptr(),
                        dialect,
                        environment,
                        operation.currency(),
                    ));
                }
                return Ok(None);
            }
            UnaryOp::Neg => crate::bignum::neg(kernel.ptr()),
            UnaryOp::BitNot => crate::bignum::bnot(kernel.ptr()),
            _ => return Err(ExprError::msg(b"unsupported operator")),
        };
        result
            .map(|value| Some(Owned::fresh(value)))
            .map_err(super::arith_err)
    });
    match result {
        Ok(None) => operation.after(Ok(value)),
        Ok(Some(result)) => {
            drop(value);
            operation.after(Ok(result))
        }
        Err(error) => {
            drop(value);
            operation.after(Err(error))
        }
    }
}

pub(super) fn retained_i64_add(
    ctx: &mut dyn ExprCtx,
    left: Owned,
    right: Owned,
    primitive: impl FnOnce(&mut dyn ExprCtx) -> Result<bool, ExprError>,
) -> Result<(Owned, bool), ExprError> {
    let operation = NumericOperation::enter(ctx)?;
    let result = operation.with_environment(|environment| {
        if operation.dialect().arithmetic() != Some(tcl_dialect::NativeArithmetic::TclBignum) {
            return Err(ExprError::host_refusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::ScalarNumericInputUnavailable,
            ));
        }
        let (_, left_number) = binary_operand(
            &operation,
            BinOp::Add,
            left.ptr(),
            tcl_syntax::expr::errors::OperandSide::Left,
            environment,
            false,
        )?;
        let (_, right_number) = binary_operand(
            &operation,
            BinOp::Add,
            right.ptr(),
            tcl_syntax::expr::errors::OperandSide::Right,
            environment,
            false,
        )?;
        let integer = match (&left_number, &right_number) {
            (Number::Int(left), Number::Int(right)) => left.checked_add(*right),
            _ => None,
        };
        operation.check()?;
        let primitive = primitive(ctx)?;
        if let Some(integer) = integer.filter(|_| primitive) {
            return super::native_integer_result(
                operation.dialect(),
                integer,
                &[left.ptr(), right.ptr()],
            )
            .map(|value| (value, true));
        }
        // A reached Host stage may have changed an accepted operand. Complete
        // from those same retained originals instead of replaying their reads.
        let left_kernel = kernel_operand(left_number)?;
        let right_kernel = kernel_operand(right_number)?;
        operation.check()?;
        binary_kernel(BinOp::Add, &left_kernel, &right_kernel).map(|value| (value, false))
    });
    drop(left);
    drop(right);
    operation.after(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::{ExprOps, TowerOps};
    use crate::interp::Interp;
    use tcl_runtime_api::NativeExecutionError;
    use tcl_syntax::raw_string::NativeValueAccessRefusal;
    thread_local! { static CALLS: Cell<usize> = const { Cell::new(0) }; }
    extern "C" fn updater(value: *mut TclObj) {
        CALLS.with(|calls| calls.set(calls.get() + 1));
        let mut interpreter = unsafe { crate::codegen_abi::current_interp().as_ref() }
            .unwrap()
            .clone();
        let original = interpreter.runtime_context();
        let mut changed = original.clone();
        changed.packages = vec![("comparison-updater".into(), "1.0".into())];
        interpreter.pin_context(&changed).unwrap();
        interpreter.pin_context(&original).unwrap();
        unsafe { obj::set_native_updater_string_rep(value, b"17", false) };
    }
    static ORIGINAL: obj::TclObjType = obj::TclObjType {
        name: c"originalNumericComparison".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(updater),
        set_from_any_proc: None,
    };
    struct Context(Interp);
    impl ExprCtx for Context {
        fn boolean_interpreter(&mut self) -> Option<&mut Interp> {
            Some(&mut self.0)
        }
        fn numeric_host(&self) -> Option<Rc<dyn tcl_platform::Host>> {
            Some(self.0.host())
        }
        fn invocation_dialect(&self) -> tcl_registry::InvocationDialect {
            self.0.native_invocation_dialect()
        }
        fn read_var(&mut self, _: &str) -> Result<Owned, ExprError> {
            Err(ExprError::msg(b"unexpected variable"))
        }
        fn eval_command(&mut self, _: &str) -> Result<Owned, ExprError> {
            Err(ExprError::msg(b"unexpected command"))
        }
        fn call_function(&mut self, _: &str, _: &[Owned]) -> Result<Owned, ExprError> {
            Err(ExprError::msg(b"unexpected function"))
        }
    }
    fn native() -> Context {
        Context(
            Interp::with_native_core(
                crate::interp::default_host(),
                tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile(),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap(),
        )
    }
    #[test]
    fn original_comparison_updater_stops_before_second_operand_cache_and_lexical_fallback() {

        // Reached software updater/ABA; neither a native numeric route nor opcode observation.
        let mut context = native();
        context.0.set_result_bytes(b"PRIOR\0\xff");
        let prior = context.0.get_obj_result();
        let original_context = context.0.runtime_context();
        let left = Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
        let right = Owned::fresh(obj::new_string_bytes(b"17"));
        let before = obj::native_object_snapshot(right.ptr()).unwrap();
        CALLS.with(|calls| calls.set(0));
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut context.0);
        let mut ops = TowerOps {
            ctx: &mut context,
            jim: None,
            safe: false,
        };
        let error = ops.compare_numeric_checked(&left, &right).unwrap_err();
        drop(ops);
        let first = NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::CommandProtocolUnavailable("stale entered native operation"),
        );
        assert_eq!(error.native_execution_refusal, Some(first.clone()));
        assert_eq!(context.0.native_execution_refusal(), Some(first));
        assert_eq!(context.0.runtime_context(), original_context);
        assert_eq!(context.0.get_obj_result(), prior);
        assert_eq!(context.0.result_bytes(), b"PRIOR\0\xff");
        CALLS.with(|calls| assert_eq!(calls.get(), 1));
        assert_eq!(obj::bytes_of(left.ptr()), b"17");
        assert_eq!(obj::obj_type_ptr(left.ptr()), &ORIGINAL as *const _);
        assert_eq!(obj::native_object_snapshot(right.ptr()).unwrap(), before);
        assert_eq!(unsafe { (*left.ptr()).ref_count }, 1);
        assert_eq!(unsafe { (*right.ptr()).ref_count }, 1);
        crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
    }
    #[test]
    fn original_comparison_keeps_current_numeric_and_genuine_lexical_acceptance() {

        // Software acceptance control of actual selected owner; no provider equivalence.
        let mut context = native();
        let left = Owned::fresh(obj::new_string_bytes(b"2"));
        let right = Owned::fresh(obj::new_string_bytes(b"10"));
        let mut ops = TowerOps {
            ctx: &mut context,
            jim: None,
            safe: false,
        };
        assert_eq!(
            ops.compare_numeric_checked(&left, &right).unwrap(),
            Some(tcl_syntax::expr::NumericCompare::Ordered(
                core::cmp::Ordering::Less
            ))
        );
        let left = Owned::fresh(obj::new_string_bytes(b"left"));
        let right = Owned::fresh(obj::new_string_bytes(b"right"));
        assert_eq!(ops.compare_numeric_checked(&left, &right).unwrap(), None);
        assert!(ops.compare_string(&left, &right).unwrap().is_lt());
        drop(ops);
        assert!(!context.0.host_refusal_pending());
    }
    extern "C" fn first_host_updater(value: *mut TclObj) {
        CALLS.with(|calls| calls.set(calls.get() + 1));
        let mut interpreter = unsafe { crate::codegen_abi::current_interp().as_ref() }
            .unwrap()
            .clone();
        interpreter.refuse_host_command("first scalar preparation updater refusal");
        unsafe { obj::set_native_updater_string_rep(value, b"17", false) };
    }
    static FIRST_HOST: obj::TclObjType = obj::TclObjType {
        name: c"originalScalarPreparationFirstHost".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(first_host_updater),
        set_from_any_proc: None,
    };
    #[test]
    fn entered_scalar_preparation_preserves_original_first_host_without_access_projection() {

        // A genuine extension updater reaches Host refusal; no external conversion result is asserted.
        let mut context = Context(
            Interp::with_native_core(
                crate::interp::default_host(),
                tcl_registry::model::ingress::resolve_environment("tcl8.4").unit_profile(),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap(),
        );
        context.0.set_result_bytes(b"PRIOR\0\xff");
        let prior = context.0.get_obj_result();
        let input = Owned::fresh(obj::alloc_typed(&FIRST_HOST, 0));
        let references = unsafe { (*input.ptr()).ref_count };
        CALLS.with(|calls| calls.set(0));
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut context.0);
        let operation = NumericOperation::enter(&mut context).unwrap();
        let error = operation.number(input.ptr()).unwrap_err();
        let first = context.0.native_execution_refusal().unwrap();
        assert!(
            matches!(&first, NativeExecutionError::HostCommandRefusal(first) if first.reason == "first scalar preparation updater refusal")
        );
        assert_eq!(error.native_execution_refusal, Some(first.clone()));
        assert_eq!(CALLS.with(Cell::get), 1);
        assert_eq!(obj::bytes_of(input.ptr()), b"17");
        assert!(obj::native_scalar_cache(input.ptr()).unwrap().is_none());
        assert_eq!(obj::obj_type_ptr(input.ptr()), &FIRST_HOST as *const _);
        assert_eq!(unsafe { (*input.ptr()).ref_count }, references);
        assert_eq!(context.0.get_obj_result(), prior);
        assert_eq!(context.0.result_bytes(), b"PRIOR\0\xff");
        let error = operation.number(input.ptr()).unwrap_err();
        assert_eq!(error.native_execution_refusal, Some(first));
        assert_eq!(CALLS.with(Cell::get), 1);
        crate::codegen_abi::tcl_runtime_set_current_interp(std::ptr::null_mut());
    }
    #[test]
    fn original_arithmetic_updater_preserves_first_refusal_before_second_operand_and_kernel() {

        // Genuine software updater/ABA; this does not measure a C arithmetic opcode or errno sequence.
        let mut context = native();
        context.0.set_result_bytes(b"PRIOR\0\xff");
        let prior = context.0.get_obj_result();
        let left = Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
        let right = Owned::fresh(obj::new_string_bytes(b"17"));
        let left_view = Owned::retain(left.ptr());
        let right_view = Owned::retain(right.ptr());
        let right_before = obj::native_object_snapshot(right.ptr()).unwrap();
        CALLS.with(|calls| calls.set(0));
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut context.0);
        let error = binary(&mut context, BinOp::Add, left, right)
            .err()
            .expect("original operation must refuse");
        let first = context.0.native_execution_refusal().unwrap();
        assert_eq!(error.native_execution_refusal, Some(first));
        assert_eq!(CALLS.with(Cell::get), 1);
        assert_eq!(obj::bytes_of(left_view.ptr()), b"17");
        assert_eq!(obj::obj_type_ptr(left_view.ptr()), &ORIGINAL as *const _);
        assert_eq!(
            obj::native_object_snapshot(right_view.ptr()).unwrap(),
            right_before
        );
        assert_eq!(unsafe { (*left_view.ptr()).ref_count }, 1);
        assert_eq!(unsafe { (*right_view.ptr()).ref_count }, 1);
        assert_eq!(context.0.get_obj_result(), prior);
        assert_eq!(context.0.result_bytes(), b"PRIOR\0\xff");
        assert_eq!(context.0.entered_native_operation_count(), 0);
        crate::codegen_abi::tcl_runtime_set_current_interp(std::ptr::null_mut());
    }
    #[test]
    fn original_unary_preparation_keeps_reached_host_cause_and_consumed_reference() {

        // A real updater supplies its own first Host cause, independently of Guest numeric rejection.
        let mut context = native();
        let input = Owned::fresh(obj::alloc_typed(&FIRST_HOST, 0));
        let view = Owned::retain(input.ptr());
        CALLS.with(|calls| calls.set(0));
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut context.0);
        let error = unary(&mut context, UnaryOp::Neg, input)
            .err()
            .expect("original operation must refuse");
        let first = context.0.native_execution_refusal().unwrap();
        assert!(
            matches!(&first, NativeExecutionError::HostCommandRefusal(first) if first.reason == "first scalar preparation updater refusal")
        );
        assert_eq!(error.native_execution_refusal, Some(first));
        assert_eq!(CALLS.with(Cell::get), 1);
        assert_eq!(obj::bytes_of(view.ptr()), b"17");
        assert_eq!(obj::obj_type_ptr(view.ptr()), &FIRST_HOST as *const _);
        assert!(obj::native_scalar_cache(view.ptr()).unwrap().is_none());
        assert_eq!(unsafe { (*view.ptr()).ref_count }, 1);
        assert_eq!(context.0.entered_native_operation_count(), 0);
        crate::codegen_abi::tcl_runtime_set_current_interp(std::ptr::null_mut());
    }
    #[test]
    fn selected_arithmetic_rejection_cannot_reparse_an_original_spelling_with_ambient_syntax() {

        // The deliberately different ambient parser is a software control, not an original C process observation.
        let mut context = Context(
            Interp::with_native_core(
                crate::interp::default_host(),
                tcl_registry::model::ingress::resolve_environment("tcl8.4").unit_profile(),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap(),
        );
        let original_syntax = tcl_syntax::number::runtime_syntax();
        tcl_syntax::number::set_runtime_syntax(tcl_dialect::NumberSyntax::Tcl90);
        let left = Owned::fresh(obj::new_string_bytes(b"0b10"));
        let right = Owned::fresh(obj::new_wide_int_obj(1));
        let view = Owned::retain(left.ptr());
        let error = binary(&mut context, BinOp::Add, left, right)
            .err()
            .expect("original operation must refuse");
        tcl_syntax::number::set_runtime_syntax(original_syntax);
        assert!(error.native_execution_refusal.is_none());
        assert!(error.native_access_refusal.is_none());
        assert!(error.msg.starts_with(b"can't use non-numeric string"));
        assert_eq!(obj::bytes_of(view.ptr()), b"0b10");
        assert!(obj::native_scalar_cache(view.ptr()).unwrap().is_none());
        assert_eq!(unsafe { (*view.ptr()).ref_count }, 1);
        assert_eq!(context.0.entered_native_operation_count(), 0);
        assert!(!context.0.host_refusal_pending());
    }
    #[test]
    fn original_c84_left_rejection_stops_before_right_updater_and_guest_replacement() {

        // Actual software sequencing control; the updater would refuse if reached.
        // The selected C84 left conversion must reject before right conversion.
        let mut context = Context(
            Interp::with_native_core(
                crate::interp::default_host(),
                tcl_registry::model::ingress::resolve_environment("tcl8.4").unit_profile(),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap(),
        );
        context.0.set_result_bytes(b"PRIOR\0\xff");
        let prior = context.0.get_obj_result();
        let left = Owned::fresh(obj::new_string_bytes(b"bad-left"));
        let right = Owned::fresh(obj::alloc_typed(&FIRST_HOST, 0));
        let left_view = Owned::retain(left.ptr());
        let right_view = Owned::retain(right.ptr());
        let before = obj::native_object_snapshot(right.ptr()).unwrap();
        CALLS.with(|calls| calls.set(0));
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut context.0);
        let error = binary(&mut context, BinOp::Add, left, right)
            .err()
            .expect("original operation must refuse");
        assert!(error.msg.starts_with(b"can't use non-numeric string"));
        assert!(error.native_execution_refusal.is_none());
        assert!(error.native_access_refusal.is_none());
        assert_eq!(CALLS.with(Cell::get), 0);
        assert_eq!(
            obj::native_object_snapshot(right_view.ptr()).unwrap(),
            before
        );
        assert_eq!(unsafe { (*left_view.ptr()).ref_count }, 1);
        assert_eq!(unsafe { (*right_view.ptr()).ref_count }, 1);
        assert_eq!(context.0.get_obj_result(), prior);
        assert_eq!(context.0.result_bytes(), b"PRIOR\0\xff");
        assert!(context.0.native_execution_refusal().is_none());
        assert_eq!(context.0.entered_native_operation_count(), 0);
        crate::codegen_abi::tcl_runtime_set_current_interp(std::ptr::null_mut());
    }
    #[test]
    fn original_arithmetic_and_unary_positive_paths_share_selected_current_operands() {

        // Bounded software acceptance/identity controls; no full release arithmetic equivalence is asserted.
        for version in ["tcl8.4", "tcl8.6"] {
            let mut context = Context(
                Interp::with_native_core(
                    crate::interp::default_host(),
                    tcl_registry::model::ingress::resolve_environment(version).unit_profile(),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap(),
            );
            let sum = binary(
                &mut context,
                BinOp::Add,
                Owned::fresh(obj::new_string_bytes(b"2")),
                Owned::fresh(obj::new_string_bytes(b"3")),
            )
            .unwrap();
            assert_eq!(obj::bytes_of(sum.ptr()), b"5");
            let negative = unary(
                &mut context,
                UnaryOp::Neg,
                Owned::fresh(obj::new_string_bytes(b"2.5")),
            )
            .unwrap();
            assert_eq!(obj::bytes_of(negative.ptr()), b"-2.5");
            let original = Owned::fresh(obj::new_double_obj(2.5));
            let pointer = original.ptr();
            let forwarded = unary(&mut context, UnaryOp::Pos, original).unwrap();
            assert_eq!(forwarded.ptr(), pointer);
            assert_eq!(obj::double_of(forwarded.ptr()), 2.5);
            assert_eq!(context.0.entered_native_operation_count(), 0);
            assert!(!context.0.host_refusal_pending());
        }
    }
    thread_local! { static INPUT_FREES: Cell<usize> = const { Cell::new(0) }; }
    extern "C" fn retired_input(_: *mut TclObj) {
        INPUT_FREES.with(|calls| calls.set(calls.get() + 1));
    }
    static INPUT_OWNER: obj::TclObjType = obj::TclObjType {
        name: c"originalArithmeticTransferredInput".as_ptr(),
        free_int_rep_proc: Some(retired_input),
        dup_int_rep_proc: None,
        update_string_proc: None,
        set_from_any_proc: None,
    };
    #[test]
    fn original_expr_add_facade_consumes_partial_null_and_first_host_inputs_before_return() {

        // Real public facade with transferred references; original pending cause remains independent of Guest arithmetic.
        for pending in [false, true] {
            for null_left in [false, true] {
                let mut context = native();
                context.0.set_result_bytes(b"PRIOR\0\xff");
                let prior = context.0.get_obj_result();
                if pending {
                    context
                        .0
                        .refuse_host_command("first arithmetic facade cause");
                }
                let first = context.0.native_execution_refusal();
                INPUT_FREES.with(|calls| calls.set(0));
                crate::codegen_abi::tcl_runtime_set_current_interp(&mut context.0);
                let input = Owned::fresh(obj::alloc_typed(&INPUT_OWNER, 0)).into_raw();
                let (left, right) = if null_left {
                    (std::ptr::null_mut(), input)
                } else {
                    (input, std::ptr::null_mut())
                };
                let result = unsafe { crate::codegen_abi::tcl_codegen_expr_add(left, right) };
                assert!(result.is_null());
                assert_eq!(INPUT_FREES.with(Cell::get), 1);
                assert_eq!(context.0.native_execution_refusal(), first);
                assert_eq!(context.0.get_obj_result(), prior);
                assert_eq!(context.0.result_bytes(), b"PRIOR\0\xff");
                assert_eq!(context.0.entered_native_operation_count(), 0);
                crate::codegen_abi::tcl_runtime_set_current_interp(std::ptr::null_mut());
            }
        }
        let mut context = native();
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut context.0);
        let left = Owned::fresh(obj::new_string_bytes(b"2"));
        let right = Owned::fresh(obj::new_string_bytes(b"3"));
        let result =
            unsafe { crate::codegen_abi::tcl_codegen_expr_add(left.into_raw(), right.into_raw()) };
        assert!(!result.is_null());
        let result = unsafe { Owned::from_raw(result) };
        assert_eq!(obj::bytes_of(result.ptr()), b"5");
        assert_eq!(unsafe { (*result.ptr()).ref_count }, 1);
        assert!(!context.0.host_refusal_pending());
        crate::codegen_abi::tcl_runtime_set_current_interp(std::ptr::null_mut());
    }
}
