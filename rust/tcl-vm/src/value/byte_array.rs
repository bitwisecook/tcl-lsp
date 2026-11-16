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

//! The byte-array representation: construction, inspection, string-to-bytes conversion and binary decoder input.

use super::{ByteArrayAccessError, ByteArrayRep, IntRep, Value};
use std::cell::Cell;
use std::rc::Rc;
use tcl_syntax::native_string::NativeStringProtocol;
use tcl_syntax::raw_string::{RawString, UnicodeAccessError};
use tcl_syntax::value::ValueError;

impl Value {
    /// An actual binary object with a byte payload and no generated string.
    #[must_use]
    pub fn byte_array(bytes: impl Into<Rc<[u8]>>) -> Self {
        Self::byte_array_with_resident_string(bytes, None)
    }

    /// Retain binary storage together with an existing native string cache.
    /// Inspecting or transferring this value does not generate a missing cache.
    #[must_use]
    pub fn byte_array_with_resident_string(
        bytes: impl Into<Rc<[u8]>>,
        resident: Option<Rc<[u8]>>,
    ) -> Self {
        Self::from_raw_parts(
            resident.map(RawString::from_bytes),
            IntRep::ByteArray(Rc::new(ByteArrayRep {
                bytes: bytes.into(),
                string_protocol: Cell::new(None),
                origin: None,
                conversion: None,
                proper: true,
            })),
        )
    }

    /// Inspect the binary payload without converting or generating a string.
    #[must_use]
    pub fn byte_array_representation(&self) -> Option<Rc<[u8]>> {
        match &*self.0.intrep.borrow() {
            IntRep::ByteArray(bytes) => Some(Rc::clone(&bytes.bytes)),
            _ => None,
        }
    }

    /// Inspect the sealed constructor receipt without materialising a string.
    /// A missing receipt identifies imported backing or a conversion cache;
    /// it does not authenticate a native or authored producer.
    #[must_use]
    pub fn byte_array_origin(
        &self,
    ) -> Option<tcl_registry::native_string_materialization::ByteArrayStringRecipe> {
        match &*self.0.intrep.borrow() {
            IntRep::ByteArray(bytes) => bytes.origin,
            _ => None,
        }
    }

    /// Actual native updater protocol of the original binary backing.
    /// This metadata grants no command or String getter authority.
    pub(crate) fn native_byte_array_protocol(&self) -> Option<NativeStringProtocol> {
        match &*self.0.intrep.borrow() {
            IntRep::ByteArray(bytes) => bytes.string_protocol.get(),
            _ => None,
        }
    }

    /// Whether the native pure-byte-array decoder door may use this payload.
    #[must_use]
    pub fn is_pure_byte_array(&self) -> bool {
        self.0.string.borrow().is_none() && self.byte_array_representation().is_some()
    }

    /// Selected native string-to-byte conversion, preserving original string
    /// bytes before replacing the internal representation. Failure retains it.
    pub fn as_byte_array(
        &self,
        conversion: tcl_registry::native_binary_value::NativeBinaryByteConversion,
    ) -> Result<Rc<[u8]>, ByteArrayAccessError> {
        use tcl_registry::native_binary_value::NativeBinaryByteConversion;
        if conversion == NativeBinaryByteConversion::Utf8 {
            // Jim's binary commands read the string bytes; they do not install
            // a C Tcl byte-array representation or withdraw a cached count.
            return Ok(self.string_bytes());
        }
        if let IntRep::ByteArray(rep) = &*self.0.intrep.borrow()
            && (rep.conversion.is_none() || rep.conversion == Some(conversion))
            && (rep.proper || conversion != NativeBinaryByteConversion::CheckedLatin1)
        {
            return Ok(Rc::clone(&rep.bytes));
        }
        let text = self.try_to_str().map_err(ByteArrayAccessError::Unicode)?;
        let bytes: Rc<[u8]> = Rc::from(
            conversion
                .convert(&text)
                .map_err(ByteArrayAccessError::Conversion)?,
        );
        let proper = text.chars().all(|character| u32::from(character) <= 255);
        *self.0.intrep.borrow_mut() = IntRep::ByteArray(Rc::new(ByteArrayRep {
            bytes: Rc::clone(&bytes),
            string_protocol: Cell::new(None),
            origin: None,
            conversion: Some(conversion),
            proper,
        }));
        Ok(bytes)
    }

    /// Convert the original object's actual native string units into bytes.
    /// Cache adoption follows successful conversion; a proper-byte rejection
    /// preserves its original cache and resident string.
    ///
    /// # Errors
    /// Returns a guest byte-conversion error or unavailable native storage.
    pub fn as_native_byte_array(
        &self,
        conversion: tcl_registry::native_binary_value::NativeBinaryByteConversion,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Rc<[u8]>, ByteArrayAccessError> {
        self.as_native_byte_array_using::<ValueError>(conversion, protocol, &mut None, &mut || {
            Ok(self
                .native_string_bytes(protocol)
                .map_err(ByteArrayAccessError::Unavailable))
        })
        .expect("neutral native binary conversion has no checked stages")
    }

    pub(crate) fn as_native_byte_array_checked(
        &self,
        conversion: tcl_registry::native_binary_value::NativeBinaryByteConversion,
        protocol: NativeStringProtocol,
        current: &mut dyn FnMut() -> Result<(), tcl_cmd_core::CmdError>,
        original_string: &mut dyn FnMut() -> Result<Rc<[u8]>, tcl_cmd_core::CmdError>,
    ) -> Result<Rc<[u8]>, tcl_cmd_core::CmdError> {
        self.as_native_byte_array_checked_result(conversion, protocol, current, original_string)?
            .map_err(Into::into)
    }

    /// Preserve the selected proper-byte refusal for the format worker's
    /// existing copy-narrowing fallback, independently of checked stage errors.
    pub(crate) fn as_native_byte_array_checked_result(
        &self,
        conversion: tcl_registry::native_binary_value::NativeBinaryByteConversion,
        protocol: NativeStringProtocol,
        current: &mut dyn FnMut() -> Result<(), tcl_cmd_core::CmdError>,
        original_string: &mut dyn FnMut() -> Result<Rc<[u8]>, tcl_cmd_core::CmdError>,
    ) -> Result<Result<Rc<[u8]>, ByteArrayAccessError>, tcl_cmd_core::CmdError> {
        self.as_native_byte_array_using(conversion, protocol, &mut Some(current), &mut || {
            original_string().map(Ok)
        })
    }

    fn as_native_byte_array_using<E: From<ValueError>>(
        &self,
        conversion: tcl_registry::native_binary_value::NativeBinaryByteConversion,
        protocol: NativeStringProtocol,
        current: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        original_string: &mut dyn FnMut() -> Result<Result<Rc<[u8]>, ByteArrayAccessError>, E>,
    ) -> Result<Result<Rc<[u8]>, ByteArrayAccessError>, E> {
        use tcl_registry::native_binary_value::NativeBinaryByteConversion;
        Self::native_stage_check(current)?;
        if conversion == NativeBinaryByteConversion::Utf8 {
            return original_string();
        }
        let Some(version) = protocol.tcl_version() else {
            return Ok(Err(ByteArrayAccessError::Unavailable(
                tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable,
            )));
        };
        if current.is_some()
            && let IntRep::ByteArray(rep) = &*self.0.intrep.borrow()
            && rep.string_protocol.get() != Some(protocol)
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "original binary getter cache issuer",
            )
            .into());
        }
        if let IntRep::ByteArray(rep) = &*self.0.intrep.borrow()
            && (rep.conversion.is_none() || rep.conversion == Some(conversion))
            && (rep.proper || conversion != NativeBinaryByteConversion::CheckedLatin1)
        {
            return Ok(Ok(Rc::clone(&rep.bytes)));
        }
        let reached = original_string();
        Self::native_stage_check(current)?;
        let original = match reached? {
            Ok(bytes) => bytes,
            Err(error) => return Ok(Err(error)),
        };
        let units = tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version);
        let bytes: Rc<[u8]> = match conversion.convert_native(&original, units) {
            Ok(bytes) => Rc::from(bytes),
            Err(error) => return Ok(Err(ByteArrayAccessError::Conversion(error))),
        };
        let proper = NativeBinaryByteConversion::CheckedLatin1
            .convert_native(&original, units)
            .is_ok();
        let primary = IntRep::ByteArray(Rc::new(ByteArrayRep {
            bytes: Rc::clone(&bytes),
            string_protocol: Cell::new(Some(protocol)),
            origin: None,
            conversion: Some(conversion),
            proper,
        }));
        if current.is_some() {
            self.replace_primary_using(primary, current)?;
        } else {
            *self.0.intrep.borrow_mut() = primary;
        }
        Self::native_stage_check(current)?;
        Ok(Ok(bytes))
    }

    /// Select decoder bytes from the actual object/cache protocol; UTF8
    /// fallback retains its source interpretation for Unicode diagnostics.
    pub fn binary_decode_input(
        &self,
        policy: tcl_registry::native_binary_value::NativeBinaryDecodeSource,
    ) -> Result<tcl_registry::native_binary_value::NativeBinaryDecodeInput, UnicodeAccessError>
    {
        use tcl_registry::native_binary_value::{
            NativeBinaryByteConversion, NativeBinaryDecodeInput, NativeBinaryDecodeSource,
        };
        if policy == NativeBinaryDecodeSource::PureByteArrayOrString && self.is_pure_byte_array() {
            return Ok(NativeBinaryDecodeInput::Bytes(
                self.byte_array_representation()
                    .expect("pure byte array")
                    .to_vec(),
            ));
        }
        if policy == NativeBinaryDecodeSource::ProperByteArrayOrString {
            match self.as_byte_array(NativeBinaryByteConversion::CheckedLatin1) {
                Ok(bytes) => return Ok(NativeBinaryDecodeInput::Bytes(bytes.to_vec())),
                Err(ByteArrayAccessError::Unicode(error)) => return Err(error),
                Err(ByteArrayAccessError::Conversion(_)) => {}
                Err(ByteArrayAccessError::Unavailable(_)) => {
                    unreachable!("Unicode-only byte conversion has no native recipe branch")
                }
            }
        }
        Ok(NativeBinaryDecodeInput::String(
            self.try_to_str()?.to_string(),
        ))
    }

    /// Construct proper binary backing with its native updater recipe sealed
    /// before allocation. Imported untyped bytes are a separate carrier.
    pub fn from_native_byte_array(
        bytes: Rc<[u8]>,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        let recipe = dialect.byte_array_string_recipe(None).ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native byte-array constructor",
            ),
        )?;
        Ok(Self::from_byte_array_recipe(bytes, recipe))
    }

    /// Construct proper binary backing from its selected, sealed producer recipe.
    /// The recipe retains an authored logical origin independently of its C updater.
    /// It is data provenance; consumers require their own actual native authority.
    #[must_use]
    pub fn from_byte_array_recipe(
        bytes: Rc<[u8]>,
        recipe: tcl_registry::native_string_materialization::ByteArrayStringRecipe,
    ) -> Self {
        Self::from_raw_parts(
            None,
            IntRep::ByteArray(Rc::new(ByteArrayRep {
                bytes,
                string_protocol: Cell::new(Some(recipe.protocol())),
                origin: Some(recipe),
                conversion: None,
                proper: true,
            })),
        )
    }
}
