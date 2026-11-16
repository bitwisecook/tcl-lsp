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
fn request_string_hints_decode_selected_literal_components_without_activating_them() {
    let inventory = SourceInventory::capture(
        "when HTTP_REQUEST {pool p}",
        crate::environment::store_for_dialect("f5-irules"),
    );
    let syntax = inventory.context.as_ref().unwrap().expression_syntax();
    for (operand, expected) in [
        (r#""^/api/.*$""#, "^/api/.*$"),
        (r#""prefix\$suffix""#, "prefix$suffix"),
        (r#""\x24literal""#, "$literal"),
        (r#""\"literal\"""#, "\"literal\""),
        (r"{host[set ::hint 1]}", "host[set ::hint 1]"),
        (r"{a\ b}", r"a\ b"),
    ] {
        let condition = json!({"kind":"if", "condition":format!("[HTTP::uri] eq {operand}")});
        let lines = build_request_setup("HTTP_REQUEST", &[&condition], Some(syntax));
        let line = lines.first().expect("literal request hint");
        let parsed = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(line),
            tcl_lexer::Span::new(0, u32::try_from(line.len()).unwrap()),
            syntax.0.nested(),
        )
        .unwrap();
        assert!(parsed.fatal_tail.is_none());
        assert_eq!(parsed.commands.len(), 1);
        assert_eq!(parsed.commands[0].words.len(), 5);
        let uri = &parsed.commands[0].words[4];
        assert_eq!(
            tcl_syntax::word_rules::original_static_word_unicode_value(uri).as_deref(),
            Some(expected.as_bytes()),
            "{operand}"
        );
        assert!(
            uri.executable_parts()
                .all_parts()
                .all(|part| matches!(part.part, tcl_lexer::ExecutablePart::Text(_)))
        );
    }
    for operand in [r#""prefix$dynamic""#, r#""[set ::hint 1]""#] {
        let condition = json!({"kind":"if", "condition":format!("[HTTP::uri] eq {operand}")});
        assert_eq!(
            build_request_setup("HTTP_REQUEST", &[&condition], Some(syntax)),
            ["::orch::run_http_request -host \"example.com\" -uri \"/\""]
        );
    }
}
