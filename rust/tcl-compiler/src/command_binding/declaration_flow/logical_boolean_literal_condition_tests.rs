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

use super::logical_boolean_literal_condition;
use tcl_syntax::expr::parser::{
    CheckedExprParse, ExprParseUnsupported, parse_expr_checked_with_context,
};

#[test]
fn common_source_booleans_leave_native_syntax_unknown() {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let dialect = tcl_native_model::InvocationDialect::of_profile(profile);
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    for source in ["0", "1", " 1 ", "\t0\n"] {
        assert!(matches!(
            parse_expr_checked_with_context(source, &dialect.expression_parse_context(None)),
            CheckedExprParse::Unsupported(ExprParseUnsupported::UnknownNativeSyntax)
        ));
        assert!(
            logical_boolean_literal_condition(source, dialect, config).is_some(),
            "{source:?}"
        );
        assert!(matches!(
            parse_expr_checked_with_context(source, &dialect.expression_parse_context(None)),
            CheckedExprParse::Unsupported(ExprParseUnsupported::UnknownNativeSyntax)
        ));
    }
}

#[test]
fn source_literal_branch_rejects_other_effects_and_selected_native_contexts() {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let dialect = tcl_native_model::InvocationDialect::of_profile(profile);
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    for source in [
        "$x",
        "[set x]",
        "1+0",
        "1 || [effect]",
        "double(1)",
        "1;",
        "1 #comment",
        "2",
        "01",
        "\"1\"",
    ] {
        assert!(
            logical_boolean_literal_condition(source, dialect, config).is_none(),
            "{source}"
        );
    }
    for name in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        let profile = tcl_dialect::DialectProfile::find(name).unwrap();
        assert!(
            logical_boolean_literal_condition(
                "1",
                tcl_native_model::InvocationDialect::of_profile(profile),
                tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
            )
            .is_none()
        );
    }
}
