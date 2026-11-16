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
fn actual_method_word_and_parent_argv_own_configured_proposal() {
    let source = "oo::class create Dog {method bark {} {return {}}}\n[Dog new] {bakr}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let method = u32::try_from(source.rfind("bakr").unwrap()).unwrap();
    let head = u32::try_from(source.rfind("[Dog new]").unwrap()).unwrap();
    let site = W308DiagnosticSite {
        reach: tcl_registry::definer::MethodReach::ObjectCommand,
        method_span: Some(tcl_lexer::Span::new(method, method + 4)),
        cmd_span: tcl_lexer::Span::new(head, head + 9),
    };
    let (span, proposal) = analyser
        .w308_configured_method_proposal(site, "$name [not_a_call]")
        .unwrap();
    assert_eq!(source.get(span.as_range()), Some("{bakr}"));
    assert!(tcl_syntax::word_rules::lexical_unicode_source_word_matches(
        &proposal,
        "$name [not_a_call]",
        analyser.file_lexer_config()
    ));
    let foreign_parent = W308DiagnosticSite {
        cmd_span: tcl_lexer::Span::new(0, 9),
        ..site
    };
    assert!(
        analyser
            .w308_configured_method_proposal(foreign_parent, "bark")
            .is_none()
    );
    let source = "oo::class create Dog {method bark {} {return {}}}\n[Dog new] bakr";
    let result = Analyser::new().analyse_per_item(source, "tcl");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::W308)
        .unwrap();
    assert_eq!(diagnostic.fixes[0].new_text, "bark");
    assert_eq!(diagnostic.fixes[0].span, diagnostic.span);
}
