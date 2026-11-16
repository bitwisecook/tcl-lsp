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
fn fallback_http_pool_values_remain_literal_words_and_preserve_the_test_body() {
    let initial = SourceInventory::capture(
        "when HTTP_REQUEST {pool p}",
        crate::environment::store_for_dialect("f5-irules"),
    );
    let config = initial.context.as_ref().unwrap().expression_syntax().0;
    for value in [
        "",
        "plain",
        "a\"b",
        "$literal",
        "p[set ::hint 1]",
        "}",
        "a\\b",
        "a\nb",
    ] {
        let source_word = request_literal_word(value, config).unwrap();
        let source = format!("when HTTP_REQUEST {{pool {source_word}}}\nwhen HTTP_RESPONSE {{}}\n");
        let inventory =
            SourceInventory::capture(&source, crate::environment::store_for_dialect("f5-irules"));
        let context = inventory.context.as_ref().unwrap();
        let config = context.expression_syntax().0;
        let objects = extract_object_refs(&source, context);
        assert_eq!(objects.pools, [value]);
        let blocks = build_http_test_blocks(
            "literal-pool",
            &["HTTP_REQUEST".to_owned(), "HTTP_RESPONSE".to_owned()],
            &["pool".to_owned()],
            &objects,
            Some(config),
        );
        let mut observed = Vec::new();
        for block in &blocks {
            let outer = tcl_lexer::native_script_words_in(
                tcl_lexer::SourceImage::document(block),
                tcl_lexer::Span::new(0, u32::try_from(block.len()).unwrap()),
                config,
            )
            .unwrap();
            assert!(outer.fatal_tail.is_none());
            assert_eq!(outer.commands.len(), 1);
            let body_word = outer.commands[0].words.last().unwrap();
            let body =
                tcl_syntax::word_rules::original_static_word_unicode_value(body_word).unwrap();
            let body = String::from_utf8(body).unwrap();
            let inner = tcl_lexer::native_script_words_in(
                tcl_lexer::SourceImage::document(&body),
                tcl_lexer::Span::new(0, u32::try_from(body.len()).unwrap()),
                config.nested(),
            )
            .unwrap();
            assert!(inner.fatal_tail.is_none());
            for command in &inner.commands {
                if command.words.len() == 4
                    && tcl_syntax::word_rules::original_static_word_unicode_value(&command.words[1])
                        .as_deref()
                        == Some(b"pool_selected")
                {
                    let word = &command.words[3];
                    assert!(
                        word.executable_parts()
                            .all_parts()
                            .all(|part| matches!(part.part, tcl_lexer::ExecutablePart::Text(_)))
                    );
                    observed.push(
                        tcl_syntax::word_rules::original_static_word_unicode_value(word).unwrap(),
                    );
                }
            }
        }
        assert_eq!(observed, vec![value.as_bytes().to_vec(); 3]);
        let unavailable = build_http_test_blocks(
            "literal-pool",
            &["HTTP_RESPONSE".to_owned()],
            &["pool".to_owned()],
            &objects,
            None,
        );
        assert!(unavailable.iter().all(|block| !block.lines().any(|line| {
            line.trim_start()
                .starts_with("::orch::assert_that pool_selected equals")
        })));
    }
}
