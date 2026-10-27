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

//! Exact original external UTF-8 constructor inputs and observation columns.
//!
//! Constructor kind is authored independently from the captured result type;
//! counted input octets come from the original INPUT row, not a reconstructed
//! source literal. Provider result primary, residency, getter bytes and public
//! error fields remain separate observations. No execution authority follows.

/// One original provider's whole immutable stdout.
#[derive(Clone, Copy)]
pub enum Column {
    /// A keyed record of the original constructor windows' table.
    Object(&'static str),
    /// A whole prefixed-offset window stdout.
    Offset(&'static str),
}

const OBJECT_CAPTURES: &str =
    include_str!("../../tcl-registry/tests/data/native_encoding_utf8_object306/captures.tsv");

impl Column {
    /// The complete original stdout bytes.
    #[must_use]
    pub fn text(self) -> String {
        match self {
            Self::Object(key) => crate::fixture_tables::text(OBJECT_CAPTURES, key),
            Self::Offset(text) => text.to_owned(),
        }
    }
}

/// One fixed original C object-vector control.
pub struct Utf8EncodingControl {
    /// Original native probe case identifier.
    pub case: &'static str,
    /// The original probe's explicit `ByteArray` constructor, not inferred from bytes.
    pub binary: bool,
    /// Whole immutable stdout for each of the five original C providers.
    pub columns: [Column; 5],
}

/// The original fifty constructor windows and five independent prefixed-offset windows.
pub const CONTROLS: &[Utf8EncodingControl] = &[
    Utf8EncodingControl {
        case: "string-eacute",
        binary: false,
        columns: [
            Column::Object("8.4.20/string-eacute/stdout"),
            Column::Object("8.5.19/string-eacute/stdout"),
            Column::Object("8.6.18/string-eacute/stdout"),
            Column::Object("9.0.4/string-eacute/stdout"),
            Column::Object("9.1.0/string-eacute/stdout"),
        ],
    },
    Utf8EncodingControl {
        case: "string-modified-zero",
        binary: false,
        columns: [
            Column::Object("8.4.20/string-modified-zero/stdout"),
            Column::Object("8.5.19/string-modified-zero/stdout"),
            Column::Object("8.6.18/string-modified-zero/stdout"),
            Column::Object("9.0.4/string-modified-zero/stdout"),
            Column::Object("9.1.0/string-modified-zero/stdout"),
        ],
    },
    Utf8EncodingControl {
        case: "string-literal-zero",
        binary: false,
        columns: [
            Column::Object("8.4.20/string-literal-zero/stdout"),
            Column::Object("8.5.19/string-literal-zero/stdout"),
            Column::Object("8.6.18/string-literal-zero/stdout"),
            Column::Object("9.0.4/string-literal-zero/stdout"),
            Column::Object("9.1.0/string-literal-zero/stdout"),
        ],
    },
    Utf8EncodingControl {
        case: "string-ff",
        binary: false,
        columns: [
            Column::Object("8.4.20/string-ff/stdout"),
            Column::Object("8.5.19/string-ff/stdout"),
            Column::Object("8.6.18/string-ff/stdout"),
            Column::Object("9.0.4/string-ff/stdout"),
            Column::Object("9.1.0/string-ff/stdout"),
        ],
    },
    Utf8EncodingControl {
        case: "byte-array-ff",
        binary: true,
        columns: [
            Column::Object("8.4.20/byte-array-ff/stdout"),
            Column::Object("8.5.19/byte-array-ff/stdout"),
            Column::Object("8.6.18/byte-array-ff/stdout"),
            Column::Object("9.0.4/byte-array-ff/stdout"),
            Column::Object("9.1.0/byte-array-ff/stdout"),
        ],
    },
    Utf8EncodingControl {
        case: "byte-array-zero",
        binary: true,
        columns: [
            Column::Object("8.4.20/byte-array-zero/stdout"),
            Column::Object("8.5.19/byte-array-zero/stdout"),
            Column::Object("8.6.18/byte-array-zero/stdout"),
            Column::Object("9.0.4/byte-array-zero/stdout"),
            Column::Object("9.1.0/byte-array-zero/stdout"),
        ],
    },
    Utf8EncodingControl {
        case: "string-surrogate-pair",
        binary: false,
        columns: [
            Column::Object("8.4.20/string-surrogate-pair/stdout"),
            Column::Object("8.5.19/string-surrogate-pair/stdout"),
            Column::Object("8.6.18/string-surrogate-pair/stdout"),
            Column::Object("9.0.4/string-surrogate-pair/stdout"),
            Column::Object("9.1.0/string-surrogate-pair/stdout"),
        ],
    },
    Utf8EncodingControl {
        case: "string-single-high-surrogate",
        binary: false,
        columns: [
            Column::Object("8.4.20/string-single-high-surrogate/stdout"),
            Column::Object("8.5.19/string-single-high-surrogate/stdout"),
            Column::Object("8.6.18/string-single-high-surrogate/stdout"),
            Column::Object("9.0.4/string-single-high-surrogate/stdout"),
            Column::Object("9.1.0/string-single-high-surrogate/stdout"),
        ],
    },
    Utf8EncodingControl {
        case: "string-astral",
        binary: false,
        columns: [
            Column::Object("8.4.20/string-astral/stdout"),
            Column::Object("8.5.19/string-astral/stdout"),
            Column::Object("8.6.18/string-astral/stdout"),
            Column::Object("9.0.4/string-astral/stdout"),
            Column::Object("9.1.0/string-astral/stdout"),
        ],
    },
    Utf8EncodingControl {
        case: "byte-array-utf8-eacute-octets",
        binary: true,
        columns: [
            Column::Object("8.4.20/byte-array-utf8-eacute-octets/stdout"),
            Column::Object("8.5.19/byte-array-utf8-eacute-octets/stdout"),
            Column::Object("8.6.18/byte-array-utf8-eacute-octets/stdout"),
            Column::Object("9.0.4/byte-array-utf8-eacute-octets/stdout"),
            Column::Object("9.1.0/byte-array-utf8-eacute-octets/stdout"),
        ],
    },
    Utf8EncodingControl {
        case: "string-eacute-single-high-surrogate",
        binary: false,
        columns: [
            Column::Offset(include_str!(
                "../../tcl-registry/tests/data/native_encoding_utf8_offset307/8.4.20/string-eacute-single-high-surrogate/stdout"
            )),
            Column::Offset(include_str!(
                "../../tcl-registry/tests/data/native_encoding_utf8_offset307/8.5.19/string-eacute-single-high-surrogate/stdout"
            )),
            Column::Offset(include_str!(
                "../../tcl-registry/tests/data/native_encoding_utf8_offset307/8.6.18/string-eacute-single-high-surrogate/stdout"
            )),
            Column::Offset(include_str!(
                "../../tcl-registry/tests/data/native_encoding_utf8_offset307/9.0.4/string-eacute-single-high-surrogate/stdout"
            )),
            Column::Offset(include_str!(
                "../../tcl-registry/tests/data/native_encoding_utf8_offset307/9.1.0/string-eacute-single-high-surrogate/stdout"
            )),
        ],
    },
];

/// Decode the exact ASCII hexadecimal observation channel.
#[must_use]
pub fn decode_hex(hex: &str) -> Vec<u8> {
    assert!(hex.len().is_multiple_of(2));
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            u8::from_str_radix(std::str::from_utf8(pair).expect("ASCII capture"), 16)
                .expect("hex capture")
        })
        .collect()
}

/// Original counted constructor data recorded before evaluation/getters.
#[must_use]
pub fn original_input(column: Column) -> Vec<u8> {
    let column = column.text();
    let input = column
        .lines()
        .find(|line| line.starts_with("INPUT|"))
        .expect("original input row");
    decode_hex(input.rsplit('|').next().expect("input bytes"))
}

/// Preserve every observed field independently of the version-reporting row.
#[must_use]
pub fn object_observation_rows(column: Column) -> Vec<String> {
    column
        .text()
        .lines()
        .filter(|line| !line.starts_with("VERSION|"))
        .map(str::to_owned)
        .collect()
}
