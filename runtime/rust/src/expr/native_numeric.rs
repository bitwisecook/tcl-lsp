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
use tcl_syntax::number::Number;

pub(super) struct NumericOperation {
    scope: Option<NativeOperationScope>,
    dialect: tcl_registry::InvocationDialect,
    host: Option<Rc<dyn tcl_platform::Host>>,
    abi: Cell<Option<tcl_platform::NativeCIntegerAbi>>,
}
impl NumericOperation {
    pub(super) fn enter(ctx: &mut dyn ExprCtx) -> Result<Self, ExprError> {
        let scope = ctx
            .boolean_interpreter()
            .map(|interp| NativeOperationScope::enter(interp))
            .transpose()
            .map_err(ExprError::from_execution_refusal)?;
        let operation = Self {
            scope,
            dialect: ctx.invocation_dialect(),
            host: ctx.numeric_host(),
            abi: Cell::new(None),
        };
        operation.check()?;
        operation.dialect.arithmetic().ok_or_else(|| {
            ExprError::host_refusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::ScalarNumericInputUnavailable,
            )
        })?;
        Ok(operation)
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
        self.check()?;
        let recipe = self.dialect.native_string_materialization(Some(tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation)).ok_or_else(|| ExprError::host_refusal(tcl_syntax::raw_string::NativeValueAccessRefusal::ScalarNumericInputUnavailable))?;
        let bytes = crate::dict::native_object_bytes(value, recipe.protocol());
        self.check()?;
        bytes.map_err(|error| ExprError::from_cmd_error(error.into()))
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
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
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
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
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
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
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
}
