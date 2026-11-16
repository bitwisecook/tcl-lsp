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

use super::*;
use crate::native_string::{NativeStringInput, NativeStringProtocol};
use tcl_dialect::TclVersion;

fn unhex(text: &str) -> Vec<u8> {
    assert_eq!(text.len() % 2, 0);
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let digit = |byte| {
                char::from(byte)
                    .to_digit(16)
                    .expect("native vector has hex bytes")
            };
            u8::try_from(digit(pair[0]) * 16 + digit(pair[1])).expect("two hex digits fit a byte")
        })
        .collect()
}

#[test]
fn source_values_match_actual_c84_through_c91_and_current_jim() {
    let protocols = [
        NativeStringProtocol::C(TclVersion::V8_4),
        NativeStringProtocol::C(TclVersion::V8_5),
        NativeStringProtocol::C(TclVersion::V8_6),
        NativeStringProtocol::C(TclVersion::V9_0),
        NativeStringProtocol::C(TclVersion::V9_1),
        NativeStringProtocol::Jim084,
    ];
    let vectors = include_str!("../../test-data/native-source-bytes.tsv");
    let mut count = 0;
    for row in vectors.lines().filter(|row| !row.starts_with('#')) {
        let columns: Vec<_> = row.split('\t').collect();
        assert_eq!(columns.len(), 8);
        let original = unhex(columns[0]);
        for (index, protocol) in protocols.into_iter().enumerate() {
            let actual =
                native_source_string_bytes_in(&original, protocol.escape_syntax(), protocol)
                    .unwrap();
            assert_eq!(
                actual.as_ref(),
                unhex(columns[index + 1]),
                "{} under {protocol:?}",
                columns[7]
            );
        }
        count += 1;
    }
    assert_eq!(count, 38);
}

#[test]
fn materialisation_source_escape_and_literal_bytes_keep_separate_inputs() {
    let protocol = NativeStringProtocol::C(TclVersion::V9_0);
    assert_eq!(
        native_source_string_bytes_in(&[0], protocol.escape_syntax(), protocol)
            .unwrap()
            .as_ref(),
        &[0]
    );
    assert_eq!(
        native_source_string_bytes_in(br"\u0000", protocol.escape_syntax(), protocol)
            .unwrap()
            .as_ref(),
        &[0xc0, 0x80]
    );
    assert_eq!(
        protocol
            .materialize(NativeStringInput::PureByteArray(&[0]))
            .unwrap()
            .as_ref(),
        &[0xc0, 0x80]
    );
    assert_eq!(
        protocol
            .materialize(NativeStringInput::ResidentString(&[0]))
            .unwrap()
            .as_ref(),
        &[0]
    );
    assert_eq!(
        native_source_string_bytes_in(br"\u0000", EscapeSyntax::Jim, protocol),
        Err(NativeSourceUnavailable::GrammarProtocolMismatch)
    );
    assert_eq!(
        native_source_escape_in(b"ordinary", 0, protocol.escape_syntax(), protocol),
        Err(NativeSourceUnavailable::InvalidEscapePosition)
    );
}

#[test]
fn c86_selected_token_decode_retains_original_literal_suffix() {
    let protocol = NativeStringProtocol::C(TclVersion::V8_6);
    let source = [b'\\', 0xf0, 0x9f, 0x98];
    let escape = native_source_escape_in(&source, 0, protocol.escape_syntax(), protocol).unwrap();
    assert_eq!(escape.end, 2);
    assert_eq!(escape.bytes, [0xc3, 0xb0]);
    assert_eq!(
        native_source_string_bytes_in(&source, protocol.escape_syntax(), protocol)
            .unwrap()
            .as_ref(),
        &[0xc3, 0xb0, 0x9f, 0x98]
    );
}
