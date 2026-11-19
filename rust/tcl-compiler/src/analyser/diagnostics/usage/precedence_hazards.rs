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

//! W154: an unparenthesised grouping that operator precedence parses differently from how it reads.

use crate::analyser::state::Analyser;
use crate::analyser::types::{CodeFix, Diagnostic, Severity};
use crate::irules_checks::FixSafety;
use tcl_core_types::DiagCode;

impl Analyser {
    /// W154 on one braced expression operand. Only a braced word reaches the
    /// evaluator as written, so only its source grouping is the parse.
    pub(in crate::analyser) fn emit_w154_precedence_hazards(&mut self, word: &tcl_lexer::NativeWord) {
        if word.group().kind != tcl_lexer::WordKind::Braced
            || word.image().bytes() != self.source.as_bytes()
        {
            return;
        }
        let Ok(content) = word.content_span() else {
            return;
        };
        let Some(text) = self.source.get(content.as_range()) else {
            return;
        };
        let Some(context) = self.original_expression_parser_context() else {
            return;
        };
        let Some(expression) =
            tcl_syntax::expr::OriginalSourceExpression::capture(text, &context, word.config())
        else {
            return;
        };
        for hazard in expression.precedence_hazards() {
            let shift = |span: tcl_lexer::Span| {
                tcl_lexer::Span::new(span.start() + content.start(), span.end() + content.start())
            };
            let extent = shift(hazard.extent);
            let fixes = vec![
                CodeFix {
                    span: extent,
                    new_text: hazard.as_parsed.clone(),
                    description: format!("Parenthesise as parsed: {}", hazard.as_parsed),
                    safety: FixSafety::SemanticsEquivalent,
                },
                CodeFix {
                    span: extent,
                    new_text: hazard.likely_intended.clone(),
                    description: format!("Regroup as {}", hazard.likely_intended),
                    safety: FixSafety::RequiresReview,
                },
            ];
            self.result.diagnostics.push(
                Diagnostic::new(
                    DiagCode::W154,
                    shift(hazard.operator),
                    format!(
                        "'{}' binds tighter than '{}', so this parses as `{}`; if you meant `{}`, add parentheses.",
                        hazard.inner, hazard.outer, hazard.as_parsed, hazard.likely_intended
                    ),
                    Severity::Warning,
                )
                .with_fixes(fixes),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::analyser::types::Diagnostic;
    use tcl_core_types::DiagCode;

    fn w154(dialect: &str, source: &str) -> Vec<Diagnostic> {
        crate::analyser::state::Analyser::new()
            .analyse(source, dialect)
            .diagnostics
            .into_iter()
            .filter(|d| d.code == DiagCode::W154)
            .collect()
    }

    #[test]
    fn bitwise_and_over_equality_is_reported_on_the_operator() {
        let source = "if {$c & 0xff == 0x81} {puts hit}";
        let found = w154("tcl8.6", source);
        let [diagnostic] = found.as_slice() else {
            panic!("{found:?}");
        };
        assert_eq!(&source[diagnostic.span.as_range()], "&");
        assert!(diagnostic.message.contains("`$c & (0xff == 0x81)`"));
        let intended = diagnostic
            .fixes
            .iter()
            .find(|fix| fix.new_text == "($c & 0xff) == 0x81")
            .unwrap();
        assert_eq!(&source[intended.span.as_range()], "$c & 0xff == 0x81");
    }

    #[test]
    fn explicit_or_unbraced_groupings_are_quiet() {
        assert!(w154("tcl8.6", "if {($c & 0xff) == 0x81} {puts hit}").is_empty());
        assert!(w154("tcl8.6", "set x [expr {$a + $b == 3}]").is_empty());
    }

    #[test]
    fn irules_word_operators_and_nested_expressions_are_checked() {
        let found = w154(
            "f5-irules",
            "when HTTP_REQUEST { set f [expr {$flags | 2 == 2}]\n if { not $a equals \"x\" } { log local0. x } }",
        );
        assert_eq!(found.len(), 2, "{found:?}");
    }
}
