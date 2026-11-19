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
    /// **W105.** Emit "unbraced code block" warnings for body
    /// arguments that aren't braced.
    ///
    /// Severity is ERROR when the unbraced body contains
    /// substitutions (``$var`` / ``[cmd]``) — those risk double
    /// substitution.  Severity is WARNING otherwise.  Single
    /// barewords without substitution are silently allowed
    /// (some commands accept a proc name as a body alternative).
    pub(in crate::analyser) fn emit_w105_unbraced_body(
        &mut self,
        cmd_name: &str,
        body_text: &str,
        body_tok: tcl_lexer::Token,
        is_single_token: bool,
    ) {
        // Already braced — `Str` token kind means the source
        // started with ``{``.
        if matches!(body_tok.kind, tcl_lexer::TokenType::Str) {
            return;
        }
        // The source-schema pass and the body walk both reach a script word.
        if self
            .result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::W105 && diagnostic.span == body_tok.span)
        {
            return;
        }
        // A whole-word command-substitution body — `eval [list set y $x]`
        // (the recommended *safe* form), `uplevel [buildScript]` — is
        // produced dynamically and parsed once by the consumer: there is
        // no double-substitution risk and it cannot be braced
        // (`eval {[list …]}` changes the meaning).  (A `Var` body such as
        // `while {$cond} $body` is *not* exempt — only a `Cmd` word is.)
        if matches!(body_tok.kind, tcl_lexer::TokenType::Cmd)
            || self.is_prefixed_command_substitution(body_tok)
        {
            return;
        }
        // A body that is a *single bare variable substitution* (`eval $cmd`,
        // `proc $n $a $body`, `after 0 $coroName`, `$state(-command)`) is a
        // script-valued reference, not an inline code block: the variable
        // already holds the script.  Bracing it (`{$cmd}`) would turn the
        // reference into the literal text `$cmd` — the W105 quick-fix is
        // actively wrong here — and the genuine double-substitution (eval) /
        // dynamic-dispatch (command name) risks are W101's and W307's to
        // flag.  A *single-token* word whose token is a `Var` is exactly this
        // case; a composite word (`${t}--Coro`) or quoted interpolated body
        // (`"do $script"`) has more than one fragment and is not exempt.
        if is_single_token && matches!(body_tok.kind, tcl_lexer::TokenType::Var) {
            return;
        }
        let trimmed = body_text.trim();
        // Textual ``$`` / ``[`` count as substitutions, and so do
        // ``Var`` / ``Cmd`` tokens — even when the entire body is a direct
        // substitution (``while {$cond} $body``).  Those still
        // emit W105 at ERROR severity because an unbraced
        // substituted body double-evaluates at runtime.
        let has_substitution = trimmed.contains('$')
            || trimmed.contains('[')
            || matches!(
                body_tok.kind,
                tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
            );
        // Single bareword + no substitution is the alternative
        // form (e.g. body = a proc name).  Skip.
        if !trimmed.contains(char::is_whitespace) && !has_substitution {
            return;
        }
        // Deliberate severity escalation (documented in the DiagCode
        // catalogue): a provable substitution in an unbraced block is a
        // double-substitution bug waiting to run, not just style — the
        // W-family code stays, the severity is Error.
        let severity = if has_substitution {
            super::super::types::Severity::Error
        } else {
            super::super::types::Severity::Warning
        };
        let message = if has_substitution {
            format!(
                "Code block argument to '{cmd_name}' is not braced and \
contains substitutions \u{2014} risk of double substitution. \
Use braces: {{ \u{2026} }}"
            )
        } else {
            format!(
                "Code block argument to '{cmd_name}' should be braced \
for clarity and to prevent accidental substitution. \
Use braces: {{ \u{2026} }}"
            )
        };
        let new_text = format!("{{{body_text}}}");
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(
                DiagCode::W105,
                body_tok.span,
                message,
                severity,
            )
            .with_fixes(vec![super::super::types::CodeFix {
                span: body_tok.span,
                new_text,
                description: "Wrap code block in braces".to_string(),
                // Per-instance: bracing a substitution-free body reaches the
                // command byte-identically; bracing one that substitutes
                // removes a round of substitution by design.
                safety: super::super::helpers::brace_wrap_fix_safety(trimmed, has_substitution),
            }]),
        );
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
        let Some((texts, tokens, single)) = original.source_arguments() else {
            return;
        };
        let command = original.command().to_owned();
        let count = if concatenates { 1 } else { scripts.len() };
        for index in scripts.into_iter().take(count) {
            let (Some(text), Some(&first), Some(&one), Some(word)) = (
                texts.get(index),
                tokens.get(index),
                single.get(index),
                original.word(index),
            ) else {
                continue;
            };
            // The whole written word, as the body walk reports it.
            let token = tcl_lexer::Token::new(first.kind, word.span());
            self.emit_w105_unbraced_body(&command, text, token, one);
        }
    }

    /// A bare word that is one command substitution behind a literal,
    /// whitespace-free prefix (`bind $w <Destroy> +[list [self] destroy]`):
    /// the substitution builds the script once, so the prefix changes nothing
    /// about double substitution and bracing would stop the list being built.
    fn is_prefixed_command_substitution(&self, body_tok: tcl_lexer::Token) -> bool {
        if body_tok.kind != tcl_lexer::TokenType::Esc {
            return false;
        }
        let Ok(arena) = tcl_lexer::ExecutablePartArena::decompose(
            tcl_lexer::SourceImage::document(&self.source),
            body_tok.span,
            tcl_lexer::word_parts::SubstFlags::default(),
            self.file_lexer_config(),
        ) else {
            return false;
        };
        let mut commands = 0usize;
        let mut ends_with_command = false;
        for part in arena.all_parts() {
            ends_with_command = false;
            match part.part {
                tcl_lexer::ExecutablePart::Command { .. } => {
                    commands += 1;
                    ends_with_command = true;
                }
                tcl_lexer::ExecutablePart::Text(_)
                    if commands == 0
                        && self
                            .source
                            .get(part.span.as_range())
                            .is_some_and(|text| !text.contains(char::is_whitespace)) => {}
                _ => return false,
            }
        }
        commands == 1 && ends_with_command
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
