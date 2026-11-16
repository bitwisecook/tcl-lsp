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
use tcl_lexer::{ExecutablePartArena, LexerConfig, SourceImage, Span, SubstFlags};

#[test]
fn escaped_text_extents_keep_complete_escapes_and_channel_unit_boundaries() {
    // Implementation contract: naming.source.native-string-original-extents
    // docs/design/analysis/name-resolution-proofs/native-string-original-extents.md
    use tcl_lexer::SourceChannel::{Document, NativeValue};
    for protocol in tcl_dialect::TclVersion::ALL
        .into_iter()
        .map(crate::native_string::NativeStringProtocol::C)
        .chain([crate::native_string::NativeStringProtocol::Jim084])
    {
        let raw = br"::N::v\uD800(k)";
        let escapes = protocol.escape_syntax();
        assert_eq!(
            native_source_string_extent(raw, NativeValue, escapes, protocol, 5..9),
            Some(5..12),
        );
        assert!(native_source_string_extent(raw, NativeValue, escapes, protocol, 6..7).is_none());
        assert!(native_source_string_extent(raw, NativeValue, escapes, protocol, 5..100).is_none());
        let raw = b"v\0\\uD800";
        let zero_length = native_source_literal_bytes(&raw[..2], Document, protocol)
            .unwrap()
            .len();
        assert_eq!(
            native_source_string_extent(raw, Document, escapes, protocol, 1..zero_length),
            Some(1..2),
        );
        assert_eq!(
            native_source_string_extent(raw, Document, escapes, protocol, 1..zero_length + 3),
            Some(1..raw.len()),
        );
        if zero_length == 3 {
            assert!(native_source_string_extent(raw, Document, escapes, protocol, 1..2).is_none());
        }
    }
}

#[test]
fn source_channel_changes_value_bytes_without_changing_original_extents() {
    use tcl_lexer::SourceChannel::{Document, NativeValue};
    let protocol = crate::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1);
    let config =
        LexerConfig::from_grammar(tcl_dialect::DialectProfile::find("tcl9.1").unwrap().grammar);
    let source = b"A\\\r\n\t B";
    for (channel, expected) in [
        (Document, b"A B".as_slice()),
        (NativeValue, b"A\r\n\t B".as_slice()),
    ] {
        let image = SourceImage::from_bytes(source.as_slice(), channel);
        let arena = ExecutablePartArena::decompose(
            image.clone(),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            SubstFlags::default(),
            config,
        )
        .unwrap();
        let part = &arena.list(arena.root())[0];
        assert_eq!(
            native_arena_text(&arena, part, config.escapes, protocol)
                .unwrap()
                .as_ref(),
            expected
        );
        assert_eq!(arena.image(), &image);
        assert_eq!(arena.bytes(part.span), Some(source.as_slice()));
    }
    assert_eq!(
        source_braced_word_bytes(source, Document, config.brace_backslash_newline).as_ref(),
        b"A B"
    );
    assert_eq!(
        source_braced_word_bytes(source, NativeValue, config.brace_backslash_newline).as_ref(),
        source
    );
    assert_eq!(source_literal_bytes(b"A\r\nB", Document).as_ref(), b"A\nB");
    assert_eq!(
        source_literal_bytes(b"A\r\nB", NativeValue).as_ref(),
        b"A\r\nB"
    );
}

#[test]
fn original_arena_text_keeps_raw_nul_separate_from_escaped_nul() {
    for (name, protocol, escaped) in [
        (
            "tcl8.4",
            crate::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4),
            b"\xc0\x80".as_slice(),
        ),
        (
            "tcl8.5",
            crate::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5),
            b"\xc0\x80".as_slice(),
        ),
        (
            "tcl8.6",
            crate::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6),
            b"\xc0\x80".as_slice(),
        ),
        (
            "tcl9.0",
            crate::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
            b"\xc0\x80".as_slice(),
        ),
        (
            "tcl9.1",
            crate::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1),
            b"\xc0\x80".as_slice(),
        ),
        (
            "jimtcl",
            crate::native_string::NativeStringProtocol::Jim084,
            b"\0".as_slice(),
        ),
    ] {
        let grammar = if protocol == crate::native_string::NativeStringProtocol::Jim084 {
            tcl_dialect::model::grammar(
                tcl_dialect::model::Family::Jim,
                tcl_dialect::model::Release::JIM_0_84,
            )
        } else {
            tcl_dialect::DialectProfile::find(name).unwrap().grammar
        };
        let config = LexerConfig::from_grammar(grammar);
        for (source, expected) in [
            (b"\\u0000".as_slice(), escaped),
            (b"\0".as_slice(), b"\0".as_slice()),
        ] {
            let arena = ExecutablePartArena::decompose(
                SourceImage::native(source),
                Span::new(0, u32::try_from(source.len()).unwrap()),
                SubstFlags::default(),
                config,
            )
            .unwrap();
            let part = &arena.list(arena.root())[0];
            assert_eq!(
                native_arena_text(&arena, part, config.escapes, protocol)
                    .unwrap()
                    .as_ref(),
                expected,
                "{name}: {source:?}"
            );
        }
    }
}
