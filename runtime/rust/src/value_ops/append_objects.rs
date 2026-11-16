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

//! The physical append and string-concatenation adapter.

use super::original_append::OriginalRuntimeAppendObjects;
use super::{RuntimeAppendValue, RuntimeDictionaryChecks};
use crate::obj;
use crate::obj::TclObj;
use std::rc::Rc;
use tcl_syntax::value::ValueError;

impl RuntimeAppendObjects {
    /// The actual append caller supplies its original receiver/frame checks;
    /// this uses the existing shared String/current/error owner without issuing
    /// a dictionary, variable, operation or Native reference role.
    pub(crate) fn with_original_checks(
        self,
        current: impl Fn() -> Result<(), tcl_cmd_core::CmdError> + 'static,
        string: impl Fn(
                *mut TclObj,
                &dyn Fn() -> Result<(), tcl_cmd_core::CmdError>,
            ) -> Result<std::rc::Rc<[u8]>, tcl_cmd_core::CmdError>
            + 'static,
        refuse: impl Fn(ValueError) -> tcl_cmd_core::CmdError + 'static,
        retain: impl Fn(tcl_cmd_core::CmdError) -> tcl_cmd_core::CmdError + 'static,
    ) -> OriginalRuntimeAppendObjects {
        OriginalRuntimeAppendObjects::new(
            self,
            RuntimeDictionaryChecks::new(current, string, refuse, retain),
        )
    }
}

/// Physical append adapter with independently retained issuer and binary updater capability.
pub(crate) struct RuntimeAppendObjects {
    pub(crate) dialect: tcl_registry::InvocationDialect,
    pub(crate) binary_recipe:
        Option<tcl_registry::native_string_materialization::ByteArrayStringRecipe>,
}

impl tcl_cmd_core::native_append::NativeAppendObjects for RuntimeAppendObjects {
    type Value = RuntimeAppendValue;
    fn snapshot(
        &self,
        value: &Self::Value,
    ) -> Result<tcl_syntax::native_object::NativeObjectSnapshot, ValueError> {
        obj::native_object_snapshot(value.as_ptr())
    }
    fn prepare_receiver(
        &self,
        value: &Self::Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Self::Value {
        RuntimeAppendValue::retain(if obj::is_shared(value.as_ptr()) {
            let duplicate = obj::duplicate(value.as_ptr());
            crate::list::duplicate_native_backing(value.as_ptr(), duplicate, protocol);
            duplicate
        } else {
            value.as_ptr()
        })
    }
    fn duplicate_into(&self, receiver: &Self::Value, source: &Self::Value) {
        obj::duplicate_into(receiver.as_ptr(), source.as_ptr());
    }
    fn string(
        &self,
        value: &Self::Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Rc<[u8]>, ValueError> {
        if !obj::native_string_available(value.as_ptr()) {
            return Err(ValueError::CommandProtocolUnavailable(
                "native append string updater",
            ));
        }
        if !obj::has_string_rep(value.as_ptr())
            && obj::obj_type_ptr(value.as_ptr()) == &crate::bytearray::TCL_BYTE_ARRAY_TYPE
        {
            let recipe = self
                .binary_recipe
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "native append binary string recipe",
                ))?;
            if recipe.protocol() != protocol {
                return Err(ValueError::CommandProtocolUnavailable(
                    "native append binary recipe origin",
                ));
            }
        }
        Ok(Rc::from(obj::bytes_of(value.as_ptr())))
    }
    fn unicode(
        &self,
        value: &Self::Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Rc<[u32]>, ValueError> {
        obj::native_unicode_units(value.as_ptr(), protocol)
    }
    fn decode_unicode_source(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        bytes: &[u8],
    ) -> Result<Rc<[u32]>, ValueError> {
        let version = protocol
            .tcl_version()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original append C Unicode source units",
            ))?;
        Ok(Rc::from(
            tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version).decode_storage_units(
                bytes,
                tcl_runtime_api::native_c_string_abi::supported_backend_plain_char(),
            ),
        ))
    }
    fn set_string(
        &self,
        value: &Self::Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        resident: Option<(
            Rc<[u8]>,
            tcl_syntax::native_string::NativeStringStorageIdentity,
        )>,
        count: Option<usize>,
        unicode: Option<Rc<[u32]>>,
    ) -> Result<(), ValueError> {
        if resident.is_none() && self.dialect.native_string_protocol() == Some(protocol) {
            if let Some(unicode) = &unicode {
                let fresh = obj::Owned::fresh(obj::new_native_unicode_obj(
                    Rc::clone(unicode),
                    self.dialect,
                )?);
                obj::duplicate_into(value.as_ptr(), fresh.as_ptr());
                return Ok(());
            }
        }
        obj::set_native_append_string(value.as_ptr(), protocol, resident, count, unicode)
    }
    fn set_binary(
        &self,
        value: &Self::Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        bytes: Rc<[u8]>,
    ) -> Result<(), ValueError> {
        let recipe = self
            .binary_recipe
            .filter(|recipe| recipe.protocol() == protocol)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native append binary updater recipe",
            ))?;
        crate::bytearray::set_native_append_bytes(value.as_ptr(), &bytes, recipe);
        Ok(())
    }
    fn new_string(&self, bytes: Rc<[u8]>) -> Self::Value {
        RuntimeAppendValue::retain(crate::interp::new_string(&bytes))
    }
}

impl tcl_cmd_core::native_cat::NativeCatObjects for RuntimeAppendObjects {
    fn empty_binary(
        &self,
        value: &Self::Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Rc<[u8]>, ValueError> {
        let version = protocol
            .tcl_version()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "C compiled concat binary getter",
            ))?;
        let recipe = self
            .binary_recipe
            .filter(|recipe| recipe.protocol() == protocol)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native compiled concat binary issuer",
            ))?;
        crate::bytearray::native_binary_bytes(
            value.as_ptr(),
            tcl_registry::native_binary_value::NativeBinaryByteConversion::Narrow(
                version.string_character_model(),
            ),
            true,
            recipe,
        )
        .map(Rc::from)
        .map_err(|_| {
            ValueError::CommandProtocolUnavailable("native compiled concat empty binary conversion")
        })
    }
    fn is_shared(&self, value: &Self::Value) -> bool {
        obj::is_shared(value.as_ptr())
    }
    fn set_plain_string(&self, value: &Self::Value, bytes: Rc<[u8]>) -> Result<(), ValueError> {
        unsafe {
            obj::set_string_rep(value.as_ptr(), &bytes);
        }
        obj::discard_native_internal_representation(value.as_ptr())
    }
}
