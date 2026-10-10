// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original expression token geometry and Jim primary preparation actions.

use tcl_lexer::{ExprLexicalFailure, ExprLexicalFailureKind, ExprTerm, ExprToken};

use super::parser::{CheckedExprParse, ExprParseContext, NativeExprSyntax};

/// Parsing and primary preparation are independent observable outcomes.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedNativeExprParse {
    /// Complete syntax, native rejection, or unavailable parser evidence.
    pub parsed: CheckedExprParse<Vec<u8>>,
    /// Authentic Jim representation action; absent for other or unknown engines.
    pub jim: Option<JimExpressionPreparation>,
}

/// The native stage selecting an original Jim object's representation action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JimExpressionPreparationStage {
    /// The native token scanner rejected the original source.
    Tokenization,
    /// Empty input or a missing wrapper failed before tree construction.
    Completeness,
    /// Native expression tree construction completed or failed.
    Tree,
}

/// Replacement of the original primary representation by Jim preparation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JimExpressionCacheAction {
    /// Keep the original primary representation unchanged.
    PreserveOriginal,
    /// Install the rejected Expression(NULL) primary, without a tree.
    Rejected,
    /// Install the prepared tree and its original term objects.
    Prepared,
}

/// Pure preparation facts; concrete interpreters own original term objects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JimExpressionPreparation {
    /// Stage selecting the representation action independently of diagnostics.
    pub stage: JimExpressionPreparationStage,
    /// Action on the same original source object's primary representation.
    pub action: JimExpressionCacheAction,
    /// Every original term, including lazy branches, on successful preparation.
    pub terms: Vec<ExprTerm>,
}

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

/// Jim's integer-expression primitive formats its original operand through
/// the native `%#s` C-string extent without changing retained error-code state.
#[must_use]
pub fn jim_integer_expression_message(original: &[u8]) -> Vec<u8> {
    let mut message = b"expected integer expression but got \"".to_vec();
    message.extend_from_slice(tcl_core_types::c_string_extent(original));
    message.push(b'"');
    message
}

pub(super) fn jim_expression_preparation(
    source: &[u8],
    context: &ExprParseContext,
    tokens: &[ExprToken<Vec<u8>>],
    failures: &[ExprLexicalFailure],
    parsed: &CheckedExprParse<Vec<u8>>,
) -> Option<JimExpressionPreparation> {
    if matches!(parsed, CheckedExprParse::Unsupported(_)) {
        return None;
    }
    jim_preparation_for_tree(
        source,
        context,
        tokens,
        failures,
        matches!(parsed, CheckedExprParse::Parsed(_)),
        false,
    )
}

pub(super) fn jim_preparation_for_tree(
    source: &[u8],
    context: &ExprParseContext,
    tokens: &[ExprToken<Vec<u8>>],
    failures: &[ExprLexicalFailure],
    prepared: bool,
    lexical_rejection: bool,
) -> Option<JimExpressionPreparation> {
    if context.native_syntax != NativeExprSyntax::Jim084 {
        return None;
    }
    let (stage, action) = if lexical_rejection
        || failures
            .iter()
            .any(|failure| !missing_wrapper(failure.kind))
    {
        (
            JimExpressionPreparationStage::Tokenization,
            JimExpressionCacheAction::Rejected,
        )
    } else if !failures.is_empty() || tokens.iter().all(|token| token.kind.is_skipped()) {
        (
            JimExpressionPreparationStage::Completeness,
            JimExpressionCacheAction::PreserveOriginal,
        )
    } else {
        (
            JimExpressionPreparationStage::Tree,
            if prepared {
                JimExpressionCacheAction::Prepared
            } else {
                JimExpressionCacheAction::Rejected
            },
        )
    };
    let terms = if action == JimExpressionCacheAction::Prepared {
        let config = tcl_lexer::LexerConfig::default().with_grammar(context.lexer_grammar);
        tcl_lexer::expression_terms(source, tokens, config)?
    } else {
        Vec::new()
    };
    Some(JimExpressionPreparation {
        stage,
        action,
        terms,
    })
}

fn missing_wrapper(kind: ExprLexicalFailureKind) -> bool {
    matches!(
        kind,
        ExprLexicalFailureKind::MissingQuote
            | ExprLexicalFailureKind::MissingBracket
            | ExprLexicalFailureKind::MissingBrace
    )
}

/// Authentic Jim Script preparation's source-context and cache ordering.
/// This recipe does not create a Script primary or an interpreter context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JimScriptPreparation;

impl JimScriptPreparation {
    /// String materialization precedes selection of original Source/Script info.
    #[must_use]
    pub const fn source_info_after_string_access(self) -> bool {
        true
    }

    /// A malformed script still installs its native token backing and missing marker.
    #[must_use]
    pub const fn retains_malformed_script(self) -> bool {
        true
    }

    /// Duplicating the original Script retires its primary to an untyped string.
    #[must_use]
    pub const fn duplicate_retains_script(self) -> bool {
        false
    }

    /// The actual interpreter empty object evaluates through its nullScriptObj.
    #[must_use]
    pub const fn redirects_interpreter_empty_object(self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::parser::prepare_expr_bytes_checked_with_context;
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
