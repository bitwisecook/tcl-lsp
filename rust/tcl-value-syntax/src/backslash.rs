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

//! Backslash escapes in an already counted native string value.

use std::borrow::Cow;
use tcl_dialect::EscapeSyntax;

/// Unsupported source recipe or malformed host request, never a guest error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeSourceUnavailable {
    /// Source grammar and independently selected string recipe disagree.
    GrammarProtocolMismatch,
    /// The requested position is not a backslash or has no valid bounded extent.
    InvalidEscapePosition,
    /// A Unicode Document contains native bytes that have no Unicode source value.
    InvalidDocumentUnicode,
}

/// Native bytes for one scanner-selected source escape token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeSourceEscape {
    /// End of the original selected token, in the supplied source buffer.
    pub end: usize,
    /// Exact resulting native string units.
    pub bytes: Vec<u8>,
}

/// Decode one source escape through the shared lexer fragment scanner.
///
/// C source scanning selects a token extent before the evaluator decodes that
/// bounded token. The distinction matters for truncated C8.6 astral input:
/// scanning can select a high surrogate's one-byte extent, while decoding the
/// selected token yields that original lead byte's native character unit.
/// Literal bytes after the selected extent remain literal, independently of
/// what the `subst` command does with its remaining string representation.
///
/// # Errors
/// Returns an error for a grammar/recipe mismatch or an invalid requested escape extent.
pub fn native_source_escape_in(
    raw: &[u8],
    pos: usize,
    escapes: EscapeSyntax,
    protocol: crate::native_string::NativeStringProtocol,
) -> Result<NativeSourceEscape, NativeSourceUnavailable> {
    use tcl_lexer::{BackslashFragmentValue, SourceChannel, source_backslash_fragment_in};
    if escapes != protocol.escape_syntax() {
        return Err(NativeSourceUnavailable::GrammarProtocolMismatch);
    }
    let channel = SourceChannel::NativeValue;
    let scan = source_backslash_fragment_in(raw, pos, channel, escapes, |suffix| {
        native_original_escape_unit(protocol, suffix)
    })
    .ok_or(NativeSourceUnavailable::InvalidEscapePosition)?;
    let decoded = source_backslash_fragment_in(&raw[..scan.end], pos, channel, escapes, |suffix| {
        native_original_escape_unit(protocol, suffix)
    })
    .ok_or(NativeSourceUnavailable::InvalidEscapePosition)?;
    let mut bytes = Vec::new();
    match decoded.value {
        BackslashFragmentValue::Byte(byte) => bytes.push(byte),
        BackslashFragmentValue::Literal(range) => bytes.extend_from_slice(&raw[range]),
        BackslashFragmentValue::Codepoint(value) => match protocol {
            crate::native_string::NativeStringProtocol::Jim084 => {
                tcl_lexer::encode_jim084_unicode(&mut bytes, value);
            }
            crate::native_string::NativeStringProtocol::C(version) => {
                let value = if version < tcl_dialect::TclVersion::V9_0 && value > 0xffff {
                    0xfffd
                } else {
                    value
                };
                crate::native_tcl_utf::NativeTclUtf::for_version(version).encode_unit(value, &mut bytes)
                    .expect("shared numeric scanner and native input-unit decoder select a supported C unit");
            }
        },
    }
    Ok(NativeSourceEscape {
        end: scan.end,
        bytes,
    })
}

/// Encode escaped source content as native string bytes. This does not parse
/// word boundaries, perform substitutions, materialise byte arrays, decode
/// file channels or grant compilation permission. The caller supplies already
/// selected word content and retains its original source/object provenance.
/// Backslash-free content borrows the complete original byte string.
///
/// # Errors
/// Returns an error for a grammar/recipe mismatch or an invalid requested escape extent.
pub fn native_source_string_bytes_in(
    raw: &[u8],
    escapes: EscapeSyntax,
    protocol: crate::native_string::NativeStringProtocol,
) -> Result<Cow<'_, [u8]>, NativeSourceUnavailable> {
    if escapes != protocol.escape_syntax() {
        return Err(NativeSourceUnavailable::GrammarProtocolMismatch);
    }
    if !raw.contains(&b'\\') {
        return Ok(Cow::Borrowed(raw));
    }
    let mut output = Vec::with_capacity(raw.len());
    let mut pos = 0;
    while pos < raw.len() {
        if raw[pos] == b'\\' {
            let fragment = native_source_escape_in(raw, pos, escapes, protocol)?;
            output.extend_from_slice(&fragment.bytes);
            pos = fragment.end;
        } else {
            output.push(raw[pos]);
            pos += 1;
        }
    }
    Ok(Cow::Owned(output))
}

fn native_original_escape_unit(
    protocol: crate::native_string::NativeStringProtocol,
    suffix: &[u8],
) -> tcl_lexer::EscapedInputUnit {
    use tcl_lexer::{EscapedInputUnit, EscapedInputValue};
    let crate::native_string::NativeStringProtocol::C(version) = protocol else {
        return EscapedInputUnit {
            width: 1,
            value: EscapedInputValue::CopyOriginal,
        };
    };
    let units = crate::native_tcl_utf::NativeTclUtf::for_version(version);
    let first = units
        .decode_unit(suffix, None)
        .expect("unrecognised escape has an input byte");
    if version == tcl_dialect::TclVersion::V8_6
        && (0xd800..=0xdbff).contains(&first.value)
        && let Some(low) = units.decode_unit(&suffix[first.width..], Some(first.value))
        && (0xdc00..=0xdfff).contains(&low.value)
    {
        return EscapedInputUnit {
            width: first.width + low.width,
            value: EscapedInputValue::Codepoint(
                ((first.value & 0x3ff) << 10 | (low.value & 0x3ff)) + 0x1_0000,
            ),
        };
    }
    EscapedInputUnit {
        width: first.width,
        value: EscapedInputValue::Codepoint(first.value),
    }
}
