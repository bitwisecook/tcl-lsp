// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical C Tcl character units and byte-boundary traversal.
//!
//! These recipes follow the pinned canonical builds' `tclUtf.c`; they do not
//! require Rust UTF-8 or attest an object's storage, effects or string cache.

use tcl_dialect::TclVersion;

/// A decoded native unit, including surrogate units in C8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TclUtfUnit {
    /// Native character value, without a Unicode-scalar restriction.
    pub value: u32,
    /// Number of original bytes consumed.
    pub width: usize,
}

/// Character-unit policy for an independently authenticated canonical C build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeTclUtf {
    version: TclVersion,
}

impl NativeTclUtf {
    /// Whether the available prefix satisfies the selected native UTF unit's
    /// completeness table. This does not validate Rust UTF-8 or decode a unit.
    #[must_use]
    pub fn character_complete(self, bytes: &[u8]) -> bool {
        let Some(&first) = bytes.first() else {
            return false;
        };
        let required = match first {
            0x80..=0xbf if self.version >= TclVersion::V8_6 => 3,
            0xc0 | 0xc2..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf4 if self.version >= TclVersion::V9_0 => 4,
            _ => 1,
        };
        bytes.len() >= required
    }

    /// Format a command operand for native error logging. The input retains
    /// bytes beyond the selected command length when the producer has them.
    /// An unavailable extent is a capability failure, never an empty command.
    #[must_use]
    pub fn command_log_excerpt(self, source: &[u8], length: usize) -> Option<(&[u8], bool)> {
        if length > source.len() {
            return None;
        }
        let mut end = length.min(150);
        let mut ellipsis = length > 150;
        if self.version == TclVersion::V8_4 {
            while end > 0 && source.get(end).is_some_and(|byte| byte & 0xc0 == 0x80) {
                end -= 1;
                ellipsis = true;
            }
            let slice = &source[..end];
            let nul = slice
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(slice.len());
            return Some((&slice[..nul], ellipsis));
        }
        end = source[..end]
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(end);
        if end > 0 {
            let previous = self.previous_character_boundary(source, end)?;
            if !self.character_complete(&source[previous..end]) {
                end = previous;
            }
        }
        let mut start = 0;
        let maximum = if self.version >= TclVersion::V9_0 {
            4
        } else {
            3
        };
        while start < end && start < maximum && source[start] & 0xc0 == 0x80 {
            start += 1;
        }
        Some((&source[start..end], ellipsis))
    }

    /// Decode a complete owned byte extent, retaining the previous native unit.
    /// Literal NUL participates; this operation supplies no `CString` stopping rule.
    #[must_use]
    pub fn decode_units(self, bytes: &[u8]) -> Vec<u32> {
        let mut output = Vec::new();
        let mut offset = 0;
        let mut previous = None;
        while let Some(unit) = self.decode_unit(&bytes[offset..], previous) {
            output.push(unit.value);
            offset += unit.width;
            previous = Some(unit.value);
        }
        output
    }

    /// Encode retained native Unicode units without combining adjacent surrogates.
    /// Native zero uses modified UTF-8. Older C builds retain 16-bit units;
    /// an unaudited wider unit is unavailable rather than truncated.
    #[must_use]
    pub fn encode_units(self, units: &[u32]) -> Option<Vec<u8>> {
        let maximum = if self.version >= TclVersion::V9_0 {
            0x0010_ffff
        } else {
            0xffff
        };
        let mut bytes = Vec::new();
        for &unit in units {
            if unit > maximum {
                return None;
            }
            match unit {
                0 => bytes.extend_from_slice(&[0xc0, 0x80]),
                1..=0x7f => bytes.push(u8::try_from(unit).ok()?),
                0x80..=0x7ff => bytes.extend_from_slice(&[
                    u8::try_from(0xc0 | (unit >> 6)).ok()?,
                    u8::try_from(0x80 | (unit & 0x3f)).ok()?,
                ]),
                0x800..=0xffff => bytes.extend_from_slice(&[
                    u8::try_from(0xe0 | (unit >> 12)).ok()?,
                    u8::try_from(0x80 | ((unit >> 6) & 0x3f)).ok()?,
                    u8::try_from(0x80 | (unit & 0x3f)).ok()?,
                ]),
                _ => bytes.extend_from_slice(&[
                    u8::try_from(0xf0 | (unit >> 18)).ok()?,
                    u8::try_from(0x80 | ((unit >> 12) & 0x3f)).ok()?,
                    u8::try_from(0x80 | ((unit >> 6) & 0x3f)).ok()?,
                    u8::try_from(0x80 | (unit & 0x3f)).ok()?,
                ]),
            }
        }
        Some(bytes)
    }

    /// Encode the native integer-character API for an admitted Unicode point.
    /// The older three-byte API maps supplementary points to U+FFFD; this
    /// differs from admitting a unit into a 16-bit Unicode object.
    #[must_use]
    pub fn encode_character(self, point: u32) -> Option<Vec<u8>> {
        if point > 0x0010_ffff {
            return None;
        }
        let unit = if self.version < TclVersion::V9_0 && point > 0xffff {
            0xfffd
        } else {
            point
        };
        self.encode_units(&[unit])
    }
    /// Select the canonical build's units; this grants no native object proof.
    #[must_use]
    pub const fn for_version(version: TclVersion) -> Self {
        Self { version }
    }

    /// Native `TclpUtfNcmp2` over a counted byte prefix. At its first unequal
    /// byte it treats modified NUL as zero and returns that comparison directly.
    /// Each operand retains its owned terminator for the inspected next byte.
    #[must_use]
    pub fn compare_counted_bytes(
        left: &[u8],
        right: &[u8],
        count: usize,
    ) -> Option<std::cmp::Ordering> {
        if count > left.len() || count > right.len() {
            return None;
        }
        for index in 0..count {
            if left[index] != right[index] {
                let projected = |bytes: &[u8]| {
                    if bytes[index] == 0xc0 && bytes.get(index + 1) == Some(&0x80) {
                        0
                    } else {
                        bytes[index]
                    }
                };
                return Some(projected(left).cmp(&projected(right)));
            }
        }
        Some(std::cmp::Ordering::Equal)
    }

    /// Decode a unit from a length-delimited string. C8.6's decoder retains its
    /// previous unit to recognize the second half of a four-byte character.
    /// Empty input abstains; truncated/invalid sequences use native byte units.
    #[must_use]
    pub fn decode_unit(self, bytes: &[u8], previous: Option<u32>) -> Option<TclUtfUnit> {
        self.decode_unit_with(|index| bytes.get(index).copied(), previous)
    }

    /// Decode through a boundary-owned reader, requesting only the lead and
    /// continuation bytes tested by the native decoder. Literal NUL is a unit;
    /// this reader has no C-string stopping rule. The callback must provide
    /// stable bytes and enforce its own memory boundary. A missing continuation
    /// selects the native single-byte fallback; a missing lead abstains.
    #[must_use]
    pub fn decode_unit_with(
        self,
        mut read_byte: impl FnMut(usize) -> Option<u8>,
        previous: Option<u32>,
    ) -> Option<TclUtfUnit> {
        let lead = read_byte(0)?;
        let unit = |value, width| TclUtfUnit { value, width };
        if self.version == TclVersion::V8_6
            && continuation(lead)
            && let Some(bytes) = read_continuations(&mut read_byte, lead, 3)
            && let Some(prior) = previous
            && ((((u32::from(lead).wrapping_sub(0x10) << 2) & 0xfc) | 0xd800) == (prior & 0xfcfc))
            && u32::from(bytes[1] & 0xf0) == (((prior << 4) & 0x30) | 0x80)
        {
            return Some(unit(
                (u32::from(bytes[1] & 0x0f) << 6) + u32::from(bytes[2] & 0x3f) + 0xdc00,
                3,
            ));
        }
        if lead < 0xc0 {
            let value = if self.version >= TclVersion::V9_0 && (0x80..0xa0).contains(&lead) {
                CP1252[usize::from(lead - 0x80)]
            } else {
                u32::from(lead)
            };
            return Some(unit(value, 1));
        }
        let legacy = self.version <= TclVersion::V8_5;
        let width = match lead {
            0xc1 if !legacy => return Some(unit(u32::from(lead), 1)),
            0xc0..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf4 if !legacy => 4,
            _ => return Some(unit(u32::from(lead), 1)),
        };
        if self.version == TclVersion::V8_6 && width == 4 {
            if let Some(bytes) = read_continuations(&mut read_byte, lead, 3) {
                let high = ((u32::from(lead & 7) << 8)
                    | (u32::from(bytes[1] & 0x3f) << 2)
                    | u32::from((bytes[2] & 0x3f) >> 4))
                .wrapping_sub(0x40);
                if high < 0x400 {
                    return Some(unit(0xd800 + high, 1));
                }
            }
            return Some(unit(u32::from(lead), 1));
        }
        let Some(bytes) = read_continuations(&mut read_byte, lead, width) else {
            return Some(unit(u32::from(lead), 1));
        };
        let mask = match width {
            2 => 0x1f,
            3 => 0x0f,
            _ => 7,
        };
        let value = bytes[1..width]
            .iter()
            .fold(u32::from(lead & mask), |value, byte| {
                (value << 6) | u32::from(byte & 0x3f)
            });
        let accepted = legacy
            || match width {
                2 => value == 0 || value >= 0x80,
                3 => value > 0x7ff,
                _ => (0x1_0000..=0x10_ffff).contains(&value),
            };
        Some(if accepted {
            unit(value, width)
        } else {
            unit(u32::from(lead), 1)
        })
    }

    /// Append one native unit without surrogate-pair combination. Invalid unit
    /// values abstain before modifying the output; NUL uses modified UTF-8.
    pub fn encode_unit(self, value: u32, output: &mut Vec<u8>) -> Option<()> {
        let maximum = if self.version < TclVersion::V9_0 {
            0xffff
        } else {
            0x10_ffff
        };
        if value > maximum {
            return None;
        }
        match value {
            1..=0x7f => output.push(native_byte(value)),
            0..=0x7ff => output.extend_from_slice(&[
                native_byte(0xc0 | (value >> 6)),
                native_byte(0x80 | (value & 0x3f)),
            ]),
            0x800..=0xffff => output.extend_from_slice(&[
                native_byte(0xe0 | (value >> 12)),
                native_byte(0x80 | ((value >> 6) & 0x3f)),
                native_byte(0x80 | (value & 0x3f)),
            ]),
            _ => output.extend_from_slice(&[
                native_byte(0xf0 | (value >> 18)),
                native_byte(0x80 | ((value >> 12) & 0x3f)),
                native_byte(0x80 | ((value >> 6) & 0x3f)),
                native_byte(0x80 | (value & 0x3f)),
            ]),
        }
        Some(())
    }

    /// Native `Tcl_UtfPrev` boundary before `end`, relative to this string.
    /// The retained bytes must include the proposed end; zero/out-of-bounds
    /// positions abstain instead of performing native pointer underflow.
    #[must_use]
    pub fn previous_character_boundary(self, bytes: &[u8], end: usize) -> Option<usize> {
        let fallback = end.checked_sub(1)?;
        if end > bytes.len() {
            return None;
        }
        if fallback == 0 {
            return Some(0);
        }
        let mut look = fallback;
        let mut seen = 0;
        let maximum = if self.version >= TclVersion::V9_0 {
            4
        } else {
            3
        };
        loop {
            let byte = bytes[look];
            if self.version <= TclVersion::V8_5 {
                if byte < 0x80 {
                    return Some(fallback);
                }
                if byte >= 0xc0 {
                    return Some(look);
                }
            } else {
                if byte < 0x80 {
                    return Some(fallback);
                }
                if byte >= 0xc0 {
                    let width = match byte {
                        0xc1 => 1,
                        0xc0..=0xdf => 2,
                        0xe0..=0xef => 3,
                        0xf0..=0xf4 if maximum == 4 => 4,
                        _ => 0,
                    };
                    return Some(
                        if seen == 0 || seen >= width || invalid_prefix(bytes, look) {
                            fallback
                        } else {
                            look
                        },
                    );
                }
            }
            seen += 1;
            if look == 0 || seen >= maximum {
                return Some(fallback);
            }
            look -= 1;
        }
    }
}

fn native_byte(value: u32) -> u8 {
    u8::try_from(value).expect("native encoding combines bounded byte fields")
}

fn continuation(byte: u8) -> bool {
    byte & 0xc0 == 0x80
}
fn read_continuations(
    read_byte: &mut impl FnMut(usize) -> Option<u8>,
    lead: u8,
    width: usize,
) -> Option<[u8; 4]> {
    let mut bytes = [lead, 0, 0, 0];
    for (index, byte) in bytes.iter_mut().enumerate().take(width).skip(1) {
        *byte = read_byte(index)?;
        if !continuation(*byte) {
            return None;
        }
    }
    Some(bytes)
}

fn invalid_prefix(bytes: &[u8], at: usize) -> bool {
    let next = bytes[at + 1];
    match bytes[at] {
        0xc0 => next != 0x80,
        0xe0 => next < 0xa0,
        0xf0 => next < 0x90,
        0xf4 => next > 0x8f,
        _ => false,
    }
}
const CP1252: [u32; 32] = [
    0x20ac, 0x81, 0x201a, 0x192, 0x201e, 0x2026, 0x2020, 0x2021, 0x2c6, 0x2030, 0x160, 0x2039,
    0x152, 0x8d, 0x17d, 0x8f, 0x90, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014, 0x2dc,
    0x2122, 0x161, 0x203a, 0x153, 0x9d, 0x17e, 0x178,
];

#[cfg(test)]
mod tests {
    #[test]
    fn encoding_matches_every_selected_native_unit_control() {
        use super::*;
        let fixtures = [
            include_str!("../tests/data/native_tcl_utf/unicode_encoder/8.4.20.tsv"),
            include_str!("../tests/data/native_tcl_utf/unicode_encoder/8.5.19.tsv"),
            include_str!("../tests/data/native_tcl_utf/unicode_encoder/8.6.18.tsv"),
            include_str!("../tests/data/native_tcl_utf/unicode_encoder/9.0.4.tsv"),
            include_str!("../tests/data/native_tcl_utf/unicode_encoder/9.1.0.tsv"),
        ];
        let mut rows = 0;
        for (version, fixture) in TclVersion::ALL.into_iter().zip(fixtures) {
            let protocol = NativeTclUtf::for_version(version);
            for line in fixture.lines() {
                let columns: Vec<_> = line.split('\t').collect();
                let units: Vec<u32> = columns[1]
                    .split(',')
                    .map(|unit| unit.parse().unwrap())
                    .collect();
                let expected = (columns[2] != "-").then(|| {
                    columns[2]
                        .as_bytes()
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|hex| {
                            u8::from_str_radix(std::str::from_utf8(hex).unwrap(), 16).unwrap()
                        })
                        .collect::<Vec<_>>()
                });
                let actual = match columns[0] {
                    "character" => protocol.encode_character(units[0]),
                    "units" => protocol.encode_units(&units),
                    _ => panic!("native encoder fixture kind"),
                };
                assert_eq!(actual, expected, "{version:?}: {line}");
                rows += 1;
            }
        }
        assert_eq!(rows, 70);
    }
    use super::*;

    const FIXTURES: [(TclVersion, &str); 5] = [
        (
            TclVersion::V8_4,
            include_str!("../tests/data/native_scalar_getters/errors/character-units-8.4.20.txt"),
        ),
        (
            TclVersion::V8_5,
            include_str!("../tests/data/native_scalar_getters/errors/character-units-8.5.19.txt"),
        ),
        (
            TclVersion::V8_6,
            include_str!("../tests/data/native_scalar_getters/errors/character-units-8.6.18.txt"),
        ),
        (
            TclVersion::V9_0,
            include_str!("../tests/data/native_scalar_getters/errors/character-units-9.0.4.txt"),
        ),
        (
            TclVersion::V9_1,
            include_str!("../tests/data/native_scalar_getters/errors/character-units-9.1.0.txt"),
        ),
    ];

    fn field<'a>(line: &'a str, name: &str) -> &'a str {
        line.split_ascii_whitespace()
            .find_map(|value| {
                let (key, value) = value.split_once('=')?;
                (key == name).then_some(value)
            })
            .unwrap()
    }
    fn unhex(text: &str) -> Vec<u8> {
        text.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    fn assert_reader_accesses(
        policy: NativeTclUtf,
        bytes: &[u8],
        previous: Option<u32>,
        expected: TclUtfUnit,
        accesses: &[usize],
    ) {
        let mut requested = Vec::new();
        assert_eq!(
            policy.decode_unit_with(
                |index| {
                    requested.push(index);
                    bytes.get(index).copied()
                },
                previous
            ),
            Some(expected)
        );
        assert_eq!(requested, accesses);
    }

    #[test]
    fn reader_extent_keeps_nul_units_and_native_short_circuit_accesses() {
        let c9 = NativeTclUtf::for_version(TclVersion::V9_0);
        let c86 = NativeTclUtf::for_version(TclVersion::V8_6);
        let c84 = NativeTclUtf::for_version(TclVersion::V8_4);
        for (policy, bytes, previous, expected, accesses) in [
            (
                c9,
                b"\0z".as_slice(),
                None,
                TclUtfUnit { value: 0, width: 1 },
                vec![0],
            ),
            (
                c9,
                b"\xc1\x81",
                None,
                TclUtfUnit {
                    value: 0xc1,
                    width: 1,
                },
                vec![0],
            ),
            (
                c84,
                b"\xc1\x81",
                None,
                TclUtfUnit {
                    value: 0x41,
                    width: 2,
                },
                vec![0, 1],
            ),
            (
                c9,
                b"\xe2A\xac",
                None,
                TclUtfUnit {
                    value: 0xe2,
                    width: 1,
                },
                vec![0, 1],
            ),
            (
                c9,
                b"\xe2\x82",
                None,
                TclUtfUnit {
                    value: 0xe2,
                    width: 1,
                },
                vec![0, 1, 2],
            ),
            (
                c86,
                b"\xf0\x9f\x98\x80",
                None,
                TclUtfUnit {
                    value: 0xd83d,
                    width: 1,
                },
                vec![0, 1, 2],
            ),
            (
                c86,
                b"\x9f\x98\x80",
                Some(0xd83d),
                TclUtfUnit {
                    value: 0xde00,
                    width: 3,
                },
                vec![0, 1, 2],
            ),
            (
                c9,
                b"\xf0\x9f\x98\x80",
                None,
                TclUtfUnit {
                    value: 0x1f600,
                    width: 4,
                },
                vec![0, 1, 2, 3],
            ),
        ] {
            assert_reader_accesses(policy, bytes, previous, expected, &accesses);
        }
        let bytes = b"k\0z";
        let units: Vec<_> = (0..bytes.len())
            .map(|at| {
                c9.decode_unit_with(|offset| bytes.get(at + offset).copied(), None)
                    .unwrap()
                    .value
            })
            .collect();
        assert_eq!(units, [0x6b, 0, 0x7a]);
    }

    #[test]
    fn canonical_native_units_encoding_and_previous_boundaries_match_five_releases() {
        let mut count = 0;
        for (version, fixture) in FIXTURES {
            let policy = NativeTclUtf::for_version(version);
            for line in fixture.lines() {
                let bytes = unhex(field(line, "input"));
                let mut previous = None;
                let mut at = 0;
                let mut units = Vec::new();
                let mut encoded = Vec::new();
                while at < bytes.len() {
                    let unit = policy.decode_unit(&bytes[at..], previous).unwrap();
                    units.push(format!("{:x}:{}", unit.value, unit.width));
                    policy.encode_unit(unit.value, &mut encoded).unwrap();
                    previous = Some(unit.value);
                    at += unit.width;
                }
                assert_eq!(units.join(","), field(line, "units"), "{version:?}: {line}");
                assert_eq!(
                    encoded,
                    unhex(field(line, "encoded")),
                    "{version:?}: {line}"
                );
                let boundaries = (1..=bytes.len())
                    .map(|end| {
                        policy
                            .previous_character_boundary(&bytes, end)
                            .unwrap()
                            .to_string()
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                assert_eq!(boundaries, field(line, "boundaries"), "{version:?}: {line}");
                count += 1;
            }
        }
        assert_eq!(count, 70);
    }
}
