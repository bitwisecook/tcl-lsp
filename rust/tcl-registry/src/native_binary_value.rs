// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Selected binary object conversion and decoder-source protocols.

/// Actual scripted Binary ingress, independently of the C ensemble protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeBinaryScriptedIngress {
    /// Written root registration name.
    pub name: &'static str,
    /// Native procedure formal-parameter source.
    pub parameters: &'static str,
    /// Original scripted wrapper, retaining exact compound lookup and tailcall.
    pub body: &'static str,
    /// Actual compound handlers supplied by the selected extension.
    pub members: &'static [&'static str],
}

/// Conversion of an actual string value into binary operand bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeBinaryByteConversion {
    /// Legacy C byte narrowing follows the selected native character units.
    Narrow(tcl_dialect::StringCharacterModel),
    /// Tcl9 proper byte arrays reject code points above U+00FF.
    CheckedLatin1,
    /// The pinned Jim Binary extension consumes its actual UTF8 string bytes.
    Utf8,
}

/// Failure of proper byte-array conversion, independently of decoder grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeBinaryByteError {
    /// Index of the output byte whose source code point cannot fit.
    pub offset: usize,
    /// Actual original Unicode code point.
    pub codepoint: u32,
}

impl NativeBinaryByteError {
    /// Exact Tcl9 byte-conversion diagnostic; errorCode is TCL VALUE BYTES.
    #[must_use]
    pub fn message(self) -> String {
        format!(
            "expected code point values below 0xff but value at byte offset {} was 0x{:x}",
            self.offset, self.codepoint
        )
    }
}

impl NativeBinaryByteConversion {
    /// Convert native C string units under an independently selected string recipe.
    /// Literal NUL and modified UTF-8 NUL denote the same zero-valued unit.
    /// This pure conversion grants no object cache or engine authority.
    pub fn convert_native(
        self,
        value: &[u8],
        units: tcl_syntax::native_tcl_utf::NativeTclUtf,
    ) -> Result<Vec<u8>, NativeBinaryByteError> {
        if self == Self::Utf8 {
            return Ok(value.to_vec());
        }
        let mut output = Vec::new();
        let mut offset = 0;
        let mut previous = None;
        while offset < value.len() {
            let unit = units
                .decode_unit(&value[offset..], previous)
                .expect("a remaining byte supplies a native unit");
            if self == Self::CheckedLatin1 && unit.value > 255 {
                return Err(NativeBinaryByteError {
                    offset: output.len(),
                    codepoint: unit.value,
                });
            }
            output.push(unit.value.to_le_bytes()[0]);
            offset += unit.width;
            previous = Some(unit.value);
        }
        Ok(output)
    }

    /// Convert original string contents, without choosing an object cache or
    /// overwriting a shared object's existing representation on failure.
    pub fn convert(self, value: &str) -> Result<Vec<u8>, NativeBinaryByteError> {
        use tcl_dialect::StringCharacterModel;
        if self == Self::Utf8 {
            return Ok(value.as_bytes().to_vec());
        }
        let mut bytes = Vec::new();
        if self == Self::Narrow(StringCharacterModel::Utf16CodeUnits) {
            bytes.extend(value.encode_utf16().map(|unit| unit.to_le_bytes()[0]));
            return Ok(bytes);
        }
        for character in value.chars() {
            let codepoint = u32::from(character);
            if self == Self::CheckedLatin1 && codepoint > 255 {
                return Err(NativeBinaryByteError {
                    offset: bytes.len(),
                    codepoint,
                });
            }
            if self == Self::Narrow(StringCharacterModel::BmpCharsElseUtf8Bytes)
                && codepoint > 0xffff
            {
                let mut encoded = [0; 4];
                bytes.extend_from_slice(character.encode_utf8(&mut encoded).as_bytes());
            } else {
                bytes.push(codepoint.to_le_bytes()[0]);
            }
        }
        Ok(bytes)
    }
}

/// Which actual object representation supplies native decoder bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeBinaryDecodeSource {
    /// C8.6 uses only a pure byte-array payload directly; otherwise UTF8.
    PureByteArrayOrString,
    /// C9 first attempts proper byte-array conversion, then uses UTF8 on failure.
    ProperByteArrayOrString,
}

/// Typed data bytes retain their original decoder-source interpretation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeBinaryDecodeInput {
    /// A real byte-array payload: each offending byte denotes its own character.
    Bytes(Vec<u8>),
    /// An original Unicode string, whose UTF8 byte offsets are authoritative.
    String(String),
}

impl NativeBinaryDecodeInput {
    /// Bytes passed to the selected native decoder.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        match self {
            Self::Bytes(bytes) => bytes,
            Self::String(text) => text.as_bytes(),
        }
    }
    /// Actual offending character at a decoder's retained byte offset.
    #[must_use]
    pub fn character_at(&self, offset: usize) -> Option<char> {
        match self {
            Self::Bytes(bytes) => bytes.get(offset).copied().map(char::from),
            Self::String(text) => text.get(offset..)?.chars().next(),
        }
    }
}

impl crate::InvocationDialect {
    /// The pinned Jim Binary library's real wrapper. Unknown profiles abstain.
    #[must_use]
    pub fn binary_scripted_ingress(self) -> Option<NativeBinaryScriptedIngress> {
        self.native_string_protocol()
            .filter(|protocol| protocol.is_jim084())
            .map(|_| NativeBinaryScriptedIngress {
                name: "binary",
                parameters: "cmd args",
                body: "tailcall \"binary $cmd\" {*}$args",
                members: &["binary format", "binary scan"],
            })
    }

    /// Actual encode/scan byte conversion. Unknown runtime policy abstains.
    #[must_use]
    pub fn binary_data_conversion(self) -> Option<NativeBinaryByteConversion> {
        match self.native_string_protocol()? {
            tcl_syntax::native_string::NativeStringProtocol::C(version) => {
                Some(if version >= tcl_dialect::TclVersion::V9_0 {
                    NativeBinaryByteConversion::CheckedLatin1
                } else {
                    NativeBinaryByteConversion::Narrow(version.string_character_model())
                })
            }
            tcl_syntax::native_string::NativeStringProtocol::Jim084 => {
                Some(NativeBinaryByteConversion::Utf8)
            }
        }
    }

    /// Native format narrowing is independent of checked encode/scan conversion.
    /// Tcl9 narrows a copy instead of rejecting a wide source character.
    #[must_use]
    pub fn binary_format_conversion(self) -> Option<NativeBinaryByteConversion> {
        match self.native_string_protocol()? {
            tcl_syntax::native_string::NativeStringProtocol::C(version) => Some(
                NativeBinaryByteConversion::Narrow(version.string_character_model()),
            ),
            tcl_syntax::native_string::NativeStringProtocol::Jim084 => {
                Some(NativeBinaryByteConversion::Utf8)
            }
        }
    }

    /// The selected modern C decoder protocol; absence proves no availability.
    #[must_use]
    pub fn binary_decode_source(self) -> Option<NativeBinaryDecodeSource> {
        let version = self.native_string_protocol()?.tcl_version()?;
        if version < tcl_dialect::TclVersion::V8_6 {
            return None;
        }
        Some(if version >= tcl_dialect::TclVersion::V9_0 {
            NativeBinaryDecodeSource::ProperByteArrayOrString
        } else {
            NativeBinaryDecodeSource::PureByteArrayOrString
        })
    }

    /// Native decoder diagnostics from actual input provenance and byte position.
    #[must_use]
    pub fn binary_decode_error(
        self,
        what: &str,
        input: &NativeBinaryDecodeInput,
        offset: usize,
    ) -> Option<String> {
        let character = input.character_at(offset)?;
        let point =
            if self.native_string_protocol()?.tcl_version()? >= tcl_dialect::TclVersion::V9_0 {
                format!(" (U+{:06X})", u32::from(character))
            } else {
                String::new()
            };
        self.binary_decode_source()?;
        Some(format!(
            "invalid {what} \"{character}\"{point} at position {offset}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_string_nul_and_latin1_units_roundtrip_without_unicode_repair() {
        for version in tcl_dialect::TclVersion::ALL {
            let units = tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version);
            let conversion = if version >= tcl_dialect::TclVersion::V9_0 {
                NativeBinaryByteConversion::CheckedLatin1
            } else {
                NativeBinaryByteConversion::Narrow(
                    tcl_dialect::StringCharacterModel::Utf16CodeUnits,
                )
            };
            assert_eq!(
                conversion
                    .convert_native(b"1\xc0\x80\xc3\xbf", units)
                    .unwrap(),
                [b'1', 0, 255]
            );
            assert_eq!(
                conversion.convert_native(b"1\0\xff", units).unwrap(),
                [b'1', 0, 255]
            );
        }
    }

    #[test]
    fn selected_binary_conversion_preserves_unicode_and_release_boundaries() {
        use tcl_dialect::TclVersion;
        let legacy = crate::InvocationDialect::for_version(TclVersion::V8_6);
        let modern = crate::InvocationDialect::for_version(TclVersion::V9_0);
        assert_eq!(
            legacy
                .binary_data_conversion()
                .unwrap()
                .convert("é€")
                .unwrap(),
            [0xe9, 0xac]
        );
        assert_eq!(
            modern
                .binary_format_conversion()
                .unwrap()
                .convert("é€")
                .unwrap(),
            [0xe9, 0xac]
        );
        assert_eq!(
            modern.binary_data_conversion().unwrap().convert("é€"),
            Err(NativeBinaryByteError {
                offset: 1,
                codepoint: 0x20ac
            })
        );
        let jim = crate::InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").analyser_profile(),
        );
        assert_eq!(
            jim.binary_data_conversion().unwrap().convert("é€").unwrap(),
            "é€".as_bytes()
        );
        assert!(jim.binary_decode_source().is_none());
        assert_eq!(
            jim.binary_scripted_ingress().unwrap().body,
            "tailcall \"binary $cmd\" {*}$args"
        );
        assert!(legacy.binary_scripted_ingress().is_none());
        assert!(
            crate::InvocationDialect::for_version(TclVersion::V8_5)
                .binary_decode_source()
                .is_none()
        );
    }

    #[test]
    fn decoder_diagnostics_retain_actual_unicode_or_byte_payload() {
        use tcl_dialect::TclVersion;
        let text = NativeBinaryDecodeInput::String("00€".to_owned());
        let bytes = NativeBinaryDecodeInput::Bytes(vec![b'0', b'0', 0xe9]);
        let legacy = crate::InvocationDialect::for_version(TclVersion::V8_6);
        let modern = crate::InvocationDialect::for_version(TclVersion::V9_0);
        assert_eq!(
            legacy
                .binary_decode_error("hexadecimal digit", &text, 2)
                .unwrap(),
            "invalid hexadecimal digit \"€\" at position 2"
        );
        assert_eq!(
            modern
                .binary_decode_error("hexadecimal digit", &bytes, 2)
                .unwrap(),
            "invalid hexadecimal digit \"é\" (U+0000E9) at position 2"
        );
        assert!(
            legacy
                .binary_decode_error("hexadecimal digit", &text, 3)
                .is_none()
        );
    }
}
