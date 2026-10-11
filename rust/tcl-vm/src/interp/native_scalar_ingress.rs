// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual scalar transport ingress, separate from recorded cache descriptor data.
use super::{Vm, native_operation_currency::NativeOperationScope};
use crate::{Value, error::TclHostFailure};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use tcl_core_types::NativeScalarCache as Carrier;
use tcl_platform::{NativeCIntegerAbi, NumericEnvironment};
use tcl_registry::InvocationDialect;
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::{
    native_string::NativeStringStorageIdentity,
    scalar_getter::{NativeScalarCache, carrier},
    value::ValueError,
};

/// An actual interpreter retains its original operation before recursive import.
/// This receipt adds no Native object references; recorded origin cannot issue it.
pub struct NativeScalarIngress {
    scope: NativeOperationScope,
    dialect: InvocationDialect,
    wide_abi: Cell<Option<NativeCIntegerAbi>>,
}
impl NativeScalarIngress {
    pub(crate) fn dialect(&self) -> InvocationDialect {
        self.dialect
    }
    pub(crate) fn validate_cache(&self, cache: &NativeScalarCache) -> Result<(), ValueError> {
        let protocol = self
            .dialect
            .native_scalar_getter_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        if matches!(
            cache,
            NativeScalarCache::Number(tcl_syntax::number::Number::Int(_))
        ) && protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4)
        {
            let abi = self
                .wide_abi
                .get()
                .ok_or(ValueError::ScalarNumericInputUnavailable)?;
            protocol
                .validate_cache_abi(cache, abi.char_bits, abi.int_bytes, abi.long_bytes)
                .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
        }
        Ok(())
    }
}
impl Vm {
    /// Capture the actual original interpreter before scalar/container transport.
    /// Other generic values need no numeric capability and issue no Wide descriptor.
    ///
    /// # Errors
    /// Returns the retained first failure of this actual operation.
    pub fn capture_original_scalar_ingress(
        &mut self,
    ) -> Result<NativeScalarIngress, NativeExecutionError> {
        let scope = NativeOperationScope::enter_native(self)?;
        let original = NativeScalarIngress {
            dialect: self.native_scalar_carrier_dialect(),
            scope,
            wide_abi: Cell::new(None),
        };
        self.check_original_scalar_ingress(&original)?;
        Ok(original)
    }

    /// Check the same retained actual interpreter, context, Host and operation.
    ///
    /// # Errors
    /// Retains the original typed cause instead of issuing another receipt.
    pub fn check_original_scalar_ingress(
        &mut self,
        original: &NativeScalarIngress,
    ) -> Result<(), NativeExecutionError> {
        original.scope.check_native(self)
    }

    /// Finish the actual transport boundary after its owned value cleanup.
    /// A surrounding invocation continues to own its first refusal; only the
    /// existing outer Host boundary may retire that transported cause.
    ///
    /// # Errors
    /// Returns the same original cause after releasing this allocation-only scope.
    pub fn finish_original_scalar_ingress(
        &mut self,
        original: NativeScalarIngress,
    ) -> Result<(), NativeExecutionError> {
        let current = self.check_original_scalar_ingress(&original);
        drop(original);
        match self.take_host_boundary_refusal() {
            Some(first) => Err(first),
            None => current,
        }
    }

    /// Construct a transported scalar under the actual captured ingress.
    /// C84 Long/Wide data remain distinct; an unqualified Integer supplies neither.
    ///
    /// # Errors
    /// Refuses unavailable/foreign origins and preserves the first execution cause.
    pub fn import_original_native_scalar(
        &mut self,
        original: &NativeScalarIngress,
        facts: &Carrier,
        resident: Option<(Rc<[u8]>, NativeStringStorageIdentity)>,
    ) -> Result<Value, NativeExecutionError> {
        self.check_original_scalar_ingress(original)?;
        let (cache, origin) = carrier::import_scalar(facts);
        let outcome = (|| -> Result<Value, ValueError> {
            let protocol = original
                .dialect
                .native_scalar_getter_protocol()
                .ok_or(ValueError::ScalarNumericInputUnavailable)?;
            if origin.is_some_and(|origin| protocol.tcl_version() != Some(origin))
                || (matches!(facts, Carrier::Integer(_))
                    && protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4))
            {
                return Err(ValueError::CommandProtocolUnavailable(
                    "original scalar descriptor origin unavailable or foreign",
                ));
            }
            if matches!(facts, Carrier::Tcl84Wide(_)) {
                let host = self.host_rc();
                self.check_original_scalar_ingress(original)
                    .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
                let environment = host.numeric_environment();
                self.check_original_scalar_ingress(original)
                    .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
                let environment = environment.ok_or(ValueError::ScalarNumericInputUnavailable)?;
                let abi = {
                    let vm = RefCell::new(&mut *self);
                    let current = || {
                        original
                            .scope
                            .check_native(&mut vm.borrow_mut())
                            .map_err(|_| tcl_platform::NumericEnvironmentUnavailable::Target)
                    };
                    let checked = tcl_platform::CheckedNumericEnvironment::new(
                        environment,
                        &current,
                        &original.wide_abi,
                    );
                    checked
                        .c_integer_abi()
                        .map_err(|_| ValueError::ScalarNumericInputUnavailable)
                };
                self.check_original_scalar_ingress(original)
                    .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
                abi?;
            }
            Value::from_native_scalar_cache_with_ingress(cache, origin, original, resident)
        })();
        // Retain the actual intrinsic cause before any unused created owner drops.
        let outcome = outcome.map_err(|error| {
            self.retain_tcl_host_failure(TclHostFailure::ValueAccess(
                error.native_access_refusal().unwrap_or(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::ScalarNumericInputUnavailable,
                ),
            ))
        });
        self.check_original_scalar_ingress(original)?;
        outcome
    }
}

#[cfg(test)]
#[path = "native_scalar_ingress/tests.rs"]
mod tests;
