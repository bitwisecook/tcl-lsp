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
fn quoted_fragments_preserve_literal_characters_and_original_substitutions() {
    // naming.diagnostics.original-w216-literal-source-replacement
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-w216-literal-source-replacement.md
    for escapes in EscapeSyntax::ALL {
        let config = tcl_lexer::LexerConfig {
            escapes: *escapes,
            ..Default::default()
        };
        for text in ["$arr", "a\"b", "a[b]", "a\\b", "é", "a\nb"] {
            let fragment = literal_quoted_source_fragment(text, *escapes).unwrap();
            assert_eq!(decode_in(&fragment, *escapes).as_ref(), text);
        }
        assert_eq!(
            substituting_quoted_source_fragment("$key\"x\"", config).as_deref(),
            Some("$key\\\"x\\\"")
        );
        assert_eq!(
            substituting_quoted_source_fragment("[format \"%s\" $key]", config).as_deref(),
            Some("[format \"%s\" $key]")
        );
        assert!(substituting_quoted_source_fragment("[unterminated", config).is_none());
        assert!(substituting_quoted_source_fragment("trailing\\", config).is_none());
    }
}

#[test]
fn literal_source_renderer_preserves_selected_native_bytes_and_channel() {
    // Implementation contract: naming.source.native-literal-word-renderer
    // docs/design/analysis/name-resolution-proofs/native-literal-word-renderer.md
    use tcl_lexer::SourceChannel::{Document, NativeValue};
    for version in tcl_dialect::TclVersion::ALL {
        let protocol = crate::native_string::NativeStringProtocol::C(version);
        let config = tcl_lexer::LexerConfig {
            escapes: protocol.escape_syntax(),
            ..tcl_lexer::LexerConfig::default()
        };
        for channel in [Document, NativeValue] {
            for value in [
                b"plain".as_slice(),
                b"a $[x]\\\"\n\r\ttail",
                b"p\xc0\x80tail",
                b"p\xed\xa0\x80",
                b"p\xed\xa0\x81",
            ] {
                let word = native_literal_source_word(value, channel, config, protocol)
                    .expect("exact selected spelling");
                assert_eq!(
                    native_source_string_bytes_channel_in(
                        &word.as_bytes()[1..word.len() - 1],
                        channel,
                        config.escapes,
                        protocol
                    )
                    .unwrap()
                    .as_ref(),
                    value
                );
            }
        }
        assert!(native_literal_source_word(b"p\0tail", Document, config, protocol).is_none());
        let counted =
            native_literal_source_word(b"p\0tail", NativeValue, config, protocol).unwrap();
        assert!(counted.as_bytes().contains(&0));
        assert!(native_literal_source_word(b"\xff", NativeValue, config, protocol).is_none());
    }
    let protocol = crate::native_string::NativeStringProtocol::Jim084;
    let config = tcl_lexer::LexerConfig::for_dialect("jimtcl");
    assert!(native_literal_source_word(b"p\0tail", Document, config, protocol).is_some());
    assert!(native_literal_source_word(b"\xff", NativeValue, config, protocol).is_none());
}

fn ingress_observed_bytes(
    version: tcl_dialect::TclVersion,
    path: &str,
    operation: &str,
) -> Vec<u8> {
    let release = match version {
        tcl_dialect::TclVersion::V8_4 => "8.4.20",
        tcl_dialect::TclVersion::V8_5 => "8.5.19",
        tcl_dialect::TclVersion::V8_6 => "8.6.18",
        tcl_dialect::TclVersion::V9_0 => "9.0.4",
        tcl_dialect::TclVersion::V9_1 => "9.1.0",
    };
    let row = include_str!("../../tests/data/native_source_ingress/observations.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| line.split('\t').collect::<Vec<_>>())
        .find(|row| row[..3] == [release, path, operation])
        .expect("actual native row");
    row[3]
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn document_native_ingress_matches_readchars_and_preserves_counted_source() {
    // Native proof: naming.source.document-native-utf-ingress
    // docs/design/analysis/name-resolution-proofs/document-native-utf-ingress.md
    use tcl_lexer::SourceChannel::{Document, NativeValue};
    let source = include_bytes!("../../tests/data/native_source_ingress/source.tcl");
    for version in tcl_dialect::TclVersion::ALL {
        let protocol = crate::native_string::NativeStringProtocol::C(version);
        assert_eq!(
            native_source_literal_bytes(source, Document, protocol)
                .unwrap()
                .as_ref(),
            ingress_observed_bytes(version, "character-channel", "source")
        );
        assert_eq!(
            native_source_literal_bytes(source, NativeValue, protocol)
                .unwrap()
                .as_ref(),
            source.as_slice()
        );
        for (operation, raw) in [
            ("plain", "A\0😀\r\nB\rC".as_bytes()),
            ("escaped", "\\😀".as_bytes()),
            ("numeric", br"A\u0000\uD83D\uDE00".as_slice()),
            ("slashzero", b"\\\0tail".as_slice()),
        ] {
            for (channel, path) in [
                (Document, "character-channel-source"),
                (NativeValue, "counted-native-source"),
            ] {
                assert_eq!(
                    native_source_string_bytes_channel_in(
                        raw,
                        channel,
                        protocol.escape_syntax(),
                        protocol
                    )
                    .unwrap()
                    .as_ref(),
                    ingress_observed_bytes(version, path, operation),
                    "{version:?} {channel:?} {operation}"
                );
            }
        }
        assert_eq!(
            native_source_braced_word_bytes(
                "A\0😀\r\nB\rC".as_bytes(),
                Document,
                tcl_dialect::BraceBackslashNewline::Folds,
                protocol
            )
            .unwrap()
            .as_ref(),
            ingress_observed_bytes(version, "character-channel-source", "braced")
        );
        assert_eq!(
            native_source_literal_bytes(b"\xff", Document, protocol),
            Err(NativeSourceUnavailable::InvalidDocumentUnicode)
        );
        assert_eq!(
            native_source_literal_bytes(b"\xff", NativeValue, protocol)
                .unwrap()
                .as_ref(),
            b"\xff"
        );
        let fragment = native_source_escape_channel_in(
            b"A\\\0tail",
            1,
            Document,
            protocol.escape_syntax(),
            protocol,
        )
        .unwrap();
        assert_eq!(
            fragment.end, 3,
            "original extent must not become native byte extent"
        );
        assert_eq!(fragment.bytes, b"\xc0\x80");
    }
}

#[test]
fn literal_text_inverse_roundtrips_actual_document_ingress_units() {
    // Implementation contract: naming.source.literal-text-source-inversion
    // docs/design/analysis/name-resolution-proofs/literal-text-source-inversion.md
    // Native proof: naming.source.document-native-utf-ingress
    // docs/design/analysis/name-resolution-proofs/document-native-utf-ingress.md
    // The retained guest observations provide the units. This separately
    // asserts source-renderer inversion, without an execution/name grant.
    use tcl_lexer::SourceChannel::{Document, NativeValue};
    for version in tcl_dialect::TclVersion::ALL {
        let protocol = crate::native_string::NativeStringProtocol::C(version);
        let actual = ingress_observed_bytes(version, "character-channel-source", "plain");
        let text = native_literal_source_text(&actual, Document, protocol).unwrap();
        assert_eq!(text, "A\0😀\nB\nC", "{version:?}");
        assert_eq!(
            native_source_literal_bytes(text.as_bytes(), Document, protocol)
                .unwrap()
                .as_ref(),
            actual
        );
        assert!(native_literal_source_text(b"\xff", Document, protocol).is_none());
        assert!(native_literal_source_text(b"\xc0\x80", NativeValue, protocol).is_none());
        if version < tcl_dialect::TclVersion::V9_0 {
            assert!(native_literal_source_text(b"\xed\xa0\x80", Document, protocol).is_none());
        }
    }
}

#[test]
fn brace_continuations_collapse_like_tcl9() {
    // `\<newline>` plus following spaces/tabs ⇒ one space.
    assert_eq!(&*collapse_brace_continuations(b"a\\\nb"), b"a b");
    assert_eq!(&*collapse_brace_continuations(b"a\\\n   b"), b"a b");
    assert_eq!(&*collapse_brace_continuations(b"a\\\n\t b"), b"a b");
    // TclParseBackslash has no CR arm: raw CR and CRLF stay byte-exact.
    assert_eq!(&*collapse_brace_continuations(b"a\\\rb"), b"a\\\rb");
    assert_eq!(
        &*collapse_brace_continuations(b"a\\\r\n   b"),
        b"a\\\r\n   b"
    );
    assert_eq!(&*collapse_brace_continuations(b"a\\\r\tb"), b"a\\\r\tb");
    // No continuation ⇒ borrowed, byte-identical (including a literal `\n`).
    assert!(matches!(
        collapse_brace_continuations(b"p\\nq"),
        Cow::Borrowed(_)
    ));
    // An escaped backslash before a newline is not a continuation.
    assert_eq!(&*collapse_brace_continuations(b"x\\\\\ny"), b"x\\\\\ny");
    assert_eq!(&*collapse_brace_continuations(b"x\\\\\r\ny"), b"x\\\\\r\ny");
    // Two continuations in a row ⇒ two spaces (each `\<eol>` → one space).
    assert_eq!(&*collapse_brace_continuations(b"m\\\n\\\nn"), b"m  n");
}

#[test]
fn brace_continuations_str_matches_bytes_and_borrows() {
    assert_eq!(&*collapse_brace_continuations_str("a\\\n   b"), "a b");
    assert_eq!(&*collapse_brace_continuations_str("a\\\r\nb"), "a\\\r\nb");
    // No continuation ⇒ borrowed (no allocation), including a literal `\t`.
    assert!(matches!(
        collapse_brace_continuations_str("a\\tb"),
        Cow::Borrowed(_)
    ));
}

#[test]
fn separator_continuations_trim_preceding_whitespace() {
    // Word-separator context: the whole `<ws>\<eol><ws>` run is ONE
    // separator, so the spaces/tabs before the backslash collapse too —
    // for LF. Raw CR/CRLF are ordinary escaped data.
    assert_eq!(&*collapse_separator_continuations(b"a \\\n  b"), b"a b");
    assert_eq!(
        &*collapse_separator_continuations(b"a\t \\\r\n\t b"),
        b"a\t \\\r\n\t b"
    );
    assert_eq!(
        &*collapse_separator_continuations(b"a \\\r  b"),
        b"a \\\r  b"
    );
    // Contrast: the brace/quote rule keeps the preceding space as data.
    assert_eq!(&*collapse_brace_continuations(b"a \\\n  b"), b"a  b");
    // FP guards: no continuation borrows unchanged (preceding whitespace
    // untouched), and an escaped backslash is not a continuation.
    assert!(matches!(
        collapse_separator_continuations(b"a  b"),
        Cow::Borrowed(_)
    ));
    assert_eq!(
        &*collapse_separator_continuations(b"x \\\\\ny"),
        b"x \\\\\ny"
    );
}

#[test]
fn separator_continuations_str_matches_bytes() {
    assert_eq!(
        &*collapse_separator_continuations_str("a \\\r\n b"),
        "a \\\r\n b"
    );
    assert!(matches!(
        collapse_separator_continuations_str("a\\tb"),
        Cow::Borrowed(_)
    ));
}

#[test]
fn bytes_decode_matches_tcl9() {
    assert_eq!(&*decode_bytes(b"a\\tb"), b"a\tb");
    // `\xff` is U+00FF ⇒ two UTF-8 bytes (the old bs.rs emitted one raw byte).
    assert_eq!(&*decode_bytes(b"\\xff"), "\u{FF}".as_bytes());
    assert_eq!(&*decode_bytes(b"\\u00e9"), "é".as_bytes());
    // no backslash ⇒ borrowed, byte-identical.
    assert!(matches!(decode_bytes(b"plain"), Cow::Borrowed(_)));
}

#[test]
fn decode_control_escapes_match_tclsh() {
    // Codepoints verified against tclsh8.6/9.0 (`scan "\X" %c`):
    // \a=7 \b=8 \f=12 \n=10 \r=13 \t=9 \v=11 \\=92.
    assert_eq!(&*decode("\\a"), "\u{07}");
    assert_eq!(&*decode("\\b"), "\u{08}");
    assert_eq!(&*decode("\\f"), "\u{0c}");
    assert_eq!(&*decode("\\n"), "\n");
    assert_eq!(&*decode("\\r"), "\r");
    assert_eq!(&*decode("\\t"), "\t");
    assert_eq!(&*decode("\\v"), "\u{0b}");
    assert_eq!(&*decode("\\\\"), "\\");
}

#[test]
fn decode_numeric_escapes_match_tclsh() {
    // tclsh: \x41=A (exactly 2 hex), \101=A (octal), A=A (1-4 hex).
    assert_eq!(&*decode("\\x41"), "A");
    assert_eq!(&*decode("\\101"), "A");
    assert_eq!(&*decode("\\u0041"), "A");
    // \x consumes exactly two hex digits, so `\x4142` → "A42" (tclsh: the
    // string-length is 3, not 1).
    assert_eq!(&*decode("\\x4142"), "A42");
    // \xff → U+00FF (Tcl 9 / UTF-8 internal rep), not a raw 0xFF byte.
    assert_eq!(&*decode("\\xff"), "\u{FF}");
    // \u with fewer than 4 hex digits still decodes (\u41 → A).
    assert_eq!(&*decode("\\u41"), "A");
}

#[test]
fn decode_bytes_follows_the_release() {
    // The user-visible symptom of issue #1479: a script pinned to 8.4/8.5
    // reads `\x4142` as `B` (all trailing hex digits, low byte), 8.6+ as
    // `A42` (TIP 388's two-digit cap). `\U` exists only from 8.6, and 8.6's
    // stock UTF-16-internal build degrades an astral scalar to U+FFFD.
    assert_eq!(&*decode_bytes_in(b"\\x4142", EscapeSyntax::Tcl84), b"B");
    assert_eq!(&*decode_bytes_in(b"\\x4142", EscapeSyntax::Tcl86), b"A42");
    assert_eq!(&*decode_bytes_in(b"\\x4142", EscapeSyntax::Tcl90), b"A42");
    assert_eq!(
        &*decode_bytes_in(b"\\U0001F600", EscapeSyntax::Tcl84),
        b"U0001F600"
    );
    assert_eq!(
        &*decode_bytes_in(b"\\U0001F600", EscapeSyntax::Tcl86),
        "\u{FFFD}".as_bytes()
    );
    assert_eq!(
        &*decode_bytes_in(b"\\U0001F600", EscapeSyntax::Tcl90),
        "\u{1F600}".as_bytes()
    );
    // The release-blind entry point is the 9.0 one.
    assert_eq!(
        &*decode_bytes(b"\\x4142"),
        &*decode_bytes_in(b"\\x4142", EscapeSyntax::Tcl90)
    );
}

#[test]
fn escape_extent_follows_the_release() {
    // Width and value must agree per release, or a per-escape scanner
    // slices mid-escape.
    assert_eq!(escape_end_in("\\x4142", 0, EscapeSyntax::Tcl84), 6);
    assert_eq!(escape_end_in("\\x4142", 0, EscapeSyntax::Tcl90), 4);
    assert_eq!(escape_end_in("\\U0001F600", 0, EscapeSyntax::Tcl84), 2);
    assert_eq!(escape_end_in("\\U0001F600", 0, EscapeSyntax::Tcl90), 10);
    assert_eq!(
        escape_end("\\x4142", 0),
        escape_end_in("\\x4142", 0, EscapeSyntax::Tcl90)
    );
}

#[test]
fn decode_unknown_escape_drops_backslash() {
    // tclsh: an unknown escape keeps the character (`\q` → `q`).
    assert_eq!(&*decode("\\q"), "q");
    // Escaped quote/brace are literal.
    assert_eq!(&*decode("\\\""), "\"");
    assert_eq!(&*decode("\\{"), "{");
}

#[test]
fn decode_no_backslash_borrows() {
    // Nothing to decode ⇒ a borrowed slice (no allocation).
    assert!(matches!(decode("plain text"), Cow::Borrowed(_)));
}

#[test]
fn decode_bytes_passes_through_non_utf8() {
    // A non-UTF-8 byte slice cannot be a well-formed internal rep, but the
    // defensive branch borrows it unchanged rather than panicking.
    let raw = [0xff, 0xfe, b'a'];
    assert!(matches!(decode_bytes(&raw), Cow::Borrowed(_)));
    assert_eq!(&*decode_bytes(&raw), &raw);
}
