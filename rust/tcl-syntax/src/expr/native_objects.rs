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

//! Interpreter-owned Jim objects for a checked expression preparation.

pub use tcl_expr::expr::native_objects::*;
use tcl_lexer::ExprTerm;

/// Actual original term objects, indexed by authenticated native source spans.
/// This holder owns terms; it grants no interpreter or native ABI authority.
pub struct JimExpressionObjects<V> {
    terms: Vec<(ExprTerm, V)>,
}

/// Fresh native term constructor input; existing-object getters are separate.
pub enum JimExpressionTermValue<'a> {
    /// Integer/double constructor selected by the native token scanner.
    Number(crate::scalar_getter::JimExpressionNumber),
    /// Original counted token body, without substitution or escape decoding.
    String(&'a [u8]),
}

impl<V> JimExpressionObjects<V> {
    /// Create every original term once, including unvisited lazy branches.
    ///
    /// # Errors
    /// Returns unavailable original geometry or the constructor's exact failure.
    pub fn prepare(
        source: &[u8],
        preparation: &JimExpressionPreparation,
        mut construct: impl FnMut(
            &ExprTerm,
            JimExpressionTermValue<'_>,
        ) -> Result<V, crate::value::ValueError>,
    ) -> Result<Self, crate::value::ValueError> {
        if preparation.action != JimExpressionCacheAction::Prepared {
            return Err(crate::value::ValueError::CommandProtocolUnavailable(
                "prepared Jim expression terms",
            ));
        }
        let mut terms = Vec::with_capacity(preparation.terms.len());
        for term in &preparation.terms {
            let body = source
                .get(term.value.start as usize..term.value.end as usize)
                .ok_or(crate::value::ValueError::CommandProtocolUnavailable(
                    "original Jim expression term geometry",
                ))?;
            let numeric = term
                .jim_numeric_kind
                .and_then(|kind| crate::scalar_getter::jim_expression_number_for_kind(body, kind));
            let payload = numeric.map_or(
                JimExpressionTermValue::String(body),
                JimExpressionTermValue::Number,
            );
            terms.push((term.clone(), construct(term, payload)?));
        }
        Ok(Self { terms })
    }

    /// Borrow the original term selected by its full native syntax extent.
    #[must_use]
    pub fn at(&self, start: u32, end: Option<u32>) -> Option<(&ExprTerm, &V)> {
        self.terms
            .iter()
            .find(|(term, _)| {
                term.source.start == start
                    && end.is_none_or(|end| end.checked_add(1) == Some(term.source.end))
            })
            .map(|(term, value)| (term, value))
    }

    /// Number of real original term objects retained by this tree.
    #[must_use]
    pub fn len(&self) -> usize {
        self.terms.len()
    }

    /// Whether this prepared tree contains no real term objects.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::super::parser::{
        CheckedExprParse, ExprParseContext, prepare_expr_bytes_checked_with_context,
    };
    use super::*;

    fn context() -> ExprParseContext {
        let profile = tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        );
        ExprParseContext::for_profile(&profile)
    }

    #[test]
    fn jim_numeric_frontiers_keep_native_tree_outcomes() {
        for source in [b"0x1.8p1".as_slice(), b"0x1.", b".5", b"1eq2", b"Inf"] {
            let prepared = prepare_expr_bytes_checked_with_context(source, &context());
            assert!(
                matches!(prepared.parsed, CheckedExprParse::Parsed(_)),
                "{source:?}"
            );
            let receipt = prepared.jim.unwrap();
            assert_eq!(
                receipt.action,
                JimExpressionCacheAction::Prepared,
                "{source:?}"
            );
            assert!(
                receipt
                    .terms
                    .iter()
                    .all(|term| term.jim_numeric_kind.is_some())
            );
        }
        for source in [
            b"0x".as_slice(),
            b"0xg",
            b"0b2",
            b"0o8",
            b"0d",
            b"12x",
            b"1_",
            b"09e",
            b"1e+",
            b"0x1p4",
            b"0x.p4",
            b"NaNfoo",
            b"Infinity",
            b".",
        ] {
            let prepared = prepare_expr_bytes_checked_with_context(source, &context());
            assert!(
                !matches!(prepared.parsed, CheckedExprParse::Parsed(_)),
                "{source:?}"
            );
            assert_eq!(
                prepared.jim.unwrap().action,
                JimExpressionCacheAction::Rejected,
                "{source:?}"
            );
        }
    }

    #[test]
    fn jim_rejected_preparation_distinguishes_preserved_primary() {
        let context = context();
        for source in [b"".as_slice(), b"{", b"\"", b"["] {
            let receipt = prepare_expr_bytes_checked_with_context(source, &context)
                .jim
                .unwrap();
            assert_eq!(
                receipt.stage,
                JimExpressionPreparationStage::Completeness,
                "{source:?}"
            );
            assert_eq!(
                receipt.action,
                JimExpressionCacheAction::PreserveOriginal,
                "{source:?}"
            );
        }
        for source in [b"$".as_slice(), b"1+", b"1 2", b"."] {
            let receipt = prepare_expr_bytes_checked_with_context(source, &context)
                .jim
                .unwrap();
            assert_eq!(
                receipt.action,
                JimExpressionCacheAction::Rejected,
                "{source:?}"
            );
        }
        let receipt = prepare_expr_bytes_checked_with_context(b"1 2 @", &context)
            .jim
            .unwrap();
        assert_eq!(receipt.stage, JimExpressionPreparationStage::Tokenization);
    }

    #[test]
    fn jim_prepared_terms_keep_lazy_commands_and_original_name_objects() {
        let source = b"0 ? [never] : $a($key)";
        let prepared = prepare_expr_bytes_checked_with_context(source, &context());
        assert!(matches!(prepared.parsed, CheckedExprParse::Parsed(_)));
        let receipt = prepared.jim.unwrap();
        assert_eq!(receipt.action, JimExpressionCacheAction::Prepared);
        assert_eq!(receipt.terms.len(), 3);
        let command = &receipt.terms[1];
        assert_eq!(command.kind, tcl_lexer::ExprTermKind::Command);
        assert_eq!(
            &source[command.source.start as usize..command.source.end as usize],
            b"[never]"
        );
        assert_eq!(
            &source[command.value.start as usize..command.value.end as usize],
            b"never"
        );
        let name = &receipt.terms[2];
        assert_eq!(name.kind, tcl_lexer::ExprTermKind::IndexedVariable);
        assert_eq!(
            &source[name.value.start as usize..name.value.end as usize],
            b"a($key)"
        );
    }

    #[test]
    fn jim_term_geometry_keeps_empty_opaque_and_escaped_operands() {
        let source = b"{\xff} eq \"\xff\" || (\"\" eq \"$x\")\n|| [step]";
        let (tokens, failures) = tcl_lexer::tokenise_expr_bytes_with_failures(
            source,
            &context().lexer_grammar,
            context().expr_grammar_base,
            None,
        );
        assert_eq!(failures, [] as [tcl_lexer::ExprLexicalFailure; 0]);
        let terms = tcl_lexer::expression_terms(
            source,
            &tokens,
            tcl_lexer::LexerConfig::default().with_grammar(context().lexer_grammar),
        )
        .unwrap();
        assert_eq!(terms[0].kind, tcl_lexer::ExprTermKind::String);
        assert_eq!(terms[1].kind, tcl_lexer::ExprTermKind::String);
        assert!(terms[2].value.is_empty());
        assert_eq!(terms[3].kind, tcl_lexer::ExprTermKind::EscapedString);
        assert_eq!(terms[4].kind, tcl_lexer::ExprTermKind::Command);
        assert_eq!(terms[4].line_delta, 1);
    }
}
