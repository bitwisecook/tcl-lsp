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

//! Unbraced expressions (W100), script bodies (W105) and `switch` bodies (W106).

use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use tcl_core_types::DiagCode;

/// A proposal prepared from whole original expression words.
struct UnbracedSourceExpression {
    pub(super) span: tcl_lexer::Span,
    text: String,
    has_sub: bool,
    fix_inner: Option<String>,
    subject: crate::analyser::DiagnosticSubject,
    /// A hexadecimal prefix followed by one substitution warrants scan
    /// advice, without proving an equivalent replacement for unknown text.
    hex_conversion: bool,
}

/// `0x[string range $hex 0 1]` / `0x$byte`: a hexadecimal prefix followed by
/// exactly one substituted value.
fn original_word_is_hex_conversion(word: &tcl_lexer::NativeWord) -> bool {
    let arena = word.executable_parts();
    let [prefix, value] = arena.list(arena.root()) else {
        return false;
    };
    matches!(arena.text(prefix), Some(b"0x" | b"0X"))
        && matches!(
            value.part,
            tcl_lexer::ExecutablePart::Command { .. } | tcl_lexer::ExecutablePart::Variable { .. }
        )
}

pub(in crate::analyser) fn original_word_has_substitution(word: &tcl_lexer::NativeWord) -> bool {
    word.executable_parts().all_parts().any(|part| {
        matches!(
            part.part,
            tcl_lexer::ExecutablePart::Variable { .. }
                | tcl_lexer::ExecutablePart::Command { .. }
                | tcl_lexer::ExecutablePart::Expression { .. }
        )
    })
}

impl Analyser {
    /// Classify the selected original script word before its outer substitutions.
    pub(in crate::analyser) fn emit_w105_unbraced_body(
        &mut self,
        command: &str,
        word: &tcl_lexer::NativeWord,
    ) {
        use crate::analyser::types::{CodeFix, Diagnostic, FixSafety};
        use tcl_lexer::{ExecutablePart, WordKind};
        use tcl_syntax::word_rules::{
            WordValueRules, original_static_word_unicode_value, whole_braced_word,
        };
        let span = word.span();
        if word.group().expand
            || word.group().kind == WordKind::Braced
            || self
                .result
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W105 && d.span == span)
        {
            return;
        }
        let Ok(content) = word.content_span() else {
            return;
        };
        let Some(written) = self.source.get(content.as_range()) else {
            return;
        };
        let arena = word.executable_parts();
        let parts = arena.list(arena.root());
        if parts
            .iter()
            .any(|part| matches!(part.part, ExecutablePart::ParseError(_)))
        {
            return;
        }
        // A whole script-valued reference, or a Tk +[list ...] prefix,
        // already supplies the script value the receiver will execute.
        let separates =
            |byte| word.config().word_separators.is_separator(byte) || matches!(byte, b'\n' | b';');
        match parts {
            [only]
                if matches!(
                    only.part,
                    ExecutablePart::Variable { .. } | ExecutablePart::Command { .. }
                ) =>
            {
                return;
            }
            [prefix, command]
                if word.group().kind == WordKind::Bare
                    && matches!(command.part, ExecutablePart::Command { .. })
                    && arena
                        .text(prefix)
                        .is_some_and(|bytes| !bytes.iter().copied().any(separates)) =>
            {
                return;
            }
            _ => {}
        }
        let dynamic = original_word_has_substitution(word);
        let (inner, severity, safety, message) = if dynamic {
            (
                std::borrow::Cow::Borrowed(written),
                Severity::Error,
                FixSafety::BehaviourHardening,
                format!(
                    "Code block argument to '{command}' is not braced and contains substitutions — risk of double substitution."
                ),
            )
        } else {
            let Some(value) = original_static_word_unicode_value(word)
                .and_then(|value| String::from_utf8(value).ok())
            else {
                return;
            };
            if !value.bytes().any(separates) {
                return;
            }
            (
                std::borrow::Cow::Owned(value),
                Severity::Warning,
                FixSafety::SemanticsEquivalent,
                format!(
                    "Code block argument to '{command}' should be braced for clarity and to prevent accidental substitution."
                ),
            )
        };
        let replacement = format!("{{{inner}}}");
        // The braced replacement must form one word, and a continuation in
        // its selected dialect must preserve the intended body bytes.
        let can_brace = whole_braced_word(&replacement).is_some_and(|text| {
            WordValueRules::from_config(&word.config()).collapse_braced_word(text) == inner
        });
        let (message, fixes) = if can_brace {
            (
                format!("{message} Use braces: {{ … }}"),
                vec![CodeFix {
                    span,
                    new_text: replacement,
                    description: "Wrap code block in braces".to_owned(),
                    safety,
                }],
            )
        } else {
            (message, Vec::new())
        };
        self.result
            .diagnostics
            .push(Diagnostic::new(DiagCode::W105, span, message, severity).with_fixes(fixes));
    }

    /// W105 at the script positions the selected source schema declares.
    /// This is source advice: it does not wait for the walk to prove the call
    /// executes, which an unset operand or an unproven package withholds.
    /// A concatenating command is judged by its first script word only.
    pub(in crate::analyser) fn emit_w105_original_script_words(
        &mut self,
        original: Option<&crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        let Some(original) = original else { return };
        if !original.matches_analysis(&self.result, &self.source) {
            return;
        }
        let Some((scripts, concatenates)) = original
            .with_schema(|schema| {
                let mut scripts = schema
                    .authored_source_script_arguments()
                    .unwrap_or_default();
                // Clause-grammar bodies (`if`, `while`, `try`) sit in the plan.
                // Advice reads a computed word as no keyword, the inert reading.
                let plan = match schema.clause_walk() {
                    Some(Ok(plan)) => Some(plan),
                    Some(Err(abstention)) => Some(abstention.inert),
                    None => None,
                };
                if let Some(plan) = plan {
                    scripts.extend(plan.roles.iter().filter_map(|&(index, role)| {
                        (role == tcl_registry::ArgRole::Body).then_some(index)
                    }));
                }
                scripts.sort_unstable();
                scripts.dedup();
                (!scripts.is_empty()).then(|| {
                    (
                        scripts,
                        schema
                            .semantics
                            .traits
                            .contains(tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS),
                    )
                })
            })
            .flatten()
        else {
            return;
        };
        let count = if concatenates { 1 } else { scripts.len() };
        for index in scripts.into_iter().take(count) {
            if let Some(word) = original.operand(index).and_then(|operand| operand.word()) {
                self.emit_w105_unbraced_body(original.command(), word);
            }
        }
    }

    /// W100 source advice at original expression words selected by the shared
    /// schema. Whole-tail warnings require an entirely written vector. Brace
    /// proposals preserve grouping deliberately and always require review.
    pub(in crate::analyser) fn emit_w100_unbraced_expr(
        &mut self,
        original: &crate::analyser::diagnostic_registry::OriginalDiagnosticSource,
        expressions: &tcl_registry::AuthoredSourceExpressionArguments,
    ) {
        for &argument in &expressions.arguments {
            let Some(advice) = self.original_unbraced_expression(original, expressions, argument)
            else {
                continue;
            };
            self.push_w100(original.command(), expressions.concatenates, advice);
            if expressions.concatenates {
                break;
            }
        }
    }

    fn original_unbraced_expression(
        &self,
        original: &crate::analyser::diagnostic_registry::OriginalDiagnosticSource,
        expressions: &tcl_registry::AuthoredSourceExpressionArguments,
        argument: usize,
    ) -> Option<UnbracedSourceExpression> {
        let last = if expressions.concatenates {
            original.arguments().len().checked_sub(1)?
        } else {
            argument
        };
        let word = original.word(argument)?;
        if argument == last && word.group().kind == tcl_lexer::WordKind::Braced {
            return None;
        }
        let subject = original.expression_subject(argument..=last)?;
        let end = original.word(last)?;
        let span = tcl_lexer::Span::new(word.span().start(), end.span().end());
        let text = self.source.get(span.as_range()).map(str::to_owned)?;
        let has_sub = (argument..=last).any(|index| {
            original
                .word(index)
                .is_some_and(original_word_has_substitution)
        });
        if !has_sub {
            // Inspect the complete written expression. A failed source parse
            // cannot establish that the proposed braced spelling is valid.
            let expression = tcl_syntax::expr::substitution::OriginalSourceExpression::capture(
                &text,
                &self.original_expression_parser_context()?,
                self.file_lexer_config(),
            )?;
            if expression.is_signed_or_unsigned_literal()
                || expressions.concatenates
                    && !expression.has_opaque_or_substitution_operands()
                    && expression.function_calls().is_empty()
            {
                return None;
            }
        }
        let hex_conversion = argument == last && original_word_is_hex_conversion(word);
        let fix_inner = if hex_conversion {
            // A substituted hexadecimal suffix forms a number before expr
            // parses it. Bracing leaves an invalid `0x` token before it.
            None
        } else if argument == last {
            word.content_span()
                .ok()
                .and_then(|content| self.source.get(content.as_range()))
                .map(str::to_owned)
        } else if (argument..=last).all(|index| {
            original.word(index).is_some_and(|word| {
                word.group().kind == tcl_lexer::WordKind::Bare && !word.bytes().contains(&b'\\')
            })
        }) {
            Some(text.clone())
        } else {
            None
        };
        Some(UnbracedSourceExpression {
            span,
            text,
            has_sub,
            fix_inner,
            subject,
            hex_conversion,
        })
    }

    /// Preserve the selected source purpose independently of message wording.
    fn push_w100(&mut self, command: &str, concatenates: bool, advice: UnbracedSourceExpression) {
        let UnbracedSourceExpression {
            span,
            text,
            has_sub,
            fix_inner,
            subject,
            hex_conversion,
        } = advice;
        let severity = if has_sub {
            Severity::Error
        } else {
            Severity::Warning
        };
        let mut message = if concatenates {
            "Expression is not braced: may cause double substitution and prevents byte-compilation. Use expr {...} instead.".to_owned()
        } else {
            format!(
                "Expression argument to '{command}' is not braced: may cause double substitution. Use braces: {{{text}}}"
            )
        };
        if hex_conversion {
            message.push_str(
                " This conversion cannot be braced directly. To convert a hexadecimal string to an integer, consider `scan $hex %x var`.",
            );
        }
        // Bracing a word nothing substitutes hands `expr` the same string.
        let safety = super::super::helpers::brace_wrap_fix_safety(&text, has_sub);
        let fixes = fix_inner
            .into_iter()
            .map(|inner| super::super::types::CodeFix {
                span,
                new_text: format!("{{{inner}}}"),
                description: "Wrap expression in braces".to_owned(),
                safety,
            })
            .collect();
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(DiagCode::W100, span, message, severity)
                .with_fixes(fixes)
                .with_subject(subject),
        );
    }

    /// W106: original unbraced action words selected by the current case-list
    /// grammar. Unknown payloads retain their executable lexical parts; captured
    /// actions have no written call-site span and therefore supply no warning.
    pub(in crate::analyser) fn emit_w106_unbraced_switch_body(
        &mut self,
        original: Option<&crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) {
        use crate::analyser::diagnostic_registry::{
            RegistrySourceDiagnosticKind as Kind, source_case_body_arguments,
        };
        let Some(original) = original else { return };
        let Some(bodies) = original.with_schema(source_case_body_arguments).flatten() else {
            return;
        };
        for body in bodies {
            if original.written_index(body.argument).is_none() {
                continue;
            }
            let Some(word) = original.word(body.argument) else {
                continue;
            };
            if word.group().kind == tcl_lexer::WordKind::Braced {
                continue;
            }
            let Some(subject) = original.subject(Kind::CaseBody, Some(body.argument)) else {
                continue;
            };
            let dangerous = original_word_has_substitution(word) || body.regexp;
            self.push_w106(
                word.span(),
                dangerous,
                body.regexp,
                body.single_block,
                subject,
            );
        }
    }

    fn push_w106(
        &mut self,
        span: tcl_lexer::Span,
        dangerous: bool,
        has_regexp: bool,
        single_block: bool,
        subject: crate::analyser::DiagnosticSubject,
    ) {
        let message = if has_regexp {
            "switch -regexp body is not braced — patterns and actions undergo extra substitution. Use braces: { … }"
        } else if dangerous && single_block {
            "switch body is not braced — contains substitutions. Use braces: switch … { pattern { body } … }"
        } else if single_block {
            "switch body is not braced — Use braces: switch … { pattern { body } … }"
        } else if dangerous {
            "switch body is not braced and contains substitutions. Use braces: { … }"
        } else {
            "switch body should be braced to prevent accidental substitution. Use braces: { … }"
        };
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(
                DiagCode::W106,
                span,
                message.to_owned(),
                if dangerous {
                    Severity::Error
                } else {
                    Severity::Warning
                },
            )
            .with_subject(subject),
        );
    }
}
