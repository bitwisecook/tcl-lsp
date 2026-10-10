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

//! Original C external UTF-8 conversion, independent of binary narrowing.
//!
//! The actual string getter owns input primary materialization. The selected
//! native UTF decoder supplies counted character units; the backend result
//! constructor independently authenticates binary storage. Explicit codepage,
//! conversion profile, fail-index and streaming support are separate capabilities.

use crate::CmdError;
use tcl_dialect::TclVersion;
use tcl_syntax::{
    naming::{NamePolicyAuthority, NativeNameProtocol},
    native_string::NativeStringProtocol,
    native_tcl_utf::NativeTclUtf,
    value::{ValueError, ValueOps},
};

/// Reached default-strict C9 UTF-8 output failure, retaining both coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Utf8EncodingError {
    /// Native character position used by the public diagnostic.
    pub character_index: usize,
    /// Counted original source-byte offset used by errorCode.
    pub byte_offset: usize,
    /// Decoded native unit rejected by the external UTF-8 target.
    pub unit: u32,
}

/// Convert the complete string produced by the selected C native getter.
/// This pure operation supplies no physical object or result-storage authority.
///
/// # Errors
/// Returns the exact distinct coordinates of a default-strict C9 surrogate
/// failure. C8.4/8.5 retain surrogate units; C8.6 combines adjacent pairs.
pub fn external_utf8_octets(
    version: TclVersion,
    source: &[u8],
) -> Result<Vec<u8>, Utf8EncodingError> {
    let units = NativeTclUtf::for_version(version);
    let mut result = Vec::with_capacity(source.len());
    let mut byte_offset = 0;
    let mut character_index = 0;
    let mut previous = None;
    while let Some(unit) = units.decode_unit(&source[byte_offset..], previous) {
        if version >= TclVersion::V9_0 && (0xd800..=0xdfff).contains(&unit.value) {
            return Err(Utf8EncodingError {
                character_index,
                byte_offset,
                unit: unit.value,
            });
        }
        if version == TclVersion::V8_6
            && (0xd800..=0xdbff).contains(&unit.value)
            && let Some(low) =
                units.decode_unit(&source[byte_offset + unit.width..], Some(unit.value))
            && (0xdc00..=0xdfff).contains(&low.value)
        {
            let scalar = 0x10000 + ((unit.value - 0xd800) << 10) + low.value - 0xdc00;
            let scalar = char::from_u32(scalar).expect("bounded surrogate pair is a scalar");
            let mut encoded = [0; 4];
            result.extend_from_slice(scalar.encode_utf8(&mut encoded).as_bytes());
            byte_offset += unit.width + low.width;
            character_index += 2;
            previous = Some(low.value);
            continue;
        }
        if unit.value == 0 {
            // All five C targets emit literal external zero, including an
            // original internal modified-zero sequence or resident raw zero.
            result.push(0);
        } else {
            units
                .encode_unit(unit.value, &mut result)
                .expect("the selected native decoder returns encodable units");
        }
        byte_offset += unit.width;
        character_index += 1;
        previous = Some(unit.value);
    }
    Ok(result)
}

/// Convert an original value through its actual C string getter to proper
/// external UTF-8 binary storage. A native name/string recipe supplies no
/// backend binary constructor authority by itself.
///
/// # Errors
/// Refuses unavailable native getter/result owners, Jim's unmeasured encoding
/// extension, or a purely authored simulation. C9 default-strict failures retain
/// structured coordinates, errorCode and the selected printf String producer.
pub fn convert_to_utf8<O: ValueOps>(
    ops: &mut O,
    original: &O::Value,
) -> Result<O::Value, CmdError> {
    let policy = ops
        .name_policy_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "original external UTF-8 conversion",
        ))?;
    let NativeNameProtocol::C(version) = policy.recipe() else {
        return Err(ValueError::CommandProtocolUnavailable("C external UTF-8 conversion").into());
    };
    if policy.authority() != NamePolicyAuthority::Native {
        return Err(ValueError::CommandProtocolUnavailable("native external UTF-8 issuer").into());
    }
    let source = ops.native_string_bytes(original)?;
    let external = external_utf8_octets(version, &source).map_err(|failure| {
        // naming.encoding.original-utf8-convertto-error-offset
        // docs/design/analysis/name-resolution-proofs/encoding-original-utf8-convertto-error-offset.md
        failure.command_error(version)
    })?;
    ops.native_external_utf8_result(&external, version)
        .map_err(Into::into)
}

impl Utf8EncodingError {
    fn command_error(self, version: TclVersion) -> CmdError {
        let message = format!("unexpected character at index {}: 'U+{:06X}'", self.character_index, self.unit);
        let code = format!("TCL ENCODING ILLEGALSEQUENCE {}", self.byte_offset);
        CmdError::with_error_code_bytes(message.into_bytes(), code.into_bytes())
            .with_native_string_result(NativeStringProtocol::C(version))
    }
}

#[cfg(test)]
mod native_controls;
