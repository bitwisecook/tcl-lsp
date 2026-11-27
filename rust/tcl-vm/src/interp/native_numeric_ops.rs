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

//! Original numeric stages share the actual entered currency and Host ABI owner.

use super::{
    Vm,
    native_operation_currency::{NativeOperationCurrency, NativeOperationScope},
};
use crate::Value;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use tcl_cmd_core::CmdError;
use tcl_native_model::InvocationDialect;
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
    objects: Option<tcl_syntax::native_string::NativeStringProtocol>,
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

    /// The list and dictionary recipe of the object purpose entered with this
    /// operation, beside its physical numeric dialect.
    pub(crate) fn object_protocol(
        &self,
    ) -> Option<tcl_syntax::native_string::NativeStringProtocol> {
        self.objects
    }

    #[cfg(all(test, debug_assertions))]
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
        Self::in_original_scope_native(vm, scope).map_err(CmdError::from_execution_refusal)
    }

    pub(crate) fn in_original_scope_native(
        vm: &'a mut Vm,
        scope: &NativeOperationScope,
    ) -> Result<Self, tcl_runtime_api::NativeExecutionError> {
        scope.check_native(vm)?;
        Ok(Self {
            host: vm.host_rc(),
            dialect: vm.actual_native_invocation_dialect(),
            objects: vm.object_protocol(),
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
        if !crate::verify_currency() {
            return Ok(());
        }
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

    pub(crate) fn current_value_for_value(&self, value: &Value) -> Result<(), ValueError> {
        self.current_for_value_native(value)
            .map_err(|_| Self::value_unwind())
    }

    pub(crate) fn cache_value_number(
        &self,
        value: &Value,
        number: Number,
    ) -> Result<(), ValueError> {
        let protocol = self
            .dialect
            .native_string_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        value.cache_numeric_representation_checked(
            number,
            &mut || self.current_value_for_value(value),
            &mut || self.string_value(value, protocol),
        )
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

    fn list_using<E: From<ValueError>>(
        &self,
        value: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        current: impl Fn(&Value) -> Result<(), E>,
    ) -> Result<crate::NativeListItems, E> {
        current(value)?;
        if self.objects != Some(protocol) {
            return Err(ValueError::CommandProtocolUnavailable("original VM List protocol").into());
        }
        self.bind_jim_using(value, || current(value))?;
        let reached = value.native_object_list_elements_with_strings(
            protocol,
            &mut || current(value),
            &mut |actual| self.string_using(actual, protocol, &current),
        );
        current(value)?;
        reached
    }

    pub(crate) fn list(
        &self,
        value: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<crate::NativeListItems, CmdError> {
        self.list_using(value, protocol, |actual| self.current_for_value(actual))
    }

    pub(crate) fn list_with_original_check(
        &self,
        value: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        additional: impl Fn() -> Result<(), CmdError>,
    ) -> Result<crate::NativeListItems, CmdError> {
        self.list_using(value, protocol, |actual| {
            self.current_for_value(actual)?;
            additional()
        })
    }

    /// Keep the original consumer's caller checks through nested List getters.
    pub(crate) fn list_with_original_caller_check(
        &self,
        value: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        additional: impl Fn(&mut Vm) -> Result<(), CmdError>,
    ) -> Result<crate::NativeListItems, CmdError> {
        self.list_using(value, protocol, |actual| {
            self.current_for_value(actual)?;
            let reached = additional(&mut self.vm.borrow_mut());
            if let Err(error) = &reached {
                self.retain_failure(error);
            }
            reached?;
            self.current_for_value(actual)
        })
    }

    pub(crate) fn list_for_expression(
        &self,
        value: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<crate::NativeListItems, crate::error::TclError> {
        self.list_using(value, protocol, |actual| {
            self.arithmetic_current_for_value(actual)
        })
    }

    /// Retain the first original cause before mandatory result/backing cleanup.
    /// This has no currentness precondition: a refused original still owns its
    /// first cause, and cleanup must not replace it with a later refusal.
    pub(crate) fn retain_failure(&self, error: &CmdError) {
        if let Some(first) = error.native_execution_refusal() {
            self.vm
                .borrow_mut()
                .retain_tcl_host_failure(crate::error::TclHostFailure::Execution(first.clone()));
        }
    }

    fn retain_compound_failure<T>(&self, reached: &Result<T, CmdError>) {
        if let Err(error) = reached {
            self.retain_failure(error);
        }
    }

    fn compound_with_currency<T>(
        &self,
        original: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        currency: &NativeOperationCurrency,
        operation: impl FnOnce(
            &mut dyn FnMut() -> Result<(), CmdError>,
            &mut dyn FnMut(&Value) -> Result<Rc<[u8]>, CmdError>,
        ) -> Result<T, CmdError>,
    ) -> Result<T, CmdError> {
        let current = |actual: &Value| {
            self.current_for_value(original)?;
            self.current_for_value(actual)?;
            currency.check(&mut self.vm.borrow_mut())
        };
        current(original)?;
        if self.objects != Some(protocol) {
            return Err(
                ValueError::CommandProtocolUnavailable("original VM compound protocol").into(),
            );
        }
        self.bind_jim_using(original, || current(original))?;
        let reached = operation(&mut || current(original), &mut |actual| {
            let reached = self.string_using(actual, protocol, current);
            self.retain_compound_failure(&reached);
            current(actual)?;
            reached
        });
        self.retain_compound_failure(&reached);
        if let Err(first) = current(original) {
            drop(reached);
            current(original)?;
            return Err(first);
        }
        reached
    }

    pub(crate) fn copy_list_with_currency(
        &self,
        original: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        currency: &NativeOperationCurrency,
    ) -> Result<Value, CmdError> {
        self.compound_with_currency(original, protocol, currency, |check, strings| {
            original.native_list_copy_checked(protocol, check, strings)
        })
    }

    pub(crate) fn dictionary_with_currency(
        &self,
        original: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        currency: &NativeOperationCurrency,
    ) -> Result<Vec<(Value, Value)>, CmdError> {
        self.compound_with_currency(original, protocol, currency, |check, strings| {
            original.native_object_dict_pairs_with_strings(protocol, check, strings)
        })
    }

    pub(crate) fn prepare_dictionary_with_currency(
        &self,
        original: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        currency: &NativeOperationCurrency,
    ) -> Result<crate::value::PreparedNativeDictionary, CmdError> {
        self.compound_with_currency(original, protocol, currency, |check, strings| {
            original.prepare_native_dictionary_after_conversion_checked(protocol, check, strings)
        })
    }

    pub(crate) fn set_dictionary_member_with_currency(
        &self,
        original: &mut crate::value::PreparedNativeDictionary,
        key: Value,
        value: Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        currency: &NativeOperationCurrency,
    ) -> Result<(), CmdError> {
        self.current_for_value(original.original())?;
        if original.string_protocol() != protocol {
            return Err(ValueError::CommandProtocolUnavailable(
                "original prepared Dictionary protocol",
            )
            .into());
        }
        let root = original.original().native_lifetime_lease();
        let reached =
            self.compound_with_currency(root.value(), protocol, currency, |check, strings| {
                original.set_member_checked(key, value, check, strings)
            });
        drop(root);
        self.current()?;
        reached
    }

    pub(crate) fn getter(
        &self,
        value: &Value,
        kind: NativeScalarGetterKind,
    ) -> Result<NativeScalarGetterValue, CmdError> {
        self.bind_jim_using(value, || self.current_for_value(value))?;
        self.with_environment_for_value(value, |environment, check| {
            value.native_scalar_getter_checked_with_string(
                self.dialect,
                kind,
                environment,
                check,
                &mut || {
                    self.string(
                        value,
                        self.dialect
                            .native_string_protocol()
                            .ok_or(ValueError::ScalarNumericInputUnavailable)?,
                    )
                },
            )
        })
    }

    pub(crate) fn getter_value(
        &self,
        value: &Value,
        kind: NativeScalarGetterKind,
    ) -> Result<NativeScalarGetterValue, ValueError> {
        self.bind_jim_using(value, || self.current_value_for_value(value))?;
        self.with_environment_value_for_value(value, |environment, check| {
            value.native_scalar_getter_checked_with_string(
                self.dialect,
                kind,
                environment,
                check,
                &mut || {
                    self.string_value(
                        value,
                        self.dialect
                            .native_string_protocol()
                            .ok_or(ValueError::ScalarNumericInputUnavailable)?,
                    )
                },
            )
        })
    }

    pub(crate) fn probe_value(
        &self,
        value: &Value,
        kind: NativeScalarGetterKind,
    ) -> Result<Result<NativeScalarGetterValue, NativeScalarGetterFailure>, ValueError> {
        self.bind_jim_using(value, || self.current_value_for_value(value))?;
        self.with_environment_value_for_value(value, |environment, check| {
            value.native_scalar_probe_checked_with_string(
                self.dialect,
                kind,
                environment,
                check,
                &mut || {
                    self.string_value(
                        value,
                        self.dialect
                            .native_string_protocol()
                            .ok_or(ValueError::ScalarNumericInputUnavailable)?,
                    )
                },
            )
        })
    }

    pub(crate) fn number_value(
        &self,
        value: &Value,
        kind: NativeNumberGetterKind,
    ) -> Result<Result<Number, NativeScalarGetterFailure>, ValueError> {
        self.current_value_for_value(value)?;
        let reached = value.native_number_probe_checked_with_string(
            self.dialect,
            kind,
            &mut || self.current_value_for_value(value),
            &mut || {
                self.string_value(
                    value,
                    self.dialect
                        .native_string_protocol()
                        .ok_or(ValueError::ScalarNumericInputUnavailable)?,
                )
            },
        );
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
            value.native_scalar_probe_checked_with_string(
                self.dialect,
                kind,
                environment,
                check,
                &mut || {
                    self.string(
                        value,
                        self.dialect
                            .native_string_protocol()
                            .ok_or(ValueError::ScalarNumericInputUnavailable)?,
                    )
                },
            )
        })
    }

    pub(crate) fn number(
        &self,
        value: &Value,
        kind: NativeNumberGetterKind,
    ) -> Result<Result<Number, NativeScalarGetterFailure>, CmdError> {
        self.current_for_value(value)?;
        let reached = value.native_number_probe_checked_with_string(
            self.dialect,
            kind,
            &mut || self.current_for_value(value),
            &mut || {
                self.string(
                    value,
                    self.dialect
                        .native_string_protocol()
                        .ok_or(ValueError::ScalarNumericInputUnavailable)?,
                )
            },
        );
        self.current_for_value(value)?;
        reached
    }

    pub(crate) fn expression_integer84(
        &self,
        value: &Value,
    ) -> Result<Result<NativeScalarGetterValue, NativeScalarGetterFailure>, CmdError> {
        self.with_environment_for_value(value, |environment, check| {
            value.original_expression_integer84_checked_with_string(
                self.dialect,
                environment,
                check,
                &mut || {
                    self.string(
                        value,
                        self.dialect
                            .native_string_protocol()
                            .ok_or(ValueError::ScalarNumericInputUnavailable)?,
                    )
                },
            )
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
        let reached = value.native_scalar_failure_presentation_checked_with_string(
            self.dialect,
            kind,
            failure,
            &mut || self.current_for_value(value),
            &mut || {
                self.string(
                    value,
                    self.dialect
                        .native_string_protocol()
                        .ok_or(ValueError::ScalarNumericInputUnavailable)?,
                )
            },
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
        // A scalar takes the physical recipe; a list or dictionary member the
        // object recipe entered with this operation.
        if self.dialect.native_string_protocol() != Some(protocol) && self.objects != Some(protocol)
        {
            return Err(
                ValueError::CommandProtocolUnavailable("original VM String protocol").into(),
            );
        }
        self.bind_jim_using(value, || current(value))?;
        if let Some(bytes) = value.resident_string_bytes() {
            current(value)?;
            return Ok(bytes);
        }
        // A compound's members reach the host formatter only where the
        // release formats integers through it; a scalar's stage always asks.
        let formatter = (!value.is_native_compound()
            || crate::value::host_formats_integers(protocol))
        .then(|| self.host.native_integer_formatter())
        .flatten();
        current(value)?;
        let reached = value.native_string_bytes_checked_for_value_retaining(
            protocol,
            formatter,
            &mut |original| {
                current(value)?;
                current(original)
            },
            &|error| self.retain_failure(&error.clone().into()),
        );
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

    /// Compose an original caller/frame check at every reached String stage.
    pub(crate) fn string_with_original_check(
        &self,
        value: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        check: impl Fn(&mut Vm) -> Result<(), CmdError>,
    ) -> Result<Rc<[u8]>, CmdError> {
        self.string_using(value, protocol, |actual| {
            self.current_for_value(actual)?;
            let reached = check(&mut self.vm.borrow_mut());
            if let Err(error) = &reached {
                self.retain_failure(error);
            }
            reached
        })
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

impl OriginalNumericOps<'_> {
    fn arithmetic_current(&self) -> Result<(), crate::error::TclError> {
        self.scope
            .check_native(&mut self.vm.borrow_mut())
            .map_err(crate::error::TclError::from_execution_failure)
    }

    fn arithmetic_current_for_value(&self, value: &Value) -> Result<(), crate::error::TclError> {
        self.current_for_value_native(value)
            .map_err(crate::error::TclError::from_execution_failure)
    }
}

impl crate::expr::OriginalArithmeticStages for OriginalNumericOps<'_> {
    fn current(&self) -> Result<(), crate::error::TclError> {
        self.arithmetic_current()
    }

    fn current_for_value(&self, value: &Value) -> Result<(), crate::error::TclError> {
        self.arithmetic_current_for_value(value)
    }

    fn string(&self, value: &Value) -> Result<Rc<[u8]>, crate::error::TclError> {
        self.arithmetic_current_for_value(value)?;
        let protocol = self
            .dialect
            .native_string_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        self.string_using(value, protocol, |actual| {
            self.arithmetic_current_for_value(actual)
        })
    }

    fn prepare_integer84(&self, value: &Value) -> Result<bool, crate::error::TclError> {
        self.with_environment_using(
            |environment, check| {
                value.prepare_native_expression_integer84_checked(
                    self.dialect,
                    environment,
                    check,
                    &mut || crate::expr::OriginalArithmeticStages::string(self, value),
                )
            },
            || self.arithmetic_current_for_value(value),
        )
    }

    fn scalar_probe(
        &self,
        value: &Value,
        kind: NativeScalarGetterKind,
    ) -> Result<Result<NativeScalarGetterValue, NativeScalarGetterFailure>, crate::error::TclError>
    {
        self.bind_jim_using(value, || self.arithmetic_current_for_value(value))?;
        self.with_environment_using(
            |environment, check| {
                value.native_scalar_probe_checked_with_string(
                    self.dialect,
                    kind,
                    environment,
                    check,
                    &mut || crate::expr::OriginalArithmeticStages::string(self, value),
                )
            },
            || self.arithmetic_current_for_value(value),
        )
    }

    fn integer(
        &self,
        value: &Value,
        input: tcl_syntax::number::NativeScalarNumericInputPolicy,
        syntax: tcl_dialect::NumberSyntax,
    ) -> Result<i64, crate::error::TclError> {
        value.native_int_checked(
            input,
            syntax,
            &mut || self.arithmetic_current_for_value(value),
            &mut || crate::expr::OriginalArithmeticStages::string(self, value),
        )
    }

    fn boolean(
        &self,
        value: &Value,
        input: tcl_syntax::number::NativeScalarNumericInputPolicy,
        syntax: tcl_dialect::NumberSyntax,
    ) -> Result<bool, crate::error::TclError> {
        value.native_bool_checked(
            input,
            syntax,
            &mut || self.arithmetic_current_for_value(value),
            &mut || crate::expr::OriginalArithmeticStages::string(self, value),
        )
    }

    fn cache(&self, value: &Value, number: Number) -> Result<(), crate::error::TclError> {
        value.cache_numeric_representation_checked(
            number,
            &mut || self.arithmetic_current_for_value(value),
            &mut || crate::expr::OriginalArithmeticStages::string(self, value),
        )
    }

    fn store_long84(&self, value: &Value, integer: i64) -> Result<(), crate::error::TclError> {
        value.store_native_expression_long84_checked(integer, self.dialect, &mut || {
            self.arithmetic_current_for_value(value)
        })
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "native_string_stage_tests.rs"]
mod original_string_stage_tests;

#[cfg(test)]
#[path = "native_compound_stage_tests.rs"]
mod original_compound_stage_tests;
