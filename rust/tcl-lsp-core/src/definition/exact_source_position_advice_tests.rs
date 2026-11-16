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
fn exact_current_cursor_keeps_unicode_endline_and_declines_clamped_coordinates() {
    let source = "proc café {} {}; café\nset emoji 😀";
    let analysis = Analyser::new().analyse(source, "tcl");
    let valid = CurrentSourcePosition::capture(source, &analysis, 0, 18).unwrap();
    assert_eq!(
        &source[usize::try_from(valid.offset()).unwrap()..],
        "afé\nset emoji 😀"
    );
    let index = LineIndex::new_lsp(source);
    let end = index.position_at_utf16(u32::try_from(source.find('\n').unwrap()).unwrap(), source);
    assert!(
        CurrentSourcePosition::capture(source, &analysis, end.line, end.character.get()).is_some()
    );
    assert!(
        CurrentSourcePosition::capture(source, &analysis, 1, 11).is_none(),
        "surrogate interior"
    );
    assert!(
        CurrentSourcePosition::capture(source, &analysis, 0, end.character.get() + 1).is_none()
    );
    assert!(CurrentSourcePosition::capture(source, &analysis, 20, 0).is_none());
    assert_eq!(
        definition(source, 1, 11, &analysis),
        [] as [crate::definition::LspRange; 0]
    );
    assert_eq!(
        definition(source, 0, u32::MAX, &analysis),
        [] as [crate::definition::LspRange; 0]
    );
    assert_eq!(
        definition(source, u32::MAX, 0, &analysis),
        [] as [crate::definition::LspRange; 0]
    );
    assert_ne!(
        definition(source, 0, 18, &analysis),
        [] as [crate::definition::LspRange; 0]
    );
}

#[test]
fn exact_source_cursor_withdraws_stale_input_and_full_grammar_changes() {
    let source = "proc p {} {}; p";
    let mut analysis = Analyser::new().analyse(source, "tcl");
    assert!(CurrentSourcePosition::capture(source, &analysis, 0, 14).is_some());
    assert!(CurrentSourcePosition::capture("proc q {} {}; q", &analysis, 0, 14).is_none());
    let config = analysis.body_lexer_config.unwrap();
    analysis.body_lexer_config.as_mut().unwrap().strict_quoting = !config.strict_quoting;
    assert!(CurrentSourcePosition::capture(source, &analysis, 0, 14).is_none());
    assert_eq!(
        definition(source, 0, 14, &analysis),
        [] as [crate::definition::LspRange; 0]
    );
    analysis.body_lexer_config = Some(config);
    analysis.resolved_input = None;
    assert!(CurrentSourcePosition::capture(source, &analysis, 0, 14).is_none());
}

#[test]
fn source_scope_visibility_requires_original_body_correspondence_before_document_walk() {
    let source = "set top 1; proc p {} {global top; return $top}; p";
    let mut analysis = Analyser::new().analyse(source, "tcl");
    let body_cursor = u32::try_from(source.find("return").unwrap()).unwrap();
    let root = CurrentSourcePosition::capture(source, &analysis, 0, 0).unwrap();
    assert!(
        matches!(scope_body_spans_at(&root), Some(SourceScopeBodySpans::Document(span)) if span == tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()))
    );
    let body = CurrentSourcePosition::capture(source, &analysis, 0, body_cursor).unwrap();
    assert!(
        matches!(scope_body_spans_at(&body), Some(SourceScopeBodySpans::Bodies(spans)) if spans == vec![analysis.global_scope.children[0].body_span.unwrap()])
    );
    let original = analysis.global_scope.children[0].body_span;
    let child = analysis.global_scope.children.remove(0);
    let removed = CurrentSourcePosition::capture(source, &analysis, 0, body_cursor).unwrap();
    assert!(
        scope_body_spans_at(&removed).is_none(),
        "absent scope cannot borrow the enclosing document"
    );
    analysis.global_scope.children.insert(0, child);
    analysis.global_scope.children[0].body_span = None;
    let missing = CurrentSourcePosition::capture(source, &analysis, 0, body_cursor).unwrap();
    assert!(
        scope_body_spans_at(&missing).is_none(),
        "missing body cannot mean whole file"
    );
    analysis.global_scope.children[0].body_span =
        Some(tcl_lexer::Span::new(body_cursor, body_cursor + 3));
    let foreign = CurrentSourcePosition::capture(source, &analysis, 0, body_cursor).unwrap();
    assert!(
        scope_body_spans_at(&foreign).is_none(),
        "an arbitrary valid substring is no original body owner"
    );
    analysis.global_scope.children[0].body_span = original;
    let restored = CurrentSourcePosition::capture(source, &analysis, 0, body_cursor).unwrap();
    assert!(matches!(
        scope_body_spans_at(&restored),
        Some(SourceScopeBodySpans::Bodies(_))
    ));
}
#[test]
fn exact_current_cursor_preserves_crlf_multibyte_and_empty_final_line() {
    let source = "p😀é\r\n";
    let analysis = Analyser::new().analyse(source, "tcl");
    for (line, column, byte) in [(0, 0, 0), (0, 1, 1), (0, 3, 5), (0, 4, 7), (1, 0, 9)] {
        assert_eq!(
            CurrentSourcePosition::capture(source, &analysis, line, column)
                .unwrap()
                .offset(),
            byte
        );
    }
    for (line, column) in [(0, 2), (0, 5), (1, 1), (2, 0)] {
        assert!(CurrentSourcePosition::capture(source, &analysis, line, column).is_none());
    }
}

#[test]
fn declaration_caller_uses_exact_source_cursor_before_any_fallback() {
    let source = "proc p {} {}; p";
    let analysis = Analyser::new().analyse(source, "tcl");
    let profile = analysis.resolved_profile().unwrap();
    let registry = analysis.resolved_registry().unwrap();
    assert_ne!(
        crate::declaration::declaration(source, 0, 14, profile, &analysis, registry),
        [] as [crate::definition::LspRange; 0]
    );
    assert_eq!(
        crate::declaration::declaration(source, 0, 100, profile, &analysis, registry),
        [] as [crate::definition::LspRange; 0]
    );
    assert_eq!(
        crate::declaration::declaration(source, 2, 0, profile, &analysis, registry),
        [] as [crate::definition::LspRange; 0]
    );
}
