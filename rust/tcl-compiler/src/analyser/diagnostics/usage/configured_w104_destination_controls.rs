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
fn original_append_uses_conditional_stock_destination_and_configured_words() {
    let comment = Analyser::new().analyse("append value \" #item\"", "tcl8.6");
    let diagnostic = comment
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::W104)
        .unwrap();
    let [fix] = diagnostic.fixes.as_slice() else {
        panic!("literal operand is not a comment head")
    };
    assert!(fix.new_text.starts_with("lappend value "));
    assert!(fix.new_text.contains("#item"));
    for logical in [false, true] {
        let profile = if logical {
            tcl_dialect::DialectProfile::plain_tcl()
        } else {
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap()
        };
        let mut config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        config.strict_quoting = !config.strict_quoting;
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            config,
        );
        let result = Analyser::new()
            .with_resolved_input(input)
            .analyse("set item x\nappend items \" $item\"", "tcl");
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagCode::W104)
            .unwrap();
        let [fix] = diagnostic.fixes.as_slice() else {
            panic!("original phase-preserving proposal")
        };
        assert_eq!(fix.new_text, "lappend items $item");
        assert_eq!(fix.safety, crate::irules_checks::FixSafety::RequiresReview);
    }
}
