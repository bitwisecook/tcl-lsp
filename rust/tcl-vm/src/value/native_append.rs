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

//! In-place native append: the `VmAppendObjects` adapter and the binary and string append kernels.

use super::{ByteArrayRep, IntRep, NativeBinaryStorageReceipt, Value};
use std::cell::Cell;
use std::rc::Rc;
use tcl_syntax::native_string::{NativeStringProtocol, NativeStringStorageIdentity};
use tcl_syntax::raw_string::RawString;
use tcl_syntax::value::ValueError;

/// Physical append operations; command adapters supply the independently issued recipe.
pub(crate) struct VmAppendObjects;

impl tcl_cmd_core::native_append::NativeAppendObjects for VmAppendObjects {
    type Value = Value;

    fn snapshot(
        &self,
        value: &Value,
    ) -> Result<tcl_syntax::native_object::NativeObjectSnapshot, tcl_syntax::value::ValueError>
    {
        Ok(value.native_object_snapshot())
    }

    fn prepare_receiver(
        &self,
        value: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Value {
        // Observe original table/argv ownership before the working handle exists.
        if value.native_object_is_shared() {
            value.duplicate_native_object_in(protocol)
        } else {
            value.clone()
        }
    }

    fn duplicate_into(&self, receiver: &Value, source: &Value) {
        let string = source.0.string.borrow().clone();
        let cache = {
            let original = source.0.intrep.borrow();
            match &*original {
                IntRep::NativeString {
                    protocol,
                    num_chars,
                    ..
                } if !protocol.string_primary_survives_duplicate(*num_chars) => IntRep::Str,
                IntRep::JimRegexp(cache) => IntRep::JimRegexp(cache.duplicate()),
                _ => original.clone(),
            }
        };
        let format = source.0.double_format.borrow().clone();
        *receiver.0.string.borrow_mut() = string;
        receiver.0.string_storage.set(source.0.string_storage.get());
        *receiver.0.intrep.borrow_mut() = cache;
        *receiver.0.double_format.borrow_mut() = format;
        receiver
            .0
            .source_location
            .borrow_mut()
            .clone_from(&source.0.source_location.borrow());
    }

    fn string(
        &self,
        value: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Rc<[u8]>, tcl_syntax::value::ValueError> {
        value.native_string_bytes(protocol).map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native append string updater",
            )
        })
    }

    fn unicode(
        &self,
        value: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Rc<[u32]>, tcl_syntax::value::ValueError> {
        value.native_unicode_units(protocol)
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
        value: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        resident: Option<(Rc<[u8]>, NativeStringStorageIdentity)>,
        count: Option<usize>,
        unicode: Option<Rc<[u32]>>,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        value.native_append_string_using(protocol, resident, count, unicode, &mut None)
    }

    fn set_binary(
        &self,
        value: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        bytes: Rc<[u8]>,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        value.native_append_binary_using(protocol, bytes, &mut None)
    }

    fn new_string(&self, bytes: Rc<[u8]>) -> Value {
        Value::new_native_string_bytes(bytes)
    }
}

impl tcl_cmd_core::native_cat::NativeCatObjects for VmAppendObjects {
    fn empty_binary(
        &self,
        value: &Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Rc<[u8]>, tcl_syntax::value::ValueError> {
        let version = protocol.tcl_version().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "C compiled concat binary getter",
            ),
        )?;
        value
            .as_native_byte_array(
                tcl_native_model::native_binary_value::NativeBinaryByteConversion::Narrow(
                    version.string_character_model(),
                ),
                protocol,
            )
            .map_err(|_| {
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native compiled concat empty binary conversion",
                )
            })
    }
    fn is_shared(&self, value: &Value) -> bool {
        value.native_object_is_shared()
    }
    fn set_plain_string(
        &self,
        value: &Value,
        bytes: Rc<[u8]>,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        *value.0.string.borrow_mut() = Some(RawString::from_bytes(bytes));
        value
            .0
            .string_storage
            .set(NativeStringStorageIdentity::Allocated);
        *value.0.intrep.borrow_mut() = IntRep::Str;
        *value.0.double_format.borrow_mut() = None;
        *value.0.source_location.borrow_mut() = None;
        Ok(())
    }
}

impl Value {
    pub(crate) fn capture_native_binary_storage(
        &self,
    ) -> Result<NativeBinaryStorageReceipt, ValueError> {
        self.check_native_header()?;
        let original = self.0.intrep.borrow();
        let IntRep::ByteArray(backing) = &*original else {
            return Err(ValueError::CommandProtocolUnavailable(
                "original binary storage receipt",
            ));
        };
        Ok(NativeBinaryStorageReceipt {
            header: self.native_lifetime_lease(),
            backing: Rc::downgrade(backing),
        })
    }

    pub(crate) fn native_append_binary_checked<E: From<ValueError>>(
        &self,
        protocol: NativeStringProtocol,
        bytes: Rc<[u8]>,
        current: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        self.native_append_binary_using(protocol, bytes, &mut Some(current))
    }

    fn native_append_binary_using<E: From<ValueError>>(
        &self,
        protocol: NativeStringProtocol,
        bytes: Rc<[u8]>,
        current: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
    ) -> Result<(), E> {
        Self::native_stage_check(current)?;
        self.check_native_header()?;
        if protocol.tcl_version().is_none() {
            return Err(
                ValueError::CommandProtocolUnavailable("native append binary backing").into(),
            );
        }
        let origin = match &*self.0.intrep.borrow() {
            IntRep::ByteArray(rep) if rep.string_protocol.get() == Some(protocol) => rep.origin,
            _ => None,
        };
        let primary = IntRep::ByteArray(Rc::new(ByteArrayRep {
            bytes,
            string_protocol: Cell::new(Some(protocol)),
            origin,
            conversion: None,
            proper: true,
        }));
        self.replace_primary_using(primary, current)?;
        *self.0.string.borrow_mut() = None;
        self.0
            .string_storage
            .set(NativeStringStorageIdentity::Unknown);
        *self.0.double_format.borrow_mut() = None;
        *self.0.source_location.borrow_mut() = None;
        Self::native_stage_check(current)
    }

    pub(crate) fn native_append_string_checked<E: From<ValueError>>(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        resident: Option<(Rc<[u8]>, NativeStringStorageIdentity)>,
        count: Option<usize>,
        unicode: Option<Rc<[u32]>>,
        current: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        self.native_append_string_using(protocol, resident, count, unicode, &mut Some(current))
    }

    fn native_append_string_using<E: From<ValueError>>(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        resident: Option<(Rc<[u8]>, NativeStringStorageIdentity)>,
        count: Option<usize>,
        unicode: Option<Rc<[u32]>>,
        current: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
    ) -> Result<(), E> {
        Self::native_stage_check(current)?;
        if protocol.is_jim084() && (resident.is_none() || unicode.is_some()) {
            return Err(ValueError::CommandProtocolUnavailable("Jim append String backing").into());
        }
        let primary = if protocol.is_jim084() {
            IntRep::JimString(count)
        } else {
            IntRep::NativeString {
                protocol,
                num_chars: count,
                unicode,
            }
        };
        self.replace_primary_using(primary, current)?;
        *self.0.string.borrow_mut() = resident
            .as_ref()
            .map(|(bytes, _)| RawString::from_bytes(Rc::clone(bytes)));
        self.0
            .string_storage
            .set(resident.map_or(NativeStringStorageIdentity::Unknown, |(_, storage)| storage));
        *self.0.double_format.borrow_mut() = None;
        *self.0.source_location.borrow_mut() = None;
        Self::native_stage_check(current)
    }
}
