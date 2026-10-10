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
}
impl std::fmt::Display for NativeScalarCarrierError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WordBooleanOriginUnavailable => {
                formatter.write_str("native word-Boolean descriptor origin is unavailable")
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

#[cfg(test)]
mod tests {
    use super::*;

    // Software transport controls: these assert preserved input facts, not a
    // claim that a native interpreter independently produced these caches.
    #[test]
    fn scalar_carriers_preserve_full_payload_and_independent_origins() {
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
}
