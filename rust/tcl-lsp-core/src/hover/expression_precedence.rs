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

//! Hover on an expression operator or parenthesis: the parse as a tree.

use super::Hover;
use tcl_compiler::analyser::AnalysisResult;

/// The precedence tree of the braced expression operand under the cursor,
/// when the cursor is on one of its operators or grouping parentheses.
pub(super) fn expression_precedence_hover(
    source: &str,
    analysis: &AnalysisResult,
    cursor_offset: u32,
) -> Option<Hover> {
    let operand =
        crate::expr_context::source_expression_operand_at(source, analysis, cursor_offset)?;
    let content = operand.content_span()?;
    let local = cursor_offset.checked_sub(content.start())?;
    let expression = operand.original_expression(source, analysis)?;
    let tree = expression.precedence_tree_at(local)?;
    let mut text = String::from("**Expression precedence** — higher binds tighter\n\n");
    if let Some(grouped) = expression.fully_parenthesised() {
        text.push_str("```tcl\n");
        text.push_str(&grouped);
        text.push_str("\n```\n\n");
    }
    text.push_str("```text\n");
    text.push_str(&tree);
    text.push_str("```\n");
    Some(Hover::markdown(text))
}

#[cfg(test)]
mod tests {
    fn hover_at(source: &str, dialect: &str, needle: &str) -> Option<String> {
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, dialect);
        let offset = source.find(needle)?;
        let character = u32::try_from(offset).ok()?;
        super::super::hover(source, 0, character, &analysis, None).map(|hover| hover.value)
    }

    #[test]
    fn operator_hover_draws_the_parse_tree() {
        let text = hover_at("if {$c & 0xff == 0x81} {puts x}", "tcl8.6", "&").unwrap();
        assert!(text.contains("$c & (0xff == 0x81)"), "{text}");
        assert!(
            text.contains("&  (precedence 5)  ◀\n├─ $c\n└─ ==  (precedence 6)"),
            "{text}"
        );
    }

    #[test]
    fn irules_word_operator_hover() {
        let source =
            "when HTTP_REQUEST { if {[HTTP::uri] starts_with \"/a\" and $x} { log local0. y } }";
        let text = hover_at(source, "f5-irules", "and").unwrap();
        assert!(text.contains("and  (precedence 2)  ◀"), "{text}");
        assert!(text.contains("starts_with  (precedence 6)"), "{text}");
    }

    #[test]
    fn operands_keep_their_own_hover() {
        let text = hover_at("if {$c & 0xff == 0x81} {puts x}", "tcl8.6", "0xff");
        assert!(text.is_none_or(|text| !text.contains("Expression precedence")));
    }
}
