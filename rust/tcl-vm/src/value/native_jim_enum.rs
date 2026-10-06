// SPDX-License-Identifier: AGPL-3.0-or-later
//! Same-header Jim option caches with retained original static declaration authority.
use super::{IntRep, Value};
use tcl_core_types::NativeJimOptionCache;
use tcl_syntax::value::ValueError;
impl Value {
    pub(crate) fn native_jim_option_cache(&self) -> Option<NativeJimOptionCache> {
        match &*self.0.intrep.borrow() {
            IntRep::JimOption(cache) => Some(cache.clone()),
            _ => None,
        }
    }
    pub(crate) fn install_native_jim_option_cache(
        &self,
        cache: NativeJimOptionCache,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<(), ValueError> {
        self.check_native_header()?;
        dialect
            .native_jim_enum_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "Jim Enum primary issuer",
            ))?;
        if self.resident_string_bytes().is_none() {
            return Err(ValueError::CommandProtocolUnavailable(
                "Jim Enum resident spelling",
            ));
        }
        self.replace_primary(IntRep::JimOption(cache));
        Ok(())
    }
}
