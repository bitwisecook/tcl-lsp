// SPDX-License-Identifier: AGPL-3.0-or-later
//! Full native numeric probes, independently of bounded integer extraction.

use super::{
    NativeScalarCache, NativeScalarGetterFailure, NativeScalarGetterKind,
    NativeScalarGetterProtocol, parse_number,
};
use crate::number::Number;
use tcl_dialect::TclVersion;

/// Native numeric purpose before a command applies its arithmetic constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeNumberGetterKind {
    /// C's internal `GetNumberFromObj`; fresh floating spellings install Double.
    Number,
    /// `TclIncrObj`'s inline Number stage, including its physical empty-string guard.
    IncrementNumber,
    /// Copying `GetBignumFromObj`; fresh parsing admits only integers.
    Bignum,
}

/// Reached full numeric cache and independent native probe outcome.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeNumberGetterConversion {
    cache: Option<NativeScalarCache>,
    outcome: Result<Number, NativeScalarGetterFailure>,
}

impl NativeNumberGetterConversion {
    /// Cache change retained on the original object before the probe completes.
    #[must_use]
    pub fn cache(&self) -> Option<&NativeScalarCache> {
        self.cache.as_ref()
    }

    /// Native full magnitude/category, independently of any later integer restriction.
    #[must_use]
    pub fn outcome(&self) -> &Result<Number, NativeScalarGetterFailure> {
        &self.outcome
    }

    /// Consume the reached cache change and separate native outcome.
    pub fn into_parts(
        self,
    ) -> (
        Option<NativeScalarCache>,
        Result<Number, NativeScalarGetterFailure>,
    ) {
        (self.cache, self.outcome)
    }
}

impl NativeScalarGetterProtocol {
    /// Whether this actual engine supplies the C full-number/Bignum primitive.
    /// C8.4 and Jim use independently selected integer/expression operations.
    #[must_use]
    pub fn supports_number_getter(self) -> bool {
        self.tcl_version()
            .is_some_and(|version| version >= TclVersion::V8_5)
    }

    /// Inspect the original numeric category without string generation.
    /// None requires the actual string; it never permits foreign-cache donation.
    #[must_use]
    pub fn cached_number_conversion(
        self,
        kind: NativeNumberGetterKind,
        cache: &NativeScalarCache,
    ) -> Option<NativeNumberGetterConversion> {
        if !self.supports_number_getter() {
            return None;
        }
        let NativeScalarCache::Number(number) = cache else {
            return None;
        };
        let outcome = if kind == NativeNumberGetterKind::Bignum
            && !matches!(number, Number::Int(_) | Number::Big { .. })
        {
            Err(NativeScalarGetterFailure::CachedNonInteger)
        } else {
            Ok(number.clone())
        };
        Some(NativeNumberGetterConversion {
            cache: None,
            outcome,
        })
    }

    /// Reach the increment macro's cache/empty check before any string updater.
    /// The original NULL descriptor is independent of absence of a numeric cache.
    #[must_use]
    pub fn increment_number_preflight(
        self,
        cache: Option<&NativeScalarCache>,
        resident_length: Option<usize>,
        null_primary: bool,
    ) -> Option<NativeNumberGetterConversion> {
        if !self.supports_number_getter() {
            return None;
        }
        if let Some(
            cache @ NativeScalarCache::Number(
                Number::Int(_) | Number::Double(_) | Number::Nan { .. },
            ),
        ) = cache
        {
            return self.cached_number_conversion(NativeNumberGetterKind::Number, cache);
        }
        if resident_length == Some(0)
            || (self.tcl_version() == Some(TclVersion::V8_5)
                && null_primary
                && resident_length.is_none())
        {
            return Some(NativeNumberGetterConversion {
                cache: None,
                outcome: Err(NativeScalarGetterFailure::Invalid),
            });
        }
        cache.and_then(|cache| self.cached_number_conversion(NativeNumberGetterKind::Number, cache))
    }

    /// Parse actual materialised native bytes using the selected numeric purpose.
    /// Bignum parsing never borrows `GetNumber`'s floating cache installation.
    #[must_use]
    pub fn fresh_number_conversion(
        self,
        kind: NativeNumberGetterKind,
        materialised: &[u8],
    ) -> Option<NativeNumberGetterConversion> {
        if !self.supports_number_getter() {
            return None;
        }
        let input = self.parser_input(materialised);
        let Some(number) = parse_number(
            input,
            self.tcl_version()?,
            kind == NativeNumberGetterKind::Bignum,
        ) else {
            let failure = self
                .invalid_conversion(NativeScalarGetterKind::Wide, input)
                .outcome
                .expect_err("invalid numeral conversion");
            return Some(NativeNumberGetterConversion {
                cache: None,
                outcome: Err(failure),
            });
        };
        Some(NativeNumberGetterConversion {
            cache: Some(NativeScalarCache::Number(number.clone())),
            outcome: Ok(number),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::number::Radix;

    fn cache(shape: usize) -> Option<NativeScalarCache> {
        match shape {
            10 => Some(NativeScalarCache::Number(Number::Double(1.5))),
            11 => Some(NativeScalarCache::Number(Number::Double(f64::NAN))),
            12 => Some(NativeScalarCache::Number(Number::Int(7))),
            13 => Some(NativeScalarCache::WordBoolean(true)),
            14 | 15 => Some(NativeScalarCache::Number(Number::Big {
                negative: false,
                radix: Radix::Dec,
                digits: "184467440737095516160000".into(),
            })),
            _ => None,
        }
    }
    fn native_type(cache: Option<&NativeScalarCache>, version: TclVersion) -> &'static str {
        match cache {
            None => "none",
            Some(NativeScalarCache::WordBoolean(_)) => {
                if version >= TclVersion::V9_0 {
                    "boolean"
                } else {
                    "booleanString"
                }
            }
            Some(NativeScalarCache::Number(Number::Int(_))) => "int",
            Some(NativeScalarCache::Number(Number::Double(_) | Number::Nan { .. })) => "double",
            Some(NativeScalarCache::Number(Number::Big { .. })) => "bignum",
            _ => unreachable!("C fixture cache"),
        }
    }

    #[test]
    fn numeric_purposes_match_all_128_direct_native_probe_rows() {
        let fixtures = [
            (
                TclVersion::V8_5,
                include_str!("../../tests/data/native_scalar_getters/number/8.5.19.tsv"),
            ),
            (
                TclVersion::V8_6,
                include_str!("../../tests/data/native_scalar_getters/number/8.6.18.tsv"),
            ),
            (
                TclVersion::V9_0,
                include_str!("../../tests/data/native_scalar_getters/number/9.0.4.tsv"),
            ),
            (
                TclVersion::V9_1,
                include_str!("../../tests/data/native_scalar_getters/number/9.1.0.tsv"),
            ),
        ];
        let texts: [&[u8]; 10] = [
            b"1",
            b"1.5",
            b"NaN",
            b"08",
            b"184467440737095516160000",
            b"true",
            b"",
            b" 2 ",
            b"1\0x",
            b"1\xc0\x80x",
        ];
        let mut count = 0;
        for (version, fixture) in fixtures {
            let protocol = NativeScalarGetterProtocol::for_tcl_version(version);
            for row in fixture.lines() {
                let fields: Vec<_> = row.split('\t').collect();
                let shape: usize = fields[0].parse().unwrap();
                let stage: usize = fields[1].parse().unwrap();
                if stage == 2 {
                    continue;
                }
                let kind = if stage == 0 {
                    NativeNumberGetterKind::Number
                } else {
                    NativeNumberGetterKind::Bignum
                };
                let old = cache(shape);
                let conversion = old
                    .as_ref()
                    .and_then(|cache| protocol.cached_number_conversion(kind, cache))
                    .unwrap_or_else(|| {
                        protocol
                            .fresh_number_conversion(
                                kind,
                                if shape < 10 { texts[shape] } else { b"true" },
                            )
                            .unwrap()
                    });
                assert_eq!(
                    conversion.outcome().is_err(),
                    fields[5] == "1",
                    "{version:?} {row}"
                );
                assert_eq!(
                    native_type(conversion.cache().or(old.as_ref()), version),
                    fields[7],
                    "{version:?} {row}"
                );
                if let Ok(Number::Big {
                    negative,
                    radix,
                    digits,
                }) = conversion.outcome()
                {
                    assert!(!*negative);
                    assert_eq!(*radix, Radix::Dec);
                    assert_eq!(digits, "184467440737095516160000");
                }
                count += 1;
            }
        }
        assert_eq!(count, 128);
    }

    #[test]
    fn increment_preflight_distinguishes_empty_big_from_direct_number() {
        let big = cache(15).unwrap();
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let protocol = NativeScalarGetterProtocol::for_tcl_version(version);
            assert!(
                protocol
                    .cached_number_conversion(NativeNumberGetterKind::Number, &big)
                    .unwrap()
                    .outcome()
                    .is_ok()
            );
            let rejected = protocol
                .increment_number_preflight(Some(&big), Some(0), false)
                .unwrap();
            assert!(rejected.outcome().is_err());
            assert!(rejected.cache().is_none());
            let int = NativeScalarCache::Number(Number::Int(7));
            assert!(
                protocol
                    .increment_number_preflight(Some(&int), Some(0), false)
                    .unwrap()
                    .outcome()
                    .is_ok()
            );
        }
        for protocol in [
            NativeScalarGetterProtocol::for_tcl_version(TclVersion::V8_4),
            NativeScalarGetterProtocol {
                engine: super::super::Engine::Jim084,
            },
        ] {
            assert!(!protocol.supports_number_getter());
            assert!(
                protocol
                    .cached_number_conversion(NativeNumberGetterKind::Number, &big)
                    .is_none()
            );
            assert!(
                protocol
                    .fresh_number_conversion(NativeNumberGetterKind::Bignum, b"1")
                    .is_none()
            );
        }
    }
}
