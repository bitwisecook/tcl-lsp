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

#[test]
fn braced_continuation_correspondence_retains_separate_original_occurrences() {
    use tcl_lexer::SourceChannel::{Document, NativeValue};
    for version in tcl_dialect::TclVersion::ALL {
        let protocol = crate::native_string::NativeStringProtocol::C(version);
        let rule = tcl_dialect::BraceBackslashNewline::Folds;
        for channel in [Document, NativeValue] {
            let raw = b"a  \\\n\t b";
            let map = native_source_braced_word_map(raw, channel, rule, protocol).unwrap();
            assert_eq!(map.bytes(), b"a   b");
            assert_eq!(map.original_extent(0..3), Some(0..3));
            assert_eq!(map.original_extent(4..5), Some(7..8));
            assert_eq!(map.native_extent(7..8), Some(4..5));
            assert!(map.original_extent(2..5).is_none());
            assert!(map.original_extent(3..4).is_none());
            assert!(map.native_extent(3..7).is_none());

            let escaped = b"a\\\\\n b";
            let map = native_source_braced_word_map(escaped, channel, rule, protocol).unwrap();
            assert_eq!(map.bytes(), escaped);
            assert_eq!(
                map.original_extent(0..escaped.len()),
                Some(0..escaped.len())
            );
        }
        let raw = b"A\\\r\n\t B";
        let document = native_source_braced_word_map(raw, Document, rule, protocol).unwrap();
        assert_eq!(document.bytes(), b"A B");
        assert_eq!(document.original_extent(2..3), Some(6..7));
        assert!(document.native_extent(2..3).is_none());
        assert!(document.original_extent(0..3).is_none());
        let native = native_source_braced_word_map(raw, NativeValue, rule, protocol).unwrap();
        assert_eq!(native.bytes(), raw);
        assert_eq!(native.original_extent(0..raw.len()), Some(0..raw.len()));
    }
    let raw = b"a\\\n b";
    let literal = native_source_braced_word_map(
        raw,
        NativeValue,
        tcl_dialect::BraceBackslashNewline::Literal,
        crate::native_string::NativeStringProtocol::Jim084,
    )
    .unwrap();
    assert_eq!(literal.bytes(), raw);
    assert_eq!(literal.original_extent(0..raw.len()), Some(0..raw.len()));
}

#[test]
fn braced_correspondence_refuses_partial_native_units_and_foreign_ranges() {
    for version in tcl_dialect::TclVersion::ALL {
        let protocol = crate::native_string::NativeStringProtocol::C(version);
        let map = native_source_braced_word_map(
            b"a\0b\\\n c",
            tcl_lexer::SourceChannel::Document,
            tcl_dialect::BraceBackslashNewline::Folds,
            protocol,
        )
        .unwrap();
        let zero = native_source_literal_bytes(b"\0", tcl_lexer::SourceChannel::Document, protocol)
            .unwrap()
            .len();
        assert_eq!(map.original_extent(1..1 + zero), Some(1..2));
        if zero > 1 {
            assert!(map.original_extent(1..2).is_none());
        }
        assert!(map.original_extent(0..map.bytes().len() + 1).is_none());
        assert!(map.native_extent(0..100).is_none());
    }
    assert!(
        native_source_braced_word_map(
            &[0xff],
            tcl_lexer::SourceChannel::Document,
            tcl_dialect::BraceBackslashNewline::Folds,
            crate::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
        )
        .is_err()
    );
}

#[test]
fn source_literal_extents_preserve_ascii_and_unicode_boundaries() {
    use tcl_lexer::SourceChannel::Document;

    assert_eq!(source_literal_extent(b"scalar", Document, 1..4), Some(1..4));
    let source = "café🙂".as_bytes();
    assert_eq!(source_literal_bytes(source, Document).as_ref(), source);
    assert_eq!(source_literal_extent(source, Document, 0..9), Some(0..9));
    assert_eq!(source_literal_extent(source, Document, 3..5), Some(3..5));
    assert_eq!(source_literal_extent(source, Document, 5..9), Some(5..9));
    assert_eq!(source_literal_extent(source, Document, 5..5), Some(5..5));
    assert!(source_literal_extent(source, Document, 3..4).is_none());
    assert!(source_literal_extent(source, Document, 4..5).is_none());
    assert!(source_literal_extent(source, Document, 6..6).is_none());
    assert!(source_literal_extent(source, Document, 0..10).is_none());
    assert!(source_literal_extent(source, Document, 10..10).is_none());
    assert!(
        source_literal_extent(source, Document, std::ops::Range { start: 5, end: 3 }).is_none()
    );
}

#[test]
fn source_literal_extents_share_exact_document_newline_translation() {
    use tcl_lexer::SourceChannel::Document;

    let source = b"a\r\nb\rc";
    let value = source_literal_bytes(source, Document);
    assert_eq!(value.as_ref(), b"a\nb\nc");
    for (literal, original) in [
        (0..5, 0..6),
        (1..2, 1..3),
        (2..3, 3..4),
        (3..4, 4..5),
        (5..5, 6..6),
    ] {
        assert_eq!(
            source_literal_extent(source, Document, literal.clone()),
            Some(original.clone())
        );
        assert_eq!(
            source_literal_bytes(&source[original], Document).as_ref(),
            &value[literal]
        );
    }
    assert_eq!(source_literal_extent(b"\r", Document, 0..1), Some(0..1));
    assert_eq!(source_literal_extent(b"\r\n", Document, 0..1), Some(0..2));
    assert!(source_literal_extent(b"\r\n", Document, 0..2).is_none());
    assert_eq!(source_literal_extent(b"\\r\\n", Document, 1..3), Some(1..3));
    let unicode = "a\r\ncafé🙂".as_bytes();
    assert_eq!(source_literal_extent(unicode, Document, 2..11), Some(3..12));
    assert!(source_literal_extent(unicode, Document, 6..7).is_none());
}

#[test]
fn source_literal_extents_retain_bom_and_validate_empty_document_ranges() {
    use tcl_lexer::SourceChannel::Document;

    let source = "\u{feff}café🙂\r\n".as_bytes();
    assert_eq!(
        source_literal_bytes(source, Document).as_ref(),
        "\u{feff}café🙂\n".as_bytes()
    );
    assert_eq!(source_literal_extent(source, Document, 0..3), Some(0..3));
    assert_eq!(source_literal_extent(source, Document, 3..12), Some(3..12));
    assert_eq!(source_literal_extent(source, Document, 0..13), Some(0..14));
    assert!(source_literal_extent(source, Document, 0..1).is_none());
    assert_eq!(source_literal_extent(b"", Document, 0..0), Some(0..0));
    assert!(source_literal_extent(b"", Document, 0..1).is_none());
    assert!(source_literal_extent(b"\xff", Document, 0..0).is_none());
    assert!(source_literal_extent(b"\xff", Document, 0..1).is_none());
}

#[test]
fn source_literal_extents_keep_native_channel_counted_bytes() {
    use tcl_lexer::SourceChannel::NativeValue;

    let source = b"\xff\r\n\xc3\xa9";
    assert_eq!(source_literal_bytes(source, NativeValue).as_ref(), source);
    assert_eq!(source_literal_extent(source, NativeValue, 0..5), Some(0..5));
    assert_eq!(source_literal_extent(source, NativeValue, 1..2), Some(1..2));
    assert_eq!(source_literal_extent(source, NativeValue, 3..4), Some(3..4));
    assert_eq!(source_literal_extent(source, NativeValue, 5..5), Some(5..5));
    assert!(source_literal_extent(source, NativeValue, 0..6).is_none());
    assert!(
        source_literal_extent(source, NativeValue, std::ops::Range { start: 2, end: 1 }).is_none()
    );
}

#[test]
fn source_literal_extents_do_not_relax_native_unit_mapping() {
    use tcl_lexer::SourceChannel::Document;

    let source = "café🙂".as_bytes();
    let protocol = crate::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6);
    assert_eq!(source_literal_extent(source, Document, 0..9), Some(0..9));
    assert_eq!(
        native_source_literal_bytes(source, Document, protocol)
            .unwrap()
            .len(),
        11
    );
    assert!(native_source_literal_extent(source, Document, protocol, 0..9).is_none());
    assert_eq!(
        native_source_literal_extent(source, Document, protocol, 5..11),
        Some(5..9)
    );
}

#[test]
fn original_literal_extents_share_channel_translation_and_exact_unit_boundaries() {
    use tcl_lexer::SourceChannel::{Document, NativeValue};
    let source = "x\0😀\r\ny".as_bytes();
    for version in tcl_dialect::TclVersion::ALL {
        let protocol = crate::native_string::NativeStringProtocol::C(version);
        let value = native_source_literal_bytes(source, Document, protocol).unwrap();
        assert_eq!(
            native_source_literal_extent(source, Document, protocol, 1..3),
            Some(1..2)
        );
        assert!(native_source_literal_extent(source, Document, protocol, 1..2).is_none());
        let emoji_start = 3;
        let emoji_end = native_source_literal_bytes("x\0😀".as_bytes(), Document, protocol)
            .unwrap()
            .len();
        assert_eq!(
            native_source_literal_extent(source, Document, protocol, emoji_start..emoji_end),
            Some(2..6)
        );
        assert!(
            native_source_literal_extent(source, Document, protocol, emoji_start..emoji_start + 1)
                .is_none()
        );
        assert_eq!(
            native_source_literal_extent(source, Document, protocol, emoji_end..emoji_end + 1),
            Some(6..8)
        );
        assert!(
            native_source_literal_extent(source, Document, protocol, 0..value.len() + 1).is_none()
        );
        assert_eq!(
            native_source_literal_extent(source, NativeValue, protocol, 1..2),
            Some(1..2)
        );
    }
}
