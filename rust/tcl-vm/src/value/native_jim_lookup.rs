// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original Jim lookup descriptors with their actual native ownership.

use super::{IntRep, Value};
use tcl_runtime_api::native_compilation::NativeInterpreterIdentity;
use tcl_syntax::value::ValueError;

#[derive(Clone)]
pub(crate) struct JimCommandCache {
    pub(crate) interpreter: NativeInterpreterIdentity,
    pub(crate) epoch: u64,
    pub(crate) token: u64,
    // A native command descriptor owns the original current namespace object.
    pub(crate) namespace: Value,
}

#[derive(Clone)]
pub(crate) struct JimVariableCache {
    pub(crate) interpreter: NativeInterpreterIdentity,
    pub(crate) frame: u64,
    pub(crate) global: bool,
    // This weak birth receipt owns neither VarVal nor its contents.
    pub(crate) cell: crate::vars::WeakJimVariableCell,
}

impl Value {
    pub(crate) fn with_jim_command_cache<R>(
        &self,
        read: impl FnOnce(&JimCommandCache) -> R,
    ) -> Option<R> {
        match &*self.0.intrep.borrow() {
            IntRep::JimCommand(cache) => Some(read(cache)),
            _ => None,
        }
    }

    pub(crate) fn with_jim_variable_cache<R>(
        &self,
        read: impl FnOnce(&JimVariableCache) -> R,
    ) -> Option<R> {
        match &*self.0.intrep.borrow() {
            IntRep::JimVariable(cache) => Some(read(cache)),
            _ => None,
        }
    }

    fn require_jim_lookup_origin(
        &self,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<(), ValueError> {
        dialect
            .native_jim_lookup_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "Jim original lookup primary issuer",
            ))?;
        if self.resident_string_bytes().is_none() {
            return Err(ValueError::CommandProtocolUnavailable(
                "Jim original lookup resident spelling",
            ));
        }
        Ok(())
    }

    pub(crate) fn install_jim_command_cache(
        &self,
        cache: JimCommandCache,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<(), ValueError> {
        self.require_jim_lookup_origin(dialect)?;
        self.replace_primary(IntRep::JimCommand(cache));
        Ok(())
    }

    pub(crate) fn install_jim_variable_cache(
        &self,
        cache: JimVariableCache,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<(), ValueError> {
        self.require_jim_lookup_origin(dialect)?;
        self.replace_primary(IntRep::JimVariable(cache));
        Ok(())
    }
}
