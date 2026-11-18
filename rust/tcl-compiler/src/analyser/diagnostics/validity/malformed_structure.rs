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

//! E004 for a malformed clause structure and E006 for an invalid parameter or static-variable list.

use super::super::helpers::has_substitution;
use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use tcl_core_types::DiagCode;

/// Whether `word` is a literal list value: braced, or free of outer Tcl
/// substitutions. A braced word is literal even when its eventual elements
/// contain `$` / `[`. A computed value can be valid or invalid at runtime, so
/// reporting it would be an unsound claim.
fn is_literal_list_word(word: &str, token: &tcl_lexer::Token) -> bool {
    token.kind == tcl_lexer::TokenType::Str
        || (token.kind != tcl_lexer::TokenType::Expand && !has_substitution(word, token))
}

/// Emit E006 for every registry-identified, statically-known formal-parameter
/// list that Tcl would reject while creating its callable.  Callers supply the
/// role-derived indices so this routine deliberately has no knowledge of the
/// command or definition-body member that owns the list.
pub(in crate::analyser) fn emit_invalid_formal_parameter_list_diagnostics(
    analyser: &mut Analyser,
    args: &[String],
    arg_tokens: &[tcl_lexer::Token],
    parameter_indices: &[usize],
) {
    let Some(grammar) = tcl_registry::InvocationDialect::of_profile(
        analyser.resolved_analysis_input().unit_profile(),
    )
    .parameter_grammar() else {
        return;
    };
    for &index in parameter_indices {
        let (Some(params), Some(token)) = (args.get(index), arg_tokens.get(index)) else {
            continue;
        };
        if !is_literal_list_word(params, token) {
            continue;
        }
        let Err(error) = crate::signature_scan::params::parse_param_list_strict_in(
            params,
            analyser.word_rules(),
            grammar,
        ) else {
            continue;
        };
        let span = super::super::super::utils::full_word_span(*token, &analyser.source);
        let fixes = tcl_syntax::formal_params::split_overlong_parameter_specifier(params, &error)
            .and_then(|repaired| {
                let original = analyser.retained_value_word(*token)?;
                (original.span() == span
                    && tcl_syntax::word_rules::original_static_word_unicode_value(&original)?
                        .as_slice()
                        == params.as_bytes())
                .then_some(())?;
                let replacement = tcl_syntax::word_rules::lexical_unicode_source_word(
                    &repaired,
                    original.config(),
                )?;
                Some(vec![crate::analyser::types::CodeFix::review(
                    span,
                    replacement,
                    "Split the grouped fields into separate parameters",
                )])
            })
            .unwrap_or_default();
        analyser.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(
                DiagCode::E006,
                span,
                format!("Invalid formal parameter list: {error}"),
                Severity::Error,
            )
            .with_fixes(fixes),
        );
    }
}

/// E006 for a registry-identified ([`tcl_registry::ArgRole::StaticVarList`]),
/// statically-known static-variable list with an element of more than two
/// fields: a static is a name, a `{name value}` pair or `&name`, and `jimsh`
/// rejects any other shape while creating the procedure.
pub(in crate::analyser) fn emit_invalid_static_variable_list_diagnostics(
    analyser: &mut Analyser,
    args: &[String],
    arg_tokens: &[tcl_lexer::Token],
    static_indices: &[usize],
) {
    for &index in static_indices {
        let (Some(statics), Some(token)) = (args.get(index), arg_tokens.get(index)) else {
            continue;
        };
        if !is_literal_list_word(statics, token) {
            continue;
        }
        let word_rules = analyser.word_rules();
        let Ok(elements) = word_rules.split_list(statics) else {
            continue;
        };
        let too_many = elements.iter().find(|element| {
            word_rules
                .split_list(element)
                .is_ok_and(|fields| fields.len() > 2)
        });
        let Some(element) = too_many else {
            continue;
        };
        let span = super::super::super::utils::full_word_span(*token, &analyser.source);
        analyser.result.diagnostics.push(crate::analyser::types::Diagnostic::new(
            DiagCode::E006,
            span,
            format!("Invalid static variable list: too many fields in static specifier \"{element}\""),
            Severity::Error,
        ));
    }
}

/// E006's lambda-literal companion.  `ArgRole::LambdaLiteral` declares the
/// standard `{parameters body ?namespace?}` value shape, so a consumer can
/// discover the nested parameter list without knowing the command that owns
/// the literal.
pub(in crate::analyser) fn emit_invalid_lambda_parameter_list_diagnostics(
    analyser: &mut Analyser,
    args: &[String],
    arg_tokens: &[tcl_lexer::Token],
    lambda_indices: &[usize],
) {
    let Some(grammar) = tcl_registry::InvocationDialect::of_profile(
        analyser.resolved_analysis_input().unit_profile(),
    )
    .parameter_grammar() else {
        return;
    };
    for &index in lambda_indices {
        let (Some(lambda), Some(token)) = (args.get(index), arg_tokens.get(index)) else {
            continue;
        };
        if token.kind != tcl_lexer::TokenType::Str {
            continue;
        }
        let word_rules = analyser.word_rules();
        let Ok(fields) = word_rules.split_list(lambda) else {
            continue;
        };
        let Some(params) = fields.first() else {
            continue;
        };
        let Err(error) =
            crate::signature_scan::params::parse_param_list_strict_in(params, word_rules, grammar)
        else {
            continue;
        };
        let span = super::super::super::utils::full_word_span(*token, &analyser.source);
        let fixes = tcl_syntax::formal_params::split_overlong_parameter_specifier(params, &error)
            .and_then(|repaired| {
                let original = analyser.retained_value_word(*token)?;
                (original.span() == span
                    && tcl_syntax::word_rules::original_static_word_unicode_value(&original)?
                        .as_slice()
                        == lambda.as_bytes())
                .then_some(())?;
                let mut repaired_fields = fields.clone();
                repaired_fields[0] = repaired.into();
                let repaired_lambda = tcl_syntax::list::join_list(repaired_fields);
                let replacement = tcl_syntax::word_rules::lexical_unicode_source_word(
                    &repaired_lambda,
                    original.config(),
                )?;
                Some(vec![crate::analyser::types::CodeFix::review(
                    span,
                    replacement,
                    "Split the grouped fields into separate parameters",
                )])
            })
            .unwrap_or_default();
        analyser.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(
                DiagCode::E006,
                span,
                format!("Invalid formal parameter list: {error}"),
                Severity::Error,
            )
            .with_fixes(fixes),
        );
    }
}

impl Analyser {
    /// **E004.** Shape advice from the genuinely selected clause grammar over
    /// structured argv. Unknown payloads keep their positions, while unknown
    /// selector words cannot select a shape. The typed subject owns the
    /// offending written operand; captures cannot borrow its span. Fixes
    /// require a complete consecutive vector of ordinary original words.
    pub(in crate::analyser) fn emit_e004_clause_shape_diagnostic(
        &mut self,
        original: Option<&super::super::super::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        use tcl_registry::ClauseShapeError;
        let Some(original) = original else { return };
        let Some(issue) = original
            .with_schema(super::super::super::diagnostic_registry::source_clause_issue)
            .flatten()
        else {
            return;
        };
        let error = issue.error();
        let argument = match error {
            ClauseShapeError::MissingExpr { after } => after,
            ClauseShapeError::MissingBody { after } => Some(after),
            ClauseShapeError::ExtraWords { first_extra } => Some(first_extra),
        };
        if argument.is_some_and(|index| original.is_captured(index)) {
            return;
        }
        let Some(subject) = original.subject(
            super::super::super::RegistrySourceDiagnosticKind::ClauseShape,
            argument,
        ) else {
            return;
        };
        let mut span = argument.map_or(original.head().span(), |index| {
            original.word(index).unwrap().span()
        });
        let label = |index| {
            original
                .literal(index)
                .or_else(|| original.word(index)?.try_text().ok())
                .unwrap_or("argument")
        };
        let message = match error {
            ClauseShapeError::MissingExpr { after: None } => {
                format!("No expression after \"{}\" argument", original.command())
            }
            ClauseShapeError::MissingExpr { after: Some(index) } => {
                format!("No expression after \"{}\" argument", label(index))
            }
            ClauseShapeError::MissingBody { after } => {
                format!("No script following \"{}\" argument", label(after))
            }
            ClauseShapeError::ExtraWords { .. } => {
                if let Some(last) = original
                    .words()
                    .arguments()
                    .len()
                    .checked_sub(1)
                    .and_then(|index| original.word(index))
                {
                    span = tcl_lexer::Span::new(span.start(), last.span().end());
                }
                "Extra words after \"else\" clause in \"if\" command".to_owned()
            }
        };
        // Edits require an entirely written, ordinary and consecutive vector.
        // Captured prefixes and expansion children never borrow editable words.
        let written_spans = (0..original.words().arguments().len())
            .map(|index| {
                (original.written_index(index)? == index).then_some(())?;
                Some(original.word(index)?.span())
            })
            .collect::<Option<Vec<_>>>();
        let fixes =
            written_spans.map_or_else(Vec::new, |spans| self.e004_fixes(&spans, issue.repair()));
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(DiagCode::E004, span, message, Severity::Error)
                .with_subject(subject)
                .with_fixes(fixes),
        );
    }

    /// Proposals use anchors authored by the same selected clause grammar and
    /// complete original word spans. No condition spelling selects an anchor.
    fn e004_fixes(
        &self,
        argument_spans: &[tcl_lexer::Span],
        repair: Option<tcl_registry::ClauseShapeRepair>,
    ) -> Vec<super::super::types::CodeFix> {
        use tcl_registry::ClauseShapeRepair;
        let Some(repair) = repair else {
            return Vec::new();
        };
        let start_index = match repair {
            ClauseShapeRepair::MergeTrailingWords { body } => body,
            ClauseShapeRepair::RemoveTrailingClause { keyword } => keyword,
        };
        let (Some(first), Some(last)) = (argument_spans.get(start_index), argument_spans.last())
        else {
            return Vec::new();
        };
        let span = tcl_lexer::Span::new(first.start(), last.end());
        let (new_text, description) = match repair {
            ClauseShapeRepair::MergeTrailingWords { .. } => {
                let Some(slice) = self.source.get(span.as_range()) else {
                    return Vec::new();
                };
                (
                    format!("{{{slice}}}"),
                    "Merge trailing words into the if body",
                )
            }
            ClauseShapeRepair::RemoveTrailingClause { .. } => {
                (String::new(), "Remove incomplete trailing clause")
            }
        };
        vec![super::super::types::CodeFix {
            span,
            new_text,
            description: description.to_owned(),
            // Merging or deleting written code requires choosing its intended meaning.
            safety: crate::irules_checks::FixSafety::RequiresReview,
        }]
    }
}
