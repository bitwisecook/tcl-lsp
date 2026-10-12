// SPDX-License-Identifier: AGPL-3.0-or-later
//! One original level header within its actual entered frame operation.
//!
//! The release-selected Registry recipe owns conversion and cache order. The
//! argument lease keeps memory safe without changing native sharing; neither a
//! numeric cache nor a levelReference retains an interpreter or frame target.

use crate::{
    interp::{
        Interp, native_arguments::OriginalArguments,
        native_operation_currency::CheckedNumericEnvironment,
    },
    obj::{self, TclObj},
};
use std::{
    cell::Cell,
    rc::{Rc, Weak},
};
use tcl_cmd_core::CmdError;
use tcl_registry::{InvocationDialect, frame_effect::NativeFrameLevelObject};
use tcl_syntax::{
    scalar_getter::{
        NativeScalarGetterConversion, NativeScalarGetterFailure, NativeScalarGetterKind,
        NativeScalarGetterProtocol, NativeScalarGetterValue,
    },
    value::ValueError,
};

pub(super) struct OriginalLevel {
    arguments: OriginalArguments,
    interpreter: Interp,
    dialect: InvocationDialect,
    host: Rc<dyn tcl_platform::Host>,
    abi: Cell<Option<tcl_platform::NativeCIntegerAbi>>,
    current_level: usize,
    current_activation: u64,
    current_owner: Weak<()>,
}

impl OriginalLevel {
    pub(super) fn selected(interpreter: &Interp, value: *mut TclObj) -> Result<Self, CmdError> {
        let arguments = OriginalArguments::enter(interpreter, &[value])?;
        let dialect = interpreter.eval_frame_dialect();
        dialect
            .native_frame_level_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native original-object frame protocol",
            ))?;
        let (current_level, current_activation, current_owner) = {
            let frames = interpreter.frames.borrow();
            (
                frames.current_level(),
                frames.current_activation(),
                frames.current_activation_owner(),
            )
        };
        let selected = Self {
            arguments,
            interpreter: interpreter.clone(),
            dialect,
            host: interpreter.host(),
            abi: Cell::new(None),
            current_level,
            current_activation,
            current_owner,
        };
        selected.check()?;
        selected.after(interpreter.associate_native_jim_arguments(&[value]))?;
        Ok(selected)
    }

    pub(super) fn current_level(&self) -> usize {
        self.current_level
    }

    fn value(&self) -> Result<*mut TclObj, CmdError> {
        self.check()?;
        self.arguments.original(0)
    }

    pub(super) fn check(&self) -> Result<(), CmdError> {
        self.arguments.check()?;
        let same_frame = {
            let frames = self.interpreter.frames.borrow();
            frames.current_level() == self.current_level
                && frames.current_activation() == self.current_activation
                && Weak::ptr_eq(&frames.current_activation_owner(), &self.current_owner)
        };
        if !same_frame || self.interpreter.eval_frame_dialect() != self.dialect {
            return Err(self.retain(
                ValueError::CommandProtocolUnavailable("stale original frame-level operation")
                    .into(),
            ));
        }
        self.arguments.original(0)?;
        Ok(())
    }

    pub(super) fn after<T>(&self, result: Result<T, ValueError>) -> Result<T, CmdError> {
        self.check()?;
        result.map_err(|error| self.retain(error.into()))
    }

    fn retain(&self, error: CmdError) -> CmdError {
        if let Some(first) = error.native_execution_refusal() {
            self.interpreter
                .clone()
                .refuse_native_execution(first.clone());
        }
        error
    }

    /// C9's reached single-script List probe uses the same original header.
    pub(super) fn single_script_list_len(&self) -> Result<usize, CmdError> {
        let original = self.value()?;
        let protocol =
            self.dialect
                .native_string_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "native frame List probe",
                ))?;
        let result = crate::list::list_elements_native_with_currency(
            original,
            protocol,
            Some(self.arguments.currency()),
        );
        self.after(result).map(|elements| elements.len())
    }

    fn environment(&self) -> Result<CheckedNumericEnvironment<'_>, CmdError> {
        self.check()?;
        let actual = self.host.numeric_environment();
        self.check()?;
        let actual = self.after(actual.ok_or(ValueError::ScalarNumericInputUnavailable))?;
        Ok(CheckedNumericEnvironment::new(
            actual,
            Some(self.arguments.currency()),
            &self.abi,
        ))
    }
}

impl NativeFrameLevelObject for OriginalLevel {
    type Error = CmdError;

    fn native_frame_string(&mut self) -> Result<Vec<u8>, Self::Error> {
        let original = self.value()?;
        let protocol =
            self.dialect
                .native_string_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "native frame string",
                ))?;
        self.after(obj::native_frame_level_cache_in(original, self.dialect))?;
        let formatter = self.host.native_integer_formatter();
        self.check()?;
        self.after(crate::dict::native_object_bytes_with_integer_formatter(
            original, protocol, formatter,
        ))
    }

    fn native_frame_probe(
        &mut self,
        kind: NativeScalarGetterKind,
    ) -> Result<Result<i64, NativeScalarGetterFailure>, Self::Error> {
        let original = self.value()?;
        self.after(obj::native_frame_level_cache_in(original, self.dialect))?;
        let environment = self.environment()?;
        let outcome = crate::typed_value::native_scalar_probe_with_environment_and_currency(
            original,
            self.dialect,
            kind,
            Some(&environment),
            Some(self.arguments.currency()),
        );
        self.after(outcome).map(|result| {
            result.map(|value| match value {
                NativeScalarGetterValue::Wide(integer) => integer,
                _ => unreachable!("integer frame getter"),
            })
        })
    }

    fn native_frame_temporary_int(
        &mut self,
        protocol: NativeScalarGetterProtocol,
        bytes: &[u8],
    ) -> Result<NativeScalarGetterConversion, Self::Error> {
        self.check()?;
        if self.dialect.native_scalar_getter_protocol() != Some(protocol) {
            return Err(self.retain(ValueError::ScalarNumericInputUnavailable.into()));
        }
        let environment = self.environment()?;
        self.after(tcl_cmd_core::native_numeric::scalar_getter_recipe_abi(
            protocol,
            NativeScalarGetterKind::Int,
            tcl_syntax::scalar_getter::NativeScalarGetterStage::Fresh,
            &environment,
        ))?;
        let result = if protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4) {
            tcl_cmd_core::native_numeric::fresh_c84_conversion(
                protocol,
                NativeScalarGetterKind::Int,
                bytes,
                &environment,
            )
        } else {
            protocol
                .fresh_conversion(NativeScalarGetterKind::Int, bytes)
                .ok_or(ValueError::ScalarNumericInputUnavailable)
        };
        // This is the recipe's temporary getter. Its cache is never adopted
        // on the original level header; Registry selects any levelReference.
        self.after(result)
    }

    fn native_frame_is_integer(&self) -> Result<bool, Self::Error> {
        Ok(matches!(
            self.after(obj::native_scalar_cache(self.value()?))?,
            Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
                tcl_syntax::number::Number::Int(_) | tcl_syntax::number::Number::Big { .. }
            ))
        ))
    }

    fn native_frame_is_machine_integer(&self) -> Result<bool, Self::Error> {
        Ok(matches!(
            self.after(obj::native_scalar_cache(self.value()?))?,
            Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
                tcl_syntax::number::Number::Int(_)
            ))
        ))
    }

    fn native_frame_cache(
        &self,
    ) -> Result<Option<tcl_registry::NativeFrameLevelCache>, Self::Error> {
        self.after(obj::native_frame_level_cache_in(
            self.value()?,
            self.dialect,
        ))
    }

    fn native_frame_set_cache(
        &mut self,
        cache: tcl_registry::NativeFrameLevelCache,
    ) -> Result<(), Self::Error> {
        self.after(obj::install_native_frame_level_cache(
            self.value()?,
            cache,
            self.dialect,
        ))
    }
}
