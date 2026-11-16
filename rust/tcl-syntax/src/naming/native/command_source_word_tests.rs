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
use tcl_core_types::ByteCommandSlot;

#[test]
fn command_source_word_preserves_opaque_slots_and_original_channel() {
    let config =
        tcl_lexer::LexerConfig::from_grammar(tcl_dialect::grammar_of_dialect_name(Some("tcl8.6")));
    let protocol = NativeNameProtocol::C(TclVersion::V8_6);
    let first = ByteCommandSlot::new(ByteNamespacePath::root(), NameBytes::from(b"p\xed\xa0\x80"));
    let second = ByteCommandSlot::new(ByteNamespacePath::root(), NameBytes::from(b"p\xed\xa0\x81"));
    let first_word =
        native_command_source_word(protocol, &first, tcl_lexer::SourceChannel::Document, config)
            .unwrap();
    let second_word = native_command_source_word(
        protocol,
        &second,
        tcl_lexer::SourceChannel::Document,
        config,
    )
    .unwrap();
    assert_ne!(first_word, second_word);
    for (slot, word) in [(first, first_word), (second, second_word)] {
        let produced = crate::backslash::native_source_string_bytes_channel_in(
            &word.as_bytes()[1..word.len() - 1],
            tcl_lexer::SourceChannel::Document,
            config.escapes,
            protocol.string_protocol(),
        )
        .unwrap();
        assert_eq!(
            protocol
                .command_lookup_slot(NativeNameContext::root(), &produced)
                .unwrap(),
            slot
        );
    }
    let raw_zero = ByteCommandSlot::new(ByteNamespacePath::root(), NameBytes::from(b"p\0suffix"));
    assert!(
        native_command_source_word(
            protocol,
            &raw_zero,
            tcl_lexer::SourceChannel::NativeValue,
            config
        )
        .is_none()
    );
    let ambiguous_colon = ByteCommandSlot::new(
        ByteNamespacePath::from_segments([b"a:".as_slice()]),
        NameBytes::from(b"p"),
    );
    assert!(
        native_command_source_word(
            protocol,
            &ambiguous_colon,
            tcl_lexer::SourceChannel::Document,
            config
        )
        .is_none()
    );
}
