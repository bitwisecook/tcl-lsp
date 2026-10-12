// SPDX-License-Identifier: AGPL-3.0-or-later
//! Frame recipes retain their own operand purpose and the original entered currency.

use crate::interp::{
    native_numeric_ops::OriginalNumericOps, native_operation_currency::NativeOperationScope,
};
use crate::{Value, Vm};
use tcl_cmd_core::CmdError;
use tcl_registry::frame_effect::NativeFrameLevelObject;
use tcl_syntax::{
    scalar_getter::{NativeScalarGetterKind, NativeScalarGetterProtocol, NativeScalarGetterValue},
    value::ValueError,
};

pub(super) struct CheckedFrameLevel<'a, 'vm> {
    value: &'a Value,
    numeric: OriginalNumericOps<'vm>,
}

impl<'a, 'vm> CheckedFrameLevel<'a, 'vm> {
    pub(super) fn selected(
        vm: &'vm mut Vm,
        value: &'a Value,
        scope: &NativeOperationScope,
    ) -> Result<Self, CmdError> {
        let numeric = OriginalNumericOps::in_original_scope(vm, scope)?;
        numeric.current_for_value(value)?;
        Ok(Self { value, numeric })
    }

    pub(super) fn current(&self) -> Result<(), CmdError> {
        self.numeric.current_for_value(self.value)
    }

    fn after<T>(&self, reached: Result<T, ValueError>) -> Result<T, CmdError> {
        self.current()?;
        reached.map_err(Into::into)
    }
}

impl NativeFrameLevelObject for CheckedFrameLevel<'_, '_> {
    type Error = CmdError;

    fn native_frame_string(&mut self) -> Result<Vec<u8>, Self::Error> {
        self.current()?;
        let protocol = self.numeric.dialect().native_string_protocol().ok_or(
            ValueError::CommandProtocolUnavailable("native frame string"),
        )?;
        let reached = self.numeric.string(self.value, protocol);
        self.current()?;
        reached.map(|bytes| bytes.to_vec())
    }

    fn native_frame_probe(
        &mut self,
        kind: NativeScalarGetterKind,
    ) -> Result<Result<i64, tcl_syntax::scalar_getter::NativeScalarGetterFailure>, Self::Error>
    {
        self.current()?;
        let reached = self.numeric.probe(self.value, kind);
        self.current()?;
        reached.map(|outcome| {
            outcome.map(|value| match value {
                NativeScalarGetterValue::Wide(integer) => integer,
                _ => unreachable!("integer frame getter"),
            })
        })
    }

    fn native_frame_temporary_int(
        &mut self,
        protocol: NativeScalarGetterProtocol,
        bytes: &[u8],
    ) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterConversion, Self::Error> {
        use tcl_syntax::scalar_getter::NativeScalarGetterStage;
        self.current()?;
        if self.numeric.dialect().native_scalar_getter_protocol() != Some(protocol) {
            return Err(ValueError::ScalarNumericInputUnavailable.into());
        }
        let reached = self
            .numeric
            .with_environment_for_value(self.value, |environment, check| {
                let actual = environment.ok_or(ValueError::ScalarNumericInputUnavailable)?;
                check()?;
                let abi = tcl_cmd_core::native_numeric::scalar_getter_recipe_abi(
                    protocol,
                    NativeScalarGetterKind::Int,
                    NativeScalarGetterStage::Fresh,
                    actual,
                );
                check()?;
                abi?;
                let converted = if protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4) {
                    tcl_cmd_core::native_numeric::fresh_c84_conversion(
                        protocol,
                        NativeScalarGetterKind::Int,
                        bytes,
                        actual,
                    )
                } else {
                    protocol
                        .fresh_conversion(NativeScalarGetterKind::Int, bytes)
                        .ok_or(ValueError::ScalarNumericInputUnavailable)
                };
                check()?;
                converted.map_err(Into::into)
            });
        self.current()?;
        reached
    }

    fn native_frame_is_integer(&self) -> Result<bool, Self::Error> {
        self.current()?;
        self.after(Ok(matches!(
            self.value.native_scalar_cache(),
            Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
                tcl_syntax::number::Number::Int(_) | tcl_syntax::number::Number::Big { .. }
            ))
        )))
    }

    fn native_frame_is_machine_integer(&self) -> Result<bool, Self::Error> {
        self.current()?;
        self.after(Ok(matches!(
            self.value.native_scalar_cache(),
            Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
                tcl_syntax::number::Number::Int(_)
            ))
        )))
    }

    fn native_frame_cache(
        &self,
    ) -> Result<Option<tcl_registry::NativeFrameLevelCache>, Self::Error> {
        self.current()?;
        self.after(
            self.value
                .native_frame_level_cache_in(self.numeric.dialect()),
        )
    }

    fn native_frame_set_cache(
        &mut self,
        cache: tcl_registry::NativeFrameLevelCache,
    ) -> Result<(), Self::Error> {
        self.current()?;
        self.after(
            self.value
                .install_native_frame_level_cache(cache, self.numeric.dialect()),
        )
    }
}

#[cfg(all(test, unix, not(target_arch = "wasm32")))]
mod tests;
