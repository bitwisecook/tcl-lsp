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
        self.install_native_jim_option_cache_using::<ValueError>(cache, dialect, &mut None)
    }
    pub(crate) fn install_native_jim_option_cache_checked<E: From<ValueError>>(
        &self,
        cache: NativeJimOptionCache,
        dialect: tcl_registry::InvocationDialect,
        check: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        self.install_native_jim_option_cache_using(cache, dialect, &mut Some(check))
    }
    fn install_native_jim_option_cache_using<E: From<ValueError>>(
        &self,
        cache: NativeJimOptionCache,
        dialect: tcl_registry::InvocationDialect,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
    ) -> Result<(), E> {
        Self::native_stage_check(check)?;
        self.check_native_header()?;
        dialect
            .native_jim_enum_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "Jim Enum primary issuer",
            ))?;
        if self.resident_string_bytes().is_none() {
            return Err(
                ValueError::CommandProtocolUnavailable("Jim Enum resident spelling").into(),
            );
        }
        if check.is_none() {
            self.replace_primary(IntRep::JimOption(cache));
            return Ok(());
        }
        self.replace_primary_using_with(
            || {
                if self.resident_string_bytes().is_none() {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "Jim Enum resident spelling",
                    )
                    .into());
                }
                Ok(IntRep::JimOption(cache))
            },
            check,
        )
    }
}
