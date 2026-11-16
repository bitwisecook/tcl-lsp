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
use tcl_compiler::analyser::Analyser;

#[test]
fn command_completion_replaces_actual_escaped_and_delimited_words() {
    for dialect in ["tcl", "tcl8.6"] {
        for (written, extends_to_eof) in [
            (r"two\ wo<|>suffix", false),
            ("{two wo<|>suffix}", false),
            ("\"two wo<|>suffix\"", false),
            ("{two wo<|>", true),
        ] {
            const HEADER: &str = "proc {two words} {} {}; ";
            let marked = format!("{HEADER}{written} remaining");
            let cursor = marked.find("<|>").unwrap();
            let source = marked.replacen("<|>", "", 1);
            let analysis = Analyser::new().analyse(&source, dialect);
            let index = tcl_lexer::LineIndex::new(&source);
            let position = index.position_at_utf16(u32::try_from(cursor).unwrap(), &source);
            let items = completions(
                &source,
                position.line,
                position.character.get(),
                &analysis,
                None,
                None,
                analysis.resolved_profile().unwrap(),
            );
            let item = items
                .iter()
                .find(|item| item.label == "two words")
                .expect("the actual source declaration remains a candidate");
            let edit = item.text_edit.as_ref().expect("original container edit");
            let start = u32::try_from(HEADER.encode_utf16().count()).unwrap();
            let end = if extends_to_eof {
                u32::try_from(source.encode_utf16().count()).unwrap()
            } else {
                start + u32::try_from(written.replace("<|>", "").encode_utf16().count()).unwrap()
            };
            assert_eq!((edit.start_char, edit.end_char), (start, end));
            assert_eq!(edit.new_text, item.insert_text);
            assert!(edit.start_char < position.character.get());
            if !extends_to_eof {
                assert!(source[end as usize..].starts_with(" remaining"));
            }
            let mut missing = analysis.clone();
            missing.body_lexer_config = None;
            assert_eq!(
                completions(
                    &source,
                    position.line,
                    position.character.get(),
                    &missing,
                    None,
                    None,
                    analysis.resolved_profile().unwrap()
                ),
                [] as [crate::completion::CompletionItem; 0]
            );
        }
    }
}

#[test]
fn switch_completion_keeps_actual_prefix_and_complete_original_container() {
    let marked = r"lsearch \x2dno<|>suffix";
    let cursor = marked.find("<|>").unwrap();
    let source = marked.replacen("<|>", "", 1);
    let analysis = Analyser::new().analyse(&source, "tcl8.6");
    let position = tcl_lexer::LineIndex::new(&source)
        .position_at_utf16(u32::try_from(cursor).unwrap(), &source);
    let items = completions(
        &source,
        0,
        position.character.get(),
        &analysis,
        None,
        None,
        analysis.resolved_profile().unwrap(),
    );
    let edit = items
        .iter()
        .find(|item| item.label == "-nocase")
        .unwrap()
        .text_edit
        .as_ref()
        .unwrap();
    assert_eq!(edit.start_char, 8);
    assert_eq!(edit.end_char, u32::try_from(source.len()).unwrap());
    assert_eq!(edit.new_text, "-nocase");
}

#[test]
fn command_cursor_cannot_promote_data_comments_or_invalid_utf16_points() {
    for tail in ["puts {two wo<|>rds}", "# two wo<|>rds", "{*}two\\ wo<|>rds"] {
        let marked = format!("proc {{two words}} {{}} {{}}; {tail}");
        let cursor = marked.find("<|>").unwrap();
        let source = marked.replacen("<|>", "", 1);
        let analysis = Analyser::new().analyse(&source, "tcl");
        let position = tcl_lexer::LineIndex::new(&source)
            .position_at_utf16(u32::try_from(cursor).unwrap(), &source);
        assert!(
            !completions(
                &source,
                position.line,
                position.character.get(),
                &analysis,
                None,
                None,
                analysis.resolved_profile().unwrap()
            )
            .iter()
            .any(|item| item.label == "two words")
        );
    }
    let source = "proc 😀name {} {}; 😀";
    let analysis = Analyser::new().analyse(source, "tcl");
    let end = tcl_lexer::LineIndex::new(source)
        .position_at_utf16(u32::try_from(source.len()).unwrap(), source);
    assert_eq!(
        completions(
            source,
            0,
            end.character.get() - 1,
            &analysis,
            None,
            None,
            analysis.resolved_profile().unwrap()
        ),
        [] as [crate::completion::CompletionItem; 0]
    );
}
