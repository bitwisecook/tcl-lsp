// SPDX-License-Identifier: AGPL-3.0-or-later
//! The actual C execution environment's retained Boolean constants.

use super::Interp;
use crate::obj::{self, TclObj};
use tcl_syntax::value::ValueError;

impl Interp {
    pub(super) fn native_compiled_match_result(
        &mut self,
        pattern: obj::Owned,
        matched: bool,
        version: tcl_dialect::TclVersion,
        operation: tcl_registry::native_string_compilation::NativeStringMatchOperation,
    ) -> Result<obj::Owned, ValueError> {
        let dialect = self.native_invocation_dialect();
        if dialect.native_error_log_protocol().is_none() || dialect.tcl_version != Some(version) {
            return Err(ValueError::CommandProtocolUnavailable(
                "native compiled match result",
            ));
        }
        if tcl_registry::native_string_compilation::uses_execution_constant(version) {
            return self
                .native_execution_boolean_constant(matched)
                .map(obj::Owned::retain);
        }
        let protocol = dialect
            .native_scalar_getter_protocol()
            .filter(|_| version == tcl_dialect::TclVersion::V8_4)
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let result =
            if operation.reuses_unshared_pattern(version) && !obj::is_shared(pattern.as_ptr()) {
                pattern
            } else {
                obj::Owned::fresh(obj::new_obj())
            };
        obj::invalidate_string(result.as_ptr());
        obj::adopt_native_scalar_cache(
            result.as_ptr(),
            tcl_syntax::scalar_getter::NativeScalarCache::Tcl84Long(i64::from(matched)),
            protocol,
        )?;
        Ok(result)
    }

    pub(crate) fn native_execution_boolean_constant(
        &self,
        value: bool,
    ) -> Result<*mut TclObj, ValueError> {
        if self
            .native_invocation_dialect()
            .native_command_name_protocol()
            .is_none()
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "native C execution environment constants",
            ));
        }
        let mut constants = self.native_execution_constants.borrow_mut();
        let constants = constants.get_or_insert_with(|| {
            [
                obj::Owned::fresh(obj::new_boolean_obj(0)),
                obj::Owned::fresh(obj::new_boolean_obj(1)),
            ]
        });
        Ok(constants[usize::from(value)].as_ptr())
    }
}
