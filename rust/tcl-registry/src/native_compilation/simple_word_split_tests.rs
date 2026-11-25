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
use crate::native_compiler_words::NativeCompilerWords;
use crate::native_string_compilation::{NativeStringMatchOperation, NativeStringMatchScope};

fn words(source: &[u8], version: TclVersion) -> Vec<tcl_lexer::NativeWord> {
    let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
    tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::native(source),
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    )
    .unwrap()
    .commands
    .remove(0)
    .words
}

// tclsh 9.0.4 and 8.6.18 `disassemble lambda {s} ...`: `string match W $s`
// gives streq only for a SIMPLE W, and `switch -exact -- $s W {..} b {..}`
// compiles a jumpTable only for a SIMPLE W (otherwise invokeStk).
#[test]
fn a_dollar_beside_text_is_not_a_simple_word() {
    for version in [TclVersion::V8_6, TclVersion::V9_0] {
        let protocol = tcl_syntax::native_string::NativeStringProtocol::C(version);
        for (word, simple) in [
            ("a$", false),
            ("\"a$\"", false),
            ("$$", false),
            ("\"$$\"", false),
            ("$", true),
            ("\"$\"", true),
            ("{a$}", true),
        ] {
            let source = format!("string match {word} $s");
            let parsed = words(source.as_bytes(), version);
            let captured = NativeCompilerWords::capture(&parsed, protocol).unwrap();
            let recipe = crate::native_string_compilation::instruction(
                &captured,
                1,
                NativeStringMatchScope::PublicMember,
                version,
            )
            .unwrap();
            assert_eq!(
                recipe.operation == NativeStringMatchOperation::Equal,
                simple,
                "{version:?} {source}"
            );
            let source = format!("switch -exact -- $s {word} {{return 1}} b {{return 2}}");
            let parsed = words(source.as_bytes(), version);
            let captured = NativeCompilerWords::capture(&parsed, protocol).unwrap();
            assert_eq!(
                crate::native_switch_compilation::native_switch_instruction(&captured, 1, version)
                    .is_ok(),
                simple,
                "{version:?} {source}"
            );
        }
    }
}

#[test]
fn text_token_splits_follow_parse_tokens() {
    for (value, split) in [
        (b"a$".as_slice(), true),
        (b"$$", true),
        (b"$", false),
        (b"a\0", false),
        (b"abc", false),
        (b"", false),
    ] {
        assert_eq!(splits_into_text_tokens(value), split, "{value:?}");
    }
}
