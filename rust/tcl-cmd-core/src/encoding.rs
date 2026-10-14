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

//! `encoding convertfrom` / `encoding convertto`: the argument grammar, the
//! string-to-bytes view of the data word, and the conversions themselves for
//! the encodings the native runtimes name (`utf-8`, `unicode`, `ascii`,
//! `iso8859-1`).
//!
//! Each release reads malformed input its own way. Tcl 8.4/8.5 cap a UTF-8
//! character at three bytes and accept overlong forms; 8.6 rejects overlongs
//! other than `C0 80` and assembles four-byte sequences; both map a byte they
//! cannot decode to the character of the same value. Tcl 9 adds conversion
//! profiles: `strict` (the default) fails at the first bad sequence, `tcl8`
//! maps a stray byte through cp1252, and `replace` substitutes U+FFFD.
//!
//! A Tcl 8 string or a Tcl 9 `tcl8` result can hold an unpaired surrogate; a
//! Rust string cannot, so one becomes U+FFFD here.

use tcl_dialect::{ByteStringEncoding, TclVersion};
use tcl_platform::SystemEncoding;

use crate::CmdError;
use crate::channel::{EncodingProfile, resolve_profile};
use crate::prefix::OptionTable;

/// Which way an `encoding convert*` call converts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// `convertfrom`: external bytes to a Tcl string.
    From,
    /// `convertto`: a Tcl string to external bytes.
    To,
}

impl Direction {
    const fn subcommand(self) -> &'static str {
        match self {
            Self::From => "convertfrom",
            Self::To => "convertto",
        }
    }
}

/// A parsed `encoding convertfrom|convertto` argument list. Word positions
/// index the slice handed to [`parse_convert_args`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConvertArgs {
    /// The named encoding, or `None` for the interpreter's system encoding.
    pub encoding: Option<SystemEncoding>,
    pub profile: EncodingProfile,
    /// The `-failindex` variable-name word.
    pub fail_var: Option<usize>,
    /// The data word.
    pub data: usize,
}

const OPTIONS: &[&str] = &["-profile", "-failindex"];

/// Parse the words after `encoding convertfrom` / `encoding convertto`.
///
/// Tcl 8 takes `?encoding? data`. Tcl 9 takes `data` alone or
/// `?-profile profile? ?-failindex var? encoding data`, with the `strict`
/// profile by default (`EncodingConvertParseOptions`).
///
/// # Errors
/// An arity error, an unknown option or profile, or an unknown encoding.
pub fn parse_convert_args(
    version: TclVersion,
    direction: Direction,
    words: &[&str],
) -> Result<ConvertArgs, CmdError> {
    let sub = direction.subcommand();
    if version < TclVersion::V9_0 {
        return match words {
            [_] => Ok(ConvertArgs {
                encoding: None,
                profile: EncodingProfile::Tcl8,
                fail_var: None,
                data: 0,
            }),
            [name, _] => Ok(ConvertArgs {
                encoding: Some(resolve_encoding(name)?),
                profile: EncodingProfile::Tcl8,
                fail_var: None,
                data: 1,
            }),
            _ => Err(CmdError::wrong_args(&format!(
                "encoding {sub} ?encoding? data"
            ))),
        };
    }
    let arity = || {
        CmdError::new(format!(
            "wrong # args: should be \"encoding {sub} ?-profile profile? ?-failindex var? encoding data\" or \"encoding {sub} data\""
        ))
    };
    let mut parsed = ConvertArgs {
        encoding: None,
        profile: EncodingProfile::Strict,
        fail_var: None,
        data: 0,
    };
    match words.len() {
        0 => return Err(arity()),
        1 => return Ok(parsed),
        _ => {}
    }
    let encoding_at = words.len() - 2;
    let table = OptionTable::abbreviating("option", OPTIONS);
    let mut at = 0;
    while at < encoding_at {
        let option = table.index_of_str(words[at])?;
        at += 1;
        if at == encoding_at {
            return Err(arity());
        }
        if option == 0 {
            parsed.profile = resolve_profile(words[at])?;
        } else {
            parsed.fail_var = Some(at);
        }
        at += 1;
    }
    parsed.encoding = Some(resolve_encoding(words[encoding_at])?);
    parsed.data = encoding_at + 1;
    Ok(parsed)
}

/// Resolve an exact encoding name the conversions implement.
///
/// # Errors
/// Any other name, with Tcl's `TCL LOOKUP ENCODING` identity.
pub fn resolve_encoding(name: &str) -> Result<SystemEncoding, CmdError> {
    match name {
        "utf-8" => Ok(SystemEncoding::Utf8),
        "iso8859-1" => Ok(SystemEncoding::Iso88591),
        "ascii" => Ok(SystemEncoding::Ascii),
        "unicode" => Ok(SystemEncoding::Unicode),
        _ => Err(CmdError::with_error_code(
            format!("unknown encoding \"{name}\""),
            tcl_syntax::list::join_list(["TCL", "LOOKUP", "ENCODING", name]),
        )),
    }
}

/// A character outside the byte domain of a Tcl 9 string read as bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteConversionError {
    /// Byte offset in the UTF-8 text where conversion fails.
    pub byte_offset: usize,
    pub code_point: u32,
}

impl From<ByteConversionError> for CmdError {
    fn from(error: ByteConversionError) -> Self {
        Self::with_error_code(
            format!(
                "expected code point values below 0xff but value at byte offset {} was 0x{:x}",
                error.byte_offset, error.code_point
            ),
            "TCL VALUE BYTES",
        )
    }
}

/// The bytes a Tcl string stands for when a command reads it as binary data
/// (`Tcl_GetBytesFromObj`): each character's value, which Tcl 8 truncates to
/// its low byte and Tcl 9 rejects above U+00FF.
///
/// # Errors
/// Under [`ByteStringEncoding::CheckedLatin1`], the first character above
/// U+00FF.
pub fn text_bytes(text: &str, policy: ByteStringEncoding) -> Result<Vec<u8>, ByteConversionError> {
    let mut out = Vec::with_capacity(text.len());
    for (byte_offset, ch) in text.char_indices() {
        let code_point = u32::from(ch);
        if let Ok(byte) = u8::try_from(code_point) {
            out.push(byte);
            continue;
        }
        match policy {
            ByteStringEncoding::LegacyTruncate => out.push(code_point.to_le_bytes()[0]),
            ByteStringEncoding::CheckedLatin1 => {
                return Err(ByteConversionError {
                    byte_offset,
                    code_point,
                });
            }
        }
    }
    Ok(out)
}

/// A conversion's result: everything converted before the first failure, and
/// that failure, which only the Tcl 9 `strict` profile reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Converted<T> {
    pub value: T,
    pub failure: Option<ConversionFailure>,
}

/// Where a `strict` conversion stopped and the error Tcl raises for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionFailure {
    /// The byte offset `-failindex` reports: into the external bytes for
    /// `convertfrom`, into Tcl's internal UTF-8 for `convertto`.
    pub index: usize,
    pub error: CmdError,
}

impl ConversionFailure {
    fn byte(bytes: &[u8], index: usize) -> Self {
        Self {
            index,
            error: CmdError::with_error_code(
                format!(
                    "unexpected byte sequence starting at index {index}: '\\x{:02X}'",
                    bytes[index]
                ),
                format!("TCL ENCODING ILLEGALSEQUENCE {index}"),
            ),
        }
    }

    /// The message counts characters; the error code and `-failindex`
    /// count the bytes before `ch` in Tcl's internal UTF-8, where NUL takes
    /// two (`C0 80`).
    fn character(text: &str, char_index: usize, ch: char) -> Self {
        let index = text
            .chars()
            .take(char_index)
            .map(|ch| if ch == '\0' { 2 } else { ch.len_utf8() })
            .sum();
        Self {
            index,
            error: CmdError::with_error_code(
                format!(
                    "unexpected character at index {char_index}: 'U+{:06X}'",
                    u32::from(ch)
                ),
                format!("TCL ENCODING ILLEGALSEQUENCE {index}"),
            ),
        }
    }
}

/// Tcl 9's cp1252 reading of `0x80..=0x9F` for a stray byte under `tcl8`
/// (`Tcl_UtfToUniChar`); the five bytes cp1252 leaves undefined keep their
/// own value.
const CP1252_C1: [u16; 32] = [
    0x20AC, 0x81, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039,
    0x0152, 0x8D, 0x017D, 0x8F, 0x90, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014,
    0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x9D, 0x017E, 0x0178,
];

/// The character a byte that decodes as nothing becomes under `profile`, or
/// `None` when `profile` is `strict`.
fn stray_byte(version: TclVersion, profile: EncodingProfile, byte: u8) -> Option<char> {
    match profile {
        EncodingProfile::Strict => None,
        EncodingProfile::Replace => Some(char::REPLACEMENT_CHARACTER),
        EncodingProfile::Tcl8 if version >= TclVersion::V9_0 && (0x80..0xA0).contains(&byte) => {
            char::from_u32(u32::from(CP1252_C1[usize::from(byte - 0x80)]))
        }
        EncodingProfile::Tcl8 => Some(char::from(byte)),
    }
}

/// Decode `bytes` from `encoding` as `version` does under `profile`.
#[must_use]
pub fn convert_from(
    bytes: &[u8],
    encoding: SystemEncoding,
    version: TclVersion,
    profile: EncodingProfile,
) -> Converted<String> {
    match encoding {
        SystemEncoding::Utf8 => utf8_decode(bytes, version, profile),
        SystemEncoding::Unicode => utf16_decode(bytes, version, profile),
        SystemEncoding::Iso88591 => Converted {
            value: bytes.iter().copied().map(char::from).collect(),
            failure: None,
        },
        SystemEncoding::Ascii => {
            let mut value = String::with_capacity(bytes.len());
            for (at, &byte) in bytes.iter().enumerate() {
                if byte.is_ascii() || version < TclVersion::V9_0 {
                    value.push(char::from(byte));
                } else if let Some(ch) = stray_byte(version, profile, byte) {
                    value.push(ch);
                } else {
                    return Converted {
                        value,
                        failure: Some(ConversionFailure::byte(bytes, at)),
                    };
                }
            }
            Converted {
                value,
                failure: None,
            }
        }
    }
}

/// One step of a release's UTF-8 reader.
enum Utf8Step {
    /// A code point (possibly a surrogate) and its length in bytes.
    Char(u32, usize),
    /// The lead byte decodes as nothing and stands alone.
    Stray,
}

fn is_continuation(rest: &[u8], at: usize) -> bool {
    rest.get(at).is_some_and(|&byte| byte & 0xC0 == 0x80)
}

fn assemble(rest: &[u8], len: usize, lead_bits: u8) -> u32 {
    rest[1..len]
        .iter()
        .fold(u32::from(rest[0] & lead_bits), |ch, &byte| {
            (ch << 6) | u32::from(byte & 0x3F)
        })
}

/// `Tcl_UtfToUniChar` for the release, at the start of `rest`.
fn utf8_step(version: TclVersion, rest: &[u8]) -> Utf8Step {
    let lead = rest[0];
    if lead < 0x80 {
        return Utf8Step::Char(u32::from(lead), 1);
    }
    let legacy = version < TclVersion::V8_6;
    let (len, accept): (usize, fn(u32) -> bool) = match lead {
        0xC0..=0xDF if legacy => (2, |_| true),
        // `C0 80` is Tcl's internal NUL; every other overlong is rejected.
        0xC0 | 0xC2..=0xDF => (2, |ch| ch == 0 || ch >= 0x80),
        0xE0..=0xEF if legacy => (3, |_| true),
        0xE0..=0xEF => (3, |ch| ch > 0x7FF),
        // 8.4/8.5 hold at most three bytes per character.
        0xF0..=0xF4 if !legacy => (4, |ch| (0x1_0000..=0x10_FFFF).contains(&ch)),
        _ => return Utf8Step::Stray,
    };
    if !(1..len).all(|at| is_continuation(rest, at)) {
        return Utf8Step::Stray;
    }
    let ch = assemble(rest, len, 0x7F >> len);
    if accept(ch) {
        Utf8Step::Char(ch, len)
    } else {
        Utf8Step::Stray
    }
}

const fn is_surrogate(ch: u32) -> bool {
    matches!(ch, 0xD800..=0xDFFF)
}

fn utf8_decode(bytes: &[u8], version: TclVersion, profile: EncodingProfile) -> Converted<String> {
    let mut value = String::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let rest = &bytes[at..];
        let (ch, len) = match utf8_step(version, rest) {
            Utf8Step::Char(ch, len) if !is_surrogate(ch) => (ch, len),
            // Tcl 8.6 strings are UTF-16, so a high and a low surrogate read
            // back to back (CESU-8) are one character.
            Utf8Step::Char(high @ 0xD800..=0xDBFF, 3) if version == TclVersion::V8_6 => {
                let tail = &rest[3..];
                match (!tail.is_empty()).then(|| utf8_step(version, tail)) {
                    Some(Utf8Step::Char(low @ 0xDC00..=0xDFFF, 3)) => {
                        (0x1_0000 + ((high - 0xD800) << 10) + (low - 0xDC00), 6)
                    }
                    _ => (u32::from(char::REPLACEMENT_CHARACTER), 3),
                }
            }
            Utf8Step::Char(_, len) => {
                if profile == EncodingProfile::Strict {
                    return Converted {
                        value,
                        failure: Some(ConversionFailure::byte(bytes, at)),
                    };
                }
                (u32::from(char::REPLACEMENT_CHARACTER), len)
            }
            Utf8Step::Stray => match stray_byte(version, profile, rest[0]) {
                Some(ch) => (u32::from(ch), 1),
                None => {
                    return Converted {
                        value,
                        failure: Some(ConversionFailure::byte(bytes, at)),
                    };
                }
            },
        };
        // Tcl 9 reads `C0 80` as NUL only under `tcl8`.
        if ch == 0 && len == 2 && version >= TclVersion::V9_0 {
            match profile {
                EncodingProfile::Tcl8 => {}
                EncodingProfile::Replace => {
                    value.push(char::REPLACEMENT_CHARACTER);
                    at += len;
                    continue;
                }
                EncodingProfile::Strict => {
                    return Converted {
                        value,
                        failure: Some(ConversionFailure::byte(bytes, at)),
                    };
                }
            }
        }
        value.push(char::from_u32(ch).unwrap_or(char::REPLACEMENT_CHARACTER));
        at += len;
    }
    Converted {
        value,
        failure: None,
    }
}

/// Native-endian UTF-16, the `unicode` encoding.
fn utf16_decode(bytes: &[u8], version: TclVersion, profile: EncodingProfile) -> Converted<String> {
    let (units, odd) = bytes.as_chunks::<2>();
    let units: Vec<u16> = units.iter().copied().map(u16::from_ne_bytes).collect();
    let mut value = String::with_capacity(units.len());
    // 8.4/8.5 never pair surrogates.
    let pairs = version >= TclVersion::V8_6;
    let mut at = 0;
    while at < units.len() {
        let unit = units[at];
        let next = units.get(at + 1).copied();
        match (unit, next) {
            (0xD800..=0xDBFF, Some(low @ 0xDC00..=0xDFFF)) if pairs => {
                let ch = 0x1_0000 + ((u32::from(unit) - 0xD800) << 10) + (u32::from(low) - 0xDC00);
                value.push(char::from_u32(ch).unwrap_or(char::REPLACEMENT_CHARACTER));
                at += 2;
                continue;
            }
            (0xD800..=0xDFFF, _) if profile == EncodingProfile::Strict => {
                return Converted {
                    value,
                    failure: Some(ConversionFailure::byte(bytes, at * 2)),
                };
            }
            _ => {}
        }
        value.push(char::from_u32(u32::from(unit)).unwrap_or(char::REPLACEMENT_CHARACTER));
        at += 1;
    }
    // A trailing odd byte: 8.4/8.5 drop it, later releases read U+FFFD.
    // Tcl 9 fails at it under every profile when an unpaired high surrogate
    // precedes it (`Utf16ToUtfProc` keeps its truncation result).
    if !odd.is_empty() {
        let after_high = matches!(units.last(), Some(0xD800..=0xDBFF));
        if profile == EncodingProfile::Strict || (after_high && version >= TclVersion::V9_0) {
            return Converted {
                value,
                failure: Some(ConversionFailure::byte(bytes, bytes.len() - 1)),
            };
        }
        if version >= TclVersion::V8_6 {
            value.push(char::REPLACEMENT_CHARACTER);
        }
    }
    Converted {
        value,
        failure: None,
    }
}

/// Encode `text` into `encoding` as `version` does under `profile`. A
/// character the encoding cannot represent becomes `?`, or fails under
/// `strict`.
#[must_use]
pub fn convert_to(
    text: &str,
    encoding: SystemEncoding,
    version: TclVersion,
    profile: EncodingProfile,
) -> Converted<Vec<u8>> {
    let limit = match encoding {
        SystemEncoding::Utf8 => {
            return Converted {
                value: text.as_bytes().to_vec(),
                failure: None,
            };
        }
        SystemEncoding::Unicode => {
            return Converted {
                value: text.encode_utf16().flat_map(u16::to_ne_bytes).collect(),
                failure: None,
            };
        }
        SystemEncoding::Iso88591 => 0xFF,
        // Tcl 8's `ascii` table has no DEL.
        SystemEncoding::Ascii if version < TclVersion::V9_0 => 0x7E,
        SystemEncoding::Ascii => 0x7F,
    };
    let mut value = Vec::with_capacity(text.len());
    for (index, ch) in text.chars().enumerate() {
        match u8::try_from(u32::from(ch)) {
            Ok(byte) if byte <= limit => value.push(byte),
            _ if profile == EncodingProfile::Strict => {
                return Converted {
                    value,
                    failure: Some(ConversionFailure::character(text, index, ch)),
                };
            }
            _ => value.push(b'?'),
        }
    }
    Converted {
        value,
        failure: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const V84: TclVersion = TclVersion::V8_4;
    const V86: TclVersion = TclVersion::V8_6;
    const V90: TclVersion = TclVersion::V9_0;

    fn from_utf8(bytes: &[u8], version: TclVersion, profile: EncodingProfile) -> Vec<u32> {
        let converted = convert_from(bytes, SystemEncoding::Utf8, version, profile);
        assert!(converted.failure.is_none(), "{bytes:02x?} failed");
        converted.value.chars().map(u32::from).collect()
    }

    fn failure(converted: &Converted<String>) -> (usize, &str) {
        let failure = converted.failure.as_ref().expect("a strict failure");
        (failure.index, failure.error.message())
    }

    /// The witnesses of the report (tclsh 8.4.20 / 8.6.18 / 9.0.4).
    #[test]
    fn well_formed_utf8_decodes_per_release() {
        let tcl8 = EncodingProfile::Tcl8;
        let strict = EncodingProfile::Strict;
        let euro = [0xE2, 0x82, 0xAC];
        let clef = [0xF0, 0x9D, 0x84, 0x9E];
        assert_eq!(from_utf8(&euro, V84, tcl8), [0x20AC]);
        assert_eq!(from_utf8(&euro, V90, strict), [0x20AC]);
        assert_eq!(from_utf8(&clef, V84, tcl8), [0xF0, 0x9D, 0x84, 0x9E]);
        assert_eq!(from_utf8(&clef, V86, tcl8), [0x1D11E]);
        assert_eq!(from_utf8(&clef, V90, strict), [0x1D11E]);
    }

    #[test]
    fn malformed_utf8_follows_each_release_and_profile() {
        let tcl8 = EncodingProfile::Tcl8;
        // 8.4 accepts overlongs; 8.6 only `C0 80`.
        assert_eq!(from_utf8(&[0xC1, 0xBF], V84, tcl8), [0x7F]);
        assert_eq!(from_utf8(&[0xC1, 0xBF], V86, tcl8), [0xC1, 0xBF]);
        assert_eq!(from_utf8(&[0xC0, 0x80], V86, tcl8), [0]);
        assert_eq!(from_utf8(&[0xE0, 0x80, 0x80], V84, tcl8), [0]);
        assert_eq!(
            from_utf8(&[0xE0, 0x80, 0x80], V86, tcl8),
            [0xE0, 0x80, 0x80]
        );
        // 8.6 pairs CESU-8 surrogates; 9.0 `tcl8` reads them as strays.
        let cesu = [0xED, 0xA0, 0xBD, 0xED, 0xB8, 0x80];
        assert_eq!(from_utf8(&cesu, V86, tcl8), [0x1F600]);
        assert_eq!(from_utf8(&cesu, V90, tcl8), [0xFFFD, 0xFFFD]);
        // 9.0 `tcl8` reads a stray C1 byte through cp1252, 8.x as Latin-1.
        let stray = [0x41, 0xF0, 0x9D, 0x84, 0x41];
        assert_eq!(from_utf8(&stray, V86, tcl8), [0x41, 0xF0, 0x9D, 0x84, 0x41]);
        assert_eq!(
            from_utf8(&stray, V90, tcl8),
            [0x41, 0xF0, 0x9D, 0x201E, 0x41]
        );
        let replace = EncodingProfile::Replace;
        assert_eq!(
            from_utf8(&stray, V90, replace),
            [0x41, 0xFFFD, 0xFFFD, 0xFFFD, 0x41]
        );
        assert_eq!(from_utf8(&[0xC0, 0x80], V90, replace), [0xFFFD]);
        assert_eq!(from_utf8(&[0xC0, 0x80], V90, tcl8), [0]);
    }

    #[test]
    fn strict_reports_the_first_bad_byte() {
        let converted = convert_from(
            &[0x41, 0xF0, 0x9D],
            SystemEncoding::Utf8,
            V90,
            EncodingProfile::Strict,
        );
        assert_eq!(converted.value, "A");
        assert_eq!(
            failure(&converted),
            (1, "unexpected byte sequence starting at index 1: '\\xF0'")
        );
        let odd = convert_from(
            &[0x41, 0x00, 0x42],
            SystemEncoding::Unicode,
            V90,
            EncodingProfile::Strict,
        );
        assert_eq!(odd.value, "A");
        assert_eq!(failure(&odd).0, 2);
        let encoded = convert_to(
            "\0\u{E9}\u{20AC}",
            SystemEncoding::Iso88591,
            V90,
            EncodingProfile::Strict,
        );
        assert_eq!(encoded.value, [0, 0xE9]);
        let failure = encoded.failure.expect("a strict failure");
        assert_eq!(failure.index, 4);
        assert_eq!(
            failure.error.message(),
            "unexpected character at index 2: 'U+0020AC'"
        );
        assert_eq!(
            failure.error.error_code(),
            Some("TCL ENCODING ILLEGALSEQUENCE 4")
        );
    }

    #[test]
    fn table_encodings_substitute_a_question_mark() {
        let tcl8 = EncodingProfile::Tcl8;
        let encoded = convert_to("\u{E9}A\x7F", SystemEncoding::Ascii, V86, tcl8);
        assert_eq!(encoded.value, b"?A?");
        let encoded = convert_to("\u{E9}A\x7F", SystemEncoding::Ascii, V90, tcl8);
        assert_eq!(encoded.value, b"?A\x7F");
        let latin = convert_to("\u{E9}\u{20AC}", SystemEncoding::Iso88591, V86, tcl8);
        assert_eq!(latin.value, [0xE9, b'?']);
    }

    #[test]
    fn tcl8_takes_an_optional_encoding_and_tcl9_options() {
        let parsed = parse_convert_args(V86, Direction::From, &["utf-8", "x"]).expect("parses");
        assert_eq!(parsed.encoding, Some(SystemEncoding::Utf8));
        assert_eq!(parsed.data, 1);
        let error = parse_convert_args(V86, Direction::From, &["-profile", "tcl8", "utf-8", "x"])
            .expect_err("8.6 has no options");
        assert_eq!(
            error.message(),
            "wrong # args: should be \"encoding convertfrom ?encoding? data\""
        );

        let parsed = parse_convert_args(
            V90,
            Direction::To,
            &["-prof", "replace", "-failindex", "v", "ascii", "x"],
        )
        .expect("parses");
        assert_eq!(parsed.profile, EncodingProfile::Replace);
        assert_eq!(parsed.fail_var, Some(3));
        assert_eq!(parsed.encoding, Some(SystemEncoding::Ascii));
        assert_eq!(parsed.data, 5);

        let lone = parse_convert_args(V90, Direction::From, &["x"]).expect("data alone");
        assert_eq!(
            (lone.encoding, lone.profile),
            (None, EncodingProfile::Strict)
        );
        for (words, message) in [
            (
                &["-failindex", "v", "x"][..],
                "wrong # args: should be \"encoding convertfrom ?-profile profile? ?-failindex var? encoding data\" or \"encoding convertfrom data\"",
            ),
            (
                &["-bogus", "v", "x"][..],
                "bad option \"-bogus\": must be -profile or -failindex",
            ),
            (
                &["-profile", "x", "utf-8", "d"][..],
                "bad profile name \"x\": must be replace, strict, or tcl8",
            ),
            (&["-failindex", "v", "x", "d"][..], "unknown encoding \"x\""),
        ] {
            let error = parse_convert_args(V90, Direction::From, words).expect_err("rejected");
            assert_eq!(error.message(), message);
        }
    }

    #[test]
    fn text_bytes_follows_the_release_byte_policy() {
        assert_eq!(
            text_bytes("A\u{178}", ByteStringEncoding::LegacyTruncate),
            Ok(vec![b'A', 0x78])
        );
        let error =
            text_bytes("A\u{178}", ByteStringEncoding::CheckedLatin1).expect_err("above U+00FF");
        assert_eq!(
            CmdError::from(error).message(),
            "expected code point values below 0xff but value at byte offset 1 was 0x178"
        );
    }
}
