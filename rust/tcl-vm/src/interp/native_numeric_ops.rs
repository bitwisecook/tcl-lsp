// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original numeric stages share the actual entered currency and Host ABI owner.

use super::{Vm, native_operation_currency::NativeOperationScope};
use crate::Value;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use tcl_cmd_core::CmdError;
use tcl_registry::InvocationDialect;
use tcl_syntax::{
    number::Number,
    scalar_getter::{
        NativeNumberGetterKind, NativeScalarGetterError, NativeScalarGetterFailure,
        NativeScalarGetterKind, NativeScalarGetterValue,
    },
    value::ValueError,
};

pub(crate) struct OriginalNumericOps<'a> {
    vm: RefCell<&'a mut Vm>,
    scope: NativeOperationScope,
    host: Rc<dyn tcl_platform::Host>,
    dialect: InvocationDialect,
    abi: Cell<Option<tcl_platform::NativeCIntegerAbi>>,
}

impl<'a> OriginalNumericOps<'a> {
    pub(crate) fn entered(vm: &'a mut Vm) -> Result<Self, CmdError> {
        let scope = NativeOperationScope::enter(vm)?;
        Self::in_original_scope(vm, &scope)
    }

    pub(crate) fn entered_value(vm: &'a mut Vm) -> Result<Self, ValueError> {
        Self::entered(vm).map_err(|_| Self::value_unwind())
    }

    // This is only an unwind token for a legacy ValueError seam. The full
    // original cause stays in the Vm and must be checked at its real boundary.
    pub(crate) fn value_unwind() -> ValueError {
        ValueError::CommandProtocolUnavailable("original VM numeric operation")
    }

    pub(crate) fn dialect(&self) -> InvocationDialect {
        self.dialect
    }

    #[cfg(test)]
    pub(crate) fn with_vm<T>(
        &self,
        operation: impl FnOnce(&mut Vm) -> Result<T, CmdError>,
    ) -> Result<T, CmdError> {
        self.current()?;
        let reached = operation(&mut self.vm.borrow_mut());
        self.current()?;
        reached
    }

    pub(crate) fn with_vm_for_value<T>(
        &self,
        value: &Value,
        operation: impl FnOnce(&mut Vm) -> Result<T, CmdError>,
    ) -> Result<T, CmdError> {
        self.current_for_value(value)?;
        let reached = operation(&mut self.vm.borrow_mut());
        self.current_for_value(value)?;
        reached
    }

    pub(crate) fn in_original_scope(
        vm: &'a mut Vm,
        scope: &NativeOperationScope,
    ) -> Result<Self, CmdError> {
        scope.check(vm)?;
        Ok(Self {
            host: vm.host_rc(),
            dialect: vm.actual_native_invocation_dialect(),
            vm: RefCell::new(vm),
            scope: scope.clone(),
            abi: Cell::new(None),
        })
    }

    pub(crate) fn current(&self) -> Result<(), CmdError> {
        self.scope.check(&mut self.vm.borrow_mut())
    }

    pub(crate) fn current_value(&self) -> Result<(), ValueError> {
        self.current().map_err(|_| Self::value_unwind())
    }

    fn current_for_value_native(
        &self,
        value: &Value,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let mut vm = self.vm.borrow_mut();
        self.scope.check_native(&mut vm)?;
        if let Err(refusal) = value.original_native_header_currentness() {
            return Err(
                vm.retain_tcl_host_failure(crate::error::TclHostFailure::ValueAccess(refusal))
            );
        }
        self.scope.check_native(&mut vm)
    }

    pub(crate) fn current_for_value(&self, value: &Value) -> Result<(), CmdError> {
        self.current_for_value_native(value)
            .map_err(CmdError::from_execution_refusal)
    }

    fn current_value_for_value(&self, value: &Value) -> Result<(), ValueError> {
        self.current_for_value_native(value)
            .map_err(|_| Self::value_unwind())
    }

    pub(crate) fn with_environment<T>(
        &self,
        operation: impl FnOnce(
            Option<&dyn tcl_platform::NumericEnvironment>,
            &mut dyn FnMut() -> Result<(), CmdError>,
        ) -> Result<T, CmdError>,
    ) -> Result<T, CmdError> {
        self.with_environment_using(operation, || self.current())
    }

    pub(crate) fn with_environment_for_value<T>(
        &self,
        value: &Value,
        operation: impl FnOnce(
            Option<&dyn tcl_platform::NumericEnvironment>,
            &mut dyn FnMut() -> Result<(), CmdError>,
        ) -> Result<T, CmdError>,
    ) -> Result<T, CmdError> {
        self.with_environment_using(operation, || self.current_for_value(value))
    }

    fn with_environment_value_for_value<T>(
        &self,
        value: &Value,
        operation: impl FnOnce(
            Option<&dyn tcl_platform::NumericEnvironment>,
            &mut dyn FnMut() -> Result<(), ValueError>,
        ) -> Result<T, ValueError>,
    ) -> Result<T, ValueError> {
        self.with_environment_using(operation, || self.current_value_for_value(value))
    }

    fn with_environment_using<T, E>(
        &self,
        operation: impl FnOnce(
            Option<&dyn tcl_platform::NumericEnvironment>,
            &mut dyn FnMut() -> Result<(), E>,
        ) -> Result<T, E>,
        current: impl Fn() -> Result<(), E>,
    ) -> Result<T, E> {
        current()?;
        let actual = self.host.numeric_environment();
        current()?;
        let mut stage = || current();
        let outcome = if let Some(actual) = actual {
            // The neutral numeric facade never carries or reconstructs a Host
            // cause. The original scope retains it and the outer check returns it.
            let check =
                || current().map_err(|_| tcl_platform::NumericEnvironmentUnavailable::Target);
            let checked = tcl_platform::CheckedNumericEnvironment::new(actual, &check, &self.abi);
            operation(Some(&checked), &mut stage)
        } else {
            operation(None, &mut stage)
        };
        current()?;
        outcome
    }

    fn bind_jim_using<E: From<ValueError>>(
        &self,
        value: &Value,
        current: impl Fn() -> Result<(), E>,
    ) -> Result<(), E> {
        current()?;
        if self.dialect.native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            let context = self.vm.borrow().native_jim_object_context();
            current()?;
            let reached = value.bind_native_jim_context(&context?);
            current()?;
            reached?;
        }
        current()
    }

    pub(crate) fn getter(
        &self,
        value: &Value,
        kind: NativeScalarGetterKind,
    ) -> Result<NativeScalarGetterValue, CmdError> {
        self.bind_jim_using(value, || self.current_for_value(value))?;
        self.with_environment_for_value(value, |environment, check| {
            value.native_scalar_getter_checked(self.dialect, kind, environment, check)
        })
    }

    pub(crate) fn getter_value(
        &self,
        value: &Value,
        kind: NativeScalarGetterKind,
    ) -> Result<NativeScalarGetterValue, ValueError> {
        self.bind_jim_using(value, || self.current_value_for_value(value))?;
        self.with_environment_value_for_value(value, |environment, check| {
            value.native_scalar_getter_checked(self.dialect, kind, environment, check)
        })
    }

    pub(crate) fn probe_value(
        &self,
        value: &Value,
        kind: NativeScalarGetterKind,
    ) -> Result<Result<NativeScalarGetterValue, NativeScalarGetterFailure>, ValueError> {
        self.bind_jim_using(value, || self.current_value_for_value(value))?;
        self.with_environment_value_for_value(value, |environment, check| {
            value.native_scalar_probe_checked(self.dialect, kind, environment, check)
        })
    }

    pub(crate) fn number_value(
        &self,
        value: &Value,
        kind: NativeNumberGetterKind,
    ) -> Result<Result<Number, NativeScalarGetterFailure>, ValueError> {
        self.current_value_for_value(value)?;
        let reached = value.native_number_probe_checked(self.dialect, kind, &mut || {
            self.current_value_for_value(value)
        });
        self.current_value_for_value(value)?;
        reached
    }

    pub(crate) fn probe(
        &self,
        value: &Value,
        kind: NativeScalarGetterKind,
    ) -> Result<Result<NativeScalarGetterValue, NativeScalarGetterFailure>, CmdError> {
        self.bind_jim_using(value, || self.current_for_value(value))?;
        self.with_environment_for_value(value, |environment, check| {
            value.native_scalar_probe_checked(self.dialect, kind, environment, check)
        })
    }

    pub(crate) fn number(
        &self,
        value: &Value,
        kind: NativeNumberGetterKind,
    ) -> Result<Result<Number, NativeScalarGetterFailure>, CmdError> {
        self.current_for_value(value)?;
        let reached = value
            .native_number_probe_checked(self.dialect, kind, &mut || self.current_for_value(value));
        self.current_for_value(value)?;
        reached
    }

    pub(crate) fn expression_integer84(
        &self,
        value: &Value,
    ) -> Result<Result<NativeScalarGetterValue, NativeScalarGetterFailure>, CmdError> {
        self.with_environment_for_value(value, |environment, check| {
            value.original_expression_integer84_checked(self.dialect, environment, check)
        })
    }

    pub(crate) fn word_boolean84(
        &self,
        value: &Value,
        boolean: bool,
    ) -> Result<NativeScalarGetterValue, CmdError> {
        self.current_for_value(value)?;
        let reached =
            value.original_expression_word_boolean84_checked(self.dialect, boolean, &mut || {
                self.current_for_value(value)
            });
        self.current_for_value(value)?;
        reached
    }

    pub(crate) fn failure(
        &self,
        value: &Value,
        kind: NativeScalarGetterKind,
        failure: NativeScalarGetterFailure,
    ) -> Result<NativeScalarGetterError, CmdError> {
        self.current_for_value(value)?;
        let reached = value.native_scalar_failure_presentation_checked(
            self.dialect,
            kind,
            failure,
            &mut || self.current_for_value(value),
        );
        self.current_for_value(value)?;
        reached
    }

    fn string_using<E: From<ValueError>>(
        &self,
        value: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        current: impl Fn(&Value) -> Result<(), E>,
    ) -> Result<Rc<[u8]>, E> {
        current(value)?;
        if self.dialect.native_string_protocol() != Some(protocol) {
            return Err(
                ValueError::CommandProtocolUnavailable("original VM String protocol").into(),
            );
        }
        self.bind_jim_using(value, || current(value))?;
        let formatter = self.host.native_integer_formatter();
        current(value)?;
        let reached =
            value.native_string_bytes_checked_for_value(protocol, formatter, &mut |original| {
                current(value)?;
                current(original)
            });
        current(value)?;
        reached
    }

    pub(crate) fn string(
        &self,
        value: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Rc<[u8]>, CmdError> {
        self.string_using(value, protocol, |original| self.current_for_value(original))
    }

    pub(crate) fn string_value(
        &self,
        value: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Rc<[u8]>, ValueError> {
        self.string_using(value, protocol, |original| {
            self.current_value_for_value(original)
        })
    }

    pub(crate) fn nonfinite84(
        &self,
        protocol: tcl_syntax::scalar_getter::NativeScalarGetterProtocol,
        value: f64,
    ) -> Result<Option<tcl_syntax::expr::errors::NativeFloatError>, CmdError> {
        self.with_environment(|environment, check| {
            check()?;
            let reached =
                tcl_cmd_core::native_numeric::c84_nonfinite_error(protocol, value, environment);
            check()?;
            reached.map_err(Into::into)
        })
    }

    pub(crate) fn c84_precision(&self, original: &[u8]) -> Result<Option<u8>, CmdError> {
        self.with_environment(|environment, check| {
            check()?;
            let actual = environment.ok_or(ValueError::ScalarNumericInputUnavailable)?;
            let reached = tcl_cmd_core::native_numeric::c84_precision_from_host(original, actual);
            check()?;
            reached.map_err(Into::into)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use tcl_platform::{
        DoubleNumericConversion, NativeCIntegerAbi, NumericEnvironment,
        NumericEnvironmentUnavailable, NumericErrorState, UnsignedNumericConversion,
    };
    use tcl_syntax::scalar_getter::{NativeScalarCache, NativeScalarGetterKind as Kind};

    type Call = (&'static str, Vec<u8>, usize, u32, bool);
    #[derive(Default)]
    struct ActualNumeric {
        calls: RefCell<Vec<Call>>,
        host_queries: Cell<usize>,
        changed_abi: Cell<bool>,
        original: RefCell<Option<crate::value::NativeObjectLifetimeLease>>,
        references: RefCell<Vec<(&'static str, usize)>>,
        retire_at: Cell<Option<&'static str>>,
        retire_owner: RefCell<Option<Value>>,
    }
    impl ActualNumeric {
        fn record(&self, name: &'static str, bytes: &[u8], offset: usize, base: u32, flag: bool) {
            if let Some(original) = self.original.borrow().as_ref() {
                self.references
                    .borrow_mut()
                    .push((name, original.value().native_object_reference_count()));
            }
            self.calls
                .borrow_mut()
                .push((name, bytes.to_vec(), offset, base, flag));
            if self.retire_at.get() == Some(name) {
                let retired = self.retire_owner.borrow_mut().take();
                drop(retired);
            }
        }
        fn state_value() -> NumericErrorState {
            NumericErrorState {
                errno: 37,
                domain_error: false,
                range_error: false,
            }
        }
        fn unsigned_value(input: &[u8]) -> UnsignedNumericConversion {
            let (value, end) = match input {
                b"6" => (6, 1),
                b"18" => (18, 2),
                _ => (0, 0),
            };
            UnsignedNumericConversion {
                value,
                end,
                before: Self::state_value(),
                after: Self::state_value(),
            }
        }
    }
    impl NumericEnvironment for ActualNumeric {
        fn c_integer_abi(&self) -> Result<NativeCIntegerAbi, NumericEnvironmentUnavailable> {
            self.record("abi", &[], 0, 0, false);
            let changed = self.changed_abi.get()
                && self
                    .calls
                    .borrow()
                    .iter()
                    .filter(|call| call.0 == "abi")
                    .count()
                    > 1;
            Ok(NativeCIntegerAbi {
                char_bits: 8,
                int_bytes: 4,
                long_bytes: if changed { 4 } else { 8 },
            })
        }
        fn state(&self) -> Result<NumericErrorState, NumericEnvironmentUnavailable> {
            self.record("state", &[], 0, 0, false);
            Ok(Self::state_value())
        }
        fn reset(&self) -> Result<(), NumericEnvironmentUnavailable> {
            self.record("reset", &[], 0, 0, false);
            Ok(())
        }
        fn unsigned(
            &self,
            input: &[u8],
            offset: usize,
            base: u32,
        ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
            self.record("unsigned", input, offset, base, false);
            Ok(Self::unsigned_value(input))
        }
        fn unsigned_long(
            &self,
            input: &[u8],
            offset: usize,
            base: u32,
        ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
            self.record("long", input, offset, base, false);
            Ok(Self::unsigned_value(input))
        }
        fn double(
            &self,
            input: &[u8],
            reset: bool,
        ) -> Result<DoubleNumericConversion, NumericEnvironmentUnavailable> {
            self.record("double", input, 0, 0, reset);
            Ok(DoubleNumericConversion {
                value: 1.25,
                end: 4,
                before: Self::state_value(),
                after: Self::state_value(),
            })
        }
    }
    struct NumericHost {
        actual: Rc<dyn tcl_platform::Host>,
        numeric: Rc<ActualNumeric>,
    }
    impl tcl_platform::Host for NumericHost {
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
        fn numeric_environment(&self) -> Option<&dyn NumericEnvironment> {
            self.numeric
                .host_queries
                .set(self.numeric.host_queries.get() + 1);
            Some(&*self.numeric)
        }
    }
    fn provided(version: &str) -> (Vm, Rc<ActualNumeric>) {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect(version));
        let numeric = Rc::new(ActualNumeric::default());
        vm.set_host(Rc::new(NumericHost {
            actual: vm.host_rc(),
            numeric: Rc::clone(&numeric),
        }));
        numeric.host_queries.set(0);
        numeric.calls.borrow_mut().clear();
        (vm, numeric)
    }

    #[test]
    fn original_numeric_facade_keeps_cached_header_string_and_reference_roles() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // The supplied Host is an authored software worker, not provider evidence.
        let (mut vm, numeric) = provided("tcl8.6");
        let dialect = vm.actual_native_invocation_dialect();
        let value = Value::from_native_scalar_cache(
            NativeScalarCache::Number(Number::Int(6)),
            None,
            dialect,
        )
        .unwrap();
        let alias = value.clone();
        let snapshot = value.native_object_snapshot();
        let references = value.native_object_reference_count();
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let ops = OriginalNumericOps::in_original_scope(&mut vm, &scope).unwrap();
        assert_eq!(
            ops.probe(&value, Kind::Int).unwrap().unwrap(),
            NativeScalarGetterValue::Wide(6)
        );
        assert!(numeric.host_queries.get() > 0);
        assert!(numeric.calls.borrow().iter().all(|call| call.0 == "abi"));
        assert!(alias.is_same_object(&value));
        assert_eq!(value.native_object_snapshot(), snapshot);
        assert_eq!(value.native_object_reference_count(), references);
        assert!(value.resident_string_bytes().is_none());
        drop(ops);
        scope.check(&mut vm).unwrap();
    }

    #[test]
    fn original_numeric_facade_refuses_withdrawn_currency_before_host_or_cache() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let (mut vm, numeric) = provided("tcl8.6");
        let value = Value::new_native_string_bytes(b"6");
        let before = value.native_object_snapshot();
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let original = vm.host_rc();
        vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
        vm.set_host(original);
        let result = OriginalNumericOps::in_original_scope(&mut vm, &scope)
            .err()
            .unwrap();
        let first = vm.execution_refusal.clone().unwrap();
        assert_eq!(result.native_execution_refusal(), Some(&first));
        assert_eq!(numeric.host_queries.get(), 0);
        assert!(numeric.calls.borrow().is_empty());
        assert_eq!(value.native_object_snapshot(), before);
    }

    #[test]
    fn original_numeric_reached_host_worker_retains_first_cause_before_cache() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // The software checker introduces refusal at the reached Host boundary;
        // no VM extension object callback issuer is asserted.
        let (mut vm, numeric) = provided("tcl8.4");
        let dialect = vm.actual_native_invocation_dialect();
        let value = Value::new_native_string_bytes(b"1.25");
        let alias = value.clone();
        let before = value.native_object_snapshot();
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let actual = RefCell::new(&mut vm);
        let current = || {
            if numeric.calls.borrow().iter().any(|call| call.0 == "double") {
                let _ = actual
                    .borrow_mut()
                    .refuse_host_command("reached original numeric Host worker".into());
            }
            scope.check(&mut actual.borrow_mut())
        };
        let check = || current().map_err(|_| NumericEnvironmentUnavailable::Target);
        let abi = Cell::new(None);
        let checked = tcl_platform::CheckedNumericEnvironment::new(&*numeric, &check, &abi);
        let result = value.native_scalar_probe_checked::<CmdError>(
            dialect,
            Kind::Double,
            Some(&checked),
            &mut || current(),
        );
        let first = actual.borrow().execution_refusal.clone().unwrap();
        assert_eq!(result.unwrap_err().native_execution_refusal(), Some(&first));
        assert_eq!(
            numeric
                .calls
                .borrow()
                .iter()
                .filter(|call| call.0 == "double")
                .count(),
            1
        );
        assert!(
            numeric
                .calls
                .borrow()
                .last()
                .is_some_and(|call| call.0 == "double")
        );
        assert!(alias.is_same_object(&value));
        assert_eq!(value.native_object_snapshot(), before);
        assert!(value.native_scalar_cache().is_none());
        assert_eq!(value.resident_string_bytes().unwrap().as_ref(), b"1.25");
    }

    #[test]
    fn original_numeric_primary_retirement_checker_prevents_later_cache_installation() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // This exercises the software checker after actual primary retirement.
        // VM Value has no extension free-callback issuer; no such callback is claimed.
        let (mut vm, _) = provided("tcl8.6");
        let dialect = vm.actual_native_invocation_dialect();
        let value = Value::native_list_constructor(
            vec![Value::new_native_string_bytes(b"6")],
            dialect.native_string_protocol().unwrap(),
        );
        let alias = value.clone();
        let references = value.native_object_reference_count();
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let mut retired = false;
        let result = value.native_number_probe_checked::<CmdError>(
            dialect,
            NativeNumberGetterKind::Number,
            &mut || {
                if value.cached_list_representation().is_none()
                    && value.native_scalar_cache().is_none()
                {
                    retired = true;
                    let _ = vm.refuse_host_command("retired original numeric primary".into());
                }
                scope.check(&mut vm)
            },
        );
        assert!(retired);
        let first = vm.execution_refusal.clone().unwrap();
        assert_eq!(result.unwrap_err().native_execution_refusal(), Some(&first));
        assert!(alias.is_same_object(&value));
        assert!(value.native_scalar_cache().is_none());
        assert!(value.cached_list_representation().is_none());
        assert_eq!(value.resident_string_bytes().unwrap().as_ref(), b"6");
        assert_eq!(value.native_object_reference_count(), references);
    }

    #[test]
    fn original_precision_safe_and_cached_modern_reads_preserve_getter_order() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Software integration uses the selected source policy; no native run is implied.
        for safe in [false, true] {
            let (mut vm, numeric) = provided("tcl8.6");
            let dialect = vm.actual_native_invocation_dialect();
            let cell = vm.trace_cell_bytes(b"tcl_precision").unwrap();
            let id = cell.id.unwrap();
            let value = Value::from_native_scalar_cache(
                NativeScalarCache::Number(Number::Int(6)),
                None,
                dialect,
            )
            .unwrap();
            let retired = vm
                .var_arena
                .replace_state(id, super::super::Local::Scalar(value.clone()));
            drop(retired);
            let context = crate::value::DoubleFormatContext::for_dialect(dialect).unwrap();
            let before = context.precision();
            vm.is_safe = safe;
            let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
            let result = vm
                .fire_native_precision_trace_bytes(b"tcl_precision", "write", &cell, &scope)
                .unwrap();
            assert_eq!(result.is_err(), safe);
            assert_eq!(context.precision(), if safe { before } else { 6 });
            assert_eq!(numeric.host_queries.get() == 0, safe);
            assert!(value.resident_string_bytes().is_none());
            assert!(numeric.calls.borrow().iter().all(|call| call.0 == "abi"));
        }
    }

    #[test]
    fn original_c84_precision_null_read_reaches_decimal_worker_without_reset() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let (mut vm, numeric) = provided("tcl8.4");
        let cell = vm.trace_cell_bytes(b"tcl_precision").unwrap();
        let retired = vm
            .var_arena
            .replace_state(cell.id.unwrap(), super::super::Local::Undefined);
        drop(retired);
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let result = vm
            .fire_native_precision_trace_bytes(b"tcl_precision", "write", &cell, &scope)
            .unwrap();
        assert!(result.is_err());
        let calls = numeric.calls.borrow();
        assert!(
            calls
                .iter()
                .any(|call| call == &("long", Vec::new(), 0, 10, false))
        );
        assert!(calls.iter().all(|call| call.0 != "reset"));
        assert!(vm.read_resolved_cell(cell.id.unwrap()).is_some());
    }

    #[test]
    fn original_checked_jim_context_keeps_its_host_and_stops_the_next_worker() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // These are software Host stages with genuine retained VM Jim context,
        // not an extension-object updater or external Jim execution claim.
        for withdraw in [false, true] {
            let (mut vm, numeric) = provided("jim");
            let dialect = vm.actual_native_invocation_dialect();
            let context = vm.native_jim_object_context().unwrap();
            let value = Value::new_native_string_bytes(if withdraw {
                b"1.25".as_slice()
            } else {
                b"6".as_slice()
            });
            value.bind_native_jim_context(&context).unwrap();
            let before = value.native_object_snapshot();
            let references = value.native_object_reference_count();
            numeric.calls.borrow_mut().clear();
            numeric.host_queries.set(0);
            let scope = NativeOperationScope::enter(&mut vm).unwrap();
            let result = value.native_scalar_probe_checked::<CmdError>(
                dialect,
                if withdraw { Kind::Double } else { Kind::Wide },
                None,
                &mut || {
                    if withdraw
                        && numeric
                            .calls
                            .borrow()
                            .iter()
                            .any(|call| call.0 == "unsigned")
                    {
                        let _ =
                            vm.refuse_host_command("reached original Jim unsigned worker".into());
                    }
                    scope.check(&mut vm)
                },
            );
            assert!(numeric.host_queries.get() > 0);
            assert_eq!(
                numeric
                    .calls
                    .borrow()
                    .iter()
                    .filter(|call| call.0 == "unsigned")
                    .count(),
                1
            );
            assert!(
                numeric
                    .calls
                    .borrow()
                    .iter()
                    .all(|call| call.0 != "double" && call.0 != "reset")
            );
            assert_eq!(value.native_object_reference_count(), references);
            if withdraw {
                let first = vm.execution_refusal.clone().unwrap();
                assert_eq!(result.unwrap_err().native_execution_refusal(), Some(&first));
                assert_eq!(value.native_object_snapshot(), before);
                assert!(value.native_scalar_cache().is_none());
                assert_eq!(value.resident_string_bytes().unwrap().as_ref(), b"1.25");
                assert_eq!(numeric.calls.borrow().last().unwrap().0, "unsigned");
            } else {
                assert_eq!(result.unwrap().unwrap(), NativeScalarGetterValue::Wide(6));
                assert_eq!(value.integer_representation(), Some(6));
                assert_eq!(value.resident_string_bytes().unwrap().as_ref(), b"6");
                scope.check(&mut vm).unwrap();
            }
        }
    }

    #[test]
    fn original_precision_getters_borrow_the_cell_without_a_native_reference() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // The actual software Host observes the original cell header during
        // the selected worker; the observer itself holds an allocation-only lease.
        for version in ["tcl8.4", "tcl8.6"] {
            let (mut vm, numeric) = provided(version);
            let dialect = vm.actual_native_invocation_dialect();
            let value = if version == "tcl8.4" {
                Value::new_native_string_bytes(b"6")
            } else {
                Value::from_native_scalar_cache(
                    NativeScalarCache::Number(Number::Int(6)),
                    None,
                    dialect,
                )
                .unwrap()
            };
            let cell = vm.trace_cell_bytes(b"tcl_precision").unwrap();
            let retired = vm
                .var_arena
                .replace_state(cell.id.unwrap(), super::super::Local::Scalar(value.clone()));
            drop(retired);
            let references = value.native_object_reference_count();
            *numeric.original.borrow_mut() = Some(value.native_lifetime_lease());
            assert_eq!(value.native_object_reference_count(), references);
            let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
            assert!(
                vm.fire_native_precision_trace_bytes(b"tcl_precision", "write", &cell, &scope)
                    .unwrap()
                    .is_ok()
            );
            let observed = numeric.references.borrow();
            assert!(!observed.is_empty());
            assert!(observed.iter().all(|(_, count)| *count == references));
            if version == "tcl8.4" {
                assert!(observed.iter().any(|(name, _)| *name == "long"));
            } else {
                assert!(observed.iter().all(|(name, _)| *name == "abi"));
                assert!(value.resident_string_bytes().is_none());
            }
            assert_eq!(value.native_object_reference_count(), references);
            assert!(
                vm.initial_value_at_cell(cell.id.unwrap(), cell.array)
                    .unwrap()
                    .unwrap()
                    .is_same_object(&value)
            );
            scope.check_native(&mut vm).unwrap();
        }
    }

    #[test]
    fn original_checked_jim_probe_refuses_changed_abi_between_context_stages() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // A supplied outer worker is not the retained Jim context's numeric Host.
        let (mut vm, numeric) = provided("jim");
        let dialect = vm.actual_native_invocation_dialect();
        let context = vm.native_jim_object_context().unwrap();
        let value = Value::new_native_string_bytes(b"6");
        value.bind_native_jim_context(&context).unwrap();
        let before = value.native_object_snapshot();
        numeric.calls.borrow_mut().clear();
        numeric.changed_abi.set(true);
        let foreign = ActualNumeric::default();
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let reached = value.native_scalar_probe_checked::<CmdError>(
            dialect,
            Kind::Long,
            Some(&foreign),
            &mut || scope.check(&mut vm),
        );
        assert!(reached.unwrap_err().native_access_refusal().is_some());
        assert_eq!(
            numeric
                .calls
                .borrow()
                .iter()
                .filter(|call| call.0 == "abi")
                .count(),
            2
        );
        assert!(numeric.calls.borrow().iter().all(|call| call.0 == "abi"));
        assert!(foreign.calls.borrow().is_empty());
        assert_eq!(value.native_object_snapshot(), before);
        assert!(value.native_scalar_cache().is_none());
        assert!(vm.execution_refusal.is_none());
        scope.check(&mut vm).unwrap();
    }

    #[test]
    fn original_value_ops_scalar_consumers_keep_cached_headers_and_jim_context() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        use tcl_syntax::value::ValueOps;
        let (mut vm, numeric) = provided("tcl8.6");
        let dialect = vm.actual_native_invocation_dialect();
        let value = Value::from_native_scalar_cache(
            NativeScalarCache::Number(Number::Int(6)),
            None,
            dialect,
        )
        .unwrap();
        let references = value.native_object_reference_count();
        assert_eq!(vm.as_int(&value).unwrap(), 6);
        assert_eq!(vm.as_double(&value).unwrap().to_bits(), 6.0_f64.to_bits());
        assert!(vm.as_bool(&value).unwrap());
        assert_eq!(value.native_object_reference_count(), references);
        assert!(value.resident_string_bytes().is_none());
        assert!(numeric.calls.borrow().iter().all(|call| call.0 == "abi"));

        let (mut jim, numeric) = provided("jim");
        let value = Value::new_native_string_bytes(b"6");
        assert_eq!(jim.as_int(&value).unwrap(), 6);
        assert!(
            numeric
                .calls
                .borrow()
                .iter()
                .any(|call| call.0 == "unsigned")
        );
        assert_eq!(value.integer_representation(), Some(6));
        assert!(jim.native_jim_object_context().unwrap().is_live());
    }

    #[test]
    fn original_value_error_numeric_seam_returns_the_retained_host_cause_at_completion() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        use tcl_syntax::value::ValueOps;
        let (mut vm, numeric) = provided("tcl8.6");
        let value = Value::new_native_string_bytes(b"6");
        let before = value.native_object_snapshot();
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let original = vm.host_rc();
        vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
        vm.set_host(original);
        let error = vm.as_int(&value).unwrap_err();
        let first = vm.execution_refusal.clone().unwrap();
        assert!(error.native_access_refusal().is_some());
        assert_eq!(numeric.host_queries.get(), 0);
        assert!(numeric.calls.borrow().is_empty());
        assert_eq!(value.native_object_snapshot(), before);
        let completion = crate::command::completion_from_cmd_error(&mut vm, error.into());
        assert_eq!(completion.code, tcl_runtime_api::Code::Error);
        assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
        assert_eq!(vm.try_invoke_command("set", &[]).unwrap_err(), first);
        assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
        assert_eq!(
            scope.check(&mut vm).unwrap_err().native_execution_refusal(),
            Some(&first)
        );
    }

    #[test]
    fn original_list_index_and_info_numeric_consumers_keep_their_selected_purposes() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        use tcl_cmd_core::{
            native_info_level::NativeInfoLevelObjects, native_list_index::NativeListIndexOps,
        };
        for version in ["tcl8.6", "tcl9.0"] {
            let (mut vm, _) = provided(version);
            let dialect = vm.actual_native_invocation_dialect();
            let value = Value::from_native_scalar_cache(
                NativeScalarCache::Number(Number::Int(6)),
                None,
                dialect,
            )
            .unwrap();
            let references = value.native_object_reference_count();
            assert_eq!(vm.index_number(&value).unwrap(), Some(Number::Int(6)));
            assert_eq!(vm.native_level_integer(&value, Kind::Int).unwrap(), 6);
            assert_eq!(value.native_object_reference_count(), references);
            assert!(value.resident_string_bytes().is_none());
            let scope = NativeOperationScope::enter(&mut vm).unwrap();
            let original = vm.host_rc();
            vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
            vm.set_host(original);
            let first_index = vm.index_number(&value).unwrap_err();
            let first = vm.execution_refusal.clone().unwrap();
            assert!(first_index.native_access_refusal().is_some());
            assert!(
                vm.native_level_integer(&value, Kind::Int)
                    .unwrap_err()
                    .native_access_refusal()
                    .is_some()
            );
            assert_eq!(
                scope.check(&mut vm).unwrap_err().native_execution_refusal(),
                Some(&first)
            );
            assert!(value.resident_string_bytes().is_none());
        }
    }

    #[test]
    fn original_numeric_value_seam_keeps_genuine_guest_scalar_receipts() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        use tcl_syntax::value::ValueOps;
        let (mut vm, _) = provided("tcl8.6");
        let original = Value::new_native_string_bytes(b"not_an_integer");
        let before = original.native_object_snapshot();
        let error = vm.as_int(&original).unwrap_err();
        assert!(matches!(error, ValueError::NativeScalarGetter(_)));
        assert!(error.native_access_refusal().is_none());
        assert!(vm.execution_refusal.is_none());
        assert_eq!(original.native_object_snapshot(), before);
        let guest = crate::command::completion_from_cmd_error(&mut vm, error.into());
        assert_eq!(guest.code, tcl_runtime_api::Code::Error);
        assert!(vm.execution_refusal.is_none());
    }

    #[test]
    fn original_authored_numeric_selection_does_not_query_the_native_host() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        use tcl_syntax::value::ValueOps;
        let (mut vm, numeric) = provided("tcl9.0");
        vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        assert!(vm.set_logical_numeric_provider(
            tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core
        ));
        numeric.calls.borrow_mut().clear();
        numeric.host_queries.set(0);
        let context = vm.authored_numeric_context().unwrap();
        assert!(context.environment.is_none());
        assert_eq!(context.dialect, vm.native_invocation_dialect());
        assert!(vm.numeric_policy_context().environment.is_none());
        let original = Value::new_native_string_bytes(b"077");
        assert_eq!(vm.as_int(&original).unwrap(), 63);
        assert!(original.native_scalar_cache().is_none());
        assert_eq!(
            vm.as_double(&Value::new_native_string_bytes(b"1.25"))
                .unwrap()
                .to_bits(),
            1.25_f64.to_bits()
        );
        assert!(vm.as_bool(&Value::new_native_string_bytes(b"2")).unwrap());
        assert!(
            crate::expr::boolean_for_vm(
                &mut vm,
                &Value::new_native_string_bytes(b"2"),
                tcl_registry::native_boolean_truth::NativeBooleanTruthPurpose::LogicalNot
            )
            .unwrap()
        );
        assert_eq!(numeric.host_queries.get(), 0);
        assert!(numeric.calls.borrow().is_empty());
        assert!(vm.execution_refusal.is_none());
    }
    #[test]
    fn original_authored_expression_increment_and_boolean_policies_do_not_query_host() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // These assertions reach the actual routing adapters, not just the selector.
        let (mut vm, numeric) = provided("tcl9.0");
        vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        assert!(vm.set_logical_expression_parse_provider(
            tcl_registry::invocation_words::LogicalExpressionParseProvider::Tcl84CoreSimulation
        ));
        assert!(vm.set_logical_numeric_provider(
            tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core
        ));
        numeric.calls.borrow_mut().clear();
        numeric.host_queries.set(0);
        let policy = vm.expression_evaluation_policy().unwrap();
        assert_eq!(
            policy.origin,
            tcl_runtime_api::expression_policy::ExpressionEvaluationOrigin::AuthoredTcl84Parser
        );
        assert!(policy.numeric_simulation.is_some());
        vm.prepare_increment_amount(&Value::new_native_string_bytes(b"6"))
            .unwrap();
        let value = crate::expr::logical_value_for_vm(
            &mut vm,
            Value::new_native_string_bytes(b"2"),
            &Value::new_native_string_bytes(b"3"),
            true,
        )
        .unwrap();
        assert_eq!(value.string_bytes().as_ref(), b"1");
        assert_eq!(numeric.host_queries.get(), 0);
        assert!(numeric.calls.borrow().is_empty());
        assert!(vm.execution_refusal.is_none());
    }

    #[test]
    fn original_value_environment_keeps_the_same_header_and_shared_numeric_stages() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Actual VM header and software Host stages; no Runtime extension issuer is claimed.
        let (mut vm, numeric) = provided("tcl8.4");
        let original = Value::new_native_string_bytes(b"6");
        let held = original.native_lifetime_lease();
        *numeric.original.borrow_mut() = Some(original.native_lifetime_lease());
        let count = original.native_object_reference_count();
        let before = original.native_object_snapshot();
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let reached = OriginalNumericOps::in_original_scope(&mut vm, &scope)
            .unwrap()
            .with_environment_for_value(held.value(), |environment, check| {
                let actual = environment.ok_or(ValueError::ScalarNumericInputUnavailable)?;
                let abi = actual
                    .c_integer_abi()
                    .map_err(|_| ValueError::ScalarNumericInputUnavailable);
                check()?;
                assert_eq!(abi?.long_bytes, 8);
                let state = actual
                    .state()
                    .map_err(|_| ValueError::ScalarNumericInputUnavailable);
                check()?;
                assert_eq!(state?.errno, 37);
                let converted = actual
                    .unsigned(b"6", 0, 10)
                    .map_err(|_| ValueError::ScalarNumericInputUnavailable);
                check()?;
                Ok(converted?.value)
            })
            .unwrap();
        assert_eq!(reached, 6);
        assert_eq!(numeric.host_queries.get(), 1);
        assert_eq!(
            numeric
                .calls
                .borrow()
                .iter()
                .map(|call| call.0)
                .collect::<Vec<_>>(),
            ["abi", "state", "unsigned"]
        );
        assert!(
            numeric
                .references
                .borrow()
                .iter()
                .all(|entry| entry.1 == count)
        );
        assert_eq!(original.native_object_reference_count(), count);
        assert_eq!(original.native_object_snapshot(), before);
        assert!(vm.execution_refusal.is_none());
        scope.check_native(&mut vm).unwrap();
    }

    #[test]
    fn original_value_environment_retains_header_refusal_before_the_next_host_worker() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // A software callback releases the actual VM owner; the lease retains allocation only.
        for stage in ["abi", "state", "unsigned"] {
            let (mut vm, numeric) = provided("tcl8.4");
            let original = Value::new_native_string_bytes(b"6");
            let held = original.native_lifetime_lease();
            *numeric.original.borrow_mut() = Some(original.native_lifetime_lease());
            assert_eq!(original.native_object_reference_count(), 1);
            numeric.retire_at.set(Some(stage));
            *numeric.retire_owner.borrow_mut() = Some(original);
            let scope = NativeOperationScope::enter(&mut vm).unwrap();
            let reached = OriginalNumericOps::in_original_scope(&mut vm, &scope)
                .unwrap()
                .with_environment_for_value(held.value(), |environment, check| {
                    let actual = environment.ok_or(ValueError::ScalarNumericInputUnavailable)?;
                    let abi = actual
                        .c_integer_abi()
                        .map_err(|_| ValueError::ScalarNumericInputUnavailable);
                    check()?;
                    abi?;
                    let state = actual
                        .state()
                        .map_err(|_| ValueError::ScalarNumericInputUnavailable);
                    check()?;
                    state?;
                    let result = actual
                        .unsigned(b"6", 0, 10)
                        .map_err(|_| ValueError::ScalarNumericInputUnavailable);
                    check()?;
                    result?;
                    Ok(())
                });
            let first = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "retired native object header",
                ),
            );
            assert_eq!(
                reached.unwrap_err().native_execution_refusal(),
                Some(&first)
            );
            assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
            let expected = match stage {
                "abi" => &["abi"][..],
                "state" => &["abi", "state"][..],
                _ => &["abi", "state", "unsigned"][..],
            };
            assert_eq!(
                numeric
                    .calls
                    .borrow()
                    .iter()
                    .map(|call| call.0)
                    .collect::<Vec<_>>(),
                expected
            );
            assert!(numeric.references.borrow().iter().all(|entry| entry.1 == 1));
            assert!(!held.value().native_object_is_live());
            assert!(held.value().native_scalar_cache().is_none());
            assert_eq!(vm.try_invoke_command("set", &[]).unwrap_err(), first);
            assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
        }
    }

    #[test]
    fn original_scalar_facade_preserves_retired_header_cause_through_quiet_value_error() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Genuine reached numeric callbacks retire the actual header before any returned cache publication.
        for stage in ["abi", "double"] {
            for legacy in [false, true] {
                let (mut vm, numeric) = provided("tcl8.4");
                let original = Value::new_native_string_bytes(b"1.25");
                let held = original.native_lifetime_lease();
                *numeric.original.borrow_mut() = Some(original.native_lifetime_lease());
                numeric.retire_at.set(Some(stage));
                *numeric.retire_owner.borrow_mut() = Some(original);
                let scope = NativeOperationScope::enter(&mut vm).unwrap();
                let ops = OriginalNumericOps::in_original_scope(&mut vm, &scope).unwrap();
                let first = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "retired native object header",
                    ),
                );
                if legacy {
                    assert_eq!(
                        ops.probe_value(held.value(), Kind::Double).unwrap_err(),
                        OriginalNumericOps::value_unwind()
                    );
                } else {
                    assert_eq!(
                        ops.getter(held.value(), Kind::Double)
                            .unwrap_err()
                            .native_execution_refusal(),
                        Some(&first)
                    );
                }
                drop(ops);
                assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
                assert_eq!(numeric.calls.borrow().last().unwrap().0, stage);
                assert!(numeric.references.borrow().iter().all(|entry| entry.1 == 1));
                assert!(!held.value().native_object_is_live());
                assert!(held.value().native_scalar_cache().is_none());
                assert_eq!(vm.try_invoke_command("set", &[]).unwrap_err(), first);
                assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
            }
        }
    }

    #[test]
    fn original_value_header_check_preserves_an_earlier_entered_scope_cause() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let (mut vm, numeric) = provided("tcl8.6");
        let original = Value::new_native_string_bytes(b"6");
        let held = original.native_lifetime_lease();
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let ops = OriginalNumericOps::in_original_scope(&mut vm, &scope).unwrap();
        let _ = ops
            .vm
            .borrow_mut()
            .refuse_host_command("earlier actual operation refusal".into());
        let first = ops.vm.borrow().execution_refusal.clone().unwrap();
        drop(original);
        assert!(!held.value().native_object_is_live());
        assert_eq!(
            ops.current_for_value(held.value())
                .unwrap_err()
                .native_execution_refusal(),
            Some(&first)
        );
        drop(ops);
        assert_eq!(numeric.host_queries.get(), 0);
        assert!(numeric.calls.borrow().is_empty());
        assert_eq!(vm.try_invoke_command("set", &[]).unwrap_err(), first);
        assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    }
}

#[cfg(test)]
#[path = "native_string_stage_tests.rs"]
mod original_string_stage_tests;
