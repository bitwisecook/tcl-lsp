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
