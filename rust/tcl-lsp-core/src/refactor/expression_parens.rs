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

//! Regroup a braced expression by operator precedence: make every grouping
//! explicit, or drop the parentheses the parse does not need. Both rewrites
//! are re-parsed under the document's expression grammar (F5 word operators
//! included) and offered only when the tree is unchanged.

use tcl_compiler::analyser::AnalysisResult;

use super::{RefactorEdit, Refactoring};
use crate::code_actions::ActionKind;

fn rewrite(
    source: &str,
    cursor: u32,
    analysis: &AnalysisResult,
    title: &str,
    render: impl Fn(&tcl_syntax::expr::OriginalSourceExpression) -> Option<String>,
) -> Option<Refactoring> {
    let operand = crate::expr_context::source_expression_operand_at(source, analysis, cursor)?;
    let content = operand.content_span()?;
    let expression = operand.original_expression(source, analysis)?;
    let text = expression.source();
    let lead = text.len() - text.trim_start().len();
    let trail = text.len() - text.trim_end().len();
    let new_text = render(&expression)?;
    Some(Refactoring {
        title: title.to_owned(),
        edits: vec![RefactorEdit {
            start: content.start() + u32::try_from(lead).ok()?,
            end: content.end() - u32::try_from(trail).ok()?,
            new_text,
        }],
        kind: ActionKind::RefactorRewrite,
        data_group: None,
        disabled: None,
    })
}

/// Parenthesise every operator's operands in the braced expression at
/// `cursor`, so the grouping no longer depends on precedence.
#[must_use]
pub fn parenthesise_expression(
    source: &str,
    cursor: u32,
    analysis: &AnalysisResult,
) -> Option<Refactoring> {
    rewrite(
        source,
        cursor,
        analysis,
        "Parenthesise expression by precedence",
        tcl_syntax::expr::OriginalSourceExpression::fully_parenthesised,
    )
}

/// Remove the parentheses in the braced expression at `cursor` that
/// precedence makes redundant.
#[must_use]
pub fn remove_redundant_parens(
    source: &str,
    cursor: u32,
    analysis: &AnalysisResult,
) -> Option<Refactoring> {
    rewrite(
        source,
        cursor,
        analysis,
        "Remove redundant parentheses",
        tcl_syntax::expr::OriginalSourceExpression::without_redundant_parens,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analysis(source: &str, dialect: &str) -> AnalysisResult {
        tcl_compiler::analyser::Analyser::new().analyse(source, dialect)
    }

    #[test]
    fn parenthesise_and_remove_round_trip() {
        let source = "if {$c & 0xff == 0x81 || ($d)} {puts x}";
        let a = analysis(source, "tcl8.6");
        let grouped = parenthesise_expression(source, 6, &a).unwrap();
        assert_eq!(
            grouped.apply(source),
            "if {($c & (0xff == 0x81)) || $d} {puts x}"
        );
        let trimmed = remove_redundant_parens(source, 6, &a).unwrap();
        assert_eq!(
            trimmed.apply(source),
            "if {$c & 0xff == 0x81 || $d} {puts x}"
        );
    }

    #[test]
    fn irules_word_operators_are_regrouped() {
        let source = "when HTTP_REQUEST { if { ([HTTP::uri] starts_with \"/a\") and not ($x) } { log local0. y } }";
        let a = analysis(source, "f5-irules");
        let cursor = u32::try_from(source.find("and").unwrap()).unwrap();
        let trimmed = remove_redundant_parens(source, cursor, &a).unwrap();
        assert!(
            trimmed
                .apply(source)
                .contains("{ [HTTP::uri] starts_with \"/a\" and not $x }"),
            "{}",
            trimmed.apply(source)
        );
    }

    #[test]
    fn outside_an_expression_offers_nothing() {
        let source = "puts ($a)";
        let a = analysis(source, "tcl8.6");
        assert!(remove_redundant_parens(source, 6, &a).is_none());
    }
}
