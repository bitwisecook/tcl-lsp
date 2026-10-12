// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original authored operand observation, independent of Native entry/provider grants.

use super::{
    Vm,
    native_name_consumers::OriginalNameConsumerState,
    native_numeric_ops::OriginalNumericOps,
    native_operation_currency::{NativeOperationScope, OriginalOperationObservation},
    object_materialization::ObjectMaterialization,
};
use crate::{Completion, Value, error::TclHostFailure};
use std::{cell::RefCell, rc::Rc};
use tcl_cmd_core::CmdError;
use tcl_syntax::{native_string::NativeStringProtocol, value::ValueError};

pub(crate) struct OriginalAuthoredOperandScope {
    observed: OriginalOperationObservation,
    entered: Option<NativeOperationScope>,
    names: OriginalNameConsumerState,
    host: Rc<dyn tcl_platform::Host>,
    strings: Option<NativeStringProtocol>,
    objects: Option<ObjectMaterialization>,
}
impl OriginalAuthoredOperandScope {
    pub(crate) fn capture(vm: &mut Vm) -> Result<Self, CmdError> {
        let observed =
            OriginalOperationObservation::capture(vm).map_err(CmdError::from_execution_refusal)?;
        let entered =
            NativeOperationScope::current_entered(vm).map_err(CmdError::from_execution_refusal)?;
        let names = OriginalNameConsumerState::capture(vm, None, entered.is_some())?;
        let original = Self {
            observed,
            entered,
            names,
            host: vm.host_rc(),
            strings: vm
                .actual_native_invocation_dialect()
                .native_string_protocol(),
            objects: vm.object_materialization(),
        };
        original.check(vm)?;
        Ok(original)
    }
    pub(crate) fn retain(&self, vm: &mut Vm, error: &CmdError) {
        if let Some(first) = error.native_execution_refusal() {
            vm.retain_tcl_host_failure(TclHostFailure::Execution(first.clone()));
        }
    }
    pub(crate) fn check(&self, vm: &mut Vm) -> Result<(), CmdError> {
        self.observed.check(vm)?;
        self.names.check_using(vm, true, |vm, current, purpose| {
            self.observed.check(vm)?;
            if !current {
                let error: CmdError = ValueError::CommandProtocolUnavailable(purpose).into();
                self.retain(vm, &error);
                return Err(error);
            }
            Ok(())
        })
    }

    pub(super) fn replace_observed_inputs(
        &self,
        vm: &mut Vm,
        names: Option<super::execution_name_policy::ObservedNameSelection>,
        storage: Option<super::execution_name_policy::ObservedFrameStorage>,
        mandatory_retirement: bool,
    ) -> Result<(), CmdError> {
        let before = self.check(vm);
        if !mandatory_retirement && let Err(first) = &before {
            return Err(first.clone());
        }
        let reached = self.observed.replace_observed_inputs_using(
            vm,
            names,
            storage,
            mandatory_retirement,
            &mut |vm| self.check(vm),
        );
        if let Err(error) = &reached {
            self.retain(vm, error);
        }
        before?;
        self.check(vm)?;
        reached
    }
    pub(crate) fn current_for_value(&self, vm: &mut Vm, value: &Value) -> Result<(), CmdError> {
        self.check(vm)?;
        value
            .original_native_header_currentness()
            .map_err(|error| {
                let first = vm.retain_tcl_host_failure(TclHostFailure::ValueAccess(error));
                CmdError::from_execution_refusal(first)
            })?;
        self.check(vm)
    }
    pub(crate) fn string(&self, vm: &mut Vm, value: &Value) -> Result<Rc<[u8]>, CmdError> {
        self.current_for_value(vm, value)?;
        let original = value.native_lifetime_lease();
        let reached = if let Some(bytes) = original.value().resident_string_bytes() {
            Ok(bytes)
        } else {
            self.strings
                .ok_or(
                    ValueError::CommandProtocolUnavailable(
                        "authored original operand String provider",
                    )
                    .into(),
                )
                .and_then(|protocol| self.string_with_recipe(vm, original.value(), protocol, false))
        };
        if let Err(error) = &reached {
            self.retain(vm, error);
        }
        drop(original);
        self.current_for_value(vm, value)?;
        reached
    }
    fn string_with_recipe(
        &self,
        vm: &mut Vm,
        value: &Value,
        protocol: NativeStringProtocol,
        independent_objects: bool,
    ) -> Result<Rc<[u8]>, CmdError> {
        self.current_for_value(vm, value)?;
        if !independent_objects && let Some(entered) = &self.entered {
            return OriginalNumericOps::in_original_scope(vm, entered)?.string_with_original_check(
                value,
                protocol,
                |vm| self.check(vm),
            );
        }
        // Independent authored/standalone object conversion keeps its existing
        // NULL formatter. The ordinary None operand door only consumes the
        // genuine recipe already selected at capture, never a profile fallback.
        let formatter = if independent_objects {
            None
        } else {
            self.host.native_integer_formatter()
        };
        self.current_for_value(vm, value)?;
        let vm = RefCell::new(vm);
        value.native_string_bytes_checked_for_value_retaining::<CmdError>(
            protocol,
            formatter,
            &mut |actual| self.current_for_value(&mut vm.borrow_mut(), actual),
            &|error| self.retain(&mut vm.borrow_mut(), &error.clone().into()),
        )
    }
    /// The already captured object purpose supplies binary String conversion;
    /// authored and standalone purposes retain their independent NULL formatter.
    pub(crate) fn binary_string(&self, vm: &mut Vm, value: &Value) -> Result<Rc<[u8]>, CmdError> {
        let purpose = self.objects.ok_or(ValueError::CommandProtocolUnavailable(
            "original binary object materialization",
        ))?;
        self.string_with_recipe(
            vm,
            value,
            purpose.protocol(),
            !matches!(purpose, ObjectMaterialization::Native(_)),
        )
    }
    pub(crate) fn binary_grammar(
        &self,
    ) -> Result<tcl_registry::native_binary_value::NativeBinaryFormatGrammar, CmdError> {
        let purpose = self.objects.ok_or(ValueError::CommandProtocolUnavailable(
            "original binary format grammar",
        ))?;
        Ok(
            tcl_registry::native_binary_value::NativeBinaryFormatGrammar::from_string_protocol(
                purpose.protocol(),
            ),
        )
    }
    pub(crate) fn binary_bytes(
        &self,
        vm: &mut Vm,
        value: &Value,
        conversion: tcl_registry::native_binary_value::NativeBinaryByteConversion,
        format_narrowing: Option<tcl_registry::native_binary_value::NativeBinaryByteConversion>,
    ) -> Result<Rc<[u8]>, CmdError> {
        self.current_for_value(vm, value)?;
        let purpose = self.objects.ok_or(ValueError::CommandProtocolUnavailable(
            "original binary byte materialization",
        ))?;
        let protocol = purpose.protocol();
        let vm = RefCell::new(vm);
        let reached = value.as_native_byte_array_checked_result(
            conversion,
            protocol,
            &mut || self.current_for_value(&mut vm.borrow_mut(), value),
            &mut || self.binary_string(&mut vm.borrow_mut(), value),
        );
        if let Err(error) = &reached {
            self.retain(&mut vm.borrow_mut(), error);
        }
        self.current_for_value(&mut vm.borrow_mut(), value)?;
        let reached = match reached? {
            Ok(bytes) => Ok(bytes),
            Err(crate::value::ByteArrayAccessError::Conversion(error)) => {
                if let Some(narrowing) = format_narrowing {
                    let bytes = self.binary_string(&mut vm.borrow_mut(), value)?;
                    let version =
                        protocol
                            .tcl_version()
                            .ok_or(ValueError::CommandProtocolUnavailable(
                                "original binary narrowing units",
                            ))?;
                    narrowing
                        .convert_native(
                            &bytes,
                            tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version),
                        )
                        .map(Rc::from)
                        .map_err(|error| CmdError::new(error.message()))
                } else {
                    Err(crate::value::ByteArrayAccessError::Conversion(error).into())
                }
            }
            Err(error) => Err(error.into()),
        };
        if let Err(error) = &reached {
            self.retain(&mut vm.borrow_mut(), error);
        }
        self.current_for_value(&mut vm.borrow_mut(), value)?;
        reached
    }
    pub(crate) fn binary_result(&self, vm: &mut Vm, bytes: Vec<u8>) -> Result<Value, CmdError> {
        self.check(vm)?;
        let purpose = self.objects.ok_or(ValueError::CommandProtocolUnavailable(
            "original binary result materialization",
        ))?;
        let value = if purpose.protocol().is_jim084() {
            Value::from_string_bytes(bytes)
        } else {
            let recipe = purpose.byte_array_string_recipe().ok_or(
                ValueError::CommandProtocolUnavailable("original binary result recipe"),
            )?;
            Value::from_byte_array_recipe(Rc::from(bytes), recipe)
        };
        self.check(vm)?;
        Ok(value)
    }

    pub(crate) fn list(&self, vm: &mut Vm, value: &Value) -> Result<Vec<Value>, CmdError> {
        self.current_for_value(vm, value)?;
        let purpose = self.objects.ok_or(ValueError::CommandProtocolUnavailable(
            "authored original object list conversion",
        ))?;
        let protocol = purpose.protocol();
        if protocol.is_jim084() {
            let context = vm.native_jim_object_context()?;
            let reached = value
                .bind_native_jim_context(&context)
                .map_err(CmdError::from);
            if let Err(error) = &reached {
                self.retain(vm, error);
            }
            self.current_for_value(vm, value)?;
            reached?;
        }
        let independent = !matches!(purpose, ObjectMaterialization::Native(_));
        let vm = RefCell::new(vm);
        let reached = value.native_object_list_elements_with_strings(
            protocol,
            &mut || self.current_for_value(&mut vm.borrow_mut(), value),
            &mut |actual| {
                self.string_with_recipe(&mut vm.borrow_mut(), actual, protocol, independent)
            },
        );
        if let Err(error) = &reached {
            self.retain(&mut vm.borrow_mut(), error);
        }
        self.current_for_value(&mut vm.borrow_mut(), value)?;
        reached.map(|items| items.as_ref().clone())
    }
    /// Preserve the legacy raw String projection and formatter-None behavior.
    /// Each reached child uses this original owner before any backing retires.
    pub(crate) fn raw_bytes(&self, vm: &mut Vm, value: &Value) -> Result<Rc<[u8]>, CmdError> {
        self.current_for_value(vm, value)?;
        let vm = RefCell::new(vm);
        value.string_bytes_checked_for_value_retaining::<CmdError>(
            &mut |actual| self.current_for_value(&mut vm.borrow_mut(), actual),
            &|error| self.retain(&mut vm.borrow_mut(), &error.clone().into()),
        )
    }
    pub(crate) fn unicode(&self, vm: &mut Vm, value: &Value) -> Result<String, CmdError> {
        let bytes = self.raw_bytes(vm, value)?;
        let reached = std::str::from_utf8(&bytes)
            .map(str::to_owned)
            .map_err(tcl_syntax::raw_string::UnicodeAccessError::from)
            .map_err(ValueError::from)
            .map_err(CmdError::from);
        if let Err(error) = &reached {
            self.retain(vm, error);
        }
        self.current_for_value(vm, value)?;
        reached
    }
    pub(crate) fn finish(mut self, vm: &mut Vm) -> Result<(), CmdError> {
        let before = self.check(vm);
        self.names.retire_header_lease();
        self.check(vm)?;
        before
    }
}

pub(crate) fn with_original_authored_operands(
    vm: &mut Vm,
    work: impl FnOnce(&mut Vm, &OriginalAuthoredOperandScope) -> Result<Completion<Value>, CmdError>,
) -> Completion<Value> {
    let original = match OriginalAuthoredOperandScope::capture(vm) {
        Ok(original) => original,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    let reached = work(vm, &original);
    if let Err(error) = &reached {
        original.retain(vm, error);
    }
    if let Err(first) = original.check(vm) {
        drop(reached);
        let _ = original.finish(vm);
        return crate::command::completion_from_cmd_error(vm, first);
    }
    if let Err(error) = original.finish(vm) {
        drop(reached);
        return crate::command::completion_from_cmd_error(vm, error);
    }
    reached.unwrap_or_else(|error| crate::command::completion_from_cmd_error(vm, error))
}
