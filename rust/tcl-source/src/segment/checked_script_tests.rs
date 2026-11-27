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
use tcl_lexer::{ParseIssueKind, SourceImage};

#[test]
fn selected_braced_variable_cut_preserves_complete_prefix_and_editor_tail() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    let source = SourceImage::document("set before 1; puts ${a{b}; set after 2");
    let first_close = LexerConfig::for_dialect("tcl8.4");
    let complete = segment_script_image_with_offset_and_config(&source, 7, first_close).unwrap();
    assert_eq!(complete.commands.len(), 3);
    assert!(complete.fatal_tail.is_none());
    assert_eq!(complete.commands[0].span.start(), 7);
    let nesting = LexerConfig::for_dialect("tcl9.1");
    let script = segment_script_image_with_offset_and_config(&source, 7, nesting).unwrap();
    assert_eq!(script.commands.len(), 1);
    assert_eq!(script.commands[0].name(), "set");
    let tail = script.fatal_tail.as_ref().unwrap();
    assert_eq!(tail.cut.kind, ParseIssueKind::MissingVariableBrace);
    assert_eq!(tail.cut.command, 1);
    assert_eq!(tail.command.span.start(), 21);
    assert!(tail.command.is_partial);
    assert_eq!(tail.command.partial_delimiter, None);
    assert_eq!(script.into_editor_commands().len(), 2);
}

#[test]
fn terminal_word_error_keeps_prefix_without_inventing_an_incomplete_command() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    let image = SourceImage::document("set before 1; set invalid {a}b; set after 2");
    let config = LexerConfig::for_dialect("tcl9.1");
    let script = segment_script_image_with_offset_and_config(&image, 0, config).unwrap();
    assert_eq!(script.commands.len(), 1);
    let tail = script.fatal_tail.as_ref().unwrap();
    assert_eq!(tail.cut.kind, ParseIssueKind::ExtraAfterBrace);
    assert!(!tail.command.is_partial);
    assert_eq!(script.into_editor_commands().len(), 1);
    let strict = LexerConfig {
        strict_quoting: true,
        ..config
    };
    assert!(matches!(
        segment_script_image_with_offset_and_config(&image, 0, strict),
        Err(tcl_lexer::ParseCutUnavailable::LexicalStream(_))
    ));
}

#[test]
fn checked_f5_cut_ordinal_joins_before_if_else_lookahead() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    let image = SourceImage::document("if 1 {set a 1}\nelse {set b 2}\nputs ${abc");
    let config = LexerConfig::for_dialect("f5-irules");
    let script = segment_script_image_with_offset_and_config(&image, 0, config).unwrap();
    assert_eq!(script.commands.len(), 1);
    assert_eq!(script.commands[0].name(), "if");
    let tail = script.fatal_tail.unwrap();
    assert_eq!(tail.cut.command, 2);
    assert_eq!(tail.cut.kind, ParseIssueKind::MissingVariableBrace);
    assert_eq!(tail.command.name(), "puts");
}

#[test]
fn checked_script_refuses_opaque_structural_views_and_offset_overflow() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    let config = LexerConfig::for_dialect("tcl9.1");
    for (image, base) in [
        (SourceImage::native(b"set a \xff".as_slice()), 0),
        (SourceImage::document("set a 1"), u32::MAX),
    ] {
        assert!(matches!(
            segment_script_image_with_offset_and_config(&image, base, config),
            Err(tcl_lexer::ParseCutUnavailable::SourceGeometry(_))
        ));
    }
}
