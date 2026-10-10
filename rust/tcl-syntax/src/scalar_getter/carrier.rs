// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Lossless codecs between neutral payload carriers and selected scalar caches.
//! These functions transport retained facts. The receiving engine separately
//! validates descriptor origin, dialect, storage and original-object authority
//! before adopting a cache. Conversion supplies none of those capabilities.

use super::NativeScalarCache;
use crate::native_string::NativeStringStorageIdentity as Storage;
use crate::number::{Number, Radix};
use std::rc::Rc;
use tcl_core_types::{
    NativeCVersion, NativeIntegerRadix, NativeScalarCache as Carrier,
    NativeStringStorageIdentity as CarrierStorage,
};
use tcl_dialect::TclVersion;

/// A fact required to retain the original scalar descriptor is unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeScalarCarrierError {
    /// A word Boolean has no independently retained C descriptor release.
    WordBooleanOriginUnavailable,
    /// Recorded allocation kind is unavailable.
    ResidentStorageUnavailable,
    /// Canonical empty allocation was paired with non-empty bytes.
    CanonicalEmptyInconsistent,
}
impl std::fmt::Display for NativeScalarCarrierError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WordBooleanOriginUnavailable => {
                formatter.write_str("native word-Boolean descriptor origin is unavailable")
            }
            Self::ResidentStorageUnavailable => {
                formatter.write_str("native resident storage identity is unavailable")
            }
            Self::CanonicalEmptyInconsistent => {
                formatter.write_str("canonical empty storage has non-empty bytes")
            }
        }
    }
}
impl std::error::Error for NativeScalarCarrierError {}

/// Decode an independently retained C release, without guessing an execution profile.
#[must_use]
pub const fn import_version(version: NativeCVersion) -> TclVersion {
    match version {
        NativeCVersion::V8_4 => TclVersion::V8_4,
        NativeCVersion::V8_5 => TclVersion::V8_5,
        NativeCVersion::V8_6 => TclVersion::V8_6,
        NativeCVersion::V9_0 => TclVersion::V9_0,
        NativeCVersion::V9_1 => TclVersion::V9_1,
    }
}
/// Encode the actual recorded descriptor release.
#[must_use]
pub const fn export_version(version: TclVersion) -> NativeCVersion {
    match version {
        TclVersion::V8_4 => NativeCVersion::V8_4,
        TclVersion::V8_5 => NativeCVersion::V8_5,
        TclVersion::V8_6 => NativeCVersion::V8_6,
        TclVersion::V9_0 => NativeCVersion::V9_0,
        TclVersion::V9_1 => NativeCVersion::V9_1,
    }
}
/// Decode recorded allocation identity without inferring it from byte length.
#[must_use]
pub const fn import_storage(storage: CarrierStorage) -> Storage {
    match storage {
        CarrierStorage::CanonicalEmpty => Storage::CanonicalEmpty,
        CarrierStorage::Allocated => Storage::Allocated,
        CarrierStorage::Unknown => Storage::Unknown,
    }
}
/// Validate recorded allocation data before importing its independent payload.
///
/// # Errors
/// Refuses unknown allocation identity and non-empty canonical empty storage.
pub fn checked_storage(
    storage: CarrierStorage,
    length: usize,
) -> Result<Storage, NativeScalarCarrierError> {
    match storage {
        CarrierStorage::Unknown => Err(NativeScalarCarrierError::ResidentStorageUnavailable),
        CarrierStorage::CanonicalEmpty if length != 0 => {
            Err(NativeScalarCarrierError::CanonicalEmptyInconsistent)
        }
        _ => Ok(import_storage(storage)),
    }
}

/// Encode recorded allocation identity without creating live allocation authority.
#[must_use]
pub const fn export_storage(storage: Storage) -> CarrierStorage {
    match storage {
        Storage::CanonicalEmpty => CarrierStorage::CanonicalEmpty,
        Storage::Allocated => CarrierStorage::Allocated,
        Storage::Unknown => CarrierStorage::Unknown,
    }
}

/// Decode the full retained cache and its independent word-Boolean origin.
/// The receiver must validate that origin before installing a native descriptor.
#[must_use]
pub fn import_scalar(cache: &Carrier) -> (NativeScalarCache, Option<TclVersion>) {
    let number = match cache {
        Carrier::Integer(value) => Number::Int(*value),
        Carrier::Double(value) => Number::Double(*value),
        Carrier::Nan { negative, payload } => Number::Nan {
            negative: *negative,
            payload: *payload,
        },
        Carrier::BigInteger {
            negative,
            radix,
            digits,
        } => Number::Big {
            negative: *negative,
            radix: match radix {
                NativeIntegerRadix::Binary => Radix::Bin,
                NativeIntegerRadix::Octal => Radix::Oct,
                NativeIntegerRadix::Decimal => Radix::Dec,
                NativeIntegerRadix::Hexadecimal => Radix::Hex,
            },
            digits: digits.to_string(),
        },
        Carrier::WordBoolean { value, origin } => {
            return (
                NativeScalarCache::WordBoolean(*value),
                Some(import_version(*origin)),
            );
        }
        Carrier::JimCoercedInteger(value) => {
            return (NativeScalarCache::JimCoercedInteger(*value), None);
        }
        Carrier::Tcl84Long(value) => return (NativeScalarCache::Tcl84Long(*value), None),
    };
    (NativeScalarCache::Number(number), None)
}

/// Encode an actual full cache; narrowing getter results never replace its magnitude.
///
/// # Errors
/// Refuses a word Boolean whose original descriptor release is unavailable.
pub fn export_scalar(
    cache: NativeScalarCache,
    origin: Option<TclVersion>,
) -> Result<Carrier, NativeScalarCarrierError> {
    Ok(match cache {
        NativeScalarCache::Number(Number::Int(value)) => Carrier::Integer(value),
        NativeScalarCache::Number(Number::Double(value)) => Carrier::Double(value),
        NativeScalarCache::Number(Number::Nan { negative, payload }) => {
            Carrier::Nan { negative, payload }
        }
        NativeScalarCache::Number(Number::Big {
            negative,
            radix,
            digits,
        }) => Carrier::BigInteger {
            negative,
            radix: match radix {
                Radix::Bin => NativeIntegerRadix::Binary,
                Radix::Oct => NativeIntegerRadix::Octal,
                Radix::Dec => NativeIntegerRadix::Decimal,
                Radix::Hex => NativeIntegerRadix::Hexadecimal,
            },
            digits: Rc::from(digits),
        },
        NativeScalarCache::WordBoolean(value) => Carrier::WordBoolean {
            value,
            origin: export_version(
                origin.ok_or(NativeScalarCarrierError::WordBooleanOriginUnavailable)?,
            ),
        },
        NativeScalarCache::JimCoercedInteger(value) => Carrier::JimCoercedInteger(value),
        NativeScalarCache::Tcl84Long(value) => Carrier::Tcl84Long(value),
    })
}

/// Malformed counted scalar transport. No native getter or cache is inferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarWireError {
    /// An unrecognised carrier variant or descriptor value.
    InvalidTag,
    /// A field extends beyond the supplied counted buffer.
    Truncated,
    /// Bytes remain after one complete payload.
    TrailingBytes,
    /// Big integer digit storage is not Unicode.
    DigitsNotUnicode,
    /// A counted field cannot fit in the wire length.
    InputTooLong,
}
impl std::fmt::Display for ScalarWireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidTag => "invalid scalar carrier tag",
            Self::Truncated => "truncated scalar carrier",
            Self::TrailingBytes => "trailing scalar carrier bytes",
            Self::DigitsNotUnicode => "non-Unicode scalar digit storage",
            Self::InputTooLong => "scalar digit storage exceeds counted transport",
        })
    }
}
impl std::error::Error for ScalarWireError {}

/// Encode full retained facts for a counted transport; supplies no getter authority.
///
/// # Errors
/// Refuses a digit field that exceeds its explicit 32-bit count.
pub fn encode_scalar(cache: &Carrier) -> Result<Vec<u8>, ScalarWireError> {
    let mut out = Vec::new();
    match cache {
        Carrier::Tcl84Long(value) | Carrier::Integer(value) | Carrier::JimCoercedInteger(value) => {
            out.push(match cache {
                Carrier::Tcl84Long(_) => 0,
                Carrier::Integer(_) => 1,
                _ => 6,
            });
            out.extend_from_slice(&value.to_le_bytes());
        }
        Carrier::BigInteger {
            negative,
            radix,
            digits,
        } => {
            out.extend_from_slice(&[
                2,
                u8::from(*negative),
                match radix {
                    NativeIntegerRadix::Binary => 0,
                    NativeIntegerRadix::Octal => 1,
                    NativeIntegerRadix::Decimal => 2,
                    NativeIntegerRadix::Hexadecimal => 3,
                },
            ]);
            let length = u32::try_from(digits.len()).map_err(|_| ScalarWireError::InputTooLong)?;
            out.extend_from_slice(&length.to_le_bytes());
            out.extend_from_slice(digits.as_bytes());
        }
        Carrier::Double(value) => {
            out.push(3);
            out.extend_from_slice(&value.to_bits().to_le_bytes());
        }
        Carrier::Nan { negative, payload } => {
            out.extend_from_slice(&[4, u8::from(*negative), u8::from(payload.is_some())]);
            if let Some(payload) = payload {
                out.extend_from_slice(&payload.to_le_bytes());
            }
        }
        Carrier::WordBoolean { value, origin } => out.extend_from_slice(&[
            5,
            u8::from(*value),
            match origin {
                NativeCVersion::V8_4 => 0,
                NativeCVersion::V8_5 => 1,
                NativeCVersion::V8_6 => 2,
                NativeCVersion::V9_0 => 3,
                NativeCVersion::V9_1 => 4,
            },
        ]),
    }
    Ok(out)
}

struct WireReader<'a>(&'a [u8]);
impl<'a> WireReader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], ScalarWireError> {
        if count > self.0.len() {
            return Err(ScalarWireError::Truncated);
        }
        let (taken, rest) = self.0.split_at(count);
        self.0 = rest;
        Ok(taken)
    }
    fn byte(&mut self) -> Result<u8, ScalarWireError> {
        Ok(self.take(1)?[0])
    }
    fn boolean(&mut self) -> Result<bool, ScalarWireError> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(ScalarWireError::InvalidTag),
        }
    }
    fn number(&mut self) -> Result<u64, ScalarWireError> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().expect("eight counted bytes"),
        ))
    }
}

/// Decode complete retained facts, without parsing a scalar or creating a cache.
///
/// # Errors
/// Refuses malformed fields, incomplete buffers and extra payload bytes.
pub fn decode_scalar(bytes: &[u8]) -> Result<Carrier, ScalarWireError> {
    let mut input = WireReader(bytes);
    let value = match input.byte()? {
        0 => Carrier::Tcl84Long(input.number()?.cast_signed()),
        1 => Carrier::Integer(input.number()?.cast_signed()),
        2 => {
            let negative = input.boolean()?;
            let radix = match input.byte()? {
                0 => NativeIntegerRadix::Binary,
                1 => NativeIntegerRadix::Octal,
                2 => NativeIntegerRadix::Decimal,
                3 => NativeIntegerRadix::Hexadecimal,
                _ => return Err(ScalarWireError::InvalidTag),
            };
            let length =
                u32::from_le_bytes(input.take(4)?.try_into().expect("four counted bytes")) as usize;
            let digits = std::str::from_utf8(input.take(length)?)
                .map_err(|_| ScalarWireError::DigitsNotUnicode)?;
            Carrier::BigInteger {
                negative,
                radix,
                digits: Rc::from(digits),
            }
        }
        3 => Carrier::Double(f64::from_bits(input.number()?)),
        4 => {
            let negative = input.boolean()?;
            let payload = if input.boolean()? {
                Some(input.number()?)
            } else {
                None
            };
            Carrier::Nan { negative, payload }
        }
        5 => {
            let value = input.boolean()?;
            let origin = match input.byte()? {
                0 => NativeCVersion::V8_4,
                1 => NativeCVersion::V8_5,
                2 => NativeCVersion::V8_6,
                3 => NativeCVersion::V9_0,
                4 => NativeCVersion::V9_1,
                _ => return Err(ScalarWireError::InvalidTag),
            };
            Carrier::WordBoolean { value, origin }
        }
        6 => Carrier::JimCoercedInteger(input.number()?.cast_signed()),
        _ => return Err(ScalarWireError::InvalidTag),
    };
    if !input.0.is_empty() {
        return Err(ScalarWireError::TrailingBytes);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Software transport controls: these assert preserved input facts, not a
    // claim that a native interpreter independently produced these caches.
    #[test]
    fn scalar_carriers_preserve_full_payload_and_independent_origins() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut cases = vec![
            Carrier::Tcl84Long(i64::MIN),
            Carrier::Integer(i64::MAX),
            Carrier::JimCoercedInteger(9_007_199_254_740_993),
            Carrier::Nan {
                negative: true,
                payload: Some(0x1234),
            },
        ];
        for radix in [
            NativeIntegerRadix::Binary,
            NativeIntegerRadix::Octal,
            NativeIntegerRadix::Decimal,
            NativeIntegerRadix::Hexadecimal,
        ] {
            cases.push(Carrier::BigInteger {
                negative: true,
                radix,
                digits: Rc::from("1000000000000000000000001"),
            });
        }
        for origin in [
            NativeCVersion::V8_4,
            NativeCVersion::V8_5,
            NativeCVersion::V8_6,
            NativeCVersion::V9_0,
            NativeCVersion::V9_1,
        ] {
            cases.push(Carrier::WordBoolean {
                value: true,
                origin,
            });
        }
        for original in cases {
            let (cache, origin) = import_scalar(&original);
            assert_eq!(export_scalar(cache, origin).unwrap(), original);
        }
        for bits in [
            (-0.0_f64).to_bits(),
            0x7ff8_0000_0000_1234,
            0xfff8_0000_0000_4321,
        ] {
            let (cache, origin) = import_scalar(&Carrier::Double(f64::from_bits(bits)));
            let Carrier::Double(value) = export_scalar(cache, origin).unwrap() else {
                panic!("double cache lost");
            };
            assert_eq!(value.to_bits(), bits);
        }
    }

    #[test]
    fn missing_boolean_origin_refuses_and_storage_is_never_inferred() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        assert_eq!(
            export_scalar(NativeScalarCache::WordBoolean(true), None),
            Err(NativeScalarCarrierError::WordBooleanOriginUnavailable)
        );
        for storage in [
            CarrierStorage::CanonicalEmpty,
            CarrierStorage::Allocated,
            CarrierStorage::Unknown,
        ] {
            assert_eq!(export_storage(import_storage(storage)), storage);
        }
    }
    #[test]
    fn counted_scalar_wire_preserves_facts_and_rejects_malformed_boundaries() {
        // Software transport: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let integer = Carrier::Integer(-42);
        assert_eq!(
            encode_scalar(&integer).unwrap(),
            [1, 214, 255, 255, 255, 255, 255, 255, 255]
        );
        let cases = [
            Carrier::Tcl84Long(i64::MIN),
            Carrier::JimCoercedInteger(i64::MAX),
            Carrier::BigInteger {
                negative: true,
                radix: NativeIntegerRadix::Hexadecimal,
                digits: Rc::from("123456789abcdef00123456789abcdef"),
            },
            Carrier::WordBoolean {
                value: false,
                origin: NativeCVersion::V8_5,
            },
            Carrier::Nan {
                negative: true,
                payload: Some(0x1234),
            },
            Carrier::Nan {
                negative: false,
                payload: None,
            },
        ];
        for cache in cases {
            assert_eq!(
                decode_scalar(&encode_scalar(&cache).unwrap()).unwrap(),
                cache
            );
        }
        for bits in [
            (-0.0_f64).to_bits(),
            0x7ff8_0000_0000_1234,
            0xfff8_0000_0000_4321,
        ] {
            let Carrier::Double(value) =
                decode_scalar(&encode_scalar(&Carrier::Double(f64::from_bits(bits))).unwrap())
                    .unwrap()
            else {
                panic!("float carrier lost its variant")
            };
            assert_eq!(value.to_bits(), bits);
        }
        assert_eq!(decode_scalar(&[5, 1, 9]), Err(ScalarWireError::InvalidTag));
        assert_eq!(decode_scalar(&[1, 42]), Err(ScalarWireError::Truncated));
        assert_eq!(
            decode_scalar(&[2, 0, 2, 255, 255, 255, 255]),
            Err(ScalarWireError::Truncated)
        );
        assert_eq!(decode_scalar(&[5, 2, 1]), Err(ScalarWireError::InvalidTag));
        assert_eq!(
            decode_scalar(&[5, 1, 1, 0]),
            Err(ScalarWireError::TrailingBytes)
        );
    }
}
