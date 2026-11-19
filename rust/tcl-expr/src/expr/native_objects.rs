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
        let config = tcl_lexer::LexerConfig::from_grammar(context.lexer_grammar);
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
